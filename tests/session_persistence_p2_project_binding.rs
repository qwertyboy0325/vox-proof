use std::fs;

use tempfile::TempDir;
use vox_proof::application_service::{
    ApplicationMaterialUseDeclaration, DeclaredApplicationMaterialUseBasis,
    DeclaredSessionAuthority, DeclaredSessionOperatorRole,
};
use vox_proof::candidate::SessionTermEntry;
use vox_proof::project_memory::{
    PROJECT_MEMORY_SNAPSHOT_IDENTITY_TAG_PREFIX, ProductProjectMemoryStore, ProjectMemoryOpenMode,
};
use vox_proof::session_persistence::{
    DurableApplicationSession, OpenMode, ProductSessionStore, SessionPersistenceError,
};
use vox_proof::srt::parse_srt;

fn material_use() -> ApplicationMaterialUseDeclaration {
    ApplicationMaterialUseDeclaration::new(DeclaredApplicationMaterialUseBasis::SelfOwned)
}

fn session_authority(label: &str) -> DeclaredSessionAuthority {
    DeclaredSessionAuthority::new(
        DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
        label,
    )
    .expect("authority")
}

fn alias_entry(canonical: &str, alias: &str) -> SessionTermEntry {
    SessionTermEntry::new(canonical, vec![alias.to_string()], Vec::new())
}

fn fixture_transcript() -> vox_proof::transcript::Transcript {
    parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("transcript")
}

fn fixture_terms() -> Vec<SessionTermEntry> {
    vec![alias_entry("Kafka", "Kafak")]
}

#[test]
fn v1_create_remains_format_one() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let durable = DurableApplicationSession::create(
        &store,
        fixture_transcript(),
        fixture_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    assert_eq!(durable.format_version(), 1);
    assert!(durable.bound_project_id().is_none());
    durable.close().expect("close");
}

#[test]
fn v2_bind_reopen_preserves_project_id_and_snapshot_domain() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());
    let project = project_store.create("Japan SKU").expect("project");
    let project_id = project.project_id().clone();
    project.close().expect("close project");

    let durable = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &project_id,
        fixture_transcript(),
        fixture_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("bound create");
    assert_eq!(durable.format_version(), 2);
    assert_eq!(durable.bound_project_id(), Some(&project_id));
    assert!(durable.project_memory_available());
    let session_id = durable.session_id().to_owned();
    durable.close().expect("close session");

    let reopened = DurableApplicationSession::open(&session_store, &session_id, OpenMode::ReadOnly)
        .expect("reopen");
    assert_eq!(reopened.format_version(), 2);
    assert_eq!(reopened.bound_project_id(), Some(&project_id));
    assert!(reopened.project_memory_available());
}

#[test]
fn missing_project_keeps_committed_session_readable_and_blocks_reuse_writes() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());
    let project = project_store.create("Japan SKU").expect("project");
    let project_id = project.project_id().clone();
    project.close().expect("close project");

    let mut durable = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &project_id,
        fixture_transcript(),
        fixture_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("bound create");
    let target = durable.session().review_items()[0].target;
    let prepared = durable
        .prepare_manual_replacement(target, "Kafka")
        .expect("prepare MR");
    durable
        .record_manual_replacement(prepared)
        .expect("record MR");
    let session_id = durable.session_id().to_owned();
    durable.close().expect("close session");

    fs::remove_dir_all(
        project_store
            .database_path(project_id.as_str())
            .parent()
            .expect("parent"),
    )
    .expect("remove project");

    let readable = DurableApplicationSession::open(&session_store, &session_id, OpenMode::ReadOnly)
        .expect("read-only open");
    assert!(!readable.project_memory_available());
    assert_eq!(readable.session().review_ledger().events().len(), 1);
    assert!(readable.project_memory_snapshot_identity().is_none());
    readable.close().expect("close");

    let writable = DurableApplicationSession::open(&session_store, &session_id, OpenMode::Writable)
        .expect("writable open");
    assert!(!writable.project_memory_available());
    let candidates = writable.session().reuse_candidates().expect("candidates");
    assert_eq!(candidates.len(), 1);
    let error = writable
        .prepare_accept_reuse_candidate(&candidates[0].key)
        .err()
        .expect("blocked");
    assert_eq!(error, SessionPersistenceError::WritableReuseBlocked);
    writable.close().expect("close");
}

#[test]
fn v2_promotion_writes_project_store_and_second_session_hydrates_snapshot() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());
    let project = project_store.create("Japan SKU").expect("project");
    let project_id = project.project_id().clone();
    project.close().expect("close project");

    let mut session_a = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &project_id,
        fixture_transcript(),
        fixture_terms(),
        material_use(),
        session_authority("operator-a"),
    )
    .expect("session a");
    let target = session_a.session().review_items()[0].target;
    let prepared = session_a
        .prepare_manual_replacement(target, "Kafka")
        .expect("prepare MR");
    session_a
        .record_manual_replacement(prepared)
        .expect("record MR");
    let candidate = session_a.session().reuse_candidates().expect("candidates")[0].clone();
    let prepared = session_a
        .prepare_accept_reuse_candidate(&candidate.key)
        .expect("prepare accept");
    session_a
        .record_accept_reuse_candidate(prepared)
        .expect("accept");
    let snapshot = session_a
        .project_memory_snapshot_identity()
        .expect("snapshot")
        .to_tagged_string();
    assert!(snapshot.starts_with(PROJECT_MEMORY_SNAPSHOT_IDENTITY_TAG_PREFIX));
    assert!(!snapshot.contains("reusable-influence-snapshot:sha256-v2:"));
    assert_eq!(
        session_a.session().reuse_state().governance_events().len(),
        1
    );
    session_a.close().expect("close a");

    let session_b = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &project_id,
        parse_srt("1\n00:00:00,000 --> 00:00:01,000\nHello").expect("b transcript"),
        Vec::new(),
        material_use(),
        session_authority("operator-b"),
    )
    .expect("session b");
    assert_eq!(
        session_b.session().reuse_state().governance_events().len(),
        1
    );
    assert_eq!(
        session_b
            .project_memory_snapshot_identity()
            .expect("b snapshot")
            .to_tagged_string(),
        snapshot
    );
}

#[test]
fn v2_promotion_records_source_session_id_from_promoter() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());
    let project = project_store.create("Japan SKU").expect("project");
    let project_id = project.project_id().clone();
    project.close().expect("close project");

    let mut session_a = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &project_id,
        fixture_transcript(),
        fixture_terms(),
        material_use(),
        session_authority("operator-a"),
    )
    .expect("session a");
    let session_a_id = session_a.session_id().to_owned();
    let target = session_a.session().review_items()[0].target;
    let prepared = session_a
        .prepare_manual_replacement(target, "Kafka")
        .expect("prepare MR");
    session_a
        .record_manual_replacement(prepared)
        .expect("record MR");
    let candidate = session_a.session().reuse_candidates().expect("candidates")[0].clone();
    let prepared = session_a
        .prepare_accept_reuse_candidate(&candidate.key)
        .expect("prepare accept");
    session_a
        .record_accept_reuse_candidate(prepared)
        .expect("accept");
    session_a.close().expect("close a");

    let reopened_project = project_store
        .open(&project_id, ProjectMemoryOpenMode::ReadOnly)
        .expect("project reopen");
    assert_eq!(
        reopened_project.records()[0].source_session_id,
        session_a_id
    );
}

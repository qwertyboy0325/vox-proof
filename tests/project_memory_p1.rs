use tempfile::TempDir;
use vox_proof::application_reuse::build_promotion_accepted_event;
use vox_proof::application_service::{
    ApplicationMaterialUseDeclaration, DeclaredApplicationMaterialUseBasis,
    DeclaredSessionAuthority, DeclaredSessionOperatorRole, begin_application_review,
};
use vox_proof::candidate::SessionTermEntry;
use vox_proof::project_memory::{
    PROJECT_MEMORY_FORMAT_VERSION, PROJECT_MEMORY_SNAPSHOT_IDENTITY_TAG_PREFIX,
    ProductProjectMemoryStore, ProjectMemoryError, ProjectMemoryOpenMode,
    compute_project_memory_snapshot_identity,
};
use vox_proof::srt::parse_srt;

fn material_use() -> ApplicationMaterialUseDeclaration {
    ApplicationMaterialUseDeclaration::new(DeclaredApplicationMaterialUseBasis::SelfOwned)
}

fn authority(label: &str) -> DeclaredSessionAuthority {
    DeclaredSessionAuthority::new(
        DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
        label,
    )
    .expect("authority")
}

fn promoted_event(source_label: &str) -> vox_proof::reusable_influence::ReusableGovernanceEvent {
    let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("transcript");
    let mut session = begin_application_review(
        transcript,
        vec![SessionTermEntry::new(
            "Kafka",
            vec!["Kafak".to_string()],
            Vec::new(),
        )],
        material_use(),
        authority(source_label),
    )
    .expect("session");
    let target = session.review_items()[0].target;
    session
        .record_manual_replacement(target, "Kafka")
        .expect("manual replacement");
    session
        .initialize_project_scope("placeholder", "Placeholder")
        .expect("scope");
    let candidate = session.reuse_candidates().expect("candidates")[0].clone();
    let scope = session
        .reuse_state()
        .project_scope()
        .expect("scope")
        .clone();
    build_promotion_accepted_event(&candidate, &scope, session.session_authority())
}

#[test]
fn create_assigns_product_generated_opaque_id_distinct_from_display_name() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductProjectMemoryStore::new(temp.path());
    let project = store.create("Japan SKU series").expect("create");
    assert_ne!(project.project_id().as_str(), "Japan SKU series");
    assert_eq!(project.display_name().as_str(), "Japan SKU series");
    assert!(uuid::Uuid::parse_str(project.project_id().as_str()).is_ok());
    project.close().expect("close");
}

#[test]
fn promotion_survives_reopen_and_uses_new_snapshot_domain() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductProjectMemoryStore::new(temp.path());
    let mut project = store.create("Project A").expect("create");
    let project_id = project.project_id().clone();
    let event = promoted_event("operator-a");
    project
        .append_promotion("session-a", event)
        .expect("append");
    let identity = project.snapshot_identity().to_tagged_string();
    assert!(identity.starts_with(PROJECT_MEMORY_SNAPSHOT_IDENTITY_TAG_PREFIX));
    assert!(!identity.contains("reusable-influence-snapshot:sha256-v2:"));
    project.close().expect("close");

    let reopened = store
        .open(&project_id, ProjectMemoryOpenMode::ReadOnly)
        .expect("reopen");
    assert_eq!(reopened.records().len(), 1);
    assert_eq!(reopened.records()[0].source_session_id, "session-a");
    assert_eq!(reopened.snapshot_identity().to_tagged_string(), identity);
}

#[test]
fn source_session_id_changes_snapshot_identity() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductProjectMemoryStore::new(temp.path());
    let mut project = store.create("Project").expect("create");
    project
        .append_promotion("session-a", promoted_event("operator-a"))
        .expect("append");
    let mut records = project.records().to_vec();
    let left = compute_project_memory_snapshot_identity(
        project.project_id(),
        PROJECT_MEMORY_FORMAT_VERSION,
        records.len(),
        &records,
    );
    records[0].source_session_id = "session-b".to_owned();
    let right = compute_project_memory_snapshot_identity(
        project.project_id(),
        PROJECT_MEMORY_FORMAT_VERSION,
        records.len(),
        &records,
    );
    assert_ne!(left, right);
}

#[test]
fn display_name_update_does_not_change_identity_or_snapshot() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductProjectMemoryStore::new(temp.path());
    let mut project = store.create("Old name").expect("create");
    let project_id = project.project_id().clone();
    project
        .append_promotion("session-a", promoted_event("op"))
        .expect("append");
    let snapshot = project.snapshot_identity();
    project.update_display_name("New name").expect("rename");
    assert_eq!(project.display_name().as_str(), "New name");
    assert_eq!(project.project_id(), &project_id);
    assert_eq!(project.snapshot_identity(), snapshot);
}

#[test]
fn missing_project_is_not_found() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductProjectMemoryStore::new(temp.path());
    let missing =
        vox_proof::reuse_primitives::ProjectScopeId::new(uuid::Uuid::new_v4().to_string())
            .expect("id");
    let error = store
        .open(&missing, ProjectMemoryOpenMode::ReadOnly)
        .err()
        .expect("missing");
    assert_eq!(error, ProjectMemoryError::ProjectNotFound);
}

#[test]
fn second_writer_is_refused() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductProjectMemoryStore::new(temp.path());
    let first = store.create("Held").expect("create");
    let project_id = first.project_id().clone();
    let error = store
        .open(&project_id, ProjectMemoryOpenMode::Writable)
        .err()
        .expect("second writer");
    assert_eq!(error, ProjectMemoryError::WriterOwnershipHeld);
    first.close().expect("close");
    store
        .open(&project_id, ProjectMemoryOpenMode::Writable)
        .expect("reopen after release")
        .close()
        .expect("close");
}

#[test]
fn empty_source_session_id_is_rejected() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductProjectMemoryStore::new(temp.path());
    let mut project = store.create("Project").expect("create");
    let error = project
        .append_promotion("", promoted_event("op"))
        .expect_err("empty");
    assert_eq!(error, ProjectMemoryError::MissingSourceSessionId);
}

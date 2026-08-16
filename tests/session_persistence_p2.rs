use std::fs;

use tempfile::TempDir;
use vox_proof::application_service::{
    ApplicationMaterialUseDeclaration, DeclaredApplicationMaterialUseBasis,
    DeclaredSessionAuthority, DeclaredSessionOperatorRole,
};
use vox_proof::candidate::SessionTermEntry;
use vox_proof::review::CorrectionDecision;
use vox_proof::reuse_primitives::ReusableInfluenceRecordId;
use vox_proof::session_persistence::{
    AuthorityScope, DurableApplicationSession, OpenMode, ProductSessionStore,
    SessionPersistenceError,
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

fn manual_replacement_durable(
    store: &ProductSessionStore,
    replacement: &str,
) -> DurableApplicationSession {
    let mut durable = DurableApplicationSession::create(
        store,
        fixture_transcript(),
        fixture_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    let target = durable.session().review_items()[0].target;
    let prepared = durable
        .prepare_manual_replacement(target, replacement)
        .expect("prepare");
    durable.record_manual_replacement(prepared).expect("record");
    durable
}

fn initialize_scope(durable: &mut DurableApplicationSession) {
    let prepared = durable
        .prepare_initialize_project_scope("proj-a", "Project A")
        .expect("prepare scope");
    durable
        .record_initialize_project_scope(prepared)
        .expect("record scope");
}

#[test]
fn durable_project_scope_initialize_survives_reopen() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = manual_replacement_durable(&store, "Kafka");
    initialize_scope(&mut durable);
    let session_id = durable.session_id().to_owned();
    durable.close().expect("close");
    let reopened =
        DurableApplicationSession::open(&store, &session_id, OpenMode::ReadOnly).expect("reopen");
    assert!(reopened.session().has_project_scope());
    assert_eq!(
        reopened
            .session()
            .reuse_state()
            .project_scope()
            .unwrap()
            .stable_id
            .as_str(),
        "proj-a"
    );
}

#[test]
fn durable_display_name_update_is_metadata_only() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = manual_replacement_durable(&store, "Kafka");
    initialize_scope(&mut durable);
    let prepared = durable
        .prepare_update_project_scope_display_name("Project A Renamed")
        .expect("prepare");
    durable
        .record_update_project_scope_display_name(prepared)
        .expect("record");
    assert_eq!(
        durable
            .session()
            .reuse_state()
            .project_scope()
            .unwrap()
            .display_name
            .as_str(),
        "Project A Renamed"
    );
}

#[test]
fn durable_accept_reuse_candidate_survives_reopen() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = manual_replacement_durable(&store, "Kafka");
    initialize_scope(&mut durable);
    let key = durable.session().reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    let prepared = durable
        .prepare_accept_reuse_candidate(&key)
        .expect("prepare accept");
    durable
        .record_accept_reuse_candidate(prepared)
        .expect("record accept");
    let session_id = durable.session_id().to_owned();
    durable.close().expect("close");
    let reopened =
        DurableApplicationSession::open(&store, &session_id, OpenMode::ReadOnly).expect("reopen");
    assert_eq!(reopened.session().reuse_state().governance_events().len(), 1);
    assert_eq!(
        reopened.session().active_reusable_records().expect("records").len(),
        1
    );
}

#[test]
fn durable_reject_reuse_candidate_persists_governance_event() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = manual_replacement_durable(&store, "Kafka");
    initialize_scope(&mut durable);
    let key = durable.session().reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    let prepared = durable
        .prepare_reject_reuse_candidate(&key)
        .expect("prepare reject");
    durable
        .record_reject_reuse_candidate(prepared)
        .expect("record reject");
    assert_eq!(durable.session().reuse_state().governance_events().len(), 1);
    assert!(durable.session().reuse_candidates().expect("candidates").is_empty());
}

#[test]
fn durable_revoke_reusable_influence_persists() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = manual_replacement_durable(&store, "Kafka");
    initialize_scope(&mut durable);
    let key = durable.session().reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    durable
        .record_accept_reuse_candidate(
            durable
                .prepare_accept_reuse_candidate(&key)
                .expect("prepare accept"),
        )
        .expect("accept");
    let record_id = ReusableInfluenceRecordId::from_promotion_event_index(0);
    let prepared = durable
        .prepare_revoke_reusable_influence(record_id)
        .expect("prepare revoke");
    durable
        .record_revoke_reusable_influence(prepared)
        .expect("record revoke");
    assert!(durable
        .session()
        .active_reusable_records()
        .expect("records")
        .is_empty());
}

#[test]
fn revoke_prepared_before_review_ledger_change_still_commits() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = DurableApplicationSession::create(
        &store,
        parse_srt(
            "1\n00:00:00,000 --> 00:00:01,000\nKafak\n\n2\n00:00:01,000 --> 00:00:02,000\nKafak",
        )
        .expect("transcript"),
        fixture_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    let first = durable.session().review_items()[0].target;
    durable
        .record_manual_replacement(
            durable.prepare_manual_replacement(first, "Kafka").expect("prep"),
        )
        .expect("manual first");
    initialize_scope(&mut durable);
    let key = durable.session().reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    durable
        .record_accept_reuse_candidate(
            durable
                .prepare_accept_reuse_candidate(&key)
                .expect("prepare accept"),
        )
        .expect("accept");
    let record_id = ReusableInfluenceRecordId::from_promotion_event_index(0);
    let prepared_revoke = durable
        .prepare_revoke_reusable_influence(record_id)
        .expect("prepare revoke");
    let second = durable.session().review_items()[1].target;
    durable
        .record_human_decision(
            durable
                .prepare_human_decision(second, CorrectionDecision::Reject)
                .expect("prepare reject review"),
        )
        .expect("review reject");
    durable
        .record_revoke_reusable_influence(prepared_revoke)
        .expect("revoke after review advance");
    assert_eq!(durable.session().reuse_state().governance_events().len(), 2);
}

#[test]
fn durable_supersede_persists_two_governance_events_atomically() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let transcript = parse_srt(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak\n\n2\n00:00:01,000 --> 00:00:02,000\nKafak",
    )
    .expect("transcript");
    let mut durable = DurableApplicationSession::create(
        &store,
        transcript,
        fixture_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    let first = durable.session().review_items()[0].target;
    durable
        .record_manual_replacement(
            durable.prepare_manual_replacement(first, "Kafka").expect("prep"),
        )
        .expect("manual first");
    let second = durable.session().review_items()[1].target;
    durable
        .record_manual_replacement(
            durable.prepare_manual_replacement(second, "Kafka").expect("prep"),
        )
        .expect("manual second");
    initialize_scope(&mut durable);
    let candidates = durable.session().reuse_candidates().expect("candidates");
    let first_key = candidates[0].key.clone();
    durable
        .record_accept_reuse_candidate(
            durable
                .prepare_accept_reuse_candidate(&first_key)
                .expect("prepare accept"),
        )
        .expect("accept first");
    let predecessor = ReusableInfluenceRecordId::from_promotion_event_index(0);
    let successor_key = candidates[1].key.clone();
    let prepared = durable
        .prepare_supersede_reusable_influence(predecessor, &successor_key)
        .expect("prepare supersede");
    durable
        .record_supersede_reusable_influence(prepared)
        .expect("record supersede");
    assert_eq!(durable.session().reuse_state().governance_events().len(), 3);
}

#[test]
fn durable_run_reuse_enabled_review_survives_reopen() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = manual_replacement_durable(&store, "Kafka");
    initialize_scope(&mut durable);
    let key = durable.session().reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    durable
        .record_accept_reuse_candidate(
            durable
                .prepare_accept_reuse_candidate(&key)
                .expect("prepare accept"),
        )
        .expect("accept");
    let (prepared, precondition) = durable
        .prepare_run_reuse_enabled_review()
        .expect("prepare run");
    durable
        .record_run_reuse_enabled_review(prepared, precondition)
        .expect("record run");
    assert!(durable.session().reuse_enabled_run().is_some());
    let session_id = durable.session_id().to_owned();
    durable.close().expect("close");
    let reopened =
        DurableApplicationSession::open(&store, &session_id, OpenMode::ReadOnly).expect("reopen");
    assert!(reopened.session().reuse_enabled_run().is_some());
}

#[test]
fn historical_bindings_preserved_after_second_reuse_enabled_commit() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let transcript = parse_srt(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak\n\n2\n00:00:01,000 --> 00:00:02,000\nKafak",
    )
    .expect("transcript");
    let mut durable = DurableApplicationSession::create(
        &store,
        transcript,
        fixture_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    let first = durable.session().review_items()[0].target;
    durable
        .record_manual_replacement(
            durable.prepare_manual_replacement(first, "Kafka").expect("prep"),
        )
        .expect("manual first");
    let second = durable.session().review_items()[1].target;
    durable
        .record_manual_replacement(
            durable.prepare_manual_replacement(second, "Kafka").expect("prep"),
        )
        .expect("manual second");
    initialize_scope(&mut durable);
    let key = durable.session().reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    durable
        .record_accept_reuse_candidate(
            durable
                .prepare_accept_reuse_candidate(&key)
                .expect("prepare accept"),
        )
        .expect("accept");
    let (prepared, precondition) = durable
        .prepare_run_reuse_enabled_review()
        .expect("prepare run");
    durable
        .record_run_reuse_enabled_review(prepared, precondition)
        .expect("record run");
    let session_id = durable.session_id().to_owned();
    let db_path = temp.path().join(&session_id).join("session.db");
    durable.close().expect("close");
    let connection = rusqlite::Connection::open(&db_path).expect("open db");
    let binding_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM reuse_enabled_bindings WHERE session_id = ?1",
            [&session_id],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(binding_count, 1);
}

#[test]
fn stale_reuse_governance_precondition_on_accept() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let transcript = parse_srt(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak\n\n2\n00:00:01,000 --> 00:00:02,000\nKafak",
    )
    .expect("transcript");
    let mut durable = DurableApplicationSession::create(
        &store,
        transcript,
        fixture_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    let first = durable.session().review_items()[0].target;
    durable
        .record_manual_replacement(
            durable.prepare_manual_replacement(first, "Kafka").expect("prep"),
        )
        .expect("manual first");
    let second = durable.session().review_items()[1].target;
    durable
        .record_manual_replacement(
            durable.prepare_manual_replacement(second, "Kafka").expect("prep"),
        )
        .expect("manual second");
    initialize_scope(&mut durable);
    let candidates = durable.session().reuse_candidates().expect("candidates");
    let stale_prepared = durable
        .prepare_accept_reuse_candidate(&candidates[0].key)
        .expect("prepare first accept");
    durable
        .record_accept_reuse_candidate(
            durable
                .prepare_accept_reuse_candidate(&candidates[1].key)
                .expect("prepare second accept"),
        )
        .expect("accept second");
    let err = durable
        .record_accept_reuse_candidate(stale_prepared)
        .expect_err("stale");
    assert!(matches!(
        err,
        SessionPersistenceError::StaleAuthorityPrecondition(scope)
            if scope.scope == AuthorityScope::ReuseGovernance
    ));
}

#[test]
fn stale_active_analysis_precondition_on_commit() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = manual_replacement_durable(&store, "Kafka");
    initialize_scope(&mut durable);
    let key = durable.session().reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    durable
        .record_accept_reuse_candidate(
            durable
                .prepare_accept_reuse_candidate(&key)
                .expect("prepare accept"),
        )
        .expect("accept");
    let (prepared, precondition) = durable
        .prepare_run_reuse_enabled_review()
        .expect("prepare run");
    durable
        .record_revoke_reusable_influence(
            durable
                .prepare_revoke_reusable_influence(
                    ReusableInfluenceRecordId::from_promotion_event_index(0),
                )
                .expect("prepare revoke"),
        )
        .expect("revoke");
    let err = durable
        .record_run_reuse_enabled_review(prepared, precondition)
        .expect_err("stale analysis");
    assert!(matches!(
        err,
        SessionPersistenceError::StaleAuthorityPrecondition(scope)
            if scope.scope == AuthorityScope::ActiveAnalysis
                || scope.scope == AuthorityScope::ReuseGovernance
    ));
}

#[test]
fn tampered_governance_event_fails_reopen() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = manual_replacement_durable(&store, "Kafka");
    initialize_scope(&mut durable);
    let key = durable.session().reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    durable
        .record_accept_reuse_candidate(
            durable
                .prepare_accept_reuse_candidate(&key)
                .expect("prepare accept"),
        )
        .expect("accept");
    let session_id = durable.session_id().to_owned();
    let db_path = temp.path().join(&session_id).join("session.db");
    durable.close().expect("close");
    let connection = rusqlite::Connection::open(&db_path).expect("open db");
    connection
        .execute(
            "UPDATE reuse_governance_events SET event_json = json_set(event_json, '$.event_kind', 'tampered') WHERE session_id = ?1",
            [&session_id],
        )
        .expect("tamper");
    assert!(matches!(
        DurableApplicationSession::open(&store, &session_id, OpenMode::ReadOnly),
        Err(SessionPersistenceError::CanonicalMismatch(_))
    ));
}

#[test]
fn partial_supersession_single_event_fails_reopen() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let transcript = parse_srt(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak\n\n2\n00:00:01,000 --> 00:00:02,000\nKafak",
    )
    .expect("transcript");
    let mut durable = DurableApplicationSession::create(
        &store,
        transcript,
        fixture_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    let first = durable.session().review_items()[0].target;
    durable
        .record_manual_replacement(
            durable.prepare_manual_replacement(first, "Kafka").expect("prep"),
        )
        .expect("manual first");
    let second = durable.session().review_items()[1].target;
    durable
        .record_manual_replacement(
            durable.prepare_manual_replacement(second, "Kafka").expect("prep"),
        )
        .expect("manual second");
    initialize_scope(&mut durable);
    let candidates = durable.session().reuse_candidates().expect("candidates");
    let first_key = candidates[0].key.clone();
    durable
        .record_accept_reuse_candidate(
            durable
                .prepare_accept_reuse_candidate(&first_key)
                .expect("prepare accept"),
        )
        .expect("accept first");
    let session_id = durable.session_id().to_owned();
    let db_path = temp.path().join(&session_id).join("session.db");
    durable.close().expect("close");
    let connection = rusqlite::Connection::open(&db_path).expect("open db");
    let event_json: String = connection
        .query_row(
            "SELECT event_json FROM reuse_governance_events WHERE session_id = ?1 ORDER BY event_index DESC LIMIT 1",
            [&session_id],
            |row| row.get(0),
        )
        .expect("event");
    connection
        .execute(
            "INSERT INTO reuse_governance_events (session_id, event_index, event_json) VALUES (?1, 2, ?2)",
            rusqlite::params![session_id, event_json],
        )
        .expect("insert orphan promotion");
    connection
        .execute(
            "UPDATE command_tokens SET reuse_governance_head = 3 WHERE session_id = ?1",
            [&session_id],
        )
        .expect("bump head");
    assert!(matches!(
        DurableApplicationSession::open(&store, &session_id, OpenMode::ReadOnly),
        Err(SessionPersistenceError::CanonicalMismatch(_))
    ));
    let _ = fs::read_dir(temp.path());
}

#[test]
fn second_project_scope_initialize_is_rejected() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = manual_replacement_durable(&store, "Kafka");
    initialize_scope(&mut durable);
    let err = durable
        .prepare_initialize_project_scope("proj-b", "Project B")
        .expect_err("second init prepare");
    assert!(matches!(
        err,
        SessionPersistenceError::CanonicalMismatch(message)
            if message.contains("ProjectScopeFrozen")
    ));
}

#[test]
fn durable_display_name_survives_reopen() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = manual_replacement_durable(&store, "Kafka");
    initialize_scope(&mut durable);
    let head_before = durable.session().reuse_state().governance_events().len();
    durable
        .record_update_project_scope_display_name(
            durable
                .prepare_update_project_scope_display_name("Renamed Project")
                .expect("prepare"),
        )
        .expect("record");
    let session_id = durable.session_id().to_owned();
    durable.close().expect("close");
    let reopened =
        DurableApplicationSession::open(&store, &session_id, OpenMode::ReadOnly).expect("reopen");
    assert_eq!(
        reopened
            .session()
            .reuse_state()
            .project_scope()
            .unwrap()
            .display_name
            .as_str(),
        "Renamed Project"
    );
    assert_eq!(
        reopened.session().reuse_state().governance_events().len(),
        head_before
    );
}

#[test]
fn accept_prepared_before_unrelated_review_ledger_change_still_commits() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let transcript = parse_srt(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak\n\n2\n00:00:01,000 --> 00:00:02,000\nKafak",
    )
    .expect("transcript");
    let mut durable = DurableApplicationSession::create(
        &store,
        transcript,
        fixture_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    let first = durable.session().review_items()[0].target;
    durable
        .record_manual_replacement(
            durable.prepare_manual_replacement(first, "Kafka").expect("prep"),
        )
        .expect("manual first");
    initialize_scope(&mut durable);
    let key = durable.session().reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    let prepared_accept = durable
        .prepare_accept_reuse_candidate(&key)
        .expect("prepare accept");
    let second = durable.session().review_items()[1].target;
    durable
        .record_human_decision(
            durable
                .prepare_human_decision(second, CorrectionDecision::Reject)
                .expect("prepare reject review"),
        )
        .expect("review reject");
    durable
        .record_accept_reuse_candidate(prepared_accept)
        .expect("accept after unrelated ledger change");
    assert_eq!(durable.session().reuse_state().governance_events().len(), 1);
}

#[test]
fn accept_refused_when_source_manual_replacement_changed() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = manual_replacement_durable(&store, "Kafka");
    initialize_scope(&mut durable);
    let key = durable.session().reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    let prepared_accept = durable
        .prepare_accept_reuse_candidate(&key)
        .expect("prepare accept");
    let target = durable.session().review_items()[0].target;
    durable
        .record_manual_replacement(
            durable
                .prepare_manual_replacement(target, "Kafaka")
                .expect("prep"),
        )
        .expect("manual replacement change");
    let err = durable
        .record_accept_reuse_candidate(prepared_accept)
        .expect_err("stale source");
    assert!(matches!(
        err,
        SessionPersistenceError::CanonicalMismatch(_)
            | SessionPersistenceError::StaleAuthorityPrecondition(_)
    ));
    assert_eq!(durable.session().reuse_state().governance_events().len(), 0);
}

#[test]
fn distinct_selection_tokens_for_different_reusable_snapshots() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let transcript = parse_srt(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak\n\n2\n00:00:01,000 --> 00:00:02,000\nKafak",
    )
    .expect("transcript");
    let mut durable = DurableApplicationSession::create(
        &store,
        transcript,
        fixture_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    let first = durable.session().review_items()[0].target;
    durable
        .record_manual_replacement(
            durable.prepare_manual_replacement(first, "Kafka").expect("prep"),
        )
        .expect("manual first");
    let second = durable.session().review_items()[1].target;
    durable
        .record_manual_replacement(
            durable.prepare_manual_replacement(second, "Kafka").expect("prep"),
        )
        .expect("manual second");
    initialize_scope(&mut durable);
    let candidates = durable.session().reuse_candidates().expect("candidates");
    durable
        .record_accept_reuse_candidate(
            durable
                .prepare_accept_reuse_candidate(&candidates[0].key)
                .expect("prepare accept"),
        )
        .expect("accept first");
    let (prepared_one, token_one) = durable
        .prepare_run_reuse_enabled_review()
        .expect("prepare run one");
    durable
        .record_run_reuse_enabled_review(prepared_one, token_one.clone())
        .expect("record run one");
    let identity_after_first = durable
        .session()
        .reuse_enabled_run()
        .expect("run")
        .reusable_snapshot_identity();
    durable
        .record_accept_reuse_candidate(
            durable
                .prepare_accept_reuse_candidate(&candidates[1].key)
                .expect("prepare accept second"),
        )
        .expect("accept second");
    let (prepared_two, token_two) = durable
        .prepare_run_reuse_enabled_review()
        .expect("prepare run two");
    assert_ne!(token_one, token_two);
    durable
        .record_run_reuse_enabled_review(prepared_two, token_two)
        .expect("record run two");
    let identity_after_second = durable
        .session()
        .reuse_enabled_run()
        .expect("run")
        .reusable_snapshot_identity();
    assert_ne!(identity_after_first, identity_after_second);
}

#[test]
fn governance_change_after_run_invalidates_current_reuse_enabled_run() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = manual_replacement_durable(&store, "Kafka");
    initialize_scope(&mut durable);
    let key = durable.session().reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    durable
        .record_accept_reuse_candidate(
            durable
                .prepare_accept_reuse_candidate(&key)
                .expect("prepare accept"),
        )
        .expect("accept");
    let (prepared, precondition) = durable
        .prepare_run_reuse_enabled_review()
        .expect("prepare run");
    durable
        .record_run_reuse_enabled_review(prepared, precondition)
        .expect("record run");
    let session_id = durable.session_id().to_owned();
    let db_path = temp.path().join(&session_id).join("session.db");
    durable
        .record_revoke_reusable_influence(
            durable
                .prepare_revoke_reusable_influence(
                    ReusableInfluenceRecordId::from_promotion_event_index(0),
                )
                .expect("prepare revoke"),
        )
        .expect("revoke");
    durable.close().expect("close");
    let connection = rusqlite::Connection::open(&db_path).expect("open db");
    let binding_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM reuse_enabled_bindings WHERE session_id = ?1",
            [&session_id],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(binding_count, 1);
    let reopened =
        DurableApplicationSession::open(&store, &session_id, OpenMode::ReadOnly).expect("reopen");
    assert!(reopened.session().reuse_enabled_run().is_none());
}

#[test]
fn export_v3_parity_after_reopen_with_reuse_state() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = manual_replacement_durable(&store, "Kafka");
    initialize_scope(&mut durable);
    let key = durable.session().reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    durable
        .record_accept_reuse_candidate(
            durable
                .prepare_accept_reuse_candidate(&key)
                .expect("prepare accept"),
        )
        .expect("accept");
    let (prepared, precondition) = durable
        .prepare_run_reuse_enabled_review()
        .expect("prepare run");
    durable
        .record_run_reuse_enabled_review(prepared, precondition)
        .expect("record run");
    let before = durable.session().materialize_review_export_bundle_v3();
    let session_id = durable.session_id().to_owned();
    durable.close().expect("close");
    let reopened =
        DurableApplicationSession::open(&store, &session_id, OpenMode::ReadOnly).expect("reopen");
    let after = reopened.session().materialize_review_export_bundle_v3();
    assert_eq!(
        before.as_ref().map_err(|e| e.to_string()),
        after.as_ref().map_err(|e| e.to_string())
    );
    assert!(before.is_ok());
    assert_eq!(before.unwrap(), after.unwrap());
}

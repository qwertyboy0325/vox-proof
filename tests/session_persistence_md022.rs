use tempfile::TempDir;
use vox_proof::application_service::{
    ApplicationMaterialUseDeclaration, ApplicationServiceError,
    DeclaredApplicationMaterialUseBasis, DeclaredSessionAuthority, DeclaredSessionOperatorRole,
};
use vox_proof::candidate::SessionTermEntry;
use vox_proof::project_memory::{
    PROJECT_MEMORY_FORMAT_VERSION_V3, ProductProjectMemoryStore, ProjectMemoryOpenMode,
};
use vox_proof::review::{CorrectionDecision, ReviewLedgerEvent};
use vox_proof::session_persistence::{
    DurableApplicationSession, OpenMode, ProductSessionStore, SessionPersistenceError,
    arm_fail_after_human_raise_before_decision_for_test, arm_fail_before_commit_for_test,
    disarm_fail_after_human_raise_before_decision_for_test, disarm_fail_before_commit_for_test,
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

fn mixed_transcript() -> vox_proof::transcript::Transcript {
    parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak and Postgres").expect("transcript")
}

fn kafka_terms() -> Vec<SessionTermEntry> {
    vec![alias_entry("Kafka", "Kafak")]
}

fn freeze_bound(durable: &mut DurableApplicationSession) {
    let (prepared, precondition) = durable
        .prepare_run_reuse_enabled_review()
        .expect("prepare freeze");
    durable
        .record_run_reuse_enabled_review(prepared, precondition)
        .expect("freeze");
}

#[test]
fn new_unbound_session_is_format_v4() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let durable = DurableApplicationSession::create(
        &store,
        mixed_transcript(),
        kafka_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    assert_eq!(durable.format_version(), 4);
    assert!(durable.bound_project_id().is_none());
    durable.close().expect("close");
}

#[test]
fn raise_and_replace_persists_both_events_and_survives_reopen() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = DurableApplicationSession::create(
        &store,
        mixed_transcript(),
        kafka_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    let detector_target = durable.session().review_items()[0].target;
    let prepared = durable
        .prepare_human_decision(detector_target, CorrectionDecision::Reject)
        .expect("prepare reject");
    durable
        .record_human_decision(prepared)
        .expect("reject detector");
    let before = durable
        .session()
        .materialize_reviewed_output()
        .expect("before");
    assert!(before.srt.contains("Postgres"));
    assert!(!before.srt.contains("PostgreSQL"));

    let case_id = durable
        .raise_and_manual_replace(0, 10, 18, "PostgreSQL")
        .expect("raise");
    assert!(case_id.is_human_raised());
    let events = durable.session().review_ledger().events();
    assert!(matches!(events[1], ReviewLedgerEvent::CaseRaised { .. }));
    assert!(matches!(
        events[2],
        ReviewLedgerEvent::DecisionRecorded {
            decision: CorrectionDecision::ManualReplacement { .. },
            ..
        }
    ));
    let after = durable
        .session()
        .materialize_reviewed_output()
        .expect("after");
    assert!(after.srt.contains("PostgreSQL"));
    assert!(!after.srt.contains("and Postgres"));
    assert!(
        durable.session().human_raised_cases()[0]
            .as_detector_span()
            .is_none()
    );
    let session_id = durable.session_id().to_owned();
    durable.close().expect("close");

    let reopened =
        DurableApplicationSession::open(&store, &session_id, OpenMode::ReadOnly).expect("reopen");
    assert_eq!(reopened.format_version(), 4);
    assert_eq!(reopened.session().human_raised_cases().len(), 1);
    assert_eq!(reopened.session().review_ledger().events().len(), 3);
    let reopened_srt = reopened
        .session()
        .materialize_reviewed_output()
        .expect("reopened output");
    assert!(reopened_srt.srt.contains("PostgreSQL"));
}

#[test]
fn fault_before_commit_leaves_neither_human_raised_event() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = DurableApplicationSession::create(
        &store,
        mixed_transcript(),
        kafka_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    let session_id = durable.session_id().to_owned();
    arm_fail_before_commit_for_test();
    let error = durable
        .raise_and_manual_replace(0, 10, 18, "PostgreSQL")
        .err()
        .expect("fault");
    disarm_fail_before_commit_for_test();
    assert!(matches!(error, SessionPersistenceError::Sqlite(_)));
    durable.close().expect("close after fault");

    let reopened =
        DurableApplicationSession::open(&store, &session_id, OpenMode::ReadOnly).expect("reopen");
    assert!(reopened.session().human_raised_cases().is_empty());
    assert!(reopened.session().review_ledger().events().is_empty());
}

#[test]
fn fault_after_case_raised_before_decision_leaves_neither_event() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = DurableApplicationSession::create(
        &store,
        mixed_transcript(),
        kafka_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    let session_id = durable.session_id().to_owned();
    arm_fail_after_human_raise_before_decision_for_test();
    let error = durable
        .raise_and_manual_replace(0, 10, 18, "PostgreSQL")
        .err()
        .expect("fault");
    disarm_fail_after_human_raise_before_decision_for_test();
    assert!(matches!(error, SessionPersistenceError::Sqlite(_)));
    durable.close().expect("close after fault");

    let reopened =
        DurableApplicationSession::open(&store, &session_id, OpenMode::ReadOnly).expect("reopen");
    assert!(reopened.session().human_raised_cases().is_empty());
    assert!(reopened.session().review_ledger().events().is_empty());
}

fn session_db(root: &std::path::Path, session_id: &str) -> std::path::PathBuf {
    root.join(session_id).join("session.db")
}

fn table_exists(db_path: &std::path::Path, name: &str) -> bool {
    let connection = rusqlite::Connection::open(db_path).expect("open db");
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [name],
            |row| row.get(0),
        )
        .expect("sqlite_master");
    count > 0
}

#[test]
fn historical_v1_session_opens_without_human_raised_table_and_refuses_writes() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable =
        DurableApplicationSession::create_historical_unbound_format_v1_for_compatibility_test(
            &store,
            mixed_transcript(),
            kafka_terms(),
            material_use(),
            session_authority("operator"),
        )
        .expect("historical v1 create");
    assert_eq!(durable.format_version(), 1);
    assert!(durable.bound_project_id().is_none());
    let session_id = durable.session_id().to_owned();
    let db_path = session_db(temp.path(), &session_id);
    assert!(
        !table_exists(&db_path, "human_raised_cases"),
        "historical v1 must not create human_raised_cases"
    );

    let target = durable.session().review_items()[0].target;
    let prepared = durable
        .prepare_human_decision(target, CorrectionDecision::Reject)
        .expect("prepare historical detector reject");
    durable
        .record_human_decision(prepared)
        .expect("historical detector reject remains valid");
    assert_eq!(
        durable.raise_and_manual_replace(0, 10, 18, "PostgreSQL"),
        Err(SessionPersistenceError::Replay(
            ApplicationServiceError::HumanRaisedRequiresFormatV3
        ))
    );
    durable.close().expect("close");

    let mut reopened = DurableApplicationSession::open(&store, &session_id, OpenMode::Writable)
        .expect("reopen historical v1");
    assert_eq!(reopened.format_version(), 1);
    assert!(
        !table_exists(&db_path, "human_raised_cases"),
        "opening historical v1 must not add human_raised_cases"
    );
    assert_eq!(reopened.session().review_ledger().events().len(), 1);
    assert_eq!(
        reopened.raise_and_manual_replace(0, 10, 18, "PostgreSQL"),
        Err(SessionPersistenceError::Replay(
            ApplicationServiceError::HumanRaisedRequiresFormatV3
        ))
    );
}

#[test]
fn historical_v2_bound_session_keeps_reuse_and_refuses_human_raised() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());
    let project = project_store.create("Lecture").expect("project");
    let project_id = project.project_id().clone();
    project.close().expect("close project");
    let mut durable =
        DurableApplicationSession::create_historical_bound_format_v2_for_compatibility_test(
            &session_store,
            &project_store,
            &project_id,
            mixed_transcript(),
            kafka_terms(),
            material_use(),
            session_authority("operator"),
        )
        .expect("historical v2 create");
    assert_eq!(durable.format_version(), 2);
    assert_eq!(durable.bound_project_id(), Some(&project_id));
    let session_id = durable.session_id().to_owned();
    let db_path = session_db(temp.path(), &session_id);
    assert!(
        !table_exists(&db_path, "human_raised_cases"),
        "historical v2 must not create human_raised_cases"
    );

    freeze_bound(&mut durable);
    assert!(durable.session().compose_project_reuse_proposals());
    let target = durable.session().review_items()[0].target;
    let prepared = durable
        .prepare_human_decision(target, CorrectionDecision::Reject)
        .expect("prepare historical detector reject");
    durable
        .record_human_decision(prepared)
        .expect("historical detector reject remains valid on v2");
    assert_eq!(
        durable.raise_and_manual_replace(0, 10, 18, "PostgreSQL"),
        Err(SessionPersistenceError::Replay(
            ApplicationServiceError::HumanRaisedRequiresFormatV3
        ))
    );
    durable.close().expect("close");

    let mut reopened =
        DurableApplicationSession::open(&session_store, &session_id, OpenMode::Writable)
            .expect("reopen historical v2");
    assert_eq!(reopened.format_version(), 2);
    assert_eq!(reopened.bound_project_id(), Some(&project_id));
    assert!(reopened.session().compose_project_reuse_proposals());
    assert!(
        !table_exists(&db_path, "human_raised_cases"),
        "opening historical v2 must not add human_raised_cases"
    );
    assert_eq!(
        reopened.raise_and_manual_replace(0, 10, 18, "PostgreSQL"),
        Err(SessionPersistenceError::Replay(
            ApplicationServiceError::HumanRaisedRequiresFormatV3
        ))
    );
}

#[test]
fn human_raised_rejects_byte_range_inside_multibyte_character() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\n你好").expect("cjk transcript");
    let source_text = transcript.segments()[0].text().to_owned();
    assert_eq!(source_text, "你好");
    assert_eq!(source_text.len(), 6);
    assert!(!source_text.is_char_boundary(1));

    let mut durable = DurableApplicationSession::create(
        &store,
        transcript,
        Vec::new(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    let before = durable
        .session()
        .materialize_reviewed_output()
        .expect("before");
    assert!(before.srt.contains("你好"));

    let error = durable
        .raise_and_manual_replace(0, 1, 3, "Hello")
        .err()
        .expect("mid-character range must fail");
    assert!(
        matches!(
            error,
            SessionPersistenceError::Replay(ApplicationServiceError::HumanRaisedAnchorInvalid(_))
        ),
        "unexpected error: {error:?}"
    );
    assert!(durable.session().human_raised_cases().is_empty());
    assert!(durable.session().review_ledger().events().is_empty());
    let after = durable
        .session()
        .materialize_reviewed_output()
        .expect("after failed raise");
    assert_eq!(after.srt, before.srt);

    let session_id = durable.session_id().to_owned();
    durable.close().expect("close");
    let reopened =
        DurableApplicationSession::open(&store, &session_id, OpenMode::ReadOnly).expect("reopen");
    assert!(reopened.session().human_raised_cases().is_empty());
    assert!(reopened.session().review_ledger().events().is_empty());
    let reopened_srt = reopened
        .session()
        .materialize_reviewed_output()
        .expect("reopened output");
    assert_eq!(reopened_srt.srt, before.srt);
}

#[test]
fn overlapping_human_raised_create_is_refused_before_persist() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = DurableApplicationSession::create(
        &store,
        mixed_transcript(),
        kafka_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    durable
        .raise_and_manual_replace(0, 10, 18, "PostgreSQL")
        .expect("first");
    assert_eq!(
        durable.raise_and_manual_replace(0, 14, 18, "SQL"),
        Err(SessionPersistenceError::Replay(
            ApplicationServiceError::HumanRaisedOverlap
        ))
    );
    assert_eq!(durable.session().human_raised_cases().len(), 1);
    assert_eq!(durable.session().review_ledger().events().len(), 2);
}

#[test]
fn human_raised_promotion_bumps_project_memory_format_and_proposes_on_material_b() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());
    let project = project_store.create("Lecture").expect("project");
    let project_id = project.project_id().clone();
    project.close().expect("close project");

    let mut session_a = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &project_id,
        mixed_transcript(),
        kafka_terms(),
        material_use(),
        session_authority("operator-a"),
    )
    .expect("session a");
    freeze_bound(&mut session_a);
    session_a
        .raise_and_manual_replace(0, 10, 18, "PostgreSQL")
        .expect("raise");
    let candidates = session_a.session().reuse_candidates().expect("candidates");
    let human_candidate = candidates
        .iter()
        .find(|candidate| candidate.key.source_locator.is_human_raised())
        .expect("human candidate")
        .clone();
    let prepared = session_a
        .prepare_accept_reuse_candidate(&human_candidate.key)
        .expect("prepare accept");
    session_a
        .record_accept_reuse_candidate(prepared)
        .expect("accept");
    session_a.close().expect("close a");

    let project = project_store
        .open(&project_id, ProjectMemoryOpenMode::ReadOnly)
        .expect("open project");
    assert_eq!(project.format_version(), PROJECT_MEMORY_FORMAT_VERSION_V3);
    project.close().expect("close project");

    let mut session_b = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &project_id,
        parse_srt("1\n00:00:00,000 --> 00:00:01,000\nHello Postgres").expect("b"),
        Vec::new(),
        material_use(),
        session_authority("operator-b"),
    )
    .expect("session b");
    freeze_bound(&mut session_b);
    let items = session_b.session().review_items();
    assert!(
        items.iter().any(|item| {
            item.review_case
                .as_detector_span()
                .is_none()
                || matches!(
                    item.target,
                    vox_proof::application_service::ApplicationReviewTarget::ProjectReuseProposal {
                        ..
                    }
                )
        }),
        "material B should expose the exact Postgres→PostgreSQL reuse proposal"
    );
    let reuse_item = items
        .iter()
        .find(|item| {
            matches!(
                item.target,
                vox_proof::application_service::ApplicationReviewTarget::ProjectReuseProposal { .. }
            )
        })
        .expect("reuse proposal");
    assert!(
        reuse_item
            .review_case
            .candidate_span()
            .anchor()
            .start_byte()
            >= 6
    );
}

#[test]
fn detector_only_promotion_writes_explicit_effects_and_bumps_project_memory_format_three() {
    let temp = TempDir::new().expect("tempdir");
    let session_store = ProductSessionStore::new(temp.path());
    let project_store = ProductProjectMemoryStore::new(temp.path());
    let project = project_store.create("Lecture").expect("project");
    let project_id = project.project_id().clone();
    project.close().expect("close project");

    let mut session_a = DurableApplicationSession::create_bound_to_project(
        &session_store,
        &project_store,
        &project_id,
        parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("a"),
        kafka_terms(),
        material_use(),
        session_authority("operator-a"),
    )
    .expect("session a");
    freeze_bound(&mut session_a);
    let target = session_a.session().review_items()[0].target;
    let prepared = session_a
        .prepare_manual_replacement(target, "Kafka")
        .expect("prepare MR");
    session_a
        .record_manual_replacement(prepared)
        .expect("record MR");
    let candidate = session_a.session().reuse_candidates().expect("candidates")[0].clone();
    assert!(!candidate.key.source_locator.is_human_raised());
    let prepared = session_a
        .prepare_accept_reuse_candidate(&candidate.key)
        .expect("prepare accept");
    session_a
        .record_accept_reuse_candidate(prepared)
        .expect("accept");
    session_a.close().expect("close a");

    let project = project_store
        .open(&project_id, ProjectMemoryOpenMode::ReadOnly)
        .expect("open project");
    assert_eq!(project.format_version(), PROJECT_MEMORY_FORMAT_VERSION_V3);
    project.close().expect("close project");
}

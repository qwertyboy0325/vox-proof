use std::fs;
use std::sync::{Arc, Barrier};
use std::thread;

use tempfile::TempDir;
use vox_proof::application_service::{
    ApplicationMaterialUseDeclaration, DeclaredApplicationMaterialUseBasis,
    DeclaredSessionAuthority, DeclaredSessionOperatorRole,
};
use vox_proof::candidate::{DetectionKind, Evidence, SessionTermEntry};
use vox_proof::review::{CorrectionDecision, ReviewLedgerEvent};
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

fn phonetic_transcript() -> vox_proof::transcript::Transcript {
    parse_srt("1\n00:00:00,000 --> 00:00:01,000\nASIS").expect("transcript")
}

fn phonetic_terms() -> Vec<SessionTermEntry> {
    vec![SessionTermEntry::new("ASUS", vec![], vec![])]
}

fn phonetic_review_case(
    session: &vox_proof::application_service::ApplicationReviewSession,
) -> vox_proof::review::ReviewCase {
    session
        .review_items()
        .into_iter()
        .find(|item| item.review_case.candidate_span().kind() == DetectionKind::PhoneticSimilarity)
        .expect("phonetic review case")
        .review_case
}

#[test]
fn create_and_reopen_read_only_preserves_canonical_authority() {
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
    let session_id = durable.session_id().to_owned();
    assert_eq!(durable.session().review_ledger().events().len(), 0);
    durable.close().expect("close");

    let reopened = DurableApplicationSession::open(&store, &session_id, OpenMode::ReadOnly)
        .expect("reopen read-only");
    assert_eq!(reopened.session().review_ledger().events().len(), 0);
    reopened
        .session()
        .verify_in_memory_replay()
        .expect("replay parity");
}

#[test]
fn list_session_summaries_returns_created_session_metadata() {
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
    let session_id = durable.session_id().to_owned();
    durable.close().expect("close");

    let summaries = store.list_session_summaries().expect("summaries");
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].session_id, session_id);
    assert_eq!(summaries[0].authority_display_label, "operator");
    assert_eq!(summaries[0].review_case_count, 1);
}

#[test]
fn durable_human_decision_survives_close_and_reopen() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = DurableApplicationSession::create(
        &store,
        fixture_transcript(),
        fixture_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    let target = durable.session().review_items()[0].target;
    let prepared = durable
        .prepare_human_decision(
            target,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .expect("prepare");
    durable
        .record_human_decision(prepared)
        .expect("durable decision");
    let session_id = durable.session_id().to_owned();
    durable.close().expect("close");

    let reopened =
        DurableApplicationSession::open(&store, &session_id, OpenMode::Writable).expect("reopen");
    assert_eq!(reopened.session().review_ledger().events().len(), 1);
    let projection = reopened
        .session()
        .derive_current_projection()
        .expect("projection");
    assert!(projection.srt.contains("Kafka"));
}

#[test]
fn durable_manual_replacement_survives_reopen() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = DurableApplicationSession::create(
        &store,
        fixture_transcript(),
        fixture_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    let target = durable.session().review_items()[0].target;
    let prepared = durable
        .prepare_manual_replacement(target, "華碩")
        .expect("prepare manual");
    durable
        .record_manual_replacement(prepared)
        .expect("durable manual");
    let session_id = durable.session_id().to_owned();
    durable.close().expect("close");
    let reopened =
        DurableApplicationSession::open(&store, &session_id, OpenMode::ReadOnly).expect("reopen");
    let output = reopened
        .session()
        .materialize_reviewed_output()
        .expect("reviewed output");
    assert!(output.srt.contains("華碩"));
}

#[test]
fn stale_review_ledger_head_rejects_without_mutation() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = DurableApplicationSession::create(
        &store,
        fixture_transcript(),
        fixture_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    let target = durable.session().review_items()[0].target;
    let mut prepared = durable
        .prepare_human_decision(
            target,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .expect("prepare");
    prepared.expected_review_ledger_head = 99;
    let error = durable
        .record_human_decision(prepared)
        .expect_err("stale head");
    assert!(matches!(
        error,
        SessionPersistenceError::StaleAuthorityPrecondition(precondition)
            if precondition.scope == AuthorityScope::ReviewLedger
    ));
    assert_eq!(durable.session().review_ledger().events().len(), 0);
}

#[test]
fn second_writable_open_is_rejected_until_close() {
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
    let session_id = durable.session_id().to_owned();
    assert!(matches!(
        DurableApplicationSession::open(&store, &session_id, OpenMode::Writable),
        Err(SessionPersistenceError::WriterOwnershipHeld)
    ));
    durable.close().expect("close");
    DurableApplicationSession::open(&store, &session_id, OpenMode::Writable)
        .expect("reopen writable");
}

#[test]
fn fail_before_commit_leaves_db_and_memory_unchanged() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = DurableApplicationSession::create(
        &store,
        fixture_transcript(),
        fixture_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    let session_id = durable.session_id().to_owned();
    let target = durable.session().review_items()[0].target;
    let prepared = durable
        .prepare_human_decision(
            target,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .expect("prepare");
    vox_proof::session_persistence::arm_fail_before_commit_for_test();
    let result = durable.record_human_decision(prepared);
    vox_proof::session_persistence::disarm_fail_before_commit_for_test();
    assert!(result.is_err());
    assert_eq!(durable.session().review_ledger().events().len(), 0);
    durable.close().expect("close");
    let reopened =
        DurableApplicationSession::open(&store, &session_id, OpenMode::ReadOnly).expect("reopen");
    assert_eq!(reopened.session().review_ledger().events().len(), 0);
}

#[test]
fn post_commit_refresh_failure_enters_recovery_required() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = DurableApplicationSession::create(
        &store,
        fixture_transcript(),
        fixture_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    let target = durable.session().review_items()[0].target;
    let prepared = durable
        .prepare_human_decision(
            target,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .expect("prepare");
    vox_proof::session_persistence::arm_force_hydrate_failure_for_test();
    let result = durable.record_human_decision(prepared);
    vox_proof::session_persistence::disarm_force_hydrate_failure_for_test();
    assert!(matches!(
        result,
        Err(SessionPersistenceError::RecoveryRequired)
    ));
    assert!(durable.is_recovery_required());
    let target = durable.session().review_items()[0].target;
    assert!(matches!(
        durable.prepare_human_decision(target, CorrectionDecision::Reject),
        Err(SessionPersistenceError::RecoveryRequired)
    ));
    durable.rehydrate().expect("rehydrate");
    assert!(!durable.is_recovery_required());
    assert_eq!(durable.session().review_ledger().events().len(), 1);
}

#[test]
fn export_v2_parity_after_reopen() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = DurableApplicationSession::create(
        &store,
        fixture_transcript(),
        fixture_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    let target = durable.session().review_items()[0].target;
    let prepared = durable
        .prepare_human_decision(
            target,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .expect("prepare");
    durable
        .record_human_decision(prepared)
        .expect("durable decision");
    let bundle_before = durable
        .session()
        .materialize_review_export_bundle()
        .expect("export");
    let session_id = durable.session_id().to_owned();
    durable.close().expect("close");
    let reopened =
        DurableApplicationSession::open(&store, &session_id, OpenMode::ReadOnly).expect("reopen");
    let bundle_after = reopened
        .session()
        .materialize_review_export_bundle()
        .expect("export");
    assert_eq!(bundle_before, bundle_after);
}

#[test]
fn unknown_format_version_fails_closed() {
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
    let session_id = durable.session_id().to_owned();
    let db_path = temp.path().join(&session_id).join("session.db");
    durable.close().expect("close");
    let connection = rusqlite::Connection::open(&db_path).expect("open db");
    connection
        .execute(
            "UPDATE session_metadata SET format_version = 99 WHERE session_id = ?1",
            [&session_id],
        )
        .expect("bump version");
    assert!(matches!(
        DurableApplicationSession::open(&store, &session_id, OpenMode::ReadOnly),
        Err(SessionPersistenceError::UnsupportedFormatVersion {
            found: 99,
            supported: 4
        })
    ));
}

#[test]
fn schema_skeleton_tables_exist_on_disk() {
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
    let session_id = durable.session_id().to_owned();
    let db_path = temp.path().join(&session_id).join("session.db");
    durable.close().expect("close");
    assert!(db_path.exists());
    let connection = rusqlite::Connection::open(&db_path).expect("open");
    for table in [
        "session_metadata",
        "source_payload",
        "session_terms",
        "declarations",
        "analysis_snapshots",
        "review_cases",
        "review_ledger_events",
        "project_scope",
        "reuse_governance_events",
        "reuse_enabled_bindings",
        "command_tokens",
        "authority_transitions",
        "writer_ownership",
        "human_raised_cases",
    ] {
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |row| row.get(0),
            )
            .expect("table");
        assert_eq!(count, 1, "missing {table}");
    }
    let _ = fs::read_dir(temp.path());
}

#[test]
fn tampered_persisted_review_case_fails_closed_on_reopen() {
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
    let session_id = durable.session_id().to_owned();
    let db_path = temp.path().join(&session_id).join("session.db");
    durable.close().expect("close");
    let connection = rusqlite::Connection::open(&db_path).expect("open db");
    connection
        .execute(
            "UPDATE review_cases SET case_json = json_set(case_json, '$.evidence.kind', 'tampered') WHERE session_id = ?1",
            [&session_id],
        )
        .expect("tamper case");
    drop(connection);
    assert!(matches!(
        DurableApplicationSession::open(&store, &session_id, OpenMode::ReadOnly),
        Err(SessionPersistenceError::CanonicalMismatch(_))
            | Err(SessionPersistenceError::ReviewCaseVerificationFailed)
    ));
}

#[test]
fn canonical_authority_round_trips_through_reopen() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let transcript = fixture_transcript();
    let terms = fixture_terms();
    let durable = DurableApplicationSession::create(
        &store,
        transcript,
        terms,
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    let before_revision = durable.session().source().revision_id();
    let before_terms = durable.session().reuse_parts().session_terms.to_vec();
    let before_items = durable.session().review_items();
    let session_id = durable.session_id().to_owned();
    durable.close().expect("close");
    let reopened =
        DurableApplicationSession::open(&store, &session_id, OpenMode::ReadOnly).expect("reopen");
    assert_eq!(reopened.session().source().revision_id(), before_revision);
    assert_eq!(
        reopened.session().reuse_parts().session_terms,
        before_terms.as_slice()
    );
    assert_eq!(reopened.session().review_items(), before_items);
}

#[test]
fn multiple_ledger_events_preserve_append_order() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = DurableApplicationSession::create(
        &store,
        fixture_transcript(),
        fixture_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    let target = durable.session().review_items()[0].target;
    let reject = durable
        .prepare_human_decision(target, CorrectionDecision::Reject)
        .expect("prepare reject");
    durable.record_human_decision(reject).expect("reject");
    let accept = durable
        .prepare_human_decision(
            target,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .expect("prepare accept");
    durable.record_human_decision(accept).expect("accept");
    let events = durable.session().review_ledger().events();
    assert_eq!(events.len(), 2);
    assert!(matches!(
        &events[0],
        ReviewLedgerEvent::DecisionRecorded {
            decision: CorrectionDecision::Reject,
            ..
        }
    ));
    assert!(matches!(
        &events[1],
        ReviewLedgerEvent::DecisionRecorded {
            decision: CorrectionDecision::AcceptAlternative {
                alternative_index: 0
            },
            ..
        }
    ));
}

#[test]
fn projection_parity_after_reopen() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = DurableApplicationSession::create(
        &store,
        fixture_transcript(),
        fixture_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    let target = durable.session().review_items()[0].target;
    let prepared = durable
        .prepare_human_decision(
            target,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .expect("prepare");
    durable
        .record_human_decision(prepared)
        .expect("durable decision");
    let projection_before = durable
        .session()
        .derive_current_projection()
        .expect("projection");
    let session_id = durable.session_id().to_owned();
    durable.close().expect("close");
    let reopened =
        DurableApplicationSession::open(&store, &session_id, OpenMode::ReadOnly).expect("reopen");
    let projection_after = reopened
        .session()
        .derive_current_projection()
        .expect("projection");
    assert_eq!(projection_before, projection_after);
}

#[test]
fn export_v3_parity_after_reopen_in_gate3_empty_state() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = DurableApplicationSession::create(
        &store,
        fixture_transcript(),
        fixture_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    let target = durable.session().review_items()[0].target;
    let prepared = durable
        .prepare_human_decision(
            target,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .expect("prepare");
    durable
        .record_human_decision(prepared)
        .expect("durable decision");
    let before = durable.session().materialize_review_export_bundle_v3();
    let before_err = before.as_ref().map_err(|error| error.to_string());
    let session_id = durable.session_id().to_owned();
    durable.close().expect("close");
    let reopened =
        DurableApplicationSession::open(&store, &session_id, OpenMode::ReadOnly).expect("reopen");
    let after = reopened.session().materialize_review_export_bundle_v3();
    let after_err = after.as_ref().map_err(|error| error.to_string());
    assert_eq!(before_err, after_err);
    assert!(before.is_err());
}

#[test]
fn physical_generation_is_not_semantic_stale_guard() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = DurableApplicationSession::create(
        &store,
        fixture_transcript(),
        fixture_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    let session_id = durable.session_id().to_owned();
    let db_path = temp.path().join(&session_id).join("session.db");
    let connection = rusqlite::Connection::open(&db_path).expect("open db");
    connection
        .execute(
            "INSERT INTO authority_transitions (session_id, generation, acknowledgement_status) VALUES (?1, 999, 'committed')",
            [&session_id],
        )
        .expect("bump generation");
    drop(connection);
    let target = durable.session().review_items()[0].target;
    let prepared = durable
        .prepare_human_decision(
            target,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .expect("prepare");
    durable
        .record_human_decision(prepared)
        .expect("commit despite unrelated generation");
    assert_eq!(durable.session().review_ledger().events().len(), 1);
}

#[test]
fn read_only_open_does_not_acquire_writer_or_mutate_ledger() {
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
    let session_id = durable.session_id().to_owned();
    durable.close().expect("close");
    let reopened =
        DurableApplicationSession::open(&store, &session_id, OpenMode::ReadOnly).expect("reopen");
    let target = reopened.session().review_items()[0].target;
    let prepared = reopened
        .prepare_human_decision(
            target,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .expect("prepare");
    assert!(matches!(
        DurableApplicationSession::open(&store, &session_id, OpenMode::Writable),
        Ok(_)
    ));
    drop(prepared);
    drop(reopened);
    let readonly_again =
        DurableApplicationSession::open(&store, &session_id, OpenMode::ReadOnly).expect("reopen");
    assert_eq!(readonly_again.session().review_ledger().events().len(), 0);
}

#[test]
fn detector_version_tamper_fails_closed_on_reopen() {
    tampered_analysis_snapshot_reopen_fails(
        "UPDATE analysis_snapshots SET snapshot_json = json_set(snapshot_json, '$.detectors[0].version', 'tampered') WHERE session_id = ?1",
    );
}

#[test]
fn detector_config_identity_tamper_fails_closed_on_reopen() {
    tampered_analysis_snapshot_reopen_fails(
        "UPDATE analysis_snapshots SET snapshot_json = json_set(snapshot_json, '$.detector_config.version', 'tampered') WHERE session_id = ?1",
    );
}

#[test]
fn algorithm_identity_tamper_fails_closed_on_reopen() {
    tampered_analysis_snapshot_reopen_fails(
        "UPDATE analysis_snapshots SET snapshot_json = json_set(snapshot_json, '$.algorithm.version', 'tampered') WHERE session_id = ?1",
    );
}

fn tampered_analysis_snapshot_reopen_fails(sql: &str) {
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
    let session_id = durable.session_id().to_owned();
    let db_path = temp.path().join(&session_id).join("session.db");
    durable.close().expect("close");
    let connection = rusqlite::Connection::open(&db_path).expect("open db");
    connection
        .execute(sql, [&session_id])
        .expect("tamper analysis snapshot");
    drop(connection);
    assert!(matches!(
        DurableApplicationSession::open(&store, &session_id, OpenMode::ReadOnly),
        Err(SessionPersistenceError::CanonicalMismatch(_))
    ));
}

#[test]
fn phonetic_review_case_round_trips_as_authoritative_authority() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let durable = DurableApplicationSession::create(
        &store,
        phonetic_transcript(),
        phonetic_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    let original_case = phonetic_review_case(durable.session());
    assert!(matches!(
        original_case.candidate_span().evidence(),
        Evidence::PhoneticSimilarity(_)
    ));
    let session_id = durable.session_id().to_owned();
    durable.close().expect("close");
    let reopened =
        DurableApplicationSession::open(&store, &session_id, OpenMode::ReadOnly).expect("reopen");
    let reopened_case = phonetic_review_case(reopened.session());
    assert_eq!(reopened_case, original_case);
}

#[test]
fn phonetic_review_case_decision_survives_reopen() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = DurableApplicationSession::create(
        &store,
        phonetic_transcript(),
        phonetic_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    let target = durable.session().review_items()[0].target;
    let prepared = durable
        .prepare_human_decision(
            target,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .expect("prepare");
    durable
        .record_human_decision(prepared)
        .expect("durable decision");
    let session_id = durable.session_id().to_owned();
    durable.close().expect("close");
    let reopened =
        DurableApplicationSession::open(&store, &session_id, OpenMode::ReadOnly).expect("reopen");
    let projection = reopened
        .session()
        .derive_current_projection()
        .expect("projection");
    assert!(projection.srt.contains("ASUS"));
}

#[test]
fn concurrent_writable_acquisition_allows_exactly_one_owner() {
    let temp = TempDir::new().expect("tempdir");
    let store_path = temp.path().to_path_buf();
    let store = ProductSessionStore::new(&store_path);
    let durable = DurableApplicationSession::create(
        &store,
        fixture_transcript(),
        fixture_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    let session_id = durable.session_id().to_owned();
    durable.close().expect("close");

    let barrier = Arc::new(Barrier::new(2));
    let session_id_a = session_id.clone();
    let session_id_b = session_id.clone();
    let path_a = store_path.clone();
    let path_b = store_path;
    let barrier_a = Arc::clone(&barrier);
    let barrier_b = Arc::clone(&barrier);
    let handle_a = thread::spawn(move || {
        barrier_a.wait();
        let store = ProductSessionStore::new(&path_a);
        DurableApplicationSession::open(&store, &session_id_a, OpenMode::Writable).is_ok()
    });
    let handle_b = thread::spawn(move || {
        barrier_b.wait();
        let store = ProductSessionStore::new(&path_b);
        DurableApplicationSession::open(&store, &session_id_b, OpenMode::Writable).is_ok()
    });
    let ok_a = handle_a.join().expect("join a");
    let ok_b = handle_b.join().expect("join b");
    assert_ne!(ok_a, ok_b);
    assert!(ok_a || ok_b);
}

#[test]
fn authoritative_transaction_rejects_mismatched_writer_token_without_mutation() {
    let temp = TempDir::new().expect("tempdir");
    let store = ProductSessionStore::new(temp.path());
    let mut durable = DurableApplicationSession::create(
        &store,
        fixture_transcript(),
        fixture_terms(),
        material_use(),
        session_authority("operator"),
    )
    .expect("create");
    let session_id = durable.session_id().to_owned();
    let db_path = temp.path().join(&session_id).join("session.db");
    let connection = rusqlite::Connection::open(&db_path).expect("open db");
    connection
        .execute(
            "UPDATE writer_ownership SET writer_token = 'foreign-token' WHERE session_id = ?1",
            [&session_id],
        )
        .expect("tamper writer token");
    drop(connection);
    let target = durable.session().review_items()[0].target;
    let prepared = durable
        .prepare_human_decision(
            target,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .expect("prepare");
    assert!(matches!(
        durable.record_human_decision(prepared),
        Err(SessionPersistenceError::WriterOwnershipHeld)
    ));
    assert_eq!(durable.session().review_ledger().events().len(), 0);
    durable
        .close()
        .expect_err("stale handle cannot release foreign token");
}

#[test]
fn release_only_clears_own_writer_token() {
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
    let session_id = durable.session_id().to_owned();
    durable.close().expect("close");
    let first =
        DurableApplicationSession::open(&store, &session_id, OpenMode::Writable).expect("first");
    first.close().expect("release first");
    DurableApplicationSession::open(&store, &session_id, OpenMode::Writable)
        .expect("second acquire");
}

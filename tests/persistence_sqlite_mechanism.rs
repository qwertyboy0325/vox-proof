#![cfg(feature = "persistence-spike")]

use rusqlite::Connection;
use vox_proof::persistence_evidence::candidates::fault::FaultPoint;
use vox_proof::persistence_evidence::candidates::semantic_ops::{
    apply_command, sample_append_event,
};
use vox_proof::persistence_evidence::{
    AuthoritativeCommand, EmbeddedRelationalAdapter, EvidenceFixture, PersistenceCandidateAdapter,
    SemanticOpenMode, SemanticOracle, SemanticPrecondition, fresh_storage_root,
};

fn op_id(label: &str) -> String {
    format!("bt-op:{label}")
}

fn db_path(locator: &str) -> String {
    format!("{locator}/session.db")
}

fn count_ledger_events(locator: &str) -> usize {
    let connection = Connection::open(db_path(locator)).expect("open db");
    connection
        .query_row("SELECT COUNT(*) FROM review_ledger_events", [], |row| {
            row.get(0)
        })
        .expect("count")
}

fn applied_status(locator: &str, command_operation_id: &str) -> Option<String> {
    let connection = Connection::open(db_path(locator)).expect("open db");
    connection
        .query_row(
            "SELECT outcome_status FROM applied_authoritative_commands WHERE command_operation_id = ?1",
            [command_operation_id],
            |row| row.get(0),
        )
        .ok()
}

fn writer_token(locator: &str) -> Option<String> {
    let connection = Connection::open(db_path(locator)).expect("open db");
    connection
        .query_row(
            "SELECT token FROM writer_ownership WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .ok()
}

fn append_command(
    state: &vox_proof::persistence_evidence::NormalizedSemanticState,
    operation_id: &str,
) -> AuthoritativeCommand {
    let event = sample_append_event(state);
    AuthoritativeCommand::AppendCorrectionEvent {
        command_operation_id: operation_id.to_string(),
        event,
        preconditions: vec![SemanticPrecondition::ReviewLedgerHead {
            expected_event_id: state
                .review_ledger_events
                .last()
                .map(|event| event.event_id.clone()),
        }],
    }
}

#[test]
fn bt_001_relational_create_close_reopen() {
    let fixture = EvidenceFixture::small();
    let root = fresh_storage_root("bt-001");
    let mut adapter = EmbeddedRelationalAdapter::new(root.clone());
    let session = adapter.create(&fixture).expect("create");
    let handle = adapter
        .open(&session, SemanticOpenMode::ReadOnly)
        .expect("open read-only");
    adapter.close(&handle).expect("close");

    let mut reopened = EmbeddedRelationalAdapter::new(root);
    let handle = reopened
        .open(&session, SemanticOpenMode::ReadOnly)
        .expect("reopen");
    let actual = reopened.read_normalized_state(&handle).expect("read state");
    assert!(SemanticOracle::compare(&fixture.normalized_state(), &actual).passed);
}

#[test]
fn bt_002a_committed_only_retry_reconciliation() {
    let fixture = EvidenceFixture::small();
    let mut adapter = EmbeddedRelationalAdapter::new(fresh_storage_root("bt-002a"));
    let session = adapter.create(&fixture).expect("create");
    let handle = adapter
        .open(&session, SemanticOpenMode::Writable)
        .expect("open");
    let before = adapter.read_normalized_state(&handle).expect("state");
    let command = append_command(&before, &op_id("002a"));
    adapter.set_force_post_commit_verify_failure(true);
    let error = adapter
        .apply_authoritative_command(&handle, &command)
        .expect_err("verify failure");
    assert_eq!(error.code, "post-commit-verify-failed");
    assert_eq!(
        applied_status(session.adapter_locator(), &op_id("002a")),
        Some("committed".to_string())
    );
    assert_eq!(
        count_ledger_events(session.adapter_locator()),
        before.review_ledger_events.len() + 1
    );

    adapter.set_force_post_commit_verify_failure(false);
    adapter
        .apply_authoritative_command(&handle, &command)
        .expect("reconcile");
    assert_eq!(
        applied_status(session.adapter_locator(), &op_id("002a")),
        Some("acknowledged".to_string())
    );
    assert_eq!(
        count_ledger_events(session.adapter_locator()),
        before.review_ledger_events.len() + 1
    );
}

#[test]
fn bt_003_command_ack_after_commit_verify_and_acknowledge() {
    let fixture = EvidenceFixture::small();
    let root = fresh_storage_root("bt-003");
    let mut adapter = EmbeddedRelationalAdapter::new(root.clone());
    let session = adapter.create(&fixture).expect("create");
    let handle = adapter
        .open(&session, SemanticOpenMode::Writable)
        .expect("open");
    let before = adapter.read_normalized_state(&handle).expect("state");
    let command = append_command(&before, &op_id("003"));
    adapter
        .apply_authoritative_command(&handle, &command)
        .expect("apply");
    assert_eq!(
        applied_status(session.adapter_locator(), &op_id("003")),
        Some("acknowledged".to_string())
    );
    assert_eq!(
        count_ledger_events(session.adapter_locator()),
        before.review_ledger_events.len() + 1
    );
    adapter.close(&handle).expect("close");

    let mut reopened = EmbeddedRelationalAdapter::new(root);
    let handle = reopened
        .open(&session, SemanticOpenMode::ReadOnly)
        .expect("reopen");
    let after = reopened.read_normalized_state(&handle).expect("after");
    let mut expected = before.clone();
    apply_command(&mut expected, &command).expect("oracle apply");
    assert!(SemanticOracle::compare(&expected, &after).passed);
}

#[test]
fn bt_004_second_writer_rejected() {
    let fixture = EvidenceFixture::small();
    let mut adapter = EmbeddedRelationalAdapter::new(fresh_storage_root("bt-004"));
    let session = adapter.create(&fixture).expect("create");
    let first = adapter
        .open(&session, SemanticOpenMode::Writable)
        .expect("first writer");
    let second = adapter.open(&session, SemanticOpenMode::Writable);
    assert!(matches!(second, Err(error) if error.code == "writer-already-open"));
    adapter.close(&first).expect("close first");
}

#[test]
fn bt_005_stale_takeover_after_lease_expiry() {
    let fixture = EvidenceFixture::small();
    let root = fresh_storage_root("bt-005");
    let mut first = EmbeddedRelationalAdapter::new(root.clone());
    first.set_test_clock_ms(Some(1_000));
    let session = first.create(&fixture).expect("create");
    let first_handle = first
        .open(&session, SemanticOpenMode::Writable)
        .expect("open first");

    let mut second = EmbeddedRelationalAdapter::new(root);
    second.set_test_clock_ms(Some(60_000));
    let second_handle = second
        .open(&session, SemanticOpenMode::Writable)
        .expect("stale takeover");
    assert_eq!(second_handle.mode, SemanticOpenMode::Writable);
    let _ = first_handle;
}

#[test]
fn bt_006_old_handle_rejected_after_epoch_takeover() {
    let fixture = EvidenceFixture::small();
    let root = fresh_storage_root("bt-006");
    let mut first = EmbeddedRelationalAdapter::new(root.clone());
    first.set_test_clock_ms(Some(1_000));
    let session = first.create(&fixture).expect("create");
    let old_handle = first
        .open(&session, SemanticOpenMode::Writable)
        .expect("open old");

    let mut second = EmbeddedRelationalAdapter::new(root);
    second.set_test_clock_ms(Some(60_000));
    let _new_handle = second
        .open(&session, SemanticOpenMode::Writable)
        .expect("takeover");
    let before = first.read_normalized_state(&old_handle).expect("state");
    let command = append_command(&before, &op_id("006"));
    let error = first
        .apply_authoritative_command(&old_handle, &command)
        .expect_err("old epoch");
    assert_eq!(error.code, "writer-epoch-mismatch");
}

#[test]
fn bt_007_format_rejection_before_ownership() {
    let fixture = EvidenceFixture::small();
    let mut adapter = EmbeddedRelationalAdapter::new(fresh_storage_root("bt-007"));
    let session = adapter.create(&fixture).expect("create");
    let connection = Connection::open(db_path(session.adapter_locator())).expect("open");
    connection
        .execute(
            "UPDATE session_meta SET value = ?1 WHERE key = 'format_version'",
            ["2".to_string()],
        )
        .expect("bump format");
    let before_token = writer_token(session.adapter_locator());
    let newer = adapter.open(&session, SemanticOpenMode::Writable);
    assert!(matches!(newer, Err(error) if error.code == "unsupported-newer-format"));
    assert_eq!(writer_token(session.adapter_locator()), before_token);

    connection
        .execute(
            "UPDATE session_meta SET value = '0' WHERE key = 'format_version'",
            [],
        )
        .expect("older format");
    let older = adapter.open(&session, SemanticOpenMode::Writable);
    assert!(matches!(older, Err(error) if error.code == "unsupported-older-format"));
}

#[test]
fn bt_008_online_backup_duplication_distinct_identity() {
    let fixture = EvidenceFixture::small();
    let mut adapter = EmbeddedRelationalAdapter::new(fresh_storage_root("bt-008"));
    let session = adapter.create(&fixture).expect("create");
    let handle = adapter
        .open(&session, SemanticOpenMode::Writable)
        .expect("open");
    let duplicate_id = format!("{}:dup", fixture.normalized_state().session.session_id);
    let duplicate = adapter
        .duplicate_session(&handle, &duplicate_id)
        .expect("duplicate");
    assert_eq!(duplicate.session.session_id, duplicate_id);
    assert_eq!(
        duplicate
            .normalized_state
            .session
            .duplicated_from_session_id
            .as_deref(),
        Some(fixture.normalized_state().session.session_id.as_str())
    );
    assert_eq!(writer_token(duplicate.session.adapter_locator()), None);
}

#[test]
fn bt_009_source_unchanged_after_duplication() {
    let fixture = EvidenceFixture::small();
    let root = fresh_storage_root("bt-009");
    let mut adapter = EmbeddedRelationalAdapter::new(root.clone());
    let session = adapter.create(&fixture).expect("create");
    let handle = adapter
        .open(&session, SemanticOpenMode::Writable)
        .expect("open");
    let before = adapter
        .read_normalized_state(&handle)
        .expect("source state");
    let duplicate_id = format!("{}:dup", fixture.normalized_state().session.session_id);
    adapter
        .duplicate_session(&handle, &duplicate_id)
        .expect("duplicate");
    let mut verify = EmbeddedRelationalAdapter::new(root);
    let reopened = verify
        .open(&session, SemanticOpenMode::ReadOnly)
        .expect("reopen source");
    let after = verify
        .read_normalized_state(&reopened)
        .expect("source after");
    assert!(SemanticOracle::compare(&before, &after).passed);
}

#[test]
fn bt_010_derived_cache_rebuild() {
    let fixture = EvidenceFixture::small();
    let mut adapter = EmbeddedRelationalAdapter::new(fresh_storage_root("bt-010"));
    let session = adapter.create(&fixture).expect("create");
    let connection = Connection::open(db_path(session.adapter_locator())).expect("open");
    connection
        .execute(
            "UPDATE derived_cache SET value = 'corrupted', content_hash = 'bad' WHERE key = 'queue-index-v1'",
            [],
        )
        .expect("corrupt derived");
    let handle = adapter
        .open(&session, SemanticOpenMode::ReadOnly)
        .expect("open");
    let actual = adapter
        .read_normalized_state(&handle)
        .expect("canonical still readable");
    assert!(SemanticOracle::compare(&fixture.normalized_state(), &actual).passed);
    let (value, hash): (String, String) = connection
        .query_row(
            "SELECT value, content_hash FROM derived_cache WHERE key = 'queue-index-v1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("derived row");
    assert_ne!(value, "corrupted");
    assert_ne!(hash, "bad");
}

#[test]
fn bt_011_canonical_corruption_fails_closed() {
    let fixture = EvidenceFixture::small();
    let mut adapter = EmbeddedRelationalAdapter::new(fresh_storage_root("bt-011"));
    let session = adapter.create(&fixture).expect("create");
    let connection = Connection::open(db_path(session.adapter_locator())).expect("open");
    connection
        .execute(
            "UPDATE review_cases SET payload_json = '{invalid' WHERE case_id = (SELECT case_id FROM review_cases LIMIT 1)",
            [],
        )
        .expect("corrupt");
    let result = adapter.open(&session, SemanticOpenMode::Writable);
    assert!(matches!(result, Err(error) if error.code == "canonical-corruption"));
}

#[test]
fn bt_012_pre_commit_fault_leaves_state_unchanged() {
    let fixture = EvidenceFixture::small();
    let root = fresh_storage_root("bt-012");
    let mut adapter = EmbeddedRelationalAdapter::new(root.clone());
    let session = adapter.create(&fixture).expect("create");
    let handle = adapter
        .open(&session, SemanticOpenMode::Writable)
        .expect("open");
    let before = adapter.read_normalized_state(&handle).expect("state");
    adapter.arm_test_fault(FaultPoint::BeforeSqliteCommit);
    let command = append_command(&before, &op_id("012"));
    let error = adapter
        .apply_authoritative_command(&handle, &command)
        .expect_err("fault");
    assert_eq!(error.code, "simulated-durability-failure");
    adapter.close(&handle).expect("close");
    let mut reopened = EmbeddedRelationalAdapter::new(root);
    let handle = reopened
        .open(&session, SemanticOpenMode::ReadOnly)
        .expect("reopen");
    let after = reopened.read_normalized_state(&handle).expect("after");
    assert!(SemanticOracle::compare(&before, &after).passed);
}

#[test]
fn bt_013_same_operation_id_same_fingerprint_idempotent() {
    let fixture = EvidenceFixture::small();
    let mut adapter = EmbeddedRelationalAdapter::new(fresh_storage_root("bt-013"));
    let session = adapter.create(&fixture).expect("create");
    let handle = adapter
        .open(&session, SemanticOpenMode::Writable)
        .expect("open");
    let before = adapter.read_normalized_state(&handle).expect("state");
    let command = append_command(&before, &op_id("013"));
    adapter
        .apply_authoritative_command(&handle, &command)
        .expect("first");
    adapter
        .apply_authoritative_command(&handle, &command)
        .expect("second");
    assert_eq!(
        count_ledger_events(session.adapter_locator()),
        before.review_ledger_events.len() + 1
    );
}

#[test]
fn bt_014_same_operation_id_different_payload_fails_closed() {
    let fixture = EvidenceFixture::small();
    let mut adapter = EmbeddedRelationalAdapter::new(fresh_storage_root("bt-014"));
    let session = adapter.create(&fixture).expect("create");
    let handle = adapter
        .open(&session, SemanticOpenMode::Writable)
        .expect("open");
    let before = adapter.read_normalized_state(&handle).expect("state");
    let first = append_command(&before, &op_id("014"));
    adapter
        .apply_authoritative_command(&handle, &first)
        .expect("first");
    let after = adapter.read_normalized_state(&handle).expect("after first");
    let mut different_event = sample_append_event(&after);
    different_event.event_id = "ledger-event:999".to_string();
    different_event.sequence = 999;
    let second = AuthoritativeCommand::AppendCorrectionEvent {
        command_operation_id: op_id("014"),
        event: different_event,
        preconditions: vec![SemanticPrecondition::ReviewLedgerHead {
            expected_event_id: after
                .review_ledger_events
                .last()
                .map(|event| event.event_id.clone()),
        }],
    };
    let error = adapter
        .apply_authoritative_command(&handle, &second)
        .expect_err("mismatch");
    assert_eq!(error.code, "command-operation-id-mismatch");
}

#[test]
fn bt_015_busy_does_not_bypass_epoch_validation() {
    let fixture = EvidenceFixture::small();
    let root = fresh_storage_root("bt-015");
    let mut first = EmbeddedRelationalAdapter::new(root.clone());
    first.set_test_clock_ms(Some(1_000));
    let session = first.create(&fixture).expect("create");
    let old_handle = first
        .open(&session, SemanticOpenMode::Writable)
        .expect("open old");

    let mut second = EmbeddedRelationalAdapter::new(root);
    second.set_test_clock_ms(Some(60_000));
    let _ = second
        .open(&session, SemanticOpenMode::Writable)
        .expect("takeover");

    first.set_force_sqlite_busy_on_tx_begin_once(true);
    let before = first.read_normalized_state(&old_handle).expect("state");
    let command = append_command(&before, &op_id("015"));
    let busy_error = first
        .apply_authoritative_command(&old_handle, &command)
        .expect_err("busy before transaction");
    assert_eq!(busy_error.code, "sqlite-busy");
    let epoch_error = first
        .apply_authoritative_command(&old_handle, &command)
        .expect_err("epoch mismatch after busy clears");
    assert_eq!(epoch_error.code, "writer-epoch-mismatch");
}

#[test]
fn bt_016_post_commit_verify_failure_reconciles_without_remutation() {
    let fixture = EvidenceFixture::small();
    let mut adapter = EmbeddedRelationalAdapter::new(fresh_storage_root("bt-016"));
    let session = adapter.create(&fixture).expect("create");
    let handle = adapter
        .open(&session, SemanticOpenMode::Writable)
        .expect("open");
    let before = adapter.read_normalized_state(&handle).expect("state");
    let command = append_command(&before, &op_id("016"));
    adapter.set_force_post_commit_verify_failure(true);
    let error = adapter
        .apply_authoritative_command(&handle, &command)
        .expect_err("verify failure");
    assert_eq!(error.code, "post-commit-verify-failed");
    assert_eq!(
        applied_status(session.adapter_locator(), &op_id("016")),
        Some("committed".to_string())
    );
    assert_eq!(
        count_ledger_events(session.adapter_locator()),
        before.review_ledger_events.len() + 1
    );
    adapter.set_force_post_commit_verify_failure(false);
    adapter
        .apply_authoritative_command(&handle, &command)
        .expect("reconcile");
    assert_eq!(
        applied_status(session.adapter_locator(), &op_id("016")),
        Some("acknowledged".to_string())
    );
    assert_eq!(
        count_ledger_events(session.adapter_locator()),
        before.review_ledger_events.len() + 1
    );
}

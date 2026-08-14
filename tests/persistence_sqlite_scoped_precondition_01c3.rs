#![cfg(feature = "persistence-spike")]

use std::time::{SystemTime, UNIX_EPOCH};

use vox_proof::persistence_evidence::current_contract::evidence_01c::measurement_transitions::{
    measurement_transition_states, unrelated_scope_success_fixture,
};
use vox_proof::persistence_evidence::current_contract::evidence_01c::scenario_observation::{
    Fcr03StaleRejectionObservation, Fcr03UnrelatedSuccessObservation,
};
use vox_proof::persistence_evidence::current_contract::sqlite_authoritative_01c3::{
    reject_full_state_authority_replace, SqliteCommandScope, SqliteScopedCommand,
    SqliteScopedPreconditionCandidateAdapter, SQLITE_SCOPED_PRECONDITION_CANDIDATE_VERSION,
};
use vox_proof::persistence_evidence::current_contract::finalize_derived_fields;
use vox_proof::persistence_evidence::{
    CurrentContractOracle, MeasurementFixtureScale, SqliteOpenMode,
    SQLITE_AUTHORITATIVE_CANDIDATE_VERSION,
};

fn new_adapter(label: &str) -> SqliteScopedPreconditionCandidateAdapter {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    SqliteScopedPreconditionCandidateAdapter::new(std::env::temp_dir().join(format!(
        "voxproof-01c3-sqlite-{label}-{nonce}"
    )))
    .expect("candidate root")
}

fn open_writer(
    adapter: &SqliteScopedPreconditionCandidateAdapter,
    state: &vox_proof::persistence_evidence::CurrentContractState,
) -> vox_proof::persistence_evidence::OpenedSqliteAuthoritySession {
    let session = adapter.create(state).expect("create");
    adapter
        .open_existing(session.session_id(), SqliteOpenMode::Writable)
        .expect("open writer")
}

fn expected_merged_review(
    reuse_advanced: &vox_proof::persistence_evidence::CurrentContractState,
    review_target: &vox_proof::persistence_evidence::CurrentContractState,
) -> vox_proof::persistence_evidence::CurrentContractState {
    let mut expected = reuse_advanced.clone();
    expected.review_ledger_events = review_target.review_ledger_events.clone();
    expected.effective_review_status = review_target.effective_review_status.clone();
    expected.durable_command_tokens.review_ledger_head =
        review_target.durable_command_tokens.review_ledger_head;
    finalize_derived_fields(&mut expected);
    expected.normalize()
}

fn analysis_only_target_from(
    precursor: &vox_proof::persistence_evidence::CurrentContractState,
    attachment_source: &vox_proof::persistence_evidence::CurrentContractState,
) -> vox_proof::persistence_evidence::CurrentContractState {
    let mut target = precursor.clone();
    for snapshot in &attachment_source.analysis_snapshots {
        if !target
            .analysis_snapshots
            .iter()
            .any(|existing| existing.identity == snapshot.identity)
        {
            target.analysis_snapshots.push(snapshot.clone());
        }
    }
    target.durable_command_tokens.active_analysis_snapshot_identity = attachment_source
        .durable_command_tokens
        .active_analysis_snapshot_identity
        .clone();
    finalize_derived_fields(&mut target);
    target.normalize()
}

#[test]
fn sqlite_01c3_candidate_version_is_distinct_from_01c_sqlite_2() {
    assert_eq!(SQLITE_AUTHORITATIVE_CANDIDATE_VERSION, "01C-SQLITE-2");
    assert_eq!(SQLITE_SCOPED_PRECONDITION_CANDIDATE_VERSION, "01C-SQLITE-3");
}

#[test]
fn u1_review_succeeds_after_unrelated_reuse_advance_preserving_reuse_authority() {
    let adapter = new_adapter("u1-review-after-reuse");
    let (review_precursor, reuse_advanced, review_target) =
        unrelated_scope_success_fixture(MeasurementFixtureScale::Small);
    let mut writer = open_writer(&adapter, &review_precursor);

    adapter
        .apply_scoped_command(
            &mut writer,
            &SqliteScopedCommand::from_transition_pair(
                SqliteCommandScope::ReuseGovernance,
                &review_precursor,
                &reuse_advanced,
            ),
        )
        .expect("reuse-only advance");

    adapter
        .apply_scoped_command(
            &mut writer,
            &SqliteScopedCommand::from_transition_pair(
                SqliteCommandScope::ReviewLedger,
                &review_precursor,
                &review_target,
            ),
        )
        .expect("review after unrelated reuse");

    let expected = expected_merged_review(&reuse_advanced, &review_target);
    let actual = writer.normalized_state().clone();
    assert!(CurrentContractOracle::compare(&expected, &actual).passed);
    assert_eq!(
        actual.durable_command_tokens.reuse_governance_head,
        reuse_advanced.durable_command_tokens.reuse_governance_head
    );

    let observation = Fcr03UnrelatedSuccessObservation::record(
        &expected,
        &actual,
        actual.durable_command_tokens.reuse_governance_head
            == reuse_advanced.durable_command_tokens.reuse_governance_head,
    );
    assert!(observation.unrelated_scope_preserved);
    assert!(observation.stale_full_state_not_persisted);
}

#[test]
fn u1_review_succeeds_after_unrelated_analysis_advance() {
    let adapter = new_adapter("u1-review-after-analysis");
    let (review_precursor, _, review_target) =
        unrelated_scope_success_fixture(MeasurementFixtureScale::Small);
    let (_, promotion_target) = measurement_transition_states(
        "append_reusable_promotion",
        MeasurementFixtureScale::Medium,
    )
    .expect("promotion transition");
    let analysis_target = analysis_only_target_from(&review_precursor, &promotion_target);

    let mut writer = open_writer(&adapter, &review_precursor);
    adapter
        .apply_scoped_command(
            &mut writer,
            &SqliteScopedCommand::from_transition_pair(
                SqliteCommandScope::ActiveAnalysis,
                &review_precursor,
                &analysis_target,
            ),
        )
        .expect("analysis-only advance");

    adapter
        .apply_scoped_command(
            &mut writer,
            &SqliteScopedCommand::from_transition_pair(
                SqliteCommandScope::ReviewLedger,
                &review_precursor,
                &review_target,
            ),
        )
        .expect("review after unrelated analysis");

    let expected = expected_merged_review(&analysis_target, &review_target);
    let actual = writer.normalized_state().clone();
    assert!(CurrentContractOracle::compare(&expected, &actual).passed);
    assert_eq!(
        actual.durable_command_tokens.active_analysis_snapshot_identity,
        analysis_target
            .durable_command_tokens
            .active_analysis_snapshot_identity
    );

    let observation = Fcr03UnrelatedSuccessObservation::record(
        &expected,
        &actual,
        actual.durable_command_tokens.active_analysis_snapshot_identity
            == analysis_target
                .durable_command_tokens
                .active_analysis_snapshot_identity,
    );
    assert!(observation.unrelated_scope_preserved);
    assert!(observation.stale_full_state_not_persisted);
}

#[test]
fn u1_reuse_succeeds_after_unrelated_review_advance() {
    let adapter = new_adapter("u1-reuse-after-review");
    let (review_precursor, reuse_advanced, review_target) =
        unrelated_scope_success_fixture(MeasurementFixtureScale::Small);
    let mut writer = open_writer(&adapter, &review_precursor);

    adapter
        .apply_scoped_command(
            &mut writer,
            &SqliteScopedCommand::from_transition_pair(
                SqliteCommandScope::ReviewLedger,
                &review_precursor,
                &review_target,
            ),
        )
        .expect("review advance");

    adapter
        .apply_scoped_command(
            &mut writer,
            &SqliteScopedCommand::from_transition_pair(
                SqliteCommandScope::ReuseGovernance,
                &review_precursor,
                &reuse_advanced,
            ),
        )
        .expect("reuse after unrelated review");

    let mut expected = review_target.clone();
    expected.reuse_governance_events = reuse_advanced.reuse_governance_events.clone();
    expected.effective_reusable_records = reuse_advanced.effective_reusable_records.clone();
    expected.historical_reusable_records = reuse_advanced.historical_reusable_records.clone();
    expected.reusable_snapshot_identity = reuse_advanced.reusable_snapshot_identity.clone();
    expected.reuse_enabled_analysis_binding =
        reuse_advanced.reuse_enabled_analysis_binding.clone();
    expected.durable_command_tokens.reuse_governance_head =
        reuse_advanced.durable_command_tokens.reuse_governance_head;
    if let Some(binding) = &reuse_advanced.reuse_enabled_analysis_binding {
        if !expected
            .analysis_snapshots
            .iter()
            .any(|snapshot| snapshot.identity == binding.analysis_snapshot_identity)
        {
            expected
                .analysis_snapshots
                .push(binding.analysis_snapshot.clone());
        }
    }
    finalize_derived_fields(&mut expected);
    let expected = expected.normalize();
    assert!(CurrentContractOracle::compare(&expected, writer.normalized_state()).passed);
}

#[test]
fn u1_analysis_succeeds_after_unrelated_review_and_reuse_advance() {
    let adapter = new_adapter("u1-analysis-after-review-reuse");
    let (review_precursor, reuse_advanced, review_target) =
        unrelated_scope_success_fixture(MeasurementFixtureScale::Small);
    let (_, promotion_target) = measurement_transition_states(
        "append_reusable_promotion",
        MeasurementFixtureScale::Medium,
    )
    .expect("analysis target");
    let analysis_target = analysis_only_target_from(&review_precursor, &promotion_target);

    let mut writer = open_writer(&adapter, &review_precursor);
    adapter
        .apply_scoped_command(
            &mut writer,
            &SqliteScopedCommand::from_transition_pair(
                SqliteCommandScope::ReviewLedger,
                &review_precursor,
                &review_target,
            ),
        )
        .expect("review advance");
    adapter
        .apply_scoped_command(
            &mut writer,
            &SqliteScopedCommand::from_transition_pair(
                SqliteCommandScope::ReuseGovernance,
                &review_precursor,
                &reuse_advanced,
            ),
        )
        .expect("reuse advance");

    let analysis_precursor = writer.normalized_state().clone();
    adapter
        .apply_scoped_command(
            &mut writer,
            &SqliteScopedCommand::from_transition_pair(
                SqliteCommandScope::ActiveAnalysis,
                &review_precursor,
                &analysis_target,
            ),
        )
        .expect("analysis after unrelated review+reuse");

    let mut expected = analysis_precursor.clone();
    for snapshot in &analysis_target.analysis_snapshots {
        if !expected
            .analysis_snapshots
            .iter()
            .any(|existing| existing.identity == snapshot.identity)
        {
            expected.analysis_snapshots.push(snapshot.clone());
        }
    }
    expected.durable_command_tokens.active_analysis_snapshot_identity = analysis_target
        .durable_command_tokens
        .active_analysis_snapshot_identity
        .clone();
    finalize_derived_fields(&mut expected);
    let expected = expected.normalize();
    assert!(CurrentContractOracle::compare(&expected, writer.normalized_state()).passed);
}

#[test]
fn c1_stale_review_command_rejected_with_authority_unchanged() {
    let adapter = new_adapter("c1-stale-review");
    let (precursor, target) =
        measurement_transition_states("append_review_decision", MeasurementFixtureScale::Small)
            .expect("review transition");
    let mut writer = open_writer(&adapter, &precursor);

    adapter
        .apply_scoped_command(
            &mut writer,
            &SqliteScopedCommand::from_transition_pair(
                SqliteCommandScope::ReviewLedger,
                &precursor,
                &target,
            ),
        )
        .expect("first review");

    let before = writer.normalized_state().clone();
    let mut stale = SqliteScopedCommand::from_transition_pair(
        SqliteCommandScope::ReviewLedger,
        &precursor,
        &target,
    );
    stale.preconditions.review_ledger_head = precursor.durable_command_tokens.review_ledger_head;

    let error = adapter
        .apply_scoped_command(&mut writer, &stale)
        .expect_err("stale review");
    assert_eq!(error.code, "stale-review-ledger-precondition");
    assert_eq!(before, writer.normalized_state().clone());

    let observation = Fcr03StaleRejectionObservation::record(
        error.code,
        &before,
        writer.normalized_state(),
        true,
    );
    assert!(observation.post_rejection_authority_unchanged);
}

#[test]
fn c1_stale_reuse_command_rejected_with_authority_unchanged() {
    let adapter = new_adapter("c1-stale-reuse");
    let (precursor, target) = measurement_transition_states(
        "append_reusable_revocation",
        MeasurementFixtureScale::Small,
    )
    .expect("reuse transition");
    let mut writer = open_writer(&adapter, &precursor);

    adapter
        .apply_scoped_command(
            &mut writer,
            &SqliteScopedCommand::from_transition_pair(
                SqliteCommandScope::ReuseGovernance,
                &precursor,
                &target,
            ),
        )
        .expect("first reuse");

    let before = writer.normalized_state().clone();
    let mut stale = SqliteScopedCommand::from_transition_pair(
        SqliteCommandScope::ReuseGovernance,
        &precursor,
        &target,
    );
    stale.preconditions.reuse_governance_head =
        precursor.durable_command_tokens.reuse_governance_head;

    let error = adapter
        .apply_scoped_command(&mut writer, &stale)
        .expect_err("stale reuse");
    assert_eq!(error.code, "stale-reuse-governance-precondition");
    assert_eq!(before, writer.normalized_state().clone());
}

#[test]
fn c1_stale_active_analysis_command_rejected_with_authority_unchanged() {
    let adapter = new_adapter("c1-stale-analysis");
    let (precursor, _, _) = unrelated_scope_success_fixture(MeasurementFixtureScale::Small);
    let mut writer = open_writer(&adapter, &precursor);
    let before = writer.normalized_state().clone();

    let mut stale = SqliteScopedCommand::from_transition_pair(
        SqliteCommandScope::ActiveAnalysis,
        &precursor,
        &precursor,
    );
    stale.preconditions.active_analysis_snapshot_identity = "analysis:stale-selection".to_owned();

    let error = adapter
        .apply_scoped_command(&mut writer, &stale)
        .expect_err("stale analysis");
    assert_eq!(error.code, "stale-analysis-selection-precondition");
    assert_eq!(before, writer.normalized_state().clone());
}

#[test]
fn c3_stale_caller_body_cannot_rewind_unrelated_reuse_authority() {
    let adapter = new_adapter("c3-overwrite-reuse");
    let (review_precursor, reuse_advanced, review_target) =
        unrelated_scope_success_fixture(MeasurementFixtureScale::Small);
    let mut writer = open_writer(&adapter, &review_precursor);

    adapter
        .apply_scoped_command(
            &mut writer,
            &SqliteScopedCommand::from_transition_pair(
                SqliteCommandScope::ReuseGovernance,
                &review_precursor,
                &reuse_advanced,
            ),
        )
        .expect("reuse advance");

    let rewind = reject_full_state_authority_replace(writer.normalized_state(), &review_precursor)
        .expect_err("stale full-state rewind");
    assert_eq!(rewind.code, "stale-full-state-unrelated-rewind");

    adapter
        .apply_scoped_command(
            &mut writer,
            &SqliteScopedCommand::from_transition_pair(
                SqliteCommandScope::ReviewLedger,
                &review_precursor,
                &review_target,
            ),
        )
        .expect("scoped review preserves reuse");

    assert_eq!(
        writer.normalized_state().durable_command_tokens.reuse_governance_head,
        reuse_advanced.durable_command_tokens.reuse_governance_head
    );
}

#[test]
fn c3_stale_caller_body_cannot_rewind_unrelated_review_authority() {
    let adapter = new_adapter("c3-overwrite-review");
    let (review_precursor, reuse_advanced, review_target) =
        unrelated_scope_success_fixture(MeasurementFixtureScale::Small);
    let mut writer = open_writer(&adapter, &review_precursor);

    adapter
        .apply_scoped_command(
            &mut writer,
            &SqliteScopedCommand::from_transition_pair(
                SqliteCommandScope::ReviewLedger,
                &review_precursor,
                &review_target,
            ),
        )
        .expect("review advance");

    adapter
        .apply_scoped_command(
            &mut writer,
            &SqliteScopedCommand::from_transition_pair(
                SqliteCommandScope::ReuseGovernance,
                &review_precursor,
                &reuse_advanced,
            ),
        )
        .expect("reuse from stale precursor body must merge onto latest review");

    assert!(
        writer.normalized_state().durable_command_tokens.review_ledger_head
            >= review_target.durable_command_tokens.review_ledger_head,
        "review authority must not be rewound"
    );
}

#[test]
fn c3_stale_caller_body_cannot_rewind_active_analysis_authority() {
    let adapter = new_adapter("c3-overwrite-analysis");
    let (review_precursor, reuse_advanced, _) =
        unrelated_scope_success_fixture(MeasurementFixtureScale::Small);
    let (_, promotion_target) = measurement_transition_states(
        "append_reusable_promotion",
        MeasurementFixtureScale::Medium,
    )
    .expect("promotion transition");
    let analysis_target = analysis_only_target_from(&review_precursor, &promotion_target);
    let mut writer = open_writer(&adapter, &review_precursor);

    adapter
        .apply_scoped_command(
            &mut writer,
            &SqliteScopedCommand::from_transition_pair(
                SqliteCommandScope::ActiveAnalysis,
                &review_precursor,
                &analysis_target,
            ),
        )
        .expect("analysis advance");

    let active_before = writer
        .normalized_state()
        .durable_command_tokens
        .active_analysis_snapshot_identity
        .clone();

    adapter
        .apply_scoped_command(
            &mut writer,
            &SqliteScopedCommand::from_transition_pair(
                SqliteCommandScope::ReuseGovernance,
                &review_precursor,
                &reuse_advanced,
            ),
        )
        .expect("unrelated reuse on top of analysis");

    assert_eq!(
        writer
            .normalized_state()
            .durable_command_tokens
            .active_analysis_snapshot_identity,
        active_before
    );
}

#[test]
fn persistence_merged_command_survives_close_reopen_with_oracle_match() {
    let adapter = new_adapter("persistence-close-reopen");
    let (precursor, target) =
        measurement_transition_states("append_review_decision", MeasurementFixtureScale::Small)
            .expect("review transition");
    let session = adapter.create(&precursor).expect("create");
    let mut writer = adapter
        .open_existing(session.session_id(), SqliteOpenMode::Writable)
        .expect("open writer");
    adapter
        .apply_scoped_command(
            &mut writer,
            &SqliteScopedCommand::from_transition_pair(
                SqliteCommandScope::ReviewLedger,
                &precursor,
                &target,
            ),
        )
        .expect("apply");
    let expected = writer.normalized_state().clone();
    let session_id = session.session_id().to_owned();
    adapter.close(writer).expect("close");

    let reopened = adapter
        .open_existing(&session_id, SqliteOpenMode::ReadOnly)
        .expect("reopen");
    assert!(CurrentContractOracle::compare(&expected, reopened.normalized_state()).passed);
    adapter.close(reopened).expect("close read-only");
}

#[test]
fn rejected_command_leaves_persisted_authority_unchanged() {
    let adapter = new_adapter("rejection-unchanged");
    let (precursor, target) =
        measurement_transition_states("append_review_decision", MeasurementFixtureScale::Small)
            .expect("review transition");
    let session = adapter.create(&precursor).expect("create");
    let session_id = session.session_id().to_owned();
    let mut writer = adapter
        .open_existing(&session_id, SqliteOpenMode::Writable)
        .expect("open writer");

    adapter
        .apply_scoped_command(
            &mut writer,
            &SqliteScopedCommand::from_transition_pair(
                SqliteCommandScope::ReviewLedger,
                &precursor,
                &target,
            ),
        )
        .expect("advance");
    adapter.close(writer).expect("close writer");

    let before = adapter
        .open_existing(&session_id, SqliteOpenMode::ReadOnly)
        .expect("read before")
        .normalized_state()
        .clone();
    adapter.close(
        adapter
            .open_existing(&session_id, SqliteOpenMode::ReadOnly)
            .expect("read before close"),
    )
    .expect("close read-only");

    let mut writer = adapter
        .open_existing(&session_id, SqliteOpenMode::Writable)
        .expect("reopen writer");
    let mut stale = SqliteScopedCommand::from_transition_pair(
        SqliteCommandScope::ReviewLedger,
        &precursor,
        &target,
    );
    stale.preconditions.review_ledger_head = precursor.durable_command_tokens.review_ledger_head;
    let _ = adapter
        .apply_scoped_command(&mut writer, &stale)
        .expect_err("stale");
    adapter.close(writer).expect("close");

    let after = adapter
        .open_existing(&session_id, SqliteOpenMode::ReadOnly)
        .expect("read after")
        .normalized_state()
        .clone();
    assert_eq!(before, after);
}

#[test]
fn physical_committed_generation_advances_only_on_successful_scoped_apply() {
    let adapter = new_adapter("generation-physical-order");
    let (precursor, target) =
        measurement_transition_states("append_review_decision", MeasurementFixtureScale::Small)
            .expect("review transition");
    let mut writer = open_writer(&adapter, &precursor);
    assert_eq!(writer.committed_generation, 1);

    let ack = adapter
        .apply_scoped_command(
            &mut writer,
            &SqliteScopedCommand::from_transition_pair(
                SqliteCommandScope::ReviewLedger,
                &precursor,
                &target,
            ),
        )
        .expect("apply");
    assert_eq!(ack.committed_generation, 2);
    assert_eq!(writer.committed_generation, 2);
}

#[test]
fn regression_interrupted_transition_before_commit_leaves_authority_unchanged() {
    let adapter = new_adapter("regression-interrupt");
    let (precursor, target) =
        measurement_transition_states("append_review_decision", MeasurementFixtureScale::Small)
            .expect("review transition");
    let session = adapter.create(&precursor).expect("create");
    let session_id = session.session_id().to_owned();
    let mut writer = adapter
        .open_existing(&session_id, SqliteOpenMode::Writable)
        .expect("open writer");
    let before = writer.normalized_state().clone();
    adapter.arm_fail_before_commit_for_test();
    let _ = adapter
        .apply_scoped_command(
            &mut writer,
            &SqliteScopedCommand::from_transition_pair(
                SqliteCommandScope::ReviewLedger,
                &precursor,
                &target,
            ),
        )
        .expect_err("injected interrupt");
    adapter.close(writer).ok();

    let reopened = adapter
        .open_existing(&session_id, SqliteOpenMode::ReadOnly)
        .expect("reopen");
    assert_eq!(before, reopened.normalized_state().clone());
    adapter.close(reopened).expect("close");
}

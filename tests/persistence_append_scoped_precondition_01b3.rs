#![cfg(feature = "persistence-spike")]

use std::time::{SystemTime, UNIX_EPOCH};

use vox_proof::persistence_evidence::current_contract::append_authoritative_01b3::{
    AppendCommandScope, AppendScopedCommand, AppendScopedPreconditionCandidateAdapter,
    APPEND_SCOPED_PRECONDITION_CANDIDATE_VERSION,
};
use vox_proof::persistence_evidence::current_contract::evidence_01c::measurement_transitions::{
    measurement_transition_states, unrelated_scope_success_fixture,
};
use vox_proof::persistence_evidence::current_contract::evidence_01c::scenario_observation::{
    Fcr03StaleRejectionObservation, Fcr03UnrelatedSuccessObservation,
};
use vox_proof::persistence_evidence::{
    AppendOpenMode, CurrentContractOracle, MeasurementFixtureScale,
    APPEND_AUTHORITATIVE_CANDIDATE_VERSION,
};

fn new_adapter(label: &str) -> AppendScopedPreconditionCandidateAdapter {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    AppendScopedPreconditionCandidateAdapter::new(
        std::env::temp_dir().join(format!("voxproof-01b3-{label}-{nonce}")),
    )
    .expect("candidate root")
}

#[test]
fn append_01b3_candidate_version_is_distinct_from_01b2() {
    assert_eq!(APPEND_AUTHORITATIVE_CANDIDATE_VERSION, "01B-2");
    assert_eq!(APPEND_SCOPED_PRECONDITION_CANDIDATE_VERSION, "01B-3");
}

#[test]
fn u1_review_succeeds_after_unrelated_reuse_advance_preserving_reuse_authority() {
    let adapter = new_adapter("u1-unrelated-success");
    let (review_precursor, reuse_advanced, review_target) =
        unrelated_scope_success_fixture(MeasurementFixtureScale::Small);

    let session = adapter.create(&review_precursor).expect("create");
    let mut writer = adapter
        .open_existing(session.session_id(), AppendOpenMode::Writable)
        .expect("open writer");

    let reuse_command = AppendScopedCommand::from_transition_pair(
        AppendCommandScope::ReuseGovernance,
        &review_precursor,
        &reuse_advanced,
    );
    adapter
        .apply_scoped_command(&mut writer, &reuse_command)
        .expect("reuse-only advance");

    let review_command = AppendScopedCommand::from_transition_pair(
        AppendCommandScope::ReviewLedger,
        &review_precursor,
        &review_target,
    );
    adapter
        .apply_scoped_command(&mut writer, &review_command)
        .expect("review command remains valid after unrelated reuse advance");

    let mut expected = reuse_advanced.clone();
    expected.review_ledger_events = review_target.review_ledger_events.clone();
    expected.effective_review_status = review_target.effective_review_status.clone();
    expected.durable_command_tokens.review_ledger_head =
        review_target.durable_command_tokens.review_ledger_head;
    vox_proof::persistence_evidence::current_contract::finalize_derived_fields(&mut expected);
    let expected = expected.normalize();

    let actual = writer.normalized_state().clone();
    assert!(
        CurrentContractOracle::compare(&expected, &actual).passed,
        "oracle mismatch after unrelated-scope success"
    );
    assert_eq!(
        actual.durable_command_tokens.reuse_governance_head,
        reuse_advanced.durable_command_tokens.reuse_governance_head,
        "newest reuse authority must be preserved"
    );

    let observation = Fcr03UnrelatedSuccessObservation::record(
        &expected,
        &actual,
        actual.durable_command_tokens.reuse_governance_head
            == reuse_advanced.durable_command_tokens.reuse_governance_head,
    );
    assert!(observation.transition_applied);
    assert!(observation.post_apply_oracle_compare);
    assert!(observation.unrelated_scope_preserved);
    assert!(observation.stale_full_state_not_persisted);
}

#[test]
fn c1_stale_review_command_rejected_with_authority_unchanged() {
    let adapter = new_adapter("c1-same-scope-conflict");
    let (review_precursor, review_target) = measurement_transition_states(
        "append_review_decision",
        MeasurementFixtureScale::Small,
    )
    .expect("review transition");

    let session = adapter.create(&review_precursor).expect("create");
    let mut writer = adapter
        .open_existing(session.session_id(), AppendOpenMode::Writable)
        .expect("open writer");

    let first_review = AppendScopedCommand::from_transition_pair(
        AppendCommandScope::ReviewLedger,
        &review_precursor,
        &review_target,
    );
    adapter
        .apply_scoped_command(&mut writer, &first_review)
        .expect("advance review scope");

    let authority_before = writer.normalized_state().clone();
    let mut stale_review = AppendScopedCommand::from_transition_pair(
        AppendCommandScope::ReviewLedger,
        &review_precursor,
        &review_target,
    );
    stale_review.preconditions.review_ledger_head =
        review_precursor.durable_command_tokens.review_ledger_head;

    let error = adapter
        .apply_scoped_command(&mut writer, &stale_review)
        .expect_err("stale review must be rejected");
    assert_eq!(error.code, "stale-review-ledger-precondition");

    let authority_after = writer.normalized_state().clone();
    assert_eq!(authority_before, authority_after);

    let observation = Fcr03StaleRejectionObservation::record(
        error.code,
        &authority_before,
        &authority_after,
        CurrentContractOracle::compare(&authority_before, &authority_after).passed,
    );
    assert!(!observation.transition_applied);
    assert_eq!(
        observation.observed_failure_code,
        "stale-review-ledger-precondition"
    );
    assert!(observation.post_rejection_oracle_compare);
    assert!(observation.post_rejection_authority_unchanged);
}

#[test]
fn c3_stale_full_state_proposal_cannot_rewind_unrelated_reuse_authority() {
    let adapter = new_adapter("c3-stale-full-state");
    let (review_precursor, reuse_advanced, review_target) =
        unrelated_scope_success_fixture(MeasurementFixtureScale::Small);

    let session = adapter.create(&review_precursor).expect("create");
    let mut writer = adapter
        .open_existing(session.session_id(), AppendOpenMode::Writable)
        .expect("open writer");

    let reuse_command = AppendScopedCommand::from_transition_pair(
        AppendCommandScope::ReuseGovernance,
        &review_precursor,
        &reuse_advanced,
    );
    adapter
        .apply_scoped_command(&mut writer, &reuse_command)
        .expect("reuse advance");

    let rewind_error = AppendScopedPreconditionCandidateAdapter::reject_full_state_authority_replace(
        writer.normalized_state(),
        &review_precursor,
    )
    .expect_err("stale full-state replace must be rejected");
    assert_eq!(rewind_error.code, "stale-full-state-unrelated-rewind");

    assert_eq!(
        writer.normalized_state().durable_command_tokens.reuse_governance_head,
        reuse_advanced.durable_command_tokens.reuse_governance_head
    );

    let stale_review_command = AppendScopedCommand::from_transition_pair(
        AppendCommandScope::ReviewLedger,
        &review_precursor,
        &review_target,
    );
    adapter
        .apply_scoped_command(&mut writer, &stale_review_command)
        .expect("scoped review apply must preserve unrelated reuse authority");
    assert_eq!(
        writer.normalized_state().durable_command_tokens.reuse_governance_head,
        reuse_advanced.durable_command_tokens.reuse_governance_head,
        "scoped apply must not rewind reuse via stale full-state target fields"
    );
}

#[test]
fn physical_committed_sequence_advances_only_on_successful_scoped_apply() {
    let adapter = new_adapter("sequence-physical-order");
    let (precursor, target) = measurement_transition_states(
        "append_review_decision",
        MeasurementFixtureScale::Small,
    )
    .expect("review transition");
    let session = adapter.create(&precursor).expect("create");
    let mut writer = adapter
        .open_existing(session.session_id(), AppendOpenMode::Writable)
        .expect("open writer");
    assert_eq!(writer.committed_sequence, 1);

    let command =
        AppendScopedCommand::from_transition_pair(AppendCommandScope::ReviewLedger, &precursor, &target);
    let ack = adapter
        .apply_scoped_command(&mut writer, &command)
        .expect("apply");
    assert_eq!(ack.committed_sequence, 2);
    assert_eq!(writer.committed_sequence, 2);
}

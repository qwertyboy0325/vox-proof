use vox_proof::persistence_evidence::{
    CurrentContractOracle, OracleViolationCodeV3, build_golden_small_state,
};

#[test]
fn review_ledger_order_change_fails_oracle() {
    let expected = build_golden_small_state();
    let mut actual = expected.clone();
    if actual.review_ledger_events.len() >= 2 {
        actual.review_ledger_events.swap(0, 1);
    } else {
        actual
            .review_ledger_events
            .push(actual.review_ledger_events[0].clone());
    }
    let result = CurrentContractOracle::compare(&expected, &actual);
    assert!(!result.passed);
}

#[test]
fn reuse_governance_order_change_fails_oracle() {
    let expected = build_golden_small_state();
    let mut actual = expected.clone();
    if actual.reuse_governance_events.len() >= 2 {
        actual.reuse_governance_events.swap(0, 1);
    } else {
        actual
            .reuse_governance_events
            .push(actual.reuse_governance_events[0].clone());
    }
    let result = CurrentContractOracle::compare(&expected, &actual);
    assert!(!result.passed);
}

#[test]
fn promotion_actor_change_fails_oracle() {
    let expected = build_golden_small_state();
    let mut actual = expected.clone();
    if let Some(event) = actual.reuse_governance_events.first_mut() {
        event.actor_display_label = "Tampered Actor".to_owned();
    }
    let result = CurrentContractOracle::compare(&expected, &actual);
    assert!(!result.passed);
}

#[test]
fn reusable_snapshot_linkage_mismatch_fails_oracle() {
    let expected = build_golden_small_state();
    let mut actual = expected.clone();
    actual.reusable_snapshot_identity = "reusable-influence-snapshot:sha256-v2:0000".to_owned();
    let result = CurrentContractOracle::compare(&expected, &actual);
    assert!(!result.passed);
    assert!(
        result
            .violations
            .iter()
            .any(|v| v.code == OracleViolationCodeV3::ChangedReusableSnapshotIdentity)
    );
}

#[test]
fn removing_derived_cache_does_not_change_canonical_truth() {
    let expected = build_golden_small_state();
    let mut actual = expected.clone();
    actual.derived_queue_projection.clear();
    actual.effective_review_status.clear();
    let result = CurrentContractOracle::compare(&expected, &actual);
    assert!(result.passed, "{:?}", result.violations);
}

#[test]
fn missing_canonical_history_fails_oracle() {
    let expected = build_golden_small_state();
    let mut actual = expected.clone();
    actual.review_ledger_events.clear();
    let result = CurrentContractOracle::compare(&expected, &actual);
    assert!(!result.passed);
}

#[test]
fn fabricated_automatic_decision_fails_oracle() {
    let mut state = build_golden_small_state();
    if let Some(event) = state.review_ledger_events.first_mut() {
        event.provenance = "automatic".to_owned();
    }
    let result = CurrentContractOracle::compare(&state, &state);
    assert!(!result.passed);
    assert!(
        result
            .violations
            .iter()
            .any(|v| v.code == OracleViolationCodeV3::FabricatedAutomaticDecision)
    );
}

#[test]
fn source_locator_boundary_change_fails_oracle() {
    let expected = build_golden_small_state();
    let mut actual = expected.clone();
    if let Some(record) = actual.effective_reusable_records.first_mut() {
        record.source_locator_digest = "sha256:deadbeef".to_owned();
    }
    let result = CurrentContractOracle::compare(&expected, &actual);
    assert!(!result.passed);
}

#[test]
fn export_transport_fields_are_absent_from_hydration_fixture() {
    let state = build_golden_small_state();
    let serialized = serde_json::to_string(&state).expect("serialize");
    assert!(!serialized.contains("application_export_v2"));
    assert!(!serialized.contains("application_export_v3"));
}

#[test]
fn golden_state_passes_oracle_validation() {
    let state = build_golden_small_state();
    let result = CurrentContractOracle::validate(&state);
    assert!(result.passed, "{:?}", result.violations);
    assert!(!result.canonical_fingerprint.is_empty());
}

use vox_proof::persistence_evidence::{
    CurrentContractOracle, EvidenceReuseGovernanceEvent, OracleViolationCodeV3,
    build_candidate_rejected_state, build_golden_small_state, build_promoted_active_state,
    build_revoked_historical_state, build_superseded_state, canonical_fingerprint,
    derive_contract_projection,
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
    if let Some(EvidenceReuseGovernanceEvent::PromotionAccepted { actor, .. }) =
        actual.reuse_governance_events.first_mut()
    {
        actor.display_label = "Tampered Actor".to_owned();
    }
    let result = CurrentContractOracle::compare(&expected, &actual);
    assert!(!result.passed);
}

#[test]
fn reusable_snapshot_linkage_mismatch_fails_oracle() {
    let expected = build_golden_small_state();
    let mut actual = expected.clone();
    actual.reusable_snapshot_identity = "reusable-influence-snapshot:sha256-v2:0000".to_owned();
    let result = CurrentContractOracle::validate(&actual);
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
    actual.effective_reusable_records.clear();
    actual.historical_reusable_records.clear();
    actual.reusable_snapshot_identity.clear();
    actual.reuse_enabled_analysis_binding = Default::default();
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
    state.review_ledger_events[0].provenance = "automatic".to_owned();
    let result = CurrentContractOracle::validate(&state);
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
    if let Some(EvidenceReuseGovernanceEvent::PromotionAccepted { source_locator, .. }) =
        actual.reuse_governance_events.first_mut()
    {
        source_locator.effective_at_ledger_length += 1;
    }
    let result = CurrentContractOracle::validate(&actual);
    assert!(!result.passed);
    assert!(result.violations.iter().any(|v| {
        v.code == OracleViolationCodeV3::SourceLocatorBoundaryViolation
            || v.code == OracleViolationCodeV3::DecisionDigestMismatch
    }));
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

#[test]
fn candidate_rejected_fixture_passes() {
    let state = build_candidate_rejected_state();
    assert!(CurrentContractOracle::validate(&state).passed);
}

#[test]
fn revoked_historical_fixture_passes() {
    let state = build_revoked_historical_state();
    assert!(CurrentContractOracle::validate(&state).passed);
}

#[test]
fn superseded_fixture_passes() {
    let state = build_superseded_state();
    assert!(CurrentContractOracle::validate(&state).passed);
}

#[test]
fn derived_mutation_inconsistent_with_fold_fails_oracle() {
    let mut state = build_promoted_active_state();
    state.effective_reusable_records[0].confirmed_replacement = "Tampered".to_owned();
    let result = CurrentContractOracle::validate(&state);
    assert!(!result.passed);
    assert!(
        result
            .violations
            .iter()
            .any(|v| v.code == OracleViolationCodeV3::DerivedRebuildMismatch)
    );
}

#[test]
fn pure_derived_mutation_does_not_alter_canonical_fingerprint() {
    let state = build_promoted_active_state();
    let baseline = canonical_fingerprint(&state.canonical_projection());
    let mut mutated = state.clone();
    mutated.reusable_snapshot_identity = "reusable-influence-snapshot:sha256-v2:ffff".to_owned();
    assert_eq!(
        baseline,
        canonical_fingerprint(&mutated.canonical_projection())
    );
}

#[test]
fn independent_derivation_matches_golden_state() {
    let state = build_promoted_active_state();
    let (derived, violations) = derive_contract_projection(&state);
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(
        state.effective_review_status,
        derived.effective_review_status
    );
    assert_eq!(
        state.reusable_snapshot_identity,
        derived.reusable_snapshot_identity
    );
}

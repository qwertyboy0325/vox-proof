use vox_proof::persistence_evidence::{
    CurrentContractOracle, EvidenceReuseGovernanceEvent, OracleViolationCodeV3,
    build_candidate_rejected_state, build_promoted_active_state, build_revoked_historical_state,
    build_superseded_state, candidate_key_canonical_digest, candidate_key_identity_digest,
    derive_contract_projection,
};

fn promotion_locator(
    state: &vox_proof::persistence_evidence::CurrentContractState,
) -> vox_proof::persistence_evidence::EvidenceSourceDecisionLocator {
    if let EvidenceReuseGovernanceEvent::PromotionAccepted { source_locator, .. } =
        &state.reuse_governance_events[0]
    {
        return source_locator.clone();
    }
    panic!("expected promotion");
}

fn promotion_candidate_key(
    state: &vox_proof::persistence_evidence::CurrentContractState,
) -> vox_proof::persistence_evidence::EvidenceReuseCandidateKey {
    if let EvidenceReuseGovernanceEvent::PromotionAccepted { candidate_key, .. } =
        &state.reuse_governance_events[0]
    {
        return candidate_key.clone();
    }
    panic!("expected promotion");
}

#[test]
fn locator_fields_are_present_in_canonical_evidence() {
    let locator = promotion_locator(&build_promoted_active_state());
    assert!(!locator.source_revision_id.is_empty());
    assert!(!locator.source_analysis_snapshot_identity.is_empty());
    assert!(!locator.source_review_case_id.is_empty());
    assert!(!locator.decision_digest.is_empty());
    assert!(locator.effective_at_ledger_length > 0);
}

#[test]
fn changing_source_revision_fails() {
    let mut state = build_promoted_active_state();
    if let EvidenceReuseGovernanceEvent::PromotionAccepted { source_locator, .. } =
        &mut state.reuse_governance_events[0]
    {
        source_locator.source_revision_id = "rev:sha256-v1:dead".to_owned();
    }
    assert!(!CurrentContractOracle::validate(&state).passed);
}

#[test]
fn changing_review_ledger_position_fails() {
    let mut state = build_promoted_active_state();
    if let EvidenceReuseGovernanceEvent::PromotionAccepted { source_locator, .. } =
        &mut state.reuse_governance_events[0]
    {
        source_locator.review_ledger_position = 99;
    }
    assert!(!CurrentContractOracle::validate(&state).passed);
}

#[test]
fn candidate_key_identity_is_stable_and_not_debug_based() {
    let key = promotion_candidate_key(&build_promoted_active_state());
    assert_eq!(
        candidate_key_identity_digest(&key),
        candidate_key_canonical_digest(&key)
    );
    assert!(!candidate_key_canonical_digest(&key).contains("ReuseCandidateKey"));
}

#[test]
fn review_ledger_duplicate_index_fails() {
    let mut state = build_promoted_active_state();
    state
        .review_ledger_events
        .push(state.review_ledger_events[0].clone());
    let result = CurrentContractOracle::validate(&state);
    assert!(result.violations.iter().any(|v| {
        matches!(
            v.code,
            OracleViolationCodeV3::ReviewLedgerDuplicateIndex
                | OracleViolationCodeV3::ReviewLedgerIndexGap
        )
    }));
}

#[test]
fn wrong_source_anchor_fails() {
    let mut state = build_promoted_active_state();
    state.review_cases[0].anchor_start_byte += 1;
    assert!(!CurrentContractOracle::validate(&state).passed);
}

#[test]
fn candidate_rejection_fixture_independently_derives_rejection() {
    let state = build_candidate_rejected_state();
    let (derived, violations) = derive_contract_projection(&state);
    assert!(violations.is_empty(), "{violations:?}");
    assert!(!derived.rejected_candidate_identities.is_empty());
    assert!(derived.effective_reusable_records.is_empty());
}

#[test]
fn revoked_fixture_keeps_historical_record() {
    let state = build_revoked_historical_state();
    let (derived, _) = derive_contract_projection(&state);
    assert!(derived.effective_reusable_records.is_empty());
    assert!(!derived.historical_reusable_records.is_empty());
}

#[test]
fn superseded_fixture_preserves_lineage() {
    let state = build_superseded_state();
    let (derived, _) = derive_contract_projection(&state);
    assert_eq!(derived.effective_reusable_records.len(), 1);
    assert_eq!(derived.historical_reusable_records.len(), 1);
    assert_eq!(
        derived.historical_reusable_records[0].superseded_by,
        Some(derived.effective_reusable_records[0].record_id)
    );
}

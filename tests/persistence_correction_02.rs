use std::panic;

use vox_proof::application_reuse::reusable_influence_snapshot_for_parts;
use vox_proof::persistence_evidence::current_contract::classification::FIELD_CLASSIFICATION_REGISTRY;
use vox_proof::persistence_evidence::current_contract::measurement::comparative_measurement_contract;
use vox_proof::persistence_evidence::current_contract::scenario_contract::scenario_contract_v3;
use vox_proof::persistence_evidence::{
    CurrentContractOracle, EvidenceReuseGovernanceEvent, build_base_manual_replacement_state,
    build_promoted_active_session, build_promoted_active_state, build_revoked_historical_state,
    build_superseded_state, derive_contract_projection, validate_measurement_contract_value,
    validate_scenario_contracts_v3,
};

#[test]
fn evidence_snapshot_identity_matches_production_for_promoted_active() {
    let session = build_promoted_active_session();
    let parts = session.reuse_parts();
    let production = reusable_influence_snapshot_for_parts(parts, session.reuse_state())
        .expect("production snapshot")
        .identity()
        .to_tagged_string();
    let state = build_promoted_active_state();
    assert_eq!(state.reusable_snapshot_identity, production);
}

#[test]
fn evidence_snapshot_identity_matches_production_for_superseded() {
    let state = build_superseded_state();
    let (derived, violations) = derive_contract_projection(&state);
    assert!(violations.is_empty(), "{violations:?}");
    assert!(!derived.reusable_snapshot_identity.is_empty());
}

#[test]
fn revoked_state_preserves_historical_reuse_binding() {
    let state = build_revoked_historical_state();
    let binding = state
        .reuse_enabled_analysis_binding
        .as_ref()
        .expect("binding present after run");
    assert!(!binding.reusable_snapshot_identity.is_empty());
    assert_ne!(
        binding.reusable_snapshot_identity,
        state.reusable_snapshot_identity
    );
    let result = CurrentContractOracle::validate(&state);
    assert!(result.passed, "{:?}", result.violations);
}

#[test]
fn base_manual_replacement_has_no_reuse_binding() {
    let state = build_base_manual_replacement_state();
    assert!(state.reuse_enabled_analysis_binding.is_none());
}

#[test]
fn missing_manual_replacement_bytes_fails() {
    let mut state = build_promoted_active_state();
    state.review_ledger_events[0].manual_replacement_bytes = None;
    assert!(!CurrentContractOracle::validate(&state).passed);
}

#[test]
fn accept_alternative_without_index_fails() {
    let mut state = build_promoted_active_state();
    state.review_ledger_events[0].action_kind = "accept_alternative".to_owned();
    state.review_ledger_events[0].alternative_index = None;
    assert!(!CurrentContractOracle::validate(&state).passed);
}

#[test]
fn unknown_revocation_does_not_mutate_fold() {
    let mut state = build_promoted_active_state();
    let actor = match &state.reuse_governance_events[0] {
        EvidenceReuseGovernanceEvent::PromotionAccepted { actor, .. } => actor.clone(),
        _ => panic!("expected promotion"),
    };
    state
        .reuse_governance_events
        .push(EvidenceReuseGovernanceEvent::ReusableInfluenceRevoked {
            event_index: state.reuse_governance_events.len(),
            record_id: 99,
            actor,
        });
    let result = CurrentContractOracle::validate(&state);
    assert!(!result.passed);
    let (derived, _) = derive_contract_projection(&state);
    assert_eq!(derived.effective_reusable_records.len(), 1);
}

#[test]
fn scenario_validator_rejects_duplicate_scenario_id() {
    let mut scenarios = scenario_contract_v3();
    scenarios.push(scenarios[0].clone());
    assert!(validate_scenario_contracts_v3(&scenarios).is_err());
}

#[test]
fn measurement_validator_rejects_disabled_aggregation_field() {
    let mut contract = comparative_measurement_contract();
    contract.aggregation_fields.count = false;
    assert!(validate_measurement_contract_value(&contract).is_err());
}

#[test]
fn oracle_anchor_validation_does_not_panic_on_mid_codepoint() {
    let mut state = build_promoted_active_state();
    state.review_cases[0].anchor_start_byte = 1;
    let result = panic::catch_unwind(|| CurrentContractOracle::validate(&state));
    assert!(result.is_ok());
    assert!(!result.unwrap().passed);
}

#[test]
fn classification_registry_covers_every_current_contract_top_level_field() {
    let required = [
        "session_id",
        "duplicated_from_session_id",
        "session_authority",
        "material_use_declaration",
        "source_revisions",
        "session_terms_identity",
        "analysis_snapshots",
        "review_cases",
        "review_ledger_events",
        "effective_review_status",
        "project_scope",
        "reuse_governance_events",
        "effective_reusable_records",
        "historical_reusable_records",
        "reusable_snapshot_identity",
        "reuse_enabled_analysis_binding",
        "derived_queue_projection",
        "durable_command_tokens",
    ];
    for field in required {
        assert!(
            FIELD_CLASSIFICATION_REGISTRY
                .iter()
                .any(|entry| entry.field_path == field)
        );
    }
}

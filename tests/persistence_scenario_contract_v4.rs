use vox_proof::persistence_evidence::{
    ExpectedOpenState, ExpectedRecoveryClass, ReadOnlyOpenPolicy, ScenarioRequirementLevel,
    scenario_contract_v3, scenario_contract_v4, validate_scenario_contract_v3,
    validate_scenario_contract_v4,
};

#[test]
fn v4_catalog_validates_and_preserves_v3_history() {
    validate_scenario_contract_v4().expect("scenario contract v4 valid");
    validate_scenario_contract_v3().expect("scenario contract v3 historical valid");
}

#[test]
fn authoritative_corruption_scenarios_adopt_fail_closed_semantics() {
    for scenario_id in ["canonical-reference-corruption", "source-locator-corruption"] {
        let v3 = scenario_contract_v3()
            .into_iter()
            .find(|scenario| scenario.scenario_id == scenario_id)
            .expect("v3 scenario");
        let v4 = scenario_contract_v4()
            .into_iter()
            .find(|scenario| scenario.scenario_id == scenario_id)
            .expect("v4 scenario");
        assert_eq!(v3.scenario_version, 1);
        assert_eq!(v4.scenario_version, 2);
        assert_eq!(v3.expected_open_state, ExpectedOpenState::ReadOnlySalvage);
        assert_eq!(v4.expected_open_state, ExpectedOpenState::Unrecoverable);
        assert_eq!(v3.read_only_open, ReadOnlyOpenPolicy::Allowed);
        assert_eq!(v4.read_only_open, ReadOnlyOpenPolicy::Forbidden);
        assert!(!v4.writable_open);
        assert_eq!(
            v4.expected_recovery_class,
            ExpectedRecoveryClass::ManualReviewRequired
        );
        assert_eq!(
            v4.expected_durable_boundary,
            "corruption_detected_fail_closed"
        );
        assert_eq!(v3.requirement, ScenarioRequirementLevel::Required);
        assert_eq!(v4.requirement, ScenarioRequirementLevel::Required);
    }
}

#[test]
fn non_authoritative_corruption_scenarios_remain_unchanged_in_v4() {
    for scenario_id in [
        "review-ledger-order-corruption",
        "reuse-governance-order-corruption",
        "derived-state-corruption-and-rebuild",
    ] {
        let v3 = scenario_contract_v3()
            .into_iter()
            .find(|scenario| scenario.scenario_id == scenario_id)
            .expect("v3 scenario");
        let v4 = scenario_contract_v4()
            .into_iter()
            .find(|scenario| scenario.scenario_id == scenario_id)
            .expect("v4 scenario");
        assert_eq!(v3, v4, "{scenario_id} should be unchanged in v4");
    }
}

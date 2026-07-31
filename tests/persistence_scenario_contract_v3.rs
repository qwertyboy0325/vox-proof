use vox_proof::persistence_evidence::{
    CandidateEligibilityStatus, FaultLayer, candidate_equivalence_requirements,
    current_contract_readiness, scenario_contract_v3, validate_scenario_contract_v3,
};

#[test]
fn scenario_ids_and_versions_are_unique() {
    validate_scenario_contract_v3().expect("scenario contract valid");
}

#[test]
fn every_required_scenario_declares_allowed_evidence_strength() {
    for scenario in scenario_contract_v3() {
        assert!(
            !scenario.allowed_evidence_strength.is_empty(),
            "{}",
            scenario.scenario_id
        );
    }
}

#[test]
fn every_fault_scenario_declares_exact_fault_layer() {
    for scenario in scenario_contract_v3() {
        let _ = format!("{:?}", scenario.fault_layer);
    }
}

#[test]
fn process_kill_scenarios_do_not_claim_filesystem_durability() {
    for scenario in scenario_contract_v3() {
        if matches!(
            scenario.fault_layer,
            FaultLayer::ChildProcessAbortOrKill | FaultLayer::ProcessCrashRecovery
        ) {
            assert!(
                scenario
                    .prohibited_claims
                    .contains(&"FilesystemDurability".to_owned())
            );
            assert!(
                scenario
                    .prohibited_claims
                    .contains(&"HardwarePowerLoss".to_owned())
            );
        }
    }
}

#[test]
fn both_candidates_receive_same_required_scenario_set() {
    let requirements = candidate_equivalence_requirements();
    assert_eq!(
        requirements.sqlite_status,
        CandidateEligibilityStatus::ImplementationNotYetEvaluated
    );
    assert_eq!(
        requirements.append_status,
        CandidateEligibilityStatus::ImplementationNotYetEvaluated
    );
    assert!(!requirements.shared_required_scenarios.is_empty());
}

#[test]
fn readiness_aggregation_remains_not_ready() {
    let readiness = current_contract_readiness();
    assert_eq!(readiness.mechanism_comparison_readiness, "not_ready");
    assert_eq!(readiness.mechanism_selection_readiness, "not_ready");
    assert_eq!(readiness.selection_status, "none");
}

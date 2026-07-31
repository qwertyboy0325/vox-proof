use vox_proof::persistence_evidence::{
    CandidateEligibilityStatus, ExpectedOpenState, ExpectedRecoveryClass, FaultLayer,
    ScenarioRequirementLevel, candidate_equivalence_requirements, current_contract_readiness,
    scenario_contract_v3, validate_scenario_contract_v3,
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
fn concurrent_writer_attempt_does_not_claim_crash_recovery() {
    let scenario = scenario_contract_v3()
        .into_iter()
        .find(|scenario| scenario.scenario_id == "concurrent-writer-attempt")
        .expect("scenario");
    assert_eq!(scenario.fault_layer, FaultLayer::MultiProcessConcurrency);
    assert!(
        !scenario
            .allowed_evidence_strength
            .contains(&"ProcessCrashRecovery".to_owned())
    );
}

#[test]
fn writer_crash_and_takeover_requires_process_interruption() {
    let scenario = scenario_contract_v3()
        .into_iter()
        .find(|scenario| scenario.scenario_id == "writer-crash-and-takeover")
        .expect("scenario");
    assert_eq!(scenario.fault_layer, FaultLayer::ChildProcessAbortOrKill);
    assert!(scenario.fault_point != "none");
    assert!(
        scenario
            .allowed_evidence_strength
            .contains(&"ProcessCrashRecovery".to_owned())
    );
}

#[test]
fn interrupted_compaction_is_capability_dependent() {
    let scenario = scenario_contract_v3()
        .into_iter()
        .find(|scenario| scenario.scenario_id == "interrupted-compaction")
        .expect("scenario");
    assert_eq!(
        scenario.requirement,
        ScenarioRequirementLevel::CapabilityDependent
    );
    assert!(scenario.capability_requirement.is_some());
}

#[test]
fn unknown_newer_format_maps_to_unsupported_version() {
    let scenario = scenario_contract_v3()
        .into_iter()
        .find(|scenario| scenario.scenario_id == "unknown-newer-format")
        .expect("scenario");
    assert_eq!(
        scenario.expected_recovery_class,
        ExpectedRecoveryClass::UnsupportedVersion
    );
    assert_eq!(
        scenario.expected_open_state,
        ExpectedOpenState::UnsupportedVersion
    );
    assert!(!scenario.writable_open);
}

#[test]
fn corruption_recovery_classifications_are_scenario_specific() {
    let canonical = scenario_contract_v3()
        .into_iter()
        .find(|scenario| scenario.scenario_id == "canonical-reference-corruption")
        .expect("canonical");
    let ledger = scenario_contract_v3()
        .into_iter()
        .find(|scenario| scenario.scenario_id == "review-ledger-order-corruption")
        .expect("ledger");
    assert_eq!(
        canonical.expected_recovery_class,
        ExpectedRecoveryClass::ManualReviewRequired
    );
    assert_eq!(
        ledger.expected_recovery_class,
        ExpectedRecoveryClass::Unrecoverable
    );
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

#[test]
fn scenario_validator_rejects_mismatched_fault_semantics() {
    assert!(validate_scenario_contract_v3().is_ok());
}

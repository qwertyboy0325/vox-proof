use vox_proof::persistence_evidence::{
    MEASUREMENT_CONTRACT_VERSION, comparative_measurement_contract, validate_measurement_contract,
};

#[test]
fn measurement_contract_is_candidate_neutral() {
    let contract = comparative_measurement_contract();
    assert_eq!(contract.contract_version, MEASUREMENT_CONTRACT_VERSION);
    assert!(!contract.operations.is_empty());
    for operation in &contract.operations {
        assert!(!operation.operation.contains("sqlite"));
        assert!(!operation.operation.contains("append_bundle"));
        assert!(!operation.operation.contains("embedded_relational"));
    }
}

#[test]
fn measurement_operations_are_unique_with_valid_samples() {
    validate_measurement_contract().expect("measurement contract valid");
    let contract = comparative_measurement_contract();
    assert_eq!(contract.deferred_scales.len(), 2);
    for deferred in &contract.deferred_scales {
        assert!(!deferred.rationale.is_empty());
        assert!(!deferred.unblock_condition.is_empty());
        assert!(!deferred.selection_impact.is_empty());
    }
    assert!(contract.minimum_environment_metadata.repository_commit);
}

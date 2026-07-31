use vox_proof::persistence_evidence::{
    comparative_measurement_contract, validate_measurement_contract,
};

#[test]
fn measurement_contract_includes_required_aggregation_fields() {
    let contract = comparative_measurement_contract();
    assert!(contract.aggregation_fields.count);
    assert!(contract.aggregation_fields.median);
    assert!(contract.aggregation_fields.p95);
    assert!(contract.aggregation_fields.maximum);
    assert!(contract.aggregation_fields.failure_count);
}

#[test]
fn measurement_contract_is_candidate_neutral() {
    let contract = comparative_measurement_contract();
    assert!(!contract.operations.is_empty());
    assert!(
        contract
            .correctness_disqualification_gates
            .iter()
            .any(|gate| gate.contains("failed_oracle_compare"))
    );
    validate_measurement_contract().expect("measurement contract valid");
}

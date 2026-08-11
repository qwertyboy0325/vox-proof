#![cfg(feature = "persistence-spike")]

use vox_proof::persistence_evidence::{
    CurrentContractOracle, EVIDENCE_01C_HARNESS_VERSION, MeasurementFixtureScale,
    build_medium_fixture_state, comparative_measurement_contract, methodology_record,
    validate_measurement_contract,
};

#[test]
fn medium_fixture_passes_oracle_v3() {
    let state = build_medium_fixture_state();
    let oracle = CurrentContractOracle::validate(&state);
    assert!(
        oracle.passed,
        "medium fixture oracle failures: {:?}",
        oracle.violations
    );
}

#[test]
fn measurement_contract_v2_medium_is_materialized() {
    validate_measurement_contract().expect("measurement contract");
    let contract = comparative_measurement_contract();
    assert_eq!(contract.deferred_scales.len(), 1);
    assert_eq!(
        contract.deferred_scales[0].scale,
        MeasurementFixtureScale::Stress
    );
    for operation in &contract.operations {
        assert!(
            operation
                .fixture_scales
                .contains(&MeasurementFixtureScale::Medium),
            "operation {} missing Medium scale",
            operation.operation
        );
    }
}

#[test]
fn evidence_01c_methodology_is_frozen() {
    let record = methodology_record();
    assert_eq!(record.harness_version, EVIDENCE_01C_HARNESS_VERSION);
    assert_eq!(record.candidates_order.len(), 2);
    assert!(!record.freeze_id.is_empty());
}

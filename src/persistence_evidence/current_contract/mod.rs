pub mod candidate_equivalence;
pub mod classification;
pub mod fixture;
pub mod measurement;
pub mod model;
pub mod oracle;
pub mod projection;
pub mod readiness;
pub mod scenario_contract;

pub use candidate_equivalence::{
    CandidateEligibilityStatus, CandidateEquivalenceRequirements,
    candidate_equivalence_requirements,
};
pub use classification::{
    FIELD_CLASSIFICATION_REGISTRY, FieldClassification, canonical_classes, derived_classes,
    excluded_classes,
};
pub use fixture::{
    GOLDEN_SMALL_TRANSCRIPT, build_golden_small_session, build_golden_small_state, golden_small,
};
pub use measurement::{
    ComparativeMeasurementContract, MEASUREMENT_CONTRACT_VERSION, comparative_measurement_contract,
    validate_measurement_contract,
};
pub use model::{
    CURRENT_CONTRACT_FIXTURE_ID, CURRENT_CONTRACT_FIXTURE_VERSION, CurrentContractFixture,
    CurrentContractFixtureScale, CurrentContractState,
};
pub use oracle::{
    CURRENT_CONTRACT_ORACLE_VERSION, CurrentContractOracle, OracleDiagnosticV3, OracleResultV3,
    OracleViolationCodeV3, canonical_fingerprint,
};
pub use projection::project_current_contract_state;
pub use readiness::{CurrentContractReadiness, EVIDENCE_PACKAGE_FILES, current_contract_readiness};
pub use scenario_contract::{
    FaultLayer, SCENARIO_CONTRACT_VERSION, ScenarioContractV3, scenario_contract_v3,
    validate_scenario_contract_v3,
};

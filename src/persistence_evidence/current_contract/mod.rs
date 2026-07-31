pub mod candidate_equivalence;
pub mod classification;
pub mod derivation;
pub mod fixture;
pub mod measurement;
pub mod model;
pub mod oracle;
pub mod production_bridge;
pub mod projection;
pub mod readiness;
pub mod scenario_contract;
pub mod serialization;
pub mod violations;

pub use candidate_equivalence::{
    CandidateEligibilityStatus, CandidateEquivalenceRequirements,
    candidate_equivalence_requirements,
};
pub use classification::{
    FIELD_CLASSIFICATION_REGISTRY, FieldClassification, canonical_classes, derived_classes,
    excluded_classes,
};
pub use derivation::{derive_contract_projection, finalize_derived_fields};
pub use fixture::{
    GOLDEN_SMALL_TRANSCRIPT, OFFSET_ANCHOR_TRANSCRIPT, VARIANT_BASE_MANUAL_REPLACEMENT,
    VARIANT_CANDIDATE_REJECTED, VARIANT_DUPLICATED_LINEAGE, VARIANT_OFFSET_ANCHOR,
    VARIANT_PROMOTED_ACTIVE, VARIANT_REVOKED_HISTORICAL, VARIANT_SUPERSEDED, all_fixture_variants,
    build_base_manual_replacement_state, build_candidate_rejected_state,
    build_duplicated_session_lineage_state, build_golden_small_session, build_golden_small_state,
    build_offset_anchor_manual_replacement_state, build_original_for_duplication_fixture,
    build_promoted_active_session, build_promoted_active_state, build_revoked_historical_state,
    build_superseded_state, golden_small,
};
pub use measurement::{
    ComparativeMeasurementContract, DeferredFixtureScale, MEASUREMENT_CONTRACT_VERSION,
    MinimumEnvironmentMetadata, comparative_measurement_contract, validate_measurement_contract,
    validate_measurement_contract_value,
};
pub use model::{
    CURRENT_CONTRACT_FIXTURE_ID, CURRENT_CONTRACT_FIXTURE_VERSION, CurrentContractFixture,
    CurrentContractFixtureScale, CurrentContractState, DerivedContractProjection,
    EvidenceReuseCandidateKey, EvidenceReuseGovernanceEvent, EvidenceSourceDecisionLocator,
};
pub use oracle::{
    CURRENT_CONTRACT_ORACLE_VERSION, CurrentContractOracle, OracleDiagnosticV3, OracleResultV3,
    OracleViolationCodeV3, canonical_fingerprint, derived_fingerprint,
};
pub use projection::{
    candidate_key_identity_digest, map_candidate_key, map_locator, project_current_contract_state,
};
pub use readiness::{CurrentContractReadiness, EVIDENCE_PACKAGE_FILES, current_contract_readiness};
pub use scenario_contract::{
    ExpectedOpenState, ExpectedRecoveryClass, FaultLayer, ReadOnlyOpenPolicy,
    SCENARIO_CONTRACT_VERSION, ScenarioContractV3, ScenarioRequirementLevel, scenario_contract_v3,
    validate_scenario_contract_v3, validate_scenario_contracts_v3,
};
pub use serialization::{
    CANDIDATE_KEY_SERIALIZATION_VERSION, LOCATOR_SERIALIZATION_VERSION,
    candidate_key_canonical_digest, locator_canonical_digest,
};

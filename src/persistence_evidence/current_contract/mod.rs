#[cfg(feature = "persistence-spike")]
pub mod append_authoritative;
#[cfg(feature = "persistence-spike")]
pub mod append_authoritative_01b3;
pub mod candidate_equivalence;
pub mod classification;
pub mod derivation;
#[cfg(feature = "persistence-spike")]
pub mod evidence_01c;
pub mod fixture;
pub mod measurement;
pub mod model;
pub mod oracle;
pub mod production_bridge;
pub mod projection;
pub mod readiness;
pub mod scenario_contract;
pub mod serialization;
#[cfg(feature = "persistence-spike")]
pub mod sqlite_authoritative;
pub mod violations;

#[cfg(feature = "persistence-spike")]
pub use append_authoritative::{
    APPEND_AUTHORITATIVE_CANDIDATE_ID, APPEND_AUTHORITATIVE_CANDIDATE_VERSION,
    APPEND_AUTHORITATIVE_FORMAT_VERSION, AppendAuthoritativeCandidateAdapter, AppendAuthorityError,
    AppendAuthoritySession, AppendCheckpointStatus, AppendOpenMode, AppendTailStatus, CleanupPlan,
    DurableAppendAck, OpenedAppendAuthoritySession,
};
#[cfg(feature = "persistence-spike")]
pub use append_authoritative_01b3::{
    infer_command_scope, AppendCommandScope, AppendScopedCommand,
    AppendScopedPreconditionCandidateAdapter, AppendScopedPreconditions,
    APPEND_SCOPED_PRECONDITION_CANDIDATE_VERSION,
};
pub use candidate_equivalence::{
    CandidateEligibilityStatus, CandidateEquivalenceRequirements,
    candidate_equivalence_requirements,
};
pub use classification::{
    FIELD_CLASSIFICATION_REGISTRY, FieldClassification, canonical_classes, derived_classes,
    excluded_classes,
};
pub use derivation::{derive_contract_projection, finalize_derived_fields};
#[cfg(feature = "persistence-spike")]
pub use evidence_01c::{EVIDENCE_01C_HARNESS_VERSION, methodology_record, run_01c_evidence};
pub use fixture::{
    GOLDEN_SMALL_TRANSCRIPT, OFFSET_ANCHOR_TRANSCRIPT, VARIANT_BASE_MANUAL_REPLACEMENT,
    VARIANT_CANDIDATE_REJECTED, VARIANT_DUPLICATED_LINEAGE, VARIANT_OFFSET_ANCHOR,
    VARIANT_PROMOTED_ACTIVE, VARIANT_REVOKED_HISTORICAL, VARIANT_SUPERSEDED, all_fixture_variants,
    build_base_manual_replacement_state, build_candidate_rejected_state,
    build_duplicated_session_lineage_state, build_golden_small_session, build_golden_small_state,
    build_medium_fixture_state, build_offset_anchor_manual_replacement_state,
    build_original_for_duplication_fixture, build_pre_supersession_run_session,
    build_promoted_active_session, build_promoted_active_state,
    build_promoted_then_run_then_superseded_state, build_revoked_historical_state,
    build_superseded_session, build_superseded_state, build_superseded_with_reuse_run_state,
    golden_small, medium_fixture_dimensions, medium_fixture_serialized_semantic_size_bytes,
};
pub use measurement::{
    ComparativeMeasurementContract, DeferredFixtureScale, MEASUREMENT_CONTRACT_VERSION,
    MeasurementFixtureScale, MinimumEnvironmentMetadata, comparative_measurement_contract,
    validate_measurement_contract, validate_measurement_contract_value,
};
pub use model::{
    CURRENT_CONTRACT_FIXTURE_ID, CURRENT_CONTRACT_FIXTURE_VERSION, CurrentContractFixture,
    CurrentContractFixtureScale, CurrentContractState, DerivedContractProjection,
    EvidenceCorrectionDecision, EvidenceReuseCandidateKey, EvidenceReuseGovernanceEvent,
    EvidenceSourceDecisionLocator,
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
    SCENARIO_CONTRACT_VERSION, SCENARIO_CONTRACT_VERSION_V3, ScenarioContractV3,
    ScenarioRequirementLevel, scenario_contract_v3, scenario_contract_v4,
    validate_scenario_contract_v3, validate_scenario_contract_v4, validate_scenario_contracts_v3,
    validate_scenario_contracts_v4,
};
pub use serialization::{
    CANDIDATE_KEY_SERIALIZATION_VERSION, LOCATOR_SERIALIZATION_VERSION,
    candidate_key_canonical_digest, locator_canonical_digest,
};
#[cfg(feature = "persistence-spike")]
pub use sqlite_authoritative::{
    CurrentContractPreconditions, DurableSqliteAck, OpenedSqliteAuthoritySession,
    SQLITE_AUTHORITATIVE_CANDIDATE_ID, SQLITE_AUTHORITATIVE_CANDIDATE_VERSION,
    SQLITE_AUTHORITATIVE_FORMAT_VERSION, SqliteAuthoritativeCandidateAdapter, SqliteAuthorityError,
    SqliteAuthoritySession, SqliteOpenMode,
};

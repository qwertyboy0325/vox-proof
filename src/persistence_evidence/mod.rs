//! Experimental persistence evidence harness for accepted MD-014/MD-015.
//!
//! Spike-only module. Not production persistence.

mod adapter;
mod aggregation;
#[cfg(feature = "persistence-spike")]
pub mod candidates;
#[cfg(feature = "persistence-spike")]
mod canonical_sql_reader;
#[cfg(feature = "persistence-spike")]
mod cross_platform;
pub mod current_contract;
#[cfg(feature = "persistence-spike")]
mod durability;
mod fixture;
#[cfg(feature = "persistence-spike")]
mod independent_oracle;
mod metadata;
mod model;
mod oracle;
#[cfg(feature = "persistence-spike")]
mod platform;
#[cfg(feature = "persistence-spike")]
mod process_harness;
mod readiness_result;
mod runner;
mod scenario;
#[cfg(feature = "persistence-spike")]
mod scenario_runner;
#[cfg(feature = "persistence-spike")]
mod sqlite_scenario_runner;

pub use aggregation::{
    aggregate_readiness_from_json, aggregate_validated_metadata_internal_error_test,
    evaluate_persistence_readiness, serialize_readiness_result,
};
pub use current_contract::{
    CURRENT_CONTRACT_FIXTURE_ID, CURRENT_CONTRACT_FIXTURE_VERSION, CURRENT_CONTRACT_ORACLE_VERSION,
    CandidateEligibilityStatus, CandidateEquivalenceRequirements, ComparativeMeasurementContract,
    CurrentContractFixture, CurrentContractOracle, CurrentContractReadiness, CurrentContractState,
    EvidenceCorrectionDecision, EvidenceReuseCandidateKey, EvidenceReuseGovernanceEvent,
    EvidenceSourceDecisionLocator, ExpectedOpenState, ExpectedRecoveryClass,
    FIELD_CLASSIFICATION_REGISTRY, FaultLayer, FieldClassification, MEASUREMENT_CONTRACT_VERSION,
    OracleViolationCodeV3, SCENARIO_CONTRACT_VERSION, ScenarioRequirementLevel,
    VARIANT_PROMOTED_ACTIVE, all_fixture_variants, build_base_manual_replacement_state,
    build_candidate_rejected_state, build_duplicated_session_lineage_state,
    build_golden_small_session, build_golden_small_state,
    build_offset_anchor_manual_replacement_state, build_original_for_duplication_fixture,
    build_promoted_active_session, build_promoted_active_state, build_revoked_historical_state,
    build_superseded_state, candidate_equivalence_requirements, candidate_key_canonical_digest,
    candidate_key_identity_digest, canonical_classes, canonical_fingerprint,
    comparative_measurement_contract, current_contract_readiness, derive_contract_projection,
    derived_classes, excluded_classes, golden_small, project_current_contract_state,
    scenario_contract_v3, validate_measurement_contract, validate_measurement_contract_value,
    validate_scenario_contract_v3, validate_scenario_contracts_v3,
};
pub use metadata::{
    MetadataValidationError, SUPPORTED_CONTRACT_VERSION, SUPPORTED_SCENARIO_CATALOG_VERSION,
    ValidatedClaimContractDocument, contract_json_with_mutation, parse_and_validate_contract,
    parse_and_validate_metadata_bundle, parse_and_validate_reclassification,
};
pub use readiness_result::{
    MetadataValidationFailure, PositiveEvidenceEntry, READINESS_RESULT_VERSION,
    ReadinessAggregationOutput, ReadinessBlocker, ReadinessBlockerCode, ReadinessOutcome,
    ReadinessResultKind,
};

pub use adapter::{
    AdapterError, AuthoritativeCommand, CandidateCapabilities, DuplicatedSession,
    EvidenceSessionHandle, EvidenceSessionRef, MaintenanceOperation, OptionalCapability,
    OptionalOperationOutcome, PersistenceCandidateAdapter, SemanticOpenMode, SemanticPrecondition,
};
#[cfg(feature = "persistence-spike")]
pub use candidates::{AppendBundleAdapter, EmbeddedRelationalAdapter};
#[cfg(feature = "persistence-spike")]
pub use cross_platform::{PlatformMatrixDocument, build_platform_matrix, compare_scenario};
#[cfg(feature = "persistence-spike")]
pub use durability::{
    DurabilityExperimentSpec, DurabilityTrialResult, DurabilityTrialRunner, MIN_TRIALS_PER_POINT,
    TrialOutcome, durability_experiments,
};
pub use fixture::{EvidenceFixture, FixtureScale, SMALL_FIXTURE_ID, SMALL_FIXTURE_VERSION};
#[cfg(feature = "persistence-spike")]
pub use independent_oracle::{
    INDEPENDENT_ORACLE_VERSION, IndependentSqliteOracle, OracleObservationRecord,
};
pub use model::{
    ActiveAnalysisSelection, AnalysisResultState, ArtifactClass, ArtifactState,
    CanonicalEventProvenance, EvidenceAggregationIssue, EvidenceAggregationIssueCode,
    EvidenceManifest, EvidenceRunEligibility, EvidenceRunResult, EvidenceRunSummary,
    KnowledgeSnapshotReference, KnownOrUnavailable, LineageConflict, LineageConflictKind,
    NormalizedAnchor, NormalizedReviewCase, NormalizedSemanticState, NormalizedSourceRevision,
    RecoveryClassification, RetentionReference, RetentionRelation, RetentionRootClass,
    ReviewCaseOrigin, ReviewCaseRaisedEventState, ReviewLedgerAction, ReviewLedgerEventState,
    ScenarioMeasurement, ScenarioResult, ScenarioStatus, SessionIdentityState, SourceSegmentState,
};
pub use oracle::{
    ORACLE_VERSION, OracleDiagnostic, OracleResult, OracleViolationCode, SemanticOracle,
};
#[cfg(feature = "persistence-spike")]
pub use platform::{
    CROSS_PLATFORM_SCENARIO_IDS, DirectorySyncCapability, PACKAGE_2C_EVIDENCE_RUN, PACKAGE_2C_HEAD,
    PlatformEquivalenceResult, PlatformProfile, PlatformScenarioRow, SqlitePragmaSnapshot,
    V3_HARNESS_VERSION, filesystem_safe_path_segment, normalize_platform_label,
};
#[cfg(feature = "persistence-spike")]
pub use process_harness::{ProcessEventRecord, ProcessExitClassification, ProcessHarness};
pub use runner::{EvidenceHarness, HARNESS_VERSION};
pub use scenario::{
    FailureModel, REQUIRED_SCENARIO_IDS, SCENARIO_CATALOG_VERSION, ScenarioCategory,
    ScenarioEvidenceKind, ScenarioIdentity, ScenarioRequirement, scenario_catalog,
};
#[cfg(feature = "persistence-spike")]
pub use scenario_runner::{ScenarioRunner, fresh_storage_root};
#[cfg(feature = "persistence-spike")]
pub use sqlite_scenario_runner::{
    FaultExecutionRecord, SQLITE_EVIDENCE_HARNESS_VERSION, SqliteEvidenceArtifacts,
    SqliteScenarioRunner,
};

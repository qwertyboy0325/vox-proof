//! Experimental, pre-authorization evidence-harness draft for proposed MD-015.
//!
//! This module models semantic fixtures, candidate-neutral observations, and
//! evidence results. It is not a production session format, persistence
//! implementation, mechanism selection, or product data contract. MD-015
//! remains proposed; this API may change without compatibility guarantees.
//! Production modules must not depend on it.

mod adapter;
mod fixture;
mod model;
mod oracle;
mod runner;
mod scenario;

pub use adapter::{
    AdapterError, AuthoritativeCommand, CandidateCapabilities, DuplicatedSession,
    EvidenceSessionHandle, EvidenceSessionRef, MaintenanceOperation, OptionalCapability,
    OptionalOperationOutcome, PersistenceCandidateAdapter, SemanticOpenMode, SemanticPrecondition,
};
pub use fixture::{EvidenceFixture, FixtureScale, SMALL_FIXTURE_ID, SMALL_FIXTURE_VERSION};
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
pub use runner::{EvidenceHarness, HARNESS_VERSION};
pub use scenario::{
    FailureModel, REQUIRED_SCENARIO_IDS, SCENARIO_CATALOG_VERSION, ScenarioCategory,
    ScenarioEvidenceKind, ScenarioIdentity, ScenarioRequirement, scenario_catalog,
};

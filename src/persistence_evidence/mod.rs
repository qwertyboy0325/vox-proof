//! Experimental persistence evidence harness for accepted MD-014/MD-015.
//!
//! Spike-only module. Not production persistence.

mod adapter;
#[cfg(feature = "persistence-spike")]
pub mod candidates;
mod fixture;
mod model;
mod oracle;
mod runner;
mod scenario;
#[cfg(feature = "persistence-spike")]
mod scenario_runner;

pub use adapter::{
    AdapterError, AuthoritativeCommand, CandidateCapabilities, DuplicatedSession,
    EvidenceSessionHandle, EvidenceSessionRef, MaintenanceOperation, OptionalCapability,
    OptionalOperationOutcome, PersistenceCandidateAdapter, SemanticOpenMode, SemanticPrecondition,
};
#[cfg(feature = "persistence-spike")]
pub use candidates::{AppendBundleAdapter, EmbeddedRelationalAdapter};
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
#[cfg(feature = "persistence-spike")]
pub use scenario_runner::{ScenarioRunner, fresh_storage_root};

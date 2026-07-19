use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::oracle::OracleResult;
use super::scenario::ScenarioIdentity;

/// Spike-only normalized session identity. These fields are not a production
/// persistence schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionIdentityState {
    pub session_id: String,
    pub duplicated_from_session_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSegmentState {
    pub cue_index: u32,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedSourceRevision {
    pub revision_id: String,
    pub predecessor_revision_id: Option<String>,
    pub segments: Vec<SourceSegmentState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedAnchor {
    pub source_revision_id: String,
    pub segment_position: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewCaseOrigin {
    DetectorRaised { analysis_result_id: String },
    HumanRaised { creation_event_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedReviewCase {
    pub case_id: String,
    pub origin: ReviewCaseOrigin,
    pub observed_revision_id: String,
    pub anchor: NormalizedAnchor,
    /// Must remain absent unless a future accepted decision explicitly
    /// authorizes decision migration. The oracle treats any value as an
    /// automatic-migration violation.
    pub copied_decision_from_case_id: Option<String>,
}

/// Spike-only canonical history for explicitly raising a HumanRaised case.
/// This is distinct from correction-decision authority in ReviewLedger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewCaseRaisedEventState {
    pub event_id: String,
    pub sequence: u64,
    pub case_id: String,
    pub observed_revision_id: String,
    pub anchor: NormalizedAnchor,
    pub provenance: CanonicalEventProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewLedgerAction {
    AcceptAlternative { alternative_index: usize },
    ManualReplacement { replacement_text: String },
    Withdraw { target_event_id: String },
    Supersede { target_event_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CanonicalEventProvenance {
    Human,
    Recovery,
    AutomaticMigration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewLedgerEventState {
    pub event_id: String,
    pub sequence: u64,
    pub case_id: String,
    pub observed_revision_id: String,
    pub action: ReviewLedgerAction,
    pub provenance: CanonicalEventProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisResultState {
    pub analysis_result_id: String,
    pub source_revision_id: String,
    pub knowledge_snapshot_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveAnalysisSelection {
    pub analysis_result_id: String,
    pub selection_event_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeSnapshotReference {
    pub knowledge_snapshot_id: String,
    pub referenced_by_analysis_result_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineageConflictKind {
    AmbiguousCaseLineage,
    ConflictingKnowledgeReference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineageConflict {
    pub conflict_id: String,
    pub kind: LineageConflictKind,
    pub related_case_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactClass {
    ReferencedHistorical,
    UnreferencedHistorical,
    RebuildableDerived,
    Temporary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactState {
    pub artifact_id: String,
    pub class: ArtifactClass,
    pub content_marker: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RetentionRootClass {
    ReviewLedgerEvent,
    ReviewCaseRaisedEvent,
    AnalysisResult,
    SourceRevision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RetentionRelation {
    PreservesHistoricalProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionReference {
    pub root_id: String,
    pub root_class: RetentionRootClass,
    pub artifact_id: String,
    pub relation: RetentionRelation,
}

/// Candidate-neutral logical state consumed by the semantic oracle.
///
/// Canonical event order is preserved. Logically unordered collections are
/// sorted by stable logical identity by [`NormalizedSemanticState::normalize`].
/// Derived and temporary artifacts remain observable but are excluded from
/// canonical comparison and fingerprints. Physical storage metadata,
/// environment timestamps, lock details, filenames, offsets, and transaction
/// identifiers do not belong here. Authoritative text is preserved exactly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedSemanticState {
    pub session: SessionIdentityState,
    pub session_format_version: String,
    pub interpretation_version: String,
    pub source_revisions: Vec<NormalizedSourceRevision>,
    pub review_cases: Vec<NormalizedReviewCase>,
    pub review_case_raised_events: Vec<ReviewCaseRaisedEventState>,
    pub review_ledger_events: Vec<ReviewLedgerEventState>,
    pub analysis_results: Vec<AnalysisResultState>,
    pub active_analysis_selection: Option<ActiveAnalysisSelection>,
    pub knowledge_snapshot_references: Vec<KnowledgeSnapshotReference>,
    pub lineage_conflicts: Vec<LineageConflict>,
    pub artifacts: Vec<ArtifactState>,
    pub retention_references: Vec<RetentionReference>,
}

impl NormalizedSemanticState {
    pub fn normalize(mut self) -> Self {
        self.source_revisions
            .sort_by(|left, right| left.revision_id.cmp(&right.revision_id));
        self.review_cases
            .sort_by(|left, right| left.case_id.cmp(&right.case_id));
        // ReviewCaseRaised event order is canonical and is therefore preserved.
        self.analysis_results
            .sort_by(|left, right| left.analysis_result_id.cmp(&right.analysis_result_id));
        for result in &mut self.analysis_results {
            result.knowledge_snapshot_ids.sort();
        }
        self.knowledge_snapshot_references
            .sort_by(|left, right| left.knowledge_snapshot_id.cmp(&right.knowledge_snapshot_id));
        for reference in &mut self.knowledge_snapshot_references {
            reference.referenced_by_analysis_result_ids.sort();
        }
        self.lineage_conflicts
            .sort_by(|left, right| left.conflict_id.cmp(&right.conflict_id));
        for conflict in &mut self.lineage_conflicts {
            conflict.related_case_ids.sort();
        }
        self.artifacts
            .sort_by(|left, right| left.artifact_id.cmp(&right.artifact_id));
        self.retention_references.sort_by(|left, right| {
            (
                &left.root_class,
                &left.root_id,
                &left.artifact_id,
                &left.relation,
            )
                .cmp(&(
                    &right.root_class,
                    &right.root_id,
                    &right.artifact_id,
                    &right.relation,
                ))
        });
        self
    }

    pub(crate) fn canonical_projection(&self) -> Self {
        let mut projection = self.clone().normalize();
        projection.artifacts.retain(|artifact| {
            matches!(
                artifact.class,
                ArtifactClass::ReferencedHistorical | ArtifactClass::UnreferencedHistorical
            )
        });
        projection
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnownOrUnavailable<T> {
    Known(T),
    Unavailable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceManifest {
    pub evidence_protocol_version: String,
    pub repository_commit: String,
    pub candidate_id: String,
    pub candidate_version: String,
    pub fixture_id: String,
    pub fixture_version: String,
    pub harness_version: String,
    pub oracle_version: String,
    pub scenario_ids: Vec<String>,
    pub operating_system: KnownOrUnavailable<String>,
    pub operating_system_version: KnownOrUnavailable<String>,
    pub filesystem: KnownOrUnavailable<String>,
    pub hardware_summary: KnownOrUnavailable<String>,
    pub runtime_versions: BTreeMap<String, KnownOrUnavailable<String>>,
    pub configuration: BTreeMap<String, String>,
    pub start_timestamp: KnownOrUnavailable<String>,
    pub end_timestamp: KnownOrUnavailable<String>,
    pub known_limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScenarioStatus {
    Passed,
    Failed,
    Unsupported,
    NotRun,
    Inconclusive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryClassification {
    LastCommittedState,
    SafeAutomaticRecovery,
    ManualReviewRequired,
    ReadOnlySalvage,
    Unrecoverable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScenarioMeasurement {
    Integer { value: i128, unit: String },
    Decimal { value: String, unit: String },
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioResult {
    pub scenario_identity: ScenarioIdentity,
    pub status: ScenarioStatus,
    pub oracle_result: Option<OracleResult>,
    pub measurements: BTreeMap<String, ScenarioMeasurement>,
    pub failure_classification: Option<RecoveryClassification>,
    pub limitations: Vec<String>,
    pub raw_artifact_references: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub achieved_evidence_strength: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_interruption_performed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reopen_performed: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EvidenceAggregationIssueCode {
    UnknownScenario,
    ScenarioVersionMismatch,
    ScenarioDefinitionMismatch,
    DuplicateScenarioIdentity,
    MissingRequiredScenario,
    PassedWithoutOracle,
    PassedWithFailingOracle,
    UnsupportedWithoutLimitation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceAggregationIssue {
    pub code: EvidenceAggregationIssueCode,
    pub scenario_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceRunEligibility {
    EligibleForComparison,
    NotEligible,
    Inconclusive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRunSummary {
    pub passed: usize,
    pub failed: usize,
    pub unsupported: usize,
    pub not_run: usize,
    pub inconclusive: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRunResult {
    pub manifest: EvidenceManifest,
    pub scenario_results: Vec<ScenarioResult>,
    pub negative_results: Vec<ScenarioResult>,
    pub aggregation_issues: Vec<EvidenceAggregationIssue>,
    pub summary: EvidenceRunSummary,
    /// Eligibility is evidence output, not acceptance or production authority.
    pub eligibility: EvidenceRunEligibility,
}

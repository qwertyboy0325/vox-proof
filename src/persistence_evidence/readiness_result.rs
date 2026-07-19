use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const READINESS_RESULT_VERSION: &str = "persistence-readiness-result-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessOutcome {
    Ready,
    NotReady,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReadinessBlockerCode {
    InvalidContract,
    UnsupportedContractVersion,
    DuplicateScenarioId,
    DuplicateSubclaimId,
    InvalidClaimStructure,
    InvalidStatusEvidenceCombination,
    UnsupportedScenarioNoCredit,
    DeferredClaimNoCredit,
    UnresolvedCorrectionRequired,
    NotDemonstratedSubclaim,
    MechanismBlockerRetained,
    FailClosedInvariantActive,
    SelectionProhibitedWhileNotReady,
    MissingReclassificationAuthority,
    AggregationBlockedByMissingAuthority,
    EmptyRequiredString,
    InvalidInvariantMetadata,
    MissingInvariantField,
    UnexpectedInvariantField,
    InvalidUnsupportedScenarioShape,
    InvalidFinalConflictCorrection,
    MissingResolvedConflictId,
    UnexpectedResolvedConflictId,
    InvalidPriorReviewCommit,
    MissingCandidateSpecificBlockers,
    MissingCandidateBlocker,
    MalformedCandidateBlocker,
    InternalInvariantViolation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadinessBlocker {
    pub code: ReadinessBlockerCode,
    pub scenario_id: Option<String>,
    pub subclaim_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositiveEvidenceEntry {
    pub scenario_id: String,
    pub subclaim_id: Option<String>,
    pub evidence_strength: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadinessAggregationOutput {
    pub result_version: String,
    pub outcome_kind: ReadinessResultKind,
    pub contract_version: Option<String>,
    pub scenario_count: usize,
    pub subclaim_count: usize,
    pub validated_scenario_count: usize,
    pub invalid_scenario_count: usize,
    pub status_counts: BTreeMap<String, usize>,
    pub evidence_strength_counts: BTreeMap<String, usize>,
    pub unsupported_scenario_ids: Vec<String>,
    pub deferred_scenario_ids: Vec<String>,
    pub correction_required_scenario_ids: Vec<String>,
    pub partially_demonstrated_scenario_ids: Vec<String>,
    pub not_demonstrated_subclaim_ids: Vec<String>,
    pub positive_evidence_by_scenario_or_subclaim: Vec<PositiveEvidenceEntry>,
    pub mechanism_comparison_readiness: ReadinessOutcome,
    pub mechanism_selection_readiness: ReadinessOutcome,
    pub selection_status: String,
    pub blocking_reasons: Vec<ReadinessBlocker>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReadinessResultKind {
    ValidNotReady,
    ValidReady,
    InvalidInput,
    UnsupportedContractVersion,
    AggregationBlocked,
    InternalValidationError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetadataValidationFailure {
    pub code: ReadinessBlockerCode,
    pub message: String,
    pub scenario_id: Option<String>,
    pub subclaim_id: Option<String>,
}

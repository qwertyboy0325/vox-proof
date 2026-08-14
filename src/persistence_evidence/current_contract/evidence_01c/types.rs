use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::super::candidate_equivalence::CandidateEligibilityStatus;
use super::super::measurement::MeasurementFixtureScale;
use super::super::scenario_contract::{ExpectedOpenState, ExpectedRecoveryClass};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterleavingStrategy {
    AlternatingPerSample,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioExecutionStatus {
    Passed,
    Failed,
    Unsupported,
    NotRun,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrectnessDisqualification {
    pub gate: String,
    pub scenario_id: Option<String>,
    pub candidate_id: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedScenarioResult {
    pub scenario_id: String,
    pub scenario_version: u32,
    pub candidate_id: String,
    pub candidate_version: String,
    pub platform: String,
    pub fixture_scale: String,
    pub status: ScenarioExecutionStatus,
    pub oracle_compare: bool,
    pub recovery_class: String,
    pub open_state: String,
    pub failure_code: Option<String>,
    pub elapsed_ms: u128,
    pub correctness_disqualification: Option<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetricAvailability<T> {
    pub value: Option<T>,
    pub status: String,
    pub reason: Option<String>,
}

impl<T> MetricAvailability<T> {
    pub fn available(value: T) -> Self {
        Self {
            value: Some(value),
            status: "available".to_owned(),
            reason: None,
        }
    }

    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            value: None,
            status: "unavailable".to_owned(),
            reason: Some(reason.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeasurementSample {
    pub elapsed_ms: u128,
    pub peak_memory_bytes: u64,
    pub storage_size_before: u64,
    pub storage_size_after: u64,
    pub bytes_read: MetricAvailability<u64>,
    pub bytes_written: MetricAvailability<u64>,
    pub failed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeasurementAggregate {
    pub operation: String,
    pub fixture_scale: MeasurementFixtureScale,
    pub candidate_id: String,
    pub candidate_version: String,
    pub platform: String,
    pub count: u32,
    pub minimum_ms: u128,
    pub median_ms: u128,
    pub p95_ms: u128,
    pub maximum_ms: u128,
    pub failure_count: u32,
    pub peak_memory_bytes: MetricAvailability<u64>,
    pub bytes_read: MetricAvailability<u64>,
    pub bytes_written: MetricAvailability<u64>,
    pub storage_size_before: MetricAvailability<u64>,
    pub storage_size_after: MetricAvailability<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateEligibilityRecord {
    pub candidate_id: String,
    pub candidate_version: String,
    pub status: CandidateEligibilityStatus,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvironmentRecord {
    pub repository_commit: String,
    pub harness_version: String,
    pub platform_label: String,
    pub operating_system: String,
    pub operating_system_version: String,
    pub architecture: String,
    pub execution_environment: String,
    pub filesystem: String,
    pub hardware_summary: String,
    pub rustc_version: String,
    pub dependency_versions: BTreeMap<String, String>,
    pub timing_source: String,
    pub start_timestamp: String,
    pub end_timestamp: Option<String>,
    pub configuration: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediumFixtureRecord {
    pub fixture_id: String,
    pub fixture_version: String,
    pub scale: String,
    pub oracle_validated: bool,
    pub dimensions: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateRunArtifacts {
    pub candidate_id: String,
    pub candidate_version: String,
    pub scenario_results: Vec<NormalizedScenarioResult>,
    pub measurements: Vec<MeasurementAggregate>,
    pub disqualifications: Vec<CorrectnessDisqualification>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformComparisonSection {
    pub platform: String,
    pub tradeoffs: Vec<String>,
    pub append_disqualified: bool,
    pub sqlite_disqualified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvalidPredecessorEvidence {
    pub harness_sha: String,
    pub evidence_record_sha: String,
    pub verdict: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComparativeEvidencePackage {
    pub work_package_id: String,
    pub harness_semantic_sha: String,
    pub environment: EnvironmentRecord,
    pub methodology: super::methodology::MethodologyRecord,
    pub medium_fixture: MediumFixtureRecord,
    pub eligibility: Vec<CandidateEligibilityRecord>,
    pub append: CandidateRunArtifacts,
    pub sqlite: CandidateRunArtifacts,
    pub comparison: PlatformComparisonSection,
    pub limitations: Vec<String>,
    pub mechanism_comparison_readiness: String,
    pub mechanism_selection_readiness: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub predecessor_invalid_evidence: Option<InvalidPredecessorEvidence>,
}

pub fn recovery_class_label(value: ExpectedRecoveryClass) -> String {
    match value {
        ExpectedRecoveryClass::None => "none".to_owned(),
        ExpectedRecoveryClass::LastCommittedState => "last_committed_state".to_owned(),
        ExpectedRecoveryClass::SafeAutomaticRecovery => "safe_automatic_recovery".to_owned(),
        ExpectedRecoveryClass::ManualReviewRequired => "manual_review_required".to_owned(),
        ExpectedRecoveryClass::ReadOnlySalvage => "read_only_salvage".to_owned(),
        ExpectedRecoveryClass::Unrecoverable => "unrecoverable".to_owned(),
        ExpectedRecoveryClass::UnsupportedVersion => "unsupported_version".to_owned(),
    }
}

pub fn open_state_label(value: ExpectedOpenState) -> String {
    match value {
        ExpectedOpenState::Normal => "normal".to_owned(),
        ExpectedOpenState::UnsupportedVersion => "unsupported_version".to_owned(),
        ExpectedOpenState::ReadOnlySalvage => "read_only_salvage".to_owned(),
        ExpectedOpenState::Unrecoverable => "unrecoverable".to_owned(),
    }
}

pub fn fixture_scale_label(scale: MeasurementFixtureScale) -> String {
    match scale {
        MeasurementFixtureScale::Small => "small".to_owned(),
        MeasurementFixtureScale::Medium => "medium".to_owned(),
        MeasurementFixtureScale::Stress => "stress".to_owned(),
    }
}

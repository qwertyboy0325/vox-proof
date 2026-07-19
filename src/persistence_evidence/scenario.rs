use serde::{Deserialize, Serialize};

use super::adapter::OptionalCapability;

pub const SCENARIO_CATALOG_VERSION: &str = "1";
pub const REQUIRED_SCENARIO_IDS: &[&str] = &[
    "baseline-create-open-close",
    "append-correction-event",
    "attach-analysis-result",
    "stale-review-ledger-command",
    "stale-active-analysis-selection",
    "stale-analysis-attachment",
    "concurrent-writer-attempt",
    "unknown-newer-format",
    "derived-state-corruption",
    "canonical-reference-corruption",
    "semantic-duplication",
    "interrupted-authoritative-transition",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScenarioCategory {
    Baseline,
    AuthoritativeCommand,
    Concurrency,
    FormatCompatibility,
    Corruption,
    Recovery,
    Maintenance,
    Duplication,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureModel {
    None,
    LogicalFaultPoint,
    ConcurrentAccess,
    MalformedOrUnsupportedInput,
    ProcessInterruption,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScenarioRequirement {
    Required,
    CapabilityDependent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScenarioEvidenceKind {
    SemanticCorrectness,
    MeasurementOnly,
}

/// Stable, versioned identity for one evidence scenario. Descriptions may be
/// clarified without changing identity only when the observable workload is
/// unchanged; semantic workload changes require a version increment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioIdentity {
    pub scenario_id: String,
    pub scenario_version: u32,
    pub category: ScenarioCategory,
    pub description: String,
    pub failure_model: FailureModel,
    pub required_capabilities: Vec<OptionalCapability>,
    pub requirement: ScenarioRequirement,
    pub evidence_kind: ScenarioEvidenceKind,
}

fn scenario(
    id: &str,
    category: ScenarioCategory,
    description: &str,
    failure_model: FailureModel,
) -> ScenarioIdentity {
    ScenarioIdentity {
        scenario_id: id.to_string(),
        scenario_version: 1,
        category,
        description: description.to_string(),
        failure_model,
        required_capabilities: Vec::new(),
        requirement: ScenarioRequirement::Required,
        evidence_kind: ScenarioEvidenceKind::SemanticCorrectness,
    }
}

fn optional_scenario(
    id: &str,
    category: ScenarioCategory,
    description: &str,
    capability: OptionalCapability,
) -> ScenarioIdentity {
    ScenarioIdentity {
        scenario_id: id.to_string(),
        scenario_version: 1,
        category,
        description: description.to_string(),
        failure_model: FailureModel::ProcessInterruption,
        required_capabilities: vec![capability],
        requirement: ScenarioRequirement::CapabilityDependent,
        evidence_kind: ScenarioEvidenceKind::SemanticCorrectness,
    }
}

pub fn scenario_catalog() -> Vec<ScenarioIdentity> {
    vec![
        scenario(
            "baseline-create-open-close",
            ScenarioCategory::Baseline,
            "Create, open, read normalized state, and close",
            FailureModel::None,
        ),
        scenario(
            "append-correction-event",
            ScenarioCategory::AuthoritativeCommand,
            "Append one representative correction event",
            FailureModel::None,
        ),
        scenario(
            "attach-analysis-result",
            ScenarioCategory::AuthoritativeCommand,
            "Attach one immutable analysis result",
            FailureModel::None,
        ),
        scenario(
            "stale-review-ledger-command",
            ScenarioCategory::Concurrency,
            "Reject a stale ReviewLedger command",
            FailureModel::ConcurrentAccess,
        ),
        scenario(
            "stale-active-analysis-selection",
            ScenarioCategory::Concurrency,
            "Reject a stale active-analysis selection",
            FailureModel::ConcurrentAccess,
        ),
        scenario(
            "stale-analysis-attachment",
            ScenarioCategory::Concurrency,
            "Reject a stale analysis attachment",
            FailureModel::ConcurrentAccess,
        ),
        scenario(
            "concurrent-writer-attempt",
            ScenarioCategory::Concurrency,
            "Permit at most one authoritative writer",
            FailureModel::ConcurrentAccess,
        ),
        scenario(
            "unknown-newer-format",
            ScenarioCategory::FormatCompatibility,
            "Reject writable open for an unknown newer format",
            FailureModel::MalformedOrUnsupportedInput,
        ),
        scenario(
            "derived-state-corruption",
            ScenarioCategory::Corruption,
            "Classify and rebuild derived-state corruption",
            FailureModel::LogicalFaultPoint,
        ),
        scenario(
            "canonical-reference-corruption",
            ScenarioCategory::Corruption,
            "Detect a broken canonical reference",
            FailureModel::LogicalFaultPoint,
        ),
        scenario(
            "semantic-duplication",
            ScenarioCategory::Duplication,
            "Create a new session identity with source lineage",
            FailureModel::None,
        ),
        scenario(
            "interrupted-authoritative-transition",
            ScenarioCategory::Recovery,
            "Interrupt an authoritative transition around acknowledgement",
            FailureModel::ProcessInterruption,
        ),
        optional_scenario(
            "interrupted-compaction",
            ScenarioCategory::Maintenance,
            "Interrupt candidate-supported compaction",
            OptionalCapability::Compaction,
        ),
        optional_scenario(
            "interrupted-cleanup",
            ScenarioCategory::Maintenance,
            "Interrupt candidate-supported destructive historical cleanup",
            OptionalCapability::DestructiveHistoricalGc,
        ),
    ]
}

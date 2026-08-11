use serde::{Deserialize, Serialize};

use super::types::InterleavingStrategy;

pub const EVIDENCE_01C_HARNESS_VERSION: &str = "gate4-01c-evidence-v1";
pub const METHODOLOGY_FREEZE_ID: &str = "VP-GATE4-EVIDENCE-COMPLETION-01C-methodology-freeze-v1";

pub const INTERLEAVING_STRATEGY: InterleavingStrategy = InterleavingStrategy::AlternatingPerSample;

/// Frozen before any comparative measurement execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MethodologyRecord {
    pub freeze_id: String,
    pub harness_version: String,
    pub work_package_id: String,
    pub candidates_order: Vec<String>,
    pub fixture_id: String,
    pub fixture_version: String,
    pub oracle_version: String,
    pub scenario_contract_version: String,
    pub measurement_contract_version: String,
    pub small_fixture: String,
    pub medium_fixture: String,
    pub stress_fixture_policy: String,
    pub scenario_execution_order: String,
    pub correctness_gate_order: String,
    pub measurement_operation_order: String,
    pub warmup_strategy: String,
    pub sample_interleaving_strategy: String,
    pub timing_source: String,
    pub peak_memory_strategy: String,
    pub bytes_read_write_strategy: String,
    pub storage_size_strategy: String,
    pub environment_metadata_strategy: String,
    pub macos_strategy: String,
    pub windows_strategy: String,
    pub comparison_algorithm: String,
    pub missing_metric_policy: String,
    pub capability_dependent_policy: String,
    pub platform_pooling_performed: bool,
}

pub fn methodology_record() -> MethodologyRecord {
    MethodologyRecord {
        freeze_id: METHODOLOGY_FREEZE_ID.to_owned(),
        harness_version: EVIDENCE_01C_HARNESS_VERSION.to_owned(),
        work_package_id: "VP-GATE4-EVIDENCE-COMPLETION-01C".to_owned(),
        candidates_order: vec![
            "current-contract-append-authoritative-candidate".to_owned(),
            "current-contract-sqlite-authoritative-candidate".to_owned(),
        ],
        fixture_id: super::super::model::CURRENT_CONTRACT_FIXTURE_ID.to_owned(),
        fixture_version: super::super::model::CURRENT_CONTRACT_FIXTURE_VERSION.to_owned(),
        oracle_version: super::super::oracle::CURRENT_CONTRACT_ORACLE_VERSION.to_owned(),
        scenario_contract_version: super::super::scenario_contract::SCENARIO_CONTRACT_VERSION
            .to_owned(),
        measurement_contract_version: super::super::measurement::MEASUREMENT_CONTRACT_VERSION
            .to_owned(),
        small_fixture: "golden_small promoted_active variant".to_owned(),
        medium_fixture: "scaled_multi_lifecycle_current_contract_v3".to_owned(),
        stress_fixture_policy: "not_implemented_record_limitation".to_owned(),
        scenario_execution_order: "scenario_contract_v3 declaration order".to_owned(),
        correctness_gate_order: "all required scenarios before any performance samples".to_owned(),
        measurement_operation_order: "comparative_measurement_contract declaration order"
            .to_owned(),
        warmup_strategy: "contract warmup_count per operation before interleaved samples"
            .to_owned(),
        sample_interleaving_strategy: "alternating_per_sample: even sample append-first, odd sqlite-first"
            .to_owned(),
        timing_source: "std::time::Instant elapsed wall clock per operation".to_owned(),
        peak_memory_strategy:
            "process peak RSS via getrusage (unix) or GetProcessMemoryInfo PeakWorkingSetSize (windows)"
                .to_owned(),
        bytes_read_write_strategy:
            "unavailable unless candidate-neutral filesystem observation exists".to_owned(),
        storage_size_strategy: "recursive byte sum of candidate session storage directory".to_owned(),
        environment_metadata_strategy: "capture_environment at run start from live host".to_owned(),
        macos_strategy: "native host execution via persistence_evidence_01c_run".to_owned(),
        windows_strategy: "github-actions windows-latest workflow step".to_owned(),
        comparison_algorithm: "paired within-platform tradeoff report without cross-platform pooling"
            .to_owned(),
        missing_metric_policy: "explicit unavailable sentinel; never encode as zero".to_owned(),
        capability_dependent_policy:
            "unsupported capability yields ScenarioExecutionStatus::Unsupported with limitation"
                .to_owned(),
        platform_pooling_performed: false,
    }
}

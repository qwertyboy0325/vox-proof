use serde::{Deserialize, Serialize};

pub const MEASUREMENT_CONTRACT_VERSION: &str = "2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementFixtureScale {
    Small,
    Medium,
    Stress,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeasurementOperationSpec {
    pub operation: String,
    pub fixture_scales: Vec<MeasurementFixtureScale>,
    pub sample_count: u32,
    pub warmup_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeferredFixtureScale {
    pub scale: MeasurementFixtureScale,
    pub rationale: String,
    pub required_future_package: String,
    pub unblock_condition: String,
    pub selection_impact: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinimumEnvironmentMetadata {
    pub repository_commit: bool,
    pub candidate_id_version: bool,
    pub fixture_version: bool,
    pub oracle_version: bool,
    pub scenario_contract_version: bool,
    pub os_version: bool,
    pub execution_environment: bool,
    pub filesystem: bool,
    pub hardware_summary: bool,
    pub rustc_version: bool,
    pub dependency_versions: bool,
    pub sample_timing_source: bool,
    pub start_end_timestamps: bool,
    pub configuration: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeasurementAggregationFields {
    pub count: bool,
    pub minimum: bool,
    pub median: bool,
    pub p95: bool,
    pub maximum: bool,
    pub failure_count: bool,
    pub peak_memory_bytes: bool,
    pub bytes_read_if_available: bool,
    pub bytes_written_if_available: bool,
    pub storage_size_before: bool,
    pub storage_size_after: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComparativeMeasurementContract {
    pub contract_version: String,
    pub fixture_id: String,
    pub fixture_version: String,
    pub operations: Vec<MeasurementOperationSpec>,
    pub aggregation_fields: MeasurementAggregationFields,
    pub correctness_disqualification_gates: Vec<String>,
    pub resource_bound_refusal_requirements: Vec<String>,
    pub deferred_scales: Vec<DeferredFixtureScale>,
    pub minimum_environment_metadata: MinimumEnvironmentMetadata,
}

const REQUIRED_OPERATIONS: &[&str] = &[
    "create_session",
    "open_cold",
    "open_warm",
    "append_review_decision",
    "append_manual_replacement",
    "append_reusable_promotion",
    "append_reusable_revocation",
    "append_reusable_supersession",
    "close",
    "reopen_and_validate",
    "semantic_duplication",
    "derived_rebuild",
    "compaction_where_supported",
];

pub fn comparative_measurement_contract() -> ComparativeMeasurementContract {
    ComparativeMeasurementContract {
        contract_version: MEASUREMENT_CONTRACT_VERSION.to_owned(),
        fixture_id: super::model::CURRENT_CONTRACT_FIXTURE_ID.to_owned(),
        fixture_version: super::model::CURRENT_CONTRACT_FIXTURE_VERSION.to_owned(),
        operations: vec![
            op("create_session", &[MeasurementFixtureScale::Small], 5, 1),
            op("open_cold", &[MeasurementFixtureScale::Small], 5, 1),
            op("open_warm", &[MeasurementFixtureScale::Small], 5, 2),
            op(
                "append_review_decision",
                &[MeasurementFixtureScale::Small],
                5,
                1,
            ),
            op(
                "append_manual_replacement",
                &[MeasurementFixtureScale::Small],
                5,
                1,
            ),
            op(
                "append_reusable_promotion",
                &[MeasurementFixtureScale::Small],
                5,
                1,
            ),
            op(
                "append_reusable_revocation",
                &[MeasurementFixtureScale::Small],
                5,
                1,
            ),
            op(
                "append_reusable_supersession",
                &[MeasurementFixtureScale::Small],
                5,
                1,
            ),
            op("close", &[MeasurementFixtureScale::Small], 5, 1),
            op(
                "reopen_and_validate",
                &[MeasurementFixtureScale::Small],
                5,
                1,
            ),
            op(
                "semantic_duplication",
                &[MeasurementFixtureScale::Small],
                5,
                1,
            ),
            op("derived_rebuild", &[MeasurementFixtureScale::Small], 5, 1),
            op(
                "compaction_where_supported",
                &[MeasurementFixtureScale::Small],
                5,
                1,
            ),
        ],
        aggregation_fields: MeasurementAggregationFields {
            count: true,
            minimum: true,
            median: true,
            p95: true,
            maximum: true,
            failure_count: true,
            peak_memory_bytes: true,
            bytes_read_if_available: true,
            bytes_written_if_available: true,
            storage_size_before: true,
            storage_size_after: true,
        },
        correctness_disqualification_gates: vec![
            "failed_oracle_compare".to_owned(),
            "partial_authoritative_transition_exposed".to_owned(),
            "stale_write_accepted".to_owned(),
            "unknown_newer_format_writable".to_owned(),
        ],
        resource_bound_refusal_requirements: vec![
            "bounded_malformed_input_handling".to_owned(),
            "no_unbounded_startup_or_memory_for_small_fixture".to_owned(),
        ],
        deferred_scales: vec![
            DeferredFixtureScale {
                scale: MeasurementFixtureScale::Medium,
                rationale: "Medium fixture variants and multi-case lifecycle projections are not yet materialized in package 01A correction.".to_owned(),
                required_future_package: "VP-GATE4-EVIDENCE-COMPLETION-01C".to_owned(),
                unblock_condition: "fixture variants at medium scale implemented and validated by oracle v3".to_owned(),
                selection_impact: "Mechanism selection remains blocked until medium-scale comparative measurements are executed for both candidates.".to_owned(),
            },
            DeferredFixtureScale {
                scale: MeasurementFixtureScale::Stress,
                rationale: "Stress-scale measurements require authorized evidence execution infrastructure and larger fixture corpora not defined in correction-01.".to_owned(),
                required_future_package: "VP-GATE4-EVIDENCE-COMPLETION-01C".to_owned(),
                unblock_condition: "stress fixture corpus and bounded resource refusal gates executed under owner-authorized evidence runs".to_owned(),
                selection_impact: "Stress results inform owner tradeoffs but do not alone block correctness gates; absence must be recorded as a limitation.".to_owned(),
            },
        ],
        minimum_environment_metadata: MinimumEnvironmentMetadata {
            repository_commit: true,
            candidate_id_version: true,
            fixture_version: true,
            oracle_version: true,
            scenario_contract_version: true,
            os_version: true,
            execution_environment: true,
            filesystem: true,
            hardware_summary: true,
            rustc_version: true,
            dependency_versions: true,
            sample_timing_source: true,
            start_end_timestamps: true,
            configuration: true,
        },
    }
}

fn op(
    operation: &str,
    scales: &[MeasurementFixtureScale],
    sample_count: u32,
    warmup_count: u32,
) -> MeasurementOperationSpec {
    MeasurementOperationSpec {
        operation: operation.to_owned(),
        fixture_scales: scales.to_vec(),
        sample_count,
        warmup_count,
    }
}

pub fn validate_measurement_contract() -> Result<(), String> {
    let contract = comparative_measurement_contract();
    if !contract.aggregation_fields.count
        || !contract.aggregation_fields.median
        || !contract.aggregation_fields.p95
        || !contract.aggregation_fields.maximum
        || !contract.aggregation_fields.failure_count
    {
        return Err("measurement contract missing required aggregation fields".to_owned());
    }
    if contract.operations.is_empty() {
        return Err("measurement contract has no operations".to_owned());
    }
    let mut names = std::collections::BTreeSet::new();
    for operation in &contract.operations {
        if operation.sample_count == 0 {
            return Err(format!(
                "operation {} must have sample_count > 0",
                operation.operation
            ));
        }
        if !names.insert(operation.operation.clone()) {
            return Err(format!(
                "duplicate measurement operation {}",
                operation.operation
            ));
        }
    }
    for required in REQUIRED_OPERATIONS {
        if !names.contains(*required) {
            return Err(format!("missing required measurement operation {required}"));
        }
    }
    if contract.correctness_disqualification_gates.is_empty() {
        return Err("correctness disqualification gates must be non-empty".to_owned());
    }
    if contract.resource_bound_refusal_requirements.is_empty() {
        return Err("resource-bound refusal requirements must be non-empty".to_owned());
    }
    if !contract.operations.iter().any(|operation| {
        operation
            .fixture_scales
            .contains(&MeasurementFixtureScale::Small)
    }) {
        return Err("small fixture scale must be implemented".to_owned());
    }
    for deferred in &contract.deferred_scales {
        if deferred.rationale.trim().is_empty()
            || deferred.required_future_package.trim().is_empty()
            || deferred.unblock_condition.trim().is_empty()
            || deferred.selection_impact.trim().is_empty()
        {
            return Err(format!(
                "deferred scale {:?} missing complete rationale metadata",
                deferred.scale
            ));
        }
    }
    if !contract.minimum_environment_metadata.repository_commit
        || !contract.minimum_environment_metadata.fixture_version
        || !contract.minimum_environment_metadata.oracle_version
    {
        return Err("minimum environment metadata incomplete".to_owned());
    }
    Ok(())
}

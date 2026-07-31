use serde::{Deserialize, Serialize};

pub const MEASUREMENT_CONTRACT_VERSION: &str = "1";

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
pub struct MeasurementRecordTemplate {
    pub operation: String,
    pub fixture_scale: MeasurementFixtureScale,
    pub sample_count: u32,
    pub warmup_count: u32,
    pub count: Option<u32>,
    pub minimum_ms: Option<u64>,
    pub median_ms: Option<u64>,
    pub p95_ms: Option<u64>,
    pub maximum_ms: Option<u64>,
    pub failure_count: u32,
    pub peak_memory_bytes: Option<u64>,
    pub bytes_read_if_available: Option<u64>,
    pub bytes_written_if_available: Option<u64>,
    pub storage_size_before: Option<u64>,
    pub storage_size_after: Option<u64>,
    pub environment_metadata: Vec<String>,
    pub known_limitations: Vec<String>,
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
    pub deferred_scales: Vec<MeasurementFixtureScale>,
}

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
            MeasurementFixtureScale::Medium,
            MeasurementFixtureScale::Stress,
        ],
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
    Ok(())
}

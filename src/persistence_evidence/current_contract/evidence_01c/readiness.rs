use super::super::candidate_equivalence::CandidateEligibilityStatus;
use super::super::measurement::{comparative_measurement_contract, MeasurementFixtureScale};
use super::super::scenario_contract::{
    scenario_contract_v3, ScenarioContractV3, ScenarioRequirementLevel,
};
use super::types::{
    CandidateEligibilityRecord, CandidateRunArtifacts, MeasurementAggregate, MetricAvailability,
    ScenarioExecutionStatus,
};

pub const EXPECTED_PLATFORMS: &[&str] = &["macos_native", "windows-github_actions"];

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReadinessAssessment {
    pub mechanism_comparison_readiness: String,
    pub mechanism_selection_readiness: String,
    pub limitations: Vec<String>,
    pub blockers: Vec<String>,
}

pub fn assess_readiness(
    append: &CandidateRunArtifacts,
    sqlite: &CandidateRunArtifacts,
) -> ReadinessAssessment {
    let mut limitations = Vec::new();
    let mut blockers = Vec::new();
    let contract = comparative_measurement_contract();

    for candidate in [&append.scenario_results, &sqlite.scenario_results] {
        for scenario in scenario_contract_v3() {
            let Some(result) = candidate
                .iter()
                .find(|row| row.scenario_id == scenario.scenario_id)
            else {
                if scenario.requirement == ScenarioRequirementLevel::Required {
                    blockers.push(format!("missing scenario result {}", scenario.scenario_id));
                }
                continue;
            };
            evaluate_scenario_result(&scenario, result, &mut limitations, &mut blockers);
        }
    }

    for artifacts in [append, sqlite] {
        for operation in &contract.operations {
            for scale in &operation.fixture_scales {
                let Some(aggregate) = artifacts.measurements.iter().find(|row| {
                    row.operation == operation.operation && row.fixture_scale == *scale
                }) else {
                    blockers.push(format!(
                        "missing measurement {} {:?} for {}",
                        operation.operation, scale, artifacts.candidate_id
                    ));
                    continue;
                };
                evaluate_measurement_aggregate(aggregate, operation.sample_count, &mut blockers);
            }
        }
    }

    limitations.extend(collect_declared_limitations(append, sqlite));

    let ready = blockers.is_empty();
    ReadinessAssessment {
        mechanism_comparison_readiness: if ready {
            "ready_for_owner_decision".to_owned()
        } else {
            "not_ready".to_owned()
        },
        mechanism_selection_readiness: if ready {
            "ready_for_owner_decision".to_owned()
        } else {
            "not_ready".to_owned()
        },
        limitations,
        blockers,
    }
}

pub fn compute_eligibility(artifacts: &CandidateRunArtifacts) -> CandidateEligibilityRecord {
    let mut blockers = Vec::new();
    for scenario in scenario_contract_v3() {
        let Some(result) = artifacts
            .scenario_results
            .iter()
            .find(|row| row.scenario_id == scenario.scenario_id)
        else {
            if scenario.requirement == ScenarioRequirementLevel::Required {
                blockers.push(format!("missing {}", scenario.scenario_id));
            }
            continue;
        };
        match scenario.requirement {
            ScenarioRequirementLevel::Required => {
                if result.status != ScenarioExecutionStatus::Passed {
                    blockers.push(format!(
                        "{} status {:?}",
                        scenario.scenario_id, result.status
                    ));
                }
            }
            ScenarioRequirementLevel::CapabilityDependent => {
                if result.status == ScenarioExecutionStatus::Failed {
                    blockers.push(format!(
                        "{} failed capability-dependent scenario",
                        scenario.scenario_id
                    ));
                }
            }
        }
    }
    if !artifacts.disqualifications.is_empty() {
        blockers.push(format!(
            "{} correctness disqualification(s)",
            artifacts.disqualifications.len()
        ));
    }

    let status = if blockers.is_empty() {
        CandidateEligibilityStatus::EligibleForEquivalentExecution
    } else {
        CandidateEligibilityStatus::DisqualifiedByDemonstratedFailure
    };
    CandidateEligibilityRecord {
        candidate_id: artifacts.candidate_id.clone(),
        candidate_version: artifacts.candidate_version.clone(),
        status,
        rationale: if blockers.is_empty() {
            "required scenarios executed with valid evidence".to_owned()
        } else {
            blockers.join("; ")
        },
    }
}

pub fn aggregate_is_valid(aggregate: &MeasurementAggregate, expected_count: u32) -> bool {
    if aggregate.operation == "compaction_where_supported" && aggregate.count == 0 {
        return true;
    }
    aggregate.count == expected_count
        && aggregate.failure_count == 0
        && peak_memory_is_satisfied(&aggregate.peak_memory_bytes)
        && storage_metric_is_satisfied(&aggregate.storage_size_before)
        && storage_metric_is_satisfied(&aggregate.storage_size_after)
}

fn evaluate_scenario_result(
    scenario: &ScenarioContractV3,
    result: &super::types::NormalizedScenarioResult,
    limitations: &mut Vec<String>,
    blockers: &mut Vec<String>,
) {
    match scenario.requirement {
        ScenarioRequirementLevel::Required => {
            if result.status != ScenarioExecutionStatus::Passed {
                blockers.push(format!(
                    "required scenario {} not passed ({:?})",
                    scenario.scenario_id, result.status
                ));
            }
        }
        ScenarioRequirementLevel::CapabilityDependent => {
            if result.status == ScenarioExecutionStatus::Unsupported {
                limitations.push(format!(
                    "capability-dependent scenario {} unsupported: {}",
                    scenario.scenario_id,
                    result.limitations.join(", ")
                ));
            } else if result.status != ScenarioExecutionStatus::Passed {
                blockers.push(format!(
                    "capability-dependent scenario {} invalid ({:?})",
                    scenario.scenario_id, result.status
                ));
            }
        }
    }
}

fn evaluate_measurement_aggregate(
    aggregate: &MeasurementAggregate,
    expected_count: u32,
    blockers: &mut Vec<String>,
) {
    if aggregate.operation == "compaction_where_supported" && aggregate.count == 0 {
        return;
    }
    if aggregate.count != expected_count {
        blockers.push(format!(
            "measurement {} {:?} count {} != {}",
            aggregate.operation, aggregate.fixture_scale, aggregate.count, expected_count
        ));
    }
    if aggregate.failure_count > 0 {
        blockers.push(format!(
            "measurement {} {:?} failure_count {}",
            aggregate.operation, aggregate.fixture_scale, aggregate.failure_count
        ));
    }
    if !peak_memory_is_satisfied(&aggregate.peak_memory_bytes) {
        blockers.push(format!(
            "measurement {} {:?} peak_memory unavailable",
            aggregate.operation, aggregate.fixture_scale
        ));
    }
}

fn peak_memory_is_satisfied(metric: &MetricAvailability<u64>) -> bool {
    metric.status == "available" && metric.value.is_some_and(|value| value > 0)
}

fn storage_metric_is_satisfied(metric: &MetricAvailability<u64>) -> bool {
    metric.status == "available" || metric.status == "unavailable"
}

fn collect_declared_limitations(
    append: &CandidateRunArtifacts,
    sqlite: &CandidateRunArtifacts,
) -> Vec<String> {
    let mut out = Vec::new();
    if append
        .measurements
        .iter()
        .any(|row| row.operation == "compaction_where_supported" && row.count == 0)
        || sqlite
            .measurements
            .iter()
            .any(|row| row.operation == "compaction_where_supported" && row.count == 0)
    {
        out.push("compaction_where_supported unsupported for measured candidates".to_owned());
    }
    if append
        .measurements
        .iter()
        .any(|row| matches!(row.bytes_read.status.as_str(), "unavailable"))
    {
        out.push("bytes_read unavailable".to_owned());
    }
    if append
        .measurements
        .iter()
        .any(|row| matches!(row.bytes_written.status.as_str(), "unavailable"))
    {
        out.push("bytes_written unavailable".to_owned());
    }
    if !contract_has_stress() {
        out.push("stress fixture not implemented".to_owned());
    }
    out.sort();
    out.dedup();
    out
}

fn contract_has_stress() -> bool {
    comparative_measurement_contract()
        .operations
        .iter()
        .any(|operation| {
            operation
                .fixture_scales
                .contains(&MeasurementFixtureScale::Stress)
        })
}

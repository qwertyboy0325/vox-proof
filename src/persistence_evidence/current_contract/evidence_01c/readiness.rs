use super::scenario_observation::APPEND_STALE_ASYMMETRY_LIMITATION;
use super::super::candidate_equivalence::CandidateEligibilityStatus;
use super::super::measurement::{comparative_measurement_contract, MeasurementFixtureScale};
use super::super::scenario_contract::{
    scenario_contract_v4, ReadOnlyOpenPolicy, ScenarioContractV3, ScenarioRequirementLevel,
};
use super::types::{
    open_state_label, recovery_class_label, CandidateEligibilityRecord, CandidateRunArtifacts,
    MeasurementAggregate, MetricAvailability, NormalizedScenarioResult, ScenarioExecutionStatus,
};

pub const EXPECTED_PLATFORMS: &[&str] = &["macos_native", "windows-github_actions"];

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReadinessAssessment {
    pub mechanism_comparison_readiness: String,
    pub mechanism_selection_readiness: String,
    pub single_platform_readiness: String,
    pub limitations: Vec<String>,
    pub comparison_blockers: Vec<String>,
    pub selection_blockers: Vec<String>,
    /// Comparison blockers retained for manifest backward compatibility.
    pub blockers: Vec<String>,
}

pub fn assess_readiness(
    append: &CandidateRunArtifacts,
    sqlite: &CandidateRunArtifacts,
) -> ReadinessAssessment {
    let mut limitations = Vec::new();
    let mut comparison_blockers = Vec::new();
    let contract = comparative_measurement_contract();

    for candidate in [&append.scenario_results, &sqlite.scenario_results] {
        for scenario in scenario_contract_v4() {
            let Some(result) = candidate
                .iter()
                .find(|row| row.scenario_id == scenario.scenario_id)
            else {
                if scenario.requirement == ScenarioRequirementLevel::Required {
                    comparison_blockers
                        .push(format!("missing scenario result {}", scenario.scenario_id));
                }
                continue;
            };
            evaluate_scenario_result(
                &scenario,
                result,
                &mut limitations,
                &mut comparison_blockers,
            );
        }
    }

    for artifacts in [append, sqlite] {
        for operation in &contract.operations {
            for scale in &operation.fixture_scales {
                let Some(aggregate) = artifacts.measurements.iter().find(|row| {
                    row.operation == operation.operation && row.fixture_scale == *scale
                }) else {
                    comparison_blockers.push(format!(
                        "missing measurement {} {:?} for {}",
                        operation.operation, scale, artifacts.candidate_id
                    ));
                    continue;
                };
                evaluate_measurement_aggregate(
                    aggregate,
                    operation.sample_count,
                    &mut comparison_blockers,
                );
            }
        }
    }

    limitations.extend(collect_declared_limitations(append, sqlite));

    let append_eligibility = compute_eligibility(append);
    let sqlite_eligibility = compute_eligibility(sqlite);
    let mut selection_only_blockers = Vec::new();
    if append_eligibility.status == CandidateEligibilityStatus::ImplementationNotYetEvaluated {
        selection_only_blockers.push(
            "eligibility: append stale-precondition asymmetry blocks mechanism_selection_readiness"
                .to_owned(),
        );
    }
    if append_eligibility.status == CandidateEligibilityStatus::DisqualifiedByDemonstratedFailure
        || sqlite_eligibility.status == CandidateEligibilityStatus::DisqualifiedByDemonstratedFailure
    {
        comparison_blockers
            .push("eligibility: candidate disqualified by demonstrated failure".to_owned());
    }

    let single_platform_comparison_ready = comparison_blockers.is_empty();
    let single_platform_readiness = if single_platform_comparison_ready {
        "ready_for_owner_decision".to_owned()
    } else {
        "not_ready".to_owned()
    };

    let cross_platform_complete = std::env::var("VOXPROOF_01C_CROSS_PLATFORM_COMPLETE")
        .is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true"));
    if single_platform_comparison_ready && !cross_platform_complete {
        comparison_blockers.push(
            "cross_platform: awaiting peer platform evidence aggregation".to_owned(),
        );
    }

    let mut selection_blockers = comparison_blockers.clone();
    selection_blockers.extend(selection_only_blockers);

    let comparison_ready = comparison_blockers.is_empty();
    let selection_ready = selection_blockers.is_empty();
    ReadinessAssessment {
        mechanism_comparison_readiness: if comparison_ready {
            "ready_for_owner_decision".to_owned()
        } else {
            "not_ready".to_owned()
        },
        mechanism_selection_readiness: if selection_ready {
            "ready_for_owner_decision".to_owned()
        } else {
            "not_ready".to_owned()
        },
        single_platform_readiness,
        limitations,
        comparison_blockers: comparison_blockers.clone(),
        selection_blockers,
        blockers: comparison_blockers,
    }
}

pub fn compute_eligibility(artifacts: &CandidateRunArtifacts) -> CandidateEligibilityRecord {
    let mut blockers = Vec::new();
    for scenario in scenario_contract_v4() {
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

    let append_stale_asymmetry = artifacts.scenario_results.iter().any(|result| {
        result
            .limitations
            .iter()
            .any(|limitation| limitation == APPEND_STALE_ASYMMETRY_LIMITATION)
    });
    if blockers.is_empty()
        && append_stale_asymmetry
        && artifacts.candidate_id == "current-contract-append-authoritative-candidate"
    {
        return CandidateEligibilityRecord {
            candidate_id: artifacts.candidate_id.clone(),
            candidate_version: artifacts.candidate_version.clone(),
            status: CandidateEligibilityStatus::ImplementationNotYetEvaluated,
            rationale: "required scenarios passed with documented append stale-precondition asymmetry; not equivalent to sqlite precondition-class coverage".to_owned(),
        };
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
    result: &NormalizedScenarioResult,
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
                return;
            }
        }
        ScenarioRequirementLevel::CapabilityDependent => {
            if result.status == ScenarioExecutionStatus::Unsupported {
                limitations.push(format!(
                    "capability-dependent scenario {} unsupported: {}",
                    scenario.scenario_id,
                    result.limitations.join(", ")
                ));
                return;
            } else if result.status != ScenarioExecutionStatus::Passed {
                blockers.push(format!(
                    "capability-dependent scenario {} invalid ({:?})",
                    scenario.scenario_id, result.status
                ));
                return;
            }
        }
    }

    if result.status == ScenarioExecutionStatus::Passed {
        validate_observation_matches_contract(scenario, result, blockers);
    }
}

fn validate_observation_matches_contract(
    scenario: &ScenarioContractV3,
    result: &NormalizedScenarioResult,
    blockers: &mut Vec<String>,
) {
    let expected_recovery = recovery_class_label(scenario.expected_recovery_class);
    if result.recovery_class != expected_recovery {
        blockers.push(format!(
            "scenario {} recovery_class {:?} != contract {}",
            scenario.scenario_id, result.recovery_class, expected_recovery
        ));
    }
    let expected_open = open_state_label(scenario.expected_open_state);
    if result.open_state != expected_open {
        blockers.push(format!(
            "scenario {} open_state {:?} != contract {}",
            scenario.scenario_id, result.open_state, expected_open
        ));
    }
    if is_fail_closed_authoritative_corruption(scenario) && result.oracle_compare {
        blockers.push(format!(
            "scenario {} must not claim oracle_compare on fail-closed refusal",
            scenario.scenario_id
        ));
    }
    if matches!(scenario.read_only_open, ReadOnlyOpenPolicy::Forbidden)
        && result.open_state != "unrecoverable"
        && result.open_state != "unsupported_version"
        && result.open_state != "open_refused"
    {
        blockers.push(format!(
            "scenario {} forbids trusted read exposure; observed open_state {}",
            scenario.scenario_id, result.open_state
        ));
    }
    if scenario.fault_layer == super::super::scenario_contract::FaultLayer::LogicalReturnedError
        && scenario.category == "corruption"
        && result.failure_code.is_none()
    {
        blockers.push(format!(
            "scenario {} missing observed corruption/refusal classification",
            scenario.scenario_id
        ));
    }
}

fn is_fail_closed_authoritative_corruption(scenario: &ScenarioContractV3) -> bool {
    scenario.scenario_version == 2
        && matches!(
            scenario.scenario_id.as_str(),
            "canonical-reference-corruption" | "source-locator-corruption"
        )
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
    if !storage_metric_is_satisfied(&aggregate.storage_size_before) {
        blockers.push(format!(
            "measurement {} {:?} storage_size_before unavailable or zero sentinel",
            aggregate.operation, aggregate.fixture_scale
        ));
    }
    if !storage_metric_is_satisfied(&aggregate.storage_size_after) {
        blockers.push(format!(
            "measurement {} {:?} storage_size_after unavailable or zero sentinel",
            aggregate.operation, aggregate.fixture_scale
        ));
    }
}

fn peak_memory_is_satisfied(metric: &MetricAvailability<u64>) -> bool {
    metric.status == "available" && metric.value.is_some_and(|value| value > 0)
}

fn storage_metric_is_satisfied(metric: &MetricAvailability<u64>) -> bool {
    match metric.status.as_str() {
        "unavailable" => true,
        "available" => metric.value.is_some_and(|value| value > 0),
        _ => false,
    }
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

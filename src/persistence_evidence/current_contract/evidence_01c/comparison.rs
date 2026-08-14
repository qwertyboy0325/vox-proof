use super::super::measurement::comparative_measurement_contract;
use super::readiness::aggregate_is_valid;
use super::types::{CandidateRunArtifacts, ComparativeEvidencePackage, PlatformComparisonSection};

pub fn build_comparison_report(
    platform: &str,
    append: &CandidateRunArtifacts,
    sqlite: &CandidateRunArtifacts,
) -> PlatformComparisonSection {
    let contract = comparative_measurement_contract();
    let mut tradeoffs = Vec::new();
    for append_measurement in &append.measurements {
        let Some(sqlite_measurement) = sqlite.measurements.iter().find(|measurement| {
            measurement.operation == append_measurement.operation
                && measurement.fixture_scale == append_measurement.fixture_scale
        }) else {
            continue;
        };
        let expected_count = contract
            .operations
            .iter()
            .find(|operation| operation.operation == append_measurement.operation)
            .map(|operation| operation.sample_count)
            .unwrap_or(0);
        if !aggregate_is_valid(append_measurement, expected_count)
            || !aggregate_is_valid(sqlite_measurement, expected_count)
        {
            continue;
        }
        if append_measurement.median_ms < sqlite_measurement.median_ms {
            tradeoffs.push(format!(
                "{} {}: append lower median latency ({} ms vs {} ms)",
                append_measurement.operation,
                super::measurements::scale_label(append_measurement.fixture_scale),
                append_measurement.median_ms,
                sqlite_measurement.median_ms
            ));
        } else if sqlite_measurement.median_ms < append_measurement.median_ms {
            tradeoffs.push(format!(
                "{} {}: sqlite lower median latency ({} ms vs {} ms)",
                append_measurement.operation,
                super::measurements::scale_label(sqlite_measurement.fixture_scale),
                sqlite_measurement.median_ms,
                append_measurement.median_ms
            ));
        }
        if let (Some(append_peak), Some(sqlite_peak)) = (
            append_measurement.peak_memory_bytes.value,
            sqlite_measurement.peak_memory_bytes.value,
        ) {
            if append_peak < sqlite_peak {
                tradeoffs.push(format!(
                    "{} {}: append lower subprocess peak RSS ({} vs {} bytes)",
                    append_measurement.operation,
                    super::measurements::scale_label(append_measurement.fixture_scale),
                    append_peak,
                    sqlite_peak
                ));
            } else if sqlite_peak < append_peak {
                tradeoffs.push(format!(
                    "{} {}: sqlite lower subprocess peak RSS ({} vs {} bytes)",
                    sqlite_measurement.operation,
                    super::measurements::scale_label(sqlite_measurement.fixture_scale),
                    sqlite_peak,
                    append_peak
                ));
            }
        }
    }

    let mut recovery_observations = Vec::new();
    for (label, artifacts) in [("append", append), ("sqlite", sqlite)] {
        if let Some(result) = artifacts
            .scenario_results
            .iter()
            .find(|row| row.scenario_id == "writer-crash-and-takeover")
        {
            recovery_observations.push(format!(
                "writer-crash-and-takeover {label}: elapsed_ms={} recovery_class={} open_state={}",
                result.elapsed_ms, result.recovery_class, result.open_state
            ));
        }
    }
    if let (Some(append_result), Some(sqlite_result)) = (
        append
            .scenario_results
            .iter()
            .find(|row| row.scenario_id == "writer-crash-and-takeover"),
        sqlite
            .scenario_results
            .iter()
            .find(|row| row.scenario_id == "writer-crash-and-takeover"),
    ) {
        recovery_observations.push(format!(
            "takeover latency asymmetry on {platform}: append {} ms vs sqlite {} ms (sqlite harness uses {} ms test lease + {} ms wait)",
            append_result.elapsed_ms,
            sqlite_result.elapsed_ms,
            super::candidate::SQLITE_WRITER_LEASE_DURATION_MS,
            super::candidate::SQLITE_LEASE_EXPIRY_WAIT_MS
        ));
    }

    let measurement_caveats = vec![
        "peak_memory_bytes is subprocess peak RSS after fixture setup, not operation-isolated attribution"
            .to_owned(),
        "storage_size_before is unavailable for isolated per-sample measurement roots"
            .to_owned(),
    ];

    PlatformComparisonSection {
        platform: platform.to_owned(),
        tradeoffs,
        recovery_observations,
        measurement_caveats,
        append_disqualified: !append.disqualifications.is_empty(),
        sqlite_disqualified: !sqlite.disqualifications.is_empty(),
    }
}

pub fn comparison_markdown(section: &PlatformComparisonSection) -> String {
    let mut lines = vec![format!("# 01C comparison ({})", section.platform)];
    if section.tradeoffs.is_empty() {
        lines.push("No material within-platform latency/memory tradeoffs observed.".to_owned());
    } else {
        for tradeoff in &section.tradeoffs {
            lines.push(format!("- {tradeoff}"));
        }
    }
    if !section.recovery_observations.is_empty() {
        lines.push(String::new());
        lines.push("## Recovery observations".to_owned());
        for observation in &section.recovery_observations {
            lines.push(format!("- {observation}"));
        }
    }
    if !section.measurement_caveats.is_empty() {
        lines.push(String::new());
        lines.push("## Measurement caveats".to_owned());
        for caveat in &section.measurement_caveats {
            lines.push(format!("- {caveat}"));
        }
    }
    lines.join("\n")
}

#[allow(clippy::too_many_arguments)]
pub fn package_from_runs(
    work_package_id: &str,
    harness_semantic_sha: &str,
    environment: super::types::EnvironmentRecord,
    methodology: super::methodology::MethodologyRecord,
    medium_fixture: super::types::MediumFixtureRecord,
    eligibility: Vec<super::types::CandidateEligibilityRecord>,
    append: CandidateRunArtifacts,
    sqlite: CandidateRunArtifacts,
    comparison: PlatformComparisonSection,
    readiness: &super::readiness::ReadinessAssessment,
    invalid_evidence_chain: Vec<super::types::InvalidPredecessorEvidence>,
) -> ComparativeEvidencePackage {
    ComparativeEvidencePackage {
        work_package_id: work_package_id.to_owned(),
        harness_semantic_sha: harness_semantic_sha.to_owned(),
        environment,
        methodology,
        medium_fixture,
        eligibility,
        append,
        sqlite,
        comparison,
        limitations: readiness.limitations.clone(),
        mechanism_comparison_readiness: readiness.mechanism_comparison_readiness.clone(),
        mechanism_selection_readiness: readiness.mechanism_selection_readiness.clone(),
        predecessor_invalid_evidence: invalid_evidence_chain.last().cloned(),
        invalid_evidence_chain,
    }
}

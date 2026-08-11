use super::types::{CandidateRunArtifacts, ComparativeEvidencePackage, PlatformComparisonSection};

pub fn build_comparison_report(
    platform: &str,
    append: &CandidateRunArtifacts,
    sqlite: &CandidateRunArtifacts,
) -> PlatformComparisonSection {
    let mut tradeoffs = Vec::new();
    for append_measurement in &append.measurements {
        if let Some(sqlite_measurement) = sqlite.measurements.iter().find(|measurement| {
            measurement.operation == append_measurement.operation
                && measurement.fixture_scale == append_measurement.fixture_scale
        }) {
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
                    sqlite_measurement.operation,
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
                        "{} {}: append lower peak memory ({} vs {} bytes)",
                        append_measurement.operation,
                        super::measurements::scale_label(append_measurement.fixture_scale),
                        append_peak,
                        sqlite_peak
                    ));
                } else if sqlite_peak < append_peak {
                    tradeoffs.push(format!(
                        "{} {}: sqlite lower peak memory ({} vs {} bytes)",
                        sqlite_measurement.operation,
                        super::measurements::scale_label(sqlite_measurement.fixture_scale),
                        sqlite_peak,
                        append_peak
                    ));
                }
            }
        }
    }
    PlatformComparisonSection {
        platform: platform.to_owned(),
        tradeoffs,
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
    limitations: Vec<String>,
) -> ComparativeEvidencePackage {
    let ready = append.disqualifications.is_empty()
        && sqlite.disqualifications.is_empty()
        && !append.measurements.is_empty()
        && !sqlite.measurements.is_empty();
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
        limitations,
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
    }
}

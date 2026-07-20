//! Cross-platform semantic comparison helpers (Package 2D).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::model::ScenarioResult;
use super::platform::{
    CROSS_PLATFORM_SCENARIO_IDS, PlatformEquivalenceResult, PlatformScenarioRow,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformMatrixDocument {
    pub matrix_version: String,
    pub macos_run_id: Option<String>,
    pub windows_run_id: Option<String>,
    pub scenarios: Vec<PlatformScenarioRow>,
    pub hardware_power_loss_status: String,
    pub filesystem_durability_status: String,
    pub cross_platform_summary: BTreeMap<String, u32>,
}

pub fn build_platform_matrix(
    macos_run_id: Option<&str>,
    windows_run_id: Option<&str>,
    macos_results: &[ScenarioResult],
    windows_results: &[ScenarioResult],
) -> PlatformMatrixDocument {
    let macos_by_id: BTreeMap<_, _> = macos_results
        .iter()
        .map(|r| (r.scenario_identity.scenario_id.clone(), r))
        .collect();
    let windows_by_id: BTreeMap<_, _> = windows_results
        .iter()
        .map(|r| (r.scenario_identity.scenario_id.clone(), r))
        .collect();

    let mut scenarios = Vec::new();
    let mut equivalent_count = 0u32;
    let mut different_count = 0u32;
    let mut missing_count = 0u32;

    for scenario_id in CROSS_PLATFORM_SCENARIO_IDS {
        let macos = macos_by_id.get(*scenario_id);
        let windows = windows_by_id.get(*scenario_id);
        let row = compare_scenario(scenario_id, macos.copied(), windows.copied());
        match row.equivalence_result {
            PlatformEquivalenceResult::Equivalent => equivalent_count += 1,
            PlatformEquivalenceResult::Different => different_count += 1,
            PlatformEquivalenceResult::MissingPlatform => missing_count += 1,
            PlatformEquivalenceResult::NotCompared => {}
        }
        scenarios.push(row);
    }

    let mut cross_platform_summary = BTreeMap::new();
    cross_platform_summary.insert("equivalent".to_string(), equivalent_count);
    cross_platform_summary.insert("different".to_string(), different_count);
    cross_platform_summary.insert("missing_platform".to_string(), missing_count);

    PlatformMatrixDocument {
        matrix_version: "1".to_string(),
        macos_run_id: macos_run_id.map(str::to_string),
        windows_run_id: windows_run_id.map(str::to_string),
        scenarios,
        hardware_power_loss_status: "not_demonstrated".to_string(),
        filesystem_durability_status: "not_demonstrated".to_string(),
        cross_platform_summary,
    }
}

pub fn compare_scenario(
    scenario_id: &str,
    macos: Option<&ScenarioResult>,
    windows: Option<&ScenarioResult>,
) -> PlatformScenarioRow {
    let macos_status = macos.map(|r| format!("{:?}", r.status));
    let windows_status = windows.map(|r| format!("{:?}", r.status));
    let macos_fp = macos.and_then(|r| {
        r.oracle_result
            .as_ref()
            .map(|o| o.actual_fingerprint.clone())
    });
    let windows_fp = windows.and_then(|r| {
        r.oracle_result
            .as_ref()
            .map(|o| o.actual_fingerprint.clone())
    });

    let (equivalence_result, cross_platform_credited, differences) = match (macos, windows) {
        (None, None) => (
            PlatformEquivalenceResult::NotCompared,
            false,
            vec!["no platform results".to_string()],
        ),
        (Some(_), None) | (None, Some(_)) => (
            PlatformEquivalenceResult::MissingPlatform,
            false,
            vec!["awaiting paired platform execution".to_string()],
        ),
        (Some(m), Some(w)) => {
            let mut diffs = Vec::new();
            if m.status != w.status {
                diffs.push(format!(
                    "status macos={:?} windows={:?}",
                    m.status, w.status
                ));
            }
            let fp_match = macos_fp == windows_fp;
            if !fp_match {
                diffs.push("oracle fingerprint differs (may be acceptable if semantic oracle still passes)".to_string());
            }
            let both_pass = m.status == super::model::ScenarioStatus::Passed
                && w.status == super::model::ScenarioStatus::Passed;
            let error_match = error_codes_equivalent(scenario_id, m, w);
            if !error_match {
                diffs.push(format!(
                    "observed error code macos={:?} windows={:?}",
                    macos.and_then(|r| r.observed_error_code.clone()),
                    windows.and_then(|r| r.observed_error_code.clone()),
                ));
            }
            let equivalent = both_pass
                && error_match
                && (fp_match || scenario_allows_fingerprint_drift(scenario_id));
            if equivalent && !fp_match {
                diffs.push(
                    "semantic equivalence accepted without byte-identical fingerprint".to_string(),
                );
            }
            (
                if equivalent {
                    PlatformEquivalenceResult::Equivalent
                } else {
                    PlatformEquivalenceResult::Different
                },
                equivalent,
                diffs,
            )
        }
    };

    PlatformScenarioRow {
        scenario_id: scenario_id.to_string(),
        macos_status,
        windows_status,
        macos_oracle_fingerprint: macos_fp,
        windows_oracle_fingerprint: windows_fp,
        macos_error_code: macos.and_then(|r| r.observed_error_code.clone()),
        windows_error_code: windows.and_then(|r| r.observed_error_code.clone()),
        equivalence_result,
        cross_platform_credited,
        differences,
    }
}

fn scenario_allows_fingerprint_drift(scenario_id: &str) -> bool {
    matches!(
        scenario_id,
        "concurrent-writer-attempt" | "interrupted-authoritative-transition"
    )
}

fn error_codes_equivalent(
    scenario_id: &str,
    macos: &ScenarioResult,
    windows: &ScenarioResult,
) -> bool {
    match (
        macos.observed_error_code.as_deref(),
        windows.observed_error_code.as_deref(),
    ) {
        (None, None) => !matches!(
            scenario_id,
            "unknown-newer-format" | "canonical-reference-corruption"
        ),
        (Some(a), Some(b)) if scenario_id == "canonical-reference-corruption" => a == b,
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

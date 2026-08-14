#![cfg(feature = "persistence-spike")]

use std::sync::Mutex;

use vox_proof::persistence_evidence::current_contract::evidence_01c::readiness::assess_readiness;
use vox_proof::persistence_evidence::current_contract::evidence_01c::scenario_observation::APPEND_STALE_ASYMMETRY_LIMITATION;
use vox_proof::persistence_evidence::current_contract::evidence_01c::types::{
    open_state_label, recovery_class_label, CandidateRunArtifacts, MeasurementAggregate,
    MetricAvailability, NormalizedScenarioResult, ScenarioExecutionStatus,
};
use vox_proof::persistence_evidence::current_contract::scenario_contract::ScenarioContractV3;
use vox_proof::persistence_evidence::{
    comparative_measurement_contract, scenario_contract_v4,
};

static READINESS_ENV_LOCK: Mutex<()> = Mutex::new(());

const APPEND_ID: &str = "current-contract-append-authoritative-candidate";
const APPEND_VERSION: &str = "01B-2";
const SQLITE_ID: &str = "current-contract-sqlite-authoritative-candidate";
const SQLITE_VERSION: &str = "01C-SQLITE-2";

fn valid_measurements(candidate_id: &str, candidate_version: &str) -> Vec<MeasurementAggregate> {
    comparative_measurement_contract()
        .operations
        .iter()
        .flat_map(|operation| {
            operation.fixture_scales.iter().map(move |scale| MeasurementAggregate {
                operation: operation.operation.clone(),
                fixture_scale: *scale,
                candidate_id: candidate_id.to_owned(),
                candidate_version: candidate_version.to_owned(),
                platform: "macos_native".to_owned(),
                count: operation.sample_count,
                minimum_ms: 1,
                median_ms: 1,
                p95_ms: 1,
                maximum_ms: 1,
                failure_count: 0,
                peak_memory_bytes: MetricAvailability::available(1024),
                bytes_read: MetricAvailability::unavailable("not observed"),
                bytes_written: MetricAvailability::unavailable("not observed"),
                storage_size_before: MetricAvailability::available(1),
                storage_size_after: MetricAvailability::available(2),
            })
        })
        .collect()
}

fn observation_fields_for_scenario(
    scenario: &ScenarioContractV3,
) -> (String, String, Option<String>, bool) {
    match scenario.scenario_id.as_str() {
        "canonical-reference-corruption" | "source-locator-corruption" => (
            "manual_review_required".to_owned(),
            "unrecoverable".to_owned(),
            Some("canonical-corruption".to_owned()),
            false,
        ),
        "review-ledger-order-corruption" | "reuse-governance-order-corruption" => (
            "unrecoverable".to_owned(),
            "unrecoverable".to_owned(),
            Some("canonical-corruption".to_owned()),
            false,
        ),
        "malformed-format-version" => (
            "unrecoverable".to_owned(),
            "unrecoverable".to_owned(),
            Some("unsupported-format".to_owned()),
            false,
        ),
        "unknown-newer-format" => (
            "unsupported_version".to_owned(),
            "unsupported_version".to_owned(),
            Some("unsupported-newer-format".to_owned()),
            false,
        ),
        "writer-crash-and-takeover"
        | "interrupted-authoritative-review-transition"
        | "interrupted-authoritative-reuse-transition"
        | "derived-state-corruption-and-rebuild" => (
            "safe_automatic_recovery".to_owned(),
            "normal".to_owned(),
            None,
            true,
        ),
        "read-only-open-during-writer" => (
            recovery_class_label(scenario.expected_recovery_class),
            open_state_label(scenario.expected_open_state),
            None,
            true,
        ),
        _ => (
            recovery_class_label(scenario.expected_recovery_class),
            open_state_label(scenario.expected_open_state),
            None,
            false,
        ),
    }
}

fn passed_scenario_results(
    candidate_id: &str,
    candidate_version: &str,
    append_stale_asymmetry: bool,
) -> Vec<NormalizedScenarioResult> {
    scenario_contract_v4()
        .iter()
        .map(|scenario| {
            let mut limitations = Vec::new();
            if append_stale_asymmetry
                && candidate_id == APPEND_ID
                && scenario.scenario_id.starts_with("stale-")
            {
                limitations.push(APPEND_STALE_ASYMMETRY_LIMITATION.to_owned());
            }
            let (recovery_class, open_state, failure_code, oracle_compare) =
                observation_fields_for_scenario(scenario);
            let status = if scenario.capability_requirement.is_some()
                && matches!(
                    scenario.scenario_id.as_str(),
                    "interrupted-compaction" | "interrupted-cleanup"
                )
            {
                ScenarioExecutionStatus::Unsupported
            } else {
                ScenarioExecutionStatus::Passed
            };
            if status == ScenarioExecutionStatus::Unsupported {
                limitations.push(format!(
                    "capability {} not declared",
                    scenario.capability_requirement.as_deref().unwrap_or("unknown")
                ));
            }
            NormalizedScenarioResult {
                scenario_id: scenario.scenario_id.clone(),
                scenario_version: scenario.scenario_version,
                candidate_id: candidate_id.to_owned(),
                candidate_version: candidate_version.to_owned(),
                platform: "macos_native".to_owned(),
                fixture_scale: "small".to_owned(),
                status,
                oracle_compare,
                recovery_class,
                open_state,
                failure_code,
                elapsed_ms: 1,
                correctness_disqualification: None,
                limitations,
                fcr03_stale_rejection: None,
                fcr03_unrelated_success: None,
            }
        })
        .collect()
}

fn valid_candidate_pair(append_stale_asymmetry: bool) -> (CandidateRunArtifacts, CandidateRunArtifacts) {
    (
        CandidateRunArtifacts {
            candidate_id: APPEND_ID.to_owned(),
            candidate_version: APPEND_VERSION.to_owned(),
            scenario_results: passed_scenario_results(APPEND_ID, APPEND_VERSION, append_stale_asymmetry),
            measurements: valid_measurements(APPEND_ID, APPEND_VERSION),
            disqualifications: Vec::new(),
        },
        CandidateRunArtifacts {
            candidate_id: SQLITE_ID.to_owned(),
            candidate_version: SQLITE_VERSION.to_owned(),
            scenario_results: passed_scenario_results(SQLITE_ID, SQLITE_VERSION, false),
            measurements: valid_measurements(SQLITE_ID, SQLITE_VERSION),
            disqualifications: Vec::new(),
        },
    )
}

#[test]
fn append_stale_asymmetry_blocks_selection_only_when_comparison_valid() {
    let _guard = READINESS_ENV_LOCK.lock().unwrap();
    let (append, sqlite) = valid_candidate_pair(true);
    unsafe {
        std::env::set_var("VOXPROOF_01C_CROSS_PLATFORM_COMPLETE", "1");
    }
    let readiness = assess_readiness(&append, &sqlite);
    unsafe {
        std::env::remove_var("VOXPROOF_01C_CROSS_PLATFORM_COMPLETE");
    }

    assert_eq!(
        readiness.mechanism_comparison_readiness,
        "ready_for_owner_decision"
    );
    assert_eq!(readiness.mechanism_selection_readiness, "not_ready");
    assert!(readiness.comparison_blockers.is_empty());
    assert!(readiness
        .selection_blockers
        .iter()
        .any(|blocker| blocker.contains("append stale-precondition asymmetry")));
}

#[test]
fn required_scenario_failure_blocks_comparison_and_selection() {
    let _guard = READINESS_ENV_LOCK.lock().unwrap();
    let (mut append, sqlite) = valid_candidate_pair(false);
    append.scenario_results[0].status = ScenarioExecutionStatus::Failed;
    let readiness = assess_readiness(&append, &sqlite);

    assert_eq!(readiness.mechanism_comparison_readiness, "not_ready");
    assert_eq!(readiness.mechanism_selection_readiness, "not_ready");
    assert!(!readiness.comparison_blockers.is_empty());
    assert!(!readiness.selection_blockers.is_empty());
}

#[test]
fn cross_platform_incomplete_blocks_comparison_and_selection() {
    let _guard = READINESS_ENV_LOCK.lock().unwrap();
    let (append, sqlite) = valid_candidate_pair(false);
    let prior = std::env::var("VOXPROOF_01C_CROSS_PLATFORM_COMPLETE").ok();
    unsafe {
        std::env::remove_var("VOXPROOF_01C_CROSS_PLATFORM_COMPLETE");
    }
    let readiness = assess_readiness(&append, &sqlite);
    match prior {
        Some(value) => unsafe {
            std::env::set_var("VOXPROOF_01C_CROSS_PLATFORM_COMPLETE", value);
        },
        None => unsafe {
            std::env::remove_var("VOXPROOF_01C_CROSS_PLATFORM_COMPLETE");
        },
    }

    assert_eq!(readiness.mechanism_comparison_readiness, "not_ready");
    assert_eq!(readiness.mechanism_selection_readiness, "not_ready");
    assert!(readiness
        .comparison_blockers
        .iter()
        .any(|blocker| blocker.starts_with("cross_platform:")));
}

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::super::fixture::{
    build_medium_fixture_state, medium_fixture_dimensions,
    medium_fixture_serialized_semantic_size_bytes,
};
use super::super::model::CURRENT_CONTRACT_FIXTURE_ID;
use super::super::oracle::CurrentContractOracle;
use super::candidate::CurrentContractCandidate;
use super::comparison::{build_comparison_report, comparison_markdown, package_from_runs};
use super::environment::{capture_environment, timestamp_iso};
use super::measurements::run_comparative_measurements;
use super::methodology::{methodology_record, EVIDENCE_01C_HARNESS_VERSION};
use super::readiness::{assess_readiness, compute_eligibility};
use super::scenarios::run_required_scenarios;
use super::types::{
    CandidateRunArtifacts, ComparativeEvidencePackage, InvalidPredecessorEvidence,
    MediumFixtureRecord,
};

pub const INVALID_PREDECESSOR_HARNESS_SHA: &str = "970b3c38bfbf07fd9750df93b285527b73beb43c";
pub const INVALID_PREDECESSOR_EVIDENCE_SHA: &str = "5b446b430f358d3da821461775cc50efe0943033";

pub struct RunOutput {
    pub package: ComparativeEvidencePackage,
    pub output_root: PathBuf,
}

pub fn run_01c_evidence(output_root: impl Into<PathBuf>) -> RunOutput {
    let output_root = output_root.into();
    let _ = fs::remove_dir_all(&output_root);
    fs::create_dir_all(&output_root).expect("output root");
    let repository_commit = git_head();
    let mut environment = capture_environment(&repository_commit, EVIDENCE_01C_HARNESS_VERSION);
    let methodology = methodology_record();
    fs::write(
        output_root.join("methodology.json"),
        serde_json::to_string_pretty(&methodology).expect("methodology json"),
    )
    .expect("write methodology");

    let medium_state = build_medium_fixture_state();
    let medium_oracle = CurrentContractOracle::validate(&medium_state);
    assert!(
        medium_oracle.passed,
        "medium fixture must pass oracle v3 before execution"
    );
    let dims = medium_fixture_dimensions();
    let medium_fixture = MediumFixtureRecord {
        fixture_id: CURRENT_CONTRACT_FIXTURE_ID.to_owned(),
        fixture_version: super::super::model::CURRENT_CONTRACT_FIXTURE_VERSION.to_owned(),
        scale: "medium".to_owned(),
        oracle_validated: true,
        dimensions: BTreeMap::from([
            (
                "source_revision_count".to_owned(),
                dims.source_revision_count as u64,
            ),
            (
                "analysis_snapshot_count".to_owned(),
                dims.analysis_snapshot_count as u64,
            ),
            (
                "detector_count_per_snapshot".to_owned(),
                dims.detector_count_per_snapshot as u64,
            ),
            (
                "review_case_count".to_owned(),
                dims.review_case_count as u64,
            ),
            (
                "review_ledger_event_count".to_owned(),
                dims.review_ledger_event_count as u64,
            ),
            (
                "reuse_governance_event_count".to_owned(),
                dims.reuse_governance_event_count as u64,
            ),
            (
                "serialized_semantic_size_bytes".to_owned(),
                medium_fixture_serialized_semantic_size_bytes(&medium_state),
            ),
        ]),
    };
    fs::create_dir_all(output_root.join("fixtures")).expect("fixtures dir");
    fs::write(
        output_root.join("fixtures/medium.json"),
        serde_json::to_string_pretty(&medium_fixture).expect("medium json"),
    )
    .expect("write medium fixture");

    let append_root = output_root.join("work/append");
    let sqlite_root = output_root.join("work/sqlite");
    fs::create_dir_all(&append_root).expect("append work");
    fs::create_dir_all(&sqlite_root).expect("sqlite work");
    let append = CurrentContractCandidate::append(&append_root).expect("append candidate");
    let sqlite = CurrentContractCandidate::sqlite(&sqlite_root).expect("sqlite candidate");
    let platform = environment.platform_label.clone();

    let (append_scenarios, append_disqualifications) =
        run_required_scenarios(&append, &platform, &append_root);
    let (sqlite_scenarios, sqlite_disqualifications) =
        run_required_scenarios(&sqlite, &platform, &sqlite_root);

    let (append_measurements, sqlite_measurements) = run_comparative_measurements(
        &append,
        &sqlite,
        &platform,
        &output_root.join("work/measurements"),
    );

    let append_artifacts = CandidateRunArtifacts {
        candidate_id: append.candidate_id().to_owned(),
        candidate_version: append.candidate_version().to_owned(),
        scenario_results: append_scenarios,
        measurements: append_measurements,
        disqualifications: append_disqualifications,
    };
    let sqlite_artifacts = CandidateRunArtifacts {
        candidate_id: sqlite.candidate_id().to_owned(),
        candidate_version: sqlite.candidate_version().to_owned(),
        scenario_results: sqlite_scenarios,
        measurements: sqlite_measurements,
        disqualifications: sqlite_disqualifications,
    };

    let comparison = build_comparison_report(&platform, &append_artifacts, &sqlite_artifacts);
    let eligibility = vec![
        compute_eligibility(&append_artifacts),
        compute_eligibility(&sqlite_artifacts),
    ];
    let readiness = assess_readiness(&append_artifacts, &sqlite_artifacts);
    environment.end_timestamp = Some(timestamp_iso());
    let predecessor_invalid_evidence = InvalidPredecessorEvidence {
        harness_sha: INVALID_PREDECESSOR_HARNESS_SHA.to_owned(),
        evidence_record_sha: INVALID_PREDECESSOR_EVIDENCE_SHA.to_owned(),
        verdict: "BLOCKED_01C_EVIDENCE".to_owned(),
    };
    let package = package_from_runs(
        "VP-GATE4-01C-EVIDENCE-HARNESS-CORRECTION-01",
        &repository_commit,
        environment.clone(),
        methodology,
        medium_fixture,
        eligibility,
        append_artifacts,
        sqlite_artifacts,
        comparison.clone(),
        &readiness,
        predecessor_invalid_evidence,
    );

    write_package(&output_root, &package, &comparison, &readiness);
    let _ = fs::remove_dir_all(output_root.join("work"));
    RunOutput {
        package,
        output_root,
    }
}

fn write_package(
    output_root: &Path,
    package: &ComparativeEvidencePackage,
    comparison: &super::types::PlatformComparisonSection,
    readiness: &super::readiness::ReadinessAssessment,
) {
    fs::create_dir_all(output_root.join("macos-native")).ok();
    fs::create_dir_all(output_root.join("windows-github-actions")).ok();
    let platform_dir = if package.environment.platform_label.contains("windows") {
        output_root.join("windows-github-actions")
    } else {
        output_root.join("macos-native")
    };
    fs::create_dir_all(platform_dir.join("append")).expect("append dir");
    fs::create_dir_all(platform_dir.join("sqlite")).expect("sqlite dir");
    fs::create_dir_all(output_root.join("fixtures")).expect("fixtures dir");
    fs::write(
        platform_dir.join("environment.json"),
        serde_json::to_string_pretty(&package.environment).expect("environment"),
    )
    .expect("write environment");
    fs::write(
        platform_dir.join("append/scenario-results.json"),
        serde_json::to_string_pretty(&package.append.scenario_results).expect("append scenarios"),
    )
    .expect("write append scenarios");
    fs::write(
        platform_dir.join("sqlite/scenario-results.json"),
        serde_json::to_string_pretty(&package.sqlite.scenario_results).expect("sqlite scenarios"),
    )
    .expect("write sqlite scenarios");
    fs::write(
        platform_dir.join("append/measurements.json"),
        serde_json::to_string_pretty(&package.append.measurements).expect("append measurements"),
    )
    .expect("write append measurements");
    fs::write(
        platform_dir.join("sqlite/measurements.json"),
        serde_json::to_string_pretty(&package.sqlite.measurements).expect("sqlite measurements"),
    )
    .expect("write sqlite measurements");
    fs::write(
        output_root.join("comparison.json"),
        serde_json::to_string_pretty(comparison).expect("comparison"),
    )
    .expect("write comparison");
    fs::write(
        output_root.join("comparison.md"),
        comparison_markdown(comparison),
    )
    .expect("write comparison md");
    fs::write(
        output_root.join("eligibility.json"),
        serde_json::to_string_pretty(&package.eligibility).expect("eligibility"),
    )
    .expect("write eligibility");
    fs::write(
        output_root.join("readiness.json"),
        serde_json::to_string_pretty(readiness).expect("readiness"),
    )
    .expect("write readiness");
    fs::write(
        output_root.join("manifest.json"),
        serde_json::to_string_pretty(package).expect("manifest"),
    )
    .expect("write manifest");
}

fn git_head() -> String {
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_owned())
}

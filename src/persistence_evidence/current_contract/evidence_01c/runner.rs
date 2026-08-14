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
    MediumFixtureRecord, SelectionValidityRecord,
};

pub const INVALID_PREDECESSOR_HARNESS_SHA: &str = "970b3c38bfbf07fd9750df93b285527b73beb43c";
pub const INVALID_PREDECESSOR_EVIDENCE_SHA: &str = "5b446b430f358d3da821461775cc50efe0943033";
pub const CORRECTION_01_HARNESS_SHA: &str = "b2569a022856f3cbfecdbdb950bc32584741a9a3";
pub const CORRECTION_02_HARNESS_SHA: &str = "629bb916a33bee839e24db62d7aea28486bf5790";
pub const CORRECTION_02_EVIDENCE_SHA: &str = "e487a0bbaa4e75ac088e47bb48715b453afe82d9";

pub const CORRECTION_03_HARNESS_SHA: &str = "6adb73198813bb8d9d9b7924b8759294a198f8ff";

pub const WORK_PACKAGE_ID: &str = "VP-GATE4-SCENARIO-CONTRACT-V4-RECONCILIATION-01";

pub struct RunOutput {
    pub package: ComparativeEvidencePackage,
    pub output_root: PathBuf,
}

pub fn invalid_evidence_chain() -> Vec<InvalidPredecessorEvidence> {
    vec![
        InvalidPredecessorEvidence {
            harness_sha: INVALID_PREDECESSOR_HARNESS_SHA.to_owned(),
            evidence_record_sha: INVALID_PREDECESSOR_EVIDENCE_SHA.to_owned(),
            verdict: "BLOCKED_01C_EVIDENCE".to_owned(),
        },
        InvalidPredecessorEvidence {
            harness_sha: CORRECTION_01_HARNESS_SHA.to_owned(),
            evidence_record_sha: "see_gate4-01c-b2569a0_negative_evidence".to_owned(),
            verdict: "BLOCKED_01C_EVIDENCE".to_owned(),
        },
        InvalidPredecessorEvidence {
            harness_sha: correction_02_harness_sha(),
            evidence_record_sha: correction_02_evidence_sha(),
            verdict: "invalid_for_gate_preserve".to_owned(),
        },
        InvalidPredecessorEvidence {
            harness_sha: CORRECTION_03_HARNESS_SHA.to_owned(),
            evidence_record_sha: "see_gate4-01c-6adb731_with_capability_audit".to_owned(),
            verdict: "CANDIDATE_CONTRACT_CAPABILITY_GAP".to_owned(),
        },
    ]
}

fn correction_02_harness_sha() -> String {
    std::env::var("VOXPROOF_CORRECTION_02_HARNESS_SHA")
        .unwrap_or_else(|_| CORRECTION_02_HARNESS_SHA.to_owned())
}

fn correction_02_evidence_sha() -> String {
    std::env::var("VOXPROOF_CORRECTION_02_EVIDENCE_SHA")
        .unwrap_or_else(|_| CORRECTION_02_EVIDENCE_SHA.to_owned())
}

pub fn run_01c_evidence(output_root: impl Into<PathBuf>) -> RunOutput {
    let output_root = output_root.into();
    let _ = fs::remove_dir_all(&output_root);
    fs::create_dir_all(&output_root).expect("output root");
    let repository_commit = git_head();
    let mut environment = capture_environment(&repository_commit, EVIDENCE_01C_HARNESS_VERSION);
    environment.configuration.insert(
        "sqlite_writer_lease_duration_ms".to_owned(),
        super::candidate::SQLITE_WRITER_LEASE_DURATION_MS.to_string(),
    );
    environment.configuration.insert(
        "sqlite_lease_expiry_wait_ms".to_owned(),
        super::candidate::SQLITE_LEASE_EXPIRY_WAIT_MS.to_string(),
    );
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
    let invalid_chain = invalid_evidence_chain();
    let package = package_from_runs(
        WORK_PACKAGE_ID,
        &repository_commit,
        environment.clone(),
        methodology,
        medium_fixture,
        eligibility,
        append_artifacts,
        sqlite_artifacts,
        comparison.clone(),
        &readiness,
        invalid_chain.clone(),
    );

    write_package(
        &output_root,
        &package,
        &comparison,
        &readiness,
        &repository_commit,
        &invalid_chain,
    );
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
    harness_sha: &str,
    invalid_chain: &[InvalidPredecessorEvidence],
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

    let selection_validity = selection_validity_record(
        harness_sha,
        readiness,
        invalid_chain,
    );
    fs::write(
        output_root.join("selection-validity.json"),
        serde_json::to_string_pretty(&selection_validity).expect("selection validity"),
    )
    .expect("write selection validity");
}

fn selection_validity_record(
    harness_sha: &str,
    readiness: &super::readiness::ReadinessAssessment,
    invalid_chain: &[InvalidPredecessorEvidence],
) -> SelectionValidityRecord {
    let gate_ready = readiness.mechanism_selection_readiness == "ready_for_owner_decision";
    SelectionValidityRecord {
        selection_validity: if gate_ready {
            "pending_owner_gate_review".to_owned()
        } else {
            "not_ready_for_selection".to_owned()
        },
        harness_semantic_sha: harness_sha.to_owned(),
        harness_version: EVIDENCE_01C_HARNESS_VERSION.to_owned(),
        evidence_record_sha: "see_git_commit_preserving_this_directory".to_owned(),
        verdict: if gate_ready {
            "EVIDENCE_PACKAGE_COMPLETE_PENDING_GOVERNANCE".to_owned()
        } else {
            "NOT_READY_01C_EVIDENCE".to_owned()
        },
        reason: if readiness.blockers.is_empty() {
            vec![
                "single-platform evidence complete; awaiting governance review pipeline".to_owned(),
            ]
        } else {
            readiness.blockers.clone()
        },
        invalid_evidence_chain: invalid_chain.to_vec(),
        preserve_raw_artifacts: true,
        note: format!(
            "Contract v4 reconciliation harness at {harness_sha}. Mechanism selection remains owner-gated after Grok methodology, PRE_FINAL_GOVERNANCE_AUDIT, GPT-5.6 Sol High STRONG_FINAL_CONFLICT_REVIEW, and FINAL_GATE_GOVERNANCE_CHECK."
        ),
    }
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

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use vox_proof::persistence_evidence::{
    AppendBundleAdapter, EmbeddedRelationalAdapter, EvidenceFixture, EvidenceManifest,
    EvidenceRunResult, HARNESS_VERSION, KnownOrUnavailable, ORACLE_VERSION,
    PersistenceCandidateAdapter, SCENARIO_CATALOG_VERSION, SMALL_FIXTURE_ID, SMALL_FIXTURE_VERSION,
    ScenarioRunner, fresh_storage_root,
};

fn main() {
    let run_id = std::env::var("VOXPROOF_EVIDENCE_RUN_ID")
        .unwrap_or_else(|_| format!("spike-v1-{}", timestamp_slug()));
    let output_root = PathBuf::from("evidence/persistence").join(&run_id);
    fs::create_dir_all(&output_root).expect("evidence output directory");
    fs::create_dir_all(output_root.join("raw")).expect("raw evidence directory");

    let repository_commit = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());

    let host_platform = std::env::consts::OS;
    let fixture = EvidenceFixture::small();
    let mut embedded = EmbeddedRelationalAdapter::new(fresh_storage_root("embedded-run"));
    let mut append = AppendBundleAdapter::new(fresh_storage_root("append-run"));

    let embedded_id = embedded.candidate_id().to_string();
    let embedded_version = embedded.candidate_version().to_string();
    let embedded_result = run_candidate(
        &mut embedded,
        &fixture,
        base_manifest(&repository_commit, &embedded_id, &embedded_version),
    );
    let append_id = append.candidate_id().to_string();
    let append_version = append.candidate_version().to_string();
    let append_result = run_candidate(
        &mut append,
        &fixture,
        base_manifest(&repository_commit, &append_id, &append_version),
    );

    write_json(
        &output_root.join("manifest.json"),
        &serde_json::json!({
            "run_id": run_id,
            "repository_commit": repository_commit,
            "targets_path": "evidence/persistence/spike-v1/targets.json",
            "candidate_classes_path": "evidence/persistence/spike-v1/candidate-classes.json",
            "platform": host_platform,
            "harness_version": HARNESS_VERSION,
            "oracle_version": ORACLE_VERSION,
            "scenario_catalog_version": SCENARIO_CATALOG_VERSION,
        }),
    );
    write_json(
        &output_root.join("candidate-a-results.json"),
        &embedded_result,
    );
    write_json(
        &output_root.join("candidate-b-results.json"),
        &append_result,
    );
    let windows_status = if host_platform == "windows" {
        "executed_on_host"
    } else {
        "NotRun"
    };
    let macos_status = if host_platform == "macos" {
        "executed_on_host"
    } else {
        "NotRun"
    };
    write_json(
        &output_root.join("comparison.json"),
        &comparison_summary(
            &embedded_result,
            &append_result,
            host_platform,
            macos_status,
            windows_status,
        ),
    );
    write_json(
        &output_root.join("limitations.json"),
        &serde_json::json!({
            "host_platform": host_platform,
            "macos_status": macos_status,
            "windows_status": windows_status,
            "cross_platform_claims": "Inconclusive",
            "comparative_measurements_status": "not_executed",
            "comparative_measurements_note": "Spike v1 records per-scenario scenario_elapsed_ms only; declared cold/warm multi-sample metrics in targets.json are deferred.",
            "hardware_power_loss": "not tested",
            "destructive_historical_gc": "unsupported in spike adapters",
            "fault_injection_layers": ["logical"],
            "negative_results_retained": true
        }),
    );

    println!("Evidence written to {}", output_root.display());
    println!("Embedded eligibility: {:?}", embedded_result.eligibility);
    println!("Append eligibility: {:?}", append_result.eligibility);
}

fn run_candidate(
    adapter: &mut impl PersistenceCandidateAdapter,
    fixture: &EvidenceFixture,
    manifest: EvidenceManifest,
) -> EvidenceRunResult {
    ScenarioRunner::run_catalog(adapter, fixture, manifest)
}

fn base_manifest(
    repository_commit: &str,
    candidate_id: &str,
    candidate_version: &str,
) -> EvidenceManifest {
    EvidenceManifest {
        evidence_protocol_version: "md-015-accepted-v1".to_string(),
        repository_commit: repository_commit.to_string(),
        candidate_id: candidate_id.to_string(),
        candidate_version: candidate_version.to_string(),
        fixture_id: SMALL_FIXTURE_ID.to_string(),
        fixture_version: SMALL_FIXTURE_VERSION.to_string(),
        harness_version: HARNESS_VERSION.to_string(),
        oracle_version: ORACLE_VERSION.to_string(),
        scenario_ids: Vec::new(),
        operating_system: KnownOrUnavailable::Known(std::env::consts::OS.to_string()),
        operating_system_version: KnownOrUnavailable::Unavailable {
            reason: "host metadata collected separately".to_string(),
        },
        filesystem: KnownOrUnavailable::Unavailable {
            reason: "not probed in spike runner".to_string(),
        },
        hardware_summary: KnownOrUnavailable::Unavailable {
            reason: "not probed in spike runner".to_string(),
        },
        runtime_versions: BTreeMap::from([(
            "rustc".to_string(),
            KnownOrUnavailable::Known(
                std::env::var("RUSTC_VERSION").unwrap_or_else(|_| "unknown".to_string()),
            ),
        )]),
        configuration: BTreeMap::from([("feature".to_string(), "persistence-spike".to_string())]),
        start_timestamp: KnownOrUnavailable::Unavailable {
            reason: "runner uses deterministic scenario execution".to_string(),
        },
        end_timestamp: KnownOrUnavailable::Unavailable {
            reason: "runner uses deterministic scenario execution".to_string(),
        },
        known_limitations: vec![
            "spike adapters only".to_string(),
            "macOS host execution unless CI overrides".to_string(),
        ],
    }
}

fn comparison_summary(
    embedded: &EvidenceRunResult,
    append: &EvidenceRunResult,
    host_platform: &str,
    macos_status: &str,
    windows_status: &str,
) -> serde_json::Value {
    serde_json::json!({
        "host_platform": host_platform,
        "embedded_relational": summary_row(embedded, host_platform),
        "append_bundle": summary_row(append, host_platform),
        "macos_status": macos_status,
        "windows_status": windows_status,
        "cross_platform_claims": "Inconclusive",
        "selection_status": "none",
        "eligibility_note": "Host-platform correctness eligibility only; EligibleForComparison is non-authoritative and does not select a mechanism"
    })
}

fn summary_row(result: &EvidenceRunResult, host_platform: &str) -> serde_json::Value {
    serde_json::json!({
        "candidate_id": result.manifest.candidate_id,
        "platform_scope": host_platform,
        "host_correctness_eligibility": format!("{:?}", result.eligibility),
        "passed": result.summary.passed,
        "failed": result.summary.failed,
        "unsupported": result.summary.unsupported,
        "not_run": result.summary.not_run,
        "inconclusive": result.summary.inconclusive,
        "negative_results": result.negative_results.len(),
    })
}

fn write_json(path: &PathBuf, value: &impl serde::Serialize) {
    let json = serde_json::to_vec_pretty(value).expect("serialize evidence json");
    fs::write(path, json).expect("write evidence json");
}

fn timestamp_slug() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

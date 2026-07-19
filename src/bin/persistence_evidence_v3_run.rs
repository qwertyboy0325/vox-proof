//! Package 2D v3 cross-platform and durability evidence runner.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};
use vox_proof::persistence_evidence::{
    EvidenceFixture, EvidenceManifest, EvidenceRunResult, KnownOrUnavailable, ORACLE_VERSION,
    SCENARIO_CATALOG_VERSION, SMALL_FIXTURE_ID, SMALL_FIXTURE_VERSION, SqliteScenarioRunner,
    V3_HARNESS_VERSION, PlatformProfile, TrialOutcome,
    build_platform_matrix,
    DurabilityTrialRunner, MIN_TRIALS_PER_POINT,
};

fn main() {
    let run_id = std::env::var("VOXPROOF_EVIDENCE_RUN_ID")
        .unwrap_or_else(|_| format!("spike-v3-sqlite-{}", timestamp_slug()));
    let output_root = PathBuf::from("evidence/persistence").join(&run_id);
    let platform_dir = platform_subdirectory(&output_root);
    fs::create_dir_all(&platform_dir).expect("platform dir");
    fs::create_dir_all(platform_dir.join("scenario-results")).expect("scenario-results");

    let repository_commit = git_head();
    let profile = PlatformProfile::capture(&repository_commit);
    let trials_per = std::env::var("VOXPROOF_TRIALS_PER_EXPERIMENT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(MIN_TRIALS_PER_POINT);
    let rustc = profile.rustc_version.clone();
    let execution_command = format!(
        "RUSTC_VERSION={} VOXPROOF_EVIDENCE_RUN_ID={} cargo run --features persistence-spike --bin persistence_evidence_v3_run",
        rustc, run_id
    );

    let fixture = EvidenceFixture::small();
    let manifest = build_manifest(&repository_commit, &execution_command, &profile);

    let runner = SqliteScenarioRunner::new();
    let (mut scenario_result, mut artifacts) = runner.run_cross_platform_subset(&fixture, manifest);
    scenario_result.manifest.end_timestamp =
        KnownOrUnavailable::Known(timestamp_iso());
    artifacts.commands.push(execution_command.clone());

    let durability_runner = DurabilityTrialRunner::new(profile.platform_label.clone());
    let trial_results = durability_runner.run_all(&fixture, trials_per);

    write_scenario_results(&platform_dir, &scenario_result);
    write_platform_artifacts(
        &output_root,
        &platform_dir,
        &run_id,
        &repository_commit,
        &profile,
        &scenario_result,
        &artifacts,
        &trial_results,
        trials_per,
        &execution_command,
    );

    if let Ok(peer_dir) = std::env::var("VOXPROOF_PEER_PLATFORM_DIR") {
        merge_peer_platform(&output_root, &run_id, &platform_dir, Path::new(&peer_dir));
    }

    println!("V3 evidence written to {}", output_root.display());
}

fn platform_subdirectory(output_root: &Path) -> PathBuf {
    let label = std::env::var("VOXPROOF_PLATFORM_LABEL").unwrap_or_else(|_| {
        match std::env::consts::OS {
            "macos" => "macos".to_string(),
            "windows" => "windows".to_string(),
            other => other.to_string(),
        }
    });
    output_root.join("platforms").join(label)
}

fn build_manifest(
    repository_commit: &str,
    execution_command: &str,
    profile: &PlatformProfile,
) -> EvidenceManifest {
    EvidenceManifest {
        evidence_protocol_version: "md-015-accepted-v1".to_string(),
        repository_commit: repository_commit.to_string(),
        candidate_id: "embedded-relational-sqlite-spike".to_string(),
        candidate_version: "1".to_string(),
        fixture_id: SMALL_FIXTURE_ID.to_string(),
        fixture_version: SMALL_FIXTURE_VERSION.to_string(),
        harness_version: V3_HARNESS_VERSION.to_string(),
        oracle_version: ORACLE_VERSION.to_string(),
        scenario_ids: Vec::new(),
        operating_system: KnownOrUnavailable::Known(profile.operating_system.clone()),
        operating_system_version: KnownOrUnavailable::Known(profile.operating_system_version.clone()),
        filesystem: KnownOrUnavailable::Known(profile.filesystem.clone()),
        hardware_summary: KnownOrUnavailable::Unavailable {
            reason: "not probed".to_string(),
        },
        runtime_versions: BTreeMap::from([
            (
                "rustc".to_string(),
                KnownOrUnavailable::Known(profile.rustc_version.clone()),
            ),
            (
                "sqlite".to_string(),
                KnownOrUnavailable::Known(profile.sqlite_version.clone()),
            ),
            (
                "rusqlite".to_string(),
                KnownOrUnavailable::Known(profile.rusqlite_version.clone()),
            ),
        ]),
        configuration: BTreeMap::from([
            ("feature".to_string(), "persistence-spike".to_string()),
            ("execution_command".to_string(), execution_command.to_string()),
            (
                "platform_label".to_string(),
                profile.platform_label.clone(),
            ),
        ]),
        start_timestamp: KnownOrUnavailable::Known(timestamp_iso()),
        end_timestamp: KnownOrUnavailable::Unavailable {
            reason: "filled after run".to_string(),
        },
        known_limitations: vec![
            "Package 2D cross-platform and durability evidence".to_string(),
            "FilesystemDurability not credited without VM power-off".to_string(),
            "HardwarePowerLoss not demonstrated".to_string(),
            "no mechanism selection claim".to_string(),
        ],
    }
}

#[allow(clippy::too_many_arguments)]
fn write_platform_artifacts(
    output_root: &Path,
    platform_dir: &Path,
    run_id: &str,
    repository_commit: &str,
    profile: &PlatformProfile,
    scenario_result: &EvidenceRunResult,
    artifacts: &vox_proof::persistence_evidence::SqliteEvidenceArtifacts,
    trial_results: &[vox_proof::persistence_evidence::DurabilityTrialResult],
    trials_per: u32,
    execution_command: &str,
) {
    let environment = serde_json::json!({
        "run_id": run_id,
        "repository_commit": repository_commit,
        "work_package_id": "persistence-package-2d-cross-platform-durability",
        "package_2c_head": profile.package_2c_head,
        "package_2c_evidence_run": profile.package_2c_evidence_run,
        "platform_profile": profile,
        "trials_per_experiment": trials_per,
        "scenario_contract_version": "4",
        "design_contract_version": "sqlite-design-v1",
        "harness_version": V3_HARNESS_VERSION,
        "authorized_file_scope_audit": {
            "work_package_id": "persistence-package-2d-cross-platform-durability",
            "scope_threshold_exceeded": true,
            "note": "Package 2D cross-platform durability harness"
        },
    });

    write_json(
        &output_root.join("manifest.json"),
        &serde_json::json!({
            "run_id": run_id,
            "repository_commit": repository_commit,
            "candidate_id": "embedded-relational-sqlite-spike",
            "harness_version": V3_HARNESS_VERSION,
            "oracle_version": ORACLE_VERSION,
            "scenario_catalog_version": SCENARIO_CATALOG_VERSION,
            "design_contract_reference": "evidence/persistence/sqlite-design-v1/authority-contract.json",
            "platform": profile.operating_system,
            "platform_label": profile.platform_label,
            "architecture": profile.architecture,
            "package_2c_head": profile.package_2c_head,
        }),
    );
    write_json(&output_root.join("environment.json"), &environment);
    write_json(
        &output_root.join("candidate.json"),
        &serde_json::json!({
            "candidate_id": "embedded-relational-sqlite-spike",
            "candidate_version": "1",
        }),
    );
    write_json(&platform_dir.join("scenario-results.json"), scenario_result);
    write_json(
        &output_root.join("scenario-results.json"),
        scenario_result,
    );
    write_json(
        &platform_dir.join("process-events.json"),
        &artifacts.process_events,
    );
    write_json(
        &platform_dir.join("fault-executions.json"),
        &artifacts.fault_executions,
    );
    write_json(
        &platform_dir.join("oracle-observations.json"),
        &artifacts.oracle_observations,
    );
    write_json(
        &output_root.join("trial-results.json"),
        trial_results,
    );
    write_json(
        &output_root.join("durability-events.json"),
        trial_results,
    );
    write_json(
        &output_root.join("filesystem-observations.json"),
        &serde_json::json!({
            "directory_sync_capability": profile.directory_sync_capability,
            "note": "synchronous=FULL does not imply stable storage proof",
        }),
    );
    write_json(
        &output_root.join("commands.json"),
        &vec![execution_command.to_string()],
    );

    let checksums = checksum_dir(output_root);
    write_json(&output_root.join("checksums.json"), &checksums);

    let passed = scenario_result.summary.passed;
    let failed = scenario_result.summary.failed;
    let trials_passed = trial_results
        .iter()
        .filter(|t| t.outcome == TrialOutcome::Passed)
        .count();
    let summary = format!(
        "# Package 2D V3 Evidence\n\nrun_id: {run_id}\nplatform: {}\ncommit: {repository_commit}\nscenarios passed: {passed} failed: {failed}\ntrials passed: {trials_passed}/{}\n",
        profile.platform_label,
        trial_results.len(),
    );
    fs::write(output_root.join("summary.md"), summary).expect("summary");
}

fn write_scenario_results(platform_dir: &Path, result: &EvidenceRunResult) {
    for scenario in &result.scenario_results {
        let path = platform_dir
            .join("scenario-results")
            .join(format!("{}.json", scenario.scenario_identity.scenario_id));
        write_json(&path, scenario);
    }
}

fn merge_peer_platform(output_root: &Path, run_id: &str, local_dir: &Path, peer_dir: &Path) {
    let local_results: EvidenceRunResult = match read_json(&local_dir.join("scenario-results.json")) {
        Ok(r) => r,
        Err(error) => {
            eprintln!("merge skip local: {error}");
            return;
        }
    };
    let peer_results: EvidenceRunResult = match read_json(&peer_dir.join("scenario-results.json")) {
        Ok(r) => r,
        Err(error) => {
            eprintln!("merge skip peer: {error}");
            return;
        }
    };

    let (macos_id, windows_id, macos_results, windows_results) =
        if std::env::consts::OS == "macos" {
            (
                Some(run_id.to_string()),
                std::env::var("VOXPROOF_PEER_RUN_ID").ok(),
                local_results.scenario_results.clone(),
                peer_results.scenario_results.clone(),
            )
        } else {
            (
                std::env::var("VOXPROOF_PEER_RUN_ID").ok(),
                Some(run_id.to_string()),
                peer_results.scenario_results.clone(),
                local_results.scenario_results.clone(),
            )
        };

    let matrix = build_platform_matrix(
        macos_id.as_deref(),
        windows_id.as_deref(),
        &macos_results,
        &windows_results,
    );
    write_json(&output_root.join("platform-matrix.json"), &matrix);
}

fn write_json(path: &Path, value: &(impl serde::Serialize + ?Sized)) {
    let json = serde_json::to_vec_pretty(value).expect("serialize");
    fs::write(path, json).expect("write");
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

fn git_head() -> String {
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn timestamp_slug() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

fn timestamp_iso() -> String {
    timestamp_slug()
}

fn checksum_dir(root: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    fn walk(base: &Path, root: &Path, out: &mut BTreeMap<String, String>) {
        let Ok(entries) = fs::read_dir(base) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if path.file_name().is_some_and(|name| name == "checksums.json") {
                    continue;
                }
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();
                let data = fs::read(&path).expect("read for checksum");
                let hash = Sha256::digest(&data);
                out.insert(rel, format!("sha256:{hash:x}"));
            } else if path.is_dir() {
                walk(&path, root, out);
            }
        }
    }
    walk(root, root, &mut out);
    out
}

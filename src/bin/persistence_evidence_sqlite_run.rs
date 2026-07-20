use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};
use vox_proof::persistence_evidence::{
    EvidenceFixture, EvidenceManifest, KnownOrUnavailable, ORACLE_VERSION,
    SCENARIO_CATALOG_VERSION, SMALL_FIXTURE_ID, SMALL_FIXTURE_VERSION,
    SQLITE_EVIDENCE_HARNESS_VERSION, SqliteScenarioRunner,
};

fn main() {
    let run_id = std::env::var("VOXPROOF_EVIDENCE_RUN_ID")
        .unwrap_or_else(|_| format!("spike-v2-sqlite-{}", timestamp_slug()));
    let output_root = PathBuf::from("evidence/persistence").join(&run_id);
    fs::create_dir_all(&output_root).expect("output root");
    fs::create_dir_all(output_root.join("scenario-results")).expect("scenario-results dir");

    let repository_commit = git_head();
    let rustc = std::env::var("RUSTC_VERSION").unwrap_or_else(|_| rustc_version());
    let execution_command = format!(
        "RUSTC_VERSION={} VOXPROOF_EVIDENCE_RUN_ID={} cargo run --features persistence-spike --bin persistence_evidence_sqlite_run",
        rustc, run_id
    );
    let fixture = EvidenceFixture::small();
    let manifest = EvidenceManifest {
        evidence_protocol_version: "md-015-accepted-v1".to_string(),
        repository_commit: repository_commit.clone(),
        candidate_id: "embedded-relational-sqlite-spike".to_string(),
        candidate_version: "1".to_string(),
        fixture_id: SMALL_FIXTURE_ID.to_string(),
        fixture_version: SMALL_FIXTURE_VERSION.to_string(),
        harness_version: SQLITE_EVIDENCE_HARNESS_VERSION.to_string(),
        oracle_version: ORACLE_VERSION.to_string(),
        scenario_ids: Vec::new(),
        operating_system: KnownOrUnavailable::Known(std::env::consts::OS.to_string()),
        operating_system_version: KnownOrUnavailable::Known(os_version()),
        filesystem: KnownOrUnavailable::Unavailable {
            reason: "not probed in sqlite evidence run".to_string(),
        },
        hardware_summary: KnownOrUnavailable::Unavailable {
            reason: "not probed in sqlite evidence run".to_string(),
        },
        runtime_versions: BTreeMap::from([
            (
                "rustc".to_string(),
                KnownOrUnavailable::Known(rustc.clone()),
            ),
            (
                "sqlite".to_string(),
                KnownOrUnavailable::Known(sqlite_version()),
            ),
            (
                "rusqlite".to_string(),
                KnownOrUnavailable::Known("0.32".to_string()),
            ),
        ]),
        configuration: BTreeMap::from([
            ("feature".to_string(), "persistence-spike".to_string()),
            ("execution_command".to_string(), execution_command.clone()),
        ]),
        start_timestamp: KnownOrUnavailable::Known(timestamp_iso()),
        end_timestamp: KnownOrUnavailable::Unavailable {
            reason: "filled after run".to_string(),
        },
        known_limitations: vec![
            "Package 2C sqlite-targeted evidence only".to_string(),
            "no FilesystemDurability claim".to_string(),
            "no CrossPlatform claim".to_string(),
        ],
    };

    let runner = SqliteScenarioRunner::new();
    let (result, mut artifacts) = runner.run_catalog(&fixture, manifest);
    artifacts.commands.push(execution_command);

    for scenario in &result.scenario_results {
        let path = output_root
            .join("scenario-results")
            .join(format!("{}.json", scenario.scenario_identity.scenario_id));
        write_json(&path, scenario);
    }

    let environment = serde_json::json!({
        "run_id": run_id,
        "repository_commit": repository_commit,
        "platform": std::env::consts::OS,
        "architecture": std::env::consts::ARCH,
        "rustc": rustc,
        "sqlite_version": sqlite_version(),
        "rusqlite_version": "0.32",
        "child_termination_mechanism": "std::process::abort for crash faults; SIGKILL for parent-killed hold workers",
        "process_topology": "parent-controller + persistence_evidence_worker child",
        "authorized_file_scope_audit": {
            "work_package_id": "persistence-package-2c-sqlite-targeted-evidence",
            "tracked_files_changed": 17,
            "scope_threshold_exceeded": true,
            "authorized_areas": [
                "src/persistence_evidence/process_harness.rs",
                "src/persistence_evidence/independent_oracle.rs",
                "src/persistence_evidence/sqlite_scenario_runner.rs",
                "src/bin/persistence_evidence_worker.rs",
                "src/bin/persistence_evidence_sqlite_run.rs",
                "src/persistence_evidence/scenario_runner.rs",
                "src/persistence_evidence/model.rs",
                "src/persistence_evidence/mod.rs",
                "src/persistence_evidence/candidates/fault.rs",
                "src/persistence_evidence/candidates/embedded_relational.rs",
                "tests/persistence_scenarios.rs",
                "tests/persistence_sqlite_evidence.rs",
                "tests/persistence_evidence.rs",
                "tests/persistence_metadata.rs",
                "evidence/persistence/spike-v1-review/scenario-claim-contracts.json",
                "evidence/persistence/spike-v1-review/reclassification.json",
                "Cargo.toml"
            ],
            "explicitly_excluded": ["append_bundle.rs", "Package 2D filesystem durability"]
        },
        "scenario_contract_version": "4",
        "design_contract_version": "sqlite-design-v1",
        "implementation_baseline": "persistence-package-2b-sqlite-implementation",
    });

    write_json(
        &output_root.join("manifest.json"),
        &serde_json::json!({
            "run_id": run_id,
            "repository_commit": repository_commit,
            "candidate_id": "embedded-relational-sqlite-spike",
            "harness_version": SQLITE_EVIDENCE_HARNESS_VERSION,
            "oracle_version": ORACLE_VERSION,
            "scenario_catalog_version": SCENARIO_CATALOG_VERSION,
            "design_contract_reference": "evidence/persistence/sqlite-design-v1/authority-contract.json",
            "platform": std::env::consts::OS,
            "architecture": std::env::consts::ARCH,
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
    write_json(&output_root.join("scenario-results.json"), &result);
    write_json(
        &output_root.join("process-events.json"),
        &artifacts.process_events,
    );
    write_json(
        &output_root.join("fault-executions.json"),
        &artifacts.fault_executions,
    );
    write_json(
        &output_root.join("oracle-observations.json"),
        &artifacts.oracle_observations,
    );
    write_json(&output_root.join("commands.json"), &artifacts.commands);

    let checksums = checksum_dir(&output_root);
    write_json(&output_root.join("checksums.json"), &checksums);

    let summary = format!(
        "# SQLite Package 2C Evidence\n\nrun_id: {run_id}\ncommit: {repository_commit}\npassed: {}\nfailed: {}\nunsupported: {}\n",
        result.summary.passed, result.summary.failed, result.summary.unsupported,
    );
    fs::write(output_root.join("summary.md"), summary).expect("summary");

    println!("Evidence written to {}", output_root.display());
}

fn write_json(path: &PathBuf, value: &impl serde::Serialize) {
    let json = serde_json::to_vec_pretty(value).expect("serialize");
    fs::write(path, json).expect("write");
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

fn rustc_version() -> String {
    Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn sqlite_version() -> String {
    "bundled-with-rusqlite-0.32".to_string()
}

fn os_version() -> String {
    Command::new("uname")
        .args(["-sr"])
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

fn checksum_dir(root: &PathBuf) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for entry in fs::read_dir(root).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_file() {
            let data = fs::read(&path).expect("read for checksum");
            let hash = Sha256::digest(&data);
            out.insert(
                path.file_name().unwrap().to_string_lossy().to_string(),
                format!("sha256:{hash:x}"),
            );
        }
    }
    out
}

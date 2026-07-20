//! Platform capture and cross-platform normalization (Package 2D).

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

pub const PACKAGE_2C_HEAD: &str = "8cc0b8209282065db0521d612b07b1559e4f1183";
pub const PACKAGE_2C_EVIDENCE_RUN: &str = "spike-v2-sqlite-20260719-210916";
/// Design contract revision referenced by SQLite spike evidence manifests.
pub const DESIGN_CONTRACT_VERSION: &str = "sqlite-design-v1";
pub const V3_HARNESS_VERSION: &str = "sqlite-evidence-v3";

/// Scenarios required for Package 2D cross-platform subset.
pub const CROSS_PLATFORM_SCENARIO_IDS: &[&str] = &[
    "baseline-create-open-close",
    "append-correction-event",
    "attach-analysis-result",
    "stale-review-ledger-command",
    "concurrent-writer-attempt",
    "unknown-newer-format",
    "derived-state-corruption",
    "canonical-reference-corruption",
    "semantic-duplication",
    "interrupted-authoritative-transition",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SqlitePragmaSnapshot {
    pub journal_mode: String,
    pub synchronous: i64,
    pub foreign_keys: i64,
    pub busy_timeout_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectorySyncCapability {
    pub parent_directory_fsync: bool,
    pub note: String,
}

impl DirectorySyncCapability {
    pub fn not_implemented() -> Self {
        Self {
            parent_directory_fsync: false,
            note: "publication uses fs::rename without parent directory fsync".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformProfile {
    pub platform_label: String,
    pub operating_system: String,
    pub operating_system_version: String,
    pub architecture: String,
    pub execution_environment: String,
    pub filesystem: String,
    pub hypervisor: Option<String>,
    pub storage_cache_model: Option<String>,
    pub rustc_version: String,
    pub sqlite_version: String,
    pub rusqlite_version: String,
    pub repository_commit: String,
    pub package_2c_head: String,
    pub package_2c_evidence_run: String,
    pub child_termination_mechanism: String,
    pub directory_sync_capability: DirectorySyncCapability,
    pub design_contract_version: String,
}

impl PlatformProfile {
    pub fn capture(repository_commit: &str) -> Self {
        let platform_label =
            std::env::var("VOXPROOF_PLATFORM_LABEL").unwrap_or_else(
                |_| match std::env::consts::OS {
                    "macos" => "macos-native".to_string(),
                    "windows" => "windows-native".to_string(),
                    other => format!("{other}-native"),
                },
            );
        let execution_environment =
            std::env::var("VOXPROOF_EXECUTION_ENVIRONMENT").unwrap_or_else(|_| {
                if platform_label.contains("github") {
                    "github-actions".to_string()
                } else {
                    format!("{}-native-host", std::env::consts::OS)
                }
            });
        Self {
            platform_label: platform_label.clone(),
            operating_system: std::env::consts::OS.to_string(),
            operating_system_version: os_version(),
            architecture: std::env::consts::ARCH.to_string(),
            execution_environment,
            filesystem: probe_filesystem(),
            hypervisor: std::env::var("VOXPROOF_HYPERVISOR").ok(),
            storage_cache_model: std::env::var("VOXPROOF_STORAGE_CACHE_MODEL").ok(),
            rustc_version: rustc_version(),
            sqlite_version: "bundled-with-rusqlite-0.32".to_string(),
            rusqlite_version: "0.32".to_string(),
            repository_commit: repository_commit.to_string(),
            package_2c_head: PACKAGE_2C_HEAD.to_string(),
            package_2c_evidence_run: PACKAGE_2C_EVIDENCE_RUN.to_string(),
            child_termination_mechanism: child_termination_note(),
            directory_sync_capability: DirectorySyncCapability::not_implemented(),
            design_contract_version: DESIGN_CONTRACT_VERSION.to_string(),
        }
    }
}

/// Evidence harness utility for optional platform capture fields.
#[allow(dead_code)]
pub fn probe_sqlite_pragma(db_path: &Path) -> Result<SqlitePragmaSnapshot, String> {
    let connection =
        Connection::open(db_path).map_err(|error| format!("open for pragma probe: {error}"))?;
    let journal_mode: String = connection
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    let synchronous: i64 = connection
        .query_row("PRAGMA synchronous", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    let foreign_keys: i64 = connection
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    let busy_timeout_ms: i64 = connection
        .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    Ok(SqlitePragmaSnapshot {
        journal_mode,
        synchronous,
        foreign_keys,
        busy_timeout_ms,
    })
}

/// Evidence harness utility for optional WAL companion observation.
#[allow(dead_code)]
pub fn wal_companion_paths(db_path: &Path) -> BTreeMap<String, bool> {
    let mut out = BTreeMap::new();
    let stem = db_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("session");
    let parent = db_path.parent().unwrap_or_else(|| Path::new("."));
    for suffix in ["-wal", "-shm"] {
        let companion = parent.join(format!("{stem}{suffix}"));
        out.insert(
            companion
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            companion.exists(),
        );
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformScenarioRow {
    pub scenario_id: String,
    pub macos_status: Option<String>,
    pub windows_status: Option<String>,
    pub macos_oracle_fingerprint: Option<String>,
    pub windows_oracle_fingerprint: Option<String>,
    pub macos_error_code: Option<String>,
    pub windows_error_code: Option<String>,
    pub equivalence_result: PlatformEquivalenceResult,
    pub cross_platform_credited: bool,
    pub differences: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformEquivalenceResult {
    Equivalent,
    Different,
    MissingPlatform,
    NotCompared,
}

pub fn filesystem_safe_path_segment(segment: &str) -> String {
    segment
        .chars()
        .map(|character| match character {
            ':' | '<' | '>' | '"' | '|' | '?' | '*' | '/' | '\\' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect()
}

pub fn normalize_platform_label(os: &str, env: &str) -> String {
    if env.contains("github") {
        format!("{os}-github-actions")
    } else if os == "macos" {
        "macos-native".to_string()
    } else if os == "windows" {
        "windows-native".to_string()
    } else {
        format!("{os}-native")
    }
}

fn os_version() -> String {
    if std::env::consts::OS == "windows" {
        Command::new("cmd")
            .args(["/C", "ver"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    } else {
        Command::new("uname")
            .args(["-sr"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }
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

fn probe_filesystem() -> String {
    std::env::var("VOXPROOF_FILESYSTEM").unwrap_or_else(|_| match std::env::consts::OS {
        "macos" => "apfs-assumed-native".to_string(),
        "windows" => "ntfs-assumed-native".to_string(),
        other => format!("{other}-unknown"),
    })
}

fn child_termination_note() -> String {
    if cfg!(unix) {
        "std::process::abort for crash faults; child.kill() maps to SIGKILL on Unix".to_string()
    } else {
        "std::process::abort for crash faults; child.kill() uses TerminateProcess on Windows"
            .to_string()
    }
}

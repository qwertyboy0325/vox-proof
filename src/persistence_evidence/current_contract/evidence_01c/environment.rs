use std::collections::BTreeMap;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use super::types::EnvironmentRecord;

pub fn capture_environment(repository_commit: &str, harness_version: &str) -> EnvironmentRecord {
    let platform_label =
        std::env::var("VOXPROOF_PLATFORM_LABEL").unwrap_or_else(|_| match std::env::consts::OS {
            "macos" => "macos_native".to_owned(),
            "windows" => "windows_github_actions".to_owned(),
            other => format!("{other}_native"),
        });
    let execution_environment =
        std::env::var("VOXPROOF_EXECUTION_ENVIRONMENT").unwrap_or_else(|_| {
            if platform_label.contains("github") {
                "github-actions".to_owned()
            } else {
                format!("{}-native-host", std::env::consts::OS)
            }
        });
    EnvironmentRecord {
        repository_commit: repository_commit.to_owned(),
        harness_version: harness_version.to_owned(),
        platform_label,
        operating_system: std::env::consts::OS.to_owned(),
        operating_system_version: os_version(),
        architecture: std::env::consts::ARCH.to_owned(),
        execution_environment,
        filesystem: std::env::var("VOXPROOF_FILESYSTEM").unwrap_or_else(|_| {
            match std::env::consts::OS {
                "macos" => "apfs-assumed-native".to_owned(),
                "windows" => "ntfs-assumed-native".to_owned(),
                other => format!("{other}-unknown"),
            }
        }),
        hardware_summary: hardware_summary(),
        rustc_version: rustc_version(),
        dependency_versions: BTreeMap::from([
            ("rusqlite".to_owned(), "0.32".to_owned()),
            ("serde".to_owned(), "1".to_owned()),
            ("serde_json".to_owned(), "1".to_owned()),
        ]),
        timing_source: "std::time::Instant wall clock".to_owned(),
        start_timestamp: timestamp_iso(),
        end_timestamp: None,
        configuration: BTreeMap::from([
            ("feature".to_owned(), "persistence-spike".to_owned()),
            (
                "measurement_contract_version".to_owned(),
                super::super::measurement::MEASUREMENT_CONTRACT_VERSION.to_owned(),
            ),
        ]),
    }
}

pub fn timestamp_iso() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch");
    format!("{}.{:03}Z", duration.as_secs(), duration.subsec_millis())
}

fn os_version() -> String {
    if std::env::consts::OS == "windows" {
        Command::new("cmd")
            .args(["/C", "ver"])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .unwrap_or_else(|| "unknown".to_owned())
    } else {
        Command::new("uname")
            .args(["-sr"])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .unwrap_or_else(|| "unknown".to_owned())
    }
}

fn rustc_version() -> String {
    Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn hardware_summary() -> String {
    if std::env::consts::OS == "macos" {
        if std::env::consts::ARCH == "aarch64" {
            Command::new("sysctl")
                .args(["-n", "machdep.cpu.brand_string"])
                .output()
                .ok()
                .filter(|output| output.status.success())
                .map(|output| {
                    let brand = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if brand.is_empty() || brand == "Apple processor" {
                        format!("Apple Silicon ({})", std::env::consts::ARCH)
                    } else {
                        brand
                    }
                })
                .unwrap_or_else(|| format!("Apple Silicon ({})", std::env::consts::ARCH))
        } else {
            Command::new("sysctl")
                .args(["-n", "machdep.cpu.brand_string"])
                .output()
                .ok()
                .filter(|output| output.status.success())
                .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
                .unwrap_or_else(|| "unknown-mac-hardware".to_owned())
        }
    } else if std::env::consts::OS == "windows" {
        "github-actions-windows-latest-runner".to_owned()
    } else {
        "unknown".to_owned()
    }
}

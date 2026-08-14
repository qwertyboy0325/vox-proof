use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use super::super::measurement::MeasurementFixtureScale;
use super::candidate::{CurrentContractCandidate, CurrentContractCandidateKind};
use super::measurement_ops::{execute_measured_operation, MEASURE_RESULT_PREFIX};
use super::types::MeasurementSample;

pub fn run_sample_in_worker(
    sample_root: &Path,
    kind: CurrentContractCandidateKind,
    operation: &str,
    scale: MeasurementFixtureScale,
    sample_index: u32,
) -> MeasurementSample {
    let worker = worker_binary();
    let child = Command::new(&worker)
        .arg("measure-sample")
        .env("VOXPROOF_MEASURE_ROOT", sample_root)
        .env(
            "VOXPROOF_MEASURE_KIND",
            match kind {
                CurrentContractCandidateKind::Append => "append",
                CurrentContractCandidateKind::Append01B3 => "append-01b3",
                CurrentContractCandidateKind::Sqlite => "sqlite",
                CurrentContractCandidateKind::Sqlite01C3 => "sqlite-01c3",
            },
        )
        .env("VOXPROOF_MEASURE_OPERATION", operation)
        .env(
            "VOXPROOF_MEASURE_SCALE",
            match scale {
                MeasurementFixtureScale::Small => "small",
                MeasurementFixtureScale::Medium => "medium",
                MeasurementFixtureScale::Stress => "stress",
            },
        )
        .env("VOXPROOF_MEASURE_INDEX", sample_index.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| {
            panic!("spawn measurement worker at {}: {error}", worker.display())
        });

    let stdout = child
        .wait_with_output()
        .expect("measurement worker output")
        .stdout;
    let line = String::from_utf8_lossy(&stdout);
    for candidate in line.lines() {
        if let Some(json) = candidate.strip_prefix(MEASURE_RESULT_PREFIX) {
            return serde_json::from_str(json).expect("measurement worker json");
        }
    }
    panic!("measurement worker missing result prefix in {line}");
}

pub fn measure_sample_main() -> Result<(), String> {
    let root = std::env::var("VOXPROOF_MEASURE_ROOT").map_err(|_| "VOXPROOF_MEASURE_ROOT")?;
    let kind = std::env::var("VOXPROOF_MEASURE_KIND").map_err(|_| "VOXPROOF_MEASURE_KIND")?;
    let operation =
        std::env::var("VOXPROOF_MEASURE_OPERATION").map_err(|_| "VOXPROOF_MEASURE_OPERATION")?;
    let scale = std::env::var("VOXPROOF_MEASURE_SCALE").map_err(|_| "VOXPROOF_MEASURE_SCALE")?;
    let index = std::env::var("VOXPROOF_MEASURE_INDEX")
        .map_err(|_| "VOXPROOF_MEASURE_INDEX")?
        .parse::<u32>()
        .map_err(|_| "invalid index")?;
    let scale = match scale.as_str() {
        "small" => MeasurementFixtureScale::Small,
        "medium" => MeasurementFixtureScale::Medium,
        "stress" => MeasurementFixtureScale::Stress,
        other => return Err(format!("unknown scale {other}")),
    };
    let candidate = match kind.as_str() {
        "append" => CurrentContractCandidate::append(&root)?,
        "append-01b3" => CurrentContractCandidate::append_01b3(&root)?,
        "sqlite" => CurrentContractCandidate::sqlite(&root)?,
        "sqlite-01c3" => CurrentContractCandidate::sqlite_01c3(&root)?,
        other => return Err(format!("unknown kind {other}")),
    };
    let sample = execute_measured_operation(&candidate, &operation, scale, index);
    println!(
        "{MEASURE_RESULT_PREFIX}{}",
        serde_json::to_string(&sample).map_err(|e| e.to_string())?
    );
    Ok(())
}

pub fn child_hold_writer_main() -> Result<(), String> {
    let root = std::env::var("VOXPROOF_CHILD_ROOT").map_err(|_| "VOXPROOF_CHILD_ROOT")?;
    let session_id = std::env::var("VOXPROOF_CHILD_SESSION_ID").map_err(|_| "session id")?;
    let ready = std::env::var("VOXPROOF_CHILD_READY").map_err(|_| "ready path")?;
    let release = std::env::var("VOXPROOF_CHILD_RELEASE").map_err(|_| "release path")?;
    let kind = std::env::var("VOXPROOF_CHILD_KIND").map_err(|_| "kind")?;
    let candidate = match kind.as_str() {
        "append" => CurrentContractCandidate::append(&root)?,
        "append-01b3" => CurrentContractCandidate::append_01b3(&root)?,
        "sqlite" => CurrentContractCandidate::sqlite(&root)?,
        "sqlite-01c3" => CurrentContractCandidate::sqlite_01c3(&root)?,
        other => return Err(format!("unknown child kind {other}")),
    };
    let writer = candidate.open_writable(&session_id)?;
    std::fs::write(&ready, b"ready").map_err(|error| error.to_string())?;
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    while !Path::new(&release).exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    drop(writer);
    std::process::abort();
}

pub fn child_interrupt_transition_main() -> Result<(), String> {
    let root = std::env::var("VOXPROOF_CHILD_ROOT").map_err(|_| "VOXPROOF_CHILD_ROOT")?;
    let session_id = std::env::var("VOXPROOF_CHILD_SESSION_ID").map_err(|_| "session id")?;
    let ready = std::env::var("VOXPROOF_CHILD_READY").map_err(|_| "ready path")?;
    let release = std::env::var("VOXPROOF_CHILD_RELEASE").map_err(|_| "release path")?;
    let next_state_json = std::env::var("VOXPROOF_CHILD_NEXT_STATE").map_err(|_| "next state")?;
    let kind = std::env::var("VOXPROOF_CHILD_KIND").map_err(|_| "kind")?;
    let next_state: super::super::model::CurrentContractState =
        serde_json::from_str(&next_state_json).map_err(|error| error.to_string())?;
    let candidate = match kind.as_str() {
        "append" => CurrentContractCandidate::append(&root)?,
        "append-01b3" => CurrentContractCandidate::append_01b3(&root)?,
        "sqlite" => CurrentContractCandidate::sqlite(&root)?,
        "sqlite-01c3" => CurrentContractCandidate::sqlite_01c3(&root)?,
        other => return Err(format!("unknown child kind {other}")),
    };
    let mut writer = candidate.open_writable(&session_id)?;
    match kind.as_str() {
        "append" => {
            candidate.append_incomplete_tail_for_test(&mut writer, &next_state)?;
        }
        "append-01b3" => {
            candidate.append_incomplete_tail_for_test(&mut writer, &next_state)?;
        }
        "sqlite" => {
            candidate.arm_fail_before_commit_for_test();
            let _ = candidate.apply_transition(&mut writer, &next_state);
        }
        "sqlite-01c3" => {
            candidate.arm_fail_before_commit_for_test();
            let _ = candidate.apply_transition(&mut writer, &next_state);
        }
        _ => {}
    }
    std::fs::write(&ready, b"ready").map_err(|error| error.to_string())?;
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    while !Path::new(&release).exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    drop(writer);
    std::process::abort();
}

pub fn worker_binary() -> PathBuf {
    if let Ok(path) = std::env::var("VOXPROOF_01C_WORKER_BIN") {
        return PathBuf::from(path);
    }
    std::env::current_exe().expect("current exe")
}

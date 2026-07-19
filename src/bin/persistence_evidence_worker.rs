//! Child worker for persistence evidence process scenarios.

use std::io::{self, BufRead, Write};

use vox_proof::persistence_evidence::candidates::fault::{FaultExecutionMode, FaultPoint};
use vox_proof::persistence_evidence::candidates::semantic_ops::sample_append_event;
use vox_proof::persistence_evidence::{
    AuthoritativeCommand, EmbeddedRelationalAdapter, PersistenceCandidateAdapter, SemanticOpenMode,
    SemanticPrecondition,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("VOXPROOF_ERROR:{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let command = std::env::args().nth(1).ok_or("missing worker command")?;
    match command.as_str() {
        "hold-writer" => hold_writer(),
        "attempt-writer" => attempt_writer(),
        "apply-command-crash" => apply_command_crash(),
        "create-only" => create_only(),
        other => Err(format!("unknown worker command: {other}")),
    }
}

fn storage_root() -> Result<std::path::PathBuf, String> {
    std::env::var("VOXPROOF_STORAGE_ROOT")
        .map(std::path::PathBuf::from)
        .map_err(|_| "VOXPROOF_STORAGE_ROOT required".to_string())
}

fn fixture_from_env() -> Result<vox_proof::persistence_evidence::EvidenceFixture, String> {
    let fixture_json = std::env::var("VOXPROOF_FIXTURE_JSON")
        .map_err(|_| "VOXPROOF_FIXTURE_JSON required".to_string())?;
    serde_json::from_str(&fixture_json).map_err(|error| error.to_string())
}

fn fault_point_from_env() -> Result<FaultPoint, String> {
    let id = std::env::var("VOXPROOF_FAULT_POINT")
        .map_err(|_| "VOXPROOF_FAULT_POINT required".to_string())?;
    FaultPoint::from_env_id(&id).ok_or_else(|| format!("unknown fault point: {id}"))
}

fn hold_writer() -> Result<(), String> {
    let root = storage_root()?;
    let fixture = fixture_from_env()?;
    let mut adapter = EmbeddedRelationalAdapter::new(root);
    let session = adapter.create(&fixture).map_err(|e| e.to_string())?;
    let _handle = adapter
        .open(&session, SemanticOpenMode::Writable)
        .map_err(|e| e.to_string())?;
    println!("VOXPROOF_READY");
    io::stdout().flush().map_err(|e| e.to_string())?;
    let mut line = String::new();
    io::stdin()
        .lock()
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn attempt_writer() -> Result<(), String> {
    let root = storage_root()?;
    let session_id = std::env::var("VOXPROOF_SESSION_ID")
        .map_err(|_| "VOXPROOF_SESSION_ID required".to_string())?;
    let locator = root.join(&session_id);
    let mut adapter = EmbeddedRelationalAdapter::new(root);
    let session = vox_proof::persistence_evidence::EvidenceSessionRef::new(
        session_id,
        locator.to_string_lossy().to_string(),
    );
    match adapter.open(&session, SemanticOpenMode::Writable) {
        Ok(handle) => {
            let _ = adapter.close(&handle);
            println!("{RESULT_PREFIX}{}", serde_json::json!({"ok": true}));
        }
        Err(error) => {
            println!(
                "{RESULT_PREFIX}{}",
                serde_json::json!({"ok": false, "code": error.code, "message": error.message})
            );
        }
    }
    Ok(())
}

fn apply_command_crash() -> Result<(), String> {
    let root = storage_root()?;
    let fixture = fixture_from_env()?;
    let fault = fault_point_from_env()?;
    let command_id = std::env::var("VOXPROOF_COMMAND_OPERATION_ID")
        .map_err(|_| "VOXPROOF_COMMAND_OPERATION_ID required".to_string())?;
    let mut adapter = EmbeddedRelationalAdapter::new(root);
    adapter.set_fault_execution_mode(FaultExecutionMode::ProcessAbort);
    adapter.arm_fault_abort(fault);
    let session = adapter.create(&fixture).map_err(|e| e.to_string())?;
    let handle = adapter
        .open(&session, SemanticOpenMode::Writable)
        .map_err(|e| e.to_string())?;
    let state = adapter
        .read_normalized_state(&handle)
        .map_err(|e| e.to_string())?;
    println!("VOXPROOF_READY");
    io::stdout().flush().map_err(|e| e.to_string())?;
    let mut line = String::new();
    io::stdin()
        .lock()
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    let event = sample_append_event(&state);
    let command = AuthoritativeCommand::AppendCorrectionEvent {
        command_operation_id: command_id,
        event,
        preconditions: vec![SemanticPrecondition::ReviewLedgerHead {
            expected_event_id: state
                .review_ledger_events
                .last()
                .map(|event| event.event_id.clone()),
        }],
    };
    adapter
        .apply_authoritative_command(&handle, &command)
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn create_only() -> Result<(), String> {
    let root = storage_root()?;
    let fixture = fixture_from_env()?;
    let mut adapter = EmbeddedRelationalAdapter::new(root);
    let _session = adapter.create(&fixture).map_err(|e| e.to_string())?;
    println!("VOXPROOF_READY");
    Ok(())
}

const RESULT_PREFIX: &str = "VOXPROOF_RESULT:";

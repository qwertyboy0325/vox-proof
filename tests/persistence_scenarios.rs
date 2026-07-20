#![cfg(feature = "persistence-spike")]

use std::time::Duration;

use vox_proof::persistence_evidence::{EvidenceFixture, ProcessExitClassification, ProcessHarness};

fn harness_with_worker() -> ProcessHarness {
    ProcessHarness::new(std::path::PathBuf::from(env!(
        "CARGO_BIN_EXE_persistence_evidence_worker"
    )))
}

#[test]
fn worker_reaches_ready_and_can_be_killed() {
    let harness = harness_with_worker();
    let root = vox_proof::persistence_evidence::fresh_storage_root("harness-ready");
    let fixture = EvidenceFixture::small();
    let fixture_json = serde_json::to_string(&fixture).expect("fixture json");
    let root_lossy = root.to_string_lossy();
    let env = [
        ("VOXPROOF_STORAGE_ROOT", root_lossy.as_ref()),
        ("VOXPROOF_FIXTURE_JSON", fixture_json.as_str()),
    ];
    let held = harness
        .spawn_waiting_ready("hold-writer", &env, Duration::from_secs(10))
        .expect("ready");
    assert!(held.pid > 0);
    let outcome = harness.kill_held_worker(held, std::time::Instant::now());
    assert_eq!(outcome.classification, ProcessExitClassification::Signaled);
}

#[test]
fn attempt_writer_reports_rejection_json() {
    let harness = harness_with_worker();
    let root = vox_proof::persistence_evidence::fresh_storage_root("harness-attempt");
    let fixture = EvidenceFixture::small();
    let fixture_json = serde_json::to_string(&fixture).expect("fixture json");
    let session_id = fixture.normalized_state().session.session_id.clone();
    let root_lossy = root.to_string_lossy();
    let hold_env = [
        ("VOXPROOF_STORAGE_ROOT", root_lossy.as_ref()),
        ("VOXPROOF_FIXTURE_JSON", fixture_json.as_str()),
    ];
    let held = harness
        .spawn_waiting_ready("hold-writer", &hold_env, Duration::from_secs(10))
        .expect("ready");
    let attempt_env = [
        ("VOXPROOF_STORAGE_ROOT", root_lossy.as_ref()),
        ("VOXPROOF_SESSION_ID", session_id.as_str()),
    ];
    let attempt = harness
        .spawn_worker("attempt-writer", &attempt_env, Duration::from_secs(10))
        .expect("attempt");
    assert!(attempt.stdout.contains("writer-already-open"));
    let _ = harness.kill_held_worker(held, std::time::Instant::now());
}

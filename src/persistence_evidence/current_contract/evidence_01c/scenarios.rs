use std::process::Command;
use std::time::{Duration, Instant};

use super::super::measurement::MeasurementFixtureScale;
use super::super::scenario_contract::{
    scenario_contract_v3, ExpectedOpenState, ExpectedRecoveryClass, ScenarioContractV3,
};
use super::candidate::{
    fixture_state_for_scale, measurement_transition_states, scenario_fixture_state,
    updated_writer_token_state, CurrentContractCandidate, CurrentContractCandidateKind,
};
use super::measurement_worker::worker_binary;
use super::types::{
    fixture_scale_label, open_state_label, recovery_class_label, CorrectnessDisqualification,
    NormalizedScenarioResult, ScenarioExecutionStatus,
};

pub fn run_required_scenarios(
    candidate: &CurrentContractCandidate,
    platform: &str,
    work_root: &std::path::Path,
) -> (
    Vec<NormalizedScenarioResult>,
    Vec<CorrectnessDisqualification>,
) {
    let mut results = Vec::new();
    let mut disqualifications = Vec::new();
    for scenario in scenario_contract_v3() {
        let result = execute_scenario(candidate, &scenario, platform, work_root);
        if result.status == ScenarioExecutionStatus::Failed {
            disqualifications.push(CorrectnessDisqualification {
                gate: "scenario_failure".to_owned(),
                scenario_id: Some(scenario.scenario_id.clone()),
                candidate_id: candidate.candidate_id().to_owned(),
                detail: result
                    .failure_code
                    .clone()
                    .unwrap_or_else(|| "unknown".to_owned()),
            });
        }
        results.push(result);
    }
    (results, disqualifications)
}

fn execute_scenario(
    candidate: &CurrentContractCandidate,
    scenario: &ScenarioContractV3,
    platform: &str,
    work_root: &std::path::Path,
) -> NormalizedScenarioResult {
    let scenario_root = work_root.join("scenarios").join(&scenario.scenario_id);
    let _ = std::fs::remove_dir_all(&scenario_root);
    std::fs::create_dir_all(&scenario_root).expect("scenario root");
    let started = std::time::Instant::now();
    if let Some(capability) = &scenario.capability_requirement {
        let supported = match capability.as_str() {
            "compaction_supported" => candidate.compaction_supported(),
            "destructive_cleanup_supported" => candidate.destructive_cleanup_supported(),
            "verified_writer_ownership_loss" => true,
            _ => {
                return base_result(
                    candidate,
                    scenario,
                    platform,
                    started.elapsed().as_millis(),
                    ScenarioExecutionStatus::Failed,
                    false,
                    Some(format!("unknown capability requirement {capability}")),
                    Vec::new(),
                );
            }
        };
        if !supported {
            return base_result(
                candidate,
                scenario,
                platform,
                started.elapsed().as_millis(),
                ScenarioExecutionStatus::Unsupported,
                false,
                None,
                vec![format!("capability {capability} not declared")],
            );
        }
    }
    let outcome = match scenario.scenario_id.as_str() {
        "baseline-create-open-close" => run_baseline(candidate, &scenario_root),
        "semantic-duplication" => run_duplication(candidate, &scenario_root),
        "derived-state-corruption-and-rebuild" => run_derived_rebuild(candidate, &scenario_root),
        "unknown-newer-format" => run_unknown_format(candidate, &scenario_root),
        "malformed-format-version" => run_malformed_format(candidate, &scenario_root),
        "concurrent-writer-attempt" => run_concurrent_writer(candidate, &scenario_root),
        "read-only-open-during-writer" => run_read_only_during_writer(candidate, &scenario_root),
        id if id.starts_with("append-") => run_fixture_round_trip(candidate, &scenario_root, id),
        id if id.starts_with("stale-") => run_stale_precondition(candidate, &scenario_root, id),
        id if id.contains("corruption") || id.contains("malformed") => {
            run_negative_corruption(candidate, &scenario_root, id)
        }
        "interrupted-authoritative-review-transition" => {
            run_interrupted_transition(candidate, &scenario_root, "review")
        }
        "interrupted-authoritative-reuse-transition" => {
            run_interrupted_transition(candidate, &scenario_root, "reuse")
        }
        "writer-crash-and-takeover" => run_writer_takeover(candidate, &scenario_root),
        "interrupted-compaction" => {
            run_capability_interrupt(candidate, &scenario_root, "compaction")
        }
        "interrupted-cleanup" => run_capability_interrupt(candidate, &scenario_root, "cleanup"),
        _ => run_fixture_round_trip(candidate, &scenario_root, &scenario.scenario_id),
    };
    let (status, oracle_compare, failure_code, limitations) = match outcome {
        Ok(status) => (
            status,
            status == ScenarioExecutionStatus::Passed,
            None,
            Vec::new(),
        ),
        Err((status, code)) => (status, false, Some(code), Vec::new()),
    };
    let mut result = base_result(
        candidate,
        scenario,
        platform,
        started.elapsed().as_millis(),
        status,
        oracle_compare,
        failure_code,
        limitations,
    );
    if scenario.expected_recovery_class != ExpectedRecoveryClass::None {
        result.recovery_class = recovery_class_label(scenario.expected_recovery_class);
    }
    if scenario.expected_open_state != ExpectedOpenState::Normal {
        result.open_state = open_state_label(scenario.expected_open_state);
    }
    result
}

#[allow(clippy::too_many_arguments)]
fn base_result(
    candidate: &CurrentContractCandidate,
    scenario: &ScenarioContractV3,
    platform: &str,
    elapsed_ms: u128,
    status: ScenarioExecutionStatus,
    oracle_compare: bool,
    failure_code: Option<String>,
    limitations: Vec<String>,
) -> NormalizedScenarioResult {
    NormalizedScenarioResult {
        scenario_id: scenario.scenario_id.clone(),
        scenario_version: scenario.scenario_version,
        candidate_id: candidate.candidate_id().to_owned(),
        candidate_version: candidate.candidate_version().to_owned(),
        platform: platform.to_owned(),
        fixture_scale: fixture_scale_label(MeasurementFixtureScale::Small),
        status,
        oracle_compare,
        recovery_class: recovery_class_label(scenario.expected_recovery_class),
        open_state: open_state_label(scenario.expected_open_state),
        failure_code,
        elapsed_ms,
        correctness_disqualification: None,
        limitations,
    }
}

fn run_baseline(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
) -> Result<ScenarioExecutionStatus, (ScenarioExecutionStatus, String)> {
    let target = candidate_for_root(candidate, root)?;
    let state = fixture_state_for_scale(MeasurementFixtureScale::Small);
    let (session_id, _) = target.create_session(&state).map_err(err)?;
    let writer = target.open_writable(&session_id).map_err(err)?;
    target.close(writer).map_err(err)?;
    let reopened = target.open_read_only(&session_id).map_err(err)?;
    if !target.oracle_compare(&state, &reopened) {
        return Err((
            ScenarioExecutionStatus::Failed,
            "failed_oracle_compare".to_owned(),
        ));
    }
    target.close(reopened).map_err(err)?;
    Ok(ScenarioExecutionStatus::Passed)
}

fn run_fixture_round_trip(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
    scenario_id: &str,
) -> Result<ScenarioExecutionStatus, (ScenarioExecutionStatus, String)> {
    let adapter = candidate_for_root(candidate, root)?;
    let state = scenario_fixture_state(scenario_id);
    let (session_id, _) = adapter.create_session(&state).map_err(err)?;
    let reopened = adapter.open_read_only(&session_id).map_err(err)?;
    if !adapter.oracle_compare(&state, &reopened) {
        return Err((
            ScenarioExecutionStatus::Failed,
            "failed_oracle_compare".to_owned(),
        ));
    }
    adapter.close(reopened).map_err(err)?;
    Ok(ScenarioExecutionStatus::Passed)
}

fn run_duplication(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
) -> Result<ScenarioExecutionStatus, (ScenarioExecutionStatus, String)> {
    let adapter = candidate_for_root(candidate, root)?;
    let state = scenario_fixture_state("semantic-duplication");
    let (session_id, _) = adapter.create_session(&state).map_err(err)?;
    let mut writer = adapter.open_writable(&session_id).map_err(err)?;
    let duplicate_id = "session:current-contract:duplicate-medium";
    let _duplicate = adapter.duplicate(&mut writer, duplicate_id).map_err(err)?;
    adapter.close(writer).map_err(err)?;
    let reopened = adapter.open_read_only(duplicate_id).map_err(err)?;
    adapter.close(reopened).map_err(err)?;
    Ok(ScenarioExecutionStatus::Passed)
}

fn run_derived_rebuild(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
) -> Result<ScenarioExecutionStatus, (ScenarioExecutionStatus, String)> {
    let adapter = candidate_for_root(candidate, root)?;
    let state = fixture_state_for_scale(MeasurementFixtureScale::Small);
    let (session_id, _) = adapter.create_session(&state).map_err(err)?;
    let mut writer = adapter.open_writable(&session_id).map_err(err)?;
    adapter
        .tamper_derived_projection_for_test(&mut writer)
        .map_err(err)?;
    adapter.close(writer).map_err(err)?;
    let reopened = adapter.open_read_only(&session_id).map_err(err)?;
    if !adapter.oracle_compare(&state, &reopened) {
        adapter.close(reopened).map_err(err)?;
        return Err((
            ScenarioExecutionStatus::Failed,
            "derived-rebuild-oracle-failed".to_owned(),
        ));
    }
    adapter.close(reopened).map_err(err)?;
    Ok(ScenarioExecutionStatus::Passed)
}

fn run_malformed_format(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
) -> Result<ScenarioExecutionStatus, (ScenarioExecutionStatus, String)> {
    let adapter = candidate_for_root(candidate, root)?;
    let state = fixture_state_for_scale(MeasurementFixtureScale::Small);
    let (session_id, _) = adapter.create_session(&state).map_err(err)?;
    let mut writer = adapter.open_writable(&session_id).map_err(err)?;
    set_format_version(&adapter, &mut writer, 0)?;
    adapter.close(writer).map_err(err)?;
    match adapter.open_read_only(&session_id) {
        Err(_) => Ok(ScenarioExecutionStatus::Passed),
        Ok(handle) => {
            adapter.close(handle).map_err(err)?;
            Err((
                ScenarioExecutionStatus::Failed,
                "corruption-not-rejected".to_owned(),
            ))
        }
    }
}

fn run_unknown_format(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
) -> Result<ScenarioExecutionStatus, (ScenarioExecutionStatus, String)> {
    let adapter = candidate_for_root(candidate, root)?;
    let state = fixture_state_for_scale(MeasurementFixtureScale::Small);
    let (session_id, _) = adapter.create_session(&state).map_err(err)?;
    let mut writer = adapter.open_writable(&session_id).map_err(err)?;
    set_format_version(&adapter, &mut writer, 2)?;
    adapter.close(writer).map_err(err)?;
    match adapter.open_writable(&session_id) {
        Err(code) if code == "unsupported-newer-format" => Ok(ScenarioExecutionStatus::Passed),
        Ok(_) => Err((
            ScenarioExecutionStatus::Failed,
            "unknown_newer_format_writable".to_owned(),
        )),
        Err(code) => Err((ScenarioExecutionStatus::Failed, code)),
    }
}

fn run_concurrent_writer(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
) -> Result<ScenarioExecutionStatus, (ScenarioExecutionStatus, String)> {
    let adapter_a = candidate_for_root(candidate, root)?;
    let adapter_b = candidate_for_root(candidate, root)?;
    let state = fixture_state_for_scale(MeasurementFixtureScale::Small);
    let (session_id, _) = adapter_a.create_session(&state).map_err(err)?;
    let writer = adapter_a.open_writable(&session_id).map_err(err)?;
    match adapter_b.open_writable(&session_id) {
        Err(code) if code == "writer-already-open" => {
            adapter_a.close(writer).map_err(err)?;
            Ok(ScenarioExecutionStatus::Passed)
        }
        Ok(_) => Err((
            ScenarioExecutionStatus::Failed,
            "concurrent-writer-not-rejected".to_owned(),
        )),
        Err(code) => Err((ScenarioExecutionStatus::Failed, code)),
    }
}

fn run_read_only_during_writer(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
) -> Result<ScenarioExecutionStatus, (ScenarioExecutionStatus, String)> {
    let adapter_a = candidate_for_root(candidate, root)?;
    let adapter_b = candidate_for_root(candidate, root)?;
    let state = fixture_state_for_scale(MeasurementFixtureScale::Small);
    let (session_id, _) = adapter_a.create_session(&state).map_err(err)?;
    let writer = adapter_a.open_writable(&session_id).map_err(err)?;
    let reader = adapter_b.open_read_only(&session_id).map_err(err)?;
    adapter_a.close(writer).map_err(err)?;
    adapter_b.close(reader).map_err(err)?;
    Ok(ScenarioExecutionStatus::Passed)
}

fn run_stale_precondition(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
    scenario_id: &str,
) -> Result<ScenarioExecutionStatus, (ScenarioExecutionStatus, String)> {
    let adapter = candidate_for_root(candidate, root)?;
    let state = fixture_state_for_scale(MeasurementFixtureScale::Small);
    let (session_id, _) = adapter.create_session(&state).map_err(err)?;
    let mut writer = adapter.open_writable(&session_id).map_err(err)?;
    let next = updated_writer_token_state(state.clone(), "writer:stale-test");
    let baseline = adapter.authoritative_preconditions(&writer).map_err(err)?;
    let stale = stale_preconditions(candidate.kind(), scenario_id, &baseline);
    let stale_result = adapter.apply_transition_with_preconditions(&mut writer, &stale, &next);
    let expected = expected_stale_code(candidate.kind(), scenario_id);
    match stale_result {
        Err(code) if code == expected => {
            adapter.close(writer).map_err(err)?;
            Ok(ScenarioExecutionStatus::Passed)
        }
        Err(code) => Err((ScenarioExecutionStatus::Failed, code)),
        Ok(()) => Err((
            ScenarioExecutionStatus::Failed,
            format!("expected {expected}"),
        )),
    }
}

fn run_negative_corruption(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
    scenario_id: &str,
) -> Result<ScenarioExecutionStatus, (ScenarioExecutionStatus, String)> {
    let adapter = candidate_for_root(candidate, root)?;
    let state = fixture_state_for_scale(MeasurementFixtureScale::Small);
    let (session_id, _) = adapter.create_session(&state).map_err(err)?;
    tamper_for_scenario(candidate, root, &session_id, scenario_id)?;
    match adapter.open_read_only(&session_id) {
        Err(_) => Ok(ScenarioExecutionStatus::Passed),
        Ok(handle) => {
            adapter.close(handle).map_err(err)?;
            Err((
                ScenarioExecutionStatus::Failed,
                "corruption-not-rejected".to_owned(),
            ))
        }
    }
}

fn run_interrupted_transition(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
    transition_kind: &str,
) -> Result<ScenarioExecutionStatus, (ScenarioExecutionStatus, String)> {
    let adapter = candidate_for_root(candidate, root)?;
    let (precursor, next) = match transition_kind {
        "review" => {
            measurement_transition_states("append_review_decision", MeasurementFixtureScale::Small)
                .ok_or_else(|| err("missing review transition".to_owned()))?
        }
        "reuse" => measurement_transition_states(
            "append_reusable_revocation",
            MeasurementFixtureScale::Small,
        )
        .ok_or_else(|| err("missing reuse transition".to_owned()))?,
        other => return Err(err(format!("unknown interrupted transition {other}"))),
    };
    let (session_id, _) = adapter.create_session(&precursor).map_err(err)?;
    spawn_child_interrupt(candidate, root, &session_id, &next)?;
    let reopened = adapter.open_read_only(&session_id).map_err(err)?;
    if !adapter.oracle_compare(&precursor, &reopened) {
        adapter.close(reopened).map_err(err)?;
        return Err((
            ScenarioExecutionStatus::Failed,
            "partial-authority-exposed".to_owned(),
        ));
    }
    adapter.close(reopened).map_err(err)?;
    Ok(ScenarioExecutionStatus::Passed)
}

fn run_writer_takeover(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
) -> Result<ScenarioExecutionStatus, (ScenarioExecutionStatus, String)> {
    let adapter = candidate_for_root(candidate, root)?;
    let state = fixture_state_for_scale(MeasurementFixtureScale::Small);
    let (session_id, _) = adapter.create_session(&state).map_err(err)?;
    let ready = root.join("child.ready");
    let release = root.join("child.release");
    let _ = std::fs::remove_file(&ready);
    let _ = std::fs::remove_file(&release);
    let mut child = Command::new(worker_binary())
        .arg("child-hold-writer")
        .env("VOXPROOF_CHILD_ROOT", root)
        .env("VOXPROOF_CHILD_SESSION_ID", &session_id)
        .env("VOXPROOF_CHILD_READY", &ready)
        .env("VOXPROOF_CHILD_RELEASE", &release)
        .env(
            "VOXPROOF_CHILD_KIND",
            match candidate.kind() {
                CurrentContractCandidateKind::Append => "append",
                CurrentContractCandidateKind::Sqlite => "sqlite",
            },
        )
        .spawn()
        .map_err(|error| err(error.to_string()))?;
    let deadline = Instant::now() + Duration::from_secs(30);
    while !ready.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    if !ready.exists() {
        return Err(err("child-writer-not-ready".to_owned()));
    }
    match adapter.open_writable(&session_id) {
        Err(code) if code == "writer-already-open" => {}
        Ok(_) => return Err(err("concurrent-writer-not-rejected".to_owned())),
        Err(code) => return Err(err(code)),
    }
    std::fs::write(&release, b"abort").map_err(|error| err(error.to_string()))?;
    assert!(!child
        .wait()
        .map_err(|error| err(error.to_string()))?
        .success());
    let takeover = adapter.open_writable(&session_id).map_err(err)?;
    adapter.close(takeover).map_err(err)?;
    let reopened = adapter.open_read_only(&session_id).map_err(err)?;
    if !adapter.oracle_compare(&state, &reopened) {
        adapter.close(reopened).map_err(err)?;
        return Err((
            ScenarioExecutionStatus::Failed,
            "takeover-oracle-failed".to_owned(),
        ));
    }
    adapter.close(reopened).map_err(err)?;
    Ok(ScenarioExecutionStatus::Passed)
}

fn run_capability_interrupt(
    candidate: &CurrentContractCandidate,
    _root: &std::path::Path,
    capability: &str,
) -> Result<ScenarioExecutionStatus, (ScenarioExecutionStatus, String)> {
    let supported = match capability {
        "compaction" => candidate.compaction_supported(),
        "cleanup" => candidate.destructive_cleanup_supported(),
        _ => false,
    };
    if supported {
        Err((
            ScenarioExecutionStatus::Failed,
            format!("capability {capability} declared but not executable"),
        ))
    } else {
        Ok(ScenarioExecutionStatus::Unsupported)
    }
}

fn candidate_for_root(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
) -> Result<CurrentContractCandidate, (ScenarioExecutionStatus, String)> {
    match candidate.kind() {
        super::candidate::CurrentContractCandidateKind::Append => {
            CurrentContractCandidate::append(root).map_err(err)
        }
        super::candidate::CurrentContractCandidateKind::Sqlite => {
            CurrentContractCandidate::sqlite(root).map_err(err)
        }
    }
}

fn err(code: String) -> (ScenarioExecutionStatus, String) {
    (ScenarioExecutionStatus::Failed, code)
}

fn set_format_version(
    candidate: &CurrentContractCandidate,
    writer: &mut super::candidate::OpenedCandidateSession,
    version: u32,
) -> Result<(), (ScenarioExecutionStatus, String)> {
    match (candidate, writer) {
        (
            CurrentContractCandidate::Append { adapter, .. },
            super::candidate::OpenedCandidateSession::Append(handle),
        ) => adapter
            .set_format_version_for_test(handle, version)
            .map_err(|error| err(error.code.to_owned())),
        (
            CurrentContractCandidate::Sqlite { adapter, .. },
            super::candidate::OpenedCandidateSession::Sqlite(handle),
        ) => adapter
            .set_format_version_for_test(handle, version)
            .map_err(|error| err(error.code.to_owned())),
        _ => Err(err("candidate-handle-mismatch".to_owned())),
    }
}

fn tamper_for_scenario(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
    session_id: &str,
    scenario_id: &str,
) -> Result<(), (ScenarioExecutionStatus, String)> {
    let scoped = candidate_for_root(candidate, root)?;
    let mut writer = scoped.open_writable(session_id).map_err(err)?;
    match (&scoped, &mut writer, scenario_id) {
        (
            CurrentContractCandidate::Sqlite { adapter, .. },
            super::candidate::OpenedCandidateSession::Sqlite(handle),
            "canonical-reference-corruption"
            | "review-ledger-order-corruption"
            | "reuse-governance-order-corruption"
            | "source-locator-corruption",
        ) => {
            adapter
                .tamper_canonical_provenance_for_test(handle)
                .map_err(|error| err(error.code.to_owned()))?;
        }
        (
            CurrentContractCandidate::Sqlite { adapter, .. },
            super::candidate::OpenedCandidateSession::Sqlite(handle),
            "derived-state-corruption-and-rebuild",
        ) => {
            adapter
                .tamper_derived_cache_for_test(handle, "not-a-derived-projection")
                .map_err(|error| err(error.code.to_owned()))?;
        }
        (
            CurrentContractCandidate::Append { adapter, .. },
            super::candidate::OpenedCandidateSession::Append(handle),
            _,
        ) => {
            adapter
                .tamper_committed_state_for_test(handle, "writer:promoted", "writer:tampered")
                .map_err(|error| err(error.code.to_owned()))?;
        }
        _ => {}
    }
    scoped.close(writer).map_err(err)?;
    Ok(())
}

fn spawn_child_interrupt(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
    session_id: &str,
    next_state: &super::super::model::CurrentContractState,
) -> Result<(), (ScenarioExecutionStatus, String)> {
    let ready = root.join("interrupt.ready");
    let release = root.join("interrupt.release");
    let _ = std::fs::remove_file(&ready);
    let _ = std::fs::remove_file(&release);
    let encoded = serde_json::to_string(next_state).map_err(|error| err(error.to_string()))?;
    let mut child = Command::new(worker_binary())
        .arg("child-interrupt-transition")
        .env("VOXPROOF_CHILD_ROOT", root)
        .env("VOXPROOF_CHILD_SESSION_ID", session_id)
        .env("VOXPROOF_CHILD_READY", &ready)
        .env("VOXPROOF_CHILD_RELEASE", &release)
        .env("VOXPROOF_CHILD_NEXT_STATE", encoded)
        .env(
            "VOXPROOF_CHILD_KIND",
            match candidate.kind() {
                CurrentContractCandidateKind::Append => "append",
                CurrentContractCandidateKind::Sqlite => "sqlite",
            },
        )
        .spawn()
        .map_err(|error| err(error.to_string()))?;
    let deadline = Instant::now() + Duration::from_secs(30);
    while !ready.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    if !ready.exists() {
        return Err(err("child-interrupt-not-ready".to_owned()));
    }
    std::fs::write(&release, b"abort").map_err(|error| err(error.to_string()))?;
    assert!(!child
        .wait()
        .map_err(|error| err(error.to_string()))?
        .success());
    Ok(())
}

fn stale_preconditions(
    kind: CurrentContractCandidateKind,
    scenario_id: &str,
    baseline: &super::super::CurrentContractPreconditions,
) -> super::super::CurrentContractPreconditions {
    match (kind, scenario_id) {
        (_, "stale-review-ledger-command") => super::super::CurrentContractPreconditions {
            review_ledger_head: baseline.review_ledger_head + 1,
            ..baseline.clone()
        },
        (_, "stale-reuse-governance-command") => super::super::CurrentContractPreconditions {
            reuse_governance_head: baseline.reuse_governance_head + 1,
            ..baseline.clone()
        },
        (_, "stale-analysis-attachment-or-selection") => {
            super::super::CurrentContractPreconditions {
                active_analysis_snapshot_identity: "analysis:stale".to_owned(),
                ..baseline.clone()
            }
        }
        (CurrentContractCandidateKind::Append, _) => super::super::CurrentContractPreconditions {
            expected_generation: 0,
            ..baseline.clone()
        },
        (_, _) => super::super::CurrentContractPreconditions {
            expected_generation: 0,
            ..baseline.clone()
        },
    }
}

fn expected_stale_code(kind: CurrentContractCandidateKind, scenario_id: &str) -> &'static str {
    match kind {
        CurrentContractCandidateKind::Sqlite => match scenario_id {
            "stale-review-ledger-command" => "stale-review-ledger-precondition",
            "stale-reuse-governance-command" => "stale-reuse-governance-precondition",
            "stale-analysis-attachment-or-selection" => "stale-analysis-selection-precondition",
            _ => "stale-generation-precondition",
        },
        CurrentContractCandidateKind::Append => "stale-append-precondition",
    }
}

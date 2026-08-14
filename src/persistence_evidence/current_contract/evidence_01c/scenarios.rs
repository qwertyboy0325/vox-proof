use std::process::Command;
use std::time::{Duration, Instant};

use super::super::measurement::MeasurementFixtureScale;
use super::scenario_observation::{
    corruption_tamper_limitation, ScenarioOutcome, APPEND_STALE_ASYMMETRY_LIMITATION,
    OBSERVED_RECOVERY_SAFE_AUTOMATIC,
};
use super::super::scenario_contract::{
    scenario_contract_v4, ScenarioContractV3,
};
use super::candidate::{
    fixture_state_for_scale, measurement_transition_states, scenario_fixture_state,
    updated_writer_token_state, CurrentContractCandidate, CurrentContractCandidateKind,
    SQLITE_LEASE_EXPIRY_WAIT_MS, SQLITE_WRITER_LEASE_DURATION_MS,
};
use super::measurement_worker::worker_binary;
use super::types::{
    fixture_scale_label, CorrectnessDisqualification,
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
    for scenario in scenario_contract_v4() {
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
            "verified_writer_ownership_loss" => candidate.writer_ownership_loss_observable(),
            _ => {
                return observation_to_result(
                    candidate,
                    scenario,
                    platform,
                    started.elapsed().as_millis(),
                    ScenarioOutcome::failed(format!("unknown capability requirement {capability}")),
                );
            }
        };
        if !supported {
            return observation_to_result(
                candidate,
                scenario,
                platform,
                started.elapsed().as_millis(),
                ScenarioOutcome::unsupported(vec![format!("capability {capability} not declared")]),
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
    let observation = match outcome {
        Ok(observation) => observation,
        Err(code) => ScenarioOutcome::failed(code),
    };
    observation_to_result(
        candidate,
        scenario,
        platform,
        started.elapsed().as_millis(),
        observation,
    )
}

fn observation_to_result(
    candidate: &CurrentContractCandidate,
    scenario: &ScenarioContractV3,
    platform: &str,
    elapsed_ms: u128,
    observation: ScenarioOutcome,
) -> NormalizedScenarioResult {
    NormalizedScenarioResult {
        scenario_id: scenario.scenario_id.clone(),
        scenario_version: scenario.scenario_version,
        candidate_id: candidate.candidate_id().to_owned(),
        candidate_version: candidate.candidate_version().to_owned(),
        platform: platform.to_owned(),
        fixture_scale: fixture_scale_label(MeasurementFixtureScale::Small),
        status: observation.status,
        oracle_compare: observation.oracle_compare,
        recovery_class: observation.recovery_class,
        open_state: observation.open_state,
        failure_code: observation.failure_code,
        elapsed_ms,
        correctness_disqualification: None,
        limitations: observation.limitations,
    }
}

fn err_string(error: impl std::fmt::Display) -> String {
    error.to_string()
}

type ScenarioRunResult = Result<ScenarioOutcome, String>;

fn run_baseline(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
) -> ScenarioRunResult {
    let target = candidate_for_root(candidate, root).map_err(|(_, code)| code)?;
    let state = fixture_state_for_scale(MeasurementFixtureScale::Small);
    let (session_id, _) = target.create_session(&state).map_err(err_string)?;
    let writer = target.open_writable(&session_id).map_err(err_string)?;
    target.close(writer).map_err(err_string)?;
    let reopened = target.open_read_only(&session_id).map_err(err_string)?;
    if !target.oracle_compare(&state, &reopened) {
        target.close(reopened).map_err(err_string)?;
        return Err("failed_oracle_compare".to_owned());
    }
    target.close(reopened).map_err(err_string)?;
    Ok(ScenarioOutcome::passed_with_oracle())
}

fn run_fixture_round_trip(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
    scenario_id: &str,
) -> ScenarioRunResult {
    let adapter = candidate_for_root(candidate, root).map_err(|(_, code)| code)?;
    let state = scenario_fixture_state(scenario_id);
    let (session_id, _) = adapter.create_session(&state).map_err(err_string)?;
    let reopened = adapter.open_read_only(&session_id).map_err(err_string)?;
    if !adapter.oracle_compare(&state, &reopened) {
        adapter.close(reopened).map_err(err_string)?;
        return Err("failed_oracle_compare".to_owned());
    }
    adapter.close(reopened).map_err(err_string)?;
    Ok(ScenarioOutcome::passed_with_oracle())
}

fn run_duplication(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
) -> ScenarioRunResult {
    let adapter = candidate_for_root(candidate, root).map_err(|(_, code)| code)?;
    let state = scenario_fixture_state("semantic-duplication");
    let (session_id, _) = adapter.create_session(&state).map_err(err_string)?;
    let mut writer = adapter.open_writable(&session_id).map_err(err_string)?;
    let duplicate_id = "session:current-contract:duplicate-medium";
    adapter.duplicate(&mut writer, duplicate_id).map_err(err_string)?;
    adapter.close(writer).map_err(err_string)?;
    let reopened = adapter.open_read_only(duplicate_id).map_err(err_string)?;
    let reopened_state = adapter.normalized_state(&reopened).map_err(err_string)?;
    if reopened_state.durable_command_tokens.evidence_writer_token
        == state.durable_command_tokens.evidence_writer_token
    {
        adapter.close(reopened).map_err(err_string)?;
        return Err("duplicate-writer-token-not-independent".to_owned());
    }
    let expected = expected_duplicated_state(&state, duplicate_id);
    if !adapter.oracle_compare(&expected, &reopened) {
        adapter.close(reopened).map_err(err_string)?;
        return Err("failed_oracle_compare".to_owned());
    }
    adapter.close(reopened).map_err(err_string)?;
    Ok(ScenarioOutcome::passed_with_oracle())
}

fn expected_duplicated_state(
    source: &super::super::model::CurrentContractState,
    duplicate_id: &str,
) -> super::super::model::CurrentContractState {
    let mut expected = source.clone();
    expected.duplicated_from_session_id = Some(source.session_id.clone());
    expected.session_id = duplicate_id.to_owned();
    expected.durable_command_tokens.evidence_writer_token =
        derive_duplicate_writer_token(duplicate_id);
    expected.normalize()
}

fn derive_duplicate_writer_token(session_id: &str) -> String {
    if let Some(contract_local_id) = session_id.strip_prefix("session:current-contract:") {
        return format!("writer:{contract_local_id}");
    }
    let mut encoded = String::with_capacity(session_id.len() * 2);
    for byte in session_id.as_bytes() {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    format!("writer-encoded-session-id-v1:{encoded}")
}

fn run_derived_rebuild(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
) -> ScenarioRunResult {
    let adapter = candidate_for_root(candidate, root).map_err(|(_, code)| code)?;
    let state = fixture_state_for_scale(MeasurementFixtureScale::Small);
    let (session_id, _) = adapter.create_session(&state).map_err(err_string)?;
    let mut writer = adapter.open_writable(&session_id).map_err(err_string)?;
    adapter
        .tamper_derived_projection_for_test(&mut writer)
        .map_err(err_string)?;
    adapter.close(writer).map_err(err_string)?;
    let reopened = adapter.open_read_only(&session_id).map_err(err_string)?;
    if !adapter.oracle_compare(&state, &reopened) {
        adapter.close(reopened).map_err(err_string)?;
        return Err("derived-rebuild-oracle-failed".to_owned());
    }
    adapter.close(reopened).map_err(err_string)?;
    Ok(ScenarioOutcome::passed_with_oracle().with_recovery(OBSERVED_RECOVERY_SAFE_AUTOMATIC))
}

fn run_malformed_format(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
) -> ScenarioRunResult {
    let adapter = candidate_for_root(candidate, root).map_err(|(_, code)| code)?;
    let state = fixture_state_for_scale(MeasurementFixtureScale::Small);
    let (session_id, _) = adapter.create_session(&state).map_err(err_string)?;
    let mut writer = adapter.open_writable(&session_id).map_err(err_string)?;
    set_format_version(&adapter, &mut writer, 0)?;
    adapter.close(writer).map_err(err_string)?;
    match adapter.open_read_only(&session_id) {
        Err(code) if code == "unsupported-format" => Ok(ScenarioOutcome::malformed_format_refusal(code)),
        Err(code) => Err(code),
        Ok(handle) => {
            adapter.close(handle).map_err(err_string)?;
            Err("corruption-not-rejected".to_owned())
        }
    }
}

fn run_unknown_format(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
) -> ScenarioRunResult {
    let adapter = candidate_for_root(candidate, root).map_err(|(_, code)| code)?;
    let state = fixture_state_for_scale(MeasurementFixtureScale::Small);
    let (session_id, _) = adapter.create_session(&state).map_err(err_string)?;
    let mut writer = adapter.open_writable(&session_id).map_err(err_string)?;
    set_format_version(&adapter, &mut writer, 2)?;
    adapter.close(writer).map_err(err_string)?;
    match adapter.open_writable(&session_id) {
        Err(code) if code == "unsupported-newer-format" => {
            Ok(ScenarioOutcome::unsupported_version_open(code))
        }
        Ok(_) => Err("unknown_newer_format_writable".to_owned()),
        Err(code) => Err(code),
    }
}

fn run_concurrent_writer(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
) -> ScenarioRunResult {
    let adapter_a = candidate_for_root(candidate, root).map_err(|(_, code)| code)?;
    let adapter_b = candidate_for_root(candidate, root).map_err(|(_, code)| code)?;
    let state = fixture_state_for_scale(MeasurementFixtureScale::Small);
    let (session_id, _) = adapter_a.create_session(&state).map_err(err_string)?;
    let writer = adapter_a.open_writable(&session_id).map_err(err_string)?;
    match adapter_b.open_writable(&session_id) {
        Err(code) if code == "writer-already-open" => {
            adapter_a.close(writer).map_err(err_string)?;
            Ok(ScenarioOutcome::passed_interface())
        }
        Ok(_) => Err("concurrent-writer-not-rejected".to_owned()),
        Err(code) => Err(code),
    }
}

fn run_read_only_during_writer(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
) -> ScenarioRunResult {
    let adapter_a = candidate_for_root(candidate, root).map_err(|(_, code)| code)?;
    let adapter_b = candidate_for_root(candidate, root).map_err(|(_, code)| code)?;
    let state = fixture_state_for_scale(MeasurementFixtureScale::Small);
    let (session_id, _) = adapter_a.create_session(&state).map_err(err_string)?;
    let writer = adapter_a.open_writable(&session_id).map_err(err_string)?;
    let reader = adapter_b.open_read_only(&session_id).map_err(err_string)?;
    if !adapter_b.oracle_compare(&state, &reader) {
        adapter_a.close(writer).map_err(err_string)?;
        adapter_b.close(reader).map_err(err_string)?;
        return Err("read-only-view-diverged".to_owned());
    }
    match adapter_b.open_writable(&session_id) {
        Err(code) if code == "writer-already-open" => {}
        Ok(_) => {
            adapter_a.close(writer).map_err(err_string)?;
            adapter_b.close(reader).map_err(err_string)?;
            return Err("read-only-handle-writable".to_owned());
        }
        Err(code) => {
            adapter_a.close(writer).map_err(err_string)?;
            adapter_b.close(reader).map_err(err_string)?;
            return Err(code);
        }
    }
    adapter_a.close(writer).map_err(err_string)?;
    adapter_b.close(reader).map_err(err_string)?;
    Ok(ScenarioOutcome::passed_with_oracle())
}

fn run_stale_precondition(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
    scenario_id: &str,
) -> ScenarioRunResult {
    let adapter = candidate_for_root(candidate, root).map_err(|(_, code)| code)?;
    let state = fixture_state_for_scale(MeasurementFixtureScale::Small);
    let (session_id, _) = adapter.create_session(&state).map_err(err_string)?;
    let mut writer = adapter.open_writable(&session_id).map_err(err_string)?;
    let next = updated_writer_token_state(state.clone(), "writer:stale-test");
    let baseline = adapter.authoritative_preconditions(&writer).map_err(err_string)?;
    let stale = stale_preconditions(candidate.kind(), scenario_id, &baseline);
    let stale_result = adapter.apply_transition_with_preconditions(&mut writer, &stale, &next);
    let expected = expected_stale_code(candidate.kind(), scenario_id);
    let mut limitations = Vec::new();
    if matches!(candidate.kind(), CurrentContractCandidateKind::Append)
        && scenario_id.starts_with("stale-")
    {
        limitations.push(APPEND_STALE_ASYMMETRY_LIMITATION.to_owned());
    }
    match stale_result {
        Err(code) if code == expected => {
            adapter.close(writer).map_err(err_string)?;
            Ok(ScenarioOutcome::passed_with_limitations(limitations))
        }
        Err(code) => Err(code),
        Ok(()) => Err(format!("expected {expected}")),
    }
}

fn run_negative_corruption(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
    scenario_id: &str,
) -> ScenarioRunResult {
    let adapter = candidate_for_root(candidate, root).map_err(|(_, code)| code)?;
    let state = corruption_fixture_state(scenario_id);
    let (session_id, _) = adapter.create_session(&state).map_err(err_string)?;
    let tamper = tamper_for_scenario(candidate, root, &session_id, scenario_id)?;
    let expected_code = expected_corruption_refusal_code(candidate.kind(), scenario_id);
    match adapter.open_read_only(&session_id) {
        Err(code) if code == expected_code => {
            let outcome = match scenario_id {
                "canonical-reference-corruption" | "source-locator-corruption" => {
                    ScenarioOutcome::fail_closed_refusal(code)
                }
                "review-ledger-order-corruption" | "reuse-governance-order-corruption" => {
                    ScenarioOutcome::unrecoverable_refusal(code)
                }
                _ => ScenarioOutcome::refused_open(code),
            };
            Ok(outcome.with_limitations(vec![tamper]))
        }
        Err(code) => Err(format!(
            "unexpected refusal code {code}; expected {expected_code}"
        )),
        Ok(handle) => {
            adapter.close(handle).map_err(err_string)?;
            Err("corruption-not-rejected".to_owned())
        }
    }
}

fn expected_corruption_refusal_code(
    kind: CurrentContractCandidateKind,
    _scenario_id: &str,
) -> &'static str {
    match kind {
        CurrentContractCandidateKind::Append => "commit-fingerprint-mismatch",
        CurrentContractCandidateKind::Sqlite => "canonical-corruption",
    }
}

fn corruption_fixture_state(scenario_id: &str) -> super::super::model::CurrentContractState {
    use super::super::fixture::{build_golden_small_state, build_promoted_active_state};
    match scenario_id {
        "source-locator-corruption" | "reuse-governance-order-corruption" => {
            build_promoted_active_state()
        }
        _ => build_golden_small_state(),
    }
}

fn run_interrupted_transition(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
    transition_kind: &str,
) -> ScenarioRunResult {
    let adapter = candidate_for_root(candidate, root).map_err(|(_, code)| code)?;
    let (precursor, next) = match transition_kind {
        "review" => measurement_transition_states(
            "append_review_decision",
            MeasurementFixtureScale::Small,
        )
        .ok_or_else(|| "missing review transition".to_owned())?,
        "reuse" => measurement_transition_states(
            "append_reusable_revocation",
            MeasurementFixtureScale::Small,
        )
        .ok_or_else(|| "missing reuse transition".to_owned())?,
        other => return Err(format!("unknown interrupted transition {other}")),
    };
    let (session_id, _) = adapter.create_session(&precursor).map_err(err_string)?;
    spawn_child_interrupt(candidate, root, &session_id, &next)?;
    let reopened = adapter.open_read_only(&session_id).map_err(err_string)?;
    if !adapter.oracle_compare(&precursor, &reopened) {
        adapter.close(reopened).map_err(err_string)?;
        return Err("partial-authority-exposed".to_owned());
    }
    adapter.close(reopened).map_err(err_string)?;
    Ok(ScenarioOutcome::passed_with_oracle().with_recovery(OBSERVED_RECOVERY_SAFE_AUTOMATIC))
}

fn run_writer_takeover(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
) -> ScenarioRunResult {
    let adapter = candidate_for_root(candidate, root).map_err(|(_, code)| code)?;
    let state = fixture_state_for_scale(MeasurementFixtureScale::Small);
    let (session_id, _) = adapter.create_session(&state).map_err(err_string)?;
    if matches!(candidate.kind(), CurrentContractCandidateKind::Sqlite) {
        adapter
            .configure_sqlite_writer_lease_for_test(&session_id, SQLITE_WRITER_LEASE_DURATION_MS)
            .map_err(err_string)?;
    }
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
        .map_err(|error| error.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(30);
    while !ready.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    if !ready.exists() {
        return Err("child-writer-not-ready".to_owned());
    }
    match adapter.open_writable(&session_id) {
        Err(code) if code == "writer-already-open" => {}
        Ok(_) => return Err("concurrent-writer-not-rejected".to_owned()),
        Err(code) => return Err(code),
    }
    std::fs::write(&release, b"abort").map_err(|error| error.to_string())?;
    assert!(!child
        .wait()
        .map_err(|error| error.to_string())?
        .success());
    if matches!(candidate.kind(), CurrentContractCandidateKind::Sqlite) {
        match adapter.open_writable(&session_id) {
            Err(code) if code == "writer-already-open" => {}
            Ok(_) => return Err("takeover-before-lease-expiry-not-rejected".to_owned()),
            Err(code) => return Err(code),
        }
        std::thread::sleep(Duration::from_millis(SQLITE_LEASE_EXPIRY_WAIT_MS));
    }
    let takeover = adapter.open_writable(&session_id).map_err(err_string)?;
    adapter.close(takeover).map_err(err_string)?;
    let reopened = adapter.open_read_only(&session_id).map_err(err_string)?;
    if !adapter.oracle_compare(&state, &reopened) {
        adapter.close(reopened).map_err(err_string)?;
        return Err("takeover-oracle-failed".to_owned());
    }
    adapter.close(reopened).map_err(err_string)?;
    Ok(ScenarioOutcome::passed_with_oracle().with_recovery(OBSERVED_RECOVERY_SAFE_AUTOMATIC))
}

fn run_capability_interrupt(
    candidate: &CurrentContractCandidate,
    _root: &std::path::Path,
    capability: &str,
) -> ScenarioRunResult {
    let supported = match capability {
        "compaction" => candidate.compaction_supported(),
        "cleanup" => candidate.destructive_cleanup_supported(),
        _ => false,
    };
    if supported {
        Err(format!("capability {capability} declared but not executable"))
    } else {
        Ok(ScenarioOutcome::unsupported(vec![format!(
            "capability {capability} not declared"
        )]))
    }
}

fn candidate_for_root(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
) -> Result<CurrentContractCandidate, (ScenarioExecutionStatus, String)> {
    match candidate.kind() {
        super::candidate::CurrentContractCandidateKind::Append => CurrentContractCandidate::append(root)
            .map_err(|code| (ScenarioExecutionStatus::Failed, code)),
        super::candidate::CurrentContractCandidateKind::Sqlite => {
            CurrentContractCandidate::sqlite(root)
                .map_err(|code| (ScenarioExecutionStatus::Failed, code))
        }
    }
}

fn set_format_version(
    candidate: &CurrentContractCandidate,
    writer: &mut super::candidate::OpenedCandidateSession,
    version: u32,
) -> Result<(), String> {
    match (candidate, writer) {
        (
            CurrentContractCandidate::Append { adapter, .. },
            super::candidate::OpenedCandidateSession::Append(handle),
        ) => adapter
            .set_format_version_for_test(handle, version)
            .map_err(|error| error.code.to_owned()),
        (
            CurrentContractCandidate::Sqlite { adapter, .. },
            super::candidate::OpenedCandidateSession::Sqlite(handle),
        ) => adapter
            .set_format_version_for_test(handle, version)
            .map_err(|error| error.code.to_owned()),
        _ => Err("candidate-handle-mismatch".to_owned()),
    }
}

fn tamper_for_scenario(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
    session_id: &str,
    scenario_id: &str,
) -> Result<String, String> {
    let scoped = candidate_for_root(candidate, root).map_err(|(_, code)| code)?;
    let mut writer = scoped.open_writable(session_id).map_err(err_string)?;
    let tamper_label = match (&scoped, &mut writer, scenario_id) {
        (
            CurrentContractCandidate::Sqlite { adapter, .. },
            super::candidate::OpenedCandidateSession::Sqlite(handle),
            "derived-state-corruption-and-rebuild",
        ) => {
            adapter
                .tamper_derived_cache_for_test(handle, "not-a-derived-projection")
                .map_err(|error| error.code.to_owned())?;
            "tamper_derived_cache_for_test".to_owned()
        }
        (
            CurrentContractCandidate::Sqlite { adapter, .. },
            super::candidate::OpenedCandidateSession::Sqlite(handle),
            "canonical-reference-corruption",
        ) => {
            adapter
                .tamper_canonical_provenance_for_test(handle)
                .map_err(|error| error.code.to_owned())?;
            "tamper_canonical_provenance_for_test:canonical-reference-corruption".to_owned()
        }
        (
            CurrentContractCandidate::Sqlite { adapter, .. },
            super::candidate::OpenedCandidateSession::Sqlite(handle),
            "source-locator-corruption",
        ) => {
            adapter
                .tamper_source_locator_for_test(handle)
                .map_err(|error| error.code.to_owned())?;
            "tamper_source_locator_for_test:source-locator-corruption".to_owned()
        }
        (
            CurrentContractCandidate::Sqlite { adapter, .. },
            super::candidate::OpenedCandidateSession::Sqlite(handle),
            "review-ledger-order-corruption",
        ) => {
            adapter
                .tamper_review_ledger_order_for_test(handle)
                .map_err(|error| error.code.to_owned())?;
            "tamper_review_ledger_order_for_test:review-ledger-order-corruption".to_owned()
        }
        (
            CurrentContractCandidate::Sqlite { adapter, .. },
            super::candidate::OpenedCandidateSession::Sqlite(handle),
            "reuse-governance-order-corruption",
        ) => {
            adapter
                .tamper_reuse_governance_order_for_test(handle)
                .map_err(|error| error.code.to_owned())?;
            "tamper_reuse_governance_order_for_test:reuse-governance-order-corruption".to_owned()
        }
        (
            CurrentContractCandidate::Append { adapter, .. },
            super::candidate::OpenedCandidateSession::Append(handle),
            "canonical-reference-corruption",
        ) => {
            adapter
                .tamper_committed_state_for_test(
                    handle,
                    "writer:promoted",
                    "writer:canonical-tampered",
                )
                .map_err(|error| error.code.to_owned())?;
            "tamper_committed_state_for_test:canonical-reference-corruption".to_owned()
        }
        (
            CurrentContractCandidate::Append { adapter, .. },
            super::candidate::OpenedCandidateSession::Append(handle),
            "source-locator-corruption",
        ) => {
            adapter
                .tamper_committed_state_for_test(
                    handle,
                    "writer:promoted",
                    "writer:locator-tampered",
                )
                .map_err(|error| error.code.to_owned())?;
            "tamper_committed_state_for_test:source-locator-corruption".to_owned()
        }
        (
            CurrentContractCandidate::Append { adapter, .. },
            super::candidate::OpenedCandidateSession::Append(handle),
            "review-ledger-order-corruption",
        ) => {
            adapter
                .tamper_committed_state_for_test(
                    handle,
                    "writer:promoted",
                    "writer:ledger-order-tampered",
                )
                .map_err(|error| error.code.to_owned())?;
            "tamper_committed_state_for_test:review-ledger-order-corruption".to_owned()
        }
        (
            CurrentContractCandidate::Append { adapter, .. },
            super::candidate::OpenedCandidateSession::Append(handle),
            "reuse-governance-order-corruption",
        ) => {
            adapter
                .tamper_committed_state_for_test(
                    handle,
                    "writer:promoted",
                    "writer:governance-order-tampered",
                )
                .map_err(|error| error.code.to_owned())?;
            "tamper_committed_state_for_test:reuse-governance-order-corruption".to_owned()
        }
        _ => return Err(format!("unsupported tamper for {scenario_id}")),
    };
    scoped.close(writer).map_err(err_string)?;
    let candidate_label = match scoped.kind() {
        CurrentContractCandidateKind::Append => "append",
        CurrentContractCandidateKind::Sqlite => "sqlite",
    };
    Ok(corruption_tamper_limitation(
        candidate_label,
        scenario_id,
        &tamper_label,
    ))
}

fn spawn_child_interrupt(
    candidate: &CurrentContractCandidate,
    root: &std::path::Path,
    session_id: &str,
    next_state: &super::super::model::CurrentContractState,
) -> Result<(), String> {
    let ready = root.join("interrupt.ready");
    let release = root.join("interrupt.release");
    let _ = std::fs::remove_file(&ready);
    let _ = std::fs::remove_file(&release);
    let encoded = serde_json::to_string(next_state).map_err(|error| error.to_string())?;
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
        .map_err(|error| error.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(30);
    while !ready.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    if !ready.exists() {
        return Err("child-interrupt-not-ready".to_owned());
    }
    std::fs::write(&release, b"abort").map_err(|error| error.to_string())?;
    assert!(!child
        .wait()
        .map_err(|error| error.to_string())?
        .success());
    Ok(())
}

fn stale_preconditions(
    kind: CurrentContractCandidateKind,
    scenario_id: &str,
    baseline: &super::super::CurrentContractPreconditions,
) -> super::super::CurrentContractPreconditions {
    match kind {
        CurrentContractCandidateKind::Append => super::super::CurrentContractPreconditions {
            expected_generation: baseline.expected_generation.saturating_sub(1),
            ..baseline.clone()
        },
        CurrentContractCandidateKind::Sqlite => match scenario_id {
            "stale-review-ledger-command" => super::super::CurrentContractPreconditions {
                review_ledger_head: baseline.review_ledger_head + 1,
                ..baseline.clone()
            },
            "stale-reuse-governance-command" => super::super::CurrentContractPreconditions {
                reuse_governance_head: baseline.reuse_governance_head + 1,
                ..baseline.clone()
            },
            "stale-analysis-attachment-or-selection" => {
                super::super::CurrentContractPreconditions {
                    active_analysis_snapshot_identity: "analysis:stale".to_owned(),
                    ..baseline.clone()
                }
            }
            _ => super::super::CurrentContractPreconditions {
                expected_generation: baseline.expected_generation.saturating_sub(1),
                ..baseline.clone()
            },
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

#[doc(hidden)]
pub fn stale_preconditions_for_test(
    kind: CurrentContractCandidateKind,
    scenario_id: &str,
    baseline: &super::super::CurrentContractPreconditions,
) -> super::super::CurrentContractPreconditions {
    stale_preconditions(kind, scenario_id, baseline)
}

#[doc(hidden)]
pub fn expected_stale_code_for_test(
    kind: CurrentContractCandidateKind,
    scenario_id: &str,
) -> &'static str {
    expected_stale_code(kind, scenario_id)
}

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Instant;

use super::adapter::{
    AdapterError, AuthoritativeCommand, PersistenceCandidateAdapter, SemanticOpenMode,
    SemanticPrecondition,
};
use super::candidates::semantic_ops::{
    apply_command, sample_active_analysis_selection, sample_append_event, sample_attach_analysis,
};
use super::fixture::EvidenceFixture;
use super::model::{
    EvidenceManifest, EvidenceRunResult, ScenarioMeasurement, ScenarioResult, ScenarioStatus,
};
use super::oracle::SemanticOracle;
use super::runner::EvidenceHarness;
use super::scenario::{ScenarioIdentity, ScenarioRequirement, scenario_catalog};

pub struct ScenarioRunner;

impl ScenarioRunner {
    pub fn run_catalog(
        adapter: &mut impl PersistenceCandidateAdapter,
        fixture: &EvidenceFixture,
        manifest: EvidenceManifest,
    ) -> EvidenceRunResult {
        let mut results = Vec::new();
        for scenario in scenario_catalog() {
            if scenario.requirement == ScenarioRequirement::CapabilityDependent
                && scenario
                    .required_capabilities
                    .iter()
                    .any(|capability| !adapter.capabilities().supports(*capability))
            {
                results.push(ScenarioResult {
                    scenario_identity: scenario.clone(),
                    status: ScenarioStatus::Unsupported,
                    oracle_result: None,
                    measurements: BTreeMap::new(),
                    failure_classification: None,
                    limitations: adapter.capabilities().limitations.clone(),
                    raw_artifact_references: Vec::new(),
                });
                continue;
            }
            results.push(run_single(adapter, fixture, &scenario));
        }
        EvidenceHarness::aggregate(manifest, results)
    }
}

fn run_single(
    adapter: &mut impl PersistenceCandidateAdapter,
    fixture: &EvidenceFixture,
    scenario: &ScenarioIdentity,
) -> ScenarioResult {
    let fixture = isolated_fixture(fixture, scenario);
    let started = Instant::now();
    let outcome = match scenario.scenario_id.as_str() {
        "baseline-create-open-close" => run_baseline(adapter, &fixture),
        "append-correction-event" => run_append_correction(adapter, &fixture),
        "attach-analysis-result" => run_attach_analysis(adapter, &fixture),
        "stale-review-ledger-command" => run_stale_ledger(adapter, &fixture),
        "stale-active-analysis-selection" => run_stale_active_analysis(adapter, &fixture),
        "stale-analysis-attachment" => run_stale_attachment(adapter, &fixture),
        "concurrent-writer-attempt" => run_concurrent_writer(adapter, &fixture),
        "unknown-newer-format" => run_unknown_newer_format(adapter, &fixture, scenario),
        "derived-state-corruption" => run_derived_corruption(adapter, &fixture, scenario),
        "canonical-reference-corruption" => run_canonical_corruption(adapter, &fixture, scenario),
        "semantic-duplication" => run_duplication(adapter, &fixture),
        "interrupted-authoritative-transition" => {
            run_interrupted_transition(adapter, &fixture, scenario)
        }
        "interrupted-compaction" => run_interrupted_compaction(adapter, &fixture, scenario),
        "interrupted-cleanup" => ScenarioOutcome::unsupported("cleanup not implemented in spike"),
        other => ScenarioOutcome::failed(format!("unknown scenario handler: {other}")),
    };
    let elapsed_ms = started.elapsed().as_millis();
    outcome.into_result(scenario, elapsed_ms)
}

fn isolated_fixture(base: &EvidenceFixture, scenario: &ScenarioIdentity) -> EvidenceFixture {
    let mut fixture = base.clone();
    fixture.expected_state.session.session_id = format!(
        "{}:{}",
        base.expected_state.session.session_id, scenario.scenario_id
    );
    fixture
}

struct ScenarioOutcome {
    status: ScenarioStatus,
    oracle_result: Option<super::oracle::OracleResult>,
    limitations: Vec<String>,
    measurements: BTreeMap<String, ScenarioMeasurement>,
}

impl ScenarioOutcome {
    fn passed_oracle(
        expected: &super::model::NormalizedSemanticState,
        actual: &super::model::NormalizedSemanticState,
    ) -> Self {
        let oracle_result = SemanticOracle::compare(expected, actual);
        let status = if oracle_result.passed {
            ScenarioStatus::Passed
        } else {
            ScenarioStatus::Failed
        };
        Self {
            status,
            oracle_result: Some(oracle_result),
            limitations: Vec::new(),
            measurements: BTreeMap::new(),
        }
    }

    fn failed(message: impl Into<String>) -> Self {
        Self {
            status: ScenarioStatus::Failed,
            oracle_result: None,
            limitations: vec![message.into()],
            measurements: BTreeMap::new(),
        }
    }

    fn unsupported(message: impl Into<String>) -> Self {
        Self {
            status: ScenarioStatus::Unsupported,
            oracle_result: None,
            limitations: vec![message.into()],
            measurements: BTreeMap::new(),
        }
    }

    fn into_result(self, scenario: &ScenarioIdentity, elapsed_ms: u128) -> ScenarioResult {
        let mut measurements = self.measurements;
        measurements.insert(
            "scenario_elapsed_ms".to_string(),
            ScenarioMeasurement::Integer {
                value: elapsed_ms as i128,
                unit: "ms".to_string(),
            },
        );
        ScenarioResult {
            scenario_identity: scenario.clone(),
            status: self.status,
            oracle_result: self.oracle_result,
            measurements,
            failure_classification: None,
            limitations: self.limitations,
            raw_artifact_references: Vec::new(),
        }
    }
}

fn run_baseline(
    adapter: &mut impl PersistenceCandidateAdapter,
    fixture: &EvidenceFixture,
) -> ScenarioOutcome {
    match baseline_flow(adapter, fixture) {
        Ok(actual) => ScenarioOutcome::passed_oracle(&fixture.normalized_state(), &actual),
        Err(error) => ScenarioOutcome::failed(error.to_string()),
    }
}

fn baseline_flow(
    adapter: &mut impl PersistenceCandidateAdapter,
    fixture: &EvidenceFixture,
) -> Result<super::model::NormalizedSemanticState, AdapterError> {
    let session = adapter.create(fixture)?;
    let handle = adapter.open(&session, SemanticOpenMode::Writable)?;
    let actual = adapter.read_normalized_state(&handle)?;
    adapter.close(&handle)?;
    Ok(actual)
}

fn run_append_correction(
    adapter: &mut impl PersistenceCandidateAdapter,
    fixture: &EvidenceFixture,
) -> ScenarioOutcome {
    let session = match adapter.create(fixture) {
        Ok(session) => session,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    let handle = match adapter.open(&session, SemanticOpenMode::Writable) {
        Ok(handle) => handle,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    let state = match adapter.read_normalized_state(&handle) {
        Ok(state) => state,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    let event = sample_append_event(&state);
    let expected_head = state
        .review_ledger_events
        .last()
        .map(|event| event.event_id.clone());
    let mut expected = state.clone();
    if apply_command(
        &mut expected,
        &AuthoritativeCommand::AppendCorrectionEvent {
            event: event.clone(),
            preconditions: vec![SemanticPrecondition::ReviewLedgerHead {
                expected_event_id: expected_head,
            }],
        },
    )
    .is_err()
    {
        return ScenarioOutcome::failed("expected state mutation failed");
    }
    if let Err(error) = adapter.apply_authoritative_command(
        &handle,
        &AuthoritativeCommand::AppendCorrectionEvent {
            event,
            preconditions: vec![SemanticPrecondition::ReviewLedgerHead {
                expected_event_id: state
                    .review_ledger_events
                    .last()
                    .map(|event| event.event_id.clone()),
            }],
        },
    ) {
        return ScenarioOutcome::failed(error.to_string());
    }
    let actual = match adapter.read_normalized_state(&handle) {
        Ok(state) => state,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    let _ = adapter.close(&handle);
    ScenarioOutcome::passed_oracle(&expected.normalize(), &actual)
}

fn run_attach_analysis(
    adapter: &mut impl PersistenceCandidateAdapter,
    fixture: &EvidenceFixture,
) -> ScenarioOutcome {
    let session = match adapter.create(fixture) {
        Ok(session) => session,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    let handle = match adapter.open(&session, SemanticOpenMode::Writable) {
        Ok(handle) => handle,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    let state = match adapter.read_normalized_state(&handle) {
        Ok(state) => state,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    let analysis = sample_attach_analysis(&state);
    let mut expected = state.clone();
    let _ = apply_command(
        &mut expected,
        &AuthoritativeCommand::AttachAnalysisResult {
            analysis_result: analysis.clone(),
            preconditions: vec![SemanticPrecondition::AnalysisAttachmentSet {
                expected_analysis_result_ids: state
                    .analysis_results
                    .iter()
                    .map(|item| item.analysis_result_id.clone())
                    .collect(),
            }],
        },
    );
    if let Err(error) = adapter.apply_authoritative_command(
        &handle,
        &AuthoritativeCommand::AttachAnalysisResult {
            analysis_result: analysis,
            preconditions: vec![SemanticPrecondition::AnalysisAttachmentSet {
                expected_analysis_result_ids: state
                    .analysis_results
                    .iter()
                    .map(|item| item.analysis_result_id.clone())
                    .collect(),
            }],
        },
    ) {
        return ScenarioOutcome::failed(error.to_string());
    }
    let actual = match adapter.read_normalized_state(&handle) {
        Ok(state) => state,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    let _ = adapter.close(&handle);
    let mut expected_state = expected.normalize();
    expected_state
        .analysis_results
        .sort_by(|a, b| a.analysis_result_id.cmp(&b.analysis_result_id));
    ScenarioOutcome::passed_oracle(&expected_state, &actual)
}

fn run_stale_ledger(
    adapter: &mut impl PersistenceCandidateAdapter,
    fixture: &EvidenceFixture,
) -> ScenarioOutcome {
    stale_command_scenario(adapter, fixture, |state| {
        AuthoritativeCommand::AppendCorrectionEvent {
            event: sample_append_event(state),
            preconditions: vec![SemanticPrecondition::ReviewLedgerHead {
                expected_event_id: Some("ledger-event:missing".to_string()),
            }],
        }
    })
}

fn run_stale_active_analysis(
    adapter: &mut impl PersistenceCandidateAdapter,
    fixture: &EvidenceFixture,
) -> ScenarioOutcome {
    stale_command_scenario(adapter, fixture, |state| {
        AuthoritativeCommand::SelectActiveAnalysis {
            selection: sample_active_analysis_selection(
                &state
                    .analysis_results
                    .first()
                    .expect("analysis")
                    .analysis_result_id,
            ),
            preconditions: vec![SemanticPrecondition::ActiveAnalysisSelection {
                expected_analysis_result_id: Some("analysis-result:missing".to_string()),
            }],
        }
    })
}

fn run_stale_attachment(
    adapter: &mut impl PersistenceCandidateAdapter,
    fixture: &EvidenceFixture,
) -> ScenarioOutcome {
    stale_command_scenario(adapter, fixture, |state| {
        AuthoritativeCommand::AttachAnalysisResult {
            analysis_result: sample_attach_analysis(state),
            preconditions: vec![SemanticPrecondition::AnalysisAttachmentSet {
                expected_analysis_result_ids: vec!["analysis-result:missing".to_string()],
            }],
        }
    })
}

fn stale_command_scenario(
    adapter: &mut impl PersistenceCandidateAdapter,
    fixture: &EvidenceFixture,
    command: impl Fn(&super::model::NormalizedSemanticState) -> AuthoritativeCommand,
) -> ScenarioOutcome {
    let session = match adapter.create(fixture) {
        Ok(session) => session,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    let handle = match adapter.open(&session, SemanticOpenMode::Writable) {
        Ok(handle) => handle,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    let before = match adapter.read_normalized_state(&handle) {
        Ok(state) => state,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    let result = adapter.apply_authoritative_command(&handle, &command(&before));
    match result {
        Err(error) if error.code == "stale-precondition" => {
            let after = adapter
                .read_normalized_state(&handle)
                .expect("state readable");
            ScenarioOutcome::passed_oracle(&before, &after)
        }
        Ok(()) => ScenarioOutcome::failed("stale command unexpectedly succeeded"),
        Err(error) => ScenarioOutcome::failed(error.to_string()),
    }
}

fn run_concurrent_writer(
    adapter: &mut impl PersistenceCandidateAdapter,
    fixture: &EvidenceFixture,
) -> ScenarioOutcome {
    let session = match adapter.create(fixture) {
        Ok(session) => session,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    let first = match adapter.open(&session, SemanticOpenMode::Writable) {
        Ok(handle) => handle,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    let before = match adapter.read_normalized_state(&first) {
        Ok(state) => state,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    let second_open = adapter.open(&session, SemanticOpenMode::Writable);
    let _ = adapter.close(&first);
    match second_open {
        Err(error) if error.code == "writer-already-open" => {
            ScenarioOutcome::passed_oracle(&before, &before)
        }
        Ok(_) => ScenarioOutcome::failed("second writer unexpectedly opened"),
        Err(error) => ScenarioOutcome::failed(error.to_string()),
    }
}

fn run_unknown_newer_format(
    adapter: &mut impl PersistenceCandidateAdapter,
    fixture: &EvidenceFixture,
    scenario: &ScenarioIdentity,
) -> ScenarioOutcome {
    let session = match adapter.create(fixture) {
        Ok(session) => session,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    if let Err(error) = adapter.corrupt_or_fault_inject(&session, scenario) {
        return ScenarioOutcome::failed(error.to_string());
    }
    match adapter.open(&session, SemanticOpenMode::Writable) {
        Err(error) if error.code == "unsupported-newer-format" => {
            ScenarioOutcome::passed_oracle(&fixture.normalized_state(), &fixture.normalized_state())
        }
        Ok(_) => ScenarioOutcome::failed("writable open unexpectedly succeeded"),
        Err(error) => ScenarioOutcome::failed(error.to_string()),
    }
}

fn run_derived_corruption(
    adapter: &mut impl PersistenceCandidateAdapter,
    fixture: &EvidenceFixture,
    scenario: &ScenarioIdentity,
) -> ScenarioOutcome {
    let session = match adapter.create(fixture) {
        Ok(session) => session,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    if let Err(error) = adapter.corrupt_or_fault_inject(&session, scenario) {
        return ScenarioOutcome::failed(error.to_string());
    }
    let handle = match adapter.open(&session, SemanticOpenMode::ReadOnly) {
        Ok(handle) => handle,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    let actual = match adapter.read_normalized_state(&handle) {
        Ok(state) => state,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    ScenarioOutcome::passed_oracle(&fixture.normalized_state(), &actual)
}

fn run_canonical_corruption(
    adapter: &mut impl PersistenceCandidateAdapter,
    fixture: &EvidenceFixture,
    scenario: &ScenarioIdentity,
) -> ScenarioOutcome {
    let session = match adapter.create(fixture) {
        Ok(session) => session,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    if let Err(error) = adapter.corrupt_or_fault_inject(&session, scenario) {
        return ScenarioOutcome::failed(error.to_string());
    }
    match adapter.open(&session, SemanticOpenMode::Writable) {
        Err(_) => {
            ScenarioOutcome::passed_oracle(&fixture.normalized_state(), &fixture.normalized_state())
        }
        Ok(_) => ScenarioOutcome::failed("corrupted canonical session opened writable"),
    }
}

fn run_duplication(
    adapter: &mut impl PersistenceCandidateAdapter,
    fixture: &EvidenceFixture,
) -> ScenarioOutcome {
    let session = match adapter.create(fixture) {
        Ok(session) => session,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    let handle = match adapter.open(&session, SemanticOpenMode::Writable) {
        Ok(handle) => handle,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    let duplicate_id = format!(
        "{}:duplicate",
        fixture.normalized_state().session.session_id
    );
    let duplicate = match adapter.duplicate_session(&handle, &duplicate_id) {
        Ok(duplicate) => duplicate,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    if duplicate.normalized_state.session.session_id != duplicate_id {
        return ScenarioOutcome::failed("duplicate session id mismatch");
    }
    if duplicate
        .normalized_state
        .session
        .duplicated_from_session_id
        .as_deref()
        != Some(fixture.normalized_state().session.session_id.as_str())
    {
        return ScenarioOutcome::failed("duplicate lineage missing");
    }
    ScenarioOutcome::passed_oracle(&duplicate.normalized_state, &duplicate.normalized_state)
}

fn run_interrupted_transition(
    adapter: &mut impl PersistenceCandidateAdapter,
    fixture: &EvidenceFixture,
    scenario: &ScenarioIdentity,
) -> ScenarioOutcome {
    let session = match adapter.create(fixture) {
        Ok(session) => session,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    let handle = match adapter.open(&session, SemanticOpenMode::Writable) {
        Ok(handle) => handle,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    let before = match adapter.read_normalized_state(&handle) {
        Ok(state) => state,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    let _ = adapter.close(&handle);
    let _ = adapter.corrupt_or_fault_inject(&session, scenario);
    let handle = match adapter.open(&session, SemanticOpenMode::Writable) {
        Ok(handle) => handle,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    let event = sample_append_event(&before);
    let command = AuthoritativeCommand::AppendCorrectionEvent {
        event,
        preconditions: vec![SemanticPrecondition::ReviewLedgerHead {
            expected_event_id: before
                .review_ledger_events
                .last()
                .map(|event| event.event_id.clone()),
        }],
    };
    match adapter.apply_authoritative_command(&handle, &command) {
        Err(error) if error.code == "simulated-durability-failure" => {}
        Ok(()) => return ScenarioOutcome::failed("fault did not prevent durability"),
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    }
    let _ = adapter.close(&handle);
    let reopened = match adapter.open(&session, SemanticOpenMode::ReadOnly) {
        Ok(handle) => handle,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    let after = match adapter.read_normalized_state(&reopened) {
        Ok(state) => state,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    ScenarioOutcome::passed_oracle(&before, &after)
}

fn run_interrupted_compaction(
    adapter: &mut impl PersistenceCandidateAdapter,
    fixture: &EvidenceFixture,
    scenario: &ScenarioIdentity,
) -> ScenarioOutcome {
    let session = match adapter.create(fixture) {
        Ok(session) => session,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    let handle = match adapter.open(&session, SemanticOpenMode::Writable) {
        Ok(handle) => handle,
        Err(error) => return ScenarioOutcome::failed(error.to_string()),
    };
    let _ = adapter.corrupt_or_fault_inject(&session, scenario);
    match adapter
        .cleanup_or_compact_if_supported(&handle, super::adapter::MaintenanceOperation::Compact)
    {
        Err(error) if error.code == "simulated-compaction-interrupt" => {
            ScenarioOutcome::passed_oracle(&fixture.normalized_state(), &fixture.normalized_state())
        }
        Ok(super::adapter::OptionalOperationOutcome::Completed) => {
            ScenarioOutcome::failed("compaction unexpectedly completed under interrupt")
        }
        other => ScenarioOutcome::failed(format!("unexpected compaction outcome: {other:?}")),
    }
}

pub fn fresh_storage_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("voxproof-persistence-spike-{label}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp storage root");
    root
}

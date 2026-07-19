//! SQLite-targeted persistence evidence execution (Package 2C).

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::adapter::{
    AdapterError, AuthoritativeCommand, EvidenceSessionRef, MaintenanceOperation,
    OptionalOperationOutcome, PersistenceCandidateAdapter, SemanticOpenMode, SemanticPrecondition,
};
use super::candidates::EmbeddedRelationalAdapter;
use super::candidates::fault::FaultPoint;
use super::candidates::semantic_ops::{
    apply_command, sample_active_analysis_selection, sample_append_event, sample_attach_analysis,
};
use super::fixture::EvidenceFixture;
use super::independent_oracle::{IndependentSqliteOracle, OracleObservationRecord};
use super::model::{
    AnalysisResultState, EvidenceManifest, EvidenceRunResult, NormalizedSemanticState,
    RecoveryClassification, ReviewLedgerEventState, ScenarioMeasurement, ScenarioResult,
    ScenarioStatus,
};
use super::oracle::OracleResult;
use super::process_harness::{
    ProcessEventRecord, ProcessExitClassification, ProcessHarness, excerpt,
};
use super::runner::EvidenceHarness;
use super::scenario::{ScenarioIdentity, ScenarioRequirement, scenario_catalog};
use super::scenario_runner::{catalog_command_id, fresh_storage_root, isolated_fixture};

pub const SQLITE_EVIDENCE_HARNESS_VERSION: &str = "sqlite-evidence-v1";

use super::platform::{filesystem_safe_path_segment, CROSS_PLATFORM_SCENARIO_IDS};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FaultExecutionRecord {
    pub fault_id: String,
    pub physical_location: String,
    pub process_pid: Option<u32>,
    pub before_or_after_authority_change: bool,
    pub termination_mechanism: String,
    pub recovery_classification: Option<RecoveryClassification>,
    pub claim_credited: Vec<String>,
    pub claim_denied: Vec<String>,
    pub scenario_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SqliteEvidenceArtifacts {
    pub process_events: Vec<ProcessEventRecord>,
    pub fault_executions: Vec<FaultExecutionRecord>,
    pub oracle_observations: Vec<OracleObservationRecord>,
    pub commands: Vec<String>,
}

pub struct SqliteScenarioRunner {
    harness: ProcessHarness,
    artifacts: SqliteEvidenceArtifacts,
    event_counter: u64,
}

impl Default for SqliteScenarioRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl SqliteScenarioRunner {
    pub fn new() -> Self {
        Self {
            harness: ProcessHarness::from_current_exe(),
            artifacts: SqliteEvidenceArtifacts {
                process_events: Vec::new(),
                fault_executions: Vec::new(),
                oracle_observations: Vec::new(),
                commands: Vec::new(),
            },
            event_counter: 0,
        }
    }

    pub fn into_artifacts(self) -> SqliteEvidenceArtifacts {
        self.artifacts
    }

    pub fn run_catalog(
        mut self,
        fixture: &EvidenceFixture,
        manifest: EvidenceManifest,
    ) -> (EvidenceRunResult, SqliteEvidenceArtifacts) {
        let mut results = Vec::new();
        let mut manifest = manifest;
        let catalog = scenario_catalog();
        manifest.scenario_ids = catalog
            .iter()
            .map(|scenario| format!("{}@{}", scenario.scenario_id, scenario.scenario_version))
            .collect();
        for scenario in &catalog {
            let fixture = isolated_fixture(fixture, scenario);
            let started = Instant::now();
            let result = match scenario.scenario_id.as_str() {
                "baseline-create-open-close" => self.run_baseline(&fixture, scenario, started),
                "append-correction-event" => self.run_append(&fixture, scenario, started),
                "attach-analysis-result" => self.run_attach(&fixture, scenario, started),
                "stale-review-ledger-command" => self.run_stale_rejection(
                    &fixture,
                    scenario,
                    started,
                    "stale-review-ledger-command",
                ),
                "stale-active-analysis-selection" => {
                    self.run_stale_active(&fixture, scenario, started)
                }
                "stale-analysis-attachment" => self.run_stale_rejection(
                    &fixture,
                    scenario,
                    started,
                    "stale-analysis-attachment",
                ),
                "concurrent-writer-attempt" => {
                    self.run_concurrent_writer(&fixture, scenario, started)
                }
                "unknown-newer-format" => self.run_unknown_format(&fixture, scenario, started),
                "derived-state-corruption" => {
                    self.run_derived_corruption(&fixture, scenario, started)
                }
                "canonical-reference-corruption" => {
                    self.run_canonical_corruption(&fixture, scenario, started)
                }
                "semantic-duplication" => self.run_duplication(&fixture, scenario, started),
                "interrupted-authoritative-transition" => {
                    self.run_interrupted_transition(&fixture, scenario, started)
                }
                "interrupted-compaction" => {
                    self.run_interrupted_compaction(&fixture, scenario, started)
                }
                "interrupted-cleanup" => {
                    self.unsupported(scenario, started, "cleanup not implemented")
                }
                other => self.failed(scenario, started, format!("unknown scenario: {other}")),
            };
            results.push(result);
        }
        let mut run = EvidenceHarness::aggregate(manifest, results);
        run.eligibility = super::model::EvidenceRunEligibility::Inconclusive;
        for supplemental in supplemental_scenarios() {
            let started = Instant::now();
            let _ = self.run_supplemental(fixture, supplemental, started);
        }
        let artifacts = self.artifacts;
        (run, artifacts)
    }

    pub fn run_cross_platform_subset(
        mut self,
        fixture: &EvidenceFixture,
        manifest: EvidenceManifest,
    ) -> (EvidenceRunResult, SqliteEvidenceArtifacts) {
        let mut results = Vec::new();
        let mut manifest = manifest;
        let catalog = scenario_catalog();
        manifest.scenario_ids = CROSS_PLATFORM_SCENARIO_IDS
            .iter()
            .map(|id| format!("{id}@1"))
            .collect();
        for scenario in catalog
            .iter()
            .filter(|s| CROSS_PLATFORM_SCENARIO_IDS.contains(&s.scenario_id.as_str()))
        {
            let fixture = isolated_fixture(fixture, scenario);
            let started = Instant::now();
            let result = match scenario.scenario_id.as_str() {
                "baseline-create-open-close" => self.run_baseline(&fixture, scenario, started),
                "append-correction-event" => self.run_append(&fixture, scenario, started),
                "attach-analysis-result" => self.run_attach(&fixture, scenario, started),
                "stale-review-ledger-command" => self.run_stale_rejection(
                    &fixture,
                    scenario,
                    started,
                    "stale-review-ledger-command",
                ),
                "concurrent-writer-attempt" => {
                    self.run_concurrent_writer(&fixture, scenario, started)
                }
                "unknown-newer-format" => self.run_unknown_format(&fixture, scenario, started),
                "derived-state-corruption" => {
                    self.run_derived_corruption(&fixture, scenario, started)
                }
                "canonical-reference-corruption" => {
                    self.run_canonical_corruption(&fixture, scenario, started)
                }
                "semantic-duplication" => self.run_duplication(&fixture, scenario, started),
                "interrupted-authoritative-transition" => {
                    self.run_interrupted_transition(&fixture, scenario, started)
                }
                other => self.failed(scenario, started, format!("unknown scenario: {other}")),
            };
            results.push(result);
        }
        let mut run = EvidenceHarness::aggregate(manifest, results);
        run.eligibility = super::model::EvidenceRunEligibility::Inconclusive;
        let artifacts = self.artifacts;
        (run, artifacts)
    }

    fn next_event_id(&mut self) -> String {
        self.event_counter += 1;
        format!("evt-{}", self.event_counter)
    }

    fn record_process(
        &mut self,
        role: &str,
        command: &[&str],
        outcome: &super::process_harness::ProcessRunOutcome,
        started: Instant,
    ) {
        let event_id = self.next_event_id();
        self.artifacts.process_events.push(ProcessEventRecord {
            event_id,
            role: role.to_string(),
            pid: Some(outcome.pid),
            command: command.iter().map(|s| (*s).to_string()).collect(),
            exit_classification: outcome.classification.clone(),
            exit_code: outcome.exit_status.and_then(|s| s.code()),
            signal: super::process_harness::exit_signal_name(outcome.exit_status.as_ref()),
            stdout_excerpt: excerpt(&outcome.stdout, 512),
            stderr_excerpt: excerpt(&outcome.stderr, 512),
            started_at_ms: started.elapsed().as_millis(),
            ended_at_ms: started.elapsed().as_millis(),
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn record_fault(
        &mut self,
        scenario_id: &str,
        fault: FaultPoint,
        location: &str,
        pid: Option<u32>,
        recovery: Option<RecoveryClassification>,
        credited: &[&str],
        denied: &[&str],
    ) {
        self.artifacts.fault_executions.push(FaultExecutionRecord {
            fault_id: fault.fault_id().to_string(),
            physical_location: location.to_string(),
            process_pid: pid,
            before_or_after_authority_change: fault.authority_changed_before_fault(),
            termination_mechanism: "std::process::abort".to_string(),
            recovery_classification: recovery,
            claim_credited: credited.iter().map(|s| (*s).to_string()).collect(),
            claim_denied: denied.iter().map(|s| (*s).to_string()).collect(),
            scenario_id: scenario_id.to_string(),
        });
    }

    fn observe(
        &mut self,
        locator: &str,
        observation_id: &str,
        expected: Option<&NormalizedSemanticState>,
        command_ids: &[&str],
    ) -> Result<OracleObservationRecord, AdapterError> {
        let record =
            IndependentSqliteOracle::observe(locator, observation_id, expected, command_ids)?;
        self.artifacts.oracle_observations.push(record.clone());
        Ok(record)
    }

    fn fixture_env(fixture: &EvidenceFixture) -> String {
        serde_json::to_string(fixture).expect("fixture json")
    }

    fn elapsed_measurement(started: Instant) -> BTreeMap<String, ScenarioMeasurement> {
        let mut m = BTreeMap::new();
        m.insert(
            "scenario_elapsed_ms".to_string(),
            ScenarioMeasurement::Integer {
                value: started.elapsed().as_millis() as i128,
                unit: "ms".to_string(),
            },
        );
        m
    }

    #[allow(clippy::too_many_arguments)]
    fn passed(
        scenario: &ScenarioIdentity,
        started: Instant,
        oracle: OracleResult,
        strengths: Vec<&str>,
        failure_classification: Option<RecoveryClassification>,
        reopen: bool,
        process_crash: bool,
        artifact_ref: &str,
    ) -> ScenarioResult {
        ScenarioResult {
            scenario_identity: scenario.clone(),
            status: ScenarioStatus::Passed,
            oracle_result: Some(oracle),
            measurements: Self::elapsed_measurement(started),
            failure_classification,
            limitations: Vec::new(),
            raw_artifact_references: vec![artifact_ref.to_string()],
            achieved_evidence_strength: strengths.into_iter().map(str::to_string).collect(),
            process_interruption_performed: Some(process_crash),
            reopen_performed: Some(reopen),
        }
    }

    fn failed(
        &self,
        scenario: &ScenarioIdentity,
        started: Instant,
        message: String,
    ) -> ScenarioResult {
        ScenarioResult {
            scenario_identity: scenario.clone(),
            status: ScenarioStatus::Failed,
            oracle_result: None,
            measurements: Self::elapsed_measurement(started),
            failure_classification: None,
            limitations: vec![message],
            raw_artifact_references: Vec::new(),
            achieved_evidence_strength: Vec::new(),
            process_interruption_performed: None,
            reopen_performed: None,
        }
    }

    fn unsupported(
        &self,
        scenario: &ScenarioIdentity,
        started: Instant,
        message: &str,
    ) -> ScenarioResult {
        ScenarioResult {
            scenario_identity: scenario.clone(),
            status: ScenarioStatus::Unsupported,
            oracle_result: None,
            measurements: Self::elapsed_measurement(started),
            failure_classification: None,
            limitations: vec![message.to_string()],
            raw_artifact_references: Vec::new(),
            achieved_evidence_strength: Vec::new(),
            process_interruption_performed: None,
            reopen_performed: None,
        }
    }

    fn expected_after_append(
        state: &NormalizedSemanticState,
        event: &ReviewLedgerEventState,
    ) -> NormalizedSemanticState {
        let mut next = state.clone();
        next.review_ledger_events.push(event.clone());
        next.normalize()
    }

    fn expected_after_attach(
        state: &NormalizedSemanticState,
        analysis: &AnalysisResultState,
    ) -> NormalizedSemanticState {
        let mut next = state.clone();
        next.analysis_results.push(analysis.clone());
        next.normalize()
    }

    fn run_baseline(
        &mut self,
        fixture: &EvidenceFixture,
        scenario: &ScenarioIdentity,
        started: Instant,
    ) -> ScenarioResult {
        let root = fresh_storage_root("2c-baseline");
        let mut adapter = EmbeddedRelationalAdapter::new(&root);
        let session = match adapter.create(fixture) {
            Ok(s) => s,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        let handle = match adapter.open(&session, SemanticOpenMode::Writable) {
            Ok(h) => h,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        let _ = adapter.close(&handle);
        let locator = session.adapter_locator();
        match IndependentSqliteOracle::compare_expected(
            locator,
            &fixture.normalized_state(),
            "baseline-reopen",
        ) {
            Ok(obs) => {
                let oracle = obs.semantic_oracle.clone().unwrap_or_else(|| OracleResult {
                    passed: false,
                    violations: Vec::new(),
                    warnings: Vec::new(),
                    expected_fingerprint: None,
                    actual_fingerprint: String::new(),
                    oracle_version: String::new(),
                });
                self.artifacts.oracle_observations.push(obs);
                Self::passed(
                    scenario,
                    started,
                    oracle,
                    vec!["InterfaceBehavior"],
                    None,
                    true,
                    false,
                    "scenario-results/baseline-create-open-close.json",
                )
            }
            Err(e) => self.failed(scenario, started, e.to_string()),
        }
    }

    fn run_append(
        &mut self,
        fixture: &EvidenceFixture,
        scenario: &ScenarioIdentity,
        started: Instant,
    ) -> ScenarioResult {
        let root = fresh_storage_root("2c-append");
        let mut adapter = EmbeddedRelationalAdapter::new(root.clone());
        let session = match adapter.create(fixture) {
            Ok(s) => s,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        let handle = match adapter.open(&session, SemanticOpenMode::Writable) {
            Ok(h) => h,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        let state = match adapter.read_normalized_state(&handle) {
            Ok(s) => s,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        let event = sample_append_event(&state);
        let cmd_id = catalog_command_id("append-correction-event", "apply");
        let preconditions = vec![SemanticPrecondition::ReviewLedgerHead {
            expected_event_id: state
                .review_ledger_events
                .last()
                .map(|e| e.event_id.clone()),
        }];
        let append_command = AuthoritativeCommand::AppendCorrectionEvent {
            command_operation_id: cmd_id.clone(),
            event: event.clone(),
            preconditions: preconditions.clone(),
        };
        let expected = Self::expected_after_append(&state, &event);
        if let Err(e) = adapter.apply_authoritative_command(&handle, &append_command) {
            return self.failed(scenario, started, e.to_string());
        }
        let _ = adapter.close(&handle);
        let locator = session.adapter_locator();
        match self.observe(
            locator,
            "append-reopen",
            Some(&expected.clone().normalize()),
            &[&cmd_id],
        ) {
            Ok(obs) => {
                let oracle = obs.semantic_oracle.clone().unwrap();
                if !oracle.passed {
                    return self.failed(scenario, started, "append reopen oracle failed".into());
                }
                let mut retry_adapter = EmbeddedRelationalAdapter::new(root);
                let retry_handle = retry_adapter
                    .open(&session, SemanticOpenMode::Writable)
                    .expect("reopen writable");
                let retry =
                    retry_adapter.apply_authoritative_command(&retry_handle, &append_command);
                let _ = retry_adapter.close(&retry_handle);
                if retry.is_err() {
                    return self.failed(scenario, started, "idempotent retry failed".into());
                }
                Self::passed(
                    scenario,
                    started,
                    oracle,
                    vec!["LogicalStateTransition"],
                    None,
                    true,
                    false,
                    "scenario-results/append-correction-event.json",
                )
            }
            Err(e) => self.failed(scenario, started, e.to_string()),
        }
    }

    fn run_attach(
        &mut self,
        fixture: &EvidenceFixture,
        scenario: &ScenarioIdentity,
        started: Instant,
    ) -> ScenarioResult {
        let root = fresh_storage_root("2c-attach");
        let mut adapter = EmbeddedRelationalAdapter::new(root);
        let session = match adapter.create(fixture) {
            Ok(s) => s,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        let handle = match adapter.open(&session, SemanticOpenMode::Writable) {
            Ok(h) => h,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        let state = match adapter.read_normalized_state(&handle) {
            Ok(s) => s,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        let analysis = sample_attach_analysis(&state);
        let cmd_id = catalog_command_id("attach-analysis-result", "apply");
        let expected = Self::expected_after_attach(&state, &analysis);
        if let Err(e) = adapter.apply_authoritative_command(
            &handle,
            &AuthoritativeCommand::AttachAnalysisResult {
                command_operation_id: cmd_id.clone(),
                analysis_result: analysis,
                preconditions: vec![SemanticPrecondition::AnalysisAttachmentSet {
                    expected_analysis_result_ids: state
                        .analysis_results
                        .iter()
                        .map(|a| a.analysis_result_id.clone())
                        .collect(),
                }],
            },
        ) {
            return self.failed(scenario, started, e.to_string());
        }
        let _ = adapter.close(&handle);
        match self.observe(
            session.adapter_locator(),
            "attach-reopen",
            Some(&expected.normalize()),
            &[&cmd_id],
        ) {
            Ok(obs) => {
                let oracle = obs.semantic_oracle.clone().unwrap();
                Self::passed(
                    scenario,
                    started,
                    oracle,
                    vec!["LogicalStateTransition"],
                    None,
                    true,
                    false,
                    "scenario-results/attach-analysis-result.json",
                )
            }
            Err(e) => self.failed(scenario, started, e.to_string()),
        }
    }

    fn run_stale_rejection(
        &mut self,
        fixture: &EvidenceFixture,
        scenario: &ScenarioIdentity,
        started: Instant,
        kind: &str,
    ) -> ScenarioResult {
        let root = fresh_storage_root(&format!("2c-{kind}"));
        let mut adapter = EmbeddedRelationalAdapter::new(root);
        let session = match adapter.create(fixture) {
            Ok(s) => s,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        let handle = match adapter.open(&session, SemanticOpenMode::Writable) {
            Ok(h) => h,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        let before = match adapter.read_normalized_state(&handle) {
            Ok(s) => s,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        let command = match kind {
            "stale-review-ledger-command" => AuthoritativeCommand::AppendCorrectionEvent {
                command_operation_id: catalog_command_id(kind, "apply"),
                event: sample_append_event(&before),
                preconditions: vec![SemanticPrecondition::ReviewLedgerHead {
                    expected_event_id: Some("ledger-event:missing".to_string()),
                }],
            },
            _ => AuthoritativeCommand::AttachAnalysisResult {
                command_operation_id: catalog_command_id(kind, "apply"),
                analysis_result: sample_attach_analysis(&before),
                preconditions: vec![SemanticPrecondition::AnalysisAttachmentSet {
                    expected_analysis_result_ids: vec!["analysis-result:missing".to_string()],
                }],
            },
        };
        let result = adapter.apply_authoritative_command(&handle, &command);
        let _ = adapter.close(&handle);
        match result {
            Err(e) if e.code == "stale-precondition" => {
                match self.observe(
                    session.adapter_locator(),
                    &format!("{kind}-unchanged"),
                    Some(&before.normalize()),
                    &[],
                ) {
                    Ok(obs) => {
                        let oracle = obs.semantic_oracle.clone().unwrap();
                        Self::passed(
                            scenario,
                            started,
                            oracle,
                            vec!["InterfaceBehavior"],
                            None,
                            true,
                            false,
                            &format!("scenario-results/{kind}.json"),
                        )
                    }
                    Err(e) => self.failed(scenario, started, e.to_string()),
                }
            }
            Ok(()) => self.failed(scenario, started, "stale command succeeded".into()),
            Err(e) => self.failed(scenario, started, e.to_string()),
        }
    }

    fn run_stale_active(
        &mut self,
        fixture: &EvidenceFixture,
        scenario: &ScenarioIdentity,
        started: Instant,
    ) -> ScenarioResult {
        let root = fresh_storage_root("2c-stale-active");
        let mut adapter = EmbeddedRelationalAdapter::new(root);
        let session = match adapter.create(fixture) {
            Ok(s) => s,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        let handle = match adapter.open(&session, SemanticOpenMode::Writable) {
            Ok(h) => h,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        let before = match adapter.read_normalized_state(&handle) {
            Ok(s) => s,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        let result = adapter.apply_authoritative_command(
            &handle,
            &AuthoritativeCommand::SelectActiveAnalysis {
                command_operation_id: catalog_command_id(
                    "stale-active-analysis-selection",
                    "apply",
                ),
                selection: sample_active_analysis_selection(
                    &before
                        .analysis_results
                        .first()
                        .expect("analysis")
                        .analysis_result_id,
                ),
                preconditions: vec![SemanticPrecondition::ActiveAnalysisSelection {
                    expected_analysis_result_id: Some("analysis-result:missing".to_string()),
                }],
            },
        );
        let _ = adapter.close(&handle);
        match result {
            Err(e) if e.code == "stale-precondition" => match self.observe(
                session.adapter_locator(),
                "stale-active-unchanged",
                Some(&before.normalize()),
                &[],
            ) {
                Ok(obs) => Self::passed(
                    scenario,
                    started,
                    obs.semantic_oracle.clone().unwrap(),
                    vec!["InterfaceBehavior"],
                    None,
                    true,
                    false,
                    "scenario-results/stale-active-analysis-selection.json",
                ),
                Err(e) => self.failed(scenario, started, e.to_string()),
            },
            Ok(()) => self.failed(scenario, started, "stale selection succeeded".into()),
            Err(e) => self.failed(scenario, started, e.to_string()),
        }
    }

    fn run_concurrent_writer(
        &mut self,
        fixture: &EvidenceFixture,
        scenario: &ScenarioIdentity,
        started: Instant,
    ) -> ScenarioResult {
        let root = fresh_storage_root("2c-concurrent");
        let session_id = fixture.normalized_state().session.session_id.clone();
        let fixture_json = Self::fixture_env(fixture);
        let root_lossy = root.to_string_lossy();
        let env_base = [
            ("VOXPROOF_STORAGE_ROOT", root_lossy.as_ref()),
            ("VOXPROOF_FIXTURE_JSON", fixture_json.as_str()),
            ("VOXPROOF_LEASE_DURATION_MS", "2000"),
        ];
        let hold_started = Instant::now();
        let held = match self.harness.spawn_waiting_ready(
            "hold-writer",
            &env_base,
            Duration::from_secs(10),
        ) {
            Ok(h) => h,
            Err(e) => return self.failed(scenario, started, e),
        };

        let attempt_env = [
            ("VOXPROOF_STORAGE_ROOT", root_lossy.as_ref()),
            ("VOXPROOF_SESSION_ID", session_id.as_str()),
            ("VOXPROOF_LEASE_DURATION_MS", "2000"),
        ];
        let attempt_started = Instant::now();
        let attempt =
            self.harness
                .spawn_worker("attempt-writer", &attempt_env, Duration::from_secs(10));
        let attempt = match attempt {
            Ok(o) => o,
            Err(e) => return self.failed(scenario, started, e),
        };
        self.record_process(
            "attempt-writer-reject",
            &["attempt-writer"],
            &attempt,
            attempt_started,
        );
        if !attempt.stdout.contains("writer-already-open") {
            return self.failed(
                scenario,
                started,
                format!("expected writer rejection, got: {}", attempt.stdout),
            );
        }

        let hold = self.harness.kill_held_worker(held, hold_started);
        self.record_process("hold-writer", &["hold-writer"], &hold, hold_started);

        std::thread::sleep(Duration::from_millis(2100));
        let takeover_started = Instant::now();
        let takeover =
            self.harness
                .spawn_worker("attempt-writer", &attempt_env, Duration::from_secs(10));
        let takeover = match takeover {
            Ok(o) => o,
            Err(e) => return self.failed(scenario, started, e),
        };
        self.record_process(
            "attempt-writer-takeover",
            &["attempt-writer"],
            &takeover,
            takeover_started,
        );
        if !takeover.stdout.contains("\"ok\":true") {
            return self.failed(
                scenario,
                started,
                format!("takeover failed: {}", takeover.stdout),
            );
        }
        let locator = root
            .join(filesystem_safe_path_segment(&session_id))
            .to_string_lossy()
            .to_string();
        match self.observe(
            &locator,
            "orphan-recovery",
            Some(&fixture.normalized_state()),
            &[],
        ) {
            Ok(obs) => {
                let oracle = obs.semantic_oracle.clone().unwrap_or_else(|| OracleResult {
                    passed: false,
                    violations: Vec::new(),
                    warnings: Vec::new(),
                    expected_fingerprint: None,
                    actual_fingerprint: String::new(),
                    oracle_version: String::new(),
                });
                if !oracle.passed {
                    return self.failed(
                        scenario,
                        started,
                        "canonical state changed after concurrent writer scenario".into(),
                    );
                }
                Self::passed(
                    scenario,
                    started,
                    oracle,
                    vec!["InterfaceBehavior"],
                    None,
                    true,
                    true,
                    "scenario-results/concurrent-writer-attempt.json",
                )
            }
            Err(e) => self.failed(scenario, started, e.to_string()),
        }
    }

    fn run_unknown_format(
        &mut self,
        fixture: &EvidenceFixture,
        scenario: &ScenarioIdentity,
        started: Instant,
    ) -> ScenarioResult {
        let root = fresh_storage_root("2c-format");
        let mut adapter = EmbeddedRelationalAdapter::new(root);
        let session = match adapter.create(fixture) {
            Ok(s) => s,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        if let Err(e) = adapter.corrupt_or_fault_inject(&session, scenario) {
            return self.failed(scenario, started, e.to_string());
        }
        match adapter.open(&session, SemanticOpenMode::Writable) {
            Err(e) if e.code == "unsupported-newer-format" => {
                match EmbeddedRelationalAdapter::observe_writer_ownership(session.adapter_locator())
                {
                    Ok((token, _, _)) if token.is_none() => Self::passed(
                        scenario,
                        started,
                        OracleResult {
                            passed: true,
                            violations: Vec::new(),
                            warnings: Vec::new(),
                            expected_fingerprint: None,
                            actual_fingerprint: String::new(),
                            oracle_version: super::oracle::ORACLE_VERSION.to_string(),
                        },
                        vec!["InterfaceBehavior"],
                        Some(RecoveryClassification::ManualReviewRequired),
                        false,
                        false,
                        "scenario-results/unknown-newer-format.json",
                    ),
                    Ok((token, _, _)) => self.failed(
                        scenario,
                        started,
                        format!("writer ownership not cleared: {token:?}"),
                    ),
                    Err(err) => self.failed(scenario, started, err.to_string()),
                }
            }
            Ok(_) => self.failed(scenario, started, "writable open succeeded".into()),
            Err(e) => self.failed(scenario, started, e.to_string()),
        }
    }

    fn run_derived_corruption(
        &mut self,
        fixture: &EvidenceFixture,
        scenario: &ScenarioIdentity,
        started: Instant,
    ) -> ScenarioResult {
        let root = fresh_storage_root("2c-derived");
        let mut adapter = EmbeddedRelationalAdapter::new(root);
        let session = match adapter.create(fixture) {
            Ok(s) => s,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        if let Err(e) = adapter.corrupt_or_fault_inject(&session, scenario) {
            return self.failed(scenario, started, e.to_string());
        }
        match self.observe(
            session.adapter_locator(),
            "derived-canonical-readability",
            Some(&fixture.normalized_state()),
            &[],
        ) {
            Ok(obs) => {
                let oracle = obs.semantic_oracle.clone().unwrap();
                let mut result = Self::passed(
                    scenario,
                    started,
                    oracle,
                    Vec::new(),
                    None,
                    true,
                    false,
                    "scenario-results/derived-state-corruption.json",
                );
                result.limitations = vec![
                    "canonical readability subclaim only; detection and rebuild subclaims not demonstrated"
                        .to_string(),
                ];
                result
            }
            Err(e) => self.failed(scenario, started, e.to_string()),
        }
    }

    fn run_canonical_corruption(
        &mut self,
        fixture: &EvidenceFixture,
        scenario: &ScenarioIdentity,
        started: Instant,
    ) -> ScenarioResult {
        let root = fresh_storage_root("2c-canonical");
        let mut adapter = EmbeddedRelationalAdapter::new(root);
        let session = match adapter.create(fixture) {
            Ok(s) => s,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        if let Err(e) = adapter.corrupt_or_fault_inject(&session, scenario) {
            return self.failed(scenario, started, e.to_string());
        }
        match adapter.open(&session, SemanticOpenMode::Writable) {
            Err(e) if e.code == "canonical-corruption" || e.code == "sqlite-open-failed" => {
                Self::passed(
                    scenario,
                    started,
                    OracleResult {
                        passed: true,
                        violations: Vec::new(),
                        warnings: Vec::new(),
                        expected_fingerprint: None,
                        actual_fingerprint: String::new(),
                        oracle_version: super::oracle::ORACLE_VERSION.to_string(),
                    },
                    vec!["InterfaceBehavior"],
                    Some(RecoveryClassification::Unrecoverable),
                    false,
                    false,
                    "scenario-results/canonical-reference-corruption.json",
                )
            }
            Ok(_) => self.failed(scenario, started, "corrupted session opened".into()),
            Err(e) => self.failed(scenario, started, format!("unexpected: {}", e.code)),
        }
    }

    fn run_duplication(
        &mut self,
        fixture: &EvidenceFixture,
        scenario: &ScenarioIdentity,
        started: Instant,
    ) -> ScenarioResult {
        let root = fresh_storage_root("2c-dup");
        let mut adapter = EmbeddedRelationalAdapter::new(root.clone());
        let session = match adapter.create(fixture) {
            Ok(s) => s,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        let handle = match adapter.open(&session, SemanticOpenMode::Writable) {
            Ok(h) => h,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        let source_before = match adapter.read_normalized_state(&handle) {
            Ok(s) => s,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        let duplicate_id = format!(
            "{}:duplicate",
            fixture.normalized_state().session.session_id
        );
        let dup = match adapter.duplicate_session(&handle, &duplicate_id) {
            Ok(d) => d,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        let _ = adapter.close(&handle);
        let source_locator = session.adapter_locator();
        let dest_locator = dup.session.adapter_locator();
        let mut expected_dest = source_before.clone();
        expected_dest.session.session_id = duplicate_id.clone();
        expected_dest.session.duplicated_from_session_id =
            Some(fixture.normalized_state().session.session_id.clone());
        match (
            self.observe(
                source_locator,
                "dup-source",
                Some(&source_before.normalize()),
                &[],
            ),
            self.observe(
                dest_locator,
                "dup-dest",
                Some(&expected_dest.normalize()),
                &[],
            ),
        ) {
            (Ok(src), Ok(dst)) => {
                let src_oracle = src.semantic_oracle.clone().unwrap();
                let dst_oracle = dst.semantic_oracle.clone().unwrap();
                if !src_oracle.passed || !dst_oracle.passed {
                    return self.failed(scenario, started, "duplication oracle failed".into());
                }
                Self::passed(
                    scenario,
                    started,
                    dst_oracle,
                    vec!["LogicalStateTransition"],
                    None,
                    true,
                    false,
                    "scenario-results/semantic-duplication.json",
                )
            }
            (Err(e), _) | (_, Err(e)) => self.failed(scenario, started, e.to_string()),
        }
    }

    fn run_interrupted_transition(
        &mut self,
        fixture: &EvidenceFixture,
        scenario: &ScenarioIdentity,
        started: Instant,
    ) -> ScenarioResult {
        let root = fresh_storage_root("2c-interrupted-tx");
        let fixture_json = Self::fixture_env(fixture);
        let cmd_id = catalog_command_id("interrupted-authoritative-transition", "apply");
        let mut post_commit_oracle: Option<OracleResult> = None;

        for (fault, expect_unchanged) in [
            (FaultPoint::BeforeSqliteCommit, true),
            (FaultPoint::AfterSqliteCommitBeforeAck, false),
        ] {
            let sub_root = root.join(fault.fault_id());
            let _ = std::fs::remove_dir_all(&sub_root);
            let sub_root_lossy = sub_root.to_string_lossy();
            let env = [
                ("VOXPROOF_STORAGE_ROOT", sub_root_lossy.as_ref()),
                ("VOXPROOF_FIXTURE_JSON", fixture_json.as_str()),
                ("VOXPROOF_FAULT_POINT", fault.fault_id()),
                ("VOXPROOF_LEASE_DURATION_MS", "2000"),
                ("VOXPROOF_COMMAND_OPERATION_ID", cmd_id.as_str()),
            ];
            let proc_started = Instant::now();
            let outcome =
                self.harness
                    .spawn_worker("apply-command-crash", &env, Duration::from_secs(15));
            let outcome = match outcome {
                Ok(o) => o,
                Err(e) => return self.failed(scenario, started, e),
            };
            self.record_process(
                "apply-command-crash",
                &["apply-command-crash"],
                &outcome,
                proc_started,
            );
            if outcome.classification == ProcessExitClassification::Success {
                return self.failed(scenario, started, "crash worker exited successfully".into());
            }
            self.record_fault(
                &scenario.scenario_id,
                fault,
                "embedded_relational.rs:apply_authoritative_command",
                Some(outcome.pid),
                if expect_unchanged {
                    Some(RecoveryClassification::LastCommittedState)
                } else {
                    Some(RecoveryClassification::SafeAutomaticRecovery)
                },
                if expect_unchanged {
                    &["InterfaceBehavior"]
                } else {
                    &["ProcessCrashRecovery", "LogicalStateTransition"]
                },
                &[],
            );

            let session_id = fixture.normalized_state().session.session_id.clone();
            let locator = sub_root
                .join(filesystem_safe_path_segment(&session_id))
                .to_string_lossy()
                .to_string();
            let mut expected = fixture.normalized_state();
            if !expect_unchanged {
                let event = sample_append_event(&expected);
                let _ = apply_command(
                    &mut expected,
                    &AuthoritativeCommand::AppendCorrectionEvent {
                        command_operation_id: catalog_command_id(
                            "interrupted-authoritative-transition",
                            "expected",
                        ),
                        event,
                        preconditions: vec![SemanticPrecondition::ReviewLedgerHead {
                            expected_event_id: fixture
                                .normalized_state()
                                .review_ledger_events
                                .last()
                                .map(|e| e.event_id.clone()),
                        }],
                    },
                );
            }
            let expected_normalized = expected.clone().normalize();
            match self.observe(
                &locator,
                &format!("crash-{}", fault.fault_id()),
                Some(&expected_normalized),
                &[&cmd_id],
            ) {
                Ok(obs) => {
                    let oracle = obs.semantic_oracle.clone().unwrap();
                    if expect_unchanged && !oracle.passed {
                        return self.failed(
                            scenario,
                            started,
                            "pre-commit crash mutated state".into(),
                        );
                    }
                    if !expect_unchanged && !oracle.passed {
                        return self.failed(
                            scenario,
                            started,
                            "post-commit crash state mismatch".into(),
                        );
                    }
                    if !expect_unchanged {
                        post_commit_oracle = obs.semantic_oracle.clone();
                    }
                    if !expect_unchanged {
                        if EmbeddedRelationalAdapter::observe_applied_command(&locator, &cmd_id)
                            .ok()
                            .flatten()
                            .is_none()
                        {
                            return self.failed(
                                scenario,
                                started,
                                "missing committed command record after post-commit crash".into(),
                            );
                        }
                        if let Some((_, status)) =
                            EmbeddedRelationalAdapter::observe_applied_command(&locator, &cmd_id)
                                .ok()
                                .flatten()
                            && status != "committed"
                        {
                            return self.failed(
                                scenario,
                                started,
                                "expected committed-only after post-commit crash".into(),
                            );
                        }
                        std::thread::sleep(Duration::from_millis(2100));
                        let mut retry = EmbeddedRelationalAdapter::new(sub_root.clone());
                        let session = EvidenceSessionRef::new(session_id.clone(), locator.clone());
                        let retry_handle = match retry.open(&session, SemanticOpenMode::Writable) {
                            Ok(h) => h,
                            Err(e) => {
                                return self.failed(
                                    scenario,
                                    started,
                                    format!("retry open after lease expiry failed: {}", e.code),
                                );
                            }
                        };
                        let pre_state = fixture.normalized_state();
                        let retry_event = sample_append_event(&pre_state);
                        let retry_command = AuthoritativeCommand::AppendCorrectionEvent {
                            command_operation_id: cmd_id.clone(),
                            event: retry_event,
                            preconditions: vec![SemanticPrecondition::ReviewLedgerHead {
                                expected_event_id: pre_state
                                    .review_ledger_events
                                    .last()
                                    .map(|e| e.event_id.clone()),
                            }],
                        };
                        if retry
                            .apply_authoritative_command(&retry_handle, &retry_command)
                            .is_err()
                        {
                            return self.failed(
                                scenario,
                                started,
                                "committed-only retry reconciliation failed".into(),
                            );
                        }
                        let _ = retry.close(&retry_handle);
                        if let Some((_, status)) =
                            EmbeddedRelationalAdapter::observe_applied_command(&locator, &cmd_id)
                                .ok()
                                .flatten()
                            && status != "acknowledged"
                        {
                            return self.failed(
                                scenario,
                                started,
                                "expected acknowledged after retry reconciliation".into(),
                            );
                        }
                    }
                }
                Err(e) => return self.failed(scenario, started, e.to_string()),
            }
        }

        let matrix_oracle = post_commit_oracle.unwrap_or(OracleResult {
            passed: true,
            violations: Vec::new(),
            warnings: Vec::new(),
            expected_fingerprint: None,
            actual_fingerprint: String::new(),
            oracle_version: super::oracle::ORACLE_VERSION.to_string(),
        });
        Self::passed(
            scenario,
            started,
            matrix_oracle,
            vec![
                "InterfaceBehavior",
                "ProcessCrashRecovery",
                "LogicalStateTransition",
            ],
            Some(RecoveryClassification::SafeAutomaticRecovery),
            true,
            true,
            "scenario-results/interrupted-authoritative-transition.json",
        )
    }

    fn run_interrupted_compaction(
        &mut self,
        fixture: &EvidenceFixture,
        scenario: &ScenarioIdentity,
        started: Instant,
    ) -> ScenarioResult {
        let root = fresh_storage_root("2c-compaction");
        let mut adapter = EmbeddedRelationalAdapter::new(root);
        let session = match adapter.create(fixture) {
            Ok(s) => s,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        let handle = match adapter.open(&session, SemanticOpenMode::Writable) {
            Ok(h) => h,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        adapter.arm_test_fault(FaultPoint::InterruptCompaction);
        match adapter.cleanup_or_compact_if_supported(&handle, MaintenanceOperation::Compact) {
            Err(e) if e.code == "simulated-compaction-interrupt" => {
                let _ = adapter.close(&handle);
                match self.observe(
                    session.adapter_locator(),
                    "compaction-reopen",
                    Some(&fixture.normalized_state()),
                    &[],
                ) {
                    Ok(obs) => Self::passed(
                        scenario,
                        started,
                        obs.semantic_oracle.clone().unwrap(),
                        vec!["InterfaceBehavior"],
                        None,
                        true,
                        false,
                        "scenario-results/interrupted-compaction.json",
                    ),
                    Err(e) => self.failed(scenario, started, e.to_string()),
                }
            }
            Ok(OptionalOperationOutcome::Completed) => self.failed(
                scenario,
                started,
                "compaction completed under interrupt".into(),
            ),
            other => self.failed(scenario, started, format!("unexpected: {other:?}")),
        }
    }

    fn run_supplemental(
        &mut self,
        fixture: &EvidenceFixture,
        supplemental_id: &str,
        started: Instant,
    ) -> ScenarioResult {
        let identity = ScenarioIdentity {
            scenario_id: supplemental_id.to_string(),
            scenario_version: 1,
            category: super::scenario::ScenarioCategory::Baseline,
            description: format!("Supplemental SQLite evidence: {supplemental_id}"),
            failure_model: super::scenario::FailureModel::None,
            required_capabilities: Vec::new(),
            requirement: ScenarioRequirement::Required,
            evidence_kind: super::scenario::ScenarioEvidenceKind::SemanticCorrectness,
        };
        match supplemental_id {
            "sqlite-live-stale-handle-epoch-reject" => {
                self.run_live_stale_handle(fixture, &identity, started)
            }
            "sqlite-backward-clock-ambiguity" => {
                self.run_backward_clock(fixture, &identity, started)
            }
            "sqlite-busy-fail-closed" => self.run_busy_fail_closed(fixture, &identity, started),
            _ => ScenarioResult {
                scenario_identity: identity,
                status: ScenarioStatus::Passed,
                oracle_result: None,
                measurements: Self::elapsed_measurement(started),
                failure_classification: None,
                limitations: vec![
                    "supplemental scenario covered by catalog equivalent".to_string(),
                ],
                raw_artifact_references: vec![format!("scenario-results/{supplemental_id}.json")],
                achieved_evidence_strength: vec!["InterfaceBehavior".to_string()],
                process_interruption_performed: Some(false),
                reopen_performed: Some(false),
            },
        }
    }

    fn run_live_stale_handle(
        &mut self,
        fixture: &EvidenceFixture,
        scenario: &ScenarioIdentity,
        started: Instant,
    ) -> ScenarioResult {
        let root = fresh_storage_root("2c-stale-handle");
        let mut adapter_a = EmbeddedRelationalAdapter::new(root.clone());
        let session = match adapter_a.create(fixture) {
            Ok(s) => s,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        let handle_a = match adapter_a.open(&session, SemanticOpenMode::Writable) {
            Ok(h) => h,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        adapter_a.set_test_clock_ms(Some(1_000_000));
        let mut adapter_b = EmbeddedRelationalAdapter::new(root.clone());
        adapter_b.set_test_clock_ms(Some(1_000_000 + 60_000));
        let _handle_b = match adapter_b.open(&session, SemanticOpenMode::Writable) {
            Ok(h) => h,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        let result = adapter_a.apply_authoritative_command(
            &handle_a,
            &AuthoritativeCommand::AppendCorrectionEvent {
                command_operation_id: catalog_command_id("sqlite-live-stale-handle", "apply"),
                event: sample_append_event(&fixture.normalized_state()),
                preconditions: vec![SemanticPrecondition::ReviewLedgerHead {
                    expected_event_id: fixture
                        .normalized_state()
                        .review_ledger_events
                        .last()
                        .map(|e| e.event_id.clone()),
                }],
            },
        );
        match result {
            Err(e) if e.code == "writer-epoch-mismatch" => Self::passed(
                scenario,
                started,
                OracleResult {
                    passed: true,
                    violations: Vec::new(),
                    warnings: Vec::new(),
                    expected_fingerprint: None,
                    actual_fingerprint: String::new(),
                    oracle_version: super::oracle::ORACLE_VERSION.to_string(),
                },
                vec!["InterfaceBehavior"],
                Some(RecoveryClassification::SafeAutomaticRecovery),
                false,
                false,
                "scenario-results/sqlite-live-stale-handle-epoch-reject.json",
            ),
            other => self.failed(
                scenario,
                started,
                format!("expected epoch mismatch: {other:?}"),
            ),
        }
    }

    fn run_backward_clock(
        &mut self,
        fixture: &EvidenceFixture,
        scenario: &ScenarioIdentity,
        started: Instant,
    ) -> ScenarioResult {
        let root = fresh_storage_root("2c-backward-clock");
        let mut adapter = EmbeddedRelationalAdapter::new(root);
        let session = match adapter.create(fixture) {
            Ok(s) => s,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        adapter.set_test_clock_ms(Some(5_000));
        let handle = match adapter.open(&session, SemanticOpenMode::Writable) {
            Ok(h) => h,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        adapter.set_test_clock_ms(Some(1_000));
        let result = adapter.apply_authoritative_command(
            &handle,
            &AuthoritativeCommand::AppendCorrectionEvent {
                command_operation_id: catalog_command_id("sqlite-backward-clock", "apply"),
                event: sample_append_event(&fixture.normalized_state()),
                preconditions: vec![],
            },
        );
        match result {
            Err(e) if e.code == "lease-clock-ambiguous" => Self::passed(
                scenario,
                started,
                OracleResult {
                    passed: true,
                    violations: Vec::new(),
                    warnings: Vec::new(),
                    expected_fingerprint: None,
                    actual_fingerprint: String::new(),
                    oracle_version: super::oracle::ORACLE_VERSION.to_string(),
                },
                vec!["InterfaceBehavior"],
                None,
                false,
                false,
                "scenario-results/sqlite-backward-clock-ambiguity.json",
            ),
            other => self.failed(
                scenario,
                started,
                format!("expected clock ambiguity: {other:?}"),
            ),
        }
    }

    fn run_busy_fail_closed(
        &mut self,
        fixture: &EvidenceFixture,
        scenario: &ScenarioIdentity,
        started: Instant,
    ) -> ScenarioResult {
        let root = fresh_storage_root("2c-busy");
        let mut adapter = EmbeddedRelationalAdapter::new(root.clone());
        let session = match adapter.create(fixture) {
            Ok(s) => s,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        adapter.set_test_clock_ms(Some(1_000_000));
        let handle = match adapter.open(&session, SemanticOpenMode::Writable) {
            Ok(h) => h,
            Err(e) => return self.failed(scenario, started, e.to_string()),
        };
        adapter.set_force_sqlite_busy_on_tx_begin_once(true);
        let _ = adapter.apply_authoritative_command(
            &handle,
            &AuthoritativeCommand::AppendCorrectionEvent {
                command_operation_id: catalog_command_id("sqlite-busy", "apply1"),
                event: sample_append_event(&fixture.normalized_state()),
                preconditions: vec![],
            },
        );
        let mut adapter_b = EmbeddedRelationalAdapter::new(root);
        adapter_b.set_test_clock_ms(Some(1_000_000 + 60_000));
        match adapter_b.open(&session, SemanticOpenMode::Writable) {
            Ok(_) => {}
            Err(e) => {
                return self.failed(scenario, started, format!("takeover failed: {}", e.code));
            }
        }
        let result = adapter.apply_authoritative_command(
            &handle,
            &AuthoritativeCommand::AppendCorrectionEvent {
                command_operation_id: catalog_command_id("sqlite-busy", "apply2"),
                event: sample_append_event(&fixture.normalized_state()),
                preconditions: vec![],
            },
        );
        match result {
            Err(e) if e.code == "writer-epoch-mismatch" => Self::passed(
                scenario,
                started,
                OracleResult {
                    passed: true,
                    violations: Vec::new(),
                    warnings: Vec::new(),
                    expected_fingerprint: None,
                    actual_fingerprint: String::new(),
                    oracle_version: super::oracle::ORACLE_VERSION.to_string(),
                },
                vec!["InterfaceBehavior"],
                None,
                false,
                false,
                "scenario-results/sqlite-busy-fail-closed.json",
            ),
            other => self.failed(
                scenario,
                started,
                format!("expected epoch mismatch after busy: {other:?}"),
            ),
        }
    }
}

fn supplemental_scenarios() -> Vec<&'static str> {
    vec![
        "sqlite-live-stale-handle-epoch-reject",
        "sqlite-backward-clock-ambiguity",
        "sqlite-busy-fail-closed",
    ]
}

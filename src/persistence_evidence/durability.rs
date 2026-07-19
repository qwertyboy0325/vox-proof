//! Durability trial scheduling and outcome recording (Package 2D).

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::adapter::{
    AuthoritativeCommand, MaintenanceOperation, PersistenceCandidateAdapter, SemanticOpenMode,
    SemanticPrecondition,
};
use super::candidates::fault::FaultPoint;
use super::candidates::semantic_ops::{apply_command, sample_append_event};
use super::fixture::EvidenceFixture;
use super::independent_oracle::IndependentSqliteOracle;
use super::model::NormalizedSemanticState;
use super::process_harness::{ProcessExitClassification, ProcessHarness, ProcessRunOutcome};
use super::platform::filesystem_safe_path_segment;
use super::scenario_runner::{catalog_command_id, fresh_storage_root};
use super::EmbeddedRelationalAdapter;

pub const MIN_TRIALS_PER_POINT: u32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterruptionModel {
    ClassAProcessKill,
    LogicalReturnError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncBoundaryRecord {
    pub boundary_id: String,
    pub description: String,
    pub commit_returned: Option<bool>,
    pub ack_returned: Option<bool>,
    pub checkpoint_completed: Option<bool>,
    pub rename_completed: Option<bool>,
    pub directory_sync_performed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrialOutcome {
    Passed,
    Failed,
    Indeterminate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurabilityTrialResult {
    pub trial_id: String,
    pub experiment_id: String,
    pub platform_label: String,
    pub trial_index: u32,
    pub interruption_model: InterruptionModel,
    pub sync_boundary: SyncBoundaryRecord,
    pub fault_point: String,
    pub outcome: TrialOutcome,
    pub oracle_passed: Option<bool>,
    pub claim_credited: Vec<String>,
    pub claim_denied: Vec<String>,
    pub failure_reason: Option<String>,
    pub elapsed_ms: u128,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_exit_classification: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_exit_signal: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurabilityExperimentSpec {
    pub experiment_id: String,
    pub fault_point: FaultPoint,
    pub interruption_model: InterruptionModel,
    pub sync_boundary: SyncBoundaryRecord,
    pub credited: &'static [&'static str],
    pub denied: &'static [&'static str],
}

pub fn durability_experiments() -> Vec<DurabilityExperimentSpec> {
    vec![
        DurabilityExperimentSpec {
            experiment_id: "post-commit-before-ack-process-kill".to_string(),
            fault_point: FaultPoint::AfterSqliteCommitBeforeAck,
            interruption_model: InterruptionModel::ClassAProcessKill,
            sync_boundary: SyncBoundaryRecord {
                boundary_id: "after_sqlite_commit_before_adapter_ack".to_string(),
                description: "SQLite COMMIT completed; adapter ack not returned".to_string(),
                commit_returned: Some(true),
                ack_returned: Some(false),
                checkpoint_completed: None,
                rename_completed: None,
                directory_sync_performed: false,
            },
            credited: &["ProcessCrashRecovery"],
            denied: &["FilesystemDurability", "HardwarePowerLoss"],
        },
        DurabilityExperimentSpec {
            experiment_id: "pre-commit-process-kill".to_string(),
            fault_point: FaultPoint::BeforeSqliteCommit,
            interruption_model: InterruptionModel::ClassAProcessKill,
            sync_boundary: SyncBoundaryRecord {
                boundary_id: "before_sqlite_commit".to_string(),
                description: "Mutation begun; COMMIT not reached".to_string(),
                commit_returned: Some(false),
                ack_returned: Some(false),
                checkpoint_completed: None,
                rename_completed: None,
                directory_sync_performed: false,
            },
            credited: &["InterfaceBehavior"],
            denied: &["ProcessCrashRecovery", "FilesystemDurability", "HardwarePowerLoss"],
        },
        DurabilityExperimentSpec {
            experiment_id: "wal-checkpoint-interrupt".to_string(),
            fault_point: FaultPoint::DuringCheckpoint,
            interruption_model: InterruptionModel::LogicalReturnError,
            sync_boundary: SyncBoundaryRecord {
                boundary_id: "before_wal_checkpoint_truncate".to_string(),
                description: "Fault armed before wal_checkpoint(TRUNCATE) invocation; logical ReturnError".to_string(),
                commit_returned: None,
                ack_returned: None,
                checkpoint_completed: Some(false),
                rename_completed: None,
                directory_sync_performed: false,
            },
            credited: &["InterfaceBehavior"],
            denied: &["FilesystemDurability", "HardwarePowerLoss"],
        },
        DurabilityExperimentSpec {
            experiment_id: "duplication-backup-interrupt".to_string(),
            fault_point: FaultPoint::DuringBackupCopy,
            interruption_model: InterruptionModel::LogicalReturnError,
            sync_boundary: SyncBoundaryRecord {
                boundary_id: "before_online_backup_copy".to_string(),
                description: "Fault armed before backup.run_to_completion; pre-rename temp state".to_string(),
                commit_returned: None,
                ack_returned: None,
                checkpoint_completed: None,
                rename_completed: Some(false),
                directory_sync_performed: false,
            },
            credited: &["InterfaceBehavior"],
            denied: &["FilesystemDurability", "HardwarePowerLoss"],
        },
        DurabilityExperimentSpec {
            experiment_id: "post-publication-process-kill".to_string(),
            fault_point: FaultPoint::AfterPublishBeforeReturn,
            interruption_model: InterruptionModel::ClassAProcessKill,
            sync_boundary: SyncBoundaryRecord {
                boundary_id: "after_rename_before_return".to_string(),
                description: "fs::rename completed; no parent directory fsync".to_string(),
                commit_returned: None,
                ack_returned: None,
                checkpoint_completed: Some(true),
                rename_completed: Some(true),
                directory_sync_performed: false,
            },
            credited: &["ProcessCrashRecovery"],
            denied: &["FilesystemDurability", "HardwarePowerLoss"],
        },
    ]
}

fn intentional_process_abort(outcome: &ProcessRunOutcome) -> bool {
    if let Some(status) = outcome.exit_status.as_ref() {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            if status.signal() == Some(6) {
                return true;
            }
        }
        #[cfg(windows)]
        {
            if let Some(code) = status.code() {
                if code == 3 || code as u32 == 0xC0000409 {
                    return true;
                }
            }
        }
    }
    super::process_harness::exit_signal_name(outcome.exit_status.as_ref()) == Some("SIGABRT".to_string())
}

pub struct DurabilityTrialRunner {
    harness: ProcessHarness,
    platform_label: String,
}

impl DurabilityTrialRunner {
    pub fn new(platform_label: String) -> Self {
        Self {
            harness: ProcessHarness::from_current_exe(),
            platform_label,
        }
    }

    pub fn run_all(
        &self,
        fixture: &EvidenceFixture,
        trials_per_experiment: u32,
    ) -> Vec<DurabilityTrialResult> {
        let mut results = Vec::new();
        for spec in durability_experiments() {
            for trial_index in 0..trials_per_experiment {
                let started = Instant::now();
                let trial_id = format!("{}-trial-{}", spec.experiment_id, trial_index);
                let result = match spec.experiment_id.as_str() {
                    "post-publication-process-kill" => {
                        self.run_duplicate_crash_trial(fixture, &spec, trial_index, started)
                    }
                    "wal-checkpoint-interrupt" | "duplication-backup-interrupt" => {
                        self.run_logical_fault_trial(fixture, &spec, trial_index, started)
                    }
                    _ => self.run_apply_crash_trial(fixture, &spec, trial_index, started),
                };
                results.push(result);
            }
        }
        results
    }

    fn run_apply_crash_trial(
        &self,
        fixture: &EvidenceFixture,
        spec: &DurabilityExperimentSpec,
        trial_index: u32,
        started: Instant,
    ) -> DurabilityTrialResult {
        let trial_id = format!("{}-trial-{}", spec.experiment_id, trial_index);
        let root = fresh_storage_root(&format!("2d-{}", spec.experiment_id));
        let trial_root = root.join(format!("trial-{trial_index}"));
        let _ = std::fs::remove_dir_all(&trial_root);
        let fixture_json = serde_json::to_string(fixture).expect("fixture json");
        let cmd_id = catalog_command_id(&spec.experiment_id, &format!("trial-{trial_index}"));
        let trial_root_string = trial_root.to_string_lossy().to_string();
        let env = [
            ("VOXPROOF_STORAGE_ROOT", trial_root_string.as_str()),
            ("VOXPROOF_FIXTURE_JSON", fixture_json.as_str()),
            ("VOXPROOF_FAULT_POINT", spec.fault_point.fault_id()),
            ("VOXPROOF_LEASE_DURATION_MS", "2000"),
            ("VOXPROOF_COMMAND_OPERATION_ID", cmd_id.as_str()),
        ];
        let outcome = match self.harness.spawn_worker("apply-command-crash", &env, Duration::from_secs(15))
        {
            Ok(o) => o,
            Err(error) => {
                return trial_result(
                    trial_id,
                    spec,
                    trial_index,
                    TrialOutcome::Failed,
                    None,
                    Some(error),
                    started,
                    &self.platform_label,
                    None,
                );
            }
        };
        if outcome.classification == ProcessExitClassification::Success {
            return trial_result(
                trial_id,
                spec,
                trial_index,
                TrialOutcome::Failed,
                None,
                Some("crash worker exited successfully".to_string()),
                started,
                &self.platform_label,
                Some(&outcome),
            );
        }
        if spec.interruption_model == InterruptionModel::ClassAProcessKill
            && !intentional_process_abort(&outcome)
        {
            return trial_result(
                trial_id,
                spec,
                trial_index,
                TrialOutcome::Failed,
                None,
                Some(format!(
                    "expected intentional process abort, got {:?}",
                    outcome.classification
                )),
                started,
                &self.platform_label,
                Some(&outcome),
            );
        }
        let expect_unchanged = spec.fault_point == FaultPoint::BeforeSqliteCommit;
        let session_id = fixture.normalized_state().session.session_id.clone();
        let locator = trial_root
            .join(filesystem_safe_path_segment(&session_id))
            .to_string_lossy()
            .to_string();
        let mut expected = fixture.normalized_state();
        if !expect_unchanged {
            let event = sample_append_event(&expected);
            let _ = apply_command(
                &mut expected,
                &AuthoritativeCommand::AppendCorrectionEvent {
                    command_operation_id: catalog_command_id(&spec.experiment_id, "expected"),
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
        let oracle_passed = match IndependentSqliteOracle::observe(
            &locator,
            &trial_id,
            Some(&expected.normalize()),
            &[&cmd_id],
        ) {
            Ok(obs) => obs.semantic_oracle.as_ref().map(|o| o.passed),
            Err(error) => {
                return trial_result(
                    trial_id,
                    spec,
                    trial_index,
                    TrialOutcome::Indeterminate,
                    None,
                    Some(error.to_string()),
                    started,
                    &self.platform_label,
                    Some(&outcome),
                );
            }
        };
        let passed = oracle_passed == Some(true);
        trial_result(
            trial_id,
            spec,
            trial_index,
            if passed {
                TrialOutcome::Passed
            } else {
                TrialOutcome::Failed
            },
            oracle_passed,
            if passed {
                None
            } else {
                Some("oracle mismatch after trial".to_string())
            },
            started,
            &self.platform_label,
            Some(&outcome),
        )
    }

    fn run_logical_fault_trial(
        &self,
        fixture: &EvidenceFixture,
        spec: &DurabilityExperimentSpec,
        trial_index: u32,
        started: Instant,
    ) -> DurabilityTrialResult {
        let trial_id = format!("{}-trial-{}", spec.experiment_id, trial_index);
        let root = fresh_storage_root(&format!("2d-{}", spec.experiment_id));
        let trial_root = root.join(format!("trial-{trial_index}"));
        let _ = std::fs::remove_dir_all(&trial_root);
        let mut adapter = EmbeddedRelationalAdapter::new(trial_root.clone());
        adapter.arm_fault_return_error(spec.fault_point);
        let session = match adapter.create(fixture) {
            Ok(s) => s,
            Err(error) => {
                return trial_result(
                    trial_id,
                    spec,
                    trial_index,
                    TrialOutcome::Failed,
                    None,
                    Some(error.to_string()),
                    started,
                    &self.platform_label,
                    None,
                );
            }
        };
        let handle = match adapter.open(&session, SemanticOpenMode::Writable) {
            Ok(h) => h,
            Err(error) => {
                return trial_result(
                    trial_id,
                    spec,
                    trial_index,
                    TrialOutcome::Failed,
                    None,
                    Some(error.to_string()),
                    started,
                    &self.platform_label,
                    None,
                );
            }
        };
        let outcome = if spec.fault_point == FaultPoint::DuringBackupCopy {
            PersistenceCandidateAdapter::duplicate_session(
                &mut adapter,
                &handle,
                &format!("dup-{trial_index}"),
            )
            .map(|_| ())
        } else if spec.fault_point == FaultPoint::DuringCheckpoint {
            PersistenceCandidateAdapter::duplicate_session(
                &mut adapter,
                &handle,
                &format!("dup-{trial_index}"),
            )
            .map(|_| ())
        } else {
            adapter
                .cleanup_or_compact_if_supported(&handle, MaintenanceOperation::Compact)
                .map(|_| ())
        };
        let passed = outcome.is_err();
        trial_result(
            trial_id,
            spec,
            trial_index,
            if passed {
                TrialOutcome::Passed
            } else {
                TrialOutcome::Failed
            },
            None,
            if passed {
                None
            } else {
                Some("expected fault did not trigger".to_string())
            },
            started,
            &self.platform_label,
            None,
        )
    }

    fn run_duplicate_crash_trial(
        &self,
        fixture: &EvidenceFixture,
        spec: &DurabilityExperimentSpec,
        trial_index: u32,
        started: Instant,
    ) -> DurabilityTrialResult {
        let trial_id = format!("{}-trial-{}", spec.experiment_id, trial_index);
        let root = fresh_storage_root(&format!("2d-{}", spec.experiment_id));
        let trial_root = root.join(format!("trial-{trial_index}"));
        let _ = std::fs::remove_dir_all(&trial_root);
        let fixture_json = serde_json::to_string(fixture).expect("fixture json");
        let trial_root_string = trial_root.to_string_lossy().to_string();
        let dest_id = format!("dup-{trial_index}");
        let env = [
            ("VOXPROOF_STORAGE_ROOT", trial_root_string.as_str()),
            ("VOXPROOF_FIXTURE_JSON", fixture_json.as_str()),
            ("VOXPROOF_FAULT_POINT", spec.fault_point.fault_id()),
            ("VOXPROOF_DUP_DEST_ID", dest_id.as_str()),
        ];
        let outcome = match self.harness.spawn_worker("duplicate-and-crash", &env, Duration::from_secs(20))
        {
            Ok(o) => o,
            Err(error) => {
                return trial_result(
                    trial_id,
                    spec,
                    trial_index,
                    TrialOutcome::Failed,
                    None,
                    Some(error),
                    started,
                    &self.platform_label,
                    None,
                );
            }
        };
        if outcome.classification == ProcessExitClassification::Success {
            return trial_result(
                trial_id,
                spec,
                trial_index,
                TrialOutcome::Failed,
                None,
                Some("duplicate crash worker exited successfully".to_string()),
                started,
                &self.platform_label,
                Some(&outcome),
            );
        }
        if !intentional_process_abort(&outcome) {
            return trial_result(
                trial_id,
                spec,
                trial_index,
                TrialOutcome::Failed,
                None,
                Some(format!(
                    "expected intentional process abort, got {:?}",
                    outcome.classification
                )),
                started,
                &self.platform_label,
                Some(&outcome),
            );
        }
        let dest_id = format!("dup-{trial_index}");
        let dest_locator = trial_root
            .join(filesystem_safe_path_segment(&dest_id))
            .to_string_lossy()
            .to_string();
        let mut expected_dest = fixture.normalized_state().clone();
        expected_dest.session.session_id = dest_id.clone();
        expected_dest.session.duplicated_from_session_id =
            Some(fixture.normalized_state().session.session_id.clone());
        let oracle_passed = IndependentSqliteOracle::observe(
            &dest_locator,
            &trial_id,
            Some(&expected_dest.normalize()),
            &[],
        )
        .ok()
        .and_then(|obs| obs.semantic_oracle.map(|o| o.passed));
        let passed = oracle_passed == Some(true);
        trial_result(
            trial_id,
            spec,
            trial_index,
            if passed {
                TrialOutcome::Passed
            } else {
                TrialOutcome::Indeterminate
            },
            oracle_passed,
            if passed {
                None
            } else {
                Some("destination not readable after post-publish crash".to_string())
            },
            started,
            &self.platform_label,
            Some(&outcome),
        )
    }
}

fn trial_result(
    trial_id: String,
    spec: &DurabilityExperimentSpec,
    trial_index: u32,
    outcome: TrialOutcome,
    oracle_passed: Option<bool>,
    failure_reason: Option<String>,
    started: Instant,
    platform_label: &str,
    process_outcome: Option<&ProcessRunOutcome>,
) -> DurabilityTrialResult {
    let claim_credited = if outcome == TrialOutcome::Passed {
        spec.credited.iter().map(|s| (*s).to_string()).collect()
    } else {
        Vec::new()
    };
    DurabilityTrialResult {
        trial_id,
        experiment_id: spec.experiment_id.clone(),
        platform_label: platform_label.to_string(),
        trial_index,
        interruption_model: spec.interruption_model,
        sync_boundary: spec.sync_boundary.clone(),
        fault_point: spec.fault_point.fault_id().to_string(),
        outcome,
        oracle_passed,
        claim_credited,
        claim_denied: spec.denied.iter().map(|s| (*s).to_string()).collect(),
        failure_reason,
        elapsed_ms: started.elapsed().as_millis(),
        process_exit_classification: process_outcome.map(|o| format!("{:?}", o.classification)),
        process_exit_code: process_outcome.and_then(|o| o.exit_status.and_then(|s| s.code())),
        process_exit_signal: process_outcome
            .and_then(|o| super::process_harness::exit_signal_name(o.exit_status.as_ref())),
    }
}

use std::path::{Path, PathBuf};
use std::time::Instant;

use super::super::append_authoritative::{
    AppendAuthoritativeCandidateAdapter, AppendOpenMode, OpenedAppendAuthoritySession,
};
use super::super::fixture::{
    build_base_manual_replacement_state, build_candidate_rejected_state, build_golden_small_state,
    build_promoted_active_state, build_revoked_historical_state, build_superseded_state,
};
use super::super::model::CurrentContractState;
use super::super::sqlite_authoritative::{
    OpenedSqliteAuthoritySession, SqliteAuthoritativeCandidateAdapter, SqliteOpenMode,
};
use super::super::{finalize_derived_fields, CurrentContractOracle, CurrentContractPreconditions};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentContractCandidateKind {
    Append,
    Sqlite,
}

pub enum CurrentContractCandidate {
    Append {
        adapter: AppendAuthoritativeCandidateAdapter,
        storage_root: PathBuf,
    },
    Sqlite {
        adapter: SqliteAuthoritativeCandidateAdapter,
        storage_root: PathBuf,
    },
}

#[derive(Debug)]
pub enum OpenedCandidateSession {
    Append(OpenedAppendAuthoritySession),
    Sqlite(OpenedSqliteAuthoritySession),
}

pub struct OperationTiming {
    pub elapsed_ms: u128,
    pub error_code: Option<String>,
    pub succeeded: bool,
}

impl CurrentContractCandidate {
    pub fn append(root: impl Into<PathBuf>) -> Result<Self, String> {
        let storage_root = root.into();
        AppendAuthoritativeCandidateAdapter::new(&storage_root)
            .map(|adapter| Self::Append {
                adapter,
                storage_root,
            })
            .map_err(|error| error.code.to_owned())
    }

    pub fn sqlite(root: impl Into<PathBuf>) -> Result<Self, String> {
        let storage_root = root.into();
        SqliteAuthoritativeCandidateAdapter::new(&storage_root)
            .map(|adapter| Self::Sqlite {
                adapter,
                storage_root,
            })
            .map_err(|error| error.code.to_owned())
    }

    pub fn kind(&self) -> CurrentContractCandidateKind {
        match self {
            Self::Append { .. } => CurrentContractCandidateKind::Append,
            Self::Sqlite { .. } => CurrentContractCandidateKind::Sqlite,
        }
    }

    pub fn candidate_id(&self) -> &str {
        match self {
            Self::Append { adapter, .. } => adapter.candidate_id(),
            Self::Sqlite { adapter, .. } => adapter.candidate_id(),
        }
    }

    pub fn candidate_version(&self) -> &str {
        match self {
            Self::Append { adapter, .. } => adapter.candidate_version(),
            Self::Sqlite { adapter, .. } => adapter.candidate_version(),
        }
    }

    pub fn compaction_supported(&self) -> bool {
        false
    }

    pub fn destructive_cleanup_supported(&self) -> bool {
        false
    }

    pub fn storage_root(&self) -> &Path {
        match self {
            Self::Append { storage_root, .. } | Self::Sqlite { storage_root, .. } => storage_root,
        }
    }

    pub fn create_session(
        &self,
        state: &CurrentContractState,
    ) -> Result<(String, PathBuf), String> {
        match self {
            Self::Append { adapter, .. } => adapter
                .create(state)
                .map(|session| {
                    let path = session.storage_path_for_test().to_path_buf();
                    (session.session_id().to_owned(), path)
                })
                .map_err(|error| error.code.to_owned()),
            Self::Sqlite { adapter, .. } => adapter
                .create(state)
                .map(|session| {
                    let path = session.storage_path_for_test().to_path_buf();
                    (session.session_id().to_owned(), path)
                })
                .map_err(|error| error.code.to_owned()),
        }
    }

    pub fn open_writable(&self, session_id: &str) -> Result<OpenedCandidateSession, String> {
        match self {
            Self::Append { adapter, .. } => adapter
                .open_existing(session_id, AppendOpenMode::Writable)
                .map(OpenedCandidateSession::Append)
                .map_err(|error| error.code.to_owned()),
            Self::Sqlite { adapter, .. } => adapter
                .open_existing(session_id, SqliteOpenMode::Writable)
                .map(OpenedCandidateSession::Sqlite)
                .map_err(|error| error.code.to_owned()),
        }
    }

    pub fn open_read_only(&self, session_id: &str) -> Result<OpenedCandidateSession, String> {
        match self {
            Self::Append { adapter, .. } => adapter
                .open_existing(session_id, AppendOpenMode::ReadOnly)
                .map(OpenedCandidateSession::Append)
                .map_err(|error| error.code.to_owned()),
            Self::Sqlite { adapter, .. } => adapter
                .open_existing(session_id, SqliteOpenMode::ReadOnly)
                .map(OpenedCandidateSession::Sqlite)
                .map_err(|error| error.code.to_owned()),
        }
    }

    pub fn close(&self, opened: OpenedCandidateSession) -> Result<(), String> {
        match (self, opened) {
            (Self::Append { adapter, .. }, OpenedCandidateSession::Append(handle)) => {
                adapter.close(handle).map_err(|error| error.code.to_owned())
            }
            (Self::Sqlite { adapter, .. }, OpenedCandidateSession::Sqlite(handle)) => {
                adapter.close(handle).map_err(|error| error.code.to_owned())
            }
            _ => Err("candidate-handle-mismatch".to_owned()),
        }
    }

    pub fn session_storage_path(&self, session_path: &Path) -> PathBuf {
        session_path.to_path_buf()
    }

    pub fn duplicate(
        &self,
        opened: &mut OpenedCandidateSession,
        new_session_id: &str,
    ) -> Result<String, String> {
        match (self, opened) {
            (Self::Append { adapter, .. }, OpenedCandidateSession::Append(handle)) => adapter
                .duplicate(handle, new_session_id)
                .map(|session| session.session_id().to_owned())
                .map_err(|error| error.code.to_owned()),
            (Self::Sqlite { adapter, .. }, OpenedCandidateSession::Sqlite(handle)) => adapter
                .duplicate(handle, new_session_id)
                .map(|session| session.session_id().to_owned())
                .map_err(|error| error.code.to_owned()),
            _ => Err("candidate-handle-mismatch".to_owned()),
        }
    }

    pub fn apply_transition_stale(
        &self,
        opened: &mut OpenedCandidateSession,
        next_state: &CurrentContractState,
    ) -> Result<(), String> {
        match (self, opened) {
            (Self::Append { adapter, .. }, OpenedCandidateSession::Append(handle)) => adapter
                .append_authoritative_transition(handle, 0, next_state)
                .map(|_| ())
                .map_err(|error| error.code.to_owned()),
            (Self::Sqlite { adapter, .. }, OpenedCandidateSession::Sqlite(handle)) => {
                let preconditions = handle.authoritative_preconditions();
                let stale = CurrentContractPreconditions {
                    expected_generation: 0,
                    review_ledger_head: preconditions.review_ledger_head,
                    reuse_governance_head: preconditions.reuse_governance_head,
                    active_analysis_snapshot_identity: preconditions
                        .active_analysis_snapshot_identity
                        .clone(),
                };
                adapter
                    .apply_authoritative_transition(handle, &stale, next_state)
                    .map(|_| ())
                    .map_err(|error| error.code.to_owned())
            }
            _ => Err("candidate-handle-mismatch".to_owned()),
        }
    }

    pub fn apply_transition(
        &self,
        opened: &mut OpenedCandidateSession,
        next_state: &CurrentContractState,
    ) -> Result<(), String> {
        match (self, opened) {
            (Self::Append { adapter, .. }, OpenedCandidateSession::Append(handle)) => adapter
                .append_authoritative_transition(handle, handle.committed_sequence, next_state)
                .map(|_| ())
                .map_err(|error| error.code.to_owned()),
            (Self::Sqlite { adapter, .. }, OpenedCandidateSession::Sqlite(handle)) => {
                let preconditions = handle.authoritative_preconditions();
                let mapped = CurrentContractPreconditions {
                    expected_generation: preconditions.expected_generation,
                    review_ledger_head: preconditions.review_ledger_head,
                    reuse_governance_head: preconditions.reuse_governance_head,
                    active_analysis_snapshot_identity: preconditions
                        .active_analysis_snapshot_identity
                        .clone(),
                };
                adapter
                    .apply_authoritative_transition(handle, &mapped, next_state)
                    .map(|_| ())
                    .map_err(|error| error.code.to_owned())
            }
            _ => Err("candidate-handle-mismatch".to_owned()),
        }
    }

    pub fn normalized_state(
        &self,
        opened: &OpenedCandidateSession,
    ) -> Result<CurrentContractState, String> {
        match opened {
            OpenedCandidateSession::Append(handle) => Ok(handle.normalized_state().clone()),
            OpenedCandidateSession::Sqlite(handle) => Ok(handle.normalized_state().clone()),
        }
    }

    pub fn oracle_compare(
        &self,
        expected: &CurrentContractState,
        opened: &OpenedCandidateSession,
    ) -> bool {
        self.normalized_state(opened)
            .map(|actual| CurrentContractOracle::compare(expected, &actual).passed)
            .unwrap_or(false)
    }

    pub fn measure<F>(&self, operation: F) -> (OperationTiming, F::Output)
    where
        F: FnOnce() -> Result<(), String>,
    {
        let started = Instant::now();
        let result = operation();
        let elapsed_ms = started.elapsed().as_millis();
        match &result {
            Ok(()) => (
                OperationTiming {
                    elapsed_ms,
                    error_code: None,
                    succeeded: true,
                },
                result,
            ),
            Err(code) => (
                OperationTiming {
                    elapsed_ms,
                    error_code: Some(code.clone()),
                    succeeded: false,
                },
                result,
            ),
        }
    }

    pub fn tamper_derived_cache_for_test(
        &self,
        opened: &mut OpenedCandidateSession,
        key: &str,
    ) -> Result<(), String> {
        match (self, opened) {
            (Self::Sqlite { adapter, .. }, OpenedCandidateSession::Sqlite(handle)) => adapter
                .tamper_derived_cache_for_test(handle, key)
                .map_err(|error| error.code.to_owned()),
            _ => Err("derived-cache-tamper-unsupported".to_owned()),
        }
    }

    pub fn tamper_derived_projection_for_test(
        &self,
        opened: &mut OpenedCandidateSession,
    ) -> Result<(), String> {
        match (self, opened) {
            (Self::Append { adapter, .. }, OpenedCandidateSession::Append(handle)) => {
                lag_append_checkpoint_for_test(adapter, handle)
            }
            (Self::Sqlite { adapter, .. }, OpenedCandidateSession::Sqlite(handle)) => adapter
                .tamper_derived_cache_for_test(handle, "not-a-derived-projection")
                .map_err(|error| error.code.to_owned()),
            _ => Err("derived-projection-tamper-unsupported".to_owned()),
        }
    }

    pub fn append_incomplete_tail_for_test(
        &self,
        opened: &mut OpenedCandidateSession,
        next_state: &CurrentContractState,
    ) -> Result<(), String> {
        match (self, opened) {
            (Self::Append { adapter, .. }, OpenedCandidateSession::Append(handle)) => adapter
                .append_incomplete_tail_for_test(handle, next_state)
                .map_err(|error| error.code.to_owned()),
            _ => Err("incomplete-tail-unsupported".to_owned()),
        }
    }

    pub fn arm_fail_before_commit_for_test(&self) {
        if let Self::Sqlite { adapter, .. } = self {
            adapter.arm_fail_before_commit_for_test();
        }
    }

    pub fn apply_transition_with_preconditions(
        &self,
        opened: &mut OpenedCandidateSession,
        preconditions: &CurrentContractPreconditions,
        next_state: &CurrentContractState,
    ) -> Result<(), String> {
        match (self, opened) {
            (Self::Append { adapter, .. }, OpenedCandidateSession::Append(handle)) => adapter
                .append_authoritative_transition(
                    handle,
                    preconditions.expected_generation,
                    next_state,
                )
                .map(|_| ())
                .map_err(|error| error.code.to_owned()),
            (Self::Sqlite { adapter, .. }, OpenedCandidateSession::Sqlite(handle)) => adapter
                .apply_authoritative_transition(handle, preconditions, next_state)
                .map(|_| ())
                .map_err(|error| error.code.to_owned()),
            _ => Err("candidate-handle-mismatch".to_owned()),
        }
    }

    pub fn authoritative_preconditions(
        &self,
        opened: &OpenedCandidateSession,
    ) -> Result<CurrentContractPreconditions, String> {
        match opened {
            OpenedCandidateSession::Append(handle) => Ok(CurrentContractPreconditions {
                expected_generation: handle.committed_sequence,
                review_ledger_head: handle
                    .normalized_state()
                    .durable_command_tokens
                    .review_ledger_head,
                reuse_governance_head: handle
                    .normalized_state()
                    .durable_command_tokens
                    .reuse_governance_head,
                active_analysis_snapshot_identity: handle
                    .normalized_state()
                    .durable_command_tokens
                    .active_analysis_snapshot_identity
                    .clone(),
            }),
            OpenedCandidateSession::Sqlite(handle) => {
                let preconditions = handle.authoritative_preconditions();
                Ok(CurrentContractPreconditions {
                    expected_generation: preconditions.expected_generation,
                    review_ledger_head: preconditions.review_ledger_head,
                    reuse_governance_head: preconditions.reuse_governance_head,
                    active_analysis_snapshot_identity: preconditions
                        .active_analysis_snapshot_identity
                        .clone(),
                })
            }
        }
    }

    pub fn session_storage_bytes(&self, session_id: &str) -> u64 {
        match self {
            Self::Append { adapter, .. } => adapter
                .open_existing(session_id, AppendOpenMode::ReadOnly)
                .map(|handle| directory_size_bytes(handle.session.storage_path_for_test()))
                .unwrap_or(0),
            Self::Sqlite { adapter, .. } => adapter
                .open_existing(session_id, SqliteOpenMode::ReadOnly)
                .map(|handle| directory_size_bytes(handle.session.storage_path_for_test()))
                .unwrap_or(0),
        }
    }
}

pub fn updated_writer_token_state(
    mut state: CurrentContractState,
    token: &str,
) -> CurrentContractState {
    state.durable_command_tokens.evidence_writer_token = token.to_owned();
    finalize_derived_fields(&mut state);
    state.normalize()
}

pub fn fixture_state_for_scale(
    scale: super::super::measurement::MeasurementFixtureScale,
) -> CurrentContractState {
    use super::super::measurement::MeasurementFixtureScale;
    match scale {
        MeasurementFixtureScale::Small => build_golden_small_state(),
        MeasurementFixtureScale::Medium => super::super::fixture::build_medium_fixture_state(),
        MeasurementFixtureScale::Stress => build_golden_small_state(),
    }
}

pub fn directory_size_bytes(path: &Path) -> u64 {
    if !path.exists() {
        return 0;
    }
    if path.is_file() {
        return path.metadata().map(|m| m.len()).unwrap_or(0);
    }
    let mut total = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            total += directory_size_bytes(&entry.path());
        }
    }
    total
}

pub fn unique_session_id_for_sample(
    state: &CurrentContractState,
    _sample_index: u32,
) -> CurrentContractState {
    state.clone()
}

pub fn measurement_transition_states(
    operation: &str,
    _scale: super::super::measurement::MeasurementFixtureScale,
) -> Option<(CurrentContractState, CurrentContractState)> {
    use super::super::fixture::{
        build_base_manual_replacement_state, build_candidate_rejected_state,
        build_promoted_active_state, build_revoked_historical_state, build_superseded_state,
    };
    match operation {
        "append_review_decision" => Some((
            build_candidate_rejected_state(),
            build_base_manual_replacement_state(),
        )),
        "append_manual_replacement" | "append_reusable_promotion" => Some((
            build_base_manual_replacement_state(),
            build_promoted_active_state(),
        )),
        "append_reusable_revocation" => Some((
            build_promoted_active_state(),
            build_revoked_historical_state(),
        )),
        "append_reusable_supersession" => {
            Some((build_promoted_active_state(), build_superseded_state()))
        }
        _ => None,
    }
}

fn lag_append_checkpoint_for_test(
    _adapter: &AppendAuthoritativeCandidateAdapter,
    handle: &mut OpenedAppendAuthoritySession,
) -> Result<(), String> {
    let manifest_path = handle.session.storage_path_for_test().join("manifest.json");
    let bytes = std::fs::read_to_string(&manifest_path).map_err(|error| error.to_string())?;
    let mut manifest: serde_json::Value =
        serde_json::from_str(&bytes).map_err(|error| error.to_string())?;
    if let Some(sequence) = manifest
        .get_mut("committed_sequence")
        .and_then(|value| value.as_u64())
    {
        let updated = sequence.saturating_sub(1);
        manifest["committed_sequence"] = serde_json::Value::from(updated);
    } else {
        return Err("manifest-missing-committed-sequence".to_owned());
    }
    std::fs::write(
        &manifest_path,
        serde_json::to_string(&manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn scenario_fixture_state(scenario_id: &str) -> CurrentContractState {
    match scenario_id {
        "append-manual-replacement" => build_base_manual_replacement_state(),
        "append-promotion-candidate-rejection" => build_candidate_rejected_state(),
        "append-reusable-promotion" => build_promoted_active_state(),
        "append-reusable-revocation" => build_revoked_historical_state(),
        "append-reusable-supersession" => build_superseded_state(),
        _ => build_golden_small_state(),
    }
}

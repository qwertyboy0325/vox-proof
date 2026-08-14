//! SQLite candidate 01C-SQLITE-3 — scoped-command apply onto latest canonical authority.
//!
//! Physical `committed_generation` remains transactional ordering only. Semantic command
//! validity is scoped to the command write-set, not a global generation CAS.

use super::derivation::finalize_derived_fields;
use super::model::CurrentContractState;
use super::sqlite_authoritative::{
    validate_state, DurableSqliteAck, OpenedSqliteAuthoritySession,
    SqliteAuthoritativeCandidateAdapter, SqliteAuthorityError, SqliteAuthoritySession,
    SqliteOpenMode, SQLITE_AUTHORITATIVE_CANDIDATE_ID,
};

pub const SQLITE_SCOPED_PRECONDITION_CANDIDATE_VERSION: &str = "01C-SQLITE-3";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqliteCommandScope {
    ReviewLedger,
    ReuseGovernance,
    ActiveAnalysis,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteScopedPreconditions {
    pub review_ledger_head: usize,
    pub reuse_governance_head: usize,
    pub active_analysis_snapshot_identity: String,
}

impl SqliteScopedPreconditions {
    pub fn from_authority_at_command_prepare(state: &CurrentContractState) -> Self {
        Self {
            review_ledger_head: state.durable_command_tokens.review_ledger_head,
            reuse_governance_head: state.durable_command_tokens.reuse_governance_head,
            active_analysis_snapshot_identity: state
                .durable_command_tokens
                .active_analysis_snapshot_identity
                .clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SqliteScopedCommand {
    pub scope: SqliteCommandScope,
    pub preconditions: SqliteScopedPreconditions,
    precursor: CurrentContractState,
    target: CurrentContractState,
}

impl SqliteScopedCommand {
    pub fn from_transition_pair(
        scope: SqliteCommandScope,
        precursor: &CurrentContractState,
        target: &CurrentContractState,
    ) -> Self {
        Self {
            scope,
            preconditions: SqliteScopedPreconditions::from_authority_at_command_prepare(precursor),
            precursor: precursor.clone(),
            target: target.clone(),
        }
    }

    pub fn precursor(&self) -> &CurrentContractState {
        &self.precursor
    }

    pub fn target(&self) -> &CurrentContractState {
        &self.target
    }

    fn apply_onto(&self, current: &CurrentContractState) -> Result<CurrentContractState, SqliteAuthorityError> {
        match self.scope {
            SqliteCommandScope::ReviewLedger => apply_review_scope_patch(current, &self.target),
            SqliteCommandScope::ReuseGovernance => apply_reuse_scope_patch(current, &self.target),
            SqliteCommandScope::ActiveAnalysis => apply_analysis_scope_patch(current, &self.target),
        }
    }
}

pub struct SqliteScopedPreconditionCandidateAdapter {
    inner: SqliteAuthoritativeCandidateAdapter,
}

impl SqliteScopedPreconditionCandidateAdapter {
    pub fn new(storage_root: impl Into<std::path::PathBuf>) -> Result<Self, SqliteAuthorityError> {
        Ok(Self {
            inner: SqliteAuthoritativeCandidateAdapter::new(storage_root)?,
        })
    }

    pub fn candidate_id(&self) -> &'static str {
        SQLITE_AUTHORITATIVE_CANDIDATE_ID
    }

    pub fn candidate_version(&self) -> &'static str {
        SQLITE_SCOPED_PRECONDITION_CANDIDATE_VERSION
    }

    pub fn create(
        &self,
        state: &CurrentContractState,
    ) -> Result<SqliteAuthoritySession, SqliteAuthorityError> {
        self.inner.create(state)
    }

    pub fn open_existing(
        &self,
        session_id: impl Into<String>,
        mode: SqliteOpenMode,
    ) -> Result<OpenedSqliteAuthoritySession, SqliteAuthorityError> {
        self.inner.open_existing(session_id, mode)
    }

    pub fn close(&self, opened: OpenedSqliteAuthoritySession) -> Result<(), SqliteAuthorityError> {
        self.inner.close(opened)
    }

    pub fn duplicate(
        &self,
        source: &mut OpenedSqliteAuthoritySession,
        new_session_id: impl Into<String>,
    ) -> Result<SqliteAuthoritySession, SqliteAuthorityError> {
        self.inner.duplicate(source, new_session_id)
    }

    pub fn set_format_version_for_test(
        &self,
        opened: &mut OpenedSqliteAuthoritySession,
        format_version: u32,
    ) -> Result<(), SqliteAuthorityError> {
        self.inner.set_format_version_for_test(opened, format_version)
    }

    pub fn arm_fail_before_commit_for_test(&self) {
        self.inner.arm_fail_before_commit_for_test();
    }

    pub fn arm_fail_after_commit_before_ack_for_test(&self) {
        self.inner.arm_fail_after_commit_before_ack_for_test();
    }

    pub fn set_lease_duration_for_session_id_for_test(
        &self,
        session_id: &str,
        lease_duration_ms: i64,
    ) -> Result<(), SqliteAuthorityError> {
        self.inner
            .set_lease_duration_for_session_id_for_test(session_id, lease_duration_ms)
    }

    pub fn tamper_derived_cache_for_test(
        &self,
        opened: &mut OpenedSqliteAuthoritySession,
        key: &str,
    ) -> Result<(), SqliteAuthorityError> {
        self.inner.tamper_derived_cache_for_test(opened, key)
    }

    pub fn tamper_canonical_provenance_for_test(
        &self,
        opened: &mut OpenedSqliteAuthoritySession,
    ) -> Result<(), SqliteAuthorityError> {
        self.inner.tamper_canonical_provenance_for_test(opened)
    }

    pub fn tamper_source_locator_for_test(
        &self,
        opened: &mut OpenedSqliteAuthoritySession,
    ) -> Result<(), SqliteAuthorityError> {
        self.inner.tamper_source_locator_for_test(opened)
    }

    pub fn tamper_review_ledger_order_for_test(
        &self,
        opened: &mut OpenedSqliteAuthoritySession,
    ) -> Result<(), SqliteAuthorityError> {
        self.inner.tamper_review_ledger_order_for_test(opened)
    }

    pub fn tamper_reuse_governance_order_for_test(
        &self,
        opened: &mut OpenedSqliteAuthoritySession,
    ) -> Result<(), SqliteAuthorityError> {
        self.inner.tamper_reuse_governance_order_for_test(opened)
    }

    pub fn apply_scoped_command(
        &self,
        opened: &mut OpenedSqliteAuthoritySession,
        command: &SqliteScopedCommand,
    ) -> Result<DurableSqliteAck, SqliteAuthorityError> {
        let scope = command.scope;
        let preconditions = command.preconditions.clone();
        self.inner.persist_merged_from_latest_authority(opened, |latest| {
            validate_scoped_preconditions(latest, scope, &preconditions)?;
            command.apply_onto(latest)
        })
    }
}

/// Reject disqualified full-state authority replacement (off-path guard for C3 class).
pub fn reject_full_state_authority_replace(
    current: &CurrentContractState,
    proposed: &CurrentContractState,
) -> Result<(), SqliteAuthorityError> {
    if proposed.durable_command_tokens.reuse_governance_head
        < current.durable_command_tokens.reuse_governance_head
    {
        return Err(SqliteAuthorityError::new(
            "stale-full-state-unrelated-rewind",
            "full-state proposal would rewind reuse-governance authority",
        ));
    }
    if proposed.durable_command_tokens.review_ledger_head
        < current.durable_command_tokens.review_ledger_head
    {
        return Err(SqliteAuthorityError::new(
            "stale-full-state-unrelated-rewind",
            "full-state proposal would rewind review-ledger authority",
        ));
    }
    if proposed.durable_command_tokens.active_analysis_snapshot_identity
        != current.durable_command_tokens.active_analysis_snapshot_identity
        && !proposed
            .analysis_snapshots
            .iter()
            .any(|snapshot| {
                snapshot.identity
                    == current
                        .durable_command_tokens
                        .active_analysis_snapshot_identity
            })
    {
        return Err(SqliteAuthorityError::new(
            "stale-full-state-unrelated-rewind",
            "full-state proposal would rewind active-analysis authority",
        ));
    }
    Ok(())
}

pub fn command_scope_for_stale_scenario(scenario_id: &str) -> Option<SqliteCommandScope> {
    match scenario_id {
        "stale-review-ledger-command" => Some(SqliteCommandScope::ReviewLedger),
        "stale-reuse-governance-command" => Some(SqliteCommandScope::ReuseGovernance),
        "stale-analysis-attachment-or-selection" => Some(SqliteCommandScope::ActiveAnalysis),
        _ => None,
    }
}

pub fn command_scope_for_measurement_operation(operation: &str) -> Option<SqliteCommandScope> {
    match operation {
        "append_review_decision" | "append_manual_replacement" => {
            Some(SqliteCommandScope::ReviewLedger)
        }
        "append_reusable_promotion"
        | "append_reusable_revocation"
        | "append_reusable_supersession" => Some(SqliteCommandScope::ReuseGovernance),
        _ => None,
    }
}

pub fn infer_command_scope(
    precursor: &CurrentContractState,
    target: &CurrentContractState,
) -> Result<SqliteCommandScope, SqliteAuthorityError> {
    let review_changed = precursor.durable_command_tokens.review_ledger_head
        != target.durable_command_tokens.review_ledger_head;
    let reuse_changed = precursor.durable_command_tokens.reuse_governance_head
        != target.durable_command_tokens.reuse_governance_head;
    let analysis_changed = precursor
        .durable_command_tokens
        .active_analysis_snapshot_identity
        != target
            .durable_command_tokens
            .active_analysis_snapshot_identity;
    match (
        review_changed as u8 + reuse_changed as u8 + analysis_changed as u8,
        review_changed,
        reuse_changed,
        analysis_changed,
    ) {
        (1, true, false, false) => Ok(SqliteCommandScope::ReviewLedger),
        (1, false, true, false) => Ok(SqliteCommandScope::ReuseGovernance),
        (1, false, false, true) => Ok(SqliteCommandScope::ActiveAnalysis),
        _ => Err(SqliteAuthorityError::new(
            "ambiguous-scoped-command",
            "transition must change exactly one scoped write-set",
        )),
    }
}

fn validate_scoped_preconditions(
    current: &CurrentContractState,
    scope: SqliteCommandScope,
    preconditions: &SqliteScopedPreconditions,
) -> Result<(), SqliteAuthorityError> {
    match scope {
        SqliteCommandScope::ReviewLedger => {
            if current.durable_command_tokens.review_ledger_head
                != preconditions.review_ledger_head
            {
                return Err(SqliteAuthorityError::new(
                    "stale-review-ledger-precondition",
                    "review-ledger head precondition is stale",
                ));
            }
        }
        SqliteCommandScope::ReuseGovernance => {
            if current.durable_command_tokens.reuse_governance_head
                != preconditions.reuse_governance_head
            {
                return Err(SqliteAuthorityError::new(
                    "stale-reuse-governance-precondition",
                    "reuse-governance head precondition is stale",
                ));
            }
        }
        SqliteCommandScope::ActiveAnalysis => {
            if current.durable_command_tokens.active_analysis_snapshot_identity
                != preconditions.active_analysis_snapshot_identity
            {
                return Err(SqliteAuthorityError::new(
                    "stale-analysis-selection-precondition",
                    "active analysis selection precondition is stale",
                ));
            }
        }
    }
    Ok(())
}

fn apply_review_scope_patch(
    current: &CurrentContractState,
    target: &CurrentContractState,
) -> Result<CurrentContractState, SqliteAuthorityError> {
    let mut result = current.clone();
    result.review_ledger_events = target.review_ledger_events.clone();
    result.effective_review_status = target.effective_review_status.clone();
    result.durable_command_tokens.review_ledger_head =
        target.durable_command_tokens.review_ledger_head;
    finalize_and_validate(&mut result)
}

fn apply_reuse_scope_patch(
    current: &CurrentContractState,
    target: &CurrentContractState,
) -> Result<CurrentContractState, SqliteAuthorityError> {
    let mut result = current.clone();
    result.reuse_governance_events = target.reuse_governance_events.clone();
    result.effective_reusable_records = target.effective_reusable_records.clone();
    result.historical_reusable_records = target.historical_reusable_records.clone();
    result.reusable_snapshot_identity = target.reusable_snapshot_identity.clone();
    result.reuse_enabled_analysis_binding = target.reuse_enabled_analysis_binding.clone();
    if let Some(binding) = &target.reuse_enabled_analysis_binding
        && !result
            .analysis_snapshots
            .iter()
            .any(|snapshot| snapshot.identity == binding.analysis_snapshot_identity)
    {
        result
            .analysis_snapshots
            .push(binding.analysis_snapshot.clone());
    }
    result.durable_command_tokens.reuse_governance_head =
        target.durable_command_tokens.reuse_governance_head;
    finalize_and_validate(&mut result)
}

fn apply_analysis_scope_patch(
    current: &CurrentContractState,
    target: &CurrentContractState,
) -> Result<CurrentContractState, SqliteAuthorityError> {
    let mut result = current.clone();
    for snapshot in &target.analysis_snapshots {
        if !result
            .analysis_snapshots
            .iter()
            .any(|existing| existing.identity == snapshot.identity)
        {
            result.analysis_snapshots.push(snapshot.clone());
        }
    }
    result.durable_command_tokens.active_analysis_snapshot_identity = target
        .durable_command_tokens
        .active_analysis_snapshot_identity
        .clone();
    finalize_and_validate(&mut result)
}

fn finalize_and_validate(
    state: &mut CurrentContractState,
) -> Result<CurrentContractState, SqliteAuthorityError> {
    finalize_derived_fields(state);
    let normalized = state.clone().normalize();
    validate_state(&normalized)?;
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence_evidence::current_contract::evidence_01c::measurement_transitions::measurement_transition_states;
    use crate::persistence_evidence::MeasurementFixtureScale;

    #[test]
    fn infer_scope_detects_single_write_set_change() {
        let (precursor, target) =
            measurement_transition_states("append_review_decision", MeasurementFixtureScale::Small)
                .expect("review transition");
        assert_eq!(
            infer_command_scope(&precursor, &target).expect("scope"),
            SqliteCommandScope::ReviewLedger
        );
        let (precursor, target) = measurement_transition_states(
            "append_reusable_revocation",
            MeasurementFixtureScale::Small,
        )
        .expect("reuse transition");
        assert_eq!(
            infer_command_scope(&precursor, &target).expect("scope"),
            SqliteCommandScope::ReuseGovernance
        );
    }
}

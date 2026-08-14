//! Append candidate 01B-3 — scoped-command apply onto replayed authoritative snapshot.
//!
//! Physical `committed_sequence` remains append-log ordering only. Semantic command
//! validity is scoped to the command write-set, not a global generation CAS.

use super::append_authoritative::{
    read_manifest, recover_committed_prefix, repair_manifest_checkpoint, replay, validate_manifest,
    validate_state, AppendAuthoritativeCandidateAdapter, AppendAuthorityError,
    AppendAuthoritySession, AppendCheckpointStatus, AppendOpenMode, DurableAppendAck,
    OpenedAppendAuthoritySession, APPEND_AUTHORITATIVE_CANDIDATE_ID,
};
use super::derivation::finalize_derived_fields;
use super::model::CurrentContractState;
use super::oracle::canonical_fingerprint;

pub const APPEND_SCOPED_PRECONDITION_CANDIDATE_VERSION: &str = "01B-3";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppendCommandScope {
    ReviewLedger,
    ReuseGovernance,
    ActiveAnalysis,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppendScopedPreconditions {
    pub review_ledger_head: usize,
    pub reuse_governance_head: usize,
    pub active_analysis_snapshot_identity: String,
}

impl AppendScopedPreconditions {
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
pub struct AppendScopedCommand {
    pub scope: AppendCommandScope,
    pub preconditions: AppendScopedPreconditions,
    precursor: CurrentContractState,
    target: CurrentContractState,
}

impl AppendScopedCommand {
    pub fn from_transition_pair(
        scope: AppendCommandScope,
        precursor: &CurrentContractState,
        target: &CurrentContractState,
    ) -> Self {
        Self {
            scope,
            preconditions: AppendScopedPreconditions::from_authority_at_command_prepare(precursor),
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

    fn apply_onto(&self, current: &CurrentContractState) -> Result<CurrentContractState, AppendAuthorityError> {
        reject_stale_full_state_rewind(current, self.scope, &self.precursor, &self.target)?;
        match self.scope {
            AppendCommandScope::ReviewLedger => {
                apply_review_scope_patch(current, &self.target)
            }
            AppendCommandScope::ReuseGovernance => {
                apply_reuse_scope_patch(current, &self.target)
            }
            AppendCommandScope::ActiveAnalysis => {
                apply_analysis_scope_patch(current, &self.target)
            }
        }
    }
}

pub struct AppendScopedPreconditionCandidateAdapter {
    inner: AppendAuthoritativeCandidateAdapter,
}

impl AppendScopedPreconditionCandidateAdapter {
    pub fn new(storage_root: impl Into<std::path::PathBuf>) -> Result<Self, AppendAuthorityError> {
        Ok(Self {
            inner: AppendAuthoritativeCandidateAdapter::new(storage_root)?,
        })
    }

    pub fn candidate_id(&self) -> &'static str {
        APPEND_AUTHORITATIVE_CANDIDATE_ID
    }

    pub fn candidate_version(&self) -> &'static str {
        APPEND_SCOPED_PRECONDITION_CANDIDATE_VERSION
    }

    pub fn create(
        &self,
        state: &CurrentContractState,
    ) -> Result<AppendAuthoritySession, AppendAuthorityError> {
        self.inner.create(state)
    }

    pub fn open_existing(
        &self,
        session_id: impl Into<String>,
        mode: AppendOpenMode,
    ) -> Result<OpenedAppendAuthoritySession, AppendAuthorityError> {
        self.inner.open_existing(session_id, mode)
    }

    pub fn close(&self, opened: OpenedAppendAuthoritySession) -> Result<(), AppendAuthorityError> {
        self.inner.close(opened)
    }

    pub fn duplicate(
        &self,
        source: &mut OpenedAppendAuthoritySession,
        new_session_id: impl Into<String>,
    ) -> Result<AppendAuthoritySession, AppendAuthorityError> {
        self.inner.duplicate(source, new_session_id)
    }

    pub fn set_format_version_for_test(
        &self,
        opened: &mut OpenedAppendAuthoritySession,
        format_version: u32,
    ) -> Result<(), AppendAuthorityError> {
        self.inner.set_format_version_for_test(opened, format_version)
    }

    pub fn tamper_committed_state_for_test(
        &self,
        opened: &mut OpenedAppendAuthoritySession,
        from: &str,
        to: &str,
    ) -> Result<(), AppendAuthorityError> {
        self.inner
            .tamper_committed_state_for_test(opened, from, to)
    }

    pub fn apply_scoped_command(
        &self,
        opened: &mut OpenedAppendAuthoritySession,
        command: &AppendScopedCommand,
    ) -> Result<DurableAppendAck, AppendAuthorityError> {
        if !opened.is_writable() {
            return Err(AppendAuthorityError::new(
                "not-authoritative-writer",
                "scoped command apply requires a writable open",
            ));
        }
        self.inner.validate_writer_lock_for_scoped_apply(opened)?;
        let mut manifest = read_manifest(&opened.session)?;
        validate_manifest(&opened.session, &manifest, AppendOpenMode::Writable)?;
        let mut current = replay(&opened.session)?;
        if current.tail_status != super::append_authoritative::AppendTailStatus::Clean {
            recover_committed_prefix(&opened.session, &current)?;
            current = replay(&opened.session)?;
        }
        if manifest.committed_sequence > current.committed_sequence {
            return Err(AppendAuthorityError::new(
                "manifest-ahead-of-append-authority",
                "manifest claims an uncommitted authoritative boundary",
            ));
        }
        if manifest.committed_sequence < current.committed_sequence {
            repair_manifest_checkpoint(&opened.session, &manifest, current.committed_sequence)?;
            manifest.committed_sequence = current.committed_sequence;
            opened.checkpoint_status = AppendCheckpointStatus::Current;
        }
        validate_scoped_preconditions(&current.state, command.scope, &command.preconditions)?;
        let next_state = command.apply_onto(&current.state)?;
        validate_state(&next_state)?;
        if next_state.session_id != opened.session.session_id() {
            return Err(AppendAuthorityError::new(
                "session-identity-transition",
                "scoped command cannot change the session identity",
            ));
        }
        let next_fingerprint = canonical_fingerprint(&next_state.canonical_projection());
        if current.canonical_fingerprints.contains(&next_fingerprint) {
            return Err(AppendAuthorityError::new(
                "semantic-duplicate-transition",
                "a scoped command must add distinct canonical authority",
            ));
        }
        let acknowledgement = self.inner.append_committed_state_for_scoped_apply(
            &opened.session,
            current.committed_sequence,
            current.record_count,
            &next_state,
            &manifest,
            false,
        )?;
        opened.committed_sequence = acknowledgement.committed_sequence;
        opened.tail_status = super::append_authoritative::AppendTailStatus::Clean;
        opened.checkpoint_status = acknowledgement.checkpoint_status.clone();
        opened.set_normalized_state(next_state);
        Ok(acknowledgement)
    }

    /// Disqualified 01B-3 path: reject caller-supplied full authority replacement.
    pub fn reject_full_state_authority_replace(
        current: &CurrentContractState,
        proposed: &CurrentContractState,
    ) -> Result<(), AppendAuthorityError> {
        if proposed.durable_command_tokens.reuse_governance_head
            < current.durable_command_tokens.reuse_governance_head
        {
            return Err(AppendAuthorityError::new(
                "stale-full-state-unrelated-rewind",
                "full-state proposal would rewind reuse-governance authority",
            ));
        }
        if proposed.durable_command_tokens.review_ledger_head
            < current.durable_command_tokens.review_ledger_head
        {
            return Err(AppendAuthorityError::new(
                "stale-full-state-unrelated-rewind",
                "full-state proposal would rewind review-ledger authority",
            ));
        }
        Ok(())
    }
}

pub fn infer_command_scope(
    precursor: &CurrentContractState,
    target: &CurrentContractState,
) -> Result<AppendCommandScope, AppendAuthorityError> {
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
        (1, true, false, false) => Ok(AppendCommandScope::ReviewLedger),
        (1, false, true, false) => Ok(AppendCommandScope::ReuseGovernance),
        (1, false, false, true) => Ok(AppendCommandScope::ActiveAnalysis),
        _ => Err(AppendAuthorityError::new(
            "ambiguous-scoped-command",
            "transition must change exactly one scoped write-set",
        )),
    }
}

fn validate_scoped_preconditions(
    current: &CurrentContractState,
    scope: AppendCommandScope,
    preconditions: &AppendScopedPreconditions,
) -> Result<(), AppendAuthorityError> {
    match scope {
        AppendCommandScope::ReviewLedger => {
            if current.durable_command_tokens.review_ledger_head
                != preconditions.review_ledger_head
            {
                return Err(AppendAuthorityError::new(
                    "stale-review-ledger-precondition",
                    "review-ledger head precondition is stale",
                ));
            }
        }
        AppendCommandScope::ReuseGovernance => {
            if current.durable_command_tokens.reuse_governance_head
                != preconditions.reuse_governance_head
            {
                return Err(AppendAuthorityError::new(
                    "stale-reuse-governance-precondition",
                    "reuse-governance head precondition is stale",
                ));
            }
        }
        AppendCommandScope::ActiveAnalysis => {
            if current.durable_command_tokens.active_analysis_snapshot_identity
                != preconditions.active_analysis_snapshot_identity
            {
                return Err(AppendAuthorityError::new(
                    "stale-analysis-selection-precondition",
                    "active analysis selection precondition is stale",
                ));
            }
        }
    }
    Ok(())
}

fn reject_stale_full_state_rewind(
    current: &CurrentContractState,
    scope: AppendCommandScope,
    precursor: &CurrentContractState,
    target: &CurrentContractState,
) -> Result<(), AppendAuthorityError> {
    if scope != AppendCommandScope::ReviewLedger
        && current.durable_command_tokens.reuse_governance_head
            > precursor.durable_command_tokens.reuse_governance_head
        && target.durable_command_tokens.reuse_governance_head
            == precursor.durable_command_tokens.reuse_governance_head
    {
        return Err(AppendAuthorityError::new(
            "stale-full-state-unrelated-rewind",
            "command target would rewind unrelated reuse-governance authority",
        ));
    }
    if scope != AppendCommandScope::ReuseGovernance
        && current.durable_command_tokens.review_ledger_head
            > precursor.durable_command_tokens.review_ledger_head
        && target.durable_command_tokens.review_ledger_head
            == precursor.durable_command_tokens.review_ledger_head
    {
        return Err(AppendAuthorityError::new(
            "stale-full-state-unrelated-rewind",
            "command target would rewind unrelated review-ledger authority",
        ));
    }
    Ok(())
}

fn apply_review_scope_patch(
    current: &CurrentContractState,
    target: &CurrentContractState,
) -> Result<CurrentContractState, AppendAuthorityError> {
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
) -> Result<CurrentContractState, AppendAuthorityError> {
    let mut result = current.clone();
    result.reuse_governance_events = target.reuse_governance_events.clone();
    result.effective_reusable_records = target.effective_reusable_records.clone();
    result.historical_reusable_records = target.historical_reusable_records.clone();
    result.reusable_snapshot_identity = target.reusable_snapshot_identity.clone();
    result.reuse_enabled_analysis_binding = target.reuse_enabled_analysis_binding.clone();
    result.durable_command_tokens.reuse_governance_head =
        target.durable_command_tokens.reuse_governance_head;
    finalize_and_validate(&mut result)
}

fn apply_analysis_scope_patch(
    current: &CurrentContractState,
    target: &CurrentContractState,
) -> Result<CurrentContractState, AppendAuthorityError> {
    let mut result = current.clone();
    result.analysis_snapshots = target.analysis_snapshots.clone();
    result.durable_command_tokens.active_analysis_snapshot_identity = target
        .durable_command_tokens
        .active_analysis_snapshot_identity
        .clone();
    finalize_and_validate(&mut result)
}

fn finalize_and_validate(state: &mut CurrentContractState) -> Result<CurrentContractState, AppendAuthorityError> {
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
            AppendCommandScope::ReviewLedger
        );
        let (precursor, target) = measurement_transition_states(
            "append_reusable_revocation",
            MeasurementFixtureScale::Small,
        )
        .expect("reuse transition");
        assert_eq!(
            infer_command_scope(&precursor, &target).expect("scope"),
            AppendCommandScope::ReuseGovernance
        );
    }
}

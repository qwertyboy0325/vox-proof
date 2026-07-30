use std::fmt;

use crate::application_service::{DeclaredSessionAuthority, DeclaredSessionOperatorRole};
use crate::candidate::DetectionError;
use crate::pipeline::{CanonicalTermReviewRun, ReuseEnabledTermReviewRun};
use crate::reusable_influence::{
    EffectiveReusableInfluenceRecord, GovernanceActorContext, ReusableGovernanceEvent,
    ReusableInfluenceEffectiveState, ReusableInfluenceError, ReusableInfluenceLedger,
    ReusableInfluenceSnapshot, ReuseCandidate, ReuseCandidateKey, build_reusable_influence_snapshot,
    derive_reuse_candidates, fold_effective_state, resolve_exact_input_projection,
    verify_source_locator_against_ledger,
};
use crate::reuse_primitives::{
    ProjectScope, ProjectScopeDisplayName, ProjectScopeId, ProjectScopeTextError,
    ReusableInfluenceRecordId,
};
use crate::review::ReviewLedger;
use crate::transcript::Transcript;

#[derive(Copy, Clone)]
pub struct ReuseSessionParts<'a> {
    pub transcript: &'a Transcript,
    pub session_terms: &'a [crate::candidate::SessionTermEntry],
    pub canonical_run: &'a CanonicalTermReviewRun,
    pub ledger: &'a ReviewLedger,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ApplicationReuseError {
    Reuse(ReusableInfluenceError),
    Detection(DetectionError),
    MissingProjectScope,
    ProjectScopeFrozen,
}

impl fmt::Display for ApplicationReuseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ApplicationReuseError {}

impl From<ReusableInfluenceError> for ApplicationReuseError {
    fn from(value: ReusableInfluenceError) -> Self {
        Self::Reuse(value)
    }
}

impl From<ProjectScopeTextError> for ApplicationReuseError {
    fn from(_value: ProjectScopeTextError) -> Self {
        Self::Reuse(ReusableInfluenceError::InvalidSourceLocator)
    }
}

pub struct ApplicationReuseState {
    pub project_scope: Option<ProjectScope>,
    pub governance_ledger: ReusableInfluenceLedger,
}

impl Default for ApplicationReuseState {
    fn default() -> Self {
        Self {
            project_scope: None,
            governance_ledger: ReusableInfluenceLedger::new(),
        }
    }
}

impl ApplicationReuseState {
    pub fn effective_state(&self, review_ledger: &ReviewLedger) -> ReusableInfluenceEffectiveState {
        fold_effective_state(&self.governance_ledger, review_ledger)
    }
}

pub fn governance_actor_from_authority(
    authority: &DeclaredSessionAuthority,
) -> GovernanceActorContext {
    let role_label = match authority.role() {
        DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator => {
            "declared_local_owner_operator".to_owned()
        }
        DeclaredSessionOperatorRole::DeclaredAuthorizedHumanReviewer => {
            "declared_authorized_human_reviewer".to_owned()
        }
    };
    GovernanceActorContext {
        role_label,
        display_label: authority.display_label().to_owned(),
    }
}

pub fn initialize_project_scope(
    reuse_state: &mut ApplicationReuseState,
    stable_id: impl Into<String>,
    display_name: impl Into<String>,
) -> Result<(), ApplicationReuseError> {
    if reuse_state.project_scope.is_some() {
        return Err(ApplicationReuseError::ProjectScopeFrozen);
    }
    let stable_id = ProjectScopeId::new(stable_id)?;
    let display_name = ProjectScopeDisplayName::new(display_name)?;
    reuse_state.project_scope = Some(ProjectScope::new(stable_id, display_name));
    Ok(())
}

pub fn update_project_scope_display_name(
    reuse_state: &mut ApplicationReuseState,
    display_name: impl Into<String>,
) -> Result<(), ApplicationReuseError> {
    let scope = reuse_state
        .project_scope
        .as_mut()
        .ok_or(ApplicationReuseError::MissingProjectScope)?;
    scope.display_name = ProjectScopeDisplayName::new(display_name)?;
    Ok(())
}

pub fn reuse_candidates_for_parts(
    parts: ReuseSessionParts<'_>,
    reuse_state: &ApplicationReuseState,
) -> Result<Vec<ReuseCandidate>, ApplicationReuseError> {
    let project_scope = reuse_state
        .project_scope
        .as_ref()
        .ok_or(ApplicationReuseError::MissingProjectScope)?;
    let effective = reuse_state.effective_state(parts.ledger);
    derive_reuse_candidates(
        parts.transcript,
        parts.canonical_run,
        parts.ledger,
        project_scope,
        &effective,
    )
    .map_err(ApplicationReuseError::from)
}

pub fn reusable_influence_snapshot_for_parts(
    parts: ReuseSessionParts<'_>,
    reuse_state: &ApplicationReuseState,
) -> Result<ReusableInfluenceSnapshot, ApplicationReuseError> {
    let project_scope = reuse_state
        .project_scope
        .as_ref()
        .ok_or(ApplicationReuseError::MissingProjectScope)?;
    let effective = reuse_state.effective_state(parts.ledger);
    Ok(build_reusable_influence_snapshot(
        project_scope,
        &reuse_state.governance_ledger,
        &effective,
    ))
}

pub fn accept_reuse_candidate(
    parts: ReuseSessionParts<'_>,
    reuse_state: &mut ApplicationReuseState,
    authority: &DeclaredSessionAuthority,
    candidate_key: &ReuseCandidateKey,
) -> Result<ReusableInfluenceRecordId, ApplicationReuseError> {
    let project_scope = reuse_state
        .project_scope
        .as_ref()
        .ok_or(ApplicationReuseError::MissingProjectScope)?;
    if candidate_key.project_scope_id != project_scope.stable_id {
        return Err(ReusableInfluenceError::WrongProjectScope.into());
    }
    let effective = reuse_state.effective_state(parts.ledger);
    if effective.rejected_candidate_keys.contains(candidate_key) {
        return Err(ReusableInfluenceError::CandidateAlreadyRejected.into());
    }
    let candidates = derive_reuse_candidates(
        parts.transcript,
        parts.canonical_run,
        parts.ledger,
        project_scope,
        &effective,
    )?;
    let Some(candidate) = candidates
        .into_iter()
        .find(|item| &item.key == candidate_key)
    else {
        return Err(ReusableInfluenceError::UnknownCandidate.into());
    };
    if effective
        .active_records
        .iter()
        .any(|record| record.source_locator == candidate.key.source_locator)
    {
        return Err(ReusableInfluenceError::CandidateAlreadyPromoted.into());
    }
    let review_case = parts
        .canonical_run
        .review_cases()
        .get(
            candidate
                .key
                .source_locator
                .source_review_case_id
                .local_index(),
        )
        .ok_or(ReusableInfluenceError::InvalidSourceLocator)?;
    verify_source_locator_against_ledger(
        parts.ledger,
        &candidate.key.source_locator,
        &candidate.exact_payload.confirmed_replacement,
        &candidate.exact_payload.observed_text,
        parts.transcript,
        parts.canonical_run,
        review_case,
    )?;

    let event_index =
        reuse_state
            .governance_ledger
            .append(ReusableGovernanceEvent::PromotionAccepted {
                candidate_key: Box::new(candidate.key.clone()),
                payload: candidate.exact_payload.clone(),
                source_locator: Box::new(candidate.key.source_locator.clone()),
                actor: governance_actor_from_authority(authority),
                project_scope: Box::new(project_scope.clone()),
            });
    Ok(ReusableInfluenceRecordId::from_promotion_event_index(
        event_index,
    ))
}

pub fn reject_reuse_candidate(
    parts: ReuseSessionParts<'_>,
    reuse_state: &mut ApplicationReuseState,
    authority: &DeclaredSessionAuthority,
    candidate_key: &ReuseCandidateKey,
) -> Result<(), ApplicationReuseError> {
    let project_scope = reuse_state
        .project_scope
        .as_ref()
        .ok_or(ApplicationReuseError::MissingProjectScope)?;
    if candidate_key.project_scope_id != project_scope.stable_id {
        return Err(ReusableInfluenceError::WrongProjectScope.into());
    }
    let effective = reuse_state.effective_state(parts.ledger);
    let candidates = derive_reuse_candidates(
        parts.transcript,
        parts.canonical_run,
        parts.ledger,
        project_scope,
        &effective,
    )?;
    if !candidates.iter().any(|item| &item.key == candidate_key) {
        return Err(ReusableInfluenceError::UnknownCandidate.into());
    }
    reuse_state
        .governance_ledger
        .append(ReusableGovernanceEvent::PromotionCandidateRejected {
            candidate_key: Box::new(candidate_key.clone()),
            actor: governance_actor_from_authority(authority),
        });
    Ok(())
}

pub fn revoke_reusable_influence(
    reuse_state: &mut ApplicationReuseState,
    review_ledger: &ReviewLedger,
    authority: &DeclaredSessionAuthority,
    record_id: ReusableInfluenceRecordId,
) -> Result<(), ApplicationReuseError> {
    let effective = reuse_state.effective_state(review_ledger);
    if !effective
        .active_records
        .iter()
        .any(|record| record.record_id == record_id)
    {
        return Err(ReusableInfluenceError::RecordNotActive { record_id }.into());
    }
    reuse_state
        .governance_ledger
        .append(ReusableGovernanceEvent::ReusableInfluenceRevoked {
            record_id,
            actor: governance_actor_from_authority(authority),
        });
    Ok(())
}

pub fn supersede_reusable_influence(
    parts: ReuseSessionParts<'_>,
    reuse_state: &mut ApplicationReuseState,
    authority: &DeclaredSessionAuthority,
    predecessor_id: ReusableInfluenceRecordId,
    successor_candidate_key: &ReuseCandidateKey,
) -> Result<ReusableInfluenceRecordId, ApplicationReuseError> {
    let effective = reuse_state.effective_state(parts.ledger);
    if !effective
        .active_records
        .iter()
        .any(|record| record.record_id == predecessor_id)
    {
        return Err(ReusableInfluenceError::RecordNotActive {
            record_id: predecessor_id,
        }
        .into());
    }
    let successor_id =
        accept_reuse_candidate(parts, reuse_state, authority, successor_candidate_key)?;
    if successor_id == predecessor_id {
        return Err(ReusableInfluenceError::SelfSupersession.into());
    }
    reuse_state
        .governance_ledger
        .append(ReusableGovernanceEvent::ReusableInfluenceSuperseded {
            predecessor_id,
            successor_id,
            actor: governance_actor_from_authority(authority),
        });
    Ok(successor_id)
}

pub fn run_reuse_enabled_review_for_parts(
    parts: ReuseSessionParts<'_>,
    reuse_state: &ApplicationReuseState,
) -> Result<ReuseEnabledTermReviewRun, ApplicationReuseError> {
    let project_scope = reuse_state
        .project_scope
        .as_ref()
        .ok_or(ApplicationReuseError::MissingProjectScope)?;
    let snapshot = reusable_influence_snapshot_for_parts(parts, reuse_state)?;
    let projection = resolve_exact_input_projection(project_scope, &snapshot, parts.session_terms)?;
    crate::pipeline::run_reuse_enabled_term_review(
        parts.transcript,
        parts.session_terms,
        &projection,
        &snapshot,
    )
    .map_err(ApplicationReuseError::Detection)
}

pub fn active_reusable_records(
    parts: ReuseSessionParts<'_>,
    reuse_state: &ApplicationReuseState,
) -> Result<Vec<EffectiveReusableInfluenceRecord>, ApplicationReuseError> {
    let project_scope = reuse_state
        .project_scope
        .as_ref()
        .ok_or(ApplicationReuseError::MissingProjectScope)?;
    let effective = reuse_state.effective_state(parts.ledger);
    Ok(effective
        .active_records
        .iter()
        .filter(|record| record.project_scope.stable_id == project_scope.stable_id)
        .cloned()
        .collect())
}

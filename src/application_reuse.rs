use std::fmt;

use crate::application_service::{DeclaredSessionAuthority, DeclaredSessionOperatorRole};
use crate::candidate::DetectionError;
use crate::pipeline::{CanonicalTermReviewRun, ReuseEnabledTermReviewRun};
use crate::reusable_influence::{
    EffectiveReusableInfluenceRecord, GovernanceActorContext, ReusableGovernanceEvent,
    ReusableInfluenceEffectiveState, ReusableInfluenceError, ReusableInfluenceLedger,
    ReusableInfluenceSnapshot, ReuseCandidate, ReuseCandidateKey,
    build_reusable_influence_snapshot, derive_reuse_candidates, fold_effective_state,
    resolve_exact_input_projection, verify_source_locator_against_ledger,
};
use crate::reuse_primitives::{
    ProjectScope, ProjectScopeDisplayName, ProjectScopeId, ProjectScopeTextError,
    PromotionCandidateRejectionIdentity, ReusableInfluenceRecordId,
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
    project_scope: Option<ProjectScope>,
    governance_ledger: ReusableInfluenceLedger,
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
    pub fn project_scope(&self) -> Option<&ProjectScope> {
        self.project_scope.as_ref()
    }

    pub fn governance_events(&self) -> &[ReusableGovernanceEvent] {
        self.governance_ledger.events()
    }

    pub fn governance_ledger(&self) -> &ReusableInfluenceLedger {
        &self.governance_ledger
    }

    pub(crate) fn governance_ledger_mut(&mut self) -> &mut ReusableInfluenceLedger {
        &mut self.governance_ledger
    }

    pub(crate) fn from_replayed_governance(
        project_scope: ProjectScope,
        governance_ledger: &ReusableInfluenceLedger,
    ) -> Self {
        let mut state = Self {
            project_scope: Some(project_scope),
            governance_ledger: ReusableInfluenceLedger::new(),
        };
        for event in governance_ledger.events() {
            state.governance_ledger_mut().append(event.clone());
        }
        state
    }

    pub fn effective_state(
        &self,
        review_ledger: &ReviewLedger,
        canonical_run: &CanonicalTermReviewRun,
    ) -> ReusableInfluenceEffectiveState {
        fold_effective_state(&self.governance_ledger, review_ledger, canonical_run)
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
    let effective = reuse_state.effective_state(parts.ledger, parts.canonical_run);
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
    let effective = reuse_state.effective_state(parts.ledger, parts.canonical_run);
    build_reusable_influence_snapshot(project_scope, reuse_state.governance_ledger(), &effective)
        .map_err(ApplicationReuseError::from)
}

pub fn validate_accept_reuse_candidate(
    parts: ReuseSessionParts<'_>,
    reuse_state: &ApplicationReuseState,
    candidate_key: &ReuseCandidateKey,
) -> Result<ReuseCandidate, ApplicationReuseError> {
    let project_scope = reuse_state
        .project_scope()
        .ok_or(ApplicationReuseError::MissingProjectScope)?;
    if candidate_key.project_scope_id != project_scope.stable_id {
        return Err(ReusableInfluenceError::WrongProjectScope.into());
    }
    let effective = reuse_state.effective_state(parts.ledger, parts.canonical_run);
    if effective
        .rejected_candidate_identities()
        .contains(&PromotionCandidateRejectionIdentity::from(candidate_key))
    {
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
        .active_records()
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
    Ok(candidate)
}

pub fn build_promotion_accepted_event(
    candidate: &ReuseCandidate,
    project_scope: &ProjectScope,
    authority: &DeclaredSessionAuthority,
) -> ReusableGovernanceEvent {
    ReusableGovernanceEvent::PromotionAccepted {
        candidate_key: Box::new(candidate.key.clone()),
        payload: candidate.exact_payload.clone(),
        source_locator: Box::new(candidate.key.source_locator.clone()),
        actor: governance_actor_from_authority(authority),
        project_scope: Box::new(project_scope.clone()),
    }
}

pub fn accept_reuse_candidate(
    parts: ReuseSessionParts<'_>,
    reuse_state: &mut ApplicationReuseState,
    authority: &DeclaredSessionAuthority,
    candidate_key: &ReuseCandidateKey,
) -> Result<ReusableInfluenceRecordId, ApplicationReuseError> {
    let project_scope = reuse_state
        .project_scope()
        .ok_or(ApplicationReuseError::MissingProjectScope)?
        .clone();
    let candidate = validate_accept_reuse_candidate(parts, reuse_state, candidate_key)?;

    let event_index = reuse_state
        .governance_ledger_mut()
        .append(build_promotion_accepted_event(
            &candidate,
            &project_scope,
            authority,
        ));
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
    let effective = reuse_state.effective_state(parts.ledger, parts.canonical_run);
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
    reuse_state.governance_ledger_mut().append(
        ReusableGovernanceEvent::PromotionCandidateRejected {
            candidate_key: Box::new(candidate_key.clone()),
            actor: governance_actor_from_authority(authority),
        },
    );
    Ok(())
}

pub fn revoke_reusable_influence(
    reuse_state: &mut ApplicationReuseState,
    review_ledger: &ReviewLedger,
    canonical_run: &CanonicalTermReviewRun,
    authority: &DeclaredSessionAuthority,
    record_id: ReusableInfluenceRecordId,
) -> Result<(), ApplicationReuseError> {
    let effective = reuse_state.effective_state(review_ledger, canonical_run);
    if !effective
        .active_records()
        .iter()
        .any(|record| record.record_id == record_id)
    {
        return Err(ReusableInfluenceError::RecordNotActive { record_id }.into());
    }
    reuse_state
        .governance_ledger_mut()
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
    let effective = reuse_state.effective_state(parts.ledger, parts.canonical_run);
    if !effective
        .active_records()
        .iter()
        .any(|record| record.record_id == predecessor_id)
    {
        return Err(ReusableInfluenceError::RecordNotActive {
            record_id: predecessor_id,
        }
        .into());
    }
    let project_scope = reuse_state
        .project_scope()
        .ok_or(ApplicationReuseError::MissingProjectScope)?
        .clone();
    let candidate = validate_accept_reuse_candidate(parts, reuse_state, successor_candidate_key)?;
    let promotion_index = reuse_state.governance_ledger().events().len();
    let successor_id = ReusableInfluenceRecordId::from_promotion_event_index(promotion_index);
    if successor_id == predecessor_id {
        return Err(ReusableInfluenceError::SelfSupersession.into());
    }
    let events = vec![
        build_promotion_accepted_event(&candidate, &project_scope, authority),
        ReusableGovernanceEvent::ReusableInfluenceSuperseded {
            predecessor_id,
            successor_id,
            actor: governance_actor_from_authority(authority),
        },
    ];
    reuse_state.governance_ledger_mut().append_batch(events);
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
    let effective = reuse_state.effective_state(parts.ledger, parts.canonical_run);
    Ok(effective
        .active_records()
        .iter()
        .filter(|record| record.project_scope.stable_id == project_scope.stable_id)
        .cloned()
        .collect())
}

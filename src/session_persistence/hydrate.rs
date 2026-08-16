use std::cell::Cell;

use crate::application_gate3_replay::replay_reuse_governance_for_hydrate;
use crate::application_reuse::{
    ApplicationReuseState, ReuseSessionParts, run_reuse_enabled_review_for_parts,
};
use crate::application_service::{
    ApplicationMaterialUseDeclaration, ApplicationReviewSession,
    DeclaredApplicationMaterialUseBasis, DeclaredSessionAuthority, DeclaredSessionOperatorRole,
    ProjectReuseSessionState, ProjectTerminologySessionState, assemble_application_review_session,
};
use crate::pipeline::{CanonicalTermReviewRun, ReuseEnabledTermReviewRun};
use crate::project_memory::{
    ProjectMemoryRecord, ProjectMemorySnapshotIdentity, compute_project_memory_snapshot_identity,
    required_project_memory_format_version,
};
use crate::reuse_primitives::ProjectScope;
use crate::review::ReviewLedger;
use crate::session_persistence::canonical::{
    restore_frozen_project_reuse, restore_human_raised_cases, restore_ledger_event,
    restore_project_terminology_proposal_target, restore_reuse_proposal_target,
    restore_session_terms, restore_transcript, session_format_supports_human_raised,
    session_format_supports_project_binding, session_format_supports_project_terminology,
    verify_analysis_snapshot, verify_review_cases,
};
use crate::session_persistence::error::SessionPersistenceError;
use crate::session_persistence::reuse_canonical::{
    project_scope_is_uninitialized, restore_governance_event, restore_project_scope,
    restore_reusable_snapshot_identity,
};
use crate::session_persistence::reuse_store::find_active_binding;
use crate::session_persistence::store::{OpenedStoreSession, ProductSessionStore};

thread_local! {
    static FORCE_HYDRATE_FAILURE: Cell<bool> = const { Cell::new(false) };
}

#[doc(hidden)]
pub fn arm_force_hydrate_failure_for_test() {
    FORCE_HYDRATE_FAILURE.with(|flag| flag.set(true));
}

#[doc(hidden)]
pub fn disarm_force_hydrate_failure_for_test() {
    FORCE_HYDRATE_FAILURE.with(|flag| flag.set(false));
}

pub(crate) struct ProjectMemoryHydrateOverlay {
    pub available: bool,
    pub project_scope: ProjectScope,
    pub records: Vec<ProjectMemoryRecord>,
    pub current_snapshot: Option<ProjectMemorySnapshotIdentity>,
}

pub(crate) fn hydrate_application_review_session(
    opened: &OpenedStoreSession,
    project_overlay: Option<&ProjectMemoryHydrateOverlay>,
) -> Result<ApplicationReviewSession, SessionPersistenceError> {
    if FORCE_HYDRATE_FAILURE.with(|flag| flag.get()) {
        return Err(SessionPersistenceError::CanonicalMismatch(
            "forced hydrate failure".to_owned(),
        ));
    }
    let capture = ProductSessionStore::load_canonical_capture(opened)?;
    let transcript = restore_transcript(&capture.transcript);
    let session_terms = restore_session_terms(&capture.session_terms);
    let detector_run = run_canonical_term_review(&transcript, &session_terms).map_err(|error| {
        SessionPersistenceError::Replay(
            crate::application_service::ApplicationServiceError::Detection(error),
        )
    })?;
    verify_analysis_snapshot(
        &capture.analysis_snapshot,
        detector_run.analysis_run().snapshot(),
    )?;
    let authoritative_cases = verify_review_cases(
        &capture.review_cases,
        &detector_run,
        transcript.revision_id(),
    )?;
    let canonical_run =
        CanonicalTermReviewRun::new(detector_run.analysis_run(), authoritative_cases);
    let material_use = restore_material_use(&capture.material_use_basis)?;
    let session_authority =
        restore_session_authority(&capture.authority_role, &capture.authority_display_label)?;
    let mut persisted_targets = Vec::new();
    for persisted in &capture.reuse_proposal_targets {
        persisted_targets.push(restore_reuse_proposal_target(persisted)?);
    }
    let mut persisted_terminology_targets = Vec::new();
    for persisted in &capture.terminology_proposal_targets {
        persisted_terminology_targets.push(restore_project_terminology_proposal_target(persisted)?);
    }
    let frozen = capture
        .frozen_project_reuse
        .as_ref()
        .map(restore_frozen_project_reuse)
        .transpose()?;
    if !capture.human_raised_cases.is_empty()
        && !session_format_supports_human_raised(capture.format_version)
    {
        return Err(SessionPersistenceError::CanonicalMismatch(
            "human-raised cases require session format v3 or v4".to_owned(),
        ));
    }
    if !capture.terminology_proposal_targets.is_empty()
        && !session_format_supports_project_terminology(capture.format_version)
    {
        return Err(SessionPersistenceError::CanonicalMismatch(
            "project terminology targets require session format v4".to_owned(),
        ));
    }
    let human_raised_cases = restore_human_raised_cases(&capture.human_raised_cases, &transcript)?;
    let mut ledger = ReviewLedger::new();
    for persisted_event in &capture.ledger_events {
        let event = restore_ledger_event(persisted_event, transcript.revision_id())?;
        match event {
            crate::review::ReviewLedgerEvent::DecisionRecorded {
                case_id,
                observed_revision,
                decision,
            } => {
                let review_case = if case_id.is_human_raised() {
                    human_raised_cases.get(case_id.local_index())
                } else {
                    canonical_run.review_cases().get(case_id.local_index())
                }
                .filter(|review_case| review_case.id() == case_id)
                .ok_or_else(|| {
                    SessionPersistenceError::CanonicalMismatch("unknown review case".to_owned())
                })?;
                ledger
                    .record_decision(review_case, observed_revision, decision)
                    .map_err(|error| {
                        SessionPersistenceError::Replay(
                            crate::application_service::ApplicationServiceError::Decision(error),
                        )
                    })?;
            }
            crate::review::ReviewLedgerEvent::ReuseProposalDecisionRecorded {
                target_identity,
                observed_revision,
                decision,
            } => {
                if !session_format_supports_project_binding(capture.format_version) {
                    return Err(SessionPersistenceError::CanonicalMismatch(
                        "v1 session must not contain reuse proposal decisions".to_owned(),
                    ));
                }
                if !persisted_targets
                    .iter()
                    .any(|target| target.identity() == target_identity)
                {
                    return Err(SessionPersistenceError::CanonicalMismatch(
                        "reuse decision missing thin target".to_owned(),
                    ));
                }
                ledger
                    .record_reuse_decision(target_identity, observed_revision, decision)
                    .map_err(|error| {
                        SessionPersistenceError::Replay(
                            crate::application_service::ApplicationServiceError::Decision(error),
                        )
                    })?;
            }
            crate::review::ReviewLedgerEvent::CaseRaised {
                case_id,
                observed_revision,
                ..
            } => {
                if !session_format_supports_human_raised(capture.format_version) {
                    return Err(SessionPersistenceError::CanonicalMismatch(
                        "CaseRaised requires session format v3 or v4".to_owned(),
                    ));
                }
                let review_case = human_raised_cases
                    .iter()
                    .find(|case| case.id() == case_id)
                    .ok_or_else(|| {
                        SessionPersistenceError::CanonicalMismatch(
                            "unknown human-raised case".to_owned(),
                        )
                    })?;
                ledger
                    .record_case_raised(review_case, observed_revision)
                    .map_err(|error| {
                        SessionPersistenceError::Replay(
                            crate::application_service::ApplicationServiceError::Decision(error),
                        )
                    })?;
            }
            crate::review::ReviewLedgerEvent::TerminologyProposalDecisionRecorded {
                target_identity,
                observed_revision,
                decision,
            } => {
                if !session_format_supports_project_terminology(capture.format_version) {
                    return Err(SessionPersistenceError::CanonicalMismatch(
                        "terminology proposal decisions require session format v4".to_owned(),
                    ));
                }
                if !persisted_terminology_targets
                    .iter()
                    .any(|target| target.identity() == target_identity)
                {
                    return Err(SessionPersistenceError::CanonicalMismatch(
                        "terminology decision missing thin target".to_owned(),
                    ));
                }
                ledger
                    .record_terminology_decision(target_identity, observed_revision, decision)
                    .map_err(|error| {
                        SessionPersistenceError::Replay(
                            crate::application_service::ApplicationServiceError::Decision(error),
                        )
                    })?;
            }
        }
    }
    if ledger.events().len() != capture.review_ledger_head {
        return Err(SessionPersistenceError::CanonicalMismatch(
            "review ledger head mismatch".to_owned(),
        ));
    }
    if capture.reuse_governance_events.len() != capture.reuse_governance_head {
        return Err(SessionPersistenceError::CanonicalMismatch(
            "reuse governance head mismatch".to_owned(),
        ));
    }

    let reuse_state = restore_reuse_state(
        &capture,
        &session_authority,
        &transcript,
        &session_terms,
        &canonical_run,
        &human_raised_cases,
        &ledger,
        project_overlay,
    )?;
    let reuse_enabled_run = if project_overlay.is_some_and(|overlay| !overlay.available) {
        None
    } else {
        reconstruct_reuse_enabled_run(
            &capture,
            &transcript,
            &session_terms,
            &canonical_run,
            &human_raised_cases,
            &ledger,
            &reuse_state,
            frozen.as_ref(),
        )?
    };

    if capture.bound_project_id.is_some() {
        if let (Some(run), Some(frozen_analysis), Some(scope)) = (
            reuse_enabled_run.as_ref(),
            frozen.as_ref(),
            reuse_state.project_scope(),
        ) {
            let derived = crate::reuse_proposal_target::derive_reuse_proposal_targets(
                &transcript,
                run,
                &scope.stable_id,
                frozen_analysis.project_memory_snapshot_identity,
                frozen_analysis.governance_event_boundary,
            );
            for target in &persisted_targets {
                if target.project_memory_snapshot_identity()
                    == frozen_analysis.project_memory_snapshot_identity
                    && !derived
                        .iter()
                        .any(|item| item.identity() == target.identity())
                {
                    return Err(SessionPersistenceError::CanonicalMismatch(
                        "persisted reuse target does not reconstruct from frozen analysis"
                            .to_owned(),
                    ));
                }
            }
        }
    }

    let compose_terminology = session_format_supports_project_terminology(capture.format_version)
        && frozen.is_some()
        && project_overlay.is_some_and(|overlay| overlay.available);
    if compose_terminology {
        if let (Some(frozen_analysis), Some(scope)) = (frozen.as_ref(), reuse_state.project_scope())
        {
            let parts = ReuseSessionParts {
                transcript: &transcript,
                session_terms: &session_terms,
                canonical_run: &canonical_run,
                human_raised_cases: &human_raised_cases,
                ledger: &ledger,
            };
            let records = crate::application_reuse::active_reusable_records(parts, &reuse_state)
                .map_err(|error| {
                    SessionPersistenceError::CanonicalMismatch(format!(
                        "terminology active records: {error:?}"
                    ))
                })?;
            let derived = crate::project_terminology::derive_project_terminology_proposal_targets(
                &transcript,
                &session_terms,
                &records,
                &scope.stable_id,
                frozen_analysis.project_memory_snapshot_identity,
                frozen_analysis.governance_event_boundary,
            )
            .map_err(|error| {
                SessionPersistenceError::CanonicalMismatch(format!(
                    "terminology derivation: {error:?}"
                ))
            })?;
            for target in &persisted_terminology_targets {
                if target.project_memory_snapshot_identity()
                    == frozen_analysis.project_memory_snapshot_identity
                    && !derived
                        .iter()
                        .any(|item| item.identity() == target.identity())
                {
                    return Err(SessionPersistenceError::CanonicalMismatch(
                        "persisted terminology target does not reconstruct from frozen analysis"
                            .to_owned(),
                    ));
                }
            }
        }
    }

    let project_reuse = ProjectReuseSessionState {
        compose: frozen.is_some() && project_overlay.is_some_and(|overlay| overlay.available),
        persisted_targets,
        frozen,
        project_memory_available: project_overlay
            .map(|overlay| overlay.available)
            .unwrap_or(true),
        current_project_snapshot: project_overlay.and_then(|overlay| overlay.current_snapshot),
        project_memory_records: project_overlay
            .map(|overlay| overlay.records.clone())
            .unwrap_or_default(),
    };

    let project_terminology = ProjectTerminologySessionState {
        compose: compose_terminology,
        persisted_targets: persisted_terminology_targets,
    };

    let reuse_active = !capture.reuse_governance_events.is_empty() || reuse_enabled_run.is_some();

    let session = assemble_application_review_session(
        transcript,
        session_terms,
        canonical_run,
        human_raised_cases,
        ledger,
        material_use,
        session_authority,
        reuse_state,
        reuse_enabled_run,
        project_reuse,
        project_terminology,
    )
    .map_err(SessionPersistenceError::Replay)?;

    if session_format_supports_project_binding(capture.format_version)
        || session_format_supports_human_raised(capture.format_version)
    {
        session.verify_canonical_replay().map_err(|_| {
            SessionPersistenceError::CanonicalMismatch(
                "in-memory replay verification failed".to_owned(),
            )
        })?;
    } else if reuse_active {
        crate::application_gate3_replay::verify_gate3_independent_replay(&session).map_err(
            |_| {
                SessionPersistenceError::CanonicalMismatch(
                    "gate3 replay verification failed".to_owned(),
                )
            },
        )?;
    } else {
        session.verify_in_memory_replay().map_err(|_| {
            SessionPersistenceError::CanonicalMismatch(
                "in-memory replay verification failed".to_owned(),
            )
        })?;
    }
    Ok(session)
}

fn restore_reuse_state(
    capture: &crate::session_persistence::canonical::SessionCanonicalCapture,
    session_authority: &DeclaredSessionAuthority,
    transcript: &crate::transcript::Transcript,
    session_terms: &[crate::candidate::SessionTermEntry],
    canonical_run: &CanonicalTermReviewRun,
    human_raised_cases: &[crate::review::ReviewCase],
    ledger: &ReviewLedger,
    project_overlay: Option<&ProjectMemoryHydrateOverlay>,
) -> Result<ApplicationReuseState, SessionPersistenceError> {
    if capture.bound_project_id.is_some() {
        let overlay = project_overlay.ok_or_else(|| {
            SessionPersistenceError::CanonicalMismatch(
                "project-bound session missing project overlay".to_owned(),
            )
        })?;
        if !capture.reuse_governance_events.is_empty() {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "project-bound session must not hold session-local reuse authority".to_owned(),
            ));
        }
        if overlay.available {
            let records = if let Some(frozen_persisted) = &capture.frozen_project_reuse {
                let frozen = restore_frozen_project_reuse(frozen_persisted)?;
                if overlay.records.len() < frozen.governance_event_boundary {
                    return Err(SessionPersistenceError::CanonicalMismatch(
                        "frozen project boundary exceeds available project records".to_owned(),
                    ));
                }
                let prefix = overlay.records[..frozen.governance_event_boundary].to_vec();
                let computed = compute_project_memory_snapshot_identity(
                    &overlay.project_scope.stable_id,
                    required_project_memory_format_version(&prefix),
                    frozen.governance_event_boundary,
                    &prefix,
                );
                if computed != frozen.project_memory_snapshot_identity {
                    return Err(SessionPersistenceError::CanonicalMismatch(
                        "frozen project memory snapshot mismatch".to_owned(),
                    ));
                }
                prefix
            } else {
                overlay.records.clone()
            };
            return Ok(ApplicationReuseState::from_project_memory_records(
                overlay.project_scope.clone(),
                &records,
            ));
        }
        return Ok(ApplicationReuseState::from_project_memory_records(
            overlay.project_scope.clone(),
            &[],
        ));
    }
    if project_overlay.is_some() {
        return Err(SessionPersistenceError::CanonicalMismatch(
            "unbound session must not carry project overlay".to_owned(),
        ));
    }
    if project_scope_is_uninitialized(&capture.project_scope) {
        if !capture.reuse_governance_events.is_empty() {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "reuse governance without project scope".to_owned(),
            ));
        }
        return Ok(ApplicationReuseState::default());
    }
    let project_scope = restore_project_scope(&capture.project_scope)?
        .ok_or_else(|| SessionPersistenceError::CanonicalMismatch("project scope".to_owned()))?;
    let mut governance_events = Vec::new();
    for persisted in &capture.reuse_governance_events {
        governance_events.push(restore_governance_event(persisted)?);
    }
    let parts = ReuseSessionParts {
        transcript,
        session_terms,
        canonical_run,
        human_raised_cases,
        ledger,
    };
    let replayed = replay_reuse_governance_for_hydrate(
        parts,
        &project_scope,
        &governance_events,
        session_authority,
    )
    .map_err(|error| {
        SessionPersistenceError::CanonicalMismatch(format!("governance replay: {error:?}"))
    })?;
    if replayed.events().len() != governance_events.len() {
        return Err(SessionPersistenceError::CanonicalMismatch(
            "reuse governance replay length mismatch".to_owned(),
        ));
    }
    Ok(ApplicationReuseState::from_replayed_governance(
        project_scope,
        &replayed,
    ))
}

fn reconstruct_reuse_enabled_run(
    capture: &crate::session_persistence::canonical::SessionCanonicalCapture,
    transcript: &crate::transcript::Transcript,
    session_terms: &[crate::candidate::SessionTermEntry],
    canonical_run: &CanonicalTermReviewRun,
    human_raised_cases: &[crate::review::ReviewCase],
    ledger: &ReviewLedger,
    reuse_state: &ApplicationReuseState,
    frozen: Option<&crate::reuse_proposal_target::FrozenProjectReuseAnalysis>,
) -> Result<Option<ReuseEnabledTermReviewRun>, SessionPersistenceError> {
    let parts = ReuseSessionParts {
        transcript,
        session_terms,
        canonical_run,
        human_raised_cases,
        ledger,
    };
    if capture.bound_project_id.is_some() {
        let Some(frozen) = frozen else {
            return Ok(None);
        };
        let run = run_reuse_enabled_review_for_parts(parts, reuse_state)
            .map_err(|error| SessionPersistenceError::CanonicalMismatch(format!("{error:?}")))?;
        if run.analysis_run().snapshot() != frozen.reuse_analysis_snapshot
            || run.governance_event_boundary_at_run() != frozen.governance_event_boundary
        {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "reconstructed reuse analysis does not match freeze".to_owned(),
            ));
        }
        return Ok(Some(run));
    }
    let binding = find_active_binding(capture)?;
    if binding.is_none() {
        return Ok(None);
    }
    let binding = binding.expect("checked above");
    let snapshot =
        crate::application_reuse::reusable_influence_snapshot_for_parts(parts, reuse_state)
            .map_err(|error| SessionPersistenceError::CanonicalMismatch(format!("{error:?}")))?;
    let binding_reusable_identity =
        restore_reusable_snapshot_identity(&binding.reusable_snapshot_identity)?;
    if binding_reusable_identity != snapshot.identity()
        || binding.governance_event_boundary != reuse_state.governance_events().len()
    {
        return Ok(None);
    }
    let run = run_reuse_enabled_review_for_parts(parts, reuse_state)
        .map_err(|error| SessionPersistenceError::CanonicalMismatch(format!("{error:?}")))?;
    Ok(Some(run))
}

use crate::pipeline::run_canonical_term_review;

fn restore_material_use(
    basis: &str,
) -> Result<ApplicationMaterialUseDeclaration, SessionPersistenceError> {
    let basis = match basis {
        "self_owned" => DeclaredApplicationMaterialUseBasis::SelfOwned,
        "explicit_permission" => DeclaredApplicationMaterialUseBasis::ExplicitPermission,
        _ => {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "material use basis".to_owned(),
            ));
        }
    };
    Ok(ApplicationMaterialUseDeclaration::new(basis))
}

fn restore_session_authority(
    role: &str,
    display_label: &str,
) -> Result<DeclaredSessionAuthority, SessionPersistenceError> {
    let role = match role {
        "declared_local_owner_operator" => DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
        "declared_authorized_human_reviewer" => {
            DeclaredSessionOperatorRole::DeclaredAuthorizedHumanReviewer
        }
        _ => {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "authority role".to_owned(),
            ));
        }
    };
    DeclaredSessionAuthority::new(role, display_label)
        .map_err(|error| SessionPersistenceError::CanonicalMismatch(format!("{error:?}")))
}

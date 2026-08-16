use std::cell::Cell;

use crate::application_gate3_replay::replay_reuse_governance_for_hydrate;
use crate::application_reuse::{
    run_reuse_enabled_review_for_parts, ApplicationReuseState, ReuseSessionParts,
};
use crate::application_service::{
    ApplicationMaterialUseDeclaration, ApplicationReviewSession, DeclaredApplicationMaterialUseBasis,
    DeclaredSessionAuthority, DeclaredSessionOperatorRole, assemble_application_review_session,
};
use crate::pipeline::{CanonicalTermReviewRun, ReuseEnabledTermReviewRun};
use crate::review::ReviewLedger;
use crate::session_persistence::canonical::{
    restore_ledger_event, restore_session_terms, restore_transcript, verify_analysis_snapshot,
    verify_review_cases,
};
use crate::session_persistence::error::SessionPersistenceError;
use crate::session_persistence::reuse_canonical::{
    project_scope_is_uninitialized, restore_governance_event, restore_project_scope,
    restore_reusable_snapshot_identity,
};
use crate::session_persistence::reuse_store::{
    find_active_binding,
};
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

pub(crate) fn hydrate_application_review_session(
    opened: &OpenedStoreSession,
) -> Result<ApplicationReviewSession, SessionPersistenceError> {
    if FORCE_HYDRATE_FAILURE.with(|flag| flag.get()) {
        return Err(SessionPersistenceError::CanonicalMismatch(
            "forced hydrate failure".to_owned(),
        ));
    }
    let capture = ProductSessionStore::load_canonical_capture(opened)?;
    let transcript = restore_transcript(&capture.transcript);
    let session_terms = restore_session_terms(&capture.session_terms);
    let detector_run = run_canonical_term_review(&transcript, &session_terms)
        .map_err(|error| {
            SessionPersistenceError::Replay(crate::application_service::ApplicationServiceError::Detection(
                error,
            ))
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
    let canonical_run = CanonicalTermReviewRun::new(
        detector_run.analysis_run(),
        authoritative_cases,
    );
    let material_use = restore_material_use(&capture.material_use_basis)?;
    let session_authority =
        restore_session_authority(&capture.authority_role, &capture.authority_display_label)?;
    let mut ledger = ReviewLedger::new();
    for persisted_event in &capture.ledger_events {
        let event = restore_ledger_event(persisted_event, transcript.revision_id())?;
        let crate::review::ReviewLedgerEvent::DecisionRecorded {
            case_id,
            observed_revision,
            decision,
        } = &event;
        let review_case = canonical_run
            .review_cases()
            .get(case_id.local_index())
            .ok_or_else(|| {
                SessionPersistenceError::CanonicalMismatch("unknown review case".to_owned())
            })?;
        ledger
            .record_decision(review_case, *observed_revision, decision.clone())
            .map_err(|error| {
                SessionPersistenceError::Replay(crate::application_service::ApplicationServiceError::Decision(error))
            })?;
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

    let reuse_state = restore_reuse_state(&capture, &session_authority, &transcript, &session_terms, &canonical_run, &ledger)?;
    let reuse_enabled_run = reconstruct_reuse_enabled_run(
        &capture,
        &transcript,
        &session_terms,
        &canonical_run,
        &ledger,
        &reuse_state,
    )?;

    let reuse_active = !capture.reuse_governance_events.is_empty() || reuse_enabled_run.is_some();

    let session = assemble_application_review_session(
        transcript,
        session_terms,
        canonical_run,
        ledger,
        material_use,
        session_authority,
        reuse_state,
        reuse_enabled_run,
    )
    .map_err(SessionPersistenceError::Replay)?;

    if reuse_active {
        crate::application_gate3_replay::verify_gate3_independent_replay(&session).map_err(|_| {
            SessionPersistenceError::CanonicalMismatch("gate3 replay verification failed".to_owned())
        })?;
    } else {
        session.verify_in_memory_replay().map_err(|_| {
            SessionPersistenceError::CanonicalMismatch("in-memory replay verification failed".to_owned())
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
    ledger: &ReviewLedger,
) -> Result<ApplicationReuseState, SessionPersistenceError> {
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
        ledger,
    };
    let replayed = replay_reuse_governance_for_hydrate(
        parts,
        &project_scope,
        &governance_events,
        session_authority,
    )
    .map_err(|error| SessionPersistenceError::CanonicalMismatch(format!("governance replay: {error:?}")))?;
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
    ledger: &ReviewLedger,
    reuse_state: &ApplicationReuseState,
) -> Result<Option<ReuseEnabledTermReviewRun>, SessionPersistenceError> {
    let binding = find_active_binding(capture)?;
    if binding.is_none() {
        return Ok(None);
    }
    let binding = binding.expect("checked above");
    let parts = ReuseSessionParts {
        transcript,
        session_terms,
        canonical_run,
        ledger,
    };
    let snapshot = crate::application_reuse::reusable_influence_snapshot_for_parts(parts, reuse_state)
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

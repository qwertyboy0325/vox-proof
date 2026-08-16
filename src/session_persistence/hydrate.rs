use std::cell::Cell;

use crate::application_service::{
    ApplicationMaterialUseDeclaration, ApplicationReviewSession, DeclaredApplicationMaterialUseBasis,
    DeclaredSessionAuthority, DeclaredSessionOperatorRole, assemble_application_review_session,
};
use crate::pipeline::{CanonicalTermReviewRun, run_canonical_term_review};
use crate::review::ReviewLedger;
use crate::session_persistence::canonical::{
    restore_ledger_event, restore_session_terms, restore_transcript, verify_analysis_snapshot,
    verify_review_cases,
};
use crate::session_persistence::error::SessionPersistenceError;
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
        let ReviewLedgerEvent::DecisionRecorded {
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
    let session = assemble_application_review_session(
        transcript,
        session_terms,
        canonical_run,
        ledger,
        material_use,
        session_authority,
    )
    .map_err(SessionPersistenceError::Replay)?;
    session.verify_in_memory_replay().map_err(|_| {
        SessionPersistenceError::CanonicalMismatch("in-memory replay verification failed".to_owned())
    })?;
    Ok(session)
}

use crate::review::ReviewLedgerEvent;

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

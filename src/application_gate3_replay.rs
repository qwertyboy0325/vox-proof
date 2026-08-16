use crate::application_export_v3::build_export_bundle_v3;
use crate::application_reuse::{
    ApplicationReuseState, ReuseSessionParts, governance_actor_from_authority,
    reusable_influence_snapshot_for_parts, run_reuse_enabled_review_for_parts,
};
use crate::application_service::{
    ApplicationReplayError, ApplicationReplayField, ApplicationReviewSession,
    DeclaredSessionAuthority,
};
use crate::reusable_influence::{
    ReusableGovernanceEvent, ReusableInfluenceLedger, fold_effective_state,
    resolve_exact_input_projection, validate_governance_actor_matches_session,
    validate_reuse_candidate_key_at_historical_boundary,
};
use crate::reuse_primitives::PromotionCandidateRejectionIdentity;

pub fn verify_gate3_independent_replay(
    session: &ApplicationReviewSession,
) -> Result<(), ApplicationReplayError> {
    if session.reuse_state().governance_events().is_empty() && session.reuse_enabled_run().is_none()
    {
        return Ok(());
    }

    let parts = session.reuse_parts();
    let replay_ledger = replay_and_validate_governance_ledger(session)?;
    let replay_effective = fold_effective_state(&replay_ledger, parts.ledger, parts.canonical_run);
    let current_effective = session
        .reuse_state()
        .effective_state(parts.ledger, parts.canonical_run);
    if replay_effective != current_effective {
        return Err(ApplicationReplayError::Mismatch {
            field: ApplicationReplayField::ReuseEffectiveState,
        });
    }

    if replay_ledger.events().len() != session.reuse_state().governance_events().len() {
        return Err(ApplicationReplayError::Mismatch {
            field: ApplicationReplayField::ReuseGovernanceLedger,
        });
    }

    let project_scope = session
        .reuse_state()
        .project_scope()
        .ok_or(ApplicationReplayError::Mismatch {
            field: ApplicationReplayField::ReuseSnapshotIdentity,
        })?
        .clone();

    let replay_state =
        ApplicationReuseState::from_replayed_governance(project_scope.clone(), &replay_ledger);
    let replay_snapshot = crate::reusable_influence::build_reusable_influence_snapshot(
        &project_scope,
        &replay_ledger,
        &replay_effective,
    )
    .map_err(|_| ApplicationReplayError::Mismatch {
        field: ApplicationReplayField::ReuseSnapshotIdentity,
    })?;
    let current_snapshot = reusable_influence_snapshot_for_parts(parts, session.reuse_state())
        .map_err(|_| ApplicationReplayError::Mismatch {
            field: ApplicationReplayField::ReuseSnapshotIdentity,
        })?;
    if current_snapshot.identity() != replay_snapshot.identity() {
        return Err(ApplicationReplayError::Mismatch {
            field: ApplicationReplayField::ReuseSnapshotIdentity,
        });
    }

    let replay_projection =
        resolve_exact_input_projection(&project_scope, &replay_snapshot, parts.session_terms)
            .map_err(|_| ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::ReuseSnapshotIdentity,
            })?;
    let current_projection =
        resolve_exact_input_projection(&project_scope, &current_snapshot, parts.session_terms)
            .map_err(|_| ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::ReuseSnapshotIdentity,
            })?;
    if replay_projection != current_projection {
        return Err(ApplicationReplayError::Mismatch {
            field: ApplicationReplayField::ReuseSnapshotIdentity,
        });
    }

    let replay_run = if session.reuse_enabled_run().is_some() {
        let replay_run =
            run_reuse_enabled_review_for_parts(parts, &replay_state).map_err(|_| {
                ApplicationReplayError::Mismatch {
                    field: ApplicationReplayField::ReuseEnabledRun,
                }
            })?;
        let current_run = session.reuse_enabled_run().expect("checked above");
        if current_run.review_cases() != replay_run.review_cases() {
            return Err(ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::ReuseEnabledRun,
            });
        }
        if current_run.reuse_enabled_snapshot() != replay_run.reuse_enabled_snapshot() {
            return Err(ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::ReuseEnabledRun,
            });
        }
        Some(replay_run)
    } else {
        None
    };

    if matches!(
        session.progress().decision_coverage,
        crate::application_service::ApplicationDecisionCoverage::Complete
    ) {
        let current_v3 = materialize_v3_for_reuse_state(
            session,
            session.reuse_state(),
            session.reuse_enabled_run(),
        )?;
        let replay_v3 =
            materialize_v3_for_reuse_state(session, &replay_state, replay_run.as_ref())?;
        if current_v3 != replay_v3 {
            return Err(ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::ExportBundleV3,
            });
        }
    }

    Ok(())
}

fn materialize_v3_for_reuse_state(
    session: &ApplicationReviewSession,
    reuse_state: &ApplicationReuseState,
    reuse_enabled_run: Option<&crate::pipeline::ReuseEnabledTermReviewRun>,
) -> Result<crate::application_export_v3::ApplicationReviewExportBundleV3, ApplicationReplayError> {
    let base = session
        .materialize_review_export_bundle()
        .map_err(ApplicationReplayError::Service)?;
    let parts = session.reuse_parts();
    let derived_candidates =
        crate::application_reuse::reuse_candidates_for_parts(parts, reuse_state).map_err(|_| {
            ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::ExportBundleV3,
            }
        })?;
    build_export_bundle_v3(
        base,
        reuse_state,
        session.review_ledger(),
        parts.canonical_run,
        derived_candidates,
        reuse_enabled_run,
    )
    .map_err(|_| ApplicationReplayError::Mismatch {
        field: ApplicationReplayField::ExportBundleV3,
    })
}

pub(crate) fn replay_reuse_governance_for_hydrate(
    parts: ReuseSessionParts<'_>,
    project_scope: &crate::reuse_primitives::ProjectScope,
    events: &[ReusableGovernanceEvent],
    session_authority: &DeclaredSessionAuthority,
) -> Result<ReusableInfluenceLedger, ApplicationReplayError> {
    let expected_actor = governance_actor_from_authority(session_authority);
    let mut replay = ReusableInfluenceLedger::new();
    for event in events {
        validate_governance_event(parts, project_scope, &replay, event, &expected_actor)?;
        replay.append(event.clone());
    }
    Ok(replay)
}

/// Test-only helper for integration tests validating replay rejection of tampered events.
pub fn validate_replayed_governance_events_for_test(
    parts: ReuseSessionParts<'_>,
    project_scope: &crate::reuse_primitives::ProjectScope,
    events: &[ReusableGovernanceEvent],
    session_authority: &DeclaredSessionAuthority,
) -> Result<(), ApplicationReplayError> {
    let expected_actor = governance_actor_from_authority(session_authority);
    let mut replay = ReusableInfluenceLedger::new();
    for event in events {
        validate_governance_event(parts, project_scope, &replay, event, &expected_actor)?;
        replay.append(event.clone());
    }
    Ok(())
}

fn replay_and_validate_governance_ledger(
    session: &ApplicationReviewSession,
) -> Result<ReusableInfluenceLedger, ApplicationReplayError> {
    let parts = session.reuse_parts();
    let project_scope =
        session
            .reuse_state()
            .project_scope()
            .ok_or(ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::ReuseGovernanceLedger,
            })?;
    let expected_actor = governance_actor_from_authority(session.session_authority());
    let mut replay = ReusableInfluenceLedger::new();

    for event in session.reuse_state().governance_events() {
        validate_governance_event(parts, project_scope, &replay, event, &expected_actor)?;
        replay.append(event.clone());
    }

    Ok(replay)
}

fn validate_governance_event(
    parts: ReuseSessionParts<'_>,
    project_scope: &crate::reuse_primitives::ProjectScope,
    replay: &ReusableInfluenceLedger,
    event: &ReusableGovernanceEvent,
    expected_actor: &crate::reusable_influence::GovernanceActorContext,
) -> Result<(), ApplicationReplayError> {
    let replay_effective = fold_effective_state(replay, parts.ledger, parts.canonical_run);
    match event {
        ReusableGovernanceEvent::PromotionCandidateRejected {
            candidate_key,
            actor,
        } => {
            validate_governance_actor_matches_session(actor, expected_actor).map_err(|_| {
                ApplicationReplayError::Mismatch {
                    field: ApplicationReplayField::ReuseGovernanceLedger,
                }
            })?;
            if candidate_key.project_scope_id != project_scope.stable_id {
                return Err(ApplicationReplayError::Mismatch {
                    field: ApplicationReplayField::ReuseGovernanceLedger,
                });
            }
            validate_reuse_candidate_key_at_historical_boundary(
                candidate_key.as_ref(),
                parts.ledger,
                parts.canonical_run,
                parts.human_raised_cases,
                parts.transcript,
                &replay_effective,
            )
            .map_err(|_| ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::ReuseGovernanceLedger,
            })?;
        }
        ReusableGovernanceEvent::PromotionAccepted {
            candidate_key,
            payload,
            source_locator,
            project_scope: event_scope,
            actor,
            ..
        } => {
            validate_governance_actor_matches_session(actor, expected_actor).map_err(|_| {
                ApplicationReplayError::Mismatch {
                    field: ApplicationReplayField::ReuseGovernanceLedger,
                }
            })?;
            if candidate_key.project_scope_id != event_scope.stable_id
                || event_scope.stable_id != project_scope.stable_id
            {
                return Err(ApplicationReplayError::Mismatch {
                    field: ApplicationReplayField::ReuseGovernanceLedger,
                });
            }
            if candidate_key.source_locator != **source_locator {
                return Err(ApplicationReplayError::Mismatch {
                    field: ApplicationReplayField::ReuseGovernanceLedger,
                });
            }
            if replay_effective.rejected_candidate_identities().contains(
                &PromotionCandidateRejectionIdentity::from(candidate_key.as_ref()),
            ) {
                return Err(ApplicationReplayError::Mismatch {
                    field: ApplicationReplayField::ReuseGovernanceLedger,
                });
            }
            validate_reuse_candidate_key_at_historical_boundary(
                candidate_key.as_ref(),
                parts.ledger,
                parts.canonical_run,
                parts.human_raised_cases,
                parts.transcript,
                &replay_effective,
            )
            .map_err(|_| ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::ReuseGovernanceLedger,
            })?;
            let review_case = crate::reusable_influence::resolve_locator_review_case(
                source_locator.as_ref(),
                parts.canonical_run,
                parts.human_raised_cases,
            )
            .ok_or(ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::ReuseGovernanceLedger,
            })?;
            let observed = crate::reusable_influence::resolve_review_case_observed_text(
                parts.transcript,
                review_case,
            )
            .map_err(|_| ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::ReuseGovernanceLedger,
            })?;
            if observed != payload.observed_text
                || payload.confirmed_replacement
                    != match parts
                        .ledger
                        .events()
                        .get(source_locator.review_ledger_position)
                    {
                        Some(crate::review::ReviewLedgerEvent::DecisionRecorded {
                            decision:
                                crate::review::CorrectionDecision::ManualReplacement { replacement },
                            ..
                        }) => replacement.as_str(),
                        _ => {
                            return Err(ApplicationReplayError::Mismatch {
                                field: ApplicationReplayField::ReuseGovernanceLedger,
                            });
                        }
                    }
            {
                return Err(ApplicationReplayError::Mismatch {
                    field: ApplicationReplayField::ReuseGovernanceLedger,
                });
            }
            if replay_effective
                .active_records()
                .iter()
                .any(|record| record.source_locator == **source_locator)
            {
                return Err(ApplicationReplayError::Mismatch {
                    field: ApplicationReplayField::ReuseGovernanceLedger,
                });
            }
        }
        ReusableGovernanceEvent::ReusableInfluenceRevoked { record_id, actor } => {
            validate_governance_actor_matches_session(actor, expected_actor).map_err(|_| {
                ApplicationReplayError::Mismatch {
                    field: ApplicationReplayField::ReuseGovernanceLedger,
                }
            })?;
            if !replay_effective
                .active_records()
                .iter()
                .any(|record| record.record_id == *record_id)
            {
                return Err(ApplicationReplayError::Mismatch {
                    field: ApplicationReplayField::ReuseGovernanceLedger,
                });
            }
        }
        ReusableGovernanceEvent::ReusableInfluenceSuperseded {
            predecessor_id,
            successor_id,
            actor,
        } => {
            validate_governance_actor_matches_session(actor, expected_actor).map_err(|_| {
                ApplicationReplayError::Mismatch {
                    field: ApplicationReplayField::ReuseGovernanceLedger,
                }
            })?;
            if predecessor_id == successor_id {
                return Err(ApplicationReplayError::Mismatch {
                    field: ApplicationReplayField::ReuseGovernanceLedger,
                });
            }
            if !replay_effective
                .active_records()
                .iter()
                .any(|record| record.record_id == *predecessor_id)
            {
                return Err(ApplicationReplayError::Mismatch {
                    field: ApplicationReplayField::ReuseGovernanceLedger,
                });
            }
            if !replay_effective
                .active_records()
                .iter()
                .any(|record| record.record_id == *successor_id)
            {
                return Err(ApplicationReplayError::Mismatch {
                    field: ApplicationReplayField::ReuseGovernanceLedger,
                });
            }
        }
    }
    Ok(())
}

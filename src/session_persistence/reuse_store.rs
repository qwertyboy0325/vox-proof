use rusqlite::{TransactionBehavior, params};

use crate::application_reuse::{
    ApplicationReuseState, PreparedActiveAnalysis, PreparedProjectScopeDisplayNameUpdate,
    PreparedProjectScopeInitialization, PreparedReusableInfluenceRevocation,
    PreparedReusableInfluenceSupersession, PreparedReuseCandidateAcceptance,
    PreparedReuseCandidateRejection, ReuseSessionParts, build_promotion_accepted_event,
    governance_actor_from_authority, run_reuse_enabled_review_for_parts,
    validate_accept_reuse_candidate_for_governance_commit,
    validate_reject_reuse_candidate_for_governance_commit,
};
use crate::application_service::DeclaredSessionAuthority;
use crate::candidate::SessionTermEntry;
use crate::pipeline::CanonicalTermReviewRun;
use crate::reusable_influence::{ReusableGovernanceEvent, ReusableInfluenceLedger};
use crate::review::ReviewLedger;
use crate::review::ReviewLedgerEvent;
use crate::session_persistence::canonical::{
    persist_analysis_snapshot, restore_ledger_event, restore_review_case, restore_session_terms,
    restore_transcript,
};
use crate::session_persistence::error::{
    AuthorityScope, SessionPersistenceError, StaleAuthorityPrecondition,
};
use crate::session_persistence::reuse_canonical::{
    active_analysis_selection_identity_for_reuse_enabled, persist_governance_event,
    persist_reuse_enabled_binding, restore_governance_event, restore_project_scope,
    restore_reusable_snapshot_identity,
};
use crate::session_persistence::store::{
    OpenMode, OpenedStoreSession, ProductSessionStore, load_canonical_capture_from_connection,
    verify_writer_token_in_transaction,
};
use crate::transcript::Transcript;

impl ProductSessionStore {
    pub fn initialize_project_scope(
        opened: &mut OpenedStoreSession,
        prepared: &PreparedProjectScopeInitialization,
    ) -> Result<(), SessionPersistenceError> {
        ensure_writable(opened)?;
        let writer_token = opened.writer_token.clone().expect("writer token");
        let tx = opened
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        verify_writer_token_in_transaction(&tx, &opened.session_id, &writer_token)?;
        let current_head: usize = tx
            .query_row(
                "SELECT reuse_governance_head FROM command_tokens WHERE session_id = ?1",
                [&opened.session_id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?
            as usize;
        if current_head != prepared.expected_reuse_governance_head {
            return Err(SessionPersistenceError::StaleAuthorityPrecondition(
                StaleAuthorityPrecondition {
                    scope: AuthorityScope::ReuseGovernance,
                },
            ));
        }
        let updated = tx
            .execute(
                "UPDATE project_scope SET stable_id = ?1, display_name = ?2 WHERE session_id = ?3 AND stable_id = ''",
                params![
                    prepared.stable_id,
                    prepared.display_name,
                    opened.session_id
                ],
            )
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        if updated != 1 {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "project scope frozen or missing".to_owned(),
            ));
        }
        insert_authority_transition(&tx, &opened.session_id)?;
        tx.commit()
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        Ok(())
    }

    pub fn update_project_scope_display_name(
        opened: &mut OpenedStoreSession,
        prepared: &PreparedProjectScopeDisplayNameUpdate,
    ) -> Result<(), SessionPersistenceError> {
        ensure_writable(opened)?;
        let writer_token = opened.writer_token.clone().expect("writer token");
        let tx = opened
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        verify_writer_token_in_transaction(&tx, &opened.session_id, &writer_token)?;
        let updated = tx
            .execute(
                "UPDATE project_scope SET display_name = ?1 WHERE session_id = ?2 AND stable_id != ''",
                params![prepared.display_name, opened.session_id],
            )
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        if updated != 1 {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "project scope not initialized".to_owned(),
            ));
        }
        tx.commit()
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        Ok(())
    }

    pub fn append_reuse_governance_accept(
        opened: &mut OpenedStoreSession,
        prepared: &PreparedReuseCandidateAcceptance,
        session_authority: &DeclaredSessionAuthority,
    ) -> Result<(), SessionPersistenceError> {
        let loaded = load_reuse_context(&opened.connection, &opened.session_id)?;
        let parts = loaded.parts();
        let reuse_state = load_reuse_state_for_append(&opened.connection, &opened.session_id)?;
        let candidate = validate_accept_reuse_candidate_for_governance_commit(
            parts,
            &reuse_state,
            &prepared.candidate_key,
        )
        .map_err(map_reuse_error)?;
        let project_scope = reuse_state
            .project_scope()
            .expect("validated candidate requires scope")
            .clone();
        let event = build_promotion_accepted_event(&candidate, &project_scope, session_authority);
        append_reuse_governance_events(
            opened,
            prepared.expected_reuse_governance_head,
            vec![event],
        )?;
        Ok(())
    }

    pub fn append_reuse_governance_reject(
        opened: &mut OpenedStoreSession,
        prepared: &PreparedReuseCandidateRejection,
        session_authority: &DeclaredSessionAuthority,
    ) -> Result<(), SessionPersistenceError> {
        let loaded = load_reuse_context(&opened.connection, &opened.session_id)?;
        let parts = loaded.parts();
        let reuse_state = load_reuse_state_for_append(&opened.connection, &opened.session_id)?;
        validate_reject_reuse_candidate_for_governance_commit(
            parts,
            &reuse_state,
            &prepared.candidate_key,
        )
        .map_err(map_reuse_error)?;
        let event = ReusableGovernanceEvent::PromotionCandidateRejected {
            candidate_key: Box::new(prepared.candidate_key.clone()),
            actor: governance_actor_from_authority(session_authority),
        };
        append_reuse_governance_events(
            opened,
            prepared.expected_reuse_governance_head,
            vec![event],
        )?;
        Ok(())
    }

    pub fn append_reuse_governance_revoke(
        opened: &mut OpenedStoreSession,
        prepared: &PreparedReusableInfluenceRevocation,
        session_authority: &DeclaredSessionAuthority,
    ) -> Result<(), SessionPersistenceError> {
        let loaded = load_reuse_context(&opened.connection, &opened.session_id)?;
        let parts = loaded.parts();
        let reuse_state = load_reuse_state_for_append(&opened.connection, &opened.session_id)?;
        let effective = reuse_state.effective_state(parts.ledger, parts.canonical_run);
        if !effective
            .active_records()
            .iter()
            .any(|record| record.record_id == prepared.record_id)
        {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "reusable influence record not active".to_owned(),
            ));
        }
        let _ = session_authority;
        let event = ReusableGovernanceEvent::ReusableInfluenceRevoked {
            record_id: prepared.record_id,
            actor: governance_actor_from_authority(session_authority),
        };
        append_reuse_governance_events(
            opened,
            prepared.expected_reuse_governance_head,
            vec![event],
        )?;
        Ok(())
    }

    pub fn append_reuse_governance_supersede(
        opened: &mut OpenedStoreSession,
        prepared: &PreparedReusableInfluenceSupersession,
        session_authority: &DeclaredSessionAuthority,
    ) -> Result<(), SessionPersistenceError> {
        let loaded = load_reuse_context(&opened.connection, &opened.session_id)?;
        let parts = loaded.parts();
        let reuse_state = load_reuse_state_for_append(&opened.connection, &opened.session_id)?;
        let candidate = validate_accept_reuse_candidate_for_governance_commit(
            parts,
            &reuse_state,
            &prepared.successor_candidate_key,
        )
        .map_err(map_reuse_error)?;
        let project_scope = reuse_state
            .project_scope()
            .expect("validated candidate requires scope")
            .clone();
        let promotion_index = reuse_state.governance_events().len();
        let successor_id =
            crate::reuse_primitives::ReusableInfluenceRecordId::from_promotion_event_index(
                promotion_index,
            );
        let events = vec![
            build_promotion_accepted_event(&candidate, &project_scope, session_authority),
            ReusableGovernanceEvent::ReusableInfluenceSuperseded {
                predecessor_id: prepared.predecessor_id,
                successor_id,
                actor: governance_actor_from_authority(session_authority),
            },
        ];
        append_reuse_governance_events(opened, prepared.expected_reuse_governance_head, events)?;
        Ok(())
    }

    pub fn commit_active_analysis(
        opened: &mut OpenedStoreSession,
        prepared: &PreparedActiveAnalysis,
        precondition_selection_token: &str,
    ) -> Result<(), SessionPersistenceError> {
        ensure_writable(opened)?;
        let writer_token = opened.writer_token.clone().expect("writer token");
        let tx = opened
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        verify_writer_token_in_transaction(&tx, &opened.session_id, &writer_token)?;
        let current_selection: String = tx
            .query_row(
                "SELECT active_analysis_snapshot_identity FROM command_tokens WHERE session_id = ?1",
                [&opened.session_id],
                |row| row.get(0),
            )
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        if current_selection != precondition_selection_token {
            return Err(SessionPersistenceError::StaleAuthorityPrecondition(
                StaleAuthorityPrecondition {
                    scope: AuthorityScope::ActiveAnalysis,
                },
            ));
        }
        let loaded = load_reuse_context_from_tx(&tx, &opened.session_id)?;
        let parts = loaded.parts();
        let reuse_state = load_reuse_state_for_append_from_tx(&tx, &opened.session_id)?;
        if reuse_state.governance_events().len() != prepared.governance_event_boundary {
            return Err(SessionPersistenceError::StaleAuthorityPrecondition(
                StaleAuthorityPrecondition {
                    scope: AuthorityScope::ReuseGovernance,
                },
            ));
        }
        let snapshot =
            crate::application_reuse::reusable_influence_snapshot_for_parts(parts, &reuse_state)
                .map_err(map_reuse_error)?;
        if snapshot.identity() != prepared.reusable_snapshot_identity {
            return Err(SessionPersistenceError::StaleAuthorityPrecondition(
                StaleAuthorityPrecondition {
                    scope: AuthorityScope::ActiveAnalysis,
                },
            ));
        }
        let run =
            run_reuse_enabled_review_for_parts(parts, &reuse_state).map_err(map_reuse_error)?;
        let selection_token = active_analysis_selection_identity_for_reuse_enabled(
            run.analysis_run().snapshot(),
            snapshot.identity(),
            prepared.governance_event_boundary,
        );
        if selection_token != prepared.expected_selection_token {
            return Err(SessionPersistenceError::StaleAuthorityPrecondition(
                StaleAuthorityPrecondition {
                    scope: AuthorityScope::ActiveAnalysis,
                },
            ));
        }
        let persisted_snapshot = persist_analysis_snapshot(run.analysis_run().snapshot());
        let snapshot_json = serde_json::to_string(&persisted_snapshot)
            .map_err(|error| SessionPersistenceError::CanonicalMismatch(error.to_string()))?;
        tx.execute(
            "INSERT INTO analysis_snapshots (session_id, snapshot_identity, snapshot_json) VALUES (?1, ?2, ?3)",
            params![
                opened.session_id,
                prepared.expected_selection_token,
                snapshot_json
            ],
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        let next_binding_id: i64 = tx
            .query_row(
                "SELECT COALESCE(MAX(binding_id), 0) + 1 FROM reuse_enabled_bindings WHERE session_id = ?1",
                [&opened.session_id],
                |row| row.get(0),
            )
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        let binding = persist_reuse_enabled_binding(
            next_binding_id as usize,
            run.analysis_run().snapshot(),
            snapshot.identity(),
            prepared.governance_event_boundary,
        );
        let binding_json = serde_json::to_string(&binding)
            .map_err(|error| SessionPersistenceError::CanonicalMismatch(error.to_string()))?;
        tx.execute(
            "INSERT INTO reuse_enabled_bindings (session_id, binding_id, binding_json) VALUES (?1, ?2, ?3)",
            params![opened.session_id, next_binding_id, binding_json],
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        tx.execute(
            "UPDATE command_tokens SET active_analysis_snapshot_identity = ?1 WHERE session_id = ?2",
            params![prepared.expected_selection_token, opened.session_id],
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        insert_authority_transition(&tx, &opened.session_id)?;
        tx.commit()
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        Ok(())
    }
}

fn append_reuse_governance_events(
    opened: &mut OpenedStoreSession,
    expected_head: usize,
    events: Vec<ReusableGovernanceEvent>,
) -> Result<(), SessionPersistenceError> {
    ensure_writable(opened)?;
    let writer_token = opened.writer_token.clone().expect("writer token");
    let tx = opened
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    verify_writer_token_in_transaction(&tx, &opened.session_id, &writer_token)?;
    let current_head: usize =
        tx.query_row(
            "SELECT reuse_governance_head FROM command_tokens WHERE session_id = ?1",
            [&opened.session_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))? as usize;
    if current_head != expected_head {
        return Err(SessionPersistenceError::StaleAuthorityPrecondition(
            StaleAuthorityPrecondition {
                scope: AuthorityScope::ReuseGovernance,
            },
        ));
    }
    let mut event_index = current_head;
    for event in events {
        let persisted = persist_governance_event(&event);
        let event_json = serde_json::to_string(&persisted)
            .map_err(|error| SessionPersistenceError::CanonicalMismatch(error.to_string()))?;
        tx.execute(
            "INSERT INTO reuse_governance_events (session_id, event_index, event_json) VALUES (?1, ?2, ?3)",
            params![opened.session_id, event_index as i64, event_json],
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        event_index += 1;
    }
    tx.execute(
        "UPDATE command_tokens SET reuse_governance_head = ?1 WHERE session_id = ?2",
        params![event_index as i64, opened.session_id],
    )
    .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    insert_authority_transition(&tx, &opened.session_id)?;
    tx.commit()
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    Ok(())
}

fn insert_authority_transition(
    tx: &rusqlite::Transaction<'_>,
    session_id: &str,
) -> Result<(), SessionPersistenceError> {
    let generation: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(generation), 0) + 1 FROM authority_transitions WHERE session_id = ?1",
            [session_id],
            |row| row.get(0),
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    tx.execute(
        "INSERT INTO authority_transitions (session_id, generation, acknowledgement_status) VALUES (?1, ?2, 'committed')",
        params![session_id, generation],
    )
    .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    Ok(())
}

fn ensure_writable(opened: &OpenedStoreSession) -> Result<(), SessionPersistenceError> {
    if opened.mode != OpenMode::Writable {
        return Err(SessionPersistenceError::SessionNotWritable);
    }
    if opened.writer_token.is_none() {
        return Err(SessionPersistenceError::WriterOwnershipHeld);
    }
    Ok(())
}

fn map_reuse_error(
    error: crate::application_reuse::ApplicationReuseError,
) -> SessionPersistenceError {
    SessionPersistenceError::CanonicalMismatch(format!("{error:?}"))
}

struct LoadedReuseContext {
    transcript: Transcript,
    session_terms: Vec<SessionTermEntry>,
    canonical_run: CanonicalTermReviewRun,
    ledger: ReviewLedger,
}

impl LoadedReuseContext {
    fn parts(&self) -> ReuseSessionParts<'_> {
        ReuseSessionParts {
            transcript: &self.transcript,
            session_terms: &self.session_terms,
            canonical_run: &self.canonical_run,
            ledger: &self.ledger,
        }
    }
}

fn load_reuse_context(
    connection: &rusqlite::Connection,
    session_id: &str,
) -> Result<LoadedReuseContext, SessionPersistenceError> {
    load_reuse_context_from_connection(connection, session_id)
}

fn load_reuse_context_from_tx(
    tx: &rusqlite::Transaction<'_>,
    session_id: &str,
) -> Result<LoadedReuseContext, SessionPersistenceError> {
    load_reuse_context_from_connection(tx, session_id)
}

fn load_reuse_context_from_connection(
    connection: &rusqlite::Connection,
    session_id: &str,
) -> Result<LoadedReuseContext, SessionPersistenceError> {
    let capture = load_canonical_capture_from_connection(connection, session_id)?;
    let transcript = restore_transcript(&capture.transcript);
    let session_terms = restore_session_terms(&capture.session_terms);
    let revision = transcript.revision_id();
    let review_cases = capture
        .review_cases
        .iter()
        .map(|case| restore_review_case(case, revision))
        .collect::<Result<Vec<_>, _>>()?;
    let canonical_run = CanonicalTermReviewRun::new(
        crate::pipeline::run_canonical_term_review(&transcript, &session_terms)
            .map_err(|error| {
                SessionPersistenceError::Replay(
                    crate::application_service::ApplicationServiceError::Detection(error),
                )
            })?
            .analysis_run(),
        review_cases,
    );
    let mut ledger = ReviewLedger::new();
    for persisted_event in &capture.ledger_events {
        let event = restore_ledger_event(persisted_event, revision)?;
        match event {
            ReviewLedgerEvent::DecisionRecorded {
                case_id,
                observed_revision,
                decision,
            } => {
                let review_case = canonical_run
                    .review_cases()
                    .get(case_id.local_index())
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
            ReviewLedgerEvent::ReuseProposalDecisionRecorded {
                target_identity,
                observed_revision,
                decision,
            } => {
                ledger
                    .record_reuse_decision(target_identity, observed_revision, decision)
                    .map_err(|error| {
                        SessionPersistenceError::Replay(
                            crate::application_service::ApplicationServiceError::Decision(error),
                        )
                    })?;
            }
        }
    }
    Ok(LoadedReuseContext {
        transcript,
        session_terms,
        canonical_run,
        ledger,
    })
}

fn load_reuse_state_for_append(
    connection: &rusqlite::Connection,
    session_id: &str,
) -> Result<ApplicationReuseState, SessionPersistenceError> {
    load_reuse_state_for_append_from_tx(connection, session_id)
}

fn load_reuse_state_for_append_from_tx(
    connection: &rusqlite::Connection,
    session_id: &str,
) -> Result<ApplicationReuseState, SessionPersistenceError> {
    let capture = load_canonical_capture_from_connection(connection, session_id)?;
    build_reuse_state_from_capture(&capture)
}

pub(crate) fn build_reuse_state_from_capture(
    capture: &crate::session_persistence::canonical::SessionCanonicalCapture,
) -> Result<ApplicationReuseState, SessionPersistenceError> {
    let project_scope = restore_project_scope(&capture.project_scope)?;
    let mut governance_ledger = ReusableInfluenceLedger::new();
    for persisted in &capture.reuse_governance_events {
        governance_ledger.append(restore_governance_event(persisted)?);
    }
    if let Some(scope) = project_scope {
        Ok(ApplicationReuseState::from_replayed_governance(
            scope,
            &governance_ledger,
        ))
    } else {
        Ok(ApplicationReuseState::default())
    }
}

pub(crate) fn find_active_binding(
    capture: &crate::session_persistence::canonical::SessionCanonicalCapture,
) -> Result<
    Option<&crate::session_persistence::reuse_canonical::PersistedReuseEnabledBindingV1>,
    SessionPersistenceError,
> {
    for binding in &capture.reuse_enabled_bindings {
        let analysis_snapshot =
            crate::session_persistence::canonical::restore_analysis_snapshot_from_persisted(
                &binding.analysis_snapshot,
            )?;
        let reusable_identity =
            restore_reusable_snapshot_identity(&binding.reusable_snapshot_identity)?;
        let identity = active_analysis_selection_identity_for_reuse_enabled(
            analysis_snapshot,
            reusable_identity,
            binding.governance_event_boundary,
        );
        if identity == capture.active_analysis_selection_identity {
            return Ok(Some(binding));
        }
    }
    Ok(None)
}

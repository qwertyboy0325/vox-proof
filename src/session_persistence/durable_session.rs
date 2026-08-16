use crate::application_reuse::{
    PreparedActiveAnalysis, PreparedProjectScopeDisplayNameUpdate,
    PreparedProjectScopeInitialization, PreparedReusableInfluenceRevocation,
    PreparedReusableInfluenceSupersession, PreparedReuseCandidateAcceptance,
    PreparedReuseCandidateRejection,
};
use crate::application_service::{
    ApplicationMaterialUseDeclaration, ApplicationReviewSession, DeclaredSessionAuthority,
    PreparedHumanDecision, PreparedManualReplacement,
};
use crate::candidate::SessionTermEntry;
use crate::session_persistence::error::SessionPersistenceError;
use crate::session_persistence::hydrate::hydrate_application_review_session;
use crate::session_persistence::store::{OpenedStoreSession, OpenMode, ProductSessionStore};
use crate::transcript::Transcript;

pub struct DurableApplicationSession {
    session_id: String,
    opened: OpenedStoreSession,
    session: ApplicationReviewSession,
    recovery_required: bool,
}

impl DurableApplicationSession {
    pub fn create(
        store: &ProductSessionStore,
        transcript: Transcript,
        session_terms: Vec<SessionTermEntry>,
        material_use: ApplicationMaterialUseDeclaration,
        session_authority: DeclaredSessionAuthority,
    ) -> Result<Self, SessionPersistenceError> {
        let (session_id, opened) = store.create_session(
            transcript,
            session_terms,
            material_use,
            session_authority,
        )?;
        let session = hydrate_application_review_session(&opened)?;
        Ok(Self {
            session_id,
            opened,
            session,
            recovery_required: false,
        })
    }

    pub fn open(
        store: &ProductSessionStore,
        session_id: &str,
        mode: OpenMode,
    ) -> Result<Self, SessionPersistenceError> {
        let opened = store.open_session(session_id, mode)?;
        let session = hydrate_application_review_session(&opened)?;
        Ok(Self {
            session_id: session_id.to_owned(),
            opened,
            session,
            recovery_required: false,
        })
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn session(&self) -> &ApplicationReviewSession {
        &self.session
    }

    pub fn is_recovery_required(&self) -> bool {
        self.recovery_required
    }

    pub fn open_mode(&self) -> OpenMode {
        self.opened.mode
    }

    pub fn release_writer(&mut self) -> Result<(), SessionPersistenceError> {
        ProductSessionStore::release_writer(&mut self.opened)
    }

    pub fn rehydrate(&mut self) -> Result<(), SessionPersistenceError> {
        self.session = hydrate_application_review_session(&self.opened)?;
        self.recovery_required = false;
        Ok(())
    }

    pub fn record_human_decision(
        &mut self,
        prepared: PreparedHumanDecision,
    ) -> Result<(), SessionPersistenceError> {
        self.ensure_writable()?;
        ProductSessionStore::append_review_ledger_event(&mut self.opened, &prepared)?;
        self.refresh_after_commit()
    }

    pub fn record_manual_replacement(
        &mut self,
        prepared: PreparedManualReplacement,
    ) -> Result<(), SessionPersistenceError> {
        self.record_human_decision(prepared.prepared)
    }

    pub fn prepare_human_decision(
        &self,
        target: crate::application_service::ApplicationReviewTarget,
        decision: crate::review::CorrectionDecision,
    ) -> Result<PreparedHumanDecision, SessionPersistenceError> {
        self.ensure_readable()?;
        self.session
            .prepare_human_decision(target, decision)
            .map_err(SessionPersistenceError::Replay)
    }

    pub fn prepare_manual_replacement(
        &self,
        target: crate::application_service::ApplicationReviewTarget,
        replacement: impl Into<String>,
    ) -> Result<PreparedManualReplacement, SessionPersistenceError> {
        self.ensure_readable()?;
        self.session
            .prepare_manual_replacement(target, replacement)
            .map_err(SessionPersistenceError::Replay)
    }

    pub fn prepare_initialize_project_scope(
        &self,
        stable_id: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Result<PreparedProjectScopeInitialization, SessionPersistenceError> {
        self.ensure_readable()?;
        self.session
            .prepare_initialize_project_scope(stable_id, display_name)
            .map_err(map_reuse_error)
    }

    pub fn record_initialize_project_scope(
        &mut self,
        prepared: PreparedProjectScopeInitialization,
    ) -> Result<(), SessionPersistenceError> {
        self.ensure_writable()?;
        ProductSessionStore::initialize_project_scope(&mut self.opened, &prepared)?;
        self.refresh_after_commit()
    }

    pub fn prepare_update_project_scope_display_name(
        &self,
        display_name: impl Into<String>,
    ) -> Result<PreparedProjectScopeDisplayNameUpdate, SessionPersistenceError> {
        self.ensure_readable()?;
        self.session
            .prepare_update_project_scope_display_name(display_name)
            .map_err(map_reuse_error)
    }

    pub fn record_update_project_scope_display_name(
        &mut self,
        prepared: PreparedProjectScopeDisplayNameUpdate,
    ) -> Result<(), SessionPersistenceError> {
        self.ensure_writable()?;
        ProductSessionStore::update_project_scope_display_name(&mut self.opened, &prepared)?;
        self.refresh_after_commit()
    }

    pub fn prepare_accept_reuse_candidate(
        &self,
        candidate_key: &crate::reusable_influence::ReuseCandidateKey,
    ) -> Result<PreparedReuseCandidateAcceptance, SessionPersistenceError> {
        self.ensure_readable()?;
        self.session
            .prepare_accept_reuse_candidate(candidate_key)
            .map_err(map_reuse_error)
    }

    pub fn record_accept_reuse_candidate(
        &mut self,
        prepared: PreparedReuseCandidateAcceptance,
    ) -> Result<(), SessionPersistenceError> {
        self.ensure_writable()?;
        let authority = self.session.session_authority().clone();
        ProductSessionStore::append_reuse_governance_accept(
            &mut self.opened,
            &prepared,
            &authority,
        )?;
        self.refresh_after_commit()
    }

    pub fn prepare_reject_reuse_candidate(
        &self,
        candidate_key: &crate::reusable_influence::ReuseCandidateKey,
    ) -> Result<PreparedReuseCandidateRejection, SessionPersistenceError> {
        self.ensure_readable()?;
        self.session
            .prepare_reject_reuse_candidate(candidate_key)
            .map_err(map_reuse_error)
    }

    pub fn record_reject_reuse_candidate(
        &mut self,
        prepared: PreparedReuseCandidateRejection,
    ) -> Result<(), SessionPersistenceError> {
        self.ensure_writable()?;
        let authority = self.session.session_authority().clone();
        ProductSessionStore::append_reuse_governance_reject(
            &mut self.opened,
            &prepared,
            &authority,
        )?;
        self.refresh_after_commit()
    }

    pub fn prepare_revoke_reusable_influence(
        &self,
        record_id: crate::reuse_primitives::ReusableInfluenceRecordId,
    ) -> Result<PreparedReusableInfluenceRevocation, SessionPersistenceError> {
        self.ensure_readable()?;
        self.session
            .prepare_revoke_reusable_influence(record_id)
            .map_err(map_reuse_error)
    }

    pub fn record_revoke_reusable_influence(
        &mut self,
        prepared: PreparedReusableInfluenceRevocation,
    ) -> Result<(), SessionPersistenceError> {
        self.ensure_writable()?;
        let authority = self.session.session_authority().clone();
        ProductSessionStore::append_reuse_governance_revoke(
            &mut self.opened,
            &prepared,
            &authority,
        )?;
        self.refresh_after_commit()
    }

    pub fn prepare_supersede_reusable_influence(
        &self,
        predecessor_id: crate::reuse_primitives::ReusableInfluenceRecordId,
        successor_candidate_key: &crate::reusable_influence::ReuseCandidateKey,
    ) -> Result<PreparedReusableInfluenceSupersession, SessionPersistenceError> {
        self.ensure_readable()?;
        self.session
            .prepare_supersede_reusable_influence(predecessor_id, successor_candidate_key)
            .map_err(map_reuse_error)
    }

    pub fn record_supersede_reusable_influence(
        &mut self,
        prepared: PreparedReusableInfluenceSupersession,
    ) -> Result<(), SessionPersistenceError> {
        self.ensure_writable()?;
        let authority = self.session.session_authority().clone();
        ProductSessionStore::append_reuse_governance_supersede(
            &mut self.opened,
            &prepared,
            &authority,
        )?;
        self.refresh_after_commit()
    }

    pub fn prepare_run_reuse_enabled_review(
        &self,
    ) -> Result<(PreparedActiveAnalysis, String), SessionPersistenceError> {
        self.ensure_readable()?;
        let capture = ProductSessionStore::load_canonical_capture(&self.opened)?;
        let precondition = capture.active_analysis_selection_identity;
        let prepared = self
            .session
            .prepare_run_reuse_enabled_review()
            .map_err(map_reuse_error)?;
        Ok((prepared, precondition))
    }

    pub fn record_run_reuse_enabled_review(
        &mut self,
        prepared: PreparedActiveAnalysis,
        precondition_selection_token: String,
    ) -> Result<(), SessionPersistenceError> {
        self.ensure_writable()?;
        ProductSessionStore::commit_active_analysis(
            &mut self.opened,
            &prepared,
            &precondition_selection_token,
        )?;
        self.refresh_after_commit()
    }

    pub fn close(mut self) -> Result<(), SessionPersistenceError> {
        ProductSessionStore::release_writer(&mut self.opened)
    }

    fn refresh_after_commit(&mut self) -> Result<(), SessionPersistenceError> {
        match hydrate_application_review_session(&self.opened) {
            Ok(session) => {
                self.session = session;
                self.recovery_required = false;
                Ok(())
            }
            Err(_error) => {
                self.recovery_required = true;
                Err(SessionPersistenceError::RecoveryRequired)
            }
        }
    }

    fn ensure_writable(&self) -> Result<(), SessionPersistenceError> {
        if self.recovery_required {
            return Err(SessionPersistenceError::RecoveryRequired);
        }
        if self.opened.mode != OpenMode::Writable {
            return Err(SessionPersistenceError::SessionNotWritable);
        }
        Ok(())
    }

    fn ensure_readable(&self) -> Result<(), SessionPersistenceError> {
        if self.recovery_required {
            return Err(SessionPersistenceError::RecoveryRequired);
        }
        Ok(())
    }
}

fn map_reuse_error(error: crate::application_reuse::ApplicationReuseError) -> SessionPersistenceError {
    SessionPersistenceError::CanonicalMismatch(format!("{error:?}"))
}

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

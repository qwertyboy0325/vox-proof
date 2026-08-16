use std::path::{Path, PathBuf};

use crate::application_reuse::{
    PreparedActiveAnalysis, PreparedProjectScopeDisplayNameUpdate,
    PreparedProjectScopeInitialization, PreparedReusableInfluenceRevocation,
    PreparedReusableInfluenceSupersession, PreparedReuseCandidateAcceptance,
    PreparedReuseCandidateRejection, build_promotion_accepted_event,
    validate_accept_reuse_candidate_for_governance_commit,
};
use crate::application_service::{
    ApplicationMaterialUseDeclaration, ApplicationReviewSession, DeclaredSessionAuthority,
    PreparedHumanDecision, PreparedManualReplacement,
};
use crate::candidate::SessionTermEntry;
use crate::project_memory::{
    ProductProjectMemoryStore, ProjectMemoryError, ProjectMemoryOpenMode,
    ProjectMemorySnapshotIdentity,
};
use crate::reuse_primitives::ProjectScopeId;
use crate::review::ReviewCaseId;
use crate::session_persistence::canonical::{
    PRODUCT_SESSION_FORMAT_VERSION_V2, session_format_supports_human_raised,
};
use crate::session_persistence::error::SessionPersistenceError;
use crate::session_persistence::hydrate::{
    ProjectMemoryHydrateOverlay, hydrate_application_review_session,
};
use crate::session_persistence::store::{OpenMode, OpenedStoreSession, ProductSessionStore};
use crate::session_persistence::{
    load_bound_project_id, load_optional_bound_project_id, restore_project_scope,
};
use crate::transcript::Transcript;

pub struct DurableApplicationSession {
    session_id: String,
    store_root: PathBuf,
    opened: OpenedStoreSession,
    session: ApplicationReviewSession,
    recovery_required: bool,
    bound_project_id: Option<ProjectScopeId>,
    project_memory_available: bool,
    project_memory_snapshot: Option<ProjectMemorySnapshotIdentity>,
}

impl DurableApplicationSession {
    pub fn create(
        store: &ProductSessionStore,
        transcript: Transcript,
        session_terms: Vec<SessionTermEntry>,
        material_use: ApplicationMaterialUseDeclaration,
        session_authority: DeclaredSessionAuthority,
    ) -> Result<Self, SessionPersistenceError> {
        let (session_id, opened) =
            store.create_session(transcript, session_terms, material_use, session_authority)?;
        assemble_opened(store.root(), session_id, opened)
    }

    pub fn create_bound_to_project(
        store: &ProductSessionStore,
        project_store: &ProductProjectMemoryStore,
        project_id: &ProjectScopeId,
        transcript: Transcript,
        session_terms: Vec<SessionTermEntry>,
        material_use: ApplicationMaterialUseDeclaration,
        session_authority: DeclaredSessionAuthority,
    ) -> Result<Self, SessionPersistenceError> {
        let project = project_store
            .open(project_id, ProjectMemoryOpenMode::ReadOnly)
            .map_err(map_project_memory_error)?;
        let display_name = project.display_name().as_str().to_owned();
        project.close().map_err(map_project_memory_error)?;
        let (session_id, opened) = store.create_session_bound(
            transcript,
            session_terms,
            material_use,
            session_authority,
            project_id.as_str(),
            &display_name,
        )?;
        assemble_opened(store.root(), session_id, opened)
    }

    /// Compatibility-test constructor using the historical unbound format-1 create path.
    pub fn create_historical_unbound_format_v1_for_compatibility_test(
        store: &ProductSessionStore,
        transcript: Transcript,
        session_terms: Vec<SessionTermEntry>,
        material_use: ApplicationMaterialUseDeclaration,
        session_authority: DeclaredSessionAuthority,
    ) -> Result<Self, SessionPersistenceError> {
        let (session_id, opened) = store
            .create_historical_unbound_format_v1_for_compatibility_test(
                transcript,
                session_terms,
                material_use,
                session_authority,
            )?;
        assemble_opened(store.root(), session_id, opened)
    }

    /// Compatibility-test constructor using the historical bound format-2 create path.
    pub fn create_historical_bound_format_v2_for_compatibility_test(
        store: &ProductSessionStore,
        project_store: &ProductProjectMemoryStore,
        project_id: &ProjectScopeId,
        transcript: Transcript,
        session_terms: Vec<SessionTermEntry>,
        material_use: ApplicationMaterialUseDeclaration,
        session_authority: DeclaredSessionAuthority,
    ) -> Result<Self, SessionPersistenceError> {
        let project = project_store
            .open(project_id, ProjectMemoryOpenMode::ReadOnly)
            .map_err(map_project_memory_error)?;
        let display_name = project.display_name().as_str().to_owned();
        project.close().map_err(map_project_memory_error)?;
        let (session_id, opened) = store.create_historical_bound_format_v2_for_compatibility_test(
            transcript,
            session_terms,
            material_use,
            session_authority,
            project_id.as_str(),
            &display_name,
        )?;
        assemble_opened(store.root(), session_id, opened)
    }

    /// Compatibility-test constructor using the historical unbound format-3 create path.
    pub fn create_historical_unbound_format_v3_for_compatibility_test(
        store: &ProductSessionStore,
        transcript: Transcript,
        session_terms: Vec<SessionTermEntry>,
        material_use: ApplicationMaterialUseDeclaration,
        session_authority: DeclaredSessionAuthority,
    ) -> Result<Self, SessionPersistenceError> {
        let (session_id, opened) = store
            .create_historical_unbound_format_v3_for_compatibility_test(
                transcript,
                session_terms,
                material_use,
                session_authority,
            )?;
        assemble_opened(store.root(), session_id, opened)
    }

    /// Compatibility-test constructor using the historical bound format-3 create path.
    pub fn create_historical_bound_format_v3_for_compatibility_test(
        store: &ProductSessionStore,
        project_store: &ProductProjectMemoryStore,
        project_id: &ProjectScopeId,
        transcript: Transcript,
        session_terms: Vec<SessionTermEntry>,
        material_use: ApplicationMaterialUseDeclaration,
        session_authority: DeclaredSessionAuthority,
    ) -> Result<Self, SessionPersistenceError> {
        let project = project_store
            .open(project_id, ProjectMemoryOpenMode::ReadOnly)
            .map_err(map_project_memory_error)?;
        let display_name = project.display_name().as_str().to_owned();
        project.close().map_err(map_project_memory_error)?;
        let (session_id, opened) = store.create_historical_bound_format_v3_for_compatibility_test(
            transcript,
            session_terms,
            material_use,
            session_authority,
            project_id.as_str(),
            &display_name,
        )?;
        assemble_opened(store.root(), session_id, opened)
    }

    pub fn open(
        store: &ProductSessionStore,
        session_id: &str,
        mode: OpenMode,
    ) -> Result<Self, SessionPersistenceError> {
        let opened = store.open_session(session_id, mode)?;
        assemble_opened(store.root(), session_id.to_owned(), opened)
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn session(&self) -> &ApplicationReviewSession {
        &self.session
    }

    pub fn format_version(&self) -> u32 {
        self.opened.format_version
    }

    pub fn bound_project_id(&self) -> Option<&ProjectScopeId> {
        self.bound_project_id.as_ref()
    }

    pub fn project_memory_available(&self) -> bool {
        self.project_memory_available
    }

    pub fn project_memory_snapshot_identity(&self) -> Option<ProjectMemorySnapshotIdentity> {
        self.project_memory_snapshot
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
        let hydrated = hydrate_from_opened(&self.opened, &self.store_root)?;
        self.apply_hydrated(hydrated);
        self.recovery_required = false;
        Ok(())
    }

    pub fn record_human_decision(
        &mut self,
        prepared: PreparedHumanDecision,
    ) -> Result<(), SessionPersistenceError> {
        self.ensure_writable()?;
        if prepared.reuse_proposal_target.is_some()
            || prepared.terminology_proposal_target.is_some()
        {
            self.ensure_writable_reuse()?;
        }
        ProductSessionStore::append_review_ledger_event(&mut self.opened, &prepared)?;
        self.refresh_after_commit()
    }

    pub fn record_manual_replacement(
        &mut self,
        prepared: PreparedManualReplacement,
    ) -> Result<(), SessionPersistenceError> {
        self.record_human_decision(prepared.prepared)
    }

    pub fn raise_and_manual_replace(
        &mut self,
        segment_position: usize,
        start_byte: usize,
        end_byte: usize,
        replacement: impl Into<String>,
    ) -> Result<ReviewCaseId, SessionPersistenceError> {
        self.ensure_writable()?;
        if !session_format_supports_human_raised(self.opened.format_version) {
            return Err(SessionPersistenceError::Replay(
                crate::application_service::ApplicationServiceError::HumanRaisedRequiresFormatV3,
            ));
        }
        let expected_head = self.session.review_ledger_head();
        let case_id = match self.session.raise_and_manual_replace(
            segment_position,
            start_byte,
            end_byte,
            replacement,
        ) {
            Ok(case_id) => case_id,
            Err(error) => return Err(map_decision_error(error)),
        };
        let review_case = self
            .session
            .human_raised_cases()
            .last()
            .cloned()
            .ok_or_else(|| {
                SessionPersistenceError::CanonicalMismatch(
                    "human-raised case missing after raise".to_owned(),
                )
            })?;
        let events = self.session.review_ledger().events();
        if events.len() < 2 {
            let _ = self.rehydrate();
            return Err(SessionPersistenceError::CanonicalMismatch(
                "human-raised raise did not append CaseRaised and DecisionRecorded".to_owned(),
            ));
        }
        let case_raised = events[events.len() - 2].clone();
        let decision = events[events.len() - 1].clone();
        if let Err(error) = ProductSessionStore::append_human_raised_raise_and_decision(
            &mut self.opened,
            expected_head,
            &review_case,
            &case_raised,
            &decision,
        ) {
            let _ = self.rehydrate();
            return Err(error);
        }
        self.refresh_after_commit()?;
        Ok(case_id)
    }

    pub fn prepare_human_decision(
        &self,
        target: crate::application_service::ApplicationReviewTarget,
        decision: crate::review::CorrectionDecision,
    ) -> Result<PreparedHumanDecision, SessionPersistenceError> {
        self.ensure_readable()?;
        if matches!(
            target,
            crate::application_service::ApplicationReviewTarget::ProjectReuseProposal { .. }
                | crate::application_service::ApplicationReviewTarget::ProjectTerminologyProposal { .. }
        ) {
            self.ensure_writable_reuse()?;
        }
        self.session
            .prepare_human_decision(target, decision)
            .map_err(map_decision_error)
    }

    pub fn prepare_manual_replacement(
        &self,
        target: crate::application_service::ApplicationReviewTarget,
        replacement: impl Into<String>,
    ) -> Result<PreparedManualReplacement, SessionPersistenceError> {
        self.ensure_readable()?;
        if matches!(
            target,
            crate::application_service::ApplicationReviewTarget::ProjectReuseProposal { .. }
                | crate::application_service::ApplicationReviewTarget::ProjectTerminologyProposal { .. }
        ) {
            self.ensure_writable_reuse()?;
        }
        self.session
            .prepare_manual_replacement(target, replacement)
            .map_err(map_decision_error)
    }

    pub fn prepare_initialize_project_scope(
        &self,
        stable_id: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Result<PreparedProjectScopeInitialization, SessionPersistenceError> {
        self.ensure_readable()?;
        if self.bound_project_id.is_some() {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "v2 session is already bound to a project".to_owned(),
            ));
        }
        self.session
            .prepare_initialize_project_scope(stable_id, display_name)
            .map_err(map_reuse_error)
    }

    pub fn record_initialize_project_scope(
        &mut self,
        prepared: PreparedProjectScopeInitialization,
    ) -> Result<(), SessionPersistenceError> {
        self.ensure_writable()?;
        if self.bound_project_id.is_some() {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "v2 session is already bound to a project".to_owned(),
            ));
        }
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
        self.ensure_writable_reuse()?;
        self.session
            .prepare_accept_reuse_candidate(candidate_key)
            .map_err(map_reuse_error)
    }

    pub fn record_accept_reuse_candidate(
        &mut self,
        prepared: PreparedReuseCandidateAcceptance,
    ) -> Result<(), SessionPersistenceError> {
        self.ensure_writable()?;
        self.ensure_writable_reuse()?;
        if self.bound_project_id.is_some() {
            self.append_v2_promotion_to_project(&prepared)?;
            return self.refresh_after_commit();
        }
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
        self.ensure_writable_reuse()?;
        if self.bound_project_id.is_some() {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "v2 project memory P2 writes PromotionAccepted only".to_owned(),
            ));
        }
        self.session
            .prepare_reject_reuse_candidate(candidate_key)
            .map_err(map_reuse_error)
    }

    pub fn record_reject_reuse_candidate(
        &mut self,
        prepared: PreparedReuseCandidateRejection,
    ) -> Result<(), SessionPersistenceError> {
        self.ensure_writable()?;
        self.ensure_writable_reuse()?;
        if self.bound_project_id.is_some() {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "v2 project memory P2 writes PromotionAccepted only".to_owned(),
            ));
        }
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
        self.ensure_writable_reuse()?;
        if self.bound_project_id.is_some() {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "v2 project memory P2 writes PromotionAccepted only".to_owned(),
            ));
        }
        self.session
            .prepare_revoke_reusable_influence(record_id)
            .map_err(map_reuse_error)
    }

    pub fn record_revoke_reusable_influence(
        &mut self,
        prepared: PreparedReusableInfluenceRevocation,
    ) -> Result<(), SessionPersistenceError> {
        self.ensure_writable()?;
        self.ensure_writable_reuse()?;
        if self.bound_project_id.is_some() {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "v2 project memory P2 writes PromotionAccepted only".to_owned(),
            ));
        }
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
        self.ensure_writable_reuse()?;
        if self.bound_project_id.is_some() {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "v2 project memory P2 writes PromotionAccepted only".to_owned(),
            ));
        }
        self.session
            .prepare_supersede_reusable_influence(predecessor_id, successor_candidate_key)
            .map_err(map_reuse_error)
    }

    pub fn record_supersede_reusable_influence(
        &mut self,
        prepared: PreparedReusableInfluenceSupersession,
    ) -> Result<(), SessionPersistenceError> {
        self.ensure_writable()?;
        self.ensure_writable_reuse()?;
        if self.bound_project_id.is_some() {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "v2 project memory P2 writes PromotionAccepted only".to_owned(),
            ));
        }
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
        self.ensure_writable_reuse()?;
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
        self.ensure_writable_reuse()?;
        if self.bound_project_id.is_some() {
            ProductSessionStore::commit_v2_frozen_project_reuse(
                &mut self.opened,
                &prepared,
                &precondition_selection_token,
            )?;
        } else {
            ProductSessionStore::commit_active_analysis(
                &mut self.opened,
                &prepared,
                &precondition_selection_token,
            )?;
        }
        self.refresh_after_commit()
    }

    pub fn close(mut self) -> Result<(), SessionPersistenceError> {
        ProductSessionStore::release_writer(&mut self.opened)
    }

    fn append_v2_promotion_to_project(
        &self,
        prepared: &PreparedReuseCandidateAcceptance,
    ) -> Result<(), SessionPersistenceError> {
        let project_id = self
            .bound_project_id
            .as_ref()
            .ok_or(SessionPersistenceError::ProjectMemoryUnavailable)?;
        let project_store = ProductProjectMemoryStore::new(&self.store_root);
        let mut project = project_store
            .open(project_id, ProjectMemoryOpenMode::Writable)
            .map_err(map_project_memory_error)?;
        if project.records().len() != prepared.expected_reuse_governance_head {
            return Err(SessionPersistenceError::StaleAuthorityPrecondition(
                crate::session_persistence::error::StaleAuthorityPrecondition {
                    scope: crate::session_persistence::error::AuthorityScope::ReuseGovernance,
                },
            ));
        }
        let candidate = validate_accept_reuse_candidate_for_governance_commit(
            self.session.reuse_parts(),
            self.session.reuse_state(),
            &prepared.candidate_key,
        )
        .map_err(map_reuse_error)?;
        let project_scope = project.project_scope();
        let event = build_promotion_accepted_event(
            &candidate,
            &project_scope,
            self.session.session_authority(),
        );
        project
            .append_promotion(&self.session_id, event)
            .map_err(map_project_memory_error)?;
        project.close().map_err(map_project_memory_error)?;
        Ok(())
    }

    fn refresh_after_commit(&mut self) -> Result<(), SessionPersistenceError> {
        match hydrate_from_opened(&self.opened, &self.store_root) {
            Ok(hydrated) => {
                self.apply_hydrated(hydrated);
                self.recovery_required = false;
                Ok(())
            }
            Err(_error) => {
                self.recovery_required = true;
                Err(SessionPersistenceError::RecoveryRequired)
            }
        }
    }

    fn apply_hydrated(&mut self, hydrated: HydratedSession) {
        self.session = hydrated.session;
        self.bound_project_id = hydrated.bound_project_id;
        self.project_memory_available = hydrated.project_memory_available;
        self.project_memory_snapshot = hydrated.project_memory_snapshot;
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

    fn ensure_writable_reuse(&self) -> Result<(), SessionPersistenceError> {
        if self.bound_project_id.is_some() && !self.project_memory_available {
            return Err(SessionPersistenceError::WritableReuseBlocked);
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

struct HydratedSession {
    session: ApplicationReviewSession,
    bound_project_id: Option<ProjectScopeId>,
    project_memory_available: bool,
    project_memory_snapshot: Option<ProjectMemorySnapshotIdentity>,
}

fn assemble_opened(
    store_root: &Path,
    session_id: String,
    opened: OpenedStoreSession,
) -> Result<DurableApplicationSession, SessionPersistenceError> {
    let hydrated = hydrate_from_opened(&opened, store_root)?;
    Ok(DurableApplicationSession {
        session_id,
        store_root: store_root.to_path_buf(),
        opened,
        session: hydrated.session,
        recovery_required: false,
        bound_project_id: hydrated.bound_project_id,
        project_memory_available: hydrated.project_memory_available,
        project_memory_snapshot: hydrated.project_memory_snapshot,
    })
}

fn hydrate_from_opened(
    opened: &OpenedStoreSession,
    store_root: &Path,
) -> Result<HydratedSession, SessionPersistenceError> {
    let bound_project_id_raw = if opened.format_version == PRODUCT_SESSION_FORMAT_VERSION_V2 {
        Some(load_bound_project_id(
            &opened.connection,
            &opened.session_id,
        )?)
    } else if session_format_supports_human_raised(opened.format_version) {
        load_optional_bound_project_id(&opened.connection, &opened.session_id)?
    } else {
        None
    };
    let Some(project_id_raw) = bound_project_id_raw else {
        let session = hydrate_application_review_session(opened, None)?;
        return Ok(HydratedSession {
            session,
            bound_project_id: None,
            project_memory_available: true,
            project_memory_snapshot: None,
        });
    };
    let project_id = ProjectScopeId::new(project_id_raw.clone())
        .map_err(|_| SessionPersistenceError::CanonicalMismatch("bound project id".to_owned()))?;
    let project_store = ProductProjectMemoryStore::new(store_root);
    match project_store.open(&project_id, ProjectMemoryOpenMode::ReadOnly) {
        Ok(project) => {
            let overlay = ProjectMemoryHydrateOverlay {
                available: true,
                project_scope: project.project_scope(),
                records: project.records().to_vec(),
                current_snapshot: Some(project.snapshot_identity()),
            };
            let snapshot = Some(project.snapshot_identity());
            project.close().map_err(map_project_memory_error)?;
            let session = hydrate_application_review_session(opened, Some(&overlay))?;
            Ok(HydratedSession {
                session,
                bound_project_id: Some(project_id),
                project_memory_available: true,
                project_memory_snapshot: snapshot,
            })
        }
        Err(ProjectMemoryError::ProjectNotFound)
        | Err(ProjectMemoryError::UnsupportedFormatVersion { .. })
        | Err(ProjectMemoryError::CanonicalMismatch(_)) => {
            let capture = ProductSessionStore::load_canonical_capture(opened)?;
            let project_scope =
                restore_project_scope(&capture.project_scope)?.ok_or_else(|| {
                    SessionPersistenceError::CanonicalMismatch(
                        "v2 session missing bound project scope".to_owned(),
                    )
                })?;
            let overlay = ProjectMemoryHydrateOverlay {
                available: false,
                project_scope,
                records: Vec::new(),
                current_snapshot: None,
            };
            let session = hydrate_application_review_session(opened, Some(&overlay))?;
            Ok(HydratedSession {
                session,
                bound_project_id: Some(project_id),
                project_memory_available: false,
                project_memory_snapshot: None,
            })
        }
        Err(error) => Err(map_project_memory_error(error)),
    }
}

fn map_decision_error(
    error: crate::application_service::ApplicationServiceError,
) -> SessionPersistenceError {
    match error {
        crate::application_service::ApplicationServiceError::StaleReuseAnalysis => {
            SessionPersistenceError::StaleAuthorityPrecondition(
                crate::session_persistence::error::StaleAuthorityPrecondition {
                    scope: crate::session_persistence::error::AuthorityScope::ActiveAnalysis,
                },
            )
        }
        crate::application_service::ApplicationServiceError::ProjectEvidenceUnverifiable => {
            SessionPersistenceError::WritableReuseBlocked
        }
        other => SessionPersistenceError::Replay(other),
    }
}

fn map_reuse_error(
    error: crate::application_reuse::ApplicationReuseError,
) -> SessionPersistenceError {
    SessionPersistenceError::CanonicalMismatch(format!("{error:?}"))
}

fn map_project_memory_error(error: ProjectMemoryError) -> SessionPersistenceError {
    match error {
        ProjectMemoryError::ProjectNotFound => SessionPersistenceError::ProjectMemoryUnavailable,
        ProjectMemoryError::WriterOwnershipHeld => SessionPersistenceError::WriterOwnershipHeld,
        ProjectMemoryError::ProjectNotWritable => SessionPersistenceError::WritableReuseBlocked,
        other => SessionPersistenceError::ProjectMemory(other.to_string()),
    }
}

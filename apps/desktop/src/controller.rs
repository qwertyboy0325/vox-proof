use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use vox_proof::application_export::{
    render_application_decision_log, render_application_session_summary,
};
use vox_proof::application_export_v3::{
    render_application_decision_log_v3, render_application_session_summary_v3,
};
use vox_proof::application_reuse::ApplicationReuseError;
use vox_proof::application_service::{
    ApplicationCurrentProjection, ApplicationDecisionCoverage, ApplicationMaterialUseDeclaration,
    ApplicationResolutionStatus, ApplicationReviewProgress, ApplicationServiceError,
    DeclaredApplicationMaterialUseBasis, DeclaredSessionAuthority,
    DeclaredSessionAuthorityError, DeclaredSessionOperatorRole,
};
use vox_proof::candidate::{Evidence, SessionTermEntry};
use vox_proof::review::{CorrectionDecision, ReviewCaseStatus};
use vox_proof::session_persistence::{
    DurableApplicationSession, OpenMode, ProductSessionStore, SessionPersistenceError,
};
use vox_proof::session_terms::{SessionTermsError, parse_session_terms};
use vox_proof::srt::{ParseError, parse_srt};

use crate::export::{
    ExportError, ExportPaths, export_bundle_exclusively, export_bundle_v3_exclusively,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopPhase {
    Setup,
    ActiveReview,
    ExportCompleted,
    RecoveryRequired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewItemView {
    pub local_index: usize,
    pub cue_index: u32,
    pub source_text: String,
    pub context_before: Option<String>,
    pub context_after: Option<String>,
    pub alternatives: Vec<String>,
    pub evidence: String,
    pub detector: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionHeaderView {
    pub session_id: String,
    pub access_mode: String,
    pub source_path: String,
    pub source_path_note: Option<String>,
    pub source_revision: String,
    pub declared_operator: String,
    pub declared_role: String,
    pub total_review_cases: usize,
    pub total_recorded_events: usize,
    pub accepted: usize,
    pub manual_replacements: usize,
    pub rejected: usize,
    pub deferred: usize,
    pub needs_manual_correction: usize,
    pub undecided: usize,
}

#[derive(Debug)]
pub enum ControllerError {
    Io(std::io::Error),
    Transcript(ParseError),
    SessionTerms(SessionTermsError),
    Authority(DeclaredSessionAuthorityError),
    Service(ApplicationServiceError),
    Export(ExportError),
    Reuse(ApplicationReuseError),
    Persistence(SessionPersistenceError),
    NoActiveSession,
    StaleUiSessionEpoch { expected: u64, actual: u64 },
    NoSelectedCase,
    AlternativeOutOfRange { index: usize, count: usize },
    ReviewIncomplete { undecided: usize },
    UnresolvedConfirmationRequired,
    RecoveryRequired,
    SessionNotWritable,
    WriterOwnershipHeld,
}

impl fmt::Display for ControllerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "file I/O failed: {error}"),
            Self::Transcript(error) => write!(formatter, "SRT parsing failed: {error:?}"),
            Self::SessionTerms(error) => {
                write!(formatter, "session terms parsing failed: {error:?}")
            }
            Self::Authority(error) => write!(formatter, "operator declaration failed: {error}"),
            Self::Service(error) => write!(formatter, "review operation failed: {error}"),
            Self::Export(error) => write!(formatter, "export failed: {error}"),
            Self::Reuse(error) => write!(formatter, "reuse governance failed: {error}"),
            Self::Persistence(error) => write!(formatter, "session persistence failed: {error}"),
            Self::NoActiveSession => write!(formatter, "no active review session"),
            Self::StaleUiSessionEpoch { expected, actual } => write!(
                formatter,
                "stale UI intent refused (ui_session_epoch {expected}, active {actual})"
            ),
            Self::NoSelectedCase => write!(formatter, "no review case is selected"),
            Self::AlternativeOutOfRange { index, count } => write!(
                formatter,
                "alternative {} is unavailable; current case has {count}",
                index + 1
            ),
            Self::ReviewIncomplete { undecided } => write!(
                formatter,
                "review is incomplete: {undecided} undecided case(s)"
            ),
            Self::UnresolvedConfirmationRequired => write!(
                formatter,
                "complete unresolved review requires confirmation that source text is retained"
            ),
            Self::RecoveryRequired => write!(
                formatter,
                "recovery required: committed authority could not be reconstructed"
            ),
            Self::SessionNotWritable => write!(formatter, "session is read-only"),
            Self::WriterOwnershipHeld => write!(
                formatter,
                "writable access refused because another writer holds this session"
            ),
        }
    }
}

impl std::error::Error for ControllerError {}

impl From<std::io::Error> for ControllerError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<ApplicationReuseError> for ControllerError {
    fn from(value: ApplicationReuseError) -> Self {
        Self::Reuse(value)
    }
}

impl From<ApplicationServiceError> for ControllerError {
    fn from(value: ApplicationServiceError) -> Self {
        Self::Service(value)
    }
}

impl From<SessionPersistenceError> for ControllerError {
    fn from(value: SessionPersistenceError) -> Self {
        match value {
            SessionPersistenceError::RecoveryRequired => Self::RecoveryRequired,
            SessionPersistenceError::SessionNotWritable => Self::SessionNotWritable,
            SessionPersistenceError::WriterOwnershipHeld => Self::WriterOwnershipHeld,
            SessionPersistenceError::Replay(error) => Self::Service(error),
            other => Self::Persistence(other),
        }
    }
}

pub struct DesktopController {
    store: ProductSessionStore,
    durable: Option<DurableApplicationSession>,
    ui_session_epoch: u64,
    source_display_path: Option<PathBuf>,
    selected_index: usize,
    exported_paths: Option<ExportPaths>,
    project_scope_id_draft: String,
    project_scope_display_draft: String,
    available_session_ids: Vec<String>,
    selected_resume_session_id: Option<String>,
}

impl Default for DesktopController {
    fn default() -> Self {
        Self::with_store(ProductSessionStore::new(crate::session_root::resolve_session_root()))
    }
}

impl DesktopController {
    pub fn with_store(store: ProductSessionStore) -> Self {
        let available_session_ids = store.list_session_ids().unwrap_or_default();
        Self {
            store,
            durable: None,
            ui_session_epoch: 1,
            source_display_path: None,
            selected_index: 0,
            exported_paths: None,
            project_scope_id_draft: String::new(),
            project_scope_display_draft: String::new(),
            available_session_ids,
            selected_resume_session_id: None,
        }
    }

    pub fn store(&self) -> &ProductSessionStore {
        &self.store
    }

    pub fn phase(&self) -> DesktopPhase {
        if self
            .durable
            .as_ref()
            .is_some_and(|durable| durable.is_recovery_required())
        {
            DesktopPhase::RecoveryRequired
        } else if self.exported_paths.is_some() {
            DesktopPhase::ExportCompleted
        } else if self.durable.is_some() {
            DesktopPhase::ActiveReview
        } else {
            DesktopPhase::Setup
        }
    }

    pub fn ui_session_epoch(&self) -> u64 {
        self.ui_session_epoch
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn exported_paths(&self) -> Option<&ExportPaths> {
        self.exported_paths.as_ref()
    }

    pub fn available_session_ids(&self) -> &[String] {
        &self.available_session_ids
    }

    pub fn selected_resume_session_id(&self) -> Option<&str> {
        self.selected_resume_session_id.as_deref()
    }

    pub fn select_resume_session_id(&mut self, session_id: impl Into<String>) {
        self.selected_resume_session_id = Some(session_id.into());
    }

    pub fn refresh_available_sessions(&mut self) -> Result<(), ControllerError> {
        self.available_session_ids = self.store.list_session_ids()?;
        Ok(())
    }

    pub fn is_read_only(&self) -> bool {
        self.durable
            .as_ref()
            .is_some_and(|durable| durable.open_mode() == OpenMode::ReadOnly)
    }

    pub fn is_recovery_required(&self) -> bool {
        self.durable
            .as_ref()
            .is_some_and(|durable| durable.is_recovery_required())
    }

    pub fn mutations_enabled(&self) -> bool {
        self.durable.as_ref().is_some_and(|durable| {
            !durable.is_recovery_required() && durable.open_mode() == OpenMode::Writable
        })
    }

    pub fn session_id(&self) -> Option<&str> {
        self.durable.as_ref().map(|durable| durable.session_id())
    }

    pub fn start_from_paths(
        &mut self,
        transcript_path: &Path,
        terms_path: &Path,
        material_use: DeclaredApplicationMaterialUseBasis,
        role: DeclaredSessionOperatorRole,
        operator_label: &str,
    ) -> Result<(), ControllerError> {
        let transcript_text = fs::read_to_string(transcript_path)?;
        let terms_text = fs::read_to_string(terms_path)?;
        self.start_from_text(
            &transcript_text,
            &terms_text,
            Some(transcript_path.to_path_buf()),
            material_use,
            role,
            operator_label,
        )
    }

    pub fn start_from_text(
        &mut self,
        transcript_text: &str,
        terms_text: &str,
        source_display_path: Option<PathBuf>,
        material_use: DeclaredApplicationMaterialUseBasis,
        role: DeclaredSessionOperatorRole,
        operator_label: &str,
    ) -> Result<(), ControllerError> {
        let transcript = parse_srt(transcript_text).map_err(ControllerError::Transcript)?;
        let terms: Vec<SessionTermEntry> =
            parse_session_terms(terms_text).map_err(ControllerError::SessionTerms)?;
        let authority = DeclaredSessionAuthority::new(role, operator_label)
            .map_err(ControllerError::Authority)?;
        let durable = DurableApplicationSession::create(
            &self.store,
            transcript,
            terms,
            ApplicationMaterialUseDeclaration::new(material_use),
            authority,
        )?;
        self.install_durable_session(durable, source_display_path);
        self.refresh_available_sessions()?;
        Ok(())
    }

    pub fn open_selected_session_writable(&mut self) -> Result<(), ControllerError> {
        let session_id = self
            .selected_resume_session_id
            .clone()
            .ok_or(ControllerError::NoActiveSession)?;
        self.open_session(&session_id, OpenMode::Writable)
    }

    pub fn open_selected_session_read_only(&mut self) -> Result<(), ControllerError> {
        let session_id = self
            .selected_resume_session_id
            .clone()
            .ok_or(ControllerError::NoActiveSession)?;
        self.open_session(&session_id, OpenMode::ReadOnly)
    }

    pub fn open_session(
        &mut self,
        session_id: &str,
        mode: OpenMode,
    ) -> Result<(), ControllerError> {
        let durable = DurableApplicationSession::open(&self.store, session_id, mode)?;
        self.install_durable_session(durable, None);
        Ok(())
    }

    pub fn retry_recovery(&mut self) -> Result<(), ControllerError> {
        let durable = self
            .durable
            .as_mut()
            .ok_or(ControllerError::NoActiveSession)?;
        durable.rehydrate()?;
        Ok(())
    }

    pub fn reset(&mut self) -> Result<(), ControllerError> {
        if let Some(durable) = self.durable.as_mut() {
            durable.release_writer()?;
        }
        self.clear_session_state();
        self.ui_session_epoch = self.ui_session_epoch.wrapping_add(1);
        self.refresh_available_sessions()?;
        Ok(())
    }

    pub fn has_project_scope(&self) -> bool {
        self.presentable_session()
            .ok()
            .is_some_and(|session| session.has_project_scope())
    }

    pub fn project_scope_id_draft(&self) -> &str {
        &self.project_scope_id_draft
    }

    pub fn project_scope_id_draft_mut(&mut self) -> &mut String {
        &mut self.project_scope_id_draft
    }

    pub fn project_scope_display_draft(&self) -> &str {
        &self.project_scope_display_draft
    }

    pub fn project_scope_display_draft_mut(&mut self) -> &mut String {
        &mut self.project_scope_display_draft
    }

    pub fn reuse_enabled_case_count(&self) -> Option<usize> {
        self.presentable_session().ok().and_then(|session| {
            session
                .reuse_enabled_run()
                .map(|run| run.review_cases().len())
        })
    }

    pub fn initialize_project_scope(
        &mut self,
        expected_ui_session_epoch: u64,
    ) -> Result<(), ControllerError> {
        self.ensure_ui_epoch(expected_ui_session_epoch)?;
        let stable_id = self.project_scope_id_draft.clone();
        let display_name = self.project_scope_display_draft.clone();
        let durable = self.writable_durable_mut()?;
        let prepared = durable.prepare_initialize_project_scope(stable_id, display_name)?;
        durable.record_initialize_project_scope(prepared)?;
        self.exported_paths = None;
        Ok(())
    }

    pub fn update_project_scope_display_name(
        &mut self,
        expected_ui_session_epoch: u64,
    ) -> Result<(), ControllerError> {
        self.ensure_ui_epoch(expected_ui_session_epoch)?;
        let display_name = self.project_scope_display_draft.clone();
        let durable = self.writable_durable_mut()?;
        let prepared = durable.prepare_update_project_scope_display_name(display_name)?;
        durable.record_update_project_scope_display_name(prepared)?;
        if self.exported_paths.is_some() {
            self.exported_paths = None;
        }
        Ok(())
    }

    pub fn reuse_candidates(
        &self,
    ) -> Result<Vec<vox_proof::reusable_influence::ReuseCandidate>, ControllerError> {
        Ok(self.presentable_session()?.reuse_candidates()?)
    }

    pub fn active_reusable_records(
        &self,
    ) -> Result<Vec<vox_proof::reusable_influence::EffectiveReusableInfluenceRecord>, ControllerError>
    {
        Ok(self.presentable_session()?.active_reusable_records()?)
    }

    pub fn accept_reuse_candidate(
        &mut self,
        expected_ui_session_epoch: u64,
        candidate_key: &vox_proof::reusable_influence::ReuseCandidateKey,
    ) -> Result<(), ControllerError> {
        self.ensure_ui_epoch(expected_ui_session_epoch)?;
        let durable = self.writable_durable_mut()?;
        let prepared = durable.prepare_accept_reuse_candidate(candidate_key)?;
        durable.record_accept_reuse_candidate(prepared)?;
        self.exported_paths = None;
        Ok(())
    }

    pub fn reject_reuse_candidate(
        &mut self,
        expected_ui_session_epoch: u64,
        candidate_key: &vox_proof::reusable_influence::ReuseCandidateKey,
    ) -> Result<(), ControllerError> {
        self.ensure_ui_epoch(expected_ui_session_epoch)?;
        let durable = self.writable_durable_mut()?;
        let prepared = durable.prepare_reject_reuse_candidate(candidate_key)?;
        durable.record_reject_reuse_candidate(prepared)?;
        self.exported_paths = None;
        Ok(())
    }

    pub fn revoke_reusable_influence(
        &mut self,
        expected_ui_session_epoch: u64,
        record_id: vox_proof::reuse_primitives::ReusableInfluenceRecordId,
    ) -> Result<(), ControllerError> {
        self.ensure_ui_epoch(expected_ui_session_epoch)?;
        let durable = self.writable_durable_mut()?;
        let prepared = durable.prepare_revoke_reusable_influence(record_id)?;
        durable.record_revoke_reusable_influence(prepared)?;
        self.exported_paths = None;
        Ok(())
    }

    pub fn supersede_reusable_influence(
        &mut self,
        expected_ui_session_epoch: u64,
        predecessor_id: vox_proof::reuse_primitives::ReusableInfluenceRecordId,
        successor_candidate_key: &vox_proof::reusable_influence::ReuseCandidateKey,
    ) -> Result<(), ControllerError> {
        self.ensure_ui_epoch(expected_ui_session_epoch)?;
        let durable = self.writable_durable_mut()?;
        let prepared =
            durable.prepare_supersede_reusable_influence(predecessor_id, successor_candidate_key)?;
        durable.record_supersede_reusable_influence(prepared)?;
        self.exported_paths = None;
        Ok(())
    }

    pub fn run_reuse_enabled_analysis(
        &mut self,
        expected_ui_session_epoch: u64,
    ) -> Result<usize, ControllerError> {
        self.ensure_ui_epoch(expected_ui_session_epoch)?;
        let durable = self.writable_durable_mut()?;
        let (prepared, precondition) = durable.prepare_run_reuse_enabled_review()?;
        durable.record_run_reuse_enabled_review(prepared, precondition)?;
        let count = durable
            .session()
            .reuse_enabled_run()
            .map(|run| run.review_cases().len())
            .unwrap_or(0);
        self.exported_paths = None;
        Ok(count)
    }

    pub fn select(&mut self, index: usize) {
        if let Ok(session) = self.presentable_session() {
            let count = session.review_items().len();
            if count > 0 {
                self.selected_index = index.min(count - 1);
            }
        }
    }

    pub fn select_relative(&mut self, delta: isize) {
        let Ok(session) = self.presentable_session() else {
            return;
        };
        let count = session.review_items().len();
        if count == 0 {
            self.selected_index = 0;
            return;
        }
        self.selected_index =
            (self.selected_index as isize + delta).clamp(0, count as isize - 1) as usize;
    }

    pub fn items(&self) -> Result<Vec<ReviewItemView>, ControllerError> {
        let session = self.presentable_session()?;
        let segments = session.source().segments();
        Ok(session
            .review_items()
            .into_iter()
            .map(|item| {
                let candidate = item.review_case.candidate_span();
                let position = candidate.anchor().segment_position();
                let segment = &segments[position];
                ReviewItemView {
                    local_index: item.review_case.id().local_index(),
                    cue_index: segment.index(),
                    source_text: session
                        .source()
                        .resolve(candidate.anchor())
                        .unwrap_or_default()
                        .to_owned(),
                    context_before: position
                        .checked_sub(1)
                        .and_then(|index| segments.get(index))
                        .map(|segment| segment.text().to_owned()),
                    context_after: segments
                        .get(position + 1)
                        .map(|segment| segment.text().to_owned()),
                    alternatives: candidate
                        .alternatives()
                        .iter()
                        .map(|alternative| alternative.replacement_text().to_owned())
                        .collect(),
                    evidence: evidence_label(candidate.evidence()),
                    detector: format!(
                        "{} @ {}",
                        candidate.provenance().detector_id(),
                        candidate.provenance().detector_version()
                    ),
                    status: status_label(item.status),
                }
            })
            .collect())
    }

    pub fn header(&self) -> Result<SessionHeaderView, ControllerError> {
        let durable = self.durable.as_ref().ok_or(ControllerError::NoActiveSession)?;
        if durable.is_recovery_required() {
            return Err(ControllerError::RecoveryRequired);
        }
        let session = durable.session();
        let summary = session.decision_summary();
        let authority = session.session_authority();
        let role = match authority.role() {
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator => {
                "Declared local owner/operator"
            }
            DeclaredSessionOperatorRole::DeclaredAuthorizedHumanReviewer => {
                "Declared authorized human reviewer"
            }
        };
        let (source_path, source_path_note) = match &self.source_display_path {
            Some(path) => (path.display().to_string(), None),
            None => (
                "Embedded canonical transcript".to_owned(),
                Some("Original import path not retained".to_owned()),
            ),
        };
        Ok(SessionHeaderView {
            session_id: durable.session_id().to_owned(),
            access_mode: match durable.open_mode() {
                OpenMode::Writable => "Writable".to_owned(),
                OpenMode::ReadOnly => "Read-only".to_owned(),
            },
            source_path,
            source_path_note,
            source_revision: session.source().revision_id().to_tagged_string(),
            declared_operator: authority.display_label().to_owned(),
            declared_role: role.to_owned(),
            total_review_cases: summary.total_review_cases,
            total_recorded_events: summary.total_recorded_events,
            accepted: summary.accepted_alternatives,
            manual_replacements: summary.manual_replacements,
            rejected: summary.rejected,
            deferred: summary.deferred,
            needs_manual_correction: summary.needs_manual_correction,
            undecided: summary.undecided,
        })
    }

    pub fn projection(&self) -> Result<ApplicationCurrentProjection, ControllerError> {
        self.presentable_session()?
            .derive_current_projection()
            .map_err(ControllerError::Service)
    }

    pub fn progress(&self) -> Result<ApplicationReviewProgress, ControllerError> {
        Ok(self.presentable_session()?.progress())
    }

    pub fn record_decision(
        &mut self,
        expected_ui_session_epoch: u64,
        decision: CorrectionDecision,
    ) -> Result<(), ControllerError> {
        self.ensure_ui_epoch(expected_ui_session_epoch)?;
        let selected_index = self.selected_index;
        {
            let durable = self.writable_durable_mut()?;
            let items = durable.session().review_items();
            let item = items
                .get(selected_index)
                .ok_or(ControllerError::NoSelectedCase)?;
            if let CorrectionDecision::AcceptAlternative { alternative_index } = decision {
                let count = item.review_case.candidate_span().alternatives().len();
                if alternative_index >= count {
                    return Err(ControllerError::AlternativeOutOfRange {
                        index: alternative_index,
                        count,
                    });
                }
            }
            let prepared = durable.prepare_human_decision(item.target, decision)?;
            durable.record_human_decision(prepared)?;
        }
        self.exported_paths = None;

        let refreshed = self
            .presentable_session()?
            .review_items();
        if let Some(next) = refreshed
            .iter()
            .position(|item| matches!(item.status, ReviewCaseStatus::Undecided))
        {
            self.selected_index = next;
        }
        Ok(())
    }

    pub fn record_manual_replacement(
        &mut self,
        expected_ui_session_epoch: u64,
        replacement: impl Into<String>,
    ) -> Result<(), ControllerError> {
        self.ensure_ui_epoch(expected_ui_session_epoch)?;
        let selected_index = self.selected_index;
        {
            let durable = self.writable_durable_mut()?;
            let items = durable.session().review_items();
            let item = items
                .get(selected_index)
                .ok_or(ControllerError::NoSelectedCase)?;
            let prepared = durable.prepare_manual_replacement(item.target, replacement)?;
            durable.record_manual_replacement(prepared)?;
        }
        self.exported_paths = None;

        let refreshed = self
            .presentable_session()?
            .review_items();
        if let Some(next) = refreshed
            .iter()
            .position(|item| matches!(item.status, ReviewCaseStatus::Undecided))
        {
            self.selected_index = next;
        }
        Ok(())
    }

    pub fn export(
        &mut self,
        expected_ui_session_epoch: u64,
        destination: &Path,
        confirm_unresolved_source_retained: bool,
    ) -> Result<ExportPaths, ControllerError> {
        self.ensure_ui_epoch(expected_ui_session_epoch)?;
        let session = self.presentable_session()?;
        match session.progress().decision_coverage {
            ApplicationDecisionCoverage::Complete => {}
            ApplicationDecisionCoverage::Incomplete { undecided } => {
                return Err(ControllerError::ReviewIncomplete { undecided });
            }
        }
        if matches!(
            session.progress().resolution_status,
            ApplicationResolutionStatus::Unresolved { .. }
        ) && !confirm_unresolved_source_retained
        {
            return Err(ControllerError::UnresolvedConfirmationRequired);
        }

        let bundle = session.materialize_review_export_bundle()?;
        let paths = if session.has_project_scope() {
            let bundle_v3 = session
                .materialize_review_export_bundle_v3()
                .map_err(gate3_error)?;
            export_bundle_v3_exclusively(
                &bundle,
                &bundle_v3,
                destination,
                self.source_display_path.as_deref(),
            )
            .map_err(ControllerError::Export)?
        } else {
            export_bundle_exclusively(&bundle, destination, self.source_display_path.as_deref())
                .map_err(ControllerError::Export)?
        };
        self.exported_paths = Some(paths.clone());
        Ok(paths)
    }

    pub fn export_previews(
        &self,
        confirm_unresolved_source_retained: bool,
    ) -> Result<(String, String), ControllerError> {
        let session = self.presentable_session()?;
        match session.progress().decision_coverage {
            ApplicationDecisionCoverage::Complete => {}
            ApplicationDecisionCoverage::Incomplete { undecided } => {
                return Err(ControllerError::ReviewIncomplete { undecided });
            }
        }
        if matches!(
            session.progress().resolution_status,
            ApplicationResolutionStatus::Unresolved { .. }
        ) && !confirm_unresolved_source_retained
        {
            return Err(ControllerError::UnresolvedConfirmationRequired);
        }
        let bundle = session.materialize_review_export_bundle()?;
        if session.has_project_scope() {
            let bundle_v3 = session
                .materialize_review_export_bundle_v3()
                .map_err(gate3_error)?;
            Ok((
                render_application_decision_log_v3(&bundle_v3),
                render_application_session_summary_v3(&bundle_v3),
            ))
        } else {
            Ok((
                render_application_decision_log(&bundle),
                render_application_session_summary(&bundle),
            ))
        }
    }

    fn install_durable_session(
        &mut self,
        durable: DurableApplicationSession,
        source_display_path: Option<PathBuf>,
    ) {
        if let Some(mut existing) = self.durable.take() {
            let _ = existing.release_writer();
        }
        self.ui_session_epoch = self.ui_session_epoch.wrapping_add(1);
        self.durable = Some(durable);
        self.source_display_path = source_display_path;
        self.selected_index = 0;
        self.exported_paths = None;
        self.project_scope_id_draft.clear();
        self.project_scope_display_draft.clear();
    }

    fn clear_session_state(&mut self) {
        self.durable = None;
        self.source_display_path = None;
        self.selected_index = 0;
        self.exported_paths = None;
        self.project_scope_id_draft.clear();
        self.project_scope_display_draft.clear();
        self.selected_resume_session_id = None;
    }

    fn ensure_ui_epoch(&self, expected: u64) -> Result<(), ControllerError> {
        if expected != self.ui_session_epoch {
            return Err(ControllerError::StaleUiSessionEpoch {
                expected,
                actual: self.ui_session_epoch,
            });
        }
        Ok(())
    }

    fn presentable_session(
        &self,
    ) -> Result<&vox_proof::application_service::ApplicationReviewSession, ControllerError> {
        let durable = self.durable.as_ref().ok_or(ControllerError::NoActiveSession)?;
        if durable.is_recovery_required() {
            return Err(ControllerError::RecoveryRequired);
        }
        Ok(durable.session())
    }

    fn writable_durable_mut(&mut self) -> Result<&mut DurableApplicationSession, ControllerError> {
        let durable = self.durable.as_mut().ok_or(ControllerError::NoActiveSession)?;
        if durable.is_recovery_required() {
            return Err(ControllerError::RecoveryRequired);
        }
        if durable.open_mode() == OpenMode::ReadOnly {
            return Err(ControllerError::SessionNotWritable);
        }
        Ok(durable)
    }
}

impl Drop for DesktopController {
    fn drop(&mut self) {
        if let Some(mut durable) = self.durable.take() {
            let _ = durable.release_writer();
        }
    }
}

fn gate3_error(error: vox_proof::application_service::ApplicationGate3Error) -> ControllerError {
    match error {
        vox_proof::application_service::ApplicationGate3Error::Reuse(error) => {
            ControllerError::Reuse(error)
        }
        vox_proof::application_service::ApplicationGate3Error::Service(error) => {
            ControllerError::Service(error)
        }
    }
}

fn evidence_label(evidence: &Evidence) -> String {
    match evidence {
        Evidence::GlossaryAlias(item) => {
            format!("Glossary alias match: {}", item.matched_form)
        }
        Evidence::ObservedErrorForm(item) => {
            format!("Observed error form: {}", item.matched_form)
        }
        Evidence::PhoneticSimilarity(item) => format!(
            "Phonetic similarity: {} → {} (distance {})",
            item.observed_surface, item.target_surface, item.comparison.edit_distance
        ),
        Evidence::ReusableExactObservedForm(item) => format!(
            "Reusable exact observed form: {} → {}",
            item.observed_text, item.confirmed_replacement
        ),
    }
}

fn status_label(status: ReviewCaseStatus) -> String {
    match status {
        ReviewCaseStatus::Undecided => "Undecided".to_owned(),
        ReviewCaseStatus::Decided { decision, .. } => match decision {
            CorrectionDecision::Reject => "Rejected".to_owned(),
            CorrectionDecision::Defer => "Deferred".to_owned(),
            CorrectionDecision::AcceptAlternative { alternative_index } => {
                format!("Accepted alternative {}", alternative_index + 1)
            }
            CorrectionDecision::NeedsManualCorrection => "Needs manual correction".to_owned(),
            CorrectionDecision::ManualReplacement { .. } => {
                "Manual replacement recorded".to_owned()
            }
        },
    }
}

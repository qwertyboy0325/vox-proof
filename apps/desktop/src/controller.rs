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
    ApplicationResolutionStatus, ApplicationReviewItemKind, ApplicationReviewProgress,
    ApplicationReviewTarget, ApplicationServiceError, DeclaredApplicationMaterialUseBasis,
    DeclaredSessionAuthority, DeclaredSessionAuthorityError, DeclaredSessionOperatorRole,
};
use vox_proof::candidate::{Evidence, SessionTermEntry};
use vox_proof::project_memory::{
    ProductProjectMemoryStore, ProjectListSummary, ProjectMemoryError,
};
use vox_proof::reusable_influence::{ReusableGovernanceEvent, ReuseCandidate};
use vox_proof::reuse_primitives::ProjectScopeId;
use vox_proof::review::{CorrectionDecision, ReviewCaseStatus};
use vox_proof::session_persistence::{
    DurableApplicationSession, OpenMode, ProductSessionStore, SessionListSummary,
    SessionPersistenceError,
};
use vox_proof::session_terms::{SessionTermsError, parse_session_terms};
use vox_proof::srt::{ParseError, parse_srt};

use crate::desktop_presentation::DesktopPresentation;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewItemOrigin {
    TermSuggestion {
        also_supported_by_previous_correction: bool,
        disagrees_with_previous_correction: bool,
    },
    PreviousCorrection {
        conflict_with_canonical: bool,
    },
    ProjectTerminology {
        conflict_with_canonical: bool,
    },
    HumanRaisedCorrection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CueView {
    pub segment_position: usize,
    pub cue_number: u32,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewItemView {
    pub queue_index: usize,
    pub local_index: usize,
    pub cue_index: u32,
    pub segment_position: usize,
    pub start_ms: u64,
    pub source_text: String,
    pub context_before: Option<String>,
    pub context_after: Option<String>,
    pub alternatives: Vec<String>,
    pub evidence: String,
    pub detector: String,
    pub status: String,
    pub origin: ReviewItemOrigin,
    pub uses_reuse_proposal_target: bool,
    pub reuse_decision_blocked: bool,
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
    pub project_name: Option<String>,
    pub bound_to_project: bool,
    pub project_memory_available: bool,
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
    WritableReuseBlocked,
    ProjectMemoryUnavailable,
    ProjectMemory(ProjectMemoryError),
    NoPromotionCandidate,
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
            Self::WritableReuseBlocked => write!(
                formatter,
                "project memory is unavailable; new reuse actions are blocked"
            ),
            Self::ProjectMemoryUnavailable => {
                write!(formatter, "project memory is unavailable")
            }
            Self::ProjectMemory(error) => write!(formatter, "project memory failed: {error}"),
            Self::NoPromotionCandidate => {
                write!(
                    formatter,
                    "no reusable correction is available for this item"
                )
            }
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
            SessionPersistenceError::WritableReuseBlocked => Self::WritableReuseBlocked,
            SessionPersistenceError::ProjectMemoryUnavailable => Self::ProjectMemoryUnavailable,
            SessionPersistenceError::Replay(error) => Self::Service(error),
            other => Self::Persistence(other),
        }
    }
}

impl From<ProjectMemoryError> for ControllerError {
    fn from(value: ProjectMemoryError) -> Self {
        Self::ProjectMemory(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionResumeView {
    pub session_id: String,
    pub created_at_unix_ms: i64,
    pub authority_display_label: String,
    pub review_case_count: usize,
    pub review_ledger_head: usize,
    pub source_display_name: String,
    pub project_display_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectChoiceView {
    pub project_id: ProjectScopeId,
    pub display_name: String,
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectMemoryEntryView {
    pub observed_text: String,
    pub confirmed_replacement: String,
    pub source_review_label: Option<String>,
    pub provenance_available: bool,
}

pub struct DesktopController {
    store: ProductSessionStore,
    durable: Option<DurableApplicationSession>,
    ui_session_epoch: u64,
    source_display_path: Option<PathBuf>,
    source_display_name: Option<String>,
    selected_index: usize,
    exported_paths: Option<ExportPaths>,
    project_scope_id_draft: String,
    project_scope_display_draft: String,
    available_sessions: Vec<SessionResumeView>,
    selected_resume_session_id: Option<String>,
    available_projects: Vec<ProjectChoiceView>,
    selected_project_id: Option<ProjectScopeId>,
}

impl Default for DesktopController {
    fn default() -> Self {
        Self::with_store(ProductSessionStore::new(
            crate::session_root::resolve_session_root(),
        ))
    }
}

impl DesktopController {
    pub fn with_store(store: ProductSessionStore) -> Self {
        let mut controller = Self {
            store,
            durable: None,
            ui_session_epoch: 1,
            source_display_path: None,
            source_display_name: None,
            selected_index: 0,
            exported_paths: None,
            project_scope_id_draft: String::new(),
            project_scope_display_draft: String::new(),
            available_sessions: Vec::new(),
            selected_resume_session_id: None,
            available_projects: Vec::new(),
            selected_project_id: None,
        };
        let _ = controller.refresh_available_sessions_internal();
        let _ = controller.refresh_available_projects_internal();
        controller
    }

    fn refresh_available_sessions_internal(&mut self) -> Result<(), ControllerError> {
        let summaries = self.store.list_session_summaries()?;
        self.available_sessions = summaries
            .into_iter()
            .map(|summary| self.resume_view_from_summary(summary))
            .collect();
        Ok(())
    }

    fn resume_view_from_summary(&self, summary: SessionListSummary) -> SessionResumeView {
        let source_display_name =
            DesktopPresentation::read_source_display_name(self.store.root(), &summary.session_id)
                .unwrap_or_else(|| short_session_label(&summary.session_id));
        SessionResumeView {
            session_id: summary.session_id,
            created_at_unix_ms: summary.created_at_unix_ms,
            authority_display_label: summary.authority_display_label,
            review_case_count: summary.review_case_count,
            review_ledger_head: summary.review_ledger_head,
            source_display_name,
            project_display_name: summary.project_display_name,
        }
    }

    fn persist_presentation_sidecar(&self, session_id: &str) {
        if let Some(name) = self
            .source_display_name
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            DesktopPresentation::write_best_effort(self.store.root(), session_id, name);
        }
    }

    fn load_presentation_for_session(&mut self, session_id: &str) {
        self.source_display_name =
            DesktopPresentation::read_source_display_name(self.store.root(), session_id);
        self.source_display_path = None;
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

    pub fn available_sessions(&self) -> &[SessionResumeView] {
        &self.available_sessions
    }

    pub fn selected_resume_session_id(&self) -> Option<&str> {
        self.selected_resume_session_id.as_deref()
    }

    pub fn select_resume_session_id(&mut self, session_id: impl Into<String>) {
        self.selected_resume_session_id = Some(session_id.into());
    }

    pub fn refresh_available_sessions(&mut self) -> Result<(), ControllerError> {
        self.refresh_available_sessions_internal()
    }

    fn project_store(&self) -> ProductProjectMemoryStore {
        ProductProjectMemoryStore::new(self.store.root())
    }

    fn refresh_available_projects_internal(&mut self) -> Result<(), ControllerError> {
        self.available_projects = self
            .project_store()
            .list_projects()?
            .into_iter()
            .map(|summary: ProjectListSummary| ProjectChoiceView {
                project_id: summary.project_id,
                display_name: summary.display_name,
                created_at_unix_ms: summary.created_at_unix_ms,
            })
            .collect();
        Ok(())
    }

    pub fn refresh_available_projects(&mut self) -> Result<(), ControllerError> {
        self.refresh_available_projects_internal()
    }

    pub fn available_projects(&self) -> &[ProjectChoiceView] {
        &self.available_projects
    }

    pub fn selected_project_id(&self) -> Option<&ProjectScopeId> {
        self.selected_project_id.as_ref()
    }

    pub fn select_project(&mut self, project_id: ProjectScopeId) {
        self.selected_project_id = Some(project_id);
    }

    pub fn clear_selected_project(&mut self) {
        self.selected_project_id = None;
    }

    pub fn create_project(
        &mut self,
        display_name: impl Into<String>,
    ) -> Result<ProjectScopeId, ControllerError> {
        let project = self.project_store().create(display_name)?;
        let project_id = project.project_id().clone();
        project.close()?;
        self.refresh_available_projects_internal()?;
        self.selected_project_id = Some(project_id.clone());
        Ok(project_id)
    }

    pub fn is_bound_to_project(&self) -> bool {
        self.durable
            .as_ref()
            .is_some_and(|durable| durable.bound_project_id().is_some())
    }

    pub fn project_memory_available(&self) -> bool {
        self.durable
            .as_ref()
            .is_some_and(|durable| durable.project_memory_available())
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

    pub fn human_raised_available(&self) -> bool {
        self.durable
            .as_ref()
            .is_some_and(|durable| durable.format_version() >= 3)
    }

    pub fn cue_texts(&self) -> Result<Vec<(usize, String)>, ControllerError> {
        Ok(self
            .cues()?
            .into_iter()
            .map(|cue| (cue.segment_position, cue.text))
            .collect())
    }

    pub fn cues(&self) -> Result<Vec<CueView>, ControllerError> {
        let session = self.presentable_session()?;
        Ok(session
            .source()
            .segments()
            .iter()
            .enumerate()
            .map(|(position, segment)| CueView {
                segment_position: position,
                cue_number: segment.index(),
                start_ms: segment.start_ms(),
                end_ms: segment.end_ms(),
                text: segment.text().to_owned(),
            })
            .collect())
    }

    pub fn raise_and_manual_replace(
        &mut self,
        expected_ui_session_epoch: u64,
        segment_position: usize,
        start_byte: usize,
        end_byte: usize,
        replacement: impl Into<String>,
    ) -> Result<(), ControllerError> {
        self.ensure_ui_epoch(expected_ui_session_epoch)?;
        let case_id = {
            let durable = self.writable_durable_mut()?;
            durable.raise_and_manual_replace(segment_position, start_byte, end_byte, replacement)?
        };
        self.exported_paths = None;
        let refreshed = self.presentable_session()?.review_items();
        if let Some(index) = refreshed
            .iter()
            .position(|item| item.review_case.id() == case_id)
        {
            self.selected_index = index;
        }
        Ok(())
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
        let terms = parse_session_terms(&terms_text).map_err(ControllerError::SessionTerms)?;
        self.start_from_transcript_and_terms(
            transcript_path,
            &transcript_text,
            terms,
            material_use,
            role,
            operator_label,
        )
    }

    pub fn start_from_transcript_and_terms(
        &mut self,
        transcript_path: &Path,
        transcript_text: &str,
        terms: Vec<SessionTermEntry>,
        material_use: DeclaredApplicationMaterialUseBasis,
        role: DeclaredSessionOperatorRole,
        operator_label: &str,
    ) -> Result<(), ControllerError> {
        let transcript = parse_srt(transcript_text).map_err(ControllerError::Transcript)?;
        let authority = DeclaredSessionAuthority::new(role, operator_label)
            .map_err(ControllerError::Authority)?;
        let durable = DurableApplicationSession::create(
            &self.store,
            transcript,
            terms,
            ApplicationMaterialUseDeclaration::new(material_use),
            authority,
        )?;
        let display_name = transcript_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned());
        self.install_durable_session(durable, Some(transcript_path.to_path_buf()), display_name);
        self.maybe_freeze_bound_reuse()?;
        if let Some(session_id) = self.session_id() {
            self.persist_presentation_sidecar(session_id);
        }
        self.refresh_available_sessions_internal()?;
        Ok(())
    }

    pub fn start_from_transcript_and_terms_in_project(
        &mut self,
        transcript_path: &Path,
        transcript_text: &str,
        terms: Vec<SessionTermEntry>,
        material_use: DeclaredApplicationMaterialUseBasis,
        role: DeclaredSessionOperatorRole,
        operator_label: &str,
        project_id: &ProjectScopeId,
    ) -> Result<(), ControllerError> {
        let transcript = parse_srt(transcript_text).map_err(ControllerError::Transcript)?;
        let authority = DeclaredSessionAuthority::new(role, operator_label)
            .map_err(ControllerError::Authority)?;
        let project_store = self.project_store();
        let durable = DurableApplicationSession::create_bound_to_project(
            &self.store,
            &project_store,
            project_id,
            transcript,
            terms,
            ApplicationMaterialUseDeclaration::new(material_use),
            authority,
        )?;
        let display_name = transcript_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned());
        self.install_durable_session(durable, Some(transcript_path.to_path_buf()), display_name);
        self.maybe_freeze_bound_reuse()?;
        if let Some(session_id) = self.session_id() {
            self.persist_presentation_sidecar(session_id);
        }
        self.refresh_available_sessions_internal()?;
        Ok(())
    }

    pub fn start_from_text_in_project(
        &mut self,
        transcript_text: &str,
        terms_text: &str,
        source_display_path: Option<PathBuf>,
        material_use: DeclaredApplicationMaterialUseBasis,
        role: DeclaredSessionOperatorRole,
        operator_label: &str,
        project_id: &ProjectScopeId,
    ) -> Result<(), ControllerError> {
        let terms = parse_session_terms(terms_text).map_err(ControllerError::SessionTerms)?;
        let transcript_path = source_display_path
            .clone()
            .unwrap_or_else(|| PathBuf::from("transcript.srt"));
        self.start_from_transcript_and_terms_in_project(
            &transcript_path,
            transcript_text,
            terms,
            material_use,
            role,
            operator_label,
            project_id,
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
        let terms = parse_session_terms(terms_text).map_err(ControllerError::SessionTerms)?;
        let transcript_path = source_display_path
            .clone()
            .unwrap_or_else(|| PathBuf::from("transcript.srt"));
        self.start_from_transcript_and_terms(
            &transcript_path,
            transcript_text,
            terms,
            material_use,
            role,
            operator_label,
        )
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
        self.install_durable_session(durable, None, None);
        self.load_presentation_for_session(session_id);
        self.maybe_freeze_bound_reuse()?;
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
        self.refresh_available_projects_internal()?;
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

    pub fn use_selected_correction_in_related_reviews(
        &mut self,
        expected_ui_session_epoch: u64,
    ) -> Result<(), ControllerError> {
        let candidate = self
            .promotion_candidate_for_selected()?
            .ok_or(ControllerError::NoPromotionCandidate)?;
        self.accept_reuse_candidate(expected_ui_session_epoch, &candidate.key)
    }

    pub fn promotion_candidate_for_selected(
        &self,
    ) -> Result<Option<ReuseCandidate>, ControllerError> {
        let session = self.presentable_session()?;
        let items = session.review_items();
        if items.is_empty() {
            return Ok(None);
        }
        let item = items
            .get(self.selected_index)
            .ok_or(ControllerError::NoSelectedCase)?;
        if !matches!(
            item.target,
            ApplicationReviewTarget::CanonicalTermCase { .. }
                | ApplicationReviewTarget::HumanRaisedCase { .. }
        ) {
            return Ok(None);
        }
        if !matches!(
            item.status,
            ReviewCaseStatus::Decided {
                decision: CorrectionDecision::ManualReplacement { .. },
                ..
            }
        ) {
            return Ok(None);
        }
        let case_id = item.review_case.id();
        Ok(session
            .reuse_candidates()?
            .into_iter()
            .find(|candidate| candidate.key.source_locator.source_review_case_id() == case_id))
    }

    pub fn project_memory_entries(&self) -> Result<Vec<ProjectMemoryEntryView>, ControllerError> {
        let durable = self
            .durable
            .as_ref()
            .ok_or(ControllerError::NoActiveSession)?;
        if durable.is_recovery_required() {
            return Err(ControllerError::RecoveryRequired);
        }
        let available = durable.project_memory_available();
        let session = durable.session();
        let mut entries = Vec::new();
        for record in session.project_memory_records() {
            let ReusableGovernanceEvent::PromotionAccepted { payload, .. } = &record.event else {
                continue;
            };
            let source_review_label = DesktopPresentation::read_source_display_name(
                self.store.root(),
                &record.source_session_id,
            );
            entries.push(ProjectMemoryEntryView {
                observed_text: payload.observed_text.clone(),
                confirmed_replacement: payload.confirmed_replacement.clone(),
                source_review_label,
                provenance_available: available,
            });
        }
        Ok(entries)
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
        let prepared = durable
            .prepare_supersede_reusable_influence(predecessor_id, successor_candidate_key)?;
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
        let derived = session.derived_reuse_proposal_targets();
        let project_available = session.project_memory_available();
        Ok(session
            .review_items()
            .into_iter()
            .enumerate()
            .map(|(queue_index, item)| {
                let detector_span = item.review_case.as_detector_span();
                let anchor = item.review_case.source_anchor();
                let position = anchor.segment_position();
                let segment = &segments[position];
                let occurrence = (
                    anchor.segment_position(),
                    anchor.start_byte(),
                    anchor.end_byte(),
                );
                let canonical_replacement = detector_span
                    .and_then(|span| span.alternatives().first())
                    .map(|alternative| alternative.replacement_text());
                let origin = match item.kind {
                    ApplicationReviewItemKind::CanonicalTermCase { .. } => {
                        let also_supported = derived.iter().any(|target| {
                            target.occurrence_key() == occurrence
                                && Some(target.proposed_replacement()) == canonical_replacement
                        });
                        let disagrees = derived.iter().any(|target| {
                            target.occurrence_key() == occurrence
                                && Some(target.proposed_replacement()) != canonical_replacement
                        });
                        ReviewItemOrigin::TermSuggestion {
                            also_supported_by_previous_correction: also_supported,
                            disagrees_with_previous_correction: disagrees,
                        }
                    }
                    ApplicationReviewItemKind::HumanRaisedCase {} => {
                        ReviewItemOrigin::HumanRaisedCorrection
                    }
                    ApplicationReviewItemKind::ProjectReuseProposal {
                        conflict_with_canonical,
                        ..
                    } => ReviewItemOrigin::PreviousCorrection {
                        conflict_with_canonical,
                    },
                    ApplicationReviewItemKind::ProjectTerminologyProposal {
                        conflict_with_canonical,
                    } => ReviewItemOrigin::ProjectTerminology {
                        conflict_with_canonical,
                    },
                };
                let uses_reuse_proposal_target = matches!(
                    item.target,
                    ApplicationReviewTarget::ProjectReuseProposal { .. }
                );
                let uses_terminology_proposal_target = matches!(
                    item.target,
                    ApplicationReviewTarget::ProjectTerminologyProposal { .. }
                );
                ReviewItemView {
                    queue_index,
                    local_index: item.review_case.id().local_index(),
                    cue_index: segment.index(),
                    segment_position: position,
                    start_ms: segment.start_ms(),
                    source_text: session
                        .source()
                        .resolve(&anchor)
                        .unwrap_or_default()
                        .to_owned(),
                    context_before: position
                        .checked_sub(1)
                        .and_then(|index| segments.get(index))
                        .map(|segment| segment.text().to_owned()),
                    context_after: segments
                        .get(position + 1)
                        .map(|segment| segment.text().to_owned()),
                    alternatives: detector_span
                        .map(|span| {
                            span.alternatives()
                                .iter()
                                .map(|alternative| alternative.replacement_text().to_owned())
                                .collect()
                        })
                        .unwrap_or_default(),
                    evidence: detector_span
                        .map(|span| evidence_label(span.evidence()))
                        .unwrap_or_else(|| "Text you selected".to_owned()),
                    detector: detector_span
                        .map(|span| friendly_detector_label(span.provenance().detector_id()))
                        .unwrap_or_else(|| "You".to_owned()),
                    status: status_label(item.status.clone()),
                    origin,
                    uses_reuse_proposal_target,
                    reuse_decision_blocked: (uses_reuse_proposal_target
                        || uses_terminology_proposal_target)
                        && matches!(item.status, ReviewCaseStatus::Undecided)
                        && !project_available,
                }
            })
            .collect())
    }

    pub fn header(&self) -> Result<SessionHeaderView, ControllerError> {
        let durable = self
            .durable
            .as_ref()
            .ok_or(ControllerError::NoActiveSession)?;
        if durable.is_recovery_required() {
            return Err(ControllerError::RecoveryRequired);
        }
        let session = durable.session();
        let summary = session.decision_summary();
        let authority = session.session_authority();
        let role = match authority.role() {
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator => {
                "I own or manage this material"
            }
            DeclaredSessionOperatorRole::DeclaredAuthorizedHumanReviewer => "Authorized reviewer",
        };
        let (source_path, source_path_note) = if let Some(name) = &self.source_display_name {
            (name.clone(), None)
        } else {
            match &self.source_display_path {
                Some(path) => (path.display().to_string(), None),
                None => (
                    "Saved review".to_owned(),
                    Some("Original file name is unavailable for this session.".to_owned()),
                ),
            }
        };
        Ok(SessionHeaderView {
            session_id: durable.session_id().to_owned(),
            access_mode: match durable.open_mode() {
                OpenMode::Writable => "Editable".to_owned(),
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
            project_name: session.project_display_name().map(str::to_owned),
            bound_to_project: durable.bound_project_id().is_some(),
            project_memory_available: durable.project_memory_available(),
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
                let count = item
                    .review_case
                    .as_detector_span()
                    .map(|span| span.alternatives().len())
                    .unwrap_or(0);
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

        let refreshed = self.presentable_session()?.review_items();
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

        let refreshed = self.presentable_session()?.review_items();
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
        source_display_name: Option<String>,
    ) {
        if let Some(mut existing) = self.durable.take() {
            let _ = existing.release_writer();
        }
        self.ui_session_epoch = self.ui_session_epoch.wrapping_add(1);
        self.durable = Some(durable);
        self.source_display_path = source_display_path;
        self.source_display_name = source_display_name.or_else(|| {
            self.source_display_path
                .as_ref()
                .and_then(|path| path.file_name())
                .map(|name| name.to_string_lossy().into_owned())
        });
        self.selected_index = 0;
        self.exported_paths = None;
        self.project_scope_id_draft.clear();
        self.project_scope_display_draft.clear();
    }

    fn clear_session_state(&mut self) {
        self.durable = None;
        self.source_display_path = None;
        self.source_display_name = None;
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
        let durable = self
            .durable
            .as_ref()
            .ok_or(ControllerError::NoActiveSession)?;
        if durable.is_recovery_required() {
            return Err(ControllerError::RecoveryRequired);
        }
        Ok(durable.session())
    }

    fn writable_durable_mut(&mut self) -> Result<&mut DurableApplicationSession, ControllerError> {
        let durable = self
            .durable
            .as_mut()
            .ok_or(ControllerError::NoActiveSession)?;
        if durable.is_recovery_required() {
            return Err(ControllerError::RecoveryRequired);
        }
        if durable.open_mode() == OpenMode::ReadOnly {
            return Err(ControllerError::SessionNotWritable);
        }
        Ok(durable)
    }

    fn maybe_freeze_bound_reuse(&mut self) -> Result<(), ControllerError> {
        let should_freeze = {
            let Some(durable) = self.durable.as_ref() else {
                return Ok(());
            };
            if durable.is_recovery_required() || durable.open_mode() != OpenMode::Writable {
                return Ok(());
            }
            durable.bound_project_id().is_some()
                && durable.project_memory_available()
                && !durable.session().compose_project_reuse_proposals()
        };
        if !should_freeze {
            return Ok(());
        }
        let durable = self.writable_durable_mut()?;
        let (prepared, precondition) = durable.prepare_run_reuse_enabled_review()?;
        durable.record_run_reuse_enabled_review(prepared, precondition)?;
        self.exported_paths = None;
        Ok(())
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
            format!("Matches alias: {}", item.matched_form)
        }
        Evidence::ObservedErrorForm(item) => {
            format!("Known misrecognition: {}", item.matched_form)
        }
        Evidence::PhoneticSimilarity(item) => format!(
            "Sounds like {} (near {})",
            item.observed_surface, item.target_surface
        ),
        Evidence::ReusableExactObservedForm(item) => format!(
            "Previously confirmed: {} → {}",
            item.observed_text, item.confirmed_replacement
        ),
    }
}

fn status_label(status: ReviewCaseStatus) -> String {
    match status {
        ReviewCaseStatus::Undecided => "Needs review".to_owned(),
        ReviewCaseStatus::Decided { decision, .. } => match decision {
            CorrectionDecision::Reject => "Rejected".to_owned(),
            CorrectionDecision::Defer => "Deferred".to_owned(),
            CorrectionDecision::AcceptAlternative { alternative_index } => {
                format!("Accepted suggestion {}", alternative_index + 1)
            }
            CorrectionDecision::NeedsManualCorrection => "Needs manual correction".to_owned(),
            CorrectionDecision::ManualReplacement { .. } => "Manually corrected".to_owned(),
        },
    }
}

fn friendly_detector_label(detector_id: &str) -> String {
    match detector_id {
        "exact-alias" => "Term alias check".to_owned(),
        "observed-error-form" => "Known misrecognition check".to_owned(),
        "ascii-latin-phonetic-similarity-v0" => "Similar spelling check".to_owned(),
        "reusable-exact-observed-form" => "Saved correction check".to_owned(),
        other => other.to_owned(),
    }
}

fn short_session_label(session_id: &str) -> String {
    if session_id.len() <= 8 {
        session_id.to_owned()
    } else {
        format!("{}…", &session_id[..8])
    }
}

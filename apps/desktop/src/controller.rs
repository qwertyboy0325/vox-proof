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
    ApplicationResolutionStatus, ApplicationReviewProgress, ApplicationReviewSession,
    ApplicationServiceError, DeclaredApplicationMaterialUseBasis, DeclaredSessionAuthority,
    DeclaredSessionAuthorityError, DeclaredSessionOperatorRole, begin_application_review,
};
use vox_proof::candidate::{Evidence, SessionTermEntry};
use vox_proof::review::{CorrectionDecision, ReviewCaseStatus};
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
    pub source_path: String,
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
    NoActiveSession,
    StaleGeneration { expected: u64, actual: u64 },
    NoSelectedCase,
    AlternativeOutOfRange { index: usize, count: usize },
    ReviewIncomplete { undecided: usize },
    UnresolvedConfirmationRequired,
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
            Self::NoActiveSession => write!(formatter, "no active review session"),
            Self::StaleGeneration { expected, actual } => write!(
                formatter,
                "stale UI intent refused (generation {expected}, active {actual})"
            ),
            Self::NoSelectedCase => write!(formatter, "no review case is selected"),
            Self::AlternativeOutOfRange { index, count } => write!(
                formatter,
                "alternative {} is unavailable; current case has {count}",
                index + 1
            ),
            Self::ReviewIncomplete { undecided } => {
                write!(
                    formatter,
                    "review is incomplete: {undecided} undecided case(s)"
                )
            }
            Self::UnresolvedConfirmationRequired => write!(
                formatter,
                "complete unresolved review requires confirmation that source text is retained"
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

pub struct DesktopController {
    generation: u64,
    session: Option<ApplicationReviewSession>,
    source_path: Option<PathBuf>,
    declared_operator: Option<String>,
    declared_role: Option<DeclaredSessionOperatorRole>,
    selected_index: usize,
    exported_paths: Option<ExportPaths>,
    project_scope_id_draft: String,
    project_scope_display_draft: String,
    reuse_enabled_case_count: Option<usize>,
}

impl Default for DesktopController {
    fn default() -> Self {
        Self {
            generation: 1,
            session: None,
            source_path: None,
            declared_operator: None,
            declared_role: None,
            selected_index: 0,
            exported_paths: None,
            project_scope_id_draft: String::new(),
            project_scope_display_draft: String::new(),
            reuse_enabled_case_count: None,
        }
    }
}

impl DesktopController {
    pub fn phase(&self) -> DesktopPhase {
        if self.exported_paths.is_some() {
            DesktopPhase::ExportCompleted
        } else if self.session.is_some() {
            DesktopPhase::ActiveReview
        } else {
            DesktopPhase::Setup
        }
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn exported_paths(&self) -> Option<&ExportPaths> {
        self.exported_paths.as_ref()
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
            transcript_path.to_path_buf(),
            material_use,
            role,
            operator_label,
        )
    }

    pub fn start_from_text(
        &mut self,
        transcript_text: &str,
        terms_text: &str,
        source_path: PathBuf,
        material_use: DeclaredApplicationMaterialUseBasis,
        role: DeclaredSessionOperatorRole,
        operator_label: &str,
    ) -> Result<(), ControllerError> {
        let transcript = parse_srt(transcript_text).map_err(ControllerError::Transcript)?;
        let terms: Vec<SessionTermEntry> =
            parse_session_terms(terms_text).map_err(ControllerError::SessionTerms)?;
        let authority = DeclaredSessionAuthority::new(role, operator_label)
            .map_err(ControllerError::Authority)?;
        let session = begin_application_review(
            transcript,
            terms,
            ApplicationMaterialUseDeclaration::new(material_use),
            authority,
        )?;

        self.generation = self.generation.wrapping_add(1);
        self.session = Some(session);
        self.source_path = Some(source_path);
        self.declared_operator = Some(operator_label.trim().to_owned());
        self.declared_role = Some(role);
        self.selected_index = 0;
        self.exported_paths = None;
        self.project_scope_id_draft.clear();
        self.project_scope_display_draft.clear();
        self.reuse_enabled_case_count = None;
        Ok(())
    }

    pub fn reset(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.session = None;
        self.source_path = None;
        self.declared_operator = None;
        self.declared_role = None;
        self.selected_index = 0;
        self.exported_paths = None;
        self.project_scope_id_draft.clear();
        self.project_scope_display_draft.clear();
        self.reuse_enabled_case_count = None;
    }

    pub fn has_project_scope(&self) -> bool {
        self.session
            .as_ref()
            .map(|session| session.has_project_scope())
            .unwrap_or(false)
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
        self.reuse_enabled_case_count
    }

    pub fn initialize_project_scope(
        &mut self,
        expected_generation: u64,
    ) -> Result<(), ControllerError> {
        if expected_generation != self.generation {
            return Err(ControllerError::StaleGeneration {
                expected: expected_generation,
                actual: self.generation,
            });
        }
        let session = self
            .session
            .as_mut()
            .ok_or(ControllerError::NoActiveSession)?;
        let stable_id = self.project_scope_id_draft.clone();
        let display_name = self.project_scope_display_draft.clone();
        session.initialize_project_scope(stable_id, display_name)?;
        self.exported_paths = None;
        Ok(())
    }

    pub fn update_project_scope_display_name(
        &mut self,
        expected_generation: u64,
    ) -> Result<(), ControllerError> {
        if expected_generation != self.generation {
            return Err(ControllerError::StaleGeneration {
                expected: expected_generation,
                actual: self.generation,
            });
        }
        let session = self
            .session
            .as_mut()
            .ok_or(ControllerError::NoActiveSession)?;
        session.update_project_scope_display_name(self.project_scope_display_draft.clone())?;
        if self.exported_paths.is_some() {
            self.exported_paths = None;
        }
        Ok(())
    }

    pub fn reuse_candidates(
        &self,
    ) -> Result<Vec<vox_proof::reusable_influence::ReuseCandidate>, ControllerError> {
        let session = self
            .session
            .as_ref()
            .ok_or(ControllerError::NoActiveSession)?;
        Ok(session.reuse_candidates()?)
    }

    pub fn active_reusable_records(
        &self,
    ) -> Result<Vec<vox_proof::reusable_influence::EffectiveReusableInfluenceRecord>, ControllerError>
    {
        let session = self
            .session
            .as_ref()
            .ok_or(ControllerError::NoActiveSession)?;
        Ok(session.active_reusable_records()?)
    }

    pub fn accept_reuse_candidate(
        &mut self,
        expected_generation: u64,
        candidate_key: &vox_proof::reusable_influence::ReuseCandidateKey,
    ) -> Result<(), ControllerError> {
        if expected_generation != self.generation {
            return Err(ControllerError::StaleGeneration {
                expected: expected_generation,
                actual: self.generation,
            });
        }
        let session = self
            .session
            .as_mut()
            .ok_or(ControllerError::NoActiveSession)?;
        session.accept_reuse_candidate(candidate_key)?;
        self.exported_paths = None;
        Ok(())
    }

    pub fn reject_reuse_candidate(
        &mut self,
        expected_generation: u64,
        candidate_key: &vox_proof::reusable_influence::ReuseCandidateKey,
    ) -> Result<(), ControllerError> {
        if expected_generation != self.generation {
            return Err(ControllerError::StaleGeneration {
                expected: expected_generation,
                actual: self.generation,
            });
        }
        let session = self
            .session
            .as_mut()
            .ok_or(ControllerError::NoActiveSession)?;
        session.reject_reuse_candidate(candidate_key)?;
        self.exported_paths = None;
        Ok(())
    }

    pub fn revoke_reusable_influence(
        &mut self,
        expected_generation: u64,
        record_id: vox_proof::reuse_primitives::ReusableInfluenceRecordId,
    ) -> Result<(), ControllerError> {
        if expected_generation != self.generation {
            return Err(ControllerError::StaleGeneration {
                expected: expected_generation,
                actual: self.generation,
            });
        }
        let session = self
            .session
            .as_mut()
            .ok_or(ControllerError::NoActiveSession)?;
        session.revoke_reusable_influence(record_id)?;
        self.exported_paths = None;
        Ok(())
    }

    pub fn supersede_reusable_influence(
        &mut self,
        expected_generation: u64,
        predecessor_id: vox_proof::reuse_primitives::ReusableInfluenceRecordId,
        successor_candidate_key: &vox_proof::reusable_influence::ReuseCandidateKey,
    ) -> Result<(), ControllerError> {
        if expected_generation != self.generation {
            return Err(ControllerError::StaleGeneration {
                expected: expected_generation,
                actual: self.generation,
            });
        }
        let session = self
            .session
            .as_mut()
            .ok_or(ControllerError::NoActiveSession)?;
        session.supersede_reusable_influence(predecessor_id, successor_candidate_key)?;
        self.exported_paths = None;
        Ok(())
    }

    pub fn run_reuse_enabled_analysis(
        &mut self,
        expected_generation: u64,
    ) -> Result<usize, ControllerError> {
        if expected_generation != self.generation {
            return Err(ControllerError::StaleGeneration {
                expected: expected_generation,
                actual: self.generation,
            });
        }
        let session = self
            .session
            .as_mut()
            .ok_or(ControllerError::NoActiveSession)?;
        let run = session.run_reuse_enabled_review()?;
        let count = run.review_cases().len();
        self.reuse_enabled_case_count = Some(count);
        self.exported_paths = None;
        Ok(count)
    }

    pub fn select(&mut self, index: usize) {
        if let Some(session) = &self.session {
            let count = session.review_items().len();
            if count > 0 {
                self.selected_index = index.min(count - 1);
            }
        }
    }

    pub fn select_relative(&mut self, delta: isize) {
        let Some(session) = &self.session else {
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
        let session = self
            .session
            .as_ref()
            .ok_or(ControllerError::NoActiveSession)?;
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
        let session = self
            .session
            .as_ref()
            .ok_or(ControllerError::NoActiveSession)?;
        let summary = session.decision_summary();
        let role = match self.declared_role {
            Some(DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator) => {
                "Declared local owner/operator"
            }
            Some(DeclaredSessionOperatorRole::DeclaredAuthorizedHumanReviewer) => {
                "Declared authorized human reviewer"
            }
            None => "Unavailable",
        };
        Ok(SessionHeaderView {
            source_path: self
                .source_path
                .as_deref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "Unavailable".to_owned()),
            source_revision: session.source().revision_id().to_tagged_string(),
            declared_operator: self
                .declared_operator
                .clone()
                .unwrap_or_else(|| "Unavailable".to_owned()),
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
        self.session
            .as_ref()
            .ok_or(ControllerError::NoActiveSession)?
            .derive_current_projection()
            .map_err(ControllerError::Service)
    }

    pub fn progress(&self) -> Result<ApplicationReviewProgress, ControllerError> {
        Ok(self
            .session
            .as_ref()
            .ok_or(ControllerError::NoActiveSession)?
            .progress())
    }

    pub fn record_decision(
        &mut self,
        expected_generation: u64,
        decision: CorrectionDecision,
    ) -> Result<(), ControllerError> {
        if expected_generation != self.generation {
            return Err(ControllerError::StaleGeneration {
                expected: expected_generation,
                actual: self.generation,
            });
        }
        let session = self
            .session
            .as_mut()
            .ok_or(ControllerError::NoActiveSession)?;

        // Targets are intentionally obtained from a fresh application-service
        // projection at the moment of the decision and never retained in GUI state.
        let items = session.review_items();
        let item = items
            .get(self.selected_index)
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
        session.record_human_decision(item.target, decision)?;
        self.exported_paths = None;

        let refreshed = session.review_items();
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
        expected_generation: u64,
        replacement: impl Into<String>,
    ) -> Result<(), ControllerError> {
        if expected_generation != self.generation {
            return Err(ControllerError::StaleGeneration {
                expected: expected_generation,
                actual: self.generation,
            });
        }
        let session = self
            .session
            .as_mut()
            .ok_or(ControllerError::NoActiveSession)?;

        let items = session.review_items();
        let item = items
            .get(self.selected_index)
            .ok_or(ControllerError::NoSelectedCase)?;
        session.record_manual_replacement(item.target, replacement)?;
        self.exported_paths = None;

        let refreshed = session.review_items();
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
        expected_generation: u64,
        destination: &Path,
        confirm_unresolved_source_retained: bool,
    ) -> Result<ExportPaths, ControllerError> {
        if expected_generation != self.generation {
            return Err(ControllerError::StaleGeneration {
                expected: expected_generation,
                actual: self.generation,
            });
        }
        let session = self
            .session
            .as_ref()
            .ok_or(ControllerError::NoActiveSession)?;
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
                self.source_path.as_deref(),
            )
            .map_err(ControllerError::Export)?
        } else {
            export_bundle_exclusively(&bundle, destination, self.source_path.as_deref())
                .map_err(ControllerError::Export)?
        };
        self.exported_paths = Some(paths.clone());
        Ok(paths)
    }

    pub fn export_previews(
        &self,
        confirm_unresolved_source_retained: bool,
    ) -> Result<(String, String), ControllerError> {
        let session = self
            .session
            .as_ref()
            .ok_or(ControllerError::NoActiveSession)?;
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

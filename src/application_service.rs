use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;

use crate::analysis::AnalysisSnapshot;
use crate::anchor::TranscriptRevisionId;
use crate::application_export_v3::{ApplicationReviewExportBundleV3, build_export_bundle_v3};
use crate::application_reuse::{
    ApplicationReuseError, ApplicationReuseState, ReuseSessionParts, accept_reuse_candidate,
    active_reusable_records, initialize_project_scope, reject_reuse_candidate,
    reuse_candidates_for_parts, revoke_reusable_influence, run_reuse_enabled_review_for_parts,
    supersede_reusable_influence, update_project_scope_display_name,
};
use crate::candidate::{DetectionError, DetectionKind, SessionTermEntry};
use crate::pipeline::{
    CanonicalTermReviewRun, ReuseEnabledTermReviewRun, run_canonical_term_review,
};
use crate::project_memory::{ProjectMemoryRecord, ProjectMemorySnapshotIdentity};
use crate::reuse_proposal_target::{
    FrozenProjectReuseAnalysis, ReuseProposalTarget, ReuseProposalTargetIdentity,
    derive_reuse_proposal_targets,
};
use crate::review::{
    CorrectionDecision, ManualReplacementText, ManualReplacementTextError, ReviewCase,
    ReviewCaseId, ReviewCaseStatus, ReviewLedger, ReviewLedgerError, ReviewLedgerEvent,
};
use crate::reviewed_output::{ReviewedOutputError, derive_reviewed_srt_with_reuse};
use crate::transcript::Transcript;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclaredSessionOperatorRole {
    DeclaredLocalOwnerOperator,
    DeclaredAuthorizedHumanReviewer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredSessionAuthority {
    role: DeclaredSessionOperatorRole,
    display_label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclaredSessionAuthorityError {
    EmptyDisplayLabel,
    ControlCharacterInDisplayLabel,
    UnicodeLineSeparatorInDisplayLabel,
    NonCanonicalDisplayLabel,
}

impl fmt::Display for DeclaredSessionAuthorityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for DeclaredSessionAuthorityError {}

impl DeclaredSessionAuthority {
    pub fn new(
        role: DeclaredSessionOperatorRole,
        display_label: impl Into<String>,
    ) -> Result<Self, DeclaredSessionAuthorityError> {
        let display_label =
            crate::reusable_influence::canonicalize_actor_display_label_for_declaration(
                display_label,
            )
            .map_err(map_actor_label_error)?;

        Ok(Self {
            role,
            display_label,
        })
    }

    pub const fn role(&self) -> DeclaredSessionOperatorRole {
        self.role
    }

    pub fn display_label(&self) -> &str {
        &self.display_label
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BoundDeclaredSessionAuthority {
    authority: DeclaredSessionAuthority,
    source_revision: TranscriptRevisionId,
    analysis_snapshot: AnalysisSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclaredApplicationMaterialUseBasis {
    SelfOwned,
    ExplicitPermission,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplicationMaterialUseDeclaration {
    basis: DeclaredApplicationMaterialUseBasis,
}

impl ApplicationMaterialUseDeclaration {
    pub const fn new(basis: DeclaredApplicationMaterialUseBasis) -> Self {
        Self { basis }
    }

    pub const fn basis(&self) -> DeclaredApplicationMaterialUseBasis {
        self.basis
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BoundApplicationMaterialUseDeclaration {
    declaration: ApplicationMaterialUseDeclaration,
    source_revision: TranscriptRevisionId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationReviewTarget {
    CanonicalTermCase {
        analysis_snapshot: AnalysisSnapshot,
        case_id: ReviewCaseId,
    },
    ProjectReuseProposal {
        reuse_analysis_snapshot: AnalysisSnapshot,
        target_identity: ReuseProposalTargetIdentity,
    },
}

impl ApplicationReviewTarget {
    pub fn case_id(self) -> Option<ReviewCaseId> {
        match self {
            Self::CanonicalTermCase { case_id, .. } => Some(case_id),
            Self::ProjectReuseProposal { .. } => None,
        }
    }

    pub fn analysis_snapshot(self) -> AnalysisSnapshot {
        match self {
            Self::CanonicalTermCase {
                analysis_snapshot, ..
            } => analysis_snapshot,
            Self::ProjectReuseProposal {
                reuse_analysis_snapshot,
                ..
            } => reuse_analysis_snapshot,
        }
    }

    pub fn reuse_proposal_identity(self) -> Option<ReuseProposalTargetIdentity> {
        match self {
            Self::ProjectReuseProposal {
                target_identity, ..
            } => Some(target_identity),
            Self::CanonicalTermCase { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationReviewItemKind {
    CanonicalTermCase {
        gate7_repeated_correction_avoided_eligible: bool,
    },
    ProjectReuseProposal {
        gate7_repeated_correction_avoided_eligible: bool,
        conflict_with_canonical: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationReviewItem {
    pub target: ApplicationReviewTarget,
    pub review_case: ReviewCase,
    pub status: ReviewCaseStatus,
    pub kind: ApplicationReviewItemKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationDecisionCoverage {
    Complete,
    Incomplete { undecided: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationResolutionStatus {
    Resolved,
    Unresolved {
        deferred: usize,
        needs_manual_correction: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplicationReviewProgress {
    pub decision_coverage: ApplicationDecisionCoverage,
    pub resolution_status: ApplicationResolutionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationDecisionSummary {
    pub total_review_cases: usize,
    pub total_recorded_events: usize,
    pub accepted_alternatives: usize,
    pub manual_replacements: usize,
    pub rejected: usize,
    pub deferred: usize,
    pub needs_manual_correction: usize,
    pub undecided: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationCurrentProjection {
    pub srt: String,
    pub progress: ApplicationReviewProgress,
    pub decision_summary: ApplicationDecisionSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationReviewedOutput {
    pub srt: String,
    pub progress: ApplicationReviewProgress,
    pub decision_summary: ApplicationDecisionSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationDecisionProjectionRecord {
    pub event_index: usize,
    pub case_id: Option<ReviewCaseId>,
    pub reuse_proposal_target_identity: Option<ReuseProposalTargetIdentity>,
    pub observed_revision: TranscriptRevisionId,
    pub decision: CorrectionDecision,
    pub session_authority: DeclaredSessionAuthority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationExportPosture {
    DeclaredOperatorUnauthenticatedInMemoryV0_2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationDetectionKindCount {
    pub kind: DetectionKind,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationDetectorCount {
    pub detector_id: String,
    pub detector_version: String,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationAcceptedReplacementCount {
    pub replacement_text: String,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationSessionOutcomeCounts {
    pub accepted_replacements_materialized: usize,
    pub source_segments_affected: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationSessionSummaryProjection {
    pub source_revision: TranscriptRevisionId,
    pub transcript_segments: usize,
    pub session_term_entry_count: usize,
    pub review_cases_raised: usize,
    pub cases_by_detection_kind: Vec<ApplicationDetectionKindCount>,
    pub cases_by_detector: Vec<ApplicationDetectorCount>,
    pub outcomes: ApplicationSessionOutcomeCounts,
    pub accepted_replacements: Vec<ApplicationAcceptedReplacementCount>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationReviewExportBundle {
    pub reviewed_srt: String,
    pub progress: ApplicationReviewProgress,
    pub decision_summary: ApplicationDecisionSummary,
    pub decision_records: Vec<ApplicationDecisionProjectionRecord>,
    pub declared_session_authority: DeclaredSessionAuthority,
    pub material_use_basis: DeclaredApplicationMaterialUseBasis,
    pub source_revision: TranscriptRevisionId,
    pub analysis_snapshot: AnalysisSnapshot,
    pub session_summary: ApplicationSessionSummaryProjection,
    pub export_posture: ApplicationExportPosture,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ApplicationServiceError {
    Detection(DetectionError),
    TargetAnalysisMismatch,
    UnknownReviewCase { case_id: ReviewCaseId },
    UnknownReuseProposal,
    StaleReuseAnalysis,
    ProjectEvidenceUnverifiable,
    Decision(ReviewLedgerError),
    ManualReplacement(ManualReplacementTextError),
    DecisionCoverageIncomplete { undecided: usize },
    ReviewedOutput(ReviewedOutputError),
}

impl fmt::Display for ApplicationServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ApplicationServiceError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationReplayField {
    AnalysisSnapshot,
    ReviewCases,
    LedgerEvents,
    EffectiveStatuses,
    Progress,
    DecisionSummary,
    SessionSummary,
    CurrentProjection,
    ReviewedOutput,
    ExportBundle,
    ReuseGovernanceLedger,
    ReuseEffectiveState,
    ReuseSnapshotIdentity,
    ReuseEnabledRun,
    ExportBundleV3,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ApplicationReplayError {
    Service(ApplicationServiceError),
    Mismatch { field: ApplicationReplayField },
}

impl fmt::Display for ApplicationReplayError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ApplicationReplayError {}

#[derive(Debug, PartialEq, Eq)]
pub enum ApplicationGate3Error {
    Reuse(ApplicationReuseError),
    Service(ApplicationServiceError),
}

impl fmt::Display for ApplicationGate3Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ApplicationGate3Error {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedHumanDecision {
    pub target: ApplicationReviewTarget,
    pub decision: CorrectionDecision,
    pub expected_review_ledger_head: usize,
    pub reuse_proposal_target: Option<ReuseProposalTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedManualReplacement {
    pub prepared: PreparedHumanDecision,
}

pub struct ApplicationReviewSession {
    transcript: Transcript,
    session_terms: Vec<SessionTermEntry>,
    canonical_run: CanonicalTermReviewRun,
    ledger: ReviewLedger,
    material_use: BoundApplicationMaterialUseDeclaration,
    session_authority: BoundDeclaredSessionAuthority,
    reuse_state: ApplicationReuseState,
    reuse_enabled_run: Option<ReuseEnabledTermReviewRun>,
    project_reuse: ProjectReuseSessionState,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ProjectReuseSessionState {
    pub compose: bool,
    pub persisted_targets: Vec<ReuseProposalTarget>,
    pub frozen: Option<FrozenProjectReuseAnalysis>,
    pub project_memory_available: bool,
    pub current_project_snapshot: Option<ProjectMemorySnapshotIdentity>,
    pub project_memory_records: Vec<ProjectMemoryRecord>,
}

pub fn begin_application_review(
    transcript: Transcript,
    session_terms: Vec<SessionTermEntry>,
    material_use: ApplicationMaterialUseDeclaration,
    session_authority: DeclaredSessionAuthority,
) -> Result<ApplicationReviewSession, ApplicationServiceError> {
    let source_revision = transcript.revision_id();
    let canonical_run = run_canonical_term_review(&transcript, &session_terms)
        .map_err(ApplicationServiceError::Detection)?;
    let analysis_snapshot = canonical_run.analysis_run().snapshot();

    Ok(ApplicationReviewSession {
        transcript,
        session_terms,
        canonical_run,
        ledger: ReviewLedger::new(),
        material_use: BoundApplicationMaterialUseDeclaration {
            declaration: material_use,
            source_revision,
        },
        session_authority: BoundDeclaredSessionAuthority {
            authority: session_authority,
            source_revision,
            analysis_snapshot,
        },
        reuse_state: ApplicationReuseState::default(),
        reuse_enabled_run: None,
        project_reuse: ProjectReuseSessionState::default(),
    })
}

pub(crate) fn assemble_application_review_session(
    transcript: Transcript,
    session_terms: Vec<SessionTermEntry>,
    canonical_run: CanonicalTermReviewRun,
    ledger: ReviewLedger,
    material_use: ApplicationMaterialUseDeclaration,
    session_authority: DeclaredSessionAuthority,
    reuse_state: ApplicationReuseState,
    reuse_enabled_run: Option<ReuseEnabledTermReviewRun>,
    project_reuse: ProjectReuseSessionState,
) -> Result<ApplicationReviewSession, ApplicationServiceError> {
    let source_revision = transcript.revision_id();
    let analysis_snapshot = canonical_run.analysis_run().snapshot();

    Ok(ApplicationReviewSession {
        transcript,
        session_terms,
        canonical_run,
        ledger,
        material_use: BoundApplicationMaterialUseDeclaration {
            declaration: material_use,
            source_revision,
        },
        session_authority: BoundDeclaredSessionAuthority {
            authority: session_authority,
            source_revision,
            analysis_snapshot,
        },
        reuse_state,
        reuse_enabled_run,
        project_reuse,
    })
}

impl ApplicationReviewSession {
    pub fn source(&self) -> &Transcript {
        &self.transcript
    }

    pub fn review_ledger(&self) -> &ReviewLedger {
        &self.ledger
    }

    pub(crate) fn canonical_run(&self) -> &CanonicalTermReviewRun {
        &self.canonical_run
    }

    pub(crate) fn session_terms(&self) -> &[SessionTermEntry] {
        &self.session_terms
    }

    pub(crate) fn review_ledger_head(&self) -> usize {
        self.ledger.events().len()
    }

    pub(crate) fn material_use_declaration(&self) -> ApplicationMaterialUseDeclaration {
        self.material_use.declaration
    }

    pub fn session_authority(&self) -> &DeclaredSessionAuthority {
        &self.session_authority.authority
    }

    pub fn reuse_state(&self) -> &ApplicationReuseState {
        &self.reuse_state
    }

    pub fn reuse_enabled_run(&self) -> Option<&ReuseEnabledTermReviewRun> {
        self.reuse_enabled_run.as_ref()
    }

    pub fn persisted_reuse_proposal_targets(&self) -> &[ReuseProposalTarget] {
        &self.project_reuse.persisted_targets
    }

    pub fn frozen_project_reuse_analysis(&self) -> Option<&FrozenProjectReuseAnalysis> {
        self.project_reuse.frozen.as_ref()
    }

    pub fn compose_project_reuse_proposals(&self) -> bool {
        self.project_reuse.compose
    }

    pub fn derived_reuse_proposal_targets(&self) -> Vec<ReuseProposalTarget> {
        derived_targets_from_session(self)
    }

    pub fn has_project_scope(&self) -> bool {
        self.reuse_state.project_scope().is_some()
    }

    pub fn project_memory_available(&self) -> bool {
        self.project_reuse.project_memory_available
    }

    pub fn project_memory_records(&self) -> &[crate::project_memory::ProjectMemoryRecord] {
        &self.project_reuse.project_memory_records
    }

    pub fn project_display_name(&self) -> Option<&str> {
        self.reuse_state
            .project_scope()
            .map(|scope| scope.display_name.as_str())
    }

    pub fn reuse_parts(&self) -> ReuseSessionParts<'_> {
        ReuseSessionParts {
            transcript: &self.transcript,
            session_terms: &self.session_terms,
            canonical_run: &self.canonical_run,
            ledger: &self.ledger,
        }
    }

    pub fn initialize_project_scope(
        &mut self,
        stable_id: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Result<(), ApplicationReuseError> {
        initialize_project_scope(&mut self.reuse_state, stable_id, display_name)?;
        self.reuse_enabled_run = None;
        Ok(())
    }

    pub fn update_project_scope_display_name(
        &mut self,
        display_name: impl Into<String>,
    ) -> Result<(), ApplicationReuseError> {
        update_project_scope_display_name(&mut self.reuse_state, display_name)?;
        Ok(())
    }

    pub fn reuse_candidates(
        &self,
    ) -> Result<Vec<crate::reusable_influence::ReuseCandidate>, ApplicationReuseError> {
        reuse_candidates_for_parts(self.reuse_parts(), &self.reuse_state)
    }

    pub fn accept_reuse_candidate(
        &mut self,
        candidate_key: &crate::reusable_influence::ReuseCandidateKey,
    ) -> Result<crate::reuse_primitives::ReusableInfluenceRecordId, ApplicationReuseError> {
        let authority = self.session_authority().clone();
        let parts = ReuseSessionParts {
            transcript: &self.transcript,
            session_terms: &self.session_terms,
            canonical_run: &self.canonical_run,
            ledger: &self.ledger,
        };
        let record_id =
            accept_reuse_candidate(parts, &mut self.reuse_state, &authority, candidate_key)?;
        self.reuse_enabled_run = None;
        Ok(record_id)
    }

    pub fn reject_reuse_candidate(
        &mut self,
        candidate_key: &crate::reusable_influence::ReuseCandidateKey,
    ) -> Result<(), ApplicationReuseError> {
        let authority = self.session_authority().clone();
        let parts = ReuseSessionParts {
            transcript: &self.transcript,
            session_terms: &self.session_terms,
            canonical_run: &self.canonical_run,
            ledger: &self.ledger,
        };
        reject_reuse_candidate(parts, &mut self.reuse_state, &authority, candidate_key)?;
        self.reuse_enabled_run = None;
        Ok(())
    }

    pub fn revoke_reusable_influence(
        &mut self,
        record_id: crate::reuse_primitives::ReusableInfluenceRecordId,
    ) -> Result<(), ApplicationReuseError> {
        let authority = self.session_authority().clone();
        revoke_reusable_influence(
            &mut self.reuse_state,
            &self.ledger,
            &self.canonical_run,
            &authority,
            record_id,
        )?;
        self.reuse_enabled_run = None;
        Ok(())
    }

    pub fn supersede_reusable_influence(
        &mut self,
        predecessor_id: crate::reuse_primitives::ReusableInfluenceRecordId,
        successor_candidate_key: &crate::reusable_influence::ReuseCandidateKey,
    ) -> Result<crate::reuse_primitives::ReusableInfluenceRecordId, ApplicationReuseError> {
        let authority = self.session_authority().clone();
        let parts = ReuseSessionParts {
            transcript: &self.transcript,
            session_terms: &self.session_terms,
            canonical_run: &self.canonical_run,
            ledger: &self.ledger,
        };
        let record_id = supersede_reusable_influence(
            parts,
            &mut self.reuse_state,
            &authority,
            predecessor_id,
            successor_candidate_key,
        )?;
        self.reuse_enabled_run = None;
        Ok(record_id)
    }

    pub fn run_reuse_enabled_review(
        &mut self,
    ) -> Result<&ReuseEnabledTermReviewRun, ApplicationReuseError> {
        let run = run_reuse_enabled_review_for_parts(self.reuse_parts(), &self.reuse_state)?;
        self.reuse_enabled_run = Some(run);
        Ok(self.reuse_enabled_run.as_ref().expect("just stored"))
    }

    pub fn prepare_initialize_project_scope(
        &self,
        stable_id: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Result<crate::application_reuse::PreparedProjectScopeInitialization, ApplicationReuseError>
    {
        crate::application_reuse::prepare_initialize_project_scope(
            &self.reuse_state,
            stable_id,
            display_name,
        )
    }

    pub fn prepare_update_project_scope_display_name(
        &self,
        display_name: impl Into<String>,
    ) -> Result<
        crate::application_reuse::PreparedProjectScopeDisplayNameUpdate,
        ApplicationReuseError,
    > {
        crate::application_reuse::prepare_update_project_scope_display_name(
            &self.reuse_state,
            display_name,
        )
    }

    pub fn prepare_accept_reuse_candidate(
        &self,
        candidate_key: &crate::reusable_influence::ReuseCandidateKey,
    ) -> Result<crate::application_reuse::PreparedReuseCandidateAcceptance, ApplicationReuseError>
    {
        crate::application_reuse::prepare_accept_reuse_candidate(
            self.reuse_parts(),
            &self.reuse_state,
            candidate_key,
        )
    }

    pub fn prepare_reject_reuse_candidate(
        &self,
        candidate_key: &crate::reusable_influence::ReuseCandidateKey,
    ) -> Result<crate::application_reuse::PreparedReuseCandidateRejection, ApplicationReuseError>
    {
        crate::application_reuse::prepare_reject_reuse_candidate(
            self.reuse_parts(),
            &self.reuse_state,
            candidate_key,
        )
    }

    pub fn prepare_revoke_reusable_influence(
        &self,
        record_id: crate::reuse_primitives::ReusableInfluenceRecordId,
    ) -> Result<crate::application_reuse::PreparedReusableInfluenceRevocation, ApplicationReuseError>
    {
        crate::application_reuse::prepare_revoke_reusable_influence(
            &self.reuse_state,
            &self.ledger,
            &self.canonical_run,
            record_id,
        )
    }

    pub fn prepare_supersede_reusable_influence(
        &self,
        predecessor_id: crate::reuse_primitives::ReusableInfluenceRecordId,
        successor_candidate_key: &crate::reusable_influence::ReuseCandidateKey,
    ) -> Result<
        crate::application_reuse::PreparedReusableInfluenceSupersession,
        ApplicationReuseError,
    > {
        crate::application_reuse::prepare_supersede_reusable_influence(
            self.reuse_parts(),
            &self.reuse_state,
            predecessor_id,
            successor_candidate_key,
        )
    }

    pub fn prepare_run_reuse_enabled_review(
        &self,
    ) -> Result<crate::application_reuse::PreparedActiveAnalysis, ApplicationReuseError> {
        crate::application_reuse::prepare_reuse_enabled_review(
            self.reuse_parts(),
            &self.reuse_state,
        )
    }

    pub fn active_reusable_records(
        &self,
    ) -> Result<
        Vec<crate::reusable_influence::EffectiveReusableInfluenceRecord>,
        ApplicationReuseError,
    > {
        active_reusable_records(self.reuse_parts(), &self.reuse_state)
    }

    pub fn materialize_review_export_bundle_v3(
        &self,
    ) -> Result<ApplicationReviewExportBundleV3, ApplicationGate3Error> {
        let base = self
            .materialize_review_export_bundle()
            .map_err(ApplicationGate3Error::Service)?;
        let derived_candidates = self
            .reuse_candidates()
            .map_err(ApplicationGate3Error::Reuse)?;
        build_export_bundle_v3(
            base,
            &self.reuse_state,
            &self.ledger,
            &self.canonical_run,
            derived_candidates,
            self.reuse_enabled_run.as_ref(),
        )
        .map_err(ApplicationGate3Error::Reuse)
    }

    pub fn review_items(&self) -> Vec<ApplicationReviewItem> {
        compose_review_items(self)
    }

    pub fn prepare_human_decision(
        &self,
        target: ApplicationReviewTarget,
        decision: CorrectionDecision,
    ) -> Result<PreparedHumanDecision, ApplicationServiceError> {
        match target {
            ApplicationReviewTarget::CanonicalTermCase {
                analysis_snapshot,
                case_id,
            } => {
                if analysis_snapshot != self.canonical_run.analysis_run().snapshot() {
                    return Err(ApplicationServiceError::TargetAnalysisMismatch);
                }
                let review_case = resolve_case(&self.canonical_run, case_id)
                    .ok_or(ApplicationServiceError::UnknownReviewCase { case_id })?;
                let decision =
                    revalidate_decision_for_case(&self.transcript, review_case, decision)?;
                Ok(PreparedHumanDecision {
                    target,
                    decision,
                    expected_review_ledger_head: self.review_ledger_head(),
                    reuse_proposal_target: None,
                })
            }
            ApplicationReviewTarget::ProjectReuseProposal {
                reuse_analysis_snapshot,
                target_identity,
            } => {
                let prepared_target = self.prepare_reuse_target(target_identity)?;
                if reuse_analysis_snapshot != prepared_target.reuse_analysis_snapshot() {
                    return Err(ApplicationServiceError::TargetAnalysisMismatch);
                }
                let decision =
                    revalidate_decision_for_reuse(&self.transcript, &prepared_target, decision)?;
                Ok(PreparedHumanDecision {
                    target,
                    decision,
                    expected_review_ledger_head: self.review_ledger_head(),
                    reuse_proposal_target: Some(prepared_target),
                })
            }
        }
    }

    fn prepare_reuse_target(
        &self,
        target_identity: ReuseProposalTargetIdentity,
    ) -> Result<ReuseProposalTarget, ApplicationServiceError> {
        if !self.project_reuse.project_memory_available {
            return Err(ApplicationServiceError::ProjectEvidenceUnverifiable);
        }
        if !self.project_reuse.compose {
            return Err(ApplicationServiceError::UnknownReuseProposal);
        }
        let frozen = self
            .project_reuse
            .frozen
            .as_ref()
            .ok_or(ApplicationServiceError::UnknownReuseProposal)?;
        match self.project_reuse.current_project_snapshot {
            Some(current) if current == frozen.project_memory_snapshot_identity => {}
            _ => return Err(ApplicationServiceError::StaleReuseAnalysis),
        }
        let derived = derived_targets_from_session(self);
        let target = derived
            .into_iter()
            .find(|item| item.identity() == target_identity)
            .ok_or(ApplicationServiceError::UnknownReuseProposal)?;
        if collapsed_same_replacement_canonical(self, &target).is_some() {
            return Err(ApplicationServiceError::UnknownReuseProposal);
        }
        Ok(target)
    }

    pub fn prepare_manual_replacement(
        &self,
        target: ApplicationReviewTarget,
        replacement: impl Into<String>,
    ) -> Result<PreparedManualReplacement, ApplicationServiceError> {
        let selected_source_text = match target {
            ApplicationReviewTarget::CanonicalTermCase {
                analysis_snapshot,
                case_id,
            } => {
                if analysis_snapshot != self.canonical_run.analysis_run().snapshot() {
                    return Err(ApplicationServiceError::TargetAnalysisMismatch);
                }
                let review_case = resolve_case(&self.canonical_run, case_id)
                    .ok_or(ApplicationServiceError::UnknownReviewCase { case_id })?;
                self.transcript
                    .resolve(review_case.candidate_span().anchor())
                    .ok_or(ApplicationServiceError::ReviewedOutput(
                        ReviewedOutputError::AnchorResolutionFailed {
                            case_id: review_case.id(),
                        },
                    ))?
                    .to_owned()
            }
            ApplicationReviewTarget::ProjectReuseProposal {
                target_identity, ..
            } => {
                let prepared_target = self.prepare_reuse_target(target_identity)?;
                prepared_target.occurrence().observed_text.clone()
            }
        };
        let replacement = ManualReplacementText::new(replacement, &selected_source_text)
            .map_err(ApplicationServiceError::ManualReplacement)?;
        let prepared = self.prepare_human_decision(
            target,
            CorrectionDecision::ManualReplacement { replacement },
        )?;
        Ok(PreparedManualReplacement { prepared })
    }

    pub fn record_human_decision(
        &mut self,
        target: ApplicationReviewTarget,
        decision: CorrectionDecision,
    ) -> Result<(), ApplicationServiceError> {
        let prepared = self.prepare_human_decision(target, decision)?;
        match prepared.reuse_proposal_target {
            None => {
                let case_id = prepared
                    .target
                    .case_id()
                    .ok_or(ApplicationServiceError::UnknownReuseProposal)?;
                let review_case = resolve_case(&self.canonical_run, case_id)
                    .ok_or(ApplicationServiceError::UnknownReviewCase { case_id })?;
                self.ledger
                    .record_decision(
                        review_case,
                        self.transcript.revision_id(),
                        prepared.decision,
                    )
                    .map_err(ApplicationServiceError::Decision)
            }
            Some(reuse_target) => {
                if !self
                    .project_reuse
                    .persisted_targets
                    .iter()
                    .any(|item| item.identity() == reuse_target.identity())
                {
                    self.project_reuse
                        .persisted_targets
                        .push(reuse_target.clone());
                }
                self.ledger
                    .record_reuse_decision(
                        reuse_target.identity(),
                        self.transcript.revision_id(),
                        prepared.decision,
                    )
                    .map_err(ApplicationServiceError::Decision)
            }
        }
    }

    pub fn record_manual_replacement(
        &mut self,
        target: ApplicationReviewTarget,
        replacement: impl Into<String>,
    ) -> Result<(), ApplicationServiceError> {
        let prepared = self.prepare_manual_replacement(target, replacement)?;
        self.record_human_decision(prepared.prepared.target, prepared.prepared.decision)
    }

    pub fn progress(&self) -> ApplicationReviewProgress {
        derive_progress(
            &self.canonical_run,
            &self.ledger,
            &self.project_reuse.persisted_targets,
        )
    }

    pub fn decision_summary(&self) -> ApplicationDecisionSummary {
        derive_decision_summary(
            &self.canonical_run,
            &self.ledger,
            &self.project_reuse.persisted_targets,
        )
    }

    pub fn derive_current_projection(
        &self,
    ) -> Result<ApplicationCurrentProjection, ApplicationServiceError> {
        build_current_projection(
            &self.transcript,
            &self.canonical_run,
            &self.ledger,
            &self.project_reuse.persisted_targets,
        )
    }

    pub fn materialize_reviewed_output(
        &self,
    ) -> Result<ApplicationReviewedOutput, ApplicationServiceError> {
        build_reviewed_output(
            &self.transcript,
            &self.canonical_run,
            &self.ledger,
            &self.project_reuse.persisted_targets,
        )
    }

    pub fn materialize_review_export_bundle(
        &self,
    ) -> Result<ApplicationReviewExportBundle, ApplicationServiceError> {
        build_export_bundle(
            &self.transcript,
            &self.session_terms,
            &self.canonical_run,
            &self.ledger,
            self.material_use,
            self.session_authority.clone(),
            &self.project_reuse.persisted_targets,
        )
    }

    pub fn verify_in_memory_replay(&self) -> Result<(), ApplicationReplayError> {
        self.verify_canonical_replay()?;
        self.verify_gate3_replay_state()?;
        Ok(())
    }

    pub(crate) fn verify_canonical_replay(&self) -> Result<(), ApplicationReplayError> {
        if self.material_use.source_revision != self.transcript.revision_id() {
            return Err(ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::AnalysisSnapshot,
            });
        }
        if self.session_authority.source_revision != self.transcript.revision_id() {
            return Err(ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::AnalysisSnapshot,
            });
        }
        if self.session_authority.analysis_snapshot != self.canonical_run.analysis_run().snapshot()
        {
            return Err(ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::AnalysisSnapshot,
            });
        }
        let _declared_basis = self.material_use.declaration.basis();

        let replay_run = run_canonical_term_review(&self.transcript, &self.session_terms)
            .map_err(ApplicationServiceError::Detection)
            .map_err(ApplicationReplayError::Service)?;

        if replay_run.analysis_run().snapshot() != self.canonical_run.analysis_run().snapshot() {
            return Err(ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::AnalysisSnapshot,
            });
        }
        if replay_run.review_cases() != self.canonical_run.review_cases() {
            return Err(ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::ReviewCases,
            });
        }

        let mut replay_ledger = ReviewLedger::new();
        for event in self.ledger.events() {
            match event {
                ReviewLedgerEvent::DecisionRecorded {
                    case_id,
                    observed_revision,
                    decision,
                } => {
                    if *observed_revision != self.transcript.revision_id() {
                        return Err(ApplicationReplayError::Mismatch {
                            field: ApplicationReplayField::LedgerEvents,
                        });
                    }
                    let review_case = resolve_case(&replay_run, *case_id).ok_or(
                        ApplicationReplayError::Mismatch {
                            field: ApplicationReplayField::ReviewCases,
                        },
                    )?;
                    let replay_decision = revalidate_decision_for_case(
                        &self.transcript,
                        review_case,
                        decision.clone(),
                    )
                    .map_err(ApplicationReplayError::Service)?;
                    replay_ledger
                        .record_decision(review_case, *observed_revision, replay_decision)
                        .map_err(ApplicationServiceError::Decision)
                        .map_err(ApplicationReplayError::Service)?;
                }
                ReviewLedgerEvent::ReuseProposalDecisionRecorded {
                    target_identity,
                    observed_revision,
                    decision,
                } => {
                    if *observed_revision != self.transcript.revision_id() {
                        return Err(ApplicationReplayError::Mismatch {
                            field: ApplicationReplayField::LedgerEvents,
                        });
                    }
                    let target = self
                        .project_reuse
                        .persisted_targets
                        .iter()
                        .find(|item| item.identity() == *target_identity)
                        .ok_or(ApplicationReplayError::Mismatch {
                            field: ApplicationReplayField::LedgerEvents,
                        })?;
                    let replay_decision =
                        revalidate_decision_for_reuse(&self.transcript, target, decision.clone())
                            .map_err(ApplicationReplayError::Service)?;
                    replay_ledger
                        .record_reuse_decision(
                            *target_identity,
                            *observed_revision,
                            replay_decision,
                        )
                        .map_err(ApplicationServiceError::Decision)
                        .map_err(ApplicationReplayError::Service)?;
                }
            }
        }

        if replay_ledger.events() != self.ledger.events() {
            return Err(ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::LedgerEvents,
            });
        }

        if effective_statuses(&replay_run, &replay_ledger)
            != effective_statuses(&self.canonical_run, &self.ledger)
        {
            return Err(ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::EffectiveStatuses,
            });
        }

        if derive_progress(
            &replay_run,
            &replay_ledger,
            &self.project_reuse.persisted_targets,
        ) != self.progress()
        {
            return Err(ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::Progress,
            });
        }
        if derive_decision_summary(
            &replay_run,
            &replay_ledger,
            &self.project_reuse.persisted_targets,
        ) != self.decision_summary()
        {
            return Err(ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::DecisionSummary,
            });
        }
        let replay_session_summary = derive_session_summary_projection(
            &self.transcript,
            self.session_terms.len(),
            &replay_run,
            &replay_ledger,
            &self.project_reuse.persisted_targets,
        );
        let current_session_summary = derive_session_summary_projection(
            &self.transcript,
            self.session_terms.len(),
            &self.canonical_run,
            &self.ledger,
            &self.project_reuse.persisted_targets,
        );
        if replay_session_summary != current_session_summary {
            return Err(ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::SessionSummary,
            });
        }

        let replay_projection = build_current_projection(
            &self.transcript,
            &replay_run,
            &replay_ledger,
            &self.project_reuse.persisted_targets,
        );
        let current_projection = self.derive_current_projection();
        if replay_projection != current_projection {
            return Err(ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::CurrentProjection,
            });
        }

        let replay_output = build_reviewed_output(
            &self.transcript,
            &replay_run,
            &replay_ledger,
            &self.project_reuse.persisted_targets,
        );
        let current_output = self.materialize_reviewed_output();
        if replay_output != current_output {
            return Err(ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::ReviewedOutput,
            });
        }

        let replay_bundle = build_export_bundle(
            &self.transcript,
            &self.session_terms,
            &replay_run,
            &replay_ledger,
            self.material_use,
            self.session_authority.clone(),
            &self.project_reuse.persisted_targets,
        );
        let current_bundle = self.materialize_review_export_bundle();
        if replay_bundle != current_bundle {
            return Err(ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::ExportBundle,
            });
        }

        Ok(())
    }

    fn verify_gate3_replay_state(&self) -> Result<(), ApplicationReplayError> {
        crate::application_gate3_replay::verify_gate3_independent_replay(self)
    }
}

fn resolve_case(
    canonical_run: &CanonicalTermReviewRun,
    case_id: ReviewCaseId,
) -> Option<&ReviewCase> {
    canonical_run
        .review_cases()
        .get(case_id.local_index())
        .filter(|review_case| review_case.id() == case_id)
}

fn revalidate_decision_for_case(
    transcript: &Transcript,
    review_case: &ReviewCase,
    decision: CorrectionDecision,
) -> Result<CorrectionDecision, ApplicationServiceError> {
    let CorrectionDecision::ManualReplacement { replacement } = decision else {
        return Ok(decision);
    };
    let selected_source_text = transcript
        .resolve(review_case.candidate_span().anchor())
        .ok_or(ApplicationServiceError::ReviewedOutput(
            ReviewedOutputError::AnchorResolutionFailed {
                case_id: review_case.id(),
            },
        ))?;
    let replacement = ManualReplacementText::new(replacement.as_str(), selected_source_text)
        .map_err(ApplicationServiceError::ManualReplacement)?;
    Ok(CorrectionDecision::ManualReplacement { replacement })
}

fn revalidate_decision_for_reuse(
    _transcript: &Transcript,
    target: &ReuseProposalTarget,
    decision: CorrectionDecision,
) -> Result<CorrectionDecision, ApplicationServiceError> {
    let CorrectionDecision::ManualReplacement { replacement } = decision else {
        return Ok(decision);
    };
    let replacement =
        ManualReplacementText::new(replacement.as_str(), &target.occurrence().observed_text)
            .map_err(ApplicationServiceError::ManualReplacement)?;
    Ok(CorrectionDecision::ManualReplacement { replacement })
}

fn derived_targets_from_session(session: &ApplicationReviewSession) -> Vec<ReuseProposalTarget> {
    let (Some(run), Some(frozen), Some(scope)) = (
        session.reuse_enabled_run.as_ref(),
        session.project_reuse.frozen.as_ref(),
        session.reuse_state.project_scope(),
    ) else {
        return Vec::new();
    };
    derive_reuse_proposal_targets(
        &session.transcript,
        run,
        &scope.stable_id,
        frozen.project_memory_snapshot_identity,
        frozen.governance_event_boundary,
    )
}

fn canonical_occurrence_key(review_case: &ReviewCase) -> (usize, usize, usize) {
    let anchor = review_case.candidate_span().anchor();
    (
        anchor.segment_position(),
        anchor.start_byte(),
        anchor.end_byte(),
    )
}

fn canonical_proposed_replacement(review_case: &ReviewCase) -> Option<&str> {
    review_case
        .candidate_span()
        .alternatives()
        .first()
        .map(|alternative| alternative.replacement_text())
}

fn collapsed_same_replacement_canonical(
    session: &ApplicationReviewSession,
    target: &ReuseProposalTarget,
) -> Option<ReviewCaseId> {
    session
        .canonical_run
        .review_cases()
        .iter()
        .find_map(|review_case| {
            if canonical_occurrence_key(review_case) == target.occurrence_key()
                && canonical_proposed_replacement(review_case)
                    == Some(target.proposed_replacement())
            {
                Some(review_case.id())
            } else {
                None
            }
        })
}

fn compose_review_items(session: &ApplicationReviewSession) -> Vec<ApplicationReviewItem> {
    let analysis_snapshot = session.canonical_run.analysis_run().snapshot();
    let mut items: Vec<ApplicationReviewItem> = session
        .canonical_run
        .review_cases()
        .iter()
        .map(|review_case| ApplicationReviewItem {
            target: ApplicationReviewTarget::CanonicalTermCase {
                analysis_snapshot,
                case_id: review_case.id(),
            },
            review_case: review_case.clone(),
            status: session.ledger.status_for(review_case.id()),
            kind: ApplicationReviewItemKind::CanonicalTermCase {
                gate7_repeated_correction_avoided_eligible: false,
            },
        })
        .collect();

    let reuse_targets = if session.project_reuse.compose {
        derived_targets_from_session(session)
    } else {
        session
            .project_reuse
            .persisted_targets
            .iter()
            .filter(|target| {
                !matches!(
                    session.ledger.status_for_reuse(target.identity()),
                    ReviewCaseStatus::Undecided
                )
            })
            .cloned()
            .collect()
    };

    for target in reuse_targets {
        if collapsed_same_replacement_canonical(session, &target).is_some() {
            continue;
        }
        let conflict_with_canonical =
            session
                .canonical_run
                .review_cases()
                .iter()
                .any(|review_case| {
                    canonical_occurrence_key(review_case) == target.occurrence_key()
                        && canonical_proposed_replacement(review_case)
                            != Some(target.proposed_replacement())
                });
        let display_index = 10_000 + items.len();
        items.push(ApplicationReviewItem {
            target: ApplicationReviewTarget::ProjectReuseProposal {
                reuse_analysis_snapshot: target.reuse_analysis_snapshot(),
                target_identity: target.identity(),
            },
            review_case: target.display_review_case(display_index),
            status: session.ledger.status_for_reuse(target.identity()),
            kind: ApplicationReviewItemKind::ProjectReuseProposal {
                gate7_repeated_correction_avoided_eligible: session.project_reuse.compose
                    && !conflict_with_canonical,
                conflict_with_canonical,
            },
        });
    }

    items.sort_by(|left, right| {
        let left_anchor = left.review_case.candidate_span().anchor();
        let right_anchor = right.review_case.candidate_span().anchor();
        (
            left_anchor.segment_position(),
            left_anchor.start_byte(),
            left_anchor.end_byte(),
            review_item_kind_rank(left.kind),
            left.review_case.id().local_index(),
        )
            .cmp(&(
                right_anchor.segment_position(),
                right_anchor.start_byte(),
                right_anchor.end_byte(),
                review_item_kind_rank(right.kind),
                right.review_case.id().local_index(),
            ))
    });
    items
}

fn review_item_kind_rank(kind: ApplicationReviewItemKind) -> u8 {
    match kind {
        ApplicationReviewItemKind::CanonicalTermCase { .. } => 0,
        ApplicationReviewItemKind::ProjectReuseProposal { .. } => 1,
    }
}

fn apply_decided_status_counts(summary: &mut ApplicationDecisionSummary, status: ReviewCaseStatus) {
    match status {
        ReviewCaseStatus::Undecided => summary.undecided += 1,
        ReviewCaseStatus::Decided { decision, .. } => match decision {
            CorrectionDecision::AcceptAlternative { .. } => {
                summary.accepted_alternatives += 1;
            }
            CorrectionDecision::ManualReplacement { .. } => {
                summary.manual_replacements += 1;
            }
            CorrectionDecision::Reject => summary.rejected += 1,
            CorrectionDecision::Defer => summary.deferred += 1,
            CorrectionDecision::NeedsManualCorrection => {
                summary.needs_manual_correction += 1;
            }
        },
    }
}

fn effective_statuses(
    canonical_run: &CanonicalTermReviewRun,
    ledger: &ReviewLedger,
) -> Vec<(ReviewCaseId, ReviewCaseStatus)> {
    canonical_run
        .review_cases()
        .iter()
        .map(|review_case| {
            let case_id = review_case.id();
            (case_id, ledger.status_for(case_id))
        })
        .collect()
}

fn derive_progress(
    canonical_run: &CanonicalTermReviewRun,
    ledger: &ReviewLedger,
    reuse_targets: &[ReuseProposalTarget],
) -> ApplicationReviewProgress {
    let summary = derive_decision_summary(canonical_run, ledger, reuse_targets);
    let decision_coverage = if summary.undecided == 0 {
        ApplicationDecisionCoverage::Complete
    } else {
        ApplicationDecisionCoverage::Incomplete {
            undecided: summary.undecided,
        }
    };
    let resolution_status = if summary.deferred == 0 && summary.needs_manual_correction == 0 {
        ApplicationResolutionStatus::Resolved
    } else {
        ApplicationResolutionStatus::Unresolved {
            deferred: summary.deferred,
            needs_manual_correction: summary.needs_manual_correction,
        }
    };

    ApplicationReviewProgress {
        decision_coverage,
        resolution_status,
    }
}

fn derive_decision_summary(
    canonical_run: &CanonicalTermReviewRun,
    ledger: &ReviewLedger,
    reuse_targets: &[ReuseProposalTarget],
) -> ApplicationDecisionSummary {
    let mut summary = ApplicationDecisionSummary {
        total_review_cases: canonical_run.review_cases().len(),
        total_recorded_events: ledger.events().len(),
        accepted_alternatives: 0,
        manual_replacements: 0,
        rejected: 0,
        deferred: 0,
        needs_manual_correction: 0,
        undecided: 0,
    };

    for review_case in canonical_run.review_cases() {
        apply_decided_status_counts(&mut summary, ledger.status_for(review_case.id()));
    }
    for target in reuse_targets {
        match ledger.status_for_reuse(target.identity()) {
            ReviewCaseStatus::Undecided => {}
            decided => apply_decided_status_counts(&mut summary, decided),
        }
    }

    summary
}

fn derive_session_summary_projection(
    transcript: &Transcript,
    session_term_entry_count: usize,
    canonical_run: &CanonicalTermReviewRun,
    ledger: &ReviewLedger,
    reuse_targets: &[ReuseProposalTarget],
) -> ApplicationSessionSummaryProjection {
    let mut kind_counts = HashMap::<DetectionKind, usize>::new();
    let mut detector_counts = BTreeMap::<(String, String), usize>::new();

    for review_case in canonical_run.review_cases() {
        let candidate = review_case.candidate_span();
        *kind_counts.entry(candidate.kind()).or_default() += 1;

        let provenance = candidate.provenance();
        *detector_counts
            .entry((
                provenance.detector_id().to_string(),
                provenance.detector_version().to_string(),
            ))
            .or_default() += 1;
    }

    let mut cases_by_detection_kind = kind_counts
        .into_iter()
        .map(|(kind, count)| ApplicationDetectionKindCount { kind, count })
        .collect::<Vec<_>>();
    cases_by_detection_kind.sort_by_key(|item| detection_kind_sort_key(item.kind));

    let cases_by_detector = detector_counts
        .into_iter()
        .map(
            |((detector_id, detector_version), count)| ApplicationDetectorCount {
                detector_id,
                detector_version,
                count,
            },
        )
        .collect();

    let mut accepted_replacements_materialized = 0usize;
    let mut affected_segments = HashSet::new();
    let mut accepted_replacements = BTreeMap::<String, usize>::new();

    for review_case in canonical_run.review_cases() {
        match ledger.status_for(review_case.id()) {
            ReviewCaseStatus::Undecided => {}
            ReviewCaseStatus::Decided { decision, .. } => {
                let replacement_text = match decision {
                    CorrectionDecision::AcceptAlternative { alternative_index } => review_case
                        .candidate_span()
                        .alternatives()
                        .get(alternative_index)
                        .map(|alternative| alternative.replacement_text()),
                    CorrectionDecision::ManualReplacement { ref replacement } => {
                        Some(replacement.as_str())
                    }
                    CorrectionDecision::Reject
                    | CorrectionDecision::Defer
                    | CorrectionDecision::NeedsManualCorrection => None,
                };
                if let Some(replacement_text) = replacement_text {
                    accepted_replacements_materialized += 1;
                    affected_segments
                        .insert(review_case.candidate_span().anchor().segment_position());
                    *accepted_replacements
                        .entry(replacement_text.to_string())
                        .or_default() += 1;
                }
            }
        }
    }
    for target in reuse_targets {
        match ledger.status_for_reuse(target.identity()) {
            ReviewCaseStatus::Decided { decision, .. } => {
                let replacement_text = match decision {
                    CorrectionDecision::AcceptAlternative { .. } => {
                        Some(target.proposed_replacement())
                    }
                    CorrectionDecision::ManualReplacement { ref replacement } => {
                        Some(replacement.as_str())
                    }
                    CorrectionDecision::Reject
                    | CorrectionDecision::Defer
                    | CorrectionDecision::NeedsManualCorrection => None,
                };
                if let Some(replacement_text) = replacement_text {
                    accepted_replacements_materialized += 1;
                    affected_segments.insert(target.occurrence().segment_position);
                    *accepted_replacements
                        .entry(replacement_text.to_string())
                        .or_default() += 1;
                }
            }
            ReviewCaseStatus::Undecided => {}
        }
    }

    let accepted_replacements = accepted_replacements
        .into_iter()
        .map(
            |(replacement_text, count)| ApplicationAcceptedReplacementCount {
                replacement_text,
                count,
            },
        )
        .collect();

    ApplicationSessionSummaryProjection {
        source_revision: transcript.revision_id(),
        transcript_segments: transcript.segments().len(),
        session_term_entry_count,
        review_cases_raised: canonical_run.review_cases().len(),
        cases_by_detection_kind,
        cases_by_detector,
        outcomes: ApplicationSessionOutcomeCounts {
            accepted_replacements_materialized,
            source_segments_affected: affected_segments.len(),
        },
        accepted_replacements,
    }
}

fn detection_kind_sort_key(kind: DetectionKind) -> &'static str {
    match kind {
        DetectionKind::GlossaryAliasMatch => "glossary_alias_match",
        DetectionKind::MixedLanguageAnomaly => "mixed_language_anomaly",
        DetectionKind::PhoneticSimilarity => "phonetic_similarity",
        DetectionKind::RepeatedPhrase => "repeated_phrase",
    }
}

fn derive_decision_projection_records(
    ledger: &ReviewLedger,
    session_authority: &DeclaredSessionAuthority,
) -> Vec<ApplicationDecisionProjectionRecord> {
    ledger
        .events()
        .iter()
        .enumerate()
        .map(|(event_index, event)| match event {
            ReviewLedgerEvent::DecisionRecorded {
                case_id,
                observed_revision,
                decision,
            } => ApplicationDecisionProjectionRecord {
                event_index,
                case_id: Some(*case_id),
                reuse_proposal_target_identity: None,
                observed_revision: *observed_revision,
                decision: decision.clone(),
                session_authority: session_authority.clone(),
            },
            ReviewLedgerEvent::ReuseProposalDecisionRecorded {
                target_identity,
                observed_revision,
                decision,
            } => ApplicationDecisionProjectionRecord {
                event_index,
                case_id: None,
                reuse_proposal_target_identity: Some(*target_identity),
                observed_revision: *observed_revision,
                decision: decision.clone(),
                session_authority: session_authority.clone(),
            },
        })
        .collect()
}

fn build_export_bundle(
    transcript: &Transcript,
    session_terms: &[SessionTermEntry],
    canonical_run: &CanonicalTermReviewRun,
    ledger: &ReviewLedger,
    material_use: BoundApplicationMaterialUseDeclaration,
    session_authority: BoundDeclaredSessionAuthority,
    reuse_targets: &[ReuseProposalTarget],
) -> Result<ApplicationReviewExportBundle, ApplicationServiceError> {
    let reviewed_output = build_reviewed_output(transcript, canonical_run, ledger, reuse_targets)?;
    let session_summary = derive_session_summary_projection(
        transcript,
        session_terms.len(),
        canonical_run,
        ledger,
        reuse_targets,
    );

    Ok(ApplicationReviewExportBundle {
        reviewed_srt: reviewed_output.srt,
        progress: reviewed_output.progress,
        decision_summary: reviewed_output.decision_summary,
        decision_records: derive_decision_projection_records(ledger, &session_authority.authority),
        declared_session_authority: session_authority.authority,
        material_use_basis: material_use.declaration.basis(),
        source_revision: transcript.revision_id(),
        analysis_snapshot: canonical_run.analysis_run().snapshot(),
        session_summary,
        export_posture: ApplicationExportPosture::DeclaredOperatorUnauthenticatedInMemoryV0_2,
    })
}

fn build_current_projection(
    transcript: &Transcript,
    canonical_run: &CanonicalTermReviewRun,
    ledger: &ReviewLedger,
    reuse_targets: &[ReuseProposalTarget],
) -> Result<ApplicationCurrentProjection, ApplicationServiceError> {
    let srt = derive_reviewed_srt_with_reuse(
        transcript,
        canonical_run.review_cases(),
        ledger,
        reuse_targets,
    )
    .map_err(ApplicationServiceError::ReviewedOutput)?;

    Ok(ApplicationCurrentProjection {
        srt,
        progress: derive_progress(canonical_run, ledger, reuse_targets),
        decision_summary: derive_decision_summary(canonical_run, ledger, reuse_targets),
    })
}

fn build_reviewed_output(
    transcript: &Transcript,
    canonical_run: &CanonicalTermReviewRun,
    ledger: &ReviewLedger,
    reuse_targets: &[ReuseProposalTarget],
) -> Result<ApplicationReviewedOutput, ApplicationServiceError> {
    let progress = derive_progress(canonical_run, ledger, reuse_targets);
    if let ApplicationDecisionCoverage::Incomplete { undecided } = progress.decision_coverage {
        return Err(ApplicationServiceError::DecisionCoverageIncomplete { undecided });
    }

    let srt = derive_reviewed_srt_with_reuse(
        transcript,
        canonical_run.review_cases(),
        ledger,
        reuse_targets,
    )
    .map_err(ApplicationServiceError::ReviewedOutput)?;

    Ok(ApplicationReviewedOutput {
        srt,
        progress,
        decision_summary: derive_decision_summary(canonical_run, ledger, reuse_targets),
    })
}

fn map_actor_label_error(
    error: crate::reusable_influence::ActorDisplayLabelValidationError,
) -> DeclaredSessionAuthorityError {
    match error {
        crate::reusable_influence::ActorDisplayLabelValidationError::Empty => {
            DeclaredSessionAuthorityError::EmptyDisplayLabel
        }
        crate::reusable_influence::ActorDisplayLabelValidationError::ControlCharacter => {
            DeclaredSessionAuthorityError::ControlCharacterInDisplayLabel
        }
        crate::reusable_influence::ActorDisplayLabelValidationError::UnicodeLineSeparator => {
            DeclaredSessionAuthorityError::UnicodeLineSeparatorInDisplayLabel
        }
        crate::reusable_influence::ActorDisplayLabelValidationError::NonCanonical => {
            DeclaredSessionAuthorityError::NonCanonicalDisplayLabel
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidate::SessionTermEntry;
    use crate::srt::parse_srt;

    fn test_authority() -> DeclaredSessionAuthority {
        DeclaredSessionAuthority::new(
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "test-operator",
        )
        .expect("valid authority")
    }

    fn one_case_session() -> ApplicationReviewSession {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("fixture transcript");
        let session_terms = vec![SessionTermEntry::new(
            "Kafka",
            vec!["Kafak".to_string()],
            Vec::new(),
        )];

        begin_application_review(
            transcript,
            session_terms,
            ApplicationMaterialUseDeclaration::new(DeclaredApplicationMaterialUseBasis::SelfOwned),
            test_authority(),
        )
        .expect("application session")
    }

    #[test]
    fn authority_rejects_empty_display_label() {
        assert_eq!(
            DeclaredSessionAuthority::new(
                DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
                "   ",
            ),
            Err(DeclaredSessionAuthorityError::EmptyDisplayLabel)
        );
    }

    #[test]
    fn authority_rejects_unicode_line_separators_in_display_label() {
        for label in ["Ezra\u{2028}Reviewer", "Ezra\u{2029}Reviewer"] {
            assert_eq!(
                DeclaredSessionAuthority::new(
                    DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
                    label,
                ),
                Err(DeclaredSessionAuthorityError::UnicodeLineSeparatorInDisplayLabel)
            );
        }
    }

    #[test]
    fn authority_canonicalizes_surrounding_whitespace_on_declaration() {
        let authority = DeclaredSessionAuthority::new(
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            " Ezra ",
        )
        .expect("valid authority");
        assert_eq!(authority.display_label(), "Ezra");
        assert_eq!(
            crate::application_reuse::governance_actor_from_authority(&authority).display_label,
            "Ezra"
        );
    }

    #[test]
    fn material_declaration_is_bound_to_derived_source_revision() {
        let session = one_case_session();

        assert_eq!(
            session.material_use.source_revision,
            session.source().revision_id()
        );
        assert_eq!(
            session.material_use.declaration.basis(),
            DeclaredApplicationMaterialUseBasis::SelfOwned
        );
    }

    #[test]
    fn session_authority_is_bound_to_source_revision_and_analysis_snapshot() {
        let session = one_case_session();

        assert_eq!(
            session.session_authority.source_revision,
            session.source().revision_id()
        );
        assert_eq!(
            session.session_authority.analysis_snapshot,
            session.canonical_run.analysis_run().snapshot()
        );
    }

    #[test]
    fn unknown_case_under_matching_analysis_is_rejected_before_mutation() {
        let mut session = one_case_session();
        let snapshot = session.canonical_run.analysis_run().snapshot();
        let target = ApplicationReviewTarget::CanonicalTermCase {
            analysis_snapshot: snapshot,
            case_id: ReviewCaseId::local(usize::MAX),
        };

        assert_eq!(
            session.record_human_decision(target, CorrectionDecision::Reject),
            Err(ApplicationServiceError::UnknownReviewCase {
                case_id: ReviewCaseId::local(usize::MAX),
            })
        );
        assert!(session.ledger.events().is_empty());
    }
}

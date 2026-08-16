use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;

use crate::analysis::AnalysisSnapshot;
use crate::anchor::{AnchorError, TranscriptRevisionId};
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
use crate::project_terminology::{
    ProjectTerminologyProposalTarget, ProjectTerminologyProposalTargetIdentity,
    derive_project_terminology_proposal_targets,
};
use crate::reuse_proposal_target::{
    FrozenProjectReuseAnalysis, ReuseProposalTarget, ReuseProposalTargetIdentity,
    derive_reuse_proposal_targets,
};
use crate::review::{
    CorrectionDecision, HumanSelectedSpan, ManualReplacementText, ManualReplacementTextError,
    ReviewCase, ReviewCaseId, ReviewCaseStatus, ReviewLedger, ReviewLedgerError, ReviewLedgerEvent,
};
use crate::reviewed_output::{ReviewedOutputError, derive_reviewed_srt_with_proposals};
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
    HumanRaisedCase {
        case_id: ReviewCaseId,
    },
    ProjectReuseProposal {
        reuse_analysis_snapshot: AnalysisSnapshot,
        target_identity: ReuseProposalTargetIdentity,
    },
    ProjectTerminologyProposal {
        analysis_snapshot: AnalysisSnapshot,
        target_identity: ProjectTerminologyProposalTargetIdentity,
    },
}

impl ApplicationReviewTarget {
    pub fn case_id(self) -> Option<ReviewCaseId> {
        match self {
            Self::CanonicalTermCase { case_id, .. } | Self::HumanRaisedCase { case_id } => {
                Some(case_id)
            }
            Self::ProjectReuseProposal { .. } | Self::ProjectTerminologyProposal { .. } => None,
        }
    }

    /// A HumanRaised target binds no analysis snapshot: it is not detector output and must not
    /// carry a fabricated analysis identity.
    pub fn analysis_snapshot(self) -> Option<AnalysisSnapshot> {
        match self {
            Self::CanonicalTermCase {
                analysis_snapshot, ..
            } => Some(analysis_snapshot),
            Self::ProjectReuseProposal {
                reuse_analysis_snapshot,
                ..
            } => Some(reuse_analysis_snapshot),
            Self::ProjectTerminologyProposal {
                analysis_snapshot, ..
            } => Some(analysis_snapshot),
            Self::HumanRaisedCase { .. } => None,
        }
    }

    pub fn reuse_proposal_identity(self) -> Option<ReuseProposalTargetIdentity> {
        match self {
            Self::ProjectReuseProposal {
                target_identity, ..
            } => Some(target_identity),
            Self::CanonicalTermCase { .. }
            | Self::HumanRaisedCase { .. }
            | Self::ProjectTerminologyProposal { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationReviewItemKind {
    CanonicalTermCase {
        gate7_repeated_correction_avoided_eligible: bool,
    },
    HumanRaisedCase {},
    ProjectReuseProposal {
        gate7_repeated_correction_avoided_eligible: bool,
        conflict_with_canonical: bool,
    },
    ProjectTerminologyProposal {
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
    pub terminology_proposal_target_identity: Option<ProjectTerminologyProposalTargetIdentity>,
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
    UnknownTerminologyProposal,
    StaleReuseAnalysis,
    ProjectEvidenceUnverifiable,
    Decision(ReviewLedgerError),
    ManualReplacement(ManualReplacementTextError),
    DecisionCoverageIncomplete { undecided: usize },
    ReviewedOutput(ReviewedOutputError),
    HumanRaisedRequiresFormatV3,
    HumanRaisedAnchorInvalid(AnchorError),
    HumanRaisedRevisionStale,
    HumanRaisedOverlap,
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
    pub terminology_proposal_target: Option<ProjectTerminologyProposalTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedManualReplacement {
    pub prepared: PreparedHumanDecision,
}

pub struct ApplicationReviewSession {
    transcript: Transcript,
    session_terms: Vec<SessionTermEntry>,
    canonical_run: CanonicalTermReviewRun,
    human_raised_cases: Vec<ReviewCase>,
    ledger: ReviewLedger,
    material_use: BoundApplicationMaterialUseDeclaration,
    session_authority: BoundDeclaredSessionAuthority,
    reuse_state: ApplicationReuseState,
    reuse_enabled_run: Option<ReuseEnabledTermReviewRun>,
    project_reuse: ProjectReuseSessionState,
    project_terminology: ProjectTerminologySessionState,
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ProjectTerminologySessionState {
    pub compose: bool,
    pub persisted_targets: Vec<ProjectTerminologyProposalTarget>,
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
        human_raised_cases: Vec::new(),
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
        project_terminology: ProjectTerminologySessionState::default(),
    })
}

pub(crate) fn assemble_application_review_session(
    transcript: Transcript,
    session_terms: Vec<SessionTermEntry>,
    canonical_run: CanonicalTermReviewRun,
    human_raised_cases: Vec<ReviewCase>,
    ledger: ReviewLedger,
    material_use: ApplicationMaterialUseDeclaration,
    session_authority: DeclaredSessionAuthority,
    reuse_state: ApplicationReuseState,
    reuse_enabled_run: Option<ReuseEnabledTermReviewRun>,
    project_reuse: ProjectReuseSessionState,
    project_terminology: ProjectTerminologySessionState,
) -> Result<ApplicationReviewSession, ApplicationServiceError> {
    let source_revision = transcript.revision_id();
    let analysis_snapshot = canonical_run.analysis_run().snapshot();

    Ok(ApplicationReviewSession {
        transcript,
        session_terms,
        canonical_run,
        human_raised_cases,
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
        project_terminology,
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

    pub fn human_raised_cases(&self) -> &[ReviewCase] {
        &self.human_raised_cases
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

    pub fn compose_project_terminology_proposals(&self) -> bool {
        self.project_terminology.compose
    }

    pub fn derived_reuse_proposal_targets(&self) -> Vec<ReuseProposalTarget> {
        derived_targets_from_session(self)
    }

    pub fn persisted_project_terminology_proposal_targets(
        &self,
    ) -> &[ProjectTerminologyProposalTarget] {
        &self.project_terminology.persisted_targets
    }

    pub fn derived_project_terminology_proposal_targets(
        &self,
    ) -> Vec<ProjectTerminologyProposalTarget> {
        derived_terminology_targets_from_session(self)
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
            human_raised_cases: &self.human_raised_cases,
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
            human_raised_cases: &self.human_raised_cases,
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
            human_raised_cases: &self.human_raised_cases,
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
            human_raised_cases: &self.human_raised_cases,
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

    /// Raise one human-selected span and immediately decide it as a Manual Replacement.
    ///
    /// Creation and decision stay two ledger events recorded in one gesture. The span is refused
    /// before any mutation when it is not a Unicode-safe single-cue range, or when it overlaps an
    /// existing human-raised anchor, an effective materializing edit, or an undecided card.
    pub fn raise_and_manual_replace(
        &mut self,
        segment_position: usize,
        start_byte: usize,
        end_byte: usize,
        replacement: impl Into<String>,
    ) -> Result<ReviewCaseId, ApplicationServiceError> {
        let anchor = self
            .transcript
            .anchor(segment_position, start_byte, end_byte)
            .map_err(ApplicationServiceError::HumanRaisedAnchorInvalid)?;
        let observed_text = self
            .transcript
            .resolve(&anchor)
            .ok_or(ApplicationServiceError::HumanRaisedRevisionStale)?
            .to_owned();

        let requested = (segment_position, start_byte, end_byte);
        if human_raise_blocked_ranges(self)
            .into_iter()
            .any(|occupied| spans_overlap(requested, occupied))
        {
            return Err(ApplicationServiceError::HumanRaisedOverlap);
        }

        let replacement = ManualReplacementText::new(replacement, &observed_text)
            .map_err(ApplicationServiceError::ManualReplacement)?;
        let case_id = ReviewCaseId::human(self.human_raised_cases.len());
        let review_case =
            ReviewCase::human_raised(case_id, HumanSelectedSpan::new(anchor, observed_text));
        let observed_revision = self.transcript.revision_id();

        self.ledger
            .record_case_raised(&review_case, observed_revision)
            .map_err(ApplicationServiceError::Decision)?;
        self.ledger
            .record_decision(
                &review_case,
                observed_revision,
                CorrectionDecision::ManualReplacement { replacement },
            )
            .map_err(ApplicationServiceError::Decision)?;
        self.human_raised_cases.push(review_case);

        Ok(case_id)
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
                    terminology_proposal_target: None,
                })
            }
            ApplicationReviewTarget::HumanRaisedCase { case_id } => {
                if matches!(decision, CorrectionDecision::AcceptAlternative { .. }) {
                    return Err(ApplicationServiceError::Decision(
                        ReviewLedgerError::AcceptAlternativeOnHumanRaised { case_id },
                    ));
                }
                let review_case = resolve_human_raised_case(&self.human_raised_cases, case_id)
                    .ok_or(ApplicationServiceError::UnknownReviewCase { case_id })?;
                let decision =
                    revalidate_decision_for_case(&self.transcript, review_case, decision)?;
                Ok(PreparedHumanDecision {
                    target,
                    decision,
                    expected_review_ledger_head: self.review_ledger_head(),
                    reuse_proposal_target: None,
                    terminology_proposal_target: None,
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
                    terminology_proposal_target: None,
                })
            }
            ApplicationReviewTarget::ProjectTerminologyProposal {
                analysis_snapshot,
                target_identity,
            } => {
                let prepared_target = self.prepare_terminology_target(target_identity)?;
                if analysis_snapshot != prepared_target.analysis_snapshot() {
                    return Err(ApplicationServiceError::TargetAnalysisMismatch);
                }
                let decision = revalidate_decision_for_terminology(
                    &self.transcript,
                    &prepared_target,
                    decision,
                )?;
                Ok(PreparedHumanDecision {
                    target,
                    decision,
                    expected_review_ledger_head: self.review_ledger_head(),
                    reuse_proposal_target: None,
                    terminology_proposal_target: Some(prepared_target),
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

    fn prepare_terminology_target(
        &self,
        target_identity: ProjectTerminologyProposalTargetIdentity,
    ) -> Result<ProjectTerminologyProposalTarget, ApplicationServiceError> {
        if !self.project_reuse.project_memory_available {
            return Err(ApplicationServiceError::ProjectEvidenceUnverifiable);
        }
        if !self.project_terminology.compose {
            return Err(ApplicationServiceError::UnknownTerminologyProposal);
        }
        let frozen = self
            .project_reuse
            .frozen
            .as_ref()
            .ok_or(ApplicationServiceError::UnknownTerminologyProposal)?;
        match self.project_reuse.current_project_snapshot {
            Some(current) if current == frozen.project_memory_snapshot_identity => {}
            _ => return Err(ApplicationServiceError::StaleReuseAnalysis),
        }
        let derived = derived_terminology_targets_from_session(self);
        let target = derived
            .into_iter()
            .find(|item| item.identity() == target_identity)
            .ok_or(ApplicationServiceError::UnknownTerminologyProposal)?;
        if collapsed_same_replacement_canonical_text(
            self,
            target.occurrence_key(),
            target.proposed_replacement(),
        )
        .is_some()
        {
            return Err(ApplicationServiceError::UnknownTerminologyProposal);
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
                resolve_selected_source_text(&self.transcript, review_case)?
            }
            ApplicationReviewTarget::HumanRaisedCase { case_id } => {
                let review_case = resolve_human_raised_case(&self.human_raised_cases, case_id)
                    .ok_or(ApplicationServiceError::UnknownReviewCase { case_id })?;
                resolve_selected_source_text(&self.transcript, review_case)?
            }
            ApplicationReviewTarget::ProjectReuseProposal {
                target_identity, ..
            } => {
                let prepared_target = self.prepare_reuse_target(target_identity)?;
                prepared_target.occurrence().observed_text.clone()
            }
            ApplicationReviewTarget::ProjectTerminologyProposal {
                target_identity, ..
            } => {
                let prepared_target = self.prepare_terminology_target(target_identity)?;
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
        if let Some(terminology_target) = prepared.terminology_proposal_target {
            if !self
                .project_terminology
                .persisted_targets
                .iter()
                .any(|item| item.identity() == terminology_target.identity())
            {
                self.project_terminology
                    .persisted_targets
                    .push(terminology_target.clone());
            }
            return self
                .ledger
                .record_terminology_decision(
                    terminology_target.identity(),
                    self.transcript.revision_id(),
                    prepared.decision,
                )
                .map_err(ApplicationServiceError::Decision);
        }
        match prepared.reuse_proposal_target {
            None => {
                let case_id = prepared
                    .target
                    .case_id()
                    .ok_or(ApplicationServiceError::UnknownReuseProposal)?;
                let review_case = if case_id.is_human_raised() {
                    resolve_human_raised_case(&self.human_raised_cases, case_id)
                } else {
                    resolve_case(&self.canonical_run, case_id)
                }
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
            &self.human_raised_cases,
            &self.ledger,
            &self.project_reuse.persisted_targets,
            &self.project_terminology.persisted_targets,
        )
    }

    pub fn decision_summary(&self) -> ApplicationDecisionSummary {
        derive_decision_summary(
            &self.canonical_run,
            &self.human_raised_cases,
            &self.ledger,
            &self.project_reuse.persisted_targets,
            &self.project_terminology.persisted_targets,
        )
    }

    pub fn derive_current_projection(
        &self,
    ) -> Result<ApplicationCurrentProjection, ApplicationServiceError> {
        build_current_projection(
            &self.transcript,
            &self.canonical_run,
            &self.human_raised_cases,
            &self.ledger,
            &self.project_reuse.persisted_targets,
            &self.project_terminology.persisted_targets,
        )
    }

    pub fn materialize_reviewed_output(
        &self,
    ) -> Result<ApplicationReviewedOutput, ApplicationServiceError> {
        build_reviewed_output(
            &self.transcript,
            &self.canonical_run,
            &self.human_raised_cases,
            &self.ledger,
            &self.project_reuse.persisted_targets,
            &self.project_terminology.persisted_targets,
        )
    }

    pub fn materialize_review_export_bundle(
        &self,
    ) -> Result<ApplicationReviewExportBundle, ApplicationServiceError> {
        build_export_bundle(
            &self.transcript,
            &self.session_terms,
            &self.canonical_run,
            &self.human_raised_cases,
            &self.ledger,
            self.material_use,
            self.session_authority.clone(),
            &self.project_reuse.persisted_targets,
            &self.project_terminology.persisted_targets,
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
                    let review_case = if case_id.is_human_raised() {
                        resolve_human_raised_case(&self.human_raised_cases, *case_id)
                    } else {
                        resolve_case(&replay_run, *case_id)
                    }
                    .ok_or(ApplicationReplayError::Mismatch {
                        field: ApplicationReplayField::ReviewCases,
                    })?;
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
                ReviewLedgerEvent::TerminologyProposalDecisionRecorded {
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
                        .project_terminology
                        .persisted_targets
                        .iter()
                        .find(|item| item.identity() == *target_identity)
                        .ok_or(ApplicationReplayError::Mismatch {
                            field: ApplicationReplayField::LedgerEvents,
                        })?;
                    let replay_decision = revalidate_decision_for_terminology(
                        &self.transcript,
                        target,
                        decision.clone(),
                    )
                    .map_err(ApplicationReplayError::Service)?;
                    replay_ledger
                        .record_terminology_decision(
                            *target_identity,
                            *observed_revision,
                            replay_decision,
                        )
                        .map_err(ApplicationServiceError::Decision)
                        .map_err(ApplicationReplayError::Service)?;
                }
                ReviewLedgerEvent::CaseRaised {
                    case_id,
                    observed_revision,
                    ..
                } => {
                    if *observed_revision != self.transcript.revision_id() {
                        return Err(ApplicationReplayError::Mismatch {
                            field: ApplicationReplayField::LedgerEvents,
                        });
                    }
                    let review_case = self
                        .human_raised_cases
                        .iter()
                        .find(|case| case.id() == *case_id)
                        .ok_or(ApplicationReplayError::Mismatch {
                            field: ApplicationReplayField::LedgerEvents,
                        })?;
                    replay_ledger
                        .record_case_raised(review_case, *observed_revision)
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

        if effective_statuses(&replay_run, &self.human_raised_cases, &replay_ledger)
            != effective_statuses(&self.canonical_run, &self.human_raised_cases, &self.ledger)
        {
            return Err(ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::EffectiveStatuses,
            });
        }

        if derive_progress(
            &replay_run,
            &self.human_raised_cases,
            &replay_ledger,
            &self.project_reuse.persisted_targets,
            &self.project_terminology.persisted_targets,
        ) != self.progress()
        {
            return Err(ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::Progress,
            });
        }
        if derive_decision_summary(
            &replay_run,
            &self.human_raised_cases,
            &replay_ledger,
            &self.project_reuse.persisted_targets,
            &self.project_terminology.persisted_targets,
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
            &self.human_raised_cases,
            &replay_ledger,
            &self.project_reuse.persisted_targets,
            &self.project_terminology.persisted_targets,
        );
        let current_session_summary = derive_session_summary_projection(
            &self.transcript,
            self.session_terms.len(),
            &self.canonical_run,
            &self.human_raised_cases,
            &self.ledger,
            &self.project_reuse.persisted_targets,
            &self.project_terminology.persisted_targets,
        );
        if replay_session_summary != current_session_summary {
            return Err(ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::SessionSummary,
            });
        }

        let replay_projection = build_current_projection(
            &self.transcript,
            &replay_run,
            &self.human_raised_cases,
            &replay_ledger,
            &self.project_reuse.persisted_targets,
            &self.project_terminology.persisted_targets,
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
            &self.human_raised_cases,
            &replay_ledger,
            &self.project_reuse.persisted_targets,
            &self.project_terminology.persisted_targets,
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
            &self.human_raised_cases,
            &replay_ledger,
            self.material_use,
            self.session_authority.clone(),
            &self.project_reuse.persisted_targets,
            &self.project_terminology.persisted_targets,
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

fn resolve_human_raised_case(
    human_raised_cases: &[ReviewCase],
    case_id: ReviewCaseId,
) -> Option<&ReviewCase> {
    human_raised_cases
        .get(case_id.local_index())
        .filter(|review_case| review_case.id() == case_id)
}

fn resolve_selected_source_text(
    transcript: &Transcript,
    review_case: &ReviewCase,
) -> Result<String, ApplicationServiceError> {
    let anchor = review_case.source_anchor();
    transcript
        .resolve(&anchor)
        .map(str::to_owned)
        .ok_or(ApplicationServiceError::ReviewedOutput(
            ReviewedOutputError::AnchorResolutionFailed {
                case_id: review_case.id(),
            },
        ))
}

fn revalidate_decision_for_case(
    transcript: &Transcript,
    review_case: &ReviewCase,
    decision: CorrectionDecision,
) -> Result<CorrectionDecision, ApplicationServiceError> {
    let CorrectionDecision::ManualReplacement { replacement } = decision else {
        return Ok(decision);
    };
    let selected_source_text = resolve_selected_source_text(transcript, review_case)?;
    let replacement = ManualReplacementText::new(replacement.as_str(), &selected_source_text)
        .map_err(ApplicationServiceError::ManualReplacement)?;
    Ok(CorrectionDecision::ManualReplacement { replacement })
}

fn spans_overlap(left: (usize, usize, usize), right: (usize, usize, usize)) -> bool {
    left.0 == right.0 && left.1 < right.2 && right.1 < left.2
}

fn decision_materializes_text(status: &ReviewCaseStatus) -> bool {
    match status {
        ReviewCaseStatus::Decided { decision, .. } => matches!(
            decision,
            CorrectionDecision::AcceptAlternative { .. }
                | CorrectionDecision::ManualReplacement { .. }
        ),
        ReviewCaseStatus::Undecided => false,
    }
}

/// Source ranges a new human-raised span must not touch: existing human anchors, effective
/// materializing detector or reuse edits, and undecided cards still awaiting a decision.
fn human_raise_blocked_ranges(session: &ApplicationReviewSession) -> Vec<(usize, usize, usize)> {
    let mut occupied = Vec::new();

    for review_case in &session.human_raised_cases {
        let anchor = review_case.source_anchor();
        occupied.push((
            anchor.segment_position(),
            anchor.start_byte(),
            anchor.end_byte(),
        ));
    }

    for review_case in session.canonical_run.review_cases() {
        let status = session.ledger.status_for(review_case.id());
        if matches!(status, ReviewCaseStatus::Undecided) || decision_materializes_text(&status) {
            let anchor = review_case.source_anchor();
            occupied.push((
                anchor.segment_position(),
                anchor.start_byte(),
                anchor.end_byte(),
            ));
        }
    }

    for target in visible_reuse_targets(session) {
        let status = session.ledger.status_for_reuse(target.identity());
        if matches!(status, ReviewCaseStatus::Undecided) || decision_materializes_text(&status) {
            occupied.push(target.occurrence_key());
        }
    }

    for target in visible_terminology_targets(session) {
        let status = session.ledger.status_for_terminology(target.identity());
        if matches!(status, ReviewCaseStatus::Undecided) || decision_materializes_text(&status) {
            occupied.push(target.occurrence_key());
        }
    }

    occupied
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

fn revalidate_decision_for_terminology(
    _transcript: &Transcript,
    target: &ProjectTerminologyProposalTarget,
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

fn derived_terminology_targets_from_session(
    session: &ApplicationReviewSession,
) -> Vec<ProjectTerminologyProposalTarget> {
    if !session.project_terminology.compose {
        return Vec::new();
    }
    let (Some(frozen), Some(scope)) = (
        session.project_reuse.frozen.as_ref(),
        session.reuse_state.project_scope(),
    ) else {
        return Vec::new();
    };
    let Ok(records) = crate::application_reuse::active_reusable_records(
        session.reuse_parts(),
        &session.reuse_state,
    ) else {
        return Vec::new();
    };
    derive_project_terminology_proposal_targets(
        &session.transcript,
        &session.session_terms,
        &records,
        &scope.stable_id,
        frozen.project_memory_snapshot_identity,
        frozen.governance_event_boundary,
    )
    .unwrap_or_default()
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

fn collapsed_same_replacement_canonical_text(
    session: &ApplicationReviewSession,
    occurrence_key: (usize, usize, usize),
    proposed_replacement: &str,
) -> Option<ReviewCaseId> {
    session
        .canonical_run
        .review_cases()
        .iter()
        .find_map(|review_case| {
            if canonical_occurrence_key(review_case) == occurrence_key
                && canonical_proposed_replacement(review_case) == Some(proposed_replacement)
            {
                Some(review_case.id())
            } else {
                None
            }
        })
}

fn collapsed_same_replacement_canonical(
    session: &ApplicationReviewSession,
    target: &ReuseProposalTarget,
) -> Option<ReviewCaseId> {
    collapsed_same_replacement_canonical_text(
        session,
        target.occurrence_key(),
        target.proposed_replacement(),
    )
}

fn visible_reuse_targets(session: &ApplicationReviewSession) -> Vec<ReuseProposalTarget> {
    if session.project_reuse.compose {
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
    }
}

fn visible_terminology_targets(
    session: &ApplicationReviewSession,
) -> Vec<ProjectTerminologyProposalTarget> {
    if session.project_terminology.compose {
        derived_terminology_targets_from_session(session)
    } else {
        session
            .project_terminology
            .persisted_targets
            .iter()
            .filter(|target| {
                !matches!(
                    session.ledger.status_for_terminology(target.identity()),
                    ReviewCaseStatus::Undecided
                )
            })
            .cloned()
            .collect()
    }
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

    for review_case in &session.human_raised_cases {
        items.push(ApplicationReviewItem {
            target: ApplicationReviewTarget::HumanRaisedCase {
                case_id: review_case.id(),
            },
            review_case: review_case.clone(),
            status: session.ledger.status_for(review_case.id()),
            kind: ApplicationReviewItemKind::HumanRaisedCase {},
        });
    }

    let terminology_targets = visible_terminology_targets(session);
    for target in &terminology_targets {
        if collapsed_same_replacement_canonical_text(
            session,
            target.occurrence_key(),
            target.proposed_replacement(),
        )
        .is_some()
        {
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
        let display_index = 20_000 + items.len();
        items.push(ApplicationReviewItem {
            target: ApplicationReviewTarget::ProjectTerminologyProposal {
                analysis_snapshot: target.analysis_snapshot(),
                target_identity: target.identity(),
            },
            review_case: target.display_review_case(display_index),
            status: session.ledger.status_for_terminology(target.identity()),
            kind: ApplicationReviewItemKind::ProjectTerminologyProposal {
                conflict_with_canonical,
            },
        });
    }

    for target in visible_reuse_targets(session) {
        if collapsed_same_replacement_canonical(session, &target).is_some() {
            continue;
        }
        let terminology_same_y = terminology_targets.iter().any(|item| {
            item.occurrence_key() == target.occurrence_key()
                && item.proposed_replacement() == target.proposed_replacement()
        });
        if terminology_same_y {
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
                    && !conflict_with_canonical
                    && !terminology_same_y,
                conflict_with_canonical,
            },
        });
    }

    items.sort_by(|left, right| {
        let left_anchor = left.review_case.source_anchor();
        let right_anchor = right.review_case.source_anchor();
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
        ApplicationReviewItemKind::HumanRaisedCase {} => 1,
        ApplicationReviewItemKind::ProjectTerminologyProposal { .. } => 2,
        ApplicationReviewItemKind::ProjectReuseProposal { .. } => 3,
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
    human_raised_cases: &[ReviewCase],
    ledger: &ReviewLedger,
) -> Vec<(ReviewCaseId, ReviewCaseStatus)> {
    canonical_run
        .review_cases()
        .iter()
        .chain(human_raised_cases)
        .map(|review_case| {
            let case_id = review_case.id();
            (case_id, ledger.status_for(case_id))
        })
        .collect()
}

fn derive_progress(
    canonical_run: &CanonicalTermReviewRun,
    human_raised_cases: &[ReviewCase],
    ledger: &ReviewLedger,
    reuse_targets: &[ReuseProposalTarget],
    terminology_targets: &[ProjectTerminologyProposalTarget],
) -> ApplicationReviewProgress {
    let summary = derive_decision_summary(
        canonical_run,
        human_raised_cases,
        ledger,
        reuse_targets,
        terminology_targets,
    );
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
    human_raised_cases: &[ReviewCase],
    ledger: &ReviewLedger,
    reuse_targets: &[ReuseProposalTarget],
    terminology_targets: &[ProjectTerminologyProposalTarget],
) -> ApplicationDecisionSummary {
    let mut summary = ApplicationDecisionSummary {
        total_review_cases: canonical_run.review_cases().len() + human_raised_cases.len(),
        total_recorded_events: ledger.events().len(),
        accepted_alternatives: 0,
        manual_replacements: 0,
        rejected: 0,
        deferred: 0,
        needs_manual_correction: 0,
        undecided: 0,
    };

    for review_case in canonical_run
        .review_cases()
        .iter()
        .chain(human_raised_cases)
    {
        apply_decided_status_counts(&mut summary, ledger.status_for(review_case.id()));
    }
    for target in reuse_targets {
        match ledger.status_for_reuse(target.identity()) {
            ReviewCaseStatus::Undecided => {}
            decided => apply_decided_status_counts(&mut summary, decided),
        }
    }
    for target in terminology_targets {
        match ledger.status_for_terminology(target.identity()) {
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
    human_raised_cases: &[ReviewCase],
    ledger: &ReviewLedger,
    reuse_targets: &[ReuseProposalTarget],
    terminology_targets: &[ProjectTerminologyProposalTarget],
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

    for review_case in canonical_run
        .review_cases()
        .iter()
        .chain(human_raised_cases)
    {
        match ledger.status_for(review_case.id()) {
            ReviewCaseStatus::Undecided => {}
            ReviewCaseStatus::Decided { decision, .. } => {
                let replacement_text = match decision {
                    CorrectionDecision::AcceptAlternative { alternative_index } => review_case
                        .as_detector_span()
                        .and_then(|span| span.alternatives().get(alternative_index))
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
                    affected_segments.insert(review_case.source_anchor().segment_position());
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
    for target in terminology_targets {
        match ledger.status_for_terminology(target.identity()) {
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
        review_cases_raised: canonical_run.review_cases().len() + human_raised_cases.len(),
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
        .filter_map(|(event_index, event)| match event {
            ReviewLedgerEvent::DecisionRecorded {
                case_id,
                observed_revision,
                decision,
            } => Some(ApplicationDecisionProjectionRecord {
                event_index,
                case_id: Some(*case_id),
                reuse_proposal_target_identity: None,
                terminology_proposal_target_identity: None,
                observed_revision: *observed_revision,
                decision: decision.clone(),
                session_authority: session_authority.clone(),
            }),
            ReviewLedgerEvent::ReuseProposalDecisionRecorded {
                target_identity,
                observed_revision,
                decision,
            } => Some(ApplicationDecisionProjectionRecord {
                event_index,
                case_id: None,
                reuse_proposal_target_identity: Some(*target_identity),
                terminology_proposal_target_identity: None,
                observed_revision: *observed_revision,
                decision: decision.clone(),
                session_authority: session_authority.clone(),
            }),
            ReviewLedgerEvent::TerminologyProposalDecisionRecorded {
                target_identity,
                observed_revision,
                decision,
            } => Some(ApplicationDecisionProjectionRecord {
                event_index,
                case_id: None,
                reuse_proposal_target_identity: None,
                terminology_proposal_target_identity: Some(*target_identity),
                observed_revision: *observed_revision,
                decision: decision.clone(),
                session_authority: session_authority.clone(),
            }),
            ReviewLedgerEvent::CaseRaised { .. } => None,
        })
        .collect()
}

fn build_export_bundle(
    transcript: &Transcript,
    session_terms: &[SessionTermEntry],
    canonical_run: &CanonicalTermReviewRun,
    human_raised_cases: &[ReviewCase],
    ledger: &ReviewLedger,
    material_use: BoundApplicationMaterialUseDeclaration,
    session_authority: BoundDeclaredSessionAuthority,
    reuse_targets: &[ReuseProposalTarget],
    terminology_targets: &[ProjectTerminologyProposalTarget],
) -> Result<ApplicationReviewExportBundle, ApplicationServiceError> {
    let reviewed_output = build_reviewed_output(
        transcript,
        canonical_run,
        human_raised_cases,
        ledger,
        reuse_targets,
        terminology_targets,
    )?;
    let session_summary = derive_session_summary_projection(
        transcript,
        session_terms.len(),
        canonical_run,
        human_raised_cases,
        ledger,
        reuse_targets,
        terminology_targets,
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

/// Detector-raised and human-raised cases share one materialization plane, so overlap refusal in
/// `reviewed_output` sees both families together.
fn materializable_review_cases(
    canonical_run: &CanonicalTermReviewRun,
    human_raised_cases: &[ReviewCase],
) -> Vec<ReviewCase> {
    canonical_run
        .review_cases()
        .iter()
        .chain(human_raised_cases)
        .cloned()
        .collect()
}

fn build_current_projection(
    transcript: &Transcript,
    canonical_run: &CanonicalTermReviewRun,
    human_raised_cases: &[ReviewCase],
    ledger: &ReviewLedger,
    reuse_targets: &[ReuseProposalTarget],
    terminology_targets: &[ProjectTerminologyProposalTarget],
) -> Result<ApplicationCurrentProjection, ApplicationServiceError> {
    let materializable = materializable_review_cases(canonical_run, human_raised_cases);
    let srt = derive_reviewed_srt_with_proposals(
        transcript,
        &materializable,
        ledger,
        reuse_targets,
        terminology_targets,
    )
    .map_err(ApplicationServiceError::ReviewedOutput)?;

    Ok(ApplicationCurrentProjection {
        srt,
        progress: derive_progress(
            canonical_run,
            human_raised_cases,
            ledger,
            reuse_targets,
            terminology_targets,
        ),
        decision_summary: derive_decision_summary(
            canonical_run,
            human_raised_cases,
            ledger,
            reuse_targets,
            terminology_targets,
        ),
    })
}

fn build_reviewed_output(
    transcript: &Transcript,
    canonical_run: &CanonicalTermReviewRun,
    human_raised_cases: &[ReviewCase],
    ledger: &ReviewLedger,
    reuse_targets: &[ReuseProposalTarget],
    terminology_targets: &[ProjectTerminologyProposalTarget],
) -> Result<ApplicationReviewedOutput, ApplicationServiceError> {
    let progress = derive_progress(
        canonical_run,
        human_raised_cases,
        ledger,
        reuse_targets,
        terminology_targets,
    );
    if let ApplicationDecisionCoverage::Incomplete { undecided } = progress.decision_coverage {
        return Err(ApplicationServiceError::DecisionCoverageIncomplete { undecided });
    }

    let materializable = materializable_review_cases(canonical_run, human_raised_cases);
    let srt = derive_reviewed_srt_with_proposals(
        transcript,
        &materializable,
        ledger,
        reuse_targets,
        terminology_targets,
    )
    .map_err(ApplicationServiceError::ReviewedOutput)?;

    Ok(ApplicationReviewedOutput {
        srt,
        progress,
        decision_summary: derive_decision_summary(
            canonical_run,
            human_raised_cases,
            ledger,
            reuse_targets,
            terminology_targets,
        ),
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

    /// Cue text `Kafak and Postgres`: the detector raises `Kafak` at 0..5, leaving `Postgres` at
    /// 10..18 unflagged and available for a human-raised span.
    fn two_span_session() -> ApplicationReviewSession {
        let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak and Postgres")
            .expect("fixture transcript");
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
    fn human_raised_manual_replacement_materializes_and_replays() {
        let mut session = two_span_session();
        let detector_case = session.canonical_run.review_cases()[0].id();
        session
            .record_human_decision(
                ApplicationReviewTarget::CanonicalTermCase {
                    analysis_snapshot: session.canonical_run.analysis_run().snapshot(),
                    case_id: detector_case,
                },
                CorrectionDecision::Reject,
            )
            .expect("detector decision");

        let case_id = session
            .raise_and_manual_replace(0, 10, 18, "PostgreSQL")
            .expect("human-raised manual replacement");

        assert!(case_id.is_human_raised());
        assert_eq!(session.human_raised_cases().len(), 1);
        assert!(matches!(
            session.review_ledger().events()[1],
            ReviewLedgerEvent::CaseRaised { .. }
        ));
        assert!(matches!(
            session.review_ledger().events()[2],
            ReviewLedgerEvent::DecisionRecorded {
                decision: CorrectionDecision::ManualReplacement { .. },
                ..
            }
        ));
        assert_eq!(
            session
                .materialize_reviewed_output()
                .expect("reviewed output")
                .srt,
            "1\n00:00:00,000 --> 00:00:01,000\nKafak and PostgreSQL\n"
        );
        assert_eq!(session.source().segments()[0].text, "Kafak and Postgres");
        assert!(session.verify_in_memory_replay().is_ok());
    }

    #[test]
    fn human_raised_span_is_refused_when_it_overlaps_an_undecided_detector_case() {
        let mut session = two_span_session();

        assert_eq!(
            session.raise_and_manual_replace(0, 0, 5, "Kafka"),
            Err(ApplicationServiceError::HumanRaisedOverlap)
        );
        assert!(session.review_ledger().events().is_empty());
        assert!(session.human_raised_cases().is_empty());
    }

    #[test]
    fn second_human_raised_span_overlapping_the_first_is_refused() {
        let mut session = two_span_session();
        session
            .raise_and_manual_replace(0, 10, 18, "PostgreSQL")
            .expect("first human-raised span");

        assert_eq!(
            session.raise_and_manual_replace(0, 14, 18, "SQL"),
            Err(ApplicationServiceError::HumanRaisedOverlap)
        );
        assert_eq!(session.human_raised_cases().len(), 1);
        assert_eq!(session.review_ledger().events().len(), 2);
    }

    #[test]
    fn human_raised_span_refuses_invalid_and_non_materializing_payloads() {
        let mut session = two_span_session();

        assert!(matches!(
            session.raise_and_manual_replace(0, 10, 10, "PostgreSQL"),
            Err(ApplicationServiceError::HumanRaisedAnchorInvalid(_))
        ));
        assert_eq!(
            session.raise_and_manual_replace(0, 10, 18, "Postgres"),
            Err(ApplicationServiceError::ManualReplacement(
                ManualReplacementTextError::IdenticalToSelectedSource
            ))
        );
        assert!(session.review_ledger().events().is_empty());
    }

    #[test]
    fn accept_alternative_on_a_human_raised_target_is_refused() {
        let mut session = two_span_session();
        let case_id = session
            .raise_and_manual_replace(0, 10, 18, "PostgreSQL")
            .expect("human-raised manual replacement");

        assert_eq!(
            session.record_human_decision(
                ApplicationReviewTarget::HumanRaisedCase { case_id },
                CorrectionDecision::AcceptAlternative {
                    alternative_index: 0
                },
            ),
            Err(ApplicationServiceError::Decision(
                ReviewLedgerError::AcceptAlternativeOnHumanRaised { case_id }
            ))
        );
        assert_eq!(session.review_ledger().events().len(), 2);
    }

    #[test]
    fn human_raised_case_appears_as_a_review_item_bound_to_no_analysis_snapshot() {
        let mut session = two_span_session();
        let case_id = session
            .raise_and_manual_replace(0, 10, 18, "PostgreSQL")
            .expect("human-raised manual replacement");

        let item = session
            .review_items()
            .into_iter()
            .find(|item| item.target.case_id() == Some(case_id))
            .expect("human-raised review item");

        assert_eq!(item.kind, ApplicationReviewItemKind::HumanRaisedCase {});
        assert_eq!(item.target.analysis_snapshot(), None);
        assert_eq!(item.review_case.as_detector_span(), None);
        assert_eq!(
            item.review_case
                .as_human_selection()
                .expect("human selection")
                .observed_text(),
            "Postgres"
        );
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

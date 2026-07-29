use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;

use crate::analysis::AnalysisSnapshot;
use crate::anchor::TranscriptRevisionId;
use crate::candidate::{DetectionError, DetectionKind, SessionTermEntry};
use crate::pipeline::{CanonicalTermReviewRun, run_canonical_term_review};
use crate::review::{
    CorrectionDecision, ManualReplacementText, ManualReplacementTextError, ReviewCase,
    ReviewCaseId, ReviewCaseStatus, ReviewLedger, ReviewLedgerError, ReviewLedgerEvent,
};
use crate::reviewed_output::{ReviewedOutputError, derive_reviewed_srt};
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
        let display_label = display_label.into();
        if display_label.chars().any(char::is_control) {
            return Err(DeclaredSessionAuthorityError::ControlCharacterInDisplayLabel);
        }
        let display_label = display_label.trim().to_string();
        if display_label.is_empty() {
            return Err(DeclaredSessionAuthorityError::EmptyDisplayLabel);
        }

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
pub struct ApplicationReviewTarget {
    analysis_snapshot: AnalysisSnapshot,
    case_id: ReviewCaseId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationReviewItem {
    pub target: ApplicationReviewTarget,
    pub review_case: ReviewCase,
    pub status: ReviewCaseStatus,
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
    pub case_id: ReviewCaseId,
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

pub struct ApplicationReviewSession {
    transcript: Transcript,
    session_terms: Vec<SessionTermEntry>,
    canonical_run: CanonicalTermReviewRun,
    ledger: ReviewLedger,
    material_use: BoundApplicationMaterialUseDeclaration,
    session_authority: BoundDeclaredSessionAuthority,
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
    })
}

impl ApplicationReviewSession {
    pub fn source(&self) -> &Transcript {
        &self.transcript
    }

    pub fn review_items(&self) -> Vec<ApplicationReviewItem> {
        let analysis_snapshot = self.canonical_run.analysis_run().snapshot();

        self.canonical_run
            .review_cases()
            .iter()
            .map(|review_case| ApplicationReviewItem {
                target: ApplicationReviewTarget {
                    analysis_snapshot,
                    case_id: review_case.id(),
                },
                review_case: review_case.clone(),
                status: self.ledger.status_for(review_case.id()),
            })
            .collect()
    }

    pub fn record_human_decision(
        &mut self,
        target: ApplicationReviewTarget,
        decision: CorrectionDecision,
    ) -> Result<(), ApplicationServiceError> {
        if target.analysis_snapshot != self.canonical_run.analysis_run().snapshot() {
            return Err(ApplicationServiceError::TargetAnalysisMismatch);
        }

        let review_case = resolve_case(&self.canonical_run, target.case_id).ok_or(
            ApplicationServiceError::UnknownReviewCase {
                case_id: target.case_id,
            },
        )?;
        let decision = revalidate_decision_for_case(&self.transcript, review_case, decision)?;

        self.ledger
            .record_decision(review_case, self.transcript.revision_id(), decision)
            .map_err(ApplicationServiceError::Decision)
    }

    pub fn record_manual_replacement(
        &mut self,
        target: ApplicationReviewTarget,
        replacement: impl Into<String>,
    ) -> Result<(), ApplicationServiceError> {
        if target.analysis_snapshot != self.canonical_run.analysis_run().snapshot() {
            return Err(ApplicationServiceError::TargetAnalysisMismatch);
        }

        let review_case = resolve_case(&self.canonical_run, target.case_id).ok_or(
            ApplicationServiceError::UnknownReviewCase {
                case_id: target.case_id,
            },
        )?;
        let selected_source_text = self
            .transcript
            .resolve(review_case.candidate_span().anchor())
            .ok_or(ApplicationServiceError::ReviewedOutput(
                ReviewedOutputError::AnchorResolutionFailed {
                    case_id: review_case.id(),
                },
            ))?;
        let replacement = ManualReplacementText::new(replacement, selected_source_text)
            .map_err(ApplicationServiceError::ManualReplacement)?;

        self.record_human_decision(
            target,
            CorrectionDecision::ManualReplacement { replacement },
        )
    }

    pub fn progress(&self) -> ApplicationReviewProgress {
        derive_progress(&self.canonical_run, &self.ledger)
    }

    pub fn decision_summary(&self) -> ApplicationDecisionSummary {
        derive_decision_summary(&self.canonical_run, &self.ledger)
    }

    pub fn derive_current_projection(
        &self,
    ) -> Result<ApplicationCurrentProjection, ApplicationServiceError> {
        build_current_projection(&self.transcript, &self.canonical_run, &self.ledger)
    }

    pub fn materialize_reviewed_output(
        &self,
    ) -> Result<ApplicationReviewedOutput, ApplicationServiceError> {
        build_reviewed_output(&self.transcript, &self.canonical_run, &self.ledger)
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
        )
    }

    pub fn verify_in_memory_replay(&self) -> Result<(), ApplicationReplayError> {
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
            let ReviewLedgerEvent::DecisionRecorded {
                case_id,
                observed_revision,
                decision,
            } = event;

            if *observed_revision != self.transcript.revision_id() {
                return Err(ApplicationReplayError::Mismatch {
                    field: ApplicationReplayField::LedgerEvents,
                });
            }

            let review_case =
                resolve_case(&replay_run, *case_id).ok_or(ApplicationReplayError::Mismatch {
                    field: ApplicationReplayField::ReviewCases,
                })?;
            let replay_decision =
                revalidate_decision_for_case(&self.transcript, review_case, decision.clone())
                    .map_err(ApplicationReplayError::Service)?;
            replay_ledger
                .record_decision(review_case, *observed_revision, replay_decision)
                .map_err(ApplicationServiceError::Decision)
                .map_err(ApplicationReplayError::Service)?;
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

        if derive_progress(&replay_run, &replay_ledger) != self.progress() {
            return Err(ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::Progress,
            });
        }
        if derive_decision_summary(&replay_run, &replay_ledger) != self.decision_summary() {
            return Err(ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::DecisionSummary,
            });
        }
        let replay_session_summary = derive_session_summary_projection(
            &self.transcript,
            self.session_terms.len(),
            &replay_run,
            &replay_ledger,
        );
        let current_session_summary = derive_session_summary_projection(
            &self.transcript,
            self.session_terms.len(),
            &self.canonical_run,
            &self.ledger,
        );
        if replay_session_summary != current_session_summary {
            return Err(ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::SessionSummary,
            });
        }

        let replay_projection =
            build_current_projection(&self.transcript, &replay_run, &replay_ledger);
        let current_projection = self.derive_current_projection();
        if replay_projection != current_projection {
            return Err(ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::CurrentProjection,
            });
        }

        let replay_output = build_reviewed_output(&self.transcript, &replay_run, &replay_ledger);
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
        );
        let current_bundle = self.materialize_review_export_bundle();
        if replay_bundle != current_bundle {
            return Err(ApplicationReplayError::Mismatch {
                field: ApplicationReplayField::ExportBundle,
            });
        }

        Ok(())
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
) -> ApplicationReviewProgress {
    let summary = derive_decision_summary(canonical_run, ledger);
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
        match ledger.status_for(review_case.id()) {
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

    summary
}

fn derive_session_summary_projection(
    transcript: &Transcript,
    session_term_entry_count: usize,
    canonical_run: &CanonicalTermReviewRun,
    ledger: &ReviewLedger,
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
        .map(|(event_index, event)| {
            let ReviewLedgerEvent::DecisionRecorded {
                case_id,
                observed_revision,
                decision,
            } = event;

            ApplicationDecisionProjectionRecord {
                event_index,
                case_id: *case_id,
                observed_revision: *observed_revision,
                decision: decision.clone(),
                session_authority: session_authority.clone(),
            }
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
) -> Result<ApplicationReviewExportBundle, ApplicationServiceError> {
    let reviewed_output = build_reviewed_output(transcript, canonical_run, ledger)?;
    let session_summary =
        derive_session_summary_projection(transcript, session_terms.len(), canonical_run, ledger);

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
) -> Result<ApplicationCurrentProjection, ApplicationServiceError> {
    let srt = derive_reviewed_srt(transcript, canonical_run.review_cases(), ledger)
        .map_err(ApplicationServiceError::ReviewedOutput)?;

    Ok(ApplicationCurrentProjection {
        srt,
        progress: derive_progress(canonical_run, ledger),
        decision_summary: derive_decision_summary(canonical_run, ledger),
    })
}

fn build_reviewed_output(
    transcript: &Transcript,
    canonical_run: &CanonicalTermReviewRun,
    ledger: &ReviewLedger,
) -> Result<ApplicationReviewedOutput, ApplicationServiceError> {
    let progress = derive_progress(canonical_run, ledger);
    if let ApplicationDecisionCoverage::Incomplete { undecided } = progress.decision_coverage {
        return Err(ApplicationServiceError::DecisionCoverageIncomplete { undecided });
    }

    let srt = derive_reviewed_srt(transcript, canonical_run.review_cases(), ledger)
        .map_err(ApplicationServiceError::ReviewedOutput)?;

    Ok(ApplicationReviewedOutput {
        srt,
        progress,
        decision_summary: derive_decision_summary(canonical_run, ledger),
    })
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
        let target = ApplicationReviewTarget {
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

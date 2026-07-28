use std::fmt;

use crate::analysis::AnalysisSnapshot;
use crate::anchor::TranscriptRevisionId;
use crate::candidate::{DetectionError, SessionTermEntry};
use crate::pipeline::{CanonicalTermReviewRun, run_canonical_term_review};
use crate::review::{
    CorrectionDecision, ReviewCase, ReviewCaseId, ReviewCaseStatus, ReviewLedger,
    ReviewLedgerError, ReviewLedgerEvent,
};
use crate::reviewed_output::{ReviewedOutputError, derive_reviewed_srt};
use crate::transcript::Transcript;

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

#[derive(Debug, PartialEq, Eq)]
pub enum ApplicationServiceError {
    Detection(DetectionError),
    TargetAnalysisMismatch,
    UnknownReviewCase { case_id: ReviewCaseId },
    Decision(ReviewLedgerError),
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
    CurrentProjection,
    ReviewedOutput,
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
}

pub fn begin_application_review(
    transcript: Transcript,
    session_terms: Vec<SessionTermEntry>,
    material_use: ApplicationMaterialUseDeclaration,
) -> Result<ApplicationReviewSession, ApplicationServiceError> {
    let source_revision = transcript.revision_id();
    let canonical_run = run_canonical_term_review(&transcript, &session_terms)
        .map_err(ApplicationServiceError::Detection)?;

    Ok(ApplicationReviewSession {
        transcript,
        session_terms,
        canonical_run,
        ledger: ReviewLedger::new(),
        material_use: BoundApplicationMaterialUseDeclaration {
            declaration: material_use,
            source_revision,
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

        self.ledger
            .record_decision(review_case, self.transcript.revision_id(), decision)
            .map_err(ApplicationServiceError::Decision)
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

    pub fn verify_in_memory_replay(&self) -> Result<(), ApplicationReplayError> {
        if self.material_use.source_revision != self.transcript.revision_id() {
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
            } = *event;

            if observed_revision != self.transcript.revision_id() {
                return Err(ApplicationReplayError::Mismatch {
                    field: ApplicationReplayField::LedgerEvents,
                });
            }

            let review_case =
                resolve_case(&replay_run, case_id).ok_or(ApplicationReplayError::Mismatch {
                    field: ApplicationReplayField::ReviewCases,
                })?;
            replay_ledger
                .record_decision(review_case, observed_revision, decision)
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
        )
        .expect("application session")
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

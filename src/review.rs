use crate::anchor::TranscriptRevisionId;
use crate::candidate::CandidateSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReviewCaseId {
    local_index: usize,
}

impl ReviewCaseId {
    pub(crate) fn local(local_index: usize) -> Self {
        Self { local_index }
    }

    pub fn local_index(self) -> usize {
        self.local_index
    }
}

/// The human-facing review unit, distinct from `CandidateSpan` (the
/// detector-level finding). For v0.1 this relationship is exactly 1:1: one
/// detector-raised `ReviewCase` wraps exactly one `CandidateSpan`. This type
/// intentionally carries no status, decision, or history: review status is
/// derived from append-only ledger events. Future aggregation of multiple
/// `CandidateSpan` values into one `ReviewCase` is deferred and not
/// implemented here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewCase {
    id: ReviewCaseId,
    candidate: CandidateSpan,
}

impl ReviewCase {
    pub(crate) fn detector_raised(id: ReviewCaseId, candidate: CandidateSpan) -> Self {
        Self { id, candidate }
    }

    pub(crate) fn from_detector_candidates(candidates: Vec<CandidateSpan>) -> Vec<Self> {
        candidates
            .into_iter()
            .enumerate()
            .map(|(position, candidate)| {
                Self::detector_raised(ReviewCaseId::local(position), candidate)
            })
            .collect()
    }

    pub fn id(&self) -> ReviewCaseId {
        self.id
    }

    pub fn candidate_span(&self) -> &CandidateSpan {
        &self.candidate
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorrectionDecision {
    Reject,
    Defer,
    AcceptAlternative { alternative_index: usize },
    NeedsManualCorrection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewLedgerEvent {
    DecisionRecorded {
        case_id: ReviewCaseId,
        observed_revision: TranscriptRevisionId,
        decision: CorrectionDecision,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewCaseStatus {
    Undecided,
    Decided {
        observed_revision: TranscriptRevisionId,
        decision: CorrectionDecision,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReviewLedgerError {
    AlternativeIndexOutOfRange {
        case_id: ReviewCaseId,
        alternative_index: usize,
        alternative_count: usize,
    },
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ReviewLedger {
    events: Vec<ReviewLedgerEvent>,
}

impl ReviewLedger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_decision(
        &mut self,
        review_case: &ReviewCase,
        observed_revision: TranscriptRevisionId,
        decision: CorrectionDecision,
    ) -> Result<(), ReviewLedgerError> {
        validate_decision(review_case, decision)?;

        self.events.push(ReviewLedgerEvent::DecisionRecorded {
            case_id: review_case.id(),
            observed_revision,
            decision,
        });

        Ok(())
    }

    pub fn status_for(&self, case_id: ReviewCaseId) -> ReviewCaseStatus {
        let mut status = ReviewCaseStatus::Undecided;

        for event in &self.events {
            match event {
                ReviewLedgerEvent::DecisionRecorded {
                    case_id: event_case_id,
                    observed_revision,
                    decision,
                } if *event_case_id == case_id => {
                    status = ReviewCaseStatus::Decided {
                        observed_revision: *observed_revision,
                        decision: *decision,
                    };
                }
                _ => {}
            }
        }

        status
    }

    pub fn events(&self) -> &[ReviewLedgerEvent] {
        &self.events
    }

    #[cfg(test)]
    pub(crate) fn from_events(events: Vec<ReviewLedgerEvent>) -> Self {
        Self { events }
    }
}

fn validate_decision(
    review_case: &ReviewCase,
    decision: CorrectionDecision,
) -> Result<(), ReviewLedgerError> {
    if let CorrectionDecision::AcceptAlternative { alternative_index } = decision {
        let alternative_count = review_case.candidate_span().alternatives().len();
        if alternative_index >= alternative_count {
            return Err(ReviewLedgerError::AlternativeIndexOutOfRange {
                case_id: review_case.id(),
                alternative_index,
                alternative_count,
            });
        }
    }

    Ok(())
}

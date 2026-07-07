use crate::candidate::CandidateSpan;

/// The human-facing review unit, distinct from `CandidateSpan` (the
/// detector-level finding). For v0.1 this relationship is exactly 1:1: one
/// `ReviewCase` wraps exactly one `CandidateSpan`. This type intentionally
/// carries no status, decision, or history: review-state and decision
/// semantics are not decided by this contract and must not be added here
/// without a separate accepted decision. Future aggregation of multiple
/// `CandidateSpan` values into one `ReviewCase` is deferred and not
/// implemented here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewCase {
    candidate: CandidateSpan,
}

impl From<CandidateSpan> for ReviewCase {
    fn from(candidate: CandidateSpan) -> Self {
        Self { candidate }
    }
}

impl ReviewCase {
    pub fn candidate_span(&self) -> &CandidateSpan {
        &self.candidate
    }
}

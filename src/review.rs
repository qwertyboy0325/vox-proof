use crate::anchor::{SourceAnchor, TranscriptRevisionId};
use crate::candidate::CandidateSpan;
use crate::project_terminology::ProjectTerminologyProposalTargetIdentity;
use crate::reuse_proposal_target::ReuseProposalTargetIdentity;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReviewCaseFamily {
    DetectorRaised,
    HumanRaised,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReviewCaseId {
    family: ReviewCaseFamily,
    local_index: usize,
}

impl ReviewCaseId {
    /// Detector-raised case identity. Kept as `local` for historical call sites.
    pub(crate) fn local(local_index: usize) -> Self {
        Self {
            family: ReviewCaseFamily::DetectorRaised,
            local_index,
        }
    }

    pub(crate) fn human(local_index: usize) -> Self {
        Self {
            family: ReviewCaseFamily::HumanRaised,
            local_index,
        }
    }

    pub fn family(self) -> ReviewCaseFamily {
        self.family
    }

    pub fn local_index(self) -> usize {
        self.local_index
    }

    pub fn is_human_raised(self) -> bool {
        matches!(self.family, ReviewCaseFamily::HumanRaised)
    }

    pub fn is_detector_raised(self) -> bool {
        matches!(self.family, ReviewCaseFamily::DetectorRaised)
    }
}

/// Contiguous human-selected span. Not Evidence and not detector output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HumanSelectedSpan {
    anchor: SourceAnchor,
    observed_text: String,
}

impl HumanSelectedSpan {
    pub(crate) fn new(anchor: SourceAnchor, observed_text: impl Into<String>) -> Self {
        Self {
            anchor,
            observed_text: observed_text.into(),
        }
    }

    pub fn anchor(&self) -> SourceAnchor {
        self.anchor
    }

    pub fn observed_text(&self) -> &str {
        &self.observed_text
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewCaseOrigin {
    DetectorRaised { candidate: CandidateSpan },
    HumanRaised { selection: HumanSelectedSpan },
}

/// The human-facing review unit. Detector-raised cases wrap one `CandidateSpan`.
/// Human-raised cases wrap a `HumanSelectedSpan` and are never injected into
/// analysis snapshots. Review status is derived from append-only ledger events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewCase {
    id: ReviewCaseId,
    origin: ReviewCaseOrigin,
}

impl ReviewCase {
    pub(crate) fn detector_raised(id: ReviewCaseId, candidate: CandidateSpan) -> Self {
        debug_assert!(id.is_detector_raised());
        Self {
            id,
            origin: ReviewCaseOrigin::DetectorRaised { candidate },
        }
    }

    pub(crate) fn human_raised(id: ReviewCaseId, selection: HumanSelectedSpan) -> Self {
        debug_assert!(id.is_human_raised());
        Self {
            id,
            origin: ReviewCaseOrigin::HumanRaised { selection },
        }
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

    pub fn origin(&self) -> &ReviewCaseOrigin {
        &self.origin
    }

    pub fn is_human_raised(&self) -> bool {
        self.id.is_human_raised()
    }

    /// Detector-raised candidate span. Panics if called on a HumanRaised case.
    pub fn candidate_span(&self) -> &CandidateSpan {
        self.as_detector_span()
            .expect("candidate_span requires a DetectorRaised ReviewCase")
    }

    pub fn as_detector_span(&self) -> Option<&CandidateSpan> {
        match &self.origin {
            ReviewCaseOrigin::DetectorRaised { candidate } => Some(candidate),
            ReviewCaseOrigin::HumanRaised { .. } => None,
        }
    }

    pub fn as_human_selection(&self) -> Option<&HumanSelectedSpan> {
        match &self.origin {
            ReviewCaseOrigin::HumanRaised { selection } => Some(selection),
            ReviewCaseOrigin::DetectorRaised { .. } => None,
        }
    }

    pub fn source_anchor(&self) -> SourceAnchor {
        match &self.origin {
            ReviewCaseOrigin::DetectorRaised { candidate } => *candidate.anchor(),
            ReviewCaseOrigin::HumanRaised { selection } => selection.anchor(),
        }
    }
}

pub const MAX_MANUAL_REPLACEMENT_UTF8_BYTES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualReplacementText {
    value: String,
}

impl ManualReplacementText {
    pub(crate) fn new(
        value: impl Into<String>,
        selected_source_text: &str,
    ) -> Result<Self, ManualReplacementTextError> {
        let value = value.into();

        if value.is_empty() {
            return Err(ManualReplacementTextError::Empty);
        }
        if value.chars().all(char::is_whitespace) {
            return Err(ManualReplacementTextError::WhitespaceOnly);
        }
        if value.len() > MAX_MANUAL_REPLACEMENT_UTF8_BYTES {
            return Err(ManualReplacementTextError::TooLong {
                utf8_bytes: value.len(),
                maximum_utf8_bytes: MAX_MANUAL_REPLACEMENT_UTF8_BYTES,
            });
        }
        if let Some((character_index, character)) = value
            .chars()
            .enumerate()
            .find(|(_, character)| character.is_control())
        {
            return Err(ManualReplacementTextError::UnicodeControl {
                character_index,
                character,
            });
        }
        if let Some((character_index, character)) = value
            .chars()
            .enumerate()
            .find(|(_, character)| matches!(character, '\u{2028}' | '\u{2029}'))
        {
            return Err(ManualReplacementTextError::UnicodeLineSeparator {
                character_index,
                character,
            });
        }
        if value.as_bytes() == selected_source_text.as_bytes() {
            return Err(ManualReplacementTextError::IdenticalToSelectedSource);
        }

        Ok(Self { value })
    }

    pub(crate) fn from_persisted_storage(value: String) -> Self {
        Self { value }
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManualReplacementTextError {
    Empty,
    WhitespaceOnly,
    UnicodeControl {
        character_index: usize,
        character: char,
    },
    UnicodeLineSeparator {
        character_index: usize,
        character: char,
    },
    IdenticalToSelectedSource,
    TooLong {
        utf8_bytes: usize,
        maximum_utf8_bytes: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorrectionDecision {
    Reject,
    Defer,
    AcceptAlternative { alternative_index: usize },
    NeedsManualCorrection,
    ManualReplacement { replacement: ManualReplacementText },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewLedgerEvent {
    CaseRaised {
        case_id: ReviewCaseId,
        observed_revision: TranscriptRevisionId,
        segment_position: usize,
        start_byte: usize,
        end_byte: usize,
        observed_text: String,
    },
    DecisionRecorded {
        case_id: ReviewCaseId,
        observed_revision: TranscriptRevisionId,
        decision: CorrectionDecision,
    },
    ReuseProposalDecisionRecorded {
        target_identity: ReuseProposalTargetIdentity,
        observed_revision: TranscriptRevisionId,
        decision: CorrectionDecision,
    },
    TerminologyProposalDecisionRecorded {
        target_identity: ProjectTerminologyProposalTargetIdentity,
        observed_revision: TranscriptRevisionId,
        decision: CorrectionDecision,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
    AcceptAlternativeOnHumanRaised {
        case_id: ReviewCaseId,
    },
    ReuseAlternativeIndexOutOfRange {
        alternative_index: usize,
    },
    CaseRaisedRequiresHumanRaised {
        case_id: ReviewCaseId,
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

    pub fn record_case_raised(
        &mut self,
        review_case: &ReviewCase,
        observed_revision: TranscriptRevisionId,
    ) -> Result<(), ReviewLedgerError> {
        let Some(selection) = review_case.as_human_selection() else {
            return Err(ReviewLedgerError::CaseRaisedRequiresHumanRaised {
                case_id: review_case.id(),
            });
        };
        let anchor = selection.anchor();
        self.events.push(ReviewLedgerEvent::CaseRaised {
            case_id: review_case.id(),
            observed_revision,
            segment_position: anchor.segment_position(),
            start_byte: anchor.start_byte(),
            end_byte: anchor.end_byte(),
            observed_text: selection.observed_text().to_owned(),
        });
        Ok(())
    }

    pub fn record_decision(
        &mut self,
        review_case: &ReviewCase,
        observed_revision: TranscriptRevisionId,
        decision: CorrectionDecision,
    ) -> Result<(), ReviewLedgerError> {
        validate_decision(review_case, &decision)?;

        self.events.push(ReviewLedgerEvent::DecisionRecorded {
            case_id: review_case.id(),
            observed_revision,
            decision,
        });

        Ok(())
    }

    pub fn record_reuse_decision(
        &mut self,
        target_identity: ReuseProposalTargetIdentity,
        observed_revision: TranscriptRevisionId,
        decision: CorrectionDecision,
    ) -> Result<(), ReviewLedgerError> {
        if let CorrectionDecision::AcceptAlternative { alternative_index } = decision {
            if alternative_index != 0 {
                return Err(ReviewLedgerError::ReuseAlternativeIndexOutOfRange {
                    alternative_index,
                });
            }
        }

        self.events
            .push(ReviewLedgerEvent::ReuseProposalDecisionRecorded {
                target_identity,
                observed_revision,
                decision,
            });

        Ok(())
    }

    pub fn record_terminology_decision(
        &mut self,
        target_identity: ProjectTerminologyProposalTargetIdentity,
        observed_revision: TranscriptRevisionId,
        decision: CorrectionDecision,
    ) -> Result<(), ReviewLedgerError> {
        if let CorrectionDecision::AcceptAlternative { alternative_index } = decision {
            if alternative_index != 0 {
                return Err(ReviewLedgerError::ReuseAlternativeIndexOutOfRange {
                    alternative_index,
                });
            }
        }

        self.events
            .push(ReviewLedgerEvent::TerminologyProposalDecisionRecorded {
                target_identity,
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
                        decision: decision.clone(),
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

    pub fn status_for_reuse(
        &self,
        target_identity: ReuseProposalTargetIdentity,
    ) -> ReviewCaseStatus {
        let mut status = ReviewCaseStatus::Undecided;
        for event in &self.events {
            if let ReviewLedgerEvent::ReuseProposalDecisionRecorded {
                target_identity: event_identity,
                observed_revision,
                decision,
            } = event
            {
                if *event_identity == target_identity {
                    status = ReviewCaseStatus::Decided {
                        observed_revision: *observed_revision,
                        decision: decision.clone(),
                    };
                }
            }
        }
        status
    }

    pub fn status_for_terminology(
        &self,
        target_identity: ProjectTerminologyProposalTargetIdentity,
    ) -> ReviewCaseStatus {
        let mut status = ReviewCaseStatus::Undecided;
        for event in &self.events {
            if let ReviewLedgerEvent::TerminologyProposalDecisionRecorded {
                target_identity: event_identity,
                observed_revision,
                decision,
            } = event
            {
                if *event_identity == target_identity {
                    status = ReviewCaseStatus::Decided {
                        observed_revision: *observed_revision,
                        decision: decision.clone(),
                    };
                }
            }
        }
        status
    }

    pub fn status_for_at_prefix(
        &self,
        case_id: ReviewCaseId,
        prefix_length: usize,
    ) -> ReviewCaseStatus {
        let mut status = ReviewCaseStatus::Undecided;
        for event in self.events.iter().take(prefix_length) {
            if let ReviewLedgerEvent::DecisionRecorded {
                case_id: event_case_id,
                observed_revision,
                decision,
            } = event
            {
                if *event_case_id == case_id {
                    status = ReviewCaseStatus::Decided {
                        observed_revision: *observed_revision,
                        decision: decision.clone(),
                    };
                }
            }
        }
        status
    }

    pub fn locate_effective_manual_replacement_at_prefix(
        &self,
        case_id: ReviewCaseId,
        prefix_length: usize,
    ) -> Option<usize> {
        self.events()
            .iter()
            .take(prefix_length)
            .enumerate()
            .rev()
            .find_map(|(index, event)| match event {
                ReviewLedgerEvent::DecisionRecorded {
                    case_id: event_case_id,
                    decision,
                    ..
                } if *event_case_id == case_id
                    && matches!(decision, CorrectionDecision::ManualReplacement { .. }) =>
                {
                    Some(index)
                }
                _ => None,
            })
    }

    #[cfg(test)]
    pub(crate) fn from_events(events: Vec<ReviewLedgerEvent>) -> Self {
        Self { events }
    }
}

fn validate_decision(
    review_case: &ReviewCase,
    decision: &CorrectionDecision,
) -> Result<(), ReviewLedgerError> {
    if review_case.is_human_raised() {
        if matches!(decision, CorrectionDecision::AcceptAlternative { .. }) {
            return Err(ReviewLedgerError::AcceptAlternativeOnHumanRaised {
                case_id: review_case.id(),
            });
        }
        return Ok(());
    }

    if let CorrectionDecision::AcceptAlternative { alternative_index } = decision {
        let alternative_count = review_case.candidate_span().alternatives().len();
        if *alternative_index >= alternative_count {
            return Err(ReviewLedgerError::AlternativeIndexOutOfRange {
                case_id: review_case.id(),
                alternative_index: *alternative_index,
                alternative_count,
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod manual_replacement_tests {
    use super::*;

    #[test]
    fn manual_replacement_preserves_exact_utf8_bytes() {
        let replacement =
            ManualReplacementText::new("  華碩 café  ", "華說").expect("valid replacement");

        assert_eq!(replacement.as_str().as_bytes(), "  華碩 café  ".as_bytes());
    }

    #[test]
    fn manual_replacement_rejects_empty_whitespace_and_identical_values() {
        assert_eq!(
            ManualReplacementText::new("", "source"),
            Err(ManualReplacementTextError::Empty)
        );
        assert_eq!(
            ManualReplacementText::new(" \u{3000}\t", "source"),
            Err(ManualReplacementTextError::WhitespaceOnly)
        );
        assert_eq!(
            ManualReplacementText::new("source", "source"),
            Err(ManualReplacementTextError::IdenticalToSelectedSource)
        );
    }

    #[test]
    fn manual_replacement_rejects_controls_and_unicode_line_separators() {
        assert!(matches!(
            ManualReplacementText::new("line\nbreak", "source"),
            Err(ManualReplacementTextError::UnicodeControl {
                character: '\n',
                ..
            })
        ));
        assert!(matches!(
            ManualReplacementText::new("line\u{2028}break", "source"),
            Err(ManualReplacementTextError::UnicodeLineSeparator {
                character: '\u{2028}',
                ..
            })
        ));
        assert!(matches!(
            ManualReplacementText::new("line\u{2029}break", "source"),
            Err(ManualReplacementTextError::UnicodeLineSeparator {
                character: '\u{2029}',
                ..
            })
        ));
    }

    #[test]
    fn manual_replacement_enforces_utf8_byte_limit() {
        let exact_limit = "a".repeat(MAX_MANUAL_REPLACEMENT_UTF8_BYTES);
        assert!(ManualReplacementText::new(exact_limit, "source").is_ok());

        let over_limit = "a".repeat(MAX_MANUAL_REPLACEMENT_UTF8_BYTES + 1);
        assert!(matches!(
            ManualReplacementText::new(over_limit, "source"),
            Err(ManualReplacementTextError::TooLong {
                utf8_bytes,
                maximum_utf8_bytes: MAX_MANUAL_REPLACEMENT_UTF8_BYTES,
            }) if utf8_bytes > MAX_MANUAL_REPLACEMENT_UTF8_BYTES
        ));
    }
}

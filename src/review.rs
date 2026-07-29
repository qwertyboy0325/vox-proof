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
    DecisionRecorded {
        case_id: ReviewCaseId,
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
        validate_decision(review_case, &decision)?;

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

    #[cfg(test)]
    pub(crate) fn from_events(events: Vec<ReviewLedgerEvent>) -> Self {
        Self { events }
    }
}

fn validate_decision(
    review_case: &ReviewCase,
    decision: &CorrectionDecision,
) -> Result<(), ReviewLedgerError> {
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

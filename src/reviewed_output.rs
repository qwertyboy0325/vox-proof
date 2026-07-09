use crate::review::{CorrectionDecision, ReviewCase, ReviewCaseId, ReviewCaseStatus, ReviewLedger};
use crate::transcript::{Segment, Transcript};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewedOutputError {
    RevisionMismatch {
        case_id: ReviewCaseId,
    },
    InvalidAlternativeIndex {
        case_id: ReviewCaseId,
        alternative_index: usize,
        alternative_count: usize,
    },
    AnchorResolutionFailed {
        case_id: ReviewCaseId,
    },
    OverlappingEdits {
        first_case_id: ReviewCaseId,
        second_case_id: ReviewCaseId,
    },
}

pub fn derive_reviewed_srt(
    transcript: &Transcript,
    review_cases: &[ReviewCase],
    ledger: &ReviewLedger,
) -> Result<String, ReviewedOutputError> {
    let mut edits = materializing_edits(transcript, review_cases, ledger)?;
    reject_overlapping_edits(&mut edits)?;

    let mut reviewed_texts: Vec<String> = transcript
        .segments()
        .iter()
        .map(|segment| segment.text.clone())
        .collect();

    for edit in edits.iter().rev() {
        let text = reviewed_texts.get_mut(edit.segment_position).ok_or(
            ReviewedOutputError::AnchorResolutionFailed {
                case_id: edit.case_id,
            },
        )?;
        text.replace_range(edit.start_byte..edit.end_byte, &edit.replacement_text);
    }

    Ok(serialize_canonical_srt(
        transcript.segments(),
        &reviewed_texts,
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MaterializingEdit {
    case_id: ReviewCaseId,
    segment_position: usize,
    start_byte: usize,
    end_byte: usize,
    replacement_text: String,
}

fn materializing_edits(
    transcript: &Transcript,
    review_cases: &[ReviewCase],
    ledger: &ReviewLedger,
) -> Result<Vec<MaterializingEdit>, ReviewedOutputError> {
    let mut edits = Vec::new();

    for review_case in review_cases {
        let case_id = review_case.id();
        let ReviewCaseStatus::Decided {
            observed_revision,
            decision: CorrectionDecision::AcceptAlternative { alternative_index },
        } = ledger.status_for(case_id)
        else {
            continue;
        };

        if observed_revision != transcript.revision_id() {
            return Err(ReviewedOutputError::RevisionMismatch { case_id });
        }

        let candidate = review_case.candidate_span();
        let alternatives = candidate.alternatives();
        let alternative = alternatives.get(alternative_index).ok_or(
            ReviewedOutputError::InvalidAlternativeIndex {
                case_id,
                alternative_index,
                alternative_count: alternatives.len(),
            },
        )?;

        let anchor = candidate.anchor();
        if transcript.resolve(anchor).is_none() {
            return Err(ReviewedOutputError::AnchorResolutionFailed { case_id });
        }

        edits.push(MaterializingEdit {
            case_id,
            segment_position: anchor.segment_position,
            start_byte: anchor.start_byte,
            end_byte: anchor.end_byte,
            replacement_text: alternative.replacement_text().to_string(),
        });
    }

    Ok(edits)
}

fn reject_overlapping_edits(edits: &mut [MaterializingEdit]) -> Result<(), ReviewedOutputError> {
    edits.sort_by_key(|edit| {
        (
            edit.segment_position,
            edit.start_byte,
            edit.end_byte,
            edit.case_id.local_index(),
        )
    });

    for pair in edits.windows(2) {
        let first = &pair[0];
        let second = &pair[1];

        if first.segment_position == second.segment_position
            && first.start_byte < second.end_byte
            && second.start_byte < first.end_byte
        {
            return Err(ReviewedOutputError::OverlappingEdits {
                first_case_id: first.case_id,
                second_case_id: second.case_id,
            });
        }
    }

    Ok(())
}

fn serialize_canonical_srt(segments: &[Segment], reviewed_texts: &[String]) -> String {
    let mut output = String::new();

    for (position, segment) in segments.iter().enumerate() {
        if position > 0 {
            output.push('\n');
        }

        output.push_str(&segment.index.to_string());
        output.push('\n');
        output.push_str(&format_timestamp(segment.start_ms));
        output.push_str(" --> ");
        output.push_str(&format_timestamp(segment.end_ms));
        output.push('\n');
        output.push_str(&reviewed_texts[position]);
        output.push('\n');
    }

    output
}

fn format_timestamp(total_ms: u64) -> String {
    let total_seconds = total_ms / 1000;
    let millis = total_ms % 1000;
    let seconds = total_seconds % 60;
    let total_minutes = total_seconds / 60;
    let minutes = total_minutes % 60;
    let hours = total_minutes / 60;

    format!("{hours:02}:{minutes:02}:{seconds:02},{millis:03}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anchor::SourceAnchor;
    use crate::candidate::{
        CandidateAlternative, CandidateSpan, DetectionKind, DetectorProvenance, Evidence,
        GlossaryEntry, GlossaryEvidence,
    };
    use crate::pipeline::run_glossary_review;
    use crate::review::{ReviewLedgerError, ReviewLedgerEvent};
    use crate::srt::parse_srt;

    fn glossary_entry(canonical_term: &str, aliases: &[&str]) -> GlossaryEntry {
        GlossaryEntry::new(
            canonical_term,
            aliases.iter().map(|alias| alias.to_string()).collect(),
        )
    }

    fn kafka_case(transcript: &Transcript) -> Vec<ReviewCase> {
        run_glossary_review(transcript, &[glossary_entry("Apache Kafka", &["Kafka"])])
            .expect("glossary has no ambiguous aliases")
    }

    fn record_decision(
        ledger: &mut ReviewLedger,
        review_case: &ReviewCase,
        transcript: &Transcript,
        decision: CorrectionDecision,
    ) {
        ledger
            .record_decision(review_case, transcript.revision_id(), decision)
            .expect("decision is valid for test case");
    }

    fn record_accept(ledger: &mut ReviewLedger, review_case: &ReviewCase, transcript: &Transcript) {
        record_decision(
            ledger,
            review_case,
            transcript,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        );
    }

    #[test]
    fn derives_unchanged_canonical_srt_when_there_are_no_accepted_decisions() {
        let transcript =
            parse_srt("1\r\n00:00:00,000 --> 00:00:01,250\r\nI use Kafka\r\n").expect("valid srt");
        let ledger = ReviewLedger::new();

        let reviewed =
            derive_reviewed_srt(&transcript, &[], &ledger).expect("derives reviewed srt");

        assert_eq!(reviewed, "1\n00:00:00,000 --> 00:00:01,250\nI use Kafka\n");
    }

    #[test]
    fn reject_produces_no_text_change() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nI use Kafka").expect("valid srt");
        let review_cases = kafka_case(&transcript);
        let mut ledger = ReviewLedger::new();
        record_decision(
            &mut ledger,
            &review_cases[0],
            &transcript,
            CorrectionDecision::Reject,
        );

        let reviewed =
            derive_reviewed_srt(&transcript, &review_cases, &ledger).expect("derives reviewed srt");

        assert_eq!(reviewed, "1\n00:00:00,000 --> 00:00:01,000\nI use Kafka\n");
    }

    #[test]
    fn defer_produces_no_text_change() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nI use Kafka").expect("valid srt");
        let review_cases = kafka_case(&transcript);
        let mut ledger = ReviewLedger::new();
        record_decision(
            &mut ledger,
            &review_cases[0],
            &transcript,
            CorrectionDecision::Defer,
        );

        let reviewed =
            derive_reviewed_srt(&transcript, &review_cases, &ledger).expect("derives reviewed srt");

        assert_eq!(reviewed, "1\n00:00:00,000 --> 00:00:01,000\nI use Kafka\n");
    }

    #[test]
    fn needs_manual_correction_produces_no_text_change() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nI use Kafka").expect("valid srt");
        let review_cases = kafka_case(&transcript);
        let mut ledger = ReviewLedger::new();
        record_decision(
            &mut ledger,
            &review_cases[0],
            &transcript,
            CorrectionDecision::NeedsManualCorrection,
        );

        let reviewed =
            derive_reviewed_srt(&transcript, &review_cases, &ledger).expect("derives reviewed srt");

        assert_eq!(reviewed, "1\n00:00:00,000 --> 00:00:01,000\nI use Kafka\n");
    }

    #[test]
    fn accept_alternative_replaces_candidate_anchor_range() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nI use Kafka").expect("valid srt");
        let review_cases = kafka_case(&transcript);
        let mut ledger = ReviewLedger::new();
        record_accept(&mut ledger, &review_cases[0], &transcript);

        let reviewed =
            derive_reviewed_srt(&transcript, &review_cases, &ledger).expect("derives reviewed srt");

        assert_eq!(
            reviewed,
            "1\n00:00:00,000 --> 00:00:01,000\nI use Apache Kafka\n"
        );
    }

    #[test]
    fn source_transcript_remains_unchanged_after_derivation() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nI use Kafka").expect("valid srt");
        let review_cases = kafka_case(&transcript);
        let mut ledger = ReviewLedger::new();
        record_accept(&mut ledger, &review_cases[0], &transcript);

        let _reviewed =
            derive_reviewed_srt(&transcript, &review_cases, &ledger).expect("derives reviewed srt");

        assert_eq!(transcript.segments()[0].text, "I use Kafka");
    }

    #[test]
    fn revision_mismatch_for_accept_alternative_refuses_output() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nI use Kafka").expect("valid srt");
        let other =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nI use Postgres").expect("valid srt");
        let review_cases = kafka_case(&transcript);
        let mut ledger = ReviewLedger::new();
        ledger
            .record_decision(
                &review_cases[0],
                other.revision_id(),
                CorrectionDecision::AcceptAlternative {
                    alternative_index: 0,
                },
            )
            .expect("alternative index exists");

        let result = derive_reviewed_srt(&transcript, &review_cases, &ledger);

        assert_eq!(
            result,
            Err(ReviewedOutputError::RevisionMismatch {
                case_id: review_cases[0].id(),
            })
        );
    }

    #[test]
    fn invalid_alternative_index_refuses_output() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nI use Kafka").expect("valid srt");
        let review_cases = kafka_case(&transcript);
        let ledger = ReviewLedger::from_events(vec![ReviewLedgerEvent::DecisionRecorded {
            case_id: review_cases[0].id(),
            observed_revision: transcript.revision_id(),
            decision: CorrectionDecision::AcceptAlternative {
                alternative_index: 1,
            },
        }]);

        let result = derive_reviewed_srt(&transcript, &review_cases, &ledger);

        assert_eq!(
            result,
            Err(ReviewedOutputError::InvalidAlternativeIndex {
                case_id: review_cases[0].id(),
                alternative_index: 1,
                alternative_count: 1,
            })
        );
    }

    #[test]
    fn public_ledger_api_rejects_invalid_alternative_index_before_derivation() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nI use Kafka").expect("valid srt");
        let review_cases = kafka_case(&transcript);
        let mut ledger = ReviewLedger::new();

        let result = ledger.record_decision(
            &review_cases[0],
            transcript.revision_id(),
            CorrectionDecision::AcceptAlternative {
                alternative_index: 1,
            },
        );

        assert_eq!(
            result,
            Err(ReviewLedgerError::AlternativeIndexOutOfRange {
                case_id: review_cases[0].id(),
                alternative_index: 1,
                alternative_count: 1,
            })
        );
    }

    #[test]
    fn anchor_resolution_failure_refuses_output() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nI use Kafka").expect("valid srt");
        let bad_anchor = SourceAnchor {
            revision: transcript.revision_id(),
            segment_position: 99,
            start_byte: 0,
            end_byte: 1,
        };
        let candidate = CandidateSpan::new(
            DetectionKind::GlossaryAliasMatch,
            DetectorProvenance::new("test-detector", "0.1.0"),
            bad_anchor,
            Evidence::Glossary(GlossaryEvidence {
                entry: glossary_entry("Apache Kafka", &["Kafka"]),
                matched_form: "Kafka".to_string(),
            }),
            vec![CandidateAlternative::new("Apache Kafka")],
        );
        let review_case = ReviewCase::detector_raised(ReviewCaseId::local(0), candidate);
        let ledger = ReviewLedger::from_events(vec![ReviewLedgerEvent::DecisionRecorded {
            case_id: review_case.id(),
            observed_revision: transcript.revision_id(),
            decision: CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        }]);

        let result = derive_reviewed_srt(&transcript, &[review_case.clone()], &ledger);

        assert_eq!(
            result,
            Err(ReviewedOutputError::AnchorResolutionFailed {
                case_id: review_case.id(),
            })
        );
    }

    #[test]
    fn overlapping_accepted_edits_in_same_segment_refuse_output() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafka and Postgres").expect("valid srt");
        let review_cases = run_glossary_review(
            &transcript,
            &[
                glossary_entry("Apache Kafka", &["Kafka"]),
                glossary_entry("Database stack", &["Kafka and Postgres"]),
            ],
        )
        .expect("glossary has no ambiguous aliases");
        let mut ledger = ReviewLedger::new();
        record_accept(&mut ledger, &review_cases[0], &transcript);
        record_accept(&mut ledger, &review_cases[1], &transcript);

        let result = derive_reviewed_srt(&transcript, &review_cases, &ledger);

        assert_eq!(
            result,
            Err(ReviewedOutputError::OverlappingEdits {
                first_case_id: review_cases[0].id(),
                second_case_id: review_cases[1].id(),
            })
        );
    }

    #[test]
    fn non_overlapping_accepted_edits_in_same_segment_both_apply() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafka and Postgres").expect("valid srt");
        let review_cases = run_glossary_review(
            &transcript,
            &[
                glossary_entry("Apache Kafka", &["Kafka"]),
                glossary_entry("PostgreSQL", &["Postgres"]),
            ],
        )
        .expect("glossary has no ambiguous aliases");
        let mut ledger = ReviewLedger::new();
        record_accept(&mut ledger, &review_cases[0], &transcript);
        record_accept(&mut ledger, &review_cases[1], &transcript);

        let reviewed =
            derive_reviewed_srt(&transcript, &review_cases, &ledger).expect("derives reviewed srt");

        assert_eq!(
            reviewed,
            "1\n00:00:00,000 --> 00:00:01,000\nApache Kafka and PostgreSQL\n"
        );
    }

    #[test]
    fn accepted_edits_across_different_segments_apply() {
        let transcript = parse_srt(
            "1\n00:00:00,000 --> 00:00:01,000\nKafka\n\n2\n00:00:01,000 --> 00:00:02,000\nPostgres",
        )
        .expect("valid srt");
        let review_cases = run_glossary_review(
            &transcript,
            &[
                glossary_entry("Apache Kafka", &["Kafka"]),
                glossary_entry("PostgreSQL", &["Postgres"]),
            ],
        )
        .expect("glossary has no ambiguous aliases");
        let mut ledger = ReviewLedger::new();
        record_accept(&mut ledger, &review_cases[0], &transcript);
        record_accept(&mut ledger, &review_cases[1], &transcript);

        let reviewed =
            derive_reviewed_srt(&transcript, &review_cases, &ledger).expect("derives reviewed srt");

        assert_eq!(
            reviewed,
            "1\n00:00:00,000 --> 00:00:01,000\nApache Kafka\n\n2\n00:00:01,000 --> 00:00:02,000\nPostgreSQL\n"
        );
    }

    #[test]
    fn canonical_srt_serialization_preserves_index_timing_order_and_reviewed_text() {
        let transcript = parse_srt(
            "7\n01:02:03,004 --> 01:02:04,005\nI use Kafka\n\n9\n100:00:00,000 --> 100:00:01,007\nunchanged",
        )
        .expect("valid srt");
        let review_cases = kafka_case(&transcript);
        let mut ledger = ReviewLedger::new();
        record_accept(&mut ledger, &review_cases[0], &transcript);

        let reviewed =
            derive_reviewed_srt(&transcript, &review_cases, &ledger).expect("derives reviewed srt");

        assert_eq!(
            reviewed,
            "7\n01:02:03,004 --> 01:02:04,005\nI use Apache Kafka\n\n9\n100:00:00,000 --> 100:00:01,007\nunchanged\n"
        );
    }
}

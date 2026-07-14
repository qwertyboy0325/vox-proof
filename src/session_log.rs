use crate::review::{CorrectionDecision, ReviewLedger, ReviewLedgerEvent};

const DECISION_LOG_HEADER: &str = "voxproof decision log v0";

pub fn render_decision_log(ledger: &ReviewLedger) -> String {
    let mut output = String::from(DECISION_LOG_HEADER);
    output.push('\n');

    for (position, event) in ledger.events().iter().enumerate() {
        output.push('\n');
        output.push_str(&format!("event {}\n", position + 1));
        render_event(event, &mut output);
    }

    output
}

fn render_event(event: &ReviewLedgerEvent, output: &mut String) {
    match event {
        ReviewLedgerEvent::DecisionRecorded {
            case_id,
            observed_revision,
            decision,
        } => {
            output.push_str("type: decision_recorded\n");
            output.push_str(&format!("case_id: local:{}\n", case_id.local_index()));
            output.push_str(&format!(
                "observed_revision: {}\n",
                observed_revision.to_tagged_string()
            ));
            render_decision(*decision, output);
        }
    }
}

fn render_decision(decision: CorrectionDecision, output: &mut String) {
    match decision {
        CorrectionDecision::Reject => output.push_str("decision: reject\n"),
        CorrectionDecision::Defer => output.push_str("decision: defer\n"),
        CorrectionDecision::AcceptAlternative { alternative_index } => {
            output.push_str("decision: accept_alternative\n");
            output.push_str(&format!("alternative_index: {alternative_index}\n"));
        }
        CorrectionDecision::NeedsManualCorrection => {
            output.push_str("decision: needs_manual_correction\n");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidate::SessionTermEntry;
    use crate::pipeline::run_term_review;
    use crate::review::ReviewCase;
    use crate::reviewed_output::derive_reviewed_srt;
    use crate::srt::parse_srt;
    use crate::transcript::Transcript;

    fn glossary_entry(canonical_term: &str, aliases: &[&str]) -> SessionTermEntry {
        SessionTermEntry::new(
            canonical_term,
            aliases.iter().map(|alias| alias.to_string()).collect(),
            Vec::new(),
        )
    }

    fn kafka_case(transcript: &Transcript) -> Vec<ReviewCase> {
        run_term_review(transcript, &[glossary_entry("Apache Kafka", &["Kafka"])])
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

    fn expected_decision_record(
        event_number: usize,
        review_case: &ReviewCase,
        transcript: &Transcript,
        decision_lines: &str,
    ) -> String {
        format!(
            "\nevent {event_number}\n\
             type: decision_recorded\n\
             case_id: local:{}\n\
             observed_revision: {}\n\
             {decision_lines}",
            review_case.id().local_index(),
            transcript.revision_id().to_tagged_string(),
        )
    }

    #[test]
    fn empty_ledger_renders_stable_header_and_trailing_newline() {
        let ledger = ReviewLedger::new();

        assert_eq!(render_decision_log(&ledger), "voxproof decision log v0\n");
    }

    #[test]
    fn single_reject_decision_renders_event_fields() {
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

        let expected = format!(
            "voxproof decision log v0\n{}",
            expected_decision_record(1, &review_cases[0], &transcript, "decision: reject\n")
        );

        assert_eq!(render_decision_log(&ledger), expected);
    }

    #[test]
    fn accept_alternative_renders_alternative_index() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nI use Kafka").expect("valid srt");
        let review_cases = kafka_case(&transcript);
        let mut ledger = ReviewLedger::new();
        record_decision(
            &mut ledger,
            &review_cases[0],
            &transcript,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        );

        let log = render_decision_log(&ledger);

        assert!(log.contains("decision: accept_alternative\n"));
        assert!(log.contains("alternative_index: 0\n"));
    }

    #[test]
    fn defer_renders_defer_decision() {
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

        assert!(render_decision_log(&ledger).contains("decision: defer\n"));
    }

    #[test]
    fn needs_manual_correction_renders_needs_manual_correction_decision() {
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

        assert!(render_decision_log(&ledger).contains("decision: needs_manual_correction\n"));
    }

    #[test]
    fn multiple_events_render_in_append_order_with_one_based_numbering() {
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
        record_decision(
            &mut ledger,
            &review_cases[0],
            &transcript,
            CorrectionDecision::Defer,
        );
        record_decision(
            &mut ledger,
            &review_cases[0],
            &transcript,
            CorrectionDecision::NeedsManualCorrection,
        );

        let expected = format!(
            "voxproof decision log v0\n{}{}{}",
            expected_decision_record(1, &review_cases[0], &transcript, "decision: reject\n"),
            expected_decision_record(2, &review_cases[0], &transcript, "decision: defer\n"),
            expected_decision_record(
                3,
                &review_cases[0],
                &transcript,
                "decision: needs_manual_correction\n"
            ),
        );

        assert_eq!(render_decision_log(&ledger), expected);
    }

    #[test]
    fn later_decisions_for_same_case_are_not_folded_away() {
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
        record_decision(
            &mut ledger,
            &review_cases[0],
            &transcript,
            CorrectionDecision::Defer,
        );

        let log = render_decision_log(&ledger);

        assert!(log.contains("event 1\ntype: decision_recorded\n"));
        assert!(log.contains("decision: reject\n"));
        assert!(log.contains("event 2\ntype: decision_recorded\n"));
        assert!(log.contains("decision: defer\n"));
    }

    #[test]
    fn rendering_does_not_mutate_ledger() {
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
        let before = ledger.events().to_vec();

        let _log = render_decision_log(&ledger);

        assert_eq!(ledger.events(), before.as_slice());
    }

    #[test]
    fn decision_log_rendering_is_independent_from_reviewed_output_derivation() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nI use Kafka").expect("valid srt");
        let review_cases = kafka_case(&transcript);
        let mut ledger = ReviewLedger::new();
        record_decision(
            &mut ledger,
            &review_cases[0],
            &transcript,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        );

        let log = render_decision_log(&ledger);
        let reviewed =
            derive_reviewed_srt(&transcript, &review_cases, &ledger).expect("derives reviewed srt");

        assert!(log.contains("decision: accept_alternative\n"));
        assert_eq!(
            reviewed,
            "1\n00:00:00,000 --> 00:00:01,000\nI use Apache Kafka\n"
        );
    }
}

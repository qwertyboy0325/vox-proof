use std::collections::{BTreeMap, HashMap, HashSet};

use crate::anchor::TranscriptRevisionId;
use crate::candidate::DetectionKind;
use crate::review::{CorrectionDecision, ReviewCase, ReviewCaseStatus, ReviewLedger};
use crate::transcript::Transcript;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionTiming {
    pub start_unix_ms: u128,
    pub end_unix_ms: u128,
    pub elapsed_ms: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionOutputPaths {
    pub reviewed_srt: String,
    pub decision_log: String,
    pub session_summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionInputPaths {
    pub input_srt: String,
    pub session_terms: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectionKindCount {
    pub kind: DetectionKind,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectorCount {
    pub detector_id: String,
    pub detector_version: String,
    pub count: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DecisionCounts {
    pub accepted_alternatives: usize,
    pub rejected: usize,
    pub deferred: usize,
    pub needs_manual_correction: usize,
    pub total_recorded_events: usize,
    pub undecided_review_cases: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptedReplacementCount {
    pub replacement_text: String,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionOutcomeCounts {
    pub accepted_replacements_materialized: usize,
    pub source_segments_affected: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSummary {
    pub transcript_revision: TranscriptRevisionId,
    pub transcript_segments: usize,
    pub inputs: SessionInputPaths,
    pub session_term_entries: usize,
    pub review_cases_raised: usize,
    pub cases_by_detection_kind: Vec<DetectionKindCount>,
    pub cases_by_detector: Vec<DetectorCount>,
    pub decisions: DecisionCounts,
    pub accepted_replacements: Vec<AcceptedReplacementCount>,
    pub outcomes: SessionOutcomeCounts,
    pub timing: SessionTiming,
    pub outputs: SessionOutputPaths,
}

pub struct CompletedSession<'a> {
    pub transcript: &'a Transcript,
    pub review_cases: &'a [ReviewCase],
    pub ledger: &'a ReviewLedger,
    pub session_term_entries: usize,
    pub inputs: SessionInputPaths,
    pub timing: SessionTiming,
    pub outputs: SessionOutputPaths,
}

pub fn collect_session_summary(completed: CompletedSession<'_>) -> SessionSummary {
    let mut kind_counts = HashMap::<DetectionKind, usize>::new();
    let mut detector_counts = BTreeMap::<(String, String), usize>::new();

    for review_case in completed.review_cases {
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
        .map(|(kind, count)| DetectionKindCount { kind, count })
        .collect::<Vec<_>>();
    cases_by_detection_kind.sort_by_key(|item| detection_kind_name(item.kind));

    let cases_by_detector = detector_counts
        .into_iter()
        .map(|((detector_id, detector_version), count)| DetectorCount {
            detector_id,
            detector_version,
            count,
        })
        .collect();

    let mut decisions = DecisionCounts {
        total_recorded_events: completed.ledger.events().len(),
        ..DecisionCounts::default()
    };
    let mut affected_segments = HashSet::new();
    let mut accepted_replacements = BTreeMap::<String, usize>::new();

    for review_case in completed.review_cases {
        match completed.ledger.status_for(review_case.id()) {
            ReviewCaseStatus::Undecided => decisions.undecided_review_cases += 1,
            ReviewCaseStatus::Decided { decision, .. } => match decision {
                CorrectionDecision::AcceptAlternative { alternative_index } => {
                    decisions.accepted_alternatives += 1;
                    affected_segments
                        .insert(review_case.candidate_span().anchor().segment_position());

                    if let Some(alternative) = review_case
                        .candidate_span()
                        .alternatives()
                        .get(alternative_index)
                    {
                        *accepted_replacements
                            .entry(alternative.replacement_text().to_string())
                            .or_default() += 1;
                    }
                }
                CorrectionDecision::Reject => decisions.rejected += 1,
                CorrectionDecision::Defer => decisions.deferred += 1,
                CorrectionDecision::NeedsManualCorrection => {
                    decisions.needs_manual_correction += 1;
                }
            },
        }
    }

    let accepted_replacements = accepted_replacements
        .into_iter()
        .map(|(replacement_text, count)| AcceptedReplacementCount {
            replacement_text,
            count,
        })
        .collect();

    SessionSummary {
        transcript_revision: completed.transcript.revision_id(),
        transcript_segments: completed.transcript.segments().len(),
        inputs: completed.inputs,
        session_term_entries: completed.session_term_entries,
        review_cases_raised: completed.review_cases.len(),
        cases_by_detection_kind,
        cases_by_detector,
        outcomes: SessionOutcomeCounts {
            accepted_replacements_materialized: decisions.accepted_alternatives,
            source_segments_affected: affected_segments.len(),
        },
        decisions,
        accepted_replacements,
        timing: completed.timing,
        outputs: completed.outputs,
    }
}

pub fn render_session_summary(summary: &SessionSummary) -> String {
    let mut output = String::from(
        "VoxProof session summary (provisional)\n\
         Human-readable, session-scoped export. Not for machine re-import.\n\
         Not a Language Pack, durable persistence record, or validation evidence by itself.\n",
    );

    output.push_str("\nRun identity and inputs\n");
    output.push_str(&format!(
        "transcript_revision: {}\n",
        summary.transcript_revision.to_tagged_string()
    ));
    output.push_str(&format!(
        "transcript_segments: {}\n",
        summary.transcript_segments
    ));
    output.push_str(&format!("input_srt: {}\n", summary.inputs.input_srt));
    output.push_str(&format!(
        "session_terms: {}\n",
        summary.inputs.session_terms
    ));
    output.push_str(&format!(
        "session_term_entries: {}\n",
        summary.session_term_entries
    ));

    output.push_str("\nTiming\n");
    output.push_str(&format!(
        "session_start_unix_ms: {}\n",
        summary.timing.start_unix_ms
    ));
    output.push_str(&format!(
        "session_end_unix_ms: {}\n",
        summary.timing.end_unix_ms
    ));
    output.push_str(&format!(
        "session_elapsed_ms: {}\n",
        summary.timing.elapsed_ms
    ));
    output.push_str(&format!(
        "session_elapsed: {}\n",
        format_duration(summary.timing.elapsed_ms)
    ));

    output.push_str("\nCandidates\n");
    output.push_str(&format!(
        "review_cases_raised: {}\n",
        summary.review_cases_raised
    ));
    output.push_str("by_detection_kind:\n");
    if summary.cases_by_detection_kind.is_empty() {
        output.push_str("  (none)\n");
    } else {
        for item in &summary.cases_by_detection_kind {
            output.push_str(&format!(
                "  {}: {}\n",
                detection_kind_name(item.kind),
                item.count
            ));
        }
    }
    output.push_str("by_detector:\n");
    if summary.cases_by_detector.is_empty() {
        output.push_str("  (none)\n");
    } else {
        for item in &summary.cases_by_detector {
            output.push_str(&format!(
                "  {} @ {}: {}\n",
                item.detector_id, item.detector_version, item.count
            ));
        }
    }

    output.push_str("\nSession correction profile\n");
    output.push_str("decision_counts_basis: effective last-decision-wins status per review case\n");
    output.push_str(&format!(
        "accepted_alternatives: {}\n",
        summary.decisions.accepted_alternatives
    ));
    output.push_str(&format!("rejected: {}\n", summary.decisions.rejected));
    output.push_str(&format!("deferred: {}\n", summary.decisions.deferred));
    output.push_str(&format!(
        "needs_manual_correction: {}\n",
        summary.decisions.needs_manual_correction
    ));
    output.push_str(&format!(
        "total_decisions_recorded: {}\n",
        summary.decisions.total_recorded_events
    ));
    output.push_str(&format!(
        "undecided_review_cases: {}\n",
        summary.decisions.undecided_review_cases
    ));
    output.push_str("accepted_replacement_texts_this_session:\n");
    if summary.accepted_replacements.is_empty() {
        output.push_str("  (none)\n");
    } else {
        for item in &summary.accepted_replacements {
            output.push_str(&format!("  {}: {}\n", item.replacement_text, item.count));
        }
    }

    output.push_str("\nOutcomes\n");
    output.push_str(&format!(
        "accepted_replacements_materialized: {}\n",
        summary.outcomes.accepted_replacements_materialized
    ));
    output.push_str(&format!(
        "source_segments_affected: {}\n",
        summary.outcomes.source_segments_affected
    ));
    output.push_str(
        "summary_generation_precondition: reviewed SRT materialized and decision log rendered\n",
    );

    output.push_str("\nOutputs\n");
    output.push_str(&format!("reviewed_srt: {}\n", summary.outputs.reviewed_srt));
    output.push_str(&format!("decision_log: {}\n", summary.outputs.decision_log));
    output.push_str(&format!(
        "session_summary: {}\n",
        summary.outputs.session_summary
    ));

    output
}

fn detection_kind_name(kind: DetectionKind) -> &'static str {
    match kind {
        DetectionKind::GlossaryAliasMatch => "glossary_alias_match",
        DetectionKind::MixedLanguageAnomaly => "mixed_language_anomaly",
        DetectionKind::PhoneticSimilarity => "phonetic_similarity",
        DetectionKind::RepeatedPhrase => "repeated_phrase",
    }
}

fn format_duration(total_ms: u128) -> String {
    let total_seconds = total_ms / 1000;
    let millis = total_ms % 1000;
    let seconds = total_seconds % 60;
    let total_minutes = total_seconds / 60;
    let minutes = total_minutes % 60;
    let hours = total_minutes / 60;

    format!("{hours:02}:{minutes:02}:{seconds:02}.{millis:03}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidate::{
        CandidateAlternative, CandidateSpan, DetectorProvenance, Evidence, GlossaryEntry,
        GlossaryEvidence,
    };
    use crate::pipeline::run_glossary_review;
    use crate::review::{ReviewCase, ReviewCaseId};
    use crate::srt::parse_srt;

    fn paths() -> (SessionInputPaths, SessionOutputPaths) {
        (
            SessionInputPaths {
                input_srt: "素材/字幕.srt".to_string(),
                session_terms: "素材/詞彙.txt".to_string(),
            },
            SessionOutputPaths {
                reviewed_srt: "輸出/reviewed.srt".to_string(),
                decision_log: "輸出/decision-log.txt".to_string(),
                session_summary: "輸出/session-summary.txt".to_string(),
            },
        )
    }

    fn timing() -> SessionTiming {
        SessionTiming {
            start_unix_ms: 1_700_000_000_000,
            end_unix_ms: 1_700_003_723_004,
            elapsed_ms: 3_723_004,
        }
    }

    fn glossary_entry(canonical_term: &str, aliases: &[&str]) -> GlossaryEntry {
        GlossaryEntry::new(
            canonical_term,
            aliases.iter().map(|alias| alias.to_string()).collect(),
        )
    }

    fn collect<'a>(
        transcript: &'a Transcript,
        review_cases: &'a [ReviewCase],
        ledger: &'a ReviewLedger,
        session_term_entries: usize,
    ) -> SessionSummary {
        let (inputs, outputs) = paths();
        collect_session_summary(CompletedSession {
            transcript,
            review_cases,
            ledger,
            session_term_entries,
            inputs,
            timing: timing(),
            outputs,
        })
    }

    #[test]
    fn deterministic_rendering_from_typed_summary_data() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\n使用 卡夫卡").expect("valid srt");
        let review_cases =
            run_glossary_review(&transcript, &[glossary_entry("Apache 卡夫卡", &["卡夫卡"])])
                .expect("valid glossary");
        let mut ledger = ReviewLedger::new();
        ledger
            .record_decision(
                &review_cases[0],
                transcript.revision_id(),
                CorrectionDecision::AcceptAlternative {
                    alternative_index: 0,
                },
            )
            .expect("valid decision");
        let summary = collect(&transcript, &review_cases, &ledger, 1);

        let first = render_session_summary(&summary);
        let second = render_session_summary(&summary);

        assert_eq!(first, second);
        assert!(first.contains(&transcript.revision_id().to_tagged_string()));
        assert!(first.contains("session_terms: 素材/詞彙.txt"));
        assert!(first.contains("Apache 卡夫卡: 1"));
    }

    #[test]
    fn zero_review_cases_render_zero_counts_and_empty_groups() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\n沒有候選").expect("valid srt");
        let ledger = ReviewLedger::new();

        let summary = collect(&transcript, &[], &ledger, 2);
        let rendered = render_session_summary(&summary);

        assert_eq!(summary.review_cases_raised, 0);
        assert_eq!(summary.decisions.undecided_review_cases, 0);
        assert!(rendered.contains("review_cases_raised: 0"));
        assert!(rendered.contains("by_detection_kind:\n  (none)"));
        assert!(rendered.contains("accepted_replacement_texts_this_session:\n  (none)"));
    }

    #[test]
    fn counts_multiple_effective_decision_kinds() {
        let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nA B C D").expect("valid srt");
        let review_cases = run_glossary_review(
            &transcript,
            &[
                glossary_entry("AA", &["A"]),
                glossary_entry("BB", &["B"]),
                glossary_entry("CC", &["C"]),
                glossary_entry("DD", &["D"]),
            ],
        )
        .expect("valid glossary");
        let mut ledger = ReviewLedger::new();
        let decisions = [
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
            CorrectionDecision::Reject,
            CorrectionDecision::Defer,
            CorrectionDecision::NeedsManualCorrection,
        ];
        for (review_case, decision) in review_cases.iter().zip(decisions) {
            ledger
                .record_decision(review_case, transcript.revision_id(), decision)
                .expect("valid decision");
        }

        let summary = collect(&transcript, &review_cases, &ledger, 4);

        assert_eq!(
            summary.decisions,
            DecisionCounts {
                accepted_alternatives: 1,
                rejected: 1,
                deferred: 1,
                needs_manual_correction: 1,
                total_recorded_events: 4,
                undecided_review_cases: 0,
            }
        );
    }

    #[test]
    fn counts_accepted_replacements_and_unique_affected_segments() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafka and Postgres").expect("valid srt");
        let review_cases = run_glossary_review(
            &transcript,
            &[
                glossary_entry("Apache Kafka", &["Kafka"]),
                glossary_entry("PostgreSQL", &["Postgres"]),
            ],
        )
        .expect("valid glossary");
        let mut ledger = ReviewLedger::new();
        for review_case in &review_cases {
            ledger
                .record_decision(
                    review_case,
                    transcript.revision_id(),
                    CorrectionDecision::AcceptAlternative {
                        alternative_index: 0,
                    },
                )
                .expect("valid decision");
        }

        let summary = collect(&transcript, &review_cases, &ledger, 2);

        assert_eq!(summary.outcomes.accepted_replacements_materialized, 2);
        assert_eq!(summary.outcomes.source_segments_affected, 1);
    }

    #[test]
    fn summary_uses_effective_last_decision_while_preserving_event_count() {
        let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafka").expect("valid srt");
        let review_cases =
            run_glossary_review(&transcript, &[glossary_entry("Apache Kafka", &["Kafka"])])
                .expect("valid glossary");
        let mut ledger = ReviewLedger::new();
        ledger
            .record_decision(
                &review_cases[0],
                transcript.revision_id(),
                CorrectionDecision::AcceptAlternative {
                    alternative_index: 0,
                },
            )
            .expect("valid accept decision");
        ledger
            .record_decision(
                &review_cases[0],
                transcript.revision_id(),
                CorrectionDecision::Reject,
            )
            .expect("valid reject decision");

        let summary = collect(&transcript, &review_cases, &ledger, 1);

        assert_eq!(summary.decisions.accepted_alternatives, 0);
        assert_eq!(summary.decisions.rejected, 1);
        assert_eq!(summary.decisions.total_recorded_events, 2);
        assert_eq!(summary.decisions.undecided_review_cases, 0);
        assert!(summary.accepted_replacements.is_empty());
        assert_eq!(summary.outcomes.accepted_replacements_materialized, 0);
        assert_eq!(summary.outcomes.source_segments_affected, 0);
    }

    #[test]
    fn aggregates_detector_provenance_by_id_and_version() {
        let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nA B").expect("valid srt");
        let first_anchor = transcript.anchor(0, 0, 1).expect("valid anchor");
        let second_anchor = transcript.anchor(0, 2, 3).expect("valid anchor");
        let entry = glossary_entry("Canonical", &["A"]);
        let make_candidate = |anchor, detector_id: &str, detector_version: &str| {
            CandidateSpan::new(
                DetectionKind::GlossaryAliasMatch,
                DetectorProvenance::new(detector_id, detector_version),
                anchor,
                Evidence::Glossary(GlossaryEvidence {
                    entry: entry.clone(),
                    matched_form: "A".to_string(),
                }),
                vec![CandidateAlternative::new("Canonical")],
            )
        };
        let review_cases = vec![
            ReviewCase::detector_raised(
                ReviewCaseId::local(0),
                make_candidate(first_anchor, "detector-a", "1.0"),
            ),
            ReviewCase::detector_raised(
                ReviewCaseId::local(1),
                make_candidate(second_anchor, "detector-b", "2.0"),
            ),
        ];
        let ledger = ReviewLedger::new();

        let summary = collect(&transcript, &review_cases, &ledger, 1);

        assert_eq!(
            summary.cases_by_detector,
            [
                DetectorCount {
                    detector_id: "detector-a".to_string(),
                    detector_version: "1.0".to_string(),
                    count: 1,
                },
                DetectorCount {
                    detector_id: "detector-b".to_string(),
                    detector_version: "2.0".to_string(),
                    count: 1,
                },
            ]
        );
        assert_eq!(
            summary.cases_by_detection_kind,
            [DetectionKindCount {
                kind: DetectionKind::GlossaryAliasMatch,
                count: 2,
            }]
        );
    }

    #[test]
    fn formats_duration_with_unbounded_hours_and_milliseconds() {
        assert_eq!(format_duration(3_723_004), "01:02:03.004");
        assert_eq!(format_duration(360_000_007), "100:00:00.007");
    }
}

pub mod analysis;
pub mod anchor;
pub mod calibration;
pub mod calibration_join;
pub mod candidate;
pub mod experimental_ranking;
pub mod experimental_retrieval;
pub mod persistence_evidence;
pub mod phonetic;
pub mod pipeline;
pub mod review;
pub mod reviewed_output;
pub mod run_manifest;
pub mod session_log;
pub mod session_summary;
pub mod session_terms;
pub mod srt;
pub mod transcript;

#[cfg(test)]
mod tests {
    use crate::analysis::{
        AlgorithmIdentity, AnalysisConfigurationIdentity, AnalysisRun,
        CanonicalDetectorSetIdentity, DetectorConfigIdentity, DetectorIdentity,
        SessionTermsIdentity,
    };
    use crate::anchor::AnchorError;
    use crate::candidate::{
        CANONICAL_SESSION_TERM_ALGORITHM, CANONICAL_SESSION_TERM_ANALYSIS_IDENTITY,
        CANONICAL_SESSION_TERM_DETECTOR_CONFIG, CANONICAL_SESSION_TERM_DETECTOR_SET,
        GLOSSARY_DETECTOR, OBSERVED_ERROR_FORM_DETECTOR, PHONETIC_DETECTOR,
    };
    use crate::candidate::{
        CandidateAlternative, CandidateSpan, DetectionError, DetectionKind, Evidence,
        GlossaryAliasEvidence, ObservedErrorFormEvidence, SessionTermEntry,
        detect_glossary_matches, detect_observed_error_form_matches,
    };
    use crate::pipeline::run_term_review;
    use crate::review::{
        CorrectionDecision, ReviewCase, ReviewCaseStatus, ReviewLedger, ReviewLedgerError,
    };
    use crate::srt::{ParseError, parse_srt};
    use crate::transcript::{
        DurationError, NormalizedSegment, Segment, Transcript, ValidationError, ValidationIssue,
    };

    fn segment(index: u32, start_ms: u64, end_ms: u64, text: &str) -> Segment {
        Segment {
            index,
            start_ms,
            end_ms,
            text: text.to_string(),
        }
    }

    #[test]
    fn valid_segment_returns_positive_duration() {
        let segment = segment(1, 100, 2600, "Valid segment");

        assert_eq!(segment.duration_ms(), Ok(2500));
    }

    #[test]
    fn zero_duration_segment_returns_zero() {
        let segment = segment(2, 500, 500, "Zero duration");

        assert_eq!(segment.duration_ms(), Ok(0));
    }

    #[test]
    fn reversed_range_returns_typed_duration_error() {
        let segment = segment(3, 3000, 2500, "Reversed range");

        assert_eq!(
            segment.duration_ms(),
            Err(DurationError::EndBeforeStart {
                start_ms: 3000,
                end_ms: 2500,
            })
        );
    }

    #[test]
    fn transcript_collects_expected_validation_issue() {
        let mut transcript = Transcript::new();
        transcript.add_segment(segment(1, 0, 2500, "Valid"));
        transcript.add_segment(segment(2, 4000, 3500, "Invalid"));

        assert_eq!(
            transcript.validation_issues(),
            vec![ValidationIssue {
                segment_position: 1,
                cue_index: 2,
                error: ValidationError::Duration(DurationError::EndBeforeStart {
                    start_ms: 4000,
                    end_ms: 3500,
                }),
            }]
        );
    }

    #[test]
    fn transcript_segments_preserve_insertion_order() {
        let first = segment(1, 0, 1000, "First");
        let second = segment(2, 1000, 2000, "Second");

        let mut transcript = Transcript::new();
        transcript.add_segment(first);
        transcript.add_segment(second);

        assert_eq!(
            transcript.segments(),
            &[
                segment(1, 0, 1000, "First"),
                segment(2, 1000, 2000, "Second"),
            ]
        );
    }

    #[test]
    fn parse_srt_parses_blocks_in_order() {
        let input = "1\n00:00:00,000 --> 00:00:02,500\n我們使用 Kafka 處理事件流\n\n\
                     2\n00:00:02,500 --> 00:00:05,000\nSecond segment";
        let transcript = parse_srt(input).expect("valid srt");
        assert_eq!(
            transcript.segments(),
            &[
                segment(1, 0, 2500, "我們使用 Kafka 處理事件流"),
                segment(2, 2500, 5000, "Second segment"),
            ]
        );
    }

    #[test]
    fn parse_srt_converts_timestamp_components_to_ms() {
        let input = "1\n01:02:03,004 --> 01:02:03,005\nx";
        let transcript = parse_srt(input).expect("valid srt");
        let parsed = &transcript.segments()[0];
        assert_eq!(parsed.start_ms, 3_723_004);
        assert_eq!(parsed.end_ms, 3_723_005);
    }

    #[test]
    fn parse_srt_accepts_strict_timestamp_grammar() {
        let input = "1\n01:02:03,004 --> 01:02:03,005\nx";
        let transcript = parse_srt(input).expect("valid srt");
        let parsed = &transcript.segments()[0];
        assert_eq!(parsed.start_ms, 3_723_004);
        assert_eq!(parsed.end_ms, 3_723_005);
    }

    #[test]
    fn parse_srt_accepts_hours_with_more_than_two_digits() {
        let input = "1\n100:00:00,000 --> 100:00:01,000\nx";
        let transcript = parse_srt(input).expect("valid srt");
        let parsed = &transcript.segments()[0];
        assert_eq!(parsed.start_ms, 360_000_000);
        assert_eq!(parsed.end_ms, 360_001_000);
    }

    #[test]
    fn parse_srt_rejects_one_digit_clock_fields() {
        for timing in [
            "1:2:3,000 --> 00:00:04,000",
            "01:2:03,000 --> 00:00:04,000",
            "01:02:3,000 --> 00:00:04,000",
        ] {
            let input = format!("1\n{timing}\nx");
            assert_eq!(
                parse_srt(&input),
                Err(ParseError::MalformedTiming {
                    block: 1,
                    found: timing.to_string(),
                })
            );
        }
    }

    #[test]
    fn parse_srt_rejects_minutes_out_of_range() {
        let timing = "00:60:00,000 --> 00:00:01,000";
        let input = format!("1\n{timing}\nx");

        assert_eq!(
            parse_srt(&input),
            Err(ParseError::MalformedTiming {
                block: 1,
                found: timing.to_string(),
            })
        );
    }

    #[test]
    fn parse_srt_rejects_seconds_out_of_range() {
        let timing = "00:00:60,000 --> 00:00:01,000";
        let input = format!("1\n{timing}\nx");

        assert_eq!(
            parse_srt(&input),
            Err(ParseError::MalformedTiming {
                block: 1,
                found: timing.to_string(),
            })
        );
    }

    #[test]
    fn parse_srt_rejects_dot_millisecond_separator() {
        let timing = "00:00:01.000 --> 00:00:02,000";
        let input = format!("1\n{timing}\nx");

        assert_eq!(
            parse_srt(&input),
            Err(ParseError::MalformedTiming {
                block: 1,
                found: timing.to_string(),
            })
        );
    }

    #[test]
    fn parse_srt_rejects_extra_clock_components() {
        let timing = "00:00:01:02,000 --> 00:00:03,000";
        let input = format!("1\n{timing}\nx");

        assert_eq!(
            parse_srt(&input),
            Err(ParseError::MalformedTiming {
                block: 1,
                found: timing.to_string(),
            })
        );
    }

    #[test]
    fn parse_srt_rejects_non_three_digit_milliseconds() {
        for timing in [
            "00:00:01,00 --> 00:00:02,000",
            "00:00:01,0000 --> 00:00:02,000",
        ] {
            let input = format!("1\n{timing}\nx");
            assert_eq!(
                parse_srt(&input),
                Err(ParseError::MalformedTiming {
                    block: 1,
                    found: timing.to_string(),
                })
            );
        }
    }

    #[test]
    fn parse_srt_joins_multiline_text_with_newline() {
        let input = "1\n00:00:00,000 --> 00:00:01,000\nline one\nline two";
        let transcript = parse_srt(input).expect("valid srt");
        assert_eq!(transcript.segments()[0].text, "line one\nline two");
    }

    #[test]
    fn parse_srt_handles_crlf_line_endings() {
        let input = "1\r\n00:00:00,000 --> 00:00:01,000\r\nhello\r\n";
        let transcript = parse_srt(input).expect("valid srt");
        assert_eq!(transcript.segments(), &[segment(1, 0, 1000, "hello")]);
    }

    #[test]
    fn parse_srt_preserves_empty_cue_text_as_validation_issue() {
        let input = "1\n00:00:00,000 --> 00:00:01,000\n\n\
                     2\n00:00:01,000 --> 00:00:02,000\nhello";

        let transcript = parse_srt(input).expect("syntactically valid srt");

        assert_eq!(
            transcript.segments(),
            &[segment(1, 0, 1000, ""), segment(2, 1000, 2000, "hello"),]
        );

        assert_eq!(
            transcript.validation_issues(),
            vec![ValidationIssue {
                segment_position: 0,
                cue_index: 1,
                error: ValidationError::EmptyText,
            }]
        );
    }

    #[test]
    fn parse_srt_rejects_malformed_index() {
        let input = "x\n00:00:00,000 --> 00:00:01,000\nhello";
        assert_eq!(
            parse_srt(input),
            Err(ParseError::MalformedIndex {
                block: 1,
                found: "x".to_string(),
            })
        );
    }

    #[test]
    fn parse_srt_rejects_block_missing_timing() {
        let input = "1";
        assert_eq!(
            parse_srt(input),
            Err(ParseError::MissingTiming { block: 1 })
        );
    }

    #[test]
    fn parse_srt_rejects_malformed_timing() {
        let input = "1\n00:00:00 --> bad\nhello";
        assert_eq!(
            parse_srt(input),
            Err(ParseError::MalformedTiming {
                block: 1,
                found: "00:00:00 --> bad".to_string(),
            })
        );
    }

    #[test]
    fn parse_srt_treats_reversed_timing_as_validation_issue_not_parse_error() {
        let input = "1\n00:00:03,000 --> 00:00:02,500\nreversed";
        let transcript = parse_srt(input).expect("reversed timing still parses");
        assert_eq!(
            transcript.validation_issues(),
            vec![ValidationIssue {
                segment_position: 0,
                cue_index: 1,
                error: ValidationError::Duration(DurationError::EndBeforeStart {
                    start_ms: 3000,
                    end_ms: 2500,
                }),
            }]
        );
    }

    #[test]
    fn parse_srt_treats_empty_text_as_validation_issue_not_parse_error() {
        let input = "1\n00:00:00,000 --> 00:00:01,000\n\n2\n00:00:01,000 --> 00:00:02,000\nok";
        let transcript = parse_srt(input).expect("empty text still parses");
        assert_eq!(
            transcript.validation_issues(),
            vec![ValidationIssue {
                segment_position: 0,
                cue_index: 1,
                error: ValidationError::EmptyText,
            }]
        );
    }

    #[test]
    fn parse_srt_treats_duplicate_indices_as_validation_issue_not_parse_error() {
        let input =
            "1\n00:00:00,000 --> 00:00:01,000\nfirst\n\n1\n00:00:01,000 --> 00:00:02,000\nsecond";
        let transcript = parse_srt(input).expect("duplicate indices still parse");
        assert_eq!(
            transcript.validation_issues(),
            vec![ValidationIssue {
                segment_position: 1,
                cue_index: 1,
                error: ValidationError::NonConsecutiveIndex {
                    previous: 1,
                    found: 1,
                },
            }]
        );
    }

    #[test]
    fn parse_srt_treats_index_gaps_as_validation_issue_not_parse_error() {
        let input =
            "1\n00:00:00,000 --> 00:00:01,000\nfirst\n\n3\n00:00:01,000 --> 00:00:02,000\nthird";
        let transcript = parse_srt(input).expect("index gaps still parse");
        assert_eq!(
            transcript.validation_issues(),
            vec![ValidationIssue {
                segment_position: 1,
                cue_index: 3,
                error: ValidationError::NonConsecutiveIndex {
                    previous: 1,
                    found: 3,
                },
            }]
        );
    }

    #[test]
    fn normalized_view_preserves_segment_order_and_source_mapping() {
        let transcript = parse_srt(
            "1\n00:00:00,000 --> 00:00:01,000\nfirst\n\n2\n00:00:01,000 --> 00:00:02,000\nsecond",
        )
        .expect("valid srt");

        assert_eq!(
            transcript.normalized_view().segments(),
            &[
                NormalizedSegment {
                    source_segment_index: 1,
                    normalized_text: "first".to_string(),
                },
                NormalizedSegment {
                    source_segment_index: 2,
                    normalized_text: "second".to_string(),
                },
            ]
        );
    }

    #[test]
    fn normalized_view_is_identity_preserving_for_now() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nline one\nline two").expect("valid srt");

        assert_eq!(
            transcript.normalized_view().segments(),
            &[NormalizedSegment {
                source_segment_index: 1,
                normalized_text: "line one\nline two".to_string(),
            }]
        );
    }

    #[test]
    fn normalized_view_does_not_mutate_source_transcript() {
        let transcript =
            parse_srt("1\r\n00:00:00,000 --> 00:00:01,000\r\nhello\r\n").expect("valid srt");

        let _normalized = transcript.normalized_view();

        assert_eq!(transcript.segments(), &[segment(1, 0, 1000, "hello")]);
    }

    #[test]
    fn revision_id_is_stable_for_equal_content() {
        let first = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nhello").expect("valid srt");
        let second = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nhello").expect("valid srt");

        assert_eq!(first.revision_id(), second.revision_id());
    }

    #[test]
    fn revision_id_differs_for_different_content() {
        let first = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nhello").expect("valid srt");
        let second = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nworld").expect("valid srt");

        assert_ne!(first.revision_id(), second.revision_id());
    }

    #[test]
    fn revision_id_differs_when_cue_index_changes() {
        let first = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nhello").expect("valid srt");
        let second = parse_srt("2\n00:00:00,000 --> 00:00:01,000\nhello").expect("valid srt");

        assert_ne!(first.revision_id(), second.revision_id());
    }

    #[test]
    fn revision_id_matches_for_crlf_and_lf_equivalent_input() {
        let lf = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nhello").expect("valid srt");
        let crlf = parse_srt("1\r\n00:00:00,000 --> 00:00:01,000\r\nhello\r\n").expect("valid srt");

        assert_eq!(lf.segments(), crlf.segments());
        assert_eq!(lf.revision_id(), crlf.revision_id());
    }

    #[test]
    fn revision_id_known_value_locks_canonical_encoding() {
        let mut transcript = Transcript::new();
        transcript.add_segment(segment(1, 1000, 2000, "Kafka"));

        assert_eq!(
            transcript.revision_id().to_tagged_string(),
            "rev:sha256-v1:eafc2fd34c2d5e9f79729e52c59704579bd8a1b9d047ec680e09aed76bc4c976"
        );
    }

    #[test]
    fn anchor_resolves_to_expected_substring() {
        let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\n我們A").expect("valid srt");

        let anchor = transcript.anchor(0, 0, 3).expect("valid anchor");

        assert_eq!(transcript.resolve(&anchor), Some("我"));
    }

    #[test]
    fn validation_issue_and_anchor_share_segment_position_plane() {
        let transcript = parse_srt(
            "1\n00:00:00,000 --> 00:00:01,000\nfirst\n\n2\n00:00:03,000 --> 00:00:02,000\nsecond",
        )
        .expect("reversed timing still parses");
        let anchor = transcript.anchor(1, 0, 6).expect("valid anchor");

        assert_eq!(anchor.segment_position, 1);
        assert_eq!(
            transcript.validation_issues(),
            vec![ValidationIssue {
                segment_position: 1,
                cue_index: 2,
                error: ValidationError::Duration(DurationError::EndBeforeStart {
                    start_ms: 3000,
                    end_ms: 2000,
                }),
            }]
        );
    }

    #[test]
    fn anchor_rejects_range_crossing_char_boundary() {
        let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\n我們A").expect("valid srt");

        assert_eq!(
            transcript.anchor(0, 0, 2),
            Err(AnchorError::NotCharBoundary { byte: 2 })
        );
    }

    #[test]
    fn anchor_rejects_empty_range() {
        let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nhello").expect("valid srt");

        assert_eq!(
            transcript.anchor(0, 2, 2),
            Err(AnchorError::EmptyOrInvertedRange {
                start_byte: 2,
                end_byte: 2,
            })
        );
    }

    #[test]
    fn anchor_rejects_out_of_bounds_range() {
        let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nhello").expect("valid srt");

        assert_eq!(
            transcript.anchor(0, 0, 99),
            Err(AnchorError::RangeOutOfBounds {
                end_byte: 99,
                text_len: 5,
            })
        );
    }

    #[test]
    fn anchor_rejects_unknown_segment() {
        let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nhello").expect("valid srt");

        assert_eq!(
            transcript.anchor(9, 0, 1),
            Err(AnchorError::UnknownSegment {
                segment_position: 9
            })
        );
    }

    #[test]
    fn resolve_rejects_anchor_from_a_different_revision() {
        let original = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nhello").expect("valid srt");
        let anchor = original.anchor(0, 0, 5).expect("valid anchor");

        let other = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nworld").expect("valid srt");

        assert_eq!(other.resolve(&anchor), None);
    }

    #[test]
    fn resolve_rejects_anchor_from_revision_before_add_segment() {
        let mut transcript = Transcript::new();
        transcript.add_segment(segment(1, 0, 1000, "hello"));
        let anchor = transcript.anchor(0, 0, 5).expect("valid anchor");

        transcript.add_segment(segment(2, 1000, 2000, "world"));

        assert_eq!(transcript.resolve(&anchor), None);
    }

    fn glossary_entry(canonical_term: &str, aliases: &[&str]) -> SessionTermEntry {
        SessionTermEntry::new(
            canonical_term,
            aliases.iter().map(|alias| alias.to_string()).collect(),
            Vec::new(),
        )
    }

    fn observed_error_entry(
        canonical_term: &str,
        observed_error_forms: &[&str],
    ) -> SessionTermEntry {
        SessionTermEntry::new(
            canonical_term,
            Vec::new(),
            observed_error_forms
                .iter()
                .map(|form| form.to_string())
                .collect(),
        )
    }

    fn run_glossary_review(
        transcript: &Transcript,
        entries: &[SessionTermEntry],
    ) -> Result<Vec<ReviewCase>, DetectionError> {
        run_term_review(transcript, entries)
    }

    fn canonical_analysis_run(
        transcript: &Transcript,
        entries: &[SessionTermEntry],
    ) -> AnalysisRun {
        AnalysisRun::for_canonical_session_terms(transcript, entries)
    }

    #[test]
    fn detect_glossary_matches_finds_exact_alias_occurrence() {
        let transcript = parse_srt("1\n00:00:00,000 --> 00:00:02,500\n我們使用 Kafka 處理事件流")
            .expect("valid srt");
        let glossary = vec![glossary_entry("Apache Kafka", &["Kafka"])];
        let run = canonical_analysis_run(&transcript, &glossary);

        let spans = detect_glossary_matches(&run, &transcript, &glossary)
            .expect("glossary has no ambiguous aliases");

        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].kind(), DetectionKind::GlossaryAliasMatch);
        assert_eq!(spans[0].provenance().detector_id(), "glossary-alias-match");
    }

    #[test]
    fn detect_glossary_matches_ignores_non_matching_text() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nhello world").expect("valid srt");
        let glossary = vec![glossary_entry("Apache Kafka", &["Kafka"])];
        let run = canonical_analysis_run(&transcript, &glossary);

        let spans = detect_glossary_matches(&run, &transcript, &glossary)
            .expect("glossary has no ambiguous aliases");

        assert!(spans.is_empty());
    }

    #[test]
    fn detect_glossary_matches_finds_occurrences_across_segments() {
        let transcript = parse_srt(
            "1\n00:00:00,000 --> 00:00:01,000\nfirst Kafka mention\n\n2\n00:00:01,000 --> 00:00:02,000\nsecond Kafka mention",
        )
        .expect("valid srt");
        let glossary = vec![glossary_entry("Apache Kafka", &["Kafka"])];
        let run = canonical_analysis_run(&transcript, &glossary);

        let spans = detect_glossary_matches(&run, &transcript, &glossary)
            .expect("glossary has no ambiguous aliases");

        assert_eq!(spans.len(), 2);
    }

    #[test]
    fn detect_glossary_matches_produces_typed_glossary_evidence() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nusing Kafka here").expect("valid srt");
        let glossary = vec![glossary_entry("Apache Kafka", &["Kafka"])];
        let run = canonical_analysis_run(&transcript, &glossary);

        let spans = detect_glossary_matches(&run, &transcript, &glossary)
            .expect("glossary has no ambiguous aliases");

        match spans[0].evidence() {
            Evidence::GlossaryAlias(GlossaryAliasEvidence {
                entry,
                matched_form,
            }) => {
                assert_eq!(entry.canonical_term, "Apache Kafka");
                assert_eq!(matched_form, "Kafka");
            }
            Evidence::ObservedErrorForm(_) => panic!("expected glossary-alias evidence"),
            Evidence::PhoneticSimilarity(_) => panic!("expected glossary-alias evidence"),
        }
    }

    #[test]
    fn detect_glossary_matches_anchor_resolves_to_matched_text() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nusing Kafka here").expect("valid srt");
        let glossary = vec![glossary_entry("Apache Kafka", &["Kafka"])];
        let run = canonical_analysis_run(&transcript, &glossary);

        let spans = detect_glossary_matches(&run, &transcript, &glossary)
            .expect("glossary has no ambiguous aliases");

        assert_eq!(transcript.resolve(spans[0].anchor()), Some("Kafka"));
    }

    #[test]
    fn candidate_key_is_stable_for_equal_detector_kind_and_anchor() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nusing Kafka here").expect("valid srt");
        let glossary = vec![glossary_entry("Apache Kafka", &["Kafka"])];
        let run = canonical_analysis_run(&transcript, &glossary);

        let first = detect_glossary_matches(&run, &transcript, &glossary)
            .expect("glossary has no ambiguous aliases");
        let second = detect_glossary_matches(&run, &transcript, &glossary)
            .expect("glossary has no ambiguous aliases");

        assert_eq!(first[0].key(), second[0].key());
    }

    #[test]
    fn candidate_key_differs_for_different_anchor() {
        let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafka appears twice: Kafka")
            .expect("valid srt");
        let glossary = vec![glossary_entry("Apache Kafka", &["Kafka"])];
        let run = canonical_analysis_run(&transcript, &glossary);

        let spans = detect_glossary_matches(&run, &transcript, &glossary)
            .expect("glossary has no ambiguous aliases");

        assert_eq!(spans.len(), 2);
        assert_ne!(spans[0].key(), spans[1].key());
    }

    #[test]
    fn candidate_key_differs_for_different_detection_kind() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nusing Kafka here").expect("valid srt");
        let glossary = vec![glossary_entry("Apache Kafka", &["Kafka"])];
        let run = canonical_analysis_run(&transcript, &glossary);

        let spans = detect_glossary_matches(&run, &transcript, &glossary)
            .expect("glossary has no ambiguous aliases");
        let anchor = *spans[0].anchor();
        let provenance = spans[0].provenance().clone();
        let evidence = spans[0].evidence().clone();
        let alternatives = spans[0].alternatives().to_vec();

        let alternate_kind_span = CandidateSpan::new(
            DetectionKind::RepeatedPhrase,
            provenance,
            anchor,
            evidence,
            alternatives,
        );

        assert_ne!(spans[0].key(), alternate_kind_span.key());
    }

    #[test]
    fn detect_glossary_matches_rejects_empty_alias() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nanything at all").expect("valid srt");
        let glossary = vec![glossary_entry("Empty Alias Entry", &[""])];
        let run = canonical_analysis_run(&transcript, &glossary);

        let result = detect_glossary_matches(&run, &transcript, &glossary);

        assert_eq!(
            result,
            Err(DetectionError::EmptyAlias {
                canonical_term: "Empty Alias Entry".to_string(),
            })
        );
    }

    #[test]
    fn detect_glossary_matches_rejects_ambiguous_alias_configuration() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nusing Kafka here").expect("valid srt");
        let glossary = vec![
            glossary_entry("Apache Kafka", &["Kafka"]),
            glossary_entry("Kafka the author", &["Kafka"]),
        ];
        let run = canonical_analysis_run(&transcript, &glossary);

        let result = detect_glossary_matches(&run, &transcript, &glossary);

        assert_eq!(
            result,
            Err(DetectionError::DuplicateSourceForm {
                source_form: "Kafka".to_string(),
            })
        );
    }

    #[test]
    fn detect_glossary_matches_rejects_mismatched_analysis_run_revision() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nusing Kafka here").expect("valid srt");
        let other = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nusing Kafka elsewhere")
            .expect("valid srt");
        let glossary = vec![glossary_entry("Apache Kafka", &["Kafka"])];
        let run = canonical_analysis_run(&other, &glossary);

        let result = detect_glossary_matches(&run, &transcript, &glossary);

        assert_eq!(
            result,
            Err(DetectionError::RevisionMismatch {
                run_revision: other.revision_id(),
                transcript_revision: transcript.revision_id(),
            })
        );
    }

    #[test]
    fn exact_detector_rejects_mismatched_session_terms_identity() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nusing Kafka here").expect("valid srt");
        let run_entries = [glossary_entry("Apache Kafka", &["Kafka"])];
        let provided_entries = [glossary_entry("Kafka", &["Kafka"])];
        let run = canonical_analysis_run(&transcript, &run_entries);

        let result = detect_glossary_matches(&run, &transcript, &provided_entries);

        assert_eq!(
            result,
            Err(DetectionError::SessionTermsIdentityMismatch {
                run_identity: SessionTermsIdentity::from_entries(&run_entries),
                provided_identity: SessionTermsIdentity::from_entries(&provided_entries),
            })
        );
    }

    #[test]
    fn exact_detector_rejects_mismatched_detector_set_config_and_algorithm_identity() {
        const OTHER_DETECTOR: DetectorIdentity = DetectorIdentity::new("other-detector", "0.1.0");
        const OTHER_DETECTORS: &[DetectorIdentity] = &[OTHER_DETECTOR];
        const OTHER_DETECTOR_SET: CanonicalDetectorSetIdentity =
            CanonicalDetectorSetIdentity::new(OTHER_DETECTORS);

        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nusing Kafka here").expect("valid srt");
        let entries = [glossary_entry("Apache Kafka", &["Kafka"])];
        let required = CANONICAL_SESSION_TERM_ANALYSIS_IDENTITY;
        let configurations = [
            AnalysisConfigurationIdentity::new(
                OTHER_DETECTOR_SET,
                CANONICAL_SESSION_TERM_DETECTOR_CONFIG,
                CANONICAL_SESSION_TERM_ALGORITHM,
            ),
            AnalysisConfigurationIdentity::new(
                CANONICAL_SESSION_TERM_DETECTOR_SET,
                DetectorConfigIdentity::new("other-config", "0.1.0"),
                CANONICAL_SESSION_TERM_ALGORITHM,
            ),
            AnalysisConfigurationIdentity::new(
                CANONICAL_SESSION_TERM_DETECTOR_SET,
                CANONICAL_SESSION_TERM_DETECTOR_CONFIG,
                AlgorithmIdentity::new("other-algorithm", "0.1.0"),
            ),
        ];

        let detector_set_result = detect_glossary_matches(
            &AnalysisRun::new(&transcript, &entries, configurations[0]),
            &transcript,
            &entries,
        );
        assert_eq!(
            detector_set_result,
            Err(DetectionError::DetectorSetIdentityMismatch {
                run_identity: configurations[0].detector_set(),
                required_identity: required.detector_set(),
            })
        );

        let config_result = detect_glossary_matches(
            &AnalysisRun::new(&transcript, &entries, configurations[1]),
            &transcript,
            &entries,
        );
        assert_eq!(
            config_result,
            Err(DetectionError::DetectorConfigIdentityMismatch {
                run_identity: configurations[1].detector_config(),
                required_identity: required.detector_config(),
            })
        );

        let algorithm_result = detect_glossary_matches(
            &AnalysisRun::new(&transcript, &entries, configurations[2]),
            &transcript,
            &entries,
        );
        assert_eq!(
            algorithm_result,
            Err(DetectionError::AlgorithmIdentityMismatch {
                run_identity: configurations[2].algorithm(),
                required_identity: required.algorithm(),
            })
        );
    }

    #[test]
    fn observed_error_form_match_has_distinct_typed_evidence_and_identity() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nPostgre SQL").expect("valid srt");
        let entries = vec![observed_error_entry("PostgreSQL", &["Postgre SQL"])];
        let run = canonical_analysis_run(&transcript, &entries);
        let alias_entries = [glossary_entry("PostgreSQL", &["Postgre SQL"])];
        let alias_run = canonical_analysis_run(&transcript, &alias_entries);

        let spans = detect_observed_error_form_matches(&run, &transcript, &entries)
            .expect("valid observed error configuration");
        let alias_spans = detect_glossary_matches(&alias_run, &transcript, &alias_entries)
            .expect("valid alias configuration");

        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].kind(), DetectionKind::GlossaryAliasMatch);
        assert_eq!(
            spans[0].provenance().detector_id(),
            "observed-error-form-match"
        );
        assert_eq!(spans[0].key().kind(), DetectionKind::GlossaryAliasMatch);
        assert_eq!(spans[0].anchor(), alias_spans[0].anchor());
        assert_ne!(spans[0].key(), alias_spans[0].key());
        assert_eq!(
            spans[0].alternatives(),
            &[CandidateAlternative::new("PostgreSQL")]
        );
        match spans[0].evidence() {
            Evidence::ObservedErrorForm(ObservedErrorFormEvidence {
                entry,
                matched_form,
            }) => {
                assert_eq!(entry.canonical_term, "PostgreSQL");
                assert_eq!(matched_form, "Postgre SQL");
            }
            Evidence::GlossaryAlias(_) => panic!("expected observed-error-form evidence"),
            Evidence::PhoneticSimilarity(_) => panic!("expected observed-error-form evidence"),
        }
    }

    #[test]
    fn observed_error_matching_is_exact_case_sensitive_and_ignores_canonical_term() {
        let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nPostgre sql and PostgreSQL")
            .expect("valid srt");
        let entries = vec![observed_error_entry("PostgreSQL", &["Postgre SQL"])];
        let run = canonical_analysis_run(&transcript, &entries);

        let spans = detect_observed_error_form_matches(&run, &transcript, &entries)
            .expect("valid observed error configuration");

        assert!(spans.is_empty());
    }

    #[test]
    fn observed_error_detector_rejects_mismatched_analysis_run_revision() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nPostgre SQL").expect("valid srt");
        let other = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nOther").expect("valid srt");
        let entries = [observed_error_entry("PostgreSQL", &["Postgre SQL"])];
        let run = canonical_analysis_run(&other, &entries);

        let result = detect_observed_error_form_matches(&run, &transcript, &entries);

        assert_eq!(
            result,
            Err(DetectionError::RevisionMismatch {
                run_revision: other.revision_id(),
                transcript_revision: transcript.revision_id(),
            })
        );
    }

    #[test]
    fn combined_term_pipeline_returns_both_detectors_in_source_order() {
        let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nPostgre SQL then Postgres")
            .expect("valid srt");
        let entries = vec![SessionTermEntry::new(
            "PostgreSQL",
            vec!["Postgres".to_string()],
            vec!["Postgre SQL".to_string()],
        )];

        let review_cases =
            run_term_review(&transcript, &entries).expect("valid session-term configuration");
        let repeated =
            run_term_review(&transcript, &entries).expect("valid session-term configuration");

        assert_eq!(review_cases.len(), 3);
        assert_eq!(
            review_cases
                .iter()
                .map(|case| case.candidate_span().provenance().detector_id())
                .collect::<Vec<_>>(),
            repeated
                .iter()
                .map(|case| case.candidate_span().provenance().detector_id())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            review_cases[0].candidate_span().kind(),
            DetectionKind::PhoneticSimilarity
        );
        assert_eq!(
            review_cases[1].candidate_span().kind(),
            DetectionKind::GlossaryAliasMatch
        );
        assert_eq!(
            review_cases[2].candidate_span().kind(),
            DetectionKind::GlossaryAliasMatch
        );
        assert_eq!(
            review_cases[0].candidate_span().provenance().detector_id(),
            "ascii-latin-phonetic-similarity"
        );
        assert_eq!(
            review_cases[1].candidate_span().provenance().detector_id(),
            "observed-error-form-match"
        );
        assert_eq!(
            review_cases[2].candidate_span().provenance().detector_id(),
            "glossary-alias-match"
        );
        assert_ne!(
            review_cases[0].candidate_span().key(),
            review_cases[1].candidate_span().key()
        );
        assert_ne!(
            review_cases[1].candidate_span().key(),
            review_cases[2].candidate_span().key()
        );
    }

    #[test]
    fn canonical_review_cases_are_independent_of_session_term_source_order() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafka then Postgre SQL then Postgres")
                .expect("valid srt");
        let first = [
            SessionTermEntry::new("Apache Kafka", vec!["Kafka".to_string()], Vec::new()),
            SessionTermEntry::new(
                "PostgreSQL",
                vec!["Postgres".to_string()],
                vec!["Postgre SQL".to_string()],
            ),
        ];
        let second = [first[1].clone(), first[0].clone()];

        let first_cases =
            run_term_review(&transcript, &first).expect("valid session-term configuration");
        let second_cases =
            run_term_review(&transcript, &second).expect("valid session-term configuration");

        assert_eq!(first_cases, second_cases);
        assert_eq!(
            first_cases
                .iter()
                .map(|review_case| review_case.id().local_index())
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );
    }

    #[test]
    fn combined_term_pipeline_rejects_cross_kind_duplicate_source_form() {
        let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafka").expect("valid srt");
        let entries = vec![SessionTermEntry::new(
            "Apache Kafka",
            vec!["Kafka".to_string()],
            vec!["Kafka".to_string()],
        )];

        let result = run_term_review(&transcript, &entries);

        assert_eq!(
            result,
            Err(DetectionError::DuplicateSourceForm {
                source_form: "Kafka".to_string(),
            })
        );
    }

    #[test]
    fn review_case_wraps_exactly_one_candidate_span() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nusing Kafka here").expect("valid srt");
        let glossary = vec![glossary_entry("Apache Kafka", &["Kafka"])];
        let run = canonical_analysis_run(&transcript, &glossary);

        let mut spans = detect_glossary_matches(&run, &transcript, &glossary)
            .expect("glossary has no ambiguous aliases");
        let candidate = spans.remove(0);
        let expected = candidate.clone();

        let mut review_cases = ReviewCase::from_detector_candidates(vec![candidate]);
        let review_case = review_cases.remove(0);

        assert_eq!(review_case.candidate_span(), &expected);
    }

    #[test]
    fn review_cases_preserve_one_to_one_mapping_and_order() {
        let transcript = parse_srt(
            "1\n00:00:00,000 --> 00:00:01,000\nfirst Kafka mention\n\n2\n00:00:01,000 --> 00:00:02,000\nsecond Kafka mention",
        )
        .expect("valid srt");
        let glossary = vec![glossary_entry("Apache Kafka", &["Kafka"])];
        let run = canonical_analysis_run(&transcript, &glossary);

        let candidates = detect_glossary_matches(&run, &transcript, &glossary)
            .expect("glossary has no ambiguous aliases");
        let expected: Vec<CandidateSpan> = candidates.clone();

        let review_cases = ReviewCase::from_detector_candidates(candidates);

        assert_eq!(review_cases.len(), expected.len());
        for (review_case, expected_candidate) in review_cases.iter().zip(expected.iter()) {
            assert_eq!(review_case.candidate_span(), expected_candidate);
        }
    }

    #[test]
    fn review_case_ids_are_assigned_deterministically_for_detector_cases() {
        let transcript = parse_srt(
            "1\n00:00:00,000 --> 00:00:01,000\nfirst Kafka mention\n\n2\n00:00:01,000 --> 00:00:02,000\nsecond Kafka mention",
        )
        .expect("valid srt");
        let glossary = vec![glossary_entry("Apache Kafka", &["Kafka"])];
        let run = canonical_analysis_run(&transcript, &glossary);

        let candidates = detect_glossary_matches(&run, &transcript, &glossary)
            .expect("glossary has no ambiguous aliases");
        let review_cases = ReviewCase::from_detector_candidates(candidates);

        assert_eq!(review_cases[0].id().local_index(), 0);
        assert_eq!(review_cases[1].id().local_index(), 1);
    }

    #[test]
    fn new_review_case_has_undecided_status() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nusing Kafka here").expect("valid srt");
        let review_cases =
            run_glossary_review(&transcript, &[glossary_entry("Apache Kafka", &["Kafka"])])
                .expect("glossary has no ambiguous aliases");
        let ledger = ReviewLedger::new();

        assert_eq!(
            ledger.status_for(review_cases[0].id()),
            ReviewCaseStatus::Undecided
        );
    }

    #[test]
    fn recording_reject_decides_that_case() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nusing Kafka here").expect("valid srt");
        let review_cases =
            run_glossary_review(&transcript, &[glossary_entry("Apache Kafka", &["Kafka"])])
                .expect("glossary has no ambiguous aliases");
        let mut ledger = ReviewLedger::new();

        ledger
            .record_decision(
                &review_cases[0],
                transcript.revision_id(),
                CorrectionDecision::Reject,
            )
            .expect("reject is valid");

        assert_eq!(
            ledger.status_for(review_cases[0].id()),
            ReviewCaseStatus::Decided {
                observed_revision: transcript.revision_id(),
                decision: CorrectionDecision::Reject,
            }
        );
    }

    #[test]
    fn later_decision_supersedes_earlier_decision_for_same_case() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nusing Kafka here").expect("valid srt");
        let review_cases =
            run_glossary_review(&transcript, &[glossary_entry("Apache Kafka", &["Kafka"])])
                .expect("glossary has no ambiguous aliases");
        let mut ledger = ReviewLedger::new();

        ledger
            .record_decision(
                &review_cases[0],
                transcript.revision_id(),
                CorrectionDecision::Reject,
            )
            .expect("reject is valid");
        ledger
            .record_decision(
                &review_cases[0],
                transcript.revision_id(),
                CorrectionDecision::Defer,
            )
            .expect("defer is valid");

        assert_eq!(ledger.events().len(), 2);
        assert_eq!(
            ledger.status_for(review_cases[0].id()),
            ReviewCaseStatus::Decided {
                observed_revision: transcript.revision_id(),
                decision: CorrectionDecision::Defer,
            }
        );
    }

    #[test]
    fn decisions_for_one_case_do_not_affect_another_case() {
        let transcript = parse_srt(
            "1\n00:00:00,000 --> 00:00:01,000\nfirst Kafka mention\n\n2\n00:00:01,000 --> 00:00:02,000\nsecond Kafka mention",
        )
        .expect("valid srt");
        let review_cases =
            run_glossary_review(&transcript, &[glossary_entry("Apache Kafka", &["Kafka"])])
                .expect("glossary has no ambiguous aliases");
        let mut ledger = ReviewLedger::new();

        ledger
            .record_decision(
                &review_cases[0],
                transcript.revision_id(),
                CorrectionDecision::Reject,
            )
            .expect("reject is valid");

        assert_eq!(
            ledger.status_for(review_cases[0].id()),
            ReviewCaseStatus::Decided {
                observed_revision: transcript.revision_id(),
                decision: CorrectionDecision::Reject,
            }
        );
        assert_eq!(
            ledger.status_for(review_cases[1].id()),
            ReviewCaseStatus::Undecided
        );
    }

    #[test]
    fn recorded_status_preserves_observed_revision() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nusing Kafka here").expect("valid srt");
        let observed_revision = transcript.revision_id();
        let review_cases =
            run_glossary_review(&transcript, &[glossary_entry("Apache Kafka", &["Kafka"])])
                .expect("glossary has no ambiguous aliases");
        let mut ledger = ReviewLedger::new();

        ledger
            .record_decision(
                &review_cases[0],
                observed_revision,
                CorrectionDecision::NeedsManualCorrection,
            )
            .expect("needs-manual-correction is valid");

        assert_eq!(
            ledger.status_for(review_cases[0].id()),
            ReviewCaseStatus::Decided {
                observed_revision,
                decision: CorrectionDecision::NeedsManualCorrection,
            }
        );
    }

    #[test]
    fn accept_alternative_records_decision_when_index_exists() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nusing Kafka here").expect("valid srt");
        let review_cases =
            run_glossary_review(&transcript, &[glossary_entry("Apache Kafka", &["Kafka"])])
                .expect("glossary has no ambiguous aliases");
        let mut ledger = ReviewLedger::new();

        ledger
            .record_decision(
                &review_cases[0],
                transcript.revision_id(),
                CorrectionDecision::AcceptAlternative {
                    alternative_index: 0,
                },
            )
            .expect("alternative index exists");

        assert_eq!(
            ledger.status_for(review_cases[0].id()),
            ReviewCaseStatus::Decided {
                observed_revision: transcript.revision_id(),
                decision: CorrectionDecision::AcceptAlternative {
                    alternative_index: 0,
                },
            }
        );
    }

    #[test]
    fn accept_alternative_rejects_out_of_range_alternative_index() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nusing Kafka here").expect("valid srt");
        let review_cases =
            run_glossary_review(&transcript, &[glossary_entry("Apache Kafka", &["Kafka"])])
                .expect("glossary has no ambiguous aliases");
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
        assert!(ledger.events().is_empty());
        assert_eq!(
            ledger.status_for(review_cases[0].id()),
            ReviewCaseStatus::Undecided
        );
    }

    #[test]
    fn review_case_stores_no_mutable_status() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nusing Kafka here").expect("valid srt");
        let review_cases =
            run_glossary_review(&transcript, &[glossary_entry("Apache Kafka", &["Kafka"])])
                .expect("glossary has no ambiguous aliases");
        let mut decided_ledger = ReviewLedger::new();

        decided_ledger
            .record_decision(
                &review_cases[0],
                transcript.revision_id(),
                CorrectionDecision::Reject,
            )
            .expect("reject is valid");

        let fresh_ledger = ReviewLedger::new();
        assert_eq!(
            decided_ledger.status_for(review_cases[0].id()),
            ReviewCaseStatus::Decided {
                observed_revision: transcript.revision_id(),
                decision: CorrectionDecision::Reject,
            }
        );
        assert_eq!(
            fresh_ledger.status_for(review_cases[0].id()),
            ReviewCaseStatus::Undecided
        );
    }

    #[test]
    fn detect_glossary_matches_proposes_canonical_term_as_non_binding_alternative() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nusing Kafka here").expect("valid srt");
        let glossary = vec![glossary_entry("Apache Kafka", &["Kafka"])];
        let run = canonical_analysis_run(&transcript, &glossary);

        let spans = detect_glossary_matches(&run, &transcript, &glossary)
            .expect("glossary has no ambiguous aliases");

        assert_eq!(
            spans[0].alternatives(),
            &[CandidateAlternative::new("Apache Kafka")]
        );
    }

    #[test]
    fn detect_glossary_matches_produces_no_candidate_for_canonical_form_occurrence() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nusing Kafka here").expect("valid srt");
        let glossary = vec![glossary_entry("Kafka", &["Kafka"])];
        let run = canonical_analysis_run(&transcript, &glossary);

        let spans = detect_glossary_matches(&run, &transcript, &glossary)
            .expect("glossary has no ambiguous aliases");

        assert!(spans.is_empty());
    }

    #[test]
    fn candidate_span_may_carry_zero_alternatives() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nusing Kafka here").expect("valid srt");
        let glossary = vec![glossary_entry("Apache Kafka", &["Kafka"])];
        let run = canonical_analysis_run(&transcript, &glossary);

        let spans = detect_glossary_matches(&run, &transcript, &glossary)
            .expect("glossary has no ambiguous aliases");
        let anchor = *spans[0].anchor();
        let provenance = spans[0].provenance().clone();
        let evidence = spans[0].evidence().clone();

        let span_without_alternatives = CandidateSpan::new(
            DetectionKind::GlossaryAliasMatch,
            provenance,
            anchor,
            evidence,
            Vec::new(),
        );

        assert!(span_without_alternatives.alternatives().is_empty());
    }

    #[test]
    fn run_glossary_review_wraps_matches_as_review_cases() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nusing Kafka here").expect("valid srt");
        let glossary = vec![glossary_entry("Apache Kafka", &["Kafka"])];

        let review_cases =
            run_glossary_review(&transcript, &glossary).expect("glossary has no ambiguous aliases");

        assert_eq!(review_cases.len(), 1);
        let candidate = review_cases[0].candidate_span();
        assert_eq!(candidate.kind(), DetectionKind::GlossaryAliasMatch);
        assert_eq!(
            candidate.alternatives(),
            &[CandidateAlternative::new("Apache Kafka")]
        );
    }

    #[test]
    fn run_glossary_review_returns_empty_vec_when_nothing_matches() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nhello world").expect("valid srt");
        let glossary = vec![glossary_entry("Apache Kafka", &["Kafka"])];

        let review_cases =
            run_glossary_review(&transcript, &glossary).expect("glossary has no ambiguous aliases");

        assert!(review_cases.is_empty());
    }

    #[test]
    fn run_glossary_review_returns_empty_vec_for_empty_glossary() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nusing Kafka here").expect("valid srt");

        let review_cases =
            run_glossary_review(&transcript, &[]).expect("empty glossary is not a config error");

        assert!(review_cases.is_empty());
    }

    #[test]
    fn detector_provenance_uses_detector_identities_bound_in_active_set() {
        assert!(
            CANONICAL_SESSION_TERM_DETECTOR_SET
                .detectors()
                .iter()
                .any(|identity| identity.id() == GLOSSARY_DETECTOR.id()
                    && identity.version() == GLOSSARY_DETECTOR.version())
        );
        assert!(CANONICAL_SESSION_TERM_DETECTOR_SET.detectors().iter().any(
            |identity| identity.id() == OBSERVED_ERROR_FORM_DETECTOR.id()
                && identity.version() == OBSERVED_ERROR_FORM_DETECTOR.version()
        ));
        assert!(
            CANONICAL_SESSION_TERM_DETECTOR_SET
                .detectors()
                .iter()
                .any(|identity| identity.id() == PHONETIC_DETECTOR.id()
                    && identity.version() == PHONETIC_DETECTOR.version())
        );

        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nusing Kafka here").expect("valid srt");
        let glossary = vec![glossary_entry("Apache Kafka", &["Kafka"])];
        let run = canonical_analysis_run(&transcript, &glossary);

        let glossary_spans = detect_glossary_matches(&run, &transcript, &glossary)
            .expect("glossary has no ambiguous aliases");
        assert_eq!(
            glossary_spans[0].provenance().detector_id(),
            GLOSSARY_DETECTOR.id()
        );
        assert_eq!(
            glossary_spans[0].provenance().detector_version(),
            GLOSSARY_DETECTOR.version()
        );

        let observed_entries = vec![observed_error_entry("PostgreSQL", &["Postgre SQL"])];
        let observed_transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nPostgre SQL").expect("valid srt");
        let observed_run = canonical_analysis_run(&observed_transcript, &observed_entries);
        let observed_spans = detect_observed_error_form_matches(
            &observed_run,
            &observed_transcript,
            &observed_entries,
        )
        .expect("valid observed error configuration");
        assert_eq!(
            observed_spans[0].provenance().detector_id(),
            OBSERVED_ERROR_FORM_DETECTOR.id()
        );
        assert_eq!(
            observed_spans[0].provenance().detector_version(),
            OBSERVED_ERROR_FORM_DETECTOR.version()
        );
    }

    #[test]
    fn direct_detector_finding_order_follows_source_term_order() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafka then Postgres").expect("valid srt");
        let first_order = [
            glossary_entry("Apache Kafka", &["Kafka"]),
            glossary_entry("PostgreSQL", &["Postgres"]),
        ];
        let second_order = [first_order[1].clone(), first_order[0].clone()];
        let run_first = canonical_analysis_run(&transcript, &first_order);
        let run_second = canonical_analysis_run(&transcript, &second_order);

        let first_spans = detect_glossary_matches(&run_first, &transcript, &first_order)
            .expect("glossary has no ambiguous aliases");
        let second_spans = detect_glossary_matches(&run_second, &transcript, &second_order)
            .expect("glossary has no ambiguous aliases");

        assert_eq!(first_spans.len(), 2);
        assert_eq!(second_spans.len(), 2);
        assert_ne!(first_spans[0].key(), second_spans[0].key());
        assert_eq!(transcript.resolve(first_spans[0].anchor()), Some("Kafka"));
        assert_eq!(
            transcript.resolve(second_spans[0].anchor()),
            Some("Postgres")
        );
    }

    #[test]
    fn observed_error_detector_rejects_mismatched_session_terms_identity() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nPostgre SQL").expect("valid srt");
        let run_entries = [observed_error_entry("PostgreSQL", &["Postgre SQL"])];
        let provided_entries = [observed_error_entry("Postgres", &["Postgre SQL"])];
        let run = canonical_analysis_run(&transcript, &run_entries);

        let result = detect_observed_error_form_matches(&run, &transcript, &provided_entries);

        assert_eq!(
            result,
            Err(DetectionError::SessionTermsIdentityMismatch {
                run_identity: SessionTermsIdentity::from_entries(&run_entries),
                provided_identity: SessionTermsIdentity::from_entries(&provided_entries),
            })
        );
    }

    #[test]
    fn authoritative_pipeline_binds_owned_canonical_analysis_profile() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nusing Kafka here").expect("valid srt");
        let entries = vec![glossary_entry("Apache Kafka", &["Kafka"])];

        let run = AnalysisRun::for_canonical_session_terms(&transcript, &entries);
        let review_cases =
            run_term_review(&transcript, &entries).expect("glossary has no ambiguous aliases");

        assert_eq!(
            run.snapshot().configuration(),
            CANONICAL_SESSION_TERM_ANALYSIS_IDENTITY
        );
        assert_eq!(review_cases.len(), 1);
    }

    #[test]
    fn run_glossary_review_propagates_duplicate_alias_config_error() {
        let transcript =
            parse_srt("1\n00:00:00,000 --> 00:00:01,000\nusing Kafka here").expect("valid srt");
        let glossary = vec![
            glossary_entry("Apache Kafka", &["Kafka"]),
            glossary_entry("Kafka the author", &["Kafka"]),
        ];

        let result = run_glossary_review(&transcript, &glossary);

        assert_eq!(
            result,
            Err(DetectionError::DuplicateSourceForm {
                source_form: "Kafka".to_string(),
            })
        );
    }
}

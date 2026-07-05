pub mod anchor;
pub mod srt;
pub mod transcript;

#[cfg(test)]
mod tests {
    use crate::anchor::AnchorError;
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
                segment_index: 2,
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
                segment_index: 1,
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
                segment_index: 1,
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
                segment_index: 1,
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
                segment_index: 3,
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
    fn anchor_resolves_to_expected_substring() {
        let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\n我們A").expect("valid srt");

        let anchor = transcript.anchor(0, 0, 3).expect("valid anchor");

        assert_eq!(transcript.resolve(&anchor), Some("我"));
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
}

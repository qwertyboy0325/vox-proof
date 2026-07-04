#[derive(Debug, PartialEq, Eq)]
enum DurationError {
    EndBeforeStart { start_ms: u64, end_ms: u64 },
}

#[derive(Debug, PartialEq, Eq)]
struct ValidationIssue {
    segment_index: u32,
    error: ValidationError,
}

#[derive(Debug, PartialEq, Eq)]
enum ValidationError {
    Duration(DurationError),
    EmptyText,
    NonConsecutiveIndex { previous: u32, found: u32 },
}

#[derive(Debug, PartialEq, Eq)]
enum ParseError {
    MalformedIndex { block: usize, found: String },
    MissingTiming { block: usize },
    MalformedTiming { block: usize, found: String },
}

#[derive(Debug, PartialEq, Eq)]
struct Transcript {
    segments: Vec<Segment>,
}

impl Transcript {
    fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    fn add_segment(&mut self, segment: Segment) {
        self.segments.push(segment);
    }

    fn segments(&self) -> &[Segment] {
        &self.segments
    }

    fn validation_issues(&self) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        let mut previous_index = None;

        for segment in self.segments() {
            if let Some(previous) = previous_index {
                if segment.index != previous + 1 {
                    issues.push(ValidationIssue {
                        segment_index: segment.index,
                        error: ValidationError::NonConsecutiveIndex {
                            previous,
                            found: segment.index,
                        },
                    });
                }
            }

            if segment.text.trim().is_empty() {
                issues.push(ValidationIssue {
                    segment_index: segment.index,
                    error: ValidationError::EmptyText,
                });
            }

            if let Err(error) = segment.duration_ms() {
                issues.push(ValidationIssue {
                    segment_index: segment.index,
                    error: ValidationError::Duration(error),
                });
            }

            previous_index = Some(segment.index);
        }

        issues
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Segment {
    index: u32,
    start_ms: u64,
    end_ms: u64,
    text: String,
}

impl Segment {
    fn duration_ms(&self) -> Result<u64, DurationError> {
        if self.end_ms < self.start_ms {
            Err(DurationError::EndBeforeStart {
                start_ms: self.start_ms,
                end_ms: self.end_ms,
            })
        } else {
            Ok(self.end_ms - self.start_ms)
        }
    }
}

fn parse_srt(input: &str) -> Result<Transcript, ParseError> {
    let mut transcript = Transcript::new();

    for (position, raw_block) in split_into_blocks(input).into_iter().enumerate() {
        let block_number = position + 1;

        let mut lines = raw_block.into_iter();
        let index_line = lines.next().expect("blocks are never empty");

        let index: u32 = index_line
            .trim()
            .parse()
            .map_err(|_| ParseError::MalformedIndex {
                block: block_number,
                found: index_line.to_string(),
            })?;

        let timing_line = lines.next().ok_or(ParseError::MissingTiming {
            block: block_number,
        })?;

        let (start_ms, end_ms) =
            parse_timing(timing_line).ok_or_else(|| ParseError::MalformedTiming {
                block: block_number,
                found: timing_line.to_string(),
            })?;

        let text = lines.collect::<Vec<&str>>().join("\n");

        transcript.add_segment(Segment {
            index,
            start_ms,
            end_ms,
            text,
        });
    }

    Ok(transcript)
}

fn split_into_blocks(input: &str) -> Vec<Vec<&str>> {
    let mut blocks = Vec::new();
    let mut current: Vec<&str> = Vec::new();

    for line in input.lines() {
        if line.trim().is_empty() {
            if !current.is_empty() {
                blocks.push(std::mem::take(&mut current));
            }
        } else {
            current.push(line);
        }
    }

    if !current.is_empty() {
        blocks.push(current);
    }

    blocks
}

fn parse_timing(line: &str) -> Option<(u64, u64)> {
    let (start, end) = line.split_once("-->")?;
    let start_ms = parse_timestamp(start.trim())?;
    let end_ms = parse_timestamp(end.trim())?;
    Some((start_ms, end_ms))
}

fn parse_timestamp(value: &str) -> Option<u64> {
    let (clock, millis) = value.split_once(',')?;
    if millis.len() != 3 || !millis.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let millis: u64 = millis.parse().ok()?;

    let mut parts = clock.split(':');
    let hours: u64 = parts.next()?.parse().ok()?;
    let minutes: u64 = parts.next()?.parse().ok()?;
    let seconds: u64 = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }

    Some(((hours * 60 + minutes) * 60 + seconds) * 1000 + millis)
}

fn main() {}

#[cfg(test)]
mod tests {
    use super::{
        DurationError, ParseError, Segment, Transcript, ValidationError, ValidationIssue, parse_srt,
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
}

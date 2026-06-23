#[derive(Debug, PartialEq, Eq)]
enum DurationError {
    EndBeforeStart { start_ms: u64, end_ms: u64 },
}

#[derive(Debug, PartialEq, Eq)]
struct ValidationIssue {
    segment_index: u32,
    error: DurationError,
}

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

        for segment in self.segments() {
            if let Err(error) = segment.duration_ms() {
                issues.push(ValidationIssue {
                    segment_index: segment.index,
                    error,
                });
            }
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

fn main() {}

#[cfg(test)]
mod tests {
    use super::{DurationError, Segment, Transcript, ValidationIssue};

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
                error: DurationError::EndBeforeStart {
                    start_ms: 4000,
                    end_ms: 3500,
                },
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
}

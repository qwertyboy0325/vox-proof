use std::fs::OpenOptions;
use std::io::{self, Write};

use serde::Serialize;

use crate::transcript::{Transcript, ValidationIssue};

pub const SCHEMA_REVISION: &str = "voxproof-calibration-comparison-v0";
pub const COMPATIBILITY_POLICY_ID: &str = "identical-cue-count-index-and-timing-v0";
pub const CALIBRATION_NOTE: &str = "Calibration artifact only. Not canonical Evidence, not ground truth, not precision/recall/correctness, and not a Material Decision.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CalibrationComparisonReport {
    pub schema_revision: &'static str,
    pub compatibility_policy_id: &'static str,
    pub note: &'static str,
    pub inputs: ComparisonInputs,
    pub summary: ComparisonSummary,
    pub cues: Vec<ComparisonCueRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ComparisonInputs {
    pub raw_path: String,
    pub final_path: String,
    pub raw_revision_id: String,
    pub final_revision_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ComparisonSummary {
    pub cue_count: usize,
    pub unchanged_count: usize,
    pub text_changed_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ComparisonCueRecord {
    pub segment_position: usize,
    pub cue_index: u32,
    pub start_ms: u64,
    pub end_ms: u64,
    pub change_kind: ChangeKind,
    pub raw_text: String,
    pub final_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Unchanged,
    TextChanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComparisonRefusal {
    RawHasValidationIssues {
        issues: Vec<String>,
    },
    FinalHasValidationIssues {
        issues: Vec<String>,
    },
    CueCountMismatch {
        raw: usize,
        final_count: usize,
    },
    CueIndexMismatch {
        segment_position: usize,
        raw_index: u32,
        final_index: u32,
    },
    StartTimingMismatch {
        segment_position: usize,
        cue_index: u32,
    },
    EndTimingMismatch {
        segment_position: usize,
        cue_index: u32,
    },
}

fn validation_issue_lines(issues: &[ValidationIssue]) -> Vec<String> {
    issues
        .iter()
        .map(|issue| {
            format!(
                "  segment position {} (cue index {}): {:?}",
                issue.segment_position(),
                issue.cue_index(),
                issue.error()
            )
        })
        .collect()
}

fn validation_refusal_message(heading: &str, issues: &[String]) -> String {
    let mut message = heading.to_string();
    for line in issues {
        message.push('\n');
        message.push_str(line);
    }
    message
}

impl ComparisonRefusal {
    pub fn message(&self) -> String {
        match self {
            Self::RawHasValidationIssues { issues } => validation_refusal_message(
                "raw SRT has validation issues; comparison refused",
                issues,
            ),
            Self::FinalHasValidationIssues { issues } => validation_refusal_message(
                "final SRT has validation issues; comparison refused",
                issues,
            ),
            Self::CueCountMismatch { raw, final_count } => {
                format!("comparison refused: cue count mismatch (raw: {raw}, final: {final_count})")
            }
            Self::CueIndexMismatch {
                segment_position,
                raw_index,
                final_index,
            } => format!(
                "comparison refused: cue index mismatch at segment_position {segment_position} (raw: {raw_index}, final: {final_index})"
            ),
            Self::StartTimingMismatch {
                segment_position,
                cue_index,
            } => format!(
                "comparison refused: start timing mismatch at segment_position {segment_position} cue_index {cue_index}"
            ),
            Self::EndTimingMismatch {
                segment_position,
                cue_index,
            } => format!(
                "comparison refused: end timing mismatch at segment_position {segment_position} cue_index {cue_index}"
            ),
        }
    }
}

pub fn build_comparison_report(
    raw: &Transcript,
    final_transcript: &Transcript,
    raw_path: &str,
    final_path: &str,
) -> Result<CalibrationComparisonReport, ComparisonRefusal> {
    ensure_compatible(raw, final_transcript)?;

    let mut unchanged_count = 0usize;
    let mut text_changed_count = 0usize;
    let mut cues = Vec::with_capacity(raw.segments().len());

    for (segment_position, (raw_segment, final_segment)) in raw
        .segments()
        .iter()
        .zip(final_transcript.segments().iter())
        .enumerate()
    {
        let change_kind = if raw_segment.text() == final_segment.text() {
            unchanged_count += 1;
            ChangeKind::Unchanged
        } else {
            text_changed_count += 1;
            ChangeKind::TextChanged
        };

        cues.push(ComparisonCueRecord {
            segment_position,
            cue_index: raw_segment.index(),
            start_ms: raw_segment.start_ms(),
            end_ms: raw_segment.end_ms(),
            change_kind,
            raw_text: raw_segment.text().to_string(),
            final_text: final_segment.text().to_string(),
        });
    }

    let cue_count = cues.len();
    debug_assert_eq!(unchanged_count + text_changed_count, cue_count);

    Ok(CalibrationComparisonReport {
        schema_revision: SCHEMA_REVISION,
        compatibility_policy_id: COMPATIBILITY_POLICY_ID,
        note: CALIBRATION_NOTE,
        inputs: ComparisonInputs {
            raw_path: raw_path.to_string(),
            final_path: final_path.to_string(),
            raw_revision_id: raw.revision_id().to_tagged_string(),
            final_revision_id: final_transcript.revision_id().to_tagged_string(),
        },
        summary: ComparisonSummary {
            cue_count,
            unchanged_count,
            text_changed_count,
        },
        cues,
    })
}

pub fn ensure_compatible(
    raw: &Transcript,
    final_transcript: &Transcript,
) -> Result<(), ComparisonRefusal> {
    let raw_issues = raw.validation_issues();
    if !raw_issues.is_empty() {
        return Err(ComparisonRefusal::RawHasValidationIssues {
            issues: validation_issue_lines(&raw_issues),
        });
    }

    let final_issues = final_transcript.validation_issues();
    if !final_issues.is_empty() {
        return Err(ComparisonRefusal::FinalHasValidationIssues {
            issues: validation_issue_lines(&final_issues),
        });
    }

    let raw_segments = raw.segments();
    let final_segments = final_transcript.segments();

    if raw_segments.len() != final_segments.len() {
        return Err(ComparisonRefusal::CueCountMismatch {
            raw: raw_segments.len(),
            final_count: final_segments.len(),
        });
    }

    for (segment_position, (raw_segment, final_segment)) in
        raw_segments.iter().zip(final_segments.iter()).enumerate()
    {
        if raw_segment.index() != final_segment.index() {
            return Err(ComparisonRefusal::CueIndexMismatch {
                segment_position,
                raw_index: raw_segment.index(),
                final_index: final_segment.index(),
            });
        }

        if raw_segment.start_ms() != final_segment.start_ms() {
            return Err(ComparisonRefusal::StartTimingMismatch {
                segment_position,
                cue_index: raw_segment.index(),
            });
        }

        if raw_segment.end_ms() != final_segment.end_ms() {
            return Err(ComparisonRefusal::EndTimingMismatch {
                segment_position,
                cue_index: raw_segment.index(),
            });
        }
    }

    Ok(())
}

pub fn render_comparison_report(
    report: &CalibrationComparisonReport,
) -> Result<String, serde_json::Error> {
    let mut json = serde_json::to_string_pretty(report)?;
    json.push('\n');
    Ok(json)
}

pub fn write_comparison_report_exclusive(path: &str, json: &str) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                format!("refused to write comparison report: destination already exists: {path}")
            } else {
                format!("failed to create comparison report: {error}")
            }
        })?;

    if let Err(error) = file.write_all(json.as_bytes()) {
        drop(file);
        if let Err(cleanup_error) = std::fs::remove_file(path) {
            return Err(format!(
                "failed to write comparison report: {error}; failed to remove partial destination: {cleanup_error}; partial destination may remain: {path}"
            ));
        }
        return Err(format!("failed to write comparison report: {error}"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::srt::parse_srt;

    fn parse_pair(raw: &str, final_srt: &str) -> (Transcript, Transcript) {
        (
            parse_srt(raw).expect("raw parse"),
            parse_srt(final_srt).expect("final parse"),
        )
    }

    #[test]
    fn raw_validation_issues_include_details_in_refusal_message() {
        let raw_srt = "1\n00:00:03,000 --> 00:00:02,500\nreversed";
        let final_srt = "1\n00:00:00,000 --> 00:00:01,000\none";
        let (raw, final_t) = parse_pair(raw_srt, final_srt);
        let refusal = ensure_compatible(&raw, &final_t).expect_err("refused");
        let message = refusal.message();
        assert!(message.contains("raw SRT has validation issues; comparison refused"));
        assert!(message.contains("segment position 0 (cue index 1):"));
        assert!(message.contains("EndBeforeStart { start_ms: 3000, end_ms: 2500 }"));
    }

    #[test]
    fn identical_single_cue_is_unchanged() {
        let srt = "1\n00:00:00,000 --> 00:00:01,000\nKafka";
        let (raw, final_t) = parse_pair(srt, srt);
        let report =
            build_comparison_report(&raw, &final_t, "raw.srt", "final.srt").expect("compatible");
        assert_eq!(report.summary.cue_count, 1);
        assert_eq!(report.summary.unchanged_count, 1);
        assert_eq!(report.summary.text_changed_count, 0);
        assert_eq!(report.cues[0].change_kind, ChangeKind::Unchanged);
    }

    #[test]
    fn identical_multiple_cues_are_unchanged() {
        let srt =
            "1\n00:00:00,000 --> 00:00:01,000\nfirst\n\n2\n00:00:01,000 --> 00:00:02,000\nsecond";
        let (raw, final_t) = parse_pair(srt, srt);
        let report =
            build_comparison_report(&raw, &final_t, "raw.srt", "final.srt").expect("compatible");
        assert_eq!(report.summary.cue_count, 2);
        assert_eq!(report.summary.unchanged_count, 2);
        assert_eq!(report.summary.text_changed_count, 0);
    }

    #[test]
    fn middle_text_change_is_recorded() {
        let raw_srt = "1\n00:00:00,000 --> 00:00:01,000\nfirst\n\n2\n00:00:01,000 --> 00:00:02,000\nKafka\n\n3\n00:00:02,000 --> 00:00:03,000\nlast";
        let final_srt = "1\n00:00:00,000 --> 00:00:01,000\nfirst\n\n2\n00:00:01,000 --> 00:00:02,000\nApache Kafka\n\n3\n00:00:02,000 --> 00:00:03,000\nlast";
        let (raw, final_t) = parse_pair(raw_srt, final_srt);
        let report =
            build_comparison_report(&raw, &final_t, "raw.srt", "final.srt").expect("compatible");
        assert_eq!(report.summary.text_changed_count, 1);
        assert_eq!(report.cues[1].change_kind, ChangeKind::TextChanged);
        assert_eq!(report.cues[1].raw_text, "Kafka");
        assert_eq!(report.cues[1].final_text, "Apache Kafka");
    }

    #[test]
    fn unicode_and_multiline_text_are_preserved_exactly() {
        let raw_srt = "1\n00:00:00,000 --> 00:00:01,000\n前段\n\n2\n00:00:01,000 --> 00:00:02,000\n中間\nKafka\n段";
        let final_srt = "1\n00:00:00,000 --> 00:00:01,000\n前段\n\n2\n00:00:01,000 --> 00:00:02,000\n中間\nApache Kafka\n段";
        let (raw, final_t) = parse_pair(raw_srt, final_srt);
        let report =
            build_comparison_report(&raw, &final_t, "raw.srt", "final.srt").expect("compatible");
        assert_eq!(report.cues[1].raw_text, "中間\nKafka\n段");
        assert_eq!(report.cues[1].final_text, "中間\nApache Kafka\n段");
        assert_eq!(report.cues[1].change_kind, ChangeKind::TextChanged);
    }

    #[test]
    fn cue_count_mismatch_is_refused() {
        let raw_srt =
            "1\n00:00:00,000 --> 00:00:01,000\none\n\n2\n00:00:01,000 --> 00:00:02,000\ntwo";
        let final_srt = "1\n00:00:00,000 --> 00:00:01,000\none";
        let (raw, final_t) = parse_pair(raw_srt, final_srt);
        let refusal =
            build_comparison_report(&raw, &final_t, "raw.srt", "final.srt").expect_err("refused");
        assert_eq!(
            refusal,
            ComparisonRefusal::CueCountMismatch {
                raw: 2,
                final_count: 1
            }
        );
    }

    #[test]
    fn index_mismatch_is_refused() {
        let raw_srt =
            "1\n00:00:00,000 --> 00:00:01,000\none\n\n2\n00:00:01,000 --> 00:00:02,000\ntwo";
        let final_srt =
            "2\n00:00:00,000 --> 00:00:01,000\none\n\n3\n00:00:01,000 --> 00:00:02,000\ntwo";
        let (raw, final_t) = parse_pair(raw_srt, final_srt);
        let refusal = ensure_compatible(&raw, &final_t).expect_err("refused");
        assert_eq!(
            refusal,
            ComparisonRefusal::CueIndexMismatch {
                segment_position: 0,
                raw_index: 1,
                final_index: 2
            }
        );
    }

    #[test]
    fn start_timing_mismatch_is_refused() {
        let raw_srt = "1\n00:00:00,000 --> 00:00:01,000\none";
        let final_srt = "1\n00:00:00,100 --> 00:00:01,000\none";
        let (raw, final_t) = parse_pair(raw_srt, final_srt);
        let refusal = ensure_compatible(&raw, &final_t).expect_err("refused");
        assert_eq!(
            refusal,
            ComparisonRefusal::StartTimingMismatch {
                segment_position: 0,
                cue_index: 1
            }
        );
    }

    #[test]
    fn end_timing_mismatch_is_refused() {
        let raw_srt = "1\n00:00:00,000 --> 00:00:01,000\none";
        let final_srt = "1\n00:00:00,000 --> 00:00:01,100\none";
        let (raw, final_t) = parse_pair(raw_srt, final_srt);
        let refusal = ensure_compatible(&raw, &final_t).expect_err("refused");
        assert_eq!(
            refusal,
            ComparisonRefusal::EndTimingMismatch {
                segment_position: 0,
                cue_index: 1
            }
        );
    }

    #[test]
    fn first_mismatch_wins_for_count_before_index() {
        let raw_srt = "1\n00:00:00,000 --> 00:00:01,000\none";
        let final_srt =
            "2\n00:00:00,000 --> 00:00:01,000\none\n\n3\n00:00:01,000 --> 00:00:02,000\ntwo";
        let (raw, final_t) = parse_pair(raw_srt, final_srt);
        let refusal = ensure_compatible(&raw, &final_t).expect_err("refused");
        assert!(matches!(
            refusal,
            ComparisonRefusal::CueCountMismatch { .. }
        ));
    }

    #[test]
    fn summary_invariants_hold() {
        let raw_srt = "1\n00:00:00,000 --> 00:00:01,000\nsame\n\n2\n00:00:01,000 --> 00:00:02,000\nraw\n\n3\n00:00:02,000 --> 00:00:03,000\nalso";
        let final_srt = "1\n00:00:00,000 --> 00:00:01,000\nsame\n\n2\n00:00:01,000 --> 00:00:02,000\nfinal\n\n3\n00:00:02,000 --> 00:00:03,000\nalso";
        let (raw, final_t) = parse_pair(raw_srt, final_srt);
        let report =
            build_comparison_report(&raw, &final_t, "raw.srt", "final.srt").expect("compatible");
        assert_eq!(
            report.summary.unchanged_count + report.summary.text_changed_count,
            report.summary.cue_count
        );
        assert_eq!(report.cues.len(), report.summary.cue_count);
    }

    #[test]
    fn json_serialization_is_byte_identical_for_identical_inputs() {
        let srt =
            "1\n00:00:00,000 --> 00:00:01,000\nKafka\n\n2\n00:00:01,000 --> 00:00:02,000\n後段";
        let (raw, final_t) = parse_pair(srt, srt);
        let first = render_comparison_report(
            &build_comparison_report(&raw, &final_t, "raw.srt", "final.srt").expect("report"),
        )
        .expect("json");
        let second = render_comparison_report(
            &build_comparison_report(&raw, &final_t, "raw.srt", "final.srt").expect("report"),
        )
        .expect("json");
        assert_eq!(first, second);
        assert!(first.ends_with('\n'));
    }
}

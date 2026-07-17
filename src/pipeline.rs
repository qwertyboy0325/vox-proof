use crate::analysis::AnalysisRun;
use crate::candidate::{
    CandidateSpan, DetectionError, SessionTermEntry, detect_glossary_matches,
    detect_observed_error_form_matches,
};
use crate::review::ReviewCase;
use crate::transcript::Transcript;

/// Canonical analysis run plus the review cases it produced.
/// Constructed only by the canonical pipeline; fields are not publicly mutable.
pub struct CanonicalTermReviewRun {
    analysis_run: AnalysisRun,
    review_cases: Vec<ReviewCase>,
}

impl CanonicalTermReviewRun {
    pub(crate) fn new(analysis_run: AnalysisRun, review_cases: Vec<ReviewCase>) -> Self {
        Self {
            analysis_run,
            review_cases,
        }
    }

    pub fn analysis_run(&self) -> AnalysisRun {
        self.analysis_run
    }

    pub fn review_cases(&self) -> &[ReviewCase] {
        &self.review_cases
    }
}

fn collect_canonical_spans(
    run: &AnalysisRun,
    transcript: &Transcript,
    entries: &[SessionTermEntry],
) -> Result<Vec<CandidateSpan>, DetectionError> {
    let mut spans = detect_glossary_matches(run, transcript, entries)?;
    spans.extend(detect_observed_error_form_matches(
        run, transcript, entries,
    )?);
    spans.extend(crate::phonetic::detect_ascii_latin_phonetic_matches(
        run, transcript, entries,
    )?);
    spans.sort_by(|left, right| {
        let left_anchor = left.anchor();
        let right_anchor = right.anchor();
        (
            left_anchor.segment_position,
            left_anchor.start_byte,
            left_anchor.end_byte,
            left.provenance().detector_id(),
        )
            .cmp(&(
                right_anchor.segment_position,
                right_anchor.start_byte,
                right_anchor.end_byte,
                right.provenance().detector_id(),
            ))
    });
    Ok(spans)
}

/// Composes the canonical session-term evidence paths into human-facing
/// review units under one `AnalysisRun`, returning both the run and cases.
///
/// Findings are ordered by source segment, byte range, and detector identity.
/// The final detector-id tie-break is fixed ordering, not a confidence rank.
pub fn run_canonical_term_review(
    transcript: &Transcript,
    entries: &[SessionTermEntry],
) -> Result<CanonicalTermReviewRun, DetectionError> {
    let run = AnalysisRun::for_canonical_session_terms(transcript, entries);
    let spans = collect_canonical_spans(&run, transcript, entries)?;
    Ok(CanonicalTermReviewRun::new(
        run,
        ReviewCase::from_detector_candidates(spans),
    ))
}

/// Composes the canonical session-term evidence paths into human-facing
/// review units under one `AnalysisRun`.
///
/// Findings are ordered by source segment, byte range, and detector identity.
/// The final detector-id tie-break is fixed ordering, not a confidence rank.
pub fn run_term_review(
    transcript: &Transcript,
    entries: &[SessionTermEntry],
) -> Result<Vec<ReviewCase>, DetectionError> {
    Ok(run_canonical_term_review(transcript, entries)?.review_cases)
}

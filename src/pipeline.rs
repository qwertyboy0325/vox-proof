use crate::analysis::AnalysisRun;
use crate::candidate::{
    DetectionError, SessionTermEntry, detect_glossary_matches, detect_observed_error_form_matches,
};
use crate::review::ReviewCase;
use crate::transcript::Transcript;

/// Composes the two exact session-term evidence paths into human-facing
/// review units under one `AnalysisRun`.
///
/// Findings are ordered by source segment, byte range, and detector identity.
/// The final detector-id tie-break is fixed ordering, not a confidence rank.
pub fn run_term_review(
    transcript: &Transcript,
    entries: &[SessionTermEntry],
) -> Result<Vec<ReviewCase>, DetectionError> {
    let run = AnalysisRun::for_exact_session_terms(transcript, entries);
    let mut spans = detect_glossary_matches(&run, transcript, entries)?;
    spans.extend(detect_observed_error_form_matches(
        &run, transcript, entries,
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

    Ok(ReviewCase::from_detector_candidates(spans))
}

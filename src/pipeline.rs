use crate::analysis::AnalysisRun;
use crate::candidate::{DetectionError, GlossaryEntry, detect_glossary_matches};
use crate::review::ReviewCase;
use crate::transcript::Transcript;

/// Composes the glossary detector into the human-facing review unit: builds
/// an `AnalysisRun` for `transcript`, runs the glossary alias detector, and
/// wraps each resulting `CandidateSpan` in a `ReviewCase`.
///
/// This is a single-detector assembly, not a general pipeline: it exists to
/// prove that `AnalysisRun`, the glossary detector, `CandidateSpan`, and
/// `ReviewCase` compose end-to-end, not to host ranking, review status,
/// decisions, persistence, or multi-detector orchestration.
pub fn run_glossary_review(
    transcript: &Transcript,
    glossary: &[GlossaryEntry],
) -> Result<Vec<ReviewCase>, DetectionError> {
    let run = AnalysisRun::new(transcript);
    let spans = detect_glossary_matches(&run, transcript, glossary)?;
    Ok(ReviewCase::from_detector_candidates(spans))
}

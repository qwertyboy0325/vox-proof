use crate::analysis::{AnalysisRun, ReuseEnabledAnalysisSnapshot};
use crate::candidate::{
    CandidateSpan, DetectionError, SessionTermEntry, detect_glossary_matches,
    detect_glossary_matches_reuse_enabled, detect_observed_error_form_matches,
};
use crate::reusable_influence::{
    ResolvedExactInputProjection, ReusableInfluenceSnapshot,
    detect_resolved_exact_observed_form_matches,
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

/// Reuse-enabled analysis run plus review cases and bound reusable snapshot identity.
pub struct ReuseEnabledTermReviewRun {
    analysis_run: AnalysisRun,
    review_cases: Vec<ReviewCase>,
    reusable_snapshot_identity: crate::reuse_primitives::ReusableInfluenceSnapshotIdentity,
}

impl ReuseEnabledTermReviewRun {
    pub fn analysis_run(&self) -> AnalysisRun {
        self.analysis_run
    }

    pub fn review_cases(&self) -> &[ReviewCase] {
        &self.review_cases
    }

    pub fn reusable_snapshot_identity(
        &self,
    ) -> crate::reuse_primitives::ReusableInfluenceSnapshotIdentity {
        self.reusable_snapshot_identity
    }

    pub fn reuse_enabled_snapshot(&self) -> ReuseEnabledAnalysisSnapshot {
        ReuseEnabledAnalysisSnapshot::new(
            self.analysis_run.snapshot(),
            self.reusable_snapshot_identity,
        )
    }
}

fn collect_reuse_enabled_spans(
    run: &AnalysisRun,
    transcript: &Transcript,
    entries: &[SessionTermEntry],
    projection: &ResolvedExactInputProjection,
    snapshot: &ReusableInfluenceSnapshot,
) -> Result<Vec<CandidateSpan>, DetectionError> {
    let mut spans = detect_glossary_matches_reuse_enabled(run, transcript, entries)?;
    spans.extend(detect_resolved_exact_observed_form_matches(
        run, transcript, entries, projection, snapshot,
    )?);
    spans.extend(
        crate::phonetic::detect_ascii_latin_phonetic_matches_reuse_enabled(
            run, transcript, entries,
        )?,
    );
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

/// Explicit reuse-enabled term review path. Canonical callers without reusable
/// influence must continue using `run_canonical_term_review`.
pub fn run_reuse_enabled_term_review(
    transcript: &Transcript,
    entries: &[SessionTermEntry],
    projection: &ResolvedExactInputProjection,
    snapshot: &ReusableInfluenceSnapshot,
) -> Result<ReuseEnabledTermReviewRun, DetectionError> {
    let run = AnalysisRun::for_reuse_enabled_session_terms(transcript, entries);
    let spans = collect_reuse_enabled_spans(&run, transcript, entries, projection, snapshot)?;
    Ok(ReuseEnabledTermReviewRun {
        analysis_run: run,
        review_cases: ReviewCase::from_detector_candidates(spans),
        reusable_snapshot_identity: snapshot.identity,
    })
}

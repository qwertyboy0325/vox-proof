use crate::anchor::TranscriptRevisionId;
use crate::transcript::Transcript;

/// The effective inputs and configuration under which analysis was
/// performed. For v0.1 this models only the source transcript revision,
/// because no other configurable analysis input (language pack revision,
/// normalizer configuration, ranking configuration) exists in the
/// implementation yet. Detector identity and version are recorded per
/// finding via `DetectorProvenance` rather than duplicated here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnalysisSnapshot {
    source_revision: TranscriptRevisionId,
}

impl AnalysisSnapshot {
    pub fn source_revision(&self) -> TranscriptRevisionId {
        self.source_revision
    }
}

/// One bounded analysis execution over one transcript revision under one
/// effective `AnalysisSnapshot`. `AnalysisRun` is a provenance and
/// reproducibility boundary: it does not schedule, persist, or run
/// detectors itself. Detectors are called with a run and must verify the
/// run's snapshot matches the transcript actually being analyzed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnalysisRun {
    snapshot: AnalysisSnapshot,
}

impl AnalysisRun {
    pub fn new(transcript: &Transcript) -> Self {
        Self {
            snapshot: AnalysisSnapshot {
                source_revision: transcript.revision_id(),
            },
        }
    }

    pub fn snapshot(&self) -> AnalysisSnapshot {
        self.snapshot
    }
}

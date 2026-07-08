use crate::anchor::{AnchorError, SourceAnchor, TranscriptRevisionId};
use sha2::{Digest, Sha256};

#[derive(Debug, PartialEq, Eq)]
pub enum DurationError {
    EndBeforeStart { start_ms: u64, end_ms: u64 },
}

#[derive(Debug, PartialEq, Eq)]
pub struct ValidationIssue {
    pub(crate) segment_index: u32,
    pub(crate) error: ValidationError,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ValidationError {
    Duration(DurationError),
    EmptyText,
    NonConsecutiveIndex { previous: u32, found: u32 },
}

#[derive(Debug, PartialEq, Eq)]
pub struct Transcript {
    segments: Vec<Segment>,
    revision: TranscriptRevisionId,
}

impl Transcript {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self::from_segments(Vec::new())
    }

    pub(crate) fn from_segments(segments: Vec<Segment>) -> Self {
        let revision = compute_revision(&segments);

        Self { segments, revision }
    }

    #[cfg(test)]
    pub(crate) fn add_segment(&mut self, segment: Segment) {
        self.segments.push(segment);
        self.revision = compute_revision(&self.segments);
    }

    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    pub fn revision_id(&self) -> TranscriptRevisionId {
        self.revision
    }

    pub fn anchor(
        &self,
        segment_position: usize,
        start_byte: usize,
        end_byte: usize,
    ) -> Result<SourceAnchor, AnchorError> {
        let segment = self
            .segments
            .get(segment_position)
            .ok_or(AnchorError::UnknownSegment { segment_position })?;

        if start_byte >= end_byte {
            return Err(AnchorError::EmptyOrInvertedRange {
                start_byte,
                end_byte,
            });
        }

        let text_len = segment.text.len();
        if end_byte > text_len {
            return Err(AnchorError::RangeOutOfBounds { end_byte, text_len });
        }

        if !segment.text.is_char_boundary(start_byte) {
            return Err(AnchorError::NotCharBoundary { byte: start_byte });
        }
        if !segment.text.is_char_boundary(end_byte) {
            return Err(AnchorError::NotCharBoundary { byte: end_byte });
        }

        Ok(SourceAnchor {
            revision: self.revision_id(),
            segment_position,
            start_byte,
            end_byte,
        })
    }

    pub fn resolve(&self, anchor: &SourceAnchor) -> Option<&str> {
        if anchor.revision != self.revision_id() {
            return None;
        }

        self.segments
            .get(anchor.segment_position)?
            .text
            .get(anchor.start_byte..anchor.end_byte)
    }

    pub fn normalized_view(&self) -> NormalizedTranscript {
        let segments = self
            .segments()
            .iter()
            .map(|segment| NormalizedSegment {
                source_segment_index: segment.index,
                normalized_text: segment.text.clone(),
            })
            .collect();

        NormalizedTranscript { segments }
    }

    pub fn validation_issues(&self) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        let mut previous_index = None;

        for segment in self.segments() {
            if let Some(previous) = previous_index
                && segment.index != previous + 1
            {
                issues.push(ValidationIssue {
                    segment_index: segment.index,
                    error: ValidationError::NonConsecutiveIndex {
                        previous,
                        found: segment.index,
                    },
                });
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

fn compute_revision(segments: &[Segment]) -> TranscriptRevisionId {
    const DOMAIN_SEPARATOR: &[u8] = b"voxproof-transcript-rev-v1";

    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_SEPARATOR);
    hasher.update([0x00]);
    hasher.update((segments.len() as u64).to_le_bytes());

    for segment in segments {
        hasher.update(segment.index.to_le_bytes());
        hasher.update(segment.start_ms.to_le_bytes());
        hasher.update(segment.end_ms.to_le_bytes());
        hasher.update((segment.text.len() as u64).to_le_bytes());
        hasher.update(segment.text.as_bytes());
    }

    TranscriptRevisionId::from_sha256_digest(hasher.finalize().into())
}

#[derive(Debug, PartialEq, Eq)]
pub struct NormalizedTranscript {
    segments: Vec<NormalizedSegment>,
}

impl NormalizedTranscript {
    pub fn segments(&self) -> &[NormalizedSegment] {
        &self.segments
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct NormalizedSegment {
    pub(crate) source_segment_index: u32,
    pub(crate) normalized_text: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Segment {
    pub(crate) index: u32,
    pub(crate) start_ms: u64,
    pub(crate) end_ms: u64,
    pub(crate) text: String,
}

impl Segment {
    pub fn duration_ms(&self) -> Result<u64, DurationError> {
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

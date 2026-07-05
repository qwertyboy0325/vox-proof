#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TranscriptRevisionId(pub(crate) u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceAnchor {
    pub(crate) revision: TranscriptRevisionId,
    pub(crate) segment_position: usize,
    pub(crate) start_byte: usize,
    pub(crate) end_byte: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AnchorError {
    UnknownSegment { segment_position: usize },
    EmptyOrInvertedRange { start_byte: usize, end_byte: usize },
    RangeOutOfBounds { end_byte: usize, text_len: usize },
    NotCharBoundary { byte: usize },
}

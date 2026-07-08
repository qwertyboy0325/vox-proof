#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TranscriptRevisionId {
    digest: [u8; 32],
}

impl TranscriptRevisionId {
    pub(crate) fn from_sha256_digest(digest: [u8; 32]) -> Self {
        Self { digest }
    }

    pub fn to_tagged_string(self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";

        let mut encoded = String::with_capacity("rev:sha256-v1:".len() + 64);
        encoded.push_str("rev:sha256-v1:");

        for byte in self.digest {
            encoded.push(HEX[(byte >> 4) as usize] as char);
            encoded.push(HEX[(byte & 0x0f) as usize] as char);
        }

        encoded
    }
}

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

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::anchor::SourceAnchor;
use crate::transcript::Transcript;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DetectionKind {
    GlossaryAliasMatch,
    MixedLanguageAnomaly,
    PhoneticSimilarity,
    RepeatedPhrase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectorProvenance {
    detector_id: String,
    detector_version: String,
}

impl DetectorProvenance {
    pub fn new(detector_id: impl Into<String>, detector_version: impl Into<String>) -> Self {
        Self {
            detector_id: detector_id.into(),
            detector_version: detector_version.into(),
        }
    }

    pub fn detector_id(&self) -> &str {
        &self.detector_id
    }

    pub fn detector_version(&self) -> &str {
        &self.detector_version
    }
}

/// Semantic identity of a finding. Deliberately excludes `detector_version`:
/// the contract defers detector-version migration semantics, so a finding's
/// identity must not be pinned to the exact version that produced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CandidateKey(u64);

impl CandidateKey {
    fn compute(detector_id: &str, kind: DetectionKind, anchor: &SourceAnchor) -> Self {
        let mut hasher = DefaultHasher::new();
        detector_id.hash(&mut hasher);
        kind.hash(&mut hasher);
        anchor.hash(&mut hasher);
        CandidateKey(hasher.finish())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlossaryEntry {
    pub canonical_term: String,
    pub aliases: Vec<String>,
}

impl GlossaryEntry {
    pub fn new(canonical_term: impl Into<String>, aliases: Vec<String>) -> Self {
        Self {
            canonical_term: canonical_term.into(),
            aliases,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlossaryEvidence {
    pub entry: GlossaryEntry,
    pub matched_form: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Evidence {
    Glossary(GlossaryEvidence),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateSpan {
    key: CandidateKey,
    anchor: SourceAnchor,
    kind: DetectionKind,
    provenance: DetectorProvenance,
    evidence: Evidence,
}

impl CandidateSpan {
    pub(crate) fn new(
        kind: DetectionKind,
        provenance: DetectorProvenance,
        anchor: SourceAnchor,
        evidence: Evidence,
    ) -> Self {
        let key = CandidateKey::compute(provenance.detector_id(), kind, &anchor);
        Self {
            key,
            anchor,
            kind,
            provenance,
            evidence,
        }
    }

    pub fn key(&self) -> CandidateKey {
        self.key
    }

    pub fn anchor(&self) -> &SourceAnchor {
        &self.anchor
    }

    pub fn kind(&self) -> DetectionKind {
        self.kind
    }

    pub fn provenance(&self) -> &DetectorProvenance {
        &self.provenance
    }

    pub fn evidence(&self) -> &Evidence {
        &self.evidence
    }
}

const GLOSSARY_DETECTOR_ID: &str = "glossary-alias-match";
const GLOSSARY_DETECTOR_VERSION: &str = "0.1.0";

/// Finds exact, case-sensitive occurrences of glossary aliases in the
/// transcript. Matching is byte-exact on the parsed segment text: no case
/// folding or other text normalization is applied, because non-identity
/// normalization is still an open decision gate.
pub fn detect_glossary_matches(
    transcript: &Transcript,
    glossary: &[GlossaryEntry],
) -> Vec<CandidateSpan> {
    let provenance = DetectorProvenance::new(GLOSSARY_DETECTOR_ID, GLOSSARY_DETECTOR_VERSION);
    let mut spans = Vec::new();

    for (position, segment) in transcript.segments().iter().enumerate() {
        for entry in glossary {
            for alias in &entry.aliases {
                if alias.is_empty() {
                    continue;
                }

                for (start, matched) in segment.text.match_indices(alias.as_str()) {
                    let end = start + matched.len();
                    let anchor = transcript
                        .anchor(position, start, end)
                        .expect("a matched substring is always a valid char-boundary anchor");

                    let evidence = Evidence::Glossary(GlossaryEvidence {
                        entry: entry.clone(),
                        matched_form: matched.to_string(),
                    });

                    spans.push(CandidateSpan::new(
                        DetectionKind::GlossaryAliasMatch,
                        provenance.clone(),
                        anchor,
                        evidence,
                    ));
                }
            }
        }
    }

    spans
}

use std::collections::HashSet;

use crate::analysis::AnalysisRun;
use crate::anchor::{SourceAnchor, TranscriptRevisionId};
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

/// Semantic identity of a finding: detector identity, detection kind, and
/// the source anchor (which itself carries the transcript revision, so
/// revision is not duplicated as a separate field here). Deliberately
/// excludes `detector_version`: the contract defers detector-version
/// migration semantics, so a finding's identity must not be pinned to the
/// exact version that produced it. This is a plain value, not an opaque
/// hash: hashing/serialization is a future representation choice layered
/// on top, not the identity itself.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CandidateKey {
    detector_id: String,
    kind: DetectionKind,
    anchor: SourceAnchor,
}

impl CandidateKey {
    fn new(detector_id: &str, kind: DetectionKind, anchor: SourceAnchor) -> Self {
        Self {
            detector_id: detector_id.to_string(),
            kind,
            anchor,
        }
    }

    pub fn detector_id(&self) -> &str {
        &self.detector_id
    }

    pub fn kind(&self) -> DetectionKind {
        self.kind
    }

    pub fn anchor(&self) -> &SourceAnchor {
        &self.anchor
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
        let key = CandidateKey::new(provenance.detector_id(), kind, anchor);
        Self {
            key,
            anchor,
            kind,
            provenance,
            evidence,
        }
    }

    pub fn key(&self) -> &CandidateKey {
        &self.key
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

#[derive(Debug, PartialEq, Eq)]
pub enum DetectionError {
    RevisionMismatch {
        run_revision: TranscriptRevisionId,
        transcript_revision: TranscriptRevisionId,
    },
    DuplicateGlossaryAlias {
        alias: String,
    },
}

const GLOSSARY_DETECTOR_ID: &str = "glossary-alias-match";
const GLOSSARY_DETECTOR_VERSION: &str = "0.1.0";

/// Finds exact, case-sensitive occurrences of glossary aliases in the
/// transcript. Matching is byte-exact on the parsed segment text: no case
/// folding or other text normalization is applied, because non-identity
/// normalization is still an open decision gate.
///
/// `run` must be an `AnalysisRun` created from `transcript`; a run from a
/// different transcript revision is rejected. Aliases must be unique across
/// the whole glossary: a shared alias would let two entries produce
/// different `Evidence` for the same `CandidateKey`, which would violate
/// `CandidateKey` as an unambiguous deduplication identity, so it is
/// rejected as a configuration error rather than silently merged or
/// arbitrarily chosen.
pub fn detect_glossary_matches(
    run: &AnalysisRun,
    transcript: &Transcript,
    glossary: &[GlossaryEntry],
) -> Result<Vec<CandidateSpan>, DetectionError> {
    let run_revision = run.snapshot().source_revision();
    let transcript_revision = transcript.revision_id();
    if run_revision != transcript_revision {
        return Err(DetectionError::RevisionMismatch {
            run_revision,
            transcript_revision,
        });
    }

    reject_ambiguous_aliases(glossary)?;

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

    Ok(spans)
}

fn reject_ambiguous_aliases(glossary: &[GlossaryEntry]) -> Result<(), DetectionError> {
    let mut seen_aliases = HashSet::new();

    for entry in glossary {
        for alias in &entry.aliases {
            if alias.is_empty() {
                continue;
            }

            if !seen_aliases.insert(alias.as_str()) {
                return Err(DetectionError::DuplicateGlossaryAlias {
                    alias: alias.clone(),
                });
            }
        }
    }

    Ok(())
}

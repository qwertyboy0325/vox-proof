use std::collections::HashSet;

use crate::analysis::{
    AlgorithmIdentity, AnalysisConfigurationIdentity, AnalysisRun, CanonicalDetectorSetIdentity,
    DetectorConfigIdentity, DetectorIdentity, SessionTermsIdentity,
};
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

    pub(crate) fn from_detector_identity(identity: DetectorIdentity) -> Self {
        Self {
            detector_id: identity.id().to_string(),
            detector_version: identity.version().to_string(),
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
pub struct SessionTermEntry {
    pub canonical_term: String,
    pub aliases: Vec<String>,
    pub observed_error_forms: Vec<String>,
}

impl SessionTermEntry {
    pub fn new(
        canonical_term: impl Into<String>,
        aliases: Vec<String>,
        observed_error_forms: Vec<String>,
    ) -> Self {
        Self {
            canonical_term: canonical_term.into(),
            aliases,
            observed_error_forms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlossaryAliasEvidence {
    pub entry: SessionTermEntry,
    pub matched_form: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedErrorFormEvidence {
    pub entry: SessionTermEntry,
    pub matched_form: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhoneticTargetKind {
    CanonicalTerm,
    Alias,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsciiLatinPhoneticRepresentation {
    pub normalized_letters: String,
    pub primary_key: String,
    pub alternate_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhoneticComparisonFacts {
    pub edit_distance: usize,
    pub ratio_numerator: usize,
    pub ratio_denominator: usize,
    pub ratio_permille: usize,
    pub matched_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhoneticSimilarityEvidence {
    pub observed_surface: String,
    pub target_surface: String,
    pub target_kind: PhoneticTargetKind,
    pub canonical_term: String,
    pub source_representation: AsciiLatinPhoneticRepresentation,
    pub target_representation: AsciiLatinPhoneticRepresentation,
    pub comparison: PhoneticComparisonFacts,
    pub detector_config: DetectorConfigIdentity,
    pub algorithm: AlgorithmIdentity,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Evidence {
    GlossaryAlias(GlossaryAliasEvidence),
    ObservedErrorForm(ObservedErrorFormEvidence),
    PhoneticSimilarity(PhoneticSimilarityEvidence),
}

/// A non-binding suggested replacement. It is not an edit decision and must
/// not automatically modify source text: turning an alternative into an
/// edit requires a separate, explicit policy or human decision, which is
/// outside this contract gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateAlternative {
    replacement_text: String,
}

impl CandidateAlternative {
    pub fn new(replacement_text: impl Into<String>) -> Self {
        Self {
            replacement_text: replacement_text.into(),
        }
    }

    pub fn replacement_text(&self) -> &str {
        &self.replacement_text
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateSpan {
    key: CandidateKey,
    anchor: SourceAnchor,
    kind: DetectionKind,
    provenance: DetectorProvenance,
    evidence: Evidence,
    alternatives: Vec<CandidateAlternative>,
}

impl CandidateSpan {
    pub(crate) fn new(
        kind: DetectionKind,
        provenance: DetectorProvenance,
        anchor: SourceAnchor,
        evidence: Evidence,
        alternatives: Vec<CandidateAlternative>,
    ) -> Self {
        let key = CandidateKey::new(provenance.detector_id(), kind, anchor);
        Self {
            key,
            anchor,
            kind,
            provenance,
            evidence,
            alternatives,
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

    /// Zero or more non-binding suggested replacements. An empty slice
    /// means the detector found the span suspicious but has no concrete
    /// suggestion, not that the source text is confirmed correct.
    pub fn alternatives(&self) -> &[CandidateAlternative] {
        &self.alternatives
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum DetectionError {
    RevisionMismatch {
        run_revision: TranscriptRevisionId,
        transcript_revision: TranscriptRevisionId,
    },
    SessionTermsIdentityMismatch {
        run_identity: SessionTermsIdentity,
        provided_identity: SessionTermsIdentity,
    },
    DetectorSetIdentityMismatch {
        run_identity: CanonicalDetectorSetIdentity,
        required_identity: CanonicalDetectorSetIdentity,
    },
    DetectorConfigIdentityMismatch {
        run_identity: DetectorConfigIdentity,
        required_identity: DetectorConfigIdentity,
    },
    AlgorithmIdentityMismatch {
        run_identity: AlgorithmIdentity,
        required_identity: AlgorithmIdentity,
    },
    DuplicateSourceForm {
        source_form: String,
    },
    DuplicateCanonicalTerm {
        canonical_term: String,
    },
    EmptyAlias {
        canonical_term: String,
    },
    EmptyObservedErrorForm {
        canonical_term: String,
    },
}

pub(crate) const GLOSSARY_DETECTOR: DetectorIdentity =
    DetectorIdentity::new("glossary-alias-match", "0.1.0");
pub(crate) const OBSERVED_ERROR_FORM_DETECTOR: DetectorIdentity =
    DetectorIdentity::new("observed-error-form-match", "0.1.0");

pub(crate) const PHONETIC_DETECTOR: DetectorIdentity =
    DetectorIdentity::new("ascii-latin-phonetic-similarity", "0.1.0");

pub(crate) const CANONICAL_SESSION_TERM_DETECTORS: &[DetectorIdentity] = &[
    GLOSSARY_DETECTOR,
    OBSERVED_ERROR_FORM_DETECTOR,
    PHONETIC_DETECTOR,
];

pub(crate) const CANONICAL_SESSION_TERM_DETECTOR_SET: CanonicalDetectorSetIdentity =
    CanonicalDetectorSetIdentity::new(CANONICAL_SESSION_TERM_DETECTORS);

pub(crate) const CANONICAL_SESSION_TERM_DETECTOR_CONFIG: DetectorConfigIdentity =
    DetectorConfigIdentity::new("canonical-session-term-cue-local", "0.2.0");

pub(crate) const CANONICAL_SESSION_TERM_ALGORITHM: AlgorithmIdentity = AlgorithmIdentity::new(
    "canonical-exact-plus-ascii-double-metaphone-levenshtein",
    "rphonetic-3.0.6-v1",
);

pub(crate) const CANONICAL_SESSION_TERM_ANALYSIS_IDENTITY: AnalysisConfigurationIdentity =
    AnalysisConfigurationIdentity::new(
        CANONICAL_SESSION_TERM_DETECTOR_SET,
        CANONICAL_SESSION_TERM_DETECTOR_CONFIG,
        CANONICAL_SESSION_TERM_ALGORITHM,
    );

pub(crate) const fn canonical_session_term_analysis_identity() -> AnalysisConfigurationIdentity {
    CANONICAL_SESSION_TERM_ANALYSIS_IDENTITY
}

pub(crate) fn validate_detection_inputs(
    run: &AnalysisRun,
    transcript: &Transcript,
    entries: &[SessionTermEntry],
) -> Result<(), DetectionError> {
    let run_revision = run.snapshot().source_revision();
    let transcript_revision = transcript.revision_id();
    if run_revision != transcript_revision {
        return Err(DetectionError::RevisionMismatch {
            run_revision,
            transcript_revision,
        });
    }

    let run_session_terms = run.snapshot().session_terms();
    let provided_session_terms = SessionTermsIdentity::from_entries(entries);
    if run_session_terms != provided_session_terms {
        return Err(DetectionError::SessionTermsIdentityMismatch {
            run_identity: run_session_terms,
            provided_identity: provided_session_terms,
        });
    }

    let run_configuration = run.snapshot().configuration();
    let required_configuration = canonical_session_term_analysis_identity();
    if run_configuration.detector_set() != required_configuration.detector_set() {
        return Err(DetectionError::DetectorSetIdentityMismatch {
            run_identity: run_configuration.detector_set(),
            required_identity: required_configuration.detector_set(),
        });
    }
    if run_configuration.detector_config() != required_configuration.detector_config() {
        return Err(DetectionError::DetectorConfigIdentityMismatch {
            run_identity: run_configuration.detector_config(),
            required_identity: required_configuration.detector_config(),
        });
    }
    if run_configuration.algorithm() != required_configuration.algorithm() {
        return Err(DetectionError::AlgorithmIdentityMismatch {
            run_identity: run_configuration.algorithm(),
            required_identity: required_configuration.algorithm(),
        });
    }

    let mut seen_canonical_terms = HashSet::new();
    let mut seen_source_forms = HashSet::new();
    for entry in entries {
        if !seen_canonical_terms.insert(entry.canonical_term.as_str()) {
            return Err(DetectionError::DuplicateCanonicalTerm {
                canonical_term: entry.canonical_term.clone(),
            });
        }

        for alias in &entry.aliases {
            if alias.is_empty() {
                return Err(DetectionError::EmptyAlias {
                    canonical_term: entry.canonical_term.clone(),
                });
            }
        }
        for observed_form in &entry.observed_error_forms {
            if observed_form.is_empty() {
                return Err(DetectionError::EmptyObservedErrorForm {
                    canonical_term: entry.canonical_term.clone(),
                });
            }
        }

        for source_form in entry.aliases.iter().chain(&entry.observed_error_forms) {
            if !seen_source_forms.insert(source_form.as_str()) {
                return Err(DetectionError::DuplicateSourceForm {
                    source_form: source_form.clone(),
                });
            }
        }
    }

    Ok(())
}

/// Finds exact, case-sensitive occurrences of a matched non-canonical
/// glossary form in the transcript. Matching is byte-exact on the parsed
/// segment text: no case folding or other text normalization is applied,
/// because non-identity normalization is still an open decision gate.
///
/// An occurrence whose matched text is exactly the entry's canonical term
/// is not a finding: `GlossaryAliasMatch` means the source used a
/// non-canonical form, not merely that a glossary term is present. Such an
/// occurrence produces no `CandidateSpan`, not a `CandidateSpan` with an
/// empty `alternatives()`; an empty alternatives list is reserved for
/// detectors that flag a suspicious span without a reliable replacement.
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
    entries: &[SessionTermEntry],
) -> Result<Vec<CandidateSpan>, DetectionError> {
    validate_detection_inputs(run, transcript, entries)?;

    let provenance = DetectorProvenance::from_detector_identity(GLOSSARY_DETECTOR);
    let mut spans = Vec::new();

    for (position, segment) in transcript.segments().iter().enumerate() {
        for entry in entries {
            for alias in &entry.aliases {
                for (start, matched) in segment.text.match_indices(alias.as_str()) {
                    if matched == entry.canonical_term {
                        continue;
                    }

                    let end = start + matched.len();
                    let anchor = transcript
                        .anchor(position, start, end)
                        .expect("a matched substring is always a valid char-boundary anchor");

                    let evidence = Evidence::GlossaryAlias(GlossaryAliasEvidence {
                        entry: entry.clone(),
                        matched_form: matched.to_string(),
                    });

                    // The canonical term is factual supporting evidence, not
                    // an accepted replacement; wrapping it as a
                    // CandidateAlternative keeps it explicitly non-binding.
                    let alternatives =
                        vec![CandidateAlternative::new(entry.canonical_term.clone())];

                    spans.push(CandidateSpan::new(
                        DetectionKind::GlossaryAliasMatch,
                        provenance.clone(),
                        anchor,
                        evidence,
                        alternatives,
                    ));
                }
            }
        }
    }

    Ok(spans)
}

/// Finds exact, case-sensitive occurrences of explicitly supplied observed
/// ASR error forms. The evidence reports that the session input classified
/// the matched form as observed; it does not make the form ground truth or
/// authorize replacement without a human decision.
pub fn detect_observed_error_form_matches(
    run: &AnalysisRun,
    transcript: &Transcript,
    entries: &[SessionTermEntry],
) -> Result<Vec<CandidateSpan>, DetectionError> {
    validate_detection_inputs(run, transcript, entries)?;

    let provenance = DetectorProvenance::from_detector_identity(OBSERVED_ERROR_FORM_DETECTOR);
    let mut spans = Vec::new();

    for (position, segment) in transcript.segments().iter().enumerate() {
        for entry in entries {
            for observed_form in &entry.observed_error_forms {
                for (start, matched) in segment.text.match_indices(observed_form.as_str()) {
                    if matched == entry.canonical_term {
                        continue;
                    }

                    let end = start + matched.len();
                    let anchor = transcript
                        .anchor(position, start, end)
                        .expect("a matched substring is always a valid char-boundary anchor");
                    let evidence = Evidence::ObservedErrorForm(ObservedErrorFormEvidence {
                        entry: entry.clone(),
                        matched_form: matched.to_string(),
                    });
                    let alternatives =
                        vec![CandidateAlternative::new(entry.canonical_term.clone())];

                    spans.push(CandidateSpan::new(
                        DetectionKind::GlossaryAliasMatch,
                        provenance.clone(),
                        anchor,
                        evidence,
                        alternatives,
                    ));
                }
            }
        }
    }

    Ok(spans)
}

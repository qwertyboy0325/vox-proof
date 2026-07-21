use std::collections::{HashMap, HashSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::artifact_bundle::{ArtifactBundle, ArtifactBundleValidationError, ArtifactId};
use crate::candidate::DetectionKind;
use crate::reference_coverage::{CueReferenceId, CueReferenceIdError};
use crate::run_manifest::{
    CalibrationValidityMode, InputIdentityReference, InputIdentityValidationError, RunEnvelope,
    RunEnvelopeValidationError, RunId, RunIdError, RunLifecycleState,
    validate_input_identity_reference, validate_opaque_identifier,
};

pub const DETECTOR_PROPOSAL_SNAPSHOT_SCHEMA: &str = "voxproof-detector-proposal-snapshot-v1";

const SESSION_TERMS_IDENTITY_PREFIX: &str = "session-terms:sha256-v1:";
const COMPONENT_TOKEN_MAX_LEN: usize = 128;

/// Private content-bearing artifact contract. Authorization to define or create
/// a detector snapshot does not authorize committing real detector surfaces.
/// This contract is not public or commit-safe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetectorProposalSnapshot {
    pub schema_revision: String,
    pub run_id: RunId,
    pub input_identity: InputIdentityReference,
    pub calibration_validity: CalibrationValidityMode,
    pub snapshot_revision: DetectorSnapshotRevisionId,
    pub detector_output_artifact_id: ArtifactId,
    pub analysis_identity: DetectorAnalysisIdentity,
    pub proposals: Vec<DetectorProposalRecord>,
    pub frozen_at_unix_ms: u64,
    pub state: DetectorProposalSnapshotState,
    pub assessment: DetectorProposalSnapshotAssessment,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DetectorSnapshotRevisionId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DetectorProposalId(String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetectorProposalSourceAnchor {
    pub input_identity: InputIdentityReference,
    pub cue_id: CueReferenceId,
    pub segment_position: u32,
    pub start_byte: u32,
    pub end_byte: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetectorComponentIdentity {
    pub id: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetectorAnalysisIdentity {
    pub input_identity: InputIdentityReference,
    pub session_terms_identity: String,
    pub detector_set: Vec<DetectorComponentIdentity>,
    pub detector_config: DetectorComponentIdentity,
    pub algorithm: DetectorComponentIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetectorProposalSemanticKey {
    pub detector_id: String,
    pub detection_kind: DetectionKind,
    pub source_anchor: DetectorProposalSourceAnchor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetectorProposalAlternative {
    pub alternative_index: u32,
    pub replacement_surface: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetectorSessionTermEntry {
    pub canonical_term: String,
    pub aliases: Vec<String>,
    pub observed_error_forms: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectorPhoneticTargetKind {
    CanonicalTerm,
    Alias,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetectorAsciiLatinPhoneticRepresentation {
    pub normalized_letters: String,
    pub primary_key: String,
    pub alternate_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetectorPhoneticComparisonFacts {
    pub edit_distance: u32,
    pub ratio_numerator: u32,
    pub ratio_denominator: u32,
    pub ratio_permille: u32,
    pub matched_key: String,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DetectorProposalEvidence {
    GlossaryAlias {
        entry: DetectorSessionTermEntry,
        matched_form: String,
    },
    ObservedErrorForm {
        entry: DetectorSessionTermEntry,
        matched_form: String,
    },
    PhoneticSimilarity {
        observed_surface: String,
        target_surface: String,
        target_kind: DetectorPhoneticTargetKind,
        canonical_term: String,
        source_representation: DetectorAsciiLatinPhoneticRepresentation,
        target_representation: DetectorAsciiLatinPhoneticRepresentation,
        comparison: DetectorPhoneticComparisonFacts,
        detector_config: DetectorComponentIdentity,
        algorithm: DetectorComponentIdentity,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetectorProposalRecord {
    pub detector_proposal_id: DetectorProposalId,
    pub snapshot_revision: DetectorSnapshotRevisionId,
    pub input_identity: InputIdentityReference,
    pub semantic_key: DetectorProposalSemanticKey,
    pub detector: DetectorComponentIdentity,
    pub source_anchor: DetectorProposalSourceAnchor,
    pub observed_surface: String,
    pub detection_kind: DetectionKind,
    pub evidence: DetectorProposalEvidence,
    pub alternatives: Vec<DetectorProposalAlternative>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectorProposalSnapshotState {
    Draft,
    Frozen,
    Invalidated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetectorProposalSnapshotAssessment {
    pub total_proposal_count: u32,
    pub duplicate_proposal_ids: Vec<DetectorProposalId>,
    pub duplicate_semantic_keys: Vec<DetectorProposalSemanticKey>,
    pub context_mismatch_proposal_ids: Vec<DetectorProposalId>,
    pub detector_not_in_analysis_set: Vec<DetectorProposalId>,
    pub context_consistent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectorSnapshotIdentityError {
    Empty,
    TooLong { len: usize, max: usize },
    InvalidCharacter { character: char },
    PathLikeContent,
    AbsolutePathLike,
    RelativePathLike,
    HomeDirectoryFragment,
    GenerationUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectorComponentIdentityError {
    EmptyField {
        field: &'static str,
    },
    TooLong {
        field: &'static str,
        len: usize,
        max: usize,
    },
    InvalidCharacter {
        field: &'static str,
        character: char,
    },
    PathLikeContent {
        field: &'static str,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionTermsIdentityError {
    MissingPrefix,
    InvalidLength,
    InvalidHexCharacter { character: char },
    UppercaseHexNotCanonical,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectorProposalSourceAnchorError {
    InvalidInputIdentity(InputIdentityValidationError),
    InvalidCueReferenceId(CueReferenceIdError),
    EmptyOrInvertedRange { start_byte: u32, end_byte: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectorProposalAlternativeValidationError {
    DuplicateIndex { alternative_index: u32 },
    NonContiguousIndex { expected: u32, found: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectorProposalEvidenceValidationError {
    EmptyMatchedForm,
    EmptyObservedSurface,
    ObservedSurfaceMismatch,
    ZeroRatioDenominator,
    InconsistentRatioPermille,
    DetectorConfigMismatch,
    AlgorithmMismatch,
    IncompatibleDetectionKind {
        evidence: &'static str,
        detection_kind: DetectionKind,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectorProposalRecordValidationError {
    EmptyObservedSurface,
    InputIdentityMismatch,
    AnchorInputIdentityMismatch,
    SnapshotRevisionMismatch,
    SemanticKeyMismatch,
    ObservedSurfaceAnchorLengthMismatch,
    InvalidSourceAnchor(DetectorProposalSourceAnchorError),
    DetectorNotInAnalysisSet,
    EvidenceValidation(DetectorProposalEvidenceValidationError),
    AlternativeValidation(DetectorProposalAlternativeValidationError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectorAnalysisIdentityValidationError {
    InputIdentityMismatch,
    InvalidInputIdentity(InputIdentityValidationError),
    InvalidSessionTermsIdentity(SessionTermsIdentityError),
    InvalidDetectorConfig(DetectorComponentIdentityError),
    InvalidAlgorithm(DetectorComponentIdentityError),
    InvalidDetectorInSet {
        index: usize,
        error: DetectorComponentIdentityError,
    },
    DuplicateDetectorInSet {
        detector_id: String,
    },
    EmptyDetectorSet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectorProposalSnapshotValidationError {
    MissingSchemaRevision,
    UnsupportedSchemaRevision {
        found: String,
        expected: String,
    },
    InvalidSnapshotRevisionId(DetectorSnapshotIdentityError),
    InvalidProposalId(DetectorSnapshotIdentityError),
    InvalidDetectorOutputArtifactId(DetectorSnapshotIdentityError),
    AnalysisIdentityValidation(DetectorAnalysisIdentityValidationError),
    RecordValidation(DetectorProposalRecordValidationError),
    AssessmentMismatch {
        stored: Box<DetectorProposalSnapshotAssessment>,
        derived: Box<DetectorProposalSnapshotAssessment>,
    },
    SnapshotStateMismatch {
        state: DetectorProposalSnapshotState,
        assessment: Box<DetectorProposalSnapshotAssessment>,
    },
    ZeroFrozenTimestamp,
    RunIdMismatch,
    InputIdentityMismatch,
    CalibrationModeMismatch,
    EnvelopeLifecycleIncompatible {
        lifecycle_state: RunLifecycleState,
    },
    EnvelopeInvalidated,
    EnvelopeValidation(RunEnvelopeValidationError),
    BundleValidation(ArtifactBundleValidationError),
    DetectorOutputArtifactMissing,
    DetectorOutputArtifactMismatch,
    AmbiguousDetectorOutputRole,
}

impl DetectorSnapshotRevisionId {
    pub fn new(value: impl Into<String>) -> Result<Self, DetectorSnapshotIdentityError> {
        let value = value.into();
        validate_identity_value(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn generate() -> Result<Self, DetectorSnapshotIdentityError> {
        use std::time::{SystemTime, UNIX_EPOCH};

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| DetectorSnapshotIdentityError::GenerationUnavailable)?
            .as_nanos();
        Self::new(format!("det-snap-rev-{nanos:x}"))
    }
}

impl DetectorProposalId {
    pub fn new(value: impl Into<String>) -> Result<Self, DetectorSnapshotIdentityError> {
        let value = value.into();
        validate_identity_value(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn generate() -> Result<Self, DetectorSnapshotIdentityError> {
        use std::time::{SystemTime, UNIX_EPOCH};

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| DetectorSnapshotIdentityError::GenerationUnavailable)?
            .as_nanos();
        Self::new(format!("det-prop-{nanos:x}"))
    }
}

impl fmt::Display for DetectorSnapshotRevisionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for DetectorProposalId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl DetectorProposalSourceAnchor {
    pub fn validate(&self) -> Result<(), DetectorProposalSourceAnchorError> {
        validate_input_identity_reference(&self.input_identity)
            .map_err(DetectorProposalSourceAnchorError::InvalidInputIdentity)?;

        CueReferenceId::new(self.cue_id.value())
            .map_err(DetectorProposalSourceAnchorError::InvalidCueReferenceId)?;

        if self.start_byte >= self.end_byte {
            return Err(DetectorProposalSourceAnchorError::EmptyOrInvertedRange {
                start_byte: self.start_byte,
                end_byte: self.end_byte,
            });
        }

        Ok(())
    }
}

impl DetectorComponentIdentity {
    pub fn validate(&self) -> Result<(), DetectorComponentIdentityError> {
        validate_component_token("id", &self.id)?;
        validate_component_token("version", &self.version)?;
        Ok(())
    }
}

impl DetectorAnalysisIdentity {
    pub fn validate(
        &self,
        expected_input: &InputIdentityReference,
    ) -> Result<(), DetectorAnalysisIdentityValidationError> {
        if self.input_identity != *expected_input {
            return Err(DetectorAnalysisIdentityValidationError::InputIdentityMismatch);
        }

        validate_input_identity_reference(&self.input_identity)
            .map_err(DetectorAnalysisIdentityValidationError::InvalidInputIdentity)?;

        validate_session_terms_identity(&self.session_terms_identity)
            .map_err(DetectorAnalysisIdentityValidationError::InvalidSessionTermsIdentity)?;

        self.detector_config
            .validate()
            .map_err(DetectorAnalysisIdentityValidationError::InvalidDetectorConfig)?;

        self.algorithm
            .validate()
            .map_err(DetectorAnalysisIdentityValidationError::InvalidAlgorithm)?;

        if self.detector_set.is_empty() {
            return Err(DetectorAnalysisIdentityValidationError::EmptyDetectorSet);
        }

        let mut seen_detector_ids = HashSet::new();
        for (index, detector) in self.detector_set.iter().enumerate() {
            detector.validate().map_err(|error| {
                DetectorAnalysisIdentityValidationError::InvalidDetectorInSet { index, error }
            })?;
            if !seen_detector_ids.insert(detector.id.clone()) {
                return Err(
                    DetectorAnalysisIdentityValidationError::DuplicateDetectorInSet {
                        detector_id: detector.id.clone(),
                    },
                );
            }
        }

        Ok(())
    }
}

impl DetectorProposalAlternative {
    pub fn validate(&self) -> Result<(), DetectorProposalAlternativeValidationError> {
        Ok(())
    }
}

impl DetectorProposalEvidence {
    pub fn validate(
        &self,
        observed_surface: &str,
        detection_kind: DetectionKind,
        analysis_identity: &DetectorAnalysisIdentity,
    ) -> Result<(), DetectorProposalEvidenceValidationError> {
        match self {
            Self::GlossaryAlias {
                entry: _,
                matched_form,
            } => {
                if matched_form.is_empty() {
                    return Err(DetectorProposalEvidenceValidationError::EmptyMatchedForm);
                }
                if matched_form != observed_surface {
                    return Err(DetectorProposalEvidenceValidationError::ObservedSurfaceMismatch);
                }
                if detection_kind != DetectionKind::GlossaryAliasMatch {
                    return Err(
                        DetectorProposalEvidenceValidationError::IncompatibleDetectionKind {
                            evidence: "glossary_alias",
                            detection_kind,
                        },
                    );
                }
            }
            Self::ObservedErrorForm {
                entry: _,
                matched_form,
            } => {
                if matched_form.is_empty() {
                    return Err(DetectorProposalEvidenceValidationError::EmptyMatchedForm);
                }
                if matched_form != observed_surface {
                    return Err(DetectorProposalEvidenceValidationError::ObservedSurfaceMismatch);
                }
                if detection_kind != DetectionKind::GlossaryAliasMatch {
                    return Err(
                        DetectorProposalEvidenceValidationError::IncompatibleDetectionKind {
                            evidence: "observed_error_form",
                            detection_kind,
                        },
                    );
                }
            }
            Self::PhoneticSimilarity {
                observed_surface: evidence_observed_surface,
                target_surface: _,
                target_kind: _,
                canonical_term: _,
                source_representation: _,
                target_representation: _,
                comparison,
                detector_config,
                algorithm,
            } => {
                if evidence_observed_surface.is_empty() {
                    return Err(DetectorProposalEvidenceValidationError::EmptyObservedSurface);
                }
                if evidence_observed_surface != observed_surface {
                    return Err(DetectorProposalEvidenceValidationError::ObservedSurfaceMismatch);
                }
                if detection_kind != DetectionKind::PhoneticSimilarity {
                    return Err(
                        DetectorProposalEvidenceValidationError::IncompatibleDetectionKind {
                            evidence: "phonetic_similarity",
                            detection_kind,
                        },
                    );
                }
                validate_phonetic_comparison(comparison)?;
                if detector_config != &analysis_identity.detector_config {
                    return Err(DetectorProposalEvidenceValidationError::DetectorConfigMismatch);
                }
                if algorithm != &analysis_identity.algorithm {
                    return Err(DetectorProposalEvidenceValidationError::AlgorithmMismatch);
                }
            }
        }

        Ok(())
    }
}

impl DetectorProposalRecord {
    pub fn derive_semantic_key(&self) -> DetectorProposalSemanticKey {
        DetectorProposalSemanticKey {
            detector_id: self.detector.id.clone(),
            detection_kind: self.detection_kind,
            source_anchor: self.source_anchor.clone(),
        }
    }

    pub fn validate(
        &self,
        expected_snapshot_revision: &DetectorSnapshotRevisionId,
        expected_input: &InputIdentityReference,
        analysis_identity: &DetectorAnalysisIdentity,
    ) -> Result<(), DetectorProposalRecordValidationError> {
        if self.observed_surface.is_empty() {
            return Err(DetectorProposalRecordValidationError::EmptyObservedSurface);
        }

        if self.input_identity != *expected_input {
            return Err(DetectorProposalRecordValidationError::InputIdentityMismatch);
        }

        if self.input_identity != self.source_anchor.input_identity {
            return Err(DetectorProposalRecordValidationError::AnchorInputIdentityMismatch);
        }

        if self.snapshot_revision != *expected_snapshot_revision {
            return Err(DetectorProposalRecordValidationError::SnapshotRevisionMismatch);
        }

        self.source_anchor
            .validate()
            .map_err(DetectorProposalRecordValidationError::InvalidSourceAnchor)?;

        let derived_semantic_key = self.derive_semantic_key();
        if self.semantic_key != derived_semantic_key {
            return Err(DetectorProposalRecordValidationError::SemanticKeyMismatch);
        }

        let anchor_byte_len = self.source_anchor.end_byte - self.source_anchor.start_byte;
        #[allow(clippy::needless_as_bytes)] // anchor ranges are byte offsets, not char counts
        let observed_byte_len = self.observed_surface.as_bytes().len() as u32;
        if observed_byte_len != anchor_byte_len {
            return Err(DetectorProposalRecordValidationError::ObservedSurfaceAnchorLengthMismatch);
        }

        if !analysis_identity
            .detector_set
            .iter()
            .any(|detector| detector == &self.detector)
        {
            return Err(DetectorProposalRecordValidationError::DetectorNotInAnalysisSet);
        }

        self.evidence
            .validate(
                &self.observed_surface,
                self.detection_kind,
                analysis_identity,
            )
            .map_err(DetectorProposalRecordValidationError::EvidenceValidation)?;

        validate_alternatives(&self.alternatives)
            .map_err(DetectorProposalRecordValidationError::AlternativeValidation)?;

        Ok(())
    }
}

impl DetectorProposalSnapshot {
    pub fn derive_assessment(
        snapshot_revision: &DetectorSnapshotRevisionId,
        input_identity: &InputIdentityReference,
        analysis_identity: &DetectorAnalysisIdentity,
        proposals: &[DetectorProposalRecord],
    ) -> Result<DetectorProposalSnapshotAssessment, DetectorProposalSnapshotValidationError> {
        let mut proposal_id_counts: HashMap<DetectorProposalId, u32> = HashMap::new();
        let mut semantic_key_counts: HashMap<String, u32> = HashMap::new();
        let mut context_mismatch_proposal_ids = Vec::new();
        let mut detector_not_in_analysis_set = Vec::new();
        let mut context_consistent = true;

        for proposal in proposals {
            validate_proposal_id(proposal.detector_proposal_id.as_str())
                .map_err(DetectorProposalSnapshotValidationError::InvalidProposalId)?;

            let mut proposal_context_mismatch = false;

            if proposal.input_identity != *input_identity
                || proposal.snapshot_revision != *snapshot_revision
            {
                proposal_context_mismatch = true;
            }

            if !analysis_identity
                .detector_set
                .iter()
                .any(|detector| detector == &proposal.detector)
            {
                detector_not_in_analysis_set.push(proposal.detector_proposal_id.clone());
                proposal_context_mismatch = true;
            }

            if let Err(error) =
                proposal.validate(snapshot_revision, input_identity, analysis_identity)
            {
                match error {
                    DetectorProposalRecordValidationError::InputIdentityMismatch
                    | DetectorProposalRecordValidationError::SnapshotRevisionMismatch
                    | DetectorProposalRecordValidationError::DetectorNotInAnalysisSet => {
                        proposal_context_mismatch = true;
                    }
                    other => {
                        return Err(DetectorProposalSnapshotValidationError::RecordValidation(
                            other,
                        ));
                    }
                }
            }

            if proposal_context_mismatch {
                context_consistent = false;
                context_mismatch_proposal_ids.push(proposal.detector_proposal_id.clone());
            }

            *proposal_id_counts
                .entry(proposal.detector_proposal_id.clone())
                .or_default() += 1;
            *semantic_key_counts
                .entry(semantic_key_string(&proposal.semantic_key))
                .or_default() += 1;
        }

        let total_proposal_count = proposals.len() as u32;

        let mut duplicate_proposal_ids = proposal_id_counts
            .iter()
            .filter_map(|(id, count)| (*count > 1).then_some(id.clone()))
            .collect::<Vec<_>>();
        duplicate_proposal_ids.sort_by(|left, right| left.as_str().cmp(right.as_str()));

        let mut duplicate_semantic_keys = Vec::new();
        let mut seen_duplicate_semantic_keys = HashSet::new();
        for proposal in proposals {
            let key = semantic_key_string(&proposal.semantic_key);
            if semantic_key_counts.get(&key).copied().unwrap_or(0) > 1
                && seen_duplicate_semantic_keys.insert(key)
            {
                duplicate_semantic_keys.push(proposal.semantic_key.clone());
            }
        }
        duplicate_semantic_keys.sort_by(cmp_semantic_keys);

        context_mismatch_proposal_ids.sort_by(|left, right| left.as_str().cmp(right.as_str()));
        detector_not_in_analysis_set.sort_by(|left, right| left.as_str().cmp(right.as_str()));

        Ok(DetectorProposalSnapshotAssessment {
            total_proposal_count,
            duplicate_proposal_ids,
            duplicate_semantic_keys,
            context_mismatch_proposal_ids,
            detector_not_in_analysis_set,
            context_consistent,
        })
    }

    pub fn validate(&self) -> Result<(), DetectorProposalSnapshotValidationError> {
        if self.schema_revision.is_empty() {
            return Err(DetectorProposalSnapshotValidationError::MissingSchemaRevision);
        }

        if self.schema_revision != DETECTOR_PROPOSAL_SNAPSHOT_SCHEMA {
            return Err(
                DetectorProposalSnapshotValidationError::UnsupportedSchemaRevision {
                    found: self.schema_revision.clone(),
                    expected: DETECTOR_PROPOSAL_SNAPSHOT_SCHEMA.to_string(),
                },
            );
        }

        validate_identity_value(self.snapshot_revision.as_str())
            .map_err(DetectorProposalSnapshotValidationError::InvalidSnapshotRevisionId)?;

        validate_identity_value(self.detector_output_artifact_id.as_str())
            .map_err(DetectorProposalSnapshotValidationError::InvalidDetectorOutputArtifactId)?;

        validate_opaque_identifier(self.run_id.as_str()).map_err(|error| {
            DetectorProposalSnapshotValidationError::InvalidSnapshotRevisionId(map_run_id_error(
                error,
            ))
        })?;

        self.analysis_identity
            .validate(&self.input_identity)
            .map_err(DetectorProposalSnapshotValidationError::AnalysisIdentityValidation)?;

        let derived = Self::derive_assessment(
            &self.snapshot_revision,
            &self.input_identity,
            &self.analysis_identity,
            &self.proposals,
        )?;

        if self.assessment != derived {
            return Err(
                DetectorProposalSnapshotValidationError::AssessmentMismatch {
                    stored: Box::new(self.assessment.clone()),
                    derived: Box::new(derived.clone()),
                },
            );
        }

        if self.state == DetectorProposalSnapshotState::Frozen {
            if self.frozen_at_unix_ms == 0 {
                return Err(DetectorProposalSnapshotValidationError::ZeroFrozenTimestamp);
            }

            if !derived.context_consistent
                || !derived.duplicate_proposal_ids.is_empty()
                || !derived.duplicate_semantic_keys.is_empty()
            {
                return Err(
                    DetectorProposalSnapshotValidationError::SnapshotStateMismatch {
                        state: self.state,
                        assessment: Box::new(derived),
                    },
                );
            }
        }

        Ok(())
    }

    pub fn validate_for_freeze_against(
        &self,
        envelope: &RunEnvelope,
    ) -> Result<(), DetectorProposalSnapshotValidationError> {
        self.validate()?;

        envelope
            .validate()
            .map_err(DetectorProposalSnapshotValidationError::EnvelopeValidation)?;

        if envelope.lifecycle_state == RunLifecycleState::Invalidated {
            return Err(DetectorProposalSnapshotValidationError::EnvelopeInvalidated);
        }

        if self.state != DetectorProposalSnapshotState::Frozen {
            return Err(
                DetectorProposalSnapshotValidationError::SnapshotStateMismatch {
                    state: self.state,
                    assessment: Box::new(self.assessment.clone()),
                },
            );
        }

        if self.frozen_at_unix_ms == 0 {
            return Err(DetectorProposalSnapshotValidationError::ZeroFrozenTimestamp);
        }

        if envelope.lifecycle_state != RunLifecycleState::DetectorExecution {
            return Err(
                DetectorProposalSnapshotValidationError::EnvelopeLifecycleIncompatible {
                    lifecycle_state: envelope.lifecycle_state,
                },
            );
        }

        validate_envelope_binding(self, envelope)
    }

    pub fn validate_context_against(
        &self,
        envelope: &RunEnvelope,
    ) -> Result<(), DetectorProposalSnapshotValidationError> {
        self.validate()?;

        envelope
            .validate()
            .map_err(DetectorProposalSnapshotValidationError::EnvelopeValidation)?;

        if envelope.lifecycle_state == RunLifecycleState::Invalidated {
            return Err(DetectorProposalSnapshotValidationError::EnvelopeInvalidated);
        }

        if !lifecycle_accepts_snapshot_context(envelope.lifecycle_state) {
            return Err(
                DetectorProposalSnapshotValidationError::EnvelopeLifecycleIncompatible {
                    lifecycle_state: envelope.lifecycle_state,
                },
            );
        }

        validate_envelope_binding(self, envelope)
    }

    pub fn validate_against_bundle(
        &self,
        envelope: &RunEnvelope,
        bundle: &ArtifactBundle,
    ) -> Result<(), DetectorProposalSnapshotValidationError> {
        self.validate_context_against(envelope)?;

        bundle
            .validate_against_envelope(envelope)
            .map_err(DetectorProposalSnapshotValidationError::BundleValidation)?;

        let detector_outputs = bundle
            .artifacts
            .iter()
            .filter(|descriptor| {
                descriptor.role == crate::run_manifest::ArtifactRole::DetectorOutput
            })
            .collect::<Vec<_>>();

        if detector_outputs.is_empty() {
            return Err(DetectorProposalSnapshotValidationError::DetectorOutputArtifactMissing);
        }

        if detector_outputs.len() != 1 {
            return Err(DetectorProposalSnapshotValidationError::AmbiguousDetectorOutputRole);
        }

        let descriptor = detector_outputs[0];
        if descriptor.artifact_id != self.detector_output_artifact_id {
            return Err(DetectorProposalSnapshotValidationError::DetectorOutputArtifactMismatch);
        }

        Ok(())
    }
}

fn validate_envelope_binding(
    snapshot: &DetectorProposalSnapshot,
    envelope: &RunEnvelope,
) -> Result<(), DetectorProposalSnapshotValidationError> {
    if snapshot.run_id != envelope.run_id {
        return Err(DetectorProposalSnapshotValidationError::RunIdMismatch);
    }

    if snapshot.input_identity != envelope.input_identity {
        return Err(DetectorProposalSnapshotValidationError::InputIdentityMismatch);
    }

    if snapshot.calibration_validity != envelope.calibration_validity {
        return Err(DetectorProposalSnapshotValidationError::CalibrationModeMismatch);
    }

    Ok(())
}

fn lifecycle_accepts_snapshot_context(state: RunLifecycleState) -> bool {
    matches!(
        state,
        RunLifecycleState::DetectorExecution
            | RunLifecycleState::AssistedReview
            | RunLifecycleState::Finalized
    )
}

fn validate_alternatives(
    alternatives: &[DetectorProposalAlternative],
) -> Result<(), DetectorProposalAlternativeValidationError> {
    for (expected_index, alternative) in alternatives.iter().enumerate() {
        let expected_index = expected_index as u32;
        if alternative.alternative_index != expected_index {
            return Err(
                DetectorProposalAlternativeValidationError::NonContiguousIndex {
                    expected: expected_index,
                    found: alternative.alternative_index,
                },
            );
        }
        alternative.validate()?;
    }

    let mut seen = HashSet::new();
    for alternative in alternatives {
        if !seen.insert(alternative.alternative_index) {
            return Err(DetectorProposalAlternativeValidationError::DuplicateIndex {
                alternative_index: alternative.alternative_index,
            });
        }
    }

    Ok(())
}

fn validate_phonetic_comparison(
    comparison: &DetectorPhoneticComparisonFacts,
) -> Result<(), DetectorProposalEvidenceValidationError> {
    if comparison.ratio_denominator == 0 {
        return Err(DetectorProposalEvidenceValidationError::ZeroRatioDenominator);
    }

    let expected_permille =
        comparison.ratio_numerator as u64 * 1000 / comparison.ratio_denominator as u64;
    if comparison.ratio_permille as u64 != expected_permille {
        return Err(DetectorProposalEvidenceValidationError::InconsistentRatioPermille);
    }

    Ok(())
}

fn validate_session_terms_identity(value: &str) -> Result<(), SessionTermsIdentityError> {
    if !value.starts_with(SESSION_TERMS_IDENTITY_PREFIX) {
        return Err(SessionTermsIdentityError::MissingPrefix);
    }

    let digest = &value[SESSION_TERMS_IDENTITY_PREFIX.len()..];
    if digest.len() != 64 {
        return Err(SessionTermsIdentityError::InvalidLength);
    }

    for character in digest.chars() {
        if character.is_ascii_digit() || matches!(character, 'a'..='f') {
            continue;
        }
        if character.is_ascii_hexdigit() {
            return Err(SessionTermsIdentityError::UppercaseHexNotCanonical);
        }
        return Err(SessionTermsIdentityError::InvalidHexCharacter { character });
    }

    Ok(())
}

fn validate_component_token(
    field: &'static str,
    value: &str,
) -> Result<(), DetectorComponentIdentityError> {
    if value.is_empty() {
        return Err(DetectorComponentIdentityError::EmptyField { field });
    }

    if value.len() > COMPONENT_TOKEN_MAX_LEN {
        return Err(DetectorComponentIdentityError::TooLong {
            field,
            len: value.len(),
            max: COMPONENT_TOKEN_MAX_LEN,
        });
    }

    if value.contains('/') || value.contains('\\') {
        return Err(DetectorComponentIdentityError::PathLikeContent { field });
    }

    for character in value.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
            continue;
        }
        return Err(DetectorComponentIdentityError::InvalidCharacter { field, character });
    }

    Ok(())
}

fn validate_identity_value(value: &str) -> Result<(), DetectorSnapshotIdentityError> {
    validate_opaque_identifier(value).map_err(map_run_id_error)
}

fn validate_proposal_id(value: &str) -> Result<(), DetectorSnapshotIdentityError> {
    validate_identity_value(value)
}

fn semantic_key_string(key: &DetectorProposalSemanticKey) -> String {
    format!(
        "{}:{}:{}:{}:{}:{}:{}",
        key.detector_id,
        detection_kind_name(key.detection_kind),
        key.source_anchor.input_identity.transcript_revision_id,
        key.source_anchor.cue_id.value(),
        key.source_anchor.segment_position,
        key.source_anchor.start_byte,
        key.source_anchor.end_byte
    )
}

fn detection_kind_name(kind: DetectionKind) -> &'static str {
    match kind {
        DetectionKind::GlossaryAliasMatch => "glossary_alias_match",
        DetectionKind::MixedLanguageAnomaly => "mixed_language_anomaly",
        DetectionKind::PhoneticSimilarity => "phonetic_similarity",
        DetectionKind::RepeatedPhrase => "repeated_phrase",
    }
}

fn cmp_semantic_keys(
    left: &DetectorProposalSemanticKey,
    right: &DetectorProposalSemanticKey,
) -> std::cmp::Ordering {
    left.detector_id
        .cmp(&right.detector_id)
        .then_with(|| {
            detection_kind_name(left.detection_kind).cmp(detection_kind_name(right.detection_kind))
        })
        .then_with(|| {
            left.source_anchor
                .input_identity
                .transcript_revision_id
                .cmp(&right.source_anchor.input_identity.transcript_revision_id)
        })
        .then_with(|| {
            left.source_anchor
                .cue_id
                .value()
                .cmp(&right.source_anchor.cue_id.value())
        })
        .then_with(|| {
            left.source_anchor
                .segment_position
                .cmp(&right.source_anchor.segment_position)
        })
        .then_with(|| {
            left.source_anchor
                .start_byte
                .cmp(&right.source_anchor.start_byte)
        })
        .then_with(|| {
            left.source_anchor
                .end_byte
                .cmp(&right.source_anchor.end_byte)
        })
}

fn map_run_id_error(error: RunIdError) -> DetectorSnapshotIdentityError {
    match error {
        RunIdError::Empty => DetectorSnapshotIdentityError::Empty,
        RunIdError::TooLong { len, max } => DetectorSnapshotIdentityError::TooLong { len, max },
        RunIdError::InvalidCharacter { character } => {
            DetectorSnapshotIdentityError::InvalidCharacter { character }
        }
        RunIdError::PathLikeContent => DetectorSnapshotIdentityError::PathLikeContent,
        RunIdError::AbsolutePathLike => DetectorSnapshotIdentityError::AbsolutePathLike,
        RunIdError::RelativePathLike => DetectorSnapshotIdentityError::RelativePathLike,
        RunIdError::HomeDirectoryFragment => DetectorSnapshotIdentityError::HomeDirectoryFragment,
        RunIdError::GenerationUnavailable => DetectorSnapshotIdentityError::GenerationUnavailable,
    }
}

pub fn detector_proposal_snapshot_from_json(
    json: &str,
) -> Result<DetectorProposalSnapshot, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn detector_proposal_snapshot_to_json(
    snapshot: &DetectorProposalSnapshot,
) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(snapshot)
}

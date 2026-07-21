use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::human_final_reference::{HumanFinalReference, HumanFinalReferenceValidationError};
use crate::reference_alignment::{
    cue_id_for_segment_position, validate_completion_record_mapping,
    validate_coverage_against_human_reference,
};
use crate::reference_identity::{
    CueSourceTextDigest, CueSourceTextDigestError, ReferenceIdentityIdError,
    ReferenceReviewerIdentityClass, ReferenceRevisionId, VerificationBasis,
    validate_identity_value,
};
use crate::reference_seal::{
    CalibrationValidityImpact, ReferenceCalibrationValidity, ReferenceSeal, ReferenceSealId,
    ReferenceSealState, ReferenceSealValidationError,
};
use crate::run_manifest::{
    CalibrationValidityMode, InputIdentityReference, RunEnvelope, RunEnvelopeValidationError,
    RunId, RunIdError, RunLifecycleState, validate_opaque_identifier,
};

pub const REFERENCE_COVERAGE_SCHEMA: &str = "voxproof-per-cue-reference-coverage-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceCoverage {
    pub schema_revision: String,
    pub coverage_id: ReferenceCoverageId,
    pub run_id: RunId,
    pub input_identity: InputIdentityReference,
    pub seal_id: ReferenceSealId,
    pub reference_revision: ReferenceRevisionId,
    pub coverage_purpose: ReferenceCoveragePurpose,
    pub expected_universe: ExpectedCueUniverse,
    pub records: Vec<CueReviewCompletionRecord>,
    pub coverage_state: ReferenceCoverageState,
    pub assessment: ReferenceCoverageAssessment,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReferenceCoverageId(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CueReferenceId(u32);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedCueUniverse {
    pub total_cues: u32,
    pub cue_ids: Vec<CueReferenceId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CueReviewCompletionRecord {
    pub cue_id: CueReferenceId,
    pub segment_position: u32,
    pub source_text_digest: CueSourceTextDigest,
    pub disposition: ReferenceCueDisposition,
    pub fully_reviewed: bool,
    pub all_known_transcription_errors_enumerated: bool,
    pub verification_source_used: VerificationBasis,
    pub reviewer_identity_class: ReferenceReviewerIdentityClass,
    pub completed_at_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceCoveragePurpose {
    PrimaryBlindCalibration,
    DiagnosticOnly,
    SyntheticProtocolValidation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceCueDisposition {
    NoTranscriptionError,
    TranscriptionError,
    Uncertain,
    Unreviewable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceCoverageState {
    Draft,
    Complete,
    Invalidated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceCoverageAssessment {
    pub expected_count: u32,
    pub observed_unique_count: u32,
    pub completed_cue_count: u32,
    pub incomplete_cue_count: u32,
    pub cues_with_transcription_errors: u32,
    pub total_eligible_transcription_errors: u32,
    pub missing_cue_ids: Vec<CueReferenceId>,
    pub duplicate_cue_ids: Vec<CueReferenceId>,
    pub duplicate_segment_positions: Vec<u32>,
    pub unknown_cue_ids: Vec<CueReferenceId>,
    pub invalid_mapping_cue_ids: Vec<CueReferenceId>,
    pub unresolved_cue_ids: Vec<CueReferenceId>,
    pub incomplete_attestation_cue_ids: Vec<CueReferenceId>,
    pub inventory_complete: bool,
    pub reference_resolved: bool,
    pub coverage_complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceCoverageIdError {
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
pub enum CueReferenceIdError {
    ZeroNotPermitted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceCoverageValidationError {
    MissingSchemaRevision,
    UnsupportedSchemaRevision {
        found: String,
        expected: String,
    },
    InvalidCoverageId(ReferenceCoverageIdError),
    InvalidReferenceRevisionId(ReferenceIdentityIdError),
    InvalidCueReferenceId(CueReferenceIdError),
    InvalidSourceTextDigest(CueSourceTextDigestError),
    ZeroCompletionTimestamp,
    EmptyExpectedUniverse,
    DuplicateExpectedCueId {
        cue_id: CueReferenceId,
    },
    ExpectedCountMismatch {
        total_cues: u32,
        unique_count: usize,
    },
    DuplicateObservedCueId {
        cue_id: CueReferenceId,
    },
    UnknownObservedCueId {
        cue_id: CueReferenceId,
    },
    MissingExpectedCueId {
        cue_id: CueReferenceId,
    },
    SegmentPositionOutOfRange {
        segment_position: u32,
    },
    DuplicateSegmentPosition {
        segment_position: u32,
    },
    CueMappingMismatch {
        cue_id: CueReferenceId,
        segment_position: u32,
        expected_cue_id: CueReferenceId,
    },
    AssessmentMismatch {
        stored: Box<ReferenceCoverageAssessment>,
        derived: Box<ReferenceCoverageAssessment>,
    },
    CoverageStateMismatch {
        state: ReferenceCoverageState,
        assessment: Box<ReferenceCoverageAssessment>,
    },
    PrimaryAttachmentRequiresCompleteState,
    HumanReferenceRequiredForCompleteCoverage,
    RunIdMismatch,
    InputIdentityMismatch,
    SealIdMismatch,
    ReferenceRevisionMismatch,
    EnvelopeNotBlindReference,
    EnvelopeLifecycleIncompatible {
        lifecycle_state: RunLifecycleState,
    },
    SealStateIncompatible {
        seal_state: ReferenceSealState,
    },
    SealClassificationIncompatible {
        purpose: ReferenceCoveragePurpose,
        classification: ReferenceCalibrationValidity,
    },
    SealValidityImpactIncompatible {
        purpose: ReferenceCoveragePurpose,
        impact: CalibrationValidityImpact,
    },
    UnknownReferenceCueId {
        cue_id: CueReferenceId,
    },
    ReferenceAnchorMappingMismatch {
        cue_id: CueReferenceId,
        segment_position: u32,
    },
    TranscriptionErrorCueMissingRecord {
        cue_id: CueReferenceId,
    },
    NoTranscriptionErrorCueHasRecord {
        cue_id: CueReferenceId,
    },
    TranscriptionErrorRecordForUnresolvedCue {
        cue_id: CueReferenceId,
    },
    EligibleTranscriptionErrorCountMismatch {
        stored: u32,
        derived: u32,
    },
    EnvelopeValidation(RunEnvelopeValidationError),
    SealValidation(ReferenceSealValidationError),
    HumanReferenceValidation(Box<HumanFinalReferenceValidationError>),
}

impl CueReferenceId {
    pub fn new(value: u32) -> Result<Self, CueReferenceIdError> {
        if value == 0 {
            return Err(CueReferenceIdError::ZeroNotPermitted);
        }
        Ok(Self(value))
    }

    pub fn value(self) -> u32 {
        self.0
    }
}

impl ReferenceCoverageId {
    pub fn new(value: impl Into<String>) -> Result<Self, ReferenceCoverageIdError> {
        let value = value.into();
        validate_coverage_id_value(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn generate() -> Result<Self, ReferenceCoverageIdError> {
        use std::time::{SystemTime, UNIX_EPOCH};

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ReferenceCoverageIdError::GenerationUnavailable)?
            .as_nanos();
        Self::new(format!("coverage-{nanos:x}"))
    }
}

impl fmt::Display for ReferenceCoverageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl CueReviewCompletionRecord {
    pub fn validate_fields(&self) -> Result<(), ReferenceCoverageValidationError> {
        validate_cue_reference_id(self.cue_id)?;
        CueSourceTextDigest::new(self.source_text_digest.as_str())
            .map_err(ReferenceCoverageValidationError::InvalidSourceTextDigest)?;

        if (self.fully_reviewed || self.all_known_transcription_errors_enumerated)
            && self.completed_at_unix_ms == 0
        {
            return Err(ReferenceCoverageValidationError::ZeroCompletionTimestamp);
        }

        Ok(())
    }

    fn is_recall_complete(&self) -> bool {
        self.fully_reviewed && self.all_known_transcription_errors_enumerated
    }

    fn is_unresolved_disposition(&self) -> bool {
        matches!(
            self.disposition,
            ReferenceCueDisposition::Uncertain | ReferenceCueDisposition::Unreviewable
        )
    }
}

impl ReferenceCoverage {
    pub fn derive_assessment(
        expected_universe: &ExpectedCueUniverse,
        records: &[CueReviewCompletionRecord],
    ) -> Result<ReferenceCoverageAssessment, ReferenceCoverageValidationError> {
        validate_expected_universe(expected_universe)?;

        let expected_ids: BTreeSet<CueReferenceId> =
            expected_universe.cue_ids.iter().copied().collect();

        let mut counts: HashMap<CueReferenceId, u32> = HashMap::new();
        let mut segment_counts: HashMap<u32, u32> = HashMap::new();
        let mut invalid_mapping_cue_ids = Vec::new();
        let mut incomplete_attestation_cue_ids = Vec::new();
        let mut unresolved_cue_ids = Vec::new();
        let mut cues_with_transcription_errors = 0u32;
        let mut completed_cue_count = 0u32;

        for record in records {
            record.validate_fields()?;
            if let Err(error) = validate_completion_record_mapping(expected_universe, record) {
                match error {
                    ReferenceCoverageValidationError::CueMappingMismatch { .. }
                    | ReferenceCoverageValidationError::SegmentPositionOutOfRange { .. } => {
                        if !invalid_mapping_cue_ids.contains(&record.cue_id) {
                            invalid_mapping_cue_ids.push(record.cue_id);
                        }
                    }
                    other => return Err(other),
                }
            }

            *counts.entry(record.cue_id).or_default() += 1;
            *segment_counts.entry(record.segment_position).or_default() += 1;

            if record.disposition == ReferenceCueDisposition::TranscriptionError {
                cues_with_transcription_errors += 1;
            }

            if record.is_unresolved_disposition() && !unresolved_cue_ids.contains(&record.cue_id) {
                unresolved_cue_ids.push(record.cue_id);
            }

            let attestation_complete =
                record.is_recall_complete() && !record.is_unresolved_disposition();
            if !attestation_complete && !incomplete_attestation_cue_ids.contains(&record.cue_id) {
                incomplete_attestation_cue_ids.push(record.cue_id);
            }
            if attestation_complete {
                completed_cue_count += 1;
            }
        }

        invalid_mapping_cue_ids.sort_unstable();
        incomplete_attestation_cue_ids.sort_unstable();
        unresolved_cue_ids.sort_unstable();

        let mut duplicate_cue_ids = counts
            .iter()
            .filter_map(|(cue_id, count)| (*count > 1).then_some(*cue_id))
            .collect::<Vec<_>>();
        duplicate_cue_ids.sort_unstable();

        let mut duplicate_segment_positions = segment_counts
            .iter()
            .filter_map(|(segment_position, count)| (*count > 1).then_some(*segment_position))
            .collect::<Vec<_>>();
        duplicate_segment_positions.sort_unstable();

        let mut unknown_cue_ids = counts
            .keys()
            .filter(|cue_id| !expected_ids.contains(cue_id))
            .copied()
            .collect::<Vec<_>>();
        unknown_cue_ids.sort_unstable();

        let observed_unique: BTreeSet<CueReferenceId> = counts
            .keys()
            .filter(|cue_id| expected_ids.contains(cue_id))
            .copied()
            .collect();

        let missing_cue_ids = expected_ids
            .difference(&observed_unique)
            .copied()
            .collect::<Vec<_>>();

        let mut out_of_range_positions = Vec::new();
        for record in records {
            if cue_id_for_segment_position(expected_universe, record.segment_position).is_none()
                && !out_of_range_positions.contains(&record.segment_position)
            {
                out_of_range_positions.push(record.segment_position);
            }
        }

        let inventory_complete = missing_cue_ids.is_empty()
            && duplicate_cue_ids.is_empty()
            && unknown_cue_ids.is_empty()
            && duplicate_segment_positions.is_empty()
            && invalid_mapping_cue_ids.is_empty()
            && out_of_range_positions.is_empty()
            && observed_unique.len() == expected_ids.len();

        let reference_resolved = inventory_complete && unresolved_cue_ids.is_empty();

        let expected_count = expected_universe.total_cues;
        let incomplete_cue_count = expected_count.saturating_sub(completed_cue_count);

        let coverage_complete = inventory_complete
            && reference_resolved
            && completed_cue_count == expected_count
            && incomplete_cue_count == 0
            && incomplete_attestation_cue_ids.is_empty();

        Ok(ReferenceCoverageAssessment {
            expected_count,
            observed_unique_count: observed_unique.len() as u32,
            completed_cue_count,
            incomplete_cue_count,
            cues_with_transcription_errors,
            total_eligible_transcription_errors: 0,
            missing_cue_ids,
            duplicate_cue_ids,
            duplicate_segment_positions,
            unknown_cue_ids,
            invalid_mapping_cue_ids,
            unresolved_cue_ids,
            incomplete_attestation_cue_ids,
            inventory_complete,
            reference_resolved,
            coverage_complete,
        })
    }

    pub fn validate(&self) -> Result<(), ReferenceCoverageValidationError> {
        if self.schema_revision.is_empty() {
            return Err(ReferenceCoverageValidationError::MissingSchemaRevision);
        }

        if self.schema_revision != REFERENCE_COVERAGE_SCHEMA {
            return Err(
                ReferenceCoverageValidationError::UnsupportedSchemaRevision {
                    found: self.schema_revision.clone(),
                    expected: REFERENCE_COVERAGE_SCHEMA.to_string(),
                },
            );
        }

        validate_coverage_id_value(self.coverage_id.as_str())
            .map_err(ReferenceCoverageValidationError::InvalidCoverageId)?;

        validate_identity_value(self.reference_revision.as_str())
            .map_err(ReferenceCoverageValidationError::InvalidReferenceRevisionId)?;

        validate_expected_universe(&self.expected_universe)?;

        let derived = Self::derive_assessment(&self.expected_universe, &self.records)?;
        if assessment_without_eligible_count(&self.assessment)
            != assessment_without_eligible_count(&derived)
        {
            return Err(ReferenceCoverageValidationError::AssessmentMismatch {
                stored: Box::new(self.assessment.clone()),
                derived: Box::new(derived.clone()),
            });
        }

        if self.coverage_state == ReferenceCoverageState::Complete && !derived.coverage_complete {
            return Err(ReferenceCoverageValidationError::CoverageStateMismatch {
                state: self.coverage_state,
                assessment: Box::new(derived),
            });
        }

        Ok(())
    }

    pub fn validate_against(
        &self,
        envelope: &RunEnvelope,
        seal: &ReferenceSeal,
        human_reference: Option<&HumanFinalReference>,
    ) -> Result<(), ReferenceCoverageValidationError> {
        self.validate()?;

        envelope
            .validate()
            .map_err(ReferenceCoverageValidationError::EnvelopeValidation)?;

        seal.validate()
            .map_err(ReferenceCoverageValidationError::SealValidation)?;

        if self.run_id != envelope.run_id || self.run_id != seal.run_id {
            return Err(ReferenceCoverageValidationError::RunIdMismatch);
        }

        if self.input_identity != envelope.input_identity
            || self.input_identity != seal.input_identity
        {
            return Err(ReferenceCoverageValidationError::InputIdentityMismatch);
        }

        if self.seal_id != seal.seal_id {
            return Err(ReferenceCoverageValidationError::SealIdMismatch);
        }

        if self.reference_revision != seal.reference_revision {
            return Err(ReferenceCoverageValidationError::ReferenceRevisionMismatch);
        }

        if envelope.calibration_validity != CalibrationValidityMode::BlindReference {
            return Err(ReferenceCoverageValidationError::EnvelopeNotBlindReference);
        }

        validate_attachment_for_purpose(self, envelope, seal)?;

        if self.coverage_state == ReferenceCoverageState::Complete
            && self.coverage_purpose == ReferenceCoveragePurpose::PrimaryBlindCalibration
            && human_reference.is_none()
        {
            return Err(
                ReferenceCoverageValidationError::HumanReferenceRequiredForCompleteCoverage,
            );
        }

        if let Some(human_reference) = human_reference {
            human_reference
                .validate_against(envelope, seal)
                .map_err(|error| {
                    ReferenceCoverageValidationError::HumanReferenceValidation(Box::new(error))
                })?;
            validate_coverage_against_human_reference(self, human_reference)?;
        } else if self.assessment.total_eligible_transcription_errors != 0 {
            return Err(
                ReferenceCoverageValidationError::EligibleTranscriptionErrorCountMismatch {
                    stored: self.assessment.total_eligible_transcription_errors,
                    derived: 0,
                },
            );
        }

        Ok(())
    }
}

fn assessment_without_eligible_count(
    assessment: &ReferenceCoverageAssessment,
) -> ReferenceCoverageAssessment {
    let mut normalized = assessment.clone();
    normalized.total_eligible_transcription_errors = 0;
    normalized
}

fn validate_attachment_for_purpose(
    coverage: &ReferenceCoverage,
    envelope: &RunEnvelope,
    seal: &ReferenceSeal,
) -> Result<(), ReferenceCoverageValidationError> {
    if seal.seal_state != ReferenceSealState::Sealed {
        return Err(ReferenceCoverageValidationError::SealStateIncompatible {
            seal_state: seal.seal_state,
        });
    }

    match coverage.coverage_purpose {
        ReferenceCoveragePurpose::PrimaryBlindCalibration => {
            if coverage.coverage_state != ReferenceCoverageState::Complete {
                return Err(
                    ReferenceCoverageValidationError::PrimaryAttachmentRequiresCompleteState,
                );
            }

            if envelope.lifecycle_state != RunLifecycleState::ReferenceSealed {
                return Err(
                    ReferenceCoverageValidationError::EnvelopeLifecycleIncompatible {
                        lifecycle_state: envelope.lifecycle_state,
                    },
                );
            }

            if seal.calibration_classification
                != ReferenceCalibrationValidity::BlindReferenceEligible
            {
                return Err(
                    ReferenceCoverageValidationError::SealClassificationIncompatible {
                        purpose: coverage.coverage_purpose,
                        classification: seal.calibration_classification,
                    },
                );
            }

            if seal.calibration_validity_impact != CalibrationValidityImpact::None {
                return Err(
                    ReferenceCoverageValidationError::SealValidityImpactIncompatible {
                        purpose: coverage.coverage_purpose,
                        impact: seal.calibration_validity_impact,
                    },
                );
            }
        }
        ReferenceCoveragePurpose::DiagnosticOnly => {
            if !matches!(
                envelope.lifecycle_state,
                RunLifecycleState::ReferencePreparation | RunLifecycleState::ReferenceSealed
            ) {
                return Err(
                    ReferenceCoverageValidationError::EnvelopeLifecycleIncompatible {
                        lifecycle_state: envelope.lifecycle_state,
                    },
                );
            }

            if !matches!(
                seal.calibration_classification,
                ReferenceCalibrationValidity::TermConditionedDiagnostic
                    | ReferenceCalibrationValidity::DetectorContaminated
                    | ReferenceCalibrationValidity::Invalid
            ) {
                return Err(
                    ReferenceCoverageValidationError::SealClassificationIncompatible {
                        purpose: coverage.coverage_purpose,
                        classification: seal.calibration_classification,
                    },
                );
            }
        }
        ReferenceCoveragePurpose::SyntheticProtocolValidation => {
            if !matches!(
                envelope.lifecycle_state,
                RunLifecycleState::ReferencePreparation | RunLifecycleState::ReferenceSealed
            ) {
                return Err(
                    ReferenceCoverageValidationError::EnvelopeLifecycleIncompatible {
                        lifecycle_state: envelope.lifecycle_state,
                    },
                );
            }

            if seal.calibration_classification
                != ReferenceCalibrationValidity::SyntheticProtocolOnly
            {
                return Err(
                    ReferenceCoverageValidationError::SealClassificationIncompatible {
                        purpose: coverage.coverage_purpose,
                        classification: seal.calibration_classification,
                    },
                );
            }
        }
    }

    if matches!(
        envelope.lifecycle_state,
        RunLifecycleState::DetectorExecution
            | RunLifecycleState::AssistedReview
            | RunLifecycleState::Finalized
            | RunLifecycleState::Invalidated
            | RunLifecycleState::Declared
    ) {
        return Err(
            ReferenceCoverageValidationError::EnvelopeLifecycleIncompatible {
                lifecycle_state: envelope.lifecycle_state,
            },
        );
    }

    Ok(())
}

fn validate_expected_universe(
    universe: &ExpectedCueUniverse,
) -> Result<(), ReferenceCoverageValidationError> {
    if universe.cue_ids.is_empty() {
        return Err(ReferenceCoverageValidationError::EmptyExpectedUniverse);
    }

    let mut seen = HashSet::new();
    for cue_id in &universe.cue_ids {
        validate_cue_reference_id(*cue_id)?;
        if !seen.insert(*cue_id) {
            return Err(ReferenceCoverageValidationError::DuplicateExpectedCueId {
                cue_id: *cue_id,
            });
        }
    }

    if universe.total_cues as usize != universe.cue_ids.len() {
        return Err(ReferenceCoverageValidationError::ExpectedCountMismatch {
            total_cues: universe.total_cues,
            unique_count: universe.cue_ids.len(),
        });
    }

    Ok(())
}

fn validate_cue_reference_id(
    cue_id: CueReferenceId,
) -> Result<(), ReferenceCoverageValidationError> {
    CueReferenceId::new(cue_id.value())
        .map(|_| ())
        .map_err(ReferenceCoverageValidationError::InvalidCueReferenceId)
}

fn validate_coverage_id_value(value: &str) -> Result<(), ReferenceCoverageIdError> {
    validate_opaque_identifier(value).map_err(map_run_id_error)
}

fn map_run_id_error(error: RunIdError) -> ReferenceCoverageIdError {
    match error {
        RunIdError::Empty => ReferenceCoverageIdError::Empty,
        RunIdError::TooLong { len, max } => ReferenceCoverageIdError::TooLong { len, max },
        RunIdError::InvalidCharacter { character } => {
            ReferenceCoverageIdError::InvalidCharacter { character }
        }
        RunIdError::PathLikeContent => ReferenceCoverageIdError::PathLikeContent,
        RunIdError::AbsolutePathLike => ReferenceCoverageIdError::AbsolutePathLike,
        RunIdError::RelativePathLike => ReferenceCoverageIdError::RelativePathLike,
        RunIdError::HomeDirectoryFragment => ReferenceCoverageIdError::HomeDirectoryFragment,
        RunIdError::GenerationUnavailable => ReferenceCoverageIdError::GenerationUnavailable,
    }
}

pub fn coverage_from_json(json: &str) -> Result<ReferenceCoverage, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn coverage_to_json(coverage: &ReferenceCoverage) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(coverage)
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    const SAMPLE_DIGEST: &str =
        "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn completion_record(
        cue_id: u32,
        segment_position: u32,
        disposition: ReferenceCueDisposition,
    ) -> CueReviewCompletionRecord {
        CueReviewCompletionRecord {
            cue_id: CueReferenceId::new(cue_id).expect("cue"),
            segment_position,
            source_text_digest: CueSourceTextDigest::new(SAMPLE_DIGEST).expect("digest"),
            disposition,
            fully_reviewed: true,
            all_known_transcription_errors_enumerated: true,
            verification_source_used: VerificationBasis::AudioListened,
            reviewer_identity_class: ReferenceReviewerIdentityClass::OwnerBlindReviewer,
            completed_at_unix_ms: 1_700_000_000_000,
        }
    }

    #[test]
    fn missing_cue_prevents_inventory_complete() {
        let universe = ExpectedCueUniverse {
            total_cues: 2,
            cue_ids: vec![
                CueReferenceId::new(1).expect("cue"),
                CueReferenceId::new(2).expect("cue"),
            ],
        };
        let records = vec![completion_record(
            1,
            0,
            ReferenceCueDisposition::NoTranscriptionError,
        )];

        let assessment = ReferenceCoverage::derive_assessment(&universe, &records).expect("derive");
        assert!(!assessment.inventory_complete);
        assert_eq!(
            assessment.missing_cue_ids,
            vec![CueReferenceId::new(2).expect("cue")]
        );
    }
}

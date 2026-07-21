use std::collections::{HashMap, HashSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::reference_coverage::{
    CueReferenceId, CueReferenceIdError, ReferenceCoverage, ReferenceCoverageValidationError,
    ReferenceCueDisposition,
};
use crate::reference_identity::validate_identity_value;
use crate::reference_seal::{
    ReferenceSeal, ReferenceSealId, ReferenceSealState, ReferenceSealValidationError,
};
use crate::run_manifest::{
    CalibrationValidityMode, InputIdentityReference, RunEnvelope, RunEnvelopeValidationError,
    RunId, RunLifecycleState,
};

pub use crate::reference_identity::{ReferenceIdentityIdError, ReferenceRevisionId};

pub const HUMAN_FINAL_REFERENCE_SCHEMA: &str = "voxproof-human-final-reference-v1";

/// Private content-bearing artifact contract. Authorization to create or execute a
/// reference does not authorize committing its surfaces. This contract is not
/// public or commit-safe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HumanFinalReference {
    pub schema_revision: String,
    pub run_id: RunId,
    pub input_identity: InputIdentityReference,
    pub seal_id: ReferenceSealId,
    pub reference_revision: ReferenceRevisionId,
    pub records: Vec<ReferenceErrorRecord>,
    pub state: HumanFinalReferenceState,
    pub assessment: HumanFinalReferenceAssessment,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReferenceErrorId(String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceSourceAnchor {
    pub input_identity: InputIdentityReference,
    pub cue_id: CueReferenceId,
    pub segment_position: u32,
    pub start_byte: u32,
    pub end_byte: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceClass {
    TranscriptionError,
    StylePreference,
    Ambiguous,
    Unsupported,
    NonError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationBasis {
    AudioListened,
    TranscriptContextOnly,
    MixedSources,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceReviewerIdentityClass {
    OwnerBlindReviewer,
    AuthorizedDomainReviewer,
    SyntheticFixtureGenerator,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceErrorRecord {
    pub reference_error_id: ReferenceErrorId,
    pub reference_revision: ReferenceRevisionId,
    pub input_identity: InputIdentityReference,
    pub source_anchor: ReferenceSourceAnchor,
    pub original_surface: String,
    pub human_final_surface: String,
    pub reference_class: ReferenceClass,
    pub verification_basis: VerificationBasis,
    pub reviewer_identity_class: ReferenceReviewerIdentityClass,
    pub reviewed_at_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HumanFinalReferenceState {
    Draft,
    Sealed,
    Invalidated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HumanFinalReferenceAssessment {
    pub total_record_count: u32,
    pub transcription_error_count: u32,
    pub recall_eligible_transcription_error_count: u32,
    pub excluded_reference_count: u32,
    pub duplicate_reference_error_ids: Vec<ReferenceErrorId>,
    pub duplicate_exact_anchors: Vec<ReferenceSourceAnchor>,
    pub context_consistent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceSourceAnchorError {
    EmptyInputIdentity,
    InvalidCueReferenceId(CueReferenceIdError),
    EmptyOrInvertedRange { start_byte: u32, end_byte: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceErrorRecordValidationError {
    EmptyOriginalSurface,
    InputIdentityMismatch,
    AnchorInputIdentityMismatch,
    ReferenceRevisionMismatch,
    ZeroReviewedAt,
    UnchangedRecallEligibleTranscriptionError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HumanFinalReferenceValidationError {
    MissingSchemaRevision,
    UnsupportedSchemaRevision {
        found: String,
        expected: String,
    },
    InvalidReferenceRevisionId(ReferenceIdentityIdError),
    InvalidReferenceErrorId(ReferenceIdentityIdError),
    InvalidSourceAnchor(ReferenceSourceAnchorError),
    RecordValidation(ReferenceErrorRecordValidationError),
    AssessmentMismatch {
        stored: Box<HumanFinalReferenceAssessment>,
        derived: Box<HumanFinalReferenceAssessment>,
    },
    ReferenceStateMismatch {
        state: HumanFinalReferenceState,
        assessment: Box<HumanFinalReferenceAssessment>,
    },
    RunIdMismatch,
    InputIdentityMismatch,
    SealIdMismatch,
    ReferenceRevisionMismatch,
    CoverageReferenceRevisionMismatch,
    EnvelopeNotBlindReference,
    EnvelopeLifecycleIncompatible {
        lifecycle_state: RunLifecycleState,
    },
    SealStateIncompatible {
        seal_state: ReferenceSealState,
    },
    EnvelopeValidation(RunEnvelopeValidationError),
    SealValidation(ReferenceSealValidationError),
    CoverageValidation(ReferenceCoverageValidationError),
    UnknownCueReferenceId {
        cue_id: CueReferenceId,
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
}

impl ReferenceErrorId {
    pub fn new(value: impl Into<String>) -> Result<Self, ReferenceIdentityIdError> {
        let value = value.into();
        validate_identity_value(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn generate() -> Result<Self, ReferenceIdentityIdError> {
        use std::time::{SystemTime, UNIX_EPOCH};

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ReferenceIdentityIdError::GenerationUnavailable)?
            .as_nanos();
        Self::new(format!("ref-err-{nanos:x}"))
    }
}

impl fmt::Display for ReferenceErrorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl ReferenceSourceAnchor {
    pub fn validate(&self) -> Result<(), ReferenceSourceAnchorError> {
        if self.input_identity.transcript_revision_id.is_empty() {
            return Err(ReferenceSourceAnchorError::EmptyInputIdentity);
        }

        validate_cue_reference_id(self.cue_id)?;

        if self.start_byte >= self.end_byte {
            return Err(ReferenceSourceAnchorError::EmptyOrInvertedRange {
                start_byte: self.start_byte,
                end_byte: self.end_byte,
            });
        }

        Ok(())
    }
}

impl ReferenceErrorRecord {
    pub fn is_recall_eligible(&self) -> bool {
        if self.reference_class != ReferenceClass::TranscriptionError {
            return false;
        }

        if self.original_surface == self.human_final_surface {
            return false;
        }

        matches!(
            self.verification_basis,
            VerificationBasis::AudioListened | VerificationBasis::MixedSources
        )
    }

    pub fn validate(
        &self,
        expected_revision: &ReferenceRevisionId,
        expected_input: &InputIdentityReference,
    ) -> Result<(), ReferenceErrorRecordValidationError> {
        if self.original_surface.is_empty() {
            return Err(ReferenceErrorRecordValidationError::EmptyOriginalSurface);
        }

        if self.input_identity != self.source_anchor.input_identity {
            return Err(ReferenceErrorRecordValidationError::AnchorInputIdentityMismatch);
        }

        if self.input_identity != *expected_input {
            return Err(ReferenceErrorRecordValidationError::InputIdentityMismatch);
        }

        if self.reference_revision != *expected_revision {
            return Err(ReferenceErrorRecordValidationError::ReferenceRevisionMismatch);
        }

        if self.reviewed_at_unix_ms == 0 {
            return Err(ReferenceErrorRecordValidationError::ZeroReviewedAt);
        }

        if self.reference_class == ReferenceClass::TranscriptionError
            && self.original_surface == self.human_final_surface
            && matches!(
                self.verification_basis,
                VerificationBasis::AudioListened | VerificationBasis::MixedSources
            )
        {
            return Err(
                ReferenceErrorRecordValidationError::UnchangedRecallEligibleTranscriptionError,
            );
        }

        Ok(())
    }
}

impl HumanFinalReference {
    pub fn derive_assessment(
        reference_revision: &ReferenceRevisionId,
        input_identity: &InputIdentityReference,
        records: &[ReferenceErrorRecord],
    ) -> Result<HumanFinalReferenceAssessment, HumanFinalReferenceValidationError> {
        let mut error_id_counts: HashMap<ReferenceErrorId, u32> = HashMap::new();
        let mut anchor_counts: HashMap<String, u32> = HashMap::new();
        let mut context_consistent = true;

        let mut transcription_error_count = 0u32;
        let mut recall_eligible_transcription_error_count = 0u32;

        for record in records {
            validate_reference_error_id(record.reference_error_id.as_str())
                .map_err(HumanFinalReferenceValidationError::InvalidReferenceErrorId)?;
            record
                .source_anchor
                .validate()
                .map_err(HumanFinalReferenceValidationError::InvalidSourceAnchor)?;

            if record.input_identity != *input_identity
                || record.reference_revision != *reference_revision
            {
                context_consistent = false;
            }

            if let Err(error) = record.validate(reference_revision, input_identity) {
                if matches!(
                    error,
                    ReferenceErrorRecordValidationError::InputIdentityMismatch
                        | ReferenceErrorRecordValidationError::ReferenceRevisionMismatch
                ) {
                    context_consistent = false;
                } else {
                    return Err(HumanFinalReferenceValidationError::RecordValidation(error));
                }
            }

            *error_id_counts
                .entry(record.reference_error_id.clone())
                .or_default() += 1;
            *anchor_counts
                .entry(anchor_key(&record.source_anchor))
                .or_default() += 1;

            if record.reference_class == ReferenceClass::TranscriptionError {
                transcription_error_count += 1;
                if record.is_recall_eligible() {
                    recall_eligible_transcription_error_count += 1;
                }
            }
        }

        let total_record_count = records.len() as u32;
        let excluded_reference_count =
            total_record_count.saturating_sub(recall_eligible_transcription_error_count);

        let mut duplicate_reference_error_ids = error_id_counts
            .iter()
            .filter_map(|(id, count)| (*count > 1).then_some(id.clone()))
            .collect::<Vec<_>>();
        duplicate_reference_error_ids.sort_by(|left, right| left.as_str().cmp(right.as_str()));

        let mut duplicate_exact_anchors = Vec::new();
        let mut seen_duplicate_anchor_keys = HashSet::new();
        for record in records {
            let key = anchor_key(&record.source_anchor);
            if anchor_counts.get(&key).copied().unwrap_or(0) > 1
                && seen_duplicate_anchor_keys.insert(key.clone())
            {
                duplicate_exact_anchors.push(record.source_anchor.clone());
            }
        }
        duplicate_exact_anchors.sort_by(|left, right| {
            left.cue_id
                .value()
                .cmp(&right.cue_id.value())
                .then_with(|| left.segment_position.cmp(&right.segment_position))
                .then_with(|| left.start_byte.cmp(&right.start_byte))
                .then_with(|| left.end_byte.cmp(&right.end_byte))
        });

        Ok(HumanFinalReferenceAssessment {
            total_record_count,
            transcription_error_count,
            recall_eligible_transcription_error_count,
            excluded_reference_count,
            duplicate_reference_error_ids,
            duplicate_exact_anchors,
            context_consistent,
        })
    }

    pub fn validate(&self) -> Result<(), HumanFinalReferenceValidationError> {
        if self.schema_revision.is_empty() {
            return Err(HumanFinalReferenceValidationError::MissingSchemaRevision);
        }

        if self.schema_revision != HUMAN_FINAL_REFERENCE_SCHEMA {
            return Err(
                HumanFinalReferenceValidationError::UnsupportedSchemaRevision {
                    found: self.schema_revision.clone(),
                    expected: HUMAN_FINAL_REFERENCE_SCHEMA.to_string(),
                },
            );
        }

        validate_identity_value(self.reference_revision.as_str())
            .map_err(HumanFinalReferenceValidationError::InvalidReferenceRevisionId)?;

        let derived = Self::derive_assessment(
            &self.reference_revision,
            &self.input_identity,
            &self.records,
        )?;

        if self.assessment != derived {
            return Err(HumanFinalReferenceValidationError::AssessmentMismatch {
                stored: Box::new(self.assessment.clone()),
                derived: Box::new(derived.clone()),
            });
        }

        if self.state == HumanFinalReferenceState::Sealed
            && (!derived.context_consistent
                || !derived.duplicate_reference_error_ids.is_empty()
                || !derived.duplicate_exact_anchors.is_empty())
        {
            return Err(HumanFinalReferenceValidationError::ReferenceStateMismatch {
                state: self.state,
                assessment: Box::new(derived),
            });
        }

        Ok(())
    }

    pub fn validate_against(
        &self,
        envelope: &RunEnvelope,
        seal: &ReferenceSeal,
    ) -> Result<(), HumanFinalReferenceValidationError> {
        self.validate()?;

        envelope
            .validate()
            .map_err(HumanFinalReferenceValidationError::EnvelopeValidation)?;

        if self.run_id != envelope.run_id || self.run_id != seal.run_id {
            return Err(HumanFinalReferenceValidationError::RunIdMismatch);
        }

        if self.input_identity != envelope.input_identity
            || self.input_identity != seal.input_identity
        {
            return Err(HumanFinalReferenceValidationError::InputIdentityMismatch);
        }

        if self.seal_id != seal.seal_id {
            return Err(HumanFinalReferenceValidationError::SealIdMismatch);
        }

        if self.reference_revision != seal.reference_revision {
            return Err(HumanFinalReferenceValidationError::ReferenceRevisionMismatch);
        }

        if envelope.calibration_validity != CalibrationValidityMode::BlindReference {
            return Err(HumanFinalReferenceValidationError::EnvelopeNotBlindReference);
        }

        if envelope.lifecycle_state != RunLifecycleState::ReferenceSealed {
            return Err(
                HumanFinalReferenceValidationError::EnvelopeLifecycleIncompatible {
                    lifecycle_state: envelope.lifecycle_state,
                },
            );
        }

        if seal.seal_state != ReferenceSealState::Sealed {
            return Err(HumanFinalReferenceValidationError::SealStateIncompatible {
                seal_state: seal.seal_state,
            });
        }

        if self.state != HumanFinalReferenceState::Sealed {
            return Err(HumanFinalReferenceValidationError::ReferenceStateMismatch {
                state: self.state,
                assessment: Box::new(self.assessment.clone()),
            });
        }

        seal.validate_with_envelope(envelope)
            .map_err(HumanFinalReferenceValidationError::SealValidation)?;

        Ok(())
    }

    pub fn validate_against_coverage(
        &self,
        coverage: &ReferenceCoverage,
    ) -> Result<(), HumanFinalReferenceValidationError> {
        self.validate()?;

        coverage
            .validate()
            .map_err(HumanFinalReferenceValidationError::CoverageValidation)?;

        if self.run_id != coverage.run_id {
            return Err(HumanFinalReferenceValidationError::RunIdMismatch);
        }

        if self.input_identity != coverage.input_identity {
            return Err(HumanFinalReferenceValidationError::InputIdentityMismatch);
        }

        if self.seal_id != coverage.seal_id {
            return Err(HumanFinalReferenceValidationError::SealIdMismatch);
        }

        if self.reference_revision != coverage.reference_revision {
            return Err(HumanFinalReferenceValidationError::CoverageReferenceRevisionMismatch);
        }

        let expected_cue_ids: HashSet<CueReferenceId> =
            coverage.expected_universe.cue_ids.iter().copied().collect();

        let mut te_records_by_cue: HashMap<CueReferenceId, u32> = HashMap::new();

        for record in &self.records {
            let cue_id = record.source_anchor.cue_id;
            if !expected_cue_ids.contains(&cue_id) {
                return Err(HumanFinalReferenceValidationError::UnknownCueReferenceId { cue_id });
            }

            if record.reference_class == ReferenceClass::TranscriptionError {
                *te_records_by_cue.entry(cue_id).or_default() += 1;
            }
        }

        for coverage_record in &coverage.records {
            match coverage_record.disposition {
                ReferenceCueDisposition::TranscriptionError => {
                    if te_records_by_cue
                        .get(&coverage_record.cue_id)
                        .copied()
                        .unwrap_or(0)
                        == 0
                    {
                        return Err(
                            HumanFinalReferenceValidationError::TranscriptionErrorCueMissingRecord {
                                cue_id: coverage_record.cue_id,
                            },
                        );
                    }
                }
                ReferenceCueDisposition::NoTranscriptionError => {
                    if te_records_by_cue.contains_key(&coverage_record.cue_id) {
                        return Err(
                            HumanFinalReferenceValidationError::NoTranscriptionErrorCueHasRecord {
                                cue_id: coverage_record.cue_id,
                            },
                        );
                    }
                }
                ReferenceCueDisposition::Uncertain | ReferenceCueDisposition::Unreviewable => {
                    if te_records_by_cue.contains_key(&coverage_record.cue_id) {
                        return Err(
                            HumanFinalReferenceValidationError::TranscriptionErrorRecordForUnresolvedCue {
                                cue_id: coverage_record.cue_id,
                            },
                        );
                    }
                }
            }
        }

        Ok(())
    }
}

fn validate_reference_error_id(value: &str) -> Result<(), ReferenceIdentityIdError> {
    validate_identity_value(value)
}

fn validate_cue_reference_id(cue_id: CueReferenceId) -> Result<(), ReferenceSourceAnchorError> {
    CueReferenceId::new(cue_id.value())
        .map_err(ReferenceSourceAnchorError::InvalidCueReferenceId)?;
    Ok(())
}

fn anchor_key(anchor: &ReferenceSourceAnchor) -> String {
    format!(
        "{}:{}:{}:{}:{}",
        anchor.input_identity.transcript_revision_id,
        anchor.cue_id.value(),
        anchor.segment_position,
        anchor.start_byte,
        anchor.end_byte
    )
}

pub fn human_final_reference_from_json(
    json: &str,
) -> Result<HumanFinalReference, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn human_final_reference_to_json(
    reference: &HumanFinalReference,
) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(reference)
}

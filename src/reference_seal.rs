use std::fmt;

use serde::{Deserialize, Serialize};

use crate::reference_identity::{
    ReferenceIdentityIdError, ReferenceRevisionId, validate_identity_value,
};
use crate::run_manifest::{
    CalibrationValidityMode, InputIdentityReference, RunEnvelope, RunEnvelopeValidationError,
    RunId, RunIdError, RunLifecycleState, validate_opaque_identifier,
};

pub const REFERENCE_SEAL_SCHEMA: &str = "voxproof-blind-reference-seal-v1";

/// Protocol record finalization only. Does not imply cryptographic signing,
/// tamper-proof storage, or durable persistence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceSeal {
    pub schema_revision: String,
    pub seal_id: ReferenceSealId,
    pub run_id: RunId,
    pub input_identity: InputIdentityReference,
    pub reference_revision: ReferenceRevisionId,
    pub producer_class: ReferenceProducerClass,
    pub reference_created_before_detector_run: bool,
    pub prior_detector_run_on_same_input: bool,
    pub prior_knowledge_of_detector_targets: bool,
    pub session_terms_visible_during_reference: bool,
    pub external_notes_encode_detector_targets: bool,
    pub seal_state: ReferenceSealState,
    pub calibration_classification: ReferenceCalibrationValidity,
    pub calibration_validity_impact: CalibrationValidityImpact,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReferenceSealId(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceProducerClass {
    HumanBlindReviewer,
    HumanDetectorAssistedReviewer,
    SyntheticFixtureGenerator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceCalibrationValidity {
    BlindReferenceEligible,
    TermConditionedDiagnostic,
    DetectorContaminated,
    SyntheticProtocolOnly,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationValidityImpact {
    None,
    ExcludedFromPrimaryMetrics,
    ProtocolOnly,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceSealState {
    Draft,
    Sealed,
    Invalidated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceSealIdError {
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
pub enum ReferenceSealValidationError {
    MissingSchemaRevision,
    UnsupportedSchemaRevision {
        found: String,
        expected: String,
    },
    InvalidSealId(ReferenceSealIdError),
    InvalidReferenceRevisionId(ReferenceIdentityIdError),
    ClassificationMismatch {
        stored: ReferenceCalibrationValidity,
        derived: ReferenceCalibrationValidity,
    },
    ValidityImpactMismatch {
        stored: CalibrationValidityImpact,
        derived: CalibrationValidityImpact,
    },
    ImmutableSealedStateMutation,
    RunIdMismatch {
        seal: RunId,
        envelope: RunId,
    },
    InputIdentityMismatch,
    EnvelopeNotBlindReference,
    EnvelopeLifecycleIncompatible {
        lifecycle_state: RunLifecycleState,
    },
    HistoricalEnvelopeLifecycleIncompatible {
        lifecycle_state: RunLifecycleState,
    },
    SealStateIncompatible {
        seal_state: ReferenceSealState,
    },
    EnvelopeValidation(RunEnvelopeValidationError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceSealEnvelopeConsistencyError {
    RunIdMismatch { seal: RunId, envelope: RunId },
    InputIdentityMismatch,
    EnvelopeNotBlindReference,
    EnvelopeLifecycleIncompatible { lifecycle_state: RunLifecycleState },
    HistoricalEnvelopeLifecycleIncompatible { lifecycle_state: RunLifecycleState },
    EnvelopeValidation(RunEnvelopeValidationError),
}

impl ReferenceSealId {
    pub fn new(value: impl Into<String>) -> Result<Self, ReferenceSealIdError> {
        let value = value.into();
        validate_seal_id_value(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn generate() -> Result<Self, ReferenceSealIdError> {
        use std::time::{SystemTime, UNIX_EPOCH};

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ReferenceSealIdError::GenerationUnavailable)?
            .as_nanos();
        Self::new(format!("seal-{nanos:x}"))
    }
}

impl fmt::Display for ReferenceSealId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl ReferenceSealState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Invalidated)
    }

    pub fn can_transition_to(self, next: ReferenceSealState) -> bool {
        if self == next {
            return true;
        }

        matches!(
            (self, next),
            (Self::Draft, Self::Sealed | Self::Invalidated)
        )
    }

    pub fn transition(
        self,
        next: ReferenceSealState,
    ) -> Result<ReferenceSealState, ReferenceSealValidationError> {
        if self.can_transition_to(next) {
            Ok(next)
        } else {
            Err(ReferenceSealValidationError::ImmutableSealedStateMutation)
        }
    }
}

impl ReferenceSeal {
    pub fn derive_calibration_classification(&self) -> ReferenceCalibrationValidity {
        if self.producer_class == ReferenceProducerClass::SyntheticFixtureGenerator {
            return ReferenceCalibrationValidity::SyntheticProtocolOnly;
        }

        if is_detector_contaminated(self) {
            return ReferenceCalibrationValidity::DetectorContaminated;
        }

        if self.session_terms_visible_during_reference {
            return ReferenceCalibrationValidity::TermConditionedDiagnostic;
        }

        if is_blind_reference_eligible(self) {
            return ReferenceCalibrationValidity::BlindReferenceEligible;
        }

        ReferenceCalibrationValidity::Invalid
    }

    pub fn derive_calibration_validity_impact(
        classification: ReferenceCalibrationValidity,
    ) -> CalibrationValidityImpact {
        match classification {
            ReferenceCalibrationValidity::BlindReferenceEligible => CalibrationValidityImpact::None,
            ReferenceCalibrationValidity::TermConditionedDiagnostic
            | ReferenceCalibrationValidity::DetectorContaminated => {
                CalibrationValidityImpact::ExcludedFromPrimaryMetrics
            }
            ReferenceCalibrationValidity::SyntheticProtocolOnly => {
                CalibrationValidityImpact::ProtocolOnly
            }
            ReferenceCalibrationValidity::Invalid => CalibrationValidityImpact::Invalid,
        }
    }

    pub fn validate(&self) -> Result<(), ReferenceSealValidationError> {
        if self.schema_revision.is_empty() {
            return Err(ReferenceSealValidationError::MissingSchemaRevision);
        }

        if self.schema_revision != REFERENCE_SEAL_SCHEMA {
            return Err(ReferenceSealValidationError::UnsupportedSchemaRevision {
                found: self.schema_revision.clone(),
                expected: REFERENCE_SEAL_SCHEMA.to_string(),
            });
        }

        validate_seal_id_value(self.seal_id.as_str())
            .map_err(ReferenceSealValidationError::InvalidSealId)?;

        validate_identity_value(self.reference_revision.as_str())
            .map_err(ReferenceSealValidationError::InvalidReferenceRevisionId)?;

        validate_opaque_identifier(self.run_id.as_str()).map_err(|error| {
            ReferenceSealValidationError::InvalidSealId(map_run_id_error(error))
        })?;

        let derived_classification = self.derive_calibration_classification();
        if self.calibration_classification != derived_classification {
            return Err(ReferenceSealValidationError::ClassificationMismatch {
                stored: self.calibration_classification,
                derived: derived_classification,
            });
        }

        let derived_impact = Self::derive_calibration_validity_impact(derived_classification);
        if self.calibration_validity_impact != derived_impact {
            return Err(ReferenceSealValidationError::ValidityImpactMismatch {
                stored: self.calibration_validity_impact,
                derived: derived_impact,
            });
        }

        Ok(())
    }

    pub fn validate_against_run_envelope(
        &self,
        envelope: &RunEnvelope,
    ) -> Result<(), ReferenceSealEnvelopeConsistencyError> {
        envelope
            .validate()
            .map_err(ReferenceSealEnvelopeConsistencyError::EnvelopeValidation)?;

        if self.run_id != envelope.run_id {
            return Err(ReferenceSealEnvelopeConsistencyError::RunIdMismatch {
                seal: self.run_id.clone(),
                envelope: envelope.run_id.clone(),
            });
        }

        if self.input_identity != envelope.input_identity {
            return Err(ReferenceSealEnvelopeConsistencyError::InputIdentityMismatch);
        }

        if envelope.calibration_validity != CalibrationValidityMode::BlindReference {
            return Err(ReferenceSealEnvelopeConsistencyError::EnvelopeNotBlindReference);
        }

        if !is_creation_envelope_lifecycle_compatible(envelope.lifecycle_state) {
            return Err(
                ReferenceSealEnvelopeConsistencyError::EnvelopeLifecycleIncompatible {
                    lifecycle_state: envelope.lifecycle_state,
                },
            );
        }

        Ok(())
    }

    pub fn validate_historical_against_run_envelope(
        &self,
        envelope: &RunEnvelope,
    ) -> Result<(), ReferenceSealEnvelopeConsistencyError> {
        envelope
            .validate()
            .map_err(ReferenceSealEnvelopeConsistencyError::EnvelopeValidation)?;

        if self.run_id != envelope.run_id {
            return Err(ReferenceSealEnvelopeConsistencyError::RunIdMismatch {
                seal: self.run_id.clone(),
                envelope: envelope.run_id.clone(),
            });
        }

        if self.input_identity != envelope.input_identity {
            return Err(ReferenceSealEnvelopeConsistencyError::InputIdentityMismatch);
        }

        if envelope.calibration_validity != CalibrationValidityMode::BlindReference {
            return Err(ReferenceSealEnvelopeConsistencyError::EnvelopeNotBlindReference);
        }

        if !is_historical_envelope_lifecycle_compatible(envelope.lifecycle_state) {
            return Err(
                ReferenceSealEnvelopeConsistencyError::HistoricalEnvelopeLifecycleIncompatible {
                    lifecycle_state: envelope.lifecycle_state,
                },
            );
        }

        Ok(())
    }

    pub fn validate_with_envelope(
        &self,
        envelope: &RunEnvelope,
    ) -> Result<(), ReferenceSealValidationError> {
        self.validate()?;
        self.validate_against_run_envelope(envelope)
            .map_err(map_envelope_consistency_error)?;
        Ok(())
    }

    pub fn validate_historical_context(
        &self,
        envelope: &RunEnvelope,
    ) -> Result<(), ReferenceSealValidationError> {
        self.validate()?;

        if self.seal_state != ReferenceSealState::Sealed {
            return Err(ReferenceSealValidationError::SealStateIncompatible {
                seal_state: self.seal_state,
            });
        }

        self.validate_historical_against_run_envelope(envelope)
            .map_err(map_envelope_consistency_error)?;
        Ok(())
    }
}

fn is_blind_reference_eligible(seal: &ReferenceSeal) -> bool {
    seal.producer_class == ReferenceProducerClass::HumanBlindReviewer
        && seal.reference_created_before_detector_run
        && !seal.prior_detector_run_on_same_input
        && !seal.prior_knowledge_of_detector_targets
        && !seal.session_terms_visible_during_reference
        && !seal.external_notes_encode_detector_targets
}

fn is_detector_contaminated(seal: &ReferenceSeal) -> bool {
    !seal.reference_created_before_detector_run
        || seal.prior_detector_run_on_same_input
        || seal.prior_knowledge_of_detector_targets
        || seal.external_notes_encode_detector_targets
        || seal.producer_class == ReferenceProducerClass::HumanDetectorAssistedReviewer
}

fn is_creation_envelope_lifecycle_compatible(lifecycle_state: RunLifecycleState) -> bool {
    matches!(
        lifecycle_state,
        RunLifecycleState::ReferencePreparation | RunLifecycleState::ReferenceSealed
    )
}

fn is_historical_envelope_lifecycle_compatible(lifecycle_state: RunLifecycleState) -> bool {
    matches!(
        lifecycle_state,
        RunLifecycleState::ReferenceSealed
            | RunLifecycleState::DetectorExecution
            | RunLifecycleState::AssistedReview
            | RunLifecycleState::Finalized
    )
}

fn validate_seal_id_value(value: &str) -> Result<(), ReferenceSealIdError> {
    validate_opaque_identifier(value).map_err(map_run_id_error)
}

fn map_run_id_error(error: RunIdError) -> ReferenceSealIdError {
    match error {
        RunIdError::Empty => ReferenceSealIdError::Empty,
        RunIdError::TooLong { len, max } => ReferenceSealIdError::TooLong { len, max },
        RunIdError::InvalidCharacter { character } => {
            ReferenceSealIdError::InvalidCharacter { character }
        }
        RunIdError::PathLikeContent => ReferenceSealIdError::PathLikeContent,
        RunIdError::AbsolutePathLike => ReferenceSealIdError::AbsolutePathLike,
        RunIdError::RelativePathLike => ReferenceSealIdError::RelativePathLike,
        RunIdError::HomeDirectoryFragment => ReferenceSealIdError::HomeDirectoryFragment,
        RunIdError::GenerationUnavailable => ReferenceSealIdError::GenerationUnavailable,
    }
}

fn map_envelope_consistency_error(
    error: ReferenceSealEnvelopeConsistencyError,
) -> ReferenceSealValidationError {
    match error {
        ReferenceSealEnvelopeConsistencyError::RunIdMismatch { seal, envelope } => {
            ReferenceSealValidationError::RunIdMismatch { seal, envelope }
        }
        ReferenceSealEnvelopeConsistencyError::InputIdentityMismatch => {
            ReferenceSealValidationError::InputIdentityMismatch
        }
        ReferenceSealEnvelopeConsistencyError::EnvelopeNotBlindReference => {
            ReferenceSealValidationError::EnvelopeNotBlindReference
        }
        ReferenceSealEnvelopeConsistencyError::EnvelopeLifecycleIncompatible {
            lifecycle_state,
        } => ReferenceSealValidationError::EnvelopeLifecycleIncompatible { lifecycle_state },
        ReferenceSealEnvelopeConsistencyError::HistoricalEnvelopeLifecycleIncompatible {
            lifecycle_state,
        } => ReferenceSealValidationError::HistoricalEnvelopeLifecycleIncompatible {
            lifecycle_state,
        },
        ReferenceSealEnvelopeConsistencyError::EnvelopeValidation(error) => {
            ReferenceSealValidationError::EnvelopeValidation(error)
        }
    }
}

pub fn seal_from_json(json: &str) -> Result<ReferenceSeal, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn seal_to_json(seal: &ReferenceSeal) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(seal)
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::run_manifest::{
        ArtifactRole, InputClass, RUN_ENVELOPE_SCHEMA, RunEnvelope, WorkflowObservationMode,
    };

    fn blind_attestations() -> ReferenceSeal {
        ReferenceSeal {
            schema_revision: REFERENCE_SEAL_SCHEMA.to_string(),
            seal_id: ReferenceSealId::new("seal-blind-eligible").expect("seal id"),
            run_id: RunId::new("run-blind").expect("run id"),
            input_identity: InputIdentityReference {
                transcript_revision_id:
                    "rev:sha256-v1:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                        .to_string(),
            },
            reference_revision: ReferenceRevisionId::new("ref-rev-unit").expect("revision id"),
            producer_class: ReferenceProducerClass::HumanBlindReviewer,
            reference_created_before_detector_run: true,
            prior_detector_run_on_same_input: false,
            prior_knowledge_of_detector_targets: false,
            session_terms_visible_during_reference: false,
            external_notes_encode_detector_targets: false,
            seal_state: ReferenceSealState::Draft,
            calibration_classification: ReferenceCalibrationValidity::BlindReferenceEligible,
            calibration_validity_impact: CalibrationValidityImpact::None,
        }
    }

    #[test]
    fn classification_precedence_detector_over_term_conditioned() {
        let mut seal = blind_attestations();
        seal.session_terms_visible_during_reference = true;
        seal.prior_detector_run_on_same_input = true;
        assert_eq!(
            seal.derive_calibration_classification(),
            ReferenceCalibrationValidity::DetectorContaminated
        );
    }

    #[test]
    fn blind_envelope_reference_preparation_is_compatible() {
        let seal = blind_attestations();
        let envelope = RunEnvelope {
            schema_revision: RUN_ENVELOPE_SCHEMA.to_string(),
            run_id: seal.run_id.clone(),
            input_identity: seal.input_identity.clone(),
            calibration_validity: CalibrationValidityMode::BlindReference,
            workflow_observation: WorkflowObservationMode::Disabled,
            input_class: InputClass::SelfOwnedReal,
            qualifies_as_real_material_evidence: false,
            lifecycle_state: RunLifecycleState::ReferencePreparation,
            expected_artifact_roles: vec![ArtifactRole::ReferenceSeal],
        };

        seal.validate_with_envelope(&envelope)
            .expect("compatible posture");
    }
}

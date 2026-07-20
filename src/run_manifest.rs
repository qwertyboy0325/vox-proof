use std::fmt;

use serde::{Deserialize, Serialize};

pub const RUN_ENVELOPE_SCHEMA: &str = "voxproof-run-envelope-v1";

const RUN_ID_MAX_LEN: usize = 128;
const TRANSCRIPT_REVISION_PREFIX: &str = "rev:sha256-v1:";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunEnvelope {
    pub schema_revision: String,
    pub run_id: RunId,
    pub input_identity: InputIdentityReference,
    pub calibration_validity: CalibrationValidityMode,
    pub workflow_observation: WorkflowObservationMode,
    pub input_class: InputClass,
    pub qualifies_as_real_material_evidence: bool,
    pub lifecycle_state: RunLifecycleState,
    pub expected_artifact_roles: Vec<ArtifactRole>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RunId(String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputIdentityReference {
    pub transcript_revision_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationValidityMode {
    BlindReference,
    DetectorAssisted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowObservationMode {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputClass {
    SelfOwnedReal,
    ExplicitPermissionReal,
    SyntheticProtocolFixture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunLifecycleState {
    Declared,
    ReferencePreparation,
    ReferenceSealed,
    DetectorExecution,
    AssistedReview,
    Finalized,
    Invalidated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactRole {
    InputAuthorization,
    ReferenceSeal,
    CueReviewCompletion,
    DetectorOutput,
    ReviewLedger,
    ReviewedTranscript,
    Comparison,
    EvaluationJoin,
    Metrics,
    WorkflowTiming,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunIdError {
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
pub enum RunEnvelopeValidationError {
    MissingSchemaRevision,
    UnsupportedSchemaRevision {
        found: String,
        expected: String,
    },
    InvalidRunId(RunIdError),
    InvalidTranscriptRevisionId(String),
    SyntheticCannotQualifyAsRealMaterial,
    IllegalLifecycleTransition {
        from: RunLifecycleState,
        to: RunLifecycleState,
    },
    ForbiddenCalibrationLifecycleCombination {
        calibration_validity: CalibrationValidityMode,
        lifecycle_state: RunLifecycleState,
    },
    TerminalLifecycleState {
        state: RunLifecycleState,
    },
}

impl RunId {
    pub fn new(value: impl Into<String>) -> Result<Self, RunIdError> {
        let value = value.into();
        validate_run_id_value(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn generate() -> Result<Self, RunIdError> {
        use std::time::{SystemTime, UNIX_EPOCH};

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| RunIdError::GenerationUnavailable)?
            .as_nanos();
        Self::new(format!("run-{nanos:x}"))
    }
}

impl fmt::Display for RunId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl RunLifecycleState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Finalized | Self::Invalidated)
    }

    pub fn can_transition_to_for_calibration(
        self,
        next: RunLifecycleState,
        calibration_validity: CalibrationValidityMode,
    ) -> bool {
        if self == next {
            return true;
        }

        if self.is_terminal() {
            return false;
        }

        match calibration_validity {
            CalibrationValidityMode::BlindReference => matches!(
                (self, next),
                (
                    Self::Declared,
                    Self::ReferencePreparation | Self::Invalidated
                ) | (
                    Self::ReferencePreparation,
                    Self::ReferenceSealed | Self::Invalidated
                ) | (
                    Self::ReferenceSealed,
                    Self::DetectorExecution | Self::Invalidated
                ) | (
                    Self::DetectorExecution,
                    Self::AssistedReview | Self::Invalidated
                ) | (Self::AssistedReview, Self::Finalized | Self::Invalidated)
            ),
            CalibrationValidityMode::DetectorAssisted => matches!(
                (self, next),
                (Self::Declared, Self::DetectorExecution | Self::Invalidated)
                    | (
                        Self::DetectorExecution,
                        Self::AssistedReview | Self::Invalidated
                    )
                    | (Self::AssistedReview, Self::Finalized | Self::Invalidated)
            ),
        }
    }

    pub fn transition_for_calibration(
        self,
        next: RunLifecycleState,
        calibration_validity: CalibrationValidityMode,
    ) -> Result<RunLifecycleState, RunEnvelopeValidationError> {
        if self.can_transition_to_for_calibration(next, calibration_validity) {
            Ok(next)
        } else {
            Err(RunEnvelopeValidationError::IllegalLifecycleTransition {
                from: self,
                to: next,
            })
        }
    }
}

impl RunEnvelope {
    pub fn validate(&self) -> Result<(), RunEnvelopeValidationError> {
        if self.schema_revision.is_empty() {
            return Err(RunEnvelopeValidationError::MissingSchemaRevision);
        }

        if self.schema_revision != RUN_ENVELOPE_SCHEMA {
            return Err(RunEnvelopeValidationError::UnsupportedSchemaRevision {
                found: self.schema_revision.clone(),
                expected: RUN_ENVELOPE_SCHEMA.to_string(),
            });
        }

        validate_run_id_value(self.run_id.as_str())
            .map_err(RunEnvelopeValidationError::InvalidRunId)?;

        validate_transcript_revision_id(&self.input_identity.transcript_revision_id)?;

        if self.input_class == InputClass::SyntheticProtocolFixture
            && self.qualifies_as_real_material_evidence
        {
            return Err(RunEnvelopeValidationError::SyntheticCannotQualifyAsRealMaterial);
        }

        validate_calibration_lifecycle_combination(
            self.calibration_validity,
            self.lifecycle_state,
        )?;

        Ok(())
    }

    pub fn validate_transition(
        from: RunLifecycleState,
        to: RunLifecycleState,
        calibration_validity: CalibrationValidityMode,
    ) -> Result<(), RunEnvelopeValidationError> {
        from.transition_for_calibration(to, calibration_validity)
            .map(|_| ())
    }
}

pub fn validate_opaque_identifier(value: &str) -> Result<(), RunIdError> {
    validate_run_id_value(value)
}

fn validate_run_id_value(value: &str) -> Result<(), RunIdError> {
    if value.is_empty() {
        return Err(RunIdError::Empty);
    }

    if value.len() > RUN_ID_MAX_LEN {
        return Err(RunIdError::TooLong {
            len: value.len(),
            max: RUN_ID_MAX_LEN,
        });
    }

    if value.contains('/') || value.contains('\\') {
        return Err(RunIdError::PathLikeContent);
    }

    if value.starts_with('/') || value.starts_with('\\') {
        return Err(RunIdError::AbsolutePathLike);
    }

    if value.starts_with("..") || value.contains("/..") || value.contains("\\..") {
        return Err(RunIdError::RelativePathLike);
    }

    if value.contains("/Users/") || value.contains("\\Users\\") || value.contains("~/") {
        return Err(RunIdError::HomeDirectoryFragment);
    }

    for character in value.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
            continue;
        }
        return Err(RunIdError::InvalidCharacter { character });
    }

    Ok(())
}

fn validate_transcript_revision_id(value: &str) -> Result<(), RunEnvelopeValidationError> {
    if value.contains('/') || value.contains('\\') {
        return Err(RunEnvelopeValidationError::InvalidTranscriptRevisionId(
            value.to_string(),
        ));
    }

    if value.starts_with('/') || value.starts_with('\\') || value.starts_with("..") {
        return Err(RunEnvelopeValidationError::InvalidTranscriptRevisionId(
            value.to_string(),
        ));
    }

    if value.contains("/Users/") || value.contains("\\Users\\") || value.contains("~/") {
        return Err(RunEnvelopeValidationError::InvalidTranscriptRevisionId(
            value.to_string(),
        ));
    }

    if !value.starts_with(TRANSCRIPT_REVISION_PREFIX) {
        return Err(RunEnvelopeValidationError::InvalidTranscriptRevisionId(
            value.to_string(),
        ));
    }

    let digest = &value[TRANSCRIPT_REVISION_PREFIX.len()..];
    if digest.len() != 64 || !digest.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(RunEnvelopeValidationError::InvalidTranscriptRevisionId(
            value.to_string(),
        ));
    }

    Ok(())
}

fn validate_calibration_lifecycle_combination(
    calibration_validity: CalibrationValidityMode,
    lifecycle_state: RunLifecycleState,
) -> Result<(), RunEnvelopeValidationError> {
    match calibration_validity {
        CalibrationValidityMode::DetectorAssisted
            if matches!(
                lifecycle_state,
                RunLifecycleState::ReferencePreparation | RunLifecycleState::ReferenceSealed
            ) =>
        {
            Err(
                RunEnvelopeValidationError::ForbiddenCalibrationLifecycleCombination {
                    calibration_validity,
                    lifecycle_state,
                },
            )
        }
        _ => Ok(()),
    }
}

pub fn envelope_from_json(json: &str) -> Result<RunEnvelope, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn envelope_to_json(envelope: &RunEnvelope) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(envelope)
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn lifecycle_transition_matrix_matches_protocol_minimum() {
        assert!(
            RunLifecycleState::Declared.can_transition_to_for_calibration(
                RunLifecycleState::ReferencePreparation,
                CalibrationValidityMode::BlindReference,
            )
        );
        assert!(
            RunLifecycleState::Declared.can_transition_to_for_calibration(
                RunLifecycleState::DetectorExecution,
                CalibrationValidityMode::DetectorAssisted,
            )
        );
        assert!(
            !RunLifecycleState::Declared.can_transition_to_for_calibration(
                RunLifecycleState::DetectorExecution,
                CalibrationValidityMode::BlindReference,
            )
        );
        assert!(
            !RunLifecycleState::Finalized.can_transition_to_for_calibration(
                RunLifecycleState::Declared,
                CalibrationValidityMode::BlindReference,
            )
        );
    }
}

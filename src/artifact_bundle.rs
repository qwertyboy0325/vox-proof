use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::human_final_reference::{HumanFinalReference, HumanFinalReferenceValidationError};
use crate::reference_coverage::{
    ReferenceCoverage, ReferenceCoverageId, ReferenceCoverageState,
    ReferenceCoverageValidationError,
};
use crate::reference_identity::{ReferenceRevisionId, validate_identity_value};
use crate::reference_seal::{ReferenceSeal, ReferenceSealId, ReferenceSealValidationError};
use crate::run_manifest::{
    ArtifactRole, CalibrationValidityMode, InputIdentityReference, RunEnvelope,
    RunEnvelopeValidationError, RunId, RunIdError, RunLifecycleState, validate_opaque_identifier,
};

pub const ARTIFACT_BUNDLE_SCHEMA: &str = "voxproof-artifact-bundle-v1";

const SHA256_DIGEST_PREFIX: &str = "sha256:";
const SCHEMA_TOKEN_MAX_LEN: usize = 128;

/// Bundle manifests describe metadata and content references only. A digest binds
/// bytes but does not prove correctness, privacy, semantic compatibility,
/// provenance truth, payload validity, or join success.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactBundle {
    pub schema_revision: String,
    pub bundle_id: ArtifactBundleId,
    pub binding_context: ArtifactBindingContext,
    pub expected_roles: Vec<ArtifactRole>,
    pub artifacts: Vec<ArtifactDescriptor>,
    pub bundle_state: ArtifactBundleState,
    pub assessment: ArtifactBundleAssessment,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArtifactBundleId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArtifactId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArtifactContentDigest(String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactSchemaIdentity {
    pub schema_id: String,
    pub schema_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactBindingContext {
    pub run_id: RunId,
    pub input_identity: InputIdentityReference,
    pub calibration_validity: CalibrationValidityMode,
    pub reference_seal_id: Option<ReferenceSealId>,
    pub reference_coverage_id: Option<ReferenceCoverageId>,
    pub reference_revision: Option<ReferenceRevisionId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactDescriptor {
    pub artifact_id: ArtifactId,
    pub role: ArtifactRole,
    pub payload_schema: ArtifactSchemaIdentity,
    pub content_digest: ArtifactContentDigest,
    pub byte_length: u64,
    pub binding_context: ArtifactBindingContext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactBundleState {
    Draft,
    Complete,
    Invalidated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactBundleAssessment {
    pub expected_roles: Vec<ArtifactRole>,
    pub present_roles: Vec<ArtifactRole>,
    pub missing_roles: Vec<ArtifactRole>,
    pub unexpected_roles: Vec<ArtifactRole>,
    pub duplicate_roles: Vec<ArtifactRole>,
    pub duplicate_artifact_ids: Vec<ArtifactId>,
    pub context_mismatch_artifact_ids: Vec<ArtifactId>,
    pub inventory_complete: bool,
    pub context_consistent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactBundleIdError {
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
pub enum ArtifactContentDigestError {
    MissingPrefix,
    InvalidLength,
    InvalidHexCharacter { character: char },
    UppercaseHexNotCanonical,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactSchemaIdentityError {
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
pub enum ArtifactBundleValidationError {
    MissingSchemaRevision,
    UnsupportedSchemaRevision {
        found: String,
        expected: String,
    },
    InvalidBundleId(ArtifactBundleIdError),
    InvalidArtifactId(ArtifactBundleIdError),
    InvalidContentDigest(ArtifactContentDigestError),
    InvalidSchemaIdentity(ArtifactSchemaIdentityError),
    ZeroByteLength,
    DuplicateExpectedRole {
        role: ArtifactRole,
    },
    CoverageReferenceWithoutSeal,
    ReferenceRevisionWithoutContext,
    SealContextWithoutRevision,
    DetectorAssistedReferenceRevisionContext,
    DetectorAssistedBlindReferenceContext,
    AssessmentMismatch {
        stored: Box<ArtifactBundleAssessment>,
        derived: Box<ArtifactBundleAssessment>,
    },
    BundleStateMismatch {
        state: ArtifactBundleState,
        assessment: Box<ArtifactBundleAssessment>,
    },
    RunIdMismatch,
    InputIdentityMismatch,
    CalibrationModeMismatch,
    ExpectedRolesMismatch,
    EnvelopeInvalidated,
    EnvelopeValidation(RunEnvelopeValidationError),
    SealValidation(ReferenceSealValidationError),
    CoverageValidation(ReferenceCoverageValidationError),
    ReferenceSealRequired {
        role: ArtifactRole,
    },
    ReferenceCoverageRequired {
        role: ArtifactRole,
    },
    SealContextMissing,
    CoverageContextMissing,
    UnexpectedSealSupplied,
    UnexpectedCoverageSupplied,
    SealContextMismatch,
    CoverageContextMismatch,
    SealRecordMismatch,
    CoverageRecordMismatch,
    ReferenceRevisionContextMismatch,
    UnexpectedHumanReferenceSupplied,
    HumanReferenceContextMissing,
    HumanReferenceRequired {
        role: ArtifactRole,
    },
    HumanReferenceRecordMismatch,
    HumanReferenceValidation(HumanFinalReferenceValidationError),
    CoverageWithoutSealInInventory,
    ReferenceContextLifecycleIncompatible {
        lifecycle_state: RunLifecycleState,
    },
}

impl ArtifactBundleId {
    pub fn new(value: impl Into<String>) -> Result<Self, ArtifactBundleIdError> {
        let value = value.into();
        validate_bundle_id_value(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn generate() -> Result<Self, ArtifactBundleIdError> {
        use std::time::{SystemTime, UNIX_EPOCH};

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ArtifactBundleIdError::GenerationUnavailable)?
            .as_nanos();
        Self::new(format!("bundle-{nanos:x}"))
    }
}

impl ArtifactId {
    pub fn new(value: impl Into<String>) -> Result<Self, ArtifactBundleIdError> {
        let value = value.into();
        validate_bundle_id_value(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl ArtifactContentDigest {
    pub fn new(value: impl Into<String>) -> Result<Self, ArtifactContentDigestError> {
        let value = value.into();
        validate_content_digest(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl ArtifactSchemaIdentity {
    pub fn new(
        schema_id: impl Into<String>,
        schema_revision: impl Into<String>,
    ) -> Result<Self, ArtifactSchemaIdentityError> {
        let schema_id = schema_id.into();
        let schema_revision = schema_revision.into();
        validate_schema_token("schema_id", &schema_id)?;
        validate_schema_token("schema_revision", &schema_revision)?;
        Ok(Self {
            schema_id,
            schema_revision,
        })
    }
}

impl fmt::Display for ArtifactBundleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl ArtifactBundle {
    pub fn derive_assessment(
        expected_roles: &[ArtifactRole],
        artifacts: &[ArtifactDescriptor],
        binding_context: &ArtifactBindingContext,
    ) -> Result<ArtifactBundleAssessment, ArtifactBundleValidationError> {
        validate_unique_roles("expected_roles", expected_roles)?;

        let expected_set: BTreeSet<ArtifactRole> = expected_roles.iter().copied().collect();
        let mut role_counts: HashMap<ArtifactRole, u32> = HashMap::new();
        let mut artifact_id_counts: HashMap<ArtifactId, u32> = HashMap::new();
        let mut context_mismatch_artifact_ids = Vec::new();

        for descriptor in artifacts {
            validate_descriptor_fields(descriptor)?;
            *role_counts.entry(descriptor.role).or_default() += 1;
            *artifact_id_counts
                .entry(descriptor.artifact_id.clone())
                .or_default() += 1;
            if descriptor.binding_context != *binding_context {
                context_mismatch_artifact_ids.push(descriptor.artifact_id.clone());
            }
        }

        context_mismatch_artifact_ids.sort_by(|left, right| left.as_str().cmp(right.as_str()));

        let mut duplicate_roles = role_counts
            .iter()
            .filter_map(|(role, count)| (*count > 1).then_some(*role))
            .collect::<Vec<_>>();
        duplicate_roles.sort_unstable();

        let mut duplicate_artifact_ids = artifact_id_counts
            .iter()
            .filter_map(|(artifact_id, count)| (*count > 1).then_some(artifact_id.clone()))
            .collect::<Vec<_>>();
        duplicate_artifact_ids.sort_by(|left, right| left.as_str().cmp(right.as_str()));

        let present_set: BTreeSet<ArtifactRole> = role_counts.keys().copied().collect();
        let mut expected_sorted = expected_set.iter().copied().collect::<Vec<_>>();
        expected_sorted.sort_unstable();
        let mut present_roles = present_set.iter().copied().collect::<Vec<_>>();
        present_roles.sort_unstable();

        let missing_roles = expected_set
            .difference(&present_set)
            .copied()
            .collect::<Vec<_>>();
        let mut unexpected_roles = present_set
            .difference(&expected_set)
            .copied()
            .collect::<Vec<_>>();
        unexpected_roles.sort_unstable();

        let inventory_complete = missing_roles.is_empty()
            && unexpected_roles.is_empty()
            && duplicate_roles.is_empty()
            && duplicate_artifact_ids.is_empty()
            && artifacts.len() == expected_roles.len();

        let context_consistent = context_mismatch_artifact_ids.is_empty();

        Ok(ArtifactBundleAssessment {
            expected_roles: expected_sorted,
            present_roles,
            missing_roles,
            unexpected_roles,
            duplicate_roles,
            duplicate_artifact_ids,
            context_mismatch_artifact_ids,
            inventory_complete,
            context_consistent,
        })
    }

    pub fn validate(&self) -> Result<(), ArtifactBundleValidationError> {
        if self.schema_revision.is_empty() {
            return Err(ArtifactBundleValidationError::MissingSchemaRevision);
        }

        if self.schema_revision != ARTIFACT_BUNDLE_SCHEMA {
            return Err(ArtifactBundleValidationError::UnsupportedSchemaRevision {
                found: self.schema_revision.clone(),
                expected: ARTIFACT_BUNDLE_SCHEMA.to_string(),
            });
        }

        validate_bundle_id_value(self.bundle_id.as_str())
            .map_err(ArtifactBundleValidationError::InvalidBundleId)?;

        validate_binding_context(&self.binding_context)?;

        let derived =
            Self::derive_assessment(&self.expected_roles, &self.artifacts, &self.binding_context)?;

        if self.assessment != derived {
            return Err(ArtifactBundleValidationError::AssessmentMismatch {
                stored: Box::new(self.assessment.clone()),
                derived: Box::new(derived.clone()),
            });
        }

        if self.bundle_state == ArtifactBundleState::Complete
            && (!derived.inventory_complete || !derived.context_consistent)
        {
            return Err(ArtifactBundleValidationError::BundleStateMismatch {
                state: self.bundle_state,
                assessment: Box::new(derived),
            });
        }

        Ok(())
    }

    pub fn validate_against_envelope(
        &self,
        envelope: &RunEnvelope,
    ) -> Result<(), ArtifactBundleValidationError> {
        self.validate()?;

        envelope
            .validate()
            .map_err(ArtifactBundleValidationError::EnvelopeValidation)?;

        if envelope.lifecycle_state == RunLifecycleState::Invalidated {
            return Err(ArtifactBundleValidationError::EnvelopeInvalidated);
        }

        if self.binding_context.run_id != envelope.run_id {
            return Err(ArtifactBundleValidationError::RunIdMismatch);
        }

        if self.binding_context.input_identity != envelope.input_identity {
            return Err(ArtifactBundleValidationError::InputIdentityMismatch);
        }

        if self.binding_context.calibration_validity != envelope.calibration_validity {
            return Err(ArtifactBundleValidationError::CalibrationModeMismatch);
        }

        validate_unique_roles(
            "envelope.expected_artifact_roles",
            &envelope.expected_artifact_roles,
        )?;

        if canonical_role_set(&self.expected_roles)
            != canonical_role_set(&envelope.expected_artifact_roles)
        {
            return Err(ArtifactBundleValidationError::ExpectedRolesMismatch);
        }

        Ok(())
    }

    pub fn validate_with_reference_context(
        &self,
        envelope: &RunEnvelope,
        seal: Option<&ReferenceSeal>,
        coverage: Option<&ReferenceCoverage>,
        human_reference: Option<&HumanFinalReference>,
    ) -> Result<(), ArtifactBundleValidationError> {
        self.validate_against_envelope(envelope)?;

        validate_reference_context_binding(
            &self.binding_context,
            envelope.calibration_validity,
            seal,
            coverage,
            human_reference,
        )?;

        if self.binding_context.reference_coverage_id.is_some()
            && self
                .assessment
                .present_roles
                .contains(&ArtifactRole::CueReviewCompletion)
            && seal.is_none()
        {
            return Err(ArtifactBundleValidationError::CoverageWithoutSealInInventory);
        }

        validate_role_specific_context(self, seal, coverage, human_reference)?;

        let reference_validation_mode =
            if seal.is_some() || coverage.is_some() || human_reference.is_some() {
                Some(reference_context_validation_mode(envelope.lifecycle_state)?)
            } else {
                None
            };

        if let Some(seal_record) = seal {
            match reference_validation_mode
                .expect("reference validation mode required when seal supplied")
            {
                ReferenceContextValidationMode::CreationTime => seal_record
                    .validate_with_envelope(envelope)
                    .map_err(ArtifactBundleValidationError::SealValidation)?,
                ReferenceContextValidationMode::HistoricalContext => seal_record
                    .validate_historical_context(envelope)
                    .map_err(ArtifactBundleValidationError::SealValidation)?,
            }
            if seal_record.run_id != self.binding_context.run_id
                || seal_record.input_identity != self.binding_context.input_identity
            {
                return Err(ArtifactBundleValidationError::SealRecordMismatch);
            }
            if self.binding_context.reference_seal_id.as_ref() != Some(&seal_record.seal_id) {
                return Err(ArtifactBundleValidationError::SealRecordMismatch);
            }
            if self.binding_context.reference_revision.as_ref()
                != Some(&seal_record.reference_revision)
            {
                return Err(ArtifactBundleValidationError::ReferenceRevisionContextMismatch);
            }
        }

        if let Some(coverage_record) = coverage {
            coverage_record
                .validate()
                .map_err(ArtifactBundleValidationError::CoverageValidation)?;
            if coverage_record.run_id != self.binding_context.run_id
                || coverage_record.input_identity != self.binding_context.input_identity
            {
                return Err(ArtifactBundleValidationError::CoverageRecordMismatch);
            }
            if self.binding_context.reference_coverage_id.as_ref()
                != Some(&coverage_record.coverage_id)
            {
                return Err(ArtifactBundleValidationError::CoverageRecordMismatch);
            }
            if self.binding_context.reference_seal_id.as_ref() != Some(&coverage_record.seal_id) {
                return Err(ArtifactBundleValidationError::CoverageRecordMismatch);
            }
            if self.binding_context.reference_revision.as_ref()
                != Some(&coverage_record.reference_revision)
            {
                return Err(ArtifactBundleValidationError::ReferenceRevisionContextMismatch);
            }

            if let Some(seal_record) = seal {
                match reference_validation_mode
                    .expect("reference validation mode required when coverage and seal supplied")
                {
                    ReferenceContextValidationMode::CreationTime => coverage_record
                        .validate_against(envelope, seal_record, human_reference)
                        .map_err(ArtifactBundleValidationError::CoverageValidation)?,
                    ReferenceContextValidationMode::HistoricalContext => coverage_record
                        .validate_historical_context(envelope, seal_record, human_reference)
                        .map_err(ArtifactBundleValidationError::CoverageValidation)?,
                }
            } else if coverage_record.coverage_state == ReferenceCoverageState::Complete {
                return Err(ArtifactBundleValidationError::CoverageContextMissing);
            }
        }

        if let Some(human_reference_record) = human_reference {
            human_reference_record
                .validate()
                .map_err(ArtifactBundleValidationError::HumanReferenceValidation)?;
            if human_reference_record.run_id != self.binding_context.run_id
                || human_reference_record.input_identity != self.binding_context.input_identity
            {
                return Err(ArtifactBundleValidationError::HumanReferenceRecordMismatch);
            }
            if self.binding_context.reference_seal_id.as_ref()
                != Some(&human_reference_record.seal_id)
            {
                return Err(ArtifactBundleValidationError::HumanReferenceRecordMismatch);
            }
            if self.binding_context.reference_revision.as_ref()
                != Some(&human_reference_record.reference_revision)
            {
                return Err(ArtifactBundleValidationError::ReferenceRevisionContextMismatch);
            }

            if let Some(seal_record) = seal {
                match reference_validation_mode.expect(
                    "reference validation mode required when human reference and seal supplied",
                ) {
                    ReferenceContextValidationMode::CreationTime => human_reference_record
                        .validate_against(envelope, seal_record)
                        .map_err(ArtifactBundleValidationError::HumanReferenceValidation)?,
                    ReferenceContextValidationMode::HistoricalContext => human_reference_record
                        .validate_historical_context(envelope, seal_record)
                        .map_err(ArtifactBundleValidationError::HumanReferenceValidation)?,
                }
            }
        }

        Ok(())
    }
}

#[derive(Clone, Copy)]
enum ReferenceContextValidationMode {
    CreationTime,
    HistoricalContext,
}

fn reference_context_validation_mode(
    lifecycle_state: RunLifecycleState,
) -> Result<ReferenceContextValidationMode, ArtifactBundleValidationError> {
    match lifecycle_state {
        RunLifecycleState::ReferencePreparation | RunLifecycleState::ReferenceSealed => {
            Ok(ReferenceContextValidationMode::CreationTime)
        }
        RunLifecycleState::DetectorExecution
        | RunLifecycleState::AssistedReview
        | RunLifecycleState::Finalized => Ok(ReferenceContextValidationMode::HistoricalContext),
        RunLifecycleState::Declared | RunLifecycleState::Invalidated => Err(
            ArtifactBundleValidationError::ReferenceContextLifecycleIncompatible {
                lifecycle_state,
            },
        ),
    }
}

fn validate_descriptor_fields(
    descriptor: &ArtifactDescriptor,
) -> Result<(), ArtifactBundleValidationError> {
    validate_bundle_id_value(descriptor.artifact_id.as_str())
        .map_err(ArtifactBundleValidationError::InvalidArtifactId)?;
    validate_content_digest(descriptor.content_digest.as_str())
        .map_err(ArtifactBundleValidationError::InvalidContentDigest)?;
    ArtifactSchemaIdentity::new(
        descriptor.payload_schema.schema_id.clone(),
        descriptor.payload_schema.schema_revision.clone(),
    )
    .map_err(ArtifactBundleValidationError::InvalidSchemaIdentity)?;

    if descriptor.byte_length == 0 {
        return Err(ArtifactBundleValidationError::ZeroByteLength);
    }

    Ok(())
}

fn validate_binding_context(
    context: &ArtifactBindingContext,
) -> Result<(), ArtifactBundleValidationError> {
    validate_opaque_identifier(context.run_id.as_str())
        .map_err(|error| ArtifactBundleValidationError::InvalidBundleId(map_run_id_error(error)))?;

    if context.reference_coverage_id.is_some() && context.reference_seal_id.is_none() {
        return Err(ArtifactBundleValidationError::CoverageReferenceWithoutSeal);
    }

    if context.reference_seal_id.is_some() && context.reference_revision.is_none() {
        return Err(ArtifactBundleValidationError::SealContextWithoutRevision);
    }

    if context.reference_coverage_id.is_some()
        && (context.reference_seal_id.is_none() || context.reference_revision.is_none())
    {
        return Err(ArtifactBundleValidationError::CoverageReferenceWithoutSeal);
    }

    if context.reference_revision.is_some()
        && context.reference_seal_id.is_none()
        && context.reference_coverage_id.is_none()
    {
        return Err(ArtifactBundleValidationError::ReferenceRevisionWithoutContext);
    }

    if context.calibration_validity == CalibrationValidityMode::DetectorAssisted
        && (context.reference_seal_id.is_some()
            || context.reference_coverage_id.is_some()
            || context.reference_revision.is_some())
    {
        return Err(ArtifactBundleValidationError::DetectorAssistedBlindReferenceContext);
    }

    if let Some(reference_revision) = context.reference_revision.as_ref() {
        validate_identity_value(reference_revision.as_str()).map_err(|error| {
            ArtifactBundleValidationError::InvalidBundleId(map_identity_id_error(error))
        })?;
    }

    Ok(())
}

fn validate_reference_context_binding(
    context: &ArtifactBindingContext,
    envelope_mode: CalibrationValidityMode,
    seal: Option<&ReferenceSeal>,
    coverage: Option<&ReferenceCoverage>,
    human_reference: Option<&HumanFinalReference>,
) -> Result<(), ArtifactBundleValidationError> {
    match (
        context.reference_seal_id.as_ref(),
        context.reference_coverage_id.as_ref(),
    ) {
        (None, None) => {
            if seal.is_some() {
                return Err(ArtifactBundleValidationError::UnexpectedSealSupplied);
            }
            if coverage.is_some() {
                return Err(ArtifactBundleValidationError::UnexpectedCoverageSupplied);
            }
            if human_reference.is_some() {
                return Err(ArtifactBundleValidationError::UnexpectedHumanReferenceSupplied);
            }
        }
        (Some(_), None) => {
            if seal.is_none() {
                return Err(ArtifactBundleValidationError::SealContextMissing);
            }
            if coverage.is_some() {
                return Err(ArtifactBundleValidationError::UnexpectedCoverageSupplied);
            }
        }
        (Some(_), Some(_)) => {
            if seal.is_none() {
                return Err(ArtifactBundleValidationError::SealContextMissing);
            }
            if coverage.is_none() {
                return Err(ArtifactBundleValidationError::CoverageContextMissing);
            }
        }
        (None, Some(_)) => return Err(ArtifactBundleValidationError::CoverageReferenceWithoutSeal),
    }

    if human_reference.is_some() {
        if seal.is_none() || context.reference_seal_id.is_none() {
            return Err(ArtifactBundleValidationError::SealContextMissing);
        }
        if context.reference_revision.is_none() {
            return Err(ArtifactBundleValidationError::SealContextWithoutRevision);
        }
    }

    if envelope_mode == CalibrationValidityMode::DetectorAssisted
        && (context.reference_seal_id.is_some()
            || context.reference_coverage_id.is_some()
            || context.reference_revision.is_some())
    {
        return Err(ArtifactBundleValidationError::DetectorAssistedBlindReferenceContext);
    }

    if envelope_mode == CalibrationValidityMode::DetectorAssisted
        && context.reference_revision.is_some()
    {
        return Err(ArtifactBundleValidationError::DetectorAssistedReferenceRevisionContext);
    }

    if let (Some(expected_seal_id), Some(seal_record)) = (context.reference_seal_id.as_ref(), seal)
        && expected_seal_id != &seal_record.seal_id
    {
        return Err(ArtifactBundleValidationError::SealContextMismatch);
    }

    if let (Some(expected_coverage_id), Some(coverage_record)) =
        (context.reference_coverage_id.as_ref(), coverage)
        && expected_coverage_id != &coverage_record.coverage_id
    {
        return Err(ArtifactBundleValidationError::CoverageContextMismatch);
    }

    if let (Some(expected_revision), Some(seal_record)) =
        (context.reference_revision.as_ref(), seal)
        && expected_revision != &seal_record.reference_revision
    {
        return Err(ArtifactBundleValidationError::ReferenceRevisionContextMismatch);
    }

    if let (Some(expected_revision), Some(coverage_record)) =
        (context.reference_revision.as_ref(), coverage)
        && expected_revision != &coverage_record.reference_revision
    {
        return Err(ArtifactBundleValidationError::ReferenceRevisionContextMismatch);
    }

    if let (Some(expected_revision), Some(human_reference_record)) =
        (context.reference_revision.as_ref(), human_reference)
        && expected_revision != &human_reference_record.reference_revision
    {
        return Err(ArtifactBundleValidationError::ReferenceRevisionContextMismatch);
    }

    Ok(())
}

fn validate_role_specific_context(
    bundle: &ArtifactBundle,
    seal: Option<&ReferenceSeal>,
    coverage: Option<&ReferenceCoverage>,
    human_reference: Option<&HumanFinalReference>,
) -> Result<(), ArtifactBundleValidationError> {
    let roles = canonical_role_set(&bundle.expected_roles);

    if roles.contains(&ArtifactRole::ReferenceSeal)
        && (bundle.binding_context.reference_seal_id.is_none()
            || bundle.binding_context.reference_revision.is_none())
    {
        return Err(ArtifactBundleValidationError::ReferenceSealRequired {
            role: ArtifactRole::ReferenceSeal,
        });
    }

    if roles.contains(&ArtifactRole::HumanFinalReference)
        && (bundle.binding_context.reference_seal_id.is_none()
            || bundle.binding_context.reference_revision.is_none())
    {
        return Err(ArtifactBundleValidationError::HumanReferenceRequired {
            role: ArtifactRole::HumanFinalReference,
        });
    }

    if roles.contains(&ArtifactRole::CueReviewCompletion)
        && (bundle.binding_context.reference_seal_id.is_none()
            || bundle.binding_context.reference_coverage_id.is_none()
            || bundle.binding_context.reference_revision.is_none())
    {
        return Err(ArtifactBundleValidationError::ReferenceCoverageRequired {
            role: ArtifactRole::CueReviewCompletion,
        });
    }

    if human_reference.is_some() && !roles.contains(&ArtifactRole::HumanFinalReference) {
        return Err(ArtifactBundleValidationError::UnexpectedHumanReferenceSupplied);
    }

    if bundle.bundle_state == ArtifactBundleState::Complete {
        if roles.contains(&ArtifactRole::ReferenceSeal) && seal.is_none() {
            return Err(ArtifactBundleValidationError::SealContextMissing);
        }
        if roles.contains(&ArtifactRole::HumanFinalReference)
            && (seal.is_none() || human_reference.is_none())
        {
            return Err(ArtifactBundleValidationError::HumanReferenceContextMissing);
        }
        if roles.contains(&ArtifactRole::CueReviewCompletion)
            && (seal.is_none() || coverage.is_none())
        {
            return Err(ArtifactBundleValidationError::CoverageContextMissing);
        }
    }

    Ok(())
}

fn validate_unique_roles(
    label: &str,
    roles: &[ArtifactRole],
) -> Result<(), ArtifactBundleValidationError> {
    let _ = label;
    let mut seen = HashSet::new();
    for role in roles {
        if !seen.insert(*role) {
            return Err(ArtifactBundleValidationError::DuplicateExpectedRole { role: *role });
        }
    }
    Ok(())
}

fn canonical_role_set(roles: &[ArtifactRole]) -> BTreeSet<ArtifactRole> {
    roles.iter().copied().collect()
}

fn validate_bundle_id_value(value: &str) -> Result<(), ArtifactBundleIdError> {
    validate_opaque_identifier(value).map_err(map_run_id_error)
}

fn validate_content_digest(value: &str) -> Result<(), ArtifactContentDigestError> {
    if !value.starts_with(SHA256_DIGEST_PREFIX) {
        return Err(ArtifactContentDigestError::MissingPrefix);
    }

    let digest = &value[SHA256_DIGEST_PREFIX.len()..];
    if digest.len() != 64 {
        return Err(ArtifactContentDigestError::InvalidLength);
    }

    for character in digest.chars() {
        if character.is_ascii_digit() || matches!(character, 'a'..='f') {
            continue;
        }
        if character.is_ascii_hexdigit() {
            return Err(ArtifactContentDigestError::UppercaseHexNotCanonical);
        }
        return Err(ArtifactContentDigestError::InvalidHexCharacter { character });
    }

    Ok(())
}

fn validate_schema_token(
    field: &'static str,
    value: &str,
) -> Result<(), ArtifactSchemaIdentityError> {
    if value.is_empty() {
        return Err(ArtifactSchemaIdentityError::EmptyField { field });
    }

    if value.len() > SCHEMA_TOKEN_MAX_LEN {
        return Err(ArtifactSchemaIdentityError::TooLong {
            field,
            len: value.len(),
            max: SCHEMA_TOKEN_MAX_LEN,
        });
    }

    if value.contains('/') || value.contains('\\') {
        return Err(ArtifactSchemaIdentityError::PathLikeContent { field });
    }

    for character in value.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
            continue;
        }
        return Err(ArtifactSchemaIdentityError::InvalidCharacter { field, character });
    }

    Ok(())
}

fn map_run_id_error(error: RunIdError) -> ArtifactBundleIdError {
    match error {
        RunIdError::Empty => ArtifactBundleIdError::Empty,
        RunIdError::TooLong { len, max } => ArtifactBundleIdError::TooLong { len, max },
        RunIdError::InvalidCharacter { character } => {
            ArtifactBundleIdError::InvalidCharacter { character }
        }
        RunIdError::PathLikeContent => ArtifactBundleIdError::PathLikeContent,
        RunIdError::AbsolutePathLike => ArtifactBundleIdError::AbsolutePathLike,
        RunIdError::RelativePathLike => ArtifactBundleIdError::RelativePathLike,
        RunIdError::HomeDirectoryFragment => ArtifactBundleIdError::HomeDirectoryFragment,
        RunIdError::GenerationUnavailable => ArtifactBundleIdError::GenerationUnavailable,
    }
}

fn map_identity_id_error(
    error: crate::reference_identity::ReferenceIdentityIdError,
) -> ArtifactBundleIdError {
    match error {
        crate::reference_identity::ReferenceIdentityIdError::Empty => ArtifactBundleIdError::Empty,
        crate::reference_identity::ReferenceIdentityIdError::TooLong { len, max } => {
            ArtifactBundleIdError::TooLong { len, max }
        }
        crate::reference_identity::ReferenceIdentityIdError::InvalidCharacter { character } => {
            ArtifactBundleIdError::InvalidCharacter { character }
        }
        crate::reference_identity::ReferenceIdentityIdError::PathLikeContent => {
            ArtifactBundleIdError::PathLikeContent
        }
        crate::reference_identity::ReferenceIdentityIdError::AbsolutePathLike => {
            ArtifactBundleIdError::AbsolutePathLike
        }
        crate::reference_identity::ReferenceIdentityIdError::RelativePathLike => {
            ArtifactBundleIdError::RelativePathLike
        }
        crate::reference_identity::ReferenceIdentityIdError::HomeDirectoryFragment => {
            ArtifactBundleIdError::HomeDirectoryFragment
        }
        crate::reference_identity::ReferenceIdentityIdError::GenerationUnavailable => {
            ArtifactBundleIdError::GenerationUnavailable
        }
    }
}

pub fn bundle_from_json(json: &str) -> Result<ArtifactBundle, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn bundle_to_json(bundle: &ArtifactBundle) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(bundle)
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    const SAMPLE_REVISION: &str =
        "rev:sha256-v1:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const SAMPLE_DIGEST: &str =
        "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn binding_context() -> ArtifactBindingContext {
        ArtifactBindingContext {
            run_id: RunId::new("run-bundle").expect("run id"),
            input_identity: InputIdentityReference {
                transcript_revision_id: SAMPLE_REVISION.to_string(),
            },
            calibration_validity: CalibrationValidityMode::DetectorAssisted,
            reference_seal_id: None,
            reference_coverage_id: None,
            reference_revision: None,
        }
    }

    fn descriptor(role: ArtifactRole, artifact_id: &str) -> ArtifactDescriptor {
        ArtifactDescriptor {
            artifact_id: ArtifactId::new(artifact_id).expect("artifact id"),
            role,
            payload_schema: ArtifactSchemaIdentity::new("voxproof-calibration-comparison-v0", "v0")
                .expect("schema"),
            content_digest: ArtifactContentDigest::new(SAMPLE_DIGEST).expect("digest"),
            byte_length: 128,
            binding_context: binding_context(),
        }
    }

    #[test]
    fn complete_inventory_requires_expected_role_set() {
        let context = binding_context();
        let expected = vec![ArtifactRole::DetectorOutput];
        let artifacts = vec![descriptor(
            ArtifactRole::DetectorOutput,
            "artifact-detector",
        )];
        let assessment =
            ArtifactBundle::derive_assessment(&expected, &artifacts, &context).expect("derive");
        assert!(assessment.inventory_complete);
    }
}

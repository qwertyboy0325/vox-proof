use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::reference_identity::{
    ReferenceIdentityIdError, ReferenceRevisionId, validate_identity_value,
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
    pub records: Vec<CueReferenceCoverageRecord>,
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
pub struct CueReferenceCoverageRecord {
    pub cue_id: CueReferenceId,
    pub disposition: ReferenceCueDisposition,
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
    pub missing_cue_ids: Vec<CueReferenceId>,
    pub duplicate_cue_ids: Vec<CueReferenceId>,
    pub unknown_cue_ids: Vec<CueReferenceId>,
    pub unresolved_cue_ids: Vec<CueReferenceId>,
    pub inventory_complete: bool,
    pub reference_resolved: bool,
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
    AssessmentMismatch {
        stored: Box<ReferenceCoverageAssessment>,
        derived: Box<ReferenceCoverageAssessment>,
    },
    CoverageStateMismatch {
        state: ReferenceCoverageState,
        assessment: Box<ReferenceCoverageAssessment>,
    },
    PrimaryAttachmentRequiresCompleteState,
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
    EnvelopeValidation(RunEnvelopeValidationError),
    SealValidation(ReferenceSealValidationError),
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

impl ReferenceCoverage {
    pub fn derive_assessment(
        expected_universe: &ExpectedCueUniverse,
        records: &[CueReferenceCoverageRecord],
    ) -> Result<ReferenceCoverageAssessment, ReferenceCoverageValidationError> {
        validate_expected_universe(expected_universe)?;

        let expected_ids: BTreeSet<CueReferenceId> =
            expected_universe.cue_ids.iter().copied().collect();

        let mut counts: HashMap<CueReferenceId, u32> = HashMap::new();
        for record in records {
            validate_cue_reference_id(record.cue_id)?;
            *counts.entry(record.cue_id).or_default() += 1;
        }

        let mut duplicate_cue_ids = counts
            .iter()
            .filter_map(|(cue_id, count)| (*count > 1).then_some(*cue_id))
            .collect::<Vec<_>>();
        duplicate_cue_ids.sort_unstable();

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

        let mut unresolved_cue_ids = Vec::new();
        let mut seen_unresolved = HashSet::new();
        for record in records {
            if !expected_ids.contains(&record.cue_id) {
                continue;
            }
            if matches!(
                record.disposition,
                ReferenceCueDisposition::Uncertain | ReferenceCueDisposition::Unreviewable
            ) && seen_unresolved.insert(record.cue_id)
            {
                unresolved_cue_ids.push(record.cue_id);
            }
        }
        unresolved_cue_ids.sort_unstable();

        let inventory_complete = missing_cue_ids.is_empty()
            && duplicate_cue_ids.is_empty()
            && unknown_cue_ids.is_empty()
            && observed_unique.len() == expected_ids.len();

        let reference_resolved = inventory_complete && unresolved_cue_ids.is_empty();

        Ok(ReferenceCoverageAssessment {
            expected_count: expected_universe.total_cues,
            observed_unique_count: observed_unique.len() as u32,
            missing_cue_ids,
            duplicate_cue_ids,
            unknown_cue_ids,
            unresolved_cue_ids,
            inventory_complete,
            reference_resolved,
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
        if self.assessment != derived {
            return Err(ReferenceCoverageValidationError::AssessmentMismatch {
                stored: Box::new(self.assessment.clone()),
                derived: Box::new(derived.clone()),
            });
        }

        if self.coverage_state == ReferenceCoverageState::Complete
            && (!derived.inventory_complete || !derived.reference_resolved)
        {
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

        Ok(())
    }
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
    use crate::reference_seal::{REFERENCE_SEAL_SCHEMA, ReferenceProducerClass};
    use crate::run_manifest::{
        ArtifactRole, InputClass, RUN_ENVELOPE_SCHEMA, RunEnvelope, WorkflowObservationMode,
    };

    const SAMPLE_REVISION: &str =
        "rev:sha256-v1:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn primary_posture() -> (RunEnvelope, ReferenceSeal) {
        let envelope = RunEnvelope {
            schema_revision: RUN_ENVELOPE_SCHEMA.to_string(),
            run_id: RunId::new("run-primary").expect("run id"),
            input_identity: InputIdentityReference {
                transcript_revision_id: SAMPLE_REVISION.to_string(),
            },
            calibration_validity: CalibrationValidityMode::BlindReference,
            workflow_observation: WorkflowObservationMode::Disabled,
            input_class: InputClass::SelfOwnedReal,
            qualifies_as_real_material_evidence: false,
            lifecycle_state: RunLifecycleState::ReferenceSealed,
            expected_artifact_roles: vec![ArtifactRole::CueReviewCompletion],
        };

        let seal = ReferenceSeal {
            schema_revision: REFERENCE_SEAL_SCHEMA.to_string(),
            seal_id: ReferenceSealId::new("seal-primary").expect("seal id"),
            run_id: envelope.run_id.clone(),
            input_identity: envelope.input_identity.clone(),
            producer_class: ReferenceProducerClass::HumanBlindReviewer,
            reference_created_before_detector_run: true,
            prior_detector_run_on_same_input: false,
            prior_knowledge_of_detector_targets: false,
            session_terms_visible_during_reference: false,
            external_notes_encode_detector_targets: false,
            seal_state: ReferenceSealState::Sealed,
            calibration_classification: ReferenceCalibrationValidity::BlindReferenceEligible,
            calibration_validity_impact: CalibrationValidityImpact::None,
            reference_revision: ReferenceRevisionId::new("ref-rev-primary").expect("revision id"),
        };

        (envelope, seal)
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
        let records = vec![CueReferenceCoverageRecord {
            cue_id: CueReferenceId::new(1).expect("cue"),
            disposition: ReferenceCueDisposition::NoTranscriptionError,
        }];

        let assessment = ReferenceCoverage::derive_assessment(&universe, &records).expect("derive");
        assert!(!assessment.inventory_complete);
        assert_eq!(
            assessment.missing_cue_ids,
            vec![CueReferenceId::new(2).expect("cue")]
        );
    }

    #[test]
    fn primary_attachment_requires_sealed_seal_and_reference_sealed_lifecycle() {
        let (envelope, seal) = primary_posture();
        let coverage = ReferenceCoverage {
            schema_revision: REFERENCE_COVERAGE_SCHEMA.to_string(),
            coverage_id: ReferenceCoverageId::new("coverage-primary").expect("coverage id"),
            run_id: envelope.run_id.clone(),
            input_identity: envelope.input_identity.clone(),
            seal_id: seal.seal_id.clone(),
            reference_revision: ReferenceRevisionId::new("ref-rev-primary").expect("revision id"),
            coverage_purpose: ReferenceCoveragePurpose::PrimaryBlindCalibration,
            expected_universe: ExpectedCueUniverse {
                total_cues: 1,
                cue_ids: vec![CueReferenceId::new(1).expect("cue")],
            },
            records: vec![CueReferenceCoverageRecord {
                cue_id: CueReferenceId::new(1).expect("cue"),
                disposition: ReferenceCueDisposition::NoTranscriptionError,
            }],
            coverage_state: ReferenceCoverageState::Complete,
            assessment: ReferenceCoverageAssessment {
                expected_count: 1,
                observed_unique_count: 1,
                missing_cue_ids: vec![],
                duplicate_cue_ids: vec![],
                unknown_cue_ids: vec![],
                unresolved_cue_ids: vec![],
                inventory_complete: true,
                reference_resolved: true,
            },
        };

        coverage
            .validate_against(&envelope, &seal)
            .expect("primary attachment");
    }
}

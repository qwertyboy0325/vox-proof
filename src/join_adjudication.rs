use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::artifact_bundle::ArtifactId;
use crate::detector_snapshot::{DetectorProposalId, DetectorSnapshotRevisionId};
use crate::human_final_reference::ReferenceErrorId;
use crate::reference_identity::{ReferenceRevisionId, validate_identity_value};
use crate::run_manifest::{
    InputIdentityReference, RunEnvelope, RunEnvelopeValidationError, RunId, RunIdError,
    RunLifecycleState, validate_opaque_identifier,
};

pub const OVERLAP_ADJUDICATION_SCHEMA: &str = "voxproof-overlap-adjudication-v1";

const EXPECTED_JOIN_CONTRACT_REVISION: &str = "voxproof-detector-reference-join-v1";
const EXPECTED_OVERLAP_RULE_REVISION: &str = "voxproof-overlap-v1";

const ADJUDICATION_REASON_MAX_LEN: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OverlapAdjudicationSetId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OverlapAdjudicationId(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlapAdjudicationSetState {
    Draft,
    Frozen,
    Invalidated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlapAdjudicatorRole {
    OwnerAdjudicator,
    AuthorizedDomainAdjudicator,
    SyntheticFixtureAdjudicator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlapAdjudicationResult {
    SameErrorSameCorrection,
    SameErrorWrongCorrection,
    DifferentError,
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverlapAdjudicationRecord {
    pub adjudication_id: OverlapAdjudicationId,
    pub detector_proposal_id: DetectorProposalId,
    pub reference_error_id: ReferenceErrorId,
    pub join_contract_revision: String,
    pub adjudicator_role: OverlapAdjudicatorRole,
    pub adjudication_result: OverlapAdjudicationResult,
    pub adjudication_reason: String,
    pub adjudicated_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverlapAdjudicationAssessment {
    pub record_count: u32,
    pub duplicate_adjudication_ids: Vec<OverlapAdjudicationId>,
    pub duplicate_pairs: Vec<(DetectorProposalId, ReferenceErrorId)>,
    pub context_consistent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverlapAdjudicationSet {
    pub schema_revision: String,
    pub adjudication_set_id: OverlapAdjudicationSetId,
    pub run_id: RunId,
    pub input_identity: InputIdentityReference,
    pub reference_revision: ReferenceRevisionId,
    pub detector_snapshot_revision: DetectorSnapshotRevisionId,
    pub join_contract_revision: String,
    pub overlap_rule_revision: String,
    pub join_adjudication_artifact_id: ArtifactId,
    pub state: OverlapAdjudicationSetState,
    pub records: Vec<OverlapAdjudicationRecord>,
    pub assessment: OverlapAdjudicationAssessment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlapAdjudicationIdError {
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
pub enum OverlapAdjudicationValidationError {
    MissingSchemaRevision,
    UnsupportedSchemaRevision {
        found: String,
        expected: String,
    },
    InvalidAdjudicationSetId(OverlapAdjudicationIdError),
    InvalidAdjudicationId(OverlapAdjudicationIdError),
    InvalidReferenceRevisionId(crate::reference_identity::ReferenceIdentityIdError),
    ZeroAdjudicationTimestamp,
    EmptyAdjudicationReason,
    AdjudicationReasonTooLong {
        len: usize,
        max: usize,
    },
    UnsupportedJoinContractRevision {
        found: String,
        expected: String,
    },
    UnsupportedOverlapRuleRevision {
        found: String,
        expected: String,
    },
    AssessmentMismatch {
        stored: Box<OverlapAdjudicationAssessment>,
        derived: Box<OverlapAdjudicationAssessment>,
    },
    SetStateMismatch {
        state: OverlapAdjudicationSetState,
        assessment: Box<OverlapAdjudicationAssessment>,
    },
    RunIdMismatch,
    InputIdentityMismatch,
    ReferenceRevisionMismatch,
    DetectorSnapshotRevisionMismatch,
    EnvelopeInvalidated,
    EnvelopeLifecycleIncompatible {
        lifecycle_state: RunLifecycleState,
    },
    AdjudicationSetNotFrozen,
    AdjudicationSetInvalidated,
    EnvelopeValidation(RunEnvelopeValidationError),
}

impl OverlapAdjudicationSetId {
    pub fn new(value: impl Into<String>) -> Result<Self, OverlapAdjudicationIdError> {
        let value = value.into();
        validate_adjudication_id_value(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl OverlapAdjudicationId {
    pub fn new(value: impl Into<String>) -> Result<Self, OverlapAdjudicationIdError> {
        let value = value.into();
        validate_adjudication_id_value(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl OverlapAdjudicationSet {
    pub fn derive_assessment(
        records: &[OverlapAdjudicationRecord],
    ) -> OverlapAdjudicationAssessment {
        let mut adjudication_id_counts = HashMap::new();
        let mut pair_counts = HashMap::new();
        for record in records {
            *adjudication_id_counts
                .entry(record.adjudication_id.clone())
                .or_insert(0u32) += 1;
            *pair_counts
                .entry((
                    record.detector_proposal_id.clone(),
                    record.reference_error_id.clone(),
                ))
                .or_insert(0u32) += 1;
        }

        let mut duplicate_adjudication_ids = adjudication_id_counts
            .iter()
            .filter_map(|(id, count)| (*count > 1).then_some(id.clone()))
            .collect::<Vec<_>>();
        duplicate_adjudication_ids.sort_by(|left, right| left.as_str().cmp(right.as_str()));

        let mut duplicate_pairs = pair_counts
            .iter()
            .filter_map(|(pair, count)| (*count > 1).then_some(pair.clone()))
            .collect::<Vec<_>>();
        duplicate_pairs.sort_by(|left, right| {
            left.0
                .as_str()
                .cmp(right.0.as_str())
                .then_with(|| left.1.as_str().cmp(right.1.as_str()))
        });

        OverlapAdjudicationAssessment {
            record_count: records.len() as u32,
            duplicate_adjudication_ids,
            duplicate_pairs,
            context_consistent: true,
        }
    }

    pub fn validate(&self) -> Result<(), OverlapAdjudicationValidationError> {
        if self.schema_revision.is_empty() {
            return Err(OverlapAdjudicationValidationError::MissingSchemaRevision);
        }

        if self.schema_revision != OVERLAP_ADJUDICATION_SCHEMA {
            return Err(
                OverlapAdjudicationValidationError::UnsupportedSchemaRevision {
                    found: self.schema_revision.clone(),
                    expected: OVERLAP_ADJUDICATION_SCHEMA.to_string(),
                },
            );
        }

        validate_adjudication_id_value(self.adjudication_set_id.as_str())
            .map_err(OverlapAdjudicationValidationError::InvalidAdjudicationSetId)?;

        validate_identity_value(self.reference_revision.as_str())
            .map_err(OverlapAdjudicationValidationError::InvalidReferenceRevisionId)?;

        if self.join_contract_revision != EXPECTED_JOIN_CONTRACT_REVISION {
            return Err(
                OverlapAdjudicationValidationError::UnsupportedJoinContractRevision {
                    found: self.join_contract_revision.clone(),
                    expected: EXPECTED_JOIN_CONTRACT_REVISION.to_string(),
                },
            );
        }

        if self.overlap_rule_revision != EXPECTED_OVERLAP_RULE_REVISION {
            return Err(
                OverlapAdjudicationValidationError::UnsupportedOverlapRuleRevision {
                    found: self.overlap_rule_revision.clone(),
                    expected: EXPECTED_OVERLAP_RULE_REVISION.to_string(),
                },
            );
        }

        for record in &self.records {
            validate_adjudication_record(record)?;
        }

        let derived = Self::derive_assessment(&self.records);
        if self.assessment != derived {
            return Err(OverlapAdjudicationValidationError::AssessmentMismatch {
                stored: Box::new(self.assessment.clone()),
                derived: Box::new(derived.clone()),
            });
        }

        if self.state == OverlapAdjudicationSetState::Frozen
            && (!derived.context_consistent
                || !derived.duplicate_adjudication_ids.is_empty()
                || !derived.duplicate_pairs.is_empty())
        {
            return Err(OverlapAdjudicationValidationError::SetStateMismatch {
                state: self.state,
                assessment: Box::new(derived),
            });
        }

        Ok(())
    }

    pub fn validate_against_envelope(
        &self,
        envelope: &RunEnvelope,
    ) -> Result<(), OverlapAdjudicationValidationError> {
        self.validate()?;

        envelope
            .validate()
            .map_err(OverlapAdjudicationValidationError::EnvelopeValidation)?;

        if envelope.lifecycle_state == RunLifecycleState::Invalidated {
            return Err(OverlapAdjudicationValidationError::EnvelopeInvalidated);
        }

        if self.run_id != envelope.run_id {
            return Err(OverlapAdjudicationValidationError::RunIdMismatch);
        }

        if self.input_identity != envelope.input_identity {
            return Err(OverlapAdjudicationValidationError::InputIdentityMismatch);
        }

        if self.state == OverlapAdjudicationSetState::Invalidated {
            return Err(OverlapAdjudicationValidationError::AdjudicationSetInvalidated);
        }

        Ok(())
    }

    pub fn validate_frozen_for_join(
        &self,
        envelope: &RunEnvelope,
        reference_revision: &ReferenceRevisionId,
        detector_snapshot_revision: &DetectorSnapshotRevisionId,
    ) -> Result<(), OverlapAdjudicationValidationError> {
        self.validate_against_envelope(envelope)?;

        if self.state != OverlapAdjudicationSetState::Frozen {
            return Err(OverlapAdjudicationValidationError::AdjudicationSetNotFrozen);
        }

        if &self.reference_revision != reference_revision {
            return Err(OverlapAdjudicationValidationError::ReferenceRevisionMismatch);
        }

        if &self.detector_snapshot_revision != detector_snapshot_revision {
            return Err(OverlapAdjudicationValidationError::DetectorSnapshotRevisionMismatch);
        }

        Ok(())
    }

    pub fn record_for_pair(
        &self,
        detector_proposal_id: &DetectorProposalId,
        reference_error_id: &ReferenceErrorId,
    ) -> Option<&OverlapAdjudicationRecord> {
        self.records.iter().find(|record| {
            record.detector_proposal_id == *detector_proposal_id
                && record.reference_error_id == *reference_error_id
        })
    }
}

fn validate_adjudication_record(
    record: &OverlapAdjudicationRecord,
) -> Result<(), OverlapAdjudicationValidationError> {
    validate_adjudication_id_value(record.adjudication_id.as_str())
        .map_err(OverlapAdjudicationValidationError::InvalidAdjudicationId)?;

    if record.join_contract_revision != EXPECTED_JOIN_CONTRACT_REVISION {
        return Err(
            OverlapAdjudicationValidationError::UnsupportedJoinContractRevision {
                found: record.join_contract_revision.clone(),
                expected: EXPECTED_JOIN_CONTRACT_REVISION.to_string(),
            },
        );
    }

    if record.adjudicated_at_unix_ms == 0 {
        return Err(OverlapAdjudicationValidationError::ZeroAdjudicationTimestamp);
    }

    if record.adjudication_reason.is_empty() {
        return Err(OverlapAdjudicationValidationError::EmptyAdjudicationReason);
    }

    if record.adjudication_reason.len() > ADJUDICATION_REASON_MAX_LEN {
        return Err(
            OverlapAdjudicationValidationError::AdjudicationReasonTooLong {
                len: record.adjudication_reason.len(),
                max: ADJUDICATION_REASON_MAX_LEN,
            },
        );
    }

    Ok(())
}

pub fn validate_adjudication_id_value(value: &str) -> Result<(), OverlapAdjudicationIdError> {
    validate_opaque_identifier(value).map_err(map_run_id_error)
}

fn map_run_id_error(error: RunIdError) -> OverlapAdjudicationIdError {
    match error {
        RunIdError::Empty => OverlapAdjudicationIdError::Empty,
        RunIdError::TooLong { len, max } => OverlapAdjudicationIdError::TooLong { len, max },
        RunIdError::InvalidCharacter { character } => {
            OverlapAdjudicationIdError::InvalidCharacter { character }
        }
        RunIdError::PathLikeContent => OverlapAdjudicationIdError::PathLikeContent,
        RunIdError::AbsolutePathLike => OverlapAdjudicationIdError::AbsolutePathLike,
        RunIdError::RelativePathLike => OverlapAdjudicationIdError::RelativePathLike,
        RunIdError::HomeDirectoryFragment => OverlapAdjudicationIdError::HomeDirectoryFragment,
        RunIdError::GenerationUnavailable => OverlapAdjudicationIdError::GenerationUnavailable,
    }
}

impl fmt::Display for OverlapAdjudicationSetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for OverlapAdjudicationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub fn overlap_adjudication_from_json(
    json: &str,
) -> Result<OverlapAdjudicationSet, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn overlap_adjudication_to_json(
    set: &OverlapAdjudicationSet,
) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(set)
}

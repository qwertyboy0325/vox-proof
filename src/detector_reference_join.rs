#![allow(clippy::too_many_arguments)]

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

use crate::artifact_bundle::{ArtifactBundle, ArtifactBundleValidationError, ArtifactId};
use crate::detector_snapshot::{
    DetectorProposalId, DetectorProposalRecord, DetectorProposalSnapshot,
    DetectorProposalSnapshotState, DetectorProposalSnapshotValidationError,
    DetectorProposalSourceAnchor, DetectorSnapshotRevisionId,
};
use crate::human_final_reference::{
    HumanFinalReference, HumanFinalReferenceValidationError, ReferenceClass, ReferenceErrorId,
    ReferenceErrorRecord, ReferenceSourceAnchor,
};
use crate::join_adjudication::{
    OverlapAdjudicationResult, OverlapAdjudicationSet, OverlapAdjudicationValidationError,
};
use crate::reference_alignment::cue_id_for_segment_position;
use crate::reference_coverage::{
    ReferenceCoverage, ReferenceCoveragePurpose, ReferenceCoverageValidationError,
};
use crate::reference_identity::{ReferenceRevisionId, VerificationBasis};
use crate::reference_seal::{ReferenceSeal, ReferenceSealId, ReferenceSealValidationError};
use crate::run_manifest::{
    ArtifactRole, CalibrationValidityMode, InputIdentityReference, RunEnvelope,
    RunEnvelopeValidationError, RunId, RunIdError, RunLifecycleState, validate_opaque_identifier,
};

pub const DETECTOR_REFERENCE_JOIN_SCHEMA: &str = "voxproof-detector-reference-join-v1";
pub const OVERLAP_RULE_REVISION: &str = "voxproof-overlap-v1";
pub const CORRECTION_EQUALITY_REVISION: &str = "unicode-nfc-equality-v1";
pub const ALTERNATIVE_CARDINALITY_POLICY: &str = "exactly-one-alternative-v1";

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DetectorReferenceJoinId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DetectorReferenceJoinRevisionId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JoinRecordId(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectorReferenceJoinState {
    Draft,
    RequiresAdjudication,
    Resolved,
    Invalidated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectorReferenceJoinPurpose {
    PrimaryBlindCalibration,
    DiagnosticOnly,
    SyntheticProtocolValidation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceJoinEligibility {
    RecallEligibleTranscriptionError,
    ExcludedVerificationBasis,
    ExcludedReferenceClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JoinAnchorRelation {
    Exact,
    Overlap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JoinCorrectionRelation {
    NfcEqual,
    Different,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JoinEdgeResolution {
    PrimaryAssignment,
    DuplicateProposal,
    OverlapCandidate,
    RejectedDifferentError,
    Ambiguous,
    ExcludedReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectorReferenceMatchDisposition {
    ExactMatch,
    AcceptedOverlap,
    DetectorWrongCorrection,
    DuplicateProposal,
    UnmatchedReference,
    UnmatchedDetector,
    AmbiguousMatch,
    ExcludedFromErrorMetrics,
    OverlapCandidate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetectorReferenceJoinEdge {
    pub join_record_id: JoinRecordId,
    pub detector_proposal_id: DetectorProposalId,
    pub reference_error_id: ReferenceErrorId,
    pub anchor_relation: JoinAnchorRelation,
    pub correction_relation: JoinCorrectionRelation,
    pub reference_eligibility: ReferenceJoinEligibility,
    pub adjudication_id: Option<crate::join_adjudication::OverlapAdjudicationId>,
    pub adjudication_result: Option<OverlapAdjudicationResult>,
    pub resolution: JoinEdgeResolution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetectorJoinDispositionRecord {
    pub detector_proposal_id: DetectorProposalId,
    pub disposition: DetectorReferenceMatchDisposition,
    pub primary_reference_error_id: Option<ReferenceErrorId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceJoinDispositionRecord {
    pub reference_error_id: ReferenceErrorId,
    pub disposition: DetectorReferenceMatchDisposition,
    pub primary_detector_proposal_id: Option<DetectorProposalId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetectorReferenceJoinAssessment {
    pub detector_proposal_count: u32,
    pub reference_record_count: u32,
    pub recall_eligible_reference_count: u32,
    pub exact_match_count: u32,
    pub accepted_overlap_count: u32,
    pub detector_wrong_correction_count: u32,
    pub duplicate_proposal_count: u32,
    pub unmatched_detector_count: u32,
    pub unmatched_reference_count: u32,
    pub ambiguous_match_count: u32,
    pub excluded_reference_count: u32,
    pub unresolved_overlap_edge_count: u32,
    pub detector_primary_assignment_count: u32,
    pub reference_primary_assignment_count: u32,
    pub one_to_one_consistent: bool,
    pub fully_resolved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetectorReferenceJoin {
    pub schema_revision: String,
    pub join_id: DetectorReferenceJoinId,
    pub join_revision: DetectorReferenceJoinRevisionId,
    pub run_id: RunId,
    pub input_identity: InputIdentityReference,
    pub calibration_validity: CalibrationValidityMode,
    pub reference_seal_id: ReferenceSealId,
    pub reference_revision: ReferenceRevisionId,
    pub reference_coverage_id: crate::reference_coverage::ReferenceCoverageId,
    pub detector_snapshot_revision: DetectorSnapshotRevisionId,
    pub detector_output_artifact_id: ArtifactId,
    pub evaluation_join_artifact_id: ArtifactId,
    pub join_purpose: DetectorReferenceJoinPurpose,
    pub overlap_rule_revision: String,
    pub correction_equality_revision: String,
    pub alternative_cardinality_policy: String,
    pub state: DetectorReferenceJoinState,
    pub edges: Vec<DetectorReferenceJoinEdge>,
    pub detector_dispositions: Vec<DetectorJoinDispositionRecord>,
    pub reference_dispositions: Vec<ReferenceJoinDispositionRecord>,
    pub assessment: DetectorReferenceJoinAssessment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectorReferenceJoinContext {
    pub join_id: DetectorReferenceJoinId,
    pub join_revision: DetectorReferenceJoinRevisionId,
    pub evaluation_join_artifact_id: ArtifactId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JoinIdentityIdError {
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
pub enum DetectorReferenceJoinError {
    MissingSchemaRevision,
    UnsupportedSchemaRevision {
        found: String,
        expected: String,
    },
    InvalidJoinId(JoinIdentityIdError),
    InvalidJoinRevisionId(JoinIdentityIdError),
    InvalidJoinRecordId(JoinIdentityIdError),
    UnsupportedPolicyRevision {
        field: &'static str,
        found: String,
        expected: String,
    },
    EnvelopeValidation(RunEnvelopeValidationError),
    SealValidation(ReferenceSealValidationError),
    CoverageValidation(ReferenceCoverageValidationError),
    HumanReferenceValidation(Box<HumanFinalReferenceValidationError>),
    SnapshotValidation(DetectorProposalSnapshotValidationError),
    BundleValidation(ArtifactBundleValidationError),
    AdjudicationValidation(OverlapAdjudicationValidationError),
    RunIdMismatch,
    InputIdentityMismatch,
    ReferenceRevisionMismatch,
    CoverageIdMismatch,
    SnapshotRevisionMismatch,
    DetectorOutputArtifactMismatch,
    EvaluationJoinArtifactMismatch,
    JoinAdjudicationArtifactMismatch,
    JoinPurposeIncompatible {
        purpose: DetectorReferenceJoinPurpose,
    },
    JoinCreationLifecycleIncompatible {
        lifecycle_state: RunLifecycleState,
    },
    JoinHistoricalLifecycleIncompatible {
        lifecycle_state: RunLifecycleState,
    },
    AdjudicationResolutionLifecycleIncompatible {
        lifecycle_state: RunLifecycleState,
    },
    AdjudicationRecordsForbiddenAtDetectorExecution,
    SnapshotNotFrozen,
    ProposalAnchorMappingMismatch {
        detector_proposal_id: DetectorProposalId,
    },
    ReferenceAnchorMappingMismatch {
        reference_error_id: ReferenceErrorId,
    },
    UnsupportedProposalAlternativeCardinality {
        detector_proposal_id: DetectorProposalId,
        observed_count: usize,
    },
    AdjudicationPairNotOverlapEdge {
        detector_proposal_id: DetectorProposalId,
        reference_error_id: ReferenceErrorId,
    },
    MissingAdjudicationRecord {
        detector_proposal_id: DetectorProposalId,
        reference_error_id: ReferenceErrorId,
    },
    AdjudicationCorrectionResultMismatch {
        detector_proposal_id: DetectorProposalId,
        reference_error_id: ReferenceErrorId,
    },
    ConflictingPrimaryAdjudications {
        detector_proposal_id: DetectorProposalId,
    },
    OneDetectorMultiplePrimaries {
        detector_proposal_id: DetectorProposalId,
    },
    OneReferenceMultiplePrimaries {
        reference_error_id: ReferenceErrorId,
    },
    AssessmentMismatch {
        stored: Box<DetectorReferenceJoinAssessment>,
        derived: Box<DetectorReferenceJoinAssessment>,
    },
    JoinStateMismatch {
        state: DetectorReferenceJoinState,
        assessment: Box<DetectorReferenceJoinAssessment>,
    },
    InvalidatedJoinContext,
}

impl DetectorReferenceJoinId {
    pub fn new(value: impl Into<String>) -> Result<Self, JoinIdentityIdError> {
        let value = value.into();
        validate_join_id_value(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl DetectorReferenceJoinRevisionId {
    pub fn new(value: impl Into<String>) -> Result<Self, JoinIdentityIdError> {
        let value = value.into();
        validate_join_id_value(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl JoinRecordId {
    pub fn new(value: impl Into<String>) -> Result<Self, JoinIdentityIdError> {
        let value = value.into();
        validate_join_id_value(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl DetectorReferenceJoin {
    pub fn derive(
        context: &DetectorReferenceJoinContext,
        envelope: &RunEnvelope,
        seal: &ReferenceSeal,
        coverage: &ReferenceCoverage,
        human_reference: &HumanFinalReference,
        detector_snapshot: &DetectorProposalSnapshot,
        artifact_bundle: &ArtifactBundle,
        adjudication_set: Option<&OverlapAdjudicationSet>,
    ) -> Result<Self, DetectorReferenceJoinError> {
        validate_join_creation_lifecycle(envelope.lifecycle_state)?;
        validate_join_inputs(
            envelope,
            seal,
            coverage,
            human_reference,
            detector_snapshot,
            artifact_bundle,
            adjudication_set,
            context,
            DeriveMode::Creation,
        )?;

        if envelope.lifecycle_state == RunLifecycleState::DetectorExecution
            && adjudication_set.is_some_and(|set| !set.records.is_empty())
        {
            return Err(
                DetectorReferenceJoinError::AdjudicationRecordsForbiddenAtDetectorExecution,
            );
        }

        if adjudication_set.is_some_and(|set| !set.records.is_empty())
            && envelope.lifecycle_state != RunLifecycleState::AssistedReview
        {
            return Err(
                DetectorReferenceJoinError::AdjudicationResolutionLifecycleIncompatible {
                    lifecycle_state: envelope.lifecycle_state,
                },
            );
        }

        let purpose = join_purpose_from_coverage(coverage.coverage_purpose)?;
        let join = derive_join_body(
            context,
            envelope,
            seal,
            coverage,
            human_reference,
            detector_snapshot,
            adjudication_set,
            purpose,
        )?;

        join.validate_against(
            envelope,
            seal,
            coverage,
            human_reference,
            detector_snapshot,
            artifact_bundle,
            adjudication_set,
        )?;

        Ok(join)
    }

    pub fn validate_against(
        &self,
        envelope: &RunEnvelope,
        seal: &ReferenceSeal,
        coverage: &ReferenceCoverage,
        human_reference: &HumanFinalReference,
        detector_snapshot: &DetectorProposalSnapshot,
        artifact_bundle: &ArtifactBundle,
        adjudication_set: Option<&OverlapAdjudicationSet>,
    ) -> Result<(), DetectorReferenceJoinError> {
        self.validate()?;

        let mode = if is_join_creation_lifecycle(envelope.lifecycle_state) {
            DeriveMode::Creation
        } else if is_join_historical_lifecycle(envelope.lifecycle_state) {
            DeriveMode::Historical
        } else {
            return Err(
                DetectorReferenceJoinError::JoinHistoricalLifecycleIncompatible {
                    lifecycle_state: envelope.lifecycle_state,
                },
            );
        };

        validate_join_inputs(
            envelope,
            seal,
            coverage,
            human_reference,
            detector_snapshot,
            artifact_bundle,
            adjudication_set,
            &DetectorReferenceJoinContext {
                join_id: self.join_id.clone(),
                join_revision: self.join_revision.clone(),
                evaluation_join_artifact_id: self.evaluation_join_artifact_id.clone(),
            },
            mode,
        )?;

        let purpose = join_purpose_from_coverage(coverage.coverage_purpose)?;
        if self.join_purpose != purpose {
            return Err(DetectorReferenceJoinError::JoinPurposeIncompatible {
                purpose: self.join_purpose,
            });
        }

        let derived = derive_join_body(
            &DetectorReferenceJoinContext {
                join_id: self.join_id.clone(),
                join_revision: self.join_revision.clone(),
                evaluation_join_artifact_id: self.evaluation_join_artifact_id.clone(),
            },
            envelope,
            seal,
            coverage,
            human_reference,
            detector_snapshot,
            adjudication_set,
            purpose,
        )?;

        if self.edges != derived.edges
            || self.detector_dispositions != derived.detector_dispositions
            || self.reference_dispositions != derived.reference_dispositions
            || self.assessment != derived.assessment
            || self.state != derived.state
        {
            return Err(DetectorReferenceJoinError::AssessmentMismatch {
                stored: Box::new(self.assessment.clone()),
                derived: Box::new(derived.assessment.clone()),
            });
        }

        Ok(())
    }

    pub fn validate(&self) -> Result<(), DetectorReferenceJoinError> {
        if self.schema_revision.is_empty() {
            return Err(DetectorReferenceJoinError::MissingSchemaRevision);
        }
        if self.schema_revision != DETECTOR_REFERENCE_JOIN_SCHEMA {
            return Err(DetectorReferenceJoinError::UnsupportedSchemaRevision {
                found: self.schema_revision.clone(),
                expected: DETECTOR_REFERENCE_JOIN_SCHEMA.to_string(),
            });
        }
        validate_policy_revision(
            "overlap_rule_revision",
            &self.overlap_rule_revision,
            OVERLAP_RULE_REVISION,
        )?;
        validate_policy_revision(
            "correction_equality_revision",
            &self.correction_equality_revision,
            CORRECTION_EQUALITY_REVISION,
        )?;
        validate_policy_revision(
            "alternative_cardinality_policy",
            &self.alternative_cardinality_policy,
            ALTERNATIVE_CARDINALITY_POLICY,
        )?;
        validate_join_id_value(self.join_id.as_str())
            .map_err(DetectorReferenceJoinError::InvalidJoinId)?;
        validate_join_id_value(self.join_revision.as_str())
            .map_err(DetectorReferenceJoinError::InvalidJoinRevisionId)?;

        let mut edge_ids = HashSet::new();
        for edge in &self.edges {
            validate_join_id_value(edge.join_record_id.as_str())
                .map_err(DetectorReferenceJoinError::InvalidJoinRecordId)?;
            if !edge_ids.insert(edge.join_record_id.clone()) {
                return Err(DetectorReferenceJoinError::AssessmentMismatch {
                    stored: Box::new(self.assessment.clone()),
                    derived: Box::new(self.assessment.clone()),
                });
            }
        }

        let derived_assessment = derive_assessment_from_records(
            self.detector_dispositions.len() as u32,
            self.reference_dispositions.len() as u32,
            &self.detector_dispositions,
            &self.reference_dispositions,
            &self.edges,
        );

        if self.assessment != derived_assessment {
            return Err(DetectorReferenceJoinError::AssessmentMismatch {
                stored: Box::new(self.assessment.clone()),
                derived: Box::new(derived_assessment),
            });
        }

        if self.state == DetectorReferenceJoinState::Resolved && !derived_assessment.fully_resolved
        {
            return Err(DetectorReferenceJoinError::JoinStateMismatch {
                state: self.state,
                assessment: Box::new(derived_assessment),
            });
        }

        if self.state == DetectorReferenceJoinState::Invalidated {
            return Err(DetectorReferenceJoinError::InvalidatedJoinContext);
        }

        Ok(())
    }
}

enum DeriveMode {
    Creation,
    Historical,
}

fn validate_policy_revision(
    field: &'static str,
    found: &str,
    expected: &str,
) -> Result<(), DetectorReferenceJoinError> {
    if found != expected {
        return Err(DetectorReferenceJoinError::UnsupportedPolicyRevision {
            field,
            found: found.to_string(),
            expected: expected.to_string(),
        });
    }
    Ok(())
}

fn join_purpose_from_coverage(
    purpose: ReferenceCoveragePurpose,
) -> Result<DetectorReferenceJoinPurpose, DetectorReferenceJoinError> {
    Ok(match purpose {
        ReferenceCoveragePurpose::PrimaryBlindCalibration => {
            DetectorReferenceJoinPurpose::PrimaryBlindCalibration
        }
        ReferenceCoveragePurpose::DiagnosticOnly => DetectorReferenceJoinPurpose::DiagnosticOnly,
        ReferenceCoveragePurpose::SyntheticProtocolValidation => {
            DetectorReferenceJoinPurpose::SyntheticProtocolValidation
        }
    })
}

fn is_join_creation_lifecycle(state: RunLifecycleState) -> bool {
    matches!(
        state,
        RunLifecycleState::DetectorExecution | RunLifecycleState::AssistedReview
    )
}

fn is_join_historical_lifecycle(state: RunLifecycleState) -> bool {
    matches!(
        state,
        RunLifecycleState::DetectorExecution
            | RunLifecycleState::AssistedReview
            | RunLifecycleState::Finalized
    )
}

fn validate_join_creation_lifecycle(
    lifecycle_state: RunLifecycleState,
) -> Result<(), DetectorReferenceJoinError> {
    if !is_join_creation_lifecycle(lifecycle_state) {
        return Err(
            DetectorReferenceJoinError::JoinCreationLifecycleIncompatible { lifecycle_state },
        );
    }
    Ok(())
}

fn validate_join_inputs(
    envelope: &RunEnvelope,
    seal: &ReferenceSeal,
    coverage: &ReferenceCoverage,
    human_reference: &HumanFinalReference,
    detector_snapshot: &DetectorProposalSnapshot,
    artifact_bundle: &ArtifactBundle,
    adjudication_set: Option<&OverlapAdjudicationSet>,
    context: &DetectorReferenceJoinContext,
    mode: DeriveMode,
) -> Result<(), DetectorReferenceJoinError> {
    envelope
        .validate()
        .map_err(DetectorReferenceJoinError::EnvelopeValidation)?;

    if envelope.lifecycle_state == RunLifecycleState::Invalidated {
        return Err(DetectorReferenceJoinError::InvalidatedJoinContext);
    }

    match mode {
        DeriveMode::Creation => validate_join_creation_lifecycle(envelope.lifecycle_state)?,
        DeriveMode::Historical => {
            if !is_join_historical_lifecycle(envelope.lifecycle_state) {
                return Err(
                    DetectorReferenceJoinError::JoinHistoricalLifecycleIncompatible {
                        lifecycle_state: envelope.lifecycle_state,
                    },
                );
            }
        }
    }

    seal.validate_historical_context(envelope)
        .map_err(DetectorReferenceJoinError::SealValidation)?;
    coverage
        .validate_historical_context(envelope, seal, Some(human_reference))
        .map_err(DetectorReferenceJoinError::CoverageValidation)?;
    human_reference
        .validate_historical_context(envelope, seal)
        .map_err(|error| DetectorReferenceJoinError::HumanReferenceValidation(Box::new(error)))?;

    if detector_snapshot.state != DetectorProposalSnapshotState::Frozen {
        return Err(DetectorReferenceJoinError::SnapshotNotFrozen);
    }

    detector_snapshot
        .validate_against_bundle(envelope, artifact_bundle)
        .map_err(DetectorReferenceJoinError::SnapshotValidation)?;

    artifact_bundle
        .validate_with_reference_context(
            envelope,
            Some(seal),
            Some(coverage),
            Some(human_reference),
        )
        .map_err(DetectorReferenceJoinError::BundleValidation)?;

    validate_join_bundle_roles(artifact_bundle, context, adjudication_set)?;

    if seal.run_id != envelope.run_id
        || coverage.run_id != envelope.run_id
        || human_reference.run_id != envelope.run_id
        || detector_snapshot.run_id != envelope.run_id
    {
        return Err(DetectorReferenceJoinError::RunIdMismatch);
    }

    if seal.input_identity != envelope.input_identity
        || coverage.input_identity != envelope.input_identity
        || human_reference.input_identity != envelope.input_identity
        || detector_snapshot.input_identity != envelope.input_identity
    {
        return Err(DetectorReferenceJoinError::InputIdentityMismatch);
    }

    if seal.reference_revision != coverage.reference_revision
        || seal.reference_revision != human_reference.reference_revision
    {
        return Err(DetectorReferenceJoinError::ReferenceRevisionMismatch);
    }

    if coverage.coverage_id
        != artifact_bundle
            .binding_context
            .reference_coverage_id
            .as_ref()
            .cloned()
            .unwrap_or(coverage.coverage_id.clone())
        && artifact_bundle
            .binding_context
            .reference_coverage_id
            .is_some()
    {
        return Err(DetectorReferenceJoinError::CoverageIdMismatch);
    }

    if let Some(set) = adjudication_set {
        set.validate_frozen_for_join(
            envelope,
            &seal.reference_revision,
            &detector_snapshot.snapshot_revision,
        )
        .map_err(DetectorReferenceJoinError::AdjudicationValidation)?;

        for record in &set.records {
            let Some(proposal) = detector_snapshot
                .proposals
                .iter()
                .find(|proposal| proposal.detector_proposal_id == record.detector_proposal_id)
            else {
                continue;
            };
            let Some(reference) = human_reference
                .records
                .iter()
                .find(|reference| reference.reference_error_id == record.reference_error_id)
            else {
                continue;
            };
            validate_adjudication_for_overlap(
                record,
                &proposal.source_anchor,
                &reference.source_anchor,
            )?;
        }
    }

    validate_proposal_cardinality(detector_snapshot)?;
    validate_proposal_anchor_mapping(detector_snapshot, coverage)?;
    validate_reference_anchor_mapping(human_reference, coverage)?;

    Ok(())
}

fn validate_join_bundle_roles(
    bundle: &ArtifactBundle,
    context: &DetectorReferenceJoinContext,
    adjudication_set: Option<&OverlapAdjudicationSet>,
) -> Result<(), DetectorReferenceJoinError> {
    let evaluation_joins = bundle
        .artifacts
        .iter()
        .filter(|descriptor| descriptor.role == ArtifactRole::EvaluationJoin)
        .collect::<Vec<_>>();
    if evaluation_joins.len() != 1 {
        return Err(DetectorReferenceJoinError::EvaluationJoinArtifactMismatch);
    }
    if evaluation_joins[0].artifact_id != context.evaluation_join_artifact_id {
        return Err(DetectorReferenceJoinError::EvaluationJoinArtifactMismatch);
    }

    let adjudication_roles = bundle
        .artifacts
        .iter()
        .filter(|descriptor| descriptor.role == ArtifactRole::JoinAdjudication)
        .collect::<Vec<_>>();
    if adjudication_roles.len() != 1 {
        return Err(DetectorReferenceJoinError::JoinAdjudicationArtifactMismatch);
    }

    if let Some(set) = adjudication_set
        && adjudication_roles[0].artifact_id != set.join_adjudication_artifact_id
    {
        return Err(DetectorReferenceJoinError::JoinAdjudicationArtifactMismatch);
    }

    Ok(())
}

fn validate_proposal_cardinality(
    snapshot: &DetectorProposalSnapshot,
) -> Result<(), DetectorReferenceJoinError> {
    for proposal in &snapshot.proposals {
        let count = proposal.alternatives.len();
        if count != 1 {
            return Err(
                DetectorReferenceJoinError::UnsupportedProposalAlternativeCardinality {
                    detector_proposal_id: proposal.detector_proposal_id.clone(),
                    observed_count: count,
                },
            );
        }
    }
    Ok(())
}

fn validate_proposal_anchor_mapping(
    snapshot: &DetectorProposalSnapshot,
    coverage: &ReferenceCoverage,
) -> Result<(), DetectorReferenceJoinError> {
    for proposal in &snapshot.proposals {
        if !proposal_anchor_maps_to_coverage(&proposal.source_anchor, coverage) {
            return Err(DetectorReferenceJoinError::ProposalAnchorMappingMismatch {
                detector_proposal_id: proposal.detector_proposal_id.clone(),
            });
        }
    }
    Ok(())
}

fn validate_reference_anchor_mapping(
    human_reference: &HumanFinalReference,
    coverage: &ReferenceCoverage,
) -> Result<(), DetectorReferenceJoinError> {
    for record in &human_reference.records {
        if !reference_anchor_maps_to_coverage(&record.source_anchor, coverage) {
            return Err(DetectorReferenceJoinError::ReferenceAnchorMappingMismatch {
                reference_error_id: record.reference_error_id.clone(),
            });
        }
    }
    Ok(())
}

fn proposal_anchor_maps_to_coverage(
    anchor: &DetectorProposalSourceAnchor,
    coverage: &ReferenceCoverage,
) -> bool {
    cue_id_for_segment_position(&coverage.expected_universe, anchor.segment_position)
        .map(|expected| expected == anchor.cue_id)
        .unwrap_or(false)
        && anchor.input_identity == coverage.input_identity
}

fn reference_anchor_maps_to_coverage(
    anchor: &ReferenceSourceAnchor,
    coverage: &ReferenceCoverage,
) -> bool {
    cue_id_for_segment_position(&coverage.expected_universe, anchor.segment_position)
        .map(|expected| expected == anchor.cue_id)
        .unwrap_or(false)
        && anchor.input_identity == coverage.input_identity
}

pub fn nfc_correction_equal(left: &str, right: &str) -> bool {
    left.nfc().eq(right.nfc())
}

pub fn anchors_exact(
    proposal: &DetectorProposalSourceAnchor,
    reference: &ReferenceSourceAnchor,
) -> bool {
    proposal.input_identity == reference.input_identity
        && proposal.cue_id == reference.cue_id
        && proposal.segment_position == reference.segment_position
        && proposal.start_byte == reference.start_byte
        && proposal.end_byte == reference.end_byte
}

pub fn anchors_overlap(
    proposal: &DetectorProposalSourceAnchor,
    reference: &ReferenceSourceAnchor,
) -> bool {
    if proposal.input_identity != reference.input_identity
        || proposal.cue_id != reference.cue_id
        || proposal.segment_position != reference.segment_position
    {
        return false;
    }
    if anchors_exact(proposal, reference) {
        return false;
    }
    proposal.start_byte.max(reference.start_byte) < proposal.end_byte.min(reference.end_byte)
}

fn proposal_correction(proposal: &DetectorProposalRecord) -> &str {
    &proposal.alternatives[0].replacement_surface
}

fn reference_eligibility(record: &ReferenceErrorRecord) -> ReferenceJoinEligibility {
    match record.reference_class {
        ReferenceClass::TranscriptionError => match record.verification_basis {
            VerificationBasis::AudioListened | VerificationBasis::MixedSources => {
                ReferenceJoinEligibility::RecallEligibleTranscriptionError
            }
            VerificationBasis::TranscriptContextOnly => {
                ReferenceJoinEligibility::ExcludedVerificationBasis
            }
        },
        ReferenceClass::StylePreference
        | ReferenceClass::Ambiguous
        | ReferenceClass::Unsupported
        | ReferenceClass::NonError => ReferenceJoinEligibility::ExcludedReferenceClass,
    }
}

fn is_te_reference(record: &ReferenceErrorRecord) -> bool {
    record.reference_class == ReferenceClass::TranscriptionError
}

fn is_recall_eligible_for_matching(record: &ReferenceErrorRecord) -> bool {
    reference_eligibility(record) == ReferenceJoinEligibility::RecallEligibleTranscriptionError
}

fn derive_join_body(
    context: &DetectorReferenceJoinContext,
    envelope: &RunEnvelope,
    seal: &ReferenceSeal,
    coverage: &ReferenceCoverage,
    human_reference: &HumanFinalReference,
    detector_snapshot: &DetectorProposalSnapshot,
    adjudication_set: Option<&OverlapAdjudicationSet>,
    purpose: DetectorReferenceJoinPurpose,
) -> Result<DetectorReferenceJoin, DetectorReferenceJoinError> {
    let mut proposals = detector_snapshot.proposals.clone();
    proposals.sort_by(|left, right| {
        left.detector_proposal_id
            .as_str()
            .cmp(right.detector_proposal_id.as_str())
    });

    let mut references = human_reference.records.clone();
    references.sort_by(|left, right| {
        left.reference_error_id
            .as_str()
            .cmp(right.reference_error_id.as_str())
    });

    let mut edges = Vec::new();
    let mut join_record_counter = 0u32;
    let mut primary_detector: HashMap<DetectorProposalId, ReferenceErrorId> = HashMap::new();
    let mut primary_reference: HashMap<ReferenceErrorId, DetectorProposalId> = HashMap::new();
    let mut assigned_detectors = HashSet::new();
    let mut assigned_references = HashSet::new();

    let mut push_edge = |edge: DetectorReferenceJoinEdge| {
        edges.push(edge);
    };

    let make_edge_id = |counter: &mut u32| -> JoinRecordId {
        *counter += 1;
        JoinRecordId::new(format!("join-edge-{counter:06}")).expect("generated join record id")
    };

    // Phase 1: exact anchor, NFC equal
    for reference in &references {
        if !is_recall_eligible_for_matching(reference) {
            continue;
        }
        if assigned_references.contains(&reference.reference_error_id) {
            continue;
        }
        let mut qualifying = proposals
            .iter()
            .filter(|proposal| {
                anchors_exact(&proposal.source_anchor, &reference.source_anchor)
                    && nfc_correction_equal(
                        proposal_correction(proposal),
                        &reference.human_final_surface,
                    )
            })
            .collect::<Vec<_>>();
        qualifying.sort_by(|left, right| {
            left.detector_proposal_id
                .as_str()
                .cmp(right.detector_proposal_id.as_str())
        });
        if qualifying.is_empty() {
            continue;
        }
        let primary = qualifying[0];
        if !assigned_detectors.contains(&primary.detector_proposal_id) {
            push_edge(DetectorReferenceJoinEdge {
                join_record_id: make_edge_id(&mut join_record_counter),
                detector_proposal_id: primary.detector_proposal_id.clone(),
                reference_error_id: reference.reference_error_id.clone(),
                anchor_relation: JoinAnchorRelation::Exact,
                correction_relation: JoinCorrectionRelation::NfcEqual,
                reference_eligibility: reference_eligibility(reference),
                adjudication_id: None,
                adjudication_result: None,
                resolution: JoinEdgeResolution::PrimaryAssignment,
            });
            primary_detector.insert(
                primary.detector_proposal_id.clone(),
                reference.reference_error_id.clone(),
            );
            primary_reference.insert(
                reference.reference_error_id.clone(),
                primary.detector_proposal_id.clone(),
            );
            assigned_detectors.insert(primary.detector_proposal_id.clone());
            assigned_references.insert(reference.reference_error_id.clone());
        }
        for duplicate in qualifying.iter().skip(1) {
            push_edge(DetectorReferenceJoinEdge {
                join_record_id: make_edge_id(&mut join_record_counter),
                detector_proposal_id: duplicate.detector_proposal_id.clone(),
                reference_error_id: reference.reference_error_id.clone(),
                anchor_relation: JoinAnchorRelation::Exact,
                correction_relation: JoinCorrectionRelation::NfcEqual,
                reference_eligibility: reference_eligibility(reference),
                adjudication_id: None,
                adjudication_result: None,
                resolution: JoinEdgeResolution::DuplicateProposal,
            });
        }
    }

    // Phase 2: exact anchor, correction different
    for reference in &references {
        if !is_recall_eligible_for_matching(reference) {
            continue;
        }
        if assigned_references.contains(&reference.reference_error_id) {
            continue;
        }
        let mut qualifying = proposals
            .iter()
            .filter(|proposal| {
                anchors_exact(&proposal.source_anchor, &reference.source_anchor)
                    && !nfc_correction_equal(
                        proposal_correction(proposal),
                        &reference.human_final_surface,
                    )
            })
            .collect::<Vec<_>>();
        qualifying.sort_by(|left, right| {
            left.detector_proposal_id
                .as_str()
                .cmp(right.detector_proposal_id.as_str())
        });
        if qualifying.is_empty() {
            continue;
        }
        let primary = qualifying[0];
        if !assigned_detectors.contains(&primary.detector_proposal_id) {
            push_edge(DetectorReferenceJoinEdge {
                join_record_id: make_edge_id(&mut join_record_counter),
                detector_proposal_id: primary.detector_proposal_id.clone(),
                reference_error_id: reference.reference_error_id.clone(),
                anchor_relation: JoinAnchorRelation::Exact,
                correction_relation: JoinCorrectionRelation::Different,
                reference_eligibility: reference_eligibility(reference),
                adjudication_id: None,
                adjudication_result: None,
                resolution: JoinEdgeResolution::PrimaryAssignment,
            });
            primary_detector.insert(
                primary.detector_proposal_id.clone(),
                reference.reference_error_id.clone(),
            );
            primary_reference.insert(
                reference.reference_error_id.clone(),
                primary.detector_proposal_id.clone(),
            );
            assigned_detectors.insert(primary.detector_proposal_id.clone());
            assigned_references.insert(reference.reference_error_id.clone());
        }
        for duplicate in qualifying.iter().skip(1) {
            push_edge(DetectorReferenceJoinEdge {
                join_record_id: make_edge_id(&mut join_record_counter),
                detector_proposal_id: duplicate.detector_proposal_id.clone(),
                reference_error_id: reference.reference_error_id.clone(),
                anchor_relation: JoinAnchorRelation::Exact,
                correction_relation: JoinCorrectionRelation::Different,
                reference_eligibility: reference_eligibility(reference),
                adjudication_id: None,
                adjudication_result: None,
                resolution: JoinEdgeResolution::DuplicateProposal,
            });
        }
    }

    // Phase 3: non-exact overlap
    let mut unresolved_overlap_pairs = HashSet::new();
    for reference in &references {
        if !is_recall_eligible_for_matching(reference) {
            continue;
        }
        for proposal in &proposals {
            if !anchors_overlap(&proposal.source_anchor, &reference.source_anchor) {
                continue;
            }
            let pair = (
                proposal.detector_proposal_id.clone(),
                reference.reference_error_id.clone(),
            );
            let correction_equal = nfc_correction_equal(
                proposal_correction(proposal),
                &reference.human_final_surface,
            );
            let adjudication_record = adjudication_set.and_then(|set| {
                set.record_for_pair(
                    &proposal.detector_proposal_id,
                    &reference.reference_error_id,
                )
            });

            if let Some(record) = adjudication_record {
                validate_adjudication_for_overlap(
                    record,
                    &proposal.source_anchor,
                    &reference.source_anchor,
                )?;
                let (mut resolution, adjudication_id, adjudication_result) =
                    match record.adjudication_result {
                        OverlapAdjudicationResult::SameErrorSameCorrection => {
                            if !correction_equal {
                                return Err(
                                DetectorReferenceJoinError::AdjudicationCorrectionResultMismatch {
                                    detector_proposal_id: proposal.detector_proposal_id.clone(),
                                    reference_error_id: reference.reference_error_id.clone(),
                                },
                            );
                            }
                            (
                                JoinEdgeResolution::PrimaryAssignment,
                                Some(record.adjudication_id.clone()),
                                Some(record.adjudication_result),
                            )
                        }
                        OverlapAdjudicationResult::SameErrorWrongCorrection => {
                            if correction_equal {
                                return Err(
                                DetectorReferenceJoinError::AdjudicationCorrectionResultMismatch {
                                    detector_proposal_id: proposal.detector_proposal_id.clone(),
                                    reference_error_id: reference.reference_error_id.clone(),
                                },
                            );
                            }
                            (
                                JoinEdgeResolution::PrimaryAssignment,
                                Some(record.adjudication_id.clone()),
                                Some(record.adjudication_result),
                            )
                        }
                        OverlapAdjudicationResult::DifferentError => (
                            JoinEdgeResolution::RejectedDifferentError,
                            Some(record.adjudication_id.clone()),
                            Some(record.adjudication_result),
                        ),
                        OverlapAdjudicationResult::Ambiguous => (
                            JoinEdgeResolution::Ambiguous,
                            Some(record.adjudication_id.clone()),
                            Some(record.adjudication_result),
                        ),
                    };
                if resolution == JoinEdgeResolution::PrimaryAssignment {
                    if assigned_references.contains(&reference.reference_error_id) {
                        resolution = JoinEdgeResolution::DuplicateProposal;
                    } else if assigned_detectors.contains(&proposal.detector_proposal_id) {
                        resolution = JoinEdgeResolution::Ambiguous;
                    }
                }
                push_edge(DetectorReferenceJoinEdge {
                    join_record_id: make_edge_id(&mut join_record_counter),
                    detector_proposal_id: proposal.detector_proposal_id.clone(),
                    reference_error_id: reference.reference_error_id.clone(),
                    anchor_relation: JoinAnchorRelation::Overlap,
                    correction_relation: if correction_equal {
                        JoinCorrectionRelation::NfcEqual
                    } else {
                        JoinCorrectionRelation::Different
                    },
                    reference_eligibility: reference_eligibility(reference),
                    adjudication_id,
                    adjudication_result,
                    resolution,
                });
                if resolution == JoinEdgeResolution::PrimaryAssignment
                    && !assigned_detectors.contains(&proposal.detector_proposal_id)
                    && !assigned_references.contains(&reference.reference_error_id)
                {
                    primary_detector.insert(
                        proposal.detector_proposal_id.clone(),
                        reference.reference_error_id.clone(),
                    );
                    primary_reference.insert(
                        reference.reference_error_id.clone(),
                        proposal.detector_proposal_id.clone(),
                    );
                    assigned_detectors.insert(proposal.detector_proposal_id.clone());
                    assigned_references.insert(reference.reference_error_id.clone());
                }
            } else {
                unresolved_overlap_pairs.insert(pair.clone());
                push_edge(DetectorReferenceJoinEdge {
                    join_record_id: make_edge_id(&mut join_record_counter),
                    detector_proposal_id: proposal.detector_proposal_id.clone(),
                    reference_error_id: reference.reference_error_id.clone(),
                    anchor_relation: JoinAnchorRelation::Overlap,
                    correction_relation: if correction_equal {
                        JoinCorrectionRelation::NfcEqual
                    } else {
                        JoinCorrectionRelation::Different
                    },
                    reference_eligibility: reference_eligibility(reference),
                    adjudication_id: None,
                    adjudication_result: None,
                    resolution: JoinEdgeResolution::OverlapCandidate,
                });
            }
        }
    }

    // Phase 4: primary uniqueness, duplicate selection, and multi-reference ambiguity
    demote_multi_reference_overlap_adjudications(&mut edges);
    resolve_primary_assignment_conflicts(&mut edges);
    primary_detector.clear();
    primary_reference.clear();
    assigned_detectors.clear();
    assigned_references.clear();
    for edge in &edges {
        if edge.resolution != JoinEdgeResolution::PrimaryAssignment {
            continue;
        }
        if assigned_detectors.contains(&edge.detector_proposal_id)
            || assigned_references.contains(&edge.reference_error_id)
        {
            continue;
        }
        primary_detector.insert(
            edge.detector_proposal_id.clone(),
            edge.reference_error_id.clone(),
        );
        primary_reference.insert(
            edge.reference_error_id.clone(),
            edge.detector_proposal_id.clone(),
        );
        assigned_detectors.insert(edge.detector_proposal_id.clone());
        assigned_references.insert(edge.reference_error_id.clone());
    }

    // Phase 5: terminal dispositions
    let mut detector_dispositions = Vec::new();
    let mut reference_dispositions = Vec::new();

    for proposal in &proposals {
        let disposition =
            if let Some(reference) = primary_detector.get(&proposal.detector_proposal_id) {
                let edge = edges.iter().find(|edge| {
                    edge.detector_proposal_id == proposal.detector_proposal_id
                        && edge.reference_error_id == *reference
                        && edge.resolution == JoinEdgeResolution::PrimaryAssignment
                });
                match edge {
                    Some(edge) if edge.anchor_relation == JoinAnchorRelation::Exact => {
                        if edge.correction_relation == JoinCorrectionRelation::NfcEqual {
                            DetectorReferenceMatchDisposition::ExactMatch
                        } else {
                            DetectorReferenceMatchDisposition::DetectorWrongCorrection
                        }
                    }
                    Some(edge) if edge.anchor_relation == JoinAnchorRelation::Overlap => {
                        if edge.correction_relation == JoinCorrectionRelation::NfcEqual {
                            DetectorReferenceMatchDisposition::AcceptedOverlap
                        } else {
                            DetectorReferenceMatchDisposition::DetectorWrongCorrection
                        }
                    }
                    _ => DetectorReferenceMatchDisposition::AmbiguousMatch,
                }
            } else if unresolved_overlap_pairs
                .iter()
                .any(|(detector, _)| detector == &proposal.detector_proposal_id)
            {
                DetectorReferenceMatchDisposition::OverlapCandidate
            } else if edges.iter().any(|edge| {
                edge.detector_proposal_id == proposal.detector_proposal_id
                    && edge.resolution == JoinEdgeResolution::DuplicateProposal
            }) {
                DetectorReferenceMatchDisposition::DuplicateProposal
            } else if edges.iter().any(|edge| {
                edge.detector_proposal_id == proposal.detector_proposal_id
                    && edge.resolution == JoinEdgeResolution::Ambiguous
            }) {
                DetectorReferenceMatchDisposition::AmbiguousMatch
            } else {
                DetectorReferenceMatchDisposition::UnmatchedDetector
            };

        detector_dispositions.push(DetectorJoinDispositionRecord {
            detector_proposal_id: proposal.detector_proposal_id.clone(),
            disposition,
            primary_reference_error_id: primary_detector
                .get(&proposal.detector_proposal_id)
                .cloned(),
        });
    }

    for reference in &references {
        let disposition =
            if let Some(detector) = primary_reference.get(&reference.reference_error_id) {
                let edge = edges.iter().find(|edge| {
                    edge.reference_error_id == reference.reference_error_id
                        && edge.detector_proposal_id == *detector
                        && edge.resolution == JoinEdgeResolution::PrimaryAssignment
                });
                match edge {
                    Some(edge) if edge.anchor_relation == JoinAnchorRelation::Exact => {
                        if edge.correction_relation == JoinCorrectionRelation::NfcEqual {
                            DetectorReferenceMatchDisposition::ExactMatch
                        } else {
                            DetectorReferenceMatchDisposition::DetectorWrongCorrection
                        }
                    }
                    Some(edge) if edge.anchor_relation == JoinAnchorRelation::Overlap => {
                        if edge.correction_relation == JoinCorrectionRelation::NfcEqual {
                            DetectorReferenceMatchDisposition::AcceptedOverlap
                        } else {
                            DetectorReferenceMatchDisposition::DetectorWrongCorrection
                        }
                    }
                    _ => DetectorReferenceMatchDisposition::AmbiguousMatch,
                }
            } else if unresolved_overlap_pairs
                .iter()
                .any(|(_, reference_id)| reference_id == &reference.reference_error_id)
            {
                DetectorReferenceMatchDisposition::OverlapCandidate
            } else if !is_te_reference(reference)
                || reference_eligibility(reference)
                    != ReferenceJoinEligibility::RecallEligibleTranscriptionError
            {
                DetectorReferenceMatchDisposition::ExcludedFromErrorMetrics
            } else if edges.iter().any(|edge| {
                edge.reference_error_id == reference.reference_error_id
                    && edge.resolution == JoinEdgeResolution::Ambiguous
            }) {
                DetectorReferenceMatchDisposition::AmbiguousMatch
            } else if is_te_reference(reference) {
                DetectorReferenceMatchDisposition::UnmatchedReference
            } else {
                DetectorReferenceMatchDisposition::ExcludedFromErrorMetrics
            };

        reference_dispositions.push(ReferenceJoinDispositionRecord {
            reference_error_id: reference.reference_error_id.clone(),
            disposition,
            primary_detector_proposal_id: primary_reference
                .get(&reference.reference_error_id)
                .cloned(),
        });
    }

    detector_dispositions.sort_by(|left, right| {
        left.detector_proposal_id
            .as_str()
            .cmp(right.detector_proposal_id.as_str())
    });
    reference_dispositions.sort_by(|left, right| {
        left.reference_error_id
            .as_str()
            .cmp(right.reference_error_id.as_str())
    });
    edges.sort_by(|left, right| {
        left.join_record_id
            .as_str()
            .cmp(right.join_record_id.as_str())
    });

    let assessment = derive_assessment_from_records(
        proposals.len() as u32,
        references.len() as u32,
        &detector_dispositions,
        &reference_dispositions,
        &edges,
    );

    let state = if assessment.unresolved_overlap_edge_count > 0 {
        DetectorReferenceJoinState::RequiresAdjudication
    } else if assessment.fully_resolved && assessment.one_to_one_consistent {
        DetectorReferenceJoinState::Resolved
    } else {
        DetectorReferenceJoinState::RequiresAdjudication
    };

    Ok(DetectorReferenceJoin {
        schema_revision: DETECTOR_REFERENCE_JOIN_SCHEMA.to_string(),
        join_id: context.join_id.clone(),
        join_revision: context.join_revision.clone(),
        run_id: envelope.run_id.clone(),
        input_identity: envelope.input_identity.clone(),
        calibration_validity: envelope.calibration_validity,
        reference_seal_id: seal.seal_id.clone(),
        reference_revision: seal.reference_revision.clone(),
        reference_coverage_id: coverage.coverage_id.clone(),
        detector_snapshot_revision: detector_snapshot.snapshot_revision.clone(),
        detector_output_artifact_id: detector_snapshot.detector_output_artifact_id.clone(),
        evaluation_join_artifact_id: context.evaluation_join_artifact_id.clone(),
        join_purpose: purpose,
        overlap_rule_revision: OVERLAP_RULE_REVISION.to_string(),
        correction_equality_revision: CORRECTION_EQUALITY_REVISION.to_string(),
        alternative_cardinality_policy: ALTERNATIVE_CARDINALITY_POLICY.to_string(),
        state,
        edges,
        detector_dispositions,
        reference_dispositions,
        assessment,
    })
}

fn demote_multi_reference_overlap_adjudications(edges: &mut [DetectorReferenceJoinEdge]) {
    let mut detector_positive_overlap: HashMap<DetectorProposalId, HashSet<ReferenceErrorId>> =
        HashMap::new();
    for edge in edges.iter() {
        if edge.anchor_relation != JoinAnchorRelation::Overlap {
            continue;
        }
        if matches!(
            edge.adjudication_result,
            Some(OverlapAdjudicationResult::SameErrorSameCorrection)
                | Some(OverlapAdjudicationResult::SameErrorWrongCorrection)
        ) {
            detector_positive_overlap
                .entry(edge.detector_proposal_id.clone())
                .or_default()
                .insert(edge.reference_error_id.clone());
        }
    }

    for (detector, references) in detector_positive_overlap {
        if references.len() <= 1 {
            continue;
        }
        for edge in edges.iter_mut() {
            if edge.detector_proposal_id == detector
                && edge.anchor_relation == JoinAnchorRelation::Overlap
                && references.contains(&edge.reference_error_id)
                && matches!(
                    edge.resolution,
                    JoinEdgeResolution::PrimaryAssignment | JoinEdgeResolution::Ambiguous
                )
            {
                edge.resolution = JoinEdgeResolution::Ambiguous;
            }
        }
    }
}

fn resolve_primary_assignment_conflicts(edges: &mut [DetectorReferenceJoinEdge]) {
    let primary_indices: Vec<usize> = edges
        .iter()
        .enumerate()
        .filter(|(_, edge)| edge.resolution == JoinEdgeResolution::PrimaryAssignment)
        .map(|(index, _)| index)
        .collect();

    let mut detector_groups: HashMap<DetectorProposalId, Vec<usize>> = HashMap::new();
    for index in primary_indices {
        detector_groups
            .entry(edges[index].detector_proposal_id.clone())
            .or_default()
            .push(index);
    }

    for (_detector, indices) in detector_groups {
        let unique_refs = indices
            .iter()
            .map(|index| edges[*index].reference_error_id.clone())
            .collect::<HashSet<_>>();
        if unique_refs.len() <= 1 {
            continue;
        }

        let exact_indices = indices
            .iter()
            .copied()
            .filter(|index| edges[*index].anchor_relation == JoinAnchorRelation::Exact)
            .collect::<Vec<_>>();

        if exact_indices.len() == 1 {
            let keep = exact_indices[0];
            for index in indices {
                if index != keep && edges[index].anchor_relation == JoinAnchorRelation::Overlap {
                    edges[index].resolution = JoinEdgeResolution::Ambiguous;
                }
            }
            continue;
        }

        for index in indices {
            edges[index].resolution = JoinEdgeResolution::Ambiguous;
        }
    }

    let primary_indices: Vec<usize> = edges
        .iter()
        .enumerate()
        .filter(|(_, edge)| edge.resolution == JoinEdgeResolution::PrimaryAssignment)
        .map(|(index, _)| index)
        .collect();

    let mut reference_groups: HashMap<ReferenceErrorId, Vec<usize>> = HashMap::new();
    for index in primary_indices {
        reference_groups
            .entry(edges[index].reference_error_id.clone())
            .or_default()
            .push(index);
    }

    for (_reference, mut indices) in reference_groups {
        if indices.len() <= 1 {
            continue;
        }
        let exact_indices = indices
            .iter()
            .copied()
            .filter(|index| edges[*index].anchor_relation == JoinAnchorRelation::Exact)
            .collect::<Vec<_>>();
        let keep = if exact_indices.is_empty() {
            indices.sort_by(|left, right| {
                edges[*left]
                    .detector_proposal_id
                    .as_str()
                    .cmp(edges[*right].detector_proposal_id.as_str())
            });
            indices[0]
        } else {
            *exact_indices
                .iter()
                .min_by(|left, right| {
                    edges[**left]
                        .detector_proposal_id
                        .as_str()
                        .cmp(edges[**right].detector_proposal_id.as_str())
                })
                .expect("exact primary index")
        };
        for index in indices {
            if index != keep {
                edges[index].resolution = JoinEdgeResolution::DuplicateProposal;
            }
        }
    }

    let mut ambiguous_indices = Vec::new();
    for (index, edge) in edges.iter().enumerate() {
        if edge.resolution != JoinEdgeResolution::PrimaryAssignment {
            continue;
        }
        let competing_detector = edges.iter().any(|other| {
            other.resolution == JoinEdgeResolution::PrimaryAssignment
                && other.reference_error_id == edge.reference_error_id
                && other.detector_proposal_id != edge.detector_proposal_id
        });
        let competing_reference = edges.iter().any(|other| {
            other.resolution == JoinEdgeResolution::PrimaryAssignment
                && other.detector_proposal_id == edge.detector_proposal_id
                && other.reference_error_id != edge.reference_error_id
        });
        if competing_detector || competing_reference {
            ambiguous_indices.push(index);
        }
    }
    for index in ambiguous_indices {
        edges[index].resolution = JoinEdgeResolution::Ambiguous;
    }
}

fn validate_adjudication_for_overlap(
    record: &crate::join_adjudication::OverlapAdjudicationRecord,
    proposal_anchor: &DetectorProposalSourceAnchor,
    reference_anchor: &ReferenceSourceAnchor,
) -> Result<(), DetectorReferenceJoinError> {
    if anchors_exact(proposal_anchor, reference_anchor) {
        return Err(DetectorReferenceJoinError::AdjudicationPairNotOverlapEdge {
            detector_proposal_id: record.detector_proposal_id.clone(),
            reference_error_id: record.reference_error_id.clone(),
        });
    }
    if !anchors_overlap(proposal_anchor, reference_anchor) {
        return Err(DetectorReferenceJoinError::AdjudicationPairNotOverlapEdge {
            detector_proposal_id: record.detector_proposal_id.clone(),
            reference_error_id: record.reference_error_id.clone(),
        });
    }
    Ok(())
}

fn derive_assessment_from_records(
    detector_proposal_count: u32,
    reference_record_count: u32,
    detector_dispositions: &[DetectorJoinDispositionRecord],
    reference_dispositions: &[ReferenceJoinDispositionRecord],
    edges: &[DetectorReferenceJoinEdge],
) -> DetectorReferenceJoinAssessment {
    let recall_eligible_reference_count = reference_dispositions
        .iter()
        .filter(|record| {
            record.disposition != DetectorReferenceMatchDisposition::ExcludedFromErrorMetrics
        })
        .count() as u32;

    let count_disposition = |disposition: DetectorReferenceMatchDisposition| -> u32 {
        detector_dispositions
            .iter()
            .filter(|record| record.disposition == disposition)
            .count() as u32
    };

    let count_reference_disposition = |disposition: DetectorReferenceMatchDisposition| -> u32 {
        reference_dispositions
            .iter()
            .filter(|record| record.disposition == disposition)
            .count() as u32
    };

    let unresolved_overlap_edge_count = edges
        .iter()
        .filter(|edge| edge.resolution == JoinEdgeResolution::OverlapCandidate)
        .count() as u32;

    let detector_primary_assignment_count = detector_dispositions
        .iter()
        .filter(|record| record.primary_reference_error_id.is_some())
        .count() as u32;
    let reference_primary_assignment_count = reference_dispositions
        .iter()
        .filter(|record| record.primary_detector_proposal_id.is_some())
        .count() as u32;

    let one_to_one_consistent = detector_primary_assignment_count
        == reference_primary_assignment_count
        && detector_dispositions.iter().all(|record| {
            record
                .primary_reference_error_id
                .as_ref()
                .map(|reference| {
                    reference_dispositions.iter().any(|other| {
                        other.reference_error_id == *reference
                            && other.primary_detector_proposal_id.as_ref()
                                == Some(&record.detector_proposal_id)
                    })
                })
                .unwrap_or(true)
        });

    let fully_resolved = unresolved_overlap_edge_count == 0 && one_to_one_consistent;

    DetectorReferenceJoinAssessment {
        detector_proposal_count,
        reference_record_count,
        recall_eligible_reference_count,
        exact_match_count: count_disposition(DetectorReferenceMatchDisposition::ExactMatch),
        accepted_overlap_count: count_disposition(
            DetectorReferenceMatchDisposition::AcceptedOverlap,
        ),
        detector_wrong_correction_count: count_disposition(
            DetectorReferenceMatchDisposition::DetectorWrongCorrection,
        ),
        duplicate_proposal_count: count_disposition(
            DetectorReferenceMatchDisposition::DuplicateProposal,
        ),
        unmatched_detector_count: count_disposition(
            DetectorReferenceMatchDisposition::UnmatchedDetector,
        ),
        unmatched_reference_count: count_reference_disposition(
            DetectorReferenceMatchDisposition::UnmatchedReference,
        ),
        ambiguous_match_count: count_disposition(DetectorReferenceMatchDisposition::AmbiguousMatch),
        excluded_reference_count: count_reference_disposition(
            DetectorReferenceMatchDisposition::ExcludedFromErrorMetrics,
        ),
        unresolved_overlap_edge_count,
        detector_primary_assignment_count,
        reference_primary_assignment_count,
        one_to_one_consistent,
        fully_resolved,
    }
}

pub fn validate_join_id_value(value: &str) -> Result<(), JoinIdentityIdError> {
    validate_opaque_identifier(value).map_err(map_run_id_error)
}

fn map_run_id_error(error: RunIdError) -> JoinIdentityIdError {
    match error {
        RunIdError::Empty => JoinIdentityIdError::Empty,
        RunIdError::TooLong { len, max } => JoinIdentityIdError::TooLong { len, max },
        RunIdError::InvalidCharacter { character } => {
            JoinIdentityIdError::InvalidCharacter { character }
        }
        RunIdError::PathLikeContent => JoinIdentityIdError::PathLikeContent,
        RunIdError::AbsolutePathLike => JoinIdentityIdError::AbsolutePathLike,
        RunIdError::RelativePathLike => JoinIdentityIdError::RelativePathLike,
        RunIdError::HomeDirectoryFragment => JoinIdentityIdError::HomeDirectoryFragment,
        RunIdError::GenerationUnavailable => JoinIdentityIdError::GenerationUnavailable,
    }
}

pub fn join_from_json(json: &str) -> Result<DetectorReferenceJoin, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn join_to_json(join: &DetectorReferenceJoin) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(join)
}

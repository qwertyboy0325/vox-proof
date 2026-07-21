#![allow(clippy::too_many_arguments)]

use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::artifact_bundle::{
    ARTIFACT_BUNDLE_SCHEMA, ArtifactBindingContext, ArtifactBundle, ArtifactBundleId,
    ArtifactBundleState, ArtifactBundleValidationError, ArtifactContentDigest, ArtifactDescriptor,
    ArtifactId, ArtifactSchemaIdentity,
};
use crate::detector_reference_join::{
    DETECTOR_REFERENCE_JOIN_SCHEMA, DetectorReferenceJoin, DetectorReferenceJoinContext,
    DetectorReferenceJoinError, DetectorReferenceJoinState,
};
use crate::detector_snapshot::{
    DETECTOR_PROPOSAL_SNAPSHOT_SCHEMA, DetectorProposalSnapshot,
    DetectorProposalSnapshotValidationError,
};
use crate::human_final_reference::{
    HUMAN_FINAL_REFERENCE_SCHEMA, HumanFinalReference, HumanFinalReferenceValidationError,
};
use crate::join_adjudication::{
    OVERLAP_ADJUDICATION_SCHEMA, OverlapAdjudicationSet, OverlapAdjudicationValidationError,
    OverlapAdjudicatorRole,
};
use crate::join_metric_aggregation::{
    JOIN_METRIC_AGGREGATION_SCHEMA, JoinMetricAggregateContext, JoinMetricAggregateSet,
    JoinMetricAggregationError,
};
use crate::join_metric_contribution::{
    JOIN_METRIC_CONTRIBUTION_SCHEMA, JoinMetricContributionContext, JoinMetricContributionError,
    JoinMetricContributionSet, MetricContributionSetState,
};
use crate::reference_coverage::{
    REFERENCE_COVERAGE_SCHEMA, ReferenceCoverage, ReferenceCoveragePurpose,
    ReferenceCoverageValidationError,
};
use crate::reference_seal::{
    CalibrationValidityImpact, REFERENCE_SEAL_SCHEMA, ReferenceCalibrationValidity,
    ReferenceProducerClass, ReferenceSeal, ReferenceSealValidationError,
};
use crate::run_manifest::{
    ArtifactRole, CalibrationValidityMode, InputClass, InputIdentityReference, RUN_ENVELOPE_SCHEMA,
    RunEnvelope, RunEnvelopeValidationError, RunId, RunIdError, RunLifecycleState,
    WorkflowObservationMode,
};

pub const SYNTHETIC_EVALUATION_HARNESS_REVISION: &str = "voxproof-synthetic-evaluation-harness-v1";
pub const SYNTHETIC_PAYLOAD_SERIALIZATION_POLICY: &str = "serde-json-compact-utf8-v1";
pub const SYNTHETIC_PAYLOAD_DIGEST_POLICY: &str = "sha256-payload-bytes-v1";

const FINAL_ARTIFACT_ROLES: [ArtifactRole; 8] = [
    ArtifactRole::ReferenceSeal,
    ArtifactRole::HumanFinalReference,
    ArtifactRole::CueReviewCompletion,
    ArtifactRole::DetectorOutput,
    ArtifactRole::EvaluationJoin,
    ArtifactRole::JoinAdjudication,
    ArtifactRole::MetricContributions,
    ArtifactRole::Metrics,
];

const LIFECYCLE_CHAIN: [RunLifecycleState; 6] = [
    RunLifecycleState::Declared,
    RunLifecycleState::ReferencePreparation,
    RunLifecycleState::ReferenceSealed,
    RunLifecycleState::DetectorExecution,
    RunLifecycleState::AssistedReview,
    RunLifecycleState::Finalized,
];

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SyntheticEvaluationFixtureId(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticEvaluationArtifactIds {
    pub reference_seal: ArtifactId,
    pub human_final_reference: ArtifactId,
    pub cue_review_completion: ArtifactId,
    pub detector_output: ArtifactId,
    pub evaluation_join: ArtifactId,
    pub join_adjudication: ArtifactId,
    pub metric_contributions: ArtifactId,
    pub metrics: ArtifactId,
    pub bundle: ArtifactBundleId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticEvaluationRevisionIds {
    pub join_context: DetectorReferenceJoinContext,
    pub contribution_context: JoinMetricContributionContext,
    pub aggregate_context: JoinMetricAggregateContext,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticEvaluationTimestamps {
    pub reference_sealed_unix_ms: u64,
    pub detector_frozen_unix_ms: u64,
    pub adjudication_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticEvaluationFixture {
    pub fixture_id: SyntheticEvaluationFixtureId,
    pub input_class: InputClass,
    pub qualifies_as_real_material_evidence: bool,
    pub reference_seal: ReferenceSeal,
    pub reference_coverage: ReferenceCoverage,
    pub human_final_reference: HumanFinalReference,
    pub detector_snapshot: DetectorProposalSnapshot,
    pub detector_execution_adjudication_set: OverlapAdjudicationSet,
    pub assisted_review_adjudication_set: Option<OverlapAdjudicationSet>,
    pub artifact_ids: SyntheticEvaluationArtifactIds,
    pub revision_ids: SyntheticEvaluationRevisionIds,
    pub fixed_timestamps: SyntheticEvaluationTimestamps,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyntheticEvaluationCompletionStage {
    DetectorExecution,
    AssistedReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyntheticEvaluationStage {
    ReferenceSealed,
    DetectorSnapshotFrozen,
    JoinRequiresAdjudication,
    JoinResolved,
    ContributionsPending,
    ContributionsComplete,
    AggregatesComplete,
    FinalBundleComplete,
    HistoricalReplayValidated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyntheticEvaluationStageRecord {
    pub stage: SyntheticEvaluationStage,
    pub lifecycle_state: RunLifecycleState,
    pub related_artifact_ids: Vec<ArtifactId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticSerializedArtifact {
    pub artifact_id: ArtifactId,
    pub role: ArtifactRole,
    pub payload_schema: ArtifactSchemaIdentity,
    pub payload_bytes: Vec<u8>,
    pub content_digest: ArtifactContentDigest,
    pub byte_length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticEvaluationHarnessResult {
    pub fixture_id: SyntheticEvaluationFixtureId,
    pub completion_stage: SyntheticEvaluationCompletionStage,
    pub detector_execution_envelope: RunEnvelope,
    pub assisted_review_envelope: RunEnvelope,
    pub finalized_envelope: RunEnvelope,
    pub final_adjudication_set: OverlapAdjudicationSet,
    pub final_join: DetectorReferenceJoin,
    pub final_contributions: JoinMetricContributionSet,
    pub final_aggregates: JoinMetricAggregateSet,
    pub final_bundle: ArtifactBundle,
    pub serialized_payloads: Vec<SyntheticSerializedArtifact>,
    pub execution_trace: Vec<SyntheticEvaluationStageRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticPendingEvaluationResult {
    pub pending_join: DetectorReferenceJoin,
    pub pending_contributions: JoinMetricContributionSet,
    pub aggregation_error: JoinMetricAggregationError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyntheticEvaluationHarnessError {
    InvalidFixtureId(RunIdError),
    NonSyntheticInputClass,
    RealMaterialQualificationForbidden,
    SyntheticSealProducerMismatch,
    SyntheticSealClassificationMismatch,
    SyntheticSealImpactMismatch,
    SyntheticCoveragePurposeMismatch,
    SyntheticJoinPurposeMismatch,
    NonSyntheticAdjudicatorRole,
    DetectorExecutionAdjudicationNotEmpty,
    AssistedReviewAdjudicationRequired,
    AssistedReviewAdjudicationForbidden,
    SourceLineageMismatch {
        field: &'static str,
    },
    InvalidLifecycleTransition(RunEnvelopeValidationError),
    SourceValidationFailure {
        component: &'static str,
    },
    EnvelopeValidation(Box<RunEnvelopeValidationError>),
    SealValidation(ReferenceSealValidationError),
    CoverageValidation(ReferenceCoverageValidationError),
    HumanReferenceValidation(Box<HumanFinalReferenceValidationError>),
    SnapshotValidation(DetectorProposalSnapshotValidationError),
    AdjudicationValidation(OverlapAdjudicationValidationError),
    JoinValidation(DetectorReferenceJoinError),
    ContributionValidation(JoinMetricContributionError),
    AggregationValidation(JoinMetricAggregationError),
    BundleValidation(ArtifactBundleValidationError),
    UnexpectedJoinStateAtCompletion {
        expected_resolved: bool,
        observed: DetectorReferenceJoinState,
    },
    UnexpectedContributionState {
        expected: MetricContributionSetState,
        observed: MetricContributionSetState,
    },
    UnexpectedAggregateSuccessForPending,
    ArtifactRoleInventoryMismatch,
    PayloadSerializationFailure,
    PayloadSchemaMismatch {
        role: ArtifactRole,
    },
    PayloadRoleMismatch {
        artifact_id: ArtifactId,
    },
    PayloadArtifactIdMismatch {
        role: ArtifactRole,
    },
    PayloadDigestMismatch {
        artifact_id: ArtifactId,
    },
    PayloadLengthMismatch {
        artifact_id: ArtifactId,
    },
    DuplicatePayloadRole {
        role: ArtifactRole,
    },
    DuplicatePayloadArtifactId {
        artifact_id: ArtifactId,
    },
    MissingPayload {
        role: ArtifactRole,
    },
    ExtraPayload {
        artifact_id: ArtifactId,
    },
    BootstrapFinalDerivationMismatch {
        component: &'static str,
    },
    RoundTripMismatch {
        role: ArtifactRole,
    },
    HistoricalReplayValidationFailure {
        component: &'static str,
    },
    NonDeterministicReplay,
    IntegerConversionOverflow,
}

impl SyntheticEvaluationFixtureId {
    pub fn new(value: impl Into<String>) -> Result<Self, RunIdError> {
        let value = value.into();
        crate::run_manifest::validate_opaque_identifier(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SyntheticEvaluationFixtureId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub struct SyntheticEvaluationHarness;

impl SyntheticEvaluationHarness {
    pub fn execute(
        fixture: &SyntheticEvaluationFixture,
    ) -> Result<SyntheticEvaluationHarnessResult, SyntheticEvaluationHarnessError> {
        let source_snapshot = capture_source_snapshot(fixture);
        validate_fixture(fixture)?;
        validate_lifecycle_chain(CalibrationValidityMode::BlindReference)?;

        let reference_sealed_envelope =
            build_envelope(fixture, RunLifecycleState::ReferenceSealed)?;
        validate_reference_sealed_stage(fixture, &reference_sealed_envelope)?;

        let detector_execution_envelope =
            build_envelope(fixture, RunLifecycleState::DetectorExecution)?;
        let assisted_review_envelope = build_envelope(fixture, RunLifecycleState::AssistedReview)?;
        let finalized_envelope = build_envelope(fixture, RunLifecycleState::Finalized)?;

        let binding_context = binding_context_from_fixture(fixture);
        let mut trace = Vec::new();
        push_trace(
            &mut trace,
            SyntheticEvaluationStage::ReferenceSealed,
            RunLifecycleState::ReferenceSealed,
            &fixture.artifact_ids,
        );
        push_trace(
            &mut trace,
            SyntheticEvaluationStage::DetectorSnapshotFrozen,
            RunLifecycleState::DetectorExecution,
            &fixture.artifact_ids,
        );

        let (completion_stage, creation_lifecycle, final_adjudication) =
            resolve_completion_path(fixture)?;

        let (final_join, final_contributions, final_aggregates) = derive_complete_chain(
            fixture,
            creation_lifecycle,
            &final_adjudication,
            &binding_context,
            &mut trace,
        )?;

        let (serialized_payloads, final_bundle) = finalize_bundle_two_pass(
            fixture,
            creation_lifecycle,
            &binding_context,
            &final_adjudication,
            &final_join,
            &final_contributions,
            &final_aggregates,
            &mut trace,
        )?;

        historical_replay_validate(
            fixture,
            &finalized_envelope,
            &final_adjudication,
            &final_join,
            &final_contributions,
            &final_aggregates,
            &final_bundle,
        )?;
        push_trace(
            &mut trace,
            SyntheticEvaluationStage::HistoricalReplayValidated,
            RunLifecycleState::Finalized,
            &fixture.artifact_ids,
        );

        verify_source_unchanged(fixture, &source_snapshot)?;

        let result = SyntheticEvaluationHarnessResult {
            fixture_id: fixture.fixture_id.clone(),
            completion_stage,
            detector_execution_envelope,
            assisted_review_envelope,
            finalized_envelope,
            final_adjudication_set: final_adjudication,
            final_join,
            final_contributions,
            final_aggregates,
            final_bundle,
            serialized_payloads,
            execution_trace: trace,
        };

        Self::verify_payload_integrity(&result)?;
        Ok(result)
    }

    pub fn execute_pending_probe(
        fixture: &SyntheticEvaluationFixture,
    ) -> Result<SyntheticPendingEvaluationResult, SyntheticEvaluationHarnessError> {
        let source_snapshot = capture_source_snapshot(fixture);
        validate_fixture(fixture)?;

        if fixture.assisted_review_adjudication_set.is_none() {
            return Err(SyntheticEvaluationHarnessError::AssistedReviewAdjudicationRequired);
        }

        if !fixture
            .detector_execution_adjudication_set
            .records
            .is_empty()
        {
            return Err(SyntheticEvaluationHarnessError::DetectorExecutionAdjudicationNotEmpty);
        }

        let envelope = build_envelope(fixture, RunLifecycleState::DetectorExecution)?;
        let binding_context = binding_context_from_fixture(fixture);

        let pending_join = DetectorReferenceJoin::derive(
            &fixture.revision_ids.join_context,
            &envelope,
            &fixture.reference_seal,
            &fixture.reference_coverage,
            &fixture.human_final_reference,
            &fixture.detector_snapshot,
            &build_bootstrap_bundle(
                fixture,
                &binding_context,
                &fixture.detector_execution_adjudication_set,
                None,
                None,
                None,
            )?,
            &fixture.detector_execution_adjudication_set,
        )
        .map_err(SyntheticEvaluationHarnessError::JoinValidation)?;

        if pending_join.state != DetectorReferenceJoinState::RequiresAdjudication {
            return Err(
                SyntheticEvaluationHarnessError::UnexpectedJoinStateAtCompletion {
                    expected_resolved: false,
                    observed: pending_join.state,
                },
            );
        }

        let bootstrap_bundle = build_bootstrap_bundle(
            fixture,
            &binding_context,
            &fixture.detector_execution_adjudication_set,
            Some(&pending_join),
            None,
            None,
        )?;

        let pending_contributions = JoinMetricContributionSet::derive(
            &fixture.revision_ids.contribution_context,
            &envelope,
            &fixture.reference_seal,
            &fixture.reference_coverage,
            &fixture.human_final_reference,
            &fixture.detector_snapshot,
            &pending_join,
            &fixture.detector_execution_adjudication_set,
            &bootstrap_bundle,
        )
        .map_err(SyntheticEvaluationHarnessError::ContributionValidation)?;

        if pending_contributions.state != MetricContributionSetState::PendingJoinResolution {
            return Err(
                SyntheticEvaluationHarnessError::UnexpectedContributionState {
                    expected: MetricContributionSetState::PendingJoinResolution,
                    observed: pending_contributions.state,
                },
            );
        }

        let aggregation_error = match JoinMetricAggregateSet::derive(
            &fixture.revision_ids.aggregate_context,
            &envelope,
            &fixture.reference_seal,
            &fixture.reference_coverage,
            &fixture.human_final_reference,
            &fixture.detector_snapshot,
            &pending_join,
            &fixture.detector_execution_adjudication_set,
            &pending_contributions,
            &build_bootstrap_bundle(
                fixture,
                &binding_context,
                &fixture.detector_execution_adjudication_set,
                Some(&pending_join),
                Some(&pending_contributions),
                None,
            )?,
        ) {
            Err(JoinMetricAggregationError::PendingContributionRejected) => {
                JoinMetricAggregationError::PendingContributionRejected
            }
            Err(JoinMetricAggregationError::ContributionSetNotComplete)
                if pending_contributions.state
                    == MetricContributionSetState::PendingJoinResolution =>
            {
                JoinMetricAggregationError::PendingContributionRejected
            }
            Err(error) => {
                return Err(SyntheticEvaluationHarnessError::AggregationValidation(
                    error,
                ));
            }
            Ok(_) => {
                return Err(SyntheticEvaluationHarnessError::UnexpectedAggregateSuccessForPending);
            }
        };

        verify_source_unchanged(fixture, &source_snapshot)?;

        Ok(SyntheticPendingEvaluationResult {
            pending_join,
            pending_contributions,
            aggregation_error,
        })
    }

    pub fn verify_payload_integrity(
        result: &SyntheticEvaluationHarnessResult,
    ) -> Result<(), SyntheticEvaluationHarnessError> {
        let mut seen_roles = std::collections::BTreeSet::new();
        let mut seen_ids = std::collections::HashSet::new();

        for payload in &result.serialized_payloads {
            if !seen_roles.insert(payload.role) {
                return Err(SyntheticEvaluationHarnessError::DuplicatePayloadRole {
                    role: payload.role,
                });
            }
            if !seen_ids.insert(payload.artifact_id.clone()) {
                return Err(
                    SyntheticEvaluationHarnessError::DuplicatePayloadArtifactId {
                        artifact_id: payload.artifact_id.clone(),
                    },
                );
            }

            let expected_schema = schema_for_role(payload.role)?;
            if payload.payload_schema != expected_schema {
                return Err(SyntheticEvaluationHarnessError::PayloadSchemaMismatch {
                    role: payload.role,
                });
            }

            let expected_id = artifact_id_for_role(&result.final_bundle, payload.role)?;
            if payload.artifact_id != expected_id {
                return Err(SyntheticEvaluationHarnessError::PayloadArtifactIdMismatch {
                    role: payload.role,
                });
            }

            let digest = compute_payload_digest(&payload.payload_bytes)?;
            if digest != payload.content_digest {
                return Err(SyntheticEvaluationHarnessError::PayloadDigestMismatch {
                    artifact_id: payload.artifact_id.clone(),
                });
            }

            let byte_length = payload_byte_length(&payload.payload_bytes)?;
            if byte_length != payload.byte_length {
                return Err(SyntheticEvaluationHarnessError::PayloadLengthMismatch {
                    artifact_id: payload.artifact_id.clone(),
                });
            }

            let descriptor = result
                .final_bundle
                .artifacts
                .iter()
                .find(|entry| entry.role == payload.role)
                .ok_or(SyntheticEvaluationHarnessError::MissingPayload { role: payload.role })?;

            if descriptor.role != payload.role {
                return Err(SyntheticEvaluationHarnessError::PayloadRoleMismatch {
                    artifact_id: payload.artifact_id.clone(),
                });
            }
            if descriptor.content_digest != payload.content_digest {
                return Err(SyntheticEvaluationHarnessError::PayloadDigestMismatch {
                    artifact_id: payload.artifact_id.clone(),
                });
            }
            if descriptor.byte_length != payload.byte_length {
                return Err(SyntheticEvaluationHarnessError::PayloadLengthMismatch {
                    artifact_id: payload.artifact_id.clone(),
                });
            }
            if descriptor.payload_schema != payload.payload_schema {
                return Err(SyntheticEvaluationHarnessError::PayloadSchemaMismatch {
                    role: payload.role,
                });
            }
        }

        if seen_roles.len() != FINAL_ARTIFACT_ROLES.len() {
            for role in FINAL_ARTIFACT_ROLES {
                if !seen_roles.contains(&role) {
                    return Err(SyntheticEvaluationHarnessError::MissingPayload { role });
                }
            }
        }

        let bundle_ids: std::collections::HashSet<_> = result
            .final_bundle
            .artifacts
            .iter()
            .map(|descriptor| descriptor.artifact_id.clone())
            .collect();
        if bundle_ids.len() != seen_ids.len() || !seen_ids.iter().all(|id| bundle_ids.contains(id))
        {
            return Err(SyntheticEvaluationHarnessError::ArtifactRoleInventoryMismatch);
        }

        Ok(())
    }

    pub fn verify_deterministic_replay(
        fixture: &SyntheticEvaluationFixture,
    ) -> Result<SyntheticEvaluationHarnessResult, SyntheticEvaluationHarnessError> {
        let first = Self::execute(fixture)?;
        let second = Self::execute(fixture)?;
        if first != second {
            return Err(SyntheticEvaluationHarnessError::NonDeterministicReplay);
        }
        Ok(first)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceSnapshot {
    reference_seal: ReferenceSeal,
    reference_coverage: ReferenceCoverage,
    human_final_reference: HumanFinalReference,
    detector_snapshot: DetectorProposalSnapshot,
    detector_execution_adjudication_set: OverlapAdjudicationSet,
    assisted_review_adjudication_set: Option<OverlapAdjudicationSet>,
}

fn capture_source_snapshot(fixture: &SyntheticEvaluationFixture) -> SourceSnapshot {
    SourceSnapshot {
        reference_seal: fixture.reference_seal.clone(),
        reference_coverage: fixture.reference_coverage.clone(),
        human_final_reference: fixture.human_final_reference.clone(),
        detector_snapshot: fixture.detector_snapshot.clone(),
        detector_execution_adjudication_set: fixture.detector_execution_adjudication_set.clone(),
        assisted_review_adjudication_set: fixture.assisted_review_adjudication_set.clone(),
    }
}

fn verify_source_unchanged(
    fixture: &SyntheticEvaluationFixture,
    snapshot: &SourceSnapshot,
) -> Result<(), SyntheticEvaluationHarnessError> {
    if fixture.reference_seal != snapshot.reference_seal
        || fixture.reference_coverage != snapshot.reference_coverage
        || fixture.human_final_reference != snapshot.human_final_reference
        || fixture.detector_snapshot != snapshot.detector_snapshot
        || fixture.detector_execution_adjudication_set
            != snapshot.detector_execution_adjudication_set
        || fixture.assisted_review_adjudication_set != snapshot.assisted_review_adjudication_set
    {
        return Err(SyntheticEvaluationHarnessError::SourceValidationFailure {
            component: "fixture_mutation_detected",
        });
    }
    Ok(())
}

fn validate_fixture(
    fixture: &SyntheticEvaluationFixture,
) -> Result<(), SyntheticEvaluationHarnessError> {
    if fixture.fixture_id.as_str().is_empty() {
        return Err(SyntheticEvaluationHarnessError::InvalidFixtureId(
            RunIdError::Empty,
        ));
    }

    fixture
        .detector_execution_adjudication_set
        .validate()
        .map_err(
            |_| SyntheticEvaluationHarnessError::SourceValidationFailure {
                component: "detector_execution_adjudication_set",
            },
        )?;
    if let Some(set) = &fixture.assisted_review_adjudication_set {
        set.validate().map_err(
            |_| SyntheticEvaluationHarnessError::SourceValidationFailure {
                component: "assisted_review_adjudication_set",
            },
        )?;
    }

    validate_synthetic_posture(fixture)?;
    validate_source_lineage(fixture)?;

    fixture.reference_seal.validate().map_err(|_| {
        SyntheticEvaluationHarnessError::SourceValidationFailure {
            component: "reference_seal",
        }
    })?;
    fixture.reference_coverage.validate().map_err(|_| {
        SyntheticEvaluationHarnessError::SourceValidationFailure {
            component: "reference_coverage",
        }
    })?;
    fixture.human_final_reference.validate().map_err(|_| {
        SyntheticEvaluationHarnessError::SourceValidationFailure {
            component: "human_final_reference",
        }
    })?;
    fixture.detector_snapshot.validate().map_err(|_| {
        SyntheticEvaluationHarnessError::SourceValidationFailure {
            component: "detector_snapshot",
        }
    })?;
    Ok(())
}

fn validate_synthetic_posture(
    fixture: &SyntheticEvaluationFixture,
) -> Result<(), SyntheticEvaluationHarnessError> {
    if fixture.input_class != InputClass::SyntheticProtocolFixture {
        return Err(SyntheticEvaluationHarnessError::NonSyntheticInputClass);
    }
    if fixture.qualifies_as_real_material_evidence {
        return Err(SyntheticEvaluationHarnessError::RealMaterialQualificationForbidden);
    }

    if fixture.reference_seal.producer_class != ReferenceProducerClass::SyntheticFixtureGenerator {
        return Err(SyntheticEvaluationHarnessError::SyntheticSealProducerMismatch);
    }
    if fixture.reference_seal.calibration_classification
        != ReferenceCalibrationValidity::SyntheticProtocolOnly
    {
        return Err(SyntheticEvaluationHarnessError::SyntheticSealClassificationMismatch);
    }
    if fixture.reference_seal.calibration_validity_impact != CalibrationValidityImpact::ProtocolOnly
    {
        return Err(SyntheticEvaluationHarnessError::SyntheticSealImpactMismatch);
    }
    if fixture.reference_coverage.coverage_purpose
        != ReferenceCoveragePurpose::SyntheticProtocolValidation
    {
        return Err(SyntheticEvaluationHarnessError::SyntheticCoveragePurposeMismatch);
    }

    if !fixture
        .detector_execution_adjudication_set
        .records
        .is_empty()
    {
        return Err(SyntheticEvaluationHarnessError::DetectorExecutionAdjudicationNotEmpty);
    }
    for record in &fixture.detector_execution_adjudication_set.records {
        if record.adjudicator_role != OverlapAdjudicatorRole::SyntheticFixtureAdjudicator {
            return Err(SyntheticEvaluationHarnessError::NonSyntheticAdjudicatorRole);
        }
    }
    if let Some(set) = &fixture.assisted_review_adjudication_set {
        for record in &set.records {
            if record.adjudicator_role != OverlapAdjudicatorRole::SyntheticFixtureAdjudicator {
                return Err(SyntheticEvaluationHarnessError::NonSyntheticAdjudicatorRole);
            }
        }
    }

    Ok(())
}

fn validate_source_lineage(
    fixture: &SyntheticEvaluationFixture,
) -> Result<(), SyntheticEvaluationHarnessError> {
    let run_id = &fixture.reference_seal.run_id;
    let input_identity = &fixture.reference_seal.input_identity;
    let reference_revision = &fixture.reference_seal.reference_revision;

    let components: [(
        &str,
        &RunId,
        &InputIdentityReference,
        Option<&crate::reference_identity::ReferenceRevisionId>,
    ); 4] = [
        (
            "reference_coverage.run_id",
            &fixture.reference_coverage.run_id,
            &fixture.reference_coverage.input_identity,
            Some(&fixture.reference_coverage.reference_revision),
        ),
        (
            "human_final_reference.run_id",
            &fixture.human_final_reference.run_id,
            &fixture.human_final_reference.input_identity,
            Some(&fixture.human_final_reference.reference_revision),
        ),
        (
            "detector_snapshot.run_id",
            &fixture.detector_snapshot.run_id,
            &fixture.detector_snapshot.input_identity,
            None,
        ),
        (
            "detector_execution_adjudication_set.run_id",
            &fixture.detector_execution_adjudication_set.run_id,
            &fixture.detector_execution_adjudication_set.input_identity,
            Some(
                &fixture
                    .detector_execution_adjudication_set
                    .reference_revision,
            ),
        ),
    ];

    for (field, component_run_id, component_input, component_revision) in components {
        if component_run_id != run_id {
            return Err(SyntheticEvaluationHarnessError::SourceLineageMismatch { field });
        }
        if component_input != input_identity {
            return Err(SyntheticEvaluationHarnessError::SourceLineageMismatch { field });
        }
        if let Some(revision) = component_revision
            && revision != reference_revision
        {
            return Err(SyntheticEvaluationHarnessError::SourceLineageMismatch { field });
        }
    }

    if fixture.detector_snapshot.detector_output_artifact_id != fixture.artifact_ids.detector_output
    {
        return Err(SyntheticEvaluationHarnessError::SourceLineageMismatch {
            field: "detector_output_artifact_id",
        });
    }
    if fixture
        .revision_ids
        .join_context
        .evaluation_join_artifact_id
        != fixture.artifact_ids.evaluation_join
    {
        return Err(SyntheticEvaluationHarnessError::SourceLineageMismatch {
            field: "evaluation_join_artifact_id",
        });
    }
    if fixture
        .revision_ids
        .join_context
        .join_adjudication_artifact_id
        != fixture.artifact_ids.join_adjudication
    {
        return Err(SyntheticEvaluationHarnessError::SourceLineageMismatch {
            field: "join_adjudication_artifact_id",
        });
    }
    if fixture
        .revision_ids
        .contribution_context
        .metric_contributions_artifact_id
        != fixture.artifact_ids.metric_contributions
    {
        return Err(SyntheticEvaluationHarnessError::SourceLineageMismatch {
            field: "metric_contributions_artifact_id",
        });
    }
    if fixture.revision_ids.aggregate_context.metrics_artifact_id != fixture.artifact_ids.metrics {
        return Err(SyntheticEvaluationHarnessError::SourceLineageMismatch {
            field: "metrics_artifact_id",
        });
    }

    Ok(())
}

fn validate_lifecycle_chain(
    calibration_validity: CalibrationValidityMode,
) -> Result<(), SyntheticEvaluationHarnessError> {
    for window in LIFECYCLE_CHAIN.windows(2) {
        RunEnvelope::validate_transition(window[0], window[1], calibration_validity)
            .map_err(SyntheticEvaluationHarnessError::InvalidLifecycleTransition)?;
    }
    Ok(())
}

fn build_envelope(
    fixture: &SyntheticEvaluationFixture,
    lifecycle_state: RunLifecycleState,
) -> Result<RunEnvelope, SyntheticEvaluationHarnessError> {
    let envelope = RunEnvelope {
        schema_revision: RUN_ENVELOPE_SCHEMA.to_string(),
        run_id: fixture.reference_seal.run_id.clone(),
        input_identity: fixture.reference_seal.input_identity.clone(),
        calibration_validity: CalibrationValidityMode::BlindReference,
        workflow_observation: WorkflowObservationMode::Disabled,
        input_class: fixture.input_class,
        qualifies_as_real_material_evidence: fixture.qualifies_as_real_material_evidence,
        lifecycle_state,
        expected_artifact_roles: FINAL_ARTIFACT_ROLES.to_vec(),
    };
    envelope
        .validate()
        .map_err(|error| SyntheticEvaluationHarnessError::EnvelopeValidation(Box::new(error)))?;
    if envelope.input_class != InputClass::SyntheticProtocolFixture {
        return Err(SyntheticEvaluationHarnessError::NonSyntheticInputClass);
    }
    if envelope.qualifies_as_real_material_evidence {
        return Err(SyntheticEvaluationHarnessError::RealMaterialQualificationForbidden);
    }
    Ok(envelope)
}

fn validate_reference_sealed_stage(
    fixture: &SyntheticEvaluationFixture,
    envelope: &RunEnvelope,
) -> Result<(), SyntheticEvaluationHarnessError> {
    fixture
        .reference_seal
        .validate_with_envelope(envelope)
        .map_err(SyntheticEvaluationHarnessError::SealValidation)?;
    fixture
        .reference_coverage
        .validate_against(
            envelope,
            &fixture.reference_seal,
            Some(&fixture.human_final_reference),
        )
        .map_err(SyntheticEvaluationHarnessError::CoverageValidation)?;
    fixture
        .human_final_reference
        .validate_against(envelope, &fixture.reference_seal)
        .map_err(|error| {
            SyntheticEvaluationHarnessError::HumanReferenceValidation(Box::new(error))
        })?;
    Ok(())
}

fn resolve_completion_path(
    fixture: &SyntheticEvaluationFixture,
) -> Result<
    (
        SyntheticEvaluationCompletionStage,
        RunLifecycleState,
        OverlapAdjudicationSet,
    ),
    SyntheticEvaluationHarnessError,
> {
    let detector_envelope = build_envelope(fixture, RunLifecycleState::DetectorExecution)?;
    let binding_context = binding_context_from_fixture(fixture);
    let bootstrap_bundle = build_bootstrap_bundle(
        fixture,
        &binding_context,
        &fixture.detector_execution_adjudication_set,
        None,
        None,
        None,
    )?;

    let probe_join = DetectorReferenceJoin::derive(
        &fixture.revision_ids.join_context,
        &detector_envelope,
        &fixture.reference_seal,
        &fixture.reference_coverage,
        &fixture.human_final_reference,
        &fixture.detector_snapshot,
        &bootstrap_bundle,
        &fixture.detector_execution_adjudication_set,
    )
    .map_err(SyntheticEvaluationHarnessError::JoinValidation)?;

    match probe_join.state {
        DetectorReferenceJoinState::Resolved => {
            if fixture.assisted_review_adjudication_set.is_some() {
                return Err(SyntheticEvaluationHarnessError::AssistedReviewAdjudicationForbidden);
            }
            Ok((
                SyntheticEvaluationCompletionStage::DetectorExecution,
                RunLifecycleState::DetectorExecution,
                fixture.detector_execution_adjudication_set.clone(),
            ))
        }
        DetectorReferenceJoinState::RequiresAdjudication => {
            let Some(adjudication) = fixture.assisted_review_adjudication_set.clone() else {
                return Err(SyntheticEvaluationHarnessError::AssistedReviewAdjudicationRequired);
            };
            if adjudication.records.is_empty() {
                return Err(SyntheticEvaluationHarnessError::AssistedReviewAdjudicationRequired);
            }
            Ok((
                SyntheticEvaluationCompletionStage::AssistedReview,
                RunLifecycleState::AssistedReview,
                adjudication,
            ))
        }
        other => Err(
            SyntheticEvaluationHarnessError::UnexpectedJoinStateAtCompletion {
                expected_resolved: false,
                observed: other,
            },
        ),
    }
}

#[allow(clippy::type_complexity)]
fn derive_complete_chain(
    fixture: &SyntheticEvaluationFixture,
    creation_lifecycle: RunLifecycleState,
    adjudication: &OverlapAdjudicationSet,
    binding_context: &ArtifactBindingContext,
    trace: &mut Vec<SyntheticEvaluationStageRecord>,
) -> Result<
    (
        DetectorReferenceJoin,
        JoinMetricContributionSet,
        JoinMetricAggregateSet,
    ),
    SyntheticEvaluationHarnessError,
> {
    let envelope = build_envelope(fixture, creation_lifecycle)?;
    let bootstrap_bundle =
        build_bootstrap_bundle(fixture, binding_context, adjudication, None, None, None)?;

    let join = DetectorReferenceJoin::derive(
        &fixture.revision_ids.join_context,
        &envelope,
        &fixture.reference_seal,
        &fixture.reference_coverage,
        &fixture.human_final_reference,
        &fixture.detector_snapshot,
        &bootstrap_bundle,
        adjudication,
    )
    .map_err(SyntheticEvaluationHarnessError::JoinValidation)?;

    match join.state {
        DetectorReferenceJoinState::Resolved => push_trace(
            trace,
            SyntheticEvaluationStage::JoinResolved,
            creation_lifecycle,
            &fixture.artifact_ids,
        ),
        DetectorReferenceJoinState::RequiresAdjudication => push_trace(
            trace,
            SyntheticEvaluationStage::JoinRequiresAdjudication,
            creation_lifecycle,
            &fixture.artifact_ids,
        ),
        other => {
            return Err(
                SyntheticEvaluationHarnessError::UnexpectedJoinStateAtCompletion {
                    expected_resolved: true,
                    observed: other,
                },
            );
        }
    }

    let bootstrap_bundle = build_bootstrap_bundle(
        fixture,
        binding_context,
        adjudication,
        Some(&join),
        None,
        None,
    )?;

    let contributions = JoinMetricContributionSet::derive(
        &fixture.revision_ids.contribution_context,
        &envelope,
        &fixture.reference_seal,
        &fixture.reference_coverage,
        &fixture.human_final_reference,
        &fixture.detector_snapshot,
        &join,
        adjudication,
        &bootstrap_bundle,
    )
    .map_err(SyntheticEvaluationHarnessError::ContributionValidation)?;

    match contributions.state {
        MetricContributionSetState::Complete => push_trace(
            trace,
            SyntheticEvaluationStage::ContributionsComplete,
            creation_lifecycle,
            &fixture.artifact_ids,
        ),
        MetricContributionSetState::PendingJoinResolution => {
            push_trace(
                trace,
                SyntheticEvaluationStage::ContributionsPending,
                creation_lifecycle,
                &fixture.artifact_ids,
            );
            return Err(
                SyntheticEvaluationHarnessError::UnexpectedContributionState {
                    expected: MetricContributionSetState::Complete,
                    observed: contributions.state,
                },
            );
        }
        other => {
            return Err(
                SyntheticEvaluationHarnessError::UnexpectedContributionState {
                    expected: MetricContributionSetState::Complete,
                    observed: other,
                },
            );
        }
    }

    let bootstrap_bundle = build_bootstrap_bundle(
        fixture,
        binding_context,
        adjudication,
        Some(&join),
        Some(&contributions),
        None,
    )?;

    let aggregates = JoinMetricAggregateSet::derive(
        &fixture.revision_ids.aggregate_context,
        &envelope,
        &fixture.reference_seal,
        &fixture.reference_coverage,
        &fixture.human_final_reference,
        &fixture.detector_snapshot,
        &join,
        adjudication,
        &contributions,
        &bootstrap_bundle,
    )
    .map_err(SyntheticEvaluationHarnessError::AggregationValidation)?;

    push_trace(
        trace,
        SyntheticEvaluationStage::AggregatesComplete,
        creation_lifecycle,
        &fixture.artifact_ids,
    );

    Ok((join, contributions, aggregates))
}

#[allow(clippy::type_complexity)]
fn finalize_bundle_two_pass(
    fixture: &SyntheticEvaluationFixture,
    creation_lifecycle: RunLifecycleState,
    binding_context: &ArtifactBindingContext,
    adjudication: &OverlapAdjudicationSet,
    pass_a_join: &DetectorReferenceJoin,
    pass_a_contributions: &JoinMetricContributionSet,
    pass_a_aggregates: &JoinMetricAggregateSet,
    trace: &mut Vec<SyntheticEvaluationStageRecord>,
) -> Result<(Vec<SyntheticSerializedArtifact>, ArtifactBundle), SyntheticEvaluationHarnessError> {
    let envelope = build_envelope(fixture, creation_lifecycle)?;

    let pass_a_payloads = serialize_payload_set(
        fixture,
        adjudication,
        pass_a_join,
        pass_a_contributions,
        pass_a_aggregates,
    )?;
    let pass_a_bundle = build_bundle_from_payloads(
        fixture,
        binding_context,
        &pass_a_payloads,
        ArtifactBundleState::Complete,
    )?;

    let pass_b_join = DetectorReferenceJoin::derive(
        &fixture.revision_ids.join_context,
        &envelope,
        &fixture.reference_seal,
        &fixture.reference_coverage,
        &fixture.human_final_reference,
        &fixture.detector_snapshot,
        &pass_a_bundle,
        adjudication,
    )
    .map_err(SyntheticEvaluationHarnessError::JoinValidation)?;
    if pass_b_join != *pass_a_join {
        return Err(
            SyntheticEvaluationHarnessError::BootstrapFinalDerivationMismatch { component: "join" },
        );
    }

    let pass_b_contributions = JoinMetricContributionSet::derive(
        &fixture.revision_ids.contribution_context,
        &envelope,
        &fixture.reference_seal,
        &fixture.reference_coverage,
        &fixture.human_final_reference,
        &fixture.detector_snapshot,
        &pass_b_join,
        adjudication,
        &pass_a_bundle,
    )
    .map_err(SyntheticEvaluationHarnessError::ContributionValidation)?;
    if pass_b_contributions != *pass_a_contributions {
        return Err(
            SyntheticEvaluationHarnessError::BootstrapFinalDerivationMismatch {
                component: "contributions",
            },
        );
    }

    let pass_b_aggregates = JoinMetricAggregateSet::derive(
        &fixture.revision_ids.aggregate_context,
        &envelope,
        &fixture.reference_seal,
        &fixture.reference_coverage,
        &fixture.human_final_reference,
        &fixture.detector_snapshot,
        &pass_b_join,
        adjudication,
        &pass_b_contributions,
        &pass_a_bundle,
    )
    .map_err(SyntheticEvaluationHarnessError::AggregationValidation)?;
    if pass_b_aggregates != *pass_a_aggregates {
        return Err(
            SyntheticEvaluationHarnessError::BootstrapFinalDerivationMismatch {
                component: "aggregates",
            },
        );
    }

    let final_payloads = serialize_payload_set(
        fixture,
        adjudication,
        &pass_b_join,
        &pass_b_contributions,
        &pass_b_aggregates,
    )?;
    let final_bundle = build_bundle_from_payloads(
        fixture,
        binding_context,
        &final_payloads,
        ArtifactBundleState::Complete,
    )?;

    let pass_c_join = DetectorReferenceJoin::derive(
        &fixture.revision_ids.join_context,
        &envelope,
        &fixture.reference_seal,
        &fixture.reference_coverage,
        &fixture.human_final_reference,
        &fixture.detector_snapshot,
        &final_bundle,
        adjudication,
    )
    .map_err(SyntheticEvaluationHarnessError::JoinValidation)?;
    if pass_c_join != pass_b_join {
        return Err(
            SyntheticEvaluationHarnessError::BootstrapFinalDerivationMismatch {
                component: "join_pass_b",
            },
        );
    }

    let pass_c_contributions = JoinMetricContributionSet::derive(
        &fixture.revision_ids.contribution_context,
        &envelope,
        &fixture.reference_seal,
        &fixture.reference_coverage,
        &fixture.human_final_reference,
        &fixture.detector_snapshot,
        &pass_c_join,
        adjudication,
        &final_bundle,
    )
    .map_err(SyntheticEvaluationHarnessError::ContributionValidation)?;
    if pass_c_contributions != pass_b_contributions {
        return Err(
            SyntheticEvaluationHarnessError::BootstrapFinalDerivationMismatch {
                component: "contributions_pass_b",
            },
        );
    }

    let pass_c_aggregates = JoinMetricAggregateSet::derive(
        &fixture.revision_ids.aggregate_context,
        &envelope,
        &fixture.reference_seal,
        &fixture.reference_coverage,
        &fixture.human_final_reference,
        &fixture.detector_snapshot,
        &pass_c_join,
        adjudication,
        &pass_c_contributions,
        &final_bundle,
    )
    .map_err(SyntheticEvaluationHarnessError::AggregationValidation)?;
    if pass_c_aggregates != pass_b_aggregates {
        return Err(
            SyntheticEvaluationHarnessError::BootstrapFinalDerivationMismatch {
                component: "aggregates_pass_b",
            },
        );
    }

    let reserialized = serialize_payload_set(
        fixture,
        adjudication,
        &pass_c_join,
        &pass_c_contributions,
        &pass_c_aggregates,
    )?;
    if reserialized != final_payloads {
        return Err(SyntheticEvaluationHarnessError::RoundTripMismatch {
            role: ArtifactRole::EvaluationJoin,
        });
    }

    push_trace(
        trace,
        SyntheticEvaluationStage::FinalBundleComplete,
        creation_lifecycle,
        &fixture.artifact_ids,
    );

    Ok((final_payloads, final_bundle))
}

fn historical_replay_validate(
    fixture: &SyntheticEvaluationFixture,
    finalized_envelope: &RunEnvelope,
    adjudication: &OverlapAdjudicationSet,
    join: &DetectorReferenceJoin,
    contributions: &JoinMetricContributionSet,
    aggregates: &JoinMetricAggregateSet,
    bundle: &ArtifactBundle,
) -> Result<(), SyntheticEvaluationHarnessError> {
    fixture
        .reference_seal
        .validate_historical_context(finalized_envelope)
        .map_err(
            |_| SyntheticEvaluationHarnessError::HistoricalReplayValidationFailure {
                component: "reference_seal",
            },
        )?;
    fixture
        .reference_coverage
        .validate_historical_context(
            finalized_envelope,
            &fixture.reference_seal,
            Some(&fixture.human_final_reference),
        )
        .map_err(
            |_| SyntheticEvaluationHarnessError::HistoricalReplayValidationFailure {
                component: "reference_coverage",
            },
        )?;
    fixture
        .human_final_reference
        .validate_historical_context(finalized_envelope, &fixture.reference_seal)
        .map_err(
            |_| SyntheticEvaluationHarnessError::HistoricalReplayValidationFailure {
                component: "human_final_reference",
            },
        )?;
    fixture
        .detector_snapshot
        .validate_against_bundle(finalized_envelope, bundle)
        .map_err(
            |_| SyntheticEvaluationHarnessError::HistoricalReplayValidationFailure {
                component: "detector_snapshot",
            },
        )?;
    adjudication
        .validate_against_envelope(finalized_envelope)
        .map_err(
            |_| SyntheticEvaluationHarnessError::HistoricalReplayValidationFailure {
                component: "adjudication_set",
            },
        )?;
    join.validate_against(
        finalized_envelope,
        &fixture.reference_seal,
        &fixture.reference_coverage,
        &fixture.human_final_reference,
        &fixture.detector_snapshot,
        bundle,
        adjudication,
    )
    .map_err(
        |_| SyntheticEvaluationHarnessError::HistoricalReplayValidationFailure {
            component: "join",
        },
    )?;
    contributions
        .validate_against(
            finalized_envelope,
            &fixture.reference_seal,
            &fixture.reference_coverage,
            &fixture.human_final_reference,
            &fixture.detector_snapshot,
            join,
            adjudication,
            bundle,
        )
        .map_err(
            |_| SyntheticEvaluationHarnessError::HistoricalReplayValidationFailure {
                component: "contributions",
            },
        )?;
    aggregates
        .validate_against(
            finalized_envelope,
            &fixture.reference_seal,
            &fixture.reference_coverage,
            &fixture.human_final_reference,
            &fixture.detector_snapshot,
            join,
            adjudication,
            contributions,
            bundle,
        )
        .map_err(
            |_| SyntheticEvaluationHarnessError::HistoricalReplayValidationFailure {
                component: "aggregates",
            },
        )?;
    bundle
        .validate_with_reference_context(
            finalized_envelope,
            Some(&fixture.reference_seal),
            Some(&fixture.reference_coverage),
            Some(&fixture.human_final_reference),
        )
        .map_err(
            |_| SyntheticEvaluationHarnessError::HistoricalReplayValidationFailure {
                component: "artifact_bundle",
            },
        )?;
    Ok(())
}

fn binding_context_from_fixture(fixture: &SyntheticEvaluationFixture) -> ArtifactBindingContext {
    ArtifactBindingContext {
        run_id: fixture.reference_seal.run_id.clone(),
        input_identity: fixture.reference_seal.input_identity.clone(),
        calibration_validity: CalibrationValidityMode::BlindReference,
        reference_seal_id: Some(fixture.reference_seal.seal_id.clone()),
        reference_coverage_id: Some(fixture.reference_coverage.coverage_id.clone()),
        reference_revision: Some(fixture.reference_seal.reference_revision.clone()),
    }
}

fn build_bootstrap_bundle(
    fixture: &SyntheticEvaluationFixture,
    binding_context: &ArtifactBindingContext,
    adjudication: &OverlapAdjudicationSet,
    join: Option<&DetectorReferenceJoin>,
    contributions: Option<&JoinMetricContributionSet>,
    aggregates: Option<&JoinMetricAggregateSet>,
) -> Result<ArtifactBundle, SyntheticEvaluationHarnessError> {
    let join = join
        .cloned()
        .unwrap_or_else(|| bootstrap_stub_join(fixture));
    let contributions = contributions
        .cloned()
        .unwrap_or_else(|| bootstrap_stub_contributions(fixture));
    let aggregates = aggregates
        .cloned()
        .unwrap_or_else(|| bootstrap_stub_aggregates(fixture));

    let payloads =
        serialize_payload_set(fixture, adjudication, &join, &contributions, &aggregates)?;
    build_bundle_from_payloads(
        fixture,
        binding_context,
        &payloads,
        ArtifactBundleState::Complete,
    )
}

fn bootstrap_stub_join(fixture: &SyntheticEvaluationFixture) -> DetectorReferenceJoin {
    DetectorReferenceJoin {
        schema_revision: DETECTOR_REFERENCE_JOIN_SCHEMA.to_string(),
        join_id: fixture.revision_ids.join_context.join_id.clone(),
        join_revision: fixture.revision_ids.join_context.join_revision.clone(),
        run_id: fixture.reference_seal.run_id.clone(),
        input_identity: fixture.reference_seal.input_identity.clone(),
        calibration_validity: CalibrationValidityMode::BlindReference,
        reference_seal_id: fixture.reference_seal.seal_id.clone(),
        reference_revision: fixture.reference_seal.reference_revision.clone(),
        reference_coverage_id: fixture.reference_coverage.coverage_id.clone(),
        detector_snapshot_revision: fixture.detector_snapshot.snapshot_revision.clone(),
        detector_output_artifact_id: fixture.artifact_ids.detector_output.clone(),
        evaluation_join_artifact_id: fixture.artifact_ids.evaluation_join.clone(),
        join_adjudication_artifact_id: fixture.artifact_ids.join_adjudication.clone(),
        overlap_rule_revision: "voxproof-overlap-v1".to_string(),
        correction_equality_revision: "unicode-nfc-equality-v1".to_string(),
        alternative_cardinality_policy: "exactly-one-alternative-v1".to_string(),
        join_purpose: crate::detector_reference_join::DetectorReferenceJoinPurpose::SyntheticProtocolValidation,
        state: DetectorReferenceJoinState::Draft,
        edges: Vec::new(),
        detector_dispositions: Vec::new(),
        reference_dispositions: Vec::new(),
        assessment: crate::detector_reference_join::DetectorReferenceJoinAssessment {
            detector_proposal_count: 0,
            reference_record_count: 0,
            recall_eligible_reference_count: 0,
            exact_match_count: 0,
            accepted_overlap_count: 0,
            detector_wrong_correction_count: 0,
            duplicate_proposal_count: 0,
            unmatched_detector_count: 0,
            unmatched_reference_count: 0,
            ambiguous_match_count: 0,
            excluded_reference_count: 0,
            unresolved_overlap_edge_count: 0,
            detector_primary_assignment_count: 0,
            reference_primary_assignment_count: 0,
            one_to_one_consistent: true,
            fully_resolved: false,
        },
    }
}

fn bootstrap_stub_contributions(fixture: &SyntheticEvaluationFixture) -> JoinMetricContributionSet {
    JoinMetricContributionSet {
        schema_revision: JOIN_METRIC_CONTRIBUTION_SCHEMA.to_string(),
        contribution_set_id: fixture
            .revision_ids
            .contribution_context
            .contribution_set_id
            .clone(),
        contribution_revision: fixture
            .revision_ids
            .contribution_context
            .contribution_revision
            .clone(),
        run_id: fixture.reference_seal.run_id.clone(),
        input_identity: fixture.reference_seal.input_identity.clone(),
        input_class: InputClass::SyntheticProtocolFixture,
        qualifies_as_real_material_evidence: false,
        reference_seal_id: fixture.reference_seal.seal_id.clone(),
        reference_revision: fixture.reference_seal.reference_revision.clone(),
        reference_coverage_id: fixture.reference_coverage.coverage_id.clone(),
        detector_snapshot_revision: fixture.detector_snapshot.snapshot_revision.clone(),
        detector_output_artifact_id: fixture.artifact_ids.detector_output.clone(),
        join_id: fixture.revision_ids.join_context.join_id.clone(),
        join_revision: fixture.revision_ids.join_context.join_revision.clone(),
        evaluation_join_artifact_id: fixture.artifact_ids.evaluation_join.clone(),
        join_adjudication_artifact_id: fixture.artifact_ids.join_adjudication.clone(),
        metric_contributions_artifact_id: fixture.artifact_ids.metric_contributions.clone(),
        eligibility_policy_revision: crate::join_metric_contribution::PRIMARY_METRIC_ELIGIBILITY_POLICY.to_string(),
        contribution_policy_revision: crate::join_metric_contribution::METRIC_CONTRIBUTION_POLICY.to_string(),
        state: MetricContributionSetState::PendingJoinResolution,
        eligibility: crate::join_metric_contribution::PrimaryMetricEligibilityAssessment {
            policy_revision: crate::join_metric_contribution::PRIMARY_METRIC_ELIGIBILITY_POLICY.to_string(),
            report_class: crate::join_metric_contribution::MetricContributionReportClass::SyntheticProtocolValidation,
            primary_metrics_allowed: false,
            eligible_primary_metrics: Vec::new(),
            blocking_reasons: Vec::new(),
            qualifies_as_real_material_evidence: false,
        },
        detector_contributions: Vec::new(),
        reference_contributions: Vec::new(),
        assessment: crate::join_metric_contribution::MetricContributionSetAssessment {
            detector_source_count: 0,
            detector_contribution_count: 0,
            reference_source_count: 0,
            reference_contribution_count: 0,
            pending_detector_contribution_count: 0,
            pending_reference_contribution_count: 0,
            mapping_complete: false,
        },
    }
}

fn bootstrap_stub_aggregates(fixture: &SyntheticEvaluationFixture) -> JoinMetricAggregateSet {
    JoinMetricAggregateSet {
        schema_revision: JOIN_METRIC_AGGREGATION_SCHEMA.to_string(),
        aggregate_set_id: fixture
            .revision_ids
            .aggregate_context
            .aggregate_set_id
            .clone(),
        aggregate_revision: fixture
            .revision_ids
            .aggregate_context
            .aggregate_revision
            .clone(),
        run_id: fixture.reference_seal.run_id.clone(),
        input_identity: fixture.reference_seal.input_identity.clone(),
        input_class: InputClass::SyntheticProtocolFixture,
        qualifies_as_real_material_evidence: false,
        reference_seal_id: fixture.reference_seal.seal_id.clone(),
        reference_revision: fixture.reference_seal.reference_revision.clone(),
        reference_coverage_id: fixture.reference_coverage.coverage_id.clone(),
        detector_snapshot_revision: fixture.detector_snapshot.snapshot_revision.clone(),
        detector_output_artifact_id: fixture.artifact_ids.detector_output.clone(),
        join_id: fixture.revision_ids.join_context.join_id.clone(),
        join_revision: fixture.revision_ids.join_context.join_revision.clone(),
        evaluation_join_artifact_id: fixture.artifact_ids.evaluation_join.clone(),
        join_adjudication_artifact_id: fixture.artifact_ids.join_adjudication.clone(),
        contribution_set_id: fixture
            .revision_ids
            .contribution_context
            .contribution_set_id
            .clone(),
        contribution_revision: fixture
            .revision_ids
            .contribution_context
            .contribution_revision
            .clone(),
        metric_contributions_artifact_id: fixture.artifact_ids.metric_contributions.clone(),
        metrics_artifact_id: fixture.artifact_ids.metrics.clone(),
        aggregation_policy_revision: crate::join_metric_aggregation::PRIMARY_METRIC_AGGREGATION_POLICY.to_string(),
        zero_denominator_policy_revision: crate::join_metric_aggregation::ZERO_DENOMINATOR_POLICY.to_string(),
        report_class: crate::join_metric_contribution::MetricContributionReportClass::SyntheticProtocolValidation,
        primary_metrics_allowed: false,
        eligible_primary_metrics: Vec::new(),
        blocking_reasons: Vec::new(),
        qualifies_as_primary_metric_evidence: false,
        state: crate::join_metric_aggregation::MetricAggregateSetState::Complete,
        metrics: Vec::new(),
        assessment: crate::join_metric_aggregation::MetricAggregateSetAssessment {
            required_metric_count: 5,
            aggregate_metric_count: 0,
            defined_metric_count: 0,
            undefined_zero_denominator_count: 0,
            all_required_metrics_present: false,
            aggregate_consistent: false,
        },
    }
}

fn serialize_payload_set(
    fixture: &SyntheticEvaluationFixture,
    adjudication: &OverlapAdjudicationSet,
    join: &DetectorReferenceJoin,
    contributions: &JoinMetricContributionSet,
    aggregates: &JoinMetricAggregateSet,
) -> Result<Vec<SyntheticSerializedArtifact>, SyntheticEvaluationHarnessError> {
    let mut payloads = vec![
        serialize_role_payload(
            fixture,
            ArtifactRole::ReferenceSeal,
            &fixture.reference_seal,
        )?,
        serialize_role_payload(
            fixture,
            ArtifactRole::HumanFinalReference,
            &fixture.human_final_reference,
        )?,
        serialize_role_payload(
            fixture,
            ArtifactRole::CueReviewCompletion,
            &fixture.reference_coverage,
        )?,
        serialize_role_payload(
            fixture,
            ArtifactRole::DetectorOutput,
            &fixture.detector_snapshot,
        )?,
        serialize_role_payload(fixture, ArtifactRole::EvaluationJoin, join)?,
        serialize_role_payload(fixture, ArtifactRole::JoinAdjudication, adjudication)?,
        serialize_role_payload(fixture, ArtifactRole::MetricContributions, contributions)?,
        serialize_role_payload(fixture, ArtifactRole::Metrics, aggregates)?,
    ];

    payloads.sort_by_key(|payload| payload.role);
    Ok(payloads)
}

fn serialize_role_payload<T: Serialize>(
    fixture: &SyntheticEvaluationFixture,
    role: ArtifactRole,
    value: &T,
) -> Result<SyntheticSerializedArtifact, SyntheticEvaluationHarnessError> {
    let artifact_id = artifact_id_for_role_from_fixture(fixture, role)?;
    let payload_bytes = serde_json::to_vec(value)
        .map_err(|_| SyntheticEvaluationHarnessError::PayloadSerializationFailure)?;
    let content_digest = compute_payload_digest(&payload_bytes)?;
    let byte_length = payload_byte_length(&payload_bytes)?;
    Ok(SyntheticSerializedArtifact {
        artifact_id,
        role,
        payload_schema: schema_for_role(role)?,
        payload_bytes,
        content_digest,
        byte_length,
    })
}

fn build_bundle_from_payloads(
    fixture: &SyntheticEvaluationFixture,
    binding_context: &ArtifactBindingContext,
    payloads: &[SyntheticSerializedArtifact],
    bundle_state: ArtifactBundleState,
) -> Result<ArtifactBundle, SyntheticEvaluationHarnessError> {
    let mut artifacts = Vec::with_capacity(payloads.len());
    for payload in payloads {
        artifacts.push(ArtifactDescriptor {
            artifact_id: payload.artifact_id.clone(),
            role: payload.role,
            payload_schema: payload.payload_schema.clone(),
            content_digest: payload.content_digest.clone(),
            byte_length: payload.byte_length,
            binding_context: binding_context.clone(),
        });
    }
    artifacts.sort_by_key(|descriptor| descriptor.role);

    let expected_roles = FINAL_ARTIFACT_ROLES.to_vec();
    let assessment =
        ArtifactBundle::derive_assessment(&expected_roles, &artifacts, binding_context)
            .map_err(SyntheticEvaluationHarnessError::BundleValidation)?;

    let bundle = ArtifactBundle {
        schema_revision: ARTIFACT_BUNDLE_SCHEMA.to_string(),
        bundle_id: fixture.artifact_ids.bundle.clone(),
        binding_context: binding_context.clone(),
        expected_roles,
        artifacts,
        bundle_state,
        assessment,
    };
    bundle
        .validate()
        .map_err(SyntheticEvaluationHarnessError::BundleValidation)?;
    Ok(bundle)
}

fn schema_for_role(
    role: ArtifactRole,
) -> Result<ArtifactSchemaIdentity, SyntheticEvaluationHarnessError> {
    let schema_id = match role {
        ArtifactRole::ReferenceSeal => REFERENCE_SEAL_SCHEMA,
        ArtifactRole::HumanFinalReference => HUMAN_FINAL_REFERENCE_SCHEMA,
        ArtifactRole::CueReviewCompletion => REFERENCE_COVERAGE_SCHEMA,
        ArtifactRole::DetectorOutput => DETECTOR_PROPOSAL_SNAPSHOT_SCHEMA,
        ArtifactRole::EvaluationJoin => DETECTOR_REFERENCE_JOIN_SCHEMA,
        ArtifactRole::JoinAdjudication => OVERLAP_ADJUDICATION_SCHEMA,
        ArtifactRole::MetricContributions => JOIN_METRIC_CONTRIBUTION_SCHEMA,
        ArtifactRole::Metrics => JOIN_METRIC_AGGREGATION_SCHEMA,
        _ => return Err(SyntheticEvaluationHarnessError::PayloadSchemaMismatch { role }),
    };
    ArtifactSchemaIdentity::new(schema_id, "v1")
        .map_err(|_| SyntheticEvaluationHarnessError::PayloadSchemaMismatch { role })
}

fn artifact_id_for_role_from_fixture(
    fixture: &SyntheticEvaluationFixture,
    role: ArtifactRole,
) -> Result<ArtifactId, SyntheticEvaluationHarnessError> {
    Ok(match role {
        ArtifactRole::ReferenceSeal => fixture.artifact_ids.reference_seal.clone(),
        ArtifactRole::HumanFinalReference => fixture.artifact_ids.human_final_reference.clone(),
        ArtifactRole::CueReviewCompletion => fixture.artifact_ids.cue_review_completion.clone(),
        ArtifactRole::DetectorOutput => fixture.artifact_ids.detector_output.clone(),
        ArtifactRole::EvaluationJoin => fixture.artifact_ids.evaluation_join.clone(),
        ArtifactRole::JoinAdjudication => fixture.artifact_ids.join_adjudication.clone(),
        ArtifactRole::MetricContributions => fixture.artifact_ids.metric_contributions.clone(),
        ArtifactRole::Metrics => fixture.artifact_ids.metrics.clone(),
        _ => {
            return Err(SyntheticEvaluationHarnessError::MissingPayload { role });
        }
    })
}

fn artifact_id_for_role(
    bundle: &ArtifactBundle,
    role: ArtifactRole,
) -> Result<ArtifactId, SyntheticEvaluationHarnessError> {
    bundle
        .artifacts
        .iter()
        .find(|descriptor| descriptor.role == role)
        .map(|descriptor| descriptor.artifact_id.clone())
        .ok_or(SyntheticEvaluationHarnessError::MissingPayload { role })
}

fn compute_payload_digest(
    bytes: &[u8],
) -> Result<ArtifactContentDigest, SyntheticEvaluationHarnessError> {
    let hash = Sha256::digest(bytes);
    let hex = hash
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    ArtifactContentDigest::new(format!("sha256:{hex}")).map_err(|_| {
        SyntheticEvaluationHarnessError::PayloadDigestMismatch {
            artifact_id: ArtifactId::new("unknown").expect("artifact id"),
        }
    })
}

fn payload_byte_length(bytes: &[u8]) -> Result<u64, SyntheticEvaluationHarnessError> {
    bytes
        .len()
        .try_into()
        .map_err(|_| SyntheticEvaluationHarnessError::IntegerConversionOverflow)
}

fn push_trace(
    trace: &mut Vec<SyntheticEvaluationStageRecord>,
    stage: SyntheticEvaluationStage,
    lifecycle_state: RunLifecycleState,
    artifact_ids: &SyntheticEvaluationArtifactIds,
) {
    let mut related_artifact_ids = vec![
        artifact_ids.reference_seal.clone(),
        artifact_ids.human_final_reference.clone(),
        artifact_ids.cue_review_completion.clone(),
        artifact_ids.detector_output.clone(),
        artifact_ids.evaluation_join.clone(),
        artifact_ids.join_adjudication.clone(),
        artifact_ids.metric_contributions.clone(),
        artifact_ids.metrics.clone(),
    ];
    related_artifact_ids.sort_by_key(|id| id.as_str().to_string());
    related_artifact_ids.dedup();
    trace.push(SyntheticEvaluationStageRecord {
        stage,
        lifecycle_state,
        related_artifact_ids,
    });
}

#![allow(clippy::too_many_arguments)]

use std::fmt;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::artifact_bundle::{
    ARTIFACT_BUNDLE_SCHEMA, ArtifactBindingContext, ArtifactBundle, ArtifactBundleId,
    ArtifactBundleState, ArtifactBundleValidationError, ArtifactContentDigest, ArtifactDescriptor,
    ArtifactId, ArtifactSchemaIdentity,
};
use crate::detector_reference_join::{
    DETECTOR_REFERENCE_JOIN_SCHEMA, DetectorReferenceJoin, DetectorReferenceJoinContext,
    DetectorReferenceJoinError, DetectorReferenceJoinPurpose, DetectorReferenceJoinState,
    JoinAnchorRelation, JoinEdgeResolution, Phase3OverlapPair, join_from_json,
};
use crate::detector_snapshot::{
    DETECTOR_PROPOSAL_SNAPSHOT_SCHEMA, DetectorProposalSnapshot,
    DetectorProposalSnapshotValidationError, detector_proposal_snapshot_from_json,
};
use crate::human_final_reference::{
    HUMAN_FINAL_REFERENCE_SCHEMA, HumanFinalReference, human_final_reference_from_json,
};
use crate::input_authorization::{
    INPUT_AUTHORIZATION_SCHEMA, InputAuthorization, input_authorization_from_json,
};
use crate::join_adjudication::{
    OVERLAP_ADJUDICATION_SCHEMA, OverlapAdjudicationSet, OverlapAdjudicationValidationError,
    OverlapAdjudicatorRole, overlap_adjudication_from_json,
};
use crate::join_metric_aggregation::{
    JOIN_METRIC_AGGREGATION_SCHEMA, JoinMetricAggregateContext, JoinMetricAggregateSet,
    JoinMetricAggregationError, aggregate_from_json,
};
use crate::join_metric_contribution::{
    JOIN_METRIC_CONTRIBUTION_SCHEMA, JoinMetricContributionContext, JoinMetricContributionError,
    JoinMetricContributionSet, MetricContributionReportClass, MetricContributionSetState,
    contribution_from_json,
};
use crate::real_transcript_evaluation_runner::{
    RealTranscriptEvaluationRunRequest, RealTranscriptEvaluationRunnerContractError,
    ValidatedRealTranscriptEvaluationRunPlan, validate_real_transcript_evaluation_run_request,
};
use crate::reference_coverage::{REFERENCE_COVERAGE_SCHEMA, ReferenceCoverage, coverage_from_json};
use crate::reference_identity::ReferenceRevisionId;
use crate::reference_seal::{REFERENCE_SEAL_SCHEMA, ReferenceSeal, seal_from_json};
use crate::run_manifest::{
    ArtifactRole, CalibrationValidityMode, InputIdentityReference, RunEnvelope, RunId,
    RunLifecycleState,
};

pub const REAL_TRANSCRIPT_EVALUATION_EXECUTION_REVISION: &str =
    "voxproof-real-transcript-evaluation-execution-v1";
pub const REAL_PAYLOAD_SERIALIZATION_POLICY: &str = "serde-json-compact-utf8-v1";
pub const REAL_PAYLOAD_DIGEST_POLICY: &str = "sha256-payload-bytes-v1";

const FINAL_ARTIFACT_ROLES: [ArtifactRole; 9] = [
    ArtifactRole::InputAuthorization,
    ArtifactRole::ReferenceSeal,
    ArtifactRole::HumanFinalReference,
    ArtifactRole::CueReviewCompletion,
    ArtifactRole::DetectorOutput,
    ArtifactRole::EvaluationJoin,
    ArtifactRole::JoinAdjudication,
    ArtifactRole::MetricContributions,
    ArtifactRole::Metrics,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealTranscriptEvaluationExecutionInput {
    pub detector_snapshot: DetectorProposalSnapshot,
    pub detector_execution_adjudication_set: OverlapAdjudicationSet,
    pub assisted_review_adjudication_set: Option<OverlapAdjudicationSet>,
    pub artifact_ids: RealTranscriptEvaluationArtifactIds,
    pub revision_ids: RealTranscriptEvaluationRevisionIds,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealTranscriptEvaluationArtifactIds {
    pub input_authorization: ArtifactId,
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
pub struct RealTranscriptEvaluationRevisionIds {
    pub join_context: DetectorReferenceJoinContext,
    pub contribution_context: JoinMetricContributionContext,
    pub aggregate_context: JoinMetricAggregateContext,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum RealTranscriptEvaluationExecutionOutcome {
    RequiresHumanAdjudication(RealTranscriptEvaluationPendingResult),
    Completed(RealTranscriptEvaluationCompletedResult),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealTranscriptEvaluationPendingResult {
    pub validated_plan: ValidatedRealTranscriptEvaluationRunPlan,
    pub pending_join: DetectorReferenceJoin,
    pub pending_contributions: JoinMetricContributionSet,
    pub required_human_adjudication: RequiredHumanOverlapAdjudication,
    pub execution_trace: Vec<RealTranscriptEvaluationStageRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequiredHumanOverlapAdjudication {
    pub run_id: RunId,
    pub input_identity: InputIdentityReference,
    pub reference_revision: ReferenceRevisionId,
    pub detector_snapshot_revision: crate::detector_snapshot::DetectorSnapshotRevisionId,
    pub join_adjudication_artifact_id: ArtifactId,
    pub overlap_pairs: Vec<Phase3OverlapPair>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealTranscriptEvaluationCompletedResult {
    pub request: RealTranscriptEvaluationRunRequest,
    pub validated_plan: ValidatedRealTranscriptEvaluationRunPlan,
    pub completion_stage: RealTranscriptEvaluationCompletionStage,
    pub detector_snapshot: DetectorProposalSnapshot,
    pub final_adjudication_set: OverlapAdjudicationSet,
    pub final_join: DetectorReferenceJoin,
    pub final_contributions: JoinMetricContributionSet,
    pub final_aggregates: JoinMetricAggregateSet,
    pub final_bundle: ArtifactBundle,
    pub serialized_payloads: Vec<RealTranscriptEvaluationSerializedArtifact>,
    pub execution_trace: Vec<RealTranscriptEvaluationStageRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealTranscriptEvaluationCompletionStage {
    DetectorExecution,
    AssistedReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RealTranscriptEvaluationStage {
    RequestValidated,
    DetectorSnapshotValidated,
    JoinRequiresAdjudication,
    HumanAdjudicationValidated,
    JoinResolved,
    ContributionsPending,
    ContributionsComplete,
    AggregatesComplete,
    FinalBundleComplete,
    FinalBundleRederivationValidated,
    TypedPayloadReplayValidated,
    HistoricalReplayValidated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealTranscriptEvaluationStageRecord {
    pub stage: RealTranscriptEvaluationStage,
    pub lifecycle_state: RunLifecycleState,
    pub related_artifact_ids: Vec<ArtifactId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealTranscriptEvaluationSerializedArtifact {
    pub artifact_id: ArtifactId,
    pub role: ArtifactRole,
    pub payload_schema: ArtifactSchemaIdentity,
    pub payload_bytes: Vec<u8>,
    pub content_digest: ArtifactContentDigest,
    pub byte_length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RealTranscriptEvaluationExecutionError {
    RunnerContractValidationFailure(RealTranscriptEvaluationRunnerContractError),
    InvalidArtifactId,
    DuplicateArtifactId,
    ArtifactIdBindingMismatch {
        field: &'static str,
    },
    InvalidRevisionContext,
    RevisionArtifactBindingMismatch {
        field: &'static str,
    },
    DetectorSnapshotValidationFailure(DetectorProposalSnapshotValidationError),
    DetectorSnapshotAnalysisIdentityMismatch,
    DetectorSnapshotArtifactIdMismatch,
    DetectorExecutionAdjudicationValidationFailure(OverlapAdjudicationValidationError),
    DetectorExecutionAdjudicationNotEmpty,
    AdjudicationArtifactIdMismatch,
    AssistedReviewAdjudicationValidationFailure(OverlapAdjudicationValidationError),
    AssistedReviewAdjudicationForbidden,
    AssistedReviewAdjudicationEmpty,
    UnsupportedRealAdjudicatorRole {
        adjudication_id: crate::join_adjudication::OverlapAdjudicationId,
        role: OverlapAdjudicatorRole,
    },
    AssistedReviewAdjudicationIncomplete,
    JoinValidationFailure(DetectorReferenceJoinError),
    UnexpectedJoinState,
    RequiredOverlapPairSetEmpty,
    DuplicateRequiredOverlapPair,
    ContributionValidationFailure(JoinMetricContributionError),
    UnexpectedContributionState,
    PendingContributionEligibilityMismatch,
    AggregationValidationFailure(JoinMetricAggregationError),
    PendingAggregationUnexpectedSuccess,
    UnexpectedAggregateEligibility,
    BootstrapBundleValidationFailure(ArtifactBundleValidationError),
    FinalBundleValidationFailure(ArtifactBundleValidationError),
    BootstrapFinalDerivationMismatch {
        component: &'static str,
    },
    PayloadSerializationFailure,
    PayloadDeserializationFailure {
        role: ArtifactRole,
    },
    PayloadSchemaMismatch {
        role: ArtifactRole,
    },
    PayloadArtifactIdMismatch {
        role: ArtifactRole,
    },
    PayloadDigestMismatch {
        role: ArtifactRole,
    },
    PayloadLengthMismatch {
        role: ArtifactRole,
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
    TypedPayloadMismatch {
        role: ArtifactRole,
    },
    DecodedPayloadValidationFailure {
        role: ArtifactRole,
    },
    DecodedPayloadReserializationMismatch {
        role: ArtifactRole,
    },
    ValidatedPlanMismatch,
    HistoricalReplayValidationFailure {
        component: &'static str,
    },
    SourceMutationDetected {
        component: &'static str,
    },
    NonDeterministicReplay,
    IntegerConversionOverflow,
}

impl fmt::Display for RealTranscriptEvaluationExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for RealTranscriptEvaluationExecutionError {}

pub fn execute_real_transcript_evaluation(
    request: &RealTranscriptEvaluationRunRequest,
    input: &RealTranscriptEvaluationExecutionInput,
) -> Result<RealTranscriptEvaluationExecutionOutcome, RealTranscriptEvaluationExecutionError> {
    let source_snapshot = capture_source_snapshot(request, input);
    let mut trace = Vec::new();

    let validated_plan = validate_real_transcript_evaluation_run_request(request)
        .map_err(RealTranscriptEvaluationExecutionError::RunnerContractValidationFailure)?;
    push_trace(
        &mut trace,
        RealTranscriptEvaluationStage::RequestValidated,
        RunLifecycleState::Declared,
        artifact_ids_from_input(input),
    );

    validate_artifact_and_revision_identity(request, input)?;
    validate_detector_snapshot_authority(request, &validated_plan, input)?;
    push_trace(
        &mut trace,
        RealTranscriptEvaluationStage::DetectorSnapshotValidated,
        RunLifecycleState::DetectorExecution,
        artifact_ids_from_input(input),
    );

    validate_detector_execution_adjudication(request, input)?;

    let binding_context = build_binding_context(request, &validated_plan);

    let bootstrap_bundle = build_bootstrap_bundle(
        request,
        input,
        &binding_context,
        &input.detector_execution_adjudication_set,
        None,
        None,
        None,
    )?;

    let probe_join = DetectorReferenceJoin::derive(
        &input.revision_ids.join_context,
        &request.detector_execution_envelope,
        &request.reference_seal,
        &request.reference_coverage,
        &request.human_final_reference,
        &input.detector_snapshot,
        &bootstrap_bundle,
        &input.detector_execution_adjudication_set,
    )
    .map_err(RealTranscriptEvaluationExecutionError::JoinValidationFailure)?;

    match probe_join.state {
        DetectorReferenceJoinState::Resolved => {
            if input.assisted_review_adjudication_set.is_some() {
                return Err(
                    RealTranscriptEvaluationExecutionError::AssistedReviewAdjudicationForbidden,
                );
            }
            push_trace(
                &mut trace,
                RealTranscriptEvaluationStage::JoinResolved,
                RunLifecycleState::DetectorExecution,
                artifact_ids_from_input(input),
            );

            let completed = complete_execution(
                request,
                input,
                &validated_plan,
                &binding_context,
                RealTranscriptEvaluationCompletionStage::DetectorExecution,
                RunLifecycleState::DetectorExecution,
                &request.detector_execution_envelope,
                &input.detector_execution_adjudication_set,
                &mut trace,
            )?;
            verify_source_unchanged(request, input, &source_snapshot)?;
            Ok(RealTranscriptEvaluationExecutionOutcome::Completed(
                completed,
            ))
        }
        DetectorReferenceJoinState::RequiresAdjudication => {
            push_trace(
                &mut trace,
                RealTranscriptEvaluationStage::JoinRequiresAdjudication,
                RunLifecycleState::DetectorExecution,
                artifact_ids_from_input(input),
            );

            let pending_bootstrap = build_bootstrap_bundle(
                request,
                input,
                &binding_context,
                &input.detector_execution_adjudication_set,
                Some(&probe_join),
                None,
                None,
            )?;

            let pending_contributions = JoinMetricContributionSet::derive(
                &input.revision_ids.contribution_context,
                &request.detector_execution_envelope,
                &request.reference_seal,
                &request.reference_coverage,
                &request.human_final_reference,
                &input.detector_snapshot,
                &probe_join,
                &input.detector_execution_adjudication_set,
                &pending_bootstrap,
            )
            .map_err(RealTranscriptEvaluationExecutionError::ContributionValidationFailure)?;

            if pending_contributions.state != MetricContributionSetState::PendingJoinResolution {
                return Err(RealTranscriptEvaluationExecutionError::UnexpectedContributionState);
            }
            if pending_contributions.eligibility.primary_metrics_allowed {
                return Err(
                    RealTranscriptEvaluationExecutionError::PendingContributionEligibilityMismatch,
                );
            }

            push_trace(
                &mut trace,
                RealTranscriptEvaluationStage::ContributionsPending,
                RunLifecycleState::DetectorExecution,
                artifact_ids_from_input(input),
            );

            match JoinMetricAggregateSet::derive(
                &input.revision_ids.aggregate_context,
                &request.detector_execution_envelope,
                &request.reference_seal,
                &request.reference_coverage,
                &request.human_final_reference,
                &input.detector_snapshot,
                &probe_join,
                &input.detector_execution_adjudication_set,
                &pending_contributions,
                &build_bootstrap_bundle(
                    request,
                    input,
                    &binding_context,
                    &input.detector_execution_adjudication_set,
                    Some(&probe_join),
                    Some(&pending_contributions),
                    None,
                )?,
            ) {
                Err(JoinMetricAggregationError::PendingContributionRejected) => {}
                Err(JoinMetricAggregationError::ContributionSetNotComplete)
                    if pending_contributions.state
                        == MetricContributionSetState::PendingJoinResolution => {}
                Err(error) => {
                    return Err(
                        RealTranscriptEvaluationExecutionError::AggregationValidationFailure(error),
                    );
                }
                Ok(_) => {
                    return Err(
                        RealTranscriptEvaluationExecutionError::PendingAggregationUnexpectedSuccess,
                    );
                }
            }

            let overlap_pairs = extract_required_overlap_pairs(&probe_join)?;

            let Some(assisted_set) = &input.assisted_review_adjudication_set else {
                verify_source_unchanged(request, input, &source_snapshot)?;
                return Ok(
                    RealTranscriptEvaluationExecutionOutcome::RequiresHumanAdjudication(
                        RealTranscriptEvaluationPendingResult {
                            validated_plan: validated_plan.clone(),
                            pending_join: probe_join,
                            pending_contributions,
                            required_human_adjudication: RequiredHumanOverlapAdjudication {
                                run_id: validated_plan.run_id.clone(),
                                input_identity: validated_plan.input_identity.clone(),
                                reference_revision: validated_plan.reference_revision.clone(),
                                detector_snapshot_revision: input
                                    .detector_snapshot
                                    .snapshot_revision
                                    .clone(),
                                join_adjudication_artifact_id: input
                                    .artifact_ids
                                    .join_adjudication
                                    .clone(),
                                overlap_pairs,
                            },
                            execution_trace: trace,
                        },
                    ),
                );
            };

            validate_assisted_review_adjudication(request, input, assisted_set)?;
            push_trace(
                &mut trace,
                RealTranscriptEvaluationStage::HumanAdjudicationValidated,
                RunLifecycleState::AssistedReview,
                artifact_ids_from_input(input),
            );

            let assisted_probe = DetectorReferenceJoin::derive(
                &input.revision_ids.join_context,
                &request.assisted_review_transition_envelope,
                &request.reference_seal,
                &request.reference_coverage,
                &request.human_final_reference,
                &input.detector_snapshot,
                &build_bootstrap_bundle(
                    request,
                    input,
                    &binding_context,
                    assisted_set,
                    Some(&probe_join),
                    None,
                    None,
                )?,
                assisted_set,
            )
            .map_err(RealTranscriptEvaluationExecutionError::JoinValidationFailure)?;

            if assisted_probe.state != DetectorReferenceJoinState::Resolved
                || !assisted_probe.assessment.fully_resolved
                || !assisted_probe.assessment.one_to_one_consistent
                || assisted_probe.assessment.unresolved_overlap_edge_count != 0
            {
                return Err(
                    RealTranscriptEvaluationExecutionError::AssistedReviewAdjudicationIncomplete,
                );
            }

            push_trace(
                &mut trace,
                RealTranscriptEvaluationStage::JoinResolved,
                RunLifecycleState::AssistedReview,
                artifact_ids_from_input(input),
            );

            let completed = complete_execution(
                request,
                input,
                &validated_plan,
                &binding_context,
                RealTranscriptEvaluationCompletionStage::AssistedReview,
                RunLifecycleState::AssistedReview,
                &request.assisted_review_transition_envelope,
                assisted_set,
                &mut trace,
            )?;
            verify_source_unchanged(request, input, &source_snapshot)?;
            Ok(RealTranscriptEvaluationExecutionOutcome::Completed(
                completed,
            ))
        }
        _ => Err(RealTranscriptEvaluationExecutionError::UnexpectedJoinState),
    }
}

pub fn verify_real_transcript_evaluation_completed_result(
    result: &RealTranscriptEvaluationCompletedResult,
) -> Result<(), RealTranscriptEvaluationExecutionError> {
    let recomputed_plan = validate_real_transcript_evaluation_run_request(&result.request)
        .map_err(RealTranscriptEvaluationExecutionError::RunnerContractValidationFailure)?;
    if recomputed_plan != result.validated_plan {
        return Err(RealTranscriptEvaluationExecutionError::ValidatedPlanMismatch);
    }

    verify_payload_integrity(result)?;
    verify_typed_payload_round_trip(result)?;
    verify_final_bundle_rederivation(result)?;
    Ok(())
}

fn verify_final_bundle_rederivation(
    result: &RealTranscriptEvaluationCompletedResult,
) -> Result<(), RealTranscriptEvaluationExecutionError> {
    let envelope = match result.completion_stage {
        RealTranscriptEvaluationCompletionStage::DetectorExecution => {
            &result.request.detector_execution_envelope
        }
        RealTranscriptEvaluationCompletionStage::AssistedReview => {
            &result.request.assisted_review_transition_envelope
        }
    };

    let join_context = DetectorReferenceJoinContext {
        join_id: result.final_join.join_id.clone(),
        join_revision: result.final_join.join_revision.clone(),
        evaluation_join_artifact_id: result.final_join.evaluation_join_artifact_id.clone(),
        join_adjudication_artifact_id: result.final_join.join_adjudication_artifact_id.clone(),
    };
    let contribution_context = JoinMetricContributionContext {
        contribution_set_id: result.final_contributions.contribution_set_id.clone(),
        contribution_revision: result.final_contributions.contribution_revision.clone(),
        metric_contributions_artifact_id: result
            .final_contributions
            .metric_contributions_artifact_id
            .clone(),
    };
    let aggregate_context = JoinMetricAggregateContext {
        aggregate_set_id: result.final_aggregates.aggregate_set_id.clone(),
        aggregate_revision: result.final_aggregates.aggregate_revision.clone(),
        metrics_artifact_id: result.final_aggregates.metrics_artifact_id.clone(),
    };

    let verified_join = DetectorReferenceJoin::derive(
        &join_context,
        envelope,
        &result.request.reference_seal,
        &result.request.reference_coverage,
        &result.request.human_final_reference,
        &result.detector_snapshot,
        &result.final_bundle,
        &result.final_adjudication_set,
    )
    .map_err(RealTranscriptEvaluationExecutionError::JoinValidationFailure)?;
    if verified_join != result.final_join {
        return Err(
            RealTranscriptEvaluationExecutionError::BootstrapFinalDerivationMismatch {
                component: "join_verifier",
            },
        );
    }

    let verified_contributions = JoinMetricContributionSet::derive(
        &contribution_context,
        envelope,
        &result.request.reference_seal,
        &result.request.reference_coverage,
        &result.request.human_final_reference,
        &result.detector_snapshot,
        &verified_join,
        &result.final_adjudication_set,
        &result.final_bundle,
    )
    .map_err(RealTranscriptEvaluationExecutionError::ContributionValidationFailure)?;
    if verified_contributions != result.final_contributions {
        return Err(
            RealTranscriptEvaluationExecutionError::BootstrapFinalDerivationMismatch {
                component: "contributions_verifier",
            },
        );
    }

    let verified_aggregates = JoinMetricAggregateSet::derive(
        &aggregate_context,
        envelope,
        &result.request.reference_seal,
        &result.request.reference_coverage,
        &result.request.human_final_reference,
        &result.detector_snapshot,
        &verified_join,
        &result.final_adjudication_set,
        &verified_contributions,
        &result.final_bundle,
    )
    .map_err(RealTranscriptEvaluationExecutionError::AggregationValidationFailure)?;
    if verified_aggregates != result.final_aggregates {
        return Err(
            RealTranscriptEvaluationExecutionError::BootstrapFinalDerivationMismatch {
                component: "aggregates_verifier",
            },
        );
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceSnapshot {
    request: RealTranscriptEvaluationRunRequest,
    detector_snapshot: DetectorProposalSnapshot,
    detector_execution_adjudication_set: OverlapAdjudicationSet,
    assisted_review_adjudication_set: Option<OverlapAdjudicationSet>,
    artifact_ids: RealTranscriptEvaluationArtifactIds,
    revision_ids: RealTranscriptEvaluationRevisionIds,
}

fn capture_source_snapshot(
    request: &RealTranscriptEvaluationRunRequest,
    input: &RealTranscriptEvaluationExecutionInput,
) -> SourceSnapshot {
    SourceSnapshot {
        request: request.clone(),
        detector_snapshot: input.detector_snapshot.clone(),
        detector_execution_adjudication_set: input.detector_execution_adjudication_set.clone(),
        assisted_review_adjudication_set: input.assisted_review_adjudication_set.clone(),
        artifact_ids: input.artifact_ids.clone(),
        revision_ids: input.revision_ids.clone(),
    }
}

fn verify_source_unchanged(
    request: &RealTranscriptEvaluationRunRequest,
    input: &RealTranscriptEvaluationExecutionInput,
    snapshot: &SourceSnapshot,
) -> Result<(), RealTranscriptEvaluationExecutionError> {
    if *request != snapshot.request {
        return Err(
            RealTranscriptEvaluationExecutionError::SourceMutationDetected {
                component: "request",
            },
        );
    }
    if input.detector_snapshot != snapshot.detector_snapshot {
        return Err(
            RealTranscriptEvaluationExecutionError::SourceMutationDetected {
                component: "detector_snapshot",
            },
        );
    }
    if input.detector_execution_adjudication_set != snapshot.detector_execution_adjudication_set {
        return Err(
            RealTranscriptEvaluationExecutionError::SourceMutationDetected {
                component: "detector_execution_adjudication_set",
            },
        );
    }
    if input.assisted_review_adjudication_set != snapshot.assisted_review_adjudication_set {
        return Err(
            RealTranscriptEvaluationExecutionError::SourceMutationDetected {
                component: "assisted_review_adjudication_set",
            },
        );
    }
    if input.artifact_ids != snapshot.artifact_ids {
        return Err(
            RealTranscriptEvaluationExecutionError::SourceMutationDetected {
                component: "artifact_ids",
            },
        );
    }
    if input.revision_ids != snapshot.revision_ids {
        return Err(
            RealTranscriptEvaluationExecutionError::SourceMutationDetected {
                component: "revision_ids",
            },
        );
    }
    Ok(())
}

fn validate_artifact_and_revision_identity(
    request: &RealTranscriptEvaluationRunRequest,
    input: &RealTranscriptEvaluationExecutionInput,
) -> Result<(), RealTranscriptEvaluationExecutionError> {
    let ids = [
        (
            "input_authorization",
            input.artifact_ids.input_authorization.as_str(),
        ),
        ("reference_seal", input.artifact_ids.reference_seal.as_str()),
        (
            "human_final_reference",
            input.artifact_ids.human_final_reference.as_str(),
        ),
        (
            "cue_review_completion",
            input.artifact_ids.cue_review_completion.as_str(),
        ),
        (
            "detector_output",
            input.artifact_ids.detector_output.as_str(),
        ),
        (
            "evaluation_join",
            input.artifact_ids.evaluation_join.as_str(),
        ),
        (
            "join_adjudication",
            input.artifact_ids.join_adjudication.as_str(),
        ),
        (
            "metric_contributions",
            input.artifact_ids.metric_contributions.as_str(),
        ),
        ("metrics", input.artifact_ids.metrics.as_str()),
    ];
    for (_field, value) in ids {
        if value.is_empty() {
            return Err(RealTranscriptEvaluationExecutionError::InvalidArtifactId);
        }
    }
    if input.artifact_ids.bundle.as_str().is_empty() {
        return Err(RealTranscriptEvaluationExecutionError::InvalidArtifactId);
    }

    let mut seen = std::collections::HashSet::new();
    for (_field, value) in ids {
        if !seen.insert(value) {
            return Err(RealTranscriptEvaluationExecutionError::DuplicateArtifactId);
        }
    }

    if input.artifact_ids.detector_output != input.detector_snapshot.detector_output_artifact_id {
        return Err(RealTranscriptEvaluationExecutionError::DetectorSnapshotArtifactIdMismatch);
    }

    if input.revision_ids.join_context.evaluation_join_artifact_id
        != input.artifact_ids.evaluation_join
    {
        return Err(
            RealTranscriptEvaluationExecutionError::RevisionArtifactBindingMismatch {
                field: "join_context.evaluation_join_artifact_id",
            },
        );
    }
    if input
        .revision_ids
        .join_context
        .join_adjudication_artifact_id
        != input.artifact_ids.join_adjudication
    {
        return Err(
            RealTranscriptEvaluationExecutionError::RevisionArtifactBindingMismatch {
                field: "join_context.join_adjudication_artifact_id",
            },
        );
    }
    if input
        .revision_ids
        .contribution_context
        .metric_contributions_artifact_id
        != input.artifact_ids.metric_contributions
    {
        return Err(
            RealTranscriptEvaluationExecutionError::RevisionArtifactBindingMismatch {
                field: "contribution_context.metric_contributions_artifact_id",
            },
        );
    }
    if input.revision_ids.aggregate_context.metrics_artifact_id != input.artifact_ids.metrics {
        return Err(
            RealTranscriptEvaluationExecutionError::RevisionArtifactBindingMismatch {
                field: "aggregate_context.metrics_artifact_id",
            },
        );
    }

    if request.expected_artifact_roles != FINAL_ARTIFACT_ROLES.as_slice() {
        return Err(RealTranscriptEvaluationExecutionError::InvalidRevisionContext);
    }

    Ok(())
}

fn validate_detector_snapshot_authority(
    request: &RealTranscriptEvaluationRunRequest,
    plan: &ValidatedRealTranscriptEvaluationRunPlan,
    input: &RealTranscriptEvaluationExecutionInput,
) -> Result<(), RealTranscriptEvaluationExecutionError> {
    let snapshot = &input.detector_snapshot;
    if snapshot.state != crate::detector_snapshot::DetectorProposalSnapshotState::Frozen
        || snapshot.frozen_at_unix_ms == 0
    {
        return Err(
            RealTranscriptEvaluationExecutionError::DetectorSnapshotValidationFailure(
                if snapshot.frozen_at_unix_ms == 0 {
                    DetectorProposalSnapshotValidationError::ZeroFrozenTimestamp
                } else {
                    DetectorProposalSnapshotValidationError::SnapshotStateMismatch {
                        state: snapshot.state,
                        assessment: Box::new(snapshot.assessment.clone()),
                    }
                },
            ),
        );
    }

    snapshot
        .validate_for_freeze_against(&request.detector_execution_envelope)
        .map_err(RealTranscriptEvaluationExecutionError::DetectorSnapshotValidationFailure)?;

    if snapshot.analysis_identity != plan.detector_analysis_identity {
        return Err(
            RealTranscriptEvaluationExecutionError::DetectorSnapshotAnalysisIdentityMismatch,
        );
    }

    Ok(())
}

fn validate_detector_execution_adjudication(
    request: &RealTranscriptEvaluationRunRequest,
    input: &RealTranscriptEvaluationExecutionInput,
) -> Result<(), RealTranscriptEvaluationExecutionError> {
    let set = &input.detector_execution_adjudication_set;
    if set.state != crate::join_adjudication::OverlapAdjudicationSetState::Frozen {
        return Err(
            RealTranscriptEvaluationExecutionError::DetectorExecutionAdjudicationValidationFailure(
                OverlapAdjudicationValidationError::AdjudicationSetNotFrozen,
            ),
        );
    }
    if !set.records.is_empty() {
        return Err(RealTranscriptEvaluationExecutionError::DetectorExecutionAdjudicationNotEmpty);
    }

    set.validate_frozen_for_join(
        &request.detector_execution_envelope,
        &request.reference_seal.reference_revision,
        &input.detector_snapshot.snapshot_revision,
    )
    .map_err(
        RealTranscriptEvaluationExecutionError::DetectorExecutionAdjudicationValidationFailure,
    )?;

    if set.join_adjudication_artifact_id != input.artifact_ids.join_adjudication {
        return Err(RealTranscriptEvaluationExecutionError::AdjudicationArtifactIdMismatch);
    }

    Ok(())
}

fn validate_assisted_review_adjudication(
    request: &RealTranscriptEvaluationRunRequest,
    input: &RealTranscriptEvaluationExecutionInput,
    set: &OverlapAdjudicationSet,
) -> Result<(), RealTranscriptEvaluationExecutionError> {
    if set.records.is_empty() {
        return Err(RealTranscriptEvaluationExecutionError::AssistedReviewAdjudicationEmpty);
    }

    set.validate_frozen_for_join(
        &request.assisted_review_transition_envelope,
        &request.reference_seal.reference_revision,
        &input.detector_snapshot.snapshot_revision,
    )
    .map_err(RealTranscriptEvaluationExecutionError::AssistedReviewAdjudicationValidationFailure)?;

    if set.join_adjudication_artifact_id != input.artifact_ids.join_adjudication {
        return Err(RealTranscriptEvaluationExecutionError::AdjudicationArtifactIdMismatch);
    }

    for record in &set.records {
        match record.adjudicator_role {
            OverlapAdjudicatorRole::OwnerAdjudicator
            | OverlapAdjudicatorRole::AuthorizedDomainAdjudicator => {}
            role => {
                return Err(
                    RealTranscriptEvaluationExecutionError::UnsupportedRealAdjudicatorRole {
                        adjudication_id: record.adjudication_id.clone(),
                        role,
                    },
                );
            }
        }
    }

    Ok(())
}

fn extract_required_overlap_pairs(
    join: &DetectorReferenceJoin,
) -> Result<Vec<Phase3OverlapPair>, RealTranscriptEvaluationExecutionError> {
    let mut pairs = join
        .edges
        .iter()
        .filter(|edge| {
            edge.anchor_relation == JoinAnchorRelation::Overlap
                && edge.resolution == JoinEdgeResolution::OverlapCandidate
        })
        .map(|edge| Phase3OverlapPair {
            detector_proposal_id: edge.detector_proposal_id.clone(),
            reference_error_id: edge.reference_error_id.clone(),
        })
        .collect::<Vec<_>>();

    pairs.sort_by(|left, right| {
        left.detector_proposal_id
            .as_str()
            .cmp(right.detector_proposal_id.as_str())
            .then_with(|| {
                left.reference_error_id
                    .as_str()
                    .cmp(right.reference_error_id.as_str())
            })
    });
    if join.state == DetectorReferenceJoinState::RequiresAdjudication && pairs.is_empty() {
        return Err(RealTranscriptEvaluationExecutionError::RequiredOverlapPairSetEmpty);
    }

    let mut seen = std::collections::HashSet::new();
    for pair in &pairs {
        if !seen.insert((
            pair.detector_proposal_id.clone(),
            pair.reference_error_id.clone(),
        )) {
            return Err(RealTranscriptEvaluationExecutionError::DuplicateRequiredOverlapPair);
        }
    }

    Ok(pairs)
}

fn build_binding_context(
    request: &RealTranscriptEvaluationRunRequest,
    plan: &ValidatedRealTranscriptEvaluationRunPlan,
) -> ArtifactBindingContext {
    ArtifactBindingContext {
        run_id: plan.run_id.clone(),
        input_identity: plan.input_identity.clone(),
        calibration_validity: CalibrationValidityMode::BlindReference,
        reference_seal_id: Some(request.reference_seal.seal_id.clone()),
        reference_coverage_id: Some(request.reference_coverage.coverage_id.clone()),
        reference_revision: Some(request.reference_seal.reference_revision.clone()),
    }
}

#[allow(clippy::type_complexity)]
fn complete_execution(
    request: &RealTranscriptEvaluationRunRequest,
    input: &RealTranscriptEvaluationExecutionInput,
    validated_plan: &ValidatedRealTranscriptEvaluationRunPlan,
    binding_context: &ArtifactBindingContext,
    completion_stage: RealTranscriptEvaluationCompletionStage,
    creation_lifecycle: RunLifecycleState,
    creation_envelope: &RunEnvelope,
    final_adjudication: &OverlapAdjudicationSet,
    trace: &mut Vec<RealTranscriptEvaluationStageRecord>,
) -> Result<RealTranscriptEvaluationCompletedResult, RealTranscriptEvaluationExecutionError> {
    let (pass_a_join, pass_a_contributions, pass_a_aggregates) = derive_complete_chain(
        request,
        input,
        creation_envelope,
        final_adjudication,
        binding_context,
        creation_lifecycle,
        trace,
    )?;

    let (serialized_payloads, final_bundle) = finalize_bundle_two_pass(
        request,
        input,
        creation_envelope,
        creation_lifecycle,
        binding_context,
        final_adjudication,
        &pass_a_join,
        &pass_a_contributions,
        &pass_a_aggregates,
        trace,
    )?;

    final_bundle
        .validate_against_envelope(creation_envelope)
        .map_err(RealTranscriptEvaluationExecutionError::FinalBundleValidationFailure)?;
    final_bundle
        .validate_with_reference_context(
            creation_envelope,
            Some(&request.reference_seal),
            Some(&request.reference_coverage),
            Some(&request.human_final_reference),
        )
        .map_err(RealTranscriptEvaluationExecutionError::FinalBundleValidationFailure)?;

    let mut result = RealTranscriptEvaluationCompletedResult {
        request: request.clone(),
        validated_plan: validated_plan.clone(),
        completion_stage,
        detector_snapshot: input.detector_snapshot.clone(),
        final_adjudication_set: final_adjudication.clone(),
        final_join: pass_a_join,
        final_contributions: pass_a_contributions,
        final_aggregates: pass_a_aggregates,
        final_bundle,
        serialized_payloads,
        execution_trace: trace.clone(),
    };

    verify_real_transcript_evaluation_completed_result(&result)?;

    push_trace(
        &mut result.execution_trace,
        RealTranscriptEvaluationStage::HistoricalReplayValidated,
        RunLifecycleState::Finalized,
        artifact_ids_from_input(input),
    );

    Ok(result)
}

#[allow(clippy::type_complexity)]
fn derive_complete_chain(
    request: &RealTranscriptEvaluationRunRequest,
    input: &RealTranscriptEvaluationExecutionInput,
    envelope: &RunEnvelope,
    adjudication: &OverlapAdjudicationSet,
    binding_context: &ArtifactBindingContext,
    lifecycle: RunLifecycleState,
    trace: &mut Vec<RealTranscriptEvaluationStageRecord>,
) -> Result<
    (
        DetectorReferenceJoin,
        JoinMetricContributionSet,
        JoinMetricAggregateSet,
    ),
    RealTranscriptEvaluationExecutionError,
> {
    let bootstrap_bundle = build_bootstrap_bundle(
        request,
        input,
        binding_context,
        adjudication,
        None,
        None,
        None,
    )?;

    let join = DetectorReferenceJoin::derive(
        &input.revision_ids.join_context,
        envelope,
        &request.reference_seal,
        &request.reference_coverage,
        &request.human_final_reference,
        &input.detector_snapshot,
        &bootstrap_bundle,
        adjudication,
    )
    .map_err(RealTranscriptEvaluationExecutionError::JoinValidationFailure)?;

    if join.state != DetectorReferenceJoinState::Resolved
        || !join.assessment.fully_resolved
        || !join.assessment.one_to_one_consistent
        || join.assessment.unresolved_overlap_edge_count != 0
    {
        return Err(RealTranscriptEvaluationExecutionError::UnexpectedJoinState);
    }

    let bootstrap_bundle = build_bootstrap_bundle(
        request,
        input,
        binding_context,
        adjudication,
        Some(&join),
        None,
        None,
    )?;

    let contributions = JoinMetricContributionSet::derive(
        &input.revision_ids.contribution_context,
        envelope,
        &request.reference_seal,
        &request.reference_coverage,
        &request.human_final_reference,
        &input.detector_snapshot,
        &join,
        adjudication,
        &bootstrap_bundle,
    )
    .map_err(RealTranscriptEvaluationExecutionError::ContributionValidationFailure)?;

    if contributions.state != MetricContributionSetState::Complete
        || !contributions.assessment.mapping_complete
        || contributions.assessment.pending_detector_contribution_count != 0
        || contributions
            .assessment
            .pending_reference_contribution_count
            != 0
    {
        return Err(RealTranscriptEvaluationExecutionError::UnexpectedContributionState);
    }
    if contributions.eligibility.report_class
        != MetricContributionReportClass::PrimaryBlindCalibration
        || !contributions.eligibility.primary_metrics_allowed
        || !contributions.eligibility.blocking_reasons.is_empty()
        || !contributions
            .eligibility
            .qualifies_as_real_material_evidence
    {
        return Err(RealTranscriptEvaluationExecutionError::PendingContributionEligibilityMismatch);
    }

    push_trace(
        trace,
        RealTranscriptEvaluationStage::ContributionsComplete,
        lifecycle,
        artifact_ids_from_input(input),
    );

    let bootstrap_bundle = build_bootstrap_bundle(
        request,
        input,
        binding_context,
        adjudication,
        Some(&join),
        Some(&contributions),
        None,
    )?;

    let aggregates = JoinMetricAggregateSet::derive(
        &input.revision_ids.aggregate_context,
        envelope,
        &request.reference_seal,
        &request.reference_coverage,
        &request.human_final_reference,
        &input.detector_snapshot,
        &join,
        adjudication,
        &contributions,
        &bootstrap_bundle,
    )
    .map_err(RealTranscriptEvaluationExecutionError::AggregationValidationFailure)?;

    if aggregates.state != crate::join_metric_aggregation::MetricAggregateSetState::Complete
        || aggregates.report_class != MetricContributionReportClass::PrimaryBlindCalibration
        || !aggregates.primary_metrics_allowed
        || !aggregates.blocking_reasons.is_empty()
        || !aggregates.qualifies_as_real_material_evidence
        || !aggregates.qualifies_as_primary_metric_evidence
        || !aggregates.assessment.all_required_metrics_present
        || aggregates.metrics.len() != 5
    {
        return Err(RealTranscriptEvaluationExecutionError::UnexpectedAggregateEligibility);
    }

    push_trace(
        trace,
        RealTranscriptEvaluationStage::AggregatesComplete,
        lifecycle,
        artifact_ids_from_input(input),
    );

    Ok((join, contributions, aggregates))
}

#[allow(clippy::type_complexity)]
fn finalize_bundle_two_pass(
    request: &RealTranscriptEvaluationRunRequest,
    input: &RealTranscriptEvaluationExecutionInput,
    envelope: &RunEnvelope,
    lifecycle: RunLifecycleState,
    binding_context: &ArtifactBindingContext,
    adjudication: &OverlapAdjudicationSet,
    pass_a_join: &DetectorReferenceJoin,
    pass_a_contributions: &JoinMetricContributionSet,
    pass_a_aggregates: &JoinMetricAggregateSet,
    trace: &mut Vec<RealTranscriptEvaluationStageRecord>,
) -> Result<
    (
        Vec<RealTranscriptEvaluationSerializedArtifact>,
        ArtifactBundle,
    ),
    RealTranscriptEvaluationExecutionError,
> {
    let pass_a_payloads = serialize_payload_set(
        request,
        input,
        adjudication,
        pass_a_join,
        pass_a_contributions,
        pass_a_aggregates,
    )?;
    let pass_a_bundle = build_bundle_from_payloads(
        request,
        input,
        binding_context,
        &pass_a_payloads,
        ArtifactBundleState::Complete,
    )?;

    let pass_b_join = DetectorReferenceJoin::derive(
        &input.revision_ids.join_context,
        envelope,
        &request.reference_seal,
        &request.reference_coverage,
        &request.human_final_reference,
        &input.detector_snapshot,
        &pass_a_bundle,
        adjudication,
    )
    .map_err(RealTranscriptEvaluationExecutionError::JoinValidationFailure)?;
    if pass_b_join != *pass_a_join {
        return Err(
            RealTranscriptEvaluationExecutionError::BootstrapFinalDerivationMismatch {
                component: "join",
            },
        );
    }

    let pass_b_contributions = JoinMetricContributionSet::derive(
        &input.revision_ids.contribution_context,
        envelope,
        &request.reference_seal,
        &request.reference_coverage,
        &request.human_final_reference,
        &input.detector_snapshot,
        &pass_b_join,
        adjudication,
        &pass_a_bundle,
    )
    .map_err(RealTranscriptEvaluationExecutionError::ContributionValidationFailure)?;
    if pass_b_contributions != *pass_a_contributions {
        return Err(
            RealTranscriptEvaluationExecutionError::BootstrapFinalDerivationMismatch {
                component: "contributions",
            },
        );
    }

    let pass_b_aggregates = JoinMetricAggregateSet::derive(
        &input.revision_ids.aggregate_context,
        envelope,
        &request.reference_seal,
        &request.reference_coverage,
        &request.human_final_reference,
        &input.detector_snapshot,
        &pass_b_join,
        adjudication,
        &pass_b_contributions,
        &pass_a_bundle,
    )
    .map_err(RealTranscriptEvaluationExecutionError::AggregationValidationFailure)?;
    if pass_b_aggregates != *pass_a_aggregates {
        return Err(
            RealTranscriptEvaluationExecutionError::BootstrapFinalDerivationMismatch {
                component: "aggregates",
            },
        );
    }

    let final_payloads = serialize_payload_set(
        request,
        input,
        adjudication,
        &pass_b_join,
        &pass_b_contributions,
        &pass_b_aggregates,
    )?;
    let final_bundle = build_bundle_from_payloads(
        request,
        input,
        binding_context,
        &final_payloads,
        ArtifactBundleState::Complete,
    )?;

    let pass_c_join = DetectorReferenceJoin::derive(
        &input.revision_ids.join_context,
        envelope,
        &request.reference_seal,
        &request.reference_coverage,
        &request.human_final_reference,
        &input.detector_snapshot,
        &final_bundle,
        adjudication,
    )
    .map_err(RealTranscriptEvaluationExecutionError::JoinValidationFailure)?;
    if pass_c_join != pass_b_join {
        return Err(
            RealTranscriptEvaluationExecutionError::BootstrapFinalDerivationMismatch {
                component: "join_pass_b",
            },
        );
    }

    let pass_c_contributions = JoinMetricContributionSet::derive(
        &input.revision_ids.contribution_context,
        envelope,
        &request.reference_seal,
        &request.reference_coverage,
        &request.human_final_reference,
        &input.detector_snapshot,
        &pass_c_join,
        adjudication,
        &final_bundle,
    )
    .map_err(RealTranscriptEvaluationExecutionError::ContributionValidationFailure)?;
    if pass_c_contributions != pass_b_contributions {
        return Err(
            RealTranscriptEvaluationExecutionError::BootstrapFinalDerivationMismatch {
                component: "contributions_pass_b",
            },
        );
    }

    let pass_c_aggregates = JoinMetricAggregateSet::derive(
        &input.revision_ids.aggregate_context,
        envelope,
        &request.reference_seal,
        &request.reference_coverage,
        &request.human_final_reference,
        &input.detector_snapshot,
        &pass_c_join,
        adjudication,
        &pass_c_contributions,
        &final_bundle,
    )
    .map_err(RealTranscriptEvaluationExecutionError::AggregationValidationFailure)?;
    if pass_c_aggregates != pass_b_aggregates {
        return Err(
            RealTranscriptEvaluationExecutionError::BootstrapFinalDerivationMismatch {
                component: "aggregates_pass_b",
            },
        );
    }

    let reserialized = serialize_payload_set(
        request,
        input,
        adjudication,
        &pass_c_join,
        &pass_c_contributions,
        &pass_c_aggregates,
    )?;
    if reserialized != final_payloads {
        return Err(
            RealTranscriptEvaluationExecutionError::BootstrapFinalDerivationMismatch {
                component: "payload_reserialization",
            },
        );
    }

    push_trace(
        trace,
        RealTranscriptEvaluationStage::FinalBundleComplete,
        lifecycle,
        artifact_ids_from_input(input),
    );
    push_trace(
        trace,
        RealTranscriptEvaluationStage::FinalBundleRederivationValidated,
        lifecycle,
        artifact_ids_from_input(input),
    );
    push_trace(
        trace,
        RealTranscriptEvaluationStage::TypedPayloadReplayValidated,
        RunLifecycleState::Finalized,
        artifact_ids_from_input(input),
    );

    Ok((final_payloads, final_bundle))
}

fn build_bootstrap_bundle(
    request: &RealTranscriptEvaluationRunRequest,
    input: &RealTranscriptEvaluationExecutionInput,
    binding_context: &ArtifactBindingContext,
    adjudication: &OverlapAdjudicationSet,
    join: Option<&DetectorReferenceJoin>,
    contributions: Option<&JoinMetricContributionSet>,
    aggregates: Option<&JoinMetricAggregateSet>,
) -> Result<ArtifactBundle, RealTranscriptEvaluationExecutionError> {
    let join = join
        .cloned()
        .unwrap_or_else(|| bootstrap_stub_join(request, input));
    let contributions = contributions
        .cloned()
        .unwrap_or_else(|| bootstrap_stub_contributions(request, input));
    let aggregates = aggregates
        .cloned()
        .unwrap_or_else(|| bootstrap_stub_aggregates(request, input));

    let payloads = serialize_payload_set(
        request,
        input,
        adjudication,
        &join,
        &contributions,
        &aggregates,
    )?;
    build_bundle_from_payloads(
        request,
        input,
        binding_context,
        &payloads,
        ArtifactBundleState::Complete,
    )
}

fn bootstrap_stub_join(
    request: &RealTranscriptEvaluationRunRequest,
    input: &RealTranscriptEvaluationExecutionInput,
) -> DetectorReferenceJoin {
    DetectorReferenceJoin {
        schema_revision: DETECTOR_REFERENCE_JOIN_SCHEMA.to_string(),
        join_id: input.revision_ids.join_context.join_id.clone(),
        join_revision: input.revision_ids.join_context.join_revision.clone(),
        run_id: request.declared_envelope.run_id.clone(),
        input_identity: request.declared_envelope.input_identity.clone(),
        calibration_validity: CalibrationValidityMode::BlindReference,
        reference_seal_id: request.reference_seal.seal_id.clone(),
        reference_revision: request.reference_seal.reference_revision.clone(),
        reference_coverage_id: request.reference_coverage.coverage_id.clone(),
        detector_snapshot_revision: input.detector_snapshot.snapshot_revision.clone(),
        detector_output_artifact_id: input.artifact_ids.detector_output.clone(),
        evaluation_join_artifact_id: input.artifact_ids.evaluation_join.clone(),
        join_adjudication_artifact_id: input.artifact_ids.join_adjudication.clone(),
        overlap_rule_revision: "voxproof-overlap-v1".to_string(),
        correction_equality_revision: "unicode-nfc-equality-v1".to_string(),
        alternative_cardinality_policy: "exactly-one-alternative-v1".to_string(),
        join_purpose: DetectorReferenceJoinPurpose::PrimaryBlindCalibration,
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

fn bootstrap_stub_contributions(
    request: &RealTranscriptEvaluationRunRequest,
    input: &RealTranscriptEvaluationExecutionInput,
) -> JoinMetricContributionSet {
    JoinMetricContributionSet {
        schema_revision: JOIN_METRIC_CONTRIBUTION_SCHEMA.to_string(),
        contribution_set_id: input
            .revision_ids
            .contribution_context
            .contribution_set_id
            .clone(),
        contribution_revision: input
            .revision_ids
            .contribution_context
            .contribution_revision
            .clone(),
        run_id: request.declared_envelope.run_id.clone(),
        input_identity: request.declared_envelope.input_identity.clone(),
        input_class: request.declared_envelope.input_class,
        qualifies_as_real_material_evidence: request
            .declared_envelope
            .qualifies_as_real_material_evidence,
        reference_seal_id: request.reference_seal.seal_id.clone(),
        reference_revision: request.reference_seal.reference_revision.clone(),
        reference_coverage_id: request.reference_coverage.coverage_id.clone(),
        detector_snapshot_revision: input.detector_snapshot.snapshot_revision.clone(),
        detector_output_artifact_id: input.artifact_ids.detector_output.clone(),
        join_id: input.revision_ids.join_context.join_id.clone(),
        join_revision: input.revision_ids.join_context.join_revision.clone(),
        evaluation_join_artifact_id: input.artifact_ids.evaluation_join.clone(),
        join_adjudication_artifact_id: input.artifact_ids.join_adjudication.clone(),
        metric_contributions_artifact_id: input.artifact_ids.metric_contributions.clone(),
        eligibility_policy_revision:
            crate::join_metric_contribution::PRIMARY_METRIC_ELIGIBILITY_POLICY.to_string(),
        contribution_policy_revision: crate::join_metric_contribution::METRIC_CONTRIBUTION_POLICY
            .to_string(),
        state: MetricContributionSetState::PendingJoinResolution,
        eligibility: crate::join_metric_contribution::PrimaryMetricEligibilityAssessment {
            policy_revision: crate::join_metric_contribution::PRIMARY_METRIC_ELIGIBILITY_POLICY
                .to_string(),
            report_class: MetricContributionReportClass::PrimaryBlindCalibration,
            primary_metrics_allowed: false,
            eligible_primary_metrics: Vec::new(),
            blocking_reasons: Vec::new(),
            qualifies_as_real_material_evidence: request
                .declared_envelope
                .qualifies_as_real_material_evidence,
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

fn bootstrap_stub_aggregates(
    request: &RealTranscriptEvaluationRunRequest,
    input: &RealTranscriptEvaluationExecutionInput,
) -> JoinMetricAggregateSet {
    JoinMetricAggregateSet {
        schema_revision: JOIN_METRIC_AGGREGATION_SCHEMA.to_string(),
        aggregate_set_id: input
            .revision_ids
            .aggregate_context
            .aggregate_set_id
            .clone(),
        aggregate_revision: input
            .revision_ids
            .aggregate_context
            .aggregate_revision
            .clone(),
        run_id: request.declared_envelope.run_id.clone(),
        input_identity: request.declared_envelope.input_identity.clone(),
        input_class: request.declared_envelope.input_class,
        qualifies_as_real_material_evidence: request
            .declared_envelope
            .qualifies_as_real_material_evidence,
        reference_seal_id: request.reference_seal.seal_id.clone(),
        reference_revision: request.reference_seal.reference_revision.clone(),
        reference_coverage_id: request.reference_coverage.coverage_id.clone(),
        detector_snapshot_revision: input.detector_snapshot.snapshot_revision.clone(),
        detector_output_artifact_id: input.artifact_ids.detector_output.clone(),
        join_id: input.revision_ids.join_context.join_id.clone(),
        join_revision: input.revision_ids.join_context.join_revision.clone(),
        evaluation_join_artifact_id: input.artifact_ids.evaluation_join.clone(),
        join_adjudication_artifact_id: input.artifact_ids.join_adjudication.clone(),
        contribution_set_id: input
            .revision_ids
            .contribution_context
            .contribution_set_id
            .clone(),
        contribution_revision: input
            .revision_ids
            .contribution_context
            .contribution_revision
            .clone(),
        metric_contributions_artifact_id: input.artifact_ids.metric_contributions.clone(),
        metrics_artifact_id: input.artifact_ids.metrics.clone(),
        aggregation_policy_revision:
            crate::join_metric_aggregation::PRIMARY_METRIC_AGGREGATION_POLICY.to_string(),
        zero_denominator_policy_revision: crate::join_metric_aggregation::ZERO_DENOMINATOR_POLICY
            .to_string(),
        report_class: MetricContributionReportClass::PrimaryBlindCalibration,
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
    request: &RealTranscriptEvaluationRunRequest,
    input: &RealTranscriptEvaluationExecutionInput,
    adjudication: &OverlapAdjudicationSet,
    join: &DetectorReferenceJoin,
    contributions: &JoinMetricContributionSet,
    aggregates: &JoinMetricAggregateSet,
) -> Result<Vec<RealTranscriptEvaluationSerializedArtifact>, RealTranscriptEvaluationExecutionError>
{
    Ok(vec![
        serialize_role_payload(
            input,
            ArtifactRole::InputAuthorization,
            &request.input_authorization,
        )?,
        serialize_role_payload(input, ArtifactRole::ReferenceSeal, &request.reference_seal)?,
        serialize_role_payload(
            input,
            ArtifactRole::HumanFinalReference,
            &request.human_final_reference,
        )?,
        serialize_role_payload(
            input,
            ArtifactRole::CueReviewCompletion,
            &request.reference_coverage,
        )?,
        serialize_role_payload(
            input,
            ArtifactRole::DetectorOutput,
            &input.detector_snapshot,
        )?,
        serialize_role_payload(input, ArtifactRole::EvaluationJoin, join)?,
        serialize_role_payload(input, ArtifactRole::JoinAdjudication, adjudication)?,
        serialize_role_payload(input, ArtifactRole::MetricContributions, contributions)?,
        serialize_role_payload(input, ArtifactRole::Metrics, aggregates)?,
    ])
}

fn serialize_role_payload<T: Serialize>(
    input: &RealTranscriptEvaluationExecutionInput,
    role: ArtifactRole,
    value: &T,
) -> Result<RealTranscriptEvaluationSerializedArtifact, RealTranscriptEvaluationExecutionError> {
    let artifact_id = artifact_id_for_role(input, role)?;
    let payload_bytes = serde_json::to_vec(value)
        .map_err(|_| RealTranscriptEvaluationExecutionError::PayloadSerializationFailure)?;
    let content_digest = compute_payload_digest(&payload_bytes)?;
    let byte_length = payload_byte_length(&payload_bytes)?;
    Ok(RealTranscriptEvaluationSerializedArtifact {
        artifact_id,
        role,
        payload_schema: schema_for_role(role)?,
        payload_bytes,
        content_digest,
        byte_length,
    })
}

fn build_bundle_from_payloads(
    request: &RealTranscriptEvaluationRunRequest,
    input: &RealTranscriptEvaluationExecutionInput,
    binding_context: &ArtifactBindingContext,
    payloads: &[RealTranscriptEvaluationSerializedArtifact],
    bundle_state: ArtifactBundleState,
) -> Result<ArtifactBundle, RealTranscriptEvaluationExecutionError> {
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

    let expected_roles = request.expected_artifact_roles.clone();
    let assessment =
        ArtifactBundle::derive_assessment(&expected_roles, &artifacts, binding_context)
            .map_err(RealTranscriptEvaluationExecutionError::BootstrapBundleValidationFailure)?;

    let bundle = ArtifactBundle {
        schema_revision: ARTIFACT_BUNDLE_SCHEMA.to_string(),
        bundle_id: input.artifact_ids.bundle.clone(),
        binding_context: binding_context.clone(),
        expected_roles,
        artifacts,
        bundle_state,
        assessment,
    };
    bundle
        .validate()
        .map_err(RealTranscriptEvaluationExecutionError::BootstrapBundleValidationFailure)?;
    Ok(bundle)
}

fn verify_payload_integrity(
    result: &RealTranscriptEvaluationCompletedResult,
) -> Result<(), RealTranscriptEvaluationExecutionError> {
    if result.serialized_payloads.len() != FINAL_ARTIFACT_ROLES.len() {
        return Err(RealTranscriptEvaluationExecutionError::MissingPayload {
            role: ArtifactRole::InputAuthorization,
        });
    }

    let mut seen_roles = std::collections::BTreeSet::new();
    let mut seen_ids = std::collections::HashSet::new();

    for (index, expected_role) in FINAL_ARTIFACT_ROLES.iter().enumerate() {
        let payload = &result.serialized_payloads[index];
        if payload.role != *expected_role {
            return Err(
                RealTranscriptEvaluationExecutionError::PayloadSchemaMismatch {
                    role: *expected_role,
                },
            );
        }
        if !seen_roles.insert(payload.role) {
            return Err(
                RealTranscriptEvaluationExecutionError::DuplicatePayloadRole { role: payload.role },
            );
        }
        if !seen_ids.insert(payload.artifact_id.clone()) {
            return Err(
                RealTranscriptEvaluationExecutionError::DuplicatePayloadArtifactId {
                    artifact_id: payload.artifact_id.clone(),
                },
            );
        }

        let expected_schema = schema_for_role(payload.role)?;
        if payload.payload_schema != expected_schema {
            return Err(
                RealTranscriptEvaluationExecutionError::PayloadSchemaMismatch {
                    role: payload.role,
                },
            );
        }

        let expected_id = artifact_id_for_role_in_bundle(&result.final_bundle, payload.role)?;
        if payload.artifact_id != expected_id {
            return Err(
                RealTranscriptEvaluationExecutionError::PayloadArtifactIdMismatch {
                    role: payload.role,
                },
            );
        }

        let digest = compute_payload_digest(&payload.payload_bytes)?;
        if digest != payload.content_digest {
            return Err(
                RealTranscriptEvaluationExecutionError::PayloadDigestMismatch {
                    role: payload.role,
                },
            );
        }

        let byte_length = payload_byte_length(&payload.payload_bytes)?;
        if byte_length != payload.byte_length {
            return Err(
                RealTranscriptEvaluationExecutionError::PayloadLengthMismatch {
                    role: payload.role,
                },
            );
        }

        let descriptor = result
            .final_bundle
            .artifacts
            .iter()
            .find(|entry| entry.role == payload.role)
            .ok_or(RealTranscriptEvaluationExecutionError::MissingPayload { role: payload.role })?;

        if descriptor.content_digest != payload.content_digest {
            return Err(
                RealTranscriptEvaluationExecutionError::PayloadDigestMismatch {
                    role: payload.role,
                },
            );
        }
        if descriptor.byte_length != payload.byte_length {
            return Err(
                RealTranscriptEvaluationExecutionError::PayloadLengthMismatch {
                    role: payload.role,
                },
            );
        }
    }

    if result.final_bundle.expected_roles != result.request.expected_artifact_roles {
        return Err(
            RealTranscriptEvaluationExecutionError::FinalBundleValidationFailure(
                ArtifactBundleValidationError::ExpectedRolesMismatch,
            ),
        );
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DecodedArtifactSet {
    input_authorization: InputAuthorization,
    reference_seal: ReferenceSeal,
    human_final_reference: HumanFinalReference,
    reference_coverage: ReferenceCoverage,
    detector_snapshot: DetectorProposalSnapshot,
    join: DetectorReferenceJoin,
    adjudication: OverlapAdjudicationSet,
    contributions: JoinMetricContributionSet,
    aggregates: JoinMetricAggregateSet,
}

fn verify_typed_payload_round_trip(
    result: &RealTranscriptEvaluationCompletedResult,
) -> Result<(), RealTranscriptEvaluationExecutionError> {
    let decoded = decode_artifact_set(&result.serialized_payloads)?;
    let authoritative = authoritative_artifact_set_from_result(result);
    verify_decoded_authoritative_equality(&decoded, &authoritative)?;
    validate_decoded_local(&decoded)?;
    verify_decoded_reserialization(&result.serialized_payloads, &decoded)?;
    decoded_historical_replay_validate(
        &result.request.finalized_envelope,
        &result.request.input_authorization,
        &decoded,
        &result.final_bundle,
    )?;
    Ok(())
}

fn authoritative_artifact_set_from_result(
    result: &RealTranscriptEvaluationCompletedResult,
) -> DecodedArtifactSet {
    DecodedArtifactSet {
        input_authorization: result.request.input_authorization.clone(),
        reference_seal: result.request.reference_seal.clone(),
        human_final_reference: result.request.human_final_reference.clone(),
        reference_coverage: result.request.reference_coverage.clone(),
        detector_snapshot: result.detector_snapshot.clone(),
        join: result.final_join.clone(),
        adjudication: result.final_adjudication_set.clone(),
        contributions: result.final_contributions.clone(),
        aggregates: result.final_aggregates.clone(),
    }
}

fn decode_artifact_set(
    payloads: &[RealTranscriptEvaluationSerializedArtifact],
) -> Result<DecodedArtifactSet, RealTranscriptEvaluationExecutionError> {
    macro_rules! decode_role {
        ($role:expr, $from_json:expr) => {{
            let payload = payload_for_role(payloads, $role)?.payload_bytes.as_slice();
            let json = std::str::from_utf8(payload).map_err(|_| {
                RealTranscriptEvaluationExecutionError::PayloadDeserializationFailure {
                    role: $role,
                }
            })?;
            $from_json(json).map_err(|_| {
                RealTranscriptEvaluationExecutionError::PayloadDeserializationFailure {
                    role: $role,
                }
            })?
        }};
    }

    Ok(DecodedArtifactSet {
        input_authorization: decode_role!(
            ArtifactRole::InputAuthorization,
            input_authorization_from_json
        ),
        reference_seal: decode_role!(ArtifactRole::ReferenceSeal, seal_from_json),
        human_final_reference: decode_role!(
            ArtifactRole::HumanFinalReference,
            human_final_reference_from_json
        ),
        reference_coverage: decode_role!(ArtifactRole::CueReviewCompletion, coverage_from_json),
        detector_snapshot: decode_role!(
            ArtifactRole::DetectorOutput,
            detector_proposal_snapshot_from_json
        ),
        join: decode_role!(ArtifactRole::EvaluationJoin, join_from_json),
        adjudication: decode_role!(
            ArtifactRole::JoinAdjudication,
            overlap_adjudication_from_json
        ),
        contributions: decode_role!(ArtifactRole::MetricContributions, contribution_from_json),
        aggregates: decode_role!(ArtifactRole::Metrics, aggregate_from_json),
    })
}

fn verify_decoded_authoritative_equality(
    decoded: &DecodedArtifactSet,
    authoritative: &DecodedArtifactSet,
) -> Result<(), RealTranscriptEvaluationExecutionError> {
    let checks = [
        (
            ArtifactRole::InputAuthorization,
            decoded.input_authorization == authoritative.input_authorization,
        ),
        (
            ArtifactRole::ReferenceSeal,
            decoded.reference_seal == authoritative.reference_seal,
        ),
        (
            ArtifactRole::HumanFinalReference,
            decoded.human_final_reference == authoritative.human_final_reference,
        ),
        (
            ArtifactRole::CueReviewCompletion,
            decoded.reference_coverage == authoritative.reference_coverage,
        ),
        (
            ArtifactRole::DetectorOutput,
            decoded.detector_snapshot == authoritative.detector_snapshot,
        ),
        (
            ArtifactRole::EvaluationJoin,
            decoded.join == authoritative.join,
        ),
        (
            ArtifactRole::JoinAdjudication,
            decoded.adjudication == authoritative.adjudication,
        ),
        (
            ArtifactRole::MetricContributions,
            decoded.contributions == authoritative.contributions,
        ),
        (
            ArtifactRole::Metrics,
            decoded.aggregates == authoritative.aggregates,
        ),
    ];
    for (role, ok) in checks {
        if !ok {
            return Err(RealTranscriptEvaluationExecutionError::TypedPayloadMismatch { role });
        }
    }
    Ok(())
}

fn validate_decoded_local(
    decoded: &DecodedArtifactSet,
) -> Result<(), RealTranscriptEvaluationExecutionError> {
    decoded.input_authorization.validate().map_err(|_| {
        RealTranscriptEvaluationExecutionError::DecodedPayloadValidationFailure {
            role: ArtifactRole::InputAuthorization,
        }
    })?;
    decoded.reference_seal.validate().map_err(|_| {
        RealTranscriptEvaluationExecutionError::DecodedPayloadValidationFailure {
            role: ArtifactRole::ReferenceSeal,
        }
    })?;
    decoded.human_final_reference.validate().map_err(|_| {
        RealTranscriptEvaluationExecutionError::DecodedPayloadValidationFailure {
            role: ArtifactRole::HumanFinalReference,
        }
    })?;
    decoded.reference_coverage.validate().map_err(|_| {
        RealTranscriptEvaluationExecutionError::DecodedPayloadValidationFailure {
            role: ArtifactRole::CueReviewCompletion,
        }
    })?;
    decoded.detector_snapshot.validate().map_err(|_| {
        RealTranscriptEvaluationExecutionError::DecodedPayloadValidationFailure {
            role: ArtifactRole::DetectorOutput,
        }
    })?;
    decoded.join.validate().map_err(|_| {
        RealTranscriptEvaluationExecutionError::DecodedPayloadValidationFailure {
            role: ArtifactRole::EvaluationJoin,
        }
    })?;
    decoded.adjudication.validate().map_err(|_| {
        RealTranscriptEvaluationExecutionError::DecodedPayloadValidationFailure {
            role: ArtifactRole::JoinAdjudication,
        }
    })?;
    decoded.contributions.validate().map_err(|_| {
        RealTranscriptEvaluationExecutionError::DecodedPayloadValidationFailure {
            role: ArtifactRole::MetricContributions,
        }
    })?;
    decoded.aggregates.validate().map_err(|_| {
        RealTranscriptEvaluationExecutionError::DecodedPayloadValidationFailure {
            role: ArtifactRole::Metrics,
        }
    })?;
    Ok(())
}

fn verify_decoded_reserialization(
    payloads: &[RealTranscriptEvaluationSerializedArtifact],
    decoded: &DecodedArtifactSet,
) -> Result<(), RealTranscriptEvaluationExecutionError> {
    let role_payloads = [
        (
            ArtifactRole::InputAuthorization,
            serde_json::to_vec(&decoded.input_authorization),
        ),
        (
            ArtifactRole::ReferenceSeal,
            serde_json::to_vec(&decoded.reference_seal),
        ),
        (
            ArtifactRole::HumanFinalReference,
            serde_json::to_vec(&decoded.human_final_reference),
        ),
        (
            ArtifactRole::CueReviewCompletion,
            serde_json::to_vec(&decoded.reference_coverage),
        ),
        (
            ArtifactRole::DetectorOutput,
            serde_json::to_vec(&decoded.detector_snapshot),
        ),
        (
            ArtifactRole::EvaluationJoin,
            serde_json::to_vec(&decoded.join),
        ),
        (
            ArtifactRole::JoinAdjudication,
            serde_json::to_vec(&decoded.adjudication),
        ),
        (
            ArtifactRole::MetricContributions,
            serde_json::to_vec(&decoded.contributions),
        ),
        (
            ArtifactRole::Metrics,
            serde_json::to_vec(&decoded.aggregates),
        ),
    ];

    for (role, reserialized) in role_payloads {
        let reserialized = reserialized
            .map_err(|_| RealTranscriptEvaluationExecutionError::PayloadSerializationFailure)?;
        let payload = payload_for_role(payloads, role)?;
        if reserialized != payload.payload_bytes {
            return Err(
                RealTranscriptEvaluationExecutionError::DecodedPayloadReserializationMismatch {
                    role,
                },
            );
        }
    }
    Ok(())
}

fn decoded_historical_replay_validate(
    finalized_envelope: &RunEnvelope,
    input_authorization: &InputAuthorization,
    decoded: &DecodedArtifactSet,
    bundle: &ArtifactBundle,
) -> Result<(), RealTranscriptEvaluationExecutionError> {
    if input_authorization.run_id != finalized_envelope.run_id
        || input_authorization.input_identity != finalized_envelope.input_identity
        || input_authorization.input_class != finalized_envelope.input_class
        || input_authorization.state
            != crate::input_authorization::InputAuthorizationState::Confirmed
        || input_authorization.scope_policy_revision
            != crate::input_authorization::INPUT_AUTHORIZATION_SCOPE_POLICY
    {
        return Err(
            RealTranscriptEvaluationExecutionError::HistoricalReplayValidationFailure {
                component: "input_authorization",
            },
        );
    }

    decoded
        .reference_seal
        .validate_historical_context(finalized_envelope)
        .map_err(
            |_| RealTranscriptEvaluationExecutionError::HistoricalReplayValidationFailure {
                component: "reference_seal",
            },
        )?;
    decoded
        .reference_coverage
        .validate_historical_context(
            finalized_envelope,
            &decoded.reference_seal,
            Some(&decoded.human_final_reference),
        )
        .map_err(
            |_| RealTranscriptEvaluationExecutionError::HistoricalReplayValidationFailure {
                component: "reference_coverage",
            },
        )?;
    decoded
        .human_final_reference
        .validate_historical_context(finalized_envelope, &decoded.reference_seal)
        .map_err(
            |_| RealTranscriptEvaluationExecutionError::HistoricalReplayValidationFailure {
                component: "human_final_reference",
            },
        )?;
    decoded
        .detector_snapshot
        .validate_against_bundle(finalized_envelope, bundle)
        .map_err(
            |_| RealTranscriptEvaluationExecutionError::HistoricalReplayValidationFailure {
                component: "detector_snapshot",
            },
        )?;
    decoded
        .adjudication
        .validate_against_envelope(finalized_envelope)
        .map_err(
            |_| RealTranscriptEvaluationExecutionError::HistoricalReplayValidationFailure {
                component: "adjudication_set",
            },
        )?;
    decoded
        .join
        .validate_against(
            finalized_envelope,
            &decoded.reference_seal,
            &decoded.reference_coverage,
            &decoded.human_final_reference,
            &decoded.detector_snapshot,
            bundle,
            &decoded.adjudication,
        )
        .map_err(
            |_| RealTranscriptEvaluationExecutionError::HistoricalReplayValidationFailure {
                component: "join",
            },
        )?;
    decoded
        .contributions
        .validate_against(
            finalized_envelope,
            &decoded.reference_seal,
            &decoded.reference_coverage,
            &decoded.human_final_reference,
            &decoded.detector_snapshot,
            &decoded.join,
            &decoded.adjudication,
            bundle,
        )
        .map_err(
            |_| RealTranscriptEvaluationExecutionError::HistoricalReplayValidationFailure {
                component: "contributions",
            },
        )?;
    decoded
        .aggregates
        .validate_against(
            finalized_envelope,
            &decoded.reference_seal,
            &decoded.reference_coverage,
            &decoded.human_final_reference,
            &decoded.detector_snapshot,
            &decoded.join,
            &decoded.adjudication,
            &decoded.contributions,
            bundle,
        )
        .map_err(
            |_| RealTranscriptEvaluationExecutionError::HistoricalReplayValidationFailure {
                component: "aggregates",
            },
        )?;
    bundle
        .validate_with_reference_context(
            finalized_envelope,
            Some(&decoded.reference_seal),
            Some(&decoded.reference_coverage),
            Some(&decoded.human_final_reference),
        )
        .map_err(
            |_| RealTranscriptEvaluationExecutionError::HistoricalReplayValidationFailure {
                component: "artifact_bundle",
            },
        )?;
    Ok(())
}

fn payload_for_role(
    payloads: &[RealTranscriptEvaluationSerializedArtifact],
    role: ArtifactRole,
) -> Result<&RealTranscriptEvaluationSerializedArtifact, RealTranscriptEvaluationExecutionError> {
    payloads
        .iter()
        .find(|payload| payload.role == role)
        .ok_or(RealTranscriptEvaluationExecutionError::MissingPayload { role })
}

fn schema_for_role(
    role: ArtifactRole,
) -> Result<ArtifactSchemaIdentity, RealTranscriptEvaluationExecutionError> {
    let schema_id = match role {
        ArtifactRole::InputAuthorization => INPUT_AUTHORIZATION_SCHEMA,
        ArtifactRole::ReferenceSeal => REFERENCE_SEAL_SCHEMA,
        ArtifactRole::HumanFinalReference => HUMAN_FINAL_REFERENCE_SCHEMA,
        ArtifactRole::CueReviewCompletion => REFERENCE_COVERAGE_SCHEMA,
        ArtifactRole::DetectorOutput => DETECTOR_PROPOSAL_SNAPSHOT_SCHEMA,
        ArtifactRole::EvaluationJoin => DETECTOR_REFERENCE_JOIN_SCHEMA,
        ArtifactRole::JoinAdjudication => OVERLAP_ADJUDICATION_SCHEMA,
        ArtifactRole::MetricContributions => JOIN_METRIC_CONTRIBUTION_SCHEMA,
        ArtifactRole::Metrics => JOIN_METRIC_AGGREGATION_SCHEMA,
        _ => {
            return Err(RealTranscriptEvaluationExecutionError::PayloadSchemaMismatch { role });
        }
    };
    ArtifactSchemaIdentity::new(schema_id, "v1")
        .map_err(|_| RealTranscriptEvaluationExecutionError::PayloadSchemaMismatch { role })
}

fn artifact_id_for_role(
    input: &RealTranscriptEvaluationExecutionInput,
    role: ArtifactRole,
) -> Result<ArtifactId, RealTranscriptEvaluationExecutionError> {
    Ok(match role {
        ArtifactRole::InputAuthorization => input.artifact_ids.input_authorization.clone(),
        ArtifactRole::ReferenceSeal => input.artifact_ids.reference_seal.clone(),
        ArtifactRole::HumanFinalReference => input.artifact_ids.human_final_reference.clone(),
        ArtifactRole::CueReviewCompletion => input.artifact_ids.cue_review_completion.clone(),
        ArtifactRole::DetectorOutput => input.artifact_ids.detector_output.clone(),
        ArtifactRole::EvaluationJoin => input.artifact_ids.evaluation_join.clone(),
        ArtifactRole::JoinAdjudication => input.artifact_ids.join_adjudication.clone(),
        ArtifactRole::MetricContributions => input.artifact_ids.metric_contributions.clone(),
        ArtifactRole::Metrics => input.artifact_ids.metrics.clone(),
        _ => {
            return Err(RealTranscriptEvaluationExecutionError::MissingPayload { role });
        }
    })
}

fn artifact_id_for_role_in_bundle(
    bundle: &ArtifactBundle,
    role: ArtifactRole,
) -> Result<ArtifactId, RealTranscriptEvaluationExecutionError> {
    bundle
        .artifacts
        .iter()
        .find(|descriptor| descriptor.role == role)
        .map(|descriptor| descriptor.artifact_id.clone())
        .ok_or(RealTranscriptEvaluationExecutionError::MissingPayload { role })
}

fn compute_payload_digest(
    bytes: &[u8],
) -> Result<ArtifactContentDigest, RealTranscriptEvaluationExecutionError> {
    let hash = Sha256::digest(bytes);
    let hex = hash
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    ArtifactContentDigest::new(format!("sha256:{hex}")).map_err(|_| {
        RealTranscriptEvaluationExecutionError::PayloadDigestMismatch {
            role: ArtifactRole::InputAuthorization,
        }
    })
}

fn payload_byte_length(bytes: &[u8]) -> Result<u64, RealTranscriptEvaluationExecutionError> {
    bytes
        .len()
        .try_into()
        .map_err(|_| RealTranscriptEvaluationExecutionError::IntegerConversionOverflow)
}

fn artifact_ids_from_input(input: &RealTranscriptEvaluationExecutionInput) -> Vec<ArtifactId> {
    vec![
        input.artifact_ids.input_authorization.clone(),
        input.artifact_ids.reference_seal.clone(),
        input.artifact_ids.human_final_reference.clone(),
        input.artifact_ids.cue_review_completion.clone(),
        input.artifact_ids.detector_output.clone(),
        input.artifact_ids.evaluation_join.clone(),
        input.artifact_ids.join_adjudication.clone(),
        input.artifact_ids.metric_contributions.clone(),
        input.artifact_ids.metrics.clone(),
    ]
}

fn push_trace(
    trace: &mut Vec<RealTranscriptEvaluationStageRecord>,
    stage: RealTranscriptEvaluationStage,
    lifecycle_state: RunLifecycleState,
    related_artifact_ids: Vec<ArtifactId>,
) {
    trace.push(RealTranscriptEvaluationStageRecord {
        stage,
        lifecycle_state,
        related_artifact_ids,
    });
}

use std::fmt;

use crate::artifact_bundle::{ArtifactBundleId, ArtifactId};
use crate::detector_reference_join::{
    DETECTOR_REFERENCE_JOIN_SCHEMA, DetectorReferenceJoinContext, OVERLAP_RULE_REVISION,
};
use crate::detector_snapshot::DetectorProposalSnapshot;
use crate::join_adjudication::{
    OVERLAP_ADJUDICATION_SCHEMA, OverlapAdjudicationSet, OverlapAdjudicationSetId,
    OverlapAdjudicationSetState,
};
use crate::join_metric_aggregation::JoinMetricAggregateContext;
use crate::join_metric_contribution::JoinMetricContributionContext;
use crate::pipeline::CanonicalTermReviewRun;
use crate::real_transcript_detector_snapshot_adapter::{
    RealTranscriptDetectorSnapshotAdapterRequest,
    RealTranscriptDetectorSnapshotMaterializationError,
    materialize_real_transcript_detector_snapshot,
};
use crate::real_transcript_evaluation_execution::{
    RealTranscriptEvaluationArtifactIds, RealTranscriptEvaluationCompletedResult,
    RealTranscriptEvaluationExecutionError, RealTranscriptEvaluationExecutionInput,
    RealTranscriptEvaluationExecutionOutcome, RealTranscriptEvaluationPendingResult,
    RealTranscriptEvaluationRevisionIds, execute_real_transcript_evaluation,
};
use crate::real_transcript_evaluation_runner::RealTranscriptEvaluationRunRequest;
use crate::transcript::Transcript;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealTranscriptInitialExecutionBindings {
    pub input_authorization_artifact_id: ArtifactId,
    pub reference_seal_artifact_id: ArtifactId,
    pub human_final_reference_artifact_id: ArtifactId,
    pub cue_review_completion_artifact_id: ArtifactId,
    pub join_context: DetectorReferenceJoinContext,
    pub contribution_context: JoinMetricContributionContext,
    pub aggregate_context: JoinMetricAggregateContext,
    pub bundle_id: ArtifactBundleId,
    pub detector_execution_adjudication_set_id: OverlapAdjudicationSetId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum RealTranscriptInitialExecutionOutcome {
    RequiresHumanAdjudication {
        detector_snapshot: DetectorProposalSnapshot,
        pending: RealTranscriptEvaluationPendingResult,
    },
    Completed(RealTranscriptEvaluationCompletedResult),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RealTranscriptInitialExecutionError {
    Materialization(RealTranscriptDetectorSnapshotMaterializationError),
    Execution(RealTranscriptEvaluationExecutionError),
}

impl fmt::Display for RealTranscriptInitialExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for RealTranscriptInitialExecutionError {}

pub fn materialize_and_begin_real_transcript_evaluation(
    run_request: &RealTranscriptEvaluationRunRequest,
    adapter_request: &RealTranscriptDetectorSnapshotAdapterRequest,
    transcript: &Transcript,
    canonical_run: &CanonicalTermReviewRun,
    bindings: &RealTranscriptInitialExecutionBindings,
) -> Result<RealTranscriptInitialExecutionOutcome, RealTranscriptInitialExecutionError> {
    let materialized = materialize_real_transcript_detector_snapshot(
        run_request,
        adapter_request,
        transcript,
        canonical_run,
    )
    .map_err(RealTranscriptInitialExecutionError::Materialization)?;

    let detector_execution_adjudication_set =
        initial_detector_adjudication_set(run_request, &materialized.detector_snapshot, bindings);
    let artifact_ids = initial_artifact_ids(&materialized.detector_snapshot, bindings);
    let revision_ids = RealTranscriptEvaluationRevisionIds {
        join_context: bindings.join_context.clone(),
        contribution_context: bindings.contribution_context.clone(),
        aggregate_context: bindings.aggregate_context.clone(),
    };
    let input = RealTranscriptEvaluationExecutionInput {
        detector_snapshot: materialized.detector_snapshot,
        detector_execution_adjudication_set,
        assisted_review_adjudication_set: None,
        artifact_ids,
        revision_ids,
    };

    match execute_real_transcript_evaluation(run_request, &input)
        .map_err(RealTranscriptInitialExecutionError::Execution)?
    {
        RealTranscriptEvaluationExecutionOutcome::RequiresHumanAdjudication(pending) => Ok(
            RealTranscriptInitialExecutionOutcome::RequiresHumanAdjudication {
                detector_snapshot: input.detector_snapshot,
                pending,
            },
        ),
        RealTranscriptEvaluationExecutionOutcome::Completed(result) => {
            Ok(RealTranscriptInitialExecutionOutcome::Completed(result))
        }
    }
}

fn initial_artifact_ids(
    detector_snapshot: &DetectorProposalSnapshot,
    bindings: &RealTranscriptInitialExecutionBindings,
) -> RealTranscriptEvaluationArtifactIds {
    RealTranscriptEvaluationArtifactIds {
        input_authorization: bindings.input_authorization_artifact_id.clone(),
        reference_seal: bindings.reference_seal_artifact_id.clone(),
        human_final_reference: bindings.human_final_reference_artifact_id.clone(),
        cue_review_completion: bindings.cue_review_completion_artifact_id.clone(),
        detector_output: detector_snapshot.detector_output_artifact_id.clone(),
        evaluation_join: bindings.join_context.evaluation_join_artifact_id.clone(),
        join_adjudication: bindings.join_context.join_adjudication_artifact_id.clone(),
        metric_contributions: bindings
            .contribution_context
            .metric_contributions_artifact_id
            .clone(),
        metrics: bindings.aggregate_context.metrics_artifact_id.clone(),
        bundle: bindings.bundle_id.clone(),
    }
}

fn initial_detector_adjudication_set(
    run_request: &RealTranscriptEvaluationRunRequest,
    detector_snapshot: &DetectorProposalSnapshot,
    bindings: &RealTranscriptInitialExecutionBindings,
) -> OverlapAdjudicationSet {
    let records = Vec::new();
    let assessment = OverlapAdjudicationSet::derive_assessment(&records);

    OverlapAdjudicationSet {
        schema_revision: OVERLAP_ADJUDICATION_SCHEMA.to_string(),
        adjudication_set_id: bindings.detector_execution_adjudication_set_id.clone(),
        run_id: run_request.detector_execution_envelope.run_id.clone(),
        input_identity: run_request
            .detector_execution_envelope
            .input_identity
            .clone(),
        reference_revision: run_request.reference_seal.reference_revision.clone(),
        detector_snapshot_revision: detector_snapshot.snapshot_revision.clone(),
        join_contract_revision: DETECTOR_REFERENCE_JOIN_SCHEMA.to_string(),
        overlap_rule_revision: OVERLAP_RULE_REVISION.to_string(),
        join_adjudication_artifact_id: bindings.join_context.join_adjudication_artifact_id.clone(),
        state: OverlapAdjudicationSetState::Frozen,
        records,
        assessment,
    }
}

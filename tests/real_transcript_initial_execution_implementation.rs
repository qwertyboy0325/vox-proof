#[allow(dead_code)]
#[path = "support/real_transcript_detector_snapshot_adapter_fixtures.rs"]
mod fixtures;

use std::fs;

use fixtures::{
    adapter_request_for, aligned_run_request, combined_canonical_fixture,
    single_cue_zero_run_request, zero_candidate_fixture,
};
use vox_proof::artifact_bundle::{ArtifactBundleId, ArtifactId};
use vox_proof::candidate::SessionTermEntry;
use vox_proof::detector_reference_join::{
    DetectorReferenceJoinContext, DetectorReferenceJoinId, DetectorReferenceJoinRevisionId,
};
use vox_proof::join_adjudication::{OverlapAdjudicationSetId, OverlapAdjudicationSetState};
use vox_proof::join_metric_aggregation::{
    JoinMetricAggregateContext, MetricAggregateRevisionId, MetricAggregateSetId,
};
use vox_proof::join_metric_contribution::{
    JoinMetricContributionContext, MetricContributionRevisionId, MetricContributionSetId,
};
use vox_proof::pipeline::{CanonicalTermReviewRun, run_canonical_term_review};
use vox_proof::real_transcript_detector_snapshot_adapter::{
    RealTranscriptDetectorSnapshotMaterializationError,
    materialize_real_transcript_detector_snapshot,
};
use vox_proof::real_transcript_evaluation_execution::{
    RealTranscriptEvaluationCompletionStage, RealTranscriptEvaluationExecutionError,
};
use vox_proof::real_transcript_initial_execution::{
    RealTranscriptInitialExecutionBindings, RealTranscriptInitialExecutionError,
    RealTranscriptInitialExecutionOutcome, materialize_and_begin_real_transcript_evaluation,
};
use vox_proof::run_manifest::InputClass;
use vox_proof::srt::parse_srt;
use vox_proof::transcript::Transcript;

const ARTIFICIAL_FIXTURE_PROVENANCE: &str =
    "inline-artificial-content-with-required-real-posture-contract-metadata";

fn assert_artificial_real_posture_fixture(
    run_request: &vox_proof::real_transcript_evaluation_runner::RealTranscriptEvaluationRunRequest,
) {
    assert_eq!(
        ARTIFICIAL_FIXTURE_PROVENANCE,
        "inline-artificial-content-with-required-real-posture-contract-metadata"
    );
    assert!(matches!(
        run_request.input_authorization.input_class,
        InputClass::SelfOwnedReal | InputClass::ExplicitPermissionReal
    ));
    assert!(
        run_request
            .detector_execution_envelope
            .qualifies_as_real_material_evidence
    );
}

fn bindings() -> RealTranscriptInitialExecutionBindings {
    RealTranscriptInitialExecutionBindings {
        input_authorization_artifact_id: ArtifactId::new("artifact-input-auth-initial-exec")
            .expect("id"),
        reference_seal_artifact_id: ArtifactId::new("artifact-reference-seal-initial-exec")
            .expect("id"),
        human_final_reference_artifact_id: ArtifactId::new("artifact-human-reference-initial-exec")
            .expect("id"),
        cue_review_completion_artifact_id: ArtifactId::new("artifact-cue-coverage-initial-exec")
            .expect("id"),
        join_context: DetectorReferenceJoinContext {
            join_id: DetectorReferenceJoinId::new("join-initial-exec-001").expect("join id"),
            join_revision: DetectorReferenceJoinRevisionId::new("join-rev-initial-exec-001")
                .expect("join revision"),
            evaluation_join_artifact_id: ArtifactId::new("artifact-evaluation-join-initial-exec")
                .expect("id"),
            join_adjudication_artifact_id: ArtifactId::new(
                "artifact-join-adjudication-initial-exec",
            )
            .expect("id"),
        },
        contribution_context: JoinMetricContributionContext {
            contribution_set_id: MetricContributionSetId::new("metric-contrib-set-initial-001")
                .expect("contribution set id"),
            contribution_revision: MetricContributionRevisionId::new(
                "metric-contrib-rev-initial-001",
            )
            .expect("contribution revision"),
            metric_contributions_artifact_id: ArtifactId::new(
                "artifact-metric-contributions-initial-exec",
            )
            .expect("id"),
        },
        aggregate_context: JoinMetricAggregateContext {
            aggregate_set_id: MetricAggregateSetId::new("metric-aggregate-set-initial-001")
                .expect("aggregate set id"),
            aggregate_revision: MetricAggregateRevisionId::new("metric-aggregate-rev-initial-001")
                .expect("aggregate revision"),
            metrics_artifact_id: ArtifactId::new("artifact-metrics-initial-exec").expect("id"),
        },
        bundle_id: ArtifactBundleId::new("bundle-initial-exec-001").expect("bundle id"),
        detector_execution_adjudication_set_id: OverlapAdjudicationSetId::new(
            "adj-set-initial-exec-empty",
        )
        .expect("adjudication set id"),
    }
}

fn non_empty_exact_artificial_fixture() -> (Transcript, CanonicalTermReviewRun) {
    let transcript = parse_srt(
        "1\n00:00:00,000 --> 00:00:01,000\nNo finding\n\n\
         2\n00:00:01,000 --> 00:00:02,000\nwrng",
    )
    .expect("valid srt");
    let entries = vec![SessionTermEntry::new(
        "wrong",
        vec!["wrng".to_string()],
        Vec::new(),
    )];
    let canonical_run =
        run_canonical_term_review(&transcript, &entries).expect("canonical exact run");
    assert!(!canonical_run.review_cases().is_empty());
    (transcript, canonical_run)
}

#[test]
fn artificial_zero_proposal_initial_execution_completes_at_detector_stage() {
    let (transcript, canonical_run) = zero_candidate_fixture();
    let run_request = single_cue_zero_run_request(&transcript, &canonical_run);
    let adapter_request = adapter_request_for(&canonical_run);
    assert_artificial_real_posture_fixture(&run_request);

    let outcome = materialize_and_begin_real_transcript_evaluation(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
        &bindings(),
    )
    .expect("zero proposal initial execution");

    let RealTranscriptInitialExecutionOutcome::Completed(result) = outcome else {
        panic!("expected completed zero-proposal outcome");
    };
    assert_eq!(
        result.completion_stage,
        RealTranscriptEvaluationCompletionStage::DetectorExecution
    );
    assert!(result.detector_snapshot.proposals.is_empty());
    assert_eq!(
        result.final_adjudication_set.state,
        OverlapAdjudicationSetState::Frozen
    );
    assert!(result.final_adjudication_set.records.is_empty());
    assert_eq!(result.final_adjudication_set.assessment.record_count, 0);
}

#[test]
fn artificial_non_empty_exact_initial_execution_completes_with_materialized_snapshot() {
    let (transcript, canonical_run) = non_empty_exact_artificial_fixture();
    let run_request = aligned_run_request(&transcript, &canonical_run, InputClass::SelfOwnedReal);
    let adapter_request = adapter_request_for(&canonical_run);
    assert_artificial_real_posture_fixture(&run_request);
    let expected = materialize_real_transcript_detector_snapshot(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
    )
    .expect("expected materialization");

    let outcome = materialize_and_begin_real_transcript_evaluation(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
        &bindings(),
    )
    .expect("exact initial execution");

    let RealTranscriptInitialExecutionOutcome::Completed(result) = outcome else {
        panic!("expected completed exact outcome");
    };
    assert_eq!(
        result.completion_stage,
        RealTranscriptEvaluationCompletionStage::DetectorExecution
    );
    assert_eq!(result.detector_snapshot, expected.detector_snapshot);
    assert!(!result.detector_snapshot.proposals.is_empty());
    assert!(result.final_adjudication_set.records.is_empty());
}

#[test]
fn artificial_overlap_pending_returns_the_snapshot_used_by_execution() {
    let (transcript, canonical_run) = combined_canonical_fixture();
    let run_request = aligned_run_request(&transcript, &canonical_run, InputClass::SelfOwnedReal);
    let adapter_request = adapter_request_for(&canonical_run);
    assert_artificial_real_posture_fixture(&run_request);
    let expected = materialize_real_transcript_detector_snapshot(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
    )
    .expect("expected materialization");

    let outcome = materialize_and_begin_real_transcript_evaluation(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
        &bindings(),
    )
    .expect("pending initial execution");

    let RealTranscriptInitialExecutionOutcome::RequiresHumanAdjudication {
        detector_snapshot,
        pending,
    } = outcome
    else {
        panic!("expected pending overlap outcome");
    };
    assert_eq!(detector_snapshot, expected.detector_snapshot);
    assert_eq!(
        detector_snapshot.snapshot_revision,
        pending
            .required_human_adjudication
            .detector_snapshot_revision
    );
    assert!(!pending.required_human_adjudication.overlap_pairs.is_empty());
}

#[test]
fn artificial_repeated_execution_is_deterministic_and_leaves_sources_unchanged() {
    let (transcript, canonical_run) = combined_canonical_fixture();
    let run_request = aligned_run_request(&transcript, &canonical_run, InputClass::SelfOwnedReal);
    let adapter_request = adapter_request_for(&canonical_run);
    assert_artificial_real_posture_fixture(&run_request);
    let transcript_revision_before = transcript.revision_id();
    let transcript_segments_before = transcript
        .segments()
        .iter()
        .map(|segment| {
            (
                segment.index(),
                segment.start_ms(),
                segment.end_ms(),
                segment.text().to_string(),
            )
        })
        .collect::<Vec<_>>();
    let analysis_before = canonical_run.analysis_run();
    let cases_before = canonical_run.review_cases().to_vec();
    let request_before = run_request.clone();
    let adapter_before = adapter_request.clone();
    let bindings = bindings();

    let first = materialize_and_begin_real_transcript_evaluation(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
        &bindings,
    )
    .expect("first execution");
    let second = materialize_and_begin_real_transcript_evaluation(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
        &bindings,
    )
    .expect("second execution");

    assert_eq!(first, second);
    assert_eq!(transcript.revision_id(), transcript_revision_before);
    assert_eq!(
        transcript
            .segments()
            .iter()
            .map(|segment| {
                (
                    segment.index(),
                    segment.start_ms(),
                    segment.end_ms(),
                    segment.text().to_string(),
                )
            })
            .collect::<Vec<_>>(),
        transcript_segments_before
    );
    assert_eq!(canonical_run.analysis_run(), analysis_before);
    assert_eq!(canonical_run.review_cases(), cases_before.as_slice());
    assert_eq!(run_request, request_before);
    assert_eq!(adapter_request, adapter_before);
}

#[test]
fn artificial_invalid_adapter_input_returns_materialization_stage_error() {
    let (transcript, canonical_run) = zero_candidate_fixture();
    let run_request = single_cue_zero_run_request(&transcript, &canonical_run);
    let mut adapter_request = adapter_request_for(&canonical_run);
    assert_artificial_real_posture_fixture(&run_request);
    adapter_request.schema_revision.clear();

    let error = materialize_and_begin_real_transcript_evaluation(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
        &bindings(),
    )
    .expect_err("invalid adapter must fail materialization");

    assert!(matches!(
        error,
        RealTranscriptInitialExecutionError::Materialization(
            RealTranscriptDetectorSnapshotMaterializationError::ContractValidationFailure(_)
        )
    ));
}

#[test]
fn artificial_duplicate_artifact_binding_returns_existing_execution_error() {
    let (transcript, canonical_run) = zero_candidate_fixture();
    let run_request = single_cue_zero_run_request(&transcript, &canonical_run);
    let adapter_request = adapter_request_for(&canonical_run);
    assert_artificial_real_posture_fixture(&run_request);
    let mut invalid_bindings = bindings();
    invalid_bindings.input_authorization_artifact_id =
        invalid_bindings.reference_seal_artifact_id.clone();

    let error = materialize_and_begin_real_transcript_evaluation(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
        &invalid_bindings,
    )
    .expect_err("duplicate artifact binding must fail execution");

    assert_eq!(
        error,
        RealTranscriptInitialExecutionError::Execution(
            RealTranscriptEvaluationExecutionError::DuplicateArtifactId
        )
    );
}

#[test]
fn artificial_duplicate_context_artifact_returns_existing_execution_error() {
    let (transcript, canonical_run) = zero_candidate_fixture();
    let run_request = single_cue_zero_run_request(&transcript, &canonical_run);
    let adapter_request = adapter_request_for(&canonical_run);
    assert_artificial_real_posture_fixture(&run_request);
    let mut invalid_bindings = bindings();
    invalid_bindings
        .contribution_context
        .metric_contributions_artifact_id = invalid_bindings
        .join_context
        .evaluation_join_artifact_id
        .clone();

    let error = materialize_and_begin_real_transcript_evaluation(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
        &invalid_bindings,
    )
    .expect_err("duplicate context artifact must fail execution");

    assert_eq!(
        error,
        RealTranscriptInitialExecutionError::Execution(
            RealTranscriptEvaluationExecutionError::DuplicateArtifactId
        )
    );
}

#[test]
fn artificial_fixture_target_proves_no_forbidden_operations_or_substitution_hooks() {
    let source = fs::read_to_string("src/real_transcript_initial_execution.rs")
        .expect("read production integration source");

    for forbidden in [
        "run_canonical_term_review(",
        "run_term_review(",
        "detect_glossary_matches(",
        "detect_observed_error_form_matches(",
        "detect_ascii_latin_phonetic_matches(",
        "std::fs",
        "std::path",
        "serde_json",
        "EvaluationArtifactPacket",
        "SystemTime",
        "UNIX_EPOCH",
        "rand::",
        "tokio",
        "TcpStream",
        "persistence",
        "OverlapAdjudicationRecord",
        "OverlapAdjudicatorRole",
    ] {
        assert!(
            !source.contains(forbidden),
            "forbidden integration dependency found: {forbidden}"
        );
    }

    let signature = source
        .split("pub fn materialize_and_begin_real_transcript_evaluation")
        .nth(1)
        .expect("public function")
        .split(") -> Result")
        .next()
        .expect("signature");
    for forbidden_parameter in [
        "DetectorProposalSnapshot",
        "RealTranscriptEvaluationExecutionInput",
        "OverlapAdjudicationSet",
        "assisted_review",
    ] {
        assert!(
            !signature.contains(forbidden_parameter),
            "forbidden public parameter found: {forbidden_parameter}"
        );
    }
}

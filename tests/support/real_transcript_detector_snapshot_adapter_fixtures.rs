use vox_proof::artifact_bundle::{ArtifactBundleId, ArtifactId};
use vox_proof::candidate::SessionTermEntry;
use vox_proof::detector_reference_join::{
    DetectorReferenceJoinContext, DetectorReferenceJoinId, DetectorReferenceJoinRevisionId,
};
use vox_proof::detector_snapshot::DetectorProposalId;
use vox_proof::human_final_reference::{
    HUMAN_FINAL_REFERENCE_SCHEMA, HumanFinalReference, HumanFinalReferenceState, ReferenceClass,
    ReferenceErrorId, ReferenceErrorRecord, ReferenceReviewerIdentityClass, ReferenceSourceAnchor,
    VerificationBasis,
};
use vox_proof::input_authorization::{
    INPUT_AUTHORIZATION_SCHEMA, INPUT_AUTHORIZATION_SCOPE_POLICY, InputAuthorization,
    InputAuthorizationBasis, InputAuthorizationId, InputAuthorizationState,
};
use vox_proof::join_adjudication::{
    OVERLAP_ADJUDICATION_SCHEMA, OverlapAdjudicationSet, OverlapAdjudicationSetId,
    OverlapAdjudicationSetState,
};
use vox_proof::join_metric_aggregation::{
    JoinMetricAggregateContext, MetricAggregateRevisionId, MetricAggregateSetId,
};
use vox_proof::join_metric_contribution::{
    JoinMetricContributionContext, MetricContributionRevisionId, MetricContributionSetId,
};
use vox_proof::pipeline::{CanonicalTermReviewRun, run_canonical_term_review};
use vox_proof::real_transcript_detector_snapshot_adapter::{
    REAL_TRANSCRIPT_DETECTOR_PROPOSAL_ID_POLICY, REAL_TRANSCRIPT_DETECTOR_SNAPSHOT_ADAPTER_POLICY,
    REAL_TRANSCRIPT_DETECTOR_SNAPSHOT_ADAPTER_REQUEST_SCHEMA,
    RealTranscriptDetectorSnapshotAdapterRequest,
    RealTranscriptDetectorSnapshotMaterializationResult,
};
use vox_proof::real_transcript_evaluation_execution::{
    RealTranscriptEvaluationArtifactIds, RealTranscriptEvaluationExecutionInput,
    RealTranscriptEvaluationRevisionIds,
};
use vox_proof::real_transcript_evaluation_runner::{
    REAL_TRANSCRIPT_EVALUATION_RUNNER_POLICY, REAL_TRANSCRIPT_EVALUATION_RUNNER_REQUEST_SCHEMA,
    REAL_TRANSCRIPT_OVERLAP_AUTHORITY_POLICY, RealTranscriptEvaluationRunRequest,
    canonical_real_evaluation_artifact_roles,
};
use vox_proof::reference_coverage::{
    CueReferenceId, CueReviewCompletionRecord, ExpectedCueUniverse, REFERENCE_COVERAGE_SCHEMA,
    ReferenceCoverage, ReferenceCoverageId, ReferenceCoveragePurpose, ReferenceCoverageState,
    ReferenceCueDisposition,
};
use vox_proof::reference_identity::{CueSourceTextDigest, ReferenceRevisionId};
use vox_proof::reference_seal::{
    CalibrationValidityImpact, REFERENCE_SEAL_SCHEMA, ReferenceCalibrationValidity,
    ReferenceProducerClass, ReferenceSeal, ReferenceSealId, ReferenceSealState,
};
use vox_proof::run_manifest::{
    CalibrationValidityMode, InputClass, InputIdentityReference, RUN_ENVELOPE_SCHEMA, RunEnvelope,
    RunId, RunLifecycleState, WorkflowObservationMode,
};
use vox_proof::srt::parse_srt;
use vox_proof::transcript::Transcript;

pub const RUN_ID: &str = "run-adapter-contract-001";
const SEAL_ID: &str = "seal-adapter-001";
const COVERAGE_ID: &str = "coverage-adapter-001";
pub const SAMPLE_REFERENCE_REVISION: &str = "ref-rev-adapter-001";
const SAMPLE_CUE_DIGEST: &str =
    "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
pub const FROZEN_AT_MS: u64 = 1_700_000_000_000;
const JOIN_CONTRACT_REVISION: &str = "voxproof-detector-reference-join-v1";

pub fn input_identity_for(transcript: &Transcript) -> InputIdentityReference {
    InputIdentityReference {
        transcript_revision_id: transcript.revision_id().to_tagged_string(),
    }
}

pub fn combined_canonical_fixture() -> (Transcript, CanonicalTermReviewRun) {
    let transcript = parse_srt(
        "1\n00:00:00,000 --> 00:00:01,000\nPostgre SQL\n\n2\n00:00:01,000 --> 00:00:02,000\nPostgres",
    )
    .expect("valid srt");
    let entries = vec![SessionTermEntry::new(
        "PostgreSQL",
        vec!["Postgres".to_string()],
        vec!["Postgre SQL".to_string()],
    )];
    let canonical_run = run_canonical_term_review(&transcript, &entries).expect("canonical run");
    (transcript, canonical_run)
}

pub fn zero_candidate_fixture() -> (Transcript, CanonicalTermReviewRun) {
    let transcript =
        parse_srt("1\n00:00:00,000 --> 00:00:01,000\nNo findings here").expect("valid srt");
    let canonical_run = run_canonical_term_review(&transcript, &[]).expect("zero findings");
    (transcript, canonical_run)
}

pub fn aligned_run_request(
    transcript: &Transcript,
    canonical_run: &CanonicalTermReviewRun,
    input_class: InputClass,
) -> RealTranscriptEvaluationRunRequest {
    let input_identity = input_identity_for(transcript);
    let detector_analysis_identity =
        vox_proof::real_transcript_detector_snapshot_adapter::derive_detector_analysis_identity_from_canonical_run(
            canonical_run,
        )
        .expect("identity");
    let mut identity = detector_analysis_identity;
    identity.input_identity = input_identity.clone();
    let coverage = primary_coverage(&input_identity, 2);

    RealTranscriptEvaluationRunRequest {
        schema_revision: REAL_TRANSCRIPT_EVALUATION_RUNNER_REQUEST_SCHEMA.to_string(),
        runner_policy_revision: REAL_TRANSCRIPT_EVALUATION_RUNNER_POLICY.to_string(),
        overlap_authority_policy_revision: REAL_TRANSCRIPT_OVERLAP_AUTHORITY_POLICY.to_string(),
        input_authorization: input_authorization_for(input_class, &input_identity),
        declared_envelope: envelope_at(RunLifecycleState::Declared, input_class, &input_identity),
        reference_preparation_envelope: envelope_at(
            RunLifecycleState::ReferencePreparation,
            input_class,
            &input_identity,
        ),
        reference_sealed_envelope: envelope_at(
            RunLifecycleState::ReferenceSealed,
            input_class,
            &input_identity,
        ),
        detector_execution_envelope: envelope_at(
            RunLifecycleState::DetectorExecution,
            input_class,
            &input_identity,
        ),
        assisted_review_transition_envelope: envelope_at(
            RunLifecycleState::AssistedReview,
            input_class,
            &input_identity,
        ),
        finalized_envelope: envelope_at(RunLifecycleState::Finalized, input_class, &input_identity),
        reference_seal: blind_seal(&input_identity),
        reference_coverage: coverage.clone(),
        human_final_reference: human_reference_for_coverage(&coverage),
        detector_analysis_identity: identity,
        expected_artifact_roles: canonical_real_evaluation_artifact_roles(),
    }
}

pub fn single_cue_zero_run_request(
    transcript: &Transcript,
    canonical_run: &CanonicalTermReviewRun,
) -> RealTranscriptEvaluationRunRequest {
    let input_identity = input_identity_for(transcript);
    let mut detector_analysis_identity =
        vox_proof::real_transcript_detector_snapshot_adapter::derive_detector_analysis_identity_from_canonical_run(
            canonical_run,
        )
        .expect("identity");
    detector_analysis_identity.input_identity = input_identity.clone();
    let coverage = primary_coverage(&input_identity, 1);

    RealTranscriptEvaluationRunRequest {
        schema_revision: REAL_TRANSCRIPT_EVALUATION_RUNNER_REQUEST_SCHEMA.to_string(),
        runner_policy_revision: REAL_TRANSCRIPT_EVALUATION_RUNNER_POLICY.to_string(),
        overlap_authority_policy_revision: REAL_TRANSCRIPT_OVERLAP_AUTHORITY_POLICY.to_string(),
        input_authorization: input_authorization_for(InputClass::SelfOwnedReal, &input_identity),
        declared_envelope: envelope_at(
            RunLifecycleState::Declared,
            InputClass::SelfOwnedReal,
            &input_identity,
        ),
        reference_preparation_envelope: envelope_at(
            RunLifecycleState::ReferencePreparation,
            InputClass::SelfOwnedReal,
            &input_identity,
        ),
        reference_sealed_envelope: envelope_at(
            RunLifecycleState::ReferenceSealed,
            InputClass::SelfOwnedReal,
            &input_identity,
        ),
        detector_execution_envelope: envelope_at(
            RunLifecycleState::DetectorExecution,
            InputClass::SelfOwnedReal,
            &input_identity,
        ),
        assisted_review_transition_envelope: envelope_at(
            RunLifecycleState::AssistedReview,
            InputClass::SelfOwnedReal,
            &input_identity,
        ),
        finalized_envelope: envelope_at(
            RunLifecycleState::Finalized,
            InputClass::SelfOwnedReal,
            &input_identity,
        ),
        reference_seal: blind_seal(&input_identity),
        reference_coverage: coverage.clone(),
        human_final_reference: human_reference_for_coverage(&coverage),
        detector_analysis_identity,
        expected_artifact_roles: canonical_real_evaluation_artifact_roles(),
    }
}

pub fn adapter_request_for(
    canonical_run: &CanonicalTermReviewRun,
) -> RealTranscriptDetectorSnapshotAdapterRequest {
    RealTranscriptDetectorSnapshotAdapterRequest {
        schema_revision: REAL_TRANSCRIPT_DETECTOR_SNAPSHOT_ADAPTER_REQUEST_SCHEMA.to_string(),
        adapter_policy_revision: REAL_TRANSCRIPT_DETECTOR_SNAPSHOT_ADAPTER_POLICY.to_string(),
        proposal_id_policy_revision: REAL_TRANSCRIPT_DETECTOR_PROPOSAL_ID_POLICY.to_string(),
        snapshot_revision: vox_proof::detector_snapshot::DetectorSnapshotRevisionId::new(
            "snap-rev-adapter-001",
        )
        .expect("rev"),
        detector_output_artifact_id: ArtifactId::new("artifact-det-out-adapter").expect("artifact"),
        frozen_at_unix_ms: FROZEN_AT_MS,
        proposal_ids: canonical_run
            .review_cases()
            .iter()
            .enumerate()
            .map(|(index, _)| {
                DetectorProposalId::new(format!("det-prop-adapter-{index:03}")).expect("id")
            })
            .collect(),
    }
}

pub fn reversed_proposal_ids(ids: &[DetectorProposalId]) -> Vec<DetectorProposalId> {
    let mut reversed = ids.to_vec();
    reversed.reverse();
    reversed
}

pub fn execution_input_for_materialized(
    run_request: &RealTranscriptEvaluationRunRequest,
    materialized: &RealTranscriptDetectorSnapshotMaterializationResult,
) -> RealTranscriptEvaluationExecutionInput {
    let snapshot = &materialized.detector_snapshot;
    let artifact_ids = RealTranscriptEvaluationArtifactIds {
        input_authorization: ArtifactId::new("artifact-input-auth-adapter-exec").expect("id"),
        reference_seal: ArtifactId::new("artifact-reference-seal-adapter-exec").expect("id"),
        human_final_reference: ArtifactId::new("artifact-human-reference-adapter-exec")
            .expect("id"),
        cue_review_completion: ArtifactId::new("artifact-cue-coverage-adapter-exec").expect("id"),
        detector_output: snapshot.detector_output_artifact_id.clone(),
        evaluation_join: ArtifactId::new("artifact-evaluation-join-adapter-exec").expect("id"),
        join_adjudication: ArtifactId::new("artifact-join-adjudication-adapter-exec").expect("id"),
        metric_contributions: ArtifactId::new("artifact-metric-contributions-adapter-exec")
            .expect("id"),
        metrics: ArtifactId::new("artifact-metrics-adapter-exec").expect("id"),
        bundle: ArtifactBundleId::new("bundle-adapter-exec-001").expect("id"),
    };

    let revision_ids = RealTranscriptEvaluationRevisionIds {
        join_context: DetectorReferenceJoinContext {
            join_id: DetectorReferenceJoinId::new("join-adapter-exec-001").expect("join id"),
            join_revision: DetectorReferenceJoinRevisionId::new("join-rev-adapter-exec-001")
                .expect("join revision"),
            evaluation_join_artifact_id: artifact_ids.evaluation_join.clone(),
            join_adjudication_artifact_id: artifact_ids.join_adjudication.clone(),
        },
        contribution_context: JoinMetricContributionContext {
            contribution_set_id: MetricContributionSetId::new("metric-contrib-set-adapter-001")
                .expect("contribution set id"),
            contribution_revision: MetricContributionRevisionId::new(
                "metric-contrib-rev-adapter-001",
            )
            .expect("contribution revision"),
            metric_contributions_artifact_id: artifact_ids.metric_contributions.clone(),
        },
        aggregate_context: JoinMetricAggregateContext {
            aggregate_set_id: MetricAggregateSetId::new("metric-aggregate-set-adapter-001")
                .expect("aggregate set id"),
            aggregate_revision: MetricAggregateRevisionId::new("metric-aggregate-rev-adapter-001")
                .expect("aggregate revision"),
            metrics_artifact_id: artifact_ids.metrics.clone(),
        },
    };

    let assessment = OverlapAdjudicationSet::derive_assessment(&[]);
    let detector_execution_adjudication_set = OverlapAdjudicationSet {
        schema_revision: OVERLAP_ADJUDICATION_SCHEMA.to_string(),
        adjudication_set_id: OverlapAdjudicationSetId::new("adj-set-adapter-exec-empty")
            .expect("set id"),
        run_id: run_request.declared_envelope.run_id.clone(),
        input_identity: run_request.declared_envelope.input_identity.clone(),
        reference_revision: run_request.reference_coverage.reference_revision.clone(),
        detector_snapshot_revision: snapshot.snapshot_revision.clone(),
        join_contract_revision: JOIN_CONTRACT_REVISION.to_string(),
        overlap_rule_revision: "voxproof-overlap-v1".to_string(),
        join_adjudication_artifact_id: artifact_ids.join_adjudication.clone(),
        state: OverlapAdjudicationSetState::Frozen,
        records: Vec::new(),
        assessment,
    };

    RealTranscriptEvaluationExecutionInput {
        detector_snapshot: snapshot.clone(),
        detector_execution_adjudication_set,
        assisted_review_adjudication_set: None,
        artifact_ids,
        revision_ids,
    }
}

fn envelope_at(
    lifecycle_state: RunLifecycleState,
    input_class: InputClass,
    input_identity: &InputIdentityReference,
) -> RunEnvelope {
    RunEnvelope {
        schema_revision: RUN_ENVELOPE_SCHEMA.to_string(),
        run_id: RunId::new(RUN_ID).expect("run id"),
        input_identity: input_identity.clone(),
        calibration_validity: CalibrationValidityMode::BlindReference,
        workflow_observation: WorkflowObservationMode::Disabled,
        input_class,
        qualifies_as_real_material_evidence: true,
        lifecycle_state,
        expected_artifact_roles: canonical_real_evaluation_artifact_roles(),
    }
}

fn input_authorization_for(
    input_class: InputClass,
    input_identity: &InputIdentityReference,
) -> InputAuthorization {
    let (basis, authorization_id) = match input_class {
        InputClass::SelfOwnedReal => (InputAuthorizationBasis::SelfOwned, "auth-self-owned-adapt"),
        InputClass::ExplicitPermissionReal => (
            InputAuthorizationBasis::ExplicitPermission,
            "auth-explicit-adapt",
        ),
        InputClass::SyntheticProtocolFixture => {
            (InputAuthorizationBasis::SelfOwned, "auth-synthetic-adapt")
        }
    };

    InputAuthorization {
        schema_revision: INPUT_AUTHORIZATION_SCHEMA.to_string(),
        authorization_id: InputAuthorizationId::new(authorization_id).expect("authorization id"),
        run_id: RunId::new(RUN_ID).expect("run id"),
        input_identity: input_identity.clone(),
        input_class,
        authorization_basis: basis,
        scope_policy_revision: INPUT_AUTHORIZATION_SCOPE_POLICY.to_string(),
        state: InputAuthorizationState::Confirmed,
    }
}

fn completion_record(
    cue_id: u32,
    segment_position: u32,
    disposition: ReferenceCueDisposition,
) -> CueReviewCompletionRecord {
    CueReviewCompletionRecord {
        cue_id: CueReferenceId::new(cue_id).expect("cue id"),
        segment_position,
        source_text_digest: CueSourceTextDigest::new(SAMPLE_CUE_DIGEST).expect("digest"),
        disposition,
        fully_reviewed: true,
        all_known_transcription_errors_enumerated: true,
        verification_source_used: VerificationBasis::AudioListened,
        reviewer_identity_class: ReferenceReviewerIdentityClass::OwnerBlindReviewer,
        completed_at_unix_ms: FROZEN_AT_MS,
    }
}

fn primary_coverage(input_identity: &InputIdentityReference, cue_count: u32) -> ReferenceCoverage {
    let records: Vec<CueReviewCompletionRecord> = (1..=cue_count)
        .map(|cue_id| {
            completion_record(
                cue_id,
                cue_id - 1,
                if cue_id == cue_count && cue_count > 1 {
                    ReferenceCueDisposition::TranscriptionError
                } else {
                    ReferenceCueDisposition::NoTranscriptionError
                },
            )
        })
        .collect();
    let cue_ids: Vec<u32> = records.iter().map(|record| record.cue_id.value()).collect();
    let expected = ExpectedCueUniverse {
        total_cues: cue_ids.len() as u32,
        cue_ids: cue_ids
            .iter()
            .map(|id| CueReferenceId::new(*id).expect("cue id"))
            .collect(),
    };
    let assessment =
        ReferenceCoverage::derive_assessment(&expected, &records).expect("derive assessment");

    let mut coverage = ReferenceCoverage {
        schema_revision: REFERENCE_COVERAGE_SCHEMA.to_string(),
        coverage_id: ReferenceCoverageId::new(COVERAGE_ID).expect("coverage id"),
        run_id: RunId::new(RUN_ID).expect("run id"),
        input_identity: input_identity.clone(),
        seal_id: ReferenceSealId::new(SEAL_ID).expect("seal id"),
        reference_revision: ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION)
            .expect("revision id"),
        coverage_purpose: ReferenceCoveragePurpose::PrimaryBlindCalibration,
        expected_universe: expected,
        records,
        coverage_state: ReferenceCoverageState::Draft,
        assessment,
    };

    let human_reference = human_reference_for_coverage(&coverage);
    coverage.assessment.total_eligible_transcription_errors = human_reference
        .assessment
        .recall_eligible_transcription_error_count;
    coverage.coverage_state = ReferenceCoverageState::Complete;
    coverage
}

fn human_reference_for_coverage(coverage: &ReferenceCoverage) -> HumanFinalReference {
    let mut te_records = Vec::new();
    for completion in &coverage.records {
        if completion.disposition != ReferenceCueDisposition::TranscriptionError {
            continue;
        }
        te_records.push(ReferenceErrorRecord {
            reference_error_id: ReferenceErrorId::new(format!(
                "ref-err-{}",
                completion.cue_id.value()
            ))
            .expect("error id"),
            reference_revision: coverage.reference_revision.clone(),
            input_identity: coverage.input_identity.clone(),
            source_anchor: ReferenceSourceAnchor {
                input_identity: coverage.input_identity.clone(),
                cue_id: completion.cue_id,
                segment_position: completion.segment_position,
                start_byte: 0,
                end_byte: 4,
            },
            original_surface: "wrng".to_string(),
            human_final_surface: "wrong".to_string(),
            reference_class: ReferenceClass::TranscriptionError,
            verification_basis: VerificationBasis::AudioListened,
            reviewer_identity_class: ReferenceReviewerIdentityClass::OwnerBlindReviewer,
            reviewed_at_unix_ms: FROZEN_AT_MS,
        });
    }

    let assessment = HumanFinalReference::derive_assessment(
        &coverage.reference_revision,
        &coverage.input_identity,
        &te_records,
    )
    .expect("derive assessment");

    HumanFinalReference {
        schema_revision: HUMAN_FINAL_REFERENCE_SCHEMA.to_string(),
        run_id: coverage.run_id.clone(),
        input_identity: coverage.input_identity.clone(),
        seal_id: coverage.seal_id.clone(),
        reference_revision: coverage.reference_revision.clone(),
        records: te_records,
        state: HumanFinalReferenceState::Sealed,
        assessment,
    }
}

fn blind_seal(input_identity: &InputIdentityReference) -> ReferenceSeal {
    ReferenceSeal {
        schema_revision: REFERENCE_SEAL_SCHEMA.to_string(),
        seal_id: ReferenceSealId::new(SEAL_ID).expect("seal id"),
        run_id: RunId::new(RUN_ID).expect("run id"),
        input_identity: input_identity.clone(),
        reference_revision: ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION)
            .expect("revision id"),
        producer_class: ReferenceProducerClass::HumanBlindReviewer,
        reference_created_before_detector_run: true,
        prior_detector_run_on_same_input: false,
        prior_knowledge_of_detector_targets: false,
        session_terms_visible_during_reference: false,
        external_notes_encode_detector_targets: false,
        seal_state: ReferenceSealState::Sealed,
        calibration_classification: ReferenceCalibrationValidity::BlindReferenceEligible,
        calibration_validity_impact: CalibrationValidityImpact::None,
    }
}

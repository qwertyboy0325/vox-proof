#![allow(dead_code)]

use vox_proof::artifact_bundle::{ArtifactBundleId, ArtifactId};
use vox_proof::candidate::DetectionKind;
use vox_proof::detector_reference_join::{
    DetectorReferenceJoinContext, DetectorReferenceJoinId, DetectorReferenceJoinRevisionId,
};
use vox_proof::detector_snapshot::{
    DETECTOR_PROPOSAL_SNAPSHOT_SCHEMA, DetectorAnalysisIdentity, DetectorComponentIdentity,
    DetectorProposalAlternative, DetectorProposalEvidence, DetectorProposalId,
    DetectorProposalRecord, DetectorProposalSemanticKey, DetectorProposalSnapshot,
    DetectorProposalSnapshotState, DetectorProposalSourceAnchor, DetectorSessionTermEntry,
    DetectorSnapshotRevisionId,
};
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
    OVERLAP_ADJUDICATION_SCHEMA, OverlapAdjudicationId, OverlapAdjudicationRecord,
    OverlapAdjudicationResult, OverlapAdjudicationSet, OverlapAdjudicationSetId,
    OverlapAdjudicationSetState, OverlapAdjudicatorRole,
};
use vox_proof::join_metric_aggregation::{
    JoinMetricAggregateContext, MetricAggregateRevisionId, MetricAggregateSetId,
};
use vox_proof::join_metric_contribution::{
    JoinMetricContributionContext, MetricContributionRevisionId, MetricContributionSetId,
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

pub const SAMPLE_REVISION: &str =
    "rev:sha256-v1:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
pub const SAMPLE_REFERENCE_REVISION: &str = "ref-rev-real-exec-001";
pub const SAMPLE_CUE_DIGEST: &str =
    "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
pub const SAMPLE_SESSION_TERMS: &str =
    "session-terms:sha256-v1:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
pub const JOIN_CONTRACT_REVISION: &str = "voxproof-detector-reference-join-v1";
pub const RUN_ID: &str = "run-real-exec-001";
pub const TIMESTAMP_MS: u64 = 1_700_000_000_000;

pub struct RealExecutionFixture {
    pub request: RealTranscriptEvaluationRunRequest,
    pub input: RealTranscriptEvaluationExecutionInput,
}

pub fn exact_only_self_owned_fixture() -> RealExecutionFixture {
    build_fixture(
        InputClass::SelfOwnedReal,
        "auth-self-owned-exec-001",
        InputAuthorizationBasis::SelfOwned,
        vec![
            record(1, ReferenceCueDisposition::TranscriptionError),
            record(2, ReferenceCueDisposition::TranscriptionError),
            record(3, ReferenceCueDisposition::TranscriptionError),
            record(4, ReferenceCueDisposition::TranscriptionError),
            record(5, ReferenceCueDisposition::TranscriptionError),
            record(6, ReferenceCueDisposition::TranscriptionError),
        ],
        vec![
            reference_error_record("ref-err-exact", 1, 0, 0, 4, "wrong"),
            reference_error_record("ref-err-wrong", 2, 1, 0, 4, "wrong"),
            reference_error_record("ref-err-dup", 3, 2, 0, 4, "wrong"),
            reference_error_record("ref-err-unmatched", 4, 3, 8, 12, "wrong"),
            ReferenceErrorRecord {
                reference_error_id: ReferenceErrorId::new("ref-err-excluded").expect("error id"),
                reference_revision: ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION)
                    .expect("revision"),
                input_identity: input_identity(),
                source_anchor: reference_source_anchor(5, 4, 0, 4),
                original_surface: "wrng".to_string(),
                human_final_surface: "wrong".to_string(),
                reference_class: ReferenceClass::TranscriptionError,
                verification_basis: VerificationBasis::TranscriptContextOnly,
                reviewer_identity_class: ReferenceReviewerIdentityClass::OwnerBlindReviewer,
                reviewed_at_unix_ms: TIMESTAMP_MS,
            },
            reference_error_record("ref-err-extra", 6, 5, 0, 4, "wrong"),
        ],
        vec![
            glossary_proposal("det-prop-exact", 1, 0, 0, 4, "wrng", "wrong"),
            glossary_proposal("det-prop-wrong", 2, 1, 0, 4, "wrng", "wright"),
            observed_error_proposal("det-prop-dup-b", 3, 2, 0, 4, "wrng", "wrong"),
            glossary_proposal("det-prop-dup-a", 3, 2, 0, 4, "wrng", "wrong"),
            glossary_proposal("det-prop-unmatched", 5, 4, 0, 4, "wrng", "wrong"),
        ],
        None,
    )
}

pub fn overlap_explicit_permission_fixture(
    assisted: Option<Vec<OverlapAdjudicationRecord>>,
) -> RealExecutionFixture {
    build_fixture(
        InputClass::ExplicitPermissionReal,
        "auth-explicit-perm-exec-001",
        InputAuthorizationBasis::ExplicitPermission,
        vec![record(1, ReferenceCueDisposition::TranscriptionError)],
        vec![reference_error_record(
            "ref-err-overlap",
            1,
            0,
            0,
            4,
            "wrong",
        )],
        vec![glossary_proposal(
            "det-prop-overlap",
            1,
            0,
            2,
            6,
            "wrng",
            "wrong",
        )],
        assisted.map(|records| adjudication_set("adj-set-assisted-real", records)),
    )
}

pub fn overlap_assisted_record(role: OverlapAdjudicatorRole) -> OverlapAdjudicationRecord {
    adjudication_record(
        "adj-overlap-real-001",
        "det-prop-overlap",
        "ref-err-overlap",
        OverlapAdjudicationResult::SameErrorSameCorrection,
        role,
    )
}

pub fn dual_overlap_assisted_record(role: OverlapAdjudicatorRole) -> OverlapAdjudicationRecord {
    adjudication_record(
        "adj-overlap-a-real-001",
        "det-prop-overlap-a",
        "ref-err-overlap-a",
        OverlapAdjudicationResult::SameErrorSameCorrection,
        role,
    )
}

pub fn dual_overlap_explicit_permission_fixture(
    assisted: Option<Vec<OverlapAdjudicationRecord>>,
) -> RealExecutionFixture {
    build_fixture(
        InputClass::ExplicitPermissionReal,
        "auth-explicit-perm-exec-001",
        InputAuthorizationBasis::ExplicitPermission,
        vec![
            record(1, ReferenceCueDisposition::TranscriptionError),
            record(2, ReferenceCueDisposition::TranscriptionError),
        ],
        vec![
            reference_error_record("ref-err-overlap-a", 1, 0, 0, 4, "wrong"),
            reference_error_record("ref-err-overlap-b", 2, 1, 0, 4, "wrong"),
        ],
        vec![
            glossary_proposal("det-prop-overlap-a", 1, 0, 2, 6, "wrng", "wrong"),
            glossary_proposal("det-prop-overlap-b", 2, 1, 2, 6, "wrng", "wrong"),
        ],
        assisted.map(|records| adjudication_set("adj-set-assisted-dual", records)),
    )
}

pub fn zero_population_fixture() -> RealExecutionFixture {
    build_fixture(
        InputClass::SelfOwnedReal,
        "auth-self-owned-exec-001",
        InputAuthorizationBasis::SelfOwned,
        vec![record(1, ReferenceCueDisposition::NoTranscriptionError)],
        Vec::new(),
        Vec::new(),
        None,
    )
}

fn build_fixture(
    input_class: InputClass,
    authorization_id: &str,
    basis: InputAuthorizationBasis,
    coverage_records: Vec<CueReviewCompletionRecord>,
    reference_records: Vec<ReferenceErrorRecord>,
    proposals: Vec<DetectorProposalRecord>,
    assisted_adjudication: Option<OverlapAdjudicationSet>,
) -> RealExecutionFixture {
    let request = valid_request(
        input_class,
        authorization_id,
        basis,
        &coverage_records,
        &reference_records,
    );
    let detector_snapshot = frozen_snapshot(proposals);
    let detector_execution_adjudication_set = empty_adjudication_set("adj-set-detector-empty-real");
    let input = RealTranscriptEvaluationExecutionInput {
        detector_snapshot,
        detector_execution_adjudication_set,
        assisted_review_adjudication_set: assisted_adjudication,
        artifact_ids: artifact_ids(),
        revision_ids: revision_ids(),
    };
    RealExecutionFixture { request, input }
}

fn valid_request(
    input_class: InputClass,
    authorization_id: &str,
    basis: InputAuthorizationBasis,
    coverage_records: &[CueReviewCompletionRecord],
    reference_records: &[ReferenceErrorRecord],
) -> RealTranscriptEvaluationRunRequest {
    let seal = blind_seal();
    let mut coverage = primary_coverage(coverage_records);
    let human_reference = human_reference_for_records(&coverage, reference_records);
    coverage.assessment.total_eligible_transcription_errors = human_reference
        .assessment
        .recall_eligible_transcription_error_count;
    coverage.coverage_state = ReferenceCoverageState::Complete;

    RealTranscriptEvaluationRunRequest {
        schema_revision: REAL_TRANSCRIPT_EVALUATION_RUNNER_REQUEST_SCHEMA.to_string(),
        runner_policy_revision: REAL_TRANSCRIPT_EVALUATION_RUNNER_POLICY.to_string(),
        overlap_authority_policy_revision: REAL_TRANSCRIPT_OVERLAP_AUTHORITY_POLICY.to_string(),
        input_authorization: input_authorization_for(input_class, authorization_id, basis),
        declared_envelope: envelope_at(RunLifecycleState::Declared, input_class),
        reference_preparation_envelope: envelope_at(
            RunLifecycleState::ReferencePreparation,
            input_class,
        ),
        reference_sealed_envelope: envelope_at(RunLifecycleState::ReferenceSealed, input_class),
        detector_execution_envelope: envelope_at(RunLifecycleState::DetectorExecution, input_class),
        assisted_review_transition_envelope: envelope_at(
            RunLifecycleState::AssistedReview,
            input_class,
        ),
        finalized_envelope: envelope_at(RunLifecycleState::Finalized, input_class),
        reference_seal: seal,
        reference_coverage: coverage,
        human_final_reference: human_reference,
        detector_analysis_identity: analysis_identity(),
        expected_artifact_roles: canonical_real_evaluation_artifact_roles(),
    }
}

pub fn artifact_ids() -> RealTranscriptEvaluationArtifactIds {
    RealTranscriptEvaluationArtifactIds {
        input_authorization: ArtifactId::new("artifact-input-authorization-real").expect("id"),
        reference_seal: ArtifactId::new("artifact-reference-seal-real").expect("id"),
        human_final_reference: ArtifactId::new("artifact-human-reference-real").expect("id"),
        cue_review_completion: ArtifactId::new("artifact-cue-coverage-real").expect("id"),
        detector_output: ArtifactId::new("artifact-detector-output-real").expect("id"),
        evaluation_join: ArtifactId::new("artifact-evaluation-join-real").expect("id"),
        join_adjudication: ArtifactId::new("artifact-join-adjudication-real").expect("id"),
        metric_contributions: ArtifactId::new("artifact-metric-contributions-real").expect("id"),
        metrics: ArtifactId::new("artifact-metrics-real").expect("id"),
        bundle: ArtifactBundleId::new("bundle-real-exec-001").expect("id"),
    }
}

pub fn revision_ids() -> RealTranscriptEvaluationRevisionIds {
    RealTranscriptEvaluationRevisionIds {
        join_context: DetectorReferenceJoinContext {
            join_id: DetectorReferenceJoinId::new("join-real-exec-001").expect("join id"),
            join_revision: DetectorReferenceJoinRevisionId::new("join-rev-real-exec-001")
                .expect("join revision"),
            evaluation_join_artifact_id: ArtifactId::new("artifact-evaluation-join-real")
                .expect("artifact id"),
            join_adjudication_artifact_id: ArtifactId::new("artifact-join-adjudication-real")
                .expect("artifact id"),
        },
        contribution_context: JoinMetricContributionContext {
            contribution_set_id: MetricContributionSetId::new("metric-contrib-set-real-001")
                .expect("contribution set id"),
            contribution_revision: MetricContributionRevisionId::new("metric-contrib-rev-real-001")
                .expect("contribution revision"),
            metric_contributions_artifact_id: ArtifactId::new("artifact-metric-contributions-real")
                .expect("artifact id"),
        },
        aggregate_context: JoinMetricAggregateContext {
            aggregate_set_id: MetricAggregateSetId::new("metric-aggregate-set-real-001")
                .expect("aggregate set id"),
            aggregate_revision: MetricAggregateRevisionId::new("metric-aggregate-rev-real-001")
                .expect("aggregate revision"),
            metrics_artifact_id: ArtifactId::new("artifact-metrics-real").expect("artifact id"),
        },
    }
}

fn input_identity() -> InputIdentityReference {
    InputIdentityReference {
        transcript_revision_id: SAMPLE_REVISION.to_string(),
    }
}

fn envelope_at(lifecycle_state: RunLifecycleState, input_class: InputClass) -> RunEnvelope {
    RunEnvelope {
        schema_revision: RUN_ENVELOPE_SCHEMA.to_string(),
        run_id: RunId::new(RUN_ID).expect("run id"),
        input_identity: input_identity(),
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
    authorization_id: &str,
    basis: InputAuthorizationBasis,
) -> InputAuthorization {
    InputAuthorization {
        schema_revision: INPUT_AUTHORIZATION_SCHEMA.to_string(),
        authorization_id: InputAuthorizationId::new(authorization_id).expect("authorization id"),
        run_id: RunId::new(RUN_ID).expect("run id"),
        input_identity: input_identity(),
        input_class,
        authorization_basis: basis,
        scope_policy_revision: INPUT_AUTHORIZATION_SCOPE_POLICY.to_string(),
        state: InputAuthorizationState::Confirmed,
    }
}

fn blind_seal() -> ReferenceSeal {
    ReferenceSeal {
        schema_revision: REFERENCE_SEAL_SCHEMA.to_string(),
        seal_id: ReferenceSealId::new("seal-real-exec-001").expect("seal id"),
        run_id: RunId::new(RUN_ID).expect("run id"),
        input_identity: input_identity(),
        reference_revision: ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION).expect("revision"),
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

fn record(cue_id: u32, disposition: ReferenceCueDisposition) -> CueReviewCompletionRecord {
    CueReviewCompletionRecord {
        cue_id: CueReferenceId::new(cue_id).expect("cue id"),
        segment_position: cue_id - 1,
        source_text_digest: CueSourceTextDigest::new(SAMPLE_CUE_DIGEST).expect("digest"),
        disposition,
        fully_reviewed: true,
        all_known_transcription_errors_enumerated: true,
        verification_source_used: VerificationBasis::AudioListened,
        reviewer_identity_class: ReferenceReviewerIdentityClass::OwnerBlindReviewer,
        completed_at_unix_ms: TIMESTAMP_MS,
    }
}

fn primary_coverage(records: &[CueReviewCompletionRecord]) -> ReferenceCoverage {
    let cue_ids: Vec<u32> = records.iter().map(|record| record.cue_id.value()).collect();
    let expected = ExpectedCueUniverse {
        total_cues: cue_ids.len() as u32,
        cue_ids: cue_ids
            .iter()
            .map(|id| CueReferenceId::new(*id).expect("cue id"))
            .collect(),
    };
    let assessment =
        ReferenceCoverage::derive_assessment(&expected, records).expect("derive assessment");
    ReferenceCoverage {
        schema_revision: REFERENCE_COVERAGE_SCHEMA.to_string(),
        coverage_id: ReferenceCoverageId::new("coverage-real-exec-001").expect("coverage id"),
        run_id: RunId::new(RUN_ID).expect("run id"),
        input_identity: input_identity(),
        seal_id: ReferenceSealId::new("seal-real-exec-001").expect("seal id"),
        reference_revision: ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION).expect("revision"),
        coverage_purpose: ReferenceCoveragePurpose::PrimaryBlindCalibration,
        expected_universe: expected,
        records: records.to_vec(),
        coverage_state: ReferenceCoverageState::Complete,
        assessment,
    }
}

fn human_reference_for_records(
    coverage: &ReferenceCoverage,
    records: &[ReferenceErrorRecord],
) -> HumanFinalReference {
    let assessment = HumanFinalReference::derive_assessment(
        &coverage.reference_revision,
        &coverage.input_identity,
        records,
    )
    .expect("derive assessment");
    HumanFinalReference {
        schema_revision: HUMAN_FINAL_REFERENCE_SCHEMA.to_string(),
        run_id: coverage.run_id.clone(),
        input_identity: coverage.input_identity.clone(),
        seal_id: coverage.seal_id.clone(),
        reference_revision: coverage.reference_revision.clone(),
        records: records.to_vec(),
        state: HumanFinalReferenceState::Sealed,
        assessment,
    }
}

fn frozen_snapshot(proposals: Vec<DetectorProposalRecord>) -> DetectorProposalSnapshot {
    let snapshot_revision =
        DetectorSnapshotRevisionId::new("det-snap-rev-real-exec-001").expect("snapshot revision");
    let assessment = DetectorProposalSnapshot::derive_assessment(
        &snapshot_revision,
        &input_identity(),
        &analysis_identity(),
        &proposals,
    )
    .expect("derive assessment");
    DetectorProposalSnapshot {
        schema_revision: DETECTOR_PROPOSAL_SNAPSHOT_SCHEMA.to_string(),
        run_id: RunId::new(RUN_ID).expect("run id"),
        input_identity: input_identity(),
        calibration_validity: CalibrationValidityMode::BlindReference,
        snapshot_revision,
        detector_output_artifact_id: ArtifactId::new("artifact-detector-output-real")
            .expect("artifact id"),
        analysis_identity: analysis_identity(),
        proposals,
        frozen_at_unix_ms: TIMESTAMP_MS,
        state: DetectorProposalSnapshotState::Frozen,
        assessment,
    }
}

fn empty_adjudication_set(set_id: &str) -> OverlapAdjudicationSet {
    adjudication_set(set_id, Vec::new())
}

fn adjudication_set(
    set_id: &str,
    records: Vec<OverlapAdjudicationRecord>,
) -> OverlapAdjudicationSet {
    let assessment = OverlapAdjudicationSet::derive_assessment(&records);
    OverlapAdjudicationSet {
        schema_revision: OVERLAP_ADJUDICATION_SCHEMA.to_string(),
        adjudication_set_id: OverlapAdjudicationSetId::new(set_id).expect("set id"),
        run_id: RunId::new(RUN_ID).expect("run id"),
        input_identity: input_identity(),
        reference_revision: ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION).expect("revision"),
        detector_snapshot_revision: DetectorSnapshotRevisionId::new("det-snap-rev-real-exec-001")
            .expect("snapshot revision"),
        join_contract_revision: JOIN_CONTRACT_REVISION.to_string(),
        overlap_rule_revision: "voxproof-overlap-v1".to_string(),
        join_adjudication_artifact_id: ArtifactId::new("artifact-join-adjudication-real")
            .expect("artifact id"),
        state: OverlapAdjudicationSetState::Frozen,
        records,
        assessment,
    }
}

fn reference_source_anchor(
    cue_id: u32,
    segment_position: u32,
    start: u32,
    end: u32,
) -> ReferenceSourceAnchor {
    ReferenceSourceAnchor {
        input_identity: input_identity(),
        cue_id: CueReferenceId::new(cue_id).expect("cue id"),
        segment_position,
        start_byte: start,
        end_byte: end,
    }
}

fn reference_error_record(
    error_id: &str,
    cue_id: u32,
    segment_position: u32,
    start: u32,
    end: u32,
    human_final_surface: &str,
) -> ReferenceErrorRecord {
    ReferenceErrorRecord {
        reference_error_id: ReferenceErrorId::new(error_id).expect("error id"),
        reference_revision: ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION).expect("revision"),
        input_identity: input_identity(),
        source_anchor: reference_source_anchor(cue_id, segment_position, start, end),
        original_surface: "wrng".to_string(),
        human_final_surface: human_final_surface.to_string(),
        reference_class: ReferenceClass::TranscriptionError,
        verification_basis: VerificationBasis::AudioListened,
        reviewer_identity_class: ReferenceReviewerIdentityClass::OwnerBlindReviewer,
        reviewed_at_unix_ms: TIMESTAMP_MS,
    }
}

fn source_anchor(
    cue_id: u32,
    segment_position: u32,
    start: u32,
    end: u32,
) -> DetectorProposalSourceAnchor {
    DetectorProposalSourceAnchor {
        input_identity: input_identity(),
        cue_id: CueReferenceId::new(cue_id).expect("cue id"),
        segment_position,
        start_byte: start,
        end_byte: end,
    }
}

fn analysis_identity() -> DetectorAnalysisIdentity {
    DetectorAnalysisIdentity {
        input_identity: input_identity(),
        session_terms_identity: SAMPLE_SESSION_TERMS.to_string(),
        detector_set: vec![
            DetectorComponentIdentity {
                id: "glossary-alias-match".to_string(),
                version: "0.1.0".to_string(),
            },
            DetectorComponentIdentity {
                id: "observed-error-form-match".to_string(),
                version: "0.1.0".to_string(),
            },
        ],
        detector_config: DetectorComponentIdentity {
            id: "detector-config".to_string(),
            version: "0.1.0".to_string(),
        },
        algorithm: DetectorComponentIdentity {
            id: "algorithm-v1".to_string(),
            version: "0.1.0".to_string(),
        },
    }
}

fn glossary_proposal(
    proposal_id: &str,
    cue_id: u32,
    segment_position: u32,
    start: u32,
    end: u32,
    observed: &str,
    correction: &str,
) -> DetectorProposalRecord {
    let detector = DetectorComponentIdentity {
        id: "glossary-alias-match".to_string(),
        version: "0.1.0".to_string(),
    };
    let anchor = source_anchor(cue_id, segment_position, start, end);
    let mut record = DetectorProposalRecord {
        detector_proposal_id: DetectorProposalId::new(proposal_id).expect("proposal id"),
        snapshot_revision: DetectorSnapshotRevisionId::new("det-snap-rev-real-exec-001")
            .expect("snapshot revision"),
        input_identity: input_identity(),
        semantic_key: DetectorProposalSemanticKey {
            detector_id: detector.id.clone(),
            detection_kind: DetectionKind::GlossaryAliasMatch,
            source_anchor: anchor.clone(),
        },
        detector: detector.clone(),
        source_anchor: anchor,
        observed_surface: observed.to_string(),
        detection_kind: DetectionKind::GlossaryAliasMatch,
        evidence: DetectorProposalEvidence::GlossaryAlias {
            entry: DetectorSessionTermEntry {
                canonical_term: "widget".to_string(),
                aliases: vec!["wijet".to_string()],
                observed_error_forms: vec!["widgit".to_string()],
            },
            matched_form: observed.to_string(),
        },
        alternatives: vec![DetectorProposalAlternative {
            alternative_index: 0,
            replacement_surface: correction.to_string(),
        }],
    };
    record.semantic_key = record.derive_semantic_key();
    record
}

fn observed_error_proposal(
    proposal_id: &str,
    cue_id: u32,
    segment_position: u32,
    start: u32,
    end: u32,
    observed: &str,
    correction: &str,
) -> DetectorProposalRecord {
    let detector = DetectorComponentIdentity {
        id: "observed-error-form-match".to_string(),
        version: "0.1.0".to_string(),
    };
    let anchor = source_anchor(cue_id, segment_position, start, end);
    let mut record = DetectorProposalRecord {
        detector_proposal_id: DetectorProposalId::new(proposal_id).expect("proposal id"),
        snapshot_revision: DetectorSnapshotRevisionId::new("det-snap-rev-real-exec-001")
            .expect("snapshot revision"),
        input_identity: input_identity(),
        semantic_key: DetectorProposalSemanticKey {
            detector_id: detector.id.clone(),
            detection_kind: DetectionKind::GlossaryAliasMatch,
            source_anchor: anchor.clone(),
        },
        detector: detector.clone(),
        source_anchor: anchor,
        observed_surface: observed.to_string(),
        detection_kind: DetectionKind::GlossaryAliasMatch,
        evidence: DetectorProposalEvidence::ObservedErrorForm {
            entry: DetectorSessionTermEntry {
                canonical_term: "widget".to_string(),
                aliases: vec!["wijet".to_string()],
                observed_error_forms: vec!["widgit".to_string()],
            },
            matched_form: observed.to_string(),
        },
        alternatives: vec![DetectorProposalAlternative {
            alternative_index: 0,
            replacement_surface: correction.to_string(),
        }],
    };
    record.semantic_key = record.derive_semantic_key();
    record
}

fn adjudication_record(
    adjudication_id: &str,
    proposal_id: &str,
    reference_error_id: &str,
    result: OverlapAdjudicationResult,
    role: OverlapAdjudicatorRole,
) -> OverlapAdjudicationRecord {
    OverlapAdjudicationRecord {
        adjudication_id: OverlapAdjudicationId::new(adjudication_id).expect("adjudication id"),
        detector_proposal_id: DetectorProposalId::new(proposal_id).expect("proposal id"),
        reference_error_id: ReferenceErrorId::new(reference_error_id).expect("reference error id"),
        join_contract_revision: JOIN_CONTRACT_REVISION.to_string(),
        adjudicator_role: role,
        adjudication_result: result,
        adjudication_reason: "artificial real-posture overlap adjudication".to_string(),
        adjudicated_at_unix_ms: TIMESTAMP_MS,
    }
}

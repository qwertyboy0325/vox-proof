#![allow(clippy::too_many_arguments, dead_code)]

use vox_proof::artifact_bundle::{
    ARTIFACT_BUNDLE_SCHEMA, ArtifactBindingContext, ArtifactBundle, ArtifactBundleId,
    ArtifactBundleState, ArtifactContentDigest, ArtifactDescriptor, ArtifactId,
    ArtifactSchemaIdentity,
};
use vox_proof::candidate::DetectionKind;
use vox_proof::detector_reference_join::{
    DetectorReferenceJoin, DetectorReferenceJoinContext, DetectorReferenceJoinId,
    DetectorReferenceJoinPurpose, DetectorReferenceJoinRevisionId,
    DetectorReferenceMatchDisposition, ReferenceJoinEligibility,
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
    ReferenceErrorId, ReferenceErrorRecord, ReferenceSourceAnchor,
};
use vox_proof::join_adjudication::{
    OVERLAP_ADJUDICATION_SCHEMA, OverlapAdjudicationId, OverlapAdjudicationRecord,
    OverlapAdjudicationResult, OverlapAdjudicationSet, OverlapAdjudicationSetId,
    OverlapAdjudicationSetState, OverlapAdjudicatorRole,
};
use vox_proof::join_metric_aggregation::{
    JOIN_METRIC_AGGREGATION_SCHEMA, JoinMetricAggregateContext, JoinMetricAggregateSet,
    JoinMetricAggregationError, MetricAggregateRecord,
    MetricAggregateRevisionId, MetricAggregateSetId,
    MetricAggregateSetState, MetricAggregateValueState, PRIMARY_METRIC_AGGREGATION_POLICY,
    ZERO_DENOMINATOR_POLICY, aggregate_from_json, aggregate_to_json, validate_aggregate_id_value,
};
use vox_proof::join_metric_contribution::{
    JoinMetricContributionContext, JoinMetricContributionError, JoinMetricContributionSet,
    MetricContributionExclusionReason, MetricContributionReportClass, MetricContributionRevisionId,
    MetricContributionSetId, MetricContributionSetState, PrimaryMetricBlockingReason,
    PrimaryMetricKind, RatioContribution,
};
use vox_proof::reference_coverage::{
    CueReferenceId, CueReviewCompletionRecord, ExpectedCueUniverse, REFERENCE_COVERAGE_SCHEMA,
    ReferenceCoverage, ReferenceCoverageId, ReferenceCoveragePurpose, ReferenceCoverageState,
    ReferenceCueDisposition,
};
use vox_proof::reference_identity::{
    CueSourceTextDigest, ReferenceReviewerIdentityClass, ReferenceRevisionId, VerificationBasis,
};
use vox_proof::reference_seal::{
    CalibrationValidityImpact, REFERENCE_SEAL_SCHEMA, ReferenceCalibrationValidity,
    ReferenceProducerClass, ReferenceSeal, ReferenceSealId, ReferenceSealState,
};
use vox_proof::run_manifest::{
    ArtifactRole, CalibrationValidityMode, InputClass, InputIdentityReference, RUN_ENVELOPE_SCHEMA,
    RunEnvelope, RunId, RunLifecycleState, WorkflowObservationMode,
};

const SAMPLE_REVISION: &str =
    "rev:sha256-v1:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const SAMPLE_REFERENCE_REVISION: &str = "ref-rev-001";
const SAMPLE_DIGEST: &str =
    "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const SAMPLE_SESSION_TERMS: &str =
    "session-terms:sha256-v1:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const JOIN_CONTRACT_REVISION: &str = "voxproof-detector-reference-join-v1";

fn input_identity() -> InputIdentityReference {
    InputIdentityReference {
        transcript_revision_id: SAMPLE_REVISION.to_string(),
    }
}

fn snapshot_revision_id() -> DetectorSnapshotRevisionId {
    DetectorSnapshotRevisionId::new("det-snap-rev-001").expect("snapshot revision")
}

fn detector_output_artifact_id() -> ArtifactId {
    ArtifactId::new("artifact-detector-output").expect("artifact id")
}

fn evaluation_join_artifact_id() -> ArtifactId {
    ArtifactId::new("artifact-evaluation-join").expect("artifact id")
}

fn join_adjudication_artifact_id() -> ArtifactId {
    ArtifactId::new("artifact-join-adjudication").expect("artifact id")
}

fn metric_contributions_artifact_id() -> ArtifactId {
    ArtifactId::new("artifact-metric-contributions").expect("artifact id")
}

fn metrics_artifact_id() -> ArtifactId {
    ArtifactId::new("artifact-metrics").expect("artifact id")
}

fn aggregate_context() -> JoinMetricAggregateContext {
    JoinMetricAggregateContext {
        aggregate_set_id: MetricAggregateSetId::new("metric-aggregate-set-001")
            .expect("aggregate set id"),
        aggregate_revision: MetricAggregateRevisionId::new("metric-aggregate-rev-001")
            .expect("aggregate revision"),
        metrics_artifact_id: metrics_artifact_id(),
    }
}

fn build_aggregation_bundle(
    context: ArtifactBindingContext,
    bundle_state: ArtifactBundleState,
) -> ArtifactBundle {
    let expected_roles = vec![
        ArtifactRole::ReferenceSeal,
        ArtifactRole::HumanFinalReference,
        ArtifactRole::CueReviewCompletion,
        ArtifactRole::DetectorOutput,
        ArtifactRole::EvaluationJoin,
        ArtifactRole::JoinAdjudication,
        ArtifactRole::MetricContributions,
        ArtifactRole::Metrics,
    ];
    let artifacts = vec![
        descriptor(&context, ArtifactRole::ReferenceSeal, "artifact-seal"),
        descriptor(
            &context,
            ArtifactRole::HumanFinalReference,
            "artifact-human-reference",
        ),
        descriptor(
            &context,
            ArtifactRole::CueReviewCompletion,
            "artifact-coverage",
        ),
        descriptor(
            &context,
            ArtifactRole::DetectorOutput,
            detector_output_artifact_id().as_str(),
        ),
        descriptor(
            &context,
            ArtifactRole::EvaluationJoin,
            evaluation_join_artifact_id().as_str(),
        ),
        descriptor(
            &context,
            ArtifactRole::JoinAdjudication,
            join_adjudication_artifact_id().as_str(),
        ),
        descriptor(
            &context,
            ArtifactRole::MetricContributions,
            metric_contributions_artifact_id().as_str(),
        ),
        descriptor(
            &context,
            ArtifactRole::Metrics,
            metrics_artifact_id().as_str(),
        ),
    ];
    let assessment =
        ArtifactBundle::derive_assessment(&expected_roles, &artifacts, &context).expect("derive");

    ArtifactBundle {
        schema_revision: ARTIFACT_BUNDLE_SCHEMA.to_string(),
        bundle_id: ArtifactBundleId::new("bundle-metric-aggregate").expect("bundle id"),
        binding_context: context,
        expected_roles,
        artifacts,
        bundle_state,
        assessment,
    }
}

fn schema_identity(schema: &str) -> ArtifactSchemaIdentity {
    ArtifactSchemaIdentity::new(schema, "v1").expect("schema")
}

fn detector_component(id: &str, version: &str) -> DetectorComponentIdentity {
    DetectorComponentIdentity {
        id: id.to_string(),
        version: version.to_string(),
    }
}

fn analysis_identity() -> DetectorAnalysisIdentity {
    DetectorAnalysisIdentity {
        input_identity: input_identity(),
        session_terms_identity: SAMPLE_SESSION_TERMS.to_string(),
        detector_set: vec![
            detector_component("glossary-alias-match", "0.1.0"),
            detector_component("observed-error-form-match", "0.1.0"),
        ],
        detector_config: detector_component("detector-config", "0.1.0"),
        algorithm: detector_component("algorithm-v1", "0.1.0"),
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

fn universe(cue_ids: &[u32]) -> ExpectedCueUniverse {
    ExpectedCueUniverse {
        total_cues: cue_ids.len() as u32,
        cue_ids: cue_ids
            .iter()
            .map(|id| CueReferenceId::new(*id).expect("cue id"))
            .collect(),
    }
}

fn record(cue_id: u32, disposition: ReferenceCueDisposition) -> CueReviewCompletionRecord {
    CueReviewCompletionRecord {
        cue_id: CueReferenceId::new(cue_id).expect("cue id"),
        segment_position: cue_id - 1,
        source_text_digest: CueSourceTextDigest::new(SAMPLE_DIGEST).expect("digest"),
        disposition,
        fully_reviewed: true,
        all_known_transcription_errors_enumerated: true,
        verification_source_used: VerificationBasis::AudioListened,
        reviewer_identity_class: ReferenceReviewerIdentityClass::OwnerBlindReviewer,
        completed_at_unix_ms: 1_700_000_000_000,
    }
}

fn build_coverage(
    purpose: ReferenceCoveragePurpose,
    records: Vec<CueReviewCompletionRecord>,
    run_id: &str,
) -> ReferenceCoverage {
    let cue_ids: Vec<u32> = records.iter().map(|entry| entry.cue_id.value()).collect();
    let expected = universe(&cue_ids);
    let assessment =
        ReferenceCoverage::derive_assessment(&expected, &records).expect("derive assessment");

    ReferenceCoverage {
        schema_revision: REFERENCE_COVERAGE_SCHEMA.to_string(),
        coverage_id: ReferenceCoverageId::new("coverage-join").expect("coverage id"),
        run_id: RunId::new(run_id).expect("run id"),
        input_identity: input_identity(),
        seal_id: ReferenceSealId::new("seal-join").expect("seal id"),
        reference_revision: ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION)
            .expect("revision id"),
        coverage_purpose: purpose,
        expected_universe: expected,
        records,
        coverage_state: ReferenceCoverageState::Complete,
        assessment,
    }
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
            reviewed_at_unix_ms: 1_700_000_000_000,
        });
    }

    let assessment = HumanFinalReference::derive_assessment(
        &coverage.reference_revision,
        &coverage.input_identity,
        &te_records,
    )
    .expect("derive human reference assessment");

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
        reviewed_at_unix_ms: 1_700_000_000_000,
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
    let detector = detector_component("glossary-alias-match", "0.1.0");
    let anchor = source_anchor(cue_id, segment_position, start, end);
    let mut record = DetectorProposalRecord {
        detector_proposal_id: DetectorProposalId::new(proposal_id).expect("proposal id"),
        snapshot_revision: snapshot_revision_id(),
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
    let detector = detector_component("observed-error-form-match", "0.1.0");
    let anchor = source_anchor(cue_id, segment_position, start, end);
    let mut record = DetectorProposalRecord {
        detector_proposal_id: DetectorProposalId::new(proposal_id).expect("proposal id"),
        snapshot_revision: snapshot_revision_id(),
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

fn build_snapshot(
    proposals: Vec<DetectorProposalRecord>,
    state: DetectorProposalSnapshotState,
) -> DetectorProposalSnapshot {
    let assessment = DetectorProposalSnapshot::derive_assessment(
        &snapshot_revision_id(),
        &input_identity(),
        &analysis_identity(),
        &proposals,
    )
    .expect("derive assessment");

    DetectorProposalSnapshot {
        schema_revision: DETECTOR_PROPOSAL_SNAPSHOT_SCHEMA.to_string(),
        run_id: RunId::new("run-join").expect("run id"),
        input_identity: input_identity(),
        calibration_validity: CalibrationValidityMode::BlindReference,
        snapshot_revision: snapshot_revision_id(),
        detector_output_artifact_id: detector_output_artifact_id(),
        analysis_identity: analysis_identity(),
        proposals,
        frozen_at_unix_ms: if state == DetectorProposalSnapshotState::Frozen {
            1_700_000_000_000
        } else {
            0
        },
        state,
        assessment,
    }
}

fn join_envelope(lifecycle: RunLifecycleState) -> RunEnvelope {
    RunEnvelope {
        schema_revision: RUN_ENVELOPE_SCHEMA.to_string(),
        run_id: RunId::new("run-join").expect("run id"),
        input_identity: input_identity(),
        calibration_validity: CalibrationValidityMode::BlindReference,
        workflow_observation: WorkflowObservationMode::Disabled,
        input_class: InputClass::SelfOwnedReal,
        qualifies_as_real_material_evidence: false,
        lifecycle_state: lifecycle,
        expected_artifact_roles: vec![
            ArtifactRole::ReferenceSeal,
            ArtifactRole::HumanFinalReference,
            ArtifactRole::CueReviewCompletion,
            ArtifactRole::DetectorOutput,
            ArtifactRole::EvaluationJoin,
            ArtifactRole::JoinAdjudication,
            ArtifactRole::MetricContributions,
            ArtifactRole::Metrics,
        ],
    }
}

fn join_seal() -> ReferenceSeal {
    ReferenceSeal {
        schema_revision: REFERENCE_SEAL_SCHEMA.to_string(),
        seal_id: ReferenceSealId::new("seal-join").expect("seal id"),
        run_id: RunId::new("run-join").expect("run id"),
        input_identity: input_identity(),
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

fn join_context() -> DetectorReferenceJoinContext {
    DetectorReferenceJoinContext {
        join_id: DetectorReferenceJoinId::new("join-test").expect("join id"),
        join_revision: DetectorReferenceJoinRevisionId::new("join-rev-001").expect("join revision"),
        evaluation_join_artifact_id: evaluation_join_artifact_id(),
        join_adjudication_artifact_id: join_adjudication_artifact_id(),
    }
}

fn contribution_context() -> JoinMetricContributionContext {
    JoinMetricContributionContext {
        contribution_set_id: MetricContributionSetId::new("metric-contrib-set-001")
            .expect("contribution set id"),
        contribution_revision: MetricContributionRevisionId::new("metric-contrib-rev-001")
            .expect("contribution revision"),
        metric_contributions_artifact_id: metric_contributions_artifact_id(),
    }
}

fn descriptor(
    context: &ArtifactBindingContext,
    role: ArtifactRole,
    artifact_id: &str,
) -> ArtifactDescriptor {
    ArtifactDescriptor {
        artifact_id: ArtifactId::new(artifact_id).expect("artifact id"),
        role,
        payload_schema: schema_identity("voxproof-artifact-v1"),
        content_digest: ArtifactContentDigest::new(SAMPLE_DIGEST).expect("digest"),
        byte_length: 128,
        binding_context: context.clone(),
    }
}

fn build_contribution_bundle(
    context: ArtifactBindingContext,
    bundle_state: ArtifactBundleState,
) -> ArtifactBundle {
    let expected_roles = vec![
        ArtifactRole::ReferenceSeal,
        ArtifactRole::HumanFinalReference,
        ArtifactRole::CueReviewCompletion,
        ArtifactRole::DetectorOutput,
        ArtifactRole::EvaluationJoin,
        ArtifactRole::JoinAdjudication,
        ArtifactRole::MetricContributions,
    ];
    let artifacts = vec![
        descriptor(&context, ArtifactRole::ReferenceSeal, "artifact-seal"),
        descriptor(
            &context,
            ArtifactRole::HumanFinalReference,
            "artifact-human-reference",
        ),
        descriptor(
            &context,
            ArtifactRole::CueReviewCompletion,
            "artifact-coverage",
        ),
        descriptor(
            &context,
            ArtifactRole::DetectorOutput,
            detector_output_artifact_id().as_str(),
        ),
        descriptor(
            &context,
            ArtifactRole::EvaluationJoin,
            evaluation_join_artifact_id().as_str(),
        ),
        descriptor(
            &context,
            ArtifactRole::JoinAdjudication,
            join_adjudication_artifact_id().as_str(),
        ),
        descriptor(
            &context,
            ArtifactRole::MetricContributions,
            metric_contributions_artifact_id().as_str(),
        ),
    ];
    let assessment =
        ArtifactBundle::derive_assessment(&expected_roles, &artifacts, &context).expect("derive");

    ArtifactBundle {
        schema_revision: ARTIFACT_BUNDLE_SCHEMA.to_string(),
        bundle_id: ArtifactBundleId::new("bundle-metric-contrib").expect("bundle id"),
        binding_context: context,
        expected_roles,
        artifacts,
        bundle_state,
        assessment,
    }
}

fn frozen_adjudication_set(records: Vec<OverlapAdjudicationRecord>) -> OverlapAdjudicationSet {
    let assessment = OverlapAdjudicationSet::derive_assessment(&records);
    OverlapAdjudicationSet {
        schema_revision: OVERLAP_ADJUDICATION_SCHEMA.to_string(),
        adjudication_set_id: OverlapAdjudicationSetId::new("adj-set-join").expect("set id"),
        run_id: RunId::new("run-join").expect("run id"),
        input_identity: input_identity(),
        reference_revision: ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION)
            .expect("reference revision"),
        detector_snapshot_revision: snapshot_revision_id(),
        join_contract_revision: JOIN_CONTRACT_REVISION.to_string(),
        overlap_rule_revision: "voxproof-overlap-v1".to_string(),
        join_adjudication_artifact_id: join_adjudication_artifact_id(),
        state: OverlapAdjudicationSetState::Frozen,
        records,
        assessment,
    }
}

fn adjudication_record(
    adjudication_id: &str,
    proposal_id: &str,
    reference_error_id: &str,
    result: OverlapAdjudicationResult,
) -> OverlapAdjudicationRecord {
    OverlapAdjudicationRecord {
        adjudication_id: OverlapAdjudicationId::new(adjudication_id).expect("adjudication id"),
        detector_proposal_id: DetectorProposalId::new(proposal_id).expect("proposal id"),
        reference_error_id: ReferenceErrorId::new(reference_error_id).expect("reference error id"),
        join_contract_revision: JOIN_CONTRACT_REVISION.to_string(),
        adjudicator_role: OverlapAdjudicatorRole::OwnerAdjudicator,
        adjudication_result: result,
        adjudication_reason: "synthetic overlap adjudication".to_string(),
        adjudicated_at_unix_ms: 1_700_000_000_000,
    }
}

fn term_conditioned_seal(envelope: &RunEnvelope) -> ReferenceSeal {
    ReferenceSeal {
        schema_revision: REFERENCE_SEAL_SCHEMA.to_string(),
        seal_id: ReferenceSealId::new("seal-diagnostic").expect("seal id"),
        run_id: envelope.run_id.clone(),
        input_identity: envelope.input_identity.clone(),
        reference_revision: ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION)
            .expect("revision id"),
        producer_class: ReferenceProducerClass::HumanBlindReviewer,
        reference_created_before_detector_run: true,
        prior_detector_run_on_same_input: false,
        prior_knowledge_of_detector_targets: false,
        session_terms_visible_during_reference: true,
        external_notes_encode_detector_targets: false,
        seal_state: ReferenceSealState::Sealed,
        calibration_classification: ReferenceCalibrationValidity::TermConditionedDiagnostic,
        calibration_validity_impact: CalibrationValidityImpact::ExcludedFromPrimaryMetrics,
    }
}

fn detector_contaminated_seal(envelope: &RunEnvelope) -> ReferenceSeal {
    ReferenceSeal {
        schema_revision: REFERENCE_SEAL_SCHEMA.to_string(),
        seal_id: ReferenceSealId::new("seal-contaminated").expect("seal id"),
        run_id: envelope.run_id.clone(),
        input_identity: envelope.input_identity.clone(),
        reference_revision: ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION)
            .expect("revision id"),
        producer_class: ReferenceProducerClass::HumanBlindReviewer,
        reference_created_before_detector_run: true,
        prior_detector_run_on_same_input: true,
        prior_knowledge_of_detector_targets: false,
        session_terms_visible_during_reference: false,
        external_notes_encode_detector_targets: false,
        seal_state: ReferenceSealState::Sealed,
        calibration_classification: ReferenceCalibrationValidity::DetectorContaminated,
        calibration_validity_impact: CalibrationValidityImpact::ExcludedFromPrimaryMetrics,
    }
}

fn diagnostic_metric_stack(coverage_state: ReferenceCoverageState) -> PrimaryMetricStack {
    let envelope = join_envelope(RunLifecycleState::AssistedReview);
    let seal = term_conditioned_seal(&envelope);
    let mut coverage = build_coverage(
        ReferenceCoveragePurpose::DiagnosticOnly,
        vec![record(1, ReferenceCueDisposition::TranscriptionError)],
        "run-join",
    );
    coverage.seal_id = seal.seal_id.clone();
    coverage.coverage_state = coverage_state;
    if coverage_state == ReferenceCoverageState::Complete {
        coverage.assessment.coverage_complete = true;
        coverage.assessment.reference_resolved = true;
    }
    let human_reference = HumanFinalReference {
        schema_revision: HUMAN_FINAL_REFERENCE_SCHEMA.to_string(),
        run_id: coverage.run_id.clone(),
        input_identity: coverage.input_identity.clone(),
        seal_id: coverage.seal_id.clone(),
        reference_revision: coverage.reference_revision.clone(),
        records: vec![reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong")],
        state: HumanFinalReferenceState::Sealed,
        assessment: HumanFinalReference::derive_assessment(
            &coverage.reference_revision,
            &coverage.input_identity,
            &[reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong")],
        )
        .expect("derive assessment"),
    };
    coverage.assessment.total_eligible_transcription_errors = human_reference
        .assessment
        .recall_eligible_transcription_error_count;
    let snapshot = build_snapshot(
        vec![glossary_proposal(
            "det-prop-001",
            1,
            0,
            0,
            4,
            "wrng",
            "wrong",
        )],
        DetectorProposalSnapshotState::Frozen,
    );
    let binding_context = ArtifactBindingContext {
        run_id: envelope.run_id.clone(),
        input_identity: envelope.input_identity.clone(),
        calibration_validity: envelope.calibration_validity,
        reference_seal_id: Some(seal.seal_id.clone()),
        reference_coverage_id: Some(coverage.coverage_id.clone()),
        reference_revision: Some(seal.reference_revision.clone()),
    };
    let bundle = build_aggregation_bundle(binding_context, ArtifactBundleState::Complete);
    let adjudication = frozen_adjudication_set(vec![]);
    let join = DetectorReferenceJoin::derive(
        &join_context(),
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive diagnostic join");
    assert_eq!(
        join.join_purpose,
        DetectorReferenceJoinPurpose::DiagnosticOnly
    );
    (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contribution_context(),
        adjudication,
        join,
    )
}

fn contaminated_metric_stack() -> PrimaryMetricStack {
    let envelope = join_envelope(RunLifecycleState::AssistedReview);
    let seal = detector_contaminated_seal(&envelope);
    let mut coverage = build_coverage(
        ReferenceCoveragePurpose::DiagnosticOnly,
        vec![record(1, ReferenceCueDisposition::TranscriptionError)],
        "run-join",
    );
    coverage.seal_id = seal.seal_id.clone();
    let human_reference = HumanFinalReference {
        schema_revision: HUMAN_FINAL_REFERENCE_SCHEMA.to_string(),
        run_id: coverage.run_id.clone(),
        input_identity: coverage.input_identity.clone(),
        seal_id: coverage.seal_id.clone(),
        reference_revision: coverage.reference_revision.clone(),
        records: vec![reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong")],
        state: HumanFinalReferenceState::Sealed,
        assessment: HumanFinalReference::derive_assessment(
            &coverage.reference_revision,
            &coverage.input_identity,
            &[reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong")],
        )
        .expect("derive assessment"),
    };
    coverage.assessment.total_eligible_transcription_errors = human_reference
        .assessment
        .recall_eligible_transcription_error_count;
    let snapshot = build_snapshot(
        vec![glossary_proposal(
            "det-prop-001",
            1,
            0,
            0,
            4,
            "wrng",
            "wrong",
        )],
        DetectorProposalSnapshotState::Frozen,
    );
    let binding_context = ArtifactBindingContext {
        run_id: envelope.run_id.clone(),
        input_identity: envelope.input_identity.clone(),
        calibration_validity: envelope.calibration_validity,
        reference_seal_id: Some(seal.seal_id.clone()),
        reference_coverage_id: Some(coverage.coverage_id.clone()),
        reference_revision: Some(seal.reference_revision.clone()),
    };
    let bundle = build_aggregation_bundle(binding_context, ArtifactBundleState::Complete);
    let adjudication = frozen_adjudication_set(vec![]);
    let join = DetectorReferenceJoin::derive(
        &join_context(),
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive contaminated join");
    (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contribution_context(),
        adjudication,
        join,
    )
}

type PrimaryMetricStack = (
    RunEnvelope,
    ReferenceSeal,
    ReferenceCoverage,
    HumanFinalReference,
    DetectorProposalSnapshot,
    ArtifactBundle,
    JoinMetricContributionContext,
    OverlapAdjudicationSet,
    DetectorReferenceJoin,
);

fn primary_metric_stack() -> PrimaryMetricStack {
    let envelope = join_envelope(RunLifecycleState::AssistedReview);
    let seal = join_seal();
    let mut coverage = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        vec![record(1, ReferenceCueDisposition::TranscriptionError)],
        "run-join",
    );
    let human_reference = HumanFinalReference {
        schema_revision: HUMAN_FINAL_REFERENCE_SCHEMA.to_string(),
        run_id: coverage.run_id.clone(),
        input_identity: coverage.input_identity.clone(),
        seal_id: coverage.seal_id.clone(),
        reference_revision: coverage.reference_revision.clone(),
        records: vec![reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong")],
        state: HumanFinalReferenceState::Sealed,
        assessment: HumanFinalReference::derive_assessment(
            &coverage.reference_revision,
            &coverage.input_identity,
            &[reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong")],
        )
        .expect("derive assessment"),
    };
    coverage.assessment.total_eligible_transcription_errors = human_reference
        .assessment
        .recall_eligible_transcription_error_count;

    let snapshot = build_snapshot(
        vec![glossary_proposal(
            "det-prop-001",
            1,
            0,
            2,
            6,
            "wrng",
            "wrong",
        )],
        DetectorProposalSnapshotState::Frozen,
    );

    let binding_context = ArtifactBindingContext {
        run_id: envelope.run_id.clone(),
        input_identity: envelope.input_identity.clone(),
        calibration_validity: envelope.calibration_validity,
        reference_seal_id: Some(seal.seal_id.clone()),
        reference_coverage_id: Some(coverage.coverage_id.clone()),
        reference_revision: Some(seal.reference_revision.clone()),
    };
    let bundle = build_aggregation_bundle(binding_context, ArtifactBundleState::Complete);
    let adjudication = frozen_adjudication_set(vec![adjudication_record(
        "adj-001",
        "det-prop-001",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);
    let join = DetectorReferenceJoin::derive(
        &join_context(),
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive overlap join");

    (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contribution_context(),
        adjudication,
        join,
    )
}

#[allow(clippy::too_many_arguments)]
fn derive_contribution_set(
    envelope: &RunEnvelope,
    seal: &ReferenceSeal,
    coverage: &ReferenceCoverage,
    human_reference: &HumanFinalReference,
    snapshot: &DetectorProposalSnapshot,
    join: &DetectorReferenceJoin,
    adjudication: &OverlapAdjudicationSet,
    bundle: &ArtifactBundle,
    context: &JoinMetricContributionContext,
) -> JoinMetricContributionSet {
    JoinMetricContributionSet::derive(
        context,
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
    )
    .expect("derive contribution set")
}

#[allow(clippy::too_many_arguments)]
fn validate_contribution_against_stack(
    set: &JoinMetricContributionSet,
    envelope: &RunEnvelope,
    seal: &ReferenceSeal,
    coverage: &ReferenceCoverage,
    human_reference: &HumanFinalReference,
    snapshot: &DetectorProposalSnapshot,
    join: &DetectorReferenceJoin,
    adjudication: &OverlapAdjudicationSet,
    bundle: &ArtifactBundle,
) {
    set.validate_against(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
    )
    .expect("validate contribution set");
}

#[allow(clippy::too_many_arguments)]
fn assert_top_level_binding_mismatch(
    set: &JoinMetricContributionSet,
    envelope: &RunEnvelope,
    seal: &ReferenceSeal,
    coverage: &ReferenceCoverage,
    human_reference: &HumanFinalReference,
    snapshot: &DetectorProposalSnapshot,
    join: &DetectorReferenceJoin,
    adjudication: &OverlapAdjudicationSet,
    bundle: &ArtifactBundle,
    mutate: impl FnOnce(&mut JoinMetricContributionSet),
    expected_field: &str,
) {
    let mut mutated = set.clone();
    mutate(&mut mutated);
    assert!(
        matches!(
            mutated.validate_against(
                envelope,
                seal,
                coverage,
                human_reference,
                snapshot,
                join,
                adjudication,
                bundle,
            ),
            Err(JoinMetricContributionError::TopLevelBindingMismatch { field })
                if field == expected_field
        ),
        "expected TopLevelBindingMismatch for field {expected_field}"
    );
}

fn metric_record(set: &JoinMetricAggregateSet, kind: PrimaryMetricKind) -> &MetricAggregateRecord {
    set.metrics
        .iter()
        .find(|record| record.metric_kind == kind)
        .unwrap_or_else(|| panic!("missing aggregate metric {kind:?}"))
}

fn assert_metric_counts(
    set: &JoinMetricAggregateSet,
    kind: PrimaryMetricKind,
    num: u64,
    den: u64,
    value_state: MetricAggregateValueState,
) {
    let record = metric_record(set, kind);
    assert_eq!(record.numerator_count, num, "{kind:?} numerator");
    assert_eq!(record.denominator_count, den, "{kind:?} denominator");
    assert_eq!(record.value_state, value_state, "{kind:?} value_state");
}

#[allow(clippy::too_many_arguments)]
fn derive_aggregate_set(
    envelope: &RunEnvelope,
    seal: &ReferenceSeal,
    coverage: &ReferenceCoverage,
    human_reference: &HumanFinalReference,
    snapshot: &DetectorProposalSnapshot,
    join: &DetectorReferenceJoin,
    adjudication: &OverlapAdjudicationSet,
    bundle: &ArtifactBundle,
    contribution: &JoinMetricContributionSet,
    context: &JoinMetricAggregateContext,
) -> JoinMetricAggregateSet {
    JoinMetricAggregateSet::derive(
        context,
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        contribution,
        bundle,
    )
    .expect("derive aggregate set")
}

#[allow(clippy::too_many_arguments)]
fn derive_contribution_and_aggregate(
    envelope: &RunEnvelope,
    seal: &ReferenceSeal,
    coverage: &ReferenceCoverage,
    human_reference: &HumanFinalReference,
    snapshot: &DetectorProposalSnapshot,
    join: &DetectorReferenceJoin,
    adjudication: &OverlapAdjudicationSet,
    bundle: &ArtifactBundle,
    contribution_context: &JoinMetricContributionContext,
    aggregate_context: &JoinMetricAggregateContext,
) -> (JoinMetricContributionSet, JoinMetricAggregateSet) {
    let contribution = derive_contribution_set(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contribution_context,
    );
    let aggregate = derive_aggregate_set(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        &contribution,
        aggregate_context,
    );
    (contribution, aggregate)
}

#[allow(clippy::too_many_arguments)]
fn validate_aggregate_against_stack(
    set: &JoinMetricAggregateSet,
    envelope: &RunEnvelope,
    seal: &ReferenceSeal,
    coverage: &ReferenceCoverage,
    human_reference: &HumanFinalReference,
    snapshot: &DetectorProposalSnapshot,
    join: &DetectorReferenceJoin,
    adjudication: &OverlapAdjudicationSet,
    contribution: &JoinMetricContributionSet,
    bundle: &ArtifactBundle,
) {
    set.validate_against(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        contribution,
        bundle,
    )
    .expect("validate aggregate set");
}

#[allow(clippy::too_many_arguments)]
fn assert_aggregate_top_level_binding_mismatch(
    set: &JoinMetricAggregateSet,
    envelope: &RunEnvelope,
    seal: &ReferenceSeal,
    coverage: &ReferenceCoverage,
    human_reference: &HumanFinalReference,
    snapshot: &DetectorProposalSnapshot,
    join: &DetectorReferenceJoin,
    adjudication: &OverlapAdjudicationSet,
    contribution: &JoinMetricContributionSet,
    bundle: &ArtifactBundle,
    mutate: impl FnOnce(&mut JoinMetricAggregateSet),
    expected_field: &str,
) {
    let mut mutated = set.clone();
    mutate(&mut mutated);
    let result = mutated.validate_against(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        contribution,
        bundle,
    );
    assert!(
        result.is_err(),
        "expected validate_against rejection for field {expected_field}, got {result:?}"
    );
}

fn defined() -> MetricAggregateValueState {
    MetricAggregateValueState::DefinedExactRatio
}

fn undefined() -> MetricAggregateValueState {
    MetricAggregateValueState::UndefinedZeroDenominator
}

fn assert_all_five_metrics_present(set: &JoinMetricAggregateSet) {
    assert_eq!(set.metrics.len(), 5);
    assert_eq!(
        set.metrics[0].metric_kind,
        PrimaryMetricKind::ProposalPrecision
    );
    assert_eq!(
        set.metrics[1].metric_kind,
        PrimaryMetricKind::ErrorLocalizationRecall
    );
    assert_eq!(
        set.metrics[2].metric_kind,
        PrimaryMetricKind::CorrectionExactnessGivenLocalization
    );
    assert_eq!(
        set.metrics[3].metric_kind,
        PrimaryMetricKind::EndToEndExactCorrectionRecall
    );
    assert_eq!(
        set.metrics[4].metric_kind,
        PrimaryMetricKind::DuplicateProposalBurden
    );
}

fn derive_join_mixed_fixture() -> PrimaryMetricStack {
    let envelope = join_envelope(RunLifecycleState::DetectorExecution);
    let seal = join_seal();
    let mut coverage = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        vec![
            record(1, ReferenceCueDisposition::TranscriptionError),
            record(2, ReferenceCueDisposition::TranscriptionError),
        ],
        "run-join",
    );
    let records = vec![
        reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong"),
        reference_error_record("ref-err-2", 2, 1, 0, 4, "wright"),
    ];
    let human_reference = HumanFinalReference {
        schema_revision: HUMAN_FINAL_REFERENCE_SCHEMA.to_string(),
        run_id: coverage.run_id.clone(),
        input_identity: coverage.input_identity.clone(),
        seal_id: coverage.seal_id.clone(),
        reference_revision: coverage.reference_revision.clone(),
        records: records.clone(),
        state: HumanFinalReferenceState::Sealed,
        assessment: HumanFinalReference::derive_assessment(
            &coverage.reference_revision,
            &coverage.input_identity,
            &records,
        )
        .expect("derive assessment"),
    };
    coverage.assessment.total_eligible_transcription_errors = human_reference
        .assessment
        .recall_eligible_transcription_error_count;
    let snapshot = build_snapshot(
        vec![
            glossary_proposal("det-prop-1", 1, 0, 0, 4, "wrng", "wrong"),
            glossary_proposal("det-prop-2", 2, 1, 0, 4, "wrng", "wright"),
        ],
        DetectorProposalSnapshotState::Frozen,
    );
    let binding_context = ArtifactBindingContext {
        run_id: envelope.run_id.clone(),
        input_identity: envelope.input_identity.clone(),
        calibration_validity: envelope.calibration_validity,
        reference_seal_id: Some(seal.seal_id.clone()),
        reference_coverage_id: Some(coverage.coverage_id.clone()),
        reference_revision: Some(seal.reference_revision.clone()),
    };
    let bundle = build_aggregation_bundle(binding_context, ArtifactBundleState::Complete);
    let adjudication = frozen_adjudication_set(vec![]);
    let join = DetectorReferenceJoin::derive(
        &join_context(),
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive mixed join");
    (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contribution_context(),
        adjudication,
        join,
    )
}

fn derive_join_empty() -> PrimaryMetricStack {
    let envelope = join_envelope(RunLifecycleState::DetectorExecution);
    let seal = join_seal();
    let mut coverage = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        vec![record(1, ReferenceCueDisposition::NoTranscriptionError)],
        "run-join",
    );
    let human_reference = HumanFinalReference {
        schema_revision: HUMAN_FINAL_REFERENCE_SCHEMA.to_string(),
        run_id: coverage.run_id.clone(),
        input_identity: coverage.input_identity.clone(),
        seal_id: coverage.seal_id.clone(),
        reference_revision: coverage.reference_revision.clone(),
        records: vec![],
        state: HumanFinalReferenceState::Sealed,
        assessment: HumanFinalReference::derive_assessment(
            &coverage.reference_revision,
            &coverage.input_identity,
            &[],
        )
        .expect("derive assessment"),
    };
    coverage.assessment.total_eligible_transcription_errors = 0;
    let snapshot = build_snapshot(vec![], DetectorProposalSnapshotState::Frozen);
    let binding_context = ArtifactBindingContext {
        run_id: envelope.run_id.clone(),
        input_identity: envelope.input_identity.clone(),
        calibration_validity: envelope.calibration_validity,
        reference_seal_id: Some(seal.seal_id.clone()),
        reference_coverage_id: Some(coverage.coverage_id.clone()),
        reference_revision: Some(seal.reference_revision.clone()),
    };
    let bundle = build_aggregation_bundle(binding_context, ArtifactBundleState::Complete);
    let adjudication = frozen_adjudication_set(vec![]);
    let join = DetectorReferenceJoin::derive(
        &join_context(),
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive empty join");
    (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contribution_context(),
        adjudication,
        join,
    )
}

fn derive_join_no_eligible_references() -> PrimaryMetricStack {
    derive_join_excluded_reference()
}

fn stack_tuple(
    stack: &PrimaryMetricStack,
) -> (
    &RunEnvelope,
    &ReferenceSeal,
    &ReferenceCoverage,
    &HumanFinalReference,
    &DetectorProposalSnapshot,
    &ArtifactBundle,
    &JoinMetricContributionContext,
    &OverlapAdjudicationSet,
    &DetectorReferenceJoin,
) {
    (
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.5, &stack.6, &stack.7, &stack.8,
    )
}

fn canonical_primary_metrics() -> Vec<PrimaryMetricKind> {
    vec![
        PrimaryMetricKind::ProposalPrecision,
        PrimaryMetricKind::ErrorLocalizationRecall,
        PrimaryMetricKind::CorrectionExactnessGivenLocalization,
        PrimaryMetricKind::EndToEndExactCorrectionRecall,
        PrimaryMetricKind::DuplicateProposalBurden,
    ]
}

fn expected_detector_ratios(
    disposition: DetectorReferenceMatchDisposition,
) -> (RatioContribution, RatioContribution) {
    match disposition {
        DetectorReferenceMatchDisposition::ExactMatch
        | DetectorReferenceMatchDisposition::AcceptedOverlap => (
            RatioContribution::NumeratorAndDenominator,
            RatioContribution::DenominatorOnly,
        ),
        DetectorReferenceMatchDisposition::DetectorWrongCorrection
        | DetectorReferenceMatchDisposition::UnmatchedDetector => (
            RatioContribution::DenominatorOnly,
            RatioContribution::DenominatorOnly,
        ),
        DetectorReferenceMatchDisposition::DuplicateProposal => (
            RatioContribution::DenominatorOnly,
            RatioContribution::NumeratorAndDenominator,
        ),
        DetectorReferenceMatchDisposition::AmbiguousMatch => (
            RatioContribution::Excluded(MetricContributionExclusionReason::AmbiguousMatch),
            RatioContribution::Excluded(MetricContributionExclusionReason::AmbiguousMatch),
        ),
        DetectorReferenceMatchDisposition::ExcludedFromErrorMetrics => (
            RatioContribution::Excluded(MetricContributionExclusionReason::JoinExcluded),
            RatioContribution::Excluded(MetricContributionExclusionReason::JoinExcluded),
        ),
        DetectorReferenceMatchDisposition::OverlapCandidate => (
            RatioContribution::PendingAdjudication,
            RatioContribution::PendingAdjudication,
        ),
        DetectorReferenceMatchDisposition::UnmatchedReference => (
            RatioContribution::Excluded(MetricContributionExclusionReason::JoinExcluded),
            RatioContribution::Excluded(MetricContributionExclusionReason::JoinExcluded),
        ),
    }
}

fn expected_reference_ratios(
    eligibility: ReferenceJoinEligibility,
    disposition: DetectorReferenceMatchDisposition,
) -> (RatioContribution, RatioContribution, RatioContribution) {
    if eligibility != ReferenceJoinEligibility::RecallEligibleTranscriptionError {
        let excluded =
            RatioContribution::Excluded(MetricContributionExclusionReason::ReferenceIneligible);
        return (excluded, excluded, excluded);
    }

    match disposition {
        DetectorReferenceMatchDisposition::ExactMatch
        | DetectorReferenceMatchDisposition::AcceptedOverlap => (
            RatioContribution::NumeratorAndDenominator,
            RatioContribution::NumeratorAndDenominator,
            RatioContribution::NumeratorAndDenominator,
        ),
        DetectorReferenceMatchDisposition::DetectorWrongCorrection => (
            RatioContribution::NumeratorAndDenominator,
            RatioContribution::DenominatorOnly,
            RatioContribution::DenominatorOnly,
        ),
        DetectorReferenceMatchDisposition::UnmatchedReference => (
            RatioContribution::DenominatorOnly,
            RatioContribution::Excluded(MetricContributionExclusionReason::NotLocalized),
            RatioContribution::DenominatorOnly,
        ),
        DetectorReferenceMatchDisposition::AmbiguousMatch => (
            RatioContribution::Excluded(MetricContributionExclusionReason::AmbiguousMatch),
            RatioContribution::Excluded(MetricContributionExclusionReason::AmbiguousMatch),
            RatioContribution::Excluded(MetricContributionExclusionReason::AmbiguousMatch),
        ),
        DetectorReferenceMatchDisposition::OverlapCandidate => (
            RatioContribution::PendingAdjudication,
            RatioContribution::PendingAdjudication,
            RatioContribution::PendingAdjudication,
        ),
        DetectorReferenceMatchDisposition::ExcludedFromErrorMetrics => (
            RatioContribution::Excluded(MetricContributionExclusionReason::JoinExcluded),
            RatioContribution::Excluded(MetricContributionExclusionReason::JoinExcluded),
            RatioContribution::Excluded(MetricContributionExclusionReason::JoinExcluded),
        ),
        DetectorReferenceMatchDisposition::DuplicateProposal
        | DetectorReferenceMatchDisposition::UnmatchedDetector => (
            RatioContribution::Excluded(MetricContributionExclusionReason::JoinExcluded),
            RatioContribution::Excluded(MetricContributionExclusionReason::JoinExcluded),
            RatioContribution::Excluded(MetricContributionExclusionReason::JoinExcluded),
        ),
    }
}

fn derive_join_exact_match() -> PrimaryMetricStack {
    let envelope = join_envelope(RunLifecycleState::DetectorExecution);
    let seal = join_seal();
    let mut coverage = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        vec![record(1, ReferenceCueDisposition::TranscriptionError)],
        "run-join",
    );
    let human_reference = human_reference_for_coverage(&coverage);
    coverage.assessment.total_eligible_transcription_errors = human_reference
        .assessment
        .recall_eligible_transcription_error_count;
    let snapshot = build_snapshot(
        vec![glossary_proposal(
            "det-prop-001",
            1,
            0,
            0,
            4,
            "wrng",
            "wrong",
        )],
        DetectorProposalSnapshotState::Frozen,
    );
    let binding_context = ArtifactBindingContext {
        run_id: envelope.run_id.clone(),
        input_identity: envelope.input_identity.clone(),
        calibration_validity: envelope.calibration_validity,
        reference_seal_id: Some(seal.seal_id.clone()),
        reference_coverage_id: Some(coverage.coverage_id.clone()),
        reference_revision: Some(seal.reference_revision.clone()),
    };
    let bundle = build_aggregation_bundle(binding_context, ArtifactBundleState::Complete);
    let adjudication = frozen_adjudication_set(vec![]);
    let join = DetectorReferenceJoin::derive(
        &join_context(),
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive exact match join");
    (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contribution_context(),
        adjudication,
        join,
    )
}

fn derive_join_duplicate_proposal() -> PrimaryMetricStack {
    let envelope = join_envelope(RunLifecycleState::DetectorExecution);
    let seal = join_seal();
    let mut coverage = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        vec![record(1, ReferenceCueDisposition::TranscriptionError)],
        "run-join",
    );
    let human_reference = human_reference_for_coverage(&coverage);
    coverage.assessment.total_eligible_transcription_errors = 1;
    let snapshot = build_snapshot(
        vec![
            glossary_proposal("det-prop-b", 1, 0, 0, 4, "wrng", "wrong"),
            observed_error_proposal("det-prop-a", 1, 0, 0, 4, "wrng", "wrong"),
        ],
        DetectorProposalSnapshotState::Frozen,
    );
    let binding_context = ArtifactBindingContext {
        run_id: envelope.run_id.clone(),
        input_identity: envelope.input_identity.clone(),
        calibration_validity: envelope.calibration_validity,
        reference_seal_id: Some(seal.seal_id.clone()),
        reference_coverage_id: Some(coverage.coverage_id.clone()),
        reference_revision: Some(seal.reference_revision.clone()),
    };
    let bundle = build_aggregation_bundle(binding_context, ArtifactBundleState::Complete);
    let adjudication = frozen_adjudication_set(vec![]);
    let join = DetectorReferenceJoin::derive(
        &join_context(),
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive duplicate proposal join");
    (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contribution_context(),
        adjudication,
        join,
    )
}

fn derive_join_wrong_correction() -> PrimaryMetricStack {
    let envelope = join_envelope(RunLifecycleState::DetectorExecution);
    let seal = join_seal();
    let mut coverage = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        vec![record(1, ReferenceCueDisposition::TranscriptionError)],
        "run-join",
    );
    let human_reference = human_reference_for_coverage(&coverage);
    coverage.assessment.total_eligible_transcription_errors = human_reference
        .assessment
        .recall_eligible_transcription_error_count;
    let snapshot = build_snapshot(
        vec![glossary_proposal(
            "det-prop-001",
            1,
            0,
            0,
            4,
            "wrng",
            "wright",
        )],
        DetectorProposalSnapshotState::Frozen,
    );
    let binding_context = ArtifactBindingContext {
        run_id: envelope.run_id.clone(),
        input_identity: envelope.input_identity.clone(),
        calibration_validity: envelope.calibration_validity,
        reference_seal_id: Some(seal.seal_id.clone()),
        reference_coverage_id: Some(coverage.coverage_id.clone()),
        reference_revision: Some(seal.reference_revision.clone()),
    };
    let bundle = build_aggregation_bundle(binding_context, ArtifactBundleState::Complete);
    let adjudication = frozen_adjudication_set(vec![]);
    let join = DetectorReferenceJoin::derive(
        &join_context(),
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive wrong correction join");
    (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contribution_context(),
        adjudication,
        join,
    )
}

fn derive_join_ambiguous_match() -> PrimaryMetricStack {
    let envelope = join_envelope(RunLifecycleState::AssistedReview);
    let seal = join_seal();
    let mut coverage = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        vec![record(1, ReferenceCueDisposition::TranscriptionError)],
        "run-join",
    );
    coverage.expected_universe = universe(&[1]);
    coverage.assessment =
        ReferenceCoverage::derive_assessment(&coverage.expected_universe, &coverage.records)
            .expect("derive assessment");
    coverage.assessment.total_eligible_transcription_errors = 2;
    let records = vec![
        reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong"),
        reference_error_record("ref-err-2", 1, 0, 2, 6, "wrong"),
    ];
    let human_reference = HumanFinalReference {
        schema_revision: HUMAN_FINAL_REFERENCE_SCHEMA.to_string(),
        run_id: coverage.run_id.clone(),
        input_identity: coverage.input_identity.clone(),
        seal_id: coverage.seal_id.clone(),
        reference_revision: coverage.reference_revision.clone(),
        records: records.clone(),
        state: HumanFinalReferenceState::Sealed,
        assessment: HumanFinalReference::derive_assessment(
            &coverage.reference_revision,
            &coverage.input_identity,
            &records,
        )
        .expect("derive assessment"),
    };
    let snapshot = build_snapshot(
        vec![glossary_proposal(
            "det-prop-001",
            1,
            0,
            1,
            5,
            "wrng",
            "wrong",
        )],
        DetectorProposalSnapshotState::Frozen,
    );
    let binding_context = ArtifactBindingContext {
        run_id: envelope.run_id.clone(),
        input_identity: envelope.input_identity.clone(),
        calibration_validity: envelope.calibration_validity,
        reference_seal_id: Some(seal.seal_id.clone()),
        reference_coverage_id: Some(coverage.coverage_id.clone()),
        reference_revision: Some(seal.reference_revision.clone()),
    };
    let bundle = build_aggregation_bundle(binding_context, ArtifactBundleState::Complete);
    let adjudication = frozen_adjudication_set(vec![
        adjudication_record(
            "adj-001",
            "det-prop-001",
            "ref-err-1",
            OverlapAdjudicationResult::SameErrorSameCorrection,
        ),
        adjudication_record(
            "adj-002",
            "det-prop-001",
            "ref-err-2",
            OverlapAdjudicationResult::SameErrorSameCorrection,
        ),
    ]);
    let join = DetectorReferenceJoin::derive(
        &join_context(),
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive ambiguous join");
    (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contribution_context(),
        adjudication,
        join,
    )
}

fn derive_join_excluded_reference() -> PrimaryMetricStack {
    let envelope = join_envelope(RunLifecycleState::DetectorExecution);
    let seal = join_seal();
    let mut coverage = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        vec![record(1, ReferenceCueDisposition::TranscriptionError)],
        "run-join",
    );
    coverage.assessment.total_eligible_transcription_errors = 0;
    let human_reference = HumanFinalReference {
        schema_revision: HUMAN_FINAL_REFERENCE_SCHEMA.to_string(),
        run_id: coverage.run_id.clone(),
        input_identity: coverage.input_identity.clone(),
        seal_id: coverage.seal_id.clone(),
        reference_revision: coverage.reference_revision.clone(),
        records: vec![ReferenceErrorRecord {
            reference_error_id: ReferenceErrorId::new("ref-err-tco").expect("error id"),
            reference_revision: ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION)
                .expect("revision"),
            input_identity: input_identity(),
            source_anchor: reference_source_anchor(1, 0, 0, 4),
            original_surface: "wrng".to_string(),
            human_final_surface: "wrong".to_string(),
            reference_class: ReferenceClass::TranscriptionError,
            verification_basis: VerificationBasis::TranscriptContextOnly,
            reviewer_identity_class: ReferenceReviewerIdentityClass::OwnerBlindReviewer,
            reviewed_at_unix_ms: 1_700_000_000_000,
        }],
        state: HumanFinalReferenceState::Sealed,
        assessment: HumanFinalReference::derive_assessment(
            &coverage.reference_revision,
            &coverage.input_identity,
            &[ReferenceErrorRecord {
                reference_error_id: ReferenceErrorId::new("ref-err-tco").expect("error id"),
                reference_revision: ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION)
                    .expect("revision"),
                input_identity: input_identity(),
                source_anchor: reference_source_anchor(1, 0, 0, 4),
                original_surface: "wrng".to_string(),
                human_final_surface: "wrong".to_string(),
                reference_class: ReferenceClass::TranscriptionError,
                verification_basis: VerificationBasis::TranscriptContextOnly,
                reviewer_identity_class: ReferenceReviewerIdentityClass::OwnerBlindReviewer,
                reviewed_at_unix_ms: 1_700_000_000_000,
            }],
        )
        .expect("derive assessment"),
    };
    let snapshot = build_snapshot(
        vec![glossary_proposal(
            "det-prop-001",
            1,
            0,
            0,
            4,
            "wrng",
            "wrong",
        )],
        DetectorProposalSnapshotState::Frozen,
    );
    let binding_context = ArtifactBindingContext {
        run_id: envelope.run_id.clone(),
        input_identity: envelope.input_identity.clone(),
        calibration_validity: envelope.calibration_validity,
        reference_seal_id: Some(seal.seal_id.clone()),
        reference_coverage_id: Some(coverage.coverage_id.clone()),
        reference_revision: Some(seal.reference_revision.clone()),
    };
    let bundle = build_aggregation_bundle(binding_context, ArtifactBundleState::Complete);
    let adjudication = frozen_adjudication_set(vec![]);
    let join = DetectorReferenceJoin::derive(
        &join_context(),
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive excluded reference join");
    (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contribution_context(),
        adjudication,
        join,
    )
}

fn derive_join_unmatched_reference() -> PrimaryMetricStack {
    let envelope = join_envelope(RunLifecycleState::DetectorExecution);
    let seal = join_seal();
    let mut coverage = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        vec![record(1, ReferenceCueDisposition::TranscriptionError)],
        "run-join",
    );
    let records = vec![
        reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong"),
        reference_error_record("ref-err-2", 1, 0, 8, 12, "wrong"),
    ];
    let human_reference = HumanFinalReference {
        schema_revision: HUMAN_FINAL_REFERENCE_SCHEMA.to_string(),
        run_id: coverage.run_id.clone(),
        input_identity: coverage.input_identity.clone(),
        seal_id: coverage.seal_id.clone(),
        reference_revision: coverage.reference_revision.clone(),
        records: records.clone(),
        state: HumanFinalReferenceState::Sealed,
        assessment: HumanFinalReference::derive_assessment(
            &coverage.reference_revision,
            &coverage.input_identity,
            &records,
        )
        .expect("derive assessment"),
    };
    coverage.assessment.total_eligible_transcription_errors = human_reference
        .assessment
        .recall_eligible_transcription_error_count;
    let snapshot = build_snapshot(
        vec![glossary_proposal(
            "det-prop-001",
            1,
            0,
            0,
            4,
            "wrng",
            "wrong",
        )],
        DetectorProposalSnapshotState::Frozen,
    );
    let binding_context = ArtifactBindingContext {
        run_id: envelope.run_id.clone(),
        input_identity: envelope.input_identity.clone(),
        calibration_validity: envelope.calibration_validity,
        reference_seal_id: Some(seal.seal_id.clone()),
        reference_coverage_id: Some(coverage.coverage_id.clone()),
        reference_revision: Some(seal.reference_revision.clone()),
    };
    let bundle = build_aggregation_bundle(binding_context, ArtifactBundleState::Complete);
    let adjudication = frozen_adjudication_set(vec![]);
    let join = DetectorReferenceJoin::derive(
        &join_context(),
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive unmatched reference join");
    (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contribution_context(),
        adjudication,
        join,
    )
}

fn derive_join_unmatched_detector() -> PrimaryMetricStack {
    let envelope = join_envelope(RunLifecycleState::DetectorExecution);
    let seal = join_seal();
    let mut coverage = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        vec![record(1, ReferenceCueDisposition::TranscriptionError)],
        "run-join",
    );
    let human_reference = HumanFinalReference {
        schema_revision: HUMAN_FINAL_REFERENCE_SCHEMA.to_string(),
        run_id: coverage.run_id.clone(),
        input_identity: coverage.input_identity.clone(),
        seal_id: coverage.seal_id.clone(),
        reference_revision: coverage.reference_revision.clone(),
        records: vec![reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong")],
        state: HumanFinalReferenceState::Sealed,
        assessment: HumanFinalReference::derive_assessment(
            &coverage.reference_revision,
            &coverage.input_identity,
            &[reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong")],
        )
        .expect("derive assessment"),
    };
    coverage.assessment.total_eligible_transcription_errors = human_reference
        .assessment
        .recall_eligible_transcription_error_count;
    let snapshot = build_snapshot(
        vec![glossary_proposal(
            "det-prop-001",
            1,
            0,
            8,
            12,
            "wrng",
            "wrong",
        )],
        DetectorProposalSnapshotState::Frozen,
    );
    let binding_context = ArtifactBindingContext {
        run_id: envelope.run_id.clone(),
        input_identity: envelope.input_identity.clone(),
        calibration_validity: envelope.calibration_validity,
        reference_seal_id: Some(seal.seal_id.clone()),
        reference_coverage_id: Some(coverage.coverage_id.clone()),
        reference_revision: Some(seal.reference_revision.clone()),
    };
    let bundle = build_aggregation_bundle(binding_context, ArtifactBundleState::Complete);
    let adjudication = frozen_adjudication_set(vec![]);
    let join = DetectorReferenceJoin::derive(
        &join_context(),
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive unmatched detector join");
    (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contribution_context(),
        adjudication,
        join,
    )
}

fn detector_record_for_disposition(
    set: &JoinMetricContributionSet,
    disposition: DetectorReferenceMatchDisposition,
) -> &vox_proof::join_metric_contribution::DetectorMetricContributionRecord {
    set.detector_contributions
        .iter()
        .find(|record| record.join_disposition == disposition)
        .unwrap_or_else(|| panic!("missing detector disposition {disposition:?}"))
}

fn reference_record_for_disposition(
    set: &JoinMetricContributionSet,
    disposition: DetectorReferenceMatchDisposition,
) -> &vox_proof::join_metric_contribution::ReferenceMetricContributionRecord {
    set.reference_contributions
        .iter()
        .find(|record| record.join_disposition == disposition)
        .unwrap_or_else(|| panic!("missing reference disposition {disposition:?}"))
}

fn assert_validate_against_rejects(
    set: &JoinMetricAggregateSet,
    envelope: &RunEnvelope,
    seal: &ReferenceSeal,
    coverage: &ReferenceCoverage,
    human_reference: &HumanFinalReference,
    snapshot: &DetectorProposalSnapshot,
    join: &DetectorReferenceJoin,
    adjudication: &OverlapAdjudicationSet,
    contribution: &JoinMetricContributionSet,
    bundle: &ArtifactBundle,
) {
    assert!(
        set.validate_against(
            envelope,
            seal,
            coverage,
            human_reference,
            snapshot,
            join,
            adjudication,
            contribution,
            bundle,
        )
        .is_err(),
        "validate_against must reject mutated aggregate"
    );
}

// --- Schema / serialization ---

#[test]
fn json_round_trip_retains_schema_and_policy_constants() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (_, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );

    let json = aggregate_to_json(&set).expect("serialize");
    assert!(json.contains(JOIN_METRIC_AGGREGATION_SCHEMA));
    assert!(json.contains(PRIMARY_METRIC_AGGREGATION_POLICY));
    assert!(json.contains(ZERO_DENOMINATOR_POLICY));

    let restored = aggregate_from_json(&json).expect("deserialize");
    assert_eq!(restored.schema_revision, JOIN_METRIC_AGGREGATION_SCHEMA);
    assert_eq!(
        restored.aggregation_policy_revision,
        PRIMARY_METRIC_AGGREGATION_POLICY
    );
    assert_eq!(
        restored.zero_denominator_policy_revision,
        ZERO_DENOMINATOR_POLICY
    );
    restored.validate().expect("valid aggregate set");
}

#[test]
fn schema_revision_constants_enforced() {
    assert_eq!(
        JOIN_METRIC_AGGREGATION_SCHEMA,
        "voxproof-join-metric-aggregates-v1"
    );
    assert_eq!(
        PRIMARY_METRIC_AGGREGATION_POLICY,
        "voxproof-primary-metric-aggregation-v1"
    );
    assert_eq!(
        ZERO_DENOMINATOR_POLICY,
        "voxproof-zero-denominator-undefined-v1"
    );
}

#[test]
fn metric_order_is_canonical_primary_sequence() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (_, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    assert_all_five_metrics_present(&set);
}

#[test]
fn primary_metric_kind_and_value_state_enum_spellings() {
    assert_eq!(
        serde_json::to_string(&PrimaryMetricKind::ProposalPrecision).expect("serialize"),
        "\"proposal_precision\""
    );
    assert_eq!(
        serde_json::to_string(&MetricAggregateValueState::DefinedExactRatio).expect("serialize"),
        "\"defined_exact_ratio\""
    );
    assert_eq!(
        serde_json::to_string(&MetricAggregateValueState::UndefinedZeroDenominator)
            .expect("serialize"),
        "\"undefined_zero_denominator\""
    );
    assert_eq!(
        serde_json::to_string(&MetricAggregateSetState::Complete).expect("serialize"),
        "\"complete\""
    );
    assert_eq!(
        serde_json::to_string(&MetricAggregateSetState::Invalidated).expect("serialize"),
        "\"invalidated\""
    );
}

#[test]
fn unknown_top_level_field_rejected_on_round_trip() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (_, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    let mut value = serde_json::to_value(&set).expect("value");
    value
        .as_object_mut()
        .expect("object")
        .insert("ratio".to_string(), serde_json::json!(0.95));

    let error = serde_json::from_value::<JoinMetricAggregateSet>(value).expect_err("must fail");
    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn invalid_and_path_like_aggregate_ids_rejected() {
    for value in [
        "",
        "/Users/example/private/metric-aggregate.json",
        "../private/metric-aggregate.json",
    ] {
        assert!(
            MetricAggregateSetId::new(value).is_err(),
            "aggregate set id must reject {value:?}"
        );
        assert!(
            MetricAggregateRevisionId::new(value).is_err(),
            "aggregate revision id must reject {value:?}"
        );
        assert!(
            validate_aggregate_id_value(value).is_err(),
            "validate_aggregate_id_value must reject {value:?}"
        );
    }
}

#[test]
fn serialized_aggregate_contains_no_performance_or_pii_fields() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (_, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    let json = aggregate_to_json(&set).expect("serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");

    for forbidden in [
        "transcript_text",
        "cue_text",
        "correction_text",
        "human_final_surface",
        "original_surface",
        "observed_surface",
        "reviewer_name",
        "email",
        "path",
        "audio",
        "float",
        "decimal",
        "percentage",
        "threshold",
        "true_positive",
        "false_positive",
        "false_negative",
        "pass",
        "fail",
        "precision_ratio",
        "recall_ratio",
        "f1",
    ] {
        assert!(
            value.get(forbidden).is_none(),
            "serialized aggregate must not contain {forbidden}"
        );
    }
    for token in ["0.", "1.0", "95%", "TP", "FP", "FN"] {
        assert!(
            !json.contains(token),
            "serialized aggregate must not contain performance token {token:?}"
        );
    }
}

// --- Standard aggregation scenarios ---

#[test]
fn aggregate_exact_match_counts_all_five_metrics() {
    let stack = derive_join_exact_match();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (_, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    assert_metric_counts(&set, PrimaryMetricKind::ProposalPrecision, 1, 1, defined());
    assert_metric_counts(
        &set,
        PrimaryMetricKind::DuplicateProposalBurden,
        0,
        1,
        defined(),
    );
    assert_metric_counts(
        &set,
        PrimaryMetricKind::ErrorLocalizationRecall,
        1,
        1,
        defined(),
    );
    assert_metric_counts(
        &set,
        PrimaryMetricKind::CorrectionExactnessGivenLocalization,
        1,
        1,
        defined(),
    );
    assert_metric_counts(
        &set,
        PrimaryMetricKind::EndToEndExactCorrectionRecall,
        1,
        1,
        defined(),
    );
}

#[test]
fn aggregate_accepted_overlap_counts_all_five_metrics() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (_, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    assert_metric_counts(&set, PrimaryMetricKind::ProposalPrecision, 1, 1, defined());
    assert_metric_counts(
        &set,
        PrimaryMetricKind::DuplicateProposalBurden,
        0,
        1,
        defined(),
    );
    assert_metric_counts(
        &set,
        PrimaryMetricKind::ErrorLocalizationRecall,
        1,
        1,
        defined(),
    );
    assert_metric_counts(
        &set,
        PrimaryMetricKind::CorrectionExactnessGivenLocalization,
        1,
        1,
        defined(),
    );
    assert_metric_counts(
        &set,
        PrimaryMetricKind::EndToEndExactCorrectionRecall,
        1,
        1,
        defined(),
    );
}

#[test]
fn aggregate_wrong_correction_counts_all_five_metrics() {
    let stack = derive_join_wrong_correction();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (_, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    assert_metric_counts(&set, PrimaryMetricKind::ProposalPrecision, 0, 1, defined());
    assert_metric_counts(
        &set,
        PrimaryMetricKind::DuplicateProposalBurden,
        0,
        1,
        defined(),
    );
    assert_metric_counts(
        &set,
        PrimaryMetricKind::ErrorLocalizationRecall,
        1,
        1,
        defined(),
    );
    assert_metric_counts(
        &set,
        PrimaryMetricKind::CorrectionExactnessGivenLocalization,
        0,
        1,
        defined(),
    );
    assert_metric_counts(
        &set,
        PrimaryMetricKind::EndToEndExactCorrectionRecall,
        0,
        1,
        defined(),
    );
}

#[test]
fn aggregate_unmatched_detector_counts_all_five_metrics() {
    let stack = derive_join_unmatched_detector();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (_, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    assert_metric_counts(&set, PrimaryMetricKind::ProposalPrecision, 0, 1, defined());
    assert_metric_counts(
        &set,
        PrimaryMetricKind::DuplicateProposalBurden,
        0,
        1,
        defined(),
    );
    assert_metric_counts(
        &set,
        PrimaryMetricKind::ErrorLocalizationRecall,
        0,
        1,
        defined(),
    );
    assert_metric_counts(
        &set,
        PrimaryMetricKind::CorrectionExactnessGivenLocalization,
        0,
        0,
        undefined(),
    );
    assert_metric_counts(
        &set,
        PrimaryMetricKind::EndToEndExactCorrectionRecall,
        0,
        1,
        defined(),
    );
}

#[test]
fn aggregate_unmatched_reference_counts_all_five_metrics() {
    let stack = derive_join_unmatched_reference();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (_, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    assert_metric_counts(&set, PrimaryMetricKind::ProposalPrecision, 1, 1, defined());
    assert_metric_counts(
        &set,
        PrimaryMetricKind::DuplicateProposalBurden,
        0,
        1,
        defined(),
    );
    assert_metric_counts(
        &set,
        PrimaryMetricKind::ErrorLocalizationRecall,
        1,
        2,
        defined(),
    );
    assert_metric_counts(
        &set,
        PrimaryMetricKind::CorrectionExactnessGivenLocalization,
        1,
        1,
        defined(),
    );
    assert_metric_counts(
        &set,
        PrimaryMetricKind::EndToEndExactCorrectionRecall,
        1,
        2,
        defined(),
    );
}

#[test]
fn aggregate_duplicate_proposals_counts_all_five_metrics() {
    let stack = derive_join_duplicate_proposal();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (_, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    assert_metric_counts(&set, PrimaryMetricKind::ProposalPrecision, 1, 2, defined());
    assert_metric_counts(
        &set,
        PrimaryMetricKind::DuplicateProposalBurden,
        1,
        2,
        defined(),
    );
    assert_metric_counts(
        &set,
        PrimaryMetricKind::ErrorLocalizationRecall,
        1,
        1,
        defined(),
    );
    assert_metric_counts(
        &set,
        PrimaryMetricKind::CorrectionExactnessGivenLocalization,
        1,
        1,
        defined(),
    );
    assert_metric_counts(
        &set,
        PrimaryMetricKind::EndToEndExactCorrectionRecall,
        1,
        1,
        defined(),
    );
}

#[test]
fn aggregate_ambiguous_exclusions_yield_zero_denominators() {
    let stack = derive_join_ambiguous_match();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (_, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    for kind in canonical_primary_metrics() {
        assert_metric_counts(&set, kind, 0, 0, undefined());
    }
}

#[test]
fn aggregate_ineligible_reference_exclusions() {
    let stack = derive_join_excluded_reference();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (_, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    assert_metric_counts(&set, PrimaryMetricKind::ProposalPrecision, 0, 1, defined());
    assert_metric_counts(
        &set,
        PrimaryMetricKind::DuplicateProposalBurden,
        0,
        1,
        defined(),
    );
    for kind in [
        PrimaryMetricKind::ErrorLocalizationRecall,
        PrimaryMetricKind::CorrectionExactnessGivenLocalization,
        PrimaryMetricKind::EndToEndExactCorrectionRecall,
    ] {
        assert_metric_counts(&set, kind, 0, 0, undefined());
    }
}

#[test]
fn aggregate_mixed_fixture_counts_all_five_metrics() {
    let stack = derive_join_mixed_fixture();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (_, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    assert_metric_counts(&set, PrimaryMetricKind::ProposalPrecision, 2, 2, defined());
    assert_metric_counts(
        &set,
        PrimaryMetricKind::DuplicateProposalBurden,
        0,
        2,
        defined(),
    );
    assert_metric_counts(
        &set,
        PrimaryMetricKind::ErrorLocalizationRecall,
        2,
        2,
        defined(),
    );
    assert_metric_counts(
        &set,
        PrimaryMetricKind::CorrectionExactnessGivenLocalization,
        2,
        2,
        defined(),
    );
    assert_metric_counts(
        &set,
        PrimaryMetricKind::EndToEndExactCorrectionRecall,
        2,
        2,
        defined(),
    );
}

#[test]
fn shuffled_contributions_produce_identical_aggregates() {
    let stack_a = derive_join_duplicate_proposal();
    let stack_b = {
        let mut stack = derive_join_duplicate_proposal();
        stack.4.proposals.reverse();
        stack.4.assessment = DetectorProposalSnapshot::derive_assessment(
            &snapshot_revision_id(),
            &input_identity(),
            &analysis_identity(),
            &stack.4.proposals,
        )
        .expect("derive assessment");
        let binding_context = ArtifactBindingContext {
            run_id: stack.0.run_id.clone(),
            input_identity: stack.0.input_identity.clone(),
            calibration_validity: stack.0.calibration_validity,
            reference_seal_id: Some(stack.1.seal_id.clone()),
            reference_coverage_id: Some(stack.2.coverage_id.clone()),
            reference_revision: Some(stack.1.reference_revision.clone()),
        };
        stack.5 = build_aggregation_bundle(binding_context, ArtifactBundleState::Complete);
        stack.8 = DetectorReferenceJoin::derive(
            &join_context(),
            &stack.0,
            &stack.1,
            &stack.2,
            &stack.3,
            &stack.4,
            &stack.5,
            &stack.7,
        )
        .expect("derive shuffled join");
        stack
    };

    let (ea, _, _, _, _, _, ca, _, _ja) = stack_tuple(&stack_a);
    let (_, set_a) = derive_contribution_and_aggregate(
        ea,
        &stack_a.1,
        &stack_a.2,
        &stack_a.3,
        &stack_a.4,
        &stack_a.8,
        &stack_a.7,
        &stack_a.5,
        ca,
        &aggregate_context(),
    );
    let (eb, _, _, _, _, _, cb, _, _) = stack_tuple(&stack_b);
    let (_, set_b) = derive_contribution_and_aggregate(
        eb,
        &stack_b.1,
        &stack_b.2,
        &stack_b.3,
        &stack_b.4,
        &stack_b.8,
        &stack_b.7,
        &stack_b.5,
        cb,
        &aggregate_context(),
    );
    assert_eq!(set_a.metrics, set_b.metrics);
    assert_eq!(set_a.assessment, set_b.assessment);
}

// --- Zero denominators ---

#[test]
fn zero_denominator_no_detector_proposals() {
    let stack = derive_join_empty();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (_, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    assert_metric_counts(
        &set,
        PrimaryMetricKind::ProposalPrecision,
        0,
        0,
        undefined(),
    );
    assert_metric_counts(
        &set,
        PrimaryMetricKind::DuplicateProposalBurden,
        0,
        0,
        undefined(),
    );
}

#[test]
fn zero_denominator_no_eligible_references() {
    let stack = derive_join_no_eligible_references();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (_, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    for kind in [
        PrimaryMetricKind::ErrorLocalizationRecall,
        PrimaryMetricKind::CorrectionExactnessGivenLocalization,
        PrimaryMetricKind::EndToEndExactCorrectionRecall,
    ] {
        assert_metric_counts(&set, kind, 0, 0, undefined());
    }
}

#[test]
fn zero_denominator_both_empty() {
    let stack = derive_join_empty();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (_, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    for kind in canonical_primary_metrics() {
        assert_metric_counts(&set, kind, 0, 0, undefined());
    }
}

#[test]
fn zero_denominator_zero_numerator_when_zero_denominator() {
    let stack = derive_join_empty();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (_, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    for record in &set.metrics {
        if record.denominator_count == 0 {
            assert_eq!(record.numerator_count, 0);
            assert_eq!(
                record.value_state,
                MetricAggregateValueState::UndefinedZeroDenominator
            );
        }
    }
}

#[test]
fn zero_denominator_never_defined_ratio() {
    let stack = derive_join_empty();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (_, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    for record in &set.metrics {
        if record.denominator_count == 0 {
            assert_ne!(
                record.value_state,
                MetricAggregateValueState::DefinedExactRatio
            );
        }
    }
}

// --- Contribution source ---

#[test]
fn contribution_source_numerator_and_denominator_increments() {
    let stack = derive_join_exact_match();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (contribution, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    let detector = detector_record_for_disposition(
        &contribution,
        DetectorReferenceMatchDisposition::ExactMatch,
    );
    assert_eq!(
        detector.proposal_precision,
        RatioContribution::NumeratorAndDenominator
    );
    assert_metric_counts(&set, PrimaryMetricKind::ProposalPrecision, 1, 1, defined());
}

#[test]
fn contribution_source_denominator_only_increments() {
    let stack = derive_join_wrong_correction();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (contribution, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    let detector = detector_record_for_disposition(
        &contribution,
        DetectorReferenceMatchDisposition::DetectorWrongCorrection,
    );
    assert_eq!(
        detector.proposal_precision,
        RatioContribution::DenominatorOnly
    );
    assert_metric_counts(&set, PrimaryMetricKind::ProposalPrecision, 0, 1, defined());
}

#[test]
fn contribution_source_exclusions_do_not_increment_counts() {
    let stack = derive_join_ambiguous_match();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (contribution, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    let detector = detector_record_for_disposition(
        &contribution,
        DetectorReferenceMatchDisposition::AmbiguousMatch,
    );
    assert!(matches!(
        detector.proposal_precision,
        RatioContribution::Excluded(_)
    ));
    assert_metric_counts(
        &set,
        PrimaryMetricKind::ProposalPrecision,
        0,
        0,
        undefined(),
    );
}

#[test]
fn pending_contribution_rejects_aggregation() {
    let envelope = join_envelope(RunLifecycleState::DetectorExecution);
    let seal = join_seal();
    let mut coverage = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        vec![record(1, ReferenceCueDisposition::TranscriptionError)],
        "run-join",
    );
    let human_reference = HumanFinalReference {
        schema_revision: HUMAN_FINAL_REFERENCE_SCHEMA.to_string(),
        run_id: coverage.run_id.clone(),
        input_identity: coverage.input_identity.clone(),
        seal_id: coverage.seal_id.clone(),
        reference_revision: coverage.reference_revision.clone(),
        records: vec![reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong")],
        state: HumanFinalReferenceState::Sealed,
        assessment: HumanFinalReference::derive_assessment(
            &coverage.reference_revision,
            &coverage.input_identity,
            &[reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong")],
        )
        .expect("derive assessment"),
    };
    coverage.assessment.total_eligible_transcription_errors = human_reference
        .assessment
        .recall_eligible_transcription_error_count;
    let snapshot = build_snapshot(
        vec![glossary_proposal(
            "det-prop-001",
            1,
            0,
            2,
            6,
            "wrng",
            "wrong",
        )],
        DetectorProposalSnapshotState::Frozen,
    );
    let binding_context = ArtifactBindingContext {
        run_id: envelope.run_id.clone(),
        input_identity: envelope.input_identity.clone(),
        calibration_validity: envelope.calibration_validity,
        reference_seal_id: Some(seal.seal_id.clone()),
        reference_coverage_id: Some(coverage.coverage_id.clone()),
        reference_revision: Some(seal.reference_revision.clone()),
    };
    let bundle = build_aggregation_bundle(binding_context, ArtifactBundleState::Complete);
    let adjudication = frozen_adjudication_set(vec![]);
    let join = DetectorReferenceJoin::derive(
        &join_context(),
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive pending join");
    let contribution = derive_contribution_set(
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &join,
        &adjudication,
        &bundle,
        &contribution_context(),
    );
    assert_eq!(
        contribution.state,
        MetricContributionSetState::PendingJoinResolution
    );
    assert!(matches!(
        JoinMetricAggregateSet::derive(
            &aggregate_context(),
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &join,
            &adjudication,
            &contribution,
            &bundle,
        ),
        Err(JoinMetricAggregationError::ContributionSetNotComplete)
            | Err(JoinMetricAggregationError::PendingContributionRejected)
    ));
}

#[test]
fn invalidated_contribution_set_rejects_aggregation() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let mut contribution = derive_contribution_set(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
    );
    contribution.state = MetricContributionSetState::Invalidated;
    assert!(matches!(
        JoinMetricAggregateSet::derive(
            &aggregate_context(),
            envelope,
            seal,
            coverage,
            human_reference,
            snapshot,
            join,
            adjudication,
            &contribution,
            bundle,
        ),
        Err(JoinMetricAggregationError::ContributionSetInvalidated)
            | Err(JoinMetricAggregationError::ContributionSetNotComplete)
    ));
}

#[test]
fn diagnostic_non_primary_contribution_aggregates_with_blocked_eligibility() {
    let stack = diagnostic_metric_stack(ReferenceCoverageState::Complete);
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (_, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    assert!(!set.primary_metrics_allowed);
    assert!(!set.qualifies_as_primary_metric_evidence);
    assert_eq!(
        set.report_class,
        MetricContributionReportClass::NonCalibrationDiagnostic
    );
}

#[test]
fn synthetic_protocol_only_contribution_aggregates_with_protocol_posture() {
    let mut stack = primary_metric_stack();
    stack.0.input_class = InputClass::SyntheticProtocolFixture;
    stack.1.producer_class = ReferenceProducerClass::SyntheticFixtureGenerator;
    stack.1.calibration_classification = ReferenceCalibrationValidity::SyntheticProtocolOnly;
    stack.1.calibration_validity_impact = CalibrationValidityImpact::ProtocolOnly;
    stack.2.coverage_purpose = ReferenceCoveragePurpose::SyntheticProtocolValidation;
    stack.8.join_purpose = DetectorReferenceJoinPurpose::SyntheticProtocolValidation;
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (_, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    assert!(!set.primary_metrics_allowed);
    assert!(
        set.blocking_reasons
            .contains(&PrimaryMetricBlockingReason::SyntheticProtocolOnly)
    );
    assert_eq!(
        set.report_class,
        MetricContributionReportClass::SyntheticProtocolValidation
    );
}

// --- Cross-metric invariants ---

#[test]
fn cross_metric_detector_denominator_parity_enforced() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (contribution, mut set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    set.metrics[0].denominator_count = 99;
    assert!(matches!(
        set.validate(),
        Err(JoinMetricAggregationError::CrossMetricInvariantViolation { .. })
            | Err(JoinMetricAggregationError::NumeratorExceedsDenominator { .. })
            | Err(JoinMetricAggregationError::AssessmentMismatch { .. })
    ));
    assert!(matches!(
        set.validate_against(
            envelope,
            seal,
            coverage,
            human_reference,
            snapshot,
            join,
            adjudication,
            &contribution,
            bundle,
        ),
        Err(JoinMetricAggregationError::TopLevelBindingMismatch { .. })
            | Err(JoinMetricAggregationError::AssessmentMismatch { .. })
            | Err(JoinMetricAggregationError::NonCanonicalMetricOrder)
            | Err(JoinMetricAggregationError::PrimaryEligibilityInconsistent)
            | Err(JoinMetricAggregationError::ReportClassInconsistent)
            | Err(JoinMetricAggregationError::CrossMetricInvariantViolation { .. })
            | Err(JoinMetricAggregationError::ZeroDenominatorValueStateMismatch { .. })
            | Err(JoinMetricAggregationError::NumeratorExceedsDenominator { .. })
    ));
}

#[test]
fn cross_metric_correction_vs_localization_denominator_parity() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (contribution, mut set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    set.metrics[2].denominator_count = 99;
    assert!(matches!(
        set.validate(),
        Err(JoinMetricAggregationError::CrossMetricInvariantViolation { .. })
            | Err(JoinMetricAggregationError::AssessmentMismatch { .. })
    ));
    assert!(matches!(
        set.validate_against(
            envelope,
            seal,
            coverage,
            human_reference,
            snapshot,
            join,
            adjudication,
            &contribution,
            bundle,
        ),
        Err(JoinMetricAggregationError::TopLevelBindingMismatch { .. })
            | Err(JoinMetricAggregationError::AssessmentMismatch { .. })
            | Err(JoinMetricAggregationError::NonCanonicalMetricOrder)
            | Err(JoinMetricAggregationError::PrimaryEligibilityInconsistent)
            | Err(JoinMetricAggregationError::ReportClassInconsistent)
            | Err(JoinMetricAggregationError::CrossMetricInvariantViolation { .. })
            | Err(JoinMetricAggregationError::ZeroDenominatorValueStateMismatch { .. })
            | Err(JoinMetricAggregationError::NumeratorExceedsDenominator { .. })
    ));
}

#[test]
fn cross_metric_end_to_end_vs_localization_denominator_parity() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (contribution, mut set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    set.metrics[3].denominator_count = 99;
    assert!(matches!(
        set.validate(),
        Err(JoinMetricAggregationError::CrossMetricInvariantViolation { .. })
            | Err(JoinMetricAggregationError::AssessmentMismatch { .. })
    ));
    assert!(matches!(
        set.validate_against(
            envelope,
            seal,
            coverage,
            human_reference,
            snapshot,
            join,
            adjudication,
            &contribution,
            bundle,
        ),
        Err(JoinMetricAggregationError::TopLevelBindingMismatch { .. })
            | Err(JoinMetricAggregationError::AssessmentMismatch { .. })
            | Err(JoinMetricAggregationError::NonCanonicalMetricOrder)
            | Err(JoinMetricAggregationError::PrimaryEligibilityInconsistent)
            | Err(JoinMetricAggregationError::ReportClassInconsistent)
            | Err(JoinMetricAggregationError::CrossMetricInvariantViolation { .. })
            | Err(JoinMetricAggregationError::ZeroDenominatorValueStateMismatch { .. })
            | Err(JoinMetricAggregationError::NumeratorExceedsDenominator { .. })
    ));
}

#[test]
fn cross_metric_end_to_end_numerator_vs_correction_numerator_parity() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (contribution, mut set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    set.metrics[3].numerator_count = 99;
    assert!(matches!(
        set.validate(),
        Err(JoinMetricAggregationError::CrossMetricInvariantViolation { .. })
            | Err(JoinMetricAggregationError::NumeratorExceedsDenominator { .. })
            | Err(JoinMetricAggregationError::AssessmentMismatch { .. })
    ));
    assert!(matches!(
        set.validate_against(
            envelope,
            seal,
            coverage,
            human_reference,
            snapshot,
            join,
            adjudication,
            &contribution,
            bundle,
        ),
        Err(JoinMetricAggregationError::TopLevelBindingMismatch { .. })
            | Err(JoinMetricAggregationError::AssessmentMismatch { .. })
            | Err(JoinMetricAggregationError::NonCanonicalMetricOrder)
            | Err(JoinMetricAggregationError::PrimaryEligibilityInconsistent)
            | Err(JoinMetricAggregationError::ReportClassInconsistent)
            | Err(JoinMetricAggregationError::CrossMetricInvariantViolation { .. })
            | Err(JoinMetricAggregationError::ZeroDenominatorValueStateMismatch { .. })
            | Err(JoinMetricAggregationError::NumeratorExceedsDenominator { .. })
    ));
}

#[test]
fn cross_metric_numerator_exceeds_denominator_rejected() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (contribution, mut set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    set.metrics[0].numerator_count = 99;
    assert!(matches!(
        set.validate(),
        Err(JoinMetricAggregationError::NumeratorExceedsDenominator { .. })
    ));
    assert!(matches!(
        set.validate_against(
            envelope,
            seal,
            coverage,
            human_reference,
            snapshot,
            join,
            adjudication,
            &contribution,
            bundle,
        ),
        Err(JoinMetricAggregationError::TopLevelBindingMismatch { .. })
            | Err(JoinMetricAggregationError::AssessmentMismatch { .. })
            | Err(JoinMetricAggregationError::NonCanonicalMetricOrder)
            | Err(JoinMetricAggregationError::PrimaryEligibilityInconsistent)
            | Err(JoinMetricAggregationError::ReportClassInconsistent)
            | Err(JoinMetricAggregationError::CrossMetricInvariantViolation { .. })
            | Err(JoinMetricAggregationError::ZeroDenominatorValueStateMismatch { .. })
            | Err(JoinMetricAggregationError::NumeratorExceedsDenominator { .. })
    ));
}

// --- Eligibility ---

#[test]
fn eligibility_primary_preserved_from_contribution() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (contribution, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    assert_eq!(
        set.primary_metrics_allowed,
        contribution.eligibility.primary_metrics_allowed
    );
    assert_eq!(set.report_class, contribution.eligibility.report_class);
    assert_eq!(
        set.eligible_primary_metrics,
        contribution.eligibility.eligible_primary_metrics
    );
    assert_eq!(
        set.blocking_reasons,
        contribution.eligibility.blocking_reasons
    );
    assert!(set.qualifies_as_primary_metric_evidence);
}

#[test]
fn eligibility_diagnostic_non_primary() {
    let stack = diagnostic_metric_stack(ReferenceCoverageState::Complete);
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (_, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    assert!(!set.primary_metrics_allowed);
    assert!(!set.qualifies_as_primary_metric_evidence);
}

#[test]
fn eligibility_synthetic_non_primary() {
    let mut stack = primary_metric_stack();
    stack.0.input_class = InputClass::SyntheticProtocolFixture;
    stack.1.producer_class = ReferenceProducerClass::SyntheticFixtureGenerator;
    stack.1.calibration_classification = ReferenceCalibrationValidity::SyntheticProtocolOnly;
    stack.1.calibration_validity_impact = CalibrationValidityImpact::ProtocolOnly;
    stack.2.coverage_purpose = ReferenceCoveragePurpose::SyntheticProtocolValidation;
    stack.8.join_purpose = DetectorReferenceJoinPurpose::SyntheticProtocolValidation;
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (_, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    assert!(!set.primary_metrics_allowed);
    assert!(!set.qualifies_as_primary_metric_evidence);
}

#[test]
fn eligibility_cannot_change_blocking_report_class_or_subset() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (contribution, mut set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    set.primary_metrics_allowed = false;
    assert!(matches!(
        set.validate_against(
            envelope,
            seal,
            coverage,
            human_reference,
            snapshot,
            join,
            adjudication,
            &contribution,
            bundle,
        ),
        Err(JoinMetricAggregationError::TopLevelBindingMismatch { .. })
            | Err(JoinMetricAggregationError::AssessmentMismatch { .. })
            | Err(JoinMetricAggregationError::NonCanonicalMetricOrder)
            | Err(JoinMetricAggregationError::PrimaryEligibilityInconsistent)
            | Err(JoinMetricAggregationError::ReportClassInconsistent)
            | Err(JoinMetricAggregationError::CrossMetricInvariantViolation { .. })
            | Err(JoinMetricAggregationError::ZeroDenominatorValueStateMismatch { .. })
            | Err(JoinMetricAggregationError::NumeratorExceedsDenominator { .. })
    ));
    set = derive_aggregate_set(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        &contribution,
        &aggregate_context(),
    );
    set.report_class = MetricContributionReportClass::NonCalibrationDiagnostic;
    assert!(matches!(
        set.validate(),
        Err(JoinMetricAggregationError::ReportClassInconsistent)
            | Err(JoinMetricAggregationError::PrimaryEligibilityInconsistent)
    ));
    assert!(matches!(
        set.validate_against(
            envelope,
            seal,
            coverage,
            human_reference,
            snapshot,
            join,
            adjudication,
            &contribution,
            bundle,
        ),
        Err(JoinMetricAggregationError::TopLevelBindingMismatch { .. })
            | Err(JoinMetricAggregationError::ReportClassInconsistent)
    ));
}

#[test]
fn eligibility_zero_denominator_does_not_change_primary_eligibility() {
    let stack = derive_join_empty();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (_, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    assert!(set.primary_metrics_allowed);
    assert!(set.qualifies_as_primary_metric_evidence);
}

#[test]
fn eligibility_real_material_evidence_preserved_separately() {
    let mut stack = primary_metric_stack();
    stack.0.qualifies_as_real_material_evidence = true;
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (_, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    assert!(set.qualifies_as_real_material_evidence);
    assert!(set.primary_metrics_allowed);
}

#[test]
fn eligibility_qualifies_as_primary_metric_evidence_is_derived() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (_, mut set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    set.qualifies_as_primary_metric_evidence = false;
    assert!(matches!(
        set.validate(),
        Err(JoinMetricAggregationError::PrimaryMetricEvidenceDerivationMismatch)
    ));
}

// --- Lineage ---

#[test]
fn stored_top_level_binding_mismatch_rejects_every_authoritative_field() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (contribution, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );

    macro_rules! check_field {
        ($mutate:expr, $field:expr) => {
            assert_aggregate_top_level_binding_mismatch(
                &set,
                envelope,
                seal,
                coverage,
                human_reference,
                snapshot,
                join,
                adjudication,
                &contribution,
                bundle,
                $mutate,
                $field,
            );
        };
    }

    check_field!(
        |set| set.run_id = RunId::new("run-other").expect("run id"),
        "run_id"
    );
    check_field!(
        |set| set.input_identity.transcript_revision_id = "rev-other".to_string(),
        "input_identity"
    );
    check_field!(
        |set| set.input_class = InputClass::SyntheticProtocolFixture,
        "input_class"
    );
    check_field!(
        |set| set.qualifies_as_real_material_evidence = true,
        "qualifies_as_real_material_evidence"
    );
    check_field!(
        |set| set.reference_seal_id = ReferenceSealId::new("seal-other").expect("seal id"),
        "reference_seal_id"
    );
    check_field!(
        |set| set.reference_revision =
            ReferenceRevisionId::new("ref-rev-other").expect("reference revision"),
        "reference_revision"
    );
    check_field!(
        |set| set.reference_coverage_id =
            ReferenceCoverageId::new("coverage-other").expect("coverage id"),
        "reference_coverage_id"
    );
    check_field!(
        |set| set.detector_snapshot_revision =
            DetectorSnapshotRevisionId::new("snap-rev-other").expect("snapshot revision"),
        "detector_snapshot_revision"
    );
    check_field!(
        |set| set.detector_output_artifact_id =
            ArtifactId::new("detector-output-other").expect("artifact id"),
        "detector_output_artifact_id"
    );
    check_field!(
        |set| set.join_id = DetectorReferenceJoinId::new("join-other").expect("join id"),
        "join_id"
    );
    check_field!(
        |set| set.join_revision =
            DetectorReferenceJoinRevisionId::new("join-rev-other").expect("join revision"),
        "join_revision"
    );
    check_field!(
        |set| set.evaluation_join_artifact_id =
            ArtifactId::new("evaluation-join-other").expect("artifact id"),
        "evaluation_join_artifact_id"
    );
    check_field!(
        |set| set.join_adjudication_artifact_id =
            ArtifactId::new("join-adj-other").expect("artifact id"),
        "join_adjudication_artifact_id"
    );
    check_field!(
        |set| set.contribution_set_id =
            MetricContributionSetId::new("contrib-set-other").expect("id"),
        "contribution_set_id"
    );
    check_field!(
        |set| set.contribution_revision =
            MetricContributionRevisionId::new("contrib-rev-other").expect("id"),
        "contribution_revision"
    );
    check_field!(
        |set| set.metric_contributions_artifact_id =
            ArtifactId::new("metric-contrib-other").expect("artifact id"),
        "metric_contributions_artifact_id"
    );
    let mut metrics_artifact_mismatch = set.clone();
    metrics_artifact_mismatch.metrics_artifact_id =
        ArtifactId::new("metrics-other").expect("artifact id");
    assert!(matches!(
        metrics_artifact_mismatch.validate_against(
            envelope,
            seal,
            coverage,
            human_reference,
            snapshot,
            join,
            adjudication,
            &contribution,
            bundle,
        ),
        Err(JoinMetricAggregationError::MetricsArtifactMismatch)
            | Err(JoinMetricAggregationError::TopLevelBindingMismatch { .. })
    ));
    let mut bad_agg_policy = set.clone();
    bad_agg_policy.aggregation_policy_revision = "other-policy".to_string();
    assert!(matches!(
        bad_agg_policy.validate(),
        Err(JoinMetricAggregationError::UnsupportedPolicyRevision { .. })
    ));
    let mut bad_zero_policy = set.clone();
    bad_zero_policy.zero_denominator_policy_revision = "other-zero-policy".to_string();
    assert!(matches!(
        bad_zero_policy.validate(),
        Err(JoinMetricAggregationError::UnsupportedPolicyRevision { .. })
    ));
    let mut bad_primary_allowed = set.clone();
    bad_primary_allowed.primary_metrics_allowed = false;
    assert!(matches!(
        bad_primary_allowed.validate(),
        Err(JoinMetricAggregationError::PrimaryEligibilityInconsistent)
    ));
    assert_validate_against_rejects(
        &bad_primary_allowed,
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        &contribution,
        bundle,
    );
    check_field!(
        |set| set.eligible_primary_metrics.clear(),
        "eligible_primary_metrics"
    );
    check_field!(
        |set| set
            .blocking_reasons
            .push(PrimaryMetricBlockingReason::SyntheticProtocolOnly),
        "blocking_reasons"
    );
    check_field!(
        |set| set.qualifies_as_primary_metric_evidence = false,
        "qualifies_as_primary_metric_evidence"
    );
    let mut bad_report_class = set.clone();
    bad_report_class.report_class = MetricContributionReportClass::NonCalibrationDiagnostic;
    assert!(matches!(
        bad_report_class.validate(),
        Err(JoinMetricAggregationError::ReportClassInconsistent)
    ));
    assert_validate_against_rejects(
        &bad_report_class,
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        &contribution,
        bundle,
    );
    check_field!(
        |set| set.state = MetricAggregateSetState::Invalidated,
        "state"
    );
    check_field!(|set| set.metrics[0].numerator_count = 99, "metrics");
    check_field!(|set| set.assessment.defined_metric_count = 0, "assessment");
    let mut bad_schema = set.clone();
    bad_schema.schema_revision = "voxproof-other-schema".to_string();
    assert!(matches!(
        bad_schema.validate(),
        Err(JoinMetricAggregationError::UnsupportedSchemaRevision { .. })
    ));
    assert_validate_against_rejects(
        &bad_schema,
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        &contribution,
        bundle,
    );
}

// --- Artifact bundle ---

#[test]
fn bundle_must_include_exactly_one_metric_contributions_and_metrics_roles() {
    let stack = primary_metric_stack();
    let (envelope, seal, coverage, human_reference, snapshot, contrib_ctx, adjudication, join) = (
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.6, &stack.7, &stack.8,
    );
    let contribution = derive_contribution_set(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        &stack.5,
        contrib_ctx,
    );

    let mut missing_metrics = stack.5.clone();
    missing_metrics
        .artifacts
        .retain(|descriptor| descriptor.role != ArtifactRole::Metrics);
    assert!(
        JoinMetricAggregateSet::derive(
            &aggregate_context(),
            envelope,
            seal,
            coverage,
            human_reference,
            snapshot,
            join,
            adjudication,
            &contribution,
            &missing_metrics,
        )
        .is_err(),
        "missing Metrics role must reject aggregation derive"
    );

    let mut duplicate_metrics = stack.5.clone();
    duplicate_metrics.artifacts.push(descriptor(
        &duplicate_metrics.binding_context,
        ArtifactRole::Metrics,
        "artifact-metrics-duplicate",
    ));
    assert!(
        JoinMetricAggregateSet::derive(
            &aggregate_context(),
            envelope,
            seal,
            coverage,
            human_reference,
            snapshot,
            join,
            adjudication,
            &contribution,
            &duplicate_metrics,
        )
        .is_err(),
        "duplicate Metrics role must reject aggregation derive"
    );

    let mut mismatched_metrics = stack.5.clone();
    for descriptor in &mut mismatched_metrics.artifacts {
        if descriptor.role == ArtifactRole::Metrics {
            descriptor.artifact_id =
                ArtifactId::new("artifact-metrics-wrong").expect("artifact id");
        }
    }
    assert!(
        JoinMetricAggregateSet::derive(
            &aggregate_context(),
            envelope,
            seal,
            coverage,
            human_reference,
            snapshot,
            join,
            adjudication,
            &contribution,
            &mismatched_metrics,
        )
        .is_err(),
        "mismatched Metrics artifact id must reject aggregation derive"
    );
}

// --- Stored validation ---

#[test]
fn stored_numerator_mutation_rejected_on_validate_against() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (contribution, mut set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    set.metrics[0].numerator_count += 1;
    assert!(matches!(
        set.validate_against(
            envelope,
            seal,
            coverage,
            human_reference,
            snapshot,
            join,
            adjudication,
            &contribution,
            bundle,
        ),
        Err(JoinMetricAggregationError::TopLevelBindingMismatch { .. })
            | Err(JoinMetricAggregationError::AssessmentMismatch { .. })
            | Err(JoinMetricAggregationError::NonCanonicalMetricOrder)
            | Err(JoinMetricAggregationError::PrimaryEligibilityInconsistent)
            | Err(JoinMetricAggregationError::ReportClassInconsistent)
            | Err(JoinMetricAggregationError::CrossMetricInvariantViolation { .. })
            | Err(JoinMetricAggregationError::ZeroDenominatorValueStateMismatch { .. })
            | Err(JoinMetricAggregationError::NumeratorExceedsDenominator { .. })
    ));
}

#[test]
fn stored_denominator_mutation_rejected_on_validate_against() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (contribution, mut set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    set.metrics[1].denominator_count += 1;
    assert!(matches!(
        set.validate_against(
            envelope,
            seal,
            coverage,
            human_reference,
            snapshot,
            join,
            adjudication,
            &contribution,
            bundle,
        ),
        Err(JoinMetricAggregationError::TopLevelBindingMismatch { .. })
            | Err(JoinMetricAggregationError::AssessmentMismatch { .. })
            | Err(JoinMetricAggregationError::NonCanonicalMetricOrder)
            | Err(JoinMetricAggregationError::PrimaryEligibilityInconsistent)
            | Err(JoinMetricAggregationError::ReportClassInconsistent)
            | Err(JoinMetricAggregationError::CrossMetricInvariantViolation { .. })
            | Err(JoinMetricAggregationError::ZeroDenominatorValueStateMismatch { .. })
            | Err(JoinMetricAggregationError::NumeratorExceedsDenominator { .. })
    ));
}

#[test]
fn stored_value_state_mutation_rejected_on_validate_against() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (contribution, mut set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    set.metrics[0].value_state = MetricAggregateValueState::UndefinedZeroDenominator;
    assert_validate_against_rejects(
        &set,
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        &contribution,
        bundle,
    );
}

#[test]
fn stored_metric_order_mutation_rejected_on_validate_against() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (contribution, mut set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    set.metrics.swap(0, 1);
    assert!(matches!(
        set.validate_against(
            envelope,
            seal,
            coverage,
            human_reference,
            snapshot,
            join,
            adjudication,
            &contribution,
            bundle,
        ),
        Err(JoinMetricAggregationError::TopLevelBindingMismatch { .. })
            | Err(JoinMetricAggregationError::AssessmentMismatch { .. })
            | Err(JoinMetricAggregationError::NonCanonicalMetricOrder)
            | Err(JoinMetricAggregationError::PrimaryEligibilityInconsistent)
            | Err(JoinMetricAggregationError::ReportClassInconsistent)
            | Err(JoinMetricAggregationError::CrossMetricInvariantViolation { .. })
            | Err(JoinMetricAggregationError::ZeroDenominatorValueStateMismatch { .. })
            | Err(JoinMetricAggregationError::NumeratorExceedsDenominator { .. })
    ));
}

#[test]
fn stored_metric_kind_mutation_rejected_on_validate_against() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (contribution, mut set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    set.metrics[0].metric_kind = PrimaryMetricKind::DuplicateProposalBurden;
    assert!(matches!(
        set.validate(),
        Err(JoinMetricAggregationError::NonCanonicalMetricOrder)
            | Err(JoinMetricAggregationError::DuplicateMetricKind { .. })
    ));
    assert!(matches!(
        set.validate_against(
            envelope,
            seal,
            coverage,
            human_reference,
            snapshot,
            join,
            adjudication,
            &contribution,
            bundle,
        ),
        Err(JoinMetricAggregationError::TopLevelBindingMismatch { .. })
            | Err(JoinMetricAggregationError::AssessmentMismatch { .. })
            | Err(JoinMetricAggregationError::NonCanonicalMetricOrder)
            | Err(JoinMetricAggregationError::PrimaryEligibilityInconsistent)
            | Err(JoinMetricAggregationError::ReportClassInconsistent)
            | Err(JoinMetricAggregationError::CrossMetricInvariantViolation { .. })
            | Err(JoinMetricAggregationError::ZeroDenominatorValueStateMismatch { .. })
            | Err(JoinMetricAggregationError::NumeratorExceedsDenominator { .. })
    ));
}

#[test]
fn stored_assessment_mutation_rejected_on_validate_against() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (contribution, mut set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    set.assessment.defined_metric_count = 0;
    assert!(matches!(
        set.validate(),
        Err(JoinMetricAggregationError::AssessmentMismatch { .. })
    ));
    assert!(matches!(
        set.validate_against(
            envelope,
            seal,
            coverage,
            human_reference,
            snapshot,
            join,
            adjudication,
            &contribution,
            bundle,
        ),
        Err(JoinMetricAggregationError::TopLevelBindingMismatch { .. })
            | Err(JoinMetricAggregationError::AssessmentMismatch { .. })
            | Err(JoinMetricAggregationError::NonCanonicalMetricOrder)
            | Err(JoinMetricAggregationError::PrimaryEligibilityInconsistent)
            | Err(JoinMetricAggregationError::ReportClassInconsistent)
            | Err(JoinMetricAggregationError::CrossMetricInvariantViolation { .. })
            | Err(JoinMetricAggregationError::ZeroDenominatorValueStateMismatch { .. })
            | Err(JoinMetricAggregationError::NumeratorExceedsDenominator { .. })
    ));
}

#[test]
fn stored_report_class_mutation_rejected_on_validate_against() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (contribution, mut set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    set.report_class = MetricContributionReportClass::NonCalibrationDiagnostic;
    assert!(matches!(
        set.validate(),
        Err(JoinMetricAggregationError::ReportClassInconsistent)
            | Err(JoinMetricAggregationError::PrimaryEligibilityInconsistent)
    ));
    assert!(matches!(
        set.validate_against(
            envelope,
            seal,
            coverage,
            human_reference,
            snapshot,
            join,
            adjudication,
            &contribution,
            bundle,
        ),
        Err(JoinMetricAggregationError::TopLevelBindingMismatch { .. })
            | Err(JoinMetricAggregationError::ReportClassInconsistent)
    ));
}

#[test]
fn stored_eligibility_mutation_rejected_on_validate_against() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (contribution, mut set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    set.eligible_primary_metrics.pop();
    assert!(matches!(
        set.validate_against(
            envelope,
            seal,
            coverage,
            human_reference,
            snapshot,
            join,
            adjudication,
            &contribution,
            bundle,
        ),
        Err(JoinMetricAggregationError::TopLevelBindingMismatch { .. })
            | Err(JoinMetricAggregationError::AssessmentMismatch { .. })
            | Err(JoinMetricAggregationError::NonCanonicalMetricOrder)
            | Err(JoinMetricAggregationError::PrimaryEligibilityInconsistent)
            | Err(JoinMetricAggregationError::ReportClassInconsistent)
            | Err(JoinMetricAggregationError::CrossMetricInvariantViolation { .. })
            | Err(JoinMetricAggregationError::ZeroDenominatorValueStateMismatch { .. })
            | Err(JoinMetricAggregationError::NumeratorExceedsDenominator { .. })
    ));
}

#[test]
fn stored_state_mutation_rejected_on_validate_against() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (contribution, mut set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    set.state = MetricAggregateSetState::Invalidated;
    assert!(matches!(
        set.validate_against(
            envelope,
            seal,
            coverage,
            human_reference,
            snapshot,
            join,
            adjudication,
            &contribution,
            bundle,
        ),
        Err(JoinMetricAggregationError::InvalidatedAggregateContext)
    ));
}

#[test]
fn stored_lineage_ids_mutation_rejected_on_validate_against() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (contribution, mut set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    set.contribution_set_id = MetricContributionSetId::new("metric-contrib-set-002").expect("id");
    assert_validate_against_rejects(
        &set,
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        &contribution,
        bundle,
    );
}

// --- Lifecycle ---

#[test]
fn derive_at_detector_execution_lifecycle() {
    let stack = derive_join_exact_match();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    assert_eq!(
        envelope.lifecycle_state,
        RunLifecycleState::DetectorExecution
    );
    let (_, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    assert_eq!(set.state, MetricAggregateSetState::Complete);
}

#[test]
fn derive_at_assisted_review_lifecycle() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    assert_eq!(envelope.lifecycle_state, RunLifecycleState::AssistedReview);
    let (_, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    assert_eq!(set.state, MetricAggregateSetState::Complete);
}

#[test]
fn historical_validate_against_accepts_finalized_lifecycle() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (contribution, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    let mut finalized = envelope.clone();
    finalized.lifecycle_state = RunLifecycleState::Finalized;
    set.validate_against(
        &finalized,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        &contribution,
        bundle,
    )
    .expect("historical validation at Finalized");
}

#[test]
fn derive_at_finalized_lifecycle_fails() {
    let stack = primary_metric_stack();
    let (
        mut envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = (
        stack.0.clone(),
        &stack.1,
        &stack.2,
        &stack.3,
        &stack.4,
        &stack.5,
        &stack.6,
        &stack.7,
        &stack.8,
    );
    envelope.lifecycle_state = RunLifecycleState::Finalized;
    let contribution = derive_contribution_set(
        &stack.0,
        &stack.1,
        &stack.2,
        &stack.3,
        &stack.4,
        &stack.8,
        &stack.7,
        &stack.5,
        contrib_ctx,
    );
    assert!(matches!(
        JoinMetricAggregateSet::derive(
            &aggregate_context(),
            &envelope,
            seal,
            coverage,
            human_reference,
            snapshot,
            join,
            adjudication,
            &contribution,
            bundle,
        ),
        Err(JoinMetricAggregationError::AggregateCreationLifecycleIncompatible { .. })
    ));
}

#[test]
fn reference_sealed_lifecycle_too_early_for_derive() {
    let stack = primary_metric_stack();
    let (
        mut envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = (
        stack.0.clone(),
        &stack.1,
        &stack.2,
        &stack.3,
        &stack.4,
        &stack.5,
        &stack.6,
        &stack.7,
        &stack.8,
    );
    envelope.lifecycle_state = RunLifecycleState::ReferenceSealed;
    let contribution = derive_contribution_set(
        &stack.0,
        &stack.1,
        &stack.2,
        &stack.3,
        &stack.4,
        &stack.8,
        &stack.7,
        &stack.5,
        contrib_ctx,
    );
    assert!(matches!(
        JoinMetricAggregateSet::derive(
            &aggregate_context(),
            &envelope,
            seal,
            coverage,
            human_reference,
            snapshot,
            join,
            adjudication,
            &contribution,
            bundle,
        ),
        Err(JoinMetricAggregationError::AggregateCreationLifecycleIncompatible { .. })
    ));
}

#[test]
fn invalidated_envelope_context_rejected() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (contribution, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    let mut invalidated = envelope.clone();
    invalidated.lifecycle_state = RunLifecycleState::Invalidated;
    assert!(matches!(
        set.validate_against(
            &invalidated,
            seal,
            coverage,
            human_reference,
            snapshot,
            join,
            adjudication,
            &contribution,
            bundle,
        ),
        Err(JoinMetricAggregationError::InvalidatedAggregateContext)
            | Err(JoinMetricAggregationError::AggregateHistoricalLifecycleIncompatible { .. })
    ));
}

// --- Scope guards ---

#[test]
fn scope_guard_forbidden_performance_fields_absent_from_json() {
    let stack = primary_metric_stack();
    let (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        contrib_ctx,
        adjudication,
        join,
    ) = stack_tuple(&stack);
    let (_, set) = derive_contribution_and_aggregate(
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        join,
        adjudication,
        bundle,
        contrib_ctx,
        &aggregate_context(),
    );
    let json = aggregate_to_json(&set).expect("serialize");
    for token in [
        "\"float\"",
        "\"decimal\"",
        "\"percentage\"",
        "\"threshold\"",
        "\"true_positive\"",
        "\"false_positive\"",
        "\"false_negative\"",
        "\"pass\"",
        "\"fail\"",
        "0.5",
        "1.0",
    ] {
        assert!(
            !json.contains(token),
            "scope guard violated: found {token} in aggregate JSON"
        );
    }
}

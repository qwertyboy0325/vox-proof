use vox_proof::artifact_bundle::{
    ARTIFACT_BUNDLE_SCHEMA, ArtifactBindingContext, ArtifactBundle, ArtifactBundleId,
    ArtifactBundleState, ArtifactContentDigest, ArtifactDescriptor, ArtifactId,
    ArtifactSchemaIdentity,
};
use vox_proof::candidate::DetectionKind;
use vox_proof::detector_reference_join::{
    DetectorReferenceJoin, DetectorReferenceJoinContext, DetectorReferenceJoinId,
    DetectorReferenceJoinPurpose, DetectorReferenceJoinRevisionId, DetectorReferenceJoinState,
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
use vox_proof::join_metric_contribution::{
    JOIN_METRIC_CONTRIBUTION_SCHEMA, JoinMetricContributionContext, JoinMetricContributionError,
    JoinMetricContributionSet, METRIC_CONTRIBUTION_POLICY, MetricContributionExclusionReason,
    MetricContributionReportClass, MetricContributionRevisionId, MetricContributionSetAssessment,
    MetricContributionSetId, MetricContributionSetState, PRIMARY_METRIC_ELIGIBILITY_POLICY,
    PrimaryMetricBlockingReason, PrimaryMetricKind, RatioContribution, contribution_from_json,
    contribution_to_json, validate_contribution_id_value,
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

fn build_metric_contribution_bundle(
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
    let bundle = build_metric_contribution_bundle(binding_context, ArtifactBundleState::Complete);
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
    let bundle = build_metric_contribution_bundle(binding_context, ArtifactBundleState::Complete);
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
    let bundle = build_metric_contribution_bundle(binding_context, ArtifactBundleState::Complete);
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

// --- Schema / serialization ---

#[test]
fn json_round_trip_retains_schema_and_policy_constants() {
    let (envelope, seal, coverage, human_reference, snapshot, bundle, context, adjudication, join) =
        primary_metric_stack();
    let set = derive_contribution_set(
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &join,
        &adjudication,
        &bundle,
        &context,
    );

    let json = contribution_to_json(&set).expect("serialize");
    assert!(json.contains(JOIN_METRIC_CONTRIBUTION_SCHEMA));
    assert!(json.contains(PRIMARY_METRIC_ELIGIBILITY_POLICY));
    assert!(json.contains(METRIC_CONTRIBUTION_POLICY));

    let restored = contribution_from_json(&json).expect("deserialize");
    assert_eq!(restored.schema_revision, JOIN_METRIC_CONTRIBUTION_SCHEMA);
    assert_eq!(
        restored.eligibility_policy_revision,
        PRIMARY_METRIC_ELIGIBILITY_POLICY
    );
    assert_eq!(
        restored.contribution_policy_revision,
        METRIC_CONTRIBUTION_POLICY
    );
    restored.validate().expect("valid contribution set");
}

#[test]
fn primary_metric_kind_enum_spellings() {
    let json = serde_json::to_string(&PrimaryMetricKind::ProposalPrecision).expect("serialize");
    assert_eq!(json, "\"proposal_precision\"");
    let json =
        serde_json::to_string(&PrimaryMetricKind::ErrorLocalizationRecall).expect("serialize");
    assert_eq!(json, "\"error_localization_recall\"");
    let json = serde_json::to_string(&PrimaryMetricKind::CorrectionExactnessGivenLocalization)
        .expect("serialize");
    assert_eq!(json, "\"correction_exactness_given_localization\"");
    let json = serde_json::to_string(&PrimaryMetricKind::EndToEndExactCorrectionRecall)
        .expect("serialize");
    assert_eq!(json, "\"end_to_end_exact_correction_recall\"");
    let json =
        serde_json::to_string(&PrimaryMetricKind::DuplicateProposalBurden).expect("serialize");
    assert_eq!(json, "\"duplicate_proposal_burden\"");
}

#[test]
fn artifact_role_metric_contributions_vs_metrics_spellings() {
    let contributions =
        serde_json::to_string(&ArtifactRole::MetricContributions).expect("serialize");
    assert_eq!(contributions, "\"metric_contributions\"");
    let metrics = serde_json::to_string(&ArtifactRole::Metrics).expect("serialize");
    assert_eq!(metrics, "\"metrics\"");
    assert_ne!(contributions, metrics);
}

#[test]
fn unknown_top_level_field_rejected_on_round_trip() {
    let (envelope, seal, coverage, human_reference, snapshot, bundle, context, adjudication, join) =
        primary_metric_stack();
    let set = derive_contribution_set(
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &join,
        &adjudication,
        &bundle,
        &context,
    );
    let mut value = serde_json::to_value(&set).expect("value");
    value.as_object_mut().expect("object").insert(
        "transcript_text".to_string(),
        serde_json::json!("forbidden"),
    );

    let error = serde_json::from_value::<JoinMetricContributionSet>(value).expect_err("must fail");
    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn invalid_and_path_like_contribution_ids_rejected() {
    for value in [
        "",
        "/Users/example/private/metric-contrib.json",
        "../private/metric-contrib.json",
    ] {
        assert!(
            MetricContributionSetId::new(value).is_err(),
            "contribution set id must reject {value:?}"
        );
        assert!(
            MetricContributionRevisionId::new(value).is_err(),
            "contribution revision id must reject {value:?}"
        );
        assert!(
            validate_contribution_id_value(value).is_err(),
            "validate_contribution_id_value must reject {value:?}"
        );
    }
}

#[test]
fn serialized_contribution_contains_no_transcript_cue_correction_or_pii_fields() {
    let (envelope, seal, coverage, human_reference, snapshot, bundle, context, adjudication, join) =
        primary_metric_stack();
    let set = derive_contribution_set(
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &join,
        &adjudication,
        &bundle,
        &context,
    );
    let json = contribution_to_json(&set).expect("serialize");
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
    ] {
        assert!(
            value.get(forbidden).is_none(),
            "serialized contribution must not contain {forbidden}"
        );
    }
    assert!(!json.contains("wrng"));
    assert!(!json.contains("wrong"));
}

// --- Top-level lineage ---

#[test]
fn stored_top_level_binding_mismatch_rejects_every_authoritative_field() {
    let (envelope, seal, coverage, human_reference, snapshot, bundle, context, adjudication, join) =
        primary_metric_stack();
    let set = derive_contribution_set(
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &join,
        &adjudication,
        &bundle,
        &context,
    );

    assert_top_level_binding_mismatch(
        &set,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &join,
        &adjudication,
        &bundle,
        |set| set.run_id = RunId::new("run-other").expect("run id"),
        "run_id",
    );
    assert_top_level_binding_mismatch(
        &set,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &join,
        &adjudication,
        &bundle,
        |set| set.input_identity.transcript_revision_id = "rev-other".to_string(),
        "input_identity",
    );
    assert_top_level_binding_mismatch(
        &set,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &join,
        &adjudication,
        &bundle,
        |set| set.input_class = InputClass::SyntheticProtocolFixture,
        "input_class",
    );
    assert_top_level_binding_mismatch(
        &set,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &join,
        &adjudication,
        &bundle,
        |set| set.qualifies_as_real_material_evidence = true,
        "qualifies_as_real_material_evidence",
    );
    assert_top_level_binding_mismatch(
        &set,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &join,
        &adjudication,
        &bundle,
        |set| set.reference_seal_id = ReferenceSealId::new("seal-other").expect("seal id"),
        "reference_seal_id",
    );
    assert_top_level_binding_mismatch(
        &set,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &join,
        &adjudication,
        &bundle,
        |set| {
            set.reference_revision =
                ReferenceRevisionId::new("ref-rev-other").expect("reference revision")
        },
        "reference_revision",
    );
    assert_top_level_binding_mismatch(
        &set,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &join,
        &adjudication,
        &bundle,
        |set| {
            set.reference_coverage_id =
                ReferenceCoverageId::new("coverage-other").expect("coverage id")
        },
        "reference_coverage_id",
    );
    assert_top_level_binding_mismatch(
        &set,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &join,
        &adjudication,
        &bundle,
        |set| {
            set.detector_snapshot_revision =
                DetectorSnapshotRevisionId::new("snap-rev-other").expect("snapshot revision")
        },
        "detector_snapshot_revision",
    );
    assert_top_level_binding_mismatch(
        &set,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &join,
        &adjudication,
        &bundle,
        |set| {
            set.detector_output_artifact_id =
                ArtifactId::new("detector-output-other").expect("artifact id")
        },
        "detector_output_artifact_id",
    );
    assert_top_level_binding_mismatch(
        &set,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &join,
        &adjudication,
        &bundle,
        |set| set.join_id = DetectorReferenceJoinId::new("join-other").expect("join id"),
        "join_id",
    );
    assert_top_level_binding_mismatch(
        &set,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &join,
        &adjudication,
        &bundle,
        |set| {
            set.join_revision =
                DetectorReferenceJoinRevisionId::new("join-rev-other").expect("join revision")
        },
        "join_revision",
    );
    assert_top_level_binding_mismatch(
        &set,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &join,
        &adjudication,
        &bundle,
        |set| {
            set.evaluation_join_artifact_id =
                ArtifactId::new("evaluation-join-other").expect("artifact id")
        },
        "evaluation_join_artifact_id",
    );
    assert_top_level_binding_mismatch(
        &set,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &join,
        &adjudication,
        &bundle,
        |set| {
            set.join_adjudication_artifact_id =
                ArtifactId::new("join-adj-other").expect("artifact id")
        },
        "join_adjudication_artifact_id",
    );
    assert_top_level_binding_mismatch(
        &set,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &join,
        &adjudication,
        &bundle,
        |set| {
            set.join_adjudication_artifact_id =
                ArtifactId::new("join-adj-other").expect("artifact id")
        },
        "join_adjudication_artifact_id",
    );
    let mut artifact_mismatch = set.clone();
    artifact_mismatch.metric_contributions_artifact_id =
        ArtifactId::new("metric-contrib-other").expect("artifact id");
    assert!(matches!(
        artifact_mismatch.validate_against(
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &join,
            &adjudication,
            &bundle,
        ),
        Err(JoinMetricContributionError::MetricContributionsArtifactMismatch)
    ));
    let mut unsupported_policy = set.clone();
    unsupported_policy.eligibility_policy_revision = "other-eligibility-policy".to_string();
    assert!(matches!(
        unsupported_policy.validate(),
        Err(JoinMetricContributionError::UnsupportedPolicyRevision { .. })
    ));
    let mut unsupported_contribution_policy = set.clone();
    unsupported_contribution_policy.contribution_policy_revision =
        "other-contribution-policy".to_string();
    assert!(matches!(
        unsupported_contribution_policy.validate(),
        Err(JoinMetricContributionError::UnsupportedPolicyRevision { .. })
    ));
}

// --- Primary eligibility ---

#[test]
fn valid_complete_primary_stack_allows_all_five_metrics_in_canonical_order() {
    let (envelope, seal, coverage, human_reference, snapshot, bundle, context, adjudication, join) =
        primary_metric_stack();
    let set = derive_contribution_set(
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &join,
        &adjudication,
        &bundle,
        &context,
    );

    assert_eq!(set.state, MetricContributionSetState::Complete);
    assert!(set.eligibility.primary_metrics_allowed);
    assert_eq!(
        set.eligibility.eligible_primary_metrics,
        canonical_primary_metrics()
    );
    assert_eq!(
        set.eligibility.report_class,
        MetricContributionReportClass::PrimaryBlindCalibration
    );
    assert!(set.eligibility.blocking_reasons.is_empty());
    validate_contribution_against_stack(
        &set,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &join,
        &adjudication,
        &bundle,
    );
}

#[test]
fn detector_assisted_envelope_blocks_primary_metrics() {
    let stack = primary_metric_stack();
    let mut envelope = stack.0.clone();
    envelope.calibration_validity = CalibrationValidityMode::DetectorAssisted;
    let mut snapshot = stack.4.clone();
    snapshot.calibration_validity = CalibrationValidityMode::DetectorAssisted;

    let result = JoinMetricContributionSet::derive(
        &stack.6, &envelope, &stack.1, &stack.2, &stack.3, &snapshot, &stack.8, &stack.7, &stack.5,
    );
    match result {
        Err(
            JoinMetricContributionError::EnvelopeValidation(_)
            | JoinMetricContributionError::JoinValidation(_)
            | JoinMetricContributionError::BundleValidation(_)
            | JoinMetricContributionError::SnapshotValidation(_)
            | JoinMetricContributionError::SealValidation(_),
        ) => {}
        Ok(set) => {
            assert!(!set.eligibility.primary_metrics_allowed);
            assert!(
                set.eligibility
                    .blocking_reasons
                    .contains(&PrimaryMetricBlockingReason::EnvelopeNotBlindReference)
            );
        }
        Err(other) => panic!("unexpected derive error: {other:?}"),
    }
}

#[test]
fn term_conditioned_diagnostic_seal_blocks_primary_metrics() {
    let stack = diagnostic_metric_stack(ReferenceCoverageState::Complete);
    let set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    assert!(!set.eligibility.primary_metrics_allowed);
    assert!(
        set.eligibility
            .blocking_reasons
            .contains(&PrimaryMetricBlockingReason::ReferenceNotBlindEligible)
    );
    assert!(
        set.eligibility
            .blocking_reasons
            .contains(&PrimaryMetricBlockingReason::ReferenceValidityImpactNotNone)
    );
}

#[test]
fn detector_contaminated_seal_blocks_primary_metrics() {
    let stack = contaminated_metric_stack();
    let set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    assert!(!set.eligibility.primary_metrics_allowed);
    assert!(
        set.eligibility
            .blocking_reasons
            .contains(&PrimaryMetricBlockingReason::ReferenceNotBlindEligible)
    );
}

#[test]
fn synthetic_protocol_posture_blocks_primary_metrics() {
    let (
        mut envelope,
        mut seal,
        mut coverage,
        human_reference,
        snapshot,
        bundle,
        context,
        adjudication,
        mut join,
    ) = primary_metric_stack();
    envelope.input_class = InputClass::SyntheticProtocolFixture;
    seal.producer_class = ReferenceProducerClass::SyntheticFixtureGenerator;
    seal.calibration_classification = ReferenceCalibrationValidity::SyntheticProtocolOnly;
    seal.calibration_validity_impact = CalibrationValidityImpact::ProtocolOnly;
    coverage.coverage_purpose = ReferenceCoveragePurpose::SyntheticProtocolValidation;
    join.join_purpose = DetectorReferenceJoinPurpose::SyntheticProtocolValidation;

    let set = derive_contribution_set(
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &join,
        &adjudication,
        &bundle,
        &context,
    );
    assert!(!set.eligibility.primary_metrics_allowed);
    assert!(
        set.eligibility
            .blocking_reasons
            .contains(&PrimaryMetricBlockingReason::SyntheticProtocolOnly)
    );
    assert_eq!(
        set.eligibility.report_class,
        MetricContributionReportClass::SyntheticProtocolValidation
    );
}

#[test]
fn diagnostic_coverage_and_join_block_primary_metrics() {
    let stack = diagnostic_metric_stack(ReferenceCoverageState::Complete);
    let set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    assert!(!set.eligibility.primary_metrics_allowed);
    assert!(
        set.eligibility
            .blocking_reasons
            .contains(&PrimaryMetricBlockingReason::CoverageNotPrimary)
    );
    assert!(
        set.eligibility
            .blocking_reasons
            .contains(&PrimaryMetricBlockingReason::JoinNotPrimary)
    );
    assert_eq!(
        set.eligibility.report_class,
        MetricContributionReportClass::NonCalibrationDiagnostic
    );
}

#[test]
fn incomplete_coverage_blocks_primary_metrics() {
    let stack = diagnostic_metric_stack(ReferenceCoverageState::Draft);
    let set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    assert!(!set.eligibility.primary_metrics_allowed);
    assert!(
        set.eligibility
            .blocking_reasons
            .contains(&PrimaryMetricBlockingReason::CoverageIncomplete)
    );
}

#[test]
fn requires_adjudication_join_blocks_primary_metrics() {
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
    let bundle = build_metric_contribution_bundle(binding_context, ArtifactBundleState::Complete);
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
    assert_eq!(join.state, DetectorReferenceJoinState::RequiresAdjudication);

    let set = derive_contribution_set(
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
    assert_eq!(set.state, MetricContributionSetState::PendingJoinResolution);
    assert!(!set.eligibility.primary_metrics_allowed);
    assert!(
        set.eligibility
            .blocking_reasons
            .contains(&PrimaryMetricBlockingReason::JoinNotResolved)
    );
    assert!(
        set.eligibility
            .blocking_reasons
            .contains(&PrimaryMetricBlockingReason::ContributionSetPending)
    );
}

#[test]
fn pending_contribution_blocks_primary_metrics() {
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
    let bundle = build_metric_contribution_bundle(binding_context, ArtifactBundleState::Complete);
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
    let set = derive_contribution_set(
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
    assert_eq!(set.state, MetricContributionSetState::PendingJoinResolution);
    assert!(!set.eligibility.primary_metrics_allowed);
    assert!(
        set.eligibility
            .blocking_reasons
            .contains(&PrimaryMetricBlockingReason::ContributionSetPending)
    );
}

#[test]
fn caller_cannot_force_primary_metrics_allowed() {
    let (envelope, seal, coverage, human_reference, snapshot, bundle, context, adjudication, join) =
        primary_metric_stack();
    let mut set = derive_contribution_set(
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &join,
        &adjudication,
        &bundle,
        &context,
    );
    set.eligibility.primary_metrics_allowed = false;

    assert!(matches!(
        set.validate(),
        Err(JoinMetricContributionError::PrimaryEligibilityInconsistent)
    ));
}

#[test]
fn caller_cannot_remove_blocking_reasons() {
    let stack = diagnostic_metric_stack(ReferenceCoverageState::Complete);
    let mut set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    set.eligibility.blocking_reasons.clear();

    assert!(matches!(
        set.validate(),
        Err(JoinMetricContributionError::PrimaryEligibilityInconsistent)
    ));
}

#[test]
fn caller_cannot_subset_eligible_primary_metrics() {
    let (envelope, seal, coverage, human_reference, snapshot, bundle, context, adjudication, join) =
        primary_metric_stack();
    let mut set = derive_contribution_set(
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &join,
        &adjudication,
        &bundle,
        &context,
    );
    set.eligibility.eligible_primary_metrics = vec![PrimaryMetricKind::ProposalPrecision];

    assert!(matches!(
        set.validate(),
        Err(JoinMetricContributionError::PrimaryEligibilityInconsistent)
    ));
}

#[test]
fn qualifies_as_real_material_evidence_preserved_separately_from_primary_eligibility() {
    let stack = diagnostic_metric_stack(ReferenceCoverageState::Complete);
    let mut envelope = stack.0.clone();
    envelope.qualifies_as_real_material_evidence = true;
    let set = derive_contribution_set(
        &envelope, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    assert!(!set.eligibility.primary_metrics_allowed);
    assert!(set.eligibility.qualifies_as_real_material_evidence);
    assert!(set.qualifies_as_real_material_evidence);
}

#[test]
fn diagnostic_complete_set_is_non_primary() {
    let stack = diagnostic_metric_stack(ReferenceCoverageState::Complete);
    let set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    assert_eq!(set.state, MetricContributionSetState::Complete);
    assert!(!set.eligibility.primary_metrics_allowed);
    assert_eq!(
        set.eligibility.report_class,
        MetricContributionReportClass::NonCalibrationDiagnostic
    );
}

#[test]
fn synthetic_complete_set_is_protocol_only() {
    let (
        mut envelope,
        mut seal,
        mut coverage,
        human_reference,
        snapshot,
        bundle,
        context,
        adjudication,
        mut join,
    ) = primary_metric_stack();
    envelope.input_class = InputClass::SyntheticProtocolFixture;
    seal.producer_class = ReferenceProducerClass::SyntheticFixtureGenerator;
    seal.calibration_classification = ReferenceCalibrationValidity::SyntheticProtocolOnly;
    seal.calibration_validity_impact = CalibrationValidityImpact::ProtocolOnly;
    coverage.coverage_purpose = ReferenceCoveragePurpose::SyntheticProtocolValidation;
    join.join_purpose = DetectorReferenceJoinPurpose::SyntheticProtocolValidation;

    let set = derive_contribution_set(
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &join,
        &adjudication,
        &bundle,
        &context,
    );
    assert_eq!(set.state, MetricContributionSetState::Complete);
    assert!(!set.eligibility.primary_metrics_allowed);
    assert_eq!(
        set.eligibility.report_class,
        MetricContributionReportClass::SyntheticProtocolValidation
    );
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
    let bundle = build_metric_contribution_bundle(binding_context, ArtifactBundleState::Complete);
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
    let bundle = build_metric_contribution_bundle(binding_context, ArtifactBundleState::Complete);
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
    let bundle = build_metric_contribution_bundle(binding_context, ArtifactBundleState::Complete);
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
    let bundle = build_metric_contribution_bundle(binding_context, ArtifactBundleState::Complete);
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
    let bundle = build_metric_contribution_bundle(binding_context, ArtifactBundleState::Complete);
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
    let bundle = build_metric_contribution_bundle(binding_context, ArtifactBundleState::Complete);
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

// --- Detector mappings ---

#[test]
fn detector_exact_match_ratio_mapping() {
    let stack = derive_join_exact_match();
    let set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    let record =
        detector_record_for_disposition(&set, DetectorReferenceMatchDisposition::ExactMatch);
    let (precision, duplicate) =
        expected_detector_ratios(DetectorReferenceMatchDisposition::ExactMatch);
    assert_eq!(record.proposal_precision, precision);
    assert_eq!(record.duplicate_proposal_burden, duplicate);
}

#[test]
fn detector_accepted_overlap_ratio_mapping() {
    let stack = primary_metric_stack();
    let set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    let record =
        detector_record_for_disposition(&set, DetectorReferenceMatchDisposition::AcceptedOverlap);
    let (precision, duplicate) =
        expected_detector_ratios(DetectorReferenceMatchDisposition::AcceptedOverlap);
    assert_eq!(record.proposal_precision, precision);
    assert_eq!(record.duplicate_proposal_burden, duplicate);
}

#[test]
fn detector_wrong_correction_ratio_mapping() {
    let stack = derive_join_wrong_correction();
    let set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    let record = detector_record_for_disposition(
        &set,
        DetectorReferenceMatchDisposition::DetectorWrongCorrection,
    );
    let (precision, duplicate) =
        expected_detector_ratios(DetectorReferenceMatchDisposition::DetectorWrongCorrection);
    assert_eq!(record.proposal_precision, precision);
    assert_eq!(record.duplicate_proposal_burden, duplicate);
}

#[test]
fn detector_duplicate_proposal_ratio_mapping() {
    let stack = derive_join_duplicate_proposal();
    let set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    let record =
        detector_record_for_disposition(&set, DetectorReferenceMatchDisposition::DuplicateProposal);
    let (precision, duplicate) =
        expected_detector_ratios(DetectorReferenceMatchDisposition::DuplicateProposal);
    assert_eq!(record.proposal_precision, precision);
    assert_eq!(record.duplicate_proposal_burden, duplicate);
}

#[test]
fn detector_unmatched_detector_ratio_mapping() {
    let stack = derive_join_excluded_reference();
    let set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    let record =
        detector_record_for_disposition(&set, DetectorReferenceMatchDisposition::UnmatchedDetector);
    let (precision, duplicate) =
        expected_detector_ratios(DetectorReferenceMatchDisposition::UnmatchedDetector);
    assert_eq!(record.proposal_precision, precision);
    assert_eq!(record.duplicate_proposal_burden, duplicate);
}

#[test]
fn detector_ambiguous_match_ratio_mapping() {
    let stack = derive_join_ambiguous_match();
    let set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    let record =
        detector_record_for_disposition(&set, DetectorReferenceMatchDisposition::AmbiguousMatch);
    let (precision, duplicate) =
        expected_detector_ratios(DetectorReferenceMatchDisposition::AmbiguousMatch);
    assert_eq!(record.proposal_precision, precision);
    assert_eq!(record.duplicate_proposal_burden, duplicate);
}

#[test]
fn detector_excluded_from_error_metrics_ratio_mapping() {
    let (precision, duplicate) =
        expected_detector_ratios(DetectorReferenceMatchDisposition::ExcludedFromErrorMetrics);
    assert_eq!(
        precision,
        RatioContribution::Excluded(MetricContributionExclusionReason::JoinExcluded)
    );
    assert_eq!(
        duplicate,
        RatioContribution::Excluded(MetricContributionExclusionReason::JoinExcluded)
    );
}

#[test]
fn detector_overlap_candidate_ratio_mapping() {
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
    let bundle = build_metric_contribution_bundle(binding_context, ArtifactBundleState::Complete);
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
    .expect("derive overlap candidate join");
    let set = derive_contribution_set(
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
    let record =
        detector_record_for_disposition(&set, DetectorReferenceMatchDisposition::OverlapCandidate);
    let (precision, duplicate) =
        expected_detector_ratios(DetectorReferenceMatchDisposition::OverlapCandidate);
    assert_eq!(record.proposal_precision, precision);
    assert_eq!(record.duplicate_proposal_burden, duplicate);
}

#[test]
fn side_incompatible_unmatched_reference_on_detector_side_rejected() {
    let stack = primary_metric_stack();
    let set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    let mut tampered = set.clone();
    tampered
        .detector_contributions
        .push(tampered.detector_contributions[0].clone());
    tampered.detector_contributions[1].detector_proposal_id =
        DetectorProposalId::new("det-prop-side").expect("proposal id");
    tampered.detector_contributions[1].join_disposition =
        DetectorReferenceMatchDisposition::UnmatchedReference;
    tampered.detector_contributions[1].proposal_precision =
        RatioContribution::Excluded(MetricContributionExclusionReason::JoinExcluded);
    tampered.detector_contributions[1].duplicate_proposal_burden =
        RatioContribution::Excluded(MetricContributionExclusionReason::JoinExcluded);
    tampered.assessment.detector_contribution_count = 2;
    tampered.assessment.detector_source_count = 2;

    assert!(matches!(
        tampered.validate(),
        Err(
            JoinMetricContributionError::SideIncompatibleDetectorDisposition {
                disposition: DetectorReferenceMatchDisposition::UnmatchedReference
            }
        )
    ));
}

#[test]
fn duplicate_detector_contribution_id_rejected() {
    let stack = primary_metric_stack();
    let mut set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    let duplicate = set.detector_contributions[0].clone();
    set.detector_contributions.push(duplicate);

    assert!(matches!(
        set.validate(),
        Err(JoinMetricContributionError::DuplicateDetectorContributionId { .. })
    ));
}

#[test]
fn missing_and_extra_detector_contribution_ids_rejected() {
    let stack = primary_metric_stack();
    let mut set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    set.detector_contributions.pop();
    assert!(matches!(
        set.validate(),
        Err(JoinMetricContributionError::AssessmentMismatch { .. })
    ));

    let mut extra = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    extra
        .detector_contributions
        .push(extra.detector_contributions[0].clone());
    assert!(matches!(
        extra.validate(),
        Err(JoinMetricContributionError::DuplicateDetectorContributionId { .. })
    ));
}

#[test]
fn shuffled_detector_source_order_is_stabilized() {
    let stack = derive_join_duplicate_proposal();
    let set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    let ids: Vec<_> = set
        .detector_contributions
        .iter()
        .map(|record| record.detector_proposal_id.as_str().to_string())
        .collect();
    assert_eq!(ids, vec!["det-prop-a", "det-prop-b"]);
}

#[test]
fn detector_denominator_participation_must_agree() {
    let stack = primary_metric_stack();
    let mut set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    set.detector_contributions[0].proposal_precision = RatioContribution::NumeratorAndDenominator;
    set.detector_contributions[0].duplicate_proposal_burden =
        RatioContribution::Excluded(MetricContributionExclusionReason::AmbiguousMatch);

    assert!(matches!(
        set.validate(),
        Err(JoinMetricContributionError::StoredContributionMismatch)
    ));
}

// --- Reference mappings ---

#[test]
fn reference_exact_match_ratio_mapping() {
    let stack = derive_join_exact_match();
    let set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    let record =
        reference_record_for_disposition(&set, DetectorReferenceMatchDisposition::ExactMatch);
    let (localization, correction, end_to_end) = expected_reference_ratios(
        ReferenceJoinEligibility::RecallEligibleTranscriptionError,
        DetectorReferenceMatchDisposition::ExactMatch,
    );
    assert_eq!(record.error_localization_recall, localization);
    assert_eq!(record.correction_exactness_given_localization, correction);
    assert_eq!(record.end_to_end_exact_correction_recall, end_to_end);
}

#[test]
fn reference_accepted_overlap_ratio_mapping() {
    let stack = primary_metric_stack();
    let set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    let record =
        reference_record_for_disposition(&set, DetectorReferenceMatchDisposition::AcceptedOverlap);
    let (localization, correction, end_to_end) = expected_reference_ratios(
        ReferenceJoinEligibility::RecallEligibleTranscriptionError,
        DetectorReferenceMatchDisposition::AcceptedOverlap,
    );
    assert_eq!(record.error_localization_recall, localization);
    assert_eq!(record.correction_exactness_given_localization, correction);
    assert_eq!(record.end_to_end_exact_correction_recall, end_to_end);
}

#[test]
fn reference_wrong_correction_ratio_mapping() {
    let stack = derive_join_wrong_correction();
    let set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    let record = reference_record_for_disposition(
        &set,
        DetectorReferenceMatchDisposition::DetectorWrongCorrection,
    );
    let (localization, correction, end_to_end) = expected_reference_ratios(
        ReferenceJoinEligibility::RecallEligibleTranscriptionError,
        DetectorReferenceMatchDisposition::DetectorWrongCorrection,
    );
    assert_eq!(record.error_localization_recall, localization);
    assert_eq!(record.correction_exactness_given_localization, correction);
    assert_eq!(record.end_to_end_exact_correction_recall, end_to_end);
}

#[test]
fn reference_unmatched_reference_ratio_mapping() {
    let stack = derive_join_unmatched_reference();
    let set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    let record = reference_record_for_disposition(
        &set,
        DetectorReferenceMatchDisposition::UnmatchedReference,
    );
    let (localization, correction, end_to_end) = expected_reference_ratios(
        ReferenceJoinEligibility::RecallEligibleTranscriptionError,
        DetectorReferenceMatchDisposition::UnmatchedReference,
    );
    assert_eq!(record.error_localization_recall, localization);
    assert_eq!(record.correction_exactness_given_localization, correction);
    assert_eq!(record.end_to_end_exact_correction_recall, end_to_end);
}

#[test]
fn reference_ambiguous_match_ratio_mapping() {
    let stack = derive_join_ambiguous_match();
    let set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    for reference_error_id in ["ref-err-1", "ref-err-2"] {
        let record = set
            .reference_contributions
            .iter()
            .find(|entry| entry.reference_error_id.as_str() == reference_error_id)
            .expect("reference contribution");
        assert_eq!(
            record.join_disposition,
            DetectorReferenceMatchDisposition::AmbiguousMatch
        );
        let (localization, correction, end_to_end) = expected_reference_ratios(
            ReferenceJoinEligibility::RecallEligibleTranscriptionError,
            DetectorReferenceMatchDisposition::AmbiguousMatch,
        );
        assert_eq!(record.error_localization_recall, localization);
        assert_eq!(record.correction_exactness_given_localization, correction);
        assert_eq!(record.end_to_end_exact_correction_recall, end_to_end);
    }
}

#[test]
fn reference_overlap_candidate_ratio_mapping() {
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
    let bundle = build_metric_contribution_bundle(binding_context, ArtifactBundleState::Complete);
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
    .expect("derive overlap candidate join");
    let set = derive_contribution_set(
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
    let record =
        reference_record_for_disposition(&set, DetectorReferenceMatchDisposition::OverlapCandidate);
    let (localization, correction, end_to_end) = expected_reference_ratios(
        ReferenceJoinEligibility::RecallEligibleTranscriptionError,
        DetectorReferenceMatchDisposition::OverlapCandidate,
    );
    assert_eq!(record.error_localization_recall, localization);
    assert_eq!(record.correction_exactness_given_localization, correction);
    assert_eq!(record.end_to_end_exact_correction_recall, end_to_end);
}

#[test]
fn reference_excluded_from_error_metrics_ratio_mapping() {
    let stack = derive_join_excluded_reference();
    let set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    let record = reference_record_for_disposition(
        &set,
        DetectorReferenceMatchDisposition::ExcludedFromErrorMetrics,
    );
    assert_eq!(
        record.reference_eligibility,
        ReferenceJoinEligibility::ExcludedVerificationBasis
    );
    let (localization, correction, end_to_end) = expected_reference_ratios(
        ReferenceJoinEligibility::ExcludedVerificationBasis,
        DetectorReferenceMatchDisposition::ExcludedFromErrorMetrics,
    );
    assert_eq!(record.error_localization_recall, localization);
    assert_eq!(record.correction_exactness_given_localization, correction);
    assert_eq!(record.end_to_end_exact_correction_recall, end_to_end);
}

#[test]
fn reference_eligibility_categories_map_to_excluded_contributions() {
    for (class, verification, expected_eligibility) in [
        (
            ReferenceClass::TranscriptionError,
            VerificationBasis::AudioListened,
            ReferenceJoinEligibility::RecallEligibleTranscriptionError,
        ),
        (
            ReferenceClass::TranscriptionError,
            VerificationBasis::TranscriptContextOnly,
            ReferenceJoinEligibility::ExcludedVerificationBasis,
        ),
        (
            ReferenceClass::StylePreference,
            VerificationBasis::AudioListened,
            ReferenceJoinEligibility::ExcludedReferenceClass,
        ),
        (
            ReferenceClass::Ambiguous,
            VerificationBasis::AudioListened,
            ReferenceJoinEligibility::ExcludedReferenceClass,
        ),
        (
            ReferenceClass::Unsupported,
            VerificationBasis::AudioListened,
            ReferenceJoinEligibility::ExcludedReferenceClass,
        ),
        (
            ReferenceClass::NonError,
            VerificationBasis::AudioListened,
            ReferenceJoinEligibility::ExcludedReferenceClass,
        ),
    ] {
        let envelope = join_envelope(RunLifecycleState::DetectorExecution);
        let seal = join_seal();
        let cue_disposition = if class == ReferenceClass::TranscriptionError {
            ReferenceCueDisposition::TranscriptionError
        } else {
            ReferenceCueDisposition::NoTranscriptionError
        };
        let mut coverage = build_coverage(
            ReferenceCoveragePurpose::PrimaryBlindCalibration,
            vec![record(1, cue_disposition)],
            "run-join",
        );
        coverage.assessment.total_eligible_transcription_errors =
            if expected_eligibility == ReferenceJoinEligibility::RecallEligibleTranscriptionError {
                1
            } else {
                0
            };
        let error_record = ReferenceErrorRecord {
            reference_error_id: ReferenceErrorId::new("ref-err-elig").expect("error id"),
            reference_revision: ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION)
                .expect("revision"),
            input_identity: input_identity(),
            source_anchor: reference_source_anchor(1, 0, 0, 4),
            original_surface: "wrng".to_string(),
            human_final_surface: "wrong".to_string(),
            reference_class: class,
            verification_basis: verification,
            reviewer_identity_class: ReferenceReviewerIdentityClass::OwnerBlindReviewer,
            reviewed_at_unix_ms: 1_700_000_000_000,
        };
        let human_reference = HumanFinalReference {
            schema_revision: HUMAN_FINAL_REFERENCE_SCHEMA.to_string(),
            run_id: coverage.run_id.clone(),
            input_identity: coverage.input_identity.clone(),
            seal_id: coverage.seal_id.clone(),
            reference_revision: coverage.reference_revision.clone(),
            records: vec![error_record.clone()],
            state: HumanFinalReferenceState::Sealed,
            assessment: HumanFinalReference::derive_assessment(
                &coverage.reference_revision,
                &coverage.input_identity,
                &[error_record],
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
        let bundle =
            build_metric_contribution_bundle(binding_context, ArtifactBundleState::Complete);
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
        .expect("derive join for eligibility category");
        let set = derive_contribution_set(
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
        let record = &set.reference_contributions[0];
        assert_eq!(record.reference_eligibility, expected_eligibility);
        if expected_eligibility == ReferenceJoinEligibility::RecallEligibleTranscriptionError {
            assert_eq!(
                record.join_disposition,
                DetectorReferenceMatchDisposition::ExactMatch
            );
        } else {
            assert_eq!(
                record.join_disposition,
                DetectorReferenceMatchDisposition::ExcludedFromErrorMetrics
            );
            let excluded =
                RatioContribution::Excluded(MetricContributionExclusionReason::ReferenceIneligible);
            assert_eq!(record.error_localization_recall, excluded);
            assert_eq!(record.correction_exactness_given_localization, excluded);
            assert_eq!(record.end_to_end_exact_correction_recall, excluded);
        }
    }
}

#[test]
fn ineligible_reference_is_excluded_but_pending_is_not_excluded() {
    let stack = derive_join_excluded_reference();
    let excluded = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    let excluded_record = reference_record_for_disposition(
        &excluded,
        DetectorReferenceMatchDisposition::ExcludedFromErrorMetrics,
    );
    assert!(matches!(
        excluded_record.error_localization_recall,
        RatioContribution::Excluded(MetricContributionExclusionReason::ReferenceIneligible)
    ));

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
    let bundle = build_metric_contribution_bundle(binding_context, ArtifactBundleState::Complete);
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
    let pending = derive_contribution_set(
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
    let pending_record = reference_record_for_disposition(
        &pending,
        DetectorReferenceMatchDisposition::OverlapCandidate,
    );
    assert!(matches!(
        pending_record.error_localization_recall,
        RatioContribution::PendingAdjudication
    ));
}

#[test]
fn side_incompatible_duplicate_proposal_and_unmatched_detector_on_reference_side_rejected() {
    let stack = primary_metric_stack();
    let mut set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    set.reference_contributions[0].join_disposition =
        DetectorReferenceMatchDisposition::DuplicateProposal;
    assert!(matches!(
        set.validate(),
        Err(
            JoinMetricContributionError::SideIncompatibleReferenceDisposition {
                disposition: DetectorReferenceMatchDisposition::DuplicateProposal
            }
        )
    ));

    set.reference_contributions[0].join_disposition =
        DetectorReferenceMatchDisposition::UnmatchedDetector;
    assert!(matches!(
        set.validate(),
        Err(
            JoinMetricContributionError::SideIncompatibleReferenceDisposition {
                disposition: DetectorReferenceMatchDisposition::UnmatchedDetector
            }
        )
    ));
}

#[test]
fn duplicate_reference_contribution_id_rejected() {
    let stack = primary_metric_stack();
    let mut set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    let duplicate = set.reference_contributions[0].clone();
    set.reference_contributions.push(duplicate);
    assert!(matches!(
        set.validate(),
        Err(JoinMetricContributionError::DuplicateReferenceContributionId { .. })
    ));
}

// --- Set state / assessment / lifecycle / stored validation / scope guards ---

#[test]
fn complete_state_requires_mapping_complete_and_no_pending_counts() {
    let stack = primary_metric_stack();
    let set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    assert_eq!(set.state, MetricContributionSetState::Complete);
    assert!(set.assessment.mapping_complete);
    assert_eq!(set.assessment.pending_detector_contribution_count, 0);
    assert_eq!(set.assessment.pending_reference_contribution_count, 0);
}

#[test]
fn pending_state_tracks_pending_counts_and_blocks_complete_primary_eligibility() {
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
    let bundle = build_metric_contribution_bundle(binding_context, ArtifactBundleState::Complete);
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
    let set = derive_contribution_set(
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
    assert_eq!(set.state, MetricContributionSetState::PendingJoinResolution);
    assert_eq!(set.assessment.pending_detector_contribution_count, 1);
    assert_eq!(set.assessment.pending_reference_contribution_count, 1);
}

#[test]
fn invalidated_contribution_set_rejects_validate_against() {
    let stack = primary_metric_stack();
    let mut set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    set.state = MetricContributionSetState::Invalidated;
    set.eligibility.primary_metrics_allowed = false;
    set.eligibility.eligible_primary_metrics.clear();
    set.eligibility.report_class = MetricContributionReportClass::NonCalibrationDiagnostic;
    set.eligibility
        .blocking_reasons
        .push(PrimaryMetricBlockingReason::ContributionSetPending);
    assert!(matches!(
        set.validate_against(
            &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5,
        ),
        Err(JoinMetricContributionError::ContributionSetInvalidated)
    ));
}

#[test]
fn assessment_mismatch_rejected_locally() {
    let stack = primary_metric_stack();
    let mut set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    set.assessment = MetricContributionSetAssessment {
        detector_source_count: 0,
        detector_contribution_count: 0,
        reference_source_count: 0,
        reference_contribution_count: 0,
        pending_detector_contribution_count: 0,
        pending_reference_contribution_count: 0,
        mapping_complete: true,
    };
    assert!(matches!(
        set.validate(),
        Err(JoinMetricContributionError::AssessmentMismatch { .. })
    ));
}

#[test]
fn stored_eligibility_mismatch_rejected_on_validate_against() {
    let stack = primary_metric_stack();
    let mut set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    set.eligibility.blocking_reasons = vec![PrimaryMetricBlockingReason::JoinNotPrimary];
    set.eligibility.primary_metrics_allowed = false;
    set.eligibility.eligible_primary_metrics.clear();
    set.eligibility.report_class = MetricContributionReportClass::NonCalibrationDiagnostic;
    assert!(matches!(
        set.validate_against(
            &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5,
        ),
        Err(JoinMetricContributionError::TopLevelBindingMismatch {
            field: "eligibility"
        })
    ));
}

#[test]
fn creation_lifecycle_rejects_reference_preparation() {
    let stack = primary_metric_stack();
    let mut envelope = stack.0.clone();
    envelope.lifecycle_state = RunLifecycleState::ReferencePreparation;
    assert!(matches!(
        JoinMetricContributionSet::derive(
            &stack.6, &envelope, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7,
            &stack.5,
        ),
        Err(JoinMetricContributionError::ContributionCreationLifecycleIncompatible { .. })
    ));
}

#[test]
fn historical_validate_against_accepts_finalized_lifecycle() {
    let stack = primary_metric_stack();
    let mut envelope = stack.0.clone();
    envelope.lifecycle_state = RunLifecycleState::Finalized;
    let set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    set.validate_against(
        &envelope, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5,
    )
    .expect("historical validation");
}

#[test]
fn invalidated_envelope_context_rejected() {
    let stack = primary_metric_stack();
    let mut envelope = stack.0.clone();
    envelope.lifecycle_state = RunLifecycleState::Invalidated;
    let set = derive_contribution_set(
        &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5, &stack.6,
    );
    assert!(matches!(
        set.validate_against(
            &envelope, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7, &stack.5,
        ),
        Err(JoinMetricContributionError::ContributionHistoricalLifecycleIncompatible { .. })
    ));
}

#[test]
fn unfrozen_snapshot_rejected() {
    let stack = primary_metric_stack();
    let mut snapshot = stack.4.clone();
    snapshot.state = DetectorProposalSnapshotState::Draft;
    assert!(matches!(
        JoinMetricContributionSet::derive(
            &stack.6, &stack.0, &stack.1, &stack.2, &stack.3, &snapshot, &stack.8, &stack.7,
            &stack.5,
        ),
        Err(JoinMetricContributionError::SnapshotNotFrozen)
    ));
}

#[test]
fn bundle_must_include_exactly_one_metric_contributions_role() {
    let stack = primary_metric_stack();
    let mut bundle = stack.5.clone();
    for descriptor in &mut bundle.artifacts {
        if descriptor.role == ArtifactRole::MetricContributions {
            descriptor.artifact_id =
                ArtifactId::new("artifact-metric-contrib-wrong").expect("artifact id");
        }
    }
    assert!(matches!(
        JoinMetricContributionSet::derive(
            &stack.6, &stack.0, &stack.1, &stack.2, &stack.3, &stack.4, &stack.8, &stack.7,
            &bundle,
        ),
        Err(JoinMetricContributionError::MetricContributionsArtifactMismatch)
    ));
}

#[test]
fn pending_complete_state_mismatch_rejected() {
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
    let bundle = build_metric_contribution_bundle(binding_context, ArtifactBundleState::Complete);
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
    let mut set = derive_contribution_set(
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
    set.state = MetricContributionSetState::Complete;
    assert!(matches!(
        set.validate(),
        Err(JoinMetricContributionError::PendingCompleteStateMismatch)
    ));
}

#[test]
fn schema_revision_constants_enforced() {
    assert_eq!(
        JOIN_METRIC_CONTRIBUTION_SCHEMA,
        "voxproof-join-metric-contributions-v1"
    );
    assert_eq!(
        PRIMARY_METRIC_ELIGIBILITY_POLICY,
        "voxproof-primary-metric-eligibility-v1"
    );
    assert_eq!(
        METRIC_CONTRIBUTION_POLICY,
        "voxproof-metric-contribution-v1"
    );
}

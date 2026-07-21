use std::collections::{HashMap, HashSet};

use vox_proof::artifact_bundle::{
    ARTIFACT_BUNDLE_SCHEMA, ArtifactBindingContext, ArtifactBundle, ArtifactBundleId,
    ArtifactBundleState, ArtifactContentDigest, ArtifactDescriptor, ArtifactId,
    ArtifactSchemaIdentity,
};
use vox_proof::candidate::DetectionKind;
use vox_proof::detector_reference_join::{
    ALTERNATIVE_CARDINALITY_POLICY, CORRECTION_EQUALITY_REVISION, DETECTOR_REFERENCE_JOIN_SCHEMA,
    DetectorReferenceJoin, DetectorReferenceJoinAssessment, DetectorReferenceJoinContext,
    DetectorReferenceJoinEdge, DetectorReferenceJoinError, DetectorReferenceJoinId,
    DetectorReferenceJoinPurpose, DetectorReferenceJoinRevisionId, DetectorReferenceJoinState,
    DetectorReferenceMatchDisposition, JoinAnchorRelation, JoinEdgeResolution, JoinRecordId,
    OVERLAP_RULE_REVISION, Phase3AdjudicationRejectionReason, PrimaryTopologyViolation,
    anchors_exact, anchors_overlap, nfc_correction_equal,
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
    OverlapAdjudicationSetState, OverlapAdjudicationValidationError, OverlapAdjudicatorRole,
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

fn build_coverage(records: Vec<CueReviewCompletionRecord>, run_id: &str) -> ReferenceCoverage {
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
        coverage_purpose: ReferenceCoveragePurpose::PrimaryBlindCalibration,
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

fn empty_frozen_adjudication_set() -> OverlapAdjudicationSet {
    frozen_adjudication_set(vec![])
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

fn build_join_bundle(
    context: ArtifactBindingContext,
    include_evaluation_join: bool,
    include_join_adjudication: bool,
    bundle_state: ArtifactBundleState,
) -> ArtifactBundle {
    let mut artifacts = vec![
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
    ];
    if include_evaluation_join {
        artifacts.push(descriptor(
            &context,
            ArtifactRole::EvaluationJoin,
            evaluation_join_artifact_id().as_str(),
        ));
    }
    if include_join_adjudication {
        artifacts.push(descriptor(
            &context,
            ArtifactRole::JoinAdjudication,
            join_adjudication_artifact_id().as_str(),
        ));
    }

    let expected_roles = vec![
        ArtifactRole::ReferenceSeal,
        ArtifactRole::HumanFinalReference,
        ArtifactRole::CueReviewCompletion,
        ArtifactRole::DetectorOutput,
        ArtifactRole::EvaluationJoin,
        ArtifactRole::JoinAdjudication,
    ];
    let assessment =
        ArtifactBundle::derive_assessment(&expected_roles, &artifacts, &context).expect("derive");

    ArtifactBundle {
        schema_revision: ARTIFACT_BUNDLE_SCHEMA.to_string(),
        bundle_id: ArtifactBundleId::new("bundle-join").expect("bundle id"),
        binding_context: context,
        expected_roles,
        artifacts,
        bundle_state,
        assessment,
    }
}

fn join_stack() -> (
    RunEnvelope,
    ReferenceSeal,
    ReferenceCoverage,
    HumanFinalReference,
    DetectorProposalSnapshot,
    ArtifactBundle,
    DetectorReferenceJoinContext,
    OverlapAdjudicationSet,
) {
    let envelope = join_envelope(RunLifecycleState::DetectorExecution);
    let seal = join_seal();
    let mut coverage = build_coverage(
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

    let context = ArtifactBindingContext {
        run_id: envelope.run_id.clone(),
        input_identity: envelope.input_identity.clone(),
        calibration_validity: envelope.calibration_validity,
        reference_seal_id: Some(seal.seal_id.clone()),
        reference_coverage_id: Some(coverage.coverage_id.clone()),
        reference_revision: Some(seal.reference_revision.clone()),
    };
    let bundle = build_join_bundle(context.clone(), true, true, ArtifactBundleState::Complete);
    let adjudication = empty_frozen_adjudication_set();

    (
        envelope,
        seal,
        coverage,
        human_reference,
        snapshot,
        bundle,
        join_context(),
        adjudication,
    )
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
        overlap_rule_revision: OVERLAP_RULE_REVISION.to_string(),
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

// --- Serialization / policy ---

#[test]
fn json_round_trip_retains_schema_and_policy_constants() {
    let (envelope, seal, coverage, human_reference, snapshot, bundle, context, adjudication) =
        join_stack();
    let join = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive join");

    let json = serde_json::to_string_pretty(&join).expect("serialize");
    assert!(json.contains(DETECTOR_REFERENCE_JOIN_SCHEMA));
    assert!(json.contains(OVERLAP_RULE_REVISION));
    assert!(json.contains(CORRECTION_EQUALITY_REVISION));
    assert!(json.contains(ALTERNATIVE_CARDINALITY_POLICY));
    assert!(json.contains("\"exact_match\""));
    assert!(json.contains("\"resolved\""));

    let restored: DetectorReferenceJoin = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.schema_revision, DETECTOR_REFERENCE_JOIN_SCHEMA);
    assert_eq!(restored.overlap_rule_revision, OVERLAP_RULE_REVISION);
    assert_eq!(
        restored.correction_equality_revision,
        CORRECTION_EQUALITY_REVISION
    );
    assert_eq!(
        restored.alternative_cardinality_policy,
        ALTERNATIVE_CARDINALITY_POLICY
    );
    restored.validate().expect("valid join");
}

#[test]
fn evaluation_join_role_json_spelling() {
    let json = serde_json::to_string(&ArtifactRole::EvaluationJoin).expect("serialize");
    assert_eq!(json, "\"evaluation_join\"");
}

// --- NFC correction equality ---

#[test]
fn nfc_correction_equal_accepts_composed_and_decomposed_forms() {
    let composed = "caf\u{00e9}";
    let decomposed = "caf\u{0065}\u{0301}";
    assert!(nfc_correction_equal(composed, decomposed));
}

#[test]
fn nfc_correction_equal_rejects_case_punctuation_and_whitespace_differences() {
    assert!(!nfc_correction_equal("Wrong", "wrong"));
    assert!(!nfc_correction_equal("wrong.", "wrong"));
    assert!(!nfc_correction_equal("wrong", " wrong"));
}

#[test]
fn nfc_correction_equal_does_not_apply_nfkc_compatibility_mapping() {
    assert!(!nfc_correction_equal("\u{021d}", "H"));
}

// --- Anchor geometry ---

#[test]
fn anchors_exact_requires_full_anchor_identity() {
    let left = source_anchor(1, 0, 0, 4);
    let right = reference_source_anchor(1, 0, 0, 4);
    assert!(anchors_exact(&left, &right));

    let shifted = reference_source_anchor(1, 0, 1, 5);
    assert!(!anchors_exact(&left, &shifted));
}

#[test]
fn anchors_overlap_rejects_touching_half_open_ranges() {
    let left = source_anchor(1, 0, 0, 4);
    let right = reference_source_anchor(1, 0, 4, 8);
    assert!(!anchors_overlap(&left, &right));
}

#[test]
fn anchors_overlap_accepts_one_byte_overlap() {
    let left = source_anchor(1, 0, 0, 4);
    let right = reference_source_anchor(1, 0, 3, 7);
    assert!(anchors_overlap(&left, &right));
}

// --- Proposal cardinality ---

#[test]
fn zero_alternatives_rejected_with_proposal_id() {
    let (envelope, seal, coverage, human_reference, mut snapshot, bundle, context, adjudication) =
        join_stack();
    snapshot.proposals[0].alternatives.clear();

    assert!(matches!(
        DetectorReferenceJoin::derive(
            &context,
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &bundle,
            &adjudication,
        ),
        Err(DetectorReferenceJoinError::UnsupportedProposalAlternativeCardinality {
            detector_proposal_id,
            observed_count: 0,
        }) if detector_proposal_id.as_str() == "det-prop-001"
    ));
}

#[test]
fn two_alternatives_rejected_with_proposal_id() {
    let (envelope, seal, coverage, human_reference, mut snapshot, bundle, context, adjudication) =
        join_stack();
    snapshot.proposals[0]
        .alternatives
        .push(DetectorProposalAlternative {
            alternative_index: 1,
            replacement_surface: "other".to_string(),
        });

    assert!(matches!(
        DetectorReferenceJoin::derive(
            &context,
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &bundle,
            &adjudication,
        ),
        Err(DetectorReferenceJoinError::UnsupportedProposalAlternativeCardinality {
            detector_proposal_id,
            observed_count: 2,
        }) if detector_proposal_id.as_str() == "det-prop-001"
    ));
}

// --- Phase 1 exact match ---

#[test]
fn phase1_exact_match_resolves_single_cue() {
    let (envelope, seal, coverage, human_reference, snapshot, bundle, context, adjudication) =
        join_stack();
    let join = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive join");

    assert_eq!(join.state, DetectorReferenceJoinState::Resolved);
    assert_eq!(join.assessment.exact_match_count, 1);
    assert_eq!(
        join.detector_dispositions[0].disposition,
        DetectorReferenceMatchDisposition::ExactMatch
    );
    join.validate_against(
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("validate derived join");
}

#[test]
fn phase1_duplicate_proposals_use_lowest_id_as_primary() {
    let (envelope, seal, mut coverage, human_reference, _snapshot, bundle, context, adjudication) =
        join_stack();
    coverage.records = vec![record(1, ReferenceCueDisposition::TranscriptionError)];
    coverage.expected_universe = universe(&[1]);
    coverage.assessment =
        ReferenceCoverage::derive_assessment(&coverage.expected_universe, &coverage.records)
            .expect("derive assessment");
    coverage.assessment.total_eligible_transcription_errors = 1;

    let proposals = vec![
        glossary_proposal("det-prop-b", 1, 0, 0, 4, "wrng", "wrong"),
        observed_error_proposal("det-prop-a", 1, 0, 0, 4, "wrng", "wrong"),
    ];
    let snapshot = build_snapshot(proposals, DetectorProposalSnapshotState::Frozen);

    let join = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive join");

    let primary = join
        .detector_dispositions
        .iter()
        .find(|record| record.disposition == DetectorReferenceMatchDisposition::ExactMatch)
        .expect("primary exact match");
    assert_eq!(primary.detector_proposal_id.as_str(), "det-prop-a");

    let duplicate = join
        .detector_dispositions
        .iter()
        .find(|record| record.disposition == DetectorReferenceMatchDisposition::DuplicateProposal)
        .expect("duplicate proposal");
    assert_eq!(duplicate.detector_proposal_id.as_str(), "det-prop-b");
}

// --- Phase 2 wrong correction ---

#[test]
fn phase2_exact_anchor_wrong_correction_assigns_detector_wrong_correction() {
    let (envelope, seal, coverage, human_reference, _snapshot, bundle, context, adjudication) =
        join_stack();
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

    let join = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive join");

    assert_eq!(join.state, DetectorReferenceJoinState::Resolved);
    assert_eq!(join.assessment.detector_wrong_correction_count, 1);
    assert_eq!(
        join.detector_dispositions[0].disposition,
        DetectorReferenceMatchDisposition::DetectorWrongCorrection
    );
}

// --- Phase 3 overlap ---

#[test]
fn phase3_overlap_without_adjudication_requires_adjudication_state() {
    let (envelope, seal, coverage, human_reference, _snapshot, bundle, context, adjudication) =
        join_stack();
    let mut human_reference = human_reference;
    human_reference.records = vec![reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong")];
    human_reference.assessment = HumanFinalReference::derive_assessment(
        &human_reference.reference_revision,
        &human_reference.input_identity,
        &human_reference.records,
    )
    .expect("derive assessment");

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

    let join = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive join");

    assert_eq!(join.state, DetectorReferenceJoinState::RequiresAdjudication);
    assert_eq!(join.assessment.unresolved_overlap_edge_count, 1);
    assert_eq!(
        join.detector_dispositions[0].disposition,
        DetectorReferenceMatchDisposition::OverlapCandidate
    );
}

#[test]
fn accepted_overlap_requires_frozen_adjudication_at_assisted_review() {
    let (mut envelope, seal, coverage, human_reference, _snapshot, bundle, context, _adjudication) =
        join_stack();
    envelope.lifecycle_state = RunLifecycleState::AssistedReview;

    let mut human_reference = human_reference;
    human_reference.records = vec![reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong")];
    human_reference.assessment = HumanFinalReference::derive_assessment(
        &human_reference.reference_revision,
        &human_reference.input_identity,
        &human_reference.records,
    )
    .expect("derive assessment");

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

    let adjudication = frozen_adjudication_set(vec![adjudication_record(
        "adj-001",
        "det-prop-001",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);

    let join = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive overlap join");

    assert_eq!(join.state, DetectorReferenceJoinState::Resolved);
    assert_eq!(join.assessment.accepted_overlap_count, 1);
    assert_eq!(
        join.detector_dispositions[0].disposition,
        DetectorReferenceMatchDisposition::AcceptedOverlap
    );
}

#[test]
fn draft_adjudication_set_cannot_resolve_overlap_join() {
    let (mut envelope, seal, coverage, human_reference, _snapshot, bundle, context, _adjudication) =
        join_stack();
    envelope.lifecycle_state = RunLifecycleState::AssistedReview;

    let mut human_reference = human_reference;
    human_reference.records = vec![reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong")];
    human_reference.assessment = HumanFinalReference::derive_assessment(
        &human_reference.reference_revision,
        &human_reference.input_identity,
        &human_reference.records,
    )
    .expect("derive assessment");

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

    let mut adjudication = frozen_adjudication_set(vec![adjudication_record(
        "adj-001",
        "det-prop-001",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);
    adjudication.state = OverlapAdjudicationSetState::Draft;

    assert!(matches!(
        DetectorReferenceJoin::derive(
            &context,
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &bundle,
            &adjudication,
        ),
        Err(DetectorReferenceJoinError::AdjudicationValidation(_))
    ));
}

// --- Lifecycle ---

#[test]
fn exact_only_join_resolves_at_detector_execution() {
    let (envelope, seal, coverage, human_reference, snapshot, bundle, context, adjudication) =
        join_stack();
    assert_eq!(
        envelope.lifecycle_state,
        RunLifecycleState::DetectorExecution
    );

    let join = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive at detector execution");

    assert_eq!(join.state, DetectorReferenceJoinState::Resolved);
}

#[test]
fn reference_sealed_lifecycle_rejects_join_creation() {
    let (mut envelope, seal, coverage, human_reference, snapshot, bundle, context, adjudication) =
        join_stack();
    envelope.lifecycle_state = RunLifecycleState::ReferenceSealed;

    assert!(matches!(
        DetectorReferenceJoin::derive(
            &context,
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &bundle,
            &adjudication,
        ),
        Err(
            DetectorReferenceJoinError::JoinCreationLifecycleIncompatible {
                lifecycle_state: RunLifecycleState::ReferenceSealed,
            }
        )
    ));
}

// --- Bundle roles ---

#[test]
fn missing_evaluation_join_role_fails() {
    let (envelope, seal, coverage, human_reference, snapshot, context, adjudication) = {
        let (envelope, seal, coverage, human_reference, snapshot, _, context, adjudication) =
            join_stack();
        (
            envelope,
            seal,
            coverage,
            human_reference,
            snapshot,
            context,
            adjudication,
        )
    };
    let context_binding = ArtifactBindingContext {
        run_id: envelope.run_id.clone(),
        input_identity: envelope.input_identity.clone(),
        calibration_validity: envelope.calibration_validity,
        reference_seal_id: Some(seal.seal_id.clone()),
        reference_coverage_id: Some(coverage.coverage_id.clone()),
        reference_revision: Some(seal.reference_revision.clone()),
    };
    let bundle = build_join_bundle(context_binding, false, true, ArtifactBundleState::Draft);

    assert!(matches!(
        DetectorReferenceJoin::derive(
            &context,
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &bundle,
            &adjudication,
        ),
        Err(DetectorReferenceJoinError::EvaluationJoinArtifactMismatch)
    ));
}

#[test]
fn missing_join_adjudication_role_fails() {
    let (envelope, seal, coverage, human_reference, snapshot, context, adjudication) = {
        let (envelope, seal, coverage, human_reference, snapshot, _, context, adjudication) =
            join_stack();
        (
            envelope,
            seal,
            coverage,
            human_reference,
            snapshot,
            context,
            adjudication,
        )
    };
    let context_binding = ArtifactBindingContext {
        run_id: envelope.run_id.clone(),
        input_identity: envelope.input_identity.clone(),
        calibration_validity: envelope.calibration_validity,
        reference_seal_id: Some(seal.seal_id.clone()),
        reference_coverage_id: Some(coverage.coverage_id.clone()),
        reference_revision: Some(seal.reference_revision.clone()),
    };
    let bundle = build_join_bundle(context_binding, true, false, ArtifactBundleState::Draft);

    assert!(matches!(
        DetectorReferenceJoin::derive(
            &context,
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &bundle,
            &adjudication,
        ),
        Err(DetectorReferenceJoinError::JoinAdjudicationArtifactMismatch)
    ));
}

// --- Scope ---

#[test]
fn serialized_join_contains_no_tp_fp_fn_fields() {
    let (envelope, seal, coverage, human_reference, snapshot, bundle, context, adjudication) =
        join_stack();
    let join = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive join");

    let json = serde_json::to_string(&join).expect("serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");

    for forbidden in [
        "true_positive",
        "false_positive",
        "false_negative",
        "tp",
        "fp",
        "fn",
        "precision",
        "recall",
        "transcript_text",
        "path",
    ] {
        assert!(
            value.get(forbidden).is_none(),
            "serialized join must not contain {forbidden:?}"
        );
    }
}

#[test]
fn shuffled_proposal_order_produces_identical_join() {
    let (envelope, seal, coverage, human_reference, _, bundle, context, adjudication) =
        join_stack();
    let proposals = vec![
        glossary_proposal("det-prop-b", 1, 0, 0, 4, "wrng", "wrong"),
        observed_error_proposal("det-prop-a", 1, 0, 0, 4, "wrng", "wrong"),
    ];
    let canonical_snapshot =
        build_snapshot(proposals.clone(), DetectorProposalSnapshotState::Frozen);
    let canonical = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &canonical_snapshot,
        &bundle,
        &adjudication,
    )
    .expect("canonical derive");

    let shuffled_snapshot = build_snapshot(
        vec![
            observed_error_proposal("det-prop-a", 1, 0, 0, 4, "wrng", "wrong"),
            glossary_proposal("det-prop-b", 1, 0, 0, 4, "wrng", "wrong"),
        ],
        DetectorProposalSnapshotState::Frozen,
    );

    let shuffled = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &shuffled_snapshot,
        &bundle,
        &adjudication,
    )
    .expect("shuffled derive");

    shuffled
        .validate_against(
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &shuffled_snapshot,
            &bundle,
            &adjudication,
        )
        .expect("shuffled join validates");

    assert_eq!(shuffled.edges, canonical.edges);
    assert_eq!(
        shuffled.detector_dispositions,
        canonical.detector_dispositions
    );
    assert_eq!(
        shuffled.reference_dispositions,
        canonical.reference_dispositions
    );
    assert_eq!(shuffled.assessment, canonical.assessment);
    assert_eq!(shuffled.state, canonical.state);
}

#[test]
fn contract_tests_use_synthetic_strings_only() {
    let (envelope, seal, coverage, human_reference, snapshot, bundle, context, adjudication) =
        join_stack();
    let join = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive join");

    join.validate().expect("synthetic join valid");
    assert!(SAMPLE_REVISION.starts_with("rev:sha256-v1:"));
    assert_eq!(envelope.run_id.as_str(), "run-join");
}

#[test]
fn overlap_wrong_correction_requires_frozen_adjudication_and_nfc_difference() {
    let (mut envelope, seal, coverage, _human_reference, _snapshot, bundle, context, _adjudication) =
        join_stack();
    envelope.lifecycle_state = RunLifecycleState::AssistedReview;

    let mut human_reference = human_reference_for_coverage(&coverage);
    human_reference.records = vec![reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong")];
    human_reference.assessment = HumanFinalReference::derive_assessment(
        &human_reference.reference_revision,
        &human_reference.input_identity,
        &human_reference.records,
    )
    .expect("derive assessment");

    let snapshot = build_snapshot(
        vec![glossary_proposal(
            "det-prop-001",
            1,
            0,
            2,
            6,
            "wrng",
            "wright",
        )],
        DetectorProposalSnapshotState::Frozen,
    );

    let adjudication = frozen_adjudication_set(vec![adjudication_record(
        "adj-001",
        "det-prop-001",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorWrongCorrection,
    )]);

    let join = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive overlap wrong correction join");

    assert_eq!(join.state, DetectorReferenceJoinState::Resolved);
    assert_eq!(join.assessment.detector_wrong_correction_count, 1);
    assert_eq!(
        join.detector_dispositions[0].disposition,
        DetectorReferenceMatchDisposition::DetectorWrongCorrection
    );
}

#[test]
fn same_error_same_correction_fails_when_nfc_correction_differs() {
    let (mut envelope, seal, coverage, _human_reference, _snapshot, bundle, context, _adjudication) =
        join_stack();
    envelope.lifecycle_state = RunLifecycleState::AssistedReview;

    let mut human_reference = human_reference_for_coverage(&coverage);
    human_reference.records = vec![reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong")];
    human_reference.assessment = HumanFinalReference::derive_assessment(
        &human_reference.reference_revision,
        &human_reference.input_identity,
        &human_reference.records,
    )
    .expect("derive assessment");

    let snapshot = build_snapshot(
        vec![glossary_proposal(
            "det-prop-001",
            1,
            0,
            2,
            6,
            "wrng",
            "wright",
        )],
        DetectorProposalSnapshotState::Frozen,
    );

    let adjudication = frozen_adjudication_set(vec![adjudication_record(
        "adj-001",
        "det-prop-001",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);

    assert!(matches!(
        DetectorReferenceJoin::derive(
            &context,
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &bundle,
            &adjudication,
        ),
        Err(DetectorReferenceJoinError::AdjudicationCorrectionResultMismatch { .. })
    ));
}

#[test]
fn one_detector_multiple_overlap_primaries_becomes_ambiguous() {
    let (
        mut envelope,
        seal,
        mut coverage,
        _human_reference,
        _snapshot,
        bundle,
        context,
        _adjudication,
    ) = join_stack();
    envelope.lifecycle_state = RunLifecycleState::AssistedReview;
    coverage.records = vec![record(1, ReferenceCueDisposition::TranscriptionError)];
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
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive ambiguous join");

    assert_eq!(
        join.detector_dispositions[0].disposition,
        DetectorReferenceMatchDisposition::AmbiguousMatch
    );
    assert!(
        join.detector_dispositions[0]
            .primary_reference_error_id
            .is_none()
    );
}

#[test]
fn transcript_context_only_reference_is_excluded_from_matching() {
    let (envelope, seal, mut coverage, _human_reference, _snapshot, bundle, context, adjudication) =
        join_stack();
    coverage.assessment.total_eligible_transcription_errors = 0;
    let mut human_reference = human_reference_for_coverage(&coverage);
    human_reference.records = vec![ReferenceErrorRecord {
        reference_error_id: ReferenceErrorId::new("ref-err-tco").expect("error id"),
        reference_revision: ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION).expect("revision"),
        input_identity: input_identity(),
        source_anchor: reference_source_anchor(1, 0, 0, 4),
        original_surface: "wrng".to_string(),
        human_final_surface: "wrong".to_string(),
        reference_class: ReferenceClass::TranscriptionError,
        verification_basis: VerificationBasis::TranscriptContextOnly,
        reviewer_identity_class: ReferenceReviewerIdentityClass::OwnerBlindReviewer,
        reviewed_at_unix_ms: 1_700_000_000_000,
    }];
    human_reference.assessment = HumanFinalReference::derive_assessment(
        &human_reference.reference_revision,
        &human_reference.input_identity,
        &human_reference.records,
    )
    .expect("derive assessment");

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

    let join = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive join");

    assert_eq!(
        join.reference_dispositions[0].disposition,
        DetectorReferenceMatchDisposition::ExcludedFromErrorMetrics
    );
    assert_eq!(
        join.detector_dispositions[0].disposition,
        DetectorReferenceMatchDisposition::UnmatchedDetector
    );
}

#[test]
fn original_surface_equality_does_not_override_nfc_correction_authority() {
    let (envelope, seal, coverage, _human_reference, _snapshot, bundle, context, adjudication) =
        join_stack();
    let mut human_reference = human_reference_for_coverage(&coverage);
    human_reference.records = vec![ReferenceErrorRecord {
        reference_error_id: ReferenceErrorId::new("ref-err-orig").expect("error id"),
        reference_revision: ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION).expect("revision"),
        input_identity: input_identity(),
        source_anchor: reference_source_anchor(1, 0, 0, 4),
        original_surface: "wrng".to_string(),
        human_final_surface: "wrong".to_string(),
        reference_class: ReferenceClass::TranscriptionError,
        verification_basis: VerificationBasis::AudioListened,
        reviewer_identity_class: ReferenceReviewerIdentityClass::OwnerBlindReviewer,
        reviewed_at_unix_ms: 1_700_000_000_000,
    }];
    human_reference.assessment = HumanFinalReference::derive_assessment(
        &human_reference.reference_revision,
        &human_reference.input_identity,
        &human_reference.records,
    )
    .expect("derive assessment");

    let snapshot = build_snapshot(
        vec![glossary_proposal(
            "det-prop-001",
            1,
            0,
            0,
            4,
            "wrng",
            "wrng",
        )],
        DetectorProposalSnapshotState::Frozen,
    );

    let join = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive join");

    assert_eq!(
        join.detector_dispositions[0].disposition,
        DetectorReferenceMatchDisposition::DetectorWrongCorrection
    );
}

#[test]
fn exact_primary_reference_stays_resolved_when_other_detector_has_overlap_candidate() {
    let (envelope, seal, coverage, _human_reference, _snapshot, bundle, context, adjudication) =
        join_stack();
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

    let snapshot = build_snapshot(
        vec![
            glossary_proposal("det-prop-exact", 1, 0, 0, 4, "wrng", "wrong"),
            glossary_proposal("det-prop-overlap", 1, 0, 2, 6, "wrng", "wrong"),
        ],
        DetectorProposalSnapshotState::Frozen,
    );

    let join = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive join");

    assert_eq!(join.state, DetectorReferenceJoinState::Resolved);
    assert_eq!(
        join.reference_dispositions[0].disposition,
        DetectorReferenceMatchDisposition::ExactMatch
    );
    assert_eq!(
        join.detector_dispositions
            .iter()
            .find(|record| record.detector_proposal_id.as_str() == "det-prop-exact")
            .expect("exact detector")
            .disposition,
        DetectorReferenceMatchDisposition::ExactMatch
    );
    assert_eq!(
        join.detector_dispositions
            .iter()
            .find(|record| record.detector_proposal_id.as_str() == "det-prop-overlap")
            .expect("overlap detector")
            .disposition,
        DetectorReferenceMatchDisposition::UnmatchedDetector
    );
    assert_eq!(join.assessment.unresolved_overlap_edge_count, 0);
}

#[test]
fn exact_anchor_adjudication_pair_is_rejected() {
    let (mut envelope, seal, coverage, _human_reference, _snapshot, bundle, context, _adjudication) =
        join_stack();
    envelope.lifecycle_state = RunLifecycleState::AssistedReview;

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

    let snapshot = build_snapshot(
        vec![
            glossary_proposal("det-prop-a", 1, 0, 0, 4, "wrng", "wrong"),
            observed_error_proposal("det-prop-b", 1, 0, 0, 4, "wrng", "wrong"),
        ],
        DetectorProposalSnapshotState::Frozen,
    );

    let adjudication = frozen_adjudication_set(vec![adjudication_record(
        "adj-001",
        "det-prop-b",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);

    assert!(matches!(
        DetectorReferenceJoin::derive(
            &context,
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &bundle,
            &adjudication,
        ),
        Err(
            DetectorReferenceJoinError::AdjudicationPairNotAdmissiblePhase3Edge {
                reason: Phase3AdjudicationRejectionReason::ReferenceAssignedByExactPhase,
                ..
            }
        )
    ));
}

#[test]
fn exact_primary_wins_over_adjudicated_overlap_on_same_reference() {
    let (mut envelope, seal, coverage, _human_reference, _snapshot, bundle, context, _adjudication) =
        join_stack();
    envelope.lifecycle_state = RunLifecycleState::AssistedReview;

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

    let snapshot = build_snapshot(
        vec![
            glossary_proposal("det-prop-overlap", 1, 0, 2, 6, "wrng", "wrong"),
            glossary_proposal("det-prop-exact", 1, 0, 0, 4, "wrng", "wrong"),
        ],
        DetectorProposalSnapshotState::Frozen,
    );

    let adjudication = frozen_adjudication_set(vec![adjudication_record(
        "adj-001",
        "det-prop-overlap",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);

    assert!(matches!(
        DetectorReferenceJoin::derive(
            &context,
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &bundle,
            &adjudication,
        ),
        Err(
            DetectorReferenceJoinError::AdjudicationPairNotAdmissiblePhase3Edge {
                reason: Phase3AdjudicationRejectionReason::ReferenceAssignedByExactPhase,
                ..
            }
        )
    ));
}

#[test]
fn extraneous_adjudication_rejected_when_reference_exact_assigned() {
    let (mut envelope, seal, coverage, _human_reference, _snapshot, bundle, context, _adjudication) =
        join_stack();
    envelope.lifecycle_state = RunLifecycleState::AssistedReview;

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

    let snapshot = build_snapshot(
        vec![
            glossary_proposal("det-prop-overlap", 1, 0, 2, 6, "wrng", "wrong"),
            glossary_proposal("det-prop-exact", 1, 0, 0, 4, "wrng", "wrong"),
        ],
        DetectorProposalSnapshotState::Frozen,
    );

    let adjudication = frozen_adjudication_set(vec![adjudication_record(
        "adj-001",
        "det-prop-overlap",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);

    assert!(matches!(
        DetectorReferenceJoin::derive(
            &context,
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &bundle,
            &adjudication,
        ),
        Err(
            DetectorReferenceJoinError::AdjudicationPairNotAdmissiblePhase3Edge {
                reason: Phase3AdjudicationRejectionReason::ReferenceAssignedByExactPhase,
                ..
            }
        )
    ));
}

#[test]
fn stored_top_level_binding_mismatch_rejects_mutated_run_id() {
    let (envelope, seal, coverage, human_reference, snapshot, bundle, context, adjudication) =
        join_stack();
    let mut join = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive join");
    join.run_id = RunId::new("run-other").expect("run id");

    assert!(matches!(
        join.validate_against(
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &bundle,
            &adjudication,
        ),
        Err(DetectorReferenceJoinError::TopLevelBindingMismatch { field: "run_id" })
    ));
}

#[test]
fn unknown_adjudication_detector_proposal_rejects_join() {
    let (mut envelope, seal, coverage, human_reference, snapshot, bundle, context, _adjudication) =
        join_stack();
    envelope.lifecycle_state = RunLifecycleState::AssistedReview;

    let adjudication = frozen_adjudication_set(vec![adjudication_record(
        "adj-unknown-det",
        "det-prop-missing",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);

    assert!(matches!(
        DetectorReferenceJoin::derive(
            &context,
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &bundle,
            &adjudication,
        ),
        Err(DetectorReferenceJoinError::UnknownAdjudicationDetectorProposal { .. })
    ));
}

#[test]
fn unknown_adjudication_reference_error_rejects_join() {
    let (mut envelope, seal, coverage, human_reference, snapshot, bundle, context, _adjudication) =
        join_stack();
    envelope.lifecycle_state = RunLifecycleState::AssistedReview;

    let adjudication = frozen_adjudication_set(vec![adjudication_record(
        "adj-unknown-ref",
        "det-prop-001",
        "ref-err-missing",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);

    assert!(matches!(
        DetectorReferenceJoin::derive(
            &context,
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &bundle,
            &adjudication,
        ),
        Err(DetectorReferenceJoinError::UnknownAdjudicationReferenceError { .. })
    ));
}

#[test]
fn extraneous_adjudication_record_rejects_whole_join() {
    let (
        mut envelope,
        seal,
        mut coverage,
        _human_reference,
        _snapshot,
        bundle,
        context,
        _adjudication,
    ) = join_stack();
    envelope.lifecycle_state = RunLifecycleState::AssistedReview;
    coverage.records = vec![record(1, ReferenceCueDisposition::TranscriptionError)];
    coverage.expected_universe = universe(&[1]);
    coverage.assessment =
        ReferenceCoverage::derive_assessment(&coverage.expected_universe, &coverage.records)
            .expect("derive assessment");

    let human_reference = HumanFinalReference {
        schema_revision: HUMAN_FINAL_REFERENCE_SCHEMA.to_string(),
        run_id: coverage.run_id.clone(),
        input_identity: coverage.input_identity.clone(),
        seal_id: coverage.seal_id.clone(),
        reference_revision: coverage.reference_revision.clone(),
        records: vec![
            reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong"),
            reference_error_record("ref-err-2", 1, 0, 8, 12, "wrong"),
        ],
        state: HumanFinalReferenceState::Sealed,
        assessment: HumanFinalReference::derive_assessment(
            &coverage.reference_revision,
            &coverage.input_identity,
            &[
                reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong"),
                reference_error_record("ref-err-2", 1, 0, 8, 12, "wrong"),
            ],
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

    let adjudication = frozen_adjudication_set(vec![
        adjudication_record(
            "adj-valid",
            "det-prop-001",
            "ref-err-1",
            OverlapAdjudicationResult::SameErrorSameCorrection,
        ),
        adjudication_record(
            "adj-extra",
            "det-prop-missing",
            "ref-err-1",
            OverlapAdjudicationResult::DifferentError,
        ),
    ]);
    assert_eq!(adjudication.records.len(), 2);

    let result = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    );
    assert!(
        matches!(
            result,
            Err(DetectorReferenceJoinError::UnknownAdjudicationDetectorProposal { .. })
        ),
        "unexpected derive result: {result:?}"
    );
}

#[test]
fn assisted_review_join_fails_validation_at_detector_execution() {
    let (
        mut assisted_envelope,
        seal,
        coverage,
        human_reference,
        _snapshot,
        bundle,
        context,
        _adjudication,
    ) = join_stack();
    assisted_envelope.lifecycle_state = RunLifecycleState::AssistedReview;

    let mut human_reference = human_reference;
    human_reference.records = vec![reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong")];
    human_reference.assessment = HumanFinalReference::derive_assessment(
        &human_reference.reference_revision,
        &human_reference.input_identity,
        &human_reference.records,
    )
    .expect("derive assessment");

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

    let adjudication = frozen_adjudication_set(vec![adjudication_record(
        "adj-001",
        "det-prop-001",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);

    let join = DetectorReferenceJoin::derive(
        &context,
        &assisted_envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive at assisted review");

    let mut detector_envelope = assisted_envelope.clone();
    detector_envelope.lifecycle_state = RunLifecycleState::DetectorExecution;

    assert!(matches!(
        join.validate_against(
            &detector_envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &bundle,
            &adjudication,
        ),
        Err(DetectorReferenceJoinError::AdjudicationChronologyViolation { .. })
    ));
}

#[test]
fn duplicate_detector_disposition_id_rejected_locally() {
    let (envelope, seal, coverage, human_reference, snapshot, bundle, context, adjudication) =
        join_stack();
    let mut join = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive join");
    let duplicate = join.detector_dispositions[0].clone();
    join.detector_dispositions.push(duplicate);

    assert!(matches!(
        join.validate(),
        Err(DetectorReferenceJoinError::DuplicateDetectorDispositionId { .. })
    ));
}

#[test]
fn join_stores_join_adjudication_artifact_id() {
    let (envelope, seal, coverage, human_reference, snapshot, bundle, context, adjudication) =
        join_stack();
    let join = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive join");

    assert_eq!(
        join.join_adjudication_artifact_id,
        join_adjudication_artifact_id()
    );
    assert_eq!(
        join.join_adjudication_artifact_id,
        adjudication.join_adjudication_artifact_id
    );
}

#[test]
fn caller_cannot_force_join_assessment_fields() {
    let (envelope, seal, coverage, human_reference, snapshot, bundle, context, adjudication) =
        join_stack();
    let mut join = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive join");
    join.assessment = DetectorReferenceJoinAssessment {
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
        fully_resolved: true,
    };

    assert!(matches!(
        join.validate(),
        Err(DetectorReferenceJoinError::AssessmentMismatch { .. }
            | DetectorReferenceJoinError::TerminalDispositionMismatch)
    ));
}

#[allow(clippy::too_many_arguments)]
fn assert_top_level_binding_mismatch(
    join: &DetectorReferenceJoin,
    envelope: &RunEnvelope,
    seal: &ReferenceSeal,
    coverage: &ReferenceCoverage,
    human_reference: &HumanFinalReference,
    snapshot: &DetectorProposalSnapshot,
    bundle: &ArtifactBundle,
    adjudication: &OverlapAdjudicationSet,
    mutate: impl FnOnce(&mut DetectorReferenceJoin),
    expected_field: &str,
) {
    let mut mutated = join.clone();
    mutate(&mut mutated);
    assert!(
        matches!(
            mutated.validate_against(
                envelope,
                seal,
                coverage,
                human_reference,
                snapshot,
                bundle,
                adjudication,
            ),
            Err(DetectorReferenceJoinError::TopLevelBindingMismatch { field })
                if field == expected_field
        ),
        "expected TopLevelBindingMismatch for field {expected_field}"
    );
}

#[test]
fn stored_top_level_binding_mismatch_rejects_every_authoritative_field() {
    let (envelope, seal, coverage, human_reference, snapshot, bundle, context, adjudication) =
        join_stack();
    let join = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive join");

    assert_top_level_binding_mismatch(
        &join,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
        |join| join.run_id = RunId::new("run-other").expect("run id"),
        "run_id",
    );
    assert_top_level_binding_mismatch(
        &join,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
        |join| join.input_identity.transcript_revision_id = "rev-other".to_string(),
        "input_identity",
    );
    assert_top_level_binding_mismatch(
        &join,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
        |join| join.calibration_validity = CalibrationValidityMode::DetectorAssisted,
        "calibration_validity",
    );
    assert_top_level_binding_mismatch(
        &join,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
        |join| join.reference_seal_id = ReferenceSealId::new("seal-other").expect("seal id"),
        "reference_seal_id",
    );
    assert_top_level_binding_mismatch(
        &join,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
        |join| {
            join.reference_revision =
                ReferenceRevisionId::new("ref-rev-other").expect("reference revision")
        },
        "reference_revision",
    );
    assert_top_level_binding_mismatch(
        &join,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
        |join| {
            join.reference_coverage_id =
                ReferenceCoverageId::new("coverage-other").expect("coverage id")
        },
        "reference_coverage_id",
    );
    assert_top_level_binding_mismatch(
        &join,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
        |join| {
            join.detector_snapshot_revision =
                DetectorSnapshotRevisionId::new("snap-rev-other").expect("snapshot revision")
        },
        "detector_snapshot_revision",
    );
    assert_top_level_binding_mismatch(
        &join,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
        |join| {
            join.detector_output_artifact_id =
                ArtifactId::new("detector-output-other").expect("artifact id")
        },
        "detector_output_artifact_id",
    );
    assert_top_level_binding_mismatch(
        &join,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
        |join| {
            join.evaluation_join_artifact_id =
                ArtifactId::new("evaluation-join-other").expect("artifact id")
        },
        "evaluation_join_artifact_id",
    );
    assert_top_level_binding_mismatch(
        &join,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
        |join| {
            join.join_adjudication_artifact_id =
                ArtifactId::new("join-adj-other").expect("artifact id")
        },
        "join_adjudication_artifact_id",
    );
    assert_top_level_binding_mismatch(
        &join,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
        |join| join.join_purpose = DetectorReferenceJoinPurpose::DiagnosticOnly,
        "join_purpose",
    );

    let mut overlap_policy = join.clone();
    overlap_policy.overlap_rule_revision = "other-overlap-rev".to_string();
    assert!(matches!(
        overlap_policy.validate(),
        Err(DetectorReferenceJoinError::UnsupportedPolicyRevision { .. })
    ));

    let mut correction_policy = join.clone();
    correction_policy.correction_equality_revision = "other-nfc-rev".to_string();
    assert!(matches!(
        correction_policy.validate(),
        Err(DetectorReferenceJoinError::UnsupportedPolicyRevision { .. })
    ));

    let mut cardinality_policy = join.clone();
    cardinality_policy.alternative_cardinality_policy = "other-cardinality".to_string();
    assert!(matches!(
        cardinality_policy.validate(),
        Err(DetectorReferenceJoinError::UnsupportedPolicyRevision { .. })
    ));
}

#[test]
fn unknown_adjudication_both_ids_reject_join() {
    let (mut envelope, seal, coverage, human_reference, snapshot, bundle, context, _adjudication) =
        join_stack();
    envelope.lifecycle_state = RunLifecycleState::AssistedReview;

    let adjudication = frozen_adjudication_set(vec![adjudication_record(
        "adj-both-unknown",
        "det-prop-missing",
        "ref-err-missing",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);

    assert!(matches!(
        DetectorReferenceJoin::derive(
            &context,
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &bundle,
            &adjudication,
        ),
        Err(DetectorReferenceJoinError::UnknownAdjudicationDetectorProposal { .. })
    ));
}

#[test]
fn disjoint_adjudication_pair_rejects_whole_join() {
    let (
        mut envelope,
        seal,
        mut coverage,
        _human_reference,
        _snapshot,
        bundle,
        context,
        _adjudication,
    ) = join_stack();
    envelope.lifecycle_state = RunLifecycleState::AssistedReview;

    let human_reference = HumanFinalReference {
        schema_revision: HUMAN_FINAL_REFERENCE_SCHEMA.to_string(),
        run_id: coverage.run_id.clone(),
        input_identity: coverage.input_identity.clone(),
        seal_id: coverage.seal_id.clone(),
        reference_revision: coverage.reference_revision.clone(),
        records: vec![
            reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong"),
            reference_error_record("ref-err-2", 1, 0, 8, 12, "wrong"),
        ],
        state: HumanFinalReferenceState::Sealed,
        assessment: HumanFinalReference::derive_assessment(
            &coverage.reference_revision,
            &coverage.input_identity,
            &[
                reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong"),
                reference_error_record("ref-err-2", 1, 0, 8, 12, "wrong"),
            ],
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

    let adjudication = frozen_adjudication_set(vec![
        adjudication_record(
            "adj-valid",
            "det-prop-001",
            "ref-err-1",
            OverlapAdjudicationResult::SameErrorSameCorrection,
        ),
        adjudication_record(
            "adj-disjoint",
            "det-prop-001",
            "ref-err-2",
            OverlapAdjudicationResult::DifferentError,
        ),
    ]);

    assert!(matches!(
        DetectorReferenceJoin::derive(
            &context,
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &bundle,
            &adjudication,
        ),
        Err(
            DetectorReferenceJoinError::AdjudicationPairNotAdmissiblePhase3Edge {
                reason: Phase3AdjudicationRejectionReason::DisjointAnchor,
                ..
            }
        )
    ));
}

#[test]
fn adjudication_pair_removed_by_snapshot_revision_rejects_join() {
    let (mut envelope, seal, coverage, human_reference, snapshot, bundle, context, _adjudication) =
        join_stack();
    envelope.lifecycle_state = RunLifecycleState::AssistedReview;

    let mut adjudication = frozen_adjudication_set(vec![adjudication_record(
        "adj-overlap",
        "det-prop-001",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);
    adjudication.detector_snapshot_revision =
        DetectorSnapshotRevisionId::new("det-snap-rev-stale").expect("snapshot revision");

    assert!(matches!(
        DetectorReferenceJoin::derive(
            &context,
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &bundle,
            &adjudication,
        ),
        Err(DetectorReferenceJoinError::AdjudicationValidation(
            OverlapAdjudicationValidationError::DetectorSnapshotRevisionMismatch
        ))
    ));
}

#[test]
fn exact_assigned_detector_does_not_create_overlap_against_other_references() {
    let (envelope, seal, mut coverage, _human_reference, _snapshot, bundle, context, adjudication) =
        join_stack();
    coverage.records = vec![record(1, ReferenceCueDisposition::TranscriptionError)];
    coverage.expected_universe = universe(&[1]);
    coverage.assessment =
        ReferenceCoverage::derive_assessment(&coverage.expected_universe, &coverage.records)
            .expect("derive assessment");

    let human_reference = HumanFinalReference {
        schema_revision: HUMAN_FINAL_REFERENCE_SCHEMA.to_string(),
        run_id: coverage.run_id.clone(),
        input_identity: coverage.input_identity.clone(),
        seal_id: coverage.seal_id.clone(),
        reference_revision: coverage.reference_revision.clone(),
        records: vec![
            reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong"),
            reference_error_record("ref-err-2", 1, 0, 2, 8, "wrong"),
        ],
        state: HumanFinalReferenceState::Sealed,
        assessment: HumanFinalReference::derive_assessment(
            &coverage.reference_revision,
            &coverage.input_identity,
            &[
                reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong"),
                reference_error_record("ref-err-2", 1, 0, 2, 8, "wrong"),
            ],
        )
        .expect("derive assessment"),
    };
    coverage.assessment.total_eligible_transcription_errors = human_reference
        .assessment
        .recall_eligible_transcription_error_count;

    let snapshot = build_snapshot(
        vec![glossary_proposal(
            "det-prop-exact",
            1,
            0,
            0,
            4,
            "wrng",
            "wrong",
        )],
        DetectorProposalSnapshotState::Frozen,
    );

    let join = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive join");

    assert_eq!(join.state, DetectorReferenceJoinState::Resolved);
    assert!(
        join.edges
            .iter()
            .all(|edge| edge.anchor_relation != JoinAnchorRelation::Overlap),
        "exact-assigned detector must not emit overlap edges against other references"
    );
    assert_eq!(
        join.reference_dispositions
            .iter()
            .find(|record| record.reference_error_id.as_str() == "ref-err-2")
            .expect("second reference")
            .disposition,
        DetectorReferenceMatchDisposition::UnmatchedReference
    );
}

#[test]
fn duplicate_reference_disposition_id_rejected_locally() {
    let (envelope, seal, coverage, human_reference, snapshot, bundle, context, adjudication) =
        join_stack();
    let mut join = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive join");
    let duplicate = join.reference_dispositions[0].clone();
    join.reference_dispositions.push(duplicate);

    assert!(matches!(
        join.validate(),
        Err(DetectorReferenceJoinError::DuplicateReferenceDispositionId { .. })
    ));
}

#[test]
fn terminal_disposition_mismatch_rejected_locally() {
    let (envelope, seal, coverage, human_reference, snapshot, bundle, context, adjudication) =
        join_stack();
    let mut join = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive join");
    join.detector_dispositions.pop();

    assert!(matches!(
        join.validate(),
        Err(DetectorReferenceJoinError::TerminalDispositionMismatch)
    ));
}

#[test]
fn primary_assignment_inconsistent_rejected_locally() {
    let (envelope, seal, coverage, human_reference, snapshot, bundle, context, adjudication) =
        join_stack();
    let mut join = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive join");
    join.reference_dispositions[0].primary_detector_proposal_id =
        Some(DetectorProposalId::new("det-prop-other").expect("proposal id"));
    assert!(matches!(
        join.validate(),
        Err(DetectorReferenceJoinError::PrimaryTopologyViolation { .. })
    ));
}

#[test]
fn requires_adjudication_state_inconsistent_rejected_locally() {
    let (envelope, seal, coverage, human_reference, snapshot, bundle, context, adjudication) =
        join_stack();
    let mut join = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive join");
    join.state = DetectorReferenceJoinState::RequiresAdjudication;

    assert!(matches!(
        join.validate(),
        Err(DetectorReferenceJoinError::RequiresAdjudicationStateInconsistent)
    ));
}

#[test]
fn duplicate_join_record_id_rejected_locally() {
    let (envelope, seal, coverage, human_reference, snapshot, bundle, context, adjudication) =
        join_stack();
    let mut join = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive join");
    let duplicate = join.edges[0].join_record_id.clone();
    join.edges.push(DetectorReferenceJoinEdge {
        join_record_id: duplicate,
        ..join.edges[0].clone()
    });

    assert!(matches!(
        join.validate(),
        Err(DetectorReferenceJoinError::DuplicateJoinRecordId { .. })
    ));
}

// --- SOL-JOIN-FC-01 / SOL-JOIN-FC-02 regressions ---

fn assert_primary_topology_violation(
    join: &DetectorReferenceJoin,
    violation: PrimaryTopologyViolation,
) {
    match join.validate() {
        Err(DetectorReferenceJoinError::PrimaryTopologyViolation {
            violation: observed,
        }) if observed == violation => {}
        other => panic!("expected PrimaryTopologyViolation {violation:?}, got {other:?}"),
    }
}

fn assisted_review_stack() -> (
    RunEnvelope,
    ReferenceSeal,
    ReferenceCoverage,
    HumanFinalReference,
    ArtifactBundle,
    DetectorReferenceJoinContext,
) {
    let (mut envelope, seal, coverage, human_reference, _snapshot, bundle, context, _adjudication) =
        join_stack();
    envelope.lifecycle_state = RunLifecycleState::AssistedReview;
    (envelope, seal, coverage, human_reference, bundle, context)
}

fn derive_assisted_overlap_accepted_join() -> DetectorReferenceJoin {
    let (envelope, seal, coverage, mut human_reference, bundle, context) = assisted_review_stack();
    human_reference.records = vec![reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong")];
    human_reference.assessment = HumanFinalReference::derive_assessment(
        &human_reference.reference_revision,
        &human_reference.input_identity,
        &human_reference.records,
    )
    .expect("derive assessment");
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
    let adjudication = frozen_adjudication_set(vec![adjudication_record(
        "adj-001",
        "det-prop-001",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);
    DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive overlap accepted join")
}

fn derive_assisted_overlap_wrong_correction_join() -> DetectorReferenceJoin {
    let (envelope, seal, coverage, mut human_reference, bundle, context) = assisted_review_stack();
    human_reference.records = vec![reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong")];
    human_reference.assessment = HumanFinalReference::derive_assessment(
        &human_reference.reference_revision,
        &human_reference.input_identity,
        &human_reference.records,
    )
    .expect("derive assessment");
    let snapshot = build_snapshot(
        vec![glossary_proposal(
            "det-prop-001",
            1,
            0,
            2,
            6,
            "wrng",
            "wright",
        )],
        DetectorProposalSnapshotState::Frozen,
    );
    let adjudication = frozen_adjudication_set(vec![adjudication_record(
        "adj-001",
        "det-prop-001",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorWrongCorrection,
    )]);
    DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive overlap wrong correction join")
}

fn derive_assisted_exact_join() -> DetectorReferenceJoin {
    let (envelope, seal, coverage, human_reference, bundle, context) = assisted_review_stack();
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
    let adjudication = empty_frozen_adjudication_set();
    DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive exact join")
}

fn primary_edge(join: &DetectorReferenceJoin) -> &DetectorReferenceJoinEdge {
    join.edges
        .iter()
        .find(|edge| edge.resolution == JoinEdgeResolution::PrimaryAssignment)
        .expect("primary edge")
}

#[test]
fn fc01_valid_adjudication_materializes_exactly_one_overlap_edge() {
    let join = derive_assisted_overlap_accepted_join();
    let overlap_primaries: Vec<_> = join
        .edges
        .iter()
        .filter(|edge| {
            edge.resolution == JoinEdgeResolution::PrimaryAssignment
                && edge.anchor_relation == JoinAnchorRelation::Overlap
        })
        .collect();
    assert_eq!(overlap_primaries.len(), 1);
    assert!(overlap_primaries[0].adjudication_id.is_some());
}

#[test]
fn fc01_detector_consumed_by_exact_assignment_rejects_adjudication() {
    let (envelope, seal, coverage, human_reference, bundle, context) = assisted_review_stack();
    let snapshot = build_snapshot(
        vec![glossary_proposal(
            "det-prop-exact",
            1,
            0,
            0,
            4,
            "wrng",
            "wrong",
        )],
        DetectorProposalSnapshotState::Frozen,
    );
    let adjudication = frozen_adjudication_set(vec![adjudication_record(
        "adj-001",
        "det-prop-exact",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);
    assert!(matches!(
        DetectorReferenceJoin::derive(
            &context,
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &bundle,
            &adjudication,
        ),
        Err(
            DetectorReferenceJoinError::AdjudicationPairNotAdmissiblePhase3Edge {
                reason: Phase3AdjudicationRejectionReason::DetectorAssignedByExactPhase,
                ..
            }
        )
    ));
}

#[test]
fn fc01_reference_consumed_by_exact_assignment_rejects_adjudication() {
    let (envelope, seal, coverage, human_reference, bundle, context) = assisted_review_stack();
    let snapshot = build_snapshot(
        vec![
            glossary_proposal("det-prop-overlap", 1, 0, 2, 6, "wrng", "wrong"),
            glossary_proposal("det-prop-exact", 1, 0, 0, 4, "wrng", "wrong"),
        ],
        DetectorProposalSnapshotState::Frozen,
    );
    let adjudication = frozen_adjudication_set(vec![adjudication_record(
        "adj-001",
        "det-prop-overlap",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);
    assert!(matches!(
        DetectorReferenceJoin::derive(
            &context,
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &bundle,
            &adjudication,
        ),
        Err(
            DetectorReferenceJoinError::AdjudicationPairNotAdmissiblePhase3Edge {
                reason: Phase3AdjudicationRejectionReason::ReferenceAssignedByExactPhase,
                ..
            }
        )
    ));
}

#[test]
fn fc01_verification_ineligible_adjudication_rejected() {
    let (envelope, seal, mut coverage, _human_reference, bundle, context) = assisted_review_stack();
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
            2,
            6,
            "wrng",
            "wrong",
        )],
        DetectorProposalSnapshotState::Frozen,
    );
    let adjudication = frozen_adjudication_set(vec![adjudication_record(
        "adj-001",
        "det-prop-001",
        "ref-err-tco",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);
    assert!(matches!(
        DetectorReferenceJoin::derive(
            &context,
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &bundle,
            &adjudication,
        ),
        Err(
            DetectorReferenceJoinError::AdjudicationPairNotAdmissiblePhase3Edge {
                reason: Phase3AdjudicationRejectionReason::VerificationIneligibleReference,
                ..
            }
        )
    ));
}

#[test]
fn fc01_excluded_reference_class_adjudication_rejected() {
    let (envelope, seal, mut coverage, _human_reference, bundle, context) = assisted_review_stack();
    coverage.records = vec![record(1, ReferenceCueDisposition::NoTranscriptionError)];
    coverage.expected_universe = universe(&[1]);
    coverage.assessment =
        ReferenceCoverage::derive_assessment(&coverage.expected_universe, &coverage.records)
            .expect("derive assessment");
    coverage.assessment.total_eligible_transcription_errors = 0;
    let human_reference = HumanFinalReference {
        schema_revision: HUMAN_FINAL_REFERENCE_SCHEMA.to_string(),
        run_id: coverage.run_id.clone(),
        input_identity: coverage.input_identity.clone(),
        seal_id: coverage.seal_id.clone(),
        reference_revision: coverage.reference_revision.clone(),
        records: vec![ReferenceErrorRecord {
            reference_error_id: ReferenceErrorId::new("ref-err-style").expect("error id"),
            reference_revision: ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION)
                .expect("revision"),
            input_identity: input_identity(),
            source_anchor: reference_source_anchor(1, 0, 0, 4),
            original_surface: "wrng".to_string(),
            human_final_surface: "wrong".to_string(),
            reference_class: ReferenceClass::StylePreference,
            verification_basis: VerificationBasis::AudioListened,
            reviewer_identity_class: ReferenceReviewerIdentityClass::OwnerBlindReviewer,
            reviewed_at_unix_ms: 1_700_000_000_000,
        }],
        state: HumanFinalReferenceState::Sealed,
        assessment: HumanFinalReference::derive_assessment(
            &coverage.reference_revision,
            &coverage.input_identity,
            &[ReferenceErrorRecord {
                reference_error_id: ReferenceErrorId::new("ref-err-style").expect("error id"),
                reference_revision: ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION)
                    .expect("revision"),
                input_identity: input_identity(),
                source_anchor: reference_source_anchor(1, 0, 0, 4),
                original_surface: "wrng".to_string(),
                human_final_surface: "wrong".to_string(),
                reference_class: ReferenceClass::StylePreference,
                verification_basis: VerificationBasis::AudioListened,
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
            2,
            6,
            "wrng",
            "wrong",
        )],
        DetectorProposalSnapshotState::Frozen,
    );
    let adjudication = frozen_adjudication_set(vec![adjudication_record(
        "adj-001",
        "det-prop-001",
        "ref-err-style",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);
    let result = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    );
    assert!(
        matches!(
            result,
            Err(
                DetectorReferenceJoinError::AdjudicationPairNotAdmissiblePhase3Edge {
                    reason: Phase3AdjudicationRejectionReason::NonTranscriptionErrorReference,
                    ..
                }
            )
        ),
        "unexpected derive result: {result:?}"
    );
}

#[test]
fn fc01_unknown_detector_adjudication_rejected() {
    let (envelope, seal, coverage, human_reference, bundle, context) = assisted_review_stack();
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
    let adjudication = frozen_adjudication_set(vec![adjudication_record(
        "adj-001",
        "det-prop-missing",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);
    assert!(matches!(
        DetectorReferenceJoin::derive(
            &context,
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &bundle,
            &adjudication,
        ),
        Err(DetectorReferenceJoinError::UnknownAdjudicationDetectorProposal { .. })
    ));
}

#[test]
fn fc01_unknown_reference_adjudication_rejected() {
    let (envelope, seal, coverage, human_reference, bundle, context) = assisted_review_stack();
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
    let adjudication = frozen_adjudication_set(vec![adjudication_record(
        "adj-001",
        "det-prop-001",
        "ref-err-missing",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);
    assert!(matches!(
        DetectorReferenceJoin::derive(
            &context,
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &bundle,
            &adjudication,
        ),
        Err(DetectorReferenceJoinError::UnknownAdjudicationReferenceError { .. })
    ));
}

#[test]
fn fc01_mixed_valid_and_inadmissible_record_fails_whole_derivation() {
    let (envelope, seal, mut coverage, _human_reference, bundle, context) = assisted_review_stack();
    coverage.records = vec![record(1, ReferenceCueDisposition::TranscriptionError)];
    coverage.expected_universe = universe(&[1]);
    coverage.assessment =
        ReferenceCoverage::derive_assessment(&coverage.expected_universe, &coverage.records)
            .expect("derive assessment");
    let human_reference = HumanFinalReference {
        schema_revision: HUMAN_FINAL_REFERENCE_SCHEMA.to_string(),
        run_id: coverage.run_id.clone(),
        input_identity: coverage.input_identity.clone(),
        seal_id: coverage.seal_id.clone(),
        reference_revision: coverage.reference_revision.clone(),
        records: vec![
            reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong"),
            reference_error_record("ref-err-2", 1, 0, 8, 12, "wrong"),
        ],
        state: HumanFinalReferenceState::Sealed,
        assessment: HumanFinalReference::derive_assessment(
            &coverage.reference_revision,
            &coverage.input_identity,
            &[
                reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong"),
                reference_error_record("ref-err-2", 1, 0, 8, 12, "wrong"),
            ],
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
    let adjudication = frozen_adjudication_set(vec![
        adjudication_record(
            "adj-valid",
            "det-prop-001",
            "ref-err-1",
            OverlapAdjudicationResult::SameErrorSameCorrection,
        ),
        adjudication_record(
            "adj-extra",
            "det-prop-missing",
            "ref-err-1",
            OverlapAdjudicationResult::DifferentError,
        ),
    ]);
    assert!(matches!(
        DetectorReferenceJoin::derive(
            &context,
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &bundle,
            &adjudication,
        ),
        Err(DetectorReferenceJoinError::UnknownAdjudicationDetectorProposal { .. })
    ));
}

#[test]
fn fc01_stale_record_after_source_revision_change_rejected() {
    let (envelope, seal, coverage, human_reference, bundle, context) = assisted_review_stack();
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
    let mut adjudication = frozen_adjudication_set(vec![adjudication_record(
        "adj-001",
        "det-prop-001",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);
    adjudication.detector_snapshot_revision =
        DetectorSnapshotRevisionId::new("det-snap-rev-stale").expect("snapshot revision");
    assert!(matches!(
        DetectorReferenceJoin::derive(
            &context,
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &bundle,
            &adjudication,
        ),
        Err(DetectorReferenceJoinError::AdjudicationValidation(
            OverlapAdjudicationValidationError::DetectorSnapshotRevisionMismatch
        ))
    ));
}

#[test]
fn fc01_partial_adjudication_materializes_consumed_records_only() {
    let (envelope, seal, mut coverage, _human_reference, bundle, context) = assisted_review_stack();
    coverage.records = vec![record(1, ReferenceCueDisposition::TranscriptionError)];
    coverage.expected_universe = universe(&[1]);
    coverage.assessment =
        ReferenceCoverage::derive_assessment(&coverage.expected_universe, &coverage.records)
            .expect("derive assessment");
    let human_reference = HumanFinalReference {
        schema_revision: HUMAN_FINAL_REFERENCE_SCHEMA.to_string(),
        run_id: coverage.run_id.clone(),
        input_identity: coverage.input_identity.clone(),
        seal_id: coverage.seal_id.clone(),
        reference_revision: coverage.reference_revision.clone(),
        records: vec![
            reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong"),
            reference_error_record("ref-err-2", 1, 0, 8, 12, "wrong"),
        ],
        state: HumanFinalReferenceState::Sealed,
        assessment: HumanFinalReference::derive_assessment(
            &coverage.reference_revision,
            &coverage.input_identity,
            &[
                reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong"),
                reference_error_record("ref-err-2", 1, 0, 8, 12, "wrong"),
            ],
        )
        .expect("derive assessment"),
    };
    coverage.assessment.total_eligible_transcription_errors = human_reference
        .assessment
        .recall_eligible_transcription_error_count;
    let snapshot = build_snapshot(
        vec![
            glossary_proposal("det-prop-a", 1, 0, 2, 6, "wrng", "wrong"),
            glossary_proposal("det-prop-b", 1, 0, 10, 14, "wrng", "wrong"),
        ],
        DetectorProposalSnapshotState::Frozen,
    );
    let adjudication = frozen_adjudication_set(vec![adjudication_record(
        "adj-001",
        "det-prop-a",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);
    let join = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive partial adjudication join");
    assert_eq!(join.state, DetectorReferenceJoinState::RequiresAdjudication);
    assert_eq!(join.assessment.accepted_overlap_count, 1);
    assert_eq!(join.assessment.unresolved_overlap_edge_count, 1);
}

#[test]
fn fc01_exact_only_join_rejects_stale_overlap_adjudication() {
    let (envelope, seal, coverage, human_reference, _snapshot, bundle, context, _adjudication) =
        join_stack();
    let snapshot = build_snapshot(
        vec![glossary_proposal(
            "det-prop-exact",
            1,
            0,
            0,
            4,
            "wrng",
            "wrong",
        )],
        DetectorProposalSnapshotState::Frozen,
    );
    let adjudication = frozen_adjudication_set(vec![adjudication_record(
        "adj-stale",
        "det-prop-exact",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);
    assert!(matches!(
        DetectorReferenceJoin::derive(
            &context,
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &bundle,
            &adjudication,
        ),
        Err(DetectorReferenceJoinError::AdjudicationRecordsForbiddenAtDetectorExecution)
    ));
}

#[test]
fn fc01_shuffled_adjudication_order_produces_identical_join() {
    let (envelope, seal, mut coverage, _human_reference, bundle, context) = assisted_review_stack();
    coverage.records = vec![record(1, ReferenceCueDisposition::TranscriptionError)];
    coverage.expected_universe = universe(&[1]);
    coverage.assessment =
        ReferenceCoverage::derive_assessment(&coverage.expected_universe, &coverage.records)
            .expect("derive assessment");
    let human_reference = HumanFinalReference {
        schema_revision: HUMAN_FINAL_REFERENCE_SCHEMA.to_string(),
        run_id: coverage.run_id.clone(),
        input_identity: coverage.input_identity.clone(),
        seal_id: coverage.seal_id.clone(),
        reference_revision: coverage.reference_revision.clone(),
        records: vec![
            reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong"),
            reference_error_record("ref-err-2", 1, 0, 8, 12, "wrong"),
        ],
        state: HumanFinalReferenceState::Sealed,
        assessment: HumanFinalReference::derive_assessment(
            &coverage.reference_revision,
            &coverage.input_identity,
            &[
                reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong"),
                reference_error_record("ref-err-2", 1, 0, 8, 12, "wrong"),
            ],
        )
        .expect("derive assessment"),
    };
    coverage.assessment.total_eligible_transcription_errors = human_reference
        .assessment
        .recall_eligible_transcription_error_count;
    let snapshot = build_snapshot(
        vec![
            glossary_proposal("det-prop-a", 1, 0, 2, 6, "wrng", "wrong"),
            glossary_proposal("det-prop-b", 1, 0, 10, 14, "wrng", "wrong"),
        ],
        DetectorProposalSnapshotState::Frozen,
    );
    let forward = frozen_adjudication_set(vec![
        adjudication_record(
            "adj-a",
            "det-prop-a",
            "ref-err-1",
            OverlapAdjudicationResult::SameErrorSameCorrection,
        ),
        adjudication_record(
            "adj-b",
            "det-prop-b",
            "ref-err-2",
            OverlapAdjudicationResult::SameErrorSameCorrection,
        ),
    ]);
    let reverse = frozen_adjudication_set(vec![
        adjudication_record(
            "adj-b",
            "det-prop-b",
            "ref-err-2",
            OverlapAdjudicationResult::SameErrorSameCorrection,
        ),
        adjudication_record(
            "adj-a",
            "det-prop-a",
            "ref-err-1",
            OverlapAdjudicationResult::SameErrorSameCorrection,
        ),
    ]);
    let forward_join = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &forward,
    )
    .expect("forward join");
    let reverse_join = DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &reverse,
    )
    .expect("reverse join");
    assert_eq!(forward_join.edges, reverse_join.edges);
    assert_eq!(
        forward_join.detector_dispositions,
        reverse_join.detector_dispositions
    );
    assert_eq!(
        forward_join.reference_dispositions,
        reverse_join.reference_dispositions
    );
}

#[test]
fn fc02_primary_edge_with_detector_disposition_only_fails() {
    let mut join = derive_assisted_overlap_accepted_join();
    join.reference_dispositions[0].primary_detector_proposal_id = None;
    join.reference_dispositions[0].disposition =
        DetectorReferenceMatchDisposition::UnmatchedReference;
    assert_primary_topology_violation(&join, PrimaryTopologyViolation::PrimaryTopologySetMismatch);
}

#[test]
fn fc02_primary_edge_with_reference_disposition_only_fails() {
    let mut join = derive_assisted_overlap_accepted_join();
    join.detector_dispositions[0].primary_reference_error_id = None;
    join.detector_dispositions[0].disposition =
        DetectorReferenceMatchDisposition::UnmatchedDetector;
    assert_primary_topology_violation(&join, PrimaryTopologyViolation::PrimaryTopologySetMismatch);
}

#[test]
fn fc02_both_dispositions_without_primary_edge_fails() {
    let mut join = derive_assisted_overlap_accepted_join();
    join.detector_dispositions[0].primary_reference_error_id = None;
    join.detector_dispositions[0].disposition =
        DetectorReferenceMatchDisposition::UnmatchedDetector;
    join.reference_dispositions[0].primary_detector_proposal_id = None;
    join.reference_dispositions[0].disposition =
        DetectorReferenceMatchDisposition::UnmatchedReference;
    assert_primary_topology_violation(&join, PrimaryTopologyViolation::PrimaryTopologySetMismatch);
}

#[test]
fn fc02_extra_primary_edge_fails() {
    let mut join = derive_assisted_overlap_accepted_join();
    let template = primary_edge(&join).clone();
    join.edges.push(DetectorReferenceJoinEdge {
        join_record_id: JoinRecordId::new("join-edge-extra").expect("join record id"),
        detector_proposal_id: DetectorProposalId::new("det-prop-extra").expect("proposal id"),
        reference_error_id: ReferenceErrorId::new("ref-err-extra").expect("reference id"),
        ..template
    });
    assert_primary_topology_violation(&join, PrimaryTopologyViolation::PrimaryTopologySetMismatch);
}

#[test]
fn fc02_extra_detector_side_primary_id_fails() {
    let mut join = derive_assisted_overlap_accepted_join();
    join.detector_dispositions.push(
        vox_proof::detector_reference_join::DetectorJoinDispositionRecord {
            detector_proposal_id: DetectorProposalId::new("det-prop-extra").expect("proposal id"),
            disposition: DetectorReferenceMatchDisposition::AcceptedOverlap,
            primary_reference_error_id: Some(
                ReferenceErrorId::new("ref-err-1").expect("reference id"),
            ),
        },
    );
    join.assessment.detector_proposal_count += 1;
    assert_primary_topology_violation(&join, PrimaryTopologyViolation::PrimaryTopologySetMismatch);
}

#[test]
fn fc02_extra_reference_side_primary_id_fails() {
    let mut join = derive_assisted_overlap_accepted_join();
    join.reference_dispositions.push(
        vox_proof::detector_reference_join::ReferenceJoinDispositionRecord {
            reference_error_id: ReferenceErrorId::new("ref-err-extra").expect("reference id"),
            disposition: DetectorReferenceMatchDisposition::AcceptedOverlap,
            primary_detector_proposal_id: Some(
                DetectorProposalId::new("det-prop-001").expect("proposal id"),
            ),
        },
    );
    join.assessment.reference_record_count += 1;
    assert_primary_topology_violation(&join, PrimaryTopologyViolation::PrimaryTopologySetMismatch);
}

#[test]
fn fc02_detector_and_reference_primary_pair_mismatch_fails() {
    let mut join = derive_assisted_overlap_accepted_join();
    join.reference_dispositions[0].primary_detector_proposal_id =
        Some(DetectorProposalId::new("det-prop-other").expect("proposal id"));
    assert_primary_topology_violation(&join, PrimaryTopologyViolation::PrimaryTopologySetMismatch);
}

#[test]
fn fc02_duplicate_detector_disposition_id_fails() {
    let mut join = derive_assisted_exact_join();
    join.detector_dispositions
        .push(join.detector_dispositions[0].clone());
    assert!(matches!(
        join.validate(),
        Err(DetectorReferenceJoinError::DuplicateDetectorDispositionId { .. })
    ));
}

#[test]
fn fc02_duplicate_reference_disposition_id_fails() {
    let mut join = derive_assisted_exact_join();
    join.reference_dispositions
        .push(join.reference_dispositions[0].clone());
    assert!(matches!(
        join.validate(),
        Err(DetectorReferenceJoinError::DuplicateReferenceDispositionId { .. })
    ));
}

#[test]
fn fc02_duplicate_primary_edge_pair_fails() {
    let mut join = derive_assisted_overlap_accepted_join();
    let template = primary_edge(&join).clone();
    join.edges.push(DetectorReferenceJoinEdge {
        join_record_id: JoinRecordId::new("join-edge-dup").expect("join record id"),
        ..template
    });
    assert_primary_topology_violation(&join, PrimaryTopologyViolation::DuplicatePrimaryEdgePair);
}

#[test]
fn fc02_two_primary_references_for_one_detector_fails() {
    let mut join = derive_assisted_overlap_accepted_join();
    let template = primary_edge(&join).clone();
    join.edges.push(DetectorReferenceJoinEdge {
        join_record_id: JoinRecordId::new("join-edge-second-ref").expect("join record id"),
        reference_error_id: ReferenceErrorId::new("ref-err-2").expect("reference id"),
        ..template
    });
    join.reference_dispositions.push(
        vox_proof::detector_reference_join::ReferenceJoinDispositionRecord {
            reference_error_id: ReferenceErrorId::new("ref-err-2").expect("reference id"),
            disposition: DetectorReferenceMatchDisposition::AcceptedOverlap,
            primary_detector_proposal_id: Some(
                DetectorProposalId::new("det-prop-001").expect("proposal id"),
            ),
        },
    );
    join.assessment.reference_record_count += 1;
    assert_primary_topology_violation(&join, PrimaryTopologyViolation::PrimaryTopologySetMismatch);
}

#[test]
fn fc02_two_primary_detectors_for_one_reference_fails() {
    let mut join = derive_assisted_overlap_accepted_join();
    let template = primary_edge(&join).clone();
    join.edges.push(DetectorReferenceJoinEdge {
        join_record_id: JoinRecordId::new("join-edge-second-det").expect("join record id"),
        detector_proposal_id: DetectorProposalId::new("det-prop-002").expect("proposal id"),
        ..template
    });
    join.detector_dispositions.push(
        vox_proof::detector_reference_join::DetectorJoinDispositionRecord {
            detector_proposal_id: DetectorProposalId::new("det-prop-002").expect("proposal id"),
            disposition: DetectorReferenceMatchDisposition::AcceptedOverlap,
            primary_reference_error_id: Some(
                ReferenceErrorId::new("ref-err-1").expect("reference id"),
            ),
        },
    );
    join.assessment.detector_proposal_count += 1;
    assert_primary_topology_violation(&join, PrimaryTopologyViolation::PrimaryTopologySetMismatch);
}

#[test]
fn fc02_primary_disposition_without_primary_id_fails() {
    let mut join = derive_assisted_overlap_accepted_join();
    join.detector_dispositions[0].primary_reference_error_id = None;
    assert_primary_topology_violation(
        &join,
        PrimaryTopologyViolation::DispositionRequiresPrimaryId,
    );
}

#[test]
fn fc02_non_primary_disposition_carrying_primary_id_fails() {
    let mut join = derive_assisted_overlap_accepted_join();
    join.detector_dispositions[0].disposition = DetectorReferenceMatchDisposition::OverlapCandidate;
    assert_primary_topology_violation(&join, PrimaryTopologyViolation::DispositionForbidsPrimaryId);
}

#[test]
fn fc02_overlap_primary_without_adjudication_fails() {
    let mut join = derive_assisted_overlap_accepted_join();
    let edge = join
        .edges
        .iter_mut()
        .find(|edge| edge.resolution == JoinEdgeResolution::PrimaryAssignment)
        .expect("primary edge");
    edge.adjudication_id = None;
    edge.adjudication_result = None;
    assert_primary_topology_violation(
        &join,
        PrimaryTopologyViolation::OverlapPrimaryMissingAdjudication,
    );
}

#[test]
fn fc02_overlap_primary_with_inconsistent_adjudication_result_fails() {
    let mut join = derive_assisted_overlap_wrong_correction_join();
    let edge = join
        .edges
        .iter_mut()
        .find(|edge| edge.resolution == JoinEdgeResolution::PrimaryAssignment)
        .expect("primary edge");
    edge.adjudication_result = Some(OverlapAdjudicationResult::SameErrorSameCorrection);
    assert_primary_topology_violation(
        &join,
        PrimaryTopologyViolation::OverlapPrimaryAdjudicationInconsistent,
    );
}

#[test]
fn fc02_valid_exact_topology_passes_local_validation() {
    let join = derive_assisted_exact_join();
    join.validate().expect("exact topology valid");
}

#[test]
fn fc02_valid_accepted_overlap_topology_passes_local_validation() {
    let join = derive_assisted_overlap_accepted_join();
    join.validate().expect("accepted overlap topology valid");
}

#[test]
fn fc02_valid_overlap_wrong_correction_topology_passes_local_validation() {
    let join = derive_assisted_overlap_wrong_correction_join();
    join.validate()
        .expect("overlap wrong correction topology valid");
}

// --- VP-V02-JOIN-PARTIAL-ADJUDICATION-COMPONENT-CORRECTION-01 regressions ---

fn shared_detector_overlap_stack(
    adjudications: Vec<OverlapAdjudicationRecord>,
) -> DetectorReferenceJoin {
    let (envelope, seal, mut coverage, _human_reference, bundle, context) = assisted_review_stack();
    coverage.records = vec![record(1, ReferenceCueDisposition::TranscriptionError)];
    coverage.expected_universe = universe(&[1]);
    coverage.assessment =
        ReferenceCoverage::derive_assessment(&coverage.expected_universe, &coverage.records)
            .expect("derive assessment");
    let human_reference = HumanFinalReference {
        schema_revision: HUMAN_FINAL_REFERENCE_SCHEMA.to_string(),
        run_id: coverage.run_id.clone(),
        input_identity: coverage.input_identity.clone(),
        seal_id: coverage.seal_id.clone(),
        reference_revision: coverage.reference_revision.clone(),
        records: vec![
            reference_error_record("ref-err-1", 1, 0, 0, 6, "wrong"),
            reference_error_record("ref-err-2", 1, 0, 4, 8, "wrong"),
        ],
        state: HumanFinalReferenceState::Sealed,
        assessment: HumanFinalReference::derive_assessment(
            &coverage.reference_revision,
            &coverage.input_identity,
            &[
                reference_error_record("ref-err-1", 1, 0, 0, 6, "wrong"),
                reference_error_record("ref-err-2", 1, 0, 4, 8, "wrong"),
            ],
        )
        .expect("derive assessment"),
    };
    coverage.assessment.total_eligible_transcription_errors = human_reference
        .assessment
        .recall_eligible_transcription_error_count;
    let snapshot = build_snapshot(
        vec![glossary_proposal(
            "det-prop-d1",
            1,
            0,
            2,
            6,
            "wrng",
            "wrong",
        )],
        DetectorProposalSnapshotState::Frozen,
    );
    let adjudication = frozen_adjudication_set(adjudications);
    DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive shared-detector join")
}

fn shared_reference_overlap_stack(
    adjudications: Vec<OverlapAdjudicationRecord>,
) -> DetectorReferenceJoin {
    let (envelope, seal, mut coverage, _human_reference, bundle, context) = assisted_review_stack();
    coverage.records = vec![record(1, ReferenceCueDisposition::TranscriptionError)];
    coverage.expected_universe = universe(&[1]);
    coverage.assessment =
        ReferenceCoverage::derive_assessment(&coverage.expected_universe, &coverage.records)
            .expect("derive assessment");
    let human_reference = HumanFinalReference {
        schema_revision: HUMAN_FINAL_REFERENCE_SCHEMA.to_string(),
        run_id: coverage.run_id.clone(),
        input_identity: coverage.input_identity.clone(),
        seal_id: coverage.seal_id.clone(),
        reference_revision: coverage.reference_revision.clone(),
        records: vec![reference_error_record("ref-err-1", 1, 0, 2, 8, "wrong")],
        state: HumanFinalReferenceState::Sealed,
        assessment: HumanFinalReference::derive_assessment(
            &coverage.reference_revision,
            &coverage.input_identity,
            &[reference_error_record("ref-err-1", 1, 0, 2, 8, "wrong")],
        )
        .expect("derive assessment"),
    };
    coverage.assessment.total_eligible_transcription_errors = human_reference
        .assessment
        .recall_eligible_transcription_error_count;
    let snapshot = build_snapshot(
        vec![
            glossary_proposal("det-prop-d1", 1, 0, 0, 4, "wrng", "wrong"),
            glossary_proposal("det-prop-d2", 1, 0, 4, 8, "wrng", "wrong"),
        ],
        DetectorProposalSnapshotState::Frozen,
    );
    let adjudication = frozen_adjudication_set(adjudications);
    DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive shared-reference join")
}

fn overlap_edges_for_pair<'a>(
    join: &'a DetectorReferenceJoin,
    detector: &str,
    reference: &str,
) -> Vec<&'a DetectorReferenceJoinEdge> {
    join.edges
        .iter()
        .filter(|edge| {
            edge.anchor_relation == JoinAnchorRelation::Overlap
                && edge.detector_proposal_id.as_str() == detector
                && edge.reference_error_id.as_str() == reference
        })
        .collect()
}

#[test]
fn partial_shared_detector_one_positive_one_missing_materializes_both_edges() {
    let join = shared_detector_overlap_stack(vec![adjudication_record(
        "adj-001",
        "det-prop-d1",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);
    assert_eq!(
        overlap_edges_for_pair(&join, "det-prop-d1", "ref-err-1").len(),
        1
    );
    assert_eq!(
        overlap_edges_for_pair(&join, "det-prop-d1", "ref-err-2").len(),
        1
    );
    assert_eq!(
        overlap_edges_for_pair(&join, "det-prop-d1", "ref-err-1")[0].resolution,
        JoinEdgeResolution::Ambiguous
    );
    assert!(
        overlap_edges_for_pair(&join, "det-prop-d1", "ref-err-1")[0]
            .adjudication_id
            .is_some()
    );
    assert_eq!(
        overlap_edges_for_pair(&join, "det-prop-d1", "ref-err-2")[0].resolution,
        JoinEdgeResolution::OverlapCandidate
    );
    assert_eq!(join.state, DetectorReferenceJoinState::RequiresAdjudication);
    assert_eq!(join.assessment.unresolved_overlap_edge_count, 1);
    assert_eq!(join.assessment.accepted_overlap_count, 0);
    assert!(
        join.detector_dispositions[0]
            .primary_reference_error_id
            .is_none()
    );
}

#[test]
fn partial_shared_detector_positive_and_different_error_resolves_primary() {
    let join = shared_detector_overlap_stack(vec![
        adjudication_record(
            "adj-001",
            "det-prop-d1",
            "ref-err-1",
            OverlapAdjudicationResult::SameErrorSameCorrection,
        ),
        adjudication_record(
            "adj-002",
            "det-prop-d1",
            "ref-err-2",
            OverlapAdjudicationResult::DifferentError,
        ),
    ]);
    assert_eq!(
        overlap_edges_for_pair(&join, "det-prop-d1", "ref-err-1")[0].resolution,
        JoinEdgeResolution::PrimaryAssignment
    );
    assert_eq!(
        overlap_edges_for_pair(&join, "det-prop-d1", "ref-err-2")[0].resolution,
        JoinEdgeResolution::RejectedDifferentError
    );
    assert_eq!(join.state, DetectorReferenceJoinState::Resolved);
    assert_eq!(
        join.detector_dispositions[0]
            .primary_reference_error_id
            .as_ref()
            .unwrap()
            .as_str(),
        "ref-err-1"
    );
}

#[test]
fn partial_shared_detector_two_positive_records_become_ambiguous() {
    let join = shared_detector_overlap_stack(vec![
        adjudication_record(
            "adj-001",
            "det-prop-d1",
            "ref-err-1",
            OverlapAdjudicationResult::SameErrorSameCorrection,
        ),
        adjudication_record(
            "adj-002",
            "det-prop-d1",
            "ref-err-2",
            OverlapAdjudicationResult::SameErrorSameCorrection,
        ),
    ]);
    assert_eq!(
        overlap_edges_for_pair(&join, "det-prop-d1", "ref-err-1")[0].resolution,
        JoinEdgeResolution::Ambiguous
    );
    assert_eq!(
        overlap_edges_for_pair(&join, "det-prop-d1", "ref-err-2")[0].resolution,
        JoinEdgeResolution::Ambiguous
    );
    assert!(
        join.detector_dispositions[0]
            .primary_reference_error_id
            .is_none()
    );
}

#[test]
fn partial_shared_detector_positive_and_ambiguous_become_ambiguous() {
    let join = shared_detector_overlap_stack(vec![
        adjudication_record(
            "adj-001",
            "det-prop-d1",
            "ref-err-1",
            OverlapAdjudicationResult::SameErrorSameCorrection,
        ),
        adjudication_record(
            "adj-002",
            "det-prop-d1",
            "ref-err-2",
            OverlapAdjudicationResult::Ambiguous,
        ),
    ]);
    assert_eq!(
        overlap_edges_for_pair(&join, "det-prop-d1", "ref-err-1")[0].resolution,
        JoinEdgeResolution::Ambiguous
    );
    assert!(
        join.detector_dispositions[0]
            .primary_reference_error_id
            .is_none()
    );
}

#[test]
fn partial_shared_reference_one_positive_one_missing_materializes_both_edges() {
    let join = shared_reference_overlap_stack(vec![adjudication_record(
        "adj-001",
        "det-prop-d1",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);
    assert_eq!(
        overlap_edges_for_pair(&join, "det-prop-d1", "ref-err-1").len(),
        1
    );
    assert_eq!(
        overlap_edges_for_pair(&join, "det-prop-d2", "ref-err-1").len(),
        1
    );
    assert_eq!(
        overlap_edges_for_pair(&join, "det-prop-d2", "ref-err-1")[0].resolution,
        JoinEdgeResolution::OverlapCandidate
    );
    assert_eq!(join.state, DetectorReferenceJoinState::RequiresAdjudication);
    assert!(
        join.reference_dispositions[0]
            .primary_detector_proposal_id
            .is_none()
    );
}

#[test]
fn partial_shared_reference_positive_and_different_error_selects_primary() {
    let join = shared_reference_overlap_stack(vec![
        adjudication_record(
            "adj-001",
            "det-prop-d1",
            "ref-err-1",
            OverlapAdjudicationResult::SameErrorSameCorrection,
        ),
        adjudication_record(
            "adj-002",
            "det-prop-d2",
            "ref-err-1",
            OverlapAdjudicationResult::DifferentError,
        ),
    ]);
    assert_eq!(
        overlap_edges_for_pair(&join, "det-prop-d1", "ref-err-1")[0].resolution,
        JoinEdgeResolution::PrimaryAssignment
    );
    assert_eq!(
        join.reference_dispositions[0]
            .primary_detector_proposal_id
            .as_ref()
            .unwrap()
            .as_str(),
        "det-prop-d1"
    );
    assert_eq!(
        join.detector_dispositions
            .iter()
            .find(|record| record.detector_proposal_id.as_str() == "det-prop-d2")
            .expect("second detector")
            .disposition,
        DetectorReferenceMatchDisposition::UnmatchedDetector
    );
}

#[test]
fn partial_shared_reference_two_positive_detectors_select_lowest_id() {
    let join = shared_reference_overlap_stack(vec![
        adjudication_record(
            "adj-001",
            "det-prop-d1",
            "ref-err-1",
            OverlapAdjudicationResult::SameErrorSameCorrection,
        ),
        adjudication_record(
            "adj-002",
            "det-prop-d2",
            "ref-err-1",
            OverlapAdjudicationResult::SameErrorSameCorrection,
        ),
    ]);
    assert_eq!(
        join.reference_dispositions[0]
            .primary_detector_proposal_id
            .as_ref()
            .unwrap()
            .as_str(),
        "det-prop-d1"
    );
    assert_eq!(
        join.detector_dispositions
            .iter()
            .find(|record| record.detector_proposal_id.as_str() == "det-prop-d2")
            .expect("duplicate detector")
            .disposition,
        DetectorReferenceMatchDisposition::DuplicateProposal
    );
}

#[test]
fn partial_shared_reference_shuffled_adjudication_order_is_identical() {
    let forward = shared_reference_overlap_stack(vec![
        adjudication_record(
            "adj-001",
            "det-prop-d1",
            "ref-err-1",
            OverlapAdjudicationResult::SameErrorSameCorrection,
        ),
        adjudication_record(
            "adj-002",
            "det-prop-d2",
            "ref-err-1",
            OverlapAdjudicationResult::DifferentError,
        ),
    ]);
    let reverse = shared_reference_overlap_stack(vec![
        adjudication_record(
            "adj-002",
            "det-prop-d2",
            "ref-err-1",
            OverlapAdjudicationResult::DifferentError,
        ),
        adjudication_record(
            "adj-001",
            "det-prop-d1",
            "ref-err-1",
            OverlapAdjudicationResult::SameErrorSameCorrection,
        ),
    ]);
    assert_eq!(forward.edges, reverse.edges);
    assert_eq!(forward.detector_dispositions, reverse.detector_dispositions);
    assert_eq!(
        forward.reference_dispositions,
        reverse.reference_dispositions
    );
}

#[test]
fn partial_independent_resolved_and_unresolved_components_coexist() {
    let join = fc01_partial_adjudication_materializes_consumed_records_only_join();
    assert_eq!(join.state, DetectorReferenceJoinState::RequiresAdjudication);
    assert_eq!(join.assessment.accepted_overlap_count, 1);
    assert_eq!(join.assessment.unresolved_overlap_edge_count, 1);
    assert_eq!(
        join.detector_dispositions
            .iter()
            .find(|record| record.detector_proposal_id.as_str() == "det-prop-a")
            .expect("resolved detector")
            .disposition,
        DetectorReferenceMatchDisposition::AcceptedOverlap
    );
    assert_eq!(
        join.detector_dispositions
            .iter()
            .find(|record| record.detector_proposal_id.as_str() == "det-prop-b")
            .expect("unresolved detector")
            .disposition,
        DetectorReferenceMatchDisposition::OverlapCandidate
    );
}

fn fc01_partial_adjudication_materializes_consumed_records_only_join() -> DetectorReferenceJoin {
    let (envelope, seal, mut coverage, _human_reference, bundle, context) = assisted_review_stack();
    coverage.records = vec![record(1, ReferenceCueDisposition::TranscriptionError)];
    coverage.expected_universe = universe(&[1]);
    coverage.assessment =
        ReferenceCoverage::derive_assessment(&coverage.expected_universe, &coverage.records)
            .expect("derive assessment");
    let human_reference = HumanFinalReference {
        schema_revision: HUMAN_FINAL_REFERENCE_SCHEMA.to_string(),
        run_id: coverage.run_id.clone(),
        input_identity: coverage.input_identity.clone(),
        seal_id: coverage.seal_id.clone(),
        reference_revision: coverage.reference_revision.clone(),
        records: vec![
            reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong"),
            reference_error_record("ref-err-2", 1, 0, 8, 12, "wrong"),
        ],
        state: HumanFinalReferenceState::Sealed,
        assessment: HumanFinalReference::derive_assessment(
            &coverage.reference_revision,
            &coverage.input_identity,
            &[
                reference_error_record("ref-err-1", 1, 0, 0, 4, "wrong"),
                reference_error_record("ref-err-2", 1, 0, 8, 12, "wrong"),
            ],
        )
        .expect("derive assessment"),
    };
    coverage.assessment.total_eligible_transcription_errors = human_reference
        .assessment
        .recall_eligible_transcription_error_count;
    let snapshot = build_snapshot(
        vec![
            glossary_proposal("det-prop-a", 1, 0, 2, 6, "wrng", "wrong"),
            glossary_proposal("det-prop-b", 1, 0, 10, 14, "wrng", "wrong"),
        ],
        DetectorProposalSnapshotState::Frozen,
    );
    let adjudication = frozen_adjudication_set(vec![adjudication_record(
        "adj-001",
        "det-prop-a",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);
    DetectorReferenceJoin::derive(
        &context,
        &envelope,
        &seal,
        &coverage,
        &human_reference,
        &snapshot,
        &bundle,
        &adjudication,
    )
    .expect("derive partial adjudication join")
}

#[test]
fn partial_exact_assigned_units_remain_excluded_from_phase3() {
    let (envelope, seal, coverage, human_reference, bundle, context) = assisted_review_stack();
    let snapshot = build_snapshot(
        vec![glossary_proposal(
            "det-prop-exact",
            1,
            0,
            0,
            4,
            "wrng",
            "wrong",
        )],
        DetectorProposalSnapshotState::Frozen,
    );
    let adjudication = frozen_adjudication_set(vec![adjudication_record(
        "adj-001",
        "det-prop-exact",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);
    assert!(matches!(
        DetectorReferenceJoin::derive(
            &context,
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &bundle,
            &adjudication,
        ),
        Err(
            DetectorReferenceJoinError::AdjudicationPairNotAdmissiblePhase3Edge {
                reason: Phase3AdjudicationRejectionReason::DetectorAssignedByExactPhase,
                ..
            }
        )
    ));
}

#[test]
fn partial_stale_adjudication_on_exact_assigned_units_fails_closed() {
    let (envelope, seal, coverage, human_reference, bundle, context) = assisted_review_stack();
    let snapshot = build_snapshot(
        vec![
            glossary_proposal("det-prop-overlap", 1, 0, 2, 6, "wrng", "wrong"),
            glossary_proposal("det-prop-exact", 1, 0, 0, 4, "wrng", "wrong"),
        ],
        DetectorProposalSnapshotState::Frozen,
    );
    let adjudication = frozen_adjudication_set(vec![adjudication_record(
        "adj-stale",
        "det-prop-overlap",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);
    assert!(matches!(
        DetectorReferenceJoin::derive(
            &context,
            &envelope,
            &seal,
            &coverage,
            &human_reference,
            &snapshot,
            &bundle,
            &adjudication,
        ),
        Err(
            DetectorReferenceJoinError::AdjudicationPairNotAdmissiblePhase3Edge {
                reason: Phase3AdjudicationRejectionReason::ReferenceAssignedByExactPhase,
                ..
            }
        )
    ));
}

#[test]
fn partial_every_admissible_pair_materializes_exactly_once() {
    let join = shared_detector_overlap_stack(vec![adjudication_record(
        "adj-001",
        "det-prop-d1",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);
    let overlap_edges: Vec<_> = join
        .edges
        .iter()
        .filter(|edge| edge.anchor_relation == JoinAnchorRelation::Overlap)
        .collect();
    assert_eq!(overlap_edges.len(), 2);
    let pairs: HashSet<_> = overlap_edges
        .iter()
        .map(|edge| {
            (
                edge.detector_proposal_id.as_str(),
                edge.reference_error_id.as_str(),
            )
        })
        .collect();
    assert_eq!(
        pairs,
        HashSet::from([("det-prop-d1", "ref-err-1"), ("det-prop-d1", "ref-err-2")])
    );
}

#[test]
fn partial_positive_on_one_edge_does_not_remove_other_admissible_pair() {
    let join = shared_detector_overlap_stack(vec![adjudication_record(
        "adj-001",
        "det-prop-d1",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);
    assert!(
        join.edges.iter().any(|edge| {
            edge.anchor_relation == JoinAnchorRelation::Overlap
                && edge.detector_proposal_id.as_str() == "det-prop-d1"
                && edge.reference_error_id.as_str() == "ref-err-2"
                && edge.resolution == JoinEdgeResolution::OverlapCandidate
        }),
        "sibling admissible pair must remain materialized after partial positive adjudication"
    );
}

#[test]
fn partial_no_duplicate_phase3_edge_pairs() {
    let join = shared_reference_overlap_stack(vec![
        adjudication_record(
            "adj-001",
            "det-prop-d1",
            "ref-err-1",
            OverlapAdjudicationResult::SameErrorSameCorrection,
        ),
        adjudication_record(
            "adj-002",
            "det-prop-d2",
            "ref-err-1",
            OverlapAdjudicationResult::DifferentError,
        ),
    ]);
    let mut pair_counts: HashMap<(&str, &str), u32> = HashMap::new();
    for edge in join
        .edges
        .iter()
        .filter(|edge| edge.anchor_relation == JoinAnchorRelation::Overlap)
    {
        *pair_counts
            .entry((
                edge.detector_proposal_id.as_str(),
                edge.reference_error_id.as_str(),
            ))
            .or_insert(0) += 1;
    }
    assert!(pair_counts.values().all(|count| *count == 1));
}

#[test]
fn partial_local_validation_primary_sharing_detector_with_unresolved_fails() {
    let mut join = shared_detector_overlap_stack(vec![adjudication_record(
        "adj-001",
        "det-prop-d1",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);
    if let Some(edge) = join.edges.iter_mut().find(|edge| {
        edge.detector_proposal_id.as_str() == "det-prop-d1"
            && edge.reference_error_id.as_str() == "ref-err-1"
    }) {
        edge.resolution = JoinEdgeResolution::PrimaryAssignment;
    }
    assert_primary_topology_violation(
        &join,
        PrimaryTopologyViolation::PrimaryAssignmentCoexistsWithUnresolvedOverlap,
    );
}

#[test]
fn partial_local_validation_primary_sharing_reference_with_unresolved_fails() {
    let mut join = shared_reference_overlap_stack(vec![adjudication_record(
        "adj-001",
        "det-prop-d1",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);
    if let Some(edge) = join.edges.iter_mut().find(|edge| {
        edge.detector_proposal_id.as_str() == "det-prop-d1"
            && edge.reference_error_id.as_str() == "ref-err-1"
    }) {
        edge.resolution = JoinEdgeResolution::PrimaryAssignment;
    }
    assert_primary_topology_violation(
        &join,
        PrimaryTopologyViolation::PrimaryAssignmentCoexistsWithUnresolvedOverlap,
    );
}

#[test]
fn partial_local_validation_overlap_candidate_with_adjudication_metadata_fails() {
    let mut join = shared_detector_overlap_stack(vec![]);
    let edge = join
        .edges
        .iter_mut()
        .find(|edge| edge.reference_error_id.as_str() == "ref-err-2")
        .expect("candidate edge");
    edge.adjudication_id = Some(OverlapAdjudicationId::new("adj-bad").expect("adjudication id"));
    edge.adjudication_result = Some(OverlapAdjudicationResult::SameErrorSameCorrection);
    assert_primary_topology_violation(
        &join,
        PrimaryTopologyViolation::OverlapCandidateCarriesAdjudicationMetadata,
    );
}

#[test]
fn partial_local_validation_adjudicated_overlap_missing_metadata_fails() {
    let mut join = shared_detector_overlap_stack(vec![
        adjudication_record(
            "adj-001",
            "det-prop-d1",
            "ref-err-1",
            OverlapAdjudicationResult::SameErrorSameCorrection,
        ),
        adjudication_record(
            "adj-002",
            "det-prop-d1",
            "ref-err-2",
            OverlapAdjudicationResult::DifferentError,
        ),
    ]);
    let edge = join
        .edges
        .iter_mut()
        .find(|edge| edge.resolution == JoinEdgeResolution::RejectedDifferentError)
        .expect("rejected different-error edge");
    edge.adjudication_result = None;
    assert_primary_topology_violation(
        &join,
        PrimaryTopologyViolation::AdjudicatedOverlapEdgeMissingMetadata,
    );
}

#[test]
fn partial_local_validation_valid_unresolved_component_passes() {
    let join = shared_detector_overlap_stack(vec![adjudication_record(
        "adj-001",
        "det-prop-d1",
        "ref-err-1",
        OverlapAdjudicationResult::SameErrorSameCorrection,
    )]);
    join.validate()
        .expect("valid partially adjudicated unresolved component");
}

#[test]
fn partial_local_validation_valid_fully_resolved_component_passes() {
    let join = shared_detector_overlap_stack(vec![
        adjudication_record(
            "adj-001",
            "det-prop-d1",
            "ref-err-1",
            OverlapAdjudicationResult::SameErrorSameCorrection,
        ),
        adjudication_record(
            "adj-002",
            "det-prop-d1",
            "ref-err-2",
            OverlapAdjudicationResult::DifferentError,
        ),
    ]);
    join.validate()
        .expect("valid fully resolved shared-detector component");
}

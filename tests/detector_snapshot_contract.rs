use vox_proof::artifact_bundle::{
    ARTIFACT_BUNDLE_SCHEMA, ArtifactBindingContext, ArtifactBundle, ArtifactBundleId,
    ArtifactBundleState, ArtifactContentDigest, ArtifactDescriptor, ArtifactId,
    ArtifactSchemaIdentity,
};
use vox_proof::candidate::DetectionKind;
use vox_proof::detector_snapshot::{
    DETECTOR_PROPOSAL_SNAPSHOT_SCHEMA, DetectorAnalysisIdentity,
    DetectorAnalysisIdentityValidationError, DetectorAsciiLatinPhoneticRepresentation,
    DetectorComponentIdentity, DetectorPhoneticComparisonFacts, DetectorPhoneticTargetKind,
    DetectorProposalAlternative, DetectorProposalAlternativeValidationError,
    DetectorProposalEvidence, DetectorProposalEvidenceValidationError, DetectorProposalId,
    DetectorProposalRecord, DetectorProposalRecordValidationError, DetectorProposalSemanticKey,
    DetectorProposalSnapshot, DetectorProposalSnapshotAssessment, DetectorProposalSnapshotState,
    DetectorProposalSnapshotValidationError, DetectorProposalSourceAnchor,
    DetectorSessionTermEntry, DetectorSnapshotRevisionId,
};
use vox_proof::reference_coverage::CueReferenceId;
use vox_proof::run_manifest::{
    ArtifactRole, CalibrationValidityMode, InputClass, InputIdentityReference,
    InputIdentityValidationError, RUN_ENVELOPE_SCHEMA, RunEnvelope, RunId, RunLifecycleState,
    WorkflowObservationMode,
};

const SAMPLE_REVISION: &str =
    "rev:sha256-v1:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const SAMPLE_SESSION_TERMS: &str =
    "session-terms:sha256-v1:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const SAMPLE_DIGEST: &str =
    "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

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
            detector_component("ascii-latin-phonetic-similarity", "0.1.0"),
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

fn session_term_entry() -> DetectorSessionTermEntry {
    DetectorSessionTermEntry {
        canonical_term: "widget".to_string(),
        aliases: vec!["wijet".to_string()],
        observed_error_forms: vec!["widgit".to_string()],
    }
}

fn phonetic_representation(
    letters: &str,
    primary: &str,
    alternate: &str,
) -> DetectorAsciiLatinPhoneticRepresentation {
    DetectorAsciiLatinPhoneticRepresentation {
        normalized_letters: letters.to_string(),
        primary_key: primary.to_string(),
        alternate_key: alternate.to_string(),
    }
}

fn phonetic_comparison(numerator: u32, denominator: u32) -> DetectorPhoneticComparisonFacts {
    let ratio_permille = numerator as u64 * 1000 / denominator as u64;
    DetectorPhoneticComparisonFacts {
        edit_distance: 1,
        ratio_numerator: numerator,
        ratio_denominator: denominator,
        ratio_permille: ratio_permille as u32,
        matched_key: "primary-key".to_string(),
    }
}

#[allow(clippy::too_many_arguments)]
fn glossary_proposal(
    proposal_id: &str,
    cue_id: u32,
    segment_position: u32,
    start: u32,
    end: u32,
    observed: &str,
    matched_form: &str,
    detector: &DetectorComponentIdentity,
    alternatives: Vec<DetectorProposalAlternative>,
) -> DetectorProposalRecord {
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
            entry: session_term_entry(),
            matched_form: matched_form.to_string(),
        },
        alternatives,
    };
    record.semantic_key = record.derive_semantic_key();
    record
}

#[allow(clippy::too_many_arguments)]
fn observed_error_proposal(
    proposal_id: &str,
    cue_id: u32,
    segment_position: u32,
    start: u32,
    end: u32,
    observed: &str,
    matched_form: &str,
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
            entry: session_term_entry(),
            matched_form: matched_form.to_string(),
        },
        alternatives: vec![],
    };
    record.semantic_key = record.derive_semantic_key();
    record
}

#[allow(clippy::too_many_arguments)]
fn phonetic_proposal(
    proposal_id: &str,
    cue_id: u32,
    segment_position: u32,
    start: u32,
    end: u32,
    observed: &str,
    target: &str,
) -> DetectorProposalRecord {
    let detector = detector_component("ascii-latin-phonetic-similarity", "0.1.0");
    let analysis = analysis_identity();
    let anchor = source_anchor(cue_id, segment_position, start, end);
    let mut record = DetectorProposalRecord {
        detector_proposal_id: DetectorProposalId::new(proposal_id).expect("proposal id"),
        snapshot_revision: snapshot_revision_id(),
        input_identity: input_identity(),
        semantic_key: DetectorProposalSemanticKey {
            detector_id: detector.id.clone(),
            detection_kind: DetectionKind::PhoneticSimilarity,
            source_anchor: anchor.clone(),
        },
        detector: detector.clone(),
        source_anchor: anchor,
        observed_surface: observed.to_string(),
        detection_kind: DetectionKind::PhoneticSimilarity,
        evidence: DetectorProposalEvidence::PhoneticSimilarity {
            observed_surface: observed.to_string(),
            target_surface: target.to_string(),
            target_kind: DetectorPhoneticTargetKind::CanonicalTerm,
            canonical_term: "widget".to_string(),
            source_representation: phonetic_representation("widgit", "W-J-T", "W-D-J-T"),
            target_representation: phonetic_representation("widget", "W-J-T", "W-D-G-T"),
            comparison: phonetic_comparison(850, 1000),
            detector_config: analysis.detector_config.clone(),
            algorithm: analysis.algorithm.clone(),
        },
        alternatives: vec![],
    };
    record.semantic_key = record.derive_semantic_key();
    record
}

fn build_snapshot(
    proposals: Vec<DetectorProposalRecord>,
    state: DetectorProposalSnapshotState,
    frozen_at_unix_ms: u64,
    calibration_validity: CalibrationValidityMode,
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
        run_id: RunId::new("run-detector").expect("run id"),
        input_identity: input_identity(),
        calibration_validity,
        snapshot_revision: snapshot_revision_id(),
        detector_output_artifact_id: detector_output_artifact_id(),
        analysis_identity: analysis_identity(),
        proposals,
        frozen_at_unix_ms,
        state,
        assessment,
    }
}

fn detector_execution_envelope(mode: CalibrationValidityMode) -> RunEnvelope {
    RunEnvelope {
        schema_revision: RUN_ENVELOPE_SCHEMA.to_string(),
        run_id: RunId::new("run-detector").expect("run id"),
        input_identity: input_identity(),
        calibration_validity: mode,
        workflow_observation: WorkflowObservationMode::Disabled,
        input_class: if mode == CalibrationValidityMode::BlindReference {
            InputClass::SelfOwnedReal
        } else {
            InputClass::SyntheticProtocolFixture
        },
        qualifies_as_real_material_evidence: false,
        lifecycle_state: RunLifecycleState::DetectorExecution,
        expected_artifact_roles: vec![ArtifactRole::DetectorOutput],
    }
}

fn lifecycle_envelope(
    mode: CalibrationValidityMode,
    lifecycle_state: RunLifecycleState,
) -> RunEnvelope {
    let mut envelope = detector_execution_envelope(mode);
    envelope.lifecycle_state = lifecycle_state;
    envelope
}

fn binding_context(mode: CalibrationValidityMode) -> ArtifactBindingContext {
    ArtifactBindingContext {
        run_id: RunId::new("run-detector").expect("run id"),
        input_identity: input_identity(),
        calibration_validity: mode,
        reference_seal_id: None,
        reference_coverage_id: None,
        reference_revision: None,
    }
}

fn schema_identity() -> ArtifactSchemaIdentity {
    ArtifactSchemaIdentity::new("voxproof-detector-output-v1", "v1").expect("schema")
}

fn detector_descriptor(context: &ArtifactBindingContext, artifact_id: &str) -> ArtifactDescriptor {
    ArtifactDescriptor {
        artifact_id: ArtifactId::new(artifact_id).expect("artifact id"),
        role: ArtifactRole::DetectorOutput,
        payload_schema: schema_identity(),
        content_digest: ArtifactContentDigest::new(SAMPLE_DIGEST).expect("digest"),
        byte_length: 128,
        binding_context: context.clone(),
    }
}

fn build_draft_bundle(
    mode: CalibrationValidityMode,
    artifacts: Vec<ArtifactDescriptor>,
) -> ArtifactBundle {
    let context = binding_context(mode);
    let expected_roles = vec![ArtifactRole::DetectorOutput];
    let assessment =
        ArtifactBundle::derive_assessment(&expected_roles, &artifacts, &context).expect("derive");

    ArtifactBundle {
        schema_revision: ARTIFACT_BUNDLE_SCHEMA.to_string(),
        bundle_id: ArtifactBundleId::new("bundle-detector").expect("bundle id"),
        binding_context: context,
        expected_roles,
        artifacts,
        bundle_state: ArtifactBundleState::Draft,
        assessment,
    }
}

fn build_bundle(
    mode: CalibrationValidityMode,
    artifacts: Vec<ArtifactDescriptor>,
) -> ArtifactBundle {
    let context = binding_context(mode);
    let expected_roles = vec![ArtifactRole::DetectorOutput];
    let assessment =
        ArtifactBundle::derive_assessment(&expected_roles, &artifacts, &context).expect("derive");

    ArtifactBundle {
        schema_revision: ARTIFACT_BUNDLE_SCHEMA.to_string(),
        bundle_id: ArtifactBundleId::new("bundle-detector").expect("bundle id"),
        binding_context: context,
        expected_roles,
        artifacts,
        bundle_state: ArtifactBundleState::Complete,
        assessment,
    }
}

// --- Serialization ---

#[test]
fn json_round_trip_retains_schema_and_enum_spellings() {
    let proposals = vec![
        glossary_proposal(
            "det-prop-001",
            1,
            0,
            0,
            5,
            "wijet",
            "wijet",
            &detector_component("glossary-alias-match", "0.1.0"),
            vec![],
        ),
        observed_error_proposal("det-prop-002", 2, 0, 0, 6, "widgit", "widgit"),
        phonetic_proposal("det-prop-003", 3, 0, 0, 6, "widgit", "widget"),
    ];
    let snapshot = build_snapshot(
        proposals,
        DetectorProposalSnapshotState::Draft,
        0,
        CalibrationValidityMode::BlindReference,
    );

    let json = serde_json::to_string_pretty(&snapshot).expect("serialize");
    assert!(json.contains(DETECTOR_PROPOSAL_SNAPSHOT_SCHEMA));
    assert!(json.contains("\"glossary_alias_match\""));
    assert!(json.contains("\"glossary_alias\""));
    assert!(json.contains("\"observed_error_form\""));
    assert!(json.contains("\"phonetic_similarity\""));
    assert!(json.contains("\"canonical_term\""));
    assert!(json.contains("\"draft\""));
    assert_eq!(
        serde_json::to_string(&DetectorProposalSnapshotState::Frozen).expect("serialize state"),
        "\"frozen\""
    );

    let restored: DetectorProposalSnapshot = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, snapshot);
    restored.validate().expect("valid snapshot");
}

#[test]
fn unknown_top_level_field_rejected() {
    let snapshot = build_snapshot(
        vec![],
        DetectorProposalSnapshotState::Draft,
        0,
        CalibrationValidityMode::BlindReference,
    );
    let mut value = serde_json::to_value(&snapshot).expect("value");
    value.as_object_mut().expect("object").insert(
        "review_decision".to_string(),
        serde_json::json!("forbidden"),
    );

    let error = serde_json::from_value::<DetectorProposalSnapshot>(value).expect_err("must fail");
    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn join_field_on_proposal_record_rejected() {
    let proposal = glossary_proposal(
        "det-prop-001",
        1,
        0,
        0,
        5,
        "wijet",
        "wijet",
        &detector_component("glossary-alias-match", "0.1.0"),
        vec![],
    );
    let mut value = serde_json::to_value(&proposal).expect("value");
    value
        .as_object_mut()
        .expect("object")
        .insert("match_disposition".to_string(), serde_json::json!("tp"));

    let error = serde_json::from_value::<DetectorProposalRecord>(value).expect_err("must fail");
    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn decision_field_on_session_term_entry_rejected() {
    let mut value = serde_json::to_value(session_term_entry()).expect("value");
    value
        .as_object_mut()
        .expect("object")
        .insert("review_decision".to_string(), serde_json::json!("accepted"));

    let error = serde_json::from_value::<DetectorSessionTermEntry>(value).expect_err("must fail");
    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn unsupported_schema_fails_validation() {
    let mut snapshot = build_snapshot(
        vec![],
        DetectorProposalSnapshotState::Draft,
        0,
        CalibrationValidityMode::BlindReference,
    );
    snapshot.schema_revision = "voxproof-detector-proposal-snapshot-v0".to_string();

    assert!(matches!(
        snapshot.validate(),
        Err(DetectorProposalSnapshotValidationError::UnsupportedSchemaRevision { .. })
    ));
}

// --- Identity ---

#[test]
fn valid_snapshot_and_proposal_ids_accepted() {
    assert!(DetectorSnapshotRevisionId::new("det-snap-rev-alpha").is_ok());
    assert!(DetectorProposalId::new("det-prop-alpha").is_ok());
}

#[test]
fn path_like_ids_rejected() {
    for value in [
        "/Users/example/private/snapshot.json",
        "C:\\Users\\example\\private\\snapshot.json",
    ] {
        assert!(DetectorSnapshotRevisionId::new(value).is_err());
        assert!(DetectorProposalId::new(value).is_err());
    }
}

#[test]
fn malformed_input_revision_rejected_in_analysis_identity() {
    let mut identity = analysis_identity();
    identity.input_identity = InputIdentityReference {
        transcript_revision_id: "not-a-revision".to_string(),
    };

    assert!(matches!(
        identity.validate(&input_identity()),
        Err(DetectorAnalysisIdentityValidationError::InputIdentityMismatch)
    ));
    assert!(matches!(
        identity.validate(&identity.input_identity.clone()),
        Err(
            DetectorAnalysisIdentityValidationError::InvalidInputIdentity(
                InputIdentityValidationError::InvalidTranscriptRevisionId(_)
            )
        )
    ));
}

#[test]
fn proposal_id_is_distinct_from_semantic_key() {
    let proposal = glossary_proposal(
        "det-prop-001",
        1,
        0,
        0,
        5,
        "wijet",
        "wijet",
        &detector_component("glossary-alias-match", "0.1.0"),
        vec![],
    );
    assert_ne!(
        proposal.detector_proposal_id.as_str(),
        proposal.semantic_key.detector_id
    );
    assert_ne!(proposal.detector_proposal_id.as_str(), SAMPLE_REVISION);
}

// --- Analysis identity ---

#[test]
fn session_terms_identity_format_accepted() {
    analysis_identity()
        .validate(&input_identity())
        .expect("valid session terms");
}

#[test]
fn session_terms_wrong_prefix_rejected() {
    let mut identity = analysis_identity();
    identity.session_terms_identity = format!(
        "terms:sha256-v1:{}",
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    );

    assert!(matches!(
        identity.validate(&input_identity()),
        Err(
            DetectorAnalysisIdentityValidationError::InvalidSessionTermsIdentity(
                vox_proof::detector_snapshot::SessionTermsIdentityError::MissingPrefix
            )
        )
    ));
}

#[test]
fn session_terms_wrong_length_rejected() {
    let mut identity = analysis_identity();
    identity.session_terms_identity = "session-terms:sha256-v1:0123".to_string();

    assert!(matches!(
        identity.validate(&input_identity()),
        Err(
            DetectorAnalysisIdentityValidationError::InvalidSessionTermsIdentity(
                vox_proof::detector_snapshot::SessionTermsIdentityError::InvalidLength
            )
        )
    ));
}

#[test]
fn session_terms_uppercase_hex_rejected() {
    let mut identity = analysis_identity();
    identity.session_terms_identity = format!(
        "session-terms:sha256-v1:{}",
        "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF"
    );

    assert!(matches!(
        identity.validate(&input_identity()),
        Err(
            DetectorAnalysisIdentityValidationError::InvalidSessionTermsIdentity(
                vox_proof::detector_snapshot::SessionTermsIdentityError::UppercaseHexNotCanonical
            )
        )
    ));
}

#[test]
fn duplicate_detector_in_set_rejected() {
    let mut identity = analysis_identity();
    identity.detector_set = vec![
        detector_component("glossary-alias-match", "0.1.0"),
        detector_component("glossary-alias-match", "2.0.0"),
    ];

    assert!(matches!(
        identity.validate(&input_identity()),
        Err(DetectorAnalysisIdentityValidationError::DuplicateDetectorInSet { .. })
    ));
}

#[test]
fn missing_detector_in_set_fails_record_validation() {
    let mut proposal = glossary_proposal(
        "det-prop-001",
        1,
        0,
        0,
        5,
        "wijet",
        "wijet",
        &detector_component("unknown-detector", "0.1.0"),
        vec![],
    );
    proposal.detector = detector_component("unknown-detector", "0.1.0");
    proposal.semantic_key.detector_id = "unknown-detector".to_string();
    proposal.semantic_key = proposal.derive_semantic_key();

    assert!(matches!(
        proposal.validate(
            &snapshot_revision_id(),
            &input_identity(),
            &analysis_identity()
        ),
        Err(DetectorProposalRecordValidationError::DetectorNotInAnalysisSet)
    ));
}

// --- Proposal semantics ---

#[test]
fn semantic_key_derivation_matches_record_fields() {
    let proposal = glossary_proposal(
        "det-prop-001",
        1,
        0,
        0,
        5,
        "wijet",
        "wijet",
        &detector_component("glossary-alias-match", "0.1.0"),
        vec![],
    );
    assert_eq!(proposal.semantic_key, proposal.derive_semantic_key());
}

#[test]
fn semantic_key_excludes_detector_version() {
    let first = glossary_proposal(
        "det-prop-001",
        1,
        0,
        0,
        5,
        "wijet",
        "wijet",
        &detector_component("glossary-alias-match", "0.1.0"),
        vec![],
    );
    let mut second = first.clone();
    second.detector.version = "9.9.9".to_string();
    assert_eq!(first.derive_semantic_key(), second.derive_semantic_key());
    assert!(!format!("{:?}", first.derive_semantic_key()).contains("9.9.9"));

    let mut wrong_key = first.clone();
    wrong_key.semantic_key = DetectorProposalSemanticKey {
        detector_id: "glossary-alias-match".to_string(),
        detection_kind: DetectionKind::GlossaryAliasMatch,
        source_anchor: {
            let mut anchor = first.source_anchor.clone();
            anchor.segment_position = 99;
            anchor
        },
    };
    assert!(matches!(
        wrong_key.validate(
            &snapshot_revision_id(),
            &input_identity(),
            &analysis_identity()
        ),
        Err(DetectorProposalRecordValidationError::SemanticKeyMismatch)
    ));
}

#[test]
fn duplicate_proposal_ids_fail_frozen_state() {
    let proposals = vec![
        glossary_proposal(
            "det-prop-dup",
            1,
            0,
            0,
            5,
            "wijet",
            "wijet",
            &detector_component("glossary-alias-match", "0.1.0"),
            vec![],
        ),
        glossary_proposal(
            "det-prop-dup",
            2,
            0,
            0,
            5,
            "wijet",
            "wijet",
            &detector_component("glossary-alias-match", "0.1.0"),
            vec![],
        ),
    ];
    let snapshot = build_snapshot(
        proposals,
        DetectorProposalSnapshotState::Frozen,
        1_700_000_000_000,
        CalibrationValidityMode::BlindReference,
    );

    assert!(matches!(
        snapshot.validate(),
        Err(DetectorProposalSnapshotValidationError::SnapshotStateMismatch { .. })
    ));
}

#[test]
fn duplicate_semantic_keys_fail_frozen_state() {
    let proposals = vec![
        glossary_proposal(
            "det-prop-001",
            1,
            0,
            0,
            5,
            "wijet",
            "wijet",
            &detector_component("glossary-alias-match", "0.1.0"),
            vec![],
        ),
        glossary_proposal(
            "det-prop-002",
            1,
            0,
            0,
            5,
            "wijet",
            "wijet",
            &detector_component("glossary-alias-match", "0.1.0"),
            vec![],
        ),
    ];
    let snapshot = build_snapshot(
        proposals,
        DetectorProposalSnapshotState::Frozen,
        1_700_000_000_000,
        CalibrationValidityMode::BlindReference,
    );

    assert!(matches!(
        snapshot.validate(),
        Err(DetectorProposalSnapshotValidationError::SnapshotStateMismatch { .. })
    ));
}

#[test]
fn same_anchor_different_detectors_allowed() {
    let proposals = vec![
        glossary_proposal(
            "det-prop-001",
            1,
            0,
            0,
            5,
            "wijet",
            "wijet",
            &detector_component("glossary-alias-match", "0.1.0"),
            vec![],
        ),
        phonetic_proposal("det-prop-002", 1, 0, 0, 5, "wijet", "widget"),
    ];
    let snapshot = build_snapshot(
        proposals,
        DetectorProposalSnapshotState::Frozen,
        1_700_000_000_000,
        CalibrationValidityMode::DetectorAssisted,
    );
    snapshot
        .validate()
        .expect("different detectors same anchor");
}

#[test]
fn same_anchor_glossary_and_observed_error_remain_distinct() {
    let proposals = vec![
        glossary_proposal(
            "det-prop-001",
            1,
            0,
            0,
            6,
            "widgit",
            "widgit",
            &detector_component("glossary-alias-match", "0.1.0"),
            vec![],
        ),
        observed_error_proposal("det-prop-002", 1, 0, 0, 6, "widgit", "widgit"),
    ];
    assert_ne!(proposals[0].semantic_key, proposals[1].semantic_key);
    let snapshot = build_snapshot(
        proposals,
        DetectorProposalSnapshotState::Frozen,
        1_700_000_000_000,
        CalibrationValidityMode::DetectorAssisted,
    );
    snapshot
        .validate()
        .expect("glossary and observed-error distinct at same anchor");
}

#[test]
fn overlapping_anchors_remain_representable() {
    let proposals = vec![
        glossary_proposal(
            "det-prop-001",
            1,
            0,
            0,
            5,
            "wijet",
            "wijet",
            &detector_component("glossary-alias-match", "0.1.0"),
            vec![],
        ),
        glossary_proposal(
            "det-prop-002",
            1,
            0,
            2,
            6,
            "jetx",
            "jetx",
            &detector_component("glossary-alias-match", "0.1.0"),
            vec![],
        ),
    ];
    let snapshot = build_snapshot(
        proposals,
        DetectorProposalSnapshotState::Draft,
        0,
        CalibrationValidityMode::BlindReference,
    );
    snapshot.validate().expect("overlapping anchors allowed");
}

#[test]
fn cue_id_and_segment_position_remain_separate() {
    let first = source_anchor(1, 0, 0, 4);
    let second = source_anchor(1, 1, 0, 4);
    assert_ne!(first, second);
}

#[test]
fn invalid_byte_range_rejected() {
    let inverted = source_anchor(1, 0, 8, 4);
    assert!(inverted.validate().is_err());

    let zero = source_anchor(1, 0, 0, 0);
    assert!(zero.validate().is_err());
}

#[test]
fn observed_surface_anchor_length_mismatch_rejected() {
    let proposal = glossary_proposal(
        "det-prop-001",
        1,
        0,
        0,
        6,
        "short",
        "short",
        &detector_component("glossary-alias-match", "0.1.0"),
        vec![],
    );

    assert!(matches!(
        proposal.validate(
            &snapshot_revision_id(),
            &input_identity(),
            &analysis_identity()
        ),
        Err(DetectorProposalRecordValidationError::ObservedSurfaceAnchorLengthMismatch)
    ));
}

#[test]
fn observed_surface_evidence_mismatch_rejected() {
    let mut proposal = glossary_proposal(
        "det-prop-001",
        1,
        0,
        0,
        5,
        "wijet",
        "wijet",
        &detector_component("glossary-alias-match", "0.1.0"),
        vec![],
    );
    proposal.evidence = DetectorProposalEvidence::GlossaryAlias {
        entry: session_term_entry(),
        matched_form: "other".to_string(),
    };

    assert!(matches!(
        proposal.validate(
            &snapshot_revision_id(),
            &input_identity(),
            &analysis_identity()
        ),
        Err(DetectorProposalRecordValidationError::EvidenceValidation(
            DetectorProposalEvidenceValidationError::ObservedSurfaceMismatch
        ))
    ));
}

// --- Evidence ---

#[test]
fn glossary_evidence_round_trip_validates() {
    let proposal = glossary_proposal(
        "det-prop-001",
        1,
        0,
        0,
        5,
        "wijet",
        "wijet",
        &detector_component("glossary-alias-match", "0.1.0"),
        vec![],
    );
    proposal
        .validate(
            &snapshot_revision_id(),
            &input_identity(),
            &analysis_identity(),
        )
        .expect("glossary evidence");
}

#[test]
fn observed_error_form_is_distinct_evidence_variant() {
    let proposal = observed_error_proposal("det-prop-001", 1, 0, 0, 6, "widgit", "widgit");
    assert!(matches!(
        proposal.evidence,
        DetectorProposalEvidence::ObservedErrorForm { .. }
    ));
    proposal
        .validate(
            &snapshot_revision_id(),
            &input_identity(),
            &analysis_identity(),
        )
        .expect("observed error form");
}

#[test]
fn observed_error_form_not_inferred_from_detection_kind() {
    let mut proposal = observed_error_proposal("det-prop-001", 1, 0, 0, 6, "widgit", "widgit");
    proposal.detection_kind = DetectionKind::PhoneticSimilarity;
    proposal.semantic_key = proposal.derive_semantic_key();

    assert!(matches!(
        proposal.validate(
            &snapshot_revision_id(),
            &input_identity(),
            &analysis_identity()
        ),
        Err(DetectorProposalRecordValidationError::EvidenceValidation(
            DetectorProposalEvidenceValidationError::IncompatibleDetectionKind {
                evidence: "observed_error_form",
                ..
            }
        ))
    ));
}

#[test]
fn phonetic_evidence_round_trip_validates() {
    let proposal = phonetic_proposal("det-prop-001", 1, 0, 0, 6, "widgit", "widget");
    proposal
        .validate(
            &snapshot_revision_id(),
            &input_identity(),
            &analysis_identity(),
        )
        .expect("phonetic evidence");
}

#[test]
fn phonetic_observed_surface_mismatch_rejected() {
    let mut proposal = phonetic_proposal("det-prop-001", 1, 0, 0, 6, "widgit", "widget");
    if let DetectorProposalEvidence::PhoneticSimilarity {
        observed_surface, ..
    } = &mut proposal.evidence
    {
        *observed_surface = "other".to_string();
    }

    assert!(matches!(
        proposal.validate(
            &snapshot_revision_id(),
            &input_identity(),
            &analysis_identity()
        ),
        Err(DetectorProposalRecordValidationError::EvidenceValidation(
            DetectorProposalEvidenceValidationError::ObservedSurfaceMismatch
        ))
    ));
}

#[test]
fn phonetic_zero_denominator_rejected() {
    let mut proposal = phonetic_proposal("det-prop-001", 1, 0, 0, 6, "widgit", "widget");
    if let DetectorProposalEvidence::PhoneticSimilarity { comparison, .. } = &mut proposal.evidence
    {
        comparison.ratio_denominator = 0;
        comparison.ratio_permille = 0;
    }

    assert!(matches!(
        proposal.validate(
            &snapshot_revision_id(),
            &input_identity(),
            &analysis_identity()
        ),
        Err(DetectorProposalRecordValidationError::EvidenceValidation(
            DetectorProposalEvidenceValidationError::ZeroRatioDenominator
        ))
    ));
}

#[test]
fn phonetic_config_mismatch_rejected() {
    let mut proposal = phonetic_proposal("det-prop-001", 1, 0, 0, 6, "widgit", "widget");
    if let DetectorProposalEvidence::PhoneticSimilarity {
        detector_config, ..
    } = &mut proposal.evidence
    {
        *detector_config = detector_component("other-config", "0.1.0");
    }

    assert!(matches!(
        proposal.validate(
            &snapshot_revision_id(),
            &input_identity(),
            &analysis_identity()
        ),
        Err(DetectorProposalRecordValidationError::EvidenceValidation(
            DetectorProposalEvidenceValidationError::DetectorConfigMismatch
        ))
    ));
}

// --- Alternatives ---

#[test]
fn contiguous_ordered_alternatives_pass() {
    let alternatives = vec![
        DetectorProposalAlternative {
            alternative_index: 0,
            replacement_surface: "widget".to_string(),
        },
        DetectorProposalAlternative {
            alternative_index: 1,
            replacement_surface: "gadget".to_string(),
        },
    ];
    let proposal = glossary_proposal(
        "det-prop-001",
        1,
        0,
        0,
        5,
        "wijet",
        "wijet",
        &detector_component("glossary-alias-match", "0.1.0"),
        alternatives,
    );
    proposal
        .validate(
            &snapshot_revision_id(),
            &input_identity(),
            &analysis_identity(),
        )
        .expect("contiguous alternatives");
}

#[test]
fn duplicate_alternative_indices_fail() {
    let alternatives = vec![
        DetectorProposalAlternative {
            alternative_index: 0,
            replacement_surface: "widget".to_string(),
        },
        DetectorProposalAlternative {
            alternative_index: 0,
            replacement_surface: "gadget".to_string(),
        },
    ];
    let proposal = glossary_proposal(
        "det-prop-001",
        1,
        0,
        0,
        5,
        "wijet",
        "wijet",
        &detector_component("glossary-alias-match", "0.1.0"),
        alternatives,
    );

    assert!(matches!(
        proposal.validate(
            &snapshot_revision_id(),
            &input_identity(),
            &analysis_identity()
        ),
        Err(
            DetectorProposalRecordValidationError::AlternativeValidation(
                DetectorProposalAlternativeValidationError::NonContiguousIndex {
                    expected: 1,
                    found: 0
                }
            )
        )
    ));
}

#[test]
fn skipped_alternative_indices_fail() {
    let alternatives = vec![DetectorProposalAlternative {
        alternative_index: 1,
        replacement_surface: "widget".to_string(),
    }];
    let proposal = glossary_proposal(
        "det-prop-001",
        1,
        0,
        0,
        5,
        "wijet",
        "wijet",
        &detector_component("glossary-alias-match", "0.1.0"),
        alternatives,
    );

    assert!(matches!(
        proposal.validate(
            &snapshot_revision_id(),
            &input_identity(),
            &analysis_identity()
        ),
        Err(
            DetectorProposalRecordValidationError::AlternativeValidation(
                DetectorProposalAlternativeValidationError::NonContiguousIndex {
                    expected: 0,
                    found: 1
                }
            )
        )
    ));
}

#[test]
fn empty_alternatives_pass() {
    let proposal = glossary_proposal(
        "det-prop-001",
        1,
        0,
        0,
        5,
        "wijet",
        "wijet",
        &detector_component("glossary-alias-match", "0.1.0"),
        vec![],
    );
    proposal
        .validate(
            &snapshot_revision_id(),
            &input_identity(),
            &analysis_identity(),
        )
        .expect("empty alternatives");
}

#[test]
fn empty_replacement_surface_passes() {
    let alternatives = vec![DetectorProposalAlternative {
        alternative_index: 0,
        replacement_surface: String::new(),
    }];
    let proposal = glossary_proposal(
        "det-prop-001",
        1,
        0,
        0,
        5,
        "wijet",
        "wijet",
        &detector_component("glossary-alias-match", "0.1.0"),
        alternatives,
    );
    proposal
        .validate(
            &snapshot_revision_id(),
            &input_identity(),
            &analysis_identity(),
        )
        .expect("empty replacement surface allowed");
}

// --- Snapshot state ---

#[test]
fn frozen_zero_proposals_passes() {
    let snapshot = build_snapshot(
        vec![],
        DetectorProposalSnapshotState::Frozen,
        1_700_000_000_000,
        CalibrationValidityMode::BlindReference,
    );
    snapshot.validate().expect("frozen empty snapshot");
}

#[test]
fn frozen_valid_proposals_pass() {
    let proposals = vec![glossary_proposal(
        "det-prop-001",
        1,
        0,
        0,
        5,
        "wijet",
        "wijet",
        &detector_component("glossary-alias-match", "0.1.0"),
        vec![],
    )];
    let snapshot = build_snapshot(
        proposals,
        DetectorProposalSnapshotState::Frozen,
        1_700_000_000_000,
        CalibrationValidityMode::BlindReference,
    );
    snapshot.validate().expect("frozen valid snapshot");
}

#[test]
fn frozen_duplicate_proposals_fail() {
    let proposals = vec![
        glossary_proposal(
            "det-prop-dup",
            1,
            0,
            0,
            5,
            "wijet",
            "wijet",
            &detector_component("glossary-alias-match", "0.1.0"),
            vec![],
        ),
        glossary_proposal(
            "det-prop-dup",
            2,
            0,
            0,
            5,
            "wijet",
            "wijet",
            &detector_component("glossary-alias-match", "0.1.0"),
            vec![],
        ),
    ];
    let snapshot = build_snapshot(
        proposals,
        DetectorProposalSnapshotState::Frozen,
        1_700_000_000_000,
        CalibrationValidityMode::BlindReference,
    );

    assert!(matches!(
        snapshot.validate(),
        Err(DetectorProposalSnapshotValidationError::SnapshotStateMismatch { .. })
    ));
}

#[test]
fn draft_incomplete_snapshot_ok() {
    let proposals = vec![
        glossary_proposal(
            "det-prop-dup",
            1,
            0,
            0,
            5,
            "wijet",
            "wijet",
            &detector_component("glossary-alias-match", "0.1.0"),
            vec![],
        ),
        glossary_proposal(
            "det-prop-dup",
            2,
            0,
            0,
            5,
            "wijet",
            "wijet",
            &detector_component("glossary-alias-match", "0.1.0"),
            vec![],
        ),
    ];
    let snapshot = build_snapshot(
        proposals,
        DetectorProposalSnapshotState::Draft,
        0,
        CalibrationValidityMode::BlindReference,
    );
    snapshot.validate().expect("draft allows duplicates");
    assert_eq!(snapshot.assessment.duplicate_proposal_ids.len(), 1);
    assert!(snapshot.assessment.context_consistent);
}

#[test]
fn zero_frozen_timestamp_fails() {
    let snapshot = build_snapshot(
        vec![],
        DetectorProposalSnapshotState::Frozen,
        0,
        CalibrationValidityMode::BlindReference,
    );

    assert!(matches!(
        snapshot.validate(),
        Err(DetectorProposalSnapshotValidationError::ZeroFrozenTimestamp)
    ));
}

// --- Envelope ---

#[test]
fn blind_reference_detector_execution_freeze_passes() {
    let snapshot = build_snapshot(
        vec![],
        DetectorProposalSnapshotState::Frozen,
        1_700_000_000_000,
        CalibrationValidityMode::BlindReference,
    );
    let envelope = detector_execution_envelope(CalibrationValidityMode::BlindReference);

    snapshot
        .validate_for_freeze_against(&envelope)
        .expect("blind reference freeze");
}

#[test]
fn detector_assisted_detector_execution_freeze_passes() {
    let snapshot = build_snapshot(
        vec![],
        DetectorProposalSnapshotState::Frozen,
        1_700_000_000_000,
        CalibrationValidityMode::DetectorAssisted,
    );
    let envelope = detector_execution_envelope(CalibrationValidityMode::DetectorAssisted);

    snapshot
        .validate_for_freeze_against(&envelope)
        .expect("detector assisted freeze");
}

#[test]
fn pre_detector_lifecycle_fails_freeze() {
    let snapshot = build_snapshot(
        vec![],
        DetectorProposalSnapshotState::Frozen,
        1_700_000_000_000,
        CalibrationValidityMode::BlindReference,
    );
    let envelope = lifecycle_envelope(
        CalibrationValidityMode::BlindReference,
        RunLifecycleState::ReferenceSealed,
    );

    assert!(matches!(
        snapshot.validate_for_freeze_against(&envelope),
        Err(DetectorProposalSnapshotValidationError::EnvelopeLifecycleIncompatible { .. })
    ));
}

#[test]
fn assisted_review_cannot_retroactive_freeze() {
    let snapshot = build_snapshot(
        vec![],
        DetectorProposalSnapshotState::Frozen,
        1_700_000_000_000,
        CalibrationValidityMode::DetectorAssisted,
    );
    let envelope = lifecycle_envelope(
        CalibrationValidityMode::DetectorAssisted,
        RunLifecycleState::AssistedReview,
    );

    assert!(matches!(
        snapshot.validate_for_freeze_against(&envelope),
        Err(DetectorProposalSnapshotValidationError::EnvelopeLifecycleIncompatible { .. })
    ));
}

#[test]
fn frozen_valid_during_assisted_review_context() {
    let snapshot = build_snapshot(
        vec![glossary_proposal(
            "det-prop-001",
            1,
            0,
            0,
            5,
            "wijet",
            "wijet",
            &detector_component("glossary-alias-match", "0.1.0"),
            vec![],
        )],
        DetectorProposalSnapshotState::Frozen,
        1_700_000_000_000,
        CalibrationValidityMode::DetectorAssisted,
    );
    let envelope = lifecycle_envelope(
        CalibrationValidityMode::DetectorAssisted,
        RunLifecycleState::AssistedReview,
    );

    snapshot
        .validate_context_against(&envelope)
        .expect("historical frozen snapshot in assisted review");
}

#[test]
fn finalized_context_accepts_historical_snapshot() {
    let snapshot = build_snapshot(
        vec![],
        DetectorProposalSnapshotState::Frozen,
        1_700_000_000_000,
        CalibrationValidityMode::BlindReference,
    );
    let envelope = lifecycle_envelope(
        CalibrationValidityMode::BlindReference,
        RunLifecycleState::Finalized,
    );

    snapshot
        .validate_context_against(&envelope)
        .expect("finalized historical context");
}

#[test]
fn invalidated_envelope_fails() {
    let snapshot = build_snapshot(
        vec![],
        DetectorProposalSnapshotState::Frozen,
        1_700_000_000_000,
        CalibrationValidityMode::BlindReference,
    );
    let envelope = lifecycle_envelope(
        CalibrationValidityMode::BlindReference,
        RunLifecycleState::Invalidated,
    );

    assert!(matches!(
        snapshot.validate_context_against(&envelope),
        Err(DetectorProposalSnapshotValidationError::EnvelopeInvalidated)
    ));
}

// --- Bundle ---

#[test]
fn matching_detector_output_in_bundle_passes() {
    let mode = CalibrationValidityMode::DetectorAssisted;
    let context = binding_context(mode);
    let bundle = build_bundle(
        mode,
        vec![detector_descriptor(
            &context,
            detector_output_artifact_id().as_str(),
        )],
    );
    let envelope = detector_execution_envelope(mode);
    let snapshot = build_snapshot(
        vec![],
        DetectorProposalSnapshotState::Frozen,
        1_700_000_000_000,
        mode,
    );

    snapshot
        .validate_against_bundle(&envelope, &bundle)
        .expect("matching detector output");
}

#[test]
fn absent_detector_output_in_bundle_fails() {
    let mode = CalibrationValidityMode::DetectorAssisted;
    let bundle = build_draft_bundle(mode, vec![]);
    let envelope = detector_execution_envelope(mode);
    let snapshot = build_snapshot(
        vec![],
        DetectorProposalSnapshotState::Frozen,
        1_700_000_000_000,
        mode,
    );

    assert!(matches!(
        snapshot.validate_against_bundle(&envelope, &bundle),
        Err(DetectorProposalSnapshotValidationError::DetectorOutputArtifactMissing)
    ));
}

#[test]
fn mismatched_detector_output_id_fails() {
    let mode = CalibrationValidityMode::DetectorAssisted;
    let context = binding_context(mode);
    let bundle = build_bundle(
        mode,
        vec![detector_descriptor(&context, "artifact-other-output")],
    );
    let envelope = detector_execution_envelope(mode);
    let snapshot = build_snapshot(
        vec![],
        DetectorProposalSnapshotState::Frozen,
        1_700_000_000_000,
        mode,
    );

    assert!(matches!(
        snapshot.validate_against_bundle(&envelope, &bundle),
        Err(DetectorProposalSnapshotValidationError::DetectorOutputArtifactMismatch)
    ));
}

#[test]
fn duplicate_detector_output_role_fails() {
    let mode = CalibrationValidityMode::DetectorAssisted;
    let context = binding_context(mode);
    let bundle = build_draft_bundle(
        mode,
        vec![
            detector_descriptor(&context, "artifact-detector-a"),
            detector_descriptor(&context, "artifact-detector-b"),
        ],
    );
    let envelope = detector_execution_envelope(mode);
    let snapshot = build_snapshot(
        vec![],
        DetectorProposalSnapshotState::Frozen,
        1_700_000_000_000,
        mode,
    );

    assert!(matches!(
        snapshot.validate_against_bundle(&envelope, &bundle),
        Err(DetectorProposalSnapshotValidationError::AmbiguousDetectorOutputRole)
    ));
}

// --- Scope ---

#[test]
fn contract_tests_use_synthetic_strings_only() {
    let proposals = vec![
        glossary_proposal(
            "det-prop-001",
            1,
            0,
            0,
            5,
            "wijet",
            "wijet",
            &detector_component("glossary-alias-match", "0.1.0"),
            vec![],
        ),
        observed_error_proposal("det-prop-002", 2, 0, 0, 6, "widgit", "widgit"),
        phonetic_proposal("det-prop-003", 3, 0, 0, 6, "widgit", "widget"),
    ];
    let snapshot = build_snapshot(
        proposals,
        DetectorProposalSnapshotState::Draft,
        0,
        CalibrationValidityMode::DetectorAssisted,
    );

    snapshot
        .validate()
        .expect("synthetic contract-only validation");
    assert!(SAMPLE_REVISION.starts_with("rev:sha256-v1:"));
    assert!(SAMPLE_SESSION_TERMS.starts_with("session-terms:sha256-v1:"));
    assert!(!snapshot.proposals.is_empty());
}

#[test]
fn caller_cannot_force_assessment_fields() {
    let proposals = vec![glossary_proposal(
        "det-prop-001",
        1,
        0,
        0,
        5,
        "wijet",
        "wijet",
        &detector_component("glossary-alias-match", "0.1.0"),
        vec![],
    )];
    let mut snapshot = build_snapshot(
        proposals,
        DetectorProposalSnapshotState::Draft,
        0,
        CalibrationValidityMode::BlindReference,
    );
    snapshot.assessment = DetectorProposalSnapshotAssessment {
        total_proposal_count: 0,
        duplicate_proposal_ids: vec![],
        duplicate_semantic_keys: vec![],
        context_mismatch_proposal_ids: vec![],
        detector_not_in_analysis_set: vec![],
        context_consistent: true,
    };

    assert!(matches!(
        snapshot.validate(),
        Err(DetectorProposalSnapshotValidationError::AssessmentMismatch { .. })
    ));
}

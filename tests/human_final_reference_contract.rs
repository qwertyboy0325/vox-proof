use vox_proof::human_final_reference::{
    HUMAN_FINAL_REFERENCE_SCHEMA, HumanFinalReference, HumanFinalReferenceState,
    HumanFinalReferenceValidationError, ReferenceClass, ReferenceErrorId, ReferenceErrorRecord,
    ReferenceReviewerIdentityClass, ReferenceRevisionId, ReferenceSourceAnchor, VerificationBasis,
};
use vox_proof::reference_coverage::{
    CueReferenceCoverageRecord, CueReferenceId, ExpectedCueUniverse, REFERENCE_COVERAGE_SCHEMA,
    ReferenceCoverage, ReferenceCoverageId, ReferenceCoveragePurpose, ReferenceCoverageState,
    ReferenceCueDisposition,
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
const OTHER_REVISION: &str =
    "rev:sha256-v1:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

fn input_identity() -> InputIdentityReference {
    InputIdentityReference {
        transcript_revision_id: SAMPLE_REVISION.to_string(),
    }
}

fn revision_id() -> ReferenceRevisionId {
    ReferenceRevisionId::new("ref-rev-001").expect("revision id")
}

fn anchor(cue_id: u32, segment_position: u32, start: u32, end: u32) -> ReferenceSourceAnchor {
    ReferenceSourceAnchor {
        input_identity: input_identity(),
        cue_id: CueReferenceId::new(cue_id).expect("cue id"),
        segment_position,
        start_byte: start,
        end_byte: end,
    }
}

#[allow(clippy::too_many_arguments)]
fn record(
    error_id: &str,
    cue_id: u32,
    segment_position: u32,
    start: u32,
    end: u32,
    original: &str,
    final_surface: &str,
    class: ReferenceClass,
    basis: VerificationBasis,
) -> ReferenceErrorRecord {
    ReferenceErrorRecord {
        reference_error_id: ReferenceErrorId::new(error_id).expect("error id"),
        reference_revision: revision_id(),
        input_identity: input_identity(),
        source_anchor: anchor(cue_id, segment_position, start, end),
        original_surface: original.to_string(),
        human_final_surface: final_surface.to_string(),
        reference_class: class,
        verification_basis: basis,
        reviewer_identity_class: ReferenceReviewerIdentityClass::OwnerBlindReviewer,
        reviewed_at_unix_ms: 1_700_000_000_000,
    }
}

fn build_reference(
    records: Vec<ReferenceErrorRecord>,
    state: HumanFinalReferenceState,
) -> HumanFinalReference {
    let assessment =
        HumanFinalReference::derive_assessment(&revision_id(), &input_identity(), &records)
            .expect("derive assessment");

    HumanFinalReference {
        schema_revision: HUMAN_FINAL_REFERENCE_SCHEMA.to_string(),
        run_id: RunId::new("run-reference").expect("run id"),
        input_identity: input_identity(),
        seal_id: ReferenceSealId::new("seal-primary").expect("seal id"),
        reference_revision: revision_id(),
        records,
        state,
        assessment,
    }
}

fn primary_posture() -> (RunEnvelope, ReferenceSeal) {
    let envelope = RunEnvelope {
        schema_revision: RUN_ENVELOPE_SCHEMA.to_string(),
        run_id: RunId::new("run-reference").expect("run id"),
        input_identity: input_identity(),
        calibration_validity: CalibrationValidityMode::BlindReference,
        workflow_observation: WorkflowObservationMode::Disabled,
        input_class: InputClass::SelfOwnedReal,
        qualifies_as_real_material_evidence: false,
        lifecycle_state: RunLifecycleState::ReferenceSealed,
        expected_artifact_roles: vec![ArtifactRole::CueReviewCompletion],
    };

    let seal = ReferenceSeal {
        schema_revision: REFERENCE_SEAL_SCHEMA.to_string(),
        seal_id: ReferenceSealId::new("seal-primary").expect("seal id"),
        run_id: envelope.run_id.clone(),
        input_identity: envelope.input_identity.clone(),
        reference_revision: revision_id(),
        producer_class: ReferenceProducerClass::HumanBlindReviewer,
        reference_created_before_detector_run: true,
        prior_detector_run_on_same_input: false,
        prior_knowledge_of_detector_targets: false,
        session_terms_visible_during_reference: false,
        external_notes_encode_detector_targets: false,
        seal_state: ReferenceSealState::Sealed,
        calibration_classification: ReferenceCalibrationValidity::BlindReferenceEligible,
        calibration_validity_impact: CalibrationValidityImpact::None,
    };

    (envelope, seal)
}

fn coverage_for(
    cue_dispositions: &[(u32, ReferenceCueDisposition)],
    state: ReferenceCoverageState,
) -> ReferenceCoverage {
    let cue_ids: Vec<CueReferenceId> = cue_dispositions
        .iter()
        .map(|(id, _)| CueReferenceId::new(*id).expect("cue id"))
        .collect();
    let expected = ExpectedCueUniverse {
        total_cues: cue_ids.len() as u32,
        cue_ids: cue_ids.clone(),
    };
    let records: Vec<CueReferenceCoverageRecord> = cue_dispositions
        .iter()
        .map(|(id, disposition)| CueReferenceCoverageRecord {
            cue_id: CueReferenceId::new(*id).expect("cue id"),
            disposition: *disposition,
        })
        .collect();
    let assessment =
        vox_proof::reference_coverage::ReferenceCoverage::derive_assessment(&expected, &records)
            .expect("derive coverage");

    ReferenceCoverage {
        schema_revision: REFERENCE_COVERAGE_SCHEMA.to_string(),
        coverage_id: ReferenceCoverageId::new("coverage-test").expect("coverage id"),
        run_id: RunId::new("run-reference").expect("run id"),
        input_identity: input_identity(),
        seal_id: ReferenceSealId::new("seal-primary").expect("seal id"),
        reference_revision: revision_id(),
        coverage_purpose: ReferenceCoveragePurpose::PrimaryBlindCalibration,
        expected_universe: expected,
        records,
        coverage_state: state,
        assessment,
    }
}

#[test]
fn json_round_trip_retains_schema_and_enum_spellings() {
    let records = vec![record(
        "ref-err-001",
        1,
        0,
        0,
        4,
        "wrng",
        "wrong",
        ReferenceClass::TranscriptionError,
        VerificationBasis::AudioListened,
    )];
    let reference = build_reference(records, HumanFinalReferenceState::Draft);

    let json = serde_json::to_string_pretty(&reference).expect("serialize");
    assert!(json.contains(HUMAN_FINAL_REFERENCE_SCHEMA));
    assert!(json.contains("\"transcription_error\""));
    assert!(json.contains("\"audio_listened\""));
    assert!(json.contains("\"owner_blind_reviewer\""));

    let restored: HumanFinalReference = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, reference);
    restored.validate().expect("valid reference");
}

#[test]
fn unknown_top_level_field_rejected() {
    let reference = build_reference(vec![], HumanFinalReferenceState::Draft);
    let mut value = serde_json::to_value(&reference).expect("value");
    value.as_object_mut().expect("object").insert(
        "detector_case_id".to_string(),
        serde_json::json!("forbidden"),
    );

    let error = serde_json::from_value::<HumanFinalReference>(value).expect_err("must fail");
    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn join_field_on_record_rejected() {
    let record = record(
        "ref-err-001",
        1,
        0,
        0,
        4,
        "wrng",
        "wrong",
        ReferenceClass::TranscriptionError,
        VerificationBasis::AudioListened,
    );
    let mut value = serde_json::to_value(&record).expect("value");
    value
        .as_object_mut()
        .expect("object")
        .insert("match_disposition".to_string(), serde_json::json!("tp"));

    let error = serde_json::from_value::<ReferenceErrorRecord>(value).expect_err("must fail");
    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn valid_revision_and_error_ids_accepted() {
    assert!(ReferenceRevisionId::new("ref-rev-alpha").is_ok());
    assert!(ReferenceErrorId::new("ref-err-alpha").is_ok());
}

#[test]
fn path_like_ids_rejected() {
    for value in [
        "/Users/example/private/reference.json",
        "C:\\Users\\example\\private\\reference.json",
    ] {
        assert!(ReferenceRevisionId::new(value).is_err());
        assert!(ReferenceErrorId::new(value).is_err());
    }
}

#[test]
fn anchor_identity_mismatch_rejected() {
    let mut bad = record(
        "ref-err-001",
        1,
        0,
        0,
        4,
        "wrng",
        "wrong",
        ReferenceClass::TranscriptionError,
        VerificationBasis::AudioListened,
    );
    bad.source_anchor.input_identity = InputIdentityReference {
        transcript_revision_id: OTHER_REVISION.to_string(),
    };

    assert!(matches!(
        HumanFinalReference::derive_assessment(&revision_id(), &input_identity(), &[bad]),
        Err(HumanFinalReferenceValidationError::RecordValidation(
            vox_proof::human_final_reference::ReferenceErrorRecordValidationError::AnchorInputIdentityMismatch
        ))
    ));
}

#[test]
fn zero_and_inverted_anchor_range_rejected() {
    let mut zero = anchor(1, 0, 0, 0);
    assert!(zero.validate().is_err());
    zero.end_byte = 4;
    zero.validate().expect("valid range");

    let inverted = anchor(1, 0, 8, 4);
    assert!(inverted.validate().is_err());
}

#[test]
fn cue_id_zero_rejected() {
    assert!(CueReferenceId::new(0).is_err());
}

#[test]
fn segment_position_and_cue_id_remain_separate() {
    let first = anchor(1, 0, 0, 4);
    let second = anchor(1, 1, 0, 4);
    assert_ne!(first, second);
}

#[test]
fn exact_duplicate_error_ids_rejected_in_sealed_state() {
    let records = vec![
        record(
            "ref-err-dup",
            1,
            0,
            0,
            4,
            "wrng",
            "wrong",
            ReferenceClass::TranscriptionError,
            VerificationBasis::AudioListened,
        ),
        record(
            "ref-err-dup",
            2,
            0,
            0,
            4,
            "othr",
            "other",
            ReferenceClass::TranscriptionError,
            VerificationBasis::AudioListened,
        ),
    ];
    let mut reference = build_reference(records, HumanFinalReferenceState::Sealed);
    reference.state = HumanFinalReferenceState::Sealed;

    assert!(matches!(
        reference.validate(),
        Err(HumanFinalReferenceValidationError::ReferenceStateMismatch { .. })
    ));
}

#[test]
fn exact_duplicate_anchors_rejected_in_sealed_state() {
    let records = vec![
        record(
            "ref-err-001",
            1,
            0,
            0,
            4,
            "wrng",
            "wrong",
            ReferenceClass::TranscriptionError,
            VerificationBasis::AudioListened,
        ),
        record(
            "ref-err-002",
            1,
            0,
            0,
            4,
            "wrng",
            "right",
            ReferenceClass::TranscriptionError,
            VerificationBasis::AudioListened,
        ),
    ];
    let reference = build_reference(records, HumanFinalReferenceState::Sealed);

    assert!(matches!(
        reference.validate(),
        Err(HumanFinalReferenceValidationError::ReferenceStateMismatch { .. })
    ));
}

#[test]
fn overlapping_non_identical_anchors_remain_representable() {
    let records = vec![
        record(
            "ref-err-001",
            1,
            0,
            0,
            4,
            "wrng",
            "wrong",
            ReferenceClass::TranscriptionError,
            VerificationBasis::AudioListened,
        ),
        record(
            "ref-err-002",
            1,
            0,
            2,
            6,
            "ngwo",
            "ngwo",
            ReferenceClass::StylePreference,
            VerificationBasis::TranscriptContextOnly,
        ),
    ];
    let reference = build_reference(records, HumanFinalReferenceState::Draft);
    reference.validate().expect("overlapping anchors allowed");
}

#[test]
fn transcription_error_with_audio_listened_is_recall_eligible() {
    let rec = record(
        "ref-err-001",
        1,
        0,
        0,
        4,
        "wrng",
        "wrong",
        ReferenceClass::TranscriptionError,
        VerificationBasis::AudioListened,
    );
    assert!(rec.is_recall_eligible());
}

#[test]
fn transcription_error_with_mixed_sources_is_recall_eligible() {
    let rec = record(
        "ref-err-001",
        1,
        0,
        0,
        4,
        "wrng",
        "wrong",
        ReferenceClass::TranscriptionError,
        VerificationBasis::MixedSources,
    );
    assert!(rec.is_recall_eligible());
}

#[test]
fn transcript_context_only_is_not_recall_eligible() {
    let rec = record(
        "ref-err-001",
        1,
        0,
        0,
        4,
        "wrng",
        "wrong",
        ReferenceClass::TranscriptionError,
        VerificationBasis::TranscriptContextOnly,
    );
    assert!(!rec.is_recall_eligible());
}

#[test]
fn non_error_classes_are_not_recall_eligible() {
    let rec = record(
        "ref-err-001",
        1,
        0,
        0,
        4,
        "wrng",
        "wrong",
        ReferenceClass::StylePreference,
        VerificationBasis::AudioListened,
    );
    assert!(!rec.is_recall_eligible());
}

#[test]
fn unchanged_original_and_final_cannot_be_recall_eligible_transcription_error() {
    let rec = record(
        "ref-err-001",
        1,
        0,
        0,
        4,
        "same",
        "same",
        ReferenceClass::TranscriptionError,
        VerificationBasis::AudioListened,
    );
    assert!(!rec.is_recall_eligible());
    assert!(matches!(
        rec.validate(&revision_id(), &input_identity()),
        Err(
            vox_proof::human_final_reference::ReferenceErrorRecordValidationError::UnchangedRecallEligibleTranscriptionError
        )
    ));
}

#[test]
fn empty_deletion_correction_is_explicit_and_allowed() {
    let rec = record(
        "ref-err-001",
        1,
        0,
        0,
        4,
        "drop",
        "",
        ReferenceClass::TranscriptionError,
        VerificationBasis::AudioListened,
    );
    assert!(rec.is_recall_eligible());
    rec.validate(&revision_id(), &input_identity())
        .expect("empty final surface allowed");
}

#[test]
fn original_surface_equality_is_not_correction_correctness() {
    let unchanged = record(
        "ref-err-001",
        1,
        0,
        0,
        4,
        "same",
        "same",
        ReferenceClass::TranscriptionError,
        VerificationBasis::TranscriptContextOnly,
    );
    unchanged
        .validate(&revision_id(), &input_identity())
        .expect("unchanged allowed when not recall-eligible basis");
    assert!(!unchanged.is_recall_eligible());
}

#[test]
fn matching_sealed_blind_posture_passes() {
    let records = vec![record(
        "ref-err-001",
        1,
        0,
        0,
        4,
        "wrng",
        "wrong",
        ReferenceClass::TranscriptionError,
        VerificationBasis::AudioListened,
    )];
    let reference = build_reference(records, HumanFinalReferenceState::Sealed);
    let (envelope, seal) = primary_posture();

    reference
        .validate_against(&envelope, &seal)
        .expect("matching sealed blind posture");
}

#[test]
fn mismatched_run_id_fails() {
    let reference = build_reference(vec![], HumanFinalReferenceState::Sealed);
    let (mut envelope, seal) = primary_posture();
    envelope.run_id = RunId::new("run-other").expect("run id");

    assert!(matches!(
        reference.validate_against(&envelope, &seal),
        Err(HumanFinalReferenceValidationError::RunIdMismatch)
    ));
}

#[test]
fn mismatched_input_identity_fails() {
    let reference = build_reference(vec![], HumanFinalReferenceState::Sealed);
    let (mut envelope, mut seal) = primary_posture();
    envelope.input_identity = InputIdentityReference {
        transcript_revision_id: OTHER_REVISION.to_string(),
    };
    seal.input_identity = envelope.input_identity.clone();

    assert!(matches!(
        reference.validate_against(&envelope, &seal),
        Err(HumanFinalReferenceValidationError::InputIdentityMismatch)
    ));
}

#[test]
fn mismatched_seal_id_fails() {
    let reference = build_reference(vec![], HumanFinalReferenceState::Sealed);
    let (envelope, mut seal) = primary_posture();
    seal.seal_id = ReferenceSealId::new("seal-other").expect("seal id");

    assert!(matches!(
        reference.validate_against(&envelope, &seal),
        Err(HumanFinalReferenceValidationError::SealIdMismatch)
    ));
}

#[test]
fn draft_seal_fails_sealed_reference_validation() {
    let reference = build_reference(vec![], HumanFinalReferenceState::Sealed);
    let (envelope, mut seal) = primary_posture();
    seal.seal_state = ReferenceSealState::Draft;

    assert!(matches!(
        reference.validate_against(&envelope, &seal),
        Err(HumanFinalReferenceValidationError::SealStateIncompatible { .. })
    ));
}

#[test]
fn detector_assisted_envelope_fails() {
    let reference = build_reference(vec![], HumanFinalReferenceState::Sealed);
    let (mut envelope, seal) = primary_posture();
    envelope.calibration_validity = CalibrationValidityMode::DetectorAssisted;
    envelope.lifecycle_state = RunLifecycleState::DetectorExecution;

    assert!(matches!(
        reference.validate_against(&envelope, &seal),
        Err(HumanFinalReferenceValidationError::EnvelopeNotBlindReference)
    ));
}

#[test]
fn non_reference_sealed_lifecycle_fails() {
    let reference = build_reference(vec![], HumanFinalReferenceState::Sealed);
    let (mut envelope, seal) = primary_posture();
    envelope.lifecycle_state = RunLifecycleState::Declared;

    assert!(matches!(
        reference.validate_against(&envelope, &seal),
        Err(HumanFinalReferenceValidationError::EnvelopeLifecycleIncompatible { .. })
    ));
}

#[test]
fn transcription_error_cue_with_one_reference_error_passes_coverage() {
    let records = vec![record(
        "ref-err-001",
        1,
        0,
        0,
        4,
        "wrng",
        "wrong",
        ReferenceClass::TranscriptionError,
        VerificationBasis::AudioListened,
    )];
    let reference = build_reference(records, HumanFinalReferenceState::Sealed);
    let coverage = coverage_for(
        &[(1, ReferenceCueDisposition::TranscriptionError)],
        ReferenceCoverageState::Complete,
    );

    reference
        .validate_against_coverage(&coverage)
        .expect("te cue with one te record");
}

#[test]
fn transcription_error_cue_with_multiple_distinct_errors_passes_coverage() {
    let records = vec![
        record(
            "ref-err-001",
            1,
            0,
            0,
            4,
            "wrng",
            "wrong",
            ReferenceClass::TranscriptionError,
            VerificationBasis::AudioListened,
        ),
        record(
            "ref-err-002",
            1,
            1,
            10,
            14,
            "othr",
            "other",
            ReferenceClass::TranscriptionError,
            VerificationBasis::MixedSources,
        ),
    ];
    let reference = build_reference(records, HumanFinalReferenceState::Sealed);
    let coverage = coverage_for(
        &[(1, ReferenceCueDisposition::TranscriptionError)],
        ReferenceCoverageState::Complete,
    );

    reference
        .validate_against_coverage(&coverage)
        .expect("multiple distinct errors in one cue");
}

#[test]
fn transcription_error_cue_without_te_record_fails_coverage() {
    let reference = build_reference(vec![], HumanFinalReferenceState::Sealed);
    let coverage = coverage_for(
        &[(1, ReferenceCueDisposition::TranscriptionError)],
        ReferenceCoverageState::Complete,
    );

    assert!(matches!(
        reference.validate_against_coverage(&coverage),
        Err(HumanFinalReferenceValidationError::TranscriptionErrorCueMissingRecord { .. })
    ));
}

#[test]
fn no_error_cue_with_no_record_passes_coverage() {
    let reference = build_reference(vec![], HumanFinalReferenceState::Sealed);
    let coverage = coverage_for(
        &[(1, ReferenceCueDisposition::NoTranscriptionError)],
        ReferenceCoverageState::Complete,
    );

    reference
        .validate_against_coverage(&coverage)
        .expect("no fabricated records required");
}

#[test]
fn no_error_cue_with_te_record_fails_coverage() {
    let records = vec![record(
        "ref-err-001",
        1,
        0,
        0,
        4,
        "wrng",
        "wrong",
        ReferenceClass::TranscriptionError,
        VerificationBasis::AudioListened,
    )];
    let reference = build_reference(records, HumanFinalReferenceState::Sealed);
    let coverage = coverage_for(
        &[(1, ReferenceCueDisposition::NoTranscriptionError)],
        ReferenceCoverageState::Complete,
    );

    assert!(matches!(
        reference.validate_against_coverage(&coverage),
        Err(HumanFinalReferenceValidationError::NoTranscriptionErrorCueHasRecord { .. })
    ));
}

#[test]
fn unknown_cue_in_record_fails_coverage() {
    let records = vec![record(
        "ref-err-001",
        99,
        0,
        0,
        4,
        "wrng",
        "wrong",
        ReferenceClass::TranscriptionError,
        VerificationBasis::AudioListened,
    )];
    let reference = build_reference(records, HumanFinalReferenceState::Sealed);
    let coverage = coverage_for(
        &[(1, ReferenceCueDisposition::TranscriptionError)],
        ReferenceCoverageState::Complete,
    );

    assert!(matches!(
        reference.validate_against_coverage(&coverage),
        Err(HumanFinalReferenceValidationError::UnknownCueReferenceId { .. })
    ));
}

#[test]
fn uncertain_cue_with_te_record_fails_coverage() {
    let records = vec![record(
        "ref-err-001",
        1,
        0,
        0,
        4,
        "wrng",
        "wrong",
        ReferenceClass::TranscriptionError,
        VerificationBasis::AudioListened,
    )];
    let reference = build_reference(records, HumanFinalReferenceState::Sealed);
    let coverage = coverage_for(
        &[(1, ReferenceCueDisposition::Uncertain)],
        ReferenceCoverageState::Draft,
    );

    assert!(matches!(
        reference.validate_against_coverage(&coverage),
        Err(HumanFinalReferenceValidationError::TranscriptionErrorRecordForUnresolvedCue { .. })
    ));
}

#[test]
fn caller_cannot_force_assessment_fields() {
    let records = vec![record(
        "ref-err-001",
        1,
        0,
        0,
        4,
        "wrng",
        "wrong",
        ReferenceClass::TranscriptionError,
        VerificationBasis::AudioListened,
    )];
    let mut reference = build_reference(records, HumanFinalReferenceState::Draft);
    reference
        .assessment
        .recall_eligible_transcription_error_count = 0;

    assert!(matches!(
        reference.validate(),
        Err(HumanFinalReferenceValidationError::AssessmentMismatch { .. })
    ));
}

#[test]
fn mismatched_seal_reference_revision_fails() {
    let reference = build_reference(vec![], HumanFinalReferenceState::Sealed);
    let (envelope, mut seal) = primary_posture();
    seal.reference_revision = ReferenceRevisionId::new("ref-rev-other").expect("revision id");

    assert!(matches!(
        reference.validate_against(&envelope, &seal),
        Err(HumanFinalReferenceValidationError::ReferenceRevisionMismatch)
    ));
}

#[test]
fn mismatched_coverage_reference_revision_fails() {
    let reference = build_reference(vec![], HumanFinalReferenceState::Sealed);
    let mut coverage = coverage_for(
        &[(1, ReferenceCueDisposition::NoTranscriptionError)],
        ReferenceCoverageState::Complete,
    );
    coverage.reference_revision = ReferenceRevisionId::new("ref-rev-other").expect("revision id");

    assert!(matches!(
        reference.validate_against_coverage(&coverage),
        Err(HumanFinalReferenceValidationError::CoverageReferenceRevisionMismatch)
    ));
}

#[test]
fn post_seal_new_revision_cannot_reuse_old_coverage() {
    let records = vec![record(
        "ref-err-001",
        1,
        0,
        0,
        4,
        "wrng",
        "wrong",
        ReferenceClass::TranscriptionError,
        VerificationBasis::AudioListened,
    )];
    let mut reference = build_reference(records, HumanFinalReferenceState::Sealed);
    reference.reference_revision = ReferenceRevisionId::new("ref-rev-next").expect("revision id");
    for record in &mut reference.records {
        record.reference_revision = reference.reference_revision.clone();
    }
    reference.assessment = HumanFinalReference::derive_assessment(
        &reference.reference_revision,
        &reference.input_identity,
        &reference.records,
    )
    .expect("derive assessment");

    let coverage = coverage_for(
        &[(1, ReferenceCueDisposition::TranscriptionError)],
        ReferenceCoverageState::Complete,
    );

    assert!(matches!(
        reference.validate_against_coverage(&coverage),
        Err(HumanFinalReferenceValidationError::CoverageReferenceRevisionMismatch)
    ));
}

#[test]
fn unsupported_schema_fails_validation() {
    let mut reference = build_reference(vec![], HumanFinalReferenceState::Draft);
    reference.schema_revision = "voxproof-human-final-reference-v0".to_string();

    assert!(matches!(
        reference.validate(),
        Err(HumanFinalReferenceValidationError::UnsupportedSchemaRevision { .. })
    ));
}

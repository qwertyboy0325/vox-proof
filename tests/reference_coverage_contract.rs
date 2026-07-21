use vox_proof::human_final_reference::{
    HUMAN_FINAL_REFERENCE_SCHEMA, HumanFinalReference, HumanFinalReferenceState,
    HumanFinalReferenceValidationError, ReferenceClass, ReferenceErrorId, ReferenceErrorRecord,
    ReferenceReviewerIdentityClass, ReferenceSourceAnchor, VerificationBasis,
};
use vox_proof::reference_coverage::{
    CueReferenceId, CueReviewCompletionRecord, ExpectedCueUniverse, REFERENCE_COVERAGE_SCHEMA,
    ReferenceCoverage, ReferenceCoverageId, ReferenceCoveragePurpose, ReferenceCoverageState,
    ReferenceCoverageValidationError, ReferenceCueDisposition,
};
use vox_proof::reference_identity::{CueSourceTextDigest, ReferenceRevisionId};
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
const SAMPLE_REFERENCE_REVISION: &str = "ref-rev-001";
const SAMPLE_CUE_DIGEST: &str =
    "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn universe(cue_ids: &[u32]) -> ExpectedCueUniverse {
    ExpectedCueUniverse {
        total_cues: cue_ids.len() as u32,
        cue_ids: cue_ids
            .iter()
            .map(|id| CueReferenceId::new(*id).expect("cue id"))
            .collect(),
    }
}

fn record_at(
    cue_id: u32,
    segment_position: u32,
    disposition: ReferenceCueDisposition,
) -> CueReviewCompletionRecord {
    record_with_attestations(cue_id, segment_position, disposition, true, true)
}

fn record(cue_id: u32, disposition: ReferenceCueDisposition) -> CueReviewCompletionRecord {
    record_at(cue_id, cue_id - 1, disposition)
}

fn record_with_attestations(
    cue_id: u32,
    segment_position: u32,
    disposition: ReferenceCueDisposition,
    fully_reviewed: bool,
    all_known_transcription_errors_enumerated: bool,
) -> CueReviewCompletionRecord {
    CueReviewCompletionRecord {
        cue_id: CueReferenceId::new(cue_id).expect("cue id"),
        segment_position,
        source_text_digest: CueSourceTextDigest::new(SAMPLE_CUE_DIGEST).expect("digest"),
        disposition,
        fully_reviewed,
        all_known_transcription_errors_enumerated,
        verification_source_used: VerificationBasis::AudioListened,
        reviewer_identity_class: ReferenceReviewerIdentityClass::OwnerBlindReviewer,
        completed_at_unix_ms: if fully_reviewed || all_known_transcription_errors_enumerated {
            1_700_000_000_000
        } else {
            0
        },
    }
}

fn build_coverage(
    purpose: ReferenceCoveragePurpose,
    expected: ExpectedCueUniverse,
    records: Vec<CueReviewCompletionRecord>,
    run_id: &str,
    revision: &str,
    seal_id: &str,
) -> ReferenceCoverage {
    let assessment =
        ReferenceCoverage::derive_assessment(&expected, &records).expect("derive assessment");

    ReferenceCoverage {
        schema_revision: REFERENCE_COVERAGE_SCHEMA.to_string(),
        coverage_id: ReferenceCoverageId::new("coverage-test").expect("coverage id"),
        run_id: RunId::new(run_id).expect("run id"),
        input_identity: InputIdentityReference {
            transcript_revision_id: revision.to_string(),
        },
        seal_id: ReferenceSealId::new(seal_id).expect("seal id"),
        reference_revision: ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION)
            .expect("revision id"),
        coverage_purpose: purpose,
        expected_universe: expected,
        records,
        coverage_state: ReferenceCoverageState::Draft,
        assessment,
    }
}

fn primary_posture() -> (RunEnvelope, ReferenceSeal) {
    let envelope = RunEnvelope {
        schema_revision: RUN_ENVELOPE_SCHEMA.to_string(),
        run_id: RunId::new("run-primary").expect("run id"),
        input_identity: InputIdentityReference {
            transcript_revision_id: SAMPLE_REVISION.to_string(),
        },
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
    };

    (envelope, seal)
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

#[test]
fn json_round_trip_retains_schema_and_enum_spellings() {
    let expected = universe(&[1, 2]);
    let records = vec![
        record(1, ReferenceCueDisposition::NoTranscriptionError),
        record(2, ReferenceCueDisposition::TranscriptionError),
    ];
    let coverage = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        expected,
        records,
        "run-primary",
        SAMPLE_REVISION,
        "seal-primary",
    );

    let json = serde_json::to_string_pretty(&coverage).expect("serialize");
    assert!(json.contains(REFERENCE_COVERAGE_SCHEMA));
    assert!(json.contains("\"no_transcription_error\""));
    assert!(json.contains("\"primary_blind_calibration\""));

    let restored: ReferenceCoverage = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, coverage);
    restored.validate().expect("valid coverage");
}

#[test]
fn unknown_top_level_field_rejected() {
    let coverage = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        universe(&[1]),
        vec![record(1, ReferenceCueDisposition::NoTranscriptionError)],
        "run-primary",
        SAMPLE_REVISION,
        "seal-primary",
    );
    let mut value = serde_json::to_value(&coverage).expect("value");
    value.as_object_mut().expect("object").insert(
        "transcript_text".to_string(),
        serde_json::json!("forbidden"),
    );

    let error = serde_json::from_value::<ReferenceCoverage>(value).expect_err("must fail");
    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn unknown_enum_value_rejected() {
    let json = format!(
        r#"{{
  "schema_revision": "{REFERENCE_COVERAGE_SCHEMA}",
  "coverage_id": "coverage-test",
  "run_id": "run-primary",
  "input_identity": {{ "transcript_revision_id": "{SAMPLE_REVISION}" }},
  "seal_id": "seal-primary",
  "reference_revision": "{SAMPLE_REFERENCE_REVISION}",
  "coverage_purpose": "primary_blind_calibration",
  "expected_universe": {{ "total_cues": 1, "cue_ids": [1] }},
  "records": [{{ "cue_id": 1, "disposition": "maybe_error" }}],
  "coverage_state": "draft",
  "assessment": {{
    "expected_count": 1,
    "observed_unique_count": 1,
    "missing_cue_ids": [],
    "duplicate_cue_ids": [],
    "unknown_cue_ids": [],
    "unresolved_cue_ids": [],
    "inventory_complete": true,
    "reference_resolved": true
  }}
}}"#
    );

    let error = serde_json::from_str::<ReferenceCoverage>(&json).expect_err("must fail");
    assert!(error.to_string().contains("unknown variant"));
}

#[test]
fn unsupported_schema_rejected_during_validation() {
    let mut coverage = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        universe(&[1]),
        vec![record(1, ReferenceCueDisposition::NoTranscriptionError)],
        "run-primary",
        SAMPLE_REVISION,
        "seal-primary",
    );
    coverage.schema_revision = "voxproof-per-cue-reference-coverage-v0".to_string();

    assert!(matches!(
        coverage.validate(),
        Err(ReferenceCoverageValidationError::UnsupportedSchemaRevision { .. })
    ));
}

#[test]
fn duplicate_expected_cue_ids_rejected() {
    let expected = ExpectedCueUniverse {
        total_cues: 2,
        cue_ids: vec![
            CueReferenceId::new(1).expect("cue"),
            CueReferenceId::new(1).expect("cue"),
        ],
    };

    assert!(matches!(
        ReferenceCoverage::derive_assessment(&expected, &[]),
        Err(ReferenceCoverageValidationError::DuplicateExpectedCueId { .. })
    ));
}

#[test]
fn expected_count_mismatch_rejected() {
    let expected = ExpectedCueUniverse {
        total_cues: 3,
        cue_ids: vec![CueReferenceId::new(1).expect("cue")],
    };

    assert!(matches!(
        ReferenceCoverage::derive_assessment(&expected, &[]),
        Err(ReferenceCoverageValidationError::ExpectedCountMismatch { .. })
    ));
}

#[test]
fn empty_universe_rejected() {
    let expected = ExpectedCueUniverse {
        total_cues: 0,
        cue_ids: vec![],
    };

    assert!(matches!(
        ReferenceCoverage::derive_assessment(&expected, &[]),
        Err(ReferenceCoverageValidationError::EmptyExpectedUniverse)
    ));
}

#[test]
fn zero_cue_id_rejected() {
    assert!(CueReferenceId::new(0).is_err());
}

#[test]
fn explicit_no_error_counts_as_reviewed_inventory() {
    let expected = universe(&[1]);
    let records = vec![record(1, ReferenceCueDisposition::NoTranscriptionError)];
    let assessment = ReferenceCoverage::derive_assessment(&expected, &records).expect("derive");

    assert!(assessment.inventory_complete);
    assert!(assessment.reference_resolved);
    assert!(assessment.coverage_complete);
}

#[test]
fn missing_record_is_not_treated_as_no_error() {
    let expected = universe(&[1, 2]);
    let records = vec![record(1, ReferenceCueDisposition::NoTranscriptionError)];
    let assessment = ReferenceCoverage::derive_assessment(&expected, &records).expect("derive");

    assert!(!assessment.inventory_complete);
    assert_eq!(
        assessment.missing_cue_ids,
        vec![CueReferenceId::new(2).expect("cue")]
    );
}

#[test]
fn duplicate_observed_cue_prevents_inventory_complete() {
    let expected = universe(&[1]);
    let records = vec![
        record(1, ReferenceCueDisposition::NoTranscriptionError),
        record(1, ReferenceCueDisposition::TranscriptionError),
    ];
    let assessment = ReferenceCoverage::derive_assessment(&expected, &records).expect("derive");

    assert!(!assessment.inventory_complete);
    assert_eq!(
        assessment.duplicate_cue_ids,
        vec![CueReferenceId::new(1).expect("cue")]
    );
}

#[test]
fn unknown_observed_cue_prevents_inventory_complete() {
    let expected = universe(&[1]);
    let records = vec![record(2, ReferenceCueDisposition::NoTranscriptionError)];
    let assessment = ReferenceCoverage::derive_assessment(&expected, &records).expect("derive");

    assert!(!assessment.inventory_complete);
    assert_eq!(
        assessment.unknown_cue_ids,
        vec![CueReferenceId::new(2).expect("cue")]
    );
    assert_eq!(
        assessment.missing_cue_ids,
        vec![CueReferenceId::new(1).expect("cue")]
    );
}

#[test]
fn uncertain_prevents_reference_resolved() {
    let expected = universe(&[1]);
    let records = vec![record(1, ReferenceCueDisposition::Uncertain)];
    let assessment = ReferenceCoverage::derive_assessment(&expected, &records).expect("derive");

    assert!(assessment.inventory_complete);
    assert!(!assessment.reference_resolved);
    assert_eq!(
        assessment.unresolved_cue_ids,
        vec![CueReferenceId::new(1).expect("cue")]
    );
}

#[test]
fn unreviewable_prevents_reference_resolved() {
    let expected = universe(&[1]);
    let records = vec![record(1, ReferenceCueDisposition::Unreviewable)];
    let assessment = ReferenceCoverage::derive_assessment(&expected, &records).expect("derive");

    assert!(assessment.inventory_complete);
    assert!(!assessment.reference_resolved);
}

#[test]
fn caller_cannot_force_stored_assessment_inconsistent_with_derivation() {
    let mut coverage = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        universe(&[1, 2]),
        vec![record(1, ReferenceCueDisposition::NoTranscriptionError)],
        "run-primary",
        SAMPLE_REVISION,
        "seal-primary",
    );
    coverage.assessment.inventory_complete = true;
    coverage.assessment.reference_resolved = true;

    assert!(matches!(
        coverage.validate(),
        Err(ReferenceCoverageValidationError::AssessmentMismatch { .. })
    ));
}

#[test]
fn same_cue_id_under_different_revision_is_separate_attachment_context() {
    let (envelope_a, seal_a) = primary_posture();
    let coverage_a = primary_coverage_for_attachment(vec![record(
        1,
        ReferenceCueDisposition::NoTranscriptionError,
    )]);
    let human_reference = human_reference_for_coverage(&coverage_a);

    let mut envelope_b = envelope_a.clone();
    envelope_b.input_identity.transcript_revision_id = OTHER_REVISION.to_string();
    let mut seal_b = seal_a.clone();
    seal_b.input_identity = envelope_b.input_identity.clone();

    coverage_a
        .validate_against(&envelope_a, &seal_a, Some(&human_reference))
        .expect("matches revision A");

    assert!(matches!(
        coverage_a.validate_against(&envelope_b, &seal_b, None),
        Err(ReferenceCoverageValidationError::InputIdentityMismatch)
    ));
}

#[test]
fn primary_attachment_passes_for_reference_sealed_and_sealed_seal() {
    let (envelope, seal) = primary_posture();
    let coverage = primary_coverage_for_attachment(vec![
        record(1, ReferenceCueDisposition::NoTranscriptionError),
        record(2, ReferenceCueDisposition::TranscriptionError),
    ]);
    let human_reference = human_reference_for_coverage(&coverage);

    coverage
        .validate_against(&envelope, &seal, Some(&human_reference))
        .expect("primary attachment");
}

fn synthetic_protocol_seal(envelope: &RunEnvelope) -> ReferenceSeal {
    ReferenceSeal {
        schema_revision: REFERENCE_SEAL_SCHEMA.to_string(),
        seal_id: ReferenceSealId::new("seal-synthetic").expect("seal id"),
        run_id: envelope.run_id.clone(),
        input_identity: envelope.input_identity.clone(),
        reference_revision: ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION)
            .expect("revision id"),
        producer_class: ReferenceProducerClass::SyntheticFixtureGenerator,
        reference_created_before_detector_run: true,
        prior_detector_run_on_same_input: false,
        prior_knowledge_of_detector_targets: false,
        session_terms_visible_during_reference: false,
        external_notes_encode_detector_targets: false,
        seal_state: ReferenceSealState::Sealed,
        calibration_classification: ReferenceCalibrationValidity::SyntheticProtocolOnly,
        calibration_validity_impact: CalibrationValidityImpact::ProtocolOnly,
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

fn primary_coverage_for_attachment(records: Vec<CueReviewCompletionRecord>) -> ReferenceCoverage {
    let cue_ids: Vec<u32> = records.iter().map(|record| record.cue_id.value()).collect();
    let mut coverage = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        universe(&cue_ids),
        records,
        "run-primary",
        SAMPLE_REVISION,
        "seal-primary",
    );
    let human_reference = human_reference_for_coverage(&coverage);
    coverage.assessment.total_eligible_transcription_errors = human_reference
        .assessment
        .recall_eligible_transcription_error_count;
    coverage.coverage_state = ReferenceCoverageState::Complete;
    coverage
}

fn reference_error_record(
    error_id: &str,
    cue_id: u32,
    segment_position: u32,
    start: u32,
    end: u32,
) -> ReferenceErrorRecord {
    ReferenceErrorRecord {
        reference_error_id: ReferenceErrorId::new(error_id).expect("error id"),
        reference_revision: ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION).expect("revision"),
        input_identity: InputIdentityReference {
            transcript_revision_id: SAMPLE_REVISION.to_string(),
        },
        source_anchor: ReferenceSourceAnchor {
            input_identity: InputIdentityReference {
                transcript_revision_id: SAMPLE_REVISION.to_string(),
            },
            cue_id: CueReferenceId::new(cue_id).expect("cue id"),
            segment_position,
            start_byte: start,
            end_byte: end,
        },
        original_surface: "wrng".to_string(),
        human_final_surface: "wrong".to_string(),
        reference_class: ReferenceClass::TranscriptionError,
        verification_basis: VerificationBasis::AudioListened,
        reviewer_identity_class: ReferenceReviewerIdentityClass::OwnerBlindReviewer,
        reviewed_at_unix_ms: 1_700_000_000_000,
    }
}

fn valid_primary_attachment() -> (
    RunEnvelope,
    ReferenceSeal,
    ReferenceCoverage,
    HumanFinalReference,
) {
    let (envelope, seal) = primary_posture();
    let coverage = primary_coverage_for_attachment(vec![
        record(1, ReferenceCueDisposition::NoTranscriptionError),
        record(2, ReferenceCueDisposition::TranscriptionError),
    ]);
    let human_reference = human_reference_for_coverage(&coverage);
    (envelope, seal, coverage, human_reference)
}

fn assert_human_reference_validation_failure(
    result: Result<(), ReferenceCoverageValidationError>,
    expected: HumanFinalReferenceValidationError,
) {
    assert!(matches!(
        result,
        Err(ReferenceCoverageValidationError::HumanReferenceValidation(inner))
        if *inner == expected
    ));
}

#[test]
fn draft_seal_fails_primary_coverage() {
    let (envelope, mut seal) = primary_posture();
    seal.seal_state = ReferenceSealState::Draft;

    let coverage = primary_coverage_for_attachment(vec![record(
        1,
        ReferenceCueDisposition::NoTranscriptionError,
    )]);

    assert!(matches!(
        coverage.validate_against(&envelope, &seal, None),
        Err(ReferenceCoverageValidationError::SealStateIncompatible { .. })
    ));
}

#[test]
fn term_conditioned_seal_fails_primary_coverage() {
    let (envelope, seal) = primary_posture();
    let mut contaminated = seal.clone();
    contaminated.session_terms_visible_during_reference = true;
    contaminated.calibration_classification =
        ReferenceCalibrationValidity::TermConditionedDiagnostic;
    contaminated.calibration_validity_impact =
        CalibrationValidityImpact::ExcludedFromPrimaryMetrics;

    let coverage = primary_coverage_for_attachment(vec![record(
        1,
        ReferenceCueDisposition::NoTranscriptionError,
    )]);

    assert!(matches!(
        coverage.validate_against(&envelope, &contaminated, None),
        Err(ReferenceCoverageValidationError::SealClassificationIncompatible { .. })
    ));
}

#[test]
fn contaminated_seal_fails_primary_coverage() {
    let (envelope, seal) = primary_posture();
    let mut contaminated = seal.clone();
    contaminated.calibration_classification = ReferenceCalibrationValidity::DetectorContaminated;
    contaminated.calibration_validity_impact =
        CalibrationValidityImpact::ExcludedFromPrimaryMetrics;
    contaminated.prior_detector_run_on_same_input = true;

    let coverage = primary_coverage_for_attachment(vec![record(
        1,
        ReferenceCueDisposition::NoTranscriptionError,
    )]);

    assert!(matches!(
        coverage.validate_against(&envelope, &contaminated, None),
        Err(ReferenceCoverageValidationError::SealClassificationIncompatible { .. })
    ));
}

#[test]
fn synthetic_seal_fails_primary_coverage() {
    let (envelope, mut seal) = primary_posture();
    seal.producer_class = ReferenceProducerClass::SyntheticFixtureGenerator;
    seal.calibration_classification = ReferenceCalibrationValidity::SyntheticProtocolOnly;
    seal.calibration_validity_impact = CalibrationValidityImpact::ProtocolOnly;

    let coverage = primary_coverage_for_attachment(vec![record(
        1,
        ReferenceCueDisposition::NoTranscriptionError,
    )]);

    assert!(matches!(
        coverage.validate_against(&envelope, &seal, None),
        Err(ReferenceCoverageValidationError::SealClassificationIncompatible { .. })
    ));
}

#[test]
fn detector_assisted_envelope_fails_coverage_attachment() {
    let (mut envelope, seal) = primary_posture();
    envelope.calibration_validity = CalibrationValidityMode::DetectorAssisted;
    envelope.lifecycle_state = RunLifecycleState::Declared;

    let coverage = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        universe(&[1]),
        vec![record(1, ReferenceCueDisposition::NoTranscriptionError)],
        "run-primary",
        SAMPLE_REVISION,
        "seal-primary",
    );

    assert!(matches!(
        coverage.validate_against(&envelope, &seal, None),
        Err(ReferenceCoverageValidationError::EnvelopeNotBlindReference)
    ));
}

#[test]
fn reference_preparation_lifecycle_fails_primary_coverage() {
    let (mut envelope, seal) = primary_posture();
    envelope.lifecycle_state = RunLifecycleState::ReferencePreparation;

    let coverage = primary_coverage_for_attachment(vec![record(
        1,
        ReferenceCueDisposition::NoTranscriptionError,
    )]);

    assert!(matches!(
        coverage.validate_against(&envelope, &seal, None),
        Err(ReferenceCoverageValidationError::EnvelopeLifecycleIncompatible { .. })
    ));
}

#[test]
fn detector_execution_lifecycle_rejects_retroactive_coverage() {
    let (mut envelope, seal) = primary_posture();
    envelope.lifecycle_state = RunLifecycleState::DetectorExecution;

    let coverage = primary_coverage_for_attachment(vec![record(
        1,
        ReferenceCueDisposition::NoTranscriptionError,
    )]);

    assert!(matches!(
        coverage.validate_against(&envelope, &seal, None),
        Err(ReferenceCoverageValidationError::EnvelopeLifecycleIncompatible { .. })
    ));
}

#[test]
fn historical_primary_coverage_passes_at_later_active_lifecycle_states() {
    let (mut envelope, seal, coverage, human_reference) = valid_primary_attachment();

    for lifecycle_state in [
        RunLifecycleState::DetectorExecution,
        RunLifecycleState::AssistedReview,
        RunLifecycleState::Finalized,
    ] {
        envelope.lifecycle_state = lifecycle_state;
        coverage
            .validate_historical_context(&envelope, &seal, Some(&human_reference))
            .unwrap_or_else(|error| {
                panic!("historical primary coverage must pass at {lifecycle_state:?}: {error:?}")
            });
    }
}

#[test]
fn historical_primary_coverage_requires_sealed_human_reference() {
    let (mut envelope, seal, coverage, _) = valid_primary_attachment();
    envelope.lifecycle_state = RunLifecycleState::DetectorExecution;

    assert!(matches!(
        coverage.validate_historical_context(&envelope, &seal, None),
        Err(ReferenceCoverageValidationError::HumanReferenceRequiredForCompleteCoverage)
    ));
}

#[test]
fn draft_human_reference_fails_before_historical_alignment() {
    let (mut envelope, seal, coverage, mut human_reference) = valid_primary_attachment();
    envelope.lifecycle_state = RunLifecycleState::DetectorExecution;
    human_reference.state = HumanFinalReferenceState::Draft;

    assert_human_reference_validation_failure(
        coverage.validate_historical_context(&envelope, &seal, Some(&human_reference)),
        HumanFinalReferenceValidationError::ReferenceStateMismatch {
            state: HumanFinalReferenceState::Draft,
            assessment: Box::new(human_reference.assessment.clone()),
        },
    );
}

#[test]
fn incomplete_coverage_fails_historical_primary_validation() {
    let (mut envelope, seal, mut coverage, human_reference) = valid_primary_attachment();
    envelope.lifecycle_state = RunLifecycleState::DetectorExecution;
    coverage.coverage_state = ReferenceCoverageState::Draft;

    assert!(matches!(
        coverage.validate_historical_context(&envelope, &seal, Some(&human_reference)),
        Err(ReferenceCoverageValidationError::PrimaryAttachmentRequiresCompleteState)
    ));
}

#[test]
fn historical_primary_coverage_rejects_contaminated_seal() {
    let (mut envelope, mut seal, coverage, human_reference) = valid_primary_attachment();
    envelope.lifecycle_state = RunLifecycleState::DetectorExecution;
    seal.calibration_classification = ReferenceCalibrationValidity::DetectorContaminated;
    seal.calibration_validity_impact = CalibrationValidityImpact::ExcludedFromPrimaryMetrics;
    seal.prior_detector_run_on_same_input = true;

    assert!(matches!(
        coverage.validate_historical_context(&envelope, &seal, Some(&human_reference)),
        Err(ReferenceCoverageValidationError::SealClassificationIncompatible { .. })
    ));
}

#[test]
fn historical_validator_rejects_reference_preparation_and_invalidated() {
    let (mut envelope, seal, coverage, human_reference) = valid_primary_attachment();

    for lifecycle_state in [
        RunLifecycleState::Declared,
        RunLifecycleState::ReferencePreparation,
        RunLifecycleState::Invalidated,
    ] {
        envelope.lifecycle_state = lifecycle_state;
        assert!(matches!(
            coverage.validate_historical_context(&envelope, &seal, Some(&human_reference)),
            Err(ReferenceCoverageValidationError::HistoricalEnvelopeLifecycleIncompatible { .. })
        ));
    }
}

#[test]
fn mismatched_run_id_fails_attachment() {
    let (envelope, seal) = primary_posture();
    let coverage = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        universe(&[1]),
        vec![record(1, ReferenceCueDisposition::NoTranscriptionError)],
        "run-other",
        SAMPLE_REVISION,
        "seal-primary",
    );

    assert!(matches!(
        coverage.validate_against(&envelope, &seal, None),
        Err(ReferenceCoverageValidationError::RunIdMismatch)
    ));
}

#[test]
fn mismatched_seal_id_fails_attachment() {
    let (envelope, seal) = primary_posture();
    let coverage = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        universe(&[1]),
        vec![record(1, ReferenceCueDisposition::NoTranscriptionError)],
        "run-primary",
        SAMPLE_REVISION,
        "seal-other",
    );

    assert!(matches!(
        coverage.validate_against(&envelope, &seal, None),
        Err(ReferenceCoverageValidationError::SealIdMismatch)
    ));
}

#[test]
fn diagnostic_coverage_allows_term_conditioned_seal() {
    let (mut envelope, _seal) = primary_posture();
    envelope.lifecycle_state = RunLifecycleState::ReferencePreparation;
    let diagnostic_seal = term_conditioned_seal(&envelope);

    let coverage = build_coverage(
        ReferenceCoveragePurpose::DiagnosticOnly,
        universe(&[1]),
        vec![record(1, ReferenceCueDisposition::Uncertain)],
        "run-primary",
        SAMPLE_REVISION,
        "seal-diagnostic",
    );

    coverage
        .validate_against(&envelope, &diagnostic_seal, None)
        .expect("diagnostic attachment");
    assert!(!coverage.assessment.reference_resolved);
}

#[test]
fn complete_diagnostic_coverage_validates_when_resolved() {
    let mut coverage = build_coverage(
        ReferenceCoveragePurpose::DiagnosticOnly,
        universe(&[1, 2]),
        vec![
            record(1, ReferenceCueDisposition::NoTranscriptionError),
            record(2, ReferenceCueDisposition::TranscriptionError),
        ],
        "run-primary",
        SAMPLE_REVISION,
        "seal-diagnostic",
    );
    coverage.coverage_state = ReferenceCoverageState::Complete;

    assert!(coverage.assessment.inventory_complete);
    assert!(coverage.assessment.reference_resolved);
    coverage
        .validate()
        .expect("resolved diagnostic coverage valid");
}

#[test]
fn complete_synthetic_coverage_validates_when_resolved() {
    let mut coverage = build_coverage(
        ReferenceCoveragePurpose::SyntheticProtocolValidation,
        universe(&[1]),
        vec![record(1, ReferenceCueDisposition::NoTranscriptionError)],
        "run-primary",
        SAMPLE_REVISION,
        "seal-synthetic",
    );
    coverage.coverage_state = ReferenceCoverageState::Complete;

    assert!(coverage.assessment.inventory_complete);
    assert!(coverage.assessment.reference_resolved);
    coverage
        .validate()
        .expect("resolved synthetic coverage valid");
}

#[test]
fn unresolved_diagnostic_coverage_derives_reference_resolved_false() {
    let coverage = build_coverage(
        ReferenceCoveragePurpose::DiagnosticOnly,
        universe(&[1]),
        vec![record(1, ReferenceCueDisposition::Uncertain)],
        "run-primary",
        SAMPLE_REVISION,
        "seal-diagnostic",
    );

    assert!(coverage.assessment.inventory_complete);
    assert!(!coverage.assessment.reference_resolved);
}

#[test]
fn unresolved_synthetic_coverage_derives_reference_resolved_false() {
    let coverage = build_coverage(
        ReferenceCoveragePurpose::SyntheticProtocolValidation,
        universe(&[1]),
        vec![record(1, ReferenceCueDisposition::Unreviewable)],
        "run-primary",
        SAMPLE_REVISION,
        "seal-synthetic",
    );

    assert!(coverage.assessment.inventory_complete);
    assert!(!coverage.assessment.reference_resolved);
}

#[test]
fn changing_only_coverage_purpose_does_not_mutate_structural_assessment() {
    let records = vec![
        record(1, ReferenceCueDisposition::NoTranscriptionError),
        record(2, ReferenceCueDisposition::TranscriptionError),
    ];
    let expected = universe(&[1, 2]);

    let primary = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        expected.clone(),
        records.clone(),
        "run-primary",
        SAMPLE_REVISION,
        "seal-primary",
    );
    let diagnostic = build_coverage(
        ReferenceCoveragePurpose::DiagnosticOnly,
        expected,
        records,
        "run-primary",
        SAMPLE_REVISION,
        "seal-primary",
    );

    assert_eq!(primary.assessment, diagnostic.assessment);
}

#[test]
fn obsolete_primary_reference_complete_field_rejected() {
    let coverage = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        universe(&[1]),
        vec![record(1, ReferenceCueDisposition::NoTranscriptionError)],
        "run-primary",
        SAMPLE_REVISION,
        "seal-primary",
    );
    let mut value = serde_json::to_value(&coverage).expect("value");
    value
        .as_object_mut()
        .expect("object")
        .get_mut("assessment")
        .expect("assessment")
        .as_object_mut()
        .expect("assessment object")
        .insert(
            "primary_reference_complete".to_string(),
            serde_json::json!(true),
        );

    let error = serde_json::from_value::<ReferenceCoverage>(value).expect_err("must fail");
    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn json_round_trip_retains_reference_resolved_field() {
    let mut coverage = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        universe(&[1]),
        vec![record(1, ReferenceCueDisposition::NoTranscriptionError)],
        "run-primary",
        SAMPLE_REVISION,
        "seal-primary",
    );
    coverage.coverage_state = ReferenceCoverageState::Complete;

    let json = serde_json::to_string(&coverage).expect("serialize");
    assert!(json.contains("\"reference_resolved\""));
    assert!(!json.contains("primary_reference_complete"));
}

#[test]
fn primary_attachment_requires_complete_state() {
    let (envelope, seal) = primary_posture();
    let coverage = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        universe(&[1]),
        vec![record(1, ReferenceCueDisposition::NoTranscriptionError)],
        "run-primary",
        SAMPLE_REVISION,
        "seal-primary",
    );

    assert!(matches!(
        coverage.validate_against(&envelope, &seal, None),
        Err(ReferenceCoverageValidationError::PrimaryAttachmentRequiresCompleteState)
    ));
}

#[test]
fn draft_unresolved_coverage_fails_primary_attachment_before_complete_state() {
    let (envelope, seal) = primary_posture();
    let coverage = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        universe(&[1]),
        vec![record(1, ReferenceCueDisposition::Uncertain)],
        "run-primary",
        SAMPLE_REVISION,
        "seal-primary",
    );

    assert!(matches!(
        coverage.validate_against(&envelope, &seal, None),
        Err(ReferenceCoverageValidationError::PrimaryAttachmentRequiresCompleteState)
    ));
}

#[test]
fn complete_unresolved_coverage_fails_validation() {
    let mut coverage = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        universe(&[1]),
        vec![record(1, ReferenceCueDisposition::Uncertain)],
        "run-primary",
        SAMPLE_REVISION,
        "seal-primary",
    );
    coverage.coverage_state = ReferenceCoverageState::Complete;

    assert!(matches!(
        coverage.validate(),
        Err(ReferenceCoverageValidationError::CoverageStateMismatch { .. })
    ));
}

#[test]
fn draft_coverage_fails_primary_attachment() {
    let (envelope, seal) = primary_posture();
    let coverage = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        universe(&[1]),
        vec![record(1, ReferenceCueDisposition::NoTranscriptionError)],
        "run-primary",
        SAMPLE_REVISION,
        "seal-primary",
    );

    assert!(matches!(
        coverage.validate_against(&envelope, &seal, None),
        Err(ReferenceCoverageValidationError::PrimaryAttachmentRequiresCompleteState)
    ));
}

#[test]
fn forbidden_path_like_coverage_id_rejected() {
    for value in [
        "/Users/example/private/reference.json",
        "C:\\Users\\example\\private\\reference.json",
        "../private/reference.json",
    ] {
        assert!(
            ReferenceCoverageId::new(value).is_err(),
            "coverage id must reject {value:?}"
        );
    }
}

#[test]
fn serialized_coverage_contains_no_content_or_path_fields() {
    let coverage = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        universe(&[1]),
        vec![record(1, ReferenceCueDisposition::NoTranscriptionError)],
        "run-primary",
        SAMPLE_REVISION,
        "seal-primary",
    );
    let json = serde_json::to_string(&coverage).expect("serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");

    for forbidden in [
        "transcript_text",
        "cue_text",
        "correction",
        "detector_output",
        "session_terms",
        "path",
        "reviewer_name",
        "precision",
        "recall",
    ] {
        assert!(
            value.get(forbidden).is_none(),
            "serialized coverage must not contain {forbidden:?}"
        );
    }
}

#[test]
fn complete_state_requires_reference_resolved_for_all_purposes() {
    let mut coverage = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        universe(&[1, 2]),
        vec![record(1, ReferenceCueDisposition::NoTranscriptionError)],
        "run-primary",
        SAMPLE_REVISION,
        "seal-primary",
    );
    coverage.coverage_state = ReferenceCoverageState::Complete;

    assert!(matches!(
        coverage.validate(),
        Err(ReferenceCoverageValidationError::CoverageStateMismatch { .. })
    ));
}

#[test]
fn complete_state_accepts_resolved_diagnostic_coverage() {
    let mut envelope = primary_posture().0;
    envelope.lifecycle_state = RunLifecycleState::ReferencePreparation;
    let seal = term_conditioned_seal(&envelope);

    let mut coverage = build_coverage(
        ReferenceCoveragePurpose::DiagnosticOnly,
        universe(&[1]),
        vec![record(1, ReferenceCueDisposition::NoTranscriptionError)],
        "run-primary",
        SAMPLE_REVISION,
        "seal-diagnostic",
    );
    coverage.coverage_state = ReferenceCoverageState::Complete;

    coverage
        .validate()
        .expect("resolved diagnostic complete valid");
    coverage
        .validate_against(&envelope, &seal, None)
        .expect("diagnostic attachment");
}

#[test]
fn synthetic_protocol_attachment_allows_resolved_protocol_only_seal() {
    let mut envelope = primary_posture().0;
    envelope.lifecycle_state = RunLifecycleState::ReferencePreparation;
    let seal = synthetic_protocol_seal(&envelope);

    let mut coverage = build_coverage(
        ReferenceCoveragePurpose::SyntheticProtocolValidation,
        universe(&[1]),
        vec![record(1, ReferenceCueDisposition::NoTranscriptionError)],
        "run-primary",
        SAMPLE_REVISION,
        "seal-synthetic",
    );
    coverage.coverage_state = ReferenceCoverageState::Complete;

    coverage.validate().expect("synthetic complete valid");
    coverage
        .validate_against(&envelope, &seal, None)
        .expect("synthetic protocol attachment");
}

#[test]
fn missing_reference_revision_rejected_in_json() {
    let json = format!(
        r#"{{
  "schema_revision": "{REFERENCE_COVERAGE_SCHEMA}",
  "coverage_id": "coverage-test",
  "run_id": "run-primary",
  "input_identity": {{ "transcript_revision_id": "{SAMPLE_REVISION}" }},
  "seal_id": "seal-primary",
  "coverage_purpose": "primary_blind_calibration",
  "expected_universe": {{ "total_cues": 1, "cue_ids": [1] }},
  "records": [{{ "cue_id": 1, "disposition": "no_transcription_error" }}],
  "coverage_state": "draft",
  "assessment": {{
    "expected_count": 1,
    "observed_unique_count": 1,
    "missing_cue_ids": [],
    "duplicate_cue_ids": [],
    "unknown_cue_ids": [],
    "unresolved_cue_ids": [],
    "inventory_complete": true,
    "reference_resolved": true
  }}
}}"#
    );

    let error = serde_json::from_str::<ReferenceCoverage>(&json).expect_err("must fail");
    assert!(error.to_string().contains("missing field"));
}

#[test]
fn mismatched_reference_revision_fails_attachment() {
    let (envelope, seal) = primary_posture();
    let mut coverage = build_coverage(
        ReferenceCoveragePurpose::PrimaryBlindCalibration,
        universe(&[1]),
        vec![record(1, ReferenceCueDisposition::NoTranscriptionError)],
        "run-primary",
        SAMPLE_REVISION,
        "seal-primary",
    );
    coverage.reference_revision = ReferenceRevisionId::new("ref-rev-other").expect("revision id");
    coverage.coverage_state = ReferenceCoverageState::Complete;

    assert!(matches!(
        coverage.validate_against(&envelope, &seal, None),
        Err(ReferenceCoverageValidationError::ReferenceRevisionMismatch)
    ));
}

#[test]
fn reference_revision_does_not_change_structural_assessment() {
    let expected = universe(&[1]);
    let records = vec![record(1, ReferenceCueDisposition::Uncertain)];
    let assessment = ReferenceCoverage::derive_assessment(&expected, &records).expect("derive");
    assert!(assessment.inventory_complete);
    assert!(!assessment.reference_resolved);
}

#[test]
fn legacy_minimal_record_missing_required_fields_rejected() {
    let json = format!(
        r#"{{
  "schema_revision": "{REFERENCE_COVERAGE_SCHEMA}",
  "coverage_id": "coverage-test",
  "run_id": "run-primary",
  "input_identity": {{ "transcript_revision_id": "{SAMPLE_REVISION}" }},
  "seal_id": "seal-primary",
  "reference_revision": "{SAMPLE_REFERENCE_REVISION}",
  "coverage_purpose": "primary_blind_calibration",
  "expected_universe": {{ "total_cues": 1, "cue_ids": [1] }},
  "records": [{{ "cue_id": 1, "disposition": "no_transcription_error" }}],
  "coverage_state": "draft",
  "assessment": {{
    "expected_count": 1,
    "observed_unique_count": 0,
    "completed_cue_count": 0,
    "incomplete_cue_count": 1,
    "cues_with_transcription_errors": 0,
    "total_eligible_transcription_errors": 0,
    "missing_cue_ids": [1],
    "duplicate_cue_ids": [],
    "duplicate_segment_positions": [],
    "unknown_cue_ids": [],
    "invalid_mapping_cue_ids": [],
    "unresolved_cue_ids": [],
    "incomplete_attestation_cue_ids": [],
    "inventory_complete": false,
    "reference_resolved": false,
    "coverage_complete": false
  }}
}}"#
    );

    let error = serde_json::from_str::<ReferenceCoverage>(&json).expect_err("must fail");
    assert!(error.to_string().contains("missing field"));
}

#[test]
fn cue_source_digest_rejects_invalid_forms() {
    for invalid in [
        "SHA256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "sha256:0123456789abcdef",
        "sha256:gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg",
    ] {
        assert!(
            CueSourceTextDigest::new(invalid).is_err(),
            "digest must reject {invalid:?}"
        );
    }

    assert!(CueSourceTextDigest::new(SAMPLE_CUE_DIGEST).is_ok());
}

#[test]
fn non_contiguous_cue_ids_map_by_segment_position() {
    let expected = universe(&[5, 17]);
    let records = vec![
        record_at(5, 0, ReferenceCueDisposition::NoTranscriptionError),
        record_at(17, 1, ReferenceCueDisposition::NoTranscriptionError),
    ];
    let assessment = ReferenceCoverage::derive_assessment(&expected, &records).expect("derive");
    assert!(assessment.inventory_complete);
    assert!(assessment.coverage_complete);
}

#[test]
fn duplicate_segment_position_prevents_inventory_complete() {
    let expected = universe(&[1, 2]);
    let records = vec![
        record_at(1, 0, ReferenceCueDisposition::NoTranscriptionError),
        record_at(2, 0, ReferenceCueDisposition::NoTranscriptionError),
    ];
    let assessment = ReferenceCoverage::derive_assessment(&expected, &records).expect("derive");
    assert!(!assessment.inventory_complete);
    assert_eq!(assessment.duplicate_segment_positions, vec![0]);
}

#[test]
fn cue_mapping_mismatch_prevents_inventory_complete() {
    let expected = universe(&[1]);
    let records = vec![record_at(
        2,
        0,
        ReferenceCueDisposition::NoTranscriptionError,
    )];
    let assessment = ReferenceCoverage::derive_assessment(&expected, &records).expect("derive");
    assert!(!assessment.inventory_complete);
    assert_eq!(
        assessment.invalid_mapping_cue_ids,
        vec![CueReferenceId::new(2).expect("cue")]
    );
}

#[test]
fn incomplete_review_attestation_prevents_coverage_complete() {
    let expected = universe(&[1]);
    let records = vec![record_with_attestations(
        1,
        0,
        ReferenceCueDisposition::NoTranscriptionError,
        false,
        true,
    )];
    let assessment = ReferenceCoverage::derive_assessment(&expected, &records).expect("derive");
    assert!(assessment.inventory_complete);
    assert!(!assessment.coverage_complete);
}

#[test]
fn incomplete_enumeration_attestation_prevents_coverage_complete() {
    let expected = universe(&[1]);
    let records = vec![record_with_attestations(
        1,
        0,
        ReferenceCueDisposition::NoTranscriptionError,
        true,
        false,
    )];
    let assessment = ReferenceCoverage::derive_assessment(&expected, &records).expect("derive");
    assert!(assessment.inventory_complete);
    assert!(!assessment.coverage_complete);
}

#[test]
fn primary_complete_coverage_requires_human_reference_for_attachment() {
    let (envelope, seal) = primary_posture();
    let coverage = primary_coverage_for_attachment(vec![record(
        1,
        ReferenceCueDisposition::NoTranscriptionError,
    )]);

    assert!(matches!(
        coverage.validate_against(&envelope, &seal, None),
        Err(ReferenceCoverageValidationError::HumanReferenceRequiredForCompleteCoverage)
    ));
}

#[test]
fn coverage_and_reference_validation_agree_for_valid_primary_set() {
    let (envelope, seal, coverage, human_reference) = valid_primary_attachment();

    coverage
        .validate_against(&envelope, &seal, Some(&human_reference))
        .expect("coverage validates with human reference");
    human_reference
        .validate_against_coverage(&coverage)
        .expect("human reference validates against coverage");
}

#[test]
fn draft_human_reference_rejected_before_coverage_alignment() {
    let (envelope, seal, coverage, mut human_reference) = valid_primary_attachment();
    human_reference.state = HumanFinalReferenceState::Draft;

    assert_human_reference_validation_failure(
        coverage.validate_against(&envelope, &seal, Some(&human_reference)),
        HumanFinalReferenceValidationError::ReferenceStateMismatch {
            state: HumanFinalReferenceState::Draft,
            assessment: Box::new(human_reference.assessment.clone()),
        },
    );
}

#[test]
fn unsupported_human_reference_schema_rejected_before_coverage_alignment() {
    let (envelope, seal, coverage, mut human_reference) = valid_primary_attachment();
    human_reference.schema_revision = "voxproof-human-final-reference-v0".to_string();

    assert!(matches!(
        coverage.validate_against(&envelope, &seal, Some(&human_reference)),
        Err(ReferenceCoverageValidationError::HumanReferenceValidation(
            _
        ))
    ));
}

#[test]
fn duplicate_human_reference_error_ids_rejected_before_coverage_alignment() {
    let (envelope, seal, coverage, mut human_reference) = valid_primary_attachment();
    human_reference
        .records
        .push(reference_error_record("ref-err-2", 2, 1, 10, 14));
    human_reference
        .records
        .push(reference_error_record("ref-err-2", 2, 1, 15, 19));
    human_reference.assessment = HumanFinalReference::derive_assessment(
        &human_reference.reference_revision,
        &human_reference.input_identity,
        &human_reference.records,
    )
    .expect("derive assessment");

    assert!(matches!(
        coverage.validate_against(&envelope, &seal, Some(&human_reference)),
        Err(ReferenceCoverageValidationError::HumanReferenceValidation(
            inner
        )) if matches!(*inner, HumanFinalReferenceValidationError::ReferenceStateMismatch { .. })
    ));
}

#[test]
fn duplicate_human_reference_anchors_rejected_before_coverage_alignment() {
    let (envelope, seal, coverage, mut human_reference) = valid_primary_attachment();
    human_reference
        .records
        .push(reference_error_record("ref-err-extra", 2, 1, 0, 4));
    human_reference.assessment = HumanFinalReference::derive_assessment(
        &human_reference.reference_revision,
        &human_reference.input_identity,
        &human_reference.records,
    )
    .expect("derive assessment");

    assert!(matches!(
        coverage.validate_against(&envelope, &seal, Some(&human_reference)),
        Err(ReferenceCoverageValidationError::HumanReferenceValidation(
            inner
        )) if matches!(*inner, HumanFinalReferenceValidationError::ReferenceStateMismatch { .. })
    ));
}

#[test]
fn mismatched_human_reference_assessment_rejected_before_coverage_alignment() {
    let (envelope, seal, coverage, mut human_reference) = valid_primary_attachment();
    human_reference
        .assessment
        .recall_eligible_transcription_error_count = 0;

    assert_human_reference_validation_failure(
        coverage.validate_against(&envelope, &seal, Some(&human_reference)),
        HumanFinalReferenceValidationError::AssessmentMismatch {
            stored: Box::new(human_reference.assessment.clone()),
            derived: Box::new(
                HumanFinalReference::derive_assessment(
                    &human_reference.reference_revision,
                    &human_reference.input_identity,
                    &human_reference.records,
                )
                .expect("derive assessment"),
            ),
        },
    );
}

#[test]
fn invalid_human_reference_record_rejected_before_coverage_alignment() {
    let (envelope, seal, coverage, mut human_reference) = valid_primary_attachment();
    human_reference.records[0].source_anchor.start_byte = 4;
    human_reference.records[0].source_anchor.end_byte = 0;

    assert!(matches!(
        coverage.validate_against(&envelope, &seal, Some(&human_reference)),
        Err(ReferenceCoverageValidationError::HumanReferenceValidation(
            inner
        )) if matches!(
            *inner,
            HumanFinalReferenceValidationError::RecordValidation(_)
                | HumanFinalReferenceValidationError::InvalidSourceAnchor(_)
        )
    ));
}

#[test]
fn mismatched_human_reference_run_id_rejected_before_coverage_alignment() {
    let (envelope, seal, coverage, mut human_reference) = valid_primary_attachment();
    human_reference.run_id = RunId::new("run-other").expect("run id");

    assert_human_reference_validation_failure(
        coverage.validate_against(&envelope, &seal, Some(&human_reference)),
        HumanFinalReferenceValidationError::RunIdMismatch,
    );
}

#[test]
fn mismatched_human_reference_revision_rejected_before_coverage_alignment() {
    let (envelope, seal, coverage, mut human_reference) = valid_primary_attachment();
    human_reference.reference_revision =
        ReferenceRevisionId::new("ref-rev-other").expect("revision id");
    for record in &mut human_reference.records {
        record.reference_revision = human_reference.reference_revision.clone();
    }
    human_reference.assessment = HumanFinalReference::derive_assessment(
        &human_reference.reference_revision,
        &human_reference.input_identity,
        &human_reference.records,
    )
    .expect("derive assessment");

    assert_human_reference_validation_failure(
        coverage.validate_against(&envelope, &seal, Some(&human_reference)),
        HumanFinalReferenceValidationError::ReferenceRevisionMismatch,
    );
}

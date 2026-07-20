use vox_proof::reference_seal::{
    CalibrationValidityImpact, REFERENCE_SEAL_SCHEMA, ReferenceCalibrationValidity,
    ReferenceProducerClass, ReferenceSeal, ReferenceSealId, ReferenceSealState,
    ReferenceSealValidationError,
};
use vox_proof::run_manifest::{
    ArtifactRole, CalibrationValidityMode, InputClass, InputIdentityReference, RUN_ENVELOPE_SCHEMA,
    RunEnvelope, RunId, RunLifecycleState, WorkflowObservationMode,
};

const SAMPLE_REVISION: &str =
    "rev:sha256-v1:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn blind_envelope(lifecycle_state: RunLifecycleState) -> RunEnvelope {
    RunEnvelope {
        schema_revision: RUN_ENVELOPE_SCHEMA.to_string(),
        run_id: RunId::new("run-blind-reference").expect("run id"),
        input_identity: InputIdentityReference {
            transcript_revision_id: SAMPLE_REVISION.to_string(),
        },
        calibration_validity: CalibrationValidityMode::BlindReference,
        workflow_observation: WorkflowObservationMode::Disabled,
        input_class: InputClass::SelfOwnedReal,
        qualifies_as_real_material_evidence: false,
        lifecycle_state,
        expected_artifact_roles: vec![ArtifactRole::ReferenceSeal],
    }
}

fn seal_with_attestations(
    producer_class: ReferenceProducerClass,
    reference_created_before_detector_run: bool,
    prior_detector_run_on_same_input: bool,
    prior_knowledge_of_detector_targets: bool,
    session_terms_visible_during_reference: bool,
    external_notes_encode_detector_targets: bool,
) -> ReferenceSeal {
    let mut seal = ReferenceSeal {
        schema_revision: REFERENCE_SEAL_SCHEMA.to_string(),
        seal_id: ReferenceSealId::new("seal-test").expect("seal id"),
        run_id: RunId::new("run-blind-reference").expect("run id"),
        input_identity: InputIdentityReference {
            transcript_revision_id: SAMPLE_REVISION.to_string(),
        },
        producer_class,
        reference_created_before_detector_run,
        prior_detector_run_on_same_input,
        prior_knowledge_of_detector_targets,
        session_terms_visible_during_reference,
        external_notes_encode_detector_targets,
        seal_state: ReferenceSealState::Draft,
        calibration_classification: ReferenceCalibrationValidity::Invalid,
        calibration_validity_impact: CalibrationValidityImpact::Invalid,
    };

    let classification = seal.derive_calibration_classification();
    seal.calibration_classification = classification;
    seal.calibration_validity_impact =
        ReferenceSeal::derive_calibration_validity_impact(classification);
    seal
}

#[test]
fn json_round_trip_retains_schema_and_enum_spellings() {
    let seal = seal_with_attestations(
        ReferenceProducerClass::HumanBlindReviewer,
        true,
        false,
        false,
        false,
        false,
    );

    let json = serde_json::to_string_pretty(&seal).expect("serialize");
    assert!(json.contains(REFERENCE_SEAL_SCHEMA));
    assert!(json.contains("\"human_blind_reviewer\""));
    assert!(json.contains("\"blind_reference_eligible\""));
    assert!(json.contains("\"none\""));

    let restored: ReferenceSeal = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, seal);
    restored.validate().expect("valid seal");
}

#[test]
fn unknown_top_level_field_rejected() {
    let json = format!(
        r#"{{
  "schema_revision": "{REFERENCE_SEAL_SCHEMA}",
  "seal_id": "seal-test",
  "run_id": "run-blind-reference",
  "input_identity": {{ "transcript_revision_id": "{SAMPLE_REVISION}" }},
  "producer_class": "human_blind_reviewer",
  "reference_created_before_detector_run": true,
  "prior_detector_run_on_same_input": false,
  "prior_knowledge_of_detector_targets": false,
  "session_terms_visible_during_reference": false,
  "external_notes_encode_detector_targets": false,
  "seal_state": "draft",
  "calibration_classification": "blind_reference_eligible",
  "calibration_validity_impact": "none",
  "transcript_text": "forbidden"
}}"#
    );

    let error = serde_json::from_str::<ReferenceSeal>(&json).expect_err("must fail");
    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn unknown_enum_value_rejected() {
    let json = format!(
        r#"{{
  "schema_revision": "{REFERENCE_SEAL_SCHEMA}",
  "seal_id": "seal-test",
  "run_id": "run-blind-reference",
  "input_identity": {{ "transcript_revision_id": "{SAMPLE_REVISION}" }},
  "producer_class": "robot_reviewer",
  "reference_created_before_detector_run": true,
  "prior_detector_run_on_same_input": false,
  "prior_knowledge_of_detector_targets": false,
  "session_terms_visible_during_reference": false,
  "external_notes_encode_detector_targets": false,
  "seal_state": "draft",
  "calibration_classification": "blind_reference_eligible",
  "calibration_validity_impact": "none"
}}"#
    );

    let error = serde_json::from_str::<ReferenceSeal>(&json).expect_err("must fail");
    assert!(error.to_string().contains("unknown variant"));
}

#[test]
fn unsupported_schema_rejected_during_validation() {
    let mut seal = seal_with_attestations(
        ReferenceProducerClass::HumanBlindReviewer,
        true,
        false,
        false,
        false,
        false,
    );
    seal.schema_revision = "voxproof-blind-reference-seal-v0".to_string();

    assert!(matches!(
        seal.validate(),
        Err(ReferenceSealValidationError::UnsupportedSchemaRevision { .. })
    ));
}

#[test]
fn exact_valid_blind_attestation_classifies_eligible() {
    let seal = seal_with_attestations(
        ReferenceProducerClass::HumanBlindReviewer,
        true,
        false,
        false,
        false,
        false,
    );

    assert_eq!(
        seal.calibration_classification,
        ReferenceCalibrationValidity::BlindReferenceEligible
    );
    assert_eq!(
        seal.calibration_validity_impact,
        CalibrationValidityImpact::None
    );
    seal.validate().expect("valid blind seal");
}

#[test]
fn each_invalidating_field_independently_prevents_eligibility() {
    let cases = [
        (false, false, false, false, false), // reference_created_before_detector_run
        (true, true, false, false, false),   // prior_detector_run_on_same_input
        (true, false, true, false, false),   // prior_knowledge_of_detector_targets
        (true, false, false, false, true),   // external_notes_encode_detector_targets
    ];

    for (
        reference_created_before_detector_run,
        prior_detector_run_on_same_input,
        prior_knowledge_of_detector_targets,
        session_terms_visible_during_reference,
        external_notes_encode_detector_targets,
    ) in cases
    {
        let seal = seal_with_attestations(
            ReferenceProducerClass::HumanBlindReviewer,
            reference_created_before_detector_run,
            prior_detector_run_on_same_input,
            prior_knowledge_of_detector_targets,
            session_terms_visible_during_reference,
            external_notes_encode_detector_targets,
        );

        assert_ne!(
            seal.calibration_classification,
            ReferenceCalibrationValidity::BlindReferenceEligible,
            "attestation case must not be eligible"
        );
    }
}

#[test]
fn session_terms_visible_produces_term_conditioned_when_uncontaminated() {
    let seal = seal_with_attestations(
        ReferenceProducerClass::HumanBlindReviewer,
        true,
        false,
        false,
        true,
        false,
    );

    assert_eq!(
        seal.calibration_classification,
        ReferenceCalibrationValidity::TermConditionedDiagnostic
    );
    assert_eq!(
        seal.calibration_validity_impact,
        CalibrationValidityImpact::ExcludedFromPrimaryMetrics
    );
}

#[test]
fn detector_exposure_precedence_over_term_conditioned() {
    let seal = seal_with_attestations(
        ReferenceProducerClass::HumanBlindReviewer,
        true,
        true,
        false,
        true,
        false,
    );

    assert_eq!(
        seal.calibration_classification,
        ReferenceCalibrationValidity::DetectorContaminated
    );
}

#[test]
fn human_assisted_producer_cannot_classify_blind() {
    let seal = seal_with_attestations(
        ReferenceProducerClass::HumanDetectorAssistedReviewer,
        true,
        false,
        false,
        false,
        false,
    );

    assert_eq!(
        seal.calibration_classification,
        ReferenceCalibrationValidity::DetectorContaminated
    );
}

#[test]
fn synthetic_producer_is_protocol_only() {
    let seal = seal_with_attestations(
        ReferenceProducerClass::SyntheticFixtureGenerator,
        true,
        false,
        false,
        false,
        false,
    );

    assert_eq!(
        seal.calibration_classification,
        ReferenceCalibrationValidity::SyntheticProtocolOnly
    );
    assert_eq!(
        seal.calibration_validity_impact,
        CalibrationValidityImpact::ProtocolOnly
    );
}

#[test]
fn caller_cannot_force_eligible_classification_inconsistent_with_attestations() {
    let mut seal = seal_with_attestations(
        ReferenceProducerClass::HumanBlindReviewer,
        true,
        true,
        false,
        false,
        false,
    );
    seal.calibration_classification = ReferenceCalibrationValidity::BlindReferenceEligible;
    seal.calibration_validity_impact = CalibrationValidityImpact::None;

    assert!(matches!(
        seal.validate(),
        Err(ReferenceSealValidationError::ClassificationMismatch { .. })
    ));
}

#[test]
fn validity_impact_must_match_derived_classification() {
    let mut seal = seal_with_attestations(
        ReferenceProducerClass::HumanBlindReviewer,
        true,
        false,
        false,
        true,
        false,
    );
    seal.calibration_validity_impact = CalibrationValidityImpact::None;

    assert!(matches!(
        seal.validate(),
        Err(ReferenceSealValidationError::ValidityImpactMismatch { .. })
    ));
}

#[test]
fn seal_run_id_must_match_envelope() {
    let seal = seal_with_attestations(
        ReferenceProducerClass::HumanBlindReviewer,
        true,
        false,
        false,
        false,
        false,
    );
    let mut envelope = blind_envelope(RunLifecycleState::ReferencePreparation);
    envelope.run_id = RunId::new("run-other").expect("run id");

    assert!(matches!(
        seal.validate_against_run_envelope(&envelope),
        Err(vox_proof::reference_seal::ReferenceSealEnvelopeConsistencyError::RunIdMismatch { .. })
    ));
}

#[test]
fn input_identity_must_match_envelope() {
    let seal = seal_with_attestations(
        ReferenceProducerClass::HumanBlindReviewer,
        true,
        false,
        false,
        false,
        false,
    );
    let mut envelope = blind_envelope(RunLifecycleState::ReferencePreparation);
    envelope.input_identity.transcript_revision_id =
        "rev:sha256-v1:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
            .to_string();

    assert!(matches!(
        seal.validate_against_run_envelope(&envelope),
        Err(
            vox_proof::reference_seal::ReferenceSealEnvelopeConsistencyError::InputIdentityMismatch
        )
    ));
}

#[test]
fn detector_assisted_envelope_rejects_seal() {
    let seal = seal_with_attestations(
        ReferenceProducerClass::HumanBlindReviewer,
        true,
        false,
        false,
        false,
        false,
    );
    let mut envelope = blind_envelope(RunLifecycleState::ReferencePreparation);
    envelope.calibration_validity = CalibrationValidityMode::DetectorAssisted;
    envelope.lifecycle_state = RunLifecycleState::Declared;

    assert!(matches!(
        seal.validate_against_run_envelope(&envelope),
        Err(
            vox_proof::reference_seal::ReferenceSealEnvelopeConsistencyError::EnvelopeNotBlindReference
        )
    ));
}

#[test]
fn detector_execution_lifecycle_rejects_retroactive_seal() {
    let seal = seal_with_attestations(
        ReferenceProducerClass::HumanBlindReviewer,
        true,
        false,
        false,
        false,
        false,
    );
    let envelope = blind_envelope(RunLifecycleState::DetectorExecution);

    assert!(matches!(
        seal.validate_against_run_envelope(&envelope),
        Err(
            vox_proof::reference_seal::ReferenceSealEnvelopeConsistencyError::EnvelopeLifecycleIncompatible { .. }
        )
    ));
}

#[test]
fn valid_blind_reference_preparation_and_sealed_postures_pass() {
    let seal = seal_with_attestations(
        ReferenceProducerClass::HumanBlindReviewer,
        true,
        false,
        false,
        false,
        false,
    );

    for lifecycle_state in [
        RunLifecycleState::ReferencePreparation,
        RunLifecycleState::ReferenceSealed,
    ] {
        let envelope = blind_envelope(lifecycle_state);
        seal.validate_with_envelope(&envelope)
            .expect("compatible lifecycle");
    }
}

#[test]
fn sealed_state_cannot_mutate_to_draft() {
    assert!(matches!(
        ReferenceSealState::Sealed.transition(ReferenceSealState::Draft),
        Err(ReferenceSealValidationError::ImmutableSealedStateMutation)
    ));
}

#[test]
fn forbidden_path_like_values_rejected_in_seal_id() {
    for value in [
        "/Users/example/private/reference.json",
        "C:\\Users\\example\\private\\reference.json",
        "../private/reference.json",
    ] {
        assert!(
            ReferenceSealId::new(value).is_err(),
            "seal id must reject {value:?}"
        );
    }
}

#[test]
fn serialized_seal_contains_no_path_or_content_fields() {
    let seal = seal_with_attestations(
        ReferenceProducerClass::HumanBlindReviewer,
        true,
        false,
        false,
        false,
        false,
    );
    let json = serde_json::to_string(&seal).expect("serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");

    assert!(value.get("transcript_text").is_none());
    assert!(value.get("cue_text").is_none());
    assert!(value.get("detector_output").is_none());
    assert!(value.get("session_terms").is_none());
    assert!(value.get("path").is_none());
    assert!(value.get("audio").is_none());
    assert!(value.get("reviewer_name").is_none());
    assert!(value.get("email").is_none());

    for forbidden in [
        "/Users/example/private/reference.json",
        "C:\\Users\\example\\private\\reference.json",
        "../private/reference.json",
    ] {
        assert!(
            !json.contains(forbidden),
            "serialized seal must not contain path {forbidden:?}"
        );
    }
}

#[test]
fn generated_seal_id_is_valid_opaque_identifier() {
    let generated = ReferenceSealId::generate().expect("generated seal id");
    assert!(generated.as_str().starts_with("seal-"));
    assert!(generated.as_str().len() <= 128);
}

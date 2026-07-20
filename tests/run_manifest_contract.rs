use vox_proof::run_manifest::{
    ArtifactRole, CalibrationValidityMode, InputClass, InputIdentityReference, RUN_ENVELOPE_SCHEMA,
    RunEnvelope, RunEnvelopeValidationError, RunId, RunIdError, RunLifecycleState,
    WorkflowObservationMode,
};

const SAMPLE_REVISION: &str =
    "rev:sha256-v1:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn sample_envelope(
    run_id: &str,
    calibration_validity: CalibrationValidityMode,
    lifecycle_state: RunLifecycleState,
    input_class: InputClass,
    qualifies_as_real_material_evidence: bool,
) -> RunEnvelope {
    RunEnvelope {
        schema_revision: RUN_ENVELOPE_SCHEMA.to_string(),
        run_id: RunId::new(run_id).expect("valid run id"),
        input_identity: InputIdentityReference {
            transcript_revision_id: SAMPLE_REVISION.to_string(),
        },
        calibration_validity,
        workflow_observation: WorkflowObservationMode::Disabled,
        input_class,
        qualifies_as_real_material_evidence,
        lifecycle_state,
        expected_artifact_roles: vec![ArtifactRole::ReviewLedger],
    }
}

#[test]
fn json_round_trip_retains_schema_and_enum_spellings() {
    let envelope = sample_envelope(
        "run-20260720-alpha",
        CalibrationValidityMode::BlindReference,
        RunLifecycleState::Declared,
        InputClass::SelfOwnedReal,
        false,
    );

    let json = serde_json::to_string_pretty(&envelope).expect("serialize");
    assert!(json.contains(RUN_ENVELOPE_SCHEMA));
    assert!(json.contains("\"blind_reference\""));
    assert!(json.contains("\"self_owned_real\""));
    assert!(json.contains("\"declared\""));

    let restored: RunEnvelope = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, envelope);
    restored.validate().expect("valid envelope");
}

#[test]
fn unknown_enum_value_fails_deserialization() {
    let json = format!(
        r#"{{
  "schema_revision": "{RUN_ENVELOPE_SCHEMA}",
  "run_id": "run-alpha",
  "input_identity": {{ "transcript_revision_id": "{SAMPLE_REVISION}" }},
  "calibration_validity": "hybrid_mode",
  "workflow_observation": "disabled",
  "input_class": "self_owned_real",
  "qualifies_as_real_material_evidence": false,
  "lifecycle_state": "declared",
  "expected_artifact_roles": ["review_ledger"]
}}"#
    );

    let error = serde_json::from_str::<RunEnvelope>(&json).expect_err("must fail");
    assert!(error.to_string().contains("unknown variant"));
}

#[test]
fn unsupported_schema_version_fails_validation() {
    let mut envelope = sample_envelope(
        "run-alpha",
        CalibrationValidityMode::BlindReference,
        RunLifecycleState::Declared,
        InputClass::SelfOwnedReal,
        false,
    );
    envelope.schema_revision = "voxproof-run-envelope-v0".to_string();

    assert_eq!(
        envelope.validate(),
        Err(RunEnvelopeValidationError::UnsupportedSchemaRevision {
            found: "voxproof-run-envelope-v0".to_string(),
            expected: RUN_ENVELOPE_SCHEMA.to_string(),
        })
    );
}

#[test]
fn valid_opaque_run_id_accepted() {
    let run_id = RunId::new("run-20260720-alpha").expect("accepted");
    assert_eq!(run_id.as_str(), "run-20260720-alpha");
}

#[test]
fn empty_run_id_rejected() {
    assert_eq!(RunId::new(""), Err(RunIdError::Empty));
}

#[test]
fn path_separators_in_run_id_rejected() {
    for value in [
        "/Users/example/private/input.srt",
        "C:\\Users\\example\\private\\input.srt",
        "../private/input.srt",
        "run/with/slash",
        "run\\with\\backslash",
    ] {
        assert!(
            RunId::new(value).is_err(),
            "expected rejection for {value:?}"
        );
    }
}

#[test]
fn same_transcript_revision_may_back_distinct_run_ids() {
    let first = sample_envelope(
        "run-first",
        CalibrationValidityMode::BlindReference,
        RunLifecycleState::Declared,
        InputClass::SelfOwnedReal,
        false,
    );
    let second = sample_envelope(
        "run-second",
        CalibrationValidityMode::BlindReference,
        RunLifecycleState::Declared,
        InputClass::SelfOwnedReal,
        false,
    );

    assert_eq!(
        first.input_identity.transcript_revision_id,
        second.input_identity.transcript_revision_id
    );
    assert_ne!(first.run_id, second.run_id);
    first.validate().expect("first valid");
    second.validate().expect("second valid");
}

#[test]
fn serialized_envelope_contains_no_input_path_field() {
    let envelope = sample_envelope(
        "run-alpha",
        CalibrationValidityMode::BlindReference,
        RunLifecycleState::Declared,
        InputClass::SelfOwnedReal,
        false,
    );
    let json = serde_json::to_string(&envelope).expect("serialize");
    assert!(!json.contains("input_path"));
    assert!(!json.contains("path"));
}

#[test]
fn evaluation_modes_round_trip_and_workflow_observation_is_explicit() {
    for calibration_validity in [
        CalibrationValidityMode::BlindReference,
        CalibrationValidityMode::DetectorAssisted,
    ] {
        for workflow_observation in [
            WorkflowObservationMode::Enabled,
            WorkflowObservationMode::Disabled,
        ] {
            let mut envelope = sample_envelope(
                "run-mode-check",
                calibration_validity,
                RunLifecycleState::Declared,
                InputClass::SyntheticProtocolFixture,
                false,
            );
            envelope.workflow_observation = workflow_observation;
            envelope.validate().expect("valid envelope");

            let json = serde_json::to_string(&envelope).expect("serialize");
            let restored: RunEnvelope = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(restored.calibration_validity, calibration_validity);
            assert_eq!(restored.workflow_observation, workflow_observation);
        }
    }
}

#[test]
fn detector_assisted_reference_sealed_combination_rejected() {
    let envelope = sample_envelope(
        "run-detector-assisted",
        CalibrationValidityMode::DetectorAssisted,
        RunLifecycleState::ReferenceSealed,
        InputClass::SyntheticProtocolFixture,
        false,
    );

    assert_eq!(
        envelope.validate(),
        Err(
            RunEnvelopeValidationError::ForbiddenCalibrationLifecycleCombination {
                calibration_validity: CalibrationValidityMode::DetectorAssisted,
                lifecycle_state: RunLifecycleState::ReferenceSealed,
            }
        )
    );
}

#[test]
fn synthetic_fixture_cannot_qualify_as_real_material_evidence() {
    let envelope = sample_envelope(
        "run-synthetic",
        CalibrationValidityMode::DetectorAssisted,
        RunLifecycleState::Declared,
        InputClass::SyntheticProtocolFixture,
        true,
    );

    assert_eq!(
        envelope.validate(),
        Err(RunEnvelopeValidationError::SyntheticCannotQualifyAsRealMaterial)
    );
}

#[test]
fn real_input_class_does_not_authorize_execution_by_itself() {
    let envelope = sample_envelope(
        "run-real-class-only",
        CalibrationValidityMode::BlindReference,
        RunLifecycleState::Declared,
        InputClass::ExplicitPermissionReal,
        false,
    );

    envelope.validate().expect("qualification posture only");
    let json = serde_json::to_string(&envelope).expect("serialize");
    assert!(!json.contains("execution_authorized"));
}

#[test]
fn qualification_fields_survive_round_trip() {
    let envelope = sample_envelope(
        "run-real",
        CalibrationValidityMode::BlindReference,
        RunLifecycleState::Declared,
        InputClass::ExplicitPermissionReal,
        true,
    );
    let json = serde_json::to_string(&envelope).expect("serialize");
    let restored: RunEnvelope = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.input_class, InputClass::ExplicitPermissionReal);
    assert!(restored.qualifies_as_real_material_evidence);
}

#[test]
fn valid_blind_reference_lifecycle_transition_sequence_accepted() {
    let sequence = [
        RunLifecycleState::Declared,
        RunLifecycleState::ReferencePreparation,
        RunLifecycleState::ReferenceSealed,
        RunLifecycleState::DetectorExecution,
        RunLifecycleState::AssistedReview,
        RunLifecycleState::Finalized,
    ];

    for window in sequence.windows(2) {
        RunEnvelope::validate_transition(
            window[0],
            window[1],
            CalibrationValidityMode::BlindReference,
        )
        .expect("legal blind-reference transition");
    }
}

#[test]
fn valid_detector_assisted_lifecycle_transition_sequence_accepted() {
    let sequence = [
        RunLifecycleState::Declared,
        RunLifecycleState::DetectorExecution,
        RunLifecycleState::AssistedReview,
        RunLifecycleState::Finalized,
    ];

    for window in sequence.windows(2) {
        RunEnvelope::validate_transition(
            window[0],
            window[1],
            CalibrationValidityMode::DetectorAssisted,
        )
        .expect("legal detector-assisted transition");
    }
}

#[test]
fn invalid_skipped_phase_transition_rejected_for_blind_reference() {
    let error = RunEnvelope::validate_transition(
        RunLifecycleState::Declared,
        RunLifecycleState::DetectorExecution,
        CalibrationValidityMode::BlindReference,
    )
    .expect_err("must reject skipped phases");

    assert_eq!(
        error,
        RunEnvelopeValidationError::IllegalLifecycleTransition {
            from: RunLifecycleState::Declared,
            to: RunLifecycleState::DetectorExecution,
        }
    );
}

#[test]
fn finalized_state_cannot_reopen() {
    assert!(
        !RunLifecycleState::Finalized.can_transition_to_for_calibration(
            RunLifecycleState::Declared,
            CalibrationValidityMode::BlindReference,
        )
    );
    assert!(
        !RunLifecycleState::Finalized.can_transition_to_for_calibration(
            RunLifecycleState::AssistedReview,
            CalibrationValidityMode::DetectorAssisted,
        )
    );
}

#[test]
fn invalidated_state_cannot_resume() {
    assert!(
        !RunLifecycleState::Invalidated.can_transition_to_for_calibration(
            RunLifecycleState::Declared,
            CalibrationValidityMode::BlindReference,
        )
    );
    assert!(
        !RunLifecycleState::Invalidated.can_transition_to_for_calibration(
            RunLifecycleState::DetectorExecution,
            CalibrationValidityMode::DetectorAssisted,
        )
    );
}

#[test]
fn any_active_state_may_invalidate() {
    for state in [
        RunLifecycleState::Declared,
        RunLifecycleState::ReferencePreparation,
        RunLifecycleState::ReferenceSealed,
        RunLifecycleState::DetectorExecution,
        RunLifecycleState::AssistedReview,
    ] {
        for calibration_validity in [
            CalibrationValidityMode::BlindReference,
            CalibrationValidityMode::DetectorAssisted,
        ] {
            if state.can_transition_to_for_calibration(
                RunLifecycleState::Invalidated,
                calibration_validity,
            ) {
                RunEnvelope::validate_transition(
                    state,
                    RunLifecycleState::Invalidated,
                    calibration_validity,
                )
                .expect("invalidation allowed");
            }
        }
    }
}

#[test]
fn forbidden_path_like_values_rejected_in_authoritative_ids() {
    for value in [
        "/Users/example/private/input.srt",
        "C:\\Users\\example\\private\\input.srt",
        "../private/input.srt",
    ] {
        assert!(RunId::new(value).is_err(), "run id must reject {value:?}");

        let envelope = RunEnvelope {
            schema_revision: RUN_ENVELOPE_SCHEMA.to_string(),
            run_id: RunId::new("run-safe-id").expect("valid run id"),
            input_identity: InputIdentityReference {
                transcript_revision_id: value.to_string(),
            },
            calibration_validity: CalibrationValidityMode::BlindReference,
            workflow_observation: WorkflowObservationMode::Disabled,
            input_class: InputClass::SyntheticProtocolFixture,
            qualifies_as_real_material_evidence: false,
            lifecycle_state: RunLifecycleState::Declared,
            expected_artifact_roles: vec![],
        };

        assert!(
            envelope.validate().is_err(),
            "transcript revision id must reject {value:?}"
        );
    }
}

#[test]
fn generated_run_id_is_valid_and_distinct_from_revision() {
    let generated = RunId::generate().expect("generated run id");
    assert!(generated.as_str().starts_with("run-"));
    assert_ne!(generated.as_str(), SAMPLE_REVISION);
    assert!(generated.as_str().len() <= 128);
}

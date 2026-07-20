use vox_proof::artifact_bundle::{
    ARTIFACT_BUNDLE_SCHEMA, ArtifactBindingContext, ArtifactBundle, ArtifactBundleAssessment,
    ArtifactBundleId, ArtifactBundleState, ArtifactBundleValidationError, ArtifactContentDigest,
    ArtifactDescriptor, ArtifactId, ArtifactSchemaIdentity,
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
const SAMPLE_DIGEST: &str =
    "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn binding_context(mode: CalibrationValidityMode) -> ArtifactBindingContext {
    ArtifactBindingContext {
        run_id: RunId::new("run-bundle").expect("run id"),
        input_identity: InputIdentityReference {
            transcript_revision_id: SAMPLE_REVISION.to_string(),
        },
        calibration_validity: mode,
        reference_seal_id: None,
        reference_coverage_id: None,
    }
}

fn schema_identity() -> ArtifactSchemaIdentity {
    ArtifactSchemaIdentity::new("voxproof-calibration-comparison-v0", "v0").expect("schema")
}

fn descriptor(
    context: &ArtifactBindingContext,
    role: ArtifactRole,
    artifact_id: &str,
) -> ArtifactDescriptor {
    ArtifactDescriptor {
        artifact_id: ArtifactId::new(artifact_id).expect("artifact id"),
        role,
        payload_schema: schema_identity(),
        content_digest: ArtifactContentDigest::new(SAMPLE_DIGEST).expect("digest"),
        byte_length: 128,
        binding_context: context.clone(),
    }
}

fn build_bundle(
    mode: CalibrationValidityMode,
    expected_roles: Vec<ArtifactRole>,
    artifacts: Vec<ArtifactDescriptor>,
) -> ArtifactBundle {
    let context = binding_context(mode);
    let assessment =
        ArtifactBundle::derive_assessment(&expected_roles, &artifacts, &context).expect("derive");

    ArtifactBundle {
        schema_revision: ARTIFACT_BUNDLE_SCHEMA.to_string(),
        bundle_id: ArtifactBundleId::new("bundle-test").expect("bundle id"),
        binding_context: context,
        expected_roles,
        artifacts,
        bundle_state: ArtifactBundleState::Draft,
        assessment,
    }
}

fn blind_envelope(roles: Vec<ArtifactRole>) -> RunEnvelope {
    RunEnvelope {
        schema_revision: RUN_ENVELOPE_SCHEMA.to_string(),
        run_id: RunId::new("run-bundle").expect("run id"),
        input_identity: InputIdentityReference {
            transcript_revision_id: SAMPLE_REVISION.to_string(),
        },
        calibration_validity: CalibrationValidityMode::BlindReference,
        workflow_observation: WorkflowObservationMode::Disabled,
        input_class: InputClass::SelfOwnedReal,
        qualifies_as_real_material_evidence: false,
        lifecycle_state: RunLifecycleState::ReferenceSealed,
        expected_artifact_roles: roles,
    }
}

fn detector_envelope(roles: Vec<ArtifactRole>) -> RunEnvelope {
    RunEnvelope {
        schema_revision: RUN_ENVELOPE_SCHEMA.to_string(),
        run_id: RunId::new("run-bundle").expect("run id"),
        input_identity: InputIdentityReference {
            transcript_revision_id: SAMPLE_REVISION.to_string(),
        },
        calibration_validity: CalibrationValidityMode::DetectorAssisted,
        workflow_observation: WorkflowObservationMode::Disabled,
        input_class: InputClass::SyntheticProtocolFixture,
        qualifies_as_real_material_evidence: false,
        lifecycle_state: RunLifecycleState::Declared,
        expected_artifact_roles: roles,
    }
}

fn blind_seal() -> ReferenceSeal {
    ReferenceSeal {
        schema_revision: REFERENCE_SEAL_SCHEMA.to_string(),
        seal_id: ReferenceSealId::new("seal-blind").expect("seal id"),
        run_id: RunId::new("run-bundle").expect("run id"),
        input_identity: InputIdentityReference {
            transcript_revision_id: SAMPLE_REVISION.to_string(),
        },
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

fn blind_coverage(seal: &ReferenceSeal) -> ReferenceCoverage {
    let expected = ExpectedCueUniverse {
        total_cues: 1,
        cue_ids: vec![CueReferenceId::new(1).expect("cue")],
    };
    let records = vec![CueReferenceCoverageRecord {
        cue_id: CueReferenceId::new(1).expect("cue"),
        disposition: ReferenceCueDisposition::NoTranscriptionError,
    }];
    let assessment =
        ReferenceCoverage::derive_assessment(&expected, &records).expect("derive coverage");

    ReferenceCoverage {
        schema_revision: REFERENCE_COVERAGE_SCHEMA.to_string(),
        coverage_id: ReferenceCoverageId::new("coverage-blind").expect("coverage id"),
        run_id: seal.run_id.clone(),
        input_identity: seal.input_identity.clone(),
        seal_id: seal.seal_id.clone(),
        coverage_purpose: ReferenceCoveragePurpose::PrimaryBlindCalibration,
        expected_universe: expected,
        records,
        coverage_state: ReferenceCoverageState::Complete,
        assessment,
    }
}

fn complete_bundle(
    expected_roles: Vec<ArtifactRole>,
    artifacts: Vec<ArtifactDescriptor>,
    context: ArtifactBindingContext,
) -> ArtifactBundle {
    let assessment =
        ArtifactBundle::derive_assessment(&expected_roles, &artifacts, &context).expect("derive");

    ArtifactBundle {
        schema_revision: ARTIFACT_BUNDLE_SCHEMA.to_string(),
        bundle_id: ArtifactBundleId::new("bundle-test").expect("bundle id"),
        binding_context: context,
        expected_roles,
        artifacts,
        bundle_state: ArtifactBundleState::Complete,
        assessment,
    }
}

#[test]
fn json_round_trip_retains_schema_and_enum_spellings() {
    let context = binding_context(CalibrationValidityMode::DetectorAssisted);
    let expected = vec![ArtifactRole::DetectorOutput];
    let artifacts = vec![descriptor(
        &context,
        ArtifactRole::DetectorOutput,
        "artifact-detector",
    )];
    let bundle = build_bundle(
        CalibrationValidityMode::DetectorAssisted,
        expected,
        artifacts,
    );

    let json = serde_json::to_string_pretty(&bundle).expect("serialize");
    assert!(json.contains(ARTIFACT_BUNDLE_SCHEMA));
    assert!(json.contains("\"detector_output\""));
    assert!(json.contains("\"inventory_complete\""));

    let restored: ArtifactBundle = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, bundle);
    restored.validate().expect("valid bundle");
}

#[test]
fn unknown_top_level_field_rejected() {
    let context = binding_context(CalibrationValidityMode::DetectorAssisted);
    let bundle = build_bundle(
        CalibrationValidityMode::DetectorAssisted,
        vec![ArtifactRole::DetectorOutput],
        vec![descriptor(
            &context,
            ArtifactRole::DetectorOutput,
            "artifact-detector",
        )],
    );
    let mut value = serde_json::to_value(&bundle).expect("value");
    value
        .as_object_mut()
        .expect("object")
        .insert("path".to_string(), serde_json::json!("/tmp/forbidden"));

    let error = serde_json::from_value::<ArtifactBundle>(value).expect_err("must fail");
    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn valid_digest_and_ids_accepted() {
    assert!(ArtifactBundleId::new("bundle-alpha").is_ok());
    assert!(ArtifactId::new("artifact-alpha").is_ok());
    assert!(ArtifactContentDigest::new(SAMPLE_DIGEST).is_ok());
}

#[test]
fn path_like_ids_and_invalid_digest_rejected() {
    for value in [
        "/Users/example/private/output.json",
        "C:\\Users\\example\\private\\output.json",
        "../private/output.json",
    ] {
        assert!(ArtifactBundleId::new(value).is_err());
        assert!(ArtifactId::new(value).is_err());
    }

    assert!(ArtifactContentDigest::new("sha256:abc").is_err());
    assert!(
        ArtifactContentDigest::new(
            "sha256:ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789"
        )
        .is_err()
    );
    assert!(
        ArtifactContentDigest::new(
            "md5:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        )
        .is_err()
    );
}

#[test]
fn same_content_digest_does_not_substitute_for_artifact_identity() {
    let context = binding_context(CalibrationValidityMode::DetectorAssisted);
    let first = descriptor(&context, ArtifactRole::DetectorOutput, "artifact-first");
    let second = descriptor(&context, ArtifactRole::ReviewLedger, "artifact-second");
    assert_eq!(first.content_digest, second.content_digest);
    assert_ne!(first.artifact_id, second.artifact_id);
}

#[test]
fn serialized_bundle_contains_no_path_field() {
    let context = binding_context(CalibrationValidityMode::DetectorAssisted);
    let bundle = build_bundle(
        CalibrationValidityMode::DetectorAssisted,
        vec![ArtifactRole::DetectorOutput],
        vec![descriptor(
            &context,
            ArtifactRole::DetectorOutput,
            "artifact-detector",
        )],
    );
    let json = serde_json::to_string(&bundle).expect("serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert!(value.get("path").is_none());
    assert!(value.get("filename").is_none());
}

#[test]
fn exact_expected_role_inventory_derives_complete() {
    let context = binding_context(CalibrationValidityMode::DetectorAssisted);
    let expected = vec![ArtifactRole::DetectorOutput, ArtifactRole::ReviewLedger];
    let artifacts = vec![
        descriptor(&context, ArtifactRole::DetectorOutput, "artifact-detector"),
        descriptor(&context, ArtifactRole::ReviewLedger, "artifact-ledger"),
    ];
    let assessment =
        ArtifactBundle::derive_assessment(&expected, &artifacts, &context).expect("derive");
    assert!(assessment.inventory_complete);
    assert!(assessment.context_consistent);
}

#[test]
fn missing_role_derives_incomplete_inventory() {
    let context = binding_context(CalibrationValidityMode::DetectorAssisted);
    let expected = vec![ArtifactRole::DetectorOutput, ArtifactRole::Metrics];
    let artifacts = vec![descriptor(
        &context,
        ArtifactRole::DetectorOutput,
        "artifact-detector",
    )];
    let assessment =
        ArtifactBundle::derive_assessment(&expected, &artifacts, &context).expect("derive");
    assert!(!assessment.inventory_complete);
    assert_eq!(assessment.missing_roles, vec![ArtifactRole::Metrics]);
}

#[test]
fn unexpected_role_derives_incomplete_inventory() {
    let context = binding_context(CalibrationValidityMode::DetectorAssisted);
    let expected = vec![ArtifactRole::DetectorOutput];
    let artifacts = vec![
        descriptor(&context, ArtifactRole::DetectorOutput, "artifact-detector"),
        descriptor(&context, ArtifactRole::Metrics, "artifact-metrics"),
    ];
    let assessment =
        ArtifactBundle::derive_assessment(&expected, &artifacts, &context).expect("derive");
    assert!(!assessment.inventory_complete);
    assert_eq!(assessment.unexpected_roles, vec![ArtifactRole::Metrics]);
}

#[test]
fn duplicate_descriptor_role_rejected() {
    let context = binding_context(CalibrationValidityMode::DetectorAssisted);
    let expected = vec![ArtifactRole::DetectorOutput];
    let artifacts = vec![
        descriptor(&context, ArtifactRole::DetectorOutput, "artifact-a"),
        descriptor(&context, ArtifactRole::DetectorOutput, "artifact-b"),
    ];
    let assessment =
        ArtifactBundle::derive_assessment(&expected, &artifacts, &context).expect("derive");
    assert!(!assessment.inventory_complete);
    assert_eq!(
        assessment.duplicate_roles,
        vec![ArtifactRole::DetectorOutput]
    );
}

#[test]
fn duplicate_artifact_id_rejected() {
    let context = binding_context(CalibrationValidityMode::DetectorAssisted);
    let expected = vec![ArtifactRole::DetectorOutput, ArtifactRole::ReviewLedger];
    let artifacts = vec![
        descriptor(&context, ArtifactRole::DetectorOutput, "artifact-same"),
        descriptor(&context, ArtifactRole::ReviewLedger, "artifact-same"),
    ];
    let assessment =
        ArtifactBundle::derive_assessment(&expected, &artifacts, &context).expect("derive");
    assert!(!assessment.inventory_complete);
    assert_eq!(assessment.duplicate_artifact_ids.len(), 1);
}

#[test]
fn duplicate_expected_role_rejected() {
    let context = binding_context(CalibrationValidityMode::DetectorAssisted);
    assert!(matches!(
        ArtifactBundle::derive_assessment(
            &[ArtifactRole::DetectorOutput, ArtifactRole::DetectorOutput],
            &[],
            &context,
        ),
        Err(ArtifactBundleValidationError::DuplicateExpectedRole { .. })
    ));
}

#[test]
fn envelope_expected_roles_must_match_bundle() {
    let context = binding_context(CalibrationValidityMode::DetectorAssisted);
    let bundle = build_bundle(
        CalibrationValidityMode::DetectorAssisted,
        vec![ArtifactRole::DetectorOutput],
        vec![descriptor(
            &context,
            ArtifactRole::DetectorOutput,
            "artifact-detector",
        )],
    );
    let envelope = detector_envelope(vec![ArtifactRole::ReviewLedger]);

    assert!(matches!(
        bundle.validate_against_envelope(&envelope),
        Err(ArtifactBundleValidationError::ExpectedRolesMismatch)
    ));
}

#[test]
fn context_mismatch_fails_consistency() {
    let context = binding_context(CalibrationValidityMode::DetectorAssisted);
    let mut mismatched = context.clone();
    mismatched.run_id = RunId::new("run-other").expect("run id");
    let artifacts = vec![descriptor(
        &mismatched,
        ArtifactRole::DetectorOutput,
        "artifact-detector",
    )];
    let assessment =
        ArtifactBundle::derive_assessment(&[ArtifactRole::DetectorOutput], &artifacts, &context)
            .expect("derive");
    assert!(!assessment.context_consistent);
    assert_eq!(assessment.context_mismatch_artifact_ids.len(), 1);
}

#[test]
fn detector_assisted_context_cannot_carry_blind_reference_ids() {
    let mut context = binding_context(CalibrationValidityMode::DetectorAssisted);
    context.reference_seal_id = Some(ReferenceSealId::new("seal-blind").expect("seal id"));

    let bundle = ArtifactBundle {
        schema_revision: ARTIFACT_BUNDLE_SCHEMA.to_string(),
        bundle_id: ArtifactBundleId::new("bundle-test").expect("bundle id"),
        binding_context: context,
        expected_roles: vec![ArtifactRole::DetectorOutput],
        artifacts: vec![],
        bundle_state: ArtifactBundleState::Draft,
        assessment: ArtifactBundleAssessment {
            expected_roles: vec![ArtifactRole::DetectorOutput],
            present_roles: vec![],
            missing_roles: vec![ArtifactRole::DetectorOutput],
            unexpected_roles: vec![],
            duplicate_roles: vec![],
            duplicate_artifact_ids: vec![],
            context_mismatch_artifact_ids: vec![],
            inventory_complete: false,
            context_consistent: true,
        },
    };

    assert!(matches!(
        bundle.validate(),
        Err(ArtifactBundleValidationError::DetectorAssistedBlindReferenceContext)
    ));
}

#[test]
fn coverage_without_seal_in_context_rejected() {
    let mut context = binding_context(CalibrationValidityMode::BlindReference);
    context.reference_coverage_id = Some(ReferenceCoverageId::new("coverage-blind").expect("id"));

    let bundle = ArtifactBundle {
        schema_revision: ARTIFACT_BUNDLE_SCHEMA.to_string(),
        bundle_id: ArtifactBundleId::new("bundle-test").expect("bundle id"),
        binding_context: context,
        expected_roles: vec![ArtifactRole::ReferenceSeal],
        artifacts: vec![],
        bundle_state: ArtifactBundleState::Draft,
        assessment: ArtifactBundleAssessment {
            expected_roles: vec![ArtifactRole::ReferenceSeal],
            present_roles: vec![],
            missing_roles: vec![ArtifactRole::ReferenceSeal],
            unexpected_roles: vec![],
            duplicate_roles: vec![],
            duplicate_artifact_ids: vec![],
            context_mismatch_artifact_ids: vec![],
            inventory_complete: false,
            context_consistent: true,
        },
    };

    assert!(matches!(
        bundle.validate(),
        Err(ArtifactBundleValidationError::CoverageReferenceWithoutSeal)
    ));
}

#[test]
fn early_blind_draft_without_reference_context_validates() {
    let context = binding_context(CalibrationValidityMode::BlindReference);
    let expected = vec![ArtifactRole::InputAuthorization];
    let artifacts = vec![descriptor(
        &context,
        ArtifactRole::InputAuthorization,
        "artifact-auth",
    )];
    let bundle = build_bundle(CalibrationValidityMode::BlindReference, expected, artifacts);
    let envelope = blind_envelope(vec![ArtifactRole::InputAuthorization]);

    bundle
        .validate_with_reference_context(&envelope, None, None)
        .expect("early draft without references");
}

#[test]
fn reference_seal_role_requires_seal_context() {
    let bundle = build_bundle(
        CalibrationValidityMode::BlindReference,
        vec![ArtifactRole::ReferenceSeal],
        vec![],
    );
    let envelope = blind_envelope(vec![ArtifactRole::ReferenceSeal]);

    assert!(matches!(
        bundle.validate_with_reference_context(&envelope, None, None),
        Err(ArtifactBundleValidationError::ReferenceSealRequired { .. })
    ));
}

#[test]
fn seal_only_context_passes_when_role_requires_seal() {
    let seal = blind_seal();
    let mut context = binding_context(CalibrationValidityMode::BlindReference);
    context.reference_seal_id = Some(seal.seal_id.clone());
    let expected = vec![ArtifactRole::ReferenceSeal];
    let artifacts = vec![descriptor(
        &context,
        ArtifactRole::ReferenceSeal,
        "artifact-seal",
    )];
    let bundle = complete_bundle(expected.clone(), artifacts, context);
    let envelope = blind_envelope(expected);

    bundle
        .validate_with_reference_context(&envelope, Some(&seal), None)
        .expect("seal-only context");
}

#[test]
fn seal_and_coverage_context_passes_for_cue_review_role() {
    let seal = blind_seal();
    let coverage = blind_coverage(&seal);
    let mut context = binding_context(CalibrationValidityMode::BlindReference);
    context.reference_seal_id = Some(seal.seal_id.clone());
    context.reference_coverage_id = Some(coverage.coverage_id.clone());

    let expected = vec![
        ArtifactRole::ReferenceSeal,
        ArtifactRole::CueReviewCompletion,
    ];
    let artifacts = vec![
        descriptor(&context, ArtifactRole::ReferenceSeal, "artifact-seal"),
        descriptor(
            &context,
            ArtifactRole::CueReviewCompletion,
            "artifact-coverage",
        ),
    ];
    let bundle = complete_bundle(expected.clone(), artifacts, context);
    let envelope = blind_envelope(expected);

    bundle
        .validate_with_reference_context(&envelope, Some(&seal), Some(&coverage))
        .expect("seal and coverage context");
}

#[test]
fn structurally_complete_detector_assisted_bundle_validates() {
    let context = binding_context(CalibrationValidityMode::DetectorAssisted);
    let expected = vec![ArtifactRole::DetectorOutput];
    let artifacts = vec![descriptor(
        &context,
        ArtifactRole::DetectorOutput,
        "artifact-detector",
    )];
    let bundle = complete_bundle(expected.clone(), artifacts, context);
    let envelope = detector_envelope(expected);

    bundle.validate().expect("valid");
    bundle
        .validate_with_reference_context(&envelope, None, None)
        .expect("detector-assisted complete");
}

#[test]
fn complete_state_requires_inventory_and_context_consistency() {
    let context = binding_context(CalibrationValidityMode::DetectorAssisted);
    let mut bundle = build_bundle(
        CalibrationValidityMode::DetectorAssisted,
        vec![ArtifactRole::DetectorOutput, ArtifactRole::Metrics],
        vec![descriptor(
            &context,
            ArtifactRole::DetectorOutput,
            "artifact-detector",
        )],
    );
    bundle.bundle_state = ArtifactBundleState::Complete;

    assert!(matches!(
        bundle.validate(),
        Err(ArtifactBundleValidationError::BundleStateMismatch { .. })
    ));
}

#[test]
fn caller_cannot_force_assessment_fields() {
    let context = binding_context(CalibrationValidityMode::DetectorAssisted);
    let mut bundle = build_bundle(
        CalibrationValidityMode::DetectorAssisted,
        vec![ArtifactRole::DetectorOutput],
        vec![descriptor(
            &context,
            ArtifactRole::DetectorOutput,
            "artifact-detector",
        )],
    );
    bundle.assessment.inventory_complete = false;

    assert!(matches!(
        bundle.validate(),
        Err(ArtifactBundleValidationError::AssessmentMismatch { .. })
    ));
}

#[test]
fn unexpected_seal_supplied_when_context_has_none_fails() {
    let context = binding_context(CalibrationValidityMode::BlindReference);
    let bundle = build_bundle(
        CalibrationValidityMode::BlindReference,
        vec![ArtifactRole::InputAuthorization],
        vec![descriptor(
            &context,
            ArtifactRole::InputAuthorization,
            "artifact-auth",
        )],
    );
    let envelope = blind_envelope(vec![ArtifactRole::InputAuthorization]);
    let seal = blind_seal();

    assert!(matches!(
        bundle.validate_with_reference_context(&envelope, Some(&seal), None),
        Err(ArtifactBundleValidationError::UnexpectedSealSupplied)
    ));
}

#[test]
fn seal_context_requires_accepted_envelope_lifecycle() {
    let seal = blind_seal();
    let mut context = binding_context(CalibrationValidityMode::BlindReference);
    context.reference_seal_id = Some(seal.seal_id.clone());
    let expected = vec![ArtifactRole::ReferenceSeal];
    let artifacts = vec![descriptor(
        &context,
        ArtifactRole::ReferenceSeal,
        "artifact-seal",
    )];
    let bundle = complete_bundle(expected.clone(), artifacts, context);
    let envelope = RunEnvelope {
        schema_revision: RUN_ENVELOPE_SCHEMA.to_string(),
        run_id: RunId::new("run-bundle").expect("run id"),
        input_identity: InputIdentityReference {
            transcript_revision_id: SAMPLE_REVISION.to_string(),
        },
        calibration_validity: CalibrationValidityMode::BlindReference,
        workflow_observation: WorkflowObservationMode::Disabled,
        input_class: InputClass::SelfOwnedReal,
        qualifies_as_real_material_evidence: false,
        lifecycle_state: RunLifecycleState::Declared,
        expected_artifact_roles: expected,
    };

    assert!(matches!(
        bundle.validate_with_reference_context(&envelope, Some(&seal), None),
        Err(ArtifactBundleValidationError::SealValidation(_))
    ));
}

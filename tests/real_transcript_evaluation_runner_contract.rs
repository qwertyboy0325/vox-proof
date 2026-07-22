use vox_proof::detector_snapshot::{DetectorAnalysisIdentity, DetectorComponentIdentity};
use vox_proof::human_final_reference::{
    HUMAN_FINAL_REFERENCE_SCHEMA, HumanFinalReference, HumanFinalReferenceState, ReferenceClass,
    ReferenceErrorId, ReferenceErrorRecord, ReferenceReviewerIdentityClass, ReferenceSourceAnchor,
    VerificationBasis,
};
use vox_proof::input_authorization::{
    INPUT_AUTHORIZATION_SCHEMA, INPUT_AUTHORIZATION_SCOPE_POLICY, InputAuthorization,
    InputAuthorizationBasis, InputAuthorizationId, InputAuthorizationState,
};
use vox_proof::join_adjudication::OverlapAdjudicatorRole;
use vox_proof::real_transcript_evaluation_runner::{
    EnvelopePostureField, REAL_TRANSCRIPT_EVALUATION_RUNNER_POLICY,
    REAL_TRANSCRIPT_EVALUATION_RUNNER_REQUEST_SCHEMA, REAL_TRANSCRIPT_OVERLAP_AUTHORITY_POLICY,
    RealTranscriptEvaluationRunReadiness, RealTranscriptEvaluationRunRequest,
    RealTranscriptEvaluationRunnerContractError, canonical_real_evaluation_artifact_roles,
    real_evaluation_forbidden_overlap_authority_roles, real_evaluation_overlap_authority_roles,
    validate_real_transcript_evaluation_run_request,
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
    ArtifactRole, CalibrationValidityMode, InputClass, InputIdentityReference, RUN_ENVELOPE_SCHEMA,
    RunEnvelope, RunEnvelopeValidationError, RunId, RunLifecycleState, WorkflowObservationMode,
};

const SAMPLE_REVISION: &str =
    "rev:sha256-v1:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const OTHER_REVISION: &str =
    "rev:sha256-v1:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
const SAMPLE_REFERENCE_REVISION: &str = "ref-rev-real-001";
const SAMPLE_CUE_DIGEST: &str =
    "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const SAMPLE_SESSION_TERMS: &str =
    "session-terms:sha256-v1:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

const RUN_ID: &str = "run-real-contract-001";
const SEAL_ID: &str = "seal-real-001";
const COVERAGE_ID: &str = "coverage-real-001";

fn input_identity() -> InputIdentityReference {
    InputIdentityReference {
        transcript_revision_id: SAMPLE_REVISION.to_string(),
    }
}

fn artifact_roles() -> Vec<ArtifactRole> {
    canonical_real_evaluation_artifact_roles()
}

fn set_all_envelope_roles(
    request: &mut RealTranscriptEvaluationRunRequest,
    roles: Vec<ArtifactRole>,
) {
    for envelope in [
        &mut request.declared_envelope,
        &mut request.reference_preparation_envelope,
        &mut request.reference_sealed_envelope,
        &mut request.detector_execution_envelope,
        &mut request.assisted_review_transition_envelope,
        &mut request.finalized_envelope,
    ] {
        envelope.expected_artifact_roles = roles.clone();
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
        expected_artifact_roles: artifact_roles(),
    }
}

fn input_authorization_for(input_class: InputClass) -> InputAuthorization {
    let (basis, authorization_id) = match input_class {
        InputClass::SelfOwnedReal => (InputAuthorizationBasis::SelfOwned, "auth-self-owned-001"),
        InputClass::ExplicitPermissionReal => (
            InputAuthorizationBasis::ExplicitPermission,
            "auth-explicit-perm-001",
        ),
        InputClass::SyntheticProtocolFixture => {
            (InputAuthorizationBasis::SelfOwned, "auth-synthetic-001")
        }
    };

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
        seal_id: ReferenceSealId::new(SEAL_ID).expect("seal id"),
        run_id: RunId::new(RUN_ID).expect("run id"),
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

fn completion_record(
    cue_id: u32,
    segment_position: u32,
    disposition: ReferenceCueDisposition,
) -> CueReviewCompletionRecord {
    CueReviewCompletionRecord {
        cue_id: CueReferenceId::new(cue_id).expect("cue id"),
        segment_position,
        source_text_digest: CueSourceTextDigest::new(SAMPLE_CUE_DIGEST).expect("digest"),
        disposition,
        fully_reviewed: true,
        all_known_transcription_errors_enumerated: true,
        verification_source_used: VerificationBasis::AudioListened,
        reviewer_identity_class: ReferenceReviewerIdentityClass::OwnerBlindReviewer,
        completed_at_unix_ms: 1_700_000_000_000,
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
    .expect("derive assessment");

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

fn primary_coverage() -> ReferenceCoverage {
    let records = vec![
        completion_record(1, 0, ReferenceCueDisposition::NoTranscriptionError),
        completion_record(2, 1, ReferenceCueDisposition::TranscriptionError),
    ];
    let cue_ids: Vec<u32> = records.iter().map(|record| record.cue_id.value()).collect();
    let expected = ExpectedCueUniverse {
        total_cues: cue_ids.len() as u32,
        cue_ids: cue_ids
            .iter()
            .map(|id| CueReferenceId::new(*id).expect("cue id"))
            .collect(),
    };
    let assessment =
        ReferenceCoverage::derive_assessment(&expected, &records).expect("derive assessment");

    let mut coverage = ReferenceCoverage {
        schema_revision: REFERENCE_COVERAGE_SCHEMA.to_string(),
        coverage_id: ReferenceCoverageId::new(COVERAGE_ID).expect("coverage id"),
        run_id: RunId::new(RUN_ID).expect("run id"),
        input_identity: input_identity(),
        seal_id: ReferenceSealId::new(SEAL_ID).expect("seal id"),
        reference_revision: ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION)
            .expect("revision id"),
        coverage_purpose: ReferenceCoveragePurpose::PrimaryBlindCalibration,
        expected_universe: expected,
        records,
        coverage_state: ReferenceCoverageState::Draft,
        assessment,
    };

    let human_reference = human_reference_for_coverage(&coverage);
    coverage.assessment.total_eligible_transcription_errors = human_reference
        .assessment
        .recall_eligible_transcription_error_count;
    coverage.coverage_state = ReferenceCoverageState::Complete;
    coverage
}

fn detector_analysis_identity() -> DetectorAnalysisIdentity {
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

fn valid_request(input_class: InputClass) -> RealTranscriptEvaluationRunRequest {
    RealTranscriptEvaluationRunRequest {
        schema_revision: REAL_TRANSCRIPT_EVALUATION_RUNNER_REQUEST_SCHEMA.to_string(),
        runner_policy_revision: REAL_TRANSCRIPT_EVALUATION_RUNNER_POLICY.to_string(),
        overlap_authority_policy_revision: REAL_TRANSCRIPT_OVERLAP_AUTHORITY_POLICY.to_string(),
        input_authorization: input_authorization_for(input_class),
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
        reference_seal: blind_seal(),
        reference_coverage: primary_coverage(),
        human_final_reference: human_reference_for_coverage(&primary_coverage()),
        detector_analysis_identity: detector_analysis_identity(),
        expected_artifact_roles: artifact_roles(),
    }
}

#[test]
fn self_owned_real_fixture_yields_ready_plan() {
    let plan =
        validate_real_transcript_evaluation_run_request(&valid_request(InputClass::SelfOwnedReal))
            .expect("ready plan");

    assert_eq!(
        plan.readiness,
        RealTranscriptEvaluationRunReadiness::ReadyForDetectorExecution
    );
    assert_eq!(plan.input_class, InputClass::SelfOwnedReal);
    assert_eq!(plan.expected_artifact_roles, artifact_roles());
}

#[test]
fn explicit_permission_real_fixture_yields_ready_plan() {
    let plan = validate_real_transcript_evaluation_run_request(&valid_request(
        InputClass::ExplicitPermissionReal,
    ))
    .expect("ready plan");

    assert_eq!(plan.input_class, InputClass::ExplicitPermissionReal);
}

#[test]
fn repeated_validation_produces_equal_plans() {
    let request = valid_request(InputClass::SelfOwnedReal);
    let first = validate_real_transcript_evaluation_run_request(&request).expect("first");
    let second = validate_real_transcript_evaluation_run_request(&request).expect("second");
    assert_eq!(first, second);
}

#[test]
fn withdrawn_authorization_rejected() {
    let mut request = valid_request(InputClass::SelfOwnedReal);
    request.input_authorization.state = InputAuthorizationState::Withdrawn;

    assert!(matches!(
        validate_real_transcript_evaluation_run_request(&request),
        Err(RealTranscriptEvaluationRunnerContractError::InputAuthorizationNotConfirmed)
    ));
}

#[test]
fn authorization_run_mismatch_rejected() {
    let mut request = valid_request(InputClass::SelfOwnedReal);
    request.input_authorization.run_id = RunId::new("run-other").expect("run id");

    assert!(matches!(
        validate_real_transcript_evaluation_run_request(&request),
        Err(RealTranscriptEvaluationRunnerContractError::InputAuthorizationRunMismatch)
    ));
}

#[test]
fn authorization_identity_mismatch_rejected() {
    let mut request = valid_request(InputClass::SelfOwnedReal);
    request
        .input_authorization
        .input_identity
        .transcript_revision_id = OTHER_REVISION.to_string();

    assert!(matches!(
        validate_real_transcript_evaluation_run_request(&request),
        Err(RealTranscriptEvaluationRunnerContractError::InputAuthorizationIdentityMismatch)
    ));
}

#[test]
fn authorization_class_mismatch_rejected() {
    let mut request = valid_request(InputClass::SelfOwnedReal);
    request.input_authorization.input_class = InputClass::ExplicitPermissionReal;
    request.input_authorization.authorization_basis = InputAuthorizationBasis::ExplicitPermission;

    assert!(matches!(
        validate_real_transcript_evaluation_run_request(&request),
        Err(RealTranscriptEvaluationRunnerContractError::InputAuthorizationClassMismatch)
    ));
}

#[test]
fn declared_to_detector_execution_bypass_rejected() {
    let mut request = valid_request(InputClass::SelfOwnedReal);
    request.reference_preparation_envelope = request.declared_envelope.clone();
    request.reference_sealed_envelope = request.declared_envelope.clone();
    request.detector_execution_envelope.lifecycle_state = RunLifecycleState::DetectorExecution;

    assert!(matches!(
        validate_real_transcript_evaluation_run_request(&request),
        Err(RealTranscriptEvaluationRunnerContractError::InvalidLifecycleEnvelope { .. })
            | Err(RealTranscriptEvaluationRunnerContractError::IllegalLifecycleTransition { .. })
    ));
}

#[test]
fn detector_assisted_posture_rejected() {
    let mut request = valid_request(InputClass::SelfOwnedReal);
    set_all_envelope_roles(&mut request, artifact_roles());
    for envelope in [
        &mut request.declared_envelope,
        &mut request.reference_preparation_envelope,
        &mut request.reference_sealed_envelope,
        &mut request.detector_execution_envelope,
        &mut request.assisted_review_transition_envelope,
        &mut request.finalized_envelope,
    ] {
        envelope.calibration_validity = CalibrationValidityMode::DetectorAssisted;
    }

    assert!(matches!(
        validate_real_transcript_evaluation_run_request(&request),
        Err(
            RealTranscriptEvaluationRunnerContractError::EnvelopeValidation(
                RunEnvelopeValidationError::ForbiddenCalibrationLifecycleCombination {
                    calibration_validity: CalibrationValidityMode::DetectorAssisted,
                    lifecycle_state: RunLifecycleState::ReferencePreparation,
                }
            )
        )
    ));
}

#[test]
fn synthetic_input_class_rejected() {
    let mut request = valid_request(InputClass::SelfOwnedReal);
    for envelope in [
        &mut request.declared_envelope,
        &mut request.reference_preparation_envelope,
        &mut request.reference_sealed_envelope,
        &mut request.detector_execution_envelope,
        &mut request.assisted_review_transition_envelope,
        &mut request.finalized_envelope,
    ] {
        envelope.input_class = InputClass::SyntheticProtocolFixture;
        envelope.qualifies_as_real_material_evidence = false;
    }

    assert!(matches!(
        validate_real_transcript_evaluation_run_request(&request),
        Err(RealTranscriptEvaluationRunnerContractError::UnsupportedRealInputClass { .. })
    ));
}

#[test]
fn qualifies_as_real_material_false_rejected() {
    let mut request = valid_request(InputClass::SelfOwnedReal);
    for envelope in [
        &mut request.declared_envelope,
        &mut request.reference_preparation_envelope,
        &mut request.reference_sealed_envelope,
        &mut request.detector_execution_envelope,
        &mut request.assisted_review_transition_envelope,
        &mut request.finalized_envelope,
    ] {
        envelope.qualifies_as_real_material_evidence = false;
    }

    assert!(matches!(
        validate_real_transcript_evaluation_run_request(&request),
        Err(RealTranscriptEvaluationRunnerContractError::RunnerNotQualifiedAsRealMaterial)
    ));
}

#[test]
fn cross_envelope_run_id_mismatch_reports_field_and_stage() {
    let mut request = valid_request(InputClass::SelfOwnedReal);
    request.detector_execution_envelope.run_id = RunId::new("run-other").expect("run id");

    assert!(matches!(
        validate_real_transcript_evaluation_run_request(&request),
        Err(
            RealTranscriptEvaluationRunnerContractError::EnvelopePostureMismatch {
                field: EnvelopePostureField::RunId,
                lifecycle_state: RunLifecycleState::DetectorExecution,
            }
        )
    ));
}

#[test]
fn cross_envelope_input_identity_mismatch_reports_field_and_stage() {
    let mut request = valid_request(InputClass::SelfOwnedReal);
    request
        .finalized_envelope
        .input_identity
        .transcript_revision_id = OTHER_REVISION.to_string();

    assert!(matches!(
        validate_real_transcript_evaluation_run_request(&request),
        Err(
            RealTranscriptEvaluationRunnerContractError::EnvelopePostureMismatch {
                field: EnvelopePostureField::InputIdentity,
                lifecycle_state: RunLifecycleState::Finalized,
            }
        )
    ));
}

#[test]
fn human_detector_assisted_reviewer_seal_rejected() {
    let mut request = valid_request(InputClass::SelfOwnedReal);
    request.reference_seal.producer_class = ReferenceProducerClass::HumanDetectorAssistedReviewer;
    let classification = request.reference_seal.derive_calibration_classification();
    request.reference_seal.calibration_classification = classification;
    request.reference_seal.calibration_validity_impact =
        ReferenceSeal::derive_calibration_validity_impact(classification);

    assert!(matches!(
        validate_real_transcript_evaluation_run_request(&request),
        Err(RealTranscriptEvaluationRunnerContractError::ReferenceSealNotBlindEligible)
            | Err(RealTranscriptEvaluationRunnerContractError::ReferenceSealValidationFailure(_))
    ));
}

#[test]
fn prior_detector_run_on_same_input_rejected() {
    let mut request = valid_request(InputClass::SelfOwnedReal);
    request.reference_seal.prior_detector_run_on_same_input = true;
    let classification = request.reference_seal.derive_calibration_classification();
    request.reference_seal.calibration_classification = classification;
    request.reference_seal.calibration_validity_impact =
        ReferenceSeal::derive_calibration_validity_impact(classification);

    assert!(matches!(
        validate_real_transcript_evaluation_run_request(&request),
        Err(RealTranscriptEvaluationRunnerContractError::ReferenceSealNotBlindEligible)
            | Err(RealTranscriptEvaluationRunnerContractError::ReferenceSealValidationFailure(_))
    ));
}

#[test]
fn diagnostic_coverage_purpose_rejected() {
    let mut request = valid_request(InputClass::SelfOwnedReal);
    request.reference_coverage.coverage_purpose = ReferenceCoveragePurpose::DiagnosticOnly;

    assert!(matches!(
        validate_real_transcript_evaluation_run_request(&request),
        Err(RealTranscriptEvaluationRunnerContractError::ReferenceCoverageNotPrimary)
    ));
}

#[test]
fn draft_human_final_reference_rejected() {
    let mut request = valid_request(InputClass::SelfOwnedReal);
    request.human_final_reference.state = HumanFinalReferenceState::Draft;

    assert!(matches!(
        validate_real_transcript_evaluation_run_request(&request),
        Err(RealTranscriptEvaluationRunnerContractError::HumanFinalReferenceNotSealed)
    ));
}

#[test]
fn detector_analysis_input_identity_mismatch_rejected() {
    let mut request = valid_request(InputClass::SelfOwnedReal);
    request
        .detector_analysis_identity
        .input_identity
        .transcript_revision_id = OTHER_REVISION.to_string();

    assert!(matches!(
        validate_real_transcript_evaluation_run_request(&request),
        Err(RealTranscriptEvaluationRunnerContractError::DetectorAnalysisIdentityInputMismatch)
    ));
}

#[test]
fn empty_detector_set_rejected() {
    let mut request = valid_request(InputClass::SelfOwnedReal);
    request.detector_analysis_identity.detector_set.clear();

    assert!(matches!(
        validate_real_transcript_evaluation_run_request(&request),
        Err(
            RealTranscriptEvaluationRunnerContractError::DetectorAnalysisIdentityValidationFailure(
                _
            )
        )
    ));
}

#[test]
fn missing_input_authorization_role_rejected() {
    let mut request = valid_request(InputClass::SelfOwnedReal);
    let mut roles = artifact_roles();
    roles.remove(0);
    request.expected_artifact_roles = roles.clone();
    set_all_envelope_roles(&mut request, roles);

    assert!(matches!(
        validate_real_transcript_evaluation_run_request(&request),
        Err(RealTranscriptEvaluationRunnerContractError::ExpectedArtifactInventoryMismatch)
    ));
}

#[test]
fn unexpected_review_ledger_role_rejected() {
    let mut request = valid_request(InputClass::SelfOwnedReal);
    let mut roles = artifact_roles();
    roles.push(ArtifactRole::ReviewLedger);
    request.expected_artifact_roles = roles.clone();
    set_all_envelope_roles(&mut request, roles);

    assert!(matches!(
        validate_real_transcript_evaluation_run_request(&request),
        Err(RealTranscriptEvaluationRunnerContractError::ExpectedArtifactInventoryMismatch)
    ));
}

#[test]
fn wrong_role_order_rejected() {
    let mut request = valid_request(InputClass::SelfOwnedReal);
    let mut roles = artifact_roles();
    roles.swap(0, 1);
    request.expected_artifact_roles = roles.clone();
    set_all_envelope_roles(&mut request, roles);

    assert!(matches!(
        validate_real_transcript_evaluation_run_request(&request),
        Err(RealTranscriptEvaluationRunnerContractError::ExpectedArtifactInventoryMismatch)
    ));
}

#[test]
fn synchronized_envelope_inventory_only_input_authorization_rejected() {
    let mut request = valid_request(InputClass::SelfOwnedReal);
    set_all_envelope_roles(&mut request, vec![ArtifactRole::InputAuthorization]);

    assert_eq!(
        validate_real_transcript_evaluation_run_request(&request),
        Err(RealTranscriptEvaluationRunnerContractError::RequestEnvelopeArtifactInventoryMismatch)
    );
}

#[test]
fn synchronized_envelope_inventory_with_review_ledger_rejected() {
    let mut request = valid_request(InputClass::SelfOwnedReal);
    let mut roles = artifact_roles();
    roles.push(ArtifactRole::ReviewLedger);
    set_all_envelope_roles(&mut request, roles);

    assert_eq!(
        validate_real_transcript_evaluation_run_request(&request),
        Err(RealTranscriptEvaluationRunnerContractError::RequestEnvelopeArtifactInventoryMismatch)
    );
}

#[test]
fn duplicate_metrics_role_rejected() {
    let mut request = valid_request(InputClass::SelfOwnedReal);
    let mut roles = artifact_roles();
    roles.push(ArtifactRole::Metrics);
    request.expected_artifact_roles = roles.clone();
    set_all_envelope_roles(&mut request, roles);

    assert_eq!(
        validate_real_transcript_evaluation_run_request(&request),
        Err(
            RealTranscriptEvaluationRunnerContractError::DuplicateExpectedArtifactRole {
                role: ArtifactRole::Metrics,
            }
        )
    );
}

#[test]
fn one_envelope_inventory_differs_from_request() {
    let mut request = valid_request(InputClass::SelfOwnedReal);
    request.finalized_envelope.expected_artifact_roles = vec![ArtifactRole::InputAuthorization];

    assert!(matches!(
        validate_real_transcript_evaluation_run_request(&request),
        Err(
            RealTranscriptEvaluationRunnerContractError::EnvelopePostureMismatch {
                field: EnvelopePostureField::ExpectedArtifactRoles,
                lifecycle_state: RunLifecycleState::Finalized,
            }
        )
    ));
}

#[test]
fn overlap_authority_policy_roles() {
    assert!(
        real_evaluation_overlap_authority_roles()
            .contains(&OverlapAdjudicatorRole::OwnerAdjudicator)
    );
    assert!(
        real_evaluation_overlap_authority_roles()
            .contains(&OverlapAdjudicatorRole::AuthorizedDomainAdjudicator)
    );
    assert!(
        real_evaluation_forbidden_overlap_authority_roles()
            .contains(&OverlapAdjudicatorRole::SyntheticFixtureAdjudicator)
    );
}

#[test]
fn acceptance_validates_assertions_not_external_truth() {
    // Successful validation proves only declared protocol readiness.
    let plan =
        validate_real_transcript_evaluation_run_request(&valid_request(InputClass::SelfOwnedReal))
            .expect("contract fixture only");
    assert_eq!(
        plan.readiness,
        RealTranscriptEvaluationRunReadiness::ReadyForDetectorExecution
    );
}

#[test]
fn runner_module_has_no_filesystem_or_packet_surface() {
    let source = include_str!("../src/real_transcript_evaluation_runner.rs");
    for forbidden in [
        "evaluation_artifact_packet",
        "evaluation_artifact_packet_file",
        "SyntheticEvaluationHarness",
        "std::path::Path",
        "std::fs",
        "tokio",
    ] {
        assert!(
            !source.contains(forbidden),
            "forbidden surface {forbidden} found in runner contract module"
        );
    }
}

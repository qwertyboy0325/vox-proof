use vox_proof::artifact_bundle::ArtifactId;
use vox_proof::candidate::{DetectionKind, Evidence, SessionTermEntry};
use vox_proof::detector_snapshot::{
    DetectorAnalysisIdentity, DetectorComponentIdentity, DetectorProposalId,
    DetectorSnapshotRevisionId,
};
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
use vox_proof::pipeline::{CanonicalTermReviewRun, run_canonical_term_review};
use vox_proof::real_transcript_detector_snapshot_adapter::{
    DetectorSnapshotAdapterAnalysisField, REAL_TRANSCRIPT_DETECTOR_PROPOSAL_ID_POLICY,
    REAL_TRANSCRIPT_DETECTOR_SNAPSHOT_ADAPTER_POLICY,
    REAL_TRANSCRIPT_DETECTOR_SNAPSHOT_ADAPTER_REQUEST_SCHEMA,
    RealTranscriptDetectorSnapshotAdapterContractError,
    RealTranscriptDetectorSnapshotAdapterReadiness, RealTranscriptDetectorSnapshotAdapterRequest,
    ValidatedRealTranscriptDetectorSnapshotAdapterPlan,
    derive_detector_analysis_identity_from_canonical_run,
    validate_real_transcript_detector_snapshot_adapter_request,
};
use vox_proof::real_transcript_evaluation_runner::{
    REAL_TRANSCRIPT_EVALUATION_RUNNER_POLICY, REAL_TRANSCRIPT_EVALUATION_RUNNER_REQUEST_SCHEMA,
    REAL_TRANSCRIPT_OVERLAP_AUTHORITY_POLICY, RealTranscriptEvaluationRunRequest,
    RealTranscriptEvaluationRunnerContractError, canonical_real_evaluation_artifact_roles,
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
    CalibrationValidityMode, InputClass, InputIdentityReference, RUN_ENVELOPE_SCHEMA, RunEnvelope,
    RunId, RunLifecycleState, WorkflowObservationMode,
};
use vox_proof::srt::parse_srt;
use vox_proof::transcript::Transcript;

const RUN_ID: &str = "run-adapter-contract-001";
const SEAL_ID: &str = "seal-adapter-001";
const COVERAGE_ID: &str = "coverage-adapter-001";
const SAMPLE_REFERENCE_REVISION: &str = "ref-rev-adapter-001";
const SAMPLE_CUE_DIGEST: &str =
    "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const FROZEN_AT_MS: u64 = 1_700_000_000_000;

fn input_identity_for(transcript: &Transcript) -> InputIdentityReference {
    InputIdentityReference {
        transcript_revision_id: transcript.revision_id().to_tagged_string(),
    }
}

fn detector_analysis_identity_for_transcript(
    transcript: &Transcript,
    canonical_run: &CanonicalTermReviewRun,
) -> DetectorAnalysisIdentity {
    let mut identity =
        derive_detector_analysis_identity_from_canonical_run(canonical_run).expect("identity");
    identity.input_identity = input_identity_for(transcript);
    identity
}

fn envelope_at(
    lifecycle_state: RunLifecycleState,
    input_class: InputClass,
    input_identity: &InputIdentityReference,
) -> RunEnvelope {
    RunEnvelope {
        schema_revision: RUN_ENVELOPE_SCHEMA.to_string(),
        run_id: RunId::new(RUN_ID).expect("run id"),
        input_identity: input_identity.clone(),
        calibration_validity: CalibrationValidityMode::BlindReference,
        workflow_observation: WorkflowObservationMode::Disabled,
        input_class,
        qualifies_as_real_material_evidence: true,
        lifecycle_state,
        expected_artifact_roles: canonical_real_evaluation_artifact_roles(),
    }
}

fn input_authorization_for(
    input_class: InputClass,
    input_identity: &InputIdentityReference,
) -> InputAuthorization {
    let (basis, authorization_id) = match input_class {
        InputClass::SelfOwnedReal => (InputAuthorizationBasis::SelfOwned, "auth-self-owned-adapt"),
        InputClass::ExplicitPermissionReal => (
            InputAuthorizationBasis::ExplicitPermission,
            "auth-explicit-adapt",
        ),
        InputClass::SyntheticProtocolFixture => {
            (InputAuthorizationBasis::SelfOwned, "auth-synthetic-adapt")
        }
    };

    InputAuthorization {
        schema_revision: INPUT_AUTHORIZATION_SCHEMA.to_string(),
        authorization_id: InputAuthorizationId::new(authorization_id).expect("authorization id"),
        run_id: RunId::new(RUN_ID).expect("run id"),
        input_identity: input_identity.clone(),
        input_class,
        authorization_basis: basis,
        scope_policy_revision: INPUT_AUTHORIZATION_SCOPE_POLICY.to_string(),
        state: InputAuthorizationState::Confirmed,
    }
}

fn completion_record(
    cue_id: u32,
    segment_position: u32,
    disposition: ReferenceCueDisposition,
    _input_identity: &InputIdentityReference,
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
        completed_at_unix_ms: FROZEN_AT_MS,
    }
}

fn primary_coverage(input_identity: &InputIdentityReference) -> ReferenceCoverage {
    let records = vec![
        completion_record(
            1,
            0,
            ReferenceCueDisposition::NoTranscriptionError,
            input_identity,
        ),
        completion_record(
            2,
            1,
            ReferenceCueDisposition::TranscriptionError,
            input_identity,
        ),
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
        input_identity: input_identity.clone(),
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
            reviewed_at_unix_ms: FROZEN_AT_MS,
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

fn blind_seal(input_identity: &InputIdentityReference) -> ReferenceSeal {
    ReferenceSeal {
        schema_revision: REFERENCE_SEAL_SCHEMA.to_string(),
        seal_id: ReferenceSealId::new(SEAL_ID).expect("seal id"),
        run_id: RunId::new(RUN_ID).expect("run id"),
        input_identity: input_identity.clone(),
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

fn aligned_run_request(
    transcript: &Transcript,
    canonical_run: &CanonicalTermReviewRun,
    input_class: InputClass,
) -> RealTranscriptEvaluationRunRequest {
    let input_identity = input_identity_for(transcript);
    let detector_analysis_identity =
        detector_analysis_identity_for_transcript(transcript, canonical_run);
    let coverage = primary_coverage(&input_identity);

    RealTranscriptEvaluationRunRequest {
        schema_revision: REAL_TRANSCRIPT_EVALUATION_RUNNER_REQUEST_SCHEMA.to_string(),
        runner_policy_revision: REAL_TRANSCRIPT_EVALUATION_RUNNER_POLICY.to_string(),
        overlap_authority_policy_revision: REAL_TRANSCRIPT_OVERLAP_AUTHORITY_POLICY.to_string(),
        input_authorization: input_authorization_for(input_class, &input_identity),
        declared_envelope: envelope_at(RunLifecycleState::Declared, input_class, &input_identity),
        reference_preparation_envelope: envelope_at(
            RunLifecycleState::ReferencePreparation,
            input_class,
            &input_identity,
        ),
        reference_sealed_envelope: envelope_at(
            RunLifecycleState::ReferenceSealed,
            input_class,
            &input_identity,
        ),
        detector_execution_envelope: envelope_at(
            RunLifecycleState::DetectorExecution,
            input_class,
            &input_identity,
        ),
        assisted_review_transition_envelope: envelope_at(
            RunLifecycleState::AssistedReview,
            input_class,
            &input_identity,
        ),
        finalized_envelope: envelope_at(RunLifecycleState::Finalized, input_class, &input_identity),
        reference_seal: blind_seal(&input_identity),
        reference_coverage: coverage.clone(),
        human_final_reference: human_reference_for_coverage(&coverage),
        detector_analysis_identity,
        expected_artifact_roles: canonical_real_evaluation_artifact_roles(),
    }
}

fn adapter_request_for(
    canonical_run: &CanonicalTermReviewRun,
) -> RealTranscriptDetectorSnapshotAdapterRequest {
    RealTranscriptDetectorSnapshotAdapterRequest {
        schema_revision: REAL_TRANSCRIPT_DETECTOR_SNAPSHOT_ADAPTER_REQUEST_SCHEMA.to_string(),
        adapter_policy_revision: REAL_TRANSCRIPT_DETECTOR_SNAPSHOT_ADAPTER_POLICY.to_string(),
        proposal_id_policy_revision: REAL_TRANSCRIPT_DETECTOR_PROPOSAL_ID_POLICY.to_string(),
        snapshot_revision: DetectorSnapshotRevisionId::new("snap-rev-adapter-001").expect("rev"),
        detector_output_artifact_id: ArtifactId::new("artifact-det-out-adapter").expect("artifact"),
        frozen_at_unix_ms: FROZEN_AT_MS,
        proposal_ids: canonical_run
            .review_cases()
            .iter()
            .enumerate()
            .map(|(index, _)| {
                DetectorProposalId::new(format!("det-prop-adapter-{index:03}")).expect("id")
            })
            .collect(),
    }
}

fn combined_canonical_fixture() -> (Transcript, CanonicalTermReviewRun) {
    let transcript = parse_srt(
        "1\n00:00:00,000 --> 00:00:01,000\nPostgre SQL\n\n2\n00:00:01,000 --> 00:00:02,000\nPostgres",
    )
    .expect("valid srt");
    let entries = vec![SessionTermEntry::new(
        "PostgreSQL",
        vec!["Postgres".to_string()],
        vec!["Postgre SQL".to_string()],
    )];
    let canonical_run = run_canonical_term_review(&transcript, &entries).expect("canonical run");
    (transcript, canonical_run)
}

fn zero_candidate_fixture() -> (Transcript, CanonicalTermReviewRun) {
    let transcript =
        parse_srt("1\n00:00:00,000 --> 00:00:01,000\nNo findings here").expect("valid srt");
    let canonical_run = run_canonical_term_review(&transcript, &[]).expect("zero findings");
    (transcript, canonical_run)
}

fn validate_fixture(
    input_class: InputClass,
) -> (
    Transcript,
    CanonicalTermReviewRun,
    RealTranscriptEvaluationRunRequest,
    RealTranscriptDetectorSnapshotAdapterRequest,
) {
    let (transcript, canonical_run) = combined_canonical_fixture();
    let run_request = aligned_run_request(&transcript, &canonical_run, input_class);
    let adapter_request = adapter_request_for(&canonical_run);
    (transcript, canonical_run, run_request, adapter_request)
}

#[test]
fn valid_combined_fixture_yields_ready_for_snapshot_materialization() {
    let (transcript, canonical_run, run_request, adapter_request) =
        validate_fixture(InputClass::SelfOwnedReal);

    let plan = validate_real_transcript_detector_snapshot_adapter_request(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
    )
    .expect("ready plan");

    assert_eq!(
        plan.readiness,
        RealTranscriptDetectorSnapshotAdapterReadiness::ReadyForSnapshotMaterialization
    );
    assert_eq!(plan.run_id.as_str(), RUN_ID);
    assert_eq!(plan.input_identity, input_identity_for(&transcript));
    assert_eq!(
        plan.calibration_validity,
        CalibrationValidityMode::BlindReference
    );
    assert_eq!(
        plan.detector_analysis_identity,
        detector_analysis_identity_for_transcript(&transcript, &canonical_run)
    );
    assert_eq!(plan.snapshot_revision.as_str(), "snap-rev-adapter-001");
    assert_eq!(
        plan.detector_output_artifact_id.as_str(),
        "artifact-det-out-adapter"
    );
    assert_eq!(plan.frozen_at_unix_ms, FROZEN_AT_MS);
    assert_eq!(plan.proposal_count, 3);
    assert_eq!(plan.proposal_ids.len(), 3);
    assert_eq!(plan.proposal_ids[0].as_str(), "det-prop-adapter-000");
    assert_eq!(canonical_run.review_cases().len(), 3);
    assert!(
        canonical_run
            .review_cases()
            .iter()
            .any(|case| case.candidate_span().kind() == DetectionKind::PhoneticSimilarity)
    );
}

#[test]
fn zero_candidate_fixture_is_valid() {
    let (transcript, canonical_run) = zero_candidate_fixture();
    let run_request = aligned_run_request(&transcript, &canonical_run, InputClass::SelfOwnedReal);
    let adapter_request = adapter_request_for(&canonical_run);

    let plan = validate_real_transcript_detector_snapshot_adapter_request(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
    )
    .expect("zero candidate ready");

    assert_eq!(plan.proposal_count, 0);
    assert!(plan.proposal_ids.is_empty());
    assert_eq!(canonical_run.review_cases().len(), 0);
}

#[test]
fn explicit_permission_real_fixture_succeeds() {
    let (transcript, canonical_run, run_request, adapter_request) =
        validate_fixture(InputClass::ExplicitPermissionReal);
    validate_real_transcript_detector_snapshot_adapter_request(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
    )
    .expect("explicit permission");
}

#[test]
fn detector_set_reorder_rejected() {
    let (transcript, canonical_run, mut run_request, adapter_request) =
        validate_fixture(InputClass::SelfOwnedReal);
    let derived = detector_analysis_identity_for_transcript(&transcript, &canonical_run);
    let mut reordered = derived.detector_set.clone();
    assert!(reordered.len() >= 2);
    reordered.swap(0, 1);
    run_request.detector_analysis_identity.detector_set = reordered;
    assert!(matches!(
        validate_real_transcript_detector_snapshot_adapter_request(
            &run_request,
            &adapter_request,
            &transcript,
            &canonical_run,
        ),
        Err(
            RealTranscriptDetectorSnapshotAdapterContractError::AnalysisIdentityMismatch {
                field: DetectorSnapshotAdapterAnalysisField::DetectorSet,
            }
        )
    ));
}

#[test]
fn glossary_alias_mapping_explicitly_accepted() {
    let (transcript, canonical_run, run_request, adapter_request) =
        validate_fixture(InputClass::SelfOwnedReal);
    let review_case = canonical_run
        .review_cases()
        .iter()
        .find(|case| matches!(case.candidate_span().evidence(), Evidence::GlossaryAlias(_)))
        .expect("glossary case");
    let candidate = review_case.candidate_span();
    let resolved = transcript
        .resolve(candidate.anchor())
        .expect("resolved surface");
    assert_eq!(candidate.kind(), DetectionKind::GlossaryAliasMatch);
    if let Evidence::GlossaryAlias(evidence) = candidate.evidence() {
        assert_eq!(evidence.matched_form, resolved);
    } else {
        panic!("expected glossary evidence");
    }
    let plan = validate_real_transcript_detector_snapshot_adapter_request(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
    )
    .expect("glossary mapping ready");
    assert_eq!(
        plan.readiness,
        RealTranscriptDetectorSnapshotAdapterReadiness::ReadyForSnapshotMaterialization
    );
}

#[test]
fn observed_error_form_mapping_explicitly_accepted() {
    let (transcript, canonical_run, run_request, adapter_request) =
        validate_fixture(InputClass::SelfOwnedReal);
    let review_case = canonical_run
        .review_cases()
        .iter()
        .find(|case| {
            matches!(
                case.candidate_span().evidence(),
                Evidence::ObservedErrorForm(_)
            )
        })
        .expect("observed error form case");
    let candidate = review_case.candidate_span();
    let resolved = transcript
        .resolve(candidate.anchor())
        .expect("resolved surface");
    assert_eq!(candidate.kind(), DetectionKind::GlossaryAliasMatch);
    if let Evidence::ObservedErrorForm(evidence) = candidate.evidence() {
        assert_eq!(evidence.matched_form, resolved);
    } else {
        panic!("expected observed error form evidence");
    }
    assert!(
        run_request
            .detector_analysis_identity
            .detector_set
            .iter()
            .any(|detector| {
                detector.id == candidate.provenance().detector_id()
                    && detector.version == candidate.provenance().detector_version()
            })
    );
    let plan = validate_real_transcript_detector_snapshot_adapter_request(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
    )
    .expect("observed error form mapping ready");
    assert_eq!(
        plan.readiness,
        RealTranscriptDetectorSnapshotAdapterReadiness::ReadyForSnapshotMaterialization
    );
}

#[test]
fn phonetic_similarity_mapping_explicitly_accepted() {
    let (transcript, canonical_run, run_request, adapter_request) =
        validate_fixture(InputClass::SelfOwnedReal);
    let review_case = canonical_run
        .review_cases()
        .iter()
        .find(|case| {
            matches!(
                case.candidate_span().evidence(),
                Evidence::PhoneticSimilarity(_)
            )
        })
        .expect("phonetic case");
    let candidate = review_case.candidate_span();
    let resolved = transcript
        .resolve(candidate.anchor())
        .expect("resolved surface");
    assert_eq!(candidate.kind(), DetectionKind::PhoneticSimilarity);
    if let Evidence::PhoneticSimilarity(evidence) = candidate.evidence() {
        assert_eq!(evidence.observed_surface, resolved);
    } else {
        panic!("expected phonetic evidence");
    }
    let plan = validate_real_transcript_detector_snapshot_adapter_request(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
    )
    .expect("phonetic mapping ready");
    assert_eq!(
        plan.readiness,
        RealTranscriptDetectorSnapshotAdapterReadiness::ReadyForSnapshotMaterialization
    );
}

#[test]
fn withdrawn_authorization_rejected_via_runner_wrapper() {
    let (transcript, canonical_run, mut run_request, adapter_request) =
        validate_fixture(InputClass::SelfOwnedReal);
    run_request.input_authorization.state = InputAuthorizationState::Withdrawn;
    assert!(matches!(
        validate_real_transcript_detector_snapshot_adapter_request(
            &run_request,
            &adapter_request,
            &transcript,
            &canonical_run,
        ),
        Err(
            RealTranscriptDetectorSnapshotAdapterContractError::RunnerContractValidationFailure(
                RealTranscriptEvaluationRunnerContractError::InputAuthorizationNotConfirmed
            )
        )
    ));
}

#[test]
fn synthetic_protocol_fixture_rejected_via_runner_wrapper() {
    let (transcript, canonical_run, mut run_request, adapter_request) =
        validate_fixture(InputClass::SelfOwnedReal);
    run_request.input_authorization.input_class = InputClass::SyntheticProtocolFixture;
    run_request.declared_envelope.input_class = InputClass::SyntheticProtocolFixture;
    assert!(matches!(
        validate_real_transcript_detector_snapshot_adapter_request(
            &run_request,
            &adapter_request,
            &transcript,
            &canonical_run,
        ),
        Err(RealTranscriptDetectorSnapshotAdapterContractError::RunnerContractValidationFailure(_))
    ));
}

#[test]
fn transcript_revision_mismatch_rejected() {
    let (_transcript, canonical_run, run_request, adapter_request) =
        validate_fixture(InputClass::SelfOwnedReal);
    let other = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nDifferent text").expect("other");
    assert!(matches!(
        validate_real_transcript_detector_snapshot_adapter_request(
            &run_request,
            &adapter_request,
            &other,
            &canonical_run,
        ),
        Err(RealTranscriptDetectorSnapshotAdapterContractError::TranscriptInputIdentityMismatch)
    ));
}

#[test]
fn analysis_source_revision_mismatch_rejected() {
    let (_transcript, canonical_run, run_request, adapter_request) =
        validate_fixture(InputClass::SelfOwnedReal);
    let other = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nOther transcript").expect("other");
    assert!(matches!(
        validate_real_transcript_detector_snapshot_adapter_request(
            &run_request,
            &adapter_request,
            &other,
            &canonical_run,
        ),
        Err(
            RealTranscriptDetectorSnapshotAdapterContractError::TranscriptInputIdentityMismatch
                | RealTranscriptDetectorSnapshotAdapterContractError::AnalysisSourceRevisionMismatch
        )
    ));
}

#[test]
fn session_terms_identity_mismatch_rejected() {
    let (transcript, canonical_run, mut run_request, adapter_request) =
        validate_fixture(InputClass::SelfOwnedReal);
    run_request
        .detector_analysis_identity
        .session_terms_identity =
        "session-terms:sha256-v1:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
            .to_string();
    assert!(matches!(
        validate_real_transcript_detector_snapshot_adapter_request(
            &run_request,
            &adapter_request,
            &transcript,
            &canonical_run,
        ),
        Err(
            RealTranscriptDetectorSnapshotAdapterContractError::AnalysisIdentityMismatch {
                field: DetectorSnapshotAdapterAnalysisField::SessionTermsIdentity,
            }
        )
    ));
}

#[test]
fn detector_set_mismatch_rejected() {
    let (transcript, canonical_run, mut run_request, adapter_request) =
        validate_fixture(InputClass::SelfOwnedReal);
    run_request
        .detector_analysis_identity
        .detector_set
        .push(DetectorComponentIdentity {
            id: "unused-extra-detector".to_string(),
            version: "0.1.0".to_string(),
        });
    assert!(matches!(
        validate_real_transcript_detector_snapshot_adapter_request(
            &run_request,
            &adapter_request,
            &transcript,
            &canonical_run,
        ),
        Err(
            RealTranscriptDetectorSnapshotAdapterContractError::AnalysisIdentityMismatch {
                field: DetectorSnapshotAdapterAnalysisField::DetectorSet,
            }
        )
    ));
}

#[test]
fn detector_config_mismatch_rejected() {
    let (transcript, canonical_run, mut run_request, adapter_request) =
        validate_fixture(InputClass::SelfOwnedReal);
    run_request.detector_analysis_identity.detector_config.id =
        "tampered-detector-config".to_string();
    assert!(matches!(
        validate_real_transcript_detector_snapshot_adapter_request(
            &run_request,
            &adapter_request,
            &transcript,
            &canonical_run,
        ),
        Err(
            RealTranscriptDetectorSnapshotAdapterContractError::AnalysisIdentityMismatch {
                field: DetectorSnapshotAdapterAnalysisField::DetectorConfig,
            }
        )
    ));
}

#[test]
fn algorithm_mismatch_rejected() {
    let (transcript, canonical_run, mut run_request, adapter_request) =
        validate_fixture(InputClass::SelfOwnedReal);
    run_request.detector_analysis_identity.algorithm.id = "tampered-algorithm".to_string();
    assert!(matches!(
        validate_real_transcript_detector_snapshot_adapter_request(
            &run_request,
            &adapter_request,
            &transcript,
            &canonical_run,
        ),
        Err(
            RealTranscriptDetectorSnapshotAdapterContractError::AnalysisIdentityMismatch {
                field: DetectorSnapshotAdapterAnalysisField::Algorithm,
            }
        )
    ));
}

#[test]
fn missing_proposal_id_rejected() {
    let (transcript, canonical_run, run_request, mut adapter_request) =
        validate_fixture(InputClass::SelfOwnedReal);
    adapter_request.proposal_ids.pop();
    assert!(matches!(
        validate_real_transcript_detector_snapshot_adapter_request(
            &run_request,
            &adapter_request,
            &transcript,
            &canonical_run,
        ),
        Err(
            RealTranscriptDetectorSnapshotAdapterContractError::ProposalIdCountMismatch {
                expected: 3,
                found: 2,
            }
        )
    ));
}

#[test]
fn extra_proposal_id_rejected() {
    let (transcript, canonical_run, run_request, mut adapter_request) =
        validate_fixture(InputClass::SelfOwnedReal);
    adapter_request
        .proposal_ids
        .push(DetectorProposalId::new("det-prop-extra").expect("id"));
    assert!(matches!(
        validate_real_transcript_detector_snapshot_adapter_request(
            &run_request,
            &adapter_request,
            &transcript,
            &canonical_run,
        ),
        Err(
            RealTranscriptDetectorSnapshotAdapterContractError::ProposalIdCountMismatch {
                expected: 3,
                found: 4,
            }
        )
    ));
}

#[test]
fn duplicate_proposal_id_rejected() {
    let (transcript, canonical_run, run_request, mut adapter_request) =
        validate_fixture(InputClass::SelfOwnedReal);
    adapter_request.proposal_ids[2] = adapter_request.proposal_ids[0].clone();
    assert!(matches!(
        validate_real_transcript_detector_snapshot_adapter_request(
            &run_request,
            &adapter_request,
            &transcript,
            &canonical_run,
        ),
        Err(RealTranscriptDetectorSnapshotAdapterContractError::DuplicateProposalId { .. })
    ));
}

#[test]
fn zero_candidates_with_non_empty_proposal_ids_rejected() {
    let (transcript, canonical_run) = zero_candidate_fixture();
    let run_request = aligned_run_request(&transcript, &canonical_run, InputClass::SelfOwnedReal);
    let mut adapter_request = adapter_request_for(&canonical_run);
    adapter_request
        .proposal_ids
        .push(DetectorProposalId::new("det-prop-unexpected").expect("id"));
    assert!(matches!(
        validate_real_transcript_detector_snapshot_adapter_request(
            &run_request,
            &adapter_request,
            &transcript,
            &canonical_run,
        ),
        Err(
            RealTranscriptDetectorSnapshotAdapterContractError::ProposalIdCountMismatch {
                expected: 0,
                found: 1,
            }
        )
    ));
}

#[test]
fn reordered_proposal_ids_preserved_in_plan() {
    let (transcript, canonical_run, run_request, mut adapter_request) =
        validate_fixture(InputClass::SelfOwnedReal);
    adapter_request.proposal_ids.reverse();
    let plan = validate_real_transcript_detector_snapshot_adapter_request(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
    )
    .expect("reordered ids preserved");
    assert_eq!(plan.proposal_ids[0].as_str(), "det-prop-adapter-002");
}

#[test]
fn canonical_review_case_local_indices_are_contiguous() {
    let (_, canonical_run) = combined_canonical_fixture();
    for (index, review_case) in canonical_run.review_cases().iter().enumerate() {
        assert_eq!(review_case.id().local_index(), index);
    }
}

#[test]
fn cue_id_derives_from_segment_index() {
    let (transcript, canonical_run) = combined_canonical_fixture();
    for review_case in canonical_run.review_cases() {
        let anchor = review_case.candidate_span().anchor();
        let segment = &transcript.segments()[anchor.segment_position()];
        let cue_index = segment.index();
        assert!(CueReferenceId::new(cue_index).is_ok());
        assert_eq!(cue_index, segment.index());
    }
}

#[test]
fn determinism_repeated_validation_is_equal() {
    let (transcript, canonical_run, run_request, adapter_request) =
        validate_fixture(InputClass::SelfOwnedReal);
    let first = validate_real_transcript_detector_snapshot_adapter_request(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
    )
    .expect("first");
    let second = validate_real_transcript_detector_snapshot_adapter_request(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
    )
    .expect("second");
    assert_eq!(first, second);
}

#[test]
fn source_inputs_remain_immutable() {
    let (transcript, canonical_run, run_request, adapter_request) =
        validate_fixture(InputClass::SelfOwnedReal);
    let before_transcript = transcript.segments()[0].text().to_string();
    let before_cases = canonical_run.review_cases().len();
    let _ = validate_real_transcript_detector_snapshot_adapter_request(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
    )
    .expect("validation");
    assert_eq!(transcript.segments()[0].text(), before_transcript);
    assert_eq!(canonical_run.review_cases().len(), before_cases);
}

#[test]
fn validated_plan_has_no_content_bearing_fields() {
    let source = include_str!("../src/real_transcript_detector_snapshot_adapter.rs");
    assert!(!source.contains("pub observed_surface:"));
    assert!(!source.contains("pub replacement_text:"));
    assert!(!source.contains("pub evidence:"));
    let (transcript, _canonical_run, _run_request, _adapter_request) =
        validate_fixture(InputClass::SelfOwnedReal);
    let _plan: ValidatedRealTranscriptDetectorSnapshotAdapterPlan =
        validate_real_transcript_detector_snapshot_adapter_request(
            &_run_request,
            &_adapter_request,
            &transcript,
            &_canonical_run,
        )
        .expect("plan");
    let _ = std::any::type_name::<ValidatedRealTranscriptDetectorSnapshotAdapterPlan>();
}

#[test]
fn production_module_has_no_forbidden_surfaces() {
    let source = include_str!("../src/real_transcript_detector_snapshot_adapter.rs");
    for forbidden in [
        "run_canonical_term_review(",
        "run_term_review(",
        "detect_glossary_matches(",
        "detect_observed_error_form_matches(",
        "detect_ascii_latin_phonetic_matches(",
        "DetectorProposalSnapshot {",
        "std::fs",
        "std::path::Path",
        "tokio",
        "SystemTime",
        "UNIX_EPOCH",
        "rand",
    ] {
        assert!(
            !source.contains(forbidden),
            "forbidden surface present: {forbidden}"
        );
    }
}

#[test]
fn zero_frozen_timestamp_rejected() {
    let (transcript, canonical_run, run_request, mut adapter_request) =
        validate_fixture(InputClass::SelfOwnedReal);
    adapter_request.frozen_at_unix_ms = 0;
    assert!(matches!(
        validate_real_transcript_detector_snapshot_adapter_request(
            &run_request,
            &adapter_request,
            &transcript,
            &canonical_run,
        ),
        Err(RealTranscriptDetectorSnapshotAdapterContractError::ZeroFrozenTimestamp)
    ));
}

#[test]
fn invalidated_authorization_rejected_via_runner_wrapper() {
    let (transcript, canonical_run, mut run_request, adapter_request) =
        validate_fixture(InputClass::SelfOwnedReal);
    run_request.input_authorization.state = InputAuthorizationState::Invalidated;
    assert!(matches!(
        validate_real_transcript_detector_snapshot_adapter_request(
            &run_request,
            &adapter_request,
            &transcript,
            &canonical_run,
        ),
        Err(
            RealTranscriptDetectorSnapshotAdapterContractError::RunnerContractValidationFailure(
                RealTranscriptEvaluationRunnerContractError::InputAuthorizationNotConfirmed
            )
        )
    ));
}

#[test]
fn detector_assisted_posture_rejected_via_runner_wrapper() {
    let (transcript, canonical_run, mut run_request, adapter_request) =
        validate_fixture(InputClass::SelfOwnedReal);
    run_request.detector_execution_envelope.calibration_validity =
        CalibrationValidityMode::DetectorAssisted;
    assert!(matches!(
        validate_real_transcript_detector_snapshot_adapter_request(
            &run_request,
            &adapter_request,
            &transcript,
            &canonical_run,
        ),
        Err(RealTranscriptDetectorSnapshotAdapterContractError::RunnerContractValidationFailure(_))
    ));
}

#[test]
fn qualifies_as_real_material_evidence_false_rejected_via_runner_wrapper() {
    let (transcript, canonical_run, mut run_request, adapter_request) =
        validate_fixture(InputClass::SelfOwnedReal);
    run_request
        .detector_execution_envelope
        .qualifies_as_real_material_evidence = false;
    assert!(matches!(
        validate_real_transcript_detector_snapshot_adapter_request(
            &run_request,
            &adapter_request,
            &transcript,
            &canonical_run,
        ),
        Err(RealTranscriptDetectorSnapshotAdapterContractError::RunnerContractValidationFailure(_))
    ));
}

#[test]
fn envelope_inventory_mismatch_rejected_via_runner_wrapper() {
    let (transcript, canonical_run, mut run_request, adapter_request) =
        validate_fixture(InputClass::SelfOwnedReal);
    run_request.declared_envelope.expected_artifact_roles.pop();
    assert!(matches!(
        validate_real_transcript_detector_snapshot_adapter_request(
            &run_request,
            &adapter_request,
            &transcript,
            &canonical_run,
        ),
        Err(RealTranscriptDetectorSnapshotAdapterContractError::RunnerContractValidationFailure(_))
    ));
}

#[test]
fn invalid_reference_readiness_rejected_via_runner_wrapper() {
    let (transcript, canonical_run, mut run_request, adapter_request) =
        validate_fixture(InputClass::SelfOwnedReal);
    run_request.reference_seal.seal_state = ReferenceSealState::Draft;
    assert!(matches!(
        validate_real_transcript_detector_snapshot_adapter_request(
            &run_request,
            &adapter_request,
            &transcript,
            &canonical_run,
        ),
        Err(RealTranscriptDetectorSnapshotAdapterContractError::RunnerContractValidationFailure(_))
    ));
    assert!(validate_real_transcript_evaluation_run_request(&run_request).is_err());
}

#[test]
fn invalid_proposal_id_values_fail_construction() {
    assert!(DetectorProposalId::new("").is_err());
    assert!(DetectorProposalId::new("   ").is_err());
}

#[test]
fn overlap_authority_roles_are_not_adapter_concern() {
    let _ = OverlapAdjudicatorRole::OwnerAdjudicator;
}

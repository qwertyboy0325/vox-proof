#[path = "support/real_transcript_evaluation_fixtures.rs"]
mod fixtures;

use fixtures::{
    RUN_ID, RealExecutionFixture, artifact_ids, dual_overlap_assisted_record,
    dual_overlap_explicit_permission_fixture, exact_only_self_owned_fixture,
    overlap_assisted_record, overlap_explicit_permission_fixture, revision_ids,
    zero_population_fixture,
};
use vox_proof::detector_reference_join::DetectorReferenceJoinState;
use vox_proof::detector_snapshot::{
    DetectorAnalysisIdentity, DetectorComponentIdentity, DetectorProposalSnapshotState,
};
use vox_proof::input_authorization::InputAuthorizationState;
use vox_proof::join_adjudication::{
    OverlapAdjudicationRecord, OverlapAdjudicationSetState, OverlapAdjudicatorRole,
};
use vox_proof::join_metric_aggregation::MetricAggregateValueState;
use vox_proof::join_metric_contribution::MetricContributionSetState;
use vox_proof::real_transcript_evaluation_execution::{
    RealTranscriptEvaluationCompletionStage, RealTranscriptEvaluationExecutionError,
    RealTranscriptEvaluationExecutionOutcome, execute_real_transcript_evaluation,
    verify_real_transcript_evaluation_completed_result,
};
use vox_proof::real_transcript_evaluation_runner::{
    RealTranscriptEvaluationRunnerContractError, validate_real_transcript_evaluation_run_request,
};
use vox_proof::run_manifest::{
    ArtifactRole, CalibrationValidityMode, InputClass, RunLifecycleState,
};

fn clone_fixture(fixture: &RealExecutionFixture) -> RealExecutionFixture {
    RealExecutionFixture {
        request: fixture.request.clone(),
        input: fixture.input.clone(),
    }
}

#[test]
fn exact_only_self_owned_completes_at_detector_execution() {
    let fixture = exact_only_self_owned_fixture();
    let outcome = execute_real_transcript_evaluation(&fixture.request, &fixture.input)
        .expect("exact-only completion");

    let RealTranscriptEvaluationExecutionOutcome::Completed(result) = outcome else {
        panic!("expected completed outcome");
    };

    assert_eq!(
        result.completion_stage,
        RealTranscriptEvaluationCompletionStage::DetectorExecution
    );
    assert_eq!(
        result.final_join.state,
        DetectorReferenceJoinState::Resolved
    );
    assert_eq!(
        result.final_contributions.state,
        MetricContributionSetState::Complete
    );
    assert!(
        result
            .final_contributions
            .eligibility
            .primary_metrics_allowed
    );
    assert!(result.final_aggregates.qualifies_as_primary_metric_evidence);
    assert_eq!(result.serialized_payloads.len(), 9);
    assert_eq!(
        result.serialized_payloads[0].role,
        ArtifactRole::InputAuthorization
    );
    assert_eq!(result.serialized_payloads[8].role, ArtifactRole::Metrics);
    assert!(result.final_bundle.assessment.inventory_complete);
    assert!(result.final_bundle.assessment.context_consistent);
    verify_real_transcript_evaluation_completed_result(&result).expect("verifier pass");
}

#[test]
fn overlap_without_assisted_review_returns_pending() {
    let fixture = overlap_explicit_permission_fixture(None);
    let outcome = execute_real_transcript_evaluation(&fixture.request, &fixture.input)
        .expect("pending outcome");

    let RealTranscriptEvaluationExecutionOutcome::RequiresHumanAdjudication(pending) = outcome
    else {
        panic!("expected pending outcome");
    };

    assert_eq!(
        pending.pending_join.state,
        DetectorReferenceJoinState::RequiresAdjudication
    );
    assert_eq!(
        pending.pending_contributions.state,
        MetricContributionSetState::PendingJoinResolution
    );
    assert!(
        !pending
            .pending_contributions
            .eligibility
            .primary_metrics_allowed
    );
    assert_eq!(pending.required_human_adjudication.overlap_pairs.len(), 1);
    assert_eq!(
        pending.required_human_adjudication.overlap_pairs[0]
            .detector_proposal_id
            .as_str(),
        "det-prop-overlap"
    );
}

#[test]
fn overlap_completes_with_owner_adjudicator() {
    let assisted = vec![overlap_assisted_record(
        OverlapAdjudicatorRole::OwnerAdjudicator,
    )];
    let fixture = overlap_explicit_permission_fixture(Some(assisted));
    let outcome = execute_real_transcript_evaluation(&fixture.request, &fixture.input)
        .expect("assisted completion");

    let RealTranscriptEvaluationExecutionOutcome::Completed(result) = outcome else {
        panic!("expected completed outcome");
    };
    assert_eq!(
        result.completion_stage,
        RealTranscriptEvaluationCompletionStage::AssistedReview
    );
    verify_real_transcript_evaluation_completed_result(&result).expect("verifier pass");
}

#[test]
fn overlap_completes_with_authorized_domain_adjudicator() {
    let assisted = vec![overlap_assisted_record(
        OverlapAdjudicatorRole::AuthorizedDomainAdjudicator,
    )];
    let fixture = overlap_explicit_permission_fixture(Some(assisted));
    let outcome = execute_real_transcript_evaluation(&fixture.request, &fixture.input)
        .expect("assisted completion");
    assert!(matches!(
        outcome,
        RealTranscriptEvaluationExecutionOutcome::Completed(_)
    ));
}

#[test]
fn synthetic_fixture_adjudicator_rejected() {
    let mut fixture = overlap_explicit_permission_fixture(Some(vec![overlap_assisted_record(
        OverlapAdjudicatorRole::OwnerAdjudicator,
    )]));
    let mut assisted = fixture
        .input
        .assisted_review_adjudication_set
        .take()
        .unwrap();
    assisted.records[0].adjudicator_role = OverlapAdjudicatorRole::SyntheticFixtureAdjudicator;
    fixture.input.assisted_review_adjudication_set = Some(assisted);

    let error = execute_real_transcript_evaluation(&fixture.request, &fixture.input)
        .expect_err("synthetic adjudicator forbidden");
    assert!(matches!(
        error,
        RealTranscriptEvaluationExecutionError::UnsupportedRealAdjudicatorRole {
            role: OverlapAdjudicatorRole::SyntheticFixtureAdjudicator,
            ..
        }
    ));
}

#[test]
fn exact_only_forbids_unnecessary_assisted_review() {
    let mut fixture = exact_only_self_owned_fixture();
    let assisted = overlap_explicit_permission_fixture(Some(vec![overlap_assisted_record(
        OverlapAdjudicatorRole::OwnerAdjudicator,
    )]));
    fixture.input.assisted_review_adjudication_set =
        assisted.input.assisted_review_adjudication_set.clone();

    let error = execute_real_transcript_evaluation(&fixture.request, &fixture.input)
        .expect_err("assisted review forbidden");
    assert!(matches!(
        error,
        RealTranscriptEvaluationExecutionError::AssistedReviewAdjudicationForbidden
    ));
}

#[test]
fn detector_execution_adjudication_must_be_empty() {
    let mut fixture = exact_only_self_owned_fixture();
    let mut set = fixture.input.detector_execution_adjudication_set.clone();
    set.records.push(overlap_assisted_record(
        OverlapAdjudicatorRole::OwnerAdjudicator,
    ));
    set.assessment =
        vox_proof::join_adjudication::OverlapAdjudicationSet::derive_assessment(&set.records);
    fixture.input.detector_execution_adjudication_set = set;

    let error = execute_real_transcript_evaluation(&fixture.request, &fixture.input)
        .expect_err("non-empty detector execution adjudication");
    assert!(matches!(
        error,
        RealTranscriptEvaluationExecutionError::DetectorExecutionAdjudicationNotEmpty
    ));
}

#[test]
fn duplicate_artifact_ids_rejected() {
    let mut fixture = exact_only_self_owned_fixture();
    fixture.input.artifact_ids.metrics = fixture.input.artifact_ids.evaluation_join.clone();

    let error = execute_real_transcript_evaluation(&fixture.request, &fixture.input)
        .expect_err("duplicate artifact ids");
    assert!(matches!(
        error,
        RealTranscriptEvaluationExecutionError::DuplicateArtifactId
    ));
}

#[test]
fn snapshot_draft_rejected() {
    let mut fixture = exact_only_self_owned_fixture();
    fixture.input.detector_snapshot.state = DetectorProposalSnapshotState::Draft;

    let error = execute_real_transcript_evaluation(&fixture.request, &fixture.input)
        .expect_err("draft snapshot");
    assert!(matches!(
        error,
        RealTranscriptEvaluationExecutionError::DetectorSnapshotValidationFailure(_)
    ));
}

#[test]
fn snapshot_zero_frozen_timestamp_rejected() {
    let mut fixture = exact_only_self_owned_fixture();
    fixture.input.detector_snapshot.frozen_at_unix_ms = 0;

    let error = execute_real_transcript_evaluation(&fixture.request, &fixture.input)
        .expect_err("zero frozen timestamp");
    assert!(matches!(
        error,
        RealTranscriptEvaluationExecutionError::DetectorSnapshotValidationFailure(_)
    ));
}

#[test]
fn determinism_exact_only() {
    let fixture = exact_only_self_owned_fixture();
    let first =
        execute_real_transcript_evaluation(&fixture.request, &fixture.input).expect("first run");
    let second =
        execute_real_transcript_evaluation(&fixture.request, &fixture.input).expect("second run");
    assert_eq!(first, second);
}

#[test]
fn determinism_pending() {
    let fixture = overlap_explicit_permission_fixture(None);
    let first =
        execute_real_transcript_evaluation(&fixture.request, &fixture.input).expect("first run");
    let second =
        execute_real_transcript_evaluation(&fixture.request, &fixture.input).expect("second run");
    assert_eq!(first, second);
}

#[test]
fn source_inputs_unchanged_after_completion() {
    let fixture = exact_only_self_owned_fixture();
    let before = clone_fixture(&fixture);
    let _ =
        execute_real_transcript_evaluation(&fixture.request, &fixture.input).expect("completion");
    assert_eq!(before.request, fixture.request);
    assert_eq!(before.input, fixture.input);
}

#[test]
fn source_inputs_unchanged_after_pending() {
    let fixture = overlap_explicit_permission_fixture(None);
    let before = clone_fixture(&fixture);
    let _ = execute_real_transcript_evaluation(&fixture.request, &fixture.input).expect("pending");
    assert_eq!(before.request, fixture.request);
    assert_eq!(before.input, fixture.input);
}

#[test]
fn payload_tamper_rejected_by_verifier() {
    let fixture = exact_only_self_owned_fixture();
    let RealTranscriptEvaluationExecutionOutcome::Completed(mut result) =
        execute_real_transcript_evaluation(&fixture.request, &fixture.input).expect("completion")
    else {
        panic!("expected completion");
    };

    result.serialized_payloads[0].payload_bytes.push(b'x');
    assert!(verify_real_transcript_evaluation_completed_result(&result).is_err());
}

#[test]
fn hash_synchronized_semantic_tamper_still_rejected() {
    use sha2::{Digest, Sha256};

    let fixture = exact_only_self_owned_fixture();
    let RealTranscriptEvaluationExecutionOutcome::Completed(mut result) =
        execute_real_transcript_evaluation(&fixture.request, &fixture.input).expect("completion")
    else {
        panic!("expected completion");
    };

    result.final_join.assessment.exact_match_count = result
        .final_join
        .assessment
        .exact_match_count
        .saturating_add(1);
    let join_bytes = serde_json::to_vec(&result.final_join).expect("reserialize join");
    let digest = {
        let hash = Sha256::digest(&join_bytes);
        let hex = hash.iter().map(|b| format!("{b:02x}")).collect::<String>();
        vox_proof::artifact_bundle::ArtifactContentDigest::new(format!("sha256:{hex}"))
            .expect("digest")
    };
    let byte_length = join_bytes.len() as u64;

    let join_index = result
        .serialized_payloads
        .iter()
        .position(|payload| payload.role == ArtifactRole::EvaluationJoin)
        .expect("join payload");
    result.serialized_payloads[join_index].payload_bytes = join_bytes;
    result.serialized_payloads[join_index].content_digest = digest.clone();
    result.serialized_payloads[join_index].byte_length = byte_length;

    let descriptor = result
        .final_bundle
        .artifacts
        .iter_mut()
        .find(|entry| entry.role == ArtifactRole::EvaluationJoin)
        .expect("join descriptor");
    descriptor.content_digest = digest;
    descriptor.byte_length = byte_length;

    assert!(verify_real_transcript_evaluation_completed_result(&result).is_err());
}

#[test]
fn withdrawn_authorization_rejected_via_runner_contract() {
    let mut fixture = exact_only_self_owned_fixture();
    fixture.request.input_authorization.state = InputAuthorizationState::Withdrawn;

    let error = execute_real_transcript_evaluation(&fixture.request, &fixture.input)
        .expect_err("withdrawn authorization");
    assert!(matches!(
        error,
        RealTranscriptEvaluationExecutionError::RunnerContractValidationFailure(
            RealTranscriptEvaluationRunnerContractError::InputAuthorizationNotConfirmed
        )
    ));
}

#[test]
fn five_primary_metrics_present_in_completed_fixture() {
    let fixture = exact_only_self_owned_fixture();
    let RealTranscriptEvaluationExecutionOutcome::Completed(result) =
        execute_real_transcript_evaluation(&fixture.request, &fixture.input).expect("completion")
    else {
        panic!("expected completion");
    };
    assert_eq!(result.final_aggregates.metrics.len(), 5);
}

#[test]
fn bundle_contains_input_authorization_role() {
    let fixture = exact_only_self_owned_fixture();
    let RealTranscriptEvaluationExecutionOutcome::Completed(result) =
        execute_real_transcript_evaluation(&fixture.request, &fixture.input).expect("completion")
    else {
        panic!("expected completion");
    };
    assert!(
        result
            .final_bundle
            .artifacts
            .iter()
            .any(|descriptor| descriptor.role == ArtifactRole::InputAuthorization)
    );
}

#[test]
fn revision_context_binding_mismatch_rejected() {
    let mut fixture = exact_only_self_owned_fixture();
    fixture
        .input
        .revision_ids
        .join_context
        .evaluation_join_artifact_id = fixture.input.artifact_ids.metrics.clone();

    let error = execute_real_transcript_evaluation(&fixture.request, &fixture.input)
        .expect_err("revision binding mismatch");
    assert!(matches!(
        error,
        RealTranscriptEvaluationExecutionError::RevisionArtifactBindingMismatch { .. }
    ));
}

#[test]
fn validated_request_is_first_authority_gate() {
    let fixture = exact_only_self_owned_fixture();
    validate_real_transcript_evaluation_run_request(&fixture.request).expect("request valid");
    execute_real_transcript_evaluation(&fixture.request, &fixture.input).expect("execution");
}

#[test]
fn production_module_has_no_forbidden_imports() {
    let source = include_str!("../src/real_transcript_evaluation_execution.rs");
    for forbidden in [
        "SyntheticEvaluationHarness",
        "evaluation_artifact_packet",
        "evaluation_artifact_packet_file",
        "std::fs",
        "std::path::Path",
        "tokio",
        "use crate::transcript",
        "SessionTermEntry",
        "run_canonical_term_review",
        "run_term_review",
    ] {
        assert!(
            !source.contains(forbidden),
            "forbidden surface present: {forbidden}"
        );
    }
}

#[test]
fn artifact_ids_and_revision_ids_are_accessible_for_binding_tests() {
    let ids = artifact_ids();
    let revisions = revision_ids();
    assert_ne!(ids.evaluation_join, ids.metrics);
    assert_eq!(
        revisions.join_context.evaluation_join_artifact_id,
        ids.evaluation_join
    );
    assert_eq!(ids.bundle.as_str(), "bundle-real-exec-001");
    assert_eq!(RUN_ID, "run-real-exec-001");

    let fixture = exact_only_self_owned_fixture();
    assert_eq!(
        fixture.request.declared_envelope.calibration_validity,
        CalibrationValidityMode::BlindReference
    );
    assert_eq!(
        fixture.request.declared_envelope.input_class,
        InputClass::SelfOwnedReal
    );
    let _ = RunLifecycleState::Finalized;
    let _ = DetectorAnalysisIdentity {
        input_identity: fixture.request.declared_envelope.input_identity.clone(),
        session_terms_identity: fixtures::SAMPLE_SESSION_TERMS.to_string(),
        detector_set: vec![DetectorComponentIdentity {
            id: "glossary-alias-match".to_string(),
            version: "0.1.0".to_string(),
        }],
        detector_config: DetectorComponentIdentity {
            id: "detector-config".to_string(),
            version: "0.1.0".to_string(),
        },
        algorithm: DetectorComponentIdentity {
            id: "algorithm-v1".to_string(),
            version: "0.1.0".to_string(),
        },
    };
}

#[test]
fn overlap_pending_adjudication_set_state_is_frozen_when_supplied() {
    let assisted = overlap_explicit_permission_fixture(Some(vec![overlap_assisted_record(
        OverlapAdjudicatorRole::OwnerAdjudicator,
    )]));
    assert_eq!(
        assisted
            .input
            .assisted_review_adjudication_set
            .as_ref()
            .expect("assisted set")
            .state,
        OverlapAdjudicationSetState::Frozen
    );
}

#[test]
fn dual_overlap_pending_requires_two_pairs() {
    let fixture = dual_overlap_explicit_permission_fixture(None);
    let outcome = execute_real_transcript_evaluation(&fixture.request, &fixture.input)
        .expect("pending outcome");
    let RealTranscriptEvaluationExecutionOutcome::RequiresHumanAdjudication(pending) = outcome
    else {
        panic!("expected pending outcome");
    };
    assert_eq!(pending.required_human_adjudication.overlap_pairs.len(), 2);
}

#[test]
fn partial_assisted_adjudication_rejected_for_dual_overlap() {
    let assisted = vec![dual_overlap_assisted_record(
        OverlapAdjudicatorRole::OwnerAdjudicator,
    )];
    let fixture = dual_overlap_explicit_permission_fixture(Some(assisted));

    let error = execute_real_transcript_evaluation(&fixture.request, &fixture.input)
        .expect_err("partial adjudication");
    assert!(
        matches!(
            error,
            RealTranscriptEvaluationExecutionError::AssistedReviewAdjudicationIncomplete
        ),
        "unexpected error: {error:?}"
    );
}

#[test]
fn zero_population_metrics_are_undefined_zero_denominator() {
    let fixture = zero_population_fixture();
    let RealTranscriptEvaluationExecutionOutcome::Completed(result) =
        execute_real_transcript_evaluation(&fixture.request, &fixture.input).expect("completion")
    else {
        panic!("expected completion");
    };

    assert!(result.final_aggregates.primary_metrics_allowed);
    assert!(result.final_aggregates.qualifies_as_primary_metric_evidence);
    assert!(result
        .final_aggregates
        .metrics
        .iter()
        .all(|metric| metric.value_state == MetricAggregateValueState::UndefinedZeroDenominator));
}

#[test]
fn determinism_assisted_review_completed() {
    let assisted = vec![overlap_assisted_record(
        OverlapAdjudicatorRole::OwnerAdjudicator,
    )];
    let fixture = overlap_explicit_permission_fixture(Some(assisted));
    let first =
        execute_real_transcript_evaluation(&fixture.request, &fixture.input).expect("first run");
    let second =
        execute_real_transcript_evaluation(&fixture.request, &fixture.input).expect("second run");
    assert_eq!(first, second);
}

#[test]
fn invalid_overlap_pair_rejected() {
    let fixture = overlap_explicit_permission_fixture(Some(vec![OverlapAdjudicationRecord {
        adjudication_id: vox_proof::join_adjudication::OverlapAdjudicationId::new("adj-invalid")
            .expect("id"),
        detector_proposal_id: vox_proof::detector_snapshot::DetectorProposalId::new(
            "det-prop-missing",
        )
        .expect("id"),
        reference_error_id: vox_proof::human_final_reference::ReferenceErrorId::new(
            "ref-err-missing",
        )
        .expect("id"),
        join_contract_revision: fixtures::JOIN_CONTRACT_REVISION.to_string(),
        adjudicator_role: OverlapAdjudicatorRole::OwnerAdjudicator,
        adjudication_result:
            vox_proof::join_adjudication::OverlapAdjudicationResult::SameErrorSameCorrection,
        adjudication_reason: "invalid pair".to_string(),
        adjudicated_at_unix_ms: fixtures::TIMESTAMP_MS,
    }]));

    let error = execute_real_transcript_evaluation(&fixture.request, &fixture.input)
        .expect_err("invalid pair");
    assert!(
        matches!(
            error,
            RealTranscriptEvaluationExecutionError::JoinValidationFailure(_)
                | RealTranscriptEvaluationExecutionError::AssistedReviewAdjudicationIncomplete
        ),
        "unexpected error: {error:?}"
    );
}

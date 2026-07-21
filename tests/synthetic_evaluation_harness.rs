#![allow(clippy::too_many_arguments)]

#[path = "synthetic_evaluation_harness_fixtures.rs"]
mod fixtures;

use fixtures::{
    FixtureMutation, exact_only_multi_disposition_fixture, mutate_fixture,
    overlap_pending_then_resolved_fixture, zero_population_fixture,
};
use vox_proof::artifact_bundle::ArtifactContentDigest;
use vox_proof::detector_reference_join::{
    DetectorReferenceJoinState, DetectorReferenceMatchDisposition,
};
use vox_proof::join_adjudication::OverlapAdjudicatorRole;
use vox_proof::join_metric_aggregation::{JoinMetricAggregationError, MetricAggregateValueState};
use vox_proof::join_metric_contribution::{
    MetricContributionReportClass, MetricContributionSetState, PrimaryMetricBlockingReason,
};
use vox_proof::reference_coverage::ReferenceCoveragePurpose;
use vox_proof::reference_seal::{ReferenceCalibrationValidity, ReferenceProducerClass};
use vox_proof::run_manifest::ArtifactRole;
use vox_proof::run_manifest::{InputClass, RunEnvelope, RunLifecycleState};
use vox_proof::synthetic_evaluation_harness::{
    SYNTHETIC_EVALUATION_HARNESS_REVISION, SYNTHETIC_PAYLOAD_DIGEST_POLICY,
    SYNTHETIC_PAYLOAD_SERIALIZATION_POLICY, SyntheticEvaluationCompletionStage,
    SyntheticEvaluationHarness, SyntheticEvaluationHarnessError, SyntheticEvaluationStage,
};

// --- Repository and source posture ---

#[test]
fn synthetic_fixture_is_accepted() {
    let fixture = exact_only_multi_disposition_fixture();
    let result = SyntheticEvaluationHarness::execute(&fixture);
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn real_input_class_rejected() {
    let mut fixture = exact_only_multi_disposition_fixture();
    mutate_fixture(&mut fixture, FixtureMutation::RealInputClass);
    assert!(matches!(
        SyntheticEvaluationHarness::execute(&fixture),
        Err(SyntheticEvaluationHarnessError::NonSyntheticInputClass)
    ));
}

#[test]
fn real_material_qualification_rejected() {
    let mut fixture = exact_only_multi_disposition_fixture();
    mutate_fixture(&mut fixture, FixtureMutation::RealMaterialQualification);
    assert!(matches!(
        SyntheticEvaluationHarness::execute(&fixture),
        Err(SyntheticEvaluationHarnessError::RealMaterialQualificationForbidden)
    ));
}

#[test]
fn non_synthetic_seal_producer_rejected() {
    let mut fixture = exact_only_multi_disposition_fixture();
    mutate_fixture(&mut fixture, FixtureMutation::HumanSealProducer);
    assert!(matches!(
        SyntheticEvaluationHarness::execute(&fixture),
        Err(SyntheticEvaluationHarnessError::SyntheticSealProducerMismatch)
    ));
}

#[test]
fn primary_coverage_purpose_rejected() {
    let mut fixture = exact_only_multi_disposition_fixture();
    mutate_fixture(&mut fixture, FixtureMutation::PrimaryCoveragePurpose);
    assert!(matches!(
        SyntheticEvaluationHarness::execute(&fixture),
        Err(SyntheticEvaluationHarnessError::SyntheticCoveragePurposeMismatch)
    ));
}

#[test]
fn non_synthetic_adjudicator_rejected() {
    let mut fixture = overlap_pending_then_resolved_fixture();
    mutate_fixture(&mut fixture, FixtureMutation::OwnerAdjudicator);
    assert!(matches!(
        SyntheticEvaluationHarness::execute(&fixture),
        Err(SyntheticEvaluationHarnessError::NonSyntheticAdjudicatorRole)
    ));
}

#[test]
fn mismatched_run_id_rejected() {
    let mut fixture = exact_only_multi_disposition_fixture();
    mutate_fixture(&mut fixture, FixtureMutation::MismatchedRunId);
    assert!(matches!(
        SyntheticEvaluationHarness::execute(&fixture),
        Err(SyntheticEvaluationHarnessError::SourceLineageMismatch { .. })
    ));
}

#[test]
fn mismatched_input_identity_rejected() {
    let mut fixture = exact_only_multi_disposition_fixture();
    mutate_fixture(&mut fixture, FixtureMutation::MismatchedInputIdentity);
    assert!(matches!(
        SyntheticEvaluationHarness::execute(&fixture),
        Err(SyntheticEvaluationHarnessError::SourceLineageMismatch { .. })
    ));
}

#[test]
fn mismatched_reference_revision_rejected() {
    let mut fixture = exact_only_multi_disposition_fixture();
    mutate_fixture(&mut fixture, FixtureMutation::MismatchedReferenceRevision);
    assert!(matches!(
        SyntheticEvaluationHarness::execute(&fixture),
        Err(SyntheticEvaluationHarnessError::SourceLineageMismatch { .. })
    ));
}

#[test]
fn source_artifacts_remain_unchanged_after_execution() {
    let fixture = exact_only_multi_disposition_fixture();
    let before_seal = fixture.reference_seal.clone();
    let before_coverage = fixture.reference_coverage.clone();
    let before_human = fixture.human_final_reference.clone();
    let before_snapshot = fixture.detector_snapshot.clone();
    let before_detector_adj = fixture.detector_execution_adjudication_set.clone();
    let before_assisted_adj = fixture.assisted_review_adjudication_set.clone();

    SyntheticEvaluationHarness::execute(&fixture).expect("execute");

    assert_eq!(fixture.reference_seal, before_seal);
    assert_eq!(fixture.reference_coverage, before_coverage);
    assert_eq!(fixture.human_final_reference, before_human);
    assert_eq!(fixture.detector_snapshot, before_snapshot);
    assert_eq!(
        fixture.detector_execution_adjudication_set,
        before_detector_adj
    );
    assert_eq!(
        fixture.assisted_review_adjudication_set,
        before_assisted_adj
    );
}

// --- Lifecycle ---

#[test]
fn complete_legal_transition_chain_validates() {
    for window in [
        (
            RunLifecycleState::Declared,
            RunLifecycleState::ReferencePreparation,
        ),
        (
            RunLifecycleState::ReferencePreparation,
            RunLifecycleState::ReferenceSealed,
        ),
        (
            RunLifecycleState::ReferenceSealed,
            RunLifecycleState::DetectorExecution,
        ),
        (
            RunLifecycleState::DetectorExecution,
            RunLifecycleState::AssistedReview,
        ),
        (
            RunLifecycleState::AssistedReview,
            RunLifecycleState::Finalized,
        ),
    ] {
        RunEnvelope::validate_transition(
            window.0,
            window.1,
            vox_proof::run_manifest::CalibrationValidityMode::BlindReference,
        )
        .expect("legal transition");
    }
}

#[test]
fn skipped_illegal_transition_rejected() {
    assert!(
        RunEnvelope::validate_transition(
            RunLifecycleState::Declared,
            RunLifecycleState::DetectorExecution,
            vox_proof::run_manifest::CalibrationValidityMode::BlindReference,
        )
        .is_err()
    );
}

#[test]
fn exact_only_completes_at_detector_execution() {
    let result = SyntheticEvaluationHarness::execute(&exact_only_multi_disposition_fixture())
        .expect("execute");
    assert_eq!(
        result.completion_stage,
        SyntheticEvaluationCompletionStage::DetectorExecution
    );
}

#[test]
fn overlap_resolution_requires_assisted_review() {
    let result = SyntheticEvaluationHarness::execute(&overlap_pending_then_resolved_fixture())
        .expect("execute");
    assert_eq!(
        result.completion_stage,
        SyntheticEvaluationCompletionStage::AssistedReview
    );
}

#[test]
fn historical_validation_at_finalized_succeeds() {
    let result = SyntheticEvaluationHarness::execute(&exact_only_multi_disposition_fixture())
        .expect("execute");
    assert_eq!(
        result.finalized_envelope.lifecycle_state,
        RunLifecycleState::Finalized
    );
    assert!(
        result
            .execution_trace
            .iter()
            .any(|record| record.stage == SyntheticEvaluationStage::HistoricalReplayValidated)
    );
}

// --- Exact-only execution ---

#[test]
fn exact_only_final_join_resolved() {
    let result = SyntheticEvaluationHarness::execute(&exact_only_multi_disposition_fixture())
        .expect("execute");
    assert_eq!(
        result.final_join.state,
        DetectorReferenceJoinState::Resolved
    );
}

#[test]
fn exact_only_final_contributions_complete() {
    let result = SyntheticEvaluationHarness::execute(&exact_only_multi_disposition_fixture())
        .expect("execute");
    assert_eq!(
        result.final_contributions.state,
        MetricContributionSetState::Complete
    );
}

#[test]
fn exact_only_final_aggregates_complete() {
    let result = SyntheticEvaluationHarness::execute(&exact_only_multi_disposition_fixture())
        .expect("execute");
    assert_eq!(
        result.final_aggregates.state,
        MetricAggregateSetState::Complete
    );
}

#[test]
fn exact_only_expected_dispositions_represented() {
    let result = SyntheticEvaluationHarness::execute(&exact_only_multi_disposition_fixture())
        .expect("execute");
    let dispositions: Vec<_> = result
        .final_join
        .reference_dispositions
        .iter()
        .map(|record| record.disposition)
        .collect();
    assert!(
        dispositions.contains(&DetectorReferenceMatchDisposition::ExactMatch)
            || result
                .final_join
                .detector_dispositions
                .iter()
                .any(|r| r.disposition == DetectorReferenceMatchDisposition::ExactMatch)
    );
    assert!(
        result
            .final_join
            .detector_dispositions
            .iter()
            .any(|r| r.disposition == DetectorReferenceMatchDisposition::DetectorWrongCorrection)
    );
    assert!(
        result
            .final_join
            .detector_dispositions
            .iter()
            .any(|r| r.disposition == DetectorReferenceMatchDisposition::DuplicateProposal)
    );
    assert!(
        result
            .final_join
            .detector_dispositions
            .iter()
            .any(|r| r.disposition == DetectorReferenceMatchDisposition::UnmatchedDetector)
    );
    assert!(
        dispositions.contains(&DetectorReferenceMatchDisposition::ExcludedFromErrorMetrics)
            || dispositions.contains(&DetectorReferenceMatchDisposition::UnmatchedReference)
    );
}

#[test]
fn exact_only_synthetic_report_class_and_non_primary() {
    let result = SyntheticEvaluationHarness::execute(&exact_only_multi_disposition_fixture())
        .expect("execute");
    assert_eq!(
        result.final_contributions.eligibility.report_class,
        MetricContributionReportClass::SyntheticProtocolValidation
    );
    assert!(
        !result
            .final_contributions
            .eligibility
            .primary_metrics_allowed
    );
    assert!(
        result
            .final_contributions
            .eligibility
            .blocking_reasons
            .contains(&PrimaryMetricBlockingReason::SyntheticProtocolOnly)
    );
    assert!(
        !result
            .final_contributions
            .qualifies_as_real_material_evidence
    );
}

#[test]
fn exact_only_expected_aggregate_counts() {
    let result = SyntheticEvaluationHarness::execute(&exact_only_multi_disposition_fixture())
        .expect("execute");
    let proposal_precision = &result.final_aggregates.metrics[0];
    assert_eq!(proposal_precision.denominator_count, 5);
    assert!(proposal_precision.numerator_count >= 1);
    let localization = &result.final_aggregates.metrics[1];
    assert_eq!(localization.denominator_count, 5);
    assert!(localization.numerator_count >= 1);
}

// --- Pending overlap ---

#[test]
fn pending_probe_join_requires_adjudication() {
    let pending =
        SyntheticEvaluationHarness::execute_pending_probe(&overlap_pending_then_resolved_fixture())
            .expect("pending probe");
    assert_eq!(
        pending.pending_join.state,
        DetectorReferenceJoinState::RequiresAdjudication
    );
}

#[test]
fn pending_probe_contributions_pending() {
    let pending =
        SyntheticEvaluationHarness::execute_pending_probe(&overlap_pending_then_resolved_fixture())
            .expect("pending probe");
    assert_eq!(
        pending.pending_contributions.state,
        MetricContributionSetState::PendingJoinResolution
    );
}

#[test]
fn pending_probe_aggregate_rejected_with_exact_error() {
    let pending =
        SyntheticEvaluationHarness::execute_pending_probe(&overlap_pending_then_resolved_fixture())
            .expect("pending probe");
    assert_eq!(
        pending.aggregation_error,
        JoinMetricAggregationError::PendingContributionRejected
    );
}

#[test]
fn pending_probe_cannot_be_promoted_via_execute_without_assisted_set() {
    let mut fixture = overlap_pending_then_resolved_fixture();
    fixture.assisted_review_adjudication_set = None;
    assert!(matches!(
        SyntheticEvaluationHarness::execute(&fixture),
        Err(SyntheticEvaluationHarnessError::AssistedReviewAdjudicationRequired)
    ));
}

// --- Resolved overlap ---

#[test]
fn resolved_overlap_final_join_one_to_one() {
    let result = SyntheticEvaluationHarness::execute(&overlap_pending_then_resolved_fixture())
        .expect("execute");
    assert_eq!(
        result.final_join.state,
        DetectorReferenceJoinState::Resolved
    );
    assert!(result.final_join.assessment.one_to_one_consistent);
}

#[test]
fn resolved_overlap_complete_contributions_and_aggregates() {
    let result = SyntheticEvaluationHarness::execute(&overlap_pending_then_resolved_fixture())
        .expect("execute");
    assert_eq!(
        result.final_contributions.state,
        MetricContributionSetState::Complete
    );
    assert_eq!(
        result.final_aggregates.state,
        MetricAggregateSetState::Complete
    );
    assert!(!result.final_aggregates.primary_metrics_allowed);
}

#[test]
fn resolved_overlap_uses_synthetic_fixture_adjudicator() {
    let fixture = overlap_pending_then_resolved_fixture();
    let records = fixture
        .assisted_review_adjudication_set
        .as_ref()
        .expect("assisted set")
        .records
        .clone();
    assert!(records.iter().all(
        |record| record.adjudicator_role == OverlapAdjudicatorRole::SyntheticFixtureAdjudicator
    ));
}

// --- Artifact bundle ---

#[test]
fn final_bundle_has_exact_eight_role_inventory() {
    let result = SyntheticEvaluationHarness::execute(&exact_only_multi_disposition_fixture())
        .expect("execute");
    assert_eq!(result.final_bundle.expected_roles.len(), 8);
    assert_eq!(result.serialized_payloads.len(), 8);
    for role in [
        ArtifactRole::ReferenceSeal,
        ArtifactRole::HumanFinalReference,
        ArtifactRole::CueReviewCompletion,
        ArtifactRole::DetectorOutput,
        ArtifactRole::EvaluationJoin,
        ArtifactRole::JoinAdjudication,
        ArtifactRole::MetricContributions,
        ArtifactRole::Metrics,
    ] {
        assert!(result.final_bundle.expected_roles.contains(&role));
    }
}

#[test]
fn payload_integrity_verification_passes() {
    let result = SyntheticEvaluationHarness::execute(&exact_only_multi_disposition_fixture())
        .expect("execute");
    SyntheticEvaluationHarness::verify_payload_integrity(&result).expect("integrity");
}

#[test]
fn one_byte_payload_mutation_fails_digest_verification() {
    let result = SyntheticEvaluationHarness::execute(&exact_only_multi_disposition_fixture())
        .expect("execute");
    let mut tampered = result.clone();
    tampered.serialized_payloads[0].payload_bytes.push(b'x');
    assert!(matches!(
        SyntheticEvaluationHarness::verify_payload_integrity(&tampered),
        Err(SyntheticEvaluationHarnessError::PayloadDigestMismatch { .. })
            | Err(SyntheticEvaluationHarnessError::PayloadLengthMismatch { .. })
    ));
}

// --- Serialization ---

#[test]
fn compact_json_bytes_are_deterministic_across_executions() {
    let first = SyntheticEvaluationHarness::execute(&exact_only_multi_disposition_fixture())
        .expect("first");
    let second = SyntheticEvaluationHarness::execute(&exact_only_multi_disposition_fixture())
        .expect("second");
    for (left, right) in first
        .serialized_payloads
        .iter()
        .zip(second.serialized_payloads.iter())
    {
        assert_eq!(left.payload_bytes, right.payload_bytes);
        assert_eq!(left.content_digest, right.content_digest);
        assert_eq!(left.byte_length, right.byte_length);
    }
}

#[test]
fn every_role_schema_mapping_is_correct() {
    let result = SyntheticEvaluationHarness::execute(&exact_only_multi_disposition_fixture())
        .expect("execute");
    for payload in &result.serialized_payloads {
        let descriptor = result
            .final_bundle
            .artifacts
            .iter()
            .find(|entry| entry.role == payload.role)
            .expect("descriptor");
        assert_eq!(descriptor.payload_schema, payload.payload_schema);
        assert_eq!(
            descriptor.payload_schema.schema_id,
            payload.payload_schema.schema_id
        );
    }
}

// --- Determinism ---

#[test]
fn verify_deterministic_replay_succeeds() {
    SyntheticEvaluationHarness::verify_deterministic_replay(&exact_only_multi_disposition_fixture())
        .expect("deterministic replay");
}

#[test]
fn verify_deterministic_replay_for_overlap_fixture() {
    SyntheticEvaluationHarness::verify_deterministic_replay(
        &overlap_pending_then_resolved_fixture(),
    )
    .expect("deterministic replay");
}

// --- Zero population ---

#[test]
fn zero_population_accepted() {
    SyntheticEvaluationHarness::execute(&zero_population_fixture()).expect("zero population");
}

#[test]
fn zero_population_all_aggregates_undefined() {
    let result =
        SyntheticEvaluationHarness::execute(&zero_population_fixture()).expect("zero population");
    assert!(result
        .final_aggregates
        .metrics
        .iter()
        .all(|metric| metric.value_state == MetricAggregateValueState::UndefinedZeroDenominator));
}

#[test]
fn zero_population_non_primary_and_replay_deterministic() {
    let result =
        SyntheticEvaluationHarness::execute(&zero_population_fixture()).expect("zero population");
    assert!(!result.final_aggregates.primary_metrics_allowed);
    SyntheticEvaluationHarness::verify_deterministic_replay(&zero_population_fixture())
        .expect("replay");
}

// --- Policy constants ---

#[test]
fn harness_revision_and_policy_constants() {
    assert_eq!(
        SYNTHETIC_EVALUATION_HARNESS_REVISION,
        "voxproof-synthetic-evaluation-harness-v1"
    );
    assert_eq!(
        SYNTHETIC_PAYLOAD_SERIALIZATION_POLICY,
        "serde-json-compact-utf8-v1"
    );
    assert_eq!(SYNTHETIC_PAYLOAD_DIGEST_POLICY, "sha256-payload-bytes-v1");
}

// --- Posture invariants on successful runs ---

#[test]
fn successful_run_retains_synthetic_posture_on_seal_and_coverage() {
    let result = SyntheticEvaluationHarness::execute(&exact_only_multi_disposition_fixture())
        .expect("execute");
    assert_eq!(
        result.detector_execution_envelope.input_class,
        InputClass::SyntheticProtocolFixture
    );
    assert_eq!(
        result.final_join.join_purpose,
        vox_proof::detector_reference_join::DetectorReferenceJoinPurpose::SyntheticProtocolValidation
    );
    assert_eq!(
        result.reference_seal_from_fixture_via_final_join_purpose(),
        vox_proof::detector_reference_join::DetectorReferenceJoinPurpose::SyntheticProtocolValidation
    );
}

trait FinalJoinPurposeCheck {
    fn reference_seal_from_fixture_via_final_join_purpose(
        &self,
    ) -> vox_proof::detector_reference_join::DetectorReferenceJoinPurpose;
}

impl FinalJoinPurposeCheck
    for vox_proof::synthetic_evaluation_harness::SyntheticEvaluationHarnessResult
{
    fn reference_seal_from_fixture_via_final_join_purpose(
        &self,
    ) -> vox_proof::detector_reference_join::DetectorReferenceJoinPurpose {
        self.final_join.join_purpose
    }
}

#[test]
fn envelope_expected_roles_match_bundle() {
    let result = SyntheticEvaluationHarness::execute(&exact_only_multi_disposition_fixture())
        .expect("execute");
    assert_eq!(
        result.finalized_envelope.expected_artifact_roles,
        result.final_bundle.expected_roles
    );
}

#[test]
fn bundle_does_not_include_self_descriptor() {
    let result = SyntheticEvaluationHarness::execute(&exact_only_multi_disposition_fixture())
        .expect("execute");
    assert!(
        !result
            .serialized_payloads
            .iter()
            .any(|payload| payload.role == ArtifactRole::Comparison)
    );
}

#[test]
fn synthetic_seal_posture_fields_on_fixture() {
    let fixture = exact_only_multi_disposition_fixture();
    assert_eq!(
        fixture.reference_seal.producer_class,
        ReferenceProducerClass::SyntheticFixtureGenerator
    );
    assert_eq!(
        fixture.reference_seal.calibration_classification,
        ReferenceCalibrationValidity::SyntheticProtocolOnly
    );
    assert_eq!(
        fixture.reference_coverage.coverage_purpose,
        ReferenceCoveragePurpose::SyntheticProtocolValidation
    );
}

// re-export for trait method above - use MetricAggregateSetState
use vox_proof::join_metric_aggregation::MetricAggregateSetState;

fn recompute_payload_descriptor(
    payload: &mut vox_proof::synthetic_evaluation_harness::SyntheticSerializedArtifact,
) {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(&payload.payload_bytes);
    let hex = hash
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    payload.content_digest = ArtifactContentDigest::new(format!("sha256:{hex}")).expect("digest");
    payload.byte_length = payload.payload_bytes.len() as u64;
}

fn sync_bundle_descriptor(
    result: &mut vox_proof::synthetic_evaluation_harness::SyntheticEvaluationHarnessResult,
    role: ArtifactRole,
) {
    let payload = result
        .serialized_payloads
        .iter()
        .find(|entry| entry.role == role)
        .expect("payload");
    let descriptor = result
        .final_bundle
        .artifacts
        .iter_mut()
        .find(|entry| entry.role == role)
        .expect("descriptor");
    descriptor.content_digest = payload.content_digest.clone();
    descriptor.byte_length = payload.byte_length;
}

// --- Typed payload round-trip ---

#[test]
fn all_eight_roles_typed_round_trip() {
    let roles = [
        ArtifactRole::ReferenceSeal,
        ArtifactRole::HumanFinalReference,
        ArtifactRole::CueReviewCompletion,
        ArtifactRole::DetectorOutput,
        ArtifactRole::EvaluationJoin,
        ArtifactRole::JoinAdjudication,
        ArtifactRole::MetricContributions,
        ArtifactRole::Metrics,
    ];

    for fixture in [
        exact_only_multi_disposition_fixture(),
        overlap_pending_then_resolved_fixture(),
        zero_population_fixture(),
    ] {
        let result = SyntheticEvaluationHarness::execute(&fixture).expect("execute");
        SyntheticEvaluationHarness::verify_typed_payload_round_trip(&result)
            .expect("typed round trip");
        for role in roles {
            assert!(
                result
                    .serialized_payloads
                    .iter()
                    .any(|payload| payload.role == role),
                "missing role {role:?} for fixture {}",
                fixture.fixture_id
            );
        }
    }
}

#[test]
fn decoded_historical_replay_exact_only() {
    let result = SyntheticEvaluationHarness::execute(&exact_only_multi_disposition_fixture())
        .expect("execute");
    SyntheticEvaluationHarness::verify_typed_payload_round_trip(&result).expect("decoded replay");
}

#[test]
fn decoded_historical_replay_overlap_resolved() {
    let result = SyntheticEvaluationHarness::execute(&overlap_pending_then_resolved_fixture())
        .expect("execute");
    SyntheticEvaluationHarness::verify_typed_payload_round_trip(&result).expect("decoded replay");
}

#[test]
fn decoded_historical_replay_zero_population() {
    let result = SyntheticEvaluationHarness::execute(&zero_population_fixture()).expect("execute");
    SyntheticEvaluationHarness::verify_typed_payload_round_trip(&result).expect("decoded replay");
}

#[test]
fn malformed_json_with_matching_digest_fails_typed_decode() {
    let mut result = SyntheticEvaluationHarness::execute(&exact_only_multi_disposition_fixture())
        .expect("execute");
    let role = ArtifactRole::ReferenceSeal;
    let payload = result
        .serialized_payloads
        .iter_mut()
        .find(|entry| entry.role == role)
        .expect("payload");
    payload.payload_bytes = br#"{"not":"valid seal"#.to_vec();
    recompute_payload_descriptor(payload);
    sync_bundle_descriptor(&mut result, role);

    SyntheticEvaluationHarness::verify_payload_integrity(&result).expect("digest integrity");
    assert!(matches!(
        SyntheticEvaluationHarness::verify_typed_payload_round_trip(&result),
        Err(
            SyntheticEvaluationHarnessError::PayloadDeserializationFailure {
                role: ArtifactRole::ReferenceSeal
            }
        )
    ));
}

#[test]
fn tampered_source_json_passes_digest_but_fails_typed_verification() {
    let mut result = SyntheticEvaluationHarness::execute(&exact_only_multi_disposition_fixture())
        .expect("execute");
    let role = ArtifactRole::ReferenceSeal;
    let payload = result
        .serialized_payloads
        .iter_mut()
        .find(|entry| entry.role == role)
        .expect("payload");
    let mut value: serde_json::Value =
        serde_json::from_slice(&payload.payload_bytes).expect("json");
    value["reference_revision"] = serde_json::Value::String("tampered-revision".to_string());
    payload.payload_bytes = serde_json::to_vec(&value).expect("serialize");
    recompute_payload_descriptor(payload);
    sync_bundle_descriptor(&mut result, role);

    SyntheticEvaluationHarness::verify_payload_integrity(&result).expect("digest integrity");
    assert!(matches!(
        SyntheticEvaluationHarness::verify_typed_payload_round_trip(&result),
        Err(SyntheticEvaluationHarnessError::TypedPayloadMismatch {
            role: ArtifactRole::ReferenceSeal
        }) | Err(
            SyntheticEvaluationHarnessError::DecodedPayloadValidationFailure {
                role: ArtifactRole::ReferenceSeal
            }
        ) | Err(SyntheticEvaluationHarnessError::DecodedHistoricalReplayValidationFailure { .. })
    ));
}

#[test]
fn tampered_derived_json_passes_digest_but_fails_typed_verification() {
    let mut result = SyntheticEvaluationHarness::execute(&exact_only_multi_disposition_fixture())
        .expect("execute");
    let role = ArtifactRole::EvaluationJoin;
    let payload = result
        .serialized_payloads
        .iter_mut()
        .find(|entry| entry.role == role)
        .expect("payload");
    let mut value: serde_json::Value =
        serde_json::from_slice(&payload.payload_bytes).expect("json");
    value["join_revision"] = serde_json::Value::String("tampered-join-revision".to_string());
    payload.payload_bytes = serde_json::to_vec(&value).expect("serialize");
    recompute_payload_descriptor(payload);
    sync_bundle_descriptor(&mut result, role);

    SyntheticEvaluationHarness::verify_payload_integrity(&result).expect("digest integrity");
    assert!(matches!(
        SyntheticEvaluationHarness::verify_typed_payload_round_trip(&result),
        Err(SyntheticEvaluationHarnessError::TypedPayloadMismatch {
            role: ArtifactRole::EvaluationJoin
        }) | Err(
            SyntheticEvaluationHarnessError::DecodedPayloadValidationFailure {
                role: ArtifactRole::EvaluationJoin
            }
        ) | Err(SyntheticEvaluationHarnessError::DecodedHistoricalReplayValidationFailure { .. })
    ));
}

#[test]
fn unknown_field_with_matching_digest_fails_typed_decode() {
    let mut result = SyntheticEvaluationHarness::execute(&exact_only_multi_disposition_fixture())
        .expect("execute");
    let role = ArtifactRole::ReferenceSeal;
    let payload = result
        .serialized_payloads
        .iter_mut()
        .find(|entry| entry.role == role)
        .expect("payload");
    let mut value: serde_json::Value =
        serde_json::from_slice(&payload.payload_bytes).expect("json");
    value["unexpected_field"] = serde_json::Value::String("not allowed".to_string());
    payload.payload_bytes = serde_json::to_vec(&value).expect("serialize");
    recompute_payload_descriptor(payload);
    sync_bundle_descriptor(&mut result, role);

    SyntheticEvaluationHarness::verify_payload_integrity(&result).expect("digest integrity");
    assert!(matches!(
        SyntheticEvaluationHarness::verify_typed_payload_round_trip(&result),
        Err(
            SyntheticEvaluationHarnessError::PayloadDeserializationFailure {
                role: ArtifactRole::ReferenceSeal
            }
        )
    ));
}

#[test]
fn role_type_mismatch_with_matching_digest_fails_typed_decode() {
    let mut result = SyntheticEvaluationHarness::execute(&exact_only_multi_disposition_fixture())
        .expect("execute");
    let join_payload = result
        .serialized_payloads
        .iter()
        .find(|entry| entry.role == ArtifactRole::EvaluationJoin)
        .expect("join payload")
        .payload_bytes
        .clone();
    let role = ArtifactRole::ReferenceSeal;
    let payload = result
        .serialized_payloads
        .iter_mut()
        .find(|entry| entry.role == role)
        .expect("payload");
    payload.payload_bytes = join_payload;
    recompute_payload_descriptor(payload);
    sync_bundle_descriptor(&mut result, role);

    SyntheticEvaluationHarness::verify_payload_integrity(&result).expect("digest integrity");
    assert!(matches!(
        SyntheticEvaluationHarness::verify_typed_payload_round_trip(&result),
        Err(
            SyntheticEvaluationHarnessError::PayloadDeserializationFailure {
                role: ArtifactRole::ReferenceSeal
            }
        ) | Err(SyntheticEvaluationHarnessError::TypedPayloadMismatch {
            role: ArtifactRole::ReferenceSeal
        }) | Err(
            SyntheticEvaluationHarnessError::DecodedPayloadValidationFailure {
                role: ArtifactRole::ReferenceSeal
            }
        )
    ));
}

#[test]
fn decoded_values_reserialize_to_exact_original_bytes() {
    let result = SyntheticEvaluationHarness::execute(&exact_only_multi_disposition_fixture())
        .expect("execute");
    for payload in &result.serialized_payloads {
        let reserialized = match payload.role {
            ArtifactRole::ReferenceSeal => serde_json::to_vec(
                &vox_proof::reference_seal::seal_from_json(
                    std::str::from_utf8(&payload.payload_bytes).expect("utf8"),
                )
                .expect("decode"),
            ),
            ArtifactRole::HumanFinalReference => serde_json::to_vec(
                &vox_proof::human_final_reference::human_final_reference_from_json(
                    std::str::from_utf8(&payload.payload_bytes).expect("utf8"),
                )
                .expect("decode"),
            ),
            ArtifactRole::CueReviewCompletion => serde_json::to_vec(
                &vox_proof::reference_coverage::coverage_from_json(
                    std::str::from_utf8(&payload.payload_bytes).expect("utf8"),
                )
                .expect("decode"),
            ),
            ArtifactRole::DetectorOutput => serde_json::to_vec(
                &vox_proof::detector_snapshot::detector_proposal_snapshot_from_json(
                    std::str::from_utf8(&payload.payload_bytes).expect("utf8"),
                )
                .expect("decode"),
            ),
            ArtifactRole::EvaluationJoin => serde_json::to_vec(
                &vox_proof::detector_reference_join::join_from_json(
                    std::str::from_utf8(&payload.payload_bytes).expect("utf8"),
                )
                .expect("decode"),
            ),
            ArtifactRole::JoinAdjudication => serde_json::to_vec(
                &vox_proof::join_adjudication::overlap_adjudication_from_json(
                    std::str::from_utf8(&payload.payload_bytes).expect("utf8"),
                )
                .expect("decode"),
            ),
            ArtifactRole::MetricContributions => serde_json::to_vec(
                &vox_proof::join_metric_contribution::contribution_from_json(
                    std::str::from_utf8(&payload.payload_bytes).expect("utf8"),
                )
                .expect("decode"),
            ),
            ArtifactRole::Metrics => serde_json::to_vec(
                &vox_proof::join_metric_aggregation::aggregate_from_json(
                    std::str::from_utf8(&payload.payload_bytes).expect("utf8"),
                )
                .expect("decode"),
            ),
            other => panic!("unexpected role {other:?}"),
        }
        .expect("reserialize");
        assert_eq!(
            reserialized, payload.payload_bytes,
            "byte mismatch for role {:?}",
            payload.role
        );
    }
}

// --- AssistedReview lifecycle semantics ---

#[test]
fn exact_only_assisted_review_transition_envelope_present_without_derivation() {
    let result = SyntheticEvaluationHarness::execute(&exact_only_multi_disposition_fixture())
        .expect("execute");
    assert_eq!(
        result.completion_stage,
        SyntheticEvaluationCompletionStage::DetectorExecution
    );
    assert_eq!(
        result.assisted_review_transition_envelope.lifecycle_state,
        RunLifecycleState::AssistedReview
    );
    assert!(result.final_adjudication_set.records.is_empty());

    for record in &result.execution_trace {
        if matches!(
            record.stage,
            SyntheticEvaluationStage::JoinResolved
                | SyntheticEvaluationStage::ContributionsComplete
                | SyntheticEvaluationStage::AggregatesComplete
        ) {
            assert_eq!(
                record.lifecycle_state,
                RunLifecycleState::DetectorExecution,
                "exact-only must not claim derivation at AssistedReview for stage {:?}",
                record.stage
            );
        }
    }
}

#[test]
fn overlap_records_derivation_at_assisted_review() {
    let result = SyntheticEvaluationHarness::execute(&overlap_pending_then_resolved_fixture())
        .expect("execute");
    assert_eq!(
        result.completion_stage,
        SyntheticEvaluationCompletionStage::AssistedReview
    );
    assert_eq!(
        result.assisted_review_transition_envelope.lifecycle_state,
        RunLifecycleState::AssistedReview
    );
    assert!(!result.final_adjudication_set.records.is_empty());

    assert!(result.execution_trace.iter().any(|record| {
        record.stage == SyntheticEvaluationStage::JoinResolved
            && record.lifecycle_state == RunLifecycleState::AssistedReview
    }));
}

#[test]
fn both_paths_reach_finalized_and_detector_to_finalized_remains_illegal() {
    for fixture in [
        exact_only_multi_disposition_fixture(),
        overlap_pending_then_resolved_fixture(),
    ] {
        let result = SyntheticEvaluationHarness::execute(&fixture).expect("execute");
        assert_eq!(
            result.finalized_envelope.lifecycle_state,
            RunLifecycleState::Finalized
        );
    }

    assert!(
        RunEnvelope::validate_transition(
            RunLifecycleState::DetectorExecution,
            RunLifecycleState::Finalized,
            vox_proof::run_manifest::CalibrationValidityMode::BlindReference,
        )
        .is_err()
    );
}

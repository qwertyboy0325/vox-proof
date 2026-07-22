#![allow(clippy::too_many_arguments)]

#[path = "synthetic_evaluation_harness_fixtures.rs"]
#[allow(dead_code)]
mod fixtures;

use fixtures::{
    exact_only_multi_disposition_fixture, overlap_pending_then_resolved_fixture,
    zero_population_fixture,
};
use sha2::{Digest, Sha256};
use vox_proof::artifact_bundle::ArtifactContentDigest;
use vox_proof::evaluation_artifact_packet::{
    DetachedPacketDigestValidationError, EVALUATION_ARTIFACT_PACKET_SCHEMA,
    EvaluationArtifactPacket, EvaluationArtifactPacketDigest, EvaluationArtifactPacketError,
    PACKET_SERIALIZATION_POLICY, build_from_synthetic_harness_result, decode_and_verify_packet,
    decode_packet, encode_packet,
};
use vox_proof::join_metric_aggregation::MetricAggregateValueState;
use vox_proof::run_manifest::{
    ArtifactRole, CalibrationValidityMode, InputClass, RunEnvelope, RunId, RunLifecycleState,
    WorkflowObservationMode,
};
use vox_proof::synthetic_evaluation_harness::{
    SyntheticEvaluationCompletionStage, SyntheticEvaluationHarness,
};

const EXACT_ONLY_PACKET_BYTE_LENGTH: u64 = 37_615;
const EXACT_ONLY_PACKET_DIGEST: &str =
    "sha256:427204f744b36a39958c4cc0dffd63f95bf6aba720984f8e32a2a519307f1d17";

const OVERLAP_PACKET_BYTE_LENGTH: u64 = 22_896;
const OVERLAP_PACKET_DIGEST: &str =
    "sha256:182e433be44f67115e0c666ad95b92d553c55da969cdf08705ef4ba5340584df";

const ZERO_POPULATION_PACKET_BYTE_LENGTH: u64 = 19_393;
const ZERO_POPULATION_PACKET_DIGEST: &str =
    "sha256:09620f8c3bbe0a8a8339f8ab99a9074ac982894052b6c7052816e3f37c2575c8";

const EXPECTED_PACKET_ROLES: [ArtifactRole; 8] = [
    ArtifactRole::ReferenceSeal,
    ArtifactRole::HumanFinalReference,
    ArtifactRole::CueReviewCompletion,
    ArtifactRole::DetectorOutput,
    ArtifactRole::EvaluationJoin,
    ArtifactRole::JoinAdjudication,
    ArtifactRole::MetricContributions,
    ArtifactRole::Metrics,
];

fn build_and_encode(
    fixture: vox_proof::synthetic_evaluation_harness::SyntheticEvaluationFixture,
) -> vox_proof::evaluation_artifact_packet::EncodedEvaluationArtifactPacket {
    let result = SyntheticEvaluationHarness::execute(&fixture).expect("execute");
    let packet = build_from_synthetic_harness_result(&result).expect("build");
    encode_packet(&packet).expect("encode")
}

fn sync_bundle_descriptor(
    packet: &mut EvaluationArtifactPacket,
    role: ArtifactRole,
    digest: ArtifactContentDigest,
    byte_length: u64,
) {
    let descriptor = packet
        .artifact_bundle
        .artifacts
        .iter_mut()
        .find(|entry| entry.role == role)
        .expect("descriptor");
    descriptor.content_digest = digest;
    descriptor.byte_length = byte_length;
}

fn payload_index(packet: &EvaluationArtifactPacket, role: ArtifactRole) -> usize {
    packet
        .payloads
        .iter()
        .position(|entry| {
            packet.artifact_bundle.artifacts.iter().any(|descriptor| {
                descriptor.artifact_id == entry.artifact_id && descriptor.role == role
            })
        })
        .expect("payload role")
}

fn apply_typed_payload<T: serde::Serialize>(
    packet: &mut EvaluationArtifactPacket,
    role: ArtifactRole,
    value: &T,
) -> EvaluationArtifactPacket {
    let index = payload_index(packet, role);
    let bytes = serde_json::to_vec(value).expect("serialize");
    packet.payloads[index].payload_utf8_json = String::from_utf8(bytes).expect("utf8");
    let hash = Sha256::digest(packet.payloads[index].payload_utf8_json.as_bytes());
    let hex = hash
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let digest = ArtifactContentDigest::new(format!("sha256:{hex}")).expect("digest");
    sync_bundle_descriptor(
        packet,
        role,
        digest,
        packet.payloads[index].payload_utf8_json.len() as u64,
    );
    packet.clone()
}

fn tamper_payload_via_value(
    packet: &mut EvaluationArtifactPacket,
    role: ArtifactRole,
    mutate: impl FnOnce(&mut serde_json::Value),
) -> EvaluationArtifactPacket {
    let index = payload_index(packet, role);
    let mut value: serde_json::Value =
        serde_json::from_str(&packet.payloads[index].payload_utf8_json).expect("json");
    mutate(&mut value);
    packet.payloads[index].payload_utf8_json =
        String::from_utf8(serde_json::to_vec(&value).expect("serialize")).expect("utf8");
    let hash = Sha256::digest(packet.payloads[index].payload_utf8_json.as_bytes());
    let hex = hash
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let digest = ArtifactContentDigest::new(format!("sha256:{hex}")).expect("digest");
    sync_bundle_descriptor(
        packet,
        role,
        digest,
        packet.payloads[index].payload_utf8_json.len() as u64,
    );
    packet.clone()
}

fn reencode_packet(packet: &EvaluationArtifactPacket) -> Vec<u8> {
    encode_packet(packet).expect("encode").packet_bytes
}

fn exact_only_decoded_packet() -> EvaluationArtifactPacket {
    let encoded = build_and_encode(exact_only_multi_disposition_fixture());
    decode_packet(&encoded.packet_bytes).expect("decode")
}

fn verify_reencoded_packet(
    packet: &EvaluationArtifactPacket,
) -> Result<
    vox_proof::evaluation_artifact_packet::VerifiedEvaluationArtifactPacket,
    EvaluationArtifactPacketError,
> {
    let encoded = encode_packet(packet).expect("encode");
    decode_and_verify_packet(&encoded.packet_bytes, Some(&encoded.content_digest))
}

fn assert_envelopes_share_posture(packet: &EvaluationArtifactPacket) {
    let detector = &packet.detector_execution_envelope;
    for (label, envelope) in [
        (
            "assisted_review",
            &packet.assisted_review_transition_envelope,
        ),
        ("finalized", &packet.finalized_envelope),
    ] {
        assert_eq!(envelope.run_id, detector.run_id, "{label} run_id");
        assert_eq!(
            envelope.input_identity, detector.input_identity,
            "{label} input_identity"
        );
        assert_eq!(
            envelope.calibration_validity, detector.calibration_validity,
            "{label} calibration_validity"
        );
        assert_eq!(
            envelope.workflow_observation, detector.workflow_observation,
            "{label} workflow_observation"
        );
        assert_eq!(
            envelope.input_class, detector.input_class,
            "{label} input_class"
        );
        assert_eq!(
            envelope.qualifies_as_real_material_evidence,
            detector.qualifies_as_real_material_evidence,
            "{label} qualifies_as_real_material_evidence"
        );
        assert_eq!(
            envelope.expected_artifact_roles, detector.expected_artifact_roles,
            "{label} expected_artifact_roles"
        );
    }
    assert_eq!(
        packet.artifact_bundle.expected_roles,
        detector.expected_artifact_roles
    );
    assert_eq!(
        packet.artifact_bundle.binding_context.run_id,
        detector.run_id
    );
    assert_eq!(
        packet.artifact_bundle.binding_context.input_identity,
        detector.input_identity
    );
    assert_eq!(
        packet.artifact_bundle.binding_context.calibration_validity,
        detector.calibration_validity
    );
}

#[test]
fn exact_only_packet_round_trip_and_verify() {
    let encoded = build_and_encode(exact_only_multi_disposition_fixture());
    let verified = decode_and_verify_packet(&encoded.packet_bytes, Some(&encoded.content_digest))
        .expect("verify");
    assert_eq!(
        verified.completion_stage,
        SyntheticEvaluationCompletionStage::DetectorExecution
    );
    assert_eq!(encoded.byte_length, EXACT_ONLY_PACKET_BYTE_LENGTH);
    assert_eq!(encoded.content_digest.as_str(), EXACT_ONLY_PACKET_DIGEST);
}

#[test]
fn overlap_packet_round_trip_and_verify() {
    let encoded = build_and_encode(overlap_pending_then_resolved_fixture());
    decode_and_verify_packet(&encoded.packet_bytes, Some(&encoded.content_digest)).expect("verify");
    assert_eq!(encoded.byte_length, OVERLAP_PACKET_BYTE_LENGTH);
    assert_eq!(encoded.content_digest.as_str(), OVERLAP_PACKET_DIGEST);
}

#[test]
fn zero_population_packet_round_trip_and_verify() {
    let encoded = build_and_encode(zero_population_fixture());
    decode_and_verify_packet(&encoded.packet_bytes, Some(&encoded.content_digest)).expect("verify");
    assert_eq!(encoded.byte_length, ZERO_POPULATION_PACKET_BYTE_LENGTH);
    assert_eq!(
        encoded.content_digest.as_str(),
        ZERO_POPULATION_PACKET_DIGEST
    );
}

#[test]
fn verify_without_expected_digest_still_runs_semantic_checks() {
    let encoded = build_and_encode(exact_only_multi_disposition_fixture());
    decode_and_verify_packet(&encoded.packet_bytes, None).expect("verify without digest");
}

#[test]
fn repeated_build_and_encode_is_deterministic() {
    let first = build_and_encode(exact_only_multi_disposition_fixture());
    let second = build_and_encode(exact_only_multi_disposition_fixture());
    assert_eq!(first.packet_bytes, second.packet_bytes);
    assert_eq!(first.content_digest, second.content_digest);
}

#[test]
fn exact_only_lifecycle_semantics_in_packet() {
    let encoded = build_and_encode(exact_only_multi_disposition_fixture());
    let verified = decode_and_verify_packet(&encoded.packet_bytes, Some(&encoded.content_digest))
        .expect("verify");
    assert_eq!(
        verified.derivation_lifecycle,
        RunLifecycleState::DetectorExecution
    );
    assert_eq!(
        verified
            .packet
            .assisted_review_transition_envelope
            .lifecycle_state,
        RunLifecycleState::AssistedReview
    );
}

#[test]
fn overlap_lifecycle_semantics_in_packet() {
    let encoded = build_and_encode(overlap_pending_then_resolved_fixture());
    let verified = decode_and_verify_packet(&encoded.packet_bytes, Some(&encoded.content_digest))
        .expect("verify");
    assert_eq!(
        verified.completion_stage,
        SyntheticEvaluationCompletionStage::AssistedReview
    );
    assert_eq!(
        verified.derivation_lifecycle,
        RunLifecycleState::AssistedReview
    );
}

#[test]
fn zero_population_aggregates_remain_undefined_non_primary() {
    let encoded = build_and_encode(zero_population_fixture());
    let verified = decode_and_verify_packet(&encoded.packet_bytes, Some(&encoded.content_digest))
        .expect("verify");
    let metrics_payload = verified.packet.payloads.last().expect("metrics payload");
    let aggregates =
        vox_proof::join_metric_aggregation::aggregate_from_json(&metrics_payload.payload_utf8_json)
            .expect("decode");
    assert_eq!(aggregates.metrics.len(), 5);
    assert!(aggregates.metrics.iter().all(
        |metric| metric.value_state == MetricAggregateValueState::UndefinedZeroDenominator
    ));
    assert!(!aggregates.primary_metrics_allowed);
}

#[test]
fn detector_execution_to_finalized_remains_illegal() {
    assert!(
        RunEnvelope::validate_transition(
            RunLifecycleState::DetectorExecution,
            RunLifecycleState::Finalized,
            vox_proof::run_manifest::CalibrationValidityMode::BlindReference,
        )
        .is_err()
    );
}

#[test]
fn malformed_outer_json_rejected() {
    assert!(matches!(
        decode_and_verify_packet(b"{not json", None),
        Err(EvaluationArtifactPacketError::PacketDeserializationFailure)
    ));
}

#[test]
fn unknown_outer_field_rejected() {
    let encoded = build_and_encode(exact_only_multi_disposition_fixture());
    let mut value: serde_json::Value = serde_json::from_slice(&encoded.packet_bytes).expect("json");
    value["unexpected"] = serde_json::Value::String("x".into());
    let bytes = serde_json::to_vec(&value).expect("serialize");
    assert!(matches!(
        decode_and_verify_packet(&bytes, None),
        Err(EvaluationArtifactPacketError::PacketDeserializationFailure)
    ));
}

#[test]
fn unsupported_schema_revision_rejected() {
    let encoded = build_and_encode(exact_only_multi_disposition_fixture());
    let mut packet = decode_packet(&encoded.packet_bytes).expect("decode");
    packet.schema_revision = "unsupported".to_string();
    let bytes = reencode_packet(&packet);
    assert!(matches!(
        decode_and_verify_packet(&bytes, None),
        Err(EvaluationArtifactPacketError::UnsupportedPacketSchemaRevision)
    ));
}

#[test]
fn unsupported_policy_rejected() {
    let encoded = build_and_encode(exact_only_multi_disposition_fixture());
    let mut packet = decode_packet(&encoded.packet_bytes).expect("decode");
    packet.packet_serialization_policy = "other".to_string();
    let bytes = reencode_packet(&packet);
    assert!(matches!(
        decode_and_verify_packet(&bytes, None),
        Err(EvaluationArtifactPacketError::UnsupportedPacketPolicy)
    ));
}

#[test]
fn payload_reorder_rejected() {
    let encoded = build_and_encode(exact_only_multi_disposition_fixture());
    let mut packet = decode_packet(&encoded.packet_bytes).expect("decode");
    packet.payloads.swap(0, 1);
    let bytes = reencode_packet(&packet);
    assert!(matches!(
        decode_and_verify_packet(&bytes, None),
        Err(EvaluationArtifactPacketError::NonCanonicalPayloadOrder)
    ));
}

#[test]
fn missing_payload_entry_rejected() {
    let encoded = build_and_encode(exact_only_multi_disposition_fixture());
    let mut packet = decode_packet(&encoded.packet_bytes).expect("decode");
    packet.payloads.pop();
    let bytes = reencode_packet(&packet);
    assert!(matches!(
        decode_and_verify_packet(&bytes, None),
        Err(EvaluationArtifactPacketError::PacketInventoryMismatch)
            | Err(EvaluationArtifactPacketError::NonCanonicalPayloadOrder)
    ));
}

#[test]
fn duplicate_payload_entry_rejected() {
    let encoded = build_and_encode(exact_only_multi_disposition_fixture());
    let mut packet = decode_packet(&encoded.packet_bytes).expect("decode");
    let duplicate = packet.payloads[0].clone();
    packet.payloads.push(duplicate);
    let bytes = reencode_packet(&packet);
    assert!(matches!(
        decode_and_verify_packet(&bytes, None),
        Err(EvaluationArtifactPacketError::PacketInventoryMismatch)
            | Err(EvaluationArtifactPacketError::DuplicatePacketPayload { .. })
    ));
}

#[test]
fn completion_stage_mismatch_rejected() {
    let encoded = build_and_encode(exact_only_multi_disposition_fixture());
    let mut packet = decode_packet(&encoded.packet_bytes).expect("decode");
    packet.completion_stage = SyntheticEvaluationCompletionStage::AssistedReview;
    let bytes = reencode_packet(&packet);
    assert!(matches!(
        decode_and_verify_packet(&bytes, None),
        Err(EvaluationArtifactPacketError::PacketCompletionStageMismatch)
    ));
}

#[test]
fn one_byte_packet_mutation_fails_detached_digest() {
    let encoded = build_and_encode(exact_only_multi_disposition_fixture());
    let mut bytes = encoded.packet_bytes.clone();
    bytes.push(b'x');
    assert!(matches!(
        decode_and_verify_packet(&bytes, Some(&encoded.content_digest)),
        Err(EvaluationArtifactPacketError::DetachedPacketDigestMismatch)
    ));
}

#[test]
fn wrong_expected_digest_rejected() {
    let encoded = build_and_encode(exact_only_multi_disposition_fixture());
    let wrong = EvaluationArtifactPacketDigest::new(OVERLAP_PACKET_DIGEST).expect("digest");
    assert!(matches!(
        decode_and_verify_packet(&encoded.packet_bytes, Some(&wrong)),
        Err(EvaluationArtifactPacketError::DetachedPacketDigestMismatch)
    ));
}

#[test]
fn tampered_reference_seal_passes_hashes_fails_semantics() {
    let encoded = build_and_encode(exact_only_multi_disposition_fixture());
    let mut packet = decode_packet(&encoded.packet_bytes).expect("decode");
    let index = payload_index(&packet, ArtifactRole::ReferenceSeal);
    let mut seal =
        vox_proof::reference_seal::seal_from_json(&packet.payloads[index].payload_utf8_json)
            .expect("decode seal");
    seal.reference_revision =
        vox_proof::reference_identity::ReferenceRevisionId::new("ref-rev-tampered-001")
            .expect("revision");
    let tampered = apply_typed_payload(&mut packet, ArtifactRole::ReferenceSeal, &seal);
    let bytes = reencode_packet(&tampered);
    let digest = encode_packet(&tampered).expect("encode").content_digest;
    let result = decode_and_verify_packet(&bytes, Some(&digest));
    assert!(matches!(
        result,
        Err(EvaluationArtifactPacketError::PacketJoinRederivationMismatch)
            | Err(EvaluationArtifactPacketError::PacketHistoricalReplayFailure { .. })
            | Err(EvaluationArtifactPacketError::JoinValidation(_))
            | Err(
                EvaluationArtifactPacketError::EmbeddedPayloadValidationFailure {
                    role: ArtifactRole::ReferenceSeal
                }
            )
    ));
}

#[test]
fn tampered_evaluation_join_passes_hashes_fails_rederivation() {
    let encoded = build_and_encode(exact_only_multi_disposition_fixture());
    let mut packet = decode_packet(&encoded.packet_bytes).expect("decode");
    let index = payload_index(&packet, ArtifactRole::EvaluationJoin);
    let mut join = vox_proof::detector_reference_join::join_from_json(
        &packet.payloads[index].payload_utf8_json,
    )
    .expect("decode join");
    join.assessment.exact_match_count = join.assessment.exact_match_count.saturating_add(1);
    let tampered = apply_typed_payload(&mut packet, ArtifactRole::EvaluationJoin, &join);
    let bytes = reencode_packet(&tampered);
    let digest = encode_packet(&tampered).expect("encode").content_digest;
    let result = decode_and_verify_packet(&bytes, Some(&digest));
    assert!(matches!(
        result,
        Err(EvaluationArtifactPacketError::PacketJoinRederivationMismatch)
            | Err(
                EvaluationArtifactPacketError::EmbeddedPayloadValidationFailure {
                    role: ArtifactRole::EvaluationJoin
                }
            )
            | Err(EvaluationArtifactPacketError::PacketHistoricalReplayFailure { .. })
    ));
}

#[test]
fn tampered_metrics_passes_hashes_fails_rederivation() {
    let encoded = build_and_encode(exact_only_multi_disposition_fixture());
    let mut packet = decode_packet(&encoded.packet_bytes).expect("decode");
    let index = payload_index(&packet, ArtifactRole::Metrics);
    let mut aggregates = vox_proof::join_metric_aggregation::aggregate_from_json(
        &packet.payloads[index].payload_utf8_json,
    )
    .expect("decode aggregates");
    aggregates.metrics[0].numerator_count = aggregates.metrics[0].numerator_count.saturating_add(1);
    let tampered = apply_typed_payload(&mut packet, ArtifactRole::Metrics, &aggregates);
    let bytes = reencode_packet(&tampered);
    let digest = encode_packet(&tampered).expect("encode").content_digest;
    let result = decode_and_verify_packet(&bytes, Some(&digest));
    assert!(matches!(
        result,
        Err(EvaluationArtifactPacketError::PacketAggregationRederivationMismatch)
            | Err(
                EvaluationArtifactPacketError::EmbeddedPayloadValidationFailure {
                    role: ArtifactRole::Metrics
                }
            )
            | Err(EvaluationArtifactPacketError::PacketHistoricalReplayFailure { .. })
    ));
}

#[test]
fn tampered_contributions_passes_hashes_fails_rederivation() {
    let encoded = build_and_encode(exact_only_multi_disposition_fixture());
    let mut packet = decode_packet(&encoded.packet_bytes).expect("decode");
    let index = payload_index(&packet, ArtifactRole::MetricContributions);
    let mut contributions = vox_proof::join_metric_contribution::contribution_from_json(
        &packet.payloads[index].payload_utf8_json,
    )
    .expect("decode contributions");
    if let Some(record) = contributions.detector_contributions.first_mut() {
        record.proposal_precision =
            vox_proof::join_metric_contribution::RatioContribution::Excluded(
                vox_proof::join_metric_contribution::MetricContributionExclusionReason::JoinExcluded,
            );
    }
    let tampered = apply_typed_payload(
        &mut packet,
        ArtifactRole::MetricContributions,
        &contributions,
    );
    let bytes = reencode_packet(&tampered);
    let digest = encode_packet(&tampered).expect("encode").content_digest;
    let result = decode_and_verify_packet(&bytes, Some(&digest));
    assert!(matches!(
        result,
        Err(EvaluationArtifactPacketError::PacketContributionRederivationMismatch)
            | Err(
                EvaluationArtifactPacketError::EmbeddedPayloadValidationFailure {
                    role: ArtifactRole::MetricContributions
                }
            )
            | Err(EvaluationArtifactPacketError::PacketHistoricalReplayFailure { .. })
    ));
}

#[test]
fn unknown_embedded_field_rejected() {
    let encoded = build_and_encode(exact_only_multi_disposition_fixture());
    let mut packet = decode_packet(&encoded.packet_bytes).expect("decode");
    let tampered = tamper_payload_via_value(&mut packet, ArtifactRole::ReferenceSeal, |value| {
        value["unexpected_field"] = serde_json::Value::String("x".into());
    });
    let bytes = reencode_packet(&tampered);
    let digest = encode_packet(&tampered).expect("encode").content_digest;
    assert!(matches!(
        decode_and_verify_packet(&bytes, Some(&digest)),
        Err(EvaluationArtifactPacketError::EmbeddedPayloadTypeDecodeFailure { .. })
    ));
}

#[test]
fn role_type_substitution_rejected() {
    let encoded = build_and_encode(exact_only_multi_disposition_fixture());
    let mut packet = decode_packet(&encoded.packet_bytes).expect("decode");
    let join_json = packet
        .payloads
        .iter()
        .find(|entry| {
            packet.artifact_bundle.artifacts.iter().any(|descriptor| {
                descriptor.artifact_id == entry.artifact_id
                    && descriptor.role == ArtifactRole::EvaluationJoin
            })
        })
        .expect("join")
        .payload_utf8_json
        .clone();
    packet.payloads[0].payload_utf8_json = join_json;
    let byte_length = packet.payloads[0].payload_utf8_json.len() as u64;
    let digest = ArtifactContentDigest::new(format!(
        "sha256:{}",
        Sha256::digest(packet.payloads[0].payload_utf8_json.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    ))
    .expect("digest");
    sync_bundle_descriptor(
        &mut packet,
        ArtifactRole::ReferenceSeal,
        digest,
        byte_length,
    );
    let reencoded = reencode_packet(&packet);
    let packet_digest = encode_packet(&packet).expect("encode").content_digest;
    let result = decode_and_verify_packet(&reencoded, Some(&packet_digest));
    assert!(result.is_err(), "unexpected success: {result:?}");
    assert!(matches!(
        result,
        Err(EvaluationArtifactPacketError::EmbeddedPayloadTypeDecodeFailure { .. })
            | Err(EvaluationArtifactPacketError::EmbeddedPayloadValidationFailure { .. })
            | Err(EvaluationArtifactPacketError::EmbeddedPayloadReserializationMismatch { .. })
            | Err(EvaluationArtifactPacketError::PacketJoinRederivationMismatch)
    ));
}

#[test]
fn bundle_descriptor_role_change_rejected() {
    let encoded = build_and_encode(exact_only_multi_disposition_fixture());
    let mut packet = decode_packet(&encoded.packet_bytes).expect("decode");
    packet.artifact_bundle.artifacts[0].role = ArtifactRole::Metrics;
    let bytes = reencode_packet(&packet);
    let result = decode_and_verify_packet(&bytes, None);
    assert!(result.is_err(), "unexpected success: {result:?}");
    assert!(matches!(
        result,
        Err(EvaluationArtifactPacketError::PacketBundleValidation(_))
            | Err(EvaluationArtifactPacketError::PacketInventoryMismatch)
            | Err(EvaluationArtifactPacketError::NonCanonicalPayloadOrder)
            | Err(EvaluationArtifactPacketError::MissingPacketPayload { .. })
            | Err(EvaluationArtifactPacketError::EmbeddedPayloadValidationFailure { .. })
            | Err(EvaluationArtifactPacketError::PacketHistoricalReplayFailure { .. })
            | Err(EvaluationArtifactPacketError::PacketJoinRederivationMismatch)
            | Err(EvaluationArtifactPacketError::NonCanonicalPacketEncoding)
            | Err(EvaluationArtifactPacketError::SnapshotValidation(_))
    ));
}

#[test]
fn fixed_regression_seals_exact_only() {
    let encoded = build_and_encode(exact_only_multi_disposition_fixture());
    assert_eq!(encoded.byte_length, EXACT_ONLY_PACKET_BYTE_LENGTH);
    assert_eq!(encoded.content_digest.as_str(), EXACT_ONLY_PACKET_DIGEST);
}

#[test]
fn fixed_regression_seals_overlap() {
    let encoded = build_and_encode(overlap_pending_then_resolved_fixture());
    assert_eq!(encoded.byte_length, OVERLAP_PACKET_BYTE_LENGTH);
    assert_eq!(encoded.content_digest.as_str(), OVERLAP_PACKET_DIGEST);
}

#[test]
fn fixed_regression_seals_zero_population() {
    let encoded = build_and_encode(zero_population_fixture());
    assert_eq!(encoded.byte_length, ZERO_POPULATION_PACKET_BYTE_LENGTH);
    assert_eq!(
        encoded.content_digest.as_str(),
        ZERO_POPULATION_PACKET_DIGEST
    );
}

#[test]
fn packet_schema_constant_matches() {
    let encoded = build_and_encode(exact_only_multi_disposition_fixture());
    let packet = decode_packet(&encoded.packet_bytes).expect("decode");
    assert_eq!(packet.schema_revision, EVALUATION_ARTIFACT_PACKET_SCHEMA);
    assert_eq!(
        packet.packet_serialization_policy,
        PACKET_SERIALIZATION_POLICY
    );
}

#[test]
fn exact_only_envelopes_share_full_posture() {
    let packet = exact_only_decoded_packet();
    assert_envelopes_share_posture(&packet);
    assert_eq!(
        packet.detector_execution_envelope.calibration_validity,
        CalibrationValidityMode::BlindReference
    );
    assert_eq!(
        packet.detector_execution_envelope.input_class,
        InputClass::SyntheticProtocolFixture
    );
    assert!(
        !packet
            .detector_execution_envelope
            .qualifies_as_real_material_evidence
    );
}

#[test]
fn overlap_envelopes_share_full_posture() {
    let encoded = build_and_encode(overlap_pending_then_resolved_fixture());
    let packet = decode_packet(&encoded.packet_bytes).expect("decode");
    assert_envelopes_share_posture(&packet);
}

#[test]
fn role_inventory_matches_canonical_order() {
    let packet = exact_only_decoded_packet();
    for envelope in [
        &packet.detector_execution_envelope,
        &packet.assisted_review_transition_envelope,
        &packet.finalized_envelope,
    ] {
        assert_eq!(
            envelope.expected_artifact_roles.as_slice(),
            EXPECTED_PACKET_ROLES
        );
    }
    assert_eq!(
        packet.artifact_bundle.expected_roles.as_slice(),
        EXPECTED_PACKET_ROLES
    );
}

#[test]
fn tampered_assisted_review_calibration_rejected_after_digest_recompute() {
    let mut packet = exact_only_decoded_packet();
    packet
        .assisted_review_transition_envelope
        .calibration_validity = CalibrationValidityMode::DetectorAssisted;
    packet
        .assisted_review_transition_envelope
        .validate()
        .expect("assisted envelope remains valid");
    let result = verify_reencoded_packet(&packet);
    assert!(matches!(
        result,
        Err(
            EvaluationArtifactPacketError::PacketEnvelopePostureMismatch {
                field: "calibration_validity",
                lifecycle_state: RunLifecycleState::AssistedReview,
            }
        )
    ));
}

#[test]
fn tampered_assisted_review_input_class_rejected_after_digest_recompute() {
    let mut packet = exact_only_decoded_packet();
    packet.assisted_review_transition_envelope.input_class = InputClass::SelfOwnedReal;
    packet
        .assisted_review_transition_envelope
        .validate()
        .expect("assisted envelope remains valid");
    let result = verify_reencoded_packet(&packet);
    assert!(matches!(
        result,
        Err(
            EvaluationArtifactPacketError::PacketEnvelopePostureMismatch {
                field: "input_class",
                lifecycle_state: RunLifecycleState::AssistedReview,
            }
        ) | Err(EvaluationArtifactPacketError::UnsupportedPacketRunPosture { .. })
    ));
}

#[test]
fn tampered_assisted_review_real_material_flag_rejected_after_digest_recompute() {
    let mut packet = exact_only_decoded_packet();
    for envelope in [
        &mut packet.detector_execution_envelope,
        &mut packet.assisted_review_transition_envelope,
        &mut packet.finalized_envelope,
    ] {
        envelope.input_class = InputClass::SelfOwnedReal;
        envelope.qualifies_as_real_material_evidence = true;
        envelope.validate().expect("envelope valid");
    }
    let result = verify_reencoded_packet(&packet);
    assert!(matches!(
        result,
        Err(EvaluationArtifactPacketError::UnsupportedPacketRunPosture {
            qualifies_as_real_material_evidence: true,
            ..
        })
    ));
}

#[test]
fn tampered_assisted_review_workflow_observation_rejected_after_digest_recompute() {
    let mut packet = exact_only_decoded_packet();
    let flipped = match packet.detector_execution_envelope.workflow_observation {
        WorkflowObservationMode::Enabled => WorkflowObservationMode::Disabled,
        WorkflowObservationMode::Disabled => WorkflowObservationMode::Enabled,
    };
    packet
        .assisted_review_transition_envelope
        .workflow_observation = flipped;
    packet
        .assisted_review_transition_envelope
        .validate()
        .expect("assisted envelope remains valid");
    let result = verify_reencoded_packet(&packet);
    assert!(matches!(
        result,
        Err(
            EvaluationArtifactPacketError::PacketEnvelopePostureMismatch {
                field: "workflow_observation",
                lifecycle_state: RunLifecycleState::AssistedReview,
            }
        )
    ));
}

#[test]
fn tampered_finalized_calibration_rejected_after_digest_recompute() {
    let mut packet = exact_only_decoded_packet();
    packet.finalized_envelope.calibration_validity = CalibrationValidityMode::DetectorAssisted;
    packet
        .finalized_envelope
        .validate()
        .expect("finalized valid");
    let result = verify_reencoded_packet(&packet);
    assert!(matches!(
        result,
        Err(
            EvaluationArtifactPacketError::PacketEnvelopePostureMismatch {
                field: "calibration_validity",
                lifecycle_state: RunLifecycleState::Finalized,
            }
        )
    ));
}

#[test]
fn tampered_finalized_input_class_rejected_after_digest_recompute() {
    let mut packet = exact_only_decoded_packet();
    packet.finalized_envelope.input_class = InputClass::SelfOwnedReal;
    packet
        .finalized_envelope
        .validate()
        .expect("finalized valid");
    let result = verify_reencoded_packet(&packet);
    assert!(matches!(
        result,
        Err(
            EvaluationArtifactPacketError::PacketEnvelopePostureMismatch {
                field: "input_class",
                lifecycle_state: RunLifecycleState::Finalized,
            }
        ) | Err(EvaluationArtifactPacketError::UnsupportedPacketRunPosture { .. })
    ));
}

#[test]
fn tampered_finalized_workflow_observation_rejected_after_digest_recompute() {
    let mut packet = exact_only_decoded_packet();
    let flipped = match packet.detector_execution_envelope.workflow_observation {
        WorkflowObservationMode::Enabled => WorkflowObservationMode::Disabled,
        WorkflowObservationMode::Disabled => WorkflowObservationMode::Enabled,
    };
    packet.finalized_envelope.workflow_observation = flipped;
    packet
        .finalized_envelope
        .validate()
        .expect("finalized valid");
    let result = verify_reencoded_packet(&packet);
    assert!(matches!(
        result,
        Err(
            EvaluationArtifactPacketError::PacketEnvelopePostureMismatch {
                field: "workflow_observation",
                lifecycle_state: RunLifecycleState::Finalized,
            }
        )
    ));
}

#[test]
fn tampered_finalized_real_material_flag_rejected_after_digest_recompute() {
    let mut packet = exact_only_decoded_packet();
    for envelope in [
        &mut packet.detector_execution_envelope,
        &mut packet.assisted_review_transition_envelope,
        &mut packet.finalized_envelope,
    ] {
        envelope.input_class = InputClass::SelfOwnedReal;
        envelope.qualifies_as_real_material_evidence = true;
        envelope.validate().expect("envelope valid");
    }
    let result = verify_reencoded_packet(&packet);
    assert!(matches!(
        result,
        Err(EvaluationArtifactPacketError::UnsupportedPacketRunPosture {
            qualifies_as_real_material_evidence: true,
            ..
        })
    ));
}

#[test]
fn unsupported_detector_assisted_posture_rejected() {
    let mut packet = exact_only_decoded_packet();
    for envelope in [
        &mut packet.detector_execution_envelope,
        &mut packet.assisted_review_transition_envelope,
        &mut packet.finalized_envelope,
    ] {
        envelope.calibration_validity = CalibrationValidityMode::DetectorAssisted;
        envelope.validate().expect("envelope valid");
    }
    let result = verify_reencoded_packet(&packet);
    assert!(matches!(
        result,
        Err(EvaluationArtifactPacketError::UnsupportedPacketRunPosture {
            calibration_validity: CalibrationValidityMode::DetectorAssisted,
            ..
        })
    ));
}

#[test]
fn bundle_binding_run_id_mismatch_rejected() {
    let mut packet = exact_only_decoded_packet();
    packet.artifact_bundle.binding_context.run_id =
        RunId::new("run-id-tampered-001").expect("run id");
    let result = verify_reencoded_packet(&packet);
    assert!(matches!(
        result,
        Err(EvaluationArtifactPacketError::PacketBundleBindingContextMismatch { field: "run_id" })
    ));
}

#[test]
fn bundle_binding_input_identity_mismatch_rejected() {
    let mut packet = exact_only_decoded_packet();
    packet
        .artifact_bundle
        .binding_context
        .input_identity
        .transcript_revision_id =
        "rev:sha256-v1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            .to_string();
    let result = verify_reencoded_packet(&packet);
    assert!(matches!(
        result,
        Err(
            EvaluationArtifactPacketError::PacketBundleBindingContextMismatch {
                field: "input_identity"
            }
        )
    ));
}

#[test]
fn bundle_binding_calibration_mismatch_rejected() {
    let mut packet = exact_only_decoded_packet();
    packet.artifact_bundle.binding_context.calibration_validity =
        CalibrationValidityMode::DetectorAssisted;
    let result = verify_reencoded_packet(&packet);
    assert!(matches!(
        result,
        Err(
            EvaluationArtifactPacketError::PacketBundleBindingContextMismatch {
                field: "calibration_validity"
            }
        )
    ));
}

#[test]
fn detached_digest_lowercase_accepted() {
    EvaluationArtifactPacketDigest::new(EXACT_ONLY_PACKET_DIGEST).expect("lowercase digest");
}

#[test]
fn detached_digest_uppercase_hex_rejected() {
    let uppercase = format!(
        "sha256:{}",
        EXACT_ONLY_PACKET_DIGEST[7..]
            .chars()
            .map(|ch| {
                if ('a'..='f').contains(&ch) {
                    ch.to_ascii_uppercase()
                } else {
                    ch
                }
            })
            .collect::<String>()
    );
    assert!(matches!(
        EvaluationArtifactPacketDigest::new(uppercase),
        Err(EvaluationArtifactPacketError::InvalidDetachedPacketDigest {
            reason: DetachedPacketDigestValidationError::UppercaseHexNotCanonical
        })
    ));
}

#[test]
fn detached_digest_wrong_prefix_rejected() {
    assert!(matches!(
        EvaluationArtifactPacketDigest::new(
            "md5:427204f744b36a39958c4cc0dffd63f95bf6aba720984f8e32a2a519307f1d17"
        ),
        Err(EvaluationArtifactPacketError::InvalidDetachedPacketDigest {
            reason: DetachedPacketDigestValidationError::MissingPrefix
        })
    ));
}

#[test]
fn detached_digest_short_rejected() {
    assert!(matches!(
        EvaluationArtifactPacketDigest::new("sha256:abc"),
        Err(EvaluationArtifactPacketError::InvalidDetachedPacketDigest {
            reason: DetachedPacketDigestValidationError::InvalidLength
        })
    ));
}

#[test]
fn detached_digest_long_rejected() {
    assert!(matches!(
        EvaluationArtifactPacketDigest::new(format!(
            "sha256:{}0",
            "427204f744b36a39958c4cc0dffd63f95bf6aba720984f8e32a2a519307f1d17"
        )),
        Err(EvaluationArtifactPacketError::InvalidDetachedPacketDigest {
            reason: DetachedPacketDigestValidationError::InvalidLength
        })
    ));
}

#[test]
fn detached_digest_non_hex_rejected() {
    assert!(matches!(
        EvaluationArtifactPacketDigest::new(
            "sha256:zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"
        ),
        Err(EvaluationArtifactPacketError::InvalidDetachedPacketDigest {
            reason: DetachedPacketDigestValidationError::InvalidHexCharacter
        })
    ));
}

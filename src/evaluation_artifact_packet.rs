#![allow(clippy::too_many_arguments)]

use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::artifact_bundle::{
    ArtifactBundle, ArtifactBundleState, ArtifactBundleValidationError, ArtifactContentDigest,
    ArtifactDescriptor, ArtifactId,
};
use crate::detector_reference_join::{
    DETECTOR_REFERENCE_JOIN_SCHEMA, DetectorReferenceJoin, DetectorReferenceJoinContext,
    DetectorReferenceJoinError, DetectorReferenceJoinState, join_from_json,
};
use crate::detector_snapshot::{
    DetectorProposalSnapshot, DetectorProposalSnapshotValidationError,
    detector_proposal_snapshot_from_json,
};
use crate::human_final_reference::{
    HumanFinalReference, HumanFinalReferenceValidationError, human_final_reference_from_json,
};
use crate::join_adjudication::OverlapAdjudicationValidationError;
use crate::join_adjudication::{OverlapAdjudicationSet, overlap_adjudication_from_json};
use crate::join_metric_aggregation::{
    JOIN_METRIC_AGGREGATION_SCHEMA, JoinMetricAggregateContext, JoinMetricAggregateSet,
    JoinMetricAggregationError, aggregate_from_json,
};
use crate::join_metric_contribution::{
    JOIN_METRIC_CONTRIBUTION_SCHEMA, JoinMetricContributionContext, JoinMetricContributionError,
    JoinMetricContributionSet, MetricContributionSetState, contribution_from_json,
};
use crate::reference_coverage::{
    ReferenceCoverage, ReferenceCoverageValidationError, coverage_from_json,
};
use crate::reference_seal::{ReferenceSeal, ReferenceSealValidationError, seal_from_json};
use crate::run_manifest::{
    ArtifactRole, CalibrationValidityMode, RunEnvelope, RunEnvelopeValidationError,
    RunLifecycleState,
};
use crate::synthetic_evaluation_harness::{
    SyntheticEvaluationCompletionStage, SyntheticEvaluationHarness,
    SyntheticEvaluationHarnessError, SyntheticEvaluationHarnessResult,
};

pub const EVALUATION_ARTIFACT_PACKET_SCHEMA: &str = "voxproof-evaluation-artifact-packet-v1";
pub const PACKET_SERIALIZATION_POLICY: &str = "serde-json-compact-utf8-v1";
pub const EMBEDDED_PAYLOAD_ENCODING_POLICY: &str = "exact-utf8-json-text-v1";
pub const EMBEDDED_PAYLOAD_SERIALIZATION_POLICY: &str = "serde-json-compact-utf8-v1";
pub const EMBEDDED_PAYLOAD_DIGEST_POLICY: &str = "sha256-payload-bytes-v1";
pub const DETACHED_PACKET_DIGEST_POLICY: &str = "sha256-packet-bytes-v1";

const PACKET_ARTIFACT_ROLES: [ArtifactRole; 8] = [
    ArtifactRole::ReferenceSeal,
    ArtifactRole::HumanFinalReference,
    ArtifactRole::CueReviewCompletion,
    ArtifactRole::DetectorOutput,
    ArtifactRole::EvaluationJoin,
    ArtifactRole::JoinAdjudication,
    ArtifactRole::MetricContributions,
    ArtifactRole::Metrics,
];

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EvaluationArtifactPacketDigest(String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationArtifactPacketPayload {
    pub artifact_id: ArtifactId,
    pub payload_utf8_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationArtifactPacket {
    pub schema_revision: String,
    pub packet_serialization_policy: String,
    pub embedded_payload_encoding_policy: String,
    pub embedded_payload_serialization_policy: String,
    pub embedded_payload_digest_policy: String,
    pub completion_stage: SyntheticEvaluationCompletionStage,
    pub detector_execution_envelope: RunEnvelope,
    pub assisted_review_transition_envelope: RunEnvelope,
    pub finalized_envelope: RunEnvelope,
    pub artifact_bundle: ArtifactBundle,
    pub payloads: Vec<EvaluationArtifactPacketPayload>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedEvaluationArtifactPacket {
    pub packet_bytes: Vec<u8>,
    pub content_digest: EvaluationArtifactPacketDigest,
    pub byte_length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedEvaluationArtifactPacket {
    pub packet: EvaluationArtifactPacket,
    pub completion_stage: SyntheticEvaluationCompletionStage,
    pub derivation_lifecycle: RunLifecycleState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluationArtifactPacketError {
    HarnessIntegrity(SyntheticEvaluationHarnessError),
    PacketSerializationFailure,
    PacketDeserializationFailure,
    UnsupportedPacketSchemaRevision,
    UnsupportedPacketPolicy,
    NonCanonicalPacketEncoding,
    DetachedPacketDigestMismatch,
    PacketByteLengthMismatch,
    InvalidLifecycleEnvelope {
        lifecycle_state: RunLifecycleState,
    },
    IllegalPacketLifecycleTransition {
        from: RunLifecycleState,
        to: RunLifecycleState,
    },
    PacketCompletionStageMismatch,
    PacketEnvelopeIdentityMismatch,
    PacketBundleValidation(ArtifactBundleValidationError),
    PacketInventoryMismatch,
    MissingPacketPayload {
        role: ArtifactRole,
    },
    DuplicatePacketPayload {
        artifact_id: ArtifactId,
    },
    UnexpectedPacketPayload {
        artifact_id: ArtifactId,
    },
    NonCanonicalPayloadOrder,
    EmbeddedPayloadNotUtf8 {
        artifact_id: ArtifactId,
    },
    EmbeddedPayloadDigestMismatch {
        artifact_id: ArtifactId,
    },
    EmbeddedPayloadLengthMismatch {
        artifact_id: ArtifactId,
    },
    EmbeddedPayloadTypeDecodeFailure {
        role: ArtifactRole,
    },
    EmbeddedPayloadValidationFailure {
        role: ArtifactRole,
    },
    EmbeddedPayloadReserializationMismatch {
        role: ArtifactRole,
    },
    PacketJoinRederivationMismatch,
    PacketContributionRederivationMismatch,
    PacketAggregationRederivationMismatch,
    PacketHistoricalReplayFailure {
        component: &'static str,
    },
    EnvelopeValidation(RunEnvelopeValidationError),
    SealValidation(ReferenceSealValidationError),
    CoverageValidation(ReferenceCoverageValidationError),
    HumanReferenceValidation(Box<HumanFinalReferenceValidationError>),
    SnapshotValidation(DetectorProposalSnapshotValidationError),
    AdjudicationValidation(OverlapAdjudicationValidationError),
    JoinValidation(DetectorReferenceJoinError),
    ContributionValidation(JoinMetricContributionError),
    AggregationValidation(JoinMetricAggregationError),
    IntegerConversionOverflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DecodedPacketArtifacts {
    reference_seal: ReferenceSeal,
    human_final_reference: HumanFinalReference,
    reference_coverage: ReferenceCoverage,
    detector_snapshot: DetectorProposalSnapshot,
    join: DetectorReferenceJoin,
    adjudication: OverlapAdjudicationSet,
    contributions: JoinMetricContributionSet,
    aggregates: JoinMetricAggregateSet,
}

impl EvaluationArtifactPacketDigest {
    pub fn new(value: impl Into<String>) -> Result<Self, EvaluationArtifactPacketError> {
        let value = value.into();
        if !value.starts_with("sha256:") || value.len() != 71 {
            return Err(EvaluationArtifactPacketError::DetachedPacketDigestMismatch);
        }
        if !value[7..].chars().all(|ch| ch.is_ascii_hexdigit()) {
            return Err(EvaluationArtifactPacketError::DetachedPacketDigestMismatch);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EvaluationArtifactPacketDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub fn build_from_synthetic_harness_result(
    result: &SyntheticEvaluationHarnessResult,
) -> Result<EvaluationArtifactPacket, EvaluationArtifactPacketError> {
    SyntheticEvaluationHarness::verify_payload_integrity(result)
        .map_err(EvaluationArtifactPacketError::HarnessIntegrity)?;
    SyntheticEvaluationHarness::verify_typed_payload_round_trip(result)
        .map_err(EvaluationArtifactPacketError::HarnessIntegrity)?;

    let payloads = build_canonical_payloads(&result.serialized_payloads, &result.final_bundle)?;

    Ok(EvaluationArtifactPacket {
        schema_revision: EVALUATION_ARTIFACT_PACKET_SCHEMA.to_string(),
        packet_serialization_policy: PACKET_SERIALIZATION_POLICY.to_string(),
        embedded_payload_encoding_policy: EMBEDDED_PAYLOAD_ENCODING_POLICY.to_string(),
        embedded_payload_serialization_policy: EMBEDDED_PAYLOAD_SERIALIZATION_POLICY.to_string(),
        embedded_payload_digest_policy: EMBEDDED_PAYLOAD_DIGEST_POLICY.to_string(),
        completion_stage: result.completion_stage,
        detector_execution_envelope: result.detector_execution_envelope.clone(),
        assisted_review_transition_envelope: result.assisted_review_transition_envelope.clone(),
        finalized_envelope: result.finalized_envelope.clone(),
        artifact_bundle: result.final_bundle.clone(),
        payloads,
    })
}

pub fn encode_packet(
    packet: &EvaluationArtifactPacket,
) -> Result<EncodedEvaluationArtifactPacket, EvaluationArtifactPacketError> {
    let packet_bytes = serde_json::to_vec(packet)
        .map_err(|_| EvaluationArtifactPacketError::PacketSerializationFailure)?;
    let content_digest = compute_packet_digest(&packet_bytes)?;
    let byte_length = packet_byte_length(&packet_bytes)?;
    Ok(EncodedEvaluationArtifactPacket {
        packet_bytes,
        content_digest,
        byte_length,
    })
}

pub fn decode_packet(
    packet_bytes: &[u8],
) -> Result<EvaluationArtifactPacket, EvaluationArtifactPacketError> {
    let json = std::str::from_utf8(packet_bytes)
        .map_err(|_| EvaluationArtifactPacketError::PacketDeserializationFailure)?;
    let packet: EvaluationArtifactPacket = serde_json::from_str(json)
        .map_err(|_| EvaluationArtifactPacketError::PacketDeserializationFailure)?;
    validate_packet_policies(&packet)?;
    validate_packet_envelopes(&packet)?;
    validate_completion_stage_consistency(&packet)?;
    validate_canonical_payload_order(&packet)?;
    packet
        .artifact_bundle
        .validate()
        .map_err(EvaluationArtifactPacketError::PacketBundleValidation)?;
    Ok(packet)
}

pub fn decode_and_verify_packet(
    packet_bytes: &[u8],
    expected_digest: Option<&EvaluationArtifactPacketDigest>,
) -> Result<VerifiedEvaluationArtifactPacket, EvaluationArtifactPacketError> {
    if let Some(expected) = expected_digest {
        let actual = compute_packet_digest(packet_bytes)?;
        if actual != *expected {
            return Err(EvaluationArtifactPacketError::DetachedPacketDigestMismatch);
        }
    }

    let packet = decode_packet(packet_bytes)?;
    let decoded = decode_and_validate_embedded_payloads(&packet)?;
    let derivation_lifecycle = derivation_lifecycle_for_completion_stage(packet.completion_stage);
    rederive_and_compare(&packet, &decoded, derivation_lifecycle)?;
    packet_historical_replay_validate(
        &packet.finalized_envelope,
        &decoded,
        &packet.artifact_bundle,
    )?;

    let reserialized = serde_json::to_vec(&packet)
        .map_err(|_| EvaluationArtifactPacketError::PacketSerializationFailure)?;
    if reserialized != packet_bytes {
        return Err(EvaluationArtifactPacketError::NonCanonicalPacketEncoding);
    }

    Ok(VerifiedEvaluationArtifactPacket {
        completion_stage: packet.completion_stage,
        derivation_lifecycle,
        packet,
    })
}

fn build_canonical_payloads(
    serialized: &[crate::synthetic_evaluation_harness::SyntheticSerializedArtifact],
    bundle: &ArtifactBundle,
) -> Result<Vec<EvaluationArtifactPacketPayload>, EvaluationArtifactPacketError> {
    let mut payloads = Vec::with_capacity(PACKET_ARTIFACT_ROLES.len());
    for role in PACKET_ARTIFACT_ROLES {
        let payload = serialized
            .iter()
            .find(|entry| entry.role == role)
            .ok_or(EvaluationArtifactPacketError::MissingPacketPayload { role })?;
        let artifact_id = artifact_id_for_role(bundle, role)?;
        if payload.artifact_id != artifact_id {
            return Err(EvaluationArtifactPacketError::PacketInventoryMismatch);
        }
        let payload_utf8_json = std::str::from_utf8(&payload.payload_bytes).map_err(|_| {
            EvaluationArtifactPacketError::EmbeddedPayloadNotUtf8 {
                artifact_id: payload.artifact_id.clone(),
            }
        })?;
        payloads.push(EvaluationArtifactPacketPayload {
            artifact_id: payload.artifact_id.clone(),
            payload_utf8_json: payload_utf8_json.to_string(),
        });
    }
    Ok(payloads)
}

fn validate_packet_policies(
    packet: &EvaluationArtifactPacket,
) -> Result<(), EvaluationArtifactPacketError> {
    if packet.schema_revision != EVALUATION_ARTIFACT_PACKET_SCHEMA {
        return Err(EvaluationArtifactPacketError::UnsupportedPacketSchemaRevision);
    }
    if packet.packet_serialization_policy != PACKET_SERIALIZATION_POLICY
        || packet.embedded_payload_encoding_policy != EMBEDDED_PAYLOAD_ENCODING_POLICY
        || packet.embedded_payload_serialization_policy != EMBEDDED_PAYLOAD_SERIALIZATION_POLICY
        || packet.embedded_payload_digest_policy != EMBEDDED_PAYLOAD_DIGEST_POLICY
    {
        return Err(EvaluationArtifactPacketError::UnsupportedPacketPolicy);
    }
    Ok(())
}

fn validate_packet_envelopes(
    packet: &EvaluationArtifactPacket,
) -> Result<(), EvaluationArtifactPacketError> {
    let detector = &packet.detector_execution_envelope;
    let assisted = &packet.assisted_review_transition_envelope;
    let finalized = &packet.finalized_envelope;

    for envelope in [detector, assisted, finalized] {
        envelope
            .validate()
            .map_err(EvaluationArtifactPacketError::EnvelopeValidation)?;
    }

    if detector.lifecycle_state != RunLifecycleState::DetectorExecution {
        return Err(EvaluationArtifactPacketError::InvalidLifecycleEnvelope {
            lifecycle_state: detector.lifecycle_state,
        });
    }
    if assisted.lifecycle_state != RunLifecycleState::AssistedReview {
        return Err(EvaluationArtifactPacketError::InvalidLifecycleEnvelope {
            lifecycle_state: assisted.lifecycle_state,
        });
    }
    if finalized.lifecycle_state != RunLifecycleState::Finalized {
        return Err(EvaluationArtifactPacketError::InvalidLifecycleEnvelope {
            lifecycle_state: finalized.lifecycle_state,
        });
    }

    RunEnvelope::validate_transition(
        RunLifecycleState::DetectorExecution,
        RunLifecycleState::AssistedReview,
        CalibrationValidityMode::BlindReference,
    )
    .map_err(
        |_| EvaluationArtifactPacketError::IllegalPacketLifecycleTransition {
            from: RunLifecycleState::DetectorExecution,
            to: RunLifecycleState::AssistedReview,
        },
    )?;

    RunEnvelope::validate_transition(
        RunLifecycleState::AssistedReview,
        RunLifecycleState::Finalized,
        CalibrationValidityMode::BlindReference,
    )
    .map_err(
        |_| EvaluationArtifactPacketError::IllegalPacketLifecycleTransition {
            from: RunLifecycleState::AssistedReview,
            to: RunLifecycleState::Finalized,
        },
    )?;

    for envelope in [detector, assisted, finalized] {
        if envelope.run_id != detector.run_id
            || envelope.input_identity != detector.input_identity
            || envelope.expected_artifact_roles != packet.artifact_bundle.expected_roles
        {
            return Err(EvaluationArtifactPacketError::PacketEnvelopeIdentityMismatch);
        }
    }

    if packet.artifact_bundle.expected_roles != PACKET_ARTIFACT_ROLES.to_vec() {
        return Err(EvaluationArtifactPacketError::PacketInventoryMismatch);
    }

    Ok(())
}

fn validate_completion_stage_consistency(
    packet: &EvaluationArtifactPacket,
) -> Result<(), EvaluationArtifactPacketError> {
    let adjudication = packet
        .payloads
        .iter()
        .find(|entry| {
            packet.artifact_bundle.artifacts.iter().any(|descriptor| {
                descriptor.artifact_id == entry.artifact_id
                    && descriptor.role == ArtifactRole::JoinAdjudication
            })
        })
        .ok_or(EvaluationArtifactPacketError::MissingPacketPayload {
            role: ArtifactRole::JoinAdjudication,
        })?;

    let adjudication_set: OverlapAdjudicationSet =
        overlap_adjudication_from_json(&adjudication.payload_utf8_json).map_err(|_| {
            EvaluationArtifactPacketError::EmbeddedPayloadTypeDecodeFailure {
                role: ArtifactRole::JoinAdjudication,
            }
        })?;

    match packet.completion_stage {
        SyntheticEvaluationCompletionStage::DetectorExecution => {
            if !adjudication_set.records.is_empty() {
                return Err(EvaluationArtifactPacketError::PacketCompletionStageMismatch);
            }
        }
        SyntheticEvaluationCompletionStage::AssistedReview => {
            if adjudication_set.records.is_empty() {
                return Err(EvaluationArtifactPacketError::PacketCompletionStageMismatch);
            }
        }
    }

    Ok(())
}

fn validate_canonical_payload_order(
    packet: &EvaluationArtifactPacket,
) -> Result<(), EvaluationArtifactPacketError> {
    if packet.payloads.len() != PACKET_ARTIFACT_ROLES.len() {
        return Err(EvaluationArtifactPacketError::PacketInventoryMismatch);
    }

    let mut seen = std::collections::HashSet::new();
    for (index, role) in PACKET_ARTIFACT_ROLES.iter().enumerate() {
        let descriptor = packet
            .artifact_bundle
            .artifacts
            .iter()
            .find(|entry| entry.role == *role)
            .ok_or(EvaluationArtifactPacketError::MissingPacketPayload { role: *role })?;
        let payload = &packet.payloads[index];
        if payload.artifact_id != descriptor.artifact_id {
            return Err(EvaluationArtifactPacketError::NonCanonicalPayloadOrder);
        }
        if !seen.insert(payload.artifact_id.clone()) {
            return Err(EvaluationArtifactPacketError::DuplicatePacketPayload {
                artifact_id: payload.artifact_id.clone(),
            });
        }
    }

    Ok(())
}

fn decode_and_validate_embedded_payloads(
    packet: &EvaluationArtifactPacket,
) -> Result<DecodedPacketArtifacts, EvaluationArtifactPacketError> {
    validate_canonical_payload_order(packet)?;

    let mut decoded_roles = [false; 8];
    let mut reference_seal = None;
    let mut human_final_reference = None;
    let mut reference_coverage = None;
    let mut detector_snapshot = None;
    let mut join = None;
    let mut adjudication = None;
    let mut contributions = None;
    let mut aggregates = None;

    for (index, role) in PACKET_ARTIFACT_ROLES.iter().enumerate() {
        let payload = &packet.payloads[index];
        let descriptor = packet
            .artifact_bundle
            .artifacts
            .iter()
            .find(|entry| entry.role == *role)
            .ok_or(EvaluationArtifactPacketError::MissingPacketPayload { role: *role })?;

        if payload.artifact_id != descriptor.artifact_id {
            return Err(EvaluationArtifactPacketError::UnexpectedPacketPayload {
                artifact_id: payload.artifact_id.clone(),
            });
        }

        let payload_bytes = payload.payload_utf8_json.as_bytes();
        if payload_bytes.len() != descriptor.byte_length as usize {
            return Err(
                EvaluationArtifactPacketError::EmbeddedPayloadLengthMismatch {
                    artifact_id: payload.artifact_id.clone(),
                },
            );
        }

        let digest = compute_payload_digest(payload_bytes)?;
        if digest != descriptor.content_digest {
            return Err(
                EvaluationArtifactPacketError::EmbeddedPayloadDigestMismatch {
                    artifact_id: payload.artifact_id.clone(),
                },
            );
        }

        let json = &payload.payload_utf8_json;
        match role {
            ArtifactRole::ReferenceSeal => {
                reference_seal = Some(decode_seal(json, *role)?);
            }
            ArtifactRole::HumanFinalReference => {
                human_final_reference = Some(decode_human_reference(json, *role)?);
            }
            ArtifactRole::CueReviewCompletion => {
                reference_coverage = Some(decode_coverage(json, *role)?);
            }
            ArtifactRole::DetectorOutput => {
                detector_snapshot = Some(decode_snapshot(json, *role)?);
            }
            ArtifactRole::EvaluationJoin => {
                join = Some(decode_join(json, *role)?);
            }
            ArtifactRole::JoinAdjudication => {
                adjudication = Some(decode_adjudication(json, *role)?);
            }
            ArtifactRole::MetricContributions => {
                contributions = Some(decode_contributions(json, *role)?);
            }
            ArtifactRole::Metrics => {
                aggregates = Some(decode_aggregates(json, *role)?);
            }
            _ => {
                return Err(EvaluationArtifactPacketError::UnexpectedPacketPayload {
                    artifact_id: payload.artifact_id.clone(),
                });
            }
        }

        decoded_roles[index] = true;
    }

    if !decoded_roles.iter().all(|seen| *seen) {
        return Err(EvaluationArtifactPacketError::PacketInventoryMismatch);
    }

    let decoded = DecodedPacketArtifacts {
        reference_seal: reference_seal.expect("reference seal"),
        human_final_reference: human_final_reference.expect("human reference"),
        reference_coverage: reference_coverage.expect("coverage"),
        detector_snapshot: detector_snapshot.expect("snapshot"),
        join: join.expect("join"),
        adjudication: adjudication.expect("adjudication"),
        contributions: contributions.expect("contributions"),
        aggregates: aggregates.expect("aggregates"),
    };

    validate_decoded_local(&decoded)?;
    verify_decoded_reserialization(packet, &decoded)?;

    Ok(decoded)
}

fn decode_seal(
    json: &str,
    role: ArtifactRole,
) -> Result<ReferenceSeal, EvaluationArtifactPacketError> {
    seal_from_json(json)
        .map_err(|_| EvaluationArtifactPacketError::EmbeddedPayloadTypeDecodeFailure { role })
}

fn decode_human_reference(
    json: &str,
    role: ArtifactRole,
) -> Result<HumanFinalReference, EvaluationArtifactPacketError> {
    human_final_reference_from_json(json)
        .map_err(|_| EvaluationArtifactPacketError::EmbeddedPayloadTypeDecodeFailure { role })
}

fn decode_coverage(
    json: &str,
    role: ArtifactRole,
) -> Result<ReferenceCoverage, EvaluationArtifactPacketError> {
    coverage_from_json(json)
        .map_err(|_| EvaluationArtifactPacketError::EmbeddedPayloadTypeDecodeFailure { role })
}

fn decode_snapshot(
    json: &str,
    role: ArtifactRole,
) -> Result<DetectorProposalSnapshot, EvaluationArtifactPacketError> {
    detector_proposal_snapshot_from_json(json)
        .map_err(|_| EvaluationArtifactPacketError::EmbeddedPayloadTypeDecodeFailure { role })
}

fn decode_join(
    json: &str,
    role: ArtifactRole,
) -> Result<DetectorReferenceJoin, EvaluationArtifactPacketError> {
    join_from_json(json)
        .map_err(|_| EvaluationArtifactPacketError::EmbeddedPayloadTypeDecodeFailure { role })
}

fn decode_adjudication(
    json: &str,
    role: ArtifactRole,
) -> Result<OverlapAdjudicationSet, EvaluationArtifactPacketError> {
    overlap_adjudication_from_json(json)
        .map_err(|_| EvaluationArtifactPacketError::EmbeddedPayloadTypeDecodeFailure { role })
}

fn decode_contributions(
    json: &str,
    role: ArtifactRole,
) -> Result<JoinMetricContributionSet, EvaluationArtifactPacketError> {
    contribution_from_json(json)
        .map_err(|_| EvaluationArtifactPacketError::EmbeddedPayloadTypeDecodeFailure { role })
}

fn decode_aggregates(
    json: &str,
    role: ArtifactRole,
) -> Result<JoinMetricAggregateSet, EvaluationArtifactPacketError> {
    aggregate_from_json(json)
        .map_err(|_| EvaluationArtifactPacketError::EmbeddedPayloadTypeDecodeFailure { role })
}

fn validate_decoded_local(
    decoded: &DecodedPacketArtifacts,
) -> Result<(), EvaluationArtifactPacketError> {
    decoded.reference_seal.validate().map_err(|_| {
        EvaluationArtifactPacketError::EmbeddedPayloadValidationFailure {
            role: ArtifactRole::ReferenceSeal,
        }
    })?;
    decoded.human_final_reference.validate().map_err(|_| {
        EvaluationArtifactPacketError::EmbeddedPayloadValidationFailure {
            role: ArtifactRole::HumanFinalReference,
        }
    })?;
    decoded.reference_coverage.validate().map_err(|_| {
        EvaluationArtifactPacketError::EmbeddedPayloadValidationFailure {
            role: ArtifactRole::CueReviewCompletion,
        }
    })?;
    decoded.detector_snapshot.validate().map_err(|_| {
        EvaluationArtifactPacketError::EmbeddedPayloadValidationFailure {
            role: ArtifactRole::DetectorOutput,
        }
    })?;
    decoded.join.validate().map_err(|_| {
        EvaluationArtifactPacketError::EmbeddedPayloadValidationFailure {
            role: ArtifactRole::EvaluationJoin,
        }
    })?;
    decoded.adjudication.validate().map_err(|_| {
        EvaluationArtifactPacketError::EmbeddedPayloadValidationFailure {
            role: ArtifactRole::JoinAdjudication,
        }
    })?;
    decoded.contributions.validate().map_err(|_| {
        EvaluationArtifactPacketError::EmbeddedPayloadValidationFailure {
            role: ArtifactRole::MetricContributions,
        }
    })?;
    decoded.aggregates.validate().map_err(|_| {
        EvaluationArtifactPacketError::EmbeddedPayloadValidationFailure {
            role: ArtifactRole::Metrics,
        }
    })?;
    Ok(())
}

fn verify_decoded_reserialization(
    packet: &EvaluationArtifactPacket,
    decoded: &DecodedPacketArtifacts,
) -> Result<(), EvaluationArtifactPacketError> {
    let role_values = [
        (
            ArtifactRole::ReferenceSeal,
            serde_json::to_vec(&decoded.reference_seal),
        ),
        (
            ArtifactRole::HumanFinalReference,
            serde_json::to_vec(&decoded.human_final_reference),
        ),
        (
            ArtifactRole::CueReviewCompletion,
            serde_json::to_vec(&decoded.reference_coverage),
        ),
        (
            ArtifactRole::DetectorOutput,
            serde_json::to_vec(&decoded.detector_snapshot),
        ),
        (
            ArtifactRole::EvaluationJoin,
            serde_json::to_vec(&decoded.join),
        ),
        (
            ArtifactRole::JoinAdjudication,
            serde_json::to_vec(&decoded.adjudication),
        ),
        (
            ArtifactRole::MetricContributions,
            serde_json::to_vec(&decoded.contributions),
        ),
        (
            ArtifactRole::Metrics,
            serde_json::to_vec(&decoded.aggregates),
        ),
    ];

    for (index, (role, reserialized)) in role_values.into_iter().enumerate() {
        let reserialized =
            reserialized.map_err(|_| EvaluationArtifactPacketError::PacketSerializationFailure)?;
        let embedded = packet.payloads[index].payload_utf8_json.as_bytes();
        if reserialized.as_slice() != embedded {
            return Err(
                EvaluationArtifactPacketError::EmbeddedPayloadReserializationMismatch { role },
            );
        }
    }

    Ok(())
}

fn derivation_lifecycle_for_completion_stage(
    stage: SyntheticEvaluationCompletionStage,
) -> RunLifecycleState {
    match stage {
        SyntheticEvaluationCompletionStage::DetectorExecution => {
            RunLifecycleState::DetectorExecution
        }
        SyntheticEvaluationCompletionStage::AssistedReview => RunLifecycleState::AssistedReview,
    }
}

fn derivation_envelope_for_packet(
    packet: &EvaluationArtifactPacket,
    lifecycle: RunLifecycleState,
) -> Result<&RunEnvelope, EvaluationArtifactPacketError> {
    match lifecycle {
        RunLifecycleState::DetectorExecution => Ok(&packet.detector_execution_envelope),
        RunLifecycleState::AssistedReview => Ok(&packet.assisted_review_transition_envelope),
        other => Err(EvaluationArtifactPacketError::InvalidLifecycleEnvelope {
            lifecycle_state: other,
        }),
    }
}

fn join_context_from_join(join: &DetectorReferenceJoin) -> DetectorReferenceJoinContext {
    DetectorReferenceJoinContext {
        join_id: join.join_id.clone(),
        join_revision: join.join_revision.clone(),
        evaluation_join_artifact_id: join.evaluation_join_artifact_id.clone(),
        join_adjudication_artifact_id: join.join_adjudication_artifact_id.clone(),
    }
}

fn contribution_context_from_contributions(
    contributions: &JoinMetricContributionSet,
) -> JoinMetricContributionContext {
    JoinMetricContributionContext {
        contribution_set_id: contributions.contribution_set_id.clone(),
        contribution_revision: contributions.contribution_revision.clone(),
        metric_contributions_artifact_id: contributions.metric_contributions_artifact_id.clone(),
    }
}

fn aggregate_context_from_aggregates(
    aggregates: &JoinMetricAggregateSet,
) -> JoinMetricAggregateContext {
    JoinMetricAggregateContext {
        aggregate_set_id: aggregates.aggregate_set_id.clone(),
        aggregate_revision: aggregates.aggregate_revision.clone(),
        metrics_artifact_id: aggregates.metrics_artifact_id.clone(),
    }
}

fn rederive_and_compare(
    packet: &EvaluationArtifactPacket,
    decoded: &DecodedPacketArtifacts,
    derivation_lifecycle: RunLifecycleState,
) -> Result<(), EvaluationArtifactPacketError> {
    let envelope = derivation_envelope_for_packet(packet, derivation_lifecycle)?;
    let join_context = join_context_from_join(&decoded.join);
    let contribution_context = contribution_context_from_contributions(&decoded.contributions);
    let aggregate_context = aggregate_context_from_aggregates(&decoded.aggregates);

    let bootstrap_join = build_rederivation_bootstrap_bundle(packet, decoded, None, None, None)?;
    let rederived_join = DetectorReferenceJoin::derive(
        &join_context,
        envelope,
        &decoded.reference_seal,
        &decoded.reference_coverage,
        &decoded.human_final_reference,
        &decoded.detector_snapshot,
        &bootstrap_join,
        &decoded.adjudication,
    )
    .map_err(EvaluationArtifactPacketError::JoinValidation)?;

    if rederived_join != decoded.join {
        return Err(EvaluationArtifactPacketError::PacketJoinRederivationMismatch);
    }

    let bootstrap_contributions =
        build_rederivation_bootstrap_bundle(packet, decoded, Some(&rederived_join), None, None)?;
    let rederived_contributions = JoinMetricContributionSet::derive(
        &contribution_context,
        envelope,
        &decoded.reference_seal,
        &decoded.reference_coverage,
        &decoded.human_final_reference,
        &decoded.detector_snapshot,
        &rederived_join,
        &decoded.adjudication,
        &bootstrap_contributions,
    )
    .map_err(EvaluationArtifactPacketError::ContributionValidation)?;

    if rederived_contributions != decoded.contributions {
        return Err(EvaluationArtifactPacketError::PacketContributionRederivationMismatch);
    }

    let bootstrap_aggregates = build_rederivation_bootstrap_bundle(
        packet,
        decoded,
        Some(&rederived_join),
        Some(&rederived_contributions),
        None,
    )?;
    let rederived_aggregates = JoinMetricAggregateSet::derive(
        &aggregate_context,
        envelope,
        &decoded.reference_seal,
        &decoded.reference_coverage,
        &decoded.human_final_reference,
        &decoded.detector_snapshot,
        &rederived_join,
        &decoded.adjudication,
        &rederived_contributions,
        &bootstrap_aggregates,
    )
    .map_err(EvaluationArtifactPacketError::AggregationValidation)?;

    if rederived_aggregates != decoded.aggregates {
        return Err(EvaluationArtifactPacketError::PacketAggregationRederivationMismatch);
    }

    Ok(())
}

fn packet_historical_replay_validate(
    finalized_envelope: &RunEnvelope,
    decoded: &DecodedPacketArtifacts,
    bundle: &ArtifactBundle,
) -> Result<(), EvaluationArtifactPacketError> {
    decoded
        .reference_seal
        .validate_historical_context(finalized_envelope)
        .map_err(
            |_| EvaluationArtifactPacketError::PacketHistoricalReplayFailure {
                component: "reference_seal",
            },
        )?;
    decoded
        .reference_coverage
        .validate_historical_context(
            finalized_envelope,
            &decoded.reference_seal,
            Some(&decoded.human_final_reference),
        )
        .map_err(
            |_| EvaluationArtifactPacketError::PacketHistoricalReplayFailure {
                component: "reference_coverage",
            },
        )?;
    decoded
        .human_final_reference
        .validate_historical_context(finalized_envelope, &decoded.reference_seal)
        .map_err(
            |_| EvaluationArtifactPacketError::PacketHistoricalReplayFailure {
                component: "human_final_reference",
            },
        )?;
    decoded
        .detector_snapshot
        .validate_against_bundle(finalized_envelope, bundle)
        .map_err(
            |_| EvaluationArtifactPacketError::PacketHistoricalReplayFailure {
                component: "detector_snapshot",
            },
        )?;
    decoded
        .adjudication
        .validate_against_envelope(finalized_envelope)
        .map_err(
            |_| EvaluationArtifactPacketError::PacketHistoricalReplayFailure {
                component: "adjudication_set",
            },
        )?;
    decoded
        .join
        .validate_against(
            finalized_envelope,
            &decoded.reference_seal,
            &decoded.reference_coverage,
            &decoded.human_final_reference,
            &decoded.detector_snapshot,
            bundle,
            &decoded.adjudication,
        )
        .map_err(
            |_| EvaluationArtifactPacketError::PacketHistoricalReplayFailure { component: "join" },
        )?;
    decoded
        .contributions
        .validate_against(
            finalized_envelope,
            &decoded.reference_seal,
            &decoded.reference_coverage,
            &decoded.human_final_reference,
            &decoded.detector_snapshot,
            &decoded.join,
            &decoded.adjudication,
            bundle,
        )
        .map_err(
            |_| EvaluationArtifactPacketError::PacketHistoricalReplayFailure {
                component: "contributions",
            },
        )?;
    decoded
        .aggregates
        .validate_against(
            finalized_envelope,
            &decoded.reference_seal,
            &decoded.reference_coverage,
            &decoded.human_final_reference,
            &decoded.detector_snapshot,
            &decoded.join,
            &decoded.adjudication,
            &decoded.contributions,
            bundle,
        )
        .map_err(
            |_| EvaluationArtifactPacketError::PacketHistoricalReplayFailure {
                component: "aggregates",
            },
        )?;
    bundle
        .validate_with_reference_context(
            finalized_envelope,
            Some(&decoded.reference_seal),
            Some(&decoded.reference_coverage),
            Some(&decoded.human_final_reference),
        )
        .map_err(
            |_| EvaluationArtifactPacketError::PacketHistoricalReplayFailure {
                component: "artifact_bundle",
            },
        )?;
    Ok(())
}

fn build_rederivation_bootstrap_bundle(
    packet: &EvaluationArtifactPacket,
    decoded: &DecodedPacketArtifacts,
    join: Option<&DetectorReferenceJoin>,
    contributions: Option<&JoinMetricContributionSet>,
    aggregates: Option<&JoinMetricAggregateSet>,
) -> Result<ArtifactBundle, EvaluationArtifactPacketError> {
    let join = join
        .cloned()
        .unwrap_or_else(|| bootstrap_stub_join(decoded, &packet.artifact_bundle));
    let contributions = contributions
        .cloned()
        .unwrap_or_else(|| bootstrap_stub_contributions(decoded, &packet.artifact_bundle, &join));
    let aggregates = aggregates.cloned().unwrap_or_else(|| {
        bootstrap_stub_aggregates(decoded, &packet.artifact_bundle, &join, &contributions)
    });

    let binding_context = packet.artifact_bundle.binding_context.clone();
    let mut descriptors = Vec::with_capacity(PACKET_ARTIFACT_ROLES.len());

    let serialized = [
        (
            ArtifactRole::ReferenceSeal,
            serde_json::to_vec(&decoded.reference_seal),
        ),
        (
            ArtifactRole::HumanFinalReference,
            serde_json::to_vec(&decoded.human_final_reference),
        ),
        (
            ArtifactRole::CueReviewCompletion,
            serde_json::to_vec(&decoded.reference_coverage),
        ),
        (
            ArtifactRole::DetectorOutput,
            serde_json::to_vec(&decoded.detector_snapshot),
        ),
        (ArtifactRole::EvaluationJoin, serde_json::to_vec(&join)),
        (
            ArtifactRole::JoinAdjudication,
            serde_json::to_vec(&decoded.adjudication),
        ),
        (
            ArtifactRole::MetricContributions,
            serde_json::to_vec(&contributions),
        ),
        (ArtifactRole::Metrics, serde_json::to_vec(&aggregates)),
    ];

    for (role, payload_bytes) in serialized {
        let payload_bytes =
            payload_bytes.map_err(|_| EvaluationArtifactPacketError::PacketSerializationFailure)?;
        let content_digest = compute_payload_digest(&payload_bytes)?;
        let byte_length = payload_byte_length(&payload_bytes)?;
        let descriptor = packet
            .artifact_bundle
            .artifacts
            .iter()
            .find(|entry| entry.role == role)
            .ok_or(EvaluationArtifactPacketError::MissingPacketPayload { role })?;
        let artifact_id = descriptor.artifact_id.clone();
        descriptors.push(ArtifactDescriptor {
            artifact_id,
            role,
            payload_schema: descriptor.payload_schema.clone(),
            content_digest,
            byte_length,
            binding_context: binding_context.clone(),
        });
    }

    descriptors.sort_by_key(|descriptor| descriptor.role);

    let assessment = crate::artifact_bundle::ArtifactBundle::derive_assessment(
        &PACKET_ARTIFACT_ROLES,
        &descriptors,
        &binding_context,
    )
    .map_err(EvaluationArtifactPacketError::PacketBundleValidation)?;

    let bundle = ArtifactBundle {
        schema_revision: packet.artifact_bundle.schema_revision.clone(),
        bundle_id: packet.artifact_bundle.bundle_id.clone(),
        binding_context,
        expected_roles: PACKET_ARTIFACT_ROLES.to_vec(),
        artifacts: descriptors,
        bundle_state: ArtifactBundleState::Complete,
        assessment,
    };
    bundle
        .validate()
        .map_err(EvaluationArtifactPacketError::PacketBundleValidation)?;
    Ok(bundle)
}

fn bootstrap_stub_join(
    decoded: &DecodedPacketArtifacts,
    bundle: &ArtifactBundle,
) -> DetectorReferenceJoin {
    DetectorReferenceJoin {
        schema_revision: DETECTOR_REFERENCE_JOIN_SCHEMA.to_string(),
        join_id: decoded.join.join_id.clone(),
        join_revision: decoded.join.join_revision.clone(),
        run_id: decoded.reference_seal.run_id.clone(),
        input_identity: decoded.reference_seal.input_identity.clone(),
        calibration_validity: CalibrationValidityMode::BlindReference,
        reference_seal_id: decoded.reference_seal.seal_id.clone(),
        reference_revision: decoded.reference_seal.reference_revision.clone(),
        reference_coverage_id: decoded.reference_coverage.coverage_id.clone(),
        detector_snapshot_revision: decoded.detector_snapshot.snapshot_revision.clone(),
        detector_output_artifact_id: artifact_id_for_role(bundle, ArtifactRole::DetectorOutput)
            .expect("descriptor"),
        evaluation_join_artifact_id: decoded.join.evaluation_join_artifact_id.clone(),
        join_adjudication_artifact_id: decoded.join.join_adjudication_artifact_id.clone(),
        overlap_rule_revision: decoded.join.overlap_rule_revision.clone(),
        correction_equality_revision: decoded.join.correction_equality_revision.clone(),
        alternative_cardinality_policy: decoded.join.alternative_cardinality_policy.clone(),
        join_purpose: decoded.join.join_purpose,
        state: DetectorReferenceJoinState::Draft,
        edges: Vec::new(),
        detector_dispositions: Vec::new(),
        reference_dispositions: Vec::new(),
        assessment: crate::detector_reference_join::DetectorReferenceJoinAssessment {
            detector_proposal_count: 0,
            reference_record_count: 0,
            recall_eligible_reference_count: 0,
            exact_match_count: 0,
            accepted_overlap_count: 0,
            detector_wrong_correction_count: 0,
            duplicate_proposal_count: 0,
            unmatched_detector_count: 0,
            unmatched_reference_count: 0,
            ambiguous_match_count: 0,
            excluded_reference_count: 0,
            unresolved_overlap_edge_count: 0,
            detector_primary_assignment_count: 0,
            reference_primary_assignment_count: 0,
            one_to_one_consistent: true,
            fully_resolved: false,
        },
    }
}

fn bootstrap_stub_contributions(
    decoded: &DecodedPacketArtifacts,
    bundle: &ArtifactBundle,
    join: &DetectorReferenceJoin,
) -> JoinMetricContributionSet {
    JoinMetricContributionSet {
        schema_revision: JOIN_METRIC_CONTRIBUTION_SCHEMA.to_string(),
        contribution_set_id: decoded.contributions.contribution_set_id.clone(),
        contribution_revision: decoded.contributions.contribution_revision.clone(),
        run_id: decoded.reference_seal.run_id.clone(),
        input_identity: decoded.reference_seal.input_identity.clone(),
        input_class: decoded.contributions.input_class,
        qualifies_as_real_material_evidence: false,
        reference_seal_id: decoded.reference_seal.seal_id.clone(),
        reference_revision: decoded.reference_seal.reference_revision.clone(),
        reference_coverage_id: decoded.reference_coverage.coverage_id.clone(),
        detector_snapshot_revision: decoded.detector_snapshot.snapshot_revision.clone(),
        detector_output_artifact_id: artifact_id_for_role(bundle, ArtifactRole::DetectorOutput)
            .expect("descriptor"),
        join_id: join.join_id.clone(),
        join_revision: join.join_revision.clone(),
        evaluation_join_artifact_id: join.evaluation_join_artifact_id.clone(),
        join_adjudication_artifact_id: join.join_adjudication_artifact_id.clone(),
        metric_contributions_artifact_id: decoded
            .contributions
            .metric_contributions_artifact_id
            .clone(),
        eligibility_policy_revision: decoded.contributions.eligibility_policy_revision.clone(),
        contribution_policy_revision: decoded.contributions.contribution_policy_revision.clone(),
        state: MetricContributionSetState::PendingJoinResolution,
        eligibility: decoded.contributions.eligibility.clone(),
        detector_contributions: Vec::new(),
        reference_contributions: Vec::new(),
        assessment: crate::join_metric_contribution::MetricContributionSetAssessment {
            detector_source_count: 0,
            detector_contribution_count: 0,
            reference_source_count: 0,
            reference_contribution_count: 0,
            pending_detector_contribution_count: 0,
            pending_reference_contribution_count: 0,
            mapping_complete: false,
        },
    }
}

fn bootstrap_stub_aggregates(
    decoded: &DecodedPacketArtifacts,
    bundle: &ArtifactBundle,
    join: &DetectorReferenceJoin,
    contributions: &JoinMetricContributionSet,
) -> JoinMetricAggregateSet {
    JoinMetricAggregateSet {
        schema_revision: JOIN_METRIC_AGGREGATION_SCHEMA.to_string(),
        aggregate_set_id: decoded.aggregates.aggregate_set_id.clone(),
        aggregate_revision: decoded.aggregates.aggregate_revision.clone(),
        run_id: decoded.reference_seal.run_id.clone(),
        input_identity: decoded.reference_seal.input_identity.clone(),
        input_class: decoded.aggregates.input_class,
        qualifies_as_real_material_evidence: false,
        reference_seal_id: decoded.reference_seal.seal_id.clone(),
        reference_revision: decoded.reference_seal.reference_revision.clone(),
        reference_coverage_id: decoded.reference_coverage.coverage_id.clone(),
        detector_snapshot_revision: decoded.detector_snapshot.snapshot_revision.clone(),
        detector_output_artifact_id: artifact_id_for_role(bundle, ArtifactRole::DetectorOutput)
            .expect("descriptor"),
        join_id: join.join_id.clone(),
        join_revision: join.join_revision.clone(),
        evaluation_join_artifact_id: join.evaluation_join_artifact_id.clone(),
        join_adjudication_artifact_id: join.join_adjudication_artifact_id.clone(),
        contribution_set_id: contributions.contribution_set_id.clone(),
        contribution_revision: contributions.contribution_revision.clone(),
        metric_contributions_artifact_id: contributions.metric_contributions_artifact_id.clone(),
        metrics_artifact_id: decoded.aggregates.metrics_artifact_id.clone(),
        aggregation_policy_revision: decoded.aggregates.aggregation_policy_revision.clone(),
        zero_denominator_policy_revision: decoded
            .aggregates
            .zero_denominator_policy_revision
            .clone(),
        report_class: decoded.aggregates.report_class,
        primary_metrics_allowed: false,
        eligible_primary_metrics: Vec::new(),
        blocking_reasons: Vec::new(),
        qualifies_as_primary_metric_evidence: false,
        state: crate::join_metric_aggregation::MetricAggregateSetState::Complete,
        metrics: Vec::new(),
        assessment: crate::join_metric_aggregation::MetricAggregateSetAssessment {
            required_metric_count: 5,
            aggregate_metric_count: 0,
            defined_metric_count: 0,
            undefined_zero_denominator_count: 0,
            all_required_metrics_present: false,
            aggregate_consistent: false,
        },
    }
}

fn artifact_id_for_role(
    bundle: &ArtifactBundle,
    role: ArtifactRole,
) -> Result<ArtifactId, EvaluationArtifactPacketError> {
    bundle
        .artifacts
        .iter()
        .find(|descriptor| descriptor.role == role)
        .map(|descriptor| descriptor.artifact_id.clone())
        .ok_or(EvaluationArtifactPacketError::MissingPacketPayload { role })
}

fn compute_packet_digest(
    bytes: &[u8],
) -> Result<EvaluationArtifactPacketDigest, EvaluationArtifactPacketError> {
    let hash = Sha256::digest(bytes);
    let hex = hash
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    EvaluationArtifactPacketDigest::new(format!("sha256:{hex}"))
}

fn compute_payload_digest(
    bytes: &[u8],
) -> Result<ArtifactContentDigest, EvaluationArtifactPacketError> {
    let hash = Sha256::digest(bytes);
    let hex = hash
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    ArtifactContentDigest::new(format!("sha256:{hex}")).map_err(|_| {
        EvaluationArtifactPacketError::EmbeddedPayloadDigestMismatch {
            artifact_id: ArtifactId::new("unknown").expect("artifact id"),
        }
    })
}

fn packet_byte_length(bytes: &[u8]) -> Result<u64, EvaluationArtifactPacketError> {
    u64::try_from(bytes.len()).map_err(|_| EvaluationArtifactPacketError::IntegerConversionOverflow)
}

fn payload_byte_length(bytes: &[u8]) -> Result<u64, EvaluationArtifactPacketError> {
    u64::try_from(bytes.len()).map_err(|_| EvaluationArtifactPacketError::IntegerConversionOverflow)
}

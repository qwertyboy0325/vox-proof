#![allow(clippy::too_many_arguments)]

#[path = "synthetic_evaluation_harness_fixtures.rs"]
#[allow(dead_code)]
mod fixtures;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use fixtures::{
    exact_only_multi_disposition_fixture, overlap_pending_then_resolved_fixture,
    zero_population_fixture,
};
use sha2::{Digest, Sha256};
use vox_proof::artifact_bundle::ArtifactContentDigest;
use vox_proof::evaluation_artifact_packet::{
    EncodedEvaluationArtifactPacket, EvaluationArtifactPacketDigest, EvaluationArtifactPacketError,
    build_from_synthetic_harness_result, decode_and_verify_packet, decode_packet, encode_packet,
};
use vox_proof::evaluation_artifact_packet_file::{
    EvaluationArtifactPacketFileError, EvaluationArtifactPacketFileLimits,
    read_and_verify_packet_file, write_encoded_packet_file_create_new,
};
use vox_proof::run_manifest::ArtifactRole;
use vox_proof::synthetic_evaluation_harness::SyntheticEvaluationHarness;

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

const EXACT_ONLY_PACKET_BYTE_LENGTH: u64 = 37_615;
const EXACT_ONLY_PACKET_DIGEST: &str =
    "sha256:427204f744b36a39958c4cc0dffd63f95bf6aba720984f8e32a2a519307f1d17";

const OVERLAP_PACKET_BYTE_LENGTH: u64 = 22_896;
const OVERLAP_PACKET_DIGEST: &str =
    "sha256:182e433be44f67115e0c666ad95b92d553c55da969cdf08705ef4ba5340584df";

const ZERO_POPULATION_PACKET_BYTE_LENGTH: u64 = 19_393;
const ZERO_POPULATION_PACKET_DIGEST: &str =
    "sha256:09620f8c3bbe0a8a8339f8ab99a9074ac982894052b6c7052816e3f37c2575c8";

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new() -> Self {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "voxproof-packet-file-test-{}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create test dir");
        Self { path }
    }

    fn file(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn build_encoded(
    fixture: vox_proof::synthetic_evaluation_harness::SyntheticEvaluationFixture,
) -> EncodedEvaluationArtifactPacket {
    let result = SyntheticEvaluationHarness::execute(&fixture).expect("execute");
    let packet = build_from_synthetic_harness_result(&result).expect("build");
    encode_packet(&packet).expect("encode")
}

fn default_limits(encoded: &EncodedEvaluationArtifactPacket) -> EvaluationArtifactPacketFileLimits {
    EvaluationArtifactPacketFileLimits::new(encoded.byte_length.saturating_add(1024))
        .expect("limits")
}

fn write_raw(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).expect("write raw");
}

#[test]
fn exact_only_write_read_round_trip() {
    let dir = TestDir::new();
    let encoded = build_encoded(exact_only_multi_disposition_fixture());
    let limits = default_limits(&encoded);
    let path = dir.file("exact-only.packet");

    let receipt = write_encoded_packet_file_create_new(&path, &encoded, limits).expect("write");
    assert_eq!(receipt.byte_length, EXACT_ONLY_PACKET_BYTE_LENGTH);
    assert_eq!(receipt.content_digest.as_str(), EXACT_ONLY_PACKET_DIGEST);
    assert_eq!(fs::read(&path).expect("read disk"), encoded.packet_bytes);

    let loaded =
        read_and_verify_packet_file(&path, Some(&encoded.content_digest), limits).expect("read");
    assert_eq!(loaded.byte_length, EXACT_ONLY_PACKET_BYTE_LENGTH);
    assert_eq!(loaded.content_digest.as_str(), EXACT_ONLY_PACKET_DIGEST);
    decode_and_verify_packet(&encoded.packet_bytes, Some(&encoded.content_digest))
        .expect("baseline");
}

#[test]
fn overlap_write_read_round_trip() {
    let dir = TestDir::new();
    let encoded = build_encoded(overlap_pending_then_resolved_fixture());
    let limits = default_limits(&encoded);
    let path = dir.file("overlap.packet");

    write_encoded_packet_file_create_new(&path, &encoded, limits).expect("write");
    read_and_verify_packet_file(&path, Some(&encoded.content_digest), limits).expect("read");
    assert_eq!(encoded.byte_length, OVERLAP_PACKET_BYTE_LENGTH);
    assert_eq!(encoded.content_digest.as_str(), OVERLAP_PACKET_DIGEST);
}

#[test]
fn zero_population_write_read_round_trip() {
    let dir = TestDir::new();
    let encoded = build_encoded(zero_population_fixture());
    let limits = default_limits(&encoded);
    let path = dir.file("zero-population.packet");

    write_encoded_packet_file_create_new(&path, &encoded, limits).expect("write");
    read_and_verify_packet_file(&path, Some(&encoded.content_digest), limits).expect("read");
    assert_eq!(encoded.byte_length, ZERO_POPULATION_PACKET_BYTE_LENGTH);
    assert_eq!(
        encoded.content_digest.as_str(),
        ZERO_POPULATION_PACKET_DIGEST
    );
}

#[test]
fn read_without_expected_digest_still_verifies() {
    let dir = TestDir::new();
    let encoded = build_encoded(exact_only_multi_disposition_fixture());
    let limits = default_limits(&encoded);
    let path = dir.file("no-digest-read.packet");

    write_encoded_packet_file_create_new(&path, &encoded, limits).expect("write");
    read_and_verify_packet_file(&path, None, limits).expect("read without digest");
}

#[test]
fn existing_destination_is_not_overwritten() {
    let dir = TestDir::new();
    let encoded = build_encoded(exact_only_multi_disposition_fixture());
    let limits = default_limits(&encoded);
    let path = dir.file("existing.packet");
    let sentinel = b"sentinel-bytes-must-remain-unchanged";

    write_raw(&path, sentinel);

    let result = write_encoded_packet_file_create_new(&path, &encoded, limits);
    assert!(matches!(
        result,
        Err(EvaluationArtifactPacketFileError::DestinationAlreadyExists)
    ));
    assert_eq!(fs::read(&path).expect("read sentinel"), sentinel);
}

#[test]
fn wrong_encoded_byte_length_fails_before_creation() {
    let dir = TestDir::new();
    let mut encoded = build_encoded(exact_only_multi_disposition_fixture());
    encoded.byte_length = encoded.byte_length.saturating_add(1);
    let limits = default_limits(&encoded);
    let path = dir.file("should-not-exist.packet");

    assert!(matches!(
        write_encoded_packet_file_create_new(&path, &encoded, limits),
        Err(EvaluationArtifactPacketFileError::EncodedPacketLengthMismatch)
    ));
    assert!(!path.exists());
}

#[test]
fn wrong_detached_digest_fails_before_creation() {
    let dir = TestDir::new();
    let mut encoded = build_encoded(exact_only_multi_disposition_fixture());
    encoded.content_digest =
        EvaluationArtifactPacketDigest::new(OVERLAP_PACKET_DIGEST).expect("digest");
    let limits = default_limits(&encoded);
    let path = dir.file("should-not-exist-2.packet");

    assert!(matches!(
        write_encoded_packet_file_create_new(&path, &encoded, limits),
        Err(
            EvaluationArtifactPacketFileError::EncodedPacketVerificationFailure(
                EvaluationArtifactPacketError::DetachedPacketDigestMismatch
            )
        )
    ));
    assert!(!path.exists());
}

#[test]
fn malformed_packet_bytes_fail_before_creation() {
    let dir = TestDir::new();
    let packet_bytes = b"{not-json".to_vec();
    let encoded = EncodedEvaluationArtifactPacket {
        packet_bytes: packet_bytes.clone(),
        content_digest: EvaluationArtifactPacketDigest::new(
            "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        )
        .expect("digest"),
        byte_length: packet_bytes.len() as u64,
    };
    let limits = EvaluationArtifactPacketFileLimits::new(1024).expect("limits");
    let path = dir.file("should-not-exist-3.packet");

    let result = write_encoded_packet_file_create_new(&path, &encoded, limits);
    assert!(
        matches!(
            result,
            Err(EvaluationArtifactPacketFileError::EncodedPacketVerificationFailure(_))
        ),
        "unexpected result: {result:?}"
    );
    assert!(!path.exists());
}

#[test]
fn non_canonical_packet_bytes_fail_before_creation() {
    let dir = TestDir::new();
    let encoded = build_encoded(exact_only_multi_disposition_fixture());
    let value: serde_json::Value = serde_json::from_slice(&encoded.packet_bytes).expect("json");
    let pretty = serde_json::to_vec_pretty(&value).expect("pretty");
    let bad = EncodedEvaluationArtifactPacket {
        packet_bytes: pretty.clone(),
        content_digest: EvaluationArtifactPacketDigest::new(format!(
            "sha256:{}",
            Sha256::digest(&pretty)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ))
        .expect("digest"),
        byte_length: pretty.len() as u64,
    };
    let limits = default_limits(&bad);
    let path = dir.file("should-not-exist-4.packet");

    assert!(matches!(
        write_encoded_packet_file_create_new(&path, &bad, limits),
        Err(
            EvaluationArtifactPacketFileError::EncodedPacketVerificationFailure(
                EvaluationArtifactPacketError::NonCanonicalPacketEncoding
            )
        )
    ));
    assert!(!path.exists());
}

#[test]
fn encoded_packet_exceeding_limit_fails_before_creation() {
    let dir = TestDir::new();
    let encoded = build_encoded(exact_only_multi_disposition_fixture());
    let limits = EvaluationArtifactPacketFileLimits::new(encoded.byte_length.saturating_sub(1))
        .expect("limits");
    let path = dir.file("should-not-exist-5.packet");

    assert!(matches!(
        write_encoded_packet_file_create_new(&path, &encoded, limits),
        Err(EvaluationArtifactPacketFileError::EncodedPacketExceedsLimit)
    ));
    assert!(!path.exists());
}

#[test]
fn existing_file_one_byte_below_limit_rejected() {
    let dir = TestDir::new();
    let encoded = build_encoded(exact_only_multi_disposition_fixture());
    let write_limits = default_limits(&encoded);
    let path = dir.file("one-byte-below-limit.packet");

    write_encoded_packet_file_create_new(&path, &encoded, write_limits).expect("write");
    let read_limits =
        EvaluationArtifactPacketFileLimits::new(encoded.byte_length.saturating_sub(1))
            .expect("limits");

    assert_eq!(
        read_and_verify_packet_file(&path, Some(&encoded.content_digest), read_limits),
        Err(EvaluationArtifactPacketFileError::FileExceedsLimit)
    );
}

#[test]
fn valid_prefix_plus_extra_bytes_at_original_limit_rejected() {
    let dir = TestDir::new();
    let encoded = build_encoded(exact_only_multi_disposition_fixture());
    let write_limits = default_limits(&encoded);
    let path = dir.file("prefix-plus-extra.packet");

    write_encoded_packet_file_create_new(&path, &encoded, write_limits).expect("write");
    let mut bytes = fs::read(&path).expect("read");
    bytes.extend_from_slice(b"EXTRA");
    write_raw(&path, &bytes);

    let read_limits = EvaluationArtifactPacketFileLimits::new(encoded.byte_length).expect("limits");
    assert_eq!(
        read_and_verify_packet_file(&path, Some(&encoded.content_digest), read_limits),
        Err(EvaluationArtifactPacketFileError::FileExceedsLimit)
    );
}

#[test]
fn oversized_file_with_valid_prefix_rejected() {
    let dir = TestDir::new();
    let encoded = build_encoded(exact_only_multi_disposition_fixture());
    let write_limits = default_limits(&encoded);
    let path = dir.file("oversized.packet");

    write_encoded_packet_file_create_new(&path, &encoded, write_limits).expect("write");
    let mut bytes = fs::read(&path).expect("read");
    bytes.push(b'x');
    write_raw(&path, &bytes);

    let read_limits = EvaluationArtifactPacketFileLimits::new(encoded.byte_length).expect("limits");
    assert_eq!(
        read_and_verify_packet_file(&path, Some(&encoded.content_digest), read_limits),
        Err(EvaluationArtifactPacketFileError::FileExceedsLimit)
    );
}

#[test]
fn file_exactly_at_limit_accepted() {
    let dir = TestDir::new();
    let encoded = build_encoded(exact_only_multi_disposition_fixture());
    let limits = EvaluationArtifactPacketFileLimits::new(encoded.byte_length).expect("limits");
    let path = dir.file("exact-limit.packet");

    write_encoded_packet_file_create_new(&path, &encoded, limits).expect("write");
    read_and_verify_packet_file(&path, Some(&encoded.content_digest), limits).expect("read");
}

#[test]
fn limit_one_byte_below_packet_rejected() {
    let dir = TestDir::new();
    let encoded = build_encoded(exact_only_multi_disposition_fixture());
    let limits = EvaluationArtifactPacketFileLimits::new(encoded.byte_length.saturating_sub(1))
        .expect("limits");
    let path = dir.file("limit-too-small.packet");

    assert!(matches!(
        write_encoded_packet_file_create_new(&path, &encoded, limits),
        Err(EvaluationArtifactPacketFileError::EncodedPacketExceedsLimit)
    ));
}

#[test]
fn truncated_packet_file_rejected() {
    let dir = TestDir::new();
    let encoded = build_encoded(exact_only_multi_disposition_fixture());
    let limits = default_limits(&encoded);
    let path = dir.file("truncated.packet");

    write_encoded_packet_file_create_new(&path, &encoded, limits).expect("write");
    let mut bytes = fs::read(&path).expect("read");
    bytes.truncate(bytes.len() / 2);
    write_raw(&path, &bytes);

    assert!(matches!(
        read_and_verify_packet_file(&path, None, limits),
        Err(EvaluationArtifactPacketFileError::PacketFileVerificationFailure(_))
    ));
}

#[test]
fn trailing_byte_rejected() {
    let dir = TestDir::new();
    let encoded = build_encoded(exact_only_multi_disposition_fixture());
    let limits = default_limits(&encoded);
    let path = dir.file("trailing.packet");

    write_encoded_packet_file_create_new(&path, &encoded, limits).expect("write");
    let mut bytes = fs::read(&path).expect("read");
    bytes.push(b'\n');
    write_raw(&path, &bytes);

    assert!(matches!(
        read_and_verify_packet_file(&path, None, limits),
        Err(EvaluationArtifactPacketFileError::FileExceedsLimit)
            | Err(EvaluationArtifactPacketFileError::PacketFileVerificationFailure(_))
    ));
}

#[test]
fn malformed_json_file_rejected() {
    let dir = TestDir::new();
    let limits = EvaluationArtifactPacketFileLimits::new(4096).expect("limits");
    let path = dir.file("malformed.packet");
    write_raw(&path, b"{not-json");

    assert!(matches!(
        read_and_verify_packet_file(&path, None, limits),
        Err(
            EvaluationArtifactPacketFileError::PacketFileVerificationFailure(
                EvaluationArtifactPacketError::PacketDeserializationFailure
            )
        )
    ));
}

#[test]
fn semantic_tamper_with_recomputed_hashes_fails_rederivation() {
    let dir = TestDir::new();
    let encoded = build_encoded(exact_only_multi_disposition_fixture());
    let limits = default_limits(&encoded);
    let path = dir.file("semantic-tamper.packet");

    write_encoded_packet_file_create_new(&path, &encoded, limits).expect("write");

    let mut packet = decode_packet(&encoded.packet_bytes).expect("decode");
    let index = packet
        .payloads
        .iter()
        .position(|entry| {
            packet.artifact_bundle.artifacts.iter().any(|descriptor| {
                descriptor.artifact_id == entry.artifact_id
                    && descriptor.role == ArtifactRole::EvaluationJoin
            })
        })
        .expect("join payload");
    let mut join = vox_proof::detector_reference_join::join_from_json(
        &packet.payloads[index].payload_utf8_json,
    )
    .expect("decode join");
    join.assessment.exact_match_count = join.assessment.exact_match_count.saturating_add(1);
    let bytes = serde_json::to_vec(&join).expect("serialize");
    packet.payloads[index].payload_utf8_json = String::from_utf8(bytes).expect("utf8");
    let hash = Sha256::digest(packet.payloads[index].payload_utf8_json.as_bytes());
    let hex = hash
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let digest = ArtifactContentDigest::new(format!("sha256:{hex}")).expect("digest");
    let descriptor = packet
        .artifact_bundle
        .artifacts
        .iter_mut()
        .find(|entry| entry.role == ArtifactRole::EvaluationJoin)
        .expect("descriptor");
    descriptor.content_digest = digest;
    descriptor.byte_length = packet.payloads[index].payload_utf8_json.len() as u64;

    let tampered_bytes = encode_packet(&packet).expect("encode").packet_bytes;
    write_raw(&path, &tampered_bytes);

    let result = read_and_verify_packet_file(&path, None, limits);
    assert!(matches!(
        result,
        Err(
            EvaluationArtifactPacketFileError::PacketFileVerificationFailure(
                EvaluationArtifactPacketError::PacketJoinRederivationMismatch
            )
        ) | Err(
            EvaluationArtifactPacketFileError::PacketFileVerificationFailure(
                EvaluationArtifactPacketError::EmbeddedPayloadValidationFailure { .. }
            )
        ) | Err(
            EvaluationArtifactPacketFileError::PacketFileVerificationFailure(
                EvaluationArtifactPacketError::PacketHistoricalReplayFailure { .. }
            )
        )
    ));
}

#[test]
fn wrong_detached_digest_on_read_propagates_mismatch() {
    let dir = TestDir::new();
    let encoded = build_encoded(exact_only_multi_disposition_fixture());
    let limits = default_limits(&encoded);
    let path = dir.file("digest-read.packet");
    let wrong = EvaluationArtifactPacketDigest::new(OVERLAP_PACKET_DIGEST).expect("digest");

    write_encoded_packet_file_create_new(&path, &encoded, limits).expect("write");
    assert!(matches!(
        read_and_verify_packet_file(&path, Some(&wrong), limits),
        Err(
            EvaluationArtifactPacketFileError::PacketFileVerificationFailure(
                EvaluationArtifactPacketError::DetachedPacketDigestMismatch
            )
        )
    ));
}

#[test]
fn uppercase_detached_digest_cannot_be_constructed() {
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
    assert!(EvaluationArtifactPacketDigest::new(uppercase).is_err());
}

#[test]
fn directory_path_rejected() {
    let dir = TestDir::new();
    let limits = EvaluationArtifactPacketFileLimits::new(4096).expect("limits");
    assert!(matches!(
        read_and_verify_packet_file(&dir.path, None, limits),
        Err(EvaluationArtifactPacketFileError::UnsupportedFileType)
    ));
}

#[cfg(unix)]
#[test]
fn newly_created_packet_file_is_owner_only() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TestDir::new();
    let encoded = build_encoded(exact_only_multi_disposition_fixture());
    let limits = default_limits(&encoded);
    let path = dir.file("permissions.packet");

    write_encoded_packet_file_create_new(&path, &encoded, limits).expect("write");
    let mode = fs::metadata(&path).expect("metadata").permissions().mode();
    assert_eq!(mode & 0o077, 0);
}

#[cfg(unix)]
#[test]
fn symlink_path_rejected() {
    use std::os::unix::fs::symlink;

    let dir = TestDir::new();
    let encoded = build_encoded(exact_only_multi_disposition_fixture());
    let limits = default_limits(&encoded);
    let target = dir.file("target.packet");
    let link = dir.file("link.packet");

    write_encoded_packet_file_create_new(&target, &encoded, limits).expect("write");
    symlink(&target, &link).expect("symlink");

    assert!(matches!(
        read_and_verify_packet_file(&link, None, limits),
        Err(EvaluationArtifactPacketFileError::UnsupportedFileType)
    ));
}

#[test]
fn packet_regression_seals_unchanged() {
    let exact = build_encoded(exact_only_multi_disposition_fixture());
    assert_eq!(exact.byte_length, EXACT_ONLY_PACKET_BYTE_LENGTH);
    assert_eq!(exact.content_digest.as_str(), EXACT_ONLY_PACKET_DIGEST);

    let overlap = build_encoded(overlap_pending_then_resolved_fixture());
    assert_eq!(overlap.byte_length, OVERLAP_PACKET_BYTE_LENGTH);
    assert_eq!(overlap.content_digest.as_str(), OVERLAP_PACKET_DIGEST);

    let zero = build_encoded(zero_population_fixture());
    assert_eq!(zero.byte_length, ZERO_POPULATION_PACKET_BYTE_LENGTH);
    assert_eq!(zero.content_digest.as_str(), ZERO_POPULATION_PACKET_DIGEST);
}

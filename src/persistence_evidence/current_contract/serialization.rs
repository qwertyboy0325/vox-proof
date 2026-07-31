use sha2::{Digest, Sha256};

use super::model::{EvidenceReuseCandidateKey, EvidenceSourceDecisionLocator};

pub const CANDIDATE_KEY_SERIALIZATION_VERSION: &str = "voxproof-evidence-candidate-key-v1";
pub const LOCATOR_SERIALIZATION_VERSION: &str = "voxproof-evidence-source-decision-locator-v1";

pub fn canonical_locator_bytes(locator: &EvidenceSourceDecisionLocator) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(LOCATOR_SERIALIZATION_VERSION.as_bytes());
    out.push(0);
    push_field(&mut out, &locator.source_revision_id);
    push_field(&mut out, &locator.source_analysis_snapshot_identity);
    push_field(&mut out, &locator.source_review_case_id);
    out.extend_from_slice(&(locator.review_ledger_position as u64).to_le_bytes());
    push_field(&mut out, &locator.decision_digest);
    out.extend_from_slice(&(locator.effective_at_ledger_length as u64).to_le_bytes());
    out
}

pub fn canonical_candidate_key_bytes(key: &EvidenceReuseCandidateKey) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(CANDIDATE_KEY_SERIALIZATION_VERSION.as_bytes());
    out.push(0);
    push_field(&mut out, &key.project_scope_stable_id);
    out.extend_from_slice(&canonical_locator_bytes(&key.source_locator));
    push_field(&mut out, &key.exact_payload.observed_text);
    push_field(&mut out, &key.exact_payload.confirmed_replacement);
    out
}

pub fn locator_canonical_digest(locator: &EvidenceSourceDecisionLocator) -> String {
    tagged_digest("sha256", &canonical_locator_bytes(locator))
}

pub fn candidate_key_canonical_digest(key: &EvidenceReuseCandidateKey) -> String {
    tagged_digest("sha256", &canonical_candidate_key_bytes(key))
}

pub fn rejection_identity_digest(
    project_scope_stable_id: &str,
    source_review_case_id: &str,
    review_ledger_position: usize,
    decision_digest: &str,
) -> String {
    let mut payload = Vec::new();
    push_field(&mut payload, project_scope_stable_id);
    push_field(&mut payload, source_review_case_id);
    payload.extend_from_slice(&(review_ledger_position as u64).to_le_bytes());
    push_field(&mut payload, decision_digest);
    tagged_digest("rejection", &payload)
}

fn push_field(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u64).to_le_bytes());
    out.extend_from_slice(value.as_bytes());
}

fn tagged_digest(prefix: &str, bytes: &[u8]) -> String {
    format!("{prefix}:{}", digest_hex(Sha256::digest(bytes)))
}

fn digest_hex(bytes: impl AsRef<[u8]>) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    bytes.as_ref().iter().fold(
        String::with_capacity(bytes.as_ref().len() * 2),
        |mut out, byte| {
            out.push(HEX[(byte >> 4) as usize] as char);
            out.push(HEX[(byte & 0x0f) as usize] as char);
            out
        },
    )
}

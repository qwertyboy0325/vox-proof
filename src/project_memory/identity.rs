use sha2::{Digest, Sha256};

use crate::reusable_influence::{
    ExactReusableCorrection, GovernanceActorContext, REUSABLE_INFLUENCE_PROJECTION_VERSION,
    ReusableGovernanceEvent,
};
use crate::reuse_primitives::{
    ProjectScopeId, SnapshotIdentityRecordProvenance, hash_source_decision_locator, hash_string,
};

pub const PROJECT_MEMORY_FORMAT_VERSION: u32 = 1;
/// Format that may contain human-raised promotion records (MD-022).
///
/// A project stays at version 1 until its first human-raised promotion append, so
/// detector-only projects keep byte-identical snapshot identities.
pub const PROJECT_MEMORY_FORMAT_VERSION_V2: u32 = 2;
pub const SUPPORTED_PROJECT_MEMORY_FORMAT_VERSIONS: [u32; 2] = [
    PROJECT_MEMORY_FORMAT_VERSION,
    PROJECT_MEMORY_FORMAT_VERSION_V2,
];

pub fn is_supported_project_memory_format_version(version: u32) -> bool {
    matches!(
        version,
        PROJECT_MEMORY_FORMAT_VERSION | PROJECT_MEMORY_FORMAT_VERSION_V2
    )
}

/// Format version required to persist `records` losslessly.
pub fn required_project_memory_format_version(records: &[ProjectMemoryRecord]) -> u32 {
    if records.iter().any(record_is_human_raised) {
        PROJECT_MEMORY_FORMAT_VERSION_V2
    } else {
        PROJECT_MEMORY_FORMAT_VERSION
    }
}

pub fn record_is_human_raised(record: &ProjectMemoryRecord) -> bool {
    match &record.event {
        ReusableGovernanceEvent::PromotionAccepted { source_locator, .. } => {
            source_locator.is_human_raised()
        }
        ReusableGovernanceEvent::PromotionCandidateRejected { candidate_key, .. } => {
            candidate_key.source_locator.is_human_raised()
        }
        ReusableGovernanceEvent::ReusableInfluenceRevoked { .. }
        | ReusableGovernanceEvent::ReusableInfluenceSuperseded { .. } => false,
    }
}
pub const PROJECT_MEMORY_SNAPSHOT_IDENTITY_DOMAIN: &[u8] =
    b"voxproof-project-memory-snapshot-identity-v1";
pub const PROJECT_MEMORY_SNAPSHOT_IDENTITY_TAG_PREFIX: &str = "project-memory-snapshot:sha256-v1:";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProjectMemorySnapshotIdentity([u8; 32]);

impl ProjectMemorySnapshotIdentity {
    pub fn to_tagged_string(self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut encoded =
            String::with_capacity(PROJECT_MEMORY_SNAPSHOT_IDENTITY_TAG_PREFIX.len() + 64);
        encoded.push_str(PROJECT_MEMORY_SNAPSHOT_IDENTITY_TAG_PREFIX);
        for byte in self.0 {
            encoded.push(HEX[(byte >> 4) as usize] as char);
            encoded.push(HEX[(byte & 0x0f) as usize] as char);
        }
        encoded
    }

    pub fn from_tagged_string(tag: &str) -> Option<Self> {
        let hex = tag.strip_prefix(PROJECT_MEMORY_SNAPSHOT_IDENTITY_TAG_PREFIX)?;
        parse_32_byte_hex(hex).map(Self)
    }

    pub fn digest(self) -> [u8; 32] {
        self.0
    }

    pub(crate) fn from_digest(digest: [u8; 32]) -> Self {
        Self(digest)
    }
}

fn parse_32_byte_hex(hex: &str) -> Option<[u8; 32]> {
    if hex.len() != 64 {
        return None;
    }
    let mut digest = [0_u8; 32];
    for (index, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let hi = (*chunk.first()? as char).to_digit(16)? as u8;
        let lo = (*chunk.get(1)? as char).to_digit(16)? as u8;
        digest[index] = (hi << 4) | lo;
    }
    Some(digest)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectMemoryRecord {
    pub source_session_id: String,
    pub event: ReusableGovernanceEvent,
}

pub fn compute_project_memory_snapshot_identity(
    project_id: &ProjectScopeId,
    format_version: u32,
    governance_event_boundary: usize,
    records: &[ProjectMemoryRecord],
) -> ProjectMemorySnapshotIdentity {
    let mut hasher = Sha256::new();
    hasher.update(PROJECT_MEMORY_SNAPSHOT_IDENTITY_DOMAIN);
    hasher.update(format_version.to_le_bytes());
    hash_string(&mut hasher, project_id.as_str());
    hasher.update((governance_event_boundary as u64).to_le_bytes());
    hash_string(&mut hasher, REUSABLE_INFLUENCE_PROJECTION_VERSION);
    let active: Vec<(&ProjectMemoryRecord, SnapshotIdentityRecordProvenance<'_>)> = records
        .iter()
        .enumerate()
        .filter_map(|(index, record)| match &record.event {
            ReusableGovernanceEvent::PromotionAccepted {
                payload,
                source_locator,
                actor,
                ..
            } => Some((record, provenance(index, payload, source_locator, actor))),
            _ => None,
        })
        .collect();
    hasher.update((active.len() as u64).to_le_bytes());
    for (record, provenance) in active {
        hash_string(&mut hasher, &record.source_session_id);
        hasher.update((provenance.record_id.promotion_event_index() as u64).to_le_bytes());
        hash_string(&mut hasher, provenance.observed_text);
        hash_string(&mut hasher, provenance.confirmed_replacement);
        hash_source_decision_locator(&mut hasher, provenance.source_locator);
        hash_string(&mut hasher, provenance.promotion_actor_role);
        hash_string(&mut hasher, provenance.promotion_actor_label);
    }
    ProjectMemorySnapshotIdentity::from_digest(hasher.finalize().into())
}

fn provenance<'a>(
    event_index: usize,
    payload: &'a ExactReusableCorrection,
    source_locator: &'a crate::reuse_primitives::SourceDecisionLocator,
    actor: &'a GovernanceActorContext,
) -> SnapshotIdentityRecordProvenance<'a> {
    SnapshotIdentityRecordProvenance {
        record_id: crate::reuse_primitives::ReusableInfluenceRecordId::from_promotion_event_index(
            event_index,
        ),
        observed_text: &payload.observed_text,
        confirmed_replacement: &payload.confirmed_replacement,
        source_locator,
        promotion_actor_role: &actor.role_label,
        promotion_actor_label: &actor.display_label,
    }
}

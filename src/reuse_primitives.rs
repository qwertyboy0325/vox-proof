use std::fmt;

use sha2::{Digest, Sha256};

use crate::analysis::AnalysisSnapshot;
use crate::anchor::TranscriptRevisionId;
use crate::review::{ManualReplacementText, ReviewCaseId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectScopeId(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectScopeTextError {
    Empty,
    WhitespaceOnly,
    UnicodeControl {
        character_index: usize,
        character: char,
    },
    UnicodeLineSeparator {
        character_index: usize,
        character: char,
    },
}

impl fmt::Display for ProjectScopeTextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ProjectScopeTextError {}

fn validate_scope_text(value: &str) -> Result<(), ProjectScopeTextError> {
    if value.is_empty() {
        return Err(ProjectScopeTextError::Empty);
    }
    if value.chars().all(char::is_whitespace) {
        return Err(ProjectScopeTextError::WhitespaceOnly);
    }
    if let Some((character_index, character)) = value
        .chars()
        .enumerate()
        .find(|(_, character)| character.is_control())
    {
        return Err(ProjectScopeTextError::UnicodeControl {
            character_index,
            character,
        });
    }
    if let Some((character_index, character)) = value
        .chars()
        .enumerate()
        .find(|(_, character)| matches!(character, '\u{2028}' | '\u{2029}'))
    {
        return Err(ProjectScopeTextError::UnicodeLineSeparator {
            character_index,
            character,
        });
    }
    Ok(())
}

impl ProjectScopeId {
    pub fn new(value: impl Into<String>) -> Result<Self, ProjectScopeTextError> {
        let value = value.into();
        validate_scope_text(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectScopeDisplayName(String);

impl ProjectScopeDisplayName {
    pub fn new(value: impl Into<String>) -> Result<Self, ProjectScopeTextError> {
        let value = value.into();
        validate_scope_text(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectScope {
    pub stable_id: ProjectScopeId,
    pub display_name: ProjectScopeDisplayName,
}

impl ProjectScope {
    pub fn new(stable_id: ProjectScopeId, display_name: ProjectScopeDisplayName) -> Self {
        Self {
            stable_id,
            display_name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceDecisionLocator {
    pub source_revision: TranscriptRevisionId,
    pub source_analysis_snapshot: AnalysisSnapshot,
    pub source_review_case_id: ReviewCaseId,
    pub review_ledger_position: usize,
    pub decision_digest: [u8; 32],
    pub effective_at_ledger_length: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ReusableInfluenceRecordId {
    promotion_event_index: usize,
}

impl ReusableInfluenceRecordId {
    pub fn from_promotion_event_index(index: usize) -> Self {
        Self {
            promotion_event_index: index,
        }
    }

    pub fn promotion_event_index(self) -> usize {
        self.promotion_event_index
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReusableInfluenceSnapshotIdentity([u8; 32]);

impl ReusableInfluenceSnapshotIdentity {
    pub fn to_tagged_string(self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut encoded =
            String::with_capacity("reusable-influence-snapshot:sha256-v1:".len() + 64);
        encoded.push_str("reusable-influence-snapshot:sha256-v1:");
        for byte in self.0 {
            encoded.push(HEX[(byte >> 4) as usize] as char);
            encoded.push(HEX[(byte & 0x0f) as usize] as char);
        }
        encoded
    }

    pub(crate) fn from_digest(digest: [u8; 32]) -> Self {
        Self(digest)
    }
}

pub fn decision_digest(
    case_id: ReviewCaseId,
    observed_revision: TranscriptRevisionId,
    replacement: &ManualReplacementText,
) -> [u8; 32] {
    const DOMAIN: &[u8] = b"voxproof-manual-replacement-decision-digest-v1";
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN);
    hasher.update((case_id.local_index() as u64).to_le_bytes());
    hasher.update(observed_revision.to_tagged_string().as_bytes());
    hasher.update((replacement.as_str().len() as u64).to_le_bytes());
    hasher.update(replacement.as_str().as_bytes());
    hasher.finalize().into()
}

pub(crate) fn hash_string(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

pub fn compute_snapshot_identity(
    project_scope_id: &ProjectScopeId,
    governance_event_boundary: usize,
    projection_version: &str,
    active_record_ids: &[(ReusableInfluenceRecordId, &str, &str, [u8; 32], usize)],
) -> ReusableInfluenceSnapshotIdentity {
    const DOMAIN: &[u8] = b"voxproof-reusable-influence-snapshot-identity-v1";
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN);
    hash_string(&mut hasher, project_scope_id.as_str());
    hasher.update((governance_event_boundary as u64).to_le_bytes());
    hash_string(&mut hasher, projection_version);
    hasher.update((active_record_ids.len() as u64).to_le_bytes());
    for (record_id, observed, replacement, digest, ledger_position) in active_record_ids {
        hasher.update((record_id.promotion_event_index() as u64).to_le_bytes());
        hash_string(&mut hasher, observed);
        hash_string(&mut hasher, replacement);
        hasher.update(*digest);
        hasher.update((*ledger_position as u64).to_le_bytes());
    }
    ReusableInfluenceSnapshotIdentity::from_digest(hasher.finalize().into())
}

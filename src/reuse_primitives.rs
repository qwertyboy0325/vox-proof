use std::fmt;

use sha2::{Digest, Sha256};

use crate::analysis::AnalysisSnapshot;
use crate::anchor::TranscriptRevisionId;
use crate::review::{ManualReplacementText, ReviewCaseId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PromotionCandidateRejectionIdentity {
    pub project_scope_id: ProjectScopeId,
    pub source_review_case_id: ReviewCaseId,
    pub review_ledger_position: usize,
    pub decision_digest: [u8; 32],
}

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
            String::with_capacity("reusable-influence-snapshot:sha256-v2:".len() + 64);
        encoded.push_str("reusable-influence-snapshot:sha256-v2:");
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

pub(crate) struct SnapshotIdentityRecordProvenance<'a> {
    pub record_id: ReusableInfluenceRecordId,
    pub observed_text: &'a str,
    pub confirmed_replacement: &'a str,
    pub source_locator: &'a SourceDecisionLocator,
    pub promotion_actor_role: &'a str,
    pub promotion_actor_label: &'a str,
}

pub(crate) fn hash_analysis_snapshot(hasher: &mut Sha256, snapshot: &AnalysisSnapshot) {
    hash_string(hasher, &snapshot.source_revision().to_tagged_string());
    hash_string(hasher, &snapshot.session_terms().to_tagged_string());
    let configuration = snapshot.configuration();
    let detector_set = configuration.detector_set();
    hasher.update((detector_set.detectors().len() as u64).to_le_bytes());
    for detector in detector_set.detectors() {
        hash_string(hasher, detector.id());
        hash_string(hasher, detector.version());
    }
    hash_string(hasher, configuration.detector_config().id());
    hash_string(hasher, configuration.detector_config().version());
    hash_string(hasher, configuration.algorithm().id());
    hash_string(hasher, configuration.algorithm().version());
}

pub(crate) fn hash_source_decision_locator(hasher: &mut Sha256, locator: &SourceDecisionLocator) {
    hash_string(hasher, &locator.source_revision.to_tagged_string());
    hash_analysis_snapshot(hasher, &locator.source_analysis_snapshot);
    hasher.update((locator.source_review_case_id.local_index() as u64).to_le_bytes());
    hasher.update((locator.review_ledger_position as u64).to_le_bytes());
    hasher.update(locator.decision_digest);
    hasher.update((locator.effective_at_ledger_length as u64).to_le_bytes());
}

pub(crate) fn compute_snapshot_identity(
    project_scope_id: &ProjectScopeId,
    governance_event_boundary: usize,
    projection_version: &str,
    active_records: &[SnapshotIdentityRecordProvenance<'_>],
) -> ReusableInfluenceSnapshotIdentity {
    const DOMAIN: &[u8] = b"voxproof-reusable-influence-snapshot-identity-v2";
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN);
    hash_string(&mut hasher, project_scope_id.as_str());
    hasher.update((governance_event_boundary as u64).to_le_bytes());
    hash_string(&mut hasher, projection_version);
    hasher.update((active_records.len() as u64).to_le_bytes());
    for record in active_records {
        hasher.update((record.record_id.promotion_event_index() as u64).to_le_bytes());
        hash_string(&mut hasher, record.observed_text);
        hash_string(&mut hasher, record.confirmed_replacement);
        hash_source_decision_locator(&mut hasher, record.source_locator);
        hash_string(&mut hasher, record.promotion_actor_role);
        hash_string(&mut hasher, record.promotion_actor_label);
    }
    ReusableInfluenceSnapshotIdentity::from_digest(hasher.finalize().into())
}

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

/// Same source-decision promotion origin, ignoring `effective_at_ledger_length`.
///
/// `effective_at_ledger_length` is a historical validation boundary only; it must
/// not make the same decision occurrence look like a distinct promotion origin
/// after unrelated ReviewLedger growth.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct SourceDecisionPromotionOrigin {
    pub project_scope_id: ProjectScopeId,
    pub source_revision: TranscriptRevisionId,
    pub origin: SourceDecisionLocatorOrigin,
    pub review_ledger_position: usize,
    pub decision_digest: [u8; 32],
}

impl SourceDecisionPromotionOrigin {
    pub(crate) fn from_locator(
        project_scope_id: &ProjectScopeId,
        locator: &SourceDecisionLocator,
    ) -> Self {
        Self {
            project_scope_id: project_scope_id.clone(),
            source_revision: locator.source_revision,
            origin: locator.origin.clone(),
            review_ledger_position: locator.review_ledger_position,
            decision_digest: locator.decision_digest,
        }
    }
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

/// Which review-case family produced the promoted source decision.
///
/// Detector-raised promotions stay bound to the analysis snapshot that produced
/// the candidate. Human-raised promotions have no analysis snapshot and must not
/// borrow one, so the variant carries only the human-raised case identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SourceDecisionLocatorOrigin {
    DetectorRaised {
        source_analysis_snapshot: AnalysisSnapshot,
        source_review_case_id: ReviewCaseId,
    },
    HumanRaised {
        human_raised_case_id: ReviewCaseId,
    },
}

impl SourceDecisionLocatorOrigin {
    pub fn review_case_id(&self) -> ReviewCaseId {
        match self {
            Self::DetectorRaised {
                source_review_case_id,
                ..
            } => *source_review_case_id,
            Self::HumanRaised {
                human_raised_case_id,
            } => *human_raised_case_id,
        }
    }

    pub fn analysis_snapshot(&self) -> Option<&AnalysisSnapshot> {
        match self {
            Self::DetectorRaised {
                source_analysis_snapshot,
                ..
            } => Some(source_analysis_snapshot),
            Self::HumanRaised { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceDecisionLocator {
    pub source_revision: TranscriptRevisionId,
    pub origin: SourceDecisionLocatorOrigin,
    pub review_ledger_position: usize,
    pub decision_digest: [u8; 32],
    pub effective_at_ledger_length: usize,
}

impl SourceDecisionLocator {
    pub fn source_review_case_id(&self) -> ReviewCaseId {
        self.origin.review_case_id()
    }

    pub fn source_analysis_snapshot(&self) -> Option<&AnalysisSnapshot> {
        self.origin.analysis_snapshot()
    }

    pub fn is_human_raised(&self) -> bool {
        matches!(self.origin, SourceDecisionLocatorOrigin::HumanRaised { .. })
    }
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

    #[cfg(test)]
    pub fn from_digest_for_test(digest: [u8; 32]) -> Self {
        Self::from_digest(digest)
    }
}

pub fn decision_digest(
    case_id: ReviewCaseId,
    observed_revision: TranscriptRevisionId,
    replacement: &ManualReplacementText,
) -> [u8; 32] {
    const DOMAIN: &[u8] = b"voxproof-manual-replacement-decision-digest-v1";
    const HUMAN_RAISED_FAMILY_MARKER: &[u8] = b"family:human";
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN);
    // Detector-raised digests must stay byte-identical to the pre-MD-022 sequence,
    // so only the human-raised family contributes an explicit marker.
    if case_id.is_human_raised() {
        hasher.update(HUMAN_RAISED_FAMILY_MARKER);
    }
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

/// Domain separator for the human-raised locator origin.
///
/// It occupies the byte position a detector-raised locator uses for the analysis
/// snapshot's own `source_revision` tagged string, which is always prefixed by a
/// transcript-revision tag, so the two origins cannot produce the same preimage.
const HUMAN_RAISED_LOCATOR_ORIGIN_DOMAIN: &str =
    "voxproof-source-decision-locator-origin:human-raised-v1";

pub(crate) fn hash_source_decision_locator(hasher: &mut Sha256, locator: &SourceDecisionLocator) {
    hash_string(hasher, &locator.source_revision.to_tagged_string());
    match &locator.origin {
        SourceDecisionLocatorOrigin::DetectorRaised {
            source_analysis_snapshot,
            source_review_case_id,
        } => {
            hash_analysis_snapshot(hasher, source_analysis_snapshot);
            hasher.update((source_review_case_id.local_index() as u64).to_le_bytes());
        }
        SourceDecisionLocatorOrigin::HumanRaised {
            human_raised_case_id,
        } => {
            hash_string(hasher, HUMAN_RAISED_LOCATOR_ORIGIN_DOMAIN);
            hasher.update((human_raised_case_id.local_index() as u64).to_le_bytes());
        }
    }
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

#[cfg(test)]
mod md022_detector_hash_stability_tests {
    use super::*;
    use crate::analysis::AnalysisRun;
    use crate::candidate::SessionTermEntry;

    fn detector_snapshot() -> AnalysisSnapshot {
        let transcript =
            crate::srt::parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid srt");
        let terms = vec![SessionTermEntry::new(
            "Kafka",
            vec!["Kafak".to_string()],
            Vec::new(),
        )];
        AnalysisRun::for_canonical_session_terms(&transcript, &terms).snapshot()
    }

    /// Byte sequence used before the MD-022 origin extension.
    fn legacy_locator_hash(
        revision: TranscriptRevisionId,
        snapshot: &AnalysisSnapshot,
        case_local_index: usize,
        review_ledger_position: usize,
        decision_digest: [u8; 32],
        effective_at_ledger_length: usize,
    ) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hash_string(&mut hasher, &revision.to_tagged_string());
        hash_analysis_snapshot(&mut hasher, snapshot);
        hasher.update((case_local_index as u64).to_le_bytes());
        hasher.update((review_ledger_position as u64).to_le_bytes());
        hasher.update(decision_digest);
        hasher.update((effective_at_ledger_length as u64).to_le_bytes());
        hasher.finalize().into()
    }

    #[test]
    fn detector_raised_locator_hash_matches_pre_md022_sequence() {
        let snapshot = detector_snapshot();
        let locator = SourceDecisionLocator {
            source_revision: snapshot.source_revision(),
            origin: SourceDecisionLocatorOrigin::DetectorRaised {
                source_analysis_snapshot: snapshot,
                source_review_case_id: ReviewCaseId::local(3),
            },
            review_ledger_position: 7,
            decision_digest: [9_u8; 32],
            effective_at_ledger_length: 11,
        };
        let mut hasher = Sha256::new();
        hash_source_decision_locator(&mut hasher, &locator);
        let actual: [u8; 32] = hasher.finalize().into();
        assert_eq!(
            actual,
            legacy_locator_hash(snapshot.source_revision(), &snapshot, 3, 7, [9_u8; 32], 11)
        );
    }

    #[test]
    fn human_raised_locator_hash_differs_from_detector_locator_hash() {
        let snapshot = detector_snapshot();
        let mut detector_hasher = Sha256::new();
        hash_source_decision_locator(
            &mut detector_hasher,
            &SourceDecisionLocator {
                source_revision: snapshot.source_revision(),
                origin: SourceDecisionLocatorOrigin::DetectorRaised {
                    source_analysis_snapshot: snapshot,
                    source_review_case_id: ReviewCaseId::local(3),
                },
                review_ledger_position: 7,
                decision_digest: [9_u8; 32],
                effective_at_ledger_length: 11,
            },
        );
        let mut human_hasher = Sha256::new();
        hash_source_decision_locator(
            &mut human_hasher,
            &SourceDecisionLocator {
                source_revision: snapshot.source_revision(),
                origin: SourceDecisionLocatorOrigin::HumanRaised {
                    human_raised_case_id: ReviewCaseId::human(3),
                },
                review_ledger_position: 7,
                decision_digest: [9_u8; 32],
                effective_at_ledger_length: 11,
            },
        );
        assert_ne!(
            <[u8; 32]>::from(detector_hasher.finalize()),
            <[u8; 32]>::from(human_hasher.finalize())
        );
    }

    #[test]
    fn detector_decision_digest_omits_human_family_marker() {
        let snapshot = detector_snapshot();
        let revision = snapshot.source_revision();
        let replacement =
            ManualReplacementText::new("Kafka", "Kafak").expect("valid replacement text");

        const DOMAIN: &[u8] = b"voxproof-manual-replacement-decision-digest-v1";
        let mut legacy = Sha256::new();
        legacy.update(DOMAIN);
        legacy.update(2_u64.to_le_bytes());
        legacy.update(revision.to_tagged_string().as_bytes());
        legacy.update((replacement.as_str().len() as u64).to_le_bytes());
        legacy.update(replacement.as_str().as_bytes());

        assert_eq!(
            decision_digest(ReviewCaseId::local(2), revision, &replacement),
            <[u8; 32]>::from(legacy.finalize())
        );
        assert_ne!(
            decision_digest(ReviewCaseId::human(2), revision, &replacement),
            decision_digest(ReviewCaseId::local(2), revision, &replacement)
        );
    }
}

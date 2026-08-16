use sha2::{Digest, Sha256};

use crate::analysis::AnalysisSnapshot;
use crate::anchor::{SourceAnchor, TranscriptRevisionId};
use crate::candidate::{
    CandidateAlternative, CandidateSpan, DetectionKind, DetectorProvenance, Evidence,
    ReusableExactObservedFormEvidence,
};
use crate::pipeline::ReuseEnabledTermReviewRun;
use crate::project_memory::{
    PROJECT_MEMORY_SNAPSHOT_IDENTITY_TAG_PREFIX, ProjectMemorySnapshotIdentity,
};
use crate::reuse_primitives::{
    ProjectScopeId, ReusableInfluenceSnapshotIdentity, hash_analysis_snapshot, hash_string,
};
use crate::review::{ReviewCase, ReviewCaseId};

pub const REUSE_PROPOSAL_TARGET_IDENTITY_DOMAIN: &[u8] =
    b"voxproof-reuse-proposal-target-identity-v1";
pub const REUSE_PROPOSAL_TARGET_IDENTITY_TAG_PREFIX: &str = "reuse-proposal-target:sha256-v1:";

pub const FROZEN_PROJECT_REUSE_ANALYSIS_IDENTITY_DOMAIN: &[u8] =
    b"voxproof-frozen-project-reuse-analysis-identity-v1";
pub const FROZEN_PROJECT_REUSE_ANALYSIS_IDENTITY_TAG_PREFIX: &str =
    "frozen-project-reuse-analysis:sha256-v1:";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReuseProposalTargetIdentity([u8; 32]);

impl ReuseProposalTargetIdentity {
    pub fn to_tagged_string(self) -> String {
        encode_tagged(REUSE_PROPOSAL_TARGET_IDENTITY_TAG_PREFIX, self.0)
    }

    pub fn from_tagged_string(tag: &str) -> Option<Self> {
        parse_tagged(REUSE_PROPOSAL_TARGET_IDENTITY_TAG_PREFIX, tag).map(Self)
    }

    pub fn digest(self) -> [u8; 32] {
        self.0
    }

    pub(crate) fn from_digest(digest: [u8; 32]) -> Self {
        Self(digest)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrozenProjectReuseAnalysisIdentity([u8; 32]);

impl FrozenProjectReuseAnalysisIdentity {
    pub fn to_tagged_string(self) -> String {
        encode_tagged(FROZEN_PROJECT_REUSE_ANALYSIS_IDENTITY_TAG_PREFIX, self.0)
    }

    pub fn from_tagged_string(tag: &str) -> Option<Self> {
        parse_tagged(FROZEN_PROJECT_REUSE_ANALYSIS_IDENTITY_TAG_PREFIX, tag).map(Self)
    }

    pub(crate) fn from_digest(digest: [u8; 32]) -> Self {
        Self(digest)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReuseProposalOccurrence {
    pub source_revision: TranscriptRevisionId,
    pub segment_position: usize,
    pub start_byte: usize,
    pub end_byte: usize,
    pub observed_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReuseProposalTarget {
    identity: ReuseProposalTargetIdentity,
    reuse_analysis_snapshot: AnalysisSnapshot,
    project_memory_snapshot_identity: ProjectMemorySnapshotIdentity,
    governance_boundary: usize,
    occurrence: ReuseProposalOccurrence,
    proposed_replacement: String,
    project_id: ProjectScopeId,
    contributing_record_ids: Vec<usize>,
    detector_id: String,
    detector_version: String,
    detector_config_id: String,
    detector_config_version: String,
    algorithm_id: String,
    algorithm_version: String,
    reusable_influence_snapshot_identity: ReusableInfluenceSnapshotIdentity,
}

impl ReuseProposalTarget {
    #[allow(clippy::too_many_arguments)]
    pub fn from_derivation_inputs(
        reuse_analysis_snapshot: AnalysisSnapshot,
        project_memory_snapshot_identity: ProjectMemorySnapshotIdentity,
        governance_boundary: usize,
        occurrence: ReuseProposalOccurrence,
        proposed_replacement: String,
        project_id: ProjectScopeId,
        mut contributing_record_ids: Vec<usize>,
        detector_id: String,
        detector_version: String,
        detector_config_id: String,
        detector_config_version: String,
        algorithm_id: String,
        algorithm_version: String,
        reusable_influence_snapshot_identity: ReusableInfluenceSnapshotIdentity,
    ) -> Self {
        contributing_record_ids.sort_unstable();
        contributing_record_ids.dedup();
        let identity = compute_reuse_proposal_target_identity(
            reuse_analysis_snapshot,
            project_memory_snapshot_identity,
            governance_boundary,
            &occurrence,
            &proposed_replacement,
            &project_id,
            &contributing_record_ids,
            &detector_id,
            &detector_version,
            &detector_config_id,
            &detector_config_version,
            &algorithm_id,
            &algorithm_version,
        );
        Self {
            identity,
            reuse_analysis_snapshot,
            project_memory_snapshot_identity,
            governance_boundary,
            occurrence,
            proposed_replacement,
            project_id,
            contributing_record_ids,
            detector_id,
            detector_version,
            detector_config_id,
            detector_config_version,
            algorithm_id,
            algorithm_version,
            reusable_influence_snapshot_identity,
        }
    }

    pub fn identity(&self) -> ReuseProposalTargetIdentity {
        self.identity
    }

    pub fn reuse_analysis_snapshot(&self) -> AnalysisSnapshot {
        self.reuse_analysis_snapshot
    }

    pub fn project_memory_snapshot_identity(&self) -> ProjectMemorySnapshotIdentity {
        self.project_memory_snapshot_identity
    }

    pub fn governance_boundary(&self) -> usize {
        self.governance_boundary
    }

    pub fn occurrence(&self) -> &ReuseProposalOccurrence {
        &self.occurrence
    }

    pub fn proposed_replacement(&self) -> &str {
        &self.proposed_replacement
    }

    pub fn project_id(&self) -> &ProjectScopeId {
        &self.project_id
    }

    pub fn contributing_record_ids(&self) -> &[usize] {
        &self.contributing_record_ids
    }

    pub fn detector_id(&self) -> &str {
        &self.detector_id
    }

    pub fn detector_version(&self) -> &str {
        &self.detector_version
    }

    pub fn detector_config_id(&self) -> &str {
        &self.detector_config_id
    }

    pub fn detector_config_version(&self) -> &str {
        &self.detector_config_version
    }

    pub fn algorithm_id(&self) -> &str {
        &self.algorithm_id
    }

    pub fn algorithm_version(&self) -> &str {
        &self.algorithm_version
    }

    pub fn reusable_influence_snapshot_identity(&self) -> ReusableInfluenceSnapshotIdentity {
        self.reusable_influence_snapshot_identity
    }

    pub fn verify_identity(&self) -> bool {
        compute_reuse_proposal_target_identity(
            self.reuse_analysis_snapshot,
            self.project_memory_snapshot_identity,
            self.governance_boundary,
            &self.occurrence,
            &self.proposed_replacement,
            &self.project_id,
            &self.contributing_record_ids,
            &self.detector_id,
            &self.detector_version,
            &self.detector_config_id,
            &self.detector_config_version,
            &self.algorithm_id,
            &self.algorithm_version,
        ) == self.identity
    }

    pub fn same_occurrence(&self, other: &Self) -> bool {
        self.occurrence.segment_position == other.occurrence.segment_position
            && self.occurrence.start_byte == other.occurrence.start_byte
            && self.occurrence.end_byte == other.occurrence.end_byte
    }

    pub fn occurrence_key(&self) -> (usize, usize, usize) {
        (
            self.occurrence.segment_position,
            self.occurrence.start_byte,
            self.occurrence.end_byte,
        )
    }

    pub fn display_review_case(&self, display_index: usize) -> ReviewCase {
        let anchor = SourceAnchor {
            revision: self.occurrence.source_revision,
            segment_position: self.occurrence.segment_position,
            start_byte: self.occurrence.start_byte,
            end_byte: self.occurrence.end_byte,
        };
        let evidence = Evidence::ReusableExactObservedForm(ReusableExactObservedFormEvidence {
            observed_text: self.occurrence.observed_text.clone(),
            confirmed_replacement: self.proposed_replacement.clone(),
            project_scope_id: self.project_id.clone(),
            snapshot_identity: self.reusable_influence_snapshot_identity,
            exact_input_contributions: Vec::new(),
            contributions: Vec::new(),
            promotion_event_indices: self.contributing_record_ids.clone(),
        });
        let span = CandidateSpan::new(
            DetectionKind::GlossaryAliasMatch,
            DetectorProvenance::new(&self.detector_id, &self.detector_version),
            anchor,
            evidence,
            vec![CandidateAlternative::new(&self.proposed_replacement)],
        );
        ReviewCase::detector_raised(ReviewCaseId::local(display_index), span)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenProjectReuseAnalysis {
    pub freeze_identity: FrozenProjectReuseAnalysisIdentity,
    pub project_memory_snapshot_identity: ProjectMemorySnapshotIdentity,
    pub governance_event_boundary: usize,
    pub reuse_analysis_snapshot: AnalysisSnapshot,
}

impl FrozenProjectReuseAnalysis {
    pub fn new(
        project_memory_snapshot_identity: ProjectMemorySnapshotIdentity,
        governance_event_boundary: usize,
        reuse_analysis_snapshot: AnalysisSnapshot,
    ) -> Self {
        let freeze_identity = compute_frozen_project_reuse_analysis_identity(
            project_memory_snapshot_identity,
            governance_event_boundary,
            reuse_analysis_snapshot,
        );
        Self {
            freeze_identity,
            project_memory_snapshot_identity,
            governance_event_boundary,
            reuse_analysis_snapshot,
        }
    }
}

pub fn derive_reuse_proposal_targets(
    transcript: &crate::transcript::Transcript,
    reuse_run: &ReuseEnabledTermReviewRun,
    project_id: &ProjectScopeId,
    project_memory_snapshot_identity: ProjectMemorySnapshotIdentity,
    governance_boundary: usize,
) -> Vec<ReuseProposalTarget> {
    let reuse_analysis_snapshot = reuse_run.analysis_run().snapshot();
    let configuration = reuse_analysis_snapshot.configuration();
    let mut targets = Vec::new();
    for review_case in reuse_run.review_cases() {
        let span = review_case.candidate_span();
        let Evidence::ReusableExactObservedForm(evidence) = span.evidence() else {
            continue;
        };
        let Some(resolved) = transcript.resolve(span.anchor()) else {
            continue;
        };
        let anchor = span.anchor();
        targets.push(ReuseProposalTarget::from_derivation_inputs(
            reuse_analysis_snapshot,
            project_memory_snapshot_identity,
            governance_boundary,
            ReuseProposalOccurrence {
                source_revision: transcript.revision_id(),
                segment_position: anchor.segment_position,
                start_byte: anchor.start_byte,
                end_byte: anchor.end_byte,
                observed_text: resolved.to_owned(),
            },
            evidence.confirmed_replacement.clone(),
            project_id.clone(),
            evidence.promotion_event_indices.clone(),
            span.provenance().detector_id().to_owned(),
            span.provenance().detector_version().to_owned(),
            configuration.detector_config().id().to_owned(),
            configuration.detector_config().version().to_owned(),
            configuration.algorithm().id().to_owned(),
            configuration.algorithm().version().to_owned(),
            evidence.snapshot_identity,
        ));
    }
    targets.sort_by(|left, right| {
        left.occurrence_key()
            .cmp(&right.occurrence_key())
            .then_with(|| left.identity().digest().cmp(&right.identity().digest()))
    });
    targets
}

#[allow(clippy::too_many_arguments)]
fn compute_reuse_proposal_target_identity(
    reuse_analysis_snapshot: AnalysisSnapshot,
    project_memory_snapshot_identity: ProjectMemorySnapshotIdentity,
    governance_boundary: usize,
    occurrence: &ReuseProposalOccurrence,
    proposed_replacement: &str,
    project_id: &ProjectScopeId,
    contributing_record_ids: &[usize],
    detector_id: &str,
    detector_version: &str,
    detector_config_id: &str,
    detector_config_version: &str,
    algorithm_id: &str,
    algorithm_version: &str,
) -> ReuseProposalTargetIdentity {
    let mut hasher = Sha256::new();
    hasher.update(REUSE_PROPOSAL_TARGET_IDENTITY_DOMAIN);
    hash_analysis_snapshot(&mut hasher, &reuse_analysis_snapshot);
    hash_string(
        &mut hasher,
        &project_memory_snapshot_identity.to_tagged_string(),
    );
    hasher.update((governance_boundary as u64).to_le_bytes());
    hash_string(&mut hasher, &occurrence.source_revision.to_tagged_string());
    hasher.update((occurrence.segment_position as u64).to_le_bytes());
    hasher.update((occurrence.start_byte as u64).to_le_bytes());
    hasher.update((occurrence.end_byte as u64).to_le_bytes());
    hash_string(&mut hasher, &occurrence.observed_text);
    hash_string(&mut hasher, proposed_replacement);
    hash_string(&mut hasher, project_id.as_str());
    hasher.update((contributing_record_ids.len() as u64).to_le_bytes());
    for record_id in contributing_record_ids {
        hasher.update((*record_id as u64).to_le_bytes());
    }
    hash_string(&mut hasher, detector_id);
    hash_string(&mut hasher, detector_version);
    hash_string(&mut hasher, detector_config_id);
    hash_string(&mut hasher, detector_config_version);
    hash_string(&mut hasher, algorithm_id);
    hash_string(&mut hasher, algorithm_version);
    let _ = PROJECT_MEMORY_SNAPSHOT_IDENTITY_TAG_PREFIX;
    ReuseProposalTargetIdentity::from_digest(hasher.finalize().into())
}

pub fn compute_frozen_project_reuse_analysis_identity(
    project_memory_snapshot_identity: ProjectMemorySnapshotIdentity,
    governance_event_boundary: usize,
    reuse_analysis_snapshot: AnalysisSnapshot,
) -> FrozenProjectReuseAnalysisIdentity {
    let mut hasher = Sha256::new();
    hasher.update(FROZEN_PROJECT_REUSE_ANALYSIS_IDENTITY_DOMAIN);
    hash_string(
        &mut hasher,
        &project_memory_snapshot_identity.to_tagged_string(),
    );
    hasher.update((governance_event_boundary as u64).to_le_bytes());
    hash_analysis_snapshot(&mut hasher, &reuse_analysis_snapshot);
    FrozenProjectReuseAnalysisIdentity::from_digest(hasher.finalize().into())
}

fn encode_tagged(prefix: &str, digest: [u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(prefix.len() + 64);
    encoded.push_str(prefix);
    for byte in digest {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn parse_tagged(prefix: &str, tag: &str) -> Option<[u8; 32]> {
    let hex = tag.strip_prefix(prefix)?;
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

use sha2::{Digest, Sha256};

use crate::analysis::{AnalysisRun, AnalysisSnapshot, SessionTermsIdentity};
use crate::anchor::{SourceAnchor, TranscriptRevisionId};
use crate::candidate::{
    CandidateAlternative, CandidateSpan, DetectionError, DetectionKind, DetectorProvenance,
    Evidence, PhoneticSimilarityEvidence, SessionTermEntry,
};
use crate::phonetic::{
    detect_ascii_latin_phonetic_matches_project_derived_terminology,
    replacement_is_derived_terminology_eligible,
};
use crate::project_memory::ProjectMemorySnapshotIdentity;
use crate::reusable_influence::{EffectiveReusableInfluenceRecord, ReuseAllowedEffect};
use crate::reuse_primitives::{ProjectScopeId, hash_analysis_snapshot, hash_string};
use crate::review::{ReviewCase, ReviewCaseId};
use crate::transcript::Transcript;

pub const PROJECT_DERIVED_TERMINOLOGY_ANALYSIS_IDENTITY_DOMAIN: &[u8] =
    b"voxproof-project-derived-terminology-analysis-identity-v1";
pub const PROJECT_DERIVED_TERMINOLOGY_ANALYSIS_IDENTITY_TAG_PREFIX: &str =
    "project-derived-terminology-analysis:sha256-v1:";

pub const PROJECT_TERMINOLOGY_PROPOSAL_TARGET_IDENTITY_DOMAIN: &[u8] =
    b"voxproof-project-terminology-proposal-target-identity-v1";
pub const PROJECT_TERMINOLOGY_PROPOSAL_TARGET_IDENTITY_TAG_PREFIX: &str =
    "project-terminology-proposal-target:sha256-v1:";

pub const DERIVED_CANONICAL_TERMINOLOGY_RULE_VERSION: &str = "derived-canonical-terminology-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProjectDerivedTerminologyAnalysisIdentity([u8; 32]);

impl ProjectDerivedTerminologyAnalysisIdentity {
    pub fn to_tagged_string(self) -> String {
        encode_tagged(
            PROJECT_DERIVED_TERMINOLOGY_ANALYSIS_IDENTITY_TAG_PREFIX,
            self.0,
        )
    }

    pub fn from_tagged_string(tag: &str) -> Option<Self> {
        parse_tagged(
            PROJECT_DERIVED_TERMINOLOGY_ANALYSIS_IDENTITY_TAG_PREFIX,
            tag,
        )
        .map(Self)
    }

    pub fn digest(self) -> [u8; 32] {
        self.0
    }

    pub(crate) fn from_digest(digest: [u8; 32]) -> Self {
        Self(digest)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProjectTerminologyProposalTargetIdentity([u8; 32]);

impl ProjectTerminologyProposalTargetIdentity {
    pub fn to_tagged_string(self) -> String {
        encode_tagged(
            PROJECT_TERMINOLOGY_PROPOSAL_TARGET_IDENTITY_TAG_PREFIX,
            self.0,
        )
    }

    pub fn from_tagged_string(tag: &str) -> Option<Self> {
        parse_tagged(PROJECT_TERMINOLOGY_PROPOSAL_TARGET_IDENTITY_TAG_PREFIX, tag).map(Self)
    }

    pub fn digest(self) -> [u8; 32] {
        self.0
    }

    pub(crate) fn from_digest(digest: [u8; 32]) -> Self {
        Self(digest)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectTerminologyOccurrence {
    pub source_revision: TranscriptRevisionId,
    pub segment_position: usize,
    pub start_byte: usize,
    pub end_byte: usize,
    pub observed_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectTerminologyProposalTarget {
    identity: ProjectTerminologyProposalTargetIdentity,
    analysis_identity: ProjectDerivedTerminologyAnalysisIdentity,
    analysis_snapshot: AnalysisSnapshot,
    project_memory_snapshot_identity: ProjectMemorySnapshotIdentity,
    governance_boundary: usize,
    occurrence: ProjectTerminologyOccurrence,
    proposed_replacement: String,
    project_id: ProjectScopeId,
    contributing_record_ids: Vec<usize>,
    detector_id: String,
    detector_version: String,
    detector_config_id: String,
    detector_config_version: String,
    algorithm_id: String,
    algorithm_version: String,
}

impl ProjectTerminologyProposalTarget {
    #[allow(clippy::too_many_arguments)]
    pub fn from_derivation_inputs(
        analysis_identity: ProjectDerivedTerminologyAnalysisIdentity,
        analysis_snapshot: AnalysisSnapshot,
        project_memory_snapshot_identity: ProjectMemorySnapshotIdentity,
        governance_boundary: usize,
        occurrence: ProjectTerminologyOccurrence,
        proposed_replacement: String,
        project_id: ProjectScopeId,
        mut contributing_record_ids: Vec<usize>,
        detector_id: String,
        detector_version: String,
        detector_config_id: String,
        detector_config_version: String,
        algorithm_id: String,
        algorithm_version: String,
    ) -> Self {
        contributing_record_ids.sort_unstable();
        contributing_record_ids.dedup();
        let identity = compute_project_terminology_proposal_target_identity(
            analysis_identity,
            analysis_snapshot,
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
            analysis_identity,
            analysis_snapshot,
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
        }
    }

    pub fn identity(&self) -> ProjectTerminologyProposalTargetIdentity {
        self.identity
    }

    pub fn analysis_identity(&self) -> ProjectDerivedTerminologyAnalysisIdentity {
        self.analysis_identity
    }

    pub fn analysis_snapshot(&self) -> AnalysisSnapshot {
        self.analysis_snapshot
    }

    pub fn project_memory_snapshot_identity(&self) -> ProjectMemorySnapshotIdentity {
        self.project_memory_snapshot_identity
    }

    pub fn governance_boundary(&self) -> usize {
        self.governance_boundary
    }

    pub fn occurrence(&self) -> &ProjectTerminologyOccurrence {
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
        let evidence = Evidence::PhoneticSimilarity(PhoneticSimilarityEvidence {
            observed_surface: self.occurrence.observed_text.clone(),
            target_surface: self.proposed_replacement.clone(),
            target_kind: crate::candidate::PhoneticTargetKind::CanonicalTerm,
            canonical_term: self.proposed_replacement.clone(),
            source_representation: crate::candidate::AsciiLatinPhoneticRepresentation {
                normalized_letters: String::new(),
                primary_key: String::new(),
                alternate_key: String::new(),
            },
            target_representation: crate::candidate::AsciiLatinPhoneticRepresentation {
                normalized_letters: String::new(),
                primary_key: String::new(),
                alternate_key: String::new(),
            },
            comparison: crate::candidate::PhoneticComparisonFacts {
                edit_distance: 0,
                ratio_numerator: 0,
                ratio_denominator: 1,
                ratio_permille: 0,
                matched_key: String::new(),
            },
            detector_config: crate::candidate::PROJECT_DERIVED_TERMINOLOGY_DETECTOR_CONFIG,
            algorithm: crate::candidate::PROJECT_DERIVED_TERMINOLOGY_ALGORITHM,
        });
        let span = CandidateSpan::new(
            DetectionKind::PhoneticSimilarity,
            DetectorProvenance::new(&self.detector_id, &self.detector_version),
            anchor,
            evidence,
            vec![CandidateAlternative::new(&self.proposed_replacement)],
        );
        ReviewCase::detector_raised(ReviewCaseId::local(display_index), span)
    }
}

pub fn compute_project_derived_terminology_analysis_identity(
    source_revision: TranscriptRevisionId,
    project_memory_snapshot_identity: ProjectMemorySnapshotIdentity,
    governance_boundary: usize,
    operator_session_terms: SessionTermsIdentity,
    analysis_snapshot: AnalysisSnapshot,
) -> ProjectDerivedTerminologyAnalysisIdentity {
    let configuration = analysis_snapshot.configuration();
    let mut hasher = Sha256::new();
    hasher.update(PROJECT_DERIVED_TERMINOLOGY_ANALYSIS_IDENTITY_DOMAIN);
    hash_string(&mut hasher, &source_revision.to_tagged_string());
    hash_string(
        &mut hasher,
        &project_memory_snapshot_identity.to_tagged_string(),
    );
    hasher.update((governance_boundary as u64).to_le_bytes());
    hash_string(&mut hasher, DERIVED_CANONICAL_TERMINOLOGY_RULE_VERSION);
    hash_string(&mut hasher, &operator_session_terms.to_tagged_string());
    hash_analysis_snapshot(&mut hasher, &analysis_snapshot);
    hash_string(&mut hasher, configuration.detector_config().id());
    hash_string(&mut hasher, configuration.detector_config().version());
    hash_string(&mut hasher, configuration.algorithm().id());
    hash_string(&mut hasher, configuration.algorithm().version());
    ProjectDerivedTerminologyAnalysisIdentity::from_digest(hasher.finalize().into())
}

pub fn derive_project_terminology_proposal_targets(
    transcript: &Transcript,
    operator_session_terms: &[SessionTermEntry],
    active_records: &[EffectiveReusableInfluenceRecord],
    project_id: &ProjectScopeId,
    project_memory_snapshot_identity: ProjectMemorySnapshotIdentity,
    governance_boundary: usize,
) -> Result<Vec<ProjectTerminologyProposalTarget>, DetectionError> {
    let derived_entries = derived_canonical_entries(active_records);
    if derived_entries.is_empty() {
        return Ok(Vec::new());
    }
    let run = AnalysisRun::for_project_derived_terminology(transcript, &derived_entries);
    let spans = detect_ascii_latin_phonetic_matches_project_derived_terminology(
        &run,
        transcript,
        &derived_entries,
    )?;
    let analysis_snapshot = run.snapshot();
    let analysis_identity = compute_project_derived_terminology_analysis_identity(
        transcript.revision_id(),
        project_memory_snapshot_identity,
        governance_boundary,
        SessionTermsIdentity::from_entries(operator_session_terms),
        analysis_snapshot,
    );
    let configuration = analysis_snapshot.configuration();
    let mut targets = Vec::new();
    for span in spans {
        let Evidence::PhoneticSimilarity(evidence) = span.evidence() else {
            continue;
        };
        let Some(resolved) = transcript.resolve(span.anchor()) else {
            continue;
        };
        let contributing_record_ids =
            contributing_ids_for_replacement(active_records, &evidence.canonical_term);
        let anchor = span.anchor();
        targets.push(ProjectTerminologyProposalTarget::from_derivation_inputs(
            analysis_identity,
            analysis_snapshot,
            project_memory_snapshot_identity,
            governance_boundary,
            ProjectTerminologyOccurrence {
                source_revision: transcript.revision_id(),
                segment_position: anchor.segment_position,
                start_byte: anchor.start_byte,
                end_byte: anchor.end_byte,
                observed_text: resolved.to_owned(),
            },
            evidence.canonical_term.clone(),
            project_id.clone(),
            contributing_record_ids,
            span.provenance().detector_id().to_owned(),
            span.provenance().detector_version().to_owned(),
            configuration.detector_config().id().to_owned(),
            configuration.detector_config().version().to_owned(),
            configuration.algorithm().id().to_owned(),
            configuration.algorithm().version().to_owned(),
        ));
    }
    targets.sort_by(|left, right| {
        left.occurrence_key()
            .cmp(&right.occurrence_key())
            .then_with(|| left.identity().digest().cmp(&right.identity().digest()))
    });
    Ok(targets)
}

fn derived_canonical_entries(
    active_records: &[EffectiveReusableInfluenceRecord],
) -> Vec<SessionTermEntry> {
    let mut entries = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for record in active_records {
        if !record
            .allowed_effects
            .includes(ReuseAllowedEffect::DerivedCanonicalTerminologyProposalGeneration)
        {
            continue;
        }
        let replacement = record.payload.confirmed_replacement.as_str();
        if !replacement_is_derived_terminology_eligible(replacement) {
            continue;
        }
        if !seen.insert(replacement.to_owned()) {
            continue;
        }
        entries.push(SessionTermEntry::new(
            replacement.to_owned(),
            Vec::new(),
            Vec::new(),
        ));
    }
    entries
}

fn contributing_ids_for_replacement(
    active_records: &[EffectiveReusableInfluenceRecord],
    replacement: &str,
) -> Vec<usize> {
    active_records
        .iter()
        .filter(|record| {
            record
                .allowed_effects
                .includes(ReuseAllowedEffect::DerivedCanonicalTerminologyProposalGeneration)
                && record.payload.confirmed_replacement == replacement
        })
        .map(|record| record.record_id.promotion_event_index())
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn compute_project_terminology_proposal_target_identity(
    analysis_identity: ProjectDerivedTerminologyAnalysisIdentity,
    analysis_snapshot: AnalysisSnapshot,
    project_memory_snapshot_identity: ProjectMemorySnapshotIdentity,
    governance_boundary: usize,
    occurrence: &ProjectTerminologyOccurrence,
    proposed_replacement: &str,
    project_id: &ProjectScopeId,
    contributing_record_ids: &[usize],
    detector_id: &str,
    detector_version: &str,
    detector_config_id: &str,
    detector_config_version: &str,
    algorithm_id: &str,
    algorithm_version: &str,
) -> ProjectTerminologyProposalTargetIdentity {
    let mut hasher = Sha256::new();
    hasher.update(PROJECT_TERMINOLOGY_PROPOSAL_TARGET_IDENTITY_DOMAIN);
    hash_string(&mut hasher, &analysis_identity.to_tagged_string());
    hash_analysis_snapshot(&mut hasher, &analysis_snapshot);
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
    ProjectTerminologyProposalTargetIdentity::from_digest(hasher.finalize().into())
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

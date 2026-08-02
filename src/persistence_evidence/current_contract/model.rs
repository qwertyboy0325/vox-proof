use serde::{Deserialize, Serialize};

/// Mechanism-independent persistence evidence state aligned with owner-accepted Gate 1–3.
///
/// Spike-only. Not production persistence authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentContractState {
    pub session_id: String,
    pub duplicated_from_session_id: Option<String>,
    pub session_authority: EvidenceSessionAuthority,
    pub material_use_declaration: EvidenceMaterialUseDeclaration,
    pub source_revisions: Vec<EvidenceSourceRevision>,
    pub session_terms_identity: String,
    pub analysis_snapshots: Vec<EvidenceAnalysisSnapshot>,
    pub review_cases: Vec<EvidenceReviewCase>,
    pub review_ledger_events: Vec<EvidenceReviewLedgerEvent>,
    pub effective_review_status: Vec<EvidenceEffectiveReviewStatus>,
    pub project_scope: EvidenceProjectScope,
    pub reuse_governance_events: Vec<EvidenceReuseGovernanceEvent>,
    pub effective_reusable_records: Vec<EvidenceReusableRecord>,
    pub historical_reusable_records: Vec<EvidenceReusableRecord>,
    pub reusable_snapshot_identity: String,
    pub reuse_enabled_analysis_binding: Option<EvidenceReuseEnabledAnalysisBinding>,
    pub derived_queue_projection: String,
    pub durable_command_tokens: EvidenceDurableCommandTokens,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceSessionAuthority {
    pub role_label: String,
    pub display_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceMaterialUseDeclaration {
    pub basis: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceSourceRevision {
    pub revision_id: String,
    pub predecessor_revision_id: Option<String>,
    pub transcript_bytes: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceDetectorIdentity {
    pub id: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceAnalysisSnapshot {
    pub identity: String,
    pub source_revision_id: String,
    pub session_terms_identity: String,
    pub detectors: Vec<EvidenceDetectorIdentity>,
    pub detector_config_id: String,
    pub detector_config_version: String,
    pub algorithm_id: String,
    pub algorithm_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceReviewCase {
    pub case_id: String,
    pub origin: String,
    pub observed_revision_id: String,
    pub anchor_revision_id: String,
    pub anchor_segment_position: usize,
    pub anchor_start_byte: usize,
    pub anchor_end_byte: usize,
    pub observed_source_bytes: String,
    pub alternative_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceReviewLedgerEvent {
    pub event_index: usize,
    pub case_id: String,
    pub observed_revision_id: String,
    pub action_kind: String,
    pub manual_replacement_bytes: Option<String>,
    pub alternative_index: Option<usize>,
    pub target_event_index: Option<usize>,
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceEffectiveReviewStatus {
    pub case_id: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceProjectScope {
    pub stable_id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceSourceDecisionLocator {
    pub source_revision_id: String,
    pub source_analysis_snapshot_identity: String,
    pub source_review_case_id: String,
    pub review_ledger_position: usize,
    pub decision_digest: String,
    pub effective_at_ledger_length: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceExactReusableCorrection {
    pub observed_text: String,
    pub confirmed_replacement: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceReuseCandidateKey {
    pub project_scope_stable_id: String,
    pub source_locator: EvidenceSourceDecisionLocator,
    pub exact_payload: EvidenceExactReusableCorrection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceGovernanceActor {
    pub role_label: String,
    pub display_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event_kind", rename_all = "snake_case")]
pub enum EvidenceReuseGovernanceEvent {
    PromotionCandidateRejected {
        event_index: usize,
        candidate_key: EvidenceReuseCandidateKey,
        actor: EvidenceGovernanceActor,
    },
    PromotionAccepted {
        event_index: usize,
        candidate_key: EvidenceReuseCandidateKey,
        payload: EvidenceExactReusableCorrection,
        source_locator: EvidenceSourceDecisionLocator,
        actor: EvidenceGovernanceActor,
        project_scope_stable_id: String,
    },
    ReusableInfluenceRevoked {
        event_index: usize,
        record_id: usize,
        actor: EvidenceGovernanceActor,
    },
    ReusableInfluenceSuperseded {
        event_index: usize,
        predecessor_record_id: usize,
        successor_record_id: usize,
        actor: EvidenceGovernanceActor,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceReusableRecord {
    pub record_id: usize,
    pub project_scope_stable_id: String,
    pub observed_text: String,
    pub confirmed_replacement: String,
    pub source_locator: EvidenceSourceDecisionLocator,
    pub promotion_actor: EvidenceGovernanceActor,
    pub superseded_by: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceReuseEnabledAnalysisBinding {
    pub analysis_snapshot: EvidenceAnalysisSnapshot,
    pub analysis_snapshot_identity: String,
    pub reusable_snapshot_identity: String,
    pub governance_event_boundary: usize,
    pub projection_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceDurableCommandTokens {
    pub review_ledger_head: usize,
    pub reuse_governance_head: usize,
    pub active_analysis_snapshot_identity: String,
    pub evidence_writer_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentContractFixture {
    pub fixture_id: String,
    pub fixture_version: String,
    pub variant_id: String,
    pub scale: CurrentContractFixtureScale,
    pub expected_state: CurrentContractState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurrentContractFixtureScale {
    Small,
    Medium,
    Stress,
}

impl CurrentContractFixtureScale {
    pub fn is_implemented(self) -> bool {
        matches!(self, Self::Small)
    }
}

pub const CURRENT_CONTRACT_FIXTURE_ID: &str = "voxproof-persistence-evidence-current-contract";
pub const CURRENT_CONTRACT_FIXTURE_VERSION: &str = "3";
pub const REUSABLE_INFLUENCE_PROJECTION_VERSION: &str = "reusable-exact-input-v1";

impl CurrentContractState {
    pub fn normalize(mut self) -> Self {
        self.source_revisions
            .sort_by(|left, right| left.revision_id.cmp(&right.revision_id));
        self.analysis_snapshots
            .sort_by(|left, right| left.identity.cmp(&right.identity));
        self.review_cases
            .sort_by(|left, right| left.case_id.cmp(&right.case_id));
        self.review_ledger_events
            .sort_by_key(|event| event.event_index);
        self.effective_review_status
            .sort_by(|left, right| left.case_id.cmp(&right.case_id));
        self.reuse_governance_events
            .sort_by_key(|event| match event {
                EvidenceReuseGovernanceEvent::PromotionCandidateRejected {
                    event_index, ..
                }
                | EvidenceReuseGovernanceEvent::PromotionAccepted { event_index, .. }
                | EvidenceReuseGovernanceEvent::ReusableInfluenceRevoked { event_index, .. }
                | EvidenceReuseGovernanceEvent::ReusableInfluenceSuperseded {
                    event_index, ..
                } => *event_index,
            });
        self.effective_reusable_records
            .sort_by_key(|record| record.record_id);
        self.historical_reusable_records
            .sort_by_key(|record| record.record_id);
        self
    }

    pub fn canonical_projection(&self) -> CurrentContractCanonicalProjection {
        CurrentContractCanonicalProjection {
            session_id: self.session_id.clone(),
            duplicated_from_session_id: self.duplicated_from_session_id.clone(),
            session_authority: self.session_authority.clone(),
            material_use_declaration: self.material_use_declaration.clone(),
            source_revisions: self.source_revisions.clone(),
            session_terms_identity: self.session_terms_identity.clone(),
            analysis_snapshots: self.analysis_snapshots.clone(),
            review_cases: self.review_cases.clone(),
            review_ledger_events: self.review_ledger_events.clone(),
            project_scope_stable_id: self.project_scope.stable_id.clone(),
            reuse_governance_events: self.reuse_governance_events.clone(),
            reuse_enabled_analysis_binding: self.reuse_enabled_analysis_binding.clone(),
            durable_command_tokens: self.durable_command_tokens.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentContractCanonicalProjection {
    pub session_id: String,
    pub duplicated_from_session_id: Option<String>,
    pub session_authority: EvidenceSessionAuthority,
    pub material_use_declaration: EvidenceMaterialUseDeclaration,
    pub source_revisions: Vec<EvidenceSourceRevision>,
    pub session_terms_identity: String,
    pub analysis_snapshots: Vec<EvidenceAnalysisSnapshot>,
    pub review_cases: Vec<EvidenceReviewCase>,
    pub review_ledger_events: Vec<EvidenceReviewLedgerEvent>,
    pub project_scope_stable_id: String,
    pub reuse_governance_events: Vec<EvidenceReuseGovernanceEvent>,
    pub reuse_enabled_analysis_binding: Option<EvidenceReuseEnabledAnalysisBinding>,
    pub durable_command_tokens: EvidenceDurableCommandTokens,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivedContractProjection {
    pub effective_review_status: Vec<EvidenceEffectiveReviewStatus>,
    pub rejected_candidate_identities: Vec<String>,
    pub effective_reusable_records: Vec<EvidenceReusableRecord>,
    pub historical_reusable_records: Vec<EvidenceReusableRecord>,
    pub reusable_snapshot_identity: String,
    pub derived_queue_projection: String,
}

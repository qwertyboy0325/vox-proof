use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;

use crate::analysis::AnalysisSnapshot;
use crate::candidate::{
    CandidateAlternative, CandidateSpan, DetectionKind, DetectorProvenance, Evidence,
    ResolvedExactInputContributionEvidence, ReusableExactObservedFormEvidence,
    ReusableProvenanceContribution, SessionTermEntry,
};
use crate::pipeline::CanonicalTermReviewRun;
use crate::reuse_primitives::{
    ProjectScope, ProjectScopeId, PromotionCandidateRejectionIdentity, ReusableInfluenceRecordId,
    ReusableInfluenceSnapshotIdentity, SnapshotIdentityRecordProvenance, SourceDecisionLocator,
    SourceDecisionPromotionOrigin, compute_snapshot_identity, decision_digest,
};
use crate::review::{
    CorrectionDecision, ManualReplacementText, ReviewCase, ReviewCaseId, ReviewCaseStatus,
    ReviewLedger, ReviewLedgerEvent,
};
use crate::transcript::Transcript;

pub use crate::reuse_primitives::ProjectScopeTextError;

pub const RESOLVED_EXACT_OBSERVED_FORM_DETECTOR_ID: &str = "resolved-exact-observed-form-match";
pub const RESOLVED_EXACT_OBSERVED_FORM_DETECTOR_VERSION: &str = "0.1.0";
pub const REUSABLE_INFLUENCE_PROJECTION_VERSION: &str = "reusable-exact-input-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactReusableCorrection {
    pub observed_text: String,
    pub confirmed_replacement: String,
}

impl ExactReusableCorrection {
    pub fn from_manual_replacement(
        observed_text: &str,
        replacement: &ManualReplacementText,
    ) -> Self {
        Self {
            observed_text: observed_text.to_owned(),
            confirmed_replacement: replacement.as_str().to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReuseCandidateKey {
    pub source_locator: SourceDecisionLocator,
    pub project_scope_id: ProjectScopeId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReuseAllowedEffect {
    ExactObservedFormProposalGeneration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReuseCandidate {
    pub key: ReuseCandidateKey,
    pub exact_payload: ExactReusableCorrection,
    pub proposed_scope: ProjectScopeId,
    pub proposed_allowed_effects: Vec<ReuseAllowedEffect>,
    pub source_decision_still_effective: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovernanceActorContext {
    pub role_label: String,
    pub display_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReusableGovernanceEvent {
    PromotionCandidateRejected {
        candidate_key: Box<ReuseCandidateKey>,
        actor: GovernanceActorContext,
    },
    PromotionAccepted {
        candidate_key: Box<ReuseCandidateKey>,
        payload: ExactReusableCorrection,
        source_locator: Box<SourceDecisionLocator>,
        actor: GovernanceActorContext,
        project_scope: Box<ProjectScope>,
    },
    ReusableInfluenceRevoked {
        record_id: ReusableInfluenceRecordId,
        actor: GovernanceActorContext,
    },
    ReusableInfluenceSuperseded {
        predecessor_id: ReusableInfluenceRecordId,
        successor_id: ReusableInfluenceRecordId,
        actor: GovernanceActorContext,
    },
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReusableInfluenceLedger {
    events: Vec<ReusableGovernanceEvent>,
}

impl ReusableInfluenceLedger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn events(&self) -> &[ReusableGovernanceEvent] {
        &self.events
    }

    pub(crate) fn append(&mut self, event: ReusableGovernanceEvent) -> usize {
        let index = self.events.len();
        self.events.push(event);
        index
    }

    pub(crate) fn append_batch(&mut self, events: Vec<ReusableGovernanceEvent>) -> Vec<usize> {
        let mut indices = Vec::with_capacity(events.len());
        for event in events {
            indices.push(self.append(event));
        }
        indices
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveReusableInfluenceRecord {
    pub record_id: ReusableInfluenceRecordId,
    pub project_scope: ProjectScope,
    pub payload: ExactReusableCorrection,
    pub source_locator: SourceDecisionLocator,
    pub promotion_actor: GovernanceActorContext,
    pub source_decision_still_effective: bool,
    pub superseded_by: Option<ReusableInfluenceRecordId>,
}

/// Effective reusable-influence state produced only by deterministic governance folding.
///
/// External callers cannot construct or mutate authority-bearing contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReusableInfluenceEffectiveState {
    active_records: Vec<EffectiveReusableInfluenceRecord>,
    historical_records: Vec<EffectiveReusableInfluenceRecord>,
    rejected_candidate_identities: HashSet<PromotionCandidateRejectionIdentity>,
}

impl ReusableInfluenceEffectiveState {
    pub fn active_records(&self) -> &[EffectiveReusableInfluenceRecord] {
        &self.active_records
    }

    pub fn historical_records(&self) -> &[EffectiveReusableInfluenceRecord] {
        &self.historical_records
    }

    pub fn rejected_candidate_identities(&self) -> &HashSet<PromotionCandidateRejectionIdentity> {
        &self.rejected_candidate_identities
    }
}

/// Immutable reusable-influence snapshot bound to validated project scope and ledger fold.
///
/// External callers cannot construct or mutate authority-bearing contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReusableInfluenceSnapshot {
    project_scope_id: ProjectScopeId,
    governance_event_boundary: usize,
    projection_version: &'static str,
    identity: ReusableInfluenceSnapshotIdentity,
    active_records: Vec<EffectiveReusableInfluenceRecord>,
}

impl ReusableInfluenceSnapshot {
    pub fn project_scope_id(&self) -> &ProjectScopeId {
        &self.project_scope_id
    }

    pub fn governance_event_boundary(&self) -> usize {
        self.governance_event_boundary
    }

    pub fn projection_version(&self) -> &'static str {
        self.projection_version
    }

    pub fn identity(&self) -> ReusableInfluenceSnapshotIdentity {
        self.identity
    }

    pub fn active_records(&self) -> &[EffectiveReusableInfluenceRecord] {
        &self.active_records
    }
}

#[cfg(test)]
impl ReusableInfluenceSnapshot {
    pub fn replace_identity_for_test(&mut self, identity: ReusableInfluenceSnapshotIdentity) {
        self.identity = identity;
    }

    pub(crate) fn clear_active_records_for_test(&mut self) {
        self.active_records.clear();
    }

    pub(crate) fn push_active_record_out_of_order_for_test(
        &mut self,
        record: EffectiveReusableInfluenceRecord,
    ) {
        self.active_records.push(record);
    }

    pub(crate) fn set_projection_version_for_test(&mut self, projection_version: &'static str) {
        self.projection_version = projection_version;
    }

    pub(crate) fn set_governance_event_boundary_for_test(&mut self, boundary: usize) {
        self.governance_event_boundary = boundary;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ExactInputContributionKind {
    BaseObservedErrorForm,
    ReusableInfluenceRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct ExactInputContribution {
    pub kind: ExactInputContributionKind,
    pub observed_text: String,
    pub confirmed_replacement: String,
    pub reusable_record_id: Option<ReusableInfluenceRecordId>,
    pub session_term_canonical: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExactInputMatcherEntry {
    pub observed_text: String,
    pub confirmed_replacement: String,
    pub contributions: Vec<ExactInputContribution>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedExactInputProjection {
    project_scope_id: ProjectScopeId,
    snapshot_identity: ReusableInfluenceSnapshotIdentity,
    entries: Vec<ExactInputMatcherEntry>,
}

impl ResolvedExactInputProjection {
    pub(crate) fn entries(&self) -> &[ExactInputMatcherEntry] {
        &self.entries
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReusableInfluenceError {
    MissingProjectScope,
    ProjectScopeMismatch,
    UnknownCandidate,
    CandidateAlreadyRejected,
    CandidateAlreadyPromoted,
    InvalidSourceLocator,
    SourceDecisionNotManualReplacement,
    SourceDecisionNotEffective,
    SourceTextMismatch,
    ReplacementMismatch,
    RecordNotActive {
        record_id: ReusableInfluenceRecordId,
    },
    SelfSupersession,
    InvalidSuccessorCandidate,
    DivergentExactMapping {
        observed_text: String,
    },
    ProjectionSnapshotIdentityMismatch,
    ProjectionContentMismatch,
    InvalidGovernanceActor,
    CandidateKeySourceLocatorMismatch,
    MissingReusableRecordProvenance {
        record_id: ReusableInfluenceRecordId,
    },
    WrongProjectScope,
    SnapshotIdentityMismatch,
    UnsupportedSnapshotProjectionVersion {
        version: String,
    },
    DuplicateActiveRecordIdentity {
        record_id: ReusableInfluenceRecordId,
    },
    NonCanonicalActiveRecordOrdering,
    SnapshotRecordWrongProjectScope,
    NonemptySnapshotRequiresGovernanceBoundary,
    RecordOutsideGovernanceBoundary {
        record_id: ReusableInfluenceRecordId,
        governance_event_boundary: usize,
    },
    ReuseEnabledSnapshotIdentityMismatch,
}

impl fmt::Display for ReusableInfluenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ReusableInfluenceError {}

pub fn locate_effective_manual_replacement_at_prefix(
    ledger: &ReviewLedger,
    case_id: ReviewCaseId,
    prefix_length: usize,
) -> Option<usize> {
    ledger.locate_effective_manual_replacement_at_prefix(case_id, prefix_length)
}

pub fn locate_effective_manual_replacement_event(
    ledger: &ReviewLedger,
    case_id: ReviewCaseId,
) -> Option<(usize, &ReviewLedgerEvent)> {
    ledger
        .events()
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, event)| {
            let ReviewLedgerEvent::DecisionRecorded {
                case_id: event_case_id,
                decision,
                ..
            } = event;
            if *event_case_id == case_id
                && matches!(decision, CorrectionDecision::ManualReplacement { .. })
            {
                Some((index, event))
            } else {
                None
            }
        })
}

pub fn build_source_decision_locator(
    analysis_snapshot: AnalysisSnapshot,
    case_id: ReviewCaseId,
    ledger_position: usize,
    observed_revision: crate::anchor::TranscriptRevisionId,
    replacement: &ManualReplacementText,
    ledger_length: usize,
) -> SourceDecisionLocator {
    SourceDecisionLocator {
        source_revision: observed_revision,
        source_analysis_snapshot: analysis_snapshot,
        source_review_case_id: case_id,
        review_ledger_position: ledger_position,
        decision_digest: decision_digest(case_id, observed_revision, replacement),
        effective_at_ledger_length: ledger_length,
    }
}

impl From<&ReuseCandidateKey> for PromotionCandidateRejectionIdentity {
    fn from(key: &ReuseCandidateKey) -> Self {
        Self {
            project_scope_id: key.project_scope_id.clone(),
            source_review_case_id: key.source_locator.source_review_case_id,
            review_ledger_position: key.source_locator.review_ledger_position,
            decision_digest: key.source_locator.decision_digest,
        }
    }
}

pub(crate) fn source_decision_promotion_origin(
    project_scope_id: &ProjectScopeId,
    locator: &SourceDecisionLocator,
) -> SourceDecisionPromotionOrigin {
    SourceDecisionPromotionOrigin::from_locator(project_scope_id, locator)
}

pub(crate) fn active_record_shares_promotion_origin(
    record: &EffectiveReusableInfluenceRecord,
    candidate_key: &ReuseCandidateKey,
) -> bool {
    source_decision_promotion_origin(&record.project_scope.stable_id, &record.source_locator)
        == source_decision_promotion_origin(
            &candidate_key.project_scope_id,
            &candidate_key.source_locator,
        )
}

pub(crate) fn has_active_promotion_origin_for_candidate(
    effective: &ReusableInfluenceEffectiveState,
    candidate_key: &ReuseCandidateKey,
) -> bool {
    effective
        .active_records()
        .iter()
        .any(|record| active_record_shares_promotion_origin(record, candidate_key))
}

pub fn source_decision_still_matches_locator(
    ledger: &ReviewLedger,
    locator: &SourceDecisionLocator,
    canonical_run: &CanonicalTermReviewRun,
) -> bool {
    let case_id = locator.source_review_case_id;
    let ReviewCaseStatus::Decided {
        observed_revision,
        decision: CorrectionDecision::ManualReplacement { replacement },
    } = ledger.status_for(case_id)
    else {
        return false;
    };
    let Some((effective_position, _)) = locate_effective_manual_replacement_event(ledger, case_id)
    else {
        return false;
    };
    if effective_position != locator.review_ledger_position {
        return false;
    }
    if decision_digest(case_id, observed_revision, &replacement) != locator.decision_digest {
        return false;
    }
    verify_source_locator_event_fields(ledger, locator, canonical_run).is_ok()
}

pub fn is_manual_replacement_effective(ledger: &ReviewLedger, case_id: ReviewCaseId) -> bool {
    matches!(
        ledger.status_for(case_id),
        ReviewCaseStatus::Decided {
            decision: CorrectionDecision::ManualReplacement { .. },
            ..
        }
    )
}

pub fn fold_effective_state(
    governance: &ReusableInfluenceLedger,
    ledger: &ReviewLedger,
    canonical_run: &CanonicalTermReviewRun,
) -> ReusableInfluenceEffectiveState {
    let mut rejected_candidate_identities = HashSet::new();
    let mut records_by_id: BTreeMap<ReusableInfluenceRecordId, EffectiveReusableInfluenceRecord> =
        BTreeMap::new();
    let mut revoked: HashSet<ReusableInfluenceRecordId> = HashSet::new();
    let mut superseded: BTreeMap<ReusableInfluenceRecordId, ReusableInfluenceRecordId> =
        BTreeMap::new();

    for (event_index, event) in governance.events().iter().enumerate() {
        match event {
            ReusableGovernanceEvent::PromotionCandidateRejected { candidate_key, .. } => {
                rejected_candidate_identities.insert(PromotionCandidateRejectionIdentity::from(
                    candidate_key.as_ref(),
                ));
            }
            ReusableGovernanceEvent::PromotionAccepted {
                payload,
                source_locator,
                actor,
                project_scope,
                ..
            } => {
                let record_id = ReusableInfluenceRecordId::from_promotion_event_index(event_index);
                records_by_id.insert(
                    record_id,
                    EffectiveReusableInfluenceRecord {
                        record_id,
                        project_scope: project_scope.as_ref().clone(),
                        payload: payload.clone(),
                        source_locator: source_locator.as_ref().clone(),
                        promotion_actor: actor.clone(),
                        source_decision_still_effective: source_decision_still_matches_locator(
                            ledger,
                            source_locator.as_ref(),
                            canonical_run,
                        ),
                        superseded_by: None,
                    },
                );
            }
            ReusableGovernanceEvent::ReusableInfluenceRevoked { record_id, .. } => {
                revoked.insert(*record_id);
            }
            ReusableGovernanceEvent::ReusableInfluenceSuperseded {
                predecessor_id,
                successor_id,
                ..
            } => {
                superseded.insert(*predecessor_id, *successor_id);
            }
        }
    }

    for (predecessor, successor) in &superseded {
        if let Some(record) = records_by_id.get_mut(predecessor) {
            record.superseded_by = Some(*successor);
        }
    }

    let mut active_records = Vec::new();
    let mut historical_records = Vec::new();

    for mut record in records_by_id.into_values() {
        let is_revoked = revoked.contains(&record.record_id);
        let is_superseded = superseded.contains_key(&record.record_id);
        record.source_decision_still_effective =
            source_decision_still_matches_locator(ledger, &record.source_locator, canonical_run);
        if is_revoked || is_superseded {
            historical_records.push(record);
        } else {
            active_records.push(record);
        }
    }

    active_records.sort_by_key(|record| record.record_id);
    historical_records.sort_by_key(|record| record.record_id);

    ReusableInfluenceEffectiveState {
        active_records,
        historical_records,
        rejected_candidate_identities,
    }
}

pub fn derive_reuse_candidates(
    transcript: &Transcript,
    canonical_run: &CanonicalTermReviewRun,
    ledger: &ReviewLedger,
    project_scope: &ProjectScope,
    effective: &ReusableInfluenceEffectiveState,
) -> Result<Vec<ReuseCandidate>, ReusableInfluenceError> {
    let analysis_snapshot = canonical_run.analysis_run().snapshot();
    let mut candidates = Vec::new();

    for review_case in canonical_run.review_cases() {
        let case_id = review_case.id();
        let ReviewCaseStatus::Decided {
            observed_revision,
            decision: CorrectionDecision::ManualReplacement { replacement },
        } = ledger.status_for(case_id)
        else {
            continue;
        };

        let Some((ledger_position, _event)) =
            locate_effective_manual_replacement_event(ledger, case_id)
        else {
            return Err(ReusableInfluenceError::InvalidSourceLocator);
        };

        let observed_text = transcript
            .resolve(review_case.candidate_span().anchor())
            .ok_or(ReusableInfluenceError::SourceTextMismatch)?
            .to_owned();

        let source_locator = build_source_decision_locator(
            analysis_snapshot,
            case_id,
            ledger_position,
            observed_revision,
            &replacement,
            ledger.events().len(),
        );
        let key = ReuseCandidateKey {
            source_locator: source_locator.clone(),
            project_scope_id: project_scope.stable_id.clone(),
        };

        if effective
            .rejected_candidate_identities()
            .contains(&PromotionCandidateRejectionIdentity::from(&key))
        {
            continue;
        }
        if effective
            .active_records()
            .iter()
            .any(|record| {
                active_record_shares_promotion_origin(
                    record,
                    &ReuseCandidateKey {
                        source_locator: source_locator.clone(),
                        project_scope_id: project_scope.stable_id.clone(),
                    },
                )
            })
        {
            continue;
        }

        candidates.push(ReuseCandidate {
            key,
            exact_payload: ExactReusableCorrection::from_manual_replacement(
                &observed_text,
                &replacement,
            ),
            proposed_scope: project_scope.stable_id.clone(),
            proposed_allowed_effects: vec![ReuseAllowedEffect::ExactObservedFormProposalGeneration],
            source_decision_still_effective: source_decision_still_matches_locator(
                ledger,
                &source_locator,
                canonical_run,
            ),
        });
    }

    candidates.sort_by(|left, right| {
        left.key
            .source_locator
            .review_ledger_position
            .cmp(&right.key.source_locator.review_ledger_position)
            .then_with(|| {
                left.key
                    .source_locator
                    .source_review_case_id
                    .local_index()
                    .cmp(&right.key.source_locator.source_review_case_id.local_index())
            })
    });
    Ok(candidates)
}

pub(crate) fn build_reusable_influence_snapshot(
    project_scope: &ProjectScope,
    governance: &ReusableInfluenceLedger,
    effective: &ReusableInfluenceEffectiveState,
) -> Result<ReusableInfluenceSnapshot, ReusableInfluenceError> {
    let active_records: Vec<_> = effective
        .active_records()
        .iter()
        .filter(|record| record.project_scope.stable_id == project_scope.stable_id)
        .cloned()
        .collect();
    let governance_event_boundary = governance.events().len();
    let identity_inputs: Vec<_> = active_records
        .iter()
        .map(|record| SnapshotIdentityRecordProvenance {
            record_id: record.record_id,
            observed_text: record.payload.observed_text.as_str(),
            confirmed_replacement: record.payload.confirmed_replacement.as_str(),
            source_locator: &record.source_locator,
            promotion_actor_role: record.promotion_actor.role_label.as_str(),
            promotion_actor_label: record.promotion_actor.display_label.as_str(),
        })
        .collect();
    let identity = compute_snapshot_identity(
        &project_scope.stable_id,
        governance_event_boundary,
        REUSABLE_INFLUENCE_PROJECTION_VERSION,
        &identity_inputs,
    );
    let snapshot = ReusableInfluenceSnapshot {
        project_scope_id: project_scope.stable_id.clone(),
        governance_event_boundary,
        projection_version: REUSABLE_INFLUENCE_PROJECTION_VERSION,
        identity,
        active_records,
    };
    assert_snapshot_identity_matches_contents(&snapshot)?;
    Ok(snapshot)
}

pub fn assert_snapshot_identity_matches_contents(
    snapshot: &ReusableInfluenceSnapshot,
) -> Result<(), ReusableInfluenceError> {
    if snapshot.projection_version() != REUSABLE_INFLUENCE_PROJECTION_VERSION {
        return Err(
            ReusableInfluenceError::UnsupportedSnapshotProjectionVersion {
                version: snapshot.projection_version().to_owned(),
            },
        );
    }

    let active_records = snapshot.active_records();
    if !active_records.is_empty() && snapshot.governance_event_boundary() == 0 {
        return Err(ReusableInfluenceError::NonemptySnapshotRequiresGovernanceBoundary);
    }

    for window in active_records.windows(2) {
        if window[0].record_id > window[1].record_id {
            return Err(ReusableInfluenceError::NonCanonicalActiveRecordOrdering);
        }
    }

    let mut seen = HashSet::new();
    for record in active_records {
        if record.project_scope.stable_id != *snapshot.project_scope_id() {
            return Err(ReusableInfluenceError::SnapshotRecordWrongProjectScope);
        }
        if record.record_id.promotion_event_index() >= snapshot.governance_event_boundary() {
            return Err(ReusableInfluenceError::RecordOutsideGovernanceBoundary {
                record_id: record.record_id,
                governance_event_boundary: snapshot.governance_event_boundary(),
            });
        }
        if !seen.insert(record.record_id) {
            return Err(ReusableInfluenceError::DuplicateActiveRecordIdentity {
                record_id: record.record_id,
            });
        }
    }

    let identity_inputs: Vec<_> = active_records
        .iter()
        .map(|record| SnapshotIdentityRecordProvenance {
            record_id: record.record_id,
            observed_text: record.payload.observed_text.as_str(),
            confirmed_replacement: record.payload.confirmed_replacement.as_str(),
            source_locator: &record.source_locator,
            promotion_actor_role: record.promotion_actor.role_label.as_str(),
            promotion_actor_label: record.promotion_actor.display_label.as_str(),
        })
        .collect();
    let expected_identity = compute_snapshot_identity(
        snapshot.project_scope_id(),
        snapshot.governance_event_boundary(),
        snapshot.projection_version(),
        &identity_inputs,
    );
    if expected_identity != snapshot.identity() {
        return Err(ReusableInfluenceError::SnapshotIdentityMismatch);
    }
    Ok(())
}

pub fn resolve_exact_input_projection(
    project_scope: &ProjectScope,
    snapshot: &ReusableInfluenceSnapshot,
    base_session_terms: &[SessionTermEntry],
) -> Result<ResolvedExactInputProjection, ReusableInfluenceError> {
    assert_snapshot_identity_matches_contents(snapshot)?;
    if snapshot.project_scope_id() != &project_scope.stable_id {
        return Err(ReusableInfluenceError::WrongProjectScope);
    }
    let entries = build_exact_input_projection_entries(snapshot, base_session_terms)?;
    Ok(ResolvedExactInputProjection {
        project_scope_id: project_scope.stable_id.clone(),
        snapshot_identity: snapshot.identity(),
        entries,
    })
}

fn build_exact_input_projection_entries(
    snapshot: &ReusableInfluenceSnapshot,
    base_session_terms: &[SessionTermEntry],
) -> Result<Vec<ExactInputMatcherEntry>, ReusableInfluenceError> {
    let mut mapping: BTreeMap<String, ExactInputMatcherEntry> = BTreeMap::new();

    for entry in base_session_terms {
        for observed_form in &entry.observed_error_forms {
            insert_exact_contribution(
                &mut mapping,
                ExactInputContribution {
                    kind: ExactInputContributionKind::BaseObservedErrorForm,
                    observed_text: observed_form.clone(),
                    confirmed_replacement: entry.canonical_term.clone(),
                    reusable_record_id: None,
                    session_term_canonical: Some(entry.canonical_term.clone()),
                },
            )?;
        }
    }

    for record in snapshot.active_records() {
        insert_exact_contribution(
            &mut mapping,
            ExactInputContribution {
                kind: ExactInputContributionKind::ReusableInfluenceRecord,
                observed_text: record.payload.observed_text.clone(),
                confirmed_replacement: record.payload.confirmed_replacement.clone(),
                reusable_record_id: Some(record.record_id),
                session_term_canonical: None,
            },
        )?;
    }

    let mut entries = mapping.into_values().collect::<Vec<_>>();
    entries.sort_by(|left, right| left.observed_text.cmp(&right.observed_text));
    Ok(entries)
}

fn insert_exact_contribution(
    mapping: &mut BTreeMap<String, ExactInputMatcherEntry>,
    contribution: ExactInputContribution,
) -> Result<(), ReusableInfluenceError> {
    let observed = contribution.observed_text.clone();
    let replacement = contribution.confirmed_replacement.clone();
    match mapping.get_mut(&observed) {
        None => {
            mapping.insert(
                observed.clone(),
                ExactInputMatcherEntry {
                    observed_text: observed,
                    confirmed_replacement: replacement,
                    contributions: vec![contribution],
                },
            );
        }
        Some(entry) => {
            if entry.confirmed_replacement != replacement {
                return Err(ReusableInfluenceError::DivergentExactMapping {
                    observed_text: observed,
                });
            }
            entry.contributions.push(contribution);
            entry.contributions.sort();
        }
    }
    Ok(())
}

pub fn verify_source_locator_against_ledger(
    ledger: &ReviewLedger,
    locator: &SourceDecisionLocator,
    expected_replacement: &str,
    expected_observed_text: &str,
    transcript: &Transcript,
    canonical_run: &CanonicalTermReviewRun,
    review_case: &ReviewCase,
) -> Result<(), ReusableInfluenceError> {
    if locator.effective_at_ledger_length != ledger.events().len() {
        return Err(ReusableInfluenceError::InvalidSourceLocator);
    }
    verify_source_locator_event_fields(ledger, locator, canonical_run)?;
    if expected_replacement
        != match ledger.events().get(locator.review_ledger_position) {
            Some(ReviewLedgerEvent::DecisionRecorded { decision, .. }) => match decision {
                CorrectionDecision::ManualReplacement { replacement } => replacement.as_str(),
                _ => return Err(ReusableInfluenceError::SourceDecisionNotManualReplacement),
            },
            _ => return Err(ReusableInfluenceError::InvalidSourceLocator),
        }
    {
        return Err(ReusableInfluenceError::ReplacementMismatch);
    }
    let observed = transcript
        .resolve(review_case.candidate_span().anchor())
        .ok_or(ReusableInfluenceError::SourceTextMismatch)?;
    if observed != expected_observed_text {
        return Err(ReusableInfluenceError::SourceTextMismatch);
    }
    Ok(())
}

pub fn verify_source_locator_at_historical_boundary(
    ledger: &ReviewLedger,
    locator: &SourceDecisionLocator,
    canonical_run: &CanonicalTermReviewRun,
) -> Result<(), ReusableInfluenceError> {
    verify_source_locator_effective_at_historical_boundary(ledger, locator, canonical_run)
}

pub fn verify_source_locator_effective_at_historical_boundary(
    ledger: &ReviewLedger,
    locator: &SourceDecisionLocator,
    canonical_run: &CanonicalTermReviewRun,
) -> Result<(), ReusableInfluenceError> {
    if locator.effective_at_ledger_length > ledger.events().len() {
        return Err(ReusableInfluenceError::InvalidSourceLocator);
    }
    verify_source_locator_event_fields(ledger, locator, canonical_run)?;
    let case_id = locator.source_review_case_id;
    let ReviewCaseStatus::Decided {
        observed_revision,
        decision: CorrectionDecision::ManualReplacement { replacement },
    } = ledger.status_for_at_prefix(case_id, locator.effective_at_ledger_length)
    else {
        return Err(ReusableInfluenceError::SourceDecisionNotEffective);
    };
    if decision_digest(case_id, observed_revision, &replacement) != locator.decision_digest {
        return Err(ReusableInfluenceError::InvalidSourceLocator);
    }
    let effective_position = locate_effective_manual_replacement_at_prefix(
        ledger,
        case_id,
        locator.effective_at_ledger_length,
    )
    .ok_or(ReusableInfluenceError::InvalidSourceLocator)?;
    if effective_position != locator.review_ledger_position {
        return Err(ReusableInfluenceError::SourceDecisionNotEffective);
    }
    Ok(())
}

pub const DECLARED_LOCAL_OWNER_ROLE: &str = "declared_local_owner_operator";
pub const DECLARED_REVIEWER_ROLE: &str = "declared_authorized_human_reviewer";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActorDisplayLabelValidationError {
    Empty,
    ControlCharacter,
    UnicodeLineSeparator,
    NonCanonical,
}

pub(crate) fn validate_actor_display_label_chars(
    label: &str,
) -> Result<(), ActorDisplayLabelValidationError> {
    if label.chars().any(char::is_control) {
        return Err(ActorDisplayLabelValidationError::ControlCharacter);
    }
    if label
        .chars()
        .any(|character| matches!(character, '\u{2028}' | '\u{2029}'))
    {
        return Err(ActorDisplayLabelValidationError::UnicodeLineSeparator);
    }
    Ok(())
}

pub(crate) fn canonicalize_actor_display_label_for_declaration(
    display_label: impl Into<String>,
) -> Result<String, ActorDisplayLabelValidationError> {
    let display_label = display_label.into();
    validate_actor_display_label_chars(&display_label)?;
    let canonical = display_label.trim().to_string();
    if canonical.is_empty() {
        return Err(ActorDisplayLabelValidationError::Empty);
    }
    Ok(canonical)
}

pub(crate) fn validate_canonical_stored_actor_display_label(
    label: &str,
) -> Result<(), ActorDisplayLabelValidationError> {
    validate_actor_display_label_chars(label)?;
    if label.trim().is_empty() {
        return Err(ActorDisplayLabelValidationError::Empty);
    }
    if label != label.trim() {
        return Err(ActorDisplayLabelValidationError::NonCanonical);
    }
    Ok(())
}

pub fn validate_governance_actor(
    actor: &GovernanceActorContext,
) -> Result<(), ReusableInfluenceError> {
    match actor.role_label.as_str() {
        DECLARED_LOCAL_OWNER_ROLE | DECLARED_REVIEWER_ROLE => {}
        _ => return Err(ReusableInfluenceError::InvalidGovernanceActor),
    }
    validate_canonical_stored_actor_display_label(&actor.display_label)
        .map_err(|_| ReusableInfluenceError::InvalidGovernanceActor)
}

pub fn validate_governance_actor_matches_session(
    actor: &GovernanceActorContext,
    expected: &GovernanceActorContext,
) -> Result<(), ReusableInfluenceError> {
    validate_governance_actor(actor)?;
    if actor != expected {
        return Err(ReusableInfluenceError::InvalidGovernanceActor);
    }
    Ok(())
}

pub fn validate_reuse_candidate_key_at_historical_boundary(
    candidate_key: &ReuseCandidateKey,
    review_ledger: &ReviewLedger,
    canonical_run: &CanonicalTermReviewRun,
    transcript: &Transcript,
    replay_effective: &ReusableInfluenceEffectiveState,
) -> Result<(), ReusableInfluenceError> {
    verify_source_locator_effective_at_historical_boundary(
        review_ledger,
        &candidate_key.source_locator,
        canonical_run,
    )?;
    let review_case = canonical_run
        .review_cases()
        .get(
            candidate_key
                .source_locator
                .source_review_case_id
                .local_index(),
        )
        .ok_or(ReusableInfluenceError::InvalidSourceLocator)?;
    let observed = transcript
        .resolve(review_case.candidate_span().anchor())
        .ok_or(ReusableInfluenceError::SourceTextMismatch)?;
    if observed.is_empty() {
        return Err(ReusableInfluenceError::SourceTextMismatch);
    }
    let event = review_ledger
        .events()
        .get(candidate_key.source_locator.review_ledger_position)
        .ok_or(ReusableInfluenceError::InvalidSourceLocator)?;
    let ReviewLedgerEvent::DecisionRecorded { decision, .. } = event;
    let CorrectionDecision::ManualReplacement { .. } = decision else {
        return Err(ReusableInfluenceError::SourceDecisionNotManualReplacement);
    };
    if replay_effective
        .rejected_candidate_identities()
        .contains(&PromotionCandidateRejectionIdentity::from(candidate_key))
    {
        return Err(ReusableInfluenceError::CandidateAlreadyRejected);
    }
    if has_active_promotion_origin_for_candidate(replay_effective, candidate_key) {
        return Err(ReusableInfluenceError::CandidateAlreadyPromoted);
    }
    Ok(())
}

fn verify_source_locator_event_fields(
    ledger: &ReviewLedger,
    locator: &SourceDecisionLocator,
    canonical_run: &CanonicalTermReviewRun,
) -> Result<(), ReusableInfluenceError> {
    let event = ledger
        .events()
        .get(locator.review_ledger_position)
        .ok_or(ReusableInfluenceError::InvalidSourceLocator)?;
    let ReviewLedgerEvent::DecisionRecorded {
        case_id,
        observed_revision,
        decision,
    } = event;
    if *case_id != locator.source_review_case_id
        || *observed_revision != locator.source_revision
        || canonical_run.analysis_run().snapshot() != locator.source_analysis_snapshot
    {
        return Err(ReusableInfluenceError::InvalidSourceLocator);
    }
    let CorrectionDecision::ManualReplacement { replacement } = decision else {
        return Err(ReusableInfluenceError::SourceDecisionNotManualReplacement);
    };
    if decision_digest(*case_id, *observed_revision, replacement) != locator.decision_digest {
        return Err(ReusableInfluenceError::InvalidSourceLocator);
    }
    Ok(())
}

pub fn validate_projection_against_snapshot(
    projection: &ResolvedExactInputProjection,
    snapshot: &ReusableInfluenceSnapshot,
    base_terms: &[SessionTermEntry],
) -> Result<(), ReusableInfluenceError> {
    assert_projection_matches_expected(projection, snapshot, base_terms)
}

pub fn assert_projection_matches_expected(
    projection: &ResolvedExactInputProjection,
    snapshot: &ReusableInfluenceSnapshot,
    base_terms: &[SessionTermEntry],
) -> Result<(), ReusableInfluenceError> {
    assert_snapshot_identity_matches_contents(snapshot)?;
    if projection.snapshot_identity != snapshot.identity() {
        return Err(ReusableInfluenceError::ProjectionSnapshotIdentityMismatch);
    }
    if projection.project_scope_id != *snapshot.project_scope_id() {
        return Err(ReusableInfluenceError::WrongProjectScope);
    }
    let expected_entries = build_exact_input_projection_entries(snapshot, base_terms)?;
    if projection.entries != expected_entries {
        return Err(ReusableInfluenceError::ProjectionContentMismatch);
    }
    Ok(())
}

pub fn detect_resolved_exact_observed_form_matches(
    run: &crate::analysis::AnalysisRun,
    transcript: &Transcript,
    entries: &[SessionTermEntry],
    projection: &ResolvedExactInputProjection,
    snapshot: &ReusableInfluenceSnapshot,
) -> Result<Vec<CandidateSpan>, crate::candidate::DetectionError> {
    crate::candidate::validate_reuse_enabled_detection_inputs(run, transcript, entries)?;
    assert_projection_matches_expected(projection, snapshot, entries).map_err(
        |error| match error {
            ReusableInfluenceError::ProjectionSnapshotIdentityMismatch => {
                crate::candidate::DetectionError::ProjectionSnapshotIdentityMismatch
            }
            ReusableInfluenceError::ProjectionContentMismatch => {
                crate::candidate::DetectionError::ProjectionContentMismatch
            }
            ReusableInfluenceError::MissingReusableRecordProvenance { record_id } => {
                crate::candidate::DetectionError::MissingReusableRecordProvenance { record_id }
            }
            _ => crate::candidate::DetectionError::ProjectionContentMismatch,
        },
    )?;

    let record_lookup: HashMap<ReusableInfluenceRecordId, &EffectiveReusableInfluenceRecord> =
        snapshot
            .active_records()
            .iter()
            .map(|record| (record.record_id, record))
            .collect();

    let provenance = DetectorProvenance::new(
        RESOLVED_EXACT_OBSERVED_FORM_DETECTOR_ID,
        RESOLVED_EXACT_OBSERVED_FORM_DETECTOR_VERSION,
    );
    let mut spans = Vec::new();

    for entry in projection.entries() {
        let mut exact_input_contributions = Vec::new();
        let mut provenance_contributions = Vec::new();
        let mut promotion_event_indices = Vec::new();
        for contribution in &entry.contributions {
            match contribution.kind {
                ExactInputContributionKind::BaseObservedErrorForm => {
                    let canonical = contribution
                        .session_term_canonical
                        .as_deref()
                        .ok_or(crate::candidate::DetectionError::ProjectionContentMismatch)?;
                    if !entries.iter().any(|term| {
                        term.canonical_term == canonical
                            && term
                                .observed_error_forms
                                .iter()
                                .any(|form| form == &contribution.observed_text)
                    }) {
                        return Err(crate::candidate::DetectionError::ProjectionContentMismatch);
                    }
                    exact_input_contributions.push(
                        ResolvedExactInputContributionEvidence::BaseObservedErrorForm {
                            session_term_canonical: canonical.to_owned(),
                            observed_form: contribution.observed_text.clone(),
                        },
                    );
                }
                ExactInputContributionKind::ReusableInfluenceRecord => {
                    let Some(record_id) = contribution.reusable_record_id else {
                        return Err(
                            crate::candidate::DetectionError::MissingReusableRecordProvenance {
                                record_id: ReusableInfluenceRecordId::from_promotion_event_index(0),
                            },
                        );
                    };
                    let Some(record) = record_lookup.get(&record_id) else {
                        return Err(
                            crate::candidate::DetectionError::MissingReusableRecordProvenance {
                                record_id,
                            },
                        );
                    };
                    if record.payload.observed_text != contribution.observed_text
                        || record.payload.confirmed_replacement
                            != contribution.confirmed_replacement
                    {
                        return Err(crate::candidate::DetectionError::ProjectionContentMismatch);
                    }
                    exact_input_contributions.push(
                        ResolvedExactInputContributionEvidence::ReusableInfluenceRecord {
                            record_id,
                            promotion_event_index: record_id.promotion_event_index(),
                            source_locator: record.source_locator.clone(),
                        },
                    );
                    promotion_event_indices.push(record_id.promotion_event_index());
                    provenance_contributions.push(ReusableProvenanceContribution {
                        record_id,
                        promotion_event_index: record_id.promotion_event_index(),
                        source_locator: record.source_locator.clone(),
                    });
                }
            }
        }
        promotion_event_indices.sort_unstable();
        promotion_event_indices.dedup();

        for (position, segment) in transcript.segments().iter().enumerate() {
            for (start, _matched) in segment.text.match_indices(entry.observed_text.as_str()) {
                let end = start + entry.observed_text.len();
                let anchor = transcript
                    .anchor(position, start, end)
                    .expect("matched substring is a valid anchor");

                let evidence =
                    Evidence::ReusableExactObservedForm(ReusableExactObservedFormEvidence {
                        observed_text: entry.observed_text.clone(),
                        confirmed_replacement: entry.confirmed_replacement.clone(),
                        project_scope_id: projection.project_scope_id.clone(),
                        snapshot_identity: projection.snapshot_identity,
                        exact_input_contributions: exact_input_contributions.clone(),
                        contributions: provenance_contributions.clone(),
                        promotion_event_indices: promotion_event_indices.clone(),
                    });

                spans.push(CandidateSpan::new(
                    DetectionKind::GlossaryAliasMatch,
                    provenance.clone(),
                    anchor,
                    evidence,
                    vec![CandidateAlternative::new(&entry.confirmed_replacement)],
                ));
            }
        }
    }

    Ok(spans)
}

#[cfg(test)]
mod correction_03_snapshot_integrity_tests {
    use super::*;
    use crate::candidate::DetectionError;
    use crate::pipeline::run_canonical_term_review;
    use crate::reuse_primitives::ProjectScopeDisplayName;

    fn sample_scope() -> ProjectScope {
        ProjectScope::new(
            ProjectScopeId::new("proj-a").expect("scope id"),
            ProjectScopeDisplayName::new("Project A").expect("name"),
        )
    }

    pub(super) fn legitimate_snapshot_with_promotion() -> ReusableInfluenceSnapshot {
        let transcript =
            crate::srt::parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
        let terms = vec![SessionTermEntry::new(
            "Kafka",
            vec!["Kafak".to_string()],
            Vec::new(),
        )];
        let canonical = run_canonical_term_review(&transcript, &terms).expect("canonical");
        let scope = sample_scope();
        let mut review_ledger = ReviewLedger::new();
        let review_case = &canonical.review_cases()[0];
        review_ledger
            .record_decision(
                review_case,
                transcript.revision_id(),
                CorrectionDecision::ManualReplacement {
                    replacement: ManualReplacementText::new("Kafka", "Kafak").expect("replacement"),
                },
            )
            .expect("decision");
        let mut ledger = ReusableInfluenceLedger::new();
        let effective = fold_effective_state(&ledger, &review_ledger, &canonical);
        let key =
            derive_reuse_candidates(&transcript, &canonical, &review_ledger, &scope, &effective)
                .expect("candidates")[0]
                .key
                .clone();
        ledger.append(ReusableGovernanceEvent::PromotionAccepted {
            candidate_key: Box::new(key.clone()),
            payload: ExactReusableCorrection {
                observed_text: "Kafak".to_owned(),
                confirmed_replacement: "Kafka".to_owned(),
            },
            source_locator: Box::new(key.source_locator.clone()),
            actor: GovernanceActorContext {
                role_label: DECLARED_LOCAL_OWNER_ROLE.to_owned(),
                display_label: "Ezra".to_owned(),
            },
            project_scope: Box::new(scope.clone()),
        });
        let effective = fold_effective_state(&ledger, &review_ledger, &canonical);
        build_reusable_influence_snapshot(&scope, &ledger, &effective).expect("snapshot")
    }

    #[test]
    fn legitimate_fold_and_snapshot_identity_remain_deterministic() {
        let left = legitimate_snapshot_with_promotion();
        let right = legitimate_snapshot_with_promotion();
        assert_eq!(left.identity(), right.identity());
    }

    #[test]
    fn tampered_snapshot_identity_fails_closed() {
        let mut snapshot = legitimate_snapshot_with_promotion();
        snapshot
            .replace_identity_for_test(ReusableInfluenceSnapshotIdentity::from_digest([1u8; 32]));
        assert!(matches!(
            assert_snapshot_identity_matches_contents(&snapshot),
            Err(ReusableInfluenceError::SnapshotIdentityMismatch)
        ));
    }

    #[test]
    fn wrong_scope_record_fails_snapshot_integrity() {
        let mut snapshot = legitimate_snapshot_with_promotion();
        let mut wrong_scope_record = snapshot.active_records()[0].clone();
        wrong_scope_record.project_scope = ProjectScope::new(
            ProjectScopeId::new("proj-b").expect("scope id"),
            ProjectScopeDisplayName::new("Project B").expect("name"),
        );
        snapshot.clear_active_records_for_test();
        snapshot.push_active_record_out_of_order_for_test(wrong_scope_record);
        assert!(matches!(
            assert_snapshot_identity_matches_contents(&snapshot),
            Err(ReusableInfluenceError::SnapshotRecordWrongProjectScope)
        ));
    }

    #[test]
    fn duplicate_active_record_ids_fail_snapshot_integrity() {
        let mut snapshot = legitimate_snapshot_with_promotion();
        let duplicate = snapshot.active_records()[0].clone();
        snapshot.push_active_record_out_of_order_for_test(duplicate);
        assert!(matches!(
            assert_snapshot_identity_matches_contents(&snapshot),
            Err(ReusableInfluenceError::DuplicateActiveRecordIdentity { .. })
        ));
    }

    #[test]
    fn non_canonical_active_record_ordering_fails_snapshot_integrity() {
        let mut snapshot = legitimate_snapshot_with_promotion();
        let mut high = snapshot.active_records()[0].clone();
        high.record_id = ReusableInfluenceRecordId::from_promotion_event_index(1);
        let mut low = snapshot.active_records()[0].clone();
        low.record_id = ReusableInfluenceRecordId::from_promotion_event_index(0);
        snapshot.clear_active_records_for_test();
        snapshot.push_active_record_out_of_order_for_test(high);
        snapshot.push_active_record_out_of_order_for_test(low);
        assert!(matches!(
            assert_snapshot_identity_matches_contents(&snapshot),
            Err(ReusableInfluenceError::NonCanonicalActiveRecordOrdering)
        ));
    }

    #[test]
    fn unsupported_projection_version_fails_snapshot_integrity() {
        let mut snapshot = legitimate_snapshot_with_promotion();
        snapshot.set_projection_version_for_test("unsupported-version");
        assert!(matches!(
            assert_snapshot_identity_matches_contents(&snapshot),
            Err(ReusableInfluenceError::UnsupportedSnapshotProjectionVersion { .. })
        ));
    }

    #[test]
    fn legitimate_snapshot_passes_integrity_validation() {
        let snapshot = legitimate_snapshot_with_promotion();
        assert_snapshot_identity_matches_contents(&snapshot).expect("valid");
    }

    #[test]
    fn projection_generation_refuses_invalid_snapshot() {
        let scope = sample_scope();
        let terms = vec![SessionTermEntry::new(
            "Kafka",
            vec!["Kafak".to_string()],
            Vec::new(),
        )];
        let mut snapshot = legitimate_snapshot_with_promotion();
        snapshot
            .replace_identity_for_test(ReusableInfluenceSnapshotIdentity::from_digest([2u8; 32]));
        assert!(matches!(
            resolve_exact_input_projection(&scope, &snapshot, &terms),
            Err(ReusableInfluenceError::SnapshotIdentityMismatch)
        ));
    }

    #[test]
    fn detector_execution_refuses_invalid_snapshot() {
        let scope = sample_scope();
        let terms = vec![SessionTermEntry::new(
            "Kafka",
            vec!["Kafak".to_string()],
            Vec::new(),
        )];
        let transcript =
            crate::srt::parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
        let mut snapshot = legitimate_snapshot_with_promotion();
        let projection =
            resolve_exact_input_projection(&scope, &snapshot, &terms).expect("projection");
        snapshot
            .replace_identity_for_test(ReusableInfluenceSnapshotIdentity::from_digest([3u8; 32]));
        let run =
            crate::analysis::AnalysisRun::for_reuse_enabled_session_terms(&transcript, &terms);
        assert!(matches!(
            detect_resolved_exact_observed_form_matches(
                &run,
                &transcript,
                &terms,
                &projection,
                &snapshot,
            ),
            Err(DetectionError::ProjectionSnapshotIdentityMismatch)
                | Err(DetectionError::ProjectionContentMismatch)
        ));
    }

    #[test]
    fn projection_validation_refuses_stripped_snapshot_contents() {
        let scope = sample_scope();
        let terms = vec![SessionTermEntry::new(
            "Kafka",
            vec!["Kafak".to_string()],
            Vec::new(),
        )];
        let snapshot = legitimate_snapshot_with_promotion();
        let projection =
            resolve_exact_input_projection(&scope, &snapshot, &terms).expect("projection");
        let mut stripped_snapshot = snapshot.clone();
        stripped_snapshot.clear_active_records_for_test();
        assert!(matches!(
            assert_projection_matches_expected(&projection, &stripped_snapshot, &terms),
            Err(ReusableInfluenceError::SnapshotIdentityMismatch)
                | Err(ReusableInfluenceError::ProjectionContentMismatch)
        ));
    }
}

#[cfg(test)]
mod correction_04_governance_boundary_tests {
    use super::*;
    use correction_03_snapshot_integrity_tests::legitimate_snapshot_with_promotion;

    #[test]
    fn nonempty_active_records_with_zero_boundary_fail_integrity() {
        let mut snapshot = legitimate_snapshot_with_promotion();
        snapshot.set_governance_event_boundary_for_test(0);
        assert!(matches!(
            assert_snapshot_identity_matches_contents(&snapshot),
            Err(ReusableInfluenceError::NonemptySnapshotRequiresGovernanceBoundary)
        ));
    }

    #[test]
    fn record_at_boundary_fails_integrity() {
        let mut snapshot = legitimate_snapshot_with_promotion();
        let mut record = snapshot.active_records()[0].clone();
        let boundary = snapshot.governance_event_boundary();
        record.record_id = ReusableInfluenceRecordId::from_promotion_event_index(boundary);
        snapshot.clear_active_records_for_test();
        snapshot.push_active_record_out_of_order_for_test(record);
        assert!(matches!(
            assert_snapshot_identity_matches_contents(&snapshot),
            Err(ReusableInfluenceError::RecordOutsideGovernanceBoundary { .. })
        ));
    }

    #[test]
    fn record_above_boundary_fails_integrity() {
        let mut snapshot = legitimate_snapshot_with_promotion();
        let mut record = snapshot.active_records()[0].clone();
        let boundary = snapshot.governance_event_boundary();
        record.record_id =
            ReusableInfluenceRecordId::from_promotion_event_index(boundary.saturating_add(1));
        snapshot.clear_active_records_for_test();
        snapshot.push_active_record_out_of_order_for_test(record);
        assert!(matches!(
            assert_snapshot_identity_matches_contents(&snapshot),
            Err(ReusableInfluenceError::RecordOutsideGovernanceBoundary { .. })
        ));
    }

    #[test]
    fn record_below_boundary_passes_integrity() {
        let snapshot = legitimate_snapshot_with_promotion();
        assert_snapshot_identity_matches_contents(&snapshot).expect("valid");
        let record = snapshot.active_records()[0]
            .record_id
            .promotion_event_index();
        assert!(record < snapshot.governance_event_boundary());
    }
}

#[cfg(test)]
mod projection_authenticity_tests {
    use super::*;
    use crate::candidate::DetectionError;
    use crate::pipeline::run_canonical_term_review;

    fn sample_scope() -> ProjectScope {
        ProjectScope::new(
            ProjectScopeId::new("proj-test").expect("scope id"),
            crate::reuse_primitives::ProjectScopeDisplayName::new("Project Test").expect("name"),
        )
    }

    #[test]
    fn forged_projection_with_legitimate_snapshot_identity_fails_detection() {
        let transcript =
            crate::srt::parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
        let terms = vec![SessionTermEntry::new(
            "Kafka",
            vec![],
            vec!["Kafak".to_string()],
        )];
        let canonical = run_canonical_term_review(&transcript, &terms).expect("canonical");
        let scope = sample_scope();
        let ledger = ReviewLedger::new();
        let effective = fold_effective_state(&ReusableInfluenceLedger::new(), &ledger, &canonical);
        let snapshot =
            build_reusable_influence_snapshot(&scope, &ReusableInfluenceLedger::new(), &effective)
                .expect("snapshot");
        let projection =
            resolve_exact_input_projection(&scope, &snapshot, &terms).expect("projection");
        let mut forged_entries = projection.entries.clone();
        forged_entries[0].confirmed_replacement = "Evil".to_owned();
        let forged = ResolvedExactInputProjection {
            project_scope_id: projection.project_scope_id.clone(),
            snapshot_identity: projection.snapshot_identity,
            entries: forged_entries,
        };
        let run =
            crate::analysis::AnalysisRun::for_reuse_enabled_session_terms(&transcript, &terms);
        assert!(matches!(
            detect_resolved_exact_observed_form_matches(
                &run,
                &transcript,
                &terms,
                &forged,
                &snapshot,
            ),
            Err(DetectionError::ProjectionContentMismatch)
        ));
    }
}

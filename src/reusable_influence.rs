use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;

use crate::analysis::AnalysisSnapshot;
use crate::candidate::{
    CandidateAlternative, CandidateSpan, DetectionKind, DetectorProvenance, Evidence,
    ReusableExactObservedFormEvidence, ReusableProvenanceContribution, SessionTermEntry,
};
use crate::pipeline::CanonicalTermReviewRun;
use crate::reuse_primitives::{
    ProjectScope, ProjectScopeId, ReusableInfluenceRecordId, ReusableInfluenceSnapshotIdentity,
    SourceDecisionLocator, compute_snapshot_identity, decision_digest,
};
use crate::review::{
    CorrectionDecision, ManualReplacementText, ReviewCase, ReviewCaseId, ReviewCaseStatus,
    ReviewLedger, ReviewLedgerEvent,
};
use crate::transcript::Transcript;

pub use crate::reuse_primitives::ProjectScopeTextError;

pub const REUSABLE_EXACT_OBSERVED_FORM_DETECTOR_ID: &str = "reusable-exact-observed-form-match";
pub const REUSABLE_EXACT_OBSERVED_FORM_DETECTOR_VERSION: &str = "0.1.0";
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

    pub fn append(&mut self, event: ReusableGovernanceEvent) -> usize {
        let index = self.events.len();
        self.events.push(event);
        index
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReusableInfluenceEffectiveState {
    pub active_records: Vec<EffectiveReusableInfluenceRecord>,
    pub historical_records: Vec<EffectiveReusableInfluenceRecord>,
    pub rejected_candidate_keys: HashSet<ReuseCandidateKey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReusableInfluenceSnapshot {
    pub project_scope_id: ProjectScopeId,
    pub governance_event_boundary: usize,
    pub projection_version: &'static str,
    pub identity: ReusableInfluenceSnapshotIdentity,
    pub active_records: Vec<EffectiveReusableInfluenceRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ExactInputContributionKind {
    BaseObservedErrorForm,
    ReusableInfluenceRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ExactInputContribution {
    pub kind: ExactInputContributionKind,
    pub observed_text: String,
    pub confirmed_replacement: String,
    pub reusable_record_id: Option<ReusableInfluenceRecordId>,
    pub session_term_canonical: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactInputMatcherEntry {
    pub observed_text: String,
    pub confirmed_replacement: String,
    pub contributions: Vec<ExactInputContribution>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedExactInputProjection {
    pub project_scope_id: ProjectScopeId,
    pub snapshot_identity: ReusableInfluenceSnapshotIdentity,
    pub entries: Vec<ExactInputMatcherEntry>,
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
    WrongProjectScope,
}

impl fmt::Display for ReusableInfluenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ReusableInfluenceError {}

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
) -> ReusableInfluenceEffectiveState {
    let mut rejected_candidate_keys = HashSet::new();
    let mut records_by_id: BTreeMap<ReusableInfluenceRecordId, EffectiveReusableInfluenceRecord> =
        BTreeMap::new();
    let mut revoked: HashSet<ReusableInfluenceRecordId> = HashSet::new();
    let mut superseded: BTreeMap<ReusableInfluenceRecordId, ReusableInfluenceRecordId> =
        BTreeMap::new();

    for (event_index, event) in governance.events().iter().enumerate() {
        match event {
            ReusableGovernanceEvent::PromotionCandidateRejected { candidate_key, .. } => {
                rejected_candidate_keys.insert(candidate_key.as_ref().clone());
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
                        source_decision_still_effective: is_manual_replacement_effective(
                            ledger,
                            source_locator.source_review_case_id,
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
            is_manual_replacement_effective(ledger, record.source_locator.source_review_case_id);
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
        rejected_candidate_keys,
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

        if effective.rejected_candidate_keys.contains(&key) {
            continue;
        }
        if effective
            .active_records
            .iter()
            .any(|record| record.source_locator == source_locator)
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
            source_decision_still_effective: true,
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

pub fn build_reusable_influence_snapshot(
    project_scope: &ProjectScope,
    governance: &ReusableInfluenceLedger,
    effective: &ReusableInfluenceEffectiveState,
) -> ReusableInfluenceSnapshot {
    let active_records: Vec<_> = effective
        .active_records
        .iter()
        .filter(|record| record.project_scope.stable_id == project_scope.stable_id)
        .cloned()
        .collect();
    let governance_event_boundary = governance.events().len();
    let identity_inputs: Vec<_> = active_records
        .iter()
        .map(|record| {
            (
                record.record_id,
                record.payload.observed_text.as_str(),
                record.payload.confirmed_replacement.as_str(),
                record.source_locator.decision_digest,
                record.source_locator.review_ledger_position,
            )
        })
        .collect();
    let identity = compute_snapshot_identity(
        &project_scope.stable_id,
        governance_event_boundary,
        REUSABLE_INFLUENCE_PROJECTION_VERSION,
        &identity_inputs,
    );
    ReusableInfluenceSnapshot {
        project_scope_id: project_scope.stable_id.clone(),
        governance_event_boundary,
        projection_version: REUSABLE_INFLUENCE_PROJECTION_VERSION,
        identity,
        active_records,
    }
}

pub fn resolve_exact_input_projection(
    project_scope: &ProjectScope,
    snapshot: &ReusableInfluenceSnapshot,
    base_session_terms: &[SessionTermEntry],
) -> Result<ResolvedExactInputProjection, ReusableInfluenceError> {
    if snapshot.project_scope_id != project_scope.stable_id {
        return Err(ReusableInfluenceError::WrongProjectScope);
    }

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

    for record in &snapshot.active_records {
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

    Ok(ResolvedExactInputProjection {
        project_scope_id: project_scope.stable_id.clone(),
        snapshot_identity: snapshot.identity,
        entries,
    })
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
    if replacement.as_str() != expected_replacement {
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

pub fn detect_reusable_exact_observed_form_matches(
    transcript: &Transcript,
    projection: &ResolvedExactInputProjection,
    snapshot: &ReusableInfluenceSnapshot,
) -> Vec<CandidateSpan> {
    let record_lookup: HashMap<ReusableInfluenceRecordId, &EffectiveReusableInfluenceRecord> =
        snapshot
            .active_records
            .iter()
            .map(|record| (record.record_id, record))
            .collect();

    let provenance = DetectorProvenance::new(
        REUSABLE_EXACT_OBSERVED_FORM_DETECTOR_ID,
        REUSABLE_EXACT_OBSERVED_FORM_DETECTOR_VERSION,
    );
    let mut spans = Vec::new();

    for entry in &projection.entries {
        let reusable_contributions: Vec<&ExactInputContribution> = entry
            .contributions
            .iter()
            .filter(|contribution| {
                matches!(
                    contribution.kind,
                    ExactInputContributionKind::ReusableInfluenceRecord
                )
            })
            .collect();
        if reusable_contributions.is_empty() {
            continue;
        }

        let mut provenance_contributions = Vec::new();
        let mut promotion_event_indices = Vec::new();
        for contribution in &reusable_contributions {
            let Some(record_id) = contribution.reusable_record_id else {
                continue;
            };
            let Some(record) = record_lookup.get(&record_id) else {
                continue;
            };
            promotion_event_indices.push(record_id.promotion_event_index());
            provenance_contributions.push(ReusableProvenanceContribution {
                record_id,
                promotion_event_index: record_id.promotion_event_index(),
                source_locator: record.source_locator.clone(),
            });
        }
        if provenance_contributions.is_empty() {
            continue;
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

    spans
}

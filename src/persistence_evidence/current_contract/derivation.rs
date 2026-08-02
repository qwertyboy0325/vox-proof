use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::review::{CorrectionDecision, ManualReplacementText};

use super::model::{
    CurrentContractState, DerivedContractProjection, EvidenceCorrectionDecision,
    EvidenceEffectiveReviewStatus, EvidenceExactReusableCorrection, EvidenceReusableRecord,
    EvidenceReuseCandidateKey, EvidenceReuseGovernanceEvent, EvidenceReviewLedgerEvent,
    EvidenceSourceDecisionLocator, REUSABLE_INFLUENCE_PROJECTION_VERSION,
};
use super::production_bridge::{
    compute_production_snapshot_identity, is_canonical_analysis_snapshot,
    is_reuse_enabled_analysis_snapshot, parse_decision_digest_hex, parse_review_case_id,
    production_decision_digest_hex, validate_analysis_snapshot,
    validate_governance_actor_session_bound, validate_revision_id,
};
use super::serialization::rejection_identity_digest;
use super::violations::{OracleDiagnosticV3, OracleViolationCodeV3, diagnostic};

pub fn finalize_derived_fields(state: &mut CurrentContractState) {
    let (derived, _) = derive_contract_projection(state);
    state.effective_review_status = derived.effective_review_status;
    state.effective_reusable_records = derived.effective_reusable_records;
    state.historical_reusable_records = derived.historical_reusable_records;
    state.reusable_snapshot_identity = derived.reusable_snapshot_identity;
    state.derived_queue_projection = derived.derived_queue_projection;
}

pub fn derive_contract_projection(
    state: &CurrentContractState,
) -> (DerivedContractProjection, Vec<OracleDiagnosticV3>) {
    let mut violations = Vec::new();
    validate_review_cases_and_ledger(state, &mut violations);
    let validated_review_cases = validate_source_anchors(state, &mut violations);
    validate_governance_events(state, &mut violations);
    let validated_review_decisions =
        validate_review_ledger(state, &validated_review_cases, &mut violations);
    validate_historical_binding(
        state,
        &validated_review_cases,
        &validated_review_decisions,
        &mut violations,
    );

    let effective_review_status = fold_effective_review_status(&validated_review_decisions);
    let (effective_reusable_records, historical_reusable_records, rejected_candidate_identities) =
        fold_reuse_governance(
            state,
            &validated_review_cases,
            &validated_review_decisions,
            state.reuse_governance_events.len(),
            &mut violations,
        );
    let reusable_snapshot_identity = recompute_snapshot_identity(
        state,
        &effective_reusable_records,
        state.reuse_governance_events.len(),
        &mut violations,
    );
    let derived_queue_projection = format!(
        "queue:{}:{}",
        state
            .source_revisions
            .first()
            .map(|revision| revision.revision_id.as_str())
            .unwrap_or("missing"),
        state.review_ledger_events.len()
    );

    (
        DerivedContractProjection {
            effective_review_status,
            rejected_candidate_identities,
            effective_reusable_records,
            historical_reusable_records,
            reusable_snapshot_identity,
            derived_queue_projection,
        },
        violations,
    )
}

#[derive(Debug, Clone)]
struct ValidatedReviewDecision {
    case_id: String,
    observed_revision_id: String,
    decision: CorrectionDecision,
}

#[derive(Debug, Clone)]
struct ValidatedReviewCase {
    observed_revision_id: String,
    anchor_revision_id: String,
    observed_source_bytes: String,
    alternative_count: usize,
}

fn validate_review_ledger(
    state: &CurrentContractState,
    validated_review_cases: &BTreeMap<String, ValidatedReviewCase>,
    violations: &mut Vec<OracleDiagnosticV3>,
) -> Vec<Option<ValidatedReviewDecision>> {
    let mut validated = Vec::with_capacity(state.review_ledger_events.len());
    for (position, event) in state.review_ledger_events.iter().enumerate() {
        if event.event_index != position {
            violations.push(diagnostic(
                OracleViolationCodeV3::ReviewLedgerIndexGap,
                "review_ledger_events",
                "review ledger event index is not contiguous",
            ));
            validated.push(None);
            continue;
        }
        let Some(decision) =
            validate_and_decode_review_event(state, validated_review_cases, event, violations)
        else {
            validated.push(None);
            continue;
        };
        if let Err(diagnostic) = validate_revision_id(&event.observed_revision_id) {
            violations.push(diagnostic);
            validated.push(None);
            continue;
        }
        validated.push(Some(ValidatedReviewDecision {
            case_id: event.case_id.clone(),
            observed_revision_id: event.observed_revision_id.clone(),
            decision,
        }));
    }
    validated
}

fn fold_effective_review_status(
    validated_review_decisions: &[Option<ValidatedReviewDecision>],
) -> Vec<EvidenceEffectiveReviewStatus> {
    let mut latest: BTreeMap<String, EvidenceEffectiveReviewStatus> = BTreeMap::new();
    for validated in validated_review_decisions {
        let Some(validated) = validated else {
            continue;
        };
        latest.insert(
            validated.case_id.clone(),
            EvidenceEffectiveReviewStatus {
                case_id: validated.case_id.clone(),
                observed_revision_id: validated.observed_revision_id.clone(),
                decision: evidence_decision(&validated.decision),
            },
        );
    }
    latest.into_values().collect()
}

fn evidence_decision(decision: &CorrectionDecision) -> EvidenceCorrectionDecision {
    match decision {
        CorrectionDecision::AcceptAlternative { alternative_index } => {
            EvidenceCorrectionDecision::AcceptAlternative {
                alternative_index: *alternative_index,
            }
        }
        CorrectionDecision::ManualReplacement { replacement } => {
            EvidenceCorrectionDecision::ManualReplacement {
                replacement: replacement.as_str().to_owned(),
            }
        }
        CorrectionDecision::Reject => EvidenceCorrectionDecision::Reject,
        CorrectionDecision::Defer => EvidenceCorrectionDecision::Defer,
        CorrectionDecision::NeedsManualCorrection => {
            EvidenceCorrectionDecision::NeedsManualCorrection
        }
    }
}

fn validate_and_decode_review_event(
    state: &CurrentContractState,
    validated_review_cases: &BTreeMap<String, ValidatedReviewCase>,
    event: &EvidenceReviewLedgerEvent,
    violations: &mut Vec<OracleDiagnosticV3>,
) -> Option<CorrectionDecision> {
    let path = format!("review_ledger_events[{}]", event.event_index);
    let review_case = state
        .review_cases
        .iter()
        .find(|case| case.case_id == event.case_id);
    let Some(review_case) = review_case else {
        violations.push(diagnostic(
            OracleViolationCodeV3::MissingReviewCaseReference,
            &path,
            "review ledger references missing review case",
        ));
        return None;
    };
    let Some(validated_review_case) = validated_review_cases.get(&event.case_id) else {
        violations.push(diagnostic(
            OracleViolationCodeV3::SourceAnchorResolvedBytesMismatch,
            &path,
            "review ledger cannot consume a review case with an invalid source anchor",
        ));
        return None;
    };
    if event.target_event_index.is_some() {
        violations.push(diagnostic(
            OracleViolationCodeV3::MalformedReviewLedgerEvent,
            &path,
            "target_event_index is not part of an MD-017 review action",
        ));
        return None;
    }
    if parse_review_case_id(&event.case_id).is_err() {
        violations.push(diagnostic(
            OracleViolationCodeV3::MalformedReviewCaseId,
            &path,
            "review ledger event references malformed review case id",
        ));
        return None;
    }
    if !state
        .source_revisions
        .iter()
        .any(|revision| revision.revision_id == event.observed_revision_id)
    {
        violations.push(diagnostic(
            OracleViolationCodeV3::ChangedSourceRevisionIdentity,
            &path,
            "review ledger references missing source revision",
        ));
        return None;
    }
    if event.observed_revision_id != review_case.observed_revision_id {
        violations.push(diagnostic(
            OracleViolationCodeV3::ReviewLedgerEventRevisionMismatch,
            &path,
            "review ledger event observed revision differs from review case",
        ));
        return None;
    }
    match event.provenance.as_str() {
        "human" => {}
        "automatic" => {
            violations.push(diagnostic(
                OracleViolationCodeV3::FabricatedAutomaticDecision,
                &path,
                "automatic provenance is forbidden",
            ));
            return None;
        }
        _ => {
            violations.push(diagnostic(
                OracleViolationCodeV3::UnknownDecisionProvenance,
                &path,
                "unknown review ledger provenance",
            ));
            return None;
        }
    }

    match event.action_kind.as_str() {
        "manual_replacement" => {
            if event.alternative_index.is_some() {
                violations.push(diagnostic(
                    OracleViolationCodeV3::MalformedReviewLedgerEvent,
                    &path,
                    "manual replacement must not include alternative index",
                ));
                return None;
            }
            let Some(replacement_text) = event.manual_replacement_bytes.as_deref() else {
                violations.push(diagnostic(
                    OracleViolationCodeV3::MalformedReviewLedgerEvent,
                    &path,
                    "manual replacement bytes are required",
                ));
                return None;
            };
            if replacement_text.is_empty() {
                violations.push(diagnostic(
                    OracleViolationCodeV3::MalformedReviewLedgerEvent,
                    &path,
                    "manual replacement bytes must not be empty",
                ));
                return None;
            }
            match ManualReplacementText::new(
                replacement_text,
                validated_review_case.observed_source_bytes.as_str(),
            ) {
                Ok(replacement) => Some(CorrectionDecision::ManualReplacement { replacement }),
                Err(_) => {
                    violations.push(diagnostic(
                        OracleViolationCodeV3::MalformedReviewLedgerEvent,
                        &path,
                        "manual replacement bytes are invalid for observed source bytes",
                    ));
                    None
                }
            }
        }
        "accept_alternative" => {
            if event.manual_replacement_bytes.is_some() {
                violations.push(diagnostic(
                    OracleViolationCodeV3::MalformedReviewLedgerEvent,
                    &path,
                    "accept alternative must not include manual replacement bytes",
                ));
                return None;
            }
            let Some(alternative_index) = event.alternative_index else {
                violations.push(diagnostic(
                    OracleViolationCodeV3::MalformedReviewLedgerEvent,
                    &path,
                    "accept alternative requires alternative index",
                ));
                return None;
            };
            if alternative_index >= validated_review_case.alternative_count {
                violations.push(diagnostic(
                    OracleViolationCodeV3::MalformedReviewLedgerEvent,
                    &path,
                    "accept alternative index is outside the recorded review-case alternatives",
                ));
                return None;
            }
            Some(CorrectionDecision::AcceptAlternative { alternative_index })
        }
        "reject" | "defer" | "needs_manual_correction" => {
            if event.manual_replacement_bytes.is_some() || event.alternative_index.is_some() {
                violations.push(diagnostic(
                    OracleViolationCodeV3::MalformedReviewLedgerEvent,
                    &path,
                    "non-replacement actions must not include replacement payload fields",
                ));
                return None;
            }
            Some(match event.action_kind.as_str() {
                "reject" => CorrectionDecision::Reject,
                "defer" => CorrectionDecision::Defer,
                _ => CorrectionDecision::NeedsManualCorrection,
            })
        }
        other => {
            violations.push(diagnostic(
                OracleViolationCodeV3::ChangedReviewLedgerPayload,
                &path,
                &format!("unsupported action kind {other}"),
            ));
            None
        }
    }
}

struct GovernanceFoldState {
    rejected: BTreeSet<String>,
    records_by_id: BTreeMap<usize, EvidenceReusableRecord>,
    revoked: HashSet<usize>,
    superseded: BTreeMap<usize, usize>,
}

impl GovernanceFoldState {
    fn new() -> Self {
        Self {
            rejected: BTreeSet::new(),
            records_by_id: BTreeMap::new(),
            revoked: HashSet::new(),
            superseded: BTreeMap::new(),
        }
    }

    fn finalize(self) -> (Vec<EvidenceReusableRecord>, Vec<EvidenceReusableRecord>) {
        let mut effective_reusable_records = Vec::new();
        let mut historical_reusable_records = Vec::new();
        for record in self.records_by_id.into_values() {
            if self.revoked.contains(&record.record_id)
                || self.superseded.contains_key(&record.record_id)
            {
                historical_reusable_records.push(record);
            } else {
                effective_reusable_records.push(record);
            }
        }
        effective_reusable_records.sort_by_key(|record| record.record_id);
        historical_reusable_records.sort_by_key(|record| record.record_id);
        (effective_reusable_records, historical_reusable_records)
    }
}

fn fold_reuse_governance(
    state: &CurrentContractState,
    validated_review_cases: &BTreeMap<String, ValidatedReviewCase>,
    validated_review_decisions: &[Option<ValidatedReviewDecision>],
    event_limit: usize,
    violations: &mut Vec<OracleDiagnosticV3>,
) -> (
    Vec<EvidenceReusableRecord>,
    Vec<EvidenceReusableRecord>,
    Vec<String>,
) {
    let mut fold = GovernanceFoldState::new();
    for (expected_event_index, event) in state
        .reuse_governance_events
        .iter()
        .take(event_limit)
        .enumerate()
    {
        if governance_event_index(event) != expected_event_index {
            violations.push(diagnostic(
                OracleViolationCodeV3::ReuseGovernanceIndexGap,
                "reuse_governance_events",
                "reuse governance event index is not contiguous",
            ));
            continue;
        }
        apply_governance_event(
            state,
            validated_review_cases,
            validated_review_decisions,
            event,
            &mut fold,
            violations,
        );
    }
    let rejected: Vec<String> = fold.rejected.iter().cloned().collect();
    let (effective, historical) = fold.finalize();
    (effective, historical, rejected)
}

fn apply_governance_event(
    state: &CurrentContractState,
    validated_review_cases: &BTreeMap<String, ValidatedReviewCase>,
    validated_review_decisions: &[Option<ValidatedReviewDecision>],
    event: &EvidenceReuseGovernanceEvent,
    fold: &mut GovernanceFoldState,
    violations: &mut Vec<OracleDiagnosticV3>,
) {
    match event {
        EvidenceReuseGovernanceEvent::PromotionCandidateRejected {
            event_index,
            candidate_key,
            actor,
        } => {
            let path = format!("reuse_governance_events[{event_index}]");
            if !validate_governance_actor_session_bound(
                actor,
                &state.session_authority,
                &path,
                violations,
            ) {
                return;
            }
            if !validate_candidate_key(
                state,
                validated_review_cases,
                validated_review_decisions,
                candidate_key,
                violations,
            ) {
                return;
            }
            let identity = rejection_identity_digest(
                &candidate_key.project_scope_stable_id,
                &candidate_key.source_locator.source_review_case_id,
                candidate_key.source_locator.review_ledger_position,
                &candidate_key.source_locator.decision_digest,
            );
            if fold.rejected.contains(&identity) {
                violations.push(diagnostic(
                    OracleViolationCodeV3::DuplicateRejectionIdentity,
                    &path,
                    "candidate has already been rejected",
                ));
                return;
            }
            if fold.records_by_id.values().any(|record| {
                record.source_locator == candidate_key.source_locator
                    && record.project_scope_stable_id == candidate_key.project_scope_stable_id
            }) {
                violations.push(diagnostic(
                    OracleViolationCodeV3::PromotionAfterRejectedCandidate,
                    &path,
                    "candidate rejection after promotion is forbidden",
                ));
                return;
            }
            fold.rejected.insert(identity);
        }
        EvidenceReuseGovernanceEvent::PromotionAccepted {
            event_index,
            candidate_key,
            payload,
            source_locator,
            actor,
            project_scope_stable_id,
        } => {
            let path = format!("reuse_governance_events[{event_index}]");
            if !validate_governance_actor_session_bound(
                actor,
                &state.session_authority,
                &path,
                violations,
            ) {
                return;
            }
            if !validate_candidate_key(
                state,
                validated_review_cases,
                validated_review_decisions,
                candidate_key,
                violations,
            ) {
                return;
            }
            if candidate_key.exact_payload != *payload {
                violations.push(diagnostic(
                    OracleViolationCodeV3::PromotionPayloadMismatch,
                    &path,
                    "promotion payload does not match candidate key payload",
                ));
                return;
            }
            if candidate_key.source_locator != *source_locator {
                violations.push(diagnostic(
                    OracleViolationCodeV3::PromotionLocatorMismatch,
                    &path,
                    "promotion source locator does not equal candidate locator",
                ));
                return;
            }
            if !validate_locator_integrity(
                state,
                validated_review_cases,
                validated_review_decisions,
                source_locator,
                payload,
                violations,
            ) {
                return;
            }
            if project_scope_stable_id != &state.project_scope.stable_id {
                violations.push(diagnostic(
                    OracleViolationCodeV3::CrossProjectRecordReference,
                    &path,
                    "promotion project scope does not match session project scope",
                ));
                return;
            }
            let rejection = rejection_identity_digest(
                &candidate_key.project_scope_stable_id,
                &candidate_key.source_locator.source_review_case_id,
                candidate_key.source_locator.review_ledger_position,
                &candidate_key.source_locator.decision_digest,
            );
            if fold.rejected.contains(&rejection) {
                violations.push(diagnostic(
                    OracleViolationCodeV3::PromotionAfterRejectedCandidate,
                    &path,
                    "promotion accepted after candidate rejection",
                ));
                return;
            }
            if fold.records_by_id.contains_key(event_index) {
                violations.push(diagnostic(
                    OracleViolationCodeV3::DuplicateAcceptedRecordId,
                    &path,
                    "duplicate promotion accepted record id",
                ));
                return;
            }
            fold.records_by_id.insert(
                *event_index,
                EvidenceReusableRecord {
                    record_id: *event_index,
                    project_scope_stable_id: project_scope_stable_id.clone(),
                    observed_text: payload.observed_text.clone(),
                    confirmed_replacement: payload.confirmed_replacement.clone(),
                    source_locator: source_locator.clone(),
                    promotion_actor: actor.clone(),
                    superseded_by: None,
                },
            );
        }
        EvidenceReuseGovernanceEvent::ReusableInfluenceRevoked {
            event_index,
            record_id,
            actor,
        } => {
            let path = format!("reuse_governance_events[{event_index}]");
            if !validate_governance_actor_session_bound(
                actor,
                &state.session_authority,
                &path,
                violations,
            ) {
                return;
            }
            let Some(record) = fold.records_by_id.get(record_id) else {
                violations.push(diagnostic(
                    OracleViolationCodeV3::RevokeUnknownRecord,
                    &path,
                    "revocation references unknown record",
                ));
                return;
            };
            if record.project_scope_stable_id != state.project_scope.stable_id {
                violations.push(diagnostic(
                    OracleViolationCodeV3::CrossProjectRecordReference,
                    &path,
                    "revocation references record outside session project scope",
                ));
                return;
            }
            if fold.revoked.contains(record_id) || fold.superseded.contains_key(record_id) {
                violations.push(diagnostic(
                    OracleViolationCodeV3::RevokeInactiveRecord,
                    &path,
                    "revocation targets already inactive record",
                ));
                return;
            }
            fold.revoked.insert(*record_id);
        }
        EvidenceReuseGovernanceEvent::ReusableInfluenceSuperseded {
            event_index,
            predecessor_record_id,
            successor_record_id,
            actor,
        } => {
            let path = format!("reuse_governance_events[{event_index}]");
            if !validate_governance_actor_session_bound(
                actor,
                &state.session_authority,
                &path,
                violations,
            ) {
                return;
            }
            if predecessor_record_id == successor_record_id {
                violations.push(diagnostic(
                    OracleViolationCodeV3::InvalidSupersessionLineage,
                    &path,
                    "supersession predecessor equals successor",
                ));
                return;
            }
            let predecessor_active = fold.records_by_id.contains_key(predecessor_record_id)
                && !fold.revoked.contains(predecessor_record_id)
                && !fold.superseded.contains_key(predecessor_record_id);
            let successor_active = fold.records_by_id.contains_key(successor_record_id)
                && !fold.revoked.contains(successor_record_id)
                && !fold.superseded.contains_key(successor_record_id);
            if !predecessor_active {
                violations.push(diagnostic(
                    OracleViolationCodeV3::SupersedeUnknownPredecessor,
                    &path,
                    "supersession references inactive or unknown predecessor",
                ));
                return;
            }
            if !successor_active {
                violations.push(diagnostic(
                    OracleViolationCodeV3::SupersedeUnknownSuccessor,
                    &path,
                    "supersession references inactive or unknown successor",
                ));
                return;
            }
            if fold
                .records_by_id
                .get(predecessor_record_id)
                .map(|record| record.project_scope_stable_id.as_str())
                != fold
                    .records_by_id
                    .get(successor_record_id)
                    .map(|record| record.project_scope_stable_id.as_str())
            {
                violations.push(diagnostic(
                    OracleViolationCodeV3::CrossProjectRecordReference,
                    &path,
                    "supersession records must belong to the same project scope",
                ));
                return;
            }
            fold.superseded
                .insert(*predecessor_record_id, *successor_record_id);
            if let Some(record) = fold.records_by_id.get_mut(predecessor_record_id) {
                record.superseded_by = Some(*successor_record_id);
            }
        }
    }
}

fn validate_locator_integrity(
    state: &CurrentContractState,
    validated_review_cases: &BTreeMap<String, ValidatedReviewCase>,
    validated_review_decisions: &[Option<ValidatedReviewDecision>],
    locator: &EvidenceSourceDecisionLocator,
    payload: &EvidenceExactReusableCorrection,
    violations: &mut Vec<OracleDiagnosticV3>,
) -> bool {
    let before = violations.len();
    if parse_review_case_id(&locator.source_review_case_id).is_err() {
        violations.push(diagnostic(
            OracleViolationCodeV3::MalformedReviewCaseId,
            "source_locator.source_review_case_id",
            "malformed review case id in source locator",
        ));
        return false;
    }
    if parse_decision_digest_hex(&locator.decision_digest).is_err() {
        violations.push(diagnostic(
            OracleViolationCodeV3::MalformedDecisionDigest,
            "source_locator.decision_digest",
            "malformed decision digest in source locator",
        ));
        return false;
    }
    if !state
        .source_revisions
        .iter()
        .any(|revision| revision.revision_id == locator.source_revision_id)
    {
        violations.push(diagnostic(
            OracleViolationCodeV3::SourceLocatorBoundaryViolation,
            "source_locator.source_revision_id",
            "referenced source revision does not exist",
        ));
    }
    if !state
        .analysis_snapshots
        .iter()
        .any(|snapshot| snapshot.identity == locator.source_analysis_snapshot_identity)
    {
        violations.push(diagnostic(
            OracleViolationCodeV3::SourceLocatorBoundaryViolation,
            "source_locator.source_analysis_snapshot_identity",
            "referenced analysis snapshot does not exist",
        ));
    }
    if let Some(snapshot) = state
        .analysis_snapshots
        .iter()
        .find(|snapshot| snapshot.identity == locator.source_analysis_snapshot_identity)
    {
        if snapshot.source_revision_id != locator.source_revision_id {
            violations.push(diagnostic(
                OracleViolationCodeV3::SourceLocatorBoundaryViolation,
                "source_locator.source_analysis_snapshot_identity",
                "analysis snapshot source revision differs from source locator revision",
            ));
        }
        if snapshot.session_terms_identity != state.session_terms_identity
            || !is_canonical_analysis_snapshot(snapshot)
        {
            violations.push(diagnostic(
                OracleViolationCodeV3::SourceLocatorBoundaryViolation,
                "source_locator.source_analysis_snapshot_identity",
                "source locator must bind the canonical session-term analysis snapshot",
            ));
        }
    }
    let Some(review_case) = validated_review_cases.get(&locator.source_review_case_id) else {
        violations.push(diagnostic(
            OracleViolationCodeV3::SourceAnchorResolvedBytesMismatch,
            "source_locator.source_review_case_id",
            "referenced review case is missing or does not resolve to its source anchor bytes",
        ));
        return false;
    };
    if review_case.observed_revision_id != locator.source_revision_id
        || review_case.anchor_revision_id != locator.source_revision_id
    {
        violations.push(diagnostic(
            OracleViolationCodeV3::SourceLocatorBoundaryViolation,
            "source_locator.source_review_case_id",
            "review case revision binding mismatch",
        ));
    }
    if payload.observed_text != review_case.observed_source_bytes {
        violations.push(diagnostic(
            OracleViolationCodeV3::SourceLocatorBoundaryViolation,
            "source_locator.source_review_case_id",
            "reusable observed text does not match exact review-case source bytes",
        ));
    }
    if locator.review_ledger_position >= state.review_ledger_events.len() {
        violations.push(diagnostic(
            OracleViolationCodeV3::SourceLocatorBoundaryViolation,
            "source_locator.review_ledger_position",
            "review ledger position out of bounds",
        ));
        return false;
    }
    if locator.review_ledger_position >= locator.effective_at_ledger_length
        || locator.effective_at_ledger_length == 0
        || locator.effective_at_ledger_length > state.review_ledger_events.len()
    {
        violations.push(diagnostic(
            OracleViolationCodeV3::SourceLocatorBoundaryViolation,
            "source_locator.effective_at_ledger_length",
            "invalid effective ledger prefix",
        ));
    }
    let Some(Some(validated)) = validated_review_decisions.get(locator.review_ledger_position)
    else {
        violations.push(diagnostic(
            OracleViolationCodeV3::SourceLocatorBoundaryViolation,
            "source_locator.review_ledger_position",
            "locator references an invalid review ledger event",
        ));
        return false;
    };
    if validated.case_id != locator.source_review_case_id {
        violations.push(diagnostic(
            OracleViolationCodeV3::SourceLocatorBoundaryViolation,
            "source_locator.review_ledger_position",
            "ledger event case mismatch",
        ));
    }
    let Some(replacement) = manual_replacement_bytes(&validated.decision) else {
        violations.push(diagnostic(
            OracleViolationCodeV3::ManualReplacementByteMismatch,
            "source_locator.review_ledger_position",
            "locator must reference a validated manual replacement decision",
        ));
        return false;
    };
    if replacement != payload.confirmed_replacement {
        violations.push(diagnostic(
            OracleViolationCodeV3::ManualReplacementByteMismatch,
            "source_locator.decision_digest",
            "manual replacement bytes do not match ledger event",
        ));
    }
    match production_decision_digest_hex(
        &locator.source_review_case_id,
        &locator.source_revision_id,
        replacement,
        &review_case.observed_source_bytes,
    ) {
        Ok(expected_digest) => {
            if locator.decision_digest != expected_digest {
                violations.push(diagnostic(
                    OracleViolationCodeV3::DecisionDigestMismatch,
                    "source_locator.decision_digest",
                    "decision digest does not match canonical exact decision bytes",
                ));
            }
        }
        Err(diagnostic) => violations.push(diagnostic),
    }
    if !prefix_effective_at(
        validated_review_decisions,
        &locator.source_review_case_id,
        locator.effective_at_ledger_length,
        replacement,
    ) {
        violations.push(diagnostic(
            OracleViolationCodeV3::SourceLocatorBoundaryViolation,
            "source_locator.effective_at_ledger_length",
            "decision is not effective at declared ledger prefix",
        ));
    }
    violations.len() == before
}

fn validate_candidate_key(
    state: &CurrentContractState,
    validated_review_cases: &BTreeMap<String, ValidatedReviewCase>,
    validated_review_decisions: &[Option<ValidatedReviewDecision>],
    candidate_key: &EvidenceReuseCandidateKey,
    violations: &mut Vec<OracleDiagnosticV3>,
) -> bool {
    if candidate_key.project_scope_stable_id != state.project_scope.stable_id {
        violations.push(diagnostic(
            OracleViolationCodeV3::CrossProjectRecordReference,
            "candidate_key.project_scope_stable_id",
            "candidate key project scope mismatch",
        ));
        return false;
    }
    validate_locator_integrity(
        state,
        validated_review_cases,
        validated_review_decisions,
        &candidate_key.source_locator,
        &candidate_key.exact_payload,
        violations,
    )
}

fn validate_review_cases_and_ledger(
    state: &CurrentContractState,
    violations: &mut Vec<OracleDiagnosticV3>,
) {
    for snapshot in &state.analysis_snapshots {
        if let Err(diagnostic) = validate_analysis_snapshot(snapshot) {
            violations.push(diagnostic);
            continue;
        }
        if !state
            .source_revisions
            .iter()
            .any(|revision| revision.revision_id == snapshot.source_revision_id)
            || snapshot.session_terms_identity != state.session_terms_identity
        {
            violations.push(diagnostic(
                OracleViolationCodeV3::MalformedAnalysisSnapshot,
                "analysis_snapshots",
                "analysis snapshot does not bind a canonical source revision and session terms identity",
            ));
        }
    }
    let mut seen_case_ids = HashSet::new();
    for review_case in &state.review_cases {
        if parse_review_case_id(&review_case.case_id).is_err() {
            violations.push(diagnostic(
                OracleViolationCodeV3::MalformedReviewCaseId,
                "review_cases",
                "malformed review case id",
            ));
        }
        if review_case.origin != "detector_raised" {
            violations.push(diagnostic(
                OracleViolationCodeV3::HumanRaisedCasePresent,
                "review_cases",
                "only detector_raised review cases are valid in current-contract fixture v3",
            ));
        }
        if !seen_case_ids.insert(review_case.case_id.clone()) {
            violations.push(diagnostic(
                OracleViolationCodeV3::DuplicateReviewCaseId,
                "review_cases",
                "duplicate review case id",
            ));
        }
    }

    let mut seen_indices = HashSet::new();
    for (offset, event) in state.review_ledger_events.iter().enumerate() {
        if event.event_index != offset {
            violations.push(diagnostic(
                OracleViolationCodeV3::ReviewLedgerIndexGap,
                "review_ledger_events",
                "review ledger event index is not contiguous",
            ));
        }
        if !seen_indices.insert(event.event_index) {
            violations.push(diagnostic(
                OracleViolationCodeV3::ReviewLedgerDuplicateIndex,
                "review_ledger_events",
                "duplicate review ledger event index",
            ));
        }
    }
}

fn validate_source_anchors(
    state: &CurrentContractState,
    violations: &mut Vec<OracleDiagnosticV3>,
) -> BTreeMap<String, ValidatedReviewCase> {
    let mut validated = BTreeMap::new();
    let mut seen_revisions = HashSet::new();
    let mut anchor_eligible_revisions = HashSet::new();
    for revision in &state.source_revisions {
        if !seen_revisions.insert(revision.revision_id.clone()) {
            violations.push(diagnostic(
                OracleViolationCodeV3::ChangedSourceRevisionIdentity,
                "source_revisions",
                "duplicate source revision id",
            ));
        }
        match crate::srt::parse_srt(&revision.transcript_bytes) {
            Ok(transcript)
                if transcript.revision_id().to_tagged_string() == revision.revision_id =>
            {
                anchor_eligible_revisions.insert(revision.revision_id.clone());
            }
            Ok(_) => violations.push(diagnostic(
                OracleViolationCodeV3::ChangedSourceRevisionIdentity,
                "source_revisions",
                "declared source revision id does not match transcript bytes",
            )),
            Err(_) => violations.push(diagnostic(
                OracleViolationCodeV3::MalformedTranscriptRepresentation,
                "source_revisions",
                "malformed transcript representation for source revision validation",
            )),
        }
        if let Some(predecessor) = &revision.predecessor_revision_id
            && (predecessor == &revision.revision_id
                || !state
                    .source_revisions
                    .iter()
                    .any(|candidate| &candidate.revision_id == predecessor))
        {
            violations.push(diagnostic(
                OracleViolationCodeV3::ChangedSourceRevisionIdentity,
                "source_revisions.predecessor_revision_id",
                "source revision predecessor is missing or self-referential",
            ));
        }
    }
    for review_case in &state.review_cases {
        if review_case.observed_revision_id != review_case.anchor_revision_id {
            violations.push(diagnostic(
                OracleViolationCodeV3::SourceAnchorRevisionMismatch,
                "review_cases",
                "observed revision differs from anchor revision",
            ));
            continue;
        }
        let Some(revision) = state
            .source_revisions
            .iter()
            .find(|revision| revision.revision_id == review_case.anchor_revision_id)
        else {
            violations.push(diagnostic(
                OracleViolationCodeV3::SourceAnchorRevisionMismatch,
                "review_cases",
                "anchor revision missing",
            ));
            continue;
        };
        if !anchor_eligible_revisions.contains(&review_case.anchor_revision_id) {
            violations.push(diagnostic(
                OracleViolationCodeV3::SourceAnchorRevisionMismatch,
                "review_cases",
                "anchor revision transcript bytes do not validate to the declared revision",
            ));
            continue;
        }
        let transcript = match crate::srt::parse_srt(&revision.transcript_bytes) {
            Ok(value) => value,
            Err(_) => {
                violations.push(diagnostic(
                    OracleViolationCodeV3::MalformedTranscriptRepresentation,
                    "source_revisions",
                    "malformed transcript representation for anchor validation",
                ));
                continue;
            }
        };
        let Some(segment) = transcript
            .segments()
            .get(review_case.anchor_segment_position)
        else {
            violations.push(diagnostic(
                OracleViolationCodeV3::SourceAnchorSegmentMismatch,
                "review_cases",
                "anchor segment position out of bounds",
            ));
            continue;
        };
        let segment_text = segment.text();
        if review_case.anchor_start_byte >= review_case.anchor_end_byte
            || !segment_text.is_char_boundary(review_case.anchor_start_byte)
            || !segment_text.is_char_boundary(review_case.anchor_end_byte)
            || review_case.anchor_end_byte > segment_text.len()
        {
            violations.push(diagnostic(
                OracleViolationCodeV3::Utf8AnchorBoundaryViolation,
                "review_cases",
                "anchor byte range is not a valid UTF-8 boundary",
            ));
            continue;
        }
        let resolved =
            match segment_text.get(review_case.anchor_start_byte..review_case.anchor_end_byte) {
                Some(value) => value,
                None => {
                    violations.push(diagnostic(
                        OracleViolationCodeV3::SourceAnchorOutOfBounds,
                        "review_cases",
                        "anchor byte range out of bounds",
                    ));
                    continue;
                }
            };
        if resolved != review_case.observed_source_bytes {
            violations.push(diagnostic(
                OracleViolationCodeV3::SourceAnchorResolvedBytesMismatch,
                "review_cases",
                "resolved anchor bytes do not match observed source bytes",
            ));
            continue;
        }
        validated.insert(
            review_case.case_id.clone(),
            ValidatedReviewCase {
                observed_revision_id: review_case.observed_revision_id.clone(),
                anchor_revision_id: review_case.anchor_revision_id.clone(),
                observed_source_bytes: resolved.to_owned(),
                alternative_count: review_case.alternative_count,
            },
        );
    }
    validated
}

fn validate_governance_events(
    state: &CurrentContractState,
    violations: &mut Vec<OracleDiagnosticV3>,
) {
    let mut seen_indices = HashSet::new();
    for (offset, event) in state.reuse_governance_events.iter().enumerate() {
        let event_index = governance_event_index(event);
        if event_index != offset {
            violations.push(diagnostic(
                OracleViolationCodeV3::ReuseGovernanceIndexGap,
                "reuse_governance_events",
                "reuse governance event index is not contiguous",
            ));
        }
        if !seen_indices.insert(event_index) {
            violations.push(diagnostic(
                OracleViolationCodeV3::ReuseGovernanceDuplicateIndex,
                "reuse_governance_events",
                "duplicate reuse governance event index",
            ));
        }
    }
}

fn governance_event_index(event: &EvidenceReuseGovernanceEvent) -> usize {
    match event {
        EvidenceReuseGovernanceEvent::PromotionCandidateRejected { event_index, .. }
        | EvidenceReuseGovernanceEvent::PromotionAccepted { event_index, .. }
        | EvidenceReuseGovernanceEvent::ReusableInfluenceRevoked { event_index, .. }
        | EvidenceReuseGovernanceEvent::ReusableInfluenceSuperseded { event_index, .. } => {
            *event_index
        }
    }
}

fn validate_historical_binding(
    state: &CurrentContractState,
    validated_review_cases: &BTreeMap<String, ValidatedReviewCase>,
    validated_review_decisions: &[Option<ValidatedReviewDecision>],
    violations: &mut Vec<OracleDiagnosticV3>,
) {
    let Some(binding) = state.reuse_enabled_analysis_binding.as_ref() else {
        return;
    };
    if binding.governance_event_boundary > state.reuse_governance_events.len() {
        violations.push(diagnostic(
            OracleViolationCodeV3::HistoricalBindingAnalysisMismatch,
            "reuse_enabled_analysis_binding.governance_event_boundary",
            "historical binding governance boundary is outside governance history",
        ));
        return;
    }
    if let Err(diagnostic) = validate_analysis_snapshot(&binding.analysis_snapshot) {
        violations.push(diagnostic);
        return;
    }
    if !is_reuse_enabled_analysis_snapshot(&binding.analysis_snapshot) {
        violations.push(diagnostic(
            OracleViolationCodeV3::HistoricalBindingAnalysisMismatch,
            "reuse_enabled_analysis_binding.analysis_snapshot",
            "historical binding must use the reuse-enabled analysis configuration",
        ));
    }
    if binding.analysis_snapshot.source_revision_id.is_empty()
        || !state
            .source_revisions
            .iter()
            .any(|revision| revision.revision_id == binding.analysis_snapshot.source_revision_id)
        || binding.analysis_snapshot.session_terms_identity != state.session_terms_identity
        || !state
            .analysis_snapshots
            .iter()
            .any(|snapshot| snapshot == &binding.analysis_snapshot)
    {
        violations.push(diagnostic(
            OracleViolationCodeV3::HistoricalBindingAnalysisMismatch,
            "reuse_enabled_analysis_binding.analysis_snapshot",
            "historical binding analysis snapshot is not an exact canonical session snapshot",
        ));
    }
    if binding.projection_version != REUSABLE_INFLUENCE_PROJECTION_VERSION {
        violations.push(diagnostic(
            OracleViolationCodeV3::HistoricalBindingAnalysisMismatch,
            "reuse_enabled_analysis_binding.projection_version",
            "historical binding projection version mismatch",
        ));
    }
    if binding.analysis_snapshot_identity != binding.analysis_snapshot.identity {
        violations.push(diagnostic(
            OracleViolationCodeV3::HistoricalBindingAnalysisMismatch,
            "reuse_enabled_analysis_binding.analysis_snapshot_identity",
            "historical binding analysis snapshot identity mismatch",
        ));
    }
    let (active_at_boundary, _, _) = fold_reuse_governance(
        state,
        validated_review_cases,
        validated_review_decisions,
        binding.governance_event_boundary,
        violations,
    );
    match compute_production_snapshot_identity(
        &state.project_scope.stable_id,
        binding.governance_event_boundary,
        &active_at_boundary,
        &state.analysis_snapshots,
    ) {
        Ok(expected) => {
            if binding.reusable_snapshot_identity != expected {
                violations.push(diagnostic(
                    OracleViolationCodeV3::HistoricalBindingSnapshotMismatch,
                    "reuse_enabled_analysis_binding.reusable_snapshot_identity",
                    "historical binding snapshot does not match production identity at boundary",
                ));
            }
        }
        Err(diagnostic) => violations.push(diagnostic),
    }
}

fn prefix_effective_at(
    validated_review_decisions: &[Option<ValidatedReviewDecision>],
    case_id: &str,
    prefix_length: usize,
    replacement: &str,
) -> bool {
    let mut effective: Option<&ValidatedReviewDecision> = None;
    for validated in validated_review_decisions
        .iter()
        .take(prefix_length)
        .flatten()
    {
        if validated.case_id == case_id {
            effective = Some(validated);
        }
    }
    effective
        .and_then(|decision| manual_replacement_bytes(&decision.decision))
        .is_some_and(|bytes| bytes == replacement)
}

fn manual_replacement_bytes(decision: &CorrectionDecision) -> Option<&str> {
    match decision {
        CorrectionDecision::ManualReplacement { replacement } => Some(replacement.as_str()),
        _ => None,
    }
}

fn recompute_snapshot_identity(
    state: &CurrentContractState,
    active_records: &[EvidenceReusableRecord],
    governance_event_boundary: usize,
    violations: &mut Vec<OracleDiagnosticV3>,
) -> String {
    if state.project_scope.stable_id.is_empty() {
        return String::new();
    }
    match compute_production_snapshot_identity(
        &state.project_scope.stable_id,
        governance_event_boundary,
        active_records,
        &state.analysis_snapshots,
    ) {
        Ok(identity) => identity,
        Err(diagnostic) => {
            violations.push(diagnostic);
            String::new()
        }
    }
}

pub fn compare_supplied_derived(
    derived: &DerivedContractProjection,
    state: &CurrentContractState,
    violations: &mut Vec<OracleDiagnosticV3>,
) {
    if !state.effective_review_status.is_empty()
        && state.effective_review_status != derived.effective_review_status
    {
        violations.push(diagnostic(
            OracleViolationCodeV3::DerivedRebuildMismatch,
            "effective_review_status",
            "candidate-supplied effective review status does not match independent fold",
        ));
    }
    if !state.effective_reusable_records.is_empty()
        && state.effective_reusable_records != derived.effective_reusable_records
    {
        violations.push(diagnostic(
            OracleViolationCodeV3::DerivedRebuildMismatch,
            "effective_reusable_records",
            "candidate-supplied effective reusable records do not match independent fold",
        ));
    }
    if !state.historical_reusable_records.is_empty()
        && state.historical_reusable_records != derived.historical_reusable_records
    {
        violations.push(diagnostic(
            OracleViolationCodeV3::DerivedRebuildMismatch,
            "historical_reusable_records",
            "candidate-supplied historical reusable records do not match independent fold",
        ));
    }
    if !state.reusable_snapshot_identity.is_empty()
        && state.reusable_snapshot_identity != derived.reusable_snapshot_identity
    {
        violations.push(diagnostic(
            OracleViolationCodeV3::ChangedReusableSnapshotIdentity,
            "reusable_snapshot_identity",
            "candidate-supplied snapshot identity does not match independent recomputation",
        ));
    }
}

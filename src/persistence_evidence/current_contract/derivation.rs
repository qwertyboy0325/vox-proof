use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::review::{CorrectionDecision, ManualReplacementText, ReviewCaseStatus};

use super::model::{
    CurrentContractState, DerivedContractProjection, EvidenceEffectiveReviewStatus,
    EvidenceExactReusableCorrection, EvidenceReusableRecord, EvidenceReuseCandidateKey,
    EvidenceReuseGovernanceEvent, EvidenceReviewLedgerEvent, EvidenceSourceDecisionLocator,
    REUSABLE_INFLUENCE_PROJECTION_VERSION,
};
use super::production_bridge::{
    compute_production_snapshot_identity, parse_decision_digest_hex, parse_review_case_id,
    parse_revision_id, production_decision_digest_hex, validate_governance_actor_session_bound,
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
    validate_source_anchors(state, &mut violations);
    validate_governance_events(state, &mut violations);
    validate_historical_binding(state, &mut violations);

    let effective_review_status = fold_effective_review_status(state, &mut violations);
    let (effective_reusable_records, historical_reusable_records, rejected_candidate_identities) =
        fold_reuse_governance(state, state.reuse_governance_events.len(), &mut violations);
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

fn fold_effective_review_status(
    state: &CurrentContractState,
    violations: &mut Vec<OracleDiagnosticV3>,
) -> Vec<EvidenceEffectiveReviewStatus> {
    let mut latest: BTreeMap<String, EvidenceEffectiveReviewStatus> = BTreeMap::new();
    for event in &state.review_ledger_events {
        let Some(decision) = validate_and_decode_review_event(state, event, violations) else {
            continue;
        };
        let observed_revision = match parse_revision_id(&event.observed_revision_id) {
            Ok(value) => value,
            Err(diagnostic) => {
                violations.push(diagnostic);
                continue;
            }
        };
        let status = ReviewCaseStatus::Decided {
            observed_revision,
            decision,
        };
        latest.insert(
            event.case_id.clone(),
            EvidenceEffectiveReviewStatus {
                case_id: event.case_id.clone(),
                status: format!("{:?}", status),
            },
        );
    }
    latest.into_values().collect()
}

fn validate_and_decode_review_event(
    state: &CurrentContractState,
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
                review_case.observed_source_bytes.as_str(),
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
    event_limit: usize,
    violations: &mut Vec<OracleDiagnosticV3>,
) -> (
    Vec<EvidenceReusableRecord>,
    Vec<EvidenceReusableRecord>,
    Vec<String>,
) {
    let mut fold = GovernanceFoldState::new();
    for event in state.reuse_governance_events.iter().take(event_limit) {
        apply_governance_event(state, event, &mut fold, violations);
    }
    let rejected: Vec<String> = fold.rejected.iter().cloned().collect();
    let (effective, historical) = fold.finalize();
    (effective, historical, rejected)
}

fn apply_governance_event(
    state: &CurrentContractState,
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
            if !validate_candidate_key(state, candidate_key, violations) {
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
            if !validate_candidate_key(state, candidate_key, violations) {
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
            validate_locator_integrity(state, source_locator, payload, violations);
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
            if *event_index != fold.records_by_id.len() {
                violations.push(diagnostic(
                    OracleViolationCodeV3::InvalidGovernanceTransition,
                    &path,
                    "record id must equal promotion event index",
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

pub fn validate_locator_integrity(
    state: &CurrentContractState,
    locator: &EvidenceSourceDecisionLocator,
    payload: &EvidenceExactReusableCorrection,
    violations: &mut Vec<OracleDiagnosticV3>,
) {
    if parse_review_case_id(&locator.source_review_case_id).is_err() {
        violations.push(diagnostic(
            OracleViolationCodeV3::MalformedReviewCaseId,
            "source_locator.source_review_case_id",
            "malformed review case id in source locator",
        ));
        return;
    }
    if parse_decision_digest_hex(&locator.decision_digest).is_err() {
        violations.push(diagnostic(
            OracleViolationCodeV3::MalformedDecisionDigest,
            "source_locator.decision_digest",
            "malformed decision digest in source locator",
        ));
        return;
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
    let Some(review_case) = state
        .review_cases
        .iter()
        .find(|case| case.case_id == locator.source_review_case_id)
    else {
        violations.push(diagnostic(
            OracleViolationCodeV3::MissingReviewCaseReference,
            "source_locator.source_review_case_id",
            "referenced review case does not exist",
        ));
        return;
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
    if locator.review_ledger_position >= state.review_ledger_events.len() {
        violations.push(diagnostic(
            OracleViolationCodeV3::SourceLocatorBoundaryViolation,
            "source_locator.review_ledger_position",
            "review ledger position out of bounds",
        ));
        return;
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
    let event = &state.review_ledger_events[locator.review_ledger_position];
    if event.case_id != locator.source_review_case_id {
        violations.push(diagnostic(
            OracleViolationCodeV3::SourceLocatorBoundaryViolation,
            "source_locator.review_ledger_position",
            "ledger event case mismatch",
        ));
    }
    let replacement = event.manual_replacement_bytes.as_deref().unwrap_or("");
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
        state,
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
}

fn validate_candidate_key(
    state: &CurrentContractState,
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
        &candidate_key.source_locator,
        &candidate_key.exact_payload,
        violations,
    );
    true
}

fn validate_review_cases_and_ledger(
    state: &CurrentContractState,
    violations: &mut Vec<OracleDiagnosticV3>,
) {
    let mut seen_case_ids = HashSet::new();
    for review_case in &state.review_cases {
        if parse_review_case_id(&review_case.case_id).is_err() {
            violations.push(diagnostic(
                OracleViolationCodeV3::MalformedReviewCaseId,
                "review_cases",
                "malformed review case id",
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
    if state
        .review_cases
        .iter()
        .any(|case| case.origin == "human_raised")
    {
        violations.push(diagnostic(
            OracleViolationCodeV3::HumanRaisedCasePresent,
            "review_cases",
            "HumanRaised cases are excluded from current-contract fixture v3",
        ));
    }
}

fn validate_source_anchors(state: &CurrentContractState, violations: &mut Vec<OracleDiagnosticV3>) {
    for review_case in &state.review_cases {
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
        }
    }
}

fn validate_governance_events(
    state: &CurrentContractState,
    violations: &mut Vec<OracleDiagnosticV3>,
) {
    let mut seen_indices = HashSet::new();
    for (offset, event) in state.reuse_governance_events.iter().enumerate() {
        let event_index = match event {
            EvidenceReuseGovernanceEvent::PromotionCandidateRejected { event_index, .. }
            | EvidenceReuseGovernanceEvent::PromotionAccepted { event_index, .. }
            | EvidenceReuseGovernanceEvent::ReusableInfluenceRevoked { event_index, .. }
            | EvidenceReuseGovernanceEvent::ReusableInfluenceSuperseded { event_index, .. } => {
                *event_index
            }
        };
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

fn validate_historical_binding(
    state: &CurrentContractState,
    violations: &mut Vec<OracleDiagnosticV3>,
) {
    let Some(binding) = state.reuse_enabled_analysis_binding.as_ref() else {
        return;
    };
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
    let (active_at_boundary, _, _) =
        fold_reuse_governance(state, binding.governance_event_boundary, violations);
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
    state: &CurrentContractState,
    case_id: &str,
    prefix_length: usize,
    replacement: &str,
) -> bool {
    let mut effective: Option<&EvidenceReviewLedgerEvent> = None;
    for event in state.review_ledger_events.iter().take(prefix_length) {
        if event.case_id == case_id {
            effective = Some(event);
        }
    }
    effective
        .and_then(|event| event.manual_replacement_bytes.as_deref())
        .is_some_and(|bytes| bytes == replacement)
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

use std::collections::{BTreeMap, BTreeSet, HashSet};

use sha2::{Digest, Sha256};

use crate::anchor::TranscriptRevisionId;
use crate::review::{CorrectionDecision, ManualReplacementText, ReviewCaseStatus};

use super::model::{
    CurrentContractState, DerivedContractProjection, EvidenceEffectiveReviewStatus,
    EvidenceExactReusableCorrection, EvidenceGovernanceActor, EvidenceReusableRecord,
    EvidenceReuseCandidateKey, EvidenceReuseEnabledAnalysisBinding, EvidenceReuseGovernanceEvent,
    EvidenceReviewLedgerEvent, EvidenceSourceDecisionLocator,
    REUSABLE_INFLUENCE_PROJECTION_VERSION,
};
use super::serialization::rejection_identity_digest;
use super::violations::{OracleDiagnosticV3, OracleViolationCodeV3, diagnostic};

pub fn finalize_derived_fields(state: &mut CurrentContractState) {
    let (derived, _) = derive_contract_projection(state);
    state.effective_review_status = derived.effective_review_status;
    state.effective_reusable_records = derived.effective_reusable_records;
    state.historical_reusable_records = derived.historical_reusable_records;
    state.reusable_snapshot_identity = derived.reusable_snapshot_identity;
    state.reuse_enabled_analysis_binding = derived.reuse_enabled_analysis_binding;
    state.derived_queue_projection = derived.derived_queue_projection;
}

pub fn derive_contract_projection(
    state: &CurrentContractState,
) -> (DerivedContractProjection, Vec<OracleDiagnosticV3>) {
    let mut violations = Vec::new();
    validate_review_cases_and_ledger(state, &mut violations);
    validate_source_anchors(state, &mut violations);
    validate_governance_events(state, &mut violations);

    let effective_review_status = fold_effective_review_status(state, &mut violations);
    let (effective_reusable_records, historical_reusable_records, rejected_candidate_identities) =
        fold_reuse_governance(state, &mut violations);
    let reusable_snapshot_identity =
        recompute_snapshot_identity(state, &effective_reusable_records, &mut violations);
    let reuse_enabled_analysis_binding =
        recompute_reuse_enabled_binding(state, &reusable_snapshot_identity, &mut violations);
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
            reuse_enabled_analysis_binding,
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
        let Some(observed_revision) = revision_id_from_tagged(&event.observed_revision_id) else {
            violations.push(diagnostic(
                OracleViolationCodeV3::ChangedSourceRevisionIdentity,
                "review_ledger_events",
                "invalid observed revision id",
            ));
            continue;
        };
        let Some(review_case) = state
            .review_cases
            .iter()
            .find(|review_case| review_case.case_id == event.case_id)
        else {
            violations.push(diagnostic(
                OracleViolationCodeV3::MissingReviewCaseReference,
                "review_ledger_events",
                "missing review case for ledger event",
            ));
            continue;
        };
        let decision = match event.action_kind.as_str() {
            "manual_replacement" => {
                let replacement_text = event.manual_replacement_bytes.as_deref().unwrap_or("");
                let replacement = match ManualReplacementText::new(
                    replacement_text,
                    review_case.observed_source_bytes.as_str(),
                ) {
                    Ok(value) => value,
                    Err(_) => continue,
                };
                CorrectionDecision::ManualReplacement { replacement }
            }
            "accept_alternative" => CorrectionDecision::AcceptAlternative {
                alternative_index: event.alternative_index.unwrap_or(0),
            },
            "reject" => CorrectionDecision::Reject,
            "defer" => CorrectionDecision::Defer,
            "needs_manual_correction" => CorrectionDecision::NeedsManualCorrection,
            other => {
                violations.push(diagnostic(
                    OracleViolationCodeV3::ChangedReviewLedgerPayload,
                    "review_ledger_events",
                    &format!("unsupported action kind {other}"),
                ));
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

fn revision_id_from_tagged(tagged: &str) -> Option<TranscriptRevisionId> {
    let hex = tagged.strip_prefix("rev:sha256-v1:")?;
    if hex.len() != 64 {
        return None;
    }
    let mut digest = [0_u8; 32];
    for (index, chunk) in hex.as_bytes().chunks(2).enumerate() {
        if index >= 32 {
            break;
        }
        let hi = (chunk[0] as char).to_digit(16)? as u8;
        let lo = (chunk[1] as char).to_digit(16)? as u8;
        digest[index] = (hi << 4) | lo;
    }
    Some(TranscriptRevisionId::from_sha256_digest(digest))
}

fn fold_reuse_governance(
    state: &CurrentContractState,
    violations: &mut Vec<OracleDiagnosticV3>,
) -> (
    Vec<EvidenceReusableRecord>,
    Vec<EvidenceReusableRecord>,
    Vec<String>,
) {
    let mut rejected = BTreeSet::new();
    let mut records_by_id: BTreeMap<usize, EvidenceReusableRecord> = BTreeMap::new();
    let mut revoked: HashSet<usize> = HashSet::new();
    let mut superseded: BTreeMap<usize, usize> = BTreeMap::new();

    for event in &state.reuse_governance_events {
        match event {
            EvidenceReuseGovernanceEvent::PromotionCandidateRejected {
                event_index,
                candidate_key,
                actor,
            } => {
                validate_candidate_key(state, candidate_key, violations);
                validate_governance_actor(actor, violations);
                let identity = rejection_identity_digest(
                    &candidate_key.project_scope_stable_id,
                    &candidate_key.source_locator.source_review_case_id,
                    candidate_key.source_locator.review_ledger_position,
                    &candidate_key.source_locator.decision_digest,
                );
                rejected.insert(identity);
                let _ = event_index;
            }
            EvidenceReuseGovernanceEvent::PromotionAccepted {
                event_index,
                candidate_key,
                payload,
                source_locator,
                actor,
                project_scope_stable_id,
            } => {
                validate_candidate_key(state, candidate_key, violations);
                validate_locator_integrity(state, source_locator, payload, violations);
                validate_promotion_locator_matches_candidate(
                    candidate_key,
                    source_locator,
                    violations,
                );
                if project_scope_stable_id != &state.project_scope.stable_id {
                    violations.push(diagnostic(
                        OracleViolationCodeV3::CrossProjectRecordReference,
                        "reuse_governance_events",
                        "promotion project scope does not match session project scope",
                    ));
                }
                if records_by_id.contains_key(event_index) {
                    violations.push(diagnostic(
                        OracleViolationCodeV3::DuplicateAcceptedRecordId,
                        "reuse_governance_events",
                        "duplicate promotion accepted record id",
                    ));
                }
                if rejected.contains(&rejection_identity_digest(
                    &candidate_key.project_scope_stable_id,
                    &candidate_key.source_locator.source_review_case_id,
                    candidate_key.source_locator.review_ledger_position,
                    &candidate_key.source_locator.decision_digest,
                )) {
                    violations.push(diagnostic(
                        OracleViolationCodeV3::PromotionAfterRejectedCandidate,
                        "reuse_governance_events",
                        "promotion accepted after candidate rejection",
                    ));
                }
                records_by_id.insert(
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
                event_index: _,
                record_id,
                actor,
            } => {
                validate_governance_actor(actor, violations);
                if !records_by_id.contains_key(record_id) {
                    violations.push(diagnostic(
                        OracleViolationCodeV3::RevokeUnknownRecord,
                        "reuse_governance_events",
                        "revocation references unknown record",
                    ));
                } else if revoked.contains(record_id) || superseded.contains_key(record_id) {
                    violations.push(diagnostic(
                        OracleViolationCodeV3::RevokeInactiveRecord,
                        "reuse_governance_events",
                        "revocation targets already inactive record",
                    ));
                }
                revoked.insert(*record_id);
            }
            EvidenceReuseGovernanceEvent::ReusableInfluenceSuperseded {
                event_index: _,
                predecessor_record_id,
                successor_record_id,
                actor,
            } => {
                validate_governance_actor(actor, violations);
                if !records_by_id.contains_key(predecessor_record_id) {
                    violations.push(diagnostic(
                        OracleViolationCodeV3::SupersedeUnknownPredecessor,
                        "reuse_governance_events",
                        "supersession references unknown predecessor",
                    ));
                }
                if !records_by_id.contains_key(successor_record_id) {
                    violations.push(diagnostic(
                        OracleViolationCodeV3::SupersedeUnknownSuccessor,
                        "reuse_governance_events",
                        "supersession references unknown successor",
                    ));
                }
                if predecessor_record_id == successor_record_id {
                    violations.push(diagnostic(
                        OracleViolationCodeV3::InvalidSupersessionLineage,
                        "reuse_governance_events",
                        "supersession predecessor equals successor",
                    ));
                }
                superseded.insert(*predecessor_record_id, *successor_record_id);
            }
        }
    }

    for (predecessor, successor) in &superseded {
        if let Some(record) = records_by_id.get_mut(predecessor) {
            record.superseded_by = Some(*successor);
        }
    }

    let mut effective_reusable_records = Vec::new();
    let mut historical_reusable_records = Vec::new();
    for record in records_by_id.into_values() {
        if revoked.contains(&record.record_id) || superseded.contains_key(&record.record_id) {
            historical_reusable_records.push(record);
        } else {
            effective_reusable_records.push(record);
        }
    }
    effective_reusable_records.sort_by_key(|record| record.record_id);
    historical_reusable_records.sort_by_key(|record| record.record_id);

    (
        effective_reusable_records,
        historical_reusable_records,
        rejected.into_iter().collect(),
    )
}

pub fn validate_locator_integrity(
    state: &CurrentContractState,
    locator: &EvidenceSourceDecisionLocator,
    payload: &EvidenceExactReusableCorrection,
    violations: &mut Vec<OracleDiagnosticV3>,
) {
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
    let expected_digest = evidence_decision_digest(
        &locator.source_review_case_id,
        &locator.source_revision_id,
        replacement,
    );
    if locator.decision_digest != expected_digest {
        violations.push(diagnostic(
            OracleViolationCodeV3::DecisionDigestMismatch,
            "source_locator.decision_digest",
            "decision digest does not match canonical exact decision bytes",
        ));
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

fn validate_promotion_locator_matches_candidate(
    candidate_key: &EvidenceReuseCandidateKey,
    source_locator: &EvidenceSourceDecisionLocator,
    violations: &mut Vec<OracleDiagnosticV3>,
) {
    if candidate_key.source_locator != *source_locator {
        violations.push(diagnostic(
            OracleViolationCodeV3::PromotionLocatorMismatch,
            "reuse_governance_events",
            "promotion source locator does not equal candidate locator",
        ));
    }
}

fn validate_candidate_key(
    state: &CurrentContractState,
    candidate_key: &EvidenceReuseCandidateKey,
    violations: &mut Vec<OracleDiagnosticV3>,
) {
    if candidate_key.project_scope_stable_id != state.project_scope.stable_id {
        violations.push(diagnostic(
            OracleViolationCodeV3::CrossProjectRecordReference,
            "candidate_key.project_scope_stable_id",
            "candidate key project scope mismatch",
        ));
    }
    validate_locator_integrity(
        state,
        &candidate_key.source_locator,
        &candidate_key.exact_payload,
        violations,
    );
}

fn validate_governance_actor(
    actor: &EvidenceGovernanceActor,
    violations: &mut Vec<OracleDiagnosticV3>,
) {
    if actor.role_label.is_empty() || actor.display_label.is_empty() {
        violations.push(diagnostic(
            OracleViolationCodeV3::ChangedReuseGovernanceActor,
            "reuse_governance_events.actor",
            "governance actor context is incomplete",
        ));
    }
}

fn validate_review_cases_and_ledger(
    state: &CurrentContractState,
    violations: &mut Vec<OracleDiagnosticV3>,
) {
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
        if event.provenance == "automatic" {
            violations.push(diagnostic(
                OracleViolationCodeV3::FabricatedAutomaticDecision,
                "review_ledger_events",
                "automatic provenance is forbidden",
            ));
        }
        if !state
            .review_cases
            .iter()
            .any(|case| case.case_id == event.case_id)
        {
            violations.push(diagnostic(
                OracleViolationCodeV3::MissingReviewCaseReference,
                "review_ledger_events",
                "review ledger references missing review case",
            ));
        }
        if !state
            .source_revisions
            .iter()
            .any(|revision| revision.revision_id == event.observed_revision_id)
        {
            violations.push(diagnostic(
                OracleViolationCodeV3::ChangedSourceRevisionIdentity,
                "review_ledger_events",
                "review ledger references missing source revision",
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
        let segment = revision
            .transcript_bytes
            .split("\n\n")
            .nth(review_case.anchor_segment_position);
        let Some(segment_text) = segment.and_then(|block| block.lines().last()) else {
            violations.push(diagnostic(
                OracleViolationCodeV3::SourceAnchorSegmentMismatch,
                "review_cases",
                "anchor segment position out of bounds",
            ));
            continue;
        };
        if review_case.anchor_end_byte > segment_text.len()
            || review_case.anchor_start_byte > review_case.anchor_end_byte
        {
            violations.push(diagnostic(
                OracleViolationCodeV3::SourceAnchorOutOfBounds,
                "review_cases",
                "anchor byte range out of bounds",
            ));
            continue;
        }
        let resolved = &segment_text[review_case.anchor_start_byte..review_case.anchor_end_byte];
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
    violations: &mut Vec<OracleDiagnosticV3>,
) -> String {
    if state.project_scope.stable_id.is_empty() {
        return String::new();
    }
    const DOMAIN: &[u8] = b"voxproof-reusable-influence-snapshot-identity-v2";
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN);
    hash_string(&mut hasher, &state.project_scope.stable_id);
    hasher.update((state.reuse_governance_events.len() as u64).to_le_bytes());
    hash_string(&mut hasher, REUSABLE_INFLUENCE_PROJECTION_VERSION);
    hasher.update((active_records.len() as u64).to_le_bytes());
    for record in active_records {
        hasher.update((record.record_id as u64).to_le_bytes());
        hash_string(&mut hasher, &record.observed_text);
        hash_string(&mut hasher, &record.confirmed_replacement);
        hash_locator(&mut hasher, &record.source_locator);
        hash_string(&mut hasher, &record.promotion_actor.role_label);
        hash_string(&mut hasher, &record.promotion_actor.display_label);
        if record.project_scope_stable_id != state.project_scope.stable_id {
            violations.push(diagnostic(
                OracleViolationCodeV3::CrossProjectRecordReference,
                "effective_reusable_records",
                "active record references another project scope",
            ));
        }
    }
    format!(
        "reusable-influence-snapshot:sha256-v2:{}",
        digest_hex(hasher.finalize())
    )
}

fn recompute_reuse_enabled_binding(
    state: &CurrentContractState,
    snapshot_identity: &str,
    _violations: &mut Vec<OracleDiagnosticV3>,
) -> EvidenceReuseEnabledAnalysisBinding {
    let governance_event_boundary = state.reuse_governance_events.len();
    if snapshot_identity.is_empty() {
        return EvidenceReuseEnabledAnalysisBinding {
            analysis_snapshot_identity: String::new(),
            reusable_snapshot_identity: String::new(),
            governance_event_boundary,
            projection_version: REUSABLE_INFLUENCE_PROJECTION_VERSION.to_owned(),
        };
    }
    let analysis_snapshot_identity = state
        .analysis_snapshots
        .iter()
        .map(|snapshot| snapshot.identity.as_str())
        .collect::<Vec<_>>();
    let reuse_enabled_analysis = analysis_snapshot_identity
        .get(1)
        .copied()
        .or(analysis_snapshot_identity.first().copied())
        .unwrap_or_default()
        .to_owned();
    EvidenceReuseEnabledAnalysisBinding {
        analysis_snapshot_identity: reuse_enabled_analysis,
        reusable_snapshot_identity: snapshot_identity.to_owned(),
        governance_event_boundary,
        projection_version: REUSABLE_INFLUENCE_PROJECTION_VERSION.to_owned(),
    }
}

pub fn evidence_decision_digest(
    case_id: &str,
    observed_revision_id: &str,
    replacement: &str,
) -> String {
    const DOMAIN: &[u8] = b"voxproof-manual-replacement-decision-digest-v1";
    let local_index = case_id
        .strip_prefix("review-case:")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN);
    hasher.update(local_index.to_le_bytes());
    hasher.update(observed_revision_id.as_bytes());
    hasher.update((replacement.len() as u64).to_le_bytes());
    hasher.update(replacement.as_bytes());
    digest_hex(hasher.finalize())
}

fn hash_locator(hasher: &mut Sha256, locator: &EvidenceSourceDecisionLocator) {
    hash_string(hasher, &locator.source_revision_id);
    hash_string(hasher, &locator.source_analysis_snapshot_identity);
    if let Some(local_index) = locator
        .source_review_case_id
        .strip_prefix("review-case:")
        .and_then(|value| value.parse::<u64>().ok())
    {
        hasher.update(local_index.to_le_bytes());
    }
    hasher.update((locator.review_ledger_position as u64).to_le_bytes());
    if let Some(bytes) = decode_hex_32(&locator.decision_digest) {
        hasher.update(bytes);
    }
    hasher.update((locator.effective_at_ledger_length as u64).to_le_bytes());
}

fn decode_hex_32(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }
    let mut out = [0_u8; 32];
    for (index, chunk) in value.as_bytes().chunks(2).enumerate() {
        let hi = (chunk[0] as char).to_digit(16)? as u8;
        let lo = (chunk[1] as char).to_digit(16)? as u8;
        out[index] = (hi << 4) | lo;
    }
    Some(out)
}

fn hash_string(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

fn digest_hex(bytes: impl AsRef<[u8]>) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    bytes.as_ref().iter().fold(
        String::with_capacity(bytes.as_ref().len() * 2),
        |mut out, byte| {
            out.push(HEX[(byte >> 4) as usize] as char);
            out.push(HEX[(byte & 0x0f) as usize] as char);
            out
        },
    )
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
    if !state
        .reuse_enabled_analysis_binding
        .reusable_snapshot_identity
        .is_empty()
        && state.reuse_enabled_analysis_binding != derived.reuse_enabled_analysis_binding
    {
        violations.push(diagnostic(
            OracleViolationCodeV3::ReuseAnalysisSnapshotMismatch,
            "reuse_enabled_analysis_binding",
            "candidate-supplied reuse-enabled analysis binding does not match independent derivation",
        ));
    }
}

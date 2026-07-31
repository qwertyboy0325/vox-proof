use crate::analysis::AnalysisSnapshot;
use sha2::{Digest, Sha256};

use crate::application_reuse::reusable_influence_snapshot_for_parts;
use crate::application_service::{
    ApplicationMaterialUseDeclaration, ApplicationReviewSession,
    DeclaredApplicationMaterialUseBasis, DeclaredSessionAuthority, DeclaredSessionOperatorRole,
};
use crate::reuse_primitives::PromotionCandidateRejectionIdentity;
use crate::review::{CorrectionDecision, ReviewLedgerEvent};

use super::model::{
    CurrentContractState, EvidenceAnalysisSnapshot, EvidenceDurableCommandTokens,
    EvidenceEffectiveReviewStatus, EvidenceMaterialUseDeclaration, EvidenceProjectScope,
    EvidenceRejectedCandidateIdentity, EvidenceReusableRecord, EvidenceReuseGovernanceEvent,
    EvidenceReviewCase, EvidenceReviewLedgerEvent, EvidenceSessionAuthority,
    EvidenceSourceRevision,
};

pub fn project_current_contract_state(
    session: &ApplicationReviewSession,
    material_use: &ApplicationMaterialUseDeclaration,
) -> CurrentContractState {
    let transcript = session.source();
    let revision = transcript.revision_id().to_tagged_string();
    let parts = session.reuse_parts();
    let canonical_run = parts.canonical_run;
    let ledger = parts.ledger;
    let reuse_state = session.reuse_state();

    let session_terms_identity = canonical_run
        .analysis_run()
        .snapshot()
        .session_terms()
        .to_tagged_string();
    let analysis_snapshots = vec![EvidenceAnalysisSnapshot {
        identity: analysis_snapshot_identity(canonical_run.analysis_run().snapshot()),
        source_revision_id: revision.clone(),
    }];

    let review_cases = canonical_run
        .review_cases()
        .iter()
        .map(|case| {
            let span = case.candidate_span();
            let anchor = span.anchor();
            let observed_source_bytes = transcript.resolve(anchor).unwrap_or_default().to_owned();
            EvidenceReviewCase {
                case_id: format!("review-case:{}", case.id().local_index()),
                origin: "detector_raised".to_owned(),
                observed_revision_id: revision.clone(),
                anchor_revision_id: revision.clone(),
                anchor_segment_position: anchor.segment_position(),
                anchor_start_byte: 0,
                anchor_end_byte: observed_source_bytes.len(),
                observed_source_bytes,
            }
        })
        .collect();

    let review_ledger_events = ledger
        .events()
        .iter()
        .enumerate()
        .map(|(index, event)| map_ledger_event(index, event))
        .collect();

    let effective_review_status = session
        .review_items()
        .iter()
        .map(|item| EvidenceEffectiveReviewStatus {
            case_id: format!("review-case:{}", item.review_case.id().local_index()),
            status: format!("{:?}", item.status),
        })
        .collect();

    let project_scope = reuse_state
        .project_scope()
        .map(|scope| EvidenceProjectScope {
            stable_id: scope.stable_id.as_str().to_owned(),
            display_name: scope.display_name.as_str().to_owned(),
        })
        .unwrap_or(EvidenceProjectScope {
            stable_id: String::new(),
            display_name: String::new(),
        });

    let reuse_governance_events = reuse_state
        .governance_events()
        .iter()
        .enumerate()
        .map(|(index, event)| map_governance_event(index, event))
        .collect();

    let effective_state = reuse_state.effective_state(ledger, canonical_run);
    let rejected_candidate_identities = effective_state
        .rejected_candidate_identities()
        .iter()
        .map(|identity| EvidenceRejectedCandidateIdentity {
            identity_digest: rejection_identity_digest(identity),
        })
        .collect();

    let effective_reusable_records = effective_state
        .active_records()
        .iter()
        .map(map_effective_record)
        .collect();
    let historical_reusable_records = effective_state
        .historical_records()
        .iter()
        .map(map_effective_record)
        .collect();

    let reusable_snapshot_identity = reusable_influence_snapshot_for_parts(parts, reuse_state)
        .map(|snapshot| snapshot.identity().to_tagged_string())
        .unwrap_or_default();

    let reuse_enabled_analysis_identity = session
        .reuse_enabled_run()
        .map(|run| analysis_snapshot_identity(run.analysis_run().snapshot()))
        .unwrap_or_default();

    let material_use_basis = match material_use.basis() {
        DeclaredApplicationMaterialUseBasis::SelfOwned => "self_owned",
        DeclaredApplicationMaterialUseBasis::ExplicitPermission => "explicit_permission",
    };

    CurrentContractState {
        session_id: "session:current-contract:001".to_owned(),
        duplicated_from_session_id: None,
        session_authority: map_session_authority(session.session_authority()),
        material_use_declaration: EvidenceMaterialUseDeclaration {
            basis: material_use_basis.to_owned(),
        },
        source_revisions: vec![EvidenceSourceRevision {
            revision_id: revision.clone(),
            predecessor_revision_id: None,
            transcript_bytes: format_transcript_bytes(transcript),
        }],
        session_terms_identity,
        analysis_snapshots,
        review_cases,
        review_ledger_events,
        effective_review_status,
        project_scope,
        reuse_governance_events,
        rejected_candidate_identities,
        effective_reusable_records,
        historical_reusable_records,
        reusable_snapshot_identity,
        reuse_enabled_analysis_identity,
        derived_queue_projection: format!("queue:{}:{}", revision, ledger.events().len()),
        durable_command_tokens: EvidenceDurableCommandTokens {
            review_ledger_head: ledger.events().len(),
            reuse_governance_head: reuse_state.governance_events().len(),
            active_analysis_snapshot_identity: analysis_snapshot_identity(
                canonical_run.analysis_run().snapshot(),
            ),
        },
    }
    .normalize()
}

fn format_transcript_bytes(transcript: &crate::transcript::Transcript) -> String {
    let mut out = String::new();
    for segment in transcript.segments() {
        out.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            segment.index(),
            format_timestamp(segment.start_ms()),
            format_timestamp(segment.end_ms()),
            segment.text()
        ));
    }
    out
}

fn format_timestamp(ms: u64) -> String {
    let hours = ms / 3_600_000;
    let minutes = (ms % 3_600_000) / 60_000;
    let seconds = (ms % 60_000) / 1_000;
    let millis = ms % 1_000;
    format!("{hours:02}:{minutes:02}:{seconds:02},{millis:03}")
}

fn map_session_authority(authority: &DeclaredSessionAuthority) -> EvidenceSessionAuthority {
    let role_label = match authority.role() {
        DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator => {
            "declared_local_owner_operator".to_owned()
        }
        DeclaredSessionOperatorRole::DeclaredAuthorizedHumanReviewer => {
            "declared_authorized_human_reviewer".to_owned()
        }
    };
    EvidenceSessionAuthority {
        role_label,
        display_label: authority.display_label().to_owned(),
    }
}

fn map_ledger_event(index: usize, event: &ReviewLedgerEvent) -> EvidenceReviewLedgerEvent {
    let ReviewLedgerEvent::DecisionRecorded {
        case_id,
        observed_revision,
        decision,
    } = event;
    let (action_kind, manual_replacement_bytes, alternative_index) = match decision {
        CorrectionDecision::AcceptAlternative { alternative_index } => (
            "accept_alternative".to_owned(),
            None,
            Some(*alternative_index),
        ),
        CorrectionDecision::ManualReplacement { replacement } => (
            "manual_replacement".to_owned(),
            Some(replacement.as_str().to_owned()),
            None,
        ),
        CorrectionDecision::Reject => ("reject".to_owned(), None, None),
        CorrectionDecision::Defer => ("defer".to_owned(), None, None),
        CorrectionDecision::NeedsManualCorrection => {
            ("needs_manual_correction".to_owned(), None, None)
        }
    };
    EvidenceReviewLedgerEvent {
        event_index: index,
        case_id: format!("review-case:{}", case_id.local_index()),
        observed_revision_id: observed_revision.to_tagged_string(),
        action_kind,
        manual_replacement_bytes,
        alternative_index,
        target_event_index: None,
        provenance: "human".to_owned(),
    }
}

fn map_governance_event(
    index: usize,
    event: &crate::reusable_influence::ReusableGovernanceEvent,
) -> EvidenceReuseGovernanceEvent {
    use crate::reusable_influence::ReusableGovernanceEvent;
    match event {
        ReusableGovernanceEvent::PromotionCandidateRejected {
            candidate_key,
            actor,
        } => EvidenceReuseGovernanceEvent {
            event_index: index,
            event_kind: "promotion_candidate_rejected".to_owned(),
            actor_role_label: actor.role_label.clone(),
            actor_display_label: actor.display_label.clone(),
            candidate_key_digest: Some(digest_hex(&format!("{candidate_key:?}"))),
            record_id: None,
            predecessor_record_id: None,
            successor_record_id: None,
            observed_text: None,
            confirmed_replacement: None,
            source_locator_digest: Some(digest_hex(
                &candidate_key
                    .source_locator
                    .source_revision
                    .to_tagged_string(),
            )),
            project_scope_stable_id: Some(candidate_key.project_scope_id.as_str().to_owned()),
        },
        ReusableGovernanceEvent::PromotionAccepted {
            candidate_key,
            payload,
            source_locator,
            actor,
            project_scope,
        } => EvidenceReuseGovernanceEvent {
            event_index: index,
            event_kind: "promotion_accepted".to_owned(),
            actor_role_label: actor.role_label.clone(),
            actor_display_label: actor.display_label.clone(),
            candidate_key_digest: Some(digest_hex(&format!("{candidate_key:?}"))),
            record_id: Some(index),
            predecessor_record_id: None,
            successor_record_id: None,
            observed_text: Some(payload.observed_text.clone()),
            confirmed_replacement: Some(payload.confirmed_replacement.clone()),
            source_locator_digest: Some(digest_hex(
                &source_locator.source_revision.to_tagged_string(),
            )),
            project_scope_stable_id: Some(project_scope.stable_id.as_str().to_owned()),
        },
        ReusableGovernanceEvent::ReusableInfluenceRevoked { record_id, actor } => {
            EvidenceReuseGovernanceEvent {
                event_index: index,
                event_kind: "reusable_influence_revoked".to_owned(),
                actor_role_label: actor.role_label.clone(),
                actor_display_label: actor.display_label.clone(),
                candidate_key_digest: None,
                record_id: Some(record_id.promotion_event_index()),
                predecessor_record_id: None,
                successor_record_id: None,
                observed_text: None,
                confirmed_replacement: None,
                source_locator_digest: None,
                project_scope_stable_id: None,
            }
        }
        ReusableGovernanceEvent::ReusableInfluenceSuperseded {
            predecessor_id,
            successor_id,
            actor,
        } => EvidenceReuseGovernanceEvent {
            event_index: index,
            event_kind: "reusable_influence_superseded".to_owned(),
            actor_role_label: actor.role_label.clone(),
            actor_display_label: actor.display_label.clone(),
            candidate_key_digest: None,
            record_id: None,
            predecessor_record_id: Some(predecessor_id.promotion_event_index()),
            successor_record_id: Some(successor_id.promotion_event_index()),
            observed_text: None,
            confirmed_replacement: None,
            source_locator_digest: None,
            project_scope_stable_id: None,
        },
    }
}

fn map_effective_record(
    record: &crate::reusable_influence::EffectiveReusableInfluenceRecord,
) -> EvidenceReusableRecord {
    EvidenceReusableRecord {
        record_id: record.record_id.promotion_event_index(),
        project_scope_stable_id: record.project_scope.stable_id.as_str().to_owned(),
        observed_text: record.payload.observed_text.clone(),
        confirmed_replacement: record.payload.confirmed_replacement.clone(),
        source_locator_digest: digest_hex(
            &record.source_locator.source_revision.to_tagged_string(),
        ),
        promotion_actor_role_label: record.promotion_actor.role_label.clone(),
        promotion_actor_display_label: record.promotion_actor.display_label.clone(),
        superseded_by: record.superseded_by.map(|id| id.promotion_event_index()),
    }
}

fn rejection_identity_digest(identity: &PromotionCandidateRejectionIdentity) -> String {
    digest_hex(&format!(
        "{}:{}:{}:{}",
        identity.project_scope_id.as_str(),
        identity.source_review_case_id.local_index(),
        identity.review_ledger_position,
        digest_hex_bytes(identity.decision_digest)
    ))
}

fn digest_hex(value: &str) -> String {
    format!(
        "sha256:{}",
        digest_hex_bytes(Sha256::digest(value.as_bytes()))
    )
}

fn digest_hex_bytes(bytes: impl AsRef<[u8]>) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let bytes = bytes.as_ref();
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn analysis_snapshot_identity(snapshot: AnalysisSnapshot) -> String {
    let detector_ids = snapshot
        .configuration()
        .detector_set()
        .detectors()
        .iter()
        .map(|detector| format!("{}@{}", detector.id(), detector.version()))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "analysis-snapshot:{}:{}:{}:{}@{}:{}@{}",
        snapshot.source_revision().to_tagged_string(),
        snapshot.session_terms().to_tagged_string(),
        detector_ids,
        snapshot.configuration().detector_config().id(),
        snapshot.configuration().detector_config().version(),
        snapshot.configuration().algorithm().id(),
        snapshot.configuration().algorithm().version(),
    )
}

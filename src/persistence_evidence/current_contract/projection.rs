use crate::analysis::AnalysisSnapshot;

use super::derivation::finalize_derived_fields;
use crate::application_service::{
    ApplicationMaterialUseDeclaration, ApplicationReviewSession,
    DeclaredApplicationMaterialUseBasis, DeclaredSessionAuthority, DeclaredSessionOperatorRole,
};
use crate::reusable_influence::{
    ExactReusableCorrection, ReusableGovernanceEvent, ReuseCandidateKey,
};
use crate::reuse_primitives::SourceDecisionLocator;
use crate::review::{CorrectionDecision, ReviewLedgerEvent};

use super::model::{
    CurrentContractState, EvidenceAnalysisSnapshot, EvidenceDetectorIdentity,
    EvidenceDurableCommandTokens, EvidenceExactReusableCorrection, EvidenceGovernanceActor,
    EvidenceMaterialUseDeclaration, EvidenceProjectScope, EvidenceReuseCandidateKey,
    EvidenceReuseEnabledAnalysisBinding, EvidenceReuseGovernanceEvent, EvidenceReviewCase,
    EvidenceReviewLedgerEvent, EvidenceSessionAuthority, EvidenceSourceDecisionLocator,
    EvidenceSourceRevision, REUSABLE_INFLUENCE_PROJECTION_VERSION,
};
use super::serialization::candidate_key_canonical_digest;

pub fn project_current_contract_state(
    session: &ApplicationReviewSession,
    material_use: &ApplicationMaterialUseDeclaration,
    session_id: &str,
    evidence_writer_token: &str,
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
    let mut analysis_snapshots = vec![map_analysis_snapshot(
        canonical_run.analysis_run().snapshot(),
        revision.clone(),
    )];
    let reuse_enabled_analysis_binding = session.reuse_enabled_run().map(|reuse_run| {
        let snapshot = map_analysis_snapshot(reuse_run.analysis_run().snapshot(), revision.clone());
        EvidenceReuseEnabledAnalysisBinding {
            analysis_snapshot_identity: snapshot.identity.clone(),
            analysis_snapshot: snapshot.clone(),
            reusable_snapshot_identity: reuse_run.reusable_snapshot_identity().to_tagged_string(),
            governance_event_boundary: reuse_run.governance_event_boundary_at_run(),
            projection_version: REUSABLE_INFLUENCE_PROJECTION_VERSION.to_owned(),
        }
    });
    if let Some(reuse_run) = session.reuse_enabled_run() {
        analysis_snapshots.push(map_analysis_snapshot(
            reuse_run.analysis_run().snapshot(),
            revision.clone(),
        ));
    }

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
                anchor_start_byte: anchor.start_byte(),
                anchor_end_byte: anchor.end_byte(),
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

    let effective_review_status = Vec::new();

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
        .map(|(index, event)| map_governance_event(index, event, session))
        .collect();

    let effective_reusable_records = Vec::new();
    let historical_reusable_records = Vec::new();
    let reusable_snapshot_identity = String::new();

    let material_use_basis = match material_use.basis() {
        DeclaredApplicationMaterialUseBasis::SelfOwned => "self_owned",
        DeclaredApplicationMaterialUseBasis::ExplicitPermission => "explicit_permission",
    };

    let mut state = CurrentContractState {
        session_id: session_id.to_owned(),
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
        effective_reusable_records,
        historical_reusable_records,
        reusable_snapshot_identity,
        reuse_enabled_analysis_binding,
        derived_queue_projection: format!("queue:{}:{}", revision, ledger.events().len()),
        durable_command_tokens: EvidenceDurableCommandTokens {
            review_ledger_head: ledger.events().len(),
            reuse_governance_head: reuse_state.governance_events().len(),
            active_analysis_snapshot_identity: analysis_snapshot_identity(
                canonical_run.analysis_run().snapshot(),
            ),
            evidence_writer_token: evidence_writer_token.to_owned(),
        },
    };
    finalize_derived_fields(&mut state);
    state.normalize()
}

pub fn map_locator(locator: &SourceDecisionLocator) -> EvidenceSourceDecisionLocator {
    EvidenceSourceDecisionLocator {
        source_revision_id: locator.source_revision.to_tagged_string(),
        source_analysis_snapshot_identity: analysis_snapshot_identity(
            locator.source_analysis_snapshot,
        ),
        source_review_case_id: format!(
            "review-case:{}",
            locator.source_review_case_id.local_index()
        ),
        review_ledger_position: locator.review_ledger_position,
        decision_digest: digest_hex(locator.decision_digest),
        effective_at_ledger_length: locator.effective_at_ledger_length,
    }
}

pub fn map_candidate_key(
    key: &ReuseCandidateKey,
    payload: &ExactReusableCorrection,
) -> EvidenceReuseCandidateKey {
    EvidenceReuseCandidateKey {
        project_scope_stable_id: key.project_scope_id.as_str().to_owned(),
        source_locator: map_locator(&key.source_locator),
        exact_payload: EvidenceExactReusableCorrection {
            observed_text: payload.observed_text.clone(),
            confirmed_replacement: payload.confirmed_replacement.clone(),
        },
    }
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
    event: &ReusableGovernanceEvent,
    session: &ApplicationReviewSession,
) -> EvidenceReuseGovernanceEvent {
    match event {
        ReusableGovernanceEvent::PromotionCandidateRejected {
            candidate_key,
            actor,
        } => {
            let payload = rejection_payload_for_key(session, candidate_key);
            EvidenceReuseGovernanceEvent::PromotionCandidateRejected {
                event_index: index,
                candidate_key: map_candidate_key(candidate_key, &payload),
                actor: map_actor(actor),
            }
        }
        ReusableGovernanceEvent::PromotionAccepted {
            candidate_key,
            payload,
            source_locator,
            actor,
            project_scope,
        } => EvidenceReuseGovernanceEvent::PromotionAccepted {
            event_index: index,
            candidate_key: map_candidate_key(candidate_key, payload),
            payload: EvidenceExactReusableCorrection {
                observed_text: payload.observed_text.clone(),
                confirmed_replacement: payload.confirmed_replacement.clone(),
            },
            source_locator: map_locator(source_locator),
            actor: map_actor(actor),
            project_scope_stable_id: project_scope.stable_id.as_str().to_owned(),
        },
        ReusableGovernanceEvent::ReusableInfluenceRevoked { record_id, actor } => {
            EvidenceReuseGovernanceEvent::ReusableInfluenceRevoked {
                event_index: index,
                record_id: record_id.promotion_event_index(),
                actor: map_actor(actor),
            }
        }
        ReusableGovernanceEvent::ReusableInfluenceSuperseded {
            predecessor_id,
            successor_id,
            actor,
        } => EvidenceReuseGovernanceEvent::ReusableInfluenceSuperseded {
            event_index: index,
            predecessor_record_id: predecessor_id.promotion_event_index(),
            successor_record_id: successor_id.promotion_event_index(),
            actor: map_actor(actor),
        },
    }
}

fn rejection_payload_for_key(
    session: &ApplicationReviewSession,
    candidate_key: &ReuseCandidateKey,
) -> ExactReusableCorrection {
    let parts = session.reuse_parts();
    let locator = &candidate_key.source_locator;
    let case_id = locator.source_review_case_id;
    let Some(review_case) = parts
        .canonical_run
        .review_cases()
        .get(case_id.local_index())
    else {
        return ExactReusableCorrection {
            observed_text: String::new(),
            confirmed_replacement: String::new(),
        };
    };
    let observed_text = parts
        .transcript
        .resolve(review_case.candidate_span().anchor())
        .unwrap_or("")
        .to_owned();
    let replacement = parts
        .ledger
        .events()
        .get(locator.review_ledger_position)
        .and_then(|event| {
            let ReviewLedgerEvent::DecisionRecorded { decision, .. } = event;
            match decision {
                CorrectionDecision::ManualReplacement { replacement } => {
                    Some(replacement.as_str().to_owned())
                }
                _ => None,
            }
        })
        .unwrap_or_default();
    ExactReusableCorrection {
        observed_text,
        confirmed_replacement: replacement,
    }
}

fn map_actor(actor: &crate::reusable_influence::GovernanceActorContext) -> EvidenceGovernanceActor {
    EvidenceGovernanceActor {
        role_label: actor.role_label.clone(),
        display_label: actor.display_label.clone(),
    }
}

fn digest_hex(bytes: [u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

pub fn map_analysis_snapshot_for_export(
    snapshot: AnalysisSnapshot,
    source_revision_id: String,
) -> EvidenceAnalysisSnapshot {
    map_analysis_snapshot(snapshot, source_revision_id)
}

fn map_analysis_snapshot(
    snapshot: AnalysisSnapshot,
    source_revision_id: String,
) -> EvidenceAnalysisSnapshot {
    let configuration = snapshot.configuration();
    let detectors = configuration
        .detector_set()
        .detectors()
        .iter()
        .map(|detector| EvidenceDetectorIdentity {
            id: detector.id().to_owned(),
            version: detector.version().to_owned(),
        })
        .collect();
    EvidenceAnalysisSnapshot {
        identity: analysis_snapshot_identity(snapshot),
        source_revision_id,
        session_terms_identity: snapshot.session_terms().to_tagged_string(),
        detectors,
        detector_config_id: configuration.detector_config().id().to_owned(),
        detector_config_version: configuration.detector_config().version().to_owned(),
        algorithm_id: configuration.algorithm().id().to_owned(),
        algorithm_version: configuration.algorithm().version().to_owned(),
    }
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

pub fn candidate_key_identity_digest(key: &EvidenceReuseCandidateKey) -> String {
    candidate_key_canonical_digest(key)
}

#[cfg(test)]
mod projection_tests {
    use super::candidate_key_identity_digest;
    use crate::persistence_evidence::current_contract::model::{
        EvidenceExactReusableCorrection, EvidenceReuseCandidateKey, EvidenceSourceDecisionLocator,
    };

    #[test]
    fn candidate_key_digest_is_stable_and_not_debug_based() {
        let key = EvidenceReuseCandidateKey {
            project_scope_stable_id: "proj-a".to_owned(),
            source_locator: EvidenceSourceDecisionLocator {
                source_revision_id: "rev:sha256-v1:abc".to_owned(),
                source_analysis_snapshot_identity: "analysis-snapshot:test".to_owned(),
                source_review_case_id: "review-case:0".to_owned(),
                review_ledger_position: 0,
                decision_digest: "deadbeef".to_owned(),
                effective_at_ledger_length: 1,
            },
            exact_payload: EvidenceExactReusableCorrection {
                observed_text: "Kafak".to_owned(),
                confirmed_replacement: "Kafka".to_owned(),
            },
        };
        let first = candidate_key_identity_digest(&key);
        let second = candidate_key_identity_digest(&key);
        assert_eq!(first, second);
        assert!(first.starts_with("sha256:"));
    }
}

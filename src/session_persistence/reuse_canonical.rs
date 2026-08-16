use serde::{Deserialize, Serialize};

use crate::reusable_influence::{
    AllowedEffectsConsent, ExactReusableCorrection, GovernanceActorContext,
    ReusableGovernanceEvent, ReuseAllowedEffect, ReuseCandidateKey,
};
use crate::reuse_primitives::{
    ProjectScope, ProjectScopeDisplayName, ProjectScopeId, ReusableInfluenceRecordId,
    ReusableInfluenceSnapshotIdentity, SourceDecisionLocator, SourceDecisionLocatorOrigin,
};
use crate::session_persistence::canonical::{
    PersistedAnalysisSnapshotV1, encode_digest_hex, persist_analysis_snapshot,
    restore_analysis_snapshot_from_persisted, verify_analysis_snapshot,
};
use crate::session_persistence::error::SessionPersistenceError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PersistedProjectScopeV1 {
    pub(crate) stable_id: String,
    pub(crate) display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PersistedGovernanceActorV1 {
    pub(crate) role_label: String,
    pub(crate) display_label: String,
}

pub(crate) const PERSISTED_LOCATOR_ORIGIN_KIND_DETECTOR: &str = "detector";
pub(crate) const PERSISTED_LOCATOR_ORIGIN_KIND_HUMAN: &str = "human";

/// Persisted locator shape.
///
/// `origin_kind` is additive: records written before MD-022 omit it and restore as
/// `DetectorRaised`, which keeps their required `source_analysis_snapshot` present.
/// Human-raised records write `origin_kind = "human"` and omit the snapshot rather
/// than inventing one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PersistedSourceDecisionLocatorV1 {
    pub(crate) source_revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) origin_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) source_analysis_snapshot: Option<PersistedAnalysisSnapshotV1>,
    pub(crate) source_review_case_local_index: usize,
    pub(crate) review_ledger_position: usize,
    pub(crate) decision_digest_hex: String,
    pub(crate) effective_at_ledger_length: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PersistedExactReusableCorrectionV1 {
    pub(crate) observed_text: String,
    pub(crate) confirmed_replacement: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PersistedReuseCandidateKeyV1 {
    pub(crate) project_scope_stable_id: String,
    pub(crate) source_locator: PersistedSourceDecisionLocatorV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PersistedReuseAllowedEffectV1 {
    ExactObservedFormProposalGeneration,
    DerivedCanonicalTerminologyProposalGeneration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event_kind", rename_all = "snake_case")]
pub(crate) enum PersistedReuseGovernanceEventV1 {
    PromotionCandidateRejected {
        candidate_key: PersistedReuseCandidateKeyV1,
        actor: PersistedGovernanceActorV1,
    },
    PromotionAccepted {
        candidate_key: PersistedReuseCandidateKeyV1,
        payload: PersistedExactReusableCorrectionV1,
        source_locator: PersistedSourceDecisionLocatorV1,
        actor: PersistedGovernanceActorV1,
        project_scope_stable_id: String,
        project_scope_display_name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        allowed_effects: Option<Vec<PersistedReuseAllowedEffectV1>>,
    },
    ReusableInfluenceRevoked {
        record_id: usize,
        actor: PersistedGovernanceActorV1,
    },
    ReusableInfluenceSuperseded {
        predecessor_record_id: usize,
        successor_record_id: usize,
        actor: PersistedGovernanceActorV1,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PersistedReuseEnabledBindingV1 {
    pub(crate) binding_id: usize,
    pub(crate) analysis_snapshot: PersistedAnalysisSnapshotV1,
    pub(crate) reusable_snapshot_identity: String,
    pub(crate) governance_event_boundary: usize,
}

pub(crate) fn project_scope_is_uninitialized(scope: &PersistedProjectScopeV1) -> bool {
    scope.stable_id.is_empty()
}

pub(crate) fn persist_project_scope(scope: &ProjectScope) -> PersistedProjectScopeV1 {
    PersistedProjectScopeV1 {
        stable_id: scope.stable_id.as_str().to_owned(),
        display_name: scope.display_name.as_str().to_owned(),
    }
}

pub(crate) fn restore_project_scope(
    persisted: &PersistedProjectScopeV1,
) -> Result<Option<ProjectScope>, SessionPersistenceError> {
    if project_scope_is_uninitialized(persisted) {
        return Ok(None);
    }
    Ok(Some(ProjectScope::new(
        ProjectScopeId::new(persisted.stable_id.clone())
            .map_err(|_| SessionPersistenceError::CanonicalMismatch("project scope id".into()))?,
        ProjectScopeDisplayName::new(persisted.display_name.clone()).map_err(|_| {
            SessionPersistenceError::CanonicalMismatch("project scope display name".into())
        })?,
    )))
}

pub(crate) fn persist_governance_actor(
    actor: &GovernanceActorContext,
) -> PersistedGovernanceActorV1 {
    PersistedGovernanceActorV1 {
        role_label: actor.role_label.clone(),
        display_label: actor.display_label.clone(),
    }
}

pub(crate) fn restore_governance_actor(
    persisted: &PersistedGovernanceActorV1,
) -> GovernanceActorContext {
    GovernanceActorContext {
        role_label: persisted.role_label.clone(),
        display_label: persisted.display_label.clone(),
    }
}

pub(crate) fn persist_source_locator(
    locator: &SourceDecisionLocator,
) -> PersistedSourceDecisionLocatorV1 {
    let (origin_kind, source_analysis_snapshot) = match &locator.origin {
        SourceDecisionLocatorOrigin::DetectorRaised {
            source_analysis_snapshot,
            ..
        } => (
            PERSISTED_LOCATOR_ORIGIN_KIND_DETECTOR,
            Some(persist_analysis_snapshot(*source_analysis_snapshot)),
        ),
        SourceDecisionLocatorOrigin::HumanRaised { .. } => {
            (PERSISTED_LOCATOR_ORIGIN_KIND_HUMAN, None)
        }
    };
    PersistedSourceDecisionLocatorV1 {
        source_revision: locator.source_revision.to_tagged_string(),
        origin_kind: Some(origin_kind.to_owned()),
        source_analysis_snapshot,
        source_review_case_local_index: locator.source_review_case_id().local_index(),
        review_ledger_position: locator.review_ledger_position,
        decision_digest_hex: encode_digest_hex(locator.decision_digest),
        effective_at_ledger_length: locator.effective_at_ledger_length,
    }
}

pub(crate) fn restore_source_locator(
    persisted: &PersistedSourceDecisionLocatorV1,
) -> Result<SourceDecisionLocator, SessionPersistenceError> {
    let local_index = persisted.source_review_case_local_index;
    let (source_revision, origin) = match persisted.origin_kind.as_deref() {
        None | Some(PERSISTED_LOCATOR_ORIGIN_KIND_DETECTOR) => {
            let persisted_snapshot =
                persisted.source_analysis_snapshot.as_ref().ok_or_else(|| {
                    SessionPersistenceError::CanonicalMismatch(
                        "detector-raised source locator requires source_analysis_snapshot".into(),
                    )
                })?;
            let source_analysis_snapshot =
                restore_analysis_snapshot_from_persisted(persisted_snapshot)?;
            (
                source_analysis_snapshot.source_revision(),
                SourceDecisionLocatorOrigin::DetectorRaised {
                    source_analysis_snapshot,
                    source_review_case_id: crate::review::ReviewCaseId::local(local_index),
                },
            )
        }
        Some(PERSISTED_LOCATOR_ORIGIN_KIND_HUMAN) => {
            if persisted.source_analysis_snapshot.is_some() {
                return Err(SessionPersistenceError::CanonicalMismatch(
                    "human-raised source locator must not carry source_analysis_snapshot".into(),
                ));
            }
            let source_revision = parse_revision_tag(&persisted.source_revision)?;
            (
                source_revision,
                SourceDecisionLocatorOrigin::HumanRaised {
                    human_raised_case_id: crate::review::ReviewCaseId::human(local_index),
                },
            )
        }
        Some(_) => {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "unknown source locator origin_kind".into(),
            ));
        }
    };
    Ok(SourceDecisionLocator {
        source_revision,
        origin,
        review_ledger_position: persisted.review_ledger_position,
        decision_digest: parse_digest_hex(&persisted.decision_digest_hex)?,
        effective_at_ledger_length: persisted.effective_at_ledger_length,
    })
}

pub(crate) fn persist_candidate_key(key: &ReuseCandidateKey) -> PersistedReuseCandidateKeyV1 {
    PersistedReuseCandidateKeyV1 {
        project_scope_stable_id: key.project_scope_id.as_str().to_owned(),
        source_locator: persist_source_locator(&key.source_locator),
    }
}

pub(crate) fn restore_candidate_key(
    persisted: &PersistedReuseCandidateKeyV1,
) -> Result<ReuseCandidateKey, SessionPersistenceError> {
    Ok(ReuseCandidateKey {
        project_scope_id: ProjectScopeId::new(persisted.project_scope_stable_id.clone())
            .map_err(|_| SessionPersistenceError::CanonicalMismatch("project scope id".into()))?,
        source_locator: restore_source_locator(&persisted.source_locator)?,
    })
}

pub(crate) fn persist_governance_event(
    event: &ReusableGovernanceEvent,
) -> PersistedReuseGovernanceEventV1 {
    match event {
        ReusableGovernanceEvent::PromotionCandidateRejected {
            candidate_key,
            actor,
        } => PersistedReuseGovernanceEventV1::PromotionCandidateRejected {
            candidate_key: persist_candidate_key(candidate_key),
            actor: persist_governance_actor(actor),
        },
        ReusableGovernanceEvent::PromotionAccepted {
            candidate_key,
            payload,
            source_locator,
            actor,
            project_scope,
            allowed_effects,
        } => PersistedReuseGovernanceEventV1::PromotionAccepted {
            candidate_key: persist_candidate_key(candidate_key),
            payload: PersistedExactReusableCorrectionV1 {
                observed_text: payload.observed_text.clone(),
                confirmed_replacement: payload.confirmed_replacement.clone(),
            },
            source_locator: persist_source_locator(source_locator),
            actor: persist_governance_actor(actor),
            project_scope_stable_id: project_scope.stable_id.as_str().to_owned(),
            project_scope_display_name: project_scope.display_name.as_str().to_owned(),
            allowed_effects: persist_allowed_effects_consent(allowed_effects),
        },
        ReusableGovernanceEvent::ReusableInfluenceRevoked { record_id, actor } => {
            PersistedReuseGovernanceEventV1::ReusableInfluenceRevoked {
                record_id: record_id.promotion_event_index(),
                actor: persist_governance_actor(actor),
            }
        }
        ReusableGovernanceEvent::ReusableInfluenceSuperseded {
            predecessor_id,
            successor_id,
            actor,
        } => PersistedReuseGovernanceEventV1::ReusableInfluenceSuperseded {
            predecessor_record_id: predecessor_id.promotion_event_index(),
            successor_record_id: successor_id.promotion_event_index(),
            actor: persist_governance_actor(actor),
        },
    }
}

pub(crate) fn restore_governance_event(
    persisted: &PersistedReuseGovernanceEventV1,
) -> Result<ReusableGovernanceEvent, SessionPersistenceError> {
    Ok(match persisted {
        PersistedReuseGovernanceEventV1::PromotionCandidateRejected {
            candidate_key,
            actor,
        } => ReusableGovernanceEvent::PromotionCandidateRejected {
            candidate_key: Box::new(restore_candidate_key(candidate_key)?),
            actor: restore_governance_actor(actor),
        },
        PersistedReuseGovernanceEventV1::PromotionAccepted {
            candidate_key,
            payload,
            source_locator,
            actor,
            project_scope_stable_id,
            project_scope_display_name,
            allowed_effects,
        } => ReusableGovernanceEvent::PromotionAccepted {
            candidate_key: Box::new(restore_candidate_key(candidate_key)?),
            payload: ExactReusableCorrection {
                observed_text: payload.observed_text.clone(),
                confirmed_replacement: payload.confirmed_replacement.clone(),
            },
            source_locator: Box::new(restore_source_locator(source_locator)?),
            actor: restore_governance_actor(actor),
            project_scope: Box::new(ProjectScope::new(
                ProjectScopeId::new(project_scope_stable_id.clone()).map_err(|_| {
                    SessionPersistenceError::CanonicalMismatch("project scope id".into())
                })?,
                ProjectScopeDisplayName::new(project_scope_display_name.clone()).map_err(|_| {
                    SessionPersistenceError::CanonicalMismatch("project scope display name".into())
                })?,
            )),
            allowed_effects: restore_allowed_effects_consent(allowed_effects)?,
        },
        PersistedReuseGovernanceEventV1::ReusableInfluenceRevoked { record_id, actor } => {
            ReusableGovernanceEvent::ReusableInfluenceRevoked {
                record_id: ReusableInfluenceRecordId::from_promotion_event_index(*record_id),
                actor: restore_governance_actor(actor),
            }
        }
        PersistedReuseGovernanceEventV1::ReusableInfluenceSuperseded {
            predecessor_record_id,
            successor_record_id,
            actor,
        } => ReusableGovernanceEvent::ReusableInfluenceSuperseded {
            predecessor_id: ReusableInfluenceRecordId::from_promotion_event_index(
                *predecessor_record_id,
            ),
            successor_id: ReusableInfluenceRecordId::from_promotion_event_index(
                *successor_record_id,
            ),
            actor: restore_governance_actor(actor),
        },
    })
}

fn persist_allowed_effects_consent(
    consent: &AllowedEffectsConsent,
) -> Option<Vec<PersistedReuseAllowedEffectV1>> {
    match consent {
        AllowedEffectsConsent::HistoricalExactOnly => None,
        AllowedEffectsConsent::Explicit(effects) => Some(
            effects
                .iter()
                .copied()
                .map(persist_allowed_effect)
                .collect(),
        ),
    }
}

fn persist_allowed_effect(effect: ReuseAllowedEffect) -> PersistedReuseAllowedEffectV1 {
    match effect {
        ReuseAllowedEffect::ExactObservedFormProposalGeneration => {
            PersistedReuseAllowedEffectV1::ExactObservedFormProposalGeneration
        }
        ReuseAllowedEffect::DerivedCanonicalTerminologyProposalGeneration => {
            PersistedReuseAllowedEffectV1::DerivedCanonicalTerminologyProposalGeneration
        }
    }
}

fn restore_allowed_effects_consent(
    persisted: &Option<Vec<PersistedReuseAllowedEffectV1>>,
) -> Result<AllowedEffectsConsent, SessionPersistenceError> {
    match persisted {
        None => Ok(AllowedEffectsConsent::HistoricalExactOnly),
        Some(effects) => {
            if effects.is_empty() {
                return Err(SessionPersistenceError::CanonicalMismatch(
                    "allowed_effects must not be empty when present".to_owned(),
                ));
            }
            let restored: Vec<ReuseAllowedEffect> = effects
                .iter()
                .copied()
                .map(restore_allowed_effect)
                .collect();
            if !restored.contains(&ReuseAllowedEffect::ExactObservedFormProposalGeneration) {
                return Err(SessionPersistenceError::CanonicalMismatch(
                    "allowed_effects must include ExactObservedFormProposalGeneration".to_owned(),
                ));
            }
            Ok(AllowedEffectsConsent::Explicit(restored))
        }
    }
}

fn restore_allowed_effect(persisted: PersistedReuseAllowedEffectV1) -> ReuseAllowedEffect {
    match persisted {
        PersistedReuseAllowedEffectV1::ExactObservedFormProposalGeneration => {
            ReuseAllowedEffect::ExactObservedFormProposalGeneration
        }
        PersistedReuseAllowedEffectV1::DerivedCanonicalTerminologyProposalGeneration => {
            ReuseAllowedEffect::DerivedCanonicalTerminologyProposalGeneration
        }
    }
}

pub(crate) fn persist_reuse_enabled_binding(
    binding_id: usize,
    analysis_snapshot: crate::analysis::AnalysisSnapshot,
    reusable_snapshot_identity: crate::reuse_primitives::ReusableInfluenceSnapshotIdentity,
    governance_event_boundary: usize,
) -> PersistedReuseEnabledBindingV1 {
    PersistedReuseEnabledBindingV1 {
        binding_id,
        analysis_snapshot: persist_analysis_snapshot(analysis_snapshot),
        reusable_snapshot_identity: reusable_snapshot_identity.to_tagged_string(),
        governance_event_boundary,
    }
}

pub(crate) fn restore_reusable_snapshot_identity(
    tagged: &str,
) -> Result<crate::reuse_primitives::ReusableInfluenceSnapshotIdentity, SessionPersistenceError> {
    let hex = tagged
        .strip_prefix("reusable-influence-snapshot:sha256-v2:")
        .ok_or_else(|| {
            SessionPersistenceError::CanonicalMismatch("reusable snapshot identity".into())
        })?;
    parse_digest_hex(hex)
        .map(crate::reuse_primitives::ReusableInfluenceSnapshotIdentity::from_digest)
}

pub(crate) fn active_analysis_selection_identity_for_reuse_enabled(
    analysis_snapshot: crate::analysis::AnalysisSnapshot,
    reusable_snapshot_identity: ReusableInfluenceSnapshotIdentity,
    governance_event_boundary: usize,
) -> String {
    crate::analysis::reuse_enabled_active_analysis_selection_identity(
        analysis_snapshot,
        reusable_snapshot_identity,
        governance_event_boundary,
    )
}

pub(crate) fn verify_persisted_analysis_matches_runtime(
    persisted: &PersistedAnalysisSnapshotV1,
    actual: crate::analysis::AnalysisSnapshot,
) -> Result<(), SessionPersistenceError> {
    verify_analysis_snapshot(persisted, actual)
}

fn parse_revision_tag(
    tag: &str,
) -> Result<crate::anchor::TranscriptRevisionId, SessionPersistenceError> {
    let hex = tag.strip_prefix("rev:sha256-v1:").ok_or_else(|| {
        SessionPersistenceError::CanonicalMismatch("source locator source_revision".into())
    })?;
    Ok(crate::anchor::TranscriptRevisionId::from_sha256_digest(
        parse_digest_hex(hex)?,
    ))
}

fn parse_digest_hex(hex: &str) -> Result<[u8; 32], SessionPersistenceError> {
    if hex.len() != 64 {
        return Err(SessionPersistenceError::CanonicalMismatch(
            "decision digest".into(),
        ));
    }
    let mut digest = [0_u8; 32];
    for (index, chunk) in hex.as_bytes().chunks(2).enumerate() {
        if index >= 32 || chunk.len() != 2 {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "decision digest".into(),
            ));
        }
        let hi = (chunk[0] as char)
            .to_digit(16)
            .ok_or_else(|| SessionPersistenceError::CanonicalMismatch("decision digest".into()))?
            as u8;
        let lo = (chunk[1] as char)
            .to_digit(16)
            .ok_or_else(|| SessionPersistenceError::CanonicalMismatch("decision digest".into()))?
            as u8;
        digest[index] = (hi << 4) | lo;
    }
    Ok(digest)
}

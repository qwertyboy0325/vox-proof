use serde::{Deserialize, Serialize};

use crate::analysis::{AnalysisConfigurationIdentity, AnalysisSnapshot, SessionTermsIdentity};
use crate::anchor::TranscriptRevisionId;
use crate::application_reuse::reusable_influence_snapshot_for_parts;
use crate::application_service::ApplicationReviewSession;
use crate::candidate::{
    AsciiLatinPhoneticRepresentation, CandidateAlternative, CandidateSpan, DetectionKind,
    DetectorProvenance, Evidence, GlossaryAliasEvidence, ObservedErrorFormEvidence,
    PhoneticComparisonFacts, PhoneticSimilarityEvidence, PhoneticTargetKind, SessionTermEntry,
    algorithm_identity_from_parts, detector_config_identity_from_parts,
};
use crate::pipeline::CanonicalTermReviewRun;
use crate::review::{CorrectionDecision, ReviewCase, ReviewCaseId, ReviewLedgerEvent};
use crate::session_persistence::error::SessionPersistenceError;
use crate::session_persistence::reuse_canonical::{
    PersistedProjectScopeV1, PersistedReuseEnabledBindingV1, PersistedReuseGovernanceEventV1,
    active_analysis_selection_identity_for_reuse_enabled, persist_governance_event,
    persist_project_scope, persist_reuse_enabled_binding,
};
use crate::transcript::{Segment, Transcript};

pub(crate) const PRODUCT_SESSION_FORMAT_VERSION: u32 = 1;
pub(crate) const PRODUCT_SESSION_FORMAT_VERSION_V2: u32 = 2;
/// New-session-only successor format that may carry HumanRaised review cases. There is no
/// automatic migration from v1 or v2.
pub(crate) const PRODUCT_SESSION_FORMAT_VERSION_V3: u32 = 3;
/// New-session-only successor format that may carry `ProjectTerminologyProposal` decisions.
/// There is no automatic migration from v1, v2, or v3.
pub(crate) const PRODUCT_SESSION_FORMAT_VERSION_V4: u32 = 4;

pub(crate) fn latest_supported_session_format() -> u32 {
    PRODUCT_SESSION_FORMAT_VERSION_V4
}

pub(crate) fn supported_session_format(format_version: u32) -> bool {
    format_version == PRODUCT_SESSION_FORMAT_VERSION
        || format_version == PRODUCT_SESSION_FORMAT_VERSION_V2
        || format_version == PRODUCT_SESSION_FORMAT_VERSION_V3
        || format_version == PRODUCT_SESSION_FORMAT_VERSION_V4
}

pub(crate) fn session_format_supports_human_raised(format_version: u32) -> bool {
    format_version == PRODUCT_SESSION_FORMAT_VERSION_V3
        || format_version == PRODUCT_SESSION_FORMAT_VERSION_V4
}

pub(crate) fn session_format_supports_project_binding(format_version: u32) -> bool {
    format_version == PRODUCT_SESSION_FORMAT_VERSION_V2
        || format_version == PRODUCT_SESSION_FORMAT_VERSION_V3
        || format_version == PRODUCT_SESSION_FORMAT_VERSION_V4
}

pub(crate) fn session_format_supports_project_terminology(format_version: u32) -> bool {
    format_version == PRODUCT_SESSION_FORMAT_VERSION_V4
}

const LEDGER_EVENT_KIND_CASE_RAISED: &str = "case_raised";
const LEDGER_EVENT_KIND_TERMINOLOGY_PROPOSAL_DECISION: &str = "terminology_proposal_decision";
const CASE_FAMILY_HUMAN: &str = "human";
const CASE_FAMILY_DETECTOR: &str = "detector";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PersistedSegmentV1 {
    pub(crate) index: u32,
    pub(crate) start_ms: u64,
    pub(crate) end_ms: u64,
    pub(crate) text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PersistedTranscriptV1 {
    pub(crate) revision_tag: String,
    pub(crate) segments: Vec<PersistedSegmentV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PersistedSessionTermV1 {
    pub(crate) canonical_term: String,
    pub(crate) aliases: Vec<String>,
    pub(crate) observed_error_forms: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PersistedDetectorIdentityV1 {
    pub(crate) id: String,
    pub(crate) version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PersistedDetectorConfigIdentityV1 {
    pub(crate) id: String,
    pub(crate) version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PersistedAlgorithmIdentityV1 {
    pub(crate) id: String,
    pub(crate) version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PersistedAnalysisSnapshotV1 {
    pub(crate) source_revision: String,
    pub(crate) session_terms_identity: String,
    pub(crate) detectors: Vec<PersistedDetectorIdentityV1>,
    pub(crate) detector_config: PersistedDetectorConfigIdentityV1,
    pub(crate) algorithm: PersistedAlgorithmIdentityV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PersistedAsciiLatinPhoneticRepresentationV1 {
    pub(crate) normalized_letters: String,
    pub(crate) primary_key: String,
    pub(crate) alternate_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PersistedPhoneticComparisonFactsV1 {
    pub(crate) edit_distance: usize,
    pub(crate) ratio_numerator: usize,
    pub(crate) ratio_denominator: usize,
    pub(crate) ratio_permille: usize,
    pub(crate) matched_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum PersistedEvidenceV1 {
    GlossaryAlias {
        canonical_term: String,
        aliases: Vec<String>,
        observed_error_forms: Vec<String>,
        matched_form: String,
    },
    ObservedErrorForm {
        canonical_term: String,
        aliases: Vec<String>,
        observed_error_forms: Vec<String>,
        matched_form: String,
    },
    PhoneticSimilarity {
        observed_surface: String,
        target_surface: String,
        target_kind: String,
        canonical_term: String,
        source_representation: PersistedAsciiLatinPhoneticRepresentationV1,
        target_representation: PersistedAsciiLatinPhoneticRepresentationV1,
        comparison: PersistedPhoneticComparisonFactsV1,
        detector_config_id: String,
        detector_config_version: String,
        algorithm_id: String,
        algorithm_version: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PersistedReviewCaseV1 {
    pub(crate) local_index: usize,
    pub(crate) detection_kind: DetectionKind,
    pub(crate) detector_id: String,
    pub(crate) detector_version: String,
    pub(crate) segment_position: usize,
    pub(crate) start_byte: usize,
    pub(crate) end_byte: usize,
    pub(crate) evidence: PersistedEvidenceV1,
    pub(crate) alternatives: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PersistedHumanRaisedCaseV1 {
    pub(crate) local_index: usize,
    pub(crate) segment_position: usize,
    pub(crate) start_byte: usize,
    pub(crate) end_byte: usize,
    pub(crate) observed_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision_kind", rename_all = "snake_case")]
pub(crate) enum PersistedCorrectionDecisionV1 {
    Reject,
    Defer,
    AcceptAlternative { alternative_index: usize },
    NeedsManualCorrection,
    ManualReplacement { replacement: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PersistedReviewLedgerEventV1 {
    case_local_index: usize,
    observed_revision: String,
    decision: PersistedCorrectionDecisionV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reuse_proposal_target_identity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    terminology_proposal_target_identity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    event_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    case_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    raised_segment_position: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    raised_start_byte: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    raised_end_byte: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    raised_observed_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PersistedReuseProposalTargetV1 {
    pub target_identity: String,
    pub reuse_analysis_snapshot: PersistedAnalysisSnapshotV1,
    pub project_memory_snapshot_identity: String,
    pub governance_boundary: usize,
    pub source_revision: String,
    pub segment_position: usize,
    pub start_byte: usize,
    pub end_byte: usize,
    pub observed_text: String,
    pub proposed_replacement: String,
    pub project_id: String,
    pub contributing_record_ids: Vec<usize>,
    pub detector_id: String,
    pub detector_version: String,
    pub detector_config_id: String,
    pub detector_config_version: String,
    pub algorithm_id: String,
    pub algorithm_version: String,
    pub reusable_influence_snapshot_identity: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PersistedProjectTerminologyProposalTargetV1 {
    pub target_identity: String,
    pub analysis_identity: String,
    pub analysis_snapshot: PersistedAnalysisSnapshotV1,
    pub project_memory_snapshot_identity: String,
    pub governance_boundary: usize,
    pub source_revision: String,
    pub segment_position: usize,
    pub start_byte: usize,
    pub end_byte: usize,
    pub observed_text: String,
    pub proposed_replacement: String,
    pub project_id: String,
    pub contributing_record_ids: Vec<usize>,
    pub detector_id: String,
    pub detector_version: String,
    pub detector_config_id: String,
    pub detector_config_version: String,
    pub algorithm_id: String,
    pub algorithm_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PersistedFrozenProjectReuseAnalysisV1 {
    pub freeze_identity: String,
    pub project_memory_snapshot_identity: String,
    pub governance_event_boundary: usize,
    pub reuse_analysis_snapshot: PersistedAnalysisSnapshotV1,
}

pub(crate) fn persist_reuse_proposal_target(
    target: &crate::reuse_proposal_target::ReuseProposalTarget,
) -> PersistedReuseProposalTargetV1 {
    PersistedReuseProposalTargetV1 {
        target_identity: target.identity().to_tagged_string(),
        reuse_analysis_snapshot: persist_analysis_snapshot(target.reuse_analysis_snapshot()),
        project_memory_snapshot_identity: target
            .project_memory_snapshot_identity()
            .to_tagged_string(),
        governance_boundary: target.governance_boundary(),
        source_revision: target.occurrence().source_revision.to_tagged_string(),
        segment_position: target.occurrence().segment_position,
        start_byte: target.occurrence().start_byte,
        end_byte: target.occurrence().end_byte,
        observed_text: target.occurrence().observed_text.clone(),
        proposed_replacement: target.proposed_replacement().to_owned(),
        project_id: target.project_id().as_str().to_owned(),
        contributing_record_ids: target.contributing_record_ids().to_vec(),
        detector_id: target.detector_id().to_owned(),
        detector_version: target.detector_version().to_owned(),
        detector_config_id: target.detector_config_id().to_owned(),
        detector_config_version: target.detector_config_version().to_owned(),
        algorithm_id: target.algorithm_id().to_owned(),
        algorithm_version: target.algorithm_version().to_owned(),
        reusable_influence_snapshot_identity: target
            .reusable_influence_snapshot_identity()
            .to_tagged_string(),
    }
}

pub(crate) fn restore_reuse_proposal_target(
    persisted: &PersistedReuseProposalTargetV1,
) -> Result<crate::reuse_proposal_target::ReuseProposalTarget, SessionPersistenceError> {
    let reuse_analysis_snapshot =
        restore_analysis_snapshot_from_persisted(&persisted.reuse_analysis_snapshot)?;
    let project_memory_snapshot_identity =
        crate::project_memory::ProjectMemorySnapshotIdentity::from_tagged_string(
            &persisted.project_memory_snapshot_identity,
        )
        .ok_or_else(|| {
            SessionPersistenceError::CanonicalMismatch(
                "project memory snapshot identity".to_owned(),
            )
        })?;
    let source_revision = parse_revision_tag_for_canonical(&persisted.source_revision)?;
    let project_id = crate::reuse_primitives::ProjectScopeId::new(persisted.project_id.clone())
        .map_err(|_| SessionPersistenceError::CanonicalMismatch("project id".to_owned()))?;
    let reusable_influence_snapshot_identity =
        crate::session_persistence::reuse_canonical::restore_reusable_snapshot_identity(
            &persisted.reusable_influence_snapshot_identity,
        )?;
    let target = crate::reuse_proposal_target::ReuseProposalTarget::from_derivation_inputs(
        reuse_analysis_snapshot,
        project_memory_snapshot_identity,
        persisted.governance_boundary,
        crate::reuse_proposal_target::ReuseProposalOccurrence {
            source_revision,
            segment_position: persisted.segment_position,
            start_byte: persisted.start_byte,
            end_byte: persisted.end_byte,
            observed_text: persisted.observed_text.clone(),
        },
        persisted.proposed_replacement.clone(),
        project_id,
        persisted.contributing_record_ids.clone(),
        persisted.detector_id.clone(),
        persisted.detector_version.clone(),
        persisted.detector_config_id.clone(),
        persisted.detector_config_version.clone(),
        persisted.algorithm_id.clone(),
        persisted.algorithm_version.clone(),
        reusable_influence_snapshot_identity,
    );
    if target.identity().to_tagged_string() != persisted.target_identity
        || !target.verify_identity()
    {
        return Err(SessionPersistenceError::CanonicalMismatch(
            "reuse proposal target identity mismatch".to_owned(),
        ));
    }
    Ok(target)
}

pub(crate) fn persist_project_terminology_proposal_target(
    target: &crate::project_terminology::ProjectTerminologyProposalTarget,
) -> PersistedProjectTerminologyProposalTargetV1 {
    PersistedProjectTerminologyProposalTargetV1 {
        target_identity: target.identity().to_tagged_string(),
        analysis_identity: target.analysis_identity().to_tagged_string(),
        analysis_snapshot: persist_analysis_snapshot(target.analysis_snapshot()),
        project_memory_snapshot_identity: target
            .project_memory_snapshot_identity()
            .to_tagged_string(),
        governance_boundary: target.governance_boundary(),
        source_revision: target.occurrence().source_revision.to_tagged_string(),
        segment_position: target.occurrence().segment_position,
        start_byte: target.occurrence().start_byte,
        end_byte: target.occurrence().end_byte,
        observed_text: target.occurrence().observed_text.clone(),
        proposed_replacement: target.proposed_replacement().to_owned(),
        project_id: target.project_id().as_str().to_owned(),
        contributing_record_ids: target.contributing_record_ids().to_vec(),
        detector_id: target.detector_id().to_owned(),
        detector_version: target.detector_version().to_owned(),
        detector_config_id: target.detector_config_id().to_owned(),
        detector_config_version: target.detector_config_version().to_owned(),
        algorithm_id: target.algorithm_id().to_owned(),
        algorithm_version: target.algorithm_version().to_owned(),
    }
}

pub(crate) fn restore_project_terminology_proposal_target(
    persisted: &PersistedProjectTerminologyProposalTargetV1,
) -> Result<crate::project_terminology::ProjectTerminologyProposalTarget, SessionPersistenceError> {
    let analysis_snapshot = restore_analysis_snapshot_from_persisted(&persisted.analysis_snapshot)?;
    let analysis_identity =
        crate::project_terminology::ProjectDerivedTerminologyAnalysisIdentity::from_tagged_string(
            &persisted.analysis_identity,
        )
        .ok_or_else(|| {
            SessionPersistenceError::CanonicalMismatch(
                "project derived terminology analysis identity".to_owned(),
            )
        })?;
    let project_memory_snapshot_identity =
        crate::project_memory::ProjectMemorySnapshotIdentity::from_tagged_string(
            &persisted.project_memory_snapshot_identity,
        )
        .ok_or_else(|| {
            SessionPersistenceError::CanonicalMismatch(
                "project memory snapshot identity".to_owned(),
            )
        })?;
    let source_revision = parse_revision_tag_for_canonical(&persisted.source_revision)?;
    let project_id = crate::reuse_primitives::ProjectScopeId::new(persisted.project_id.clone())
        .map_err(|_| SessionPersistenceError::CanonicalMismatch("project id".to_owned()))?;
    let target =
        crate::project_terminology::ProjectTerminologyProposalTarget::from_derivation_inputs(
            analysis_identity,
            analysis_snapshot,
            project_memory_snapshot_identity,
            persisted.governance_boundary,
            crate::project_terminology::ProjectTerminologyOccurrence {
                source_revision,
                segment_position: persisted.segment_position,
                start_byte: persisted.start_byte,
                end_byte: persisted.end_byte,
                observed_text: persisted.observed_text.clone(),
            },
            persisted.proposed_replacement.clone(),
            project_id,
            persisted.contributing_record_ids.clone(),
            persisted.detector_id.clone(),
            persisted.detector_version.clone(),
            persisted.detector_config_id.clone(),
            persisted.detector_config_version.clone(),
            persisted.algorithm_id.clone(),
            persisted.algorithm_version.clone(),
        );
    if target.identity().to_tagged_string() != persisted.target_identity
        || target.analysis_identity().to_tagged_string() != persisted.analysis_identity
    {
        return Err(SessionPersistenceError::CanonicalMismatch(
            "project terminology proposal target identity mismatch".to_owned(),
        ));
    }
    Ok(target)
}

pub(crate) fn persist_frozen_project_reuse(
    frozen: &crate::reuse_proposal_target::FrozenProjectReuseAnalysis,
) -> PersistedFrozenProjectReuseAnalysisV1 {
    PersistedFrozenProjectReuseAnalysisV1 {
        freeze_identity: frozen.freeze_identity.to_tagged_string(),
        project_memory_snapshot_identity: frozen
            .project_memory_snapshot_identity
            .to_tagged_string(),
        governance_event_boundary: frozen.governance_event_boundary,
        reuse_analysis_snapshot: persist_analysis_snapshot(frozen.reuse_analysis_snapshot),
    }
}

pub(crate) fn restore_frozen_project_reuse(
    persisted: &PersistedFrozenProjectReuseAnalysisV1,
) -> Result<crate::reuse_proposal_target::FrozenProjectReuseAnalysis, SessionPersistenceError> {
    let project_memory_snapshot_identity =
        crate::project_memory::ProjectMemorySnapshotIdentity::from_tagged_string(
            &persisted.project_memory_snapshot_identity,
        )
        .ok_or_else(|| {
            SessionPersistenceError::CanonicalMismatch(
                "frozen project memory snapshot identity".to_owned(),
            )
        })?;
    let reuse_analysis_snapshot =
        restore_analysis_snapshot_from_persisted(&persisted.reuse_analysis_snapshot)?;
    let frozen = crate::reuse_proposal_target::FrozenProjectReuseAnalysis::new(
        project_memory_snapshot_identity,
        persisted.governance_event_boundary,
        reuse_analysis_snapshot,
    );
    if frozen.freeze_identity.to_tagged_string() != persisted.freeze_identity {
        return Err(SessionPersistenceError::CanonicalMismatch(
            "frozen project reuse analysis identity mismatch".to_owned(),
        ));
    }
    Ok(frozen)
}

pub(crate) fn persist_transcript(transcript: &Transcript) -> PersistedTranscriptV1 {
    PersistedTranscriptV1 {
        revision_tag: transcript.revision_id().to_tagged_string(),
        segments: transcript
            .segments()
            .iter()
            .map(|segment| PersistedSegmentV1 {
                index: segment.index(),
                start_ms: segment.start_ms(),
                end_ms: segment.end_ms(),
                text: segment.text().to_owned(),
            })
            .collect(),
    }
}

pub(crate) fn restore_transcript(persisted: &PersistedTranscriptV1) -> Transcript {
    let segments = persisted
        .segments
        .iter()
        .map(|segment| Segment {
            index: segment.index,
            start_ms: segment.start_ms,
            end_ms: segment.end_ms,
            text: segment.text.clone(),
        })
        .collect();
    Transcript::from_segments(segments)
}

pub(crate) fn persist_session_terms(entries: &[SessionTermEntry]) -> Vec<PersistedSessionTermV1> {
    entries
        .iter()
        .map(|entry| PersistedSessionTermV1 {
            canonical_term: entry.canonical_term.clone(),
            aliases: entry.aliases.clone(),
            observed_error_forms: entry.observed_error_forms.clone(),
        })
        .collect()
}

pub(crate) fn restore_session_terms(persisted: &[PersistedSessionTermV1]) -> Vec<SessionTermEntry> {
    persisted
        .iter()
        .map(|entry| {
            SessionTermEntry::new(
                entry.canonical_term.clone(),
                entry.aliases.clone(),
                entry.observed_error_forms.clone(),
            )
        })
        .collect()
}

pub(crate) fn persist_analysis_snapshot(snapshot: AnalysisSnapshot) -> PersistedAnalysisSnapshotV1 {
    let configuration = snapshot.configuration();
    PersistedAnalysisSnapshotV1 {
        source_revision: snapshot.source_revision().to_tagged_string(),
        session_terms_identity: snapshot.session_terms().to_tagged_string(),
        detectors: configuration
            .detector_set()
            .detectors()
            .iter()
            .map(|detector| PersistedDetectorIdentityV1 {
                id: detector.id().to_owned(),
                version: detector.version().to_owned(),
            })
            .collect(),
        detector_config: PersistedDetectorConfigIdentityV1 {
            id: configuration.detector_config().id().to_owned(),
            version: configuration.detector_config().version().to_owned(),
        },
        algorithm: PersistedAlgorithmIdentityV1 {
            id: configuration.algorithm().id().to_owned(),
            version: configuration.algorithm().version().to_owned(),
        },
    }
}

pub(crate) fn verify_analysis_snapshot(
    persisted: &PersistedAnalysisSnapshotV1,
    actual: AnalysisSnapshot,
) -> Result<(), SessionPersistenceError> {
    if persisted.source_revision != actual.source_revision().to_tagged_string()
        || persisted.session_terms_identity != actual.session_terms().to_tagged_string()
    {
        return Err(SessionPersistenceError::CanonicalMismatch(
            "analysis snapshot identity mismatch".to_owned(),
        ));
    }
    let configuration = actual.configuration();
    let actual_detectors = configuration.detector_set().detectors();
    if persisted.detectors.len() != actual_detectors.len() {
        return Err(SessionPersistenceError::CanonicalMismatch(
            "analysis detector set length mismatch".to_owned(),
        ));
    }
    for (persisted_detector, actual_detector) in persisted.detectors.iter().zip(actual_detectors) {
        if persisted_detector.id != actual_detector.id()
            || persisted_detector.version != actual_detector.version()
        {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "analysis detector identity mismatch".to_owned(),
            ));
        }
    }
    if persisted.detector_config.id != configuration.detector_config().id()
        || persisted.detector_config.version != configuration.detector_config().version()
    {
        return Err(SessionPersistenceError::CanonicalMismatch(
            "analysis detector config identity mismatch".to_owned(),
        ));
    }
    if persisted.algorithm.id != configuration.algorithm().id()
        || persisted.algorithm.version != configuration.algorithm().version()
    {
        return Err(SessionPersistenceError::CanonicalMismatch(
            "analysis algorithm identity mismatch".to_owned(),
        ));
    }
    Ok(())
}

pub(crate) fn persist_review_case(
    review_case: &ReviewCase,
) -> Result<PersistedReviewCaseV1, SessionPersistenceError> {
    let span = review_case.candidate_span();
    let anchor = span.anchor();
    let evidence = match span.evidence() {
        Evidence::GlossaryAlias(value) => PersistedEvidenceV1::GlossaryAlias {
            canonical_term: value.entry.canonical_term.clone(),
            aliases: value.entry.aliases.clone(),
            observed_error_forms: value.entry.observed_error_forms.clone(),
            matched_form: value.matched_form.clone(),
        },
        Evidence::ObservedErrorForm(value) => PersistedEvidenceV1::ObservedErrorForm {
            canonical_term: value.entry.canonical_term.clone(),
            aliases: value.entry.aliases.clone(),
            observed_error_forms: value.entry.observed_error_forms.clone(),
            matched_form: value.matched_form.clone(),
        },
        Evidence::PhoneticSimilarity(value) => PersistedEvidenceV1::PhoneticSimilarity {
            observed_surface: value.observed_surface.clone(),
            target_surface: value.target_surface.clone(),
            target_kind: persist_phonetic_target_kind(value.target_kind),
            canonical_term: value.canonical_term.clone(),
            source_representation: persist_phonetic_representation(&value.source_representation),
            target_representation: persist_phonetic_representation(&value.target_representation),
            comparison: persist_phonetic_comparison(&value.comparison),
            detector_config_id: value.detector_config.id().to_owned(),
            detector_config_version: value.detector_config.version().to_owned(),
            algorithm_id: value.algorithm.id().to_owned(),
            algorithm_version: value.algorithm.version().to_owned(),
        },
        Evidence::ReusableExactObservedForm(_) => {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "unsupported review case evidence variant for product persistence v1".to_owned(),
            ));
        }
    };

    Ok(PersistedReviewCaseV1 {
        local_index: review_case.id().local_index(),
        detection_kind: span.kind(),
        detector_id: span.provenance().detector_id().to_owned(),
        detector_version: span.provenance().detector_version().to_owned(),
        segment_position: anchor.segment_position(),
        start_byte: anchor.start_byte(),
        end_byte: anchor.end_byte(),
        evidence,
        alternatives: span
            .alternatives()
            .iter()
            .map(|alternative| alternative.replacement_text().to_owned())
            .collect(),
    })
}

pub(crate) fn restore_review_case(
    persisted: &PersistedReviewCaseV1,
    revision: TranscriptRevisionId,
) -> Result<ReviewCase, SessionPersistenceError> {
    let anchor = crate::anchor::SourceAnchor {
        revision,
        segment_position: persisted.segment_position,
        start_byte: persisted.start_byte,
        end_byte: persisted.end_byte,
    };
    let evidence = match &persisted.evidence {
        PersistedEvidenceV1::GlossaryAlias {
            canonical_term,
            aliases,
            observed_error_forms,
            matched_form,
        } => Evidence::GlossaryAlias(GlossaryAliasEvidence {
            entry: SessionTermEntry::new(
                canonical_term.clone(),
                aliases.clone(),
                observed_error_forms.clone(),
            ),
            matched_form: matched_form.clone(),
        }),
        PersistedEvidenceV1::ObservedErrorForm {
            canonical_term,
            aliases,
            observed_error_forms,
            matched_form,
        } => Evidence::ObservedErrorForm(ObservedErrorFormEvidence {
            entry: SessionTermEntry::new(
                canonical_term.clone(),
                aliases.clone(),
                observed_error_forms.clone(),
            ),
            matched_form: matched_form.clone(),
        }),
        PersistedEvidenceV1::PhoneticSimilarity {
            observed_surface,
            target_surface,
            target_kind,
            canonical_term,
            source_representation,
            target_representation,
            comparison,
            detector_config_id,
            detector_config_version,
            algorithm_id,
            algorithm_version,
        } => {
            let detector_config =
                detector_config_identity_from_parts(detector_config_id, detector_config_version)
                    .ok_or_else(|| {
                        SessionPersistenceError::CanonicalMismatch(
                            "phonetic detector config identity".to_owned(),
                        )
                    })?;
            let algorithm = algorithm_identity_from_parts(algorithm_id, algorithm_version)
                .ok_or_else(|| {
                    SessionPersistenceError::CanonicalMismatch(
                        "phonetic algorithm identity".to_owned(),
                    )
                })?;
            Evidence::PhoneticSimilarity(PhoneticSimilarityEvidence {
                observed_surface: observed_surface.clone(),
                target_surface: target_surface.clone(),
                target_kind: restore_phonetic_target_kind(target_kind)?,
                canonical_term: canonical_term.clone(),
                source_representation: restore_phonetic_representation(source_representation)?,
                target_representation: restore_phonetic_representation(target_representation)?,
                comparison: restore_phonetic_comparison(comparison)?,
                detector_config,
                algorithm,
            })
        }
    };
    let alternatives = persisted
        .alternatives
        .iter()
        .map(|text| CandidateAlternative::new(text.clone()))
        .collect();
    let span = CandidateSpan::new(
        persisted.detection_kind,
        DetectorProvenance::new(&persisted.detector_id, &persisted.detector_version),
        anchor,
        evidence,
        alternatives,
    );
    Ok(ReviewCase::detector_raised(
        ReviewCaseId::local(persisted.local_index),
        span,
    ))
}

pub(crate) fn persist_ledger_event(
    event: &ReviewLedgerEvent,
) -> Result<PersistedReviewLedgerEventV1, SessionPersistenceError> {
    match event {
        ReviewLedgerEvent::DecisionRecorded {
            case_id,
            observed_revision,
            decision,
        } => Ok(PersistedReviewLedgerEventV1 {
            case_local_index: case_id.local_index(),
            observed_revision: observed_revision.to_tagged_string(),
            decision: persist_correction_decision(decision),
            reuse_proposal_target_identity: None,
            terminology_proposal_target_identity: None,
            event_kind: None,
            case_family: persist_case_family(*case_id),
            raised_segment_position: None,
            raised_start_byte: None,
            raised_end_byte: None,
            raised_observed_text: None,
        }),
        ReviewLedgerEvent::ReuseProposalDecisionRecorded {
            target_identity,
            observed_revision,
            decision,
        } => Ok(PersistedReviewLedgerEventV1 {
            case_local_index: 0,
            observed_revision: observed_revision.to_tagged_string(),
            decision: persist_correction_decision(decision),
            reuse_proposal_target_identity: Some(target_identity.to_tagged_string()),
            terminology_proposal_target_identity: None,
            event_kind: None,
            case_family: None,
            raised_segment_position: None,
            raised_start_byte: None,
            raised_end_byte: None,
            raised_observed_text: None,
        }),
        ReviewLedgerEvent::CaseRaised {
            case_id,
            observed_revision,
            segment_position,
            start_byte,
            end_byte,
            observed_text,
        } => {
            if !case_id.is_human_raised() {
                return Err(SessionPersistenceError::CanonicalMismatch(
                    "CaseRaised requires a HumanRaised case id".to_owned(),
                ));
            }
            Ok(PersistedReviewLedgerEventV1 {
                case_local_index: case_id.local_index(),
                observed_revision: observed_revision.to_tagged_string(),
                decision: PersistedCorrectionDecisionV1::Reject,
                reuse_proposal_target_identity: None,
                terminology_proposal_target_identity: None,
                event_kind: Some(LEDGER_EVENT_KIND_CASE_RAISED.to_owned()),
                case_family: Some(CASE_FAMILY_HUMAN.to_owned()),
                raised_segment_position: Some(*segment_position),
                raised_start_byte: Some(*start_byte),
                raised_end_byte: Some(*end_byte),
                raised_observed_text: Some(observed_text.clone()),
            })
        }
        ReviewLedgerEvent::TerminologyProposalDecisionRecorded {
            target_identity,
            observed_revision,
            decision,
        } => Ok(PersistedReviewLedgerEventV1 {
            case_local_index: 0,
            observed_revision: observed_revision.to_tagged_string(),
            decision: persist_correction_decision(decision),
            reuse_proposal_target_identity: None,
            terminology_proposal_target_identity: Some(target_identity.to_tagged_string()),
            event_kind: Some(LEDGER_EVENT_KIND_TERMINOLOGY_PROPOSAL_DECISION.to_owned()),
            case_family: None,
            raised_segment_position: None,
            raised_start_byte: None,
            raised_end_byte: None,
            raised_observed_text: None,
        }),
    }
}

fn persist_case_family(case_id: ReviewCaseId) -> Option<String> {
    if case_id.is_human_raised() {
        Some(CASE_FAMILY_HUMAN.to_owned())
    } else {
        None
    }
}

fn restore_case_id(
    local_index: usize,
    family: Option<&str>,
) -> Result<ReviewCaseId, SessionPersistenceError> {
    match family {
        None | Some(CASE_FAMILY_DETECTOR) => Ok(ReviewCaseId::local(local_index)),
        Some(CASE_FAMILY_HUMAN) => Ok(ReviewCaseId::human(local_index)),
        Some(_) => Err(SessionPersistenceError::CanonicalMismatch(
            "unknown review case family".to_owned(),
        )),
    }
}

pub(crate) fn persist_human_raised_case(
    review_case: &ReviewCase,
) -> Result<PersistedHumanRaisedCaseV1, SessionPersistenceError> {
    let selection = review_case.as_human_selection().ok_or_else(|| {
        SessionPersistenceError::CanonicalMismatch(
            "human-raised capture requires a HumanRaised review case".to_owned(),
        )
    })?;
    let anchor = selection.anchor();
    Ok(PersistedHumanRaisedCaseV1 {
        local_index: review_case.id().local_index(),
        segment_position: anchor.segment_position(),
        start_byte: anchor.start_byte(),
        end_byte: anchor.end_byte(),
        observed_text: selection.observed_text().to_owned(),
    })
}

/// Restores human-raised cases in persisted `local_index` order and refuses gaps, reordering, or
/// observed text that no longer matches the current source revision.
pub(crate) fn restore_human_raised_cases(
    persisted: &[PersistedHumanRaisedCaseV1],
    transcript: &Transcript,
) -> Result<Vec<ReviewCase>, SessionPersistenceError> {
    let revision = transcript.revision_id();
    let mut restored = Vec::with_capacity(persisted.len());
    for (position, case) in persisted.iter().enumerate() {
        if case.local_index != position {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "human-raised case local index is not dense and ordered".to_owned(),
            ));
        }
        let anchor = crate::anchor::SourceAnchor {
            revision,
            segment_position: case.segment_position,
            start_byte: case.start_byte,
            end_byte: case.end_byte,
        };
        let observed = transcript.resolve(&anchor).ok_or_else(|| {
            SessionPersistenceError::CanonicalMismatch(
                "human-raised anchor does not resolve in the persisted source".to_owned(),
            )
        })?;
        if observed != case.observed_text {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "human-raised observed text mismatch".to_owned(),
            ));
        }
        restored.push(ReviewCase::human_raised(
            ReviewCaseId::human(position),
            crate::review::HumanSelectedSpan::new(anchor, case.observed_text.clone()),
        ));
    }
    Ok(restored)
}

pub(crate) fn restore_ledger_event(
    persisted: &PersistedReviewLedgerEventV1,
    revision: TranscriptRevisionId,
) -> Result<ReviewLedgerEvent, SessionPersistenceError> {
    if persisted.observed_revision != revision.to_tagged_string() {
        return Err(SessionPersistenceError::CanonicalMismatch(
            "ledger event revision mismatch".to_owned(),
        ));
    }
    if persisted.event_kind.as_deref() == Some(LEDGER_EVENT_KIND_CASE_RAISED) {
        if persisted.reuse_proposal_target_identity.is_some()
            || persisted.terminology_proposal_target_identity.is_some()
        {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "CaseRaised must not carry a proposal identity".to_owned(),
            ));
        }
        let case_id =
            restore_case_id(persisted.case_local_index, persisted.case_family.as_deref())?;
        if !case_id.is_human_raised() {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "CaseRaised requires a HumanRaised case family".to_owned(),
            ));
        }
        let (Some(segment_position), Some(start_byte), Some(end_byte), Some(observed_text)) = (
            persisted.raised_segment_position,
            persisted.raised_start_byte,
            persisted.raised_end_byte,
            persisted.raised_observed_text.as_ref(),
        ) else {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "CaseRaised missing span fields".to_owned(),
            ));
        };
        return Ok(ReviewLedgerEvent::CaseRaised {
            case_id,
            observed_revision: revision,
            segment_position,
            start_byte,
            end_byte,
            observed_text: observed_text.clone(),
        });
    }
    if persisted.event_kind.as_deref() == Some(LEDGER_EVENT_KIND_TERMINOLOGY_PROPOSAL_DECISION) {
        if persisted.reuse_proposal_target_identity.is_some() {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "terminology decision must not carry a reuse proposal identity".to_owned(),
            ));
        }
        let Some(tagged) = &persisted.terminology_proposal_target_identity else {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "terminology decision missing target identity".to_owned(),
            ));
        };
        let target_identity =
            crate::project_terminology::ProjectTerminologyProposalTargetIdentity::from_tagged_string(
                tagged,
            )
            .ok_or_else(|| {
                SessionPersistenceError::CanonicalMismatch(
                    "terminology proposal target identity".to_owned(),
                )
            })?;
        let decision = restore_correction_decision(&persisted.decision)?;
        return Ok(ReviewLedgerEvent::TerminologyProposalDecisionRecorded {
            target_identity,
            observed_revision: revision,
            decision,
        });
    }
    if persisted.event_kind.is_some() {
        return Err(SessionPersistenceError::CanonicalMismatch(
            "unknown ledger event kind".to_owned(),
        ));
    }
    if persisted.terminology_proposal_target_identity.is_some() {
        return Err(SessionPersistenceError::CanonicalMismatch(
            "terminology proposal identity requires terminology_proposal_decision event kind"
                .to_owned(),
        ));
    }
    let decision = restore_correction_decision(&persisted.decision)?;
    if let Some(tagged) = &persisted.reuse_proposal_target_identity {
        let target_identity =
            crate::reuse_proposal_target::ReuseProposalTargetIdentity::from_tagged_string(tagged)
                .ok_or_else(|| {
                SessionPersistenceError::CanonicalMismatch(
                    "reuse proposal target identity".to_owned(),
                )
            })?;
        return Ok(ReviewLedgerEvent::ReuseProposalDecisionRecorded {
            target_identity,
            observed_revision: revision,
            decision,
        });
    }
    Ok(ReviewLedgerEvent::DecisionRecorded {
        case_id: restore_case_id(persisted.case_local_index, persisted.case_family.as_deref())?,
        observed_revision: revision,
        decision,
    })
}

fn persist_correction_decision(decision: &CorrectionDecision) -> PersistedCorrectionDecisionV1 {
    match decision {
        CorrectionDecision::Reject => PersistedCorrectionDecisionV1::Reject,
        CorrectionDecision::Defer => PersistedCorrectionDecisionV1::Defer,
        CorrectionDecision::AcceptAlternative { alternative_index } => {
            PersistedCorrectionDecisionV1::AcceptAlternative {
                alternative_index: *alternative_index,
            }
        }
        CorrectionDecision::NeedsManualCorrection => {
            PersistedCorrectionDecisionV1::NeedsManualCorrection
        }
        CorrectionDecision::ManualReplacement { replacement } => {
            PersistedCorrectionDecisionV1::ManualReplacement {
                replacement: replacement.as_str().to_owned(),
            }
        }
    }
}

fn restore_correction_decision(
    persisted: &PersistedCorrectionDecisionV1,
) -> Result<CorrectionDecision, SessionPersistenceError> {
    Ok(match persisted {
        PersistedCorrectionDecisionV1::Reject => CorrectionDecision::Reject,
        PersistedCorrectionDecisionV1::Defer => CorrectionDecision::Defer,
        PersistedCorrectionDecisionV1::AcceptAlternative { alternative_index } => {
            CorrectionDecision::AcceptAlternative {
                alternative_index: *alternative_index,
            }
        }
        PersistedCorrectionDecisionV1::NeedsManualCorrection => {
            CorrectionDecision::NeedsManualCorrection
        }
        PersistedCorrectionDecisionV1::ManualReplacement { replacement } => {
            CorrectionDecision::ManualReplacement {
                replacement: crate::review::ManualReplacementText::from_persisted_storage(
                    replacement.clone(),
                ),
            }
        }
    })
}

pub(crate) fn verify_review_cases(
    persisted: &[PersistedReviewCaseV1],
    canonical_run: &CanonicalTermReviewRun,
    revision: TranscriptRevisionId,
) -> Result<Vec<ReviewCase>, SessionPersistenceError> {
    let restored = persisted
        .iter()
        .map(|case| restore_review_case(case, revision))
        .collect::<Result<Vec<_>, _>>()?;
    if restored != canonical_run.review_cases() {
        return Err(SessionPersistenceError::ReviewCaseVerificationFailed);
    }
    Ok(restored)
}

pub(crate) fn snapshot_identity(snapshot: AnalysisSnapshot) -> String {
    crate::analysis::analysis_snapshot_identity_tag(snapshot)
}

pub(crate) fn capture_from_session(
    session: &ApplicationReviewSession,
) -> Result<SessionCanonicalCapture, SessionPersistenceError> {
    let canonical_snapshot = session.canonical_run().analysis_run().snapshot();
    let project_scope = session
        .reuse_state()
        .project_scope()
        .map(persist_project_scope)
        .unwrap_or(PersistedProjectScopeV1 {
            stable_id: String::new(),
            display_name: String::new(),
        });
    let reuse_governance_events = session
        .reuse_state()
        .governance_events()
        .iter()
        .map(persist_governance_event)
        .collect();
    let reuse_governance_head = session.reuse_state().governance_events().len();
    let reuse_enabled_bindings = if let Some(run) = session.reuse_enabled_run() {
        let parts = session.reuse_parts();
        let snapshot = reusable_influence_snapshot_for_parts(parts, session.reuse_state())
            .map_err(|error| SessionPersistenceError::CanonicalMismatch(format!("{error:?}")))?;
        vec![persist_reuse_enabled_binding(
            1,
            run.analysis_run().snapshot(),
            snapshot.identity(),
            session.reuse_state().governance_events().len(),
        )]
    } else {
        Vec::new()
    };
    let active_analysis_selection_identity = if let Some(run) = session.reuse_enabled_run() {
        let parts = session.reuse_parts();
        let snapshot = reusable_influence_snapshot_for_parts(parts, session.reuse_state())
            .map_err(|error| SessionPersistenceError::CanonicalMismatch(format!("{error:?}")))?;
        active_analysis_selection_identity_for_reuse_enabled(
            run.analysis_run().snapshot(),
            snapshot.identity(),
            session.reuse_state().governance_events().len(),
        )
    } else {
        snapshot_identity(canonical_snapshot)
    };
    Ok(SessionCanonicalCapture {
        transcript: persist_transcript(session.source()),
        session_terms: persist_session_terms(session.session_terms()),
        analysis_snapshot: persist_analysis_snapshot(canonical_snapshot),
        active_analysis_selection_identity,
        review_cases: session
            .canonical_run()
            .review_cases()
            .iter()
            .map(persist_review_case)
            .collect::<Result<Vec<_>, _>>()?,
        material_use_basis: match session.material_use_declaration().basis() {
            crate::application_service::DeclaredApplicationMaterialUseBasis::SelfOwned => {
                "self_owned".to_owned()
            }
            crate::application_service::DeclaredApplicationMaterialUseBasis::ExplicitPermission => {
                "explicit_permission".to_owned()
            }
        },
        authority_role: match session.session_authority().role() {
            crate::application_service::DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator => {
                "declared_local_owner_operator".to_owned()
            }
            crate::application_service::DeclaredSessionOperatorRole::DeclaredAuthorizedHumanReviewer => {
                "declared_authorized_human_reviewer".to_owned()
            }
        },
        authority_display_label: session.session_authority().display_label().to_owned(),
        review_ledger_head: session.review_ledger_head(),
        ledger_events: session
            .review_ledger()
            .events()
            .iter()
            .map(persist_ledger_event)
            .collect::<Result<Vec<_>, _>>()?,
        human_raised_cases: session
            .human_raised_cases()
            .iter()
            .map(persist_human_raised_case)
            .collect::<Result<Vec<_>, _>>()?,
        project_scope,
        reuse_governance_events,
        reuse_governance_head,
        reuse_enabled_bindings,
        format_version: PRODUCT_SESSION_FORMAT_VERSION,
        bound_project_id: None,
        frozen_project_reuse: None,
        reuse_proposal_targets: Vec::new(),
        terminology_proposal_targets: Vec::new(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionCanonicalCapture {
    pub transcript: PersistedTranscriptV1,
    pub session_terms: Vec<PersistedSessionTermV1>,
    pub analysis_snapshot: PersistedAnalysisSnapshotV1,
    pub active_analysis_selection_identity: String,
    pub review_cases: Vec<PersistedReviewCaseV1>,
    pub material_use_basis: String,
    pub authority_role: String,
    pub authority_display_label: String,
    pub review_ledger_head: usize,
    pub ledger_events: Vec<PersistedReviewLedgerEventV1>,
    pub human_raised_cases: Vec<PersistedHumanRaisedCaseV1>,
    pub project_scope: PersistedProjectScopeV1,
    pub reuse_governance_events: Vec<PersistedReuseGovernanceEventV1>,
    pub reuse_governance_head: usize,
    pub reuse_enabled_bindings: Vec<PersistedReuseEnabledBindingV1>,
    pub format_version: u32,
    pub bound_project_id: Option<String>,
    pub frozen_project_reuse: Option<PersistedFrozenProjectReuseAnalysisV1>,
    pub reuse_proposal_targets: Vec<PersistedReuseProposalTargetV1>,
    pub terminology_proposal_targets: Vec<PersistedProjectTerminologyProposalTargetV1>,
}

fn persist_phonetic_target_kind(kind: PhoneticTargetKind) -> String {
    match kind {
        PhoneticTargetKind::CanonicalTerm => "canonical_term".to_owned(),
        PhoneticTargetKind::Alias => "alias".to_owned(),
    }
}

fn restore_phonetic_target_kind(kind: &str) -> Result<PhoneticTargetKind, SessionPersistenceError> {
    match kind {
        "canonical_term" => Ok(PhoneticTargetKind::CanonicalTerm),
        "alias" => Ok(PhoneticTargetKind::Alias),
        _ => Err(SessionPersistenceError::CanonicalMismatch(
            "phonetic target kind".to_owned(),
        )),
    }
}

fn persist_phonetic_representation(
    representation: &AsciiLatinPhoneticRepresentation,
) -> PersistedAsciiLatinPhoneticRepresentationV1 {
    PersistedAsciiLatinPhoneticRepresentationV1 {
        normalized_letters: representation.normalized_letters.clone(),
        primary_key: representation.primary_key.clone(),
        alternate_key: representation.alternate_key.clone(),
    }
}

fn restore_phonetic_representation(
    persisted: &PersistedAsciiLatinPhoneticRepresentationV1,
) -> Result<AsciiLatinPhoneticRepresentation, SessionPersistenceError> {
    Ok(AsciiLatinPhoneticRepresentation {
        normalized_letters: persisted.normalized_letters.clone(),
        primary_key: persisted.primary_key.clone(),
        alternate_key: persisted.alternate_key.clone(),
    })
}

fn persist_phonetic_comparison(
    comparison: &PhoneticComparisonFacts,
) -> PersistedPhoneticComparisonFactsV1 {
    PersistedPhoneticComparisonFactsV1 {
        edit_distance: comparison.edit_distance,
        ratio_numerator: comparison.ratio_numerator,
        ratio_denominator: comparison.ratio_denominator,
        ratio_permille: comparison.ratio_permille,
        matched_key: comparison.matched_key.clone(),
    }
}

fn restore_phonetic_comparison(
    persisted: &PersistedPhoneticComparisonFactsV1,
) -> Result<PhoneticComparisonFacts, SessionPersistenceError> {
    Ok(PhoneticComparisonFacts {
        edit_distance: persisted.edit_distance,
        ratio_numerator: persisted.ratio_numerator,
        ratio_denominator: persisted.ratio_denominator,
        ratio_permille: persisted.ratio_permille,
        matched_key: persisted.matched_key.clone(),
    })
}

pub(crate) fn restore_analysis_snapshot_from_persisted(
    persisted: &PersistedAnalysisSnapshotV1,
) -> Result<AnalysisSnapshot, SessionPersistenceError> {
    let source_revision = parse_revision_tag_for_canonical(&persisted.source_revision)?;
    let session_terms = SessionTermsIdentity::from_tagged_string(&persisted.session_terms_identity)
        .ok_or_else(|| {
            SessionPersistenceError::CanonicalMismatch("session terms identity".to_owned())
        })?;
    let configuration = restore_configuration_from_persisted(persisted)?;
    Ok(AnalysisSnapshot::from_components(
        source_revision,
        session_terms,
        configuration,
    ))
}

fn restore_configuration_from_persisted(
    persisted: &PersistedAnalysisSnapshotV1,
) -> Result<AnalysisConfigurationIdentity, SessionPersistenceError> {
    use crate::candidate::{
        canonical_session_term_analysis_identity, project_derived_terminology_analysis_identity,
        reuse_enabled_session_term_analysis_identity,
    };
    for configuration in [
        canonical_session_term_analysis_identity(),
        reuse_enabled_session_term_analysis_identity(),
        project_derived_terminology_analysis_identity(),
    ] {
        if persisted_matches_configuration(persisted, configuration) {
            return Ok(configuration);
        }
    }
    Err(SessionPersistenceError::CanonicalMismatch(
        "analysis configuration identity".to_owned(),
    ))
}

fn persisted_matches_configuration(
    persisted: &PersistedAnalysisSnapshotV1,
    configuration: AnalysisConfigurationIdentity,
) -> bool {
    let actual_detectors = configuration.detector_set().detectors();
    if persisted.detectors.len() != actual_detectors.len() {
        return false;
    }
    for (persisted_detector, actual_detector) in persisted.detectors.iter().zip(actual_detectors) {
        if persisted_detector.id != actual_detector.id()
            || persisted_detector.version != actual_detector.version()
        {
            return false;
        }
    }
    persisted.detector_config.id == configuration.detector_config().id()
        && persisted.detector_config.version == configuration.detector_config().version()
        && persisted.algorithm.id == configuration.algorithm().id()
        && persisted.algorithm.version == configuration.algorithm().version()
}

fn parse_revision_tag_for_canonical(
    tag: &str,
) -> Result<TranscriptRevisionId, SessionPersistenceError> {
    let hex = tag
        .strip_prefix("rev:sha256-v1:")
        .ok_or_else(|| SessionPersistenceError::CanonicalMismatch("revision tag".to_owned()))?;
    let mut digest = [0_u8; 32];
    for (index, chunk) in hex.as_bytes().chunks(2).enumerate() {
        if index >= 32 || chunk.len() != 2 {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "revision digest".to_owned(),
            ));
        }
        let hi = (chunk[0] as char).to_digit(16).ok_or_else(|| {
            SessionPersistenceError::CanonicalMismatch("revision digest".to_owned())
        })? as u8;
        let lo = (chunk[1] as char).to_digit(16).ok_or_else(|| {
            SessionPersistenceError::CanonicalMismatch("revision digest".to_owned())
        })? as u8;
        digest[index] = (hi << 4) | lo;
    }
    Ok(TranscriptRevisionId::from_sha256_digest(digest))
}

pub(crate) fn encode_digest_hex(digest: [u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

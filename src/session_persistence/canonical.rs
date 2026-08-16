use serde::{Deserialize, Serialize};

use crate::analysis::{AnalysisConfigurationIdentity, AnalysisSnapshot, SessionTermsIdentity};
use crate::anchor::TranscriptRevisionId;
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
    persist_governance_event, persist_project_scope, persist_reuse_enabled_binding,
    PersistedProjectScopeV1, PersistedReuseEnabledBindingV1, PersistedReuseGovernanceEventV1,
    active_analysis_selection_identity_for_reuse_enabled,
};
use crate::application_reuse::reusable_influence_snapshot_for_parts;
use crate::transcript::{Segment, Transcript};

pub(crate) const PRODUCT_SESSION_FORMAT_VERSION: u32 = 1;

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

pub(crate) fn restore_session_terms(
    persisted: &[PersistedSessionTermV1],
) -> Vec<SessionTermEntry> {
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

pub(crate) fn persist_review_case(review_case: &ReviewCase) -> Result<PersistedReviewCaseV1, SessionPersistenceError> {
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
            let detector_config = detector_config_identity_from_parts(
                detector_config_id,
                detector_config_version,
            )
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

pub(crate) fn persist_ledger_event(event: &ReviewLedgerEvent) -> PersistedReviewLedgerEventV1 {
    let ReviewLedgerEvent::DecisionRecorded {
        case_id,
        observed_revision,
        decision,
    } = event;
    PersistedReviewLedgerEventV1 {
        case_local_index: case_id.local_index(),
        observed_revision: observed_revision.to_tagged_string(),
        decision: persist_correction_decision(decision),
    }
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
    Ok(ReviewLedgerEvent::DecisionRecorded {
        case_id: ReviewCaseId::local(persisted.case_local_index),
        observed_revision: revision,
        decision: restore_correction_decision(&persisted.decision)?,
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
        vec![
            persist_reuse_enabled_binding(
                1,
                run.analysis_run().snapshot(),
                snapshot.identity(),
                session.reuse_state().governance_events().len(),
            ),
        ]
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
            .collect(),
        project_scope,
        reuse_governance_events,
        reuse_governance_head,
        reuse_enabled_bindings,
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
    pub project_scope: PersistedProjectScopeV1,
    pub reuse_governance_events: Vec<PersistedReuseGovernanceEventV1>,
    pub reuse_governance_head: usize,
    pub reuse_enabled_bindings: Vec<PersistedReuseEnabledBindingV1>,
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
        canonical_session_term_analysis_identity, reuse_enabled_session_term_analysis_identity,
    };
    for configuration in [
        canonical_session_term_analysis_identity(),
        reuse_enabled_session_term_analysis_identity(),
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
        let hi = (chunk[0] as char)
            .to_digit(16)
            .ok_or_else(|| SessionPersistenceError::CanonicalMismatch("revision digest".to_owned()))?
            as u8;
        let lo = (chunk[1] as char)
            .to_digit(16)
            .ok_or_else(|| SessionPersistenceError::CanonicalMismatch("revision digest".to_owned()))?
            as u8;
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

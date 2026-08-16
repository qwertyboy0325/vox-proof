use serde::{Deserialize, Serialize};

use crate::analysis::AnalysisSnapshot;
use crate::anchor::TranscriptRevisionId;
use crate::application_service::ApplicationReviewSession;
use crate::candidate::{
    CandidateAlternative, CandidateSpan, DetectionKind, DetectorProvenance, Evidence,
    GlossaryAliasEvidence, ObservedErrorFormEvidence, SessionTermEntry,
};
use crate::pipeline::CanonicalTermReviewRun;
use crate::review::{CorrectionDecision, ReviewCase, ReviewCaseId, ReviewLedgerEvent};
use crate::session_persistence::error::SessionPersistenceError;
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
pub(crate) struct PersistedAnalysisSnapshotV1 {
    pub(crate) source_revision: String,
    pub(crate) session_terms_identity: String,
    pub(crate) configuration_profile: String,
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
    PersistedAnalysisSnapshotV1 {
        source_revision: snapshot.source_revision().to_tagged_string(),
        session_terms_identity: snapshot.session_terms().to_tagged_string(),
        configuration_profile: "canonical_session_terms_v1".to_owned(),
    }
}

pub(crate) fn verify_analysis_snapshot(
    persisted: &PersistedAnalysisSnapshotV1,
    actual: AnalysisSnapshot,
) -> Result<(), SessionPersistenceError> {
    if persisted.source_revision != actual.source_revision().to_tagged_string()
        || persisted.session_terms_identity != actual.session_terms().to_tagged_string()
        || persisted.configuration_profile != "canonical_session_terms_v1"
    {
        return Err(SessionPersistenceError::CanonicalMismatch(
            "analysis snapshot identity mismatch".to_owned(),
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
        _ => {
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
    format!(
        "{}|{}|canonical_session_terms_v1",
        snapshot.source_revision().to_tagged_string(),
        snapshot.session_terms().to_tagged_string()
    )
}

pub(crate) fn capture_from_session(
    session: &ApplicationReviewSession,
) -> Result<SessionCanonicalCapture, SessionPersistenceError> {
    let snapshot = session.canonical_run().analysis_run().snapshot();
    Ok(SessionCanonicalCapture {
        transcript: persist_transcript(session.source()),
        session_terms: persist_session_terms(session.session_terms()),
        analysis_snapshot: persist_analysis_snapshot(snapshot),
        active_analysis_snapshot_identity: snapshot_identity(snapshot),
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
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionCanonicalCapture {
    pub transcript: PersistedTranscriptV1,
    pub session_terms: Vec<PersistedSessionTermV1>,
    pub analysis_snapshot: PersistedAnalysisSnapshotV1,
    pub active_analysis_snapshot_identity: String,
    pub review_cases: Vec<PersistedReviewCaseV1>,
    pub material_use_basis: String,
    pub authority_role: String,
    pub authority_display_label: String,
    pub review_ledger_head: usize,
    pub ledger_events: Vec<PersistedReviewLedgerEventV1>,
}

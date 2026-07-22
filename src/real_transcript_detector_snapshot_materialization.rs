use std::fmt;

use crate::candidate::{
    DetectionKind, Evidence, PhoneticSimilarityEvidence, PhoneticTargetKind, SessionTermEntry,
};
use crate::detector_snapshot::{
    DETECTOR_PROPOSAL_SNAPSHOT_SCHEMA, DetectorAnalysisIdentity,
    DetectorAsciiLatinPhoneticRepresentation, DetectorComponentIdentity,
    DetectorPhoneticComparisonFacts, DetectorPhoneticTargetKind, DetectorProposalAlternative,
    DetectorProposalEvidence, DetectorProposalId, DetectorProposalRecord,
    DetectorProposalRecordValidationError, DetectorProposalSemanticKey, DetectorProposalSnapshot,
    DetectorProposalSnapshotState, DetectorProposalSnapshotValidationError,
    DetectorProposalSourceAnchor, DetectorSessionTermEntry,
};
use crate::pipeline::CanonicalTermReviewRun;
use crate::real_transcript_detector_snapshot_adapter::{
    RealTranscriptDetectorSnapshotAdapterContractError,
    RealTranscriptDetectorSnapshotAdapterReadiness, RealTranscriptDetectorSnapshotAdapterRequest,
    ValidatedRealTranscriptDetectorSnapshotAdapterPlan,
    validate_real_transcript_detector_snapshot_adapter_request,
};
use crate::real_transcript_evaluation_runner::RealTranscriptEvaluationRunRequest;
use crate::reference_coverage::CueReferenceId;
use crate::review::ReviewCase;
use crate::transcript::Transcript;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealTranscriptDetectorSnapshotMaterializationResult {
    pub validated_plan: ValidatedRealTranscriptDetectorSnapshotAdapterPlan,
    pub detector_snapshot: DetectorProposalSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RealTranscriptDetectorSnapshotMaterializationError {
    ContractValidationFailure(RealTranscriptDetectorSnapshotAdapterContractError),
    ProposalMappingFailure {
        review_case_index: usize,
        source: DetectorProposalRecordValidationError,
    },
    IntegerConversionFailure {
        review_case_index: usize,
        field: &'static str,
    },
    SnapshotAssessmentDerivationFailure(DetectorProposalSnapshotValidationError),
    SnapshotValidationFailure(DetectorProposalSnapshotValidationError),
    SnapshotEnvelopeValidationFailure(DetectorProposalSnapshotValidationError),
    MaterializedProposalCountMismatch {
        expected: u32,
        found: u32,
    },
    MaterializedProposalIdMismatch {
        index: usize,
    },
    MaterializedProposalOrderMismatch {
        index: usize,
    },
    SnapshotPlanBindingMismatch {
        field: &'static str,
    },
}

impl fmt::Display for RealTranscriptDetectorSnapshotMaterializationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for RealTranscriptDetectorSnapshotMaterializationError {}

pub fn materialize_real_transcript_detector_snapshot(
    run_request: &RealTranscriptEvaluationRunRequest,
    adapter_request: &RealTranscriptDetectorSnapshotAdapterRequest,
    transcript: &Transcript,
    canonical_run: &CanonicalTermReviewRun,
) -> Result<
    RealTranscriptDetectorSnapshotMaterializationResult,
    RealTranscriptDetectorSnapshotMaterializationError,
> {
    let validated_plan = validate_real_transcript_detector_snapshot_adapter_request(
        run_request,
        adapter_request,
        transcript,
        canonical_run,
    )
    .map_err(RealTranscriptDetectorSnapshotMaterializationError::ContractValidationFailure)?;

    if validated_plan.readiness
        != RealTranscriptDetectorSnapshotAdapterReadiness::ReadyForSnapshotMaterialization
    {
        return Err(
            RealTranscriptDetectorSnapshotMaterializationError::ContractValidationFailure(
                RealTranscriptDetectorSnapshotAdapterContractError::MissingSchemaRevision,
            ),
        );
    }

    let mut proposals = Vec::with_capacity(validated_plan.proposal_ids.len());
    for (index, review_case) in canonical_run.review_cases().iter().enumerate() {
        if review_case.id().local_index() != index {
            return Err(
                RealTranscriptDetectorSnapshotMaterializationError::MaterializedProposalOrderMismatch {
                    index,
                },
            );
        }

        let proposal_id = validated_plan.proposal_ids.get(index).ok_or(
            RealTranscriptDetectorSnapshotMaterializationError::MaterializedProposalIdMismatch {
                index,
            },
        )?;

        let record = map_review_case_to_proposal_record(
            transcript,
            review_case,
            index,
            proposal_id,
            &validated_plan,
        )?;

        record
            .validate(
                &validated_plan.snapshot_revision,
                &validated_plan.input_identity,
                &validated_plan.detector_analysis_identity,
            )
            .map_err(|source| {
                RealTranscriptDetectorSnapshotMaterializationError::ProposalMappingFailure {
                    review_case_index: index,
                    source,
                }
            })?;

        proposals.push(record);
    }

    let found_count = u32::try_from(proposals.len()).map_err(|_| {
        RealTranscriptDetectorSnapshotMaterializationError::MaterializedProposalCountMismatch {
            expected: validated_plan.proposal_count,
            found: u32::MAX,
        }
    })?;
    if found_count != validated_plan.proposal_count {
        return Err(
            RealTranscriptDetectorSnapshotMaterializationError::MaterializedProposalCountMismatch {
                expected: validated_plan.proposal_count,
                found: found_count,
            },
        );
    }

    for (index, (proposal_id, record)) in validated_plan
        .proposal_ids
        .iter()
        .zip(proposals.iter())
        .enumerate()
    {
        if &record.detector_proposal_id != proposal_id {
            return Err(
                RealTranscriptDetectorSnapshotMaterializationError::MaterializedProposalIdMismatch {
                    index,
                },
            );
        }
    }

    let assessment = DetectorProposalSnapshot::derive_assessment(
        &validated_plan.snapshot_revision,
        &validated_plan.input_identity,
        &validated_plan.detector_analysis_identity,
        &proposals,
    )
    .map_err(
        RealTranscriptDetectorSnapshotMaterializationError::SnapshotAssessmentDerivationFailure,
    )?;

    if assessment.total_proposal_count != validated_plan.proposal_count
        || !assessment.duplicate_proposal_ids.is_empty()
        || !assessment.duplicate_semantic_keys.is_empty()
        || !assessment.context_mismatch_proposal_ids.is_empty()
        || !assessment.detector_not_in_analysis_set.is_empty()
        || !assessment.context_consistent
    {
        return Err(
            RealTranscriptDetectorSnapshotMaterializationError::SnapshotAssessmentDerivationFailure(
                DetectorProposalSnapshotValidationError::AssessmentMismatch {
                    stored: Box::new(assessment.clone()),
                    derived: Box::new(assessment),
                },
            ),
        );
    }

    let detector_snapshot = DetectorProposalSnapshot {
        schema_revision: DETECTOR_PROPOSAL_SNAPSHOT_SCHEMA.to_string(),
        run_id: validated_plan.run_id.clone(),
        input_identity: validated_plan.input_identity.clone(),
        calibration_validity: validated_plan.calibration_validity,
        snapshot_revision: validated_plan.snapshot_revision.clone(),
        detector_output_artifact_id: validated_plan.detector_output_artifact_id.clone(),
        analysis_identity: validated_plan.detector_analysis_identity.clone(),
        proposals,
        frozen_at_unix_ms: validated_plan.frozen_at_unix_ms,
        state: DetectorProposalSnapshotState::Frozen,
        assessment,
    };

    detector_snapshot
        .validate()
        .map_err(RealTranscriptDetectorSnapshotMaterializationError::SnapshotValidationFailure)?;

    detector_snapshot
        .validate_for_freeze_against(&run_request.detector_execution_envelope)
        .map_err(
            RealTranscriptDetectorSnapshotMaterializationError::SnapshotEnvelopeValidationFailure,
        )?;

    verify_snapshot_matches_validated_plan(&detector_snapshot, &validated_plan)?;

    Ok(RealTranscriptDetectorSnapshotMaterializationResult {
        validated_plan,
        detector_snapshot,
    })
}

fn verify_snapshot_matches_validated_plan(
    snapshot: &DetectorProposalSnapshot,
    validated_plan: &ValidatedRealTranscriptDetectorSnapshotAdapterPlan,
) -> Result<(), RealTranscriptDetectorSnapshotMaterializationError> {
    let bindings = [
        ("run_id", snapshot.run_id == validated_plan.run_id),
        (
            "input_identity",
            snapshot.input_identity == validated_plan.input_identity,
        ),
        (
            "calibration_validity",
            snapshot.calibration_validity == validated_plan.calibration_validity,
        ),
        (
            "snapshot_revision",
            snapshot.snapshot_revision == validated_plan.snapshot_revision,
        ),
        (
            "detector_output_artifact_id",
            snapshot.detector_output_artifact_id == validated_plan.detector_output_artifact_id,
        ),
        (
            "analysis_identity",
            snapshot.analysis_identity == validated_plan.detector_analysis_identity,
        ),
        (
            "frozen_at_unix_ms",
            snapshot.frozen_at_unix_ms == validated_plan.frozen_at_unix_ms,
        ),
        (
            "proposal_count",
            snapshot.proposals.len() as u32 == validated_plan.proposal_count,
        ),
        (
            "snapshot_state",
            snapshot.state == DetectorProposalSnapshotState::Frozen,
        ),
    ];

    for (field, matches) in bindings {
        if !matches {
            return Err(
                RealTranscriptDetectorSnapshotMaterializationError::SnapshotPlanBindingMismatch {
                    field,
                },
            );
        }
    }

    if snapshot.proposals.len() != validated_plan.proposal_ids.len() {
        return Err(
            RealTranscriptDetectorSnapshotMaterializationError::MaterializedProposalCountMismatch {
                expected: validated_plan.proposal_count,
                found: snapshot.proposals.len() as u32,
            },
        );
    }

    for (index, (expected_id, record)) in validated_plan
        .proposal_ids
        .iter()
        .zip(snapshot.proposals.iter())
        .enumerate()
    {
        if &record.detector_proposal_id != expected_id {
            return Err(
                RealTranscriptDetectorSnapshotMaterializationError::MaterializedProposalIdMismatch {
                    index,
                },
            );
        }
    }

    Ok(())
}

fn map_review_case_to_proposal_record(
    transcript: &Transcript,
    review_case: &ReviewCase,
    review_case_index: usize,
    proposal_id: &DetectorProposalId,
    validated_plan: &ValidatedRealTranscriptDetectorSnapshotAdapterPlan,
) -> Result<DetectorProposalRecord, RealTranscriptDetectorSnapshotMaterializationError> {
    let candidate = review_case.candidate_span();
    let anchor = candidate.anchor();

    let segment_position = anchor.segment_position;
    let cue_index = transcript.segments()[segment_position].index();
    let cue_id = CueReferenceId::new(cue_index).map_err(|_| {
        RealTranscriptDetectorSnapshotMaterializationError::ProposalMappingFailure {
            review_case_index,
            source: DetectorProposalRecordValidationError::InvalidSourceAnchor(
                crate::detector_snapshot::DetectorProposalSourceAnchorError::InvalidCueReferenceId(
                    crate::reference_coverage::CueReferenceIdError::ZeroNotPermitted,
                ),
            ),
        }
    })?;

    let segment_position_u32 =
        u32_from_usize_materialization(segment_position, review_case_index, "segment_position")?;
    let start_byte_u32 =
        u32_from_usize_materialization(anchor.start_byte, review_case_index, "start_byte")?;
    let end_byte_u32 =
        u32_from_usize_materialization(anchor.end_byte, review_case_index, "end_byte")?;

    let observed_surface = transcript
        .resolve(anchor)
        .ok_or(
            RealTranscriptDetectorSnapshotMaterializationError::ProposalMappingFailure {
                review_case_index,
                source: DetectorProposalRecordValidationError::EmptyObservedSurface,
            },
        )?
        .to_string();

    let source_anchor = DetectorProposalSourceAnchor {
        input_identity: validated_plan.input_identity.clone(),
        cue_id,
        segment_position: segment_position_u32,
        start_byte: start_byte_u32,
        end_byte: end_byte_u32,
    };

    let provenance = candidate.provenance();
    let detector = DetectorComponentIdentity {
        id: provenance.detector_id().to_string(),
        version: provenance.detector_version().to_string(),
    };

    let evidence = map_candidate_evidence(
        candidate.kind(),
        candidate.evidence(),
        &observed_surface,
        review_case_index,
        &validated_plan.detector_analysis_identity,
    )?;

    let alternatives = map_candidate_alternatives(candidate.alternatives(), review_case_index)?;

    let mut record = DetectorProposalRecord {
        detector_proposal_id: proposal_id.clone(),
        snapshot_revision: validated_plan.snapshot_revision.clone(),
        input_identity: validated_plan.input_identity.clone(),
        semantic_key: DetectorProposalSemanticKey {
            detector_id: String::new(),
            detection_kind: candidate.kind(),
            source_anchor: source_anchor.clone(),
        },
        detector,
        source_anchor,
        observed_surface,
        detection_kind: candidate.kind(),
        evidence,
        alternatives,
    };

    record.semantic_key = record.derive_semantic_key();
    Ok(record)
}

fn map_candidate_alternatives(
    alternatives: &[crate::candidate::CandidateAlternative],
    review_case_index: usize,
) -> Result<Vec<DetectorProposalAlternative>, RealTranscriptDetectorSnapshotMaterializationError> {
    let mut mapped = Vec::with_capacity(alternatives.len());
    for (index, alternative) in alternatives.iter().enumerate() {
        let alternative_index =
            u32_from_usize_materialization(index, review_case_index, "alternative_index")?;
        mapped.push(DetectorProposalAlternative {
            alternative_index,
            replacement_surface: alternative.replacement_text().to_string(),
        });
    }
    Ok(mapped)
}

fn map_session_term_entry(entry: &SessionTermEntry) -> DetectorSessionTermEntry {
    DetectorSessionTermEntry {
        canonical_term: entry.canonical_term.clone(),
        aliases: entry.aliases.clone(),
        observed_error_forms: entry.observed_error_forms.clone(),
    }
}

fn map_phonetic_target_kind(kind: PhoneticTargetKind) -> DetectorPhoneticTargetKind {
    match kind {
        PhoneticTargetKind::CanonicalTerm => DetectorPhoneticTargetKind::CanonicalTerm,
        PhoneticTargetKind::Alias => DetectorPhoneticTargetKind::Alias,
    }
}

fn map_phonetic_representation(
    representation: &crate::candidate::AsciiLatinPhoneticRepresentation,
) -> DetectorAsciiLatinPhoneticRepresentation {
    DetectorAsciiLatinPhoneticRepresentation {
        normalized_letters: representation.normalized_letters.clone(),
        primary_key: representation.primary_key.clone(),
        alternate_key: representation.alternate_key.clone(),
    }
}

fn map_candidate_evidence(
    kind: DetectionKind,
    evidence: &Evidence,
    observed_surface: &str,
    review_case_index: usize,
    analysis_identity: &DetectorAnalysisIdentity,
) -> Result<DetectorProposalEvidence, RealTranscriptDetectorSnapshotMaterializationError> {
    match evidence {
        Evidence::GlossaryAlias(evidence_data) => {
            if kind != DetectionKind::GlossaryAliasMatch {
                return Err(
                    RealTranscriptDetectorSnapshotMaterializationError::ProposalMappingFailure {
                        review_case_index,
                        source: DetectorProposalRecordValidationError::EvidenceValidation(
                            crate::detector_snapshot::DetectorProposalEvidenceValidationError::IncompatibleDetectionKind {
                                evidence: "glossary_alias",
                                detection_kind: kind,
                            },
                        ),
                    },
                );
            }
            Ok(DetectorProposalEvidence::GlossaryAlias {
                entry: map_session_term_entry(&evidence_data.entry),
                matched_form: evidence_data.matched_form.clone(),
            })
        }
        Evidence::ObservedErrorForm(evidence_data) => {
            if kind != DetectionKind::GlossaryAliasMatch {
                return Err(
                    RealTranscriptDetectorSnapshotMaterializationError::ProposalMappingFailure {
                        review_case_index,
                        source: DetectorProposalRecordValidationError::EvidenceValidation(
                            crate::detector_snapshot::DetectorProposalEvidenceValidationError::IncompatibleDetectionKind {
                                evidence: "observed_error_form",
                                detection_kind: kind,
                            },
                        ),
                    },
                );
            }
            Ok(DetectorProposalEvidence::ObservedErrorForm {
                entry: map_session_term_entry(&evidence_data.entry),
                matched_form: evidence_data.matched_form.clone(),
            })
        }
        Evidence::PhoneticSimilarity(phonetic) => map_phonetic_evidence(
            kind,
            phonetic,
            observed_surface,
            review_case_index,
            analysis_identity,
        ),
    }
}

fn map_phonetic_evidence(
    kind: DetectionKind,
    phonetic: &PhoneticSimilarityEvidence,
    observed_surface: &str,
    review_case_index: usize,
    analysis_identity: &DetectorAnalysisIdentity,
) -> Result<DetectorProposalEvidence, RealTranscriptDetectorSnapshotMaterializationError> {
    if kind != DetectionKind::PhoneticSimilarity {
        return Err(
            RealTranscriptDetectorSnapshotMaterializationError::ProposalMappingFailure {
                review_case_index,
                source: DetectorProposalRecordValidationError::EvidenceValidation(
                    crate::detector_snapshot::DetectorProposalEvidenceValidationError::IncompatibleDetectionKind {
                        evidence: "phonetic_similarity",
                        detection_kind: kind,
                    },
                ),
            },
        );
    }

    if phonetic.observed_surface != observed_surface {
        return Err(
            RealTranscriptDetectorSnapshotMaterializationError::ProposalMappingFailure {
                review_case_index,
                source: DetectorProposalRecordValidationError::EvidenceValidation(
                    crate::detector_snapshot::DetectorProposalEvidenceValidationError::ObservedSurfaceMismatch,
                ),
            },
        );
    }

    let detector_config = DetectorComponentIdentity {
        id: phonetic.detector_config.id().to_string(),
        version: phonetic.detector_config.version().to_string(),
    };
    let algorithm = DetectorComponentIdentity {
        id: phonetic.algorithm.id().to_string(),
        version: phonetic.algorithm.version().to_string(),
    };

    if detector_config != analysis_identity.detector_config {
        return Err(
            RealTranscriptDetectorSnapshotMaterializationError::ProposalMappingFailure {
                review_case_index,
                source: DetectorProposalRecordValidationError::EvidenceValidation(
                    crate::detector_snapshot::DetectorProposalEvidenceValidationError::DetectorConfigMismatch,
                ),
            },
        );
    }
    if algorithm != analysis_identity.algorithm {
        return Err(
            RealTranscriptDetectorSnapshotMaterializationError::ProposalMappingFailure {
                review_case_index,
                source: DetectorProposalRecordValidationError::EvidenceValidation(
                    crate::detector_snapshot::DetectorProposalEvidenceValidationError::AlgorithmMismatch,
                ),
            },
        );
    }

    let edit_distance = u32_from_usize_materialization(
        phonetic.comparison.edit_distance,
        review_case_index,
        "edit_distance",
    )?;
    let ratio_numerator = u32_from_usize_materialization(
        phonetic.comparison.ratio_numerator,
        review_case_index,
        "ratio_numerator",
    )?;
    let ratio_denominator = u32_from_usize_materialization(
        phonetic.comparison.ratio_denominator,
        review_case_index,
        "ratio_denominator",
    )?;
    let ratio_permille = u32_from_usize_materialization(
        phonetic.comparison.ratio_permille,
        review_case_index,
        "ratio_permille",
    )?;

    Ok(DetectorProposalEvidence::PhoneticSimilarity {
        observed_surface: phonetic.observed_surface.clone(),
        target_surface: phonetic.target_surface.clone(),
        target_kind: map_phonetic_target_kind(phonetic.target_kind),
        canonical_term: phonetic.canonical_term.clone(),
        source_representation: map_phonetic_representation(&phonetic.source_representation),
        target_representation: map_phonetic_representation(&phonetic.target_representation),
        comparison: DetectorPhoneticComparisonFacts {
            edit_distance,
            ratio_numerator,
            ratio_denominator,
            ratio_permille,
            matched_key: phonetic.comparison.matched_key.clone(),
        },
        detector_config,
        algorithm,
    })
}

fn u32_from_usize_materialization(
    value: usize,
    review_case_index: usize,
    field: &'static str,
) -> Result<u32, RealTranscriptDetectorSnapshotMaterializationError> {
    value.try_into().map_err(|_| {
        RealTranscriptDetectorSnapshotMaterializationError::IntegerConversionFailure {
            review_case_index,
            field,
        }
    })
}

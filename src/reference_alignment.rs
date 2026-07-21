use std::collections::{HashMap, HashSet};

use crate::human_final_reference::{
    HumanFinalReference, HumanFinalReferenceValidationError, ReferenceClass,
};
use crate::reference_coverage::{
    CueReferenceId, CueReviewCompletionRecord, ExpectedCueUniverse, ReferenceCoverage,
    ReferenceCoverageValidationError, ReferenceCueDisposition,
};

pub fn validate_coverage_against_human_reference(
    coverage: &ReferenceCoverage,
    human_reference: &HumanFinalReference,
) -> Result<(), ReferenceCoverageValidationError> {
    if coverage.run_id != human_reference.run_id {
        return Err(ReferenceCoverageValidationError::RunIdMismatch);
    }

    if coverage.input_identity != human_reference.input_identity {
        return Err(ReferenceCoverageValidationError::InputIdentityMismatch);
    }

    if coverage.seal_id != human_reference.seal_id {
        return Err(ReferenceCoverageValidationError::SealIdMismatch);
    }

    if coverage.reference_revision != human_reference.reference_revision {
        return Err(ReferenceCoverageValidationError::ReferenceRevisionMismatch);
    }

    let expected_cue_ids: HashSet<CueReferenceId> =
        coverage.expected_universe.cue_ids.iter().copied().collect();

    let mut te_records_by_cue: HashMap<CueReferenceId, u32> = HashMap::new();
    let mut recall_eligible_count = 0u32;

    for record in &human_reference.records {
        let cue_id = record.source_anchor.cue_id;
        if !expected_cue_ids.contains(&cue_id) {
            return Err(ReferenceCoverageValidationError::UnknownReferenceCueId { cue_id });
        }

        if !cue_matches_segment_position(
            &coverage.expected_universe,
            record.source_anchor.segment_position,
            cue_id,
        ) {
            return Err(
                ReferenceCoverageValidationError::ReferenceAnchorMappingMismatch {
                    cue_id,
                    segment_position: record.source_anchor.segment_position,
                },
            );
        }

        if record.reference_class == ReferenceClass::TranscriptionError {
            *te_records_by_cue.entry(cue_id).or_default() += 1;
            if record.is_recall_eligible() {
                recall_eligible_count += 1;
            }
        }
    }

    if coverage.assessment.total_eligible_transcription_errors != recall_eligible_count {
        return Err(
            ReferenceCoverageValidationError::EligibleTranscriptionErrorCountMismatch {
                stored: coverage.assessment.total_eligible_transcription_errors,
                derived: recall_eligible_count,
            },
        );
    }

    for coverage_record in &coverage.records {
        match coverage_record.disposition {
            ReferenceCueDisposition::TranscriptionError => {
                if te_records_by_cue
                    .get(&coverage_record.cue_id)
                    .copied()
                    .unwrap_or(0)
                    == 0
                {
                    return Err(
                        ReferenceCoverageValidationError::TranscriptionErrorCueMissingRecord {
                            cue_id: coverage_record.cue_id,
                        },
                    );
                }
            }
            ReferenceCueDisposition::NoTranscriptionError => {
                if te_records_by_cue.contains_key(&coverage_record.cue_id) {
                    return Err(
                        ReferenceCoverageValidationError::NoTranscriptionErrorCueHasRecord {
                            cue_id: coverage_record.cue_id,
                        },
                    );
                }
            }
            ReferenceCueDisposition::Uncertain | ReferenceCueDisposition::Unreviewable => {
                if te_records_by_cue.contains_key(&coverage_record.cue_id) {
                    return Err(
                        ReferenceCoverageValidationError::TranscriptionErrorRecordForUnresolvedCue {
                            cue_id: coverage_record.cue_id,
                        },
                    );
                }
            }
        }
    }

    Ok(())
}

pub fn map_alignment_error_to_human_final(
    error: ReferenceCoverageValidationError,
) -> HumanFinalReferenceValidationError {
    match error {
        ReferenceCoverageValidationError::RunIdMismatch => {
            HumanFinalReferenceValidationError::RunIdMismatch
        }
        ReferenceCoverageValidationError::InputIdentityMismatch => {
            HumanFinalReferenceValidationError::InputIdentityMismatch
        }
        ReferenceCoverageValidationError::SealIdMismatch => {
            HumanFinalReferenceValidationError::SealIdMismatch
        }
        ReferenceCoverageValidationError::ReferenceRevisionMismatch => {
            HumanFinalReferenceValidationError::CoverageReferenceRevisionMismatch
        }
        ReferenceCoverageValidationError::UnknownReferenceCueId { cue_id } => {
            HumanFinalReferenceValidationError::UnknownCueReferenceId { cue_id }
        }
        ReferenceCoverageValidationError::ReferenceAnchorMappingMismatch {
            cue_id,
            segment_position,
        } => HumanFinalReferenceValidationError::ReferenceAnchorMappingMismatch {
            cue_id,
            segment_position,
        },
        ReferenceCoverageValidationError::TranscriptionErrorCueMissingRecord { cue_id } => {
            HumanFinalReferenceValidationError::TranscriptionErrorCueMissingRecord { cue_id }
        }
        ReferenceCoverageValidationError::NoTranscriptionErrorCueHasRecord { cue_id } => {
            HumanFinalReferenceValidationError::NoTranscriptionErrorCueHasRecord { cue_id }
        }
        ReferenceCoverageValidationError::TranscriptionErrorRecordForUnresolvedCue { cue_id } => {
            HumanFinalReferenceValidationError::TranscriptionErrorRecordForUnresolvedCue { cue_id }
        }
        ReferenceCoverageValidationError::EligibleTranscriptionErrorCountMismatch { .. } => {
            HumanFinalReferenceValidationError::EligibleTranscriptionErrorCountMismatch
        }
        other => HumanFinalReferenceValidationError::CoverageValidation(other),
    }
}

fn cue_matches_segment_position(
    universe: &ExpectedCueUniverse,
    segment_position: u32,
    cue_id: CueReferenceId,
) -> bool {
    universe
        .cue_ids
        .get(segment_position as usize)
        .copied()
        .is_some_and(|expected| expected == cue_id)
}

pub fn cue_id_for_segment_position(
    universe: &ExpectedCueUniverse,
    segment_position: u32,
) -> Option<CueReferenceId> {
    universe.cue_ids.get(segment_position as usize).copied()
}

pub fn validate_completion_record_mapping(
    universe: &ExpectedCueUniverse,
    record: &CueReviewCompletionRecord,
) -> Result<(), ReferenceCoverageValidationError> {
    let Some(expected_cue_id) = cue_id_for_segment_position(universe, record.segment_position)
    else {
        return Err(
            ReferenceCoverageValidationError::SegmentPositionOutOfRange {
                segment_position: record.segment_position,
            },
        );
    };

    if expected_cue_id != record.cue_id {
        return Err(ReferenceCoverageValidationError::CueMappingMismatch {
            cue_id: record.cue_id,
            segment_position: record.segment_position,
            expected_cue_id,
        });
    }

    Ok(())
}

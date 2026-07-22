use std::fmt;

use crate::analysis::AnalysisSnapshot;
use crate::analysis::{AlgorithmIdentity, DetectorConfigIdentity};
use crate::artifact_bundle::ArtifactId;
use crate::candidate::{DetectionKind, Evidence, PhoneticSimilarityEvidence};
use crate::detector_snapshot::{
    DetectorAnalysisIdentity, DetectorAnalysisIdentityValidationError, DetectorComponentIdentity,
    DetectorProposalId, DetectorSnapshotRevisionId,
};
use crate::pipeline::CanonicalTermReviewRun;
use crate::real_transcript_evaluation_runner::{
    RealTranscriptEvaluationRunRequest, RealTranscriptEvaluationRunnerContractError,
    validate_real_transcript_evaluation_run_request,
};
use crate::reference_coverage::CueReferenceId;
use crate::review::ReviewCase;
use crate::run_manifest::{CalibrationValidityMode, InputIdentityReference};
use crate::transcript::Transcript;

pub const REAL_TRANSCRIPT_DETECTOR_SNAPSHOT_ADAPTER_REQUEST_SCHEMA: &str =
    "voxproof-real-transcript-detector-snapshot-adapter-request-v1";
pub const REAL_TRANSCRIPT_DETECTOR_SNAPSHOT_ADAPTER_POLICY: &str =
    "voxproof-canonical-term-review-snapshot-adapter-v1";
pub const REAL_TRANSCRIPT_DETECTOR_PROPOSAL_ID_POLICY: &str =
    "voxproof-explicit-proposal-id-order-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealTranscriptDetectorSnapshotAdapterRequest {
    pub schema_revision: String,
    pub adapter_policy_revision: String,
    pub proposal_id_policy_revision: String,
    pub snapshot_revision: DetectorSnapshotRevisionId,
    pub detector_output_artifact_id: ArtifactId,
    pub frozen_at_unix_ms: u64,
    pub proposal_ids: Vec<DetectorProposalId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedRealTranscriptDetectorSnapshotAdapterPlan {
    pub run_id: crate::run_manifest::RunId,
    pub input_identity: InputIdentityReference,
    pub calibration_validity: CalibrationValidityMode,
    pub snapshot_revision: DetectorSnapshotRevisionId,
    pub detector_output_artifact_id: ArtifactId,
    pub frozen_at_unix_ms: u64,
    pub detector_analysis_identity: DetectorAnalysisIdentity,
    pub proposal_ids: Vec<DetectorProposalId>,
    pub proposal_count: u32,
    pub readiness: RealTranscriptDetectorSnapshotAdapterReadiness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealTranscriptDetectorSnapshotAdapterReadiness {
    ReadyForSnapshotMaterialization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectorSnapshotAdapterAnalysisField {
    InputIdentity,
    SessionTermsIdentity,
    DetectorSet,
    DetectorConfig,
    Algorithm,
}

impl fmt::Display for DetectorSnapshotAdapterAnalysisField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::InputIdentity => "input_identity",
            Self::SessionTermsIdentity => "session_terms_identity",
            Self::DetectorSet => "detector_set",
            Self::DetectorConfig => "detector_config",
            Self::Algorithm => "algorithm",
        };
        f.write_str(label)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RealTranscriptDetectorSnapshotAdapterContractError {
    MissingSchemaRevision,
    UnsupportedSchemaRevision {
        found: String,
        expected: String,
    },
    UnsupportedAdapterPolicy {
        found: String,
        expected: String,
    },
    UnsupportedProposalIdPolicy {
        found: String,
        expected: String,
    },
    RunnerContractValidationFailure(RealTranscriptEvaluationRunnerContractError),
    InvalidSnapshotRevision(crate::detector_snapshot::DetectorSnapshotIdentityError),
    InvalidDetectorOutputArtifactId(crate::artifact_bundle::ArtifactBundleIdError),
    ZeroFrozenTimestamp,
    TranscriptInputIdentityMismatch,
    AnalysisSourceRevisionMismatch,
    DerivedAnalysisIdentityValidationFailure(DetectorAnalysisIdentityValidationError),
    AnalysisIdentityMismatch {
        field: DetectorSnapshotAdapterAnalysisField,
    },
    ProposalIdCountMismatch {
        expected: usize,
        found: usize,
    },
    InvalidProposalId {
        index: usize,
    },
    DuplicateProposalId {
        proposal_id: DetectorProposalId,
    },
    ReviewCaseLocalIndexMismatch {
        expected: usize,
        found: usize,
    },
    AnchorRevisionMismatch {
        review_case_index: usize,
    },
    SegmentPositionOutOfRange {
        review_case_index: usize,
        segment_position: usize,
    },
    CueReferenceIdInvalid {
        review_case_index: usize,
        cue_index: u32,
    },
    ObservedSurfaceResolutionFailure {
        review_case_index: usize,
    },
    EmptyObservedSurface {
        review_case_index: usize,
    },
    IntegerConversionOverflow {
        review_case_index: usize,
        field: &'static str,
    },
    DetectorComponentInvalid {
        review_case_index: usize,
    },
    DetectorNotInAnalysisSet {
        review_case_index: usize,
    },
    CandidateKindEvidenceMismatch {
        review_case_index: usize,
    },
    UnsupportedCandidateEvidence {
        review_case_index: usize,
    },
    PhoneticObservedSurfaceMismatch {
        review_case_index: usize,
    },
    PhoneticDetectorConfigMismatch {
        review_case_index: usize,
    },
    PhoneticAlgorithmMismatch {
        review_case_index: usize,
    },
    PhoneticComparisonConversionOverflow {
        review_case_index: usize,
        field: &'static str,
    },
    PhoneticZeroRatioDenominator {
        review_case_index: usize,
    },
    PhoneticInconsistentRatioPermille {
        review_case_index: usize,
        expected: u32,
        found: u32,
    },
    DuplicateFutureProposalSemanticKey {
        first_review_case_index: usize,
        duplicate_review_case_index: usize,
    },
    AlternativeIndexConversionOverflow {
        review_case_index: usize,
        alternative_index: usize,
    },
}

impl fmt::Display for RealTranscriptDetectorSnapshotAdapterContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for RealTranscriptDetectorSnapshotAdapterContractError {}

/// Derives the detector analysis identity bound to a canonical pipeline run snapshot.
pub fn derive_detector_analysis_identity_from_canonical_run(
    canonical_run: &CanonicalTermReviewRun,
) -> Result<DetectorAnalysisIdentity, RealTranscriptDetectorSnapshotAdapterContractError> {
    derive_detector_analysis_identity_from_snapshot(canonical_run.analysis_run().snapshot())
}

pub fn validate_real_transcript_detector_snapshot_adapter_request(
    run_request: &RealTranscriptEvaluationRunRequest,
    adapter_request: &RealTranscriptDetectorSnapshotAdapterRequest,
    transcript: &Transcript,
    canonical_run: &CanonicalTermReviewRun,
) -> Result<
    ValidatedRealTranscriptDetectorSnapshotAdapterPlan,
    RealTranscriptDetectorSnapshotAdapterContractError,
> {
    validate_adapter_request_metadata(adapter_request)?;

    let validated_plan = validate_real_transcript_evaluation_run_request(run_request).map_err(
        RealTranscriptDetectorSnapshotAdapterContractError::RunnerContractValidationFailure,
    )?;

    bind_transcript_identity(transcript, &validated_plan.input_identity, run_request)?;
    bind_analysis_source_revision(transcript, canonical_run)?;

    let derived_identity =
        derive_detector_analysis_identity_from_snapshot(canonical_run.analysis_run().snapshot())?;
    bind_derived_analysis_identity(
        &derived_identity,
        &validated_plan.detector_analysis_identity,
    )?;

    validate_proposal_id_inventory(adapter_request, canonical_run)?;
    validate_review_case_mappings(
        transcript,
        canonical_run,
        &validated_plan.detector_analysis_identity,
    )?;

    let proposal_count = u32::try_from(adapter_request.proposal_ids.len()).map_err(|_| {
        RealTranscriptDetectorSnapshotAdapterContractError::IntegerConversionOverflow {
            review_case_index: 0,
            field: "proposal_count",
        }
    })?;

    Ok(ValidatedRealTranscriptDetectorSnapshotAdapterPlan {
        run_id: validated_plan.run_id,
        input_identity: validated_plan.input_identity,
        calibration_validity: run_request.detector_execution_envelope.calibration_validity,
        snapshot_revision: adapter_request.snapshot_revision.clone(),
        detector_output_artifact_id: adapter_request.detector_output_artifact_id.clone(),
        frozen_at_unix_ms: adapter_request.frozen_at_unix_ms,
        detector_analysis_identity: validated_plan.detector_analysis_identity,
        proposal_ids: adapter_request.proposal_ids.clone(),
        proposal_count,
        readiness: RealTranscriptDetectorSnapshotAdapterReadiness::ReadyForSnapshotMaterialization,
    })
}

fn validate_adapter_request_metadata(
    adapter_request: &RealTranscriptDetectorSnapshotAdapterRequest,
) -> Result<(), RealTranscriptDetectorSnapshotAdapterContractError> {
    if adapter_request.schema_revision.is_empty() {
        return Err(RealTranscriptDetectorSnapshotAdapterContractError::MissingSchemaRevision);
    }
    if adapter_request.schema_revision != REAL_TRANSCRIPT_DETECTOR_SNAPSHOT_ADAPTER_REQUEST_SCHEMA {
        return Err(
            RealTranscriptDetectorSnapshotAdapterContractError::UnsupportedSchemaRevision {
                found: adapter_request.schema_revision.clone(),
                expected: REAL_TRANSCRIPT_DETECTOR_SNAPSHOT_ADAPTER_REQUEST_SCHEMA.to_string(),
            },
        );
    }
    if adapter_request.adapter_policy_revision != REAL_TRANSCRIPT_DETECTOR_SNAPSHOT_ADAPTER_POLICY {
        return Err(
            RealTranscriptDetectorSnapshotAdapterContractError::UnsupportedAdapterPolicy {
                found: adapter_request.adapter_policy_revision.clone(),
                expected: REAL_TRANSCRIPT_DETECTOR_SNAPSHOT_ADAPTER_POLICY.to_string(),
            },
        );
    }
    if adapter_request.proposal_id_policy_revision != REAL_TRANSCRIPT_DETECTOR_PROPOSAL_ID_POLICY {
        return Err(
            RealTranscriptDetectorSnapshotAdapterContractError::UnsupportedProposalIdPolicy {
                found: adapter_request.proposal_id_policy_revision.clone(),
                expected: REAL_TRANSCRIPT_DETECTOR_PROPOSAL_ID_POLICY.to_string(),
            },
        );
    }

    DetectorSnapshotRevisionId::new(adapter_request.snapshot_revision.as_str())
        .map_err(RealTranscriptDetectorSnapshotAdapterContractError::InvalidSnapshotRevision)?;

    ArtifactId::new(adapter_request.detector_output_artifact_id.as_str()).map_err(
        RealTranscriptDetectorSnapshotAdapterContractError::InvalidDetectorOutputArtifactId,
    )?;

    if adapter_request.frozen_at_unix_ms == 0 {
        return Err(RealTranscriptDetectorSnapshotAdapterContractError::ZeroFrozenTimestamp);
    }

    Ok(())
}

fn bind_transcript_identity(
    transcript: &Transcript,
    validated_input_identity: &InputIdentityReference,
    run_request: &RealTranscriptEvaluationRunRequest,
) -> Result<(), RealTranscriptDetectorSnapshotAdapterContractError> {
    let transcript_identity = InputIdentityReference {
        transcript_revision_id: transcript.revision_id().to_tagged_string(),
    };

    if transcript_identity != *validated_input_identity {
        return Err(
            RealTranscriptDetectorSnapshotAdapterContractError::TranscriptInputIdentityMismatch,
        );
    }

    if transcript_identity != run_request.detector_execution_envelope.input_identity {
        return Err(
            RealTranscriptDetectorSnapshotAdapterContractError::TranscriptInputIdentityMismatch,
        );
    }

    Ok(())
}

fn bind_analysis_source_revision(
    transcript: &Transcript,
    canonical_run: &CanonicalTermReviewRun,
) -> Result<(), RealTranscriptDetectorSnapshotAdapterContractError> {
    if canonical_run.analysis_run().snapshot().source_revision() != transcript.revision_id() {
        return Err(
            RealTranscriptDetectorSnapshotAdapterContractError::AnalysisSourceRevisionMismatch,
        );
    }
    Ok(())
}

fn derive_detector_analysis_identity_from_snapshot(
    snapshot: AnalysisSnapshot,
) -> Result<DetectorAnalysisIdentity, RealTranscriptDetectorSnapshotAdapterContractError> {
    let input_identity = InputIdentityReference {
        transcript_revision_id: snapshot.source_revision().to_tagged_string(),
    };
    let configuration = snapshot.configuration();
    let detector_set = configuration
        .detector_set()
        .detectors()
        .iter()
        .map(|detector| DetectorComponentIdentity {
            id: detector.id().to_string(),
            version: detector.version().to_string(),
        })
        .collect::<Vec<_>>();

    let identity = DetectorAnalysisIdentity {
        input_identity: input_identity.clone(),
        session_terms_identity: snapshot.session_terms().to_tagged_string(),
        detector_set,
        detector_config: DetectorComponentIdentity {
            id: configuration.detector_config().id().to_string(),
            version: configuration.detector_config().version().to_string(),
        },
        algorithm: DetectorComponentIdentity {
            id: configuration.algorithm().id().to_string(),
            version: configuration.algorithm().version().to_string(),
        },
    };

    identity
        .validate(&input_identity)
        .map_err(RealTranscriptDetectorSnapshotAdapterContractError::DerivedAnalysisIdentityValidationFailure)?;

    Ok(identity)
}

fn bind_derived_analysis_identity(
    derived: &DetectorAnalysisIdentity,
    expected: &DetectorAnalysisIdentity,
) -> Result<(), RealTranscriptDetectorSnapshotAdapterContractError> {
    if derived.input_identity != expected.input_identity {
        return Err(
            RealTranscriptDetectorSnapshotAdapterContractError::AnalysisIdentityMismatch {
                field: DetectorSnapshotAdapterAnalysisField::InputIdentity,
            },
        );
    }
    if derived.session_terms_identity != expected.session_terms_identity {
        return Err(
            RealTranscriptDetectorSnapshotAdapterContractError::AnalysisIdentityMismatch {
                field: DetectorSnapshotAdapterAnalysisField::SessionTermsIdentity,
            },
        );
    }
    if derived.detector_set != expected.detector_set {
        return Err(
            RealTranscriptDetectorSnapshotAdapterContractError::AnalysisIdentityMismatch {
                field: DetectorSnapshotAdapterAnalysisField::DetectorSet,
            },
        );
    }
    if derived.detector_config != expected.detector_config {
        return Err(
            RealTranscriptDetectorSnapshotAdapterContractError::AnalysisIdentityMismatch {
                field: DetectorSnapshotAdapterAnalysisField::DetectorConfig,
            },
        );
    }
    if derived.algorithm != expected.algorithm {
        return Err(
            RealTranscriptDetectorSnapshotAdapterContractError::AnalysisIdentityMismatch {
                field: DetectorSnapshotAdapterAnalysisField::Algorithm,
            },
        );
    }
    Ok(())
}

fn validate_proposal_id_inventory(
    adapter_request: &RealTranscriptDetectorSnapshotAdapterRequest,
    canonical_run: &CanonicalTermReviewRun,
) -> Result<(), RealTranscriptDetectorSnapshotAdapterContractError> {
    let review_cases = canonical_run.review_cases();
    if adapter_request.proposal_ids.len() != review_cases.len() {
        return Err(
            RealTranscriptDetectorSnapshotAdapterContractError::ProposalIdCountMismatch {
                expected: review_cases.len(),
                found: adapter_request.proposal_ids.len(),
            },
        );
    }

    let mut seen = std::collections::HashSet::new();
    for (index, proposal_id) in adapter_request.proposal_ids.iter().enumerate() {
        DetectorProposalId::new(proposal_id.as_str()).map_err(|_| {
            RealTranscriptDetectorSnapshotAdapterContractError::InvalidProposalId { index }
        })?;
        if !seen.insert(proposal_id.clone()) {
            return Err(
                RealTranscriptDetectorSnapshotAdapterContractError::DuplicateProposalId {
                    proposal_id: proposal_id.clone(),
                },
            );
        }
    }

    for (index, review_case) in review_cases.iter().enumerate() {
        if review_case.id().local_index() != index {
            return Err(
                RealTranscriptDetectorSnapshotAdapterContractError::ReviewCaseLocalIndexMismatch {
                    expected: index,
                    found: review_case.id().local_index(),
                },
            );
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FutureDetectorProposalSemanticKey {
    detector_id: String,
    detection_kind: DetectionKind,
    input_identity: InputIdentityReference,
    cue_id: CueReferenceId,
    segment_position: u32,
    start_byte: u32,
    end_byte: u32,
}

fn validate_review_case_mappings(
    transcript: &Transcript,
    canonical_run: &CanonicalTermReviewRun,
    validated_analysis_identity: &DetectorAnalysisIdentity,
) -> Result<(), RealTranscriptDetectorSnapshotAdapterContractError> {
    let mut seen_semantic_keys: Vec<(FutureDetectorProposalSemanticKey, usize)> = Vec::new();
    for (index, review_case) in canonical_run.review_cases().iter().enumerate() {
        let semantic_key = validate_single_review_case_mapping(
            transcript,
            review_case,
            index,
            validated_analysis_identity,
        )?;
        if let Some((_, first_review_case_index)) = seen_semantic_keys
            .iter()
            .find(|(key, _)| key == &semantic_key)
        {
            return Err(
                RealTranscriptDetectorSnapshotAdapterContractError::DuplicateFutureProposalSemanticKey {
                    first_review_case_index: *first_review_case_index,
                    duplicate_review_case_index: index,
                },
            );
        }
        seen_semantic_keys.push((semantic_key, index));
    }
    Ok(())
}

fn validate_single_review_case_mapping(
    transcript: &Transcript,
    review_case: &ReviewCase,
    review_case_index: usize,
    validated_analysis_identity: &DetectorAnalysisIdentity,
) -> Result<FutureDetectorProposalSemanticKey, RealTranscriptDetectorSnapshotAdapterContractError> {
    let candidate = review_case.candidate_span();
    let anchor = candidate.anchor();

    if anchor.revision != transcript.revision_id() {
        return Err(
            RealTranscriptDetectorSnapshotAdapterContractError::AnchorRevisionMismatch {
                review_case_index,
            },
        );
    }

    let segment_position = anchor.segment_position;
    if transcript.segments().get(segment_position).is_none() {
        return Err(
            RealTranscriptDetectorSnapshotAdapterContractError::SegmentPositionOutOfRange {
                review_case_index,
                segment_position,
            },
        );
    }

    let cue_index = transcript.segments()[segment_position].index();
    let cue_id = CueReferenceId::new(cue_index).map_err(|_| {
        RealTranscriptDetectorSnapshotAdapterContractError::CueReferenceIdInvalid {
            review_case_index,
            cue_index,
        }
    })?;

    let segment_position_u32 =
        u32_from_usize(segment_position, review_case_index, "segment_position")?;
    let start_byte_u32 = u32_from_usize(anchor.start_byte, review_case_index, "start_byte")?;
    let end_byte_u32 = u32_from_usize(anchor.end_byte, review_case_index, "end_byte")?;
    if anchor.start_byte >= anchor.end_byte {
        return Err(
            RealTranscriptDetectorSnapshotAdapterContractError::EmptyObservedSurface {
                review_case_index,
            },
        );
    }

    let observed_surface = transcript.resolve(anchor).ok_or(
        RealTranscriptDetectorSnapshotAdapterContractError::ObservedSurfaceResolutionFailure {
            review_case_index,
        },
    )?;
    if observed_surface.is_empty() {
        return Err(
            RealTranscriptDetectorSnapshotAdapterContractError::EmptyObservedSurface {
                review_case_index,
            },
        );
    }

    let provenance = candidate.provenance();
    let detector_component = DetectorComponentIdentity {
        id: provenance.detector_id().to_string(),
        version: provenance.detector_version().to_string(),
    };
    detector_component.validate().map_err(|_| {
        RealTranscriptDetectorSnapshotAdapterContractError::DetectorComponentInvalid {
            review_case_index,
        }
    })?;
    if !validated_analysis_identity
        .detector_set
        .iter()
        .any(|entry| entry == &detector_component)
    {
        return Err(
            RealTranscriptDetectorSnapshotAdapterContractError::DetectorNotInAnalysisSet {
                review_case_index,
            },
        );
    }

    validate_candidate_evidence_mapping(
        candidate.kind(),
        candidate.evidence(),
        observed_surface,
        review_case_index,
        validated_analysis_identity,
    )?;

    for (alternative_index, _alternative) in candidate.alternatives().iter().enumerate() {
        u32_from_usize(alternative_index, review_case_index, "alternative_index").map_err(|_| {
            RealTranscriptDetectorSnapshotAdapterContractError::AlternativeIndexConversionOverflow {
                review_case_index,
                alternative_index,
            }
        })?;
    }

    Ok(FutureDetectorProposalSemanticKey {
        detector_id: detector_component.id,
        detection_kind: candidate.kind(),
        input_identity: validated_analysis_identity.input_identity.clone(),
        cue_id,
        segment_position: segment_position_u32,
        start_byte: start_byte_u32,
        end_byte: end_byte_u32,
    })
}

fn validate_candidate_evidence_mapping(
    kind: DetectionKind,
    evidence: &Evidence,
    observed_surface: &str,
    review_case_index: usize,
    validated_analysis_identity: &DetectorAnalysisIdentity,
) -> Result<(), RealTranscriptDetectorSnapshotAdapterContractError> {
    match evidence {
        Evidence::GlossaryAlias(evidence_data) => {
            if kind != DetectionKind::GlossaryAliasMatch {
                return Err(
                    RealTranscriptDetectorSnapshotAdapterContractError::CandidateKindEvidenceMismatch {
                        review_case_index,
                    },
                );
            }
            if evidence_data.matched_form != observed_surface {
                return Err(
                    RealTranscriptDetectorSnapshotAdapterContractError::ObservedSurfaceResolutionFailure {
                        review_case_index,
                    },
                );
            }
        }
        Evidence::ObservedErrorForm(evidence_data) => {
            if kind != DetectionKind::GlossaryAliasMatch {
                return Err(
                    RealTranscriptDetectorSnapshotAdapterContractError::CandidateKindEvidenceMismatch {
                        review_case_index,
                    },
                );
            }
            if evidence_data.matched_form != observed_surface {
                return Err(
                    RealTranscriptDetectorSnapshotAdapterContractError::ObservedSurfaceResolutionFailure {
                        review_case_index,
                    },
                );
            }
        }
        Evidence::PhoneticSimilarity(phonetic) => {
            validate_phonetic_evidence_mapping(
                kind,
                phonetic,
                observed_surface,
                review_case_index,
                validated_analysis_identity,
            )?;
        }
    }
    Ok(())
}

fn validate_phonetic_evidence_mapping(
    kind: DetectionKind,
    phonetic: &PhoneticSimilarityEvidence,
    observed_surface: &str,
    review_case_index: usize,
    validated_analysis_identity: &DetectorAnalysisIdentity,
) -> Result<(), RealTranscriptDetectorSnapshotAdapterContractError> {
    if kind != DetectionKind::PhoneticSimilarity {
        return Err(
            RealTranscriptDetectorSnapshotAdapterContractError::CandidateKindEvidenceMismatch {
                review_case_index,
            },
        );
    }
    if phonetic.observed_surface != observed_surface {
        return Err(
            RealTranscriptDetectorSnapshotAdapterContractError::PhoneticObservedSurfaceMismatch {
                review_case_index,
            },
        );
    }
    if !component_matches_config(
        &validated_analysis_identity.detector_config,
        &phonetic.detector_config,
    ) {
        return Err(
            RealTranscriptDetectorSnapshotAdapterContractError::PhoneticDetectorConfigMismatch {
                review_case_index,
            },
        );
    }
    if !component_matches_algorithm(&validated_analysis_identity.algorithm, &phonetic.algorithm) {
        return Err(
            RealTranscriptDetectorSnapshotAdapterContractError::PhoneticAlgorithmMismatch {
                review_case_index,
            },
        );
    }

    u32_from_usize(
        phonetic.comparison.edit_distance,
        review_case_index,
        "edit_distance",
    )
    .map_err(|_| {
        RealTranscriptDetectorSnapshotAdapterContractError::PhoneticComparisonConversionOverflow {
            review_case_index,
            field: "edit_distance",
        }
    })?;
    let ratio_numerator = u32_from_usize(
        phonetic.comparison.ratio_numerator,
        review_case_index,
        "ratio_numerator",
    )
    .map_err(|_| {
        RealTranscriptDetectorSnapshotAdapterContractError::PhoneticComparisonConversionOverflow {
            review_case_index,
            field: "ratio_numerator",
        }
    })?;
    let ratio_denominator = u32_from_usize(
        phonetic.comparison.ratio_denominator,
        review_case_index,
        "ratio_denominator",
    )
    .map_err(|_| {
        RealTranscriptDetectorSnapshotAdapterContractError::PhoneticComparisonConversionOverflow {
            review_case_index,
            field: "ratio_denominator",
        }
    })?;
    let ratio_permille = u32_from_usize(
        phonetic.comparison.ratio_permille,
        review_case_index,
        "ratio_permille",
    )
    .map_err(|_| {
        RealTranscriptDetectorSnapshotAdapterContractError::PhoneticComparisonConversionOverflow {
            review_case_index,
            field: "ratio_permille",
        }
    })?;

    if ratio_denominator == 0 {
        return Err(
            RealTranscriptDetectorSnapshotAdapterContractError::PhoneticZeroRatioDenominator {
                review_case_index,
            },
        );
    }

    let expected_permille = ratio_numerator as u64 * 1000 / ratio_denominator as u64;
    let expected_permille_u32 = u32::try_from(expected_permille).map_err(|_| {
        RealTranscriptDetectorSnapshotAdapterContractError::PhoneticComparisonConversionOverflow {
            review_case_index,
            field: "expected_ratio_permille",
        }
    })?;
    if ratio_permille != expected_permille_u32 {
        return Err(
            RealTranscriptDetectorSnapshotAdapterContractError::PhoneticInconsistentRatioPermille {
                review_case_index,
                expected: expected_permille_u32,
                found: ratio_permille,
            },
        );
    }

    Ok(())
}

fn component_matches_config(
    component: &DetectorComponentIdentity,
    config: &DetectorConfigIdentity,
) -> bool {
    component.id == config.id() && component.version == config.version()
}

fn component_matches_algorithm(
    component: &DetectorComponentIdentity,
    algorithm: &AlgorithmIdentity,
) -> bool {
    component.id == algorithm.id() && component.version == algorithm.version()
}

fn u32_from_usize(
    value: usize,
    review_case_index: usize,
    field: &'static str,
) -> Result<u32, RealTranscriptDetectorSnapshotAdapterContractError> {
    value.try_into().map_err(|_| {
        RealTranscriptDetectorSnapshotAdapterContractError::IntegerConversionOverflow {
            review_case_index,
            field,
        }
    })
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::analysis::AnalysisRun;
    use crate::anchor::SourceAnchor;
    use crate::candidate::{
        AsciiLatinPhoneticRepresentation, CANONICAL_SESSION_TERM_ALGORITHM,
        CANONICAL_SESSION_TERM_DETECTOR_CONFIG, CandidateAlternative, CandidateSpan,
        DetectorProvenance, Evidence, GLOSSARY_DETECTOR, GlossaryAliasEvidence, PHONETIC_DETECTOR,
        PhoneticComparisonFacts, PhoneticSimilarityEvidence, PhoneticTargetKind, SessionTermEntry,
    };
    use crate::pipeline::CanonicalTermReviewRun;
    use crate::review::{ReviewCase, ReviewCaseId};
    use crate::transcript::{Segment, Transcript};

    fn adapter_request_for_case_count(
        proposal_count: usize,
    ) -> RealTranscriptDetectorSnapshotAdapterRequest {
        RealTranscriptDetectorSnapshotAdapterRequest {
            schema_revision: REAL_TRANSCRIPT_DETECTOR_SNAPSHOT_ADAPTER_REQUEST_SCHEMA.to_string(),
            adapter_policy_revision: REAL_TRANSCRIPT_DETECTOR_SNAPSHOT_ADAPTER_POLICY.to_string(),
            proposal_id_policy_revision: REAL_TRANSCRIPT_DETECTOR_PROPOSAL_ID_POLICY.to_string(),
            snapshot_revision: DetectorSnapshotRevisionId::new("snap-rev-unit").expect("rev"),
            detector_output_artifact_id: ArtifactId::new("artifact-det-out-unit")
                .expect("artifact"),
            frozen_at_unix_ms: 1_700_000_000_000,
            proposal_ids: (0..proposal_count)
                .map(|index| {
                    DetectorProposalId::new(format!("det-prop-unit-{index:03}")).expect("id")
                })
                .collect(),
        }
    }

    fn glossary_candidate(transcript: &Transcript) -> CandidateSpan {
        let anchor = transcript.anchor(0, 0, 8).expect("anchor");
        CandidateSpan::new(
            DetectionKind::GlossaryAliasMatch,
            DetectorProvenance::from_detector_identity(GLOSSARY_DETECTOR),
            anchor,
            Evidence::GlossaryAlias(GlossaryAliasEvidence {
                entry: SessionTermEntry::new("PostgreSQL", vec!["Postgres".to_string()], vec![]),
                matched_form: "Postgres".to_string(),
            }),
            vec![CandidateAlternative::new("PostgreSQL")],
        )
    }

    fn validated_analysis_identity_for(
        canonical_run: &CanonicalTermReviewRun,
    ) -> DetectorAnalysisIdentity {
        derive_detector_analysis_identity_from_canonical_run(canonical_run).expect("identity")
    }

    #[test]
    fn u32_from_usize_rejects_max_value() {
        let error = u32_from_usize(usize::MAX, 0, "segment_position").expect_err("overflow");
        assert!(matches!(
            error,
            RealTranscriptDetectorSnapshotAdapterContractError::IntegerConversionOverflow {
                field: "segment_position",
                ..
            }
        ));
    }

    #[test]
    fn review_case_local_index_mismatch_rejected() {
        let transcript = Transcript::from_segments(vec![Segment {
            index: 1,
            start_ms: 0,
            end_ms: 1_000,
            text: "Postgres".to_string(),
        }]);
        let run = AnalysisRun::for_canonical_session_terms(&transcript, &[]);
        let canonical_run = CanonicalTermReviewRun::new(
            run,
            vec![ReviewCase::detector_raised(
                ReviewCaseId::local(1),
                glossary_candidate(&transcript),
            )],
        );
        let adapter_request = adapter_request_for_case_count(1);
        assert!(matches!(
            validate_proposal_id_inventory(&adapter_request, &canonical_run),
            Err(
                RealTranscriptDetectorSnapshotAdapterContractError::ReviewCaseLocalIndexMismatch {
                    expected: 0,
                    found: 1,
                }
            )
        ));
    }

    #[test]
    fn cue_index_zero_rejected() {
        let transcript = Transcript::from_segments(vec![Segment {
            index: 0,
            start_ms: 0,
            end_ms: 1_000,
            text: "Postgres".to_string(),
        }]);
        let run = AnalysisRun::for_canonical_session_terms(&transcript, &[]);
        let anchor = SourceAnchor {
            revision: transcript.revision_id(),
            segment_position: 0,
            start_byte: 0,
            end_byte: 8,
        };
        let candidate = CandidateSpan::new(
            DetectionKind::GlossaryAliasMatch,
            DetectorProvenance::from_detector_identity(GLOSSARY_DETECTOR),
            anchor,
            Evidence::GlossaryAlias(GlossaryAliasEvidence {
                entry: SessionTermEntry::new("PostgreSQL", vec!["Postgres".to_string()], vec![]),
                matched_form: "Postgres".to_string(),
            }),
            vec![],
        );
        let canonical_run = CanonicalTermReviewRun::new(
            run,
            vec![ReviewCase::detector_raised(
                ReviewCaseId::local(0),
                candidate,
            )],
        );
        let analysis_identity = validated_analysis_identity_for(&canonical_run);
        assert!(matches!(
            validate_review_case_mappings(&transcript, &canonical_run, &analysis_identity),
            Err(
                RealTranscriptDetectorSnapshotAdapterContractError::CueReferenceIdInvalid {
                    cue_index: 0,
                    ..
                }
            )
        ));
    }

    #[test]
    fn cue_id_derives_from_segment_index_not_position_plus_one() {
        let transcript = Transcript::from_segments(vec![Segment {
            index: 7,
            start_ms: 0,
            end_ms: 1_000,
            text: "Postgres".to_string(),
        }]);
        let run = AnalysisRun::for_canonical_session_terms(&transcript, &[]);
        let anchor = SourceAnchor {
            revision: transcript.revision_id(),
            segment_position: 0,
            start_byte: 0,
            end_byte: 8,
        };
        let candidate = CandidateSpan::new(
            DetectionKind::GlossaryAliasMatch,
            DetectorProvenance::from_detector_identity(GLOSSARY_DETECTOR),
            anchor,
            Evidence::GlossaryAlias(GlossaryAliasEvidence {
                entry: SessionTermEntry::new("PostgreSQL", vec!["Postgres".to_string()], vec![]),
                matched_form: "Postgres".to_string(),
            }),
            vec![],
        );
        let canonical_run = CanonicalTermReviewRun::new(
            run,
            vec![ReviewCase::detector_raised(
                ReviewCaseId::local(0),
                candidate,
            )],
        );
        let cue_index = transcript.segments()[0].index();
        assert_eq!(cue_index, 7);
        assert_ne!(cue_index, 1);
        let analysis_identity = validated_analysis_identity_for(&canonical_run);
        validate_review_case_mappings(&transcript, &canonical_run, &analysis_identity)
            .expect("segment index authority");
    }

    #[test]
    fn phonetic_observed_surface_mismatch_rejected() {
        let transcript = Transcript::from_segments(vec![Segment {
            index: 1,
            start_ms: 0,
            end_ms: 1_000,
            text: "Postgre SQL".to_string(),
        }]);
        let run = AnalysisRun::for_canonical_session_terms(&transcript, &[]);
        let anchor = transcript.anchor(0, 0, 5).expect("anchor");
        let candidate = CandidateSpan::new(
            DetectionKind::PhoneticSimilarity,
            DetectorProvenance::from_detector_identity(GLOSSARY_DETECTOR),
            anchor,
            Evidence::PhoneticSimilarity(PhoneticSimilarityEvidence {
                observed_surface: "wrong-surface".to_string(),
                target_surface: "Postgres".to_string(),
                target_kind: PhoneticTargetKind::Alias,
                canonical_term: "PostgreSQL".to_string(),
                source_representation: AsciiLatinPhoneticRepresentation {
                    normalized_letters: "PSTR".to_string(),
                    primary_key: "PSTR".to_string(),
                    alternate_key: "PSTR".to_string(),
                },
                target_representation: AsciiLatinPhoneticRepresentation {
                    normalized_letters: "PSTR".to_string(),
                    primary_key: "PSTR".to_string(),
                    alternate_key: "PSTR".to_string(),
                },
                comparison: PhoneticComparisonFacts {
                    edit_distance: 0,
                    ratio_numerator: 1,
                    ratio_denominator: 1,
                    ratio_permille: 1000,
                    matched_key: "PSTR".to_string(),
                },
                detector_config: CANONICAL_SESSION_TERM_DETECTOR_CONFIG,
                algorithm: CANONICAL_SESSION_TERM_ALGORITHM,
            }),
            vec![],
        );
        let canonical_run = CanonicalTermReviewRun::new(
            run,
            vec![ReviewCase::detector_raised(
                ReviewCaseId::local(0),
                candidate,
            )],
        );
        let analysis_identity = validated_analysis_identity_for(&canonical_run);
        assert!(matches!(
            validate_review_case_mappings(&transcript, &canonical_run, &analysis_identity),
            Err(RealTranscriptDetectorSnapshotAdapterContractError::PhoneticObservedSurfaceMismatch {
                review_case_index: 0,
            })
        ));
    }

    #[test]
    fn phonetic_detector_config_mismatch_rejected() {
        let transcript = Transcript::from_segments(vec![Segment {
            index: 1,
            start_ms: 0,
            end_ms: 1_000,
            text: "Postgre SQL".to_string(),
        }]);
        let run = AnalysisRun::for_canonical_session_terms(&transcript, &[]);
        let anchor = transcript.anchor(0, 0, 5).expect("anchor");
        let resolved = transcript.resolve(&anchor).expect("surface");
        let mut phonetic = PhoneticSimilarityEvidence {
            observed_surface: resolved.to_string(),
            target_surface: "Postgres".to_string(),
            target_kind: PhoneticTargetKind::Alias,
            canonical_term: "PostgreSQL".to_string(),
            source_representation: AsciiLatinPhoneticRepresentation {
                normalized_letters: "PSTR".to_string(),
                primary_key: "PSTR".to_string(),
                alternate_key: "PSTR".to_string(),
            },
            target_representation: AsciiLatinPhoneticRepresentation {
                normalized_letters: "PSTR".to_string(),
                primary_key: "PSTR".to_string(),
                alternate_key: "PSTR".to_string(),
            },
            comparison: PhoneticComparisonFacts {
                edit_distance: 0,
                ratio_numerator: 1,
                ratio_denominator: 1,
                ratio_permille: 1000,
                matched_key: "PSTR".to_string(),
            },
            detector_config: CANONICAL_SESSION_TERM_DETECTOR_CONFIG,
            algorithm: CANONICAL_SESSION_TERM_ALGORITHM,
        };
        phonetic.detector_config =
            crate::analysis::DetectorConfigIdentity::new("tampered-config", "0.0.1");
        let candidate = CandidateSpan::new(
            DetectionKind::PhoneticSimilarity,
            DetectorProvenance::from_detector_identity(GLOSSARY_DETECTOR),
            anchor,
            Evidence::PhoneticSimilarity(phonetic),
            vec![],
        );
        let canonical_run = CanonicalTermReviewRun::new(
            run,
            vec![ReviewCase::detector_raised(
                ReviewCaseId::local(0),
                candidate,
            )],
        );
        let analysis_identity = validated_analysis_identity_for(&canonical_run);
        assert!(matches!(
            validate_review_case_mappings(&transcript, &canonical_run, &analysis_identity),
            Err(RealTranscriptDetectorSnapshotAdapterContractError::PhoneticDetectorConfigMismatch {
                review_case_index: 0,
            })
        ));
    }

    #[test]
    fn phonetic_algorithm_mismatch_rejected() {
        let transcript = Transcript::from_segments(vec![Segment {
            index: 1,
            start_ms: 0,
            end_ms: 1_000,
            text: "Postgre SQL".to_string(),
        }]);
        let run = AnalysisRun::for_canonical_session_terms(&transcript, &[]);
        let anchor = transcript.anchor(0, 0, 5).expect("anchor");
        let resolved = transcript.resolve(&anchor).expect("surface");
        let mut phonetic = PhoneticSimilarityEvidence {
            observed_surface: resolved.to_string(),
            target_surface: "Postgres".to_string(),
            target_kind: PhoneticTargetKind::Alias,
            canonical_term: "PostgreSQL".to_string(),
            source_representation: AsciiLatinPhoneticRepresentation {
                normalized_letters: "PSTR".to_string(),
                primary_key: "PSTR".to_string(),
                alternate_key: "PSTR".to_string(),
            },
            target_representation: AsciiLatinPhoneticRepresentation {
                normalized_letters: "PSTR".to_string(),
                primary_key: "PSTR".to_string(),
                alternate_key: "PSTR".to_string(),
            },
            comparison: PhoneticComparisonFacts {
                edit_distance: 0,
                ratio_numerator: 1,
                ratio_denominator: 1,
                ratio_permille: 1000,
                matched_key: "PSTR".to_string(),
            },
            detector_config: CANONICAL_SESSION_TERM_DETECTOR_CONFIG,
            algorithm: CANONICAL_SESSION_TERM_ALGORITHM,
        };
        phonetic.algorithm = crate::analysis::AlgorithmIdentity::new("tampered-algorithm", "0.0.1");
        let candidate = CandidateSpan::new(
            DetectionKind::PhoneticSimilarity,
            DetectorProvenance::from_detector_identity(GLOSSARY_DETECTOR),
            anchor,
            Evidence::PhoneticSimilarity(phonetic),
            vec![],
        );
        let canonical_run = CanonicalTermReviewRun::new(
            run,
            vec![ReviewCase::detector_raised(
                ReviewCaseId::local(0),
                candidate,
            )],
        );
        let analysis_identity = validated_analysis_identity_for(&canonical_run);
        assert!(matches!(
            validate_review_case_mappings(&transcript, &canonical_run, &analysis_identity),
            Err(
                RealTranscriptDetectorSnapshotAdapterContractError::PhoneticAlgorithmMismatch {
                    review_case_index: 0,
                }
            )
        ));
    }

    fn valid_phonetic_candidate(
        transcript: &Transcript,
        comparison: PhoneticComparisonFacts,
    ) -> CandidateSpan {
        let anchor = transcript.anchor(0, 0, 5).expect("anchor");
        let resolved = transcript.resolve(&anchor).expect("surface");
        CandidateSpan::new(
            DetectionKind::PhoneticSimilarity,
            DetectorProvenance::from_detector_identity(PHONETIC_DETECTOR),
            anchor,
            Evidence::PhoneticSimilarity(PhoneticSimilarityEvidence {
                observed_surface: resolved.to_string(),
                target_surface: "Postgres".to_string(),
                target_kind: PhoneticTargetKind::Alias,
                canonical_term: "PostgreSQL".to_string(),
                source_representation: AsciiLatinPhoneticRepresentation {
                    normalized_letters: "PSTR".to_string(),
                    primary_key: "PSTR".to_string(),
                    alternate_key: "PSTR".to_string(),
                },
                target_representation: AsciiLatinPhoneticRepresentation {
                    normalized_letters: "PSTR".to_string(),
                    primary_key: "PSTR".to_string(),
                    alternate_key: "PSTR".to_string(),
                },
                comparison,
                detector_config: CANONICAL_SESSION_TERM_DETECTOR_CONFIG,
                algorithm: CANONICAL_SESSION_TERM_ALGORITHM,
            }),
            vec![],
        )
    }

    #[test]
    fn phonetic_zero_ratio_denominator_rejected() {
        let transcript = Transcript::from_segments(vec![Segment {
            index: 1,
            start_ms: 0,
            end_ms: 1_000,
            text: "Postgre SQL".to_string(),
        }]);
        let run = AnalysisRun::for_canonical_session_terms(&transcript, &[]);
        let candidate = valid_phonetic_candidate(
            &transcript,
            PhoneticComparisonFacts {
                edit_distance: 0,
                ratio_numerator: 1,
                ratio_denominator: 0,
                ratio_permille: 0,
                matched_key: "PSTR".to_string(),
            },
        );
        let canonical_run = CanonicalTermReviewRun::new(
            run,
            vec![ReviewCase::detector_raised(
                ReviewCaseId::local(0),
                candidate,
            )],
        );
        let analysis_identity = validated_analysis_identity_for(&canonical_run);
        assert!(matches!(
            validate_review_case_mappings(&transcript, &canonical_run, &analysis_identity),
            Err(
                RealTranscriptDetectorSnapshotAdapterContractError::PhoneticZeroRatioDenominator {
                    review_case_index: 0,
                }
            )
        ));
    }

    #[test]
    fn phonetic_inconsistent_ratio_permille_rejected() {
        let transcript = Transcript::from_segments(vec![Segment {
            index: 1,
            start_ms: 0,
            end_ms: 1_000,
            text: "Postgre SQL".to_string(),
        }]);
        let run = AnalysisRun::for_canonical_session_terms(&transcript, &[]);
        let candidate = valid_phonetic_candidate(
            &transcript,
            PhoneticComparisonFacts {
                edit_distance: 0,
                ratio_numerator: 1,
                ratio_denominator: 2,
                ratio_permille: 999,
                matched_key: "PSTR".to_string(),
            },
        );
        let canonical_run = CanonicalTermReviewRun::new(
            run,
            vec![ReviewCase::detector_raised(
                ReviewCaseId::local(0),
                candidate,
            )],
        );
        let analysis_identity = validated_analysis_identity_for(&canonical_run);
        assert!(matches!(
            validate_review_case_mappings(&transcript, &canonical_run, &analysis_identity),
            Err(
                RealTranscriptDetectorSnapshotAdapterContractError::PhoneticInconsistentRatioPermille {
                    review_case_index: 0,
                    expected: 500,
                    found: 999,
                }
            )
        ));
    }

    #[test]
    fn duplicate_future_semantic_key_rejected() {
        let transcript = Transcript::from_segments(vec![Segment {
            index: 1,
            start_ms: 0,
            end_ms: 1_000,
            text: "Postgres".to_string(),
        }]);
        let run = AnalysisRun::for_canonical_session_terms(&transcript, &[]);
        let anchor = transcript.anchor(0, 0, 8).expect("anchor");
        let first = CandidateSpan::new(
            DetectionKind::GlossaryAliasMatch,
            DetectorProvenance::from_detector_identity(GLOSSARY_DETECTOR),
            anchor,
            Evidence::GlossaryAlias(GlossaryAliasEvidence {
                entry: SessionTermEntry::new("PostgreSQL", vec!["Postgres".to_string()], vec![]),
                matched_form: "Postgres".to_string(),
            }),
            vec![],
        );
        let second = CandidateSpan::new(
            DetectionKind::GlossaryAliasMatch,
            DetectorProvenance::from_detector_identity(GLOSSARY_DETECTOR),
            anchor,
            Evidence::GlossaryAlias(GlossaryAliasEvidence {
                entry: SessionTermEntry::new("PostgreSQL", vec!["Postgres".to_string()], vec![]),
                matched_form: "Postgres".to_string(),
            }),
            vec![CandidateAlternative::new("PostgreSQL")],
        );
        let canonical_run = CanonicalTermReviewRun::new(
            run,
            vec![
                ReviewCase::detector_raised(ReviewCaseId::local(0), first),
                ReviewCase::detector_raised(ReviewCaseId::local(1), second),
            ],
        );
        let analysis_identity = validated_analysis_identity_for(&canonical_run);
        assert!(matches!(
            validate_review_case_mappings(&transcript, &canonical_run, &analysis_identity),
            Err(
                RealTranscriptDetectorSnapshotAdapterContractError::DuplicateFutureProposalSemanticKey {
                    first_review_case_index: 0,
                    duplicate_review_case_index: 1,
                }
            )
        ));
        let adapter_request = adapter_request_for_case_count(2);
        validate_proposal_id_inventory(&adapter_request, &canonical_run)
            .expect("distinct explicit proposal ids");
        assert!(matches!(
            validate_review_case_mappings(&transcript, &canonical_run, &analysis_identity),
            Err(
                RealTranscriptDetectorSnapshotAdapterContractError::DuplicateFutureProposalSemanticKey {
                    first_review_case_index: 0,
                    duplicate_review_case_index: 1,
                }
            )
        ));
    }

    #[test]
    fn distinct_future_semantic_keys_accepted() {
        let transcript = Transcript::from_segments(vec![
            Segment {
                index: 1,
                start_ms: 0,
                end_ms: 1_000,
                text: "Postgres".to_string(),
            },
            Segment {
                index: 2,
                start_ms: 1_000,
                end_ms: 2_000,
                text: "Kafka".to_string(),
            },
        ]);
        let run = AnalysisRun::for_canonical_session_terms(&transcript, &[]);
        let anchor_a = transcript.anchor(0, 0, 8).expect("anchor a");
        let anchor_b = transcript.anchor(1, 0, 5).expect("anchor b");
        let first = CandidateSpan::new(
            DetectionKind::GlossaryAliasMatch,
            DetectorProvenance::from_detector_identity(GLOSSARY_DETECTOR),
            anchor_a,
            Evidence::GlossaryAlias(GlossaryAliasEvidence {
                entry: SessionTermEntry::new("PostgreSQL", vec!["Postgres".to_string()], vec![]),
                matched_form: "Postgres".to_string(),
            }),
            vec![],
        );
        let second = CandidateSpan::new(
            DetectionKind::GlossaryAliasMatch,
            DetectorProvenance::from_detector_identity(GLOSSARY_DETECTOR),
            anchor_b,
            Evidence::GlossaryAlias(GlossaryAliasEvidence {
                entry: SessionTermEntry::new("Kafka", vec![], vec![]),
                matched_form: "Kafka".to_string(),
            }),
            vec![],
        );
        let canonical_run = CanonicalTermReviewRun::new(
            run,
            vec![
                ReviewCase::detector_raised(ReviewCaseId::local(0), first),
                ReviewCase::detector_raised(ReviewCaseId::local(1), second),
            ],
        );
        let analysis_identity = validated_analysis_identity_for(&canonical_run);
        validate_review_case_mappings(&transcript, &canonical_run, &analysis_identity)
            .expect("distinct semantic keys");
    }
}

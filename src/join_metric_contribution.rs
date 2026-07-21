#![allow(clippy::too_many_arguments)]

use std::collections::HashSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::artifact_bundle::{ArtifactBundle, ArtifactBundleValidationError, ArtifactId};
use crate::detector_reference_join::{
    DetectorReferenceJoin, DetectorReferenceJoinError, DetectorReferenceJoinId,
    DetectorReferenceJoinPurpose, DetectorReferenceJoinRevisionId, DetectorReferenceJoinState,
    DetectorReferenceMatchDisposition, JoinIdentityIdError, ReferenceJoinEligibility,
    validate_join_id_value,
};
use crate::detector_snapshot::{
    DetectorProposalSnapshot, DetectorProposalSnapshotState,
    DetectorProposalSnapshotValidationError, DetectorSnapshotRevisionId,
};
use crate::human_final_reference::{
    HumanFinalReference, HumanFinalReferenceValidationError, ReferenceClass, ReferenceErrorId,
    ReferenceErrorRecord, VerificationBasis,
};
use crate::join_adjudication::{OverlapAdjudicationSet, OverlapAdjudicationValidationError};
use crate::reference_coverage::{
    ReferenceCoverage, ReferenceCoverageId, ReferenceCoveragePurpose, ReferenceCoverageState,
    ReferenceCoverageValidationError,
};
use crate::reference_seal::{
    CalibrationValidityImpact, ReferenceCalibrationValidity, ReferenceSeal, ReferenceSealId,
    ReferenceSealValidationError,
};
use crate::run_manifest::{
    ArtifactRole, CalibrationValidityMode, InputClass, InputIdentityReference, RunEnvelope,
    RunEnvelopeValidationError, RunId, RunIdError, RunLifecycleState, validate_opaque_identifier,
};

pub const JOIN_METRIC_CONTRIBUTION_SCHEMA: &str = "voxproof-join-metric-contributions-v1";
pub const PRIMARY_METRIC_ELIGIBILITY_POLICY: &str = "voxproof-primary-metric-eligibility-v1";
pub const METRIC_CONTRIBUTION_POLICY: &str = "voxproof-metric-contribution-v1";

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MetricContributionSetId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MetricContributionRevisionId(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimaryMetricKind {
    ProposalPrecision,
    ErrorLocalizationRecall,
    CorrectionExactnessGivenLocalization,
    EndToEndExactCorrectionRecall,
    DuplicateProposalBurden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricContributionReportClass {
    PrimaryBlindCalibration,
    NonCalibrationDiagnostic,
    SyntheticProtocolValidation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimaryMetricBlockingReason {
    EnvelopeNotBlindReference,
    ReferenceNotBlindEligible,
    ReferenceValidityImpactNotNone,
    CoverageNotPrimary,
    CoverageIncomplete,
    JoinNotPrimary,
    JoinNotResolved,
    JoinNotOneToOne,
    JoinContainsUnresolvedOverlap,
    ContributionSetPending,
    SyntheticProtocolOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", content = "reason", rename_all = "snake_case")]
pub enum RatioContribution {
    NumeratorAndDenominator,
    DenominatorOnly,
    Excluded(MetricContributionExclusionReason),
    PendingAdjudication,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricContributionExclusionReason {
    AmbiguousMatch,
    ReferenceIneligible,
    JoinExcluded,
    NotLocalized,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetectorMetricContributionRecord {
    pub detector_proposal_id: crate::detector_snapshot::DetectorProposalId,
    pub join_disposition: DetectorReferenceMatchDisposition,
    pub proposal_precision: RatioContribution,
    pub duplicate_proposal_burden: RatioContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceMetricContributionRecord {
    pub reference_error_id: ReferenceErrorId,
    pub join_disposition: DetectorReferenceMatchDisposition,
    pub reference_eligibility: ReferenceJoinEligibility,
    pub error_localization_recall: RatioContribution,
    pub correction_exactness_given_localization: RatioContribution,
    pub end_to_end_exact_correction_recall: RatioContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimaryMetricEligibilityAssessment {
    pub policy_revision: String,
    pub report_class: MetricContributionReportClass,
    pub primary_metrics_allowed: bool,
    pub eligible_primary_metrics: Vec<PrimaryMetricKind>,
    pub blocking_reasons: Vec<PrimaryMetricBlockingReason>,
    pub qualifies_as_real_material_evidence: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricContributionSetState {
    PendingJoinResolution,
    Complete,
    Invalidated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricContributionSetAssessment {
    pub detector_source_count: u32,
    pub detector_contribution_count: u32,
    pub reference_source_count: u32,
    pub reference_contribution_count: u32,
    pub pending_detector_contribution_count: u32,
    pub pending_reference_contribution_count: u32,
    pub mapping_complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JoinMetricContributionSet {
    pub schema_revision: String,
    pub contribution_set_id: MetricContributionSetId,
    pub contribution_revision: MetricContributionRevisionId,
    pub run_id: RunId,
    pub input_identity: InputIdentityReference,
    pub input_class: InputClass,
    pub qualifies_as_real_material_evidence: bool,
    pub reference_seal_id: ReferenceSealId,
    pub reference_revision: crate::reference_identity::ReferenceRevisionId,
    pub reference_coverage_id: ReferenceCoverageId,
    pub detector_snapshot_revision: DetectorSnapshotRevisionId,
    pub detector_output_artifact_id: ArtifactId,
    pub join_id: DetectorReferenceJoinId,
    pub join_revision: DetectorReferenceJoinRevisionId,
    pub evaluation_join_artifact_id: ArtifactId,
    pub join_adjudication_artifact_id: ArtifactId,
    pub metric_contributions_artifact_id: ArtifactId,
    pub eligibility_policy_revision: String,
    pub contribution_policy_revision: String,
    pub state: MetricContributionSetState,
    pub eligibility: PrimaryMetricEligibilityAssessment,
    pub detector_contributions: Vec<DetectorMetricContributionRecord>,
    pub reference_contributions: Vec<ReferenceMetricContributionRecord>,
    pub assessment: MetricContributionSetAssessment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinMetricContributionContext {
    pub contribution_set_id: MetricContributionSetId,
    pub contribution_revision: MetricContributionRevisionId,
    pub metric_contributions_artifact_id: ArtifactId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetricContributionIdentityIdError {
    Empty,
    TooLong { len: usize, max: usize },
    InvalidCharacter { character: char },
    PathLikeContent,
    AbsolutePathLike,
    RelativePathLike,
    HomeDirectoryFragment,
    GenerationUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JoinMetricContributionError {
    MissingSchemaRevision,
    UnsupportedSchemaRevision {
        found: String,
        expected: String,
    },
    UnsupportedPolicyRevision {
        field: &'static str,
        found: String,
        expected: String,
    },
    InvalidContributionSetId(MetricContributionIdentityIdError),
    InvalidContributionRevisionId(MetricContributionIdentityIdError),
    EnvelopeValidation(RunEnvelopeValidationError),
    SealValidation(ReferenceSealValidationError),
    CoverageValidation(ReferenceCoverageValidationError),
    HumanReferenceValidation(Box<HumanFinalReferenceValidationError>),
    SnapshotValidation(DetectorProposalSnapshotValidationError),
    BundleValidation(ArtifactBundleValidationError),
    AdjudicationValidation(OverlapAdjudicationValidationError),
    JoinValidation(DetectorReferenceJoinError),
    RunIdMismatch,
    InputIdentityMismatch,
    ReferenceRevisionMismatch,
    CoverageIdMismatch,
    SnapshotRevisionMismatch,
    DetectorOutputArtifactMismatch,
    JoinIdMismatch,
    JoinRevisionMismatch,
    EvaluationJoinArtifactMismatch,
    JoinAdjudicationArtifactMismatch,
    MetricContributionsArtifactMismatch,
    TopLevelBindingMismatch {
        field: &'static str,
    },
    DuplicateDetectorContributionId {
        detector_proposal_id: crate::detector_snapshot::DetectorProposalId,
    },
    DuplicateReferenceContributionId {
        reference_error_id: ReferenceErrorId,
    },
    SideIncompatibleDetectorDisposition {
        disposition: DetectorReferenceMatchDisposition,
    },
    SideIncompatibleReferenceDisposition {
        disposition: DetectorReferenceMatchDisposition,
    },
    ReferenceEligibilityDispositionMismatch {
        reference_error_id: ReferenceErrorId,
        eligibility: ReferenceJoinEligibility,
        disposition: DetectorReferenceMatchDisposition,
    },
    AssessmentMismatch {
        stored: Box<MetricContributionSetAssessment>,
        derived: Box<MetricContributionSetAssessment>,
    },
    EligibilityMismatch {
        stored: Box<PrimaryMetricEligibilityAssessment>,
        derived: Box<PrimaryMetricEligibilityAssessment>,
    },
    ContributionSetStateMismatch {
        state: MetricContributionSetState,
        assessment: Box<MetricContributionSetAssessment>,
    },
    PrimaryEligibilityInconsistent,
    ReportClassInconsistent,
    PendingCompleteStateMismatch,
    PendingPrimaryEligibilityMismatch,
    InvalidatedContributionContext,
    ContributionCreationLifecycleIncompatible {
        lifecycle_state: RunLifecycleState,
    },
    ContributionHistoricalLifecycleIncompatible {
        lifecycle_state: RunLifecycleState,
    },
    SnapshotNotFrozen,
    ContributionSetInvalidated,
    StoredContributionMismatch,
}

impl MetricContributionSetId {
    pub fn new(value: impl Into<String>) -> Result<Self, MetricContributionIdentityIdError> {
        let value = value.into();
        validate_contribution_id_value(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl MetricContributionRevisionId {
    pub fn new(value: impl Into<String>) -> Result<Self, MetricContributionIdentityIdError> {
        let value = value.into();
        validate_contribution_id_value(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MetricContributionSetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for MetricContributionRevisionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl PrimaryMetricBlockingReason {
    fn canonical_order(self) -> u8 {
        match self {
            Self::EnvelopeNotBlindReference => 0,
            Self::ReferenceNotBlindEligible => 1,
            Self::ReferenceValidityImpactNotNone => 2,
            Self::CoverageNotPrimary => 3,
            Self::CoverageIncomplete => 4,
            Self::JoinNotPrimary => 5,
            Self::JoinNotResolved => 6,
            Self::JoinNotOneToOne => 7,
            Self::JoinContainsUnresolvedOverlap => 8,
            Self::ContributionSetPending => 9,
            Self::SyntheticProtocolOnly => 10,
        }
    }
}

impl PrimaryMetricKind {
    fn canonical_order(self) -> u8 {
        match self {
            Self::ProposalPrecision => 0,
            Self::ErrorLocalizationRecall => 1,
            Self::CorrectionExactnessGivenLocalization => 2,
            Self::EndToEndExactCorrectionRecall => 3,
            Self::DuplicateProposalBurden => 4,
        }
    }
}

fn all_primary_metric_kinds() -> Vec<PrimaryMetricKind> {
    vec![
        PrimaryMetricKind::ProposalPrecision,
        PrimaryMetricKind::ErrorLocalizationRecall,
        PrimaryMetricKind::CorrectionExactnessGivenLocalization,
        PrimaryMetricKind::EndToEndExactCorrectionRecall,
        PrimaryMetricKind::DuplicateProposalBurden,
    ]
}

fn sort_blocking_reasons(reasons: &mut Vec<PrimaryMetricBlockingReason>) {
    reasons.sort_by_key(|reason| reason.canonical_order());
    reasons.dedup();
}

fn sort_primary_metric_kinds(metrics: &mut [PrimaryMetricKind]) {
    metrics.sort_by_key(|metric| metric.canonical_order());
}

impl JoinMetricContributionSet {
    pub fn derive(
        context: &JoinMetricContributionContext,
        envelope: &RunEnvelope,
        seal: &ReferenceSeal,
        coverage: &ReferenceCoverage,
        human_reference: &HumanFinalReference,
        detector_snapshot: &DetectorProposalSnapshot,
        join: &DetectorReferenceJoin,
        adjudication_set: &OverlapAdjudicationSet,
        artifact_bundle: &ArtifactBundle,
    ) -> Result<Self, JoinMetricContributionError> {
        validate_contribution_creation_lifecycle(envelope.lifecycle_state)?;
        validate_contribution_inputs(
            envelope,
            seal,
            coverage,
            human_reference,
            detector_snapshot,
            join,
            adjudication_set,
            artifact_bundle,
            context,
            DeriveMode::Creation,
        )?;

        join.validate_against(
            envelope,
            seal,
            coverage,
            human_reference,
            detector_snapshot,
            artifact_bundle,
            adjudication_set,
        )
        .map_err(JoinMetricContributionError::JoinValidation)?;

        let set = derive_contribution_body(
            context,
            envelope,
            seal,
            coverage,
            human_reference,
            detector_snapshot,
            join,
        )?;

        set.validate_against(
            envelope,
            seal,
            coverage,
            human_reference,
            detector_snapshot,
            join,
            adjudication_set,
            artifact_bundle,
        )?;

        Ok(set)
    }

    pub fn validate(&self) -> Result<(), JoinMetricContributionError> {
        if self.schema_revision.is_empty() {
            return Err(JoinMetricContributionError::MissingSchemaRevision);
        }
        if self.schema_revision != JOIN_METRIC_CONTRIBUTION_SCHEMA {
            return Err(JoinMetricContributionError::UnsupportedSchemaRevision {
                found: self.schema_revision.clone(),
                expected: JOIN_METRIC_CONTRIBUTION_SCHEMA.to_string(),
            });
        }

        validate_policy_revision(
            "eligibility_policy_revision",
            &self.eligibility_policy_revision,
            PRIMARY_METRIC_ELIGIBILITY_POLICY,
        )?;
        validate_policy_revision(
            "contribution_policy_revision",
            &self.contribution_policy_revision,
            METRIC_CONTRIBUTION_POLICY,
        )?;

        validate_contribution_id_value(self.contribution_set_id.as_str())
            .map_err(JoinMetricContributionError::InvalidContributionSetId)?;
        validate_contribution_id_value(self.contribution_revision.as_str())
            .map_err(JoinMetricContributionError::InvalidContributionRevisionId)?;

        validate_opaque_identifier(self.run_id.as_str()).map_err(|error| {
            JoinMetricContributionError::InvalidContributionSetId(map_run_id_error(error))
        })?;

        let mut detector_ids = HashSet::new();
        for record in &self.detector_contributions {
            if !detector_ids.insert(record.detector_proposal_id.clone()) {
                return Err(
                    JoinMetricContributionError::DuplicateDetectorContributionId {
                        detector_proposal_id: record.detector_proposal_id.clone(),
                    },
                );
            }
            validate_detector_contribution_record(record)?;
        }

        let mut reference_ids = HashSet::new();
        for record in &self.reference_contributions {
            if !reference_ids.insert(record.reference_error_id.clone()) {
                return Err(
                    JoinMetricContributionError::DuplicateReferenceContributionId {
                        reference_error_id: record.reference_error_id.clone(),
                    },
                );
            }
            validate_reference_contribution_record(record)?;
        }

        let derived_assessment = derive_assessment(
            self.assessment.detector_source_count,
            self.assessment.reference_source_count,
            &self.detector_contributions,
            &self.reference_contributions,
        );
        if self.assessment != derived_assessment {
            return Err(JoinMetricContributionError::AssessmentMismatch {
                stored: Box::new(self.assessment.clone()),
                derived: Box::new(derived_assessment),
            });
        }

        validate_local_state_consistency(self)?;

        Ok(())
    }

    pub fn validate_against(
        &self,
        envelope: &RunEnvelope,
        seal: &ReferenceSeal,
        coverage: &ReferenceCoverage,
        human_reference: &HumanFinalReference,
        detector_snapshot: &DetectorProposalSnapshot,
        join: &DetectorReferenceJoin,
        adjudication_set: &OverlapAdjudicationSet,
        artifact_bundle: &ArtifactBundle,
    ) -> Result<(), JoinMetricContributionError> {
        self.validate()?;

        if self.state == MetricContributionSetState::Invalidated {
            return Err(JoinMetricContributionError::ContributionSetInvalidated);
        }

        let mode = if is_contribution_creation_lifecycle(envelope.lifecycle_state) {
            DeriveMode::Creation
        } else if is_contribution_historical_lifecycle(envelope.lifecycle_state) {
            DeriveMode::Historical
        } else {
            return Err(
                JoinMetricContributionError::ContributionHistoricalLifecycleIncompatible {
                    lifecycle_state: envelope.lifecycle_state,
                },
            );
        };

        let context = authoritative_contribution_context_for_validation(self, artifact_bundle)?;
        validate_contribution_inputs(
            envelope,
            seal,
            coverage,
            human_reference,
            detector_snapshot,
            join,
            adjudication_set,
            artifact_bundle,
            &context,
            mode,
        )?;

        join.validate_against(
            envelope,
            seal,
            coverage,
            human_reference,
            detector_snapshot,
            artifact_bundle,
            adjudication_set,
        )
        .map_err(JoinMetricContributionError::JoinValidation)?;

        let derived = derive_contribution_body(
            &context,
            envelope,
            seal,
            coverage,
            human_reference,
            detector_snapshot,
            join,
        )?;

        compare_stored_contribution_set(self, &derived)?;

        Ok(())
    }
}

enum DeriveMode {
    Creation,
    Historical,
}

fn derive_contribution_body(
    context: &JoinMetricContributionContext,
    envelope: &RunEnvelope,
    seal: &ReferenceSeal,
    coverage: &ReferenceCoverage,
    human_reference: &HumanFinalReference,
    detector_snapshot: &DetectorProposalSnapshot,
    join: &DetectorReferenceJoin,
) -> Result<JoinMetricContributionSet, JoinMetricContributionError> {
    let mut detector_contributions = derive_detector_contributions(join)?;
    detector_contributions.sort_by(|left, right| {
        left.detector_proposal_id
            .as_str()
            .cmp(right.detector_proposal_id.as_str())
    });

    let reference_records_by_id = human_reference
        .records
        .iter()
        .map(|record| (record.reference_error_id.clone(), record))
        .collect::<std::collections::HashMap<_, _>>();

    let mut reference_contributions =
        derive_reference_contributions(join, &reference_records_by_id)?;
    reference_contributions.sort_by(|left, right| {
        left.reference_error_id
            .as_str()
            .cmp(right.reference_error_id.as_str())
    });

    let assessment = derive_assessment(
        join.assessment.detector_proposal_count,
        join.assessment.reference_record_count,
        &detector_contributions,
        &reference_contributions,
    );

    let state = derive_contribution_state(join, &assessment);
    let eligibility =
        derive_primary_eligibility(envelope, seal, coverage, join, state, &assessment);

    Ok(JoinMetricContributionSet {
        schema_revision: JOIN_METRIC_CONTRIBUTION_SCHEMA.to_string(),
        contribution_set_id: context.contribution_set_id.clone(),
        contribution_revision: context.contribution_revision.clone(),
        run_id: envelope.run_id.clone(),
        input_identity: envelope.input_identity.clone(),
        input_class: envelope.input_class,
        qualifies_as_real_material_evidence: envelope.qualifies_as_real_material_evidence,
        reference_seal_id: seal.seal_id.clone(),
        reference_revision: seal.reference_revision.clone(),
        reference_coverage_id: coverage.coverage_id.clone(),
        detector_snapshot_revision: detector_snapshot.snapshot_revision.clone(),
        detector_output_artifact_id: detector_snapshot.detector_output_artifact_id.clone(),
        join_id: join.join_id.clone(),
        join_revision: join.join_revision.clone(),
        evaluation_join_artifact_id: join.evaluation_join_artifact_id.clone(),
        join_adjudication_artifact_id: join.join_adjudication_artifact_id.clone(),
        metric_contributions_artifact_id: context.metric_contributions_artifact_id.clone(),
        eligibility_policy_revision: PRIMARY_METRIC_ELIGIBILITY_POLICY.to_string(),
        contribution_policy_revision: METRIC_CONTRIBUTION_POLICY.to_string(),
        state,
        eligibility,
        detector_contributions,
        reference_contributions,
        assessment,
    })
}

fn derive_detector_contributions(
    join: &DetectorReferenceJoin,
) -> Result<Vec<DetectorMetricContributionRecord>, JoinMetricContributionError> {
    join.detector_dispositions
        .iter()
        .map(|record| {
            if record.disposition == DetectorReferenceMatchDisposition::UnmatchedReference {
                return Err(
                    JoinMetricContributionError::SideIncompatibleDetectorDisposition {
                        disposition: record.disposition,
                    },
                );
            }

            let (proposal_precision, duplicate_proposal_burden) =
                detector_ratio_contributions(record.disposition);

            Ok(DetectorMetricContributionRecord {
                detector_proposal_id: record.detector_proposal_id.clone(),
                join_disposition: record.disposition,
                proposal_precision,
                duplicate_proposal_burden,
            })
        })
        .collect()
}

fn derive_reference_contributions(
    join: &DetectorReferenceJoin,
    reference_records_by_id: &std::collections::HashMap<ReferenceErrorId, &ReferenceErrorRecord>,
) -> Result<Vec<ReferenceMetricContributionRecord>, JoinMetricContributionError> {
    join.reference_dispositions
        .iter()
        .map(|record| {
            if matches!(
                record.disposition,
                DetectorReferenceMatchDisposition::DuplicateProposal
                    | DetectorReferenceMatchDisposition::UnmatchedDetector
            ) {
                return Err(
                    JoinMetricContributionError::SideIncompatibleReferenceDisposition {
                        disposition: record.disposition,
                    },
                );
            }

            let source_record = reference_records_by_id
                .get(&record.reference_error_id)
                .ok_or(JoinMetricContributionError::StoredContributionMismatch)?;

            let eligibility = derive_reference_eligibility(source_record);
            let (localization, correction_exactness, end_to_end) =
                reference_ratio_contributions(eligibility, record.disposition);

            if eligibility != ReferenceJoinEligibility::RecallEligibleTranscriptionError
                && record.disposition != DetectorReferenceMatchDisposition::ExcludedFromErrorMetrics
            {
                return Err(
                    JoinMetricContributionError::ReferenceEligibilityDispositionMismatch {
                        reference_error_id: record.reference_error_id.clone(),
                        eligibility,
                        disposition: record.disposition,
                    },
                );
            }

            if eligibility == ReferenceJoinEligibility::RecallEligibleTranscriptionError
                && record.disposition == DetectorReferenceMatchDisposition::ExcludedFromErrorMetrics
            {
                return Err(
                    JoinMetricContributionError::ReferenceEligibilityDispositionMismatch {
                        reference_error_id: record.reference_error_id.clone(),
                        eligibility,
                        disposition: record.disposition,
                    },
                );
            }

            Ok(ReferenceMetricContributionRecord {
                reference_error_id: record.reference_error_id.clone(),
                join_disposition: record.disposition,
                reference_eligibility: eligibility,
                error_localization_recall: localization,
                correction_exactness_given_localization: correction_exactness,
                end_to_end_exact_correction_recall: end_to_end,
            })
        })
        .collect()
}

fn derive_reference_eligibility(record: &ReferenceErrorRecord) -> ReferenceJoinEligibility {
    match record.reference_class {
        ReferenceClass::TranscriptionError => match record.verification_basis {
            VerificationBasis::AudioListened | VerificationBasis::MixedSources => {
                ReferenceJoinEligibility::RecallEligibleTranscriptionError
            }
            VerificationBasis::TranscriptContextOnly => {
                ReferenceJoinEligibility::ExcludedVerificationBasis
            }
        },
        ReferenceClass::StylePreference
        | ReferenceClass::Ambiguous
        | ReferenceClass::Unsupported
        | ReferenceClass::NonError => ReferenceJoinEligibility::ExcludedReferenceClass,
    }
}

fn detector_ratio_contributions(
    disposition: DetectorReferenceMatchDisposition,
) -> (RatioContribution, RatioContribution) {
    match disposition {
        DetectorReferenceMatchDisposition::ExactMatch
        | DetectorReferenceMatchDisposition::AcceptedOverlap => (
            RatioContribution::NumeratorAndDenominator,
            RatioContribution::DenominatorOnly,
        ),
        DetectorReferenceMatchDisposition::DetectorWrongCorrection
        | DetectorReferenceMatchDisposition::UnmatchedDetector => (
            RatioContribution::DenominatorOnly,
            RatioContribution::DenominatorOnly,
        ),
        DetectorReferenceMatchDisposition::DuplicateProposal => (
            RatioContribution::DenominatorOnly,
            RatioContribution::NumeratorAndDenominator,
        ),
        DetectorReferenceMatchDisposition::AmbiguousMatch => (
            RatioContribution::Excluded(MetricContributionExclusionReason::AmbiguousMatch),
            RatioContribution::Excluded(MetricContributionExclusionReason::AmbiguousMatch),
        ),
        DetectorReferenceMatchDisposition::ExcludedFromErrorMetrics => (
            RatioContribution::Excluded(MetricContributionExclusionReason::JoinExcluded),
            RatioContribution::Excluded(MetricContributionExclusionReason::JoinExcluded),
        ),
        DetectorReferenceMatchDisposition::OverlapCandidate => (
            RatioContribution::PendingAdjudication,
            RatioContribution::PendingAdjudication,
        ),
        DetectorReferenceMatchDisposition::UnmatchedReference => (
            RatioContribution::Excluded(MetricContributionExclusionReason::JoinExcluded),
            RatioContribution::Excluded(MetricContributionExclusionReason::JoinExcluded),
        ),
    }
}

fn reference_ratio_contributions(
    eligibility: ReferenceJoinEligibility,
    disposition: DetectorReferenceMatchDisposition,
) -> (RatioContribution, RatioContribution, RatioContribution) {
    if eligibility != ReferenceJoinEligibility::RecallEligibleTranscriptionError {
        let excluded =
            RatioContribution::Excluded(MetricContributionExclusionReason::ReferenceIneligible);
        return (excluded, excluded, excluded);
    }

    match disposition {
        DetectorReferenceMatchDisposition::ExactMatch
        | DetectorReferenceMatchDisposition::AcceptedOverlap => (
            RatioContribution::NumeratorAndDenominator,
            RatioContribution::NumeratorAndDenominator,
            RatioContribution::NumeratorAndDenominator,
        ),
        DetectorReferenceMatchDisposition::DetectorWrongCorrection => (
            RatioContribution::NumeratorAndDenominator,
            RatioContribution::DenominatorOnly,
            RatioContribution::DenominatorOnly,
        ),
        DetectorReferenceMatchDisposition::UnmatchedReference => (
            RatioContribution::DenominatorOnly,
            RatioContribution::Excluded(MetricContributionExclusionReason::NotLocalized),
            RatioContribution::DenominatorOnly,
        ),
        DetectorReferenceMatchDisposition::AmbiguousMatch => (
            RatioContribution::Excluded(MetricContributionExclusionReason::AmbiguousMatch),
            RatioContribution::Excluded(MetricContributionExclusionReason::AmbiguousMatch),
            RatioContribution::Excluded(MetricContributionExclusionReason::AmbiguousMatch),
        ),
        DetectorReferenceMatchDisposition::OverlapCandidate => (
            RatioContribution::PendingAdjudication,
            RatioContribution::PendingAdjudication,
            RatioContribution::PendingAdjudication,
        ),
        DetectorReferenceMatchDisposition::ExcludedFromErrorMetrics => (
            RatioContribution::Excluded(MetricContributionExclusionReason::JoinExcluded),
            RatioContribution::Excluded(MetricContributionExclusionReason::JoinExcluded),
            RatioContribution::Excluded(MetricContributionExclusionReason::JoinExcluded),
        ),
        DetectorReferenceMatchDisposition::DuplicateProposal
        | DetectorReferenceMatchDisposition::UnmatchedDetector => (
            RatioContribution::Excluded(MetricContributionExclusionReason::JoinExcluded),
            RatioContribution::Excluded(MetricContributionExclusionReason::JoinExcluded),
            RatioContribution::Excluded(MetricContributionExclusionReason::JoinExcluded),
        ),
    }
}

fn derive_assessment(
    detector_source_count: u32,
    reference_source_count: u32,
    detector_contributions: &[DetectorMetricContributionRecord],
    reference_contributions: &[ReferenceMetricContributionRecord],
) -> MetricContributionSetAssessment {
    let pending_detector_contribution_count = detector_contributions
        .iter()
        .filter(|record| detector_contribution_record_is_pending(record))
        .count() as u32;
    let pending_reference_contribution_count = reference_contributions
        .iter()
        .filter(|record| reference_contribution_record_is_pending(record))
        .count() as u32;

    let detector_contribution_count = detector_contributions.len() as u32;
    let reference_contribution_count = reference_contributions.len() as u32;

    let mapping_complete = detector_source_count == detector_contribution_count
        && reference_source_count == reference_contribution_count;

    MetricContributionSetAssessment {
        detector_source_count,
        detector_contribution_count,
        reference_source_count,
        reference_contribution_count,
        pending_detector_contribution_count,
        pending_reference_contribution_count,
        mapping_complete,
    }
}

fn detector_contribution_record_is_pending(record: &DetectorMetricContributionRecord) -> bool {
    matches!(
        record.proposal_precision,
        RatioContribution::PendingAdjudication
    ) || matches!(
        record.duplicate_proposal_burden,
        RatioContribution::PendingAdjudication
    )
}

fn reference_contribution_record_is_pending(record: &ReferenceMetricContributionRecord) -> bool {
    matches!(
        record.error_localization_recall,
        RatioContribution::PendingAdjudication
    ) || matches!(
        record.correction_exactness_given_localization,
        RatioContribution::PendingAdjudication
    ) || matches!(
        record.end_to_end_exact_correction_recall,
        RatioContribution::PendingAdjudication
    )
}

fn derive_contribution_state(
    join: &DetectorReferenceJoin,
    assessment: &MetricContributionSetAssessment,
) -> MetricContributionSetState {
    if assessment.pending_detector_contribution_count > 0
        || assessment.pending_reference_contribution_count > 0
        || join.state != DetectorReferenceJoinState::Resolved
        || join.assessment.unresolved_overlap_edge_count > 0
    {
        MetricContributionSetState::PendingJoinResolution
    } else {
        MetricContributionSetState::Complete
    }
}

fn is_synthetic_posture(
    seal: &ReferenceSeal,
    coverage: &ReferenceCoverage,
    join: &DetectorReferenceJoin,
) -> bool {
    seal.calibration_classification == ReferenceCalibrationValidity::SyntheticProtocolOnly
        || coverage.coverage_purpose == ReferenceCoveragePurpose::SyntheticProtocolValidation
        || join.join_purpose == DetectorReferenceJoinPurpose::SyntheticProtocolValidation
}

fn derive_report_class(
    seal: &ReferenceSeal,
    coverage: &ReferenceCoverage,
    join: &DetectorReferenceJoin,
    blocking_reasons: &[PrimaryMetricBlockingReason],
) -> MetricContributionReportClass {
    if is_synthetic_posture(seal, coverage, join) {
        return MetricContributionReportClass::SyntheticProtocolValidation;
    }

    if blocking_reasons.is_empty()
        && join.join_purpose == DetectorReferenceJoinPurpose::PrimaryBlindCalibration
    {
        MetricContributionReportClass::PrimaryBlindCalibration
    } else {
        MetricContributionReportClass::NonCalibrationDiagnostic
    }
}

fn derive_primary_eligibility(
    envelope: &RunEnvelope,
    seal: &ReferenceSeal,
    coverage: &ReferenceCoverage,
    join: &DetectorReferenceJoin,
    state: MetricContributionSetState,
    assessment: &MetricContributionSetAssessment,
) -> PrimaryMetricEligibilityAssessment {
    let mut blocking_reasons = Vec::new();

    if envelope.calibration_validity != CalibrationValidityMode::BlindReference {
        blocking_reasons.push(PrimaryMetricBlockingReason::EnvelopeNotBlindReference);
    }
    if seal.calibration_classification != ReferenceCalibrationValidity::BlindReferenceEligible {
        blocking_reasons.push(PrimaryMetricBlockingReason::ReferenceNotBlindEligible);
    }
    if seal.calibration_validity_impact != CalibrationValidityImpact::None {
        blocking_reasons.push(PrimaryMetricBlockingReason::ReferenceValidityImpactNotNone);
    }
    if coverage.coverage_purpose != ReferenceCoveragePurpose::PrimaryBlindCalibration {
        blocking_reasons.push(PrimaryMetricBlockingReason::CoverageNotPrimary);
    }
    if coverage.coverage_state != ReferenceCoverageState::Complete
        || !coverage.assessment.coverage_complete
    {
        blocking_reasons.push(PrimaryMetricBlockingReason::CoverageIncomplete);
    }
    if join.join_purpose != DetectorReferenceJoinPurpose::PrimaryBlindCalibration {
        blocking_reasons.push(PrimaryMetricBlockingReason::JoinNotPrimary);
    }
    if join.state != DetectorReferenceJoinState::Resolved {
        blocking_reasons.push(PrimaryMetricBlockingReason::JoinNotResolved);
    }
    if !join.assessment.one_to_one_consistent {
        blocking_reasons.push(PrimaryMetricBlockingReason::JoinNotOneToOne);
    }
    if join.assessment.unresolved_overlap_edge_count > 0 {
        blocking_reasons.push(PrimaryMetricBlockingReason::JoinContainsUnresolvedOverlap);
    }
    if state == MetricContributionSetState::PendingJoinResolution
        || assessment.pending_detector_contribution_count > 0
        || assessment.pending_reference_contribution_count > 0
    {
        blocking_reasons.push(PrimaryMetricBlockingReason::ContributionSetPending);
    }
    if is_synthetic_posture(seal, coverage, join) {
        blocking_reasons.push(PrimaryMetricBlockingReason::SyntheticProtocolOnly);
    }

    sort_blocking_reasons(&mut blocking_reasons);

    let primary_metrics_allowed = blocking_reasons.is_empty();
    let eligible_primary_metrics = if primary_metrics_allowed {
        all_primary_metric_kinds()
    } else {
        Vec::new()
    };

    let report_class = derive_report_class(seal, coverage, join, &blocking_reasons);

    PrimaryMetricEligibilityAssessment {
        policy_revision: PRIMARY_METRIC_ELIGIBILITY_POLICY.to_string(),
        report_class,
        primary_metrics_allowed,
        eligible_primary_metrics,
        blocking_reasons,
        qualifies_as_real_material_evidence: envelope.qualifies_as_real_material_evidence,
    }
}

fn validate_detector_contribution_record(
    record: &DetectorMetricContributionRecord,
) -> Result<(), JoinMetricContributionError> {
    if record.join_disposition == DetectorReferenceMatchDisposition::UnmatchedReference {
        return Err(
            JoinMetricContributionError::SideIncompatibleDetectorDisposition {
                disposition: record.join_disposition,
            },
        );
    }

    let (expected_precision, expected_duplicate) =
        detector_ratio_contributions(record.join_disposition);
    if record.proposal_precision != expected_precision
        || record.duplicate_proposal_burden != expected_duplicate
    {
        return Err(JoinMetricContributionError::StoredContributionMismatch);
    }

    validate_detector_denominator_parity(record)?;

    Ok(())
}

fn validate_reference_contribution_record(
    record: &ReferenceMetricContributionRecord,
) -> Result<(), JoinMetricContributionError> {
    if matches!(
        record.join_disposition,
        DetectorReferenceMatchDisposition::DuplicateProposal
            | DetectorReferenceMatchDisposition::UnmatchedDetector
    ) {
        return Err(
            JoinMetricContributionError::SideIncompatibleReferenceDisposition {
                disposition: record.join_disposition,
            },
        );
    }

    let (expected_localization, expected_correction, expected_end_to_end) =
        reference_ratio_contributions(record.reference_eligibility, record.join_disposition);
    if record.error_localization_recall != expected_localization
        || record.correction_exactness_given_localization != expected_correction
        || record.end_to_end_exact_correction_recall != expected_end_to_end
    {
        return Err(JoinMetricContributionError::StoredContributionMismatch);
    }

    if record.reference_eligibility != ReferenceJoinEligibility::RecallEligibleTranscriptionError
        && record.join_disposition != DetectorReferenceMatchDisposition::ExcludedFromErrorMetrics
    {
        return Err(
            JoinMetricContributionError::ReferenceEligibilityDispositionMismatch {
                reference_error_id: record.reference_error_id.clone(),
                eligibility: record.reference_eligibility,
                disposition: record.join_disposition,
            },
        );
    }

    Ok(())
}

fn validate_detector_denominator_parity(
    record: &DetectorMetricContributionRecord,
) -> Result<(), JoinMetricContributionError> {
    let precision_denominator = matches!(
        record.proposal_precision,
        RatioContribution::NumeratorAndDenominator | RatioContribution::DenominatorOnly
    );
    let duplicate_denominator = matches!(
        record.duplicate_proposal_burden,
        RatioContribution::NumeratorAndDenominator | RatioContribution::DenominatorOnly
    );
    if precision_denominator != duplicate_denominator {
        return Err(JoinMetricContributionError::StoredContributionMismatch);
    }
    Ok(())
}

fn validate_local_state_consistency(
    set: &JoinMetricContributionSet,
) -> Result<(), JoinMetricContributionError> {
    if set.eligibility.policy_revision != PRIMARY_METRIC_ELIGIBILITY_POLICY {
        return Err(JoinMetricContributionError::PrimaryEligibilityInconsistent);
    }

    if set.eligibility.primary_metrics_allowed != set.eligibility.blocking_reasons.is_empty() {
        return Err(JoinMetricContributionError::PrimaryEligibilityInconsistent);
    }

    let mut stored_metrics = set.eligibility.eligible_primary_metrics.clone();
    sort_primary_metric_kinds(&mut stored_metrics);
    if set.eligibility.eligible_primary_metrics != stored_metrics {
        return Err(JoinMetricContributionError::PrimaryEligibilityInconsistent);
    }

    if set.eligibility.primary_metrics_allowed {
        if set.eligibility.eligible_primary_metrics != all_primary_metric_kinds() {
            return Err(JoinMetricContributionError::PrimaryEligibilityInconsistent);
        }
        if set.eligibility.report_class != MetricContributionReportClass::PrimaryBlindCalibration {
            return Err(JoinMetricContributionError::ReportClassInconsistent);
        }
    } else if !set.eligibility.eligible_primary_metrics.is_empty() {
        return Err(JoinMetricContributionError::PrimaryEligibilityInconsistent);
    } else if set.eligibility.report_class == MetricContributionReportClass::PrimaryBlindCalibration
    {
        return Err(JoinMetricContributionError::ReportClassInconsistent);
    }

    let mut blocking_reasons = set.eligibility.blocking_reasons.clone();
    sort_blocking_reasons(&mut blocking_reasons);
    if set.eligibility.blocking_reasons != blocking_reasons {
        return Err(JoinMetricContributionError::PrimaryEligibilityInconsistent);
    }

    if set
        .eligibility
        .blocking_reasons
        .contains(&PrimaryMetricBlockingReason::SyntheticProtocolOnly)
        && set.eligibility.report_class
            != MetricContributionReportClass::SyntheticProtocolValidation
    {
        return Err(JoinMetricContributionError::ReportClassInconsistent);
    }

    if set.eligibility.primary_metrics_allowed
        && (set.state == MetricContributionSetState::PendingJoinResolution
            || set.assessment.pending_detector_contribution_count > 0
            || set.assessment.pending_reference_contribution_count > 0)
    {
        return Err(JoinMetricContributionError::PendingPrimaryEligibilityMismatch);
    }

    if set.eligibility.primary_metrics_allowed && set.state != MetricContributionSetState::Complete
    {
        return Err(JoinMetricContributionError::PendingPrimaryEligibilityMismatch);
    }

    match set.state {
        MetricContributionSetState::Complete => {
            if set.assessment.pending_detector_contribution_count > 0
                || set.assessment.pending_reference_contribution_count > 0
            {
                return Err(JoinMetricContributionError::PendingCompleteStateMismatch);
            }
            if !set.assessment.mapping_complete {
                return Err(JoinMetricContributionError::ContributionSetStateMismatch {
                    state: set.state,
                    assessment: Box::new(set.assessment.clone()),
                });
            }
        }
        MetricContributionSetState::PendingJoinResolution
        | MetricContributionSetState::Invalidated => {}
    }

    Ok(())
}

fn is_contribution_creation_lifecycle(state: RunLifecycleState) -> bool {
    matches!(
        state,
        RunLifecycleState::DetectorExecution | RunLifecycleState::AssistedReview
    )
}

fn is_contribution_historical_lifecycle(state: RunLifecycleState) -> bool {
    matches!(
        state,
        RunLifecycleState::DetectorExecution
            | RunLifecycleState::AssistedReview
            | RunLifecycleState::Finalized
    )
}

fn validate_contribution_creation_lifecycle(
    lifecycle_state: RunLifecycleState,
) -> Result<(), JoinMetricContributionError> {
    if !is_contribution_creation_lifecycle(lifecycle_state) {
        return Err(
            JoinMetricContributionError::ContributionCreationLifecycleIncompatible {
                lifecycle_state,
            },
        );
    }
    Ok(())
}

fn validate_contribution_inputs(
    envelope: &RunEnvelope,
    seal: &ReferenceSeal,
    coverage: &ReferenceCoverage,
    human_reference: &HumanFinalReference,
    detector_snapshot: &DetectorProposalSnapshot,
    join: &DetectorReferenceJoin,
    adjudication_set: &OverlapAdjudicationSet,
    artifact_bundle: &ArtifactBundle,
    context: &JoinMetricContributionContext,
    mode: DeriveMode,
) -> Result<(), JoinMetricContributionError> {
    envelope
        .validate()
        .map_err(JoinMetricContributionError::EnvelopeValidation)?;

    if envelope.lifecycle_state == RunLifecycleState::Invalidated {
        return Err(JoinMetricContributionError::InvalidatedContributionContext);
    }

    match mode {
        DeriveMode::Creation => validate_contribution_creation_lifecycle(envelope.lifecycle_state)?,
        DeriveMode::Historical => {
            if !is_contribution_historical_lifecycle(envelope.lifecycle_state) {
                return Err(
                    JoinMetricContributionError::ContributionHistoricalLifecycleIncompatible {
                        lifecycle_state: envelope.lifecycle_state,
                    },
                );
            }
        }
    }

    seal.validate_historical_context(envelope)
        .map_err(JoinMetricContributionError::SealValidation)?;
    coverage
        .validate_historical_context(envelope, seal, Some(human_reference))
        .map_err(JoinMetricContributionError::CoverageValidation)?;
    human_reference
        .validate_historical_context(envelope, seal)
        .map_err(|error| JoinMetricContributionError::HumanReferenceValidation(Box::new(error)))?;

    if detector_snapshot.state != DetectorProposalSnapshotState::Frozen {
        return Err(JoinMetricContributionError::SnapshotNotFrozen);
    }

    detector_snapshot
        .validate_against_bundle(envelope, artifact_bundle)
        .map_err(JoinMetricContributionError::SnapshotValidation)?;

    artifact_bundle
        .validate_with_reference_context(
            envelope,
            Some(seal),
            Some(coverage),
            Some(human_reference),
        )
        .map_err(JoinMetricContributionError::BundleValidation)?;

    validate_contribution_bundle_roles(artifact_bundle, context, join)?;

    adjudication_set
        .validate_frozen_for_join(
            envelope,
            &seal.reference_revision,
            &detector_snapshot.snapshot_revision,
        )
        .map_err(JoinMetricContributionError::AdjudicationValidation)?;

    validate_join_lineage_binding(envelope, seal, coverage, detector_snapshot, join)?;

    if context.contribution_set_id.as_str().is_empty()
        || context.contribution_revision.as_str().is_empty()
    {
        return Err(JoinMetricContributionError::InvalidContributionSetId(
            MetricContributionIdentityIdError::Empty,
        ));
    }

    Ok(())
}

fn validate_join_lineage_binding(
    envelope: &RunEnvelope,
    seal: &ReferenceSeal,
    coverage: &ReferenceCoverage,
    detector_snapshot: &DetectorProposalSnapshot,
    join: &DetectorReferenceJoin,
) -> Result<(), JoinMetricContributionError> {
    if join.run_id != envelope.run_id
        || seal.run_id != envelope.run_id
        || coverage.run_id != envelope.run_id
        || detector_snapshot.run_id != envelope.run_id
    {
        return Err(JoinMetricContributionError::RunIdMismatch);
    }

    if join.input_identity != envelope.input_identity
        || seal.input_identity != envelope.input_identity
        || coverage.input_identity != envelope.input_identity
        || detector_snapshot.input_identity != envelope.input_identity
    {
        return Err(JoinMetricContributionError::InputIdentityMismatch);
    }

    if join.reference_seal_id != seal.seal_id {
        return Err(JoinMetricContributionError::TopLevelBindingMismatch {
            field: "reference_seal_id",
        });
    }

    if join.reference_revision != seal.reference_revision
        || join.reference_revision != coverage.reference_revision
    {
        return Err(JoinMetricContributionError::ReferenceRevisionMismatch);
    }

    if join.reference_coverage_id != coverage.coverage_id {
        return Err(JoinMetricContributionError::CoverageIdMismatch);
    }

    if join.detector_snapshot_revision != detector_snapshot.snapshot_revision {
        return Err(JoinMetricContributionError::SnapshotRevisionMismatch);
    }

    if join.detector_output_artifact_id != detector_snapshot.detector_output_artifact_id {
        return Err(JoinMetricContributionError::DetectorOutputArtifactMismatch);
    }

    Ok(())
}

fn validate_contribution_bundle_roles(
    bundle: &ArtifactBundle,
    context: &JoinMetricContributionContext,
    join: &DetectorReferenceJoin,
) -> Result<(), JoinMetricContributionError> {
    let evaluation_joins = bundle
        .artifacts
        .iter()
        .filter(|descriptor| descriptor.role == ArtifactRole::EvaluationJoin)
        .collect::<Vec<_>>();
    if evaluation_joins.len() != 1
        || evaluation_joins[0].artifact_id != join.evaluation_join_artifact_id
    {
        return Err(JoinMetricContributionError::EvaluationJoinArtifactMismatch);
    }

    let adjudication_roles = bundle
        .artifacts
        .iter()
        .filter(|descriptor| descriptor.role == ArtifactRole::JoinAdjudication)
        .collect::<Vec<_>>();
    if adjudication_roles.len() != 1
        || adjudication_roles[0].artifact_id != join.join_adjudication_artifact_id
    {
        return Err(JoinMetricContributionError::JoinAdjudicationArtifactMismatch);
    }

    let detector_outputs = bundle
        .artifacts
        .iter()
        .filter(|descriptor| descriptor.role == ArtifactRole::DetectorOutput)
        .collect::<Vec<_>>();
    if detector_outputs.len() != 1
        || detector_outputs[0].artifact_id != join.detector_output_artifact_id
    {
        return Err(JoinMetricContributionError::DetectorOutputArtifactMismatch);
    }

    let metric_contributions = bundle
        .artifacts
        .iter()
        .filter(|descriptor| descriptor.role == ArtifactRole::MetricContributions)
        .collect::<Vec<_>>();
    if metric_contributions.len() != 1 {
        return Err(JoinMetricContributionError::MetricContributionsArtifactMismatch);
    }
    if metric_contributions[0].artifact_id != context.metric_contributions_artifact_id {
        return Err(JoinMetricContributionError::MetricContributionsArtifactMismatch);
    }

    Ok(())
}

fn authoritative_contribution_context_for_validation(
    stored: &JoinMetricContributionSet,
    artifact_bundle: &ArtifactBundle,
) -> Result<JoinMetricContributionContext, JoinMetricContributionError> {
    let metric_contributions = artifact_bundle
        .artifacts
        .iter()
        .filter(|descriptor| descriptor.role == ArtifactRole::MetricContributions)
        .collect::<Vec<_>>();
    if metric_contributions.len() != 1 {
        return Err(JoinMetricContributionError::MetricContributionsArtifactMismatch);
    }
    if metric_contributions[0].artifact_id != stored.metric_contributions_artifact_id {
        return Err(JoinMetricContributionError::MetricContributionsArtifactMismatch);
    }

    Ok(JoinMetricContributionContext {
        contribution_set_id: stored.contribution_set_id.clone(),
        contribution_revision: stored.contribution_revision.clone(),
        metric_contributions_artifact_id: stored.metric_contributions_artifact_id.clone(),
    })
}

fn compare_stored_contribution_set(
    stored: &JoinMetricContributionSet,
    derived: &JoinMetricContributionSet,
) -> Result<(), JoinMetricContributionError> {
    let fields: [(&str, bool); 22] = [
        ("run_id", stored.run_id == derived.run_id),
        (
            "input_identity",
            stored.input_identity == derived.input_identity,
        ),
        ("input_class", stored.input_class == derived.input_class),
        (
            "qualifies_as_real_material_evidence",
            stored.qualifies_as_real_material_evidence
                == derived.qualifies_as_real_material_evidence,
        ),
        (
            "reference_seal_id",
            stored.reference_seal_id == derived.reference_seal_id,
        ),
        (
            "reference_revision",
            stored.reference_revision == derived.reference_revision,
        ),
        (
            "reference_coverage_id",
            stored.reference_coverage_id == derived.reference_coverage_id,
        ),
        (
            "detector_snapshot_revision",
            stored.detector_snapshot_revision == derived.detector_snapshot_revision,
        ),
        (
            "detector_output_artifact_id",
            stored.detector_output_artifact_id == derived.detector_output_artifact_id,
        ),
        ("join_id", stored.join_id == derived.join_id),
        (
            "join_revision",
            stored.join_revision == derived.join_revision,
        ),
        (
            "evaluation_join_artifact_id",
            stored.evaluation_join_artifact_id == derived.evaluation_join_artifact_id,
        ),
        (
            "join_adjudication_artifact_id",
            stored.join_adjudication_artifact_id == derived.join_adjudication_artifact_id,
        ),
        (
            "metric_contributions_artifact_id",
            stored.metric_contributions_artifact_id == derived.metric_contributions_artifact_id,
        ),
        (
            "eligibility_policy_revision",
            stored.eligibility_policy_revision == derived.eligibility_policy_revision,
        ),
        (
            "contribution_policy_revision",
            stored.contribution_policy_revision == derived.contribution_policy_revision,
        ),
        ("state", stored.state == derived.state),
        ("eligibility", stored.eligibility == derived.eligibility),
        (
            "detector_contributions",
            stored.detector_contributions == derived.detector_contributions,
        ),
        (
            "reference_contributions",
            stored.reference_contributions == derived.reference_contributions,
        ),
        ("assessment", stored.assessment == derived.assessment),
        (
            "contribution_set_id",
            stored.contribution_set_id == derived.contribution_set_id,
        ),
    ];

    for (field, matches) in fields {
        if !matches {
            return Err(JoinMetricContributionError::TopLevelBindingMismatch { field });
        }
    }

    if stored.contribution_revision != derived.contribution_revision {
        return Err(JoinMetricContributionError::TopLevelBindingMismatch {
            field: "contribution_revision",
        });
    }

    Ok(())
}

fn validate_policy_revision(
    field: &'static str,
    found: &str,
    expected: &str,
) -> Result<(), JoinMetricContributionError> {
    if found != expected {
        return Err(JoinMetricContributionError::UnsupportedPolicyRevision {
            field,
            found: found.to_string(),
            expected: expected.to_string(),
        });
    }
    Ok(())
}

pub fn validate_contribution_id_value(
    value: &str,
) -> Result<(), MetricContributionIdentityIdError> {
    validate_join_id_value(value).map_err(map_join_identity_error)
}

fn map_join_identity_error(error: JoinIdentityIdError) -> MetricContributionIdentityIdError {
    match error {
        JoinIdentityIdError::Empty => MetricContributionIdentityIdError::Empty,
        JoinIdentityIdError::TooLong { len, max } => {
            MetricContributionIdentityIdError::TooLong { len, max }
        }
        JoinIdentityIdError::InvalidCharacter { character } => {
            MetricContributionIdentityIdError::InvalidCharacter { character }
        }
        JoinIdentityIdError::PathLikeContent => MetricContributionIdentityIdError::PathLikeContent,
        JoinIdentityIdError::AbsolutePathLike => {
            MetricContributionIdentityIdError::AbsolutePathLike
        }
        JoinIdentityIdError::RelativePathLike => {
            MetricContributionIdentityIdError::RelativePathLike
        }
        JoinIdentityIdError::HomeDirectoryFragment => {
            MetricContributionIdentityIdError::HomeDirectoryFragment
        }
        JoinIdentityIdError::GenerationUnavailable => {
            MetricContributionIdentityIdError::GenerationUnavailable
        }
    }
}

fn map_run_id_error(error: RunIdError) -> MetricContributionIdentityIdError {
    match error {
        RunIdError::Empty => MetricContributionIdentityIdError::Empty,
        RunIdError::TooLong { len, max } => MetricContributionIdentityIdError::TooLong { len, max },
        RunIdError::InvalidCharacter { character } => {
            MetricContributionIdentityIdError::InvalidCharacter { character }
        }
        RunIdError::PathLikeContent => MetricContributionIdentityIdError::PathLikeContent,
        RunIdError::AbsolutePathLike => MetricContributionIdentityIdError::AbsolutePathLike,
        RunIdError::RelativePathLike => MetricContributionIdentityIdError::RelativePathLike,
        RunIdError::HomeDirectoryFragment => {
            MetricContributionIdentityIdError::HomeDirectoryFragment
        }
        RunIdError::GenerationUnavailable => {
            MetricContributionIdentityIdError::GenerationUnavailable
        }
    }
}

pub fn contribution_from_json(json: &str) -> Result<JoinMetricContributionSet, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn contribution_to_json(set: &JoinMetricContributionSet) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(set)
}

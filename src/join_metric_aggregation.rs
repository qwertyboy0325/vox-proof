#![allow(clippy::too_many_arguments)]

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::artifact_bundle::{ArtifactBundle, ArtifactBundleValidationError, ArtifactId};
use crate::detector_reference_join::{
    DetectorReferenceJoin, DetectorReferenceJoinError, DetectorReferenceJoinId,
    DetectorReferenceJoinRevisionId, JoinIdentityIdError, validate_join_id_value,
};
use crate::detector_snapshot::{
    DetectorProposalSnapshot, DetectorProposalSnapshotState,
    DetectorProposalSnapshotValidationError, DetectorSnapshotRevisionId,
};
use crate::human_final_reference::{HumanFinalReference, HumanFinalReferenceValidationError};
use crate::join_adjudication::{OverlapAdjudicationSet, OverlapAdjudicationValidationError};
use crate::join_metric_contribution::{
    JoinMetricContributionError, JoinMetricContributionSet, MetricContributionReportClass,
    MetricContributionRevisionId, MetricContributionSetId, MetricContributionSetState,
    PrimaryMetricBlockingReason, PrimaryMetricKind, RatioContribution,
};
use crate::reference_coverage::{
    ReferenceCoverage, ReferenceCoverageId, ReferenceCoverageValidationError,
};
use crate::reference_seal::{ReferenceSeal, ReferenceSealId, ReferenceSealValidationError};
use crate::run_manifest::{
    ArtifactRole, InputClass, InputIdentityReference, RunEnvelope, RunEnvelopeValidationError,
    RunId, RunIdError, RunLifecycleState, validate_opaque_identifier,
};

pub const JOIN_METRIC_AGGREGATION_SCHEMA: &str = "voxproof-join-metric-aggregates-v1";
pub const PRIMARY_METRIC_AGGREGATION_POLICY: &str = "voxproof-primary-metric-aggregation-v1";
pub const ZERO_DENOMINATOR_POLICY: &str = "voxproof-zero-denominator-undefined-v1";

const REQUIRED_METRIC_COUNT: u32 = 5;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MetricAggregateSetId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MetricAggregateRevisionId(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricAggregateValueState {
    DefinedExactRatio,
    UndefinedZeroDenominator,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricAggregateRecord {
    pub metric_kind: PrimaryMetricKind,
    pub numerator_count: u64,
    pub denominator_count: u64,
    pub value_state: MetricAggregateValueState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricAggregateSetState {
    Complete,
    Invalidated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricAggregateSetAssessment {
    pub required_metric_count: u32,
    pub aggregate_metric_count: u32,
    pub defined_metric_count: u32,
    pub undefined_zero_denominator_count: u32,
    pub all_required_metrics_present: bool,
    pub aggregate_consistent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JoinMetricAggregateSet {
    pub schema_revision: String,
    pub aggregate_set_id: MetricAggregateSetId,
    pub aggregate_revision: MetricAggregateRevisionId,
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
    pub contribution_set_id: MetricContributionSetId,
    pub contribution_revision: MetricContributionRevisionId,
    pub metric_contributions_artifact_id: ArtifactId,
    pub metrics_artifact_id: ArtifactId,
    pub aggregation_policy_revision: String,
    pub zero_denominator_policy_revision: String,
    pub report_class: MetricContributionReportClass,
    pub primary_metrics_allowed: bool,
    pub eligible_primary_metrics: Vec<PrimaryMetricKind>,
    pub blocking_reasons: Vec<PrimaryMetricBlockingReason>,
    pub qualifies_as_primary_metric_evidence: bool,
    pub state: MetricAggregateSetState,
    pub metrics: Vec<MetricAggregateRecord>,
    pub assessment: MetricAggregateSetAssessment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinMetricAggregateContext {
    pub aggregate_set_id: MetricAggregateSetId,
    pub aggregate_revision: MetricAggregateRevisionId,
    pub metrics_artifact_id: ArtifactId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetricAggregateIdentityIdError {
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
pub enum JoinMetricAggregationError {
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
    InvalidAggregateSetId(MetricAggregateIdentityIdError),
    InvalidAggregateRevisionId(MetricAggregateIdentityIdError),
    EnvelopeValidation(RunEnvelopeValidationError),
    SealValidation(ReferenceSealValidationError),
    CoverageValidation(ReferenceCoverageValidationError),
    HumanReferenceValidation(Box<HumanFinalReferenceValidationError>),
    SnapshotValidation(DetectorProposalSnapshotValidationError),
    BundleValidation(ArtifactBundleValidationError),
    AdjudicationValidation(OverlapAdjudicationValidationError),
    JoinValidation(DetectorReferenceJoinError),
    ContributionValidation(JoinMetricContributionError),
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
    MetricsArtifactMismatch,
    TopLevelBindingMismatch {
        field: &'static str,
    },
    DuplicateMetricKind {
        metric_kind: PrimaryMetricKind,
    },
    MissingMetricKind {
        metric_kind: PrimaryMetricKind,
    },
    NonCanonicalMetricOrder,
    NumeratorExceedsDenominator {
        metric_kind: PrimaryMetricKind,
    },
    ZeroDenominatorValueStateMismatch {
        metric_kind: PrimaryMetricKind,
    },
    NonZeroDenominatorValueStateMismatch {
        metric_kind: PrimaryMetricKind,
    },
    ZeroDenominatorNonZeroNumerator {
        metric_kind: PrimaryMetricKind,
    },
    AssessmentMismatch {
        stored: Box<MetricAggregateSetAssessment>,
        derived: Box<MetricAggregateSetAssessment>,
    },
    ReportClassInconsistent,
    PrimaryEligibilityInconsistent,
    PrimaryMetricEvidenceDerivationMismatch,
    CrossMetricInvariantViolation {
        invariant: &'static str,
    },
    PendingContributionRejected,
    ContributionSetInvalidated,
    InvalidatedAggregateContext,
    AggregateCreationLifecycleIncompatible {
        lifecycle_state: RunLifecycleState,
    },
    AggregateHistoricalLifecycleIncompatible {
        lifecycle_state: RunLifecycleState,
    },
    SnapshotNotFrozen,
    ContributionSetNotComplete,
    PendingContributionInAggregate,
    CountOverflow {
        metric_kind: PrimaryMetricKind,
    },
    StoredAggregateMismatch,
}

impl MetricAggregateSetId {
    pub fn new(value: impl Into<String>) -> Result<Self, MetricAggregateIdentityIdError> {
        let value = value.into();
        validate_aggregate_id_value(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl MetricAggregateRevisionId {
    pub fn new(value: impl Into<String>) -> Result<Self, MetricAggregateIdentityIdError> {
        let value = value.into();
        validate_aggregate_id_value(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MetricAggregateSetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for MetricAggregateRevisionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn primary_metric_kind_order(kind: PrimaryMetricKind) -> u8 {
    match kind {
        PrimaryMetricKind::ProposalPrecision => 0,
        PrimaryMetricKind::ErrorLocalizationRecall => 1,
        PrimaryMetricKind::CorrectionExactnessGivenLocalization => 2,
        PrimaryMetricKind::EndToEndExactCorrectionRecall => 3,
        PrimaryMetricKind::DuplicateProposalBurden => 4,
    }
}

fn blocking_reason_order(reason: PrimaryMetricBlockingReason) -> u8 {
    match reason {
        PrimaryMetricBlockingReason::EnvelopeNotBlindReference => 0,
        PrimaryMetricBlockingReason::ReferenceNotBlindEligible => 1,
        PrimaryMetricBlockingReason::ReferenceValidityImpactNotNone => 2,
        PrimaryMetricBlockingReason::CoverageNotPrimary => 3,
        PrimaryMetricBlockingReason::CoverageIncomplete => 4,
        PrimaryMetricBlockingReason::JoinNotPrimary => 5,
        PrimaryMetricBlockingReason::JoinNotResolved => 6,
        PrimaryMetricBlockingReason::JoinNotOneToOne => 7,
        PrimaryMetricBlockingReason::JoinContainsUnresolvedOverlap => 8,
        PrimaryMetricBlockingReason::ContributionSetPending => 9,
        PrimaryMetricBlockingReason::SyntheticProtocolOnly => 10,
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
    reasons.sort_by_key(|reason| blocking_reason_order(*reason));
    reasons.dedup();
}

fn sort_primary_metric_kinds(metrics: &mut [PrimaryMetricKind]) {
    metrics.sort_by_key(|metric| primary_metric_kind_order(*metric));
}

impl JoinMetricAggregateSet {
    pub fn derive(
        context: &JoinMetricAggregateContext,
        envelope: &RunEnvelope,
        seal: &ReferenceSeal,
        coverage: &ReferenceCoverage,
        human_reference: &HumanFinalReference,
        detector_snapshot: &DetectorProposalSnapshot,
        join: &DetectorReferenceJoin,
        adjudication_set: &OverlapAdjudicationSet,
        contribution_set: &JoinMetricContributionSet,
        artifact_bundle: &ArtifactBundle,
    ) -> Result<Self, JoinMetricAggregationError> {
        validate_aggregate_creation_lifecycle(envelope.lifecycle_state)?;
        validate_aggregate_inputs(
            envelope,
            seal,
            coverage,
            human_reference,
            detector_snapshot,
            join,
            adjudication_set,
            contribution_set,
            artifact_bundle,
            context,
            DeriveMode::Creation,
        )?;

        contribution_set
            .validate_against(
                envelope,
                seal,
                coverage,
                human_reference,
                detector_snapshot,
                join,
                adjudication_set,
                artifact_bundle,
            )
            .map_err(JoinMetricAggregationError::ContributionValidation)?;

        let set = derive_aggregate_body(context, contribution_set)?;

        set.validate_against(
            envelope,
            seal,
            coverage,
            human_reference,
            detector_snapshot,
            join,
            adjudication_set,
            contribution_set,
            artifact_bundle,
        )?;

        Ok(set)
    }

    pub fn validate(&self) -> Result<(), JoinMetricAggregationError> {
        if self.schema_revision.is_empty() {
            return Err(JoinMetricAggregationError::MissingSchemaRevision);
        }
        if self.schema_revision != JOIN_METRIC_AGGREGATION_SCHEMA {
            return Err(JoinMetricAggregationError::UnsupportedSchemaRevision {
                found: self.schema_revision.clone(),
                expected: JOIN_METRIC_AGGREGATION_SCHEMA.to_string(),
            });
        }

        validate_policy_revision(
            "aggregation_policy_revision",
            &self.aggregation_policy_revision,
            PRIMARY_METRIC_AGGREGATION_POLICY,
        )?;
        validate_policy_revision(
            "zero_denominator_policy_revision",
            &self.zero_denominator_policy_revision,
            ZERO_DENOMINATOR_POLICY,
        )?;

        validate_aggregate_id_value(self.aggregate_set_id.as_str())
            .map_err(JoinMetricAggregationError::InvalidAggregateSetId)?;
        validate_aggregate_id_value(self.aggregate_revision.as_str())
            .map_err(JoinMetricAggregationError::InvalidAggregateRevisionId)?;

        validate_opaque_identifier(self.run_id.as_str()).map_err(|error| {
            JoinMetricAggregationError::InvalidAggregateSetId(map_run_id_error(error))
        })?;

        validate_metric_inventory(&self.metrics)?;
        validate_local_record_consistency(&self.metrics)?;

        let derived_assessment = derive_assessment(&self.metrics);
        if self.assessment != derived_assessment {
            return Err(JoinMetricAggregationError::AssessmentMismatch {
                stored: Box::new(self.assessment.clone()),
                derived: Box::new(derived_assessment),
            });
        }

        validate_local_posture_consistency(self)?;
        validate_cross_metric_invariants(&self.metrics)?;

        if self.state == MetricAggregateSetState::Invalidated {
            return Err(JoinMetricAggregationError::InvalidatedAggregateContext);
        }

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
        contribution_set: &JoinMetricContributionSet,
        artifact_bundle: &ArtifactBundle,
    ) -> Result<(), JoinMetricAggregationError> {
        if self.state == MetricAggregateSetState::Invalidated {
            return Err(JoinMetricAggregationError::InvalidatedAggregateContext);
        }

        let mode = if is_aggregate_creation_lifecycle(envelope.lifecycle_state) {
            DeriveMode::Creation
        } else if is_aggregate_historical_lifecycle(envelope.lifecycle_state) {
            DeriveMode::Historical
        } else {
            return Err(
                JoinMetricAggregationError::AggregateHistoricalLifecycleIncompatible {
                    lifecycle_state: envelope.lifecycle_state,
                },
            );
        };

        validate_aggregate_inputs(
            envelope,
            seal,
            coverage,
            human_reference,
            detector_snapshot,
            join,
            adjudication_set,
            contribution_set,
            artifact_bundle,
            &authoritative_aggregate_context_for_validation(self, artifact_bundle)?,
            mode,
        )?;

        contribution_set
            .validate_against(
                envelope,
                seal,
                coverage,
                human_reference,
                detector_snapshot,
                join,
                adjudication_set,
                artifact_bundle,
            )
            .map_err(JoinMetricAggregationError::ContributionValidation)?;

        let context = JoinMetricAggregateContext {
            aggregate_set_id: self.aggregate_set_id.clone(),
            aggregate_revision: self.aggregate_revision.clone(),
            metrics_artifact_id: self.metrics_artifact_id.clone(),
        };
        let derived = derive_aggregate_body(&context, contribution_set)?;

        compare_stored_aggregate_set(self, &derived)?;

        Ok(())
    }
}

enum DeriveMode {
    Creation,
    Historical,
}

fn derive_aggregate_body(
    context: &JoinMetricAggregateContext,
    contribution_set: &JoinMetricContributionSet,
) -> Result<JoinMetricAggregateSet, JoinMetricAggregationError> {
    reject_non_aggregatable_contribution_state(contribution_set)?;

    let metrics = derive_metric_records(contribution_set)?;
    validate_cross_metric_invariants(&metrics)?;
    let assessment = derive_assessment(&metrics);

    let report_class = contribution_set.eligibility.report_class;
    let primary_metrics_allowed = contribution_set.eligibility.primary_metrics_allowed;
    let mut eligible_primary_metrics = contribution_set
        .eligibility
        .eligible_primary_metrics
        .clone();
    sort_primary_metric_kinds(&mut eligible_primary_metrics);
    let mut blocking_reasons = contribution_set.eligibility.blocking_reasons.clone();
    sort_blocking_reasons(&mut blocking_reasons);

    let qualifies_as_primary_metric_evidence = primary_metrics_allowed
        && contribution_set.state == MetricContributionSetState::Complete
        && assessment.all_required_metrics_present
        && assessment.aggregate_consistent;

    Ok(JoinMetricAggregateSet {
        schema_revision: JOIN_METRIC_AGGREGATION_SCHEMA.to_string(),
        aggregate_set_id: context.aggregate_set_id.clone(),
        aggregate_revision: context.aggregate_revision.clone(),
        run_id: contribution_set.run_id.clone(),
        input_identity: contribution_set.input_identity.clone(),
        input_class: contribution_set.input_class,
        qualifies_as_real_material_evidence: contribution_set.qualifies_as_real_material_evidence,
        reference_seal_id: contribution_set.reference_seal_id.clone(),
        reference_revision: contribution_set.reference_revision.clone(),
        reference_coverage_id: contribution_set.reference_coverage_id.clone(),
        detector_snapshot_revision: contribution_set.detector_snapshot_revision.clone(),
        detector_output_artifact_id: contribution_set.detector_output_artifact_id.clone(),
        join_id: contribution_set.join_id.clone(),
        join_revision: contribution_set.join_revision.clone(),
        evaluation_join_artifact_id: contribution_set.evaluation_join_artifact_id.clone(),
        join_adjudication_artifact_id: contribution_set.join_adjudication_artifact_id.clone(),
        contribution_set_id: contribution_set.contribution_set_id.clone(),
        contribution_revision: contribution_set.contribution_revision.clone(),
        metric_contributions_artifact_id: contribution_set.metric_contributions_artifact_id.clone(),
        metrics_artifact_id: context.metrics_artifact_id.clone(),
        aggregation_policy_revision: PRIMARY_METRIC_AGGREGATION_POLICY.to_string(),
        zero_denominator_policy_revision: ZERO_DENOMINATOR_POLICY.to_string(),
        report_class,
        primary_metrics_allowed,
        eligible_primary_metrics,
        blocking_reasons,
        qualifies_as_primary_metric_evidence,
        state: MetricAggregateSetState::Complete,
        metrics,
        assessment,
    })
}

fn reject_non_aggregatable_contribution_state(
    contribution_set: &JoinMetricContributionSet,
) -> Result<(), JoinMetricAggregationError> {
    match contribution_set.state {
        MetricContributionSetState::PendingJoinResolution => {
            Err(JoinMetricAggregationError::PendingContributionRejected)
        }
        MetricContributionSetState::Invalidated => {
            Err(JoinMetricAggregationError::ContributionSetInvalidated)
        }
        MetricContributionSetState::Complete => Ok(()),
    }
}

fn derive_metric_records(
    contribution_set: &JoinMetricContributionSet,
) -> Result<Vec<MetricAggregateRecord>, JoinMetricAggregationError> {
    let proposal_precision = aggregate_detector_metric(
        contribution_set,
        PrimaryMetricKind::ProposalPrecision,
        |record| record.proposal_precision,
    )?;
    let duplicate_proposal_burden = aggregate_detector_metric(
        contribution_set,
        PrimaryMetricKind::DuplicateProposalBurden,
        |record| record.duplicate_proposal_burden,
    )?;
    let error_localization_recall = aggregate_reference_metric(
        contribution_set,
        PrimaryMetricKind::ErrorLocalizationRecall,
        |record| record.error_localization_recall,
    )?;
    let correction_exactness_given_localization = aggregate_reference_metric(
        contribution_set,
        PrimaryMetricKind::CorrectionExactnessGivenLocalization,
        |record| record.correction_exactness_given_localization,
    )?;
    let end_to_end_exact_correction_recall = aggregate_reference_metric(
        contribution_set,
        PrimaryMetricKind::EndToEndExactCorrectionRecall,
        |record| record.end_to_end_exact_correction_recall,
    )?;

    Ok(vec![
        proposal_precision,
        error_localization_recall,
        correction_exactness_given_localization,
        end_to_end_exact_correction_recall,
        duplicate_proposal_burden,
    ])
}

fn aggregate_detector_metric(
    contribution_set: &JoinMetricContributionSet,
    metric_kind: PrimaryMetricKind,
    selector: impl Fn(
        &crate::join_metric_contribution::DetectorMetricContributionRecord,
    ) -> RatioContribution,
) -> Result<MetricAggregateRecord, JoinMetricAggregationError> {
    let mut numerator = 0_u64;
    let mut denominator = 0_u64;

    for record in &contribution_set.detector_contributions {
        let contribution = selector(record);
        let (num_inc, den_inc) = contribution_increments(contribution, metric_kind)?;
        numerator = checked_add(numerator, num_inc, metric_kind)?;
        denominator = checked_add(denominator, den_inc, metric_kind)?;
    }

    Ok(metric_record(metric_kind, numerator, denominator))
}

fn aggregate_reference_metric(
    contribution_set: &JoinMetricContributionSet,
    metric_kind: PrimaryMetricKind,
    selector: impl Fn(
        &crate::join_metric_contribution::ReferenceMetricContributionRecord,
    ) -> RatioContribution,
) -> Result<MetricAggregateRecord, JoinMetricAggregationError> {
    let mut numerator = 0_u64;
    let mut denominator = 0_u64;

    for record in &contribution_set.reference_contributions {
        let contribution = selector(record);
        let (num_inc, den_inc) = contribution_increments(contribution, metric_kind)?;
        numerator = checked_add(numerator, num_inc, metric_kind)?;
        denominator = checked_add(denominator, den_inc, metric_kind)?;
    }

    Ok(metric_record(metric_kind, numerator, denominator))
}

fn contribution_increments(
    contribution: RatioContribution,
    _metric_kind: PrimaryMetricKind,
) -> Result<(u64, u64), JoinMetricAggregationError> {
    match contribution {
        RatioContribution::NumeratorAndDenominator => Ok((1, 1)),
        RatioContribution::DenominatorOnly => Ok((0, 1)),
        RatioContribution::Excluded(_) => Ok((0, 0)),
        RatioContribution::PendingAdjudication => {
            Err(JoinMetricAggregationError::PendingContributionInAggregate)
        }
    }
}

fn checked_add(
    current: u64,
    increment: u64,
    metric_kind: PrimaryMetricKind,
) -> Result<u64, JoinMetricAggregationError> {
    current
        .checked_add(increment)
        .ok_or(JoinMetricAggregationError::CountOverflow { metric_kind })
}

fn metric_record(
    metric_kind: PrimaryMetricKind,
    numerator_count: u64,
    denominator_count: u64,
) -> MetricAggregateRecord {
    let value_state = if denominator_count == 0 {
        MetricAggregateValueState::UndefinedZeroDenominator
    } else {
        MetricAggregateValueState::DefinedExactRatio
    };

    MetricAggregateRecord {
        metric_kind,
        numerator_count,
        denominator_count,
        value_state,
    }
}

fn derive_assessment(metrics: &[MetricAggregateRecord]) -> MetricAggregateSetAssessment {
    let defined_metric_count = metrics
        .iter()
        .filter(|record| record.value_state == MetricAggregateValueState::DefinedExactRatio)
        .count() as u32;
    let undefined_zero_denominator_count = metrics
        .iter()
        .filter(|record| record.value_state == MetricAggregateValueState::UndefinedZeroDenominator)
        .count() as u32;
    let all_required_metrics_present = metrics.len() == REQUIRED_METRIC_COUNT as usize
        && validate_metric_inventory(metrics).is_ok();
    let aggregate_consistent = all_required_metrics_present
        && validate_local_record_consistency(metrics).is_ok()
        && validate_cross_metric_invariants(metrics).is_ok();

    MetricAggregateSetAssessment {
        required_metric_count: REQUIRED_METRIC_COUNT,
        aggregate_metric_count: metrics.len() as u32,
        defined_metric_count,
        undefined_zero_denominator_count,
        all_required_metrics_present,
        aggregate_consistent,
    }
}

fn validate_metric_inventory(
    metrics: &[MetricAggregateRecord],
) -> Result<(), JoinMetricAggregationError> {
    if metrics.len() != REQUIRED_METRIC_COUNT as usize {
        return Err(JoinMetricAggregationError::MissingMetricKind {
            metric_kind: PrimaryMetricKind::ProposalPrecision,
        });
    }

    let expected = all_primary_metric_kinds();
    for (index, expected_kind) in expected.iter().enumerate() {
        if metrics[index].metric_kind != *expected_kind {
            return Err(JoinMetricAggregationError::NonCanonicalMetricOrder);
        }
    }

    let mut seen = std::collections::HashSet::new();
    for record in metrics {
        if !seen.insert(record.metric_kind) {
            return Err(JoinMetricAggregationError::DuplicateMetricKind {
                metric_kind: record.metric_kind,
            });
        }
    }

    Ok(())
}

fn validate_local_record_consistency(
    metrics: &[MetricAggregateRecord],
) -> Result<(), JoinMetricAggregationError> {
    for record in metrics {
        match record.value_state {
            MetricAggregateValueState::DefinedExactRatio => {
                if record.denominator_count == 0 {
                    return Err(
                        JoinMetricAggregationError::ZeroDenominatorValueStateMismatch {
                            metric_kind: record.metric_kind,
                        },
                    );
                }
                if record.numerator_count > record.denominator_count {
                    return Err(JoinMetricAggregationError::NumeratorExceedsDenominator {
                        metric_kind: record.metric_kind,
                    });
                }
            }
            MetricAggregateValueState::UndefinedZeroDenominator => {
                if record.denominator_count != 0 {
                    return Err(
                        JoinMetricAggregationError::NonZeroDenominatorValueStateMismatch {
                            metric_kind: record.metric_kind,
                        },
                    );
                }
                if record.numerator_count != 0 {
                    return Err(
                        JoinMetricAggregationError::ZeroDenominatorNonZeroNumerator {
                            metric_kind: record.metric_kind,
                        },
                    );
                }
            }
        }
    }

    Ok(())
}

fn validate_cross_metric_invariants(
    metrics: &[MetricAggregateRecord],
) -> Result<(), JoinMetricAggregationError> {
    let by_kind = |kind: PrimaryMetricKind| -> Option<&MetricAggregateRecord> {
        metrics.iter().find(|record| record.metric_kind == kind)
    };

    let proposal_precision = by_kind(PrimaryMetricKind::ProposalPrecision).ok_or(
        JoinMetricAggregationError::MissingMetricKind {
            metric_kind: PrimaryMetricKind::ProposalPrecision,
        },
    )?;
    let duplicate_burden = by_kind(PrimaryMetricKind::DuplicateProposalBurden).ok_or(
        JoinMetricAggregationError::MissingMetricKind {
            metric_kind: PrimaryMetricKind::DuplicateProposalBurden,
        },
    )?;
    let localization = by_kind(PrimaryMetricKind::ErrorLocalizationRecall).ok_or(
        JoinMetricAggregationError::MissingMetricKind {
            metric_kind: PrimaryMetricKind::ErrorLocalizationRecall,
        },
    )?;
    let correction_exactness = by_kind(PrimaryMetricKind::CorrectionExactnessGivenLocalization)
        .ok_or(JoinMetricAggregationError::MissingMetricKind {
            metric_kind: PrimaryMetricKind::CorrectionExactnessGivenLocalization,
        })?;
    let end_to_end = by_kind(PrimaryMetricKind::EndToEndExactCorrectionRecall).ok_or(
        JoinMetricAggregationError::MissingMetricKind {
            metric_kind: PrimaryMetricKind::EndToEndExactCorrectionRecall,
        },
    )?;

    if proposal_precision.denominator_count != duplicate_burden.denominator_count {
        return Err(JoinMetricAggregationError::CrossMetricInvariantViolation {
            invariant: "proposal_precision_denominator_equals_duplicate_proposal_burden_denominator",
        });
    }

    if correction_exactness.denominator_count != localization.numerator_count {
        return Err(JoinMetricAggregationError::CrossMetricInvariantViolation {
            invariant: "correction_exactness_denominator_equals_localization_numerator",
        });
    }

    if end_to_end.denominator_count != localization.denominator_count {
        return Err(JoinMetricAggregationError::CrossMetricInvariantViolation {
            invariant: "end_to_end_denominator_equals_localization_denominator",
        });
    }

    if end_to_end.numerator_count != correction_exactness.numerator_count {
        return Err(JoinMetricAggregationError::CrossMetricInvariantViolation {
            invariant: "end_to_end_numerator_equals_correction_exactness_numerator",
        });
    }

    for record in metrics {
        if record.numerator_count > record.denominator_count && record.denominator_count > 0 {
            return Err(JoinMetricAggregationError::NumeratorExceedsDenominator {
                metric_kind: record.metric_kind,
            });
        }
    }

    Ok(())
}

fn validate_local_posture_consistency(
    set: &JoinMetricAggregateSet,
) -> Result<(), JoinMetricAggregationError> {
    if set.primary_metrics_allowed != set.blocking_reasons.is_empty() {
        return Err(JoinMetricAggregationError::PrimaryEligibilityInconsistent);
    }

    let mut stored_metrics = set.eligible_primary_metrics.clone();
    sort_primary_metric_kinds(&mut stored_metrics);
    if set.eligible_primary_metrics != stored_metrics {
        return Err(JoinMetricAggregationError::PrimaryEligibilityInconsistent);
    }

    if set.primary_metrics_allowed {
        if set.eligible_primary_metrics != all_primary_metric_kinds() {
            return Err(JoinMetricAggregationError::PrimaryEligibilityInconsistent);
        }
        if set.report_class != MetricContributionReportClass::PrimaryBlindCalibration {
            return Err(JoinMetricAggregationError::ReportClassInconsistent);
        }
    } else if !set.eligible_primary_metrics.is_empty() {
        return Err(JoinMetricAggregationError::PrimaryEligibilityInconsistent);
    } else if set.report_class == MetricContributionReportClass::PrimaryBlindCalibration {
        return Err(JoinMetricAggregationError::ReportClassInconsistent);
    }

    let mut blocking_reasons = set.blocking_reasons.clone();
    sort_blocking_reasons(&mut blocking_reasons);
    if set.blocking_reasons != blocking_reasons {
        return Err(JoinMetricAggregationError::PrimaryEligibilityInconsistent);
    }

    if set
        .blocking_reasons
        .contains(&PrimaryMetricBlockingReason::SyntheticProtocolOnly)
        && set.report_class != MetricContributionReportClass::SyntheticProtocolValidation
    {
        return Err(JoinMetricAggregationError::ReportClassInconsistent);
    }

    let derived_primary_evidence = set.primary_metrics_allowed
        && set.state == MetricAggregateSetState::Complete
        && set.assessment.all_required_metrics_present
        && set.assessment.aggregate_consistent;

    if set.qualifies_as_primary_metric_evidence != derived_primary_evidence {
        return Err(JoinMetricAggregationError::PrimaryMetricEvidenceDerivationMismatch);
    }

    Ok(())
}

fn is_aggregate_creation_lifecycle(state: RunLifecycleState) -> bool {
    matches!(
        state,
        RunLifecycleState::DetectorExecution | RunLifecycleState::AssistedReview
    )
}

fn is_aggregate_historical_lifecycle(state: RunLifecycleState) -> bool {
    matches!(
        state,
        RunLifecycleState::DetectorExecution
            | RunLifecycleState::AssistedReview
            | RunLifecycleState::Finalized
    )
}

fn validate_aggregate_creation_lifecycle(
    lifecycle_state: RunLifecycleState,
) -> Result<(), JoinMetricAggregationError> {
    if !is_aggregate_creation_lifecycle(lifecycle_state) {
        return Err(
            JoinMetricAggregationError::AggregateCreationLifecycleIncompatible { lifecycle_state },
        );
    }
    Ok(())
}

fn validate_aggregate_inputs(
    envelope: &RunEnvelope,
    seal: &ReferenceSeal,
    coverage: &ReferenceCoverage,
    human_reference: &HumanFinalReference,
    detector_snapshot: &DetectorProposalSnapshot,
    join: &DetectorReferenceJoin,
    adjudication_set: &OverlapAdjudicationSet,
    contribution_set: &JoinMetricContributionSet,
    artifact_bundle: &ArtifactBundle,
    context: &JoinMetricAggregateContext,
    mode: DeriveMode,
) -> Result<(), JoinMetricAggregationError> {
    envelope
        .validate()
        .map_err(JoinMetricAggregationError::EnvelopeValidation)?;

    if envelope.lifecycle_state == RunLifecycleState::Invalidated {
        return Err(JoinMetricAggregationError::InvalidatedAggregateContext);
    }

    match mode {
        DeriveMode::Creation => validate_aggregate_creation_lifecycle(envelope.lifecycle_state)?,
        DeriveMode::Historical => {
            if !is_aggregate_historical_lifecycle(envelope.lifecycle_state) {
                return Err(
                    JoinMetricAggregationError::AggregateHistoricalLifecycleIncompatible {
                        lifecycle_state: envelope.lifecycle_state,
                    },
                );
            }
        }
    }

    seal.validate_historical_context(envelope)
        .map_err(JoinMetricAggregationError::SealValidation)?;
    coverage
        .validate_historical_context(envelope, seal, Some(human_reference))
        .map_err(JoinMetricAggregationError::CoverageValidation)?;
    human_reference
        .validate_historical_context(envelope, seal)
        .map_err(|error| JoinMetricAggregationError::HumanReferenceValidation(Box::new(error)))?;

    if detector_snapshot.state != DetectorProposalSnapshotState::Frozen {
        return Err(JoinMetricAggregationError::SnapshotNotFrozen);
    }

    detector_snapshot
        .validate_against_bundle(envelope, artifact_bundle)
        .map_err(JoinMetricAggregationError::SnapshotValidation)?;

    artifact_bundle
        .validate_with_reference_context(
            envelope,
            Some(seal),
            Some(coverage),
            Some(human_reference),
        )
        .map_err(JoinMetricAggregationError::BundleValidation)?;

    validate_aggregate_bundle_roles(artifact_bundle, context, contribution_set, join)?;

    adjudication_set
        .validate_frozen_for_join(
            envelope,
            &seal.reference_revision,
            &detector_snapshot.snapshot_revision,
        )
        .map_err(JoinMetricAggregationError::AdjudicationValidation)?;

    validate_join_lineage_binding(envelope, seal, coverage, detector_snapshot, join)?;
    validate_contribution_lineage_binding(contribution_set, join)?;

    if contribution_set.state != MetricContributionSetState::Complete {
        return Err(JoinMetricAggregationError::ContributionSetNotComplete);
    }

    if context.aggregate_set_id.as_str().is_empty()
        || context.aggregate_revision.as_str().is_empty()
    {
        return Err(JoinMetricAggregationError::InvalidAggregateSetId(
            MetricAggregateIdentityIdError::Empty,
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
) -> Result<(), JoinMetricAggregationError> {
    if join.run_id != envelope.run_id
        || seal.run_id != envelope.run_id
        || coverage.run_id != envelope.run_id
        || detector_snapshot.run_id != envelope.run_id
    {
        return Err(JoinMetricAggregationError::RunIdMismatch);
    }

    if join.input_identity != envelope.input_identity
        || seal.input_identity != envelope.input_identity
        || coverage.input_identity != envelope.input_identity
        || detector_snapshot.input_identity != envelope.input_identity
    {
        return Err(JoinMetricAggregationError::InputIdentityMismatch);
    }

    if join.reference_seal_id != seal.seal_id {
        return Err(JoinMetricAggregationError::TopLevelBindingMismatch {
            field: "reference_seal_id",
        });
    }

    if join.reference_revision != seal.reference_revision
        || join.reference_revision != coverage.reference_revision
    {
        return Err(JoinMetricAggregationError::ReferenceRevisionMismatch);
    }

    if join.reference_coverage_id != coverage.coverage_id {
        return Err(JoinMetricAggregationError::CoverageIdMismatch);
    }

    if join.detector_snapshot_revision != detector_snapshot.snapshot_revision {
        return Err(JoinMetricAggregationError::SnapshotRevisionMismatch);
    }

    if join.detector_output_artifact_id != detector_snapshot.detector_output_artifact_id {
        return Err(JoinMetricAggregationError::DetectorOutputArtifactMismatch);
    }

    Ok(())
}

fn validate_contribution_lineage_binding(
    contribution_set: &JoinMetricContributionSet,
    join: &DetectorReferenceJoin,
) -> Result<(), JoinMetricAggregationError> {
    if contribution_set.join_id != join.join_id {
        return Err(JoinMetricAggregationError::JoinIdMismatch);
    }
    if contribution_set.join_revision != join.join_revision {
        return Err(JoinMetricAggregationError::JoinRevisionMismatch);
    }
    if contribution_set.evaluation_join_artifact_id != join.evaluation_join_artifact_id {
        return Err(JoinMetricAggregationError::EvaluationJoinArtifactMismatch);
    }
    if contribution_set.join_adjudication_artifact_id != join.join_adjudication_artifact_id {
        return Err(JoinMetricAggregationError::JoinAdjudicationArtifactMismatch);
    }

    Ok(())
}

fn validate_aggregate_bundle_roles(
    bundle: &ArtifactBundle,
    context: &JoinMetricAggregateContext,
    contribution_set: &JoinMetricContributionSet,
    join: &DetectorReferenceJoin,
) -> Result<(), JoinMetricAggregationError> {
    let evaluation_joins = bundle
        .artifacts
        .iter()
        .filter(|descriptor| descriptor.role == ArtifactRole::EvaluationJoin)
        .collect::<Vec<_>>();
    if evaluation_joins.len() != 1
        || evaluation_joins[0].artifact_id != join.evaluation_join_artifact_id
    {
        return Err(JoinMetricAggregationError::EvaluationJoinArtifactMismatch);
    }

    let adjudication_roles = bundle
        .artifacts
        .iter()
        .filter(|descriptor| descriptor.role == ArtifactRole::JoinAdjudication)
        .collect::<Vec<_>>();
    if adjudication_roles.len() != 1
        || adjudication_roles[0].artifact_id != join.join_adjudication_artifact_id
    {
        return Err(JoinMetricAggregationError::JoinAdjudicationArtifactMismatch);
    }

    let detector_outputs = bundle
        .artifacts
        .iter()
        .filter(|descriptor| descriptor.role == ArtifactRole::DetectorOutput)
        .collect::<Vec<_>>();
    if detector_outputs.len() != 1
        || detector_outputs[0].artifact_id != join.detector_output_artifact_id
    {
        return Err(JoinMetricAggregationError::DetectorOutputArtifactMismatch);
    }

    let metric_contributions = bundle
        .artifacts
        .iter()
        .filter(|descriptor| descriptor.role == ArtifactRole::MetricContributions)
        .collect::<Vec<_>>();
    if metric_contributions.len() != 1 {
        return Err(JoinMetricAggregationError::MetricContributionsArtifactMismatch);
    }
    if metric_contributions[0].artifact_id != contribution_set.metric_contributions_artifact_id {
        return Err(JoinMetricAggregationError::MetricContributionsArtifactMismatch);
    }

    let metrics = bundle
        .artifacts
        .iter()
        .filter(|descriptor| descriptor.role == ArtifactRole::Metrics)
        .collect::<Vec<_>>();
    if metrics.len() != 1 {
        return Err(JoinMetricAggregationError::MetricsArtifactMismatch);
    }
    if metrics[0].artifact_id != context.metrics_artifact_id {
        return Err(JoinMetricAggregationError::MetricsArtifactMismatch);
    }

    Ok(())
}

fn authoritative_aggregate_context_for_validation(
    stored: &JoinMetricAggregateSet,
    artifact_bundle: &ArtifactBundle,
) -> Result<JoinMetricAggregateContext, JoinMetricAggregationError> {
    let metrics = artifact_bundle
        .artifacts
        .iter()
        .filter(|descriptor| descriptor.role == ArtifactRole::Metrics)
        .collect::<Vec<_>>();
    if metrics.len() != 1 {
        return Err(JoinMetricAggregationError::MetricsArtifactMismatch);
    }
    if metrics[0].artifact_id != stored.metrics_artifact_id {
        return Err(JoinMetricAggregationError::MetricsArtifactMismatch);
    }

    Ok(JoinMetricAggregateContext {
        aggregate_set_id: stored.aggregate_set_id.clone(),
        aggregate_revision: stored.aggregate_revision.clone(),
        metrics_artifact_id: stored.metrics_artifact_id.clone(),
    })
}

fn compare_stored_aggregate_set(
    stored: &JoinMetricAggregateSet,
    derived: &JoinMetricAggregateSet,
) -> Result<(), JoinMetricAggregationError> {
    stored.validate()?;

    let fields: [(&str, bool); 29] = [
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
            "contribution_set_id",
            stored.contribution_set_id == derived.contribution_set_id,
        ),
        (
            "contribution_revision",
            stored.contribution_revision == derived.contribution_revision,
        ),
        (
            "metric_contributions_artifact_id",
            stored.metric_contributions_artifact_id == derived.metric_contributions_artifact_id,
        ),
        (
            "metrics_artifact_id",
            stored.metrics_artifact_id == derived.metrics_artifact_id,
        ),
        (
            "aggregation_policy_revision",
            stored.aggregation_policy_revision == derived.aggregation_policy_revision,
        ),
        (
            "zero_denominator_policy_revision",
            stored.zero_denominator_policy_revision == derived.zero_denominator_policy_revision,
        ),
        ("report_class", stored.report_class == derived.report_class),
        (
            "primary_metrics_allowed",
            stored.primary_metrics_allowed == derived.primary_metrics_allowed,
        ),
        (
            "eligible_primary_metrics",
            stored.eligible_primary_metrics == derived.eligible_primary_metrics,
        ),
        (
            "blocking_reasons",
            stored.blocking_reasons == derived.blocking_reasons,
        ),
        (
            "qualifies_as_primary_metric_evidence",
            stored.qualifies_as_primary_metric_evidence
                == derived.qualifies_as_primary_metric_evidence,
        ),
        ("state", stored.state == derived.state),
        ("metrics", stored.metrics == derived.metrics),
        ("assessment", stored.assessment == derived.assessment),
        (
            "aggregate_set_id",
            stored.aggregate_set_id == derived.aggregate_set_id,
        ),
        (
            "aggregate_revision",
            stored.aggregate_revision == derived.aggregate_revision,
        ),
    ];

    for (field, matches) in fields {
        if !matches {
            return Err(JoinMetricAggregationError::TopLevelBindingMismatch { field });
        }
    }

    Ok(())
}

fn validate_policy_revision(
    field: &'static str,
    found: &str,
    expected: &str,
) -> Result<(), JoinMetricAggregationError> {
    if found != expected {
        return Err(JoinMetricAggregationError::UnsupportedPolicyRevision {
            field,
            found: found.to_string(),
            expected: expected.to_string(),
        });
    }
    Ok(())
}

pub fn validate_aggregate_id_value(value: &str) -> Result<(), MetricAggregateIdentityIdError> {
    validate_join_id_value(value).map_err(map_join_identity_error)
}

fn map_join_identity_error(error: JoinIdentityIdError) -> MetricAggregateIdentityIdError {
    match error {
        JoinIdentityIdError::Empty => MetricAggregateIdentityIdError::Empty,
        JoinIdentityIdError::TooLong { len, max } => {
            MetricAggregateIdentityIdError::TooLong { len, max }
        }
        JoinIdentityIdError::InvalidCharacter { character } => {
            MetricAggregateIdentityIdError::InvalidCharacter { character }
        }
        JoinIdentityIdError::PathLikeContent => MetricAggregateIdentityIdError::PathLikeContent,
        JoinIdentityIdError::AbsolutePathLike => MetricAggregateIdentityIdError::AbsolutePathLike,
        JoinIdentityIdError::RelativePathLike => MetricAggregateIdentityIdError::RelativePathLike,
        JoinIdentityIdError::HomeDirectoryFragment => {
            MetricAggregateIdentityIdError::HomeDirectoryFragment
        }
        JoinIdentityIdError::GenerationUnavailable => {
            MetricAggregateIdentityIdError::GenerationUnavailable
        }
    }
}

fn map_run_id_error(error: RunIdError) -> MetricAggregateIdentityIdError {
    match error {
        RunIdError::Empty => MetricAggregateIdentityIdError::Empty,
        RunIdError::TooLong { len, max } => MetricAggregateIdentityIdError::TooLong { len, max },
        RunIdError::InvalidCharacter { character } => {
            MetricAggregateIdentityIdError::InvalidCharacter { character }
        }
        RunIdError::PathLikeContent => MetricAggregateIdentityIdError::PathLikeContent,
        RunIdError::AbsolutePathLike => MetricAggregateIdentityIdError::AbsolutePathLike,
        RunIdError::RelativePathLike => MetricAggregateIdentityIdError::RelativePathLike,
        RunIdError::HomeDirectoryFragment => MetricAggregateIdentityIdError::HomeDirectoryFragment,
        RunIdError::GenerationUnavailable => MetricAggregateIdentityIdError::GenerationUnavailable,
    }
}

pub fn aggregate_from_json(json: &str) -> Result<JoinMetricAggregateSet, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn aggregate_to_json(set: &JoinMetricAggregateSet) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(set)
}

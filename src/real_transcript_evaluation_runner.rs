use std::fmt;

use crate::detector_snapshot::{DetectorAnalysisIdentity, DetectorAnalysisIdentityValidationError};
use crate::human_final_reference::{HumanFinalReference, HumanFinalReferenceValidationError};
use crate::input_authorization::{
    INPUT_AUTHORIZATION_SCOPE_POLICY, InputAuthorization, InputAuthorizationState,
    InputAuthorizationValidationError, validate_input_class_and_basis,
};
use crate::join_adjudication::OverlapAdjudicatorRole;
use crate::reference_coverage::{
    ReferenceCoverage, ReferenceCoveragePurpose, ReferenceCoverageState,
    ReferenceCoverageValidationError,
};
use crate::reference_seal::{
    CalibrationValidityImpact, ReferenceCalibrationValidity, ReferenceProducerClass, ReferenceSeal,
    ReferenceSealState, ReferenceSealValidationError,
};
use crate::run_manifest::{
    ArtifactRole, CalibrationValidityMode, InputClass, InputIdentityReference, RunEnvelope,
    RunEnvelopeValidationError, RunId, RunLifecycleState,
};

pub const REAL_TRANSCRIPT_EVALUATION_RUNNER_REQUEST_SCHEMA: &str =
    "voxproof-real-transcript-evaluation-runner-request-v1";
pub const REAL_TRANSCRIPT_EVALUATION_RUNNER_POLICY: &str =
    "voxproof-real-transcript-evaluation-runner-v1";
pub const REAL_TRANSCRIPT_OVERLAP_AUTHORITY_POLICY: &str =
    "voxproof-human-overlap-authority-required-v1";

/// Typed request declaring protocol readiness for a future real-transcript
/// evaluation run. Successful validation establishes only readiness to enter a
/// later DetectorExecution implementation. It does not establish legal
/// sufficiency, actual real-material presence, detector execution, adjudication,
/// metrics, packet serialization, or filesystem persistence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealTranscriptEvaluationRunRequest {
    pub schema_revision: String,
    pub runner_policy_revision: String,
    pub overlap_authority_policy_revision: String,
    pub input_authorization: InputAuthorization,
    pub declared_envelope: RunEnvelope,
    pub reference_preparation_envelope: RunEnvelope,
    pub reference_sealed_envelope: RunEnvelope,
    pub detector_execution_envelope: RunEnvelope,
    pub assisted_review_transition_envelope: RunEnvelope,
    pub finalized_envelope: RunEnvelope,
    pub reference_seal: ReferenceSeal,
    pub reference_coverage: ReferenceCoverage,
    pub human_final_reference: HumanFinalReference,
    pub detector_analysis_identity: DetectorAnalysisIdentity,
    pub expected_artifact_roles: Vec<ArtifactRole>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedRealTranscriptEvaluationRunPlan {
    pub run_id: RunId,
    pub input_identity: InputIdentityReference,
    pub input_class: InputClass,
    pub workflow_observation: crate::run_manifest::WorkflowObservationMode,
    pub authorization_id: crate::input_authorization::InputAuthorizationId,
    pub reference_seal_id: crate::reference_seal::ReferenceSealId,
    pub reference_revision: crate::reference_identity::ReferenceRevisionId,
    pub reference_coverage_id: crate::reference_coverage::ReferenceCoverageId,
    pub detector_analysis_identity: DetectorAnalysisIdentity,
    pub expected_artifact_roles: Vec<ArtifactRole>,
    pub readiness: RealTranscriptEvaluationRunReadiness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealTranscriptEvaluationRunReadiness {
    ReadyForDetectorExecution,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopePostureField {
    SchemaRevision,
    RunId,
    InputIdentity,
    CalibrationValidity,
    WorkflowObservation,
    InputClass,
    QualifiesAsRealMaterialEvidence,
    ExpectedArtifactRoles,
}

impl fmt::Display for EnvelopePostureField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::SchemaRevision => "schema_revision",
            Self::RunId => "run_id",
            Self::InputIdentity => "input_identity",
            Self::CalibrationValidity => "calibration_validity",
            Self::WorkflowObservation => "workflow_observation",
            Self::InputClass => "input_class",
            Self::QualifiesAsRealMaterialEvidence => "qualifies_as_real_material_evidence",
            Self::ExpectedArtifactRoles => "expected_artifact_roles",
        };
        f.write_str(label)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RealTranscriptEvaluationRunnerContractError {
    MissingSchemaRevision,
    UnsupportedSchemaRevision {
        found: String,
        expected: String,
    },
    UnsupportedRunnerPolicy {
        found: String,
        expected: String,
    },
    UnsupportedOverlapAuthorityPolicy {
        found: String,
        expected: String,
    },
    InputAuthorizationValidationFailure(InputAuthorizationValidationError),
    InputAuthorizationNotConfirmed,
    InputAuthorizationRunMismatch,
    InputAuthorizationIdentityMismatch,
    InputAuthorizationClassMismatch,
    UnsupportedRealInputClass {
        input_class: InputClass,
    },
    RunnerNotBlindReference,
    RunnerNotQualifiedAsRealMaterial,
    InvalidLifecycleEnvelope {
        lifecycle_state: RunLifecycleState,
    },
    EnvelopePostureMismatch {
        field: EnvelopePostureField,
        lifecycle_state: RunLifecycleState,
    },
    IllegalLifecycleTransition {
        from: RunLifecycleState,
        to: RunLifecycleState,
    },
    ExpectedArtifactInventoryMismatch,
    RequestEnvelopeArtifactInventoryMismatch,
    DuplicateExpectedArtifactRole {
        role: ArtifactRole,
    },
    ReferenceSealValidationFailure(ReferenceSealValidationError),
    ReferenceSealNotBlindEligible,
    ReferenceSealBindingMismatch,
    ReferenceCoverageValidationFailure(ReferenceCoverageValidationError),
    ReferenceCoverageNotPrimary,
    ReferenceCoverageIncomplete,
    ReferenceCoverageBindingMismatch,
    HumanFinalReferenceValidationFailure(HumanFinalReferenceValidationError),
    HumanFinalReferenceNotSealed,
    HumanFinalReferenceBindingMismatch,
    DetectorAnalysisIdentityValidationFailure(DetectorAnalysisIdentityValidationError),
    DetectorAnalysisIdentityInputMismatch,
    EnvelopeValidation(RunEnvelopeValidationError),
}

pub fn canonical_real_evaluation_artifact_roles() -> Vec<ArtifactRole> {
    vec![
        ArtifactRole::InputAuthorization,
        ArtifactRole::ReferenceSeal,
        ArtifactRole::HumanFinalReference,
        ArtifactRole::CueReviewCompletion,
        ArtifactRole::DetectorOutput,
        ArtifactRole::EvaluationJoin,
        ArtifactRole::JoinAdjudication,
        ArtifactRole::MetricContributions,
        ArtifactRole::Metrics,
    ]
}

pub fn real_evaluation_overlap_authority_roles() -> &'static [OverlapAdjudicatorRole] {
    &[
        OverlapAdjudicatorRole::OwnerAdjudicator,
        OverlapAdjudicatorRole::AuthorizedDomainAdjudicator,
    ]
}

pub fn real_evaluation_forbidden_overlap_authority_roles() -> &'static [OverlapAdjudicatorRole] {
    &[OverlapAdjudicatorRole::SyntheticFixtureAdjudicator]
}

pub fn validate_real_transcript_evaluation_run_request(
    request: &RealTranscriptEvaluationRunRequest,
) -> Result<ValidatedRealTranscriptEvaluationRunPlan, RealTranscriptEvaluationRunnerContractError> {
    validate_request_revisions(request)?;
    request.input_authorization.validate().map_err(
        RealTranscriptEvaluationRunnerContractError::InputAuthorizationValidationFailure,
    )?;

    validate_lifecycle_envelope_states(request)?;
    validate_immutable_envelope_posture(request)?;

    let common_posture = &request.declared_envelope;
    validate_real_run_posture(common_posture)?;
    validate_lifecycle_transitions(request, common_posture.calibration_validity)?;
    validate_artifact_inventory(&request.expected_artifact_roles)?;
    validate_request_envelope_inventory_binding(request)?;
    validate_authorization_binding(request, common_posture)?;
    validate_reference_readiness(request)?;
    validate_detector_analysis_readiness(request, common_posture)?;

    Ok(ValidatedRealTranscriptEvaluationRunPlan {
        run_id: common_posture.run_id.clone(),
        input_identity: common_posture.input_identity.clone(),
        input_class: common_posture.input_class,
        workflow_observation: common_posture.workflow_observation,
        authorization_id: request.input_authorization.authorization_id.clone(),
        reference_seal_id: request.reference_seal.seal_id.clone(),
        reference_revision: request.reference_seal.reference_revision.clone(),
        reference_coverage_id: request.reference_coverage.coverage_id.clone(),
        detector_analysis_identity: request.detector_analysis_identity.clone(),
        expected_artifact_roles: request.expected_artifact_roles.clone(),
        readiness: RealTranscriptEvaluationRunReadiness::ReadyForDetectorExecution,
    })
}

fn validate_request_revisions(
    request: &RealTranscriptEvaluationRunRequest,
) -> Result<(), RealTranscriptEvaluationRunnerContractError> {
    if request.schema_revision.is_empty() {
        return Err(RealTranscriptEvaluationRunnerContractError::MissingSchemaRevision);
    }

    if request.schema_revision != REAL_TRANSCRIPT_EVALUATION_RUNNER_REQUEST_SCHEMA {
        return Err(
            RealTranscriptEvaluationRunnerContractError::UnsupportedSchemaRevision {
                found: request.schema_revision.clone(),
                expected: REAL_TRANSCRIPT_EVALUATION_RUNNER_REQUEST_SCHEMA.to_string(),
            },
        );
    }

    if request.runner_policy_revision != REAL_TRANSCRIPT_EVALUATION_RUNNER_POLICY {
        return Err(
            RealTranscriptEvaluationRunnerContractError::UnsupportedRunnerPolicy {
                found: request.runner_policy_revision.clone(),
                expected: REAL_TRANSCRIPT_EVALUATION_RUNNER_POLICY.to_string(),
            },
        );
    }

    if request.overlap_authority_policy_revision != REAL_TRANSCRIPT_OVERLAP_AUTHORITY_POLICY {
        return Err(
            RealTranscriptEvaluationRunnerContractError::UnsupportedOverlapAuthorityPolicy {
                found: request.overlap_authority_policy_revision.clone(),
                expected: REAL_TRANSCRIPT_OVERLAP_AUTHORITY_POLICY.to_string(),
            },
        );
    }

    Ok(())
}

fn lifecycle_envelopes(
    request: &RealTranscriptEvaluationRunRequest,
) -> [(RunLifecycleState, &RunEnvelope); 6] {
    [
        (RunLifecycleState::Declared, &request.declared_envelope),
        (
            RunLifecycleState::ReferencePreparation,
            &request.reference_preparation_envelope,
        ),
        (
            RunLifecycleState::ReferenceSealed,
            &request.reference_sealed_envelope,
        ),
        (
            RunLifecycleState::DetectorExecution,
            &request.detector_execution_envelope,
        ),
        (
            RunLifecycleState::AssistedReview,
            &request.assisted_review_transition_envelope,
        ),
        (RunLifecycleState::Finalized, &request.finalized_envelope),
    ]
}

fn validate_lifecycle_envelope_states(
    request: &RealTranscriptEvaluationRunRequest,
) -> Result<(), RealTranscriptEvaluationRunnerContractError> {
    let envelopes = lifecycle_envelopes(request);

    for (expected_state, envelope) in &envelopes {
        envelope
            .validate()
            .map_err(RealTranscriptEvaluationRunnerContractError::EnvelopeValidation)?;

        if envelope.lifecycle_state != *expected_state {
            return Err(
                RealTranscriptEvaluationRunnerContractError::InvalidLifecycleEnvelope {
                    lifecycle_state: envelope.lifecycle_state,
                },
            );
        }
    }

    Ok(())
}

fn validate_lifecycle_transitions(
    request: &RealTranscriptEvaluationRunRequest,
    calibration_validity: CalibrationValidityMode,
) -> Result<(), RealTranscriptEvaluationRunnerContractError> {
    let envelopes = lifecycle_envelopes(request);

    for window in envelopes.windows(2) {
        let (from_state, _) = window[0];
        let (_, to_envelope) = window[1];
        RunEnvelope::validate_transition(
            from_state,
            to_envelope.lifecycle_state,
            calibration_validity,
        )
        .map_err(|_| {
            RealTranscriptEvaluationRunnerContractError::IllegalLifecycleTransition {
                from: from_state,
                to: to_envelope.lifecycle_state,
            }
        })?;
    }

    Ok(())
}

fn validate_immutable_envelope_posture(
    request: &RealTranscriptEvaluationRunRequest,
) -> Result<(), RealTranscriptEvaluationRunnerContractError> {
    let reference = &request.declared_envelope;
    for (_, envelope) in lifecycle_envelopes(request).into_iter().skip(1) {
        compare_envelope_posture(reference, envelope)?;
    }
    Ok(())
}

fn compare_envelope_posture(
    reference: &RunEnvelope,
    envelope: &RunEnvelope,
) -> Result<(), RealTranscriptEvaluationRunnerContractError> {
    let lifecycle_state = envelope.lifecycle_state;

    if envelope.schema_revision != reference.schema_revision {
        return Err(envelope_posture_mismatch(
            EnvelopePostureField::SchemaRevision,
            lifecycle_state,
        ));
    }
    if envelope.run_id != reference.run_id {
        return Err(envelope_posture_mismatch(
            EnvelopePostureField::RunId,
            lifecycle_state,
        ));
    }
    if envelope.input_identity != reference.input_identity {
        return Err(envelope_posture_mismatch(
            EnvelopePostureField::InputIdentity,
            lifecycle_state,
        ));
    }
    if envelope.calibration_validity != reference.calibration_validity {
        return Err(envelope_posture_mismatch(
            EnvelopePostureField::CalibrationValidity,
            lifecycle_state,
        ));
    }
    if envelope.workflow_observation != reference.workflow_observation {
        return Err(envelope_posture_mismatch(
            EnvelopePostureField::WorkflowObservation,
            lifecycle_state,
        ));
    }
    if envelope.input_class != reference.input_class {
        return Err(envelope_posture_mismatch(
            EnvelopePostureField::InputClass,
            lifecycle_state,
        ));
    }
    if envelope.qualifies_as_real_material_evidence != reference.qualifies_as_real_material_evidence
    {
        return Err(envelope_posture_mismatch(
            EnvelopePostureField::QualifiesAsRealMaterialEvidence,
            lifecycle_state,
        ));
    }
    if envelope.expected_artifact_roles != reference.expected_artifact_roles {
        return Err(envelope_posture_mismatch(
            EnvelopePostureField::ExpectedArtifactRoles,
            lifecycle_state,
        ));
    }

    Ok(())
}

fn envelope_posture_mismatch(
    field: EnvelopePostureField,
    lifecycle_state: RunLifecycleState,
) -> RealTranscriptEvaluationRunnerContractError {
    RealTranscriptEvaluationRunnerContractError::EnvelopePostureMismatch {
        field,
        lifecycle_state,
    }
}

fn validate_real_run_posture(
    envelope: &RunEnvelope,
) -> Result<(), RealTranscriptEvaluationRunnerContractError> {
    if envelope.calibration_validity != CalibrationValidityMode::BlindReference {
        return Err(RealTranscriptEvaluationRunnerContractError::RunnerNotBlindReference);
    }

    match envelope.input_class {
        InputClass::SelfOwnedReal | InputClass::ExplicitPermissionReal => {}
        input_class => {
            return Err(
                RealTranscriptEvaluationRunnerContractError::UnsupportedRealInputClass {
                    input_class,
                },
            );
        }
    }

    if !envelope.qualifies_as_real_material_evidence {
        return Err(RealTranscriptEvaluationRunnerContractError::RunnerNotQualifiedAsRealMaterial);
    }

    Ok(())
}

fn validate_artifact_inventory(
    roles: &[ArtifactRole],
) -> Result<(), RealTranscriptEvaluationRunnerContractError> {
    let mut seen = std::collections::HashSet::new();
    for role in roles {
        if !seen.insert(*role) {
            return Err(
                RealTranscriptEvaluationRunnerContractError::DuplicateExpectedArtifactRole {
                    role: *role,
                },
            );
        }
    }

    let canonical = canonical_real_evaluation_artifact_roles();
    if roles != canonical.as_slice() {
        return Err(RealTranscriptEvaluationRunnerContractError::ExpectedArtifactInventoryMismatch);
    }

    Ok(())
}

fn validate_request_envelope_inventory_binding(
    request: &RealTranscriptEvaluationRunRequest,
) -> Result<(), RealTranscriptEvaluationRunnerContractError> {
    if request.expected_artifact_roles != request.declared_envelope.expected_artifact_roles {
        return Err(
            RealTranscriptEvaluationRunnerContractError::RequestEnvelopeArtifactInventoryMismatch,
        );
    }

    Ok(())
}

fn validate_authorization_binding(
    request: &RealTranscriptEvaluationRunRequest,
    posture: &RunEnvelope,
) -> Result<(), RealTranscriptEvaluationRunnerContractError> {
    let authorization = &request.input_authorization;

    if authorization.state != InputAuthorizationState::Confirmed {
        return Err(RealTranscriptEvaluationRunnerContractError::InputAuthorizationNotConfirmed);
    }

    if authorization.scope_policy_revision != INPUT_AUTHORIZATION_SCOPE_POLICY {
        return Err(
            RealTranscriptEvaluationRunnerContractError::InputAuthorizationValidationFailure(
                InputAuthorizationValidationError::UnsupportedScopePolicy {
                    found: authorization.scope_policy_revision.clone(),
                    expected: INPUT_AUTHORIZATION_SCOPE_POLICY.to_string(),
                },
            ),
        );
    }

    if authorization.run_id != posture.run_id {
        return Err(RealTranscriptEvaluationRunnerContractError::InputAuthorizationRunMismatch);
    }

    if authorization.input_identity != posture.input_identity {
        return Err(
            RealTranscriptEvaluationRunnerContractError::InputAuthorizationIdentityMismatch,
        );
    }

    if authorization.input_class != posture.input_class {
        return Err(RealTranscriptEvaluationRunnerContractError::InputAuthorizationClassMismatch);
    }

    validate_input_class_and_basis(authorization.input_class, authorization.authorization_basis)
        .map_err(RealTranscriptEvaluationRunnerContractError::InputAuthorizationValidationFailure)
}

fn validate_reference_readiness(
    request: &RealTranscriptEvaluationRunRequest,
) -> Result<(), RealTranscriptEvaluationRunnerContractError> {
    let seal = &request.reference_seal;
    let coverage = &request.reference_coverage;
    let human_reference = &request.human_final_reference;

    seal.validate()
        .map_err(RealTranscriptEvaluationRunnerContractError::ReferenceSealValidationFailure)?;

    if seal.seal_state != ReferenceSealState::Sealed {
        return Err(RealTranscriptEvaluationRunnerContractError::ReferenceSealBindingMismatch);
    }

    if seal.producer_class != ReferenceProducerClass::HumanBlindReviewer {
        return Err(RealTranscriptEvaluationRunnerContractError::ReferenceSealNotBlindEligible);
    }

    if seal.calibration_classification != ReferenceCalibrationValidity::BlindReferenceEligible
        || seal.calibration_validity_impact != CalibrationValidityImpact::None
    {
        return Err(RealTranscriptEvaluationRunnerContractError::ReferenceSealNotBlindEligible);
    }

    if !seal.reference_created_before_detector_run
        || seal.prior_detector_run_on_same_input
        || seal.prior_knowledge_of_detector_targets
        || seal.session_terms_visible_during_reference
        || seal.external_notes_encode_detector_targets
    {
        return Err(RealTranscriptEvaluationRunnerContractError::ReferenceSealNotBlindEligible);
    }

    if seal.run_id != request.declared_envelope.run_id
        || seal.input_identity != request.declared_envelope.input_identity
        || seal.seal_id != human_reference.seal_id
        || seal.reference_revision != human_reference.reference_revision
        || seal.reference_revision != coverage.reference_revision
        || coverage.seal_id != seal.seal_id
        || coverage.run_id != seal.run_id
        || human_reference.run_id != seal.run_id
    {
        return Err(RealTranscriptEvaluationRunnerContractError::ReferenceSealBindingMismatch);
    }

    seal.validate_with_envelope(&request.reference_sealed_envelope)
        .map_err(RealTranscriptEvaluationRunnerContractError::ReferenceSealValidationFailure)?;

    for envelope in [
        &request.detector_execution_envelope,
        &request.assisted_review_transition_envelope,
        &request.finalized_envelope,
    ] {
        seal.validate_historical_context(envelope)
            .map_err(RealTranscriptEvaluationRunnerContractError::ReferenceSealValidationFailure)?;
    }

    if coverage.coverage_purpose != ReferenceCoveragePurpose::PrimaryBlindCalibration {
        return Err(RealTranscriptEvaluationRunnerContractError::ReferenceCoverageNotPrimary);
    }

    if coverage.coverage_state != ReferenceCoverageState::Complete
        || !coverage.assessment.inventory_complete
        || !coverage.assessment.reference_resolved
        || !coverage.assessment.coverage_complete
    {
        return Err(RealTranscriptEvaluationRunnerContractError::ReferenceCoverageIncomplete);
    }

    coverage
        .validate()
        .map_err(RealTranscriptEvaluationRunnerContractError::ReferenceCoverageValidationFailure)?;

    if human_reference.state != crate::human_final_reference::HumanFinalReferenceState::Sealed {
        return Err(RealTranscriptEvaluationRunnerContractError::HumanFinalReferenceNotSealed);
    }

    human_reference.validate().map_err(
        RealTranscriptEvaluationRunnerContractError::HumanFinalReferenceValidationFailure,
    )?;

    coverage
        .validate_against(
            &request.reference_sealed_envelope,
            seal,
            Some(human_reference),
        )
        .map_err(RealTranscriptEvaluationRunnerContractError::ReferenceCoverageValidationFailure)?;

    human_reference
        .validate_against(&request.reference_sealed_envelope, seal)
        .map_err(
            RealTranscriptEvaluationRunnerContractError::HumanFinalReferenceValidationFailure,
        )?;

    human_reference
        .validate_against_coverage(coverage)
        .map_err(
            RealTranscriptEvaluationRunnerContractError::HumanFinalReferenceValidationFailure,
        )?;

    for envelope in [
        &request.detector_execution_envelope,
        &request.assisted_review_transition_envelope,
        &request.finalized_envelope,
    ] {
        coverage
            .validate_historical_context(envelope, seal, Some(human_reference))
            .map_err(
                RealTranscriptEvaluationRunnerContractError::ReferenceCoverageValidationFailure,
            )?;

        human_reference
            .validate_historical_context(envelope, seal)
            .map_err(
                RealTranscriptEvaluationRunnerContractError::HumanFinalReferenceValidationFailure,
            )?;
    }

    Ok(())
}

fn validate_detector_analysis_readiness(
    request: &RealTranscriptEvaluationRunRequest,
    posture: &RunEnvelope,
) -> Result<(), RealTranscriptEvaluationRunnerContractError> {
    request
        .detector_analysis_identity
        .validate(&posture.input_identity)
        .map_err(|error| match error {
            DetectorAnalysisIdentityValidationError::InputIdentityMismatch => {
                RealTranscriptEvaluationRunnerContractError::DetectorAnalysisIdentityInputMismatch
            }
            other => {
                RealTranscriptEvaluationRunnerContractError::DetectorAnalysisIdentityValidationFailure(
                    other,
                )
            }
        })
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn canonical_inventory_has_nine_roles_in_order() {
        let roles = canonical_real_evaluation_artifact_roles();
        assert_eq!(roles.len(), 9);
        assert_eq!(roles[0], ArtifactRole::InputAuthorization);
        assert_eq!(roles[8], ArtifactRole::Metrics);
    }

    #[test]
    fn duplicate_inventory_role_detected_before_canonical_mismatch() {
        let mut roles = canonical_real_evaluation_artifact_roles();
        roles.push(ArtifactRole::Metrics);
        assert_eq!(
            validate_artifact_inventory(&roles),
            Err(
                RealTranscriptEvaluationRunnerContractError::DuplicateExpectedArtifactRole {
                    role: ArtifactRole::Metrics,
                }
            )
        );
    }

    #[test]
    fn overlap_authority_policy_roles() {
        assert_eq!(
            real_evaluation_overlap_authority_roles(),
            &[
                OverlapAdjudicatorRole::OwnerAdjudicator,
                OverlapAdjudicatorRole::AuthorizedDomainAdjudicator,
            ]
        );
        assert_eq!(
            real_evaluation_forbidden_overlap_authority_roles(),
            &[OverlapAdjudicatorRole::SyntheticFixtureAdjudicator]
        );
    }
}

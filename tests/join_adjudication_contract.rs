use vox_proof::artifact_bundle::{ArtifactContentDigest, ArtifactId};
use vox_proof::detector_snapshot::DetectorSnapshotRevisionId;
use vox_proof::human_final_reference::ReferenceErrorId;
use vox_proof::join_adjudication::{
    OVERLAP_ADJUDICATION_SCHEMA, OverlapAdjudicationAssessment, OverlapAdjudicationId,
    OverlapAdjudicationRecord, OverlapAdjudicationResult, OverlapAdjudicationSet,
    OverlapAdjudicationSetId, OverlapAdjudicationSetState, OverlapAdjudicationValidationError,
    OverlapAdjudicatorRole,
};
use vox_proof::reference_identity::ReferenceRevisionId;
use vox_proof::run_manifest::{ArtifactRole, InputIdentityReference, RunId, RunLifecycleState};
use vox_proof::{
    detector_snapshot::DetectorProposalId,
    run_manifest::{
        CalibrationValidityMode, InputClass, RUN_ENVELOPE_SCHEMA, RunEnvelope,
        WorkflowObservationMode,
    },
};

const SAMPLE_REVISION: &str =
    "rev:sha256-v1:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const SAMPLE_REFERENCE_REVISION: &str = "ref-rev-001";
const SAMPLE_DIGEST: &str =
    "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const JOIN_CONTRACT_REVISION: &str = "voxproof-detector-reference-join-v1";
const OVERLAP_RULE_REVISION: &str = "voxproof-overlap-v1";

fn input_identity() -> InputIdentityReference {
    InputIdentityReference {
        transcript_revision_id: SAMPLE_REVISION.to_string(),
    }
}

fn join_envelope(lifecycle: RunLifecycleState) -> RunEnvelope {
    RunEnvelope {
        schema_revision: RUN_ENVELOPE_SCHEMA.to_string(),
        run_id: RunId::new("run-join").expect("run id"),
        input_identity: input_identity(),
        calibration_validity: CalibrationValidityMode::BlindReference,
        workflow_observation: WorkflowObservationMode::Disabled,
        input_class: InputClass::SelfOwnedReal,
        qualifies_as_real_material_evidence: false,
        lifecycle_state: lifecycle,
        expected_artifact_roles: vec![ArtifactRole::JoinAdjudication],
    }
}

fn adjudication_record(
    adjudication_id: &str,
    proposal_id: &str,
    reference_error_id: &str,
) -> OverlapAdjudicationRecord {
    OverlapAdjudicationRecord {
        adjudication_id: OverlapAdjudicationId::new(adjudication_id).expect("adjudication id"),
        detector_proposal_id: DetectorProposalId::new(proposal_id).expect("proposal id"),
        reference_error_id: ReferenceErrorId::new(reference_error_id).expect("reference error id"),
        join_contract_revision: JOIN_CONTRACT_REVISION.to_string(),
        adjudicator_role: OverlapAdjudicatorRole::OwnerAdjudicator,
        adjudication_result: OverlapAdjudicationResult::SameErrorSameCorrection,
        adjudication_reason: "synthetic overlap adjudication".to_string(),
        adjudicated_at_unix_ms: 1_700_000_000_000,
    }
}

fn build_adjudication_set(
    records: Vec<OverlapAdjudicationRecord>,
    state: OverlapAdjudicationSetState,
) -> OverlapAdjudicationSet {
    let assessment = OverlapAdjudicationSet::derive_assessment(&records);

    OverlapAdjudicationSet {
        schema_revision: OVERLAP_ADJUDICATION_SCHEMA.to_string(),
        adjudication_set_id: OverlapAdjudicationSetId::new("adj-set-test").expect("set id"),
        run_id: RunId::new("run-join").expect("run id"),
        input_identity: input_identity(),
        reference_revision: ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION)
            .expect("reference revision"),
        detector_snapshot_revision: DetectorSnapshotRevisionId::new("det-snap-rev-001")
            .expect("snapshot revision"),
        join_contract_revision: JOIN_CONTRACT_REVISION.to_string(),
        overlap_rule_revision: OVERLAP_RULE_REVISION.to_string(),
        join_adjudication_artifact_id: ArtifactId::new("artifact-join-adjudication")
            .expect("artifact id"),
        state,
        records,
        assessment,
    }
}

#[test]
fn json_round_trip_retains_schema_and_enum_spellings() {
    let set = build_adjudication_set(
        vec![adjudication_record("adj-001", "det-prop-a", "ref-err-1")],
        OverlapAdjudicationSetState::Draft,
    );

    let json = serde_json::to_string_pretty(&set).expect("serialize");
    assert!(json.contains(OVERLAP_ADJUDICATION_SCHEMA));
    assert!(json.contains("\"same_error_same_correction\""));
    assert!(json.contains("\"owner_adjudicator\""));
    assert!(json.contains("\"draft\""));

    let restored: OverlapAdjudicationSet = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, set);
    restored.validate().expect("valid adjudication set");
}

#[test]
fn unknown_top_level_field_rejected() {
    let set = build_adjudication_set(vec![], OverlapAdjudicationSetState::Draft);
    let mut value = serde_json::to_value(&set).expect("value");
    value.as_object_mut().expect("object").insert(
        "transcript_text".to_string(),
        serde_json::json!("forbidden"),
    );

    let error = serde_json::from_value::<OverlapAdjudicationSet>(value).expect_err("must fail");
    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn frozen_valid_adjudication_set_passes_validation() {
    let set = build_adjudication_set(
        vec![adjudication_record("adj-001", "det-prop-a", "ref-err-1")],
        OverlapAdjudicationSetState::Frozen,
    );

    set.validate().expect("frozen valid set");
    set.validate_against_envelope(&join_envelope(RunLifecycleState::AssistedReview))
        .expect("envelope alignment");
    set.validate_frozen_for_join(
        &join_envelope(RunLifecycleState::AssistedReview),
        &ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION).expect("revision"),
        &DetectorSnapshotRevisionId::new("det-snap-rev-001").expect("snapshot revision"),
    )
    .expect("frozen for join");
}

#[test]
fn draft_adjudication_set_cannot_validate_frozen_for_join() {
    let set = build_adjudication_set(
        vec![adjudication_record("adj-001", "det-prop-a", "ref-err-1")],
        OverlapAdjudicationSetState::Draft,
    );

    assert!(matches!(
        set.validate_frozen_for_join(
            &join_envelope(RunLifecycleState::AssistedReview),
            &ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION).expect("revision"),
            &DetectorSnapshotRevisionId::new("det-snap-rev-001").expect("snapshot revision"),
        ),
        Err(OverlapAdjudicationValidationError::AdjudicationSetNotFrozen)
    ));
}

#[test]
fn duplicate_adjudication_ids_rejected_in_frozen_state() {
    let set = build_adjudication_set(
        vec![
            adjudication_record("adj-dup", "det-prop-a", "ref-err-1"),
            adjudication_record("adj-dup", "det-prop-b", "ref-err-2"),
        ],
        OverlapAdjudicationSetState::Frozen,
    );

    assert!(matches!(
        set.validate(),
        Err(OverlapAdjudicationValidationError::SetStateMismatch { .. })
    ));
    assert_eq!(set.assessment.duplicate_adjudication_ids.len(), 1);
}

#[test]
fn duplicate_pairs_rejected_in_frozen_state() {
    let set = build_adjudication_set(
        vec![
            adjudication_record("adj-001", "det-prop-a", "ref-err-1"),
            adjudication_record("adj-002", "det-prop-a", "ref-err-1"),
        ],
        OverlapAdjudicationSetState::Frozen,
    );

    assert!(matches!(
        set.validate(),
        Err(OverlapAdjudicationValidationError::SetStateMismatch { .. })
    ));
    assert_eq!(set.assessment.duplicate_pairs.len(), 1);
}

#[test]
fn zero_adjudication_timestamp_rejected() {
    let mut record = adjudication_record("adj-001", "det-prop-a", "ref-err-1");
    record.adjudicated_at_unix_ms = 0;
    let set = build_adjudication_set(vec![record], OverlapAdjudicationSetState::Draft);

    assert!(matches!(
        set.validate(),
        Err(OverlapAdjudicationValidationError::ZeroAdjudicationTimestamp)
    ));
}

#[test]
fn empty_adjudication_reason_rejected() {
    let mut record = adjudication_record("adj-001", "det-prop-a", "ref-err-1");
    record.adjudication_reason.clear();
    let set = build_adjudication_set(vec![record], OverlapAdjudicationSetState::Draft);

    assert!(matches!(
        set.validate(),
        Err(OverlapAdjudicationValidationError::EmptyAdjudicationReason)
    ));
}

#[test]
fn join_adjudication_role_json_spelling() {
    let json = serde_json::to_string(&ArtifactRole::JoinAdjudication).expect("serialize");
    assert_eq!(json, "\"join_adjudication\"");
}

#[test]
fn caller_cannot_force_stored_assessment_inconsistent_with_derivation() {
    let mut set = build_adjudication_set(
        vec![adjudication_record("adj-001", "det-prop-a", "ref-err-1")],
        OverlapAdjudicationSetState::Draft,
    );
    set.assessment = OverlapAdjudicationAssessment {
        record_count: 0,
        duplicate_adjudication_ids: vec![],
        duplicate_pairs: vec![],
        context_consistent: true,
    };

    assert!(matches!(
        set.validate(),
        Err(OverlapAdjudicationValidationError::AssessmentMismatch { .. })
    ));
}

#[test]
fn unsupported_schema_revision_rejected() {
    let mut set = build_adjudication_set(vec![], OverlapAdjudicationSetState::Draft);
    set.schema_revision = "voxproof-overlap-adjudication-v0".to_string();

    assert!(matches!(
        set.validate(),
        Err(OverlapAdjudicationValidationError::UnsupportedSchemaRevision { .. })
    ));
}

#[test]
fn path_like_adjudication_ids_rejected() {
    for value in [
        "/Users/example/private/adj.json",
        "../private/adj.json",
        "C:\\Users\\example\\private\\adj.json",
    ] {
        assert!(OverlapAdjudicationSetId::new(value).is_err());
        assert!(OverlapAdjudicationId::new(value).is_err());
    }
}

#[test]
fn serialized_adjudication_set_contains_no_forbidden_fields() {
    let set = build_adjudication_set(
        vec![adjudication_record("adj-001", "det-prop-a", "ref-err-1")],
        OverlapAdjudicationSetState::Frozen,
    );
    let json = serde_json::to_string(&set).expect("serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");

    for forbidden in [
        "transcript_text",
        "path",
        "precision",
        "recall",
        "true_positive",
        "false_positive",
        "false_negative",
    ] {
        assert!(
            value.get(forbidden).is_none(),
            "serialized adjudication set must not contain {forbidden:?}"
        );
    }
}

#[test]
fn join_adjudication_artifact_id_round_trips() {
    let artifact_id = ArtifactId::new("artifact-join-adjudication").expect("artifact id");
    let digest = ArtifactContentDigest::new(SAMPLE_DIGEST).expect("digest");
    assert_eq!(artifact_id.as_str(), "artifact-join-adjudication");
    assert_eq!(digest.as_str(), SAMPLE_DIGEST);
}

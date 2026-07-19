use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use vox_proof::persistence_evidence::{
    ActiveAnalysisSelection, AdapterError, AuthoritativeCommand, CandidateCapabilities,
    CanonicalEventProvenance, DuplicatedSession, EvidenceAggregationIssueCode, EvidenceFixture,
    EvidenceHarness, EvidenceManifest, EvidenceRunEligibility, EvidenceSessionHandle,
    EvidenceSessionRef, FixtureScale, HARNESS_VERSION, KnownOrUnavailable, MaintenanceOperation,
    NormalizedSemanticState, ORACLE_VERSION, OptionalCapability, OptionalOperationOutcome,
    OracleViolationCode, PersistenceCandidateAdapter, REQUIRED_SCENARIO_IDS,
    RecoveryClassification, RetentionRootClass, ReviewCaseOrigin, ReviewLedgerAction,
    SCENARIO_CATALOG_VERSION, SMALL_FIXTURE_ID, SMALL_FIXTURE_VERSION, ScenarioEvidenceKind,
    ScenarioIdentity, ScenarioRequirement, ScenarioResult, ScenarioStatus, SemanticOpenMode,
    SemanticOracle, SemanticPrecondition, scenario_catalog,
};

fn manifest() -> EvidenceManifest {
    EvidenceManifest {
        evidence_protocol_version: "md-015-proposed-v1".to_string(),
        repository_commit: "test-commit".to_string(),
        candidate_id: "test-fake-not-a-candidate".to_string(),
        candidate_version: "1".to_string(),
        fixture_id: SMALL_FIXTURE_ID.to_string(),
        fixture_version: SMALL_FIXTURE_VERSION.to_string(),
        harness_version: HARNESS_VERSION.to_string(),
        oracle_version: ORACLE_VERSION.to_string(),
        scenario_ids: Vec::new(),
        operating_system: KnownOrUnavailable::Unavailable {
            reason: "unit test".to_string(),
        },
        operating_system_version: KnownOrUnavailable::Unavailable {
            reason: "unit test".to_string(),
        },
        filesystem: KnownOrUnavailable::Unavailable {
            reason: "unit test".to_string(),
        },
        hardware_summary: KnownOrUnavailable::Unavailable {
            reason: "unit test".to_string(),
        },
        runtime_versions: BTreeMap::new(),
        configuration: BTreeMap::new(),
        start_timestamp: KnownOrUnavailable::Unavailable {
            reason: "unit test does not use wall clock".to_string(),
        },
        end_timestamp: KnownOrUnavailable::Unavailable {
            reason: "unit test does not use wall clock".to_string(),
        },
        known_limitations: vec!["test fake has no durability semantics".to_string()],
    }
}

fn has_violation(
    result: &vox_proof::persistence_evidence::OracleResult,
    code: OracleViolationCode,
) -> bool {
    result
        .violations
        .iter()
        .any(|violation| violation.code == code)
}

fn has_aggregation_issue(
    result: &vox_proof::persistence_evidence::EvidenceRunResult,
    code: EvidenceAggregationIssueCode,
) -> bool {
    result
        .aggregation_issues
        .iter()
        .any(|issue| issue.code == code)
}

fn passing_oracle_result() -> vox_proof::persistence_evidence::OracleResult {
    let state = EvidenceFixture::small().normalized_state();
    SemanticOracle::compare(&state, &state)
}

fn scenario_result(
    scenario_identity: ScenarioIdentity,
    status: ScenarioStatus,
    oracle_result: Option<vox_proof::persistence_evidence::OracleResult>,
    limitations: Vec<String>,
) -> ScenarioResult {
    ScenarioResult {
        scenario_identity,
        status,
        oracle_result,
        measurements: BTreeMap::new(),
        failure_classification: None,
        limitations,
        raw_artifact_references: Vec::new(),
    }
}

fn complete_comparable_results() -> Vec<ScenarioResult> {
    scenario_catalog()
        .into_iter()
        .map(|scenario| {
            if scenario.requirement == ScenarioRequirement::CapabilityDependent {
                scenario_result(
                    scenario,
                    ScenarioStatus::Unsupported,
                    None,
                    vec!["test fake does not support optional maintenance".to_string()],
                )
            } else {
                scenario_result(
                    scenario,
                    ScenarioStatus::Passed,
                    Some(passing_oracle_result()),
                    Vec::new(),
                )
            }
        })
        .collect()
}

#[test]
fn small_fixture_is_deterministic_and_explicitly_versioned() {
    let first = EvidenceFixture::small();
    let second = EvidenceFixture::small();

    assert_eq!(first, second);
    assert_eq!(first.fixture_version, SMALL_FIXTURE_VERSION);
    assert_eq!(first.scale, FixtureScale::Small);
    assert!(FixtureScale::Small.is_implemented());
    assert!(!FixtureScale::Medium.is_implemented());
    assert!(!FixtureScale::Stress.is_implemented());
    assert_eq!(
        serde_json::to_vec(&first).expect("test/evidence representation serializes"),
        serde_json::to_vec(&second).expect("test/evidence representation serializes")
    );
}

#[test]
fn small_fixture_contains_every_required_semantic_category() {
    let state = EvidenceFixture::small().normalized_state();

    assert_eq!(state.source_revisions.len(), 2);
    assert!(
        state
            .source_revisions
            .iter()
            .any(|revision| revision.predecessor_revision_id.is_some())
    );
    assert!(
        state
            .review_cases
            .iter()
            .any(|case| matches!(case.origin, ReviewCaseOrigin::DetectorRaised { .. }))
    );
    assert!(
        state
            .review_cases
            .iter()
            .any(|case| matches!(case.origin, ReviewCaseOrigin::HumanRaised { .. }))
    );
    assert!(state.review_ledger_events.iter().any(|event| {
        matches!(
            event.action,
            ReviewLedgerAction::AcceptAlternative {
                alternative_index: 0
            }
        )
    }));
    assert_eq!(state.review_case_raised_events.len(), 1);
    assert!(state.review_ledger_events.iter().any(|event| matches!(
        &event.action,
        ReviewLedgerAction::ManualReplacement { replacement_text }
            if replacement_text == "PostgreSQL — production"
    )));
    assert!(state.review_ledger_events.iter().any(|event| matches!(
        event.action,
        ReviewLedgerAction::Withdraw { .. } | ReviewLedgerAction::Supersede { .. }
    )));
    assert_eq!(state.analysis_results.len(), 2);
    assert!(state.active_analysis_selection.is_some());
    assert_eq!(state.knowledge_snapshot_references.len(), 1);
    assert_eq!(state.lineage_conflicts.len(), 1);
    assert!(
        state
            .artifacts
            .iter()
            .any(|artifact| { artifact.artifact_id == "artifact:historical:referenced" })
    );
    assert!(
        state
            .artifacts
            .iter()
            .any(|artifact| { artifact.artifact_id == "artifact:historical:unreferenced" })
    );
    assert!(
        state
            .artifacts
            .iter()
            .any(|artifact| { artifact.artifact_id == "artifact:derived:index" })
    );
    assert!(
        state
            .artifacts
            .iter()
            .any(|artifact| { artifact.artifact_id == "artifact:temporary:cancelled-job" })
    );
}

#[test]
fn small_fixture_has_no_host_or_current_time_inputs_and_validates() {
    let fixture = EvidenceFixture::small();
    let encoded = serde_json::to_string(&fixture).expect("test/evidence representation serializes");

    assert!(!encoded.contains("/Users/"));
    assert!(!encoded.contains("\\Users\\"));
    if let Some(user) = option_env!("USER")
        && !user.is_empty()
    {
        assert!(!encoded.contains(user));
    }
    assert!(SemanticOracle::validate(&fixture.normalized_state()).passed);
}

#[test]
fn normalization_is_stable_for_logically_unordered_collections() {
    let expected = EvidenceFixture::small().normalized_state();
    let mut physically_reordered = expected.clone();
    physically_reordered.source_revisions.reverse();
    physically_reordered.review_cases.reverse();
    physically_reordered.analysis_results.reverse();
    physically_reordered.artifacts.reverse();
    physically_reordered.knowledge_snapshot_references.reverse();

    assert_eq!(expected, physically_reordered.clone().normalize());
    assert!(SemanticOracle::compare(&expected, &physically_reordered).passed);
}

#[test]
fn oracle_passes_identical_states() {
    let state = EvidenceFixture::small().normalized_state();
    let result = SemanticOracle::compare(&state, &state);

    assert!(result.passed);
    assert!(result.violations.is_empty());
    assert_eq!(
        result.expected_fingerprint.as_ref(),
        Some(&result.actual_fingerprint)
    );
}

#[test]
fn oracle_ignores_derived_and_temporary_artifact_differences() {
    let expected = EvidenceFixture::small().normalized_state();
    let mut actual = expected.clone();
    actual
        .artifacts
        .retain(|artifact| !artifact.artifact_id.contains("derived:index"));
    actual
        .artifacts
        .retain(|artifact| !artifact.artifact_id.contains("temporary:"));

    let result = SemanticOracle::compare(&expected, &actual);
    assert!(result.passed);
    assert_eq!(
        result.expected_fingerprint.as_ref(),
        Some(&result.actual_fingerprint)
    );
}

#[test]
fn oracle_ignores_optional_capability_metadata_outside_semantic_state() {
    let fixture = EvidenceFixture::small();
    let mut without_optional = TestFakeAdapter::new(BTreeSet::new());
    let mut with_compaction =
        TestFakeAdapter::new(BTreeSet::from([OptionalCapability::Compaction]));
    let first_session = without_optional.create(&fixture).expect("fake create");
    let second_session = with_compaction.create(&fixture).expect("fake create");
    let first_handle = without_optional
        .open(&first_session, SemanticOpenMode::ReadOnly)
        .expect("fake open");
    let second_handle = with_compaction
        .open(&second_session, SemanticOpenMode::ReadOnly)
        .expect("fake open");

    let result = SemanticOracle::compare(
        &without_optional
            .read_normalized_state(&first_handle)
            .expect("fake state"),
        &with_compaction
            .read_normalized_state(&second_handle)
            .expect("fake state"),
    );

    assert!(result.passed);
}

#[test]
fn oracle_detects_missing_ledger_event() {
    let expected = EvidenceFixture::small().normalized_state();
    let mut actual = expected.clone();
    actual.review_ledger_events.remove(1);

    let result = SemanticOracle::compare(&expected, &actual);
    assert!(has_violation(
        &result,
        OracleViolationCode::MissingCommittedEvent
    ));
}

#[test]
fn oracle_detects_reordered_canonical_events() {
    let expected = EvidenceFixture::small().normalized_state();
    let mut actual = expected.clone();
    actual.review_ledger_events.swap(0, 1);

    let result = SemanticOracle::compare(&expected, &actual);
    assert!(has_violation(
        &result,
        OracleViolationCode::ChangedEventOrder
    ));
}

#[test]
fn oracle_detects_changed_ledger_sequence_and_cannot_pass_changed_fingerprint() {
    let expected = EvidenceFixture::small().normalized_state();
    let mut actual = expected.clone();
    actual
        .review_ledger_events
        .last_mut()
        .expect("fixture ledger event")
        .sequence += 1;

    let result = SemanticOracle::compare(&expected, &actual);

    assert!(!result.passed);
    assert_ne!(
        result.expected_fingerprint.as_ref(),
        Some(&result.actual_fingerprint)
    );
    assert!(has_violation(
        &result,
        OracleViolationCode::LedgerSequenceChanged
    ));
}

#[test]
fn oracle_detects_changed_ledger_provenance() {
    let expected = EvidenceFixture::small().normalized_state();
    let mut actual = expected.clone();
    actual.review_ledger_events[0].provenance = CanonicalEventProvenance::Recovery;

    let result = SemanticOracle::compare(&expected, &actual);

    assert!(has_violation(
        &result,
        OracleViolationCode::LedgerProvenanceChanged
    ));
}

#[test]
fn oracle_detects_changed_ledger_case_and_source_binding() {
    let expected = EvidenceFixture::small().normalized_state();
    let mut actual = expected.clone();
    let human_case = actual
        .review_cases
        .iter()
        .find(|case| matches!(case.origin, ReviewCaseOrigin::HumanRaised { .. }))
        .expect("fixture human case");
    actual.review_ledger_events[0].case_id = human_case.case_id.clone();
    actual.review_ledger_events[0].observed_revision_id = human_case.observed_revision_id.clone();

    let result = SemanticOracle::compare(&expected, &actual);

    assert!(has_violation(
        &result,
        OracleViolationCode::LedgerBindingChanged
    ));
}

#[test]
fn oracle_detects_changed_manual_replacement_text_without_truncation() {
    let expected = EvidenceFixture::small().normalized_state();
    let mut actual = expected.clone();
    let event = actual
        .review_ledger_events
        .iter_mut()
        .find(|event| matches!(event.action, ReviewLedgerAction::ManualReplacement { .. }))
        .expect("fixture manual replacement");
    event.action = ReviewLedgerAction::ManualReplacement {
        replacement_text: "PostgreSQL".to_string(),
    };

    let result = SemanticOracle::compare(&expected, &actual);
    assert!(has_violation(
        &result,
        OracleViolationCode::ChangedLedgerActionPayload
    ));
}

#[test]
fn oracle_detects_changed_transcript_revision_identity() {
    let expected = EvidenceFixture::small().normalized_state();
    let mut actual = expected.clone();
    actual.source_revisions[0].revision_id = "rev:sha256-v1:changed".to_string();

    let result = SemanticOracle::compare(&expected, &actual);
    assert!(has_violation(
        &result,
        OracleViolationCode::ChangedTranscriptRevisionId
    ));
}

#[test]
fn oracle_detects_missing_referenced_analysis_result() {
    let expected = EvidenceFixture::small().normalized_state();
    let mut actual = expected.clone();
    let active = actual
        .active_analysis_selection
        .as_ref()
        .expect("fixture active analysis")
        .analysis_result_id
        .clone();
    actual
        .analysis_results
        .retain(|analysis| analysis.analysis_result_id != active);

    let result = SemanticOracle::compare(&expected, &actual);
    assert!(has_violation(
        &result,
        OracleViolationCode::MissingReferencedAnalysisResult
    ));
}

#[test]
fn oracle_detects_changed_active_analysis_selection() {
    let expected = EvidenceFixture::small().normalized_state();
    let mut actual = expected.clone();
    actual
        .active_analysis_selection
        .as_mut()
        .expect("fixture active selection")
        .analysis_result_id = actual.analysis_results[0].analysis_result_id.clone();

    let result = SemanticOracle::compare(&expected, &actual);
    assert!(has_violation(
        &result,
        OracleViolationCode::ChangedActiveAnalysisSelection
    ));
}

#[test]
fn oracle_detects_detector_and_human_case_conflation() {
    let expected = EvidenceFixture::small().normalized_state();
    let mut actual = expected.clone();
    let detector_analysis = actual.analysis_results[0].analysis_result_id.clone();
    let human_case = actual
        .review_cases
        .iter_mut()
        .find(|case| matches!(case.origin, ReviewCaseOrigin::HumanRaised { .. }))
        .expect("fixture human case");
    human_case.origin = ReviewCaseOrigin::DetectorRaised {
        analysis_result_id: detector_analysis,
    };

    let result = SemanticOracle::compare(&expected, &actual);
    assert!(has_violation(
        &result,
        OracleViolationCode::ConflatedReviewCaseOrigin
    ));
}

#[test]
fn human_raised_case_creation_event_is_valid_and_distinct_from_decisions() {
    let state = EvidenceFixture::small().normalized_state();
    let human_case = state
        .review_cases
        .iter()
        .find(|case| matches!(case.origin, ReviewCaseOrigin::HumanRaised { .. }))
        .expect("fixture human case");
    let creation_event = state
        .review_case_raised_events
        .iter()
        .find(|event| event.case_id == human_case.case_id)
        .expect("fixture HumanRaised creation event");

    assert_eq!(creation_event.anchor, human_case.anchor);
    assert!(
        state
            .review_ledger_events
            .iter()
            .all(|event| event.event_id != creation_event.event_id)
    );
    assert!(SemanticOracle::validate(&state).passed);
}

#[test]
fn oracle_detects_missing_human_raised_creation_event() {
    let expected = EvidenceFixture::small().normalized_state();
    let mut actual = expected.clone();
    actual.review_case_raised_events.clear();

    let result = SemanticOracle::compare(&expected, &actual);

    assert!(has_violation(
        &result,
        OracleViolationCode::MissingReviewCaseRaisedEvent
    ));
}

#[test]
fn oracle_detects_human_raised_creation_event_with_wrong_case() {
    let state = EvidenceFixture::small().normalized_state();
    let mut actual = state.clone();
    actual.review_case_raised_events[0].case_id = "review-case:missing".to_string();

    let result = SemanticOracle::validate(&actual);

    assert!(has_violation(
        &result,
        OracleViolationCode::MismatchedReviewCaseRaisedEvent
    ));
}

#[test]
fn oracle_detects_human_raised_case_with_wrong_origin() {
    let state = EvidenceFixture::small().normalized_state();
    let mut actual = state.clone();
    let analysis_result_id = actual.analysis_results[0].analysis_result_id.clone();
    let human_case = actual
        .review_cases
        .iter_mut()
        .find(|case| matches!(case.origin, ReviewCaseOrigin::HumanRaised { .. }))
        .expect("fixture human case");
    human_case.origin = ReviewCaseOrigin::DetectorRaised { analysis_result_id };

    let result = SemanticOracle::validate(&actual);

    assert!(has_violation(
        &result,
        OracleViolationCode::DetectorCaseHasHumanRaisedCreationEvent
    ));
}

#[test]
fn oracle_detects_human_raised_creation_event_with_wrong_source_or_anchor() {
    let state = EvidenceFixture::small().normalized_state();
    let mut actual = state.clone();
    let other_revision = actual
        .source_revisions
        .iter()
        .find(|revision| {
            revision.revision_id != actual.review_case_raised_events[0].observed_revision_id
        })
        .expect("fixture other revision")
        .revision_id
        .clone();
    actual.review_case_raised_events[0].observed_revision_id = other_revision.clone();
    actual.review_case_raised_events[0]
        .anchor
        .source_revision_id = other_revision;

    let result = SemanticOracle::validate(&actual);

    assert!(has_violation(
        &result,
        OracleViolationCode::MismatchedReviewCaseRaisedEvent
    ));
}

#[test]
fn oracle_detects_duplicate_human_raised_creation_event_identity() {
    let state = EvidenceFixture::small().normalized_state();
    let mut actual = state.clone();
    actual
        .review_case_raised_events
        .push(actual.review_case_raised_events[0].clone());

    let result = SemanticOracle::validate(&actual);

    assert!(has_violation(
        &result,
        OracleViolationCode::DuplicateCanonicalIdentity
    ));
}

#[test]
fn oracle_detects_non_human_creation_provenance() {
    let state = EvidenceFixture::small().normalized_state();
    let mut actual = state.clone();
    actual.review_case_raised_events[0].provenance = CanonicalEventProvenance::Recovery;

    let result = SemanticOracle::validate(&actual);

    assert!(has_violation(
        &result,
        OracleViolationCode::MismatchedReviewCaseRaisedEvent
    ));
}

#[test]
fn oracle_detects_detector_case_with_fabricated_human_creation_event() {
    let state = EvidenceFixture::small().normalized_state();
    let mut actual = state.clone();
    let detector_case = actual
        .review_cases
        .iter()
        .find(|case| matches!(case.origin, ReviewCaseOrigin::DetectorRaised { .. }))
        .expect("fixture detector case");
    actual.review_case_raised_events[0].case_id = detector_case.case_id.clone();

    let result = SemanticOracle::validate(&actual);

    assert!(has_violation(
        &result,
        OracleViolationCode::DetectorCaseHasHumanRaisedCreationEvent
    ));
}

#[test]
fn oracle_detects_automatic_decision_copying() {
    let state = EvidenceFixture::small().normalized_state();
    let mut actual = state.clone();
    actual.review_cases[1].copied_decision_from_case_id =
        Some(actual.review_cases[0].case_id.clone());

    let result = SemanticOracle::compare(&state, &actual);
    assert!(has_violation(
        &result,
        OracleViolationCode::AutomaticDecisionMigration
    ));
}

#[test]
fn oracle_detects_fabricated_recovery_event() {
    let expected = EvidenceFixture::small().normalized_state();
    let mut actual = expected.clone();
    let mut fabricated = actual.review_ledger_events[0].clone();
    fabricated.event_id = "ledger-event:fabricated-recovery".to_string();
    fabricated.sequence = 4;
    fabricated.provenance = CanonicalEventProvenance::Recovery;
    actual.review_ledger_events.push(fabricated);

    let result = SemanticOracle::compare(&expected, &actual);
    assert!(has_violation(
        &result,
        OracleViolationCode::FabricatedRecoveryHistory
    ));
    assert!(has_violation(
        &result,
        OracleViolationCode::UnexpectedCommittedEvent
    ));
}

#[test]
fn oracle_detects_duplicate_canonical_identity() {
    let state = EvidenceFixture::small().normalized_state();
    let mut actual = state.clone();
    actual
        .source_revisions
        .push(actual.source_revisions[0].clone());

    let result = SemanticOracle::compare(&state, &actual);
    assert!(has_violation(
        &result,
        OracleViolationCode::DuplicateCanonicalIdentity
    ));
}

#[test]
fn oracle_detects_broken_source_revision_reference() {
    let state = EvidenceFixture::small().normalized_state();
    let mut actual = state.clone();
    actual.review_cases[0].observed_revision_id = "rev:missing".to_string();
    actual.review_cases[0].anchor.source_revision_id = "rev:missing".to_string();

    let result = SemanticOracle::compare(&state, &actual);
    assert!(has_violation(
        &result,
        OracleViolationCode::BrokenCanonicalReference
    ));
}

#[test]
fn oracle_detects_lost_referenced_historical_artifact() {
    let state = EvidenceFixture::small().normalized_state();
    let mut actual = state.clone();
    actual
        .artifacts
        .retain(|artifact| artifact.artifact_id != "artifact:historical:referenced");

    let result = SemanticOracle::compare(&state, &actual);
    assert!(has_violation(
        &result,
        OracleViolationCode::LostReferencedHistoricalArtifact
    ));
}

#[test]
fn oracle_accepts_valid_canonical_retention_reachability() {
    let state = EvidenceFixture::small().normalized_state();

    assert!(SemanticOracle::validate(&state).passed);
}

#[test]
fn oracle_detects_missing_retention_root() {
    let state = EvidenceFixture::small().normalized_state();
    let mut actual = state.clone();
    actual.retention_references[0].root_id = "ledger-event:missing".to_string();

    let result = SemanticOracle::validate(&actual);

    assert!(has_violation(
        &result,
        OracleViolationCode::BrokenRetentionRoot
    ));
}

#[test]
fn oracle_detects_missing_retention_target() {
    let state = EvidenceFixture::small().normalized_state();
    let mut actual = state.clone();
    actual.retention_references[0].artifact_id = "artifact:missing".to_string();

    let result = SemanticOracle::validate(&actual);

    assert!(has_violation(
        &result,
        OracleViolationCode::BrokenRetentionTarget
    ));
}

#[test]
fn oracle_detects_invalid_retention_root_class() {
    let state = EvidenceFixture::small().normalized_state();
    let mut actual = state.clone();
    actual.retention_references[0].root_class = RetentionRootClass::AnalysisResult;

    let result = SemanticOracle::validate(&actual);

    assert!(has_violation(
        &result,
        OracleViolationCode::InvalidRetentionRootClass
    ));
}

#[test]
fn oracle_detects_duplicate_retention_reference() {
    let state = EvidenceFixture::small().normalized_state();
    let mut actual = state.clone();
    actual
        .retention_references
        .push(actual.retention_references[0].clone());

    let result = SemanticOracle::validate(&actual);

    assert!(has_violation(
        &result,
        OracleViolationCode::DuplicateRetentionReference
    ));
}

#[test]
fn derived_rebuild_cannot_hide_changed_canonical_truth() {
    let state = EvidenceFixture::small().normalized_state();
    let mut actual = state.clone();
    actual
        .artifacts
        .retain(|artifact| artifact.artifact_id != "artifact:derived:index");
    actual.review_ledger_events[0].action = ReviewLedgerAction::ManualReplacement {
        replacement_text: "fabricated by derived rebuild".to_string(),
    };

    let result = SemanticOracle::compare(&state, &actual);
    assert!(has_violation(
        &result,
        OracleViolationCode::ChangedLedgerActionPayload
    ));
}

#[test]
fn baseline_runner_captures_normalized_state_and_oracle_result() {
    let fixture = EvidenceFixture::small();
    let mut fake = TestFakeAdapter::new(BTreeSet::new());

    let result = EvidenceHarness::run_baseline(&mut fake, &fixture, manifest());

    assert_eq!(result.scenario_results.len(), 1);
    assert_eq!(result.scenario_results[0].status, ScenarioStatus::Passed);
    assert!(
        result.scenario_results[0]
            .oracle_result
            .as_ref()
            .expect("baseline oracle")
            .passed
    );
    assert_eq!(result.eligibility, EvidenceRunEligibility::Inconclusive);
}

#[test]
fn unsupported_optional_capability_is_a_limitation_not_a_pass() {
    let run = EvidenceHarness::aggregate(manifest(), complete_comparable_results());

    assert_eq!(run.summary.unsupported, 2);
    assert_eq!(run.negative_results.len(), 2);
    assert!(run.aggregation_issues.is_empty());
    assert_eq!(
        run.eligibility,
        EvidenceRunEligibility::EligibleForComparison
    );
}

#[test]
fn failed_required_scenario_is_not_eligible_and_is_retained() {
    let mut results = complete_comparable_results();
    let failed = results
        .iter_mut()
        .find(|result| result.scenario_identity.requirement == ScenarioRequirement::Required)
        .expect("required scenario");
    failed.status = ScenarioStatus::Failed;
    failed.oracle_result = None;
    failed.failure_classification = Some(RecoveryClassification::Unrecoverable);
    failed.limitations = vec!["negative evidence retained".to_string()];

    let run = EvidenceHarness::aggregate(manifest(), results);

    assert_eq!(run.eligibility, EvidenceRunEligibility::NotEligible);
    assert_eq!(run.negative_results.len(), 3);
}

#[test]
fn catalog_has_unique_stable_complete_identities() {
    let catalog = scenario_catalog();
    let identities: BTreeSet<_> = catalog
        .iter()
        .map(|scenario| (&scenario.scenario_id, scenario.scenario_version))
        .collect();
    let ids: BTreeSet<_> = catalog
        .iter()
        .map(|scenario| scenario.scenario_id.as_str())
        .collect();

    assert_eq!(catalog.len(), 14);
    assert_eq!(identities.len(), catalog.len());
    assert_eq!(ids.len(), catalog.len());
    assert_eq!(SCENARIO_CATALOG_VERSION, "1");
    assert!(catalog.iter().all(|scenario| scenario.scenario_version > 0));
    assert!(REQUIRED_SCENARIO_IDS.iter().all(|required| {
        catalog.iter().any(|scenario| {
            scenario.scenario_id == *required
                && scenario.requirement == ScenarioRequirement::Required
        })
    }));
    assert!(catalog.iter().all(|scenario| {
        scenario.evidence_kind == ScenarioEvidenceKind::SemanticCorrectness
            && (scenario.requirement != ScenarioRequirement::CapabilityDependent
                || !scenario.required_capabilities.is_empty())
    }));
}

#[test]
fn aggregation_rejects_unknown_scenario() {
    let mut results = complete_comparable_results();
    let mut unknown = results[0].clone();
    unknown.scenario_identity.scenario_id = "unknown-scenario".to_string();
    results.push(unknown);

    let run = EvidenceHarness::aggregate(manifest(), results);

    assert!(has_aggregation_issue(
        &run,
        EvidenceAggregationIssueCode::UnknownScenario
    ));
    assert_eq!(run.eligibility, EvidenceRunEligibility::NotEligible);
}

#[test]
fn aggregation_rejects_scenario_version_mismatch() {
    let mut results = complete_comparable_results();
    results[0].scenario_identity.scenario_version += 1;

    let run = EvidenceHarness::aggregate(manifest(), results);

    assert!(has_aggregation_issue(
        &run,
        EvidenceAggregationIssueCode::ScenarioVersionMismatch
    ));
    assert_eq!(run.eligibility, EvidenceRunEligibility::NotEligible);
}

#[test]
fn aggregation_rejects_scenario_definition_mismatch() {
    let mut results = complete_comparable_results();
    results[0].scenario_identity.description = "changed workload claim".to_string();

    let run = EvidenceHarness::aggregate(manifest(), results);

    assert!(has_aggregation_issue(
        &run,
        EvidenceAggregationIssueCode::ScenarioDefinitionMismatch
    ));
    assert_eq!(run.eligibility, EvidenceRunEligibility::NotEligible);
}

#[test]
fn aggregation_rejects_passed_without_oracle() {
    let mut results = complete_comparable_results();
    let required = results
        .iter_mut()
        .find(|result| result.status == ScenarioStatus::Passed)
        .expect("passed required scenario");
    required.oracle_result = None;

    let run = EvidenceHarness::aggregate(manifest(), results);

    assert!(has_aggregation_issue(
        &run,
        EvidenceAggregationIssueCode::PassedWithoutOracle
    ));
    assert_eq!(run.eligibility, EvidenceRunEligibility::NotEligible);
}

#[test]
fn aggregation_rejects_passed_with_failing_oracle() {
    let mut results = complete_comparable_results();
    let expected = EvidenceFixture::small().normalized_state();
    let mut actual = expected.clone();
    actual.review_ledger_events.pop();
    let failing_oracle = SemanticOracle::compare(&expected, &actual);
    let required = results
        .iter_mut()
        .find(|result| result.status == ScenarioStatus::Passed)
        .expect("passed required scenario");
    required.oracle_result = Some(failing_oracle);

    let run = EvidenceHarness::aggregate(manifest(), results);

    assert!(has_aggregation_issue(
        &run,
        EvidenceAggregationIssueCode::PassedWithFailingOracle
    ));
    assert_eq!(run.eligibility, EvidenceRunEligibility::NotEligible);
}

#[test]
fn aggregation_rejects_required_unsupported() {
    let mut results = complete_comparable_results();
    let required = results
        .iter_mut()
        .find(|result| result.status == ScenarioStatus::Passed)
        .expect("required scenario");
    required.status = ScenarioStatus::Unsupported;
    required.oracle_result = None;
    required.limitations = vec!["required behavior unavailable".to_string()];

    let run = EvidenceHarness::aggregate(manifest(), results);

    assert_eq!(run.eligibility, EvidenceRunEligibility::NotEligible);
}

#[test]
fn aggregation_rejects_unsupported_without_limitation() {
    let mut results = complete_comparable_results();
    let optional = results
        .iter_mut()
        .find(|result| result.status == ScenarioStatus::Unsupported)
        .expect("optional scenario");
    optional.limitations = vec![" ".to_string()];

    let run = EvidenceHarness::aggregate(manifest(), results);

    assert!(has_aggregation_issue(
        &run,
        EvidenceAggregationIssueCode::UnsupportedWithoutLimitation
    ));
    assert_eq!(run.eligibility, EvidenceRunEligibility::NotEligible);
}

#[test]
fn aggregation_keeps_not_run_inconclusive() {
    let mut results = complete_comparable_results();
    let required = results
        .iter_mut()
        .find(|result| result.status == ScenarioStatus::Passed)
        .expect("required scenario");
    required.status = ScenarioStatus::NotRun;
    required.oracle_result = None;

    let run = EvidenceHarness::aggregate(manifest(), results);

    assert_eq!(run.eligibility, EvidenceRunEligibility::Inconclusive);
}

#[test]
fn aggregation_keeps_inconclusive_inconclusive() {
    let mut results = complete_comparable_results();
    let required = results
        .iter_mut()
        .find(|result| result.status == ScenarioStatus::Passed)
        .expect("required scenario");
    required.status = ScenarioStatus::Inconclusive;
    required.oracle_result = None;

    let run = EvidenceHarness::aggregate(manifest(), results);

    assert_eq!(run.eligibility, EvidenceRunEligibility::Inconclusive);
}

#[test]
fn aggregation_rejects_duplicate_scenario_identity() {
    let mut results = complete_comparable_results();
    results.push(results[0].clone());

    let run = EvidenceHarness::aggregate(manifest(), results);

    assert!(has_aggregation_issue(
        &run,
        EvidenceAggregationIssueCode::DuplicateScenarioIdentity
    ));
    assert_eq!(run.eligibility, EvidenceRunEligibility::NotEligible);
}

#[test]
fn aggregation_marks_missing_required_scenario_inconclusive() {
    let mut results = complete_comparable_results();
    let missing_index = results
        .iter()
        .position(|result| result.status == ScenarioStatus::Passed)
        .expect("required scenario");
    results.remove(missing_index);

    let run = EvidenceHarness::aggregate(manifest(), results);

    assert!(has_aggregation_issue(
        &run,
        EvidenceAggregationIssueCode::MissingRequiredScenario
    ));
    assert_eq!(run.eligibility, EvidenceRunEligibility::Inconclusive);
}

/// Interface-only fake. It is not an MD-015 persistence candidate and makes
/// no durability, recovery, or performance claim.
#[derive(Default)]
struct FakeStore {
    sessions: BTreeMap<String, FakeSession>,
    next_handle: u64,
}

struct FakeSession {
    state: NormalizedSemanticState,
    handles: BTreeMap<String, SemanticOpenMode>,
    writer_handle: Option<String>,
}

#[derive(Clone)]
struct TestFakeAdapter {
    store: Rc<RefCell<FakeStore>>,
    optional_capabilities: BTreeSet<OptionalCapability>,
}

impl TestFakeAdapter {
    fn new(optional_capabilities: BTreeSet<OptionalCapability>) -> Self {
        Self {
            store: Rc::new(RefCell::new(FakeStore::default())),
            optional_capabilities,
        }
    }

    fn paired(optional_capabilities: BTreeSet<OptionalCapability>) -> (Self, Self) {
        let first = Self::new(optional_capabilities);
        let second = first.clone();
        (first, second)
    }
}

impl PersistenceCandidateAdapter for TestFakeAdapter {
    fn candidate_id(&self) -> &str {
        "test-fake-not-a-candidate"
    }

    fn candidate_version(&self) -> &str {
        "1"
    }

    fn capabilities(&self) -> CandidateCapabilities {
        CandidateCapabilities {
            optional: self.optional_capabilities.clone(),
            limitations: vec!["in-memory test fake; no durability semantics".to_string()],
        }
    }

    fn create(&mut self, fixture: &EvidenceFixture) -> Result<EvidenceSessionRef, AdapterError> {
        let state = fixture.normalized_state();
        let locator = format!("test-fake-session:{}", state.session.session_id);
        let session = EvidenceSessionRef::new(state.session.session_id.clone(), locator.clone());
        let mut store = self.store.borrow_mut();
        if store.sessions.contains_key(&locator) {
            return Err(AdapterError::new(
                "already-created",
                "test fake session already exists",
            ));
        }
        store.sessions.insert(
            locator,
            FakeSession {
                state,
                handles: BTreeMap::new(),
                writer_handle: None,
            },
        );
        Ok(session)
    }

    fn open(
        &mut self,
        session: &EvidenceSessionRef,
        mode: SemanticOpenMode,
    ) -> Result<EvidenceSessionHandle, AdapterError> {
        let mut store = self.store.borrow_mut();
        if !store.sessions.contains_key(session.adapter_locator()) {
            return Err(AdapterError::new("not-created", "session does not exist"));
        }
        store.next_handle += 1;
        let handle_id = format!("test-fake-handle:{}", store.next_handle);
        let stored = store
            .sessions
            .get_mut(session.adapter_locator())
            .expect("checked fake session");
        if stored.state.session.session_id != session.session_id {
            return Err(AdapterError::new(
                "session-identity-mismatch",
                "semantic session ID does not match adapter locator",
            ));
        }
        if mode == SemanticOpenMode::Writable && stored.writer_handle.is_some() {
            return Err(AdapterError::new(
                "writer-already-open",
                "test fake permits only one authoritative writer",
            ));
        }
        stored.handles.insert(handle_id.clone(), mode);
        if mode == SemanticOpenMode::Writable {
            stored.writer_handle = Some(handle_id.clone());
        }
        Ok(EvidenceSessionHandle::new(session.clone(), mode, handle_id))
    }

    fn close(&mut self, handle: &EvidenceSessionHandle) -> Result<(), AdapterError> {
        let mut store = self.store.borrow_mut();
        let stored = store
            .sessions
            .get_mut(handle.session.adapter_locator())
            .ok_or_else(|| AdapterError::new("not-created", "session does not exist"))?;
        if stored.handles.remove(handle.adapter_handle()).is_none() {
            return Err(AdapterError::new("not-open", "handle is not open"));
        }
        if stored.writer_handle.as_deref() == Some(handle.adapter_handle()) {
            stored.writer_handle = None;
        }
        Ok(())
    }

    fn apply_authoritative_command(
        &mut self,
        handle: &EvidenceSessionHandle,
        command: &AuthoritativeCommand,
    ) -> Result<(), AdapterError> {
        let mut store = self.store.borrow_mut();
        let stored = store
            .sessions
            .get_mut(handle.session.adapter_locator())
            .ok_or_else(|| AdapterError::new("not-created", "session does not exist"))?;
        if handle.mode != SemanticOpenMode::Writable
            || stored.writer_handle.as_deref() != Some(handle.adapter_handle())
        {
            return Err(AdapterError::new(
                "not-authoritative-writer",
                "command requires the active writable handle",
            ));
        }

        let preconditions = match command {
            AuthoritativeCommand::AppendCorrectionEvent { preconditions, .. }
            | AuthoritativeCommand::AttachAnalysisResult { preconditions, .. }
            | AuthoritativeCommand::SelectActiveAnalysis { preconditions, .. }
            | AuthoritativeCommand::ExecuteCleanupPlan { preconditions, .. } => preconditions,
        };
        validate_fake_preconditions(&stored.state, preconditions)?;

        match command {
            AuthoritativeCommand::AppendCorrectionEvent { event, .. } => {
                stored.state.review_ledger_events.push(event.clone());
            }
            AuthoritativeCommand::AttachAnalysisResult {
                analysis_result, ..
            } => {
                stored.state.analysis_results.push(analysis_result.clone());
            }
            AuthoritativeCommand::SelectActiveAnalysis { selection, .. } => {
                if !stored
                    .state
                    .analysis_results
                    .iter()
                    .any(|analysis| analysis.analysis_result_id == selection.analysis_result_id)
                {
                    return Err(AdapterError::new(
                        "unknown-analysis",
                        "active selection references an unknown analysis",
                    ));
                }
                stored.state.active_analysis_selection = Some(selection.clone());
            }
            AuthoritativeCommand::ExecuteCleanupPlan { .. } => {
                return Err(AdapterError::new(
                    "unsupported-test-fake-operation",
                    "test fake does not execute cleanup plans",
                ));
            }
        }
        Ok(())
    }

    fn read_normalized_state(
        &self,
        handle: &EvidenceSessionHandle,
    ) -> Result<NormalizedSemanticState, AdapterError> {
        let store = self.store.borrow();
        let stored = store
            .sessions
            .get(handle.session.adapter_locator())
            .ok_or_else(|| AdapterError::new("not-created", "session does not exist"))?;
        if !stored.handles.contains_key(handle.adapter_handle()) {
            return Err(AdapterError::new("not-open", "handle is not open"));
        }
        Ok(stored.state.clone().normalize())
    }

    fn attempt_read_only_open(
        &mut self,
        session: &EvidenceSessionRef,
    ) -> Result<EvidenceSessionHandle, AdapterError> {
        self.open(session, SemanticOpenMode::ReadOnly)
    }

    fn duplicate_session(
        &mut self,
        source: &EvidenceSessionHandle,
        new_session_id: &str,
    ) -> Result<DuplicatedSession, AdapterError> {
        let mut duplicate = self.read_normalized_state(source)?;
        duplicate.session.duplicated_from_session_id = Some(duplicate.session.session_id.clone());
        duplicate.session.session_id = new_session_id.to_string();
        let locator = format!("test-fake-session:{new_session_id}");
        let session = EvidenceSessionRef::new(new_session_id, locator.clone());
        let mut store = self.store.borrow_mut();
        if store.sessions.contains_key(&locator) {
            return Err(AdapterError::new(
                "duplicate-session-exists",
                "duplicate session identity already exists",
            ));
        }
        store.sessions.insert(
            locator,
            FakeSession {
                state: duplicate.clone(),
                handles: BTreeMap::new(),
                writer_handle: None,
            },
        );
        Ok(DuplicatedSession {
            session,
            normalized_state: duplicate,
        })
    }

    fn corrupt_or_fault_inject(
        &mut self,
        _session: &EvidenceSessionRef,
        _scenario: &ScenarioIdentity,
    ) -> Result<(), AdapterError> {
        Err(AdapterError::new(
            "unsupported-test-fake-operation",
            "no crash injection in foundation fake",
        ))
    }

    fn cleanup_or_compact_if_supported(
        &mut self,
        handle: &EvidenceSessionHandle,
        operation: MaintenanceOperation,
    ) -> Result<OptionalOperationOutcome, AdapterError> {
        if !self
            .store
            .borrow()
            .sessions
            .get(handle.session.adapter_locator())
            .is_some_and(|stored| stored.handles.contains_key(handle.adapter_handle()))
        {
            return Err(AdapterError::new("not-open", "handle is not open"));
        }
        let capability = operation.required_capability();
        if self.optional_capabilities.contains(&capability) {
            Ok(OptionalOperationOutcome::Completed)
        } else {
            Ok(OptionalOperationOutcome::Unsupported {
                capability,
                limitation: "optional operation not implemented by test fake".to_string(),
            })
        }
    }
}

fn validate_fake_preconditions(
    state: &NormalizedSemanticState,
    preconditions: &[SemanticPrecondition],
) -> Result<(), AdapterError> {
    for precondition in preconditions {
        let satisfied = match precondition {
            SemanticPrecondition::SourceRevisionExists {
                expected_revision_id,
            } => state
                .source_revisions
                .iter()
                .any(|revision| revision.revision_id == *expected_revision_id),
            SemanticPrecondition::ReviewLedgerHead { expected_event_id } => {
                state
                    .review_ledger_events
                    .last()
                    .map(|event| &event.event_id)
                    == expected_event_id.as_ref()
            }
            SemanticPrecondition::ActiveAnalysisSelection {
                expected_analysis_result_id,
            } => {
                state
                    .active_analysis_selection
                    .as_ref()
                    .map(|selection| &selection.analysis_result_id)
                    == expected_analysis_result_id.as_ref()
            }
            SemanticPrecondition::AnalysisAttachmentSet {
                expected_analysis_result_ids,
            } => {
                let mut actual: Vec<_> = state
                    .analysis_results
                    .iter()
                    .map(|analysis| analysis.analysis_result_id.clone())
                    .collect();
                let mut expected = expected_analysis_result_ids.clone();
                actual.sort();
                expected.sort();
                actual == expected
            }
        };
        if !satisfied {
            return Err(AdapterError::new(
                "stale-precondition",
                "semantic command precondition does not match current state",
            ));
        }
    }
    Ok(())
}

#[test]
fn fake_adapters_can_address_one_session_with_writer_and_reader_handles() {
    let fixture = EvidenceFixture::small();
    let (mut writer_adapter, mut reader_adapter) = TestFakeAdapter::paired(BTreeSet::new());
    let session = writer_adapter.create(&fixture).expect("fake create");
    let writer = writer_adapter
        .open(&session, SemanticOpenMode::Writable)
        .expect("writer open");
    let reader = reader_adapter
        .attempt_read_only_open(&session)
        .expect("reader open alongside writer");

    assert_eq!(writer.session, reader.session);
    assert!(
        reader_adapter
            .open(&session, SemanticOpenMode::Writable)
            .is_err()
    );
}

#[test]
fn fake_adapter_represents_and_rejects_stale_scoped_precondition() {
    let fixture = EvidenceFixture::small();
    let mut fake = TestFakeAdapter::new(BTreeSet::new());
    let session = fake.create(&fixture).expect("fake create");
    let handle = fake
        .open(&session, SemanticOpenMode::Writable)
        .expect("writer open");
    let state = fake.read_normalized_state(&handle).expect("fake state");
    let expected_head = state
        .review_ledger_events
        .last()
        .expect("fixture ledger head")
        .event_id
        .clone();
    let mut event = state.review_ledger_events[0].clone();
    event.event_id = "ledger-event:004".to_string();
    event.sequence = 4;
    let command = AuthoritativeCommand::AppendCorrectionEvent {
        event,
        preconditions: vec![SemanticPrecondition::ReviewLedgerHead {
            expected_event_id: Some(expected_head),
        }],
    };

    fake.apply_authoritative_command(&handle, &command)
        .expect("fresh command");
    let error = fake
        .apply_authoritative_command(&handle, &command)
        .expect_err("same scoped precondition is stale");

    assert_eq!(error.code, "stale-precondition");
}

#[test]
fn unrelated_state_change_does_not_invalidate_scoped_ledger_precondition() {
    let fixture = EvidenceFixture::small();
    let mut fake = TestFakeAdapter::new(BTreeSet::new());
    let session = fake.create(&fixture).expect("fake create");
    let handle = fake
        .open(&session, SemanticOpenMode::Writable)
        .expect("writer open");
    let state = fake.read_normalized_state(&handle).expect("fake state");
    let expected_head = state
        .review_ledger_events
        .last()
        .expect("fixture ledger head")
        .event_id
        .clone();
    let current_active = state
        .active_analysis_selection
        .as_ref()
        .expect("fixture active analysis")
        .analysis_result_id
        .clone();
    let alternate_analysis = state
        .analysis_results
        .iter()
        .find(|analysis| analysis.analysis_result_id != current_active)
        .expect("fixture alternate analysis")
        .analysis_result_id
        .clone();
    fake.apply_authoritative_command(
        &handle,
        &AuthoritativeCommand::SelectActiveAnalysis {
            selection: ActiveAnalysisSelection {
                analysis_result_id: alternate_analysis,
                selection_event_id: "active-analysis-selection:002".to_string(),
            },
            preconditions: vec![SemanticPrecondition::ActiveAnalysisSelection {
                expected_analysis_result_id: Some(current_active),
            }],
        },
    )
    .expect("scoped active-analysis transition");

    let mut event = state.review_ledger_events[0].clone();
    event.event_id = "ledger-event:004".to_string();
    event.sequence = 4;
    fake.apply_authoritative_command(
        &handle,
        &AuthoritativeCommand::AppendCorrectionEvent {
            event,
            preconditions: vec![SemanticPrecondition::ReviewLedgerHead {
                expected_event_id: Some(expected_head),
            }],
        },
    )
    .expect("unrelated active-analysis change does not stale ledger head");
}

#[test]
fn duplication_returns_independently_addressable_reopenable_session() {
    let fixture = EvidenceFixture::small();
    let (mut source_adapter, mut duplicate_adapter) = TestFakeAdapter::paired(BTreeSet::new());
    let source_ref = source_adapter.create(&fixture).expect("fake create");
    let source_handle = source_adapter
        .open(&source_ref, SemanticOpenMode::Writable)
        .expect("source writer open");
    let source_before = source_adapter
        .read_normalized_state(&source_handle)
        .expect("source state");

    let duplicate = source_adapter
        .duplicate_session(&source_handle, "session:evidence:duplicate")
        .expect("semantic duplicate");
    let duplicate_handle = duplicate_adapter
        .open(&duplicate.session, SemanticOpenMode::Writable)
        .expect("independent duplicate writer");
    let duplicate_state = duplicate_adapter
        .read_normalized_state(&duplicate_handle)
        .expect("duplicate state");
    duplicate_adapter
        .close(&duplicate_handle)
        .expect("close duplicate");
    let reopened_duplicate = duplicate_adapter
        .open(&duplicate.session, SemanticOpenMode::Writable)
        .expect("reopen duplicate");

    assert_eq!(
        duplicate_state.session.duplicated_from_session_id,
        Some(source_before.session.session_id.clone())
    );
    assert_eq!(
        source_adapter
            .read_normalized_state(&source_handle)
            .expect("unchanged source"),
        source_before
    );
    source_adapter.close(&source_handle).expect("close source");
    let reopened_source = source_adapter
        .open(&source_ref, SemanticOpenMode::Writable)
        .expect("reopen source independently");
    assert_eq!(reopened_duplicate.session, duplicate.session);
    assert_eq!(reopened_source.session, source_ref);
    assert_eq!(source_adapter.candidate_id(), "test-fake-not-a-candidate");
}

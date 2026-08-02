use std::panic;

use vox_proof::persistence_evidence::{
    CurrentContractOracle, EvidenceReuseGovernanceEvent, OracleViolationCodeV3,
    build_promoted_active_state, build_superseded_state, comparative_measurement_contract,
    derive_contract_projection, scenario_contract_v3, validate_measurement_contract_value,
    validate_scenario_contracts_v3,
};
use vox_proof::srt::parse_srt;

#[test]
fn noncanonical_review_case_aliases_fail_closed() {
    for alias in ["review-case:00", "review-case:+0", "review-case: 0"] {
        let mut state = build_promoted_active_state();
        state.review_cases[0].case_id = alias.to_owned();
        state.review_ledger_events[0].case_id = alias.to_owned();
        assert!(!CurrentContractOracle::validate(&state).passed, "{alias}");
    }
}

#[test]
fn review_action_payload_shapes_and_alternative_bounds_fail_closed() {
    let mut target_event = build_promoted_active_state();
    target_event.review_ledger_events[0].target_event_index = Some(0);
    assert!(!CurrentContractOracle::validate(&target_event).passed);

    let mut alternative = build_promoted_active_state();
    alternative.review_ledger_events[0].action_kind = "accept_alternative".to_owned();
    alternative.review_ledger_events[0].manual_replacement_bytes = None;
    alternative.review_ledger_events[0].alternative_index =
        Some(alternative.review_cases[0].alternative_count);
    assert!(!CurrentContractOracle::validate(&alternative).passed);

    let mut accept_payload = build_promoted_active_state();
    accept_payload.review_ledger_events[0].action_kind = "accept_alternative".to_owned();
    accept_payload.review_ledger_events[0].alternative_index = Some(0);
    assert!(!CurrentContractOracle::validate(&accept_payload).passed);

    let mut manual_index = build_promoted_active_state();
    manual_index.review_ledger_events[0].alternative_index = Some(0);
    assert!(!CurrentContractOracle::validate(&manual_index).passed);

    for action in ["reject", "defer", "needs_manual_correction"] {
        let mut state = build_promoted_active_state();
        state.review_ledger_events[0].action_kind = action.to_owned();
        state.review_ledger_events[0].alternative_index = Some(0);
        assert!(!CurrentContractOracle::validate(&state).passed, "{action}");
    }
}

#[test]
fn invalid_governance_events_do_not_mutate_the_fold() {
    let mut state = build_promoted_active_state();
    if let EvidenceReuseGovernanceEvent::PromotionAccepted { source_locator, .. } =
        &mut state.reuse_governance_events[0]
    {
        source_locator.decision_digest = "bad".to_owned();
    }
    let (derived, violations) = derive_contract_projection(&state);
    assert!(!violations.is_empty());
    assert!(derived.effective_reusable_records.is_empty());
}

#[test]
fn noncontiguous_governance_event_index_does_not_mutate_the_fold() {
    let mut state = build_promoted_active_state();
    if let EvidenceReuseGovernanceEvent::PromotionAccepted { event_index, .. } =
        &mut state.reuse_governance_events[0]
    {
        *event_index = 1;
    }

    let (derived, violations) = derive_contract_projection(&state);
    assert!(
        violations
            .iter()
            .any(|violation| { violation.code == OracleViolationCodeV3::ReuseGovernanceIndexGap })
    );
    assert!(derived.effective_reusable_records.is_empty());
}

#[test]
fn locator_rejects_analysis_snapshot_from_another_source_revision() {
    let mut state = build_promoted_active_state();
    let mut second = state.source_revisions[0].clone();
    second.transcript_bytes = "1\n00:00:00,000 --> 00:00:01,000\nKafak two\n".to_owned();
    second.revision_id = parse_srt(&second.transcript_bytes)
        .expect("distinct valid transcript")
        .revision_id()
        .to_tagged_string();
    state.source_revisions.push(second);
    let mut snapshot = state.analysis_snapshots[0].clone();
    snapshot.source_revision_id = state.source_revisions[1].revision_id.clone();
    snapshot.identity = snapshot.identity.replacen(
        &state.source_revisions[0].revision_id,
        &snapshot.source_revision_id,
        1,
    );
    state.analysis_snapshots.push(snapshot.clone());
    if let EvidenceReuseGovernanceEvent::PromotionAccepted {
        source_locator,
        candidate_key,
        ..
    } = &mut state.reuse_governance_events[0]
    {
        source_locator.source_analysis_snapshot_identity = snapshot.identity.clone();
        candidate_key
            .source_locator
            .source_analysis_snapshot_identity = snapshot.identity;
    }
    assert!(!CurrentContractOracle::validate(&state).passed);
}

#[test]
fn orphan_analysis_snapshot_fails_canonical_state_validation() {
    let mut state = build_promoted_active_state();
    let mut snapshot = state.analysis_snapshots[0].clone();
    let original = snapshot.source_revision_id.clone();
    snapshot.source_revision_id =
        "rev:sha256-v1:0000000000000000000000000000000000000000000000000000000000000000".to_owned();
    snapshot.identity = snapshot
        .identity
        .replacen(&original, &snapshot.source_revision_id, 1);
    state.analysis_snapshots.push(snapshot);
    assert!(!CurrentContractOracle::validate(&state).passed);
}

#[test]
fn rejection_before_unrelated_promotion_uses_event_index_as_record_id() {
    let state = build_superseded_state();
    let (first_key, second_promotion) = match (
        &state.reuse_governance_events[0],
        &state.reuse_governance_events[1],
    ) {
        (
            EvidenceReuseGovernanceEvent::PromotionAccepted {
                candidate_key,
                actor,
                ..
            },
            EvidenceReuseGovernanceEvent::PromotionAccepted {
                candidate_key: second_key,
                payload,
                source_locator,
                actor: second_actor,
                project_scope_stable_id,
                ..
            },
        ) => (
            EvidenceReuseGovernanceEvent::PromotionCandidateRejected {
                event_index: 0,
                candidate_key: candidate_key.clone(),
                actor: actor.clone(),
            },
            EvidenceReuseGovernanceEvent::PromotionAccepted {
                event_index: 1,
                candidate_key: second_key.clone(),
                payload: payload.clone(),
                source_locator: source_locator.clone(),
                actor: second_actor.clone(),
                project_scope_stable_id: project_scope_stable_id.clone(),
            },
        ),
        _ => panic!("expected two promotions"),
    };
    let mut amended = state;
    amended.reuse_governance_events = vec![first_key, second_promotion];
    let (derived, violations) = derive_contract_projection(&amended);
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(derived.effective_reusable_records[0].record_id, 1);
}

#[test]
fn source_identity_and_unicode_anchor_fail_closed_without_panicking() {
    let mut state = build_promoted_active_state();
    state.source_revisions[0].revision_id =
        "rev:sha256-v1:0000000000000000000000000000000000000000000000000000000000000000".to_owned();
    let result = panic::catch_unwind(|| CurrentContractOracle::validate(&state));
    assert!(result.is_ok());
    assert!(!result.unwrap().passed);
}

#[test]
fn actual_mid_codepoint_anchor_fails_without_panicking() {
    let mut state = build_promoted_active_state();
    state.source_revisions[0].transcript_bytes =
        "1\n00:00:00,000 --> 00:00:01,000\n你Kafak\n".to_owned();
    let revision = parse_srt(&state.source_revisions[0].transcript_bytes)
        .expect("valid unicode transcript")
        .revision_id()
        .to_tagged_string();
    state.source_revisions[0].revision_id = revision.clone();
    state.review_cases[0].observed_revision_id = revision.clone();
    state.review_cases[0].anchor_revision_id = revision.clone();
    state.review_cases[0].anchor_start_byte = 1;
    state.review_cases[0].anchor_end_byte = 3;
    state.review_cases[0].observed_source_bytes = "你".to_owned();
    state.review_ledger_events[0].observed_revision_id = revision;
    let result = panic::catch_unwind(|| CurrentContractOracle::validate(&state));
    assert!(result.is_ok());
    assert!(
        result.unwrap().violations.iter().any(|violation| {
            violation.code == OracleViolationCodeV3::Utf8AnchorBoundaryViolation
        })
    );
}

#[test]
fn mismatched_anchor_revision_and_resolved_bytes_fail_closed() {
    let mut mismatched_revision = build_promoted_active_state();
    mismatched_revision.review_cases[0].anchor_revision_id =
        "rev:sha256-v1:0000000000000000000000000000000000000000000000000000000000000000".to_owned();
    let revision_result = CurrentContractOracle::validate(&mismatched_revision);
    assert!(revision_result.violations.iter().any(|violation| {
        violation.code == OracleViolationCodeV3::SourceAnchorRevisionMismatch
    }));

    let mut mismatched_bytes = build_promoted_active_state();
    mismatched_bytes.review_cases[0].observed_source_bytes = "not-the-anchor".to_owned();
    let bytes_result = CurrentContractOracle::validate(&mismatched_bytes);
    assert!(bytes_result.violations.iter().any(|violation| {
        violation.code == OracleViolationCodeV3::SourceAnchorResolvedBytesMismatch
    }));
}

#[test]
fn source_duplicate_predecessor_and_origin_mutations_fail_closed() {
    let mut duplicate = build_promoted_active_state();
    duplicate
        .source_revisions
        .push(duplicate.source_revisions[0].clone());
    assert!(!CurrentContractOracle::validate(&duplicate).passed);

    let mut predecessor = build_promoted_active_state();
    predecessor.source_revisions[0].predecessor_revision_id =
        Some("rev:sha256-v1:missing".to_owned());
    assert!(!CurrentContractOracle::validate(&predecessor).passed);

    let mut origin = build_promoted_active_state();
    origin.review_cases[0].origin = "human_raised".to_owned();
    assert!(!CurrentContractOracle::validate(&origin).passed);
}

#[test]
fn scenario_mutations_reject_required_catalog_and_wrong_safe_read_condition() {
    let mut missing = scenario_contract_v3();
    missing.remove(0);
    assert!(validate_scenario_contracts_v3(&missing).is_err());

    let mut unsafe_read = scenario_contract_v3();
    let scenario = unsafe_read
        .iter_mut()
        .find(|scenario| scenario.scenario_id == "unknown-newer-format")
        .expect("unknown newer scenario");
    scenario.read_only_open =
        vox_proof::persistence_evidence::current_contract::ReadOnlyOpenPolicy::ConditionallyAllowed {
            required_condition: "probably_safe".to_owned(),
        };
    assert!(validate_scenario_contracts_v3(&unsafe_read).is_err());

    let mut forbidden_read = scenario_contract_v3();
    let scenario = forbidden_read
        .iter_mut()
        .find(|scenario| scenario.scenario_id == "unknown-newer-format")
        .expect("unknown newer scenario");
    scenario.read_only_open =
        vox_proof::persistence_evidence::current_contract::ReadOnlyOpenPolicy::Forbidden;
    assert!(validate_scenario_contracts_v3(&forbidden_read).is_err());
}

#[test]
fn measurement_mutations_reject_fixture_scale_and_candidate_names() {
    let mut wrong_fixture = comparative_measurement_contract();
    wrong_fixture.fixture_id = "wrong".to_owned();
    assert!(validate_measurement_contract_value(&wrong_fixture).is_err());

    let mut empty_scale = comparative_measurement_contract();
    empty_scale.operations[0].fixture_scales.clear();
    assert!(validate_measurement_contract_value(&empty_scale).is_err());

    let mut candidate_named = comparative_measurement_contract();
    candidate_named.operations[0].operation = "append_bundle_probe".to_owned();
    assert!(validate_measurement_contract_value(&candidate_named).is_err());

    for forbidden in [
        "sqlite_probe",
        "embedded_relational_probe",
        "append_bundle_probe",
        "append_authoritative_probe",
    ] {
        let mut contract = comparative_measurement_contract();
        contract.operations[0].operation = forbidden.to_owned();
        assert!(
            validate_measurement_contract_value(&contract).is_err(),
            "{forbidden}"
        );
    }
    let mut small_deferred = comparative_measurement_contract();
    small_deferred.deferred_scales.push(
        vox_proof::persistence_evidence::current_contract::DeferredFixtureScale {
            scale: vox_proof::persistence_evidence::current_contract::measurement::MeasurementFixtureScale::Small,
            rationale: "no".to_owned(), required_future_package: "no".to_owned(),
            unblock_condition: "no".to_owned(), selection_impact: "no".to_owned(),
        },
    );
    assert!(validate_measurement_contract_value(&small_deferred).is_err());
}

#[test]
fn malformed_binding_does_not_pass_as_historical_provenance() {
    let mut state = build_promoted_active_state();
    let binding = state
        .reuse_enabled_analysis_binding
        .as_mut()
        .expect("binding");
    binding.governance_event_boundary = state.reuse_governance_events.len() + 1;
    let result = CurrentContractOracle::validate(&state);
    assert!(result.violations.iter().any(
        |violation| violation.code == OracleViolationCodeV3::HistoricalBindingAnalysisMismatch
    ));
}

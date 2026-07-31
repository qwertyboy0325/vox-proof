use vox_proof::application_export_v3::render_application_session_summary_v3;
use vox_proof::application_service::{
    ApplicationMaterialUseDeclaration, DeclaredApplicationMaterialUseBasis,
    DeclaredSessionAuthority, DeclaredSessionOperatorRole, begin_application_review,
};
use vox_proof::candidate::{Evidence, ResolvedExactInputContributionEvidence, SessionTermEntry};
use vox_proof::pipeline::run_canonical_term_review;
use vox_proof::reusable_influence::{
    ReusableGovernanceEvent, ReusableInfluenceError, build_reusable_influence_snapshot,
    fold_effective_state, resolve_exact_input_projection,
};
use vox_proof::reuse_primitives::ReusableInfluenceRecordId;
use vox_proof::review::CorrectionDecision;
use vox_proof::srt::parse_srt;

fn material_use() -> ApplicationMaterialUseDeclaration {
    ApplicationMaterialUseDeclaration::new(DeclaredApplicationMaterialUseBasis::SelfOwned)
}

fn authority(label: &str) -> DeclaredSessionAuthority {
    DeclaredSessionAuthority::new(
        DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
        label,
    )
    .expect("valid authority")
}

fn alias_entry(canonical: &str, alias: &str) -> SessionTermEntry {
    SessionTermEntry::new(canonical, vec![alias.to_string()], Vec::new())
}

fn observed_entry(canonical: &str, observed: &str) -> SessionTermEntry {
    SessionTermEntry::new(canonical, Vec::new(), vec![observed.to_string()])
}

fn decide_all_manual_replacements(
    session: &mut vox_proof::application_service::ApplicationReviewSession,
    replacement: &str,
) {
    let targets: Vec<_> = session
        .review_items()
        .iter()
        .map(|item| item.target)
        .collect();
    for target in targets {
        session
            .record_manual_replacement(target, replacement)
            .expect("manual replacement");
    }
}

#[test]
fn rejected_candidate_replay_passes_after_later_manual_replacement() {
    let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
    let terms = vec![alias_entry("Kafka", "Kafak")];
    let mut session = begin_application_review(transcript, terms, material_use(), authority("op"))
        .expect("session");
    let target = session.review_items()[0].target;
    session
        .record_manual_replacement(target, "Kafka")
        .expect("replacement");
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session.reject_reuse_candidate(&key).expect("reject");
    session
        .record_manual_replacement(target, "Kafaka")
        .expect("later replacement");
    session.verify_in_memory_replay().expect("replay pass");
}

#[test]
fn rejected_candidate_replay_passes_with_unrelated_decision() {
    let transcript = parse_srt(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak\n\n2\n00:00:01,000 --> 00:00:02,000\nPostgres",
    )
    .expect("valid");
    let terms = vec![
        alias_entry("Kafka", "Kafak"),
        SessionTermEntry::new("PostgreSQL", vec!["Postgres".to_string()], Vec::new()),
    ];
    let mut session = begin_application_review(transcript, terms, material_use(), authority("op"))
        .expect("session");
    let targets: Vec<_> = session
        .review_items()
        .iter()
        .map(|item| item.target)
        .collect();
    session
        .record_manual_replacement(targets[0], "Kafka")
        .expect("replacement");
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session.reject_reuse_candidate(&key).expect("reject");
    session
        .record_human_decision(targets[1], CorrectionDecision::Reject)
        .expect("unrelated");
    session.verify_in_memory_replay().expect("replay pass");
}

#[test]
fn tampered_historical_boundary_replay_fails() {
    let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
    let terms = vec![alias_entry("Kafka", "Kafak")];
    let mut session = begin_application_review(transcript, terms, material_use(), authority("op"))
        .expect("session");
    let target = session.review_items()[0].target;
    session
        .record_manual_replacement(target, "Kafka")
        .expect("replacement");
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session.reject_reuse_candidate(&key).expect("reject");
    let mut tampered = session.reuse_state().governance_events().to_vec();
    if let ReusableGovernanceEvent::PromotionCandidateRejected { candidate_key, .. } =
        &mut tampered[0]
    {
        candidate_key.source_locator.effective_at_ledger_length = 99;
    }
    let scope = session.reuse_state().project_scope().expect("scope");
    let parts = session.reuse_parts();
    assert!(
        vox_proof::application_gate3_replay::validate_replayed_governance_events_for_test(
            parts, scope, &tampered
        )
        .is_err()
    );
}

#[test]
fn candidate_key_source_locator_mismatch_replay_fails() {
    let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
    let terms = vec![alias_entry("Kafka", "Kafak")];
    let mut session = begin_application_review(transcript, terms, material_use(), authority("op"))
        .expect("session");
    let target = session.review_items()[0].target;
    session
        .record_manual_replacement(target, "Kafka")
        .expect("replacement");
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session.accept_reuse_candidate(&key).expect("accept");
    let mut tampered = session.reuse_state().governance_events().to_vec();
    if let ReusableGovernanceEvent::PromotionAccepted {
        candidate_key,
        source_locator,
        ..
    } = &mut tampered[0]
    {
        candidate_key.source_locator.review_ledger_position = 99;
        source_locator.review_ledger_position = 0;
    }
    let scope = session.reuse_state().project_scope().expect("scope");
    let parts = session.reuse_parts();
    assert!(
        vox_proof::application_gate3_replay::validate_replayed_governance_events_for_test(
            parts, scope, &tampered
        )
        .is_err()
    );
}

#[test]
fn duplicate_rejection_replay_fails() {
    let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
    let terms = vec![alias_entry("Kafka", "Kafak")];
    let mut session = begin_application_review(transcript, terms, material_use(), authority("op"))
        .expect("session");
    let target = session.review_items()[0].target;
    session
        .record_manual_replacement(target, "Kafka")
        .expect("replacement");
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session.reject_reuse_candidate(&key).expect("reject");
    let mut tampered = session.reuse_state().governance_events().to_vec();
    tampered.push(tampered[0].clone());
    let scope = session.reuse_state().project_scope().expect("scope");
    let parts = session.reuse_parts();
    assert!(
        vox_proof::application_gate3_replay::validate_replayed_governance_events_for_test(
            parts, scope, &tampered
        )
        .is_err()
    );
}

#[test]
fn promotion_after_rejection_replay_fails() {
    let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
    let terms = vec![alias_entry("Kafka", "Kafak")];
    let mut session = begin_application_review(transcript, terms, material_use(), authority("op"))
        .expect("session");
    let target = session.review_items()[0].target;
    session
        .record_manual_replacement(target, "Kafka")
        .expect("replacement");
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session.reject_reuse_candidate(&key).expect("reject");
    let scope = session.reuse_state().project_scope().expect("scope");
    let mut tampered = session.reuse_state().governance_events().to_vec();
    tampered.push(ReusableGovernanceEvent::PromotionAccepted {
        candidate_key: Box::new(key.clone()),
        payload: vox_proof::reusable_influence::ExactReusableCorrection {
            observed_text: "Kafak".to_owned(),
            confirmed_replacement: "Kafka".to_owned(),
        },
        source_locator: Box::new(key.source_locator.clone()),
        actor: vox_proof::reusable_influence::GovernanceActorContext {
            role_label: "declared_local_owner_operator".to_owned(),
            display_label: "op".to_owned(),
        },
        project_scope: Box::new(scope.clone()),
    });
    let parts = session.reuse_parts();
    assert!(
        vox_proof::application_gate3_replay::validate_replayed_governance_events_for_test(
            parts, scope, &tampered
        )
        .is_err()
    );
}

#[test]
fn tampered_actor_role_replay_fails() {
    let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
    let terms = vec![alias_entry("Kafka", "Kafak")];
    let mut session = begin_application_review(transcript, terms, material_use(), authority("op"))
        .expect("session");
    let target = session.review_items()[0].target;
    session
        .record_manual_replacement(target, "Kafka")
        .expect("replacement");
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session.accept_reuse_candidate(&key).expect("accept");
    let mut tampered = session.reuse_state().governance_events().to_vec();
    if let ReusableGovernanceEvent::PromotionAccepted { actor, .. } = &mut tampered[0] {
        actor.role_label = "tampered_role".to_owned();
    }
    let scope = session.reuse_state().project_scope().expect("scope");
    let parts = session.reuse_parts();
    assert!(
        vox_proof::application_gate3_replay::validate_replayed_governance_events_for_test(
            parts, scope, &tampered
        )
        .is_err()
    );
}

#[test]
fn identical_base_and_reuse_mapping_produces_single_proposal_with_both_provenance() {
    let terms = vec![observed_entry("Kafka", "Kafak")];
    let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
    let mut session = begin_application_review(transcript, terms, material_use(), authority("op"))
        .expect("session");
    let target = session.review_items()[0].target;
    session
        .record_manual_replacement(target, "Kafka")
        .expect("replacement");
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session.accept_reuse_candidate(&key).expect("accept");
    let run = session.run_reuse_enabled_review().expect("run");
    let matching: Vec<_> = run
        .review_cases()
        .iter()
        .filter(|case| {
            matches!(
                case.candidate_span().evidence(),
                Evidence::ReusableExactObservedForm(_)
            )
        })
        .collect();
    assert_eq!(matching.len(), 1);
    let Evidence::ReusableExactObservedForm(evidence) = matching[0].candidate_span().evidence()
    else {
        panic!("expected reusable evidence");
    };
    assert_eq!(evidence.exact_input_contributions.len(), 2);
    assert!(
        evidence
            .exact_input_contributions
            .iter()
            .any(|contribution| matches!(
                contribution,
                ResolvedExactInputContributionEvidence::BaseObservedErrorForm { .. }
            ))
    );
    assert!(
        evidence
            .exact_input_contributions
            .iter()
            .any(|contribution| matches!(
                contribution,
                ResolvedExactInputContributionEvidence::ReusableInfluenceRecord { .. }
            ))
    );
}

#[test]
fn v3_export_includes_base_and_reusable_provenance() {
    let terms = vec![observed_entry("Kafka", "Kafak")];
    let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
    let mut session = begin_application_review(transcript, terms, material_use(), authority("op"))
        .expect("session");
    let target = session.review_items()[0].target;
    session
        .record_manual_replacement(target, "Kafka")
        .expect("replacement");
    for item in session.review_items().iter().skip(1) {
        session
            .record_human_decision(item.target, CorrectionDecision::Defer)
            .expect("defer secondary canonical case");
    }
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session.accept_reuse_candidate(&key).expect("accept");
    session.run_reuse_enabled_review().expect("run");
    let bundle = session
        .materialize_review_export_bundle_v3()
        .expect("bundle");
    let record = &bundle.reuse_enabled_proposal_projections[0];
    assert_eq!(record.exact_input_contributions.len(), 2);
    let summary = render_application_session_summary_v3(&bundle);
    assert!(summary.contains("kind: base_observed_error_form"));
    assert!(summary.contains("kind: reusable_influence_record"));
}

#[test]
fn snapshot_tagged_identity_reports_sha256_v2() {
    let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
    let terms = vec![alias_entry("Kafka", "Kafak")];
    let mut session = begin_application_review(transcript, terms, material_use(), authority("op"))
        .expect("session");
    let target = session.review_items()[0].target;
    session
        .record_manual_replacement(target, "Kafka")
        .expect("replacement");
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session.accept_reuse_candidate(&key).expect("accept");
    let identity = session
        .materialize_review_export_bundle_v3()
        .expect("bundle")
        .reusable_snapshot
        .identity;
    assert!(
        identity
            .to_tagged_string()
            .starts_with("reusable-influence-snapshot:sha256-v2:")
    );
}

#[test]
fn failed_supersession_appends_zero_events() {
    let transcript = parse_srt(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak\n\n2\n00:00:01,000 --> 00:00:02,000\nKafak",
    )
    .expect("valid");
    let terms = vec![alias_entry("Kafka", "Kafak")];
    let mut session = begin_application_review(transcript, terms, material_use(), authority("op"))
        .expect("session");
    decide_all_manual_replacements(&mut session, "Kafka");
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let candidates = session.reuse_candidates().expect("candidates");
    session
        .accept_reuse_candidate(&candidates[0].key)
        .expect("accept");
    let predecessor = ReusableInfluenceRecordId::from_promotion_event_index(0);
    let before = session.reuse_state().governance_events().len();
    assert!(
        session
            .supersede_reusable_influence(predecessor, &candidates[0].key)
            .is_err()
    );
    assert_eq!(session.reuse_state().governance_events().len(), before);
}

#[test]
fn successful_supersession_appends_complete_two_event_batch() {
    let transcript = parse_srt(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak\n\n2\n00:00:01,000 --> 00:00:02,000\nKafak",
    )
    .expect("valid");
    let terms = vec![alias_entry("Kafka", "Kafak")];
    let mut session = begin_application_review(transcript, terms, material_use(), authority("op"))
        .expect("session");
    decide_all_manual_replacements(&mut session, "Kafka");
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let candidates = session.reuse_candidates().expect("candidates");
    session
        .accept_reuse_candidate(&candidates[0].key)
        .expect("accept first");
    let predecessor = ReusableInfluenceRecordId::from_promotion_event_index(0);
    let before = session.reuse_state().governance_events().len();
    session
        .supersede_reusable_influence(predecessor, &candidates[1].key)
        .expect("supersede");
    let events = session.reuse_state().governance_events();
    assert_eq!(events.len(), before + 2);
    assert!(matches!(
        events[before],
        ReusableGovernanceEvent::PromotionAccepted { .. }
    ));
    assert!(matches!(
        events[before + 1],
        ReusableGovernanceEvent::ReusableInfluenceSuperseded { .. }
    ));
}

#[test]
fn projection_record_payload_mismatch_fails_detection() {
    let terms = vec![observed_entry("Kafka", "Kafak")];
    let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
    let mut session =
        begin_application_review(transcript, terms.clone(), material_use(), authority("op"))
            .expect("session");
    let target = session.review_items()[0].target;
    session
        .record_manual_replacement(target, "Kafka")
        .expect("replacement");
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session.accept_reuse_candidate(&key).expect("accept");
    let scope = session.reuse_state().project_scope().expect("scope");
    let canonical = run_canonical_term_review(session.source(), &terms).expect("canonical");
    let effective = fold_effective_state(
        session.reuse_state().governance_ledger(),
        session.review_ledger(),
        &canonical,
    );
    let snapshot = build_reusable_influence_snapshot(
        scope,
        session.reuse_state().governance_ledger(),
        &effective,
    );
    let projection = resolve_exact_input_projection(scope, &snapshot, &terms).expect("projection");
    let mut stripped_snapshot = snapshot.clone();
    stripped_snapshot.active_records.clear();
    assert!(matches!(
        vox_proof::reusable_influence::assert_projection_matches_expected(
            &projection,
            &stripped_snapshot,
            &terms,
        ),
        Err(ReusableInfluenceError::ProjectionContentMismatch)
    ));
}

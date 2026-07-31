use vox_proof::application_export_v3::render_application_session_summary_v3;
use vox_proof::application_service::{
    ApplicationMaterialUseDeclaration, DeclaredApplicationMaterialUseBasis,
    DeclaredSessionAuthority, DeclaredSessionOperatorRole, begin_application_review,
};
use vox_proof::candidate::SessionTermEntry;
use vox_proof::pipeline::run_canonical_term_review;
use vox_proof::reusable_influence::{
    ReusableGovernanceEvent, ReusableInfluenceError, assert_projection_matches_expected,
    resolve_exact_input_projection,
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

#[test]
fn governance_events_change_only_through_bounded_commands() {
    let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
    let terms = vec![alias_entry("Kafka", "Kafak")];
    let mut session = begin_application_review(transcript, terms, material_use(), authority("op"))
        .expect("session");
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let target = session.review_items()[0].target;
    session
        .record_manual_replacement(target, "Kafka")
        .expect("replacement");
    assert!(session.reuse_state().governance_events().is_empty());
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session.accept_reuse_candidate(&key).expect("accept");
    assert_eq!(session.reuse_state().governance_events().len(), 1);
}

#[test]
fn later_manual_replacement_marks_promoted_source_occurrence_ineffective() {
    let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
    let terms = vec![alias_entry("Kafka", "Kafak")];
    let mut session = begin_application_review(transcript, terms, material_use(), authority("op"))
        .expect("session");
    let target = session.review_items()[0].target;
    session
        .record_manual_replacement(target, "Kafka")
        .expect("first replacement");
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session.accept_reuse_candidate(&key).expect("accept");
    session
        .record_manual_replacement(target, "KAFKA")
        .expect("second replacement");
    let active = session.active_reusable_records().expect("active");
    assert_eq!(active.len(), 1);
    assert!(!active[0].source_decision_still_effective);
}

#[test]
fn unrelated_review_event_does_not_resurrect_rejected_candidate() {
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
    let candidates = session.reuse_candidates().expect("candidates");
    assert!(candidates.is_empty());
}

#[test]
fn new_manual_replacement_occurrence_creates_new_candidate_identity() {
    let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
    let terms = vec![alias_entry("Kafka", "Kafak")];
    let mut session = begin_application_review(transcript, terms, material_use(), authority("op"))
        .expect("session");
    let target = session.review_items()[0].target;
    session
        .record_manual_replacement(target, "Kafka")
        .expect("first");
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let first_key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session.reject_reuse_candidate(&first_key).expect("reject");
    session
        .record_manual_replacement(target, "KAFKA")
        .expect("second");
    let second_key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    assert_ne!(first_key, second_key);
}

#[test]
fn reuse_enabled_analysis_identity_binds_resolved_detector_set() {
    let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
    let terms = vec![alias_entry("Kafka", "Kafak")];
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
    let run = session.run_reuse_enabled_review().expect("run");
    let reuse_configuration = run.analysis_run().snapshot().configuration();
    assert!(
        reuse_configuration
            .detector_set()
            .detectors()
            .iter()
            .any(|detector| detector.id() == "resolved-exact-observed-form-match")
    );
    let canonical = run_canonical_term_review(session.source(), &terms).expect("canonical");
    assert_ne!(
        reuse_configuration,
        canonical.analysis_run().snapshot().configuration()
    );
}

#[test]
fn mismatched_projection_snapshot_identity_refuses_detection() {
    let transcript_a = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
    let terms = vec![alias_entry("Kafka", "Kafak")];
    let mut session =
        begin_application_review(transcript_a, terms.clone(), material_use(), authority("op"))
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
    session.run_reuse_enabled_review().expect("run");
    let snapshot = session
        .materialize_review_export_bundle_v3()
        .expect("bundle")
        .reusable_snapshot;
    let scope = session.reuse_state().project_scope().expect("scope");
    let projection = resolve_exact_input_projection(scope, &snapshot, &terms).expect("projection");
    let transcript_b = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
    let mut other_session = begin_application_review(
        transcript_b,
        terms.clone(),
        material_use(),
        authority("other"),
    )
    .expect("session");
    let other_target = other_session.review_items()[0].target;
    other_session
        .record_manual_replacement(other_target, "Kafka")
        .expect("replacement");
    other_session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let other_key = other_session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    other_session
        .accept_reuse_candidate(&other_key)
        .expect("accept");
    let mismatched_snapshot = other_session
        .materialize_review_export_bundle_v3()
        .expect("bundle")
        .reusable_snapshot;
    assert_ne!(snapshot.identity(), mismatched_snapshot.identity());
    assert!(matches!(
        assert_projection_matches_expected(&projection, &mismatched_snapshot, &terms),
        Err(ReusableInfluenceError::ProjectionSnapshotIdentityMismatch)
            | Err(ReusableInfluenceError::SnapshotIdentityMismatch)
    ));
}

#[test]
fn v3_export_includes_typed_reuse_enabled_proposal_projections() {
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
    session.run_reuse_enabled_review().expect("run");
    let bundle = session
        .materialize_review_export_bundle_v3()
        .expect("bundle");
    assert_eq!(bundle.reuse_enabled_proposal_projections.len(), 1);
    let summary = render_application_session_summary_v3(&bundle);
    assert!(summary.contains("Reuse-enabled proposal projections (non-authoritative)"));
    assert!(summary.contains("resolved-exact-observed-form-match"));
}

#[test]
fn supersede_prevalidates_successor_before_transition() {
    let transcript = parse_srt(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak\n\n2\n00:00:01,000 --> 00:00:02,000\nKafak",
    )
    .expect("valid");
    let terms = vec![alias_entry("Kafka", "Kafak")];
    let mut session = begin_application_review(transcript, terms, material_use(), authority("op"))
        .expect("session");
    let targets: Vec<_> = session
        .review_items()
        .iter()
        .map(|item| item.target)
        .collect();
    for target in &targets {
        session
            .record_manual_replacement(*target, "Kafka")
            .expect("replacement");
    }
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let candidates = session.reuse_candidates().expect("candidates");
    session
        .accept_reuse_candidate(&candidates[0].key)
        .expect("accept first");
    let predecessor = ReusableInfluenceRecordId::from_promotion_event_index(0);
    let bogus_key = candidates[0].key.clone();
    assert!(
        session
            .supersede_reusable_influence(predecessor, &bogus_key)
            .is_err()
    );
    assert_eq!(session.reuse_state().governance_events().len(), 1);
    session
        .supersede_reusable_influence(predecessor, &candidates[1].key)
        .expect("supersede");
    assert_eq!(session.reuse_state().governance_events().len(), 3);
}

#[test]
fn replay_rejects_tampered_promotion_payload() {
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
    if let ReusableGovernanceEvent::PromotionAccepted { payload, .. } = &mut tampered[0] {
        payload.observed_text = "tampered".to_owned();
    }
    let scope = session.reuse_state().project_scope().expect("scope");
    let parts = session.reuse_parts();
    assert!(
        vox_proof::application_gate3_replay::validate_replayed_governance_events_for_test(
            parts,
            scope,
            &tampered,
            session.session_authority(),
        )
        .is_err()
    );
    session.verify_in_memory_replay().expect("valid replay");
}

#[test]
fn different_promotion_actors_change_snapshot_identity() {
    fn snapshot_after_accept(
        operator: &str,
    ) -> vox_proof::reuse_primitives::ReusableInfluenceSnapshotIdentity {
        let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
        let terms = vec![alias_entry("Kafka", "Kafak")];
        let mut session =
            begin_application_review(transcript, terms, material_use(), authority(operator))
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
        session
            .materialize_review_export_bundle_v3()
            .expect("bundle")
            .reusable_snapshot
            .identity()
    }
    assert_ne!(snapshot_after_accept("op-a"), snapshot_after_accept("op-b"));
}

#[test]
fn different_source_cases_change_snapshot_identity() {
    let transcript = parse_srt(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak\n\n2\n00:00:01,000 --> 00:00:02,000\nKafak",
    )
    .expect("valid");
    let terms = vec![alias_entry("Kafka", "Kafak")];
    let mut session = begin_application_review(transcript, terms, material_use(), authority("op"))
        .expect("session");
    let targets: Vec<_> = session
        .review_items()
        .iter()
        .map(|item| item.target)
        .collect();
    for target in &targets {
        session
            .record_manual_replacement(*target, "Kafka")
            .expect("replacement");
    }
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let candidates = session.reuse_candidates().expect("candidates");
    assert_eq!(candidates.len(), 2);
    session
        .accept_reuse_candidate(&candidates[0].key)
        .expect("accept first");
    let first_identity = session
        .materialize_review_export_bundle_v3()
        .expect("bundle")
        .reusable_snapshot
        .identity();
    session
        .accept_reuse_candidate(&candidates[1].key)
        .expect("accept second");
    let both_identity = session
        .materialize_review_export_bundle_v3()
        .expect("bundle")
        .reusable_snapshot
        .identity();
    assert_ne!(first_identity, both_identity);
}

#[test]
fn same_observed_and_replacement_strings_with_distinct_provenance_differ() {
    let transcript = parse_srt(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak\n\n2\n00:00:01,000 --> 00:00:02,000\nKafak",
    )
    .expect("valid");
    let terms = vec![alias_entry("Kafka", "Kafak")];
    let mut session = begin_application_review(transcript, terms, material_use(), authority("op"))
        .expect("session");
    let targets: Vec<_> = session
        .review_items()
        .iter()
        .map(|item| item.target)
        .collect();
    for target in &targets {
        session
            .record_manual_replacement(*target, "Kafka")
            .expect("replacement");
    }
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    for key in session
        .reuse_candidates()
        .expect("candidates")
        .into_iter()
        .map(|candidate| candidate.key)
        .collect::<Vec<_>>()
    {
        session.accept_reuse_candidate(&key).expect("accept");
    }
    let active = session.active_reusable_records().expect("active");
    assert_eq!(active.len(), 2);
    assert_eq!(
        active[0].payload.observed_text,
        active[1].payload.observed_text
    );
    assert_eq!(
        active[0].payload.confirmed_replacement,
        active[1].payload.confirmed_replacement
    );
    assert_ne!(active[0].source_locator, active[1].source_locator);
    let identity = session
        .materialize_review_export_bundle_v3()
        .expect("bundle")
        .reusable_snapshot
        .identity();
    session
        .revoke_reusable_influence(active[0].record_id)
        .expect("revoke one");
    let reduced_identity = session
        .materialize_review_export_bundle_v3()
        .expect("bundle")
        .reusable_snapshot
        .identity();
    assert_ne!(identity, reduced_identity);
}

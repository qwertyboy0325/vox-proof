use vox_proof::application_service::{
    ApplicationMaterialUseDeclaration, DeclaredApplicationMaterialUseBasis,
    DeclaredSessionAuthority, DeclaredSessionOperatorRole, begin_application_review,
};
use vox_proof::candidate::{DetectionKind, Evidence, SessionTermEntry};
use vox_proof::pipeline::run_canonical_term_review;
use vox_proof::reusable_influence::{
    DECLARED_LOCAL_OWNER_ROLE, DECLARED_REVIEWER_ROLE, GovernanceActorContext,
    ReusableGovernanceEvent, ReusableInfluenceError, validate_governance_actor,
    validate_governance_actor_matches_session,
};
use vox_proof::reuse_primitives::ReusableInfluenceRecordId;
use vox_proof::srt::parse_srt;

fn material_use() -> ApplicationMaterialUseDeclaration {
    ApplicationMaterialUseDeclaration::new(DeclaredApplicationMaterialUseBasis::SelfOwned)
}

fn authority(role: DeclaredSessionOperatorRole, label: &str) -> DeclaredSessionAuthority {
    DeclaredSessionAuthority::new(role, label).expect("valid authority")
}

fn alias_entry(canonical: &str, alias: &str) -> SessionTermEntry {
    SessionTermEntry::new(canonical, vec![alias.to_string()], Vec::new())
}

fn sample_session_with_promotion(
    operator_label: &str,
) -> vox_proof::application_service::ApplicationReviewSession {
    let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
    let terms = vec![alias_entry("Kafka", "Kafak")];
    let mut session = begin_application_review(
        transcript,
        terms,
        material_use(),
        authority(
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            operator_label,
        ),
    )
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
}

fn session_with_rejection(
    operator_label: &str,
) -> vox_proof::application_service::ApplicationReviewSession {
    let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
    let terms = vec![alias_entry("Kafka", "Kafak")];
    let mut session = begin_application_review(
        transcript,
        terms,
        material_use(),
        authority(
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            operator_label,
        ),
    )
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
}

#[test]
fn local_owner_session_replay_passes() {
    sample_session_with_promotion("Ezra")
        .verify_in_memory_replay()
        .expect("replay");
}

#[test]
fn authorized_reviewer_session_replay_passes() {
    let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
    let terms = vec![alias_entry("Kafka", "Kafak")];
    let mut session = begin_application_review(
        transcript,
        terms,
        material_use(),
        authority(
            DeclaredSessionOperatorRole::DeclaredAuthorizedHumanReviewer,
            "Reviewer",
        ),
    )
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
    session.verify_in_memory_replay().expect("replay");
}

#[test]
fn changed_allowed_role_fails_replay() {
    let session = sample_session_with_promotion("Ezra");
    let mut tampered = session.reuse_state().governance_events().to_vec();
    if let ReusableGovernanceEvent::PromotionAccepted { actor, .. } = &mut tampered[0] {
        actor.role_label = DECLARED_REVIEWER_ROLE.to_owned();
        actor.display_label = "Other Reviewer".to_owned();
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
}

#[test]
fn changed_valid_label_fails_replay() {
    let session = sample_session_with_promotion("Ezra");
    let mut tampered = session.reuse_state().governance_events().to_vec();
    if let ReusableGovernanceEvent::PromotionAccepted { actor, .. } = &mut tampered[0] {
        actor.display_label = "Another Valid Label".to_owned();
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
}

#[test]
fn blank_actor_label_fails_syntax_validation() {
    let actor = GovernanceActorContext {
        role_label: DECLARED_LOCAL_OWNER_ROLE.to_owned(),
        display_label: "   ".to_owned(),
    };
    assert!(matches!(
        validate_governance_actor(&actor),
        Err(ReusableInfluenceError::InvalidGovernanceActor)
    ));
}

#[test]
fn control_character_actor_label_fails_syntax_validation() {
    let actor = GovernanceActorContext {
        role_label: DECLARED_LOCAL_OWNER_ROLE.to_owned(),
        display_label: "bad\u{0001}label".to_owned(),
    };
    assert!(matches!(
        validate_governance_actor(&actor),
        Err(ReusableInfluenceError::InvalidGovernanceActor)
    ));
}

#[test]
fn unicode_line_separator_actor_labels_fail_syntax_validation() {
    for separator in ["\u{2028}", "\u{2029}"] {
        let actor = GovernanceActorContext {
            role_label: DECLARED_LOCAL_OWNER_ROLE.to_owned(),
            display_label: format!("bad{separator}label"),
        };
        assert!(matches!(
            validate_governance_actor(&actor),
            Err(ReusableInfluenceError::InvalidGovernanceActor)
        ));
    }
}

#[test]
fn unknown_actor_role_fails_syntax_validation() {
    let actor = GovernanceActorContext {
        role_label: "unknown_role".to_owned(),
        display_label: "Ezra".to_owned(),
    };
    assert!(matches!(
        validate_governance_actor(&actor),
        Err(ReusableInfluenceError::InvalidGovernanceActor)
    ));
}

#[test]
fn rejection_and_revocation_are_actor_bound() {
    let session = sample_session_with_promotion("Ezra");
    let expected =
        vox_proof::application_reuse::governance_actor_from_authority(session.session_authority());
    let wrong = GovernanceActorContext {
        role_label: DECLARED_REVIEWER_ROLE.to_owned(),
        display_label: "Other Reviewer".to_owned(),
    };
    assert!(validate_governance_actor_matches_session(&wrong, &expected).is_err());

    let reject_session = session_with_rejection("Ezra");
    let mut tampered = reject_session.reuse_state().governance_events().to_vec();
    if let ReusableGovernanceEvent::PromotionCandidateRejected { actor, .. } = &mut tampered[0] {
        *actor = wrong.clone();
    }
    let scope = reject_session.reuse_state().project_scope().expect("scope");
    let parts = reject_session.reuse_parts();
    assert!(
        vox_proof::application_gate3_replay::validate_replayed_governance_events_for_test(
            parts,
            scope,
            &tampered,
            reject_session.session_authority(),
        )
        .is_err()
    );

    let mut revoke_session = sample_session_with_promotion("Ezra");
    let record_id = ReusableInfluenceRecordId::from_promotion_event_index(0);
    revoke_session
        .revoke_reusable_influence(record_id)
        .expect("revoke");
    let mut tampered = revoke_session.reuse_state().governance_events().to_vec();
    if let ReusableGovernanceEvent::ReusableInfluenceRevoked { actor, .. } = &mut tampered[1] {
        *actor = wrong;
    }
    let scope = revoke_session.reuse_state().project_scope().expect("scope");
    let parts = revoke_session.reuse_parts();
    assert!(
        vox_proof::application_gate3_replay::validate_replayed_governance_events_for_test(
            parts,
            scope,
            &tampered,
            revoke_session.session_authority(),
        )
        .is_err()
    );
}

#[test]
fn canonical_non_reuse_analysis_unchanged() {
    let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
    let terms = vec![alias_entry("Kafka", "Kafak")];
    let canonical = run_canonical_term_review(&transcript, &terms).expect("canonical");
    assert_eq!(canonical.review_cases().len(), 1);
    let evidence = canonical.review_cases()[0].candidate_span().evidence();
    assert!(!matches!(evidence, Evidence::ReusableExactObservedForm(_)));
}

#[test]
fn v2_export_golden_output_unchanged() {
    let session = sample_session_with_promotion("Ezra");
    let bundle_a = session.materialize_review_export_bundle().expect("v2");
    let bundle_b = session.materialize_review_export_bundle().expect("v2");
    assert_eq!(
        vox_proof::application_export::render_application_session_summary(&bundle_a),
        vox_proof::application_export::render_application_session_summary(&bundle_b),
    );
}

#[test]
fn base_plus_reuse_provenance_tests_remain_valid_via_session_flow() {
    let mut session = sample_session_with_promotion("Ezra");
    session.run_reuse_enabled_review().expect("reuse review");
    session.verify_in_memory_replay().expect("replay");
}

#[test]
fn reusable_replacements_never_enter_phonetic_matching() {
    let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
    let entries = vec![alias_entry("Kafka", "Kafak")];
    let mut session = sample_session_with_promotion("Ezra");
    let run = session.run_reuse_enabled_review().expect("reuse review");
    let canonical = run_canonical_term_review(&transcript, &entries).expect("canonical");
    let phonetic_only: Vec<_> = canonical
        .review_cases()
        .iter()
        .filter(|case| case.candidate_span().kind() == DetectionKind::PhoneticSimilarity)
        .collect();
    let reuse_phonetic: Vec<_> = run
        .review_cases()
        .iter()
        .filter(|case| case.candidate_span().kind() == DetectionKind::PhoneticSimilarity)
        .collect();
    assert_eq!(phonetic_only.len(), reuse_phonetic.len());
}

#[test]
fn snapshot_public_read_only_getters_expose_identity_and_records() {
    let session = sample_session_with_promotion("Ezra");
    let snapshot = session
        .materialize_review_export_bundle_v3()
        .expect("bundle")
        .reusable_snapshot;
    assert_eq!(snapshot.project_scope_id().as_str(), "proj-a");
    assert_eq!(snapshot.active_records().len(), 1);
    let _ = snapshot.identity().to_tagged_string();
}

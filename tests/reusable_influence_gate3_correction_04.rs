use vox_proof::application_service::{
    ApplicationMaterialUseDeclaration, DeclaredApplicationMaterialUseBasis,
    DeclaredSessionAuthority, DeclaredSessionAuthorityError, DeclaredSessionOperatorRole,
    begin_application_review,
};
use vox_proof::candidate::{DetectionKind, SessionTermEntry};
use vox_proof::pipeline::run_canonical_term_review;
use vox_proof::reusable_influence::{
    DECLARED_LOCAL_OWNER_ROLE, GovernanceActorContext, ReusableGovernanceEvent,
    validate_governance_actor,
};
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

#[test]
fn public_application_apis_materialize_snapshot_and_v3_bundle() {
    let mut session = sample_session_with_promotion("Ezra");
    session.run_reuse_enabled_review().expect("reuse review");
    let bundle = session
        .materialize_review_export_bundle_v3()
        .expect("v3 bundle");
    assert_eq!(bundle.reusable_snapshot.active_records().len(), 1);
    assert_eq!(
        bundle
            .reuse_enabled_analysis
            .expect("analysis")
            .reusable_influence_snapshot(),
        bundle.reusable_snapshot.identity()
    );
}

#[test]
fn authority_declaration_rejects_line_separators() {
    assert_eq!(
        DeclaredSessionAuthority::new(
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "Ezra\u{2028}Reviewer",
        ),
        Err(DeclaredSessionAuthorityError::UnicodeLineSeparatorInDisplayLabel)
    );
    assert_eq!(
        DeclaredSessionAuthority::new(
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "Ezra\u{2029}Reviewer",
        ),
        Err(DeclaredSessionAuthorityError::UnicodeLineSeparatorInDisplayLabel)
    );
}

#[test]
fn authority_declaration_canonicalizes_whitespace() {
    let declared = DeclaredSessionAuthority::new(
        DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
        " Ezra ",
    )
    .expect("canonical");
    assert_eq!(declared.display_label(), "Ezra");
}

#[test]
fn governance_actor_rejects_non_canonical_stored_label() {
    let actor = GovernanceActorContext {
        role_label: DECLARED_LOCAL_OWNER_ROLE.to_owned(),
        display_label: " Ezra ".to_owned(),
    };
    assert!(validate_governance_actor(&actor).is_err());
}

#[test]
fn replay_rejects_padded_event_actor_label() {
    let session = sample_session_with_promotion("Ezra");
    let mut tampered = session.reuse_state().governance_events().to_vec();
    if let ReusableGovernanceEvent::PromotionAccepted { actor, .. } = &mut tampered[0] {
        actor.display_label = " Ezra ".to_owned();
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
fn owner_and_reviewer_sessions_replay_successfully() {
    let session = sample_session_with_promotion("Ezra");
    session.verify_in_memory_replay().expect("owner replay");
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
    session.verify_in_memory_replay().expect("reviewer replay");
}

#[test]
fn canonical_non_reuse_analysis_unchanged() {
    let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
    let terms = vec![alias_entry("Kafka", "Kafak")];
    let canonical = run_canonical_term_review(&transcript, &terms).expect("canonical");
    assert_eq!(canonical.review_cases().len(), 1);
}

#[test]
fn v2_export_remains_stable_for_same_session() {
    let session = sample_session_with_promotion("Ezra");
    let a = session.materialize_review_export_bundle().expect("v2");
    let b = session.materialize_review_export_bundle().expect("v2");
    assert_eq!(
        vox_proof::application_export::render_application_session_summary(&a),
        vox_proof::application_export::render_application_session_summary(&b),
    );
}

#[test]
fn reusable_replacements_never_enter_phonetic_matching() {
    let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
    let entries = vec![alias_entry("Kafka", "Kafak")];
    let mut session = sample_session_with_promotion("Ezra");
    session.run_reuse_enabled_review().expect("reuse review");
    let canonical = run_canonical_term_review(&transcript, &entries).expect("canonical");
    let phonetic_only = canonical
        .review_cases()
        .iter()
        .filter(|case| case.candidate_span().kind() == DetectionKind::PhoneticSimilarity)
        .count();
    let reuse_phonetic = session
        .reuse_enabled_run()
        .expect("run")
        .review_cases()
        .iter()
        .filter(|case| case.candidate_span().kind() == DetectionKind::PhoneticSimilarity)
        .count();
    assert_eq!(phonetic_only, reuse_phonetic);
}

#[test]
fn historical_prefix_replay_remains_green() {
    let session = sample_session_with_promotion("Ezra");
    session.verify_in_memory_replay().expect("replay");
}

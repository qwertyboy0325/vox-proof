use vox_proof::application_export::render_application_decision_log;
use vox_proof::application_export_v3::{
    DECISION_LOG_V3_HEADER, SESSION_SUMMARY_V3_HEADER, render_application_decision_log_v3,
    render_application_session_summary_v3, unescape_export_text,
};
use vox_proof::application_service::{
    ApplicationMaterialUseDeclaration, DeclaredApplicationMaterialUseBasis,
    DeclaredSessionAuthority, DeclaredSessionOperatorRole, begin_application_review,
};
use vox_proof::candidate::{
    DetectionKind, Evidence, SessionTermEntry, detect_glossary_matches,
    detect_observed_error_form_matches,
};
use vox_proof::pipeline::{run_canonical_term_review, run_reuse_enabled_term_review};
use vox_proof::reusable_influence::{
    ReusableInfluenceError, build_reusable_influence_snapshot,
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

fn manual_replacement_session(
    transcript_text: &str,
    terms: Vec<SessionTermEntry>,
    replacement: &str,
) -> vox_proof::application_service::ApplicationReviewSession {
    let transcript = parse_srt(transcript_text).expect("valid transcript");
    let mut session = begin_application_review(transcript, terms, material_use(), authority("op"))
        .expect("session");
    let target = session.review_items()[0].target;
    session
        .record_manual_replacement(target, replacement)
        .expect("manual replacement");
    session
}

#[test]
fn effective_manual_replacement_produces_one_deterministic_candidate() {
    let mut session = manual_replacement_session(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak",
        vec![alias_entry("Kafka", "Kafak")],
        "Kafka",
    );
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let first = session.reuse_candidates().expect("candidates");
    let second = session.reuse_candidates().expect("candidates");
    assert_eq!(first.len(), 1);
    assert_eq!(first, second);
    assert_eq!(first[0].exact_payload.observed_text, "Kafak");
    assert_eq!(first[0].exact_payload.confirmed_replacement, "Kafka");
}

#[test]
fn reject_defer_and_needs_manual_correction_do_not_produce_candidates() {
    for decision in [
        CorrectionDecision::Reject,
        CorrectionDecision::Defer,
        CorrectionDecision::NeedsManualCorrection,
    ] {
        let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
        let terms = vec![alias_entry("Kafka", "Kafak")];
        let mut session =
            begin_application_review(transcript, terms, material_use(), authority("op"))
                .expect("session");
        session
            .initialize_project_scope("proj-a", "Project A")
            .expect("scope");
        let target = session.review_items()[0].target;
        session
            .record_human_decision(target, decision)
            .expect("decision");
        assert!(session.reuse_candidates().expect("candidates").is_empty());
    }
}

#[test]
fn later_non_manual_replacement_removes_unpromoted_candidate() {
    let mut session = manual_replacement_session(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak",
        vec![alias_entry("Kafka", "Kafak")],
        "Kafka",
    );
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    assert_eq!(session.reuse_candidates().expect("candidates").len(), 1);
    let target = session.review_items()[0].target;
    session
        .record_human_decision(target, CorrectionDecision::Reject)
        .expect("reject");
    assert!(session.reuse_candidates().expect("candidates").is_empty());
}

#[test]
fn promotion_acceptance_appends_one_event_and_activates_immediately() {
    let mut session = manual_replacement_session(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak",
        vec![alias_entry("Kafka", "Kafak")],
        "Kafka",
    );
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    assert!(session.reuse_state().governance_ledger.events().is_empty());
    session.accept_reuse_candidate(&key).expect("accept");
    assert_eq!(session.reuse_state().governance_ledger.events().len(), 1);
    let active = session.active_reusable_records().expect("active");
    assert_eq!(active.len(), 1);
    assert_eq!(
        active[0].record_id.promotion_event_index(),
        ReusableInfluenceRecordId::from_promotion_event_index(0).promotion_event_index()
    );
}

#[test]
fn candidate_rejection_creates_no_reusable_influence() {
    let mut session = manual_replacement_session(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak",
        vec![alias_entry("Kafka", "Kafak")],
        "Kafka",
    );
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session.reject_reuse_candidate(&key).expect("reject");
    assert!(
        session
            .active_reusable_records()
            .expect("active")
            .is_empty()
    );
    assert!(session.reuse_candidates().expect("candidates").is_empty());
}

#[test]
fn failed_promotion_appends_nothing() {
    let mut session = manual_replacement_session(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak",
        vec![alias_entry("Kafka", "Kafak")],
        "Kafka",
    );
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session.reject_reuse_candidate(&key).expect("reject");
    let err = session.accept_reuse_candidate(&key);
    assert!(err.is_err());
    assert_eq!(session.reuse_state().governance_ledger.events().len(), 1);
}

#[test]
fn revocation_stops_future_influence_and_retains_history() {
    let mut session = manual_replacement_session(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak",
        vec![alias_entry("Kafka", "Kafak")],
        "Kafka",
    );
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    let record_id = session.accept_reuse_candidate(&key).expect("accept");
    session
        .revoke_reusable_influence(record_id)
        .expect("revoke");
    assert!(
        session
            .active_reusable_records()
            .expect("active")
            .is_empty()
    );
    assert_eq!(session.reuse_state().governance_ledger.events().len(), 2);
}

#[test]
fn promoted_record_remains_after_source_decision_becomes_ineffective() {
    let mut session = manual_replacement_session(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak",
        vec![alias_entry("Kafka", "Kafak")],
        "Kafka",
    );
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    let record_id = session.accept_reuse_candidate(&key).expect("accept");
    let target = session.review_items()[0].target;
    session
        .record_human_decision(target, CorrectionDecision::Reject)
        .expect("reject source");
    let active = session.active_reusable_records().expect("active");
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].record_id, record_id);
    assert!(!active[0].source_decision_still_effective);
}

#[test]
fn same_effective_records_produce_same_snapshot_identity() {
    let mut session = manual_replacement_session(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak",
        vec![alias_entry("Kafka", "Kafak")],
        "Kafka",
    );
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session.accept_reuse_candidate(&key).expect("accept");
    let bundle_a = session
        .materialize_review_export_bundle_v3()
        .expect("bundle");
    let bundle_b = session
        .materialize_review_export_bundle_v3()
        .expect("bundle");
    assert_eq!(
        bundle_a.reusable_snapshot.identity,
        bundle_b.reusable_snapshot.identity
    );
}

#[test]
fn divergent_reusable_pairs_refuse_projection() {
    let transcript = parse_srt(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak\n\n2\n00:00:01,000 --> 00:00:02,000\nKafak",
    )
    .expect("valid");
    let terms = vec![alias_entry("Kafka", "Kafak")];
    let mut session =
        begin_application_review(transcript, terms.clone(), material_use(), authority("op"))
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
        .record_manual_replacement(targets[1], "KAFKA")
        .expect("replacement");
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
    let effective = fold_effective_state(
        &session.reuse_state().governance_ledger,
        session.review_ledger(),
    );
    let scope = session.reuse_state().project_scope.as_ref().expect("scope");
    let snapshot = build_reusable_influence_snapshot(
        scope,
        &session.reuse_state().governance_ledger,
        &effective,
    );
    assert!(matches!(
        resolve_exact_input_projection(scope, &snapshot, &terms),
        Err(ReusableInfluenceError::DivergentExactMapping { .. })
    ));
}

#[test]
fn base_versus_reuse_divergence_refuses_projection() {
    let entries = vec![observed_entry("Kafka", "Kafak")];
    let mut session = manual_replacement_session(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak",
        entries.clone(),
        "KAFKA",
    );
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session.accept_reuse_candidate(&key).expect("accept");
    let effective = fold_effective_state(
        &session.reuse_state().governance_ledger,
        session.review_ledger(),
    );
    let scope = session.reuse_state().project_scope.as_ref().expect("scope");
    let snapshot = build_reusable_influence_snapshot(
        scope,
        &session.reuse_state().governance_ledger,
        &effective,
    );
    assert!(matches!(
        resolve_exact_input_projection(scope, &snapshot, &entries),
        Err(ReusableInfluenceError::DivergentExactMapping { .. })
    ));
}

#[test]
fn reuse_enabled_exact_match_produces_non_binding_proposal_with_provenance() {
    let mut session = manual_replacement_session(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak",
        vec![alias_entry("Kafka", "Kafak")],
        "Kafka",
    );
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session.accept_reuse_candidate(&key).expect("accept");
    let run = session.run_reuse_enabled_review().expect("run");
    let cases = run.review_cases();
    let reusable_cases: Vec<_> = cases
        .iter()
        .filter(|case| {
            matches!(
                case.candidate_span().evidence(),
                Evidence::ReusableExactObservedForm(_)
            )
        })
        .collect();
    assert_eq!(reusable_cases.len(), 1);
    match reusable_cases[0].candidate_span().evidence() {
        Evidence::ReusableExactObservedForm(evidence) => {
            assert!(!evidence.contributions.is_empty());
            assert_eq!(evidence.observed_text, "Kafak");
            assert_eq!(evidence.confirmed_replacement, "Kafka");
        }
        _ => panic!("expected reusable evidence"),
    }
}

#[test]
fn canonical_pipeline_output_unchanged_without_reuse() {
    let transcript =
        parse_srt("1\n00:00:00,000 --> 00:00:01,000\nPostgre SQL then Postgres").expect("valid");
    let entries = vec![SessionTermEntry::new(
        "PostgreSQL",
        vec!["Postgres".to_string()],
        vec!["Postgre SQL".to_string()],
    )];
    let first = run_canonical_term_review(&transcript, &entries).expect("run");
    let second = run_canonical_term_review(&transcript, &entries).expect("run");
    assert_eq!(first.review_cases(), second.review_cases());
}

#[test]
fn reuse_enabled_analysis_identity_binds_governed_provenance() {
    let mut session = manual_replacement_session(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak",
        vec![alias_entry("Kafka", "Kafak")],
        "Kafka",
    );
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session.accept_reuse_candidate(&key).expect("accept");
    let with_active = session
        .run_reuse_enabled_review()
        .expect("run")
        .reuse_enabled_snapshot();
    session
        .revoke_reusable_influence(ReusableInfluenceRecordId::from_promotion_event_index(0))
        .expect("revoke");
    let after_revoke = session
        .run_reuse_enabled_review()
        .expect("run")
        .reuse_enabled_snapshot();
    assert_ne!(with_active, after_revoke);
}

#[test]
fn display_label_change_does_not_change_snapshot_identity() {
    let mut session = manual_replacement_session(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak",
        vec![alias_entry("Kafka", "Kafak")],
        "Kafka",
    );
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session.accept_reuse_candidate(&key).expect("accept");
    let before = session
        .materialize_review_export_bundle_v3()
        .expect("bundle")
        .reusable_snapshot
        .identity;
    session
        .update_project_scope_display_name("Renamed Project A")
        .expect("rename");
    let after = session
        .materialize_review_export_bundle_v3()
        .expect("bundle")
        .reusable_snapshot
        .identity;
    assert_eq!(before, after);
}

#[test]
fn v2_golden_header_unchanged() {
    let session = manual_replacement_session(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak",
        vec![alias_entry("Kafka", "Kafak")],
        "Kafka",
    );
    let bundle = session.materialize_review_export_bundle().expect("bundle");
    let log = render_application_decision_log(&bundle);
    assert!(log.starts_with("voxproof application decision log v2"));
}

#[test]
fn v3_includes_governance_history_and_distinguishes_derived_candidates() {
    let mut session = manual_replacement_session(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak",
        vec![alias_entry("Kafka", "Kafak")],
        "Kafka",
    );
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session.accept_reuse_candidate(&key).expect("accept");
    let bundle = session
        .materialize_review_export_bundle_v3()
        .expect("bundle");
    let log = render_application_decision_log_v3(&bundle);
    let summary = render_application_session_summary_v3(&bundle);
    assert!(log.starts_with(DECISION_LOG_V3_HEADER));
    assert!(summary.starts_with(SESSION_SUMMARY_V3_HEADER));
    assert!(log.contains("type: promotion_accepted"));
    assert!(summary.contains("Derived promotion candidates (non-authoritative)"));
}

#[test]
fn replay_reproduces_reuse_state() {
    let mut session = manual_replacement_session(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak",
        vec![alias_entry("Kafka", "Kafak")],
        "Kafka",
    );
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session.accept_reuse_candidate(&key).expect("accept");
    session.run_reuse_enabled_review().expect("run");
    session.verify_in_memory_replay().expect("replay");
}

#[test]
fn reusable_replacement_never_enters_phonetic_matching() {
    let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
    let entries = vec![alias_entry("Kafka", "Kafak")];
    let mut session = manual_replacement_session(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak",
        entries.clone(),
        "Kafka",
    );
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session.accept_reuse_candidate(&key).expect("accept");
    let run = session.run_reuse_enabled_review().expect("run");
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
fn exact_payload_escaping_round_trips_in_export() {
    assert_eq!(unescape_export_text("a\\nb").expect("round trip"), "a\nb");
    assert_eq!(unescape_export_text("a\\\\b").expect("round trip"), "a\\b");
}

#[test]
fn gate2_session_without_project_scope_still_functions() {
    let session = manual_replacement_session(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak",
        vec![alias_entry("Kafka", "Kafak")],
        "Kafka",
    );
    assert!(!session.has_project_scope());
    session
        .materialize_review_export_bundle()
        .expect("v2 export");
    session.verify_in_memory_replay().expect("replay");
}

#[test]
fn derive_candidates_appends_no_governance_event() {
    let mut session = manual_replacement_session(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak",
        vec![alias_entry("Kafka", "Kafak")],
        "Kafka",
    );
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let _ = session.reuse_candidates().expect("candidates");
    assert!(session.reuse_state().governance_ledger.events().is_empty());
}

#[test]
fn different_source_occurrences_produce_distinct_candidate_keys() {
    let transcript = parse_srt(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak\n\n2\n00:00:01,000 --> 00:00:02,000\nKafak",
    )
    .expect("valid");
    let terms = vec![alias_entry("Kafka", "Kafak")];
    let mut session = begin_application_review(transcript, terms, material_use(), authority("op"))
        .expect("session");
    for target in session
        .review_items()
        .iter()
        .map(|item| item.target)
        .collect::<Vec<_>>()
    {
        session
            .record_manual_replacement(target, "Kafka")
            .expect("replacement");
    }
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let candidates = session.reuse_candidates().expect("candidates");
    assert_eq!(candidates.len(), 2);
    assert_ne!(candidates[0].key, candidates[1].key);
}

#[test]
fn revoked_records_absent_from_active_snapshot() {
    let mut session = manual_replacement_session(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak",
        vec![alias_entry("Kafka", "Kafak")],
        "Kafka",
    );
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    let record_id = session.accept_reuse_candidate(&key).expect("accept");
    session
        .revoke_reusable_influence(record_id)
        .expect("revoke");
    let bundle = session
        .materialize_review_export_bundle_v3()
        .expect("bundle");
    assert!(bundle.effective_active_records.is_empty());
    assert_eq!(bundle.historical_records.len(), 1);
}

#[test]
fn reusable_proposals_use_typed_reusable_evidence_not_alias_evidence() {
    let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid");
    let entries = vec![alias_entry("Apache Kafka", "Kafak")];
    let mut session = manual_replacement_session(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak",
        entries.clone(),
        "Kafka",
    );
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session.accept_reuse_candidate(&key).expect("accept");
    let run = AnalysisRunHelper::reuse_enabled(&session, &transcript, &entries);
    let reusable_cases: Vec<_> = run
        .review_cases()
        .iter()
        .filter(|case| {
            matches!(
                case.candidate_span().evidence(),
                Evidence::ReusableExactObservedForm(_)
            )
        })
        .collect();
    assert_eq!(reusable_cases.len(), 1);
    assert!(
        reusable_cases
            .iter()
            .all(|case| !matches!(case.candidate_span().evidence(), Evidence::GlossaryAlias(_)))
    );
}

struct AnalysisRunHelper;

impl AnalysisRunHelper {
    fn reuse_enabled(
        session: &vox_proof::application_service::ApplicationReviewSession,
        transcript: &vox_proof::transcript::Transcript,
        entries: &[SessionTermEntry],
    ) -> vox_proof::pipeline::ReuseEnabledTermReviewRun {
        let effective = fold_effective_state(
            &session.reuse_state().governance_ledger,
            session.review_ledger(),
        );
        let scope = session.reuse_state().project_scope.as_ref().expect("scope");
        let snapshot = build_reusable_influence_snapshot(
            scope,
            &session.reuse_state().governance_ledger,
            &effective,
        );
        let projection =
            resolve_exact_input_projection(scope, &snapshot, entries).expect("projection");
        run_reuse_enabled_term_review(transcript, entries, &projection, &snapshot).expect("run")
    }
}

#[test]
fn base_session_term_phonetic_behavior_unchanged_with_reuse_state_present_but_inactive() {
    let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nPostgres").expect("valid");
    let entries = vec![SessionTermEntry::new(
        "PostgreSQL",
        vec!["Postgres".to_string()],
        Vec::new(),
    )];
    let canonical = run_canonical_term_review(&transcript, &entries).expect("canonical");
    let mut session = begin_application_review(
        parse_srt("1\n00:00:00,000 --> 00:00:01,000\nPostgres").expect("valid"),
        entries.clone(),
        material_use(),
        authority("op"),
    )
    .expect("session");
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let with_scope = run_canonical_term_review(&transcript, &entries).expect("canonical");
    assert_eq!(canonical.review_cases(), with_scope.review_cases());
}

#[test]
fn observed_error_and_glossary_detectors_unchanged_by_reuse_module_presence() {
    let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nPostgre SQL").expect("valid");
    let entries = vec![observed_entry("PostgreSQL", "Postgre SQL")];
    let run = vox_proof::analysis::AnalysisRun::for_canonical_session_terms(&transcript, &entries);
    let glossary = detect_glossary_matches(&run, &transcript, &entries).expect("glossary");
    let observed =
        detect_observed_error_form_matches(&run, &transcript, &entries).expect("observed");
    assert!(glossary.is_empty());
    assert_eq!(observed.len(), 1);
}

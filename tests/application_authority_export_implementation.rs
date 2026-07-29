use vox_proof::application_export::{
    render_application_decision_log, render_application_session_summary,
};
use vox_proof::application_service::{
    ApplicationDecisionCoverage, ApplicationExportPosture, ApplicationMaterialUseDeclaration,
    ApplicationResolutionStatus, ApplicationServiceError, DeclaredApplicationMaterialUseBasis,
    DeclaredSessionAuthority, DeclaredSessionAuthorityError, DeclaredSessionOperatorRole,
    begin_application_review,
};
use vox_proof::candidate::SessionTermEntry;
use vox_proof::review::CorrectionDecision;
use vox_proof::srt::parse_srt;

fn material_use() -> ApplicationMaterialUseDeclaration {
    ApplicationMaterialUseDeclaration::new(DeclaredApplicationMaterialUseBasis::SelfOwned)
}

fn alias_entry(canonical: &str, alias: &str) -> SessionTermEntry {
    SessionTermEntry::new(canonical, vec![alias.to_string()], Vec::new())
}

fn authority(
    role: DeclaredSessionOperatorRole,
    label: &str,
) -> Result<DeclaredSessionAuthority, DeclaredSessionAuthorityError> {
    DeclaredSessionAuthority::new(role, label)
}

fn one_case_session(label: &str) -> vox_proof::application_service::ApplicationReviewSession {
    let transcript =
        parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid fixture transcript");

    begin_application_review(
        transcript,
        vec![alias_entry("Kafka", "Kafak")],
        material_use(),
        authority(
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            label,
        )
        .expect("valid authority"),
    )
    .expect("application session")
}

fn accept_first(
    session: &mut vox_proof::application_service::ApplicationReviewSession,
) -> Result<(), ApplicationServiceError> {
    let target = session.review_items()[0].target;
    session.record_human_decision(
        target,
        CorrectionDecision::AcceptAlternative {
            alternative_index: 0,
        },
    )
}

#[test]
fn authority_accepts_valid_local_owner() {
    let authority = authority(
        DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
        "local-owner",
    )
    .expect("valid local owner");
    assert_eq!(
        authority.role(),
        DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator
    );
    assert_eq!(authority.display_label(), "local-owner");
}

#[test]
fn authority_accepts_valid_authorized_reviewer() {
    let authority = authority(
        DeclaredSessionOperatorRole::DeclaredAuthorizedHumanReviewer,
        "reviewer-1",
    )
    .expect("valid authorized reviewer");
    assert_eq!(
        authority.role(),
        DeclaredSessionOperatorRole::DeclaredAuthorizedHumanReviewer
    );
}

#[test]
fn authority_trims_ordinary_whitespace() {
    let authority = authority(
        DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
        "  trimmed-label  ",
    )
    .expect("trimmed authority");
    assert_eq!(authority.display_label(), "trimmed-label");
}

#[test]
fn authority_rejects_empty_label() {
    assert_eq!(
        authority(DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator, ""),
        Err(DeclaredSessionAuthorityError::EmptyDisplayLabel)
    );
}

#[test]
fn authority_rejects_whitespace_only_label() {
    assert_eq!(
        authority(
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "   "
        ),
        Err(DeclaredSessionAuthorityError::EmptyDisplayLabel)
    );
}

#[test]
fn authority_rejects_newline_in_label() {
    assert_eq!(
        authority(
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "a\nb"
        ),
        Err(DeclaredSessionAuthorityError::ControlCharacterInDisplayLabel)
    );
}

#[test]
fn authority_rejects_carriage_return_in_label() {
    assert_eq!(
        authority(
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "a\rb"
        ),
        Err(DeclaredSessionAuthorityError::ControlCharacterInDisplayLabel)
    );
}

#[test]
fn authority_rejects_tab_in_label() {
    assert_eq!(
        authority(
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "a\tb"
        ),
        Err(DeclaredSessionAuthorityError::ControlCharacterInDisplayLabel)
    );
}

#[test]
fn authority_rejects_nul_in_label() {
    assert_eq!(
        authority(
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "a\u{0000}b"
        ),
        Err(DeclaredSessionAuthorityError::ControlCharacterInDisplayLabel)
    );
}

#[test]
fn authority_rejects_other_unicode_control_in_label() {
    assert_eq!(
        authority(
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "a\u{0001}b"
        ),
        Err(DeclaredSessionAuthorityError::ControlCharacterInDisplayLabel)
    );
}

#[test]
fn authority_rejects_leading_control_before_trim() {
    assert_eq!(
        authority(
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "\u{0001} valid"
        ),
        Err(DeclaredSessionAuthorityError::ControlCharacterInDisplayLabel)
    );
}

#[test]
fn renderers_depend_only_on_export_bundle() {
    let mut session = one_case_session("bundle-only-operator");
    accept_first(&mut session).expect("accept");
    let bundle = session
        .materialize_review_export_bundle()
        .expect("export bundle");

    let decision_log = render_application_decision_log(&bundle);
    let session_summary = render_application_session_summary(&bundle);

    assert!(decision_log.contains("decision: accept_alternative"));
    assert!(session_summary.contains("bundle-only-operator"));
    assert_eq!(
        bundle.export_posture,
        ApplicationExportPosture::DeclaredOperatorUnauthenticatedInMemoryV0_2
    );
}

#[test]
fn decision_log_format_identity_and_disclaimer() {
    let mut session = one_case_session("format-operator");
    let target = session.review_items()[0].target;
    session
        .record_human_decision(target, CorrectionDecision::Reject)
        .expect("reject");
    let bundle = session
        .materialize_review_export_bundle()
        .expect("export bundle");
    let log = render_application_decision_log(&bundle);
    let lines: Vec<&str> = log.lines().collect();

    assert_eq!(lines[0], "voxproof application decision log v1");
    assert!(log.contains("Human-readable export"));
    assert!(log.contains("not machine re-import"));
    assert!(log.contains("not persistence"));
    assert!(log.contains("not authenticated identity"));
    assert!(log.contains("not legal authorization"));
    assert!(log.contains("not validation evidence by itself"));
}

#[test]
fn session_summary_format_identity_and_disclaimer() {
    let mut session = one_case_session("summary-operator");
    accept_first(&mut session).expect("accept");
    let bundle = session
        .materialize_review_export_bundle()
        .expect("export bundle");
    let summary = render_application_session_summary(&bundle);
    let lines: Vec<&str> = summary.lines().collect();

    assert_eq!(lines[0], "voxproof application session summary v1");
    assert!(summary.contains("Human-readable export"));
    assert!(summary.contains("not machine re-import"));
    assert!(summary.contains("not persistence"));
    assert!(summary.contains("not authenticated identity"));
    assert!(summary.contains("not legal authorization"));
    assert!(summary.contains("not validation evidence by itself"));
    assert!(summary.contains("decision_coverage: complete"));
    assert!(summary.contains("resolution_status: resolved"));
    assert!(summary.contains("analysis_snapshot:"));
    assert!(summary.contains("session_terms: session-terms:sha256-v1:"));
    assert!(!summary.contains("{"));
}

#[test]
fn session_summary_escapes_replacement_text_controls() {
    let transcript =
        parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid transcript");
    let session_terms = vec![SessionTermEntry::new(
        "line\nbreak",
        vec!["Kafak".to_string()],
        Vec::new(),
    )];
    let mut session = begin_application_review(
        transcript,
        session_terms,
        material_use(),
        authority(
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "escape-operator",
        )
        .expect("valid authority"),
    )
    .expect("application session");
    accept_first(&mut session).expect("accept");

    let summary = render_application_session_summary(
        &session
            .materialize_review_export_bundle()
            .expect("export bundle"),
    );

    assert!(summary.contains("line\\nbreak: 1"));
    assert!(!summary.contains("line\nbreak: 1"));
}

#[test]
fn receiving_session_attribution_uses_receiver_authority_not_target_minter() {
    let source = "1\n00:00:00,000 --> 00:00:01,000\nKafak";
    let session_a = begin_application_review(
        parse_srt(source).expect("first transcript"),
        vec![alias_entry("Kafka", "Kafak")],
        material_use(),
        authority(
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "session-a-operator",
        )
        .expect("valid authority"),
    )
    .expect("session a");
    let mut session_b = begin_application_review(
        parse_srt(source).expect("second transcript"),
        vec![alias_entry("Kafka", "Kafak")],
        material_use(),
        authority(
            DeclaredSessionOperatorRole::DeclaredAuthorizedHumanReviewer,
            "session-b-operator",
        )
        .expect("valid authority"),
    )
    .expect("session b");

    let borrowed_target = session_a.review_items()[0].target;
    session_b
        .record_human_decision(borrowed_target, CorrectionDecision::Reject)
        .expect("analysis-equivalent target accepted");

    let bundle = session_b
        .materialize_review_export_bundle()
        .expect("export bundle");
    assert_eq!(bundle.decision_records.len(), 1);
    assert_eq!(
        bundle.decision_records[0].session_authority.display_label(),
        "session-b-operator"
    );
    assert_ne!(
        bundle.decision_records[0].session_authority.display_label(),
        "session-a-operator"
    );
    assert_eq!(
        bundle.declared_session_authority.display_label(),
        "session-b-operator"
    );
}

#[test]
fn superseded_accept_does_not_inflate_effective_outcome_counts() {
    let mut session = one_case_session("supersede-operator");
    let target = session.review_items()[0].target;
    session
        .record_human_decision(
            target,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .expect("accept");
    session
        .record_human_decision(target, CorrectionDecision::Reject)
        .expect("supersede with reject");

    let bundle = session
        .materialize_review_export_bundle()
        .expect("export bundle");
    assert_eq!(
        bundle
            .session_summary
            .outcomes
            .accepted_replacements_materialized,
        0
    );
    assert_eq!(bundle.session_summary.outcomes.source_segments_affected, 0);
    assert!(bundle.session_summary.accepted_replacements.is_empty());
}

#[test]
fn later_non_accept_removes_prior_accepted_outcome() {
    for decision in [
        CorrectionDecision::Defer,
        CorrectionDecision::NeedsManualCorrection,
    ] {
        let mut session = one_case_session("outcome-operator");
        let target = session.review_items()[0].target;
        session
            .record_human_decision(
                target,
                CorrectionDecision::AcceptAlternative {
                    alternative_index: 0,
                },
            )
            .expect("accept");
        session
            .record_human_decision(target, decision)
            .expect("revise away from accept");

        let bundle = session
            .materialize_review_export_bundle()
            .expect("export bundle");
        assert_eq!(
            bundle
                .session_summary
                .outcomes
                .accepted_replacements_materialized,
            0,
            "decision {decision:?} should clear accepted outcome"
        );
    }
}

#[test]
fn multiple_accepts_in_one_segment_count_one_affected_segment() {
    let transcript =
        parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak and Kubes").expect("valid transcript");
    let mut session = begin_application_review(
        transcript,
        vec![
            alias_entry("Kubernetes", "Kubes"),
            alias_entry("Kafka", "Kafak"),
        ],
        material_use(),
        authority(
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "segment-operator",
        )
        .expect("valid authority"),
    )
    .expect("two-case session");
    let items = session.review_items();
    assert_eq!(items.len(), 2);
    assert_eq!(
        items[0]
            .review_case
            .candidate_span()
            .anchor()
            .segment_position(),
        items[1]
            .review_case
            .candidate_span()
            .anchor()
            .segment_position()
    );

    for item in &items {
        session
            .record_human_decision(
                item.target,
                CorrectionDecision::AcceptAlternative {
                    alternative_index: 0,
                },
            )
            .expect("accept");
    }

    let bundle = session
        .materialize_review_export_bundle()
        .expect("export bundle");
    assert_eq!(bundle.session_summary.outcomes.source_segments_affected, 1);
    assert_eq!(
        bundle
            .session_summary
            .outcomes
            .accepted_replacements_materialized,
        2
    );
}

#[test]
fn accepted_replacements_are_lexicographically_ordered() {
    let transcript = parse_srt(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak first\n\n\
         2\n00:00:01,000 --> 00:00:02,000\nKubes second",
    )
    .expect("valid transcript");
    let mut session = begin_application_review(
        transcript,
        vec![
            alias_entry("Kubernetes", "Kubes"),
            alias_entry("Kafka", "Kafak"),
        ],
        material_use(),
        authority(
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "ordering-operator",
        )
        .expect("valid authority"),
    )
    .expect("two-case session");

    for item in session.review_items() {
        session
            .record_human_decision(
                item.target,
                CorrectionDecision::AcceptAlternative {
                    alternative_index: 0,
                },
            )
            .expect("accept");
    }

    let replacements = session
        .materialize_review_export_bundle()
        .expect("export bundle")
        .session_summary
        .accepted_replacements;
    assert_eq!(replacements.len(), 2);
    assert_eq!(replacements[0].replacement_text, "Kafka");
    assert_eq!(replacements[1].replacement_text, "Kubernetes");
}

#[test]
fn detector_and_kind_counts_are_deterministically_ordered() {
    let transcript = parse_srt("1\n00:00:00,000 --> 00:00:01,000\nPostgre SQL then Postgres")
        .expect("valid transcript");
    let mut session = begin_application_review(
        transcript,
        vec![SessionTermEntry::new(
            "PostgreSQL",
            vec!["Postgres".to_string()],
            vec!["Postgre SQL".to_string()],
        )],
        material_use(),
        authority(
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "count-operator",
        )
        .expect("valid authority"),
    )
    .expect("multi-detector session");

    for item in session.review_items() {
        session
            .record_human_decision(item.target, CorrectionDecision::Reject)
            .expect("reject");
    }

    let bundle = session
        .materialize_review_export_bundle()
        .expect("export bundle");
    let kinds = bundle
        .session_summary
        .cases_by_detection_kind
        .iter()
        .map(|item| format!("{:?}", item.kind))
        .collect::<Vec<_>>();
    assert_eq!(kinds, vec!["GlossaryAliasMatch", "PhoneticSimilarity"]);

    let detectors = bundle
        .session_summary
        .cases_by_detector
        .iter()
        .map(|item| format!("{}@{}", item.detector_id, item.detector_version))
        .collect::<Vec<_>>();
    assert_eq!(
        detectors,
        vec![
            "ascii-latin-phonetic-similarity@0.1.0",
            "glossary-alias-match@0.1.0",
            "observed-error-form-match@0.1.0",
        ]
    );
}

#[test]
fn export_bundle_is_coverage_gated_like_reviewed_output() {
    let session = one_case_session("coverage-operator");
    assert_eq!(
        session.materialize_review_export_bundle(),
        Err(ApplicationServiceError::DecisionCoverageIncomplete { undecided: 1 })
    );
}

#[test]
fn complete_unresolved_bundle_retains_progress_and_reviewed_srt() {
    let mut session = one_case_session("unresolved-operator");
    let target = session.review_items()[0].target;
    session
        .record_human_decision(target, CorrectionDecision::Defer)
        .expect("defer");

    let output = session
        .materialize_reviewed_output()
        .expect("reviewed output");
    let bundle = session
        .materialize_review_export_bundle()
        .expect("export bundle");

    assert_eq!(bundle.reviewed_srt, output.srt);
    assert_eq!(
        bundle.progress.decision_coverage,
        ApplicationDecisionCoverage::Complete
    );
    assert!(matches!(
        bundle.progress.resolution_status,
        ApplicationResolutionStatus::Unresolved { .. }
    ));
}

#[test]
fn export_bundle_preserves_decision_event_order() {
    let transcript = parse_srt(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak first\n\n\
         2\n00:00:01,000 --> 00:00:02,000\nKubes second",
    )
    .expect("valid transcript");
    let mut session = begin_application_review(
        transcript,
        vec![
            alias_entry("Kubernetes", "Kubes"),
            alias_entry("Kafka", "Kafak"),
        ],
        material_use(),
        authority(
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "event-order-operator",
        )
        .expect("valid authority"),
    )
    .expect("two-case session");
    let items = session.review_items();

    session
        .record_human_decision(items[0].target, CorrectionDecision::Reject)
        .expect("reject first");
    session
        .record_human_decision(items[1].target, CorrectionDecision::Defer)
        .expect("defer second");
    session
        .record_human_decision(
            items[0].target,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .expect("revise first");

    let bundle = session
        .materialize_review_export_bundle()
        .expect("export bundle");
    assert_eq!(bundle.decision_records.len(), 3);
    assert_eq!(bundle.decision_records[0].event_index, 0);
    assert_eq!(bundle.decision_records[1].event_index, 1);
    assert_eq!(bundle.decision_records[2].event_index, 2);

    let log = render_application_decision_log(&bundle);
    assert!(log.contains("event 1\n"));
    assert!(log.contains("event 2\n"));
    assert!(log.contains("event 3\n"));
    assert!(log.contains("session_authority_label: event-order-operator\n"));
}

#[test]
fn replay_bundle_equality_holds_after_decisions() {
    let mut session = one_case_session("replay-operator");
    let target = session.review_items()[0].target;
    session
        .record_human_decision(target, CorrectionDecision::Reject)
        .expect("reject");
    assert_eq!(session.verify_in_memory_replay(), Ok(()));
}

#[test]
fn analysis_equivalent_bundles_render_byte_identically() {
    let source = "1\n00:00:00,000 --> 00:00:01,000\nKafak";
    let terms = vec![alias_entry("Kafka", "Kafak")];
    let material = material_use();
    let declared = authority(
        DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
        "byte-identical-operator",
    )
    .expect("valid authority");

    let mut session_a = begin_application_review(
        parse_srt(source).expect("transcript a"),
        terms.clone(),
        material,
        declared.clone(),
    )
    .expect("session a");
    let mut session_b = begin_application_review(
        parse_srt(source).expect("transcript b"),
        terms,
        material,
        declared,
    )
    .expect("session b");

    accept_first(&mut session_a).expect("accept a");
    accept_first(&mut session_b).expect("accept b");

    let bundle_a = session_a
        .materialize_review_export_bundle()
        .expect("bundle a");
    let bundle_b = session_b
        .materialize_review_export_bundle()
        .expect("bundle b");
    assert_eq!(bundle_a, bundle_b);

    assert_eq!(
        render_application_decision_log(&bundle_a),
        render_application_decision_log(&bundle_b)
    );
    assert_eq!(
        render_application_session_summary(&bundle_a),
        render_application_session_summary(&bundle_b)
    );
}

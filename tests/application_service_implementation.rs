use vox_proof::application_service::{
    ApplicationDecisionCoverage, ApplicationMaterialUseDeclaration, ApplicationResolutionStatus,
    ApplicationReviewProgress, ApplicationServiceError, DeclaredApplicationMaterialUseBasis,
    begin_application_review,
};
use vox_proof::candidate::SessionTermEntry;
use vox_proof::review::{CorrectionDecision, ReviewCaseStatus, ReviewLedgerError};
use vox_proof::reviewed_output::ReviewedOutputError;
use vox_proof::srt::parse_srt;

fn material_use() -> ApplicationMaterialUseDeclaration {
    ApplicationMaterialUseDeclaration::new(DeclaredApplicationMaterialUseBasis::SelfOwned)
}

fn alias_entry(canonical: &str, alias: &str) -> SessionTermEntry {
    SessionTermEntry::new(canonical, vec![alias.to_string()], Vec::new())
}

fn one_case_session() -> vox_proof::application_service::ApplicationReviewSession {
    let transcript =
        parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid fixture transcript");

    begin_application_review(
        transcript,
        vec![alias_entry("Kafka", "Kafak")],
        material_use(),
    )
    .expect("application session")
}

fn two_case_session() -> vox_proof::application_service::ApplicationReviewSession {
    let transcript = parse_srt(
        "1\n00:00:00,000 --> 00:00:01,000\nKafak first\n\n\
         2\n00:00:01,000 --> 00:00:02,000\nKubes second",
    )
    .expect("valid fixture transcript");

    begin_application_review(
        transcript,
        vec![
            alias_entry("Kubernetes", "Kubes"),
            alias_entry("Kafka", "Kafak"),
        ],
        material_use(),
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
fn zero_proposals_have_no_automatic_authority_and_replay_deterministically() {
    let transcript =
        parse_srt("1\n00:00:00,000 --> 00:00:01,000\nordinary text").expect("valid transcript");
    let session =
        begin_application_review(transcript, Vec::new(), material_use()).expect("empty session");

    assert!(session.review_items().is_empty());
    assert_eq!(
        session.progress(),
        ApplicationReviewProgress {
            decision_coverage: ApplicationDecisionCoverage::Complete,
            resolution_status: ApplicationResolutionStatus::Resolved,
        }
    );
    assert_eq!(session.decision_summary().total_recorded_events, 0);
    assert!(session.materialize_reviewed_output().is_ok());
    assert_eq!(session.verify_in_memory_replay(), Ok(()));
}

#[test]
fn canonical_proposal_order_is_preserved_and_source_remains_immutable() {
    let mut session = two_case_session();
    let revision_before = session.source().revision_id();
    let source_texts_before = session
        .source()
        .segments()
        .iter()
        .map(|segment| segment.text().to_string())
        .collect::<Vec<_>>();
    let items = session.review_items();

    assert_eq!(items.len(), 2);
    assert_eq!(items[0].review_case.id().local_index(), 0);
    assert_eq!(items[1].review_case.id().local_index(), 1);
    assert_eq!(
        session
            .source()
            .resolve(items[0].review_case.candidate_span().anchor()),
        Some("Kafak")
    );
    assert_eq!(
        session
            .source()
            .resolve(items[1].review_case.candidate_span().anchor()),
        Some("Kubes")
    );

    session
        .record_human_decision(items[0].target, CorrectionDecision::Reject)
        .expect("reject first");
    let _projection = session
        .derive_current_projection()
        .expect("current projection");

    assert_eq!(session.source().revision_id(), revision_before);
    assert_eq!(
        session
            .source()
            .segments()
            .iter()
            .map(|segment| segment.text().to_string())
            .collect::<Vec<_>>(),
        source_texts_before
    );
}

#[test]
fn different_analysis_target_is_rejected_before_mutation() {
    let mut receiving = one_case_session();
    let transcript =
        parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKubes").expect("valid other transcript");
    let other = begin_application_review(
        transcript,
        vec![alias_entry("Kubernetes", "Kubes")],
        material_use(),
    )
    .expect("other session");
    let other_target = other.review_items()[0].target;

    assert_eq!(
        receiving.record_human_decision(other_target, CorrectionDecision::Reject),
        Err(ApplicationServiceError::TargetAnalysisMismatch)
    );
    assert_eq!(receiving.decision_summary().total_recorded_events, 0);
}

#[test]
fn analysis_equivalent_target_resolves_identically() {
    let source = "1\n00:00:00,000 --> 00:00:01,000\nKafak";
    let first = begin_application_review(
        parse_srt(source).expect("first transcript"),
        vec![alias_entry("Kafka", "Kafak")],
        material_use(),
    )
    .expect("first session");
    let mut second = begin_application_review(
        parse_srt(source).expect("second transcript"),
        vec![alias_entry("Kafka", "Kafak")],
        material_use(),
    )
    .expect("second session");

    second
        .record_human_decision(first.review_items()[0].target, CorrectionDecision::Reject)
        .expect("analysis-equivalent target is admissible");

    assert_eq!(second.decision_summary().rejected, 1);
}

#[test]
fn review_case_clone_is_observation_only() {
    let mut session = one_case_session();
    let original_item = session.review_items().remove(0);
    assert_eq!(original_item.status, ReviewCaseStatus::Undecided);

    session
        .record_human_decision(original_item.target, CorrectionDecision::Reject)
        .expect("decision through target");

    assert_eq!(original_item.status, ReviewCaseStatus::Undecided);
    assert!(matches!(
        session.review_items()[0].status,
        ReviewCaseStatus::Decided {
            decision: CorrectionDecision::Reject,
            ..
        }
    ));
}

#[test]
fn invalid_alternative_appends_no_event() {
    let mut session = one_case_session();
    let item = session.review_items().remove(0);

    assert_eq!(
        session.record_human_decision(
            item.target,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 1,
            },
        ),
        Err(ApplicationServiceError::Decision(
            ReviewLedgerError::AlternativeIndexOutOfRange {
                case_id: item.review_case.id(),
                alternative_index: 1,
                alternative_count: 1,
            }
        ))
    );
    assert_eq!(session.decision_summary().total_recorded_events, 0);
    assert_eq!(
        session.review_items()[0].status,
        ReviewCaseStatus::Undecided
    );
}

#[test]
fn decision_revision_retains_history_and_last_event_controls_effective_state() {
    let mut session = one_case_session();
    let target = session.review_items()[0].target;

    session
        .record_human_decision(target, CorrectionDecision::Defer)
        .expect("defer");
    session
        .record_human_decision(target, CorrectionDecision::Reject)
        .expect("revised reject");

    let summary = session.decision_summary();
    assert_eq!(summary.total_recorded_events, 2);
    assert_eq!(summary.rejected, 1);
    assert_eq!(summary.deferred, 0);
    assert_eq!(
        session.progress(),
        ApplicationReviewProgress {
            decision_coverage: ApplicationDecisionCoverage::Complete,
            resolution_status: ApplicationResolutionStatus::Resolved,
        }
    );
    assert_eq!(session.verify_in_memory_replay(), Ok(()));
}

#[test]
fn mixed_effective_decisions_fold_into_complete_unresolved_progress_and_summary() {
    let mut session = two_case_session();
    let items = session.review_items();

    session
        .record_human_decision(
            items[0].target,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .expect("accept first");
    session
        .record_human_decision(items[1].target, CorrectionDecision::NeedsManualCorrection)
        .expect("manual second");

    let summary = session.decision_summary();
    assert_eq!(summary.total_review_cases, 2);
    assert_eq!(summary.total_recorded_events, 2);
    assert_eq!(
        summary.accepted_alternatives
            + summary.rejected
            + summary.deferred
            + summary.needs_manual_correction
            + summary.undecided,
        summary.total_review_cases
    );
    assert_eq!(
        session.progress(),
        ApplicationReviewProgress {
            decision_coverage: ApplicationDecisionCoverage::Complete,
            resolution_status: ApplicationResolutionStatus::Unresolved {
                deferred: 0,
                needs_manual_correction: 1,
            },
        }
    );
}

#[test]
fn undecided_cases_are_incomplete_but_may_be_resolved() {
    let session = one_case_session();

    assert_eq!(
        session.progress(),
        ApplicationReviewProgress {
            decision_coverage: ApplicationDecisionCoverage::Incomplete { undecided: 1 },
            resolution_status: ApplicationResolutionStatus::Resolved,
        }
    );
    assert!(session.derive_current_projection().is_ok());
    assert_eq!(
        session.materialize_reviewed_output(),
        Err(ApplicationServiceError::DecisionCoverageIncomplete { undecided: 1 })
    );
}

#[test]
fn accept_materializes_exact_alternative_and_complete_resolved_output() {
    let mut session = one_case_session();
    accept_first(&mut session).expect("accept");

    let output = session
        .materialize_reviewed_output()
        .expect("reviewed output");
    assert_eq!(output.srt, "1\n00:00:00,000 --> 00:00:01,000\nKafka\n");
    assert_eq!(
        output.progress,
        ApplicationReviewProgress {
            decision_coverage: ApplicationDecisionCoverage::Complete,
            resolution_status: ApplicationResolutionStatus::Resolved,
        }
    );
    assert_eq!(output.decision_summary.accepted_alternatives, 1);
}

#[test]
fn reject_defer_manual_and_undecided_preserve_source_text() {
    let expected = "1\n00:00:00,000 --> 00:00:01,000\nKafak\n";

    let undecided = one_case_session();
    assert_eq!(
        undecided
            .derive_current_projection()
            .expect("undecided projection")
            .srt,
        expected
    );

    for decision in [
        CorrectionDecision::Reject,
        CorrectionDecision::Defer,
        CorrectionDecision::NeedsManualCorrection,
    ] {
        let mut session = one_case_session();
        let target = session.review_items()[0].target;
        session
            .record_human_decision(target, decision)
            .expect("non-materializing decision");
        assert_eq!(
            session.derive_current_projection().expect("projection").srt,
            expected
        );
    }
}

#[test]
fn complete_unresolved_output_is_allowed_and_retains_context() {
    for decision in [
        CorrectionDecision::Defer,
        CorrectionDecision::NeedsManualCorrection,
    ] {
        let mut session = one_case_session();
        let target = session.review_items()[0].target;
        session
            .record_human_decision(target, decision)
            .expect("unresolved decision");

        let output = session
            .materialize_reviewed_output()
            .expect("coverage-complete unresolved output");
        assert_eq!(
            output.progress.decision_coverage,
            ApplicationDecisionCoverage::Complete
        );
        assert!(matches!(
            output.progress.resolution_status,
            ApplicationResolutionStatus::Unresolved { .. }
        ));
        assert_eq!(
            output.decision_summary.deferred + output.decision_summary.needs_manual_correction,
            1
        );
        assert_eq!(output.srt, "1\n00:00:00,000 --> 00:00:01,000\nKafak\n");
    }
}

#[test]
fn resolved_unresolved_and_incomplete_sessions_replay_without_mutation() {
    let mut resolved = one_case_session();
    resolved
        .record_human_decision(
            resolved.review_items()[0].target,
            CorrectionDecision::Reject,
        )
        .expect("reject");

    let mut unresolved = one_case_session();
    unresolved
        .record_human_decision(
            unresolved.review_items()[0].target,
            CorrectionDecision::Defer,
        )
        .expect("defer");

    let incomplete = one_case_session();

    for session in [&resolved, &unresolved, &incomplete] {
        let revision_before = session.source().revision_id();
        let summary_before = session.decision_summary();
        assert_eq!(session.verify_in_memory_replay(), Ok(()));
        assert_eq!(session.source().revision_id(), revision_before);
        assert_eq!(session.decision_summary(), summary_before);
    }
}

#[test]
fn production_module_has_no_evaluation_transport_persistence_or_gui_surface() {
    let source = include_str!("../src/application_service.rs");
    let forbidden = [
        "InputAuthorization",
        "RunEnvelope",
        "RealTranscriptEvaluationRunRequest",
        "RealTranscriptInitialExecutionBindings",
        "DetectorProposalSnapshot",
        "ReferenceSeal",
        "ReferenceCoverage",
        "HumanFinalReference",
        "artifact_packet",
        "persistence",
        "serde",
        "async fn",
        "unsafe",
    ];

    for term in forbidden {
        assert!(
            !source.contains(term),
            "production application service contains forbidden surface {term}"
        );
    }
    assert!(!source.contains("pub fn srt"));
}

#[test]
fn overlapping_accepts_preserve_typed_projection_error_and_replay_equality() {
    let transcript =
        parse_srt("1\n00:00:00,000 --> 00:00:01,000\nKafak").expect("valid transcript");
    let mut session = begin_application_review(
        transcript,
        vec![alias_entry("Kafka", "Kafak"), alias_entry("AFA", "afa")],
        material_use(),
    )
    .expect("overlap session");
    let items = session.review_items();
    assert_eq!(items.len(), 2, "fixture must produce two overlapping cases");

    for item in items {
        session
            .record_human_decision(
                item.target,
                CorrectionDecision::AcceptAlternative {
                    alternative_index: 0,
                },
            )
            .expect("valid accepted alternative");
    }

    assert!(matches!(
        session.derive_current_projection(),
        Err(ApplicationServiceError::ReviewedOutput(
            ReviewedOutputError::OverlappingEdits { .. }
        ))
    ));
    assert_eq!(session.verify_in_memory_replay(), Ok(()));
}

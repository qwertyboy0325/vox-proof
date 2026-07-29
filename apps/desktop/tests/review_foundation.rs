use std::fs;
use std::path::PathBuf;

use tempfile::tempdir;
use vox_proof::application_service::{
    ApplicationDecisionCoverage, ApplicationResolutionStatus, ApplicationServiceError,
    DeclaredApplicationMaterialUseBasis, DeclaredSessionOperatorRole,
};
use vox_proof::review::{CorrectionDecision, ManualReplacementTextError};
use vox_proof::reviewed_output::ReviewedOutputError;
use voxproof_desktop::controller::{ControllerError, DesktopController, DesktopPhase};

const SRT: &str = "1\n00:00:00,000 --> 00:00:01,000\n歡迎使用轉錄校對工具\n\n\
2\n00:00:01,000 --> 00:00:02,000\n這是華說的新產品\n\n\
3\n00:00:02,000 --> 00:00:03,000\n人工決定才是權威\n";
const TERMS: &str = "華碩 | alias:華說\n";
const TWO_CASE_SRT: &str = "1\n00:00:00,000 --> 00:00:01,000\n華說\n\n\
2\n00:00:01,000 --> 00:00:02,000\n台彎\n";
const TWO_CASE_TERMS: &str = "華碩 | alias:華說\n臺灣 | alias:台彎\n";
const OVERLAPPING_CASE_SRT: &str = "1\n00:00:00,000 --> 00:00:01,000\nKafaka\n";
const OVERLAPPING_CASE_TERMS: &str = "Kafka | alias:Kafak\nFACA | alias:faka\n";

fn start(controller: &mut DesktopController, source_path: PathBuf) {
    controller
        .start_from_text(
            SRT,
            TERMS,
            source_path,
            DeclaredApplicationMaterialUseBasis::SelfOwned,
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "本機審閱者",
        )
        .unwrap();
}

fn start_two_cases(controller: &mut DesktopController) {
    controller
        .start_from_text(
            TWO_CASE_SRT,
            TWO_CASE_TERMS,
            PathBuf::from("two.srt"),
            DeclaredApplicationMaterialUseBasis::SelfOwned,
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "Reviewer",
        )
        .unwrap();
}

#[test]
fn valid_setup_creates_exactly_one_active_session_and_selects_first_case() {
    let mut controller = DesktopController::default();
    start_two_cases(&mut controller);

    assert_eq!(controller.phase(), DesktopPhase::ActiveReview);
    assert_eq!(controller.items().unwrap().len(), 2);
    assert_eq!(controller.selected_index(), 0);
    let header = controller.header().unwrap();
    assert_eq!(header.total_review_cases, 2);
    assert_eq!(header.total_recorded_events, 0);
    assert_eq!(header.declared_operator, "Reviewer");
}

#[test]
fn invalid_inputs_and_authority_leave_setup_unchanged() {
    let cases = [
        ("not an srt", TERMS, "Reviewer"),
        (SRT, "華碩 | unsupported:華說", "Reviewer"),
        (SRT, TERMS, " \t "),
    ];

    for (srt, terms, operator) in cases {
        let mut controller = DesktopController::default();
        let generation = controller.generation();
        assert!(
            controller
                .start_from_text(
                    srt,
                    terms,
                    PathBuf::from("invalid.srt"),
                    DeclaredApplicationMaterialUseBasis::SelfOwned,
                    DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
                    operator,
                )
                .is_err()
        );
        assert_eq!(controller.phase(), DesktopPhase::Setup);
        assert_eq!(controller.generation(), generation);
        assert!(controller.exported_paths().is_none());
    }
}

#[test]
fn accept_uses_current_generation_and_recomputes_read_only_projection() {
    let mut controller = DesktopController::default();
    start(&mut controller, PathBuf::from("sample.srt"));
    let generation = controller.generation();

    controller
        .record_decision(
            generation,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .unwrap();

    assert!(
        controller
            .projection()
            .unwrap()
            .srt
            .contains("這是華碩的新產品")
    );
    assert_eq!(
        controller.progress().unwrap().decision_coverage,
        ApplicationDecisionCoverage::Complete
    );
}

#[test]
fn manual_replacement_flows_through_controller_and_preserves_exact_text() {
    let mut controller = DesktopController::default();
    start(&mut controller, PathBuf::from("manual.srt"));
    let generation = controller.generation();

    controller
        .record_manual_replacement(generation, "  華碩正式版  ")
        .expect("manual replacement");

    assert!(
        controller
            .projection()
            .unwrap()
            .srt
            .contains("這是  華碩正式版  的新產品")
    );
    let header = controller.header().unwrap();
    assert_eq!(header.manual_replacements, 1);
    assert_eq!(header.total_recorded_events, 1);
    assert_eq!(
        controller.progress().unwrap().resolution_status,
        ApplicationResolutionStatus::Resolved
    );
}

#[test]
fn invalid_manual_replacement_does_not_change_authoritative_session() {
    let mut controller = DesktopController::default();
    start(&mut controller, PathBuf::from("manual-invalid.srt"));
    let generation = controller.generation();

    let error = controller
        .record_manual_replacement(generation, " \u{3000}")
        .expect_err("whitespace-only replacement must fail");
    assert!(matches!(
        error,
        ControllerError::Service(ApplicationServiceError::ManualReplacement(
            ManualReplacementTextError::WhitespaceOnly
        ))
    ));
    assert_eq!(controller.header().unwrap().total_recorded_events, 0);
    assert_eq!(
        controller.progress().unwrap().decision_coverage,
        ApplicationDecisionCoverage::Incomplete { undecided: 1 }
    );
    assert!(
        controller
            .projection()
            .unwrap()
            .srt
            .contains("這是華說的新產品")
    );
}

#[test]
fn decision_revision_appends_an_event_and_changes_effective_status() {
    let mut controller = DesktopController::default();
    start(&mut controller, PathBuf::from("revision.srt"));
    let generation = controller.generation();

    controller
        .record_decision(
            generation,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .unwrap();
    controller
        .record_decision(generation, CorrectionDecision::Reject)
        .unwrap();

    let header = controller.header().unwrap();
    assert_eq!(header.total_recorded_events, 2);
    assert_eq!(header.accepted, 0);
    assert_eq!(header.rejected, 1);
    let item = controller.items().unwrap().remove(0);
    assert_eq!(item.status, "Rejected");
    assert_eq!(item.alternatives, vec!["華碩"]);
    assert!(item.evidence.contains("Glossary alias"));
}

#[test]
fn decision_advances_to_next_undecided_then_retains_current_when_complete() {
    let mut controller = DesktopController::default();
    start_two_cases(&mut controller);
    let generation = controller.generation();

    controller
        .record_decision(generation, CorrectionDecision::Reject)
        .unwrap();
    assert_eq!(controller.selected_index(), 1);
    controller
        .record_decision(generation, CorrectionDecision::NeedsManualCorrection)
        .unwrap();
    assert_eq!(controller.selected_index(), 1);
    assert_eq!(
        controller.progress().unwrap().decision_coverage,
        ApplicationDecisionCoverage::Complete
    );
}

#[test]
fn reset_replaces_session_and_stale_generation_fails_closed() {
    let mut controller = DesktopController::default();
    start(&mut controller, PathBuf::from("first.srt"));
    let stale_generation = controller.generation();
    controller.reset();
    start(&mut controller, PathBuf::from("second.srt"));

    let error = controller
        .record_decision(stale_generation, CorrectionDecision::Reject)
        .unwrap_err();

    assert!(matches!(error, ControllerError::StaleGeneration { .. }));
    assert_eq!(
        controller.progress().unwrap().decision_coverage,
        ApplicationDecisionCoverage::Incomplete { undecided: 1 }
    );
}

#[test]
fn old_selection_and_generation_cannot_mutate_a_replacement_session() {
    let mut controller = DesktopController::default();
    start_two_cases(&mut controller);
    controller.select(1);
    let stale_generation = controller.generation();

    start(&mut controller, PathBuf::from("replacement.srt"));
    assert_eq!(controller.selected_index(), 0);
    let error = controller
        .record_decision(stale_generation, CorrectionDecision::Reject)
        .unwrap_err();

    assert!(matches!(error, ControllerError::StaleGeneration { .. }));
    assert_eq!(controller.header().unwrap().total_recorded_events, 0);
    assert_eq!(controller.items().unwrap()[0].status, "Undecided");
}

#[test]
fn reset_clears_projection_and_export_state_and_changes_generation() {
    let directory = tempdir().unwrap();
    let mut controller = DesktopController::default();
    start(&mut controller, PathBuf::from("reset.srt"));
    controller
        .record_decision(controller.generation(), CorrectionDecision::Reject)
        .unwrap();
    controller
        .export(controller.generation(), directory.path(), false)
        .unwrap();
    let generation = controller.generation();

    controller.reset();

    assert_eq!(controller.phase(), DesktopPhase::Setup);
    assert_ne!(controller.generation(), generation);
    assert!(controller.exported_paths().is_none());
    assert!(matches!(
        controller.projection(),
        Err(ControllerError::NoActiveSession)
    ));
    assert!(matches!(
        controller.export_previews(false),
        Err(ControllerError::NoActiveSession)
    ));
}

#[test]
fn every_foundation_decision_is_recorded_through_the_application_session() {
    let decisions = [
        CorrectionDecision::AcceptAlternative {
            alternative_index: 0,
        },
        CorrectionDecision::Reject,
        CorrectionDecision::Defer,
        CorrectionDecision::NeedsManualCorrection,
    ];

    for decision in decisions {
        let mut controller = DesktopController::default();
        start(&mut controller, PathBuf::from("decision.srt"));
        controller
            .record_decision(controller.generation(), decision)
            .unwrap();
        assert_eq!(
            controller.progress().unwrap().decision_coverage,
            ApplicationDecisionCoverage::Complete
        );
    }
}

#[test]
fn complete_unresolved_export_requires_explicit_source_retention_confirmation() {
    let directory = tempdir().unwrap();
    let mut controller = DesktopController::default();
    start(&mut controller, PathBuf::from("訪談.srt"));
    controller
        .record_decision(controller.generation(), CorrectionDecision::Defer)
        .unwrap();

    assert!(matches!(
        controller.progress().unwrap().resolution_status,
        ApplicationResolutionStatus::Unresolved { deferred: 1, .. }
    ));
    assert!(matches!(
        controller.export(controller.generation(), directory.path(), false),
        Err(ControllerError::UnresolvedConfirmationRequired)
    ));

    let paths = controller
        .export(controller.generation(), directory.path(), true)
        .unwrap();
    assert_eq!(controller.phase(), DesktopPhase::ExportCompleted);
    assert_eq!(
        paths.reviewed_srt.file_name().unwrap(),
        "訪談.voxproof-reviewed.srt"
    );
    assert!(
        fs::read_to_string(paths.reviewed_srt)
            .unwrap()
            .contains("華說")
    );
    assert!(paths.decision_log.is_file());
    assert!(paths.session_summary.is_file());
}

#[test]
fn successful_post_export_revision_invalidates_only_the_in_app_export_state() {
    let directory = tempdir().unwrap();
    let mut controller = DesktopController::default();
    start(&mut controller, PathBuf::from("revision.srt"));
    let generation = controller.generation();
    controller
        .record_decision(
            generation,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .unwrap();
    let paths = controller
        .export(generation, directory.path(), false)
        .unwrap();
    let original_reviewed_srt = fs::read(&paths.reviewed_srt).unwrap();
    let original_decision_log = fs::read(&paths.decision_log).unwrap();
    let original_session_summary = fs::read(&paths.session_summary).unwrap();
    assert_eq!(controller.phase(), DesktopPhase::ExportCompleted);

    controller
        .record_decision(generation, CorrectionDecision::Reject)
        .unwrap();

    assert_eq!(controller.phase(), DesktopPhase::ActiveReview);
    assert!(controller.exported_paths().is_none());
    assert_eq!(controller.header().unwrap().total_recorded_events, 2);
    let projection = controller.projection().unwrap();
    assert!(projection.srt.contains("這是華說的新產品"));
    assert!(!projection.srt.contains("這是華碩的新產品"));
    assert_eq!(
        fs::read(&paths.reviewed_srt).unwrap(),
        original_reviewed_srt
    );
    assert_eq!(
        fs::read(&paths.decision_log).unwrap(),
        original_decision_log
    );
    assert_eq!(
        fs::read(&paths.session_summary).unwrap(),
        original_session_summary
    );

    assert!(matches!(
        controller.export(generation, directory.path(), false),
        Err(ControllerError::Export(_))
    ));
    assert!(controller.exported_paths().is_none());
    assert_eq!(
        fs::read(&paths.reviewed_srt).unwrap(),
        original_reviewed_srt
    );
    assert_eq!(
        fs::read(&paths.decision_log).unwrap(),
        original_decision_log
    );
    assert_eq!(
        fs::read(&paths.session_summary).unwrap(),
        original_session_summary
    );
}

#[test]
fn successful_post_export_manual_replacement_invalidates_export_completion_state() {
    let directory = tempdir().unwrap();
    let mut controller = DesktopController::default();
    start(&mut controller, PathBuf::from("manual-revision.srt"));
    let generation = controller.generation();
    controller
        .record_decision(generation, CorrectionDecision::Reject)
        .unwrap();
    let paths = controller
        .export(generation, directory.path(), false)
        .unwrap();
    let original_decision_log = fs::read(&paths.decision_log).unwrap();
    assert_eq!(controller.phase(), DesktopPhase::ExportCompleted);

    controller
        .record_manual_replacement(generation, "華碩手動版")
        .expect("manual revision");

    assert_eq!(controller.phase(), DesktopPhase::ActiveReview);
    assert!(controller.exported_paths().is_none());
    assert_eq!(controller.header().unwrap().total_recorded_events, 2);
    assert_eq!(controller.header().unwrap().manual_replacements, 1);
    assert!(
        controller
            .projection()
            .unwrap()
            .srt
            .contains("這是華碩手動版的新產品")
    );
    assert_eq!(
        fs::read(&paths.decision_log).unwrap(),
        original_decision_log
    );
}

#[test]
fn failed_stale_post_export_decision_preserves_export_completion_state() {
    let directory = tempdir().unwrap();
    let mut controller = DesktopController::default();
    start(&mut controller, PathBuf::from("stale.srt"));
    let generation = controller.generation();
    controller
        .record_decision(generation, CorrectionDecision::Reject)
        .unwrap();
    let paths = controller
        .export(generation, directory.path(), false)
        .unwrap();
    let original_reviewed_srt = fs::read(&paths.reviewed_srt).unwrap();
    let original_decision_log = fs::read(&paths.decision_log).unwrap();
    let original_session_summary = fs::read(&paths.session_summary).unwrap();

    let error = controller
        .record_decision(generation.wrapping_sub(1), CorrectionDecision::Defer)
        .unwrap_err();

    assert!(matches!(error, ControllerError::StaleGeneration { .. }));
    assert_eq!(controller.phase(), DesktopPhase::ExportCompleted);
    assert_eq!(controller.exported_paths(), Some(&paths));
    assert_eq!(controller.header().unwrap().total_recorded_events, 1);
    assert_eq!(
        fs::read(&paths.reviewed_srt).unwrap(),
        original_reviewed_srt
    );
    assert_eq!(
        fs::read(&paths.decision_log).unwrap(),
        original_decision_log
    );
    assert_eq!(
        fs::read(&paths.session_summary).unwrap(),
        original_session_summary
    );
}

#[test]
fn collision_preflight_writes_none_of_the_other_outputs() {
    let directory = tempdir().unwrap();
    let mut controller = DesktopController::default();
    start(&mut controller, PathBuf::from("transcript.srt"));
    controller
        .record_decision(controller.generation(), CorrectionDecision::Reject)
        .unwrap();
    let collision = directory.path().join("transcript.voxproof-decisions.txt");
    fs::write(&collision, "keep me").unwrap();

    let error = controller
        .export(controller.generation(), directory.path(), false)
        .unwrap_err();

    assert!(matches!(error, ControllerError::Export(_)));
    assert_eq!(fs::read_to_string(collision).unwrap(), "keep me");
    assert!(
        !directory
            .path()
            .join("transcript.voxproof-reviewed.srt")
            .exists()
    );
    assert!(
        !directory
            .path()
            .join("transcript.voxproof-session-summary.txt")
            .exists()
    );
}

#[test]
fn incomplete_coverage_rejects_final_projection_and_filesystem_export() {
    let directory = tempdir().unwrap();
    let mut controller = DesktopController::default();
    start(&mut controller, PathBuf::from("incomplete.srt"));

    assert!(matches!(
        controller.export_previews(false),
        Err(ControllerError::ReviewIncomplete { undecided: 1 })
    ));
    assert!(matches!(
        controller.export(controller.generation(), directory.path(), false),
        Err(ControllerError::ReviewIncomplete { undecided: 1 })
    ));
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[test]
fn overlapping_manual_and_alternative_decisions_succeed_but_projection_and_export_fail_closed() {
    let directory = tempdir().unwrap();
    let mut controller = DesktopController::default();
    controller
        .start_from_text(
            OVERLAPPING_CASE_SRT,
            OVERLAPPING_CASE_TERMS,
            PathBuf::from("overlap.srt"),
            DeclaredApplicationMaterialUseBasis::SelfOwned,
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "Reviewer",
        )
        .unwrap();

    let generation = controller.generation();
    controller
        .record_manual_replacement(generation, "Manual Kafka")
        .expect("authoritative manual decision succeeds");
    assert_eq!(controller.header().unwrap().total_recorded_events, 1);
    controller
        .record_decision(
            generation,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .expect("authoritative alternative decision succeeds");

    assert_eq!(controller.header().unwrap().total_recorded_events, 2);
    assert!(matches!(
        controller.projection(),
        Err(ControllerError::Service(
            ApplicationServiceError::ReviewedOutput(ReviewedOutputError::OverlappingEdits { .. })
        ))
    ));
    controller
        .record_decision(generation, CorrectionDecision::Reject)
        .expect("remaining non-materializing decision succeeds");
    assert_eq!(controller.header().unwrap().total_recorded_events, 3);
    let export_result = controller.export(generation, directory.path(), false);
    assert!(
        matches!(
            export_result,
            Err(ControllerError::Service(
                ApplicationServiceError::ReviewedOutput(
                    ReviewedOutputError::OverlappingEdits { .. }
                )
            ))
        ),
        "unexpected export result: {export_result:?}"
    );
    assert!(controller.exported_paths().is_none());
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[test]
fn complete_resolved_session_materializes_rendered_projections() {
    let mut controller = DesktopController::default();
    start(&mut controller, PathBuf::from("resolved.srt"));
    controller
        .record_decision(
            controller.generation(),
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .unwrap();

    let (log, summary) = controller.export_previews(false).unwrap();
    assert!(
        controller
            .projection()
            .unwrap()
            .srt
            .contains("這是華碩的新產品")
    );
    assert!(log.contains("accept_alternative"));
    assert!(summary.contains("coverage: complete"));
    assert_eq!(
        controller.progress().unwrap().resolution_status,
        ApplicationResolutionStatus::Resolved
    );
}

#[test]
fn zero_case_session_is_exportable_without_unresolved_confirmation() {
    let directory = tempdir().unwrap();
    let mut controller = DesktopController::default();
    controller
        .start_from_text(
            SRT,
            "term-not-present",
            PathBuf::from("quiet.srt"),
            DeclaredApplicationMaterialUseBasis::ExplicitPermission,
            DeclaredSessionOperatorRole::DeclaredAuthorizedHumanReviewer,
            "Reviewer",
        )
        .unwrap();

    assert!(controller.items().unwrap().is_empty());
    assert_eq!(
        controller.progress().unwrap().decision_coverage,
        ApplicationDecisionCoverage::Complete
    );
    controller
        .export(controller.generation(), directory.path(), false)
        .unwrap();
}

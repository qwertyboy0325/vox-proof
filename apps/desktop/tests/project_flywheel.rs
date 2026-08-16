use std::fs;
use std::path::PathBuf;

use tempfile::tempdir;
use vox_proof::application_service::{
    ApplicationDecisionCoverage, DeclaredApplicationMaterialUseBasis, DeclaredSessionOperatorRole,
};
use vox_proof::review::CorrectionDecision;
use vox_proof::session_persistence::ProductSessionStore;
use voxproof_desktop::controller::{
    ControllerError, DesktopController, DesktopPhase, ReviewItemOrigin,
};
use voxproof_desktop::presentation::{
    composed_coverage_label, export_enabled, show_review_complete_panel,
};

const A_SRT: &str = "1\n00:00:00,000 --> 00:00:01,000\nKafak\n";
const B_SRT: &str = "1\n00:00:00,000 --> 00:00:01,000\nKafak\n";
const EMPTY_TERMS: &str = "";
const KAFKA_TERMS: &str = "Kafka | alias:Kafak\n";
const CAFKA_TERMS: &str = "Cafka | alias:Kafak\n";

fn controller_with_store() -> (DesktopController, tempfile::TempDir) {
    let temp = tempdir().unwrap();
    let store = ProductSessionStore::new(temp.path());
    (DesktopController::with_store(store), temp)
}

fn start_unbound(controller: &mut DesktopController, srt: &str, terms: &str, name: &str) {
    controller
        .start_from_text(
            srt,
            terms,
            Some(PathBuf::from(name)),
            DeclaredApplicationMaterialUseBasis::SelfOwned,
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "Reviewer",
        )
        .unwrap();
}

fn start_bound(
    controller: &mut DesktopController,
    srt: &str,
    terms: &str,
    name: &str,
    project_id: &vox_proof::reuse_primitives::ProjectScopeId,
) {
    controller
        .start_from_text_in_project(
            srt,
            terms,
            Some(PathBuf::from(name)),
            DeclaredApplicationMaterialUseBasis::SelfOwned,
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "Reviewer",
            project_id,
        )
        .unwrap();
}

fn promote_selected(controller: &mut DesktopController) {
    let epoch = controller.ui_session_epoch();
    controller
        .record_manual_replacement(epoch, "Kafka")
        .unwrap();
    controller
        .use_selected_correction_in_related_reviews(epoch)
        .unwrap();
}

#[test]
fn new_project_creates_v2_bound_session_without_uuid_entry() {
    let (mut controller, _temp) = controller_with_store();
    let project_id = controller.create_project("Lecture series").unwrap();
    start_bound(&mut controller, A_SRT, KAFKA_TERMS, "a.srt", &project_id);
    assert_eq!(controller.phase(), DesktopPhase::ActiveReview);
    assert!(controller.is_bound_to_project());
    assert!(controller.project_scope_id_draft().is_empty());
    assert_eq!(
        controller.header().unwrap().project_name.as_deref(),
        Some("Lecture series")
    );
}

#[test]
fn existing_project_creates_another_v2_session() {
    let (mut controller, _temp) = controller_with_store();
    let project_id = controller.create_project("Lecture series").unwrap();
    start_bound(&mut controller, A_SRT, KAFKA_TERMS, "a.srt", &project_id);
    controller.reset().unwrap();
    assert!(
        controller
            .available_projects()
            .iter()
            .any(|project| project.display_name == "Lecture series")
    );
    start_bound(&mut controller, B_SRT, EMPTY_TERMS, "b.srt", &project_id);
    assert!(controller.is_bound_to_project());
    assert_eq!(
        controller.header().unwrap().project_name.as_deref(),
        Some("Lecture series")
    );
}

#[test]
fn material_a_promotion_appears_in_project_memory() {
    let (mut controller, _temp) = controller_with_store();
    let project_id = controller.create_project("Lecture series").unwrap();
    start_bound(&mut controller, A_SRT, KAFKA_TERMS, "a.srt", &project_id);
    promote_selected(&mut controller);
    let entries = controller.project_memory_entries().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].observed_text, "Kafak");
    assert_eq!(entries[0].confirmed_replacement, "Kafka");
    assert!(entries[0].provenance_available);
}

#[test]
fn material_b_reuse_proposal_visible_and_does_not_change_output_until_accept() {
    let (mut controller, _temp) = controller_with_store();
    let project_id = controller.create_project("Lecture series").unwrap();
    start_bound(&mut controller, A_SRT, KAFKA_TERMS, "a.srt", &project_id);
    promote_selected(&mut controller);
    controller.reset().unwrap();

    start_bound(&mut controller, B_SRT, EMPTY_TERMS, "b.srt", &project_id);
    let items = controller.items().unwrap();
    assert_eq!(items.len(), 1);
    assert!(items[0].uses_reuse_proposal_target);
    assert!(matches!(
        items[0].origin,
        ReviewItemOrigin::PreviousCorrection {
            conflict_with_canonical: false
        }
    ));
    assert!(controller.projection().unwrap().srt.contains("Kafak"));
    assert!(!controller.projection().unwrap().srt.contains("Kafka"));

    let epoch = controller.ui_session_epoch();
    controller
        .record_decision(
            epoch,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .unwrap();
    assert!(controller.projection().unwrap().srt.contains("Kafka"));
}

#[test]
fn reject_preserves_source_and_edit_uses_human_text() {
    let (mut controller, _temp) = controller_with_store();
    let project_id = controller.create_project("Lecture series").unwrap();
    start_bound(&mut controller, A_SRT, KAFKA_TERMS, "a.srt", &project_id);
    promote_selected(&mut controller);
    controller.reset().unwrap();

    start_bound(
        &mut controller,
        B_SRT,
        EMPTY_TERMS,
        "b-reject.srt",
        &project_id,
    );
    let epoch = controller.ui_session_epoch();
    controller
        .record_decision(epoch, CorrectionDecision::Reject)
        .unwrap();
    assert!(controller.projection().unwrap().srt.contains("Kafak"));
    controller.reset().unwrap();

    start_bound(
        &mut controller,
        B_SRT,
        EMPTY_TERMS,
        "b-edit.srt",
        &project_id,
    );
    let epoch = controller.ui_session_epoch();
    controller
        .record_manual_replacement(epoch, "KRaft")
        .unwrap();
    assert!(controller.projection().unwrap().srt.contains("KRaft"));
    let entries = controller.project_memory_entries().unwrap();
    assert_eq!(entries[0].confirmed_replacement, "Kafka");
}

#[test]
fn close_reopen_b_keeps_accepted_output() {
    let (mut controller, _temp) = controller_with_store();
    let project_id = controller.create_project("Lecture series").unwrap();
    start_bound(&mut controller, A_SRT, KAFKA_TERMS, "a.srt", &project_id);
    promote_selected(&mut controller);
    controller.reset().unwrap();

    start_bound(&mut controller, B_SRT, EMPTY_TERMS, "b.srt", &project_id);
    let session_id = controller.session_id().unwrap().to_owned();
    let epoch = controller.ui_session_epoch();
    controller
        .record_decision(
            epoch,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .unwrap();
    controller.reset().unwrap();
    controller.select_resume_session_id(session_id);
    controller.open_selected_session_writable().unwrap();
    assert!(controller.projection().unwrap().srt.contains("Kafka"));
    assert!(controller.items().unwrap()[0].uses_reuse_proposal_target);
}

#[test]
fn missing_project_keeps_review_readable_and_blocks_new_reuse() {
    let (mut controller, temp) = controller_with_store();
    let project_id = controller.create_project("Lecture series").unwrap();
    start_bound(&mut controller, A_SRT, KAFKA_TERMS, "a.srt", &project_id);
    promote_selected(&mut controller);
    controller.reset().unwrap();

    start_bound(&mut controller, B_SRT, EMPTY_TERMS, "b.srt", &project_id);
    let session_id = controller.session_id().unwrap().to_owned();
    let epoch = controller.ui_session_epoch();
    controller
        .record_decision(
            epoch,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .unwrap();
    controller.reset().unwrap();

    let project_dir = temp.path().join("projects").join(project_id.as_str());
    fs::remove_dir_all(project_dir).unwrap();

    controller.select_resume_session_id(session_id.clone());
    controller.open_selected_session_read_only().unwrap();
    assert!(controller.projection().unwrap().srt.contains("Kafka"));
    assert!(!controller.project_memory_available());
    assert!(controller.header().unwrap().bound_to_project);
    controller.reset().unwrap();

    controller.select_resume_session_id(session_id);
    controller.open_selected_session_writable().unwrap();
    let epoch = controller.ui_session_epoch();
    let err = controller.record_decision(epoch, CorrectionDecision::Reject);
    assert!(matches!(
        err,
        Err(ControllerError::WritableReuseBlocked | ControllerError::ProjectMemoryUnavailable)
    ));
}

#[test]
fn same_y_collision_is_one_canonical_card() {
    let (mut controller, _temp) = controller_with_store();
    let project_id = controller.create_project("Lecture series").unwrap();
    start_bound(&mut controller, A_SRT, KAFKA_TERMS, "a.srt", &project_id);
    promote_selected(&mut controller);
    controller.reset().unwrap();

    start_bound(&mut controller, B_SRT, KAFKA_TERMS, "b.srt", &project_id);
    let items = controller.items().unwrap();
    assert_eq!(items.len(), 1);
    assert!(!items[0].uses_reuse_proposal_target);
    assert!(matches!(
        items[0].origin,
        ReviewItemOrigin::TermSuggestion {
            also_supported_by_previous_correction: true,
            disagrees_with_previous_correction: false,
        }
    ));
}

#[test]
fn different_y_collision_shows_both_without_auto_winner() {
    let (mut controller, _temp) = controller_with_store();
    let project_id = controller.create_project("Lecture series").unwrap();
    start_bound(&mut controller, A_SRT, KAFKA_TERMS, "a.srt", &project_id);
    promote_selected(&mut controller);
    controller.reset().unwrap();

    start_bound(&mut controller, B_SRT, CAFKA_TERMS, "b.srt", &project_id);
    let items = controller.items().unwrap();
    assert_eq!(items.len(), 2);
    assert!(items.iter().any(|item| !item.uses_reuse_proposal_target));
    assert!(items.iter().any(|item| item.uses_reuse_proposal_target));
    let reuse = items
        .iter()
        .find(|item| item.uses_reuse_proposal_target)
        .unwrap();
    assert!(matches!(
        reuse.origin,
        ReviewItemOrigin::PreviousCorrection {
            conflict_with_canonical: true
        }
    ));
    assert!(controller.projection().unwrap().srt.contains("Kafak"));
}

#[test]
fn v1_unbound_desktop_path_remains_available() {
    let (mut controller, _temp) = controller_with_store();
    start_unbound(&mut controller, A_SRT, KAFKA_TERMS, "unbound.srt");
    assert!(!controller.is_bound_to_project());
    assert_eq!(controller.phase(), DesktopPhase::ActiveReview);
    let epoch = controller.ui_session_epoch();
    controller
        .record_decision(
            epoch,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .unwrap();
    assert!(controller.projection().unwrap().srt.contains("Kafka"));
}

#[test]
fn recent_reviews_carry_project_display_name() {
    let (mut controller, _temp) = controller_with_store();
    let project_id = controller.create_project("Lecture series").unwrap();
    start_bound(&mut controller, A_SRT, KAFKA_TERMS, "a.srt", &project_id);
    controller.reset().unwrap();
    let resume = controller
        .available_sessions()
        .iter()
        .find(|session| session.source_display_name == "a.srt")
        .unwrap();
    assert_eq!(
        resume.project_display_name.as_deref(),
        Some("Lecture series")
    );
}

#[test]
fn reuse_decision_uses_project_reuse_target() {
    let (mut controller, _temp) = controller_with_store();
    let project_id = controller.create_project("Lecture series").unwrap();
    start_bound(&mut controller, A_SRT, KAFKA_TERMS, "a.srt", &project_id);
    promote_selected(&mut controller);
    controller.reset().unwrap();
    start_bound(&mut controller, B_SRT, EMPTY_TERMS, "b.srt", &project_id);
    let items = controller.items().unwrap();
    assert_eq!(items.len(), 1);
    assert!(items[0].uses_reuse_proposal_target);
}

#[test]
fn reuse_only_session_keeps_export_enabled_without_claiming_queue_complete() {
    let (mut controller, _temp) = controller_with_store();
    let project_id = controller.create_project("Lecture series").unwrap();
    start_bound(&mut controller, A_SRT, KAFKA_TERMS, "a.srt", &project_id);
    promote_selected(&mut controller);
    controller.reset().unwrap();

    start_bound(&mut controller, B_SRT, EMPTY_TERMS, "b.srt", &project_id);
    let items = controller.items().unwrap();
    let progress = controller.progress().unwrap();
    let waiting = items
        .iter()
        .filter(|item| {
            item.status == "Needs review"
                && matches!(item.origin, ReviewItemOrigin::PreviousCorrection { .. })
        })
        .count();
    assert_eq!(waiting, 1);
    assert!(matches!(
        progress.decision_coverage,
        ApplicationDecisionCoverage::Complete
    ));
    assert!(export_enabled(progress));
    assert!(!show_review_complete_panel(progress, waiting));
    assert_eq!(
        composed_coverage_label(
            progress,
            controller.header().unwrap().total_review_cases,
            waiting
        ),
        "Term checks complete · 1 previous correction(s) still need a decision"
    );
}

#[test]
fn different_y_coverage_uses_canonical_total_not_composed_len() {
    let (mut controller, _temp) = controller_with_store();
    let project_id = controller.create_project("Lecture series").unwrap();
    start_bound(&mut controller, A_SRT, KAFKA_TERMS, "a.srt", &project_id);
    promote_selected(&mut controller);
    controller.reset().unwrap();

    start_bound(&mut controller, B_SRT, CAFKA_TERMS, "b.srt", &project_id);
    let items = controller.items().unwrap();
    let progress = controller.progress().unwrap();
    let canonical_total = controller.header().unwrap().total_review_cases;
    let waiting = items
        .iter()
        .filter(|item| {
            item.status == "Needs review"
                && matches!(item.origin, ReviewItemOrigin::PreviousCorrection { .. })
        })
        .count();
    assert_eq!(items.len(), 2);
    assert_eq!(canonical_total, 1);
    assert_eq!(waiting, 1);
    assert_eq!(
        composed_coverage_label(progress, canonical_total, waiting),
        "Progress: 0 of 1 reviewed"
    );
    assert!(!export_enabled(progress));
}

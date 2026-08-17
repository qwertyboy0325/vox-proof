use std::path::PathBuf;

use tempfile::tempdir;
use vox_proof::application_service::{
    ApplicationDecisionCoverage, DeclaredApplicationMaterialUseBasis, DeclaredSessionOperatorRole,
};
use vox_proof::session_persistence::ProductSessionStore;
use voxproof_desktop::controller::{
    ControllerError, DesktopController, DesktopPhase, ReviewItemOrigin,
};
use voxproof_desktop::media::{FakeMediaBackend, MediaPreview, MediaSeekIntent};
use voxproof_desktop::presentation::{
    composed_queue_complete, export_enabled, show_review_complete_panel,
};
use voxproof_desktop::source_selection::{
    CueCharRange, CueSelectionSession, SelectionError, reject_cross_cue, resolve_cue_selection,
    resolve_session_selection, selected_span_text, selection_for_cue, whole_cue_selection,
};

const MIXED_SRT: &str = "1\n00:00:00,000 --> 00:00:01,000\nHello Postgres\n";
const COLD_START_SRT: &str = "1\n00:00:00,000 --> 00:00:01,000\n上來一發\n";
const EMPTY_TERMS: &str = "";
const POSTGRES_TERMS: &str = "PostgreSQL | alias:Postgres\n";

fn controller_with_store() -> (DesktopController, tempfile::TempDir) {
    let temp = tempdir().unwrap();
    let store = ProductSessionStore::new(temp.path());
    (DesktopController::with_store(store), temp)
}

#[test]
fn selection_to_human_raised_reuses_existing_api() {
    let (mut controller, _temp) = controller_with_store();
    let project_id = controller.create_project("Lecture series").unwrap();
    controller
        .start_from_text_in_project(
            MIXED_SRT,
            EMPTY_TERMS,
            Some(PathBuf::from("a.srt")),
            DeclaredApplicationMaterialUseBasis::SelfOwned,
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "Reviewer",
            &project_id,
        )
        .unwrap();
    let cue = &controller.cues().unwrap()[0];
    let selection = CueCharRange {
        cue_index: cue.segment_position,
        start_char: 6,
        end_char: 14,
    };
    let displayed = selected_span_text(&cue.text, selection.start_char, selection.end_char);
    let span = resolve_cue_selection(&cue.text, selection, &displayed).unwrap();
    assert_eq!(span.observed_text, "Postgres");
    let epoch = controller.ui_session_epoch();
    controller
        .raise_and_manual_replace(
            epoch,
            span.segment_position,
            span.start_byte,
            span.end_byte,
            "PostgreSQL",
        )
        .unwrap();
    let items = controller.items().unwrap();
    assert!(items.iter().any(|item| {
        matches!(item.origin, ReviewItemOrigin::HumanRaisedCorrection)
            && item.source_text == "Postgres"
    }));
    assert!(controller.projection().unwrap().srt.contains("PostgreSQL"));
}

#[test]
fn cjk_cue_selection_saves_through_human_raised() {
    let (mut controller, _temp) = controller_with_store();
    controller
        .start_from_text(
            "1\n00:00:00,000 --> 00:00:01,000\n歡迎使用轉錄\n",
            EMPTY_TERMS,
            Some(PathBuf::from("cjk.srt")),
            DeclaredApplicationMaterialUseBasis::SelfOwned,
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "Reviewer",
        )
        .unwrap();
    let cue = &controller.cues().unwrap()[0];
    let displayed = selected_span_text(&cue.text, 2, 4);
    assert_eq!(displayed, "使用");
    let span = resolve_cue_selection(
        &cue.text,
        CueCharRange {
            cue_index: 0,
            start_char: 2,
            end_char: 4,
        },
        &displayed,
    )
    .unwrap();
    controller
        .raise_and_manual_replace(
            controller.ui_session_epoch(),
            span.segment_position,
            span.start_byte,
            span.end_byte,
            "採用",
        )
        .unwrap();
    assert!(controller.projection().unwrap().srt.contains("採用"));
}

#[test]
fn empty_and_cross_cue_and_mismatch_are_refused_before_save() {
    assert_eq!(
        resolve_cue_selection(
            "Postgres",
            CueCharRange {
                cue_index: 0,
                start_char: 1,
                end_char: 1,
            },
            "",
        ),
        Err(SelectionError::Empty)
    );
    assert_eq!(reject_cross_cue(0, 1), Err(SelectionError::CrossCue));
    assert_eq!(
        resolve_cue_selection(
            "Postgres",
            CueCharRange {
                cue_index: 0,
                start_char: 0,
                end_char: 8,
            },
            "PostgreSQL",
        ),
        Err(SelectionError::ObservedMismatch)
    );
}

#[test]
fn whole_line_correction_uses_human_raised() {
    let (mut controller, _temp) = controller_with_store();
    controller
        .start_from_text(
            MIXED_SRT,
            EMPTY_TERMS,
            Some(PathBuf::from("line.srt")),
            DeclaredApplicationMaterialUseBasis::SelfOwned,
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "Reviewer",
        )
        .unwrap();
    let cue = &controller.cues().unwrap()[0];
    let selection = whole_cue_selection(cue.segment_position, &cue.text);
    let span = resolve_cue_selection(&cue.text, selection, &cue.text).unwrap();
    controller
        .raise_and_manual_replace(
            controller.ui_session_epoch(),
            span.segment_position,
            span.start_byte,
            span.end_byte,
            "Hello PostgreSQL",
        )
        .unwrap();
    assert_eq!(
        controller.projection().unwrap().srt.lines().nth(2),
        Some("Hello PostgreSQL")
    );
}

#[test]
fn review_starts_without_media_and_keeps_export_rules() {
    let (mut controller, _temp) = controller_with_store();
    controller
        .start_from_text(
            MIXED_SRT,
            EMPTY_TERMS,
            Some(PathBuf::from("no-media.srt")),
            DeclaredApplicationMaterialUseBasis::SelfOwned,
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "Reviewer",
        )
        .unwrap();
    assert_eq!(controller.phase(), DesktopPhase::ActiveReview);
    let progress = controller.progress().unwrap();
    assert!(matches!(
        progress.decision_coverage,
        ApplicationDecisionCoverage::Complete
    ));
    assert!(export_enabled(progress));
    assert!(show_review_complete_panel(progress, 0));
    assert!(composed_queue_complete(progress, 0));
}

#[test]
fn cue_start_ms_is_available_for_seek() {
    let (mut controller, _temp) = controller_with_store();
    controller
        .start_from_text(
            "1\n00:00:01,250 --> 00:00:02,000\nHello\n",
            EMPTY_TERMS,
            Some(PathBuf::from("timed.srt")),
            DeclaredApplicationMaterialUseBasis::SelfOwned,
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "Reviewer",
        )
        .unwrap();
    let cue = &controller.cues().unwrap()[0];
    assert_eq!(cue.start_ms, 1_250);
    let mut preview = MediaPreview::new(FakeMediaBackend::with_duration(10_000));
    preview.seek_to_cue_start(cue.start_ms);
    preview.attach(PathBuf::from("/tmp/talk.mp4")).unwrap();
    assert_eq!(preview.snapshot().position_ms, 1_250);
}

#[test]
fn playback_failure_leaves_review_usable() {
    let (mut controller, _temp) = controller_with_store();
    controller
        .start_from_text(
            MIXED_SRT,
            EMPTY_TERMS,
            Some(PathBuf::from("fail-media.srt")),
            DeclaredApplicationMaterialUseBasis::SelfOwned,
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "Reviewer",
        )
        .unwrap();
    let mut backend = FakeMediaBackend::with_duration(1_000);
    backend.fail_next_attach = true;
    let mut preview = MediaPreview::new(backend);
    assert!(preview.attach(PathBuf::from("/tmp/missing.mp4")).is_err());
    assert_eq!(controller.phase(), DesktopPhase::ActiveReview);
    assert!(controller.projection().is_ok());
}

#[test]
fn human_raised_still_promotes_after_selection_save() {
    let (mut controller, _temp) = controller_with_store();
    let project_id = controller.create_project("Lecture series").unwrap();
    controller
        .start_from_text_in_project(
            MIXED_SRT,
            EMPTY_TERMS,
            Some(PathBuf::from("a.srt")),
            DeclaredApplicationMaterialUseBasis::SelfOwned,
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "Reviewer",
            &project_id,
        )
        .unwrap();
    let cue = &controller.cues().unwrap()[0];
    let span = resolve_cue_selection(
        &cue.text,
        CueCharRange {
            cue_index: 0,
            start_char: 6,
            end_char: 14,
        },
        "Postgres",
    )
    .unwrap();
    let epoch = controller.ui_session_epoch();
    controller
        .raise_and_manual_replace(
            epoch,
            span.segment_position,
            span.start_byte,
            span.end_byte,
            "PostgreSQL",
        )
        .unwrap();
    controller
        .use_selected_correction_in_related_reviews(epoch)
        .unwrap();
    let entries = controller.project_memory_entries().unwrap();
    assert!(
        entries.iter().any(|entry| entry.observed_text == "Postgres"
            && entry.confirmed_replacement == "PostgreSQL")
    );
}

#[test]
fn resolving_selection_does_not_mutate_output_until_human_raised_save() {
    let (mut controller, _temp) = controller_with_store();
    controller
        .start_from_text(
            MIXED_SRT,
            EMPTY_TERMS,
            Some(PathBuf::from("before-save.srt")),
            DeclaredApplicationMaterialUseBasis::SelfOwned,
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "Reviewer",
        )
        .unwrap();
    let before = controller.projection().unwrap().srt.clone();
    let cue = &controller.cues().unwrap()[0];
    let span = resolve_cue_selection(
        &cue.text,
        CueCharRange {
            cue_index: 0,
            start_char: 6,
            end_char: 14,
        },
        "Postgres",
    )
    .unwrap();
    assert_eq!(controller.projection().unwrap().srt, before);
    assert!(!before.contains("PostgreSQL"));
    controller
        .raise_and_manual_replace(
            controller.ui_session_epoch(),
            span.segment_position,
            span.start_byte,
            span.end_byte,
            "PostgreSQL",
        )
        .unwrap();
    let after = controller.projection().unwrap().srt;
    assert_ne!(after, before);
    assert!(after.contains("PostgreSQL"));
}

#[test]
fn selection_cache_cannot_cross_cues() {
    let cue_a = CueCharRange {
        cue_index: 0,
        start_char: 6,
        end_char: 14,
    };
    let cue_b_live = CueCharRange {
        cue_index: 1,
        start_char: 0,
        end_char: 2,
    };
    assert_eq!(selection_for_cue(None, Some(cue_a), 1), None);
    assert_eq!(
        selection_for_cue(Some(cue_b_live), Some(cue_a), 1),
        Some(cue_b_live)
    );
    assert_eq!(selection_for_cue(None, Some(cue_a), 0), Some(cue_a));
}

#[test]
fn explicit_cue_seek_survives_ordinary_repaint_establish() {
    let mut intent = MediaSeekIntent::default();
    let mut preview = MediaPreview::new(FakeMediaBackend::with_duration(10_000));
    preview.attach(PathBuf::from("/tmp/talk.mp4")).unwrap();
    assert!(intent.on_event(0, false));
    preview.seek_to_cue_start(0);
    assert!(intent.on_event(1, true));
    preview.seek_to_cue_start(1_250);
    assert_eq!(preview.snapshot().position_ms, 1_250);
    assert!(!intent.on_repaint_establish(0));
    assert_eq!(intent.last_segment(), Some(1));
    assert_eq!(preview.snapshot().position_ms, 1_250);
}

fn resolve_cold_start_span(
    controller: &DesktopController,
) -> voxproof_desktop::source_selection::ResolvedSourceSpan {
    let cue = &controller.cues().unwrap()[0];
    assert_eq!(cue.text, "上來一發");
    let displayed = selected_span_text(&cue.text, 0, 2);
    assert_eq!(displayed, "上來");
    resolve_cue_selection(
        &cue.text,
        CueCharRange {
            cue_index: cue.segment_position,
            start_char: 0,
            end_char: 2,
        },
        &displayed,
    )
    .unwrap()
}

#[test]
fn empty_queue_source_correction_opens_without_review_item() {
    let (mut controller, _temp) = controller_with_store();
    let project_id = controller.create_project("Lecture series").unwrap();
    controller
        .start_from_text_in_project(
            COLD_START_SRT,
            EMPTY_TERMS,
            Some(PathBuf::from("cold.srt")),
            DeclaredApplicationMaterialUseBasis::SelfOwned,
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "Reviewer",
            &project_id,
        )
        .unwrap();
    assert!(controller.items().unwrap().is_empty());
    assert!(
        controller
            .promotion_candidate_for_selected()
            .unwrap()
            .is_none()
    );
    let before = controller.projection().unwrap().srt.clone();
    let span = resolve_cold_start_span(&controller);
    assert_eq!(span.observed_text, "上來");
    assert_eq!(controller.projection().unwrap().srt, before);
    assert!(!before.contains("發射"));
    controller
        .raise_and_manual_replace(
            controller.ui_session_epoch(),
            span.segment_position,
            span.start_byte,
            span.end_byte,
            "發射",
        )
        .unwrap();
    let items = controller.items().unwrap();
    assert_eq!(items.len(), 1);
    assert!(matches!(
        items[0].origin,
        ReviewItemOrigin::HumanRaisedCorrection
    ));
    let after = controller.projection().unwrap().srt;
    assert_ne!(after, before);
    assert!(after.contains("發射"));
    assert!(!after.contains("上來一發"));
}

#[test]
fn empty_queue_source_correction_works_unbound_and_without_media() {
    let (mut controller, _temp) = controller_with_store();
    controller
        .start_from_text(
            COLD_START_SRT,
            EMPTY_TERMS,
            Some(PathBuf::from("unbound-cold.srt")),
            DeclaredApplicationMaterialUseBasis::SelfOwned,
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "Reviewer",
        )
        .unwrap();
    assert!(controller.items().unwrap().is_empty());
    assert!(!MediaPreview::default().is_attached());
    let span = resolve_cold_start_span(&controller);
    controller
        .raise_and_manual_replace(
            controller.ui_session_epoch(),
            span.segment_position,
            span.start_byte,
            span.end_byte,
            "發射",
        )
        .unwrap();
    assert!(controller.projection().unwrap().srt.contains("發射"));
}

#[test]
fn empty_queue_source_correction_survives_reopen() {
    let (mut controller, _temp) = controller_with_store();
    let project_id = controller.create_project("Lecture series").unwrap();
    controller
        .start_from_text_in_project(
            COLD_START_SRT,
            EMPTY_TERMS,
            Some(PathBuf::from("reopen-cold.srt")),
            DeclaredApplicationMaterialUseBasis::SelfOwned,
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "Reviewer",
            &project_id,
        )
        .unwrap();
    let session_id = controller.session_id().unwrap().to_owned();
    let span = resolve_cold_start_span(&controller);
    controller
        .raise_and_manual_replace(
            controller.ui_session_epoch(),
            span.segment_position,
            span.start_byte,
            span.end_byte,
            "發射",
        )
        .unwrap();
    controller.reset().unwrap();
    controller.select_resume_session_id(session_id);
    controller.open_selected_session_writable().unwrap();
    assert!(controller.projection().unwrap().srt.contains("發射"));
    assert!(matches!(
        controller.items().unwrap()[0].origin,
        ReviewItemOrigin::HumanRaisedCorrection
    ));
}

#[test]
fn empty_queue_item_family_actions_still_require_a_review_item() {
    let (mut controller, _temp) = controller_with_store();
    controller
        .start_from_text(
            COLD_START_SRT,
            EMPTY_TERMS,
            Some(PathBuf::from("family-a.srt")),
            DeclaredApplicationMaterialUseBasis::SelfOwned,
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "Reviewer",
        )
        .unwrap();
    assert!(controller.items().unwrap().is_empty());
    let epoch = controller.ui_session_epoch();
    assert!(matches!(
        controller.record_manual_replacement(epoch, "發射"),
        Err(ControllerError::NoSelectedCase)
    ));
    assert!(matches!(
        controller.record_decision(epoch, vox_proof::review::CorrectionDecision::Reject),
        Err(ControllerError::NoSelectedCase)
    ));
}

#[test]
fn existing_review_item_edit_is_unchanged() {
    let (mut controller, _temp) = controller_with_store();
    controller
        .start_from_text(
            MIXED_SRT,
            POSTGRES_TERMS,
            Some(PathBuf::from("edit-item.srt")),
            DeclaredApplicationMaterialUseBasis::SelfOwned,
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "Reviewer",
        )
        .unwrap();
    let items = controller.items().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].source_text, "Postgres");
    let before = controller.projection().unwrap().srt.clone();
    controller
        .record_manual_replacement(controller.ui_session_epoch(), "PostgreSQL")
        .unwrap();
    let after = controller.projection().unwrap().srt;
    assert_ne!(after, before);
    assert!(after.contains("PostgreSQL"));
}

#[test]
fn empty_selection_still_refused_for_cold_start_source() {
    assert_eq!(
        resolve_cue_selection(
            "上來一發",
            CueCharRange {
                cue_index: 0,
                start_char: 1,
                end_char: 1,
            },
            "",
        ),
        Err(SelectionError::Empty)
    );
}

#[test]
fn bound_human_raised_cold_start_can_still_promote() {
    let (mut controller, _temp) = controller_with_store();
    let project_id = controller.create_project("Lecture series").unwrap();
    controller
        .start_from_text_in_project(
            COLD_START_SRT,
            EMPTY_TERMS,
            Some(PathBuf::from("promote-cold.srt")),
            DeclaredApplicationMaterialUseBasis::SelfOwned,
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "Reviewer",
            &project_id,
        )
        .unwrap();
    let span = resolve_cold_start_span(&controller);
    let epoch = controller.ui_session_epoch();
    controller
        .raise_and_manual_replace(
            epoch,
            span.segment_position,
            span.start_byte,
            span.end_byte,
            "發射",
        )
        .unwrap();
    controller
        .use_selected_correction_in_related_reviews(epoch)
        .unwrap();
    let entries = controller.project_memory_entries().unwrap();
    assert!(
        entries
            .iter()
            .any(|entry| entry.observed_text == "上來" && entry.confirmed_replacement == "發射")
    );
}

#[test]
fn cue_session_drag_opens_human_raised_with_empty_queue() {
    let (mut controller, _temp) = controller_with_store();
    controller
        .start_from_text(
            COLD_START_SRT,
            EMPTY_TERMS,
            Some(PathBuf::from("session-cold.srt")),
            DeclaredApplicationMaterialUseBasis::SelfOwned,
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "Reviewer",
        )
        .unwrap();
    assert!(controller.items().unwrap().is_empty());
    let cue = &controller.cues().unwrap()[0];
    let mut session = CueSelectionSession::default();
    session.begin_primary(cue.segment_position, 0);
    session.extend_primary(cue.segment_position, 2);
    session.finish_primary(cue.segment_position, 2, true);
    session.sync_rendered_text(cue.segment_position, &cue.text);
    let before = controller.projection().unwrap().srt.clone();
    let span = resolve_session_selection(&session, cue.segment_position, &cue.text).unwrap();
    assert_eq!(span.observed_text, "上來");
    assert_eq!(controller.projection().unwrap().srt, before);
    controller
        .raise_and_manual_replace(
            controller.ui_session_epoch(),
            span.segment_position,
            span.start_byte,
            span.end_byte,
            "發射",
        )
        .unwrap();
    let items = controller.items().unwrap();
    assert_eq!(items.len(), 1);
    assert!(matches!(
        items[0].origin,
        ReviewItemOrigin::HumanRaisedCorrection
    ));
    assert!(controller.projection().unwrap().srt.contains("發射"));
}

#[test]
fn cue_session_click_and_cross_cue_cannot_open_correction() {
    let mut session = CueSelectionSession::default();
    session.begin_primary(0, 1);
    session.finish_primary(0, 1, false);
    assert_eq!(
        resolve_session_selection(&session, 0, "上來一發"),
        Err(SelectionError::Empty)
    );

    session.begin_primary(0, 0);
    session.extend_primary(0, 2);
    session.finish_primary(0, 2, true);
    assert_eq!(
        resolve_session_selection(&session, 1, "另一行"),
        Err(SelectionError::Empty)
    );
    session.begin_primary(1, 0);
    assert_eq!(
        resolve_session_selection(&session, 0, "上來一發"),
        Err(SelectionError::Empty)
    );
}

#[test]
fn cue_session_survives_focus_loss_and_clears_on_source_revision() {
    let mut session = CueSelectionSession::default();
    session.begin_primary(0, 0);
    session.extend_primary(0, 2);
    session.finish_primary(0, 2, true);
    session.sync_rendered_text(0, "上來一發");
    let before = session.clone();
    let _ = resolve_session_selection(&session, 0, "上來一發").unwrap();
    assert_eq!(session, before);
    session.sync_rendered_text(0, "發射一發");
    assert_eq!(session.selection(), None);
}

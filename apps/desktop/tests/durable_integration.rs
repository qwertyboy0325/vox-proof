use std::path::PathBuf;

use tempfile::tempdir;
use vox_proof::application_service::{
    DeclaredApplicationMaterialUseBasis, DeclaredSessionOperatorRole,
};
use vox_proof::review::CorrectionDecision;
use vox_proof::session_persistence::{
    ProductSessionStore, arm_force_hydrate_failure_for_test, disarm_force_hydrate_failure_for_test,
};
use voxproof_desktop::controller::{ControllerError, DesktopController, DesktopPhase};

const SRT: &str = "1\n00:00:00,000 --> 00:00:01,000\nKafak\n";
const TERMS: &str = "Kafka | alias:Kafak\n";

fn controller_with_store() -> (DesktopController, tempfile::TempDir) {
    let temp = tempdir().unwrap();
    let store = ProductSessionStore::new(temp.path());
    (DesktopController::with_store(store), temp)
}

fn start(controller: &mut DesktopController) {
    controller
        .start_from_text(
            SRT,
            TERMS,
            Some(PathBuf::from("sample.srt")),
            DeclaredApplicationMaterialUseBasis::SelfOwned,
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "Reviewer",
        )
        .unwrap();
}

#[test]
fn desktop_create_produces_session_db_and_exposes_session_id() {
    let (controller, temp) = controller_with_store();
    let mut controller = controller;
    start(&mut controller);
    let session_id = controller.session_id().expect("session id").to_owned();
    let db_path = temp.path().join(&session_id).join("session.db");
    assert!(db_path.is_file());
    assert_eq!(controller.phase(), DesktopPhase::ActiveReview);
}

#[test]
fn writable_resume_after_clean_close_preserves_review_ledger() {
    let (mut controller, _temp) = controller_with_store();
    start(&mut controller);
    let session_id = controller.session_id().unwrap().to_owned();
    let epoch = controller.ui_session_epoch();
    controller
        .record_decision(epoch, CorrectionDecision::Reject)
        .unwrap();
    controller.reset().unwrap();

    controller.select_resume_session_id(session_id.clone());
    controller.open_selected_session_writable().unwrap();
    assert_eq!(controller.header().unwrap().total_recorded_events, 1);
}

#[test]
fn read_only_open_allows_export_but_blocks_mutations() {
    let (mut controller, _temp) = controller_with_store();
    start(&mut controller);
    let session_id = controller.session_id().unwrap().to_owned();
    let epoch = controller.ui_session_epoch();
    controller
        .record_decision(epoch, CorrectionDecision::Reject)
        .unwrap();
    controller.reset().unwrap();

    controller.select_resume_session_id(session_id);
    controller.open_selected_session_read_only().unwrap();
    assert!(controller.is_read_only());
    assert!(controller.export_previews(false).is_ok());
    assert!(matches!(
        controller.record_decision(controller.ui_session_epoch(), CorrectionDecision::Defer),
        Err(ControllerError::SessionNotWritable)
    ));
}

#[test]
fn durable_commit_does_not_change_ui_session_epoch() {
    let (mut controller, _temp) = controller_with_store();
    start(&mut controller);
    let epoch = controller.ui_session_epoch();
    controller
        .record_decision(epoch, CorrectionDecision::Reject)
        .unwrap();
    assert_eq!(controller.ui_session_epoch(), epoch);
}

#[test]
fn recovery_required_blocks_presentable_reads_and_mutations() {
    let (mut controller, _temp) = controller_with_store();
    start(&mut controller);
    let epoch = controller.ui_session_epoch();
    arm_force_hydrate_failure_for_test();
    let result = controller.record_decision(epoch, CorrectionDecision::Reject);
    disarm_force_hydrate_failure_for_test();
    assert!(matches!(result, Err(ControllerError::RecoveryRequired)));
    assert_eq!(controller.phase(), DesktopPhase::RecoveryRequired);
    assert!(controller.items().is_err());
    assert!(controller.retry_recovery().is_ok());
    assert_eq!(controller.phase(), DesktopPhase::ActiveReview);
}

#[test]
fn reopened_session_shows_embedded_transcript_when_import_path_not_retained() {
    let (mut controller, _temp) = controller_with_store();
    start(&mut controller);
    let session_id = controller.session_id().unwrap().to_owned();
    controller.reset().unwrap();
    controller.select_resume_session_id(session_id);
    controller.open_selected_session_read_only().unwrap();
    let header = controller.header().unwrap();
    assert_eq!(header.source_path, "Embedded canonical transcript");
    assert_eq!(
        header.source_path_note.as_deref(),
        Some("Original import path not retained")
    );
}

#[test]
fn list_session_ids_discovers_created_sessions() {
    let (mut controller, _temp) = controller_with_store();
    start(&mut controller);
    controller.refresh_available_sessions().unwrap();
    assert_eq!(controller.available_session_ids().len(), 1);
}

#[test]
fn replacing_active_session_releases_prior_writer_ownership() {
    let (mut controller, _temp) = controller_with_store();
    start(&mut controller);
    let first_id = controller.session_id().unwrap().to_owned();
    start(&mut controller);
    controller.reset().unwrap();
    controller.select_resume_session_id(first_id);
    controller.open_selected_session_writable().unwrap();
}

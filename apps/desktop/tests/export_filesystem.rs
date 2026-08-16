use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;
use vox_proof::application_export::{
    render_application_decision_log, render_application_session_summary,
};
use vox_proof::application_service::{
    ApplicationMaterialUseDeclaration, ApplicationReviewExportBundle,
    DeclaredApplicationMaterialUseBasis, DeclaredSessionAuthority, DeclaredSessionOperatorRole,
    begin_application_review,
};
use vox_proof::review::CorrectionDecision;
use vox_proof::session_terms::parse_session_terms;
use vox_proof::srt::parse_srt;
use vox_proof::session_persistence::ProductSessionStore;
use voxproof_desktop::controller::{ControllerError, DesktopController};
use voxproof_desktop::export::ExportError;

const SRT: &str = "1\n00:00:00,000 --> 00:00:01,000\n這是華說的新產品\n";
const TERMS: &str = "華碩 | alias:華說\n";

fn controller(source: PathBuf) -> (DesktopController, tempfile::TempDir) {
    let temp = tempdir().unwrap();
    let store = ProductSessionStore::new(temp.path());
    let mut controller = DesktopController::with_store(store);
    controller
        .start_from_text(
            SRT,
            TERMS,
            Some(source),
            DeclaredApplicationMaterialUseBasis::SelfOwned,
            DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            "Filesystem reviewer",
        )
        .unwrap();
    let epoch = controller.ui_session_epoch();
    controller
        .record_decision(
            epoch,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .unwrap();
    (controller, temp)
}

fn expected_bundle() -> ApplicationReviewExportBundle {
    let transcript = parse_srt(SRT).unwrap();
    let terms = parse_session_terms(TERMS).unwrap();
    let authority = DeclaredSessionAuthority::new(
        DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
        "Filesystem reviewer",
    )
    .unwrap();
    let mut session = begin_application_review(
        transcript,
        terms,
        ApplicationMaterialUseDeclaration::new(DeclaredApplicationMaterialUseBasis::SelfOwned),
        authority,
    )
    .unwrap();
    let target = session.review_items()[0].target;
    session
        .record_human_decision(
            target,
            CorrectionDecision::AcceptAlternative {
                alternative_index: 0,
            },
        )
        .unwrap();
    session.materialize_review_export_bundle().unwrap()
}

fn names(directory: &Path) -> Vec<String> {
    let mut names = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    names.sort();
    names
}

#[test]
fn successful_export_uses_expected_names_and_exact_core_renderer_bytes() {
    let directory = tempdir().unwrap();
    let bundle = expected_bundle();
    let (mut controller, _temp) = controller(PathBuf::from("episode.srt"));

    let paths = controller
        .export(controller.ui_session_epoch(), directory.path(), false)
        .unwrap();

    assert_eq!(
        names(directory.path()),
        vec![
            "episode.voxproof-decisions.txt",
            "episode.voxproof-reviewed.srt",
            "episode.voxproof-session-summary.txt",
        ]
    );
    assert_eq!(
        fs::read(paths.reviewed_srt).unwrap(),
        bundle.reviewed_srt.as_bytes()
    );
    assert_eq!(
        fs::read(paths.decision_log).unwrap(),
        render_application_decision_log(&bundle).as_bytes()
    );
    assert_eq!(
        fs::read(paths.session_summary).unwrap(),
        render_application_session_summary(&bundle).as_bytes()
    );
}

#[test]
fn missing_source_stem_falls_back_to_transcript() {
    let directory = tempdir().unwrap();
    let (mut controller, _temp) = controller(PathBuf::new());

    controller
        .export(controller.ui_session_epoch(), directory.path(), false)
        .unwrap();

    assert_eq!(
        names(directory.path()),
        vec![
            "transcript.voxproof-decisions.txt",
            "transcript.voxproof-reviewed.srt",
            "transcript.voxproof-session-summary.txt",
        ]
    );
}

#[test]
fn multiple_collisions_are_all_reported_and_nothing_else_is_written() {
    let directory = tempdir().unwrap();
    let decision = directory.path().join("episode.voxproof-decisions.txt");
    let summary = directory
        .path()
        .join("episode.voxproof-session-summary.txt");
    fs::write(&decision, "decision sentinel").unwrap();
    fs::write(&summary, "summary sentinel").unwrap();
    let (mut controller, _temp) = controller(PathBuf::from("episode.srt"));

    let error = controller
        .export(controller.ui_session_epoch(), directory.path(), false)
        .unwrap_err();

    assert!(matches!(
        error,
        ControllerError::Export(ExportError::Collision { paths })
            if paths.len() == 2
    ));
    assert_eq!(fs::read_to_string(decision).unwrap(), "decision sentinel");
    assert_eq!(fs::read_to_string(summary).unwrap(), "summary sentinel");
    assert!(
        !directory
            .path()
            .join("episode.voxproof-reviewed.srt")
            .exists()
    );
}

#[test]
fn exported_renderer_bytes_inject_no_paths_timestamps_or_gui_metadata() {
    let directory = tempdir().unwrap();
    let source = PathBuf::from("/private/source/customer-interview.srt");
    let (mut controller, _temp) = controller(source.clone());
    let paths = controller
        .export(controller.ui_session_epoch(), directory.path(), false)
        .unwrap();

    for path in [
        paths.reviewed_srt,
        paths.decision_log,
        paths.session_summary,
    ] {
        let text = fs::read_to_string(path).unwrap();
        assert!(!text.contains(source.to_string_lossy().as_ref()));
        assert!(!text.contains(directory.path().to_string_lossy().as_ref()));
        assert!(!text.contains("gui_"));
        assert!(!text.contains("exported_at"));
    }
}

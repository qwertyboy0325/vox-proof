#[test]
fn core_manifest_has_no_desktop_framework_or_dialog_dependency() {
    let core_manifest = include_str!("../../../Cargo.toml");
    let dependency_section = core_manifest
        .split("[dependencies]")
        .nth(1)
        .and_then(|tail| tail.split("[dev-dependencies]").next())
        .unwrap();

    for forbidden in ["eframe", "egui", "rfd", "font-kit"] {
        assert!(!dependency_section.contains(forbidden));
    }
}

#[test]
fn desktop_uses_only_the_public_application_authority_boundaries() {
    let controller = include_str!("../src/controller.rs");
    let export = include_str!("../src/export.rs");

    for forbidden in [
        "ReviewLedger",
        "derive_reviewed_srt",
        "serde::",
        "Serialize",
        "autosave",
    ] {
        assert!(!controller.contains(forbidden));
        assert!(!export.contains(forbidden));
    }

    for required in [
        "review_items()",
        "prepare_human_decision",
        "record_human_decision",
        "derive_current_projection",
        "materialize_review_export_bundle",
        "DurableApplicationSession",
    ] {
        assert!(controller.contains(required));
    }
    for required in [
        "bundle.reviewed_srt",
        "render_application_decision_log",
        "render_application_session_summary",
        "create_new(true)",
    ] {
        assert!(export.contains(required));
    }
}

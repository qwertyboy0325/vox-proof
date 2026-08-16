use std::path::PathBuf;

use eframe::egui;
use egui::{Color32, RichText};
use vox_proof::application_service::{
    DeclaredApplicationMaterialUseBasis, DeclaredSessionOperatorRole,
};
use vox_proof::review::CorrectionDecision;

use crate::controller::{DesktopController, DesktopPhase, ReviewItemOrigin};
use crate::presentation::{
    BottomTab, SETUP_ADVANCED, SETUP_HEADING, SETUP_INTRO, SETUP_SEED_HINT, SETUP_SEED_TERMINOLOGY,
    SETUP_STEP_CONFIRM, SETUP_STEP_PROJECT, SETUP_STEP_TRANSCRIPT, accept_enabled,
    composed_coverage_label, coverage_label, decision_shortcut, export_enabled, resolution_label,
    review_is_complete, review_shortcuts_suppressed, search_matches, setup_can_start,
    setup_project_ready, show_review_complete_panel, unresolved_confirmation_needed,
};
use crate::terms_editor::TermsEditor;
use crate::user_errors;

const SEARCH_ID: &str = "review-search-field";

pub struct ReviewApp {
    controller: DesktopController,
    transcript_path: String,
    terms_editor: TermsEditor,
    material_use: DeclaredApplicationMaterialUseBasis,
    role: DeclaredSessionOperatorRole,
    operator_label: String,
    search: String,
    selected_alternative: usize,
    manual_replacement_draft: String,
    human_raise_cue_index: usize,
    human_raise_start_char: usize,
    human_raise_end_char: usize,
    human_raise_replacement: String,
    bottom_tab: BottomTab,
    confirm_unresolved_source_retained: bool,
    resume_session_id_draft: String,
    resume_writer_held_session_id: Option<String>,
    new_project_name: String,
    review_without_project: bool,
    error: Option<String>,
    status: String,
    cjk_font_loaded: bool,
}

impl ReviewApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.enable_accesskit();
        let cjk_font_loaded = crate::fonts::install_system_cjk_fonts(&cc.egui_ctx);
        Self {
            controller: DesktopController::default(),
            transcript_path: String::new(),
            terms_editor: TermsEditor::default(),
            material_use: DeclaredApplicationMaterialUseBasis::SelfOwned,
            role: DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            operator_label: String::new(),
            search: String::new(),
            selected_alternative: 0,
            manual_replacement_draft: String::new(),
            human_raise_cue_index: 0,
            human_raise_start_char: 0,
            human_raise_end_char: 0,
            human_raise_replacement: String::new(),
            bottom_tab: BottomTab::CurrentPreview,
            confirm_unresolved_source_retained: false,
            resume_session_id_draft: String::new(),
            resume_writer_held_session_id: None,
            new_project_name: String::new(),
            review_without_project: false,
            error: None,
            status: "Start a new review or continue a recent one.".to_owned(),
            cjk_font_loaded,
        }
    }

    fn ime_composing(ctx: &egui::Context) -> bool {
        ctx.input(|input| {
            input.events.iter().any(|event| {
                matches!(
                    event,
                    egui::Event::Ime(egui::ImeEvent::Preedit { text, .. }) if !text.is_empty()
                )
            })
        })
    }

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        if ctx.input(|input| input.modifiers.command && input.key_pressed(egui::Key::L)) {
            ctx.memory_mut(|memory| memory.request_focus(egui::Id::new(SEARCH_ID)));
        }
        if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            ctx.memory_mut(|memory| memory.surrender_focus(egui::Id::new(SEARCH_ID)));
            self.error = None;
        }

        let suppress_review_shortcuts = review_shortcuts_suppressed(
            ctx.egui_wants_keyboard_input(),
            Self::ime_composing(ctx),
            false,
        );
        if suppress_review_shortcuts || self.controller.phase() == DesktopPhase::Setup {
            return;
        }

        if ctx.input(|input| input.key_pressed(egui::Key::ArrowUp)) {
            self.controller.select_relative(-1);
            self.selected_alternative = 0;
            self.manual_replacement_draft.clear();
        }
        if ctx.input(|input| input.key_pressed(egui::Key::ArrowDown)) {
            self.controller.select_relative(1);
            self.selected_alternative = 0;
            self.manual_replacement_draft.clear();
        }
        if !self.controller.mutations_enabled() {
            return;
        }
        for (index, key) in [
            (0, egui::Key::Num1),
            (1, egui::Key::Num2),
            (2, egui::Key::Num3),
            (3, egui::Key::Num4),
            (4, egui::Key::Num5),
            (5, egui::Key::Num6),
            (6, egui::Key::Num7),
            (7, egui::Key::Num8),
            (8, egui::Key::Num9),
        ] {
            if ctx.input(|input| input.key_pressed(key)) {
                self.selected_alternative = index;
            }
        }

        let decisions = ctx.input(|input| {
            input
                .events
                .iter()
                .filter_map(|event| match event {
                    egui::Event::Key {
                        key,
                        pressed,
                        repeat,
                        ..
                    } => decision_shortcut(*key, *pressed, *repeat, self.selected_alternative),
                    _ => None,
                })
                .collect::<Vec<_>>()
        });
        for decision in decisions {
            self.apply_decision(decision);
        }
    }

    fn apply_decision(&mut self, decision: CorrectionDecision) {
        let ui_session_epoch = self.controller.ui_session_epoch();
        let invalidates_prior_export = self.controller.phase() == DesktopPhase::ExportCompleted;
        match self.controller.record_decision(ui_session_epoch, decision) {
            Ok(()) => {
                self.error = None;
                self.status = if invalidates_prior_export {
                    "This review changed after export. Export again to refresh the saved files."
                        .to_owned()
                } else {
                    "Decision saved.".to_owned()
                };
                self.selected_alternative = 0;
                self.manual_replacement_draft.clear();
            }
            Err(error) => self.error = Some(user_errors::user_message(&error)),
        }
    }

    fn apply_manual_replacement(&mut self) {
        let ui_session_epoch = self.controller.ui_session_epoch();
        let invalidates_prior_export = self.controller.phase() == DesktopPhase::ExportCompleted;
        match self
            .controller
            .record_manual_replacement(ui_session_epoch, self.manual_replacement_draft.clone())
        {
            Ok(()) => {
                self.error = None;
                self.status = if invalidates_prior_export {
                    "This review changed after export. Export again to refresh the saved files."
                        .to_owned()
                } else {
                    "Correction saved.".to_owned()
                };
                self.selected_alternative = 0;
                self.manual_replacement_draft.clear();
            }
            Err(error) => self.error = Some(user_errors::user_message(&error)),
        }
    }

    fn apply_human_raise(&mut self) {
        let Ok(cues) = self.controller.cue_texts() else {
            return;
        };
        let Some((_, cue_text)) = cues.get(self.human_raise_cue_index) else {
            self.error = Some("Select a subtitle line first.".to_owned());
            return;
        };
        let Some((start_byte, end_byte)) = char_range_to_utf8_bytes(
            cue_text,
            self.human_raise_start_char,
            self.human_raise_end_char,
        ) else {
            self.error = Some("Select a contiguous span inside one subtitle line.".to_owned());
            return;
        };
        let ui_session_epoch = self.controller.ui_session_epoch();
        let invalidates_prior_export = self.controller.phase() == DesktopPhase::ExportCompleted;
        match self.controller.raise_and_manual_replace(
            ui_session_epoch,
            self.human_raise_cue_index,
            start_byte,
            end_byte,
            self.human_raise_replacement.clone(),
        ) {
            Ok(()) => {
                self.error = None;
                self.status = if invalidates_prior_export {
                    "This review changed after export. Export again to refresh the saved files."
                        .to_owned()
                } else {
                    "Unflagged correction saved.".to_owned()
                };
                self.human_raise_replacement.clear();
            }
            Err(error) => self.error = Some(user_errors::user_message(&error)),
        }
    }

    fn reset(&mut self) {
        match self.controller.reset() {
            Ok(()) => {
                self.transcript_path.clear();
                self.terms_editor = TermsEditor::default();
                self.operator_label.clear();
                self.material_use = DeclaredApplicationMaterialUseBasis::SelfOwned;
                self.role = DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator;
                self.search.clear();
                self.selected_alternative = 0;
                self.manual_replacement_draft.clear();
                self.bottom_tab = BottomTab::CurrentPreview;
                self.confirm_unresolved_source_retained = false;
                self.new_project_name.clear();
                self.review_without_project = false;
                self.resume_writer_held_session_id = None;
                self.error = None;
                self.status =
                    "Review closed. Start a new review or continue a recent one.".to_owned();
            }
            Err(error) => self.error = Some(user_errors::user_message(&error)),
        }
    }
}

impl eframe::App for ReviewApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_keyboard(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("voxproof-top")
            .exact_size(44.0)
            .show(ui, |ui| self.top_bar(ui));
        egui::Panel::bottom("voxproof-status")
            .exact_size(30.0)
            .show(ui, |ui| self.status_bar(ui));

        match self.controller.phase() {
            DesktopPhase::Setup => {
                egui::CentralPanel::default().show(ui, |ui| self.setup(ui));
            }
            DesktopPhase::RecoveryRequired => {
                egui::CentralPanel::default().show(ui, |ui| self.recovery(ui));
            }
            DesktopPhase::ActiveReview | DesktopPhase::ExportCompleted => self.review(ui),
        }
    }
}

impl ReviewApp {
    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("VoxProof");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.controller.phase() != DesktopPhase::Setup
                    && ui.button("Close review").clicked()
                {
                    self.reset();
                }
            });
        });
    }

    fn status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.small("↑/↓ navigate · 1–9 choose alternative · A/R/D/M decide · ⌘/Ctrl+L search · Esc clear focus");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(error) = &self.error {
                    ui.colored_label(Color32::LIGHT_RED, error);
                } else {
                    ui.small(&self.status);
                }
            });
        });
    }

    fn setup(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_max_width(860.0);
            ui.add_space(20.0);
            ui.heading(SETUP_HEADING);
            ui.label(SETUP_INTRO);
            ui.add_space(12.0);

            ui.group(|ui| {
                ui.label(RichText::new(SETUP_STEP_TRANSCRIPT).strong());
                ui.horizontal(|ui| {
                    ui.label("Transcript");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.transcript_path)
                            .id(egui::Id::new("setup-srt-path"))
                            .desired_width(560.0),
                    );
                    if ui.button("Choose SRT…").clicked()
                        && let Some(path) = rfd::FileDialog::new()
                            .add_filter("SubRip transcript", &["srt"])
                            .pick_file()
                    {
                        self.transcript_path = path.display().to_string();
                    }
                });
            });

            ui.add_space(10.0);
            ui.group(|ui| {
                ui.label(RichText::new(SETUP_STEP_PROJECT).strong());
                ui.label(
                    "VoxProof can reuse corrections you previously approved, but every new change \
                     still requires your decision.",
                );
                ui.checkbox(
                    &mut self.review_without_project,
                    "Review without a project (won't reuse corrections)",
                );
                ui.add_enabled_ui(!self.review_without_project, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("New project");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.new_project_name)
                                .id(egui::Id::new("setup-new-project"))
                                .desired_width(280.0)
                                .hint_text("Lecture series"),
                        );
                        if ui.button("Create project").clicked() {
                            match self
                                .controller
                                .create_project(self.new_project_name.clone())
                            {
                                Ok(_) => {
                                    self.new_project_name.clear();
                                    self.error = None;
                                    self.status = "Project created.".to_owned();
                                }
                                Err(error) => {
                                    self.error = Some(user_errors::user_message(&error));
                                }
                            }
                        }
                    });
                    ui.label(RichText::new("Existing projects").strong());
                    if ui.button("Refresh projects").clicked() {
                        match self.controller.refresh_available_projects() {
                            Ok(()) => self.error = None,
                            Err(error) => self.error = Some(user_errors::user_message(&error)),
                        }
                    }
                    let projects = self.controller.available_projects().to_vec();
                    if projects.is_empty() {
                        ui.label("No projects yet. Create one above.");
                    }
                    for project in &projects {
                        let selected =
                            self.controller.selected_project_id() == Some(&project.project_id);
                        let label = format!(
                            "{} · {}",
                            project.display_name,
                            Self::format_session_date(project.created_at_unix_ms)
                        );
                        if ui.selectable_label(selected, label).clicked() {
                            self.controller.select_project(project.project_id.clone());
                        }
                    }
                });
            });

            ui.add_space(10.0);
            ui.group(|ui| {
                ui.label(RichText::new(SETUP_STEP_CONFIRM).strong());
                ui.label(RichText::new("Permission").strong());
                ui.horizontal(|ui| {
                    ui.radio_value(
                        &mut self.material_use,
                        DeclaredApplicationMaterialUseBasis::SelfOwned,
                        "This is my material",
                    );
                    ui.radio_value(
                        &mut self.material_use,
                        DeclaredApplicationMaterialUseBasis::ExplicitPermission,
                        "I have permission to review it",
                    );
                });
                ui.label(RichText::new("Your role").strong());
                ui.horizontal(|ui| {
                    ui.radio_value(
                        &mut self.role,
                        DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
                        "I own or manage this material",
                    );
                    ui.radio_value(
                        &mut self.role,
                        DeclaredSessionOperatorRole::DeclaredAuthorizedHumanReviewer,
                        "I am an authorized reviewer",
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Reviewer name");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.operator_label)
                            .id(egui::Id::new("setup-operator-label"))
                            .hint_text("Your name")
                            .desired_width(400.0),
                    );
                });
                ui.small(
                    "These choices are your declaration only. VoxProof does not verify identity \
                     or legal permission.",
                );
            });

            ui.add_space(10.0);
            egui::CollapsingHeader::new(SETUP_ADVANCED)
                .id_salt("setup-advanced")
                .default_open(false)
                .show(ui, |ui| self.seed_terminology_editor(ui));

            ui.add_space(16.0);
            ui.separator();
            ui.heading("Recent reviews");
            if ui.button("Refresh").clicked() {
                match self.controller.refresh_available_sessions() {
                    Ok(()) => self.error = None,
                    Err(error) => self.error = Some(user_errors::user_message(&error)),
                }
            }
            let available = self.controller.available_sessions().to_vec();
            if available.is_empty() {
                ui.label("No saved reviews yet.");
            } else {
                let mut groups: Vec<(String, Vec<crate::controller::SessionResumeView>)> =
                    Vec::new();
                for session in available {
                    let key = session
                        .project_display_name
                        .clone()
                        .unwrap_or_else(|| "Not in a project".to_owned());
                    if let Some((_, sessions)) = groups.iter_mut().find(|(name, _)| *name == key) {
                        sessions.push(session);
                    } else {
                        groups.push((key, vec![session]));
                    }
                }
                for (project_name, sessions) in groups {
                    ui.add_space(6.0);
                    ui.label(RichText::new(project_name).strong());
                    for session in &sessions {
                        ui.group(|ui| {
                            ui.label(RichText::new(&session.source_display_name).strong());
                            ui.small(format!(
                                "{} · {}",
                                Self::format_session_date(session.created_at_unix_ms),
                                session.authority_display_label
                            ));
                            if self.resume_writer_held_session_id.as_deref()
                                == Some(session.session_id.as_str())
                            {
                                ui.label("This review is currently open for editing elsewhere.");
                                ui.horizontal(|ui| {
                                    if ui.button("Open read-only").clicked() {
                                        self.resume_session_id_draft = session.session_id.clone();
                                        self.controller
                                            .select_resume_session_id(session.session_id.clone());
                                        match self.controller.open_selected_session_read_only() {
                                            Ok(()) => {
                                                self.resume_writer_held_session_id = None;
                                                self.error = None;
                                                self.status = format!(
                                                    "Opened {} read-only.",
                                                    session.source_display_name
                                                );
                                            }
                                            Err(error) => {
                                                self.error =
                                                    Some(user_errors::user_message(&error));
                                            }
                                        }
                                    }
                                    if ui.button("Cancel").clicked() {
                                        self.resume_writer_held_session_id = None;
                                        self.error = None;
                                    }
                                });
                            } else {
                                ui.horizontal(|ui| {
                                    if ui.button("Continue").clicked() {
                                        self.resume_session_id_draft = session.session_id.clone();
                                        self.resume_writer_held_session_id = None;
                                        self.controller
                                            .select_resume_session_id(session.session_id.clone());
                                        match self.controller.open_selected_session_writable() {
                                        Ok(()) => {
                                            self.error = None;
                                            self.status = format!(
                                                "Opened {}.",
                                                session.source_display_name
                                            );
                                        }
                                        Err(
                                            crate::controller::ControllerError::WriterOwnershipHeld,
                                        ) => {
                                            self.error = None;
                                            self.resume_writer_held_session_id =
                                                Some(session.session_id.clone());
                                        }
                                        Err(error) => {
                                            self.error =
                                                Some(user_errors::user_message(&error));
                                        }
                                    }
                                    }
                                });
                            }
                        });
                    }
                }
            }

            ui.add_space(12.0);
            let project_ready = setup_project_ready(
                self.review_without_project,
                self.controller.selected_project_id().is_some(),
                &self.new_project_name,
            );
            let can_start =
                setup_can_start(&self.transcript_path, &self.operator_label, project_ready);
            if ui
                .add_enabled(can_start, egui::Button::new("Start review"))
                .on_hover_text("Create a new local review session")
                .clicked()
            {
                let transcript = PathBuf::from(self.transcript_path.trim());
                let transcript_text = match std::fs::read_to_string(&transcript) {
                    Ok(text) => text,
                    Err(error) => {
                        self.error = Some(user_errors::user_message(
                            &crate::controller::ControllerError::Io(error),
                        ));
                        return;
                    }
                };
                let terms = match self.terms_editor.to_session_entries() {
                    Ok(entries) => entries,
                    Err(error) => {
                        self.error = Some(user_errors::user_message(
                            &crate::controller::ControllerError::SessionTerms(error),
                        ));
                        return;
                    }
                };
                let start_result = if self.review_without_project {
                    self.controller.start_from_transcript_and_terms(
                        &transcript,
                        &transcript_text,
                        terms,
                        self.material_use,
                        self.role,
                        &self.operator_label,
                    )
                } else {
                    let project_id = if let Some(id) = self.controller.selected_project_id() {
                        Ok(id.clone())
                    } else {
                        self.controller.create_project(self.new_project_name.trim())
                    };
                    match project_id {
                        Ok(project_id) => {
                            self.controller.start_from_transcript_and_terms_in_project(
                                &transcript,
                                &transcript_text,
                                terms,
                                self.material_use,
                                self.role,
                                &self.operator_label,
                                &project_id,
                            )
                        }
                        Err(error) => Err(error),
                    }
                };
                match start_result {
                    Ok(()) => {
                        self.error = None;
                        self.status = "Review started.".to_owned();
                    }
                    Err(error) => self.error = Some(user_errors::user_message(&error)),
                }
            }
        });
    }

    fn seed_terminology_editor(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new(SETUP_SEED_TERMINOLOGY).strong());
        ui.label(SETUP_SEED_HINT);
        if self.terms_editor.is_empty() {
            ui.small("None added. Review can start without seeded terminology.");
        }
        let mut remove_index = None;
        for index in 0..self.terms_editor.terms.len() {
            let term = &mut self.terms_editor.terms[index];
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(format!("Term {}", index + 1));
                if ui.button("Remove").clicked() {
                    remove_index = Some(index);
                }
            });
            ui.horizontal(|ui| {
                ui.label("Name");
                ui.add(
                    egui::TextEdit::singleline(&mut term.canonical)
                        .id(egui::Id::new(("term-canonical", index)))
                        .desired_width(420.0)
                        .hint_text("PostgreSQL"),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Aliases");
                ui.add(
                    egui::TextEdit::singleline(&mut term.aliases)
                        .id(egui::Id::new(("term-aliases", index)))
                        .desired_width(420.0)
                        .hint_text("Postgres"),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Known misrecognitions");
                ui.add(
                    egui::TextEdit::singleline(&mut term.observed_error_forms)
                        .id(egui::Id::new(("term-errors", index)))
                        .desired_width(420.0)
                        .hint_text("post gray SQL"),
                );
            });
        }
        if let Some(index) = remove_index {
            self.terms_editor.remove_term(index);
        }
        ui.horizontal(|ui| {
            if ui.button("+ Add term").clicked() {
                self.terms_editor.push_empty_term();
            }
            if ui.button("Import terms file…").clicked()
                && let Some(path) = rfd::FileDialog::new().pick_file()
                && let Ok(text) = std::fs::read_to_string(path)
            {
                match self.terms_editor.import_from_text(&text) {
                    Ok(()) => self.error = None,
                    Err(error) => {
                        self.error = Some(user_errors::user_message(
                            &crate::controller::ControllerError::SessionTerms(error),
                        ));
                    }
                }
            }
        });
    }

    fn format_session_date(unix_ms: i64) -> String {
        let millis = unix_ms.max(0) as u64;
        let secs = millis / 1000;
        let mut days = (secs / 86_400) as i32;
        let mut year = 1970i32;
        while days >= days_in_year(year) {
            days -= days_in_year(year);
            year += 1;
        }
        let leap = is_leap_year(year);
        let month_lengths = [
            31,
            if leap { 29 } else { 28 },
            31,
            30,
            31,
            30,
            31,
            31,
            30,
            31,
            30,
            31,
        ];
        let months = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];
        let mut month = 0usize;
        while month < 12 && days >= month_lengths[month] {
            days -= month_lengths[month];
            month += 1;
        }
        format!("{} {}", months[month], days + 1)
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn days_in_year(year: i32) -> i32 {
    if is_leap_year(year) { 366 } else { 365 }
}

impl ReviewApp {
    fn recovery(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_max_width(860.0);
            ui.add_space(20.0);
            ui.heading("Recovery needed");
            ui.label(
                "Your last change was saved, but this review could not be reopened cleanly. \
                 Try again or close the review.",
            );
            ui.add_space(12.0);
            if ui.button("Retry recovery").clicked() {
                match self.controller.retry_recovery() {
                    Ok(()) => {
                        self.error = None;
                        self.status = "Review reopened.".to_owned();
                    }
                    Err(error) => self.error = Some(user_errors::user_message(&error)),
                }
            }
            if ui.button("Close session").clicked() {
                self.reset();
            }
        });
    }

    fn review(&mut self, ui: &mut egui::Ui) {
        let items = match self.controller.items() {
            Ok(items) => items,
            Err(error) => {
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.colored_label(Color32::LIGHT_RED, user_errors::user_message(&error));
                });
                return;
            }
        };
        let progress = match self.controller.progress() {
            Ok(progress) => progress,
            Err(error) => {
                self.error = Some(user_errors::user_message(&error));
                return;
            }
        };

        let canonical_total = self
            .controller
            .header()
            .map(|header| header.total_review_cases)
            .unwrap_or(0);
        if self.bottom_tab == BottomTab::ProjectMemory && !self.controller.is_bound_to_project() {
            self.bottom_tab = BottomTab::CurrentPreview;
        }
        egui::Panel::bottom("review-bottom")
            .default_size(235.0)
            .resizable(true)
            .show(ui, |ui| self.bottom_panel(ui, progress, canonical_total));
        egui::Panel::left("review-queue")
            .default_size(300.0)
            .size_range(220.0..=480.0)
            .resizable(true)
            .show(ui, |ui| self.queue(ui, &items));
        egui::CentralPanel::default().show(ui, |ui| self.case_detail(ui, &items, progress));
    }

    fn queue(&mut self, ui: &mut egui::Ui, items: &[crate::controller::ReviewItemView]) {
        ui.heading("Review queue");
        ui.add(
            egui::TextEdit::singleline(&mut self.search)
                .id(egui::Id::new(SEARCH_ID))
                .hint_text("Search items"),
        );
        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt("review-queue-scroll")
            .show(ui, |ui| {
                if items.is_empty() {
                    ui.label("No review cases were raised.");
                }
                for (index, item) in items.iter().enumerate() {
                    if !search_matches(&item.source_text, &item.evidence, &self.search) {
                        continue;
                    }
                    let selected = index == self.controller.selected_index();
                    let label = match item.origin {
                        ReviewItemOrigin::PreviousCorrection { .. } => format!(
                            "Item {} · Previous correction · {}\n{}",
                            item.queue_index + 1,
                            item.status,
                            item.source_text
                        ),
                        ReviewItemOrigin::HumanRaisedCorrection => format!(
                            "Item {} · Your correction · {}\n{}",
                            item.queue_index + 1,
                            item.status,
                            item.source_text
                        ),
                        ReviewItemOrigin::TermSuggestion { .. } => format!(
                            "Item {} · {}\n{}",
                            item.queue_index + 1,
                            item.status,
                            item.source_text
                        ),
                    };
                    if ui
                        .selectable_label(selected, label)
                        .on_hover_text(format!("Select review item {}", item.queue_index + 1))
                        .clicked()
                    {
                        self.controller.select(index);
                        self.selected_alternative = 0;
                        self.manual_replacement_draft.clear();
                    }
                }
            });
    }

    fn case_detail(
        &mut self,
        ui: &mut egui::Ui,
        items: &[crate::controller::ReviewItemView],
        progress: vox_proof::application_service::ApplicationReviewProgress,
    ) {
        let header = match self.controller.header() {
            Ok(header) => header,
            Err(error) => {
                self.error = Some(user_errors::user_message(&error));
                return;
            }
        };
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(&header.source_path).strong());
            ui.separator();
            ui.label(format!("Reviewer: {}", header.declared_operator));
            if let Some(project_name) = &header.project_name {
                ui.separator();
                ui.label(format!("Project: {project_name}"));
            }
        });
        if let Some(note) = &header.source_path_note {
            ui.small(note);
        }
        if header.bound_to_project && !header.project_memory_available {
            ui.colored_label(
                Color32::YELLOW,
                "Project Memory isn't available. You can still inspect this saved review, but new reuse suggestions can't be applied.",
            );
        }
        if self.controller.is_read_only() {
            ui.colored_label(Color32::YELLOW, "This review is open read-only.");
        }
        let previous_waiting = items
            .iter()
            .filter(|item| {
                item.status == "Needs review"
                    && matches!(item.origin, ReviewItemOrigin::PreviousCorrection { .. })
            })
            .count();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(composed_coverage_label(
                    progress,
                    header.total_review_cases,
                    previous_waiting,
                ))
                .strong(),
            );
            if review_is_complete(progress) {
                ui.separator();
                ui.label(resolution_label(progress));
            } else {
                ui.separator();
                let resolution = resolution_label(progress);
                if unresolved_confirmation_needed(progress) {
                    ui.colored_label(Color32::YELLOW, resolution);
                } else {
                    ui.label(resolution);
                }
            }
        });

        if show_review_complete_panel(progress, previous_waiting) {
            self.completion_panel(ui, &header, progress);
        }

        ui.separator();

        let Some(item) = items.get(self.controller.selected_index()) else {
            ui.heading("No items to review");
            ui.label(
                "Nothing needed your decision in this file. You can still preview and export the \
                 unchanged subtitles, or correct unflagged text below.",
            );
            self.human_raise_panel(ui);
            self.export_controls(ui, progress);
            return;
        };

        if !show_review_complete_panel(progress, previous_waiting) {
            ui.heading(format!("Item {}", item.queue_index + 1));
        }
        ui.label(format!("Status: {}", item.status));
        match item.origin {
            ReviewItemOrigin::PreviousCorrection {
                conflict_with_canonical,
            } => {
                ui.label(RichText::new("Previously confirmed in this project").strong());
                ui.label(
                    "Based on a correction approved in an earlier review. This is a suggestion, \
                     not an automatic change.",
                );
                if conflict_with_canonical {
                    ui.colored_label(
                        Color32::YELLOW,
                        "A term check on this same text suggests a different replacement. \
                         Choose which text should appear.",
                    );
                }
            }
            ReviewItemOrigin::HumanRaisedCorrection => {
                ui.label(RichText::new("You raised this text yourself").strong());
                ui.label("No term check or previous correction flagged this span.");
            }
            ReviewItemOrigin::TermSuggestion {
                also_supported_by_previous_correction,
                disagrees_with_previous_correction,
            } => {
                ui.label(format!("Found by: {}", item.detector));
                if also_supported_by_previous_correction {
                    ui.label("Also supported by a previous project correction.");
                }
                if disagrees_with_previous_correction {
                    ui.colored_label(
                        Color32::YELLOW,
                        "A previous project correction suggests a different replacement for this text.",
                    );
                }
            }
        }
        ui.label(RichText::new(format!("Why flagged: {}", item.evidence)).strong());
        ui.add_space(8.0);
        if let Some(before) = &item.context_before {
            ui.label(RichText::new(format!("Previous: {before}")).weak());
        }
        ui.group(|ui| {
            ui.label(RichText::new("Current subtitle text").strong());
            ui.label(&item.source_text);
        });
        if let Some(after) = &item.context_after {
            ui.label(RichText::new(format!("Next: {after}")).weak());
        }
        ui.add_space(8.0);
        ui.label(RichText::new("Suggested corrections").strong());
        if item.alternatives.is_empty() {
            ui.label("No suggested correction is available.");
        }
        for (index, alternative) in item.alternatives.iter().enumerate() {
            ui.radio_value(
                &mut self.selected_alternative,
                index,
                format!("{}. {alternative}", index + 1),
            );
        }
        ui.add_space(8.0);
        let mutations_enabled = self.controller.mutations_enabled() && !item.reuse_decision_blocked;
        if item.reuse_decision_blocked {
            ui.colored_label(
                Color32::YELLOW,
                "Project Memory isn't available, so this previous-correction suggestion can't be decided yet.",
            );
        }
        ui.group(|ui| {
            ui.label(RichText::new("Correct text").strong());
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.manual_replacement_draft)
                        .id(egui::Id::new("manual-replacement-field"))
                        .desired_width(420.0)
                        .hint_text("Type the corrected line"),
                );
                if ui
                    .add_enabled(mutations_enabled, egui::Button::new("Edit"))
                    .on_hover_text("Save your own replacement for this item")
                    .clicked()
                {
                    self.apply_manual_replacement();
                }
            });
        });
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    mutations_enabled
                        && accept_enabled(item.alternatives.len(), self.selected_alternative),
                    egui::Button::new("Accept"),
                )
                .clicked()
            {
                self.apply_decision(CorrectionDecision::AcceptAlternative {
                    alternative_index: self.selected_alternative,
                });
            }
            if ui
                .add_enabled(mutations_enabled, egui::Button::new("Reject"))
                .clicked()
            {
                self.apply_decision(CorrectionDecision::Reject);
            }
            if ui
                .add_enabled(mutations_enabled, egui::Button::new("Defer"))
                .clicked()
            {
                self.apply_decision(CorrectionDecision::Defer);
            }
        });
        if let Ok(Some(_)) = self.controller.promotion_candidate_for_selected()
            && ui
                .add_enabled(
                    self.controller.mutations_enabled()
                        && self.controller.project_memory_available(),
                    egui::Button::new("Use this correction in related reviews"),
                )
                .on_hover_text("Save this approved correction to Project Memory")
                .clicked()
        {
            match self
                .controller
                .use_selected_correction_in_related_reviews(self.controller.ui_session_epoch())
            {
                Ok(()) => {
                    self.error = None;
                    self.status = "Saved to Project Memory for related reviews.".to_owned();
                }
                Err(error) => self.error = Some(user_errors::user_message(&error)),
            }
        }
        self.human_raise_panel(ui);
        self.export_controls(ui, progress);
    }

    fn completion_panel(
        &mut self,
        ui: &mut egui::Ui,
        header: &crate::controller::SessionHeaderView,
        progress: vox_proof::application_service::ApplicationReviewProgress,
    ) {
        ui.group(|ui| {
            ui.heading("Review complete");
            ui.label(format!("{} items reviewed", header.total_review_cases));
            ui.label(format!("{} accepted", header.accepted));
            ui.label(format!("{} rejected", header.rejected));
            if header.manual_replacements > 0 {
                ui.label(format!("{} manually corrected", header.manual_replacements));
            }
            if header.deferred > 0 || header.needs_manual_correction > 0 {
                ui.label(resolution_label(progress));
            }
            ui.horizontal(|ui| {
                if ui.button("Preview reviewed subtitles").clicked() {
                    self.bottom_tab = BottomTab::CurrentPreview;
                }
            });
        });
        ui.add_space(8.0);
    }

    fn project_memory_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        ui.separator();
        ui.heading("Project Memory");
        if !self.controller.is_bound_to_project() {
            ui.label("Open a project review to see corrections saved for reuse.");
            return;
        }
        ui.label("These are corrections you explicitly allowed VoxProof to reuse.");
        if self.controller.is_bound_to_project() && !self.controller.project_memory_available() {
            ui.colored_label(
                Color32::YELLOW,
                "Project Memory isn't available. Saved review decisions are still here.",
            );
            return;
        }
        match self.controller.project_memory_entries() {
            Ok(entries) => {
                if entries.is_empty() {
                    ui.label("No reused corrections in this project yet.");
                }
                for entry in entries {
                    ui.group(|ui| {
                        ui.label(format!(
                            "{} → {}",
                            entry.observed_text, entry.confirmed_replacement
                        ));
                        if let Some(source) = &entry.source_review_label {
                            ui.small(format!("From {source}"));
                        }
                        if !entry.provenance_available {
                            ui.small("Provenance currently unverified.");
                        }
                    });
                }
            }
            Err(error) => {
                ui.colored_label(Color32::LIGHT_RED, user_errors::user_message(&error));
            }
        }
    }

    fn export_controls(
        &mut self,
        ui: &mut egui::Ui,
        progress: vox_proof::application_service::ApplicationReviewProgress,
    ) {
        ui.add_space(12.0);
        ui.separator();
        if unresolved_confirmation_needed(progress) {
            ui.checkbox(
                &mut self.confirm_unresolved_source_retained,
                "Keep the original subtitle text for unresolved items in the export.",
            );
        }
        let enabled = export_enabled(progress)
            && (!unresolved_confirmation_needed(progress)
                || self.confirm_unresolved_source_retained);
        if ui
            .add_enabled(enabled, egui::Button::new("Export reviewed subtitles…"))
            .clicked()
            && let Some(destination) = rfd::FileDialog::new().pick_folder()
        {
            let ui_session_epoch = self.controller.ui_session_epoch();
            match self.controller.export(
                ui_session_epoch,
                &destination,
                self.confirm_unresolved_source_retained,
            ) {
                Ok(paths) => {
                    self.status = format!(
                        "Export saved to {}.",
                        paths.reviewed_srt.parent().map_or_else(
                            || paths.reviewed_srt.display().to_string(),
                            |parent| parent.display().to_string()
                        )
                    );
                    self.error = None;
                }
                Err(error) => self.error = Some(user_errors::user_message(&error)),
            }
        }
        if self.controller.phase() == DesktopPhase::ExportCompleted {
            ui.colored_label(Color32::LIGHT_GREEN, "Export complete");
            if let Some(paths) = self.controller.exported_paths() {
                ui.label(format!(
                    "Reviewed subtitles: {}",
                    paths.reviewed_srt.display()
                ));
                ui.small(format!("Also saved: {}", paths.decision_log.display()));
                ui.small(format!("Also saved: {}", paths.session_summary.display()));
            }
        }
    }

    fn human_raise_panel(&mut self, ui: &mut egui::Ui) {
        if !self.controller.human_raised_available() {
            return;
        }
        let Ok(cues) = self.controller.cue_texts() else {
            return;
        };
        if cues.is_empty() {
            return;
        }
        if self.human_raise_cue_index >= cues.len() {
            self.human_raise_cue_index = 0;
        }
        let cue_text = cues[self.human_raise_cue_index].1.clone();
        let char_count = cue_text.chars().count();
        if self.human_raise_end_char == 0 && char_count > 0 {
            self.human_raise_end_char = char_count;
        }
        if self.human_raise_end_char > char_count {
            self.human_raise_end_char = char_count;
        }
        if self.human_raise_start_char > self.human_raise_end_char {
            self.human_raise_start_char = self.human_raise_end_char;
        }

        ui.add_space(12.0);
        ui.group(|ui| {
            ui.label(RichText::new("Correct unflagged text").strong());
            ui.label("Select a contiguous span in one subtitle line. This does not add a term.");
            ui.horizontal(|ui| {
                ui.label("Subtitle");
                egui::ComboBox::from_id_salt("human-raise-cue")
                    .selected_text(format!(
                        "Cue {} · {}",
                        cues[self.human_raise_cue_index].0 + 1,
                        truncate_cue(&cue_text)
                    ))
                    .show_ui(ui, |ui| {
                        for (index, (_, text)) in cues.iter().enumerate() {
                            let label = format!("Cue {} · {}", index + 1, truncate_cue(text));
                            if ui
                                .selectable_label(index == self.human_raise_cue_index, label)
                                .clicked()
                            {
                                self.human_raise_cue_index = index;
                                self.human_raise_start_char = 0;
                                self.human_raise_end_char = text.chars().count();
                            }
                        }
                    });
                if ui.button("Whole line").clicked() {
                    self.human_raise_start_char = 0;
                    self.human_raise_end_char = char_count;
                }
            });
            ui.horizontal(|ui| {
                ui.label("From character");
                ui.add(
                    egui::DragValue::new(&mut self.human_raise_start_char)
                        .range(0..=char_count.saturating_sub(1).max(0)),
                );
                ui.label("to");
                ui.add(egui::DragValue::new(&mut self.human_raise_end_char).range(0..=char_count));
            });
            let selected = selected_span_text(
                &cue_text,
                self.human_raise_start_char,
                self.human_raise_end_char,
            );
            ui.label(RichText::new(format!("Selected: {selected}")).italics());
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.human_raise_replacement)
                        .id(egui::Id::new("human-raise-replacement"))
                        .desired_width(420.0)
                        .hint_text("Replacement for the selected span"),
                );
                let enabled = self.controller.mutations_enabled();
                if ui
                    .add_enabled(enabled, egui::Button::new("Correct selected text"))
                    .on_hover_text("Raise this span and save the replacement")
                    .clicked()
                {
                    self.apply_human_raise();
                }
            });
        });
    }

    fn bottom_panel(
        &mut self,
        ui: &mut egui::Ui,
        progress: vox_proof::application_service::ApplicationReviewProgress,
        total: usize,
    ) {
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.bottom_tab,
                BottomTab::CurrentPreview,
                "Preview reviewed subtitles",
            );
            ui.selectable_value(&mut self.bottom_tab, BottomTab::DecisionLog, "Decision log");
            ui.selectable_value(
                &mut self.bottom_tab,
                BottomTab::SessionSummary,
                "Session summary",
            );
            if self.controller.is_bound_to_project() {
                ui.selectable_value(
                    &mut self.bottom_tab,
                    BottomTab::ProjectMemory,
                    "Project Memory",
                );
            }
        });
        ui.separator();
        if self.bottom_tab == BottomTab::ProjectMemory {
            self.project_memory_panel(ui);
            return;
        }
        let text = match self.bottom_tab {
            BottomTab::CurrentPreview => self
                .controller
                .projection()
                .map(|projection| projection.srt)
                .unwrap_or_else(|error| error.to_string()),
            BottomTab::DecisionLog => self
                .controller
                .export_previews(self.confirm_unresolved_source_retained)
                .map(|(log, _)| log)
                .unwrap_or_else(|error| error.to_string()),
            BottomTab::SessionSummary => self
                .controller
                .export_previews(self.confirm_unresolved_source_retained)
                .map(|(_, summary)| summary)
                .unwrap_or_else(|error| {
                    format!(
                        "{}\n{}\n{}",
                        coverage_label(progress, total),
                        resolution_label(progress),
                        error
                    )
                }),
            BottomTab::ProjectMemory => unreachable!("handled above"),
        };
        egui::ScrollArea::both()
            .id_salt("read-only-bottom-projection")
            .show(ui, |ui| {
                let mut read_only = text;
                ui.add(
                    egui::TextEdit::multiline(&mut read_only)
                        .id(egui::Id::new("read-only-projection"))
                        .interactive(false)
                        .desired_width(f32::INFINITY),
                );
            });
    }
}

fn char_range_to_utf8_bytes(
    text: &str,
    start_char: usize,
    end_char: usize,
) -> Option<(usize, usize)> {
    let mut starts: Vec<usize> = text.char_indices().map(|(index, _)| index).collect();
    starts.push(text.len());
    if start_char >= end_char || end_char >= starts.len() {
        return None;
    }
    Some((starts[start_char], starts[end_char]))
}

fn selected_span_text(text: &str, start_char: usize, end_char: usize) -> String {
    text.chars()
        .skip(start_char)
        .take(end_char.saturating_sub(start_char))
        .collect()
}

fn truncate_cue(text: &str) -> String {
    const LIMIT: usize = 32;
    let mut truncated = String::new();
    for (index, character) in text.chars().enumerate() {
        if index >= LIMIT {
            truncated.push('…');
            break;
        }
        truncated.push(character);
    }
    truncated
}

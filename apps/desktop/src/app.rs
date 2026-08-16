use std::path::PathBuf;

use eframe::egui;
use egui::{Color32, RichText};
use vox_proof::application_service::{
    DeclaredApplicationMaterialUseBasis, DeclaredSessionOperatorRole,
};
use vox_proof::review::CorrectionDecision;

use crate::controller::{DesktopController, DesktopPhase};
use crate::presentation::{
    BottomTab, accept_enabled, coverage_label, decision_shortcut, export_enabled,
    resolution_label, review_is_complete, review_shortcuts_suppressed, search_matches,
    unresolved_confirmation_needed,
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
    bottom_tab: BottomTab,
    confirm_unresolved_source_retained: bool,
    resume_session_id_draft: String,
    show_advanced: bool,
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
            bottom_tab: BottomTab::CurrentPreview,
            confirm_unresolved_source_retained: false,
            resume_session_id_draft: String::new(),
            show_advanced: false,
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
                self.show_advanced = false;
                self.error = None;
                self.status = "Review closed. Start a new review or continue a recent one."
                    .to_owned();
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
            ui.heading("New review");
            ui.label(
                "Choose a subtitle file, add terms you care about, and review each suggested \
                 change yourself. VoxProof never silently rewrites subtitle text.",
            );
            ui.add_space(12.0);

            ui.group(|ui| {
                ui.label(RichText::new("1. Choose transcript").strong());
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
                ui.label(RichText::new("2. Terms to watch").strong());
                ui.label("Add names or phrases that often get misrecognized in this material.");
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
            });

            ui.add_space(10.0);
            ui.group(|ui| {
                ui.label(RichText::new("3. Confirm use").strong());
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
                for session in &available {
                    ui.group(|ui| {
                        ui.label(RichText::new(&session.source_display_name).strong());
                        ui.small(format!(
                            "{} · {}",
                            Self::format_session_date(session.created_at_unix_ms),
                            session.authority_display_label
                        ));
                        ui.horizontal(|ui| {
                            if ui.button("Continue").clicked() {
                                self.resume_session_id_draft = session.session_id.clone();
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
                                    Err(_) => {
                                        self.controller
                                            .select_resume_session_id(session.session_id.clone());
                                        match self.controller.open_selected_session_read_only() {
                                            Ok(()) => {
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
                                }
                            }
                        });
                    });
                }
            }

            ui.add_space(12.0);
            let can_start = !self.transcript_path.trim().is_empty()
                && !self.operator_label.trim().is_empty();
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
                match self.controller.start_from_transcript_and_terms(
                    &transcript,
                    &transcript_text,
                    terms,
                    self.material_use,
                    self.role,
                    &self.operator_label,
                ) {
                    Ok(()) => {
                        self.error = None;
                        self.status = "Review started.".to_owned();
                    }
                    Err(error) => self.error = Some(user_errors::user_message(&error)),
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

        egui::Panel::bottom("review-bottom")
            .default_size(235.0)
            .resizable(true)
            .show(ui, |ui| self.bottom_panel(ui, progress, items.len()));
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
                    let label = format!(
                        "Item {} · {}\n{}",
                        item.local_index + 1,
                        item.status,
                        item.source_text
                    );
                    if ui
                        .selectable_label(selected, label)
                        .on_hover_text(format!("Select review case {}", item.local_index + 1))
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
        });
        if let Some(note) = &header.source_path_note {
            ui.small(note);
        }
        if self.controller.is_read_only() {
            ui.colored_label(Color32::YELLOW, "This review is open read-only.");
        }
        ui.horizontal(|ui| {
            ui.label(RichText::new(coverage_label(progress, items.len())).strong());
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

        if review_is_complete(progress) {
            self.completion_panel(ui, &header, progress);
        }

        ui.separator();

        let Some(item) = items.get(self.controller.selected_index()) else {
            ui.heading("No items to review");
            ui.label(
                "Nothing needed your decision in this file. You can still preview and export the \
                 unchanged subtitles.",
            );
            self.export_controls(ui, progress);
            return;
        };

        if !review_is_complete(progress) {
            ui.heading(format!("Item {}", item.local_index + 1));
        }
        ui.label(format!("Status: {}", item.status));
        ui.label(format!("Found by: {}", item.detector));
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
        let mutations_enabled = self.controller.mutations_enabled();
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
                    .add_enabled(
                        mutations_enabled,
                        egui::Button::new("Use this correction"),
                    )
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
                    mutations_enabled && accept_enabled(item.alternatives.len(), self.selected_alternative),
                    egui::Button::new("Accept suggestion (A)"),
                )
                .clicked()
            {
                self.apply_decision(CorrectionDecision::AcceptAlternative {
                    alternative_index: self.selected_alternative,
                });
            }
            if ui
                .add_enabled(mutations_enabled, egui::Button::new("Reject (R)"))
                .clicked()
            {
                self.apply_decision(CorrectionDecision::Reject);
            }
            if ui
                .add_enabled(mutations_enabled, egui::Button::new("Defer (D)"))
                .clicked()
            {
                self.apply_decision(CorrectionDecision::Defer);
            }
            if ui
                .add_enabled(
                    mutations_enabled,
                    egui::Button::new("Fix manually (M)"),
                )
                .on_hover_text("Mark this item as needing a manual correction above")
                .clicked()
            {
                self.apply_decision(CorrectionDecision::NeedsManualCorrection);
            }
        });
        if ui
            .button("Advanced — project memory")
            .clicked()
        {
            self.show_advanced = !self.show_advanced;
        }
        if self.show_advanced {
            self.reuse_governance_panel(ui);
        }
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

    fn reuse_governance_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        ui.separator();
        ui.heading("Project memory");
        ui.label("Optional settings for saving corrections across related reviews.");
        ui.horizontal(|ui| {
            ui.label("Stable project ID:");
            ui.add(
                egui::TextEdit::singleline(self.controller.project_scope_id_draft_mut())
                    .desired_width(180.0)
                    .hint_text("opaque project id"),
            );
            ui.label("Display label:");
            ui.add(
                egui::TextEdit::singleline(self.controller.project_scope_display_draft_mut())
                    .desired_width(180.0)
                    .hint_text("presentation label"),
            );
        });
        let ui_session_epoch = self.controller.ui_session_epoch();
        let mutations_enabled = self.controller.mutations_enabled();
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    mutations_enabled,
                    egui::Button::new("Initialize project scope"),
                )
                .clicked()
            {
                match self.controller.initialize_project_scope(ui_session_epoch) {
                    Ok(()) => self.status = "Project scope initialized.".to_owned(),
                    Err(error) => self.error = Some(user_errors::user_message(&error)),
                }
            }
            if self.controller.has_project_scope()
                && ui
                    .add_enabled(
                        mutations_enabled,
                        egui::Button::new("Update display label"),
                    )
                    .clicked()
            {
                match self
                    .controller
                    .update_project_scope_display_name(ui_session_epoch)
                {
                    Ok(()) => self.status = "Display label updated.".to_owned(),
                    Err(error) => self.error = Some(user_errors::user_message(&error)),
                }
            }
        });
        if !self.controller.has_project_scope() {
            ui.label("Gate 2 review remains available without a project scope.");
            return;
        }
        match self.controller.reuse_candidates() {
            Ok(candidates) => {
                ui.label(RichText::new("Promotion candidates").strong());
                if candidates.is_empty() {
                    ui.label("No unpromoted Manual Replacement candidates.");
                }
                for candidate in candidates {
                    ui.group(|ui| {
                        ui.label(format!(
                            "observed: {} → replacement: {}",
                            candidate.exact_payload.observed_text,
                            candidate.exact_payload.confirmed_replacement
                        ));
                        ui.label(format!(
                            "source case local:{} · still effective: {}",
                            candidate
                                .key
                                .source_locator
                                .source_review_case_id
                                .local_index()
                                + 1,
                            candidate.source_decision_still_effective
                        ));
                        let key = candidate.key.clone();
                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(
                                    mutations_enabled,
                                    egui::Button::new("Accept for reuse"),
                                )
                                .clicked()
                            {
                                match self.controller.accept_reuse_candidate(ui_session_epoch, &key) {
                                    Ok(()) => self.status = "Promotion accepted.".to_owned(),
                                    Err(error) => self.error = Some(user_errors::user_message(&error)),
                                }
                            }
                            if ui
                                .add_enabled(
                                    mutations_enabled,
                                    egui::Button::new("Reject promotion candidate"),
                                )
                                .clicked()
                            {
                                match self.controller.reject_reuse_candidate(ui_session_epoch, &key) {
                                    Ok(()) => {
                                        self.status = "Promotion candidate rejected.".to_owned()
                                    }
                                    Err(error) => self.error = Some(user_errors::user_message(&error)),
                                }
                            }
                        });
                    });
                }
            }
            Err(error) => self.error = Some(user_errors::user_message(&error)),
        }
        match (
            self.controller.active_reusable_records(),
            self.controller.reuse_candidates(),
        ) {
            (Ok(records), Ok(candidates)) => {
                ui.label(RichText::new("Active reusable records").strong());
                if records.is_empty() {
                    ui.label("No active reusable records.");
                }
                for record in records {
                    ui.group(|ui| {
                        ui.label(format!(
                            "record promotion_event:{} · {} → {}",
                            record.record_id.promotion_event_index(),
                            record.payload.observed_text,
                            record.payload.confirmed_replacement
                        ));
                        if !record.source_decision_still_effective {
                            ui.colored_label(
                                Color32::YELLOW,
                                "Source decision no longer effective; revoke or supersede explicitly.",
                            );
                        }
                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(mutations_enabled, egui::Button::new("Revoke"))
                                .clicked()
                            {
                                match self.controller.revoke_reusable_influence(
                                    ui_session_epoch,
                                    record.record_id,
                                ) {
                                    Ok(()) => self.status = "Record revoked.".to_owned(),
                                    Err(error) => self.error = Some(user_errors::user_message(&error)),
                                }
                            }
                        let eligible: Vec<_> = candidates
                            .iter()
                            .filter(|candidate| {
                                candidate.key.source_locator != record.source_locator
                            })
                            .collect();
                        for candidate in eligible {
                            let key = candidate.key.clone();
                            let label = format!(
                                "Supersede with: {} → {} (case local:{}, digest:{:08x}…)",
                                candidate.exact_payload.observed_text,
                                candidate.exact_payload.confirmed_replacement,
                                candidate
                                    .key
                                    .source_locator
                                    .source_review_case_id
                                    .local_index()
                                    + 1,
                                u32::from_be_bytes(
                                    candidate.key.source_locator.decision_digest[..4]
                                        .try_into()
                                        .unwrap_or([0; 4]),
                                )
                            );
                            if ui
                                .add_enabled(mutations_enabled, egui::Button::new(label))
                                .clicked()
                            {
                                match self.controller.supersede_reusable_influence(
                                    ui_session_epoch,
                                    record.record_id,
                                    &key,
                                ) {
                                    Ok(()) => self.status = "Record superseded.".to_owned(),
                                    Err(error) => self.error = Some(user_errors::user_message(&error)),
                                }
                            }
                        }
                        });
                    });
                }
            }
            (Err(error), _) | (_, Err(error)) => {
                self.error = Some(user_errors::user_message(&error));
            }
        }
        if ui
            .add_enabled(
                mutations_enabled,
                egui::Button::new("Run reuse-enabled exact analysis"),
            )
            .clicked()
        {
            match self.controller.run_reuse_enabled_analysis(ui_session_epoch) {
                Ok(count) => {
                    self.status =
                        format!("Reuse-enabled analysis raised {count} non-binding case(s).");
                }
                Err(error) => self.error = Some(user_errors::user_message(&error)),
            }
        }
        if let Some(count) = self.controller.reuse_enabled_case_count() {
            ui.small(format!("Last reuse-enabled analysis case count: {count}"));
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
                ui.label(format!("Reviewed subtitles: {}", paths.reviewed_srt.display()));
                ui.small(format!("Also saved: {}", paths.decision_log.display()));
                ui.small(format!("Also saved: {}", paths.session_summary.display()));
            }
        }
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
        });
        ui.separator();
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

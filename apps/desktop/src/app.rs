use std::path::PathBuf;

use eframe::egui;
use egui::{Color32, RichText};
use vox_proof::application_service::{
    DeclaredApplicationMaterialUseBasis, DeclaredSessionOperatorRole,
};
use vox_proof::review::CorrectionDecision;

use crate::controller::{DesktopController, DesktopPhase};
use crate::presentation::{
    BottomTab, accept_enabled, coverage_label, decision_shortcut, export_enabled, resolution_label,
    review_shortcuts_suppressed, search_matches, unresolved_confirmation_needed,
};

const SEARCH_ID: &str = "review-search-field";

pub struct ReviewApp {
    controller: DesktopController,
    transcript_path: String,
    terms_path: String,
    material_use: DeclaredApplicationMaterialUseBasis,
    role: DeclaredSessionOperatorRole,
    operator_label: String,
    search: String,
    selected_alternative: usize,
    manual_replacement_draft: String,
    bottom_tab: BottomTab,
    confirm_unresolved_source_retained: bool,
    resume_session_id_draft: String,
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
            terms_path: String::new(),
            material_use: DeclaredApplicationMaterialUseBasis::SelfOwned,
            role: DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
            operator_label: String::new(),
            search: String::new(),
            selected_alternative: 0,
            manual_replacement_draft: String::new(),
            bottom_tab: BottomTab::CurrentPreview,
            confirm_unresolved_source_retained: false,
            resume_session_id_draft: String::new(),
            error: None,
            status: "Choose an SRT and session-terms file to begin.".to_owned(),
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
                    "Session changed after export. The prior files remain on disk but do not \
                     represent the current session; export again to produce current outputs."
                        .to_owned()
                } else {
                    "Decision durably committed.".to_owned()
                };
                self.selected_alternative = 0;
                self.manual_replacement_draft.clear();
            }
            Err(error) => self.error = Some(error.to_string()),
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
                    "Session changed after export. The prior files remain on disk but do not \
                     represent the current session; export again to produce current outputs."
                        .to_owned()
                } else {
                    "Manual Replacement durably committed.".to_owned()
                };
                self.selected_alternative = 0;
                self.manual_replacement_draft.clear();
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn reset(&mut self) {
        match self.controller.reset() {
            Ok(()) => {
                self.transcript_path.clear();
                self.terms_path.clear();
                self.operator_label.clear();
                self.material_use = DeclaredApplicationMaterialUseBasis::SelfOwned;
                self.role = DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator;
                self.search.clear();
                self.selected_alternative = 0;
                self.manual_replacement_draft.clear();
                self.bottom_tab = BottomTab::CurrentPreview;
                self.confirm_unresolved_source_retained = false;
                self.error = None;
                self.status = "Session closed. Choose inputs to begin again.".to_owned();
            }
            Err(error) => self.error = Some(error.to_string()),
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
            ui.heading("VoxProof v0.2 Native Review");
            ui.separator();
            ui.label("egui 0.35.0 · local durable session");
            ui.separator();
            ui.label("繁體中文 / 简体中文 / 日本語");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.controller.phase() != DesktopPhase::Setup
                    && ui.button("Reset session").clicked()
                {
                    self.reset();
                }
                ui.small(format!("ui_session_epoch {}", self.controller.ui_session_epoch()));
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
            ui.heading("Start governed transcript review");
            ui.label(
                "Select an existing SRT transcript and session-terms file. VoxProof will raise \
                 review cases but will never silently rewrite transcript text.",
            );
            ui.add_space(12.0);

            ui.group(|ui| {
                ui.label(RichText::new("Input files").strong());
                ui.horizontal(|ui| {
                    ui.label("SRT transcript");
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
                ui.horizontal(|ui| {
                    ui.label("Session terms");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.terms_path)
                            .id(egui::Id::new("setup-terms-path"))
                            .desired_width(560.0),
                    );
                    if ui.button("Choose terms…").clicked()
                        && let Some(path) = rfd::FileDialog::new().pick_file()
                    {
                        self.terms_path = path.display().to_string();
                    }
                });
            });

            ui.add_space(10.0);
            ui.group(|ui| {
                ui.label(RichText::new("Required caller declarations").strong());
                ui.label(
                    "These are caller-supplied declarations only. VoxProof does not authenticate \
                     identity, verify permission, or provide legal authorization.",
                );
                ui.horizontal(|ui| {
                    ui.label("Material use");
                    ui.radio_value(
                        &mut self.material_use,
                        DeclaredApplicationMaterialUseBasis::SelfOwned,
                        "Self-owned",
                    );
                    ui.radio_value(
                        &mut self.material_use,
                        DeclaredApplicationMaterialUseBasis::ExplicitPermission,
                        "Explicit permission",
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Operator role");
                    ui.radio_value(
                        &mut self.role,
                        DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
                        "Declared local owner/operator",
                    );
                    ui.radio_value(
                        &mut self.role,
                        DeclaredSessionOperatorRole::DeclaredAuthorizedHumanReviewer,
                        "Declared authorized human reviewer",
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Operator label");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.operator_label)
                            .id(egui::Id::new("setup-operator-label"))
                            .hint_text("Required display label")
                            .desired_width(400.0),
                    );
                });
            });

            ui.add_space(16.0);
            ui.separator();
            ui.heading("Resume local session");
            ui.label(
                "Open an existing durable session from the local session store. Writable open \
                 refuses when another writer holds ownership.",
            );
            if ui.button("Refresh local sessions").clicked() {
                match self.controller.refresh_available_sessions() {
                    Ok(()) => self.error = None,
                    Err(error) => self.error = Some(error.to_string()),
                }
            }
            let available = self.controller.available_session_ids().to_vec();
            if available.is_empty() {
                ui.label("No local sessions found.");
            } else {
                egui::ComboBox::from_id_salt("resume-session-select")
                    .selected_text(if self.resume_session_id_draft.is_empty() {
                        "Select session ID".to_owned()
                    } else {
                        self.resume_session_id_draft.clone()
                    })
                    .show_ui(ui, |ui| {
                        for session_id in &available {
                            ui.selectable_value(
                                &mut self.resume_session_id_draft,
                                session_id.clone(),
                                session_id,
                            );
                        }
                    });
                ui.horizontal(|ui| {
                    let selected = !self.resume_session_id_draft.is_empty();
                    if ui
                        .add_enabled(selected, egui::Button::new("Open writable"))
                        .clicked()
                    {
                        self.controller
                            .select_resume_session_id(self.resume_session_id_draft.clone());
                        match self.controller.open_selected_session_writable() {
                            Ok(()) => {
                                self.error = None;
                                if let Some(session_id) = self.controller.session_id() {
                                    self.status =
                                        format!("Writable session opened ({session_id}).");
                                }
                            }
                            Err(error) => self.error = Some(error.to_string()),
                        }
                    }
                    if ui
                        .add_enabled(selected, egui::Button::new("Open read-only"))
                        .clicked()
                    {
                        self.controller
                            .select_resume_session_id(self.resume_session_id_draft.clone());
                        match self.controller.open_selected_session_read_only() {
                            Ok(()) => {
                                self.error = None;
                                if let Some(session_id) = self.controller.session_id() {
                                    self.status = format!(
                                        "Read-only session opened ({session_id})."
                                    );
                                }
                            }
                            Err(error) => self.error = Some(error.to_string()),
                        }
                    }
                });
            }

            ui.add_space(12.0);
            let can_start = !self.transcript_path.trim().is_empty()
                && !self.terms_path.trim().is_empty()
                && !self.operator_label.trim().is_empty();
            if ui
                .add_enabled(can_start, egui::Button::new("Begin review"))
                .on_hover_text("Create a new local review session")
                .clicked()
            {
                let transcript = PathBuf::from(self.transcript_path.trim());
                let terms = PathBuf::from(self.terms_path.trim());
                match self.controller.start_from_paths(
                    &transcript,
                    &terms,
                    self.material_use,
                    self.role,
                    &self.operator_label,
                ) {
                    Ok(()) => {
                        self.error = None;
                        if let Some(session_id) = self.controller.session_id() {
                            self.status =
                                format!("Review session created ({session_id}).");
                        } else {
                            self.status = "Review session created.".to_owned();
                        }
                    }
                    Err(error) => self.error = Some(error.to_string()),
                }
            }
            ui.small(if self.cjk_font_loaded {
                "System CJK font loaded. No font file is bundled."
            } else {
                "No preferred system CJK font was found; egui default fonts are active."
            });
        });
    }

    fn recovery(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_max_width(860.0);
            ui.add_space(20.0);
            ui.heading("Recovery required");
            ui.label(
                "The last command was durably committed, but the application could not \
                 reconstruct current authority.",
            );
            if let Some(session_id) = self.controller.session_id() {
                ui.label(format!("Session ID: {session_id}"));
            }
            ui.add_space(12.0);
            if ui.button("Retry recovery").clicked() {
                match self.controller.retry_recovery() {
                    Ok(()) => {
                        self.error = None;
                        self.status = "Session recovered from durable authority.".to_owned();
                    }
                    Err(error) => self.error = Some(error.to_string()),
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
                    ui.colored_label(Color32::LIGHT_RED, error.to_string());
                });
                return;
            }
        };
        let progress = match self.controller.progress() {
            Ok(progress) => progress,
            Err(error) => {
                self.error = Some(error.to_string());
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
                .hint_text("Search cases (⌘/Ctrl+L)"),
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
                        "Case {} · cue {}\n{} · {}",
                        item.local_index + 1,
                        item.cue_index,
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
                self.error = Some(error.to_string());
                return;
            }
        };
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(format!("Session ID: {}", header.session_id)).strong());
            ui.separator();
            ui.label(RichText::new(format!("Access: {}", header.access_mode)).strong());
            ui.separator();
            ui.label(RichText::new(format!("Source: {}", header.source_path)).strong());
            ui.separator();
            ui.label(format!(
                "Declared operator: {} ({})",
                header.declared_operator, header.declared_role
            ));
        });
        if let Some(note) = &header.source_path_note {
            ui.small(note);
        }
        if self.controller.is_read_only() {
            ui.colored_label(Color32::YELLOW, "Read-only session");
        }
        ui.small(format!("Source revision: {}", header.source_revision));
        ui.horizontal(|ui| {
            ui.label(RichText::new(coverage_label(progress, items.len())).strong());
            ui.separator();
            let resolution = resolution_label(progress);
            if unresolved_confirmation_needed(progress) {
                ui.colored_label(Color32::YELLOW, resolution);
            } else {
                ui.label(resolution);
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("Cases: {}", header.total_review_cases));
            ui.label(format!("Events: {}", header.total_recorded_events));
            ui.label(format!("Accepted: {}", header.accepted));
            ui.label(format!(
                "Manual replacements: {}",
                header.manual_replacements
            ));
            ui.label(format!("Rejected: {}", header.rejected));
            ui.label(format!("Deferred: {}", header.deferred));
            ui.label(format!(
                "Needs manual correction: {}",
                header.needs_manual_correction
            ));
            ui.label(format!("Undecided: {}", header.undecided));
        });
        ui.separator();

        let Some(item) = items.get(self.controller.selected_index()) else {
            ui.heading("No cases raised");
            ui.label(
                "The session is honestly complete and resolved at 0/0. You may export the \
                 unchanged transcript and empty decision evidence.",
            );
            self.export_controls(ui, progress);
            return;
        };

        ui.heading(format!(
            "Case {} · cue {}",
            item.local_index + 1,
            item.cue_index
        ));
        ui.label(format!("Status: {}", item.status));
        ui.label(format!("Detector: {}", item.detector));
        ui.label(RichText::new(format!("Proposal evidence: {}", item.evidence)).strong());
        ui.add_space(8.0);
        if let Some(before) = &item.context_before {
            ui.label(RichText::new(format!("Previous: {before}")).weak());
        }
        ui.group(|ui| {
            ui.label(RichText::new("Source text (immutable)").strong());
            ui.label(&item.source_text);
        });
        if let Some(after) = &item.context_after {
            ui.label(RichText::new(format!("Next: {after}")).weak());
        }
        ui.add_space(8.0);
        ui.label(RichText::new("Non-binding alternatives").strong());
        if item.alternatives.is_empty() {
            ui.label("No replacement alternative is available.");
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
            ui.label(RichText::new("Governed Manual Replacement").strong());
            ui.label(
                "Single-line, exact UTF-8 payload for this existing ReviewCase anchor. \
                 Empty, whitespace-only, control-character, identical, and over-4096-byte \
                 values are refused.",
            );
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.manual_replacement_draft)
                        .id(egui::Id::new("manual-replacement-field"))
                        .desired_width(420.0)
                        .hint_text("Enter exact replacement text"),
                );
                if ui
                    .add_enabled(
                        mutations_enabled,
                        egui::Button::new("Record Manual Replacement"),
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
                    egui::Button::new("Accept alternative (A)"),
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
                    egui::Button::new("Needs manual correction (M)"),
                )
                .on_hover_text(
                    "Records only an unresolved signal; use the governed field above to record text",
                )
                .clicked()
            {
                self.apply_decision(CorrectionDecision::NeedsManualCorrection);
            }
        });
        self.reuse_governance_panel(ui);
        self.export_controls(ui, progress);
    }

    fn reuse_governance_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        ui.separator();
        ui.heading("Reusable influence governance (Gate 3)");
        ui.label(
            "Draft fields below have no authority until explicitly committed. Promotion requires \
             a project scope.",
        );
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
                    Err(error) => self.error = Some(error.to_string()),
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
                    Err(error) => self.error = Some(error.to_string()),
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
                                    Err(error) => self.error = Some(error.to_string()),
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
                                    Err(error) => self.error = Some(error.to_string()),
                                }
                            }
                        });
                    });
                }
            }
            Err(error) => self.error = Some(error.to_string()),
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
                                    Err(error) => self.error = Some(error.to_string()),
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
                                    Err(error) => self.error = Some(error.to_string()),
                                }
                            }
                        }
                        });
                    });
                }
            }
            (Err(error), _) | (_, Err(error)) => self.error = Some(error.to_string()),
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
                Err(error) => self.error = Some(error.to_string()),
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
                "I confirm unresolved cases retain their source text in the reviewed SRT.",
            );
        }
        let enabled = export_enabled(progress)
            && (!unresolved_confirmation_needed(progress)
                || self.confirm_unresolved_source_retained);
        if ui
            .add_enabled(enabled, egui::Button::new("Export three-file bundle…"))
            .on_hover_text("Refuses all existing destination filenames before writing")
            .clicked()
            && let Some(destination) = rfd::FileDialog::new().pick_folder()
        {
            let ui_session_epoch = self.controller.ui_session_epoch();
            match self.controller.export(
                ui_session_epoch,
                &destination,
                self.confirm_unresolved_source_retained,
            ) {
                Ok(_) => {
                    self.status = "Export completed with exclusive file creation.".to_owned();
                    self.error = None;
                }
                Err(error) => self.error = Some(error.to_string()),
            }
        }
        if self.controller.phase() == DesktopPhase::ExportCompleted {
            ui.colored_label(Color32::LIGHT_GREEN, "Export completed");
            if let Some(paths) = self.controller.exported_paths() {
                ui.small(paths.reviewed_srt.display().to_string());
                ui.small(paths.decision_log.display().to_string());
                ui.small(paths.session_summary.display().to_string());
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
                "Preview — not final export",
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

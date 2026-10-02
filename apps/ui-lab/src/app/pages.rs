//! Front-of-house pages: sidebar, Home, Course, and "what it learned".
//! All content is invented demo data; nothing here reads or writes files.

use super::*;

#[derive(Clone, Copy, PartialEq)]
pub enum Scope {
    Course,
    Me,
}

impl Scope {
    pub fn label(self) -> &'static str {
        match self {
            Scope::Course => "這門課",
            Scope::Me => "我自己",
        }
    }
}

pub struct MemEntry {
    pub heard: String,
    pub suggestion: String,
    pub scope: Scope,
    pub source: String,
    pub uses: u32,
    pub enabled: bool,
    pub fresh: bool,
    pub source_issue: Option<usize>,
}

pub fn starter_memory() -> Vec<MemEntry> {
    let mk = |h: &str, s: &str, scope, src: &str, uses, enabled| MemEntry {
        heard: h.into(),
        suggestion: s.into(),
        scope,
        source: src.into(),
        uses,
        enabled,
        fresh: false,
        source_issue: None,
    };
    vec![
        mk(
            "提讀下降",
            "梯度下降",
            Scope::Course,
            "第 1 堂・你確認",
            2,
            true,
        ),
        mk(
            "BackPrepitation",
            "backpropagation",
            Scope::Course,
            "第 1 堂・你確認",
            1,
            true,
        ),
        mk(
            "Fightorge",
            "PyTorch",
            Scope::Me,
            "第 2 堂・你確認",
            0,
            true,
        ),
    ]
}

fn card(ui: &mut Ui, p: &Palette, add: impl FnOnce(&mut Ui)) {
    card_with(ui, p, false, add);
}

fn card_with(
    ui: &mut Ui,
    p: &Palette,
    highlight: bool,
    add: impl FnOnce(&mut Ui),
) -> egui::Response {
    Frame::NONE
        .fill(p.panel)
        .stroke(if highlight {
            Stroke::new(2.5, p.accent)
        } else {
            Stroke::new(1.0, p.line)
        })
        .corner_radius(CornerRadius::same(14))
        .inner_margin(Margin::same(18))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        })
        .response
}

fn section_title(ui: &mut Ui, p: &Palette, text: &str) {
    ui.add_space(18.0);
    ui.label(RichText::new(text).font(bold(18.0)).color(p.text));
    ui.add_space(2.0);
}

fn nav_item(ui: &mut Ui, p: &Palette, label: &str, active: bool, badge: Option<usize>) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
    if active {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(10), p.accent.gamma_multiply(0.14));
    } else if resp.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(10), p.line.gamma_multiply(0.6));
    }
    let font = if active {
        bold(17.0)
    } else {
        FontId::proportional(17.0)
    };
    ui.painter().text(
        rect.left_center() + vec2(14.0, 0.0),
        Align2::LEFT_CENTER,
        label,
        font,
        if active { p.accent } else { p.text },
    );
    if let Some(n) = badge.filter(|n| *n > 0) {
        let c = rect.right_center() - vec2(18.0, 0.0);
        ui.painter().circle_filled(c, 10.0, p.accent);
        ui.painter().text(
            c,
            Align2::CENTER_CENTER,
            n.to_string(),
            FontId::proportional(13.0),
            p.on_accent,
        );
    }
    resp.clicked()
}

fn big_button(ui: &mut Ui, p: &Palette, title: &str, sub: &str, enabled: bool, w: f32) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 92.0), Sense::click());
    let hover = enabled && resp.hovered();
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(14),
        if hover {
            p.accent.gamma_multiply(0.10)
        } else {
            p.panel
        },
    );
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(14),
        Stroke::new(
            if hover { 1.5 } else { 1.0 },
            if hover { p.accent } else { p.line },
        ),
        StrokeKind::Inside,
    );
    let ink = if enabled { p.text } else { p.sub };
    ui.painter().text(
        rect.left_top() + vec2(16.0, 18.0),
        Align2::LEFT_TOP,
        title,
        bold(18.0),
        ink,
    );
    ui.painter().text(
        rect.left_top() + vec2(16.0, 52.0),
        Align2::LEFT_TOP,
        sub,
        FontId::proportional(14.0),
        p.sub,
    );
    if enabled {
        ui.ctx().set_cursor_icon(if resp.hovered() {
            egui::CursorIcon::PointingHand
        } else {
            egui::CursorIcon::Default
        });
    }
    enabled && resp.clicked()
}

impl Lab {
    pub(super) fn started(&self) -> bool {
        self.session_started
    }

    fn mini_progress(&self, ui: &mut Ui, p: &Palette, width: f32) {
        let n = self.lec.issues.len();
        let gap = 3.0;
        let (rect, _) = ui.allocate_exact_size(vec2(width, 8.0), Sense::hover());
        let w = (rect.width() - gap * (n as f32 - 1.0)) / n as f32;
        for i in 0..n {
            let r = Rect::from_min_size(
                pos2(rect.left() + i as f32 * (w + gap), rect.top()),
                vec2(w, rect.height()),
            );
            let fill = match &self.decisions[i] {
                Decision::Pending => p.line,
                Decision::Accepted => p.teacher,
                Decision::Kept => p.kept,
                Decision::Edited(_) => p.accent,
                Decision::Unsure => Color32::from_rgb(0xC2, 0x5B, 0x3A),
            };
            ui.painter().rect_filled(r, CornerRadius::same(4), fill);
        }
    }

    /// Index of the enabled memory entry that explains issue `i`, if any.
    pub(super) fn mem_hit(&self, i: usize) -> Option<crate::matcher::Hit> {
        self.analysis.per_issue.get(i).copied().flatten()
    }

    pub(super) fn mem_for(&self, i: usize) -> Option<usize> {
        self.mem_hit(i).map(|h| h.entry)
    }

    /// Effective tier: remembered spellings outrank the detector's own guess.
    pub(super) fn tier(&self, i: usize) -> Tier {
        if self.mem_for(i).is_some() {
            Tier::Memory
        } else {
            self.lec.issues[i].tier
        }
    }

    pub(super) fn reason(&self, i: usize) -> String {
        match self.mem_hit(i) {
            Some(h) if h.exact => format!(
                "這是你確認過的寫法（{}）。這次出現一模一樣的寫法。",
                self.memory[h.entry].source
            ),
            Some(h) => {
                let e = &self.memory[h.entry];
                format!(
                    "這次寫成「{}」，但唸起來和你在{}改過的「{}」很像，都是「{}」。",
                    self.lec.issues[i].heard, e.source, e.heard, e.suggestion
                )
            }
            None => self.lec.issues[i].reason.to_string(),
        }
    }

    fn open_target(&self) -> Stage {
        if self.started() {
            Stage::Review
        } else {
            Stage::Intro
        }
    }

    // --------------------------------------------------------------- rail

    pub(super) fn rail(&mut self, ui: &mut Ui, acts: &mut Vec<Act>) {
        let p = self.pal();
        Panel::left("rail")
            .exact_size(216.0)
            .resizable(false)
            .frame(
                Frame::NONE
                    .fill(p.panel)
                    .inner_margin(Margin::symmetric(14, 18))
                    .stroke(Stroke::new(1.0, p.line)),
            )
            .show(ui, |ui| {
                ui.label(RichText::new("VoxProof").font(bold(20.0)));
                ui.add_space(14.0);
                let here = self.stage;
                if nav_item(ui, &p, "首頁", here == Stage::Home, None) {
                    acts.push(Act::Go(Stage::Home));
                }
                if nav_item(ui, &p, "課程", here == Stage::Course, None) {
                    acts.push(Act::Go(Stage::Course));
                }
                let fresh = self.memory.iter().filter(|m| m.fresh).count();
                if nav_item(ui, &p, "它學到的", here == Stage::Memory, Some(fresh)) {
                    acts.push(Act::Go(Stage::Memory));
                }
                ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                    Frame::NONE
                        .fill(p.ok.gamma_multiply(if p.dark { 0.5 } else { 0.7 }))
                        .corner_radius(CornerRadius::same(10))
                        .inner_margin(Margin::symmetric(10, 8))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.label(
                                RichText::new("全部在這台電腦上處理")
                                    .size(13.5)
                                    .color(p.text),
                            );
                            ui.label(
                                RichText::new("音檔與逐字稿不會上傳")
                                    .size(12.5)
                                    .color(p.sub),
                            );
                        });
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("我是").color(p.sub).size(14.0));
                        ui.selectable_value(&mut self.role, Role::Student, "學生");
                        ui.selectable_value(&mut self.role, Role::Teacher, "老師");
                    });
                });
            });
    }

    // --------------------------------------------------------------- home

    pub(super) fn home(&mut self, ui: &mut Ui, acts: &mut Vec<Act>) {
        let p = self.pal();
        let dropped = ui.input(|i| i.raw.dropped_files.len());
        if dropped > 0 {
            acts.push(Act::Notice(format!(
                "收到 {dropped} 個檔案。示範版不會讀取或修改它們。"
            )));
        }
        let hovering = ui.input(|i| !i.raw.hovered_files.is_empty());
        egui::CentralPanel::default()
            .frame(
                Frame::NONE
                    .fill(p.bg)
                    .inner_margin(Margin::symmetric(24, 12)),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let w = ui.available_width().min(820.0);
                        ui.vertical_centered(|ui| {
                            ui.set_max_width(w);
                            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                                ui.add_space(26.0);
                                ui.label(RichText::new("今天要處理什麼？").font(bold(32.0)));

                                section_title(ui, &p, "繼續上次的");
                                let left = self.pending();
                                card(ui, &p, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.label(
                                                RichText::new(format!("{}・課堂", self.lec.title))
                                                    .font(bold(19.0)),
                                            );
                                            let msg = if left > 0 {
                                                format!("還有 {left} 處要看")
                                            } else {
                                                "全部處理過了，可以匯出".to_string()
                                            };
                                            ui.label(RichText::new(msg).color(p.sub));
                                            ui.add_space(4.0);
                                            self.mini_progress(ui, &p, 360.0);
                                        });
                                        ui.with_layout(
                                            Layout::right_to_left(Align::Center),
                                            |ui| {
                                                let label = if self.started() {
                                                    "繼續"
                                                } else {
                                                    "開始"
                                                };
                                                let b = Button::new(
                                                    RichText::new(label)
                                                        .font(bold(17.0))
                                                        .color(p.on_accent),
                                                )
                                                .fill(p.accent)
                                                .corner_radius(CornerRadius::same(12))
                                                .min_size(vec2(110.0, 44.0));
                                                if ui.add(b).clicked() {
                                                    acts.push(Act::Go(self.open_target()));
                                                }
                                            },
                                        );
                                    });
                                });
                                ui.add_space(8.0);
                                card(ui, &p, |ui| {
                                    ui.label(
                                        RichText::new("訪談・林小姐（示範）").font(bold(18.0)),
                                    );
                                    ui.label(
                                        RichText::new(
                                            "正在轉成文字　63%　可以先離開，完成時通知你",
                                        )
                                        .color(p.sub),
                                    );
                                    ui.add_space(4.0);
                                    let (r, _) =
                                        ui.allocate_exact_size(vec2(360.0, 8.0), Sense::hover());
                                    ui.painter().rect_filled(r, CornerRadius::same(4), p.line);
                                    ui.painter().rect_filled(
                                        Rect::from_min_size(
                                            r.min,
                                            vec2(r.width() * 0.63, r.height()),
                                        ),
                                        CornerRadius::same(4),
                                        p.accent.gamma_multiply(0.7),
                                    );
                                });

                                section_title(ui, &p, "新增一份");
                                let gap = 12.0;
                                let bw = (w - 2.0 * gap) / 3.0;
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = gap;
                                    big_button(ui, &p, "錄音", "即將推出", false, bw);
                                    if big_button(
                                        ui,
                                        &p,
                                        "匯入錄音或影片",
                                        "在這台電腦上轉成文字",
                                        true,
                                        bw,
                                    ) {
                                        acts.push(Act::Notice(
                                            "示範版不會讀取檔案。正式版會在這裡開始轉成文字。"
                                                .into(),
                                        ));
                                    }
                                    if big_button(ui, &p, "匯入字幕檔", "已經有 SRT 的話", true, bw)
                                    {
                                        acts.push(Act::Notice(
                                            "示範版不會讀取檔案。正式版會在這裡建立審閱。".into(),
                                        ));
                                    }
                                });
                                ui.add_space(8.0);
                                ui.label(
                                    RichText::new("也可以直接把檔案拖進這個視窗。")
                                        .color(p.sub)
                                        .size(14.0),
                                );

                                section_title(ui, &p, "最近完成");
                                for (title, sub) in [
                                    ("機器學習導論・第 2 堂", "12 處已處理・已匯出"),
                                    ("機器學習導論・第 1 堂", "9 處已處理・已匯出"),
                                ] {
                                    card(ui, &p, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.vertical(|ui| {
                                                ui.label(RichText::new(title).font(bold(17.0)));
                                                ui.label(
                                                    RichText::new(sub).color(p.sub).size(14.0),
                                                );
                                            });
                                            ui.with_layout(
                                                Layout::right_to_left(Align::Center),
                                                |ui| {
                                                    if ui.button("查看收據").clicked() {
                                                        acts.push(Act::Notice(
                                                            "示範資料：這堂的收據不在實驗版內。"
                                                                .into(),
                                                        ));
                                                    }
                                                },
                                            );
                                        });
                                    });
                                    ui.add_space(6.0);
                                }
                                ui.add_space(30.0);
                            });
                        });
                    });
            });
        if hovering {
            let rect = ui.max_rect().shrink(10.0);
            ui.painter()
                .rect_filled(rect, CornerRadius::same(18), p.accent.gamma_multiply(0.12));
            ui.painter().rect_stroke(
                rect,
                CornerRadius::same(18),
                Stroke::new(3.0, p.accent),
                StrokeKind::Inside,
            );
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                "放開即可加入",
                bold(26.0),
                p.accent,
            );
        }
    }

    // ------------------------------------------------------------- course

    pub(super) fn course(&mut self, ui: &mut Ui, acts: &mut Vec<Act>) {
        let p = self.pal();
        egui::CentralPanel::default().frame(Frame::NONE.fill(p.bg).inner_margin(Margin::symmetric(24, 12))).show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                let w = ui.available_width().min(820.0);
                ui.vertical_centered(|ui| {
                    ui.set_max_width(w);
                    ui.with_layout(Layout::top_down(Align::Min), |ui| {
                        ui.add_space(26.0);
                        ui.label(RichText::new("機器學習導論").font(bold(32.0)));
                        ui.label(RichText::new(format!("{}　·　4 堂　·　你是{}", self.lec.teacher, if self.role == Role::Student { "學生" } else { "老師" })).color(p.sub));
                        ui.add_space(14.0);
                        let active = self.memory.iter().filter(|m| m.enabled).count();
                        card(ui, &p, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(format!("它已經學會 {active} 個這門課的寫法")).font(bold(17.0)));
                                    ui.label(RichText::new("每一條都來自你確認過的修正，隨時可以停用或刪除。").color(p.sub).size(14.0));
                                });
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ui.button("查看它學到的").clicked() {
                                        acts.push(Act::Go(Stage::Memory));
                                    }
                                });
                            });
                        });

                        section_title(ui, &p, "這門課的每一堂");
                        let left = self.pending();
                        let rows: [(&str, String, u8); 4] = [
                            ("第 1 堂", "已完成・9 處已處理".into(), 0),
                            ("第 2 堂", "已完成・12 處已處理".into(), 0),
                            ("第 3 堂", if left > 0 { format!("審閱中・還有 {left} 處要看") } else { "全部處理過了".into() }, 1),
                            ("第 4 堂", "正在轉成文字　63%".into(), 2),
                        ];
                        for (name, status, kind) in rows {
                            card(ui, &p, |ui| {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.label(RichText::new(name).font(bold(18.0)));
                                        ui.label(RichText::new(status).color(p.sub).size(14.5));
                                    });
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| match kind {
                                        1 => {
                                            let b = Button::new(RichText::new(if self.started() { "繼續" } else { "開始" }).color(p.on_accent))
                                                .fill(p.accent)
                                                .corner_radius(CornerRadius::same(10))
                                                .min_size(vec2(90.0, 38.0));
                                            if ui.add(b).clicked() {
                                                acts.push(Act::Go(self.open_target()));
                                            }
                                        }
                                        0 => {
                                            if ui.button("匯出與收據").clicked() {
                                                acts.push(Act::Notice("示範資料：這堂不在實驗版內。".into()));
                                            }
                                        }
                                        _ => {
                                            ui.label(RichText::new("先去忙吧，好了會通知你").color(p.sub).size(13.5));
                                        }
                                    });
                                });
                            });
                            ui.add_space(6.0);
                        }
                        ui.add_space(8.0);
                        if ui.button("加入一堂").clicked() {
                            acts.push(Act::Notice("示範版不會讀取檔案。".into()));
                        }
                        ui.add_space(30.0);
                    });
                });
            });
        });
    }

    // ------------------------------------------------------------- memory

    pub(super) fn memory_page(&mut self, ui: &mut Ui, acts: &mut Vec<Act>) {
        let p = self.pal();
        let mut remove: Option<usize> = None;
        let filter = self.mem_filter;
        let focus_idx = self.mem_focus;
        // (times it has helped, pending places in this transcript)
        let stats: Vec<(u32, Vec<usize>)> = self
            .memory
            .iter()
            .enumerate()
            .map(|(idx, e)| {
                let ms: Vec<usize> = self
                    .analysis
                    .per_entry
                    .get(idx)
                    .cloned()
                    .unwrap_or_default();
                let used = e.uses
                    + ms.iter()
                        .filter(|&&i| {
                            self.decisions[i] == Decision::Accepted && Some(i) != e.source_issue
                        })
                        .count() as u32;
                let open = ms
                    .into_iter()
                    .filter(|&i| self.decisions[i] == Decision::Pending)
                    .collect();
                (used, open)
            })
            .collect();
        egui::CentralPanel::default().frame(Frame::NONE.fill(p.bg).inner_margin(Margin::symmetric(24, 12))).show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                let w = ui.available_width().min(820.0);
                ui.vertical_centered(|ui| {
                    ui.set_max_width(w);
                    ui.with_layout(Layout::top_down(Align::Min), |ui| {
                        ui.add_space(26.0);
                        ui.label(RichText::new("它學到的").font(bold(32.0)));
                        ui.label(RichText::new("這些是你確認過的寫法。下次遇到一模一樣的、或唸起來很像的，都會優先提醒你。你可以隨時停用、改範圍或刪除；已經匯出的內容不會被改動。").color(p.sub));
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            for (label, f) in [("全部", None), ("這門課", Some(Scope::Course)), ("我自己", Some(Scope::Me))] {
                                if ui.selectable_label(filter == f, label).clicked() {
                                    acts.push(Act::MemFilter(f));
                                }
                            }
                        });
                        ui.add_space(8.0);
                        let mut shown = 0;
                        for (idx, m) in self.memory.iter_mut().enumerate() {
                            if filter.is_some() && Some(m.scope) != filter {
                                continue;
                            }
                            shown += 1;
                            let (used, open) = stats[idx].clone();
                            let r = card_with(ui, &p, focus_idx == Some(idx), |ui| {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.horizontal_wrapped(|ui| {
                                            let sub = if m.enabled { p.sub } else { p.kept };
                                            ui.label(RichText::new(&m.heard).size(19.0).color(sub).strikethrough());
                                            ui.label(RichText::new("→").size(19.0).color(sub));
                                            ui.label(RichText::new(&m.suggestion).font(bold(21.0)).color(if m.enabled { p.text } else { p.kept }));
                                            if m.fresh {
                                                pill(ui, "剛剛新增", p.ok, p.teacher);
                                            }
                                        });
                                        let uses = if used == 0 { "還沒幫上忙".to_string() } else { format!("已經幫你改對 {used} 次") };
                                        ui.label(RichText::new(format!("{}　·　{}", m.source, uses)).size(14.0).color(p.sub));
                                        if !open.is_empty() {
                                            if m.enabled {
                                                if ui.link(RichText::new(format!("第 3 堂還有 {} 處用得到，去看看", open.len())).size(14.0).color(p.accent).underline()).clicked() {
                                                    acts.push(Act::OpenIssue(open[0]));
                                                }
                                            } else {
                                                ui.label(RichText::new(format!("已停用：第 3 堂的 {} 處回到一般提示", open.len())).size(14.0).color(p.ai));
                                            }
                                        }
                                    });
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if ui.button("刪除").clicked() {
                                            remove = Some(idx);
                                        }
                                        egui::ComboBox::from_id_salt(("scope", idx))
                                            .width(96.0)
                                            .selected_text(m.scope.label())
                                            .show_ui(ui, |ui| {
                                                ui.selectable_value(&mut m.scope, Scope::Course, "這門課");
                                                ui.selectable_value(&mut m.scope, Scope::Me, "我自己");
                                            });
                                        ui.checkbox(&mut m.enabled, "啟用");
                                    });
                                });
                            });
                            if focus_idx == Some(idx) {
                                r.scroll_to_me(Some(Align::Center));
                            }
                            ui.add_space(6.0);
                        }
                        if shown == 0 {
                            card(ui, &p, |ui| {
                                ui.label(RichText::new("這裡還沒有東西").font(bold(17.0)));
                                ui.label(RichText::new("審閱時接受一個建議，它會問你要不要記住這個寫法。").color(p.sub));
                            });
                        }
                        ui.add_space(30.0);
                    });
                });
            });
        });
        if let Some(i) = remove {
            acts.push(Act::MemDelete(i));
        }
    }

    // ------------------------------------------------------- learn prompt

    pub(super) fn learn_prompt(&mut self, ui: &mut Ui, acts: &mut Vec<Act>) {
        let Some(i) = self.learn else { return };
        let p = self.pal();
        let is = &self.lec.issues[i];
        Frame::NONE
            .fill(p.accent.gamma_multiply(if p.dark { 0.25 } else { 0.10 }))
            .corner_radius(CornerRadius::same(12))
            .inner_margin(Margin::symmetric(14, 8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "記住「{} → {}」，以後也這樣提醒你？",
                            is.heard, is.suggestion
                        ))
                        .font(bold(15.5)),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("不用").clicked() {
                            acts.push(Act::Learn(None));
                        }
                        if ui.button("我自己").clicked() {
                            acts.push(Act::Learn(Some(Scope::Me)));
                        }
                        let b =
                            Button::new(RichText::new("這門課").color(p.on_accent)).fill(p.accent);
                        if ui.add(b).clicked() {
                            acts.push(Act::Learn(Some(Scope::Course)));
                        }
                    });
                });
            });
        ui.add_space(8.0);
    }
}

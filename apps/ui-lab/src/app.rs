use crate::data::{self, Lecture, Tier};
use crate::theme::{self, BOLD, Palette};
use egui::text::{LayoutJob, TextFormat};
pub mod pages;
use crate::matcher::{self, Analysis, Hit};
use egui::{
    Align, Align2, Button, Color32, Context, CornerRadius, FontFamily, FontId, Frame, Key, Layout,
    Margin, Modifiers, Panel, Rect, RichText, Sense, Shape, Stroke, StrokeKind, Ui, pos2, vec2,
};
use pages::{MemEntry, Scope};

#[derive(Clone, PartialEq, Debug)]
enum Decision {
    Pending,
    Accepted,
    Kept,
    Edited(String),
    Unsure,
}

impl Decision {
    fn word(&self) -> &'static str {
        match self {
            Decision::Pending => "尚未處理",
            Decision::Accepted => "接受",
            Decision::Kept => "保留原樣",
            Decision::Edited(_) => "自己改",
            Decision::Unsure => "不確定",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Role {
    Student,
    Teacher,
}

#[derive(Clone, Copy, PartialEq)]
enum Stage {
    Home,
    Course,
    Memory,
    Intro,
    Review,
}

enum Act {
    Start,
    Decide(Decision),
    DecideMany(Vec<usize>, Decision),
    Select(usize),
    Step(i32),
    TogglePlay,
    PlaySpan,
    PlayCue(usize),
    Seek(f32),
    Undo,
    StartEdit,
    CancelEdit,
    ToggleUnfamiliar,
    OpenGroup,
    ReviewUnsure,
    Finish,
    ConfirmGloss(usize),
    Go(Stage),
    Notice(String),
    Learn(Option<Scope>),
    MemFilter(Option<Scope>),
    MemDelete(usize),
    FocusMemory(usize),
    MemToggle(usize),
    OpenIssue(usize),
}

struct LogEntry {
    secs: f64,
    issue: usize,
    decision: Decision,
    grouped: Option<u32>,
}

struct Hist {
    changes: Vec<(usize, Decision)>,
    log_len: usize,
}

struct Toast {
    msg: String,
    until: f64,
    undo: bool,
}

pub struct Lab {
    lec: Lecture,
    dark: bool,
    role: Role,
    stage: Stage,
    decisions: Vec<Decision>,
    unfamiliar: Vec<bool>,
    current: Option<usize>,
    log: Vec<LogEntry>,
    history: Vec<Hist>,
    show_meaning: bool,
    playing: bool,
    t: f32,
    speed: f32,
    until: Option<f32>,
    last_cue: usize,
    scroll_to: Option<usize>,
    editing: bool,
    edit_buf: String,
    group_open: bool,
    group_sel: Vec<(usize, bool)>,
    next_group: u32,
    toast: Option<Toast>,
    export_open: bool,
    export_tab: u8,
    now: f64,
    started: f64,
    cjk_ok: bool,
    script: std::collections::VecDeque<String>,
    shot: Option<String>,
    frame: u32,
    idle_since: u32,
    shot_requested: bool,
    session_started: bool,
    memory: Vec<MemEntry>,
    mem_filter: Option<Scope>,
    learn: Option<usize>,
    mem_focus: Option<usize>,
    analysis: Analysis,
    mem_sig: String,
}

fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(BOLD.into()))
}

fn fmt_time(t: f32) -> String {
    let s = t.max(0.0) as u32;
    format!("{}:{:02}", s / 60, s % 60)
}

fn tier_dot(ui: &mut Ui, p: &Palette, tier: Tier) {
    let (rect, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
    let c = rect.center();
    let col = p.tier_color(tier);
    let ink = if p.dark {
        p.text
    } else {
        Color32::from_rgb(0x8A, 0x6A, 0x1E)
    };
    match tier {
        Tier::Memory => {
            ui.painter().circle_filled(c, 6.5, col);
            ui.painter().circle_stroke(c, 6.5, Stroke::new(1.5, ink));
        }
        Tier::Sound => {
            ui.painter().circle_filled(c, 6.5, col);
            ui.painter().circle_stroke(c, 6.5, Stroke::new(1.0, ink));
            ui.painter().circle_filled(c, 2.5, ink);
        }
        Tier::Possible => {
            ui.painter().circle_stroke(c, 6.5, Stroke::new(1.5, ink));
        }
    }
}

fn pill(ui: &mut Ui, text: &str, fill: Color32, ink: Color32) {
    Frame::NONE
        .fill(fill)
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(9, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(13.0).color(ink));
        });
}

impl Lab {
    pub fn new(ctx: &Context) -> Self {
        let cjk_ok = theme::install_fonts(ctx);
        let dark = ctx.system_theme().map_or(false, |t| t == egui::Theme::Dark);
        theme::apply(
            ctx,
            &if dark {
                Palette::dark()
            } else {
                Palette::light()
            },
        );
        let lec = data::demo();
        let n = lec.issues.len();
        Self {
            lec,
            dark,
            role: Role::Student,
            stage: Stage::Home,
            decisions: vec![Decision::Pending; n],
            unfamiliar: vec![false; n],
            current: None,
            log: Vec::new(),
            history: Vec::new(),
            show_meaning: false,
            playing: false,
            t: 0.0,
            speed: 1.0,
            until: None,
            last_cue: 0,
            scroll_to: None,
            editing: false,
            edit_buf: String::new(),
            group_open: false,
            group_sel: Vec::new(),
            next_group: 1,
            toast: None,
            export_open: false,
            export_tab: 0,
            now: 0.0,
            started: 0.0,
            cjk_ok,
            script: std::env::var("VOXLAB_SCRIPT")
                .map(|v| {
                    v.split(';')
                        .filter(|c| !c.is_empty())
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default(),
            shot: std::env::var("VOXLAB_SHOT").ok(),
            frame: 0,
            idle_since: 0,
            shot_requested: false,
            session_started: false,
            memory: pages::starter_memory(),
            mem_filter: None,
            learn: None,
            mem_focus: None,
            analysis: Analysis::default(),
            mem_sig: String::new(),
        }
    }

    /// Developer aid: `VOXLAB_SCRIPT="start;select:1;group"` replays commands and
    /// `VOXLAB_SHOT=path.ppm` saves the final frame, then exits.
    fn run_script(&mut self, ui: &Ui, acts: &mut Vec<Act>) {
        self.frame += 1;
        if std::env::var("VOXLAB_DEBUG").is_ok() && self.frame % 30 == 0 {
            eprintln!(
                "frame {} left={} req={}",
                self.frame,
                self.script.len(),
                self.shot_requested
            );
        }
        if self.frame < 4 {
            ui.ctx().request_repaint();
            return;
        }
        if self.shot_requested {
            let image = ui.input(|i| {
                i.raw.events.iter().find_map(|e| match e {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
            });
            if let (Some(img), Some(path)) = (image, &self.shot) {
                let [w, h] = img.size;
                let mut bytes = format!("P6\n{w} {h}\n255\n").into_bytes();
                for px in &img.pixels {
                    bytes.extend_from_slice(&[px.r(), px.g(), px.b()]);
                }
                let _ = std::fs::write(path, bytes);
                std::process::exit(0);
            }
            ui.ctx().request_repaint();
            return;
        }
        if let Some(cmd) = self.script.front().cloned() {
            if self.frame % 3 != 0 {
                ui.ctx().request_repaint();
                return;
            }
            self.script.pop_front();
            self.idle_since = self.frame;
            let (name, arg) = cmd.split_once(':').unwrap_or((cmd.as_str(), ""));
            let n: usize = arg.parse().unwrap_or(0);
            match name {
                "start" => acts.push(Act::Start),
                "go" => acts.push(Act::Go(match arg {
                    "course" => Stage::Course,
                    "memory" => Stage::Memory,
                    "intro" => Stage::Intro,
                    _ => Stage::Home,
                })),
                "learn" => acts.push(Act::Learn(match arg {
                    "course" => Some(Scope::Course),
                    "me" => Some(Scope::Me),
                    _ => None,
                })),
                "select" => acts.push(Act::Select(n)),
                "accept" => acts.push(Act::Decide(Decision::Accepted)),
                "keep" => acts.push(Act::Decide(Decision::Kept)),
                "unsure" => acts.push(Act::Decide(Decision::Unsure)),
                "edit" => acts.push(Act::StartEdit),
                "group" => acts.push(Act::OpenGroup),
                "groupaccept" => {
                    let ids: Vec<usize> = self
                        .group_sel
                        .iter()
                        .filter(|(_, on)| *on)
                        .map(|(i, _)| *i)
                        .collect();
                    acts.push(Act::DecideMany(ids, Decision::Accepted));
                }
                "meaning" => self.show_meaning = true,
                "memtoggle" => acts.push(Act::MemToggle(n)),
                "memfocus" => acts.push(Act::FocusMemory(n)),
                "jump" => acts.push(Act::OpenIssue(n)),
                "teacher" => self.role = Role::Teacher,
                "dark" => {
                    self.dark = true;
                    theme::apply(ui.ctx(), &self.pal());
                }
                "finish" => acts.push(Act::Finish),
                "receipt" => self.export_tab = 1,
                "undo" => acts.push(Act::Undo),
                "play" => acts.push(Act::PlaySpan),
                "unsure_review" => acts.push(Act::ReviewUnsure),
                _ => {}
            }
            ui.ctx().request_repaint();
        } else if self.shot.is_some() && self.frame > self.idle_since + 12 {
            self.shot_requested = true;
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            ui.ctx().request_repaint();
        } else {
            ui.ctx().request_repaint();
        }
    }

    fn pal(&self) -> Palette {
        if self.dark {
            Palette::dark()
        } else {
            Palette::light()
        }
    }

    fn pending(&self) -> usize {
        self.decisions
            .iter()
            .filter(|d| **d == Decision::Pending)
            .count()
    }

    fn count(&self, f: impl Fn(&Decision) -> bool) -> usize {
        self.decisions.iter().filter(|d| f(d)).count()
    }

    fn peers(&self, i: usize) -> Vec<usize> {
        let s = self.lec.issues[i].suggestion;
        (0..self.lec.issues.len())
            .filter(|&j| {
                j != i
                    && self.lec.issues[j].suggestion == s
                    && self.decisions[j] == Decision::Pending
            })
            .collect()
    }

    fn issue_context(&self, i: usize) -> String {
        self.lec.cues[self.lec.issues[i].cue].text.to_string()
    }

    // ---------------------------------------------------------------- state

    fn advance_from(&mut self, from: usize) {
        let n = self.lec.issues.len();
        let next = (from + 1..n)
            .chain(0..=from)
            .find(|&j| self.decisions[j] == Decision::Pending);
        self.current = next;
        if let Some(j) = next {
            self.scroll_to = Some(self.lec.issues[j].cue);
        }
    }

    fn decide(&mut self, targets: Vec<usize>, d: Decision, grouped: bool) {
        let group_id = grouped.then(|| {
            let g = self.next_group;
            self.next_group += 1;
            g
        });
        let log_len = self.log.len();
        let mut changes = Vec::new();
        for &i in &targets {
            changes.push((i, self.decisions[i].clone()));
            self.decisions[i] = d.clone();
            self.log.push(LogEntry {
                secs: self.now - self.started,
                issue: i,
                decision: d.clone(),
                grouped: group_id,
            });
        }
        self.history.push(Hist { changes, log_len });
        let msg = if targets.len() > 1 {
            format!(
                "已{} {} 處（各自記錄成獨立的決定）",
                d.word(),
                targets.len()
            )
        } else {
            format!("已{}", d.word())
        };
        self.toast = Some(Toast {
            msg,
            until: self.now + 4.5,
            undo: true,
        });
        self.editing = false;
        self.learn = (d == Decision::Accepted)
            .then(|| targets.last().copied())
            .flatten()
            .filter(|&i| {
                !self.memory.iter().any(|m| {
                    m.heard == self.lec.issues[i].heard
                        && m.suggestion == self.lec.issues[i].suggestion
                })
            });
        if let Some(&last) = targets.last() {
            self.advance_from(last);
        }
    }

    fn undo(&mut self) {
        self.learn = None;
        if let Some(h) = self.history.pop() {
            for (i, prev) in h.changes.iter().rev() {
                self.decisions[*i] = prev.clone();
            }
            self.log.truncate(h.log_len);
            if let Some((i, _)) = h.changes.first() {
                self.current = Some(*i);
                self.scroll_to = Some(self.lec.issues[*i].cue);
            }
            self.toast = Some(Toast {
                msg: "已復原".into(),
                until: self.now + 2.5,
                undo: false,
            });
        }
    }

    fn apply(&mut self, act: Act) {
        match act {
            Act::Start => {
                self.stage = Stage::Review;
                self.session_started = true;
                self.started = self.now;
                self.current =
                    (0..self.lec.issues.len()).find(|&j| self.decisions[j] == Decision::Pending);
                if let Some(j) = self.current {
                    self.scroll_to = Some(self.lec.issues[j].cue);
                }
            }
            Act::Decide(d) => {
                if let Some(i) = self.current {
                    self.decide(vec![i], d, false);
                }
            }
            Act::DecideMany(ids, d) => {
                self.group_open = false;
                self.decide(ids, d, true);
            }
            Act::Select(i) => {
                self.current = Some(i);
                self.editing = false;
                self.scroll_to = Some(self.lec.issues[i].cue);
            }
            Act::Step(delta) => {
                let n = self.lec.issues.len() as i32;
                let from = self.current.map_or(-1, |c| c as i32);
                let to = (from + delta).clamp(0, n - 1) as usize;
                self.apply(Act::Select(to));
            }
            Act::TogglePlay => {
                self.until = None;
                self.playing = !self.playing;
            }
            Act::PlaySpan => {
                if let Some(i) = self.current {
                    self.apply(Act::PlayCue(self.lec.issues[i].cue));
                }
            }
            Act::PlayCue(ci) => {
                let c = &self.lec.cues[ci];
                self.t = (c.start - 1.0).max(0.0);
                self.until = Some(c.end + 0.5);
                self.playing = true;
            }
            Act::Seek(t) => {
                self.t = t.clamp(0.0, self.lec.duration());
                self.until = None;
            }
            Act::Undo => self.undo(),
            Act::StartEdit => {
                if let Some(i) = self.current {
                    self.edit_buf = match &self.decisions[i] {
                        Decision::Edited(s) => s.clone(),
                        _ => self.lec.issues[i].suggestion.to_string(),
                    };
                    self.editing = true;
                }
            }
            Act::CancelEdit => self.editing = false,
            Act::ToggleUnfamiliar => {
                if let Some(i) = self.current {
                    self.unfamiliar[i] = !self.unfamiliar[i];
                }
            }
            Act::OpenGroup => {
                if let Some(i) = self.current {
                    let mut ids: Vec<usize> = self.peers(i);
                    if self.decisions[i] == Decision::Pending {
                        ids.push(i);
                    }
                    ids.sort_unstable();
                    self.group_sel = ids.into_iter().map(|j| (j, true)).collect();
                    self.group_open = true;
                }
            }
            Act::ReviewUnsure => {
                if let Some(j) = self.decisions.iter().position(|d| *d == Decision::Unsure) {
                    self.apply(Act::Select(j));
                }
            }
            Act::Finish => {
                self.current = None;
                self.export_open = true;
            }
            Act::Go(stage) => {
                if self.stage == Stage::Memory && stage != Stage::Memory {
                    for m in &mut self.memory {
                        m.fresh = false;
                    }
                    self.mem_focus = None;
                }
                self.stage = stage;
                self.group_open = false;
                self.editing = false;
            }
            Act::Notice(msg) => {
                self.toast = Some(Toast {
                    msg,
                    until: self.now + 3.5,
                    undo: false,
                });
            }
            Act::Learn(scope) => {
                if let (Some(scope), Some(i)) = (scope, self.learn) {
                    let is = &self.lec.issues[i];
                    self.memory.insert(
                        0,
                        MemEntry {
                            heard: is.heard.to_string(),
                            suggestion: is.suggestion.to_string(),
                            scope,
                            source: "第 3 堂・你剛確認".into(),
                            uses: 0,
                            enabled: true,
                            fresh: true,
                            source_issue: Some(i),
                        },
                    );
                    self.toast = Some(Toast {
                        msg: format!("已記住（{}）。可以在「它學到的」查看或收回", scope.label()),
                        until: self.now + 3.5,
                        undo: false,
                    });
                }
                self.learn = None;
            }
            Act::MemFilter(f) => self.mem_filter = f,
            Act::FocusMemory(m) => {
                self.mem_focus = Some(m);
                self.stage = Stage::Memory;
            }
            Act::MemToggle(m) => {
                if let Some(e) = self.memory.get_mut(m) {
                    e.enabled = !e.enabled;
                    let (h, s, on) = (e.heard.clone(), e.suggestion.clone(), e.enabled);
                    let n = self.analysis.per_entry.get(m).map_or(0, |v| {
                        v.iter()
                            .filter(|&&i| self.decisions[i] == Decision::Pending)
                            .count()
                    });
                    let msg = if on {
                        format!("已重新啟用「{h} → {s}」")
                    } else {
                        format!(
                            "已停用「{h} → {s}」，第 3 堂的 {n} 處回到一般提示。可以在「它學到的」重新啟用"
                        )
                    };
                    self.toast = Some(Toast {
                        msg,
                        until: self.now + 5.0,
                        undo: false,
                    });
                }
            }
            Act::OpenIssue(i) => {
                if !self.session_started {
                    self.apply(Act::Start);
                }
                self.stage = Stage::Review;
                self.apply(Act::Select(i));
            }
            Act::MemDelete(i) => {
                if i < self.memory.len() {
                    self.memory.remove(i);
                    self.mem_focus = None;
                    self.toast = Some(Toast {
                        msg: "已刪除這一條".into(),
                        until: self.now + 2.5,
                        undo: false,
                    });
                }
            }
            Act::ConfirmGloss(g) => {
                self.lec.glosses[g].confirmed = !self.lec.glosses[g].confirmed;
            }
        }
    }

    // ---------------------------------------------------------------- frame

    pub fn tick(&mut self, ui: &Ui) {
        self.now = ui.input(|i| i.time);
        if self.playing {
            let dt = ui.input(|i| i.stable_dt).min(0.1);
            self.t += dt * self.speed;
            let end = self.until.unwrap_or(self.lec.duration());
            if self.t >= end {
                self.t = end.min(self.lec.duration());
                self.playing = false;
                self.until = None;
            }
            let c = self.lec.cue_at(self.t);
            if c != self.last_cue {
                self.last_cue = c;
                self.scroll_to = Some(c);
            }
            ui.ctx().request_repaint();
        }
        if let Some(t) = &self.toast {
            if self.now > t.until {
                self.toast = None;
            } else {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(200));
            }
        }
    }

    fn read_keys(&self, ctx: &Context, acts: &mut Vec<Act>) {
        if self.stage != Stage::Review || self.editing || ctx.egui_wants_keyboard_input() {
            return;
        }
        ctx.input_mut(|i| {
            let n = Modifiers::NONE;
            if i.consume_key(Modifiers::COMMAND, Key::Z) {
                acts.push(Act::Undo);
            }
            if self.current.is_some() {
                if i.consume_key(n, Key::Enter) {
                    acts.push(Act::Decide(Decision::Accepted));
                }
                if i.consume_key(n, Key::K) {
                    acts.push(Act::Decide(Decision::Kept));
                }
                if i.consume_key(n, Key::U) {
                    acts.push(Act::Decide(Decision::Unsure));
                }
                if i.consume_key(n, Key::E) {
                    acts.push(Act::StartEdit);
                }
                if i.consume_key(n, Key::Space) {
                    acts.push(Act::PlaySpan);
                }
                if i.consume_key(n, Key::G) {
                    acts.push(Act::OpenGroup);
                }
            } else if i.consume_key(n, Key::Space) {
                acts.push(Act::TogglePlay);
            }
            if i.consume_key(n, Key::ArrowRight) || i.consume_key(n, Key::J) {
                acts.push(Act::Step(1));
            }
            if i.consume_key(n, Key::ArrowLeft) {
                acts.push(Act::Step(-1));
            }
        });
    }

    fn issue_style(&self, i: usize, p: &Palette) -> (Color32, Stroke, Color32) {
        let is_cur = self.current == Some(i);
        let tier = self.tier(i);
        let (bg, line) = match &self.decisions[i] {
            Decision::Pending => (
                p.tier_color(tier),
                Stroke::new(
                    match tier {
                        Tier::Memory => 2.5,
                        Tier::Sound => 1.5,
                        Tier::Possible => 1.0,
                    },
                    if p.dark {
                        p.text
                    } else {
                        Color32::from_rgb(0x9A, 0x74, 0x1F)
                    },
                ),
            ),
            Decision::Accepted => (p.ok, Stroke::new(1.5, p.teacher)),
            Decision::Kept => (Color32::TRANSPARENT, Stroke::new(1.0, p.kept)),
            Decision::Edited(_) => (p.edited, Stroke::new(1.5, p.accent)),
            Decision::Unsure => (
                p.unsure,
                Stroke::new(1.5, Color32::from_rgb(0xC2, 0x5B, 0x3A)),
            ),
        };
        if is_cur {
            (bg, Stroke::new(3.0, p.accent), p.text)
        } else {
            (bg, line, p.text)
        }
    }

    pub fn ui_root(&mut self, ui: &mut Ui) {
        self.tick(ui);
        self.refresh_matches();
        let mut acts = Vec::new();
        self.read_keys(ui.ctx(), &mut acts);
        self.run_script(ui, &mut acts);
        if matches!(self.stage, Stage::Home | Stage::Course | Stage::Memory) {
            self.rail(ui, &mut acts);
            match self.stage {
                Stage::Home => self.home(ui, &mut acts),
                Stage::Course => self.course(ui, &mut acts),
                _ => self.memory_page(ui, &mut acts),
            }
            self.toast_ui(ui.ctx(), &mut acts);
        } else if self.stage == Stage::Intro {
            self.intro(ui, &mut acts);
            self.toast_ui(ui.ctx(), &mut acts);
        } else {
            let p = self.pal();
            Panel::top("top")
                .frame(
                    Frame::NONE
                        .fill(p.panel)
                        .inner_margin(Margin::symmetric(24, 12))
                        .stroke(Stroke::new(1.0, p.line)),
                )
                .show(ui, |ui| self.header(ui, &mut acts));
            Panel::bottom("player")
                .frame(
                    Frame::NONE
                        .fill(p.panel)
                        .inner_margin(Margin::symmetric(24, 10))
                        .stroke(Stroke::new(1.0, p.line)),
                )
                .show(ui, |ui| self.player(ui, &mut acts));
            Panel::bottom("tray")
                .frame(
                    Frame::NONE
                        .fill(p.panel)
                        .inner_margin(Margin::symmetric(24, 14)),
                )
                .show(ui, |ui| self.tray(ui, &mut acts));
            egui::CentralPanel::default()
                .frame(
                    Frame::NONE
                        .fill(p.bg)
                        .inner_margin(Margin::symmetric(16, 8)),
                )
                .show(ui, |ui| self.transcript(ui, &mut acts));
            self.group_window(ui.ctx(), &mut acts);
            self.export_window(ui.ctx());
            self.toast_ui(ui.ctx(), &mut acts);
        }
        if !acts.is_empty() {
            ui.ctx().request_repaint();
        }
        for a in acts {
            self.apply(a);
        }
    }

    /// Re-run near-sound matching only when the remembered spellings changed.
    fn refresh_matches(&mut self) {
        let sig: String = self
            .memory
            .iter()
            .map(|m| format!("{}|{}|{};", m.heard, m.suggestion, m.enabled))
            .collect();
        if sig != self.mem_sig {
            self.analysis = matcher::analyse(&self.lec, &self.memory);
            self.mem_sig = sig;
        }
    }

    // ---------------------------------------------------------------- intro

    fn intro(&mut self, ui: &mut Ui, acts: &mut Vec<Act>) {
        let p = self.pal();
        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(p.bg))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space((ui.available_height() * 0.12).max(24.0));
                    ui.set_max_width(560.0);
                    if ui
                        .add(
                            Button::new("返回首頁")
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::NONE),
                        )
                        .clicked()
                    {
                        acts.push(Act::Go(Stage::Home));
                    }
                    ui.label(RichText::new(self.lec.title).font(bold(30.0)));
                    ui.label(
                        RichText::new(format!(
                            "{}　·　{}　·　{} 句",
                            self.lec.teacher,
                            fmt_time(self.lec.duration()),
                            self.lec.cues.len()
                        ))
                        .color(p.sub),
                    );
                    ui.add_space(22.0);
                    Frame::NONE
                        .fill(p.panel)
                        .stroke(Stroke::new(1.0, p.line))
                        .corner_radius(CornerRadius::same(14))
                        .inner_margin(Margin::same(22))
                        .show(ui, |ui| {
                            ui.set_width(516.0);
                            let n = self.lec.issues.len();
                            let flagged_cues: std::collections::BTreeSet<usize> =
                                self.lec.issues.iter().map(|i| i.cue).collect();
                            ui.label(
                                RichText::new(format!("有 {n} 處值得你聽一下")).font(bold(22.0)),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "其餘 {} 句我沒有發現問題，但沒標記不代表一定正確。",
                                    self.lec.cues.len() - flagged_cues.len()
                                ))
                                .color(p.sub),
                            );
                            ui.add_space(12.0);
                            for tier in [Tier::Memory, Tier::Sound, Tier::Possible] {
                                let k = (0..self.lec.issues.len())
                                    .filter(|&i| self.tier(i) == tier)
                                    .count();
                                ui.horizontal(|ui| {
                                    tier_dot(ui, &p, tier);
                                    ui.label(format!("{k} 處　{}", tier.label()));
                                });
                            }
                            ui.add_space(14.0);
                            ui.label(
                                RichText::new("相同的建議可以一次看完；每一處仍由你各自決定。")
                                    .color(p.sub)
                                    .size(14.0),
                            );
                        });
                    ui.add_space(22.0);
                    ui.horizontal(|ui| {
                        ui.add_space((ui.available_width() - 330.0) / 2.0);
                        ui.label(RichText::new("我是").color(p.sub));
                        ui.selectable_value(&mut self.role, Role::Student, "學生");
                        ui.selectable_value(&mut self.role, Role::Teacher, "老師");
                    });
                    ui.add_space(10.0);
                    let go = Button::new(
                        RichText::new("開始審閱")
                            .font(bold(19.0))
                            .color(p.on_accent),
                    )
                    .fill(p.accent)
                    .corner_radius(CornerRadius::same(12))
                    .min_size(vec2(240.0, 52.0));
                    if ui.add(go).clicked() {
                        acts.push(Act::Start);
                    }
                    ui.add_space(26.0);
                    ui.label(
                        RichText::new("實驗版：示範資料與模擬音訊，不會讀取或修改任何檔案。")
                            .size(13.0)
                            .color(p.sub),
                    );
                    if !self.cjk_ok {
                        ui.colored_label(
                            Color32::from_rgb(0xC2, 0x5B, 0x3A),
                            "找不到繁體中文系統字型，文字可能顯示成方塊。",
                        );
                    }
                });
            });
    }

    // --------------------------------------------------------------- header

    fn header(&mut self, ui: &mut Ui, acts: &mut Vec<Act>) {
        let p = self.pal();
        ui.horizontal(|ui| {
            if ui
                .add(
                    Button::new("返回首頁")
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::new(1.0, p.line))
                        .corner_radius(CornerRadius::same(10)),
                )
                .clicked()
            {
                acts.push(Act::Go(Stage::Home));
            }
            ui.label(RichText::new(self.lec.title).font(bold(18.0)));
            ui.label(RichText::new(self.lec.teacher).color(p.sub));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let ready = self.pending() == 0;
                let label = if ready {
                    "完成，預覽匯出"
                } else {
                    "預覽匯出"
                };
                let b = Button::new(RichText::new(label).color(if ready {
                    p.on_accent
                } else {
                    p.text
                }))
                .fill(if ready {
                    p.accent
                } else {
                    Color32::TRANSPARENT
                })
                .stroke(Stroke::new(1.0, if ready { p.accent } else { p.line }))
                .corner_radius(CornerRadius::same(10));
                if ui.add(b).clicked() {
                    acts.push(Act::Finish);
                }
                let mode = if self.dark { "淺色" } else { "深色" };
                if ui
                    .add(
                        Button::new(mode)
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::new(1.0, p.line))
                            .corner_radius(CornerRadius::same(10)),
                    )
                    .clicked()
                {
                    self.dark = !self.dark;
                    theme::apply(ui.ctx(), &self.pal());
                }
                ui.checkbox(&mut self.show_meaning, "顯示意思");
                ui.selectable_value(&mut self.role, Role::Teacher, "老師");
                ui.selectable_value(&mut self.role, Role::Student, "學生");
            });
        });
        ui.add_space(6.0);
        // Segmented progress: one segment per flagged place.
        let n = self.lec.issues.len();
        let gap = 4.0;
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 14.0), Sense::hover());
        let w = (rect.width() - gap * (n as f32 - 1.0)) / n as f32;
        for i in 0..n {
            let r = Rect::from_min_size(
                pos2(rect.left() + i as f32 * (w + gap), rect.top()),
                vec2(w, rect.height()),
            );
            let resp = ui.interact(r, ui.id().with(("seg", i)), Sense::click());
            let fill = match &self.decisions[i] {
                Decision::Pending => p.line,
                Decision::Accepted => p.teacher,
                Decision::Kept => p.kept,
                Decision::Edited(_) => p.accent,
                Decision::Unsure => Color32::from_rgb(0xC2, 0x5B, 0x3A),
            };
            ui.painter().rect_filled(r, CornerRadius::same(5), fill);
            if self.current == Some(i) {
                ui.painter().rect_stroke(
                    r.expand(2.0),
                    CornerRadius::same(6),
                    Stroke::new(2.0, p.accent),
                    StrokeKind::Outside,
                );
            }
            if resp
                .on_hover_text(format!("第 {} 處：{}", i + 1, self.decisions[i].word()))
                .clicked()
            {
                acts.push(Act::Select(i));
            }
        }
        ui.add_space(4.0);
        let left = self.pending();
        let msg = if left == 0 {
            let u = self.count(|d| *d == Decision::Unsure);
            if u > 0 {
                format!("全部處理過了，其中 {u} 處標為不確定")
            } else {
                "全部處理過了".to_string()
            }
        } else {
            format!("還有 {left} 處需要你看")
        };
        ui.label(RichText::new(msg).size(14.0).color(p.sub));
    }

    // ----------------------------------------------------------- transcript

    fn transcript(&mut self, ui: &mut Ui, acts: &mut Vec<Act>) {
        let p = self.pal();
        let scroll_to = self.scroll_to.take();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let w = ui.available_width().min(800.0);
                let side = ((ui.available_width() - w) / 2.0).max(0.0);
                ui.horizontal_top(|ui| {
                    ui.add_space(side);
                    ui.vertical(|ui| {
                        ui.set_width(w);
                        ui.add_space(8.0);
                        for ci in 0..self.lec.cues.len() {
                            self.cue_row(ui, ci, w, scroll_to == Some(ci), &p, acts);
                        }
                        ui.add_space(40.0);
                    });
                });
            });
    }

    fn cue_row(
        &self,
        ui: &mut Ui,
        ci: usize,
        w: f32,
        scroll: bool,
        p: &Palette,
        acts: &mut Vec<Act>,
    ) {
        let cue = &self.lec.cues[ci];
        let is_cur_cue = self.current.is_some_and(|i| self.lec.issues[i].cue == ci);
        let is_playing_cue =
            (self.playing || self.until.is_some()) && self.lec.cue_at(self.t) == ci;
        let bg_slot = ui.painter().add(Shape::Noop);
        let top_left = ui.cursor().min;
        let gutter = 58.0;
        let text_w = w - gutter - 28.0;
        let row = ui.horizontal_top(|ui| {
            ui.add_space(8.0);
            let (grect, gresp) = ui.allocate_exact_size(vec2(gutter, 32.0), Sense::click());
            let tcol = if is_playing_cue { p.accent } else { p.sub };
            ui.painter().text(
                grect.left_center(),
                Align2::LEFT_CENTER,
                fmt_time(cue.start),
                FontId::proportional(14.0),
                tcol,
            );
            if gresp.on_hover_text("從這裡開始聽").clicked() {
                acts.push(Act::PlayCue(ci));
            }
            ui.vertical(|ui| {
                ui.set_width(text_w);
                let segs = self.segs(ci);
                let mut job = LayoutJob::default();
                job.wrap.max_width = text_w;
                for s in &segs {
                    let (bg, ul, col) = match s.kind {
                        SegKind::Plain => (Color32::TRANSPARENT, Stroke::NONE, p.text),
                        SegKind::Gloss(_) => {
                            (Color32::TRANSPARENT, Stroke::new(1.5, p.accent), p.text)
                        }
                        SegKind::Issue(i) => self.issue_style(i, p),
                    };
                    job.append(
                        &s.text,
                        0.0,
                        TextFormat {
                            font_id: FontId::proportional(19.0),
                            color: col,
                            background: bg,
                            underline: ul,
                            line_height: Some(32.0),
                            ..Default::default()
                        },
                    );
                }
                let galley = ui.painter().layout_job(job);
                let (rect, resp) = ui.allocate_exact_size(galley.size(), Sense::click());
                ui.painter().galley(rect.min, galley.clone(), p.text);
                if let Some(pos) = resp.hover_pos().or_else(|| resp.interact_pointer_pos()) {
                    let idx = galley.cursor_from_pos(pos - rect.min).index.0;
                    let mut start = 0;
                    for s in &segs {
                        let len = s.text.chars().count();
                        if idx >= start && idx < start + len {
                            if let SegKind::Issue(i) = s.kind {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                if resp.clicked() {
                                    acts.push(Act::Select(i));
                                }
                            }
                        }
                        start += len;
                    }
                }
                if self.show_meaning {
                    for (g, gl) in self
                        .lec
                        .glosses
                        .iter()
                        .enumerate()
                        .filter(|(_, g)| g.cue == ci)
                    {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new(format!("「{}」＝{}　·　{}", gl.term, gl.zh, gl.en))
                                    .size(14.5)
                                    .color(p.sub),
                            );
                            if gl.confirmed {
                                pill(ui, "老師已確認", p.ok, p.teacher);
                            } else {
                                pill(ui, "AI 推測，尚未確認", p.unsure.gamma_multiply(0.6), p.ai);
                                if self.role == Role::Teacher
                                    && ui.small_button("確認這個意思").clicked()
                                {
                                    acts.push(Act::ConfirmGloss(g));
                                }
                            }
                        });
                    }
                }
                ui.add_space(6.0);
            });
        });
        let rr = row.response.rect;
        let area = Rect::from_min_max(
            top_left - vec2(0.0, 2.0),
            pos2(top_left.x + w, rr.bottom() + 2.0),
        );
        if is_cur_cue || is_playing_cue {
            let fill = if is_cur_cue {
                p.panel
            } else {
                p.accent.gamma_multiply(0.07)
            };
            ui.painter().set(
                bg_slot,
                Shape::rect_filled(area, CornerRadius::same(10), fill),
            );
            if is_cur_cue {
                ui.painter().rect_stroke(
                    area,
                    CornerRadius::same(10),
                    Stroke::new(1.0, p.line),
                    StrokeKind::Inside,
                );
            }
        }
        if scroll {
            ui.scroll_to_rect(area.expand(60.0), Some(Align::Center));
        }
        ui.add_space(4.0);
    }

    fn segs(&self, ci: usize) -> Vec<Seg> {
        let text = self.lec.cues[ci].text;
        let mut marks: Vec<(usize, usize, SegKind)> = Vec::new();
        for (i, is) in self
            .lec
            .issues
            .iter()
            .enumerate()
            .filter(|(_, is)| is.cue == ci)
        {
            marks.push((is.range.0, is.range.1, SegKind::Issue(i)));
        }
        if self.show_meaning {
            for (g, gl) in self
                .lec
                .glosses
                .iter()
                .enumerate()
                .filter(|(_, g)| g.cue == ci)
            {
                marks.push((gl.range.0, gl.range.1, SegKind::Gloss(g)));
            }
        }
        marks.sort_by_key(|m| m.0);
        let mut out = Vec::new();
        let mut pos = 0;
        for (s, e, kind) in marks {
            if s > pos {
                out.push(Seg {
                    text: text[pos..s].to_string(),
                    kind: SegKind::Plain,
                });
            }
            let shown = match kind {
                SegKind::Issue(i) => match &self.decisions[i] {
                    Decision::Accepted => self.lec.issues[i].suggestion.to_string(),
                    Decision::Edited(t) => t.clone(),
                    _ => text[s..e].to_string(),
                },
                _ => text[s..e].to_string(),
            };
            out.push(Seg { text: shown, kind });
            pos = e;
        }
        if pos < text.len() {
            out.push(Seg {
                text: text[pos..].to_string(),
                kind: SegKind::Plain,
            });
        }
        out
    }

    // ----------------------------------------------------------------- tray

    fn tray(&mut self, ui: &mut Ui, acts: &mut Vec<Act>) {
        let p = self.pal();
        ui.set_min_height(150.0);
        self.learn_prompt(ui, acts);
        let Some(i) = self.current else {
            self.done_card(ui, acts, &p);
            return;
        };
        let (heard, suggestion) = {
            let is = &self.lec.issues[i];
            (is.heard, is.suggestion)
        };
        let tier = self.tier(i);
        let reason = self.reason(i);
        let mem = self.mem_hit(i).map(|h| {
            (
                h.entry,
                self.memory[h.entry].source.clone(),
                self.memory[h.entry].scope,
                h.exact,
            )
        });
        let decision = self.decisions[i].clone();
        let peers = self.peers(i).len();
        let total_w = ui.available_width();
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(total_w * 0.56);
                ui.horizontal(|ui| {
                    tier_dot(ui, &p, tier);
                    let near = mem.as_ref().is_some_and(|m| !m.3);
                    ui.label(
                        RichText::new(if near {
                            "和你確認過的寫法很像"
                        } else {
                            tier.label()
                        })
                        .size(14.0)
                        .color(p.sub),
                    );
                    ui.label(
                        RichText::new(format!("第 {} / {} 處", i + 1, self.lec.issues.len()))
                            .size(14.0)
                            .color(p.sub),
                    );
                    if decision != Decision::Pending {
                        pill(ui, &format!("目前：{}", decision.word()), p.edited, p.text);
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(heard).size(24.0).strikethrough().color(p.sub));
                    ui.label(RichText::new("→").size(24.0).color(p.sub));
                    let shown = match &decision {
                        Decision::Edited(s) => s.as_str(),
                        _ => suggestion,
                    };
                    ui.label(RichText::new(shown).font(bold(28.0)));
                });
                ui.label(RichText::new(&reason).size(15.0).color(p.sub));
                if let Some((m, source, scope, exact)) = &mem {
                    ui.horizontal(|ui| {
                        let near = if *exact { "" } else { "・相近音" };
                        pill(
                            ui,
                            &format!("來自它學到的・{}・{}{}", source, scope.label(), near),
                            p.ok,
                            p.text,
                        );
                        if ui.small_button("看這條").clicked() {
                            acts.push(Act::FocusMemory(*m));
                        }
                        if ui.small_button("停用這條").clicked() {
                            acts.push(Act::MemToggle(*m));
                        }
                    });
                }
                ui.horizontal(|ui| {
                    if peers > 0 {
                        if ui
                            .add(
                                Button::new(format!("還有 {peers} 處一樣的建議，一起看  G"))
                                    .corner_radius(CornerRadius::same(10)),
                            )
                            .clicked()
                        {
                            acts.push(Act::OpenGroup);
                        }
                    }
                    if self.role == Role::Student {
                        let mut f = self.unfamiliar[i];
                        if ui.checkbox(&mut f, "這個詞我不熟").changed() {
                            acts.push(Act::ToggleUnfamiliar);
                        }
                    }
                });
            });
            ui.add_space(16.0);
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                if self.editing {
                    ui.label(RichText::new("改成什麼？").color(p.sub));
                    let te = egui::TextEdit::singleline(&mut self.edit_buf)
                        .font(FontId::proportional(20.0))
                        .desired_width(f32::INFINITY);
                    let resp = ui.add(te);
                    if !resp.has_focus() && !resp.lost_focus() {
                        resp.request_focus();
                    }
                    let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                    let esc = ui.input(|i| i.key_pressed(Key::Escape));
                    ui.horizontal(|ui| {
                        let ok = Button::new(RichText::new("確定　Enter").color(p.on_accent))
                            .fill(p.accent)
                            .corner_radius(CornerRadius::same(10));
                        if ui.add(ok).clicked() || enter {
                            let t = self.edit_buf.trim().to_string();
                            if !t.is_empty() {
                                acts.push(Act::Decide(Decision::Edited(t)));
                            }
                        }
                        if ui.button("取消　Esc").clicked() || esc {
                            acts.push(Act::CancelEdit);
                        }
                    });
                } else {
                    let w = ui.available_width();
                    let accept = Button::new(
                        RichText::new("接受建議　Enter")
                            .font(bold(18.0))
                            .color(p.on_accent),
                    )
                    .fill(p.accent)
                    .corner_radius(CornerRadius::same(12))
                    .min_size(vec2(w, 48.0));
                    if ui.add(accept).clicked() {
                        acts.push(Act::Decide(Decision::Accepted));
                    }
                    ui.horizontal(|ui| {
                        let bw = (w - 2.0 * ui.spacing().item_spacing.x) / 3.0;
                        for (label, act) in [
                            ("保留原樣　K", Act::Decide(Decision::Kept)),
                            ("自己改　E", Act::StartEdit),
                            ("不確定　U", Act::Decide(Decision::Unsure)),
                        ] {
                            let b = Button::new(label)
                                .corner_radius(CornerRadius::same(10))
                                .min_size(vec2(bw, 38.0));
                            if ui.add(b).clicked() {
                                acts.push(act);
                            }
                        }
                    });
                    ui.label(
                        RichText::new("空白鍵 聽這段　·　← → 切換　·　⌘Z 復原")
                            .size(13.0)
                            .color(p.sub),
                    );
                }
            });
        });
    }

    fn done_card(&mut self, ui: &mut Ui, acts: &mut Vec<Act>, p: &Palette) {
        let acc = self.count(|d| *d == Decision::Accepted);
        let kept = self.count(|d| *d == Decision::Kept);
        let edited = self.count(|d| matches!(d, Decision::Edited(_)));
        let unsure = self.count(|d| *d == Decision::Unsure);
        let left = self.pending();
        ui.vertical(|ui| {
            if left > 0 {
                ui.label(RichText::new("點上方的句子或進度條，選一處開始。").font(bold(20.0)));
                return;
            }
            ui.label(RichText::new("這堂課看完了").font(bold(22.0)));
            ui.label(RichText::new(format!(
                "接受 {acc} 處　·　保留原樣 {kept} 處　·　自己改 {edited} 處　·　不確定 {unsure} 處"
            )).color(p.sub));
            ui.label(
                RichText::new("沒被標記的句子沒有經過逐句確認，匯出的收據會清楚寫出這一點。")
                    .size(14.0)
                    .color(p.sub),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let b = Button::new(
                    RichText::new("預覽匯出與收據")
                        .font(bold(17.0))
                        .color(p.on_accent),
                )
                .fill(p.accent)
                .corner_radius(CornerRadius::same(12))
                .min_size(vec2(200.0, 42.0));
                if ui.add(b).clicked() {
                    acts.push(Act::Finish);
                }
                if unsure > 0
                    && ui
                        .add(
                            Button::new(format!("回去看 {unsure} 處不確定"))
                                .min_size(vec2(0.0, 42.0))
                                .corner_radius(CornerRadius::same(12)),
                        )
                        .clicked()
                {
                    acts.push(Act::ReviewUnsure);
                }
            });
        });
    }

    // --------------------------------------------------------------- player

    fn player(&mut self, ui: &mut Ui, acts: &mut Vec<Act>) {
        let p = self.pal();
        ui.horizontal(|ui| {
            let (rect, resp) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::click());
            ui.painter().circle_filled(rect.center(), 19.0, p.accent);
            let c = rect.center();
            if self.playing {
                for dx in [-4.5, 4.5] {
                    ui.painter().rect_filled(
                        Rect::from_center_size(pos2(c.x + dx, c.y), vec2(4.0, 14.0)),
                        CornerRadius::same(1),
                        p.on_accent,
                    );
                }
            } else {
                ui.painter().add(Shape::convex_polygon(
                    vec![
                        pos2(c.x - 5.0, c.y - 8.0),
                        pos2(c.x - 5.0, c.y + 8.0),
                        pos2(c.x + 8.0, c.y),
                    ],
                    p.on_accent,
                    Stroke::NONE,
                ));
            }
            if resp.clicked() {
                acts.push(Act::TogglePlay);
            }
            ui.label(
                RichText::new(format!(
                    "{} / {}",
                    fmt_time(self.t),
                    fmt_time(self.lec.duration())
                ))
                .size(14.0),
            );
            let tail = 170.0;
            let (rect, resp) = ui.allocate_exact_size(
                vec2((ui.available_width() - tail).max(100.0), 40.0),
                Sense::click_and_drag(),
            );
            let track = Rect::from_center_size(rect.center(), vec2(rect.width(), 6.0));
            ui.painter()
                .rect_filled(track, CornerRadius::same(3), p.line);
            let dur = self.lec.duration();
            let x_of = |t: f32| rect.left() + rect.width() * t / dur;
            let fill = Rect::from_min_max(track.min, pos2(x_of(self.t), track.max.y));
            ui.painter()
                .rect_filled(fill, CornerRadius::same(3), p.accent.gamma_multiply(0.55));
            for (i, is) in self.lec.issues.iter().enumerate() {
                let cx = x_of((self.lec.cues[is.cue].start + self.lec.cues[is.cue].end) / 2.0);
                let col = match &self.decisions[i] {
                    Decision::Pending => Color32::from_rgb(0xD9, 0x9A, 0x1C),
                    Decision::Accepted => p.teacher,
                    Decision::Kept => p.kept,
                    Decision::Edited(_) => p.accent,
                    Decision::Unsure => Color32::from_rgb(0xC2, 0x5B, 0x3A),
                };
                let r = if self.current == Some(i) { 7.0 } else { 4.5 };
                ui.painter()
                    .circle_filled(pos2(cx, rect.center().y - 12.0), r, col);
            }
            ui.painter().line_segment(
                [
                    pos2(x_of(self.t), rect.top() + 4.0),
                    pos2(x_of(self.t), rect.bottom() - 4.0),
                ],
                Stroke::new(2.0, p.text),
            );
            if (resp.clicked() || resp.dragged())
                && let Some(pos) = resp.interact_pointer_pos()
            {
                acts.push(Act::Seek(((pos.x - rect.left()) / rect.width()) * dur));
            }
            egui::ComboBox::from_id_salt("speed")
                .width(70.0)
                .selected_text(format!("{}×", self.speed))
                .show_ui(ui, |ui| {
                    for s in [0.75, 1.0, 1.25] {
                        ui.selectable_value(&mut self.speed, s, format!("{s}×"));
                    }
                });
            ui.label(RichText::new("模擬音訊").size(12.0).color(p.sub));
        });
    }

    // -------------------------------------------------------------- windows

    fn group_window(&mut self, ctx: &Context, acts: &mut Vec<Act>) {
        if !self.group_open {
            return;
        }
        let p = self.pal();
        let Some(&(first, _)) = self.group_sel.first() else {
            self.group_open = false;
            return;
        };
        let (heard, suggestion) = (
            self.lec.issues[first].heard,
            self.lec.issues[first].suggestion,
        );
        let mut open = true;
        egui::Window::new("一起看相同的建議")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, vec2(0.0, -40.0))
            .fixed_size(vec2(560.0, 0.0))
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(format!(
                        "「{heard}」→「{suggestion}」　共 {} 處",
                        self.group_sel.len()
                    ))
                    .font(bold(20.0)),
                );
                ui.label(
                    RichText::new(
                        "先聽、先看上下文，再決定要勾哪幾處。每一處都會各自記錄成一筆獨立的決定。",
                    )
                    .size(14.0)
                    .color(p.sub),
                );
                ui.add_space(6.0);
                for k in 0..self.group_sel.len() {
                    let (id, _) = self.group_sel[k];
                    let cue_i = self.lec.issues[id].cue;
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.group_sel[k].1, "");
                        ui.label(RichText::new(fmt_time(self.lec.cues[cue_i].start)).color(p.sub));
                        let text = self.issue_context(id);
                        let r = self.lec.issues[id].range;
                        let mut job = LayoutJob::default();
                        job.wrap.max_width = 360.0;
                        let base = TextFormat {
                            font_id: FontId::proportional(16.0),
                            color: p.text,
                            ..Default::default()
                        };
                        job.append(&text[..r.0], 0.0, base.clone());
                        job.append(
                            &text[r.0..r.1],
                            0.0,
                            TextFormat {
                                background: p.sound,
                                underline: Stroke::new(1.5, p.accent),
                                ..base.clone()
                            },
                        );
                        job.append(&text[r.1..], 0.0, base);
                        ui.label(job);
                        if ui.small_button("聽").clicked() {
                            acts.push(Act::PlayCue(cue_i));
                        }
                    });
                }
                ui.add_space(8.0);
                let chosen: Vec<usize> = self
                    .group_sel
                    .iter()
                    .filter(|(_, on)| *on)
                    .map(|(i, _)| *i)
                    .collect();
                ui.horizontal(|ui| {
                    let label = format!("接受已勾選的 {} 處", chosen.len());
                    let b = Button::new(RichText::new(label).color(p.on_accent))
                        .fill(p.accent)
                        .corner_radius(CornerRadius::same(10));
                    if ui.add_enabled(!chosen.is_empty(), b).clicked() {
                        acts.push(Act::DecideMany(chosen.clone(), Decision::Accepted));
                    }
                    if ui.button("先不要").clicked() {
                        self.group_open = false;
                    }
                });
            });
        if !open {
            self.group_open = false;
        }
    }

    fn toast_ui(&mut self, ctx: &Context, acts: &mut Vec<Act>) {
        let Some(t) = &self.toast else { return };
        let p = self.pal();
        egui::Area::new(egui::Id::new("toast"))
            .anchor(
                if self.stage == Stage::Review {
                    Align2::CENTER_TOP
                } else {
                    Align2::CENTER_BOTTOM
                },
                if self.stage == Stage::Review {
                    vec2(0.0, 84.0)
                } else {
                    vec2(0.0, -28.0)
                },
            )
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                Frame::NONE
                    .fill(if p.dark {
                        p.line
                    } else {
                        Color32::from_rgb(0x2A, 0x26, 0x23)
                    })
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(Margin::symmetric(16, 10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(&t.msg).color(Color32::from_rgb(0xF5, 0xF0, 0xE8)),
                            );
                            if t.undo
                                && ui
                                    .link(
                                        RichText::new("復原")
                                            .color(Color32::from_rgb(0x9F, 0xC8, 0xFF)),
                                    )
                                    .clicked()
                            {
                                acts.push(Act::Undo);
                            }
                        });
                    });
            });
    }

    fn export_window(&mut self, ctx: &Context) {
        if !self.export_open {
            return;
        }
        let p = self.pal();
        let mut open = true;
        let reviewed = self.reviewed_text();
        let receipt = self.receipt_text();
        egui::Window::new("匯出預覽")
            .open(&mut open)
            .anchor(Align2::CENTER_CENTER, vec2(0.0, -30.0))
            .collapsible(false)
            .default_size(vec2(640.0, 520.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.export_tab, 0, "審閱後逐字稿");
                    ui.selectable_value(&mut self.export_tab, 1, "審閱收據");
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("複製目前這份").clicked() {
                            ctx.copy_text(if self.export_tab == 0 {
                                reviewed.clone()
                            } else {
                                receipt.clone()
                            });
                        }
                    });
                });
                ui.label(
                    RichText::new("實驗版只做預覽，不會寫入檔案。")
                        .size(13.0)
                        .color(p.sub),
                );
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let text = if self.export_tab == 0 {
                        &reviewed
                    } else {
                        &receipt
                    };
                    ui.label(RichText::new(text).size(15.5));
                });
            });
        if !open {
            self.export_open = false;
        }
    }

    fn reviewed_text(&self) -> String {
        let mut out = String::new();
        for (ci, c) in self.lec.cues.iter().enumerate() {
            let line: String = self.segs(ci).iter().map(|s| s.text.as_str()).collect();
            out.push_str(&format!("[{}] {}\n", fmt_time(c.start), line));
        }
        out
    }

    fn receipt_text(&self) -> String {
        let n = self.lec.issues.len();
        let who = if self.role == Role::Student {
            "學生"
        } else {
            "老師"
        };
        let mut s = format!(
            "審閱收據（實驗版示範）\n課程：{}\n審閱者身分：{}\n\n",
            self.lec.title, who
        );
        s += &format!(
            "共 {n} 處提示：接受 {}、保留原樣 {}、自己改 {}、不確定 {}、尚未處理 {}\n",
            self.count(|d| *d == Decision::Accepted),
            self.count(|d| *d == Decision::Kept),
            self.count(|d| matches!(d, Decision::Edited(_))),
            self.count(|d| *d == Decision::Unsure),
            self.pending(),
        );
        let groups: std::collections::BTreeSet<u32> =
            self.log.iter().filter_map(|l| l.grouped).collect();
        let grouped_n = self.log.iter().filter(|l| l.grouped.is_some()).count();
        s += &format!(
            "一起看的手勢 {} 次，展開成 {} 筆各自獨立的決定。\n\n",
            groups.len(),
            grouped_n
        );
        let open: Vec<usize> = (0..n)
            .filter(|&i| matches!(self.decisions[i], Decision::Unsure | Decision::Pending))
            .collect();
        s += "尚未解決：\n";
        if open.is_empty() {
            s += "  （沒有）\n";
        }
        for i in open {
            let is = &self.lec.issues[i];
            s += &format!(
                "  {} 「{}」（{}）\n",
                fmt_time(self.lec.cues[is.cue].start),
                is.heard,
                self.decisions[i].word()
            );
        }
        if self.role == Role::Student {
            let f: Vec<&str> = (0..n)
                .filter(|&i| self.unfamiliar[i])
                .map(|i| self.lec.issues[i].suggestion)
                .collect();
            s += &format!(
                "\n我標記為不熟的詞：{}\n",
                if f.is_empty() {
                    "（無）".into()
                } else {
                    f.join("、")
                }
            );
        }
        s += "\n逐筆決定（依處理順序）：\n";
        for l in &self.log {
            let is = &self.lec.issues[l.issue];
            let result = match &l.decision {
                Decision::Edited(t) => format!("改成「{t}」"),
                d => d.word().to_string(),
            };
            let via = if l.grouped.is_some() {
                "，一起看"
            } else {
                ""
            };
            s += &format!(
                "  +{:>3.0} 秒　{}　「{}」→「{}」：{}{}\n",
                l.secs,
                fmt_time(self.lec.cues[is.cue].start),
                is.heard,
                is.suggestion,
                result,
                via
            );
        }
        s += "\n聲明：沒有被標記的句子並未逐句確認；這份收據只說明有哪些提示被處理，不代表逐字稿完全正確。\n";
        s
    }
}

#[derive(Clone, Copy)]
enum SegKind {
    Plain,
    Issue(usize),
    Gloss(usize),
}

struct Seg {
    text: String,
    kind: SegKind,
}

impl eframe::App for Lab {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.ui_root(ui);
    }
}

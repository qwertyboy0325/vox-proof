use egui::{Color32, Context, FontData, FontDefinitions, FontFamily, TextStyle};
use font_kit::family_name::FamilyName;
use font_kit::handle::Handle;
use font_kit::properties::{Properties, Weight};
use font_kit::source::SystemSource;
use std::sync::Arc;

#[derive(Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    pub bg: Color32,
    pub panel: Color32,
    pub text: Color32,
    pub sub: Color32,
    pub line: Color32,
    pub accent: Color32,
    pub on_accent: Color32,
    pub memory: Color32,
    pub sound: Color32,
    pub possible: Color32,
    pub ok: Color32,
    pub edited: Color32,
    pub unsure: Color32,
    pub kept: Color32,
    pub teacher: Color32,
    pub ai: Color32,
}

fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

impl Palette {
    pub fn light() -> Self {
        Self {
            dark: false,
            bg: rgb(0xFBF8F3),
            panel: rgb(0xFFFFFF),
            text: rgb(0x2A2623),
            sub: rgb(0x7C7269),
            line: rgb(0xE6DDD0),
            accent: rgb(0x2B6CB0),
            on_accent: rgb(0xFFFFFF),
            memory: rgb(0xF4C24E),
            sound: rgb(0xFBE3A0),
            possible: rgb(0xEFE6D2),
            ok: rgb(0xCFEBD3),
            edited: rgb(0xD5E4F8),
            unsure: rgb(0xF8D8CC),
            kept: rgb(0xB9B0A5),
            teacher: rgb(0x2F8F5B),
            ai: rgb(0xB7791F),
        }
    }

    pub fn dark() -> Self {
        Self {
            dark: true,
            bg: rgb(0x1B1917),
            panel: rgb(0x242120),
            text: rgb(0xEDE6DC),
            sub: rgb(0xA79C90),
            line: rgb(0x3A3532),
            accent: rgb(0x6AA7F0),
            on_accent: rgb(0x10202F),
            memory: rgb(0x8A6A1E),
            sound: rgb(0x6B5A2A),
            possible: rgb(0x4A4333),
            ok: rgb(0x2D5236),
            edited: rgb(0x2F4968),
            unsure: rgb(0x6A3F31),
            kept: rgb(0x6F665D),
            teacher: rgb(0x58C48A),
            ai: rgb(0xE0A94A),
        }
    }

    pub fn tier_color(&self, tier: crate::data::Tier) -> Color32 {
        match tier {
            crate::data::Tier::Memory => self.memory,
            crate::data::Tier::Sound => self.sound,
            crate::data::Tier::Possible => self.possible,
        }
    }
}

pub const BOLD: &str = "bold";

fn load(family: &str, weight: Weight) -> Option<FontData> {
    let handle = SystemSource::new()
        .select_best_match(
            &[FamilyName::Title(family.to_string())],
            Properties::new().weight(weight),
        )
        .ok()?;
    match handle {
        Handle::Path { path, font_index } => {
            let mut data = FontData::from_owned(std::fs::read(path).ok()?);
            data.index = font_index;
            Some(data)
        }
        Handle::Memory { bytes, font_index } => {
            let mut data = FontData::from_owned(bytes.as_ref().clone());
            data.index = font_index;
            Some(data)
        }
    }
}

/// Returns false when no Traditional-Chinese-capable system font was found.
pub fn install_fonts(ctx: &Context) -> bool {
    let mut fonts = FontDefinitions::default();
    let regular = [
        "PingFang TC",
        "Hiragino Sans CNS",
        "Songti TC",
        "Arial Unicode MS",
    ]
    .iter()
    .find_map(|f| load(f, Weight::NORMAL));
    let bold = ["PingFang TC", "Hiragino Sans CNS"]
        .iter()
        .find_map(|f| load(f, Weight::SEMIBOLD));
    let found = regular.is_some();
    if let Some(data) = regular {
        fonts.font_data.insert("cjk".into(), Arc::new(data));
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            fonts.families.entry(family).or_default().push("cjk".into());
        }
    }
    let bold_key = if let Some(data) = bold {
        fonts.font_data.insert("cjk-bold".into(), Arc::new(data));
        "cjk-bold"
    } else {
        "cjk"
    };
    let mut list = vec![bold_key.to_string()];
    list.extend(fonts.families[&FontFamily::Proportional].iter().cloned());
    fonts.families.insert(FontFamily::Name(BOLD.into()), list);
    ctx.set_fonts(fonts);
    found
}

pub fn apply(ctx: &Context, p: &Palette) {
    ctx.set_visuals(if p.dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    });
    ctx.global_style_mut(|style| {
        let v = &mut style.visuals;
        v.override_text_color = Some(p.text);
        v.panel_fill = p.bg;
        v.window_fill = p.panel;
        v.window_stroke = egui::Stroke::new(1.0, p.line);
        v.extreme_bg_color = p.panel;
        v.selection.bg_fill = p.accent.gamma_multiply(0.35);
        v.selection.stroke = egui::Stroke::new(1.0, p.accent);
        v.hyperlink_color = p.accent;
        v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, p.line);
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        style.spacing.button_padding = egui::vec2(14.0, 8.0);
        style
            .text_styles
            .insert(TextStyle::Body, egui::FontId::proportional(16.0));
        style
            .text_styles
            .insert(TextStyle::Button, egui::FontId::proportional(16.0));
        style
            .text_styles
            .insert(TextStyle::Small, egui::FontId::proportional(13.0));
        style.text_styles.insert(
            TextStyle::Heading,
            egui::FontId::new(26.0, FontFamily::Name(BOLD.into())),
        );
    });
}

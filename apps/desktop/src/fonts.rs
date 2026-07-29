use egui::{FontData, FontDefinitions, FontFamily};
use font_kit::family_name::FamilyName;
use font_kit::properties::{Properties, Style, Weight};
use font_kit::source::SystemSource;

const CJK_FAMILIES: &[&str] = &[
    "PingFang TC",
    "PingFang SC",
    "Hiragino Sans GB",
    "STHeiti",
    "Apple SD Gothic Neo",
    "Noto Sans CJK TC",
];

pub fn install_system_cjk_fonts(ctx: &egui::Context) -> bool {
    let Some(bytes) = load_best_cjk_font_bytes() else {
        return false;
    };

    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "voxproof_system_cjk".to_owned(),
        FontData::from_owned(bytes).into(),
    );
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "voxproof_system_cjk".to_owned());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .push("voxproof_system_cjk".to_owned());
    ctx.set_fonts(fonts);
    true
}

fn load_best_cjk_font_bytes() -> Option<Vec<u8>> {
    let source = SystemSource::new();
    for family_name in CJK_FAMILIES {
        let properties = Properties {
            style: Style::Normal,
            weight: Weight::NORMAL,
            ..Properties::default()
        };
        if let Ok(handle) =
            source.select_best_match(&[FamilyName::Title((*family_name).to_owned())], &properties)
            && let Ok(font) = handle.load()
            && let Some(bytes) = font.copy_font_data()
        {
            return Some(bytes.to_vec());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    #[test]
    fn system_font_lookup_is_bounded_and_does_not_panic() {
        let _ = super::load_best_cjk_font_bytes();
    }
}

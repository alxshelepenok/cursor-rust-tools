use std::{collections::BTreeMap, sync::Arc};

use egui::{FontDefinitions, FontFamily, FontId, TextStyle};
use egui_aesthetix::Aesthetix;

pub fn apply_theme(ctx: &egui::Context) {
    let (fonts, text_styles) = font_definitions();
    ctx.set_fonts(fonts);

    ctx.set_style(Arc::new(egui_aesthetix::themes::NordDark.custom_style()));

    ctx.style_mut(|style| style.text_styles = text_styles);
}

fn font_definitions() -> (FontDefinitions, BTreeMap<TextStyle, FontId>) {
    let fonts = FontDefinitions::default();

    use FontFamily::{Monospace, Proportional};
    (
        fonts,
        [
            (TextStyle::Small, FontId::new(10.0, Proportional)),
            (TextStyle::Body, FontId::new(12.0, Proportional)),
            (TextStyle::Monospace, FontId::new(12.0, Monospace)),
            (TextStyle::Button, FontId::new(12.0, Proportional)),
            (TextStyle::Heading, FontId::new(16.0, Proportional)),
        ]
        .into(),
    )
}

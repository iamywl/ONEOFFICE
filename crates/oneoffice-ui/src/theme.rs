//! Frosted Glass dual-theme tokens and WCAG AAA compliant styling for ONEOFFICE.

use std::sync::{Arc, OnceLock};
use egui::{Color32, FontData, FontDefinitions, FontFamily, Stroke, Visuals};
use serde::{Deserialize, Serialize};

/// Installs native Korean system UI fonts into the egui context to prevent CJK tofu/boxes.
pub fn install_korean_fonts(ctx: &egui::Context) {
    static INSTALLED: OnceLock<()> = OnceLock::new();
    if INSTALLED.get().is_some() {
        return;
    }

    let mut fonts = FontDefinitions::default();
    let probe_chars = &['한', '글', '제', '안', '서', '문', '서', '표', '실', '측', '비', '교'];

    #[cfg(not(target_arch = "wasm32"))]
    if let Some(sys_font) = wordcraft_fonts::system_cjk_ui_font_for(false, true, probe_chars) {
        let mut font_data = FontData::from_owned(sys_font.bytes);
        font_data.index = sys_font.index;
        fonts
            .font_data
            .insert("KoreanSystemUi".to_string(), Arc::new(font_data));

        if let Some(prop) = fonts.families.get_mut(&FontFamily::Proportional) {
            prop.push("KoreanSystemUi".to_string());
        }
        if let Some(mono) = fonts.families.get_mut(&FontFamily::Monospace) {
            mono.push("KoreanSystemUi".to_string());
        }
    }

    ctx.set_fonts(fonts);
    let _ = INSTALLED.set(());
}

/// Active visual theme of the ONEOFFICE Studio.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum StudioTheme {
    #[default]
    Light,
    Dark,
}

impl StudioTheme {
    /// Applies theme tokens to the egui context visuals and ensures Korean fonts are loaded.
    pub fn apply(&self, ctx: &egui::Context) {
        install_korean_fonts(ctx);

        let mut visuals = match self {
            Self::Light => Visuals::light(),
            Self::Dark => Visuals::dark(),
        };

        match self {
            Self::Light => {
                visuals.panel_fill = Color32::from_rgba_premultiplied(248, 250, 252, 245);
                visuals.window_fill = Color32::from_rgba_premultiplied(255, 255, 255, 248);
                visuals.window_stroke = Stroke::new(1.0, Color32::from_rgb(226, 232, 240));
            }
            Self::Dark => {
                visuals.panel_fill = Color32::from_rgba_premultiplied(15, 23, 42, 245);
                visuals.window_fill = Color32::from_rgba_premultiplied(30, 41, 59, 248);
                visuals.window_stroke = Stroke::new(1.0, Color32::from_rgb(51, 65, 85));
            }
        }

        ctx.set_visuals(visuals);
    }

    /// Primary brand blue (`#2563EB`).
    #[must_use]
    pub const fn brand_accent() -> Color32 {
        Color32::from_rgb(37, 99, 235)
    }

    /// Paper surface color (strictly pure white for 100% typographic fidelity).
    #[must_use]
    pub const fn paper_fill() -> Color32 {
        Color32::WHITE
    }

    /// Paper border shadow stroke.
    #[must_use]
    pub const fn paper_border() -> Stroke {
        Stroke {
            width: 1.0,
            color: Color32::from_rgb(203, 213, 225),
        }
    }
}

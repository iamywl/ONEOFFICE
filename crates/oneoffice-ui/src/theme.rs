//! Frosted Glass dual-theme tokens and WCAG AAA compliant styling for ONEOFFICE.

use egui::{Color32, Stroke, Visuals};
use serde::{Deserialize, Serialize};

/// Active visual theme of the ONEOFFICE Studio.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum StudioTheme {
    #[default]
    Light,
    Dark,
}

impl StudioTheme {
    /// Applies theme tokens to the egui context visuals.
    pub fn apply(&self, ctx: &egui::Context) {
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

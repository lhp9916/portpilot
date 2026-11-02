use egui::{Color32, FontId, Stroke, TextStyle};

use crate::model::Theme;

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    pub bg: Color32,
    pub shell: Color32,
    pub rail: Color32,
    pub panel: Color32,
    pub panel_raised: Color32,
    pub panel_soft: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub text: Color32,
    pub text_soft: Color32,
    pub muted: Color32,
    pub faint: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub accent_strong: Color32,
    pub blue: Color32,
    pub blue_soft: Color32,
    pub violet: Color32,
    pub violet_soft: Color32,
    pub green: Color32,
    pub green_soft: Color32,
    pub amber: Color32,
    pub amber_soft: Color32,
    pub red: Color32,
    pub red_soft: Color32,
    pub hover: Color32,
    pub chart_grid: Color32,
}

impl Palette {
    pub fn dark() -> Self {
        Self {
            dark: true,
            bg: rgb(7, 11, 16),
            shell: rgb(11, 17, 25),
            rail: rgb(13, 20, 29),
            panel: rgb(16, 25, 35),
            panel_raised: rgb(20, 32, 44),
            panel_soft: rgb(13, 21, 31),
            border: rgb(30, 44, 58),
            border_strong: rgb(42, 59, 78),
            text: rgb(237, 244, 250),
            text_soft: rgb(195, 208, 220),
            muted: rgb(130, 149, 168),
            faint: rgb(92, 112, 132),
            accent: rgb(53, 214, 193),
            accent_soft: rgba(53, 214, 193, 30),
            accent_strong: rgb(99, 234, 214),
            blue: rgb(101, 168, 255),
            blue_soft: rgba(101, 168, 255, 30),
            violet: rgb(183, 161, 255),
            violet_soft: rgba(183, 161, 255, 30),
            green: rgb(72, 213, 151),
            green_soft: rgba(72, 213, 151, 30),
            amber: rgb(242, 183, 95),
            amber_soft: rgba(242, 183, 95, 30),
            red: rgb(255, 116, 116),
            red_soft: rgba(255, 116, 116, 30),
            hover: rgba(255, 255, 255, 9),
            chart_grid: rgba(130, 149, 168, 28),
        }
    }

    pub fn light() -> Self {
        Self {
            dark: false,
            bg: rgb(233, 238, 243),
            shell: rgb(247, 249, 251),
            rail: rgb(237, 242, 246),
            panel: rgb(255, 255, 255),
            panel_raised: rgb(245, 248, 250),
            panel_soft: rgb(248, 250, 252),
            border: rgb(221, 229, 237),
            border_strong: rgb(203, 214, 225),
            text: rgb(20, 32, 46),
            text_soft: rgb(52, 70, 90),
            muted: rgb(102, 121, 140),
            faint: rgb(139, 154, 171),
            accent: rgb(13, 159, 140),
            accent_soft: rgba(13, 159, 140, 26),
            accent_strong: rgb(8, 127, 112),
            blue: rgb(45, 127, 232),
            blue_soft: rgba(45, 127, 232, 24),
            violet: rgb(118, 92, 210),
            violet_soft: rgba(118, 92, 210, 24),
            green: rgb(21, 154, 100),
            green_soft: rgba(21, 154, 100, 24),
            amber: rgb(183, 118, 13),
            amber_soft: rgba(183, 118, 13, 26),
            red: rgb(217, 71, 71),
            red_soft: rgba(217, 71, 71, 24),
            hover: rgba(20, 32, 46, 9),
            chart_grid: rgba(102, 121, 140, 40),
        }
    }

    pub fn for_theme(theme: Theme, system: Option<egui::Theme>) -> Self {
        let dark = match theme {
            Theme::Dark => true,
            Theme::Light => false,
            Theme::System => system.unwrap_or(egui::Theme::Dark) == egui::Theme::Dark,
        };
        if dark { Self::dark() } else { Self::light() }
    }

    pub fn status_color(&self, status: &crate::engine::RuleStatus) -> Color32 {
        match status {
            crate::engine::RuleStatus::Running => self.green,
            crate::engine::RuleStatus::Starting => self.accent,
            crate::engine::RuleStatus::Stopping => self.muted,
            crate::engine::RuleStatus::Failed(_) => self.red,
            crate::engine::RuleStatus::Stopped => self.faint,
        }
    }
}

pub fn apply(ctx: &egui::Context, palette: Palette) {
    let egui_theme = if palette.dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    };
    ctx.set_theme(if palette.dark {
        egui::ThemePreference::Dark
    } else {
        egui::ThemePreference::Light
    });
    let mut style = (*ctx.style_of(egui_theme)).clone();
    let mut visuals = if palette.dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    visuals.override_text_color = Some(palette.text);
    visuals.panel_fill = palette.shell;
    visuals.window_fill = palette.panel;
    visuals.extreme_bg_color = palette.panel_soft;
    visuals.faint_bg_color = palette.panel_raised;
    visuals.code_bg_color = palette.panel_soft;
    visuals.hyperlink_color = palette.accent;
    visuals.warn_fg_color = palette.amber;
    visuals.error_fg_color = palette.red;
    visuals.window_stroke = Stroke::new(1.0, palette.border_strong);
    visuals.selection.bg_fill = palette.accent_soft;
    visuals.selection.stroke = Stroke::new(1.0, palette.accent);

    visuals.widgets.noninteractive.bg_fill = palette.panel;
    visuals.widgets.noninteractive.weak_bg_fill = palette.panel;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette.border);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.muted);

    visuals.widgets.inactive.weak_bg_fill = palette.panel_raised;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, palette.border);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, palette.text_soft);

    visuals.widgets.hovered.weak_bg_fill = palette.hover;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, palette.border_strong);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, palette.text);

    visuals.widgets.active.weak_bg_fill = palette.accent_soft;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, palette.accent);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, palette.text);

    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.spacing.interact_size.y = 30.0;
    style
        .text_styles
        .insert(TextStyle::Body, FontId::proportional(12.5));
    style
        .text_styles
        .insert(TextStyle::Button, FontId::proportional(12.0));
    style
        .text_styles
        .insert(TextStyle::Small, FontId::proportional(10.0));
    style
        .text_styles
        .insert(TextStyle::Heading, FontId::proportional(19.0));
    style
        .text_styles
        .insert(TextStyle::Monospace, FontId::monospace(11.0));
    ctx.set_style_of(egui_theme, style);
}

const fn rgb(red: u8, green: u8, blue: u8) -> Color32 {
    Color32::from_rgb(red, green, blue)
}

fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(red, green, blue, alpha)
}

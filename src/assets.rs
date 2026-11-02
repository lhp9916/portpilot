use std::sync::Arc;

use egui::{FontData, FontDefinitions, FontFamily};
use tray_icon::Icon;

pub const ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");
pub const NOTO_SANS_SC: &[u8] = include_bytes!("../assets/fonts/NotoSansSC-Regular.otf");

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "noto_sans_sc".to_owned(),
        Arc::new(FontData::from_static(NOTO_SANS_SC)),
    );

    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "noto_sans_sc".to_owned());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .push("noto_sans_sc".to_owned());

    ctx.set_fonts(fonts);
}

pub fn egui_icon() -> egui::IconData {
    let image = image::load_from_memory(ICON_PNG)
        .expect("embedded icon must decode")
        .into_rgba8();
    egui::IconData {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    }
}

pub fn tray_icon() -> Result<Icon, String> {
    let image = image::load_from_memory(ICON_PNG)
        .map_err(|error| format!("decode icon: {error}"))?
        .into_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height).map_err(|error| format!("build icon: {error}"))
}

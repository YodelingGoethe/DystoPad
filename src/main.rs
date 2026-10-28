// No console window next to the app in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod editor;
mod files;
mod format;
mod highlight;
mod i18n;
mod icons;
mod markdown;
mod preview;
mod print;
mod search;
mod session;
mod single_instance;
mod theme;

use std::path::PathBuf;

fn main() -> eframe::Result {
    let files: Vec<PathBuf> = std::env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .map(|p| std::path::absolute(&p).unwrap_or(p))
        .collect();
    if single_instance::hand_over(&files) {
        return Ok(());
    }
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("DystoPad")
        .with_app_id("dystopad")
        .with_inner_size([1440.0, 900.0])
        .with_min_inner_size([900.0, 540.0])
        .with_drag_and_drop(true);
    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon/icon.png")) {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "DystoPad",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, files)))),
    )
}

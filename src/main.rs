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
#[cfg(windows)]
mod win32;

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
    let options = eframe::NativeOptions {
        viewport: app::viewport().with_inner_size([1440.0, 900.0]),
        #[cfg(windows)]
        event_loop_builder: Some(Box::new(|builder| {
            use winit::platform::windows::EventLoopBuilderExtWindows;
            builder.with_msg_hook(win32::hook);
        })),
        ..Default::default()
    };
    eframe::run_native(
        "DystoPad",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, files)))),
    )
}

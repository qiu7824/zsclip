#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod cloud_sync;
mod db_runtime;
mod gdiplus;
mod hover_preview;
#[path = "i18n_runtime.rs"]
mod i18n;
mod mail_merge_native;
mod settings_model;
mod settings_render;
mod settings_ui_host;
mod shell;
mod sticker;
mod time_utils;
mod tray;
mod ui;
mod win_buffered_paint;
mod win_system_params;
mod win_system_ui;

fn main() {
    if let Some(code) = shell::maybe_run_wechat_ocr_helper_from_args() {
        std::process::exit(code);
    }
    if let Err(err) = app::run() {
        eprintln!("error: {err}");
    }
}

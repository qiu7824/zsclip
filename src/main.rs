#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod gdiplus;
#[path = "i18n_runtime.rs"]
mod i18n;
mod ui;
mod win_system_ui;
mod win_system_params;
mod settings_ui_host;
mod settings_model;
mod settings_render;
mod shell;
mod hover_preview;
mod mail_merge_native;
mod sticker;
mod tray;
mod db_runtime;
mod cloud_sync;
mod time_utils;
mod win_buffered_paint;
mod app;

fn main() {
    if let Some(code) = shell::maybe_run_wechat_ocr_helper_from_args() {
        std::process::exit(code);
    }
    if let Err(err) = app::run() {
        eprintln!("error: {err}");
    }
}

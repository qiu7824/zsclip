#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod gdiplus;
mod ui;
mod ui_core;
mod settings_ui_host;
mod settings_model;
mod settings_render;
mod settings_layout;
mod settings_registry;
mod shell;
mod mail_merge_native;
mod sticker;
mod tray;
mod window_position;
mod db_runtime;
mod time_utils;
mod win_buffered_paint;
mod app;

fn main() {
    if let Err(err) = app::run() {
        eprintln!("error: {err}");
    }
}

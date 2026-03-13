#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod gdiplus;
mod ui;
mod settings;
mod settings_layout;
mod settings_framework;
mod settings_registry;
mod shell;
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

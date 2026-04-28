# AGENTS.md

## Build & Run

- **Prerequisites**: Rust toolchain (via rustup) + MSVC Build Tools with "Desktop development with C++" workload.
- **Dev**: `cargo run`
- **Release**: `cargo build --release` (or `.\build_release.ps1`)
- `build.rs` embeds `assets/icons/app.rc` (icon + DPI-aware manifest on Windows only).

## Architecture

This is a **Windows-only Rust desktop app** — raw Win32 API via `windows-sys` 0.59, no GUI framework.

| Layer | Key Files |
|---|---|
| Entry | `src/main.rs` → `app::run()` |
| Core monolith | `src/app.rs` (~11k lines) — WndProcs, state machine, clipboard, data flow |
| App submodules | `src/app/data.rs` (DB items/queries), `src/app/hosts.rs` (window creation/UI controls), `src/app/runtime.rs` (paths/crypto/settings save), `src/app/state.rs` (app state structs) |
| Win32 UI host | `src/win_system_ui.rs`, `src/win_system_params.rs`, `src/win_buffered_paint.rs` |
| Settings framework | `src/settings_model.rs`, `src/settings_render.rs`, `src/settings_ui_host.rs` |
| Database | `src/db_runtime.rs` — SQLite (rusqlite `bundled`), thread-local connections, `OnceLock` schema migration |
| i18n | `src/i18n_runtime.rs` (aliased as `mod i18n`) — Chinese source keys, JSON translation files |
| Misc | `tray.rs`, `sticker.rs`, `shell.rs`, `hover_preview.rs`, `cloud_sync.rs`, `mail_merge_native.rs`, `gdiplus.rs`, `time_utils.rs` |

## Key Conventions & Gotchas

- **`#![windows_subsystem = "windows"]`** — release builds have no console; `eprintln!` errors are invisible. Debug with `cargo run`.
- **`app.rs` is the monolith**. Most logic lives there. The `app/` subdirectory only splits data/hosts/runtime/state — it doesn't contain the WndProc itself.
- **i18n module alias**: `#[path = "i18n_runtime.rs"] mod i18n;` — import as `crate::i18n::*`.
- **Translation keys are Chinese text**. The JSON maps Chinese source strings to translated values. Placeholders use `{variable}` syntax.
- **VERSION.txt** is a build marker string (not semver). Real version is in `Cargo.toml`.
- **Data directory**: prefers `exe/data/`, falls back to `%LOCALAPPDATA%\ZSClip\data`.
- **DB connections are thread-local** (`thread_local!` + `RefCell<Option<Connection>>`). Call `db_runtime::with_db()` to get a connection — do not create your own.
- **GDI+ must be initialized** via `gdiplus.rs` before any drawing calls.
- **Raw Win32 extern links** exist directly in `app.rs` (user32 functions like `RegisterHotKey`) and `i18n_runtime.rs` (advapi32). These are not behind a wrapper crate.
- **No automated tests or CI** exist in this repo.

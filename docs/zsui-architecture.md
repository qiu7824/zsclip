# ZSUI Architecture

ZSUI is a Rust-first native system UI framework shape for ZSClip and future
Rust tools. It is not a self-drawn UI framework and it is not a browser shell.
The application writes one Rust declaration. Platform hosts translate that
declaration into native Windows, macOS and Linux capabilities.

## Module Shape

The public API lives in the standalone `zsui` crate (`E:\rust\zsui/src/`).
ZSClip keeps `zsclip::zsui` as a crate re-export so existing product and host
code can migrate without changing every call site at once:

- `mod.rs`: public exports.
- `core.rs`: ids, `Command`, `AppEvent`, dialogs, file picker specs and
  `ZsuiError`.
- `app.rs`: `app()`, `AppBuilder`, `ZsuiApp` and `ZsuiAppRuntime`.
- `window.rs`: `WindowSpec` and the `Window` builder alias.
- `tray.rs`: `TraySpec`.
- `menu.rs`: `MenuSpec` and `MenuItemSpec`.
- `hotkey.rs`: `HotkeySpec`.
- `clipboard.rs`: `ClipboardData`.
- `settings.rs`: `SettingsPageSpec`, `SettingsItemSpec` and settings values.
- `capability.rs`: `HostCapabilities` and per-capability support state.
- `host.rs`: `ZsuiHost`, `MemoryHost` and the current `PlatformHost` scaffold.

`src/app_core/*` remains the richer current ZSClip protocol layer. It already
contains many native host contracts and product adapter plans. The external
`zsui` crate is the smaller public facade that application authors and AI
agents can use as the stable starting point.

## Host Trait Boundary

Application code should call `ZsuiHost`, not Win32, AppKit or GTK directly.
The trait contains the first cross-platform surface:

- create a main window
- show or hide a window
- create a tray/status menu
- register a global hotkey
- read and write clipboard data
- open a file picker
- show a native dialog
- query `HostCapabilities`
- poll app events and enter the host event loop

When a feature is missing, the backend returns `ZsuiError::Unsupported`.
Capability checks are part of normal control flow, especially for global
hotkeys, tray/status menus, clipboard payload types and auto paste.
Window capabilities are split into the general native window surface plus
feature-level support for resize policy, native decorations, always-on-top and
transparent windows, so a host can accept the declaration and still report the
native fallback it used.

The standalone crate also includes `NativeWindowHost` plus the convenience
`native_window("Title").run()` builder for a minimal real native window on
Windows, macOS and Linux. Full product hosts still own native controls, menus,
dialogs and product event routing.
Android and Harmony are present as explicit mobile capability scaffolds, not
complete runtime hosts yet.

## Data Model Rules

The public specs are plain Rust data:

- no platform handles
- no complex generics
- serde-friendly where practical
- debug-friendly names and command enums
- product behavior represented as `Command`, not embedded closures

This makes the API suitable for AI-assisted generation. A tool can inspect or
emit a window, tray, menu, hotkey or settings declaration without reading the
Windows message loop or ZSClip database code.

## Product Boundary

ZSUI core must not own ZSClip business features:

- clipboard history database
- AI provider execution
- WPS integrations
- LAN/WebDAV sync
- mail merge
- product-specific settings persistence

Those belong to product adapters and application modules. ZSUI only carries
the native UI declarations, commands, events, settings surface shape and host
capability model.

## Migration Path

1. Keep the existing Windows main flow running through `src/app/*` and
   `src/platform/*`.
2. Use the standalone `zsui` crate for new declaration tests and examples.
3. Gradually adapt Windows platform modules to implement `ZsuiHost` operations
   for real windows, tray menus, hotkeys, clipboard, file picker and dialogs.
4. Keep macOS and Linux conforming to the same trait with partial or stub
   implementations that return `ZsuiError::Unsupported` when needed.
5. Move common app declarations upward only after the host capability path is
   verified on each target.

The goal is not to make every platform identical. The goal is one Rust UI API
with native system behavior and honest capability-based degradation.

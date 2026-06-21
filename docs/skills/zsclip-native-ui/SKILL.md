---
name: zsclip-native-ui
description: Work on ZSClip's shared Rust UI contracts and native Windows Win32, macOS AppKit/SwiftUI, and Linux GTK/libadwaita hosts. Use this when modifying or verifying native UI features such as right-click edit, group menus, VV popup/paste, settings pages, clipboard, status/tray menus, shell-open, file picker, target smoke tests, or when an AI agent needs to understand how to call ZSClip UI without duplicating product behavior.
---

# ZSClip Native UI

Use this skill to work on the native UI host layer without turning AppKit or GTK into a second copy of the product.

## Quick Start

1. Read `references/native-ui-entrypoints.md` first for the file map and current completion vocabulary.
2. Read `docs/zsui.md` for the architecture boundary.
3. Read `docs/ui-host-porting.md` when adding or changing host surfaces.
4. Read `docs/native-host-verification.md` before claiming macOS or Linux completion.
5. Inspect the relevant Rust entry points instead of guessing from UI labels.

## Layer Rules

- Keep reusable contracts, action plans, layout/render plans and parity metadata in `src/app_core/`.
- Keep ZSClip behavior, DB mutations, settings persistence, sync, and AI execution in the product adapter or product modules.
- Keep AppKit/GTK host files thin: create native windows/widgets, wire callbacks, call shared actions, and invoke host traits.
- Do not create a new cross-platform UI framework for a feature that already has a shared action plan or host trait.
- Do not report a macOS/Linux feature as complete just because Windows-side tests pass. Use code-level, target-smoke, and system-complete separately.

## Common Workflow

1. Identify the feature surface: main window, row menu, group management, VV, settings, clipboard, status/menu, dialog, shell-open, file picker, or identity.
2. Check the shared contract in `src/app_core/` before editing platform code.
3. Edit the platform host only for native presentation or OS service calls:
   - macOS: `src/macos_app.rs`, `src/macos_native_host.rs`, `src/macos_appkit_adapter.rs`
   - Linux: `src/linux_app.rs`, `src/linux_native_host.rs`, `src/linux_gtk_adapter.rs`
   - Windows reference host: `src/app.rs`, `src/windows_win32_adapter.rs`
4. Route behavior through shared actions such as `NativeHostRowAction`, `NativeHostSettingsAction`, `NativeHostStatusMenuAction`, `NativeHostVvPastePlan`, product command ids, and host traits.
5. Update docs and source guards when a new host surface, smoke log, or platform proof expectation is added.
6. Run local Rust checks, then require target OS smoke artifacts before marking macOS/Ubuntu runtime completion.

## Completion Reporting

When answering progress questions, separate:

- `code-level`: shared Rust contract and native host route exist and local tests pass.
- `target-smoke`: the real AppKit/GTK process produced logs/screenshots on the target OS.
- `system-complete`: the OS integration is proven, including permissions, focus handoff, real paste delivery, tray/status behavior, or interactive dialogs where relevant.

Use `zsui_native_feature_parity_statuses()` and `docs/native-host-verification.md` as the source of truth for feature progress. If the current machine is Windows, say that AppKit/GTK runtime proof still requires macOS/Ubuntu artifacts.

## Verification

Local checks:

```powershell
cargo fmt --check
cargo check
cargo test
```

Target smoke checks:

```bash
bash scripts/native-host-smoke-macos.sh
bash scripts/native-host-smoke-linux.sh
```

Store and inspect target artifacts under `target/native-host-smoke/<platform>/` before calling the platform verified.

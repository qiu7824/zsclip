# Native UI Entrypoints

This reference is the short map an AI agent should use before changing or judging ZSClip native UI work.

## What To Send Another AI

If the AI has repository access, send this skill folder:

- `docs/skills/zsclip-native-ui/`

If the AI cannot load a skill folder, send these files together:

- `docs/skills/zsclip-native-ui/SKILL.md`
- `docs/skills/zsclip-native-ui/references/native-ui-entrypoints.md`
- `docs/zsui.md`
- `docs/ui-host-porting.md`
- `docs/native-host-verification.md`
- `src/app_core/native_host_actions.rs`
- `src/app_core/host_protocol.rs`
- `src/app_core/native_adapter_manifest.rs`
- `src/macos_app.rs`
- `src/macos_native_host.rs`
- `src/linux_app.rs`
- `src/linux_native_host.rs`

## Main Source Entrypoints

| Need | Start here |
| --- | --- |
| Architecture boundary | `docs/zsui.md` |
| Porting contract and host surfaces | `docs/ui-host-porting.md` |
| Target OS proof and smoke commands | `docs/native-host-verification.md` |
| Feature parity progress | `zsui_native_feature_parity_statuses()` in `src/app_core/native_adapter_manifest.rs` |
| Shared row, menu, settings, status, VV actions | `src/app_core/native_host_actions.rs` |
| Clipboard, dialog, shell-open, file-picker, paste-target traits | `src/app_core/host_protocol.rs` |
| Product command routing | `src/zsclip_product_adapter.rs` and `src/app_core/product_adapter.rs` |
| macOS AppKit host | `src/macos_app.rs`, `src/macos_native_host.rs`, `src/macos_appkit_adapter.rs` |
| Linux GTK host | `src/linux_app.rs`, `src/linux_native_host.rs`, `src/linux_gtk_adapter.rs` |
| Windows reference host | `src/app.rs`, `src/windows_win32_adapter.rs` |

## Feature Status Vocabulary

Use these states in reports:

- Code-level ready: Rust contracts, adapters and host callbacks exist; local tests pass.
- Target smoke verified: target macOS/Ubuntu run produced logs/screenshots.
- System complete: real OS behavior is proven, including permissions and focus/desktop integration.

Do not collapse these states. For example, a VV paste bridge can be code-level ready while still needing target proof that `CGEvent`, `ydotool`, or `xdotool` actually delivers the paste into the focused app.

## Current Native Host Shape

Windows is the integrated reference host.

macOS and Linux are first-pass native hosts. They have real target-only AppKit/GTK entry points and shared route wiring, but their runtime completion depends on target artifacts.

Already wired at code level for macOS and Linux:

- Main native window with DB-backed rows.
- Search route and native search control.
- Row actions and row menu routing.
- Right-click edit/save route.
- Group create, rename, delete, reorder, assign and filter routes.
- Settings window, shared settings summaries and bound Save collection.
- Status/menu actions.
- Dialog actions.
- Clipboard text/image/file path bridge and monitor probe.
- VV popup/select and VV paste bridge.
- Shell-open and file-picker host boundaries.
- Window/paste-target identity host boundaries with target System Events probes on macOS and `xdotool`/`xprop` probes on Linux.

Still requiring target proof before system-complete claims:

- Real AppKit/GTK screenshots and interaction artifacts.
- Ubuntu StatusNotifierItem artifact.
- Real global VV trigger integration.
- Proof paste shortcut delivery succeeds under macOS Accessibility or Linux desktop permissions.
- Non-dry-run shell-open handoff where safe.
- Interactive file picker proof.
- Long-running clipboard monitor/source identity behavior.
- Target smoke evidence for window identity and paste-target identity under macOS Accessibility and Linux desktop permissions, plus Wayland/AT-SPI coverage where needed.

## Editing Rules

- Add shared behavior to `app_core` or the product adapter first.
- Add only platform presentation and OS API handoff to AppKit/GTK host files.
- Prefer existing host traits and action enums before introducing a new abstraction.
- Add or update tests for shared routing and source guards when a new native host surface is introduced.
- Update `docs/native-host-verification.md` whenever target proof expectations or smoke logs change.

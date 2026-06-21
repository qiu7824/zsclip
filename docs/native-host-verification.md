# Native Host Verification

This document is the target-OS evidence path for the macOS AppKit and Linux GTK native hosts.
Windows-side Rust tests prove that the dispatch bridges compile and route to the product adapter, but they do not prove that AppKit or GTK windows render and respond on the real platforms.

## What Counts As Verified

A platform is verified only when the target OS produces these artifacts:

- Cargo target tests pass for that platform's native host launch plan and product-command bridges.
- The `zsclip` binary builds on the target OS.
- The real native host launches and stays alive long enough for a screenshot.
- A screenshot shows the native main window, status/menu entry and the expected controls.
- Optional click smoke captures evidence that native controls and status/menu entries can be clicked and that the UI updates or logs product route results.

Store the artifacts under `target/native-host-smoke/<platform>/`.

The repository also includes `.github/workflows/native-hosts.yml`. It runs the same target host checks on GitHub-hosted `macos-latest` and `ubuntu-latest` runners, installs GTK dependencies on Ubuntu, runs the smoke scripts and uploads the native-host smoke artifacts. Treat those CI artifacts as stronger evidence than Windows-side source checks, but still inspect screenshots/logs before calling a platform verified.

## Reuse Boundary

The reusable layer is the shared contract surface in `app_core`: command plans, host traits, settings protocols, render/input plans and product-adapter routes. `src/macos_native_host.rs` and `src/linux_native_host.rs` are intentionally thin platform hosts. They should create native controls, wire native callbacks and call shared actions, but they should not own clipboard history rules, settings persistence, sync behavior or ZSClip database mutations.

Platform services stay behind dedicated host traits:

| Capability | Shared contract | Native host responsibility |
| --- | --- | --- |
| Clipboard read/write | `ClipboardHost` | Map to `NSPasteboard`, GTK/GDK or desktop portal APIs. |
| File picker | `NativeFileDialogHost` | Present `NSOpenPanel` or a GTK file chooser and return a path/cancel/error. |
| Message/confirm dialog | `NativeDialogHost` | Present `NSAlert` or GTK `MessageDialog`. |
| Shell open | `NativeShellOpenHost` | Open trusted URLs/files with platform services. |
| Main/settings controls | `NativeMainWindowHost`, `NativeSettingsWindowHost`, `NativeSettingsControlHost` | Create native windows/widgets and report shared command results. |

Product actions such as paste sound selection may call a file-picker host, but the native UI host should not directly save settings or touch clipboard records. That keeps the AppKit/GTK code reusable as a host adapter instead of turning it into a ZSClip business layer.

## Current Completion Snapshot

This is the current implementation state as of the Windows-side audit. It is not final target-OS proof until the macOS/Linux smoke scripts below produce artifacts on those systems. The AppKit host is now also covered by a Windows-side `x86_64-apple-darwin` type check when Zig is available as the C toolchain for bundled sqlite.

| Area | macOS AppKit | Linux GTK | Notes |
| --- | --- | --- | --- |
| Real native process entry | First-pass target-only AppKit host entry exists | First-pass target-only GTK host entry exists | Windows builds use scaffold fallback for non-target tests. |
| Main native window | First-pass window with shared route labels, selectable DB-backed rows, row actions and DB-backed Row Menu group assignment; shared edit-text host can present an AppKit Save/Cancel editor | First-pass window with shared route labels, selectable DB-backed rows, row actions and DB-backed Row Menu group assignment; shared edit-text host can present a GTK Save/Cancel editor | Needs target screenshots for proof. |
| Search UI | Native `NSSearchField` route exists | Native `SearchEntry` route exists | Routes through shared search action/result. |
| Settings surface | First-pass settings window/buttons exists; shared six-page settings summaries plus routed/bound control blueprints are rendered; Save collects all currently bound native text/toggle/dropdown values, applies/persists through the shared settings model and logs the shared apply/collect plan; group management panel can create, rename, delete and reorder SQLite groups | First-pass settings window/buttons exists; shared six-page settings summaries plus a GTK routed/bound control-blueprint preview are rendered; Save collects all currently bound native text/toggle/dropdown values, applies/persists through the shared settings model and logs the shared apply/collect plan; group management panel can create, rename, delete and reorder SQLite groups | Target screenshots and full-section native control coverage for every shared settings section are still pending. |
| Status/menu entry | `NSStatusItem` + `NSMenu` route exists; Row Menu and Group Filter map real SQLite groups to `NSMenu`; settings group changes refresh the DB-backed list and reset deleted active filters; VV Popup opens a first-pass native window from the shared render plan, its row buttons bridge `vv_select_requested`, and the selected item enters the native VV paste bridge | GTK `StatusNotifierItem` route plus in-window GTK/GIO fallback exists; Row Menu and Group Filter map real SQLite groups to `PopoverMenu`; settings group changes refresh the GTK popup models and reset deleted active filters; VV Popup opens a first-pass native window from the shared render plan, its row buttons bridge `vv_select_requested`, and the selected item enters the native VV paste bridge | Linux target screenshots, StatusNotifier host artifact and real VV trigger smoke are still pending. |
| Simple dialogs | First-pass `NSAlert` Info/Confirm route exists; text prompts use `NSAlert` with an editable `NSTextField` accessory view | First-pass GTK `MessageDialog` Info/Confirm route exists; text prompts use a modal GTK `Dialog` with an editable `Entry` | Needs real target click smoke. |
| Platform links | Open Source / Check Updates / WPS Docs routes exist; target host uses `NSWorkspace.openURL` for shell-open handoff | Open Source / Check Updates / WPS Docs routes exist; target host uses GIO `AppInfo::launch_default_for_uri` for shell-open handoff | Shell-open behavior still needs target verification. |
| File picker | Target-only `NSOpenPanel` adapter exists; default smoke injects a safe selected path through `NativeFileDialogHost` | Target-only GTK `FileChooserNative` adapter exists; default smoke injects a safe selected path through `NativeFileDialogHost` | Interactive native picker UI still needs manual target proof. |
| Clipboard monitoring | `ClipboardHost` now has AppKit-side `NSPasteboard` file URL read/write and `changeCount` code paths; `MacosApplicationModel` polls sequence changes into the shared `clipboard_changed` product event | `ClipboardHost` now has a target Linux system clipboard fingerprint path that can advance the shared sequence counter; `LinuxApplicationModel` polls sequence changes into the shared `clipboard_changed` product event | Smoke checks now cover one observed monitor change; long-running target monitoring and source identity remain platform work. |
| Paste target identity | Shared identity/readiness snapshot plus AppKit contract host exists; target host can query frontmost process/name/bundle through System Events and can request foreground/paste shortcut through the paste-target host boundary | Shared identity/readiness snapshot plus GTK contract host exists; target host can query foreground window, pid, process name and WM_CLASS through `xdotool`/`xprop` and can request foreground/paste shortcut through the paste-target host boundary | Needs target smoke evidence under real macOS Accessibility and Linux X11/desktop permissions; Wayland/AT-SPI coverage remains separate. |

## Function Parity Matrix

This matrix separates three states that are easy to confuse during porting:

- Code-level: the shared Rust contract and AppKit/GTK route exist and are covered by local tests.
- Target smoke: the real AppKit/GTK process is launched on the target OS and produces logs/screenshots for the route.
- System-complete: the route also proves the OS integration, such as global trigger, focus handoff, real paste delivery or desktop shell handoff.

| Feature | macOS AppKit code-level | Linux GTK code-level | Target smoke evidence | Still missing before calling complete |
| --- | --- | --- | --- | --- |
| Main window and DB-backed row list | Yes | Yes | Script path exists; target artifacts still required | Broader visual/interaction screenshots on real macOS and Ubuntu. |
| Search text route | Yes | Yes | Covered by optional click smoke when target run succeeds | More keyboard/focus behavior on target OS. |
| Row actions and Row Menu | Yes | Yes | Smoke verifies representative routes; full menu click coverage still limited | More target proof for every menu command and submenu path. |
| Right-click edit/save | Yes | Yes | Edit window/save route is included in smoke expectations | Target proof that refresh and unsaved-change behavior match native expectations. |
| Group create/rename/delete/reorder/filter | Yes | Yes | Route coverage exists; target popup screenshots still required | Real target menu refresh proof after every group mutation. |
| VV popup/select | First-pass native popup and select bridge | First-pass native popup and select bridge | Smoke checks manual/native preview path | Real global VV trigger integration. |
| VV paste | Selected item writes native clipboard and AppKit host attempts `CGEvent` Backspace / `Cmd+V` delivery | Selected item writes native clipboard and GTK host attempts `ydotool` or X11 `xdotool` Backspace / `Ctrl+V` delivery | Smoke checks the paste bridge and native shortcut attempt log | Target proof that shortcut delivery succeeds under permissions, focus restoration proof and target-window identity. |
| Clipboard text/image/file paths | Text/image plus AppKit file URL path | Text/image plus first-pass file path memory/clipboard path | Smoke probes text and file-path read/write | Linux GDK/portal file URL support, long-running source identity and monitor filtering. |
| Status/tray menu | Real `NSStatusItem` route | `StatusNotifierItem` route plus in-window GTK/GIO fallback | Smoke checks route logs and whether SNI installed or reported unavailable | Target StatusNotifierHost artifact on Ubuntu desktop. |
| Shell open | `NSWorkspace.openURL` target path | GIO `AppInfo::launch_default_for_uri` target path | Not opened by default to avoid CI hangs | Safe target smoke for trusted URL/file handoff. |
| File picker | `NSOpenPanel` target adapter | GTK `FileChooserNative` target adapter | Smoke checks injected selected-path boundary | Interactive target file picker smoke. |
| Settings pages | First-pass buttons, six shared page/section/control summaries with shared command/action routes, native text/toggle/dropdown bindings, full bound-value collect on Save and shared apply/persist plan, plus group management | First-pass buttons, six shared page/section/control summaries with shared command/action routes, native text/toggle/dropdown bindings, full bound-value collect on Save, shared apply/persist plan, native control-blueprint preview and group management | Representative routes only | Target screenshots and full-section native control coverage for every shared settings section. |
| Window/paste-target identity | Shared snapshot, AppKit contract route and target System Events process/foreground probe boundary | Shared snapshot, GTK contract route and target `xdotool`/`xprop` process/window probe boundary | Local contract tests only | Target smoke evidence under macOS Accessibility and Linux X11 permissions, plus Wayland/AT-SPI coverage where needed. |

## macOS AppKit

Windows-side AppKit type check:

```powershell
rustup target add x86_64-apple-darwin
$env:CC_x86_64_apple_darwin='zig cc -target x86_64-macos'
$env:AR_x86_64_apple_darwin='zig ar'
$env:CRATE_CC_NO_DEFAULTS='1'
cargo check --target x86_64-apple-darwin --target-dir target_macos_appkit_check
```

This check compiles the `#[cfg(target_os = "macos")]` AppKit host and catches Objective-C binding/type errors, but it still does not launch the real app or prove Accessibility, pasteboard, global hotkey or screenshot behavior.

Run on macOS:

```bash
bash scripts/native-host-smoke-macos.sh
```

Expected default artifacts:

- `target/native-host-smoke/macos/zsclip-appkit.log`
- `target/native-host-smoke/macos/zsclip-appkit-main.png`

By default the script launches the AppKit host with `ZSCLIP_NATIVE_HOST_AUTO_SMOKE=1` and `ZSCLIP_NATIVE_HOST_SHELL_OPEN_DRY_RUN=1`. The real native process writes and reads text and file-path clipboard probes, checks that the clipboard sequence changes after a write, polls one clipboard monitor change into the shared `clipboard_changed` event path, records a safe shell-open host dry-run for a temporary file path, injects a safe file-picker smoke path through `ZSCLIP_NATIVE_HOST_FILE_PICKER_SMOKE_PATH`, runs the System Events window/paste-target identity smoke boundary, opens the settings surface, dispatches representative settings controls, row actions, VV select, native VV paste, AppKit native paste-shortcut attempt and status-menu routes, then the script verifies those product-route/event-route lines in `zsclip-appkit.log`. Set `ZSCLIP_NATIVE_HOST_AUTO_SMOKE=0` only when debugging a manual launch.

Optional click smoke:

```bash
NATIVE_HOST_SMOKE_CLICK=1 bash scripts/native-host-smoke-macos.sh
```

The optional click smoke uses AppleScript through System Events. If it fails with an accessibility or permission error, grant the terminal app Accessibility permission in macOS System Settings and rerun. It clicks Search, Settings, settings toggles, row actions, Row Menu, Group Filter, VV Popup, VV Select 1 and the status menu, then checks `zsclip-appkit.log` for the expected product-route/event-route lines. A successful optional click smoke also writes:

- `target/native-host-smoke/macos/zsclip-appkit-after-clicks.png`

The current AppKit host should show the main `ZSClip` window, a `ZSClip` `NSStatusItem` in the macOS menu bar, the native search field after Search is clicked, the Settings window after Settings is clicked, shared settings page summaries for General, Hotkey, Plugin, Group, Cloud and About with section/control/control-row counts plus a compact shared-control blueprint list. Executable blueprint controls display their shared `window.settings.*` command route or settings action route, and text/list controls display their shared setting-field/runtime-list binding. Pressing Save collects all currently bound native text/toggle/dropdown values, applies/persists them through the shared settings model and logs the apply/collect plan summary. The remaining settings work is target screenshot evidence and full-section native control coverage. The settings window also exposes controls such as Capture, LAN Sync and Sync Mode, a Group Management panel with Records/Phrases, Add, Rename, Delete, Up and Down actions backed by SQLite, platform actions such as Open Source, Check Updates and WPS Docs, native dialog buttons such as Info Dialog and Confirm, row action buttons such as Copy, Edit and Translate, a native `ZSClip Edit` window for the Edit action, a Row Menu button backed by shared `NativePopupMenuEntry` row actions including the Add To Group submenu populated from `clip_groups`, a Group Filter button backed by real group-filter popup entries, and a VV Popup button that opens a native window from the shared VV render plan. The row group submenu should update `items.group_id` for the selected history row and refresh the DB-backed list; the Group Filter menu should refresh the visible DB-backed list for the selected group. The VV popup row buttons should dispatch the shared `vv_select_requested` event bridge and then write the selected item into the native clipboard/paste-target bridge. The status menu exposes Show ZSClip, Toggle Capture, Toggle LAN Sync and Exit through the same product routes as the Windows tray menu.

The macOS shell-open host records shell-open requests for tests and uses `NSWorkspace.openURL` in non-test target builds. The default smoke script sets `ZSCLIP_NATIVE_HOST_SHELL_OPEN_DRY_RUN=1`, so it proves the real AppKit host reaches the shell-open boundary and records the target path without launching an external app. Set `ZSCLIP_NATIVE_HOST_SHELL_OPEN_DRY_RUN=0` only for a manual target handoff check.

The macOS file-picker host records file-picker requests for tests and uses `NSOpenPanel` in non-test target builds. The default auto smoke temporarily sets `ZSCLIP_NATIVE_HOST_FILE_PICKER_SMOKE_PATH` to a temporary file path, so it proves the real AppKit host reaches the file-picker boundary without opening an interactive panel. Leave that environment variable unset for a manual target picker check.

The `macos-appkit` GitHub Actions job runs the same smoke script and uploads `native-host-smoke-macos`.

## Linux GTK

Run inside a Linux desktop session with `DISPLAY` or `WAYLAND_DISPLAY` set:

```bash
bash scripts/native-host-smoke-linux.sh
```

Expected default artifacts:

- `target/native-host-smoke/linux/zsclip-gtk.log`
- `target/native-host-smoke/linux/zsclip-gtk-main.png`

By default the script launches the GTK host with `ZSCLIP_NATIVE_HOST_AUTO_SMOKE=1` and `ZSCLIP_NATIVE_HOST_SHELL_OPEN_DRY_RUN=1`. The real native process writes and reads text and file-path clipboard probes, checks that the shared clipboard sequence changes after a write, polls one clipboard monitor change into the shared `clipboard_changed` event path, records a safe shell-open host dry-run for a temporary file path, injects a safe file-picker smoke path through `ZSCLIP_NATIVE_HOST_FILE_PICKER_SMOKE_PATH`, runs the `xdotool`/`xprop` window/paste-target identity smoke boundary, opens the settings surface, dispatches representative settings controls, row actions, VV select, native VV paste, Linux native paste-shortcut attempt, StatusNotifier install/unavailable reporting and status-menu routes, then the script verifies those product-route/event-route lines in `zsclip-gtk.log`. Set `ZSCLIP_NATIVE_HOST_AUTO_SMOKE=0` only when debugging a manual launch.

The script can use any of these screenshot tools:

- `gnome-screenshot`
- `grim`
- ImageMagick `import`
- `scrot`

Optional click smoke:

```bash
NATIVE_HOST_SMOKE_CLICK=1 bash scripts/native-host-smoke-linux.sh
```

The optional click smoke requires `xdotool`, so it is mainly for X11 sessions. It clicks representative settings and row controls, attempts the VV Popup Select 1 path when the popup window is found, and checks `zsclip-gtk.log` for the expected product-route/event-route lines. A successful optional click smoke also writes:

- `target/native-host-smoke/linux/zsclip-gtk-after-clicks.png`

The current GTK host should show the main `ZSClip` window, a visible Status menu button backed by GTK/GIO actions, a native SearchEntry after Search is clicked, the Settings window after Settings is clicked, shared settings page summaries for General, Hotkey, Plugin, Group, Cloud and About with section/control/control-row counts plus a GTK control-blueprint preview rendered as native `CheckButton`, `Entry`, `Button` and `Label` widgets. Executable blueprint controls display their shared `window.settings.*` command route or settings action route, and text/list controls display their shared setting-field/runtime-list binding. Pressing Save collects all currently bound native text/toggle/dropdown values, applies/persists them through the shared settings model and updates the status label/log with the apply/collect plan summary. The remaining settings work is target screenshot evidence and full-section native control coverage. The settings window also exposes controls such as Capture, LAN Sync and Sync Mode, a Group Management panel with Records/Phrases, Add, Rename, Delete, Up and Down actions backed by SQLite, platform actions such as Open Source, Check Updates and WPS Docs, native dialog buttons such as Info Dialog and Confirm, row action buttons such as Copy, Edit and Translate, a native `ZSClip Edit` window for the Edit action, a Row Menu button backed by shared `NativePopupMenuEntry` row actions including the Add To Group submenu populated from `clip_groups`, a Group Filter button backed by real group-filter popup entries, and a VV Popup button that opens a native window from the shared VV render plan. The row group submenu should update `items.group_id` for the selected history row and refresh the DB-backed list; the Group Filter menu should refresh the visible DB-backed list for the selected group. The VV popup row buttons should dispatch the shared `vv_select_requested` event bridge and then write the selected item into the native clipboard/paste-target bridge. The status menu exposes Show ZSClip, Toggle Capture, Toggle LAN Sync and Exit through the same product routes as the Windows tray menu.

The Linux shell-open host records shell-open requests for tests and uses GIO `AppInfo::launch_default_for_uri` in non-test target builds. The default smoke script sets `ZSCLIP_NATIVE_HOST_SHELL_OPEN_DRY_RUN=1`, so it proves the real GTK host reaches the shell-open boundary and records the target path without launching an external app. Set `ZSCLIP_NATIVE_HOST_SHELL_OPEN_DRY_RUN=0` only for a manual target handoff check.

The Linux file-picker host records file-picker requests for tests and uses GTK `FileChooserNative` in non-test target builds. The default auto smoke temporarily sets `ZSCLIP_NATIVE_HOST_FILE_PICKER_SMOKE_PATH` to a temporary file path, so it proves the real GTK host reaches the file-picker boundary without opening an interactive chooser. Leave that environment variable unset for a manual target picker check.

The `linux-gtk` GitHub Actions job installs `libgtk-4-dev`, runs the same smoke script under Xvfb and uploads `native-host-smoke-linux`.

## Known Limits

These smoke scripts prove target launch, screenshot capture, native-host route dispatch from the real AppKit/GTK process, text clipboard read/write probes, file-path clipboard read/write probes, clipboard sequence movement after writes, one clipboard monitor event bridge into `clipboard_changed`, shell-open host boundary dry-run recording, file-picker host boundary injected-path recording, window/paste-target identity boundary execution, native Edit window presentation/save routing, bound native settings-value collection through the shared apply/persist model, native VV paste bridge execution, native paste-shortcut attempt logging and optional native button/menu interaction. They do not yet prove every system integration. The remaining native work includes full-section native settings screenshots/control coverage, StatusNotifierHost screenshot/artifact proof on a real Ubuntu desktop, real VV trigger integration beyond the manual native preview button, target proof that OS key-event paste succeeds under macOS Accessibility or Linux ydotool/xdotool permissions, successful target probe values for the System Events and `xdotool`/`xprop` window identity boundaries under real permissions, interactive target-OS file picker smoke evidence, non-dry-run shell-open handoff smoke evidence, long-running clipboard monitoring/source identity beyond the single smoke probe, Wayland/AT-SPI identity coverage behind the shared window/paste-target snapshot and target-OS smoke evidence for selected-row edit/save refresh behavior.

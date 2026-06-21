# ZSUI Host Porting Contract

ZSUI is the shared Rust UI logic and native-host contract being extracted from ZSClip.
This document describes the minimum UI host surfaces and primitives a non-Windows frontend must provide.
The same contract is intended for macOS AppKit/SwiftUI and future Linux GTK4/libadwaita hosts.
`src/windows_win32_adapter.rs` names the current Windows Win32/GDI adapter boundary, including native message loop, GDI renderer/text layout, Win32 controls, clipboard, status item, menus, dialogs, IME, window identity, paste target and window hosts.
`src/macos_appkit_adapter.rs` maps the current ZSUI/macOS host contract to named AppKit/SwiftUI adapter bindings and widget roles, while `src/macos_native_host.rs` is the target-only AppKit launcher that uses `objc2`, `objc2-app-kit` and `objc2-foundation` to create `NSApplication` / `NSWindow` and enter the AppKit event loop.
Linux now has an initial code-level scaffold in `src/linux_app.rs`; it consumes the shared lifecycle, command queue, main host execution plan count, clipboard operations, semantic style resolution, native control mapping, text layout, status/menu entries, popup/menu requests, transient-window requests, IME anchor queries, dialog requests, shell-open requests, window-identity queries, paste-target requests, text-caret anchor queries, file/text/edit dialog requests, mail-merge window requests, renderer commands, main-window/search host requests, settings-window host request, settings control specs, settings dropdown requests, shared non-host protocol summaries and host-surface summaries.
`src/linux_gtk_adapter.rs` is the next boundary layer: it maps the current ZSUI/Linux host contract to named GTK/libadwaita adapter bindings and widget roles, including the shared main execution plan bridge, while `src/linux_native_host.rs` is the target-only GTK launcher that uses `gtk4::Application` / `gtk4::ApplicationWindow` and enters the GTK event loop.
The product program owns features and data; ZSUI converts product state into platform-neutral UI models and host requests, while each platform backend translates those requests into native UI and desktop APIs.
The framework identity and API version live in `src/app_core/zsui.rs`; required
UI host surfaces and adapter names live in `src/app_core/ui_surface_protocol.rs`; stable
command ids, payloads and command queues live in `src/app_core/command_protocol.rs`;
shared component lifecycle/update/layout/render contracts live in
`src/app_core/component_protocol.rs`; lifecycle and input event protocols live in `src/app_core/event_protocol.rs`; the framework-level reuse manifest lives in
`src/app_core/framework_manifest.rs`; native
window/control host contracts live in `src/app_core/native_hosts.rs`; clipboard,
menu, dialog, IME, caret, shell, paste-target and platform service host
contracts live in `src/app_core/host_protocol.rs`; semantic style/text/renderer
protocols live in `src/app_core/render_protocol.rs`; native control specs, mappers and control-host contracts live in
`src/app_core/control_protocol.rs`; geometry, DPI compensation and shared layout
protocols live in `src/app_core/layout_protocol.rs`; main-window command ids,
shortcut routing, menu intents and tray plans live in `src/app_core/main_commands.rs`;
main-window visibility, positioning, hotkey and search behavior plans live in
`src/app_core/main_window_protocol.rs`;
native adapter manifest metadata for Windows, macOS and Linux backend discovery lives in
`src/app_core/native_adapter_manifest.rs`;
native host launch plans that distinguish real AppKit/GTK event-loop entry from contract scaffold fallback live in
`src/app_core/native_host_launch.rs`;
shared native host window actions for Search, Settings, Hide and Close live in
`src/app_core/native_host_actions.rs`;
the same module also defines native settings-surface actions for Save, Close and Open Config;
settings dimensions, DPI helpers, control roles, command/action routing and prompt models live in
`src/app_core/settings_protocol.rs`;
main/settings timer task mapping lives in `src/app_core/timer_protocol.rs`;
ZSClip-specific application events, async payloads and the AI capability catalog for LLM, skill and product-adapter integrations live in `src/app_core/product_adapter.rs`; the AI action presenter contract lives in `src/app_core/ai_action_protocol.rs`; remaining contract surfaces are the `REQUIRED_*` and `SHARED_*` constants re-exported from `src/app_core.rs`.
See `docs/zsui.md` for the reusable framework boundary.
When handing this UI boundary to another AI agent, start with the repository
skill at `docs/skills/zsclip-native-ui/SKILL.md`. It points to the compact file
map, layer rules and completion vocabulary an agent should use before editing
AppKit/GTK hosts or reporting native feature progress.

Ports and reusable product adapters should begin with `zsui_framework_manifest()`: it summarizes the framework identity, API version, framework layers, boundary rules, supported native platforms/toolkits, native backend descriptors, required host surfaces, shared non-host protocols, product adapter contract and AI capability manifest before a platform-specific adapter is selected.
Read `zsui_framework_layers()` and `zsui_framework_boundary_rules()` before adding macOS/Linux feature work. The framework layers are `core_contracts`, `layout_and_render_plans`, `adapter_boundary`, `native_host` and `product_adapter`. Core contracts own stable ids, events and host traits; layout/render plans own pure geometry, render commands and row/VV action plans; adapter boundaries own backend descriptors and binding names; native hosts own AppKit/GTK windows, controls, menus, dialogs, clipboard services, rendering and event loops; product adapters own ZSClip state projection, row command execution, settings persistence, sync, async events and AI catalog/executors.
When wiring features such as right-click edit, group menus, VV paste, settings controls or AI actions on AppKit/GTK, put only native presentation and platform service calls in `src/macos_app.rs`, `src/macos_native_host.rs`, `src/linux_app.rs` or `src/linux_native_host.rs`. Product behavior should stay behind `ProductAdapterHost` / `ZsclipProductAdapter`, and reusable layout/action decisions should stay in `app_core` plans.
Use `native_ui_backend_for_platform()`, `native_ui_backend_for_toolkit()` or `native_ui_backend_for_current_target()` to resolve the matching adapter boundary from that backend catalog.
Check the backend descriptor status before assuming native runtime readiness: Windows is `native_host_integrated`, while macOS and Linux are currently `native_host_first_pass`. First-pass means the real AppKit/GTK host entry points and adapter bindings exist, but `is_native_runtime_ready()` remains false until target smoke artifacts prove the windows, menus, dialogs, clipboard paths and settings interactions on real macOS/Ubuntu.
For target-level launch status, call `macos_native_host_launch_plan()` or `linux_native_host_launch_plan()`: on macOS/Linux target builds the plan enters `real_native_host` through `src/macos_native_host.rs` or `src/linux_native_host.rs`; on other platforms the same plan reports `contract_scaffold_fallback` so tests and agents do not confuse callable scaffolds with verified native runtime.
The real host launchers also expose action bridges: AppKit uses `NSButton` target/action selectors and GTK uses `Button::connect_clicked`; both route Search, Settings, Hide and Close through `NativeHostUiAction` into the same product command ids used by the Windows host. Search now toggles a native search control (`NSSearchField` on AppKit, `SearchEntry` on GTK), search text changes dispatch `NativeHostSearchTextAction` into `zsclip.window.search_text_update`, and the visible native preview rows are built from `ProductAdapterProjectedState::native_clip_items` before being filtered by the shared `native_host_filtered_projected_clip_item_ids()` rule. Settings opens a second native host surface: AppKit creates or focuses a `ZSClip Settings` `NSWindow`, and GTK creates a `ZSClip Settings` `ApplicationWindow`.
The settings surface exposes native Save, Open Config and Close controls through `NativeHostSettingsAction`, routing to `zsclip.settings.save`, `zsclip.settings.open_config` and `zsclip.settings.close`.
Native host status/menu entries are also shared now: AppKit creates a `ZSClip` `NSStatusItem` backed by `NSMenuItem` target/action selectors, and GTK creates a visible Status `MenuButton` plus `gio::SimpleAction` entries from the same `NativeHostStatusMenuAction` list. Those entries map to the existing Windows tray menu routes for Show ZSClip, Toggle Capture, Toggle LAN Sync and Exit.
Use `native_ui_backend_capability_matrix()` or `zsui_reuse_readiness_report()` when tooling, AI agents or another Rust product needs a compact list of platform names, ready/first-pass/scaffold split, native adapter capability names, product adapter surfaces and AI provider families without inspecting platform implementation files.
Use `zsui_reuse_bootstrap_plan(platform)` when a new product is ready to wire one concrete target: it returns the selected backend module and adapter boundary plus native adapter capabilities, product adapter surfaces/tasks, AI provider/executor boundary names and native runtime gate progress in one structure. Each runtime gate includes the adapter capability names it depends on; the AI presentation gate also names the product adapter tasks and executor boundaries needed for LLMs, skills and product tools.
The concrete Windows, macOS and Linux adapter boundaries also expose `reuse_bootstrap_plan()` so host-specific code can read the same plan from the selected adapter without going back through the framework catalog.
They also expose `adapter_binding_plan()` for host-specific binding names, such as Win32/GDI runtime driver, message loop and renderer bindings, AppKit/SwiftUI runtime driver and window/control bindings or GTK/libadwaita runtime driver and widget bindings.
Use `reuse_package()` when adapter code needs manifest, bootstrap plan and binding plan together.
Use `native_ui_adapter_parity_report()` on the selected reuse packages when checking that Windows, macOS and Linux expose the same binding count, main execution plan count and shared protocol count.
Use `zsui_adapter_reuse_package_gate_binding_summaries()` when checking that every runtime gate binding plan can be satisfied by the selected adapter binding names.
Use `zsui_adapter_reuse_package_porting_work_items()` to list only incomplete runtime gates as actionable porting work items with module path, gate name, platform binding names, product adapter tasks and AI executor boundaries.
Use `zsui_reuse_readiness_report_with_adapter_parity()` when tooling wants the high-level readiness summary and the selected adapter parity check in one result.
Use `zsui_agent_context()` or `zsui_agent_context_with_adapter_parity()` when an AI agent, skill registry or product adapter needs stable string summaries of platform bootstrap plans, readiness and AI execution routes. The parity/package variant also carries incomplete platform porting work items for macOS and Linux.
The agent context also carries the same framework layers and boundary rules, so a platform task can decide whether it is editing shared Rust UI plans, adapter metadata, native host code or product behavior before touching AppKit/GTK files.
Use `zsui_native_feature_parity_statuses()` or the `native_feature_parity` field on `zsui_framework_manifest()` / `zsui_agent_context()` when reporting completion. Each row names a macOS/Linux feature and separates `code_level_ready`, `target_smoke_verified` and `system_complete`. This is the source of truth for questions such as whether right-click edit, group management, VV paste or settings pages are merely callable through shared contracts or actually complete on the real target OS.
Use `zsui_reusable_app_blueprint()` for non-ZSClip tools that adopt ZSUI. The blueprint describes framework-level Rust UI features such as native app entry, window surfaces, control mapping, renderer/text layout, system services, settings surfaces and AI action surfaces, then maps each feature to Win32/GDI, AppKit/SwiftUI or GTK/libadwaita bindings. It also carries product function flows for app bootstrap, state projection, user commands, settings sync, async events and AI actions, plus an execution pipeline from UI intent through product command execution, async events, AI action routing and UI update projection.
The agent context embeds the same reusable app blueprint so AI agents and skills can understand how a new product should keep its logic and behavior in one Rust product adapter while native hosts translate the UI.
Implement `NativeRuntimeDriver` on each platform runtime. It is the platform entry trait for starting the native runtime, dispatching platform-neutral UI commands, polling application events and requesting shutdown; lower-level host traits still own the actual native windows, controls, menus and renderers.
Windows exposes the same driver shape through the Win32 adapter, and the macOS/Linux scaffolds implement this driver path in their application models, so AppKit/SwiftUI and GTK/libadwaita ports can now exercise startup, command dispatch, event polling and shutdown without copying product behavior into the native adapter.
The agent context includes platform runtime gates for event loop, window surfaces, control mapping, renderer, clipboard services, dialog services, settings surfaces and AI action presentation, so tools can tell whether a target is already runtime-ready or still a scaffold. The gate capability plans let tools explain why a gate is blocked, such as missing renderer/text-layout bindings or missing AI executor presentation paths. Runtime gate binding plans translate those capabilities into concrete platform bindings such as AppKit `ns_window_pair` or GTK `gtk_snapshot_renderer`. Runtime gate completion reports expose completed gate names, missing gate names, total/completed/missing gate counts, completion percent and the next gate name.
macOS and Linux currently complete the shared `native_event_loop`, `native_window_surfaces`, `native_control_mapping`, `native_renderer`, `native_clipboard_services`, `native_dialog_services`, `native_settings_surfaces` and `ai_action_presentation` contracts through `NativeRuntimeDriver`, main/settings window, dropdown, input/edit dialog, transient-window, search-control, settings-control, control-mapper, IME, text-layout, renderer, clipboard, clipboard monitor polling, paste-target, window-identity, popup-menu, shell-open, file-dialog, prompt/edit-dialog, settings surface and AI presenter tests. They now also have target-only real host launchers and are reported as `native_host_first_pass`, but they do not become runtime-ready until target OS runners prove the AppKit and GTK windows, menus, dialogs, clipboard and settings interactions with screenshots or interaction tests.
The first real-host action slice is callable now: AppKit and GTK main windows both expose native controls for Search, Settings, Hide and Close, and platform tests verify that those controls resolve to `zsclip.window.toggle_search`, `zsclip.window.open_settings`, `zsclip.window.hide` and `zsclip.window.close` product routes. The Search route toggles a native search input, forwards search text into the product adapter and filters the native list projected by the product adapter, and the Settings route opens a native settings surface with the shared settings page names, so other platform UI testers can inspect both main and settings windows before deeper settings controls are wired.
The main list also has a first native row-action slice: AppKit and GTK expose Paste, Copy, Pin, To Phrase, Delete, Edit, Open Path and Translate controls, but those controls do not reimplement row behavior. They call the existing `window.menu.invoke` command with the shared row menu ids, which the product adapter resolves to `zsclip.row.paste`, `zsclip.row.copy`, `zsclip.row.toggle_pin`, `zsclip.row.to_phrase`, `zsclip.row.delete`, `zsclip.row.edit`, `zsclip.row.open_path` and `zsclip.row.text_translate`.
The settings surface has its first callable controls too: Save, Open Config, Close, Capture, LAN Sync, Cloud Sync and Sync Mode are native AppKit/GTK buttons that dispatch through the product adapter instead of remaining static labels, and the platform action row now exposes Open Source, Check Updates and WPS Docs on both backends. The native dialog slice is also callable from the settings surface: AppKit presents `NSAlert` for Info Dialog and Confirm, while GTK presents `MessageDialog` for the same shared `NativeHostDialogAction` intents. The first native status/menu slice is callable as well: AppKit uses an `NSStatusItem` menu and GTK uses a `StatusNotifierItem` plus GIO action menu/MenuButton fallback for Show ZSClip, Toggle Capture, Toggle LAN Sync and Exit. The Row Menu and Group Filter slices now use shared popup-entry builders: AppKit maps them to `NSMenu`, GTK maps them to `PopoverMenu`, and both hosts dispatch selected menu ids through the product adapter. AppKit refreshes the DB-backed list after record-group settings changes and resets a deleted active filter back to All. GTK also refreshes those popup models after settings group create, rename, delete or reorder actions, and resets a deleted active filter back to All before refreshing the DB-backed list. Clipboard monitor polling is now shared too: AppKit and GTK application models baseline the native sequence, ignore self-writes and bridge external sequence changes as the product `clipboard_changed` event. The first VV Popup native preview slice is callable from the main surface too: both hosts render a native floating window from the shared VV popup render plan, their row buttons bridge the shared `vv_select_requested` event, and the selected DB-backed item now enters a shared native VV paste bridge that writes the selected payload through `ClipboardHost` and asks the native host to post Backspace plus paste shortcut events (`CGEvent` on AppKit, `ydotool`/`xdotool` on GTK). Real platform trigger integration and target proof that those OS key events succeed under permissions remain separate work.
Window/paste-target identity is also a platform-service host responsibility now. The shared snapshot and readiness decisions remain in `app_core`, while AppKit target builds can use System Events through `osascript` to query the frontmost process, process name and bundle id and to request paste-target foreground/Command+V. GTK target builds can use `xdotool`, `xprop` and `/proc` to query foreground window, pid, process name and WM_CLASS and to request window activation/Ctrl+V. These probes improve the real host boundary but still require target smoke artifacts under macOS Accessibility and Linux desktop permissions before the feature is system-complete.
The agent context also exposes ordered integration steps: select native adapter, verify adapter capability parity, implement product adapter surfaces/tasks, then connect the LLM executor, skill registry and product-specific AI tools.

The product adapter contract requires product identity, product state model, product command executor, settings model, async event bridge and AI capability catalog surfaces. A reusable app should implement those surfaces with its own domain data while leaving native window/control work to the selected platform adapter.
Implement `ProductAdapterHost` for the product: it is the concrete Rust trait for identity, state projection, command execution, settings binding, async event bridging, AI catalog publication and AI plan execution. Platform hosts should consume the trait through ZSUI plans, not duplicate product logic inside Win32, AppKit or GTK code.
Use `ZsuiReusableRuntimeHarness` as the common wiring path between a `NativeRuntimeDriver` and `ProductAdapterHost`: it starts the native runtime, dispatches UI commands into both sides, bridges polled application events, routes AI invocations and requests shutdown.
Use the top-level `ZsclipProductAdapter` in `src/zsclip_product_adapter.rs` as the first product-side example: it maps ZSClip clipboard row commands, tray commands, async events and AI plans onto `ProductAdapterHost` without putting that product behavior into Win32/AppKit/GTK hosts.
Use `zsclip_product_adapter_manifest()` when a porting tool, LLM, skill registry or platform host needs a compact inventory of ZSClip product command routes, event routes and AI capability/provider ids before wiring native controls.
Use `product_adapter_reuse_checklist()` to get the implementation tasks a new product must complete: provide identity, project product state, execute product commands, bind settings, bridge async events, publish the AI catalog, connect an LLM executor, connect a skill registry and connect product-specific AI tools.

For multi-platform preview and testing, AI agents should use a layered workflow:
first read `zsui_agent_context()`, `zsui_reusable_app_blueprint()`,
`zsui_reuse_bootstrap_plan(platform)` and `zsclip_product_adapter_manifest()`;
then generate platform preview artifacts from shared layout/render plans and
product routes; then run scaffold tests for macOS/Linux host contracts; inspect
`macos_native_host_launch_plan()` / `linux_native_host_launch_plan()` to see
whether the current binary would enter the real native event loop; finally, only
on the real target OS, run AppKit/SwiftUI or GTK/libadwaita screenshot and
interaction tests. Preview artifacts and launch plans are useful for inspecting
structure and function entry points, but they are not proof of real native rendering.
The concrete smoke entry points are documented in `docs/native-host-verification.md`
and live in `scripts/native-host-smoke-macos.sh` and
`scripts/native-host-smoke-linux.sh`.

AI-aware native hosts should read product capability descriptors instead of hard-coding menu labels or model/provider details. A descriptor identifies the provider family, action kind, UI surface, required context such as selected text/image/file/item ids or settings profile, and result kind. Real LLM clients, skill registries, prompts, permissions and product data adapters remain product code.
Use `product_ai_integration_manifest()` when a host or agent needs a compact summary of the whole AI surface before rendering menus or deciding which executor family to prepare. The manifest includes provider families, action kinds and execution routes so an executor can prepare LLM, skill-registry and product-adapter handlers without branching on UI strings.
For example, row-menu AI entries such as image OCR and text translation resolve to `ProductAiActionKind` first, then the native host or product executor can look up the matching descriptor for the row context menu surface.
The settings plugin page should use `settings_plugin_ai_panel()` for AI provider configuration presentation; native hosts still own controls, while the product adapter owns any real provider setup and persistence.
Main-window ports can use `main_row_ai_capability_plan()` to expose the AI actions available for the current row or selected rows. The plan is platform-neutral and should be passed to product/AI executors instead of asking native menu code to infer item semantics.
When a user picks an AI action, use `main_row_ai_invocation()` to validate that the capability is available for the current plan and to build the product invocation payload. Native hosts should not construct AI invocation ids or item-id lists by hand.
After that, route the invocation through `product_ai_execution_plan()` and hand the returned plan to the LLM, skill or product-adapter executor. The plan includes provider, executor boundary, executor integration task, action, source surface, required contexts and result kind. Platform UI code should not branch directly on capability id strings.
Execution plans and routes provide stable name helpers for those fields, so tooling can read semantic strings such as `llm_executor`, `skill_registry` or `product_adapter_tools` without relying on Rust enum debug output.

## Contract Version

- `APP_CORE_API_VERSION`: `0.78`
- Required host surfaces: `5`
- Required main host execution plans: `5`
- Required native style host operations: `1`
- Required native control mapper operations: `1`
- Required text layout host operations: `2`
- Required renderer host operations: `5`
- Required settings control host operations: `11`
- Required clipboard host operations: `8`
- Required status item host operations: `3`
- Required native popup menu host operations: `1`
- Required native transient window host operations: `4`
- Required native IME host operations: `3`
- Required native text caret host operations: `5`
- Required native dialog host operations: `2`
- Required native shell open host operations: `1`
- Required native window identity host operations: `7`
- Required native paste target host operations: `6`
- Required native file dialog host operations: `1`
- Required native text input dialog host operations: `1`
- Required native edit text dialog host operations: `1`
- Required native mail merge window host operations: `1`
- Required native main search control host operations: `8`
- Required native main window host operations: `21`
- Required native settings window host operations: `13`
- Required native settings dropdown host operations: `3`
- Shared non-host UI protocols: `3`

## Required Surfaces

The authoritative list is `REQUIRED_UI_HOST_SURFACES` from `app_core::ui_surface_protocol`.

| Surface | Adapter entry | Purpose |
| --- | --- | --- |
| `MainWindow` | `main_window_host_event_from_message` | Main clipboard window, tray/menu callbacks, async results, list interactions, search and VV popup entry. |
| `SettingsWindow` | `settings_window_host_event_from_message` | Settings window chrome, settings controls, page switching, save/close, plugin and sync configuration. |
| `SettingsDropdown` | `dropdown_window_host_event_from_message` | Settings dropdown popup pointer, wheel and selection behavior. |
| `InputDialog` | `input_dialog_host_event_from_message` | Small text input dialog used by group creation and rename flows. |
| `EditDialog` | `edit_dialog_host_event_from_message` | Multi-line item edit dialog, save/cancel, resize, close confirmation and text change routing. |

## Main Host Execution Plans

| Plan | Purpose |
| --- | --- |
| `Search` | Apply a platform-native search box visibility change from `MainSearchVisibilityRequest`. |
| `OpenSettings` | Open or focus the settings host surface. |
| `HideWindow` | Hide the main clipboard window without exiting the app. |
| `CloseWindow` | Request normal main window close/shutdown handling. |
| `InvokeMenuCommand` | Execute a platform-neutral main menu command intent. |

## Main Window Host

Startup creation of the main clipboard window and quick window is native host work. Shared code
describes the title, initial size and initial main-window visibility through
`NativeMainWindowRequest`; the platform host returns both handles or `Failed`.
Shared window behavior such as show/hide order, edge-hide restore position, hotkey
presentation and search-box visibility is planned by `app_core::main_window_protocol`;
the host should translate those plans into native window operations.

| Operation | Purpose |
| --- | --- |
| `create_main_windows` | Create the platform main and quick windows for the app session. |
| `apply_main_window_appearance` | Apply platform-native main window appearance such as rounded corners, dark frame or native material. |
| `set_main_window_app_icon` | Apply the native application icon resource to the main or quick window. |
| `hide_main_window` | Hide the native main window without destroying application state. |
| `present_main_window` | Present the native main or quick window using a platform-native activation strategy. |
| `set_main_window_bounds` | Position and size the native main or quick window using platform-native window bounds APIs. |
| `activate_main_window` | Show and activate the native main window using platform foreground/key-window behavior. |
| `foreground_main_window` | Bring an already visible native main window to the foreground without applying the stronger activation path. |
| `restore_main_window` | Restore a minimized or hidden native main window to its normal visible state. |
| `close_main_window` | Post a normal close request to a native main window without forcing immediate destruction. |
| `set_main_window_activation_policy` | Toggle whether the native main window may activate and take focus during pointer interaction. |
| `request_main_window_close` | Ask the native main window to run normal close handling. |
| `destroy_main_window` | Destroy the native main window when close handling chooses to exit. |
| `capture_main_pointer` | Capture pointer input for main-window drag workflows. |
| `release_main_pointer` | Release pointer input after main-window drag workflows end or native drag/drop begins. |
| `begin_main_window_drag` | Start a platform-native main-window move/drag operation. |
| `track_main_pointer_leave` | Ask the native window system to report when the pointer leaves the main surface. |
| `request_main_window_area_repaint` | Invalidate all or part of the main surface using the platform drawing lifecycle. |
| `main_window_layout_dpi` | Query the effective layout DPI or display scale for the main surface. |
| `main_window_client_bounds` | Query the drawable content bounds of the main surface. |
| `main_window_bounds` | Query the platform window frame bounds used for monitor fitting and DPI transitions. |

## Main Search Control Host

The main window search input is a native control. Shared code describes the owner, control id,
bounds and visibility through `NativeMainSearchControlRequest`; the platform host returns the
native search/edit control handle or `Failed`. After creation, shared code drives ordinary search
control behavior through the same host instead of directly naming Win32 `EDIT` or AppKit controls.
On Windows, `src/app/main_search.rs` is the adapter that consumes the shared search visibility
plan and delegates bounds, visibility, text, focus and style-resource work to
`WindowsMainSearchControlHost`.

| Operation | Purpose |
| --- | --- |
| `create_search_control` | Create the platform-native main search input control. |
| `apply_search_style` | Apply platform-native search font/style from shared font family and size intent, returning an opaque style resource when needed. |
| `release_search_style_resource` | Release an opaque search style resource previously returned by the host. |
| `set_search_bounds` | Move and resize the search input from model-provided bounds. |
| `set_search_visible` | Show or hide the search input. |
| `search_text` | Read current search text from the native control. |
| `set_search_text` | Update current search text in the native control. |
| `focus_search` | Move keyboard focus into the native search control. |

## Window Identity Host

Window and application identity is native platform work. Shared clipboard-source and VV popup logic
consume process names, class names and root/foreground handles without knowing whether the host is
Win32, AppKit or Accessibility-backed.

| Operation | Purpose |
| --- | --- |
| `process_name` | Return a display/process identity for a native window or app handle. |
| `class_name` | Return the platform-native window/control class or role name for a handle. |
| `root_handle` | Resolve a nested native handle to its root window or owning app surface. |
| `foreground_handle` | Return the current foreground/key native window or app handle. |
| `exists` | Report whether a native window or app handle is still valid. |
| `is_foreground` | Report whether a native window or app handle is currently foreground/key. |
| `is_current_process_window` | Report whether a native handle belongs to the clipboard app itself. |

The Windows implementation is `WindowsWindowIdentityHost` in `src/platform/window_identity.rs`;
clipboard-source detection, VV ignore rules, VV backspace decisions, VV popup target lifecycle and
paste-target availability/direct-edit validity/settings skip-class capture checks consume this host
instead of directly reading window process ids, class names, foreground windows, existence or
foreground state in `app.rs`.

## Paste Target Host

Paste target foregrounding and focus restoration are native platform work. Shared code decides when
to paste; the platform host owns how to bring the target window or app forward and how to restore a
previously captured focus target.

| Operation | Purpose |
| --- | --- |
| `force_paste_target_foreground` | Ask the platform to bring the paste target to the foreground and report whether it succeeded. |
| `restore_paste_target_focus` | Restore a previously captured paste focus inside the target when the platform can safely do so. |
| `set_paste_target_text` | Write text directly into a platform-native paste target and place the caret after the inserted text. |
| `paste_target_text_input_capabilities` | Query whether a platform-native paste target advertises text input behavior without leaking platform message flags into shared code. |
| `paste_target_focus_status` | Query whether the current native focus is still inside the paste target, outside it, unavailable, or absent. |
| `paste_target_text_input_ready` | Query whether a target is ready for direct VV text input, including platform-specific focus, IME, caret and app-compatibility heuristics. |
| `send_paste_shortcut` | Ask the platform to send the native paste shortcut after the clipboard payload is ready. |

The Windows implementation is `WindowsPasteTargetHost` in `src/platform/paste_target.rs`; the main
clipboard paste path now lives in `src/app/main_paste.rs`, where shared paste preparation and
completion plans are executed through `WindowsClipboardHost`, `WindowsPasteTargetHost` and
`WindowsWindowIdentityHost`. Direct edit passthrough and the Windows mail merge helper consume the
same host instead of calling foreground, focus, text-input detection, GUI-thread focus or
edit-control Win32 APIs directly.

## IME Host

IME candidate and composition geometry is native platform work. Shared VV popup placement consumes
screen-space anchors and keeps the popup edge-selection rules platform-neutral, while the platform
host owns IMM, AppKit input context or accessibility queries.

| Operation | Purpose |
| --- | --- |
| `candidate_anchor` | Query a candidate-window anchor for a focused native text target, returning either a screen point or screen rectangle. |
| `composition_anchor` | Query a composition-window anchor for a focused native text target, returning either a screen point or screen rectangle. |
| `has_default_ime_window` | Check whether a focused native text target has a platform IME/default input context window available. |

The Windows implementation is `WindowsImeHost` in `src/platform/ime.rs`; VV popup placement consumes
this host instead of sending `WM_IME_CONTROL`, reading IMM form structures, or probing the default
IME window directly in `app.rs`.

## Text Caret Host

Text input anchor geometry is native platform work. Shared VV popup placement consumes screen-space
anchors and keeps popup edge-selection rules platform-neutral, while the platform host owns
accessibility APIs, GUI-thread caret queries, focus resolution, cursor lookup and coordinate
conversion.

| Operation | Purpose |
| --- | --- |
| `accessible_caret_anchor` | Query the platform accessibility caret for a focused native text target as a screen-space anchor. |
| `thread_caret_anchor` | Query the platform GUI/input thread caret for a native text target as a screen-space anchor. |
| `focus_rect_anchor` | Query a compact focused native text target rectangle as a screen-space anchor. |
| `cursor_anchor` | Query the current pointer position as a fallback screen-space anchor. |
| `focus_handle_for_target` | Resolve the focused native text handle inside a target window or app. |

The Windows implementation is `WindowsTextCaretHost` in `src/platform/text_caret.rs`; VV popup
placement consumes this host instead of reading `GUITHREADINFO`, calling UIA caret helpers,
probing focused child windows, querying cursor position or doing caret coordinate conversion
directly in `app.rs`.

## Native Style And Control Mapping

Platform-native visual style is intentionally resolved outside shared product logic.

| Operation | Purpose |
| --- | --- |
| `resolve_text_style` | Convert `SemanticTextStyle` into platform text font, size, color and alignment. |
| `class_name` | Map `SettingsComponentKind` to the platform's native control family/class. |

## Rendering Host Primitives

Shared components can render through `TextLayout` and `Renderer`. A platform host that uses the
component abstraction must provide these primitives with native drawing APIs:

| Operation | Purpose |
| --- | --- |
| `measure` | Measure text using the platform text engine and a resolved `TextStyle`. |
| `layout_runs` | Break text into drawable runs inside model-provided bounds. |
| `fill_rect` | Fill a rectangle with a platform-native brush/color. |
| `stroke_rect` | Stroke a rectangle outline. |
| `draw_text` | Draw a `TextRun` with a resolved `TextStyle`. |
| `push_clip` | Push a clipping rectangle before nested drawing. |
| `pop_clip` | Restore the previous clipping state. |

## Settings Control Host

Native settings pages are built from `SettingsControlSpec` and a platform host implementing
`NativeSettingsControlHost`. A new platform host must provide these operations:

| Operation | Purpose |
| --- | --- |
| `create_control` | Create a native control from a platform-neutral `SettingsControlSpec`. |
| `destroy_control` | Release a native settings control. |
| `control_exists` | Check whether a native settings control is still alive after a page or metrics rebuild. |
| `set_control_visible` | Show or hide an existing control. |
| `set_control_enabled` | Enable or disable an existing control. |
| `set_control_bounds` | Move and resize a control using model-provided bounds. |
| `control_at_point` | Hit-test visible and enabled native controls in settings-window client coordinates. |
| `control_screen_bounds` | Query a native control's screen bounds for popup anchoring. |
| `control_text` | Read current text from a native text-capable control. |
| `set_control_text` | Write text into a native text-capable control. |
| `request_control_repaint` | Request a repaint of a native settings control after its display state changes. |

## Settings Window Host

Opening or focusing the settings surface is platform host work. Shared logic sends a
`NativeSettingsWindowRequest` with owner, optional existing handle and bounds; the native host
returns `Created`, `FocusedExisting` or `Failed`.
Settings window geometry, DPI scaling, control roles and group prompt models should come from
`app_core::settings_protocol`. Some `SettingsAction` variants still describe ZSClip product
features such as WebDAV, LAN pairing and WPS integration; non-ZSClip apps should replace those
through their own product adapter instead of copying that business logic.
`settings_action_route` assigns each action to the shared sync, group or platform execution
domain. Each platform/product adapter implements the domain executor; the Windows ZSClip
executor lives in `src/app/settings_actions.rs`, while the domain implementations live in
`src/app/settings_sync_actions.rs`, `src/app/settings_group_actions.rs` and
`src/app/settings_platform_actions.rs`.

| Operation | Purpose |
| --- | --- |
| `present_settings_window` | Create the platform settings window or focus an existing one. |
| `set_settings_window_bounds` | Position and size the platform settings window during DPI and work-area adjustments. |
| `destroy_settings_window` | Close and destroy the native settings window for host-owned lifecycle requests. |
| `focus_settings_window` | Focus the settings window for host-owned keyboard capture workflows. |
| `track_settings_pointer_leave` | Request native pointer-leave/hover tracking for settings window hover cleanup. |
| `capture_settings_pointer` | Capture pointer input for settings window drag workflows. |
| `release_settings_pointer` | Release pointer input after settings window drag workflows end. |
| `request_settings_window_repaint` | Request a native repaint of the settings window after background state changes. |
| `request_settings_window_area_repaint` | Request a native repaint for either the whole settings window or a specific client-area rectangle with host-owned erase semantics. |
| `settings_window_layout_dpi` | Query the platform layout DPI/backing scale for settings window layout and painting. |
| `settings_window_client_to_screen` | Convert a settings-window client point to screen coordinates for native popup dismissal and placement. |
| `settings_window_client_bounds` | Query the settings window content/client bounds for pointer hit testing and scroll layout. |
| `settings_window_bounds` | Query platform-native settings window frame bounds for DPI and work-area adjustment plans. |

## Settings Dropdown Host

Settings dropdown popups are platform host work. Shared settings logic decides which control opens
and which items are visible; each platform host owns native popup creation, placement and teardown:

| Operation | Purpose |
| --- | --- |
| `present_settings_dropdown` | Create and show a native settings dropdown popup for a control and item list. |
| `destroy_settings_dropdown` | Destroy the native settings dropdown popup when the parent layout, pointer state or page changes. |
| `settings_dropdown_bounds` | Query native settings dropdown popup bounds for outside-click dismissal. |

## Clipboard Host

Clipboard access is platform host work. Shared logic requests clipboard operations through
`ClipboardHost` instead of naming Win32 formats, `HWND`, `NSPasteboard` or `arboard` directly:

| Operation | Purpose |
| --- | --- |
| `read_text` | Read plain text from the system clipboard. |
| `write_text` | Write plain text to the system clipboard. |
| `read_image_rgba` | Read RGBA image data from the system clipboard. |
| `write_image_rgba` | Write RGBA image data to the system clipboard. |
| `read_file_paths` | Read file paths / file URLs from the system clipboard. |
| `write_file_paths` | Write file paths / file URLs to the system clipboard. |
| `sequence_number` | Read a platform clipboard change counter when available. |
| `write_text_ignored_by_monitors` | Write plain text with platform/source markers that ask clipboard monitors to skip a change. |
| `should_ignore_capture_by_named_format` | Honor platform/source markers that ask clipboard monitors to skip a change. |

The Windows clipboard capture path now lives in `src/app/main_clipboard_capture.rs`. It keeps
source-app filtering, retry timing, browser-download selection skipping, file/image/text capture
routing and screen-clip image normalization outside `src/app.rs`, while all platform reads still
go through `WindowsClipboardHost`.

## Status Item Host

Tray icons and menu bar status items share one platform-neutral host contract:

| Operation | Purpose |
| --- | --- |
| `install_status_item` | Install a tray icon or menu bar status item with a tooltip/title. |
| `remove_status_item` | Remove the installed status item. |
| `present_status_menu` | Present model-provided status menu entries using the native platform menu. |

## Native Popup Menu Host

Context menus, group filter menus and VV popup menus are described with
`NativePopupMenuEntry` values and rendered by each platform host. Shared row and
group menu logic builds semantic plans first, then each product/platform adapter
maps those plans to native entries before calling the host:

| Operation | Purpose |
| --- | --- |
| `present_popup_menu` | Present model-provided popup menu entries, including disabled, checked and submenu items, and return the selected command id. |

## Native Transient Window Host

Temporary floating surfaces such as the VV popup use `NativeTransientWindowHost`. Shared logic
decides content, hit testing and selection behavior; each platform adapter owns native anchoring
and each platform host owns the native no-activate floating window presentation:

| Operation | Purpose |
| --- | --- |
| `create_transient_window` | Create a reusable temporary floating window owned by a host surface. |
| `present_transient_window` | Position, size and show a temporary floating window without stealing focus. |
| `hide_transient_window` | Hide a temporary floating window without destroying its reusable native handle. |

## Native Dialog Host

Simple informational, warning, error and confirmation dialogs are described with `NativeDialogLevel`
and rendered by each platform host:

| Operation | Purpose |
| --- | --- |
| `show_message` | Present a native one-button message dialog with a title, message and severity level. |
| `confirm` | Present a native confirmation dialog and return `Yes`, `No` or `Cancel`. |

## Native Shell Open Host

Opening trusted URLs, folders and documents is platform host work. Shared code asks for a native
open operation instead of calling `ShellExecuteW`, `NSWorkspace`, `open`, or browser-specific APIs:

| Operation | Purpose |
| --- | --- |
| `open_path` | Open a trusted URL or local path with the platform default handler. |

## Native File Dialog Host

File picker UI is platform host work. Shared code describes the file selection request and each
platform presents a native picker such as Windows OpenFileDialog or macOS NSOpenPanel:

| Operation | Purpose |
| --- | --- |
| `pick_file` | Present a native single-file picker and return the selected path, cancellation, or an error. |

## Native Text Input Dialog Host

Short text prompt dialogs are platform host work. Shared code sends a request with title, label and
initial value; each platform presents a native small input dialog or sheet:

| Operation | Purpose |
| --- | --- |
| `prompt_text` | Present a native single-line text prompt and return the entered value or cancellation. |

## Native Edit Text Dialog Host

Long-form item editing is platform host work. Shared row actions describe which item should be
edited; each platform presents a native multi-line editor and returns whether the item was saved:

| Operation | Purpose |
| --- | --- |
| `open_edit_text` | Open a native multi-line edit dialog/window for a clipboard item and return whether it saved changes. |

## Native Mail Merge Window Host

The super mail merge tool is platform host work. Shared commands describe whether an initial Excel
path should be prefilled; each platform presents a native mail merge window:

| Operation | Purpose |
| --- | --- |
| `open_mail_merge` | Open the native mail merge window, optionally prefilled with an Excel/CSV path. |

## Porting Rules

- A new platform host should create equivalent events directly as `UiEvent`; ZSClip-specific async completions use `MainAsyncEvent` from `app_core::product_adapter`.
- A new platform host must not emulate Windows `WM_*` messages just to reuse Windows adapter code.
- Platform message decoding belongs only in the platform or host adapter layer.
- Product logic should consume `UiEvent`, product-adapter async events, command plans, render plans and model transitions from `app_core` / `settings_model`.
- Main window chrome commands should resolve to `MainHostExecutionPlan` kinds instead of calling platform APIs from shared command mapping.
- Main menu, tray and shortcut routing should consume `app_core::main_commands` so Windows and macOS native hosts share the same semantic intents.
- Timer callbacks should map native timer ids or runloop tokens to `MainTimerTask` / `SettingsTimerTask` before reaching shared UI logic.
- Text appearance should resolve through `NativeStyleResolver` equivalents so each platform can use its own system fonts, colors and scale.
- Settings component kinds should map through `NativeControlMapper` equivalents so shared settings logic never names native control classes directly.
- Component rendering should use `TextLayout` and `Renderer` equivalents; shared components should not call GDI, CoreGraphics, AppKit or platform drawing APIs directly.
- Settings controls should be created through `SettingsControlSpec` and `NativeSettingsControlHost` equivalents instead of hard-coded platform class names in shared settings logic.
- Clipboard reads/writes go through `ClipboardHost`: Windows maps this to `WindowsClipboardHost`, while macOS maps the same contract to `MacosClipboardHost`.
- Tray/menu bar integration goes through `StatusItemHost`; menu actions and labels come from shared status menu plans.
- Context and group popup menus go through `NativePopupMenuHost`; shared logic builds entries and platform hosts render native popup menus.
- Simple message and confirmation dialogs go through `NativeDialogHost`; specialized file dialogs remain separate platform host work.
- Opening trusted URLs, folders and documents goes through `NativeShellOpenHost`; file picker dialogs remain separate platform host work.
- Native file picker requests go through `NativeFileDialogHost`; shared logic should not run WinForms, Common Dialog, `NSOpenPanel`, or shell scripts directly.
- Short text prompt requests go through `NativeTextInputDialogHost`; shared settings actions should not directly instantiate Win32 input windows.
- Settings group create/rename prompts should use the shared `settings_group_text_input_request` model before reaching the platform text input host.
- Long-form clipboard item editing goes through `NativeEditTextDialogHost`; requests carry initial text and preferred size, while a platform-neutral save handler keeps database ownership outside native UI code.
- Super mail merge entry points go through `NativeMailMergeWindowHost`; settings and row actions should not directly instantiate Windows mail merge windows.
- Calendar-relative search terms use `SearchDateContext`: the platform host supplies its local date while `app_core` performs deterministic parsing.
- Removing Windows legacy UI means moving shared behavior into models, plans and contracts, then deleting only the replaced duplicate path. It must not remove product capabilities.
- Platform-native visual and system behavior remains in each host: Windows keeps Win32/GDI conventions, while macOS uses AppKit/SwiftUI conventions.
- Native visual implementation is platform-specific; keep platform handles, fonts, colors and drawing objects out of `app_core`.
- `Command`, `LayoutProtocol` and `Component` are shared UI protocols, not separate platform host surfaces.

## Shared Non-Host UI Protocols

These protocols live in `app_core` so shared components can compose layout, update and render logic.
They are not extra platform host surfaces:

| Protocol | Purpose |
| --- | --- |
| `Command` / `CommandQueue` | Shared command id, payload and dispatch queue contract. |
| `LayoutProtocol` | Shared component layout contract. |
| `Component` | Shared component lifecycle/update/layout/render contract. |

## Windows Reference

- The main-window adapter lives in `src/app/windows_messages.rs`; short and long text dialog adapters live with their native windows in `src/windows_text_input_dialog.rs` and `src/windows_edit_text_dialog.rs`.
- The Windows main-surface GDI executor lives in `src/app/main_renderer.rs`; it consumes the shared main-window render plan instead of defining list layout or selection behavior.
- The Windows main-surface input executor lives in `src/app/main_input.rs`; it consumes shared pointer target/transition plans and shortcut execution plans instead of keeping input behavior inside `app.rs`.
- The Windows main-window lifecycle and geometry executor lives in `src/app/main_window.rs`; it consumes `WindowsMainWindowHost` from `src/app/main_window_host.rs` for appearance, bounds, DPI/layout, hide/show, pointer capture, native drag and destroy behavior instead of keeping those adapters in `app.rs` or the large `hosts.rs` adapter.
- The Windows edge auto-hide executor lives in `src/app/main_edge_auto_hide.rs`; it owns screen-edge docking state, hidden-position calculation, animation ticks and hot-zone restoration outside the large `hosts.rs` adapter.
- The Windows main search control host lives in `src/app/main_search_host.rs`; it owns native EDIT creation, font resources, visibility, text and focus operations instead of sharing the large `hosts.rs` adapter.
- The Windows transient floating-window host lives in `src/app/transient_window_host.rs`; it owns no-activate class registration, popup creation, bounds presentation, hide and destroy operations for the shared `NativeTransientWindowHost` contract.
- The Windows main platform binding executor lives in `src/app/main_platform_bindings.rs`; it consumes shared hotkey registration plans and owns Windows global-hotkey plus clipboard-listener registration outside `app.rs`.
- The Windows main event executor lives in `src/app/main_events.rs`; it consumes shared command, timer, `UiEvent`, `ApplicationEvent` and `MainAsyncEvent` protocols while `src/app.rs` remains the module root and public re-export surface.
- Windows cloud-sync queue/completion execution lives in `src/app/main_cloud_sync.rs`; LAN envelope conversion, receive application and clipboard mirroring live in `src/app/main_lan_sync.rs`. Shared events reach these adapters through `main_events.rs`, without returning transport logic to `app.rs`.
- The Windows process entry, main window procedure and startup integration wiring live in `src/app/main_entry.rs`; `src/app.rs` no longer owns product behavior, shared imports or platform constants.
- The Windows app adapter prelude lives in `src/app/prelude.rs`; app submodules consume it explicitly for shared adapter imports instead of globbing the application root.
- Windows timer ids, class names, clipboard format ids, hotkey ids and other adapter constants live in `src/app/constants.rs`; native hosts should keep equivalent platform identifiers in their own adapter id/session layer.
- The Windows main runtime state container, window role and icon resource ownership live in `src/app/state.rs`; this separates state representation from the remaining product behavior methods in `src/app.rs`.
- The Windows clip payload data helpers live in `src/app/data.rs`; preview text, file previews, dedupe signatures, QR image generation, image materialization and thumbnail preparation are product data semantics rather than application-root window code.
- The Windows main runtime state behavior lives in `src/app/state_runtime.rs`; payload cache access, selected item resolution, scroll anchors, transient clipboard guards and dedupe insert behavior should stay outside the application root.
- The Windows settings window state lives in `src/app/settings_state.rs`; `SettingsWndState::new` owns the default Rust-side state/cache initialization while create adapters add native resources around it. Direct field access is currently scoped to the `app` adapter layer and should be reduced over time into section/session accessors rather than returning to `app.rs`.
- The Windows platform helpers live in `src/app/platform_helpers.rs`; native dialog helpers and background clipboard writes should stay in platform/helper adapters rather than returning to `app.rs`.
- The Windows main-view helpers live in `src/app/main_view_helpers.rs`; theme color mapping, hit helpers, empty-state mapping, title-button visibility and preview policy should stay outside the application root.
- The Windows row command executor lives in `src/app/main_row_commands.rs`; it consumes shared row action, data, dialog and group-assignment plans instead of keeping row command execution inside `app.rs`.
- Quick-search URL preparation and shell launch are owned by `src/app/main_row_commands.rs`; VV process identity is owned by `src/app/vv_hook.rs`, so neither product action leaks back into `app.rs`.
- The Windows row tool executor lives in `src/app/main_row_tools.rs`; it owns AI text cleanup, OCR/translation jobs, OCR image input preparation and file drag materialization outside the main window procedure.
- The Windows row/group popup menu presenter lives in `src/app/main_popup_menus.rs`; it consumes shared row and group popup-entry builders and leaves menu presentation to `WindowsPopupMenuHost`.
- The Windows VV popup presenter lives in `src/app/vv_popup.rs`; it consumes shared VV layout/render/select plans, reuses the shared group-filter popup-entry builder for the native group menu, and leaves floating-window presentation to `WindowsTransientWindowHost`.
- The Windows VV keyboard hook and target/backspace adapter lives in `src/app/vv_hook.rs`; non-Windows hosts should implement their own trigger mechanism and reuse the shared VV popup/select plans instead of copying the low-level Win32 hook.
- The Windows low-level input session adapter lives in `src/app/main_low_level_input.rs`; it owns outside-click protected-scope checks, outside-hide timers, edge-auto-hide timers and Quick Escape hook installation outside the large `hosts.rs` adapter.
- The Windows main hover preview adapter lives in `src/app/main_hover_preview.rs`; it owns hover-preview refresh, blocked-hit checks, mouse-leave cleanup and tracking rearming outside the large `hosts.rs` adapter.
- The Windows startup integration adapter lives in `src/app/main_startup_integrations.rs`; it owns tray resync, hotkey/plain-paste retry, VV hook retry, clipboard-listener retry and update-state notifications outside the large `hosts.rs` adapter.
- The Windows main-window refresh adapter lives in `src/app/main_window_refresh.rs`; it owns loaded-settings application, database reload, settings-window refresh and peer-window synchronization outside the large `hosts.rs` adapter.
- The Windows main-window registry lives in `src/app/main_window_registry.rs`; it owns main/quick handle registration, host iteration, app-window checks, state-pointer lookup and cross-host clipboard ignore guards outside the large `hosts.rs` adapter.
- Main hover clear and no-activate row hit testing live with `src/app/main_hover_preview.rs`, keeping hover cleanup and hover preview behavior together outside the large `hosts.rs` adapter.
- The Windows settings-window input dispatcher lives in `src/app/settings_input.rs`; pointer, keyboard and settings window/DPI/theme event handlers live in `src/app/settings_pointer_input.rs`, `src/app/settings_keyboard_input.rs` and `src/app/settings_window_events.rs` instead of keeping settings input behavior inside `app.rs`.
- The Windows settings command queue, timer and control-selection executors live in `src/app/settings_command_queue.rs`, `src/app/settings_timer_tasks.rs` and `src/app/settings_control_selection.rs`; they drain shared settings `Command` values, map settings timer tasks and apply dropdown selections outside `src/app.rs`, while `src/app/settings_control_selection_general.rs`, `src/app/settings_control_selection_cloud.rs`, `src/app/settings_control_selection_hotkey.rs`, `src/app/settings_control_selection_plugin.rs` and `src/app/settings_control_selection_group.rs` own domain-specific selection application.
- The Windows settings action executor lives in `src/app/settings_actions.rs`; sync, group and platform action side effects are split into `src/app/settings_sync_actions.rs`, `src/app/settings_group_actions.rs` and `src/app/settings_platform_actions.rs` instead of one large product-action file. Concrete platform actions are further split into `src/app/settings_platform_actions_hotkey.rs`, `src/app/settings_platform_actions_general.rs`, `src/app/settings_platform_actions_plugin.rs`, `src/app/settings_platform_actions_about.rs` and `src/app/settings_platform_actions_system.rs`.
- The Windows settings dropdown executor lives in `src/app/settings_dropdown.rs`; it builds `NativeSettingsDropdownRequest` values from shared settings option models and dispatches request construction by settings domain. `src/app/settings_dropdown_general.rs`, `src/app/settings_dropdown_cloud.rs`, `src/app/settings_dropdown_hotkey.rs`, `src/app/settings_dropdown_group.rs` and `src/app/settings_dropdown_plugin.rs` own General, Cloud, Hotkey, Group and Plugin option requests, while `src/app/settings_dropdown_host.rs` leaves popup creation, destruction and bounds queries to `WindowsSettingsDropdownHost`.
- The Windows plugin settings section adapter lives in `src/app/settings_plugin_sections.rs`; it owns plugin section relayout orchestration outside the large `hosts.rs` adapter. Control movement/visibility lives in `src/app/settings_plugin_sections_controls.rs`, plugin card layout and host refresh live in `src/app/settings_plugin_sections_layout.rs`, provider section relayout lives in `src/app/settings_plugin_sections_providers.rs`, and tool section relayout lives in `src/app/settings_plugin_sections_tools.rs`.
- The Windows multi-sync settings section adapter lives in `src/app/settings_multi_sync_sections.rs`; it owns WebDAV/LAN dynamic cards, cloud-page handle reset, section refresh and page rebuild outside the large `hosts.rs` adapter.
- The Windows group settings section adapter lives in `src/app/settings_group_sections.rs`; it owns Group page synchronization orchestration outside the large `hosts.rs` adapter. Group cache/current-tab helpers live in `src/app/settings_group_sections_cache.rs`, VV source/group display synchronization lives in `src/app/settings_group_sections_display.rs`, group list refresh/selection/order behavior lives in `src/app/settings_group_sections_list.rs`, and `src/app/settings_group_page.rs` owns Group page control construction.
- The Windows General settings page adapter lives in `src/app/settings_general_page.rs`; it owns the General page entry and delegates startup/retention/behavior/sound control creation to `src/app/settings_general_page_startup.rs`, and skip-window/position/config action creation to `src/app/settings_general_page_window.rs`.
- The Windows Hotkey settings page adapter lives in `src/app/settings_hotkey_page.rs`; it owns the Hotkey page entry and delegates main/plain shortcut control creation to `src/app/settings_hotkey_page_shortcuts.rs`, and Win+V system-history actions plus explanatory notes to `src/app/settings_hotkey_page_system.rs`.
- The Windows Plugin settings page adapter lives in `src/app/settings_plugin_page.rs`; it owns the Plugin page entry and delegates quick-search control creation to `src/app/settings_plugin_page_search.rs`, OCR/translation control creation to `src/app/settings_plugin_page_ocr_translate.rs`, and AI/mail-merge/WPS/QR tool control creation to `src/app/settings_plugin_page_tools.rs`.
- The Windows About settings page adapter lives in `src/app/settings_about_page.rs`; it owns the About page entry and delegates version/summary/source-link controls to `src/app/settings_about_page_metadata.rs`, update status/action controls to `src/app/settings_about_page_update.rs`, and data-directory display to `src/app/settings_about_page_data.rs`.
- The Windows Cloud/LAN settings page adapter lives in `src/app/settings_cloud_page.rs`; it owns Cloud page mode switching outside the large `hosts.rs` adapter, while `src/app/settings_cloud_page_lan_devices.rs` owns LAN pending/discovered list refresh and selected LAN row resolution, `src/app/settings_cloud_page_webdav.rs` owns WebDAV form/action page construction, and `src/app/settings_cloud_page_lan.rs` owns LAN form/list/QR page construction.
- The Windows settings owner-draw adapter lives in `src/app/settings_owner_draw.rs`; it owns settings hover checks and the owner-draw dispatch entry outside the large `hosts.rs` adapter. LAN QR rendering lives in `src/app/settings_owner_draw_qr.rs`, source-link drawing lives in `src/app/settings_owner_draw_link.rs`, and QR/toggle/dropdown/accent/button role classification lives in `src/app/settings_owner_draw_roles.rs`.
- The Windows settings page builder adapter lives in `src/app/settings_page_builder.rs`; it owns builder identity, control-registration entry and page-section helpers outside the large `hosts.rs` adapter, while `src/app/settings_raw_controls.rs` owns raw label/button/dropdown/input/listbox/toggle helpers, `src/app/settings_form_fields.rs` owns form label/value/dropdown/input/button rows, `src/app/settings_form_actions.rs` owns form action rows, QR actions and owner-draw action ownership, `src/app/settings_control_registry.rs` owns settings control registration and scrollable-page resolution and `src/app/settings_control_factory.rs` owns native label/input/listbox/button/dropdown/toggle wrappers.
- The Windows settings page navigation adapter lives in `src/app/settings_page_navigation.rs`; it owns page-navigation orchestration outside the large `hosts.rs` adapter. Child-control repositioning lives in `src/app/settings_page_navigation_controls.rs`, scroll updates and scrollbar reveal live in `src/app/settings_page_navigation_scroll.rs`, and page switching lives in `src/app/settings_page_navigation_switch.rs`.
- The Windows settings page ensure adapter lives in `src/app/settings_page_ensure.rs`; it owns lazy settings page construction routing outside the large `hosts.rs` adapter.
- The Windows settings page sync adapter lives in `src/app/settings_page_sync.rs`; it owns page text/enabled synchronization, position-field enablement and multi-sync mode helpers outside the large `hosts.rs` adapter, while `src/app/settings_page_sync_cloud.rs` owns Cloud transport summary and dispatch, `src/app/settings_page_sync_cloud_webdav.rs` owns WebDAV text/enabled/status synchronization, `src/app/settings_page_sync_cloud_lan.rs` owns LAN text/enabled/trusted-summary/list refresh synchronization, and `src/app/settings_page_sync_plugin.rs` owns quick-search, OCR, translate and tool-toggle state synchronization.
- The Windows settings toggle-state adapter lives in `src/app/settings_toggle_state.rs`; it owns settings toggle reads and state flips outside the large `hosts.rs` adapter, while `src/app/settings_toggle_state_general.rs`, `src/app/settings_toggle_state_cloud.rs`, `src/app/settings_toggle_state_hotkey.rs`, `src/app/settings_toggle_state_plugin.rs` and `src/app/settings_toggle_state_group.rs` own the concrete setting-domain toggle fields.
- The Windows settings host-helper adapter lives in `src/app/settings_host_helpers.rs`; it owns small text, visibility/enabled, invalidation and theme-resource helpers outside the large `hosts.rs` adapter.
- The Windows settings app apply/collect adapters live in `src/app/settings_app_apply.rs` and `src/app/settings_app_collect.rs`; they own native control apply/collect wiring outside the large `hosts.rs` adapter. Collect-domain native reads are split into `src/app/settings_app_collect_general.rs`, `src/app/settings_app_collect_hotkey.rs`, `src/app/settings_app_collect_plugin.rs`, `src/app/settings_app_collect_group.rs` and `src/app/settings_app_collect_cloud.rs`.
- The Windows settings app-effects adapter lives in `src/app/settings_app_effects.rs`; it owns persistence, integration refresh, data reload and peer-sync orchestration outside the apply/collect adapter, while `src/app/settings_app_integration_effects.rs`, `src/app/settings_app_data_effects.rs` and `src/app/settings_app_window_effects.rs` own the desktop-integration, data-refresh and native-window refresh pipelines.
- The old Windows `src/app/hosts.rs` aggregation module is retired; new ports should look at the named adapters and shared host contracts instead of a monolithic host file.
- The Windows settings-window boundary is split across `src/app/settings_window.rs`, `src/app/settings_window_create.rs`, `src/app/settings_window_lifecycle.rs`, `src/app/settings_window_destroy.rs`, `src/app/settings_window_metrics.rs`, `src/app/settings_window_layout.rs`, `src/app/settings_window_colors.rs`, `src/app/settings_window_surface_controls.rs`, `src/app/settings_window_owner_draw.rs` and `src/app/settings_window_paint.rs`; proc routing, native state/control creation, lifecycle/repaint facades, destroy cleanup, page/control metrics refresh, DPI/work-area geometry, CTLCOLOR role mapping, semantic surface-control classification, owner-draw item buffering and buffered full-window paint each stay in their own Windows adapter while `src/app.rs` only dispatches product commands/events into them.
- Settings window and settings dropdown host implementations live in `src/settings_ui_host.rs`; native class registration and platform operations are owned there rather than by `src/app.rs`.
- The Windows native style resolver and control mapper live in `src/win_native_style.rs`.
- The Windows renderer and text layout implementation lives in `src/platform/ui_renderer.rs`.
- The Windows settings control host implementation is `WindowsSettingsControlHost`.
- The Windows clipboard implementation is `WindowsClipboardHost` in `src/platform/clipboard.rs`; `src/app/main_paste.rs`, `src/app/main_clipboard_capture.rs` and `src/app/main_platform_bindings.rs` consume it or its listener boundary. Source-window identity, RGBA normalization and guarded legacy bitmap decoding are also owned by the capture adapter, while `src/app.rs` no longer owns clipboard capture/listener routing.
- The Windows paste-target discovery adapter lives in `src/app/main_paste_target_discovery.rs`; it owns skip-class parsing, top-level-window enumeration and next-target selection outside the large `hosts.rs` adapter.
- The Windows status item implementation is `WindowsStatusItemHost` in `src/platform/tray_icon.rs`; `src/tray.rs` only builds localized shared menu entries and window behavior.
- The Windows native popup menu implementation is `WindowsPopupMenuHost` in `src/platform/menu.rs`; shared builders in `app_core::main_window` now construct row, row-group and group-filter `NativePopupMenuEntry` values, while Windows, AppKit and GTK hosts own only native menu presentation.
- The Windows transient window implementation is `WindowsTransientWindowHost` in `src/app/transient_window_host.rs`; `src/app/vv_popup.rs` computes VV popup bounds and consumes the transient host instead of keeping floating-window presentation in `src/app.rs` or the large `hosts.rs` adapter.
- The Windows message dialog implementation is `WindowsDialogHost` in `src/platform/dialog.rs`; row action info/error messages and edit-close confirmation now use this host.
- The Windows shell open implementation is `WindowsShellOpenHost` in `src/platform/shell.rs`; `src/shell.rs` validates trusted schemes and then consumes this host.
- The Windows window identity implementation is `WindowsWindowIdentityHost` in `src/platform/window_identity.rs`; clipboard-source, VV identity and paste-target state queries consume this host instead of calling Win32 window/process probes in `src/app.rs`.
- The Windows file dialog implementation is `WindowsFileDialogHost` in `src/platform/file_dialog.rs`; paste sound selection now consumes this host instead of building a dialog in product code.
- The Windows text input dialog implementation is `WindowsTextInputDialogHost` in `src/windows_text_input_dialog.rs`; its Win32 class, window procedure, painting and modal loop no longer live in `src/app.rs`.
- The Windows edit text dialog implementation is `WindowsEditTextDialogHost` in `src/windows_edit_text_dialog.rs`; its Win32 class, painting, modal loop and close confirmation are isolated from `src/app.rs`, and the host contains no database or settings access.
- The Windows mail merge implementation is `WindowsMailMergeWindowHost` in `src/mail_merge_native.rs`; the native launch function is private to that host module, while settings and row actions only consume the shared host contract.
- The guard test `platform_ui_message_decoding_stays_in_host_adapters` keeps Windows UI message decoding inside platform/host adapters.
- The guard test `windows_host_adapters_cover_required_ui_surfaces` checks that the Windows host covers every surface in `REQUIRED_UI_HOST_SURFACES`.
- The guard test `windows_clipboard_host_covers_required_operations` checks that Windows still covers the clipboard host contract during migration.

## macOS Reference

- The macOS entry point is `src/macos_app.rs`; production builds compile it for `target_os = "macos"` and test builds compile the scaffold for contract verification.
- `MacosMainWindowModel::startup_plan` produces the shared `NativeMainWindowRequest` and mount lifecycle transition; macOS `run()` now sends that request through `MacosMainWindowHost` instead of being a contract-summary-only placeholder.
- `MacosApplicationModel` is the macOS state root for shared lifecycle state, command queue, main/settings window models and product event routing. It also tracks settings window create/lifecycle/metrics/layout/paint sessions so AppKit can mirror the split Windows adapters without copying their Win32 procedure code. `run()` mounts and activates this model before handing startup requests to the native host.
- `MacosBackgroundTaskState` tracks cloud-sync exclusivity, LAN refresh generations, completed image/text operations and thumbnail cache completions as semantic state driven by shared application/async event routes.
- `MacosWindowSessionState` owns created main/quick handles, settings-window identity and visibility plus render/presentation generations, giving the future AppKit delegate persistent window-session state instead of temporary local handles.
- `MacosClipPayloadDataState` records the latest shared `ClipItem` kind, preview text, preview generation and thumbnail cache ids consumed by the native UI, so AppKit list rendering can attach to Rust payload semantics instead of Windows row helpers.
- `MacosMainListSessionState` records visible `ClipItem` ids, selected ids, scroll anchors and list/selection/scroll generations, preparing AppKit table/list rendering without copying Windows `AppState` selection and scroll helpers.
- `MacosSettingsSessionState` records current settings page, dirty state, draft/applied generations and presentation generation, and `MacosSettingsWindowStateSessionState` records initial page, DPI, reset control count and dynamic section count. Together they prepare an AppKit preferences window without copying Windows `SettingsWndState`.
- `MacosSettingsPluginSectionSessionState` records visible plugin/provider sections, enabled feature count and generation, while `MacosSettingsPluginSectionDomainSessionState` records controls/layout/provider/tool/host-refresh domain counts, preparing native AppKit plugin settings sections without copying Windows provider-card layout code.
- `MacosSettingsMultiSyncSectionSessionState` records selected sync mode, visible section count, rebuild generation and generation, preparing native AppKit WebDAV/LAN settings sections without copying Windows cloud-page rebuild code.
- `MacosSettingsGroupSectionSessionState` records VV source tab, group view tab, selected group id and record/phrase group counts, `MacosSettingsGroupSectionDomainSessionState` records cache/display/list/selection/order domain counts, and `MacosSettingsGroupPageSessionState` records toggle/dropdown/tab/list/action/status control counts, preparing native AppKit group preferences without copying Windows listbox refresh/order or page construction code.
- `MacosSettingsGeneralPageSessionState` records startup/behavior toggle counts, max-items label and skip-window enablement, while `MacosSettingsGeneralPageSectionSessionState` records startup, retention, behavior, sound, skip-window, position and action section counts, preparing native AppKit general preferences without copying Windows control creation code.
- `MacosSettingsHotkeyPageSessionState` records main/plain hotkey previews and recording state, while `MacosSettingsHotkeyPageSectionSessionState` records main shortcut, plain shortcut, system action and note counts, preparing native AppKit shortcut preferences without copying Windows hotkey page controls.
- `MacosSettingsPluginPageSessionState` records quick-search enablement, OCR provider, translation provider and tool-toggle count, while `MacosSettingsPluginPageSectionSessionState` records quick-search, OCR, translate, tool-toggle and tool-action control counts, preparing native AppKit plugin preferences without copying Windows provider controls.
- `MacosSettingsAboutPageSessionState` records source availability, update availability and data-directory text, while `MacosSettingsAboutPageSectionSessionState` records metadata, source-link, update status/action and data-label counts, preparing native AppKit About preferences without copying Windows page creation code.
- `MacosSettingsCloudPageSessionState` records selected sync mode, pending pair count, discovered device count and selected LAN row, `MacosSettingsCloudWebdavPageSessionState` records WebDAV field/action/status counts, `MacosSettingsCloudLanPageSessionState` records LAN field/action/list/QR/helper counts, and `MacosSettingsCloudLanDeviceListSessionState` records LAN list refresh counts plus selected pair/device rows, preparing native AppKit WebDAV/LAN preferences without copying Windows listbox or form construction code.
- `MacosSettingsOwnerDrawSessionState` records hover control state, QR payload availability and button/toggle draw counts, while `MacosSettingsOwnerDrawDomainSessionState` records QR/source-link/toggle/dropdown/accent/button role counts, preparing native AppKit rendering without copying Windows owner-draw code.
- `MacosSettingsPageBuilderSessionState` records registered-control, owner-draw-control and section counts, `MacosSettingsRawControlSessionState` records raw label/button/dropdown/input/listbox/toggle helper counts, `MacosSettingsFormFieldSessionState` records label/value/dropdown/input/button row counts, `MacosSettingsFormActionSessionState` records owner-draw/action-row/QR/toggle action counts, `MacosSettingsControlRegistrySessionState` records registered/scrollable/page counts, and `MacosSettingsControlFactorySessionState` records label/input/listbox/button/toggle factory counts, preparing native AppKit settings page construction without copying Windows control factories.
- `MacosSettingsPlatformActionDomainSessionState` records Hotkey/General/Plugin/About/System platform-action domain counts, preparing native AppKit action side effects without copying the Windows product-action dispatch chain.
- `MacosSettingsPageNavigationSessionState` records current page, scroll offset and reposition count, while `MacosSettingsPageNavigationDomainSessionState` records control-reposition, scroll-update, page-switch, visibility and redraw domain counts, preparing native AppKit settings navigation/scroll containers without copying Windows child-window movement.
- `MacosSettingsPageEnsureSessionState` records the last ensured page and built-page count, preparing native AppKit lazy-page construction without copying Windows page creation routing.
- `MacosSettingsPageSyncSessionState` records synced-page count, enabled-control count and invalidation count, `MacosSettingsCloudSyncSessionState` records transport mode, WebDAV control count, LAN control count and LAN refresh generation, `MacosSettingsCloudWebdavSyncSessionState` records WebDAV sync control/status state, `MacosSettingsCloudLanSyncSessionState` records LAN sync control/list invalidation state, and `MacosSettingsPluginSyncSessionState` records quick-search/OCR/translate/tool sync state, preparing native AppKit settings state synchronization without copying Windows control updates.
- `MacosSettingsToggleStateSessionState` records the toggled control id and enabled-toggle count, while `MacosSettingsToggleDomainSessionState` records General/Cloud/Hotkey/Plugin/Group toggle-domain counts, preparing native AppKit switch/checkbox state updates without copying Windows toggle handling.
- `MacosSettingsHostHelperSessionState` records text-update count, invalidation count and theme refresh generation, preparing native AppKit control updates and theme resources without copying Windows helper calls.
- `MacosSettingsAppApplyCollectSessionState` records apply/collect generations, saved-settings count and peer-sync generation, while `MacosSettingsAppCollectDomainSessionState` records General/Hotkey/Plugin/Group/Cloud collect-domain counts, preparing native AppKit preferences synchronization without copying Windows save/reload side effects or Win32 control-read chains.
- `MacosSettingsAppEffectsSessionState` records persisted, integration refresh, data refresh, window refresh and peer-sync generations, preparing native AppKit post-save product effects without mixing them with control apply/collect state.
- `MacosSettingsSyncActionDomainSessionState` records WebDAV and LAN sync-action domain counts, preparing native AppKit WebDAV scheduling, LAN discovery/pairing and QR link actions without copying Windows sync-action side effects.
- `MacosMainVisualSessionState` records title-button visibility, empty-state kind, image-preview policy and visual generation, preparing AppKit main-window presentation without copying Windows view helpers.
- `MacosHoverPreviewSessionState` records hover preview visibility, hovered item id and mouse-leave tracking state, preparing AppKit tracking areas or popovers without copying Windows mouse-hover/message handling.
- `MacosAdapterPreludeState` records the shared contract roots and native adapter roots consumed by the macOS host, preparing an AppKit/SwiftUI prelude boundary that does not copy the Windows `app/prelude.rs`.
- `MacosNativeIdSessionState` records AppKit/SwiftUI window identifiers, timer identifiers and status item identity, preparing a native id/session boundary that does not copy Windows `app/constants.rs`.
- `MacosMainSearchSessionState` records the native search handle, visibility, text and style resource, preparing `NSSearchField` / SwiftUI search state without copying the Windows search host.
- `MacosStartupIntegrationSessionState` records status-item, hotkey, pasteboard-monitor and VV-monitor registration plus recovery ticks, preparing `NSApplicationDelegate` startup recovery without copying Windows tray/hotkey/listener retry code.
- `MacosWindowRefreshSessionState` records settings reload, database reload, settings-window refresh and peer-window sync generations, preparing AppKit main/quick window refresh without copying Windows refresh helpers.
- `MacosWindowRegistrySessionState` records main/quick native handles plus clipboard ignore and skip-next generations, preparing an AppKit window registry without copying the Windows HWND registry.
- `MacosHoverClearSessionState` records hover-clear, pointer-down clear and no-activate row hit-test state, preparing native AppKit tracking cleanup without copying Windows hover invalidation helpers.
- `MacosLowLevelInputSessionState` records outside-hide timer state, edge-auto-hide timer state, Quick Escape event-monitor state and the last protected pointer scope, preparing AppKit event monitors and run-loop timers without copying Windows low-level hooks.
- The macOS host starts from the `app_core` contract summary and must not copy Windows `app.rs` / Win32 message handling.
- Current macOS status is a host scaffold that consumes arbitrary shared main-window render inputs, shared main-window pointer/shortcut plans, shared row action plans, including populated row/selection/scroll states, plus the shared six-page settings navigation/content/paint/input plans, settings command/timer protocols, settings selection-domain sessions and settings dropdown-domain option models.
- `MacosMainEventModel` classifies shared ZSClip application and async completion events into AppKit-facing semantic routes for LAN refresh, VV presentation, page refresh, settings refresh, image paste, text operations and thumbnail caching.
- `MacosSettingsWindowModel` also consumes the shared settings-window fit and scale-transition geometry plans, so native `NSWindow.frame` and backing-scale changes can follow the same Rust policy without copying the Windows adapter.
- `MacosClipboardHost` provides text/image clipboard access without reusing Windows code, and target macOS builds now route file path read/write plus clipboard sequence queries through `NSPasteboard` file URLs and `changeCount`; `MacosApplicationModel` polls that sequence into the shared `clipboard_changed` event path.
- `LinuxClipboardHost` provides text/image clipboard access without reusing Windows code, and target Linux builds can fingerprint system text/image clipboard contents to advance the shared sequence counter for polling-based change detection; `LinuxApplicationModel` polls that sequence into the shared `clipboard_changed` event path.
- `MacosStatusItemHost` consumes the same `StatusMenuEntry` model, and the first real AppKit host slice presents those entries through an `NSStatusItem` menu; target macOS smoke evidence is still required before treating it as verified.
- `MacosPopupMenuHost` consumes the same `NativePopupMenuEntry` model, and the first real AppKit host slice maps shared full Row Menu and Group Filter entries to `NSMenu`; the main AppKit host also opens a first-pass VV Popup native window from the shared render plan, bridges row selections through `vv_select_requested`, runs the selected item through the native VV paste bridge and attempts AppKit `CGEvent` Backspace plus `Cmd+V` delivery. Real VV trigger smoke, target proof that the OS paste shortcut succeeds, target-window identity and target macOS smoke evidence remain later host work.
- `MacosDialogHost` tracks the same message and confirmation contract, and the AppKit settings surface now exposes first-pass `NSAlert` Info/Confirm actions through the shared native dialog slice.
- `MacosWindowIdentityHost` tracks process/class/root/foreground/self-window identity queries; native `NSRunningApplication`, `NSWindow` and Accessibility lookup are the next macOS implementation step.
- `MacosShellOpenHost` consumes the same `NativeShellOpenHost` contract and target macOS builds hand trusted URLs/files to `NSWorkspace.openURL`; target macOS smoke evidence is still required before treating shell-open handoff as verified.
- `MacosFileDialogHost` consumes the same `NativeFileDialogHost` contract, and `pick_macos_native_file` maps target-only requests to `NSOpenPanel`; default target smoke verifies the file-picker host boundary with an injected selected path, while interactive `NSOpenPanel` evidence is still required before treating the real picker UI as complete.
- `MacosSettingsDropdownHost` consumes the same dropdown request lifecycle and shared max-items option labels, while `MacosSettingsDropdownPluginSessionState` records plugin-provider option counts and `MacosSettingsDropdownDomainSessionState` records General/Cloud/Hotkey/Plugin/Group dropdown-domain counts; native `NSPopUpButton` or `NSMenu` presentation is the next settings popup implementation step.
- `MacosTextInputDialogHost` consumes the same `NativeTextInputDialogHost` contract; target macOS builds present an `NSAlert` with an editable `NSTextField` accessory view for single-line text prompts, while target smoke evidence is still required before treating text prompts as verified.
- `LinuxTextInputDialogHost` consumes the same `NativeTextInputDialogHost` contract; target Linux builds present a modal GTK `Dialog` with an editable `Entry` for single-line text prompts, while target smoke evidence is still required before treating text prompts as verified.
- `MacosEditTextDialogHost` consumes initial text and preferred size; target macOS builds present a first-pass AppKit Save/Cancel editor, submit edited text through the shared save handler and return final size, while richer `NSTextView` or SwiftUI presentation remains future polish.
- `LinuxEditTextDialogHost` consumes initial text and preferred size; target Linux builds present a first-pass GTK Save/Cancel editor, submit edited text through the shared save handler and return final size, while richer `TextView` presentation remains future polish.
- `MacosMailMergeWindowHost` consumes the same `NativeMailMergeWindowHost` contract; native macOS mail merge presentation is future host work.
- macOS-specific long-running pasteboard monitoring, source filtering and broader target-OS smoke evidence remain to be implemented on top of the `NSPasteboard` file URL/change-count code path.
- Linux-specific long-running clipboard monitoring, file URL support through GTK/GDK or desktop portals and broader target-OS smoke evidence remain to be implemented on top of the system fingerprint sequence path.

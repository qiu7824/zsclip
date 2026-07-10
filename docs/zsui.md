# ZSUI

## Current API Snapshot

The public framework-shaped entry point now lives in the standalone
`zsui` crate at `E:\rust\zsui` / `https://github.com/qiu7824/zsui`.
ZSClip re-exports that crate as `zsclip::zsui` for compatibility. The API is
intentionally small and declaration-first, so application code and AI tools can
read or generate one Rust UI description without importing Win32, AppKit, GTK
or ZSClip product modules.

Minimal declaration:

```rust
use zsclip::zsui::{app, Command, TraySpec, Window};

let runtime = app("ZSClip")
    .window(Window::new("ZSClip").size(900, 620).resizable(true))
    .tray(
        TraySpec::new()
            .tooltip("ZSClip")
            .item("Open", Command::ShowMainWindow)
            .separator()
            .item("Quit", Command::Quit),
    )
    .global_hotkey("Alt+V", Command::OpenQuickPanel)
    .run();
```

For tests, demos and backend bring-up, use `run_with_host`:

```rust
use zsclip::zsui::{app, Command, MemoryHost, TraySpec, Window};

let mut host = MemoryHost::new();
let runtime = app("ZSClip")
    .window(Window::new("ZSClip").size(900, 620))
    .tray(TraySpec::new().item("Open", Command::ShowMainWindow))
    .global_hotkey("Alt+V", Command::OpenQuickPanel)
    .run_with_host(&mut host)?;
```

For a minimal real native OS window on Windows, macOS or Linux, the standalone
crate now exposes:

```rust
zsui::native_window("ZSUI").size(900, 620).run()?;
```

This path uses the crate's `NativeWindowHost` to create an actual platform
window event loop while keeping the richer product UI hosts in ZSClip.
Android and Harmony are now named in the standalone crate's capability model
as mobile scaffolds; they still need dedicated Activity/Ability runtime hosts
before they can create mobile native surfaces.

The first standalone version exports these stable declaration and protocol
types:

- `WindowSpec`
- `Window`
- `TraySpec`
- `MenuSpec`
- `MenuItemSpec`
- `HotkeySpec`
- `ClipboardData`
- `SettingsPageSpec`
- `SettingsItemSpec`
- `Command`
- `AppEvent`
- `HostCapabilities`
- shared geometry / command / event / layout / component / render protocols
- required host surfaces for main/settings/dialog ingress
- native settings control specs, mappers and control host contracts
- native adapter manifest, timer routing and reusable platform service host
  contracts now come from the standalone crate

The central backend trait is `ZsuiHost`. It covers creating a main window,
showing/hiding a window, creating a tray/status menu, registering global
hotkeys, reading/writing clipboard data, opening a file picker, showing native
dialogs and querying `HostCapabilities`.

`MemoryHost` is the executable demo/test backend. `PlatformHost` is a safe
scaffold for the current target: it accepts declarations where the platform
has a known native path and reports partial or unsupported capabilities instead
of panicking. The current Windows production path is still the existing
`src/app/*` plus `src/platform/*` implementation; the new `ZsuiHost` adapter is
the migration target, not a rewrite of the running application.

Window declarations also report feature-level degradation. For example, a host
may accept a window declaration while reporting that transparent windows,
always-on-top behavior, native decorations or resize policy are only partially
supported on the active backend. The declaration stays portable; the host owns
the native fallback.
Hosts keep both sides of that decision: the requested `WindowSpec` and an
effective spec after unsupported traits fall back to a standard native window
shape. This lets adapters create a real platform window from effective values
without losing the user's original declaration.

The native host bridge uses the same semantic fields: title, default size,
minimum size, resize policy, decorations, always-on-top and transparency. Win32
maps those fields to window styles, while the AppKit and GTK hosts now collect
their main window title and sizing traits from a `Window` declaration before
translating them to `NSWindow` or `ApplicationWindow` calls.
Win32 also applies declared minimum size through the native
`WM_GETMINMAXINFO` resize constraint.
Internal startup code can call `NativeMainWindowRequest::from_zsui_window_for_host`
to apply `HostCapabilities` first, so unsupported traits are removed before the
native window is created.
That request also carries `degraded_capabilities`, so the platform startup path
can inspect which requested traits were downgraded instead of losing the reason
after effective native options are calculated.
Recording native hosts keep those details with their create-window requests,
which gives macOS and Linux bring-up tests the same visibility as the shared
`MemoryHost`.
Use the `*_native_window_host()` capability constructors for real Win32,
AppKit and GTK window hosts; keep `*_scaffold()` for recording or bring-up
hosts that only accept declarations partially.
Traits that are not wired in a real native window host, such as current main
window transparency, are marked unsupported there and resolve to the standard
opaque native window shape.

See also:

- `docs/zsui-architecture.md`
- `docs/zsui-platform-matrix.md`

ZSUI is the UI architecture being extracted from ZSClip. It is not a single
cross-platform skin. It is a Rust-first contract for sharing UI behavior while
each operating system keeps a native host.

Tagline: shared Rust UI logic with native platform hosts.

## Definition

ZSUI means:

- Shared Rust core owns state transitions, layout math, render plans and action plans.
- Platform hosts own native windows, controls, menus, dialogs, clipboard access and drawing.
- Product code asks for semantic operations such as `begin_main_window_drag`, not Win32,
  AppKit, Android or browser calls.
- Host boundaries are explicit traits with required operation lists and contract tests.
- Platform implementations may look different, as long as they consume the same plans.

## Reuse Boundary

The reusable framework layer should eventually contain:

- `app_core` contracts, command ids, host traits and platform-neutral events.
- `app_core::zsui` framework identity, API version and reusable layer markers.
- `app_core::ui_surface_protocol` required host surfaces and adapter names.
- `app_core::command_protocol` stable command ids, payloads and command queues.
- `app_core::component_protocol` component lifecycle, update, layout and render contract.
- `app_core::event_protocol` lifecycle and input events, with generic product app events.
- `app_core::framework_manifest` the single reuse entry point for framework identity, native platform/toolkit coverage, host surfaces, shared protocols and AI capability summary.
- `app_core::native_hosts` native window/control host contracts for Windows and macOS backends, including native geometry, display scale, repaint and pointer lifecycle.
- `app_core::host_protocol` clipboard, menu, dialog, IME, caret, shell, paste-target and platform service host contracts.
- `app_core::render_protocol` semantic style, text layout and renderer protocols.
- `app_core::control_protocol` native control specs, mappers and control host contracts.
- `app_core::layout_protocol` geometry, DPI compensation and shared layout protocols.
- `app_core::main_commands` main-window command ids, shortcut routing, menu intents and tray plans.
- `app_core::main_window_protocol` main-window visibility, positioning, hotkey and search behavior plans.
- `app_core::native_adapter_manifest` shared native adapter manifest metadata for Windows, macOS and Linux backend discovery.
- `app_core::settings_protocol` shared settings dimensions, DPI helpers, control roles, commands, action routes, executor contract and prompt models.
- `app_core::timer_protocol` platform-neutral main/settings timer task mapping.
- `app_core::main_window` state, hit testing, selection, menu and render plans.
- `app_core::product_adapter` product events plus AI capability catalog descriptors for LLMs, skills and product-specific adapters.
- Shared settings models, page layout plans and component descriptions.
- Renderer/text layout protocols, not concrete GDI, AppKit, UIKit or Compose drawing.
- Tests that prove the shared layer does not import platform APIs.

Application-specific layers should stay outside ZSUI:

- ZSClip database schemas and clipboard item storage.
- LAN/WebDAV/WPS/mobile sync business logic.
- Product strings that only make sense for clipboard history.
- ZSClip `ApplicationEvent`, `MainAsyncEvent` and async payloads in `app_core::product_adapter`.
- AI execution details such as model/provider clients, skill registries, prompts, tool permissions and product data access; ZSUI should only carry semantic AI capability metadata and invocation requests.
- ZSClip settings action execution in `app/settings_actions.rs` plus the sync,
  group and platform action adapters.
- Windows `HWND`, GDI, Win32 messages, AppKit objects, Android contexts or iOS shortcut code.

The framework-level reuse entry is `zsui_framework_manifest()`. It returns the
ZSUI identity, API version, framework layer rules, supported native
platforms/toolkits, native backend descriptors, required host surfaces, shared
non-host protocols, the product adapter integration contract and the product AI
integration summary.
Reusable applications can read this first, then attach their own product adapter
and choose a native backend manifest for Windows, macOS or Linux. The helper
functions `native_ui_backend_for_platform()` and `native_ui_backend_for_toolkit()`
resolve those backend descriptors without platform-specific conditionals.
`native_ui_backend_for_current_target()` resolves the descriptor for the current
Rust compilation target.
`zsui_framework_layers()` and `zsui_framework_boundary_rules()` make the UI
framework boundary machine-readable. The layers are `core_contracts`,
`layout_and_render_plans`, `adapter_boundary`, `native_host` and
`product_adapter`. The rules describe which modules may own stable contracts,
layout/render/action plans, adapter binding names, native widgets/event loops
and ZSClip product behavior. AppKit and GTK work should follow these rules:
new right-click, group, VV and settings UI wiring belongs in native hosts or
adapter bindings, while row command semantics, settings persistence, sync and
AI providers stay in the product adapter.
`zsui_native_feature_parity_statuses()` adds a machine-readable progress view
for the macOS/Linux feature ports. It intentionally separates code-level
readiness from target smoke evidence and system completion for main-window
rows, search, row menus, right-click edit/save, group management, VV popup,
VV paste, clipboard payloads, status/menu, shell open, file picker, settings
pages and window/paste-target identity. A feature with `code_level_ready = true`
is callable through shared Rust contracts and local tests; it is still not
complete until target AppKit/GTK smoke artifacts exist and
`system_complete = true`.
`native_ui_backend_capability_matrix()` exposes the required adapter capability
set for every backend, and `zsui_reuse_readiness_report()` flattens platform
names, ready/scaffold status, adapter capability names, product adapter
surfaces and AI provider families into a single machine-readable summary.
`zsui_reuse_bootstrap_plan(platform)` combines those pieces for one target
platform: backend module, adapter boundary, readiness status, native adapter
capabilities, product adapter surfaces/tasks and AI provider/executor
boundaries. It also carries native runtime gate names, missing gate names and
the next gate to implement, so a single-platform port can plan AppKit/SwiftUI
or GTK/libadwaita runtime work without loading the full agent context. Each
runtime gate also maps to required adapter capabilities; the AI action
presentation gate additionally names the product adapter tasks and AI executor
boundaries needed for LLMs, skills and product-specific tools.
`native_ui_adapter_parity_report()` compares selected adapter reuse packages
across platforms, including binding counts, shared execution-plan counts,
ready/scaffold split and adapter boundary names.
`zsui_adapter_reuse_package_gate_binding_summaries()` checks those same reuse
packages against their runtime gate binding plans, reporting per-gate binding
counts, completion percent, next gate and whether every required gate binding
is present in the selected adapter binding plan.
`zsui_adapter_reuse_package_porting_work_items()` expands incomplete runtime
gates into concrete porting work items, including platform/toolkit/status,
adapter module path, gate name, required adapter capabilities, platform binding
names, product adapter tasks and AI executor boundaries.
`zsui_reuse_readiness_report_with_adapter_parity()` attaches that comparison to
the general readiness report when adapter packages are available.
`zsui_agent_context()` and `zsui_agent_context_with_adapter_parity()` provide
AI/agent-readable summaries that combine framework identity, boundary rules,
native feature parity, readiness and per-platform bootstrap summaries plus
stable AI route strings for provider, executor boundary, action, surface,
contexts and result.
When adapter reuse packages are supplied, the agent context also includes the
same incomplete gate porting work items, letting AI agents and skills read
macOS/Linux native runtime tasks from the single agent entry point.
The same context exposes reusable application feature statuses for a generic
Rust UI program: native app entry, window surfaces, control mapping,
renderer/text layout, system services, settings surfaces and AI action
surfaces. These statuses intentionally describe framework-level Rust UI
language and native bindings rather than ZSClip clipboard-history behavior, so
another tool can reuse the UI layer and provide its own product adapter.
`zsui_reusable_app_blueprint()` packages that generic view as the recommended
entry point for a new Rust tool: it names the ZSUI Rust UI contract, supported
native platforms, reusable feature names, product adapter surfaces/tasks, AI
executor boundaries, native runtime driver operations, product function flows
and per-platform feature readiness in one structure. Function flows cover app
bootstrap, state projection, user
commands, settings sync, async events and AI actions. The execution pipeline
then orders UI intent intake, product state projection, command execution,
async event bridging, AI action routing and UI update projection, so a product
keeps both domain logic and behavior in one Rust adapter while the selected
native host translates windows, controls, menus, dialogs, rendering and desktop
services for Windows, macOS or Linux.
`NativeRuntimeDriver` is the platform-side runtime entry trait. It starts the
native runtime, dispatches platform-neutral UI commands, polls application
events and requests shutdown, while lower-level hosts still own windows,
status items, controls and renderers.
The Windows adapter exposes that same driver shape, and the macOS/Linux
scaffold application models implement the driver path, so new native hosts can
exercise one runtime contract before the real AppKit or GTK event loop is
wired.
`ProductAdapterHost` is the concrete Rust trait a reusable product implements:
identity, state projection, command execution, settings binding, async event
bridging, AI catalog publication and AI plan execution. Native hosts should
call this trait through framework plans instead of embedding product behavior
inside Win32, AppKit or GTK code.
`ZsuiReusableRuntimeHarness` connects those two sides into the reusable runtime
path: start native runtime, dispatch UI command, bridge application event,
execute AI invocation and request shutdown.
ZSClip now has its own top-level `ZsclipProductAdapter` as the first product
adapter implementation: clipboard row commands, tray commands, async sync
events and AI execution plans can be routed through `ProductAdapterHost` before
platform hosts translate the UI. Its `zsclip_product_adapter_manifest()`
exposes the product command routes, event routes and AI provider/capability ids
that LLMs, skills and platform hosts can inspect without reading Windows code.
The same context includes native runtime gates for each platform: event loop,
window surfaces, control mapping, renderer, clipboard services, dialog
services, settings surfaces and AI action presentation. Windows currently has
no missing runtime gates; macOS and Linux expose those gates as the remaining
work needed to graduate from adapter scaffold to native runtime.
Runtime gates carry the same capability plans as bootstrap plans, so agents can
explain which native adapter capability or AI/product adapter boundary a gate
depends on. Bootstrap plans and agent runtime gates also include a completion
report with total, completed and missing gate counts, completion percent, the
missing gate names and the next gate to implement.
Each gate also carries a platform binding plan that translates required
capabilities into concrete adapter binding names such as `ns_window_pair`,
`gtk_snapshot_renderer` or `shared_main_execution_plan_bridge`.
They also expose ordered integration steps so tools can first select and verify
native adapters, then implement product adapter surfaces/tasks, then connect the
LLM executor, skill registry and product-specific AI tools.
Backend descriptors also carry an implementation status: Windows is currently
`native_host_integrated`, while macOS and Linux are `native_host_first_pass`
because they have real AppKit/SwiftUI and GTK/libadwaita host entry points and
binding coverage but still need target smoke artifacts before being treated as
runtime-ready. Each platform adapter manifest repeats that status so tools can
read either the framework catalog or the selected adapter boundary. Status
helpers such as `is_native_runtime_ready()`, `is_first_pass_native_host()` and
`is_scaffold()` let reuse tools avoid comparing status strings directly.

The current Windows main-surface executors live in `src/app/main_renderer.rs`
and `src/app/main_input.rs`. They consume shared render, pointer and shortcut
plans and map them to GDI/Win32 side effects; they are native host
implementations, not reusable ZSUI core.

The current Windows Win32/GDI adapter boundary lives in
`src/windows_win32_adapter.rs`. It names the Win32/GDI adapter bindings and
settings control roles that the current Windows implementation already spreads
across dedicated native host modules, giving Windows the same top-level native
adapter boundary shape as the macOS AppKit and Linux GTK boundaries. It also
exports a shared `NativeUiAdapterManifest` so product code can discover the
Windows backend without reading Win32-specific types, and exposes the same
reuse bootstrap plan as the framework manifest. It also exposes an adapter
binding plan with stable Win32/GDI binding names for tooling and AI agents.
The adapter reuse package combines manifest, bootstrap plan and binding plan
for one-shot discovery.

The current macOS AppKit/SwiftUI adapter boundary lives in
`src/macos_appkit_adapter.rs`. It maps the macOS host summary to named
AppKit/SwiftUI adapter bindings and widget roles, so the current recording
scaffold in `src/macos_app.rs` can be replaced by native `NSApplication`,
`NSWindow`, `NSStatusItem`, `NSMenu`, `NSAlert`, `NSPasteboard`,
Accessibility and SwiftUI/AppKit control implementations without changing the
product program or ZSUI contracts. It also exports the same shared
`NativeUiAdapterManifest` and reuse bootstrap plan shape used by the Windows
and Linux adapters, plus an adapter binding plan with stable AppKit/SwiftUI
binding names. The adapter reuse package combines manifest, bootstrap plan and
binding plan for one-shot discovery.

The current Linux GTK/libadwaita adapter boundary lives in
`src/linux_gtk_adapter.rs`. It maps the Linux host summary to named GTK, GDK,
Pango, GIO, portal, AT-SPI and libadwaita adapter bindings, including the
shared main execution plan bridge and widget roles, so the recording scaffold
in `src/linux_app.rs` can be replaced by native GTK/libadwaita implementations
without changing the product program or ZSUI contracts. It also exports the
same shared `NativeUiAdapterManifest` and reuse bootstrap plan shape used by
the Windows and macOS adapters, plus an adapter binding plan with stable
GTK/libadwaita binding names. The adapter reuse package combines manifest,
bootstrap plan and binding plan for one-shot discovery.

The current Windows row command executor lives in
`src/app/main_row_commands.rs`. It consumes shared row action/data/dialog plans
and maps them to ZSClip database, clipboard, shell, OCR/translation, mail merge
and dialog side effects. Other native hosts should reuse the shared row plans
and provide their own product adapter side effects.

The current product AI catalog lives in `src/app_core/product_adapter.rs`.
It also names the product adapter integration contract: product identity,
product state model, product command executor, settings model, async event
bridge and AI capability catalog. The AI catalog describes LLM, skills and
product-adapter capabilities by id, provider, action kind, UI surface, required
context and result kind. This gives future Windows, macOS and Linux native hosts
a shared way to expose AI actions while keeping model clients, skill
registries, prompt assembly, permissions and product data access outside the
reusable ZSUI layer.
`product_adapter_reuse_checklist()` turns that contract into a concrete
implementation checklist for reusable products: provide identity, project
product state, execute product commands, bind settings, bridge async events,
publish the AI catalog, then connect LLM executors, skill registries and
product-specific AI tools.
The same module exposes `product_ai_integration_manifest()`, which summarizes
capability counts, provider families, action kinds, execution routes, UI
surfaces, required contexts and result kinds from the catalog so native hosts
and AI agents can understand the available AI surface before choosing an
action. Provider families now map to explicit executor boundaries:
`llm_executor`, `skill_registry` and `product_adapter_tools`. Provider,
executor boundary, surface, action, context and result values expose stable
name helpers so external tools do not need to parse Rust enum debug text.
Execution plans and routes expose the same stable name helpers for provider,
executor boundary, executor integration task, action, surface, required
contexts and result.
The row menu now marks AI-capable actions with `ProductAiActionKind`, and the
catalog can be queried by UI surface, action kind or context such as selected
text/image/settings profile.
The settings plugin model exposes a small AI panel presentation from the same
catalog for provider configuration, so native preferences UIs can discover AI
configuration actions without hard-coding provider clients.
The main row model can also build an AI capability plan for the current row or
selection. That plan carries selected text/image/file contexts, target item ids
and the catalog capabilities available for that context, which lets AI agents
understand what can be done before any platform-specific menu is opened.
Once the user or agent chooses a capability, `main_row_ai_invocation()` turns
the selected capability, prompt text and target item ids into a
`ProductAiInvocation`. Execution still belongs to an LLM, skill registry or
product adapter implementation.
`product_ai_execution_plan()` then resolves that invocation back to the catalog
descriptor and identifies the provider family, action kind, source UI surface,
required input contexts and result kind for the executor.

The current Windows row and group popup menu presenter lives in
`src/app/main_popup_menus.rs`. It converts shared row/group menu plans into
`NativePopupMenuEntry` values and delegates native presentation to
`NativePopupMenuHost`; macOS and Linux hosts should render those same entries
with their own native menu systems.

The current Windows VV popup presenter lives in `src/app/vv_popup.rs`. It
owns the Windows floating popup window, native caret/IME anchoring, group menu
presentation and shared VV render/select-plan execution for the Windows host;
the reusable VV layout, hit-test, render and select behavior remains in
`app_core::main_window`.

The current Windows VV keyboard hook and target/backspace adapter lives in
`src/app/vv_hook.rs`. It owns the Windows low-level keyboard hook, target
identity checks and app-compatibility backspace policy; macOS and Linux hosts
should provide their own native trigger strategy and then consume the shared VV
popup/select plans instead of copying this Win32 hook.

The current Windows settings input boundary is split across
`src/app/settings_input.rs`, `src/app/settings_pointer_input.rs`,
`src/app/settings_keyboard_input.rs` and `src/app/settings_window_events.rs`.
The dispatcher consumes shared settings pointer, navigation, scroll and
action-dispatch plans while pointer, keyboard and window/DPI/theme lifecycle
effects stay in dedicated Windows adapters.

The current Windows settings dropdown executor lives in
`src/app/settings_dropdown.rs`, with popup lifecycle helpers in
`src/app/settings_dropdown_host.rs` and plugin-provider option building in
`src/app/settings_dropdown_plugin.rs`. It builds native dropdown requests from
shared settings option models and sends creation/destruction through
`NativeSettingsDropdownHost`; plugin-provider option labels and selected-index
rules stay in their own adapter and remain portable for macOS and future Linux
hosts.

The current Windows settings command executors live in
`src/app/settings_command_queue.rs`, `src/app/settings_timer_tasks.rs` and
`src/app/settings_control_selection.rs`, with domain-specific dropdown
selection application in `src/app/settings_control_selection_general.rs`,
`src/app/settings_control_selection_cloud.rs`,
`src/app/settings_control_selection_hotkey.rs`,
`src/app/settings_control_selection_plugin.rs` and
`src/app/settings_control_selection_group.rs`. They split shared `Command`
drain, settings timer tasks and dropdown selection application for the Windows
product adapter; macOS and Linux hosts should consume the same `app_core`
command, timer and settings action protocols with their own native side
effects.

The current Windows settings-window boundary is split across
`src/app/settings_window.rs`, `src/app/settings_window_create.rs`,
`src/app/settings_window_lifecycle.rs`, `src/app/settings_window_destroy.rs`,
`src/app/settings_window_metrics.rs`, `src/app/settings_window_layout.rs`,
`src/app/settings_window_colors.rs`,
`src/app/settings_window_surface_controls.rs`,
`src/app/settings_window_owner_draw.rs` and `src/app/settings_window_paint.rs`.
The proc module owns Win32 message routing; the create module owns initial
native state/control hydration; the lifecycle module owns open/focus/destroy
requests, pointer capture, repaint forwarding and Cloud refresh; the destroy
module owns native cleanup after destruction; the metrics module owns page
content height, scroll slots, font/control refresh and visible control sync; the
layout module owns DPI/work-area geometry policy; the color module owns
CTLCOLOR surface/edit/list roles; the surface-control module owns semantic
surface-control classification by settings domain; the owner-draw module owns
per-control item buffering; and the paint module owns buffered settings-window
chrome/content/scrollbar entry points. macOS and Linux should reuse the same
settings plans and host contract with native AppKit/SwiftUI and GTK4/libadwaita
windows instead of copying these Windows adapters.

The current Windows plugin settings section adapter lives in
`src/app/settings_plugin_sections.rs`. It owns dynamic plugin/provider cards,
provider-row layout and plugin-page relayout for the Windows settings host
while consuming shared settings section models. macOS and Linux should keep
equivalent native settings section sessions instead of copying the Win32
control movement code.

The current Windows multi-sync settings section adapter lives in
`src/app/settings_multi_sync_sections.rs`. It owns WebDAV/LAN dynamic section
layout, cloud-page handle reset, card refresh and rebuild orchestration for the
Windows settings host. Other native hosts should reuse the same sync-section
model and replace only the native control/page rebuild mechanics.

The current Windows group settings section adapter lives in
`src/app/settings_group_sections.rs`. It owns group cache selection, VV
source/group display, group list refresh and ordering for the Windows settings
host, while `src/app/settings_group_page.rs` owns the Group page control
construction. Other native hosts should reuse the same group session semantics
and replace only the native list, segmented-control and action-button
presentation.

The current Windows General settings page adapter lives in
`src/app/settings_general_page.rs`. It owns the General page entry and section
dispatch for the Windows settings host; `src/app/settings_general_page_startup.rs`
owns startup, retention, behavior and paste-sound controls, while
`src/app/settings_general_page_window.rs` owns skip-window, position and config
action controls. Other native hosts should reuse the same settings semantics
and replace only native preference control presentation.

The current Windows Hotkey settings page adapter lives in
`src/app/settings_hotkey_page.rs`. It owns the Hotkey page entry and section
dispatch for the Windows settings host; `src/app/settings_hotkey_page_shortcuts.rs`
owns main/plain shortcut controls, while `src/app/settings_hotkey_page_system.rs`
owns Win+V system-history actions and explanatory notes. Other native hosts
should reuse the same shortcut state and replace only native shortcut recorder
presentation.

The current Windows Plugin settings page adapter lives in
`src/app/settings_plugin_page.rs`. It owns the Plugin page entry and section
dispatch for the Windows settings host; `src/app/settings_plugin_page_search.rs`
owns quick-search controls, `src/app/settings_plugin_page_ocr_translate.rs`
owns OCR/translation controls, and `src/app/settings_plugin_page_tools.rs`
owns AI/mail-merge/WPS/QR tool controls. Other native hosts should reuse the
same plugin/provider settings semantics and replace only native preference
controls.

The current Windows About settings page adapter lives in
`src/app/settings_about_page.rs`. It owns the About page entry and section
dispatch for the Windows settings host; `src/app/settings_about_page_metadata.rs`
owns version, summary and source-link controls, `src/app/settings_about_page_update.rs`
owns update status/action controls, and `src/app/settings_about_page_data.rs`
owns data-directory display. Other native hosts should render the same metadata
with their own native preference/about controls.

The current Windows Cloud/LAN settings page adapter lives in
`src/app/settings_cloud_page.rs`. It owns the Cloud page mode switch for the
Windows settings host and delegates LAN device/pair list refresh plus selected
row resolution to `src/app/settings_cloud_page_lan_devices.rs`. WebDAV page
construction lives in `src/app/settings_cloud_page_webdav.rs`, and LAN page
construction lives in `src/app/settings_cloud_page_lan.rs`. Other native hosts
should reuse the same sync state and replace only native list/table, form and
page-control presentation.

The current Windows settings sync action adapter lives in
`src/app/settings_sync_actions.rs` and only dispatches action domains. WebDAV
actions live in `src/app/settings_sync_actions_webdav.rs`; LAN discovery,
pairing, QR-link copy and setup-page actions live in
`src/app/settings_sync_actions_lan.rs`. macOS and Linux should keep the same
WebDAV/LAN action-domain split while replacing only native scheduling,
clipboard, shell-open and product side effects.

The current Windows settings owner-draw adapter lives in
`src/app/settings_owner_draw.rs`. It owns hover checks, LAN QR rendering and
button/toggle drawing for the Windows settings host. macOS and Linux should map
those semantics to native controls and platform renderers instead of copying
Windows owner-draw code.

The current Windows settings page builder adapter lives in
`src/app/settings_page_builder.rs`. It owns the builder identity, control
registration entry and page-section helper. Raw label/button/dropdown/input/
listbox/toggle helpers live in `src/app/settings_raw_controls.rs`. Form field
rows live in `src/app/settings_form_fields.rs`, and form action rows, QR actions
and owner-draw action ownership live in `src/app/settings_form_actions.rs`.
Windows settings control registration and scrollable-page resolution live in
`src/app/settings_control_registry.rs`, and native control factory wrappers live
in `src/app/settings_control_factory.rs`, so macOS and Linux can keep the same
Rust page-builder/session semantics while mapping control registration,
creation, field rows and action composition to AppKit/SwiftUI or GTK/libadwaita
native controls.

The current Windows settings page navigation adapter lives in
`src/app/settings_page_navigation.rs`. It owns settings page switching, scroll
state updates, scrollbar reveal and child-control repositioning for the Windows
settings host. macOS and Linux should reuse the same Rust navigation semantics
while mapping scroll containers and control visibility to native widgets.

The current Windows settings page ensure adapter lives in
`src/app/settings_page_ensure.rs`. It owns lazy settings page construction
routing for the Windows settings host. macOS and Linux should keep the same
lazy-page/session semantics while mapping page construction to native
preference containers.

The current Windows settings page sync adapter lives in
`src/app/settings_page_sync.rs`, with the Cloud sync entry in
`src/app/settings_page_sync_cloud.rs`, WebDAV control synchronization in
`src/app/settings_page_sync_cloud_webdav.rs`, LAN control/list synchronization
in `src/app/settings_page_sync_cloud_lan.rs` and plugin-provider
synchronization in `src/app/settings_page_sync_plugin.rs`. The main sync
adapter owns page text/enabled synchronization, position-field enablement and
multi-sync mode helpers for the Windows settings host; the Cloud sync entry
owns transport summary text and delegates transport-specific updates; the
WebDAV sync adapter owns WebDAV field/status enabled state; the LAN sync
adapter owns LAN field enabled state, LAN trusted summary text and LAN list
refresh; the plugin sync adapter owns quick-search, OCR, translate and tool
toggle control state. macOS and Linux should reuse the same Rust page
synchronization semantics while mapping updates to native controls.

The current Windows settings toggle-state adapter lives in
`src/app/settings_toggle_state.rs`. It owns settings toggle reads and state
flips for the Windows settings host and delegates concrete General, Cloud,
Hotkey, Plugin and Group toggles to domain modules. macOS and Linux should
reuse the same Rust toggle semantics while mapping them to native switches or
checkboxes.

The current Windows settings host-helper adapter lives in
`src/app/settings_host_helpers.rs`. It owns small Windows control text,
visibility/enabled, invalidation and theme brush-resource helpers. macOS and
Linux should map those same host-helper semantics to native control updates,
repaint requests and platform theme resources instead of copying Win32/GDI
calls.

The current Windows settings app apply/collect adapters live in
`src/app/settings_app_apply.rs` and `src/app/settings_app_collect.rs`. They own
settings state-to-native-control apply and native-control-to-draft collect
wiring, while delegating post-save product side effects to the app-effects
adapter.

The current Windows settings app-effects adapter lives in
`src/app/settings_app_effects.rs`. It owns settings persistence, autostart,
hotkey/tray refresh, cloud/LAN refresh, data reload and peer-window
synchronization. Other products should provide their own product side effects
while reusing the shared settings/session semantics.
The effect pipeline is split across `src/app/settings_app_effect_state.rs`,
`src/app/settings_app_integration_effects.rs`,
`src/app/settings_app_data_effects.rs` and
`src/app/settings_app_window_effects.rs`, separating baseline capture,
desktop integrations, product data refresh and native window refresh.

The old Windows `src/app/hosts.rs` aggregation module is retired. Windows
native behavior is now split across named adapters, while the app prelude pulls
the small remaining shared settings-model imports directly.

The current Windows clipboard capture executor lives in
`src/app/main_clipboard_capture.rs`. It owns Windows capture retries, source
app/foreground identity filtering, file/text/image capture routing, screen-clip
normalization and guarded legacy bitmap decoding while consuming
`WindowsClipboardHost`; none of that capture glue remains in `app.rs`. macOS
and Linux should provide their own native pasteboard/clipboard monitor adapters
against the same product capture semantics.

The current Windows main-window lifecycle and platform binding executors live
in `src/app/main_window.rs` and `src/app/main_platform_bindings.rs`. The native
main/quick window host lives in `src/app/main_window_host.rs` and owns Win32
class creation, appearance, bounds, DPI/layout, repaint, hide/show,
activation, pointer capture, native drag and destroy operations while consuming
the shared `NativeMainWindowHost` contract. macOS and Linux should map those
same semantic operations to AppKit/SwiftUI and GTK/libadwaita or Wayland/X11
services.

Windows edge auto-hide behavior now lives in
`src/app/main_edge_auto_hide.rs`. It owns screen-edge detection, docked bounds,
hidden-position calculation, animation ticks, hot-zone restore and move
reconciliation for the Windows host. macOS and Linux should keep the same Rust
session shape for edge docking while replacing only the native monitor,
animation and activation services.

Windows paste-target discovery now lives in
`src/app/main_paste_target_discovery.rs`. It owns skip-class parsing,
top-level-window enumeration, viability checks and next-target selection for the
Windows paste adapter. macOS and Linux should replace only the native
foreground/window discovery mechanism while keeping the same paste preparation
and paste-target host contracts.

Windows low-level input behavior now lives in
`src/app/main_low_level_input.rs`. It owns protected-scope hit testing,
outside-hide timer scheduling, edge-auto-hide timer scheduling and the Quick
Escape low-level keyboard hook for the Windows host. macOS and Linux should map
the same session intent to AppKit event monitors, GLib/GDK event controllers or
platform shortcut services instead of copying Win32 hooks.

Windows main hover preview behavior now lives in
`src/app/main_hover_preview.rs`. It owns hover-preview blocked-hit checks,
preview refresh, mouse-leave cleanup and mouse-leave tracking rearming for the
Windows main window. macOS and Linux should map this to native tracking areas,
popovers or tooltip surfaces while keeping row hover and preview state in Rust.

Windows startup integration recovery now lives in
`src/app/main_startup_integrations.rs`. It owns tray icon resync, hotkey retry,
plain-paste hotkey retry, VV monitor retry, clipboard-listener retry and update
state notification for the Windows host. macOS and Linux should map the same
intent to native status items, shortcut services and pasteboard/clipboard
monitors.

Windows main/quick window refresh now lives in
`src/app/main_window_refresh.rs`. It owns settings reload, database reload,
settings-window refresh and peer-window synchronization for the Windows host.
macOS and Linux should keep these as application refresh sessions while
replacing only the native window handles and repaint calls.

Windows main/quick window registry now lives in
`src/app/main_window_registry.rs`. It owns main/quick host handle registration,
app-window checks, state-pointer lookup and cross-host clipboard ignore guards.
macOS and Linux should keep equivalent native window registries in their own
adapter layers instead of depending on Windows handle globals.

Windows main hover cleanup now lives with `src/app/main_hover_preview.rs`.
It owns hover-clear state reset and no-activate row hit testing alongside
hover preview refresh. macOS and Linux should map this to native row
hit-testing and tracking-area cleanup without copying Windows invalidation
helpers.

The current Windows main event executor lives in `src/app/main_events.rs`. It
maps shared commands, timers, `UiEvent`, ZSClip `ApplicationEvent` and
`MainAsyncEvent` values to Windows product side effects. macOS and Linux should
consume the same semantic events through their own product event adapters
instead of copying the Windows window procedure.

Windows cloud-sync queue/completion execution now lives in
`src/app/main_cloud_sync.rs`, while LAN envelope conversion, incoming-item
application and optional clipboard mirroring live in
`src/app/main_lan_sync.rs`. The application root keeps state ownership but no
longer contains transport encoders or Windows completion glue.

The Windows process entry, main window procedure and native startup wiring now
live in `src/app/main_entry.rs`. It creates the native main/search windows,
mounts lifecycle state and installs Windows integrations while leaving
`src/app.rs` as a module root and public re-export surface. Other platforms
should provide their own entry adapter and consume the same startup window
request and lifecycle semantics.

Windows main runtime state ownership now lives in `src/app/state.rs`, including
the window role, native icon resources, `AppState`, payload/thumbnail caches and
shared lifecycle/command state. `app.rs` may implement product behavior over
that state, but no longer defines the native window state container.

Clip payload preview, dedupe signatures, QR image generation, image
materialization and thumbnail preparation now live in `src/app/data.rs`. These
are product data semantics consumed by native hosts; they are not part of the
Windows window procedure.

Main runtime behavior over `AppState` now lives in `src/app/state_runtime.rs`.
This includes payload cache access, selected item resolution, scroll anchors,
transient clipboard guards and dedupe insert behavior. `app.rs` remains the
module root, not the owner of reusable list/runtime semantics.

Windows settings window state now lives in `src/app/settings_state.rs`. Direct
field access is still scoped to the `app` adapter layer during migration, but
the application root no longer owns the settings window state definition.
`SettingsWndState::new` owns the default Rust-side settings state and cache
initialization; Windows create adapters hydrate native resources around that
state instead of holding the full state literal inline.

Windows platform helpers and main-view helpers now live in
`src/app/platform_helpers.rs` and `src/app/main_view_helpers.rs`. The
application root no longer owns free helper functions for dialogs, clipboard
background writes, theme color mapping, empty state, title-button visibility or
main-row hit support.

Windows app adapter imports now live in `src/app/prelude.rs`, while timer ids,
window class names, clipboard format ids and other platform constants live in
`src/app/constants.rs`. Submodules consume the prelude explicitly, so `app.rs`
does not act as a hidden global import bucket.

Windows main search native control work now lives in `src/app/main_search_host.rs`.
It owns the native EDIT creation, font resource lifecycle, visibility, text and
focus bridge for the shared `NativeMainSearchControlHost` contract.

Windows transient floating-window native work now lives in
`src/app/transient_window_host.rs`. It owns no-activate popup class creation,
presentation, hide and destroy operations for the shared
`NativeTransientWindowHost` contract while VV popup behavior stays in the
shared layout/render/select plans.

The current row tool executor lives in `src/app/main_row_tools.rs`. It owns
AI text cleanup, OCR/translation jobs, OCR image input preparation, file
materialization for drag export and paste passthrough cleanup. This keeps
future LLM/skills behavior close to row actions instead of hiding it in
Windows message handling.

Quick-search URL preparation and native launch now live with the Windows row
command adapter in `src/app/main_row_commands.rs`, while VV target process
identity lives in `src/app/vv_hook.rs`. This keeps product actions and native
trigger compatibility out of the application root.

## Rust Foundation Shape

ZSUI should evolve like a Rust UI foundation, not a one-off UI wrapper:

- `CoreContracts` define stable ids, events, commands and host traits.
- `LayoutModel` owns pure geometry, hit testing and state transitions.
- `RenderProtocol` emits semantic drawing/text/icon commands.
- `NativeHost` translates semantic operations into platform APIs.
- `ProductAdapter` binds ZSClip storage, sync, clipboard history and settings to ZSUI.

Only the first four layers are reusable foundation. Product adapters may depend
on ZSUI, but ZSUI should not depend on product storage or Windows implementation
details.

## Reuse Goal

Another Rust program should eventually be able to reuse ZSUI by providing:

- A product adapter that maps its own state, commands and strings into ZSUI models.
- A Windows native host for Win32/WinUI-style windows, controls and drawing.
- A macOS native host for AppKit/SwiftUI windows, controls and drawing.
- A Linux native host for GTK4/libadwaita windows, controls and drawing.

The reusable layer should not require the program to copy ZSClip clipboard
history, sync, WPS or Windows message handling code.

The current Linux code-level scaffold lives in `src/linux_app.rs`. It already
owns a lifecycle/command application model, startup plan, clipboard host,
style resolver, native control mapper, text layout host, status item host, popup
menu host, transient window host, IME host, dialog host, shell-open host, window identity host,
paste-target host, text-caret host, file dialog host, text input dialog host,
edit text dialog host, mail-merge window host, renderer, main-window host,
main-search host, settings-window host, settings-control host and
settings-dropdown host recording adapters for GTK4/libadwaita, while the actual
native event loop, widget presentation and platform service bridges remain
future adapter work.
`src/linux_gtk_adapter.rs` now describes the GTK/libadwaita adapter boundary
that will replace recording hosts with native GTK, GDK, Pango, GIO, portal and
libadwaita implementations without changing the product program or ZSUI
contracts.

The product program should own features and data; ZSUI translates those product
capabilities into platform-neutral UI state, commands and host requests. Each
platform host then translates the requests into native windows, controls, menus,
clipboard operations, dialogs, drawing and desktop services.

AI features should attach through the product adapter, not through platform
hosts. A reusable app can expose LLM actions, skills and product-specific AI
tools as `ProductAiCapability` values and invoke them through
`ProductAiInvocation`; Windows, macOS and Linux hosts should only present the
resulting commands, menus, controls and status, keeping provider details out of
native UI code.

## Porting Shape

A new platform should implement hosts in this order:

1. Status/menu host.
2. Main window host.
3. Main search control host.
4. Renderer/text layout host.
5. Popup/dialog/file/shell hosts.
6. Settings window/control/dropdown hosts.
7. Clipboard and drag/drop hosts.

The host should translate ZSUI operations into native UI APIs. It should not copy
Windows message handling, class names or drawing helpers.

## Naming

- Framework name: `ZSUI`
- Current public API source of truth: standalone `zsui` crate (`E:\rust\zsui/src/`)
- Framework manifest source: `src/app_core/zsui.rs`
- UI surface protocol source: `src/app_core/ui_surface_protocol.rs`
- Command protocol source: `src/app_core/command_protocol.rs`
- Component protocol source: `src/app_core/component_protocol.rs`
- Event protocol source: `src/app_core/event_protocol.rs`
- Native host contract source: `src/app_core/native_hosts.rs`
- Platform service host contract source: `src/app_core/host_protocol.rs`
- ZSClip product adapter source: `src/zsclip_product_adapter.rs`
- Render protocol source: `src/app_core/render_protocol.rs`
- Windows main paste adapter source: `src/app/main_paste.rs`
- Windows main search adapter source: `src/app/main_search.rs`
- Windows clip payload data source: `src/app/data.rs`
- Windows runtime state behavior source: `src/app/state_runtime.rs`
- Windows settings state source: `src/app/settings_state.rs`
- Windows plugin settings section source: `src/app/settings_plugin_sections.rs`
- Windows plugin settings controls source: `src/app/settings_plugin_sections_controls.rs`
- Windows plugin settings layout source: `src/app/settings_plugin_sections_layout.rs`
- Windows plugin settings providers source: `src/app/settings_plugin_sections_providers.rs`
- Windows plugin settings tools source: `src/app/settings_plugin_sections_tools.rs`
- Windows multi-sync settings section source: `src/app/settings_multi_sync_sections.rs`
- Windows group settings section source: `src/app/settings_group_sections.rs`
- Windows group settings cache source: `src/app/settings_group_sections_cache.rs`
- Windows group settings display source: `src/app/settings_group_sections_display.rs`
- Windows group settings list source: `src/app/settings_group_sections_list.rs`
- Windows group settings page source: `src/app/settings_group_page.rs`
- Windows General settings page source: `src/app/settings_general_page.rs`
- Windows General startup/behavior page source: `src/app/settings_general_page_startup.rs`
- Windows General window/position page source: `src/app/settings_general_page_window.rs`
- Windows Hotkey settings page source: `src/app/settings_hotkey_page.rs`
- Windows Hotkey shortcuts page source: `src/app/settings_hotkey_page_shortcuts.rs`
- Windows Hotkey system-actions page source: `src/app/settings_hotkey_page_system.rs`
- Windows Plugin settings page source: `src/app/settings_plugin_page.rs`
- Windows Plugin quick-search page source: `src/app/settings_plugin_page_search.rs`
- Windows Plugin OCR/translate page source: `src/app/settings_plugin_page_ocr_translate.rs`
- Windows Plugin tools page source: `src/app/settings_plugin_page_tools.rs`
- Windows About settings page source: `src/app/settings_about_page.rs`
- Windows About metadata page source: `src/app/settings_about_page_metadata.rs`
- Windows About update page source: `src/app/settings_about_page_update.rs`
- Windows About data page source: `src/app/settings_about_page_data.rs`
- Windows Cloud/LAN settings page source: `src/app/settings_cloud_page.rs`
- Windows Cloud/LAN device list source: `src/app/settings_cloud_page_lan_devices.rs`
- Windows Cloud/WebDAV settings page source: `src/app/settings_cloud_page_webdav.rs`
- Windows Cloud/LAN page construction source: `src/app/settings_cloud_page_lan.rs`
- Windows settings owner-draw source: `src/app/settings_owner_draw.rs`
- Windows settings page builder source: `src/app/settings_page_builder.rs`
- Windows settings raw controls source: `src/app/settings_raw_controls.rs`
- Windows settings form fields source: `src/app/settings_form_fields.rs`
- Windows settings form actions source: `src/app/settings_form_actions.rs`
- Windows settings control factory source: `src/app/settings_control_factory.rs`
- Windows settings control registry source: `src/app/settings_control_registry.rs`
- Windows settings page navigation source: `src/app/settings_page_navigation.rs`
- Windows settings page navigation controls source: `src/app/settings_page_navigation_controls.rs`
- Windows settings page navigation scroll source: `src/app/settings_page_navigation_scroll.rs`
- Windows settings page navigation switch source: `src/app/settings_page_navigation_switch.rs`
- Windows settings page ensure source: `src/app/settings_page_ensure.rs`
- Windows settings page sync source: `src/app/settings_page_sync.rs`
- Windows settings Cloud sync source: `src/app/settings_page_sync_cloud.rs`
- Windows settings Cloud WebDAV sync source: `src/app/settings_page_sync_cloud_webdav.rs`
- Windows settings Cloud LAN sync source: `src/app/settings_page_sync_cloud_lan.rs`
- Windows settings Plugin sync source: `src/app/settings_page_sync_plugin.rs`
- Windows settings toggle-state source: `src/app/settings_toggle_state.rs`
- Windows settings toggle General source: `src/app/settings_toggle_state_general.rs`
- Windows settings toggle Cloud source: `src/app/settings_toggle_state_cloud.rs`
- Windows settings toggle Hotkey source: `src/app/settings_toggle_state_hotkey.rs`
- Windows settings toggle Plugin source: `src/app/settings_toggle_state_plugin.rs`
- Windows settings toggle Group source: `src/app/settings_toggle_state_group.rs`
- Windows settings host-helper source: `src/app/settings_host_helpers.rs`
- Windows settings input dispatcher source: `src/app/settings_input.rs`
- Windows settings pointer input source: `src/app/settings_pointer_input.rs`
- Windows settings keyboard input source: `src/app/settings_keyboard_input.rs`
- Windows settings window events source: `src/app/settings_window_events.rs`
- Windows settings window proc source: `src/app/settings_window.rs`
- Windows settings window create source: `src/app/settings_window_create.rs`
- Windows settings window destroy cleanup source: `src/app/settings_window_destroy.rs`
- Windows settings window lifecycle facade source: `src/app/settings_window_lifecycle.rs`
- Windows settings window metrics source: `src/app/settings_window_metrics.rs`
- Windows settings window layout source: `src/app/settings_window_layout.rs`
- Windows settings window color source: `src/app/settings_window_colors.rs`
- Windows settings window surface-control source: `src/app/settings_window_surface_controls.rs`
- Windows settings window owner-draw source: `src/app/settings_window_owner_draw.rs`
- Windows settings window paint source: `src/app/settings_window_paint.rs`
- Windows settings owner-draw entry source: `src/app/settings_owner_draw.rs`
- Windows settings owner-draw QR source: `src/app/settings_owner_draw_qr.rs`
- Windows settings owner-draw link source: `src/app/settings_owner_draw_link.rs`
- Windows settings owner-draw role source: `src/app/settings_owner_draw_roles.rs`
- Windows settings action executor source: `src/app/settings_actions.rs`
- Windows settings sync actions source: `src/app/settings_sync_actions.rs`
- Windows settings group actions source: `src/app/settings_group_actions.rs`
- Windows settings platform actions source: `src/app/settings_platform_actions.rs`
- Windows settings platform Hotkey actions source: `src/app/settings_platform_actions_hotkey.rs`
- Windows settings platform General actions source: `src/app/settings_platform_actions_general.rs`
- Windows settings platform Plugin actions source: `src/app/settings_platform_actions_plugin.rs`
- Windows settings platform About actions source: `src/app/settings_platform_actions_about.rs`
- Windows settings platform System actions source: `src/app/settings_platform_actions_system.rs`
- Windows settings command queue source: `src/app/settings_command_queue.rs`
- Windows settings timer tasks source: `src/app/settings_timer_tasks.rs`
- Windows settings control selection source: `src/app/settings_control_selection.rs`
- Windows settings general selection source: `src/app/settings_control_selection_general.rs`
- Windows settings cloud selection source: `src/app/settings_control_selection_cloud.rs`
- Windows settings hotkey selection source: `src/app/settings_control_selection_hotkey.rs`
- Windows settings plugin selection source: `src/app/settings_control_selection_plugin.rs`
- Windows settings group selection source: `src/app/settings_control_selection_group.rs`
- Windows settings dropdown source: `src/app/settings_dropdown.rs`
- Windows settings general dropdown source: `src/app/settings_dropdown_general.rs`
- Windows settings cloud dropdown source: `src/app/settings_dropdown_cloud.rs`
- Windows settings hotkey dropdown source: `src/app/settings_dropdown_hotkey.rs`
- Windows settings group dropdown source: `src/app/settings_dropdown_group.rs`
- Windows settings dropdown host source: `src/app/settings_dropdown_host.rs`
- Windows settings dropdown plugin source: `src/app/settings_dropdown_plugin.rs`
- Windows settings app-effects source: `src/app/settings_app_effects.rs`
- Windows settings effect baseline source: `src/app/settings_app_effect_state.rs`
- Windows settings integration effects source: `src/app/settings_app_integration_effects.rs`
- Windows settings data effects source: `src/app/settings_app_data_effects.rs`
- Windows settings window effects source: `src/app/settings_app_window_effects.rs`
- Windows settings app apply source: `src/app/settings_app_apply.rs`
- Windows settings app collect source: `src/app/settings_app_collect.rs`
- Windows settings general collect source: `src/app/settings_app_collect_general.rs`
- Windows settings hotkey collect source: `src/app/settings_app_collect_hotkey.rs`
- Windows settings plugin collect source: `src/app/settings_app_collect_plugin.rs`
- Windows settings group collect source: `src/app/settings_app_collect_group.rs`
- Windows settings cloud collect source: `src/app/settings_app_collect_cloud.rs`
- Windows platform helper source: `src/app/platform_helpers.rs`
- Windows main-view helper source: `src/app/main_view_helpers.rs`
- Control protocol source: `src/app_core/control_protocol.rs`
- Layout protocol source: `src/app_core/layout_protocol.rs`
- Main command routing source: `src/app_core/main_commands.rs`
- Main window behavior protocol source: `src/app_core/main_window_protocol.rs`
- Settings protocol source: `src/app_core/settings_protocol.rs`
- Timer protocol source: `src/app_core/timer_protocol.rs`
- Porting contract: `docs/ui-host-porting.md`
- macOS scaffold plan: `docs/macos-ui.md`

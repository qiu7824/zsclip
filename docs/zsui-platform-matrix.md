# ZSUI Platform Matrix

This matrix describes the standalone `zsui` public API, the recording
`ZsuiHost` scaffold and the native main-window bridge used by current
Win32/AppKit/GTK startup paths. It does not remove or downgrade the existing
Windows ZSClip implementation. Windows still runs through the current
`src/app/*` and `src/platform/*` Win32/GDI path while the declarative host
adapter is being connected.

Status meanings:

- `Supported`: the `ZsuiHost` path can perform or fully record the operation.
- `Partial`: the platform has an existing native path or declaration model, but
  the new `ZsuiHost` adapter is not yet the production runtime.
- `Unsupported`: callers must detect this through `HostCapabilities` and use a
  fallback.

| Capability | Windows | macOS | Linux | Fallback rule |
| --- | --- | --- | --- | --- |
| Main window declaration | Partial | Partial | Partial | Keep app state alive and skip native presentation if unavailable. |
| Show/hide window | Partial | Partial | Partial | Treat as a no-op or queue a later presentation command. |
| Tray/status menu declaration | Partial | Partial | Partial | Use an in-window menu or command palette when the desktop does not expose a status area. |
| Menu items | Partial | Partial | Partial | Render as native menu items when available, otherwise expose the same `Command` list in UI. |
| Global hotkey | Partial | Unsupported | Unsupported | Let users configure app-local shortcuts or disable the command trigger. |
| Clipboard text | Supported | Partial | Partial | Use host clipboard when available; otherwise keep an app-local clipboard value. |
| Clipboard image | Partial | Partial | Partial | Fall back to text metadata or file export. |
| Clipboard files | Partial | Partial | Partial | Fall back to path text or app-local drag/export. |
| File picker | Partial | Partial | Partial | Show a text path field or ask the product layer for a known path. |
| Native dialogs | Partial | Partial | Partial | Use in-window notification or command result status. |
| Settings page declarations | Partial | Partial | Partial | Keep settings models in Rust and render with the active host. |
| Auto paste / paste target | Partial | Unsupported | Partial | Disable auto paste and keep explicit copy/paste commands. |

## Native Window Builder Path

Applications can declare a main window once:

```rust
Window::new("ZSClip")
    .size(900, 620)
    .resizable(true)
```

The startup adapters convert it with
`NativeMainWindowRequest::from_zsui_window_for_host`, which stores both the
effective native options and any `degraded_capabilities`.

| Window trait | Windows Win32 | macOS AppKit | Linux GTK | Fallback rule |
| --- | --- | --- | --- | --- |
| Native window creation | Supported | Supported | Supported | If unavailable, the host must return `ZsuiError::Unsupported` instead of panicking. |
| Default size | Supported | Supported | Supported | Clamp invalid zero-sized declarations during validation or conversion. |
| Minimum size | Supported through `WM_GETMINMAXINFO` | Supported through `setContentMinSize` | Partial through `set_size_request` | If resize policy is unsupported, clear the min-size request and create a normal native window. |
| Resizable/fixed window | Supported through Win32 styles | Supported through `NSWindowStyleMask` | Partial because compositor behavior may vary | Unsupported hosts fall back to a standard resizable native window. |
| Native decorations | Supported through Win32 styles | Supported through `NSWindowStyleMask` | Partial because client/server-side decorations vary | Unsupported hosts keep native decorations enabled. |
| Always on top | Supported through extended styles | Supported through window level | Partial because session/backend support varies | Unsupported hosts create a normal z-order window. |
| Transparency | Unsupported in the current main-window bridge | Unsupported in the current main-window bridge | Unsupported in the current main-window bridge | Resolve to an opaque native window and record the degradation. |

## Current Backend Notes

Windows is the reference implementation for existing ZSClip behavior. Native
windowing, tray, menus, hotkeys, clipboard, dialogs, file picker and paste
target code already exist in `src/app/*` and `src/platform/*`; the standalone `zsui` crate now
defines the smaller trait those pieces can gradually implement.

macOS has AppKit host work in `src/macos_app.rs`, `src/macos_appkit_adapter.rs`
and `src/macos_native_host.rs`. The new matrix marks many features as
`Partial` because there is code-level native host work, but each feature should
still be verified on macOS before being treated as production-ready.

Linux has GTK/libadwaita host work in `src/linux_app.rs`,
`src/linux_gtk_adapter.rs` and `src/linux_native_host.rs`. Desktop integration
varies by session, especially for status notifier items, file portals,
clipboard payload types and paste automation.

Every platform backend must return `ZsuiError::Unsupported` for unavailable
operations. Backends should not panic for missing desktop services.

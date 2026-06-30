//! ZSUI public framework surface.
//!
//! ZSUI is a Rust-first native system UI framework shape. It is not a self
//! drawing widget kit: applications declare windows, tray/status menus,
//! shortcuts, settings pages and commands in Rust, while platform hosts map
//! those declarations to Win32, AppKit or GTK/libadwaita backends.

pub mod app;
pub mod capability;
pub mod clipboard;
pub mod components;
pub mod core;
pub mod host;
pub mod hotkey;
pub mod icon;
pub mod menu;
pub mod settings;
pub mod tray;
pub mod window;

pub use app::{app, AppBuilder, ZsuiApp, ZsuiAppRuntime};
pub use capability::{CapabilityStatus, CapabilitySupport, HostCapabilities, PlatformName};
pub use clipboard::ClipboardData;
pub use components::ZsTabSpec;
pub use core::{
    AppEvent, Command, DialogButtons, DialogLevel, DialogResponse, FileDialogFilter,
    FileDialogSpec, HotkeyId, NativeDialogSpec, TrayId, WindowId, ZsuiError, ZsuiResult,
};
pub use host::{MemoryHost, PlatformHost, TrayRecord, WindowRecord, ZsuiHost};
pub use hotkey::HotkeySpec;
pub use icon::ZsIcon;
pub use menu::{MenuItemSpec, MenuSpec};
pub use settings::{SettingsItemKind, SettingsItemSpec, SettingsPageSpec, SettingsValue};
pub use tray::TraySpec;
pub use window::{Window, WindowNativeOptions, WindowResolvedSpec, WindowSpec};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fluent_declaration_registers_window_tray_and_hotkey() {
        let mut host = MemoryHost::new();

        let runtime = app("ZSClip")
            .window(Window::new("ZSClip").size(900, 620))
            .tray(
                TraySpec::new()
                    .tooltip("ZSClip")
                    .item("Open", Command::ShowMainWindow)
                    .separator()
                    .item("Quit", Command::Quit),
            )
            .global_hotkey("Alt+V", Command::OpenQuickPanel)
            .run_with_host(&mut host)
            .expect("memory host should accept the demo declaration");

        assert_eq!(runtime.app_name, "ZSClip");
        assert_eq!(host.windows()[0].spec.title, "ZSClip");
        assert_eq!(host.windows()[0].spec.width, 900);
        assert_eq!(host.trays()[0].spec.menu.items.len(), 3);
        assert_eq!(host.hotkeys()[0].spec.accelerator, "Alt+V");
    }

    #[test]
    fn unsupported_host_capability_returns_error_instead_of_panicking() {
        let capabilities = HostCapabilities::all_unsupported(PlatformName::Unknown);
        let mut host = MemoryHost::with_capabilities(capabilities);

        let err = app("ZSClip")
            .window(WindowSpec::new("ZSClip"))
            .run_with_host(&mut host)
            .expect_err("unsupported window creation should be reported");

        assert!(matches!(err, ZsuiError::Unsupported { .. }));
    }

    #[test]
    fn window_alias_supports_standard_builder_shape() {
        let window = Window::new("ZSClip")
            .size(900, 620)
            .min_size(640, 420)
            .resizable(true)
            .decorations(true);

        assert_eq!(window.title, "ZSClip");
        assert_eq!(window.width, 900);
        assert_eq!(window.height, 620);
        assert_eq!(window.min_width, Some(640));
        assert!(window.resizable);
        assert!(window.decorations);
    }

    #[test]
    fn window_native_options_snapshot_matches_builder_fields() {
        let options = Window::new("ZSClip")
            .min_size(640, 420)
            .resizable(false)
            .decorations(false)
            .always_on_top(true)
            .transparent(true)
            .native_options();

        assert_eq!(options.min_width, Some(640));
        assert_eq!(options.min_height, Some(420));
        assert!(!options.resizable);
        assert!(!options.decorations);
        assert!(options.always_on_top);
        assert!(options.transparent);
    }

    #[test]
    fn native_window_host_capabilities_describe_real_window_mapping() {
        let windows = HostCapabilities::windows_native_window_host();
        assert_eq!(windows.windows.status, CapabilityStatus::Supported);
        assert_eq!(windows.window_resizing.status, CapabilityStatus::Supported);
        assert_eq!(
            windows.window_decorations.status,
            CapabilityStatus::Supported
        );
        assert_eq!(
            windows.window_always_on_top.status,
            CapabilityStatus::Supported
        );
        assert_eq!(
            windows.window_transparency.status,
            CapabilityStatus::Unsupported
        );

        let macos = HostCapabilities::macos_native_window_host();
        assert_eq!(macos.windows.status, CapabilityStatus::Supported);
        assert_eq!(macos.window_resizing.status, CapabilityStatus::Supported);
        assert_eq!(macos.window_decorations.status, CapabilityStatus::Supported);
        assert_eq!(
            macos.window_always_on_top.status,
            CapabilityStatus::Supported
        );
        assert_eq!(
            macos.window_transparency.status,
            CapabilityStatus::Unsupported
        );

        let linux = HostCapabilities::linux_native_window_host();
        assert_eq!(linux.windows.status, CapabilityStatus::Supported);
        assert_eq!(linux.window_resizing.status, CapabilityStatus::Partial);
        assert_eq!(linux.window_decorations.status, CapabilityStatus::Partial);
        assert_eq!(linux.window_always_on_top.status, CapabilityStatus::Partial);
        assert_eq!(
            linux.window_transparency.status,
            CapabilityStatus::Unsupported
        );

        for capabilities in [
            HostCapabilities::windows_native_window_host(),
            HostCapabilities::macos_native_window_host(),
            HostCapabilities::linux_native_window_host(),
        ] {
            let resolved = Window::new("ZSClip")
                .transparent(true)
                .resolve_for(&capabilities);
            assert!(resolved.requested.transparent);
            assert!(!resolved.effective.transparent);
        }
    }

    #[test]
    fn unsupported_window_traits_resolve_to_standard_native_fallbacks() {
        let mut capabilities = HostCapabilities::all_supported(PlatformName::Unknown);
        capabilities.window_resizing = CapabilitySupport::unsupported("resize policy unavailable");
        capabilities.window_decorations =
            CapabilitySupport::unsupported("decoration policy unavailable");
        capabilities.window_always_on_top = CapabilitySupport::unsupported("topmost unavailable");
        capabilities.window_transparency =
            CapabilitySupport::unsupported("transparency unavailable");

        let resolved = Window::new("ZSClip")
            .min_size(640, 420)
            .resizable(false)
            .decorations(false)
            .always_on_top(true)
            .transparent(true)
            .resolve_for(&capabilities);

        assert!(!resolved.requested.resizable);
        assert!(!resolved.requested.decorations);
        assert!(resolved.requested.always_on_top);
        assert!(resolved.requested.transparent);
        assert!(resolved.effective.resizable);
        assert_eq!(resolved.effective.min_width, None);
        assert!(resolved.effective.decorations);
        assert!(!resolved.effective.always_on_top);
        assert!(!resolved.effective.transparent);
    }

    #[test]
    fn memory_host_records_requested_and_effective_window_specs() {
        let mut capabilities = HostCapabilities::all_supported(PlatformName::Unknown);
        capabilities.window_always_on_top = CapabilitySupport::unsupported("topmost unavailable");
        capabilities.window_transparency =
            CapabilitySupport::unsupported("transparency unavailable");
        let mut host = MemoryHost::with_capabilities(capabilities);

        app("ZSClip")
            .window(Window::new("ZSClip").always_on_top(true).transparent(true))
            .run_with_host(&mut host)
            .expect("window should fall back instead of failing");

        let record = &host.windows()[0];
        assert!(record.spec.always_on_top);
        assert!(record.spec.transparent);
        assert!(!record.effective_spec.always_on_top);
        assert!(!record.effective_spec.transparent);
        assert!(record
            .degraded_capabilities
            .iter()
            .any(|detail| detail.contains("window_always_on_top")));
    }

    #[test]
    fn requested_window_features_report_host_degradation() {
        let mut host = MemoryHost::with_capabilities(HostCapabilities::linux_scaffold());

        let runtime = app("ZSClip")
            .window(Window::new("ZSClip").always_on_top(true).transparent(true))
            .run_with_host(&mut host)
            .expect("partial Linux scaffold should accept window declarations");

        assert!(runtime
            .degraded_capabilities
            .iter()
            .any(|detail| detail.contains("window[0].window_always_on_top")));
        assert!(runtime
            .degraded_capabilities
            .iter()
            .any(|detail| detail.contains("window[0].window_transparency")));
    }

    #[test]
    fn specs_are_serializable_for_ai_and_tooling_contexts() {
        let spec = TraySpec::new()
            .item("Open", Command::ShowMainWindow)
            .item("Settings", Command::OpenSettings);

        let json = serde_json::to_string(&spec).expect("tray spec should serialize");
        assert!(json.contains("ShowMainWindow"));
        assert!(json.contains("OpenSettings"));
    }
}

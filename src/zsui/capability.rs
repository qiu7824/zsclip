use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlatformName {
    Windows,
    Macos,
    Linux,
    Unknown,
    Other(String),
}

impl PlatformName {
    pub fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Macos
        } else if cfg!(target_os = "linux") {
            Self::Linux
        } else {
            Self::Unknown
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Windows => "windows",
            Self::Macos => "macos",
            Self::Linux => "linux",
            Self::Unknown => "unknown",
            Self::Other(value) => value.as_str(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapabilityStatus {
    Supported,
    Partial,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySupport {
    pub status: CapabilityStatus,
    pub detail: String,
}

impl CapabilitySupport {
    pub fn supported(detail: impl Into<String>) -> Self {
        Self {
            status: CapabilityStatus::Supported,
            detail: detail.into(),
        }
    }

    pub fn partial(detail: impl Into<String>) -> Self {
        Self {
            status: CapabilityStatus::Partial,
            detail: detail.into(),
        }
    }

    pub fn unsupported(detail: impl Into<String>) -> Self {
        Self {
            status: CapabilityStatus::Unsupported,
            detail: detail.into(),
        }
    }

    pub fn accepts_declaration(&self) -> bool {
        !matches!(self.status, CapabilityStatus::Unsupported)
    }

    pub fn is_fully_supported(&self) -> bool {
        matches!(self.status, CapabilityStatus::Supported)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostCapabilities {
    pub platform: PlatformName,
    pub windows: CapabilitySupport,
    pub window_resizing: CapabilitySupport,
    pub window_decorations: CapabilitySupport,
    pub window_always_on_top: CapabilitySupport,
    pub window_transparency: CapabilitySupport,
    pub tray_or_status_menu: CapabilitySupport,
    pub menus: CapabilitySupport,
    pub global_hotkeys: CapabilitySupport,
    pub clipboard_text: CapabilitySupport,
    pub clipboard_image: CapabilitySupport,
    pub clipboard_files: CapabilitySupport,
    pub file_picker: CapabilitySupport,
    pub native_dialogs: CapabilitySupport,
    pub settings_pages: CapabilitySupport,
    pub auto_paste: CapabilitySupport,
}

impl HostCapabilities {
    pub fn all_supported(platform: PlatformName) -> Self {
        Self {
            platform,
            windows: CapabilitySupport::supported("window declarations are accepted"),
            window_resizing: CapabilitySupport::supported(
                "resizable and fixed-size windows are honored",
            ),
            window_decorations: CapabilitySupport::supported(
                "native window decorations are honored",
            ),
            window_always_on_top: CapabilitySupport::supported("always-on-top windows are honored"),
            window_transparency: CapabilitySupport::supported("transparent windows are honored"),
            tray_or_status_menu: CapabilitySupport::supported(
                "tray/status menu declarations are accepted",
            ),
            menus: CapabilitySupport::supported("menu declarations are accepted"),
            global_hotkeys: CapabilitySupport::supported("global hotkeys are accepted"),
            clipboard_text: CapabilitySupport::supported("text clipboard is available"),
            clipboard_image: CapabilitySupport::supported("image clipboard is available"),
            clipboard_files: CapabilitySupport::supported("file clipboard is available"),
            file_picker: CapabilitySupport::supported("native file picker is available"),
            native_dialogs: CapabilitySupport::supported("native dialogs are available"),
            settings_pages: CapabilitySupport::supported("settings page declarations are accepted"),
            auto_paste: CapabilitySupport::supported("native auto paste is available"),
        }
    }

    pub fn all_unsupported(platform: PlatformName) -> Self {
        let unsupported = CapabilitySupport::unsupported("not implemented by this host");
        Self {
            platform,
            windows: unsupported.clone(),
            window_resizing: unsupported.clone(),
            window_decorations: unsupported.clone(),
            window_always_on_top: unsupported.clone(),
            window_transparency: unsupported.clone(),
            tray_or_status_menu: unsupported.clone(),
            menus: unsupported.clone(),
            global_hotkeys: unsupported.clone(),
            clipboard_text: unsupported.clone(),
            clipboard_image: unsupported.clone(),
            clipboard_files: unsupported.clone(),
            file_picker: unsupported.clone(),
            native_dialogs: unsupported.clone(),
            settings_pages: unsupported.clone(),
            auto_paste: unsupported,
        }
    }

    pub fn current_platform_scaffold() -> Self {
        match PlatformName::current() {
            PlatformName::Windows => Self::windows_scaffold(),
            PlatformName::Macos => Self::macos_scaffold(),
            PlatformName::Linux => Self::linux_scaffold(),
            other => Self::all_unsupported(other),
        }
    }

    pub fn current_native_window_host() -> Self {
        match PlatformName::current() {
            PlatformName::Windows => Self::windows_native_window_host(),
            PlatformName::Macos => Self::macos_native_window_host(),
            PlatformName::Linux => Self::linux_native_window_host(),
            other => Self::all_unsupported(other),
        }
    }

    pub fn degraded_capabilities(&self) -> Vec<(&'static str, &CapabilitySupport)> {
        [
            ("windows", &self.windows),
            ("window_resizing", &self.window_resizing),
            ("window_decorations", &self.window_decorations),
            ("window_always_on_top", &self.window_always_on_top),
            ("window_transparency", &self.window_transparency),
            ("tray_or_status_menu", &self.tray_or_status_menu),
            ("menus", &self.menus),
            ("global_hotkeys", &self.global_hotkeys),
            ("clipboard_text", &self.clipboard_text),
            ("clipboard_image", &self.clipboard_image),
            ("clipboard_files", &self.clipboard_files),
            ("file_picker", &self.file_picker),
            ("native_dialogs", &self.native_dialogs),
            ("settings_pages", &self.settings_pages),
            ("auto_paste", &self.auto_paste),
        ]
        .into_iter()
        .filter(|(_, support)| !support.is_fully_supported())
        .collect()
    }

    pub fn windows_scaffold() -> Self {
        Self {
            platform: PlatformName::Windows,
            windows: CapabilitySupport::partial(
                "Win32 main window exists in ZSClip; ZsuiHost currently records declarations",
            ),
            window_resizing: CapabilitySupport::partial(
                "Win32 can create standard resizable windows; the current ZSClip main host uses popup tool windows",
            ),
            window_decorations: CapabilitySupport::partial(
                "Win32 can create decorated windows; the current ZSClip main host uses custom chrome",
            ),
            window_always_on_top: CapabilitySupport::partial(
                "Win32 topmost windows exist; ZsuiHost style mapping is not wired yet",
            ),
            window_transparency: CapabilitySupport::partial(
                "Win32 transparency exists for selected hosts; ZsuiHost mapping is not wired yet",
            ),
            tray_or_status_menu: CapabilitySupport::partial(
                "Win32 tray exists in ZSClip; ZsuiHost currently records declarations",
            ),
            menus: CapabilitySupport::partial(
                "Win32 menus exist in ZSClip; ZsuiHost currently records declarations",
            ),
            global_hotkeys: CapabilitySupport::partial(
                "Win32 hotkeys exist in ZSClip; ZsuiHost currently records declarations",
            ),
            clipboard_text: CapabilitySupport::supported("text clipboard bridge is available"),
            clipboard_image: CapabilitySupport::partial(
                "image clipboard depends on backend integration",
            ),
            clipboard_files: CapabilitySupport::partial(
                "file clipboard exists in the Windows app backend",
            ),
            file_picker: CapabilitySupport::partial(
                "Win32 file picker exists in the platform layer",
            ),
            native_dialogs: CapabilitySupport::partial("Win32 dialogs exist in the platform layer"),
            settings_pages: CapabilitySupport::partial("settings page specs are declarative"),
            auto_paste: CapabilitySupport::partial(
                "Windows paste-target code exists outside ZsuiHost",
            ),
        }
    }

    pub fn windows_native_window_host() -> Self {
        let mut capabilities = Self::windows_scaffold();
        capabilities.windows =
            CapabilitySupport::supported("Win32 native host creates main and quick windows");
        capabilities.window_resizing =
            CapabilitySupport::supported("Win32 window styles honor resizable and fixed windows");
        capabilities.window_decorations =
            CapabilitySupport::supported("Win32 window styles honor native decorations");
        capabilities.window_always_on_top =
            CapabilitySupport::supported("Win32 extended styles honor topmost windows");
        capabilities.window_transparency = CapabilitySupport::unsupported(
            "Win32 main window transparency is not mapped by the native window host yet",
        );
        capabilities
    }

    pub fn macos_scaffold() -> Self {
        Self {
            platform: PlatformName::Macos,
            windows: CapabilitySupport::partial("AppKit host exists; ZsuiHost adapter is stubbed"),
            window_resizing: CapabilitySupport::partial(
                "NSWindow supports resizable and fixed-size windows; ZsuiHost adapter is stubbed",
            ),
            window_decorations: CapabilitySupport::partial(
                "NSWindow style masks support native chrome; ZsuiHost adapter is stubbed",
            ),
            window_always_on_top: CapabilitySupport::partial(
                "NSWindow levels support floating windows; ZsuiHost adapter is stubbed",
            ),
            window_transparency: CapabilitySupport::partial(
                "transparent AppKit windows need host-specific material/backing configuration",
            ),
            tray_or_status_menu: CapabilitySupport::partial(
                "NSStatusItem host exists; ZsuiHost adapter is stubbed",
            ),
            menus: CapabilitySupport::partial("NSMenu host exists; ZsuiHost adapter is stubbed"),
            global_hotkeys: CapabilitySupport::unsupported(
                "global shortcut service is not wired in ZsuiHost",
            ),
            clipboard_text: CapabilitySupport::partial(
                "pasteboard host exists; generic ZsuiHost is not fully wired",
            ),
            clipboard_image: CapabilitySupport::partial(
                "pasteboard image support is backend dependent",
            ),
            clipboard_files: CapabilitySupport::partial(
                "pasteboard file support is backend dependent",
            ),
            file_picker: CapabilitySupport::partial("NSOpenPanel host exists in native scaffold"),
            native_dialogs: CapabilitySupport::partial("NSAlert host exists in native scaffold"),
            settings_pages: CapabilitySupport::partial("settings page specs are declarative"),
            auto_paste: CapabilitySupport::unsupported("auto paste requires accessibility trust"),
        }
    }

    pub fn macos_native_window_host() -> Self {
        let mut capabilities = Self::macos_scaffold();
        capabilities.windows = CapabilitySupport::supported("AppKit native host creates NSWindow");
        capabilities.window_resizing =
            CapabilitySupport::supported("NSWindow style masks honor resizable and fixed windows");
        capabilities.window_decorations =
            CapabilitySupport::supported("NSWindow style masks honor native chrome");
        capabilities.window_always_on_top =
            CapabilitySupport::supported("NSWindow levels honor floating always-on-top windows");
        capabilities.window_transparency = CapabilitySupport::unsupported(
            "AppKit main window transparency is not mapped by the native window host yet",
        );
        capabilities
    }

    pub fn linux_scaffold() -> Self {
        Self {
            platform: PlatformName::Linux,
            windows: CapabilitySupport::partial("GTK host exists; ZsuiHost adapter is stubbed"),
            window_resizing: CapabilitySupport::partial(
                "GTK can request default and fixed sizes; compositor behavior may vary",
            ),
            window_decorations: CapabilitySupport::partial(
                "GTK can request decorations, but server-side/client-side chrome varies by desktop",
            ),
            window_always_on_top: CapabilitySupport::partial(
                "always-on-top requires backend/session support such as X11 helpers or layer shell",
            ),
            window_transparency: CapabilitySupport::partial(
                "transparent GTK windows depend on compositor and backend support",
            ),
            tray_or_status_menu: CapabilitySupport::partial(
                "StatusNotifier host exists; desktop support may vary",
            ),
            menus: CapabilitySupport::partial("GTK/GIO menu host exists; adapter is stubbed"),
            global_hotkeys: CapabilitySupport::unsupported(
                "global shortcut support varies by display server",
            ),
            clipboard_text: CapabilitySupport::partial(
                "GTK clipboard host exists; generic ZsuiHost is not fully wired",
            ),
            clipboard_image: CapabilitySupport::partial(
                "image clipboard depends on GTK backend integration",
            ),
            clipboard_files: CapabilitySupport::partial(
                "file clipboard depends on portal/GTK backend integration",
            ),
            file_picker: CapabilitySupport::partial("GTK file picker host exists in scaffold"),
            native_dialogs: CapabilitySupport::partial("GTK dialog host exists in scaffold"),
            settings_pages: CapabilitySupport::partial("settings page specs are declarative"),
            auto_paste: CapabilitySupport::partial(
                "xdotool/keytap path is backend and session dependent",
            ),
        }
    }

    pub fn linux_native_window_host() -> Self {
        let mut capabilities = Self::linux_scaffold();
        capabilities.windows =
            CapabilitySupport::supported("GTK native host creates ApplicationWindow");
        capabilities.window_resizing = CapabilitySupport::partial(
            "GTK can request resizable and fixed windows; compositor behavior may vary",
        );
        capabilities.window_decorations = CapabilitySupport::partial(
            "GTK can request decorations; server-side and client-side chrome vary by desktop",
        );
        capabilities.window_always_on_top = CapabilitySupport::partial(
            "GTK always-on-top requires backend/session support such as X11 helpers or layer shell",
        );
        capabilities.window_transparency = CapabilitySupport::unsupported(
            "GTK main window transparency is not mapped by the native window host yet",
        );
        capabilities
    }
}

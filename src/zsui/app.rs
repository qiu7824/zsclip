use crate::zsui::{
    capability::{CapabilityStatus, HostCapabilities},
    core::{Command, TrayId, WindowId, ZsuiError, ZsuiResult},
    host::{PlatformHost, ZsuiHost},
    hotkey::HotkeySpec,
    settings::SettingsPageSpec,
    tray::TraySpec,
    window::WindowSpec,
};

pub fn app(name: impl Into<String>) -> AppBuilder {
    AppBuilder::new(name)
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppBuilder {
    name: String,
    windows: Vec<WindowSpec>,
    tray: Option<TraySpec>,
    hotkeys: Vec<HotkeySpec>,
    settings_pages: Vec<SettingsPageSpec>,
}

impl AppBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            windows: Vec::new(),
            tray: None,
            hotkeys: Vec::new(),
            settings_pages: Vec::new(),
        }
    }

    pub fn window(mut self, spec: WindowSpec) -> Self {
        self.windows.push(spec);
        self
    }

    pub fn tray(mut self, spec: TraySpec) -> Self {
        self.tray = Some(spec);
        self
    }

    pub fn global_hotkey(mut self, accelerator: impl Into<String>, command: Command) -> Self {
        self.hotkeys.push(HotkeySpec::new(accelerator, command));
        self
    }

    pub fn hotkey(mut self, spec: HotkeySpec) -> Self {
        self.hotkeys.push(spec);
        self
    }

    pub fn settings_page(mut self, spec: SettingsPageSpec) -> Self {
        self.settings_pages.push(spec);
        self
    }

    pub fn build(self) -> ZsuiResult<ZsuiApp> {
        validate_app_name(&self.name)?;
        for window in &self.windows {
            validate_window(window)?;
        }
        for hotkey in &self.hotkeys {
            validate_hotkey(hotkey)?;
        }
        Ok(ZsuiApp {
            name: self.name,
            windows: self.windows,
            tray: self.tray,
            hotkeys: self.hotkeys,
            settings_pages: self.settings_pages,
        })
    }

    pub fn run(self) -> ZsuiResult<ZsuiAppRuntime> {
        let mut host = PlatformHost::new();
        self.run_with_host(&mut host)
    }

    pub fn run_with_host<H: ZsuiHost>(self, host: &mut H) -> ZsuiResult<ZsuiAppRuntime> {
        self.build()?.run_with_host(host)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ZsuiApp {
    pub name: String,
    pub windows: Vec<WindowSpec>,
    pub tray: Option<TraySpec>,
    pub hotkeys: Vec<HotkeySpec>,
    pub settings_pages: Vec<SettingsPageSpec>,
}

impl ZsuiApp {
    pub fn run_with_host<H: ZsuiHost>(&self, host: &mut H) -> ZsuiResult<ZsuiAppRuntime> {
        let capabilities = host.capabilities();
        let mut window_ids = Vec::new();
        let mut hotkey_ids = Vec::new();

        for window in &self.windows {
            window_ids.push(host.create_main_window(window)?);
        }

        let tray_id = if let Some(tray) = &self.tray {
            Some(host.create_tray(tray)?)
        } else {
            None
        };

        for hotkey in &self.hotkeys {
            if hotkey.enabled {
                hotkey_ids.push(host.register_global_hotkey(hotkey)?);
            }
        }

        host.run_event_loop()?;

        Ok(ZsuiAppRuntime {
            app_name: self.name.clone(),
            windows: window_ids,
            tray: tray_id,
            hotkeys: hotkey_ids,
            settings_pages: self.settings_pages.len(),
            capabilities: capabilities.clone(),
            degraded_capabilities: used_degraded_capabilities(self, &capabilities),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ZsuiAppRuntime {
    pub app_name: String,
    pub windows: Vec<WindowId>,
    pub tray: Option<TrayId>,
    pub hotkeys: Vec<crate::zsui::core::HotkeyId>,
    pub settings_pages: usize,
    pub capabilities: HostCapabilities,
    pub degraded_capabilities: Vec<String>,
}

fn validate_app_name(name: &str) -> ZsuiResult<()> {
    if name.trim().is_empty() {
        Err(ZsuiError::invalid_spec(
            "app.name",
            "app name cannot be empty",
        ))
    } else {
        Ok(())
    }
}

fn validate_window(window: &WindowSpec) -> ZsuiResult<()> {
    if window.title.trim().is_empty() {
        return Err(ZsuiError::invalid_spec(
            "window.title",
            "window title cannot be empty",
        ));
    }
    if window.width == 0 || window.height == 0 {
        return Err(ZsuiError::invalid_spec(
            "window.size",
            "window size must be greater than zero",
        ));
    }
    Ok(())
}

fn validate_hotkey(hotkey: &HotkeySpec) -> ZsuiResult<()> {
    if hotkey.enabled && hotkey.accelerator.trim().is_empty() {
        Err(ZsuiError::invalid_spec(
            "hotkey.accelerator",
            "enabled hotkey accelerator cannot be empty",
        ))
    } else {
        Ok(())
    }
}

fn used_degraded_capabilities(app: &ZsuiApp, capabilities: &HostCapabilities) -> Vec<String> {
    let mut degraded = Vec::new();
    for (index, window) in app.windows.iter().enumerate() {
        for detail in window.degraded_capabilities(capabilities) {
            degraded.push(format!("window[{index}].{detail}"));
        }
    }
    if app.tray.is_some() && capabilities.tray_or_status_menu.status != CapabilityStatus::Supported
    {
        degraded.push(format!(
            "tray_or_status_menu: {}",
            capabilities.tray_or_status_menu.detail
        ));
    }
    if !app.hotkeys.is_empty() && capabilities.global_hotkeys.status != CapabilityStatus::Supported
    {
        degraded.push(format!(
            "global_hotkeys: {}",
            capabilities.global_hotkeys.detail
        ));
    }
    if !app.settings_pages.is_empty()
        && capabilities.settings_pages.status != CapabilityStatus::Supported
    {
        degraded.push(format!(
            "settings_pages: {}",
            capabilities.settings_pages.detail
        ));
    }
    degraded
}

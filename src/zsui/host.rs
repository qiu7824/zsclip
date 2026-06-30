use crate::zsui::{
    capability::{CapabilitySupport, HostCapabilities, PlatformName},
    clipboard::ClipboardData,
    core::{
        AppEvent, DialogResponse, FileDialogSpec, HotkeyId, NativeDialogSpec, TrayId, WindowId,
        ZsuiError, ZsuiResult,
    },
    hotkey::HotkeySpec,
    tray::TraySpec,
    window::WindowSpec,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowRecord {
    pub id: WindowId,
    pub spec: WindowSpec,
    pub effective_spec: WindowSpec,
    pub visible: bool,
    pub degraded_capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayRecord {
    pub id: TrayId,
    pub spec: TraySpec,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotkeyRecord {
    pub id: HotkeyId,
    pub spec: HotkeySpec,
}

pub trait ZsuiHost {
    fn capabilities(&self) -> HostCapabilities;
    fn create_main_window(&mut self, spec: &WindowSpec) -> ZsuiResult<WindowId>;
    fn set_window_visible(&mut self, window: WindowId, visible: bool) -> ZsuiResult<()>;
    fn create_tray(&mut self, spec: &TraySpec) -> ZsuiResult<TrayId>;
    fn register_global_hotkey(&mut self, spec: &HotkeySpec) -> ZsuiResult<HotkeyId>;
    fn read_clipboard(&mut self) -> ZsuiResult<Option<ClipboardData>>;
    fn write_clipboard(&mut self, data: &ClipboardData) -> ZsuiResult<()>;
    fn open_file_picker(&mut self, spec: &FileDialogSpec) -> ZsuiResult<Option<Vec<String>>>;
    fn show_native_dialog(&mut self, spec: &NativeDialogSpec) -> ZsuiResult<DialogResponse>;

    fn show_window(&mut self, window: WindowId) -> ZsuiResult<()> {
        self.set_window_visible(window, true)
    }

    fn hide_window(&mut self, window: WindowId) -> ZsuiResult<()> {
        self.set_window_visible(window, false)
    }

    fn poll_event(&mut self) -> ZsuiResult<Option<AppEvent>> {
        Ok(None)
    }

    fn run_event_loop(&mut self) -> ZsuiResult<()> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct MemoryHost {
    capabilities: HostCapabilities,
    next_id: u64,
    windows: Vec<WindowRecord>,
    trays: Vec<TrayRecord>,
    hotkeys: Vec<HotkeyRecord>,
    clipboard: Option<ClipboardData>,
    events: Vec<AppEvent>,
    file_picker_result: Option<Vec<String>>,
    file_picker_requests: Vec<FileDialogSpec>,
    dialog_requests: Vec<NativeDialogSpec>,
    dialog_response: DialogResponse,
}

impl MemoryHost {
    pub fn new() -> Self {
        Self::with_capabilities(HostCapabilities::all_supported(PlatformName::Unknown))
    }

    pub fn with_capabilities(capabilities: HostCapabilities) -> Self {
        Self {
            capabilities,
            next_id: 1,
            windows: Vec::new(),
            trays: Vec::new(),
            hotkeys: Vec::new(),
            clipboard: None,
            events: Vec::new(),
            file_picker_result: None,
            file_picker_requests: Vec::new(),
            dialog_requests: Vec::new(),
            dialog_response: DialogResponse::Ok,
        }
    }

    pub fn windows(&self) -> &[WindowRecord] {
        &self.windows
    }

    pub fn trays(&self) -> &[TrayRecord] {
        &self.trays
    }

    pub fn hotkeys(&self) -> &[HotkeyRecord] {
        &self.hotkeys
    }

    pub fn file_picker_requests(&self) -> &[FileDialogSpec] {
        &self.file_picker_requests
    }

    pub fn dialog_requests(&self) -> &[NativeDialogSpec] {
        &self.dialog_requests
    }

    pub fn set_file_picker_result(&mut self, result: Option<Vec<String>>) {
        self.file_picker_result = result;
    }

    pub fn set_dialog_response(&mut self, response: DialogResponse) {
        self.dialog_response = response;
    }

    pub fn push_event(&mut self, event: AppEvent) {
        self.events.push(event);
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

impl Default for MemoryHost {
    fn default() -> Self {
        Self::new()
    }
}

impl ZsuiHost for MemoryHost {
    fn capabilities(&self) -> HostCapabilities {
        self.capabilities.clone()
    }

    fn create_main_window(&mut self, spec: &WindowSpec) -> ZsuiResult<WindowId> {
        ensure_supported("windows", &self.capabilities.windows)?;
        let resolved = spec.resolve_for(&self.capabilities);
        let id = WindowId(self.next_id());
        self.windows.push(WindowRecord {
            id,
            spec: resolved.requested,
            effective_spec: resolved.effective.clone(),
            visible: resolved.effective.visible,
            degraded_capabilities: resolved.degraded_capabilities,
        });
        self.events.push(AppEvent::WindowCreated { window: id });
        Ok(id)
    }

    fn set_window_visible(&mut self, window: WindowId, visible: bool) -> ZsuiResult<()> {
        ensure_supported("windows", &self.capabilities.windows)?;
        let Some(record) = self.windows.iter_mut().find(|record| record.id == window) else {
            return Err(ZsuiError::host(
                "set_window_visible",
                format!("unknown window id {}", window.0),
            ));
        };
        record.visible = visible;
        self.events.push(if visible {
            AppEvent::WindowShown { window }
        } else {
            AppEvent::WindowHidden { window }
        });
        Ok(())
    }

    fn create_tray(&mut self, spec: &TraySpec) -> ZsuiResult<TrayId> {
        ensure_supported(
            "tray_or_status_menu",
            &self.capabilities.tray_or_status_menu,
        )?;
        let id = TrayId(self.next_id());
        self.trays.push(TrayRecord {
            id,
            spec: spec.clone(),
        });
        Ok(id)
    }

    fn register_global_hotkey(&mut self, spec: &HotkeySpec) -> ZsuiResult<HotkeyId> {
        ensure_supported("global_hotkeys", &self.capabilities.global_hotkeys)?;
        let id = HotkeyId(self.next_id());
        self.hotkeys.push(HotkeyRecord {
            id,
            spec: spec.clone(),
        });
        Ok(id)
    }

    fn read_clipboard(&mut self) -> ZsuiResult<Option<ClipboardData>> {
        ensure_supported("clipboard_text", &self.capabilities.clipboard_text)?;
        Ok(self.clipboard.clone())
    }

    fn write_clipboard(&mut self, data: &ClipboardData) -> ZsuiResult<()> {
        match data {
            ClipboardData::Text(_) | ClipboardData::Empty => {
                ensure_supported("clipboard_text", &self.capabilities.clipboard_text)?;
            }
            ClipboardData::ImageRgba { .. } => {
                ensure_supported("clipboard_image", &self.capabilities.clipboard_image)?;
            }
            ClipboardData::Files(_) => {
                ensure_supported("clipboard_files", &self.capabilities.clipboard_files)?;
            }
        }
        self.clipboard = Some(data.clone());
        self.events.push(AppEvent::ClipboardChanged);
        Ok(())
    }

    fn open_file_picker(&mut self, spec: &FileDialogSpec) -> ZsuiResult<Option<Vec<String>>> {
        ensure_supported("file_picker", &self.capabilities.file_picker)?;
        self.file_picker_requests.push(spec.clone());
        Ok(self.file_picker_result.clone())
    }

    fn show_native_dialog(&mut self, spec: &NativeDialogSpec) -> ZsuiResult<DialogResponse> {
        ensure_supported("native_dialogs", &self.capabilities.native_dialogs)?;
        self.dialog_requests.push(spec.clone());
        self.events.push(AppEvent::DialogClosed {
            response: self.dialog_response,
        });
        Ok(self.dialog_response)
    }

    fn poll_event(&mut self) -> ZsuiResult<Option<AppEvent>> {
        if self.events.is_empty() {
            Ok(None)
        } else {
            Ok(Some(self.events.remove(0)))
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlatformHost {
    inner: MemoryHost,
}

impl PlatformHost {
    pub fn new() -> Self {
        Self {
            inner: MemoryHost::with_capabilities(HostCapabilities::current_platform_scaffold()),
        }
    }

    pub fn recorded_windows(&self) -> &[WindowRecord] {
        self.inner.windows()
    }

    pub fn recorded_trays(&self) -> &[TrayRecord] {
        self.inner.trays()
    }
}

impl Default for PlatformHost {
    fn default() -> Self {
        Self::new()
    }
}

impl ZsuiHost for PlatformHost {
    fn capabilities(&self) -> HostCapabilities {
        self.inner.capabilities()
    }

    fn create_main_window(&mut self, spec: &WindowSpec) -> ZsuiResult<WindowId> {
        self.inner.create_main_window(spec)
    }

    fn set_window_visible(&mut self, window: WindowId, visible: bool) -> ZsuiResult<()> {
        self.inner.set_window_visible(window, visible)
    }

    fn create_tray(&mut self, spec: &TraySpec) -> ZsuiResult<TrayId> {
        self.inner.create_tray(spec)
    }

    fn register_global_hotkey(&mut self, spec: &HotkeySpec) -> ZsuiResult<HotkeyId> {
        self.inner.register_global_hotkey(spec)
    }

    fn read_clipboard(&mut self) -> ZsuiResult<Option<ClipboardData>> {
        if !self.capabilities().clipboard_text.accepts_declaration() {
            return self.inner.read_clipboard();
        }
        match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.get_text()) {
            Ok(text) => Ok(Some(ClipboardData::Text(text))),
            Err(_) => self.inner.read_clipboard(),
        }
    }

    fn write_clipboard(&mut self, data: &ClipboardData) -> ZsuiResult<()> {
        match data {
            ClipboardData::Text(text) => {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    clipboard
                        .set_text(text.clone())
                        .map_err(|err| ZsuiError::host("write_clipboard", err.to_string()))?;
                    self.inner.write_clipboard(data)?;
                    return Ok(());
                }
                self.inner.write_clipboard(data)
            }
            ClipboardData::Empty => self.inner.write_clipboard(data),
            ClipboardData::ImageRgba { .. } => Err(ZsuiError::unsupported(
                "clipboard_image",
                "PlatformHost image clipboard bridge is not wired yet",
            )),
            ClipboardData::Files(_) => Err(ZsuiError::unsupported(
                "clipboard_files",
                "PlatformHost file clipboard bridge is not wired yet",
            )),
        }
    }

    fn open_file_picker(&mut self, spec: &FileDialogSpec) -> ZsuiResult<Option<Vec<String>>> {
        self.inner.open_file_picker(spec)
    }

    fn show_native_dialog(&mut self, spec: &NativeDialogSpec) -> ZsuiResult<DialogResponse> {
        self.inner.show_native_dialog(spec)
    }

    fn poll_event(&mut self) -> ZsuiResult<Option<AppEvent>> {
        self.inner.poll_event()
    }
}

fn ensure_supported(capability: &str, support: &CapabilitySupport) -> ZsuiResult<()> {
    if support.accepts_declaration() {
        Ok(())
    } else {
        Err(ZsuiError::unsupported(capability, support.detail.as_str()))
    }
}

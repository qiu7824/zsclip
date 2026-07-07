use std::ffi::c_void;
use std::ptr::{null, null_mut};

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::Gdi::{DEFAULT_GUI_FONT, PAINTSTRUCT},
    UI::WindowsAndMessaging::*,
};

use crate::app_core::{
    native_host_settings_action_button_specs, native_host_settings_control_button_specs,
    native_host_settings_dropdown_specs, native_host_settings_group_button_specs,
    native_host_settings_platform_button_specs, native_host_settings_toggle_specs,
    set_settings_ui_dpi, settings_command_for_control_role, settings_scale, Command,
    MouseButton as UiMouseButton, NativeControlMapper, NativeHostSettingsGroupAction,
    NativeHostSettingsPlatformAction, NativeSettingsControlHost, NativeSettingsDropdownHost,
    NativeSettingsDropdownPresentation, NativeSettingsDropdownRequest, NativeSettingsWindowHost,
    NativeSettingsWindowPresentation, NativeSettingsWindowRequest, Point, SettingsAction,
    SettingsControlRole, UiEvent, UiRect,
};
pub(crate) use crate::app_core::{SettingsComponentKind, SettingsControlSpec};
use crate::i18n::translate;
use crate::platform::appearance as platform_appearance;
use crate::platform::buffered_paint::{begin_buffered_paint, end_buffered_paint};
use crate::platform::dpi as platform_dpi;
use crate::platform::gdi as platform_gdi;
use crate::platform::input as platform_input;
use crate::platform::string::to_wide;
use crate::platform::system_parameters as platform_system_parameters;
use crate::platform::ui_event as platform_ui_event;
use crate::platform::window as platform_window;
use crate::settings_model::{
    settings_child_visible_in_viewport, settings_toggle_row_layout_for_rect,
    settings_viewport_mask_rect_for_window, settings_viewport_rect_for_window,
    SettingsControlState, SettingsDropdownInteractionState, SettingsDropdownPopupLayout,
    SettingsPage, SettingsScrollControlState, SettingsToggleRowLayout, SettingsUiModel,
    SETTINGS_DROPDOWN_PAD, SETTINGS_PAGE_COUNT, SETTINGS_VIEWPORT_MASK_H,
};
pub use crate::settings_model::{
    settings_dropdown_index_for_max_items, settings_dropdown_index_for_pos_mode,
    settings_dropdown_label_for_max_items, settings_dropdown_label_for_pos_mode,
    settings_dropdown_max_items_from_label, settings_dropdown_max_items_from_label_opt,
    settings_dropdown_max_items_labels, settings_dropdown_pos_mode_from_label,
};
use crate::ui::{draw_round_fill, draw_round_rect, draw_text_ex};
use crate::win_system_params::IDC_SET_RICH_TEXT;
use crate::win_native_style::{
    rgb, ui_display_font_family, ui_icon_font_family, ui_text_font_family, Theme,
    WindowsNativeControlMapper,
};
use crate::win_system_params::{
    IDC_SET_AUTOHIDE_BLUR, IDC_SET_AUTOSTART, IDC_SET_BTN_OPENCFG, IDC_SET_CLICK_HIDE,
    IDC_SET_CLIPBOARD_HISTORY_DISABLE, IDC_SET_CLIPBOARD_HISTORY_ENABLE, IDC_SET_CLOSE,
    IDC_SET_CLOSETRAY, IDC_SET_CLOUD_APPLY_CFG, IDC_SET_CLOUD_ENABLE, IDC_SET_CLOUD_INTERVAL,
    IDC_SET_CLOUD_RESTORE_BACKUP, IDC_SET_CLOUD_SYNC_NOW, IDC_SET_CLOUD_UPLOAD_CFG,
    IDC_SET_DEDUPE_FILTER, IDC_SET_EDGEHIDE, IDC_SET_GROUP_ADD, IDC_SET_GROUP_DELETE,
    IDC_SET_GROUP_DOWN, IDC_SET_GROUP_ENABLE, IDC_SET_GROUP_LIST, IDC_SET_GROUP_RENAME,
    IDC_SET_GROUP_TYPE_FILTER, IDC_SET_GROUP_UP, IDC_SET_GROUP_VIEW_PHRASES,
    IDC_SET_GROUP_VIEW_RECORDS, IDC_SET_HK_RECORD, IDC_SET_HOTKEY_ENABLE, IDC_SET_HOTKEY_KEY,
    IDC_SET_HOTKEY_MOD, IDC_SET_HOVERPREVIEW, IDC_SET_IMAGE_PREVIEW, IDC_SET_LAN_ACCEPT_PAIR,
    IDC_SET_LAN_COPY_PAIR, IDC_SET_LAN_COPY_SETUP, IDC_SET_LAN_DOCS, IDC_SET_LAN_ENABLE,
    IDC_SET_LAN_PAIR, IDC_SET_LAN_RECEIVE_MODE, IDC_SET_LAN_REFRESH, IDC_SET_LAN_REJECT_PAIR,
    IDC_SET_MAX, IDC_SET_MULTI_SYNC_MODE, IDC_SET_OCR_PROVIDER, IDC_SET_OCR_WECHAT_DETECT,
    IDC_SET_OPEN_SOURCE, IDC_SET_OPEN_UPDATE, IDC_SET_PASTE_MOVE_TOP, IDC_SET_PASTE_SOUND_ENABLE,
    IDC_SET_PASTE_SOUND_KIND, IDC_SET_PASTE_SOUND_PICK, IDC_SET_PERSIST_SEARCH,
    IDC_SET_PLAIN_HK_ENABLE, IDC_SET_PLAIN_HK_KEY, IDC_SET_PLAIN_HK_MOD, IDC_SET_PLUGIN_AI_CLEAN,
    IDC_SET_PLUGIN_MAILMERGE, IDC_SET_PLUGIN_QR_QUICK, IDC_SET_PLUGIN_SEARCH,
    IDC_SET_PLUGIN_SUPER_MAIL_MERGE, IDC_SET_PLUGIN_WPS_TASKPANE, IDC_SET_POSMODE,
    IDC_SET_QUICK_DELETE, IDC_SET_RESTART_EXPLORER, IDC_SET_SAVE, IDC_SET_SEARCH_ENGINE,
    IDC_SET_SEARCH_ENGINE_RESET, IDC_SET_SILENTSTART, IDC_SET_SKIP_WINDOW_CAPTURE,
    IDC_SET_SKIP_WINDOW_ENABLE, IDC_SET_TRANSLATE_PROVIDER, IDC_SET_TRANSLATE_TARGET,
    IDC_SET_TRAYICON, IDC_SET_VV_GROUP, IDC_SET_VV_MODE, IDC_SET_VV_SOURCE,
    IDC_SET_WPS_TASKPANE_DOCS, SETTINGS_CLASS,
};
use crate::win_system_ui::create_font_px;
use crate::win_ui_render::{
    DT_CENTER, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, DT_WORDBREAK,
};

pub const WM_SETTINGS_DROPDOWN_SELECTED: u32 = WM_APP + 91;
const VIEWPORT_CHILD_WINDOW_CLASS: &str = "ZsClipSettingsViewportChildWindow";
const DROPDOWN_CLASS: &str = "ZsClipDropdownPopup";
const DT_CALCRECT_FLAG: u32 = 0x0400;
const DT_EDITCONTROL_FLAG: u32 = 0x2000;
const DT_EXTERNALLEADING_FLAG: u32 = 0x0200;
const EM_SETMARGINS_MSG: u32 = 0x00D3;
const EM_SETPASSWORDCHAR_MSG: u32 = 0x00CC;
const SS_EDITCONTROL_STYLE: u32 = 0x2000;
const SS_NOPREFIX_STYLE: u32 = 0x0080;

#[derive(Clone, Copy)]
pub struct WindowsSettingsWindowRequest {
    pub owner: HWND,
    pub existing: HWND,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub window_proc: WNDPROC,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowsSettingsWindowPresentation {
    FocusedExisting(HWND),
    Created(HWND),
    Failed,
}

#[derive(Clone, Copy)]
pub struct WindowsSettingsWindowHost {
    window_proc: WNDPROC,
}

#[derive(Clone, Copy, Default)]
pub struct WindowsSettingsDropdownHost;

impl WindowsSettingsWindowHost {
    pub const fn new(window_proc: WNDPROC) -> Self {
        Self { window_proc }
    }
}

unsafe fn ensure_settings_window_class(window_proc: WNDPROC) {
    let module = platform_window::module_handle();
    let class_name = to_wide(SETTINGS_CLASS);
    let mut window_class: WNDCLASSEXW = std::mem::zeroed();
    window_class.cbSize = core::mem::size_of::<WNDCLASSEXW>() as u32;
    window_class.lpfnWndProc = window_proc;
    window_class.hInstance = module;
    window_class.hCursor = platform_window::arrow_cursor();
    window_class.hbrBackground = null_mut();
    window_class.lpszClassName = class_name.as_ptr();
    platform_window::register_class_ex(&window_class);
}

unsafe extern "system" fn settings_viewport_child_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_COMMAND | WM_NOTIFY | WM_DRAWITEM | WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT
        | WM_CTLCOLORLISTBOX | WM_CTLCOLORBTN | WM_MOUSEWHEEL => {
            let parent = platform_window::parent(hwnd);
            if parent.is_null() {
                0
            } else {
                platform_window::send_message(parent, msg, wparam, lparam)
            }
        }
        WM_ERASEBKGND => 1,
        _ => platform_window::default_window_proc(hwnd, msg, wparam, lparam),
    }
}

unsafe fn ensure_settings_viewport_child_class() {
    let module = platform_window::module_handle();
    let class_name = to_wide(VIEWPORT_CHILD_WINDOW_CLASS);
    let mut window_class: WNDCLASSEXW = std::mem::zeroed();
    window_class.cbSize = core::mem::size_of::<WNDCLASSEXW>() as u32;
    window_class.lpfnWndProc = Some(settings_viewport_child_proc);
    window_class.hInstance = module;
    window_class.hCursor = platform_window::arrow_cursor();
    window_class.hbrBackground = null_mut();
    window_class.lpszClassName = class_name.as_ptr();
    platform_window::register_class_ex(&window_class);
}

pub unsafe fn create_settings_viewport_child(parent: HWND) -> HWND {
    if parent.is_null() {
        return null_mut();
    }
    ensure_settings_viewport_child_class();
    let Some(client) = platform_window::client_rect(parent) else {
        return null_mut();
    };
    let viewport = settings_viewport_rect(&client);
    let hwnd = platform_window::create_window_ex(
        WS_EX_TRANSPARENT,
        to_wide(VIEWPORT_CHILD_WINDOW_CLASS).as_ptr(),
        to_wide("").as_ptr(),
        WS_CHILD | WS_CLIPCHILDREN | WS_CLIPSIBLINGS,
        viewport.left,
        viewport.top,
        viewport.right - viewport.left,
        viewport.bottom - viewport.top,
        parent,
        null_mut(),
        platform_window::module_handle(),
        null(),
    );
    sync_settings_viewport_child_bounds(parent, hwnd);
    hwnd
}

fn apply_settings_viewport_child_region(viewport_child: HWND, width: i32, height: i32) -> bool {
    if viewport_child.is_null() {
        return false;
    }
    let mask_h = settings_scale(SETTINGS_VIEWPORT_MASK_H).clamp(0, height.max(0));
    let region = platform_gdi::create_rect_rgn(0, mask_h, width.max(0), height.max(mask_h));
    if region.is_null() {
        return false;
    }
    if platform_window::set_window_region(viewport_child, region, false) {
        true
    } else {
        platform_gdi::delete_object(region as _);
        false
    }
}

pub(crate) fn sync_settings_viewport_child_bounds(parent: HWND, viewport_child: HWND) -> bool {
    if parent.is_null() || viewport_child.is_null() {
        return false;
    }
    let Some(client) = platform_window::client_rect(parent) else {
        return false;
    };
    let viewport = settings_viewport_rect(&client);
    let width = viewport.right - viewport.left;
    let height = viewport.bottom - viewport.top;
    let moved = platform_window::set_pos(
        viewport_child,
        null_mut(),
        viewport.left,
        viewport.top,
        width,
        height,
        SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOREDRAW,
    );
    let clipped = apply_settings_viewport_child_region(viewport_child, width, height);
    moved && clipped
}

pub(crate) fn set_settings_viewport_child_visible(viewport_child: HWND, visible: bool) {
    platform_window::set_visible(viewport_child, visible);
}

pub(crate) fn settings_viewport_child_control_bounds(
    original: UiRect,
    scroll_y: i32,
    viewport: RECT,
) -> UiRect {
    UiRect::new(
        original.left - viewport.left,
        original.top - viewport.top - scroll_y,
        original.right - viewport.left,
        original.bottom - viewport.top - scroll_y,
    )
}

pub(crate) fn settings_control_is_viewport_child(hwnd: HWND) -> bool {
    !hwnd.is_null() && platform_window::class_name(hwnd) == VIEWPORT_CHILD_WINDOW_CLASS
}

pub unsafe fn present_settings_window(
    request: WindowsSettingsWindowRequest,
) -> WindowsSettingsWindowPresentation {
    if !request.existing.is_null() {
        platform_window::show(request.existing);
        platform_window::set_foreground(request.existing);
        return WindowsSettingsWindowPresentation::FocusedExisting(request.existing);
    }

    ensure_settings_window_class(request.window_proc);
    let hwnd = platform_window::create_window_ex(
        WS_EX_APPWINDOW | WS_EX_DLGMODALFRAME | WS_EX_COMPOSITED,
        to_wide(SETTINGS_CLASS).as_ptr(),
        to_wide("").as_ptr(),
        WS_OVERLAPPED
            | WS_CAPTION
            | WS_SYSMENU
            | WS_MINIMIZEBOX
            | WS_MAXIMIZEBOX
            | WS_THICKFRAME
            | WS_VISIBLE
            | WS_CLIPCHILDREN
            | WS_CLIPSIBLINGS,
        request.x,
        request.y,
        request.width,
        request.height,
        request.owner,
        null_mut(),
        platform_window::module_handle(),
        request.owner as _,
    );
    if hwnd.is_null() {
        return WindowsSettingsWindowPresentation::Failed;
    }

    platform_appearance::set_rounded_corners(hwnd);
    platform_appearance::apply_dark_mode_to_window(hwnd);
    WindowsSettingsWindowPresentation::Created(hwnd)
}

impl NativeSettingsWindowHost for WindowsSettingsWindowHost {
    type Handle = HWND;

    fn present_settings_window(
        &mut self,
        request: NativeSettingsWindowRequest<Self::Handle>,
    ) -> NativeSettingsWindowPresentation<Self::Handle> {
        let existing = request.existing.unwrap_or(null_mut());
        let result = unsafe {
            present_settings_window(WindowsSettingsWindowRequest {
                owner: request.owner,
                existing,
                x: request.bounds.left,
                y: request.bounds.top,
                width: request.bounds.right - request.bounds.left,
                height: request.bounds.bottom - request.bounds.top,
                window_proc: self.window_proc,
            })
        };
        match result {
            WindowsSettingsWindowPresentation::FocusedExisting(hwnd) => {
                NativeSettingsWindowPresentation::FocusedExisting(hwnd)
            }
            WindowsSettingsWindowPresentation::Created(hwnd) => {
                NativeSettingsWindowPresentation::Created(hwnd)
            }
            WindowsSettingsWindowPresentation::Failed => NativeSettingsWindowPresentation::Failed,
        }
    }

    fn set_settings_window_bounds(&mut self, handle: Self::Handle, bounds: UiRect) {
        platform_window::set_pos(
            handle,
            null_mut(),
            bounds.left,
            bounds.top,
            bounds.right - bounds.left,
            bounds.bottom - bounds.top,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }

    fn destroy_settings_window(&mut self, handle: Self::Handle) {
        platform_window::destroy(handle);
    }

    fn focus_settings_window(&mut self, handle: Self::Handle) {
        platform_input::set_focus(handle);
    }

    fn track_settings_pointer_leave(&mut self, handle: Self::Handle) -> bool {
        platform_input::track_mouse_leave_and_hover(
            handle,
            platform_system_parameters::mouse_hover_time_ms(),
        )
    }

    fn capture_settings_pointer(&mut self, handle: Self::Handle) {
        platform_input::set_capture(handle);
    }

    fn release_settings_pointer(&mut self, _handle: Self::Handle) {
        platform_input::release_capture();
    }

    fn request_settings_window_repaint(&mut self, handle: Self::Handle) -> bool {
        if !platform_window::exists(handle) {
            return false;
        }
        platform_gdi::invalidate_rect(handle, null(), 1);
        true
    }

    fn request_settings_window_area_repaint(
        &mut self,
        handle: Self::Handle,
        area: Option<UiRect>,
        erase: bool,
    ) -> bool {
        if !platform_window::exists(handle) {
            return false;
        }
        let rect: RECT;
        let rect_ptr = if let Some(area) = area {
            rect = area.into();
            &rect as *const RECT
        } else {
            null()
        };
        platform_gdi::invalidate_rect(handle, rect_ptr, erase as i32);
        true
    }

    fn settings_window_layout_dpi(&mut self, handle: Self::Handle) -> u32 {
        unsafe { platform_dpi::layout_dpi_for_window(handle) }
    }

    fn settings_window_client_to_screen(
        &mut self,
        handle: Self::Handle,
        point: Point,
    ) -> Option<Point> {
        if handle.is_null() {
            return None;
        }
        let mut screen_point = windows_sys::Win32::Foundation::POINT {
            x: point.x,
            y: point.y,
        };
        if platform_window::client_to_screen(handle, &mut screen_point) {
            Some(Point {
                x: screen_point.x,
                y: screen_point.y,
            })
        } else {
            None
        }
    }

    fn settings_window_client_bounds(&mut self, handle: Self::Handle) -> Option<UiRect> {
        platform_window::client_rect(handle).map(Into::into)
    }

    fn settings_window_bounds(&mut self, handle: Self::Handle) -> Option<UiRect> {
        platform_window::window_rect(handle).map(Into::into)
    }
}

impl NativeSettingsDropdownHost for WindowsSettingsDropdownHost {
    type Handle = HWND;
    type Owner = HWND;

    fn present_settings_dropdown(
        &mut self,
        request: NativeSettingsDropdownRequest<Self::Owner>,
    ) -> NativeSettingsDropdownPresentation<Self::Handle> {
        let rect: RECT = request.anchor.into();
        let items: Vec<&str> = request.items.iter().map(String::as_str).collect();
        let handle = unsafe {
            show_settings_dropdown_popup(
                request.owner,
                request.control_id,
                &rect,
                &items,
                request.selected,
                request.width,
            )
        };
        if handle.is_null() {
            NativeSettingsDropdownPresentation::Failed
        } else {
            NativeSettingsDropdownPresentation::Created(handle)
        }
    }

    fn destroy_settings_dropdown(&mut self, handle: Self::Handle) {
        platform_window::destroy(handle);
    }

    fn settings_dropdown_bounds(&self, handle: Self::Handle) -> Option<UiRect> {
        platform_window::window_rect(handle).map(Into::into)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SettingsCtrlReg {
    pub hwnd: HWND,
    pub page: usize,
    pub bounds: UiRect,
    pub scrollable: bool,
    pub visible: bool,
}

impl SettingsCtrlReg {
    pub const fn new(
        hwnd: HWND,
        page: usize,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        scrollable: bool,
    ) -> Self {
        Self {
            hwnd,
            page,
            bounds: UiRect::new(x, y, x + w, y + h),
            scrollable,
            visible: true,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SettingsCtrlSlot {
    pub hwnd: HWND,
    pub bounds: UiRect,
    pub visible: bool,
}

pub struct SettingsUiRegistry {
    model: SettingsUiModel,
    page_ctrls: Vec<Vec<HWND>>,
}

impl SettingsUiRegistry {
    pub fn new() -> Self {
        Self {
            model: SettingsUiModel::new(),
            page_ctrls: vec![Vec::new(); SETTINGS_PAGE_COUNT],
        }
    }

    pub fn is_built(&self, page: usize) -> bool {
        self.model.is_built(page)
    }

    pub fn mark_built(&mut self, page: usize) {
        self.model.mark_built(page);
    }

    pub fn register(&mut self, reg: SettingsCtrlReg) {
        let page = reg.page.min(SETTINGS_PAGE_COUNT.saturating_sub(1));
        if let Some(list) = self.page_ctrls.get_mut(page) {
            list.push(reg.hwnd);
        }
        self.model.register(SettingsControlState {
            id: reg.hwnd as usize,
            page,
            bounds: reg.bounds,
            scrollable: reg.scrollable,
            visible: reg.visible,
        });
    }

    pub fn set_control_visible(&mut self, hwnd: HWND, visible: bool) {
        if hwnd.is_null() {
            return;
        }
        self.model.set_control_visible(hwnd as usize, visible);
    }

    pub fn set_control_bounds(&mut self, hwnd: HWND, x: i32, y: i32, w: i32, h: i32) {
        if hwnd.is_null() {
            return;
        }
        let bounds = UiRect::new(x, y, x + w, y + h);
        self.model.set_control_bounds(hwnd as usize, bounds);
    }

    pub fn page_regs(&self, page: usize) -> impl Iterator<Item = SettingsCtrlReg> + '_ {
        self.model
            .controls_for_page(page)
            .map(|control| SettingsCtrlReg {
                hwnd: control.id as HWND,
                page: control.page,
                bounds: control.bounds,
                scrollable: control.scrollable,
                visible: control.visible,
            })
    }

    pub fn scroll_ctrls_for_page(
        &self,
        page: usize,
    ) -> impl Iterator<Item = SettingsCtrlSlot> + '_ {
        self.model
            .scroll_controls_for_page(page)
            .map(|slot: SettingsScrollControlState| SettingsCtrlSlot {
                hwnd: slot.id as HWND,
                bounds: slot.bounds,
                visible: slot.visible,
            })
    }

    pub fn measured_content_total_h(&self, page: usize) -> i32 {
        self.model.measured_content_total_h(page)
    }

    pub unsafe fn clear_page(&mut self, page: usize) {
        let page = page.min(SETTINGS_PAGE_COUNT.saturating_sub(1));
        if let Some(ctrls) = self.page_ctrls.get_mut(page) {
            let mut host = WindowsSettingsControlHost::new(null_mut(), null_mut());
            for hwnd in ctrls.drain(..) {
                host.destroy_control(hwnd);
            }
        }
        self.model.clear_page(page);
    }
}

pub fn settings_viewport_rect(window_rc: &RECT) -> RECT {
    settings_viewport_rect_for_window(window_rc.into()).into()
}

pub fn settings_viewport_mask_rect(window_rc: &RECT) -> RECT {
    settings_viewport_mask_rect_for_window(window_rc.into()).into()
}

pub fn settings_child_visible(new_y: i32, h: i32, viewport: &RECT) -> bool {
    settings_child_visible_in_viewport(new_y, h, viewport.into())
}

pub(crate) fn settings_control_role_for_control(cmd: isize) -> Option<SettingsControlRole> {
    match cmd {
        IDC_SET_SAVE => Some(SettingsControlRole::Save),
        IDC_SET_CLOSE => Some(SettingsControlRole::Close),
        IDC_SET_BTN_OPENCFG => Some(SettingsControlRole::OpenConfig),
        IDC_SET_MAX
        | IDC_SET_POSMODE
        | IDC_SET_CLOUD_INTERVAL
        | IDC_SET_MULTI_SYNC_MODE
        | IDC_SET_LAN_RECEIVE_MODE
        | IDC_SET_HOTKEY_MOD
        | IDC_SET_HOTKEY_KEY
        | IDC_SET_PASTE_SOUND_KIND
        | IDC_SET_PLAIN_HK_MOD
        | IDC_SET_PLAIN_HK_KEY
        | IDC_SET_SEARCH_ENGINE
        | IDC_SET_OCR_PROVIDER
        | IDC_SET_TRANSLATE_PROVIDER
        | IDC_SET_TRANSLATE_TARGET
        | IDC_SET_VV_SOURCE
        | IDC_SET_VV_GROUP => Some(SettingsControlRole::Dropdown),
        IDC_SET_AUTOSTART
        | IDC_SET_SILENTSTART
        | IDC_SET_TRAYICON
        | IDC_SET_CLOSETRAY
        | IDC_SET_CLICK_HIDE
        | IDC_SET_PASTE_MOVE_TOP
        | IDC_SET_DEDUPE_FILTER
        | IDC_SET_PERSIST_SEARCH
        | IDC_SET_PASTE_SOUND_ENABLE
        | IDC_SET_SKIP_WINDOW_ENABLE
        | IDC_SET_RICH_TEXT
        | IDC_SET_AUTOHIDE_BLUR
        | IDC_SET_EDGEHIDE
        | IDC_SET_HOVERPREVIEW
        | IDC_SET_VV_MODE
        | IDC_SET_IMAGE_PREVIEW
        | IDC_SET_QUICK_DELETE
        | IDC_SET_GROUP_ENABLE
        | IDC_SET_GROUP_TYPE_FILTER
        | IDC_SET_CLOUD_ENABLE
        | IDC_SET_LAN_ENABLE
        | IDC_SET_HOTKEY_ENABLE
        | IDC_SET_PLAIN_HK_ENABLE
        | IDC_SET_PLUGIN_AI_CLEAN
        | IDC_SET_PLUGIN_SUPER_MAIL_MERGE
        | IDC_SET_PLUGIN_SEARCH
        | IDC_SET_PLUGIN_QR_QUICK
        | IDC_SET_PLUGIN_WPS_TASKPANE => Some(SettingsControlRole::Toggle),
        _ => None,
    }
}

pub(crate) fn settings_command_for_control(cmd: isize) -> Option<Command> {
    let role = settings_control_role_for_control(cmd)?;
    let command = settings_command_for_control_role(role, cmd as i64);
    let direct_settings_action = native_host_settings_action_button_specs()
        .into_iter()
        .any(|spec| spec.action.command() == command);
    let shared_settings_control = native_host_settings_toggle_specs()
        .into_iter()
        .any(|spec| spec.action.role() == role)
        || native_host_settings_dropdown_specs()
            .into_iter()
            .any(|spec| spec.action.role() == role)
        || native_host_settings_control_button_specs()
            .into_iter()
            .any(|spec| spec.action.role() == role);
    (direct_settings_action || shared_settings_control).then_some(command)
}

fn native_settings_group_action_for_settings_action(
    action: SettingsAction,
) -> Option<NativeHostSettingsGroupAction> {
    match action {
        SettingsAction::AddGroup => Some(NativeHostSettingsGroupAction::Add),
        SettingsAction::RenameGroup => Some(NativeHostSettingsGroupAction::Rename),
        SettingsAction::DeleteGroup => Some(NativeHostSettingsGroupAction::Delete),
        SettingsAction::MoveGroupUp => Some(NativeHostSettingsGroupAction::MoveUp),
        SettingsAction::MoveGroupDown => Some(NativeHostSettingsGroupAction::MoveDown),
        SettingsAction::ShowRecordGroups => Some(NativeHostSettingsGroupAction::ShowRecords),
        SettingsAction::ShowPhraseGroups => Some(NativeHostSettingsGroupAction::ShowPhrases),
        _ => None,
    }
}

fn native_settings_platform_action_for_settings_action(
    action: SettingsAction,
) -> Option<NativeHostSettingsPlatformAction> {
    match action {
        SettingsAction::OpenWpsTaskpaneDocs => {
            Some(NativeHostSettingsPlatformAction::OpenWpsTaskpaneDocs)
        }
        SettingsAction::OpenSourceRepository => {
            Some(NativeHostSettingsPlatformAction::OpenSourceRepository)
        }
        SettingsAction::CheckForUpdates => Some(NativeHostSettingsPlatformAction::CheckForUpdates),
        _ => None,
    }
}

fn settings_action_is_declared_by_native_specs(action: SettingsAction) -> bool {
    if let Some(group_action) = native_settings_group_action_for_settings_action(action) {
        return native_host_settings_group_button_specs()
            .into_iter()
            .any(|spec| spec.action == group_action);
    }
    if let Some(platform_action) = native_settings_platform_action_for_settings_action(action) {
        return native_host_settings_platform_button_specs()
            .into_iter()
            .any(|spec| spec.action == platform_action);
    }
    true
}

pub(crate) fn settings_action_for_control(
    control_id: isize,
    notification: u16,
) -> Option<SettingsAction> {
    let action = match control_id {
        IDC_SET_HK_RECORD => Some(SettingsAction::ToggleHotkeyRecording),
        IDC_SET_GROUP_ADD => Some(SettingsAction::AddGroup),
        IDC_SET_GROUP_RENAME => Some(SettingsAction::RenameGroup),
        IDC_SET_GROUP_DELETE => Some(SettingsAction::DeleteGroup),
        IDC_SET_GROUP_UP => Some(SettingsAction::MoveGroupUp),
        IDC_SET_GROUP_DOWN => Some(SettingsAction::MoveGroupDown),
        IDC_SET_GROUP_LIST if notification as u32 == LBN_SELCHANGE => {
            Some(SettingsAction::GroupSelectionChanged)
        }
        IDC_SET_GROUP_VIEW_RECORDS => Some(SettingsAction::ShowRecordGroups),
        IDC_SET_GROUP_VIEW_PHRASES => Some(SettingsAction::ShowPhraseGroups),
        IDC_SET_PASTE_SOUND_PICK => Some(SettingsAction::PickPasteSound),
        IDC_SET_SKIP_WINDOW_CAPTURE => Some(SettingsAction::CaptureSkippedWindowClass),
        IDC_SET_SEARCH_ENGINE_RESET => Some(SettingsAction::RestoreSearchEnginePreset),
        IDC_SET_OCR_WECHAT_DETECT => Some(SettingsAction::DetectOcrRuntime),
        IDC_SET_PLUGIN_MAILMERGE => Some(SettingsAction::OpenMailMerge),
        IDC_SET_WPS_TASKPANE_DOCS => Some(SettingsAction::OpenWpsTaskpaneDocs),
        IDC_SET_OPEN_SOURCE => Some(SettingsAction::OpenSourceRepository),
        IDC_SET_OPEN_UPDATE => Some(SettingsAction::CheckForUpdates),
        IDC_SET_CLIPBOARD_HISTORY_DISABLE => Some(SettingsAction::DisableSystemClipboardHistory),
        IDC_SET_CLIPBOARD_HISTORY_ENABLE => Some(SettingsAction::EnableSystemClipboardHistory),
        IDC_SET_RESTART_EXPLORER => Some(SettingsAction::RestartSystemShell),
        IDC_SET_CLOUD_SYNC_NOW => Some(SettingsAction::SyncWebDavNow),
        IDC_SET_CLOUD_UPLOAD_CFG => Some(SettingsAction::UploadWebDavConfig),
        IDC_SET_CLOUD_APPLY_CFG => Some(SettingsAction::ApplyWebDavConfig),
        IDC_SET_CLOUD_RESTORE_BACKUP => Some(SettingsAction::RestoreWebDavBackup),
        IDC_SET_LAN_REFRESH => Some(SettingsAction::RefreshLanDevices),
        IDC_SET_LAN_PAIR => Some(SettingsAction::PairLanDevice),
        IDC_SET_LAN_ACCEPT_PAIR => Some(SettingsAction::AcceptLanPairing),
        IDC_SET_LAN_REJECT_PAIR => Some(SettingsAction::RejectLanPairing),
        IDC_SET_LAN_COPY_PAIR => Some(SettingsAction::CopyLanPairUrl),
        IDC_SET_LAN_COPY_SETUP => Some(SettingsAction::CopyLanSetupUrl),
        IDC_SET_LAN_DOCS => Some(SettingsAction::OpenLanSetupPage),
        _ => None,
    }?;
    settings_action_is_declared_by_native_specs(action).then_some(action)
}

pub(crate) fn settings_event_from_window_message(
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> Option<UiEvent> {
    if msg == WM_SETTINGS_DROPDOWN_SELECTED {
        Some(UiEvent::ControlSelectionChanged {
            control_id: wparam as u32,
            index: lparam as usize,
        })
    } else {
        None
    }
}

pub(crate) fn settings_window_host_event_from_message(
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> Option<UiEvent> {
    settings_event_from_window_message(msg, wparam, lparam)
        .or_else(|| platform_ui_event::from_window_message(msg, wparam, lparam))
}

pub(crate) fn settings_page_to_sync_after_toggle(control_id: isize) -> Option<usize> {
    if control_id == IDC_SET_PASTE_SOUND_ENABLE
        || control_id == IDC_SET_SKIP_WINDOW_ENABLE
        || control_id == IDC_SET_DEDUPE_FILTER
    {
        Some(SettingsPage::General.index())
    } else if control_id == IDC_SET_CLOUD_ENABLE || control_id == IDC_SET_LAN_ENABLE {
        Some(SettingsPage::Cloud.index())
    } else if matches!(
        control_id,
        IDC_SET_PLUGIN_SEARCH
            | IDC_SET_PLUGIN_AI_CLEAN
            | IDC_SET_PLUGIN_QR_QUICK
            | IDC_SET_PLUGIN_WPS_TASKPANE
            | IDC_SET_PLUGIN_SUPER_MAIL_MERGE
    ) {
        Some(SettingsPage::Plugin.index())
    } else {
        None
    }
}

pub unsafe fn create_settings_component(
    parent: HWND,
    text: &str,
    id: isize,
    kind: SettingsComponentKind,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    font: *mut c_void,
) -> HWND {
    create_settings_component_from_spec(
        parent,
        &SettingsControlSpec::action(kind, id as i64, text, UiRect::new(x, y, x + w, y + h)),
        font,
    )
}

pub(crate) struct WindowsSettingsControlHost {
    parent: HWND,
    font: *mut c_void,
}

impl WindowsSettingsControlHost {
    pub(crate) const fn new(parent: HWND, font: *mut c_void) -> Self {
        Self { parent, font }
    }
}

impl NativeSettingsControlHost for WindowsSettingsControlHost {
    type Handle = HWND;

    fn create_control(&mut self, spec: &SettingsControlSpec) -> Self::Handle {
        let class_name = WindowsNativeControlMapper.class_name(spec.kind);
        let translated = translate(&spec.text);
        let hwnd = platform_window::create_window_ex(
            0,
            to_wide(class_name).as_ptr(),
            to_wide(translated.as_ref()).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_OWNERDRAW as u32),
            spec.bounds.left,
            spec.bounds.top,
            spec.width(),
            spec.height(),
            self.parent,
            spec.id.unwrap_or_default() as usize as _,
            platform_window::module_handle(),
            null_mut(),
        );
        if !hwnd.is_null() {
            platform_window::send_message(hwnd, WM_SETFONT, self.font as usize, 1);
        }
        hwnd
    }

    fn destroy_control(&mut self, handle: Self::Handle) {
        if platform_window::exists(handle) {
            platform_window::destroy(handle);
        }
    }

    fn control_exists(&self, handle: Self::Handle) -> bool {
        platform_window::exists(handle)
    }

    fn set_control_visible(&mut self, handle: Self::Handle, visible: bool) {
        if !handle.is_null() {
            platform_window::set_visible(handle, visible);
        }
    }

    fn set_control_enabled(&mut self, handle: Self::Handle, enabled: bool) {
        if !handle.is_null() {
            platform_window::set_enabled(handle, enabled);
        }
    }

    fn set_control_bounds(&mut self, handle: Self::Handle, bounds: UiRect) {
        if !handle.is_null() {
            platform_window::move_window(
                handle,
                bounds.left,
                bounds.top,
                bounds.width(),
                bounds.height(),
                false,
            );
        }
    }

    fn control_at_point(&self, point: Point) -> Option<Self::Handle> {
        let handle = platform_window::child_from_point_ex(
            self.parent,
            POINT {
                x: point.x,
                y: point.y,
            },
            platform_window::CHILD_FROM_POINT_SKIP_DISABLED
                | platform_window::CHILD_FROM_POINT_SKIP_INVISIBLE,
        );
        if handle.is_null() || handle == self.parent {
            return None;
        }
        if settings_control_is_viewport_child(handle) {
            let mut child_point = POINT {
                x: point.x,
                y: point.y,
            };
            if platform_window::client_to_screen(self.parent, &mut child_point)
                && platform_window::screen_to_client(handle, &mut child_point)
            {
                let nested = platform_window::child_from_point_ex(
                    handle,
                    child_point,
                    platform_window::CHILD_FROM_POINT_SKIP_DISABLED
                        | platform_window::CHILD_FROM_POINT_SKIP_INVISIBLE,
                );
                if !nested.is_null() && nested != handle {
                    return Some(nested);
                }
            }
        }
        Some(handle)
    }

    fn control_screen_bounds(&self, handle: Self::Handle) -> Option<UiRect> {
        platform_window::window_rect(handle).map(Into::into)
    }

    fn control_text(&self, handle: Self::Handle) -> String {
        if handle.is_null() {
            String::new()
        } else {
            platform_window::text(handle)
        }
    }

    fn set_control_text(&mut self, handle: Self::Handle, text: &str) {
        if handle.is_null() {
            return;
        }
        let class_name = platform_window::class_name(handle);
        let translated = if matches!(class_name.as_str(), "BUTTON" | "STATIC") {
            translate(text).into_owned()
        } else {
            text.to_string()
        };
        if platform_window::text(handle) != translated {
            platform_window::set_text(handle, &translated);
        }
    }

    fn request_control_repaint(&mut self, handle: Self::Handle) -> bool {
        if handle.is_null() {
            return false;
        }
        platform_gdi::invalidate_rect(handle, null(), 1);
        true
    }
}

pub unsafe fn create_settings_component_from_spec(
    parent: HWND,
    spec: &SettingsControlSpec,
    font: *mut c_void,
) -> HWND {
    WindowsSettingsControlHost::new(parent, font).create_control(spec)
}

pub(crate) fn settings_host_set_visible(hwnd: HWND, visible: bool) {
    WindowsSettingsControlHost::new(null_mut(), null_mut()).set_control_visible(hwnd, visible);
}

pub(crate) fn settings_host_exists(hwnd: HWND) -> bool {
    WindowsSettingsControlHost::new(null_mut(), null_mut()).control_exists(hwnd)
}

pub(crate) fn settings_host_set_visible_enabled(hwnd: HWND, visible: bool, enabled: bool) {
    let mut host = WindowsSettingsControlHost::new(null_mut(), null_mut());
    host.set_control_visible(hwnd, visible);
    host.set_control_enabled(hwnd, visible && enabled);
}

pub(crate) fn settings_host_set_enabled(hwnd: HWND, enabled: bool) {
    WindowsSettingsControlHost::new(null_mut(), null_mut()).set_control_enabled(hwnd, enabled);
}

pub(crate) fn settings_host_set_bounds(hwnd: HWND, bounds: UiRect) {
    WindowsSettingsControlHost::new(null_mut(), null_mut()).set_control_bounds(hwnd, bounds);
}

pub(crate) fn settings_host_screen_bounds(hwnd: HWND) -> Option<UiRect> {
    WindowsSettingsControlHost::new(null_mut(), null_mut()).control_screen_bounds(hwnd)
}

pub(crate) fn settings_host_control_at_point(parent: HWND, point: Point) -> Option<HWND> {
    WindowsSettingsControlHost::new(parent, null_mut()).control_at_point(point)
}

pub(crate) fn settings_host_text(hwnd: HWND) -> String {
    WindowsSettingsControlHost::new(null_mut(), null_mut()).control_text(hwnd)
}

pub(crate) fn settings_host_set_text(hwnd: HWND, text: &str) {
    WindowsSettingsControlHost::new(null_mut(), null_mut()).set_control_text(hwnd, text);
}

pub(crate) fn settings_host_request_repaint(hwnd: HWND) -> bool {
    WindowsSettingsControlHost::new(null_mut(), null_mut()).request_control_repaint(hwnd)
}

pub(crate) fn settings_window_client_to_screen(hwnd: HWND, point: Point) -> Option<Point> {
    WindowsSettingsWindowHost::new(None).settings_window_client_to_screen(hwnd, point)
}

pub(crate) fn settings_window_client_bounds(hwnd: HWND) -> Option<UiRect> {
    WindowsSettingsWindowHost::new(None).settings_window_client_bounds(hwnd)
}

pub(crate) fn settings_window_bounds(hwnd: HWND) -> Option<UiRect> {
    WindowsSettingsWindowHost::new(None).settings_window_bounds(hwnd)
}

pub(crate) fn settings_window_track_pointer_leave(hwnd: HWND) -> bool {
    WindowsSettingsWindowHost::new(None).track_settings_pointer_leave(hwnd)
}

pub(crate) fn settings_window_layout_dpi(hwnd: HWND) -> u32 {
    WindowsSettingsWindowHost::new(None).settings_window_layout_dpi(hwnd)
}

pub(crate) fn settings_window_request_area_repaint(
    hwnd: HWND,
    area: Option<UiRect>,
    erase: bool,
) -> bool {
    WindowsSettingsWindowHost::new(None).request_settings_window_area_repaint(hwnd, area, erase)
}

pub unsafe fn set_settings_font(hwnd: HWND, hfont: *mut c_void) {
    if !hwnd.is_null() && !hfont.is_null() {
        platform_window::send_message(hwnd, WM_SETFONT, hfont as usize, 1);
    }
}

pub unsafe fn create_settings_button(
    parent: HWND,
    text: &str,
    id: isize,
    x: i32,
    y: i32,
    w: i32,
    font: *mut c_void,
) -> HWND {
    let h = platform_dpi::scale_for_window(parent, 32);
    create_settings_component_from_spec(
        parent,
        &SettingsControlSpec::action(
            SettingsComponentKind::Button,
            id as i64,
            text,
            UiRect::new(x, y, x + w, y + h),
        ),
        font,
    )
}

pub unsafe fn create_settings_small_button(
    parent: HWND,
    text: &str,
    id: isize,
    x: i32,
    y: i32,
    w: i32,
    font: *mut c_void,
) -> HWND {
    create_settings_button(parent, text, id, x, y, w, font)
}

pub unsafe fn create_settings_dropdown_button(
    parent: HWND,
    text: &str,
    id: isize,
    x: i32,
    y: i32,
    w: i32,
    font: *mut c_void,
) -> HWND {
    let h = platform_dpi::scale_for_window(parent, 32);
    create_settings_component_from_spec(
        parent,
        &SettingsControlSpec::action(
            SettingsComponentKind::Dropdown,
            id as i64,
            text,
            UiRect::new(x, y, x + w, y + h),
        ),
        font,
    )
}

pub unsafe fn create_settings_toggle_plain(
    parent: HWND,
    text: &str,
    id: isize,
    x: i32,
    y: i32,
    w: i32,
    font: *mut c_void,
) -> (HWND, HWND, SettingsToggleRowLayout) {
    const SS_CENTERIMAGE: u32 = 0x0200;
    let toggle_w = platform_dpi::scale_for_window(parent, 44);
    let toggle_h = platform_dpi::scale_for_window(parent, 24);
    let row_h = platform_dpi::scale_for_window(parent, 32);
    let gap = platform_dpi::scale_for_window(parent, 12);
    let layout = settings_toggle_row_layout_for_rect(
        UiRect::new(x, y, x + w, y + row_h),
        toggle_w,
        toggle_h,
        gap,
        platform_dpi::scale_for_window(parent, 40),
    );
    let label_spec = SettingsControlSpec::label(text, layout.label_rect);
    let label_text = translate(&label_spec.text);
    let label = platform_window::create_window_ex(
        0,
        to_wide(WindowsNativeControlMapper.class_name(label_spec.kind)).as_ptr(),
        to_wide(label_text.as_ref()).as_ptr(),
        WS_CHILD | WS_VISIBLE | SS_CENTERIMAGE,
        label_spec.bounds.left,
        label_spec.bounds.top,
        label_spec.width(),
        label_spec.height(),
        parent,
        null_mut(),
        platform_window::module_handle(),
        null(),
    );
    set_settings_font(label, font);

    let btn = create_settings_component_from_spec(
        parent,
        &SettingsControlSpec::action(
            SettingsComponentKind::Toggle,
            id as i64,
            "",
            layout.toggle_rect,
        ),
        font,
    );
    (label, btn, layout)
}

pub unsafe fn create_settings_fonts(hwnd: HWND) -> (*mut c_void, *mut c_void, *mut c_void) {
    let nav_size = platform_dpi::scale_for_window(hwnd, 18);
    let ui_size = platform_dpi::scale_for_window(hwnd, 14);
    let title_size = platform_dpi::scale_for_window(hwnd, 20);
    let nav: *mut c_void = platform_gdi::create_font_w(
        -nav_size,
        0,
        0,
        0,
        400,
        0,
        0,
        0,
        1,
        0,
        0,
        5,
        0,
        to_wide(ui_icon_font_family()).as_ptr(),
    ) as _;
    let ui: *mut c_void = platform_gdi::create_font_w(
        -ui_size,
        0,
        0,
        0,
        400,
        0,
        0,
        0,
        1,
        0,
        0,
        5,
        0,
        to_wide(ui_text_font_family()).as_ptr(),
    ) as _;
    let title: *mut c_void = platform_gdi::create_font_w(
        -title_size,
        0,
        0,
        0,
        600,
        0,
        0,
        0,
        1,
        0,
        0,
        5,
        0,
        to_wide(ui_display_font_family()).as_ptr(),
    ) as _;
    let default_ui: *mut c_void = if ui.is_null() {
        platform_gdi::get_stock_object(DEFAULT_GUI_FONT) as _
    } else {
        ui
    };
    let default_title: *mut c_void = if title.is_null() {
        platform_gdi::get_stock_object(DEFAULT_GUI_FONT) as _
    } else {
        title
    };
    (nav, default_ui, default_title)
}

pub unsafe fn create_settings_label(
    parent: HWND,
    text: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    font: *mut c_void,
) -> HWND {
    const SS_CENTERIMAGE: u32 = 0x0200;
    const SS_ENDELLIPSIS: u32 = 0x4000;
    let spec = SettingsControlSpec::label(text, UiRect::new(x, y, x + w, y + h));
    let translated = translate(&spec.text);
    let hwnd = platform_window::create_window_ex(
        0,
        to_wide(WindowsNativeControlMapper.class_name(spec.kind)).as_ptr(),
        to_wide(translated.as_ref()).as_ptr(),
        WS_CHILD | WS_VISIBLE | SS_CENTERIMAGE | SS_NOPREFIX_STYLE | SS_ENDELLIPSIS,
        spec.bounds.left,
        spec.bounds.top,
        spec.width(),
        spec.height(),
        parent,
        null_mut(),
        platform_window::module_handle(),
        null(),
    );
    if !hwnd.is_null() {
        platform_window::send_message(hwnd, WM_SETFONT, font as usize, 1);
    }
    hwnd
}

unsafe fn create_settings_label_wrap(
    parent: HWND,
    text: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    font: *mut c_void,
) -> HWND {
    let spec = SettingsControlSpec::label(text, UiRect::new(x, y, x + w, y + h));
    let translated = translate(&spec.text);
    let hwnd = platform_window::create_window_ex(
        0,
        to_wide(WindowsNativeControlMapper.class_name(spec.kind)).as_ptr(),
        to_wide(translated.as_ref()).as_ptr(),
        WS_CHILD | WS_VISIBLE | SS_EDITCONTROL_STYLE | SS_NOPREFIX_STYLE,
        spec.bounds.left,
        spec.bounds.top,
        spec.width(),
        spec.height(),
        parent,
        null_mut(),
        platform_window::module_handle(),
        null(),
    );
    if !hwnd.is_null() {
        platform_window::send_message(hwnd, WM_SETFONT, font as usize, 1);
    }
    hwnd
}

pub unsafe fn settings_measure_text_height(
    parent: HWND,
    text: &str,
    w: i32,
    font: *mut c_void,
    min_h: i32,
) -> i32 {
    let hdc = platform_gdi::get_dc(parent);
    if hdc.is_null() {
        return min_h.max(24);
    }
    let old = if !font.is_null() {
        platform_gdi::select_object(hdc, font)
    } else {
        null_mut()
    };
    let horizontal_pad = platform_dpi::scale_for_window(parent, 4);
    let vertical_pad = platform_dpi::scale_for_window(parent, 8);
    let mut rc = RECT {
        left: 0,
        top: 0,
        right: (w - horizontal_pad).max(1),
        bottom: 0,
    };
    let translated = translate(text);
    let wt = to_wide(translated.as_ref());
    platform_gdi::draw_text(
        hdc,
        wt.as_ptr(),
        -1,
        &mut rc,
        DT_LEFT
            | DT_WORDBREAK
            | DT_NOPREFIX
            | DT_EDITCONTROL_FLAG
            | DT_EXTERNALLEADING_FLAG
            | DT_CALCRECT_FLAG,
    );
    if !old.is_null() {
        platform_gdi::select_object(hdc, old);
    }
    platform_gdi::release_dc(parent, hdc);
    min_h.max((rc.bottom - rc.top) + vertical_pad)
}

pub unsafe fn create_settings_label_auto(
    parent: HWND,
    text: &str,
    x: i32,
    y: i32,
    w: i32,
    min_h: i32,
    font: *mut c_void,
) -> (HWND, i32) {
    let h = settings_measure_text_height(parent, text, w, font, min_h);
    let hwnd = create_settings_label_wrap(parent, text, x, y, w, h, font);
    (hwnd, h)
}

pub unsafe fn create_settings_edit(
    parent: HWND,
    text: &str,
    id: isize,
    x: i32,
    y: i32,
    w: i32,
    font: *mut c_void,
) -> HWND {
    let h = platform_dpi::scale_for_window(parent, 28);
    let spec = SettingsControlSpec::text_input(id as i64, text, UiRect::new(x, y, x + w, y + h));
    let style = WS_CHILD | WS_VISIBLE | WS_TABSTOP | (ES_AUTOHSCROLL as u32);
    let hwnd = platform_window::create_window_ex(
        WS_EX_CLIENTEDGE,
        to_wide(WindowsNativeControlMapper.class_name(spec.kind)).as_ptr(),
        to_wide(&spec.text).as_ptr(),
        style,
        spec.bounds.left,
        spec.bounds.top,
        spec.width(),
        spec.height(),
        parent,
        spec.id.unwrap_or_default() as usize as _,
        platform_window::module_handle(),
        null(),
    );
    if !hwnd.is_null() {
        platform_window::send_message(hwnd, WM_SETFONT, font as usize, 1);
        let theme = if platform_appearance::is_dark_mode() {
            "DarkMode_Explorer"
        } else {
            "Explorer"
        };
        platform_appearance::set_window_theme(hwnd, theme);
        platform_window::send_message(
            hwnd,
            EM_SETMARGINS_MSG,
            (EC_LEFTMARGIN | EC_RIGHTMARGIN) as WPARAM,
            ((platform_dpi::scale_for_window(parent, 6) & 0xffff)
                | ((platform_dpi::scale_for_window(parent, 6) & 0xffff) << 16))
                as LPARAM,
        );
    }
    hwnd
}

pub unsafe fn create_settings_password_edit(
    parent: HWND,
    text: &str,
    id: isize,
    x: i32,
    y: i32,
    w: i32,
    font: *mut c_void,
) -> HWND {
    let hwnd = create_settings_edit(parent, text, id, x, y, w, font);
    if !hwnd.is_null() {
        platform_window::send_message(hwnd, EM_SETPASSWORDCHAR_MSG, '*' as usize, 0);
        platform_gdi::invalidate_rect(hwnd, null(), 1);
    }
    hwnd
}

pub unsafe fn create_settings_listbox(
    parent: HWND,
    id: isize,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    font: *mut c_void,
) -> HWND {
    let hwnd = platform_window::create_window_ex(
        WS_EX_CLIENTEDGE,
        to_wide("LISTBOX").as_ptr(),
        to_wide("").as_ptr(),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | (LBS_NOTIFY as u32) | WS_VSCROLL,
        x,
        y,
        w,
        h,
        parent,
        id as usize as _,
        platform_window::module_handle(),
        null(),
    );
    if !hwnd.is_null() {
        platform_window::send_message(hwnd, WM_SETFONT, font as usize, 1);
        platform_appearance::set_window_theme(hwnd, "Explorer");
        platform_window::show_scrollbar(hwnd, SB_VERT, false);
        platform_window::show_scrollbar(hwnd, SB_HORZ, false);
    }
    hwnd
}

pub unsafe fn get_ctrl_text_wide(hwnd: HWND) -> Vec<u16> {
    settings_host_text(hwnd).encode_utf16().collect()
}

pub unsafe fn draw_text_wide_centered(
    hdc: *mut c_void,
    text_w: &[u16],
    rc: &RECT,
    color: u32,
    size: i32,
    font_name: &str,
) {
    let hdc = hdc as _;
    let font: *mut c_void = create_font_px(font_name, size, 400);
    let old = platform_gdi::select_object(hdc, font as _);
    platform_gdi::set_bk_mode(hdc, 1);
    platform_gdi::set_text_color(hdc, color);
    platform_gdi::draw_text(
        hdc,
        text_w.as_ptr(),
        text_w.len() as i32,
        rc as *const _ as *mut _,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE,
    );
    platform_gdi::select_object(hdc, old);
    platform_gdi::delete_object(font as _);
}

pub unsafe fn draw_settings_toggle_component(
    hdc: *mut c_void,
    rc: &RECT,
    hover: bool,
    checked: bool,
    th: Theme,
) {
    let bg = platform_gdi::create_solid_brush(th.surface);
    platform_gdi::fill_rect(hdc, rc, bg);
    platform_gdi::delete_object(bg as _);

    let row_h = (rc.bottom - rc.top).max(24);
    let row_w = (rc.right - rc.left).max(48);
    let thh = ((row_h * 20) / 32).clamp(20, row_h - 4);
    let tw = ((thh * 40) / 20).clamp(thh + 12, row_w - 6);
    let cx = rc.left + (rc.right - rc.left - tw) / 2;
    let cy = rc.top + (rc.bottom - rc.top - thh) / 2;
    let track = RECT {
        left: cx,
        top: cy,
        right: cx + tw,
        bottom: cy + thh,
    };
    let radius = (thh / 2).max(6);

    if checked {
        draw_round_rect(hdc, &track, th.accent, th.accent, radius);
        let k = ((thh * 14) / 20).max(12);
        let ky = cy + (thh - k) / 2;
        let knob_pad = ((thh - k) / 2).max(3);
        let krc = RECT {
            left: cx + tw - k - knob_pad,
            top: ky,
            right: cx + tw - knob_pad,
            bottom: ky + k,
        };
        draw_round_rect(hdc, &krc, rgb(255, 255, 255), rgb(255, 255, 255), 7);
    } else {
        let border = if hover {
            rgb(28, 28, 28)
        } else {
            rgb(136, 136, 136)
        };
        let fill = settings_toggle_off_track_fill(th.bg);
        draw_round_rect(hdc, &track, fill, border, radius);
        let k = ((thh * 12) / 20).max(10);
        let ky = cy + (thh - k) / 2;
        let knob_pad = ((thh - k) / 2).max(4);
        let krc = RECT {
            left: cx + knob_pad,
            top: ky,
            right: cx + knob_pad + k,
            bottom: ky + k,
        };
        let knob_color = if hover {
            rgb(28, 28, 28)
        } else {
            rgb(102, 102, 102)
        };
        draw_round_rect(hdc, &krc, knob_color, knob_color, 6);
    }
}

pub fn settings_toggle_off_track_fill(bg: u32) -> u32 {
    if bg == rgb(32, 32, 32) {
        rgb(58, 58, 58)
    } else {
        rgb(242, 242, 242)
    }
}

pub unsafe fn draw_settings_button_component(
    hdc: *mut c_void,
    rc: &RECT,
    text: &str,
    kind: SettingsComponentKind,
    hover: bool,
    pressed: bool,
    th: Theme,
) {
    let rr = RECT {
        left: rc.left + 1,
        top: rc.top + 1,
        right: rc.right - 1,
        bottom: rc.bottom - 1,
    };
    let text_px = 14;
    match kind {
        SettingsComponentKind::Dropdown => {
            draw_settings_dropdown_button(hdc, &rr, text, hover, pressed, th);
        }
        SettingsComponentKind::AccentButton => {
            let fill = if pressed {
                th.accent_pressed
            } else if hover {
                th.accent_hover
            } else {
                th.accent
            };
            draw_round_rect(hdc, &rr, fill, fill, 4);
            draw_text_ex(
                hdc,
                text,
                &rr,
                rgb(255, 255, 255),
                text_px,
                false,
                true,
                ui_text_font_family(),
            );
        }
        SettingsComponentKind::Button => {
            let fill = if pressed {
                th.button_pressed
            } else if hover {
                th.button_hover
            } else {
                th.button_bg
            };
            let border = if pressed || hover {
                rgb(196, 196, 196)
            } else {
                rgb(204, 204, 204)
            };
            draw_round_rect(hdc, &rr, fill, border, 4);
            draw_text_ex(
                hdc,
                text,
                &rr,
                th.text,
                text_px,
                false,
                true,
                ui_text_font_family(),
            );
        }
        SettingsComponentKind::Toggle => {}
        SettingsComponentKind::Label | SettingsComponentKind::TextInput => {}
    }
}

pub unsafe fn draw_settings_dropdown_button(
    hdc: *mut c_void,
    rc: &RECT,
    text: &str,
    _hover: bool,
    pressed: bool,
    th: Theme,
) {
    let rr = RECT {
        left: rc.left + 1,
        top: rc.top + 1,
        right: rc.right - 1,
        bottom: rc.bottom - 1,
    };
    let control_h = (rr.bottom - rr.top).max(24);
    let text_px = 14;
    let arrow_px = 10;
    let text_pad = (control_h * 12 / 32).max(10);
    let arrow_w = (control_h * 20 / 32).max(18);
    let fill = if pressed {
        th.button_pressed
    } else {
        th.surface
    };
    let border = th.control_stroke;
    draw_round_rect(hdc, &rr, fill, border, 6);

    let text_rc = RECT {
        left: rr.left + text_pad,
        top: rr.top,
        right: rr.right - arrow_w,
        bottom: rr.bottom,
    };
    draw_text_ex(
        hdc,
        text,
        &text_rc,
        th.text,
        text_px,
        false,
        false,
        ui_text_font_family(),
    );

    let arrow_rc = RECT {
        left: rr.right - arrow_w,
        top: rr.top,
        right: rr.right - (control_h * 8 / 32).max(6),
        bottom: rr.bottom,
    };
    draw_text_ex(
        hdc,
        "\u{25BE}",
        &arrow_rc,
        th.text_muted,
        arrow_px,
        false,
        true,
        ui_icon_font_family(),
    );
}

unsafe fn apply_dark_mode_to_window(hwnd: HWND) {
    platform_appearance::set_dark_frame(hwnd, platform_appearance::is_dark_mode());
    let theme_name = if platform_appearance::is_dark_mode() {
        "DarkMode_Explorer"
    } else {
        "Explorer"
    };
    platform_appearance::set_window_theme(hwnd, theme_name);
}

unsafe fn apply_window_corner_preference(hwnd: HWND) {
    platform_appearance::set_rounded_corners(hwnd);
}

struct DropdownPopupState {
    parent: HWND,
    control_id: isize,
    items: Vec<String>,
    selected: i32,
    interaction: SettingsDropdownInteractionState,
}

fn dropdown_index_from_y(st: &DropdownPopupState, y: i32) -> i32 {
    st.interaction.index_from_y(y).unwrap_or(-1)
}

fn dropdown_max_scroll(st: &DropdownPopupState) -> i32 {
    st.interaction.max_scroll()
}

fn dropdown_scroll_by_wheel(st: &mut DropdownPopupState, delta: i32) -> bool {
    st.interaction.scroll_by_wheel(delta)
}

unsafe fn dropdown_popup_state(hwnd: HWND) -> *mut DropdownPopupState {
    platform_window::user_data(hwnd) as *mut DropdownPopupState
}

unsafe fn handle_dropdown_pointer_move(hwnd: HWND, y: i32) -> LRESULT {
    let ptr = dropdown_popup_state(hwnd);
    if ptr.is_null() {
        return 0;
    }
    let st = &mut *ptr;
    let hover = dropdown_index_from_y(st, y);
    if hover != st.interaction.hover {
        st.interaction.hover = hover;
        platform_gdi::invalidate_rect(hwnd, null(), 0);
    }
    0
}

unsafe fn handle_dropdown_mouse_wheel(hwnd: HWND, delta: i32) -> LRESULT {
    let ptr = dropdown_popup_state(hwnd);
    if ptr.is_null() {
        return 0;
    }
    let st = &mut *ptr;
    if dropdown_scroll_by_wheel(st, delta) {
        platform_gdi::invalidate_rect(hwnd, null(), 0);
    }
    0
}

unsafe fn handle_dropdown_pointer_button(hwnd: HWND, y: i32) -> LRESULT {
    let ptr = dropdown_popup_state(hwnd);
    if !ptr.is_null() {
        let st = &mut *ptr;
        let idx = dropdown_index_from_y(st, y);
        if idx >= 0 {
            platform_window::send_message(
                st.parent,
                WM_SETTINGS_DROPDOWN_SELECTED,
                st.control_id as usize,
                idx as isize,
            );
        }
    }
    platform_window::destroy(hwnd);
    0
}

unsafe fn dispatch_dropdown_ui_event(hwnd: HWND, event: UiEvent) -> Option<LRESULT> {
    match event {
        UiEvent::PointerMove { position } => Some(handle_dropdown_pointer_move(hwnd, position.y)),
        UiEvent::MouseWheel { delta } => Some(handle_dropdown_mouse_wheel(hwnd, delta)),
        UiEvent::PointerButton {
            position,
            button: UiMouseButton::Left,
            ..
        } => Some(handle_dropdown_pointer_button(hwnd, position.y)),
        _ => None,
    }
}

fn dropdown_window_host_event_from_message(
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> Option<UiEvent> {
    platform_ui_event::from_window_message(msg, wparam, lparam)
}

unsafe extern "system" fn dropdown_popup_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if let Some(event) = dropdown_window_host_event_from_message(msg, wparam, lparam) {
        if let Some(result) = dispatch_dropdown_ui_event(hwnd, event) {
            return result;
        }
    }

    match msg {
        WM_CREATE => {
            let cs = &*(lparam as *const CREATESTRUCTW);
            let ptr = cs.lpCreateParams as *mut DropdownPopupState;
            platform_window::set_user_data(hwnd, ptr as isize);
            apply_window_corner_preference(hwnd);
            apply_dark_mode_to_window(hwnd);
            0
        }
        WM_MOUSEACTIVATE => MA_NOACTIVATE as isize,
        WM_ACTIVATE => 0,
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            let dpi = platform_dpi::layout_dpi_for_window(hwnd);
            set_settings_ui_dpi(dpi);
            let ptr = platform_window::user_data(hwnd) as *mut DropdownPopupState;
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = platform_gdi::begin_paint(hwnd, &mut ps);
            if !hdc.is_null() {
                crate::win_system_ui::set_paint_dpi_override(dpi);
                let rc = platform_window::client_rect(hwnd).unwrap_or_else(|| std::mem::zeroed());
                let paint_target = begin_buffered_paint(hdc, &rc);
                let memdc = if let Some((_, pdc)) = paint_target {
                    pdc
                } else {
                    hdc
                };
                let th = Theme::default();
                let w = (rc.right - rc.left).max(1);
                let h = (rc.bottom - rc.top).max(1);
                let bg = platform_gdi::create_solid_brush(th.bg);
                platform_gdi::fill_rect(memdc, &rc, bg);
                platform_gdi::delete_object(bg as _);
                let shell = RECT {
                    left: 0,
                    top: 0,
                    right: w,
                    bottom: h,
                };
                draw_round_rect(memdc as _, &shell, th.surface, th.stroke, 8);
                if !ptr.is_null() {
                    let st = &mut *ptr;
                    let interaction = st.interaction;
                    let start = interaction.scroll_top.max(0) as usize;
                    let end = (interaction.scroll_top + interaction.visible_rows)
                        .min(st.items.len() as i32)
                        .max(0) as usize;
                    for (visible_idx, idx) in (start..end).enumerate() {
                        let item = &st.items[idx];
                        let pad = settings_scale(SETTINGS_DROPDOWN_PAD);
                        let top = pad + visible_idx as i32 * interaction.item_height;
                        let item_rc = RECT {
                            left: pad,
                            top,
                            right: w - pad,
                            bottom: top + interaction.item_height,
                        };
                        let selected = st.selected == idx as i32;
                        if selected {
                            let fill = th.nav_sel_fill;
                            draw_round_fill(memdc as _, &item_rc, fill, 6);
                        }
                        if selected {
                            let cy = (item_rc.top + item_rc.bottom) / 2;
                            let bar = RECT {
                                left: item_rc.left + 4,
                                top: cy - 8,
                                right: item_rc.left + 7,
                                bottom: cy + 8,
                            };
                            draw_round_fill(memdc as _, &bar, th.accent, 2);
                        }
                        let text_rc = RECT {
                            left: item_rc.left + settings_scale(18),
                            top: item_rc.top,
                            right: item_rc.right - settings_scale(12),
                            bottom: item_rc.bottom,
                        };
                        draw_text_ex(
                            memdc as _,
                            item,
                            &text_rc,
                            th.text,
                            14,
                            false,
                            false,
                            ui_text_font_family(),
                        );
                    }
                    if dropdown_max_scroll(st) > 0 {
                        if interaction.scroll_top > 0 {
                            let top_hint = RECT {
                                left: w - settings_scale(22),
                                top: settings_scale(6),
                                right: w - settings_scale(8),
                                bottom: settings_scale(20),
                            };
                            draw_text_ex(
                                memdc as _,
                                "\u{25B4}",
                                &top_hint,
                                th.text_muted,
                                8,
                                false,
                                true,
                                ui_icon_font_family(),
                            );
                        }
                        if interaction.scroll_top < dropdown_max_scroll(st) {
                            let bottom_hint = RECT {
                                left: w - settings_scale(22),
                                top: h - settings_scale(20),
                                right: w - settings_scale(8),
                                bottom: h - settings_scale(6),
                            };
                            draw_text_ex(
                                memdc as _,
                                "\u{25BE}",
                                &bottom_hint,
                                th.text_muted,
                                8,
                                false,
                                true,
                                ui_icon_font_family(),
                            );
                        }
                    }
                }
                if let Some((paint_buf, _)) = paint_target {
                    end_buffered_paint(paint_buf, true);
                }
                crate::win_system_ui::clear_paint_dpi_override();
                platform_gdi::end_paint(hwnd, &ps);
            }
            0
        }
        WM_DESTROY => {
            let ptr = platform_window::user_data(hwnd) as *mut DropdownPopupState;
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
                platform_window::set_user_data(hwnd, 0);
            }
            0
        }
        _ => platform_window::default_window_proc(hwnd, msg, wparam, lparam),
    }
}

unsafe fn ensure_dropdown_popup_class() {
    let hinstance = platform_window::module_handle();
    let cname = to_wide(DROPDOWN_CLASS);
    let mut wc: WNDCLASSEXW = std::mem::zeroed();
    wc.cbSize = core::mem::size_of::<WNDCLASSEXW>() as u32;
    wc.style = CS_DROPSHADOW;
    wc.lpfnWndProc = Some(dropdown_popup_proc);
    wc.hInstance = hinstance;
    wc.hCursor = platform_window::arrow_cursor();
    wc.hbrBackground = null_mut();
    wc.lpszClassName = cname.as_ptr();
    platform_window::register_class_ex(&wc);
}

pub unsafe fn show_settings_dropdown_popup(
    parent: HWND,
    control_id: isize,
    anchor_rect: &RECT,
    items: &[&str],
    selected: usize,
    width: i32,
) -> HWND {
    ensure_dropdown_popup_class();
    let hinstance = platform_window::module_handle();
    let items_vec = items.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let layout =
        SettingsDropdownPopupLayout::new(anchor_rect.into(), items_vec.len(), selected, width);
    let state = Box::new(DropdownPopupState {
        parent,
        control_id,
        items: items_vec,
        selected: selected as i32,
        interaction: SettingsDropdownInteractionState::new(items.len(), layout),
    });
    let hwnd = platform_window::create_window_ex(
        WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
        to_wide(DROPDOWN_CLASS).as_ptr(),
        to_wide("").as_ptr(),
        WS_POPUP,
        layout.rect.left,
        layout.rect.top,
        layout.rect.right - layout.rect.left,
        layout.rect.bottom - layout.rect.top,
        parent,
        null_mut(),
        hinstance,
        Box::into_raw(state) as _,
    );
    if !hwnd.is_null() {
        platform_window::set_pos(
            hwnd,
            HWND_TOPMOST,
            layout.rect.left,
            layout.rect.top,
            layout.rect.right - layout.rect.left,
            layout.rect.bottom - layout.rect.top,
            SWP_SHOWWINDOW | SWP_NOACTIVATE,
        );
    }
    hwnd
}

pub(crate) fn settings_dropdown_popup_bounds(handle: HWND) -> Option<UiRect> {
    WindowsSettingsDropdownHost.settings_dropdown_bounds(handle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_core::{settings_timer_task_for_id, SettingsTimerIds, SettingsTimerTask};

    fn dropdown_test_state(item_count: usize, visible_rows: i32) -> DropdownPopupState {
        set_settings_ui_dpi(96);
        let layout =
            SettingsDropdownPopupLayout::new(UiRect::new(0, 0, 100, 32), item_count, 0, 100);
        let mut interaction = SettingsDropdownInteractionState::new(item_count, layout);
        interaction.visible_rows = visible_rows;
        interaction.scroll_top = 0;
        DropdownPopupState {
            parent: null_mut(),
            control_id: 1,
            items: (0..item_count).map(|idx| format!("item {idx}")).collect(),
            selected: 0,
            interaction,
        }
    }

    #[test]
    fn bdd_toggle_off_track_fill_is_visible_against_background() {
        let light_bg = rgb(250, 250, 250);
        let dark_bg = rgb(32, 32, 32);

        assert_ne!(settings_toggle_off_track_fill(light_bg), light_bg);
        assert_ne!(settings_toggle_off_track_fill(dark_bg), dark_bg);
    }

    #[test]
    fn settings_host_control_ids_map_to_roles_and_refresh_pages() {
        assert_eq!(
            settings_control_role_for_control(IDC_SET_SAVE),
            Some(SettingsControlRole::Save)
        );
        assert_eq!(
            settings_control_role_for_control(IDC_SET_MAX),
            Some(SettingsControlRole::Dropdown)
        );
        assert_eq!(
            settings_control_role_for_control(IDC_SET_PLUGIN_WPS_TASKPANE),
            Some(SettingsControlRole::Toggle)
        );
        assert_eq!(
            settings_control_role_for_control(IDC_SET_RICH_TEXT),
            Some(SettingsControlRole::Toggle)
        );
        assert_eq!(
            settings_command_for_control(IDC_SET_PLUGIN_WPS_TASKPANE),
            Some(crate::app_core::Command::window_with_payload(
                crate::app_core::command_ids::TOGGLE_SETTINGS_CONTROL,
                crate::app_core::CommandPayload::ControlId(IDC_SET_PLUGIN_WPS_TASKPANE as i64)
            ))
        );
        assert_eq!(
            settings_command_for_control(IDC_SET_RICH_TEXT),
            Some(crate::app_core::Command::window_with_payload(
                crate::app_core::command_ids::TOGGLE_SETTINGS_CONTROL,
                crate::app_core::CommandPayload::ControlId(IDC_SET_RICH_TEXT as i64)
            ))
        );
        assert_eq!(
            settings_page_to_sync_after_toggle(IDC_SET_PASTE_SOUND_ENABLE),
            Some(SettingsPage::General.index())
        );
        assert_eq!(
            settings_page_to_sync_after_toggle(IDC_SET_LAN_ENABLE),
            Some(SettingsPage::Cloud.index())
        );
        assert_eq!(
            settings_page_to_sync_after_toggle(IDC_SET_PLUGIN_SUPER_MAIL_MERGE),
            Some(SettingsPage::Plugin.index())
        );
        assert_eq!(settings_page_to_sync_after_toggle(IDC_SET_EDGEHIDE), None);
    }

    #[test]
    fn settings_host_control_ids_map_to_semantic_actions() {
        assert_eq!(
            settings_action_for_control(IDC_SET_GROUP_ADD, 0),
            Some(SettingsAction::AddGroup)
        );
        assert_eq!(
            settings_action_for_control(IDC_SET_GROUP_LIST, LBN_SELCHANGE as u16),
            Some(SettingsAction::GroupSelectionChanged)
        );
        assert_eq!(settings_action_for_control(IDC_SET_GROUP_LIST, 0), None);
        assert_eq!(
            settings_action_for_control(IDC_SET_CLOUD_SYNC_NOW, 0),
            Some(SettingsAction::SyncWebDavNow)
        );
        assert_eq!(
            settings_action_for_control(IDC_SET_LAN_COPY_PAIR, 0),
            Some(SettingsAction::CopyLanPairUrl)
        );
        assert_eq!(
            settings_action_for_control(IDC_SET_CLIPBOARD_HISTORY_DISABLE, 0),
            Some(SettingsAction::DisableSystemClipboardHistory)
        );
        assert_eq!(
            settings_action_for_control(IDC_SET_SEARCH_ENGINE_RESET, 0),
            Some(SettingsAction::RestoreSearchEnginePreset)
        );
    }

    #[test]
    fn dropdown_private_message_maps_to_platform_neutral_selection_event() {
        assert_eq!(
            settings_event_from_window_message(WM_SETTINGS_DROPDOWN_SELECTED, 5015, 3),
            Some(UiEvent::ControlSelectionChanged {
                control_id: 5015,
                index: 3,
            })
        );
        assert_eq!(settings_event_from_window_message(WM_PAINT, 0, 0), None);
    }

    #[test]
    fn settings_window_host_event_routes_private_and_generic_messages() {
        assert_eq!(
            settings_window_host_event_from_message(WM_SETTINGS_DROPDOWN_SELECTED, 5015, 3),
            Some(UiEvent::ControlSelectionChanged {
                control_id: 5015,
                index: 3,
            })
        );
        assert_eq!(
            settings_window_host_event_from_message(WM_TIMER, 42, 0),
            Some(UiEvent::Timer { id: 42 })
        );
        assert_eq!(
            settings_window_host_event_from_message(WM_PAINT, 0, 0),
            None
        );
    }

    #[test]
    fn dropdown_window_host_event_routes_pointer_messages() {
        assert_eq!(
            dropdown_window_host_event_from_message(WM_MOUSEMOVE, 0, 10 | (20 << 16)),
            Some(UiEvent::PointerMove {
                position: crate::app_core::Point { x: 10, y: 20 },
            })
        );
        assert_eq!(
            dropdown_window_host_event_from_message(WM_MOUSEWHEEL, (120usize) << 16, 0),
            Some(UiEvent::MouseWheel { delta: 120 })
        );
        assert_eq!(
            dropdown_window_host_event_from_message(WM_PAINT, 0, 0),
            None
        );
    }

    #[test]
    fn windows_settings_control_host_uses_platform_neutral_specs() {
        let source = include_str!("settings_ui_host.rs");

        assert!(source.contains("struct WindowsSettingsControlHost"));
        assert!(source.contains("impl NativeSettingsControlHost for WindowsSettingsControlHost"));
        assert!(source.contains("fn create_control(&mut self, spec: &SettingsControlSpec)"));
        assert!(source.contains("fn destroy_control(&mut self, handle: Self::Handle)"));
        assert!(source.contains("fn control_exists(&self, handle: Self::Handle)"));
        assert!(source
            .contains("fn set_control_visible(&mut self, handle: Self::Handle, visible: bool)"));
        assert!(source
            .contains("fn set_control_enabled(&mut self, handle: Self::Handle, enabled: bool)"));
        assert!(source
            .contains("fn set_control_bounds(&mut self, handle: Self::Handle, bounds: UiRect)"));
        assert!(source.contains("fn control_at_point(&self, point: Point)"));
        assert!(source.contains("fn control_screen_bounds(&self, handle: Self::Handle)"));
        assert!(source.contains("fn control_text(&self, handle: Self::Handle) -> String"));
        assert!(source.contains("fn set_control_text(&mut self, handle: Self::Handle, text: &str)"));
        assert!(source.contains("fn request_control_repaint(&mut self, handle: Self::Handle)"));
        assert!(
            source.contains("WindowsSettingsControlHost::new(parent, font).create_control(spec)")
        );
        assert!(source.contains("host.destroy_control(hwnd)"));
        assert!(source.contains("settings_host_exists"));
        assert!(source.contains("settings_host_set_enabled"));
        assert!(source.contains("settings_host_screen_bounds"));
        assert!(source.contains("settings_host_control_at_point"));
        assert!(source.contains("settings_host_text"));
        assert!(source.contains("settings_host_set_text"));
        assert!(source.contains("settings_host_request_repaint"));
    }

    #[test]
    fn settings_timer_ids_map_to_host_tasks() {
        let ids = SettingsTimerIds {
            hide_scrollbar: 4,
            clear_save_hint: 8,
            dpi_fit: 15,
        };

        assert_eq!(
            settings_timer_task_for_id(4, ids),
            Some(SettingsTimerTask::HideScrollbar)
        );
        assert_eq!(
            settings_timer_task_for_id(8, ids),
            Some(SettingsTimerTask::ClearSaveHint)
        );
        assert_eq!(
            settings_timer_task_for_id(15, ids),
            Some(SettingsTimerTask::DpiFit)
        );
        assert_eq!(settings_timer_task_for_id(1, ids), None);
        assert_eq!(settings_timer_task_for_id(usize::MAX, ids), None);
    }

    #[test]
    fn dropdown_index_uses_scroll_top_and_rejects_padding_hits() {
        let mut st = dropdown_test_state(8, 4);
        st.interaction.scroll_top = 2;
        let pad = settings_scale(SETTINGS_DROPDOWN_PAD);

        assert_eq!(dropdown_index_from_y(&st, pad - 1), -1);
        assert_eq!(
            dropdown_index_from_y(&st, pad + st.interaction.item_height / 2),
            2
        );
        assert_eq!(
            dropdown_index_from_y(
                &st,
                pad + st.interaction.item_height * 3 + st.interaction.item_height / 2
            ),
            5
        );
        assert_eq!(
            dropdown_index_from_y(&st, pad + st.interaction.item_height * 4),
            -1
        );
    }

    #[test]
    fn dropdown_wheel_scrolls_by_one_row_and_clamps() {
        let mut st = dropdown_test_state(8, 4);
        st.interaction.scroll_top = 2;
        st.interaction.hover = 3;

        assert!(dropdown_scroll_by_wheel(&mut st, 120));
        assert_eq!(st.interaction.scroll_top, 1);
        assert_eq!(st.interaction.hover, -1);

        assert!(dropdown_scroll_by_wheel(&mut st, -120));
        assert_eq!(st.interaction.scroll_top, 2);

        st.interaction.scroll_top = dropdown_max_scroll(&st);
        assert!(!dropdown_scroll_by_wheel(&mut st, -120));
        assert_eq!(st.interaction.scroll_top, dropdown_max_scroll(&st));
    }

    #[test]
    fn settings_ui_host_does_not_define_control_ids_locally() {
        let source = include_str!("settings_ui_host.rs");
        let forbidden = format!("{}{}", "const IDC_", "SET_");
        assert!(!source.contains(&forbidden));
    }

    #[test]
    fn windows_settings_window_lifecycle_is_owned_by_settings_host() {
        let source = include_str!("settings_ui_host.rs");

        assert!(source.contains("pub struct WindowsSettingsWindowHost"));
        assert!(source.contains("impl NativeSettingsWindowHost for WindowsSettingsWindowHost"));
        assert!(source.contains("pub unsafe fn present_settings_window"));
        assert!(source.contains("WindowsSettingsWindowPresentation::FocusedExisting"));
        assert!(source.contains("WindowsSettingsWindowPresentation::Created"));
        assert!(source.contains("to_wide(SETTINGS_CLASS)"));
        assert!(source.contains("window_class.lpfnWndProc = window_proc"));
        assert!(source.contains("platform_window::create_window_ex("));
        assert!(source.contains("fn destroy_settings_window(&mut self"));
        assert!(source.contains("platform_window::destroy(handle)"));
        assert!(source.contains("fn focus_settings_window(&mut self"));
        assert!(source.contains("platform_input::set_focus(handle)"));
        assert!(source.contains("fn track_settings_pointer_leave(&mut self"));
        assert!(source.contains("platform_input::track_mouse_leave_and_hover("));
        assert!(source.contains("pub(crate) fn settings_window_track_pointer_leave"));
        assert!(source.contains("fn capture_settings_pointer(&mut self"));
        assert!(source.contains("platform_input::set_capture(handle)"));
        assert!(source.contains("fn release_settings_pointer(&mut self"));
        assert!(source.contains("platform_input::release_capture()"));
        assert!(source.contains("fn request_settings_window_area_repaint("));
        assert!(source.contains("platform_gdi::invalidate_rect(handle, rect_ptr, erase as i32)"));
        assert!(source.contains("fn settings_window_layout_dpi("));
        assert!(source.contains("platform_dpi::layout_dpi_for_window(handle)"));
        assert!(source.contains("pub(crate) fn settings_window_layout_dpi"));
        assert!(source.contains("fn settings_window_client_to_screen("));
        assert!(source.contains("platform_window::client_to_screen(handle"));
        assert!(source.contains("pub(crate) fn settings_window_client_to_screen"));
        assert!(source.contains("fn settings_window_client_bounds("));
        assert!(source.contains("platform_window::client_rect(handle)"));
        assert!(source.contains("pub(crate) fn settings_window_client_bounds"));
        assert!(source.contains("fn settings_window_bounds("));
        assert!(source.contains("platform_window::window_rect(handle)"));
        assert!(source.contains("pub(crate) fn settings_window_bounds"));
    }

    #[test]
    fn windows_settings_dropdown_lifecycle_is_owned_by_settings_host() {
        let source = include_str!("settings_ui_host.rs");

        assert!(source.contains("pub struct WindowsSettingsDropdownHost"));
        assert!(source.contains("impl NativeSettingsDropdownHost for WindowsSettingsDropdownHost"));
        assert!(source.contains("fn present_settings_dropdown("));
        assert!(source.contains("NativeSettingsDropdownPresentation::Created(handle)"));
        assert!(source.contains("NativeSettingsDropdownPresentation::Failed"));
        assert!(source.contains("fn destroy_settings_dropdown(&mut self"));
        assert!(source.contains("platform_window::destroy(handle)"));
        assert!(source.contains("fn settings_dropdown_bounds(&self"));
        assert!(source.contains("platform_window::window_rect(handle)"));
        assert!(source.contains("settings_dropdown_popup_bounds"));
        assert!(source.contains("pub unsafe fn show_settings_dropdown_popup"));
        assert!(source.contains("platform_window::create_window_ex("));
    }
}

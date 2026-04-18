use std::collections::HashMap;
use std::ffi::c_void;
use std::mem::zeroed;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

pub use crate::settings_model::SettingsPage;
pub use crate::settings_render::{
    draw_settings_nav_item, draw_settings_page_cards, nav_divider_x,
    settings_title_rect_win,
};
use crate::i18n::translate;
pub use crate::settings_ui_host::{
    create_settings_button, create_settings_component, create_settings_dropdown_button, create_settings_edit,
    create_settings_fonts, create_settings_label, create_settings_label_auto, create_settings_listbox,
    create_settings_password_edit, create_settings_small_button, create_settings_toggle_plain,
    draw_settings_button_component, draw_settings_toggle_component, draw_text_wide_centered, get_ctrl_text_wide,
    set_settings_font, settings_child_visible, settings_dropdown_index_for_max_items,
    settings_dropdown_index_for_pos_mode, settings_dropdown_label_for_max_items,
    settings_dropdown_label_for_pos_mode, settings_dropdown_max_items_from_label,
    settings_dropdown_pos_mode_from_label, settings_safe_paint_rect, settings_viewport_mask_rect,
    settings_viewport_rect, show_settings_dropdown_popup, SettingsComponentKind, SettingsCtrlReg,
    SettingsUiRegistry, WM_SETTINGS_DROPDOWN_SELECTED,
};
use windows_sys::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::Gdi::{
        CreateFontW, DrawTextW, GetDC, GetDeviceCaps, GetStockObject, MonitorFromPoint,
        MonitorFromWindow, ReleaseDC, SelectObject, SetBkMode, SetTextColor, DEFAULT_GUI_FONT,
        LOGPIXELSX, LOGPIXELSY,
        MONITOR_DEFAULTTONEAREST,
    },
    System::Ole::DROPEFFECT,
    UI::{
        Input::KeyboardAndMouse::{
            keybd_event, GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
            KEYEVENTF_KEYUP, VK_BACK, VK_CONTROL, VK_MENU, VK_SHIFT, VK_V,
        },
        WindowsAndMessaging::{
            GA_ROOT, GetAncestor, GetParent, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
            IsWindow, SetForegroundWindow, SystemParametersInfoW, WindowFromPoint,
        },
    },
};

use crate::win_system_params::{
    DRAGDROP_S_CANCEL_HR, DRAGDROP_S_DROP_HR, DRAGDROP_S_USEDEFAULTCURSORS_HR,
    E_NOINTERFACE_HR, E_POINTER_HR, IID_IDROPSOURCE_RAW, IID_IUNKNOWN_RAW, MK_LBUTTON_FLAG,
    S_OK_HR,
};

const HKEY_CURRENT_USER: isize = -2147483647i32 as isize;
const KEY_READ: u32 = 0x20019;
const REG_DWORD: u32 = 4;
const SPI_GETNONCLIENTMETRICS: u32 = 0x0029;

#[repr(C)]
struct RawLogFontW {
    lf_height: i32,
    lf_width: i32,
    lf_escapement: i32,
    lf_orientation: i32,
    lf_weight: i32,
    lf_italic: u8,
    lf_underline: u8,
    lf_strike_out: u8,
    lf_char_set: u8,
    lf_out_precision: u8,
    lf_clip_precision: u8,
    lf_quality: u8,
    lf_pitch_and_family: u8,
    lf_face_name: [u16; 32],
}

#[repr(C)]
struct RawNonClientMetricsW {
    cb_size: u32,
    i_border_width: i32,
    i_scroll_width: i32,
    i_scroll_height: i32,
    i_caption_width: i32,
    i_caption_height: i32,
    lf_caption_font: RawLogFontW,
    i_sm_caption_width: i32,
    i_sm_caption_height: i32,
    lf_sm_caption_font: RawLogFontW,
    i_menu_width: i32,
    i_menu_height: i32,
    lf_menu_font: RawLogFontW,
    lf_status_font: RawLogFontW,
    lf_message_font: RawLogFontW,
    i_padded_border_width: i32,
}

static SYSTEM_UI_FONT_FAMILY: OnceLock<String> = OnceLock::new();
static SCALED_FONT_CACHE: OnceLock<Mutex<HashMap<(String, i32, i32, i32), isize>>> = OnceLock::new();

#[link(name = "advapi32")]
unsafe extern "system" {
    fn RegOpenKeyExW(
        hkey: isize,
        lpsubkey: *const u16,
        uloptions: u32,
        samdesired: u32,
        phkresult: *mut isize,
    ) -> i32;
    fn RegQueryValueExW(
        hkey: isize,
        lpvaluename: *const u16,
        lpreserved: *mut u32,
        lptype: *mut u32,
        lpdata: *mut u8,
        lpcbdata: *mut u32,
    ) -> i32;
    fn RegCloseKey(hkey: isize) -> i32;
}

#[link(name = "dwmapi")]
unsafe extern "system" {
    fn DwmSetWindowAttribute(
        hwnd: HWND,
        dwattribute: u32,
        pvattribute: *const c_void,
        cbattribute: u32,
    ) -> i32;
    fn DwmGetWindowAttribute(
        hwnd: HWND,
        dwattribute: u32,
        pvattribute: *mut c_void,
        cbattribute: u32,
    ) -> i32;
    fn DwmGetColorizationColor(pcr_colorization: *mut u32, pf_opaque_blend: *mut i32) -> i32;
}

#[repr(C)]
struct RawMonitorInfo {
    cb_size: u32,
    rc_monitor: RECT,
    rc_work: RECT,
    dw_flags: u32,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn GetMonitorInfoW(hmonitor: *mut c_void, lpmi: *mut RawMonitorInfo) -> i32;
}

#[link(name = "oleacc")]
unsafe extern "system" {
    fn AccessibleObjectFromWindow(
        hwnd: HWND,
        dw_id: i32,
        riid: *const windows_sys::core::GUID,
        ppv_object: *mut *mut c_void,
    ) -> i32;
}

pub(crate) fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

pub(crate) fn resolve_ui_font_family(family: &str) -> &str {
    match family {
        "" => system_ui_text_font_family(),
        "Segoe UI Variable Text" => system_ui_text_font_family(),
        "Segoe UI Variable Display" => system_ui_text_font_family(),
        "Segoe Fluent Icons" => "Segoe MDL2 Assets",
        other => other,
    }
}

thread_local! {
    static PAINT_DPI_OVERRIDE: std::cell::Cell<u32> = std::cell::Cell::new(0);
}

pub(crate) fn set_paint_dpi_override(dpi: u32) {
    PAINT_DPI_OVERRIDE.with(|cell| cell.set(dpi));
}

pub(crate) fn clear_paint_dpi_override() {
    PAINT_DPI_OVERRIDE.with(|cell| cell.set(0));
}

fn paint_dpi_override() -> u32 {
    PAINT_DPI_OVERRIDE.with(|cell| cell.get())
}

pub(crate) unsafe fn create_scaled_font_for_hdc(
    hdc: *mut c_void,
    family: &str,
    size: i32,
    weight: i32,
) -> *mut c_void {
    let font_name = resolve_ui_font_family(family);
    let hdc_dpi = GetDeviceCaps(hdc as _, LOGPIXELSY as i32).max(96) as u32;
    let override_dpi = paint_dpi_override();
    let dpi = if override_dpi > hdc_dpi { override_dpi as i32 } else { hdc_dpi as i32 };
    let scaled_size = ((size.max(1) * dpi) + 48) / 96;
    let key = (font_name.to_string(), dpi, scaled_size, weight);
    let cache = SCALED_FONT_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut fonts) = cache.lock() {
        if let Some(font) = fonts.get(&key) {
            return *font as _;
        }
        let font = CreateFontW(
            -scaled_size,
            0,
            0,
            0,
            weight,
            0,
            0,
            0,
            1,
            0,
            0,
            5,
            0,
            to_wide(font_name).as_ptr(),
        ) as isize;
        if font != 0 {
            fonts.insert(key, font);
        }
        font as _
    } else {
        CreateFontW(
            -scaled_size,
            0,
            0,
            0,
            weight,
            0,
            0,
            0,
            1,
            0,
            0,
            5,
            0,
            to_wide(font_name).as_ptr(),
        ) as _
    }
}

pub(crate) unsafe fn create_font_px(
    family: &str,
    pixel_size: i32,
    weight: i32,
) -> *mut c_void {
    CreateFontW(
        -pixel_size.max(1),
        0,
        0,
        0,
        weight,
        0,
        0,
        0,
        1,
        0,
        0,
        5,
        0,
        to_wide(resolve_ui_font_family(family)).as_ptr(),
    ) as _
}

pub(crate) unsafe fn draw_translated_text_line(
    hdc: *mut c_void,
    text: &str,
    rc: &mut RECT,
    color: u32,
    size: i32,
    weight: i32,
    center: bool,
    family: &str,
    transparent_mode: i32,
    flags_extra: u32,
) {
    let translated = translate(text);
    SetBkMode(hdc as _, transparent_mode);
    SetTextColor(hdc as _, color);
    let font = create_scaled_font_for_hdc(hdc, family, size, weight);
    let font = if font.is_null() {
        GetStockObject(DEFAULT_GUI_FONT) as *mut c_void
    } else {
        font
    };
    let old = SelectObject(hdc as _, font as _);
    let flags =
        (if center { crate::ui::DT_CENTER } else { crate::ui::DT_LEFT })
        | crate::ui::DT_VCENTER
        | crate::ui::DT_SINGLELINE
        | crate::ui::DT_END_ELLIPSIS
        | flags_extra;
    DrawTextW(hdc as _, to_wide(translated.as_ref()).as_ptr(), -1, rc, flags);
    SelectObject(hdc as _, old);
}

pub(crate) unsafe fn draw_translated_text_block(
    hdc: *mut c_void,
    text: &str,
    rc: &mut RECT,
    color: u32,
    size: i32,
    weight: i32,
    family: &str,
    transparent_mode: i32,
    flags_extra: u32,
) {
    let translated = translate(text);
    SetBkMode(hdc as _, transparent_mode);
    SetTextColor(hdc as _, color);
    let font = create_scaled_font_for_hdc(hdc, family, size, weight);
    let font = if font.is_null() {
        GetStockObject(DEFAULT_GUI_FONT) as *mut c_void
    } else {
        font
    };
    let old = SelectObject(hdc as _, font as _);
    let flags = crate::ui::DT_LEFT | crate::ui::DT_WORDBREAK | crate::ui::DT_NOPREFIX | flags_extra;
    DrawTextW(hdc as _, to_wide(translated.as_ref()).as_ptr(), -1, rc, flags);
    SelectObject(hdc as _, old);
}

fn rgb(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

pub(crate) fn is_dark_mode() -> bool {
    unsafe {
        let key_path = to_wide("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
        let val_name = to_wide("AppsUseLightTheme");
        let mut hkey = 0isize;
        if RegOpenKeyExW(HKEY_CURRENT_USER, key_path.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
            return false;
        }
        let mut data = 0u32;
        let mut data_size = 4u32;
        let mut reg_type = 0u32;
        let ret = RegQueryValueExW(
            hkey,
            val_name.as_ptr(),
            core::ptr::null_mut(),
            &mut reg_type,
            &mut data as *mut u32 as *mut u8,
            &mut data_size,
        );
        RegCloseKey(hkey);
        ret == 0 && reg_type == REG_DWORD && data == 0
    }
}

pub(crate) fn system_accent() -> u32 {
    unsafe {
        let mut color = 0u32;
        let mut opaque = 0i32;
        if DwmGetColorizationColor(&mut color, &mut opaque) == 0 {
            let r = ((color >> 16) & 0xFF) as u8;
            let g = ((color >> 8) & 0xFF) as u8;
            let b = (color & 0xFF) as u8;
            return rgb(r, g, b);
        }
    }
    rgb(0, 120, 212)
}

pub(crate) unsafe fn send_ctrl_v() {
    fn key_input(vk: u16, flags: u32) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    let shift_down = (GetAsyncKeyState(VK_SHIFT as i32) as u16 & 0x8000) != 0;
    let mut inputs = Vec::with_capacity(if shift_down { 6 } else { 4 });
    if shift_down {
        inputs.push(key_input(VK_SHIFT as u16, KEYEVENTF_KEYUP));
    }
    inputs.push(key_input(VK_CONTROL as u16, 0));
    inputs.push(key_input(VK_V as u16, 0));
    inputs.push(key_input(VK_V as u16, KEYEVENTF_KEYUP));
    inputs.push(key_input(VK_CONTROL as u16, KEYEVENTF_KEYUP));
    if shift_down {
        inputs.push(key_input(VK_SHIFT as u16, 0));
    }
    let _ = SendInput(
        inputs.len() as u32,
        inputs.as_mut_ptr(),
        core::mem::size_of::<INPUT>() as i32,
    );
}

pub(crate) unsafe fn send_backspace_times(count: u8) {
    for _ in 0..count {
        keybd_event(VK_BACK as u8, 0, 0, 0);
        keybd_event(VK_BACK as u8, 0, KEYEVENTF_KEYUP, 0);
    }
}

pub(crate) unsafe fn send_alt_tap() {
    keybd_event(VK_MENU as u8, 0, 0, 0);
    keybd_event(VK_MENU as u8, 0, KEYEVENTF_KEYUP, 0);
}

pub(crate) unsafe fn force_foreground_window(hwnd: HWND) -> bool {
    if hwnd.is_null() {
        return false;
    }
    if SetForegroundWindow(hwnd) != 0 {
        return true;
    }
    send_alt_tap();
    SetForegroundWindow(hwnd) != 0
}

pub(crate) unsafe fn init_dpi_awareness_for_process() {
    use windows_sys::Win32::Foundation::FreeLibrary;
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

    let user32 = LoadLibraryW(to_wide("user32.dll").as_ptr());
    if !user32.is_null() {
        type FnSetCtx = unsafe extern "system" fn(isize) -> i32;
        if let Some(f) = core::mem::transmute::<_, Option<FnSetCtx>>(GetProcAddress(
            user32,
            b"SetProcessDpiAwarenessContext\0".as_ptr(),
        )) {
            const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4isize;
            if f(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) != 0 {
                FreeLibrary(user32);
                return;
            }
        }

        type FnSetAware = unsafe extern "system" fn() -> i32;
        if let Some(f) = core::mem::transmute::<_, Option<FnSetAware>>(GetProcAddress(
            user32,
            b"SetProcessDPIAware\0".as_ptr(),
        )) {
            let _ = f();
            FreeLibrary(user32);
            return;
        }
        FreeLibrary(user32);
    }

    let shcore = LoadLibraryW(to_wide("shcore.dll").as_ptr());
    if shcore.is_null() {
        return;
    }
    type FnSetAwareness = unsafe extern "system" fn(i32) -> i32;
    if let Some(f) = core::mem::transmute::<_, Option<FnSetAwareness>>(GetProcAddress(
        shcore,
        b"SetProcessDpiAwareness\0".as_ptr(),
    )) {
        const PROCESS_PER_MONITOR_DPI_AWARE: i32 = 2;
        let _ = f(PROCESS_PER_MONITOR_DPI_AWARE);
    }
    FreeLibrary(shcore);
}

pub(crate) unsafe fn window_dpi(hwnd: HWND) -> u32 {
    use windows_sys::Win32::Foundation::FreeLibrary;
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

    if !hwnd.is_null() {
        let user32 = LoadLibraryW(to_wide("user32.dll").as_ptr());
        if !user32.is_null() {
            type FnGetDpiForWindow = unsafe extern "system" fn(HWND) -> u32;
            type FnGetDpiForSystem = unsafe extern "system" fn() -> u32;
            if let Some(f) = core::mem::transmute::<_, Option<FnGetDpiForWindow>>(GetProcAddress(
                user32,
                b"GetDpiForWindow\0".as_ptr(),
            )) {
                let dpi = f(hwnd);
                if dpi != 0 {
                    FreeLibrary(user32);
                    return dpi;
                }
            }
            if let Some(f) = core::mem::transmute::<_, Option<FnGetDpiForSystem>>(GetProcAddress(
                user32,
                b"GetDpiForSystem\0".as_ptr(),
            )) {
                let dpi = f();
                if dpi != 0 {
                    FreeLibrary(user32);
                    return dpi;
                }
            }
            FreeLibrary(user32);
        }
    }

    let screen_dc = GetDC(core::ptr::null_mut());
    if !screen_dc.is_null() {
        let dpi = GetDeviceCaps(screen_dc, LOGPIXELSX as i32);
        ReleaseDC(core::ptr::null_mut(), screen_dc);
        if dpi > 0 {
            return dpi as u32;
        }
    }
    96
}

pub(crate) unsafe fn monitor_dpi_for_point(pt: POINT) -> u32 {
    use windows_sys::Win32::Foundation::FreeLibrary;
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

    let shcore = LoadLibraryW(to_wide("shcore.dll").as_ptr());
    if !shcore.is_null() {
        type FnGetDpiForMonitor = unsafe extern "system" fn(*mut core::ffi::c_void, i32, *mut u32, *mut u32) -> i32;
        if let Some(f) = core::mem::transmute::<_, Option<FnGetDpiForMonitor>>(GetProcAddress(
            shcore,
            b"GetDpiForMonitor\0".as_ptr(),
        )) {
            let monitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
            if !monitor.is_null() {
                let mut dpi_x = 0u32;
                let mut dpi_y = 0u32;
                const MDT_EFFECTIVE_DPI: i32 = 0;
                if f(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) == 0 && dpi_x != 0 {
                    FreeLibrary(shcore);
                    return dpi_x;
                }
            }
        }
        FreeLibrary(shcore);
    }

    window_dpi(null_mut())
}

pub(crate) unsafe fn monitor_dpi_for_window(hwnd: HWND) -> u32 {
    if !hwnd.is_null() {
        let mut rc: RECT = zeroed();
        if GetWindowRect(hwnd, &mut rc) != 0 && rc.right > rc.left && rc.bottom > rc.top {
            let center = POINT {
                x: rc.left + ((rc.right - rc.left) / 2),
                y: rc.top + ((rc.bottom - rc.top) / 2),
            };
            let dpi = monitor_dpi_for_point(center);
            if dpi != 0 {
                return dpi;
            }
        }
    }
    window_dpi(hwnd)
}

pub(crate) unsafe fn scale_for_window(hwnd: HWND, value: i32) -> i32 {
    let dpi = monitor_dpi_for_window(hwnd).max(96) as i32;
    ((value * dpi) + 48) / 96
}

pub(crate) unsafe fn init_dark_mode_for_process() {
    use windows_sys::Win32::Foundation::FreeLibrary;
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

    let lib = LoadLibraryW(to_wide("uxtheme.dll").as_ptr());
    if lib.is_null() {
        return;
    }

    type FnSetMode = unsafe extern "system" fn(i32) -> i32;
    if let Some(f) = core::mem::transmute::<_, Option<FnSetMode>>(GetProcAddress(lib, 135usize as _))
    {
        f(if is_dark_mode() { 2 } else { 0 });
    }

    type FnFlush = unsafe extern "system" fn();
    if let Some(f) = core::mem::transmute::<_, Option<FnFlush>>(GetProcAddress(lib, 136usize as _))
    {
        f();
    }

    FreeLibrary(lib);
}

pub(crate) unsafe fn apply_dark_mode_to_window(hwnd: HWND) {
    use windows_sys::Win32::Foundation::FreeLibrary;
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

    if is_dark_mode() {
        let val: u32 = 1;
        let _ = DwmSetWindowAttribute(hwnd, 20, &val as *const u32 as _, 4);
        let _ = DwmSetWindowAttribute(hwnd, 19, &val as *const u32 as _, 4);
    }

    let lib = LoadLibraryW(to_wide("uxtheme.dll").as_ptr());
    if lib.is_null() {
        return;
    }

    type FnAllow = unsafe extern "system" fn(HWND, i32) -> i32;
    if let Some(f) = core::mem::transmute::<_, Option<FnAllow>>(GetProcAddress(lib, 133usize as _))
    {
        f(hwnd, if is_dark_mode() { 1 } else { 0 });
    }

    FreeLibrary(lib);
}

pub(crate) unsafe fn apply_theme_to_menu(hmenu: *mut c_void) {
    use windows_sys::Win32::Foundation::FreeLibrary;
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

    if hmenu.is_null() {
        return;
    }

    let lib = LoadLibraryW(to_wide("uxtheme.dll").as_ptr());
    if lib.is_null() {
        return;
    }

    type FnFlush = unsafe extern "system" fn();
    if let Some(f) = core::mem::transmute::<_, Option<FnFlush>>(GetProcAddress(lib, 136usize as _))
    {
        f();
    }

    FreeLibrary(lib);
}

pub(crate) unsafe fn apply_window_corner_preference(hwnd: HWND) {
    const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
    const DWMWCP_ROUND: u32 = 2;

    let pref: u32 = DWMWCP_ROUND;
    let _ = DwmSetWindowAttribute(
        hwnd,
        DWMWA_WINDOW_CORNER_PREFERENCE,
        &pref as *const _ as *const c_void,
        core::mem::size_of::<u32>() as u32,
    );
}

pub(crate) unsafe fn get_window_text(hwnd: HWND) -> String {
    let len = GetWindowTextLengthW(hwnd);
    let mut buf = vec![0u16; (len as usize) + 1];
    if !buf.is_empty() {
        GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
    }
    String::from_utf16_lossy(&buf)
        .trim_end_matches('\0')
        .to_string()
}

pub(crate) unsafe fn system_mouse_hover_time_ms() -> u32 {
    const SPI_GETMOUSEHOVERTIME: u32 = 0x0066;
    let mut hover_ms = 0u32;
    if SystemParametersInfoW(
        SPI_GETMOUSEHOVERTIME,
        0,
        &mut hover_ms as *mut _ as _,
        0,
    ) != 0
        && hover_ms > 0
    {
        hover_ms
    } else {
        400
    }
}

pub(crate) fn system_ui_text_font_family() -> &'static str {
    SYSTEM_UI_FONT_FAMILY.get_or_init(|| unsafe {
        let mut metrics = RawNonClientMetricsW {
            cb_size: core::mem::size_of::<RawNonClientMetricsW>() as u32,
            i_border_width: 0,
            i_scroll_width: 0,
            i_scroll_height: 0,
            i_caption_width: 0,
            i_caption_height: 0,
            lf_caption_font: zeroed_logfont(),
            i_sm_caption_width: 0,
            i_sm_caption_height: 0,
            lf_sm_caption_font: zeroed_logfont(),
            i_menu_width: 0,
            i_menu_height: 0,
            lf_menu_font: zeroed_logfont(),
            lf_status_font: zeroed_logfont(),
            lf_message_font: zeroed_logfont(),
            i_padded_border_width: 0,
        };
        if SystemParametersInfoW(
            SPI_GETNONCLIENTMETRICS,
            metrics.cb_size,
            &mut metrics as *mut _ as _,
            0,
        ) != 0
        {
            let end = metrics
                .lf_message_font
                .lf_face_name
                .iter()
                .position(|ch| *ch == 0)
                .unwrap_or(metrics.lf_message_font.lf_face_name.len());
            let face = String::from_utf16_lossy(&metrics.lf_message_font.lf_face_name[..end])
                .trim()
                .to_string();
            if !face.is_empty() {
                return face;
            }
        }
        "Segoe UI".to_string()
    })
}

const fn zeroed_logfont() -> RawLogFontW {
    RawLogFontW {
        lf_height: 0,
        lf_width: 0,
        lf_escapement: 0,
        lf_orientation: 0,
        lf_weight: 0,
        lf_italic: 0,
        lf_underline: 0,
        lf_strike_out: 0,
        lf_char_set: 0,
        lf_out_precision: 0,
        lf_clip_precision: 0,
        lf_quality: 0,
        lf_pitch_and_family: 0,
        lf_face_name: [0; 32],
    }
}

fn fallback_primary_rect() -> RECT {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

    RECT {
        left: 0,
        top: 0,
        right: unsafe { GetSystemMetrics(SM_CXSCREEN) },
        bottom: unsafe { GetSystemMetrics(SM_CYSCREEN) },
    }
}

unsafe fn monitor_info_from_handle(hmonitor: *mut c_void) -> Option<RawMonitorInfo> {
    if hmonitor.is_null() {
        return None;
    }
    let mut info = RawMonitorInfo {
        cb_size: core::mem::size_of::<RawMonitorInfo>() as u32,
        rc_monitor: RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
        rc_work: RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
        dw_flags: 0,
    };
    if GetMonitorInfoW(hmonitor, &mut info) != 0 {
        Some(info)
    } else {
        None
    }
}

pub(crate) unsafe fn nearest_monitor_work_rect_for_point(pt: POINT) -> RECT {
    monitor_info_from_handle(MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST))
        .map(|info| info.rc_work)
        .unwrap_or_else(fallback_primary_rect)
}

pub(crate) unsafe fn nearest_monitor_work_rect_for_window(hwnd: HWND) -> RECT {
    monitor_info_from_handle(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST))
        .map(|info| info.rc_work)
        .unwrap_or_else(fallback_primary_rect)
}

pub(crate) unsafe fn nearest_monitor_rect_for_window(hwnd: HWND) -> RECT {
    monitor_info_from_handle(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST))
        .map(|info| info.rc_monitor)
        .unwrap_or_else(fallback_primary_rect)
}

pub(crate) unsafe fn nearest_monitor_rect_for_point(pt: POINT) -> RECT {
    monitor_info_from_handle(MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST))
        .map(|info| info.rc_monitor)
        .unwrap_or_else(fallback_primary_rect)
}

pub(crate) unsafe fn window_rect_for_dock(hwnd: HWND) -> RECT {
    const DWMWA_EXTENDED_FRAME_BOUNDS: u32 = 9;

    let mut rc = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    if !hwnd.is_null()
        && DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut rc as *mut _ as *mut c_void,
            core::mem::size_of::<RECT>() as u32,
        ) == 0
        && rc.right > rc.left
        && rc.bottom > rc.top
    {
        return rc;
    }
    let _ = GetWindowRect(hwnd, &mut rc);
    rc
}

pub(crate) unsafe fn point_in_rect_screen(pt: &POINT, rc: &RECT) -> bool {
    pt.x >= rc.left && pt.x <= rc.right && pt.y >= rc.top && pt.y <= rc.bottom
}

pub(crate) unsafe fn cursor_over_window_tree(root_hwnd: HWND, cursor: POINT) -> bool {
    if root_hwnd.is_null() || IsWindow(root_hwnd) == 0 {
        return false;
    }
    let hit = WindowFromPoint(cursor);
    if hit.is_null() {
        return point_in_rect_screen(&cursor, &window_rect_for_dock(root_hwnd));
    }

    let root = GetAncestor(hit, GA_ROOT);
    if !root.is_null() {
        return root == root_hwnd;
    }

    let mut cur = hit;
    for _ in 0..32 {
        if cur.is_null() {
            break;
        }
        if cur == root_hwnd {
            return true;
        }
        cur = GetParent(cur);
    }
    point_in_rect_screen(&cursor, &window_rect_for_dock(root_hwnd))
}

pub(crate) fn get_x_lparam(lp: isize) -> i32 {
    (lp as i16) as i32
}

pub(crate) fn get_y_lparam(lp: isize) -> i32 {
    ((lp >> 16) as i16) as i32
}

#[repr(C)]
pub(crate) struct RawIUnknown {
    pub(crate) vtbl: *const RawIUnknownVtbl,
}

#[repr(C)]
pub(crate) struct RawIUnknownVtbl {
    pub(crate) query_interface: unsafe extern "system" fn(
        *mut c_void,
        *const windows_sys::core::GUID,
        *mut *mut c_void,
    ) -> windows_sys::core::HRESULT,
    pub(crate) add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub(crate) release: unsafe extern "system" fn(*mut c_void) -> u32,
}

#[repr(C)]
struct RawVariant {
    vt: u16,
    w_reserved1: u16,
    w_reserved2: u16,
    w_reserved3: u16,
    data: [u8; 16],
}

#[repr(C)]
struct RawIDispatchVtbl {
    base: RawIUnknownVtbl,
    get_type_info_count: usize,
    get_type_info: usize,
    get_ids_of_names: usize,
    invoke: usize,
}

#[repr(C)]
struct RawIAccessible {
    vtbl: *const RawIAccessibleVtbl,
}

#[repr(C)]
struct RawIAccessibleVtbl {
    base: RawIDispatchVtbl,
    get_acc_parent: usize,
    get_acc_child_count: usize,
    get_acc_child: usize,
    get_acc_name: usize,
    get_acc_value: usize,
    get_acc_description: usize,
    get_acc_role: usize,
    get_acc_state: usize,
    get_acc_help: usize,
    get_acc_help_topic: usize,
    get_acc_keyboard_shortcut: usize,
    get_acc_focus: usize,
    get_acc_selection: usize,
    get_acc_default_action: usize,
    acc_select: usize,
    acc_location: unsafe extern "system" fn(
        *mut c_void,
        *mut i32,
        *mut i32,
        *mut i32,
        *mut i32,
        RawVariant,
    ) -> i32,
    acc_navigate: usize,
    acc_hit_test: usize,
    acc_do_default_action: usize,
    put_acc_name: usize,
    put_acc_value: usize,
}

#[repr(C)]
struct RawDropSourceVtbl {
    base: RawIUnknownVtbl,
    query_continue_drag:
        unsafe extern "system" fn(*mut c_void, i32, u32) -> windows_sys::core::HRESULT,
    give_feedback: unsafe extern "system" fn(*mut c_void, DROPEFFECT) -> windows_sys::core::HRESULT,
}

#[repr(C)]
struct SimpleDropSource {
    vtbl: *const RawDropSourceVtbl,
    refs: AtomicU32,
}

fn guid_eq(a: &windows_sys::core::GUID, b: &windows_sys::core::GUID) -> bool {
    a.data1 == b.data1 && a.data2 == b.data2 && a.data3 == b.data3 && a.data4 == b.data4
}

fn variant_child_self() -> RawVariant {
    RawVariant {
        vt: 3,
        w_reserved1: 0,
        w_reserved2: 0,
        w_reserved3: 0,
        data: [0; 16],
    }
}

pub(crate) unsafe fn caret_accessible_rect(hwnd: HWND) -> Option<RECT> {
    const OBJID_CARET_V: i32 = -8;
    const IID_IACCESSIBLE_RAW: windows_sys::core::GUID =
        windows_sys::core::GUID::from_u128(0x618736e0_3c3d_11cf_810c_00aa00389b71);

    if hwnd.is_null() || IsWindow(hwnd) == 0 {
        return None;
    }

    let mut obj: *mut c_void = std::ptr::null_mut();
    let hr = AccessibleObjectFromWindow(hwnd, OBJID_CARET_V, &IID_IACCESSIBLE_RAW, &mut obj);
    if hr != S_OK_HR || obj.is_null() {
        return None;
    }

    let acc = obj as *mut RawIAccessible;
    let mut left = 0i32;
    let mut top = 0i32;
    let mut width = 0i32;
    let mut height = 0i32;
    let child = variant_child_self();
    let location_hr = ((*(*acc).vtbl).acc_location)(
        obj,
        &mut left,
        &mut top,
        &mut width,
        &mut height,
        child,
    );
    release_raw_com(obj);
    if location_hr != S_OK_HR || width <= 0 || height <= 0 {
        return None;
    }

    Some(RECT {
        left,
        top,
        right: left + width,
        bottom: top + height,
    })
}

unsafe extern "system" fn drop_source_query_interface(
    this: *mut c_void,
    riid: *const windows_sys::core::GUID,
    out: *mut *mut c_void,
) -> windows_sys::core::HRESULT {
    if out.is_null() {
        return E_POINTER_HR;
    }
    *out = std::ptr::null_mut();
    if riid.is_null() {
        return E_NOINTERFACE_HR;
    }
    if guid_eq(&*riid, &IID_IUNKNOWN_RAW) || guid_eq(&*riid, &IID_IDROPSOURCE_RAW) {
        *out = this;
        drop_source_add_ref(this);
        S_OK_HR
    } else {
        E_NOINTERFACE_HR
    }
}

unsafe extern "system" fn drop_source_add_ref(this: *mut c_void) -> u32 {
    let obj = this as *mut SimpleDropSource;
    (*obj).refs.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn drop_source_release(this: *mut c_void) -> u32 {
    let obj = this as *mut SimpleDropSource;
    let remain = (*obj).refs.fetch_sub(1, Ordering::Release) - 1;
    if remain == 0 {
        std::sync::atomic::fence(Ordering::Acquire);
        drop(Box::from_raw(obj));
    }
    remain
}

unsafe extern "system" fn drop_source_query_continue_drag(
    _this: *mut c_void,
    escape_pressed: i32,
    key_state: u32,
) -> windows_sys::core::HRESULT {
    if escape_pressed != 0 {
        return DRAGDROP_S_CANCEL_HR;
    }
    if key_state & MK_LBUTTON_FLAG == 0 {
        return DRAGDROP_S_DROP_HR;
    }
    S_OK_HR
}

unsafe extern "system" fn drop_source_give_feedback(
    _this: *mut c_void,
    _effect: DROPEFFECT,
) -> windows_sys::core::HRESULT {
    DRAGDROP_S_USEDEFAULTCURSORS_HR
}

static DROP_SOURCE_VTBL: RawDropSourceVtbl = RawDropSourceVtbl {
    base: RawIUnknownVtbl {
        query_interface: drop_source_query_interface,
        add_ref: drop_source_add_ref,
        release: drop_source_release,
    },
    query_continue_drag: drop_source_query_continue_drag,
    give_feedback: drop_source_give_feedback,
};

pub(crate) unsafe fn release_raw_com(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let unk = ptr as *mut RawIUnknown;
    ((*(*unk).vtbl).release)(ptr);
}

pub(crate) unsafe fn create_drop_source() -> *mut c_void {
    Box::into_raw(Box::new(SimpleDropSource {
        vtbl: &DROP_SOURCE_VTBL,
        refs: AtomicU32::new(1),
    })) as *mut c_void
}
use std::ptr::null_mut;

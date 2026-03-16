use std::ffi::c_void;
use std::ptr::{null, null_mut};

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::{
        BeginPaint, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint, FillRect, GetDC, InvalidateRect,
        PAINTSTRUCT, ReleaseDC, SelectObject,
    },
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::*,
};

use crate::i18n::{tr, translate};
use crate::ui::{
    draw_round_fill, draw_round_rect, draw_text_ex, rgb, Theme, SETTINGS_CONTENT_Y, SETTINGS_NAV_W,
};

pub const SETTINGS_VIEWPORT_MASK_H: i32 = 14;
pub const WM_SETTINGS_DROPDOWN_SELECTED: u32 = WM_APP + 91;
const DROPDOWN_CLASS: &str = "ZsClipDropdownPopup";
const DROPDOWN_ITEM_H: i32 = 38;
const DROPDOWN_PAD: i32 = 6;
const DT_LEFT_FLAG: u32 = 0x0000;
const DT_WORDBREAK_FLAG: u32 = 0x0010;
const DT_NOPREFIX_FLAG: u32 = 0x0800;
const DT_EDITCONTROL_FLAG: u32 = 0x2000;
const EM_SETMARGINS_MSG: u32 = 0x00D3;
const EM_SETPASSWORDCHAR_MSG: u32 = 0x00CC;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsComponentKind {
    Toggle,
    Dropdown,
    Button,
    AccentButton,
}

#[link(name = "dwmapi")]
unsafe extern "system" {
    fn DwmSetWindowAttribute(hwnd: HWND, dwattribute: u32, pvattribute: *const c_void, cbattribute: u32) -> i32;
}

#[link(name = "uxtheme")]
unsafe extern "system" {
    fn SetWindowTheme(hwnd: HWND, pszsubappid: *const u16, pszsubidlist: *const u16) -> i32;
}

fn to_wide(s: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;
    OsStr::new(s).encode_wide().chain(once(0)).collect()
}

pub fn settings_viewport_rect(window_rc: &RECT) -> RECT {
    RECT {
        left: SETTINGS_NAV_W,
        top: SETTINGS_CONTENT_Y,
        right: window_rc.right,
        bottom: window_rc.bottom,
    }
}

pub fn settings_viewport_mask_rect(window_rc: &RECT) -> RECT {
    RECT {
        left: SETTINGS_NAV_W,
        top: SETTINGS_CONTENT_Y,
        right: window_rc.right,
        bottom: SETTINGS_CONTENT_Y + SETTINGS_VIEWPORT_MASK_H,
    }
}

pub fn settings_safe_paint_rect(window_rc: &RECT) -> RECT {
    let mask = settings_viewport_mask_rect(window_rc);
    RECT {
        left: mask.left,
        top: mask.bottom,
        right: mask.right,
        bottom: window_rc.bottom,
    }
}

pub fn settings_child_visible(new_y: i32, h: i32, viewport: &RECT) -> bool {
    let safe_top = viewport.top + SETTINGS_VIEWPORT_MASK_H;
    new_y >= safe_top && new_y + h > safe_top && new_y < viewport.bottom
}

#[allow(unreachable_code)]
pub fn settings_dropdown_label_for_max_items(max_items: usize) -> &'static str {
    return match max_items {
        100 => "100",
        200 => "200",
        500 => "500",
        1000 => "1000",
        3000 => "3000",
        _ => tr("无限制", "Unlimited"),
    };
    match max_items {
        100 => "100",
        200 => "200",
        500 => "500",
        1000 => "1000",
        3000 => "3000",
        _ => "无限制",
    }
}

pub fn settings_dropdown_index_for_max_items(max_items: usize) -> usize {
    match max_items {
        100 => 0,
        200 => 1,
        500 => 2,
        1000 => 3,
        3000 => 4,
        _ => 5,
    }
}

pub fn settings_dropdown_max_items_from_label(label: &str) -> usize {
    match label.trim() {
        "100" => 100,
        "200" => 200,
        "500" => 500,
        "1000" => 1000,
        "3000" => 3000,
        _ => 0,
    }
}

#[allow(unreachable_code)]
pub fn settings_dropdown_label_for_pos_mode(mode: &str) -> &'static str {
    return match mode {
        "fixed" => tr("固定位置", "Fixed Position"),
        "last" => tr("上次位置", "Last Position"),
        _ => tr("跟随鼠标", "Follow Mouse"),
    };
    match mode {
        "fixed" => "固定位置",
        "last" => "上次位置",
        _ => "跟随鼠标",
    }
}

pub fn settings_dropdown_index_for_pos_mode(mode: &str) -> usize {
    match mode {
        "fixed" => 1,
        "last" => 2,
        _ => 0,
    }
}

#[allow(unreachable_code)]
pub fn settings_dropdown_pos_mode_from_label(label: &str) -> String {
    return match label.trim() {
        "固定位置" | "Fixed Position" => "fixed".to_string(),
        "上次位置" | "Last Position" => "last".to_string(),
        _ => "mouse".to_string(),
    };
    match label.trim() {
        "固定位置" => "fixed".to_string(),
        "上次位置" => "last".to_string(),
        _ => "mouse".to_string(),
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
    let class_name = match kind {
        SettingsComponentKind::Toggle
        | SettingsComponentKind::Dropdown
        | SettingsComponentKind::Button
        | SettingsComponentKind::AccentButton => "BUTTON",
    };
    let translated = translate(text);
    let hwnd = CreateWindowExW(
        0,
        to_wide(class_name).as_ptr(),
        to_wide(translated.as_ref()).as_ptr(),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_OWNERDRAW as u32),
        x,
        y,
        w,
        h,
        parent,
        id as usize as _,
        GetModuleHandleW(null()),
        null_mut(),
    );
    if !hwnd.is_null() {
        SendMessageW(hwnd, WM_SETFONT, font as usize, 1);
    }
    hwnd
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
    let translated = translate(text);
    let hwnd = CreateWindowExW(
        0,
        to_wide("STATIC").as_ptr(),
        to_wide(translated.as_ref()).as_ptr(),
        WS_CHILD | WS_VISIBLE,
        x,
        y,
        w,
        h,
        parent,
        null_mut(),
        GetModuleHandleW(null()),
        null(),
    );
    if !hwnd.is_null() {
        SendMessageW(hwnd, WM_SETFONT, font as usize, 1);
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
    let hdc = GetDC(parent);
    if hdc.is_null() {
        return min_h.max(24);
    }
    let old = if !font.is_null() { SelectObject(hdc, font) } else { null_mut() };
    let mut rc = RECT { left: 0, top: 0, right: w.max(1), bottom: 0 };
    let translated = translate(text);
    let wt = to_wide(translated.as_ref());
    DrawTextW(
        hdc,
        wt.as_ptr(),
        -1,
        &mut rc,
        DT_LEFT_FLAG | DT_WORDBREAK_FLAG | DT_NOPREFIX_FLAG | DT_EDITCONTROL_FLAG,
    );
    if !old.is_null() {
        SelectObject(hdc, old);
    }
    ReleaseDC(parent, hdc);
    (min_h).max((rc.bottom - rc.top) + 4)
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
    let hwnd = create_settings_label(parent, text, x, y, w, h, font);
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
    let style = WS_CHILD | WS_VISIBLE | WS_TABSTOP | (ES_AUTOHSCROLL as u32);
    let hwnd = CreateWindowExW(
        WS_EX_CLIENTEDGE,
        to_wide("EDIT").as_ptr(),
        to_wide(text).as_ptr(),
        style,
        x,
        y,
        w,
        28,
        parent,
        id as usize as _,
        GetModuleHandleW(null()),
        null(),
    );
    if !hwnd.is_null() {
        SendMessageW(hwnd, WM_SETFONT, font as usize, 1);
        let theme = if crate::ui::is_dark_mode() { "DarkMode_Explorer" } else { "Explorer" };
        SetWindowTheme(hwnd, to_wide(theme).as_ptr(), null());
        SendMessageW(
            hwnd,
            EM_SETMARGINS_MSG,
            (EC_LEFTMARGIN | EC_RIGHTMARGIN) as WPARAM,
            ((6 & 0xffff) | ((6 & 0xffff) << 16)) as LPARAM,
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
        SendMessageW(hwnd, EM_SETPASSWORDCHAR_MSG, '*' as usize, 0);
        InvalidateRect(hwnd, null(), 1);
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
    let hwnd = CreateWindowExW(
        WS_EX_CLIENTEDGE,
        to_wide("LISTBOX").as_ptr(),
        to_wide("").as_ptr(),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | (LBS_NOTIFY as u32) | (WS_VSCROLL as u32),
        x,
        y,
        w,
        h,
        parent,
        id as usize as _,
        GetModuleHandleW(null()),
        null(),
    );
    if !hwnd.is_null() {
        SendMessageW(hwnd, WM_SETFONT, font as usize, 1);
        SetWindowTheme(hwnd, to_wide("Explorer").as_ptr(), null());
    }
    hwnd
}

pub unsafe fn draw_settings_toggle_component(
    hdc: *mut c_void,
    rc: &RECT,
    hover: bool,
    checked: bool,
    th: Theme,
) {
    let bg = CreateSolidBrush(th.surface);
    FillRect(hdc, rc, bg);
    DeleteObject(bg as _);

    let tw = 40;
    let thh = 20;
    let cx = rc.left + (rc.right - rc.left - tw) / 2;
    let cy = rc.top + (rc.bottom - rc.top - thh) / 2;
    let track = RECT { left: cx, top: cy, right: cx + tw, bottom: cy + thh };

    if checked {
        draw_round_rect(hdc, &track, th.accent, th.accent, 10);
        let k = 14;
        let ky = cy + (thh - k) / 2;
        let krc = RECT { left: cx + tw - k - 3, top: ky, right: cx + tw - 3, bottom: ky + k };
        draw_round_rect(hdc, &krc, rgb(255, 255, 255), rgb(255, 255, 255), 7);
    } else {
        let border = if hover { rgb(28, 28, 28) } else { rgb(136, 136, 136) };
        draw_round_rect(hdc, &track, th.bg, border, 10);
        let k = 12;
        let ky = cy + (thh - k) / 2;
        let krc = RECT { left: cx + 4, top: ky, right: cx + 16, bottom: ky + k };
        let knob_color = if hover { rgb(28, 28, 28) } else { rgb(102, 102, 102) };
        draw_round_rect(hdc, &krc, knob_color, knob_color, 6);
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
    let rr = RECT { left: rc.left + 1, top: rc.top + 1, right: rc.right - 1, bottom: rc.bottom - 1 };
    match kind {
        SettingsComponentKind::Dropdown => {
            draw_settings_dropdown_button(hdc, &rr, text, hover, pressed, th);
        }
        SettingsComponentKind::AccentButton => {
            let fill = if pressed { th.accent_pressed } else if hover { th.accent_hover } else { th.accent };
            draw_round_rect(hdc, &rr, fill, fill, 4);
            draw_text_ex(hdc, text, &rr, rgb(255, 255, 255), 14, false, true, "Segoe UI Variable Text");
        }
        SettingsComponentKind::Button => {
            let fill = if pressed { th.button_pressed } else if hover { th.button_hover } else { th.button_bg };
            let border = if pressed || hover { rgb(196, 196, 196) } else { rgb(204, 204, 204) };
            draw_round_rect(hdc, &rr, fill, border, 4);
            draw_text_ex(hdc, text, &rr, th.text, 14, false, true, "Segoe UI Variable Text");
        }
        SettingsComponentKind::Toggle => {}
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
    let fill = if pressed { th.button_pressed } else { th.surface };
    let border = if pressed { th.accent } else { th.control_stroke };
    draw_round_rect(hdc, &rr, fill, border, 6);

    let text_rc = RECT { left: rr.left + 12, top: rr.top, right: rr.right - 28, bottom: rr.bottom };
    draw_text_ex(hdc, text, &text_rc, th.text, 14, false, false, "Segoe UI Variable Text");

    let arrow_rc = RECT { left: rr.right - 24, top: rr.top, right: rr.right - 8, bottom: rr.bottom };
    draw_text_ex(hdc, "▼", &arrow_rc, th.text_muted, 10, false, true, "Segoe UI Symbol");
}

unsafe fn apply_dark_mode_to_window(hwnd: HWND) {
    if crate::ui::is_dark_mode() {
        let val: u32 = 1;
        let _ = DwmSetWindowAttribute(hwnd, 20, &val as *const u32 as _, 4);
    }
    let theme_name = if crate::ui::is_dark_mode() { "DarkMode_Explorer" } else { "Explorer" };
    let _ = SetWindowTheme(hwnd, to_wide(theme_name).as_ptr(), null());
}

unsafe fn apply_window_corner_preference(hwnd: HWND) {
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

struct DropdownPopupState {
    parent: HWND,
    control_id: isize,
    items: Vec<String>,
    selected: i32,
    hover: i32,
    item_h: i32,
}

unsafe fn dropdown_index_from_y(st: &DropdownPopupState, y: i32) -> i32 {
    if y < DROPDOWN_PAD || y >= DROPDOWN_PAD + st.item_h * st.items.len() as i32 {
        -1
    } else {
        ((y - DROPDOWN_PAD) / st.item_h).clamp(0, st.items.len() as i32 - 1)
    }
}

unsafe extern "system" fn dropdown_popup_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            let cs = &*(lparam as *const CREATESTRUCTW);
            let ptr = cs.lpCreateParams as *mut DropdownPopupState;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, ptr as isize);
            apply_window_corner_preference(hwnd);
            apply_dark_mode_to_window(hwnd);
            0
        }
        WM_MOUSEACTIVATE => MA_NOACTIVATE as isize,
        WM_MOUSEMOVE => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut DropdownPopupState;
            if ptr.is_null() {
                return 0;
            }
            let st = &mut *ptr;
            let y = ((lparam >> 16) & 0xffff) as u16 as i16 as i32;
            let hover = dropdown_index_from_y(st, y);
            if hover != st.hover {
                st.hover = hover;
                InvalidateRect(hwnd, null(), 0);
            }
            0
        }
        WM_LBUTTONDOWN | WM_LBUTTONUP => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut DropdownPopupState;
            if !ptr.is_null() {
                let st = &mut *ptr;
                let y = ((lparam >> 16) & 0xffff) as u16 as i16 as i32;
                let idx = dropdown_index_from_y(st, y);
                if idx >= 0 {
                    SendMessageW(st.parent, WM_SETTINGS_DROPDOWN_SELECTED, st.control_id as usize, idx as isize);
                }
            }
            DestroyWindow(hwnd);
            0
        }
        WM_ACTIVATE => {
            if (wparam & 0xffff) as u32 == WA_INACTIVE {
                DestroyWindow(hwnd);
            }
            0
        }
        WM_PAINT => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut DropdownPopupState;
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            if !hdc.is_null() {
                let th = Theme::default();
                let mut rc: RECT = std::mem::zeroed();
                GetClientRect(hwnd, &mut rc);
                let w = (rc.right - rc.left).max(1);
                let h = (rc.bottom - rc.top).max(1);
                let bg = CreateSolidBrush(th.bg);
                FillRect(hdc, &rc, bg);
                DeleteObject(bg as _);
                let shell = RECT { left: 0, top: 0, right: w, bottom: h };
                draw_round_rect(hdc as _, &shell, th.surface, th.stroke, 8);
                if !ptr.is_null() {
                    let st = &mut *ptr;
                    for (idx, item) in st.items.iter().enumerate() {
                        let top = DROPDOWN_PAD + idx as i32 * st.item_h;
                        let item_rc = RECT { left: DROPDOWN_PAD, top, right: w - DROPDOWN_PAD, bottom: top + st.item_h };
                        let selected = st.selected == idx as i32;
                        if selected {
                            let fill = th.nav_sel_fill;
                            draw_round_fill(hdc as _, &item_rc, fill, 6);
                        }
                        if selected {
                            let cy = (item_rc.top + item_rc.bottom) / 2;
                            let bar = RECT { left: item_rc.left + 4, top: cy - 8, right: item_rc.left + 7, bottom: cy + 8 };
                            draw_round_fill(hdc as _, &bar, th.accent, 2);
                        }
                        let text_rc = RECT { left: item_rc.left + 18, top: item_rc.top, right: item_rc.right - 12, bottom: item_rc.bottom };
                        draw_text_ex(hdc as _, item, &text_rc, th.text, 14, false, false, "Segoe UI Variable Text");
                    }
                }
                EndPaint(hwnd, &ps);
            }
            0
        }
        WM_DESTROY => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut DropdownPopupState;
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn ensure_dropdown_popup_class() {
    let hinstance = GetModuleHandleW(null());
    let cname = to_wide(DROPDOWN_CLASS);
    let mut wc: WNDCLASSEXW = std::mem::zeroed();
    wc.cbSize = core::mem::size_of::<WNDCLASSEXW>() as u32;
    wc.style = CS_DROPSHADOW;
    wc.lpfnWndProc = Some(dropdown_popup_proc);
    wc.hInstance = hinstance;
    wc.hCursor = LoadCursorW(null_mut(), IDC_ARROW);
    wc.hbrBackground = null_mut();
    wc.lpszClassName = cname.as_ptr();
    RegisterClassExW(&wc);
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
    let hinstance = GetModuleHandleW(null());
    let items_vec = items.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let height = DROPDOWN_PAD * 2 + DROPDOWN_ITEM_H * items_vec.len() as i32;
    let state = Box::new(DropdownPopupState {
        parent,
        control_id,
        items: items_vec,
        selected: selected as i32,
        hover: -1,
        item_h: DROPDOWN_ITEM_H,
    });
    let hwnd = CreateWindowExW(
        WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
        to_wide(DROPDOWN_CLASS).as_ptr(),
        to_wide("").as_ptr(),
        WS_POPUP | WS_VISIBLE,
        anchor_rect.left,
        anchor_rect.bottom + 6,
        width.max(anchor_rect.right - anchor_rect.left),
        height,
        parent,
        null_mut(),
        hinstance,
        Box::into_raw(state) as _,
    );
    if !hwnd.is_null() {
        ShowWindow(hwnd, SW_SHOWNA);
        InvalidateRect(hwnd, null(), 0);
    }
    hwnd
}

// BUILD_MARKER: v27_win3uirust_direct_paint_dropdown
use std::ffi::c_void;
use std::ptr::{null, null_mut};

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::{
        BeginPaint, CreateSolidBrush, DeleteObject, EndPaint, FillRect, InvalidateRect, PAINTSTRUCT,
    },
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::*,
};

use crate::ui::{
    draw_round_fill, draw_round_rect, draw_text_ex, rgb, Theme,
    SETTINGS_CONTENT_Y, SETTINGS_NAV_W,
};

pub const SETTINGS_VIEWPORT_MASK_H: i32 = 14;
pub const WM_SETTINGS_DROPDOWN_SELECTED: u32 = WM_APP + 91;
const DROPDOWN_CLASS: &str = "Win3uirustDropdownPopup";
const DROPDOWN_ITEM_H: i32 = 38;
const DROPDOWN_PAD: i32 = 6;

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

pub fn settings_dropdown_label_for_max_items(max_items: usize) -> &'static str {
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

pub fn settings_dropdown_label_for_pos_mode(mode: &str) -> &'static str {
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

pub fn settings_dropdown_pos_mode_from_label(label: &str) -> String {
    match label.trim() {
        "固定位置" => "fixed".to_string(),
        "上次位置" => "last".to_string(),
        _ => "mouse".to_string(),
    }
}

pub unsafe fn draw_settings_dropdown_button(
    hdc: *mut c_void,
    rc: &RECT,
    text: &str,
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
    let fill = if pressed {
        th.button_pressed
    } else if hover {
        th.button_hover
    } else {
        th.surface
    };
    let border = if hover { th.accent } else { th.control_stroke };
    draw_round_rect(hdc, &rr, fill, border, 6);

    let text_rc = RECT {
        left: rr.left + 12,
        top: rr.top,
        right: rr.right - 28,
        bottom: rr.bottom,
    };
    draw_text_ex(hdc, text, &text_rc, th.text, 14, false, false, "Segoe UI Variable Text");

    let arrow_rc = RECT {
        left: rr.right - 24,
        top: rr.top,
        right: rr.right - 8,
        bottom: rr.bottom,
    };
    draw_text_ex(hdc, "", &arrow_rc, th.text_muted, 12, false, true, "Segoe Fluent Icons");
}

unsafe fn apply_dark_mode_to_window(hwnd: HWND) {
    if crate::ui::is_dark_mode() {
        let val: u32 = 1;
        let _ = DwmSetWindowAttribute(hwnd, 20, &val as *const u32 as _, 4);
    }
    let _ = SetWindowTheme(
        hwnd,
        to_wide(if crate::ui::is_dark_mode() { "DarkMode_Explorer" } else { "Explorer" }).as_ptr(),
        null(),
    );
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

#[allow(dead_code)]
unsafe fn apply_theme_to_menu(_hmenu: *mut c_void) {
    // 预留：后续如果需要给原生菜单补主题，可以在这里恢复 uxtheme 动态调用。
}

#[allow(dead_code)]
pub unsafe fn show_settings_dropdown_menu(
    parent: HWND,
    anchor: HWND,
    items: &[&str],
    selected: usize,
) -> Option<usize> {
    let menu = CreatePopupMenu();
    if menu.is_null() { return None; }
    apply_theme_to_menu(menu as _);
    const BASE_ID: usize = 53000;
    for (idx, item) in items.iter().enumerate() {
        let flags = if idx == selected { MF_STRING | MF_CHECKED } else { MF_STRING };
        AppendMenuW(menu, flags, BASE_ID + idx, to_wide(item).as_ptr());
    }
    let mut rc: RECT = std::mem::zeroed();
    GetWindowRect(anchor, &mut rc);
    SetForegroundWindow(parent);
    let cmd = TrackPopupMenu(
        menu,
        TPM_LEFTALIGN | TPM_TOPALIGN | TPM_RETURNCMD | TPM_RIGHTBUTTON,
        rc.left,
        rc.bottom + 6,
        0,
        parent,
        null(),
    ) as usize;
    PostMessageW(parent, WM_NULL, 0, 0);
    DestroyMenu(menu);
    if cmd >= BASE_ID && cmd < BASE_ID + items.len() {
        Some(cmd - BASE_ID)
    } else {
        None
    }
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
            if ptr.is_null() { return 0; }
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
                        let hovered = st.hover == idx as i32;
                        let selected = st.selected == idx as i32;
                        if hovered || selected {
                            let fill = if selected { th.nav_sel_fill } else if crate::ui::is_dark_mode() { rgb(60, 60, 60) } else { rgb(245, 245, 245) };
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

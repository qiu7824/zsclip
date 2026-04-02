use std::ffi::c_void;
use std::ptr::{null, null_mut};
use std::cmp::max;

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::{
        BeginPaint, CreateFontW, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint, FillRect, GetDC,
        GetStockObject, InvalidateRect, PAINTSTRUCT, ReleaseDC, SelectObject, SetBkMode, SetTextColor,
        DEFAULT_GUI_FONT,
    },
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::*,
};

use crate::i18n::{tr, translate};
use crate::settings_model::SETTINGS_PAGE_COUNT;
use crate::ui::{
    draw_round_fill, draw_round_rect, draw_text_ex, rgb,
    settings_content_y_scaled, settings_nav_w_scaled, settings_scale, ui_display_font_family,
    ui_icon_font_family, ui_text_font_family, Theme, UiRect,
    DT_CENTER, DT_SINGLELINE, DT_VCENTER,
};
use crate::win_buffered_paint::{begin_buffered_paint, end_buffered_paint};
use crate::win_system_ui::{create_font_px, scale_for_window, window_dpi};

#[link(name = "user32")]
unsafe extern "system" {
    fn ShowScrollBar(hwnd: HWND, wbar: i32, bshow: i32) -> i32;
}

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

#[derive(Clone, Copy, Debug)]
pub struct SettingsCtrlReg {
    pub hwnd: HWND,
    pub page: usize,
    pub bounds: UiRect,
    pub scrollable: bool,
}

impl SettingsCtrlReg {
    pub const fn new(hwnd: HWND, page: usize, x: i32, y: i32, w: i32, h: i32, scrollable: bool) -> Self {
        Self {
            hwnd,
            page,
            bounds: UiRect::new(x, y, x + w, y + h),
            scrollable,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SettingsCtrlSlot {
    pub hwnd: HWND,
    pub page: usize,
    pub bounds: UiRect,
}

pub struct SettingsUiRegistry {
    built_pages: [bool; SETTINGS_PAGE_COUNT],
    page_ctrls: Vec<Vec<HWND>>,
    regs: Vec<SettingsCtrlReg>,
    scroll_ctrls: Vec<SettingsCtrlSlot>,
}

impl SettingsUiRegistry {
    pub fn new() -> Self {
        Self {
            built_pages: [false; SETTINGS_PAGE_COUNT],
            page_ctrls: vec![Vec::new(); SETTINGS_PAGE_COUNT],
            regs: Vec::new(),
            scroll_ctrls: Vec::new(),
        }
    }

    pub fn is_built(&self, page: usize) -> bool {
        self.built_pages.get(page).copied().unwrap_or(false)
    }

    pub fn mark_built(&mut self, page: usize) {
        if let Some(slot) = self.built_pages.get_mut(page) {
            *slot = true;
        }
    }

    pub fn register(&mut self, reg: SettingsCtrlReg) {
        let page = reg.page.min(SETTINGS_PAGE_COUNT.saturating_sub(1));
        self.regs.push(reg);
        if let Some(list) = self.page_ctrls.get_mut(page) {
            list.push(reg.hwnd);
        }
        if reg.scrollable {
            self.scroll_ctrls.push(SettingsCtrlSlot { hwnd: reg.hwnd, page, bounds: reg.bounds });
        }
    }

    pub fn page_regs(&self, page: usize) -> impl Iterator<Item = &SettingsCtrlReg> {
        self.regs.iter().filter(move |reg| reg.page == page)
    }

    pub fn scroll_ctrls_for_page(&self, page: usize) -> impl Iterator<Item = &SettingsCtrlSlot> {
        self.scroll_ctrls.iter().filter(move |slot| slot.page == page)
    }

    pub unsafe fn clear_page(&mut self, page: usize) {
        let page = page.min(SETTINGS_PAGE_COUNT.saturating_sub(1));
        if let Some(ctrls) = self.page_ctrls.get_mut(page) {
            for hwnd in ctrls.drain(..) {
                if !hwnd.is_null() && IsWindow(hwnd) != 0 {
                    DestroyWindow(hwnd);
                }
            }
        }
        self.regs.retain(|reg| reg.page != page);
        self.scroll_ctrls
            .retain(|slot| slot.page != page && !slot.hwnd.is_null() && IsWindow(slot.hwnd) != 0);
        if let Some(flag) = self.built_pages.get_mut(page) {
            *flag = false;
        }
    }
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
        left: settings_nav_w_scaled(),
        top: settings_content_y_scaled(),
        right: window_rc.right,
        bottom: window_rc.bottom,
    }
}

pub fn settings_viewport_mask_rect(window_rc: &RECT) -> RECT {
    RECT {
        left: settings_nav_w_scaled(),
        top: settings_content_y_scaled(),
        right: window_rc.right,
        bottom: settings_content_y_scaled() + settings_scale(SETTINGS_VIEWPORT_MASK_H),
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
    let safe_top = viewport.top + settings_scale(SETTINGS_VIEWPORT_MASK_H);
    new_y >= safe_top && new_y + h > safe_top && new_y < viewport.bottom
}

pub fn settings_dropdown_label_for_max_items(max_items: usize) -> &'static str {
    match max_items {
        100 => "100",
        200 => "200",
        500 => "500",
        1000 => "1000",
        3000 => "3000",
        _ => tr("无限制", "Unlimited"),
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
        "fixed" => tr("固定位置", "Fixed Position"),
        "last" => tr("上次位置", "Last Position"),
        _ => tr("跟随鼠标", "Follow Mouse"),
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
        "固定位置" | "Fixed Position" => "fixed".to_string(),
        "上次位置" | "Last Position" => "last".to_string(),
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

pub unsafe fn set_settings_font(hwnd: HWND, hfont: *mut c_void) {
    if !hwnd.is_null() && !hfont.is_null() {
        SendMessageW(hwnd, WM_SETFONT, hfont as usize, 1);
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
    create_settings_component(
        parent,
        text,
        id,
        SettingsComponentKind::Button,
        x,
        y,
        w,
        scale_for_window(parent, 32),
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
    create_settings_component(
        parent,
        text,
        id,
        SettingsComponentKind::Dropdown,
        x,
        y,
        w,
        scale_for_window(parent, 32),
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
) -> (HWND, HWND, i32, i32, i32, i32, i32, i32) {
    const SS_CENTERIMAGE: u32 = 0x0200;
    let toggle_w = scale_for_window(parent, 44);
    let toggle_h = scale_for_window(parent, 24);
    let row_h = scale_for_window(parent, 32);
    let gap = scale_for_window(parent, 12);
    let label_w = max(scale_for_window(parent, 40), w - toggle_w - gap);
    let label_text = translate(text);
    let label = CreateWindowExW(
        0,
        to_wide("STATIC").as_ptr(),
        to_wide(label_text.as_ref()).as_ptr(),
        WS_CHILD | WS_VISIBLE | SS_CENTERIMAGE,
        x,
        y,
        label_w,
        row_h,
        parent,
        null_mut(),
        GetModuleHandleW(null()),
        null(),
    );
    set_settings_font(label, font);

    let btn_x = x + w - toggle_w;
    let btn_y = y + max(0, (row_h - toggle_h) / 2);
    let btn = create_settings_component(parent, "", id, SettingsComponentKind::Toggle, btn_x, btn_y, toggle_w, toggle_h, font);
    (label, btn, x, y, label_w, row_h, btn_x, btn_y)
}

pub unsafe fn create_settings_fonts(hwnd: HWND) -> (*mut c_void, *mut c_void, *mut c_void) {
    let nav_size = scale_for_window(hwnd, 18);
    let ui_size = scale_for_window(hwnd, 14);
    let title_size = scale_for_window(hwnd, 20);
    let nav: *mut c_void =
        CreateFontW(-nav_size, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, to_wide(ui_icon_font_family()).as_ptr()) as _;
    let ui: *mut c_void =
        CreateFontW(-ui_size, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, to_wide(ui_text_font_family()).as_ptr()) as _;
    let title: *mut c_void =
        CreateFontW(-title_size, 0, 0, 0, 600, 0, 0, 0, 1, 0, 0, 5, 0, to_wide(ui_display_font_family()).as_ptr()) as _;
    let default_ui: *mut c_void = if ui.is_null() { GetStockObject(DEFAULT_GUI_FONT) as _ } else { ui };
    let default_title: *mut c_void = if title.is_null() { GetStockObject(DEFAULT_GUI_FONT) as _ } else { title };
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
        scale_for_window(parent, 28),
        parent,
        id as usize as _,
        GetModuleHandleW(null()),
        null(),
    );
    if !hwnd.is_null() {
        SendMessageW(hwnd, WM_SETFONT, font as usize, 1);
        let theme = if crate::win_system_ui::is_dark_mode() { "DarkMode_Explorer" } else { "Explorer" };
        SetWindowTheme(hwnd, to_wide(theme).as_ptr(), null());
        SendMessageW(
            hwnd,
            EM_SETMARGINS_MSG,
            (EC_LEFTMARGIN | EC_RIGHTMARGIN) as WPARAM,
            ((scale_for_window(parent, 6) & 0xffff) | ((scale_for_window(parent, 6) & 0xffff) << 16)) as LPARAM,
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
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | (LBS_NOTIFY as u32) | WS_VSCROLL,
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
        ShowScrollBar(hwnd, SB_VERT, 0);
        ShowScrollBar(hwnd, SB_HORZ, 0);
    }
    hwnd
}

pub unsafe fn get_ctrl_text_wide(hwnd: HWND) -> Vec<u16> {
    let len = GetWindowTextLengthW(hwnd);
    let mut buf = vec![0u16; (len as usize) + 2];
    GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
    buf.truncate(len as usize);
    buf
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
    let old = SelectObject(hdc, font as _);
    SetBkMode(hdc, 1);
    SetTextColor(hdc, color);
    DrawTextW(hdc, text_w.as_ptr(), text_w.len() as i32, rc as *const _ as *mut _, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
    SelectObject(hdc, old);
    DeleteObject(font as _);
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

    let row_h = (rc.bottom - rc.top).max(24);
    let row_w = (rc.right - rc.left).max(48);
    let thh = ((row_h * 20) / 32).clamp(20, row_h - 4);
    let tw = ((thh * 40) / 20).clamp(thh + 12, row_w - 6);
    let cx = rc.left + (rc.right - rc.left - tw) / 2;
    let cy = rc.top + (rc.bottom - rc.top - thh) / 2;
    let track = RECT { left: cx, top: cy, right: cx + tw, bottom: cy + thh };
    let radius = (thh / 2).max(6);

    if checked {
        draw_round_rect(hdc, &track, th.accent, th.accent, radius);
        let k = ((thh * 14) / 20).max(12);
        let ky = cy + (thh - k) / 2;
        let knob_pad = ((thh - k) / 2).max(3);
        let krc = RECT { left: cx + tw - k - knob_pad, top: ky, right: cx + tw - knob_pad, bottom: ky + k };
        draw_round_rect(hdc, &krc, rgb(255, 255, 255), rgb(255, 255, 255), 7);
    } else {
        let border = if hover { rgb(28, 28, 28) } else { rgb(136, 136, 136) };
        draw_round_rect(hdc, &track, th.bg, border, radius);
        let k = ((thh * 12) / 20).max(10);
        let ky = cy + (thh - k) / 2;
        let knob_pad = ((thh - k) / 2).max(4);
        let krc = RECT { left: cx + knob_pad, top: ky, right: cx + knob_pad + k, bottom: ky + k };
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
    let control_h = (rr.bottom - rr.top).max(24);
    let text_px = ((control_h * 14) / 32).max(12);
    match kind {
        SettingsComponentKind::Dropdown => {
            draw_settings_dropdown_button(hdc, &rr, text, hover, pressed, th);
        }
        SettingsComponentKind::AccentButton => {
            let fill = if pressed { th.accent_pressed } else if hover { th.accent_hover } else { th.accent };
            draw_round_rect(hdc, &rr, fill, fill, 4);
            draw_text_ex(hdc, text, &rr, rgb(255, 255, 255), text_px, false, true, ui_text_font_family());
        }
        SettingsComponentKind::Button => {
            let fill = if pressed { th.button_pressed } else if hover { th.button_hover } else { th.button_bg };
            let border = if pressed || hover { rgb(196, 196, 196) } else { rgb(204, 204, 204) };
            draw_round_rect(hdc, &rr, fill, border, 4);
            draw_text_ex(hdc, text, &rr, th.text, text_px, false, true, ui_text_font_family());
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
    let control_h = (rr.bottom - rr.top).max(24);
    let text_px = ((control_h * 14) / 32).max(12);
    let arrow_px = ((control_h * 10) / 32).max(9);
    let text_pad = (control_h * 12 / 32).max(10);
    let arrow_w = (control_h * 20 / 32).max(18);
    let fill = if pressed { th.button_pressed } else { th.surface };
    let border = th.control_stroke;
    draw_round_rect(hdc, &rr, fill, border, 6);

    let text_rc = RECT {
        left: rr.left + text_pad,
        top: rr.top,
        right: rr.right - arrow_w,
        bottom: rr.bottom,
    };
    draw_text_ex(hdc, text, &text_rc, th.text, text_px, false, false, ui_text_font_family());

    let arrow_rc = RECT {
        left: rr.right - arrow_w,
        top: rr.top,
        right: rr.right - (control_h * 8 / 32).max(6),
        bottom: rr.bottom,
    };
    draw_text_ex(hdc, "\u{25BE}", &arrow_rc, th.text_muted, arrow_px, false, true, ui_icon_font_family());
}

unsafe fn apply_dark_mode_to_window(hwnd: HWND) {
    if crate::win_system_ui::is_dark_mode() {
        let val: u32 = 1;
        let _ = DwmSetWindowAttribute(hwnd, 20, &val as *const u32 as _, 4);
        let _ = DwmSetWindowAttribute(hwnd, 19, &val as *const u32 as _, 4);
    }
    let theme_name = if crate::win_system_ui::is_dark_mode() { "DarkMode_Explorer" } else { "Explorer" };
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
    scroll_top: i32,
    visible_rows: i32,
}

unsafe fn dropdown_index_from_y(st: &DropdownPopupState, y: i32) -> i32 {
    let pad = settings_scale(DROPDOWN_PAD);
    if y < pad || y >= pad + st.item_h * st.visible_rows {
        -1
    } else {
        let row = ((y - pad) / st.item_h).clamp(0, st.visible_rows - 1);
        (st.scroll_top + row).clamp(0, st.items.len() as i32 - 1)
    }
}

fn dropdown_max_scroll(st: &DropdownPopupState) -> i32 {
    (st.items.len() as i32 - st.visible_rows).max(0)
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
        WM_MOUSEWHEEL => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut DropdownPopupState;
            if ptr.is_null() {
                return 0;
            }
            let st = &mut *ptr;
            let delta = ((wparam >> 16) & 0xffff) as u16 as i16 as i32;
            let step = if delta > 0 { -1 } else { 1 };
            let next = (st.scroll_top + step).clamp(0, dropdown_max_scroll(st));
            if next != st.scroll_top {
                st.scroll_top = next;
                st.hover = -1;
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
        WM_ACTIVATE => 0,
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            let dpi = window_dpi(hwnd);
            crate::ui::set_settings_ui_dpi(dpi);
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut DropdownPopupState;
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            if !hdc.is_null() {
                let mut rc: RECT = std::mem::zeroed();
                GetClientRect(hwnd, &mut rc);
                let paint_target = begin_buffered_paint(hdc, &rc);
                let memdc = if let Some((_, pdc)) = paint_target { pdc } else { hdc };
                let th = Theme::default();
                let w = (rc.right - rc.left).max(1);
                let h = (rc.bottom - rc.top).max(1);
                let bg = CreateSolidBrush(th.bg);
                FillRect(memdc, &rc, bg);
                DeleteObject(bg as _);
                let shell = RECT { left: 0, top: 0, right: w, bottom: h };
                draw_round_rect(memdc as _, &shell, th.surface, th.stroke, 8);
                if !ptr.is_null() {
                    let st = &mut *ptr;
                    let start = st.scroll_top.max(0) as usize;
                    let end = (st.scroll_top + st.visible_rows).min(st.items.len() as i32).max(0) as usize;
                    for (visible_idx, idx) in (start..end).enumerate() {
                        let item = &st.items[idx];
                        let pad = settings_scale(DROPDOWN_PAD);
                        let top = pad + visible_idx as i32 * st.item_h;
                        let item_rc = RECT { left: pad, top, right: w - pad, bottom: top + st.item_h };
                        let selected = st.selected == idx as i32;
                        if selected {
                            let fill = th.nav_sel_fill;
                            draw_round_fill(memdc as _, &item_rc, fill, 6);
                        }
                        if selected {
                            let cy = (item_rc.top + item_rc.bottom) / 2;
                            let bar = RECT { left: item_rc.left + 4, top: cy - 8, right: item_rc.left + 7, bottom: cy + 8 };
                            draw_round_fill(memdc as _, &bar, th.accent, 2);
                        }
                        let text_rc = RECT {
                            left: item_rc.left + settings_scale(18),
                            top: item_rc.top,
                            right: item_rc.right - settings_scale(12),
                            bottom: item_rc.bottom,
                        };
                        draw_text_ex(memdc as _, item, &text_rc, th.text, settings_scale(14), false, false, ui_text_font_family());
                    }
                    if dropdown_max_scroll(st) > 0 {
                        if st.scroll_top > 0 {
                            let top_hint = RECT {
                                left: w - settings_scale(22),
                                top: settings_scale(6),
                                right: w - settings_scale(8),
                                bottom: settings_scale(20),
                            };
                            draw_text_ex(memdc as _, "\u{25B4}", &top_hint, th.text_muted, settings_scale(8), false, true, ui_icon_font_family());
                        }
                        if st.scroll_top < dropdown_max_scroll(st) {
                            let bottom_hint = RECT {
                                left: w - settings_scale(22),
                                top: h - settings_scale(20),
                                right: w - settings_scale(8),
                                bottom: h - settings_scale(6),
                            };
                            draw_text_ex(memdc as _, "\u{25BE}", &bottom_hint, th.text_muted, settings_scale(8), false, true, ui_icon_font_family());
                        }
                    }
                }
                if let Some((paint_buf, _)) = paint_target {
                    end_buffered_paint(paint_buf, true);
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
    let visible_rows = (items_vec.len() as i32).clamp(1, 8);
    let height = settings_scale(DROPDOWN_PAD) * 2 + settings_scale(DROPDOWN_ITEM_H) * visible_rows;
    let max_scroll = (items_vec.len() as i32 - visible_rows).max(0);
    let scroll_top = (selected as i32 - visible_rows / 2).clamp(0, max_scroll);
    let state = Box::new(DropdownPopupState {
        parent,
        control_id,
        items: items_vec,
        selected: selected as i32,
        hover: -1,
        item_h: settings_scale(DROPDOWN_ITEM_H),
        scroll_top,
        visible_rows,
    });
    let hwnd = CreateWindowExW(
        WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
        to_wide(DROPDOWN_CLASS).as_ptr(),
        to_wide("").as_ptr(),
        WS_POPUP,
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
        SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            anchor_rect.left,
            anchor_rect.bottom + 6,
            width.max(anchor_rect.right - anchor_rect.left),
            height,
            SWP_SHOWWINDOW | SWP_NOACTIVATE,
        );
    }
    hwnd
}


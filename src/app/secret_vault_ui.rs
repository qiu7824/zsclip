//! Native password/key manager. Sensitive values never pass through history row models.
use super::prelude::*;
use crate::platform::string::to_wide;
use crate::secret_vault::{self, EntrySummary, VaultSession};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};
use windows_sys::Win32::Graphics::Gdi::{
    CreateFontW, DeleteObject, FillRect, GetStockObject, RedrawWindow, SetBkColor, SetBkMode,
    SetDCBrushColor, SetTextColor, DC_BRUSH, HFONT,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcessId, GetCurrentThreadId};
use windows_sys::Win32::UI::Controls::{MEASUREITEMSTRUCT, WM_MOUSELEAVE};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, GetFocus, GetKeyState, IsWindowEnabled, SetFocus, VK_CONTROL, VK_ESCAPE,
    VK_RETURN, VK_SHIFT,
};
use windows_sys::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use zeroize::{Zeroize, Zeroizing};

const VAULT_CLASS: &str = "ZSClipSecretVault";
const ENTRY_CLASS: &str = "ZSClipSecretEntry";
const PASSWORD_CLASS: &str = "ZSClipSecretPasswordPrompt";
const MESSAGE_CLASS: &str = "ZSClipSecretMessage";
const CHECK_LOCK: u32 = WM_APP + 219;
const OWNER_ENABLED_PROPERTY: &str = "ZSClipVaultOwnerEnabled";
const LOGICAL_OWNER_PROPERTY: &str = "ZSClipVaultLogicalOwner";
const GENERATION_PROPERTY: &str = "ZSClipVaultGeneration";
const OWNER_GENERATION_PROPERTY: &str = "ZSClipVaultOwnerGeneration";
const ABORTED_PROPERTY: &str = "ZSClipVaultAborted";
const DEFAULT_FOCUS_PROPERTY: &str = "ZSClipVaultDefaultFocus";
static NEXT_WINDOW_GENERATION: AtomicUsize = AtomicUsize::new(1);
const LOCK_TIMER: usize = 4130;
const ID_STATUS: usize = 4101;
const STATUS_TIMER: usize = 4131;
const ID_LIST: usize = 4103;
const ROW_HOVER_PROPERTY: &str = "ZSClipVaultHoveredRow";
const ID_ADD: usize = 4104;
const ID_EDIT: usize = 4105;
const ID_DELETE: usize = 4106;
const ID_PASTE: usize = 4107;
const ID_MANAGE: usize = 4108;
const ID_REMOVE_PASSWORD: usize = 4109;
const ID_CLOSE: usize = 4110;
const ID_LABEL: usize = 4121;
const ID_NOTES: usize = 4122;
const ID_SECRET: usize = 4123;
const ID_REVEAL: usize = 4124;
const ID_HIDDEN: usize = 4125;
const ID_CONFIRM: usize = 4126;
const ID_OK: usize = 4127;
const ID_HEADING: usize = 4128;
const EM_EMPTYUNDOBUFFER_: u32 = 0x00CD;
const EM_SETLIMITTEXT_: u32 = 0x00C5;
// Deliberately above the storage byte limit so a too-long paste is rejected,
// rather than silently saved after an edit control truncates it.
const MAX_SECRET_CHARS: usize = 1_048_576;

struct VaultWindow {
    parent: HWND,
    embedded: bool,
    restore_noactivate: bool,
    session: Option<VaultSession>,
    rows: Vec<EntrySummary>,
    target: Option<(HWND, HWND)>,
    paste: Option<Zeroizing<String>>,
    font: HFONT,
}
type VaultRef = Rc<RefCell<VaultWindow>>;

struct EntryInput {
    label: String,
    notes: String,
    secret: Zeroizing<String>,
}

impl Drop for EntryInput {
    fn drop(&mut self) {
        self.label.zeroize();
        self.notes.zeroize();
    }
}

impl Drop for VaultWindow {
    fn drop(&mut self) {
        clear_rows(&mut self.rows);
    }
}

fn clear_rows(rows: &mut Vec<EntrySummary>) {
    for row in rows.iter_mut() {
        row.label.zeroize();
        row.notes.zeroize();
    }
    rows.clear();
}

struct EntryWindow {
    initial: Zeroizing<String>,
    label: String,
    notes: String,
    revealed: bool,
    result: Option<EntryInput>,
    font: HFONT,
}

struct PasswordWindow {
    setting: bool,
    result: Option<Zeroizing<String>>,
    font: HFONT,
}

struct MessageWindow {
    message: String,
    confirm: bool,
    result: bool,
    font: HFONT,
}

unsafe fn text(hwnd: HWND, value: &str) {
    SetWindowTextW(hwnd, to_wide(value).as_ptr());
}

unsafe fn control(parent: HWND, id: usize) -> HWND {
    GetDlgItem(parent, id as i32)
}

unsafe fn error(owner: HWND, message: &str) {
    message_dialog(owner, message, false);
}

unsafe fn theme_control_brush(hdc: HDC, theme: Theme, editable: bool) -> LRESULT {
    let background = if editable {
        theme.control_bg
    } else {
        theme.surface
    };
    SetBkMode(hdc, if editable { 2 } else { 1 });
    SetBkColor(hdc, background);
    SetTextColor(hdc, theme.text);
    SetDCBrushColor(hdc, background);
    GetStockObject(DC_BRUSH) as isize
}

unsafe fn theme_message(hwnd: HWND, msg: u32, wparam: WPARAM) -> Option<LRESULT> {
    match msg {
        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN | WM_CTLCOLORDLG => {
            Some(theme_control_brush(wparam as HDC, Theme::default(), false))
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX => {
            Some(theme_control_brush(wparam as HDC, Theme::default(), true))
        }
        WM_ERASEBKGND => {
            let hdc = wparam as HDC;
            let brush = theme_control_brush(hdc, Theme::default(), false);
            let mut bounds: RECT = zeroed();
            GetClientRect(hwnd, &mut bounds);
            FillRect(hdc, &bounds, brush as _);
            Some(1)
        }
        WM_THEMECHANGED | WM_SETTINGCHANGE => {
            platform_appearance::apply_dark_mode_to_window(hwnd);
            RedrawWindow(
                hwnd,
                null(),
                null_mut(),
                RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN,
            );
            Some(0)
        }
        _ => None,
    }
}

unsafe fn make_font(hwnd: HWND) -> HFONT {
    CreateFontW(
        -crate::platform::dpi::scale_for_window(hwnd, 15),
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
        to_wide("Microsoft YaHei UI").as_ptr(),
    )
}

#[allow(clippy::too_many_arguments)]
unsafe fn child(
    hwnd: HWND,
    font: HFONT,
    class: &str,
    caption: &str,
    id: usize,
    style: u32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> HWND {
    let scale = |v| crate::platform::dpi::scale_for_window(hwnd, v);
    let ex = if class == "EDIT" || (class == "LISTBOX" && style & LBS_OWNERDRAWFIXED as u32 == 0) {
        WS_EX_CLIENTEDGE
    } else {
        0
    };
    let result = CreateWindowExW(
        ex,
        to_wide(class).as_ptr(),
        to_wide(caption).as_ptr(),
        WS_CHILD | WS_VISIBLE | style,
        scale(x),
        scale(y),
        scale(width),
        scale(height),
        hwnd,
        id as _,
        platform_window::module_handle(),
        null(),
    );
    SendMessageW(result, WM_SETFONT, font as usize, 1);
    result
}

unsafe fn button(hwnd: HWND, font: HFONT, caption: &str, id: usize, x: i32, y: i32, width: i32) {
    child(
        hwnd,
        font,
        "BUTTON",
        caption,
        id,
        WS_TABSTOP | BS_PUSHBUTTON as u32,
        x,
        y,
        width,
        34,
    );
}

unsafe fn register(class: &str, proc: WNDPROC) {
    let class_w = to_wide(class);
    let wc = WNDCLASSW {
        lpfnWndProc: proc,
        hInstance: platform_window::module_handle(),
        hCursor: platform_window::arrow_cursor(),
        hbrBackground: null_mut(),
        lpszClassName: class_w.as_ptr(),
        ..zeroed()
    };
    RegisterClassW(&wc);
}

unsafe fn window_raw(
    owner: HWND,
    class: &str,
    title: &str,
    width: i32,
    height: i32,
    param: *const core::ffi::c_void,
) -> HWND {
    let dpi = crate::platform::dpi::layout_dpi_for_window(owner).max(96);
    let s = |n: i32| ((n as i64 * dpi as i64 + 48) / 96) as i32;
    let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU;
    let mut r = RECT {
        left: 0,
        top: 0,
        right: s(width),
        bottom: s(height),
    };
    windows_sys::Win32::UI::HiDpi::AdjustWindowRectExForDpi(
        &mut r,
        style,
        0,
        WS_EX_DLGMODALFRAME,
        dpi,
    );
    let mut anchor: RECT = zeroed();
    GetWindowRect(owner, &mut anchor);
    let w = r.right - r.left;
    let h = r.bottom - r.top;
    let work = platform_monitor::nearest_work_rect_for_window(owner);
    let x = (anchor.left + ((anchor.right - anchor.left - w) / 2).max(0))
        .clamp(work.left, (work.right - w).max(work.left));
    let y = (anchor.top + ((anchor.bottom - anchor.top - h) / 2).max(0))
        .clamp(work.top, (work.bottom - h).max(work.top));
    let hwnd = CreateWindowExW(
        WS_EX_DLGMODALFRAME,
        to_wide(class).as_ptr(),
        to_wide(title).as_ptr(),
        style,
        x,
        y,
        w,
        h,
        owner,
        null_mut(),
        platform_window::module_handle(),
        param,
    );
    if !hwnd.is_null() {
        initialize_window_identity(hwnd, owner);
    }
    hwnd
}

unsafe fn window<T>(
    owner: HWND,
    class: &str,
    title: &str,
    width: i32,
    height: i32,
    data: &mut T,
) -> HWND {
    window_raw(owner, class, title, width, height, data as *mut T as _)
}

unsafe fn logical_owner(hwnd: HWND) -> HWND {
    let logical = GetPropW(hwnd, to_wide(LOGICAL_OWNER_PROPERTY).as_ptr());
    if !logical.is_null() {
        let expected = GetPropW(hwnd, to_wide(OWNER_GENERATION_PROPERTY).as_ptr());
        if IsWindow(logical) == 0
            || (!expected.is_null()
                && GetPropW(logical, to_wide(GENERATION_PROPERTY).as_ptr()) != expected)
        {
            null_mut()
        } else {
            logical
        }
    } else {
        GetWindow(hwnd, GW_OWNER)
    }
}

unsafe fn initialize_window_identity(hwnd: HWND, owner: HWND) {
    SetPropW(
        hwnd,
        to_wide(GENERATION_PROPERTY).as_ptr(),
        NEXT_WINDOW_GENERATION.fetch_add(1, Ordering::Relaxed) as _,
    );
    SetPropW(hwnd, to_wide(LOGICAL_OWNER_PROPERTY).as_ptr(), owner);
    let generation = GetPropW(owner, to_wide(GENERATION_PROPERTY).as_ptr());
    if !generation.is_null() {
        SetPropW(
            hwnd,
            to_wide(OWNER_GENERATION_PROPERTY).as_ptr(),
            generation,
        );
    }
}

unsafe fn abort_dialog_tree(root: HWND) {
    unsafe extern "system" fn collect(hwnd: HWND, param: LPARAM) -> i32 {
        let (root, windows) = &mut *(param as *mut (HWND, Vec<HWND>));
        if hwnd != *root && belongs_to(hwnd, *root) {
            windows.push(hwnd);
        }
        1
    }
    let mut found = (root, Vec::<HWND>::new());
    EnumThreadWindows(
        GetCurrentThreadId(),
        Some(collect),
        &mut found as *mut _ as isize,
    );
    for hwnd in found.1 {
        if IsWindow(hwnd) != 0 {
            SetPropW(hwnd, to_wide(ABORTED_PROPERTY).as_ptr(), 1usize as _);
            DestroyWindow(hwnd);
        }
    }
}

/// Includes owned dialogs (including MessageBox) without allowing unrelated app windows.
unsafe fn belongs_to(mut candidate: HWND, root: HWND) -> bool {
    for _ in 0..24 {
        if candidate.is_null() {
            return false;
        }
        if candidate == root || IsChild(root, candidate) != 0 {
            return true;
        }
        candidate = logical_owner(candidate);
    }
    false
}

unsafe fn close_when_leaving(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> bool {
    if msg == WM_ACTIVATE && (wparam & 0xffff) == WA_INACTIVE as usize {
        if !belongs_to(lparam as HWND, hwnd) {
            PostMessageW(hwnd, CHECK_LOCK, 0, 0);
        }
    }
    if msg == WM_ACTIVATEAPP && wparam == 0 {
        PostMessageW(hwnd, CHECK_LOCK, 0, 0);
    }
    if msg == CHECK_LOCK || (msg == WM_TIMER && wparam == LOCK_TIMER) {
        if !GetPropW(hwnd, to_wide(LOGICAL_OWNER_PROPERTY).as_ptr()).is_null()
            && logical_owner(hwnd).is_null()
        {
            SetPropW(hwnd, to_wide(ABORTED_PROPERTY).as_ptr(), 1usize as _);
            DestroyWindow(hwnd);
        } else {
            lock_if_foreground_outside(hwnd, GetForegroundWindow(), GetFocus());
        }
        return true;
    }
    false
}

/// Keep the observed foreground explicit so ownership behavior can be tested
/// without depending on (or changing) the user's process-global foreground.
unsafe fn foreground_belongs_to_view(
    hwnd: HWND,
    foreground: HWND,
    focus: HWND,
    embedded: bool,
) -> bool {
    if embedded {
        (foreground == GetParent(hwnd) && belongs_to(focus, hwnd)) || belongs_to(foreground, hwnd)
    } else {
        belongs_to(foreground, hwnd)
    }
}

unsafe fn lock_if_foreground_outside(hwnd: HWND, foreground: HWND, focus: HWND) {
    let parent = GetParent(hwnd);
    let embedded = GetWindowLongW(hwnd, GWL_STYLE) as u32 & WS_CHILD != 0;
    if embedded && (IsWindowVisible(parent) == 0 || IsIconic(parent) != 0) {
        DestroyWindow(hwnd);
        return;
    }
    if foreground_belongs_to_view(hwnd, foreground, focus, embedded) {
        return;
    }
    // An editor/password child leaving the application locks its entire vault.
    let mut root = hwnd;
    let mut candidate = logical_owner(hwnd);
    while !candidate.is_null() {
        let mut name = [0u16; 80];
        let count = GetClassNameW(candidate, name.as_mut_ptr(), name.len() as i32).max(0) as usize;
        if String::from_utf16_lossy(&name[..count]) == VAULT_CLASS {
            root = candidate;
            break;
        }
        candidate = logical_owner(candidate);
    }
    DestroyWindow(root);
}

unsafe fn finish(hwnd: HWND) {
    let owner = GetPropW(hwnd, to_wide(OWNER_ENABLED_PROPERTY).as_ptr());
    if !owner.is_null() && IsWindow(owner) != 0 {
        EnableWindow(owner, 1);
        SetForegroundWindow(owner);
    }
    DestroyWindow(hwnd);
}

unsafe fn modal_loop(hwnd: HWND, owner: HWND, accept: usize, multiline: bool) {
    if hwnd.is_null() {
        return;
    }
    let owner = if GetWindowLongW(owner, GWL_STYLE) as u32 & WS_CHILD != 0 {
        GetAncestor(owner, GA_ROOT)
    } else {
        owner
    };
    let owner_enabled = IsWindowEnabled(owner) != 0;
    if owner_enabled {
        SetPropW(hwnd, to_wide(OWNER_ENABLED_PROPERTY).as_ptr(), owner);
    }
    if owner_enabled {
        EnableWindow(owner, 0);
    }
    ShowWindow(hwnd, SW_SHOW);
    SetForegroundWindow(hwnd);
    let preferred = GetPropW(hwnd, to_wide(DEFAULT_FOCUS_PROPERTY).as_ptr());
    if !preferred.is_null() {
        SetFocus(preferred);
    }
    SetTimer(hwnd, LOCK_TIMER, 250, None);
    let mut message: MSG = zeroed();
    while IsWindow(hwnd) != 0 {
        let result = GetMessageW(&mut message, null_mut(), 0, 0);
        if result <= 0 {
            if result == 0 {
                PostQuitMessage(message.wParam as i32);
            }
            if IsWindow(hwnd) != 0 {
                DestroyWindow(hwnd);
            }
            break;
        }
        if belongs_to(message.hwnd, hwnd) && message.message == WM_KEYDOWN {
            if message.wParam == VK_ESCAPE as usize {
                SendMessageW(hwnd, WM_CLOSE, 0, 0);
                continue;
            }
            if message.wParam == VK_RETURN as usize {
                if activate_focused_button(message.hwnd) {
                    continue;
                }
                let edit_multiline = multiline
                    && message.hwnd == control(hwnd, ID_SECRET)
                    && GetKeyState(VK_CONTROL as i32) >= 0;
                if !edit_multiline {
                    SendMessageW(hwnd, WM_COMMAND, accept, 0);
                    continue;
                }
                TranslateMessage(&message);
                DispatchMessageW(&message);
                continue;
            }
        }
        if IsDialogMessageW(hwnd, &message) == 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    if owner_enabled && IsWindow(owner) != 0 {
        EnableWindow(owner, 1);
    }
}

unsafe fn activate_focused_button(focus: HWND) -> bool {
    let mut class = [0u16; 32];
    let count = GetClassNameW(focus, class.as_mut_ptr(), class.len() as i32).max(0) as usize;
    if String::from_utf16_lossy(&class[..count]).eq_ignore_ascii_case("button") {
        SendMessageW(focus, BM_CLICK, 0, 0);
        true
    } else {
        false
    }
}

unsafe fn read_sensitive(hwnd: HWND) -> Zeroizing<String> {
    let length = GetWindowTextLengthW(hwnd).max(0) as usize;
    let mut wide = Zeroizing::new(vec![0u16; length + 1]);
    let count = GetWindowTextW(hwnd, wide.as_mut_ptr(), wide.len() as i32).max(0) as usize;
    Zeroizing::new(String::from_utf16_lossy(&wide[..count]))
}

unsafe fn set_sensitive(hwnd: HWND, value: &str) {
    let mut wide = Zeroizing::new(value.encode_utf16().chain(Some(0)).collect::<Vec<_>>());
    SetWindowTextW(hwnd, wide.as_ptr());
    wide.zeroize();
    SendMessageW(hwnd, EM_EMPTYUNDOBUFFER_, 0, 0);
}

unsafe fn wipe_edit(hwnd: HWND) {
    if hwnd.is_null() {
        return;
    }
    let length = GetWindowTextLengthW(hwnd).max(0) as usize;
    let mut blank = Zeroizing::new(vec![b' ' as u16; length + 1]);
    blank[length] = 0;
    SetWindowTextW(hwnd, blank.as_ptr());
    SetWindowTextW(hwnd, [0u16].as_ptr());
    SendMessageW(hwnd, EM_EMPTYUNDOBUFFER_, 0, 0);
}

unsafe fn password_prompt(owner: HWND, setting: bool) -> Option<Zeroizing<String>> {
    register(PASSWORD_CLASS, Some(password_proc));
    let mut data = PasswordWindow {
        setting,
        result: None,
        font: null_mut(),
    };
    let hwnd = window(
        owner,
        PASSWORD_CLASS,
        if setting {
            "设置管理密码"
        } else {
            "解锁密码与密钥"
        },
        480,
        if setting { 278 } else { 182 },
        &mut data,
    );
    if hwnd.is_null() {
        error(owner, "无法打开管理密码窗口。");
        return None;
    }
    modal_loop(hwnd, owner, ID_OK, false);
    data.result.take()
}

unsafe fn message_dialog(owner: HWND, message: &str, confirm: bool) -> bool {
    register(MESSAGE_CLASS, Some(message_proc));
    let mut data = MessageWindow {
        message: message.into(),
        confirm,
        result: false,
        font: null_mut(),
    };
    let hwnd = window(owner, MESSAGE_CLASS, "密码与密钥", 480, 220, &mut data);
    if hwnd.is_null() {
        return false;
    }
    modal_loop(hwnd, owner, ID_OK, false);
    data.result
}

unsafe extern "system" fn message_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if let Some(result) = theme_message(hwnd, msg, wparam) {
        return result;
    }
    if msg == WM_NCCREATE {
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (*(lparam as *const CREATESTRUCTW)).lpCreateParams as isize,
        );
    }
    if close_when_leaving(hwnd, msg, wparam, lparam) {
        return 0;
    }
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut MessageWindow;
    if ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    match msg {
        WM_CREATE => {
            (*ptr).font = make_font(hwnd);
            let font = (*ptr).font;
            child(
                hwnd,
                font,
                "STATIC",
                &(*ptr).message,
                0,
                0,
                22,
                22,
                436,
                140,
            );
            if (*ptr).confirm {
                button(hwnd, font, "取消", ID_CLOSE, 264, 172, 90);
                SetPropW(
                    hwnd,
                    to_wide(DEFAULT_FOCUS_PROPERTY).as_ptr(),
                    control(hwnd, ID_CLOSE),
                );
            }
            button(hwnd, font, "确定", ID_OK, 364, 172, 90);
            0
        }
        WM_COMMAND => {
            match wparam & 0xffff {
                ID_OK => {
                    (*ptr).result = true;
                    finish(hwnd);
                }
                ID_CLOSE => {
                    finish(hwnd);
                }
                _ => {}
            }
            0
        }
        WM_CLOSE => {
            finish(hwnd);
            0
        }
        WM_DESTROY => {
            if !GetPropW(hwnd, to_wide(ABORTED_PROPERTY).as_ptr()).is_null() {
                (*ptr).result = false;
            }
            abort_dialog_tree(hwnd);
            (*ptr).message.zeroize();
            DeleteObject((*ptr).font);
            (*ptr).font = null_mut();
            0
        }
        WM_NCDESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe extern "system" fn password_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if let Some(result) = theme_message(hwnd, msg, wparam) {
        return result;
    }
    if msg == WM_NCCREATE {
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (*(lparam as *const CREATESTRUCTW)).lpCreateParams as isize,
        );
    }
    if close_when_leaving(hwnd, msg, wparam, lparam) {
        return 0;
    }
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut PasswordWindow;
    if ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    let data = &mut *ptr;
    match msg {
        WM_CREATE => {
            data.font = make_font(hwnd);
            child(
                hwnd,
                data.font,
                "STATIC",
                if data.setting {
                    "新管理密码"
                } else {
                    "请输入管理密码"
                },
                0,
                0,
                22,
                20,
                432,
                24,
            );
            let first = child(
                hwnd,
                data.font,
                "EDIT",
                "",
                ID_SECRET,
                WS_TABSTOP | ES_PASSWORD as u32 | ES_AUTOHSCROLL as u32,
                22,
                50,
                432,
                32,
            );
            SendMessageW(first, EM_SETLIMITTEXT_, 4096, 0);
            if data.setting {
                child(hwnd, data.font, "STATIC", "再次输入", 0, 0, 22, 94, 432, 24);
                let second = child(
                    hwnd,
                    data.font,
                    "EDIT",
                    "",
                    ID_CONFIRM,
                    WS_TABSTOP | ES_PASSWORD as u32 | ES_AUTOHSCROLL as u32,
                    22,
                    124,
                    432,
                    32,
                );
                SendMessageW(second, EM_SETLIMITTEXT_, 4096, 0);
                child(
                    hwnd,
                    data.font,
                    "STATIC",
                    "忘记管理密码将无法恢复已保存的密码与密钥。",
                    0,
                    0,
                    22,
                    173,
                    432,
                    28,
                );
            } else {
                child(
                    hwnd,
                    data.font,
                    "STATIC",
                    "离开此分组后自动锁定。",
                    0,
                    0,
                    22,
                    91,
                    432,
                    26,
                );
            }
            let y = if data.setting { 220 } else { 128 };
            button(hwnd, data.font, "取消", ID_CLOSE, 264, y, 90);
            button(
                hwnd,
                data.font,
                if data.setting { "保存" } else { "解锁" },
                ID_OK,
                364,
                y,
                90,
            );
            SetFocus(first);
            0
        }
        WM_COMMAND => {
            match wparam & 0xffff {
                ID_OK => {
                    let value = read_sensitive(control(hwnd, ID_SECRET));
                    if value.is_empty() {
                        error(hwnd, "管理密码不能为空。");
                        return 0;
                    }
                    if value.len() > 1024 {
                        error(hwnd, "管理密码不能超过 1024 字节。");
                        return 0;
                    }
                    if data.setting
                        && value.as_str() != read_sensitive(control(hwnd, ID_CONFIRM)).as_str()
                    {
                        error(hwnd, "两次输入的管理密码不一致。");
                        return 0;
                    }
                    data.result = Some(value);
                    finish(hwnd);
                }
                ID_CLOSE => {
                    finish(hwnd);
                }
                _ => {}
            }
            0
        }
        WM_CLOSE => {
            finish(hwnd);
            0
        }
        WM_DESTROY => {
            if !GetPropW(hwnd, to_wide(ABORTED_PROPERTY).as_ptr()).is_null() {
                data.result = None;
            }
            abort_dialog_tree(hwnd);
            wipe_edit(control(hwnd, ID_SECRET));
            wipe_edit(control(hwnd, ID_CONFIRM));
            DeleteObject(data.font);
            data.font = null_mut();
            0
        }
        WM_NCDESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn entry_prompt(
    owner: HWND,
    label: String,
    notes: String,
    value: Zeroizing<String>,
) -> Option<EntryInput> {
    register(ENTRY_CLASS, Some(entry_proc));
    let mut data = EntryWindow {
        initial: value,
        label,
        notes,
        revealed: false,
        result: None,
        font: null_mut(),
    };
    let hwnd = window(owner, ENTRY_CLASS, "密码与密钥条目", 620, 440, &mut data);
    if hwnd.is_null() {
        error(owner, "无法打开条目编辑窗口。");
        return None;
    }
    modal_loop(hwnd, owner, ID_OK, true);
    data.result.take()
}

unsafe extern "system" fn entry_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if let Some(result) = theme_message(hwnd, msg, wparam) {
        return result;
    }
    if msg == WM_NCCREATE {
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (*(lparam as *const CREATESTRUCTW)).lpCreateParams as isize,
        );
    }
    if close_when_leaving(hwnd, msg, wparam, lparam) {
        return 0;
    }
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut EntryWindow;
    if ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    let data = &mut *ptr;
    match msg {
        WM_CREATE => {
            data.font = make_font(hwnd);
            child(hwnd, data.font, "STATIC", "名称", 0, 0, 22, 20, 70, 25);
            let label = child(
                hwnd,
                data.font,
                "EDIT",
                &data.label,
                ID_LABEL,
                WS_TABSTOP | ES_AUTOHSCROLL as u32,
                98,
                18,
                498,
                32,
            );
            SendMessageW(label, EM_SETLIMITTEXT_, 256, 0);
            child(hwnd, data.font, "STATIC", "备注", 0, 0, 22, 66, 70, 25);
            let notes = child(
                hwnd,
                data.font,
                "EDIT",
                &data.notes,
                ID_NOTES,
                WS_TABSTOP | ES_AUTOHSCROLL as u32,
                98,
                64,
                498,
                32,
            );
            SendMessageW(notes, EM_SETLIMITTEXT_, 4096, 0);
            child(hwnd, data.font, "STATIC", "内容", 0, 0, 22, 116, 100, 25);
            button(hwnd, data.font, "编辑内容", ID_REVEAL, 486, 109, 110);
            let style =
                ES_MULTILINE as u32 | ES_AUTOVSCROLL as u32 | ES_WANTRETURN as u32 | WS_VSCROLL;
            let secret = child(
                hwnd,
                data.font,
                "EDIT",
                "",
                ID_SECRET,
                WS_TABSTOP | style,
                22,
                154,
                574,
                180,
            );
            SendMessageW(secret, EM_SETLIMITTEXT_, MAX_SECRET_CHARS, 0);
            ShowWindow(secret, SW_HIDE);
            child(
                hwnd,
                data.font,
                "STATIC",
                "内容已隐藏。点击“编辑内容”查看或修改。",
                ID_HIDDEN,
                0,
                22,
                159,
                574,
                80,
            );
            child(
                hwnd,
                data.font,
                "STATIC",
                "条目仅在此分组显示；粘贴不会新增到普通剪贴板记录。",
                0,
                0,
                22,
                348,
                574,
                26,
            );
            button(hwnd, data.font, "取消", ID_CLOSE, 396, 392, 90);
            button(hwnd, data.font, "保存", ID_OK, 506, 392, 90);
            SetFocus(label);
            0
        }
        WM_COMMAND => {
            match wparam & 0xffff {
                ID_REVEAL => {
                    let edit = control(hwnd, ID_SECRET);
                    if !data.revealed {
                        set_sensitive(edit, &data.initial);
                        ShowWindow(control(hwnd, ID_HIDDEN), SW_HIDE);
                        ShowWindow(edit, SW_SHOW);
                        text(control(hwnd, ID_REVEAL), "隐藏内容");
                        data.revealed = true;
                        SetFocus(edit);
                    } else {
                        data.initial = read_sensitive(edit);
                        wipe_edit(edit);
                        ShowWindow(edit, SW_HIDE);
                        ShowWindow(control(hwnd, ID_HIDDEN), SW_SHOW);
                        text(control(hwnd, ID_REVEAL), "编辑内容");
                        data.revealed = false;
                    }
                }
                ID_OK => {
                    let label = platform_window::text(control(hwnd, ID_LABEL))
                        .trim()
                        .to_owned();
                    if label.is_empty() {
                        error(hwnd, "请填写名称，例如“邮箱登录密码”。");
                        return 0;
                    }
                    let secret = if data.revealed {
                        read_sensitive(control(hwnd, ID_SECRET))
                    } else {
                        Zeroizing::new(data.initial.to_string())
                    };
                    if secret.is_empty() {
                        error(hwnd, "内容不能为空，请点击“编辑内容”填写。");
                        return 0;
                    }
                    let notes = platform_window::text(control(hwnd, ID_NOTES));
                    data.result = Some(EntryInput {
                        label,
                        notes,
                        secret,
                    });
                    finish(hwnd);
                }
                ID_CLOSE => {
                    finish(hwnd);
                }
                _ => {}
            }
            0
        }
        WM_CLOSE => {
            finish(hwnd);
            0
        }
        WM_DESTROY => {
            if !GetPropW(hwnd, to_wide(ABORTED_PROPERTY).as_ptr()).is_null() {
                data.result = None;
            }
            abort_dialog_tree(hwnd);
            wipe_edit(control(hwnd, ID_SECRET));
            data.initial.zeroize();
            wipe_edit(control(hwnd, ID_LABEL));
            wipe_edit(control(hwnd, ID_NOTES));
            data.label.zeroize();
            data.notes.zeroize();
            DeleteObject(data.font);
            data.font = null_mut();
            0
        }
        WM_NCDESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn live_state(hwnd: HWND, state: &VaultRef) -> bool {
    IsWindow(hwnd) != 0
        && GetWindowLongPtrW(hwnd, GWLP_USERDATA) == Rc::as_ptr(state) as isize
        && state.borrow().session.is_some()
}

unsafe fn refresh(hwnd: HWND, state: &VaultRef) {
    if !live_state(hwnd, state) {
        return;
    }
    let (rows, protected) = {
        let data = state.borrow();
        let session = data.session.as_ref().unwrap();
        (session.list_all(), session.password_required())
    };
    let lines: Vec<String> = rows
        .iter()
        .map(|row| {
            format!(
                "{}    │    {}",
                row.label.replace(['\r', '\n', '\t'], " "),
                row.notes.replace(['\r', '\n', '\t'], " ")
            )
        })
        .collect();
    let populated = !rows.is_empty();
    {
        let mut data = state.borrow_mut();
        clear_rows(&mut data.rows);
        data.rows = rows;
    }
    let list = control(hwnd, ID_LIST);
    SendMessageW(list, LB_RESETCONTENT, 0, 0);
    for line in lines {
        SendMessageW(list, LB_ADDSTRING, 0, to_wide(&line).as_ptr() as isize);
    }
    if populated {
        SendMessageW(list, LB_SETCURSEL, 0, 0);
    }
    for id in [ID_EDIT, ID_DELETE, ID_PASTE] {
        EnableWindow(control(hwnd, id), populated as i32);
    }
    text(
        control(hwnd, ID_MANAGE),
        if protected {
            "更改管理密码"
        } else {
            "设置管理密码"
        },
    );
    EnableWindow(control(hwnd, ID_REMOVE_PASSWORD), protected as i32);
}

unsafe fn selected(hwnd: HWND, state: &VaultRef) -> Option<EntrySummary> {
    let index = SendMessageW(control(hwnd, ID_LIST), LB_GETCURSEL, 0, 0);
    if index < 0 {
        None
    } else {
        state.borrow().rows.get(index as usize).cloned()
    }
}

unsafe fn edit_entry(
    hwnd: HWND,
    state: &VaultRef,
    row: Option<EntrySummary>,
    initial: Option<Zeroizing<String>>,
) {
    if !live_state(hwnd, state) {
        return;
    }
    let initial_result = {
        let data = state.borrow();
        match initial {
            Some(v) => Ok(v),
            None => match &row {
                Some(row) => data.session.as_ref().unwrap().secret(&row.id),
                None => Ok(Zeroizing::new(String::new())),
            },
        }
    };
    let mut initial = match initial_result {
        Ok(v) => v,
        Err(e) => {
            error(hwnd, &e);
            return;
        }
    };
    let mut label = row.as_ref().map(|r| r.label.clone()).unwrap_or_default();
    let mut notes = row.as_ref().map(|r| r.notes.clone()).unwrap_or_default();
    loop {
        let Some(mut input) = entry_prompt(hwnd, label, notes, initial) else {
            return;
        };
        if !live_state(hwnd, state) {
            return;
        }
        let saved = crate::db_runtime::try_with_exclusive_app_data_snapshot(|| {
            state
                .borrow_mut()
                .session
                .as_mut()
                .ok_or_else(|| "分组已锁定".to_string())?
                .upsert_entry(
                    row.as_ref().map(|r| r.id.as_str()),
                    &input.label,
                    &input.notes,
                    &input.secret,
                )
        });
        match saved {
            Ok(_) => {
                let parent = state.borrow().parent;
                super::protected_entry_saved(parent, &input.secret);
                if live_state(hwnd, state) {
                    refresh(hwnd, state);
                    text(control(hwnd, ID_STATUS), "密码与密钥已自动隐藏");
                    SetTimer(hwnd, STATUS_TIMER, 5000, None);
                }
                return;
            }
            Err(e) => error(hwnd, &e),
        }
        if !live_state(hwnd, state) {
            return;
        }
        label = std::mem::take(&mut input.label);
        notes = std::mem::take(&mut input.notes);
        initial = std::mem::take(&mut input.secret);
    }
}

unsafe fn embedded_controls(hwnd: HWND, font: HFONT) {
    child(
        hwnd,
        font,
        "STATIC",
        "密码与密钥",
        ID_HEADING,
        0,
        12,
        10,
        170,
        26,
    );
    button(hwnd, font, "返回", ID_CLOSE, 212, 6, 74);
    child(
        hwnd,
        font,
        "STATIC",
        "点击条目粘贴，右键编辑或删除。",
        ID_STATUS,
        0,
        12,
        47,
        276,
        42,
    );
    let list = child(
        hwnd,
        font,
        "LISTBOX",
        "",
        ID_LIST,
        WS_TABSTOP
            | WS_VSCROLL
            | LBS_NOTIFY as u32
            | LBS_NOINTEGRALHEIGHT as u32
            | LBS_OWNERDRAWFIXED as u32
            | LBS_HASSTRINGS as u32,
        12,
        90,
        276,
        310,
    );
    SetWindowSubclass(list, Some(embedded_list_proc), 1, 0);
    SendMessageW(
        list,
        LB_SETITEMHEIGHT,
        0,
        platform_dpi::scale_for_window(hwnd, 44) as isize,
    );
    button(hwnd, font, "新增", ID_ADD, 12, 414, 84);
    button(hwnd, font, "编辑", ID_EDIT, 104, 414, 84);
    button(hwnd, font, "删除", ID_DELETE, 196, 414, 84);
    button(hwnd, font, "设置管理密码", ID_MANAGE, 12, 458, 132);
    button(
        hwnd,
        font,
        "取消管理密码",
        ID_REMOVE_PASSWORD,
        152,
        458,
        132,
    );
    layout_embedded_controls(hwnd);
}

unsafe fn layout_embedded_controls(hwnd: HWND) {
    let s = |value| platform_dpi::scale_for_window(hwnd, value);
    let mut bounds: RECT = zeroed();
    GetClientRect(hwnd, &mut bounds);
    let margin = s(10);
    let gap = s(6);
    let width = (bounds.right - 2 * margin).max(s(240));
    let bottom = bounds.bottom - margin;
    let row_height = s(32);
    let top = s(86);
    let controls_y = (bottom - row_height * 2 - gap).max(top + s(90));
    let place = |id: usize, x, y, w, h| {
        SetWindowPos(
            control(hwnd, id),
            null_mut(),
            x,
            y,
            w,
            h,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
    };
    place(ID_HEADING, margin, s(10), width - s(80), s(28));
    place(ID_CLOSE, margin + width - s(68), s(6), s(68), row_height);
    place(ID_STATUS, margin, s(46), width, s(34));
    place(
        ID_LIST,
        margin,
        top,
        width,
        (controls_y - top - gap).max(s(60)),
    );
    let third = (width - gap * 2) / 3;
    for (index, id) in [ID_ADD, ID_EDIT, ID_DELETE].into_iter().enumerate() {
        place(
            id,
            margin + (third + gap) * index as i32,
            controls_y,
            third,
            row_height,
        );
    }
    let half = (width - gap) / 2;
    place(
        ID_MANAGE,
        margin,
        controls_y + row_height + gap,
        half,
        row_height,
    );
    place(
        ID_REMOVE_PASSWORD,
        margin + half + gap,
        controls_y + row_height + gap,
        half,
        row_height,
    );
    SendMessageW(control(hwnd, ID_LIST), LB_SETITEMHEIGHT, 0, s(44) as isize);
    clip_summary_list_corners(control(hwnd, ID_LIST));
}

unsafe fn clip_summary_list_corners(list: HWND) {
    let Some(bounds) = platform_window::window_rect(list) else {
        return;
    };
    let radius = platform_dpi::scale_for_window(list, 10);
    let region = windows_sys::Win32::Graphics::Gdi::CreateRoundRectRgn(
        0,
        0,
        bounds.right - bounds.left + 1,
        bounds.bottom - bounds.top + 1,
        radius * 2,
        radius * 2,
    );
    if !region.is_null() && !platform_window::set_window_region(list, region, false) {
        DeleteObject(region as _);
    }
}

unsafe fn update_summary_hover(list: HWND, point: Option<LPARAM>) {
    let next = point
        .and_then(|point| list_item_at(list, point))
        .map(|index| index + 1)
        .unwrap_or(0);
    let previous = GetPropW(list, to_wide(ROW_HOVER_PROPERTY).as_ptr()) as usize;
    if next == previous {
        return;
    }
    if next == 0 {
        RemovePropW(list, to_wide(ROW_HOVER_PROPERTY).as_ptr());
    } else {
        SetPropW(list, to_wide(ROW_HOVER_PROPERTY).as_ptr(), next as _);
    }
    for value in [previous, next] {
        if value > 0 {
            let mut rect: RECT = zeroed();
            if SendMessageW(
                list,
                LB_GETITEMRECT,
                value - 1,
                &mut rect as *mut _ as isize,
            ) != LB_ERR as isize
            {
                platform_gdi::invalidate_rect(list, &rect, 0);
            }
        }
    }
}

unsafe fn draw_summary_row(
    hdc: HDC,
    rect: RECT,
    row: &EntrySummary,
    selected: bool,
    hovered: bool,
    dpi: u32,
    theme: Theme,
) {
    let s = |value: i32| ((value as i64 * dpi.max(96) as i64 + 48) / 96) as i32;
    let background = if selected {
        theme.item_selected
    } else if hovered {
        theme.item_hover
    } else {
        theme.surface
    };
    let brush = platform_gdi::create_solid_brush(background);
    platform_gdi::fill_rect(hdc, &rect, brush);
    DeleteObject(brush as _);
    let icon_rect = RECT {
        left: rect.left + s(10),
        top: rect.top + s(12),
        right: rect.left + s(30),
        bottom: rect.top + s(32),
    };
    let icon = icon_handle_for(IconAssetKind::Text, s(20));
    if icon != 0 {
        draw_icon_tinted_soft(
            hdc as _,
            icon_rect.left,
            icon_rect.top,
            icon,
            s(20),
            s(20),
            ((theme.surface & 255) + ((theme.surface >> 8) & 255) + ((theme.surface >> 16) & 255))
                < 384,
            0,
        );
    }
    let has_notes = !row.notes.trim().is_empty();
    let title = RECT {
        left: rect.left + s(40),
        top: rect.top + if has_notes { s(3) } else { s(7) },
        right: rect.right - s(12),
        bottom: rect.top + if has_notes { s(24) } else { s(37) },
    };
    draw_text_ex_px(
        hdc as _,
        &row.label.replace(['\r', '\n', '\t'], " "),
        &title,
        theme.text,
        s(14),
        false,
        false,
        ui_text_font_family(),
    );
    if has_notes {
        let notes = RECT {
            left: title.left,
            top: rect.top + s(23),
            right: title.right,
            bottom: rect.top + s(41),
        };
        draw_text_ex_px(
            hdc as _,
            &row.notes.replace(['\r', '\n', '\t'], " "),
            &notes,
            theme.text_muted,
            s(11),
            false,
            false,
            ui_text_font_family(),
        );
    }
}

unsafe fn draw_summary_list_item(hwnd: HWND, state: &VaultRef, lparam: LPARAM) -> LRESULT {
    let draw = &*(lparam as *const DRAWITEMSTRUCT);
    if draw.CtlID != ID_LIST as u32 {
        return 0;
    }
    let row = state.borrow().rows.get(draw.itemID as usize).cloned();
    if let Some(row) = row {
        let hovered = GetPropW(draw.hwndItem, to_wide(ROW_HOVER_PROPERTY).as_ptr()) as usize
            == draw.itemID as usize + 1;
        draw_summary_row(
            draw.hDC,
            draw.rcItem,
            &row,
            draw.itemState & ODS_SELECTED != 0,
            hovered,
            platform_dpi::layout_dpi_for_window(hwnd),
            Theme::default(),
        );
    }
    1
}

unsafe fn paint_vault_chrome(hwnd: HWND) {
    let mut ps: PAINTSTRUCT = zeroed();
    let dc = platform_gdi::begin_paint(hwnd, &mut ps);
    let theme = Theme::default();
    let bounds = platform_window::client_rect(hwnd).unwrap_or_else(|| zeroed());
    let brush = platform_gdi::create_solid_brush(theme.surface);
    platform_gdi::fill_rect(dc, &bounds, brush);
    DeleteObject(brush as _);
    if let Some(mut card) = platform_window::window_rect(control(hwnd, ID_LIST)) {
        let mut p = POINT {
            x: card.left,
            y: card.top,
        };
        platform_window::screen_to_client(hwnd, &mut p);
        let width = card.right - card.left;
        let height = card.bottom - card.top;
        card = RECT {
            left: p.x - 1,
            top: p.y - 1,
            right: p.x + width + 1,
            bottom: p.y + height + 1,
        };
        draw_round_rect(
            dc as _,
            &card,
            theme.surface,
            theme.stroke,
            platform_dpi::scale_for_window(hwnd, 10),
        );
    }
    platform_gdi::end_paint(hwnd, &ps);
}

unsafe extern "system" fn embedded_list_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    if msg == WM_MOUSEMOVE {
        platform_input::track_mouse_leave(hwnd);
        update_summary_hover(hwnd, Some(lparam));
    }
    if msg == WM_MOUSELEAVE {
        update_summary_hover(hwnd, None);
    }
    const ROW: &str = "ZSClipVaultPressedRow";
    const POINT_PROP: &str = "ZSClipVaultPressedPoint";
    const BUTTON: &str = "ZSClipVaultPressedButton";
    const DRAG: &str = "ZSClipVaultDragged";
    let clear = || {
        for key in [ROW, POINT_PROP, BUTTON, DRAG] {
            RemovePropW(hwnd, to_wide(key).as_ptr());
        }
    };
    if msg == WM_LBUTTONDOWN || msg == WM_RBUTTONDOWN {
        clear();
        if let Some(index) = list_item_at(hwnd, lparam) {
            SetPropW(hwnd, to_wide(ROW).as_ptr(), (index + 1) as _);
            SetPropW(
                hwnd,
                to_wide(POINT_PROP).as_ptr(),
                (lparam as usize).wrapping_add(1) as _,
            );
            SetPropW(
                hwnd,
                to_wide(BUTTON).as_ptr(),
                if msg == WM_LBUTTONDOWN {
                    1usize as _
                } else {
                    2usize as _
                },
            );
        }
    }
    if msg == WM_MOUSEMOVE && !GetPropW(hwnd, to_wide(ROW).as_ptr()).is_null() {
        let start =
            (GetPropW(hwnd, to_wide(POINT_PROP).as_ptr()) as usize).wrapping_sub(1) as isize;
        let dx = (get_x_lparam(lparam) - get_x_lparam(start)).abs();
        let dy = (get_y_lparam(lparam) - get_y_lparam(start)).abs();
        if dx >= GetSystemMetrics(SM_CXDRAG).max(2) || dy >= GetSystemMetrics(SM_CYDRAG).max(2) {
            SetPropW(hwnd, to_wide(DRAG).as_ptr(), 1usize as _);
        }
    }
    if matches!(
        msg,
        WM_MOUSEWHEEL | WM_CAPTURECHANGED | WM_CANCELMODE | WM_NCDESTROY
    ) {
        clear();
    }
    if msg == WM_LBUTTONUP {
        let clicked = completed_row_click(hwnd, lparam, 1);
        let modifiers = GetKeyState(VK_CONTROL as i32) < 0 || GetKeyState(VK_SHIFT as i32) < 0;
        clear();
        let result = DefSubclassProc(hwnd, msg, wparam, lparam);
        if clicked.is_some() && !modifiers {
            PostMessageW(GetParent(hwnd), WM_COMMAND, ID_PASTE, 0);
        }
        return result;
    }
    if msg == WM_RBUTTONUP {
        let clicked = completed_row_click(hwnd, lparam, 2);
        clear();
        let Some(index) = clicked else {
            return 0;
        };
        SendMessageW(hwnd, LB_SETCURSEL, index, 0);
        let menu = CreatePopupMenu();
        if !menu.is_null() {
            for (id, label) in [(ID_PASTE, "粘贴"), (ID_EDIT, "编辑"), (ID_DELETE, "删除")] {
                AppendMenuW(menu, MF_STRING, id, to_wide(label).as_ptr());
            }
            let mut point: POINT = zeroed();
            GetCursorPos(&mut point);
            let command = TrackPopupMenu(
                menu,
                TPM_RETURNCMD | TPM_RIGHTBUTTON,
                point.x,
                point.y,
                0,
                GetAncestor(hwnd, GA_ROOT),
                null(),
            );
            DestroyMenu(menu);
            if command != 0 {
                PostMessageW(GetParent(hwnd), WM_COMMAND, command as usize, 0);
            }
        }
        return 0;
    }
    DefSubclassProc(hwnd, msg, wparam, lparam)
}

unsafe fn list_item_at(hwnd: HWND, point: LPARAM) -> Option<usize> {
    let hit = SendMessageW(hwnd, LB_ITEMFROMPOINT, 0, point);
    if (hit as usize >> 16) & 0xffff != 0 {
        return None;
    }
    let index = hit as usize & 0xffff;
    let mut bounds: RECT = zeroed();
    if SendMessageW(hwnd, LB_GETITEMRECT, index, &mut bounds as *mut _ as isize) == LB_ERR as isize
    {
        return None;
    }
    let (x, y) = (get_x_lparam(point), get_y_lparam(point));
    (x >= bounds.left && x < bounds.right && y >= bounds.top && y < bounds.bottom).then_some(index)
}

unsafe fn completed_row_click(hwnd: HWND, point: LPARAM, button: usize) -> Option<usize> {
    let index = list_item_at(hwnd, point)?;
    let row = GetPropW(hwnd, to_wide("ZSClipVaultPressedRow").as_ptr()) as usize;
    let pressed_button = GetPropW(hwnd, to_wide("ZSClipVaultPressedButton").as_ptr()) as usize;
    let dragged = GetPropW(hwnd, to_wide("ZSClipVaultDragged").as_ptr());
    (row == index + 1 && pressed_button == button && dragged.is_null()).then_some(index)
}

unsafe extern "system" fn standalone_list_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    if msg == WM_MOUSEMOVE {
        platform_input::track_mouse_leave(hwnd);
        update_summary_hover(hwnd, Some(lparam));
    }
    if msg == WM_MOUSELEAVE {
        update_summary_hover(hwnd, None);
    }
    if msg == WM_LBUTTONDBLCLK {
        if let Some(index) = list_item_at(hwnd, lparam) {
            SendMessageW(hwnd, LB_SETCURSEL, index, 0);
            PostMessageW(GetParent(hwnd), WM_COMMAND, ID_PASTE, 0);
        }
        return 0;
    }
    DefSubclassProc(hwnd, msg, wparam, lparam)
}

unsafe fn paste_entry(hwnd: HWND, state: &VaultRef) {
    if !live_state(hwnd, state) {
        return;
    }
    let (target, parent, embedded) = {
        let data = state.borrow();
        (data.target, data.parent, data.embedded)
    };
    let Some(target) = target else {
        error(
            hwnd,
            "没有可用的粘贴目标。请先点击目标输入框，再打开密码与密钥分组。",
        );
        return;
    };
    let Some(row) = selected(hwnd, state) else {
        return;
    };
    let result = state.borrow().session.as_ref().unwrap().secret(&row.id);
    let value = match result {
        Ok(value) => value,
        Err(e) => {
            error(hwnd, &e);
            return;
        }
    };
    if !embedded {
        state.borrow_mut().paste = Some(value);
        finish(hwnd);
        return;
    }
    state.borrow_mut().session.take();
    DestroyWindow(hwnd);
    let ptr = get_state_ptr(parent);
    if !ptr.is_null()
        && !paste_protected_text_to_target(parent, &mut *ptr, &value, target.0, target.1)
    {
        error(
            parent,
            "目标输入框已不可用，未粘贴密码与密钥。请重新选择输入框后重试。",
        );
    }
}

unsafe fn retain_vault_state(ptr: *const RefCell<VaultWindow>) -> VaultRef {
    Rc::increment_strong_count(ptr);
    Rc::from_raw(ptr)
}

unsafe fn create_vault_window(owner: HWND, state: &VaultRef) -> HWND {
    register(VAULT_CLASS, Some(vault_proc));
    let references_before = Rc::strong_count(state);
    let raw = Rc::into_raw(Rc::clone(state));
    let embedded = state.borrow().embedded;
    let hwnd = if embedded {
        let mut bounds: RECT = zeroed();
        GetClientRect(owner, &mut bounds);
        let top = main_layout_for_window(owner).title_h;
        CreateWindowExW(
            WS_EX_CONTROLPARENT,
            to_wide(VAULT_CLASS).as_ptr(),
            to_wide("密码与密钥").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_CLIPCHILDREN,
            0,
            top,
            bounds.right,
            (bounds.bottom - top).max(1),
            owner,
            null_mut(),
            platform_window::module_handle(),
            raw as _,
        )
    } else {
        window_raw(owner, VAULT_CLASS, "密码与密钥", 730, 530, raw as _)
    };
    if hwnd.is_null() && Rc::strong_count(state) > references_before {
        drop(Rc::from_raw(raw));
    }
    hwnd
}

unsafe extern "system" fn vault_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if let Some(result) = theme_message(hwnd, msg, wparam) {
        return result;
    }
    if msg == WM_NCCREATE {
        let create = &*(lparam as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        initialize_window_identity(hwnd, create.hwndParent);
    }
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<VaultWindow>;
    if ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    let state = retain_vault_state(ptr);
    if close_when_leaving(hwnd, msg, wparam, lparam) {
        return 0;
    }
    match msg {
        WM_CREATE => {
            let font = make_font(hwnd);
            state.borrow_mut().font = font;
            let embedded = state.borrow().embedded;
            if embedded {
                embedded_controls(hwnd, font);
                refresh(hwnd, &state);
                return 0;
            }
            child(
                hwnd,
                font,
                "STATIC",
                "密码与密钥",
                ID_STATUS,
                0,
                22,
                26,
                368,
                28,
            );
            button(hwnd, font, "设置管理密码", ID_MANAGE, 402, 20, 146);
            button(hwnd, font, "取消管理密码", ID_REMOVE_PASSWORD, 560, 20, 146);
            child(
                hwnd,
                font,
                "STATIC",
                "名称    │    备注",
                0,
                0,
                24,
                69,
                680,
                24,
            );
            let list = child(
                hwnd,
                font,
                "LISTBOX",
                "",
                ID_LIST,
                WS_TABSTOP
                    | WS_VSCROLL
                    | LBS_NOTIFY as u32
                    | LBS_NOINTEGRALHEIGHT as u32
                    | LBS_OWNERDRAWFIXED as u32
                    | LBS_HASSTRINGS as u32,
                22,
                96,
                684,
                320,
            );
            SetWindowSubclass(list, Some(standalone_list_proc), 1, 0);
            SendMessageW(
                list,
                LB_SETITEMHEIGHT,
                0,
                platform_dpi::scale_for_window(hwnd, 44) as isize,
            );
            clip_summary_list_corners(list);
            child(
                hwnd,
                font,
                "STATIC",
                "双击条目粘贴；离开此窗口自动锁定。内容不会进入普通记录和同步。",
                0,
                0,
                22,
                430,
                684,
                24,
            );
            for (caption, id, x, width) in [
                ("新增", ID_ADD, 22, 90),
                ("编辑", ID_EDIT, 124, 90),
                ("删除", ID_DELETE, 226, 90),
                ("粘贴", ID_PASTE, 510, 90),
                ("关闭", ID_CLOSE, 612, 94),
            ] {
                button(hwnd, font, caption, id, x, 474, width);
            }
            refresh(hwnd, &state);
            SetFocus(list);
            0
        }
        WM_COMMAND => {
            if !live_state(hwnd, &state) {
                return 0;
            }
            let id = wparam & 0xffff;
            if id == ID_LIST && (wparam >> 16) & 0xffff == LBN_DBLCLK as usize {
                return 0;
            }
            match id {
                ID_ADD => edit_entry(hwnd, &state, None, None),
                ID_EDIT => {
                    if let Some(row) = selected(hwnd, &state) {
                        edit_entry(hwnd, &state, Some(row), None);
                    }
                }
                ID_DELETE => {
                    if let Some(row) = selected(hwnd, &state) {
                        let accepted =
                            message_dialog(hwnd, &format!("删除“{}”？", row.label), true);
                        if !live_state(hwnd, &state) {
                            return 0;
                        }
                        if accepted {
                            let result =
                                crate::db_runtime::try_with_exclusive_app_data_snapshot(|| {
                                    state
                                        .borrow_mut()
                                        .session
                                        .as_mut()
                                        .ok_or_else(|| "分组已锁定".to_string())?
                                        .remove(&row.id)
                                });
                            match result {
                                Ok(()) => {
                                    let parent = state.borrow().parent;
                                    super::protected_entry_saved(parent, "");
                                    refresh(hwnd, &state);
                                }
                                Err(e) => error(hwnd, &e),
                            }
                        }
                    }
                }
                ID_PASTE => paste_entry(hwnd, &state),
                ID_MANAGE => {
                    let password = password_prompt(hwnd, true);
                    if !live_state(hwnd, &state) {
                        return 0;
                    }
                    if let Some(password) = password {
                        let result =
                            crate::db_runtime::try_with_exclusive_app_data_snapshot(|| {
                                state
                                    .borrow_mut()
                                    .session
                                    .as_mut()
                                    .ok_or_else(|| "分组已锁定".to_string())?
                                    .set_password(Some(&password))
                            });
                        match result {
                            Ok(()) => refresh(hwnd, &state),
                            Err(e) => error(hwnd, &e),
                        }
                    }
                }
                ID_REMOVE_PASSWORD => {
                    let accepted =
                        message_dialog(hwnd, "取消管理密码后，打开此分组将无需输入密码。", true);
                    if !live_state(hwnd, &state) {
                        return 0;
                    }
                    if accepted {
                        let result =
                            crate::db_runtime::try_with_exclusive_app_data_snapshot(|| {
                                state
                                    .borrow_mut()
                                    .session
                                    .as_mut()
                                    .ok_or_else(|| "分组已锁定".to_string())?
                                    .set_password(None)
                            });
                        match result {
                            Ok(()) => refresh(hwnd, &state),
                            Err(e) => error(hwnd, &e),
                        }
                    }
                }
                ID_CLOSE => {
                    finish(hwnd);
                }
                _ => {}
            }
            0
        }
        WM_CLOSE => {
            finish(hwnd);
            0
        }
        WM_MEASUREITEM => {
            let measure = &mut *(lparam as *mut MEASUREITEMSTRUCT);
            if measure.CtlID == ID_LIST as u32 {
                measure.itemHeight = platform_dpi::scale_for_window(hwnd, 44) as u32;
                return 1;
            }
            0
        }
        WM_DRAWITEM => draw_summary_list_item(hwnd, &state, lparam),
        WM_PAINT => {
            paint_vault_chrome(hwnd);
            0
        }
        WM_TIMER if wparam == STATUS_TIMER => {
            KillTimer(hwnd, STATUS_TIMER);
            let embedded = state.borrow().embedded;
            text(
                control(hwnd, ID_STATUS),
                if embedded {
                    "点击条目粘贴，右键编辑或删除。"
                } else {
                    "密码与密钥"
                },
            );
            0
        }
        WM_SIZE => {
            let embedded = state.borrow().embedded;
            if embedded {
                layout_embedded_controls(hwnd);
            }
            0
        }
        WM_DESTROY => {
            let (parent, embedded, restore, font) = {
                let mut data = state.borrow_mut();
                data.session.take();
                clear_rows(&mut data.rows);
                let font = std::mem::replace(&mut data.font, null_mut());
                (data.parent, data.embedded, data.restore_noactivate, font)
            };
            abort_dialog_tree(hwnd);
            DeleteObject(font);
            if embedded && is_app_window(parent) && IsWindow(parent) != 0 {
                WindowsMainWindowHost::new(Some(wnd_proc))
                    .set_main_window_activation_policy(parent, !restore);
                layout_children(parent);
                repaint_main_window(parent, true);
            }
            0
        }
        WM_NCDESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            let result = DefWindowProcW(hwnd, msg, wparam, lparam);
            drop(Rc::from_raw(ptr));
            result
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn unlock_session(owner: HWND) -> Option<VaultSession> {
    let protected = match secret_vault::has_password() {
        Ok(value) => value,
        Err(e) => {
            error(owner, &e);
            return None;
        }
    };
    let session = loop {
        let password = if protected {
            match password_prompt(owner, false) {
                Some(value) => Some(value),
                None => return None,
            }
        } else {
            None
        };
        match crate::db_runtime::try_with_exclusive_app_data_snapshot(|| {
            secret_vault::unlock(password.as_ref().map(|v| v.as_str()))
        }) {
            Ok(session) => break session,
            Err(e) => {
                error(owner, &e);
                if !protected {
                    return None;
                }
            }
        }
    };
    Some(session)
}

pub(super) unsafe fn has_main_view(parent: HWND) -> bool {
    if parent.is_null() || IsWindow(parent) == 0 {
        return false;
    }
    !FindWindowExW(parent, null_mut(), to_wide(VAULT_CLASS).as_ptr(), null()).is_null()
}

pub(super) unsafe fn close_main_view(parent: HWND) {
    if parent.is_null() || IsWindow(parent) == 0 {
        return;
    }
    let panel = FindWindowExW(parent, null_mut(), to_wide(VAULT_CLASS).as_ptr(), null());
    if !panel.is_null() {
        DestroyWindow(panel);
    }
}

pub(super) unsafe fn resize_main_view(parent: HWND) {
    if parent.is_null() || IsWindow(parent) == 0 {
        return;
    }
    let panel = FindWindowExW(parent, null_mut(), to_wide(VAULT_CLASS).as_ptr(), null());
    if panel.is_null() {
        return;
    }
    let mut bounds: RECT = zeroed();
    GetClientRect(parent, &mut bounds);
    let top = main_layout_for_window(parent).title_h;
    SetWindowPos(
        panel,
        HWND_TOP,
        0,
        top,
        bounds.right,
        (bounds.bottom - top).max(1),
        SWP_NOACTIVATE,
    );
}

pub(super) unsafe fn route_main_view_message(message: &MSG) -> bool {
    let parent = GetAncestor(message.hwnd, GA_ROOT);
    if !is_app_window(parent) {
        return false;
    }
    let panel = FindWindowExW(parent, null_mut(), to_wide(VAULT_CLASS).as_ptr(), null());
    if panel.is_null() {
        return false;
    }
    if message.hwnd == parent && (message.message == WM_LBUTTONDOWN || message.message == WM_HOTKEY)
    {
        close_main_view(parent);
        return false;
    }
    if !belongs_to(message.hwnd, panel) {
        return false;
    }
    if message.message == WM_KEYDOWN {
        if message.wParam == VK_ESCAPE as usize {
            close_main_view(parent);
            return true;
        }
        if message.wParam == VK_RETURN as usize {
            if let Some(id) = focused_enter_command(panel, message.hwnd) {
                if id == ID_PASTE {
                    SendMessageW(panel, WM_COMMAND, ID_PASTE, 0);
                } else {
                    SendMessageW(message.hwnd, BM_CLICK, 0, 0);
                }
                return true;
            }
        }
    }
    if belongs_to(message.hwnd, panel) {
        return IsDialogMessageW(panel, message) != 0;
    }
    false
}

unsafe fn focused_enter_command(panel: HWND, focus: HWND) -> Option<usize> {
    if focus == control(panel, ID_LIST) {
        return Some(ID_PASTE);
    }
    if IsChild(panel, focus) == 0 {
        return None;
    }
    let id = GetDlgCtrlID(focus) as usize;
    matches!(
        id,
        ID_ADD | ID_EDIT | ID_DELETE | ID_MANAGE | ID_REMOVE_PASSWORD | ID_CLOSE
    )
    .then_some(id)
}

pub(super) unsafe fn open_main_view(owner: HWND, import_text: Option<String>) {
    let owner = if is_app_window(owner) {
        owner
    } else {
        main_window_hwnd()
    };
    let import = import_text.map(Zeroizing::new);
    if owner.is_null() || IsWindow(owner) == 0 {
        return;
    }
    close_main_view(owner);
    let ptr = get_state_ptr(owner);
    if ptr.is_null() {
        return;
    }
    let target = capture_protected_paste_target(owner, &*ptr);
    let restore_noactivate = (*ptr).main_window_noactivate;
    let Some(session) = unlock_session(owner) else {
        return;
    };
    let state = Rc::new(RefCell::new(VaultWindow {
        parent: owner,
        embedded: true,
        restore_noactivate,
        session: Some(session),
        rows: Vec::new(),
        target,
        paste: None,
        font: null_mut(),
    }));
    let panel = create_vault_window(owner, &state);
    if panel.is_null() {
        error(owner, "无法打开密码与密钥分组。");
        return;
    }
    WindowsMainWindowHost::new(Some(wnd_proc)).set_main_window_activation_policy(owner, true);
    ShowWindow(owner, SW_SHOW);
    SetForegroundWindow(owner);
    resize_main_view(owner);
    SetFocus(control(panel, ID_LIST));
    SetTimer(panel, LOCK_TIMER, 250, None);
    if let Some(value) = import {
        edit_entry(panel, &state, None, Some(value));
    }
}

pub(super) unsafe fn open(_owner: HWND, import_text: Option<String>) {
    let owner = main_window_hwnd();
    let import = import_text.map(Zeroizing::new);
    if owner.is_null() || IsWindow(owner) == 0 {
        return;
    }
    close_main_view(owner);
    let existing = current_process_vault();
    if !existing.is_null() {
        SetForegroundWindow(existing);
        return;
    }
    let ptr = get_state_ptr(owner);
    if ptr.is_null() {
        return;
    }
    let target = capture_protected_paste_target(owner, &*ptr);
    let Some(session) = unlock_session(owner) else {
        return;
    };
    let state = Rc::new(RefCell::new(VaultWindow {
        parent: owner,
        embedded: false,
        restore_noactivate: false,
        session: Some(session),
        rows: Vec::new(),
        target,
        paste: None,
        font: null_mut(),
    }));
    let hwnd = create_vault_window(owner, &state);
    if hwnd.is_null() {
        error(owner, "无法打开密码与密钥窗口。");
        return;
    }
    if let Some(value) = import {
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
        edit_entry(hwnd, &state, None, Some(value));
    }
    if IsWindow(hwnd) != 0 {
        modal_loop(hwnd, owner, ID_PASTE, false);
    }
    let value = {
        let mut data = state.borrow_mut();
        data.session.take();
        data.paste.take()
    };
    if let (Some(value), Some((target, focus))) = (value, target) {
        let ptr = get_state_ptr(owner);
        if !ptr.is_null()
            && !paste_protected_text_to_target(owner, &mut *ptr, &value, target, focus)
        {
            error(
                owner,
                "目标输入框已不可用，未粘贴密码与密钥。请重新选择输入框后重试。",
            );
        }
    }
}

unsafe fn current_process_vault() -> HWND {
    let class = to_wide(VAULT_CLASS);
    let process = GetCurrentProcessId();
    let mut candidate = null_mut();
    loop {
        candidate = FindWindowExW(null_mut(), candidate, class.as_ptr(), null());
        if candidate.is_null() {
            return null_mut();
        }
        let mut window_process = 0;
        GetWindowThreadProcessId(candidate, &mut window_process);
        if window_process == process {
            return candidate;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_rows_use_home_row_geometry_and_theme_without_secret_payloads() {
        unsafe {
            let dc = platform_gdi::create_compatible_dc(null_mut());
            let width = 400;
            let height = 132;
            let (bitmap, bits) = platform_gdi::create_top_down_32bpp_dib(dc, width, height);
            assert!(!bitmap.is_null() && !bits.is_null());
            let old = platform_gdi::select_object(dc, bitmap as _);
            let mut theme = Theme::default();
            theme.surface = rgb(255, 255, 255);
            theme.text = rgb(28, 28, 28);
            theme.text_muted = rgb(96, 96, 96);
            theme.item_selected = rgb(228, 240, 250);
            theme.item_hover = rgb(247, 247, 247);
            for (index, (label, notes)) in [
                ("GitHub 访问令牌", "代码仓库与自动发布"),
                ("开发服务器 SSH 密钥", "工作环境 · 每月检查"),
                ("邮箱登录密码", ""),
            ]
            .into_iter()
            .enumerate()
            {
                let row = EntrySummary {
                    id: format!("synthetic-{index}"),
                    kind: crate::secret_vault::VaultKind::Key,
                    label: label.into(),
                    notes: notes.into(),
                };
                draw_summary_row(
                    dc,
                    RECT {
                        left: 0,
                        top: index as i32 * 44,
                        right: width,
                        bottom: (index as i32 + 1) * 44,
                    },
                    &row,
                    index == 1,
                    index == 2,
                    96,
                    theme,
                );
            }
            let bytes =
                core::slice::from_raw_parts_mut(bits as *mut u8, (width * height * 4) as usize);
            for (y, color) in [
                (1, theme.surface),
                (45, theme.item_selected),
                (89, theme.item_hover),
            ] {
                let pixel = &bytes[((y * width + 1) * 4) as usize..][..3];
                assert_eq!(
                    pixel,
                    &[
                        ((color >> 16) & 255) as u8,
                        ((color >> 8) & 255) as u8,
                        (color & 255) as u8
                    ]
                );
            }
            if let Some(directory) = std::env::var_os("ZSCLIP_TEST_TEMP_ROOT") {
                let target = PathBuf::from(directory).join("vault-summary-rows.png");
                let mut rgba = bytes.to_vec();
                for pixel in rgba.chunks_exact_mut(4) {
                    pixel.swap(0, 2);
                    pixel[3] = 255;
                }
                let file = std::fs::File::create(target).unwrap();
                let mut encoder = png::Encoder::new(file, width as u32, height as u32);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                encoder
                    .write_header()
                    .unwrap()
                    .write_image_data(&rgba)
                    .unwrap();
            }
            platform_gdi::select_object(dc, old);
            DeleteObject(bitmap as _);
            platform_gdi::delete_dc(dc);
        }
    }

    unsafe fn hidden_parent() -> HWND {
        let hwnd = CreateWindowExW(
            0,
            to_wide("STATIC").as_ptr(),
            to_wide("Synthetic UI test").as_ptr(),
            WS_OVERLAPPED,
            0,
            0,
            800,
            700,
            null_mut(),
            null_mut(),
            platform_window::module_handle(),
            null(),
        );
        assert!(!hwnd.is_null());
        hwnd
    }

    fn synthetic_state(parent: HWND, embedded: bool) -> VaultRef {
        Rc::new(RefCell::new(VaultWindow {
            parent,
            embedded,
            restore_noactivate: false,
            session: None,
            rows: Vec::new(),
            target: None,
            paste: None,
            font: null_mut(),
        }))
    }

    #[test]
    fn labels_and_edit_controls_use_the_selected_theme_without_system_gray_bars() {
        unsafe {
            use windows_sys::Win32::Graphics::Gdi::{
                CreateCompatibleDC, DeleteDC, GetBkColor, GetTextColor,
            };
            let dc = CreateCompatibleDC(null_mut());
            assert!(!dc.is_null());
            let mut theme = Theme::default();
            theme.surface = 0x00ffffff;
            theme.control_bg = 0x00ffffff;
            theme.text = 0x00123456;
            for editable in [false, true] {
                assert_ne!(theme_control_brush(dc, theme, editable), 0);
                assert_eq!(GetBkColor(dc), 0x00ffffff);
                assert_eq!(GetTextColor(dc), 0x00123456);
            }
            DeleteDC(dc);
        }
    }

    #[test]
    fn closing_embedded_vault_clears_editor_confirm_and_password_and_keeps_callback_state_alive() {
        unsafe {
            let parent = hidden_parent();
            let state = synthetic_state(parent, true);
            let weak = Rc::downgrade(&state);
            let panel = create_vault_window(parent, &state);
            assert!(!panel.is_null());
            let callback_state = retain_vault_state(
                GetWindowLongPtrW(panel, GWLP_USERDATA) as *const RefCell<VaultWindow>
            );
            register(ENTRY_CLASS, Some(entry_proc));
            register(MESSAGE_CLASS, Some(message_proc));
            register(PASSWORD_CLASS, Some(password_proc));
            let mut editor_data = EntryWindow {
                initial: Zeroizing::new("synthetic key".into()),
                label: "Synthetic".into(),
                notes: "Synthetic note".into(),
                revealed: false,
                result: Some(EntryInput {
                    label: "Pending".into(),
                    notes: String::new(),
                    secret: Zeroizing::new("synthetic pending".into()),
                }),
                font: null_mut(),
            };
            let editor = window(
                panel,
                ENTRY_CLASS,
                "Synthetic editor",
                620,
                440,
                &mut editor_data,
            );
            set_sensitive(control(editor, ID_SECRET), "synthetic visible editor");
            let mut confirmation = MessageWindow {
                message: "Synthetic deletion".into(),
                confirm: true,
                result: true,
                font: null_mut(),
            };
            let confirm = window(
                panel,
                MESSAGE_CLASS,
                "Synthetic confirmation",
                480,
                220,
                &mut confirmation,
            );
            let mut password_data = PasswordWindow {
                setting: true,
                result: Some(Zeroizing::new("synthetic pending password".into())),
                font: null_mut(),
            };
            let password = window(
                panel,
                PASSWORD_CLASS,
                "Synthetic password",
                480,
                278,
                &mut password_data,
            );
            assert!(belongs_to(editor, panel));
            assert!(belongs_to(confirm, panel));
            drop(state);
            close_main_view(parent);
            for handle in [panel, editor, confirm, password] {
                assert_eq!(IsWindow(handle), 0);
            }
            assert!(editor_data.initial.is_empty());
            assert!(editor_data.result.is_none());
            assert!(editor_data.label.is_empty());
            assert!(editor_data.notes.is_empty());
            assert!(!confirmation.result);
            assert!(password_data.result.is_none());
            assert!(callback_state.borrow().session.is_none());
            assert!(weak.upgrade().is_some());
            drop(callback_state);
            assert!(weak.upgrade().is_none());
            DestroyWindow(parent);
        }
    }

    #[test]
    fn embedded_focus_scope_rejects_siblings_and_owner_generation_reuse() {
        unsafe {
            let parent = hidden_parent();
            let state = synthetic_state(parent, true);
            let panel = create_vault_window(parent, &state);
            register(ENTRY_CLASS, Some(entry_proc));
            register(MESSAGE_CLASS, Some(message_proc));
            let mut entry = EntryWindow {
                initial: Zeroizing::new("synthetic".into()),
                label: String::new(),
                notes: String::new(),
                revealed: false,
                result: None,
                font: null_mut(),
            };
            let editor = window(panel, ENTRY_CLASS, "Synthetic editor", 620, 440, &mut entry);
            let mut sibling_state = MessageWindow {
                message: "Unrelated settings".into(),
                confirm: false,
                result: false,
                font: null_mut(),
            };
            let sibling = window(
                parent,
                MESSAGE_CLASS,
                "Unrelated sibling",
                480,
                220,
                &mut sibling_state,
            );
            assert!(foreground_belongs_to_view(
                panel,
                parent,
                control(panel, ID_LIST),
                true
            ));
            assert!(foreground_belongs_to_view(
                panel,
                editor,
                control(editor, ID_SECRET),
                true
            ));
            assert!(!foreground_belongs_to_view(panel, parent, parent, true));
            assert!(!foreground_belongs_to_view(panel, sibling, sibling, true));
            let generation = GetPropW(panel, to_wide(GENERATION_PROPERTY).as_ptr()) as usize;
            SetPropW(
                editor,
                to_wide(OWNER_GENERATION_PROPERTY).as_ptr(),
                generation.wrapping_add(1000) as _,
            );
            assert!(logical_owner(editor).is_null());
            SendMessageW(editor, CHECK_LOCK, 0, 0);
            assert_eq!(IsWindow(editor), 0);
            assert!(entry.initial.is_empty());
            close_main_view(parent);
            DestroyWindow(sibling);
            DestroyWindow(parent);
        }
    }

    #[test]
    fn enter_targets_buttons_and_cancel_never_accepts_or_pastes() {
        unsafe {
            let parent = hidden_parent();
            let state = synthetic_state(parent, true);
            let panel = create_vault_window(parent, &state);
            assert_eq!(
                focused_enter_command(panel, control(panel, ID_LIST)),
                Some(ID_PASTE)
            );
            for id in [
                ID_ADD,
                ID_EDIT,
                ID_DELETE,
                ID_MANAGE,
                ID_REMOVE_PASSWORD,
                ID_CLOSE,
            ] {
                assert_eq!(focused_enter_command(panel, control(panel, id)), Some(id));
                assert_ne!(
                    focused_enter_command(panel, control(panel, id)),
                    Some(ID_PASTE)
                );
            }
            assert_eq!(focused_enter_command(panel, parent), None);
            register(MESSAGE_CLASS, Some(message_proc));
            let mut message = MessageWindow {
                message: "Synthetic deletion".into(),
                confirm: true,
                result: false,
                font: null_mut(),
            };
            let dialog = window(
                panel,
                MESSAGE_CLASS,
                "Synthetic confirm",
                480,
                220,
                &mut message,
            );
            assert_eq!(
                GetPropW(dialog, to_wide(DEFAULT_FOCUS_PROPERTY).as_ptr()),
                control(dialog, ID_CLOSE)
            );
            assert!(activate_focused_button(control(dialog, ID_CLOSE)));
            assert_eq!(IsWindow(dialog), 0);
            assert!(!message.result);
            close_main_view(parent);
            DestroyWindow(parent);
        }
    }

    #[test]
    fn native_row_hit_rejects_blank_drag_and_different_release_rows() {
        unsafe {
            let parent = hidden_parent();
            let list = child(
                parent,
                null_mut(),
                "LISTBOX",
                "",
                ID_LIST,
                WS_VSCROLL | LBS_NOTIFY as u32,
                0,
                0,
                300,
                300,
            );
            for label in ["Synthetic A", "Synthetic B"] {
                SendMessageW(list, LB_ADDSTRING, 0, to_wide(label).as_ptr() as isize);
            }
            let mut a: RECT = zeroed();
            let mut b: RECT = zeroed();
            assert_ne!(
                SendMessageW(list, LB_GETITEMRECT, 0, &mut a as *mut _ as isize),
                LB_ERR as isize
            );
            assert_ne!(
                SendMessageW(list, LB_GETITEMRECT, 1, &mut b as *mut _ as isize),
                LB_ERR as isize
            );
            let point =
                |x: i32, y: i32| ((x as u16 as usize) | ((y as u16 as usize) << 16)) as isize;
            let hit_a = point(5, (a.top + a.bottom) / 2);
            let hit_b = point(5, (b.top + b.bottom) / 2);
            let blank = point(5, b.bottom + 20);
            assert_eq!(list_item_at(list, hit_a), Some(0));
            assert_eq!(list_item_at(list, hit_b), Some(1));
            assert_eq!(list_item_at(list, blank), None);
            SetPropW(list, to_wide("ZSClipVaultPressedRow").as_ptr(), 1usize as _);
            SetPropW(
                list,
                to_wide("ZSClipVaultPressedButton").as_ptr(),
                1usize as _,
            );
            assert_eq!(completed_row_click(list, hit_a, 1), Some(0));
            assert_eq!(completed_row_click(list, hit_b, 1), None);
            assert_eq!(completed_row_click(list, blank, 1), None);
            SetPropW(list, to_wide("ZSClipVaultDragged").as_ptr(), 1usize as _);
            assert_eq!(completed_row_click(list, hit_a, 1), None);
            RemovePropW(list, to_wide("ZSClipVaultDragged").as_ptr());
            SetPropW(
                list,
                to_wide("ZSClipVaultPressedButton").as_ptr(),
                2usize as _,
            );
            assert_eq!(completed_row_click(list, hit_a, 2), Some(0));
            assert_eq!(completed_row_click(list, blank, 2), None);
            DestroyWindow(list);
            DestroyWindow(parent);
        }
    }
    #[test]
    fn sensitive_native_controls_preserve_long_untrimmed_values_and_clear() {
        unsafe {
            // Hidden, isolated controls contain synthetic strings only.
            let host = CreateWindowExW(
                0,
                to_wide("STATIC").as_ptr(),
                to_wide("secret test").as_ptr(),
                WS_OVERLAPPED,
                0,
                0,
                500,
                500,
                null_mut(),
                null_mut(),
                platform_window::module_handle(),
                null(),
            );
            assert!(!host.is_null());
            for style in [ES_PASSWORD | ES_AUTOHSCROLL, ES_MULTILINE | ES_AUTOVSCROLL] {
                let edit = CreateWindowExW(
                    0,
                    to_wide("EDIT").as_ptr(),
                    to_wide("").as_ptr(),
                    WS_CHILD | style as u32,
                    0,
                    0,
                    450,
                    400,
                    host,
                    null_mut(),
                    platform_window::module_handle(),
                    null(),
                );
                assert!(!edit.is_null());
                SendMessageW(edit, EM_SETLIMITTEXT_, MAX_SECRET_CHARS, 0);
                let value = if style & ES_MULTILINE != 0 {
                    Zeroizing::new(format!("  synthetic key\r\n{}\r\n  ", "a".repeat(4000)))
                } else {
                    Zeroizing::new(format!("  {}  ", "synthetic".repeat(500)))
                };
                set_sensitive(edit, &value);
                assert!(read_sensitive(edit).as_str() == value.as_str());
                wipe_edit(edit);
                assert!(read_sensitive(edit).is_empty());
                DestroyWindow(edit);
            }
            DestroyWindow(host);
        }
    }

    #[test]
    fn leaving_a_nested_editor_destroys_the_vault_and_clears_pending_secrets() {
        unsafe {
            let parent = CreateWindowExW(
                0,
                to_wide("STATIC").as_ptr(),
                to_wide("vault owner test").as_ptr(),
                WS_OVERLAPPED,
                0,
                0,
                500,
                500,
                null_mut(),
                null_mut(),
                platform_window::module_handle(),
                null(),
            );
            assert!(!parent.is_null());
            register(VAULT_CLASS, Some(vault_proc));
            register(ENTRY_CLASS, Some(entry_proc));
            let vault = Rc::new(RefCell::new(VaultWindow {
                parent,
                embedded: false,
                restore_noactivate: false,
                session: None,
                rows: Vec::new(),
                target: None,
                paste: None,
                font: null_mut(),
            }));
            let root = create_vault_window(parent, &vault);
            assert!(!root.is_null());
            let mut entry = EntryWindow {
                initial: Zeroizing::new("synthetic pending key".into()),
                label: "Synthetic".into(),
                notes: "Synthetic notes".into(),
                revealed: false,
                result: None,
                font: null_mut(),
            };
            let editor = window(root, ENTRY_CLASS, "Synthetic editor", 620, 440, &mut entry);
            assert!(!editor.is_null());
            assert!(belongs_to(editor, root));
            assert!(!belongs_to(parent, root));
            // Use explicit synthetic observations: no window is shown or activated,
            // and concurrent native tests cannot change the decision under test.
            lock_if_foreground_outside(root, editor, control(editor, ID_SECRET));
            lock_if_foreground_outside(editor, editor, control(editor, ID_SECRET));
            assert_ne!(IsWindow(root), 0);
            assert_ne!(IsWindow(editor), 0);
            lock_if_foreground_outside(editor, parent, parent);
            assert_eq!(IsWindow(root), 0);
            assert_eq!(IsWindow(editor), 0);
            assert!(entry.initial.is_empty());
            assert!(entry.label.is_empty());
            assert!(entry.notes.is_empty());
            DestroyWindow(parent);
        }
    }
}

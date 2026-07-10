use windows_sys::Win32::{
    Foundation::{HINSTANCE, HWND, POINT, RECT},
    Graphics::Gdi::{ClientToScreen, ScreenToClient},
    Graphics::Gdi::{SetWindowRgn, HRGN},
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{
        BeginDeferWindowPos, ChildWindowFromPointEx, CreateWindowExW, DefWindowProcW,
        DeferWindowPos, DestroyWindow, DispatchMessageW, EndDeferWindowPos, EnumWindows,
        FindWindowW, GetAncestor, GetClassNameW, GetClientRect, GetDlgItem, GetForegroundWindow,
        GetGUIThreadInfo, GetMessageW, GetSystemMetrics, GetWindowLongPtrW, GetWindowLongW,
        GetWindowRect, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
        IsWindow, IsWindowVisible, LoadCursorW, MoveWindow, PostMessageW, PostQuitMessage,
        RegisterClassExW, RegisterClassW, RegisterWindowMessageW, SendMessageW,
        SetForegroundWindow, SetWindowLongPtrW, SetWindowLongW, SetWindowPos, SetWindowTextW,
        ShowWindow, TranslateMessage, WindowFromPoint, GA_ROOT, GUITHREADINFO, GWLP_USERDATA,
        GWL_EXSTYLE, GWL_STYLE, HMENU, IDC_ARROW, MSG, SWP_HIDEWINDOW, SWP_NOACTIVATE,
        SWP_NOREDRAW, SWP_NOZORDER, SWP_SHOWWINDOW, SW_HIDE, SW_RESTORE, SW_SHOW,
        SW_SHOWNOACTIVATE, WM_CLOSE, WM_NULL, WNDCLASSEXW, WNDCLASSW, WS_DISABLED,
    },
};

use crate::platform::{appearance as platform_appearance, input as platform_input};

pub(crate) const CHILD_FROM_POINT_SKIP_INVISIBLE: u32 = 0x0001;
pub(crate) const CHILD_FROM_POINT_SKIP_DISABLED: u32 = 0x0002;

#[derive(Clone, Copy, Debug)]
pub(crate) struct DeferredWindowPos {
    pub hwnd: HWND,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub visible: bool,
    pub was_visible: bool,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn EnableWindow(hwnd: HWND, benable: i32) -> i32;
    fn IsWindowEnabled(hwnd: HWND) -> i32;
    fn AttachThreadInput(id_attach: u32, id_attach_to: u32, attach: i32) -> i32;
    fn ShowScrollBar(hwnd: HWND, wbar: i32, bshow: i32) -> i32;
    fn GetParent(hwnd: HWND) -> HWND;
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

pub(crate) fn is_window_alive(hwnd: isize) -> bool {
    exists(hwnd as HWND)
}

pub(crate) fn exists(hwnd: HWND) -> bool {
    !hwnd.is_null() && unsafe { IsWindow(hwnd) != 0 }
}

pub(crate) fn destroy(hwnd: HWND) -> bool {
    !hwnd.is_null() && unsafe { DestroyWindow(hwnd) != 0 }
}

pub(crate) fn is_visible(hwnd: HWND) -> bool {
    !hwnd.is_null() && unsafe { IsWindowVisible(hwnd) != 0 }
}

pub(crate) fn is_minimized(hwnd: HWND) -> bool {
    !hwnd.is_null() && unsafe { IsIconic(hwnd) != 0 }
}

pub(crate) fn root_ancestor(hwnd: HWND) -> HWND {
    if hwnd.is_null() {
        return hwnd;
    }
    unsafe { GetAncestor(hwnd, GA_ROOT) }
}

pub(crate) fn is_root_window(hwnd: HWND) -> bool {
    !hwnd.is_null() && root_ancestor(hwnd) == hwnd
}

pub(crate) fn parent(hwnd: HWND) -> HWND {
    if hwnd.is_null() {
        return hwnd;
    }
    unsafe { GetParent(hwnd) }
}

pub(crate) fn child(parent: HWND, id: i32) -> HWND {
    if parent.is_null() {
        return core::ptr::null_mut();
    }
    unsafe { GetDlgItem(parent, id) }
}

pub(crate) fn child_from_point_ex(parent: HWND, point: POINT, flags: u32) -> HWND {
    if parent.is_null() {
        return core::ptr::null_mut();
    }
    unsafe { ChildWindowFromPointEx(parent, point, flags) }
}

pub(crate) fn class_name(hwnd: HWND) -> String {
    if hwnd.is_null() {
        return String::new();
    }
    let mut buf = [0u16; 128];
    let len = unsafe { GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    if len <= 0 {
        String::new()
    } else {
        String::from_utf16_lossy(&buf[..len as usize])
    }
}

pub(crate) fn text(hwnd: HWND) -> String {
    if hwnd.is_null() {
        return String::new();
    }
    let len = unsafe { GetWindowTextLengthW(hwnd) };
    if len <= 0 {
        return String::new();
    }
    let mut buf = vec![0u16; len as usize + 1];
    let read = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    if read <= 0 {
        String::new()
    } else {
        String::from_utf16_lossy(&buf[..read as usize])
    }
}

pub(crate) fn set_text(hwnd: HWND, text: &str) -> bool {
    if hwnd.is_null() {
        return false;
    }
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { SetWindowTextW(hwnd, wide.as_ptr()) != 0 }
}

pub(crate) fn window_thread_process_id(hwnd: HWND) -> (u32, u32) {
    if hwnd.is_null() {
        return (0, 0);
    }
    let mut pid = 0u32;
    let thread_id = unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    (thread_id, pid)
}

pub(crate) fn window_thread_id(hwnd: HWND) -> u32 {
    window_thread_process_id(hwnd).0
}

pub(crate) fn window_process_id(hwnd: HWND) -> u32 {
    window_thread_process_id(hwnd).1
}

pub(crate) fn gui_thread_info(thread_id: u32, info: &mut GUITHREADINFO) -> bool {
    if thread_id == 0 {
        return false;
    }
    unsafe { GetGUIThreadInfo(thread_id, info) != 0 }
}

pub(crate) fn system_metric(index: i32) -> i32 {
    unsafe { GetSystemMetrics(index) }
}

pub(crate) fn attach_thread_input(id_attach: u32, id_attach_to: u32, attach: bool) -> bool {
    if id_attach == 0 || id_attach_to == 0 {
        return false;
    }
    unsafe { AttachThreadInput(id_attach, id_attach_to, if attach { 1 } else { 0 }) != 0 }
}

pub(crate) fn foreground() -> HWND {
    unsafe { GetForegroundWindow() }
}

pub(crate) fn module_handle() -> HINSTANCE {
    unsafe { GetModuleHandleW(core::ptr::null()) }
}

pub(crate) fn arrow_cursor() -> *mut core::ffi::c_void {
    unsafe { LoadCursorW(core::ptr::null_mut(), IDC_ARROW) }
}

pub(crate) fn register_class_ex(wc: &WNDCLASSEXW) -> u16 {
    unsafe { RegisterClassExW(wc) }
}

pub(crate) fn register_class(wc: &WNDCLASSW) -> u16 {
    unsafe { RegisterClassW(wc) }
}

pub(crate) fn register_window_message(name: &str) -> u32 {
    let name = wide_null(name);
    unsafe { RegisterWindowMessageW(name.as_ptr()) }
}

pub(crate) fn find_window_by_class(class_name: &str) -> HWND {
    let class_name = wide_null(class_name);
    unsafe { FindWindowW(class_name.as_ptr(), core::ptr::null()) }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn create_window_ex(
    ex_style: u32,
    class_name: *const u16,
    window_name: *const u16,
    style: u32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    parent: HWND,
    menu: HMENU,
    instance: HINSTANCE,
    param: *const core::ffi::c_void,
) -> HWND {
    unsafe {
        CreateWindowExW(
            ex_style,
            class_name,
            window_name,
            style,
            x,
            y,
            width,
            height,
            parent,
            menu,
            instance,
            param,
        )
    }
}

pub(crate) fn is_foreground(hwnd: HWND) -> bool {
    !hwnd.is_null() && foreground() == hwnd
}

pub(crate) fn set_foreground(hwnd: HWND) {
    if hwnd.is_null() {
        return;
    }
    unsafe {
        SetForegroundWindow(hwnd);
    }
}

pub(crate) fn try_set_foreground(hwnd: HWND) -> bool {
    if hwnd.is_null() {
        return false;
    }
    unsafe { SetForegroundWindow(hwnd) != 0 }
}

pub(crate) fn force_foreground(hwnd: HWND) -> bool {
    if hwnd.is_null() {
        return false;
    }
    if try_set_foreground(hwnd) {
        return true;
    }
    platform_input::send_alt_tap();
    try_set_foreground(hwnd)
}

pub(crate) fn show(hwnd: HWND) {
    if hwnd.is_null() {
        return;
    }
    unsafe {
        ShowWindow(hwnd, SW_SHOW);
    }
}

pub(crate) fn show_no_activate(hwnd: HWND) {
    if hwnd.is_null() {
        return;
    }
    unsafe {
        ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    }
}

pub(crate) fn restore(hwnd: HWND) {
    if hwnd.is_null() {
        return;
    }
    unsafe {
        ShowWindow(hwnd, SW_RESTORE);
    }
}

pub(crate) fn hide(hwnd: HWND) {
    if hwnd.is_null() {
        return;
    }
    unsafe {
        ShowWindow(hwnd, SW_HIDE);
    }
}

pub(crate) fn set_visible(hwnd: HWND, visible: bool) {
    if visible {
        show(hwnd);
    } else {
        hide(hwnd);
    }
}

pub(crate) fn set_enabled(hwnd: HWND, enabled: bool) {
    if hwnd.is_null() {
        return;
    }
    unsafe {
        EnableWindow(hwnd, if enabled { 1 } else { 0 });
    }
}

pub(crate) fn is_enabled(hwnd: HWND) -> bool {
    !hwnd.is_null() && unsafe { IsWindowEnabled(hwnd) != 0 }
}

pub(crate) fn show_scrollbar(hwnd: HWND, bar: i32, visible: bool) -> bool {
    if hwnd.is_null() {
        return false;
    }
    unsafe { ShowScrollBar(hwnd, bar, if visible { 1 } else { 0 }) != 0 }
}

pub(crate) fn window_rect(hwnd: HWND) -> Option<RECT> {
    if hwnd.is_null() {
        return None;
    }
    let mut rc = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    if unsafe { GetWindowRect(hwnd, &mut rc) } != 0 {
        Some(rc)
    } else {
        None
    }
}

pub(crate) fn dock_rect(hwnd: HWND) -> RECT {
    let fallback = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    if let Some(rect) = platform_appearance::extended_frame_bounds(hwnd) {
        return rect;
    }
    window_rect(hwnd).unwrap_or(fallback)
}

pub(crate) fn point_in_rect_screen(point: &POINT, rect: &RECT) -> bool {
    point.x >= rect.left && point.x <= rect.right && point.y >= rect.top && point.y <= rect.bottom
}

pub(crate) fn cursor_over_window_tree(root_hwnd: HWND, cursor: POINT) -> bool {
    if !exists(root_hwnd) {
        return false;
    }
    let hit = window_from_point(cursor);
    if hit.is_null() {
        return point_in_rect_screen(&cursor, &dock_rect(root_hwnd));
    }

    let root = root_ancestor(hit);
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
        cur = parent(cur);
    }
    point_in_rect_screen(&cursor, &dock_rect(root_hwnd))
}

pub(crate) fn client_rect(hwnd: HWND) -> Option<RECT> {
    if hwnd.is_null() {
        return None;
    }
    let mut rc = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    if unsafe { GetClientRect(hwnd, &mut rc) } != 0 {
        Some(rc)
    } else {
        None
    }
}

pub(crate) fn move_window(hwnd: HWND, x: i32, y: i32, width: i32, height: i32, repaint: bool) {
    if hwnd.is_null() {
        return;
    }
    unsafe {
        MoveWindow(hwnd, x, y, width, height, if repaint { 1 } else { 0 });
    }
}

pub(crate) fn defer_move_windows(moves: &[DeferredWindowPos]) -> bool {
    if moves.is_empty() {
        return true;
    }
    let hdwp = unsafe { BeginDeferWindowPos(moves.len() as i32) };
    if hdwp.is_null() {
        return false;
    }
    let mut hdwp = hdwp;
    for item in moves {
        if item.hwnd.is_null() {
            continue;
        }
        let visibility_flag = match (item.visible, item.was_visible) {
            (true, false) => SWP_SHOWWINDOW,
            (false, true) => SWP_HIDEWINDOW,
            _ => 0,
        };
        let flags = SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOREDRAW | visibility_flag;
        let next = unsafe {
            DeferWindowPos(
                hdwp,
                item.hwnd,
                core::ptr::null_mut(),
                item.x,
                item.y,
                item.width,
                item.height,
                flags,
            )
        };
        if !next.is_null() {
            hdwp = next;
        }
    }
    unsafe { EndDeferWindowPos(hdwp) != 0 }
}

pub(crate) fn set_pos(
    hwnd: HWND,
    insert_after: HWND,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    flags: u32,
) -> bool {
    if hwnd.is_null() {
        return false;
    }
    unsafe { SetWindowPos(hwnd, insert_after, x, y, width, height, flags) != 0 }
}

pub(crate) fn set_window_region(hwnd: HWND, region: HRGN, redraw: bool) -> bool {
    if hwnd.is_null() {
        return false;
    }
    unsafe { SetWindowRgn(hwnd, region, if redraw { 1 } else { 0 }) != 0 }
}

pub(crate) fn screen_to_client(hwnd: HWND, pt: &mut POINT) -> bool {
    if hwnd.is_null() {
        return false;
    }
    unsafe { ScreenToClient(hwnd, pt) != 0 }
}

pub(crate) fn client_to_screen(hwnd: HWND, pt: &mut POINT) -> bool {
    if hwnd.is_null() {
        return false;
    }
    unsafe { ClientToScreen(hwnd, pt) != 0 }
}

pub(crate) fn window_from_point(pt: POINT) -> HWND {
    unsafe { WindowFromPoint(pt) }
}

pub(crate) fn post_hwnd_message(hwnd: HWND, msg: u32, wparam: usize, lparam: isize) -> bool {
    if hwnd.is_null() {
        return false;
    }
    unsafe { PostMessageW(hwnd, msg, wparam, lparam) != 0 }
}

pub(crate) fn send_message(hwnd: HWND, msg: u32, wparam: usize, lparam: isize) -> isize {
    if hwnd.is_null() {
        return 0;
    }
    unsafe { SendMessageW(hwnd, msg, wparam, lparam) }
}

pub(crate) fn get_window_long_ptr(hwnd: HWND, index: i32) -> isize {
    if hwnd.is_null() {
        return 0;
    }
    unsafe { GetWindowLongPtrW(hwnd, index) }
}

pub(crate) fn set_window_long_ptr(hwnd: HWND, index: i32, value: isize) -> isize {
    if hwnd.is_null() {
        return 0;
    }
    unsafe { SetWindowLongPtrW(hwnd, index, value) }
}

pub(crate) fn window_style(hwnd: HWND) -> u32 {
    if hwnd.is_null() {
        return 0;
    }
    unsafe { GetWindowLongW(hwnd, GWL_STYLE) as u32 }
}

pub(crate) fn window_ex_style(hwnd: HWND) -> u32 {
    if hwnd.is_null() {
        return 0;
    }
    unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 }
}

pub(crate) fn set_window_ex_style(hwnd: HWND, style: u32) -> u32 {
    if hwnd.is_null() {
        return 0;
    }
    unsafe { SetWindowLongW(hwnd, GWL_EXSTYLE, style as i32) as u32 }
}

pub(crate) fn is_enabled_by_style(hwnd: HWND) -> bool {
    window_style(hwnd) & WS_DISABLED == 0
}

pub(crate) fn user_data(hwnd: HWND) -> isize {
    get_window_long_ptr(hwnd, GWLP_USERDATA)
}

pub(crate) fn set_user_data(hwnd: HWND, value: isize) -> isize {
    set_window_long_ptr(hwnd, GWLP_USERDATA, value)
}

pub(crate) fn default_window_proc(hwnd: HWND, msg: u32, wparam: usize, lparam: isize) -> isize {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

pub(crate) fn close(hwnd: HWND) -> bool {
    post_hwnd_message(hwnd, WM_CLOSE, 0, 0)
}

pub(crate) fn post_quit_message(exit_code: i32) {
    unsafe {
        PostQuitMessage(exit_code);
    }
}

pub(crate) fn get_message(msg: &mut MSG) -> i32 {
    unsafe { GetMessageW(msg, core::ptr::null_mut(), 0, 0) }
}

pub(crate) fn translate_message(msg: &MSG) -> bool {
    unsafe { TranslateMessage(msg) != 0 }
}

pub(crate) fn dispatch_message(msg: &MSG) -> isize {
    unsafe { DispatchMessageW(msg) }
}

pub(crate) fn ping(hwnd: HWND) -> bool {
    post_hwnd_message(hwnd, WM_NULL, 0, 0)
}

pub(crate) fn post_message(hwnd: isize, msg: u32, wparam: usize, lparam: isize) {
    if hwnd == 0 {
        return;
    }
    unsafe {
        let _ = PostMessageW(hwnd as HWND, msg, wparam, lparam);
    }
}

pub(crate) unsafe fn post_boxed_message<T>(
    hwnd: isize,
    msg: u32,
    wparam: usize,
    payload: Box<T>,
) -> bool {
    let raw = Box::into_raw(payload);
    if hwnd == 0
        || IsWindow(hwnd as HWND) == 0
        || PostMessageW(hwnd as HWND, msg, wparam, raw as isize) == 0
    {
        drop(Box::from_raw(raw));
        return false;
    }
    true
}

unsafe extern "system" fn enum_visible_enabled_top_level_window(hwnd: HWND, lparam: isize) -> i32 {
    let out = &mut *(lparam as *mut Vec<HWND>);
    if is_visible(hwnd) && is_enabled_by_style(hwnd) && !is_minimized(hwnd) {
        out.push(hwnd);
    }
    1
}

pub(crate) fn visible_enabled_top_level_windows() -> Vec<HWND> {
    let mut windows = Vec::new();
    unsafe {
        EnumWindows(
            Some(enum_visible_enabled_top_level_window),
            &mut windows as *mut _ as isize,
        );
    }
    windows
}

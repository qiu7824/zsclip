use std::mem::{size_of, zeroed};

use windows_sys::Win32::{
    Foundation::{HWND, POINT},
    UI::WindowsAndMessaging::GUITHREADINFO,
};

use crate::app_core::{NativeTextCaretAnchor, NativeTextCaretHost};
use crate::platform::{
    accessibility as platform_accessibility, input as platform_input, window as platform_window,
};

#[derive(Clone, Copy, Default)]
pub(crate) struct WindowsTextCaretHost;

impl WindowsTextCaretHost {
    pub(crate) const fn new() -> Self {
        Self
    }
}

impl NativeTextCaretHost for WindowsTextCaretHost {
    type Handle = HWND;

    fn accessible_caret_anchor(&mut self, focus: Self::Handle) -> Option<NativeTextCaretAnchor> {
        let rect = unsafe { platform_accessibility::caret_rect(focus) }?;
        if !rect_has_area(rect.left, rect.top, rect.right, rect.bottom) {
            return None;
        }
        Some(NativeTextCaretAnchor::new(rect.left, rect.top, rect.bottom))
    }

    fn thread_caret_anchor(&mut self, target: Self::Handle) -> Option<NativeTextCaretAnchor> {
        if !platform_window::exists(target) {
            return None;
        }
        let thread_id = platform_window::window_thread_id(target);
        if thread_id == 0 {
            return None;
        }

        let mut info: GUITHREADINFO = unsafe { zeroed() };
        info.cbSize = size_of::<GUITHREADINFO>() as u32;
        if !platform_window::gui_thread_info(thread_id, &mut info)
            || !rect_has_point(
                info.rcCaret.left,
                info.rcCaret.top,
                info.rcCaret.right,
                info.rcCaret.bottom,
            )
        {
            return None;
        }

        let anchor_window = if !info.hwndCaret.is_null() {
            info.hwndCaret
        } else if !info.hwndFocus.is_null() {
            info.hwndFocus
        } else {
            target
        };
        if !platform_window::exists(anchor_window) {
            return None;
        }

        let mut top_left = POINT {
            x: info.rcCaret.left,
            y: info.rcCaret.top,
        };
        let mut bottom_left = POINT {
            x: info.rcCaret.left,
            y: if info.rcCaret.bottom > info.rcCaret.top {
                info.rcCaret.bottom
            } else {
                info.rcCaret.top + 24
            },
        };
        if !platform_window::client_to_screen(anchor_window, &mut top_left)
            || !platform_window::client_to_screen(anchor_window, &mut bottom_left)
        {
            return None;
        }

        Some(NativeTextCaretAnchor::new(
            top_left.x,
            top_left.y,
            bottom_left.y,
        ))
    }

    fn focus_rect_anchor(
        &mut self,
        focus: Self::Handle,
        max_width: i32,
        max_height: i32,
    ) -> Option<NativeTextCaretAnchor> {
        if !platform_window::exists(focus) {
            return None;
        }
        let rc = platform_window::window_rect(focus)?;
        if !rect_has_area(rc.left, rc.top, rc.right, rc.bottom) {
            return None;
        }
        let width = rc.right - rc.left;
        let height = rc.bottom - rc.top;
        if width <= 0 || height <= 0 || height > max_height || width > max_width {
            return None;
        }
        Some(NativeTextCaretAnchor::new(rc.left, rc.top, rc.bottom))
    }

    fn cursor_anchor(&mut self) -> Option<NativeTextCaretAnchor> {
        let pt = platform_input::cursor_pos()?;
        Some(NativeTextCaretAnchor::new(pt.x, pt.y, pt.y))
    }

    fn focus_handle_for_target(&mut self, target: Self::Handle) -> Self::Handle {
        if target.is_null() {
            return target;
        }
        let thread_id = platform_window::window_thread_id(target);
        let mut info: GUITHREADINFO = unsafe { zeroed() };
        info.cbSize = size_of::<GUITHREADINFO>() as u32;
        if thread_id != 0
            && platform_window::gui_thread_info(thread_id, &mut info)
            && !info.hwndFocus.is_null()
        {
            info.hwndFocus
        } else {
            target
        }
    }
}

fn rect_has_area(left: i32, top: i32, right: i32, bottom: i32) -> bool {
    right > left && bottom > top
}

fn rect_has_point(left: i32, top: i32, right: i32, bottom: i32) -> bool {
    left != 0 || top != 0 || right != 0 || bottom != 0
}

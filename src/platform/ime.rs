#![allow(non_snake_case)]

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, POINT, RECT, WPARAM},
    UI::WindowsAndMessaging::WM_IME_CONTROL,
};

use crate::app_core::{
    NativeImeCandidateAnchor, NativeImeCompositionAnchor, NativeImeHost, Point, UiRect,
};
use crate::platform::{input as platform_input, window as platform_window};

#[link(name = "user32")]
unsafe extern "system" {
    fn GetKeyboardLayout(idThread: u32) -> isize;
}

#[link(name = "imm32")]
unsafe extern "system" {
    fn ImmGetContext(hwnd: HWND) -> isize;
    fn ImmReleaseContext(hwnd: HWND, context: isize) -> i32;
    fn ImmGetOpenStatus(context: isize) -> i32;
    fn ImmGetConversionStatus(context: isize, conversion: *mut u32, sentence: *mut u32) -> i32;
    fn ImmIsIME(layout: isize) -> i32;
}

const IMC_GETCANDIDATEPOS: WPARAM = 0x0007;
const IMC_GETCOMPOSITIONWINDOW: WPARAM = 0x000B;
const CFS_RECT_V: u32 = 0x0001;
const CFS_POINT_V: u32 = 0x0002;
const CFS_FORCE_POSITION_V: u32 = 0x0020;
const CFS_CANDIDATEPOS_V: u32 = 0x0040;
const CFS_EXCLUDE_V: u32 = 0x0080;
const IME_CMODE_NATIVE_V: u32 = 0x0001;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WindowsImeInputMode {
    Native,
    Alphanumeric,
    Unknown,
}

#[repr(C)]
struct CandidateForm {
    dwIndex: u32,
    dwStyle: u32,
    ptCurrentPos: POINT,
    rcArea: RECT,
}

#[repr(C)]
struct CompositionForm {
    dwStyle: u32,
    ptCurrentPos: POINT,
    rcArea: RECT,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct WindowsImeHost;

impl WindowsImeHost {
    pub(crate) const fn new() -> Self {
        Self
    }

    pub(crate) fn input_mode(self, focus: HWND) -> WindowsImeInputMode {
        if !platform_window::exists(focus) {
            return WindowsImeInputMode::Unknown;
        }
        let thread_id = platform_window::window_thread_id(focus);
        if thread_id != 0 {
            let layout = unsafe { GetKeyboardLayout(thread_id) };
            if layout != 0 && unsafe { ImmIsIME(layout) } == 0 {
                return WindowsImeInputMode::Alphanumeric;
            }
        }

        let context = unsafe { ImmGetContext(focus) };
        if context == 0 {
            return WindowsImeInputMode::Unknown;
        }
        let open = unsafe { ImmGetOpenStatus(context) } != 0;
        let mut conversion = 0u32;
        let mut sentence = 0u32;
        let conversion_known =
            unsafe { ImmGetConversionStatus(context, &mut conversion, &mut sentence) } != 0;
        unsafe {
            ImmReleaseContext(focus, context);
        }

        classify_windows_ime_input_mode(open, conversion_known, conversion)
    }
}

fn classify_windows_ime_input_mode(
    open: bool,
    conversion_known: bool,
    conversion: u32,
) -> WindowsImeInputMode {
    if !open {
        WindowsImeInputMode::Alphanumeric
    } else if !conversion_known {
        WindowsImeInputMode::Unknown
    } else if conversion & IME_CMODE_NATIVE_V != 0 {
        WindowsImeInputMode::Native
    } else {
        WindowsImeInputMode::Alphanumeric
    }
}

impl NativeImeHost for WindowsImeHost {
    type Handle = HWND;

    fn candidate_anchor(
        &mut self,
        focus: Self::Handle,
        index: u32,
    ) -> Option<NativeImeCandidateAnchor> {
        if !platform_window::exists(focus) {
            return None;
        }
        let ime = platform_input::default_ime_window(focus);
        if !platform_window::exists(ime) {
            return None;
        }

        let mut candidate = CandidateForm {
            dwIndex: index,
            dwStyle: 0,
            ptCurrentPos: POINT { x: 0, y: 0 },
            rcArea: empty_rect(),
        };
        if platform_window::send_message(
            ime,
            WM_IME_CONTROL,
            IMC_GETCANDIDATEPOS,
            &mut candidate as *mut _ as LPARAM,
        ) != 0
        {
            return None;
        }

        match candidate.dwStyle {
            CFS_CANDIDATEPOS_V => point_to_screen(focus, candidate.ptCurrentPos)
                .map(|position| NativeImeCandidateAnchor::CandidatePoint { position }),
            CFS_EXCLUDE_V if rect_has_area(&candidate.rcArea) => {
                rect_to_screen(focus, candidate.rcArea)
                    .map(|rect| NativeImeCandidateAnchor::ExcludeRect { rect })
            }
            _ => None,
        }
    }

    fn composition_anchor(&mut self, focus: Self::Handle) -> Option<NativeImeCompositionAnchor> {
        if !platform_window::exists(focus) {
            return None;
        }
        let ime = platform_input::default_ime_window(focus);
        if !platform_window::exists(ime) {
            return None;
        }

        let mut composition = CompositionForm {
            dwStyle: 0,
            ptCurrentPos: POINT { x: 0, y: 0 },
            rcArea: empty_rect(),
        };
        if platform_window::send_message(
            ime,
            WM_IME_CONTROL,
            IMC_GETCOMPOSITIONWINDOW,
            &mut composition as *mut _ as LPARAM,
        ) != 0
        {
            return None;
        }

        match composition.dwStyle {
            CFS_POINT_V | CFS_FORCE_POSITION_V => point_to_screen(focus, composition.ptCurrentPos)
                .map(|position| NativeImeCompositionAnchor::Point { position }),
            CFS_RECT_V if rect_has_area(&composition.rcArea) => {
                rect_to_screen(focus, composition.rcArea)
                    .map(|rect| NativeImeCompositionAnchor::Rect { rect })
            }
            _ => None,
        }
    }

    fn has_default_ime_window(&mut self, focus: Self::Handle) -> bool {
        platform_window::exists(platform_input::default_ime_window(focus))
    }
}

const fn empty_rect() -> RECT {
    RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    }
}

fn rect_has_area(rect: &RECT) -> bool {
    rect.right > rect.left && rect.bottom > rect.top
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ime_input_mode_distinguishes_native_and_english_states() {
        assert_eq!(
            classify_windows_ime_input_mode(false, true, IME_CMODE_NATIVE_V),
            WindowsImeInputMode::Alphanumeric
        );
        assert_eq!(
            classify_windows_ime_input_mode(true, true, IME_CMODE_NATIVE_V),
            WindowsImeInputMode::Native
        );
        assert_eq!(
            classify_windows_ime_input_mode(true, true, 0),
            WindowsImeInputMode::Alphanumeric
        );
        assert_eq!(
            classify_windows_ime_input_mode(true, false, 0),
            WindowsImeInputMode::Unknown
        );
    }
}

fn point_to_screen(hwnd: HWND, mut point: POINT) -> Option<Point> {
    if platform_window::client_to_screen(hwnd, &mut point) {
        Some(Point {
            x: point.x,
            y: point.y,
        })
    } else {
        None
    }
}

fn rect_to_screen(hwnd: HWND, rect: RECT) -> Option<UiRect> {
    let mut top_left = POINT {
        x: rect.left,
        y: rect.top,
    };
    let mut bottom_right = POINT {
        x: rect.right,
        y: rect.bottom,
    };
    if platform_window::client_to_screen(hwnd, &mut top_left)
        && platform_window::client_to_screen(hwnd, &mut bottom_right)
    {
        Some(UiRect::new(
            top_left.x,
            top_left.y,
            bottom_right.x,
            bottom_right.y,
        ))
    } else {
        None
    }
}

use crate::app_core::{KeyState, LifecycleEvent, MouseButton, Point, Size, UiEvent};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    SIZE_MINIMIZED, WM_ACTIVATEAPP, WM_CANCELMODE, WM_CAPTURECHANGED, WM_CLIPBOARDUPDATE, WM_CLOSE,
    WM_COMMAND, WM_DESTROY, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_EXITSIZEMOVE, WM_HOTKEY,
    WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
    WM_MOUSEWHEEL, WM_MOVE, WM_RBUTTONUP, WM_SETTINGCHANGE, WM_SHOWWINDOW, WM_SIZE, WM_SYSKEYDOWN,
    WM_SYSKEYUP, WM_THEMECHANGED, WM_TIMER,
};

const WM_MOUSEHOVER: u32 = 0x02A1;
const WM_MOUSELEAVE: u32 = 0x02A3;

pub(crate) fn from_window_message(msg: u32, wparam: usize, lparam: isize) -> Option<UiEvent> {
    match msg {
        WM_MOUSEMOVE => Some(UiEvent::PointerMove {
            position: point_from_lparam(lparam),
        }),
        WM_MOUSEHOVER => Some(UiEvent::PointerHover {
            position: point_from_lparam(lparam),
        }),
        WM_MOUSELEAVE => Some(UiEvent::PointerLeave),
        WM_CAPTURECHANGED | WM_CANCELMODE => Some(UiEvent::PointerCancel),
        WM_LBUTTONDOWN => Some(pointer_button(lparam, MouseButton::Left, true, 1)),
        WM_LBUTTONUP => Some(pointer_button(lparam, MouseButton::Left, false, 1)),
        WM_LBUTTONDBLCLK => Some(pointer_button(lparam, MouseButton::Left, true, 2)),
        WM_RBUTTONUP => Some(pointer_button(lparam, MouseButton::Right, false, 1)),
        WM_MOUSEWHEEL => Some(UiEvent::MouseWheel {
            delta: wheel_delta(wparam),
        }),
        WM_KEYDOWN => Some(UiEvent::Key {
            code: wparam as u32,
            state: KeyState::Down,
            system: false,
        }),
        WM_KEYUP => Some(UiEvent::Key {
            code: wparam as u32,
            state: KeyState::Up,
            system: false,
        }),
        WM_SYSKEYDOWN => Some(UiEvent::Key {
            code: wparam as u32,
            state: KeyState::Down,
            system: true,
        }),
        WM_SYSKEYUP => Some(UiEvent::Key {
            code: wparam as u32,
            state: KeyState::Up,
            system: true,
        }),
        WM_COMMAND => {
            let (control_id, notification) = command_words(wparam);
            Some(UiEvent::ControlCommand {
                control_id: control_id as u32,
                notification,
            })
        }
        WM_HOTKEY => Some(UiEvent::GlobalHotkey { id: wparam as i32 }),
        WM_CLIPBOARDUPDATE => Some(UiEvent::ClipboardChanged),
        WM_SHOWWINDOW => Some(UiEvent::Lifecycle(if show_window_visible(wparam) {
            LifecycleEvent::Resume
        } else {
            LifecycleEvent::Suspend
        })),
        WM_DESTROY => Some(UiEvent::Lifecycle(LifecycleEvent::Unmount)),
        WM_TIMER => Some(UiEvent::Timer { id: wparam as u64 }),
        WM_SIZE => {
            let (width, height) = size_from_lparam(lparam);
            Some(UiEvent::WindowSize {
                size: Size { width, height }.clamp_non_negative(),
                minimized: size_is_minimized(wparam),
            })
        }
        WM_ACTIVATEAPP => Some(UiEvent::AppActivationChanged {
            active: wparam != 0,
        }),
        WM_SETTINGCHANGE | WM_DISPLAYCHANGE => Some(UiEvent::SystemMetricsChanged),
        WM_MOVE => Some(UiEvent::WindowMoved),
        WM_EXITSIZEMOVE => Some(UiEvent::WindowMoveCompleted),
        WM_CLOSE => Some(UiEvent::CloseRequested),
        WM_THEMECHANGED => Some(UiEvent::ThemeChanged),
        WM_DPICHANGED => Some(UiEvent::DpiChanged {
            dpi: dpi_from_wparam(wparam),
        }),
        _ => None,
    }
}

pub(crate) fn command_words(wparam: usize) -> (u16, u16) {
    (low_word(wparam), high_word(wparam))
}

pub(crate) fn dpi_from_wparam(wparam: usize) -> u32 {
    (low_word(wparam) as u32).max(96)
}

pub(crate) fn size_from_lparam(lparam: isize) -> (i32, i32) {
    (
        low_word(lparam as usize) as i32,
        high_word(lparam as usize) as i32,
    )
}

pub(crate) fn size_is_minimized(wparam: usize) -> bool {
    wparam == SIZE_MINIMIZED as usize
}

pub(crate) fn show_window_visible(wparam: usize) -> bool {
    wparam != 0
}

fn pointer_button(lparam: isize, button: MouseButton, pressed: bool, click_count: u8) -> UiEvent {
    UiEvent::PointerButton {
        position: point_from_lparam(lparam),
        button,
        pressed,
        click_count,
    }
}

fn point_from_lparam(lparam: isize) -> Point {
    Point {
        x: signed_low_word(lparam),
        y: signed_high_word(lparam),
    }
}

fn wheel_delta(wparam: usize) -> i32 {
    signed_high_word(wparam as isize)
}

fn low_word(value: usize) -> u16 {
    (value & 0xffff) as u16
}

fn high_word(value: usize) -> u16 {
    ((value >> 16) & 0xffff) as u16
}

fn signed_low_word(value: isize) -> i32 {
    (value as i16) as i32
}

fn signed_high_word(value: isize) -> i32 {
    ((value >> 16) as i16) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_lparam(x: i16, y: i16) -> isize {
        (x as u16 as usize | ((y as u16 as usize) << 16)) as isize
    }

    #[test]
    fn pointer_message_preserves_signed_client_coordinates() {
        assert_eq!(
            from_window_message(WM_MOUSEMOVE, 0, make_lparam(-12, 34)),
            Some(UiEvent::PointerMove {
                position: Point { x: -12, y: 34 }
            })
        );
    }

    #[test]
    fn wheel_and_key_messages_use_unified_events() {
        let wheel_wparam = ((-120i16 as u16 as usize) << 16) as usize;
        assert_eq!(
            from_window_message(WM_MOUSEWHEEL, wheel_wparam, make_lparam(10, 20)),
            Some(UiEvent::MouseWheel { delta: -120 })
        );
        assert_eq!(
            from_window_message(WM_KEYDOWN, 0x41, 0),
            Some(UiEvent::Key {
                code: 0x41,
                state: KeyState::Down,
                system: false,
            })
        );
        assert_eq!(
            from_window_message(WM_SYSKEYDOWN, 0x12, 0),
            Some(UiEvent::Key {
                code: 0x12,
                state: KeyState::Down,
                system: true,
            })
        );
    }

    #[test]
    fn pointer_button_messages_preserve_click_count() {
        assert_eq!(
            from_window_message(WM_LBUTTONDBLCLK, 0, make_lparam(7, 9)),
            Some(UiEvent::PointerButton {
                position: Point { x: 7, y: 9 },
                button: MouseButton::Left,
                pressed: true,
                click_count: 2,
            })
        );
    }

    #[test]
    fn clipboard_and_lifecycle_messages_are_mapped() {
        assert_eq!(
            from_window_message(WM_CLIPBOARDUPDATE, 0, 0),
            Some(UiEvent::ClipboardChanged)
        );
        assert_eq!(
            from_window_message(WM_TIMER, 42, 0),
            Some(UiEvent::Timer { id: 42 })
        );
        assert_eq!(
            from_window_message(WM_SHOWWINDOW, 1, 0),
            Some(UiEvent::Lifecycle(LifecycleEvent::Resume))
        );
        assert_eq!(
            from_window_message(WM_SHOWWINDOW, 0, 0),
            Some(UiEvent::Lifecycle(LifecycleEvent::Suspend))
        );
        assert_eq!(
            from_window_message(WM_DESTROY, 0, 0),
            Some(UiEvent::Lifecycle(LifecycleEvent::Unmount))
        );
        assert_eq!(
            from_window_message(WM_DPICHANGED, 144, 0),
            Some(UiEvent::DpiChanged { dpi: 144 })
        );
    }

    #[test]
    fn command_words_split_control_id_and_notification_code() {
        assert_eq!(command_words(0x0300_1001), (0x1001, 0x0300));
        assert_eq!(command_words(0x0000_FFFF), (0xFFFF, 0));
        assert_eq!(
            from_window_message(WM_COMMAND, 0x0300_1001, 0),
            Some(UiEvent::ControlCommand {
                control_id: 0x1001,
                notification: 0x0300,
            })
        );
        assert_eq!(
            from_window_message(WM_HOTKEY, 3, 0),
            Some(UiEvent::GlobalHotkey { id: 3 })
        );
    }

    #[test]
    fn dpi_and_size_helpers_decode_win32_message_words() {
        assert_eq!(dpi_from_wparam(0), 96);
        assert_eq!(dpi_from_wparam(144), 144);
        assert_eq!(size_from_lparam(640 | (480 << 16)), (640, 480));
        assert_eq!(
            from_window_message(WM_SIZE, 0, (640 | (480 << 16)) as isize),
            Some(UiEvent::WindowSize {
                size: Size {
                    width: 640,
                    height: 480
                },
                minimized: false,
            })
        );
        assert_eq!(
            from_window_message(WM_SIZE, SIZE_MINIMIZED as usize, 0),
            Some(UiEvent::WindowSize {
                size: Size {
                    width: 0,
                    height: 0
                },
                minimized: true,
            })
        );
        assert_eq!(
            from_window_message(WM_ACTIVATEAPP, 1, 0),
            Some(UiEvent::AppActivationChanged { active: true })
        );
        assert_eq!(
            from_window_message(WM_ACTIVATEAPP, 0, 0),
            Some(UiEvent::AppActivationChanged { active: false })
        );
        assert_eq!(
            from_window_message(WM_SETTINGCHANGE, 0, 0),
            Some(UiEvent::SystemMetricsChanged)
        );
        assert_eq!(
            from_window_message(WM_DISPLAYCHANGE, 0, 0),
            Some(UiEvent::SystemMetricsChanged)
        );
        assert_eq!(
            from_window_message(WM_MOVE, 0, 0),
            Some(UiEvent::WindowMoved)
        );
        assert_eq!(
            from_window_message(WM_EXITSIZEMOVE, 0, 0),
            Some(UiEvent::WindowMoveCompleted)
        );
        assert_eq!(
            from_window_message(WM_CLOSE, 0, 0),
            Some(UiEvent::CloseRequested)
        );
        assert!(size_is_minimized(SIZE_MINIMIZED as usize));
        assert!(!size_is_minimized(0));
        assert!(!size_is_minimized(2));
        assert!(show_window_visible(1));
        assert!(!show_window_visible(0));
    }
}

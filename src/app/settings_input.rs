use super::prelude::*;
use crate::win_system_params::SETTINGS_CLASS;

pub(super) unsafe fn route_settings_child_mouse_wheel(message: &MSG) -> bool {
    if message.message != WM_MOUSEWHEEL || message.hwnd.is_null() {
        return false;
    }
    let root = platform_window::root_ancestor(message.hwnd);
    if root.is_null() || root == message.hwnd || platform_window::class_name(root) != SETTINGS_CLASS
    {
        return false;
    }
    let st_ptr = platform_window::user_data(root) as *mut SettingsWndState;
    if !st_ptr.is_null() && settings_dropdown_popup_exists((*st_ptr).dropdown_popup) {
        return false;
    }
    platform_window::send_message(
        root,
        WM_MOUSEWHEEL,
        message.wParam as WPARAM,
        message.lParam as LPARAM,
    );
    true
}

pub(super) unsafe fn dispatch_settings_ui_event(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    event: UiEvent,
) -> Option<LRESULT> {
    match event {
        UiEvent::PointerMove { position } => Some(handle_settings_pointer_move(hwnd, position)),
        UiEvent::PointerLeave => Some(handle_settings_pointer_leave(hwnd)),
        UiEvent::PointerCancel => Some(handle_settings_pointer_cancel(hwnd)),
        UiEvent::PointerButton {
            position,
            button: UiMouseButton::Left,
            pressed: true,
            ..
        } => Some(handle_settings_lbutton_down(
            hwnd, msg, wparam, lparam, position,
        )),
        UiEvent::PointerButton {
            button: UiMouseButton::Left,
            pressed: false,
            ..
        } => Some(handle_settings_lbutton_up(hwnd, msg, wparam, lparam)),
        UiEvent::MouseWheel { delta } => Some(handle_settings_mouse_wheel(hwnd, delta)),
        UiEvent::Key {
            code,
            state: UiKeyState::Down,
            ..
        } => Some(handle_settings_key_down(hwnd, msg, wparam, lparam, code)),
        UiEvent::ControlCommand {
            control_id,
            notification,
        } => {
            let st_ptr = platform_window::user_data(hwnd) as *mut SettingsWndState;
            if st_ptr.is_null() {
                return Some(0);
            }
            if let Some(command) = settings_command_for_control(control_id as isize) {
                let st = &mut *st_ptr;
                queue_settings_command(st, command);
                drain_settings_ui_commands(hwnd, st);
                Some(0)
            } else if let Some(action) =
                settings_action_for_control(control_id as isize, notification)
            {
                let mut executor = WindowsSettingsActionExecutor::new(hwnd);
                dispatch_settings_action(&mut executor, &mut *st_ptr, action);
                Some(0)
            } else {
                None
            }
        }
        UiEvent::ControlSelectionChanged { control_id, index } => Some(
            handle_settings_control_selection(hwnd, control_id as isize, index),
        ),
        UiEvent::Timer { id } => {
            if let Some(task) = settings_timer_task_for_id(id as usize, SETTINGS_TIMER_IDS) {
                handle_settings_timer_task(hwnd, task);
            }
            Some(0)
        }
        UiEvent::ThemeChanged => Some(handle_settings_theme_changed(hwnd)),
        UiEvent::DpiChanged { dpi } => Some(handle_settings_dpi_changed(hwnd, lparam, dpi)),
        UiEvent::WindowSize { size, minimized } => {
            Some(handle_settings_window_size(hwnd, size, minimized))
        }
        UiEvent::SystemMetricsChanged => Some(handle_settings_system_metrics_changed(hwnd)),
        UiEvent::WindowMoved => Some(0),
        UiEvent::WindowMoveCompleted => Some(handle_settings_window_move_completed(hwnd)),
        UiEvent::CloseRequested => {
            destroy_settings_window(hwnd);
            Some(0)
        }
        UiEvent::Lifecycle(LifecycleEvent::Unmount) => Some(handle_settings_destroy(hwnd)),
        _ => None,
    }
}

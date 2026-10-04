use super::prelude::*;
use crate::win_system_params::SETTINGS_CLASS;

pub(super) unsafe fn dismiss_settings_dropdown_for_message(message: &MSG) -> bool {
    let root = platform_window::root_ancestor(message.hwnd);
    if platform_window::class_name(root) != SETTINGS_CLASS {
        return false;
    }
    let ptr = platform_window::user_data(root) as *mut SettingsWndState;
    if ptr.is_null() || !settings_dropdown_popup_exists((*ptr).dropdown_popup) {
        return false;
    }
    let escape = message.message == WM_KEYDOWN && hotkey::is_escape_vk(message.wParam as u32);
    if escape
        || matches!(
            message.message,
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_MOUSEWHEEL
        )
    {
        close_settings_dropdown_popup(&mut *ptr);
    }
    escape
}

pub(super) unsafe fn route_settings_child_mouse_wheel(message: &MSG) -> bool {
    if message.message != WM_MOUSEWHEEL || message.hwnd.is_null() {
        return false;
    }
    if platform_window::class_name(message.hwnd).eq_ignore_ascii_case("ListBox") {
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
            if control_id as isize == crate::win_system_params::IDC_SET_SOUND_TEST {
                let st = &mut *st_ptr;
                settings_collect_current_page_to_draft(st);
                play_paste_success_sound(&st.draft.paste_success_sound_kind, &st.draft.paste_success_sound_path);
                let message = match crate::shell::last_feedback_sound_playback() {
                    crate::shell::FeedbackSoundPlayback::Started => "已开始试听；若听不到声音，请检查系统音量与 ZSClip 应用音量。",
                    crate::shell::FeedbackSoundPlayback::DefaultFallback => "所选音频无法播放，已改用内置默认提示音。",
                    crate::shell::FeedbackSoundPlayback::SystemFallback => "内置音频无法播放，已改用系统默认提示音。",
                    crate::shell::FeedbackSoundPlayback::Failed => "未能开始播放，请检查 WAV 文件、音频输出与应用音量。",
                    crate::shell::FeedbackSoundPlayback::NotRequested => "尚未请求播放。",
                };
                settings_set_text(st.lb_sound_status, message);
                repaint_settings_window(hwnd, true);
                return Some(0);
            }
            // Update dialogs pump messages; route them before borrowing SettingsWndState.
            if super::settings_platform_actions_about::handle_about_update_control(hwnd, control_id as isize) {
                return Some(0);
            }
            #[cfg(feature="lan-sync")]
            if super::main_qq_cloud::is_settings_command(control_id as usize) {
                let parent=(*st_ptr).parent_hwnd;
                super::main_qq_cloud::handle_settings_command(hwnd,parent,control_id as usize);
                let current=platform_window::user_data(hwnd) as *mut SettingsWndState;
                if !current.is_null() { settings_sync_cloud_page_state(&mut *current); }
                return Some(0);
            }
            if control_id as usize == super::main_secret_vault::OPEN_VAULT {
                super::secret_vault_ui::open((*st_ptr).parent_hwnd, None);
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

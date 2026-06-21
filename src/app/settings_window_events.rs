use super::prelude::*;

pub(super) unsafe fn handle_settings_theme_changed(hwnd: HWND) -> LRESULT {
    let st_ptr = platform_window::user_data(hwnd) as *mut SettingsWndState;
    if !st_ptr.is_null() {
        settings_refresh_theme_resources(&mut *st_ptr);
        platform_appearance::apply_dark_mode_to_window(hwnd);
        repaint_settings_window(hwnd, true);
    }
    0
}

pub(super) unsafe fn apply_dpi_suggested_rect(hwnd: HWND, lparam: LPARAM) {
    if lparam == 0 {
        return;
    }
    let suggested = &*(lparam as *const RECT);
    set_settings_window_bounds(
        hwnd,
        UiRect::new(
            suggested.left,
            suggested.top,
            suggested.right,
            suggested.bottom,
        ),
    );
}

pub(super) unsafe fn handle_settings_dpi_changed(hwnd: HWND, lparam: LPARAM, dpi: u32) -> LRESULT {
    apply_dpi_suggested_rect(hwnd, lparam);
    set_settings_ui_dpi(dpi);
    ensure_settings_window_in_work_area(hwnd);
    let st_ptr = platform_window::user_data(hwnd) as *mut SettingsWndState;
    if !st_ptr.is_null() {
        reset_settings_dpi_compensation(&mut *st_ptr);
        (*st_ptr).ui_dpi = dpi;
        refresh_settings_window_metrics(hwnd, &mut *st_ptr);
    }
    0
}

pub(super) unsafe fn handle_settings_window_size(
    hwnd: HWND,
    _size: UiSize,
    minimized: bool,
) -> LRESULT {
    let st_ptr = platform_window::user_data(hwnd) as *mut SettingsWndState;
    if !st_ptr.is_null() && !minimized {
        let st = &mut *st_ptr;
        if !platform_dpi::is_per_monitor_aware() && !st.dpi_comp.is_applying() {
            update_settings_dpi_compensation_base(hwnd, st);
        }
        refresh_settings_window_metrics(hwnd, st);
    }
    0
}

pub(super) unsafe fn handle_settings_system_metrics_changed(hwnd: HWND) -> LRESULT {
    let st_ptr = platform_window::user_data(hwnd) as *mut SettingsWndState;
    if !st_ptr.is_null() {
        set_settings_ui_dpi(settings_window_layout_dpi(hwnd).max(96));
        let _ = apply_settings_system_dpi_compensation(hwnd, &mut *st_ptr);
        ensure_settings_window_in_work_area(hwnd);
        refresh_settings_window_metrics(hwnd, &mut *st_ptr);
    }
    0
}

pub(super) unsafe fn handle_settings_window_move_completed(hwnd: HWND) -> LRESULT {
    let st_ptr = platform_window::user_data(hwnd) as *mut SettingsWndState;
    if !st_ptr.is_null() {
        let next_dpi = settings_window_layout_dpi(hwnd).max(96);
        let old_dpi = (*st_ptr).ui_dpi.max(96);
        match settings_dpi_move_action(old_dpi, next_dpi, platform_dpi::is_per_monitor_aware()) {
            SettingsDpiMoveAction::ResizeForDpi => {
                (*st_ptr).ui_dpi = next_dpi;
                resize_settings_window_for_dpi_transition(hwnd, old_dpi, next_dpi);
                refresh_settings_window_metrics(hwnd, &mut *st_ptr);
            }
            SettingsDpiMoveAction::SyncOnly => {
                (*st_ptr).ui_dpi = next_dpi;
                set_settings_ui_dpi(next_dpi);
                refresh_settings_window_metrics(hwnd, &mut *st_ptr);
            }
            SettingsDpiMoveAction::None => {
                if !platform_dpi::is_per_monitor_aware() {
                    set_settings_ui_dpi(next_dpi);
                }
            }
        }
        ensure_settings_window_in_work_area(hwnd);
    }
    0
}

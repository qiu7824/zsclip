use super::prelude::*;

pub(super) unsafe fn handle_settings_timer_task(hwnd: HWND, task: SettingsTimerTask) {
    match task {
        SettingsTimerTask::HideScrollbar => {
            let st_ptr = platform_window::user_data(hwnd) as *mut SettingsWndState;
            if !st_ptr.is_null() {
                let st = &mut *st_ptr;
                stop_flagged_timer(hwnd, ID_TIMER_SETTINGS_SCROLLBAR, &mut st.scroll_hide_timer);
                st.scroll_bar_visible = false;
                repaint_settings_window(hwnd, false);
            }
        }
        SettingsTimerTask::ClearSaveHint => {
            let st_ptr = platform_window::user_data(hwnd) as *mut SettingsWndState;
            if !st_ptr.is_null() {
                let st = &mut *st_ptr;
                stop_flagged_timer(hwnd, ID_TIMER_SETTINGS_SAVE_HINT, &mut st.save_hint_timer);
                settings_set_text(st.btn_save, tr("保存", "Save"));
                repaint_settings_control(st.btn_save);
            }
        }
        SettingsTimerTask::DpiFit => {
            timer::stop(hwnd, ID_TIMER_SETTINGS_DPI_FIT);
            let st_ptr = platform_window::user_data(hwnd) as *mut SettingsWndState;
            if !st_ptr.is_null() {
                let st = &mut *st_ptr;
                let _ = apply_settings_system_dpi_compensation(hwnd, st);
                refresh_settings_window_metrics(hwnd, st);
            }
        }
    }
}

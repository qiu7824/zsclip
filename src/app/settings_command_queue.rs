use super::prelude::*;

unsafe fn show_settings_saved_feedback(hwnd: HWND, st: &mut SettingsWndState) {
    settings_set_text(st.btn_save, tr("已保存", "Saved"));
    repaint_settings_control(st.btn_save);
    start_flagged_timer(
        hwnd,
        ID_TIMER_SETTINGS_SAVE_HINT,
        1200,
        &mut st.save_hint_timer,
    );
}

pub(super) fn queue_settings_command(st: &mut SettingsWndState, command: Command) {
    st.ui_commands.push(command);
}

unsafe fn execute_settings_toggle_control(
    hwnd: HWND,
    st: &mut SettingsWndState,
    control_id: isize,
) {
    settings_toggle_flip(st, control_id);
    if control_id == IDC_SET_EDGEHIDE {
        settings_sync_pos_fields_enabled(st);
    }
    if let Some(page) = settings_page_to_sync_after_toggle(control_id) {
        settings_sync_page_state(st, page);
    }
    repaint_settings_window(hwnd, true);
}

unsafe fn execute_settings_ui_command(
    hwnd: HWND,
    st: &mut SettingsWndState,
    command: Command,
) -> bool {
    if command.scope != CommandScope::Window {
        return true;
    }
    match command.id {
        command_ids::SAVE_SETTINGS => {
            settings_collect_to_app(st);
            settings_apply_from_app(st);
            settings_sync_page_state(st, st.cur_page);
            show_settings_saved_feedback(hwnd, st);
            repaint_settings_window(hwnd, true);
            true
        }
        command_ids::CLOSE_SETTINGS => {
            destroy_settings_window(hwnd);
            false
        }
        command_ids::OPEN_SETTINGS_CONFIG => {
            open_settings_config_file(st);
            true
        }
        command_ids::OPEN_SETTINGS_DROPDOWN => {
            if let CommandPayload::ControlId(control_id) = command.payload {
                let _ = open_settings_dropdown_for_control(hwnd, st, control_id as isize);
            }
            true
        }
        command_ids::TOGGLE_SETTINGS_CONTROL => {
            if let CommandPayload::ControlId(control_id) = command.payload {
                execute_settings_toggle_control(hwnd, st, control_id as isize);
            }
            true
        }
        _ => true,
    }
}

pub(super) unsafe fn drain_settings_ui_commands(hwnd: HWND, st: &mut SettingsWndState) {
    while let Some(command) = st.ui_commands.pop() {
        if !execute_settings_ui_command(hwnd, st, command) {
            break;
        }
    }
}

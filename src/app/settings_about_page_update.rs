use super::prelude::*;
use crate::win_system_ui::settings_host_set_enabled;

pub(super) unsafe fn settings_create_about_update_section(
    st: &mut SettingsWndState,
    b: &SettingsPageBuilder,
    flow: &mut SettingsFlowLayout,
) {
    let update_state = update_check_state_snapshot();
    let update = settings_update_presentation(&SettingsUpdatePresentationInput {
        started: update_state.started,
        checking: update_state.checking,
        available: update_state.available,
        latest_tag: update_state.latest_tag.clone(),
        error: update_state.error.clone(),
    });
    let update_rect = flow.full_rect(settings_scale(64));
    let (status_label, update_h) = b.label_auto(
        st,
        &update.status_text,
        update_rect.left,
        update_rect.top,
        update_rect.right - update_rect.left,
        settings_scale(64),
    );
    st.lb_update_status = status_label;
    flow.consume_full(update_h, settings_scale(8));
    let update_button = flow.button_rect(settings_scale(184), settings_scale(32));
    st.btn_open_update = b.button(
        st,
        &update.button_text,
        IDC_SET_OPEN_UPDATE,
        update_button.left,
        update_button.top,
        update_button.right - update_button.left,
    );
    if !st.btn_open_update.is_null() {
        st.ownerdraw_ctrls.push(st.btn_open_update);
    }
    flow.consume_full(settings_scale(32), settings_scale(10));
}

pub(super) unsafe fn refresh_about_update_status(hwnd: HWND) {
    let ptr = platform_window::user_data(hwnd) as *mut SettingsWndState;
    if ptr.is_null() {
        return;
    }
    let st = &mut *ptr;
    if !st.ui.is_built(SettingsPage::About.index()) {
        return;
    }
    let state = update_check_state_snapshot();
    let presentation = settings_update_presentation(&SettingsUpdatePresentationInput {
        started: state.started,
        checking: state.checking,
        available: state.available,
        latest_tag: state.latest_tag,
        error: state.error,
    });
    settings_set_text(st.lb_update_status, &presentation.status_text);
    settings_set_text(st.btn_open_update, &presentation.button_text);
    settings_host_set_enabled(st.btn_open_update, !state.checking);
    repaint_settings_control(st.lb_update_status);
    repaint_settings_control(st.btn_open_update);
}

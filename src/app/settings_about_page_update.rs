use super::prelude::*;

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
    let update_rect = flow.full_rect(settings_scale(44));
    let (_, update_h) = b.label_auto(
        st,
        &update.status_text,
        update_rect.left,
        update_rect.top,
        update_rect.right - update_rect.left,
        settings_scale(44),
    );
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

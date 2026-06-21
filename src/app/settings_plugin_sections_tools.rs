use super::prelude::*;

pub(super) unsafe fn settings_relayout_plugin_tool_sections(st: &mut SettingsWndState) {
    let sec3 = settings_plugin_layout(st, 3, 0);
    settings_plugin_move_toggle_row(st, st.lb_ai_title, st.chk_ai, &sec3, 0);

    let sec4 = settings_plugin_layout(st, 4, 0);
    settings_plugin_move_toggle_row(st, st.lb_mail_merge_title, st.chk_mm, &sec4, 0);
    settings_plugin_move_control(
        st,
        st.btn_mail_merge,
        sec4.left(),
        sec4.row_y(1),
        settings_scale(170),
        settings_scale(32),
    );

    let sec5 = settings_plugin_layout(st, 5, 0);
    settings_plugin_move_toggle_row(st, st.lb_wps_taskpane_title, st.chk_wps_taskpane, &sec5, 0);
    settings_plugin_move_control(
        st,
        st.btn_wps_taskpane_docs,
        sec5.left(),
        sec5.row_y(1),
        settings_scale(170),
        settings_scale(32),
    );

    let sec6 = settings_plugin_layout(st, 6, 0);
    settings_plugin_move_toggle_row(st, st.lb_qr_title, st.chk_qr, &sec6, 0);
}

use super::prelude::*;

pub(super) unsafe fn settings_relayout_plugin_quick_search_section(
    st: &mut SettingsWndState,
    line_h: i32,
) {
    let sec0 = settings_plugin_layout(st, 0, 110);
    settings_plugin_move_toggle_row(st, st.lb_qs_title, st.chk_qs, &sec0, 0);
    settings_plugin_move_labeled_field(
        st,
        st.lb_qs_engine,
        st.cb_engine,
        &sec0,
        1,
        line_h,
        settings_scale(240),
        settings_scale(32),
    );
    settings_plugin_move_labeled_field(
        st,
        st.lb_qs_template,
        st.ed_tpl,
        &sec0,
        2,
        line_h,
        sec0.field_w(),
        settings_scale(32),
    );
    let qs_restore = sec0.field_action_hint_layout(
        3,
        settings_scale(130),
        settings_scale(24),
        settings_scale(4),
        line_h,
    );
    settings_plugin_move_control(
        st,
        st.btn_qs_restore,
        qs_restore.action_rect.left,
        qs_restore.action_rect.top,
        qs_restore.action_rect.right - qs_restore.action_rect.left,
        qs_restore.action_rect.bottom - qs_restore.action_rect.top,
    );
    settings_plugin_move_control(
        st,
        st.lb_qs_hint,
        qs_restore.hint_rect.left,
        qs_restore.hint_rect.top,
        qs_restore.hint_rect.right - qs_restore.hint_rect.left,
        qs_restore.hint_rect.bottom - qs_restore.hint_rect.top,
    );
}

pub(super) unsafe fn settings_relayout_plugin_ocr_section(st: &mut SettingsWndState, line_h: i32) {
    let sec1 = settings_plugin_layout(st, 1, 110);
    settings_plugin_move_labeled_field(
        st,
        st.lb_ocr_title,
        st.cb_ocr_provider,
        &sec1,
        0,
        line_h,
        settings_scale(220),
        settings_scale(32),
    );
    settings_plugin_move_control(
        st,
        st.lb_ocr_primary,
        sec1.left(),
        sec1.label_y(1, line_h),
        sec1.label_w(),
        line_h,
    );
    settings_plugin_move_control(
        st,
        st.ed_ocr_cloud_url,
        sec1.field_x(),
        sec1.row_y(1),
        sec1.field_w(),
        settings_scale(32),
    );
    settings_plugin_move_control(
        st,
        st.lb_ocr_secondary,
        sec1.left(),
        sec1.label_y(2, line_h),
        sec1.label_w(),
        line_h,
    );
    settings_plugin_move_control(
        st,
        st.ed_ocr_cloud_token,
        sec1.field_x(),
        sec1.row_y(2),
        sec1.field_w(),
        settings_scale(32),
    );
    settings_plugin_move_control(
        st,
        st.btn_ocr_detect,
        sec1.field_x(),
        sec1.row_y(2),
        settings_scale(180),
        settings_scale(32),
    );
}

pub(super) unsafe fn settings_relayout_plugin_translate_section(
    st: &mut SettingsWndState,
    line_h: i32,
) {
    let sec2 = settings_plugin_layout(st, 2, 110);
    settings_plugin_move_labeled_field(
        st,
        st.lb_translate_title,
        st.cb_translate_provider,
        &sec2,
        0,
        line_h,
        settings_scale(220),
        settings_scale(32),
    );
    settings_plugin_move_control(
        st,
        st.lb_translate_primary,
        sec2.left(),
        sec2.label_y(1, line_h),
        sec2.label_w(),
        line_h,
    );
    settings_plugin_move_control(
        st,
        st.ed_translate_app_id,
        sec2.field_x(),
        sec2.row_y(1),
        sec2.field_w(),
        settings_scale(32),
    );
    settings_plugin_move_control(
        st,
        st.lb_translate_secondary,
        sec2.left(),
        sec2.label_y(2, line_h),
        sec2.label_w(),
        line_h,
    );
    settings_plugin_move_control(
        st,
        st.ed_translate_secret,
        sec2.field_x(),
        sec2.row_y(2),
        sec2.field_w(),
        settings_scale(32),
    );
    settings_plugin_move_labeled_field(
        st,
        st.lb_translate_target,
        st.cb_translate_target,
        &sec2,
        3,
        line_h,
        settings_scale(180),
        settings_scale(32),
    );
}

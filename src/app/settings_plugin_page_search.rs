use super::prelude::*;

pub(super) unsafe fn settings_create_plugin_quick_search_page(
    st: &mut SettingsWndState,
    b: &SettingsPageBuilder,
    sec0: SettingsFormSectionLayout,
    line_h: i32,
) {
    let (qs_lbl, qs_btn) = b.own_toggle_row(
        st,
        tr("启用快速搜索", "Enable quick search"),
        7102,
        sec0.left(),
        sec0.row_y(0),
        sec0.full_w(),
    );
    st.lb_qs_title = qs_lbl;
    st.chk_qs = qs_btn;
    st.lb_qs_status = null_mut();
    st.lb_qs_engine = b.label(
        st,
        tr("搜索引擎：", "Search engine:"),
        sec0.left(),
        sec0.label_y(2, line_h),
        sec0.label_w(),
        line_h,
    );
    st.cb_engine = b.dropdown(
        st,
        tr("筑森搜索（zxx.vip）", "ZS Search (zxx.vip)"),
        7201,
        sec0.field_x(),
        sec0.row_y(2),
        settings_scale(240),
    );
    if !st.cb_engine.is_null() {
        st.ownerdraw_ctrls.push(st.cb_engine);
    }
    st.lb_qs_template = b.label(
        st,
        tr("URL 模板：", "URL template:"),
        sec0.left(),
        sec0.label_y(3, line_h),
        sec0.label_w(),
        line_h,
    );
    st.ed_tpl = b.edit(st, "", 7202, sec0.field_x(), sec0.row_y(3), sec0.field_w());
    let qs_restore = sec0.field_action_hint_layout(
        4,
        settings_scale(130),
        settings_scale(24),
        settings_scale(4),
        line_h,
    );
    st.btn_qs_restore = b.button(
        st,
        tr("恢复预设模板", "Restore preset"),
        7203,
        qs_restore.action_rect.left,
        qs_restore.action_rect.top,
        qs_restore.action_rect.right - qs_restore.action_rect.left,
    );
    if !st.btn_qs_restore.is_null() {
        st.ownerdraw_ctrls.push(st.btn_qs_restore);
    }
    st.lb_qs_hint = b
        .label_auto(
            st,
            tr(
                "占位符：{q}=编码后关键词，{raw}=原文",
                "Placeholders: {q}=encoded keyword, {raw}=original text",
            ),
            qs_restore.hint_rect.left,
            qs_restore.hint_rect.top,
            qs_restore.hint_rect.right - qs_restore.hint_rect.left,
            qs_restore.hint_rect.bottom - qs_restore.hint_rect.top,
        )
        .0;
}

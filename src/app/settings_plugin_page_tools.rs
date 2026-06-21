use super::prelude::*;

pub(super) unsafe fn settings_create_plugin_tools_page(
    st: &mut SettingsWndState,
    b: &SettingsPageBuilder,
    sec3: SettingsFormSectionLayout,
    sec4: SettingsFormSectionLayout,
    sec5: SettingsFormSectionLayout,
    sec6: SettingsFormSectionLayout,
) {
    let (ai_lbl, ai_btn) = b.own_toggle_row(
        st,
        tr("AI 文本清洗", "AI text cleanup"),
        7101,
        sec3.left(),
        sec3.row_y(0),
        sec3.full_w(),
    );
    st.lb_ai_title = ai_lbl;
    st.chk_ai = ai_btn;
    st.lb_ai_status = null_mut();
    let (mm_lbl, mm_btn) = b.own_toggle_row(
        st,
        tr("启用超级邮件合并", "Enable mail merge"),
        7103,
        sec4.left(),
        sec4.row_y(0),
        sec4.full_w(),
    );
    st.lb_mail_merge_title = mm_lbl;
    st.chk_mm = mm_btn;
    st.lb_mail_merge_status = null_mut();
    st.btn_mail_merge = b.button(
        st,
        tr("打开超级邮件合并", "Open mail merge"),
        IDC_SET_PLUGIN_MAILMERGE,
        sec4.left(),
        sec4.row_y(1),
        settings_scale(170),
    );
    if !st.btn_mail_merge.is_null() {
        st.ownerdraw_ctrls.push(st.btn_mail_merge);
    }
    let (wps_lbl, wps_btn) = b.own_toggle_row(
        st,
        tr("WPS 任务窗格", "WPS task pane"),
        7106,
        sec5.left(),
        sec5.row_y(0),
        sec5.full_w(),
    );
    st.lb_wps_taskpane_title = wps_lbl;
    st.chk_wps_taskpane = wps_btn;
    st.btn_wps_taskpane_docs = b.button(
        st,
        tr("打开任务窗格说明", "Open task pane guide"),
        IDC_SET_WPS_TASKPANE_DOCS,
        sec5.left(),
        sec5.row_y(1),
        settings_scale(170),
    );
    if !st.btn_wps_taskpane_docs.is_null() {
        st.ownerdraw_ctrls.push(st.btn_wps_taskpane_docs);
    }
    let (qr_lbl, qr_btn) = b.own_toggle_row(
        st,
        tr("启用快捷转换二维码", "Enable QR conversion"),
        7104,
        sec6.left(),
        sec6.row_y(0),
        sec6.full_w(),
    );
    st.lb_qr_title = qr_lbl;
    st.chk_qr = qr_btn;
    st.lb_qr_status = null_mut();
}

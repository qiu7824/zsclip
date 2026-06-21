use super::prelude::*;

pub(super) unsafe fn settings_create_cloud_webdav_page(
    st: &mut SettingsWndState,
    b: &SettingsPageBuilder,
    line_h: i32,
) {
    let sec1 = settings_multi_sync_layout(st, 1, 110);
    b.form_label(st, &sec1, 0, "WebDAV 地址：");
    st.ed_cloud_url = b.form_edit(st, &sec1, 0, "", IDC_SET_CLOUD_URL);
    b.form_label(st, &sec1, 1, "用户名：");
    st.ed_cloud_user = b.form_edit(st, &sec1, 1, "", IDC_SET_CLOUD_USER);
    b.form_label(st, &sec1, 2, "密码：");
    st.ed_cloud_pass = b.form_password_edit(st, &sec1, 2, "", IDC_SET_CLOUD_PASS);
    b.form_label(st, &sec1, 3, "远程目录：");
    st.ed_cloud_dir = b.form_edit(st, &sec1, 3, "", IDC_SET_CLOUD_DIR);
    b.form_label(st, &sec1, 4, "同步间隔：");
    st.cb_cloud_interval = b.form_dropdown(
        st,
        &sec1,
        4,
        "1小时",
        IDC_SET_CLOUD_INTERVAL,
        settings_scale(150),
    );
    if !st.cb_cloud_interval.is_null() {
        st.ownerdraw_ctrls.push(st.cb_cloud_interval);
    }
    st.lb_cloud_status = b.label(
        st,
        "上次同步：未同步",
        sec1.left(),
        sec1.label_y(5, line_h),
        sec1.full_w(),
        line_h,
    );

    let sec2 = settings_multi_sync_layout(st, 2, 0);
    let webdav_btn_w = settings_scale(130);
    b.form_action_row(
        st,
        &sec2,
        0,
        &[
            ("立即同步", IDC_SET_CLOUD_SYNC_NOW, webdav_btn_w),
            ("上传配置", IDC_SET_CLOUD_UPLOAD_CFG, webdav_btn_w),
        ],
    );
    b.form_action_row(
        st,
        &sec2,
        1,
        &[
            ("应用云端配置", IDC_SET_CLOUD_APPLY_CFG, webdav_btn_w),
            ("云备份恢复", IDC_SET_CLOUD_RESTORE_BACKUP, webdav_btn_w),
        ],
    );
}

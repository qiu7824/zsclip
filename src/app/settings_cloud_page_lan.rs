use super::prelude::*;

pub(super) unsafe fn settings_create_cloud_lan_page(
    st: &mut SettingsWndState,
    b: &SettingsPageBuilder,
    line_h: i32,
    lan_btn_w: i32,
    small_btn_w: i32,
) {
    let sec1 = settings_multi_sync_layout(st, 1, 110);
    st.lb_lan_status = b.label(
        st,
        "局域网同步：关闭",
        sec1.left(),
        sec1.label_y(0, line_h),
        sec1.full_w(),
        line_h,
    );
    b.form_label(st, &sec1, 1, "设备名称：");
    st.ed_lan_name = b.form_edit(st, &sec1, 1, "", IDC_SET_LAN_NAME);
    b.form_label(st, &sec1, 2, "TCP 端口：");
    st.ed_lan_tcp_port = b.edit(
        st,
        "",
        IDC_SET_LAN_TCP_PORT,
        sec1.field_x(),
        sec1.row_y(2),
        settings_scale(150),
    );
    b.form_label(st, &sec1, 3, "同步方式：");
    st.cb_lan_receive_mode = b.form_dropdown(
        st,
        &sec1,
        3,
        "只进入记录",
        IDC_SET_LAN_RECEIVE_MODE,
        settings_scale(190),
    );
    if !st.cb_lan_receive_mode.is_null() {
        st.ownerdraw_ctrls.push(st.cb_lan_receive_mode);
    }
    b.label(
        st,
        "可选择远端内容只入记录，或同步覆盖本机系统剪贴板。",
        sec1.left(),
        sec1.label_y(4, line_h),
        sec1.full_w(),
        line_h,
    );
    b.label(
        st,
        "扫码绑定页会生成 Android 配对码和 iOS/浏览器入口码。",
        sec1.left(),
        sec1.label_y(5, line_h),
        sec1.full_w(),
        line_h,
    );

    let sec2 = settings_multi_sync_layout(st, 2, 110);
    b.form_label(st, &sec2, 0, "手动 IP：");
    st.ed_lan_manual_host = b.form_edit(st, &sec2, 0, "", IDC_SET_LAN_MANUAL_HOST);
    let lan_actions = b.form_action_row(
        st,
        &sec2,
        1,
        &[
            ("配对选中设备", IDC_SET_LAN_PAIR, lan_btn_w),
            ("刷新发现", IDC_SET_LAN_REFRESH, lan_btn_w),
            ("允许配对", IDC_SET_LAN_ACCEPT_PAIR, lan_btn_w),
            ("拒绝", IDC_SET_LAN_REJECT_PAIR, small_btn_w),
        ],
    );
    st.btn_lan_pair = lan_actions.get(0).copied().unwrap_or(null_mut());
    st.btn_lan_refresh = lan_actions.get(1).copied().unwrap_or(null_mut());
    st.btn_lan_accept_pair = lan_actions.get(2).copied().unwrap_or(null_mut());
    st.btn_lan_reject_pair = lan_actions.get(3).copied().unwrap_or(null_mut());
    b.label(
        st,
        "附近设备 / 待允许请求：选中后点击对应按钮；没有发现时可输入手动 IP。",
        sec2.left(),
        sec2.label_y(2, line_h),
        sec2.full_w(),
        line_h,
    );
    st.lb_lan_devices = b.listbox(
        st,
        IDC_SET_LAN_DISCOVERED_LIST,
        sec2.left(),
        sec2.row_y(3),
        sec2.full_w(),
        settings_scale(190),
    );

    let sec3 = settings_multi_sync_layout(st, 3, 126);
    b.form_label(st, &sec3, 0, "信任设备：");
    st.lb_lan_trusted = b
        .form_value_label_auto(
            st,
            &sec3,
            0,
            "暂无。可输入 IP 手动连接，或等待自动发现后配对。",
            settings_scale(70),
        )
        .0;
    b.form_label(st, &sec3, 2, "绑定说明：");
    b.form_value_label(
        st,
        &sec3,
        2,
        "Android 扫码请求绑定；iOS/浏览器扫码打开手机连接页。",
    );
    (st.qr_lan_android, st.btn_lan_copy_pair) = b.form_qr_action(
        st,
        &sec3,
        3,
        "Android 配对：",
        IDC_SET_LAN_QR_ANDROID,
        "复制配对链接",
        IDC_SET_LAN_COPY_PAIR,
    );
    (st.qr_lan_ios, st.btn_lan_copy_setup) = b.form_qr_action(
        st,
        &sec3,
        6,
        "iOS/浏览器：",
        IDC_SET_LAN_QR_IOS,
        "复制入口地址",
        IDC_SET_LAN_COPY_SETUP,
    );
    b.form_label(st, &sec3, 9, "辅助操作：");
    let lan_docs = b.form_button(st, &sec3, 9, "打开扫码绑定页", IDC_SET_LAN_DOCS, lan_btn_w);
    st.btn_lan_docs = b.own_button(st, lan_docs);
}

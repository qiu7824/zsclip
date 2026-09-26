use super::prelude::*;

pub(super) unsafe fn settings_create_cloud_lan_page(
    st: &mut SettingsWndState, b: &SettingsPageBuilder, line_h: i32,
    lan_btn_w: i32, small_btn_w: i32,
) {
    let connection = settings_multi_sync_layout(st, 1, 110);
    st.lb_lan_status = b.label(st, "局域网同步：正在检查", connection.left(), connection.label_y(0,line_h), connection.full_w(),line_h);
    b.form_label(st,&connection,1,"电脑地址：");
    st.lb_lan_addresses = settings_create_lan_address_view(st,b,&connection);
    b.form_label(st,&connection,3,"连接权限：");
    st.lb_lan_firewall = b.form_value_label_auto(st,&connection,3,"正在检查 Windows 防火墙…",settings_scale(70)).0;
    b.form_label(st,&connection,5,"设备名称：");
    st.ed_lan_name = b.form_edit(st,&connection,5,"",IDC_SET_LAN_NAME);

    let pairing = settings_multi_sync_layout(st,2,110);
    b.label(st,"手机发现电脑并连接，本机允许一次即可；无需 QQ 登录。",pairing.left(),pairing.label_y(0,line_h),pairing.full_w(),line_h);
    let actions = [
        ("刷新 / 修复连接",IDC_SET_LAN_REFRESH,lan_btn_w),
        ("允许配对",IDC_SET_LAN_ACCEPT_PAIR,small_btn_w),
        ("拒绝",IDC_SET_LAN_REJECT_PAIR,small_btn_w),
        ("连接选中电脑",IDC_SET_LAN_PAIR,lan_btn_w),
    ];
    let mut x = pairing.left();
    let buttons: Vec<HWND> = actions.iter().map(|(label,id,width)| {
        let button = b.button(st,label,*id,x,pairing.row_y(1),*width);
        x += *width + settings_scale(14);
        b.own_button(st,button)
    }).collect();
    st.btn_lan_refresh = buttons[0];
    st.btn_lan_accept_pair = buttons[1];
    st.btn_lan_reject_pair = buttons[2];
    st.btn_lan_pair = buttons[3];
    st.lb_lan_devices = b.listbox(st,IDC_SET_LAN_DISCOVERED_LIST,pairing.left(),pairing.row_y(2),pairing.full_w(),settings_scale(190));

    let devices = settings_multi_sync_layout(st,3,110);
    b.form_label(st,&devices,0,"信任设备：");
    st.lb_lan_trusted = b.form_value_label_auto(st,&devices,0,"尚无已连接设备。",settings_scale(70)).0;
    b.form_label(st,&devices,2,"自动方向：");
    let mode_label=crate::settings_model::lan_sync_mode_display(&st.draft.lan_sync_mode);
    st.cb_lan_sync_mode=b.form_dropdown(st,&devices,2,mode_label,crate::win_system_params::IDC_SET_LAN_SYNC_MODE,settings_scale(210));
    if !st.cb_lan_sync_mode.is_null() { st.ownerdraw_ctrls.push(st.cb_lan_sync_mode); }
    b.form_label(st,&devices,3,"手机内容：");
    st.cb_lan_receive_mode = b.form_dropdown(st,&devices,3,"只进入记录",IDC_SET_LAN_RECEIVE_MODE,settings_scale(190));
    if !st.cb_lan_receive_mode.is_null() { st.ownerdraw_ctrls.push(st.cb_lan_receive_mode); }
    b.label(st,"仅手动不自动收发，本机仍保存记录；已信任设备可主动发送或拉取。",devices.left(),devices.label_y(4,line_h),devices.full_w(),line_h);
}

unsafe fn settings_create_lan_address_view(
    st: &mut SettingsWndState,
    b: &SettingsPageBuilder,
    sec: &SettingsFormSectionLayout,
) -> HWND {
    use crate::platform::string::to_wide;
    let rect = sec.field_full_rect(1, settings_scale(70));
    let placement = b.control_placement(st, rect.left, rect.top);
    let hwnd = platform_window::create_window_ex(
        WS_EX_CLIENTEDGE, to_wide("EDIT").as_ptr(), to_wide("正在读取本机 IPv4 地址…").as_ptr(),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | ES_MULTILINE as u32 | ES_AUTOVSCROLL as u32 | ES_READONLY as u32,
        placement.x, placement.y, rect.width(), rect.height(), placement.parent,
        null_mut(), platform_window::module_handle(), null(),
    );
    if !hwnd.is_null() {
        platform_window::send_message(hwnd, WM_SETFONT, b.font as WPARAM, 1);
    }
    b.add(st, hwnd, rect.left, rect.top, rect.width(), rect.height())
}

#[cfg(all(test, feature = "lan-sync"))]
mod lan_layout_tests {
    use super::*;
    use crate::platform::string::to_wide;

    #[test]
    fn lan_address_firewall_and_pair_controls_have_separate_visible_rows() {
        unsafe {
            let hwnd = platform_window::create_window_ex(0, to_wide("STATIC").as_ptr(),
                to_wide("LAN geometry").as_ptr(), WS_POPUP, 0, 0, 1080, 800,
                null_mut(), null_mut(), platform_window::module_handle(), null());
            assert!(!hwnd.is_null());
            set_settings_ui_dpi(settings_window_layout_dpi(hwnd));
            let mut state = create_settings_window_state(hwnd, null_mut());
            platform_window::set_user_data(hwnd, (&mut *state as *mut SettingsWndState) as isize);
            for mode in ["lan", "webdav", "off"] {
            settings_apply_multi_sync_mode(&mut state.draft,mode);
            if state.ui.is_built(SettingsPage::Cloud.index()) {settings_rebuild_cloud_page(hwnd,&mut state);}
            else {settings_create_cloud_page(hwnd,&mut state);}
            if mode=="lan" {
                assert_eq!(settings_host_text(state.cb_multi_sync_mode), "局域网");
                assert!(!state.lb_lan_addresses.is_null());
                assert!(!state.lb_lan_firewall.is_null());
                assert!(!state.cb_lan_sync_mode.is_null());
            }
            assert!(!state.lb_qq_cloud_status.is_null());
            assert!(!state.btn_qq_cloud_connect.is_null());
            assert!(!state.chk_qq_cloud_menu.is_null());
            assert!(state.btn_lan_docs.is_null());
            let controls = state.ui.page_regs(SettingsPage::Cloud.index()).collect::<Vec<_>>();
            for (i, a) in controls.iter().enumerate() {
                for b in controls.iter().skip(i + 1) {
                    assert!(!(a.bounds.left < b.bounds.right && b.bounds.left < a.bounds.right
                        && a.bounds.top < b.bounds.bottom && b.bounds.top < a.bounds.bottom),
                        "controls overlap: {} {:?} / {} {:?}", settings_host_text(a.hwnd), a.bounds, settings_host_text(b.hwnd), b.bounds);
                }
            }
            drop(controls);
            }
            platform_window::set_user_data(hwnd, Box::into_raw(state) as isize);
            handle_settings_destroy(hwnd);
            platform_window::destroy(hwnd);
        }
    }
}

use super::prelude::*;

pub(super) unsafe fn open_settings_cloud_dropdown(
    hwnd: HWND,
    st: &mut SettingsWndState,
    control_id: isize,
) -> bool {
    match control_id {
        IDC_SET_CLOUD_INTERVAL => {
            let rc = settings_control_screen_rect_or_empty(st.cb_cloud_interval);
            let items = ["15分钟", "30分钟", "1小时", "6小时", "12小时", "24小时"];
            let current = items
                .iter()
                .position(|x| *x == settings_host_text(st.cb_cloud_interval))
                .unwrap_or(2);
            st.dropdown_popup = present_settings_dropdown_popup(
                hwnd,
                IDC_SET_CLOUD_INTERVAL,
                &rc,
                &items,
                current,
                180,
            );
            true
        }
        IDC_SET_MULTI_SYNC_MODE => {
            let rc = settings_control_screen_rect_or_empty(st.cb_multi_sync_mode);
            let current_label = settings_host_text(st.cb_multi_sync_mode);
            let current = MULTI_SYNC_MODE_OPTIONS
                .iter()
                .position(|x| *x == current_label)
                .unwrap_or(0);
            st.dropdown_popup = present_settings_dropdown_popup(
                hwnd,
                IDC_SET_MULTI_SYNC_MODE,
                &rc,
                &MULTI_SYNC_MODE_OPTIONS,
                current,
                180,
            );
            true
        }
        IDC_SET_LAN_RECEIVE_MODE => {
            let rc = settings_control_screen_rect_or_empty(st.cb_lan_receive_mode);
            let items = ["只进入记录", "直接覆盖剪贴板"];
            let current = items
                .iter()
                .position(|x| *x == settings_host_text(st.cb_lan_receive_mode))
                .unwrap_or(0);
            st.dropdown_popup = present_settings_dropdown_popup(
                hwnd,
                IDC_SET_LAN_RECEIVE_MODE,
                &rc,
                &items,
                current,
                220,
            );
            true
        }
        _ => false,
    }
}

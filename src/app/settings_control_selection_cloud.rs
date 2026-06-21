use super::prelude::*;

pub(super) unsafe fn handle_settings_cloud_selection(
    hwnd: HWND,
    st: &mut SettingsWndState,
    control_id: isize,
    index: usize,
) {
    match control_id {
        IDC_SET_CLOUD_INTERVAL => {
            let items = ["15分钟", "30分钟", "1小时", "6小时", "12小时", "24小时"];
            if let Some(label) = items.get(index) {
                settings_set_text(st.cb_cloud_interval, label);
                repaint_settings_control(st.cb_cloud_interval);
            }
        }
        IDC_SET_MULTI_SYNC_MODE => {
            if let Some(label) = MULTI_SYNC_MODE_OPTIONS.get(index) {
                settings_set_text(st.cb_multi_sync_mode, label);
                settings_collect_to_app(st);
                settings_rebuild_cloud_page(hwnd, st);
            }
        }
        IDC_SET_LAN_RECEIVE_MODE => {
            let items = ["只进入记录", "直接覆盖剪贴板"];
            if let Some(label) = items.get(index) {
                settings_set_text(st.cb_lan_receive_mode, label);
                st.draft.lan_receive_mode = lan_receive_mode_from_label(label).to_string();
                repaint_settings_control(st.cb_lan_receive_mode);
            }
        }
        _ => {}
    }
}

use super::prelude::*;
use crate::platform::string::to_wide;

pub(super) unsafe fn settings_lan_refresh_lists(st: &mut SettingsWndState) {
    if st.lb_lan_devices.is_null() {
        return;
    }

    let selected = platform_window::send_message(st.lb_lan_devices, LB_GETCURSEL, 0, 0) as i32;
    platform_window::send_message(st.lb_lan_devices, LB_RESETCONTENT, 0, 0);
    st.lan_pending_cache = crate::lan_sync::pending_pair_requests();
    st.lan_discovered_cache = crate::lan_sync::discovered_devices();
    let total_rows = st.lan_pending_cache.len() + st.lan_discovered_cache.len();
    if total_rows == 0 {
        platform_window::send_message(
            st.lb_lan_devices,
            LB_ADDSTRING,
            0,
            to_wide("暂无附近设备或待允许请求，可刷新或输入手动 IP").as_ptr() as LPARAM,
        );
    } else {
        for pair in &st.lan_pending_cache {
            let age = crate::lan_sync::now_ms_public().saturating_sub(pair.created_at_ms) / 1000;
            let text = format!(
                "[待允许] {}   {}   {}秒前   安全码 {}",
                pair.device_name, pair.addr, age, pair.code
            );
            platform_window::send_message(
                st.lb_lan_devices,
                LB_ADDSTRING,
                0,
                to_wide(&text).as_ptr() as LPARAM,
            );
        }
        for device in &st.lan_discovered_cache {
            let text = format!(
                "[设备] {}   {}:{}   {}",
                device.name,
                device.addr,
                device.tcp_port,
                if device.trusted {
                    "已信任"
                } else {
                    "未配对"
                }
            );
            platform_window::send_message(
                st.lb_lan_devices,
                LB_ADDSTRING,
                0,
                to_wide(&text).as_ptr() as LPARAM,
            );
        }
        let select = selected.max(0).min(total_rows.saturating_sub(1) as i32);
        platform_window::send_message(st.lb_lan_devices, LB_SETCURSEL, select as WPARAM, 0);
    }
    platform_window::show_scrollbar(st.lb_lan_devices, SB_HORZ, false);
}

pub(super) unsafe fn settings_lan_selected_device(
    st: &SettingsWndState,
) -> Option<crate::lan_sync::LanDevice> {
    let row = settings_lan_selected_row(st)?;
    if row < st.lan_pending_cache.len() {
        return None;
    }
    st.lan_discovered_cache
        .get(row - st.lan_pending_cache.len())
        .cloned()
}

pub(super) unsafe fn settings_lan_selected_pair(
    st: &SettingsWndState,
) -> Option<crate::lan_sync::LanPairPrompt> {
    let row = settings_lan_selected_row(st)?;
    st.lan_pending_cache.get(row).cloned()
}

unsafe fn settings_lan_selected_row(st: &SettingsWndState) -> Option<usize> {
    if st.lb_lan_devices.is_null() {
        return None;
    }
    let row = platform_window::send_message(st.lb_lan_devices, LB_GETCURSEL, 0, 0) as i32;
    (row >= 0).then_some(row as usize)
}

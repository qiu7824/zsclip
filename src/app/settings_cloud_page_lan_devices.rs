use super::prelude::*;
use crate::platform::string::to_wide;

fn same_pair_rows(
    left: &[crate::lan_sync::LanPairPrompt],
    right: &[crate::lan_sync::LanPairPrompt],
) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            left.pair_id == right.pair_id
                && left.code == right.code
                && left.device_name == right.device_name
                && left.addr == right.addr
        })
}

fn same_device_rows(
    left: &[crate::lan_sync::LanDevice],
    right: &[crate::lan_sync::LanDevice],
) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            left.device_id == right.device_id
                && left.name == right.name
                && left.addr == right.addr
                && left.tcp_port == right.tcp_port
                && left.trusted == right.trusted
        })
}

pub(super) unsafe fn settings_lan_refresh_lists(st: &mut SettingsWndState) -> bool {
    if st.lb_lan_devices.is_null() {
        return false;
    }

    let pending = crate::lan_sync::pending_pair_requests();
    let mut discovered = crate::lan_sync::discovered_devices();
    discovered.sort_by(|left, right| {
        right
            .trusted
            .cmp(&left.trusted)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.device_id.cmp(&right.device_id))
    });
    let rows_changed = !same_pair_rows(&st.lan_pending_cache, &pending)
        || !same_device_rows(&st.lan_discovered_cache, &discovered)
        || platform_window::send_message(st.lb_lan_devices, LB_GETCOUNT, 0, 0) <= 0;
    st.lan_pending_cache = pending;
    st.lan_discovered_cache = discovered;
    if !rows_changed {
        return false;
    }

    let selected = platform_window::send_message(st.lb_lan_devices, LB_GETCURSEL, 0, 0) as i32;
    platform_window::send_message(st.lb_lan_devices, LB_RESETCONTENT, 0, 0);
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
            let text = format!(
                "[待允许] {}   {}   安全码 {}",
                pair.device_name, pair.addr, pair.code
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
    true
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

#[cfg(test)]
mod tests {
    use super::*;

    fn device(last_seen_ms: u64) -> crate::lan_sync::LanDevice {
        crate::lan_sync::LanDevice {
            device_id: "device-1".to_string(),
            name: "Phone".to_string(),
            addr: "192.168.1.20".to_string(),
            tcp_port: 38473,
            token: "token".to_string(),
            last_seen_ms,
            trusted: true,
            capabilities: vec!["text".to_string()],
        }
    }

    fn pair(created_at_ms: u64) -> crate::lan_sync::LanPairPrompt {
        crate::lan_sync::LanPairPrompt {
            pair_id: "pair-1".to_string(),
            code: "123456".to_string(),
            device_name: "Phone".to_string(),
            addr: "192.168.1.20".to_string(),
            created_at_ms,
        }
    }

    #[test]
    fn repeated_discovery_heartbeat_does_not_rebuild_device_rows() {
        let original = vec![device(1_000)];
        let heartbeat = vec![device(6_000)];

        assert!(same_device_rows(&original, &heartbeat));

        let mut renamed = device(6_000);
        renamed.name = "Tablet".to_string();
        assert!(!same_device_rows(&original, &[renamed]));
    }

    #[test]
    fn pair_row_refresh_ignores_age_but_detects_actionable_changes() {
        let original = vec![pair(1_000)];
        let older = vec![pair(6_000)];

        assert!(same_pair_rows(&original, &older));

        let mut changed = pair(6_000);
        changed.code = "654321".to_string();
        assert!(!same_pair_rows(&original, &[changed]));
    }
}

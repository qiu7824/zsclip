use super::prelude::*;
use crate::platform::string::to_wide;
use crate::settings_model::settings_qr_raster_layout;
use crate::win_ui_render::{DT_CENTER, DT_VCENTER, DT_WORDBREAK};

pub(super) fn settings_lan_qr_payload(st: &SettingsWndState, cid: isize) -> Option<String> {
    if !st.draft.lan_sync_enabled || !crate::lan_sync::lan_service_ready() {
        return None;
    }
    match cid {
        IDC_SET_LAN_QR_ANDROID => crate::lan_sync::mobile_pair_url_cached(&st.draft),
        IDC_SET_LAN_QR_IOS => crate::lan_sync::mobile_setup_url_cached(&st.draft),
        _ => None,
    }
}

pub(super) fn settings_qr_cache_for_payload<'a>(
    slot: &'a mut Option<SettingsQrCache>,
    payload: &str,
) -> Option<&'a SettingsQrCache> {
    let should_rebuild = slot
        .as_ref()
        .map(|cache| cache.payload != payload)
        .unwrap_or(true);
    if should_rebuild {
        *slot = Some(crate::settings_model::settings_qr_cache_for_payload(
            payload,
        )?);
    }
    slot.as_ref()
}

pub(super) fn prepare_settings_lan_qr_caches(st: &mut SettingsWndState) {
    let android_payload = settings_lan_qr_payload(st, IDC_SET_LAN_QR_ANDROID);
    let ios_payload = settings_lan_qr_payload(st, IDC_SET_LAN_QR_IOS);
    if let Some(payload) = android_payload {
        let _ = settings_qr_cache_for_payload(&mut st.qr_lan_android_cache, &payload);
    }
    if let Some(payload) = ios_payload {
        let _ = settings_qr_cache_for_payload(&mut st.qr_lan_ios_cache, &payload);
    }
}

pub(super) unsafe fn draw_settings_qr_payload(
    hdc: *mut core::ffi::c_void,
    rc: RECT,
    qr: &SettingsQrCache,
    th: Theme,
) {
    let bg = platform_gdi::create_solid_brush(th.control_bg);
    platform_gdi::fill_rect(hdc, &rc, bg);
    platform_gdi::delete_object(bg as _);
    let frame = RECT {
        left: rc.left,
        top: rc.top,
        right: rc.right,
        bottom: rc.bottom,
    };
    draw_round_rect(
        hdc as _,
        &frame,
        th.control_bg,
        th.stroke,
        settings_scale(8),
    );

    let border_modules = 4;
    let Some((white_rect, module_size)) =
        settings_qr_raster_layout(rc.into(), qr.size, border_modules, settings_scale(16))
    else {
        return;
    };
    let white_br = platform_gdi::create_solid_brush(rgb(255, 255, 255));
    let white: RECT = white_rect.into();
    platform_gdi::fill_rect(hdc, &white, white_br);
    platform_gdi::delete_object(white_br as _);
    let qr_side = qr.size * module_size;
    let qr_left = white.left + border_modules * module_size;
    let qr_top = white.top + border_modules * module_size;
    platform_gdi::stretch_top_down_32bpp_nearest(
        hdc,
        qr_left,
        qr_top,
        qr_side,
        qr_side,
        qr.size,
        qr.size,
        &qr.bgra_pixels,
    );
}

pub(super) unsafe fn draw_settings_qr_slot(
    st: &SettingsWndState,
    hdc: *mut core::ffi::c_void,
    rc: RECT,
    cid: isize,
) {
    let th = Theme::default();
    let payload = settings_lan_qr_payload(st, cid);
    if let Some(payload) = payload.as_deref() {
        let cache = match cid {
            IDC_SET_LAN_QR_ANDROID => st.qr_lan_android_cache.as_ref(),
            IDC_SET_LAN_QR_IOS => st.qr_lan_ios_cache.as_ref(),
            _ => None,
        }
        .filter(|cache| cache.payload == payload);
        if let Some(cache) = cache {
            draw_settings_qr_payload(hdc, rc, cache, th);
            return;
        }
    }

    let bg = platform_gdi::create_solid_brush(th.control_bg);
    platform_gdi::fill_rect(hdc, &rc, bg);
    platform_gdi::delete_object(bg as _);
    draw_round_rect(hdc as _, &rc, th.control_bg, th.stroke, settings_scale(8));
    let mut text_rc = rc;
    text_rc.left += settings_scale(10);
    text_rc.right -= settings_scale(10);
    let msg = if payload.is_some()
        || (st.draft.lan_sync_enabled && crate::lan_sync::lan_service_ready())
    {
        to_wide(tr("正在准备二维码...", "Preparing QR code..."))
    } else {
        to_wide(tr("保存后生成二维码", "Save to generate QR"))
    };
    platform_gdi::set_bk_mode(hdc, 1);
    platform_gdi::set_text_color(hdc, th.text_muted);
    platform_gdi::draw_text(
        hdc,
        msg.as_ptr(),
        -1,
        &mut text_rc,
        DT_CENTER | DT_VCENTER | DT_WORDBREAK,
    );
}

pub(super) unsafe fn draw_settings_lan_qr_blocks(
    st: &SettingsWndState,
    hdc: *mut core::ffi::c_void,
    scroll_y: i32,
    viewport: RECT,
) {
    if st.cur_page != SettingsPage::Cloud.index() {
        return;
    }
    for (bounds, cid) in [
        (st.qr_lan_android_bounds, IDC_SET_LAN_QR_ANDROID),
        (st.qr_lan_ios_bounds, IDC_SET_LAN_QR_IOS),
    ] {
        if bounds.width() <= 0 || bounds.height() <= 0 {
            continue;
        }
        let rc = RECT {
            left: bounds.left,
            top: bounds.top - scroll_y,
            right: bounds.right,
            bottom: bounds.bottom - scroll_y,
        };
        if settings_child_visible(rc.top, rc.bottom - rc.top, &viewport) {
            draw_settings_qr_slot(st, hdc, rc, cid);
        }
    }
}

pub(super) unsafe fn draw_settings_qr_item(st: &mut SettingsWndState, dis: &DRAWITEMSTRUCT) {
    draw_settings_qr_slot(st, dis.hDC, dis.rcItem, dis.CtlID as isize);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qr_payload_cache_reuses_same_payload_and_rebuilds_on_change() {
        let mut slot = None;
        let first_runs = {
            let cache = settings_qr_cache_for_payload(&mut slot, "http://127.0.0.1:38473/a")
                .expect("first qr");
            assert_eq!(cache.payload, "http://127.0.0.1:38473/a");
            assert!(!cache.runs.is_empty());
            assert!(cache.runs.iter().all(|run| run.len > 0));
            cache.runs.clone()
        };

        let same =
            settings_qr_cache_for_payload(&mut slot, "http://127.0.0.1:38473/a").expect("same qr");
        assert_eq!(same.runs, first_runs);

        let changed = settings_qr_cache_for_payload(&mut slot, "http://127.0.0.1:38473/b")
            .expect("changed qr");
        assert_eq!(changed.payload, "http://127.0.0.1:38473/b");
        assert_ne!(changed.runs, first_runs);
    }
}

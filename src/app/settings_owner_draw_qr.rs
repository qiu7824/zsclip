use super::prelude::*;
use crate::platform::string::to_wide;
use crate::win_ui_render::{DT_CENTER, DT_SINGLELINE, DT_VCENTER, DT_WORDBREAK};

pub(super) fn settings_lan_qr_payload(st: &SettingsWndState, cid: isize) -> Option<String> {
    if !st.draft.lan_sync_enabled || !crate::lan_sync::lan_service_ready() {
        return None;
    }
    match cid {
        IDC_SET_LAN_QR_ANDROID => crate::lan_sync::mobile_pair_url(&st.draft),
        IDC_SET_LAN_QR_IOS => crate::lan_sync::mobile_setup_url(&st.draft),
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

    let Some(plan) = settings_qr_render_plan(rc.into(), qr, 4, settings_scale(16)) else {
        return;
    };
    let white_br = platform_gdi::create_solid_brush(rgb(255, 255, 255));
    let white: RECT = plan.white_rect.into();
    platform_gdi::fill_rect(hdc, &white, white_br);
    platform_gdi::delete_object(white_br as _);
    let black_br = platform_gdi::create_solid_brush(rgb(17, 17, 17));
    for rect in plan.module_rects {
        let mrc: RECT = rect.into();
        platform_gdi::fill_rect(hdc, &mrc, black_br);
    }
    platform_gdi::delete_object(black_br as _);
}

pub(super) unsafe fn draw_settings_qr_item(st: &mut SettingsWndState, dis: &DRAWITEMSTRUCT) {
    let th = Theme::default();
    let hdc = dis.hDC;
    let rc = dis.rcItem;
    let cid = dis.CtlID as isize;
    if let Some(payload) = settings_lan_qr_payload(st, cid) {
        let cache = match cid {
            IDC_SET_LAN_QR_ANDROID => {
                settings_qr_cache_for_payload(&mut st.qr_lan_android_cache, &payload)
            }
            IDC_SET_LAN_QR_IOS => settings_qr_cache_for_payload(&mut st.qr_lan_ios_cache, &payload),
            _ => None,
        };
        if let Some(cache) = cache {
            draw_settings_qr_payload(hdc, rc, cache, th);
            return;
        }
        let mut text_rc = rc;
        let msg = to_wide(tr("二维码生成失败", "QR failed"));
        platform_gdi::set_bk_mode(hdc, 1);
        platform_gdi::set_text_color(hdc, th.text_muted);
        platform_gdi::draw_text(
            hdc,
            msg.as_ptr(),
            -1,
            &mut text_rc,
            DT_SINGLELINE | DT_VCENTER | DT_CENTER,
        );
    } else {
        let bg = platform_gdi::create_solid_brush(th.control_bg);
        platform_gdi::fill_rect(hdc, &rc, bg);
        platform_gdi::delete_object(bg as _);
        draw_round_rect(hdc as _, &rc, th.control_bg, th.stroke, settings_scale(8));
        let mut text_rc = rc;
        text_rc.left += settings_scale(10);
        text_rc.right -= settings_scale(10);
        let msg = to_wide(tr("保存后生成二维码", "Save to generate QR"));
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

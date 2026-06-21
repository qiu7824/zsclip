use super::prelude::*;
use crate::platform::input as platform_input;
use crate::win_system_ui::{draw_settings_button_component, draw_settings_toggle_component};

pub(super) unsafe fn settings_button_hover(st: &SettingsWndState, hwnd_item: HWND) -> bool {
    if hwnd_item.is_null() {
        return false;
    }
    let Some(pt) = platform_input::cursor_pos() else {
        return false;
    };
    let Some(rc) = platform_window::window_rect(hwnd_item) else {
        return false;
    };
    pt.x >= rc.left
        && pt.x < rc.right
        && pt.y >= rc.top
        && pt.y < rc.bottom
        && st.hot_ownerdraw == hwnd_item
}

pub(super) unsafe fn settings_draw_button_item(st: &mut SettingsWndState, dis: &DRAWITEMSTRUCT) {
    let th = Theme::default();
    let hdc = dis.hDC;
    let rc = dis.rcItem;
    let cid = dis.CtlID as isize;
    let pressed = (dis.itemState & ODS_SELECTED) != 0;
    let hover = settings_button_hover(st, dis.hwndItem);
    let text = settings_host_text(dis.hwndItem);

    if settings_owner_draw_is_qr(cid) {
        draw_settings_qr_item(st, dis);
        return;
    }

    if settings_owner_draw_is_toggle(cid) {
        let checked = settings_toggle_get(st, cid);
        draw_settings_toggle_component(hdc as _, &rc, hover, checked, th);
        return;
    }

    if draw_settings_source_link_item(dis, &text, hover, pressed, th) {
        return;
    }

    let kind = settings_owner_draw_button_kind(st, cid);
    draw_settings_button_component(hdc as _, &rc, &text, kind, hover, pressed, th);
}

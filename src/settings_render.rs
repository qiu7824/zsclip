use windows_sys::Win32::Foundation::RECT;

use crate::settings_model::{settings_cards_for_page_vec, settings_page_scrollable, settings_title_rect, SettingsSection};
use crate::ui::{
    draw_round_fill, draw_round_rect, draw_text_ex, rgb, settings_nav_item_rect, settings_nav_w_scaled,
    settings_scale, Theme, SETTINGS_NAV_GLYPHS, SETTINGS_PAGES,
};

pub const SETTINGS_CLASS: &str = "ZsClipSettings";
pub const IDC_SET_SAVE: isize = 5003;
pub const IDC_SET_CLOSE: isize = 5004;
pub const IDC_SET_AUTOSTART: isize = 5010;
pub const IDC_SET_CLOSETRAY: isize = 5011;
pub const IDC_SET_CLICK_HIDE: isize = 5038;
pub const IDC_SET_EDGEHIDE: isize = 5013;
pub const IDC_SET_HOVERPREVIEW: isize = 5014;
pub const IDC_SET_MAX: isize = 5015;
pub const IDC_SET_POSMODE: isize = 5016;
pub const IDC_SET_DX: isize = 5017;
pub const IDC_SET_DY: isize = 5018;
pub const IDC_SET_FX: isize = 5019;
pub const IDC_SET_FY: isize = 5020;
pub const IDC_SET_BTN_OPENCFG: isize = 5021;
pub const IDC_SET_BTN_OPENDB: isize = 5022;
pub const IDC_SET_BTN_OPENDATA: isize = 5023;
pub const IDC_SET_GROUP_ENABLE: isize = 5030;
pub const IDC_SET_GROUP_LIST: isize = 5032;
pub const IDC_SET_GROUP_ADD: isize = 5033;
pub const IDC_SET_GROUP_RENAME: isize = 5034;
pub const IDC_SET_GROUP_DELETE: isize = 5035;
pub const IDC_SET_GROUP_UP: isize = 5036;
pub const IDC_SET_GROUP_DOWN: isize = 5037;
pub const IDC_SET_CLOUD_ENABLE: isize = 5040;
pub const IDC_SET_CLOUD_INTERVAL: isize = 5041;
pub const IDC_SET_CLOUD_URL: isize = 5042;
pub const IDC_SET_CLOUD_USER: isize = 5043;
pub const IDC_SET_CLOUD_PASS: isize = 5044;
pub const IDC_SET_CLOUD_DIR: isize = 5045;
pub const IDC_SET_CLOUD_SYNC_NOW: isize = 5046;
pub const IDC_SET_CLOUD_UPLOAD_CFG: isize = 5047;
pub const IDC_SET_CLOUD_APPLY_CFG: isize = 5048;
pub const IDC_SET_CLOUD_RESTORE_BACKUP: isize = 5049;
pub const IDC_SET_PLUGIN_MAILMERGE: isize = 5050;
pub const IDC_SET_IMAGE_PREVIEW: isize = 5051;
pub const IDC_SET_QUICK_DELETE: isize = 5052;
pub const IDC_SET_OPEN_SOURCE: isize = 5053;
pub const IDC_SET_VV_MODE: isize = 5054;
pub const IDC_SET_VV_GROUP: isize = 5055;
pub const IDC_SET_VV_SOURCE: isize = 5056;
pub const IDC_SET_GROUP_VIEW_RECORDS: isize = 5057;
pub const IDC_SET_GROUP_VIEW_PHRASES: isize = 5058;
pub const IDC_SET_SILENTSTART: isize = 5059;
pub const IDC_SET_TRAYICON: isize = 5060;
pub const IDC_SET_AUTOHIDE_BLUR: isize = 5061;
pub const IDC_SET_OPEN_UPDATE: isize = 5062;
pub const IDC_SET_PASTE_MOVE_TOP: isize = 5063;
pub const IDC_SET_DEDUPE_FILTER: isize = 5064;
pub const IDC_SET_HK_RECORD: isize = 6104;

unsafe fn draw_settings_card(hdc: *mut core::ffi::c_void, section: &SettingsSection, scroll_y: i32, th: Theme) {
    let rc: RECT = section.rect.offset_y(scroll_y).into();
    draw_round_rect(hdc, &rc, th.surface, th.stroke, settings_scale(8));
    let trc = RECT {
        left: rc.left + settings_scale(16),
        top: rc.top + settings_scale(12),
        right: rc.right - settings_scale(16),
        bottom: rc.top + settings_scale(34),
    };
    draw_text_ex(
        hdc,
        section.title,
        &trc,
        th.text_muted,
        settings_scale(12),
        true,
        false,
        "Segoe UI Variable Text",
    );
}

pub unsafe fn draw_settings_nav_item(
    hdc: *mut core::ffi::c_void,
    index: usize,
    selected: bool,
    hover: bool,
    th: Theme,
) {
    let item_rc = settings_nav_item_rect(index);
    if selected {
        draw_round_fill(hdc, &item_rc, th.nav_sel_fill, settings_scale(6));
        let bar_h = settings_scale(16);
        let bar_cy = (item_rc.top + item_rc.bottom) / 2;
        let bar = RECT {
            left: item_rc.left + settings_scale(3),
            top: bar_cy - bar_h / 2,
            right: item_rc.left + settings_scale(6),
            bottom: bar_cy + bar_h / 2,
        };
        draw_round_fill(hdc, &bar, th.accent, settings_scale(2));
    } else if hover {
        let hover_color = if th.bg == rgb(32, 32, 32) { rgb(60, 60, 60) } else { rgb(237, 237, 237) };
        draw_round_fill(hdc, &item_rc, hover_color, settings_scale(6));
    }
    let icon_rc = RECT {
        left: item_rc.left + settings_scale(10),
        top: item_rc.top,
        right: item_rc.left + settings_scale(38),
        bottom: item_rc.bottom,
    };
    let txt_rc = RECT {
        left: item_rc.left + settings_scale(40),
        top: item_rc.top,
        right: item_rc.right - settings_scale(8),
        bottom: item_rc.bottom,
    };
    let icon_color = if selected { th.accent } else if hover { th.text } else { th.text_muted };
    draw_text_ex(
        hdc,
        SETTINGS_NAV_GLYPHS[index],
        &icon_rc,
        icon_color,
        settings_scale(16),
        false,
        false,
        "Segoe Fluent Icons",
    );
    let label_color = if selected || hover { th.text } else { th.text_muted };
    draw_text_ex(
        hdc,
        SETTINGS_PAGES[index],
        &txt_rc,
        label_color,
        settings_scale(14),
        false,
        false,
        "Segoe UI Variable Text",
    );
}

pub unsafe fn draw_settings_page_cards(hdc: *mut core::ffi::c_void, page: usize, scroll_y: i32, th: Theme) {
    let effective_scroll = if settings_page_scrollable(page) { scroll_y } else { 0 };
    for section in settings_cards_for_page_vec(page) {
        draw_settings_card(hdc, &section, effective_scroll, th);
    }
}

pub fn settings_title_rect_win() -> RECT {
    settings_title_rect().into()
}

pub fn nav_divider_x() -> i32 {
    settings_nav_w_scaled()
}

pub unsafe fn draw_settings_page_content(_hdc: *mut core::ffi::c_void, _page: usize, _th: Theme) {}

use windows_sys::Win32::Foundation::RECT;
use crate::settings_model::{settings_cards_for_page, settings_page_scrollable, SettingsSection};
use crate::ui::{draw_round_fill, draw_round_rect, draw_text_ex, rgb, settings_nav_item_rect, Theme, SETTINGS_NAV_GLYPHS, SETTINGS_PAGES, SETTINGS_NAV_W};

pub const SETTINGS_CLASS: &str = "ZsClipSettings";
pub const IDC_SET_SAVE: isize = 5003;
pub const IDC_SET_CLOSE: isize = 5004;
pub const IDC_SET_AUTOSTART: isize = 5010;
pub const IDC_SET_CLOSETRAY: isize = 5011;
pub const IDC_SET_CLICK_HIDE: isize = 5038;  // 单击后是否隐藏主窗口
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

#[derive(Clone, Copy)]
pub struct SettingsSection {
    pub title: &'static str,
    pub rect: RECT,
}

#[derive(Clone, Copy)]
pub struct SettingsPageSchema {
    pub sections: &'static [SettingsSection],
    pub scrollable: bool,
}

const GENERAL_SECTIONS: [SettingsSection; 4] = [
    SettingsSection { title: "启动与显示", rect: settings_card_rect(CARD_GENERAL_Y, CARD_GENERAL_H) },
    SettingsSection { title: "数据",       rect: settings_card_rect(CARD_DATA_Y, CARD_DATA_H) },
    SettingsSection { title: "显示位置",   rect: settings_card_rect(CARD_POSITION_Y, CARD_POSITION_H) },
    SettingsSection { title: "维护",       rect: settings_card_rect(CARD_MAINTAIN_Y, CARD_MAINTAIN_H) },
];

const HOTKEY_SECTIONS: [SettingsSection; 3] = [
    SettingsSection { title: "主快捷键",               rect: settings_card_rect(16, 150) },
    SettingsSection { title: "系统剪贴板历史（Win+V）", rect: settings_card_rect(184, 132) },
    SettingsSection { title: "功能说明",               rect: settings_card_rect(356, 112) },
];

const PLUGIN_SECTIONS: [SettingsSection; 3] = [
    SettingsSection { title: "搜索插件",     rect: settings_card_rect(16, 260) },
    SettingsSection { title: "AI 文本清洗",  rect: settings_card_rect(300, 130) },
    SettingsSection { title: "超级邮件合并", rect: settings_card_rect(444, 110) },
];

const GROUP_SECTIONS: [SettingsSection; 2] = [
    SettingsSection { title: "分组功能", rect: settings_card_rect(16, 88) },
    SettingsSection { title: "分组管理", rect: settings_card_rect(124, 474) },
];

const CLOUD_SECTIONS: [SettingsSection; 3] = [
    SettingsSection { title: "云同步",     rect: settings_card_rect(16, 112) },
    SettingsSection { title: "连接与凭据", rect: settings_card_rect(140, 146) },
    SettingsSection { title: "操作",       rect: settings_card_rect(298, 92) },
];

const ABOUT_SECTIONS: [SettingsSection; 1] = [
    SettingsSection { title: "关于", rect: settings_card_rect(16, 220) },
];

const SETTINGS_PAGE_SCHEMAS: [SettingsPageSchema; 6] = [
    SettingsPageSchema { sections: &GENERAL_SECTIONS, scrollable: true },
    SettingsPageSchema { sections: &HOTKEY_SECTIONS, scrollable: false },
    SettingsPageSchema { sections: &PLUGIN_SECTIONS, scrollable: false },
    SettingsPageSchema { sections: &GROUP_SECTIONS, scrollable: false },
    SettingsPageSchema { sections: &CLOUD_SECTIONS, scrollable: false },
    SettingsPageSchema { sections: &ABOUT_SECTIONS, scrollable: false },
];

pub fn settings_title_rect() -> RECT {
    RECT { left: SETTINGS_NAV_W + 36, top: 32, right: SETTINGS_NAV_W + 360, bottom: 62 }
}

#[inline]
pub fn settings_page_schema(page: usize) -> &'static SettingsPageSchema {
    SETTINGS_PAGE_SCHEMAS.get(page).unwrap_or(&SETTINGS_PAGE_SCHEMAS[0])
}

#[inline]
pub fn settings_page_scrollable(page: usize) -> bool {
    settings_page_schema(page).scrollable
}

#[inline]
fn offset_rect(rc: RECT, dy: i32) -> RECT {
    RECT { left: rc.left, top: rc.top - dy, right: rc.right, bottom: rc.bottom - dy }
}

unsafe fn draw_settings_card(hdc: *mut core::ffi::c_void, section: &SettingsSection, scroll_y: i32, th: Theme) {
    let rc = offset_rect(section.rect, scroll_y);
    draw_round_rect(hdc, &rc, th.surface, th.stroke, 8);
    let trc = RECT { left: rc.left + 16, top: rc.top + 12, right: rc.right - 16, bottom: rc.top + 34 };
    draw_text_ex(hdc, section.title, &trc, th.text_muted, 12, true, false, "Segoe UI Variable Text");
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
        draw_round_fill(hdc, &item_rc, th.nav_sel_fill, 6);
        let bar_h = 16i32;
        let bar_cy = (item_rc.top + item_rc.bottom) / 2;
        let bar = RECT {
            left: item_rc.left + 3,
            top: bar_cy - bar_h / 2,
            right: item_rc.left + 6,
            bottom: bar_cy + bar_h / 2,
        };
        draw_round_fill(hdc, &bar, th.accent, 2);
    } else if hover {
        let hover_color = if th.bg == rgb(32, 32, 32) { rgb(60, 60, 60) } else { rgb(237, 237, 237) };
        draw_round_fill(hdc, &item_rc, hover_color, 6);
    }
    let icon_rc = RECT { left: item_rc.left + 10, top: item_rc.top, right: item_rc.left + 38, bottom: item_rc.bottom };
    let txt_rc  = RECT { left: item_rc.left + 40, top: item_rc.top, right: item_rc.right - 8, bottom: item_rc.bottom };
    let icon_color = if selected { th.accent } else if hover { th.text } else { th.text_muted };
    draw_text_ex(hdc, SETTINGS_NAV_GLYPHS[index], &icon_rc, icon_color, 16, false, false, "Segoe Fluent Icons");
    let label_color = if selected { th.text } else if hover { th.text } else { th.text_muted };
    draw_text_ex(hdc, SETTINGS_PAGES[index], &txt_rc, label_color, 14, false, false, "Segoe UI Variable Text");
}

pub fn settings_cards_for_page(page: usize) -> &'static [SettingsSection] {
    settings_page_schema(page).sections
}

pub unsafe fn draw_settings_page_cards(hdc: *mut core::ffi::c_void, page: usize, scroll_y: i32, th: Theme) {
    let effective_scroll = if settings_page_scrollable(page) { scroll_y } else { 0 };
    for section in settings_cards_for_page(page) {
        draw_settings_card(hdc, section, effective_scroll, th);
    }
}

pub fn nav_divider_x() -> i32 { SETTINGS_NAV_W }

pub unsafe fn draw_settings_page_content(_hdc: *mut core::ffi::c_void, _page: usize, _th: Theme) {
    // 当前设置页内容以真实子控件为主，这里不再额外叠加说明文字，
    // 避免与子控件产生重叠和错位。
}

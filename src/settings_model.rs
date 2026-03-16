use crate::settings_layout::settings_card_rect;
use crate::ui::{SETTINGS_CONTENT_Y, SETTINGS_NAV_W};
use crate::ui_core::UiRect;

pub const SETTINGS_FORM_HEADER_H: i32 = 52;
pub const SETTINGS_FORM_ROW_H: i32 = 32;
pub const SETTINGS_FORM_ROW_GAP: i32 = 8;
pub const SETTINGS_FORM_SECTION_GAP: i32 = 12;
pub const SETTINGS_FORM_SECTION_PAD: i32 = 18;

#[derive(Clone, Copy)]
pub struct SettingsSection {
    pub title: &'static str,
    pub rect: UiRect,
}

#[derive(Clone, Copy)]
pub struct SettingsPageSchema {
    pub sections: &'static [SettingsSection],
    pub scrollable: bool,
}

#[derive(Clone, Copy)]
pub struct SettingsFormCardSpec {
    pub rows: i32,
}

const GENERAL_SECTIONS: [SettingsSection; 4] = [
    SettingsSection { title: "启动与显示", rect: settings_card_rect(crate::settings_layout::CARD_GENERAL_Y, crate::settings_layout::CARD_GENERAL_H) },
    SettingsSection { title: "数据", rect: settings_card_rect(crate::settings_layout::CARD_DATA_Y, crate::settings_layout::CARD_DATA_H) },
    SettingsSection { title: "显示位置", rect: settings_card_rect(crate::settings_layout::CARD_POSITION_Y, crate::settings_layout::CARD_POSITION_H) },
    SettingsSection { title: "维护", rect: settings_card_rect(crate::settings_layout::CARD_MAINTAIN_Y, crate::settings_layout::CARD_MAINTAIN_H) },
];

const HOTKEY_SECTIONS: [SettingsSection; 3] = [
    SettingsSection { title: "主快捷键", rect: settings_card_rect(16, 150) },
    SettingsSection { title: "系统剪贴板历史（Win+V）", rect: settings_card_rect(184, 132) },
    SettingsSection { title: "功能说明", rect: settings_card_rect(356, 112) },
];

const PLUGIN_SECTIONS: [SettingsSection; 3] = [
    SettingsSection { title: "搜索插件", rect: settings_card_rect(16, 260) },
    SettingsSection { title: "AI 文本清洗", rect: settings_card_rect(300, 130) },
    SettingsSection { title: "超级邮件合并", rect: settings_card_rect(444, 150) },
];

const GROUP_SECTIONS: [SettingsSection; 2] = [
    SettingsSection { title: "分组功能", rect: settings_card_rect(16, 188) },
    SettingsSection { title: "分组管理", rect: settings_card_rect(216, 434) },
];

const CLOUD_SECTIONS: [SettingsSection; 3] = [
    SettingsSection { title: "云同步", rect: settings_card_rect(16, 112) },
    SettingsSection { title: "连接与凭据", rect: settings_card_rect(140, 146) },
    SettingsSection { title: "操作", rect: settings_card_rect(298, 92) },
];

const ABOUT_SECTIONS: [SettingsSection; 1] = [
    SettingsSection { title: "关于", rect: settings_card_rect(16, 280) },
];

const PLUGIN_FORM_SECTIONS: [SettingsFormCardSpec; 3] = [
    SettingsFormCardSpec { rows: 4 },
    SettingsFormCardSpec { rows: 1 },
    SettingsFormCardSpec { rows: 3 },
];

const CLOUD_FORM_SECTIONS: [SettingsFormCardSpec; 3] = [
    SettingsFormCardSpec { rows: 3 },
    SettingsFormCardSpec { rows: 4 },
    SettingsFormCardSpec { rows: 3 },
];

const HOTKEY_FORM_SECTIONS: [SettingsFormCardSpec; 3] = [
    SettingsFormCardSpec { rows: 3 },
    SettingsFormCardSpec { rows: 2 },
    SettingsFormCardSpec { rows: 2 },
];

fn settings_form_cards(y0: i32, titles: [&'static str; 3], specs: [SettingsFormCardSpec; 3]) -> Vec<SettingsSection> {
    let h0 = settings_form_section_height(specs[0].rows);
    let y1 = y0 + h0 + SETTINGS_FORM_SECTION_GAP;
    let h1 = settings_form_section_height(specs[1].rows);
    let y2 = y1 + h1 + SETTINGS_FORM_SECTION_GAP;
    let h2 = settings_form_section_height(specs[2].rows);
    vec![
        SettingsSection { title: titles[0], rect: settings_card_rect(y0, h0) },
        SettingsSection { title: titles[1], rect: settings_card_rect(y1, h1) },
        SettingsSection { title: titles[2], rect: settings_card_rect(y2, h2) },
    ]
}

const SETTINGS_PAGE_SCHEMAS: [SettingsPageSchema; 6] = [
    SettingsPageSchema { sections: &GENERAL_SECTIONS, scrollable: true },
    SettingsPageSchema { sections: &HOTKEY_SECTIONS, scrollable: false },
    SettingsPageSchema { sections: &PLUGIN_SECTIONS, scrollable: false },
    SettingsPageSchema { sections: &GROUP_SECTIONS, scrollable: false },
    SettingsPageSchema { sections: &CLOUD_SECTIONS, scrollable: false },
    SettingsPageSchema { sections: &ABOUT_SECTIONS, scrollable: false },
];

pub const fn settings_title_rect() -> UiRect {
    UiRect::new(SETTINGS_NAV_W + 36, 32, SETTINGS_NAV_W + 360, 62)
}

pub fn settings_page_schema(page: usize) -> &'static SettingsPageSchema {
    SETTINGS_PAGE_SCHEMAS.get(page).unwrap_or(&SETTINGS_PAGE_SCHEMAS[0])
}

pub fn settings_page_scrollable(page: usize) -> bool {
    settings_page_schema(page).scrollable
}

pub fn settings_cards_for_page(page: usize) -> &'static [SettingsSection] {
    settings_page_schema(page).sections
}

pub fn settings_form_section_height(rows: i32) -> i32 {
    let rows = rows.max(1);
    SETTINGS_FORM_HEADER_H
        + rows * SETTINGS_FORM_ROW_H
        + (rows - 1) * SETTINGS_FORM_ROW_GAP
        + SETTINGS_FORM_SECTION_PAD
}

pub fn settings_cards_for_page_vec(page: usize) -> Vec<SettingsSection> {
    if page == 1 {
        return settings_form_cards(
            16,
            ["主快捷键", "系统剪贴板历史（Win+V）", "功能说明"],
            HOTKEY_FORM_SECTIONS,
        );
    }
    if page == 2 {
        return settings_form_cards(
            16,
            ["搜索插件", "AI 文本清洗", "超级邮件合并"],
            PLUGIN_FORM_SECTIONS,
        );
    }
    if page == 4 {
        return settings_form_cards(
            16,
            ["云同步", "连接与凭据", "操作"],
            CLOUD_FORM_SECTIONS,
        );
    }
    settings_cards_for_page(page).to_vec()
}

#[allow(dead_code)]
pub fn settings_section_rect(page: usize, index: usize) -> UiRect {
    settings_cards_for_page_vec(page)
        .get(index)
        .map(|s| s.rect)
        .unwrap_or_else(|| settings_card_rect(16, 96))
}

#[allow(dead_code)]
pub fn settings_page_total_content_h(page: usize) -> i32 {
    settings_cards_for_page_vec(page)
        .iter()
        .map(|s| s.rect.bottom - SETTINGS_CONTENT_Y + SETTINGS_FORM_SECTION_GAP)
        .max()
        .unwrap_or(760)
}

#[allow(dead_code)]
pub fn settings_last_section_bottom(page: usize) -> i32 {
    settings_cards_for_page_vec(page)
        .last()
        .map(|s| s.rect.bottom)
        .unwrap_or(SETTINGS_CONTENT_Y + 220)
}

pub fn settings_section(page: usize, index: usize) -> SettingsSection {
    settings_cards_for_page_vec(page)
        .get(index)
        .copied()
        .unwrap_or(SettingsSection { title: "", rect: settings_card_rect(16, 96) })
}

pub fn settings_section_body_rect(page: usize, index: usize, padding: i32) -> UiRect {
    let rc = settings_section(page, index).rect;
    UiRect::new(
        rc.left + padding,
        rc.top + SETTINGS_FORM_HEADER_H,
        rc.right - padding,
        rc.bottom - padding,
    )
}

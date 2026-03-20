use crate::ui::{SETTINGS_CONTENT_W, SETTINGS_CONTENT_X, SETTINGS_CONTENT_Y, SETTINGS_NAV_W};
use crate::ui::UiRect;

pub const SCROLL_BAR_W: i32 = 3;
pub const SCROLL_BAR_W_ACTIVE: i32 = 5;
pub const SCROLL_BAR_MARGIN: i32 = 3;
pub const SETTINGS_PAGE_COUNT: usize = 6;

pub const CARD_GENERAL_Y: i32 = 16;
pub const CARD_GENERAL_H: i32 = 470;
pub const CARD_DATA_Y: i32 = 498;
pub const CARD_DATA_H: i32 = 96;
pub const CARD_ACTIONS_Y: i32 = 606;
pub const CARD_ACTIONS_H: i32 = 190;
pub const CARD_POSITION_Y: i32 = 808;
pub const CARD_POSITION_H: i32 = 168;
pub const CARD_MAINTAIN_Y: i32 = 988;
pub const CARD_MAINTAIN_H: i32 = 96;

pub const SETTINGS_FORM_HEADER_H: i32 = 52;
pub const SETTINGS_FORM_ROW_H: i32 = 32;
pub const SETTINGS_FORM_ROW_GAP: i32 = 8;
pub const SETTINGS_FORM_SECTION_GAP: i32 = 12;
pub const SETTINGS_FORM_SECTION_PAD: i32 = 18;

#[inline]
pub const fn settings_card_rect(y: i32, h: i32) -> UiRect {
    UiRect::new(
        SETTINGS_CONTENT_X,
        SETTINGS_CONTENT_Y + y,
        SETTINGS_CONTENT_X + SETTINGS_CONTENT_W,
        SETTINGS_CONTENT_Y + y + h,
    )
}

#[derive(Clone, Copy)]
pub struct SettingsSection {
    pub title: &'static str,
    pub rect: UiRect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsPage {
    General = 0,
    Hotkey = 1,
    Plugin = 2,
    Group = 3,
    Cloud = 4,
    About = 5,
}

impl SettingsPage {
    pub const fn index(self) -> usize { self as usize }
    pub fn from_index(index: usize) -> Self {
        match index {
            1 => SettingsPage::Hotkey,
            2 => SettingsPage::Plugin,
            3 => SettingsPage::Group,
            4 => SettingsPage::Cloud,
            5 => SettingsPage::About,
            _ => SettingsPage::General,
        }
    }
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

const GENERAL_SECTIONS: [SettingsSection; 5] = [
    SettingsSection { title: "启动与显示", rect: settings_card_rect(CARD_GENERAL_Y, CARD_GENERAL_H) },
    SettingsSection { title: "数据", rect: settings_card_rect(CARD_DATA_Y, CARD_DATA_H) },
    SettingsSection { title: "快捷操作", rect: settings_card_rect(CARD_ACTIONS_Y, CARD_ACTIONS_H) },
    SettingsSection { title: "显示位置", rect: settings_card_rect(CARD_POSITION_Y, CARD_POSITION_H) },
    SettingsSection { title: "维护", rect: settings_card_rect(CARD_MAINTAIN_Y, CARD_MAINTAIN_H) },
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
    SettingsSection { title: "同步设置", rect: settings_card_rect(16, 166) },
    SettingsSection { title: "WebDAV 连接", rect: settings_card_rect(194, 206) },
    SettingsSection { title: "同步操作", rect: settings_card_rect(412, 118) },
];

const ABOUT_SECTIONS: [SettingsSection; 1] = [
    SettingsSection { title: "关于", rect: settings_card_rect(16, 340) },
];

const PLUGIN_FORM_SECTIONS: [SettingsFormCardSpec; 3] = [
    SettingsFormCardSpec { rows: 4 },
    SettingsFormCardSpec { rows: 1 },
    SettingsFormCardSpec { rows: 3 },
];

const CLOUD_FORM_SECTIONS: [SettingsFormCardSpec; 3] = [
    SettingsFormCardSpec { rows: 3 },
    SettingsFormCardSpec { rows: 4 },
    SettingsFormCardSpec { rows: 2 },
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
            ["同步设置", "WebDAV 连接", "同步操作"],
            CLOUD_FORM_SECTIONS,
        );
    }
    settings_cards_for_page(page).to_vec()
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

#[derive(Clone, Copy)]
pub struct SettingsFormSectionLayout {
    body: UiRect,
    label_w: i32,
}

impl SettingsFormSectionLayout {
    pub fn new(page: usize, index: usize, label_w: i32) -> Self {
        Self {
            body: settings_section_body_rect(page, index, 18),
            label_w,
        }
    }

    pub fn left(&self) -> i32 { self.body.left }
    pub fn label_w(&self) -> i32 { self.label_w }
    pub fn full_w(&self) -> i32 { self.body.right - self.body.left }
    pub fn row_y(&self, row: i32) -> i32 { self.body.top + row * (SETTINGS_FORM_ROW_H + SETTINGS_FORM_ROW_GAP) }
    pub fn label_y(&self, row: i32, h: i32) -> i32 { self.row_y(row) + ((SETTINGS_FORM_ROW_H - h).max(0) / 2) }
    pub fn field_x(&self) -> i32 { self.body.left + self.label_w }
    pub fn field_w(&self) -> i32 { (self.body.right - self.field_x()).max(40) }
    pub fn field_w_from(&self, x: i32) -> i32 { (self.body.right - x).max(40) }
    pub fn action_x(&self, slot: i32, w: i32) -> i32 { self.body.left + slot * (w + 14) }
}

pub fn settings_page_content_total_h(page: usize) -> i32 {
    let cards = settings_cards_for_page_vec(page);
    let content_bottom = cards
        .iter()
        .map(|section| section.rect.bottom - SETTINGS_CONTENT_Y + 16)
        .max()
        .unwrap_or(0);
    content_bottom.max(0)
}

pub fn settings_page_max_scroll(page: usize, view_h: i32) -> i32 {
    if !settings_page_scrollable(page) {
        return 0;
    }
    (settings_page_content_total_h(page) - view_h).max(0)
}

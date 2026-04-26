use crate::ui::{
    settings_content_w_scaled, settings_content_x_scaled, settings_content_y_scaled,
    settings_nav_w_scaled, settings_scale, UiRect,
};

pub const SCROLL_BAR_W: i32 = 3;
pub const SCROLL_BAR_W_ACTIVE: i32 = 5;
pub const SCROLL_BAR_MARGIN: i32 = 3;
pub const SETTINGS_PAGE_COUNT: usize = 6;

pub const SETTINGS_FORM_HEADER_H: i32 = 52;
pub const SETTINGS_FORM_ROW_H: i32 = 32;
pub const SETTINGS_FORM_ROW_GAP: i32 = 8;
pub const SETTINGS_FORM_SECTION_GAP: i32 = 12;
pub const SETTINGS_FORM_SECTION_PAD: i32 = 18;
pub const SETTINGS_FORM_BOTTOM_SAFE_H: i32 = 24;

#[inline]
pub fn settings_card_rect(y: i32, h: i32) -> UiRect {
    UiRect::new(
        settings_content_x_scaled(),
        settings_content_y_scaled() + settings_scale(y),
        settings_content_x_scaled() + settings_content_w_scaled(),
        settings_content_y_scaled() + settings_scale(y + h),
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
    pub const fn index(self) -> usize {
        self as usize
    }

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
pub struct SettingsFormCardSpec {
    pub rows: i32,
    pub extra_px: i32,
}

const HOTKEY_FORM_SECTIONS: [SettingsFormCardSpec; 3] = [
    SettingsFormCardSpec {
        rows: 6,
        extra_px: 0,
    },
    SettingsFormCardSpec {
        rows: 2,
        extra_px: 24,
    },
    SettingsFormCardSpec {
        rows: 2,
        extra_px: 12,
    },
];

const GENERAL_FORM_SECTIONS: [SettingsFormCardSpec; 5] = [
    SettingsFormCardSpec {
        rows: 10,
        extra_px: 0,
    },
    SettingsFormCardSpec {
        rows: 1,
        extra_px: 0,
    },
    SettingsFormCardSpec {
        rows: 7,
        extra_px: 0,
    },
    SettingsFormCardSpec {
        rows: 3,
        extra_px: 0,
    },
    SettingsFormCardSpec {
        rows: 1,
        extra_px: 0,
    },
];

const PLUGIN_FORM_SECTIONS: [SettingsFormCardSpec; 4] = [
    SettingsFormCardSpec {
        rows: 4,
        extra_px: 10,
    },
    SettingsFormCardSpec {
        rows: 4,
        extra_px: 20,
    },
    SettingsFormCardSpec {
        rows: 5,
        extra_px: 12,
    },
    SettingsFormCardSpec {
        rows: 5,
        extra_px: 0,
    },
];

const GROUP_FORM_SECTIONS: [SettingsFormCardSpec; 2] = [
    SettingsFormCardSpec {
        rows: 3,
        extra_px: 0,
    },
    SettingsFormCardSpec {
        rows: 9,
        extra_px: 0,
    },
];

const CLOUD_FORM_SECTIONS: [SettingsFormCardSpec; 3] = [
    SettingsFormCardSpec {
        rows: 3,
        extra_px: 0,
    },
    SettingsFormCardSpec {
        rows: 4,
        extra_px: 0,
    },
    SettingsFormCardSpec {
        rows: 2,
        extra_px: 0,
    },
];

const ABOUT_FORM_SECTIONS: [SettingsFormCardSpec; 1] = [SettingsFormCardSpec {
    rows: 12,
    extra_px: 96,
}];

pub fn settings_title_rect() -> UiRect {
    UiRect::new(
        settings_nav_w_scaled() + settings_scale(36),
        settings_scale(32),
        settings_nav_w_scaled() + settings_scale(360),
        settings_scale(62),
    )
}

pub fn settings_page_scrollable(page: usize) -> bool {
    settings_page_content_total_h(page) > 0
}

pub fn settings_form_section_height_with_extra(rows: i32, extra_px: i32) -> i32 {
    let rows = rows.max(1);
    settings_scale(SETTINGS_FORM_HEADER_H)
        + rows * settings_scale(SETTINGS_FORM_ROW_H)
        + (rows - 1) * settings_scale(SETTINGS_FORM_ROW_GAP)
        + settings_scale(SETTINGS_FORM_SECTION_PAD)
        + settings_scale(SETTINGS_FORM_BOTTOM_SAFE_H)
        + settings_scale(extra_px.max(0))
}

fn settings_make_form_cards(
    y0: i32,
    titles: [&'static str; 3],
    specs: [SettingsFormCardSpec; 3],
) -> Vec<SettingsSection> {
    let top0 = settings_scale(y0);
    let gap = settings_scale(SETTINGS_FORM_SECTION_GAP);
    let h0 = settings_form_section_height_with_extra(specs[0].rows, specs[0].extra_px);
    let h1 = settings_form_section_height_with_extra(specs[1].rows, specs[1].extra_px);
    let h2 = settings_form_section_height_with_extra(specs[2].rows, specs[2].extra_px);
    let top1 = top0 + h0 + gap;
    let top2 = top1 + h1 + gap;
    vec![
        SettingsSection {
            title: titles[0],
            rect: UiRect::new(
                settings_content_x_scaled(),
                settings_content_y_scaled() + top0,
                settings_content_x_scaled() + settings_content_w_scaled(),
                settings_content_y_scaled() + top0 + h0,
            ),
        },
        SettingsSection {
            title: titles[1],
            rect: UiRect::new(
                settings_content_x_scaled(),
                settings_content_y_scaled() + top1,
                settings_content_x_scaled() + settings_content_w_scaled(),
                settings_content_y_scaled() + top1 + h1,
            ),
        },
        SettingsSection {
            title: titles[2],
            rect: UiRect::new(
                settings_content_x_scaled(),
                settings_content_y_scaled() + top2,
                settings_content_x_scaled() + settings_content_w_scaled(),
                settings_content_y_scaled() + top2 + h2,
            ),
        },
    ]
}

fn settings_make_form_cards_dyn(
    y0: i32,
    titles: &[&'static str],
    specs: &[SettingsFormCardSpec],
) -> Vec<SettingsSection> {
    let mut out = Vec::with_capacity(specs.len());
    let mut top = settings_scale(y0);
    let gap = settings_scale(SETTINGS_FORM_SECTION_GAP);
    for (idx, spec) in specs.iter().enumerate() {
        let h = settings_form_section_height_with_extra(spec.rows, spec.extra_px);
        out.push(SettingsSection {
            title: titles.get(idx).copied().unwrap_or(""),
            rect: UiRect::new(
                settings_content_x_scaled(),
                settings_content_y_scaled() + top,
                settings_content_x_scaled() + settings_content_w_scaled(),
                settings_content_y_scaled() + top + h,
            ),
        });
        top += h + gap;
    }
    out
}

pub fn settings_cards_for_page_vec(page: usize) -> Vec<SettingsSection> {
    match SettingsPage::from_index(page) {
        SettingsPage::General => settings_make_form_cards_dyn(
            16,
            &["启动与显示", "数据", "快捷操作", "显示位置", "维护"],
            &GENERAL_FORM_SECTIONS,
        ),
        SettingsPage::Hotkey => settings_make_form_cards(
            16,
            ["主快捷键", "系统剪贴板历史（Win+V）", "功能说明"],
            HOTKEY_FORM_SECTIONS,
        ),
        SettingsPage::Plugin => settings_make_form_cards_dyn(
            16,
            &[
                "搜索插件",
                "图片 OCR",
                "文本翻译",
                "AI 文本清洗 / 超级邮件合并 / 二维码 / 独立插件",
            ],
            &PLUGIN_FORM_SECTIONS,
        ),
        SettingsPage::Group => {
            settings_make_form_cards_dyn(16, &["分组功能", "分组管理"], &GROUP_FORM_SECTIONS)
        }
        SettingsPage::Cloud => settings_make_form_cards(
            16,
            ["同步设置", "WebDAV 连接", "同步操作"],
            CLOUD_FORM_SECTIONS,
        ),
        SettingsPage::About => settings_make_form_cards_dyn(16, &["关于"], &ABOUT_FORM_SECTIONS),
    }
}

pub fn settings_section(page: usize, index: usize) -> SettingsSection {
    settings_cards_for_page_vec(page)
        .get(index)
        .copied()
        .unwrap_or(SettingsSection {
            title: "",
            rect: settings_card_rect(16, 96),
        })
}

pub fn settings_section_body_rect(page: usize, index: usize, padding: i32) -> UiRect {
    let rc = settings_section(page, index).rect;
    let pad = settings_scale(padding);
    UiRect::new(
        rc.left + pad,
        rc.top + settings_scale(SETTINGS_FORM_HEADER_H),
        rc.right - pad,
        rc.bottom - pad,
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
            label_w: settings_scale(label_w),
        }
    }

    pub fn left(&self) -> i32 {
        self.body.left
    }
    pub fn label_w(&self) -> i32 {
        self.label_w
    }
    pub fn full_w(&self) -> i32 {
        self.body.right - self.body.left
    }
    pub fn row_y(&self, row: i32) -> i32 {
        self.body.top
            + row * (settings_scale(SETTINGS_FORM_ROW_H) + settings_scale(SETTINGS_FORM_ROW_GAP))
    }
    pub fn label_y(&self, row: i32, h: i32) -> i32 {
        self.row_y(row) + ((settings_scale(SETTINGS_FORM_ROW_H) - h).max(0) / 2)
    }
    pub fn field_x(&self) -> i32 {
        self.body.left + self.label_w
    }
    pub fn field_w(&self) -> i32 {
        (self.body.right - self.field_x()).max(40)
    }
    pub fn field_w_from(&self, x: i32) -> i32 {
        (self.body.right - x).max(40)
    }
    pub fn action_x(&self, slot: i32, w: i32) -> i32 {
        self.body.left + slot * (w + settings_scale(14))
    }
}

pub fn settings_page_content_total_h(page: usize) -> i32 {
    let cards = settings_cards_for_page_vec(page);
    let content_bottom = cards
        .iter()
        .map(|section| section.rect.bottom - settings_content_y_scaled() + settings_scale(16))
        .max()
        .unwrap_or(0);
    content_bottom.max(0)
}

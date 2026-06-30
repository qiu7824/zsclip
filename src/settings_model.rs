use crate::app_core::{
    clamp_window_pos_to_rect, command_ids, product_ai_capabilities_for_context,
    settings_content_w_scaled, settings_content_x_scaled, settings_content_y_scaled,
    settings_nav_item_rect, settings_nav_w_scaled, settings_scale, ProductAiActionKind,
    ProductAiContextKind, ProductAiProviderKind, ProductAiResultKind, ProductAiUiSurface, UiRect,
    SETTINGS_PAGE_LABELS,
};
use crate::i18n::{tr, translate};

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
pub const SETTINGS_VIEWPORT_MASK_H: i32 = 14;
pub const SETTINGS_DROPDOWN_ITEM_H: i32 = 38;
pub const SETTINGS_DROPDOWN_PAD: i32 = 6;
pub const SETTINGS_DROPDOWN_MAX_VISIBLE_ROWS: i32 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SettingsPluginAiCapabilityPresentation {
    pub(crate) capability_id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) provider: ProductAiProviderKind,
    pub(crate) action: ProductAiActionKind,
    pub(crate) result: ProductAiResultKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettingsPluginAiPanel {
    pub(crate) surface: ProductAiUiSurface,
    pub(crate) required_context: ProductAiContextKind,
    pub(crate) capabilities: Vec<SettingsPluginAiCapabilityPresentation>,
}

#[inline]
pub fn settings_card_rect(y: i32, h: i32) -> UiRect {
    UiRect::new(
        settings_content_x_scaled(),
        settings_content_y_scaled() + settings_scale(y),
        settings_content_x_scaled() + settings_content_w_scaled(),
        settings_content_y_scaled() + settings_scale(y + h),
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

// Target native hosts consume these summaries under macOS/Linux cfg paths.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsNativePageSummary {
    pub page: SettingsPage,
    pub label: &'static str,
    pub section_titles: Vec<&'static str>,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsNativeSectionSummary {
    pub page: SettingsPage,
    pub page_label: &'static str,
    pub section_index: usize,
    pub section_title: &'static str,
    pub control_rows: i32,
    pub extra_px: i32,
    pub rect: UiRect,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsNativeControlKind {
    Label,
    TextInput,
    Toggle,
    Dropdown,
    Button,
    List,
}

#[allow(dead_code)]
impl SettingsNativeControlKind {
    pub const fn role_name(self) -> &'static str {
        match self {
            Self::Label => "label",
            Self::TextInput => "text_input",
            Self::Toggle => "toggle",
            Self::Dropdown => "dropdown",
            Self::Button => "button",
            Self::List => "list",
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsNativeControlSummary {
    pub page: SettingsPage,
    pub page_label: &'static str,
    pub section_index: usize,
    pub section_title: &'static str,
    pub control_index: usize,
    pub key: &'static str,
    pub label: &'static str,
    pub kind: SettingsNativeControlKind,
    pub route: Option<SettingsNativeControlRoute>,
    pub binding: Option<SettingsNativeControlBinding>,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsNativeControlRouteKind {
    Command,
    Action,
}

#[allow(dead_code)]
impl SettingsNativeControlRouteKind {
    pub const fn role_name(self) -> &'static str {
        match self {
            Self::Command => "command",
            Self::Action => "action",
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsNativeControlRoute {
    pub kind: SettingsNativeControlRouteKind,
    pub route_name: &'static str,
    pub command_id: Option<&'static str>,
    pub control_id: Option<i64>,
    pub action_name: Option<&'static str>,
}

#[allow(dead_code)]
impl SettingsNativeControlSummary {
    pub fn route_label(&self) -> String {
        match self.route {
            Some(route) => match route.kind {
                SettingsNativeControlRouteKind::Command => format!(
                    "{}:{}#{}",
                    route.kind.role_name(),
                    route.command_id.unwrap_or("unknown"),
                    route.control_id.unwrap_or_default()
                ),
                SettingsNativeControlRouteKind::Action => format!(
                    "{}:{}/{}",
                    route.kind.role_name(),
                    route.route_name,
                    route.action_name.unwrap_or("unknown")
                ),
            },
            None => "unwired".to_string(),
        }
    }

    pub fn binding_label(&self) -> String {
        match self.binding {
            Some(binding) => format!(
                "{}:{}",
                binding.kind.role_name(),
                binding.field_name.unwrap_or(binding.binding_name)
            ),
            None => "unbound".to_string(),
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsNativeControlBindingKind {
    SettingField,
    RuntimeList,
    DerivedValue,
}

#[allow(dead_code)]
impl SettingsNativeControlBindingKind {
    pub const fn role_name(self) -> &'static str {
        match self {
            Self::SettingField => "setting",
            Self::RuntimeList => "list",
            Self::DerivedValue => "derived",
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsNativeControlBinding {
    pub kind: SettingsNativeControlBindingKind,
    pub binding_name: &'static str,
    pub field_name: Option<&'static str>,
    pub collect_required: bool,
    pub apply_required: bool,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsNativeApplyCollectBinding {
    pub page: SettingsPage,
    pub page_label: &'static str,
    pub control_key: &'static str,
    pub control_label: &'static str,
    pub binding: SettingsNativeControlBinding,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsNativeApplyCollectPlan {
    pub collect_bindings: Vec<SettingsNativeApplyCollectBinding>,
    pub apply_bindings: Vec<SettingsNativeApplyCollectBinding>,
    pub command_route_count: usize,
    pub action_route_count: usize,
    pub bound_control_count: usize,
}

#[allow(dead_code)]
impl SettingsNativeApplyCollectPlan {
    pub fn summary_label(&self) -> String {
        format!(
            "collect={} apply={} command_routes={} action_routes={} bound_controls={}",
            self.collect_bindings.len(),
            self.apply_bindings.len(),
            self.command_route_count,
            self.action_route_count,
            self.bound_control_count
        )
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsNativeSubmittedControlValue {
    pub control_key: String,
    pub raw_value: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsNativeAppliedField {
    pub control_key: String,
    pub control_label: &'static str,
    pub field_name: &'static str,
    pub value: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsNativeJsonFieldUpdate {
    pub field_name: String,
    pub value: serde_json::Value,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsNativeJsonApplyResult {
    pub settings_json: serde_json::Value,
    pub field_updates: Vec<SettingsNativeJsonFieldUpdate>,
    pub rejected_fields: Vec<String>,
}

#[allow(dead_code)]
impl SettingsNativeJsonApplyResult {
    pub fn summary_label(&self) -> String {
        format!(
            "json_updates={} rejected_fields={}",
            self.field_updates.len(),
            self.rejected_fields.len()
        )
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsNativeControlDisplayValue {
    pub control_key: &'static str,
    pub value: String,
    pub sensitive: bool,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsNativeDropdownOption {
    pub raw_value: String,
    pub label: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsNativeDropdownOptions {
    pub control_key: &'static str,
    pub options: Vec<SettingsNativeDropdownOption>,
    pub selected_index: usize,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsNativeCollectSubmission {
    pub applied_fields: Vec<SettingsNativeAppliedField>,
    pub missing_control_keys: Vec<&'static str>,
    pub ignored_control_keys: Vec<String>,
}

#[allow(dead_code)]
impl SettingsNativeCollectSubmission {
    pub fn summary_label(&self) -> String {
        format!(
            "submitted_fields={} missing_collect_controls={} ignored_values={}",
            self.applied_fields.len(),
            self.missing_control_keys.len(),
            self.ignored_control_keys.len()
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsDpiMoveAction {
    None,
    SyncOnly,
    ResizeForDpi,
}

pub fn settings_dpi_move_action(
    old_dpi: u32,
    next_dpi: u32,
    per_monitor: bool,
) -> SettingsDpiMoveAction {
    if old_dpi.max(96) == next_dpi.max(96) {
        SettingsDpiMoveAction::None
    } else if per_monitor {
        SettingsDpiMoveAction::ResizeForDpi
    } else {
        SettingsDpiMoveAction::SyncOnly
    }
}

pub fn settings_scroll_delta_for_wheel(delta: i32) -> i32 {
    if delta > 0 {
        -60
    } else {
        60
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsWindowMovePlan {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

fn settings_work_bounds(work: UiRect, pad: i32) -> UiRect {
    UiRect::new(
        work.left + pad,
        work.top + pad,
        work.right - pad,
        work.bottom - pad,
    )
}

fn settings_window_size_limits(
    work: UiRect,
    pad: i32,
    min_w: i32,
    min_h: i32,
) -> (i32, i32, i32, i32) {
    let work_w = (work.right - work.left).max(1);
    let work_h = (work.bottom - work.top).max(1);
    let max_w = (work_w - pad * 2).max(1);
    let max_h = (work_h - pad * 2).max(1);
    (min_w.min(max_w), min_h.min(max_h), max_w, max_h)
}

pub fn settings_window_fit_plan(
    current: UiRect,
    work: UiRect,
    pad: i32,
    min_w: i32,
    min_h: i32,
) -> Option<SettingsWindowMovePlan> {
    if current.right <= current.left || current.bottom <= current.top {
        return None;
    }
    let (min_w, min_h, max_w, max_h) = settings_window_size_limits(work, pad, min_w, min_h);
    let target_w = (current.right - current.left).clamp(min_w, max_w);
    let target_h = (current.bottom - current.top).clamp(min_h, max_h);
    let bounds = settings_work_bounds(work, pad);
    let (x, y) = clamp_window_pos_to_rect(current.left, current.top, bounds, target_w, target_h);
    if x == current.left
        && y == current.top
        && target_w == current.right - current.left
        && target_h == current.bottom - current.top
    {
        None
    } else {
        Some(SettingsWindowMovePlan {
            x,
            y,
            width: target_w,
            height: target_h,
        })
    }
}

pub fn settings_window_dpi_transition_plan(
    current: UiRect,
    work: UiRect,
    pad: i32,
    min_w: i32,
    min_h: i32,
    old_dpi: u32,
    new_dpi: u32,
) -> Option<SettingsWindowMovePlan> {
    let old_dpi = old_dpi.max(96);
    let new_dpi = new_dpi.max(96);
    if old_dpi == new_dpi || current.right <= current.left || current.bottom <= current.top {
        return None;
    }
    let (min_w, min_h, max_w, max_h) = settings_window_size_limits(work, pad, min_w, min_h);
    let cur_w = current.right - current.left;
    let cur_h = current.bottom - current.top;
    let target_w = (((cur_w as i64 * new_dpi as i64) + (old_dpi as i64 / 2)) / old_dpi as i64)
        .clamp(min_w as i64, max_w as i64) as i32;
    let target_h = (((cur_h as i64 * new_dpi as i64) + (old_dpi as i64 / 2)) / old_dpi as i64)
        .clamp(min_h as i64, max_h as i64) as i32;
    let center_x = current.left + cur_w / 2;
    let center_y = current.top + cur_h / 2;
    let bounds = settings_work_bounds(work, pad);
    let (x, y) = clamp_window_pos_to_rect(
        center_x - target_w / 2,
        center_y - target_h / 2,
        bounds,
        target_w,
        target_h,
    );
    Some(SettingsWindowMovePlan {
        x,
        y,
        width: target_w,
        height: target_h,
    })
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

const HOTKEY_TITLES: [&str; 3] = [
    "\u{4e3b}\u{5feb}\u{6377}\u{952e}",
    "\u{7cfb}\u{7edf}\u{526a}\u{8d34}\u{677f}\u{5386}\u{53f2}\u{ff08}Win+V\u{ff09}",
    "\u{529f}\u{80fd}\u{8bf4}\u{660e}",
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
        rows: 9,
        extra_px: 0,
    },
    SettingsFormCardSpec {
        rows: 4,
        extra_px: 0,
    },
    SettingsFormCardSpec {
        rows: 1,
        extra_px: 0,
    },
];

const GENERAL_TITLES: [&str; 5] = [
    "\u{542f}\u{52a8}\u{4e0e}\u{663e}\u{793a}",
    "\u{6570}\u{636e}",
    "\u{5feb}\u{6377}\u{64cd}\u{4f5c}",
    "\u{663e}\u{793a}\u{4f4d}\u{7f6e}",
    "\u{7ef4}\u{62a4}",
];

const PLUGIN_TITLES: [&str; 7] = [
    "\u{641c}\u{7d22}\u{63d2}\u{4ef6}",
    "\u{56fe}\u{7247} OCR",
    "\u{6587}\u{672c}\u{7ffb}\u{8bd1}",
    "AI \u{6587}\u{672c}\u{6e05}\u{6d17}",
    "\u{8d85}\u{7ea7}\u{90ae}\u{4ef6}\u{5408}\u{5e76}",
    "WPS \u{4efb}\u{52a1}\u{7a97}\u{683c}",
    "\u{5feb}\u{6377}\u{4e8c}\u{7ef4}\u{7801}\u{8f6c}\u{6362}",
];

const PLUGIN_FORM_SECTIONS: [SettingsFormCardSpec; 7] = [
    SettingsFormCardSpec {
        rows: 1,
        extra_px: 0,
    },
    SettingsFormCardSpec {
        rows: 1,
        extra_px: 0,
    },
    SettingsFormCardSpec {
        rows: 1,
        extra_px: 0,
    },
    SettingsFormCardSpec {
        rows: 1,
        extra_px: 0,
    },
    SettingsFormCardSpec {
        rows: 1,
        extra_px: 0,
    },
    SettingsFormCardSpec {
        rows: 1,
        extra_px: 0,
    },
    SettingsFormCardSpec {
        rows: 1,
        extra_px: 0,
    },
];

fn plugin_form_sections(
    quick_search_enabled: bool,
    image_ocr_provider: &str,
    text_translate_provider: &str,
    ai_provider_config_count: usize,
    super_mail_merge_enabled: bool,
    wps_taskpane_enabled: bool,
) -> [SettingsFormCardSpec; 7] {
    [
        SettingsFormCardSpec {
            rows: if quick_search_enabled { 4 } else { 1 },
            extra_px: 0,
        },
        SettingsFormCardSpec {
            rows: match image_ocr_provider {
                "baidu" | "winocr" => 3,
                _ => 1,
            },
            extra_px: 0,
        },
        SettingsFormCardSpec {
            rows: if text_translate_provider == "baidu" {
                4
            } else {
                1
            },
            extra_px: 0,
        },
        SettingsFormCardSpec {
            rows: i32::try_from(ai_provider_config_count.max(1)).unwrap_or(i32::MAX),
            extra_px: 0,
        },
        SettingsFormCardSpec {
            rows: if super_mail_merge_enabled { 2 } else { 1 },
            extra_px: 0,
        },
        SettingsFormCardSpec {
            rows: if wps_taskpane_enabled { 2 } else { 1 },
            extra_px: 0,
        },
        SettingsFormCardSpec {
            rows: 1,
            extra_px: 0,
        },
    ]
}

const GROUP_FORM_SECTIONS: [SettingsFormCardSpec; 2] = [
    SettingsFormCardSpec {
        rows: 3,
        extra_px: 0,
    },
    SettingsFormCardSpec {
        rows: 10,
        extra_px: 0,
    },
];

const GROUP_TITLES: [&str; 2] = [
    "\u{5206}\u{7ec4}\u{529f}\u{80fd}",
    "\u{5206}\u{7ec4}\u{7ba1}\u{7406}",
];

const MULTI_SYNC_TITLES: [&str; 6] = [
    "\u{591a}\u{7aef}\u{540c}\u{6b65}\u{6982}\u{89c8}",
    "WebDAV \u{4f20}\u{8f93}",
    "WebDAV \u{64cd}\u{4f5c}",
    "\u{5c40}\u{57df}\u{7f51}\u{4f20}\u{8f93}\u{ff08}\u{626b}\u{7801}\u{7ed1}\u{5b9a}\u{ff09}",
    "\u{8bbe}\u{5907}\u{53d1}\u{73b0} / \u{914d}\u{5bf9}",
    "\u{626b}\u{7801}\u{7ed1}\u{5b9a}",
];

const MULTI_SYNC_OVERVIEW_SPEC: SettingsFormCardSpec = SettingsFormCardSpec {
    rows: 2,
    extra_px: 0,
};
const MULTI_SYNC_WEBDAV_SPECS: [SettingsFormCardSpec; 2] = [
    SettingsFormCardSpec {
        rows: 6,
        extra_px: 0,
    },
    SettingsFormCardSpec {
        rows: 2,
        extra_px: 0,
    },
];
const MULTI_SYNC_LAN_SPECS: [SettingsFormCardSpec; 3] = [
    SettingsFormCardSpec {
        rows: 6,
        extra_px: 0,
    },
    SettingsFormCardSpec {
        rows: 7,
        extra_px: 110,
    },
    SettingsFormCardSpec {
        rows: 10,
        extra_px: 0,
    },
];

const ABOUT_FORM_SECTIONS: [SettingsFormCardSpec; 1] = [SettingsFormCardSpec {
    rows: 12,
    extra_px: 96,
}];
const ABOUT_TITLES: [&str; 1] = ["\u{5173}\u{4e8e}"];

pub fn settings_title_rect() -> UiRect {
    UiRect::new(
        settings_nav_w_scaled() + settings_scale(36),
        settings_scale(32),
        settings_nav_w_scaled() + settings_scale(360),
        settings_scale(62),
    )
}

pub fn settings_nav_index_at(x: i32, y: i32, page_count: usize) -> Option<usize> {
    (0..page_count).find(|&index| settings_nav_item_rect(index).contains(x, y))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsPointerDownTarget {
    None,
    NavPage(usize),
    ScrollbarThumb {
        drag_start_y: i32,
        drag_start_scroll: i32,
    },
    ScrollbarTrack {
        scroll_y: i32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsNavHoverTransition {
    pub next_hot: i32,
    pub invalidate_rects: Vec<UiRect>,
}

pub fn settings_nav_hover_for_pointer(
    current_hot: i32,
    x: i32,
    y: i32,
    page_count: usize,
) -> SettingsNavHoverTransition {
    let next_hot = settings_nav_index_at(x, y, page_count)
        .map(|index| index as i32)
        .unwrap_or(-1);
    settings_nav_hover_transition(current_hot, next_hot, page_count)
}

pub fn settings_nav_hover_transition(
    current_hot: i32,
    next_hot: i32,
    page_count: usize,
) -> SettingsNavHoverTransition {
    let next_hot = if next_hot >= 0 && (next_hot as usize) < page_count {
        next_hot
    } else {
        -1
    };
    if current_hot == next_hot {
        return SettingsNavHoverTransition {
            next_hot,
            invalidate_rects: Vec::new(),
        };
    }

    let mut invalidate_rects = Vec::new();
    if current_hot >= 0 && (current_hot as usize) < page_count {
        invalidate_rects.push(settings_nav_item_rect(current_hot as usize));
    }
    if next_hot >= 0 {
        invalidate_rects.push(settings_nav_item_rect(next_hot as usize));
    }
    SettingsNavHoverTransition {
        next_hot,
        invalidate_rects,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsPointerMoveTransition {
    pub drag_scroll_y: Option<i32>,
    pub nav_hover: Option<SettingsNavHoverTransition>,
}

pub fn settings_pointer_move_transition(
    x: i32,
    y: i32,
    page_count: usize,
    current_nav_hot: i32,
    scroll_dragging: bool,
    scroll_layout: SettingsScrollLayout,
    drag_start_y: i32,
    drag_start_scroll: i32,
) -> SettingsPointerMoveTransition {
    if scroll_dragging {
        return SettingsPointerMoveTransition {
            drag_scroll_y: scroll_layout.drag_scroll_target(drag_start_y, drag_start_scroll, y),
            nav_hover: None,
        };
    }

    SettingsPointerMoveTransition {
        drag_scroll_y: None,
        nav_hover: Some(settings_nav_hover_for_pointer(
            current_nav_hot,
            x,
            y,
            page_count,
        )),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsNavIconKind {
    General,
    Hotkey,
    Plugin,
    Group,
    Sync,
    About,
}

impl SettingsNavIconKind {
    pub fn for_page(page: SettingsPage) -> Self {
        match page {
            SettingsPage::General => Self::General,
            SettingsPage::Hotkey => Self::Hotkey,
            SettingsPage::Plugin => Self::Plugin,
            SettingsPage::Group => Self::Group,
            SettingsPage::Cloud => Self::Sync,
            SettingsPage::About => Self::About,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsNavItemRender {
    pub index: usize,
    pub page: SettingsPage,
    pub label: &'static str,
    pub icon: SettingsNavIconKind,
    pub rect: UiRect,
    pub selected: bool,
    pub hovered: bool,
    pub badge_rect: Option<UiRect>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsNavRenderPlan {
    pub items: Vec<SettingsNavItemRender>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsThemeRole {
    Background,
    NavBackground,
    NavSelectedFill,
    NavHoverFill,
    Surface,
    Accent,
    Stroke,
    ScrollbarTrack,
    ScrollbarThumb,
    ScrollbarThumbDragging,
    Text,
    TextMuted,
    Danger,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsTextFontRole {
    UiText,
    Display,
    FluentIcon,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsTextContent {
    Label(&'static str),
    NavIcon(SettingsNavIconKind),
    ChromeMenuIcon,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsPaintCommand {
    FillRect {
        rect: UiRect,
        fill: SettingsThemeRole,
    },
    RoundRect {
        rect: UiRect,
        fill: SettingsThemeRole,
        stroke: SettingsThemeRole,
        radius: i32,
    },
    RoundFill {
        rect: UiRect,
        fill: SettingsThemeRole,
        radius: i32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsTextCommand {
    pub rect: UiRect,
    pub content: SettingsTextContent,
    pub color: SettingsThemeRole,
    pub size: i32,
    pub bold: bool,
    pub font: SettingsTextFontRole,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SettingsPaintPlan {
    pub paint_commands: Vec<SettingsPaintCommand>,
    pub text_commands: Vec<SettingsTextCommand>,
}

pub type SettingsNavItemPaintPlan = SettingsPaintPlan;
pub type SettingsChromePaintPlan = SettingsPaintPlan;
pub type SettingsContentPaintPlan = SettingsPaintPlan;
pub type SettingsScrollbarPaintPlan = SettingsPaintPlan;

pub fn settings_nav_item_paint_plan(item: &SettingsNavItemRender) -> SettingsNavItemPaintPlan {
    let mut paint_commands = Vec::new();
    if item.selected {
        paint_commands.push(SettingsPaintCommand::RoundFill {
            rect: item.rect,
            fill: SettingsThemeRole::NavSelectedFill,
            radius: settings_scale(6),
        });
        let bar_h = settings_scale(16);
        let bar_cy = (item.rect.top + item.rect.bottom) / 2;
        paint_commands.push(SettingsPaintCommand::RoundFill {
            rect: UiRect::new(
                item.rect.left + settings_scale(3),
                bar_cy - bar_h / 2,
                item.rect.left + settings_scale(6),
                bar_cy + bar_h / 2,
            ),
            fill: SettingsThemeRole::Accent,
            radius: settings_scale(2),
        });
    } else if item.hovered {
        paint_commands.push(SettingsPaintCommand::RoundFill {
            rect: item.rect,
            fill: SettingsThemeRole::NavHoverFill,
            radius: settings_scale(6),
        });
    }

    let icon_color = if item.selected {
        SettingsThemeRole::Accent
    } else if item.hovered {
        SettingsThemeRole::Text
    } else {
        SettingsThemeRole::TextMuted
    };
    let label_color = if item.selected || item.hovered {
        SettingsThemeRole::Text
    } else {
        SettingsThemeRole::TextMuted
    };
    let icon_rect = UiRect::new(
        item.rect.left + settings_scale(10),
        item.rect.top,
        item.rect.left + settings_scale(38),
        item.rect.bottom,
    );
    let label_rect = UiRect::new(
        item.rect.left + settings_scale(40),
        item.rect.top,
        item.rect.right - settings_scale(8),
        item.rect.bottom,
    );
    let text_commands = vec![
        SettingsTextCommand {
            rect: icon_rect,
            content: SettingsTextContent::NavIcon(item.icon),
            color: icon_color,
            size: 16,
            bold: false,
            font: SettingsTextFontRole::FluentIcon,
        },
        SettingsTextCommand {
            rect: label_rect,
            content: SettingsTextContent::Label(item.label),
            color: label_color,
            size: 14,
            bold: false,
            font: SettingsTextFontRole::UiText,
        },
    ];
    if let Some(rect) = item.badge_rect {
        paint_commands.push(SettingsPaintCommand::RoundFill {
            rect,
            fill: SettingsThemeRole::Danger,
            radius: settings_scale(5),
        });
    }

    SettingsPaintPlan {
        paint_commands,
        text_commands,
    }
}

pub fn settings_nav_render_plan(
    current_page: usize,
    hover_page: Option<usize>,
    update_available: bool,
) -> SettingsNavRenderPlan {
    let page_count = SETTINGS_PAGE_COUNT.min(SETTINGS_PAGE_LABELS.len());
    let current_page = current_page.min(page_count.saturating_sub(1));
    let hover_page = hover_page.filter(|page| *page < page_count);
    let items = (0..page_count)
        .map(|index| {
            let page = SettingsPage::from_index(index);
            let rect = settings_nav_item_rect(index);
            SettingsNavItemRender {
                index,
                page,
                label: SETTINGS_PAGE_LABELS[index],
                icon: SettingsNavIconKind::for_page(page),
                rect,
                selected: index == current_page,
                hovered: hover_page == Some(index),
                badge_rect: if update_available && page == SettingsPage::About {
                    Some(settings_nav_badge_rect(rect))
                } else {
                    None
                },
            }
        })
        .collect();
    SettingsNavRenderPlan { items }
}

fn settings_nav_badge_rect(item_rect: UiRect) -> UiRect {
    UiRect::new(
        item_rect.right - settings_scale(22),
        item_rect.top + settings_scale(14),
        item_rect.right - settings_scale(12),
        item_rect.top + settings_scale(24),
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsChromeRenderPlan {
    pub window_rect: UiRect,
    pub nav_rect: UiRect,
    pub divider_x: i32,
    pub menu_icon_rect: UiRect,
    pub app_title_rect: UiRect,
    pub page_title_rect: UiRect,
    pub content_clip_rect: UiRect,
    pub viewport_mask_rect: UiRect,
    pub viewport_mask_separator_rect: UiRect,
}

pub fn settings_chrome_render_plan(window: UiRect) -> SettingsChromeRenderPlan {
    let viewport_mask_rect = settings_viewport_mask_rect_for_window(window);
    SettingsChromeRenderPlan {
        window_rect: window,
        nav_rect: UiRect::new(0, 0, settings_nav_w_scaled(), window.bottom),
        divider_x: settings_nav_w_scaled(),
        menu_icon_rect: UiRect::new(
            settings_scale(22),
            settings_scale(18),
            settings_scale(50),
            settings_scale(46),
        ),
        app_title_rect: UiRect::new(
            settings_scale(56),
            settings_scale(18),
            settings_scale(220),
            settings_scale(50),
        ),
        page_title_rect: settings_title_rect(),
        content_clip_rect: settings_safe_paint_rect_for_window(window),
        viewport_mask_separator_rect: UiRect::new(
            viewport_mask_rect.left + settings_scale(12),
            viewport_mask_rect.bottom - 1,
            viewport_mask_rect.right - settings_scale(12),
            viewport_mask_rect.bottom,
        ),
        viewport_mask_rect,
    }
}

pub fn settings_chrome_paint_plan(
    plan: &SettingsChromeRenderPlan,
    page_title: &'static str,
) -> SettingsChromePaintPlan {
    SettingsPaintPlan {
        paint_commands: vec![
            SettingsPaintCommand::FillRect {
                rect: plan.nav_rect,
                fill: SettingsThemeRole::NavBackground,
            },
            SettingsPaintCommand::FillRect {
                rect: UiRect::new(
                    plan.divider_x,
                    plan.window_rect.top,
                    plan.divider_x + 1,
                    plan.window_rect.bottom,
                ),
                fill: SettingsThemeRole::Stroke,
            },
        ],
        text_commands: vec![
            SettingsTextCommand {
                rect: plan.menu_icon_rect,
                content: SettingsTextContent::ChromeMenuIcon,
                color: SettingsThemeRole::TextMuted,
                size: 16,
                bold: false,
                font: SettingsTextFontRole::FluentIcon,
            },
            SettingsTextCommand {
                rect: plan.app_title_rect,
                content: SettingsTextContent::Label("设置"),
                color: SettingsThemeRole::Text,
                size: 15,
                bold: true,
                font: SettingsTextFontRole::UiText,
            },
            SettingsTextCommand {
                rect: plan.page_title_rect,
                content: SettingsTextContent::Label(page_title),
                color: SettingsThemeRole::Text,
                size: 24,
                bold: true,
                font: SettingsTextFontRole::Display,
            },
        ],
    }
}

pub fn settings_viewport_mask_paint_plan(
    plan: &SettingsChromeRenderPlan,
) -> SettingsContentPaintPlan {
    SettingsPaintPlan {
        paint_commands: vec![
            SettingsPaintCommand::FillRect {
                rect: plan.viewport_mask_rect,
                fill: SettingsThemeRole::Background,
            },
            SettingsPaintCommand::FillRect {
                rect: plan.viewport_mask_separator_rect,
                fill: SettingsThemeRole::Stroke,
            },
        ],
        text_commands: Vec::new(),
    }
}

pub fn settings_viewport_rect_for_window(window: UiRect) -> UiRect {
    UiRect::new(
        settings_nav_w_scaled(),
        settings_content_y_scaled(),
        window.right,
        window.bottom,
    )
}

pub fn settings_viewport_mask_rect_for_window(window: UiRect) -> UiRect {
    UiRect::new(
        settings_nav_w_scaled(),
        settings_content_y_scaled(),
        window.right,
        settings_content_y_scaled() + settings_scale(SETTINGS_VIEWPORT_MASK_H),
    )
}

pub fn settings_safe_paint_rect_for_window(window: UiRect) -> UiRect {
    let mask = settings_viewport_mask_rect_for_window(window);
    UiRect::new(mask.left, mask.bottom, mask.right, window.bottom)
}

pub fn settings_child_visible_in_viewport(new_y: i32, height: i32, viewport: UiRect) -> bool {
    let safe_top = viewport.top + settings_scale(SETTINGS_VIEWPORT_MASK_H);
    new_y + height > safe_top && new_y < viewport.bottom
}

pub fn settings_dropdown_label_for_max_items(max_items: usize) -> &'static str {
    match max_items {
        100 => "100",
        200 => "200",
        500 => "500",
        1000 => "1000",
        3000 => "3000",
        _ => tr("无限制", "Unlimited"),
    }
}

pub fn settings_dropdown_max_items_labels() -> [&'static str; 6] {
    [
        "100",
        "200",
        "500",
        "1000",
        "3000",
        tr("无限制", "Unlimited"),
    ]
}

pub fn settings_dropdown_index_for_max_items(max_items: usize) -> usize {
    match max_items {
        100 => 0,
        200 => 1,
        500 => 2,
        1000 => 3,
        3000 => 4,
        _ => 5,
    }
}

pub fn settings_dropdown_max_items_from_label_opt(label: &str) -> Option<usize> {
    match label.trim() {
        "100" => Some(100),
        "200" => Some(200),
        "500" => Some(500),
        "1000" => Some(1000),
        "3000" => Some(3000),
        value if value == tr("无限制", "Unlimited") => Some(0),
        _ => None,
    }
}

pub fn settings_dropdown_max_items_from_label(label: &str) -> usize {
    settings_dropdown_max_items_from_label_opt(label).unwrap_or(0)
}

pub fn settings_dropdown_label_for_pos_mode(mode: &str) -> &'static str {
    match mode {
        "fixed" => tr("固定位置", "Fixed Position"),
        "last" => tr("上次位置", "Last Position"),
        _ => tr("跟随鼠标", "Follow Mouse"),
    }
}

pub fn settings_dropdown_index_for_pos_mode(mode: &str) -> usize {
    match mode {
        "fixed" => 1,
        "last" => 2,
        _ => 0,
    }
}

pub fn settings_dropdown_pos_mode_from_label(label: &str) -> String {
    match label.trim() {
        "固定位置" | "Fixed Position" => "fixed".to_string(),
        "上次位置" | "Last Position" => "last".to_string(),
        _ => "mouse".to_string(),
    }
}

pub const SEARCH_ENGINE_PRESETS: [(&str, &str, &str); 12] = [
    (
        "jzxx",
        "筑森搜索（jzxx.vip）",
        "https://jzxx.vip/search/more.html?type=11&key={q}&se=2",
    ),
    ("bing", "必应", "https://www.bing.com/search?q={q}"),
    ("baidu", "百度", "https://www.baidu.com/s?wd={q}"),
    ("google", "Google", "https://www.google.com/search?q={q}"),
    ("sogou", "搜狗", "https://www.sogou.com/web?query={q}"),
    ("360", "360搜索", "https://www.so.com/s?q={q}"),
    ("quark", "夸克", "https://quark.sm.cn/s?q={q}"),
    ("sm", "神马", "https://m.sm.cn/s?q={q}"),
    ("ddg", "DuckDuckGo", "https://duckduckgo.com/?q={q}"),
    ("yahoo", "Yahoo", "https://search.yahoo.com/search?p={q}"),
    ("yandex", "Yandex", "https://yandex.com/search/?text={q}"),
    ("custom", "自定义", "https://example.com/search?q={q}"),
];

pub const IMAGE_OCR_PROVIDER_OPTIONS: [(&str, &str); 3] = [
    ("off", "关闭"),
    ("baidu", "百度 OCR"),
    ("winocr", "WinOCR（微信 OCR）"),
];

pub const TEXT_TRANSLATE_PROVIDER_OPTIONS: [(&str, &str); 2] =
    [("off", "关闭"), ("baidu", "百度翻译")];

pub const TEXT_TRANSLATE_TARGET_OPTIONS: [(&str, &str); 4] = [
    ("zh", "简体中文"),
    ("en", "英语"),
    ("jp", "日语"),
    ("kor", "韩语"),
];

pub const HOTKEY_MOD_OPTIONS: [&str; 8] = [
    "Win",
    "Ctrl",
    "Alt",
    "Shift",
    "Ctrl+Alt",
    "Ctrl+Shift",
    "Alt+Shift",
    "Ctrl+Alt+Shift",
];

pub const HOTKEY_KEY_OPTIONS: [&str; 51] = [
    "A",
    "B",
    "C",
    "D",
    "E",
    "F",
    "G",
    "H",
    "I",
    "J",
    "K",
    "L",
    "M",
    "N",
    "O",
    "P",
    "Q",
    "R",
    "S",
    "T",
    "U",
    "V",
    "W",
    "X",
    "Y",
    "Z",
    "0",
    "1",
    "2",
    "3",
    "4",
    "5",
    "6",
    "7",
    "8",
    "9",
    "Space",
    "Enter",
    "Tab",
    "Esc",
    "Backspace",
    "Delete",
    "Insert",
    "Up",
    "Down",
    "Left",
    "Right",
    "Home",
    "End",
    "PageUp",
    "PageDown",
];

pub const PASTE_SOUND_OPTIONS: [(&str, &str); 4] = [
    ("default", "默认"),
    ("soft", "柔和"),
    ("bright", "清脆"),
    ("custom", "自定义文件"),
];

pub fn search_engine_template(key: &str) -> &'static str {
    SEARCH_ENGINE_PRESETS
        .iter()
        .find(|(k, _, _)| *k == key)
        .map(|(_, _, tpl)| *tpl)
        .unwrap_or(SEARCH_ENGINE_PRESETS[0].2)
}

pub fn search_engine_display(key: &str) -> String {
    SEARCH_ENGINE_PRESETS
        .iter()
        .find(|(k, _, _)| *k == key)
        .map(|(_, name, _)| translate(name).into_owned())
        .unwrap_or_else(|| translate(SEARCH_ENGINE_PRESETS[0].1).into_owned())
}

pub fn search_engine_key_from_display(label: &str) -> &'static str {
    SEARCH_ENGINE_PRESETS
        .iter()
        .find(|(_, name, _)| *name == label || translate(name).as_ref() == label)
        .map(|(k, _, _)| *k)
        .unwrap_or("jzxx")
}

pub fn image_ocr_provider_display(key: &str) -> String {
    IMAGE_OCR_PROVIDER_OPTIONS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, name)| translate(name).into_owned())
        .unwrap_or_else(|| translate(IMAGE_OCR_PROVIDER_OPTIONS[0].1).into_owned())
}

pub fn image_ocr_provider_key_from_display(label: &str) -> &'static str {
    IMAGE_OCR_PROVIDER_OPTIONS
        .iter()
        .find(|(_, name)| *name == label || translate(name).as_ref() == label)
        .map(|(k, _)| *k)
        .unwrap_or("off")
}

pub fn text_translate_provider_display(key: &str) -> String {
    TEXT_TRANSLATE_PROVIDER_OPTIONS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, name)| translate(name).into_owned())
        .unwrap_or_else(|| translate(TEXT_TRANSLATE_PROVIDER_OPTIONS[0].1).into_owned())
}

pub fn text_translate_provider_key_from_display(label: &str) -> &'static str {
    TEXT_TRANSLATE_PROVIDER_OPTIONS
        .iter()
        .find(|(_, name)| *name == label || translate(name).as_ref() == label)
        .map(|(k, _)| *k)
        .unwrap_or("off")
}

pub fn text_translate_target_display(key: &str) -> String {
    TEXT_TRANSLATE_TARGET_OPTIONS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, name)| translate(name).into_owned())
        .unwrap_or_else(|| translate(TEXT_TRANSLATE_TARGET_OPTIONS[0].1).into_owned())
}

pub fn text_translate_target_key_from_display(label: &str) -> &'static str {
    TEXT_TRANSLATE_TARGET_OPTIONS
        .iter()
        .find(|(_, name)| *name == label || translate(name).as_ref() == label)
        .map(|(k, _)| *k)
        .unwrap_or("zh")
}

pub fn paste_sound_display(key: &str) -> String {
    PASTE_SOUND_OPTIONS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, name)| translate(name).into_owned())
        .unwrap_or_else(|| translate(PASTE_SOUND_OPTIONS[0].1).into_owned())
}

pub fn paste_sound_key_from_display(label: &str) -> &'static str {
    PASTE_SOUND_OPTIONS
        .iter()
        .find(|(_, name)| *name == label || translate(name).as_ref() == label)
        .map(|(k, _)| *k)
        .unwrap_or("default")
}

pub fn paste_sound_file_button_text(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return tr("选择文件", "Choose file").to_string();
    }
    std::path::Path::new(trimmed)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .map(|name| name.to_string())
        .unwrap_or_else(|| trimmed.to_string())
}

pub fn normalize_source_tab(tab: usize) -> usize {
    if tab == 1 {
        1
    } else {
        0
    }
}

pub fn source_tab_category(tab: usize) -> i64 {
    normalize_source_tab(tab) as i64
}

pub fn source_tab_all_label(tab: usize) -> &'static str {
    if normalize_source_tab(tab) == 1 {
        tr("全部短语", "All Phrases")
    } else {
        tr("全部记录", "All Records")
    }
}

pub fn source_tab_label(tab: usize) -> &'static str {
    if normalize_source_tab(tab) == 1 {
        tr("常用短语", "Phrases")
    } else {
        tr("复制记录", "Clipboard Records")
    }
}

pub fn settings_group_overview_text(tab: usize, current_filter: &str) -> String {
    format!(
        "{}（{}）：{}",
        tr("当前分组", "Current Group"),
        source_tab_label(tab),
        current_filter
    )
}

pub fn normalize_hotkey_mod(value: &str) -> String {
    let trimmed = value.trim();
    if HOTKEY_MOD_OPTIONS.contains(&trimmed) {
        trimmed.to_string()
    } else {
        "Win".to_string()
    }
}

pub fn normalize_hotkey_key(value: &str) -> String {
    let trimmed = value.trim();
    if HOTKEY_KEY_OPTIONS.contains(&trimmed) {
        trimmed.to_string()
    } else {
        "V".to_string()
    }
}

pub fn hotkey_preview_text(mod_label: &str, key_label: &str) -> String {
    format!(
        "{}{} + {}",
        tr("当前设置：", "Current setting: "),
        normalize_hotkey_mod(mod_label),
        normalize_hotkey_key(key_label)
    )
}

pub fn group_name_for_display_entries<'a, I>(groups: I, group_id: i64, all_label: &str) -> String
where
    I: IntoIterator<Item = (i64, &'a str)>,
{
    if group_id == 0 {
        return all_label.to_string();
    }
    groups
        .into_iter()
        .find(|(id, _)| *id == group_id)
        .map(|(_, name)| name.to_string())
        .unwrap_or_else(|| all_label.to_string())
}

#[cfg(feature = "lan-sync")]
pub const MULTI_SYNC_MODE_OPTIONS: [&str; 3] = ["关闭", "WebDAV", "局域网"];
#[cfg(not(feature = "lan-sync"))]
pub const MULTI_SYNC_MODE_OPTIONS: [&str; 2] = ["关闭", "WebDAV"];

pub fn multi_sync_mode_from_flags(
    cloud_sync_enabled: bool,
    lan_sync_enabled: bool,
) -> &'static str {
    #[cfg(not(feature = "lan-sync"))]
    {
        if cloud_sync_enabled {
            return "webdav";
        }
        return "off";
    }
    #[cfg(feature = "lan-sync")]
    if lan_sync_enabled {
        "lan"
    } else if cloud_sync_enabled {
        "webdav"
    } else {
        "off"
    }
}

pub fn multi_sync_mode_display(mode: &str) -> &'static str {
    match mode {
        "webdav" => "WebDAV",
        #[cfg(feature = "lan-sync")]
        "lan" => "局域网",
        _ => "关闭",
    }
}

pub fn multi_sync_mode_from_label(label: &str) -> &'static str {
    if label.eq_ignore_ascii_case("webdav") {
        return "webdav";
    }
    #[cfg(feature = "lan-sync")]
    if label.contains("局域") || label.eq_ignore_ascii_case("lan") {
        return "lan";
    }
    "off"
}

pub fn multi_sync_flags_for_mode(mode: &str) -> (bool, bool) {
    match mode {
        "webdav" => (true, false),
        #[cfg(feature = "lan-sync")]
        "lan" => (false, true),
        _ => (false, false),
    }
}

pub fn normalize_multi_sync_flags(
    cloud_sync_enabled: bool,
    lan_sync_enabled: bool,
) -> (bool, bool) {
    multi_sync_flags_for_mode(multi_sync_mode_from_flags(
        cloud_sync_enabled,
        lan_sync_enabled,
    ))
}

pub fn localized_cloud_status_text(status: &str) -> String {
    let trimmed = status.trim();
    if trimmed.is_empty() || trimmed == "未同步" {
        return tr("未同步", "Not synced").to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("失败：") {
        return format!("{}{}", tr("失败：", "Failed: "), rest);
    }
    translate(trimmed).into_owned()
}

pub fn lan_receive_mode_display(mode: &str) -> &'static str {
    match mode {
        "clipboard" => "直接覆盖剪贴板",
        _ => "只进入记录",
    }
}

pub fn lan_receive_mode_from_label(label: &str) -> &'static str {
    if label.contains("剪贴板") || label.eq_ignore_ascii_case("clipboard") {
        "clipboard"
    } else {
        "records_only"
    }
}

pub fn lan_trusted_summary_value_text(summary: &str) -> String {
    summary
        .strip_prefix("信任设备：")
        .or_else(|| summary.strip_prefix("信任设备:"))
        .unwrap_or(summary)
        .trim_start()
        .to_string()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsDropdownPopupLayout {
    pub rect: UiRect,
    pub item_height: i32,
    pub pad: i32,
    pub visible_rows: i32,
    pub scroll_top: i32,
    pub max_scroll: i32,
}

impl SettingsDropdownPopupLayout {
    pub fn new(anchor: UiRect, item_count: usize, selected: usize, requested_width: i32) -> Self {
        let item_count_i32 = item_count as i32;
        let visible_rows = item_count_i32.clamp(1, SETTINGS_DROPDOWN_MAX_VISIBLE_ROWS);
        let item_height = settings_scale(SETTINGS_DROPDOWN_ITEM_H);
        let pad = settings_scale(SETTINGS_DROPDOWN_PAD);
        let height = pad * 2 + item_height * visible_rows;
        let max_scroll = (item_count_i32 - visible_rows).max(0);
        let scroll_top = (selected as i32 - visible_rows / 2).clamp(0, max_scroll);
        let width = requested_width.max(anchor.right - anchor.left);
        Self {
            rect: UiRect::new(
                anchor.left,
                anchor.bottom + settings_scale(6),
                anchor.left + width,
                anchor.bottom + settings_scale(6) + height,
            ),
            item_height,
            pad,
            visible_rows,
            scroll_top,
            max_scroll,
        }
    }
}

pub fn settings_dropdown_index_from_y(
    item_count: usize,
    scroll_top: i32,
    visible_rows: i32,
    item_height: i32,
    pad: i32,
    y: i32,
) -> Option<i32> {
    if item_count == 0 || visible_rows <= 0 || item_height <= 0 {
        return None;
    }
    if y < pad || y >= pad + item_height * visible_rows {
        return None;
    }
    let row = ((y - pad) / item_height).clamp(0, visible_rows - 1);
    Some((scroll_top + row).clamp(0, item_count as i32 - 1))
}

pub fn settings_dropdown_max_scroll(item_count: usize, visible_rows: i32) -> i32 {
    (item_count as i32 - visible_rows).max(0)
}

pub fn settings_dropdown_scroll_step_for_wheel(delta: i32) -> i32 {
    if delta > 0 {
        -1
    } else {
        1
    }
}

pub fn settings_dropdown_scroll_target(
    item_count: usize,
    visible_rows: i32,
    scroll_top: i32,
    delta: i32,
) -> i32 {
    (scroll_top + settings_dropdown_scroll_step_for_wheel(delta))
        .clamp(0, settings_dropdown_max_scroll(item_count, visible_rows))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsDropdownInteractionState {
    pub item_count: usize,
    pub hover: i32,
    pub item_height: i32,
    pub pad: i32,
    pub scroll_top: i32,
    pub visible_rows: i32,
}

impl SettingsDropdownInteractionState {
    pub fn new(item_count: usize, layout: SettingsDropdownPopupLayout) -> Self {
        Self {
            item_count,
            hover: -1,
            item_height: layout.item_height,
            pad: layout.pad,
            scroll_top: layout.scroll_top,
            visible_rows: layout.visible_rows,
        }
    }

    pub fn index_from_y(self, y: i32) -> Option<i32> {
        settings_dropdown_index_from_y(
            self.item_count,
            self.scroll_top,
            self.visible_rows,
            self.item_height,
            self.pad,
            y,
        )
    }

    pub fn max_scroll(self) -> i32 {
        settings_dropdown_max_scroll(self.item_count, self.visible_rows)
    }

    pub fn scroll_by_wheel(&mut self, delta: i32) -> bool {
        let next = settings_dropdown_scroll_target(
            self.item_count,
            self.visible_rows,
            self.scroll_top,
            delta,
        );
        if next == self.scroll_top {
            return false;
        }
        self.scroll_top = next;
        self.hover = -1;
        true
    }
}

pub fn settings_page_scrollable(page: usize) -> bool {
    settings_page_content_total_h(page) > 0
}

pub fn settings_normalized_page_index(page: usize, page_count: usize) -> usize {
    if page_count == 0 {
        0
    } else {
        page.min(page_count - 1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsPageSwitchMode {
    SyncOnly,
    Switch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsPageSwitchScrollState {
    pub page: usize,
    pub content_scroll_y: i32,
    pub page_scroll_y: i32,
    pub scroll_bar_visible: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsPageSwitchPlan {
    pub mode: SettingsPageSwitchMode,
    pub old_page: usize,
    pub target_page: usize,
    pub cancel_hotkey_recording: bool,
    pub cancel_scroll_drag: bool,
    pub close_dropdown: bool,
    pub scroll_state: Option<SettingsPageSwitchScrollState>,
    pub reposition_controls: bool,
}

pub fn settings_page_switch_plan(
    current_page: usize,
    target_page: usize,
    page_count: usize,
    target_page_built: bool,
    hotkey_recording: bool,
    scroll_dragging: bool,
    dropdown_open: bool,
    target_page_scroll_y: i32,
) -> SettingsPageSwitchPlan {
    let old_page = settings_normalized_page_index(current_page, page_count);
    let target_page = settings_normalized_page_index(target_page, page_count);
    if old_page == target_page && target_page_built {
        return SettingsPageSwitchPlan {
            mode: SettingsPageSwitchMode::SyncOnly,
            old_page,
            target_page,
            cancel_hotkey_recording: false,
            cancel_scroll_drag: false,
            close_dropdown: false,
            scroll_state: None,
            reposition_controls: false,
        };
    }

    let scrollable = settings_page_scrollable(target_page);
    SettingsPageSwitchPlan {
        mode: SettingsPageSwitchMode::Switch,
        old_page,
        target_page,
        cancel_hotkey_recording: old_page == SettingsPage::Hotkey.index()
            && target_page != old_page
            && hotkey_recording,
        cancel_scroll_drag: scroll_dragging,
        close_dropdown: dropdown_open,
        scroll_state: Some(SettingsPageSwitchScrollState {
            page: target_page,
            content_scroll_y: if scrollable { target_page_scroll_y } else { 0 },
            page_scroll_y: if scrollable { target_page_scroll_y } else { 0 },
            scroll_bar_visible: false,
        }),
        reposition_controls: scrollable,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsControlMetrics {
    pub page: usize,
    pub bounds: UiRect,
    pub visible: bool,
}

pub type SettingsControlId = usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsControlState {
    pub id: SettingsControlId,
    pub page: usize,
    pub bounds: UiRect,
    pub scrollable: bool,
    pub visible: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsScrollControlState {
    pub id: SettingsControlId,
    pub page: usize,
    pub bounds: UiRect,
    pub visible: bool,
}

pub struct SettingsUiModel {
    built_pages: [bool; SETTINGS_PAGE_COUNT],
    controls: Vec<SettingsControlState>,
    scroll_controls: Vec<SettingsScrollControlState>,
    content_bottoms: [i32; SETTINGS_PAGE_COUNT],
}

impl SettingsUiModel {
    pub fn new() -> Self {
        Self {
            built_pages: [false; SETTINGS_PAGE_COUNT],
            controls: Vec::new(),
            scroll_controls: Vec::new(),
            content_bottoms: [0; SETTINGS_PAGE_COUNT],
        }
    }

    pub fn is_built(&self, page: usize) -> bool {
        self.built_pages.get(page).copied().unwrap_or(false)
    }

    pub fn mark_built(&mut self, page: usize) {
        if let Some(slot) = self.built_pages.get_mut(page) {
            *slot = true;
        }
    }

    pub fn register(&mut self, mut control: SettingsControlState) {
        control.page = control.page.min(SETTINGS_PAGE_COUNT.saturating_sub(1));
        self.controls.push(control);
        if let Some(bottom) = self.content_bottoms.get_mut(control.page) {
            *bottom = (*bottom).max(settings_control_content_bottom(control.bounds));
        }
        if control.scrollable {
            self.scroll_controls.push(SettingsScrollControlState {
                id: control.id,
                page: control.page,
                bounds: control.bounds,
                visible: control.visible,
            });
        }
    }

    pub fn set_control_visible(&mut self, id: SettingsControlId, visible: bool) {
        let mut touched_pages = Vec::new();
        for control in self.controls.iter_mut().filter(|control| control.id == id) {
            control.visible = visible;
            touched_pages.push(control.page);
        }
        for slot in self.scroll_controls.iter_mut().filter(|slot| slot.id == id) {
            slot.visible = visible;
        }
        self.recompute_touched_pages(touched_pages);
    }

    pub fn set_control_bounds(&mut self, id: SettingsControlId, bounds: UiRect) {
        let mut touched_pages = Vec::new();
        for control in self.controls.iter_mut().filter(|control| control.id == id) {
            control.bounds = bounds;
            touched_pages.push(control.page);
        }
        for slot in self.scroll_controls.iter_mut().filter(|slot| slot.id == id) {
            slot.bounds = bounds;
        }
        self.recompute_touched_pages(touched_pages);
    }

    fn recompute_touched_pages(&mut self, mut pages: Vec<usize>) {
        pages.sort_unstable();
        pages.dedup();
        for page in pages {
            self.recompute_content_bottom(page);
        }
    }

    fn recompute_content_bottom(&mut self, page: usize) {
        let page = page.min(SETTINGS_PAGE_COUNT.saturating_sub(1));
        let bottom = settings_content_bottom_for_controls(
            self.controls.iter().map(|control| SettingsControlMetrics {
                page: control.page,
                bounds: control.bounds,
                visible: control.visible,
            }),
            page,
        );
        if let Some(slot) = self.content_bottoms.get_mut(page) {
            *slot = bottom;
        }
    }

    pub fn controls_for_page(
        &self,
        page: usize,
    ) -> impl Iterator<Item = SettingsControlState> + '_ {
        self.controls
            .iter()
            .copied()
            .filter(move |control| control.page == page)
    }

    pub fn scroll_controls_for_page(
        &self,
        page: usize,
    ) -> impl Iterator<Item = SettingsScrollControlState> + '_ {
        self.scroll_controls
            .iter()
            .copied()
            .filter(move |slot| slot.page == page)
    }

    pub fn measured_content_total_h(&self, page: usize) -> i32 {
        self.content_bottoms
            .get(page)
            .copied()
            .map(settings_measured_content_total_h)
            .unwrap_or(0)
    }

    pub fn clear_page(&mut self, page: usize) {
        let page = page.min(SETTINGS_PAGE_COUNT.saturating_sub(1));
        self.controls.retain(|control| control.page != page);
        self.scroll_controls.retain(|slot| slot.page != page);
        if let Some(bottom) = self.content_bottoms.get_mut(page) {
            *bottom = 0;
        }
        if let Some(flag) = self.built_pages.get_mut(page) {
            *flag = false;
        }
    }
}

pub fn settings_control_content_bottom(bounds: UiRect) -> i32 {
    bounds.bottom + settings_scale(18)
}

pub fn settings_content_bottom_for_controls<I>(controls: I, page: usize) -> i32
where
    I: IntoIterator<Item = SettingsControlMetrics>,
{
    controls
        .into_iter()
        .filter(|control| control.page == page && control.visible)
        .map(|control| settings_control_content_bottom(control.bounds))
        .max()
        .unwrap_or(settings_content_y_scaled())
}

pub fn settings_measured_content_total_h(content_bottom: i32) -> i32 {
    content_bottom
        .saturating_sub(settings_content_y_scaled())
        .max(0)
}

pub fn settings_sections_content_total_h(sections: &[SettingsSection]) -> i32 {
    sections
        .iter()
        .map(|section| section.rect.bottom - settings_content_y_scaled() + settings_scale(16))
        .max()
        .unwrap_or(0)
        .max(0)
}

pub fn settings_page_content_total_h_for_dynamic_sections(
    page: usize,
    plugin_sections: &[SettingsSection],
    multi_sync_sections: &[SettingsSection],
    measured_content_total_h: i32,
) -> i32 {
    let dynamic_h = if page == SettingsPage::Plugin.index() && !plugin_sections.is_empty() {
        settings_sections_content_total_h(plugin_sections)
    } else if page == SettingsPage::Cloud.index() && !multi_sync_sections.is_empty() {
        settings_sections_content_total_h(multi_sync_sections)
    } else {
        settings_page_content_total_h(page)
    };
    dynamic_h.max(measured_content_total_h)
}

pub fn settings_page_max_scroll(content_total_h: i32, viewport_h: i32) -> i32 {
    (content_total_h - viewport_h).max(0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub struct SettingsScrollUpdate {
    pub page: usize,
    pub content_scroll_y: i32,
    pub page_scroll_y: i32,
}

#[allow(dead_code)]
pub fn settings_scroll_update_for_target(
    page: usize,
    current_scroll_y: i32,
    target_scroll_y: i32,
    content_total_h: i32,
    viewport_h: i32,
) -> Option<SettingsScrollUpdate> {
    let next_scroll_y =
        target_scroll_y.clamp(0, settings_page_max_scroll(content_total_h, viewport_h));
    if next_scroll_y == current_scroll_y {
        return None;
    }
    Some(SettingsScrollUpdate {
        page,
        content_scroll_y: next_scroll_y,
        page_scroll_y: next_scroll_y,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsContentSource {
    StaticPage,
    PluginDynamic,
    MultiSyncDynamic,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsContentRenderPlan {
    pub page: SettingsPage,
    pub source: SettingsContentSource,
    pub sections: Vec<SettingsSection>,
    pub scroll_y: i32,
}

pub fn settings_content_render_plan(
    page: usize,
    scroll_y: i32,
    plugin_sections: &[SettingsSection],
    multi_sync_sections: &[SettingsSection],
) -> SettingsContentRenderPlan {
    let page = SettingsPage::from_index(page);
    let page_index = page.index();
    if page == SettingsPage::Plugin && !plugin_sections.is_empty() {
        SettingsContentRenderPlan {
            page,
            source: SettingsContentSource::PluginDynamic,
            sections: plugin_sections.to_vec(),
            scroll_y,
        }
    } else if page == SettingsPage::Cloud && !multi_sync_sections.is_empty() {
        SettingsContentRenderPlan {
            page,
            source: SettingsContentSource::MultiSyncDynamic,
            sections: multi_sync_sections.to_vec(),
            scroll_y,
        }
    } else {
        SettingsContentRenderPlan {
            page,
            source: SettingsContentSource::StaticPage,
            sections: settings_cards_for_page_vec(page_index),
            scroll_y: if settings_page_scrollable(page_index) {
                scroll_y
            } else {
                0
            },
        }
    }
}

pub fn settings_content_paint_plan(plan: &SettingsContentRenderPlan) -> SettingsContentPaintPlan {
    let mut paint_commands = Vec::with_capacity(plan.sections.len());
    let mut text_commands = Vec::with_capacity(plan.sections.len());
    for section in &plan.sections {
        let rect = section.rect.offset_y(plan.scroll_y);
        paint_commands.push(SettingsPaintCommand::RoundRect {
            rect,
            fill: SettingsThemeRole::Surface,
            stroke: SettingsThemeRole::Stroke,
            radius: settings_scale(8),
        });
        text_commands.push(SettingsTextCommand {
            rect: UiRect::new(
                rect.left + settings_scale(16),
                rect.top + settings_scale(12),
                rect.right - settings_scale(16),
                rect.top + settings_scale(34),
            ),
            content: SettingsTextContent::Label(section.title),
            color: SettingsThemeRole::TextMuted,
            size: 12,
            bold: true,
            font: SettingsTextFontRole::UiText,
        });
    }
    SettingsPaintPlan {
        paint_commands,
        text_commands,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsScrollLayout {
    pub content_top: i32,
    pub viewport_bottom: i32,
    pub content_height: i32,
    pub viewport_height: i32,
    pub right: i32,
    pub margin: i32,
    pub bar_width: i32,
    pub min_thumb_height: i32,
    pub track_padding: i32,
}

impl SettingsScrollLayout {
    pub fn new(
        content_top: i32,
        viewport_bottom: i32,
        content_height: i32,
        viewport_height: i32,
        right: i32,
        margin: i32,
        bar_width: i32,
    ) -> Self {
        Self {
            content_top,
            viewport_bottom,
            content_height,
            viewport_height,
            right,
            margin,
            bar_width,
            min_thumb_height: settings_scale(24),
            track_padding: settings_scale(8),
        }
    }

    pub fn max_scroll(self) -> i32 {
        (self.content_height - self.viewport_height).max(0)
    }

    pub fn track_rect(self) -> Option<UiRect> {
        if self.max_scroll() <= 0 {
            return None;
        }
        let track_top = self.content_top + self.track_padding;
        let track_bottom = self.viewport_bottom - self.track_padding;
        if track_bottom <= track_top {
            return None;
        }
        let right = self.right - self.margin;
        Some(UiRect::new(
            right - self.bar_width,
            track_top,
            right,
            track_bottom,
        ))
    }

    pub fn thumb_rect(self, scroll_y: i32) -> Option<UiRect> {
        let track = self.track_rect()?;
        let track_h = (track.bottom - track.top).max(1);
        let max_scroll = self.max_scroll();
        if max_scroll <= 0 {
            return None;
        }
        let content_h = self.content_height.max(self.viewport_height + 1);
        let thumb_h = ((self.viewport_height as f32 / content_h as f32) * track_h as f32) as i32;
        let thumb_h = thumb_h.max(self.min_thumb_height).min(track_h);
        let drag_range = (track_h - thumb_h).max(1);
        let scroll_y = scroll_y.clamp(0, max_scroll);
        let thumb_top =
            track.top + ((scroll_y as f32 / max_scroll as f32) * drag_range as f32) as i32;
        Some(UiRect::new(
            track.left,
            thumb_top,
            track.right,
            thumb_top + thumb_h,
        ))
    }

    pub fn thumb_hit_rect(self, scroll_y: i32, extra_x: i32) -> Option<UiRect> {
        let thumb = self.thumb_rect(scroll_y)?;
        Some(UiRect::new(
            thumb.left - extra_x,
            thumb.top,
            thumb.right + extra_x,
            thumb.bottom,
        ))
    }

    pub fn drag_scroll_target(
        self,
        drag_start_y: i32,
        drag_start_scroll: i32,
        pointer_y: i32,
    ) -> Option<i32> {
        let track = self.track_rect()?;
        let thumb = self.thumb_rect(drag_start_scroll)?;
        let track_h = (track.bottom - track.top).max(1);
        let thumb_h = (thumb.bottom - thumb.top).max(1);
        let max_scroll = self.max_scroll();
        if max_scroll <= 0 {
            return Some(0);
        }
        let drag_range = (track_h - thumb_h).max(1);
        let dy = pointer_y - drag_start_y;
        let next = drag_start_scroll + ((dy as f32 / drag_range as f32) * max_scroll as f32) as i32;
        Some(next.clamp(0, max_scroll))
    }

    pub fn track_click_scroll_target(self, pointer_y: i32) -> Option<i32> {
        let track = self.track_rect()?;
        let max_scroll = self.max_scroll();
        if max_scroll <= 0 {
            return Some(0);
        }
        let track_h = (track.bottom - track.top).max(1);
        let pointer_pos = (pointer_y - track.top).clamp(0, track_h);
        Some(((pointer_pos as f32 / track_h as f32) * max_scroll as f32) as i32)
    }

    pub fn track_hit_rect(self, extra_left: i32, extra_right: i32) -> Option<UiRect> {
        let track = self.track_rect()?;
        Some(UiRect::new(
            track.left - extra_left,
            track.top - self.track_padding / 2,
            track.right + extra_right,
            track.bottom + self.track_padding / 2,
        ))
    }
}

pub fn settings_pointer_down_target(
    x: i32,
    y: i32,
    page_count: usize,
    scroll_layout: SettingsScrollLayout,
    current_scroll_y: i32,
    thumb_hit_extra_x: i32,
    track_hit_extra_left: i32,
    track_hit_extra_right: i32,
) -> SettingsPointerDownTarget {
    if let Some(page) = settings_nav_index_at(x, y, page_count) {
        return SettingsPointerDownTarget::NavPage(page);
    }

    if scroll_layout
        .thumb_hit_rect(current_scroll_y, thumb_hit_extra_x)
        .map(|hit| hit.contains(x, y))
        .unwrap_or(false)
    {
        return SettingsPointerDownTarget::ScrollbarThumb {
            drag_start_y: y,
            drag_start_scroll: current_scroll_y,
        };
    }

    if scroll_layout
        .track_hit_rect(track_hit_extra_left, track_hit_extra_right)
        .map(|hit| hit.contains(x, y))
        .unwrap_or(false)
    {
        if let Some(scroll_y) = scroll_layout.track_click_scroll_target(y) {
            return SettingsPointerDownTarget::ScrollbarTrack { scroll_y };
        }
    }

    SettingsPointerDownTarget::None
}

pub fn settings_scroll_layout_for_window(
    window: UiRect,
    content_height: i32,
    margin: i32,
    bar_width: i32,
) -> SettingsScrollLayout {
    let content_y = settings_content_y_scaled();
    let view_h = (window.bottom - window.top) - content_y;
    SettingsScrollLayout::new(
        content_y,
        window.bottom,
        content_height,
        view_h,
        window.right,
        margin,
        bar_width,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsScrollbarVisualState {
    Normal,
    Dragging,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsScrollbarRenderPlan {
    pub state: SettingsScrollbarVisualState,
    pub bar_width: i32,
    pub track_rect: Option<UiRect>,
    pub thumb_rect: UiRect,
}

pub fn settings_scrollbar_render_plan(
    window: UiRect,
    content_height: i32,
    scroll_y: i32,
    visible: bool,
    dragging: bool,
    margin: i32,
    normal_bar_width: i32,
    active_bar_width: i32,
) -> Option<SettingsScrollbarRenderPlan> {
    if !visible {
        return None;
    }
    let state = if dragging {
        SettingsScrollbarVisualState::Dragging
    } else {
        SettingsScrollbarVisualState::Normal
    };
    let bar_width = if dragging {
        active_bar_width
    } else {
        normal_bar_width
    };
    let layout = settings_scroll_layout_for_window(window, content_height, margin, bar_width);
    let thumb_rect = layout.thumb_rect(scroll_y)?;
    Some(SettingsScrollbarRenderPlan {
        state,
        bar_width,
        track_rect: if dragging { layout.track_rect() } else { None },
        thumb_rect,
    })
}

pub fn settings_scrollbar_paint_plan(
    plan: &SettingsScrollbarRenderPlan,
) -> SettingsScrollbarPaintPlan {
    let mut paint_commands = Vec::with_capacity(2);
    if let Some(rect) = plan.track_rect {
        paint_commands.push(SettingsPaintCommand::RoundFill {
            rect,
            fill: SettingsThemeRole::ScrollbarTrack,
            radius: plan.bar_width,
        });
    }
    let thumb_fill = if plan.state == SettingsScrollbarVisualState::Dragging {
        SettingsThemeRole::ScrollbarThumbDragging
    } else {
        SettingsThemeRole::ScrollbarThumb
    };
    paint_commands.push(SettingsPaintCommand::RoundFill {
        rect: plan.thumb_rect,
        fill: thumb_fill,
        radius: plan.bar_width,
    });
    SettingsPaintPlan {
        paint_commands,
        text_commands: Vec::new(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsQrRun {
    pub x: i32,
    pub y: i32,
    pub len: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsQrCache {
    pub payload: String,
    pub size: i32,
    pub runs: Vec<SettingsQrRun>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsQrRenderPlan {
    pub white_rect: UiRect,
    pub module_size: i32,
    pub module_rects: Vec<UiRect>,
}

pub fn settings_qr_cache_for_payload(payload: &str) -> Option<SettingsQrCache> {
    use qrcodegen::{QrCode, QrCodeEcc};

    let qr = QrCode::encode_text(payload, QrCodeEcc::Medium).ok()?;
    let size = qr.size();
    let mut runs = Vec::new();
    for y in 0..size {
        let mut run_start: Option<i32> = None;
        for x in 0..size {
            if qr.get_module(x, y) {
                if run_start.is_none() {
                    run_start = Some(x);
                }
            } else if let Some(start) = run_start.take() {
                runs.push(SettingsQrRun {
                    x: start,
                    y,
                    len: x - start,
                });
            }
        }
        if let Some(start) = run_start {
            runs.push(SettingsQrRun {
                x: start,
                y,
                len: size - start,
            });
        }
    }

    Some(SettingsQrCache {
        payload: payload.to_string(),
        size,
        runs,
    })
}

pub fn settings_qr_render_plan(
    bounds: UiRect,
    qr: &SettingsQrCache,
    border_modules: i32,
    inner_padding: i32,
) -> Option<SettingsQrRenderPlan> {
    if qr.size <= 0 || bounds.right <= bounds.left || bounds.bottom <= bounds.top {
        return None;
    }
    let view_modules = qr.size + border_modules.max(0) * 2;
    if view_modules <= 0 {
        return None;
    }
    let side = (bounds.right - bounds.left).min(bounds.bottom - bounds.top) - inner_padding.max(0);
    let module_size = (side / view_modules).max(1);
    let qr_side = module_size * view_modules;
    let left = bounds.left + ((bounds.right - bounds.left - qr_side) / 2);
    let top = bounds.top + ((bounds.bottom - bounds.top - qr_side) / 2);
    let white_rect = UiRect::new(left, top, left + qr_side, top + qr_side);
    let module_rects = qr
        .runs
        .iter()
        .map(|run| {
            let x = left + (run.x + border_modules) * module_size;
            let y = top + (run.y + border_modules) * module_size;
            UiRect::new(x, y, x + run.len * module_size, y + module_size)
        })
        .collect();

    Some(SettingsQrRenderPlan {
        white_rect,
        module_size,
        module_rects,
    })
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

fn settings_make_plugin_cards(
    _y0: i32,
    _titles: &[&'static str],
    _specs: &[SettingsFormCardSpec],
) -> Vec<SettingsSection> {
    settings_make_form_cards_dyn(16, &PLUGIN_TITLES, &PLUGIN_FORM_SECTIONS)
}

pub fn settings_plugin_cards_for_state(
    quick_search_enabled: bool,
    image_ocr_provider: &str,
    text_translate_provider: &str,
    super_mail_merge_enabled: bool,
    wps_taskpane_enabled: bool,
) -> Vec<SettingsSection> {
    let ai_provider_config_count = settings_plugin_ai_panel().capabilities.len();
    let specs = plugin_form_sections(
        quick_search_enabled,
        image_ocr_provider,
        text_translate_provider,
        ai_provider_config_count,
        super_mail_merge_enabled,
        wps_taskpane_enabled,
    );
    settings_make_form_cards_dyn(16, &PLUGIN_TITLES, &specs)
}

pub(crate) fn settings_plugin_ai_panel() -> SettingsPluginAiPanel {
    let surface = ProductAiUiSurface::SettingsPluginPage;
    let required_context = ProductAiContextKind::SettingsProfile;
    let capabilities = product_ai_capabilities_for_context(surface, required_context)
        .into_iter()
        .map(|capability| SettingsPluginAiCapabilityPresentation {
            capability_id: capability.id,
            label: capability.label,
            provider: capability.provider,
            action: capability.action,
            result: capability.result,
        })
        .collect();

    SettingsPluginAiPanel {
        surface,
        required_context,
        capabilities,
    }
}

pub fn settings_multi_sync_cards_for_mode(mode: &str) -> Vec<SettingsSection> {
    let mut titles = vec![MULTI_SYNC_TITLES[0]];
    let mut specs = vec![MULTI_SYNC_OVERVIEW_SPEC];
    match mode {
        "webdav" => {
            titles.extend_from_slice(&MULTI_SYNC_TITLES[1..=2]);
            specs.extend_from_slice(&MULTI_SYNC_WEBDAV_SPECS);
        }
        "lan" => {
            titles.extend_from_slice(&MULTI_SYNC_TITLES[3..=5]);
            specs.extend_from_slice(&MULTI_SYNC_LAN_SPECS);
        }
        _ => {}
    }
    settings_make_form_cards_dyn(16, &titles, &specs)
}

pub fn settings_cards_for_page_vec(page: usize) -> Vec<SettingsSection> {
    match SettingsPage::from_index(page) {
        SettingsPage::General => {
            settings_make_form_cards_dyn(16, &GENERAL_TITLES, &GENERAL_FORM_SECTIONS)
        }
        SettingsPage::Hotkey => settings_make_form_cards(16, HOTKEY_TITLES, HOTKEY_FORM_SECTIONS),
        SettingsPage::Plugin => {
            settings_make_plugin_cards(16, &PLUGIN_TITLES, &PLUGIN_FORM_SECTIONS)
        }
        SettingsPage::Group => {
            settings_make_form_cards_dyn(16, &GROUP_TITLES, &GROUP_FORM_SECTIONS)
        }
        SettingsPage::Cloud => settings_multi_sync_cards_for_mode("off"),
        SettingsPage::About => {
            settings_make_form_cards_dyn(16, &ABOUT_TITLES, &ABOUT_FORM_SECTIONS)
        }
    }
}

#[allow(dead_code)]
fn settings_native_section_specs_for_page(
    page: SettingsPage,
) -> Vec<(&'static str, SettingsFormCardSpec)> {
    match page {
        SettingsPage::General => GENERAL_TITLES
            .iter()
            .copied()
            .zip(GENERAL_FORM_SECTIONS)
            .collect(),
        SettingsPage::Hotkey => HOTKEY_TITLES
            .iter()
            .copied()
            .zip(HOTKEY_FORM_SECTIONS)
            .collect(),
        SettingsPage::Plugin => PLUGIN_TITLES
            .iter()
            .copied()
            .zip(PLUGIN_FORM_SECTIONS)
            .collect(),
        SettingsPage::Group => GROUP_TITLES
            .iter()
            .copied()
            .zip(GROUP_FORM_SECTIONS)
            .collect(),
        SettingsPage::Cloud => {
            let mut specs = vec![(MULTI_SYNC_TITLES[0], MULTI_SYNC_OVERVIEW_SPEC)];
            specs.extend(
                MULTI_SYNC_TITLES[1..=2]
                    .iter()
                    .copied()
                    .zip(MULTI_SYNC_WEBDAV_SPECS),
            );
            specs.extend(
                MULTI_SYNC_TITLES[3..=5]
                    .iter()
                    .copied()
                    .zip(MULTI_SYNC_LAN_SPECS),
            );
            specs
        }
        SettingsPage::About => ABOUT_TITLES
            .iter()
            .copied()
            .zip(ABOUT_FORM_SECTIONS)
            .collect(),
    }
}

#[allow(dead_code)]
fn settings_native_cards_for_page(page: SettingsPage) -> Vec<SettingsSection> {
    let specs = settings_native_section_specs_for_page(page);
    let titles = specs.iter().map(|(title, _)| *title).collect::<Vec<_>>();
    let form_specs = specs.iter().map(|(_, spec)| *spec).collect::<Vec<_>>();
    settings_make_form_cards_dyn(16, &titles, &form_specs)
}

#[allow(dead_code)]
pub fn settings_native_page_summaries() -> Vec<SettingsNativePageSummary> {
    (0..SETTINGS_PAGE_COUNT)
        .map(|index| {
            let page = SettingsPage::from_index(index);
            SettingsNativePageSummary {
                page,
                label: SETTINGS_PAGE_LABELS[index],
                section_titles: settings_native_section_specs_for_page(page)
                    .into_iter()
                    .map(|(title, _)| title)
                    .filter(|title| !title.is_empty())
                    .collect(),
            }
        })
        .collect()
}

#[allow(dead_code)]
pub fn settings_native_section_summaries() -> Vec<SettingsNativeSectionSummary> {
    (0..SETTINGS_PAGE_COUNT)
        .flat_map(|page_index| {
            let page = SettingsPage::from_index(page_index);
            let page_label = SETTINGS_PAGE_LABELS[page_index];
            let cards = settings_native_cards_for_page(page);
            settings_native_section_specs_for_page(page)
                .into_iter()
                .enumerate()
                .map(move |(section_index, (section_title, spec))| {
                    let rect = cards
                        .get(section_index)
                        .copied()
                        .map(|section| section.rect)
                        .unwrap_or_else(|| settings_card_rect(16, 96));
                    SettingsNativeSectionSummary {
                        page,
                        page_label,
                        section_index,
                        section_title,
                        control_rows: spec.rows,
                        extra_px: spec.extra_px,
                        rect,
                    }
                })
        })
        .collect()
}

#[allow(dead_code)]
fn native_control(
    section: &SettingsNativeSectionSummary,
    control_index: usize,
    key: &'static str,
    label: &'static str,
    kind: SettingsNativeControlKind,
) -> SettingsNativeControlSummary {
    SettingsNativeControlSummary {
        page: section.page,
        page_label: section.page_label,
        section_index: section.section_index,
        section_title: section.section_title,
        control_index,
        key,
        label,
        kind,
        route: native_control_route_for_key(key),
        binding: native_control_binding_for_key(key),
    }
}

#[allow(dead_code)]
fn native_setting_binding(field_name: &'static str) -> Option<SettingsNativeControlBinding> {
    Some(SettingsNativeControlBinding {
        kind: SettingsNativeControlBindingKind::SettingField,
        binding_name: field_name,
        field_name: Some(field_name),
        collect_required: true,
        apply_required: true,
    })
}

#[allow(dead_code)]
fn native_runtime_list_binding(binding_name: &'static str) -> Option<SettingsNativeControlBinding> {
    Some(SettingsNativeControlBinding {
        kind: SettingsNativeControlBindingKind::RuntimeList,
        binding_name,
        field_name: None,
        collect_required: false,
        apply_required: true,
    })
}

#[allow(dead_code)]
fn native_derived_binding(binding_name: &'static str) -> Option<SettingsNativeControlBinding> {
    Some(SettingsNativeControlBinding {
        kind: SettingsNativeControlBindingKind::DerivedValue,
        binding_name,
        field_name: None,
        collect_required: false,
        apply_required: true,
    })
}

#[allow(dead_code)]
fn native_control_binding_for_key(key: &str) -> Option<SettingsNativeControlBinding> {
    match key {
        "auto_start" => native_setting_binding("auto_start"),
        "silent_start" => native_setting_binding("silent_start"),
        "tray_icon" => native_setting_binding("tray_icon_enabled"),
        "capture_enable" => native_setting_binding("clipboard_capture_enabled"),
        "close_to_tray" => native_setting_binding("close_without_exit"),
        "auto_hide_on_blur" => native_setting_binding("auto_hide_on_blur"),
        "edge_auto_hide" => native_setting_binding("edge_auto_hide"),
        "hover_preview" => native_setting_binding("hover_preview"),
        "vv_mode" => native_setting_binding("vv_mode_enabled"),
        "image_preview" => native_setting_binding("image_preview_enabled"),
        "quick_delete" => native_setting_binding("quick_delete_button"),
        "max_items" => native_setting_binding("max_items"),
        "click_hide" => native_setting_binding("click_hide"),
        "paste_move_top" => native_setting_binding("move_pasted_item_to_top"),
        "dedupe_filter" => native_setting_binding("dedupe_filter_enabled"),
        "persistent_search" => native_setting_binding("persistent_search_box"),
        "paste_sound" => native_setting_binding("paste_success_sound_enabled"),
        "paste_sound_kind" => native_setting_binding("paste_success_sound_kind"),
        "skip_window" => native_setting_binding("paste_target_skip_enabled"),
        "skip_window_classes" => native_setting_binding("paste_target_skip_class_names"),
        "position_mode" => native_setting_binding("show_pos_mode"),
        "mouse_offset" => native_setting_binding("show_mouse_dx_dy"),
        "fixed_position" => native_setting_binding("show_fixed_x_y"),
        "hotkey_enable" => native_setting_binding("hotkey_enabled"),
        "hotkey_modifier" => native_setting_binding("hotkey_mod"),
        "hotkey_key" => native_setting_binding("hotkey_key"),
        "hotkey_preview" => native_derived_binding("hotkey_preview_text"),
        "plain_hotkey_enable" => native_setting_binding("plain_paste_hotkey_enabled"),
        "plain_hotkey_modifier" => native_setting_binding("plain_paste_hotkey_mod"),
        "plain_hotkey_key" => native_setting_binding("plain_paste_hotkey_key"),
        "plain_hotkey_preview" => native_derived_binding("plain_hotkey_preview_text"),
        "plugin_search" => native_setting_binding("quick_search_enabled"),
        "search_engine" => native_setting_binding("search_engine"),
        "ocr_provider" => native_setting_binding("image_ocr_provider"),
        "ocr_cloud_url" => native_setting_binding("image_ocr_cloud_url_or_wechat_dir"),
        "ocr_cloud_token" => native_setting_binding("image_ocr_cloud_token"),
        "translate_provider" => native_setting_binding("text_translate_provider"),
        "translate_app_id" => native_setting_binding("text_translate_app_id"),
        "translate_secret" => native_setting_binding("text_translate_secret"),
        "translate_target" => native_setting_binding("text_translate_target_lang"),
        "plugin_ai_clean" => native_setting_binding("ai_text_clean_enabled"),
        "plugin_super_mail_merge" => native_setting_binding("super_mail_merge_enabled"),
        "plugin_wps_taskpane" => native_setting_binding("wps_taskpane_enabled"),
        "plugin_qr_quick" => native_setting_binding("quick_qr_enabled"),
        "group_enable" => native_setting_binding("grouping_enabled"),
        "group_type_filter" => native_setting_binding("group_type_filter_enabled"),
        "vv_source" => native_setting_binding("vv_source_tab"),
        "vv_group" => native_setting_binding("vv_group_id"),
        "group_list" => native_runtime_list_binding("clip_groups"),
        "group_name" => native_derived_binding("selected_group_name"),
        "multi_sync_mode" => native_setting_binding("multi_sync_mode"),
        "cloud_enable" => native_setting_binding("cloud_sync_enabled"),
        "lan_enable" => native_setting_binding("lan_sync_enabled"),
        "cloud_webdav_url" => native_setting_binding("cloud_webdav_url"),
        "cloud_webdav_user" => native_setting_binding("cloud_webdav_user"),
        "cloud_webdav_pass" => native_setting_binding("cloud_webdav_pass"),
        "cloud_remote_dir" => native_setting_binding("cloud_remote_dir"),
        "cloud_sync_interval" => native_setting_binding("cloud_sync_interval"),
        "cloud_status" => native_derived_binding("cloud_sync_status"),
        "lan_status" => native_derived_binding("lan_sync_status"),
        "lan_device_name" => native_setting_binding("lan_device_name"),
        "lan_tcp_port" => native_setting_binding("lan_tcp_port"),
        "lan_receive_mode" => native_setting_binding("lan_receive_mode"),
        "lan_manual_host" => native_setting_binding("lan_manual_host"),
        "lan_discovered_list" => native_runtime_list_binding("lan_discovered_devices"),
        "lan_trusted_summary" => native_derived_binding("lan_trusted_devices_summary"),
        "lan_android_qr" => native_derived_binding("lan_android_pair_qr_payload"),
        "lan_ios_qr" => native_derived_binding("lan_ios_setup_qr_payload"),
        "about_version" => native_derived_binding("app_version"),
        "data_directory" => native_derived_binding("data_directory"),
        _ => None,
    }
}

#[allow(dead_code)]
fn native_command_route(
    command_id: &'static str,
    control_id: i64,
) -> Option<SettingsNativeControlRoute> {
    Some(SettingsNativeControlRoute {
        kind: SettingsNativeControlRouteKind::Command,
        route_name: "settings_command",
        command_id: Some(command_id),
        control_id: Some(control_id),
        action_name: None,
    })
}

#[allow(dead_code)]
fn native_action_route(
    route_name: &'static str,
    action_name: &'static str,
) -> Option<SettingsNativeControlRoute> {
    Some(SettingsNativeControlRoute {
        kind: SettingsNativeControlRouteKind::Action,
        route_name,
        command_id: None,
        control_id: None,
        action_name: Some(action_name),
    })
}

#[allow(dead_code)]
fn native_toggle_route(control_id: i64) -> Option<SettingsNativeControlRoute> {
    native_command_route(command_ids::TOGGLE_SETTINGS_CONTROL.0, control_id)
}

#[allow(dead_code)]
fn native_dropdown_route(control_id: i64) -> Option<SettingsNativeControlRoute> {
    native_command_route(command_ids::OPEN_SETTINGS_DROPDOWN.0, control_id)
}

#[allow(dead_code)]
fn native_control_route_for_key(key: &str) -> Option<SettingsNativeControlRoute> {
    match key {
        "auto_start" => native_toggle_route(5010),
        "silent_start" => native_toggle_route(5059),
        "tray_icon" => native_toggle_route(5060),
        "capture_enable" => native_toggle_route(5101),
        "close_to_tray" => native_toggle_route(5011),
        "auto_hide_on_blur" => native_toggle_route(5061),
        "edge_auto_hide" => native_toggle_route(5013),
        "hover_preview" => native_toggle_route(5014),
        "vv_mode" => native_toggle_route(5054),
        "image_preview" => native_toggle_route(5051),
        "quick_delete" => native_toggle_route(5052),
        "max_items" => native_dropdown_route(5015),
        "click_hide" => native_toggle_route(5038),
        "paste_move_top" => native_toggle_route(5063),
        "dedupe_filter" => native_toggle_route(5064),
        "persistent_search" => native_toggle_route(5069),
        "paste_sound" => native_toggle_route(5070),
        "paste_sound_kind" => native_dropdown_route(5071),
        "paste_sound_file" => native_action_route("settings_platform", "pick_paste_sound"),
        "skip_window" => native_toggle_route(6201),
        "capture_skip_window" => {
            native_action_route("settings_platform", "capture_skipped_window_class")
        }
        "position_mode" => native_dropdown_route(5016),
        "open_config" => native_command_route(command_ids::OPEN_SETTINGS_CONFIG.0, 5021),
        "hotkey_enable" => native_toggle_route(6101),
        "hotkey_modifier" => native_dropdown_route(6102),
        "hotkey_key" => native_dropdown_route(6103),
        "hotkey_record" => native_action_route("settings_platform", "toggle_hotkey_recording"),
        "plain_hotkey_enable" => native_toggle_route(6108),
        "plain_hotkey_modifier" => native_dropdown_route(6109),
        "plain_hotkey_key" => native_dropdown_route(6110),
        "clipboard_history_disable" => {
            native_action_route("settings_platform", "disable_system_clipboard_history")
        }
        "clipboard_history_enable" => {
            native_action_route("settings_platform", "enable_system_clipboard_history")
        }
        "restart_shell" => native_action_route("settings_platform", "restart_system_shell"),
        "plugin_search" => native_toggle_route(7101),
        "search_engine" => native_dropdown_route(7201),
        "search_engine_reset" => {
            native_action_route("settings_platform", "restore_search_engine_preset")
        }
        "ocr_provider" => native_dropdown_route(5065),
        "ocr_wechat_detect" => native_action_route("settings_platform", "detect_ocr_runtime"),
        "translate_provider" => native_dropdown_route(5075),
        "translate_target" => native_dropdown_route(5078),
        "plugin_ai_clean" => native_toggle_route(7102),
        "plugin_super_mail_merge" => native_toggle_route(7106),
        "plugin_mail_merge" => native_action_route("settings_platform", "open_mail_merge"),
        "plugin_wps_taskpane" => native_toggle_route(7104),
        "wps_taskpane_docs" => native_action_route("settings_platform", "open_wps_taskpane_docs"),
        "plugin_qr_quick" => native_toggle_route(7103),
        "group_enable" => native_toggle_route(5030),
        "group_type_filter" => native_toggle_route(5031),
        "vv_source" => native_dropdown_route(5056),
        "vv_group" => native_dropdown_route(5055),
        "group_view_records" => native_action_route("settings_group", "show_record_groups"),
        "group_view_phrases" => native_action_route("settings_group", "show_phrase_groups"),
        "group_add" => native_action_route("settings_group", "add_group"),
        "group_rename" => native_action_route("settings_group", "rename_group"),
        "group_delete" => native_action_route("settings_group", "delete_group"),
        "group_up" => native_action_route("settings_group", "move_group_up"),
        "group_down" => native_action_route("settings_group", "move_group_down"),
        "multi_sync_mode" => native_dropdown_route(5073),
        "cloud_enable" => native_toggle_route(5040),
        "lan_enable" => native_toggle_route(5080),
        "cloud_sync_interval" => native_dropdown_route(5041),
        "cloud_sync_now" => native_action_route("settings_sync", "sync_webdav_now"),
        "cloud_upload_config" => native_action_route("settings_sync", "upload_webdav_config"),
        "cloud_apply_config" => native_action_route("settings_sync", "apply_webdav_config"),
        "cloud_restore_backup" => native_action_route("settings_sync", "restore_webdav_backup"),
        "lan_receive_mode" => native_dropdown_route(5092),
        "lan_pair" => native_action_route("settings_sync", "pair_lan_device"),
        "lan_refresh" => native_action_route("settings_sync", "refresh_lan_devices"),
        "lan_accept_pair" => native_action_route("settings_sync", "accept_lan_pairing"),
        "lan_reject_pair" => native_action_route("settings_sync", "reject_lan_pairing"),
        "lan_copy_pair" => native_action_route("settings_sync", "copy_lan_pair_url"),
        "lan_copy_setup" => native_action_route("settings_sync", "copy_lan_setup_url"),
        "lan_docs" => native_action_route("settings_sync", "open_lan_setup_page"),
        "open_source" => native_action_route("settings_platform", "open_source_repository"),
        "check_updates" => native_action_route("settings_platform", "check_for_updates"),
        _ => None,
    }
}

#[allow(dead_code)]
fn push_native_controls(
    controls: &mut Vec<SettingsNativeControlSummary>,
    sections: &[SettingsNativeSectionSummary],
    page: SettingsPage,
    section_index: usize,
    specs: &[(&'static str, &'static str, SettingsNativeControlKind)],
) {
    let Some(section) = sections
        .iter()
        .find(|section| section.page == page && section.section_index == section_index)
    else {
        return;
    };
    controls.extend(
        specs
            .iter()
            .enumerate()
            .map(|(index, (key, label, kind))| native_control(section, index, key, label, *kind)),
    );
}

#[allow(dead_code)]
pub fn settings_native_control_summaries() -> Vec<SettingsNativeControlSummary> {
    use SettingsNativeControlKind::*;

    let sections = settings_native_section_summaries();
    let mut controls = Vec::new();

    push_native_controls(
        &mut controls,
        &sections,
        SettingsPage::General,
        0,
        &[
            ("auto_start", "开机自启", Toggle),
            ("silent_start", "静默启动", Toggle),
            ("tray_icon", "右下角图标", Toggle),
            ("capture_enable", "剪贴板捕获", Toggle),
            ("close_to_tray", "关闭不退出", Toggle),
            ("auto_hide_on_blur", "点击外部隐藏", Toggle),
            ("edge_auto_hide", "贴边自动隐藏", Toggle),
            ("hover_preview", "悬停预览", Toggle),
            ("vv_mode", "VV 模式", Toggle),
            ("image_preview", "图片缩略图", Toggle),
            ("quick_delete", "快速删除按钮", Toggle),
        ],
    );
    push_native_controls(
        &mut controls,
        &sections,
        SettingsPage::General,
        1,
        &[("max_items", "最大保存条数", Dropdown)],
    );
    push_native_controls(
        &mut controls,
        &sections,
        SettingsPage::General,
        2,
        &[
            ("click_hide", "单击后隐藏主窗口", Toggle),
            ("paste_move_top", "粘贴后上移到首行", Toggle),
            ("dedupe_filter", "重复内容过滤", Toggle),
            ("persistent_search", "常驻搜索框", Toggle),
            ("paste_sound", "粘贴成功声音", Toggle),
            ("paste_sound_kind", "提示音", Dropdown),
            ("paste_sound_file", "声音文件", Button),
            ("skip_window", "焦点窗口跳过", Toggle),
            ("skip_window_classes", "跳过窗口类名", TextInput),
            ("capture_skip_window", "捕获当前窗口", Button),
        ],
    );
    push_native_controls(
        &mut controls,
        &sections,
        SettingsPage::General,
        3,
        &[
            ("position_mode", "弹出位置", Dropdown),
            ("mouse_offset", "鼠标偏移 dx/dy", TextInput),
            ("fixed_position", "固定位置 x/y", TextInput),
        ],
    );
    push_native_controls(
        &mut controls,
        &sections,
        SettingsPage::General,
        4,
        &[("open_config", "打开设置文件", Button)],
    );

    push_native_controls(
        &mut controls,
        &sections,
        SettingsPage::Hotkey,
        0,
        &[
            ("hotkey_enable", "启用快捷键", Toggle),
            ("hotkey_modifier", "修饰键", Dropdown),
            ("hotkey_key", "按键", Dropdown),
            ("hotkey_preview", "当前快捷键预览", Label),
            ("hotkey_record", "录制热键", Button),
            ("plain_hotkey_enable", "启用纯文本粘贴快捷键", Toggle),
            ("plain_hotkey_modifier", "纯文本修饰键", Dropdown),
            ("plain_hotkey_key", "纯文本按键", Dropdown),
            ("plain_hotkey_preview", "纯文本快捷键预览", Label),
        ],
    );
    push_native_controls(
        &mut controls,
        &sections,
        SettingsPage::Hotkey,
        1,
        &[
            ("clipboard_history_disable", "禁用系统剪贴板历史", Button),
            ("clipboard_history_enable", "启用系统剪贴板历史", Button),
            ("restart_shell", "重启系统外壳", Button),
        ],
    );
    push_native_controls(
        &mut controls,
        &sections,
        SettingsPage::Hotkey,
        2,
        &[
            ("hotkey_note_main", "快捷键说明", Label),
            ("hotkey_note_plain", "纯文本粘贴说明", Label),
        ],
    );

    push_native_controls(
        &mut controls,
        &sections,
        SettingsPage::Plugin,
        0,
        &[
            ("plugin_search", "搜索插件", Toggle),
            ("search_engine", "搜索引擎", Dropdown),
            ("search_engine_reset", "恢复预设", Button),
        ],
    );
    push_native_controls(
        &mut controls,
        &sections,
        SettingsPage::Plugin,
        1,
        &[
            ("ocr_provider", "OCR 提供方", Dropdown),
            ("ocr_cloud_url", "OCR 地址", TextInput),
            ("ocr_cloud_token", "OCR Token", TextInput),
            ("ocr_wechat_detect", "检测 WeChat OCR", Button),
        ],
    );
    push_native_controls(
        &mut controls,
        &sections,
        SettingsPage::Plugin,
        2,
        &[
            ("translate_provider", "翻译提供方", Dropdown),
            ("translate_app_id", "翻译 App ID", TextInput),
            ("translate_secret", "翻译密钥", TextInput),
            ("translate_target", "目标语言", Dropdown),
        ],
    );
    push_native_controls(
        &mut controls,
        &sections,
        SettingsPage::Plugin,
        3,
        &[("plugin_ai_clean", "AI 文本清洗", Toggle)],
    );
    push_native_controls(
        &mut controls,
        &sections,
        SettingsPage::Plugin,
        4,
        &[
            ("plugin_super_mail_merge", "超级邮件合并", Toggle),
            ("plugin_mail_merge", "打开邮件合并", Button),
        ],
    );
    push_native_controls(
        &mut controls,
        &sections,
        SettingsPage::Plugin,
        5,
        &[
            ("plugin_wps_taskpane", "WPS 任务窗格", Toggle),
            ("wps_taskpane_docs", "WPS 文档", Button),
        ],
    );
    push_native_controls(
        &mut controls,
        &sections,
        SettingsPage::Plugin,
        6,
        &[("plugin_qr_quick", "快捷二维码转换", Toggle)],
    );

    push_native_controls(
        &mut controls,
        &sections,
        SettingsPage::Group,
        0,
        &[
            ("group_enable", "启用分组", Toggle),
            ("group_type_filter", "文件类型选项", Toggle),
            ("vv_source", "VV 来源", Dropdown),
            ("vv_group", "VV 分组", Dropdown),
        ],
    );
    push_native_controls(
        &mut controls,
        &sections,
        SettingsPage::Group,
        1,
        &[
            ("group_view_records", "复制记录分组", Button),
            ("group_view_phrases", "常用短语分组", Button),
            ("group_list", "分组列表", List),
            ("group_name", "分组名称", TextInput),
            ("group_add", "新建分组", Button),
            ("group_rename", "重命名", Button),
            ("group_delete", "删除", Button),
            ("group_up", "上移", Button),
            ("group_down", "下移", Button),
        ],
    );

    push_native_controls(
        &mut controls,
        &sections,
        SettingsPage::Cloud,
        0,
        &[
            ("multi_sync_mode", "多端同步模式", Dropdown),
            ("cloud_enable", "WebDAV 同步", Toggle),
            ("lan_enable", "局域网同步", Toggle),
        ],
    );
    push_native_controls(
        &mut controls,
        &sections,
        SettingsPage::Cloud,
        1,
        &[
            ("cloud_webdav_url", "WebDAV 地址", TextInput),
            ("cloud_webdav_user", "用户名", TextInput),
            ("cloud_webdav_pass", "密码", TextInput),
            ("cloud_remote_dir", "远程目录", TextInput),
            ("cloud_sync_interval", "同步间隔", Dropdown),
            ("cloud_status", "上次同步", Label),
        ],
    );
    push_native_controls(
        &mut controls,
        &sections,
        SettingsPage::Cloud,
        2,
        &[
            ("cloud_sync_now", "立即同步", Button),
            ("cloud_upload_config", "上传配置", Button),
            ("cloud_apply_config", "应用云端配置", Button),
            ("cloud_restore_backup", "云备份恢复", Button),
        ],
    );
    push_native_controls(
        &mut controls,
        &sections,
        SettingsPage::Cloud,
        3,
        &[
            ("lan_status", "局域网状态", Label),
            ("lan_device_name", "设备名称", TextInput),
            ("lan_tcp_port", "TCP 端口", TextInput),
            ("lan_receive_mode", "同步方式", Dropdown),
            ("lan_receive_note", "接收方式说明", Label),
            ("lan_qr_note", "扫码绑定说明", Label),
        ],
    );
    push_native_controls(
        &mut controls,
        &sections,
        SettingsPage::Cloud,
        4,
        &[
            ("lan_manual_host", "手动 IP", TextInput),
            ("lan_pair", "配对选中设备", Button),
            ("lan_refresh", "刷新发现", Button),
            ("lan_accept_pair", "允许配对", Button),
            ("lan_reject_pair", "拒绝", Button),
            ("lan_device_note", "附近设备说明", Label),
            ("lan_discovered_list", "附近设备 / 待允许请求", List),
        ],
    );
    push_native_controls(
        &mut controls,
        &sections,
        SettingsPage::Cloud,
        5,
        &[
            ("lan_trusted_summary", "信任设备", Label),
            ("lan_bind_note", "绑定说明", Label),
            ("lan_android_qr", "Android 配对", Label),
            ("lan_copy_pair", "复制配对链接", Button),
            ("lan_ios_qr", "iOS/浏览器入口", Label),
            ("lan_copy_setup", "复制入口地址", Button),
            ("lan_docs", "打开扫码绑定页", Button),
        ],
    );

    push_native_controls(
        &mut controls,
        &sections,
        SettingsPage::About,
        0,
        &[
            ("about_version", "版本信息", Label),
            ("open_source", "打开源码仓库", Button),
            ("check_updates", "检查更新", Button),
            ("data_directory", "数据目录", Label),
        ],
    );

    controls
}

#[allow(dead_code)]
pub fn settings_native_apply_collect_plan() -> SettingsNativeApplyCollectPlan {
    let controls = settings_native_control_summaries();
    let mut collect_bindings = Vec::new();
    let mut apply_bindings = Vec::new();
    let mut command_route_count = 0;
    let mut action_route_count = 0;
    let mut bound_control_count = 0;

    for control in controls {
        match control.route {
            Some(route) if route.kind == SettingsNativeControlRouteKind::Command => {
                command_route_count += 1;
            }
            Some(route) if route.kind == SettingsNativeControlRouteKind::Action => {
                action_route_count += 1;
            }
            _ => {}
        }
        if let Some(binding) = control.binding {
            bound_control_count += 1;
            let entry = SettingsNativeApplyCollectBinding {
                page: control.page,
                page_label: control.page_label,
                control_key: control.key,
                control_label: control.label,
                binding,
            };
            if binding.collect_required {
                collect_bindings.push(entry);
            }
            if binding.apply_required {
                apply_bindings.push(entry);
            }
        }
    }

    SettingsNativeApplyCollectPlan {
        collect_bindings,
        apply_bindings,
        command_route_count,
        action_route_count,
        bound_control_count,
    }
}

#[allow(dead_code)]
pub fn settings_native_collect_submission(
    submitted_values: &[SettingsNativeSubmittedControlValue],
) -> SettingsNativeCollectSubmission {
    let plan = settings_native_apply_collect_plan();
    let mut applied_fields = Vec::new();
    let mut missing_control_keys = Vec::new();

    for collect_binding in &plan.collect_bindings {
        let Some(submitted_value) = submitted_values
            .iter()
            .find(|value| value.control_key == collect_binding.control_key)
        else {
            missing_control_keys.push(collect_binding.control_key);
            continue;
        };
        if collect_binding.binding.kind != SettingsNativeControlBindingKind::SettingField {
            continue;
        }
        let Some(field_name) = collect_binding.binding.field_name else {
            continue;
        };
        applied_fields.push(SettingsNativeAppliedField {
            control_key: submitted_value.control_key.clone(),
            control_label: collect_binding.control_label,
            field_name,
            value: submitted_value.raw_value.trim().to_string(),
        });
    }

    let ignored_control_keys = submitted_values
        .iter()
        .filter(|submitted_value| {
            !plan
                .collect_bindings
                .iter()
                .any(|binding| binding.control_key == submitted_value.control_key)
        })
        .map(|submitted_value| submitted_value.control_key.clone())
        .collect();

    SettingsNativeCollectSubmission {
        applied_fields,
        missing_control_keys,
        ignored_control_keys,
    }
}

#[allow(dead_code)]
fn settings_native_json_bool_value(raw_value: &str) -> Option<serde_json::Value> {
    let value = raw_value.trim();
    if value.eq_ignore_ascii_case("true")
        || value == "1"
        || value.eq_ignore_ascii_case("on")
        || value.eq_ignore_ascii_case("yes")
        || value == "开启"
        || value == "启用"
    {
        Some(serde_json::Value::Bool(true))
    } else if value.eq_ignore_ascii_case("false")
        || value == "0"
        || value.eq_ignore_ascii_case("off")
        || value.eq_ignore_ascii_case("no")
        || value == "关闭"
        || value == "禁用"
    {
        Some(serde_json::Value::Bool(false))
    } else {
        None
    }
}

#[allow(dead_code)]
fn settings_native_json_number_value<T>(raw_value: &str) -> Option<serde_json::Value>
where
    T: std::str::FromStr + Into<serde_json::Number>,
{
    raw_value
        .trim()
        .parse::<T>()
        .ok()
        .map(|value| serde_json::Value::Number(value.into()))
}

#[allow(dead_code)]
fn settings_native_json_updates_for_applied_field(
    field: &SettingsNativeAppliedField,
) -> Option<Vec<SettingsNativeJsonFieldUpdate>> {
    let value = field.value.trim();
    let update = |field_name: &str, value: serde_json::Value| SettingsNativeJsonFieldUpdate {
        field_name: field_name.to_string(),
        value,
    };

    match field.field_name {
        "cloud_webdav_pass"
        | "image_ocr_cloud_url"
        | "image_ocr_cloud_token"
        | "image_ocr_cloud_url_or_wechat_dir"
        | "text_translate_app_id"
        | "text_translate_secret" => None,
        "multi_sync_mode" => {
            let (cloud_enabled, lan_enabled) =
                multi_sync_flags_for_mode(multi_sync_mode_from_label(value));
            Some(vec![
                update("cloud_sync_enabled", serde_json::Value::Bool(cloud_enabled)),
                update("lan_sync_enabled", serde_json::Value::Bool(lan_enabled)),
            ])
        }
        "show_mouse_dx_dy" => {
            let mut parts = value
                .split(|ch: char| ch == ',' || ch == ';' || ch.is_whitespace())
                .filter(|part| !part.trim().is_empty());
            let dx = parts.next()?.trim().parse::<i32>().ok()?;
            let dy = parts.next()?.trim().parse::<i32>().ok()?;
            Some(vec![
                update("show_mouse_dx", serde_json::Value::Number(dx.into())),
                update("show_mouse_dy", serde_json::Value::Number(dy.into())),
            ])
        }
        "show_fixed_x_y" => {
            let mut parts = value
                .split(|ch: char| ch == ',' || ch == ';' || ch.is_whitespace())
                .filter(|part| !part.trim().is_empty());
            let x = parts.next()?.trim().parse::<i32>().ok()?;
            let y = parts.next()?.trim().parse::<i32>().ok()?;
            Some(vec![
                update("show_fixed_x", serde_json::Value::Number(x.into())),
                update("show_fixed_y", serde_json::Value::Number(y.into())),
            ])
        }
        "ai_text_clean_enabled" => settings_native_json_bool_value(value)
            .map(|json_value| vec![update("ai_clean_enabled", json_value)]),
        "quick_qr_enabled" => settings_native_json_bool_value(value)
            .map(|json_value| vec![update("qr_quick_enabled", json_value)]),
        "auto_start"
        | "silent_start"
        | "tray_icon_enabled"
        | "clipboard_capture_enabled"
        | "close_without_exit"
        | "auto_hide_on_blur"
        | "edge_auto_hide"
        | "hover_preview"
        | "vv_mode_enabled"
        | "image_preview_enabled"
        | "quick_delete_button"
        | "click_hide"
        | "move_pasted_item_to_top"
        | "dedupe_filter_enabled"
        | "persistent_search_box"
        | "paste_success_sound_enabled"
        | "paste_target_skip_enabled"
        | "hotkey_enabled"
        | "plain_paste_hotkey_enabled"
        | "quick_search_enabled"
        | "super_mail_merge_enabled"
        | "wps_taskpane_enabled"
        | "grouping_enabled"
        | "cloud_sync_enabled"
        | "lan_sync_enabled" => settings_native_json_bool_value(value)
            .map(|json_value| vec![update(field.field_name, json_value)]),
        "max_items" => value.parse::<usize>().ok().map(|number| {
            vec![update(
                field.field_name,
                serde_json::Value::Number((number as u64).into()),
            )]
        }),
        "vv_source_tab" => value.parse::<usize>().ok().map(|number| {
            vec![update(
                field.field_name,
                serde_json::Value::Number((number as u64).into()),
            )]
        }),
        "vv_group_id" => value.parse::<i64>().ok().map(|number| {
            vec![update(
                field.field_name,
                serde_json::Value::Number(number.into()),
            )]
        }),
        "lan_tcp_port" | "lan_udp_port" => value.parse::<u16>().ok().map(|number| {
            vec![update(
                field.field_name,
                serde_json::Value::Number((number as u64).into()),
            )]
        }),
        _ => Some(vec![update(
            field.field_name,
            serde_json::Value::String(value.to_string()),
        )]),
    }
}

#[allow(dead_code)]
pub fn settings_native_apply_submission_to_json(
    settings_json: serde_json::Value,
    submission: &SettingsNativeCollectSubmission,
) -> SettingsNativeJsonApplyResult {
    let mut settings_json = match settings_json {
        serde_json::Value::Object(_) => settings_json,
        _ => serde_json::Value::Object(serde_json::Map::new()),
    };
    let mut field_updates = Vec::new();
    let mut rejected_fields = Vec::new();

    for field in &submission.applied_fields {
        let Some(updates) = settings_native_json_updates_for_applied_field(field) else {
            rejected_fields.push(field.field_name.to_string());
            continue;
        };
        for update in updates {
            if let Some(object) = settings_json.as_object_mut() {
                object.insert(update.field_name.clone(), update.value.clone());
            }
            field_updates.push(update);
        }
    }

    SettingsNativeJsonApplyResult {
        settings_json,
        field_updates,
        rejected_fields,
    }
}

#[allow(dead_code)]
pub fn settings_native_bool_field_update(
    field_updates: &[SettingsNativeJsonFieldUpdate],
    field_name: &str,
) -> Option<bool> {
    field_updates
        .iter()
        .find(|update| update.field_name == field_name)
        .and_then(|update| update.value.as_bool())
}

#[allow(dead_code)]
fn settings_native_json_is_sensitive_field(field_name: &str) -> bool {
    matches!(
        field_name,
        "cloud_webdav_pass"
            | "image_ocr_cloud_url"
            | "image_ocr_cloud_token"
            | "image_ocr_cloud_url_or_wechat_dir"
            | "text_translate_app_id"
            | "text_translate_secret"
    )
}

#[allow(dead_code)]
fn settings_native_json_value_to_display(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Null => String::new(),
        _ => value.to_string(),
    }
}

#[allow(dead_code)]
fn settings_native_json_field_value(
    settings_json: &serde_json::Value,
    field_name: &str,
) -> Option<String> {
    let object = settings_json.as_object()?;
    match field_name {
        "multi_sync_mode" => {
            let cloud_sync_enabled = object
                .get("cloud_sync_enabled")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or_default();
            let lan_sync_enabled = object
                .get("lan_sync_enabled")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or_default();
            Some(
                multi_sync_mode_display(multi_sync_mode_from_flags(
                    cloud_sync_enabled,
                    lan_sync_enabled,
                ))
                .to_string(),
            )
        }
        "show_mouse_dx_dy" => {
            let dx = object
                .get("show_mouse_dx")
                .map(settings_native_json_value_to_display)
                .unwrap_or_default();
            let dy = object
                .get("show_mouse_dy")
                .map(settings_native_json_value_to_display)
                .unwrap_or_default();
            Some(format!("{dx} {dy}").trim().to_string())
        }
        "show_fixed_x_y" => {
            let x = object
                .get("show_fixed_x")
                .map(settings_native_json_value_to_display)
                .unwrap_or_default();
            let y = object
                .get("show_fixed_y")
                .map(settings_native_json_value_to_display)
                .unwrap_or_default();
            Some(format!("{x} {y}").trim().to_string())
        }
        "ai_text_clean_enabled" => object
            .get("ai_clean_enabled")
            .map(settings_native_json_value_to_display),
        "quick_qr_enabled" => object
            .get("qr_quick_enabled")
            .map(settings_native_json_value_to_display),
        "image_ocr_cloud_url_or_wechat_dir" => object
            .get("image_ocr_cloud_url")
            .map(settings_native_json_value_to_display),
        _ => object
            .get(field_name)
            .map(settings_native_json_value_to_display),
    }
}

#[allow(dead_code)]
pub fn settings_native_control_display_value(
    control: &SettingsNativeControlSummary,
    settings_json: &serde_json::Value,
) -> Option<SettingsNativeControlDisplayValue> {
    let binding = control.binding?;
    let field_name = binding.field_name?;
    let sensitive = settings_native_json_is_sensitive_field(field_name);
    if sensitive {
        return Some(SettingsNativeControlDisplayValue {
            control_key: control.key,
            value: String::new(),
            sensitive,
        });
    }
    settings_native_json_field_value(settings_json, field_name).map(|value| {
        SettingsNativeControlDisplayValue {
            control_key: control.key,
            value,
            sensitive,
        }
    })
}

#[allow(dead_code)]
fn native_dropdown_options_from_pairs<const N: usize>(
    control: &SettingsNativeControlSummary,
    settings_json: &serde_json::Value,
    pairs: [(&str, String); N],
) -> Option<SettingsNativeDropdownOptions> {
    let display = settings_native_control_display_value(control, settings_json)?;
    let options = pairs
        .into_iter()
        .map(|(raw_value, label)| SettingsNativeDropdownOption {
            raw_value: raw_value.to_string(),
            label,
        })
        .collect::<Vec<_>>();
    let selected_index = options
        .iter()
        .position(|option| option.raw_value == display.value || option.label == display.value)
        .unwrap_or(0);
    Some(SettingsNativeDropdownOptions {
        control_key: control.key,
        options,
        selected_index,
    })
}

#[allow(dead_code)]
pub fn settings_native_dropdown_options(
    control: &SettingsNativeControlSummary,
    settings_json: &serde_json::Value,
) -> Option<SettingsNativeDropdownOptions> {
    if control.kind != SettingsNativeControlKind::Dropdown {
        return None;
    }
    match control.key {
        "max_items" => native_dropdown_options_from_pairs(
            control,
            settings_json,
            [
                ("100", "100".to_string()),
                ("200", "200".to_string()),
                ("500", "500".to_string()),
                ("1000", "1000".to_string()),
                ("3000", "3000".to_string()),
                ("0", settings_dropdown_label_for_max_items(0).to_string()),
            ],
        ),
        "paste_sound_kind" => native_dropdown_options_from_pairs(
            control,
            settings_json,
            PASTE_SOUND_OPTIONS.map(|(key, label)| (key, translate(label).into_owned())),
        ),
        "position_mode" => native_dropdown_options_from_pairs(
            control,
            settings_json,
            [
                (
                    "mouse",
                    settings_dropdown_label_for_pos_mode("mouse").to_string(),
                ),
                (
                    "fixed",
                    settings_dropdown_label_for_pos_mode("fixed").to_string(),
                ),
                (
                    "last",
                    settings_dropdown_label_for_pos_mode("last").to_string(),
                ),
            ],
        ),
        "hotkey_modifier" | "plain_hotkey_modifier" => native_dropdown_options_from_pairs(
            control,
            settings_json,
            HOTKEY_MOD_OPTIONS.map(|label| (label, label.to_string())),
        ),
        "hotkey_key" | "plain_hotkey_key" => native_dropdown_options_from_pairs(
            control,
            settings_json,
            HOTKEY_KEY_OPTIONS.map(|label| (label, label.to_string())),
        ),
        "search_engine" => native_dropdown_options_from_pairs(
            control,
            settings_json,
            SEARCH_ENGINE_PRESETS.map(|(key, label, _)| (key, translate(label).into_owned())),
        ),
        "ocr_provider" => native_dropdown_options_from_pairs(
            control,
            settings_json,
            IMAGE_OCR_PROVIDER_OPTIONS.map(|(key, label)| (key, translate(label).into_owned())),
        ),
        "translate_provider" => native_dropdown_options_from_pairs(
            control,
            settings_json,
            TEXT_TRANSLATE_PROVIDER_OPTIONS
                .map(|(key, label)| (key, translate(label).into_owned())),
        ),
        "translate_target" => native_dropdown_options_from_pairs(
            control,
            settings_json,
            TEXT_TRANSLATE_TARGET_OPTIONS.map(|(key, label)| (key, translate(label).into_owned())),
        ),
        "vv_source" => native_dropdown_options_from_pairs(
            control,
            settings_json,
            [
                ("0", source_tab_label(0).to_string()),
                ("1", source_tab_label(1).to_string()),
            ],
        ),
        "multi_sync_mode" => native_dropdown_options_from_pairs(
            control,
            settings_json,
            [
                ("off", multi_sync_mode_display("off").to_string()),
                ("webdav", multi_sync_mode_display("webdav").to_string()),
                ("lan", multi_sync_mode_display("lan").to_string()),
            ],
        ),
        "cloud_sync_interval" => native_dropdown_options_from_pairs(
            control,
            settings_json,
            [
                ("15分钟", "15分钟".to_string()),
                ("30分钟", "30分钟".to_string()),
                ("1小时", "1小时".to_string()),
                ("6小时", "6小时".to_string()),
                ("12小时", "12小时".to_string()),
                ("24小时", "24小时".to_string()),
            ],
        ),
        "lan_receive_mode" => native_dropdown_options_from_pairs(
            control,
            settings_json,
            [
                (
                    "records_only",
                    lan_receive_mode_display("records_only").to_string(),
                ),
                (
                    "clipboard",
                    lan_receive_mode_display("clipboard").to_string(),
                ),
            ],
        ),
        _ => None,
    }
}

#[allow(dead_code)]
pub fn settings_native_vv_source_tab(settings_json: &serde_json::Value) -> usize {
    settings_json
        .as_object()
        .and_then(|object| object.get("vv_source_tab"))
        .map(settings_native_json_value_to_display)
        .and_then(|value| value.trim().parse::<usize>().ok())
        .map(normalize_source_tab)
        .unwrap_or_default()
}

#[allow(dead_code)]
pub fn settings_native_vv_group_dropdown_options<'a, I>(
    control: &SettingsNativeControlSummary,
    settings_json: &serde_json::Value,
    groups: I,
) -> Option<SettingsNativeDropdownOptions>
where
    I: IntoIterator<Item = (i64, &'a str)>,
{
    if control.kind != SettingsNativeControlKind::Dropdown || control.key != "vv_group" {
        return None;
    }
    let display = settings_native_control_display_value(control, settings_json)?;
    let source_tab = settings_native_vv_source_tab(settings_json);
    let mut options = vec![SettingsNativeDropdownOption {
        raw_value: "0".to_string(),
        label: source_tab_all_label(source_tab).to_string(),
    }];
    options.extend(
        groups
            .into_iter()
            .map(|(id, name)| SettingsNativeDropdownOption {
                raw_value: id.to_string(),
                label: name.to_string(),
            }),
    );
    let selected_index = options
        .iter()
        .position(|option| option.raw_value == display.value || option.label == display.value)
        .unwrap_or_default();
    Some(SettingsNativeDropdownOptions {
        control_key: control.key,
        options,
        selected_index,
    })
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

pub fn settings_form_layout_for_section(
    page: usize,
    index: usize,
    label_w: i32,
    dynamic_sections: &[SettingsSection],
) -> SettingsFormSectionLayout {
    let section = dynamic_sections
        .get(index)
        .copied()
        .unwrap_or_else(|| settings_section(page, index));
    SettingsFormSectionLayout::from_section(section, label_w)
}

#[derive(Clone, Copy)]
pub struct SettingsFormSectionLayout {
    body: UiRect,
    label_w: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsQrActionLayout {
    pub qr_rect: UiRect,
    pub action_rect: UiRect,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsToggleRowLayout {
    pub label_rect: UiRect,
    pub toggle_rect: UiRect,
}

pub fn settings_toggle_row_layout_for_rect(
    row_rect: UiRect,
    toggle_w: i32,
    toggle_h: i32,
    gap: i32,
    min_label_w: i32,
) -> SettingsToggleRowLayout {
    let row_w = (row_rect.right - row_rect.left).max(0);
    let row_h = (row_rect.bottom - row_rect.top).max(0);
    let label_w = (row_w - toggle_w - gap).max(min_label_w.max(0));
    let toggle_x = row_rect.right - toggle_w;
    let toggle_y = row_rect.top + ((row_h - toggle_h).max(0) / 2);
    SettingsToggleRowLayout {
        label_rect: UiRect::new(
            row_rect.left,
            row_rect.top,
            row_rect.left + label_w,
            row_rect.bottom,
        ),
        toggle_rect: UiRect::new(toggle_x, toggle_y, toggle_x + toggle_w, toggle_y + toggle_h),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsActionHintLayout {
    pub action_rect: UiRect,
    pub hint_rect: UiRect,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsLabeledFieldLayout {
    pub label_rect: UiRect,
    pub field_rect: UiRect,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsFlowLayout {
    x: i32,
    y: i32,
    width: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SettingsUpdatePresentationInput {
    pub started: bool,
    pub checking: bool,
    pub available: bool,
    pub latest_tag: String,
    pub error: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsUpdatePresentation {
    pub status_text: String,
    pub button_text: String,
}

pub fn settings_update_presentation(
    state: &SettingsUpdatePresentationInput,
) -> SettingsUpdatePresentation {
    let status_text = if state.checking {
        tr("检查更新中…", "Checking for updates...").to_string()
    } else if !state.started {
        tr(
            "点击下方按钮后再检查更新。",
            "Click the button below to check for updates.",
        )
        .to_string()
    } else if state.available {
        format!(
            "{} {}",
            tr("发现新版本：", "New version available: "),
            if state.latest_tag.trim().is_empty() {
                "latest".to_string()
            } else {
                state.latest_tag.clone()
            }
        )
    } else if !state.error.trim().is_empty() {
        format!(
            "{} {}",
            tr("更新检查失败：", "Update check failed: "),
            state.error
        )
    } else {
        tr(
            "当前已经是最新版本。",
            "You are already on the latest version.",
        )
        .to_string()
    };
    let button_text = if state.checking {
        tr("检测中…", "Checking...")
    } else if state.available {
        tr("点击下载最新版本", "Click to download latest version")
    } else if state.started {
        tr("再次检查", "Check again")
    } else {
        tr("检查更新", "Check for updates")
    }
    .to_string();

    SettingsUpdatePresentation {
        status_text,
        button_text,
    }
}

impl SettingsFlowLayout {
    pub fn new(x: i32, y: i32, width: i32) -> Self {
        Self { x, y, width }
    }

    pub fn full_rect(&self, h: i32) -> UiRect {
        UiRect::new(self.x, self.y, self.x + self.width, self.y + h)
    }

    pub fn consume_full(&mut self, h: i32, gap: i32) -> UiRect {
        let rect = self.full_rect(h);
        self.y += h + gap;
        rect
    }

    pub fn row_label_rect(&self, label_w: i32, label_h: i32, offset_y: i32) -> UiRect {
        UiRect::new(
            self.x,
            self.y + offset_y,
            self.x + label_w,
            self.y + offset_y + label_h,
        )
    }

    pub fn row_field_rect(&self, label_w: i32, row_h: i32) -> UiRect {
        UiRect::new(
            self.x + label_w,
            self.y,
            self.x + self.width,
            self.y + row_h,
        )
    }

    pub fn consume_row(&mut self, row_h: i32, gap: i32) {
        self.y += row_h + gap;
    }

    pub fn button_rect(&self, w: i32, h: i32) -> UiRect {
        UiRect::new(self.x, self.y, self.x + w, self.y + h)
    }
}

impl SettingsFormSectionLayout {
    pub fn new(page: usize, index: usize, label_w: i32) -> Self {
        Self {
            body: settings_section_body_rect(page, index, 18),
            label_w: settings_scale(label_w),
        }
    }

    pub fn from_section(section: SettingsSection, label_w: i32) -> Self {
        let pad = settings_scale(18);
        Self {
            body: UiRect::new(
                section.rect.left + pad,
                section.rect.top + settings_scale(SETTINGS_FORM_HEADER_H),
                section.rect.right - pad,
                section.rect.bottom - pad,
            ),
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
    pub fn label_rect(&self, row: i32, h: i32) -> UiRect {
        let y = self.label_y(row, h);
        UiRect::new(self.left(), y, self.left() + self.label_w(), y + h)
    }
    pub fn field_x(&self) -> i32 {
        self.body.left + self.label_w
    }
    pub fn field_w(&self) -> i32 {
        (self.body.right - self.field_x()).max(40)
    }
    pub fn field_rect(&self, row: i32, w: i32, h: i32) -> UiRect {
        let x = self.field_x();
        let y = self.row_y(row);
        UiRect::new(x, y, x + w, y + h)
    }
    pub fn field_full_rect(&self, row: i32, h: i32) -> UiRect {
        self.field_rect(row, self.field_w(), h)
    }
    pub fn field_sized_row_rect(&self, row: i32, w: i32) -> UiRect {
        self.field_rect(row, w, settings_scale(SETTINGS_FORM_ROW_H))
    }
    pub fn field_row_rect(&self, row: i32) -> UiRect {
        self.field_full_rect(row, settings_scale(SETTINGS_FORM_ROW_H))
    }
    pub fn field_label_rect(&self, row: i32, h: i32) -> UiRect {
        let x = self.field_x();
        let y = self.label_y(row, h);
        UiRect::new(x, y, x + self.field_w(), y + h)
    }
    pub fn labeled_field_layout(
        &self,
        row: i32,
        label_h: i32,
        field_w: i32,
        field_h: i32,
    ) -> SettingsLabeledFieldLayout {
        SettingsLabeledFieldLayout {
            label_rect: self.label_rect(row, label_h),
            field_rect: self.field_rect(row, field_w, field_h),
        }
    }
    #[allow(dead_code)]
    pub fn field_w_from(&self, x: i32) -> i32 {
        (self.body.right - x).max(40)
    }
    pub fn action_x(&self, slot: i32, w: i32) -> i32 {
        self.body.left + slot * (w + settings_scale(14))
    }

    pub fn action_row_rects(&self, row: i32, widths: &[i32]) -> Vec<UiRect> {
        widths
            .iter()
            .enumerate()
            .map(|(slot, width)| {
                let w = *width;
                let x = self.action_x(slot as i32, w);
                let y = self.row_y(row);
                UiRect::new(x, y, x + w, y + settings_scale(SETTINGS_FORM_ROW_H))
            })
            .collect()
    }

    pub fn toggle_row_layout(&self, row: i32) -> SettingsToggleRowLayout {
        let row_h = settings_scale(SETTINGS_FORM_ROW_H);
        let toggle_w = settings_scale(44);
        let toggle_h = settings_scale(24);
        let gap = settings_scale(12);
        let y = self.row_y(row);
        settings_toggle_row_layout_for_rect(
            UiRect::new(self.left(), y, self.left() + self.full_w(), y + row_h),
            toggle_w,
            toggle_h,
            gap,
            settings_scale(40),
        )
    }

    pub fn field_action_hint_layout(
        &self,
        row: i32,
        action_w: i32,
        gap: i32,
        hint_y_offset: i32,
        hint_h: i32,
    ) -> SettingsActionHintLayout {
        let x = self.field_x();
        let y = self.row_y(row);
        let action_h = settings_scale(SETTINGS_FORM_ROW_H);
        let hint_x = x + action_w + gap;
        SettingsActionHintLayout {
            action_rect: UiRect::new(x, y, x + action_w, y + action_h),
            hint_rect: UiRect::new(
                hint_x,
                y + hint_y_offset,
                self.body.right,
                y + hint_y_offset + hint_h,
            ),
        }
    }

    pub fn qr_action_layout(&self, row: i32) -> SettingsQrActionLayout {
        let qr_size = settings_scale(112);
        let gap = settings_scale(16);
        let action_w = settings_scale(142);
        let x = self.field_x();
        let y = self.row_y(row);
        SettingsQrActionLayout {
            qr_rect: UiRect::new(x, y, x + qr_size, y + qr_size),
            action_rect: UiRect::new(
                x + qr_size + gap,
                y,
                x + qr_size + gap + action_w,
                y + settings_scale(SETTINGS_FORM_ROW_H),
            ),
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn section_h(section: &SettingsSection) -> i32 {
        section.rect.bottom - section.rect.top
    }

    #[test]
    fn bdd_plugin_cards_are_compact_when_plugins_are_off() {
        let cards = settings_plugin_cards_for_state(false, "off", "off", false, false);

        assert_eq!(cards.len(), 7);
        assert_eq!(cards[0].title, PLUGIN_TITLES[0]);
        assert!(cards[5].title.contains("WPS"));
        assert_eq!(cards[6].title, PLUGIN_TITLES[6]);
        assert!(!cards.iter().any(|card| card.title.contains("Disabled")));
        assert_eq!(section_h(&cards[0]), section_h(&cards[1]));
    }

    #[test]
    fn bdd_search_plugin_expands_card_and_pushes_following_cards_down() {
        let off = settings_plugin_cards_for_state(false, "off", "off", false, false);
        let on = settings_plugin_cards_for_state(true, "off", "off", false, false);

        assert!(section_h(&on[0]) > section_h(&off[0]));
        assert!(on[1].rect.top > off[1].rect.top);
        assert_eq!(section_h(&on[1]), section_h(&off[1]));
    }

    #[test]
    fn bdd_plugin_provider_cards_expand_only_for_configured_sources() {
        let off = settings_plugin_cards_for_state(false, "off", "off", false, false);
        let ocr = settings_plugin_cards_for_state(false, "baidu", "off", false, false);
        let translate = settings_plugin_cards_for_state(false, "off", "baidu", false, false);
        let mail_merge = settings_plugin_cards_for_state(false, "off", "off", true, false);

        assert!(section_h(&ocr[1]) > section_h(&off[1]));
        assert!(section_h(&translate[2]) > section_h(&off[2]));
        assert!(section_h(&mail_merge[4]) > section_h(&off[4]));
        assert_eq!(section_h(&ocr[0]), section_h(&off[0]));
    }

    #[test]
    fn settings_plugin_ai_panel_reads_product_ai_catalog() {
        let panel = settings_plugin_ai_panel();

        assert_eq!(panel.surface, ProductAiUiSurface::SettingsPluginPage);
        assert_eq!(
            panel.required_context,
            ProductAiContextKind::SettingsProfile
        );
        assert_eq!(
            panel.capabilities,
            vec![SettingsPluginAiCapabilityPresentation {
                capability_id: "clipboard.product.configure_ai",
                label: "Configure AI providers",
                provider: ProductAiProviderKind::ProductAdapter,
                action: ProductAiActionKind::ConfigureProvider,
                result: ProductAiResultKind::SettingsMutation,
            }]
        );
        assert!(!panel
            .capabilities
            .iter()
            .any(|capability| capability.capability_id == "clipboard.product.ocr"));
    }

    #[test]
    fn bdd_wps_taskpane_card_expands_when_enabled() {
        let off = settings_plugin_cards_for_state(false, "off", "off", false, false);
        let on = settings_plugin_cards_for_state(false, "off", "off", false, true);

        assert!(section_h(&on[5]) > section_h(&off[5]));
        assert!(on[6].rect.top > off[6].rect.top);
        assert_eq!(section_h(&on[6]), section_h(&off[6]));
    }

    #[test]
    fn bdd_multi_sync_cards_show_only_selected_transport_settings() {
        let off = settings_multi_sync_cards_for_mode("off");
        let webdav = settings_multi_sync_cards_for_mode("webdav");
        let lan = settings_multi_sync_cards_for_mode("lan");

        assert_eq!(off.len(), 1);
        assert_eq!(webdav.len(), 3);
        assert_eq!(lan.len(), 4);
        assert!(webdav[1].title.contains("WebDAV"));
        assert!(lan[1].title.contains("局域网"));
        assert!(lan[3].title.contains("扫码绑定"));
        assert!(webdav.iter().all(|card| !card.title.contains("局域网")));
        assert!(lan.iter().all(|card| !card.title.contains("WebDAV")));
        assert!(section_h(lan.last().unwrap()) > section_h(webdav.last().unwrap()));
        assert!(lan.last().unwrap().rect.bottom > webdav.last().unwrap().rect.bottom);
    }

    #[test]
    fn bdd_cloud_page_defaults_to_multi_sync_overview_only() {
        let cards = settings_cards_for_page_vec(SettingsPage::Cloud.index());
        let expected = settings_multi_sync_cards_for_mode("off");

        assert_eq!(cards.len(), 1);
        assert_eq!(cards.len(), expected.len());
        assert_eq!(cards[0].title, expected[0].title);
        assert_eq!(cards[0].rect.bottom, expected[0].rect.bottom);
        assert!(cards[0].title.contains("多端同步"));
    }

    #[test]
    fn settings_nav_index_hit_testing_uses_shared_nav_rects() {
        let first = settings_nav_item_rect(0);
        let cloud = settings_nav_item_rect(SettingsPage::Cloud.index());

        assert_eq!(
            settings_nav_index_at(first.left, first.top, SETTINGS_PAGE_COUNT),
            Some(0)
        );
        assert_eq!(
            settings_nav_index_at(cloud.left + 4, cloud.top + 4, SETTINGS_PAGE_COUNT),
            Some(SettingsPage::Cloud.index())
        );
        assert_eq!(
            settings_nav_index_at(first.right, first.top, SETTINGS_PAGE_COUNT),
            None
        );
        assert_eq!(
            settings_nav_index_at(first.left, first.bottom, SETTINGS_PAGE_COUNT),
            None
        );
        assert_eq!(settings_nav_index_at(first.left, first.top, 0), None);
    }

    #[test]
    fn settings_nav_render_plan_describes_nav_without_host_renderer() {
        let plan = settings_nav_render_plan(SettingsPage::Cloud.index(), Some(2), true);

        assert_eq!(plan.items.len(), SETTINGS_PAGE_COUNT);
        assert_eq!(
            plan.items
                .iter()
                .filter(|item| item.selected)
                .map(|item| item.page)
                .collect::<Vec<_>>(),
            vec![SettingsPage::Cloud]
        );
        assert_eq!(
            plan.items
                .iter()
                .filter(|item| item.hovered)
                .map(|item| item.page)
                .collect::<Vec<_>>(),
            vec![SettingsPage::Plugin]
        );

        let about = plan
            .items
            .iter()
            .find(|item| item.page == SettingsPage::About)
            .unwrap();
        assert_eq!(
            about.label,
            SETTINGS_PAGE_LABELS[SettingsPage::About.index()]
        );
        assert_eq!(about.icon, SettingsNavIconKind::About);
        assert!(about.badge_rect.is_some());
        assert_eq!(
            about.rect,
            settings_nav_item_rect(SettingsPage::About.index())
        );

        let no_badge = settings_nav_render_plan(usize::MAX, Some(usize::MAX), false);
        assert!(no_badge.items.iter().all(|item| !item.hovered));
        assert!(no_badge.items.iter().all(|item| item.badge_rect.is_none()));
        assert_eq!(
            no_badge.items.iter().filter(|item| item.selected).count(),
            1
        );
    }

    #[test]
    fn settings_nav_item_paint_plan_uses_semantic_commands() {
        let item = SettingsNavItemRender {
            index: SettingsPage::Plugin.index(),
            page: SettingsPage::Plugin,
            label: "插件",
            icon: SettingsNavIconKind::Plugin,
            rect: UiRect::new(10, 20, 200, 56),
            selected: true,
            hovered: false,
            badge_rect: Some(UiRect::new(180, 30, 190, 40)),
        };

        let plan = settings_nav_item_paint_plan(&item);
        assert_eq!(
            plan.paint_commands,
            vec![
                SettingsPaintCommand::RoundFill {
                    rect: item.rect,
                    fill: SettingsThemeRole::NavSelectedFill,
                    radius: settings_scale(6),
                },
                SettingsPaintCommand::RoundFill {
                    rect: UiRect::new(
                        item.rect.left + settings_scale(3),
                        38 - settings_scale(16) / 2,
                        item.rect.left + settings_scale(6),
                        38 + settings_scale(16) / 2,
                    ),
                    fill: SettingsThemeRole::Accent,
                    radius: settings_scale(2),
                },
                SettingsPaintCommand::RoundFill {
                    rect: UiRect::new(180, 30, 190, 40),
                    fill: SettingsThemeRole::Danger,
                    radius: settings_scale(5),
                }
            ]
        );
        assert_eq!(
            plan.text_commands,
            vec![
                SettingsTextCommand {
                    rect: UiRect::new(
                        item.rect.left + settings_scale(10),
                        item.rect.top,
                        item.rect.left + settings_scale(38),
                        item.rect.bottom,
                    ),
                    content: SettingsTextContent::NavIcon(SettingsNavIconKind::Plugin),
                    color: SettingsThemeRole::Accent,
                    size: 16,
                    bold: false,
                    font: SettingsTextFontRole::FluentIcon,
                },
                SettingsTextCommand {
                    rect: UiRect::new(
                        item.rect.left + settings_scale(40),
                        item.rect.top,
                        item.rect.right - settings_scale(8),
                        item.rect.bottom,
                    ),
                    content: SettingsTextContent::Label("插件"),
                    color: SettingsThemeRole::Text,
                    size: 14,
                    bold: false,
                    font: SettingsTextFontRole::UiText,
                },
            ]
        );

        let hovered = SettingsNavItemRender {
            selected: false,
            hovered: true,
            badge_rect: None,
            ..item
        };
        assert_eq!(
            settings_nav_item_paint_plan(&hovered).paint_commands,
            vec![SettingsPaintCommand::RoundFill {
                rect: item.rect,
                fill: SettingsThemeRole::NavHoverFill,
                radius: settings_scale(6),
            }]
        );
    }

    #[test]
    fn settings_nav_hover_transition_reports_only_changed_nav_rects() {
        let first = settings_nav_item_rect(0);
        let plugin = settings_nav_item_rect(SettingsPage::Plugin.index());

        let enter =
            settings_nav_hover_for_pointer(-1, first.left + 1, first.top + 1, SETTINGS_PAGE_COUNT);
        assert_eq!(enter.next_hot, 0);
        assert_eq!(enter.invalidate_rects, vec![first]);

        let unchanged =
            settings_nav_hover_for_pointer(0, first.left + 2, first.top + 2, SETTINGS_PAGE_COUNT);
        assert_eq!(unchanged.next_hot, 0);
        assert!(unchanged.invalidate_rects.is_empty());

        let switch = settings_nav_hover_transition(
            0,
            SettingsPage::Plugin.index() as i32,
            SETTINGS_PAGE_COUNT,
        );
        assert_eq!(switch.next_hot, SettingsPage::Plugin.index() as i32);
        assert_eq!(switch.invalidate_rects, vec![first, plugin]);

        let leave = settings_nav_hover_transition(
            SettingsPage::Plugin.index() as i32,
            -1,
            SETTINGS_PAGE_COUNT,
        );
        assert_eq!(leave.next_hot, -1);
        assert_eq!(leave.invalidate_rects, vec![plugin]);

        let out_of_range = settings_nav_hover_transition(99, 99, SETTINGS_PAGE_COUNT);
        assert_eq!(out_of_range.next_hot, -1);
        assert!(out_of_range.invalidate_rects.is_empty());
    }

    #[test]
    fn settings_pointer_move_transition_prioritizes_drag_over_nav_hover() {
        let layout = SettingsScrollLayout::new(100, 600, 1000, 500, 800, 3, 5);
        let plugin = settings_nav_item_rect(SettingsPage::Plugin.index());

        let dragging = settings_pointer_move_transition(
            plugin.left + 1,
            471,
            SETTINGS_PAGE_COUNT,
            0,
            true,
            layout,
            229,
            250,
        );
        assert_eq!(dragging.drag_scroll_y, Some(500));
        assert_eq!(dragging.nav_hover, None);

        let hover = settings_pointer_move_transition(
            plugin.left + 1,
            plugin.top + 1,
            SETTINGS_PAGE_COUNT,
            0,
            false,
            layout,
            229,
            250,
        );
        assert_eq!(hover.drag_scroll_y, None);
        let nav_hover = hover.nav_hover.unwrap();
        assert_eq!(nav_hover.next_hot, SettingsPage::Plugin.index() as i32);
        assert_eq!(
            nav_hover.invalidate_rects,
            vec![settings_nav_item_rect(0), plugin]
        );
    }

    #[test]
    fn settings_model_source_stays_platform_neutral() {
        let source = include_str!("settings_model.rs");
        let forbidden = [
            format!("{}{}", "crate::", "ui"),
            format!("{}{}", "windows", "_sys"),
            format!("{}{}", "HW", "ND"),
            format!("{}{}", "RE", "CT"),
        ];
        for token in forbidden {
            assert!(!source.contains(&token), "{token}");
        }
    }

    #[test]
    fn settings_dpi_move_action_only_reacts_to_real_dpi_changes() {
        assert_eq!(
            settings_dpi_move_action(96, 96, true),
            SettingsDpiMoveAction::None
        );
        assert_eq!(
            settings_dpi_move_action(144, 144, false),
            SettingsDpiMoveAction::None
        );
        assert_eq!(
            settings_dpi_move_action(96, 144, true),
            SettingsDpiMoveAction::ResizeForDpi
        );
        assert_eq!(
            settings_dpi_move_action(96, 144, false),
            SettingsDpiMoveAction::SyncOnly
        );
    }

    #[test]
    fn settings_wheel_delta_preserves_legacy_scroll_direction() {
        assert_eq!(settings_scroll_delta_for_wheel(120), -60);
        assert_eq!(settings_scroll_delta_for_wheel(-120), 60);
        assert_eq!(settings_scroll_delta_for_wheel(0), 60);
    }

    #[test]
    fn settings_viewport_geometry_is_platform_neutral() {
        let window = UiRect::new(0, 0, 1100, 740);
        let viewport = settings_viewport_rect_for_window(window);
        let mask = settings_viewport_mask_rect_for_window(window);
        let safe = settings_safe_paint_rect_for_window(window);

        assert_eq!(
            viewport,
            UiRect::new(
                settings_nav_w_scaled(),
                settings_content_y_scaled(),
                1100,
                740
            )
        );
        assert_eq!(
            mask,
            UiRect::new(
                settings_nav_w_scaled(),
                settings_content_y_scaled(),
                1100,
                settings_content_y_scaled() + settings_scale(SETTINGS_VIEWPORT_MASK_H)
            )
        );
        assert_eq!(safe, UiRect::new(mask.left, mask.bottom, mask.right, 740));

        assert!(!settings_child_visible_in_viewport(
            mask.top,
            settings_scale(SETTINGS_VIEWPORT_MASK_H) - 1,
            viewport
        ));
        assert!(settings_child_visible_in_viewport(
            mask.bottom - settings_scale(5),
            20,
            viewport
        ));
        assert!(settings_child_visible_in_viewport(
            mask.bottom,
            20,
            viewport
        ));
        assert!(!settings_child_visible_in_viewport(
            viewport.bottom,
            20,
            viewport
        ));
    }

    #[test]
    fn settings_chrome_render_plan_keeps_static_window_chrome_geometry_in_model() {
        crate::app_core::set_settings_ui_dpi(96);
        let window = UiRect::new(0, 0, 1100, 740);
        let plan = settings_chrome_render_plan(window);
        let mask = settings_viewport_mask_rect_for_window(window);

        assert_eq!(plan.window_rect, window);
        assert_eq!(
            plan.nav_rect,
            UiRect::new(0, 0, settings_nav_w_scaled(), 740)
        );
        assert_eq!(plan.divider_x, settings_nav_w_scaled());
        assert_eq!(plan.menu_icon_rect, UiRect::new(22, 18, 50, 46));
        assert_eq!(plan.app_title_rect, UiRect::new(56, 18, 220, 50));
        assert_eq!(plan.page_title_rect, settings_title_rect());
        assert_eq!(
            plan.content_clip_rect,
            settings_safe_paint_rect_for_window(window)
        );
        assert_eq!(plan.viewport_mask_rect, mask);
        assert_eq!(
            plan.viewport_mask_separator_rect,
            UiRect::new(
                mask.left + 12,
                mask.bottom - 1,
                mask.right - 12,
                mask.bottom
            )
        );
    }

    #[test]
    fn settings_chrome_paint_plan_describes_window_chrome_without_host_renderer() {
        crate::app_core::set_settings_ui_dpi(96);
        let window = UiRect::new(0, 0, 1100, 740);
        let chrome = settings_chrome_render_plan(window);
        let plan = settings_chrome_paint_plan(&chrome, "多端同步");

        assert_eq!(
            plan.paint_commands,
            vec![
                SettingsPaintCommand::FillRect {
                    rect: chrome.nav_rect,
                    fill: SettingsThemeRole::NavBackground,
                },
                SettingsPaintCommand::FillRect {
                    rect: UiRect::new(
                        chrome.divider_x,
                        chrome.window_rect.top,
                        chrome.divider_x + 1,
                        chrome.window_rect.bottom,
                    ),
                    fill: SettingsThemeRole::Stroke,
                },
            ]
        );
        assert_eq!(
            plan.text_commands,
            vec![
                SettingsTextCommand {
                    rect: chrome.menu_icon_rect,
                    content: SettingsTextContent::ChromeMenuIcon,
                    color: SettingsThemeRole::TextMuted,
                    size: 16,
                    bold: false,
                    font: SettingsTextFontRole::FluentIcon,
                },
                SettingsTextCommand {
                    rect: chrome.app_title_rect,
                    content: SettingsTextContent::Label("设置"),
                    color: SettingsThemeRole::Text,
                    size: 15,
                    bold: true,
                    font: SettingsTextFontRole::UiText,
                },
                SettingsTextCommand {
                    rect: chrome.page_title_rect,
                    content: SettingsTextContent::Label("多端同步"),
                    color: SettingsThemeRole::Text,
                    size: 24,
                    bold: true,
                    font: SettingsTextFontRole::Display,
                },
            ]
        );
    }

    #[test]
    fn settings_viewport_mask_paint_plan_uses_semantic_commands() {
        crate::app_core::set_settings_ui_dpi(96);
        let chrome = settings_chrome_render_plan(UiRect::new(0, 0, 1100, 740));
        let plan = settings_viewport_mask_paint_plan(&chrome);

        assert_eq!(
            plan.paint_commands,
            vec![
                SettingsPaintCommand::FillRect {
                    rect: chrome.viewport_mask_rect,
                    fill: SettingsThemeRole::Background,
                },
                SettingsPaintCommand::FillRect {
                    rect: chrome.viewport_mask_separator_rect,
                    fill: SettingsThemeRole::Stroke,
                },
            ]
        );
        assert!(plan.text_commands.is_empty());
    }

    #[test]
    fn settings_content_paint_plan_describes_cards_and_titles() {
        crate::app_core::set_settings_ui_dpi(96);
        let render_plan = SettingsContentRenderPlan {
            page: SettingsPage::Plugin,
            source: SettingsContentSource::PluginDynamic,
            scroll_y: 20,
            sections: vec![SettingsSection {
                title: "搜索插件",
                rect: UiRect::new(264, 132, 1072, 256),
            }],
        };
        let plan = settings_content_paint_plan(&render_plan);
        let card_rect = UiRect::new(264, 112, 1072, 236);

        assert_eq!(
            plan.paint_commands,
            vec![SettingsPaintCommand::RoundRect {
                rect: card_rect,
                fill: SettingsThemeRole::Surface,
                stroke: SettingsThemeRole::Stroke,
                radius: settings_scale(8),
            }]
        );
        assert_eq!(
            plan.text_commands,
            vec![SettingsTextCommand {
                rect: UiRect::new(280, 124, 1056, 146),
                content: SettingsTextContent::Label("搜索插件"),
                color: SettingsThemeRole::TextMuted,
                size: 12,
                bold: true,
                font: SettingsTextFontRole::UiText,
            }]
        );
    }

    #[test]
    fn settings_scrollbar_paint_plan_describes_track_and_thumb() {
        let normal = SettingsScrollbarRenderPlan {
            state: SettingsScrollbarVisualState::Normal,
            bar_width: 3,
            track_rect: None,
            thumb_rect: UiRect::new(1094, 100, 1097, 180),
        };
        assert_eq!(
            settings_scrollbar_paint_plan(&normal).paint_commands,
            vec![SettingsPaintCommand::RoundFill {
                rect: normal.thumb_rect,
                fill: SettingsThemeRole::ScrollbarThumb,
                radius: 3,
            }]
        );

        let dragging = SettingsScrollbarRenderPlan {
            state: SettingsScrollbarVisualState::Dragging,
            bar_width: 5,
            track_rect: Some(UiRect::new(1092, 92, 1097, 728)),
            thumb_rect: UiRect::new(1092, 220, 1097, 340),
        };
        assert_eq!(
            settings_scrollbar_paint_plan(&dragging).paint_commands,
            vec![
                SettingsPaintCommand::RoundFill {
                    rect: UiRect::new(1092, 92, 1097, 728),
                    fill: SettingsThemeRole::ScrollbarTrack,
                    radius: 5,
                },
                SettingsPaintCommand::RoundFill {
                    rect: dragging.thumb_rect,
                    fill: SettingsThemeRole::ScrollbarThumbDragging,
                    radius: 5,
                },
            ]
        );
    }

    #[test]
    fn settings_paint_plans_share_one_command_protocol() {
        crate::app_core::set_settings_ui_dpi(96);
        fn command_counts(plan: SettingsPaintPlan) -> (usize, usize) {
            (plan.paint_commands.len(), plan.text_commands.len())
        }

        let nav = SettingsNavItemRender {
            index: SettingsPage::General.index(),
            page: SettingsPage::General,
            label: "常规",
            icon: SettingsNavIconKind::General,
            rect: settings_nav_item_rect(SettingsPage::General.index()),
            selected: true,
            hovered: false,
            badge_rect: None,
        };
        let chrome = settings_chrome_render_plan(UiRect::new(0, 0, 1100, 740));
        let content = SettingsContentRenderPlan {
            page: SettingsPage::General,
            source: SettingsContentSource::StaticPage,
            sections: vec![SettingsSection {
                title: "窗口行为",
                rect: UiRect::new(264, 132, 1072, 256),
            }],
            scroll_y: 0,
        };
        let scrollbar = SettingsScrollbarRenderPlan {
            state: SettingsScrollbarVisualState::Normal,
            bar_width: 3,
            track_rect: None,
            thumb_rect: UiRect::new(1094, 100, 1097, 180),
        };

        assert_eq!(command_counts(settings_nav_item_paint_plan(&nav)), (2, 2));
        assert_eq!(
            command_counts(settings_chrome_paint_plan(&chrome, "常规")),
            (2, 3)
        );
        assert_eq!(
            command_counts(settings_viewport_mask_paint_plan(&chrome)),
            (2, 0)
        );
        assert_eq!(
            command_counts(settings_content_paint_plan(&content)),
            (1, 1)
        );
        assert_eq!(
            command_counts(settings_scrollbar_paint_plan(&scrollbar)),
            (1, 0)
        );
    }

    #[test]
    fn settings_window_fit_plan_clamps_position_and_size_to_work_area() {
        let work = UiRect::new(0, 0, 1000, 800);
        let current = UiRect::new(900, 700, 1500, 1000);

        assert_eq!(
            settings_window_fit_plan(current, work, 12, 720, 480),
            Some(SettingsWindowMovePlan {
                x: 268,
                y: 308,
                width: 720,
                height: 480,
            })
        );
        assert_eq!(
            settings_window_fit_plan(UiRect::new(100, 100, 820, 580), work, 12, 720, 480),
            None
        );
    }

    #[test]
    fn settings_window_dpi_transition_plan_scales_around_center_and_clamps() {
        let current = UiRect::new(100, 100, 900, 700);
        let work = UiRect::new(0, 0, 1200, 900);

        assert_eq!(
            settings_window_dpi_transition_plan(current, work, 12, 720, 480, 96, 144),
            Some(SettingsWindowMovePlan {
                x: 12,
                y: 12,
                width: 1176,
                height: 876,
            })
        );
        assert_eq!(
            settings_window_dpi_transition_plan(current, work, 12, 720, 480, 144, 144),
            None
        );
    }

    #[test]
    fn settings_dropdown_option_labels_are_platform_neutral() {
        let max_labels = settings_dropdown_max_items_labels();
        assert_eq!(max_labels[..5], ["100", "200", "500", "1000", "3000"]);
        assert_eq!(settings_dropdown_label_for_max_items(500), "500");
        assert_eq!(settings_dropdown_label_for_max_items(0), max_labels[5]);
        assert_eq!(settings_dropdown_index_for_max_items(100), 0);
        assert_eq!(settings_dropdown_index_for_max_items(0), 5);
        assert_eq!(
            settings_dropdown_max_items_from_label_opt(" 1000 "),
            Some(1000)
        );
        assert_eq!(
            settings_dropdown_max_items_from_label_opt(max_labels[5]),
            Some(0)
        );
        assert_eq!(settings_dropdown_max_items_from_label_opt(""), None);
        assert_eq!(settings_dropdown_max_items_from_label("bad"), 0);

        assert_eq!(
            settings_dropdown_label_for_pos_mode("fixed"),
            tr("固定位置", "Fixed Position")
        );
        assert_eq!(
            settings_dropdown_label_for_pos_mode("last"),
            tr("上次位置", "Last Position")
        );
        assert_eq!(
            settings_dropdown_label_for_pos_mode("mouse"),
            tr("跟随鼠标", "Follow Mouse")
        );
        assert_eq!(settings_dropdown_index_for_pos_mode("fixed"), 1);
        assert_eq!(settings_dropdown_index_for_pos_mode("last"), 2);
        assert_eq!(settings_dropdown_index_for_pos_mode("mouse"), 0);
        assert_eq!(
            settings_dropdown_pos_mode_from_label("Fixed Position"),
            "fixed"
        );
        assert_eq!(settings_dropdown_pos_mode_from_label("上次位置"), "last");
        assert_eq!(settings_dropdown_pos_mode_from_label("unknown"), "mouse");
    }

    #[test]
    fn multi_sync_options_are_platform_neutral() {
        assert_eq!(MULTI_SYNC_MODE_OPTIONS, ["关闭", "WebDAV", "局域网"]);
        assert_eq!(multi_sync_mode_display("off"), "关闭");
        assert_eq!(multi_sync_mode_display("webdav"), "WebDAV");
        assert_eq!(multi_sync_mode_display("lan"), "局域网");
        assert_eq!(multi_sync_mode_from_label("WebDAV"), "webdav");
        assert_eq!(multi_sync_mode_from_label("局域网"), "lan");
        assert_eq!(multi_sync_mode_from_label("lan"), "lan");
        assert_eq!(multi_sync_mode_from_label("关闭"), "off");

        assert_eq!(multi_sync_mode_from_flags(false, false), "off");
        assert_eq!(multi_sync_mode_from_flags(true, false), "webdav");
        assert_eq!(multi_sync_mode_from_flags(false, true), "lan");
        assert_eq!(multi_sync_mode_from_flags(true, true), "lan");
        assert_eq!(multi_sync_flags_for_mode("webdav"), (true, false));
        assert_eq!(multi_sync_flags_for_mode("lan"), (false, true));
        assert_eq!(multi_sync_flags_for_mode("off"), (false, false));
        assert_eq!(normalize_multi_sync_flags(true, true), (false, true));
        assert_eq!(localized_cloud_status_text(""), tr("未同步", "Not synced"));
        assert_eq!(
            localized_cloud_status_text(" 未同步 "),
            tr("未同步", "Not synced")
        );
        assert_eq!(
            localized_cloud_status_text("失败：timeout"),
            format!("{}timeout", tr("失败：", "Failed: "))
        );
        assert_eq!(
            localized_cloud_status_text("同步成功"),
            translate("同步成功")
        );
    }

    #[test]
    fn lan_receive_mode_options_are_platform_neutral() {
        assert_eq!(lan_receive_mode_display("records_only"), "只进入记录");
        assert_eq!(lan_receive_mode_display("clipboard"), "直接覆盖剪贴板");
        assert_eq!(lan_receive_mode_display("unknown"), "只进入记录");
        assert_eq!(lan_receive_mode_from_label("直接覆盖剪贴板"), "clipboard");
        assert_eq!(lan_receive_mode_from_label("clipboard"), "clipboard");
        assert_eq!(lan_receive_mode_from_label("只进入记录"), "records_only");
        assert_eq!(
            lan_trusted_summary_value_text("信任设备：暂无。可输入 IP 手动连接"),
            "暂无。可输入 IP 手动连接"
        );
        assert_eq!(
            lan_trusted_summary_value_text("信任设备:  Android"),
            "Android"
        );
        assert_eq!(lan_trusted_summary_value_text("Android"), "Android");
    }

    #[test]
    fn plugin_option_models_are_platform_neutral() {
        assert_eq!(SEARCH_ENGINE_PRESETS[0].0, "jzxx");
        assert!(search_engine_template("bing").contains("bing.com"));
        assert_eq!(
            search_engine_template("unknown"),
            search_engine_template("jzxx")
        );
        assert_eq!(search_engine_display("baidu"), "百度");
        assert_eq!(search_engine_key_from_display("Google"), "google");
        assert_eq!(
            search_engine_key_from_display(&search_engine_display("custom")),
            "custom"
        );
        assert_eq!(search_engine_key_from_display("unknown"), "jzxx");

        assert_eq!(IMAGE_OCR_PROVIDER_OPTIONS.len(), 3);
        assert_eq!(image_ocr_provider_display("baidu"), "百度 OCR");
        assert_eq!(
            image_ocr_provider_key_from_display(&image_ocr_provider_display("winocr")),
            "winocr"
        );
        assert_eq!(image_ocr_provider_key_from_display("unknown"), "off");

        assert_eq!(TEXT_TRANSLATE_PROVIDER_OPTIONS.len(), 2);
        assert_eq!(text_translate_provider_display("baidu"), "百度翻译");
        assert_eq!(
            text_translate_provider_key_from_display(&text_translate_provider_display("baidu")),
            "baidu"
        );
        assert_eq!(text_translate_provider_key_from_display("unknown"), "off");

        assert_eq!(TEXT_TRANSLATE_TARGET_OPTIONS.len(), 4);
        assert_eq!(text_translate_target_display("en"), "英语");
        assert_eq!(
            text_translate_target_key_from_display(&text_translate_target_display("kor")),
            "kor"
        );
        assert_eq!(text_translate_target_key_from_display("unknown"), "zh");
    }

    #[test]
    fn hotkey_sound_and_source_tab_models_are_platform_neutral() {
        assert_eq!(HOTKEY_MOD_OPTIONS[0], "Win");
        assert!(HOTKEY_KEY_OPTIONS.contains(&"V"));
        assert!(HOTKEY_KEY_OPTIONS.contains(&"PageDown"));
        assert_eq!(normalize_hotkey_mod(" Ctrl+Shift "), "Ctrl+Shift");
        assert_eq!(normalize_hotkey_mod("unknown"), "Win");
        assert_eq!(normalize_hotkey_key(" Enter "), "Enter");
        assert_eq!(normalize_hotkey_key("unknown"), "V");
        assert!(hotkey_preview_text("Ctrl+Shift", "V").contains("Ctrl+Shift + V"));

        assert_eq!(PASTE_SOUND_OPTIONS.len(), 4);
        assert_eq!(paste_sound_display("soft"), "柔和");
        assert_eq!(
            paste_sound_key_from_display(&paste_sound_display("custom")),
            "custom"
        );
        assert_eq!(paste_sound_key_from_display("unknown"), "default");
        assert_eq!(
            paste_sound_file_button_text(""),
            tr("选择文件", "Choose file")
        );
        assert_eq!(
            paste_sound_file_button_text(r"C:\Sounds\ding.wav"),
            "ding.wav"
        );

        assert_eq!(normalize_source_tab(0), 0);
        assert_eq!(normalize_source_tab(1), 1);
        assert_eq!(normalize_source_tab(9), 0);
        assert_eq!(source_tab_category(1), 1);
        assert_eq!(source_tab_all_label(0), tr("全部记录", "All Records"));
        assert_eq!(source_tab_all_label(1), tr("全部短语", "All Phrases"));
        assert_eq!(source_tab_label(0), tr("复制记录", "Clipboard Records"));
        assert_eq!(source_tab_label(1), tr("常用短语", "Phrases"));
        assert_eq!(
            settings_group_overview_text(0, source_tab_all_label(0)),
            format!(
                "{}（{}）：{}",
                tr("当前分组", "Current Group"),
                tr("复制记录", "Clipboard Records"),
                tr("全部记录", "All Records")
            )
        );
    }

    #[test]
    fn group_display_name_uses_platform_neutral_entries() {
        let groups = [(7, "项目资料"), (9, "常用句子")];

        assert_eq!(
            group_name_for_display_entries(groups, 0, "全部记录"),
            "全部记录"
        );
        assert_eq!(
            group_name_for_display_entries(groups, 9, "全部记录"),
            "常用句子"
        );
        assert_eq!(
            group_name_for_display_entries(groups, 88, "全部记录"),
            "全部记录"
        );
    }

    #[test]
    fn settings_dropdown_popup_layout_centers_selection_and_clamps_rows() {
        let anchor = UiRect::new(100, 200, 280, 232);
        let few = SettingsDropdownPopupLayout::new(anchor, 3, 2, 120);

        assert_eq!(few.visible_rows, 3);
        assert_eq!(few.max_scroll, 0);
        assert_eq!(few.scroll_top, 0);
        assert_eq!(
            few.rect,
            UiRect::new(
                100,
                238,
                280,
                238 + settings_scale(SETTINGS_DROPDOWN_PAD) * 2
                    + settings_scale(SETTINGS_DROPDOWN_ITEM_H) * 3
            )
        );

        let many = SettingsDropdownPopupLayout::new(anchor, 20, 12, 260);
        assert_eq!(many.visible_rows, SETTINGS_DROPDOWN_MAX_VISIBLE_ROWS);
        assert_eq!(many.max_scroll, 12);
        assert_eq!(many.scroll_top, 8);
        assert_eq!(many.rect.right - many.rect.left, 260);

        let tail = SettingsDropdownPopupLayout::new(anchor, 20, 19, 260);
        assert_eq!(tail.scroll_top, tail.max_scroll);
    }

    #[test]
    fn settings_dropdown_interaction_maps_hits_and_wheel_without_host_state() {
        let pad = settings_scale(SETTINGS_DROPDOWN_PAD);
        let item_h = settings_scale(SETTINGS_DROPDOWN_ITEM_H);

        assert_eq!(
            settings_dropdown_index_from_y(8, 2, 4, item_h, pad, pad - 1),
            None
        );
        assert_eq!(
            settings_dropdown_index_from_y(8, 2, 4, item_h, pad, pad + item_h / 2),
            Some(2)
        );
        assert_eq!(
            settings_dropdown_index_from_y(8, 2, 4, item_h, pad, pad + item_h * 3 + item_h / 2),
            Some(5)
        );
        assert_eq!(
            settings_dropdown_index_from_y(8, 2, 4, item_h, pad, pad + item_h * 4),
            None
        );

        assert_eq!(settings_dropdown_max_scroll(8, 4), 4);
        assert_eq!(settings_dropdown_scroll_target(8, 4, 2, 120), 1);
        assert_eq!(settings_dropdown_scroll_target(8, 4, 1, -120), 2);
        assert_eq!(settings_dropdown_scroll_target(8, 4, 4, -120), 4);
        assert_eq!(settings_dropdown_scroll_target(3, 4, 0, -120), 0);
    }

    #[test]
    fn settings_dropdown_interaction_state_tracks_hover_and_scroll() {
        let layout = SettingsDropdownPopupLayout::new(UiRect::new(0, 0, 100, 32), 12, 4, 100);
        let mut state = SettingsDropdownInteractionState::new(12, layout);
        state.hover = 3;
        state.scroll_top = 2;

        assert_eq!(
            state.index_from_y(state.pad + state.item_height / 2),
            Some(2)
        );
        assert!(state.scroll_by_wheel(120));
        assert_eq!(state.scroll_top, 1);
        assert_eq!(state.hover, -1);

        state.scroll_top = state.max_scroll();
        assert!(!state.scroll_by_wheel(-120));
        assert_eq!(state.scroll_top, state.max_scroll());
    }

    #[test]
    fn settings_control_metrics_measure_visible_content_height() {
        let page = SettingsPage::Plugin.index();
        let controls = [
            SettingsControlMetrics {
                page,
                bounds: UiRect::new(10, 100, 110, 150),
                visible: true,
            },
            SettingsControlMetrics {
                page,
                bounds: UiRect::new(10, 100, 110, 500),
                visible: false,
            },
            SettingsControlMetrics {
                page: SettingsPage::General.index(),
                bounds: UiRect::new(10, 100, 110, 600),
                visible: true,
            },
        ];

        let bottom = settings_content_bottom_for_controls(controls, page);
        assert_eq!(bottom, settings_control_content_bottom(controls[0].bounds));
        assert_eq!(
            settings_measured_content_total_h(bottom),
            bottom - settings_content_y_scaled()
        );
        assert_eq!(
            settings_content_bottom_for_controls([], page),
            settings_content_y_scaled()
        );
        assert_eq!(settings_measured_content_total_h(0), 0);
    }

    #[test]
    fn settings_page_content_total_h_uses_dynamic_sections_without_host_state() {
        let plugin_sections = [SettingsSection {
            title: "dynamic plugin",
            rect: UiRect::new(
                settings_content_x_scaled(),
                settings_content_y_scaled() + settings_scale(16),
                settings_content_x_scaled() + settings_content_w_scaled(),
                settings_content_y_scaled() + settings_scale(360),
            ),
        }];
        let multi_sync_sections = [SettingsSection {
            title: "dynamic sync",
            rect: UiRect::new(
                settings_content_x_scaled(),
                settings_content_y_scaled() + settings_scale(16),
                settings_content_x_scaled() + settings_content_w_scaled(),
                settings_content_y_scaled() + settings_scale(520),
            ),
        }];

        assert_eq!(
            settings_page_content_total_h_for_dynamic_sections(
                SettingsPage::Plugin.index(),
                &plugin_sections,
                &[],
                0,
            ),
            settings_scale(376)
        );
        assert_eq!(
            settings_page_content_total_h_for_dynamic_sections(
                SettingsPage::Cloud.index(),
                &[],
                &multi_sync_sections,
                settings_scale(640),
            ),
            settings_scale(640)
        );
        assert_eq!(
            settings_page_content_total_h_for_dynamic_sections(
                SettingsPage::General.index(),
                &plugin_sections,
                &multi_sync_sections,
                0,
            ),
            settings_page_content_total_h(SettingsPage::General.index())
        );
        assert_eq!(
            settings_page_max_scroll(settings_scale(900), settings_scale(500)),
            settings_scale(400)
        );
        assert_eq!(
            settings_page_max_scroll(settings_scale(300), settings_scale(500)),
            0
        );
    }

    #[test]
    fn settings_scroll_update_for_target_clamps_and_updates_page_cache_without_host_state() {
        assert_eq!(
            settings_scroll_update_for_target(2, 120, 260, 900, 500),
            Some(SettingsScrollUpdate {
                page: 2,
                content_scroll_y: 260,
                page_scroll_y: 260,
            })
        );
        assert_eq!(
            settings_scroll_update_for_target(2, 120, -30, 900, 500),
            Some(SettingsScrollUpdate {
                page: 2,
                content_scroll_y: 0,
                page_scroll_y: 0,
            })
        );
        assert_eq!(
            settings_scroll_update_for_target(2, 120, 900, 900, 500),
            Some(SettingsScrollUpdate {
                page: 2,
                content_scroll_y: 400,
                page_scroll_y: 400,
            })
        );
        assert_eq!(
            settings_scroll_update_for_target(2, 400, 900, 900, 500),
            None
        );
        assert_eq!(settings_scroll_update_for_target(2, 0, 80, 300, 500), None);
    }

    #[test]
    fn settings_page_switch_plan_normalizes_target_and_restores_scroll_without_host_state() {
        let plan = settings_page_switch_plan(
            SettingsPage::Hotkey.index(),
            usize::MAX,
            SETTINGS_PAGE_COUNT,
            false,
            true,
            true,
            true,
            180,
        );

        assert_eq!(plan.mode, SettingsPageSwitchMode::Switch);
        assert_eq!(plan.old_page, SettingsPage::Hotkey.index());
        assert_eq!(plan.target_page, SettingsPage::About.index());
        assert!(plan.cancel_hotkey_recording);
        assert!(plan.cancel_scroll_drag);
        assert!(plan.close_dropdown);
        assert_eq!(
            plan.scroll_state,
            Some(SettingsPageSwitchScrollState {
                page: SettingsPage::About.index(),
                content_scroll_y: 180,
                page_scroll_y: 180,
                scroll_bar_visible: false,
            })
        );
        assert!(plan.reposition_controls);
    }

    #[test]
    fn settings_page_switch_plan_syncs_only_when_target_page_is_already_built() {
        let same_built = settings_page_switch_plan(
            SettingsPage::Cloud.index(),
            SettingsPage::Cloud.index(),
            SETTINGS_PAGE_COUNT,
            true,
            true,
            true,
            true,
            320,
        );

        assert_eq!(same_built.mode, SettingsPageSwitchMode::SyncOnly);
        assert_eq!(same_built.target_page, SettingsPage::Cloud.index());
        assert!(!same_built.cancel_hotkey_recording);
        assert!(!same_built.cancel_scroll_drag);
        assert!(!same_built.close_dropdown);
        assert_eq!(same_built.scroll_state, None);
        assert!(!same_built.reposition_controls);

        let same_unbuilt = settings_page_switch_plan(
            SettingsPage::Cloud.index(),
            SettingsPage::Cloud.index(),
            SETTINGS_PAGE_COUNT,
            false,
            false,
            true,
            true,
            320,
        );

        assert_eq!(same_unbuilt.mode, SettingsPageSwitchMode::Switch);
        assert!(same_unbuilt.cancel_scroll_drag);
        assert!(same_unbuilt.close_dropdown);
        assert_eq!(
            same_unbuilt.scroll_state,
            Some(SettingsPageSwitchScrollState {
                page: SettingsPage::Cloud.index(),
                content_scroll_y: 320,
                page_scroll_y: 320,
                scroll_bar_visible: false,
            })
        );
    }

    #[test]
    fn settings_content_render_plan_selects_dynamic_or_static_sections_without_host_state() {
        let plugin_sections = [SettingsSection {
            title: "plugin",
            rect: UiRect::new(10, 20, 110, 120),
        }];
        let multi_sync_sections = [SettingsSection {
            title: "sync",
            rect: UiRect::new(30, 40, 130, 180),
        }];

        let plugin = settings_content_render_plan(
            SettingsPage::Plugin.index(),
            42,
            &plugin_sections,
            &multi_sync_sections,
        );
        assert_eq!(plugin.page, SettingsPage::Plugin);
        assert_eq!(plugin.source, SettingsContentSource::PluginDynamic);
        assert_eq!(plugin.sections, plugin_sections);
        assert_eq!(plugin.scroll_y, 42);

        let sync = settings_content_render_plan(
            SettingsPage::Cloud.index(),
            64,
            &[],
            &multi_sync_sections,
        );
        assert_eq!(sync.page, SettingsPage::Cloud);
        assert_eq!(sync.source, SettingsContentSource::MultiSyncDynamic);
        assert_eq!(sync.sections, multi_sync_sections);
        assert_eq!(sync.scroll_y, 64);

        let fallback = settings_content_render_plan(SettingsPage::Plugin.index(), 99, &[], &[]);
        assert_eq!(fallback.page, SettingsPage::Plugin);
        assert_eq!(fallback.source, SettingsContentSource::StaticPage);
        assert_eq!(
            fallback.sections,
            settings_cards_for_page_vec(SettingsPage::Plugin.index())
        );
        assert_eq!(fallback.scroll_y, 99);

        let general = settings_content_render_plan(SettingsPage::General.index(), 77, &[], &[]);
        assert_eq!(general.source, SettingsContentSource::StaticPage);
        assert_eq!(
            general.scroll_y,
            if settings_page_scrollable(SettingsPage::General.index()) {
                77
            } else {
                0
            }
        );
    }

    #[test]
    fn settings_native_page_summaries_cover_all_pages_and_sections() {
        let summaries = settings_native_page_summaries();
        let section_summaries = settings_native_section_summaries();
        let control_summaries = settings_native_control_summaries();

        assert_eq!(summaries.len(), SETTINGS_PAGE_COUNT);
        assert_eq!(
            summaries
                .iter()
                .map(|summary| (summary.page, summary.label))
                .collect::<Vec<_>>(),
            vec![
                (SettingsPage::General, SETTINGS_PAGE_LABELS[0]),
                (SettingsPage::Hotkey, SETTINGS_PAGE_LABELS[1]),
                (SettingsPage::Plugin, SETTINGS_PAGE_LABELS[2]),
                (SettingsPage::Group, SETTINGS_PAGE_LABELS[3]),
                (SettingsPage::Cloud, SETTINGS_PAGE_LABELS[4]),
                (SettingsPage::About, SETTINGS_PAGE_LABELS[5]),
            ]
        );
        assert!(summaries
            .iter()
            .all(|summary| !summary.section_titles.is_empty()));
        assert!(summaries[SettingsPage::Group.index()]
            .section_titles
            .contains(&"分组管理"));
        assert!(summaries[SettingsPage::Cloud.index()]
            .section_titles
            .iter()
            .any(|title| title.contains("同步")));
        assert_eq!(
            summaries[SettingsPage::Cloud.index()].section_titles.len(),
            6
        );
        assert!(summaries[SettingsPage::Cloud.index()]
            .section_titles
            .contains(&"WebDAV 传输"));
        assert!(summaries[SettingsPage::Cloud.index()]
            .section_titles
            .contains(&"设备发现 / 配对"));
        assert_eq!(
            summaries
                .iter()
                .map(|summary| summary.section_titles.len())
                .sum::<usize>(),
            section_summaries.len()
        );
        assert!(section_summaries
            .iter()
            .all(|section| section.control_rows > 0 && section.rect.right > section.rect.left));
        assert!(section_summaries.iter().any(|section| {
            section.page == SettingsPage::Group
                && section.section_title == "分组管理"
                && section.control_rows >= 10
        }));
        assert!(section_summaries.iter().any(|section| {
            section.page == SettingsPage::Cloud
                && section.section_title.contains("同步")
                && section.control_rows >= 2
        }));
        for section in &section_summaries {
            assert!(
                control_summaries.iter().any(|control| {
                    control.page == section.page && control.section_index == section.section_index
                }),
                "missing controls for {:?} section {}",
                section.page,
                section.section_index
            );
        }
        assert!(control_summaries.iter().any(|control| {
            control.page == SettingsPage::Group
                && control.key == "group_list"
                && control.kind == SettingsNativeControlKind::List
        }));
        assert!(control_summaries.iter().any(|control| {
            control.page == SettingsPage::General
                && control.key == "skip_window_classes"
                && control.kind == SettingsNativeControlKind::TextInput
        }));
        assert!(control_summaries.iter().any(|control| {
            control.page == SettingsPage::Cloud
                && control.key == "multi_sync_mode"
                && control.kind == SettingsNativeControlKind::Dropdown
        }));
        assert!(control_summaries.iter().any(|control| {
            control.page == SettingsPage::Cloud
                && control.key == "cloud_webdav_url"
                && control.kind == SettingsNativeControlKind::TextInput
                && control
                    .binding
                    .as_ref()
                    .and_then(|binding| binding.field_name)
                    == Some("cloud_webdav_url")
        }));
        assert!(control_summaries.iter().any(|control| {
            control.page == SettingsPage::Cloud
                && control.key == "lan_manual_host"
                && control.kind == SettingsNativeControlKind::TextInput
                && control
                    .binding
                    .as_ref()
                    .and_then(|binding| binding.field_name)
                    == Some("lan_manual_host")
        }));
        assert!(control_summaries.iter().any(|control| {
            control.page == SettingsPage::Cloud
                && control.key == "lan_discovered_list"
                && control.kind == SettingsNativeControlKind::List
                && control.binding.as_ref().map(|binding| binding.binding_name)
                    == Some("lan_discovered_devices")
        }));
        assert!(control_summaries
            .iter()
            .any(|control| control.kind == SettingsNativeControlKind::Toggle));
        assert!(control_summaries
            .iter()
            .any(|control| control.kind == SettingsNativeControlKind::Button));
        assert_eq!(SettingsNativeControlKind::Toggle.role_name(), "toggle");
        assert_eq!(SettingsNativeControlKind::Dropdown.role_name(), "dropdown");
        let auto_start = control_summaries
            .iter()
            .find(|control| control.key == "auto_start")
            .unwrap();
        assert_eq!(
            auto_start.route,
            Some(SettingsNativeControlRoute {
                kind: SettingsNativeControlRouteKind::Command,
                route_name: "settings_command",
                command_id: Some("window.settings.control.toggle"),
                control_id: Some(5010),
                action_name: None,
            })
        );
        assert_eq!(
            auto_start.route_label(),
            "command:window.settings.control.toggle#5010"
        );
        let sync_mode = control_summaries
            .iter()
            .find(|control| control.key == "multi_sync_mode")
            .unwrap();
        assert_eq!(
            sync_mode.route.unwrap().command_id,
            Some("window.settings.dropdown.open")
        );
        assert_eq!(sync_mode.route.unwrap().control_id, Some(5073));
        let cloud_interval = control_summaries
            .iter()
            .find(|control| control.key == "cloud_sync_interval")
            .unwrap();
        assert_eq!(
            cloud_interval.route.unwrap().command_id,
            Some("window.settings.dropdown.open")
        );
        assert_eq!(cloud_interval.route.unwrap().control_id, Some(5041));
        let lan_receive = control_summaries
            .iter()
            .find(|control| control.key == "lan_receive_mode")
            .unwrap();
        assert_eq!(lan_receive.route.unwrap().control_id, Some(5092));
        let cloud_sync_now = control_summaries
            .iter()
            .find(|control| control.key == "cloud_sync_now")
            .unwrap();
        assert_eq!(
            cloud_sync_now.route,
            Some(SettingsNativeControlRoute {
                kind: SettingsNativeControlRouteKind::Action,
                route_name: "settings_sync",
                command_id: None,
                control_id: None,
                action_name: Some("sync_webdav_now"),
            })
        );
        let lan_docs = control_summaries
            .iter()
            .find(|control| control.key == "lan_docs")
            .unwrap();
        assert_eq!(lan_docs.route.unwrap().route_name, "settings_sync");
        assert_eq!(
            lan_docs.route.unwrap().action_name,
            Some("open_lan_setup_page")
        );
        let open_source = control_summaries
            .iter()
            .find(|control| control.key == "open_source")
            .unwrap();
        assert_eq!(
            open_source.route,
            Some(SettingsNativeControlRoute {
                kind: SettingsNativeControlRouteKind::Action,
                route_name: "settings_platform",
                command_id: None,
                control_id: None,
                action_name: Some("open_source_repository"),
            })
        );
        assert_eq!(
            open_source.route_label(),
            "action:settings_platform/open_source_repository"
        );
        let skip_classes = control_summaries
            .iter()
            .find(|control| control.key == "skip_window_classes")
            .unwrap();
        assert_eq!(
            skip_classes.binding,
            Some(SettingsNativeControlBinding {
                kind: SettingsNativeControlBindingKind::SettingField,
                binding_name: "paste_target_skip_class_names",
                field_name: Some("paste_target_skip_class_names"),
                collect_required: true,
                apply_required: true,
            })
        );
        assert_eq!(
            skip_classes.binding_label(),
            "setting:paste_target_skip_class_names"
        );
        let group_list = control_summaries
            .iter()
            .find(|control| control.key == "group_list")
            .unwrap();
        assert_eq!(
            group_list.binding,
            Some(SettingsNativeControlBinding {
                kind: SettingsNativeControlBindingKind::RuntimeList,
                binding_name: "clip_groups",
                field_name: None,
                collect_required: false,
                apply_required: true,
            })
        );
        assert_eq!(group_list.binding_label(), "list:clip_groups");
        assert!(control_summaries
            .iter()
            .filter(|control| matches!(
                control.kind,
                SettingsNativeControlKind::Toggle
                    | SettingsNativeControlKind::Dropdown
                    | SettingsNativeControlKind::Button
            ))
            .all(|control| control.route.is_some()));
        assert!(control_summaries
            .iter()
            .filter(|control| matches!(
                control.kind,
                SettingsNativeControlKind::TextInput | SettingsNativeControlKind::List
            ))
            .all(|control| control.binding.is_some()));
        let apply_collect = settings_native_apply_collect_plan();
        assert!(apply_collect.collect_bindings.iter().any(|binding| {
            binding.control_key == "skip_window_classes"
                && binding.binding.field_name == Some("paste_target_skip_class_names")
        }));
        assert!(apply_collect.collect_bindings.iter().any(|binding| {
            binding.control_key == "cloud_webdav_url"
                && binding.binding.field_name == Some("cloud_webdav_url")
        }));
        assert!(apply_collect.collect_bindings.iter().any(|binding| {
            binding.control_key == "capture_enable"
                && binding.binding.field_name == Some("clipboard_capture_enabled")
        }));
        assert!(apply_collect.collect_bindings.iter().any(|binding| {
            binding.control_key == "lan_manual_host"
                && binding.binding.field_name == Some("lan_manual_host")
        }));
        assert!(apply_collect.apply_bindings.iter().any(|binding| {
            binding.control_key == "group_list" && binding.binding.binding_name == "clip_groups"
        }));
        assert!(apply_collect.apply_bindings.iter().any(|binding| {
            binding.control_key == "lan_discovered_list"
                && binding.binding.binding_name == "lan_discovered_devices"
        }));
        assert!(apply_collect.command_route_count > 10);
        assert!(apply_collect.action_route_count > 5);
        assert!(apply_collect.bound_control_count > apply_collect.collect_bindings.len() / 2);
        assert!(apply_collect.summary_label().contains("command_routes="));

        let submission = settings_native_collect_submission(&[
            SettingsNativeSubmittedControlValue {
                control_key: "cloud_webdav_url".to_string(),
                raw_value: " https://dav.example/zsclip ".to_string(),
            },
            SettingsNativeSubmittedControlValue {
                control_key: "lan_manual_host".to_string(),
                raw_value: " 192.168.1.50 ".to_string(),
            },
            SettingsNativeSubmittedControlValue {
                control_key: "unknown_control".to_string(),
                raw_value: "ignored".to_string(),
            },
        ]);
        assert!(submission.applied_fields.iter().any(|field| {
            field.control_key == "cloud_webdav_url"
                && field.field_name == "cloud_webdav_url"
                && field.value == "https://dav.example/zsclip"
        }));
        assert!(submission.applied_fields.iter().any(|field| {
            field.control_key == "lan_manual_host"
                && field.field_name == "lan_manual_host"
                && field.value == "192.168.1.50"
        }));
        assert!(submission
            .missing_control_keys
            .contains(&"skip_window_classes"));
        assert_eq!(
            submission.ignored_control_keys,
            vec!["unknown_control".to_string()]
        );
        assert!(submission.summary_label().contains("submitted_fields="));

        let json_apply = settings_native_apply_submission_to_json(
            serde_json::json!({
                "cloud_sync_enabled": false,
                "lan_sync_enabled": false,
                "cloud_webdav_url": "",
                "lan_manual_host": "",
            }),
            &submission,
        );
        assert!(json_apply.rejected_fields.is_empty());
        assert_eq!(
            json_apply.settings_json["cloud_webdav_url"],
            "https://dav.example/zsclip"
        );
        assert_eq!(json_apply.settings_json["lan_manual_host"], "192.168.1.50");
        assert!(json_apply.field_updates.iter().any(|update| {
            update.field_name == "cloud_webdav_url"
                && update.value == serde_json::json!("https://dav.example/zsclip")
        }));

        let typed_submission = settings_native_collect_submission(&[
            SettingsNativeSubmittedControlValue {
                control_key: "multi_sync_mode".to_string(),
                raw_value: "局域网".to_string(),
            },
            SettingsNativeSubmittedControlValue {
                control_key: "lan_tcp_port".to_string(),
                raw_value: "38474".to_string(),
            },
            SettingsNativeSubmittedControlValue {
                control_key: "plugin_ai_clean".to_string(),
                raw_value: "true".to_string(),
            },
            SettingsNativeSubmittedControlValue {
                control_key: "capture_enable".to_string(),
                raw_value: "false".to_string(),
            },
        ]);
        let typed_json =
            settings_native_apply_submission_to_json(serde_json::json!({}), &typed_submission);
        assert_eq!(typed_json.settings_json["cloud_sync_enabled"], false);
        assert_eq!(typed_json.settings_json["lan_sync_enabled"], true);
        assert_eq!(typed_json.settings_json["lan_tcp_port"], 38474);
        assert_eq!(typed_json.settings_json["ai_clean_enabled"], true);
        assert_eq!(typed_json.settings_json["clipboard_capture_enabled"], false);
        assert_eq!(
            settings_native_bool_field_update(
                &typed_json.field_updates,
                "clipboard_capture_enabled"
            ),
            Some(false)
        );
        assert_eq!(
            settings_native_bool_field_update(&typed_json.field_updates, "auto_start"),
            None
        );

        let dropdown_json = serde_json::json!({
            "max_items": 0,
            "show_pos_mode": "fixed",
            "search_engine": "google",
            "cloud_sync_enabled": false,
            "lan_sync_enabled": true,
            "lan_receive_mode": "clipboard",
        });
        let dropdown_for_key = |key: &str| {
            let control = control_summaries
                .iter()
                .find(|control| control.key == key)
                .unwrap();
            settings_native_dropdown_options(control, &dropdown_json).unwrap()
        };
        let max_items = dropdown_for_key("max_items");
        assert_eq!(max_items.control_key, "max_items");
        assert!(max_items
            .options
            .iter()
            .any(|option| option.raw_value == "0" && option.label == "无限制"));
        assert_eq!(max_items.options[max_items.selected_index].raw_value, "0");
        let position = dropdown_for_key("position_mode");
        assert_eq!(position.options[position.selected_index].raw_value, "fixed");
        let search_engine = dropdown_for_key("search_engine");
        assert_eq!(
            search_engine.options[search_engine.selected_index].raw_value,
            "google"
        );
        let sync_mode = dropdown_for_key("multi_sync_mode");
        assert_eq!(sync_mode.options[sync_mode.selected_index].raw_value, "lan");
        let lan_receive = dropdown_for_key("lan_receive_mode");
        assert_eq!(
            lan_receive.options[lan_receive.selected_index].raw_value,
            "clipboard"
        );
        let vv_group = control_summaries
            .iter()
            .find(|control| control.key == "vv_group")
            .unwrap();
        assert!(settings_native_dropdown_options(vv_group, &dropdown_json).is_none());
        let vv_group_options = settings_native_vv_group_dropdown_options(
            vv_group,
            &serde_json::json!({
                "vv_source_tab": 1,
                "vv_group_id": 42,
            }),
            [(7, "常用"), (42, "项目片段")],
        )
        .unwrap();
        assert_eq!(vv_group_options.control_key, "vv_group");
        assert_eq!(vv_group_options.options[0].raw_value, "0");
        assert_eq!(vv_group_options.options[0].label, source_tab_all_label(1));
        assert_eq!(
            vv_group_options.options[vv_group_options.selected_index].raw_value,
            "42"
        );
        assert_eq!(
            settings_native_vv_source_tab(&serde_json::json!({"vv_source_tab": 9})),
            0
        );

        let sensitive_submission = settings_native_collect_submission(&[
            SettingsNativeSubmittedControlValue {
                control_key: "cloud_webdav_pass".to_string(),
                raw_value: "secret".to_string(),
            },
            SettingsNativeSubmittedControlValue {
                control_key: "ocr_cloud_token".to_string(),
                raw_value: "ocr-token".to_string(),
            },
            SettingsNativeSubmittedControlValue {
                control_key: "translate_secret".to_string(),
                raw_value: "translate-secret".to_string(),
            },
        ]);
        let sensitive_json =
            settings_native_apply_submission_to_json(serde_json::json!({}), &sensitive_submission);
        assert!(sensitive_json.field_updates.is_empty());
        let mut rejected_fields = sensitive_json.rejected_fields;
        rejected_fields.sort();
        assert_eq!(
            rejected_fields,
            vec![
                "cloud_webdav_pass".to_string(),
                "image_ocr_cloud_token".to_string(),
                "text_translate_secret".to_string(),
            ]
        );

        let settings_json = serde_json::json!({
            "cloud_sync_enabled": false,
            "lan_sync_enabled": true,
            "cloud_webdav_url": "https://dav.example/zsclip",
            "cloud_webdav_pass": "secret",
            "lan_tcp_port": 38475,
            "ai_clean_enabled": true,
        });
        let display_for_key = |key: &str| {
            let control = control_summaries
                .iter()
                .find(|control| control.key == key)
                .unwrap();
            settings_native_control_display_value(control, &settings_json).unwrap()
        };
        let sync_display = display_for_key("multi_sync_mode");
        assert_eq!(sync_display.value, "局域网");
        assert!(!sync_display.sensitive);
        let url_display = display_for_key("cloud_webdav_url");
        assert_eq!(url_display.value, "https://dav.example/zsclip");
        assert!(!url_display.sensitive);
        let port_display = display_for_key("lan_tcp_port");
        assert_eq!(port_display.value, "38475");
        let ai_display = display_for_key("plugin_ai_clean");
        assert_eq!(ai_display.value, "true");
        let password_display = display_for_key("cloud_webdav_pass");
        assert!(password_display.value.is_empty());
        assert!(password_display.sensitive);
    }

    #[test]
    fn settings_form_action_layouts_are_platform_neutral() {
        let section = SettingsSection {
            title: "layout",
            rect: UiRect::new(
                settings_scale(200),
                settings_scale(100),
                settings_scale(700),
                settings_scale(500),
            ),
        };
        let layout = SettingsFormSectionLayout::from_section(section, 70);

        assert_eq!(
            layout.label_rect(1, settings_scale(24)),
            UiRect::new(
                settings_scale(218),
                settings_scale(196),
                settings_scale(288),
                settings_scale(220)
            )
        );
        assert_eq!(
            layout.field_label_rect(1, settings_scale(24)),
            UiRect::new(
                settings_scale(288),
                settings_scale(196),
                settings_scale(682),
                settings_scale(220)
            )
        );
        assert_eq!(
            layout.field_row_rect(1),
            UiRect::new(
                settings_scale(288),
                settings_scale(192),
                settings_scale(682),
                settings_scale(224)
            )
        );
        assert_eq!(
            layout.field_full_rect(1, settings_scale(40)),
            UiRect::new(
                settings_scale(288),
                settings_scale(192),
                settings_scale(682),
                settings_scale(232)
            )
        );
        assert_eq!(
            layout.field_sized_row_rect(1, settings_scale(100)),
            UiRect::new(
                settings_scale(288),
                settings_scale(192),
                settings_scale(388),
                settings_scale(224)
            )
        );
        assert_eq!(
            layout.field_rect(1, settings_scale(100), settings_scale(32)),
            UiRect::new(
                settings_scale(288),
                settings_scale(192),
                settings_scale(388),
                settings_scale(224)
            )
        );

        let labeled = layout.labeled_field_layout(
            1,
            settings_scale(24),
            settings_scale(100),
            settings_scale(32),
        );
        assert_eq!(
            labeled.label_rect,
            UiRect::new(
                settings_scale(218),
                settings_scale(196),
                settings_scale(288),
                settings_scale(220)
            )
        );
        assert_eq!(
            labeled.field_rect,
            UiRect::new(
                settings_scale(288),
                settings_scale(192),
                settings_scale(388),
                settings_scale(224)
            )
        );

        assert_eq!(
            layout.action_row_rects(1, &[settings_scale(100), settings_scale(120)]),
            vec![
                UiRect::new(
                    settings_scale(218),
                    settings_scale(192),
                    settings_scale(318),
                    settings_scale(224)
                ),
                UiRect::new(
                    settings_scale(352),
                    settings_scale(192),
                    settings_scale(472),
                    settings_scale(224)
                ),
            ]
        );

        let toggle = layout.toggle_row_layout(1);
        assert_eq!(
            toggle.label_rect,
            UiRect::new(
                settings_scale(218),
                settings_scale(192),
                settings_scale(626),
                settings_scale(224)
            )
        );
        assert_eq!(
            toggle.toggle_rect,
            UiRect::new(
                settings_scale(638),
                settings_scale(196),
                settings_scale(682),
                settings_scale(220)
            )
        );

        let action_hint = layout.field_action_hint_layout(
            1,
            settings_scale(130),
            settings_scale(24),
            settings_scale(4),
            settings_scale(24),
        );
        assert_eq!(
            action_hint.action_rect,
            UiRect::new(
                settings_scale(288),
                settings_scale(192),
                settings_scale(418),
                settings_scale(224)
            )
        );
        assert_eq!(
            action_hint.hint_rect,
            UiRect::new(
                settings_scale(442),
                settings_scale(196),
                settings_scale(682),
                settings_scale(220)
            )
        );

        let qr = layout.qr_action_layout(2);
        assert_eq!(
            qr.qr_rect,
            UiRect::new(
                settings_scale(288),
                settings_scale(232),
                settings_scale(400),
                settings_scale(344)
            )
        );
        assert_eq!(
            qr.action_rect,
            UiRect::new(
                settings_scale(416),
                settings_scale(232),
                settings_scale(558),
                settings_scale(264)
            )
        );
    }

    #[test]
    fn settings_toggle_row_raw_layout_is_platform_neutral() {
        let row = settings_toggle_row_layout_for_rect(UiRect::new(20, 40, 300, 72), 44, 24, 12, 40);

        assert_eq!(row.label_rect, UiRect::new(20, 40, 244, 72));
        assert_eq!(row.toggle_rect, UiRect::new(256, 44, 300, 68));

        let narrow =
            settings_toggle_row_layout_for_rect(UiRect::new(20, 40, 80, 72), 44, 24, 12, 40);
        assert_eq!(narrow.label_rect, UiRect::new(20, 40, 60, 72));
        assert_eq!(narrow.toggle_rect, UiRect::new(36, 44, 80, 68));
    }

    #[test]
    fn settings_form_layout_selects_dynamic_section_then_page_fallback() {
        let dynamic = [SettingsSection {
            title: "dynamic",
            rect: UiRect::new(
                settings_scale(300),
                settings_scale(120),
                settings_scale(760),
                settings_scale(360),
            ),
        }];

        let layout =
            settings_form_layout_for_section(SettingsPage::Plugin.index(), 0, 88, &dynamic);
        assert_eq!(layout.left(), settings_scale(318));
        assert_eq!(layout.label_w(), settings_scale(88));
        assert_eq!(layout.full_w(), settings_scale(424));

        let fallback = settings_form_layout_for_section(SettingsPage::Plugin.index(), 1, 88, &[]);
        let expected = SettingsFormSectionLayout::from_section(
            settings_section(SettingsPage::Plugin.index(), 1),
            88,
        );
        assert_eq!(fallback.left(), expected.left());
        assert_eq!(fallback.label_w(), expected.label_w());
        assert_eq!(fallback.full_w(), expected.full_w());
        assert_eq!(fallback.row_y(0), expected.row_y(0));
    }

    #[test]
    fn settings_flow_layout_advances_content_without_host_state() {
        let mut flow = SettingsFlowLayout::new(20, 100, 300);

        assert_eq!(flow.full_rect(40), UiRect::new(20, 100, 320, 140));
        assert_eq!(flow.consume_full(40, 8), UiRect::new(20, 100, 320, 140));
        assert_eq!(flow.full_rect(10), UiRect::new(20, 148, 320, 158));
        assert_eq!(
            flow.row_label_rect(70, 24, 2),
            UiRect::new(20, 150, 90, 174)
        );
        assert_eq!(flow.row_field_rect(70, 34), UiRect::new(90, 148, 320, 182));
        flow.consume_row(34, 10);
        assert_eq!(flow.full_rect(10), UiRect::new(20, 192, 320, 202));
        assert_eq!(flow.button_rect(184, 32), UiRect::new(20, 192, 204, 224));
    }

    #[test]
    fn settings_update_presentation_maps_state_without_host_dependencies() {
        let checking = settings_update_presentation(&SettingsUpdatePresentationInput {
            checking: true,
            ..Default::default()
        });
        assert_eq!(
            checking.status_text,
            tr("检查更新中…", "Checking for updates...")
        );
        assert_eq!(checking.button_text, tr("检测中…", "Checking..."));

        let idle = settings_update_presentation(&SettingsUpdatePresentationInput::default());
        assert_eq!(
            idle.status_text,
            tr(
                "点击下方按钮后再检查更新。",
                "Click the button below to check for updates.",
            )
        );
        assert_eq!(idle.button_text, tr("检查更新", "Check for updates"));

        let available = settings_update_presentation(&SettingsUpdatePresentationInput {
            started: true,
            available: true,
            latest_tag: "v9.9.9".to_string(),
            ..Default::default()
        });
        assert!(available.status_text.contains("v9.9.9"));
        assert_eq!(
            available.button_text,
            tr("点击下载最新版本", "Click to download latest version")
        );

        let error = settings_update_presentation(&SettingsUpdatePresentationInput {
            started: true,
            error: "network".to_string(),
            ..Default::default()
        });
        assert!(error.status_text.contains("network"));
        assert_eq!(error.button_text, tr("再次检查", "Check again"));

        let current = settings_update_presentation(&SettingsUpdatePresentationInput {
            started: true,
            ..Default::default()
        });
        assert_eq!(
            current.status_text,
            tr(
                "当前已经是最新版本。",
                "You are already on the latest version."
            )
        );
        assert_eq!(current.button_text, tr("再次检查", "Check again"));
    }

    #[test]
    fn settings_ui_model_tracks_pages_controls_and_scroll_slots_without_hwnd() {
        let page = SettingsPage::Plugin.index();
        let mut model = SettingsUiModel::new();
        let first = SettingsControlState {
            id: 10,
            page,
            bounds: UiRect::new(0, 100, 100, 150),
            scrollable: true,
            visible: true,
        };
        let second = SettingsControlState {
            id: 20,
            page,
            bounds: UiRect::new(0, 100, 100, 320),
            scrollable: true,
            visible: true,
        };

        assert!(!model.is_built(page));
        model.mark_built(page);
        assert!(model.is_built(page));
        model.register(first);
        model.register(second);
        assert_eq!(model.controls_for_page(page).count(), 2);
        assert_eq!(model.scroll_controls_for_page(page).count(), 2);
        assert_eq!(
            model.measured_content_total_h(page),
            settings_control_content_bottom(second.bounds) - settings_content_y_scaled()
        );

        model.set_control_visible(20, false);
        assert_eq!(
            model.measured_content_total_h(page),
            settings_control_content_bottom(first.bounds) - settings_content_y_scaled()
        );
        assert!(
            !model
                .scroll_controls_for_page(page)
                .find(|slot| slot.id == 20)
                .unwrap()
                .visible
        );

        model.set_control_bounds(10, UiRect::new(0, 100, 100, 260));
        assert_eq!(
            model.measured_content_total_h(page),
            settings_control_content_bottom(UiRect::new(0, 100, 100, 260))
                - settings_content_y_scaled()
        );

        model.clear_page(page);
        assert!(!model.is_built(page));
        assert_eq!(model.controls_for_page(page).count(), 0);
        assert_eq!(model.scroll_controls_for_page(page).count(), 0);
        assert_eq!(model.measured_content_total_h(page), 0);
    }

    #[test]
    fn settings_scroll_layout_maps_thumb_track_and_drag_without_win32_rects() {
        let layout = SettingsScrollLayout::new(100, 600, 1000, 500, 800, 3, 5);

        assert_eq!(layout.max_scroll(), 500);
        assert_eq!(layout.track_rect(), Some(UiRect::new(792, 108, 797, 592)));
        assert_eq!(layout.thumb_rect(0), Some(UiRect::new(792, 108, 797, 350)));
        assert_eq!(
            layout.thumb_rect(250),
            Some(UiRect::new(792, 229, 797, 471))
        );
        assert_eq!(
            layout.track_hit_rect(4, 2),
            Some(UiRect::new(788, 104, 799, 596))
        );
        assert_eq!(
            layout.thumb_hit_rect(250, 4),
            Some(UiRect::new(788, 229, 801, 471))
        );
        assert_eq!(layout.track_click_scroll_target(108), Some(0));
        assert_eq!(layout.track_click_scroll_target(350), Some(250));
        assert_eq!(layout.track_click_scroll_target(592), Some(500));
        assert_eq!(layout.drag_scroll_target(229, 250, 471), Some(500));
        assert_eq!(layout.drag_scroll_target(229, 250, -13), Some(0));
    }

    #[test]
    fn settings_pointer_down_target_reports_nav_thumb_track_and_none_without_host_state() {
        let layout = SettingsScrollLayout::new(100, 600, 1000, 500, 800, 3, 5);
        let cloud = settings_nav_item_rect(SettingsPage::Cloud.index());

        assert_eq!(
            settings_pointer_down_target(
                cloud.left + 4,
                cloud.top + 4,
                SETTINGS_PAGE_COUNT,
                layout,
                250,
                4,
                4,
                2,
            ),
            SettingsPointerDownTarget::NavPage(SettingsPage::Cloud.index())
        );
        assert_eq!(
            settings_pointer_down_target(790, 300, SETTINGS_PAGE_COUNT, layout, 250, 4, 4, 2),
            SettingsPointerDownTarget::ScrollbarThumb {
                drag_start_y: 300,
                drag_start_scroll: 250,
            }
        );
        assert_eq!(
            settings_pointer_down_target(790, 592, SETTINGS_PAGE_COUNT, layout, 250, 4, 4, 2),
            SettingsPointerDownTarget::ScrollbarTrack { scroll_y: 500 }
        );
        assert_eq!(
            settings_pointer_down_target(500, 300, SETTINGS_PAGE_COUNT, layout, 250, 4, 4, 2),
            SettingsPointerDownTarget::None
        );
        assert_eq!(
            settings_pointer_down_target(790, 300, SETTINGS_PAGE_COUNT, layout, 250, 0, 0, 0),
            SettingsPointerDownTarget::None
        );
    }

    #[test]
    fn settings_scroll_layout_for_window_uses_shared_viewport_metrics() {
        let window = UiRect::new(0, 0, 900, 700);
        let layout = settings_scroll_layout_for_window(window, settings_scale(1200), 3, 5);

        assert_eq!(layout.content_top, settings_content_y_scaled());
        assert_eq!(layout.viewport_bottom, 700);
        assert_eq!(layout.viewport_height, 700 - settings_content_y_scaled());
        assert_eq!(layout.content_height, settings_scale(1200));
        assert_eq!(layout.right, 900);
        assert_eq!(layout.margin, 3);
        assert_eq!(layout.bar_width, 5);
    }

    #[test]
    fn settings_scrollbar_render_plan_describes_visible_state_without_host_renderer() {
        crate::app_core::set_settings_ui_dpi(96);
        let window = UiRect::new(0, 0, 1100, 740);
        let content_height = 1200;
        let normal = settings_scrollbar_render_plan(
            window,
            content_height,
            60,
            true,
            false,
            SCROLL_BAR_MARGIN,
            SCROLL_BAR_W,
            SCROLL_BAR_W_ACTIVE,
        )
        .unwrap();

        assert_eq!(normal.state, SettingsScrollbarVisualState::Normal);
        assert_eq!(normal.bar_width, SCROLL_BAR_W);
        assert_eq!(normal.track_rect, None);
        assert_eq!(
            normal.thumb_rect,
            settings_scroll_layout_for_window(
                window,
                content_height,
                SCROLL_BAR_MARGIN,
                SCROLL_BAR_W
            )
            .thumb_rect(60)
            .unwrap()
        );

        let dragging = settings_scrollbar_render_plan(
            window,
            content_height,
            60,
            true,
            true,
            SCROLL_BAR_MARGIN,
            SCROLL_BAR_W,
            SCROLL_BAR_W_ACTIVE,
        )
        .unwrap();
        let active_layout = settings_scroll_layout_for_window(
            window,
            content_height,
            SCROLL_BAR_MARGIN,
            SCROLL_BAR_W_ACTIVE,
        );

        assert_eq!(dragging.state, SettingsScrollbarVisualState::Dragging);
        assert_eq!(dragging.bar_width, SCROLL_BAR_W_ACTIVE);
        assert_eq!(dragging.track_rect, active_layout.track_rect());
        assert_eq!(dragging.thumb_rect, active_layout.thumb_rect(60).unwrap());

        assert_eq!(
            settings_scrollbar_render_plan(
                window,
                content_height,
                0,
                false,
                true,
                SCROLL_BAR_MARGIN,
                SCROLL_BAR_W,
                SCROLL_BAR_W_ACTIVE,
            ),
            None
        );
        assert_eq!(
            settings_scrollbar_render_plan(
                window,
                100,
                0,
                true,
                false,
                SCROLL_BAR_MARGIN,
                SCROLL_BAR_W,
                SCROLL_BAR_W_ACTIVE,
            ),
            None
        );
    }

    #[test]
    fn settings_qr_cache_builds_compact_runs_without_host_state() {
        let qr =
            settings_qr_cache_for_payload("http://127.0.0.1:38473/mobile/setup").expect("qr cache");

        assert_eq!(qr.payload, "http://127.0.0.1:38473/mobile/setup");
        assert!(qr.size > 0);
        assert!(!qr.runs.is_empty());
        assert!(qr
            .runs
            .iter()
            .all(|run| run.len > 0 && run.x >= 0 && run.y >= 0));
        assert!(qr.runs.iter().all(|run| run.x + run.len <= qr.size));

        let changed = settings_qr_cache_for_payload("http://127.0.0.1:38473/other")
            .expect("changed qr cache");
        assert_ne!(changed.runs, qr.runs);
    }

    #[test]
    fn settings_qr_render_plan_maps_runs_to_rects_without_host_state() {
        let qr = SettingsQrCache {
            payload: "test".to_string(),
            size: 3,
            runs: vec![
                SettingsQrRun { x: 0, y: 0, len: 2 },
                SettingsQrRun { x: 2, y: 1, len: 1 },
            ],
        };
        let plan =
            settings_qr_render_plan(UiRect::new(0, 0, 100, 80), &qr, 1, 10).expect("render plan");

        assert_eq!(plan.module_size, 14);
        assert_eq!(plan.white_rect, UiRect::new(15, 5, 85, 75));
        assert_eq!(
            plan.module_rects,
            vec![UiRect::new(29, 19, 57, 33), UiRect::new(57, 33, 71, 47)]
        );
        assert_eq!(
            settings_qr_render_plan(UiRect::new(0, 0, 0, 80), &qr, 1, 10),
            None
        );
    }

    #[test]
    fn settings_scroll_layout_disables_scrollbar_when_content_fits() {
        let layout = SettingsScrollLayout::new(100, 600, 480, 500, 800, 3, 5);

        assert_eq!(layout.max_scroll(), 0);
        assert_eq!(layout.track_rect(), None);
        assert_eq!(layout.thumb_rect(0), None);
        assert_eq!(layout.track_click_scroll_target(350), None);
        assert_eq!(layout.drag_scroll_target(100, 0, 200), None);
    }
}

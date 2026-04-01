use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::gdiplus;
use crate::time_utils::{gregorian_to_days, utc_secs_to_local_parts};
use crate::win_system_ui::{
    draw_translated_text_block, draw_translated_text_line, is_dark_mode, system_accent,
};

use windows_sys::Win32::{
    Foundation::RECT,
    Graphics::Gdi::{
        BitBlt, CreateCompatibleDC, DeleteDC, CreateSolidBrush, DeleteObject, FillRect,
        GetStockObject, RoundRect, SelectObject,
        NULL_PEN, SRCCOPY,
        CreateDIBSection, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, BI_RGB,
    },
};

// Registry API 鈥?Win32_System_Registry feature not enabled, declare manually
pub const DT_LEFT: u32 = 0x0000;
pub const DT_CENTER: u32 = 0x0001;
pub const DT_VCENTER: u32 = 0x0004;
pub const DT_WORDBREAK: u32 = 0x0010;
pub const DT_SINGLELINE: u32 = 0x0020;
pub const DT_END_ELLIPSIS: u32 = 0x00008000;
pub const DT_NOPREFIX: u32 = 0x00000800;
pub const TRANSPARENT: i32 = 1;

pub const SETTINGS_PAGES: [&str; 6] = ["常规", "快捷键", "插件", "分组", "云同步", "关于"];
pub const SETTINGS_NAV_GLYPHS: [&str; 6] = ["", "", "", "", "", ""];
pub const SETTINGS_W: i32 = 1100;
pub const SETTINGS_H: i32 = 740;
pub const SETTINGS_NAV_W: i32 = 236;
pub const SETTINGS_TOP_H: i32 = 84;
pub const SETTINGS_NAV_Y: i32 = 72;
pub const SETTINGS_CONTENT_X: i32 = SETTINGS_NAV_W + 28;

static SETTINGS_UI_DPI: AtomicU32 = AtomicU32::new(96);
static DARK_ICON_CACHE: OnceLock<Mutex<HashMap<(isize, i32, i32, u8), Vec<u32>>>> = OnceLock::new();

pub fn set_settings_ui_dpi(dpi: u32) {
    SETTINGS_UI_DPI.store(dpi.max(96), Ordering::Relaxed);
}

pub fn settings_ui_dpi() -> u32 {
    SETTINGS_UI_DPI.load(Ordering::Relaxed).max(96)
}

pub fn settings_scale(value: i32) -> i32 {
    let dpi = settings_ui_dpi() as i64;
    (((value as i64) * dpi) + 48) as i32 / 96
}

pub fn settings_w_scaled() -> i32 { settings_scale(SETTINGS_W) }
pub fn settings_h_scaled() -> i32 { settings_scale(SETTINGS_H) }
pub fn settings_nav_w_scaled() -> i32 { settings_scale(SETTINGS_NAV_W) }
pub fn settings_top_h_scaled() -> i32 { settings_scale(SETTINGS_TOP_H) }
pub fn settings_content_x_scaled() -> i32 { settings_scale(SETTINGS_CONTENT_X) }
pub fn settings_content_w_scaled() -> i32 { settings_w_scaled() - settings_content_x_scaled() - settings_scale(28) }
pub fn settings_content_y_scaled() -> i32 { settings_top_h_scaled() }

pub fn ui_text_font_family() -> &'static str {
    crate::win_system_ui::system_ui_text_font_family()
}

pub fn ui_display_font_family() -> &'static str {
    crate::win_system_ui::system_ui_text_font_family()
}

pub fn ui_icon_font_family() -> &'static str {
    "Segoe MDL2 Assets"
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UiRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl UiRect {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self { left, top, right, bottom }
    }

    pub const fn offset_y(self, dy: i32) -> Self {
        Self {
            left: self.left,
            top: self.top - dy,
            right: self.right,
            bottom: self.bottom - dy,
        }
    }
}

#[cfg(target_os = "windows")]
impl From<windows_sys::Win32::Foundation::RECT> for UiRect {
    fn from(value: windows_sys::Win32::Foundation::RECT) -> Self {
        Self {
            left: value.left,
            top: value.top,
            right: value.right,
            bottom: value.bottom,
        }
    }
}

#[cfg(target_os = "windows")]
impl From<&windows_sys::Win32::Foundation::RECT> for UiRect {
    fn from(value: &windows_sys::Win32::Foundation::RECT) -> Self {
        Self::from(*value)
    }
}

#[cfg(target_os = "windows")]
impl From<UiRect> for windows_sys::Win32::Foundation::RECT {
    fn from(value: UiRect) -> Self {
        Self {
            left: value.left,
            top: value.top,
            right: value.right,
            bottom: value.bottom,
        }
    }
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct Theme {
    pub accent: u32,
    pub accent_hover: u32,
    pub accent_pressed: u32,
    pub bg: u32,
    pub nav_bg: u32,
    pub nav_sel_fill: u32,
    pub surface: u32,
    pub surface2: u32,
    pub stroke: u32,
    pub text: u32,
    pub text_muted: u32,
    pub text_dim: u32,
    pub item_hover: u32,
    pub item_selected: u32,
    pub control_bg: u32,
    pub control_stroke: u32,
    pub button_bg: u32,
    pub button_hover: u32,
    pub button_pressed: u32,
    pub close_hover: u32,
}

impl Default for Theme {
    fn default() -> Self {
        let accent = system_accent();
        let dark = is_dark_mode();
        let accent_r = (accent & 0xFF) as i32;
        let accent_g = ((accent >> 8) & 0xFF) as i32;
        let accent_b = ((accent >> 16) & 0xFF) as i32;
        let accent_hover = rgb(
            ((accent_r as f32 * 0.9 + 255.0 * 0.1) as i32).min(255) as u8,
            ((accent_g as f32 * 0.9 + 255.0 * 0.1) as i32).min(255) as u8,
            ((accent_b as f32 * 0.9 + 255.0 * 0.1) as i32).min(255) as u8,
        );
        let accent_pressed = rgb(
            ((accent_r as f32 * 0.82) as i32).min(255) as u8,
            ((accent_g as f32 * 0.82) as i32).min(255) as u8,
            ((accent_b as f32 * 0.82) as i32).min(255) as u8,
        );

        if dark {
            // WinUI3 Dark theme tokens
            Self {
                accent,
                accent_hover,
                accent_pressed,
                bg:             rgb(32, 32, 32),   // SolidBackgroundFillColorBase dark
                nav_bg:         rgb(40, 40, 40),
                nav_sel_fill:   rgb(58, 58, 58),
                surface:        rgb(44, 44, 44),   // LayerFillColorDefault dark
                surface2:       rgb(50, 50, 50),
                stroke:         rgb(60, 60, 60),   // CardStrokeColorDefault dark
                text:           rgb(255, 255, 255),// TextFillColorPrimary dark
                text_muted:     rgb(162, 162, 162),
                text_dim:       rgb(100, 100, 100),
                item_hover:     rgb(54, 54, 54),
                item_selected:  mix(accent, rgb(44, 44, 44), 0.75),
                control_bg:     rgb(58, 58, 58),   // ControlFillColorDefault dark
                control_stroke: rgb(80, 80, 80),
                button_bg:      rgb(58, 58, 58),
                button_hover:   rgb(68, 68, 68),
                button_pressed: rgb(50, 50, 50),
                close_hover:    rgb(196, 43, 28),
            }
        } else {
            // WinUI3 Light theme tokens
            Self {
                accent,
                accent_hover,
                accent_pressed,
                bg:             rgb(243, 243, 243), // SolidBackgroundFillColorBase
                nav_bg:         rgb(243, 243, 243),
                nav_sel_fill:   rgb(255, 255, 255),
                surface:        rgb(255, 255, 255),
                surface2:       rgb(250, 250, 250),
                stroke:         rgb(229, 229, 229),
                text:           rgb(28, 28, 28),
                text_muted:     rgb(96, 96, 96),
                text_dim:       rgb(160, 160, 160),
                item_hover:     rgb(249, 249, 249),
                item_selected:  mix(accent, rgb(255, 255, 255), 0.85),
                control_bg:     rgb(255, 255, 255),
                control_stroke: rgb(204, 204, 204),
                button_bg:      rgb(255, 255, 255),
                button_hover:   rgb(249, 249, 249),
                button_pressed: rgb(238, 238, 238),
                close_hover:    rgb(196, 43, 28),
            }
        }
    }
}

pub fn rgb(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

fn mix(a: u32, b: u32, t: f32) -> u32 {
    let ar = (a & 0xFF) as f32;
    let ag = ((a >> 8) & 0xFF) as f32;
    let ab = ((a >> 16) & 0xFF) as f32;
    let br = (b & 0xFF) as f32;
    let bg = ((b >> 8) & 0xFF) as f32;
    let bb = ((b >> 16) & 0xFF) as f32;
    rgb(
        (ar + (br - ar) * t).round() as u8,
        (ag + (bg - ag) * t).round() as u8,
        (ab + (bb - ab) * t).round() as u8,
    )
}

pub fn settings_nav_item_rect(index: usize) -> RECT {
    let x = settings_scale(10);
    let y = settings_scale(SETTINGS_NAV_Y + 8 + (index as i32) * 44);
    RECT {
        left: x,
        top: y,
        right: settings_nav_w_scaled() - settings_scale(10),
        bottom: y + settings_scale(36),
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ClipGroup {
    pub(crate) id: i64,
    pub(crate) category: i64,
    pub(crate) name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ClipKind {
    Text,
    Image,
    Phrase,
    Files,
}

#[derive(Clone, Debug)]
pub(crate) struct ClipItem {
    pub(crate) id: i64,
    pub(crate) kind: ClipKind,
    pub(crate) preview: String,
    pub(crate) text: Option<String>,
    pub(crate) source_app: String,
    pub(crate) file_paths: Option<Vec<String>>,
    pub(crate) image_bytes: Option<Vec<u8>>,
    pub(crate) image_path: Option<String>,
    pub(crate) image_width: usize,
    pub(crate) image_height: usize,
    pub(crate) pinned: bool,
    pub(crate) group_id: i64,
    pub(crate) created_at: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ClipListState {
    pub(crate) visible_len: usize,
    pub(crate) tab_index: usize,
    pub(crate) search_on: bool,
    pub(crate) search_text: String,
    pub(crate) hover_idx: i32,
    pub(crate) sel_idx: i32,
    pub(crate) scroll_y: i32,
    pub(crate) current_group_filter: i64,
    pub(crate) tab_group_filters: [i64; 2],
    pub(crate) selected_rows: BTreeSet<i32>,
    pub(crate) selection_anchor: i32,
    pub(crate) context_row: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SearchTimeFilter {
    ExactDay(i64),
    RecentDays(i64),
}

fn current_local_day() -> i64 {
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (y, m, d, _, _, _) = utc_secs_to_local_parts(now_secs);
    gregorian_to_days(y, m, d)
}

fn current_local_year() -> i32 {
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (y, _, _, _, _, _) = utc_secs_to_local_parts(now_secs);
    y
}

fn parse_time_filter(raw: &str) -> Option<SearchTimeFilter> {
    let value = raw.trim().to_lowercase();
    if value.is_empty() {
        return None;
    }
    match value.as_str() {
        "today" | "今天" => Some(SearchTimeFilter::ExactDay(current_local_day())),
        "yesterday" | "昨天" => Some(SearchTimeFilter::ExactDay(current_local_day() - 1)),
        "week" | "本周" | "最近7天" => Some(SearchTimeFilter::RecentDays(7)),
        "month" | "本月" | "最近30天" => Some(SearchTimeFilter::RecentDays(30)),
        _ => {
            if let Some(days) = value
                .strip_suffix('d')
                .or_else(|| value.strip_suffix("day"))
                .or_else(|| value.strip_suffix("days"))
                .or_else(|| value.strip_suffix('天'))
                .and_then(|v| v.trim().parse::<i64>().ok())
            {
                return Some(SearchTimeFilter::RecentDays(days.max(1)));
            }

            if let Some((y, m, d)) = value
                .split_once('-')
                .and_then(|(a, rest)| rest.split_once('-').map(|(b, c)| (a, b, c)))
                .and_then(|(y, m, d)| Some((y.parse::<i32>().ok()?, m.parse::<i32>().ok()?, d.parse::<i32>().ok()?)))
            {
                return Some(SearchTimeFilter::ExactDay(gregorian_to_days(y, m, d)));
            }

            if let Some((m, d)) = value
                .split_once('-')
                .and_then(|(m, d)| Some((m.parse::<i32>().ok()?, d.parse::<i32>().ok()?)))
            {
                return Some(SearchTimeFilter::ExactDay(gregorian_to_days(
                    current_local_year(),
                    m,
                    d,
                )));
            }

            None
        }
    }
}

fn tokenize_search_query(query: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for ch in query.chars() {
        if let Some(active) = quote {
            if ch == active {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }
        match ch {
            '"' | '\'' => {
                quote = Some(ch);
            }
            c if c.is_whitespace() => {
                if !current.trim().is_empty() {
                    tokens.push(current.trim().to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    if !current.trim().is_empty() {
        tokens.push(current.trim().to_string());
    }
    tokens
}

fn prefixed_search_value<'a>(token: &'a str, ascii_prefix: &str, cn_prefixes: &[&str]) -> Option<&'a str> {
    let lower = token.to_lowercase();
    if let Some(value) = lower.strip_prefix(ascii_prefix) {
        let start = token.len().saturating_sub(value.len());
        return Some(&token[start..]);
    }
    for prefix in cn_prefixes {
        if let Some(value) = token.strip_prefix(prefix) {
            return Some(value);
        }
    }
    None
}

fn is_prefixed_search_token(token: &str) -> bool {
    prefixed_search_value(token, "time:", &["时间:", "时间：", "日期:", "日期："]).is_some()
        || prefixed_search_value(token, "date:", &["时间:", "时间：", "日期:", "日期："]).is_some()
        || prefixed_search_value(token, "app:", &["应用:", "应用："]).is_some()
}

fn collect_prefixed_value(tokens: &[String], start: usize, initial: &str) -> (String, usize) {
    let mut parts = Vec::new();
    if !initial.trim().is_empty() {
        parts.push(initial.trim().to_string());
    }
    let mut index = start + 1;
    while index < tokens.len() {
        let token = tokens[index].trim();
        if token.is_empty() || is_prefixed_search_token(token) {
            break;
        }
        parts.push(token.to_string());
        index += 1;
    }
    (parts.join(" ").trim().to_string(), index)
}

pub(crate) fn parse_search_query(query: &str) -> (Vec<String>, Option<SearchTimeFilter>, Option<String>) {
    let mut text_terms = Vec::new();
    let mut time_filter = None;
    let mut app_filter = None;

    let tokens = tokenize_search_query(query);
    let mut index = 0usize;
    while index < tokens.len() {
        let token = tokens[index].trim();
        if token.is_empty() {
            index += 1;
            continue;
        }

        if let Some(initial) = prefixed_search_value(token, "time:", &["时间:", "时间：", "日期:", "日期："]) 
            .or_else(|| prefixed_search_value(token, "date:", &["时间:", "时间：", "日期:", "日期："])) {
            let (value, next_index) = collect_prefixed_value(&tokens, index, initial);
            if let Some(filter) = parse_time_filter(&value) {
                time_filter = Some(filter);
                index = next_index;
                continue;
            }
        }

        if let Some(initial) = prefixed_search_value(token, "app:", &["应用:", "应用："]) {
            let (value, next_index) = collect_prefixed_value(&tokens, index, initial);
            if !value.trim().is_empty() {
                app_filter = Some(value.trim().to_lowercase());
                index = next_index;
                continue;
            }
        }

        text_terms.push(token.to_lowercase());
        index += 1;
    }

    (text_terms, time_filter, app_filter)
}

impl Default for ClipListState {
    fn default() -> Self {
        Self {
            visible_len: 0,
            tab_index: 0,
            search_on: false,
            search_text: String::new(),
            hover_idx: -1,
            sel_idx: -1,
            scroll_y: 0,
            current_group_filter: 0,
            tab_group_filters: [0, 0],
            selected_rows: BTreeSet::new(),
            selection_anchor: -1,
            context_row: -1,
        }
    }
}

impl ClipListState {
    pub(crate) fn apply_visible_len(&mut self, len: usize) {
        self.visible_len = len;
        self.sync_visible_state();
    }

    fn sync_visible_state(&mut self) {
        if self.sel_idx >= self.visible_len as i32 {
            self.sel_idx = if self.visible_len == 0 { -1 } else { 0 };
        }
        if self.hover_idx >= self.visible_len as i32 {
            self.hover_idx = -1;
        }
        let max_idx = self.visible_len as i32;
        self.selected_rows = self
            .selected_rows
            .iter()
            .copied()
            .filter(|i| *i >= 0 && *i < max_idx)
            .collect();
        if self.sel_idx >= max_idx {
            self.sel_idx = if max_idx > 0 { max_idx - 1 } else { -1 };
        }
    }

    pub(crate) fn clear_selection(&mut self) {
        self.sel_idx = -1;
        self.hover_idx = -1;
        self.selected_rows.clear();
        self.selection_anchor = -1;
        self.context_row = -1;
    }

    pub(crate) fn row_is_selected(&self, visible_idx: i32) -> bool {
        visible_idx >= 0 && (self.sel_idx == visible_idx || self.selected_rows.contains(&visible_idx))
    }

    pub(crate) fn selected_visible_rows(&self) -> Vec<i32> {
        let mut rows: Vec<i32> = self.selected_rows.iter().copied().collect();
        if self.sel_idx >= 0 && !rows.contains(&self.sel_idx) {
            rows.push(self.sel_idx);
        }
        rows.sort_unstable();
        rows
    }

    pub(crate) fn selected_source_indices(&self) -> Vec<usize> {
        let mut src: Vec<usize> = self
            .selected_visible_rows()
            .into_iter()
            .filter(|v| *v >= 0 && (*v as usize) < self.visible_len)
            .map(|v| v as usize)
            .collect();
        src.sort_unstable();
        src.dedup();
        src
    }

    pub(crate) fn selected_count(&self) -> usize {
        self.selected_source_indices().len()
    }

    pub(crate) fn context_selection_count(&self) -> usize {
        let n = self.selected_count();
        if n == 0 && self.context_row >= 0 { 1 } else { n }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct MainUiLayout {
    pub(crate) win_w: i32,
    pub(crate) title_h: i32,
    pub(crate) seg_x: i32,
    pub(crate) seg_y: i32,
    pub(crate) seg_w: i32,
    pub(crate) seg_h: i32,
    pub(crate) list_x: i32,
    pub(crate) list_y: i32,
    pub(crate) list_w: i32,
    pub(crate) list_h: i32,
    pub(crate) list_pad: i32,
    pub(crate) row_h: i32,
    pub(crate) btn_w: i32,
    pub(crate) btn_gap: i32,
    pub(crate) search_left: i32,
    pub(crate) search_top: i32,
    pub(crate) search_w: i32,
    pub(crate) search_h: i32,
}

impl MainUiLayout {
    pub(crate) const fn zsclip() -> Self {
        Self {
            win_w: 300,
            title_h: 35,
            seg_x: 6,
            seg_y: 36,
            seg_w: 288,
            seg_h: 30,
            list_x: 6,
            list_y: 70,
            list_w: 288,
            list_h: 538,
            list_pad: 4,
            row_h: 44,
            btn_w: 32,
            btn_gap: 2,
            search_left: 58,
            search_top: 4,
            search_w: 112,
            search_h: 30,
        }
    }

    pub(crate) fn scaled(self, dpi: u32) -> Self {
        fn scale(value: i32, dpi: u32) -> i32 {
            let dpi = (dpi.max(96)) as i32;
            ((value * dpi) + 48) / 96
        }
        Self {
            win_w: scale(self.win_w, dpi),
            title_h: scale(self.title_h, dpi),
            seg_x: scale(self.seg_x, dpi),
            seg_y: scale(self.seg_y, dpi),
            seg_w: scale(self.seg_w, dpi),
            seg_h: scale(self.seg_h, dpi),
            list_x: scale(self.list_x, dpi),
            list_y: scale(self.list_y, dpi),
            list_w: scale(self.list_w, dpi),
            list_h: scale(self.list_h, dpi),
            list_pad: scale(self.list_pad, dpi),
            row_h: scale(self.row_h, dpi),
            btn_w: scale(self.btn_w, dpi),
            btn_gap: scale(self.btn_gap, dpi),
            search_left: scale(self.search_left, dpi),
            search_top: scale(self.search_top, dpi),
            search_w: scale(self.search_w, dpi),
            search_h: scale(self.search_h, dpi),
        }
    }

    pub(crate) fn list_view_height(self) -> i32 {
        self.list_h - 2 * self.list_pad
    }

    pub(crate) fn total_content_height(self, filtered_len: usize) -> i32 {
        filtered_len as i32 * self.row_h
    }

    pub(crate) fn max_scroll(self, filtered_len: usize) -> i32 {
        (self.total_content_height(filtered_len) - self.list_view_height()).max(0)
    }

    pub(crate) fn clamp_scroll(self, scroll_y: i32, filtered_len: usize) -> i32 {
        scroll_y.clamp(0, self.max_scroll(filtered_len))
    }

    pub(crate) fn ensure_visible(self, scroll_y: i32, idx: i32, filtered_len: usize) -> i32 {
        if idx < 0 {
            return self.clamp_scroll(scroll_y, filtered_len);
        }
        let top = idx * self.row_h;
        let bottom = top + self.row_h;
        let view_top = scroll_y;
        let view_bottom = scroll_y + self.list_view_height();
        let next = if top < view_top {
            top
        } else if bottom > view_bottom {
            bottom - self.list_view_height()
        } else {
            scroll_y
        };
        self.clamp_scroll(next, filtered_len)
    }

    pub(crate) fn row_rect(self, visible_idx: i32, filtered_len: usize, scroll_y: i32) -> Option<UiRect> {
        if visible_idx < 0 || visible_idx >= filtered_len as i32 {
            return None;
        }
        let inner_l = self.list_x + self.list_pad;
        let inner_t = self.list_y + self.list_pad;
        let y0 = inner_t + visible_idx * self.row_h - scroll_y;
        Some(UiRect::new(
            inner_l,
            y0,
            inner_l + self.list_w - 2 * self.list_pad,
            y0 + self.row_h,
        ))
    }

    pub(crate) fn quick_action_rect(
        self,
        visible_idx: i32,
        filtered_len: usize,
        scroll_y: i32,
        slot: i32,
    ) -> Option<UiRect> {
        let row = self.row_rect(visible_idx, filtered_len, scroll_y)?;
        let size = (self.row_h * 16 / 44).max(16);
        let gap = (self.row_h * 8 / 44).max(8);
        let right_pad = (self.row_h * 10 / 44).max(10);
        let icon_offset = (self.row_h * 12 / 44).max(12);
        let left = row.right - right_pad - size - icon_offset - slot.max(0) * (size + gap);
        let top = row.top + (self.row_h - size) / 2;
        Some(UiRect::new(left, top, left + size, top + size))
    }

    pub(crate) fn row_icon_rect(
        self,
        visible_idx: i32,
        filtered_len: usize,
        scroll_y: i32,
    ) -> Option<UiRect> {
        let row = self.row_rect(visible_idx, filtered_len, scroll_y)?;
        let size = (self.row_h * 18 / 44).max(16);
        let left_pad = (self.row_h * 12 / 44).max(10);
        let left = row.left + left_pad;
        let top = row.top + (self.row_h - size) / 2;
        Some(UiRect::new(left, top, left + size, top + size))
    }

    pub(crate) fn row_pin_rect(
        self,
        visible_idx: i32,
        filtered_len: usize,
        scroll_y: i32,
    ) -> Option<UiRect> {
        let row = self.row_rect(visible_idx, filtered_len, scroll_y)?;
        let size = (self.row_h * 16 / 44).max(16);
        let left_pad = (self.row_h * 32 / 44).max(24);
        let left = row.left + left_pad;
        let top = row.top + (self.row_h - size) / 2;
        Some(UiRect::new(left, top, left + size, top + size))
    }

    pub(crate) fn scroll_to_top_button_rect(self) -> UiRect {
        let size = (self.row_h * 36 / 44).max(32);
        let margin = (self.row_h * 10 / 44).max(10);
        UiRect::new(
            self.list_x + self.list_w - self.list_pad - size - margin,
            self.list_y + self.list_h - self.list_pad - size - margin,
            self.list_x + self.list_w - self.list_pad - margin,
            self.list_y + self.list_h - self.list_pad - margin,
        )
    }

    pub(crate) fn search_rect(self) -> UiRect {
        UiRect::new(
            self.search_left,
            self.search_top,
            self.search_left + self.search_w,
            self.search_top + self.search_h,
        )
    }

    pub(crate) fn title_button_rect(self, key: &str) -> UiRect {
        let x_close = self.win_w - 4 - self.btn_w;
        let x_min = x_close - self.btn_gap - self.btn_w;
        let x_set = x_min - self.btn_gap - self.btn_w;
        let x_search = x_set - self.btn_gap - self.btn_w;
        let x = match key {
            "search" => x_search,
            "setting" => x_set,
            "min" => x_min,
            _ => x_close,
        };
        let top = (self.title_h - self.btn_w) / 2;
        UiRect::new(x, top, x + self.btn_w, top + self.btn_w)
    }

    pub(crate) fn segment_rects(self) -> (UiRect, UiRect) {
        let inner_l = self.seg_x + 1;
        let inner_t = self.seg_y + 1;
        let inner_w = self.seg_w - 2;
        let inner_h = self.seg_h - 2;
        let gap = 1;
        let btn_w = (inner_w - gap) / 2;
        (
            UiRect::new(inner_l, inner_t, inner_l + btn_w, inner_t + inner_h),
            UiRect::new(inner_l + btn_w + gap, inner_t, inner_l + inner_w, inner_t + inner_h),
        )
    }

    pub(crate) fn scrollbar_track_rect(self, filtered_len: usize) -> Option<UiRect> {
        if self.total_content_height(filtered_len) <= self.list_view_height() {
            return None;
        }
        let track_gap = (self.row_h * 8 / 44).max(8);
        let track_w = (self.row_h * 6 / 44).max(4);
        Some(UiRect::new(
            self.list_x + self.list_w - self.list_pad - track_gap - 2,
            self.list_y + self.list_pad + 2,
            self.list_x + self.list_w - self.list_pad - 2 + (track_w - 6).max(0),
            self.list_y + self.list_h - self.list_pad - 2,
        ))
    }

    pub(crate) fn scrollbar_thumb_rect(self, filtered_len: usize, scroll_y: i32) -> Option<UiRect> {
        let track = self.scrollbar_track_rect(filtered_len)?;
        let track_h = track.bottom - track.top;
        let total_h = self.total_content_height(filtered_len);
        let view_h = self.list_view_height();
        let thumb_h = ((track_h as f32) * (view_h as f32 / total_h as f32)) as i32;
        let thumb_h = thumb_h.max(28);
        let max_scroll = self.max_scroll(filtered_len).max(1);
        let thumb_y = track.top + ((track_h - thumb_h) as f32 * (scroll_y as f32 / max_scroll as f32)) as i32;
        Some(UiRect::new(track.left + 1, thumb_y, track.right - 1, thumb_y + thumb_h))
    }
}
pub unsafe fn draw_round_rect(hdc: *mut core::ffi::c_void, rc: &RECT, fill: u32, border: u32, radius: i32) {
    if gdiplus::draw_round_rect(hdc, rc.left, rc.top, rc.right, rc.bottom, fill, border, radius.max(1)) {
        return;
    }
    let er = (radius.max(1)) * 2;
    if border != 0 && border != fill {
        let outer_pen = GetStockObject(NULL_PEN);
        let outer_br = CreateSolidBrush(border);
        let old_pen = SelectObject(hdc, outer_pen as _);
        let old_br = SelectObject(hdc, outer_br as _);
        RoundRect(hdc, rc.left, rc.top, rc.right, rc.bottom, er, er);
        SelectObject(hdc, old_pen);
        SelectObject(hdc, old_br);
        DeleteObject(outer_br as _);

        let inner = RECT {
            left: rc.left + 1,
            top: rc.top + 1,
            right: rc.right - 1,
            bottom: rc.bottom - 1,
        };
        if inner.right > inner.left && inner.bottom > inner.top {
            let inner_br = CreateSolidBrush(fill);
            let old_pen2 = SelectObject(hdc, outer_pen as _);
            let old_br2 = SelectObject(hdc, inner_br as _);
            let inner_r = (radius - 1).max(1) * 2;
            RoundRect(hdc, inner.left, inner.top, inner.right, inner.bottom, inner_r, inner_r);
            SelectObject(hdc, old_pen2);
            SelectObject(hdc, old_br2);
            DeleteObject(inner_br as _);
        }
    } else {
        let pen = GetStockObject(NULL_PEN);
        let brush = CreateSolidBrush(fill);
        let old_pen = SelectObject(hdc, pen as _);
        let old_br = SelectObject(hdc, brush as _);
        RoundRect(hdc, rc.left, rc.top, rc.right, rc.bottom, er, er);
        SelectObject(hdc, old_pen);
        SelectObject(hdc, old_br);
        DeleteObject(brush as _);
    }
}

pub unsafe fn draw_round_fill(hdc: *mut core::ffi::c_void, rc: &RECT, fill: u32, radius: i32) {
    if gdiplus::draw_round_rect(hdc, rc.left, rc.top, rc.right, rc.bottom, fill, fill, radius.max(1)) {
        return;
    }
    let er = (radius.max(1)) * 2;
    let pen = GetStockObject(NULL_PEN);
    let brush = CreateSolidBrush(fill);
    let old_pen = SelectObject(hdc, pen as _);
    let old_br = SelectObject(hdc, brush as _);
    RoundRect(hdc, rc.left, rc.top, rc.right, rc.bottom, er, er);
    SelectObject(hdc, old_pen);
    SelectObject(hdc, old_br);
    DeleteObject(brush as _);
}

pub unsafe fn draw_main_segment_bar(
    hdc: *mut core::ffi::c_void,
    outer: &RECT,
    tab0: &RECT,
    tab1: &RECT,
    selected: i32,
    hover: i32,
    th: Theme,
) {
    draw_round_rect(hdc, outer, th.surface, th.stroke, 4);

    let mut sel_rc = *tab0;
    if selected == 1 { sel_rc = *tab1; }
    let inner_sel = RECT {
        left: sel_rc.left + 2,
        top: sel_rc.top + 2,
        right: sel_rc.right - 2,
        bottom: sel_rc.bottom - 2,
    };
    let selected_fill = if th.bg == rgb(255, 255, 255) { th.surface2 } else { th.nav_sel_fill };
    draw_round_rect(hdc, &inner_sel, selected_fill, th.stroke, 3);

    if hover == 0 && selected != 0 {
        let hr = RECT { left: tab0.left + 2, top: tab0.top + 2, right: tab0.right - 2, bottom: tab0.bottom - 2 };
        draw_round_fill(hdc, &hr, th.item_hover, 3);
    }
    if hover == 1 && selected != 1 {
        let hr = RECT { left: tab1.left + 2, top: tab1.top + 2, right: tab1.right - 2, bottom: tab1.bottom - 2 };
        draw_round_fill(hdc, &hr, th.item_hover, 3);
    }

    let t0c = if selected == 0 || hover == 0 { th.text } else { th.text_muted };
    let t1c = if selected == 1 || hover == 1 { th.text } else { th.text_muted };
    let tab_font = ui_display_font_family();
    draw_text_ex(hdc, "复制记录", tab0, t0c, 13, false, true, tab_font);
    draw_text_ex(hdc, "常用短语", tab1, t1c, 13, false, true, tab_font);
}

pub unsafe fn draw_text(hdc: *mut core::ffi::c_void, text: &str, rc: &RECT, color: u32, size: i32, bold: bool, center: bool) {
    draw_text_ex(hdc, text, rc, color, size, bold, center, ui_text_font_family());
}

pub unsafe fn draw_text_block(
    hdc: *mut core::ffi::c_void,
    text: &str,
    rc: &RECT,
    color: u32,
    size: i32,
    bold: bool,
) {
    draw_text_block_ex(hdc, text, rc, color, size, bold, ui_text_font_family());
}

pub unsafe fn draw_text_ex(
    hdc: *mut core::ffi::c_void,
    text: &str,
    rc: &RECT,
    color: u32,
    size: i32,
    bold: bool,
    center: bool,
    family: &str,
) {
    let weight = if bold { 700 } else { 400 };
    let mut rc2 = *rc;
    draw_translated_text_line(
        hdc,
        text,
        &mut rc2,
        color,
        size,
        weight,
        center,
        family,
        TRANSPARENT,
        0,
    );
}

pub unsafe fn draw_text_block_ex(
    hdc: *mut core::ffi::c_void,
    text: &str,
    rc: &RECT,
    color: u32,
    size: i32,
    bold: bool,
    family: &str,
) {
    let weight = if bold { 700 } else { 400 };
    let mut rc2 = *rc;
    draw_translated_text_block(
        hdc,
        text,
        &mut rc2,
        color,
        size,
        weight,
        family,
        TRANSPARENT,
        0,
    );
}

pub fn rgba_to_bgra(bytes: &[u8]) -> Vec<u8> {
    if bytes.len() < 4 {
        return bytes.to_vec();
    }
    let mut out = bytes.to_vec();
    for px in out.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
    out
}

/// 鍦ㄦ繁鑹叉ā寮忎笅缁樺埗鍥炬爣鏃讹紝灏嗛粦鑹插浘鏍囧弽鑹蹭负鐧借壊銆?
/// 娣辫壊妯″紡涓嬫妸鍥炬爣鍙嶈壊涓虹櫧鑹茬増鏈紙涓ゆ缁樺埗鎻愬彇鐪熷疄鍍忕礌锛?
pub unsafe fn draw_icon_tinted(
    hdc: *mut core::ffi::c_void,
    x: i32, y: i32,
    icon: isize,
    w: i32, h: i32,
    dark: bool,
) {
    draw_icon_tinted_soft(hdc, x, y, icon, w, h, dark, 0);
}

pub unsafe fn draw_icon_tinted_soft(
    hdc: *mut core::ffi::c_void,
    x: i32,
    y: i32,
    icon: isize,
    w: i32,
    h: i32,
    dark: bool,
    soften: u8,
) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{DrawIconEx, DI_NORMAL};
    if icon == 0 { return; }
    if !dark {
        DrawIconEx(hdc, x, y, icon as _, w, h, 0, std::ptr::null_mut(), DI_NORMAL);
        return;
    }
    let n = (w * h) as usize;
    let derived = {
        let cache = DARK_ICON_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        let mut cache = match cache.lock() {
            Ok(cache) => cache,
            Err(_) => {
                DrawIconEx(hdc, x, y, icon as _, w, h, 0, std::ptr::null_mut(), DI_NORMAL);
                return;
            }
        };
        if let Some(cached) = cache.get(&(icon, w, h, soften)) {
            cached.clone()
        } else {
            let make_dib = |bg: u32| -> (*mut core::ffi::c_void, *mut core::ffi::c_void, *mut u32) {
                let dc = CreateCompatibleDC(hdc);
                let mut bmi: BITMAPINFO = core::mem::zeroed();
                bmi.bmiHeader.biSize = core::mem::size_of::<BITMAPINFOHEADER>() as u32;
                bmi.bmiHeader.biWidth = w;
                bmi.bmiHeader.biHeight = -h;
                bmi.bmiHeader.biPlanes = 1;
                bmi.bmiHeader.biBitCount = 32;
                bmi.bmiHeader.biCompression = BI_RGB;
                let mut ptr: *mut core::ffi::c_void = core::ptr::null_mut();
                let dib = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &mut ptr, core::ptr::null_mut(), 0);
                if dib.is_null() || ptr.is_null() {
                    DeleteDC(dc);
                    return (core::ptr::null_mut(), core::ptr::null_mut(), core::ptr::null_mut());
                }
                SelectObject(dc, dib as _);
                let br = CreateSolidBrush(bg);
                let rc = RECT { left: 0, top: 0, right: w, bottom: h };
                FillRect(dc, &rc, br);
                DeleteObject(br as _);
                DrawIconEx(dc, 0, 0, icon as _, w, h, 0, core::ptr::null_mut(), DI_NORMAL);
                (dc, dib, ptr as *mut u32)
            };

            let (dc_w, dib_w, px_w) = make_dib(0x00FFFFFFu32);
            let (dc_b, dib_b, px_b) = make_dib(0x00000000u32);
            if dc_w.is_null() || dc_b.is_null() {
                if !dc_w.is_null() { DeleteDC(dc_w); }
                if !dc_b.is_null() { DeleteDC(dc_b); }
                DrawIconEx(hdc, x, y, icon as _, w, h, 0, std::ptr::null_mut(), DI_NORMAL);
                return;
            }

            let src_w = core::slice::from_raw_parts(px_w, n);
            let src_b = core::slice::from_raw_parts(px_b, n);
            let mut derived = vec![0u32; n];
            for i in 0..n {
                let w_px = src_w[i];
                let b_px = src_b[i];
                let wr = ((w_px >> 16) & 0xFF) as i32;
                let wg = ((w_px >> 8) & 0xFF) as i32;
                let wb = (w_px & 0xFF) as i32;
                let br = ((b_px >> 16) & 0xFF) as i32;
                let bg = ((b_px >> 8) & 0xFF) as i32;
                let bb = (b_px & 0xFF) as i32;
                let ar = 255 - (wr - br).clamp(0, 255);
                let ag = 255 - (wg - bg).clamp(0, 255);
                let ab = 255 - (wb - bb).clamp(0, 255);
                let alpha = ((ar + ag + ab) / 3).clamp(0, 255) as u32;
                if alpha < 8 {
                    continue;
                }
                let icon_r = ((br * 255 / ar.max(1)) as u32).min(255);
                let icon_g = ((bg * 255 / ag.max(1)) as u32).min(255);
                let icon_b = ((bb * 255 / ab.max(1)) as u32).min(255);
                let lum = (icon_r * 299 + icon_g * 587 + icon_b * 114) / 1000;
                let (mut out_r, mut out_g, mut out_b) = if lum < 80 {
                    (255u32, 255u32, 255u32)
                } else if lum < 200 {
                    let bright = (255 - lum + 180).min(255);
                    (bright, bright, bright)
                } else {
                    (icon_r, icon_g, icon_b)
                };
                if soften > 0 {
                    let k = soften as u32;
                    let base = 32u32;
                    out_r = ((out_r * (255 - k)) + (base * k)) / 255;
                    out_g = ((out_g * (255 - k)) + (base * k)) / 255;
                    out_b = ((out_b * (255 - k)) + (base * k)) / 255;
                }
                derived[i] = (alpha << 24) | (out_r << 16) | (out_g << 8) | out_b;
            }
            DeleteObject(dib_w as _);
            DeleteObject(dib_b as _);
            DeleteDC(dc_w);
            DeleteDC(dc_b);
            cache.insert((icon, w, h, soften), derived.clone());
            derived
        }
    };

    let dc_bg = CreateCompatibleDC(hdc);
    let mut bmi_bg: BITMAPINFO = core::mem::zeroed();
    bmi_bg.bmiHeader.biSize = core::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi_bg.bmiHeader.biWidth = w;
    bmi_bg.bmiHeader.biHeight = -h;
    bmi_bg.bmiHeader.biPlanes = 1;
    bmi_bg.bmiHeader.biBitCount = 32;
    bmi_bg.bmiHeader.biCompression = BI_RGB;
    let mut px_bg_ptr: *mut core::ffi::c_void = core::ptr::null_mut();
    let dib_bg = CreateDIBSection(dc_bg, &bmi_bg, DIB_RGB_COLORS, &mut px_bg_ptr, core::ptr::null_mut(), 0);
    SelectObject(dc_bg, dib_bg as _);
    BitBlt(dc_bg, 0, 0, w, h, hdc, x, y, SRCCOPY);
    let src_bg = if !dib_bg.is_null() && !px_bg_ptr.is_null() {
        core::slice::from_raw_parts(px_bg_ptr as *const u32, n)
    } else {
        &[] as &[u32]
    };

    let dc_out = CreateCompatibleDC(hdc);
    let mut bmi_out: BITMAPINFO = core::mem::zeroed();
    bmi_out.bmiHeader.biSize = core::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi_out.bmiHeader.biWidth = w;
    bmi_out.bmiHeader.biHeight = -h;
    bmi_out.bmiHeader.biPlanes = 1;
    bmi_out.bmiHeader.biBitCount = 32;
    bmi_out.bmiHeader.biCompression = BI_RGB;
    let mut px_out_ptr: *mut core::ffi::c_void = core::ptr::null_mut();
    let dib_out = CreateDIBSection(dc_out, &bmi_out, DIB_RGB_COLORS, &mut px_out_ptr, core::ptr::null_mut(), 0);
    if dib_out.is_null() || px_out_ptr.is_null() {
        if !dib_bg.is_null() { DeleteObject(dib_bg as _); }
        DeleteDC(dc_bg);
        DeleteDC(dc_out);
        DrawIconEx(hdc, x, y, icon as _, w, h, 0, std::ptr::null_mut(), DI_NORMAL);
        return;
    }
    SelectObject(dc_out, dib_out as _);
    let dst = core::slice::from_raw_parts_mut(px_out_ptr as *mut u32, n);

    let blend = |fg: u32, bg: u32, a: u32| -> u32 { (fg * a + bg * (255 - a)) / 255 };
    for i in 0..n {
        let fg = derived[i];
        let alpha = (fg >> 24) & 0xFF;
        let (bg_r, bg_g, bg_b) = if i < src_bg.len() {
            let bg_px = src_bg[i];
            (((bg_px >> 16) & 0xFF) as u32,
             ((bg_px >> 8) & 0xFF) as u32,
             (bg_px & 0xFF) as u32)
        } else {
            (32, 32, 32)
        };
        let out_r = (fg >> 16) & 0xFF;
        let out_g = (fg >> 8) & 0xFF;
        let out_b = fg & 0xFF;
        let final_r = blend(out_r, bg_r, alpha);
        let final_g = blend(out_g, bg_g, alpha);
        let final_b = blend(out_b, bg_b, alpha);
        dst[i] = (final_r << 16) | (final_g << 8) | final_b;
    }

    BitBlt(hdc, x, y, w, h, dc_out, 0, 0, SRCCOPY);

    if !dib_bg.is_null() { DeleteObject(dib_bg as _); }
    DeleteObject(dib_out as _);
    DeleteDC(dc_bg);
    DeleteDC(dc_out);
}

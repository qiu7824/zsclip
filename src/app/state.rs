use super::*;

pub(super) const HOTKEY_MOD_OPTIONS: [&str; 8] = [
    "Win",
    "Ctrl",
    "Alt",
    "Shift",
    "Ctrl+Alt",
    "Ctrl+Shift",
    "Alt+Shift",
    "Ctrl+Alt+Shift",
];

pub(super) const HOTKEY_KEY_OPTIONS: [&str; 51] = [
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R",
    "S", "T", "U", "V", "W", "X", "Y", "Z", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9",
    "Space", "Enter", "Tab", "Esc", "Backspace", "Delete", "Insert", "Up", "Down", "Left",
    "Right", "Home", "End", "PageUp", "PageDown",
];

pub(super) const SEARCH_ENGINE_PRESETS: [(&str, &str, &str); 12] = [
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

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct AppSettings {
    pub(crate) hotkey_enabled: bool,
    pub(crate) hotkey_mod: String,
    pub(crate) hotkey_key: String,
    pub(crate) silent_start: bool,
    pub(crate) tray_icon_enabled: bool,
    pub(crate) click_hide: bool,
    pub(crate) auto_hide_on_blur: bool,
    pub(crate) edge_auto_hide: bool,
    pub(crate) hover_preview: bool,
    pub(crate) auto_start: bool,
    pub(crate) close_without_exit: bool,
    pub(crate) max_items: usize,
    pub(crate) show_pos_mode: String,
    pub(crate) show_mouse_dx: i32,
    pub(crate) show_mouse_dy: i32,
    pub(crate) show_fixed_x: i32,
    pub(crate) show_fixed_y: i32,
    pub(crate) quick_search_enabled: bool,
    pub(crate) vv_mode_enabled: bool,
    pub(crate) vv_source_tab: usize,
    pub(crate) vv_group_id: i64,
    pub(crate) image_preview_enabled: bool,
    pub(crate) quick_delete_button: bool,
    pub(crate) move_pasted_item_to_top: bool,
    pub(crate) dedupe_filter_enabled: bool,
    pub(crate) search_engine: String,
    pub(crate) search_template: String,
    pub(crate) ai_clean_enabled: bool,
    pub(crate) super_mail_merge_enabled: bool,
    pub(crate) grouping_enabled: bool,
    pub(crate) cloud_sync_enabled: bool,
    pub(crate) cloud_sync_interval: String,
    pub(crate) cloud_webdav_url: String,
    pub(crate) cloud_webdav_user: String,
    pub(crate) cloud_webdav_pass: String,
    pub(crate) cloud_remote_dir: String,
    pub(crate) cloud_last_sync_status: String,
    pub(crate) last_window_x: i32,
    pub(crate) last_window_y: i32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            hotkey_enabled: true,
            hotkey_mod: "Win".to_string(),
            hotkey_key: "V".to_string(),
            silent_start: false,
            tray_icon_enabled: true,
            click_hide: false,
            auto_hide_on_blur: false,
            edge_auto_hide: false,
            hover_preview: false,
            auto_start: false,
            close_without_exit: true,
            max_items: 200,
            show_pos_mode: "mouse".to_string(),
            show_mouse_dx: 12,
            show_mouse_dy: 12,
            show_fixed_x: 120,
            show_fixed_y: 120,
            quick_search_enabled: false,
            vv_mode_enabled: true,
            vv_source_tab: 0,
            vv_group_id: 0,
            image_preview_enabled: false,
            quick_delete_button: true,
            move_pasted_item_to_top: false,
            dedupe_filter_enabled: false,
            search_engine: "jzxx".to_string(),
            search_template: search_engine_template("jzxx").to_string(),
            ai_clean_enabled: false,
            super_mail_merge_enabled: false,
            grouping_enabled: true,
            cloud_sync_enabled: false,
            cloud_sync_interval: "1小时".to_string(),
            cloud_webdav_url: String::new(),
            cloud_webdav_user: String::new(),
            cloud_webdav_pass: String::new(),
            cloud_remote_dir: "ZSClip".to_string(),
            cloud_last_sync_status: "未同步".to_string(),
            last_window_x: -1,
            last_window_y: -1,
        }
    }
}

pub(super) fn search_engine_template(key: &str) -> &'static str {
    SEARCH_ENGINE_PRESETS
        .iter()
        .find(|(k, _, _)| *k == key)
        .map(|(_, _, tpl)| *tpl)
        .unwrap_or(SEARCH_ENGINE_PRESETS[0].2)
}

pub(super) fn search_engine_display(key: &str) -> String {
    SEARCH_ENGINE_PRESETS
        .iter()
        .find(|(k, _, _)| *k == key)
        .map(|(_, name, _)| translate(name).into_owned())
        .unwrap_or_else(|| translate(SEARCH_ENGINE_PRESETS[0].1).into_owned())
}

pub(super) fn search_engine_key_from_display(label: &str) -> &'static str {
    SEARCH_ENGINE_PRESETS
        .iter()
        .find(|(_, name, _)| *name == label || translate(name).as_ref() == label)
        .map(|(k, _, _)| *k)
        .unwrap_or("jzxx")
}

pub(super) fn group_name_for_display(
    groups: &[ClipGroup],
    group_id: i64,
    all_label: &str,
) -> String {
    if group_id == 0 {
        return all_label.to_string();
    }
    groups
        .iter()
        .find(|g| g.id == group_id)
        .map(|g| g.name.clone())
        .unwrap_or_else(|| all_label.to_string())
}

pub(super) fn normalize_source_tab(tab: usize) -> usize {
    if tab == 1 { 1 } else { 0 }
}

pub(super) fn source_tab_category(tab: usize) -> i64 {
    normalize_source_tab(tab) as i64
}

pub(super) fn source_tab_all_label(tab: usize) -> &'static str {
    if normalize_source_tab(tab) == 1 {
        "全部短语"
    } else {
        "全部记录"
    }
}

pub(super) fn source_tab_label(tab: usize) -> &'static str {
    if normalize_source_tab(tab) == 1 {
        tr("常用短语", "Phrases")
    } else {
        tr("复制记录", "Clipboard Records")
    }
}

pub(super) fn normalize_hotkey_mod(value: &str) -> String {
    let trimmed = value.trim();
    if HOTKEY_MOD_OPTIONS.contains(&trimmed) {
        trimmed.to_string()
    } else {
        "Win".to_string()
    }
}

pub(super) fn normalize_hotkey_key(value: &str) -> String {
    let trimmed = value.trim();
    if HOTKEY_KEY_OPTIONS.contains(&trimmed) {
        trimmed.to_string()
    } else {
        "V".to_string()
    }
}

pub(super) fn hotkey_preview_text(mod_label: &str, key_label: &str) -> String {
    format!(
        "{}{} + {}",
        tr("当前设置：", "Current setting: "),
        normalize_hotkey_mod(mod_label),
        normalize_hotkey_key(key_label)
    )
}

pub(super) fn startup_can_hide(settings: &AppSettings) -> bool {
    settings.silent_start && (settings.tray_icon_enabled || settings.hotkey_enabled)
}

pub(super) fn tray_mode_enabled(settings: &AppSettings) -> bool {
    settings.tray_icon_enabled
}

pub(super) fn close_to_tray_enabled(settings: &AppSettings) -> bool {
    tray_mode_enabled(settings) && settings.close_without_exit
}

pub(super) fn title_button_visible(_settings: &AppSettings, _key: &str) -> bool {
    true
}

#[derive(Default)]
pub(super) struct VvHookState {
    pub(super) main_hwnd: isize,
    pub(super) enabled: bool,
    pub(super) last_v_target: isize,
    pub(super) last_was_v: bool,
    pub(super) last_v_at: Option<Instant>,
    pub(super) popup_active: bool,
    pub(super) popup_target: isize,
    pub(super) popup_menu_active: bool,
    pub(super) popup_menu_grace_until: Option<Instant>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ItemsQuery {
    pub(super) category: i64,
    pub(super) group_id: i64,
    pub(super) search_text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ItemsCursor {
    pub(super) pinned: bool,
    pub(super) id: i64,
}

pub(super) struct TabLoadState {
    pub(super) query: Option<ItemsQuery>,
    pub(super) next_cursor: Option<ItemsCursor>,
    pub(super) has_more: bool,
    pub(super) loading: bool,
    pub(super) request_seq: u64,
    pub(super) error: Option<String>,
}

impl Default for TabLoadState {
    fn default() -> Self {
        Self {
            query: None,
            next_cursor: None,
            has_more: true,
            loading: false,
            request_seq: 0,
            error: None,
        }
    }
}

const ITEM_PAYLOAD_CACHE_LIMIT: usize = 256;
const IMAGE_THUMB_CACHE_LIMIT: usize = 96;

#[derive(Default)]
pub(super) struct ItemPayloadCache {
    entries: HashMap<i64, ClipItem>,
    order: VecDeque<i64>,
}

impl ItemPayloadCache {
    pub(super) fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }

    pub(super) fn shrink_to_fit(&mut self) {
        self.entries.shrink_to_fit();
        self.order.shrink_to_fit();
    }

    pub(super) fn remove(&mut self, id: i64) {
        self.entries.remove(&id);
        self.order.retain(|cached| *cached != id);
    }

    pub(super) fn get(&mut self, id: i64) -> Option<ClipItem> {
        let item = self.entries.get(&id).cloned()?;
        self.touch(id);
        Some(item)
    }

    pub(super) fn put(&mut self, item: ClipItem) {
        let id = item.id;
        if id <= 0 {
            return;
        }
        self.entries.insert(id, item);
        self.touch(id);
        while self.order.len() > ITEM_PAYLOAD_CACHE_LIMIT {
            if let Some(evicted) = self.order.pop_front() {
                self.entries.remove(&evicted);
            }
        }
    }

    fn touch(&mut self, id: i64) {
        self.order.retain(|cached| *cached != id);
        self.order.push_back(id);
    }
}

#[derive(Clone)]
pub(super) struct ImageThumbnail {
    pub(super) bytes: Vec<u8>,
    pub(super) width: usize,
    pub(super) height: usize,
}

#[derive(Default)]
pub(super) struct ImageThumbnailCache {
    entries: HashMap<i64, ImageThumbnail>,
    order: VecDeque<i64>,
}

impl ImageThumbnailCache {
    pub(super) fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }

    pub(super) fn shrink_to_fit(&mut self) {
        self.entries.shrink_to_fit();
        self.order.shrink_to_fit();
    }

    pub(super) fn remove(&mut self, id: i64) {
        self.entries.remove(&id);
        self.order.retain(|cached| *cached != id);
    }

    pub(super) fn get(&mut self, id: i64) -> Option<ImageThumbnail> {
        let image = self.entries.get(&id).cloned()?;
        self.touch(id);
        Some(image)
    }

    pub(super) fn put(&mut self, id: i64, image: ImageThumbnail) {
        if id <= 0 {
            return;
        }
        self.entries.insert(id, image);
        self.touch(id);
        while self.order.len() > IMAGE_THUMB_CACHE_LIMIT {
            if let Some(evicted) = self.order.pop_front() {
                self.entries.remove(&evicted);
            }
        }
    }

    fn touch(&mut self, id: i64) {
        self.order.retain(|cached| *cached != id);
        self.order.push_back(id);
    }
}

#[derive(Clone)]
pub(super) struct VvPopupEntry {
    pub(super) index: usize,
    pub(super) item: ClipItem,
}

#[derive(Clone)]
pub(super) struct PageLoadResult {
    pub(super) hwnd: isize,
    pub(super) tab: usize,
    pub(super) request_seq: u64,
    pub(super) query: ItemsQuery,
    pub(super) reset: bool,
    pub(super) items: Vec<ClipItem>,
    pub(super) next_cursor: Option<ItemsCursor>,
    pub(super) has_more: bool,
    pub(super) error: Option<String>,
}

pub(super) struct CloudSyncResult {
    pub(super) hwnd: isize,
    pub(super) action: CloudSyncAction,
    pub(super) auto_triggered: bool,
    pub(super) result: Result<CloudSyncOutcome, String>,
}

static VV_HOOK_STATE: OnceLock<Mutex<VvHookState>> = OnceLock::new();
static VV_KEYBOARD_HOOK: OnceLock<Mutex<isize>> = OnceLock::new();
static OUTSIDE_HIDE_MOUSE_HOOK: OnceLock<Mutex<isize>> = OnceLock::new();
static QUICK_ESCAPE_KEYBOARD_HOOK: OnceLock<Mutex<isize>> = OnceLock::new();
pub(super) static VV_POPUP_HWND: OnceLock<isize> = OnceLock::new();
static PAGE_LOAD_RESULTS: OnceLock<Mutex<VecDeque<PageLoadResult>>> = OnceLock::new();
static CLOUD_SYNC_RESULTS: OnceLock<Mutex<VecDeque<CloudSyncResult>>> = OnceLock::new();

pub(super) fn vv_hook_state() -> &'static Mutex<VvHookState> {
    VV_HOOK_STATE.get_or_init(|| Mutex::new(VvHookState::default()))
}

pub(super) fn vv_hook_handle() -> &'static Mutex<isize> {
    VV_KEYBOARD_HOOK.get_or_init(|| Mutex::new(0))
}

pub(super) fn outside_hide_mouse_hook_handle() -> &'static Mutex<isize> {
    OUTSIDE_HIDE_MOUSE_HOOK.get_or_init(|| Mutex::new(0))
}

pub(super) fn quick_escape_keyboard_hook_handle() -> &'static Mutex<isize> {
    QUICK_ESCAPE_KEYBOARD_HOOK.get_or_init(|| Mutex::new(0))
}

pub(super) fn page_load_results() -> &'static Mutex<VecDeque<PageLoadResult>> {
    PAGE_LOAD_RESULTS.get_or_init(|| Mutex::new(VecDeque::new()))
}

pub(super) fn cloud_sync_results() -> &'static Mutex<VecDeque<CloudSyncResult>> {
    CLOUD_SYNC_RESULTS.get_or_init(|| Mutex::new(VecDeque::new()))
}

pub(super) fn clear_page_load_results_for_hwnd(hwnd: HWND) {
    if hwnd.is_null() {
        return;
    }
    if let Ok(mut queue) = page_load_results().lock() {
        let target = hwnd as isize;
        queue.retain(|result| result.hwnd != target);
    }
}

pub(super) fn clear_cloud_sync_results_for_hwnd(hwnd: HWND) {
    if hwnd.is_null() {
        return;
    }
    if let Ok(mut queue) = cloud_sync_results().lock() {
        let target = hwnd as isize;
        queue.retain(|result| result.hwnd != target);
    }
}

pub(super) fn vv_popup_menu_active() -> bool {
    vv_hook_state()
        .lock()
        .ok()
        .map(|guard| {
            guard.popup_menu_active
                || guard
                    .popup_menu_grace_until
                    .map(|until| until > Instant::now())
                    .unwrap_or(false)
        })
        .unwrap_or(false)
}

pub(super) fn vv_set_popup_menu_active(active: bool) {
    if let Ok(mut guard) = vv_hook_state().lock() {
        guard.popup_menu_active = active;
        guard.popup_menu_grace_until = if active {
            None
        } else {
            Some(Instant::now() + std::time::Duration::from_millis(VV_POPUP_MENU_GRACE_MS))
        };
    }
}

impl AppState {
    pub(super) fn new(role: WindowRole, hwnd: HWND, search_hwnd: HWND, icons: Icons) -> Self {
        let mut settings = load_settings();
        settings.auto_start = is_autostart_enabled();
        let cloud_sync_next_due = if cloud_sync_should_schedule(&settings) {
            Some(Instant::now() + cloud_sync_interval(&settings.cloud_sync_interval))
        } else {
            None
        };
        Self {
            role,
            hwnd,
            search_hwnd,
            search_font: null_mut(),
            theme: Theme::default(),
            icons,
            records: Vec::new(),
            phrases: Vec::new(),
            record_groups: Vec::new(),
            phrase_groups: Vec::new(),
            list: ClipListState::default(),
            hover_btn: "",
            down_btn: "",
            down_row: -1,
            down_x: 0,
            down_y: 0,
            hover_tab: -1,
            last_signature: String::new(),
            ignore_clipboard_until: None,
            settings,
            tray_icon_registered: false,
            hotkey_registered: false,
            hotkey_conflict_notified: false,
            startup_recovery_ticks: if role == WindowRole::Main {
                STARTUP_RECOVERY_TICKS
            } else {
                0
            },
            settings_hwnd: null_mut(),
            hover_scroll: false,
            scroll_fade_alpha: 0,
            scroll_fade_timer: false,
            scroll_dragging: false,
            scroll_drag_start_y: 0,
            scroll_drag_start_scroll: 0,
            hover_to_top: false,
            down_to_top: false,
            tab_loads: [TabLoadState::default(), TabLoadState::default()],
            payload_cache: ItemPayloadCache::default(),
            image_thumb_cache: ImageThumbnailCache::default(),
            vv_popup_visible: false,
            vv_popup_pending_target: null_mut(),
            vv_popup_pending_retries: 0,
            vv_popup_target: null_mut(),
            vv_popup_replaces_ime: false,
            vv_popup_group_id: 0,
            vv_popup_items: Vec::new(),
            paste_target_override: null_mut(),
            paste_backspace_count: 0,
            hotkey_passthrough_active: false,
            hotkey_passthrough_target: null_mut(),
            hotkey_passthrough_focus: null_mut(),
            hotkey_passthrough_edit: null_mut(),
            main_window_noactivate: false,
            edge_hidden: false,
            edge_hidden_side: EDGE_AUTO_HIDE_NONE,
            edge_restore_x: 0,
            edge_restore_y: 0,
            edge_docked_left: 0,
            edge_docked_top: 0,
            edge_docked_right: 0,
            edge_docked_bottom: 0,
            edge_monitor_left: 0,
            edge_monitor_top: 0,
            edge_monitor_right: 0,
            edge_monitor_bottom: 0,
            edge_hide_armed: false,
            edge_hide_grace_until: None,
            edge_restore_wait_leave: false,
            cloud_sync_in_progress: false,
            cloud_sync_next_due,
        }
    }

    pub(super) fn items_for_tab(&self, tab: usize) -> &Vec<ClipItem> {
        if tab == 0 {
            &self.records
        } else {
            &self.phrases
        }
    }

    pub(super) fn groups_for_tab(&self, tab: usize) -> &Vec<ClipGroup> {
        if normalize_source_tab(tab) == 0 {
            &self.record_groups
        } else {
            &self.phrase_groups
        }
    }

    pub(super) fn release_list_memory(&mut self) {
        self.invalidate_all_queries();
        self.records.clear();
        self.phrases.clear();
        self.record_groups.clear();
        self.phrase_groups.clear();
        self.vv_popup_items.clear();
        self.clear_payload_cache();
        self.clear_selection();
        self.scroll_y = 0;
        self.list.apply_visible_len(0);
        self.record_groups.shrink_to_fit();
        self.phrase_groups.shrink_to_fit();
        self.records.shrink_to_fit();
        self.phrases.shrink_to_fit();
        self.vv_popup_items.shrink_to_fit();
        self.payload_cache.shrink_to_fit();
        self.image_thumb_cache.shrink_to_fit();
    }

    pub(super) fn items_for_tab_mut(&mut self, tab: usize) -> &mut Vec<ClipItem> {
        if tab == 0 {
            &mut self.records
        } else {
            &mut self.phrases
        }
    }

    pub(super) fn active_items(&self) -> &Vec<ClipItem> {
        self.items_for_tab(self.tab_index)
    }

    pub(super) fn load_state_for_tab(&self, tab: usize) -> &TabLoadState {
        &self.tab_loads[tab.min(self.tab_loads.len() - 1)]
    }

    pub(super) fn load_state_for_tab_mut(&mut self, tab: usize) -> &mut TabLoadState {
        let idx = tab.min(self.tab_loads.len() - 1);
        &mut self.tab_loads[idx]
    }

    pub(super) fn active_load_state(&self) -> &TabLoadState {
        self.load_state_for_tab(self.tab_index)
    }

    pub(super) fn current_item(&self) -> Option<&ClipItem> {
        if self.sel_idx < 0 {
            return None;
        }
        let visible_idx = self.sel_idx as usize;
        let src_idx = *self.filtered_indices.get(visible_idx)?;
        self.active_items().get(src_idx)
    }

    pub(super) fn current_item_owned(&self) -> Option<ClipItem> {
        self.current_item().cloned()
    }

    pub(super) fn refilter(&mut self) {
        self.ensure_tab_query_loaded(self.list.tab_index);
        let visible_len = self.active_items().len();
        self.list.apply_visible_len(visible_len);
        self.clamp_scroll();
        self.maybe_request_more_for_active_tab();
    }

    pub(super) fn desired_query_for_tab(&self, tab: usize) -> ItemsQuery {
        let group_id = if self.settings.grouping_enabled {
            self.tab_group_filters.get(tab).copied().unwrap_or(0)
        } else {
            0
        };
        ItemsQuery {
            category: tab as i64,
            group_id,
            search_text: self.search_text.trim().to_string(),
        }
    }

    pub(super) fn invalidate_tab_query(&mut self, tab: usize, clear_items: bool) {
        if tab >= self.tab_loads.len() {
            return;
        }
        let load = self.load_state_for_tab_mut(tab);
        load.request_seq = load.request_seq.wrapping_add(1);
        load.query = None;
        load.next_cursor = None;
        load.has_more = true;
        load.loading = false;
        load.error = None;
        if clear_items {
            self.items_for_tab_mut(tab).clear();
        }
    }

    pub(super) fn invalidate_all_queries(&mut self) {
        self.invalidate_tab_query(0, true);
        self.invalidate_tab_query(1, true);
    }

    pub(super) fn ensure_tab_query_loaded(&mut self, tab: usize) {
        let desired = self.desired_query_for_tab(tab);
        let load = self.load_state_for_tab(tab);
        if load.query.as_ref() == Some(&desired) {
            return;
        }
        self.request_tab_page(tab, desired, None, true);
    }

    pub(super) fn request_tab_page(
        &mut self,
        tab: usize,
        query: ItemsQuery,
        cursor: Option<ItemsCursor>,
        reset: bool,
    ) {
        if tab >= self.tab_loads.len() {
            return;
        }

        let request_seq = {
            let load = self.load_state_for_tab_mut(tab);
            load.request_seq = load.request_seq.wrapping_add(1);
            load.query = Some(query.clone());
            load.loading = true;
            load.error = None;
            if reset {
                load.next_cursor = None;
                load.has_more = true;
            }
            load.request_seq
        };

        if reset {
            self.items_for_tab_mut(tab).clear();
            if tab == self.tab_index {
                self.list.apply_visible_len(0);
                self.clamp_scroll();
            }
        }

        spawn_items_page_load(self.hwnd, tab, request_seq, query, cursor, reset);
    }

    pub(super) fn maybe_request_more_for_active_tab(&mut self) {
        let tab = self.tab_index;
        let (query, cursor, loading, has_more) = {
            let load = self.load_state_for_tab(tab);
            (
                load.query.clone(),
                load.next_cursor,
                load.loading,
                load.has_more,
            )
        };
        if loading || !has_more {
            return;
        }
        let Some(query) = query else {
            return;
        };
        let loaded = self.active_items().len();
        if loaded == 0 {
            return;
        }
        let last_visible = ((self.scroll_y + self.list_view_height()) / self.layout().row_h).max(0) as usize
            + ITEMS_LOAD_AHEAD_ROWS as usize;
        if last_visible >= loaded {
            self.request_tab_page(tab, query, cursor, false);
        }
    }

    pub(super) fn apply_page_load_result(&mut self, result: PageLoadResult) -> bool {
        if result.tab >= self.tab_loads.len() {
            return false;
        }

        let should_apply = {
            let load = self.load_state_for_tab(result.tab);
            load.request_seq == result.request_seq && load.query.as_ref() == Some(&result.query)
        };
        if !should_apply {
            return false;
        }

        {
            let load = self.load_state_for_tab_mut(result.tab);
            load.loading = false;
            load.error = result.error.clone();
            load.next_cursor = result.next_cursor;
            load.has_more = result.has_more;
        }

        if result.error.is_none() {
            let target = self.items_for_tab_mut(result.tab);
            if result.reset {
                *target = result.items;
            } else {
                for item in result.items {
                    if !target.iter().any(|loaded| loaded.id == item.id) {
                        target.push(item);
                    }
                }
            }
        }

        if result.tab == self.tab_index {
            self.list.apply_visible_len(self.active_items().len());
            self.clamp_scroll();
            self.maybe_request_more_for_active_tab();
        }

        true
    }

    pub(super) fn clear_selection(&mut self) {
        self.list.clear_selection();
        self.down_row = -1;
        self.down_x = 0;
        self.down_y = 0;
    }

    pub(super) fn row_is_selected(&self, visible_idx: i32) -> bool {
        self.list.row_is_selected(visible_idx)
    }

    pub(super) fn selected_source_indices(&self) -> Vec<usize> {
        self.list.selected_source_indices()
    }
}


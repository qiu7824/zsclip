use super::prelude::*;
pub(super) use crate::settings_model::{
    hotkey_preview_text, image_ocr_provider_display, image_ocr_provider_key_from_display,
    normalize_hotkey_key, normalize_hotkey_mod, normalize_source_tab, paste_sound_display,
    paste_sound_file_button_text, paste_sound_key_from_display, search_engine_display,
    search_engine_key_from_display, search_engine_template, source_tab_all_label,
    source_tab_category, source_tab_label, text_translate_provider_display,
    text_translate_provider_key_from_display, text_translate_target_display,
    text_translate_target_key_from_display, HOTKEY_KEY_OPTIONS, HOTKEY_MOD_OPTIONS,
    IMAGE_OCR_PROVIDER_OPTIONS, PASTE_SOUND_OPTIONS, SEARCH_ENGINE_PRESETS,
    TEXT_TRANSLATE_PROVIDER_OPTIONS, TEXT_TRANSLATE_TARGET_OPTIONS,
};

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
    pub(crate) clipboard_capture_enabled: bool,
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
    pub(crate) persistent_search_box: bool,
    pub(crate) paste_success_sound_enabled: bool,
    pub(crate) paste_success_sound_kind: String,
    pub(crate) paste_success_sound_path: String,
    pub(crate) paste_target_skip_enabled: bool,
    pub(crate) paste_target_skip_class_names: String,
    pub(crate) search_engine: String,
    pub(crate) search_template: String,
    pub(crate) plain_paste_hotkey_enabled: bool,
    pub(crate) plain_paste_hotkey_mod: String,
    pub(crate) plain_paste_hotkey_key: String,
    pub(crate) ai_clean_enabled: bool,
    pub(crate) super_mail_merge_enabled: bool,
    pub(crate) wps_taskpane_enabled: bool,
    pub(crate) grouping_enabled: bool,
    pub(crate) cloud_sync_enabled: bool,
    pub(crate) cloud_sync_interval: String,
    pub(crate) cloud_webdav_url: String,
    pub(crate) cloud_webdav_user: String,
    pub(crate) cloud_webdav_pass: String,
    pub(crate) cloud_remote_dir: String,
    pub(crate) cloud_last_sync_status: String,
    pub(crate) lan_sync_enabled: bool,
    pub(crate) lan_device_name: String,
    pub(crate) lan_device_id: String,
    pub(crate) lan_tcp_port: u16,
    pub(crate) lan_udp_port: u16,
    pub(crate) lan_manual_host: String,
    pub(crate) lan_last_status: String,
    pub(crate) lan_receive_mode: String,
    pub(crate) image_ocr_provider: String,
    pub(crate) image_ocr_cloud_url: String,
    pub(crate) image_ocr_cloud_token: String,
    pub(crate) image_ocr_wechat_dir: String,
    pub(crate) text_translate_provider: String,
    pub(crate) text_translate_app_id: String,
    pub(crate) text_translate_secret: String,
    pub(crate) text_translate_target_lang: String,
    pub(crate) qr_quick_enabled: bool,
    pub(crate) last_window_x: i32,
    pub(crate) last_window_y: i32,
    pub(crate) edit_dialog_w: i32,
    pub(crate) edit_dialog_h: i32,
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
            clipboard_capture_enabled: true,
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
            dedupe_filter_enabled: true,
            persistent_search_box: false,
            paste_success_sound_enabled: false,
            paste_success_sound_kind: "default".to_string(),
            paste_success_sound_path: String::new(),
            paste_target_skip_enabled: false,
            paste_target_skip_class_names: String::new(),
            search_engine: "jzxx".to_string(),
            search_template: search_engine_template("jzxx").to_string(),
            plain_paste_hotkey_enabled: false,
            plain_paste_hotkey_mod: "Ctrl+Shift".to_string(),
            plain_paste_hotkey_key: "V".to_string(),
            ai_clean_enabled: false,
            super_mail_merge_enabled: false,
            wps_taskpane_enabled: false,
            grouping_enabled: true,
            cloud_sync_enabled: false,
            cloud_sync_interval: "1小时".to_string(),
            cloud_webdav_url: String::new(),
            cloud_webdav_user: String::new(),
            cloud_webdav_pass: String::new(),
            cloud_remote_dir: "ZSClip".to_string(),
            cloud_last_sync_status: "未同步".to_string(),
            lan_sync_enabled: false,
            lan_device_name: String::new(),
            lan_device_id: String::new(),
            lan_tcp_port: 38473,
            lan_udp_port: 38472,
            lan_manual_host: String::new(),
            lan_last_status: "未启动".to_string(),
            lan_receive_mode: "records_only".to_string(),
            image_ocr_provider: "off".to_string(),
            image_ocr_cloud_url: String::new(),
            image_ocr_cloud_token: String::new(),
            image_ocr_wechat_dir: String::new(),
            text_translate_provider: "off".to_string(),
            text_translate_app_id: String::new(),
            text_translate_secret: String::new(),
            text_translate_target_lang: "zh".to_string(),
            qr_quick_enabled: false,
            last_window_x: -1,
            last_window_y: -1,
            edit_dialog_w: 0,
            edit_dialog_h: 0,
        }
    }
}

pub(super) fn group_name_for_display(
    groups: &[ClipGroup],
    group_id: i64,
    all_label: &str,
) -> String {
    crate::settings_model::group_name_for_display_entries(
        groups.iter().map(|group| (group.id, group.name.as_str())),
        group_id,
        all_label,
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(isize)]
pub(crate) enum WindowRole {
    Main = 1,
    Quick = 2,
}

impl WindowRole {
    pub(super) fn from_create_param(value: isize) -> Self {
        match value {
            x if x == WindowRole::Quick as isize => WindowRole::Quick,
            _ => WindowRole::Main,
        }
    }

    pub(super) fn class_name(self) -> &'static str {
        match self {
            WindowRole::Main => CLASS_NAME,
            WindowRole::Quick => QUICK_CLASS_NAME,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WindowCreateParams {
    pub(crate) role: WindowRole,
    pub(crate) min_size: Option<UiSize>,
}

impl WindowCreateParams {
    pub(super) const fn new(role: WindowRole, min_size: Option<UiSize>) -> Self {
        Self { role, min_size }
    }

    pub(super) fn from_create_param(value: isize) -> Self {
        if value == WindowRole::Quick as isize || value == WindowRole::Main as isize {
            return Self::new(WindowRole::from_create_param(value), None);
        }
        let params = value as *const Self;
        if params.is_null() {
            Self::new(WindowRole::Main, None)
        } else {
            unsafe { *params }
        }
    }
}

#[derive(Clone, Copy, Default)]
pub(crate) struct Icons {
    pub(crate) app: isize,
    pub(crate) search: isize,
    pub(crate) setting: isize,
    pub(crate) min: isize,
    pub(crate) close: isize,
    pub(crate) text: isize,
    pub(crate) image: isize,
    pub(crate) file: isize,
    pub(crate) folder: isize,
    pub(crate) pin: isize,
    pub(crate) del: isize,
}

impl Icons {
    pub(super) fn destroy(&mut self) {
        unsafe {
            for icon in [
                &mut self.app,
                &mut self.search,
                &mut self.setting,
                &mut self.min,
                &mut self.close,
                &mut self.text,
                &mut self.image,
                &mut self.file,
                &mut self.folder,
                &mut self.pin,
                &mut self.del,
            ] {
                if *icon != 0 {
                    DestroyIcon(*icon as _);
                    *icon = 0;
                }
            }
        }
    }
}

pub(crate) struct AppState {
    pub(crate) role: WindowRole,
    pub(crate) ui_lifecycle: LifecycleState,
    pub(crate) ui_commands: CommandQueue,
    pub(crate) hwnd: HWND,
    pub(crate) native_min_size: Option<UiSize>,
    pub(crate) search_hwnd: HWND,
    pub(crate) ui_dpi: u32,
    pub(crate) dpi_comp: DpiCompensationState,
    pub(crate) search_font: *mut core::ffi::c_void,
    pub(crate) theme: Theme,
    pub(crate) icons: Icons,
    pub(crate) records: Vec<ClipItem>,
    pub(crate) phrases: Vec<ClipItem>,
    pub(crate) record_groups: Vec<ClipGroup>,
    pub(crate) phrase_groups: Vec<ClipGroup>,
    pub(crate) list: ClipListState,
    pub(crate) hover_btn: &'static str,
    pub(crate) down_btn: &'static str,
    pub(crate) down_row: i32,
    pub(crate) down_x: i32,
    pub(crate) down_y: i32,
    pub(crate) hover_tab: i32,
    pub(crate) last_capture_signature: String,
    pub(crate) last_capture_source_app: String,
    pub(crate) recent_capture_signatures: VecDeque<(String, String, Instant)>,
    pub(crate) recent_lan_message_keys: VecDeque<String>,
    pub(crate) last_capture_at: Option<Instant>,
    pub(crate) last_clipboard_seq: u32,
    pub(crate) ignore_clipboard_until: Option<Instant>,
    pub(crate) skip_next_clipboard_update_once: bool,
    pub(crate) recent_programmatic_clipboard_signature: String,
    pub(crate) recent_programmatic_clipboard_until: Option<Instant>,
    pub(crate) settings: AppSettings,
    pub(crate) tray_icon_registered: bool,
    pub(crate) hotkey_registered: bool,
    pub(crate) plain_paste_hotkey_registered: bool,
    pub(crate) clipboard_listener_registered: bool,
    pub(crate) hotkey_conflict_notified: bool,
    pub(crate) startup_recovery_ticks: u8,
    pub(crate) settings_hwnd: HWND,
    pub(crate) hover_scroll: bool,
    pub(crate) scroll_fade_alpha: u8,
    pub(crate) scroll_fade_timer: bool,
    pub(crate) search_debounce_timer: bool,
    pub(crate) hidden_reclaim_timer: bool,
    pub(crate) clipboard_retry_timer: bool,
    pub(crate) clipboard_retry_sequence: u32,
    pub(crate) clipboard_retry_attempts: u8,
    pub(crate) scroll_dragging: bool,
    pub(crate) scroll_drag_start_y: i32,
    pub(crate) scroll_drag_start_scroll: i32,
    pub(crate) hover_to_top: bool,
    pub(crate) down_to_top: bool,
    pub(super) tab_loads: [TabLoadState; 2],
    pub(super) payload_cache: ItemPayloadCache,
    pub(super) image_thumb_cache: ImageThumbnailCache,
    pub(super) image_thumb_loading: HashSet<i64>,
    pub(crate) vv_popup_visible: bool,
    pub(crate) vv_popup_pending_target: HWND,
    pub(crate) vv_popup_pending_retries: u8,
    pub(crate) vv_popup_target: HWND,
    pub(crate) vv_popup_replaces_ime: bool,
    pub(crate) vv_popup_group_id: i64,
    pub(super) vv_popup_items: Vec<VvPopupEntry>,
    pub(crate) paste_target_override: HWND,
    pub(crate) paste_backspace_count: u8,
    pub(crate) hotkey_passthrough_active: bool,
    pub(crate) hotkey_passthrough_target: HWND,
    pub(crate) hotkey_passthrough_focus: HWND,
    pub(crate) hotkey_passthrough_edit: HWND,
    pub(crate) plain_text_paste_mode: bool,
    pub(crate) main_window_noactivate: bool,
    pub(crate) edge_hidden: bool,
    pub(crate) edge_hidden_side: i32,
    pub(crate) edge_restore_x: i32,
    pub(crate) edge_restore_y: i32,
    pub(crate) edge_docked_left: i32,
    pub(crate) edge_docked_top: i32,
    pub(crate) edge_docked_right: i32,
    pub(crate) edge_docked_bottom: i32,
    pub(crate) edge_hide_armed: bool,
    pub(crate) edge_hide_pending_until: Option<Instant>,
    pub(crate) edge_hide_grace_until: Option<Instant>,
    pub(crate) edge_restore_wait_leave: bool,
    pub(crate) edge_anim_from_x: i32,
    pub(crate) edge_anim_from_y: i32,
    pub(crate) edge_anim_to_x: i32,
    pub(crate) edge_anim_to_y: i32,
    pub(crate) edge_anim_until: Option<Instant>,
    pub(crate) cloud_sync_in_progress: bool,
    pub(crate) cloud_sync_next_due: Option<Instant>,
}

impl Deref for AppState {
    type Target = ClipListState;

    fn deref(&self) -> &Self::Target {
        &self.list
    }
}

impl DerefMut for AppState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.list
    }
}

pub(super) struct VvHookState {
    pub(super) main_hwnd: isize,
    pub(super) enabled: bool,
    pub(super) trigger_vk: u32,
    pub(super) last_v_target: isize,
    pub(super) last_was_v: bool,
    pub(super) last_v_at: Option<Instant>,
    pub(super) popup_active: bool,
    pub(super) popup_target: isize,
    pub(super) popup_menu_active: bool,
    pub(super) popup_menu_grace_until: Option<Instant>,
}

impl Default for VvHookState {
    fn default() -> Self {
        Self {
            main_hwnd: 0,
            enabled: false,
            trigger_vk: b'V' as u32,
            last_v_target: 0,
            last_was_v: false,
            last_v_at: None,
            popup_active: false,
            popup_target: 0,
            popup_menu_active: false,
            popup_menu_grace_until: None,
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
static QUICK_ESCAPE_KEYBOARD_HOOK: OnceLock<Mutex<isize>> = OnceLock::new();
pub(super) static VV_POPUP_HWND: OnceLock<isize> = OnceLock::new();
static PAGE_LOAD_RESULTS: OnceLock<Mutex<VecDeque<PageLoadResult>>> = OnceLock::new();
static CLOUD_SYNC_RESULTS: OnceLock<Mutex<VecDeque<CloudSyncResult>>> = OnceLock::new();
static SHARED_TAB_VIEW_STATE: OnceLock<Mutex<SharedTabViewState>> = OnceLock::new();

pub(super) fn vv_hook_state() -> &'static Mutex<VvHookState> {
    VV_HOOK_STATE.get_or_init(|| Mutex::new(VvHookState::default()))
}

pub(super) fn vv_hook_handle() -> &'static Mutex<isize> {
    VV_KEYBOARD_HOOK.get_or_init(|| Mutex::new(0))
}

pub(super) fn vv_hook_registered() -> bool {
    vv_hook_handle()
        .lock()
        .ok()
        .map(|handle| *handle != 0)
        .unwrap_or(false)
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

fn shared_tab_view_state() -> &'static Mutex<SharedTabViewState> {
    SHARED_TAB_VIEW_STATE.get_or_init(|| Mutex::new(SharedTabViewState::default()))
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
    pub(super) fn new(
        role: WindowRole,
        hwnd: HWND,
        search_hwnd: HWND,
        icons: Icons,
        native_min_size: Option<UiSize>,
    ) -> Self {
        let mut settings = load_settings();
        settings.auto_start = is_autostart_enabled();
        let cloud_sync_next_due = if cloud_sync_should_schedule(&settings) {
            Some(Instant::now() + cloud_sync_interval(&settings.cloud_sync_interval))
        } else {
            None
        };
        let theme = Theme::default();
        let mut state = Self {
            role,
            ui_lifecycle: LifecycleState::new(),
            ui_commands: CommandQueue::default(),
            hwnd,
            native_min_size,
            search_hwnd,
            ui_dpi: unsafe { crate::platform::dpi::layout_dpi_for_window(hwnd) },
            dpi_comp: DpiCompensationState::default(),
            search_font: null_mut(),
            theme,
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
            last_capture_signature: String::new(),
            last_capture_source_app: String::new(),
            recent_capture_signatures: VecDeque::new(),
            recent_lan_message_keys: VecDeque::new(),
            last_capture_at: None,
            last_clipboard_seq: 0,
            ignore_clipboard_until: None,
            skip_next_clipboard_update_once: false,
            recent_programmatic_clipboard_signature: String::new(),
            recent_programmatic_clipboard_until: None,
            settings,
            tray_icon_registered: false,
            hotkey_registered: false,
            plain_paste_hotkey_registered: false,
            clipboard_listener_registered: false,
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
            search_debounce_timer: false,
            hidden_reclaim_timer: false,
            clipboard_retry_timer: false,
            clipboard_retry_sequence: 0,
            clipboard_retry_attempts: 0,
            scroll_dragging: false,
            scroll_drag_start_y: 0,
            scroll_drag_start_scroll: 0,
            hover_to_top: false,
            down_to_top: false,
            tab_loads: [TabLoadState::default(), TabLoadState::default()],
            payload_cache: ItemPayloadCache::default(),
            image_thumb_cache: ImageThumbnailCache::default(),
            image_thumb_loading: HashSet::new(),
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
            plain_text_paste_mode: false,
            main_window_noactivate: false,
            edge_hidden: false,
            edge_hidden_side: EDGE_AUTO_HIDE_NONE,
            edge_restore_x: 0,
            edge_restore_y: 0,
            edge_docked_left: 0,
            edge_docked_top: 0,
            edge_docked_right: 0,
            edge_docked_bottom: 0,
            edge_hide_armed: false,
            edge_hide_pending_until: None,
            edge_hide_grace_until: None,
            edge_restore_wait_leave: false,
            edge_anim_from_x: 0,
            edge_anim_from_y: 0,
            edge_anim_to_x: 0,
            edge_anim_to_y: 0,
            edge_anim_until: None,
            cloud_sync_in_progress: false,
            cloud_sync_next_due,
        };
        apply_shared_tab_view_state(&mut state);
        state
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
        self.recent_capture_signatures.clear();
        self.recent_lan_message_keys.clear();
        self.last_capture_signature.clear();
        self.last_capture_source_app.clear();
        self.hover_btn = "";
        self.down_btn = "";
        self.vv_popup_pending_target = null_mut();
        self.vv_popup_target = null_mut();
        self.paste_target_override = null_mut();
        self.paste_backspace_count = 0;
        self.hotkey_passthrough_active = false;
        self.hotkey_passthrough_target = null_mut();
        self.hotkey_passthrough_focus = null_mut();
        self.hotkey_passthrough_edit = null_mut();
        self.clear_payload_cache();
        self.clear_selection();
        self.scroll_y = 0;
        self.list.apply_visible_len(0);
        self.record_groups.shrink_to_fit();
        self.phrase_groups.shrink_to_fit();
        self.records.shrink_to_fit();
        self.phrases.shrink_to_fit();
        self.vv_popup_items.shrink_to_fit();
        self.recent_capture_signatures.shrink_to_fit();
        self.recent_lan_message_keys.shrink_to_fit();
        self.last_capture_signature.shrink_to_fit();
        self.last_capture_source_app.shrink_to_fit();
        self.list.search_text.shrink_to_fit();
        self.list.selected_rows.clear();
        self.payload_cache.shrink_to_fit();
        self.image_thumb_cache.shrink_to_fit();
        self.image_thumb_loading.shrink_to_fit();
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
        let src_idx = self.visible_src_idx(self.sel_idx as usize)?;
        self.active_items().get(src_idx)
    }

    pub(super) fn visible_count(&self) -> usize {
        self.visible_len
    }

    pub(super) fn visible_src_idx(&self, visible_idx: usize) -> Option<usize> {
        if visible_idx < self.visible_len {
            Some(visible_idx)
        } else {
            None
        }
    }

    pub(super) fn current_item_owned(&self) -> Option<ClipItem> {
        self.current_item().cloned()
    }

    pub(crate) fn refilter(&mut self) {
        self.ensure_tab_query_loaded(self.list.tab_index);
        let visible_len = self.active_items().len();
        self.list.apply_visible_len(visible_len);
        self.clamp_scroll();
        self.maybe_request_more_for_active_tab();
    }

    pub(super) fn desired_query_for_tab(&self, tab: usize) -> ItemsQuery {
        ItemsQuery::for_tab(
            tab,
            self.settings.grouping_enabled,
            self.tab_group_filters,
            &self.search_text,
        )
    }

    pub(super) fn invalidate_tab_query(&mut self, tab: usize, clear_items: bool) {
        if tab >= self.tab_loads.len() {
            return;
        }
        let load = self.load_state_for_tab_mut(tab);
        load.invalidate();
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
            load.begin_request(query.clone(), reset)
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
        let last_visible = ((self.scroll_y + self.list_view_height()) / self.layout().row_h).max(0)
            as usize
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
            load.accepts_result(result.request_seq, &result.query)
        };
        if !should_apply {
            return false;
        }

        {
            let load = self.load_state_for_tab_mut(result.tab);
            load.finish_request(result.error.clone(), result.next_cursor, result.has_more);
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

    pub(super) fn selected_source_indices(&self) -> Vec<usize> {
        self.list.selected_source_indices()
    }
}

pub(crate) fn remember_shared_tab_view_state(state: &AppState) {
    if let Ok(mut shared) = shared_tab_view_state().lock() {
        shared.tab_index = normalize_source_tab(state.tab_index);
        shared.tab_group_filters = state.tab_group_filters;
    }
}

pub(crate) fn apply_shared_tab_view_state(state: &mut AppState) -> bool {
    let Ok(shared) = shared_tab_view_state().lock() else {
        return false;
    };
    let next_tab = normalize_source_tab(shared.tab_index);
    let next_filters = shared.tab_group_filters;
    let changed = state.tab_index != next_tab || state.tab_group_filters != next_filters;
    state.tab_index = next_tab;
    state.tab_group_filters = next_filters;
    state.current_group_filter = next_filters[next_tab];
    changed
}

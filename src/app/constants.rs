use crate::app_core::{MainTimerIds, MainUiLayout};
use crate::win_system_ui::SettingsTimerIds;
use std::io;

pub(in crate::app) const CLASS_NAME: &str = "ZsClipMain";
pub(in crate::app) const QUICK_CLASS_NAME: &str = "ZsClipQuick";

pub(in crate::app) const IDC_SEARCH: isize = 1001;
pub(in crate::app) const ID_TIMER_STARTUP_RECOVERY: usize = 1;
pub(in crate::app) const ID_TIMER_PASTE: usize = 2;
pub(in crate::app) const ID_TIMER_SCROLL_FADE: usize = 3;
pub(in crate::app) const ID_TIMER_SETTINGS_SCROLLBAR: usize = 4;
pub(in crate::app) const ID_TIMER_EDGE_AUTO_HIDE: usize = 5;
pub(in crate::app) const ID_TIMER_VV_SHOW: usize = 6;
pub(in crate::app) const ID_TIMER_CLOUD_SYNC: usize = 7;
pub(in crate::app) const ID_TIMER_SETTINGS_SAVE_HINT: usize = 8;
pub(in crate::app) const ID_TIMER_OUTSIDE_HIDE: usize = 9;
pub(in crate::app) const ID_TIMER_VV_WATCH: usize = 10;
pub(in crate::app) const ID_TIMER_SEARCH_DEBOUNCE: usize = 11;
pub(in crate::app) const ID_TIMER_HIDDEN_RECLAIM: usize = 12;
pub(in crate::app) const ID_TIMER_CLIPBOARD_RETRY: usize = 13;
pub(in crate::app) const ID_TIMER_DPI_FIT: usize = 14;
pub(in crate::app) const ID_TIMER_SETTINGS_DPI_FIT: usize = 15;
pub(in crate::app) const SETTINGS_TIMER_IDS: SettingsTimerIds = SettingsTimerIds {
    hide_scrollbar: ID_TIMER_SETTINGS_SCROLLBAR,
    clear_save_hint: ID_TIMER_SETTINGS_SAVE_HINT,
    dpi_fit: ID_TIMER_SETTINGS_DPI_FIT,
};
pub(in crate::app) const MAIN_TIMER_IDS: MainTimerIds = MainTimerIds {
    startup_recovery: ID_TIMER_STARTUP_RECOVERY,
    vv_watch: ID_TIMER_VV_WATCH,
    vv_show: ID_TIMER_VV_SHOW,
    paste: ID_TIMER_PASTE,
    search_debounce: ID_TIMER_SEARCH_DEBOUNCE,
    hidden_reclaim: ID_TIMER_HIDDEN_RECLAIM,
    clipboard_retry: ID_TIMER_CLIPBOARD_RETRY,
    dpi_fit: ID_TIMER_DPI_FIT,
    scroll_fade: ID_TIMER_SCROLL_FADE,
    edge_auto_hide: ID_TIMER_EDGE_AUTO_HIDE,
    outside_hide: ID_TIMER_OUTSIDE_HIDE,
    cloud_sync: ID_TIMER_CLOUD_SYNC,
};
pub(in crate::app) const STARTUP_RECOVERY_TICKS: u8 = 24;
pub(crate) const TRAY_UID: u32 = 1;
pub(in crate::app) const HOTKEY_ID: i32 = 1;
pub(in crate::app) const HOTKEY_ID_PLAIN: i32 = 3;
pub(in crate::app) const MAIN_UI_LAYOUT: MainUiLayout = MainUiLayout::zsclip();
pub(in crate::app) const CLIPBOARD_IGNORE_MS_PASTE: u64 = 1800;
pub(in crate::app) const CLIPBOARD_IGNORE_MS_DIRECT_EDIT: u64 = 600;
pub(in crate::app) const TRANSIENT_DUPLICATE_CAPTURE_MS: u64 = 3500;
pub(in crate::app) const TRANSIENT_DUPLICATE_QUEUE_MS: u64 = 5000;
pub(in crate::app) const LAN_RECENT_MESSAGE_LIMIT: usize = 512;
pub(in crate::app) const EDGE_AUTO_HIDE_TIMER_MS: u32 = 120;
pub(in crate::app) const EDGE_AUTO_HIDE_DELAY_MS: u64 = 650;
pub(in crate::app) const EDGE_AUTO_HIDE_RESTORE_GRACE_MS: u64 = 450;
pub(in crate::app) const EDGE_AUTO_HIDE_ANIM_MS: u64 = 180;
pub(in crate::app) const EDGE_AUTO_HIDE_ANIM_TIMER_MS: u32 = 16;
pub(in crate::app) const CLIPBOARD_RETRY_DELAY_FAST_MS: u32 = 30;
pub(in crate::app) const CLIPBOARD_RETRY_DELAY_MEDIUM_MS: u32 = 80;
pub(in crate::app) const CLIPBOARD_RETRY_DELAY_MS: u32 = 140;
pub(in crate::app) const CLIPBOARD_RETRY_MAX_ATTEMPTS: u8 = 5;
pub(in crate::app) const PIXPIN_CLIPBOARD_RETRY_MAX_ATTEMPTS: u8 = 18;
pub(in crate::app) const MAX_CAPTURE_PIXELS: usize = 16_000_000;
pub(in crate::app) const MAX_CAPTURE_SIDE: usize = 8192;
pub(in crate::app) const MAX_CLIPBOARD_BITMAP_DECODE_PIXELS: usize = MAX_CAPTURE_PIXELS * 2;

pub(in crate::app) const EN_CHANGE_CODE: u16 = 0x0300;

pub(in crate::app) type AppResult<T> = Result<T, io::Error>;

pub(in crate::app) const EDGE_AUTO_HIDE_PEEK: i32 = 2;
pub(in crate::app) const EDGE_AUTO_HIDE_MARGIN: i32 = 8;

pub(in crate::app) const EDGE_AUTO_HIDE_NONE: i32 = -1;
pub(in crate::app) const EDGE_AUTO_HIDE_LEFT: i32 = 0;
pub(in crate::app) const EDGE_AUTO_HIDE_RIGHT: i32 = 1;
pub(in crate::app) const EDGE_AUTO_HIDE_TOP: i32 = 2;
pub(in crate::app) const ITEMS_PAGE_SIZE: usize = 200;
pub(in crate::app) const ITEMS_LOAD_AHEAD_ROWS: i32 = 18;
pub(in crate::app) const EDGE_AUTO_HIDE_BOTTOM: i32 = 3;
pub(in crate::app) const VV_SHOW_RETRY_DELAY_MS: u32 = 30;
pub(in crate::app) const VV_SHOW_RETRY_MAX: u8 = 10;
pub(in crate::app) const VV_POPUP_MENU_GRACE_MS: u64 = 900;

use windows_sys::Win32::Foundation::RECT;
use crate::ui::{SETTINGS_CONTENT_W, SETTINGS_CONTENT_X, SETTINGS_CONTENT_Y};

pub const SETTINGS_CONTENT_TOTAL_H: i32 = 760;
pub const SCROLL_BAR_W: i32 = 3;
pub const SCROLL_BAR_W_ACTIVE: i32 = 5;
pub const SCROLL_BAR_MARGIN: i32 = 3;

pub const CARD_GENERAL_Y: i32 = 16;
pub const CARD_GENERAL_H: i32 = 240;
pub const CARD_DATA_Y: i32 = 268;
pub const CARD_DATA_H: i32 = 96;
pub const CARD_POSITION_Y: i32 = 376;
pub const CARD_POSITION_H: i32 = 168;
pub const CARD_MAINTAIN_Y: i32 = 556;
pub const CARD_MAINTAIN_H: i32 = 96;

pub const GENERAL_TOGGLE_START_Y: i32 = SETTINGS_CONTENT_Y + 76;
pub const GENERAL_TOGGLE_GAP_Y: i32 = 36;
pub const MAX_ITEMS_Y: i32 = SETTINGS_CONTENT_Y + 326;
pub const POSMODE_Y: i32 = SETTINGS_CONTENT_Y + 434;
pub const MOUSE_DXY_Y: i32 = SETTINGS_CONTENT_Y + 472;
pub const FIXED_XY_Y: i32 = SETTINGS_CONTENT_Y + 508;
pub const MAINTAIN_BTN_Y: i32 = SETTINGS_CONTENT_Y + 600;

#[inline]
pub const fn settings_card_rect(y: i32, h: i32) -> RECT {
    RECT {
        left: SETTINGS_CONTENT_X,
        top: SETTINGS_CONTENT_Y + y,
        right: SETTINGS_CONTENT_X + SETTINGS_CONTENT_W,
        bottom: SETTINGS_CONTENT_Y + y + h,
    }
}

#[inline]
pub fn general_toggle_y(index: i32) -> i32 {
    GENERAL_TOGGLE_START_Y + index * GENERAL_TOGGLE_GAP_Y
}

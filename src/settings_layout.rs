use crate::ui::{SETTINGS_CONTENT_W, SETTINGS_CONTENT_X, SETTINGS_CONTENT_Y};
use crate::ui_core::UiRect;

pub const SETTINGS_CONTENT_TOTAL_H: i32 = 848;
pub const SCROLL_BAR_W: i32 = 3;
pub const SCROLL_BAR_W_ACTIVE: i32 = 5;
pub const SCROLL_BAR_MARGIN: i32 = 3;

pub const CARD_GENERAL_Y: i32 = 16;
pub const CARD_GENERAL_H: i32 = 356;
pub const CARD_DATA_Y: i32 = 384;
pub const CARD_DATA_H: i32 = 96;
pub const CARD_POSITION_Y: i32 = 492;
pub const CARD_POSITION_H: i32 = 168;
pub const CARD_MAINTAIN_Y: i32 = 672;
pub const CARD_MAINTAIN_H: i32 = 96;

#[inline]
pub const fn settings_card_rect(y: i32, h: i32) -> UiRect {
    UiRect::new(
        SETTINGS_CONTENT_X,
        SETTINGS_CONTENT_Y + y,
        SETTINGS_CONTENT_X + SETTINGS_CONTENT_W,
        SETTINGS_CONTENT_Y + y + h,
    )
}

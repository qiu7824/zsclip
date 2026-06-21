use crate::app_core::UiRect;
use windows_sys::Win32::Foundation::RECT;

impl From<RECT> for UiRect {
    fn from(value: RECT) -> Self {
        Self {
            left: value.left,
            top: value.top,
            right: value.right,
            bottom: value.bottom,
        }
    }
}

impl From<&RECT> for UiRect {
    fn from(value: &RECT) -> Self {
        Self::from(*value)
    }
}

impl From<UiRect> for RECT {
    fn from(value: UiRect) -> Self {
        Self {
            left: value.left,
            top: value.top,
            right: value.right,
            bottom: value.bottom,
        }
    }
}

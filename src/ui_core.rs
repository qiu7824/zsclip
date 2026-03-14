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

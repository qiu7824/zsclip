use super::prelude::*;

pub(super) struct SettingsPageBuilder {
    pub(super) hwnd: HWND,
    pub(super) page: usize,
    pub(super) font: *mut core::ffi::c_void,
}

impl SettingsPageBuilder {
    pub(super) unsafe fn add(
        &self,
        st: &mut SettingsWndState,
        hwnd: HWND,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> HWND {
        if !hwnd.is_null() {
            settings_page_push_ctrl(st, self.page, hwnd, x, y, w, h);
        }
        hwnd
    }

    pub(super) fn section(&self, index: usize, label_w: i32) -> SettingsFormSectionLayout {
        SettingsFormSectionLayout::new(self.page, index, label_w)
    }
}

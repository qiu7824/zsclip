use super::prelude::*;

pub(super) struct SettingsPageBuilder {
    pub(super) hwnd: HWND,
    pub(super) page: usize,
    pub(super) font: *mut core::ffi::c_void,
}

#[derive(Clone, Copy)]
pub(super) struct SettingsControlPlacement {
    pub(super) parent: HWND,
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) origin_dx: i32,
    pub(super) origin_dy: i32,
}

impl SettingsPageBuilder {
    pub(super) fn control_placement(
        &self,
        st: &SettingsWndState,
        x: i32,
        y: i32,
    ) -> SettingsControlPlacement {
        if settings_page_control_scrollable(st, self.page) && !st.viewport_hwnd.is_null() {
            if let Some(crc) = settings_window_client_bounds(self.hwnd).map(RECT::from) {
                let viewport = settings_viewport_rect(&crc);
                let scroll_y = if self.page == st.cur_page {
                    st.content_scroll_y
                } else {
                    st.page_scroll_y.get(self.page).copied().unwrap_or(0)
                };
                return SettingsControlPlacement {
                    parent: st.viewport_hwnd,
                    x: x - viewport.left,
                    y: y - viewport.top - scroll_y,
                    origin_dx: viewport.left,
                    origin_dy: viewport.top + scroll_y,
                };
            }
        }
        SettingsControlPlacement {
            parent: self.hwnd,
            x,
            y,
            origin_dx: 0,
            origin_dy: 0,
        }
    }

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

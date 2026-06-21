use super::prelude::*;

impl SettingsPageBuilder {
    pub(super) unsafe fn label(
        &self,
        st: &mut SettingsWndState,
        text: &str,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> HWND {
        self.add(
            st,
            settings_create_label(self.hwnd, text, x, y, w, h, self.font),
            x,
            y,
            w,
            h,
        )
    }

    pub(super) unsafe fn label_auto(
        &self,
        st: &mut SettingsWndState,
        text: &str,
        x: i32,
        y: i32,
        w: i32,
        min_h: i32,
    ) -> (HWND, i32) {
        let (hwnd, h) = settings_create_label_auto(self.hwnd, text, x, y, w, min_h, self.font);
        (self.add(st, hwnd, x, y, w, h), h)
    }

    pub(super) unsafe fn button(
        &self,
        st: &mut SettingsWndState,
        text: &str,
        id: isize,
        x: i32,
        y: i32,
        w: i32,
    ) -> HWND {
        self.add(
            st,
            settings_create_small_btn(self.hwnd, text, id, x, y, w, self.font),
            x,
            y,
            w,
            settings_scale(32),
        )
    }

    pub(super) unsafe fn button_sized(
        &self,
        st: &mut SettingsWndState,
        text: &str,
        id: isize,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> HWND {
        let hwnd = settings_create_small_btn(self.hwnd, text, id, x, y, w, self.font);
        if !hwnd.is_null() {
            platform_window::move_window(hwnd, x, y, w, h, false);
        }
        self.add(st, hwnd, x, y, w, h)
    }

    pub(super) unsafe fn dropdown(
        &self,
        st: &mut SettingsWndState,
        text: &str,
        id: isize,
        x: i32,
        y: i32,
        w: i32,
    ) -> HWND {
        self.add(
            st,
            settings_create_dropdown_btn(self.hwnd, text, id, x, y, w, self.font),
            x,
            y,
            w,
            settings_scale(32),
        )
    }

    pub(super) unsafe fn edit(
        &self,
        st: &mut SettingsWndState,
        text: &str,
        id: isize,
        x: i32,
        y: i32,
        w: i32,
    ) -> HWND {
        self.add(
            st,
            settings_create_edit(self.hwnd, text, id, x, y, w, self.font),
            x,
            y,
            w,
            settings_scale(28),
        )
    }

    pub(super) unsafe fn password_edit(
        &self,
        st: &mut SettingsWndState,
        text: &str,
        id: isize,
        x: i32,
        y: i32,
        w: i32,
    ) -> HWND {
        self.add(
            st,
            settings_create_password_edit(self.hwnd, text, id, x, y, w, self.font),
            x,
            y,
            w,
            settings_scale(28),
        )
    }

    pub(super) unsafe fn listbox(
        &self,
        st: &mut SettingsWndState,
        id: isize,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> HWND {
        self.add(
            st,
            settings_create_listbox(self.hwnd, id, x, y, w, h, self.font),
            x,
            y,
            w,
            h,
        )
    }

    pub(super) unsafe fn toggle_row(
        &self,
        st: &mut SettingsWndState,
        text: &str,
        id: isize,
        x: i32,
        y: i32,
        w: i32,
    ) -> (HWND, HWND) {
        let (label, btn, layout) =
            settings_create_toggle_plain(self.hwnd, text, id, x, y, w, self.font);
        (
            self.add(
                st,
                label,
                layout.label_rect.left,
                layout.label_rect.top,
                layout.label_rect.right - layout.label_rect.left,
                layout.label_rect.bottom - layout.label_rect.top,
            ),
            self.add(
                st,
                btn,
                layout.toggle_rect.left,
                layout.toggle_rect.top,
                layout.toggle_rect.right - layout.toggle_rect.left,
                layout.toggle_rect.bottom - layout.toggle_rect.top,
            ),
        )
    }
}

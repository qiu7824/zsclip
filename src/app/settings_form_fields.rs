use super::prelude::*;

impl SettingsPageBuilder {
    pub(super) unsafe fn form_label(
        &self,
        st: &mut SettingsWndState,
        sec: &SettingsFormSectionLayout,
        row: i32,
        text: &str,
    ) -> HWND {
        let rect = sec.label_rect(row, settings_scale(24));
        self.label(
            st,
            text,
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
        )
    }

    pub(super) unsafe fn form_value_label(
        &self,
        st: &mut SettingsWndState,
        sec: &SettingsFormSectionLayout,
        row: i32,
        text: &str,
    ) -> HWND {
        let rect = sec.field_label_rect(row, settings_scale(24));
        self.label(
            st,
            text,
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
        )
    }

    pub(super) unsafe fn form_value_label_auto(
        &self,
        st: &mut SettingsWndState,
        sec: &SettingsFormSectionLayout,
        row: i32,
        text: &str,
        min_h: i32,
    ) -> (HWND, i32) {
        let rect = sec.field_full_rect(row, min_h);
        self.label_auto(st, text, rect.left, rect.top, rect.right - rect.left, min_h)
    }

    pub(super) unsafe fn form_dropdown(
        &self,
        st: &mut SettingsWndState,
        sec: &SettingsFormSectionLayout,
        row: i32,
        text: &str,
        id: isize,
        w: i32,
    ) -> HWND {
        let rect = sec.field_sized_row_rect(row, w);
        self.dropdown(st, text, id, rect.left, rect.top, rect.right - rect.left)
    }

    pub(super) unsafe fn form_edit(
        &self,
        st: &mut SettingsWndState,
        sec: &SettingsFormSectionLayout,
        row: i32,
        text: &str,
        id: isize,
    ) -> HWND {
        let rect = sec.field_row_rect(row);
        self.edit(st, text, id, rect.left, rect.top, rect.right - rect.left)
    }

    pub(super) unsafe fn form_password_edit(
        &self,
        st: &mut SettingsWndState,
        sec: &SettingsFormSectionLayout,
        row: i32,
        text: &str,
        id: isize,
    ) -> HWND {
        let rect = sec.field_row_rect(row);
        self.password_edit(st, text, id, rect.left, rect.top, rect.right - rect.left)
    }

    pub(super) unsafe fn form_button(
        &self,
        st: &mut SettingsWndState,
        sec: &SettingsFormSectionLayout,
        row: i32,
        text: &str,
        id: isize,
        w: i32,
    ) -> HWND {
        let rect = sec.field_sized_row_rect(row, w);
        self.button(st, text, id, rect.left, rect.top, rect.right - rect.left)
    }
}

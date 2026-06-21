use super::prelude::*;

impl SettingsPageBuilder {
    pub(super) unsafe fn own_button(&self, st: &mut SettingsWndState, hwnd: HWND) -> HWND {
        if !hwnd.is_null() {
            st.ownerdraw_ctrls.push(hwnd);
        }
        hwnd
    }

    pub(super) unsafe fn form_action_row(
        &self,
        st: &mut SettingsWndState,
        sec: &SettingsFormSectionLayout,
        row: i32,
        actions: &[(&str, isize, i32)],
    ) -> Vec<HWND> {
        let widths: Vec<i32> = actions.iter().map(|(_, _, width)| *width).collect();
        let rects = sec.action_row_rects(row, &widths);
        actions
            .iter()
            .zip(rects.iter())
            .map(|((text, id, _), rect)| {
                let hwnd = self.button(st, text, *id, rect.left, rect.top, rect.right - rect.left);
                self.own_button(st, hwnd)
            })
            .collect()
    }

    pub(super) unsafe fn form_qr_action(
        &self,
        st: &mut SettingsWndState,
        sec: &SettingsFormSectionLayout,
        row: i32,
        label: &str,
        qr_id: isize,
        action_text: &str,
        action_id: isize,
    ) -> (HWND, HWND) {
        let layout = sec.qr_action_layout(row);
        self.form_label(st, sec, row, label);
        let qr = self.button_sized(
            st,
            "",
            qr_id,
            layout.qr_rect.left,
            layout.qr_rect.top,
            layout.qr_rect.right - layout.qr_rect.left,
            layout.qr_rect.bottom - layout.qr_rect.top,
        );
        let action = self.button(
            st,
            action_text,
            action_id,
            layout.action_rect.left,
            layout.action_rect.top,
            layout.action_rect.right - layout.action_rect.left,
        );
        (self.own_button(st, qr), self.own_button(st, action))
    }

    pub(super) unsafe fn own_toggle_row(
        &self,
        st: &mut SettingsWndState,
        text: &str,
        id: isize,
        x: i32,
        y: i32,
        w: i32,
    ) -> (HWND, HWND) {
        let (label, btn) = self.toggle_row(st, text, id, x, y, w);
        self.own_button(st, btn);
        (label, btn)
    }
}

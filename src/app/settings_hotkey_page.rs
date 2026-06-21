use super::prelude::*;

pub(super) unsafe fn settings_create_hotkey_page(hwnd: HWND, st: &mut SettingsWndState) {
    let page = SettingsPage::Hotkey.index();
    let b = SettingsPageBuilder {
        hwnd,
        page,
        font: st.ui_font,
    };
    let sec0 = b.section(0, 86);
    let sec1 = b.section(1, 0);
    let sec2 = b.section(2, 0);
    let line_h = settings_scale(24);
    let note_h = settings_scale(40);
    let small_gap = settings_scale(6);

    settings_create_hotkey_shortcut_controls(st, &b, sec0, line_h);
    settings_create_hotkey_system_controls(st, &b, sec1, sec2, line_h, note_h, small_gap);

    st.ui.mark_built(page);
}

use super::prelude::*;

pub(super) unsafe fn settings_create_general_page(hwnd: HWND, st: &mut SettingsWndState) {
    let page = SettingsPage::General.index();
    let b = SettingsPageBuilder {
        hwnd,
        page,
        font: st.ui_font,
    };
    let sec0 = b.section(0, 0);
    let sec1 = b.section(1, 130);
    let sec2 = b.section(2, 0);
    let sec3 = b.section(3, 138);
    let sec4 = b.section(4, 0);

    settings_create_general_startup_behavior_page(st, &b, sec0, sec1, sec2);
    settings_create_general_window_position_page(st, &b, sec2, sec3, sec4);

    st.ui.mark_built(page);
}

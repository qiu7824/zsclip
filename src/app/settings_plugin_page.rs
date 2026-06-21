use super::prelude::*;

pub(super) unsafe fn settings_create_plugin_page(hwnd: HWND, st: &mut SettingsWndState) {
    let page = SettingsPage::Plugin.index();
    let b = SettingsPageBuilder {
        hwnd,
        page,
        font: st.ui_font,
    };
    let sec0 = b.section(0, 110);
    let sec1 = b.section(1, 110);
    let sec2 = b.section(2, 110);
    let sec3 = b.section(3, 0);
    let sec4 = b.section(4, 0);
    let sec5 = b.section(5, 0);
    let sec6 = b.section(6, 0);
    let line_h = settings_scale(24);

    settings_create_plugin_quick_search_page(st, &b, sec0, line_h);
    settings_create_plugin_ocr_translate_page(st, &b, sec1, sec2, line_h);
    settings_create_plugin_tools_page(st, &b, sec3, sec4, sec5, sec6);

    st.ui.mark_built(page);
}

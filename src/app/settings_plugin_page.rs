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
    let notes = b.section(7, 0);
    let (label, toggle) = b.own_toggle_row(st, tr("启用便签", "Enable notes"), IDC_SET_NOTES_ENABLE,
        notes.left(), notes.row_y(0), notes.full_w());
    st.lb_notes_title = label;
    st.chk_notes = toggle;
    st.btn_notes = b.button(st, tr("打开便签", "Open notes"), IDC_SET_NOTES_OPEN,
        notes.left(), notes.row_y(1), settings_scale(170));
    if !st.btn_notes.is_null() { st.ownerdraw_ctrls.push(st.btn_notes); }
    st.lb_notes_hint = b.label(st, tr("本地自动保存；开启并保存后，可在快捷键与 VV 中绑定侧键。",
        "Saved locally. Enable and save, then bind a side button in Hotkeys & VV."),
        notes.left(), notes.row_y(2), notes.full_w(), line_h);

    st.ui.mark_built(page);
}

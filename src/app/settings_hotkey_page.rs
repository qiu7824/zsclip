use super::prelude::*;

pub(super) unsafe fn settings_create_hotkey_page(hwnd: HWND, st: &mut SettingsWndState) {
    let page = SettingsPage::Hotkey.index();
    let b = SettingsPageBuilder {
        hwnd,
        page,
        font: st.ui_font,
    };
    let sec0 = b.section(0, 86);
    let sec1 = b.section(1, 86);
    let sec2 = b.section(2, 0);
    let sec3 = b.section(3, 0);
    let line_h = settings_scale(24);
    let note_h = settings_scale(40);
    let small_gap = settings_scale(6);

    settings_create_hotkey_shortcut_controls(st, &b, sec0, line_h);
    settings_create_mouse_side_button_controls(st, &b, sec1, line_h);
    settings_create_hotkey_system_controls(st, &b, sec2, sec3, line_h, note_h, small_gap);

    let vv = b.section(4, 138);
    settings_page_toggle(&b, st, &vv, 0, "VV 快速粘贴", IDC_SET_VV_MODE);
    let source_label = source_tab_label(st.vv_source_selected);
    let group_label = source_tab_all_label(st.vv_source_selected);
    st.cb_vv_source = settings_page_dropdown(&b, st, &vv, 1, "VV 来源：", source_label, IDC_SET_VV_SOURCE);
    st.cb_vv_group = settings_page_dropdown(&b, st, &vv, 2, "VV 默认分组：", group_label, IDC_SET_VV_GROUP);
    settings_sync_group_page(st);

    st.ui.mark_built(page);
}

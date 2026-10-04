use super::prelude::*;

pub(super) unsafe fn settings_page_toggle(
    b: &SettingsPageBuilder, st: &mut SettingsWndState,
    sec: &SettingsFormSectionLayout, row: i32, text: &str, id: isize,
) -> HWND {
    b.own_toggle_row(st, text, id, sec.left(), sec.row_y(row), sec.full_w()).1
}

pub(super) unsafe fn settings_page_dropdown(
    b: &SettingsPageBuilder, st: &mut SettingsWndState,
    sec: &SettingsFormSectionLayout, row: i32, label: &str, text: &str, id: isize,
) -> HWND {
    b.form_label(st, sec, row, label);
    let control = b.form_dropdown(st, sec, row, text, id, settings_scale(180));
    if !control.is_null() { st.ownerdraw_ctrls.push(control); }
    control
}

pub(super) unsafe fn settings_create_general_page(hwnd: HWND, st: &mut SettingsWndState) {
    let page = SettingsPage::General.index();
    let b = SettingsPageBuilder { hwnd, page, font: st.ui_font };
    let sec = b.section(0, 138);
    st.chk_autostart = settings_page_toggle(&b, st, &sec, 0, "开机自启", IDC_SET_AUTOSTART);
    st.chk_silent_start = settings_page_toggle(&b, st, &sec, 1, "静默启动", IDC_SET_SILENTSTART);
    st.chk_tray_icon = settings_page_toggle(&b, st, &sec, 2, "显示托盘图标", IDC_SET_TRAYICON);
    st.chk_app_icon = settings_page_toggle(&b, st, &sec, 3, "显示软件图标", IDC_SET_APP_ICON_VISIBLE);
    st.chk_close_tray = settings_page_toggle(&b, st, &sec, 4, "关闭窗口后驻留托盘", IDC_SET_CLOSETRAY);
    let maintenance = b.section(1, 138);
    st.btn_open_cfg = b.button(st, "打开设置文件", IDC_SET_BTN_OPENCFG,
        maintenance.left(), maintenance.row_y(0), settings_scale(160));
    if !st.btn_open_cfg.is_null() { st.ownerdraw_ctrls.push(st.btn_open_cfg); }
    st.ui.mark_built(page);
}

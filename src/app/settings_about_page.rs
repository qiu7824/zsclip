use super::prelude::*;

pub(super) unsafe fn settings_create_about_page(hwnd: HWND, st: &mut SettingsWndState) {
    let page = SettingsPage::About.index();
    let b = SettingsPageBuilder {
        hwnd,
        page,
        font: st.ui_font,
    };
    let sec = b.section(0, 96);
    let mut flow = SettingsFlowLayout::new(sec.left(), sec.row_y(0), sec.full_w());

    settings_create_about_metadata_section(st, &b, sec, &mut flow);
    settings_create_about_update_section(st, &b, &mut flow);
    settings_create_about_data_section(st, &b, &mut flow);

    st.ui.mark_built(page);
}

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
    let update = b.section(1, 96);
    let mut update_flow = SettingsFlowLayout::new(update.left(), update.row_y(0), update.full_w());
    settings_create_about_update_section(st, &b, &mut update_flow);
    let data = b.section(2, 96);
    let mut data_flow = SettingsFlowLayout::new(data.left(), data.row_y(0), data.full_w());
    settings_create_about_data_section(st, &b, &mut data_flow);

    st.ui.mark_built(page);
}

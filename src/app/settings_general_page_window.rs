use super::prelude::*;

pub(super) unsafe fn settings_create_window_position_controls(
    st: &mut SettingsWndState, b: &SettingsPageBuilder, sec: SettingsFormSectionLayout,
) {
    let position_label = settings_dropdown_label_for_pos_mode(&st.draft.show_pos_mode);
    st.cb_pos = settings_page_dropdown(b, st, &sec, 0, "弹出位置：", position_label, IDC_SET_POSMODE);
    b.form_label(st, &sec, 1, "鼠标偏移 dx/dy：");
    b.form_label(st, &sec, 2, "固定位置 x/y：");
    let dx = st.draft.show_mouse_dx.to_string();
    let dy = st.draft.show_mouse_dy.to_string();
    let fx = st.draft.show_fixed_x.to_string();
    let fy = st.draft.show_fixed_y.to_string();
    st.ed_dx = b.edit(st, &dx, IDC_SET_DX, sec.field_x(), sec.row_y(1), settings_scale(72));
    st.ed_dy = b.edit(st, &dy, IDC_SET_DY, sec.field_x() + settings_scale(84), sec.row_y(1), settings_scale(72));
    st.ed_fx = b.edit(st, &fx, IDC_SET_FX, sec.field_x(), sec.row_y(2), settings_scale(72));
    st.ed_fy = b.edit(st, &fy, IDC_SET_FY, sec.field_x() + settings_scale(84), sec.row_y(2), settings_scale(72));
}

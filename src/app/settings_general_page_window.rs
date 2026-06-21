use super::prelude::*;

pub(super) unsafe fn settings_create_general_window_position_page(
    st: &mut SettingsWndState,
    b: &SettingsPageBuilder,
    sec2: SettingsFormSectionLayout,
    sec3: SettingsFormSectionLayout,
    sec4: SettingsFormSectionLayout,
) {
    let (_, btn) = b.own_toggle_row(
        st,
        tr("焦点窗口跳过", "Skip focused window"),
        IDC_SET_SKIP_WINDOW_ENABLE,
        sec2.left(),
        sec2.row_y(7),
        sec2.full_w(),
    );
    st.chk_skip_window = btn;
    let skip_label_w = settings_scale(120);
    b.label(
        st,
        tr("跳过窗口类名：", "Skip class names:"),
        sec2.left(),
        sec2.label_y(8, settings_scale(24)),
        skip_label_w,
        settings_scale(24),
    );
    let skip_button_w = settings_scale(96);
    let skip_gap = settings_scale(10);
    let skip_edit_x = sec2.left() + skip_label_w;
    let skip_edit_w =
        (sec2.full_w() - skip_label_w - skip_button_w - skip_gap).max(settings_scale(120));
    st.ed_skip_class_names = b.edit(
        st,
        "",
        IDC_SET_SKIP_WINDOW_CLASSNAMES,
        skip_edit_x,
        sec2.row_y(8),
        skip_edit_w,
    );
    st.btn_capture_skip_window = b.button(
        st,
        tr("捕获当前", "Capture current"),
        IDC_SET_SKIP_WINDOW_CAPTURE,
        skip_edit_x + skip_edit_w + skip_gap,
        sec2.row_y(8),
        skip_button_w,
    );
    if !st.btn_capture_skip_window.is_null() {
        st.ownerdraw_ctrls.push(st.btn_capture_skip_window);
    }

    b.label(
        st,
        "弹出位置：",
        sec3.left(),
        sec3.label_y(0, settings_scale(24)),
        sec3.label_w(),
        settings_scale(24),
    );
    st.cb_pos = b.dropdown(
        st,
        "跟随鼠标",
        IDC_SET_POSMODE,
        sec3.field_x(),
        sec3.row_y(0),
        settings_scale(170),
    );
    if !st.cb_pos.is_null() {
        st.ownerdraw_ctrls.push(st.cb_pos);
    }

    b.label(
        st,
        "鼠标偏移 dx/dy：",
        sec3.left(),
        sec3.label_y(1, settings_scale(24)),
        sec3.label_w(),
        settings_scale(24),
    );
    let mouse_x = sec3.field_x();
    st.ed_dx = b.edit(
        st,
        "",
        IDC_SET_DX,
        mouse_x,
        sec3.row_y(1),
        settings_scale(64),
    );
    st.ed_dy = b.edit(
        st,
        "",
        IDC_SET_DY,
        mouse_x + settings_scale(74),
        sec3.row_y(1),
        settings_scale(64),
    );

    b.label(
        st,
        "固定位置 x/y：",
        sec3.left(),
        sec3.label_y(2, settings_scale(24)),
        sec3.label_w(),
        settings_scale(24),
    );
    let fixed_x = sec3.field_x();
    st.ed_fx = b.edit(
        st,
        "",
        IDC_SET_FX,
        fixed_x,
        sec3.row_y(2),
        settings_scale(64),
    );
    st.ed_fy = b.edit(
        st,
        "",
        IDC_SET_FY,
        fixed_x + settings_scale(74),
        sec3.row_y(2),
        settings_scale(64),
    );

    let btn_y = sec4.row_y(0);
    st.btn_open_cfg = b.button(
        st,
        "打开设置文件",
        IDC_SET_BTN_OPENCFG,
        sec4.action_x(0, settings_scale(130)),
        btn_y,
        settings_scale(130),
    );
    if !st.btn_open_cfg.is_null() {
        st.ownerdraw_ctrls.push(st.btn_open_cfg);
    }
}

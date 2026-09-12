use super::prelude::*;

pub(super) unsafe fn settings_create_hotkey_shortcut_controls(
    st: &mut SettingsWndState,
    b: &SettingsPageBuilder,
    sec0: SettingsFormSectionLayout,
    line_h: i32,
) {
    let (_hk_lbl, hk_btn) = b.own_toggle_row(
        st,
        "启用快捷键",
        6101,
        sec0.left(),
        sec0.row_y(0),
        sec0.full_w(),
    );
    st.chk_hk_enable = hk_btn;

    b.label(
        st,
        "修饰键：",
        sec0.left(),
        sec0.label_y(1, line_h),
        settings_scale(70),
        line_h,
    );
    st.cb_hk_mod = b.dropdown(
        st,
        "Win",
        6102,
        sec0.field_x(),
        sec0.row_y(1),
        settings_scale(170),
    );
    if !st.cb_hk_mod.is_null() {
        st.ownerdraw_ctrls.push(st.cb_hk_mod);
    }
    let key_label_x = sec0.field_x() + settings_scale(186);
    b.label(
        st,
        "按键：",
        key_label_x,
        sec0.label_y(1, line_h),
        settings_scale(50),
        line_h,
    );
    st.cb_hk_key = b.dropdown(
        st,
        "V",
        6103,
        key_label_x + settings_scale(50),
        sec0.row_y(1),
        settings_scale(120),
    );
    if !st.cb_hk_key.is_null() {
        st.ownerdraw_ctrls.push(st.cb_hk_key);
    }
    st.lb_hk_preview = b.label(
        st,
        "当前设置：Win + V",
        sec0.left(),
        sec0.label_y(2, line_h),
        sec0.full_w() - settings_scale(124),
        line_h,
    );
    st.btn_hk_record = b.button(
        st,
        tr("录制热键", "Record Hotkey"),
        IDC_SET_HK_RECORD,
        sec0.left() + sec0.full_w() - settings_scale(110),
        sec0.row_y(2),
        settings_scale(110),
    );
    if !st.btn_hk_record.is_null() {
        st.ownerdraw_ctrls.push(st.btn_hk_record);
    }

    let (_plain_lbl, plain_btn) = b.own_toggle_row(
        st,
        tr("启用纯文本粘贴快捷键", "Enable plain-text paste hotkey"),
        IDC_SET_PLAIN_HK_ENABLE,
        sec0.left(),
        sec0.row_y(3),
        sec0.full_w(),
    );
    st.chk_plain_hk_enable = plain_btn;
    b.label(
        st,
        tr("修饰键：", "Modifiers:"),
        sec0.left(),
        sec0.label_y(4, line_h),
        sec0.label_w(),
        line_h,
    );
    st.cb_plain_hk_mod = b.dropdown(
        st,
        "Ctrl+Shift",
        IDC_SET_PLAIN_HK_MOD,
        sec0.field_x(),
        sec0.row_y(4),
        settings_scale(170),
    );
    if !st.cb_plain_hk_mod.is_null() {
        st.ownerdraw_ctrls.push(st.cb_plain_hk_mod);
    }
    let plain_key_label_x = sec0.field_x() + settings_scale(186);
    b.label(
        st,
        tr("按键：", "Key:"),
        plain_key_label_x,
        sec0.label_y(4, line_h),
        settings_scale(50),
        line_h,
    );
    st.cb_plain_hk_key = b.dropdown(
        st,
        "V",
        IDC_SET_PLAIN_HK_KEY,
        plain_key_label_x + settings_scale(50),
        sec0.row_y(4),
        settings_scale(120),
    );
    if !st.cb_plain_hk_key.is_null() {
        st.ownerdraw_ctrls.push(st.cb_plain_hk_key);
    }
    st.lb_plain_hk_preview = b.label(
        st,
        &hotkey_preview_text("Ctrl+Shift", "V"),
        sec0.left(),
        sec0.label_y(5, line_h),
        sec0.full_w(),
        line_h,
    );
}

pub(super) unsafe fn settings_create_mouse_side_button_controls(
    st: &mut SettingsWndState,
    b: &SettingsPageBuilder,
    section: SettingsFormSectionLayout,
    line_h: i32,
) {
    let (_label, toggle) = b.own_toggle_row(
        st,
        tr("启用鼠标侧键绑定", "Enable mouse side-button bindings"),
        IDC_SET_MOUSE_SIDE_ENABLE,
        section.left(),
        section.row_y(0),
        section.full_w(),
    );
    st.chk_mouse_side_button = toggle;

    b.label(
        st,
        tr("侧键 1：", "Side button 1:"),
        section.left(),
        section.label_y(1, line_h),
        section.label_w(),
        line_h,
    );
    st.cb_mouse_side_button_1 = b.dropdown(
        st,
        mouse_side_button_action_display("quick_window"),
        IDC_SET_MOUSE_SIDE_BUTTON_1,
        section.field_x(),
        section.row_y(1),
        settings_scale(220),
    );
    if !st.cb_mouse_side_button_1.is_null() {
        st.ownerdraw_ctrls.push(st.cb_mouse_side_button_1);
    }

    b.label(
        st,
        tr("侧键 2：", "Side button 2:"),
        section.left(),
        section.label_y(2, line_h),
        section.label_w(),
        line_h,
    );
    st.cb_mouse_side_button_2 = b.dropdown(
        st,
        mouse_side_button_action_display("vv_mode"),
        IDC_SET_MOUSE_SIDE_BUTTON_2,
        section.field_x(),
        section.row_y(2),
        settings_scale(220),
    );
    if !st.cb_mouse_side_button_2.is_null() {
        st.ownerdraw_ctrls.push(st.cb_mouse_side_button_2);
    }
    b.label(
        st,
        tr(
            "绑定 VV 模式后，可独立于常规 VV 开关使用。",
            "A VV Mode binding works independently of the regular VV switch.",
        ),
        section.left(),
        section.label_y(3, line_h),
        section.full_w(),
        line_h,
    );
}

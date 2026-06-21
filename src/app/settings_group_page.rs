use super::prelude::*;

pub(super) unsafe fn settings_create_group_page(hwnd: HWND, st: &mut SettingsWndState) {
    let page = SettingsPage::Group.index();
    let b = SettingsPageBuilder {
        hwnd,
        page,
        font: st.ui_font,
    };
    let sec0 = b.section(0, 104);
    let sec1 = b.section(1, 0);
    let (_, btn) = b.own_toggle_row(
        st,
        "启用分组功能",
        IDC_SET_GROUP_ENABLE,
        sec0.left(),
        sec0.row_y(0),
        sec0.full_w(),
    );
    st.chk_group_enable = btn;

    b.label(
        st,
        tr("VV 来源：", "VV Source:"),
        sec0.left(),
        sec0.label_y(1, settings_scale(24)),
        sec0.label_w(),
        settings_scale(24),
    );
    st.cb_vv_source = b.dropdown(
        st,
        source_tab_label(0),
        IDC_SET_VV_SOURCE,
        sec0.field_x(),
        sec0.row_y(1),
        settings_scale(180),
    );
    if !st.cb_vv_source.is_null() {
        st.ownerdraw_ctrls.push(st.cb_vv_source);
    }

    b.label(
        st,
        tr("VV 默认分组：", "VV Default Group:"),
        sec0.left(),
        sec0.label_y(2, settings_scale(24)),
        sec0.label_w(),
        settings_scale(24),
    );
    st.cb_vv_group = b.dropdown(
        st,
        source_tab_all_label(0),
        IDC_SET_VV_GROUP,
        sec0.field_x(),
        sec0.row_y(2),
        settings_scale(220),
    );
    if !st.cb_vv_group.is_null() {
        st.ownerdraw_ctrls.push(st.cb_vv_group);
    }

    let tab_w = settings_scale(118);
    st.btn_group_view_records = b.button(
        st,
        "复制记录",
        IDC_SET_GROUP_VIEW_RECORDS,
        sec1.left(),
        sec1.row_y(0),
        tab_w,
    );
    st.btn_group_view_phrases = b.button(
        st,
        "常用短语",
        IDC_SET_GROUP_VIEW_PHRASES,
        sec1.left() + tab_w + settings_scale(10),
        sec1.row_y(0),
        tab_w,
    );
    for &hh in &[st.btn_group_view_records, st.btn_group_view_phrases] {
        if !hh.is_null() {
            st.ownerdraw_ctrls.push(hh);
        }
    }

    st.lb_group_current = b.label(
        st,
        "当前分组：全部记录",
        sec1.left(),
        sec1.row_y(1),
        sec1.full_w(),
        settings_scale(24),
    );
    b.label(
        st,
        "分组列表：",
        sec1.left(),
        sec1.row_y(2),
        settings_scale(220),
        settings_scale(22),
    );

    st.lb_groups = b.listbox(
        st,
        IDC_SET_GROUP_LIST,
        sec1.left(),
        sec1.row_y(3),
        sec1.full_w(),
        settings_scale(170),
    );

    let btn_y = sec1.row_y(3) + settings_scale(186);
    let bw = settings_scale(90);
    let gap = settings_scale(10);
    let x0 = sec1.left();
    st.btn_group_add = b.button(st, "新建分组", IDC_SET_GROUP_ADD, x0, btn_y, bw);
    st.btn_group_rename = b.button(
        st,
        "重命名",
        IDC_SET_GROUP_RENAME,
        x0 + (bw + gap),
        btn_y,
        bw,
    );
    st.btn_group_delete = b.button(
        st,
        "删除",
        IDC_SET_GROUP_DELETE,
        x0 + (bw + gap) * 2,
        btn_y,
        bw,
    );
    st.btn_group_up = b.button(st, "上移", IDC_SET_GROUP_UP, x0 + (bw + gap) * 3, btn_y, bw);
    st.btn_group_down = b.button(
        st,
        "下移",
        IDC_SET_GROUP_DOWN,
        x0 + (bw + gap) * 4,
        btn_y,
        bw,
    );
    for &hh in &[
        st.btn_group_add,
        st.btn_group_rename,
        st.btn_group_delete,
        st.btn_group_up,
        st.btn_group_down,
    ] {
        if !hh.is_null() {
            st.ownerdraw_ctrls.push(hh);
        }
    }

    st.ui.mark_built(page);
}

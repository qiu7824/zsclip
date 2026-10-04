use super::prelude::*;

pub(super) unsafe fn settings_create_group_page(hwnd: HWND, st: &mut SettingsWndState) {
    let page = SettingsPage::Group.index();
    let b = SettingsPageBuilder {
        hwnd,
        page,
        font: st.ui_font,
    };
    let sec0 = b.section(0, 138);
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
    let (_, type_btn) = b.own_toggle_row(
        st,
        "文件类型选项",
        IDC_SET_GROUP_TYPE_FILTER,
        sec0.left(),
        sec0.row_y(1),
        sec0.full_w(),
    );
    st.chk_group_type_filter = type_btn;

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
        sec1.left() + (tab_w + settings_scale(10)) * 2,
        sec1.label_y(0, settings_scale(24)),
        sec1.full_w() - (tab_w + settings_scale(10)) * 2,
        settings_scale(24),
    );
    b.label(
        st,
        tr(
            "右键顶部标签切换分组；密码与密钥独立保存，离开后锁定。",
            "Right-click a tab to switch groups. Passwords and keys are stored separately and lock when you leave.",
        ),
        sec1.left(),
        sec1.row_y(1),
        sec1.full_w(),
        settings_scale(24),
    );
    st.lb_groups = b.listbox(
        st,
        IDC_SET_GROUP_LIST,
        sec1.left(),
        sec1.row_y(2),
        sec1.full_w(),
        settings_scale(104),
    );

    let btn_y = sec1.row_y(2) + settings_scale(116);
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

    let protected_y = btn_y + settings_scale(44);
    let handle = b.button(
        st,
        "密码与密钥",
        super::main_secret_vault::OPEN_VAULT as isize,
        x0,
        protected_y,
        settings_scale(168),
    );
    if !handle.is_null() {
        st.ownerdraw_ctrls.push(handle);
    }
    let phrases = b.section(2, 138);
    settings_page_toggle(&b, st, &phrases, 0, "启用独立标题", crate::win_system_params::IDC_SET_PHRASE_TITLES);
    b.label(st, "关闭后显示正文摘要，已保存的标题保留。", phrases.left(), phrases.row_y(1),
        phrases.full_w(), settings_scale(32));
    st.ui.mark_built(page);
}

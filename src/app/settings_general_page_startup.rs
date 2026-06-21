use super::prelude::*;

pub(super) unsafe fn settings_create_general_startup_behavior_page(
    st: &mut SettingsWndState,
    b: &SettingsPageBuilder,
    sec0: SettingsFormSectionLayout,
    sec1: SettingsFormSectionLayout,
    sec2: SettingsFormSectionLayout,
) {
    let (_, btn) = b.own_toggle_row(
        st,
        "开机自启",
        IDC_SET_AUTOSTART,
        sec0.left(),
        sec0.row_y(0),
        sec0.full_w(),
    );
    st.chk_autostart = btn;
    let (_, btn) = b.own_toggle_row(
        st,
        "静默启动（打开默认不显示）",
        IDC_SET_SILENTSTART,
        sec0.left(),
        sec0.row_y(1),
        sec0.full_w(),
    );
    st.chk_silent_start = btn;
    let (_, btn) = b.own_toggle_row(
        st,
        "右下角图标开启/关闭",
        IDC_SET_TRAYICON,
        sec0.left(),
        sec0.row_y(2),
        sec0.full_w(),
    );
    st.chk_tray_icon = btn;
    let (_, btn) = b.own_toggle_row(
        st,
        "关闭不退出（托盘驻留）",
        IDC_SET_CLOSETRAY,
        sec0.left(),
        sec0.row_y(3),
        sec0.full_w(),
    );
    st.chk_close_tray = btn;
    let (_, btn) = b.own_toggle_row(
        st,
        "呼出后点击外部自动隐藏",
        IDC_SET_AUTOHIDE_BLUR,
        sec0.left(),
        sec0.row_y(4),
        sec0.full_w(),
    );
    st.chk_auto_hide_on_blur = btn;
    let (_, btn) = b.own_toggle_row(
        st,
        "贴边自动隐藏",
        IDC_SET_EDGEHIDE,
        sec0.left(),
        sec0.row_y(5),
        sec0.full_w(),
    );
    st.chk_edge_hide = btn;
    let (_, btn) = b.own_toggle_row(
        st,
        "悬停预览",
        IDC_SET_HOVERPREVIEW,
        sec0.left(),
        sec0.row_y(6),
        sec0.full_w(),
    );
    st.chk_hover_preview = btn;
    let _ = b.own_toggle_row(
        st,
        tr("VV 模式", "VV Mode"),
        IDC_SET_VV_MODE,
        sec0.left(),
        sec0.row_y(7),
        sec0.full_w(),
    );
    let _ = b.own_toggle_row(
        st,
        tr("显示图片缩略图", "Show image thumbnails"),
        IDC_SET_IMAGE_PREVIEW,
        sec0.left(),
        sec0.row_y(8),
        sec0.full_w(),
    );
    let _ = b.own_toggle_row(
        st,
        tr("快速删除按钮", "Quick delete button"),
        IDC_SET_QUICK_DELETE,
        sec0.left(),
        sec0.row_y(9),
        sec0.full_w(),
    );

    b.label(
        st,
        "最大保存条数：",
        sec1.left(),
        sec1.label_y(0, settings_scale(24)),
        sec1.label_w(),
        settings_scale(24),
    );
    st.cb_max = b.dropdown(
        st,
        "200",
        IDC_SET_MAX,
        sec1.field_x(),
        sec1.row_y(0),
        settings_scale(150),
    );
    if !st.cb_max.is_null() {
        st.ownerdraw_ctrls.push(st.cb_max);
    }

    let (_, btn) = b.own_toggle_row(
        st,
        "单击后隐藏主窗口",
        IDC_SET_CLICK_HIDE,
        sec2.left(),
        sec2.row_y(0),
        sec2.full_w(),
    );
    st.chk_click_hide = btn;
    let (_, btn) = b.own_toggle_row(
        st,
        "粘贴后上移到首行",
        IDC_SET_PASTE_MOVE_TOP,
        sec2.left(),
        sec2.row_y(1),
        sec2.full_w(),
    );
    st.chk_move_pasted_to_top = btn;
    let _ = b.own_toggle_row(
        st,
        "重复内容过滤并提升到首行",
        IDC_SET_DEDUPE_FILTER,
        sec2.left(),
        sec2.row_y(2),
        sec2.full_w(),
    );
    let (_, btn) = b.own_toggle_row(
        st,
        "常驻搜索框",
        IDC_SET_PERSIST_SEARCH,
        sec2.left(),
        sec2.row_y(3),
        sec2.full_w(),
    );
    st.chk_persistent_search = btn;
    let (_, btn) = b.own_toggle_row(
        st,
        "粘贴成功声音",
        IDC_SET_PASTE_SOUND_ENABLE,
        sec2.left(),
        sec2.row_y(4),
        sec2.full_w(),
    );
    st.chk_paste_sound = btn;
    b.label(
        st,
        "提示音：",
        sec2.left(),
        sec2.label_y(5, settings_scale(24)),
        sec2.label_w(),
        settings_scale(24),
    );
    st.cb_paste_sound = b.dropdown(
        st,
        &paste_sound_display("default"),
        IDC_SET_PASTE_SOUND_KIND,
        sec2.field_x(),
        sec2.row_y(5),
        settings_scale(170),
    );
    if !st.cb_paste_sound.is_null() {
        st.ownerdraw_ctrls.push(st.cb_paste_sound);
    }
    b.label(
        st,
        "声音文件：",
        sec2.left(),
        sec2.label_y(6, settings_scale(24)),
        sec2.label_w(),
        settings_scale(24),
    );
    st.btn_paste_sound_pick = b.button(
        st,
        &paste_sound_file_button_text(""),
        IDC_SET_PASTE_SOUND_PICK,
        sec2.field_x(),
        sec2.row_y(6),
        settings_scale(240),
    );
    if !st.btn_paste_sound_pick.is_null() {
        st.ownerdraw_ctrls.push(st.btn_paste_sound_pick);
    }
}

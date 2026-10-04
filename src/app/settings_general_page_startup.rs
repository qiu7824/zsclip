use super::prelude::*;

pub(super) fn content_font_size_label(size: i32) -> String {
    if size == 0 { tr("默认", "Default").to_string() } else { format!("{} px", size.clamp(12, 20)) }
}

pub(super) unsafe fn draw_settings_appearance_preview(st: &SettingsWndState, dc: HDC, scroll_y: i32, theme: Theme) {
    let sec = SettingsFormSectionLayout::new(SettingsPage::Appearance.index(), 0, 138);
    let top = sec.row_y(5) - scroll_y;
    let row = RECT { left: sec.left(), top, right: sec.left() + sec.full_w(), bottom: top + settings_scale(76) };
    if st.draft.card_view_enabled {
        draw_clipboard_row_card(dc, row, false, false, st.draft.card_border_enabled,
            st.draft.card_shadow_enabled, ((st.ui_dpi.max(96) + 48) / 96) as i32, theme);
    } else {
        draw_round_fill(dc as _, &row, theme.surface2, settings_scale(6));
    }
    let text_rect = RECT { left: row.left + settings_scale(14), top: row.top + settings_scale(8),
        right: row.right - settings_scale(14), bottom: row.bottom - settings_scale(8) };
    draw_text_ex(dc as _, "剪贴板内容预览 · ZSClip 123", &text_rect, theme.text,
        st.draft.content_font_size(), false, false, ui_text_font_family());
}

pub(super) unsafe fn settings_create_appearance_page(hwnd: HWND, st: &mut SettingsWndState) {
    let page = SettingsPage::Appearance.index();
    let b = SettingsPageBuilder { hwnd, page, font: st.ui_font };
    let sec = b.section(0, 138);
    st.chk_dark_mode = settings_page_toggle(&b, st, &sec, 0, "深色模式", IDC_SET_DARK_MODE);
    let label = content_font_size_label(st.draft.content_font_size);
    st.cb_content_font_size = settings_page_dropdown(&b, st, &sec, 1, "内容字号：", &label, IDC_SET_CONTENT_FONT_SIZE);
    settings_page_toggle(&b, st, &sec, 2, "卡片模式", IDC_SET_CARD_VIEW);
    let (border_label, border_button) = b.own_toggle_row(st, "卡片边框", IDC_SET_CARD_BORDER,
        sec.left(), sec.row_y(3), sec.full_w());
    let (shadow_label, shadow_button) = b.own_toggle_row(st, "卡片轻阴影", IDC_SET_CARD_SHADOW,
        sec.left(), sec.row_y(4), sec.full_w());
    st.card_detail_controls = [border_label, border_button, shadow_label, shadow_button];

    let list = b.section(1, 138);
    settings_page_toggle(&b, st, &list, 0, "显示图片缩略图", IDC_SET_IMAGE_PREVIEW);
    st.chk_hover_preview = settings_page_toggle(&b, st, &list, 1, "悬停预览", IDC_SET_HOVERPREVIEW);
    settings_page_toggle(&b, st, &list, 2, "快速删除按钮", IDC_SET_QUICK_DELETE);
    settings_page_toggle(&b, st, &list, 3, "显示图钉按钮", IDC_SET_SHOW_PIN_BUTTON);
    let heights = [st.draft.image_row_height, st.draft.text_row_height, st.draft.file_row_height];
    let labels = ["图片行高：", "文本行高：", "文件行高："];
    let ids = [IDC_SET_IMAGE_ROW_HEIGHT, IDC_SET_TEXT_ROW_HEIGHT, IDC_SET_FILE_ROW_HEIGHT];
    for index in 0..3 {
        st.row_height_edits[index] = settings_page_dropdown(&b, st, &list, index as i32 + 4,
            labels[index], &format!("{} px", heights[index]), ids[index]);
    }
    let behavior = b.section(2, 138);
    st.chk_auto_hide_on_blur = settings_page_toggle(&b, st, &behavior, 0, "点击外部自动隐藏", IDC_SET_AUTOHIDE_BLUR);
    st.chk_edge_hide = settings_page_toggle(&b, st, &behavior, 1, "贴边自动隐藏", IDC_SET_EDGEHIDE);
    st.chk_click_hide = settings_page_toggle(&b, st, &behavior, 2, "粘贴后隐藏主窗口", IDC_SET_CLICK_HIDE);
    st.chk_persistent_search = settings_page_toggle(&b, st, &behavior, 3, "常驻搜索框", IDC_SET_PERSIST_SEARCH);
    let position = b.section(3, 138);
    settings_create_window_position_controls(st, &b, position);
    st.ui.mark_built(page);
}

pub(super) unsafe fn settings_create_clipboard_page(hwnd: HWND, st: &mut SettingsWndState) {
    let page = SettingsPage::Clipboard.index();
    let b = SettingsPageBuilder { hwnd, page, font: st.ui_font };
    let records = b.section(0, 138);
    settings_page_toggle(&b, st, &records, 0, "记录剪贴板内容", crate::win_system_params::IDC_SET_CAPTURE_ENABLE);
    let max_label = settings_dropdown_label_for_max_items(st.draft.max_items);
    st.cb_max = settings_page_dropdown(&b, st, &records, 1, "最大保存条数：", max_label, IDC_SET_MAX);
    settings_page_toggle(&b, st, &records, 2, "保留文本与表格格式", IDC_SET_RICH_TEXT);
    settings_page_toggle(&b, st, &records, 3, "重复内容过滤并提升到首行", IDC_SET_DEDUPE_FILTER);
    let paste = b.section(1, 138);
    st.chk_move_pasted_to_top = settings_page_toggle(&b, st, &paste, 0, "粘贴后上移到首行", IDC_SET_PASTE_MOVE_TOP);
    settings_page_toggle(&b, st, &paste, 1, "右键菜单复制", IDC_SET_CONTEXT_MENU_COPY);
    st.chk_skip_window = settings_page_toggle(&b, st, &paste, 2, "跳过指定粘贴窗口", IDC_SET_SKIP_WINDOW_ENABLE);
    b.form_label(st, &paste, 3, "跳过窗口类名：");
    let gap = settings_scale(10);
    let button_w = settings_scale(96);
    let edit_w = (paste.full_w() - paste.label_w() - button_w - gap).max(settings_scale(120));
    let skip_classes = st.draft.paste_target_skip_class_names.clone();
    st.ed_skip_class_names = b.edit(st, &skip_classes,
        IDC_SET_SKIP_WINDOW_CLASSNAMES, paste.field_x(), paste.row_y(3), edit_w);
    st.btn_capture_skip_window = b.button(st, "捕获当前", IDC_SET_SKIP_WINDOW_CAPTURE,
        paste.field_x() + edit_w + gap, paste.row_y(3), button_w);
    if !st.btn_capture_skip_window.is_null() { st.ownerdraw_ctrls.push(st.btn_capture_skip_window); }
    let sound = b.section(2, 138);
    st.chk_copy_sound = settings_page_toggle(&b, st, &sound, 0, "复制成功声音", IDC_SET_COPY_SOUND_ENABLE);
    st.chk_paste_sound = settings_page_toggle(&b, st, &sound, 1, "粘贴成功声音", IDC_SET_PASTE_SOUND_ENABLE);
    let sound_label = paste_sound_display(&st.draft.paste_success_sound_kind);
    st.cb_paste_sound = settings_page_dropdown(&b, st, &sound, 2, "提示音：", &sound_label, IDC_SET_PASTE_SOUND_KIND);
    let test_button = b.button(st, "试听", crate::win_system_params::IDC_SET_SOUND_TEST,
        sound.field_x() + settings_scale(194), sound.row_y(2), settings_scale(80));
    if !test_button.is_null() { st.ownerdraw_ctrls.push(test_button); }
    b.form_label(st, &sound, 3, "声音文件：");
    let file_label = paste_sound_file_button_text(&st.draft.paste_success_sound_path);
    st.btn_paste_sound_pick = b.form_button(st, &sound, 3, &file_label, IDC_SET_PASTE_SOUND_PICK, settings_scale(240));
    if !st.btn_paste_sound_pick.is_null() { st.ownerdraw_ctrls.push(st.btn_paste_sound_pick); }
    st.lb_sound_status = b.label(st, "复制提示音用于新增记录；粘贴提示音用于 ZSClip 发起的粘贴。",
        sound.left(), sound.row_y(4), sound.full_w(), settings_scale(48));
    st.ui.mark_built(page);
}

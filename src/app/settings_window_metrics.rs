use super::prelude::*;

pub(super) fn settings_page_content_total_h_for_state(st: &SettingsWndState, page: usize) -> i32 {
    settings_page_content_total_h_for_dynamic_sections(
        page,
        &st.plugin_sections,
        &st.multi_sync_sections,
        st.ui.measured_content_total_h(page),
    )
}

pub(super) fn settings_page_max_scroll_for_state(
    st: &SettingsWndState,
    page: usize,
    view_h: i32,
) -> i32 {
    settings_page_max_scroll(settings_page_content_total_h_for_state(st, page), view_h)
}

pub(super) fn settings_scroll_layout_for_state(
    st: &SettingsWndState,
    crc: &RECT,
    bar_w: i32,
) -> SettingsScrollLayout {
    settings_scroll_layout_for_window(
        crc.into(),
        settings_page_content_total_h_for_state(st, st.cur_page),
        SCROLL_BAR_MARGIN,
        bar_w,
    )
}

pub(super) unsafe fn refresh_settings_window_metrics(hwnd: HWND, st: &mut SettingsWndState) {
    let dpi = settings_window_layout_dpi(hwnd).max(96);
    st.ui_dpi = dpi;
    set_settings_ui_dpi(dpi);
    close_settings_dropdown_popup(st);
    let Some(crc) = settings_window_client_bounds(hwnd).map(RECT::from) else {
        return;
    };
    platform_window::send_message(hwnd, WM_SETREDRAW, 0, 0);
    set_settings_viewport_child_visible(st.viewport_hwnd, false);
    if !st.nav_font.is_null() {
        platform_gdi::delete_object(st.nav_font as _);
    }
    if !st.ui_font.is_null() && st.ui_font != platform_gdi::get_stock_object(DEFAULT_GUI_FONT) {
        platform_gdi::delete_object(st.ui_font as _);
    }
    if !st.title_font.is_null() && st.title_font != platform_gdi::get_stock_object(DEFAULT_GUI_FONT)
    {
        platform_gdi::delete_object(st.title_font as _);
    }
    let (nav_font, ui_font, title_font) = create_settings_fonts(hwnd);
    st.nav_font = nav_font;
    st.ui_font = ui_font;
    st.title_font = title_font;

    sync_settings_viewport_child_bounds(hwnd, st.viewport_hwnd);
    let content_y = settings_content_y_scaled();
    let view_h = (crc.bottom - crc.top - content_y).max(0);
    for page in 0..SETTINGS_PAGE_LABELS.len() {
        st.page_scroll_y[page] =
            st.page_scroll_y[page].clamp(0, settings_page_max_scroll_for_state(st, page, view_h));
    }
    let top_margin = settings_scale(24);
    let btn_h = settings_scale(32);
    let save_w = settings_scale(72);
    let close_w = settings_scale(64);
    let gap = settings_scale(20);
    let right = crc.right - top_margin;
    if !st.btn_save.is_null() {
        settings_host_set_bounds(
            st.btn_save,
            UiRect::new(right - save_w, top_margin, right, top_margin + btn_h),
        );
        settings_set_font(st.btn_save, st.ui_font);
    }
    if !st.btn_close.is_null() {
        settings_host_set_bounds(
            st.btn_close,
            UiRect::new(
                right - save_w - gap - close_w,
                top_margin,
                right - save_w - gap,
                top_margin + btn_h,
            ),
        );
        settings_set_font(st.btn_close, st.ui_font);
    }
    for page in 0..SETTINGS_PAGE_LABELS.len() {
        for reg in st.ui.page_regs(page) {
            settings_set_font(reg.hwnd, st.ui_font);
        }
    }
    let built_pages: Vec<usize> = (0..SETTINGS_PAGE_LABELS.len())
        .filter(|&page| st.ui.is_built(page))
        .collect();
    for &page in &built_pages {
        st.ui.clear_page(page);
    }
    st.ownerdraw_ctrls
        .retain(|&ctrl| settings_host_exists(ctrl));
    if !st.hot_ownerdraw.is_null() && !settings_host_exists(st.hot_ownerdraw) {
        st.hot_ownerdraw = null_mut();
    }
    let current_page = st
        .cur_page
        .min(SETTINGS_PAGE_LABELS.len().saturating_sub(1));
    for &page in &built_pages {
        settings_ensure_page(hwnd, st, page);
        settings_sync_page_state(st, page);
    }
    if !st.ui.is_built(current_page) {
        settings_ensure_page(hwnd, st, current_page);
        settings_sync_page_state(st, current_page);
    }
    st.content_scroll_y = if settings_page_control_scrollable(st, current_page) {
        st.page_scroll_y[current_page]
    } else {
        0
    };
    for page in 0..SETTINGS_PAGE_LABELS.len() {
        for reg in st.ui.page_regs(page) {
            if !reg.hwnd.is_null() {
                settings_host_set_visible(reg.hwnd, page == current_page);
            }
        }
    }
    if settings_page_control_scrollable(st, current_page) {
        settings_repos_controls(hwnd, st, true);
    }
    set_settings_viewport_child_visible(st.viewport_hwnd, true);
    platform_window::send_message(hwnd, WM_SETREDRAW, 1, 0);
    repaint_settings_window(hwnd, true);
}

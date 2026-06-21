use super::prelude::*;

pub(super) unsafe fn paint_settings_window(hwnd: HWND) {
    let st_ptr = platform_window::user_data(hwnd) as *mut SettingsWndState;
    let mut ps: PAINTSTRUCT = zeroed();
    let hdc = platform_gdi::begin_paint(hwnd, &mut ps);
    if hdc.is_null() {
        return;
    }
    let paint_dpi = settings_window_layout_dpi(hwnd);
    set_settings_ui_dpi(paint_dpi);
    crate::win_system_ui::set_paint_dpi_override(paint_dpi);
    let theme = Theme::default();
    let rc = platform_window::client_rect(hwnd).unwrap_or_else(|| zeroed());
    let paint_target = begin_buffered_paint(hdc, &rc);
    let memdc = paint_target.map_or(hdc, |(_, target)| target);

    let bg = platform_gdi::create_solid_brush(theme.bg);
    platform_gdi::fill_rect(memdc, &rc, bg);
    platform_gdi::delete_object(bg as _);

    let cur_page = if st_ptr.is_null() {
        0
    } else {
        (*st_ptr).cur_page.min(SETTINGS_PAGE_LABELS.len() - 1)
    };
    let scroll_y = if st_ptr.is_null() {
        0
    } else {
        (*st_ptr).content_scroll_y
    };
    let chrome_plan = settings_chrome_render_plan(rc.into());
    draw_settings_chrome(
        memdc as _,
        &chrome_plan,
        SETTINGS_PAGE_LABELS[cur_page],
        theme,
    );
    let hover_page = if !st_ptr.is_null() && (*st_ptr).nav_hot >= 0 {
        Some((*st_ptr).nav_hot as usize)
    } else {
        None
    };
    let nav_plan = settings_nav_render_plan(cur_page, hover_page, update_check_available());
    for item in &nav_plan.items {
        draw_settings_nav_item(memdc as _, item, theme);
    }

    let content_clip: RECT = chrome_plan.content_clip_rect.into();
    platform_gdi::save_dc(memdc);
    platform_gdi::intersect_clip_rect(
        memdc,
        content_clip.left,
        content_clip.top,
        content_clip.right,
        content_clip.bottom,
    );
    let content_plan = if st_ptr.is_null() {
        settings_content_render_plan(cur_page, scroll_y, &[], &[])
    } else {
        settings_content_render_plan(
            cur_page,
            scroll_y,
            &(*st_ptr).plugin_sections,
            &(*st_ptr).multi_sync_sections,
        )
    };
    draw_settings_content(memdc as _, &content_plan, theme);
    platform_gdi::restore_dc(memdc, -1);
    draw_settings_viewport_mask(memdc as _, &chrome_plan, theme);

    if !st_ptr.is_null() {
        let scroll_plan = settings_scrollbar_render_plan(
            rc.into(),
            settings_page_content_total_h_for_state(&*st_ptr, cur_page),
            scroll_y,
            (*st_ptr).scroll_bar_visible,
            (*st_ptr).scroll_dragging,
            SCROLL_BAR_MARGIN,
            SCROLL_BAR_W,
            SCROLL_BAR_W_ACTIVE,
        );
        if let Some(plan) = scroll_plan {
            draw_settings_scrollbar(memdc as _, &plan, theme);
        }
    }

    if let Some((paint_buf, _)) = paint_target {
        end_buffered_paint(paint_buf, true);
    }
    crate::win_system_ui::clear_paint_dpi_override();
    platform_gdi::end_paint(hwnd, &ps);
}

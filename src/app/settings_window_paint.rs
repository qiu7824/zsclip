use super::prelude::*;
use windows_sys::Win32::Graphics::Gdi::{HBITMAP, HGDIOBJ};

const WM_PRINT_MSG: u32 = 0x0317;
const PRF_NONCLIENT_FLAG: isize = 0x0002;
const PRF_CLIENT_FLAG: isize = 0x0004;
const PRF_ERASEBKGND_FLAG: isize = 0x0008;
const BM_GETSTATE_MSG: u32 = 0x00F2;
const BST_PUSHED_STATE: isize = 0x0004;
const BS_TYPEMASK_STYLE: u32 = 0x000F;
const BS_OWNERDRAW_STYLE: u32 = 0x000B;
const GWLP_ID_INDEX: i32 = -12;
const ODT_BUTTON_TYPE: u32 = 4;
const ODA_DRAWENTIRE_ACTION: u32 = 0x0001;
const ODS_SELECTED_STATE: u32 = 0x0001;
const ODS_DISABLED_STATE: u32 = 0x0004;

fn is_owner_draw_button(control: HWND) -> bool {
    platform_window::class_name(control).eq_ignore_ascii_case("Button")
        && platform_window::window_style(control) & BS_TYPEMASK_STYLE == BS_OWNERDRAW_STYLE
}

/// Owner-drawn buttons ignore WM_PRINT while disabled, so render them through
/// the same WM_DRAWITEM path the settings window uses for live painting.
unsafe fn draw_owner_draw_button_into(hwnd: HWND, control: HWND, dc: HDC, width: i32, height: i32) {
    let mut state = 0;
    if !platform_window::is_enabled(control) {
        state |= ODS_DISABLED_STATE;
    }
    if platform_window::send_message(control, BM_GETSTATE_MSG, 0, 0) & BST_PUSHED_STATE != 0 {
        state |= ODS_SELECTED_STATE;
    }
    let dis = DRAWITEMSTRUCT {
        CtlType: ODT_BUTTON_TYPE,
        CtlID: platform_window::get_window_long_ptr(control, GWLP_ID_INDEX) as u32,
        itemID: 0,
        itemAction: ODA_DRAWENTIRE_ACTION,
        itemState: state,
        hwndItem: control,
        hDC: dc,
        rcItem: RECT {
            left: 0,
            top: 0,
            right: width,
            bottom: height,
        },
        itemData: 0,
    };
    platform_window::send_message(
        hwnd,
        WM_DRAWITEM,
        dis.CtlID as usize,
        &dis as *const DRAWITEMSTRUCT as isize,
    );
}

/// Off-screen surface for one WM_PAINT: the frame (cards plus child controls)
/// is composed here and reaches the screen in a single blit, so scrolling never
/// shows the blank background or cards without their controls.
struct SettingsBackBuffer {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
}

impl SettingsBackBuffer {
    unsafe fn new(target: HDC, width: i32, height: i32) -> Option<Self> {
        let dc = platform_gdi::create_compatible_dc(target);
        if dc.is_null() {
            return None;
        }
        let bitmap = platform_gdi::create_compatible_bitmap(target, width.max(1), height.max(1));
        if bitmap.is_null() {
            platform_gdi::delete_dc(dc);
            return None;
        }
        let previous = platform_gdi::select_object(dc, bitmap as _);
        Some(Self {
            dc,
            bitmap,
            previous,
        })
    }
}

impl Drop for SettingsBackBuffer {
    fn drop(&mut self) {
        platform_gdi::select_object(self.dc, self.previous);
        platform_gdi::delete_object(self.bitmap as _);
        platform_gdi::delete_dc(self.dc);
    }
}

fn intersect_rects(a: &RECT, b: &RECT) -> Option<RECT> {
    let rect = RECT {
        left: a.left.max(b.left),
        top: a.top.max(b.top),
        right: a.right.min(b.right),
        bottom: a.bottom.min(b.bottom),
    };
    (rect.right > rect.left && rect.bottom > rect.top).then_some(rect)
}

unsafe fn print_settings_control(hwnd: HWND, control: HWND, dc: HDC, clip: &RECT) {
    if !platform_window::is_visible(control) {
        return;
    }
    let Some(bounds) = platform_window::window_rect(control) else {
        return;
    };
    let mut origin = POINT {
        x: bounds.left,
        y: bounds.top,
    };
    if !platform_window::screen_to_client(hwnd, &mut origin) {
        return;
    }
    let rect = RECT {
        left: origin.x,
        top: origin.y,
        right: origin.x + (bounds.right - bounds.left),
        bottom: origin.y + (bounds.bottom - bounds.top),
    };
    let Some(visible) = intersect_rects(&rect, clip) else {
        return;
    };
    let saved = platform_gdi::save_dc(dc);
    platform_gdi::intersect_clip_rect(dc, visible.left, visible.top, visible.right, visible.bottom);
    platform_gdi::offset_viewport_org(dc, origin.x, origin.y);
    if is_owner_draw_button(control) {
        draw_owner_draw_button_into(
            hwnd,
            control,
            dc,
            rect.right - rect.left,
            rect.bottom - rect.top,
        );
    } else {
        platform_window::send_message(
            control,
            WM_PRINT_MSG,
            dc as usize,
            PRF_NONCLIENT_FLAG | PRF_CLIENT_FLAG | PRF_ERASEBKGND_FLAG,
        );
    }
    platform_gdi::restore_dc(dc, saved);
}

/// Child controls repaint themselves after the parent; drawing them into the
/// parent's frame first keeps every presented frame complete.
unsafe fn print_settings_child_controls(
    hwnd: HWND,
    dc: HDC,
    paint_rc: &RECT,
    viewport_hwnd: HWND,
    content_clip: &RECT,
) {
    let scroll_clip = intersect_rects(paint_rc, content_clip);
    for child in platform_window::children_bottom_to_top(hwnd) {
        if child == viewport_hwnd {
            let Some(scroll_clip) = scroll_clip else {
                continue;
            };
            if !platform_window::is_visible(child) {
                continue;
            }
            for control in platform_window::children_bottom_to_top(child) {
                print_settings_control(hwnd, control, dc, &scroll_clip);
            }
        } else {
            print_settings_control(hwnd, child, dc, paint_rc);
        }
    }
}

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
    let paint_rc = if ps.rcPaint.right > ps.rcPaint.left && ps.rcPaint.bottom > ps.rcPaint.top {
        ps.rcPaint
    } else {
        rc
    };
    let back_buffer = SettingsBackBuffer::new(hdc, rc.right - rc.left, rc.bottom - rc.top);
    let memdc = back_buffer.as_ref().map_or(hdc, |buffer| buffer.dc);

    let bg = platform_gdi::create_solid_brush(theme.bg);
    platform_gdi::fill_rect(memdc, &paint_rc, bg);
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
    let viewport_clip = settings_viewport_rect(&rc);
    let chrome_dirty =
        paint_rc.left < viewport_clip.left || paint_rc.top < settings_content_y_scaled();
    if chrome_dirty {
        draw_settings_chrome(
            memdc as _,
            &chrome_plan,
            SETTINGS_PAGE_LABELS[cur_page],
            theme,
        );
    }
    let hover_page = if !st_ptr.is_null() && (*st_ptr).nav_hot >= 0 {
        Some((*st_ptr).nav_hot as usize)
    } else {
        None
    };
    if chrome_dirty {
        let nav_plan = settings_nav_render_plan(cur_page, hover_page, update_check_available());
        for item in &nav_plan.items {
            draw_settings_nav_item(memdc as _, item, theme);
        }
    }

    let content_clip: RECT = chrome_plan.content_clip_rect.into();
    platform_gdi::save_dc(memdc);
    platform_gdi::intersect_clip_rect(
        memdc,
        viewport_clip.left,
        viewport_clip.top,
        viewport_clip.right,
        viewport_clip.bottom,
    );
    platform_gdi::intersect_clip_rect(
        memdc,
        content_clip.left,
        content_clip.top,
        content_clip.right,
        content_clip.bottom,
    );
    let mut content_plan = if st_ptr.is_null() {
        settings_content_render_plan(cur_page, scroll_y, &[], &[])
    } else {
        settings_content_render_plan(
            cur_page,
            scroll_y,
            &(*st_ptr).plugin_sections,
            &(*st_ptr).multi_sync_sections,
        )
    };
    content_plan.sections.retain(|section| {
        let top = section.rect.top - content_plan.scroll_y;
        let bottom = section.rect.bottom - content_plan.scroll_y;
        bottom > viewport_clip.top.max(paint_rc.top)
            && top < viewport_clip.bottom.min(paint_rc.bottom)
    });
    draw_settings_content(memdc as _, &content_plan, theme);
    if !st_ptr.is_null() && cur_page == SettingsPage::Appearance.index() {
        draw_settings_appearance_preview(&*st_ptr, memdc, scroll_y, theme);
    }
    if !st_ptr.is_null() {
        draw_settings_lan_qr_blocks(&mut *st_ptr, memdc as _, scroll_y, viewport_clip);
    }
    platform_gdi::restore_dc(memdc, -1);
    if back_buffer.is_some() && !st_ptr.is_null() {
        print_settings_child_controls(
            hwnd,
            memdc,
            &paint_rc,
            (*st_ptr).viewport_hwnd,
            &content_clip,
        );
    }
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

    if back_buffer.is_some() {
        platform_gdi::copy_bits(
            hdc,
            paint_rc.left,
            paint_rc.top,
            paint_rc.right - paint_rc.left,
            paint_rc.bottom - paint_rc.top,
            memdc,
            paint_rc.left,
            paint_rc.top,
        );
    }
    drop(back_buffer);
    crate::win_system_ui::clear_paint_dpi_override();
    platform_gdi::end_paint(hwnd, &ps);
}

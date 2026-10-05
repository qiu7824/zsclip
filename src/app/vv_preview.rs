use super::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};

const CLASS: &str = "ZsClipVvTextPreview";
const READY: u32 = WM_APP + 1;
const CHECKED: u32 = WM_APP + 2;
static WINDOW: OnceLock<isize> = OnceLock::new();
static REQUEST: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Ticket {
    request: u64,
    session: u64,
    item: i64,
    generation: u64,
}
struct Ready {
    ticket: Ticket,
    text: Option<String>,
    protection: Option<String>,
}
struct Checked {
    ticket: Ticket,
    available: bool,
}
struct Preview {
    main: HWND,
    ticket: Option<Ticket>,
    protection: String,
    text: String,
    wide: Vec<u16>,
    font_size: i32,
    scroll: i32,
    content_height: i32,
    checking: bool,
}

fn preview_window() -> HWND {
    WINDOW.get().copied().unwrap_or(0) as HWND
}

unsafe fn data(hwnd: HWND) -> *mut Preview {
    platform_window::user_data(hwnd) as *mut Preview
}

pub(super) unsafe fn vv_preview_contains(point: POINT) -> bool {
    platform_window::is_visible(preview_window())
        && platform_window::window_rect(preview_window())
            .is_some_and(|rect| platform_window::point_in_rect_screen(&point, &rect))
}

pub(super) unsafe fn hide_vv_preview() {
    REQUEST.fetch_add(1, Ordering::SeqCst);
    let hwnd = preview_window();
    if !platform_window::exists(hwnd) {
        return;
    }
    platform_window::hide(hwnd);
    let ptr = data(hwnd);
    if !ptr.is_null() {
        (*ptr).ticket = None;
        (*ptr).text.clear();
        (*ptr).text.shrink_to_fit();
        (*ptr).wide.clear();
        (*ptr).wide.shrink_to_fit();
        (*ptr).protection.clear();
        (*ptr).checking = false;
    }
}

pub(super) unsafe fn destroy_vv_preview() {
    hide_vv_preview();
    let hwnd = preview_window();
    if platform_window::exists(hwnd) {
        platform_window::destroy(hwnd);
    }
}

unsafe fn ensure_window(main: HWND) -> HWND {
    *WINDOW.get_or_init(|| {
        let mut host = WindowsTransientWindowHost::new(CLASS, Some(preview_proc));
        match host.create_transient_window(NativeTransientWindowRequest {
            owner: main,
            bounds: UiRect::new(0, 0, 440, 340),
        }) {
            NativeTransientWindowPresentation::Created(hwnd) => hwnd as isize,
            NativeTransientWindowPresentation::Failed => 0,
        }
    }) as HWND
}

fn ticket_current(ticket: Ticket, session: u64, item: i64, generation: u64, request: u64) -> bool {
    ticket.session == session
        && ticket.item == item
        && ticket.generation == generation
        && ticket.request == request
}

#[cfg(test)]
pub(super) unsafe fn post_stale_vv_preview_for_reclaim_test(main: HWND) -> bool {
    let hwnd = ensure_window(main);
    post_boxed_message(hwnd as isize, READY, 0, Box::new(Ready {
        ticket: Ticket { request: 0, session: 0, item: 91001, generation: 0 },
        text: Some("synthetic late VV preview\n".repeat(8192)),
        protection: None,
    }))
}

fn preview_request_reusable(ticket: Option<Ticket>, visible: bool, session: u64, item: i64, generation: u64, request: u64) -> bool {
    visible && ticket.is_some_and(|ticket| ticket_current(ticket, session, item, generation, request))
}

unsafe fn current(main: HWND, ticket: Ticket) -> bool {
    let ptr = get_state_ptr(main);
    if ptr.is_null() {
        return false;
    }
    let state = &*ptr;
    let item = state
        .vv_popup_preview_index
        .and_then(|i| state.vv_popup_items.get(i))
        .map(|e| e.item.id)
        .unwrap_or(0);
    state.vv_popup_visible
        && vv_session_target_current(state, false)
        && ticket_current(
            ticket,
            state.vv_popup_session_id,
            item,
            crate::db_runtime::current_app_data_generation(),
            REQUEST.load(Ordering::SeqCst),
        )
}

pub(super) unsafe fn request_vv_preview(main: HWND, state: &mut AppState, index: usize) {
    let Some(entry) = state.vv_popup_items.get(index) else {
        return;
    };
    state.vv_popup_preview_index = Some(index);
    if !matches!(
        entry.item.kind,
        ClipKind::Text | ClipKind::Phrase | ClipKind::Files
    ) {
        hide_vv_preview();
        return;
    }
    if !vv_session_target_current(state, false) {
        hide_vv_preview();
        return;
    }
    let id = entry.item.id;
    let hwnd = ensure_window(main);
    if hwnd.is_null() {
        return;
    }
    let ptr = data(hwnd);
    if ptr.is_null() {
        return;
    }
    if (*ptr).main == main && preview_request_reusable((*ptr).ticket,
        platform_window::is_visible(hwnd), state.vv_popup_session_id, id,
        state.app_data_generation, REQUEST.load(Ordering::SeqCst))
        && state.app_data_generation == crate::db_runtime::current_app_data_generation()
        && ((&(*ptr).protection).is_empty() || state.vv_popup_protection_revision.as_ref() == Some(&(*ptr).protection))
    {
        // Re-entering the same row must not clear its body, reset its scroll,
        // or replace an in-flight request with an identical one.
        if (*ptr).font_size != state.settings.content_font_size() {
            (*ptr).font_size = state.settings.content_font_size();
            position(hwnd);
            platform_gdi::invalidate_rect(hwnd, null(), 0);
        }
        return;
    }
    let ticket = Ticket {
        request: REQUEST.fetch_add(1, Ordering::SeqCst) + 1,
        session: state.vv_popup_session_id,
        item: id,
        generation: state.app_data_generation,
    };
    (*ptr).ticket = Some(ticket);
    (*ptr).text.clear();
    (*ptr).wide.clear();
    (*ptr).protection.clear();
    (*ptr).scroll = 0;
    (*ptr).content_height = 0;
    (*ptr).checking = false;
    (*ptr).font_size = state.settings.content_font_size();
    position(hwnd);
    platform_gdi::invalidate_rect(hwnd, null(), 1);
    let raw = hwnd as isize;
    if !crate::image_preview_jobs::submit(move || {
        if REQUEST.load(Ordering::SeqCst) != ticket.request {
            return;
        }
        let protection = crate::db_runtime::search_protection_revision().ok();
        let text = crate::db_runtime::with_shared_app_data_generation(ticket.generation, || {
            db_load_item_full(ticket.item).and_then(|item| match item.kind {
                ClipKind::Text | ClipKind::Phrase => item.text,
                ClipKind::Files => item.file_paths.map(|paths| paths.join("\n")),
                _ => None,
            })
        })
        .flatten();
        unsafe {
            let _ = post_boxed_message(
                raw,
                READY,
                0,
                Box::new(Ready {
                    ticket,
                    text,
                    protection,
                }),
            );
        }
    }) {
        hide_vv_preview();
    }
}

unsafe fn position(hwnd: HWND) {
    let Some(anchor) = platform_window::window_rect(current_vv_popup_hwnd()) else {
        return;
    };
    let wa = platform_monitor::nearest_work_rect_for_window(current_vv_popup_hwnd());
    let dpi = platform_dpi::layout_dpi_for_window(current_vv_popup_hwnd()).max(96) as i32;
    let w = (440 * dpi / 96).min((wa.right - wa.left - 24).max(120));
    let h = (340 * dpi / 96).min((wa.bottom - wa.top - 24).max(100));
    let x = if anchor.right + w + 8 <= wa.right {
        anchor.right + 8
    } else {
        (anchor.left - w - 8).max(wa.left + 8)
    };
    let y = anchor
        .top
        .clamp(wa.top + 8, (wa.bottom - h - 8).max(wa.top + 8));
    WindowsTransientWindowHost::new(CLASS, Some(preview_proc))
        .present_transient_window(hwnd, UiRect::new(x, y, x + w, y + h));
}

pub(super) unsafe fn scroll_vv_preview(pages: i32) {
    let hwnd = preview_window();
    let ptr = data(hwnd);
    if ptr.is_null() || !platform_window::is_visible(hwnd) {
        return;
    }
    let height = platform_window::client_rect(hwnd)
        .map(|r| r.bottom - r.top - 64)
        .unwrap_or(240)
        .max(1);
    (*ptr).scroll =
        ((*ptr).scroll + pages * height).clamp(0, ((*ptr).content_height - height).max(0));
    platform_gdi::invalidate_rect(hwnd, null(), 1);
}

pub(super) unsafe fn validate_vv_preview(state: &AppState) {
    let hwnd = preview_window();
    let ptr = data(hwnd);
    if ptr.is_null() {
        return;
    }
    let Some(ticket) = (*ptr).ticket else {
        return;
    };
    if !current((*ptr).main, ticket) {
        hide_vv_preview();
        return;
    }
    if !(&(*ptr).protection).is_empty()
        && crate::db_runtime::search_protection_revision()
            .ok()
            .as_ref()
            != Some(&(*ptr).protection)
    {
        hide_vv_preview();
        return;
    }
    if (*ptr).font_size != state.settings.content_font_size() {
        (*ptr).font_size = state.settings.content_font_size();
        position(hwnd);
        platform_gdi::invalidate_rect(hwnd, null(), 1);
    }
    if (*ptr).checking || (&(*ptr).protection).is_empty() {
        return;
    }
    (*ptr).checking = true;
    let raw = hwnd as isize;
    if !crate::image_preview_jobs::submit(move || {
        let available=with_db(|conn| conn.query_row("SELECT EXISTS(SELECT 1 FROM items WHERE id=?1 AND (kind NOT IN ('text','phrase') OR (NOT zsclip_is_protected(COALESCE(NULLIF(text_data,''),preview,'')) AND NOT zsclip_is_protected_html(rich_text_html))))",rusqlite::params![ticket.item],|row|row.get::<_,bool>(0))).unwrap_or(false);
        unsafe {
            let _ = post_boxed_message(raw, CHECKED, 0, Box::new(Checked { ticket, available }));
        }
    }) {
        (*ptr).checking = false;
    }
}

unsafe extern "system" fn preview_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCCREATE => {
            let cs = &*(lparam as *const CREATESTRUCTW);
            let ptr = Box::into_raw(Box::new(Preview {
                main: cs.lpCreateParams as HWND,
                ticket: None,
                protection: String::new(),
                text: String::new(),
                wide: Vec::new(),
                font_size: 12,
                scroll: 0,
                content_height: 0,
                checking: false,
            }));
            platform_window::set_user_data(hwnd, ptr as isize);
            platform_appearance::set_rounded_corners(hwnd);
            1
        }
        WM_MOUSEACTIVATE => MA_NOACTIVATE as LRESULT,
        WM_NCHITTEST => HTCLIENT as LRESULT,
        WM_LBUTTONDOWN | WM_LBUTTONUP => 0,
        WM_MOUSEWHEEL => {
            let ptr = data(hwnd);
            if !ptr.is_null() {
                let delta = ((wparam >> 16) as u16 as i16) as i32;
                let height = platform_window::client_rect(hwnd)
                    .map(|r| r.bottom - 64)
                    .unwrap_or(240)
                    .max(1);
                (*ptr).scroll = ((*ptr).scroll - delta * (*ptr).font_size / 40)
                    .clamp(0, ((*ptr).content_height - height).max(0));
                platform_gdi::invalidate_rect(hwnd, null(), 1);
            }
            0
        }
        READY => {
            let mut result = Box::from_raw(lparam as *mut Ready);
            let ptr = data(hwnd);
            if !ptr.is_null()
                && (*ptr).ticket == Some(result.ticket)
                && current((*ptr).main, result.ticket)
            {
                if result.protection.is_none()
                    || result.protection != crate::db_runtime::search_protection_revision().ok()
                    || result.text.is_none()
                {
                    hide_vv_preview();
                } else {
                    (*ptr).text = result.text.take().unwrap();
                    (*ptr).wide = (*ptr).text.encode_utf16().collect();
                    (*ptr).protection = result.protection.take().unwrap();
                    platform_gdi::invalidate_rect(hwnd, null(), 1);
                }
            }
            drop(result);
            schedule_hidden_memory_reclaim_after_activity();
            0
        }
        CHECKED => {
            let result = Box::from_raw(lparam as *mut Checked);
            let ptr = data(hwnd);
            if !ptr.is_null() && (*ptr).ticket == Some(result.ticket) {
                (*ptr).checking = false;
                if !result.available {
                    hide_vv_preview();
                }
            }
            drop(result);
            schedule_hidden_memory_reclaim_after_activity();
            0
        }
        WM_PAINT => {
            let ptr = data(hwnd);
            let mut ps: PAINTSTRUCT = zeroed();
            let hdc = platform_gdi::begin_paint(hwnd, &mut ps);
            if !ptr.is_null() && !hdc.is_null() {
                let p = &mut *ptr;
                let th = Theme::default();
                let rc = platform_window::client_rect(hwnd).unwrap_or_else(|| zeroed());
                let bg = platform_gdi::create_solid_brush(th.surface);
                platform_gdi::fill_rect(hdc, &rc, bg);
                platform_gdi::delete_object(bg as _);
                let valid = p.ticket.is_some_and(|t| current(p.main, t))
                    && (p.protection.is_empty()
                        || crate::db_runtime::search_protection_revision()
                            .ok()
                            .as_ref()
                            == Some(&p.protection));
                if valid {
                    let dpi = platform_dpi::layout_dpi_for_window(hwnd).max(96) as i32;
                    let pad = 14 * dpi / 96;
                    let top = 44 * dpi / 96;
                    let header = RECT {
                        left: pad,
                        top: pad / 2,
                        right: rc.right - pad,
                        bottom: top - pad / 2,
                    };
                    draw_text_ex_px(
                        hdc as _,
                        tr(
                            "全文预览 · 滚轮 / PgUp / PgDn",
                            "Full text · Wheel / PgUp / PgDn",
                        ),
                        &header,
                        th.text_muted,
                        12 * dpi / 96,
                        false,
                        false,
                        ui_text_font_family(),
                    );
                    if p.protection.is_empty() {
                        draw_text_ex_px(
                            hdc as _,
                            tr("正在加载…", "Loading..."),
                            &RECT {
                                left: pad,
                                top,
                                right: rc.right - pad,
                                bottom: top + 32 * dpi / 96,
                            },
                            th.text_muted,
                            p.font_size * dpi / 96,
                            false,
                            false,
                            ui_text_font_family(),
                        );
                    } else {
                        let font = crate::win_system_ui::create_font_px(
                            ui_text_font_family(),
                            p.font_size * dpi / 96,
                            400,
                        );
                        let old = platform_gdi::select_object(hdc, font as _);
                        platform_gdi::set_bk_mode(hdc, 1);
                        platform_gdi::set_text_color(hdc, th.text);
                        let mut measure = RECT {
                            left: pad,
                            top: 0,
                            right: rc.right - pad,
                            bottom: 0,
                        };
                        let flags = 0x10 | 0x800 | 0x40; // WORDBREAK, NOPREFIX, EXPANDTABS
                        p.content_height = platform_gdi::draw_text(
                            hdc,
                            p.wide.as_ptr(),
                            p.wide.len() as i32,
                            &mut measure,
                            flags | 0x400,
                        );
                        let viewport = (rc.bottom - pad - top).max(1);
                        p.scroll = p.scroll.clamp(0, (p.content_height - viewport).max(0));
                        let saved = platform_gdi::save_dc(hdc);
                        platform_gdi::intersect_clip_rect(
                            hdc,
                            pad,
                            top,
                            rc.right - pad,
                            rc.bottom - pad,
                        );
                        let mut body = RECT {
                            left: pad,
                            top: top - p.scroll,
                            right: rc.right - pad,
                            bottom: top - p.scroll + p.content_height.max(viewport),
                        };
                        platform_gdi::draw_text(
                            hdc,
                            p.wide.as_ptr(),
                            p.wide.len() as i32,
                            &mut body,
                            flags,
                        );
                        platform_gdi::restore_dc(hdc, saved);
                        platform_gdi::select_object(hdc, old);
                    }
                }
            }
            platform_gdi::end_paint(hwnd, &ps);
            0
        }
        WM_DPICHANGED | WM_SIZE => {
            if msg == WM_DPICHANGED {
                position(hwnd);
            }
            platform_gdi::invalidate_rect(hwnd, null(), 1);
            0
        }
        WM_NCDESTROY => {
            let ptr = data(hwnd);
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
                platform_window::set_user_data(hwnd, 0);
            }
            0
        }
        _ => platform_window::default_window_proc(hwnd, msg, wparam, lparam),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preview_accepts_only_the_current_record_session_generation_and_request() {
        let ticket = Ticket {
            request: 4,
            session: 8,
            item: 12,
            generation: 16,
        };
        assert!(ticket_current(ticket, 8, 12, 16, 4));
        for args in [
            (9, 12, 16, 4),
            (8, 13, 16, 4),
            (8, 12, 17, 4),
            (8, 12, 16, 5),
        ] {
            assert!(!ticket_current(ticket, args.0, args.1, args.2, args.3));
        }
    }
    #[test]
    fn repeat_request_reuses_only_a_visible_current_ticket() {
        let ticket = Ticket { request: 4, session: 8, item: 12, generation: 16 };
        assert!(preview_request_reusable(Some(ticket), true, 8, 12, 16, 4));
        assert!(!preview_request_reusable(Some(ticket), false, 8, 12, 16, 4));
        assert!(!preview_request_reusable(None, true, 8, 12, 16, 4));
        for (session, item, generation, request) in [(9, 12, 16, 4), (8, 13, 16, 4), (8, 12, 17, 4), (8, 12, 16, 5)] {
            assert!(!preview_request_reusable(Some(ticket), true, session, item, generation, request));
        }
    }
}

use super::prelude::*;

pub(super) const WM_SEARCH_HOVER_READY: u32 = WM_APP + 111;
#[derive(Clone, Debug)]
struct HoverRequest {
    token: u64,
    generation: u64,
    request_seq: u64,
    item_id: i64,
}

#[derive(Default)]
struct HoverSlot {
    token: u64,
    running: bool,
    wanted: Option<HoverRequest>,
}

#[derive(Default)]
struct HoverQueue {
    slots: HashMap<isize, HoverSlot>,
}

impl HoverQueue {
    fn enqueue(&mut self, window: isize, mut request: HoverRequest) -> bool {
        let slot = self.slots.entry(window).or_default();
        slot.token = slot.token.wrapping_add(1).max(1);
        request.token = slot.token;
        slot.wanted = Some(request);
        if slot.running {
            false
        } else {
            slot.running = true;
            true
        }
    }
    fn next(&mut self, window: isize) -> Option<HoverRequest> {
        let slot = self.slots.get_mut(&window)?;
        let next = slot.wanted.take();
        if next.is_none() {
            slot.running = false;
        }
        next
    }
    fn cancel(&mut self, window: isize) {
        let slot = self.slots.entry(window).or_default();
        slot.token = slot.token.wrapping_add(1).max(1);
        slot.wanted = None;
    }
    fn current(&self, window: isize, token: u64) -> bool {
        self.slots
            .get(&window)
            .is_some_and(|slot| slot.token == token)
    }
}

fn hover_queue() -> &'static Mutex<HoverQueue> {
    static QUEUE: OnceLock<Mutex<HoverQueue>> = OnceLock::new();
    QUEUE.get_or_init(|| Mutex::new(HoverQueue::default()))
}

pub(super) fn cancel_hover_request(hwnd: HWND) {
    hover_queue()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .cancel(hwnd as isize);
}

struct HoverPreviewResult {
    item: Option<ClipItem>,
    request: HoverRequest,
    protection_revision: String,
}

fn hover_target_matches(
    visible: bool,
    minimized: bool,
    hovered: Option<i64>,
    under_pointer: Option<i64>,
    expected: i64,
) -> bool {
    visible && !minimized && hovered == Some(expected) && under_pointer == Some(expected)
}

pub(super) unsafe fn apply_hover_preview_result(hwnd: HWND, value: LPARAM) {
    if value == 0 {
        return;
    }
    let result = Box::from_raw(value as *mut HoverPreviewResult);
    if !hover_queue()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .current(hwnd as isize, result.request.token)
    {
        return;
    }
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    if state.app_data_generation != result.request.generation
        || state.search_results_pending()
        || !state.settings.hover_preview
        || state.edge_hidden
        || state.active_load_state().request_seq != result.request.request_seq
        || crate::db_runtime::search_protection_revision()
            .ok()
            .as_deref()
            != Some(result.protection_revision.as_str())
    {
        return;
    }
    let Some(screen) = platform_input::cursor_pos() else {
        return;
    };
    let mut point = screen;
    if !platform_window::screen_to_client(hwnd, &mut point) {
        return;
    }
    let row = hit_test_row(state, point.x, point.y);
    let under_pointer = if row < 0 {
        None
    } else {
        state
            .visible_src_idx(row as usize)
            .and_then(|i| state.active_items().get(i))
            .map(|item| item.id)
    };
    let hovered = hovered_item_clone(state).map(|item| item.id);
    if !hover_target_matches(
        platform_window::is_visible(hwnd),
        platform_window::is_minimized(hwnd),
        hovered,
        under_pointer,
        result.request.item_id,
    ) || platform_window::root_ancestor(platform_window::window_from_point(screen)) != hwnd
        || hover_preview_blocked_at_point(state, point.x, point.y)
    {
        return;
    }
    let Some(item) = result
        .item
        .and_then(super::state_runtime::protected_item_for_use)
    else {
        return;
    };
    state.payload_cache.put(&item);
    show_hover_preview(
        &item,
        screen.x,
        screen.y,
        state.settings.content_font_size(),
    );
}

fn run_hover_worker(window: isize) {
    loop {
        let Some(request) = hover_queue()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .next(window)
        else {
            return;
        };
        let protection_revision =
            crate::db_runtime::search_protection_revision().unwrap_or_default();
        let item = std::panic::catch_unwind(|| {
            crate::db_runtime::with_shared_app_data_generation(request.generation, || {
                db_load_item_full(request.item_id)
            })
            .flatten()
        })
        .ok()
        .flatten();
        if hover_queue()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .current(window, request.token)
        {
            let result = HoverPreviewResult {
                item,
                request,
                protection_revision,
            };
            unsafe {
                let _ = post_boxed_message(window, WM_SEARCH_HOVER_READY, 0, Box::new(result));
            }
        }
    }
}

pub(super) unsafe fn ensure_mouse_leave_tracking(hwnd: HWND) {
    platform_input::track_mouse_leave_and_hover(
        hwnd,
        platform_system_parameters::mouse_hover_time_ms(),
    );
}

pub(super) unsafe fn hover_preview_blocked_at_point(state: &AppState, x: i32, y: i32) -> bool {
    if scroll_to_top_visible(state) && pt_in_rect(x, y, &state.scroll_to_top_rect()) {
        return true;
    }
    let Some(item) = hovered_item_clone(state) else {
        return false;
    };
    row_quick_delete_rect(state, state.hover_idx, &item)
        .map(|rc| pt_in_rect(x, y, &rc))
        .unwrap_or(false)
}

unsafe fn refresh_hover_preview(hwnd: HWND, state: &mut AppState, x: i32, y: i32) {
    if !state.settings.hover_preview || state.edge_hidden || state.search_results_pending() {
        hide_hover_preview();
        return;
    }
    let Some(item_summary) = hovered_item_clone(state) else {
        hide_hover_preview();
        return;
    };
    if hover_preview_blocked_at_point(state, x, y) {
        return;
    }
    let Some(win_rc) = platform_window::window_rect(hwnd) else {
        hide_hover_preview();
        return;
    };
    let item = if matches!(item_summary.kind, ClipKind::Text | ClipKind::Phrase) {
        let window = hwnd as isize;
        let request = HoverRequest {
            token: 0,
            generation: state.app_data_generation,
            request_seq: state.active_load_state().request_seq,
            item_id: item_summary.id,
        };
        let start = hover_queue()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .enqueue(window, request);
        if start {
            std::thread::spawn(move || run_hover_worker(window));
        }
        hide_hover_preview();
        return;
    } else {
        item_summary
    };
    show_hover_preview(
        &item,
        win_rc.left + x,
        win_rc.top + y,
        state.settings.content_font_size(),
    );
}

pub(super) unsafe fn handle_mouse_hover_main(hwnd: HWND, position: UiPoint) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    refresh_hover_preview(hwnd, state, position.x, position.y);
}

pub(super) unsafe fn handle_mouse_leave_main(hwnd: HWND) {
    cancel_hover_request(hwnd);
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let transition = main_hover_target_from_state(state).clear_transition(true);
    if transition.changed {
        apply_main_hover_target(state, transition.next);
    }
    hide_hover_preview();
    if state.settings.edge_auto_hide && !state.edge_hidden && !vv_popup_menu_active() {
        if let Some(pt) = platform_input::cursor_pos() {
            if edge_window_scope_contains_point(hwnd, pt) {
                ensure_mouse_leave_tracking(hwnd);
            }
        }
    }
    if transition.changed {
        platform_gdi::invalidate_rect(hwnd, null(), 0);
    }
}

pub(super) unsafe fn clear_main_hover_state(hwnd: HWND) {
    cancel_hover_request(hwnd);
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let transition = main_hover_target_from_state(state).clear_transition(false);
    let mut dirty = transition.changed;
    if transition.changed {
        apply_main_hover_target(state, transition.next);
    }
    if state.down_to_top {
        state.down_to_top = false;
        dirty = true;
    }
    if state.down_row != -1 {
        state.down_row = -1;
        state.down_x = 0;
        state.down_y = 0;
        dirty = true;
    }
    hide_hover_preview();
    if dirty {
        platform_gdi::invalidate_rect(hwnd, null(), 0);
    }
}

pub(super) unsafe fn main_window_should_stay_noactivate(state: &AppState, x: i32, y: i32) -> bool {
    hit_test_row(state, x, y) >= 0
}

#[cfg(test)]
mod hover_request_tests {
    use super::*;
    fn request(item_id: i64) -> HoverRequest {
        HoverRequest {
            token: 0,
            generation: 1,
            request_seq: 1,
            item_id,
        }
    }
    #[test]
    fn busy_hover_keeps_only_latest_target_and_runs_it_without_more_mouse_events() {
        let mut queue = HoverQueue::default();
        assert!(queue.enqueue(1, request(10)));
        let a = queue.next(1).unwrap();
        assert!(!queue.enqueue(1, request(20)));
        assert!(!queue.enqueue(1, request(30)));
        assert!(!queue.current(1, a.token));
        let c = queue.next(1).unwrap();
        assert_eq!(c.item_id, 30);
        assert!(queue.current(1, c.token));
        assert!(queue.next(1).is_none());
        assert!(queue.enqueue(1, request(40)));
    }
    #[test]
    fn leaving_or_hiding_invalidates_a_finished_response_even_on_same_row_reentry() {
        let mut queue = HoverQueue::default();
        queue.enqueue(1, request(10));
        let a = queue.next(1).unwrap();
        queue.cancel(1);
        assert!(!queue.current(1, a.token));
        queue.enqueue(1, request(10));
        assert!(!queue.current(1, a.token));
        assert_eq!(queue.next(1).unwrap().item_id, 10);
    }
    #[test]
    fn completion_requires_a_visible_window_and_current_pointer_target() {
        assert!(hover_target_matches(true, false, Some(1), Some(1), 1));
        assert!(!hover_target_matches(false, false, Some(1), Some(1), 1));
        assert!(!hover_target_matches(true, true, Some(1), Some(1), 1));
        assert!(!hover_target_matches(true, false, Some(1), None, 1));
        assert!(!hover_target_matches(true, false, Some(1), Some(2), 1));
    }
}

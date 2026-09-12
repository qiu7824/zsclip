use super::prelude::*;
use crate::platform::string::to_wide;

struct HiddenSettingsSurface(HWND);

impl HiddenSettingsSurface {
    unsafe fn new() -> Self {
        let hwnd = platform_window::create_window_ex(
            0,
            to_wide("STATIC").as_ptr(),
            to_wide("Settings geometry test").as_ptr(),
            WS_POPUP,
            0,
            0,
            1080,
            800,
            null_mut(),
            null_mut(),
            platform_window::module_handle(),
            null(),
        );
        assert!(!hwnd.is_null());
        let surface = Self(hwnd);
        let state = create_settings_window_state(hwnd, null_mut());
        platform_window::set_user_data(hwnd, Box::into_raw(state) as isize);
        assert!(!platform_window::is_visible(hwnd));
        surface
    }
}

impl Drop for HiddenSettingsSurface {
    fn drop(&mut self) {
        unsafe {
            handle_settings_destroy(self.0);
            platform_window::destroy(self.0);
        }
    }
}

fn overlaps(a: UiRect, b: UiRect) -> bool {
    a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom
}

#[test]
fn hotkey_and_about_native_controls_do_not_overlap() {
    unsafe {
        let surface = HiddenSettingsSurface::new();
        let ptr = platform_window::user_data(surface.0) as *mut SettingsWndState;
        let state = &mut *ptr;
        settings_create_hotkey_page(surface.0, state);
        settings_create_about_page(surface.0, state);
        for page in [SettingsPage::Hotkey, SettingsPage::About] {
            let controls = state
                .ui
                .page_regs(page.index())
                .filter(|control| control.visible)
                .collect::<Vec<_>>();
            for (index, first) in controls.iter().enumerate() {
                for second in controls.iter().skip(index + 1) {
                    assert!(
                        !overlaps(first.bounds, second.bounds),
                        "{page:?}: {:?} {:?} overlap; {:?} / {:?}",
                        settings_host_text(first.hwnd),
                        settings_host_text(second.hwnd),
                        first.bounds,
                        second.bounds
                    );
                }
            }
        }
        assert!(!platform_window::is_visible(surface.0));
    }
}

#[test]
fn hotkey_modifier_labels_and_fields_share_the_same_row() {
    unsafe {
        let surface = HiddenSettingsSurface::new();
        let ptr = platform_window::user_data(surface.0) as *mut SettingsWndState;
        let state = &mut *ptr;
        settings_create_hotkey_page(surface.0, state);
        let controls = state
            .ui
            .page_regs(SettingsPage::Hotkey.index())
            .collect::<Vec<_>>();
        let main = controls
            .iter()
            .find(|control| control.hwnd == state.cb_hk_mod)
            .unwrap();
        let key = controls
            .iter()
            .find(|control| control.hwnd == state.cb_hk_key)
            .unwrap();
        let preview = controls
            .iter()
            .find(|control| control.hwnd == state.lb_hk_preview)
            .unwrap();
        let plain = controls
            .iter()
            .find(|control| control.hwnd == state.chk_plain_hk_enable)
            .unwrap();
        assert_eq!(main.bounds.top, key.bounds.top);
        assert!(preview.bounds.bottom < plain.bounds.top);
    }
}

fn panel_state(role: WindowRole) -> AppState {
    let mut state = AppState::new(
        role,
        null_mut(),
        null_mut(),
        Icons {
            app: 0,
            search: 0,
            setting: 0,
            min: 0,
            close: 0,
            text: 0,
            image: 0,
            file: 0,
            folder: 0,
            pin: 0,
            del: 0,
        },
        None,
    );
    state.settings.persistent_search_box = false;
    state.search_on = false;
    state.search_text.clear();
    state.ui_dpi = 96;
    state
}

#[test]
fn reopening_panels_preserves_each_panels_selected_tab() {
    let mut main = panel_state(WindowRole::Main);
    let mut quick = panel_state(WindowRole::Quick);
    main.tab_index = 1;
    quick.tab_index = 0;
    super::state::remember_shared_tab_view_state(&main);
    unsafe {
        crate::tray::prepare_panel_selection_for_test(&mut quick);
    }
    assert_eq!(quick.tab_index, 0);
    super::state::remember_shared_tab_view_state(&quick);
    unsafe {
        crate::tray::prepare_panel_selection_for_test(&mut main);
    }
    assert_eq!(main.tab_index, 1);
}

#[test]
fn scrolling_time_uses_variable_row_height_and_expires_after_scrolling() {
    let mut state = panel_state(WindowRole::Main);
    state.tab_index = 0;
    state.settings.image_preview_enabled = true;
    state.settings.image_row_height = 132;
    state.settings.text_row_height = 44;
    for (index, kind) in [ClipKind::Text, ClipKind::Image, ClipKind::Text]
        .into_iter()
        .enumerate()
    {
        state.records.push(ClipItem {
            id: index as i64 + 1,
            kind,
            preview: String::new(),
            text: None,
            rich_text_html: None,
            source_app: String::new(),
            file_paths: None,
            image_bytes: None,
            image_path: None,
            image_width: 0,
            image_height: 0,
            pinned: false,
            group_id: 0,
            created_at: format!("{}-07-01 12:00:00", 2024 + index),
        });
    }
    state.scroll_y = 45;
    assert!(scroll_time_hint_label(&state).unwrap().starts_with("2025-"));
    state.scroll_y = 177;
    assert!(scroll_time_hint_label(&state).unwrap().starts_with("2026-"));
    note_scroll_time_hint(&mut state);
    assert!(scroll_time_hint_visible(&state));
    state.scroll_date_hint_until = Some(Instant::now() - std::time::Duration::from_secs(1));
    assert!(!scroll_time_hint_visible(&state));
    state.scroll_dragging = true;
    assert!(scroll_time_hint_visible(&state));
}

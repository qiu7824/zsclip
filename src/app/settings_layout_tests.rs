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
        set_settings_ui_dpi(settings_window_layout_dpi(hwnd));
        platform_window::move_window(hwnd, 0, 0, settings_scale(1080), settings_scale(800), false);
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
fn unvisited_settings_pages_do_not_overwrite_saved_values() {
    unsafe {
        let surface = HiddenSettingsSurface::new();
        let state = &mut *(platform_window::user_data(surface.0) as *mut SettingsWndState);
        state.draft.show_mouse_dx = 77;
        state.draft.show_mouse_dy = -33;
        state.draft.show_fixed_x = 1200;
        state.draft.show_fixed_y = 850;
        state.draft.show_pos_mode = "fixed".into();
        state.draft.vv_group_id = 901;
        state.draft.max_items = 500;
        settings_collect_general_to_draft(state);
        settings_collect_group_to_draft(state);
        assert_eq!(state.draft.show_mouse_dx, 77);
        assert_eq!(state.draft.show_mouse_dy, -33);
        assert_eq!(state.draft.show_fixed_x, 1200);
        assert_eq!(state.draft.show_fixed_y, 850);
        assert_eq!(state.draft.show_pos_mode, "fixed");
        assert_eq!(state.draft.vv_group_id, 901);
        assert_eq!(state.draft.max_items, 500);
    }
}

#[test]
fn hotkey_and_about_native_controls_do_not_overlap() {
    unsafe {
        let surface = HiddenSettingsSurface::new();
        let ptr = platform_window::user_data(surface.0) as *mut SettingsWndState;
        let state = &mut *ptr;
        settings_create_hotkey_page(surface.0, state);
        settings_create_about_page(surface.0, state);
        settings_create_group_page(surface.0, state);
        settings_create_appearance_page(surface.0, state);
        settings_create_clipboard_page(surface.0, state);
        for page in [
            SettingsPage::Hotkey,
            SettingsPage::About,
            SettingsPage::Group,
            SettingsPage::Appearance,
            SettingsPage::Clipboard,
        ] {
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
fn background_plugin_sync_never_shows_controls_on_other_pages() {
    unsafe {
        let surface = HiddenSettingsSurface::new();
        let state = &mut *(platform_window::user_data(surface.0) as *mut SettingsWndState);
        state.draft.image_ocr_provider = "winocr".into();
        settings_show_page(surface.0, state, SettingsPage::Plugin.index());
        settings_show_page(surface.0, state, SettingsPage::About.index());
        settings_sync_page_state(state, SettingsPage::Plugin.index());
        for reg in state.ui.page_regs(SettingsPage::Plugin.index()) {
            assert_eq!(
                platform_window::window_style(reg.hwnd) & WS_VISIBLE,
                0,
                "Background plugin control became visible: {}",
                settings_host_text(reg.hwnd)
            );
        }
        settings_show_page(surface.0, state, SettingsPage::Plugin.index());
        assert_ne!(
            platform_window::window_style(state.ed_ocr_cloud_url) & WS_VISIBLE,
            0
        );
        assert_eq!(
            platform_window::window_style(state.ed_ocr_cloud_token) & WS_VISIBLE,
            0
        );
        refresh_settings_window_metrics(surface.0, state);
        let viewport = settings_viewport_rect(&platform_window::client_rect(surface.0).unwrap());
        for page in 0..SETTINGS_PAGE_LABELS.len() {
            for reg in state.ui.page_regs(page) {
                assert_eq!(
                    platform_window::window_style(reg.hwnd) & WS_VISIBLE != 0,
                    page == state.cur_page
                        && reg.visible
                        && (!reg.scrollable
                            || settings_child_visible(
                                reg.bounds.top - state.content_scroll_y,
                                reg.bounds.height(),
                                &viewport
                            ))
                );
            }
        }
        assert!(!platform_window::is_visible(surface.0));
    }
}

#[test]
fn scrolling_reuses_controls_and_only_moves_items_intersecting_the_viewport() {
    unsafe {
        let surface = HiddenSettingsSurface::new();
        let state = &mut *(platform_window::user_data(surface.0) as *mut SettingsWndState);
        settings_show_page(surface.0, state, SettingsPage::Appearance.index());
        let page = state.cur_page;
        let handles: Vec<_> = state.ui.page_regs(page).map(|reg| reg.hwnd).collect();
        assert!(handles.len() > 10);
        let client = platform_window::client_rect(surface.0).unwrap();
        let viewport = settings_viewport_rect(&client);
        for position in [0, 100, 200, 400] {
            settings_scroll_to(surface.0, state, settings_scale(position));
            assert_eq!(
                handles,
                state
                    .ui
                    .page_regs(page)
                    .map(|reg| reg.hwnd)
                    .collect::<Vec<_>>()
            );
            for reg in state.ui.page_regs(page).filter(|reg| reg.scrollable) {
                let visible = reg.visible
                    && settings_child_visible(
                        reg.bounds.top - state.content_scroll_y,
                        reg.bounds.height(),
                        &viewport,
                    );
                assert_eq!(
                    platform_window::window_style(reg.hwnd) & WS_VISIBLE != 0,
                    visible
                );
            }
        }
        settings_repos_controls(surface.0, state, false);
        assert!(
            state.scroll_moves.len() < handles.len(),
            "screen-external controls must not move every frame"
        );
        assert!(!platform_window::is_visible(surface.0));
    }
}

#[test]
fn plain_paste_controls_have_toggle_and_dropdown_roles_and_follow_enabled_state() {
    unsafe {
        let surface = HiddenSettingsSurface::new();
        let state = &mut *(platform_window::user_data(surface.0) as *mut SettingsWndState);
        settings_create_hotkey_page(surface.0, state);
        assert!(settings_owner_draw_is_toggle(IDC_SET_PLAIN_HK_ENABLE));
        assert!(settings_owner_draw_is_toggle(IDC_SET_RICH_TEXT));
        for cid in [
            IDC_SET_PLAIN_HK_MOD,
            IDC_SET_PLAIN_HK_KEY,
            IDC_SET_OCR_PROVIDER,
            IDC_SET_TRANSLATE_PROVIDER,
            IDC_SET_TRANSLATE_TARGET,
            IDC_SET_LAN_RECEIVE_MODE,
            IDC_SET_SEARCH_ENGINE,
        ] {
            assert_eq!(
                settings_owner_draw_button_kind(state, cid),
                crate::win_system_ui::SettingsComponentKind::Dropdown
            );
        }
        state.draft.plain_paste_hotkey_enabled = false;
        settings_sync_page_state(state, SettingsPage::Hotkey.index());
        for enabled in [true, false] {
            queue_settings_command(
                state,
                settings_command_for_control(IDC_SET_PLAIN_HK_ENABLE).unwrap(),
            );
            drain_settings_ui_commands(surface.0, state);
            assert_eq!(settings_toggle_get(state, IDC_SET_PLAIN_HK_ENABLE), enabled);
            assert_eq!(
                platform_window::is_enabled_by_style(state.cb_plain_hk_mod),
                enabled
            );
            assert_eq!(
                platform_window::is_enabled_by_style(state.cb_plain_hk_key),
                enabled
            );
        }
    }
}

#[test]
fn content_font_dropdown_opens_and_selects_through_window_messages() {
    unsafe {
        let surface = HiddenSettingsSurface::new();
        let state_ptr = platform_window::user_data(surface.0) as *mut SettingsWndState;
        settings_show_page(surface.0, &mut *state_ptr, SettingsPage::Appearance.index());
        let control = (*state_ptr).cb_content_font_size;
        assert!(!control.is_null());
        settings_wnd_proc(surface.0, WM_COMMAND, IDC_SET_CONTENT_FONT_SIZE as WPARAM, control as LPARAM);
        assert!(settings_dropdown_popup_exists((*state_ptr).dropdown_popup), "Font control command must reach the dropdown host");
        close_settings_dropdown_popup(&mut *state_ptr);
        settings_wnd_proc(surface.0, crate::settings_ui_host::WM_SETTINGS_DROPDOWN_SELECTED,
            IDC_SET_CONTENT_FONT_SIZE as WPARAM, 4);
        assert_eq!((*state_ptr).draft.content_font_size, 18);
        assert_eq!(settings_host_text(control), "18 px");
    }
}

#[test]
fn card_toggle_messages_disable_decorations_without_changing_saved_choices() {
    unsafe {
        let surface = HiddenSettingsSurface::new();
        let state_ptr = platform_window::user_data(surface.0) as *mut SettingsWndState;
        settings_show_page(surface.0, &mut *state_ptr, SettingsPage::Appearance.index());
        (*state_ptr).draft.card_view_enabled = false;
        (*state_ptr).draft.card_border_enabled = true;
        (*state_ptr).draft.card_shadow_enabled = false;
        settings_sync_page_state(&mut *state_ptr, SettingsPage::Appearance.index());
        for control in (*state_ptr).card_detail_controls {
            assert!(!platform_window::is_enabled_by_style(control));
        }
        settings_wnd_proc(surface.0, WM_COMMAND, IDC_SET_CARD_BORDER as WPARAM, 0);
        settings_wnd_proc(surface.0, WM_COMMAND, IDC_SET_CARD_SHADOW as WPARAM, 0);
        assert!((*state_ptr).draft.card_border_enabled);
        assert!(!(*state_ptr).draft.card_shadow_enabled);
        for enabled in [true, false, true] {
            settings_wnd_proc(surface.0, WM_COMMAND, IDC_SET_CARD_VIEW as WPARAM, 0);
            assert_eq!((*state_ptr).draft.card_view_enabled, enabled);
            for control in (*state_ptr).card_detail_controls {
                assert_eq!(platform_window::is_enabled_by_style(control), enabled);
            }
            assert!((*state_ptr).draft.card_border_enabled);
            assert!(!(*state_ptr).draft.card_shadow_enabled);
        }
    }
}

#[test]
fn group_actions_and_about_content_are_reachable_in_the_default_settings_window() {
    unsafe {
        let surface = HiddenSettingsSurface::new();
        let state = &mut *(platform_window::user_data(surface.0) as *mut SettingsWndState);
        platform_window::move_window(surface.0, 0, 0, settings_scale(1080), settings_scale(740), false);
        refresh_settings_window_metrics(surface.0, state);
        let client = platform_window::client_rect(surface.0).unwrap();
        let viewport = settings_viewport_rect(&client);
        for page in [SettingsPage::Group, SettingsPage::About] {
            settings_show_page(surface.0, state, page.index());
            let max_scroll = settings_page_max_scroll_for_state(state, page.index(), client.bottom - settings_content_y_scaled());
            let controls = state.ui.page_regs(page.index()).filter(|reg| reg.visible)
                .map(|reg| (reg.hwnd, reg.bounds, reg.scrollable)).collect::<Vec<_>>();
            for (control, bounds, scrollable) in controls {
                let target = if scrollable {
                    (bounds.top - viewport.top - settings_scale(16)).clamp(0, max_scroll)
                } else { 0 };
                settings_scroll_to(surface.0, state, target);
                let offset = if scrollable { state.content_scroll_y } else { 0 };
                assert!(
                    bounds.top - offset >= viewport.top && bounds.bottom - offset <= viewport.bottom,
                    "{page:?} control cannot be reached by scrolling: {} {:?}, scroll={offset}",
                    settings_host_text(control), bounds
                );
                assert_ne!(platform_window::window_style(control) & WS_VISIBLE, 0,
                    "{page:?} control stayed hidden after scrolling: {}", settings_host_text(control));
            }
        }
    }
}

#[test]
fn resized_settings_cards_and_controls_keep_their_right_margin() {
    unsafe {
        let surface = HiddenSettingsSurface::new();
        let state = &mut *(platform_window::user_data(surface.0) as *mut SettingsWndState);
        for width in [920, 960, 1080, 1360] {
            platform_window::move_window(
                surface.0,
                0,
                0,
                settings_scale(width),
                settings_scale(740),
                false,
            );
            refresh_settings_window_metrics(surface.0, state);
            for page in [
                SettingsPage::Hotkey,
                SettingsPage::Group,
                SettingsPage::About,
            ] {
                settings_ensure_page(surface.0, state, page.index());
                let client = platform_window::client_rect(surface.0).unwrap();
                for card in crate::settings_model::settings_cards_for_page_vec(page.index()) {
                    assert_eq!(card.rect.right, client.right - settings_scale(28));
                }
                for reg in state.ui.page_regs(page.index()).filter(|reg| reg.visible) {
                    assert!(
                        reg.bounds.right <= client.right - settings_scale(28),
                        "{page:?} control clipped at width {width}: {} {:?}",
                        settings_host_text(reg.hwnd),
                        reg.bounds
                    );
                }
            }
        }
    }
}

#[test]
fn plugin_edits_survive_page_switch_resize_and_toggle() {
    unsafe {
        let surface = HiddenSettingsSurface::new();
        let state = &mut *(platform_window::user_data(surface.0) as *mut SettingsWndState);
        state.draft.image_ocr_provider = "winocr".into();
        settings_show_page(surface.0, state, SettingsPage::Plugin.index());
        let path = r"D:\Apps\WeChat\test-profile";
        settings_set_text(state.ed_ocr_cloud_url, path);
        settings_show_page(surface.0, state, SettingsPage::About.index());
        assert_eq!(state.draft.image_ocr_wechat_dir, path);
        settings_show_page(surface.0, state, SettingsPage::Plugin.index());
        assert_eq!(settings_host_text(state.ed_ocr_cloud_url), path);
        refresh_settings_window_metrics(surface.0, state);
        assert_eq!(settings_host_text(state.ed_ocr_cloud_url), path);
        settings_set_text(state.ed_ocr_cloud_url, r"D:\Apps\WeChat\edited");
        queue_settings_command(state, settings_command_for_control(7101).unwrap());
        drain_settings_ui_commands(surface.0, state);
        assert_eq!(
            settings_host_text(state.ed_ocr_cloud_url),
            r"D:\Apps\WeChat\edited"
        );
    }
}

#[test]
#[ignore = "Interactive native settings smoke test; requires an isolated ZSCLIP_DATA_DIR"]
fn interactive_settings_surface() {
    assert!(std::env::var_os("ZSCLIP_DATA_DIR").is_some());
    unsafe {
        platform_dpi::init_process_awareness();
        set_settings_ui_dpi(platform_dpi::layout_dpi_for_point(POINT { x: 100, y: 100 }));
        let owner = platform_window::create_window_ex(
            0,
            to_wide("STATIC").as_ptr(),
            to_wide("").as_ptr(),
            WS_POPUP,
            0,
            0,
            1,
            1,
            null_mut(),
            null_mut(),
            platform_window::module_handle(),
            null(),
        );
        let mut app = Box::new(panel_state(WindowRole::Main));
        app.settings.image_ocr_provider = "winocr".into();
        app.settings.image_ocr_wechat_dir = r"D:\Apps\WeChat".into();
        app.settings.hotkey_enabled = false;
        app.settings.vv_mode_enabled = false;
        app.settings.mouse_side_button_enabled = false;
        platform_window::set_user_data(owner, (&mut *app as *mut AppState) as isize);
        let mut host = WindowsSettingsWindowHost::new(Some(settings_wnd_proc));
        let result = host.present_settings_window(NativeSettingsWindowRequest {
            owner,
            existing: None,
            bounds: UiRect::new(
                100,
                100,
                100 + settings_w_scaled(),
                100 + settings_h_scaled(),
            ),
        });
        let NativeSettingsWindowPresentation::Created(hwnd) = result else {
            panic!("Settings window creation failed");
        };
        let state = &mut *(platform_window::user_data(hwnd) as *mut SettingsWndState);
        state.parent_hwnd = null_mut();
        let page = std::env::var("ZSCLIP_UI_SMOKE_PAGE")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(SettingsPage::Group.index());
        settings_show_page(hwnd, state, page);
        platform_window::restore(hwnd);
        platform_window::set_foreground(hwnd);
        assert!(platform_window::is_visible(hwnd));
        let mut message: MSG = std::mem::zeroed();
        while platform_window::exists(hwnd) && platform_window::get_message(&mut message) > 0 {
            if !dismiss_settings_dropdown_for_message(&message)
                && !route_settings_child_mouse_wheel(&message)
            {
                platform_window::translate_message(&message);
                platform_window::dispatch_message(&message);
            }
        }
        platform_window::set_user_data(owner, 0);
        platform_window::destroy(owner);
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
            phrase_title: String::new(),
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

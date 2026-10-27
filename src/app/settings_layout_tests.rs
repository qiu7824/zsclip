use super::prelude::*;
use crate::platform::string::to_wide;

struct HiddenSettingsSurface(HWND);

#[test]
#[ignore = "Renders all settings pages off-screen in dark mode using an isolated profile"]
fn settings_dark_pages_refresh_native_fields_and_capture_surface_colors() {
    use std::path::PathBuf;
    let output=PathBuf::from(std::env::var_os("ZSCLIP_TEST_TEMP_ROOT").expect("isolated output"));
    unsafe extern "system" fn audit_proc(hwnd:HWND,msg:u32,w:WPARAM,l:LPARAM)->LRESULT {
        if matches!(msg,WM_SIZE|WM_MOVE|WM_SHOWWINDOW|WM_DPICHANGED|WM_SETTINGCHANGE){
            return platform_window::default_window_proc(hwnd,msg,w,l);
        }
        settings_wnd_proc(hwnd,msg,w,l)
    }
    struct AuditWindow(HWND);
    impl Drop for AuditWindow{fn drop(&mut self){platform_window::destroy(self.0);}}
    struct SnapshotEnv(Option<std::ffi::OsString>);
    impl Drop for SnapshotEnv{fn drop(&mut self){unsafe{if let Some(value)=&self.0{std::env::set_var("ZSCLIP_SETTINGS_SNAPSHOT",value);}else{std::env::remove_var("ZSCLIP_SETTINGS_SNAPSHOT");}}}}
    crate::db_runtime::with_test_db(||unsafe{
        let _environment=SnapshotEnv(std::env::var_os("ZSCLIP_SETTINGS_SNAPSHOT"));
        platform_appearance::set_dark_mode_enabled(true);
        let class=to_wide("ZSClipSettingsDarkAudit");
        platform_window::register_class_ex(&WNDCLASSEXW{cbSize:std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc:Some(audit_proc),hInstance:platform_window::module_handle(),lpszClassName:class.as_ptr(),..std::mem::zeroed()});
        let hwnd=platform_window::create_window_ex(WS_EX_TOOLWINDOW,class.as_ptr(),to_wide("Settings style audit").as_ptr(),
            WS_POPUP|WS_CLIPCHILDREN,-16000,-16000,1080,800,null_mut(),null_mut(),platform_window::module_handle(),null());
        assert!(!hwnd.is_null());let _window=AuditWindow(hwnd);
        platform_window::show_no_activate(hwnd);
        let state=&mut *(platform_window::user_data(hwnd) as *mut SettingsWndState);
        state.draft.dark_mode_enabled=true;
        for page in 0..8 {
            settings_show_page(hwnd,state,page);
            state.content_scroll_y=0;
            settings_repos_controls(hwnd,state,true);
            std::env::set_var("ZSCLIP_SETTINGS_SNAPSHOT",output.join(format!("settings-dark-{page}-top.png")));
            platform_gdi::invalidate_rect(hwnd,null(),0);paint_settings_window(hwnd);
            std::env::remove_var("ZSCLIP_SETTINGS_SNAPSHOT");
            if page==SettingsPage::Appearance.index() {
                let label=state.card_detail_controls[0];
                assert!(!platform_window::is_enabled(label));
                let rect=platform_window::window_rect(label).unwrap();
                let mut origin=POINT{x:rect.left,y:rect.top};platform_window::screen_to_client(hwnd,&mut origin);
                let decoder=png::Decoder::new(std::fs::File::open(output.join("settings-dark-1-top.png")).unwrap());
                let mut reader=decoder.read_info().unwrap();let mut pixels=vec![0;reader.output_buffer_size()];
                let info=reader.next_frame(&mut pixels).unwrap();
                assert!(origin.x>=0&&origin.y>=0);
                let mut brightest=0;
                for y in origin.y..(origin.y+rect.bottom-rect.top).min(info.height as i32) {
                    for x in origin.x..(origin.x+rect.right-rect.left).min(info.width as i32) {
                        let offset=(y as usize*info.width as usize+x as usize)*4;
                        brightest=brightest.max(*pixels[offset..offset+3].iter().max().unwrap());
                    }
                }
                assert!(brightest<190,"Disabled labels must not retain light-mode embossed pixels: {brightest}");
            }
            let client=settings_window_client_bounds(hwnd).unwrap();
            state.content_scroll_y=settings_page_max_scroll_for_state(state,page,(client.bottom-settings_content_y_scaled()).max(0));
            settings_repos_controls(hwnd,state,true);
            std::env::set_var("ZSCLIP_SETTINGS_SNAPSHOT",output.join(format!("settings-dark-{page}-bottom.png")));
            platform_gdi::invalidate_rect(hwnd,null(),0);paint_settings_window(hwnd);
            std::env::remove_var("ZSCLIP_SETTINGS_SNAPSHOT");
        }
        // Existing controls must change their native borders in both directions.
        let list=state.lb_groups;
        assert!(!list.is_null());
        platform_appearance::set_dark_mode_enabled(false);settings_refresh_theme_resources(state);
        assert_ne!(platform_window::window_ex_style(list)&WS_EX_CLIENTEDGE,0);
        platform_appearance::set_dark_mode_enabled(true);settings_refresh_theme_resources(state);
        assert_eq!(platform_window::window_ex_style(list)&WS_EX_CLIENTEDGE,0);
        for id in [super::settings_customization::OPEN_CATEGORIES,super::settings_customization::OPEN_QUICK_ACTIONS,
            crate::win_system_params::IDC_SET_DATA_DIRECTORY,IDC_SET_BTN_OPENCFG,super::main_secret_vault::OPEN_VAULT as isize] {
            assert!(is_settings_surface_control(id),"Card control {id} must use the surface background");
        }
        std::env::remove_var("ZSCLIP_SETTINGS_SNAPSHOT");
        Ok(())
    }).unwrap();
}

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
fn every_scrollable_settings_control_stays_inside_a_card() {
    unsafe {
        let surface = HiddenSettingsSurface::new();
        let state = &mut *(platform_window::user_data(surface.0) as *mut SettingsWndState);
        for page in 0..SETTINGS_PAGE_LABELS.len() {
            settings_show_page(surface.0, state, page);
            let sections = settings_content_render_plan(
                page,
                0,
                &state.plugin_sections,
                &state.multi_sync_sections,
            )
            .sections;
            for control in state.ui.page_regs(page).filter(|c| c.visible && c.scrollable) {
                let bounds = control.bounds;
                assert!(
                    sections.iter().any(|section| {
                        let card = section.rect;
                        bounds.left >= card.left
                            && bounds.right <= card.right
                            && bounds.top >= card.top
                            && bounds.bottom <= card.bottom
                    }),
                    "page {page}: {:?} {bounds:?} leaves its card; cards {:?}",
                    settings_host_text(control.hwnd),
                    sections.iter().map(|s| s.rect).collect::<Vec<_>>()
                );
            }
        }
    }
}

#[test]
fn notes_plugin_toggle_controls_entry_and_roundtrips_without_enabling_by_default() {
    unsafe {
        let surface = HiddenSettingsSurface::new();
        let st = &mut *(platform_window::user_data(surface.0) as *mut SettingsWndState);
        st.cur_page = SettingsPage::Plugin.index();
        st.draft.notes_plugin_enabled = false;
        settings_create_plugin_page(surface.0, st);
        settings_sync_plugin_page_state(st);
        assert!(!platform_window::is_enabled(st.btn_notes));
        let command = settings_command_for_control(IDC_SET_NOTES_ENABLE).unwrap();
        queue_settings_command(st, command);
        drain_settings_ui_commands(surface.0, st);
        assert!(st.draft.notes_plugin_enabled);
        assert!(platform_window::is_enabled(st.btn_notes));
        let reopened: AppSettings = serde_json::from_slice(&serde_json::to_vec(&st.draft).unwrap()).unwrap();
        assert!(reopened.notes_plugin_enabled);
        assert!(!AppSettings::default().notes_plugin_enabled);
        let mut inaccessible = AppSettings::default();
        inaccessible.silent_start = true;
        inaccessible.tray_icon_enabled = false;
        inaccessible.hotkey_enabled = false;
        inaccessible.mouse_side_button_enabled = true;
        inaccessible.mouse_side_button_1_action = "notes".into();
        inaccessible.mouse_side_button_2_action = "none".into();
        assert!(!startup_can_hide(&inaccessible));
        inaccessible.notes_plugin_enabled = true;
        assert!(startup_can_hide(&inaccessible));
    }
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
    assert!(std::env::var_os("ZSCLIP_DATA_DIR").is_some()
        || std::env::var_os("ZSCLIP_TEST_TEMP_ROOT").is_some());
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
            if super::notes_window::route_message(&message) { continue; }
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

#[test]
#[ignore = "Interactive list/menu smoke test; requires an isolated ZSCLIP_DATA_DIR; F10 opens the tray menu"]
fn interactive_clipboard_surface() {
    assert!(std::env::var_os("ZSCLIP_DATA_DIR").is_some());
    unsafe extern "system" fn smoke_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        if matches!(msg, WM_KEYDOWN | WM_SYSKEYDOWN) && w == 0x75 {
            platform_window::post_hwnd_message(hwnd, WM_TOGGLE_NOTES, 0, 0);
            return 0;
        }
        if matches!(msg, WM_KEYDOWN | WM_SYSKEYDOWN) && w == 0x79 {
            crate::tray::show_tray_menu_localized(hwnd);
            return 0;
        }
        wnd_proc(hwnd, msg, w, l)
    }
    unsafe {
        platform_dpi::init_process_awareness();
        platform_appearance::set_dark_mode_enabled(load_settings().dark_mode_enabled);
        platform_appearance::init_dark_mode_for_process();
        let layout = main_layout_for_window(null_mut());
        let spec = crate::zsui::Window::new("ZSClip UI smoke")
            .size(layout.win_w as u32, (layout.list_y + layout.list_h + 7) as u32)
            .decorations(false).resizable(true).visible(true);
        let mut host = WindowsMainWindowHost::new(Some(smoke_proc));
        let NativeMainWindowPresentation::Created(handles) = host.create_main_windows(
            NativeMainWindowRequest::from_zsui_window_for_host(&spec,
                &crate::zsui::HostCapabilities::windows_native_window_host()),
        ) else { panic!("Main window creation failed") };
        // Expose this fixture in desktop accessibility inventories while
        // retaining the production client renderer and message handlers.
        let style = platform_window::window_ex_style(handles.main);
        platform_window::set_window_ex_style(handles.main, style & !WS_EX_TOOLWINDOW);
        (*get_state_ptr(handles.main)).settings.close_without_exit = false;
        host.present_main_window(handles.main, NativeMainWindowPresentMode::ActivateAndFocus);
        let mut message: MSG = std::mem::zeroed();
        while platform_window::exists(handles.main) && platform_window::get_message(&mut message) > 0 {
            if super::notes_window::route_message(&message) { continue; }
            platform_window::translate_message(&message);
            platform_window::dispatch_message(&message);
        }
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
fn pin_button_save_updates_live_and_reloaded_main_panel() {
    crate::db_runtime::with_test_db(|| unsafe {
        let owner = platform_window::create_window_ex(
            0, to_wide("STATIC").as_ptr(), to_wide("Pin save fixture").as_ptr(),
            WS_POPUP, 0, 0, 300, 600, null_mut(), null_mut(),
            platform_window::module_handle(), null(),
        );
        assert!(!owner.is_null());
        let mut app = Box::new(panel_state(WindowRole::Main));
        app.hwnd = owner;
        app.settings = AppSettings::default();
        app.settings.hotkey_enabled = false;
        app.settings.vv_mode_enabled = false;
        app.settings.mouse_side_button_enabled = false;
        app.settings.tray_icon_enabled = false;
        app.settings.cloud_sync_enabled = false;
        app.settings.lan_sync_enabled = false;
        platform_window::set_user_data(owner, (&mut *app as *mut AppState) as isize);
        let surface = HiddenSettingsSurface::new();
        let state = &mut *(platform_window::user_data(surface.0) as *mut SettingsWndState);
        state.parent_hwnd = owner;
        settings_apply_from_app(state);
        settings_show_page(surface.0, state, SettingsPage::Appearance.index());
        for enabled in [true, false, true] {
            let command = settings_command_for_control(IDC_SET_SHOW_PIN_BUTTON).unwrap();
            queue_settings_command(state, command);
            drain_settings_ui_commands(surface.0, state);
            assert_eq!(state.draft.show_pin_button, enabled, "toggle draft");
            queue_settings_command(state, settings_command_for_control(IDC_SET_SAVE).unwrap());
            drain_settings_ui_commands(surface.0, state);
            assert_eq!(app.settings.show_pin_button, enabled, "live app after Save");
            assert_eq!(load_settings().show_pin_button, enabled, "persisted after Save");
            assert_eq!(window_pin_visible(&app), enabled, "live title bar");
            assert_eq!(panel_state(WindowRole::Main).layout().pin_button_visible, enabled,
                "title bar after restart");
        }
        state.parent_hwnd = null_mut();
        platform_window::set_user_data(owner, 0);
        platform_window::destroy(owner);
        shutdown_low_level_input_hooks();
        Ok(())
    }).unwrap();
}

#[test]
fn pin_button_setting_applies_to_both_panel_entry_points() {
    for role in [WindowRole::Main, WindowRole::Quick] {
        let mut state = panel_state(role);
        for enabled in [false, true, false] {
            state.settings.show_pin_button = enabled;
            assert_eq!(window_pin_visible(&state), enabled, "{role:?} title button");
            assert_eq!(state.layout().pin_button_visible, enabled, "{role:?} title layout");
        }
    }
}

#[test]
fn pin_button_controls_topmost_and_outside_hide_for_both_panels() {
    unsafe extern "system" fn pin_fixture_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        platform_window::default_window_proc(hwnd, msg, w, l)
    }
    unsafe {
        let class = to_wide("ZSClipPinBehaviorFixture");
        let definition = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(pin_fixture_proc),
            hInstance: platform_window::module_handle(),
            lpszClassName: class.as_ptr(),
            ..std::mem::zeroed()
        };
        platform_window::register_class_ex(&definition);
        for role in [WindowRole::Main, WindowRole::Quick] {
            let hwnd = platform_window::create_window_ex(
                0, class.as_ptr(), to_wide("Pin behavior fixture").as_ptr(),
                WS_POPUP, 0, 0, 300, 600, null_mut(), null_mut(),
                platform_window::module_handle(), null(),
            );
            assert!(!hwnd.is_null());
            let mut state = Box::new(panel_state(role));
            state.hwnd = hwnd;
            state.settings.show_pin_button = true;
            state.settings.auto_hide_on_blur = true;
            platform_window::set_user_data(hwnd, (&mut *state as *mut AppState) as isize);
            // Pinning is a visible-window interaction. Establish the native
            // window's initial Z order before checking subsequent transitions.
            platform_window::show_no_activate(hwnd);
            assert!(window_auto_hide_on_blur(&state));
            toggle_window_pin(hwnd, &mut state);
            assert!(state.window_pinned);
            assert!(!window_auto_hide_on_blur(&state));
            assert_ne!(platform_window::window_ex_style(hwnd) & WS_EX_TOPMOST, 0,
                "{role:?} visible={} style={:#x}", platform_window::is_visible(hwnd),
                platform_window::window_ex_style(hwnd));
            toggle_window_pin(hwnd, &mut state);
            assert!(!state.window_pinned);
            assert!(window_auto_hide_on_blur(&state));
            assert_eq!(platform_window::window_ex_style(hwnd) & WS_EX_TOPMOST != 0,
                role == WindowRole::Quick);
            toggle_window_pin(hwnd, &mut state);
            state.settings.show_pin_button = false;
            refresh_window_pin(hwnd);
            assert!(!state.window_pinned, "hiding the button must release its pin");
            assert!(window_auto_hide_on_blur(&state));
            state.settings.show_pin_button = true;
            assert!(!state.window_pinned, "re-enabling the button must start unpinned");
            state.settings.auto_hide_on_blur = false;
            assert!(!window_auto_hide_on_blur(&state));
            platform_window::hide(hwnd);
            assert!(!platform_window::is_visible(hwnd));
            platform_window::set_user_data(hwnd, 0);
            platform_window::destroy(hwnd);
        }
    }
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
    for cards in [false, true] {
        state.settings.card_view_enabled = cards;
        state.scroll_y = 45 + if cards { 6 } else { 0 };
        assert!(scroll_time_hint_label(&state).unwrap().starts_with("2025-"));
        state.scroll_y = 177 + if cards { 12 } else { 0 };
        assert!(scroll_time_hint_label(&state).unwrap().starts_with("2026-"));
    }
    note_scroll_time_hint(&mut state);
    assert!(scroll_time_hint_visible(&state));
    state.scroll_date_hint_until = Some(Instant::now() - std::time::Duration::from_secs(1));
    assert!(!scroll_time_hint_visible(&state));
    state.scroll_dragging = true;
    assert!(scroll_time_hint_visible(&state));
}

#[test]
fn category_views_bind_queries_without_moving_or_deleting_records() {
    use crate::app_core::customization::{CategoryTab, CategoryTabMode};
    crate::db_runtime::with_test_db(|| {
        with_db(|db|db.execute_batch("INSERT INTO items(category,kind,preview,group_id) VALUES
            (0,'text','text',7),(0,'image','photo',7),(0,'image','other photo',8),(1,'phrase','phrase',0)"))?;
        let mut state=panel_state(WindowRole::Main);
        state.settings.grouping_enabled=false;state.settings.group_type_filter_enabled=false;
        state.settings.category_tabs=vec![
            CategoryTab{id:"photos".into(),name:"工程照片".into(),group_id:7,kind:"image".into(),mode:CategoryTabMode::Filter,..Default::default()},
            CategoryTab{id:"wording".into(),name:"常用表达".into(),source:1,mode:CategoryTabMode::Filter,..Default::default()}];
        state.active_category_id="photos".into();state.reconcile_category();
        let query=state.desired_query_for_tab(0);
        let rows=db_load_items_page(&query,None,100)?.0;
        assert_eq!(rows.len(),1);assert_eq!(rows[0].preview,"photo");
        state.settings.category_tabs[0].name="现场影像".into();
        state.active_category_id="wording".into();state.tab_index=1;state.reconcile_category();
        let rows=db_load_items_page(&state.desired_query_for_tab(1),None,100)?.0;
        assert_eq!(rows.len(),1);assert_eq!(rows[0].preview,"phrase");
        state.settings.category_tabs.remove(0);
        let count:i64=with_db(|db|db.query_row("SELECT count(*) FROM items",[],|r|r.get(0)))?;
        assert_eq!(count,4);
        Ok(())
    }).unwrap();
}

#[test]
fn custom_quick_buttons_follow_visibility_order_and_pending_query_guards() {
    use crate::app_core::customization::QuickRowAction;
    let mut state=panel_state(WindowRole::Main);
    state.records.push(ClipItem {id:77,kind:ClipKind::Text,preview:"entry".into(),phrase_title:String::new(),text:None,
        rich_text_html:None,source_app:String::new(),file_paths:None,image_bytes:None,image_path:None,image_width:0,image_height:0,
        pinned:false,group_id:0,created_at:String::new()});
    state.list.apply_visible_len(1);state.hover_idx=0;
    state.settings.quick_delete_button=true;state.settings.quick_pin_button=true;state.settings.quick_edit_button=true;
    state.settings.quick_action_order="edit,pin,delete".into();
    assert_eq!(state.row_actions(&state.records[0]),[QuickRowAction::Edit,QuickRowAction::Pin,QuickRowAction::Delete]);
    for (slot,action) in [(0,QuickRowAction::Delete),(1,QuickRowAction::Pin),(2,QuickRowAction::Edit)] {
        let rect=state.layout().quick_action_rect(0,1,0,slot).unwrap();
        assert_eq!(state.quick_action_hit(rect.left+2,rect.top+2),Some((77,action)));
    }
    state.search_debounce_timer=true;
    let rect=state.layout().quick_action_rect(0,1,0,0).unwrap();
    assert!(state.quick_action_hit(rect.left+2,rect.top+2).is_none());
    state.search_debounce_timer=false;state.settings.quick_delete_button=false;
    assert_eq!(state.row_actions(&state.records[0]),[QuickRowAction::Edit,QuickRowAction::Pin]);
    state.settings.category_tabs[1].visible=false;
    assert!(state.category_bar().hit(260,50).is_none());
    let previous=state.tab_index;
    assert!(unsafe {super::main_customization::handle_category_click(null_mut(),&mut state,260,50)});
    assert_eq!(state.tab_index,previous,"Blank space after compact tabs must not act as the old stretched tab hit target");
}

#[test]
fn manual_tags_are_empty_until_assigned_and_follow_records_through_promotion() {
    use crate::app_core::customization::CategoryTab;
    crate::db_runtime::with_test_db(|| {
        with_db(|db| db.execute_batch("INSERT INTO items(id,category,kind,preview,text_data)
            VALUES(1,0,'text','alpha','alpha'),(2,1,'phrase','beta','beta'),(3,0,'image','photo',NULL);"))?;
        let mut state = panel_state(WindowRole::Main);
        state.settings.category_tabs.push(CategoryTab { id:"project".into(),name:"项目".into(),..Default::default() });
        state.settings.category_tabs.push(CategoryTab { id:"review".into(),name:"待复核".into(),..Default::default() });
        state.active_category_id="project".into(); state.reconcile_category();
        let query=state.desired_query_for_tab(0);
        assert_eq!(query.manual_tab_id.as_deref(),Some("project"));
        assert!(db_load_items_page(&query,None,10)?.0.is_empty());
        super::main_customization::change_tag_membership(&state,"project",&[1,2,3],false).unwrap();
        super::main_customization::change_tag_membership(&state,"review",&[1],false).unwrap();
        super::main_customization::change_tag_membership(&state,"project",&[1],false).unwrap();
        assert_eq!(db_load_items_page(&query,None,10)?.0.len(),3);
        let (page,cursor,more)=db_load_items_page(&query,None,1)?;
        assert_eq!(page.len(),1); assert!(more);
        assert_eq!(db_load_items_page(&query,cursor,10)?.0.len(),2);
        let mut search=query.clone(); search.search_text="alpha".into();
        assert_eq!(db_load_items_page(&search,None,10)?.0.len(),1);
        let new_id=db_promote_item_to_top(1)?;
        assert_eq!(crate::clip_tags::counts(&[new_id])?.len(),2);
        assert!(!db_load_items_page(&query,None,10)?.0.iter().any(|item|item.id==1));
        super::main_customization::change_tag_membership(&state,"project",&[new_id],true).unwrap();
        assert_eq!(crate::clip_tags::counts(&[new_id])?["review"],1);
        state.settings.category_tabs[2].name="重命名".into();
        assert_eq!(db_load_items_page(&state.desired_query_for_tab(0),None,10)?.0.len(),2);
        let saved=serde_json::to_string(&state.settings).unwrap();
        state.settings=serde_json::from_str(&saved).unwrap(); state.reconcile_category();
        assert_eq!(db_load_items_page(&state.desired_query_for_tab(0),None,10)?.0.len(),2);
        state.settings.category_tabs.remove(2); state.reconcile_category();
        assert!(super::main_customization::change_tag_membership(&state,"project",&[new_id],false).is_err());
        crate::clip_tags::delete_memberships(&["project".into()])?;
        assert_eq!(with_db(|db| db.query_row("SELECT count(*) FROM items",[],|r|r.get::<_,i64>(0)))?,3);
        assert_eq!(crate::clip_tags::counts(&[new_id])?["review"],1);
        Ok(())
    }).unwrap();
}

#[test]
fn tag_assignment_menu_works_without_old_grouping_and_keeps_phrase_action() {
    use crate::app_core::customization::CategoryTab;
    crate::db_runtime::with_test_db(|| {
        with_db(|db| db.execute_batch("INSERT INTO items(id,category,kind,preview,text_data) VALUES(1,0,'text','alpha','alpha');"))?;
        let mut state=panel_state(WindowRole::Main);
        state.settings.grouping_enabled=false;
        state.settings.category_tabs.push(CategoryTab {id:"project".into(),name:"项目".into(),..Default::default()});
        state.records=db_load_items_page(&state.desired_query_for_tab(0),None,10)?.0;
        state.list.apply_visible_len(1); state.sel_idx=0;
        let plan=state.list.context_menu_state_plan(0,false,false).unwrap();
        state.list.apply_context_menu_state_plan(plan);
        let entries=super::main_customization::tag_row_menu_entries(&state);
        assert!(entries.iter().any(|entry|matches!(entry,NativePopupMenuEntry::Submenu{label,entries,..}
            if label==tr("添加到标签","Add to Tag") && entries.iter().any(|entry|matches!(entry,
                NativePopupMenuEntry::Command{label,enabled:true,..} if label=="项目")))));
        let plan=crate::app_core::main_row_menu_plan(MainRowMenuInput { grouping_enabled:false,current_kind:ClipKind::Text,
            selected_count:1,has_unpinned:true,context_menu_copy_enabled:false,current_can_ocr:false,
            current_can_translate:false,current_is_excel:false,quick_search_enabled:false,qr_quick_enabled:false,
            super_mail_merge_enabled:false,lan_push_available:false });
        assert!(plan.entries.iter().any(|e| matches!(e,crate::app_core::MainRowMenuEntry::Action {
            action:MainRowMenuAction::ToPhrase,enabled:true })));
        Ok(())
    }).unwrap();
}

#[test]
fn capturing_into_manual_tag_keeps_it_empty_and_deduplication_preserves_memberships() {
    use crate::app_core::customization::CategoryTab;
    crate::db_runtime::with_test_db(|| {
        let mut state=panel_state(WindowRole::Main);
        state.settings.lan_sync_enabled=false;
        state.settings.max_items=0;
        state.settings.dedupe_filter_enabled=true;
        state.settings.category_tabs.push(CategoryTab{id:"project".into(),name:"项目".into(),..Default::default()});
        state.active_category_id="project".into(); state.reconcile_category();
        let query=state.desired_query_for_tab(0);
        state.load_state_for_tab_mut(0).query=Some(query.clone());
        let item=ClipItem {id:0,kind:ClipKind::Text,preview:"captured text".into(),phrase_title:String::new(),
            text:Some("captured text".into()),rich_text_html:None,source_app:String::new(),file_paths:None,
            image_bytes:None,image_path:None,image_width:0,image_height:0,pinned:false,group_id:0,created_at:String::new()};
        assert!(state.add_clip_item(item.clone(),"manual-tag-capture".into()));
        assert!(state.records.is_empty());
        assert!(db_load_items_page(&query,None,10)?.0.is_empty());
        let id=with_db(|db|db.query_row("SELECT max(id) FROM items",[],|r|r.get::<_,i64>(0)))?;
        crate::clip_tags::assign("project",&[id])?;
        let signature=dedupe_signature_for_item(&item,"manual-tag-capture");
        let duplicate=db_insert_item(0,&item,Some(&signature))?;
        crate::clip_tags::assign("review",&[duplicate])?;
        let other=ClipItem {preview:"other".into(),text:Some("other".into()),..item.clone()};
        db_insert_item(0,&other,Some("other-signature"))?;
        assert!(state.add_clip_item(item,"manual-tag-capture".into()));
        let rows=db_load_items_page(&query,None,10)?.0;
        assert_eq!(rows.len(),1);
        assert_eq!(crate::clip_tags::counts(&[rows[0].id])?.len(),2);
        Ok(())
    }).unwrap();
}

//! Real window/timer coverage of hidden reclamation; no absolute memory-size target.
use super::prelude::*;
use crate::platform::string::to_wide;
use std::time::{Duration, Instant};
use windows_sys::Win32::UI::WindowsAndMessaging::{PeekMessageW, PM_REMOVE};

unsafe extern "system" fn host_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if matches!(msg, WM_TIMER | WM_IMAGE_THUMB_READY | WM_IMAGE_PASTE_READY | WM_VV_SHOW | WM_VV_HIDE) {
        return super::main_entry::wnd_proc(hwnd, msg, wp, lp);
    }
    platform_window::default_window_proc(hwnd, msg, wp, lp)
}

unsafe extern "system" fn settings_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if matches!(msg, WM_CLOSE | WM_DESTROY) || (msg == WM_SIZE && wp == SIZE_MINIMIZED as WPARAM) {
        return super::settings_window::settings_wnd_proc(hwnd, msg, wp, lp);
    }
    platform_window::default_window_proc(hwnd, msg, wp, lp)
}

unsafe fn pump_for(duration: Duration) {
    let deadline = Instant::now() + duration;
    while Instant::now() < deadline {
        let mut message: MSG = std::mem::zeroed();
        while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
            platform_window::translate_message(&message);
            platform_window::dispatch_message(&message);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn cached_item() -> ClipItem {
    ClipItem {
        id: 91001,
        kind: ClipKind::Text,
        preview: "Synthetic memory fixture".into(),
        phrase_title: String::new(),
        text: Some("Synthetic memory fixture body".into()),
        rich_text_html: None,
        source_app: "Memory fixture".into(),
        file_paths: None,
        image_bytes: None,
        image_path: None,
        image_width: 0,
        image_height: 0,
        pinned: false,
        group_id: 0,
        created_at: String::new(),
    }
}

unsafe fn create_window(class_name: &str, procedure: WNDPROC, style: u32) -> HWND {
    let class = to_wide(class_name);
    let definition = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: procedure,
        hInstance: platform_window::module_handle(),
        lpszClassName: class.as_ptr(),
        ..std::mem::zeroed()
    };
    platform_window::register_class_ex(&definition);
    let hwnd = platform_window::create_window_ex(
        WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
        class.as_ptr(),
        to_wide("Memory lifecycle fixture").as_ptr(),
        style,
        80,
        80,
        220,
        110,
        null_mut(),
        null_mut(),
        platform_window::module_handle(),
        null(),
    );
    assert!(!hwnd.is_null());
    hwnd
}

unsafe fn new_state(role: WindowRole, hwnd: HWND) -> Box<AppState> {
    let mut state = Box::new(AppState::new(
        role,
        hwnd,
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
    ));
    state.settings = AppSettings::default();
    state.settings.hotkey_enabled = false;
    state.settings.vv_mode_enabled = false;
    state.settings.mouse_side_button_enabled = false;
    state.settings.auto_hide_on_blur = false;
    state.settings.lan_sync_enabled = false;
    state.settings.cloud_sync_enabled = false;
    state.settings.copy_success_sound_enabled = false;
    state.settings.paste_success_sound_enabled = false;
    platform_window::set_user_data(hwnd, (&mut *state as *mut AppState) as isize);
    set_window_host(role, hwnd);
    state
}

struct Fixture {
    main: HWND,
    quick: HWND,
    settings: HWND,
    main_state: Box<AppState>,
    _quick_state: Box<AppState>,
}
impl Fixture {
    unsafe fn new() -> Self {
        let main = create_window("ZSClipMemoryMainFixture", Some(host_proc), WS_POPUP);
        let quick = create_window("ZSClipMemoryQuickFixture", Some(host_proc), WS_POPUP);
        let main_state = new_state(WindowRole::Main, main);
        let quick_state = new_state(WindowRole::Quick, quick);
        Self {
            main,
            quick,
            settings: null_mut(),
            main_state,
            _quick_state: quick_state,
        }
    }
    unsafe fn settings(&mut self) {
        assert!(self.settings.is_null());
        self.settings = create_window(
            "ZSClipMemorySettingsFixture",
            Some(settings_proc),
            WS_OVERLAPPEDWINDOW,
        );
        let state = Box::new(SettingsWndState::new(
            self.main,
            96,
            null_mut(),
            null_mut(),
            null_mut(),
        ));
        platform_window::set_user_data(self.settings, Box::into_raw(state) as isize);
        self.main_state.settings_hwnd = self.settings;
        platform_window::show_no_activate(self.settings);
        assert!(platform_window::is_visible(self.settings));
    }
    fn seed_cache(&mut self) {
        self.main_state.payload_cache.put(&cached_item());
        self.main_state.image_thumb_cache.put(
            91001,
            crate::app_core::ImageThumbnail {
                bytes: vec![255, 0, 0, 255],
                width: 1,
                height: 1,
            },
        );
    }
    fn assert_cache_present(&mut self) {
        assert!(self.main_state.payload_cache.get(91001).is_some());
        assert!(self.main_state.image_thumb_cache.get(91001).is_some());
    }
    unsafe fn show_preview(&self) {
        show_hover_preview(&cached_item(), 320, 220, 12);
        assert!(crate::hover_preview::hover_preview_is_showing(91001, 12));
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe {
            cancel_queued_paste_attempt(self.main, &mut self.main_state);
            cancel_queued_paste_attempt(self.quick, &mut self._quick_state);
            vv_finish_paste(&mut self.main_state);
            vv_finish_paste(&mut self._quick_state);
            vv_popup_hide(self.main, &mut self.main_state);
            super::vv_preview::destroy_vv_preview();
            hide_hover_preview();
            release_hover_preview_memory();
            if platform_window::exists(self.settings) {
                platform_window::destroy(self.settings);
            }
            cancel_hidden_memory_reclaim(self.main, &mut self.main_state);
            cancel_hidden_memory_reclaim(self.quick, &mut self._quick_state);
            shutdown_low_level_input_hooks();
            clear_window_host(WindowRole::Main, self.main);
            clear_window_host(WindowRole::Quick, self.quick);
            platform_window::set_user_data(self.main, 0);
            platform_window::set_user_data(self.quick, 0);
            platform_window::destroy(self.quick);
            platform_window::destroy(self.main);
        }
    }
}

#[test]
#[ignore = "Requires a fresh isolated profile and desktop; briefly shows nonactivating fixture windows and observes real reclamation timers"]
fn hidden_reclaim_preserves_visible_surfaces_and_rearms_after_settings_and_late_thumbnail() {
    let profile = std::env::var_os("ZSCLIP_DATA_DIR").expect("Set an isolated ZSCLIP_DATA_DIR");
    assert!(std::path::Path::new(&profile).is_absolute());
    assert!(
        window_host_hwnds().iter().all(|hwnd| hwnd.is_null()),
        "Run alone in a fresh test process"
    );
    crate::db_runtime::with_test_protected_texts(&[], || {
        crate::db_runtime::with_test_db(|| {
            unsafe {
                let mut fixture = Fixture::new();
                fixture.seed_cache();
                platform_window::show_no_activate(fixture.quick);
                fixture.show_preview();
                schedule_hidden_memory_reclaim(fixture.main, &mut fixture.main_state);
                pump_for(Duration::from_millis(950));
                assert!(
                    !fixture.main_state.hidden_reclaim_timer,
                    "A visible Quick host defers until its lifecycle changes"
                );
                fixture.assert_cache_present();
                assert!(
                    crate::hover_preview::hover_preview_is_showing(91001, 12),
                    "Hidden Main must not erase Quick's preview"
                );

                platform_window::hide(fixture.quick);
                fixture.settings();
                schedule_hidden_memory_reclaim(fixture.main, &mut fixture.main_state);
                pump_for(Duration::from_millis(950));
                assert!(
                    !fixture.main_state.hidden_reclaim_timer,
                    "Settings stays visible without a repeating reclaim timer"
                );
                fixture.assert_cache_present();
                assert!(crate::hover_preview::hover_preview_is_showing(91001, 12));
                hide_hover_preview();
                platform_window::send_message(fixture.settings, WM_CLOSE, 0, 0);
                fixture.settings = null_mut();
                assert!(fixture.main_state.settings_hwnd.is_null());
                assert!(
                    fixture.main_state.hidden_reclaim_timer,
                    "Closing settings must rearm an already-hidden Main"
                );
                pump_for(Duration::from_millis(950));
                assert!(!fixture.main_state.hidden_reclaim_timer);
                fixture._quick_state.payload_cache.put(&cached_item());
                windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(fixture.quick,SW_SHOWMINNOACTIVE);
                assert!(platform_window::is_minimized(fixture.quick));
                schedule_hidden_memory_reclaim_after_activity();
                pump_for(Duration::from_millis(950));
                assert!(fixture._quick_state.payload_cache.get(91001).is_none(),
                    "A minimized peer is inactive and must release its payload cache");
                assert!(fixture.main_state.payload_cache.get(91001).is_none());
                assert!(fixture.main_state.image_thumb_cache.get(91001).is_none());

                fixture.seed_cache();
                fixture.settings();
                windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(
                    fixture.settings,
                    SW_SHOWMINNOACTIVE,
                );
                assert!(platform_window::is_minimized(fixture.settings));
                assert!(
                    fixture.main_state.hidden_reclaim_timer,
                    "Minimizing settings must also rearm hidden reclamation"
                );
                pump_for(Duration::from_millis(950));
                assert!(fixture.main_state.payload_cache.get(91001).is_none());
                platform_window::send_message(fixture.settings, WM_CLOSE, 0, 0);
                fixture.settings = null_mut();
                cancel_hidden_memory_reclaim(fixture.main, &mut fixture.main_state);

                fixture.seed_cache();
                fixture.show_preview();
                schedule_hidden_memory_reclaim(fixture.main, &mut fixture.main_state);
                pump_for(Duration::from_millis(950));
                assert!(
                    fixture.main_state.hidden_reclaim_timer,
                    "An unregistered visible transient keeps the conservative retry"
                );
                fixture.assert_cache_present();
                assert!(crate::hover_preview::hover_preview_is_showing(91001, 12));
                hide_hover_preview();
                schedule_hidden_memory_reclaim_after_activity();
                pump_for(Duration::from_millis(950));
                assert!(!fixture.main_state.hidden_reclaim_timer);
                assert!(fixture.main_state.image_thumb_cache.get(91001).is_none());

                fixture.main_state.image_thumb_loading.insert(91002);
                let payload = Box::new(crate::app_core::ImageThumbReadyResult {
                    item_id: 91002,
                    app_data_generation: fixture.main_state.app_data_generation,
                    image: Some(crate::app_core::ImageThumbnail {
                        bytes: vec![0, 255, 0, 255],
                        width: 1,
                        height: 1,
                    }),
                });
                assert!(post_boxed_message(
                    fixture.main as isize,
                    WM_IMAGE_THUMB_READY,
                    0,
                    payload
                ));
                pump_for(Duration::from_millis(30));
                assert!(!fixture.main_state.image_thumb_loading.contains(&91002));
                assert!(
                    fixture.main_state.image_thumb_cache.get(91002).is_none(),
                    "Late hidden thumbnail must not refill released cache"
                );
                assert!(
                    fixture.main_state.hidden_reclaim_timer,
                    "Dropping a late payload must schedule one quiet-period reclaim"
                );
                pump_for(Duration::from_millis(950));
                assert!(!fixture.main_state.hidden_reclaim_timer);
            }
            Ok(())
        })
    })
    .unwrap();
}

struct ControlledTarget {
    hwnd: HWND,
    previous_foreground: HWND,
}

impl ControlledTarget {
    unsafe fn new() -> Self {
        let previous_foreground = platform_window::foreground();
        let hwnd = platform_window::create_window_ex(
            WS_EX_TOOLWINDOW,
            to_wide("EDIT").as_ptr(),
            to_wide("Synthetic memory target").as_ptr(),
            WS_OVERLAPPEDWINDOW | ES_MULTILINE as u32,
            120, 120, 320, 180,
            null_mut(), null_mut(), platform_window::module_handle(), null(),
        );
        assert!(!hwnd.is_null());
        let target = Self { hwnd, previous_foreground };
        platform_window::show(hwnd);
        assert!(platform_window::try_set_foreground(hwnd), "Controlled target must receive foreground without injected input");
        platform_input::set_focus(hwnd);
        pump_for(Duration::from_millis(30));
        assert_eq!(platform_window::foreground(), hwnd);
        target
    }
}

impl Drop for ControlledTarget {
    fn drop(&mut self) {
        platform_window::destroy(self.hwnd);
        if platform_window::exists(self.previous_foreground) {
            platform_window::set_foreground(self.previous_foreground);
        }
    }
}

#[test]
#[ignore = "Requires a fresh isolated profile and desktop; shows an owned synthetic receiver and VV popup without clipboard reads, writes or paste injection"]
fn tray_vv_lifecycle_reclaims_only_after_paste_and_late_results_finish() {
    let profile = std::env::var_os("ZSCLIP_DATA_DIR").expect("Set an isolated ZSCLIP_DATA_DIR");
    assert!(std::path::Path::new(&profile).is_absolute());
    assert!(window_host_hwnds().iter().all(|hwnd| hwnd.is_null()), "Run alone in a fresh process");
    crate::db_runtime::with_test_protected_texts(&[], || crate::db_runtime::with_test_db(|| {
        crate::platform::ime::with_test_ime_observation(
            WindowsImeInputMode::Unknown, false, || unsafe {
                crate::db_runtime::insert_native_clipboard_text(0, "Synthetic tray VV item", "Memory fixture").unwrap();
                let mut fixture = Fixture::new();
                let target = ControlledTarget::new();
                fixture.main_state.settings.vv_mode_enabled = true;
                fixture.main_state.settings.click_hide = false;
                fixture.main_state.settings.vv_source_tab = 0;
                fixture.main_state.settings.vv_group_id = 0;

                // Use the real non-text WM_VV_SHOW entry and Show timer while
                // Main/Quick remain hidden, as for a tray-launched candidate list.
                platform_window::post_hwnd_message(fixture.main, WM_VV_SHOW, target.hwnd as usize, 0);
                let show_deadline = Instant::now() + Duration::from_secs(2);
                while !fixture.main_state.vv_popup_visible && Instant::now() < show_deadline {
                    pump_for(Duration::from_millis(10));
                }
                assert!(fixture.main_state.vv_popup_visible);
                assert!(platform_window::is_visible(current_vv_popup_hwnd()));
                assert!(!platform_window::is_visible(fixture.main));
                assert!(!platform_window::is_visible(fixture.quick));
                fixture.seed_cache();
                cancel_hidden_memory_reclaim(fixture.main, &mut fixture.main_state);
                platform_window::post_hwnd_message(fixture.main, WM_VV_HIDE, 0,
                    fixture.main_state.vv_popup_session_id as isize);
                pump_for(Duration::from_millis(30));
                assert!(!fixture.main_state.vv_popup_visible);
                assert!(!platform_window::is_visible(current_vv_popup_hwnd()));
                assert!(fixture.main_state.hidden_reclaim_timer, "Closing VV from the tray must rearm reclamation");
                platform_window::hide(target.hwnd);
                pump_for(Duration::from_millis(950));
                assert!(!fixture.main_state.hidden_reclaim_timer);
                assert!(fixture.main_state.payload_cache.get(91001).is_none());

                // A queued paste outlives the reclaim deadline. The deliberately
                // stale target guard makes the real Paste timer take its failure
                // exit before reading the clipboard or injecting any input.
                fixture.seed_cache();
                fixture.main_state.vv_paste_guard = Some((fixture.main_state.vv_popup_session_id, 0, 0));
                fixture.main_state.paste_target_override = target.hwnd;
                timer::start(fixture.main, ID_TIMER_PASTE, 2_000);
                schedule_hidden_memory_reclaim(fixture.main, &mut fixture.main_state);
                pump_for(Duration::from_millis(950));
                assert!(!fixture.main_state.hidden_reclaim_timer, "Pending paste waits for completion rather than polling trim");
                fixture.assert_cache_present();
                assert_eq!(trim_hidden_process_working_set(), HiddenWorkingSetTrimResult::PastePending);
                vv_finish_paste(&mut fixture.main_state);
                assert!(!fixture.main_state.hidden_reclaim_timer, "The queued target still blocks reclaim after the VV guard finishes");
                assert_eq!(trim_hidden_process_working_set(), HiddenWorkingSetTrimResult::PastePending);
                // Restore the stale guard before dispatching the failure timer;
                // this fixture never authorizes a real paste into any window.
                fixture.main_state.vv_paste_guard = Some((fixture.main_state.vv_popup_session_id, 0, 0));
                timer::start(fixture.main, ID_TIMER_PASTE, 20);
                pump_for(Duration::from_millis(60));
                assert!(fixture.main_state.vv_paste_guard.is_none());
                assert!(fixture.main_state.paste_target_override.is_null());
                assert!(fixture.main_state.hidden_reclaim_timer, "The real paste failure exit must rearm reclamation");
                fixture.assert_cache_present();
                pump_for(Duration::from_millis(950));
                assert!(fixture.main_state.payload_cache.get(91001).is_none());

                // A pending image on the peer host also protects Main's shared
                // resources until the actual posted completion has been dropped.
                fixture.seed_cache();
                fixture._quick_state.pending_image_paste_generation = Some(77);
                schedule_hidden_memory_reclaim(fixture.main, &mut fixture.main_state);
                pump_for(Duration::from_millis(950));
                fixture.assert_cache_present();
                assert!(!fixture.main_state.hidden_reclaim_timer);
                vv_finish_paste(&mut fixture._quick_state);
                assert!(!fixture.main_state.hidden_reclaim_timer, "A completion notification cannot bypass a pending image");
                let payload = Box::new(ImagePasteReadyResult {
                    image: Some((vec![255, 0, 0, 255], 1, 1)),
                    generation: 77,
                    app_data_generation: fixture._quick_state.app_data_generation.wrapping_add(1),
                    item_id: 91003,
                    context: ImagePasteRequestContext::VvPopup,
                    target: NativeWindowToken(target.hwnd as usize),
                    hide_main: false,
                    backspaces: 0,
                    completion: MainPasteCompletionPlan {
                        promote_item_id: None, reset_plain_text_paste_mode: false,
                        clear_selection: false, clear_hover: false, hide_main_now: false,
                        play_success_sound: false, send_paste_after_clipboard: false,
                        paste_hide_main: false, paste_backspaces: 0,
                    },
                });
                assert!(post_boxed_message(fixture.quick as isize, WM_IMAGE_PASTE_READY, 0, payload));
                pump_for(Duration::from_millis(30));
                assert!(fixture._quick_state.pending_image_paste_generation.is_none());
                assert!(fixture.main_state.hidden_reclaim_timer);
                fixture.assert_cache_present();
                pump_for(Duration::from_millis(950));
                assert!(fixture.main_state.image_thumb_cache.get(91001).is_none());

                assert!(!fixture.main_state.hidden_reclaim_timer);
                assert!(super::vv_preview::post_stale_vv_preview_for_reclaim_test(fixture.main));
                pump_for(Duration::from_millis(30));
                assert!(fixture.main_state.hidden_reclaim_timer, "Dropping a late VV preview must restart the quiet period");
                assert!(!fixture.main_state.vv_popup_visible);
                pump_for(Duration::from_millis(950));
                assert!(!fixture.main_state.hidden_reclaim_timer);
            },
        );
        Ok(())
    })).unwrap();
}

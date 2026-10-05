//! Desktop integration coverage for candidate selection and deferred paste delivery.
//! Physical VV detection is outside this fixture: it starts an authorized session.

use super::prelude::*;
use crate::app_core::vv_session::VvPhase;
use crate::platform::ime::{with_test_ime_observation, WindowsImeInputMode};
use crate::platform::string::to_wide;
use std::time::{Duration, Instant};
use windows_sys::Win32::UI::WindowsAndMessaging::{PeekMessageW, PM_REMOVE};

const PAYLOAD: &str = "VV-INTEGRATION-PAYLOAD";

unsafe fn pump_messages() {
    let mut message: MSG = std::mem::zeroed();
    while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
        platform_window::translate_message(&message);
        platform_window::dispatch_message(&message);
    }
}

struct DesktopFixture {
    original_foreground: HWND,
    owner: HWND,
    target: HWND,
    edit: HWND,
    app: Box<AppState>,
}

impl DesktopFixture {
    unsafe fn new() -> Self {
        let original_foreground = platform_window::foreground();
        let owner = platform_window::create_window_ex(0, to_wide("STATIC").as_ptr(),
            to_wide("").as_ptr(), WS_POPUP, 0, 0, 1, 1, null_mut(), null_mut(),
            platform_window::module_handle(), null());
        assert!(!owner.is_null(), "Could not create isolated AppState owner");
        let mut app = Box::new(AppState::new(WindowRole::Main, owner, null_mut(), Icons {
            app: 0, search: 0, setting: 0, min: 0, close: 0, text: 0,
            image: 0, file: 0, folder: 0, pin: 0, del: 0,
        }, None));
        app.settings = AppSettings::default();
        app.settings.click_hide = false;
        app.settings.move_pasted_item_to_top = false;
        app.settings.copy_success_sound_enabled = false;
        app.settings.paste_success_sound_enabled = false;
        app.settings.ai_clean_enabled = false;
        app.settings.vv_source_tab = 0;
        app.settings.vv_group_id = 0;
        app.search_on = false;
        app.ui_dpi = 96;
        platform_window::set_user_data(owner, (&mut *app as *mut AppState) as isize);
        let mut fixture = Self { original_foreground, owner, target: null_mut(), edit: null_mut(), app };
        fixture.target = platform_window::create_window_ex(0, to_wide("STATIC").as_ptr(),
            to_wide("ZSClip VV paste integration target").as_ptr(), WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            120, 120, 600, 180, null_mut(), null_mut(), platform_window::module_handle(), null());
        assert!(!fixture.target.is_null(), "Could not create controlled target");
        fixture.edit = platform_window::create_window_ex(0, to_wide("EDIT").as_ptr(),
            to_wide("").as_ptr(), WS_CHILD | WS_VISIBLE | WS_TABSTOP | ES_AUTOHSCROLL as u32,
            16, 24, 540, 32, fixture.target, null_mut(), platform_window::module_handle(), null());
        assert!(!fixture.edit.is_null(), "Could not create controlled EDIT");
        set_window_host(WindowRole::Main, owner);
        assert!(platform_window::force_foreground(fixture.target), "Interactive desktop foreground access is required");
        platform_input::set_focus(fixture.edit);
        pump_messages();
        assert_eq!(platform_window::foreground(), fixture.target);
        assert_eq!(vv_current_focus(fixture.target), fixture.edit);
        fixture
    }

    unsafe fn select_and_paste(&mut self, mode: WindowsImeInputMode, expected_backspaces: u8) {
        eprintln!("VV desktop case: {mode:?}, cleanup backspaces={expected_backspaces}");
        with_test_ime_observation(mode, false, || {
            platform_window::set_text(self.edit, "LEFTvvRIGHT");
            platform_window::send_message(self.edit, 0x00B1, 6, 6); // EM_SETSEL: caret after vv.
            platform_input::set_focus(self.edit);
            assert!(!platform_input::paste_command_modifiers_down(), "Release modifier keys before the desktop test");
            assert!(platform_clipboard::WindowsClipboardHost::write_text("VV-INTEGRATION-SENTINEL"));
            let id = {
                let mut hook = vv_hook_state().lock().unwrap();
                hook.main_hwnd = self.owner as isize;
                hook.popup_menu_active = false;
                hook.session.begin(self.target as usize, self.edit as usize, true)
            };
            self.app.vv_popup_session_id = id;
            self.app.vv_popup_focus = self.edit;
            self.app.vv_popup_pending_target = self.target;
            assert!(vv_popup_show(self.owner, &mut self.app, self.target), "Real candidate popup did not open");
            assert!(platform_window::is_visible(current_vv_popup_hwnd()));
            assert_eq!(self.app.vv_popup_items.len(), 1);
            assert!(vv_hook_state().lock().unwrap().session.select(id, 0));

            // This invokes the real selection, clipboard publication and timer scheduling path.
            handle_vv_select(self.owner, &mut self.app, 0);
            assert!(!self.app.vv_popup_visible);
            assert!(!platform_window::is_visible(current_vv_popup_hwnd()));
            assert_eq!(platform_window::foreground(), self.target, "Hiding the popup changed the foreground target");
            assert_eq!(vv_current_focus(self.target), self.edit, "Hiding the popup changed EDIT focus");
            assert_eq!(self.app.vv_paste_guard, Some((id, self.target as isize, self.edit as isize)));
            assert_eq!(vv_hook_state().lock().unwrap().session.phase, VvPhase::Selected);
            assert!(vv_paste_target_is_current(&self.app), "Selected guard must survive popup hiding");
            assert_eq!(self.app.paste_target_override, self.target);
            assert_eq!(self.app.paste_backspace_count, expected_backspaces);
            assert!(self.app.pending_paste_completion.is_some());
            let deadline = Instant::now() + Duration::from_millis(500);
            let mut published = platform_clipboard::WindowsClipboardHost::read_text();
            while published.is_none() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(10));
                published = platform_clipboard::WindowsClipboardHost::read_text();
            }
            assert!(published.as_deref() == Some(PAYLOAD), "Synthetic candidate was not published to the clipboard");

            handle_main_timer_task(self.owner, MainTimerTask::Paste);
            let expected = if expected_backspaces == 0 {
                format!("LEFTvv{PAYLOAD}RIGHT")
            } else {
                format!("LEFT{PAYLOAD}RIGHT")
            };
            let deadline = Instant::now() + Duration::from_secs(2);
            while Instant::now() < deadline {
                pump_messages();
                if platform_window::text(self.edit) == expected { break; }
                std::thread::sleep(Duration::from_millis(10));
            }
            let actual = platform_window::text(self.edit);
            assert!(actual == expected, "Controlled EDIT paste mismatch: mode={mode:?}, actual_length={}, expected_length={}, has_payload={}, intact_before={}, intact_after={}", actual.len(), expected.len(), actual.contains(PAYLOAD), actual.starts_with("LEFT"), actual.ends_with("RIGHT"));
            assert!(self.app.vv_paste_guard.is_none());
            assert!(self.app.pending_paste_completion.is_none());
            assert_eq!(vv_hook_state().lock().unwrap().session.phase, VvPhase::Cancelled);
        });
    }

    unsafe fn failed_selection_and_stale_image_leave_session_consistent(&mut self) {
        let id = {
            let mut hook = vv_hook_state().lock().unwrap();
            let id = hook.session.begin(self.target as usize, self.edit as usize, true);
            assert!(hook.session.show(id, 1));
            assert!(hook.session.select(id, 0));
            id
        };
        self.app.vv_popup_session_id = id;
        self.app.vv_popup_target = self.target;
        self.app.vv_popup_focus = self.edit;
        self.app.vv_popup_visible = true;
        self.app.vv_popup_protection_revision = crate::db_runtime::search_protection_revision().ok();
        with_test_ime_observation(WindowsImeInputMode::Unknown, false, || {
            handle_vv_select(self.owner, &mut self.app, 99);
        });
        assert_eq!(vv_hook_state().lock().unwrap().session.phase, VvPhase::Cancelled);
        assert!(!self.app.vv_popup_visible);

        let id = {
            let mut hook = vv_hook_state().lock().unwrap();
            let id = hook.session.begin(self.target as usize, self.edit as usize, true);
            assert!(hook.session.show(id, 1));
            assert!(hook.session.select(id, 0));
            id
        };
        self.app.vv_popup_session_id = id;
        let guard = Some((id, self.target as isize, 0));
        self.app.vv_paste_guard = guard;
        self.app.pending_image_paste_generation = Some(20);
        let completion = main_paste_completion_plan(MainPasteCompletionKind::VvAsyncImage,
            paste_completion_input(&self.app, 1));
        handle_main_async_event(self.owner, MainAsyncEvent::ImagePaste(ImagePasteReadyResult {
            image: None, generation: 19, app_data_generation: self.app.app_data_generation,
            item_id: 1, context: ImagePasteRequestContext::VvPopup,
            target: NativeWindowToken(self.target as usize), hide_main: false, backspaces: 0, completion,
        }));
        assert_eq!(self.app.pending_image_paste_generation, Some(20));
        assert_eq!(self.app.vv_paste_guard, guard);
        assert_eq!(vv_hook_state().lock().unwrap().session.phase, VvPhase::Selected);
    }
}

impl Drop for DesktopFixture {
    fn drop(&mut self) {
        unsafe {
            cancel_queued_paste_attempt(self.owner, &mut self.app);
            vv_finish_paste(&mut self.app);
            vv_popup_hide(self.owner, &mut self.app);
            let popup = current_vv_popup_hwnd();
            if platform_window::exists(popup) {
                platform_window::set_user_data(popup, 0);
                platform_window::destroy(popup);
            }
            clear_window_host(WindowRole::Main, self.owner);
            platform_window::set_user_data(self.owner, 0);
            if platform_window::exists(self.target) { platform_window::destroy(self.target); }
            platform_window::destroy(self.owner);
            if platform_window::exists(self.original_foreground) {
                platform_window::set_foreground(self.original_foreground);
            }
        }
    }
}

#[test]
#[ignore = "Requires an interactive desktop and isolated ZSCLIP_DATA_DIR; writes synthetic clipboard data and temporarily changes focus; run alone in a fresh test process"]
fn vv_selection_reaches_clipboard_and_real_edit_through_paste_timer() {
    let profile = std::env::var_os("ZSCLIP_DATA_DIR").expect("Set an isolated test profile with ZSCLIP_DATA_DIR");
    assert!(std::path::Path::new(&profile).is_absolute());
    assert!(window_host_hwnds().iter().all(|handle| handle.is_null()), "Run this desktop fixture in a fresh test process");
    assert!(!platform_window::exists(current_vv_popup_hwnd()), "A different VV popup is already using the fixture process");
    crate::db_runtime::with_test_protected_texts(&[], || crate::db_runtime::with_test_db(|| {
        let item = ClipItem { id: 0, kind: ClipKind::Text, preview: PAYLOAD.into(), phrase_title: String::new(),
            text: Some(PAYLOAD.into()), rich_text_html: None, source_app: "VV integration fixture".into(),
            file_paths: None, image_bytes: None, image_path: None, image_width: 0, image_height: 0,
            pinned: false, group_id: 0, created_at: String::new() };
        db_insert_item(0, &item, None)?;
        unsafe {
            let mut fixture = DesktopFixture::new();
            fixture.select_and_paste(WindowsImeInputMode::Unknown, 0);
            fixture.select_and_paste(WindowsImeInputMode::Native, 0);
            fixture.select_and_paste(WindowsImeInputMode::Alphanumeric, 2);
            fixture.failed_selection_and_stale_image_leave_session_consistent();
        }
        Ok(())
    })).unwrap();
}

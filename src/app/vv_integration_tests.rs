//! Desktop integration coverage for the production hook callback, posted messages and timers.
//! Callback input is supplied as flags=0 hook records; hardware hook delivery and TIM's IME are separate checks.

use super::prelude::*;
use crate::app_core::vv_session::VvPhase;
use crate::platform::ime::{with_test_ime_observation, WindowsImeInputMode};
use crate::platform::string::to_wide;
use std::time::{Duration, Instant};
use windows_sys::Win32::UI::WindowsAndMessaging::{PeekMessageW, PM_REMOVE, GetClassInfoExW, KBDLLHOOKSTRUCT, MSLLHOOKSTRUCT, WNDCLASSEXW};

const PAYLOAD: &str = "VV-INTEGRATION-PAYLOAD";
const RECEIVER_FOCUS: u32 = WM_APP + 113;
const RECEIVER_EVENT_COUNT: u32 = WM_APP + 114;
static RECEIVER_INPUT_COUNTS: [std::sync::atomic::AtomicUsize;4] = [
    std::sync::atomic::AtomicUsize::new(0), std::sync::atomic::AtomicUsize::new(0),
    std::sync::atomic::AtomicUsize::new(0), std::sync::atomic::AtomicUsize::new(0),
];
static RECEIVER_EDIT_PROC: OnceLock<windows_sys::Win32::UI::WindowsAndMessaging::WNDPROC> = OnceLock::new();
static IMAGE_RECEIVER: OnceLock<std::path::PathBuf> = OnceLock::new();
static IMAGE_CTRL_V_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static IMAGE_WM_PASTE_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

unsafe extern "system" fn owner_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if matches!(message, WM_VV_SHOW | WM_VV_HIDE | WM_VV_SELECT | WM_TIMER | WM_IMAGE_PASTE_READY) {
        return super::main_entry::wnd_proc(hwnd, message, wparam, lparam);
    }
    platform_window::default_window_proc(hwnd, message, wparam, lparam)
}

unsafe fn pump_messages() {
    let mut message: MSG = std::mem::zeroed();
    while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
        platform_window::translate_message(&message);
        platform_window::dispatch_message(&message);
    }
}

unsafe fn pump_vv_requests_without_timers(owner: HWND) {
    let mut message:MSG=std::mem::zeroed();
    while PeekMessageW(&mut message,owner,WM_VV_SHOW,WM_VV_SELECT,PM_REMOVE)!=0 {
        platform_window::translate_message(&message);
        platform_window::dispatch_message(&message);
    }
}

unsafe fn pump_for(duration:Duration) {
    let deadline=Instant::now()+duration;
    while Instant::now()<deadline {pump_messages();std::thread::sleep(Duration::from_millis(5));}
}

fn clipboard_header(format: u32, length: usize) -> Option<Vec<u8>> {
    let handle=platform_clipboard::data_handle(format);
    if handle.is_null() || crate::platform::memory::global_size(handle)<length {return None;}
    let pointer=crate::platform::memory::global_lock(handle);
    if pointer.is_null() {return None;}
    let bytes=unsafe {std::slice::from_raw_parts(pointer.cast::<u8>(),length).to_vec()};
    crate::platform::memory::global_unlock(handle);
    Some(bytes)
}

unsafe fn record_synthetic_image_paste(control: HWND, ctrl_v: bool) {
    use std::sync::atomic::Ordering;
    let Some(receipt)=IMAGE_RECEIVER.get() else {return;};
    if ctrl_v {IMAGE_CTRL_V_COUNT.fetch_add(1,Ordering::SeqCst);} else {IMAGE_WM_PASTE_COUNT.fetch_add(1,Ordering::SeqCst);}
    let mut dib=None;
    let mut png=None;
    if ctrl_v && platform_clipboard::open(control) {
        // Only format dimensions are inspected: no image pixels are read or recorded.
        if let Some(header)=clipboard_header(platform_clipboard::CF_DIBV5,12) {
            let size=u32::from_le_bytes(header[0..4].try_into().unwrap());
            let width=i32::from_le_bytes(header[4..8].try_into().unwrap());
            let height=i32::from_le_bytes(header[8..12].try_into().unwrap());
            if size>=124&&width>0 {dib=Some((width as u32,height.unsigned_abs()));}
        }
        let png_format=windows_sys::Win32::System::DataExchange::RegisterClipboardFormatW(to_wide("PNG").as_ptr());
        if let Some(header)=clipboard_header(png_format,24) {
            if &header[..8]==b"\x89PNG\r\n\x1a\n"&&&header[12..16]==b"IHDR" {
                png=Some((u32::from_be_bytes(header[16..20].try_into().unwrap()),u32::from_be_bytes(header[20..24].try_into().unwrap())));
            }
        }
        platform_clipboard::close();
    }
    let result=serde_json::json!({"pid":std::process::id(),"dib_dimensions":dib,"png_dimensions":png,
        "ctrl_v_count":IMAGE_CTRL_V_COUNT.load(Ordering::SeqCst),"wm_paste_count":IMAGE_WM_PASTE_COUNT.load(Ordering::SeqCst)});
    let _=std::fs::write(receipt,serde_json::to_vec(&result).unwrap());
}

unsafe extern "system" fn image_receiver_edit_proc(hwnd: HWND,msg: u32,wp: WPARAM,lp: LPARAM)->LRESULT {
    use std::sync::atomic::Ordering;
    if msg==WM_KEYDOWN&&wp==0x1b {RECEIVER_INPUT_COUNTS[0].fetch_add(1,Ordering::SeqCst);}
    if msg==WM_KEYDOWN&&wp==0x08 {RECEIVER_INPUT_COUNTS[1].fetch_add(1,Ordering::SeqCst);}
    if msg==WM_PASTE {
        RECEIVER_INPUT_COUNTS[3].fetch_add(1,Ordering::SeqCst);
        if IMAGE_RECEIVER.get().is_some() {record_synthetic_image_paste(hwnd,false);return 0;}
    }
    // TranslateMessage may have queued Ctrl+V's character before WM_KEYDOWN
    // was dispatched. Do not let EDIT translate it into a second WM_PASTE.
    if msg==WM_CHAR&&wp==0x16&&IMAGE_RECEIVER.get().is_some() {return 0;}
    if msg==WM_KEYDOWN&&wp==0x56
        && windows_sys::Win32::UI::Input::KeyboardAndMouse::GetKeyState(0x11)<0 {
        RECEIVER_INPUT_COUNTS[2].fetch_add(1,Ordering::SeqCst);
        if IMAGE_RECEIVER.get().is_some() {record_synthetic_image_paste(hwnd,true);return 0;}
    }
    windows_sys::Win32::UI::WindowsAndMessaging::CallWindowProcW(*RECEIVER_EDIT_PROC.get().unwrap(),hwnd,msg,wp,lp)
}

struct DesktopFixture {
    original_foreground: HWND,
    owner: HWND,
    target: HWND,
    edit: HWND,
    app: Box<AppState>,
    receiver: Option<std::process::Child>,
    image_receipt: std::path::PathBuf,
}

impl DesktopFixture {
    unsafe fn new() -> Self {
        Self::new_with_receiver_mode("text")
    }

    unsafe fn new_with_receiver_mode(mode: &str) -> Self {
        let original_foreground = platform_window::foreground();
        let owner_class = to_wide("ZSClipVvSelectionTestOwner");
        let owner_type = WNDCLASSEXW {cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(owner_proc), hInstance: platform_window::module_handle(),
            lpszClassName: owner_class.as_ptr(), ..std::mem::zeroed()};
        platform_window::register_class_ex(&owner_type);
        let owner = platform_window::create_window_ex(0, owner_class.as_ptr(),
            to_wide("").as_ptr(), WS_POPUP, 0, 0, 1, 1, null_mut(), null_mut(),
            platform_window::module_handle(), null());
        assert!(!owner.is_null(), "Could not create isolated AppState owner");
        let mut app = Box::new(AppState::new(WindowRole::Main, owner, null_mut(), Icons {
            app: 0, search: 0, setting: 0, min: 0, close: 0, text: 0,
            image: 0, file: 0, folder: 0, pin: 0, del: 0,
        }, None));
        app.settings = AppSettings::default();
        app.settings.hotkey_enabled = false;
        app.settings.vv_mode_enabled = true;
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
        let mut fixture = Self { original_foreground, owner, target: null_mut(), edit: null_mut(), app, receiver: None, image_receipt: std::path::PathBuf::new() };
        let profile=std::path::PathBuf::from(std::env::var_os("ZSCLIP_DATA_DIR").unwrap());
        std::fs::create_dir_all(&profile).unwrap();
        let token=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let receipt=profile.join(format!("vv-receiver-{}-{token}.json",std::process::id()));
        fixture.image_receipt=profile.join(format!("vv-image-received-{}-{token}.json",std::process::id()));
        let log=profile.join(format!("vv-receiver-{}-{token}.log",std::process::id()));
        fixture.receiver=Some(std::process::Command::new(std::env::current_exe().unwrap())
            .args(["app::vv_integration_tests::vv_controlled_external_receiver","--ignored","--exact","--test-threads=1"])
            .env("ZSCLIP_VV_TEST_RECEIVER_RECEIPT",&receipt)
            .env("ZSCLIP_VV_TEST_RECEIVER_MODE",mode)
            .env("ZSCLIP_VV_TEST_IMAGE_RECEIPT",&fixture.image_receipt)
            .stdout(std::process::Stdio::null()).stderr(std::fs::File::create(log).unwrap()).spawn().unwrap());
        let deadline=Instant::now()+Duration::from_secs(5);
        while Instant::now()<deadline {
            if let Ok(bytes)=std::fs::read(&receipt) {
                if let Ok(value)=serde_json::from_slice::<serde_json::Value>(&bytes) {
                    if value["pid"].as_u64()==Some(fixture.receiver.as_ref().unwrap().id() as u64) {
                        fixture.target=value["target"].as_u64().unwrap() as HWND;
                        fixture.edit=value["edit"].as_u64().unwrap() as HWND;
                        break;
                    }
                }
            }
            assert!(fixture.receiver.as_mut().unwrap().try_wait().unwrap().is_none(),"Controlled receiver exited before readiness");
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(!fixture.target.is_null() && !fixture.edit.is_null(),"Controlled external receiver did not become ready");
        assert_eq!(platform_window::window_process_id(fixture.target),fixture.receiver.as_ref().unwrap().id());
        assert_ne!(platform_window::window_process_id(fixture.target),std::process::id(),"Real VV hook must never bypass self-target rejection");
        set_window_host(WindowRole::Main, owner);
        assert!(platform_window::force_foreground(fixture.target), "Interactive desktop foreground access is required");
        platform_window::send_message(fixture.target,RECEIVER_FOCUS,fixture.edit as WPARAM,0);
        assert!(update_vv_mode_hook(owner,true),"Could not install the real keyboard hook");
        pump_messages();
        assert_eq!(platform_window::foreground(), fixture.target);
        assert_eq!(vv_current_focus(fixture.target), fixture.edit);
        fixture
    }

    unsafe fn key_callback(&self, vk: u32, down: bool) -> LRESULT {
        let event=KBDLLHOOKSTRUCT {vkCode:vk,scanCode:0,flags:0,time:0,dwExtraInfo:0};
        super::vv_hook::vv_keyboard_hook_proc(0,if down {WM_KEYDOWN}else{WM_KEYUP} as WPARAM,&event as *const _ as LPARAM)
    }

    fn receiver_text(&self) -> String {
        // GetWindowText across processes reads the window caption, not the
        // live contents of an edit control. WM_GETTEXT marshals this buffer.
        let mut text = [0u16; 1024];
        let count = platform_window::send_message_bounded(
            self.edit, WM_GETTEXT, text.len(), text.as_mut_ptr() as LPARAM,
        ).expect("External receiver did not answer WM_GETTEXT");
        assert!((0..text.len() as isize).contains(&count));
        String::from_utf16_lossy(&text[..count as usize])
    }

    fn receiver_input_counts(&self) -> [isize;4] {
        std::array::from_fn(|index|platform_window::send_message_bounded(self.target,RECEIVER_EVENT_COUNT,index,0)
            .expect("Controlled receiver did not answer its input counters"))
    }

    unsafe fn set_receiver_draft(&self) {
        assert!(platform_window::force_foreground(self.target));
        platform_window::send_message(self.target,RECEIVER_FOCUS,self.edit as WPARAM,0);
        let text=to_wide("LEFTvvRIGHT");
        assert_eq!(platform_window::send_message_bounded(self.edit,WM_SETTEXT,0,text.as_ptr() as LPARAM),Some(1));
        platform_window::send_message(self.edit,0x00B1,6,6);
        assert_eq!(self.receiver_text(),"LEFTvvRIGHT");
    }

    unsafe fn continue_synthetic_character(&self) {
        // The IME switch is modeled in the parent; this committed character is
        // delivered only to the controlled EDIT, not through any real chat IME.
        self.key_callback(0x41,true);self.key_callback(0x41,false);
        platform_window::send_message(self.edit,WM_CHAR,'续' as WPARAM,0);
        pump_messages();
    }

    unsafe fn trigger_popup(&mut self) -> u64 {
        assert!(platform_window::force_foreground(self.target));
        platform_window::send_message(self.target,RECEIVER_FOCUS,self.edit as WPARAM,0);
        assert_eq!(platform_window::foreground(),self.target);
        assert_eq!(vv_current_focus(self.target),self.edit);
        assert!(!platform_input::paste_command_modifiers_down(),"Release modifier keys before the desktop test");
        self.key_callback(0x56,true);
        self.key_callback(0x56,false);
        self.key_callback(0x56,true);
        self.key_callback(0x56,false);
        let deadline=Instant::now()+Duration::from_secs(2);
        while Instant::now()<deadline && !self.app.vv_popup_visible {
            pump_messages();
            std::thread::sleep(Duration::from_millis(5));
        }
        let hook=vv_hook_state().lock().unwrap();
        assert!(self.app.vv_popup_visible,"Hook-driven VV show did not complete: phase={:?}, sid={}, main={:x}",hook.session.phase,hook.session.id,hook.main_hwnd);
        assert_eq!(hook.session.phase,VvPhase::Visible);
        let id=hook.session.id;
        drop(hook);
        assert_eq!(self.app.vv_popup_session_id,id);
        assert_eq!(platform_window::user_data(current_vv_popup_hwnd()),self.owner as isize);
        assert!(platform_window::is_visible(current_vv_popup_hwnd()));
        id
    }

    unsafe fn select_and_paste(&mut self, mode: WindowsImeInputMode, expected_backspaces: u8, mouse: bool) {
        eprintln!("VV desktop case: {mode:?}, cleanup backspaces={expected_backspaces}, mouse={mouse}, callback=flags0, target=external_process, delivery=SendInput");
        with_test_ime_observation(mode, false, || {
            let initial_text = to_wide("LEFTvvRIGHT");
            assert_eq!(platform_window::send_message_bounded(
                self.edit, WM_SETTEXT, 0, initial_text.as_ptr() as LPARAM,
            ), Some(1));
            assert_eq!(self.receiver_text(), "LEFTvvRIGHT");
            platform_window::send_message(self.edit, 0x00B1, 6, 6); // EM_SETSEL: caret after vv.
            assert_eq!(platform_window::send_message_bounded(self.edit, 0x00B0, 0, 0), Some(6 | (6 << 16)));
            assert!(!platform_input::paste_command_modifiers_down(), "Release modifier keys before the desktop test");
            assert!(platform_clipboard::WindowsClipboardHost::write_text("VV-INTEGRATION-SENTINEL"));
            let id=self.trigger_popup();
            assert_eq!(self.app.vv_popup_items.len(), 1);
            assert_eq!(vv_hook_state().lock().unwrap().session.phase,VvPhase::Visible);
            if mouse {
                let popup = current_vv_popup_hwnd();
                let row = MainVvPopupLayout::default().with_content_font_size(self.app.settings.content_font_size())
                    .scaled(platform_dpi::layout_dpi_for_window(popup)).row_rect(0);
                let x = (row.left + row.right) / 2;
                let y = (row.top + row.bottom) / 2;
                let mut point=POINT {x,y};
                assert!(platform_window::client_to_screen(popup,&mut point));
                let event=MSLLHOOKSTRUCT {pt:point,mouseData:0,flags:0,time:0,dwExtraInfo:0};
                super::main_low_level_input::outside_click_mouse_hook_proc(0,WM_LBUTTONDOWN as WPARAM,&event as *const _ as LPARAM);
                platform_window::send_message(popup, WM_LBUTTONDOWN, 1, (((y as u32) << 16) | (x as u32 & 0xffff)) as LPARAM);
                platform_window::send_message(popup, WM_LBUTTONUP, 0, (((y as u32) << 16) | (x as u32 & 0xffff)) as LPARAM);
            } else {
                assert_eq!(self.key_callback(0x31,true),1,"Numeric selection must be consumed by the production hook");
                assert_eq!(self.key_callback(0x31,false),1,"Owned numeric release must be consumed");
            }
            // Both callbacks post into the real window procedure. WM_TIMER is
            // dispatched by the message pump, never called directly by the fixture.
            pump_messages();
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

            let expected = if expected_backspaces == 0 {
                format!("LEFTvv{PAYLOAD}RIGHT")
            } else {
                format!("LEFT{PAYLOAD}RIGHT")
            };
            let deadline = Instant::now() + Duration::from_secs(2);
            while Instant::now() < deadline {
                pump_messages();
                if self.receiver_text() == expected { break; }
                std::thread::sleep(Duration::from_millis(10));
            }
            let actual = self.receiver_text();
            assert!(actual == expected, "Controlled EDIT paste mismatch: mode={mode:?}, actual_length={}, expected_length={}, has_payload={}, intact_before={}, intact_after={}", actual.len(), expected.len(), actual.contains(PAYLOAD), actual.starts_with("LEFT"), actual.ends_with("RIGHT"));
            assert!(self.app.vv_paste_guard.is_none());
            assert!(self.app.pending_paste_completion.is_none());
            assert_eq!(vv_hook_state().lock().unwrap().session.phase, VvPhase::Cancelled);
        });
    }

    unsafe fn failed_selection_and_stale_image_leave_session_consistent(&mut self) {
        with_test_ime_observation(WindowsImeInputMode::Unknown, false, || {
            let id=self.trigger_popup();
            platform_window::post_hwnd_message(self.owner,WM_VV_SELECT,99,id as LPARAM);
            pump_messages();
            assert_eq!(vv_hook_state().lock().unwrap().session.phase,VvPhase::Cancelled);
            assert!(!self.app.vv_popup_visible);

            self.trigger_popup();
            assert_eq!(self.key_callback(0x31,true),1);
            assert_eq!(self.key_callback(0x31,false),1);
            pump_messages();
            let guard=self.app.vv_paste_guard;
            assert!(guard.is_some(),"Real selection must establish a deferred paste guard");
            self.app.pending_image_paste_generation=Some(20);
            let completion=main_paste_completion_plan(MainPasteCompletionKind::VvAsyncImage,paste_completion_input(&self.app,1));
            handle_main_async_event(self.owner,MainAsyncEvent::ImagePaste(ImagePasteReadyResult {
                image:None,generation:19,app_data_generation:self.app.app_data_generation,
                item_id:1,context:ImagePasteRequestContext::VvPopup,
                target:NativeWindowToken(self.target as usize),hide_main:false,backspaces:0,completion,
            }));
            assert_eq!(self.app.pending_image_paste_generation,Some(20));
            assert_eq!(self.app.vv_paste_guard,guard);
            assert_eq!(vv_hook_state().lock().unwrap().session.phase,VvPhase::Selected);
        });
    }

    unsafe fn focus_change_rejects_selection_and_closes_the_stale_popup(&mut self) {
        with_test_ime_observation(WindowsImeInputMode::Unknown, false, || {
            let id=self.trigger_popup();
            let sequence=platform_clipboard::sequence_number();
            platform_window::send_message(self.target,RECEIVER_FOCUS,self.target as WPARAM,0);
            platform_window::post_hwnd_message(self.owner,WM_VV_SELECT,0,id as LPARAM);
            pump_messages();
            assert!(!self.app.vv_popup_visible,"A rejected selection must not leave an inert popup visible");
            assert!(self.app.vv_paste_guard.is_none());
            assert!(self.app.pending_paste_completion.is_none());
            assert_eq!(sequence,platform_clipboard::sequence_number(),"Focus rejection must not write the clipboard");
            assert_eq!(vv_hook_state().lock().unwrap().session.phase,VvPhase::Cancelled);
            platform_window::send_message(self.target,RECEIVER_FOCUS,self.edit as WPARAM,0);
        });
    }
}

impl Drop for DesktopFixture {
    fn drop(&mut self) {
        unsafe {
            cancel_queued_paste_attempt(self.owner, &mut self.app);
            vv_finish_paste(&mut self.app);
            vv_popup_hide(self.owner, &mut self.app);
            update_vv_mode_hook(self.owner,false);
            let popup = current_vv_popup_hwnd();
            if platform_window::exists(popup) {
                platform_window::set_user_data(popup, 0);
                platform_window::destroy(popup);
            }
            clear_window_host(WindowRole::Main, self.owner);
            platform_window::set_user_data(self.owner, 0);
            if platform_window::exists(self.target) { platform_window::post_hwnd_message(self.target,WM_CLOSE,0,0); }
            if let Some(mut receiver)=self.receiver.take() {
                let deadline=Instant::now()+Duration::from_secs(2);
                while Instant::now()<deadline && receiver.try_wait().ok().flatten().is_none() {std::thread::sleep(Duration::from_millis(10));}
                if receiver.try_wait().ok().flatten().is_none() {let _=receiver.kill();}
                let _=receiver.wait();
            }
            platform_window::destroy(self.owner);
            if platform_window::exists(self.original_foreground) {
                platform_window::set_foreground(self.original_foreground);
            }
        }
    }
}

#[test]
#[ignore = "Launched only as the isolated child process of the VV desktop integration test"]
fn vv_controlled_external_receiver() {
    let receipt=std::env::var_os("ZSCLIP_VV_TEST_RECEIVER_RECEIPT").expect("Receiver must be launched by its fixture parent");
    assert!(std::path::Path::new(&receipt).is_absolute());
    let image_mode=std::env::var("ZSCLIP_VV_TEST_RECEIVER_MODE").as_deref()==Ok("image");
    if image_mode {
        let image_receipt=std::path::PathBuf::from(std::env::var_os("ZSCLIP_VV_TEST_IMAGE_RECEIPT").unwrap());
        assert!(image_receipt.is_absolute());
        IMAGE_RECEIVER.set(image_receipt).unwrap();
    }
    unsafe extern "system" fn receiver_proc(hwnd: HWND,msg: u32,wp: WPARAM,lp: LPARAM)->LRESULT {
        if msg==RECEIVER_EVENT_COUNT {
            return RECEIVER_INPUT_COUNTS.get(wp).map(|count|count.load(std::sync::atomic::Ordering::SeqCst) as LRESULT).unwrap_or(-1);
        }
        if msg==RECEIVER_FOCUS {
            let focus=wp as HWND;
            if focus==hwnd || platform_window::root_ancestor(focus)==hwnd {platform_input::set_focus(focus);}
            return 1;
        }
        if msg==WM_CLOSE || msg==WM_TIMER {
            platform_window::destroy(hwnd);
            platform_window::post_quit_message(0);
            return 0;
        }
        platform_window::default_window_proc(hwnd,msg,wp,lp)
    }
    unsafe {
        let class=to_wide("ZSClipVvExternalTarget");
        let definition=WNDCLASSEXW {cbSize:std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc:Some(receiver_proc),hInstance:platform_window::module_handle(),lpszClassName:class.as_ptr(),..std::mem::zeroed()};
        platform_window::register_class_ex(&definition);
        let target=platform_window::create_window_ex(0,class.as_ptr(),to_wide("ZSClip isolated VV receiver").as_ptr(),
            WS_OVERLAPPEDWINDOW|WS_VISIBLE,120,120,600,180,null_mut(),null_mut(),platform_window::module_handle(),null());
        assert!(!target.is_null());
        let edit_class=to_wide("ZSClipVvInputReceiver");
        let mut edit_definition:WNDCLASSEXW=std::mem::zeroed();
        edit_definition.cbSize=std::mem::size_of::<WNDCLASSEXW>() as u32;
        assert_ne!(GetClassInfoExW(null_mut(),to_wide("EDIT").as_ptr(),&mut edit_definition),0);
        RECEIVER_EDIT_PROC.set(edit_definition.lpfnWndProc).unwrap();
        edit_definition.lpfnWndProc=Some(image_receiver_edit_proc);
        edit_definition.hInstance=platform_window::module_handle();
        edit_definition.lpszClassName=edit_class.as_ptr();
        platform_window::register_class_ex(&edit_definition);
        let edit=platform_window::create_window_ex(0,edit_class.as_ptr(),to_wide("").as_ptr(),
            WS_CHILD|WS_VISIBLE|WS_TABSTOP|ES_AUTOHSCROLL as u32,16,24,540,32,target,null_mut(),platform_window::module_handle(),null());
        assert!(!edit.is_null());
        platform_window::force_foreground(target);
        platform_input::set_focus(edit);
        timer::start(target,197,30_000);
        let ready=serde_json::json!({"pid":std::process::id(),"target":target as usize,"edit":edit as usize});
        std::fs::write(receipt,serde_json::to_vec(&ready).unwrap()).unwrap();
        let mut message:MSG=std::mem::zeroed();
        while platform_window::get_message(&mut message)>0 {
            platform_window::translate_message(&message);
            platform_window::dispatch_message(&message);
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
            for mouse in [false,true] {
                fixture.select_and_paste(WindowsImeInputMode::Unknown, 0, mouse);
                fixture.select_and_paste(WindowsImeInputMode::Native, 0, mouse);
                fixture.select_and_paste(WindowsImeInputMode::Alphanumeric, 2, mouse);
            }
            fixture.focus_change_rejects_selection_and_closes_the_stale_popup();
            fixture.failed_selection_and_stale_image_leave_session_consistent();
        }
        Ok(())
    })).unwrap();
}

#[test]
#[ignore = "Requires a fresh isolated ZSCLIP_DATA_DIR and interactive desktop; uses only a synthetic 2x3 image and an external receiver"]
fn vv_async_image_reaches_external_receiver_through_worker_message_and_timer() {
    let profile=std::path::PathBuf::from(std::env::var_os("ZSCLIP_DATA_DIR").expect("Set a fresh isolated profile"));
    assert!(profile.is_absolute());
    assert!(window_host_hwnds().iter().all(|hwnd|hwnd.is_null()));
    assert!(!platform_window::exists(current_vv_popup_hwnd()));
    let database=profile.join("clipboard.db");
    assert!(!database.exists(),"Use a fresh profile; this fixture must not write to existing history");
    std::fs::create_dir_all(&profile).unwrap();
    let path=profile.join("synthetic-vv-image-2x3.png");
    assert!(!path.exists());
    let rgba=[255u8,0,0,255, 0,255,0,255, 0,0,255,255, 255,255,0,255, 0,255,255,255, 255,0,255,255];
    let file=std::fs::OpenOptions::new().write(true).create_new(true).open(&path).unwrap();
    let mut encoder=png::Encoder::new(file,2,3);
    encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
    let mut writer=encoder.write_header().unwrap();writer.write_image_data(&rgba).unwrap();drop(writer);
    crate::db_runtime::with_test_protected_texts(&[],||crate::db_runtime::with_test_db_path(&database,|| {
        let item=ClipItem {id:0,kind:ClipKind::Image,preview:"Synthetic 2x3 image".into(),phrase_title:String::new(),
            text:None,rich_text_html:None,source_app:"VV image fixture".into(),file_paths:None,image_bytes:None,
            image_path:Some(path.to_string_lossy().into_owned()),image_width:2,image_height:3,pinned:false,group_id:0,created_at:String::new()};
        let item_id=db_insert_item(0,&item,None)?;
        assert!(item_id>0);
        unsafe {
            let mut fixture=DesktopFixture::new_with_receiver_mode("image");
            for (case,mouse) in [false,true].into_iter().enumerate() {
                with_test_ime_observation(WindowsImeInputMode::Unknown,false,|| {
                    assert!(platform_clipboard::WindowsClipboardHost::write_text("VV-IMAGE-SENTINEL"));
                    fixture.trigger_popup();
                    assert_eq!(fixture.app.vv_popup_items.len(),1);
                    assert_eq!(fixture.app.vv_popup_items[0].item.id,item_id);
                    assert!(fixture.app.vv_popup_items[0].item.image_bytes.is_none(),"Must enter real asynchronous image loading");
                    if mouse {
                        let popup=current_vv_popup_hwnd();
                        let row=MainVvPopupLayout::default().with_content_font_size(fixture.app.settings.content_font_size())
                            .scaled(platform_dpi::layout_dpi_for_window(popup)).row_rect(0);
                        let (x,y)=((row.left+row.right)/2,(row.top+row.bottom)/2);
                        let mut point=POINT {x,y};assert!(platform_window::client_to_screen(popup,&mut point));
                        let event=MSLLHOOKSTRUCT {pt:point,mouseData:0,flags:0,time:0,dwExtraInfo:0};
                        super::main_low_level_input::outside_click_mouse_hook_proc(0,WM_LBUTTONDOWN as WPARAM,&event as *const _ as LPARAM);
                        let location=(((y as u32)<<16)|(x as u32&0xffff)) as LPARAM;
                        platform_window::send_message(popup,WM_LBUTTONDOWN,1,location);
                        platform_window::send_message(popup,WM_LBUTTONUP,0,location);
                    } else {
                        assert_eq!(fixture.key_callback(0x31,true),1);
                        assert_eq!(fixture.key_callback(0x31,false),1);
                    }
                    let deadline=Instant::now()+Duration::from_secs(4);
                    let mut received=None;
                    while Instant::now()<deadline {
                        pump_messages();
                        if let Ok(bytes)=std::fs::read(&fixture.image_receipt) {
                            if let Ok(value)=serde_json::from_slice::<serde_json::Value>(&bytes) {
                                if value["ctrl_v_count"].as_u64()==Some((case+1) as u64) {received=Some(value);break;}
                            }
                        }
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    let received=received.expect("External receiver did not observe the real image paste Ctrl+V");
                    assert_eq!(received["pid"].as_u64(),Some(fixture.receiver.as_ref().unwrap().id() as u64));
                    assert_eq!(received["dib_dimensions"],serde_json::json!([2,3]));
                    assert_eq!(received["png_dimensions"],serde_json::json!([2,3]));
                    assert_eq!(received["wm_paste_count"],0,"The receiver must be reached through real SendInput Ctrl+V");
                    assert!(!fixture.app.vv_popup_visible);
                    assert!(fixture.app.pending_image_paste_generation.is_none());
                    assert!(fixture.app.pending_paste_completion.is_none());
                    assert!(fixture.app.vv_paste_guard.is_none());
                });
            }
        }
        Ok(())
    })).unwrap();
}

#[test]
#[ignore = "Requires an interactive desktop and isolated profile; models English-to-Native switching, checks actual external EDIT input counters and real timers"]
fn vv_shift_mode_switch_cancels_pending_visible_and_queued_paste_without_editing_draft() {
    let profile=std::env::var_os("ZSCLIP_DATA_DIR").expect("Set an isolated test profile");
    assert!(std::path::Path::new(&profile).is_absolute());
    assert!(window_host_hwnds().iter().all(|hwnd|hwnd.is_null()));
    assert!(!platform_window::exists(current_vv_popup_hwnd()));
    crate::db_runtime::with_test_protected_texts(&[],||crate::db_runtime::with_test_db(|| {
        let item=ClipItem {id:0,kind:ClipKind::Text,preview:PAYLOAD.into(),phrase_title:String::new(),text:Some(PAYLOAD.into()),
            rich_text_html:None,source_app:"VV Shift cancellation fixture".into(),file_paths:None,image_bytes:None,image_path:None,
            image_width:0,image_height:0,pinned:false,group_id:0,created_at:String::new()};
        assert!(db_insert_item(0,&item,None)?>0);
        unsafe {
            let mut fixture=DesktopFixture::new();
            for stage in ["pending","pending_lock_busy","visible","selected","split_trigger"] {
                eprintln!("VV Shift cancellation case={stage}; IME mode is simulated, receiver/timers/input delivery are real");
                fixture.set_receiver_draft();
                assert!(platform_clipboard::WindowsClipboardHost::write_text("VV-SHIFT-CANCEL-SENTINEL"));
                let initial_sequence=platform_clipboard::sequence_number();
                let before=fixture.receiver_input_counts();
                with_test_ime_observation(WindowsImeInputMode::Alphanumeric,false,|| {
                    if stage=="split_trigger" {
                        fixture.key_callback(0x56,true);fixture.key_callback(0x56,false);
                    } else if matches!(stage,"pending"|"pending_lock_busy") {
                        fixture.key_callback(0x56,true);fixture.key_callback(0x56,false);
                        fixture.key_callback(0x56,true);fixture.key_callback(0x56,false);
                        // Dispatch the real posted show request, but leave its real
                        // VvShow timer pending until after Shift changes input mode.
                        if stage=="pending" {pump_vv_requests_without_timers(fixture.owner);}
                        assert_eq!(vv_hook_state().lock().unwrap().session.phase,VvPhase::Pending);
                        assert!(!fixture.app.vv_popup_visible);
                        if stage=="pending" {assert_eq!(fixture.app.vv_popup_pending_target,fixture.target);}
                    } else {
                        fixture.trigger_popup();
                        if stage=="selected" {
                            assert_eq!(fixture.key_callback(0x31,true),1);
                            assert_eq!(fixture.key_callback(0x31,false),1);
                            // Commit the real selection and establish the 150ms
                            // paste timer without dispatching that timer early.
                            pump_vv_requests_without_timers(fixture.owner);
                            assert_eq!(vv_hook_state().lock().unwrap().session.phase,VvPhase::Selected);
                            assert!(fixture.app.vv_paste_guard.is_some());
                            assert!(fixture.app.pending_paste_completion.is_some());
                        }
                    }
                    if stage=="pending_lock_busy" {
                        // Simulate reentrant input while the UI owns the hook
                        // mutex. Revision tracking must invalidate the already
                        // posted show even when the hook's try_lock cannot run.
                        let _held=vv_hook_state().lock().unwrap();
                        assert_ne!(fixture.key_callback(0x10,true),1);
                        assert_ne!(fixture.key_callback(0x10,false),1);
                    } else {
                        assert_ne!(fixture.key_callback(0x10,true),1,"Shift itself must reach the input method");
                        assert_ne!(fixture.key_callback(0x10,false),1,"Shift release itself must reach the input method");
                    }
                });
                // Clipboard publication may already have occurred before a
                // selected paste is cancelled; it must not be changed again.
                let sequence_after_request=platform_clipboard::sequence_number();
                if stage!="selected" {assert_eq!(sequence_after_request,initial_sequence);}
                with_test_ime_observation(WindowsImeInputMode::Native,false,|| {
                    if stage=="split_trigger" {
                        fixture.key_callback(0x56,true);fixture.key_callback(0x56,false);
                    }
                    pump_for(Duration::from_millis(700));
                    assert!(!fixture.app.vv_popup_visible,"Shift must cancel a {stage} VV session before it can inject input");
                    assert!(!platform_window::is_visible(current_vv_popup_hwnd()));
                    assert_eq!(fixture.receiver_input_counts(),before,"Esc/Backspace/CtrlV/WM_PASTE must not be injected after Shift in {stage}");
                    assert_eq!(fixture.receiver_text(),"LEFTvvRIGHT","Cancelling must preserve the complete controlled draft");
                    fixture.continue_synthetic_character();
                    assert_eq!(fixture.receiver_text(),"LEFTvv续RIGHT","Continued input must preserve the prefix and suffix");
                    assert_eq!(fixture.receiver_input_counts(),before);
                    assert_eq!(platform_clipboard::sequence_number(),sequence_after_request);
                    assert!(fixture.app.vv_paste_guard.is_none());
                    assert!(fixture.app.pending_paste_completion.is_none());
                    assert!(fixture.app.pending_image_paste_generation.is_none());
                    assert_eq!(vv_hook_state().lock().unwrap().session.phase,VvPhase::Cancelled);
                });
            }
        }
        Ok(())
    })).unwrap();
}

#[test]
#[ignore = "Requires an isolated profile and interactive desktop; verifies real pointer cancellation against an external receiver"]
fn vv_pointer_activity_separates_triggers_and_cancels_queued_paste_without_editing_draft() {
    let profile=std::env::var_os("ZSCLIP_DATA_DIR").expect("Set an isolated test profile");
    assert!(std::path::Path::new(&profile).is_absolute());
    assert!(window_host_hwnds().iter().all(|hwnd|hwnd.is_null()));
    assert!(!platform_window::exists(current_vv_popup_hwnd()));
    crate::db_runtime::with_test_protected_texts(&[],||crate::db_runtime::with_test_db(|| {
        let item=ClipItem {id:0,kind:ClipKind::Text,preview:PAYLOAD.into(),phrase_title:String::new(),text:Some(PAYLOAD.into()),
            rich_text_html:None,source_app:"VV pointer fixture".into(),file_paths:None,image_bytes:None,image_path:None,
            image_width:0,image_height:0,pinned:false,group_id:0,created_at:String::new()};
        assert!(db_insert_item(0,&item,None)?>0);
        unsafe {
            let mut fixture=DesktopFixture::new();
            fixture.set_receiver_draft();
            let before=fixture.receiver_input_counts();
            let rect=platform_window::window_rect(fixture.target).unwrap();
            let point=POINT {x:rect.left+8,y:rect.top+8};
            let click=|| {
                let event=MSLLHOOKSTRUCT {pt:point,mouseData:0,flags:0,time:0,dwExtraInfo:0};
                super::main_low_level_input::outside_click_mouse_hook_proc(
                    0,WM_LBUTTONDOWN as WPARAM,&event as *const _ as LPARAM);
            };
            with_test_ime_observation(WindowsImeInputMode::Alphanumeric,false,|| {
                fixture.key_callback(0x56,true); fixture.key_callback(0x56,false);
                assert!(vv_hook_state().lock().unwrap().last_was_v);
                click();
                assert!(!vv_hook_state().lock().unwrap().last_was_v);
                fixture.key_callback(0x56,true); fixture.key_callback(0x56,false);
                assert!(!vv_hook_state().lock().unwrap().session.active());
                pump_for(Duration::from_millis(100));
                assert!(!fixture.app.vv_popup_visible,"V-click-V must not form a trigger");
                fixture.key_callback(0x41,true); fixture.key_callback(0x41,false);
                fixture.trigger_popup();
                assert_eq!(fixture.key_callback(0x31,true),1);
                assert_eq!(fixture.key_callback(0x31,false),1);
                pump_vv_requests_without_timers(fixture.owner);
                assert!(fixture.app.vv_paste_guard.is_some());
                assert_eq!(vv_hook_state().lock().unwrap().session.phase,VvPhase::Selected);
                click();
                pump_for(Duration::from_millis(700));
                assert!(fixture.app.vv_paste_guard.is_none());
                assert!(fixture.app.pending_paste_completion.is_none());
                assert_eq!(vv_hook_state().lock().unwrap().session.phase,VvPhase::Cancelled);
                assert_eq!(fixture.receiver_input_counts(),before);
                assert_eq!(fixture.receiver_text(),"LEFTvvRIGHT");
            });
        }
        Ok(())
    })).unwrap();
}

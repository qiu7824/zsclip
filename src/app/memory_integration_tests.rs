//! Real window/timer coverage of hidden reclamation; no absolute memory-size target.
use super::prelude::*;
use crate::platform::string::to_wide;
use std::time::{Duration, Instant};
use windows_sys::Win32::UI::WindowsAndMessaging::{PeekMessageW, PM_REMOVE};

unsafe extern "system" fn host_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_TIMER && wp == 9091 {
        timer::stop(hwnd, 9091);
        windows_sys::Win32::UI::WindowsAndMessaging::EndMenu();
        return 0;
    }
    if matches!(msg, WM_TIMER | WM_IMAGE_THUMB_READY | WM_IMAGE_PASTE_READY | WM_VV_SHOW | WM_VV_HIDE) {
        return super::main_entry::wnd_proc(hwnd, msg, wp, lp);
    }
    platform_window::default_window_proc(hwnd, msg, wp, lp)
}

#[test]
#[ignore = "Requires an isolated profile and a fresh desktop test process"]
fn hidden_resize_and_tray_menu_close_rearm_memory_reclaim() {
    assert!(std::env::var_os("ZSCLIP_DATA_DIR").is_some());
    crate::db_runtime::with_test_protected_texts(&[], || {
        crate::db_runtime::with_test_db(|| {
            unsafe {
                let mut fixture = Fixture::new();
                fixture.seed_cache();
                schedule_hidden_memory_reclaim(fixture.main, &mut fixture.main_state);
                handle_main_window_size(fixture.main, UiSize { width: 220, height: 110 }, false);
                assert!(fixture.main_state.hidden_reclaim_timer,
                    "A delayed non-minimized WM_SIZE must not cancel a hidden window's reclaim");
                fixture.assert_cache_present();
                pump_for(Duration::from_millis(950));
                assert!(!fixture.main_state.hidden_reclaim_timer);
                assert!(fixture.main_state.payload_cache.get(91001).is_none());

                fixture.seed_cache();
                timer::start(fixture.main, 9091, 80);
                crate::tray::show_tray_menu_localized(fixture.main);
                assert!(fixture.main_state.hidden_reclaim_timer,
                    "Dismissing a tray menu must schedule reclaim even without a visibility change");
                pump_for(Duration::from_millis(950));
                assert!(fixture.main_state.payload_cache.get(91001).is_none());
                assert!(!fixture.main_state.hidden_reclaim_timer);

                handle_global_hotkey(fixture.main, HOTKEY_ID_PLAIN);
                assert!(fixture.main_state.hidden_reclaim_timer,
                    "An empty/cancelled plain paste still ends a background activity");
                pump_for(Duration::from_millis(950));
                assert!(!fixture.main_state.hidden_reclaim_timer);
            }
            Ok(())
        })
    }).unwrap();
}

#[test]
#[ignore = "Fresh isolated profile; validates the visible edge-peek state without moving the user's pointer"]
fn collapsed_edge_peek_reclaims_cached_content_and_drops_late_thumbnails() {
    assert!(std::env::var_os("ZSCLIP_DATA_DIR").is_some());
    crate::db_runtime::with_test_db(||unsafe{
        let mut fixture=Fixture::new();
        platform_window::set_pos(fixture.main,null_mut(),-16000,-16000,640,480,SWP_NOACTIVATE|SWP_NOZORDER);
        platform_window::show_no_activate(fixture.main);
        fixture.main_state.settings.edge_auto_hide=true;
        fixture.main_state.edge_hidden=true;
        fixture.main_state.edge_hidden_side=EDGE_AUTO_HIDE_LEFT;
        fixture.main_state.edge_docked_left=-17000;fixture.main_state.edge_docked_top=-17000;
        fixture.main_state.edge_docked_right=-15000;fixture.main_state.edge_docked_bottom=-15000;
        fixture.main_state.edge_anim_from_x=-16000;fixture.main_state.edge_anim_from_y=-16000;
        fixture.main_state.edge_anim_to_x=-16000;fixture.main_state.edge_anim_to_y=-16000;
        fixture.main_state.edge_anim_until=Some(Instant::now()-Duration::from_millis(1));
        fixture.seed_cache();
        let dc=platform_gdi::get_dc(fixture.main);
        assert!(fixture.main_state.paint_buffer.ensure(dc,640,480).is_some());platform_gdi::release_dc(fixture.main,dc);
        assert!(platform_window::is_visible(fixture.main));
        assert!(window_counts_as_visible_for_memory_reclaim(fixture.main),"Pending animation must still protect visible content");
        handle_edge_auto_hide_tick(fixture.main);
        assert!(!window_counts_as_visible_for_memory_reclaim(fixture.main));
        assert!(fixture.main_state.hidden_reclaim_timer);
        assert!(fixture.main_state.paint_buffer.is_empty(),"Finishing edge collapse must release its local paint buffer immediately");
        handle_main_window_size(fixture.main,UiSize{width:640,height:480},false);
        assert!(fixture.main_state.hidden_reclaim_timer,"A late size event must not cancel edge-peek reclamation");
        repaint_main_window(fixture.main,false);super::main_renderer::paint_main_window(fixture.main);
        assert!(fixture.main_state.paint_buffer.is_empty(),"Peek repaint must not allocate a full-window bitmap");
        pump_for(Duration::from_millis(950));
        assert!(fixture.main_state.payload_cache.get(91001).is_none());
        fixture.main_state.image_thumb_loading.insert(91002);
        let ready=Box::new(crate::app_core::ImageThumbReadyResult{item_id:91002,
            app_data_generation:fixture.main_state.app_data_generation,
            image:Some(crate::app_core::ImageThumbnail{bytes:vec![0,0,255,255],width:1,height:1})});
        assert!(post_boxed_message(fixture.main as isize,WM_IMAGE_THUMB_READY,0,ready));
        pump_for(Duration::from_millis(30));
        assert!(fixture.main_state.image_thumb_cache.get(91002).is_none());
        assert!(fixture.main_state.hidden_reclaim_timer);
        fixture.main_state.edge_hidden=false;
        assert!(window_counts_as_visible_for_memory_reclaim(fixture.main));
        cancel_hidden_memory_reclaim(fixture.main,&mut fixture.main_state);
        platform_window::hide(fixture.main);
        Ok(())
    }).unwrap();
}

#[test]
#[ignore = "Fresh isolated profile; holds a temporary database lock to verify failed saves keep the editor alive"]
fn notes_failed_save_keeps_editor_and_retries_without_losing_text() {
    assert!(std::env::var_os("ZSCLIP_DATA_DIR").is_some());
    crate::db_runtime::with_test_db(|| {
        unsafe {
            let _fixture = Fixture::new();
            let notes = super::notes_window::open_hidden_for_test();
            assert!(platform_window::exists(notes));
            let editor = platform_window::child(notes, 8104);
            assert!(!paste_window_is_zsclip(notes), "Notes must be a valid ordinary paste target");
            let lock = rusqlite::Connection::open(data_dir().join("notes.sqlite3")).unwrap();
            lock.execute_batch("BEGIN IMMEDIATE").unwrap();
            let body = "保留未保存内容\r\nhttps://docs.qq.com/doc/DistinctDocument";
            platform_window::send_message(editor, 0x00B1, 0, -1); // EM_SETSEL
            platform_window::send_message(editor, 0x00C2, 1, to_wide(body).as_ptr() as isize); // EM_REPLACESEL, emits EN_CHANGE
            assert!(super::notes_window::try_close().is_err());
            assert!(platform_window::exists(notes));
            assert_eq!(platform_window::text(editor), body);
            lock.execute_batch("ROLLBACK").unwrap();
            super::notes_window::try_close().unwrap();
            assert!(!platform_window::exists(notes));
            let saved: String = lock.query_row("SELECT body FROM notes ORDER BY id DESC LIMIT 1", [], |row| row.get(0)).unwrap();
            assert_eq!(saved, body);
        }
        Ok(())
    }).unwrap();
}

#[test]
#[ignore = "Fresh isolated profile; verifies queued Ctrl shortcuts on hidden note controls without physical input"]
fn notes_queued_control_shortcuts_create_and_save_the_selected_note() {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetKeyboardState, SetKeyboardState, VK_CONTROL};
    assert!(std::env::var_os("ZSCLIP_DATA_DIR").is_some());
    crate::db_runtime::with_test_db(|| {
        unsafe {
            let _fixture = Fixture::new();
            let notes = super::notes_window::open_hidden_for_test();
            let editor = platform_window::child(notes, 8104);
            let db = rusqlite::Connection::open(data_dir().join("notes.sqlite3")).unwrap();
            let count: i64 = db.query_row("SELECT count(*) FROM notes", [], |r| r.get(0)).unwrap();
            struct KeyboardState([u8; 256]);
            impl Drop for KeyboardState {
                fn drop(&mut self) { unsafe { SetKeyboardState(self.0.as_ptr()); } }
            }
            let mut saved = KeyboardState([0; 256]);
            assert_ne!(GetKeyboardState(saved.0.as_mut_ptr()), 0);
            let mut queued = saved.0;
            queued[VK_CONTROL as usize] = 0x80;
            assert_ne!(SetKeyboardState(queued.as_ptr()), 0);
            let mut message = MSG { hwnd: editor, message: WM_KEYDOWN, wParam: 0x4E, ..std::mem::zeroed() };
            assert!(super::notes_window::route_message(&message));
            let after: i64 = db.query_row("SELECT count(*) FROM notes", [], |r| r.get(0)).unwrap();
            assert_eq!(after, count + 1);
            let body = "快捷键保存测试";
            platform_window::send_message(editor, 0x00C2, 1, to_wide(body).as_ptr() as isize);
            message.wParam = 0x53;
            assert!(super::notes_window::route_message(&message));
            let text: String = db.query_row("SELECT body FROM notes ORDER BY id DESC LIMIT 1", [], |r| r.get(0)).unwrap();
            assert_eq!(text, body);
            queued[VK_CONTROL as usize] = 0;
            assert_ne!(SetKeyboardState(queued.as_ptr()), 0);
            assert!(!super::notes_window::route_message(&message));
            super::notes_window::try_close().unwrap();
        }
        Ok(())
    }).unwrap();
}

#[test]
#[ignore = "Fresh isolated profile; verifies native resize, persistence and quick-window restoration without physical input"]
fn clipboard_window_width_survives_resize_and_reopen() {
    assert!(std::env::var_os("ZSCLIP_DATA_DIR").is_some());
    crate::db_runtime::with_test_db(|| {
        unsafe {
            let mut fixture = Fixture::new();
            let dpi = main_window_layout_dpi(fixture.main).max(96);
            fixture.main_state.ui_dpi = dpi;
            let work = platform_monitor::nearest_work_rect_for_window(fixture.main);
            let width = (720 * dpi as i32 / 96).min(work.right - work.left);
            let height=(480*dpi as i32/96).min(work.bottom-work.top);
            platform_window::set_pos(fixture.main, null_mut(), 0, 0, width,
                height, SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE);
            fixture.main_state.window_interactive_move = true;
            fixture.main_state.window_user_resized = true;
            ensure_main_window_size_for_monitor(fixture.main, &mut fixture.main_state);
            assert_eq!(platform_window::client_rect(fixture.main).unwrap().right, width);
            handle_main_window_move_completed(fixture.main);
            let saved = load_settings();
            let logical = (width * 96 + dpi as i32 / 2) / dpi as i32;
            assert_eq!(saved.clipboard_window_width, logical);
            assert_eq!(saved.clipboard_window_height,(height*96+dpi as i32/2)/dpi as i32);
            assert_eq!(fixture._quick_state.settings.clipboard_window_width, logical);
            let actual = fixture.main_state.layout();
            assert_eq!(actual.win_w, width);
            assert_eq!(actual.list_w, width - actual.list_x * 2);
            assert_eq!(actual.list_y+actual.list_h+7,height);
            assert!(actual.title_button_rect("close").right <= width);
            let rect = platform_window::window_rect(fixture.main).unwrap();
            for (x, expected_hit) in [(rect.left + 2, HTLEFT), (rect.right - 2, HTRIGHT)] {
                let y = rect.top + 100;
                let point = ((x as u32 & 0xffff) | ((y as u32 & 0xffff) << 16)) as isize;
                assert_eq!(handle_nchittest(fixture.main, point), expected_hit as isize);
            }
            let restored = AppState::new(WindowRole::Quick, null_mut(), null_mut(),
                Icons { app:0,search:0,setting:0,min:0,close:0,text:0,image:0,file:0,folder:0,pin:0,del:0 }, None);
            assert_eq!(restored.settings.clipboard_window_width, logical);
            assert_eq!(restored.settings.clipboard_window_height,saved.clipboard_window_height);
            position_main_window(fixture.quick, &saved, false);
            let quick_dpi = main_window_layout_dpi(fixture.quick);
            let quick_work = platform_monitor::nearest_work_rect_for_window(fixture.quick);
            let expected = main_layout_for_dpi(quick_dpi).with_saved_width(logical, quick_dpi, quick_work.right - quick_work.left);
            assert_eq!(platform_window::client_rect(fixture.quick).unwrap().right, expected.win_w);
            assert_eq!(platform_window::client_rect(fixture.quick).unwrap().bottom,
                (saved.clipboard_window_height*quick_dpi as i32+48)/96);
        }
        Ok(())
    }).unwrap();
}

#[repr(C)]
#[derive(Default)]
struct MemorySample {
    cb: u32,
    page_faults: u32,
    peak_working_set: usize,
    working_set: usize,
    quota_peak_paged: usize,
    quota_paged: usize,
    quota_peak_nonpaged: usize,
    quota_nonpaged: usize,
    pagefile: usize,
    peak_pagefile: usize,
    private_commit: usize,
    private_working_set: usize,
    shared_commit: usize,
}

unsafe fn memory_sample() -> MemorySample {
    #[link(name="psapi")]
    unsafe extern "system" {
        fn GetProcessMemoryInfo(process: *mut core::ffi::c_void, memory: *mut MemorySample, size: u32) -> i32;
    }
    let mut sample = MemorySample { cb: std::mem::size_of::<MemorySample>() as u32, ..Default::default() };
    assert_ne!(GetProcessMemoryInfo(windows_sys::Win32::System::Threading::GetCurrentProcess(), &mut sample, std::mem::size_of::<MemorySample>() as u32), 0);
    sample
}

#[test]
#[ignore = "Isolated profile; paints an off-screen native fixture and measures resize cache reuse and hidden release"]
fn resize_paint_cache_reuses_resources_and_releases_while_peer_is_visible() {
    use windows_sys::Win32::System::Threading::{GetCurrentProcess,GetGuiResources,GR_GDIOBJECTS};
    let output=std::path::PathBuf::from(std::env::var_os("ZSCLIP_TEST_TEMP_ROOT").expect("isolated output"));
    crate::db_runtime::with_test_db(||{
        unsafe {
            let mut fixture=Fixture::new();
            fixture.main_state.records=(0..2000).map(|i|{let mut item=cached_item();item.id=i+1;item.preview=format!("记录 {i} https://docs.qq.com/doc/shared-prefix-{i}");item}).collect();
            fixture.main_state.list.apply_visible_len(2000);
            fixture.main_state.window_interactive_move=true;
            platform_window::set_pos(fixture.main,null_mut(),-16000,-16000,480,320,SWP_NOACTIVATE|SWP_NOZORDER);
            platform_window::show_no_activate(fixture.main);
            let mut samples=Vec::new();
            for frame in 0..100 {
                let width=480+frame*4;let height=320+frame*3;
                let started=Instant::now();
                platform_window::set_pos(fixture.main,null_mut(),-16000,-16000,width,height,SWP_NOACTIVATE|SWP_NOZORDER);
                handle_main_window_size(fixture.main,UiSize{width,height},false);
                super::main_renderer::paint_main_window(fixture.main);
                samples.push(started.elapsed().as_micros() as u64);
            }
            let allocations=fixture.main_state.paint_buffer.allocations;
            let layout_builds=fixture.main_state.row_layout_cache.borrow().builds;
            assert!(allocations<=12,"Resizing allocated a fresh bitmap too often: {allocations}");
            assert!(layout_builds<=2,"Row heights were rebuilt while only window size changed: {layout_builds}");
            let gdi_before=GetGuiResources(GetCurrentProcess(),GR_GDIOBJECTS);
            fixture.seed_cache();
            platform_window::set_pos(fixture.quick,null_mut(),-16000,-16000,220,110,SWP_NOACTIVATE|SWP_NOZORDER);
            platform_window::show_no_activate(fixture.quick);
            platform_window::hide(fixture.main);
            handle_main_lifecycle_event(fixture.main,LifecycleEvent::Suspend);
            assert!(fixture.main_state.paint_buffer.is_empty());
            fixture.assert_cache_present();
            let gdi_after=GetGuiResources(GetCurrentProcess(),GR_GDIOBJECTS);
            assert!(gdi_after<gdi_before,"Hidden window retained its bitmap/DC while another panel was visible");
            platform_window::hide(fixture.quick);
            schedule_hidden_memory_reclaim_after_activity();pump_for(Duration::from_millis(950));
            assert!(fixture.main_state.payload_cache.get(91001).is_none());
            samples.sort_unstable();let memory=memory_sample();
            let report=serde_json::json!({"frames":100,"records":2000,"bitmap_allocations":allocations,"row_layout_builds":layout_builds,
                "p50_us":samples[50],"p95_us":samples[95],"max_us":samples[99],"gdi_before_hide":gdi_before,"gdi_after_hide":gdi_after,
                "closed_private_working_set":memory.private_working_set,"private_commit":memory.private_commit});
            std::fs::write(output.join("resize-performance.json"),serde_json::to_vec_pretty(&report).unwrap()).unwrap();
            eprintln!("resize_performance={report}");
        }
        Ok(())
    }).unwrap();
}

#[test]
#[ignore = "Fresh isolated process; 20 note open/save/close cycles and real idle reclamation timers"]
fn notes_reopen_cycles_release_windows_gdi_and_reclaim_idle_working_set() {
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS};
    let output = std::path::PathBuf::from(std::env::var_os("ZSCLIP_TEST_TEMP_ROOT").expect("isolated temporary root"));
    assert!(std::env::var_os("ZSCLIP_DATA_DIR").is_some());
    crate::db_runtime::with_test_protected_texts(&[], || crate::db_runtime::with_test_db(|| {
        unsafe {
            let mut fixture = Fixture::new();
            let mut measurements = Vec::new();
            for cycle in 0..20 {
                let notes = super::notes_window::open_hidden_for_test();
                assert!(platform_window::exists(notes));
                let editor = platform_window::child(notes, 8104);
                assert!(!editor.is_null());
                platform_window::send_message(editor, 0x00B1, 0, -1); // EM_SETSEL
                let body = format!("便签测试 {cycle:02}\r\n{}", "0123456789 local note text\r\n".repeat(64));
                platform_window::send_message(editor, 0x00C2, 1, to_wide(&body).as_ptr() as isize); // EM_REPLACESEL
                let opened = memory_sample();
                super::notes_window::try_close().unwrap();
                assert!(!platform_window::exists(notes));
                let saved = crate::notes_store::NotesStore::open(&data_dir().join("notes.sqlite3")).unwrap();
                assert_eq!(saved.read(saved.selected().unwrap()).unwrap(), body);
                drop(saved);
                assert!(fixture.main_state.hidden_reclaim_timer);
                pump_for(Duration::from_millis(950));
                assert!(!fixture.main_state.hidden_reclaim_timer);
                let closed = memory_sample();
                let gdi = GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS);
                measurements.push(serde_json::json!({"cycle":cycle,"open_private_working_set":opened.private_working_set,
                    "closed_private_working_set":closed.private_working_set,"private_commit":closed.private_commit,"gdi_objects":gdi}));
            }
            let warm_commit = measurements[3]["private_commit"].as_u64().unwrap();
            let warm_gdi = measurements[3]["gdi_objects"].as_u64().unwrap();
            let last = measurements.last().unwrap();
            assert!(last["private_commit"].as_u64().unwrap() <= warm_commit + 4*1024*1024,
                "private commitment kept growing after warm-up: {measurements:?}");
            assert!(last["gdi_objects"].as_u64().unwrap() <= warm_gdi + 4,
                "GDI objects leaked across note windows: {measurements:?}");
            std::fs::create_dir_all(&output).unwrap();
            std::fs::write(output.join("notes-memory-cycles.json"), serde_json::to_vec_pretty(&measurements).unwrap()).unwrap();
            eprintln!("notes_memory_cycles={}", serde_json::to_string(&measurements).unwrap());
            fixture.seed_cache();
        }
        Ok(())
    })).unwrap();
}

unsafe extern "system" fn settings_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if matches!(msg, WM_CLOSE | WM_DESTROY) || (msg == WM_SIZE && wp == SIZE_MINIMIZED as WPARAM) {
        return super::settings_window::settings_wnd_proc(hwnd, msg, wp, lp);
    }
    platform_window::default_window_proc(hwnd, msg, wp, lp)
}

#[test]
#[ignore = "Renders off-screen native dark/light previews with synthetic clipboard records into the isolated output directory"]
fn compact_tab_style_snapshots_keep_fixed_fonts_at_two_window_sizes() {
    let output=std::path::PathBuf::from(std::env::var_os("ZSCLIP_TEST_TEMP_ROOT").expect("isolated output"));
    crate::db_runtime::with_test_db(||unsafe{
        let mut fixture=Fixture::new();
        fixture.main_state.settings=AppSettings::default();
        fixture.main_state.settings.clipboard_capture_enabled=false;
        fixture.main_state.ui_dpi=96;
        fixture.main_state.records=(0..15).map(|i|{
            let mut item=cached_item();item.id=i+1;
            item.preview=match i {
                0=>"https://docs.qq.com/sheet/DWEFQb3h4b1VmRGhC?tab=000001".into(),
                1=>"https://docs.qq.com/sheet/DWEFQb3h4b1VmRHdD?tab=000002".into(),
                2=>"会议纪要：核对进度、材料和待办事项".into(),
                3=>"周计划：整理清单，完成资料复核".into(),
                _=>format!("工作记录 {}：资料整理与检查",i+1),
            };item
        }).collect();
        fixture.main_state.list.apply_visible_len(15);
        for dark in [true,false] {
            platform_appearance::set_dark_mode_enabled(dark);fixture.main_state.theme=Theme::default();
            for (width,height) in [(300,615),(710,750)] {
                platform_window::set_pos(fixture.main,null_mut(),-16000,-16000,width,height,SWP_NOACTIVATE|SWP_NOZORDER);
                platform_window::show_no_activate(fixture.main);
                repaint_main_window(fixture.main,false);super::main_renderer::paint_main_window(fixture.main);
                let rgba=fixture.main_state.paint_buffer.snapshot_rgba(width,height).unwrap();
                let path=output.join(format!("tabs-{}-{width}.png",if dark{"dark"}else{"light"}));
                let mut encoder=png::Encoder::new(std::fs::File::create(path).unwrap(),width as u32,height as u32);
                encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
                let mut writer=encoder.write_header().unwrap();writer.write_image_data(&rgba).unwrap();writer.finish().unwrap();
                assert_eq!(fixture.main_state.layout().row_text_size(),12);
            }
        }
        platform_window::hide(fixture.main);
        Ok(())
    }).unwrap();
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
            WS_EX_APPWINDOW,
            to_wide("EDIT").as_ptr(),
            to_wide("Synthetic memory target").as_ptr(),
            WS_OVERLAPPEDWINDOW | ES_MULTILINE as u32,
            120, 120, 320, 180,
            null_mut(), null_mut(), platform_window::module_handle(), null(),
        );
        assert!(!hwnd.is_null());
        let target = Self { hwnd, previous_foreground };
        platform_window::show(hwnd);
        let foreground=platform_window::try_set_foreground(hwnd);
        if !foreground {
            let wait=std::env::var("ZSCLIP_TEST_FOREGROUND_WAIT_MS").ok().and_then(|v|v.parse::<u64>().ok()).unwrap_or(0).min(30_000);
            let deadline=Instant::now()+Duration::from_millis(wait);
            while platform_window::foreground()!=hwnd && Instant::now()<deadline {pump_for(Duration::from_millis(20));}
        }
        assert_eq!(platform_window::foreground(),hwnd,"Controlled target needs foreground; a desktop test operator may activate this synthetic receiver");
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

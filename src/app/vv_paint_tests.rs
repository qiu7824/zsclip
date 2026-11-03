use super::*;
use crate::platform::string::to_wide;

#[test]
#[ignore = "Hidden native fixture only; no pointer, keyboard, focus or clipboard changes"]
fn hover_repaints_complete_frames_without_geometry_or_focus_changes() {
    let output = PathBuf::from(std::env::var_os("ZSCLIP_TEST_TEMP_ROOT").expect("isolated output"));
    crate::db_runtime::with_test_db(|| {
        unsafe {
            let class = to_wide("ZsClipVvHiddenPaintOwner");
            let wc = WNDCLASSEXW { cbSize: size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(windows_sys::Win32::UI::WindowsAndMessaging::DefWindowProcW),
                hInstance: platform_window::module_handle(), lpszClassName: class.as_ptr(), ..zeroed() };
            platform_window::register_class_ex(&wc);
            let main = platform_window::create_window_ex(WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                class.as_ptr(), class.as_ptr(), WS_POPUP, -16000, -16000, 100, 100,
                null_mut(), null_mut(), platform_window::module_handle(), null());
            assert!(!main.is_null());
            let icons = Icons { app:0, search:0, setting:0, min:0, close:0, text:0, image:0, file:0, folder:0, pin:0, del:0 };
            let mut state = Box::new(AppState::new(WindowRole::Main, main, null_mut(), icons, None));
            platform_window::set_user_data(main, (&mut *state as *mut AppState) as isize);
            state.vv_popup_protection_revision = crate::db_runtime::search_protection_revision().ok();
            state.vv_popup_items = (0..9).map(|index| VvPopupEntry { index: index + 1, item: ClipItem {
                id:index as i64 + 1, kind:ClipKind::Text, phrase_title:String::new(),
                preview:format!("Row {} — 中文 👩‍💻 long title", index+1), text:Some(format!("Row {}",index+1)),
                rich_text_html:None, source_app:String::new(), file_paths:None, image_bytes:None,
                image_path:None,image_width:0,image_height:0,pinned:false,group_id:0,created_at:String::new(),
            }}).collect();
            let mut host = WindowsTransientWindowHost::new("ZsClipVvHiddenPaint", Some(vv_popup_wnd_proc));
            let layout = vv_popup_layout_for_window(main);
            let NativeTransientWindowPresentation::Created(popup) = host.create_transient_window(
                NativeTransientWindowRequest { owner:main, bounds:UiRect::new(-16000,-16000,
                    -16000+layout.width,-16000+layout.height(9)) }) else { panic!("fixture window"); };
            let rect = platform_window::client_rect(popup).unwrap();
            let width = rect.right - rect.left; let height = rect.bottom - rect.top;
            let bounds = platform_window::window_rect(popup).unwrap();
            let mut foreground = platform_window::foreground();
            let mut external_focus_changes = 0;
            assert_eq!(vv_popup_wnd_proc(popup, WM_ERASEBKGND, 0, 0), 1);
            let mut previous: Option<Vec<u8>> = None;
            let mut changed_frames = 0;
            for frame in 0..180 {
                let index = (frame / 2) % 9;
                let row = vv_popup_layout_for_window(popup).row_rect(index);
                let point = ((row.top + 8) << 16) | (row.left + 10);
                vv_popup_wnd_proc(popup, WM_MOUSEMOVE, 0, point as isize);
                timer::stop(popup, VV_PREVIEW_HOVER_TIMER);
                platform_gdi::invalidate_rect(popup, null(), 0);
                vv_popup_wnd_proc(popup, WM_PAINT, 0, 0);
                let pixels = state.vv_popup_paint_buffer.snapshot_rgba(width,height).expect("buffered frame");
                if let Some(before) = &previous {
                    if frame % 2 == 1 { assert_eq!(before, &pixels, "same row changes pixels"); }
                    else { assert_ne!(before, &pixels, "cross-row highlight missing"); changed_frames += 1; }
                }
                if frame == 179 {
                    let mut encoder = png::Encoder::new(std::fs::File::create(output.join("vv-buffered.png")).unwrap(),width as u32,height as u32);
                    encoder.set_color(png::ColorType::Rgba); encoder.set_depth(png::BitDepth::Eight);
                    encoder.write_header().unwrap().write_image_data(&pixels).unwrap();
                }
                previous=Some(pixels);
                let current = platform_window::window_rect(popup).unwrap();
                assert_eq!((current.left,current.top,current.right,current.bottom),
                    (bounds.left,bounds.top,bounds.right,bounds.bottom));
                let current_foreground = platform_window::foreground();
                // The user's desktop can change focus while this hidden test
                // runs. Only our fixture taking focus is a regression.
                assert_ne!(current_foreground,main);
                assert_ne!(current_foreground,popup);
                if current_foreground != foreground { external_focus_changes += 1; }
                foreground = current_foreground;
                assert!(!platform_window::is_visible(popup));
            }
            assert_eq!(state.vv_popup_paint_buffer.allocations,1);
            vv_popup_hide(main,&mut state);
            assert!(state.vv_popup_paint_buffer.is_empty());
            cancel_hidden_memory_reclaim(main,&mut state);
            platform_window::destroy(popup);
            platform_window::set_user_data(main,0); platform_window::destroy(main);
            eprintln!("VV hidden paint: 180 frames, {changed_frames} cross-row changes, 1 bitmap allocation, stable bounds, fixture never focused, {external_focus_changes} external focus changes, released on hide");
        }
        Ok(())
    }).unwrap();
}

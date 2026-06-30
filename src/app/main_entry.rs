use super::prelude::*;

pub(crate) fn run() -> AppResult<()> {
    let _ = crate::cloud_sync::cleanup_cloud_sync_temp_files();
    let boot_settings = load_settings();
    // ── 单实例保护：若已有实例运行则激活它并退出 ──
    let (_single_instance_mutex, already_running) =
        platform_process::create_named_mutex("Global\\ZsClipSingleInstance");
    if already_running {
        // 已有实例：找到主窗口并激活
        let hwnd = platform_window::find_window_by_class(WindowRole::Main.class_name());
        if !hwnd.is_null() {
            let mut main_window_host = WindowsMainWindowHost::new(Some(wnd_proc));
            if !boot_settings.tray_icon_enabled {
                main_window_host.close_main_window(hwnd);
            } else {
                main_window_host.restore_main_window(hwnd);
                main_window_host.foreground_main_window(hwnd);
            }
        }
        return Ok(());
    }
    unsafe {
        platform_dpi::init_process_awareness();
        // 进程级深色模式初始化：让系统菜单、滚动条、控件跟随主题
        platform_appearance::init_dark_mode_for_process();

        let startup_layout = main_layout_for_window(null_mut());
        let startup_h = startup_layout.list_y + startup_layout.list_h + 7;
        let mut main_window_host = WindowsMainWindowHost::new(Some(wnd_proc));
        if let NativeMainWindowPresentation::Failed =
            main_window_host.create_main_windows(NativeMainWindowRequest {
                title: app_title().to_string(),
                size: UiSize {
                    width: startup_layout.win_w,
                    height: startup_h,
                },
                main_visible: !startup_can_hide(&boot_settings),
            })
        {
            return Err(io::Error::last_os_error());
        }

        let mut msg: MSG = zeroed();
        loop {
            let code = platform_window::get_message(&mut msg);
            if code == -1 {
                return Err(io::Error::last_os_error());
            }
            if code == 0 {
                break;
            }
            platform_window::translate_message(&msg);
            platform_window::dispatch_message(&msg);
        }
    }

    Ok(())
}

pub(super) unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_DPICHANGED {
        apply_dpi_suggested_rect(hwnd, lparam);
    }
    if let Some(event) = main_window_host_event_from_message(msg, wparam, lparam) {
        match event {
            MainWindowHostEvent::Async(event) => {
                handle_main_async_event(hwnd, event);
                return 0;
            }
            MainWindowHostEvent::Ui(event) => {
                if dispatch_main_ui_event(hwnd, event) {
                    return 0;
                }
            }
        }
    }
    match msg {
        WM_CREATE => {
            let cs = &*(lparam as *const CREATESTRUCTW);
            let role = WindowRole::from_create_param(cs.lpCreateParams as isize);
            match on_create(hwnd, role) {
                Ok(_) => 0,
                Err(_) => -1,
            }
        }
        WM_PAINT => {
            paint_main_window(hwnd);
            0
        }
        WM_ERASEBKGND => 1,
        WM_MOUSEACTIVATE => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                let mut keep_noactivate = false;
                let state = &*ptr;
                if state.main_window_noactivate {
                    let mut pt = platform_input::cursor_pos().unwrap_or(POINT { x: 0, y: 0 });
                    platform_window::screen_to_client(hwnd, &mut pt);
                    keep_noactivate = main_window_should_stay_noactivate(state, pt.x, pt.y);
                }
                if keep_noactivate {
                    return MA_NOACTIVATE as LRESULT;
                }
                if state.main_window_noactivate {
                    WindowsMainWindowHost::new(Some(wnd_proc))
                        .set_main_window_activation_policy(hwnd, true);
                    let ptr = get_state_ptr(hwnd);
                    if !ptr.is_null() {
                        clear_hotkey_passthrough_state(&mut *ptr);
                    }
                }
            }
            platform_window::default_window_proc(hwnd, msg, wparam, lparam)
        }
        WM_NCHITTEST => handle_nchittest(hwnd, lparam),
        WM_NCDESTROY => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                clear_window_host((*ptr).role, hwnd);
                if !(*ptr).search_font.is_null() {
                    WindowsMainSearchControlHost::new()
                        .release_search_style_resource((*ptr).search_font);
                    (*ptr).search_font = null_mut();
                }
                (*ptr).icons.destroy();
                drop(Box::from_raw(ptr));
                platform_window::set_user_data(hwnd, 0);
            }
            platform_window::default_window_proc(hwnd, msg, wparam, lparam)
        }
        _ => platform_window::default_window_proc(hwnd, msg, wparam, lparam),
    }
}

pub(super) unsafe fn on_create(hwnd: HWND, role: WindowRole) -> AppResult<()> {
    let layout = main_layout_for_window(hwnd);
    let mut search_host = WindowsMainSearchControlHost::new();
    let search_request = WindowsMainSearchControlHost::search_control_request_from_native_spec(
        hwnd,
        IDC_SEARCH as i64,
        UiRect::new(
            layout.search_left + 10,
            layout.search_top + 3,
            layout.search_left + layout.search_w - 10,
            layout.search_top + layout.search_h - 3,
        ),
        true,
    );
    let search_hwnd = match search_host.create_search_control(search_request) {
        NativeMainSearchControlPresentation::Created(search_hwnd) => search_hwnd,
        NativeMainSearchControlPresentation::Failed => return Err(io::Error::last_os_error()),
    };

    let icons = load_icons();
    let tray_icon = icons.app;
    if icons.app != 0 {
        WindowsMainWindowHost::new(Some(wnd_proc)).set_main_window_app_icon(
            hwnd,
            NativeAppIconResource {
                small: icons.app,
                big: icons.app,
            },
        );
    }

    let state = Box::new(AppState::new(role, hwnd, search_hwnd, icons));
    platform_window::set_user_data(hwnd, Box::into_raw(state) as isize);
    set_window_host(role, hwnd);
    let _ = dispatch_main_ui_event(hwnd, UiEvent::Lifecycle(LifecycleEvent::Mount));
    if let Some(state) = unsafe { get_state_mut(hwnd) } {
        refresh_search_font(state);
        ensure_db();
        if role == WindowRole::Main {
            if lan_sync::ensure_device_identity(&mut state.settings) {
                save_settings(&state.settings);
            }
            reload_state_from_db_persisting(state);
            spawn_startup_data_reconcile(hwnd, !state.settings.dedupe_filter_enabled);
            register_hotkey_for(hwnd, state);
            register_plain_paste_hotkey_for(hwnd, state);
            let _ = update_vv_mode_hook(hwnd, state.settings.vv_mode_enabled);
            position_main_window(hwnd, &state.settings, false);
            refresh_low_level_input_hooks();
            refresh_lan_latest_from_db(&state.settings);
            lan_sync::refresh_service(hwnd, &state.settings);
        }
    }

    if role == WindowRole::Main {
        if let Some(state) = unsafe { get_state_mut(hwnd) } {
            register_clipboard_listener_for(hwnd, state);
        }
    }
    WindowsMainWindowHost::new(Some(wnd_proc)).apply_main_window_appearance(hwnd);
    if role == WindowRole::Main {
        if let Some(state) = unsafe { get_state_mut(hwnd) } {
            sync_main_tray_icon(hwnd, state);
        } else if tray_icon != 0 {
            add_tray_icon_localized(hwnd, tray_icon);
        }
    } else {
        WindowsMainWindowHost::new(Some(wnd_proc)).set_main_window_activation_policy(hwnd, false);
        refresh_low_level_input_hooks();
    }
    refresh_low_level_input_hooks();
    layout_children(hwnd);
    repaint_main_window(hwnd, true);
    if role == WindowRole::Main {
        timer::start(hwnd, ID_TIMER_STARTUP_RECOVERY, 500);
        timer::start(hwnd, ID_TIMER_VV_WATCH, 500);
        timer::start(hwnd, ID_TIMER_CLOUD_SYNC, 5000);
    }
    Ok(())
}

pub(super) unsafe fn handle_control_command(hwnd: HWND, id: usize, notification: u16) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;

    if handle_search_control_command(hwnd, state, id, notification) {
        return;
    }

    if let Some(command) = main_menu_command_for_id(id) {
        state.ui_commands.push(command);
        drain_main_ui_commands(hwnd);
    }
}

pub(super) fn normalize_plain_text_for_paste(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

pub(super) unsafe fn stop_search_debounce_timer(hwnd: HWND, state: &mut AppState) {
    stop_flagged_timer(
        hwnd,
        ID_TIMER_SEARCH_DEBOUNCE,
        &mut state.search_debounce_timer,
    );
}

pub(super) unsafe fn apply_search_filter(hwnd: HWND, state: &mut AppState) {
    let plan = state.list.search_filter_apply_plan();
    state.list.apply_search_filter_plan(plan);
    state.refilter();
    repaint_main_window(hwnd, true);
}

pub(super) unsafe fn cancel_main_scroll_drag(hwnd: HWND, state: &mut AppState) {
    if state.scroll_dragging {
        state.scroll_dragging = false;
        release_main_pointer(hwnd);
        repaint_main_window(hwnd, false);
    }
}

pub(super) unsafe fn settings_set_hotkey_recording(st: &mut SettingsWndState, recording: bool) {
    st.hotkey_recording = recording;
    if !st.btn_hk_record.is_null() {
        settings_set_text(
            st.btn_hk_record,
            if recording {
                tr("按下快捷键...", "Press shortcut...")
            } else {
                tr("录制热键", "Record Hotkey")
            },
        );
        repaint_settings_control(st.btn_hk_record);
    }
    if !st.lb_hk_preview.is_null() {
        if recording {
            settings_set_text(
                st.lb_hk_preview,
                tr("请按修饰键 + 按键", "Press modifier + key"),
            );
        } else {
            settings_set_text(
                st.lb_hk_preview,
                &hotkey_preview_text(
                    &settings_host_text(st.cb_hk_mod),
                    &settings_host_text(st.cb_hk_key),
                ),
            );
        }
        repaint_settings_control(st.lb_hk_preview);
    }
}

pub(super) unsafe fn handle_vv_select(hwnd: HWND, state: &mut AppState, index: usize) {
    let popup_visible = state.vv_popup_visible;
    let target = state.vv_popup_target;
    let backspaces = if popup_visible {
        vv_backspace_count_for_target_window(target, state.vv_popup_replaces_ime)
    } else {
        0
    };
    let items = if popup_visible {
        state
            .vv_popup_items
            .iter()
            .map(|entry| entry.item.clone())
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let Some(plan) = main_vv_select_plan(popup_visible, index, &items, backspaces) else {
        return;
    };
    let (item, backspaces) = match plan {
        MainVvSelectPlan::HidePopup => {
            vv_popup_hide(hwnd, state);
            return;
        }
        MainVvSelectPlan::Paste { item, backspaces } => {
            vv_popup_hide(hwnd, state);
            (item, backspaces)
        }
    };
    if queue_async_image_paste_if_needed(
        hwnd,
        state,
        &item,
        target,
        state.settings.click_hide,
        backspaces,
    ) {
        let plan = main_paste_completion_plan(
            MainPasteCompletionKind::VvAsyncImage,
            paste_completion_input(state, item.id),
        );
        execute_paste_completion_plan(hwnd, state, plan);
        return;
    }
    if !apply_item_to_clipboard(state, &item) {
        show_clipboard_write_failure_message(hwnd);
        return;
    }
    let plan = main_paste_completion_plan_with_backspaces(
        MainPasteCompletionKind::VvClipboard,
        paste_completion_input(state, item.id),
        backspaces,
    );
    execute_paste_completion_plan_to_target(hwnd, state, plan, Some(target));
}

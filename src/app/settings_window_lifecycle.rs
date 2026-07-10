use super::prelude::*;

pub(super) unsafe fn open_settings_window(hwnd: HWND) {
    let owner_hwnd = {
        let main = main_window_hwnd();
        if main.is_null() {
            hwnd
        } else {
            main
        }
    };
    let pst = get_state_ptr(owner_hwnd);
    if pst.is_null() {
        return;
    }
    let app = &mut *pst;
    if !app.settings_hwnd.is_null() {
        app.edge_hide_pending_until = None;
        app.edge_hide_grace_until = None;
        let st_ptr = platform_window::user_data(app.settings_hwnd) as *mut SettingsWndState;
        if !st_ptr.is_null() {
            let next_dpi = settings_window_layout_dpi(app.settings_hwnd).max(96);
            let old_dpi = (*st_ptr).ui_dpi.max(96);
            if old_dpi != next_dpi {
                (*st_ptr).ui_dpi = next_dpi;
                (*st_ptr).suppress_size_refresh = true;
                resize_settings_window_for_dpi_transition(app.settings_hwnd, old_dpi, next_dpi);
                (*st_ptr).suppress_size_refresh = false;
                refresh_settings_window_metrics(app.settings_hwnd, &mut *st_ptr);
            }
        }
        let mut settings_host = WindowsSettingsWindowHost::new(Some(settings_wnd_proc));
        let _ = settings_host.present_settings_window(NativeSettingsWindowRequest {
            owner: owner_hwnd,
            existing: Some(app.settings_hwnd),
            bounds: UiRect::new(0, 0, 0, 0),
        });
        WindowsMainWindowHost::new(Some(wnd_proc)).hide_main_window(owner_hwnd);
        refresh_low_level_input_hooks();
        return;
    }

    let mut anchor = platform_input::cursor_pos().unwrap_or_else(|| POINT {
        x: platform_window::system_metric(SM_CXSCREEN) / 2,
        y: platform_window::system_metric(SM_CYSCREEN) / 2,
    });
    if let Some(owner_rc) = platform_window::window_rect(owner_hwnd) {
        anchor.x = owner_rc.left + ((owner_rc.right - owner_rc.left) / 2);
        anchor.y = owner_rc.top + ((owner_rc.bottom - owner_rc.top) / 2);
    }
    let work = platform_monitor::nearest_work_rect_for_point(anchor);
    set_settings_ui_dpi(platform_dpi::layout_dpi_for_point(anchor));
    let settings_w = settings_w_scaled();
    let settings_h = settings_h_scaled();
    let x = max(
        work.left,
        work.left + ((work.right - work.left - settings_w) / 2),
    );
    let y = max(
        work.top,
        work.top + ((work.bottom - work.top - settings_h) / 2),
    );
    let mut settings_host = WindowsSettingsWindowHost::new(Some(settings_wnd_proc));
    if let NativeSettingsWindowPresentation::Created(settings_hwnd) = settings_host
        .present_settings_window(NativeSettingsWindowRequest {
            owner: owner_hwnd,
            existing: None,
            bounds: UiRect::new(x, y, x + settings_w, y + settings_h),
        })
    {
        app.settings_hwnd = settings_hwnd;
        app.edge_hide_pending_until = None;
        app.edge_hide_grace_until = None;
        WindowsMainWindowHost::new(Some(wnd_proc)).hide_main_window(owner_hwnd);
        refresh_low_level_input_hooks();
    }
}

pub(super) fn set_settings_window_bounds(hwnd: HWND, bounds: UiRect) {
    WindowsSettingsWindowHost::new(Some(settings_wnd_proc))
        .set_settings_window_bounds(hwnd, bounds);
}

pub(super) fn destroy_settings_window(hwnd: HWND) {
    WindowsSettingsWindowHost::new(Some(settings_wnd_proc)).destroy_settings_window(hwnd);
}

pub(super) fn focus_settings_window(hwnd: HWND) {
    WindowsSettingsWindowHost::new(Some(settings_wnd_proc)).focus_settings_window(hwnd);
}

pub(super) fn capture_settings_pointer(hwnd: HWND) {
    WindowsSettingsWindowHost::new(Some(settings_wnd_proc)).capture_settings_pointer(hwnd);
}

pub(super) fn release_settings_pointer(hwnd: HWND) {
    WindowsSettingsWindowHost::new(Some(settings_wnd_proc)).release_settings_pointer(hwnd);
}

pub(super) fn repaint_settings_window_area(hwnd: HWND, area: Option<UiRect>, erase: bool) {
    let _ = settings_window_request_area_repaint(hwnd, area, erase);
}

pub(super) fn repaint_settings_window(hwnd: HWND, erase: bool) {
    repaint_settings_window_area(hwnd, None, erase);
}

pub(super) fn request_settings_window_repaint(hwnd: HWND) {
    let _ = WindowsSettingsWindowHost::new(Some(settings_wnd_proc))
        .request_settings_window_repaint(hwnd);
}

pub(super) unsafe fn refresh_settings_cloud_page_after_lan_sync(settings_hwnd: HWND) {
    if !WindowsWindowIdentityHost::new().exists(settings_hwnd) {
        return;
    }
    let st_ptr = platform_window::user_data(settings_hwnd) as *mut SettingsWndState;
    if !st_ptr.is_null() && (*st_ptr).ui.is_built(SettingsPage::Cloud.index()) {
        if settings_refresh_cloud_lan_runtime_state(&mut *st_ptr) {
            if let Some(client) = settings_window_client_bounds(settings_hwnd).map(RECT::from) {
                let viewport = settings_viewport_rect(&client);
                repaint_settings_window_area(settings_hwnd, Some(viewport.into()), false);
            }
        }
    } else {
        request_settings_window_repaint(settings_hwnd);
    }
}

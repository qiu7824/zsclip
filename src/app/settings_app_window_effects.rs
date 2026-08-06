use super::prelude::*;
use crate::platform::gdi as platform_gdi;

pub(super) unsafe fn settings_refresh_windows_after_commit(
    st: &mut SettingsWndState,
    app: &mut AppState,
    baseline: &SettingsAppEffectBaseline,
) {
    if baseline.dark_mode_enabled != app.settings.dark_mode_enabled {
        platform_appearance::set_dark_mode_enabled(app.settings.dark_mode_enabled);
        platform_appearance::init_dark_mode_for_process();
        app.theme = Theme::default();
        refresh_search_theme_resources(app);
        WindowsMainWindowHost::new(Some(wnd_proc)).apply_main_window_appearance(st.parent_hwnd);
        settings_refresh_theme_resources(st);
        platform_appearance::apply_dark_mode_to_window(app.settings_hwnd);
        repaint_settings_window(app.settings_hwnd, true);
    }
    if baseline.edge_auto_hide && !app.settings.edge_auto_hide {
        restore_edge_hidden_window(st.parent_hwnd, app);
    } else if !baseline.edge_auto_hide
        && app.settings.edge_auto_hide
        && platform_window::is_visible(st.parent_hwnd)
    {
        clear_edge_dock_state(app);
        note_window_moved_for_edge_hide(st.parent_hwnd, app);
    }
    refresh_low_level_input_hooks();
    app.refilter();
    if baseline.persistent_search_box != app.settings.persistent_search_box {
        prepare_search_ui_for_show(st.parent_hwnd, app);
    }
    layout_children(st.parent_hwnd);
    sync_peer_windows_from_settings(st.parent_hwnd);
    platform_gdi::invalidate_rect(st.parent_hwnd, null(), 1);
}

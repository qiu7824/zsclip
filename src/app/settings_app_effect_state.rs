use super::prelude::*;

pub(super) struct SettingsAppEffectBaseline {
    pub(super) grouping_enabled: bool,
    pub(super) auto_start: bool,
    pub(super) tray_icon_enabled: bool,
    pub(super) hotkey: String,
    pub(super) plain_hotkey: String,
    pub(super) edge_auto_hide: bool,
    pub(super) vv_mode_enabled: bool,
    pub(super) persistent_search_box: bool,
}

impl SettingsAppEffectBaseline {
    pub(super) fn capture(app: &AppState) -> Self {
        Self {
            grouping_enabled: app.settings.grouping_enabled,
            auto_start: app.settings.auto_start,
            tray_icon_enabled: app.settings.tray_icon_enabled,
            hotkey: format!(
                "{}+{}+{}",
                app.settings.hotkey_enabled, app.settings.hotkey_mod, app.settings.hotkey_key
            ),
            plain_hotkey: format!(
                "{}+{}+{}",
                app.settings.plain_paste_hotkey_enabled,
                app.settings.plain_paste_hotkey_mod,
                app.settings.plain_paste_hotkey_key
            ),
            edge_auto_hide: app.settings.edge_auto_hide,
            vv_mode_enabled: app.settings.vv_mode_enabled,
            persistent_search_box: app.settings.persistent_search_box,
        }
    }
}

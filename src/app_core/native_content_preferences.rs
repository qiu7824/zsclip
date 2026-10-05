use super::{ClipKind, NativeHostClipListItemProjection, NativePopupMenuEntry};

pub(crate) const NATIVE_RENAME_PHRASE_COMMAND_ID: usize = 41021;

pub(crate) fn native_settings_profile(saved: &serde_json::Value) -> serde_json::Value {
    let mut values = serde_json::Map::new();
    let enabled = [
        "hotkey_enabled",
        "tray_icon_enabled",
        "app_icon_visible",
        "clipboard_capture_enabled",
        "close_without_exit",
        "vv_mode_enabled",
        "image_preview_enabled",
        "card_border_enabled",
        "card_shadow_enabled",
        "rich_text_clipboard_enabled",
        "quick_delete_button",
        "dedupe_filter_enabled",
        "grouping_enabled",
        "phrase_titles_enabled",
    ];
    for key in [
        "auto_start",
        "silent_start",
        "tray_icon_enabled",
        "app_icon_visible",
        "clipboard_capture_enabled",
        "close_without_exit",
        "auto_hide_on_blur",
        "edge_auto_hide",
        "hover_preview",
        "vv_mode_enabled",
        "image_preview_enabled",
        "quick_delete_button",
        "context_menu_copy_enabled",
        "click_hide",
        "move_pasted_item_to_top",
        "dedupe_filter_enabled",
        "persistent_search_box",
        "copy_success_sound_enabled",
        "paste_success_sound_enabled",
        "dark_mode_enabled",
        "rich_text_clipboard_enabled",
        "show_pin_button",
        "phrase_titles_enabled",
        "card_view_enabled",
        "card_border_enabled",
        "card_shadow_enabled",
        "paste_target_skip_enabled",
        "hotkey_enabled",
        "mouse_side_button_enabled",
        "plain_paste_hotkey_enabled",
        "quick_search_enabled",
        "super_mail_merge_enabled",
        "wps_taskpane_enabled",
        "grouping_enabled",
        "group_type_filter_enabled",
        "cloud_sync_enabled",
        "lan_sync_enabled",
        "qq_cloud_menu_enabled",
        "ai_clean_enabled",
        "qr_quick_enabled",
    ] {
        values.insert(key.into(), serde_json::Value::Bool(enabled.contains(&key)));
    }
    for (key, value) in [
        ("content_font_size", 0),
        ("image_row_height", 132),
        ("text_row_height", 44),
        ("file_row_height", 44),
        ("max_items", 200),
        ("show_mouse_dx", 12),
        ("show_mouse_dy", 12),
        ("show_fixed_x", 120),
        ("show_fixed_y", 120),
        ("vv_source_tab", 0),
        ("vv_group_id", 0),
        ("lan_tcp_port", 38473),
    ] {
        values.insert(key.into(), serde_json::Value::from(value));
    }
    for (key, value) in [
        ("show_pos_mode", "mouse"),
        ("paste_success_sound_kind", "default"),
        ("hotkey_mod", "Win"),
        ("hotkey_key", "V"),
        ("plain_paste_hotkey_mod", "Ctrl+Shift"),
        ("plain_paste_hotkey_key", "V"),
        ("mouse_side_button_1_action", "quick_window"),
        ("mouse_side_button_2_action", "vv_mode"),
        ("search_engine", "jzxx"),
        ("image_ocr_provider", "off"),
        ("text_translate_provider", "off"),
        ("text_translate_target_lang", "zh"),
        ("cloud_sync_interval", "1小时"),
        ("cloud_remote_dir", "ZSClip"),
        ("lan_receive_mode", "records_only"),
        ("lan_sync_mode", "manual"),
    ] {
        values.insert(key.into(), serde_json::Value::String(value.into()));
    }
    if let Some(saved) = saved.as_object() {
        values.extend(
            saved
                .iter()
                .map(|(key, value)| (key.clone(), value.clone())),
        );
    }
    serde_json::Value::Object(values)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NativeContentPreferences {
    pub content_font_size: i32,
    pub phrase_titles_enabled: bool,
    pub card_view_enabled: bool,
    pub card_border_enabled: bool,
    pub card_shadow_enabled: bool,
}

impl Default for NativeContentPreferences {
    fn default() -> Self {
        Self {
            content_font_size: 12,
            phrase_titles_enabled: true,
            card_view_enabled: false,
            card_border_enabled: true,
            card_shadow_enabled: true,
        }
    }
}

impl NativeContentPreferences {
    pub(crate) fn from_json(value: &serde_json::Value) -> Self {
        let boolean = |key: &str, default| {
            value
                .get(key)
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(default)
        };
        let size = value
            .get("content_font_size")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0);
        Self {
            content_font_size: if [12, 14, 16, 18, 20].contains(&size) {
                size as i32
            } else {
                12
            },
            phrase_titles_enabled: boolean("phrase_titles_enabled", true),
            card_view_enabled: boolean("card_view_enabled", false),
            card_border_enabled: boolean("card_border_enabled", true),
            card_shadow_enabled: boolean("card_shadow_enabled", true),
        }
    }

    pub(crate) fn apply_projection(
        self,
        mut items: Vec<NativeHostClipListItemProjection>,
    ) -> Vec<NativeHostClipListItemProjection> {
        if !self.phrase_titles_enabled {
            for item in &mut items {
                if item.kind == ClipKind::Phrase {
                    item.title.clear();
                }
            }
        }
        items
    }

    pub(crate) fn row_height(self) -> f64 {
        (self.content_font_size as f64 * 2.0 + 18.0).max(44.0)
    }

    pub(crate) fn rename_phrase_menu_entry(
        self,
        kind: ClipKind,
        label: impl Into<String>,
    ) -> Option<NativePopupMenuEntry> {
        (self.phrase_titles_enabled && kind == ClipKind::Phrase).then(|| {
            NativePopupMenuEntry::Command {
                id: NATIVE_RENAME_PHRASE_COMMAND_ID,
                label: label.into(),
                enabled: true,
                checked: false,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "windows")]
    #[test]
    fn shared_native_boolean_defaults_match_the_saved_windows_model() {
        let windows = serde_json::to_value(crate::app::state::AppSettings::default()).unwrap();
        let native = native_settings_profile(&serde_json::json!({}));
        for (name, value) in native.as_object().unwrap() {
            if value.is_boolean() {
                if let Some(saved_default) = windows.get(name) {
                    assert_eq!(value, saved_default, "default drift for {name}");
                }
            }
        }
    }
    #[test]
    fn native_profile_defaults_preserve_enabled_legacy_flags_and_explicit_off_values() {
        let profile = native_settings_profile(
            &serde_json::json!({"hotkey_enabled":false,"rich_text_clipboard_enabled":false,"phrase_titles_enabled":false}),
        );
        for key in [
            "hotkey_enabled",
            "rich_text_clipboard_enabled",
            "phrase_titles_enabled",
        ] {
            assert_eq!(profile[key], false);
        }
        for key in [
            "dedupe_filter_enabled",
            "clipboard_capture_enabled",
            "vv_mode_enabled",
            "card_border_enabled",
        ] {
            assert_eq!(profile[key], true);
        }
        assert_eq!(profile["card_view_enabled"], false);
    }
    #[test]
    fn native_preferences_keep_independent_card_flags_and_legacy_defaults() {
        let defaults = NativeContentPreferences::from_json(&serde_json::json!({}));
        assert_eq!(defaults.content_font_size, 12);
        assert!(defaults.phrase_titles_enabled);
        assert!(!defaults.card_view_enabled);
        assert!(defaults.card_border_enabled);
        assert!(defaults.card_shadow_enabled);
        for border in [false, true] {
            for shadow in [false, true] {
                let prefs = NativeContentPreferences::from_json(
                    &serde_json::json!({"content_font_size":20,"card_view_enabled":true,"card_border_enabled":border,"card_shadow_enabled":shadow}),
                );
                assert_eq!(
                    (prefs.card_border_enabled, prefs.card_shadow_enabled),
                    (border, shadow)
                );
                assert!(prefs.row_height() >= 2.0 * prefs.content_font_size as f64);
            }
        }
    }
    #[test]
    fn disabling_titles_changes_only_projection_and_preserves_body_and_source() {
        let source = vec![
            NativeHostClipListItemProjection::with_metadata(
                1,
                "Saved title",
                "Full body preview",
                ClipKind::Phrase,
                false,
            ),
            NativeHostClipListItemProjection::new(2, "Browser", "Ordinary preview"),
        ];
        let off = NativeContentPreferences::from_json(
            &serde_json::json!({"phrase_titles_enabled":false}),
        )
        .apply_projection(source.clone());
        assert!(off[0].title.is_empty());
        assert_eq!(off[0].preview, source[0].preview);
        assert_eq!(off[1], source[1]);
        assert_eq!(source[0].title, "Saved title");
        let on = NativeContentPreferences::from_json(&serde_json::json!({}))
            .apply_projection(source.clone());
        assert_eq!(on, source);
    }
}

use super::{ClipKind, NativeHostClipListItemProjection, NativePopupMenuEntry};

pub(crate) const NATIVE_RENAME_PHRASE_COMMAND_ID: usize = 41021;

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

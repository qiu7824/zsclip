use serde_json::Value;
use std::collections::BTreeSet;

pub(crate) const CTRL: u8 = 1;
pub(crate) const ALT: u8 = 2;
pub(crate) const SHIFT: u8 = 4;
pub(crate) const SUPER: u8 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativeHotkeyAction {
    ShowMain,
    PastePlain,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NativeHotkeyBinding {
    pub(crate) action: NativeHotkeyAction,
    pub(crate) key: String,
    pub(crate) modifiers: u8,
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub(crate) struct NativeHotkeyBindings {
    pub(crate) bindings: Vec<NativeHotkeyBinding>,
    pub(crate) errors: Vec<String>,
}

impl NativeHotkeyBindings {
    pub(crate) fn from_settings(settings: &Value, macos: bool) -> Self {
        let mut result = Self::default();
        for (action, enabled, modifier, key, default_enabled, default_modifier) in [
            (
                NativeHotkeyAction::ShowMain,
                "hotkey_enabled",
                "hotkey_mod",
                "hotkey_key",
                true,
                if macos { "Ctrl+Alt" } else { "Win" },
            ),
            (
                NativeHotkeyAction::PastePlain,
                "plain_paste_hotkey_enabled",
                "plain_paste_hotkey_mod",
                "plain_paste_hotkey_key",
                false,
                "Ctrl+Shift",
            ),
        ] {
            let enabled = match settings.get(enabled) {
                None => default_enabled,
                Some(Value::Bool(value)) => *value,
                Some(_) => {
                    result
                        .errors
                        .push(format!("{action:?}: invalid enable value"));
                    continue;
                }
            };
            if !enabled {
                continue;
            }
            let modifier = match settings.get(modifier) {
                None => default_modifier,
                Some(Value::String(value)) => value.trim(),
                Some(_) => {
                    result
                        .errors
                        .push(format!("{action:?}: invalid modifier value"));
                    continue;
                }
            };
            let key = match settings.get(key) {
                None => "V",
                Some(Value::String(value)) => value.trim(),
                Some(_) => {
                    result.errors.push(format!("{action:?}: invalid key value"));
                    continue;
                }
            };
            let modifiers = match modifier {
                "Ctrl" => CTRL,
                "Alt" => ALT,
                "Shift" => SHIFT,
                "Win" => SUPER,
                "Ctrl+Alt" => CTRL | ALT,
                "Ctrl+Shift" => CTRL | SHIFT,
                "Alt+Shift" => ALT | SHIFT,
                "Ctrl+Alt+Shift" => CTRL | ALT | SHIFT,
                _ => {
                    result
                        .errors
                        .push(format!("{action:?}: unsupported modifier {modifier}"));
                    continue;
                }
            };
            if !crate::settings_model::HOTKEY_KEY_OPTIONS.contains(&key) {
                result
                    .errors
                    .push(format!("{action:?}: unsupported key {key}"));
                continue;
            }
            // Never steal the standard editing/application commands from every
            // other app. Saved values remain intact; invalid bindings stay idle.
            if macos
                && modifiers == SUPER
                && matches!(
                    key,
                    "A" | "C"
                        | "V"
                        | "X"
                        | "Z"
                        | "Y"
                        | "F"
                        | "H"
                        | "M"
                        | "Q"
                        | "W"
                        | "S"
                        | "P"
                        | "O"
                        | "N"
                        | "Tab"
                        | "Space"
                        | "Backspace"
                )
            {
                result
                    .errors
                    .push(format!("{action:?}: Command+{key} is reserved by macOS"));
                continue;
            }
            if macos && key == "Insert" {
                result.errors.push(format!(
                    "{action:?}: Insert is unavailable on this keyboard backend"
                ));
                continue;
            }
            if result
                .bindings
                .iter()
                .any(|binding| binding.key == key && binding.modifiers == modifiers)
            {
                result
                    .errors
                    .push(format!("{action:?}: duplicate global shortcut"));
                continue;
            }
            result.bindings.push(NativeHotkeyBinding {
                action,
                key: key.into(),
                modifiers,
            });
        }
        result
    }
}

#[derive(Default, Debug)]
pub(crate) struct NativeHotkeyCycles {
    owned: BTreeSet<u16>,
}

#[derive(Default, Debug, PartialEq, Eq)]
pub(crate) struct NativeHotkeyDecision {
    pub(crate) consume: bool,
    pub(crate) action: Option<NativeHotkeyAction>,
}

impl NativeHotkeyCycles {
    pub(crate) fn handle(
        &mut self,
        bindings: &NativeHotkeyBindings,
        key_code: u16,
        key_label: &str,
        modifiers: u8,
        down: bool,
        repeat: bool,
    ) -> NativeHotkeyDecision {
        if !down {
            return NativeHotkeyDecision {
                consume: self.owned.remove(&key_code),
                action: None,
            };
        }
        if self.owned.contains(&key_code) {
            return NativeHotkeyDecision {
                consume: true,
                action: None,
            };
        }
        if repeat {
            return NativeHotkeyDecision::default();
        }
        let Some(binding) = bindings.bindings.iter().find(|binding| {
            binding.key.eq_ignore_ascii_case(key_label) && binding.modifiers == modifiers
        }) else {
            return NativeHotkeyDecision::default();
        };
        self.owned.insert(key_code);
        NativeHotkeyDecision {
            consume: true,
            action: Some(binding.action),
        }
    }
}

pub(crate) fn plain_text_for_item(item: &crate::app_core::ClipItem) -> Option<String> {
    use crate::app_core::ClipKind;
    let text = match item.kind {
        ClipKind::Text | ClipKind::Phrase => item.text.clone(),
        ClipKind::Files => item
            .file_paths
            .as_ref()
            .map(|paths| paths.join("\n"))
            .or_else(|| item.text.clone()),
        ClipKind::Image => None,
    }?;
    Some(text.replace("\r\n", "\n").replace('\r', "\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn platform_defaults_and_explicit_reserved_shortcuts_are_distinct() {
        let empty = serde_json::json!({});
        assert_eq!(
            NativeHotkeyBindings::from_settings(&empty, true).bindings[0].modifiers,
            CTRL | ALT
        );
        assert_eq!(
            NativeHotkeyBindings::from_settings(&empty, false).bindings[0].modifiers,
            SUPER
        );
        for key in ["V", "C", "X", "A", "Z"] {
            let saved = serde_json::json!({"hotkey_mod":"Win", "hotkey_key":key});
            let original = saved.clone();
            let parsed = NativeHotkeyBindings::from_settings(&saved, true);
            assert!(parsed.bindings.is_empty());
            assert_eq!(parsed.errors.len(), 1);
            assert_eq!(saved, original);
        }
    }
    #[test]
    fn owned_cycle_survives_configuration_and_modifier_changes_without_retrigger() {
        let bindings = NativeHotkeyBindings::from_settings(&serde_json::json!({}), true);
        let mut cycles = NativeHotkeyCycles::default();
        let pressed = cycles.handle(&bindings, 9, "V", CTRL | ALT, true, false);
        assert_eq!(pressed.action, Some(NativeHotkeyAction::ShowMain));
        let disabled = NativeHotkeyBindings::default();
        assert!(cycles.handle(&disabled, 9, "V", 0, true, true).consume);
        assert!(cycles.handle(&disabled, 9, "V", 0, false, false).consume);
        assert!(!cycles.handle(&disabled, 9, "V", 0, false, false).consume);
        assert!(
            !cycles
                .handle(&bindings, 9, "V", CTRL | ALT | SHIFT, true, false)
                .consume
        );
        assert!(
            !cycles
                .handle(&bindings, 9, "V", CTRL | ALT, true, true)
                .consume
        );
    }
    #[test]
    fn distinct_plain_binding_and_invalid_configuration_are_not_silently_normalized() {
        let settings = serde_json::json!({"plain_paste_hotkey_enabled":true});
        let bindings = NativeHotkeyBindings::from_settings(&settings, true);
        let mut cycles = NativeHotkeyCycles::default();
        assert_eq!(
            cycles
                .handle(&bindings, 9, "v", CTRL | SHIFT, true, false)
                .action,
            Some(NativeHotkeyAction::PastePlain)
        );
        assert!(NativeHotkeyBindings::from_settings(
            &serde_json::json!({"hotkey_key":"unknown"}),
            true
        )
        .bindings
        .is_empty());
        let duplicate = NativeHotkeyBindings::from_settings(
            &serde_json::json!({"hotkey_mod":"Ctrl+Shift","plain_paste_hotkey_enabled":true}),
            true,
        );
        assert_eq!(duplicate.bindings.len(), 1);
        assert_eq!(duplicate.errors.len(), 1);
        for invalid in [
            serde_json::json!({"hotkey_enabled":"true"}),
            serde_json::json!({"hotkey_key":1}),
            serde_json::json!({"hotkey_mod":false}),
        ] {
            let parsed = NativeHotkeyBindings::from_settings(&invalid, true);
            assert!(parsed.bindings.is_empty());
            assert_eq!(parsed.errors.len(), 1);
        }
    }
    #[test]
    fn plain_mode_uses_body_or_file_paths_and_never_an_image_fallback() {
        use crate::app_core::{ClipItem, ClipKind};
        let mut item = ClipItem {
            id: 7,
            kind: ClipKind::Phrase,
            preview: "summary".into(),
            phrase_title: "A separate title".into(),
            text: Some("first\r\nsecond\rlast".into()),
            rich_text_html: Some("<b>first</b>".into()),
            source_app: String::new(),
            file_paths: None,
            image_bytes: None,
            image_path: None,
            image_width: 0,
            image_height: 0,
            pinned: false,
            group_id: 0,
            created_at: String::new(),
        };
        assert_eq!(
            plain_text_for_item(&item).as_deref(),
            Some("first\nsecond\nlast")
        );
        item.kind = ClipKind::Files;
        item.file_paths = Some(vec!["/one".into(), "/two".into()]);
        assert_eq!(plain_text_for_item(&item).as_deref(), Some("/one\n/two"));
        item.kind = ClipKind::Image;
        assert_eq!(plain_text_for_item(&item), None);
    }
}

use super::prelude::*;
use crate::win_system_ui::{settings_host_set_enabled, settings_host_text};

pub(super) fn multi_sync_mode_from_settings(settings: &AppSettings) -> &'static str {
    crate::settings_model::multi_sync_mode_from_flags(
        settings.cloud_sync_enabled,
        settings.lan_sync_enabled,
    )
}

pub(super) fn settings_apply_multi_sync_mode(settings: &mut AppSettings, mode: &str) {
    let (cloud, lan) = crate::settings_model::multi_sync_flags_for_mode(mode);
    settings.cloud_sync_enabled = cloud;
    settings.lan_sync_enabled = lan;
}

pub(super) fn settings_normalize_multi_sync_mode(settings: &mut AppSettings) {
    let (cloud, lan) = crate::settings_model::normalize_multi_sync_flags(
        settings.cloud_sync_enabled,
        settings.lan_sync_enabled,
    );
    settings.cloud_sync_enabled = cloud;
    settings.lan_sync_enabled = lan;
}

pub(super) unsafe fn settings_sync_page_state(st: &mut SettingsWndState, page: usize) {
    match SettingsPage::from_index(page) {
        SettingsPage::General => {
            if !st.cb_max.is_null() {
                settings_set_text(
                    st.cb_max,
                    settings_dropdown_label_for_max_items(st.draft.max_items),
                );
            }
            settings_sync_pos_fields_enabled(st);
            let sound_enabled = st.draft.paste_success_sound_enabled;
            if !st.cb_paste_sound.is_null() {
                settings_set_text(
                    st.cb_paste_sound,
                    &paste_sound_display(&st.draft.paste_success_sound_kind),
                );
                settings_host_set_enabled(st.cb_paste_sound, sound_enabled);
            }
            if !st.btn_paste_sound_pick.is_null() {
                settings_set_text(
                    st.btn_paste_sound_pick,
                    &paste_sound_file_button_text(&st.draft.paste_success_sound_path),
                );
                settings_host_set_enabled(
                    st.btn_paste_sound_pick,
                    sound_enabled && st.draft.paste_success_sound_kind == "custom",
                );
            }
            let skip_enabled = st.draft.paste_target_skip_enabled;
            if !st.ed_skip_class_names.is_null() {
                settings_host_set_enabled(st.ed_skip_class_names, skip_enabled);
            }
            if !st.btn_capture_skip_window.is_null() {
                settings_host_set_enabled(st.btn_capture_skip_window, skip_enabled);
            }
        }
        SettingsPage::Hotkey => {
            let s = &st.draft;
            settings_set_text(st.cb_hk_mod, &normalize_hotkey_mod(&s.hotkey_mod));
            settings_set_text(st.cb_hk_key, &normalize_hotkey_key(&s.hotkey_key));
            settings_set_text(
                st.lb_hk_preview,
                &hotkey_preview_text(&s.hotkey_mod, &s.hotkey_key),
            );
            if !st.btn_hk_record.is_null() {
                settings_set_text(
                    st.btn_hk_record,
                    if st.hotkey_recording {
                        tr("按下快捷键...", "Press shortcut...")
                    } else {
                        tr("录制热键", "Record Hotkey")
                    },
                );
            }
            if !st.cb_plain_hk_mod.is_null() {
                settings_set_text(
                    st.cb_plain_hk_mod,
                    &normalize_hotkey_mod(&s.plain_paste_hotkey_mod),
                );
                settings_host_set_enabled(st.cb_plain_hk_mod, s.plain_paste_hotkey_enabled);
            }
            if !st.cb_plain_hk_key.is_null() {
                settings_set_text(
                    st.cb_plain_hk_key,
                    &normalize_hotkey_key(&s.plain_paste_hotkey_key),
                );
                settings_host_set_enabled(st.cb_plain_hk_key, s.plain_paste_hotkey_enabled);
            }
            if !st.lb_plain_hk_preview.is_null() {
                settings_set_text(
                    st.lb_plain_hk_preview,
                    &hotkey_preview_text(&s.plain_paste_hotkey_mod, &s.plain_paste_hotkey_key),
                );
            }
        }
        SettingsPage::Plugin => settings_sync_plugin_page_state(st),
        SettingsPage::Group => settings_sync_group_page(st),
        SettingsPage::Cloud => settings_sync_cloud_page_state(st),
        SettingsPage::About => {}
    }
    settings_invalidate_page_ctrls(st.parent_hwnd, st, page);
}

pub(super) unsafe fn settings_sync_pos_fields_enabled(st: &SettingsWndState) {
    let edge_hide = st.draft.edge_auto_hide;
    let mode = settings_dropdown_pos_mode_from_label(&settings_host_text(st.cb_pos));
    let is_follow = !edge_hide && mode == "mouse";
    let is_fixed = !edge_hide && mode == "fixed";
    if !st.cb_pos.is_null() {
        settings_host_set_enabled(st.cb_pos, !edge_hide);
    }
    if !st.ed_dx.is_null() {
        settings_host_set_enabled(st.ed_dx, is_follow);
    }
    if !st.ed_dy.is_null() {
        settings_host_set_enabled(st.ed_dy, is_follow);
    }
    if !st.ed_fx.is_null() {
        settings_host_set_enabled(st.ed_fx, is_fixed);
    }
    if !st.ed_fy.is_null() {
        settings_host_set_enabled(st.ed_fy, is_fixed);
    }
}

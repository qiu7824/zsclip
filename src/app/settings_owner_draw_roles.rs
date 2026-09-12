use super::prelude::*;
use crate::win_system_ui::SettingsComponentKind;

pub(super) fn settings_owner_draw_is_qr(cid: isize) -> bool {
    matches!(cid, IDC_SET_LAN_QR_ANDROID | IDC_SET_LAN_QR_IOS)
}

pub(super) fn settings_owner_draw_is_toggle(cid: isize) -> bool {
    matches!(
        crate::settings_ui_host::settings_control_role_for_control(cid),
        Some(crate::app_core::SettingsControlRole::Toggle)
    )
}

pub(super) fn settings_owner_draw_button_kind(
    st: &SettingsWndState,
    cid: isize,
) -> SettingsComponentKind {
    if settings_owner_draw_is_dropdown(cid) {
        SettingsComponentKind::Dropdown
    } else if settings_owner_draw_is_accent(st, cid) {
        SettingsComponentKind::AccentButton
    } else {
        SettingsComponentKind::Button
    }
}

fn settings_owner_draw_is_dropdown(cid: isize) -> bool {
    matches!(
        crate::settings_ui_host::settings_control_role_for_control(cid),
        Some(crate::app_core::SettingsControlRole::Dropdown)
    )
}

fn settings_owner_draw_is_accent(st: &SettingsWndState, cid: isize) -> bool {
    (cid == IDC_SET_GROUP_VIEW_RECORDS && settings_group_view_current(st) == 0)
        || (cid == IDC_SET_GROUP_VIEW_PHRASES && settings_group_view_current(st) == 1)
        || cid == IDC_SET_SAVE
}

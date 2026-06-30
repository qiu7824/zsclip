use super::prelude::*;
use crate::win_system_ui::SettingsComponentKind;

pub(super) fn settings_owner_draw_is_qr(cid: isize) -> bool {
    matches!(cid, IDC_SET_LAN_QR_ANDROID | IDC_SET_LAN_QR_IOS)
}

pub(super) fn settings_owner_draw_is_toggle(cid: isize) -> bool {
    matches!(
        cid,
        IDC_SET_AUTOSTART
            | IDC_SET_SILENTSTART
            | IDC_SET_TRAYICON
            | IDC_SET_CLOSETRAY
            | IDC_SET_CLICK_HIDE
            | IDC_SET_PASTE_MOVE_TOP
            | IDC_SET_DEDUPE_FILTER
            | IDC_SET_PERSIST_SEARCH
            | IDC_SET_PASTE_SOUND_ENABLE
            | IDC_SET_SKIP_WINDOW_ENABLE
            | IDC_SET_AUTOHIDE_BLUR
            | IDC_SET_EDGEHIDE
            | IDC_SET_HOVERPREVIEW
            | IDC_SET_VV_MODE
            | IDC_SET_IMAGE_PREVIEW
            | IDC_SET_QUICK_DELETE
            | IDC_SET_GROUP_ENABLE
            | IDC_SET_GROUP_TYPE_FILTER
            | IDC_SET_CLOUD_ENABLE
            | IDC_SET_LAN_ENABLE
            | 6101
            | 7102
            | 7106
            | 7101
            | 7103
            | 7104
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
        cid,
        IDC_SET_MAX
            | IDC_SET_POSMODE
            | IDC_SET_MULTI_SYNC_MODE
            | IDC_SET_CLOUD_INTERVAL
            | IDC_SET_PASTE_SOUND_KIND
            | IDC_SET_VV_GROUP
            | IDC_SET_VV_SOURCE
            | 6102
            | 6103
            | 7201
    )
}

fn settings_owner_draw_is_accent(st: &SettingsWndState, cid: isize) -> bool {
    (cid == IDC_SET_GROUP_VIEW_RECORDS && settings_group_view_current(st) == 0)
        || (cid == IDC_SET_GROUP_VIEW_PHRASES && settings_group_view_current(st) == 1)
        || cid == IDC_SET_SAVE
}

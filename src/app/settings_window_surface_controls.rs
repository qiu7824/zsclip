use super::prelude::*;

pub(super) fn is_settings_surface_control(id: isize) -> bool {
    matches!(
        crate::settings_ui_host::settings_control_role_for_control(id),
        Some(
            crate::app_core::SettingsControlRole::Toggle
                | crate::app_core::SettingsControlRole::Dropdown
        )
    ) || is_general_surface_control(id)
        || is_hotkey_surface_control(id)
        || is_group_surface_control(id)
        || is_cloud_surface_control(id)
        || is_plugin_surface_control(id)
        || is_about_surface_control(id)
}

fn is_general_surface_control(id: isize) -> bool {
    matches!(
        id,
        IDC_SET_AUTOSTART
            | IDC_SET_SILENTSTART
            | IDC_SET_TRAYICON
            | IDC_SET_SHOW_PIN_BUTTON
            | IDC_SET_APP_ICON_VISIBLE
            | IDC_SET_DARK_MODE
            | IDC_SET_CLOSETRAY
            | IDC_SET_CLICK_HIDE
            | IDC_SET_PASTE_MOVE_TOP
            | IDC_SET_DEDUPE_FILTER
            | IDC_SET_PERSIST_SEARCH
            | IDC_SET_COPY_SOUND_ENABLE
            | IDC_SET_PASTE_SOUND_ENABLE
            | IDC_SET_SKIP_WINDOW_ENABLE
            | IDC_SET_RICH_TEXT
            | IDC_SET_AUTOHIDE_BLUR
            | IDC_SET_EDGEHIDE
            | IDC_SET_HOVERPREVIEW
            | IDC_SET_VV_MODE
            | IDC_SET_IMAGE_PREVIEW
            | IDC_SET_QUICK_DELETE
            | IDC_SET_CONTEXT_MENU_COPY
            | IDC_SET_MAX
            | IDC_SET_IMAGE_ROW_HEIGHT
            | IDC_SET_TEXT_ROW_HEIGHT
            | IDC_SET_FILE_ROW_HEIGHT
            | IDC_SET_POSMODE
            | IDC_SET_PASTE_SOUND_KIND
            | IDC_SET_PASTE_SOUND_PICK
            | IDC_SET_SKIP_WINDOW_CAPTURE
    )
}

fn is_hotkey_surface_control(id: isize) -> bool {
    matches!(
        id,
        IDC_SET_HK_RECORD
            | 6101
            | 6102
            | 6103
            | IDC_SET_PLAIN_HK_ENABLE
            | IDC_SET_PLAIN_HK_MOD
            | IDC_SET_PLAIN_HK_KEY
            | IDC_SET_MOUSE_SIDE_ENABLE
            | IDC_SET_MOUSE_SIDE_BUTTON_1
            | IDC_SET_MOUSE_SIDE_BUTTON_2
    )
}

fn is_group_surface_control(id: isize) -> bool {
    matches!(
        id,
        IDC_SET_GROUP_ENABLE
            | IDC_SET_GROUP_TYPE_FILTER
            | IDC_SET_VV_SOURCE
            | IDC_SET_VV_GROUP
            | IDC_SET_GROUP_VIEW_RECORDS
            | IDC_SET_GROUP_VIEW_PHRASES
            | IDC_SET_GROUP_ADD
            | IDC_SET_GROUP_RENAME
            | IDC_SET_GROUP_DELETE
            | IDC_SET_GROUP_UP
            | IDC_SET_GROUP_DOWN
    )
}

fn is_cloud_surface_control(id: isize) -> bool {
    matches!(
        id,
        IDC_SET_CLOUD_ENABLE
            | IDC_SET_MULTI_SYNC_MODE
            | IDC_SET_CLOUD_INTERVAL
            | IDC_SET_LAN_QR_ANDROID
            | IDC_SET_LAN_QR_IOS
            | IDC_SET_LAN_COPY_PAIR
            | IDC_SET_LAN_COPY_SETUP
    )
}

fn is_plugin_surface_control(id: isize) -> bool {
    matches!(
        id,
        6101 | 6102 | 6103 | 7101 | 7102 | 7103 | 7104 | 7106 | 7201
    )
}

fn is_about_surface_control(id: isize) -> bool {
    matches!(id, IDC_SET_OPEN_SOURCE | IDC_SET_OPEN_UPDATE)
}

use super::prelude::*;

pub(super) fn is_settings_surface_control(id: isize) -> bool {
    is_general_surface_control(id)
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
            | IDC_SET_MAX
            | IDC_SET_POSMODE
            | IDC_SET_PASTE_SOUND_KIND
            | IDC_SET_PASTE_SOUND_PICK
            | IDC_SET_SKIP_WINDOW_CAPTURE
    )
}

fn is_hotkey_surface_control(id: isize) -> bool {
    matches!(
        id,
        IDC_SET_HK_RECORD | IDC_SET_PLAIN_HK_ENABLE | IDC_SET_PLAIN_HK_MOD | IDC_SET_PLAIN_HK_KEY
    )
}

fn is_group_surface_control(id: isize) -> bool {
    matches!(
        id,
        IDC_SET_GROUP_ENABLE | IDC_SET_VV_SOURCE | IDC_SET_VV_GROUP
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

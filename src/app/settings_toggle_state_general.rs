use super::prelude::*;

pub(super) fn settings_toggle_general_get(st: &SettingsWndState, cid: isize) -> Option<bool> {
    match cid {
        IDC_SET_AUTOSTART => Some(st.draft.auto_start),
        IDC_SET_SILENTSTART => Some(st.draft.silent_start),
        IDC_SET_TRAYICON => Some(st.draft.tray_icon_enabled),
        IDC_SET_CLOSETRAY => Some(st.draft.close_without_exit),
        IDC_SET_CLICK_HIDE => Some(st.draft.click_hide),
        IDC_SET_PASTE_MOVE_TOP => Some(st.draft.move_pasted_item_to_top),
        IDC_SET_DEDUPE_FILTER => Some(st.draft.dedupe_filter_enabled),
        IDC_SET_PERSIST_SEARCH => Some(st.draft.persistent_search_box),
        IDC_SET_PASTE_SOUND_ENABLE => Some(st.draft.paste_success_sound_enabled),
        IDC_SET_SKIP_WINDOW_ENABLE => Some(st.draft.paste_target_skip_enabled),
        IDC_SET_RICH_TEXT => Some(st.draft.rich_text_clipboard_enabled),
        IDC_SET_AUTOHIDE_BLUR => Some(st.draft.auto_hide_on_blur),
        IDC_SET_EDGEHIDE => Some(st.draft.edge_auto_hide),
        IDC_SET_HOVERPREVIEW => Some(st.draft.hover_preview),
        IDC_SET_VV_MODE => Some(st.draft.vv_mode_enabled),
        IDC_SET_IMAGE_PREVIEW => Some(st.draft.image_preview_enabled),
        IDC_SET_QUICK_DELETE => Some(st.draft.quick_delete_button),
        _ => None,
    }
}

pub(super) fn settings_toggle_general_flip(st: &mut SettingsWndState, cid: isize) -> bool {
    match cid {
        IDC_SET_AUTOSTART => st.draft.auto_start = !st.draft.auto_start,
        IDC_SET_SILENTSTART => st.draft.silent_start = !st.draft.silent_start,
        IDC_SET_TRAYICON => st.draft.tray_icon_enabled = !st.draft.tray_icon_enabled,
        IDC_SET_CLOSETRAY => st.draft.close_without_exit = !st.draft.close_without_exit,
        IDC_SET_CLICK_HIDE => st.draft.click_hide = !st.draft.click_hide,
        IDC_SET_PASTE_MOVE_TOP => {
            st.draft.move_pasted_item_to_top = !st.draft.move_pasted_item_to_top
        }
        IDC_SET_DEDUPE_FILTER => st.draft.dedupe_filter_enabled = !st.draft.dedupe_filter_enabled,
        IDC_SET_PERSIST_SEARCH => st.draft.persistent_search_box = !st.draft.persistent_search_box,
        IDC_SET_PASTE_SOUND_ENABLE => {
            st.draft.paste_success_sound_enabled = !st.draft.paste_success_sound_enabled
        }
        IDC_SET_SKIP_WINDOW_ENABLE => {
            st.draft.paste_target_skip_enabled = !st.draft.paste_target_skip_enabled
        }
        IDC_SET_RICH_TEXT => {
            st.draft.rich_text_clipboard_enabled = !st.draft.rich_text_clipboard_enabled
        }
        IDC_SET_AUTOHIDE_BLUR => st.draft.auto_hide_on_blur = !st.draft.auto_hide_on_blur,
        IDC_SET_EDGEHIDE => st.draft.edge_auto_hide = !st.draft.edge_auto_hide,
        IDC_SET_HOVERPREVIEW => st.draft.hover_preview = !st.draft.hover_preview,
        IDC_SET_VV_MODE => st.draft.vv_mode_enabled = !st.draft.vv_mode_enabled,
        IDC_SET_IMAGE_PREVIEW => st.draft.image_preview_enabled = !st.draft.image_preview_enabled,
        IDC_SET_QUICK_DELETE => st.draft.quick_delete_button = !st.draft.quick_delete_button,
        _ => return false,
    }
    true
}

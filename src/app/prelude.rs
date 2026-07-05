pub(in crate::app) use super::constants::*;
pub(in crate::app) use super::data::*;
pub(in crate::app) use super::main_clipboard_capture::*;
pub(in crate::app) use super::main_cloud_sync::*;
pub(in crate::app) use super::main_edge_auto_hide::*;
pub(in crate::app) use super::main_entry::{
    apply_search_filter, cancel_main_scroll_drag, handle_control_command, handle_vv_select,
    normalize_plain_text_for_paste, settings_set_hotkey_recording, stop_search_debounce_timer,
    wnd_proc,
};
pub(in crate::app) use super::main_events::*;
pub(in crate::app) use super::main_hover_preview::*;
pub(in crate::app) use super::main_input::*;
pub(in crate::app) use super::main_lan_sync::*;
pub(in crate::app) use super::main_low_level_input::*;
pub(in crate::app) use super::main_paste::*;
pub(in crate::app) use super::main_paste_target_discovery::*;
pub(in crate::app) use super::main_platform_bindings::*;
pub(in crate::app) use super::main_popup_menus::*;
pub(in crate::app) use super::main_renderer::*;
pub(in crate::app) use super::main_row_commands::*;
pub(in crate::app) use super::main_row_tools::*;
pub(in crate::app) use super::main_search::*;
pub(in crate::app) use super::main_search_host::*;
pub(in crate::app) use super::main_startup_integrations::*;
pub(in crate::app) use super::main_view_helpers::*;
pub(in crate::app) use super::main_window::*;
pub(in crate::app) use super::main_window_host::*;
pub(in crate::app) use super::main_window_refresh::*;
pub(in crate::app) use super::main_window_registry::*;
pub(in crate::app) use super::platform_helpers::*;
pub(in crate::app) use super::runtime::*;
pub(in crate::app) use super::settings_about_page::*;
pub(in crate::app) use super::settings_about_page_data::*;
pub(in crate::app) use super::settings_about_page_metadata::*;
pub(in crate::app) use super::settings_about_page_update::*;
pub(in crate::app) use super::settings_actions::*;
pub(in crate::app) use super::settings_app_apply::*;
pub(in crate::app) use super::settings_app_collect::*;
pub(in crate::app) use super::settings_app_collect_cloud::*;
pub(in crate::app) use super::settings_app_collect_general::*;
pub(in crate::app) use super::settings_app_collect_group::*;
pub(in crate::app) use super::settings_app_collect_hotkey::*;
pub(in crate::app) use super::settings_app_collect_plugin::*;
pub(in crate::app) use super::settings_app_data_effects::*;
pub(in crate::app) use super::settings_app_effect_state::*;
pub(in crate::app) use super::settings_app_effects::*;
pub(in crate::app) use super::settings_app_integration_effects::*;
pub(in crate::app) use super::settings_app_window_effects::*;
pub(in crate::app) use super::settings_cloud_page::*;
pub(in crate::app) use super::settings_cloud_page_lan::*;
pub(in crate::app) use super::settings_cloud_page_lan_devices::*;
pub(in crate::app) use super::settings_cloud_page_webdav::*;
pub(in crate::app) use super::settings_command_queue::*;
pub(in crate::app) use super::settings_control_factory::*;
pub(in crate::app) use super::settings_control_registry::*;
pub(in crate::app) use super::settings_control_selection::*;
pub(in crate::app) use super::settings_control_selection_cloud::*;
pub(in crate::app) use super::settings_control_selection_general::*;
pub(in crate::app) use super::settings_control_selection_group::*;
pub(in crate::app) use super::settings_control_selection_hotkey::*;
pub(in crate::app) use super::settings_control_selection_plugin::*;
pub(in crate::app) use super::settings_dropdown::*;
pub(in crate::app) use super::settings_dropdown_cloud::*;
pub(in crate::app) use super::settings_dropdown_general::*;
pub(in crate::app) use super::settings_dropdown_group::*;
pub(in crate::app) use super::settings_dropdown_host::*;
pub(in crate::app) use super::settings_dropdown_hotkey::*;
pub(in crate::app) use super::settings_dropdown_plugin::*;
pub(in crate::app) use super::settings_general_page::*;
pub(in crate::app) use super::settings_general_page_startup::*;
pub(in crate::app) use super::settings_general_page_window::*;
pub(in crate::app) use super::settings_group_actions::*;
pub(in crate::app) use super::settings_group_page::*;
pub(in crate::app) use super::settings_group_sections::*;
pub(in crate::app) use super::settings_group_sections_cache::*;
pub(in crate::app) use super::settings_group_sections_display::*;
pub(in crate::app) use super::settings_group_sections_list::*;
pub(in crate::app) use super::settings_host_helpers::*;
pub(in crate::app) use super::settings_hotkey_page::*;
pub(in crate::app) use super::settings_hotkey_page_shortcuts::*;
pub(in crate::app) use super::settings_hotkey_page_system::*;
pub(in crate::app) use super::settings_input::*;
pub(in crate::app) use super::settings_keyboard_input::*;
pub(in crate::app) use super::settings_multi_sync_sections::*;
pub(in crate::app) use super::settings_owner_draw::*;
pub(in crate::app) use super::settings_owner_draw_link::*;
pub(in crate::app) use super::settings_owner_draw_qr::*;
pub(in crate::app) use super::settings_owner_draw_roles::*;
pub(in crate::app) use super::settings_page_builder::*;
pub(in crate::app) use super::settings_page_ensure::*;
pub(in crate::app) use super::settings_page_navigation_controls::*;
pub(in crate::app) use super::settings_page_navigation_scroll::*;
pub(in crate::app) use super::settings_page_navigation_switch::*;
pub(in crate::app) use super::settings_page_sync::*;
pub(in crate::app) use super::settings_page_sync_cloud::*;
pub(in crate::app) use super::settings_page_sync_cloud_lan::*;
pub(in crate::app) use super::settings_page_sync_cloud_webdav::*;
pub(in crate::app) use super::settings_page_sync_plugin::*;
pub(in crate::app) use super::settings_platform_actions::*;
pub(in crate::app) use super::settings_platform_actions_about::*;
pub(in crate::app) use super::settings_platform_actions_general::*;
pub(in crate::app) use super::settings_platform_actions_hotkey::*;
pub(in crate::app) use super::settings_platform_actions_plugin::*;
pub(in crate::app) use super::settings_platform_actions_system::*;
pub(in crate::app) use super::settings_plugin_page::*;
pub(in crate::app) use super::settings_plugin_page_ocr_translate::*;
pub(in crate::app) use super::settings_plugin_page_search::*;
pub(in crate::app) use super::settings_plugin_page_tools::*;
pub(in crate::app) use super::settings_plugin_sections::*;
pub(in crate::app) use super::settings_plugin_sections_controls::*;
pub(in crate::app) use super::settings_plugin_sections_layout::*;
pub(in crate::app) use super::settings_plugin_sections_providers::*;
pub(in crate::app) use super::settings_plugin_sections_tools::*;
pub(in crate::app) use super::settings_pointer_input::*;
pub(in crate::app) use super::settings_state::*;
pub(in crate::app) use super::settings_sync_actions::*;
pub(in crate::app) use super::settings_sync_actions_lan::*;
pub(in crate::app) use super::settings_sync_actions_webdav::*;
pub(in crate::app) use super::settings_timer_tasks::*;
pub(in crate::app) use super::settings_toggle_state::*;
pub(in crate::app) use super::settings_toggle_state_cloud::*;
pub(in crate::app) use super::settings_toggle_state_general::*;
pub(in crate::app) use super::settings_toggle_state_group::*;
pub(in crate::app) use super::settings_toggle_state_hotkey::*;
pub(in crate::app) use super::settings_toggle_state_plugin::*;
pub(in crate::app) use super::settings_window::*;
pub(in crate::app) use super::settings_window_colors::*;
pub(in crate::app) use super::settings_window_create::*;
pub(in crate::app) use super::settings_window_destroy::*;
pub(in crate::app) use super::settings_window_events::*;
pub(in crate::app) use super::settings_window_layout::*;
pub(in crate::app) use super::settings_window_lifecycle::*;
pub(in crate::app) use super::settings_window_metrics::*;
pub(in crate::app) use super::settings_window_owner_draw::*;
pub(in crate::app) use super::settings_window_paint::*;
pub(in crate::app) use super::settings_window_surface_controls::*;
pub(in crate::app) use super::state::*;
pub(in crate::app) use super::state_runtime::*;
pub(in crate::app) use super::transient_window_host::*;
pub(in crate::app) use super::vv_hook::*;
pub(in crate::app) use super::vv_popup::*;
pub(in crate::app) use super::windows_messages::*;
pub(in crate::app) use crate::app_core::command_ids;
pub(in crate::app) use crate::app_core::menu_ids;
#[cfg(test)]
pub(in crate::app) use crate::app_core::menu_ids::{
    GROUP_FILTER_ALL as IDM_GROUP_FILTER_ALL, GROUP_FILTER_BASE as IDM_GROUP_FILTER_BASE,
    ROW_COPY as IDM_ROW_COPY, ROW_COPY_PATH as IDM_ROW_COPY_PATH, ROW_DELETE as IDM_ROW_DELETE,
    ROW_DELETE_UNPINNED as IDM_ROW_DELETE_UNPINNED, ROW_EDIT as IDM_ROW_EDIT,
    ROW_EXPORT_FILE as IDM_ROW_EXPORT_FILE, ROW_GROUP_BASE as IDM_ROW_GROUP_BASE,
    ROW_GROUP_REMOVE as IDM_ROW_GROUP_REMOVE, ROW_IMAGE_OCR as IDM_ROW_IMAGE_OCR,
    ROW_LAN_PUSH as IDM_ROW_LAN_PUSH, ROW_MAIL_MERGE as IDM_ROW_MAIL_MERGE,
    ROW_OPEN_FOLDER as IDM_ROW_OPEN_FOLDER, ROW_OPEN_PATH as IDM_ROW_OPEN_PATH,
    ROW_PASTE as IDM_ROW_PASTE, ROW_PIN as IDM_ROW_PIN, ROW_QR_IMAGE as IDM_ROW_QR_IMAGE,
    ROW_QUICK_SEARCH as IDM_ROW_QUICK_SEARCH, ROW_SAVE_IMAGE as IDM_ROW_SAVE_IMAGE,
    ROW_STICKER as IDM_ROW_STICKER, ROW_TEXT_TRANSLATE as IDM_ROW_TEXT_TRANSLATE,
    ROW_TO_PHRASE as IDM_ROW_TO_PHRASE, TRAY_CAPTURE_TOGGLE as IDM_TRAY_CAPTURE_TOGGLE,
    TRAY_EXIT as IDM_TRAY_EXIT, TRAY_LAN_TOGGLE as IDM_TRAY_LAN_TOGGLE,
    TRAY_TOGGLE as IDM_TRAY_TOGGLE,
};
pub(in crate::app) use crate::app_core::{
    clamp_window_pos_to_rect, clip_kind_filter_options_for_tab, main_copy_selection_plan,
    main_group_filter_menu_plan, main_group_filter_popup_entries,
    main_group_filter_selection_for_id, main_host_action_for_command, main_host_execution_plan,
    main_hotkey_registration_plan, main_paste_completion_plan,
    main_paste_completion_plan_with_backspaces, main_paste_preparation_plan,
    main_row_current_item_action_plan, main_row_data_action_plan, main_row_delete_items_data_plan,
    main_row_delete_unpinned_data_plan, main_row_dialog_action_plan, main_row_external_action_plan,
    main_row_group_assignment_plan, main_row_pin_data_plan, main_timer_task_for_id,
    main_vv_select_plan, native_host_full_row_popup_menu_entries_for_groups,
    native_host_main_action_button_specs, native_host_main_tool_button_specs,
    native_host_row_action_button_specs, parse_search_query_with_context, set_settings_ui_dpi,
    settings_content_y_scaled, settings_h_scaled, settings_scale, settings_w_scaled,
    ApplicationEvent, ClipListState, ClipboardHost, Command, CommandPayload, CommandQueue,
    CommandScope, DpiCompensationState, HorizontalAlign, ImagePasteReadyResult,
    ImageThumbReadyResult, ImageThumbnail, ItemsCursor, ItemsQuery, KeyState as UiKeyState,
    LifecycleEvent, LifecycleState, MainActivateSelectionPlan, MainAsyncEvent,
    MainCopySelectionPlan, MainEmptyStateKind, MainFontRole, MainFrameHitTarget,
    MainGroupFilterSelection, MainHostExecutionPlan, MainHotkeyRegistrationInput, MainHoverTarget,
    MainIconColorMode, MainIconCommand, MainIconKind, MainMenuCommandIntent, MainPaintCommand,
    MainPaintFill, MainPasteCompletionInput, MainPasteCompletionKind, MainPasteCompletionPlan,
    MainPastePreparationInput, MainPastePreparationStep, MainPointerDownStatePlan,
    MainPointerDownTarget, MainPointerUpTarget, MainRenderInput, MainRowContentInput,
    MainRowCurrentItemActionPlan, MainRowDataActionPlan, MainRowDialogActionPlan,
    MainRowExternalActionPlan, MainRowMenuAction, MainRowMenuInput, MainRowMenuLabelInput,
    MainRowReleaseAction, MainRowTextCommand, MainSearchVisibilityInput, MainSearchVisibilityPlan,
    MainSearchVisibilityRequest, MainShortcutAction, MainShortcutExecutionPlan,
    MainShortcutRowCommand, MainTextCommand, MainTextLayer, MainTextRole, MainThemeRole,
    MainTimerTask, MainTrayActionInput, MainTrayActionPlan, MainUiLayout, MainVvPopupHit,
    MainVvPopupLayout, MainVvPopupRenderItem, MainVvPopupRenderStrings, MainVvPopupTextCommand,
    MainVvSelectPlan, MainWindowCommandIntent, MouseButton as UiMouseButton, NativeAppIconResource,
    NativeDialogButtons, NativeDialogHost, NativeDialogLevel, NativeDialogResponse,
    NativeEditTextDialogHost, NativeEditTextDialogRequest, NativeHostMainToolAction,
    NativeHostRowPopupMenuInput, NativeHostUiAction, NativeImeCandidateAnchor,
    NativeImeCompositionAnchor, NativeImeHost, NativeMainSearchControlHost,
    NativeMainSearchControlPresentation, NativeMainSearchStylePresentation,
    NativeMainSearchStyleRequest, NativeMainWindowHost, NativeMainWindowPresentMode,
    NativeMainWindowPresentation, NativeMainWindowRequest, NativePasteTargetHost,
    NativePopupMenuEntry, NativePopupMenuHost, NativePopupMenuPlacement,
    NativeSettingsDropdownHost, NativeSettingsDropdownPresentation, NativeSettingsDropdownRequest,
    NativeSettingsWindowHost, NativeSettingsWindowPresentation, NativeSettingsWindowRequest,
    NativeTextCaretHost, NativeTextInputDialogHost, NativeTransientWindowHost,
    NativeTransientWindowPresentation, NativeTransientWindowRequest, NativeWindowIdentityHost,
    NativeWindowToken, PasteTargetFocusStatus, Point as UiPoint, SearchDateContext,
    SearchTimeFilter, SettingsAction, SettingsActionExecutor, SettingsGroupTextInputKind,
    SharedTabViewState, Size as UiSize, TabLoadState, TextOperationReadyResult, TextWrap,
    TitleButtonVisibility, UiEvent, UiRect, MAIN_VV_POPUP_MAX_ITEMS, SETTINGS_PAGE_LABELS,
};
pub(in crate::app) use crate::app_core::{
    dispatch_settings_action, main_menu_command_for_id, main_menu_command_for_shortcut_row_command,
    main_search_visibility_plan, main_shortcut_action, main_shortcut_execution_plan,
    main_title_button_window_command_for_key, main_tray_action_plan,
    main_window_command_for_intent, settings_group_text_input_request,
};
pub(crate) use crate::app_core::{ClipGroup, ClipItem, ClipKind, ClipKindFilter};
#[cfg(feature = "mail-merge")]
pub(in crate::app) use crate::app_core::{NativeMailMergeWindowHost, NativeMailMergeWindowRequest};
#[cfg(test)]
pub(in crate::app) use crate::app_core::{
    UiHostSurface, REQUIRED_MAIN_HOST_EXECUTION_PLAN_KINDS,
    REQUIRED_NATIVE_CONTROL_MAPPER_OPERATIONS, REQUIRED_NATIVE_DIALOG_HOST_OPERATIONS,
    REQUIRED_NATIVE_EDIT_TEXT_DIALOG_HOST_OPERATIONS, REQUIRED_NATIVE_FILE_DIALOG_HOST_OPERATIONS,
    REQUIRED_NATIVE_MAIL_MERGE_WINDOW_HOST_OPERATIONS,
    REQUIRED_NATIVE_MAIN_SEARCH_CONTROL_HOST_OPERATIONS,
    REQUIRED_NATIVE_MAIN_WINDOW_HOST_OPERATIONS, REQUIRED_NATIVE_PASTE_TARGET_HOST_OPERATIONS,
    REQUIRED_NATIVE_POPUP_MENU_HOST_OPERATIONS, REQUIRED_NATIVE_SETTINGS_DROPDOWN_HOST_OPERATIONS,
    REQUIRED_NATIVE_SETTINGS_WINDOW_HOST_OPERATIONS, REQUIRED_NATIVE_SHELL_OPEN_HOST_OPERATIONS,
    REQUIRED_NATIVE_STYLE_HOST_OPERATIONS, REQUIRED_NATIVE_TEXT_INPUT_DIALOG_HOST_OPERATIONS,
    REQUIRED_RENDERER_HOST_OPERATIONS, REQUIRED_SETTINGS_CONTROL_HOST_OPERATIONS,
    REQUIRED_STATUS_ITEM_HOST_OPERATIONS, REQUIRED_TEXT_LAYOUT_HOST_OPERATIONS,
    REQUIRED_UI_HOST_SURFACES, SHARED_NON_HOST_UI_PROTOCOLS,
};
pub(in crate::app) use crate::cloud_sync::{
    cloud_sync_interval, perform_cloud_sync, CloudSyncAction, CloudSyncConfig, CloudSyncOutcome,
    CloudSyncPaths,
};
pub(in crate::app) use crate::db_runtime::{close_db, ensure_db, with_db, with_db_mut};
pub(in crate::app) use crate::hover_preview::{hide_hover_preview, show_hover_preview};
pub(in crate::app) use crate::i18n::{app_title, tr, translate};
pub(in crate::app) use crate::lan_sync::{self, LanClipEnvelope, LanFileMeta};
#[cfg(feature = "mail-merge")]
pub(in crate::app) use crate::mail_merge_native::WindowsMailMergeWindowHost;
pub(in crate::app) use crate::platform::appearance as platform_appearance;
pub(in crate::app) use crate::platform::buffered_paint::{
    begin_buffered_paint, end_buffered_paint,
};
pub(in crate::app) use crate::platform::clipboard as platform_clipboard;
pub(in crate::app) use crate::platform::clipboard_listener;
pub(in crate::app) use crate::platform::dialog as platform_dialog;
pub(in crate::app) use crate::platform::dpi as platform_dpi;
pub(in crate::app) use crate::platform::dragdrop as platform_dragdrop;
pub(in crate::app) use crate::platform::gdi as platform_gdi;
pub(in crate::app) use crate::platform::hook as platform_hook;
pub(in crate::app) use crate::platform::hotkey;
pub(in crate::app) use crate::platform::hotkey as platform_hotkey;
pub(in crate::app) use crate::platform::ime::WindowsImeHost;
pub(in crate::app) use crate::platform::input as platform_input;
pub(in crate::app) use crate::platform::menu as platform_menu;
pub(in crate::app) use crate::platform::monitor as platform_monitor;
pub(in crate::app) use crate::platform::paste_target::WindowsPasteTargetHost;
pub(in crate::app) use crate::platform::process as platform_process;
pub(in crate::app) use crate::platform::system_parameters as platform_system_parameters;
pub(in crate::app) use crate::platform::text_caret::WindowsTextCaretHost;
pub(in crate::app) use crate::platform::timer::{
    self, start_flagged as start_flagged_timer, stop_flagged as stop_flagged_timer,
};
pub(in crate::app) use crate::platform::window::{self as platform_window, post_boxed_message};
pub(in crate::app) use crate::platform::window_identity::WindowsWindowIdentityHost;
pub(in crate::app) use crate::settings_model::{
    lan_receive_mode_from_label, lan_trusted_summary_value_text, localized_cloud_status_text,
    multi_sync_mode_display, multi_sync_mode_from_label, settings_chrome_render_plan,
    settings_content_render_plan, settings_dpi_move_action, settings_group_overview_text,
    settings_nav_hover_transition, settings_nav_render_plan,
    settings_page_content_total_h_for_dynamic_sections, settings_page_max_scroll,
    settings_pointer_down_target, settings_pointer_move_transition, settings_qr_render_plan,
    settings_scroll_delta_for_wheel, settings_scroll_layout_for_window,
    settings_scrollbar_render_plan, settings_update_presentation,
    settings_window_dpi_transition_plan, settings_window_fit_plan, SettingsDpiMoveAction,
    SettingsFlowLayout, SettingsPointerDownTarget, SettingsQrCache, SettingsScrollLayout,
    SettingsSection, SettingsUpdatePresentationInput, MULTI_SYNC_MODE_OPTIONS, SETTINGS_PAGE_COUNT,
};
pub(in crate::app) use crate::settings_ui_host::create_settings_viewport_child;
pub(in crate::app) use crate::settings_ui_host::{
    WindowsSettingsDropdownHost, WindowsSettingsWindowHost,
};
pub(in crate::app) use crate::shell::{
    detect_wechat_runtime_dir, icon_handle_for, is_directory_item, load_icons, open_parent_folder,
    open_path_with_shell, open_source_url, open_source_url_display, pick_paste_sound_file,
    play_paste_success_sound, restart_explorer_shell, run_baidu_ocr_api, run_baidu_translate_api,
    run_winocr_dll_ocr, set_system_clipboard_history_enabled, start_update_check,
    update_check_available, update_check_latest_url_or_default, update_check_state_snapshot,
    IconAssetKind,
};
#[cfg(feature = "sticker")]
pub(in crate::app) use crate::sticker::show_image_sticker;
pub(in crate::app) use crate::time_utils::{
    days_to_sqlite_date, format_created_at_local, format_local_time_for_image_preview,
    now_utc_sqlite, utc_secs_to_local_parts,
};
pub(in crate::app) use crate::tray::{
    add_tray_icon_localized, handle_tray, position_main_window, remember_window_pos,
    remove_tray_icon, toggle_window_visibility, toggle_window_visibility_hotkey,
};
pub(in crate::app) use crate::ui::{
    draw_icon_tinted_soft, draw_round_fill, draw_round_rect, draw_text_block_ex, draw_text_ex,
};
pub(in crate::app) use crate::win_native_style::{
    rgb, ui_display_font_family, ui_text_font_family, Theme,
};
pub(in crate::app) use crate::win_system_params::{
    SettingsFormSectionLayout, IDC_SET_AUTOHIDE_BLUR, IDC_SET_AUTOSTART, IDC_SET_BTN_OPENCFG,
    IDC_SET_CLICK_HIDE, IDC_SET_CLOSE, IDC_SET_CLOSETRAY, IDC_SET_CLOUD_APPLY_CFG,
    IDC_SET_CLOUD_DIR, IDC_SET_CLOUD_ENABLE, IDC_SET_CLOUD_INTERVAL, IDC_SET_CLOUD_PASS,
    IDC_SET_CLOUD_RESTORE_BACKUP, IDC_SET_CLOUD_SYNC_NOW, IDC_SET_CLOUD_UPLOAD_CFG,
    IDC_SET_CLOUD_URL, IDC_SET_CLOUD_USER, IDC_SET_DEDUPE_FILTER, IDC_SET_DX, IDC_SET_DY,
    IDC_SET_EDGEHIDE, IDC_SET_FX, IDC_SET_FY, IDC_SET_GROUP_ADD, IDC_SET_GROUP_DELETE,
    IDC_SET_GROUP_DOWN, IDC_SET_GROUP_ENABLE, IDC_SET_GROUP_LIST, IDC_SET_GROUP_RENAME,
    IDC_SET_GROUP_TYPE_FILTER, IDC_SET_GROUP_UP, IDC_SET_GROUP_VIEW_PHRASES,
    IDC_SET_GROUP_VIEW_RECORDS, IDC_SET_HK_RECORD, IDC_SET_HOTKEY_KEY, IDC_SET_HOTKEY_MOD,
    IDC_SET_HOVERPREVIEW, IDC_SET_IMAGE_PREVIEW, IDC_SET_LAN_ACCEPT_PAIR, IDC_SET_LAN_COPY_PAIR,
    IDC_SET_LAN_COPY_SETUP, IDC_SET_LAN_DISCOVERED_LIST, IDC_SET_LAN_DOCS, IDC_SET_LAN_ENABLE,
    IDC_SET_LAN_MANUAL_HOST, IDC_SET_LAN_NAME, IDC_SET_LAN_PAIR, IDC_SET_LAN_QR_ANDROID,
    IDC_SET_LAN_QR_IOS, IDC_SET_LAN_RECEIVE_MODE, IDC_SET_LAN_REFRESH, IDC_SET_LAN_REJECT_PAIR,
    IDC_SET_LAN_TCP_PORT, IDC_SET_MAX, IDC_SET_MULTI_SYNC_MODE, IDC_SET_OCR_CLOUD_TOKEN,
    IDC_SET_OCR_CLOUD_URL, IDC_SET_OCR_PROVIDER, IDC_SET_OCR_WECHAT_DETECT, IDC_SET_OPEN_SOURCE,
    IDC_SET_OPEN_UPDATE, IDC_SET_PASTE_MOVE_TOP, IDC_SET_PASTE_SOUND_ENABLE,
    IDC_SET_PASTE_SOUND_KIND, IDC_SET_PASTE_SOUND_PICK, IDC_SET_PERSIST_SEARCH,
    IDC_SET_PLAIN_HK_ENABLE, IDC_SET_PLAIN_HK_KEY, IDC_SET_PLAIN_HK_MOD, IDC_SET_PLUGIN_MAILMERGE,
    IDC_SET_POSMODE, IDC_SET_QUICK_DELETE, IDC_SET_SAVE, IDC_SET_SEARCH_ENGINE,
    IDC_SET_SILENTSTART, IDC_SET_SKIP_WINDOW_CAPTURE, IDC_SET_SKIP_WINDOW_CLASSNAMES,
    IDC_SET_SKIP_WINDOW_ENABLE, IDC_SET_TRANSLATE_PROVIDER, IDC_SET_TRANSLATE_TARGET,
    IDC_SET_TRAYICON, IDC_SET_VV_GROUP, IDC_SET_VV_MODE, IDC_SET_VV_SOURCE,
    IDC_SET_WPS_TASKPANE_DOCS, SCROLL_BAR_MARGIN, SCROLL_BAR_W, SCROLL_BAR_W_ACTIVE,
};
pub(in crate::app) use crate::win_system_ui::{
    create_settings_button as settings_create_btn, create_settings_fonts, draw_settings_chrome,
    draw_settings_content, draw_settings_nav_item, draw_settings_scrollbar,
    draw_settings_viewport_mask, get_x_lparam, get_y_lparam,
    set_settings_font as settings_set_font, set_settings_viewport_child_visible,
    settings_action_for_control, settings_child_visible, settings_command_for_control,
    settings_dropdown_index_for_max_items, settings_dropdown_index_for_pos_mode,
    settings_dropdown_label_for_max_items, settings_dropdown_label_for_pos_mode,
    settings_dropdown_max_items_from_label, settings_dropdown_max_items_from_label_opt,
    settings_dropdown_max_items_labels, settings_dropdown_popup_bounds,
    settings_dropdown_pos_mode_from_label, settings_host_control_at_point, settings_host_exists,
    settings_host_request_repaint, settings_host_screen_bounds, settings_host_set_bounds,
    settings_host_set_visible, settings_host_text, settings_page_to_sync_after_toggle,
    settings_timer_task_for_id, settings_viewport_child_control_bounds, settings_viewport_rect,
    settings_window_bounds, settings_window_client_bounds, settings_window_client_to_screen,
    settings_window_host_event_from_message, settings_window_layout_dpi,
    settings_window_request_area_repaint, settings_window_track_pointer_leave,
    sync_settings_viewport_child_bounds, SettingsCtrlReg, SettingsPage, SettingsTimerTask,
    SettingsUiRegistry,
};
#[cfg(test)]
pub(in crate::app) use crate::windows_edit_text_dialog::edit_dialog_host_event_from_message;
pub(in crate::app) use crate::windows_edit_text_dialog::WindowsEditTextDialogHost;
#[cfg(test)]
pub(in crate::app) use crate::windows_text_input_dialog::input_dialog_host_event_from_message;
pub(in crate::app) use crate::windows_text_input_dialog::WindowsTextInputDialogHost;
pub(in crate::app) use base64::{engine::general_purpose, Engine as _};
pub(in crate::app) use image::ImageFormat;
pub(in crate::app) use rusqlite::types::Value as SqlValue;
pub(in crate::app) use rusqlite::{params, params_from_iter};
pub(in crate::app) use serde::{Deserialize, Serialize};
pub(in crate::app) use std::cmp::max;
pub(in crate::app) use std::collections::{HashMap, HashSet, VecDeque};
pub(in crate::app) use std::fs;
pub(in crate::app) use std::io;
pub(in crate::app) use std::mem::zeroed;
pub(in crate::app) use std::ops::{Deref, DerefMut};
pub(in crate::app) use std::path::{Path, PathBuf};
pub(in crate::app) use std::ptr::{null, null_mut};
pub(in crate::app) use std::sync::{Mutex, OnceLock};
pub(in crate::app) use std::time::{Instant, SystemTime, UNIX_EPOCH};
pub(in crate::app) use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::Gdi::{
        DEFAULT_GUI_FONT, HDC, PAINTSTRUCT, RDW_ALLCHILDREN, RDW_ERASE, RDW_INVALIDATE,
        RDW_UPDATENOW,
    },
    UI::{
        Controls::{DRAWITEMSTRUCT, ODS_SELECTED},
        WindowsAndMessaging::*,
    },
};

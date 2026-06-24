use crate::app_core::main_window::{
    MainPointerMoveTransition, MainPointerUpTransition, MainRenderPlan, MainVvPopupTextRole,
};
use crate::app_core::{
    main_menu_command_for_id, main_row_external_action_plan, main_shortcut_execution_plan,
    poll_clipboard_monitor, settings_action_for_route, settings_action_route,
    settings_group_text_input_request, settings_lan_device_book_projection,
    settings_lan_device_projection, settings_lan_mobile_link_projection_from_json,
    settings_lan_pair_request_projection_from_json, settings_lan_pair_request_response_projection,
    settings_lan_pair_status_projection, settings_lan_sync_action_support_plan,
    zsui_native_feature_status_for, ApplicationEvent, ClipItem, ClipKind, ClipboardHost,
    ClipboardMonitorState, Color, Command, CommandQueue, ComponentPhase, LifecycleEvent,
    LifecycleState, MainAsyncEvent, MainHoverTarget, MainPointerDownTarget, MainRenderInput,
    MainRowExternalActionPlan, MainRowMenuAction, MainShortcutAction, MainShortcutEscapePlan,
    MainShortcutExecutionPlan, MainUiLayout, MainVvPopupHit, MainVvPopupLayout,
    MainVvPopupRenderItem, MainVvPopupRenderStrings, MainVvSelectPlan, NativeAiActionMenuRequest,
    NativeAiActionPresenter, NativeAiSettingsSurfaceRequest, NativeAppIconResource,
    NativeAutostartApplyResult, NativeAutostartHost, NativeAutostartStatus, NativeDialogButtons,
    NativeDialogHost, NativeDialogLevel, NativeDialogResponse, NativeEditTextDialogHost,
    NativeEditTextDialogRequest, NativeEditTextDialogResult, NativeEditTextSaveHandler,
    NativeFileDialogHost, NativeFileDialogRequest, NativeHostClipListItemProjection,
    NativeHostClipboardWrite, NativeHostDialogAction, NativeHostLaunchMode, NativeHostLaunchPlan,
    NativeHostRowAction, NativeHostSearchTextAction, NativeHostSettingsAction,
    NativeHostSettingsControlAction, NativeHostSettingsPlatformAction, NativeHostStatusMenuAction,
    NativeHostUiAction, NativeHostVvPasteExecution, NativeHostVvPastePlan,
    NativeHostVvTriggerAction, NativeHostVvTriggerInput, NativeHostVvTriggerState,
    NativeHostVvTriggerTransition, NativeImeCandidateAnchor, NativeImeCompositionAnchor,
    NativeImeHost, NativeMailMergeWindowHost, NativeMailMergeWindowRequest,
    NativeMainSearchControlHost, NativeMainSearchControlPresentation,
    NativeMainSearchControlRequest, NativeMainSearchStylePresentation,
    NativeMainSearchStyleRequest, NativeMainWindowHandles, NativeMainWindowHost,
    NativeMainWindowPresentMode, NativeMainWindowPresentation, NativeMainWindowRequest,
    NativePasteTargetHost, NativePopupMenuEntry, NativePopupMenuHost, NativePopupMenuPlacement,
    NativeRuntimeDriver, NativeRuntimeStartupRequest, NativeRuntimeStartupResult,
    NativeSettingsControlHost, NativeSettingsDropdownHost, NativeSettingsDropdownPresentation,
    NativeSettingsDropdownRequest, NativeSettingsWindowHost, NativeSettingsWindowPresentation,
    NativeSettingsWindowRequest, NativeShellOpenHost, NativeTextCaretAnchor, NativeTextCaretHost,
    NativeTextInputDialogHost, NativeTextInputDialogRequest, NativeTransientWindowHost,
    NativeTransientWindowPresentation, NativeTransientWindowRequest, NativeUiPlatform,
    NativeUiToolkit, NativeWindowIdentityHost, NativeWindowToken, PasteTargetFocusStatus,
    PasteTargetTextInputCapabilities, Point, ProductAdapterAsyncBridgeResult,
    ProductAdapterCommandResult, ProductAdapterHost, ProductAiExecutionPlan, ProductAiInvocation,
    ProductAiUiSurface, Rect, Renderer, SettingsAction, SettingsActionExecutor,
    SettingsActionRoute, SettingsComponentKind, SettingsControlSpec, SettingsGroupTextInputKind,
    SettingsLanAcceptedDeviceProjection, StatusItemHost, StatusMenuEntry, TextLayout, TextRun,
    TextStyle, TitleButtonVisibility, UiRect, APP_CORE_API_VERSION,
    REQUIRED_MAIN_HOST_EXECUTION_PLAN_KINDS, REQUIRED_NATIVE_CONTROL_MAPPER_OPERATIONS,
    REQUIRED_NATIVE_DIALOG_HOST_OPERATIONS, REQUIRED_NATIVE_EDIT_TEXT_DIALOG_HOST_OPERATIONS,
    REQUIRED_NATIVE_FILE_DIALOG_HOST_OPERATIONS, REQUIRED_NATIVE_IME_HOST_OPERATIONS,
    REQUIRED_NATIVE_MAIL_MERGE_WINDOW_HOST_OPERATIONS,
    REQUIRED_NATIVE_MAIN_SEARCH_CONTROL_HOST_OPERATIONS,
    REQUIRED_NATIVE_MAIN_WINDOW_HOST_OPERATIONS, REQUIRED_NATIVE_PASTE_TARGET_HOST_OPERATIONS,
    REQUIRED_NATIVE_POPUP_MENU_HOST_OPERATIONS, REQUIRED_NATIVE_SETTINGS_DROPDOWN_HOST_OPERATIONS,
    REQUIRED_NATIVE_SETTINGS_WINDOW_HOST_OPERATIONS, REQUIRED_NATIVE_SHELL_OPEN_HOST_OPERATIONS,
    REQUIRED_NATIVE_STYLE_HOST_OPERATIONS, REQUIRED_NATIVE_TEXT_CARET_HOST_OPERATIONS,
    REQUIRED_NATIVE_TEXT_INPUT_DIALOG_HOST_OPERATIONS,
    REQUIRED_NATIVE_TRANSIENT_WINDOW_HOST_OPERATIONS,
    REQUIRED_NATIVE_WINDOW_IDENTITY_HOST_OPERATIONS, REQUIRED_RENDERER_HOST_OPERATIONS,
    REQUIRED_SETTINGS_CONTROL_HOST_OPERATIONS, REQUIRED_STATUS_ITEM_HOST_OPERATIONS,
    REQUIRED_TEXT_LAYOUT_HOST_OPERATIONS, REQUIRED_UI_HOST_SURFACES, SHARED_NON_HOST_UI_PROTOCOLS,
};
use crate::settings_model::{
    settings_chrome_paint_plan, settings_chrome_render_plan, settings_content_paint_plan,
    settings_content_render_plan, settings_dropdown_index_for_max_items,
    settings_dropdown_max_items_labels, settings_multi_sync_cards_for_mode,
    settings_nav_hover_transition, settings_nav_item_paint_plan, settings_nav_render_plan,
    settings_plugin_cards_for_state, settings_pointer_down_target,
    settings_pointer_move_transition, settings_scroll_delta_for_wheel,
    settings_window_dpi_transition_plan, settings_window_fit_plan, SettingsChromePaintPlan,
    SettingsChromeRenderPlan, SettingsContentPaintPlan, SettingsContentRenderPlan,
    SettingsNavHoverTransition, SettingsNavItemPaintPlan, SettingsNavRenderPlan, SettingsPage,
    SettingsPointerDownTarget, SettingsPointerMoveTransition, SettingsScrollLayout,
    SettingsWindowMovePlan,
};
use crate::zsclip_product_adapter::ZsclipProductAdapter;
#[cfg(all(target_os = "macos", not(test)))]
use arboard::{Clipboard, ImageData};
#[cfg(all(target_os = "macos", not(test)))]
use std::borrow::Cow;
#[cfg(all(target_os = "macos", not(test)))]
use std::process::Command as ProcessCommand;
use std::{
    cell::RefCell,
    env, fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

mod contract;
pub(crate) use contract::MacosHostContractSummary;
mod startup;
pub(crate) use startup::{macos_native_host_launch_plan, MacosStartupPlan};

pub(crate) struct MacosUiHost;

pub(crate) struct MacosClipboardHost;

#[derive(Debug, Clone, Default)]
pub(crate) struct MacosClipboardState {
    text: Option<String>,
    image: Option<(Vec<u8>, usize, usize)>,
    file_paths: Option<Vec<String>>,
    sequence: u32,
    ignore_next_capture: bool,
}

#[derive(Debug, Default)]
pub(crate) struct MacosMainEventModel {
    application_routes: Vec<MacosApplicationEventRoute>,
    async_routes: Vec<MacosMainAsyncEventRoute>,
}

pub(crate) struct MacosApplicationModel {
    lifecycle: LifecycleState,
    commands: CommandQueue,
    product_adapter: ZsclipProductAdapter,
    product_command_results: Vec<ProductAdapterCommandResult>,
    product_event_results: Vec<ProductAdapterAsyncBridgeResult>,
    ai_action_presentation: MacosAiActionPresentationSessionState,
    runtime_events: Vec<ApplicationEvent>,
    runtime_shutdown_requested: bool,
    clipboard_capture_enabled: bool,
    clipboard_monitor: ClipboardMonitorState,
    main_window: MacosMainWindowModel,
    settings_window: MacosSettingsWindowModel,
    text_layout: MacosTextLayout,
    renderer: MacosRenderer,
    events: MacosMainEventModel,
    background_tasks: MacosBackgroundTaskState,
    window_session: MacosWindowSessionState,
    clip_payloads: MacosClipPayloadDataState,
    list_session: MacosMainListSessionState,
    paste_target_host: MacosPasteTargetHost,
    settings_session: MacosSettingsSessionState,
    main_visual_session: MacosMainVisualSessionState,
    adapter_prelude: MacosAdapterPreludeState,
    native_ids: MacosNativeIdSessionState,
    main_search_session: MacosMainSearchSessionState,
    transient_session: MacosTransientWindowSessionState,
    paste_target_discovery: MacosPasteTargetDiscoverySessionState,
    low_level_input: MacosLowLevelInputSessionState,
    hover_preview: MacosHoverPreviewSessionState,
    startup_integrations: MacosStartupIntegrationSessionState,
    autostart_host: MacosAutostartHost,
    window_refresh: MacosWindowRefreshSessionState,
    window_registry: MacosWindowRegistrySessionState,
    hover_clear: MacosHoverClearSessionState,
    settings_plugin_sections: MacosSettingsPluginSectionSessionState,
    settings_plugin_section_domains: MacosSettingsPluginSectionDomainSessionState,
    settings_multi_sync_sections: MacosSettingsMultiSyncSectionSessionState,
    settings_group_sections: MacosSettingsGroupSectionSessionState,
    settings_group_section_domains: MacosSettingsGroupSectionDomainSessionState,
    settings_group_page: MacosSettingsGroupPageSessionState,
    settings_general_page: MacosSettingsGeneralPageSessionState,
    settings_general_page_sections: MacosSettingsGeneralPageSectionSessionState,
    settings_hotkey_page: MacosSettingsHotkeyPageSessionState,
    settings_hotkey_page_sections: MacosSettingsHotkeyPageSectionSessionState,
    settings_plugin_page: MacosSettingsPluginPageSessionState,
    settings_plugin_page_sections: MacosSettingsPluginPageSectionSessionState,
    settings_about_page: MacosSettingsAboutPageSessionState,
    settings_about_page_sections: MacosSettingsAboutPageSectionSessionState,
    settings_cloud_page: MacosSettingsCloudPageSessionState,
    settings_cloud_webdav_page: MacosSettingsCloudWebdavPageSessionState,
    settings_cloud_lan_page: MacosSettingsCloudLanPageSessionState,
    settings_cloud_lan_devices: MacosSettingsCloudLanDeviceListSessionState,
    settings_owner_draw: MacosSettingsOwnerDrawSessionState,
    settings_owner_draw_domains: MacosSettingsOwnerDrawDomainSessionState,
    settings_page_builder: MacosSettingsPageBuilderSessionState,
    settings_raw_controls: MacosSettingsRawControlSessionState,
    settings_form_actions: MacosSettingsFormActionSessionState,
    settings_form_fields: MacosSettingsFormFieldSessionState,
    settings_control_factory: MacosSettingsControlFactorySessionState,
    settings_control_registry: MacosSettingsControlRegistrySessionState,
    settings_page_navigation: MacosSettingsPageNavigationSessionState,
    settings_page_navigation_domains: MacosSettingsPageNavigationDomainSessionState,
    settings_page_ensure: MacosSettingsPageEnsureSessionState,
    settings_page_sync: MacosSettingsPageSyncSessionState,
    settings_cloud_sync: MacosSettingsCloudSyncSessionState,
    settings_cloud_webdav_sync: MacosSettingsCloudWebdavSyncSessionState,
    settings_cloud_lan_sync: MacosSettingsCloudLanSyncSessionState,
    settings_plugin_sync: MacosSettingsPluginSyncSessionState,
    settings_control_selection: MacosSettingsControlSelectionSessionState,
    settings_dropdown_plugin: MacosSettingsDropdownPluginSessionState,
    settings_dropdown_domains: MacosSettingsDropdownDomainSessionState,
    settings_toggle_state: MacosSettingsToggleStateSessionState,
    settings_toggle_domains: MacosSettingsToggleDomainSessionState,
    settings_host_helpers: MacosSettingsHostHelperSessionState,
    settings_app_apply_collect: MacosSettingsAppApplyCollectSessionState,
    settings_app_collect_domains: MacosSettingsAppCollectDomainSessionState,
    settings_app_effects: MacosSettingsAppEffectsSessionState,
    settings_sync_action_domains: MacosSettingsSyncActionDomainSessionState,
    settings_platform_action_domains: MacosSettingsPlatformActionDomainSessionState,
    settings_window_state: MacosSettingsWindowStateSessionState,
    settings_window_create: MacosSettingsWindowCreateSessionState,
    settings_window_metrics: MacosSettingsWindowMetricsSessionState,
    settings_window_layout: MacosSettingsWindowLayoutSessionState,
    settings_window_lifecycle: MacosSettingsWindowLifecycleSessionState,
    settings_window_destroy: MacosSettingsWindowDestroySessionState,
    settings_window_color: MacosSettingsWindowColorSessionState,
    settings_window_surface_controls: MacosSettingsWindowSurfaceControlSessionState,
    settings_window_paint: MacosSettingsWindowPaintSessionState,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosBackgroundTaskState {
    cloud_sync_in_progress: bool,
    lan_refresh_generation: u64,
    completed_image_pastes: u64,
    completed_text_operations: u64,
    cached_thumbnail_ids: Vec<i64>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosWindowSessionState {
    main_windows: Option<NativeMainWindowHandles<MacosMainWindowHandle>>,
    settings_window: Option<MacosSettingsWindowHandle>,
    main_visible: bool,
    settings_visible: bool,
    main_render_generation: u64,
    settings_presentation_generation: u64,
    main_host_appearance: Option<MacosMainWindowHandle>,
    main_host_bounds: Option<(MacosMainWindowHandle, UiRect)>,
    main_host_activation_policy: Option<(MacosMainWindowHandle, bool)>,
    main_host_generation: u64,
    edge_auto_hide_enabled: bool,
    edge_hidden: bool,
    edge_bounds: Option<UiRect>,
    edge_generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosClipPayloadDataState {
    latest_kind: Option<ClipKind>,
    latest_preview: String,
    text_items_seen: u64,
    file_items_seen: u64,
    image_items_seen: u64,
    preview_generation: u64,
    cached_thumbnail_ids: Vec<i64>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosMainListSessionState {
    visible_item_ids: Vec<i64>,
    selected_item_ids: Vec<i64>,
    scroll_anchor: Option<(i64, i32)>,
    list_generation: u64,
    selection_generation: u64,
    scroll_generation: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct MacosSettingsSessionState {
    current_page: SettingsPage,
    dirty: bool,
    draft_generation: u64,
    applied_generation: u64,
    presentation_generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsPluginSectionSessionState {
    visible_provider_sections: Vec<String>,
    enabled_feature_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsPluginSectionDomainSessionState {
    control_domain_count: usize,
    layout_domain_count: usize,
    provider_domain_count: usize,
    tool_domain_count: usize,
    host_refresh_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsMultiSyncSectionSessionState {
    selected_mode: String,
    visible_section_count: usize,
    rebuild_generation: u64,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsGroupSectionSessionState {
    vv_source_tab: usize,
    group_view_tab: usize,
    selected_group_id: Option<i64>,
    record_group_count: usize,
    phrase_group_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsGroupSectionDomainSessionState {
    cache_domain_count: usize,
    display_domain_count: usize,
    list_domain_count: usize,
    selection_domain_count: usize,
    order_domain_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsGroupPageSessionState {
    toggle_count: usize,
    dropdown_count: usize,
    tab_button_count: usize,
    list_count: usize,
    action_button_count: usize,
    status_label_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsGeneralPageSessionState {
    startup_toggle_count: usize,
    behavior_toggle_count: usize,
    max_items_label: String,
    skip_window_enabled: bool,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsGeneralPageSectionSessionState {
    startup_toggle_count: usize,
    retention_control_count: usize,
    behavior_toggle_count: usize,
    sound_control_count: usize,
    skip_window_control_count: usize,
    position_control_count: usize,
    action_button_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsHotkeyPageSessionState {
    main_hotkey_preview: String,
    plain_hotkey_preview: String,
    recording: bool,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsHotkeyPageSectionSessionState {
    main_shortcut_control_count: usize,
    plain_shortcut_control_count: usize,
    system_action_count: usize,
    note_label_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsPluginPageSessionState {
    quick_search_enabled: bool,
    ocr_provider: String,
    translate_provider: String,
    tool_toggle_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsPluginPageSectionSessionState {
    quick_search_control_count: usize,
    ocr_control_count: usize,
    translate_control_count: usize,
    tool_toggle_count: usize,
    tool_action_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsAboutPageSessionState {
    source_available: bool,
    update_available: bool,
    data_dir: String,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsAboutPageSectionSessionState {
    metadata_label_count: usize,
    source_link_count: usize,
    update_status_count: usize,
    update_action_count: usize,
    data_label_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsCloudPageSessionState {
    selected_mode: String,
    pending_pair_count: usize,
    discovered_device_count: usize,
    selected_lan_row: Option<usize>,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsCloudWebdavPageSessionState {
    field_count: usize,
    action_row_count: usize,
    status_label_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsCloudLanPageSessionState {
    field_count: usize,
    action_row_count: usize,
    device_list_count: usize,
    qr_action_count: usize,
    helper_label_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsCloudLanDeviceListSessionState {
    pending_pair_count: usize,
    discovered_device_count: usize,
    selected_pair_row: Option<usize>,
    selected_device_row: Option<usize>,
    refresh_generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsOwnerDrawSessionState {
    hover_control_active: bool,
    qr_payload_available: bool,
    toggle_draw_count: usize,
    button_draw_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsOwnerDrawDomainSessionState {
    qr_draw_count: usize,
    source_link_draw_count: usize,
    toggle_role_count: usize,
    dropdown_role_count: usize,
    accent_role_count: usize,
    button_role_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsPageBuilderSessionState {
    registered_control_count: usize,
    ownerdraw_control_count: usize,
    section_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsRawControlSessionState {
    label_control_count: usize,
    button_control_count: usize,
    dropdown_control_count: usize,
    input_control_count: usize,
    listbox_control_count: usize,
    toggle_row_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsFormActionSessionState {
    ownerdraw_action_count: usize,
    action_row_count: usize,
    qr_action_count: usize,
    toggle_action_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsFormFieldSessionState {
    label_row_count: usize,
    value_label_row_count: usize,
    dropdown_row_count: usize,
    input_row_count: usize,
    button_row_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsControlFactorySessionState {
    label_count: usize,
    input_count: usize,
    listbox_count: usize,
    action_button_count: usize,
    toggle_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsControlRegistrySessionState {
    registered_control_count: usize,
    scrollable_control_count: usize,
    page_count: usize,
    generation: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct MacosSettingsPageNavigationSessionState {
    current_page: SettingsPage,
    scroll_y: i32,
    reposition_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsPageNavigationDomainSessionState {
    control_reposition_count: usize,
    scroll_update_count: usize,
    page_switch_count: usize,
    visibility_update_count: usize,
    redraw_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsPageEnsureSessionState {
    ensured_page: Option<SettingsPage>,
    built_page_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsPageSyncSessionState {
    synced_page_count: usize,
    enabled_control_count: usize,
    invalidation_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsCloudSyncSessionState {
    mode: String,
    webdav_control_count: usize,
    lan_control_count: usize,
    lan_refresh_generation: u64,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsCloudWebdavSyncSessionState {
    control_count: usize,
    enabled: bool,
    status_text_available: bool,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsCloudLanSyncSessionState {
    control_count: usize,
    enabled: bool,
    list_refreshed: bool,
    invalidation_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsPluginSyncSessionState {
    search_enabled: bool,
    ocr_fields_visible: bool,
    translate_enabled: bool,
    tool_control_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsControlSelectionSessionState {
    general_selection_count: usize,
    cloud_selection_count: usize,
    hotkey_selection_count: usize,
    plugin_selection_count: usize,
    group_selection_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsDropdownPluginSessionState {
    search_option_count: usize,
    ocr_option_count: usize,
    translate_provider_count: usize,
    translate_target_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsDropdownDomainSessionState {
    general_dropdown_count: usize,
    cloud_dropdown_count: usize,
    hotkey_dropdown_count: usize,
    plugin_dropdown_count: usize,
    group_dropdown_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsToggleStateSessionState {
    toggled_control_id: i32,
    enabled_toggle_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsToggleDomainSessionState {
    general_toggle_count: usize,
    cloud_toggle_count: usize,
    hotkey_toggle_count: usize,
    plugin_toggle_count: usize,
    group_toggle_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsHostHelperSessionState {
    text_update_count: usize,
    invalidation_count: usize,
    theme_generation: u64,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsAppApplyCollectSessionState {
    applied_generation: u64,
    collected_generation: u64,
    saved_settings_count: usize,
    peer_sync_generation: u64,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsAppCollectDomainSessionState {
    general_collect_count: usize,
    hotkey_collect_count: usize,
    plugin_collect_count: usize,
    group_collect_count: usize,
    cloud_collect_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsAppEffectsSessionState {
    persisted_generation: u64,
    integration_refresh_generation: u64,
    data_refresh_generation: u64,
    window_refresh_generation: u64,
    peer_sync_generation: u64,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsSyncActionDomainSessionState {
    webdav_action_count: usize,
    lan_action_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsPlatformActionDomainSessionState {
    hotkey_action_count: usize,
    general_action_count: usize,
    plugin_action_count: usize,
    about_action_count: usize,
    system_action_count: usize,
    generation: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct MacosSettingsWindowStateSessionState {
    initial_page: SettingsPage,
    ui_dpi: u32,
    reset_control_count: usize,
    dynamic_section_count: usize,
    generation: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct MacosSettingsWindowCreateSessionState {
    parent: Option<MacosSettingsWindowHandle>,
    initial_page: SettingsPage,
    save_close_control_count: usize,
    page_built_count: usize,
    applied_generation: u64,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsWindowMetricsSessionState {
    measured_content_height: i32,
    scroll_slot_count: usize,
    rebuilt_page_count: usize,
    visible_control_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsWindowLayoutSessionState {
    layout_dpi: u32,
    client_bounds: Option<UiRect>,
    window_bounds: Option<UiRect>,
    move_plan_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsWindowLifecycleSessionState {
    presented_generation: u64,
    bounds_update_generation: u64,
    focused_generation: u64,
    destroyed_generation: u64,
    pointer_capture_generation: u64,
    repaint_generation: u64,
    cloud_refresh_generation: u64,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsWindowDestroySessionState {
    timer_cleanup_count: usize,
    dropdown_cleanup_count: usize,
    resource_cleanup_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsWindowColorSessionState {
    surface_role_count: usize,
    edit_role_count: usize,
    list_role_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsWindowSurfaceControlSessionState {
    general_count: usize,
    hotkey_count: usize,
    group_count: usize,
    cloud_count: usize,
    plugin_count: usize,
    about_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosSettingsWindowPaintSessionState {
    chrome_paint_generation: u64,
    content_paint_generation: u64,
    scrollbar_paint_generation: u64,
    owner_draw_count: usize,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosMainVisualSessionState {
    title_buttons: TitleButtonVisibility,
    empty_state: Option<crate::app_core::MainEmptyStateKind>,
    image_preview_enabled: bool,
    visual_generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosAdapterPreludeState {
    shared_contract_roots: Vec<&'static str>,
    native_adapter_roots: Vec<&'static str>,
    boundary_generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosNativeIdSessionState {
    window_identifiers: Vec<&'static str>,
    timer_identifiers: Vec<&'static str>,
    status_item_identifier: Option<&'static str>,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosMainSearchSessionState {
    handle: Option<MacosMainSearchControlHandle>,
    visible: bool,
    text: String,
    style_resource: Option<MacosMainSearchStyleResource>,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosTransientWindowSessionState {
    handle: Option<MacosTransientWindowHandle>,
    owner: Option<MacosMainWindowHandle>,
    bounds: Option<UiRect>,
    visible: bool,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosPasteTargetDiscoverySessionState {
    skip_class_names: String,
    last_candidate: Option<MacosPasteTargetHandle>,
    generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum MacosPointerScope {
    Outside,
    MainWindow,
    SettingsWindow,
    PopupMenu,
    TransientWindow,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosLowLevelInputSessionState {
    outside_hide_timer_active: bool,
    edge_auto_hide_timer_active: bool,
    quick_escape_monitor_active: bool,
    last_pointer_scope: Option<MacosPointerScope>,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosHoverPreviewSessionState {
    visible: bool,
    hovered_item_id: Option<i64>,
    mouse_leave_tracking_active: bool,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosStartupIntegrationSessionState {
    status_item_registered: bool,
    hotkeys_registered: bool,
    clipboard_monitor_registered: bool,
    vv_monitor_registered: bool,
    recovery_ticks: u32,
    generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MacosAutostartHost {
    launch_agents_dir: PathBuf,
    executable_path: Option<PathBuf>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosWindowRefreshSessionState {
    settings_reload_generation: u64,
    database_reload_generation: u64,
    settings_window_refresh_generation: u64,
    peer_sync_generation: u64,
    last_peer_source: Option<MacosMainWindowHandle>,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosWindowRegistrySessionState {
    main: Option<MacosMainWindowHandle>,
    quick: Option<MacosMainWindowHandle>,
    clipboard_ignore_generation: u64,
    skip_next_clipboard_generation: u64,
    generation: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosHoverClearSessionState {
    preserved_scrollbar_hover: bool,
    cleared_pointer_down_state: bool,
    noactivate_hit_item: bool,
    generation: u64,
}

impl Default for MacosSettingsSessionState {
    fn default() -> Self {
        Self {
            current_page: SettingsPage::General,
            dirty: false,
            draft_generation: 0,
            applied_generation: 0,
            presentation_generation: 0,
        }
    }
}

impl Default for MacosSettingsPageNavigationSessionState {
    fn default() -> Self {
        Self {
            current_page: SettingsPage::General,
            scroll_y: 0,
            reposition_count: 0,
            generation: 0,
        }
    }
}

impl Default for MacosSettingsWindowCreateSessionState {
    fn default() -> Self {
        Self {
            parent: None,
            initial_page: SettingsPage::General,
            save_close_control_count: 0,
            page_built_count: 0,
            applied_generation: 0,
            generation: 0,
        }
    }
}

impl Default for MacosSettingsWindowStateSessionState {
    fn default() -> Self {
        Self {
            initial_page: SettingsPage::General,
            ui_dpi: 96,
            reset_control_count: 0,
            dynamic_section_count: 0,
            generation: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MacosApplicationEventRoute {
    RefreshLan,
    ScheduleVvShow { target: NativeWindowToken },
    HideVv,
    SelectVv { index: usize },
    CaptureClipboardChange { sequence: u32 },
    RefreshItems,
    ReconcileData { deleted: usize },
    ApplyCloudSync,
    RefreshSettings,
    RestoreShellIntegration,
    TrayCallback { code: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MacosMainAsyncEventRoute {
    PasteImage {
        has_image: bool,
        target: NativeWindowToken,
        hide_main: bool,
        backspaces: u8,
    },
    ImageOcr {
        has_text: bool,
        has_error: bool,
    },
    TextTranslate {
        has_text: bool,
        has_error: bool,
    },
    CacheThumbnail {
        item_id: i64,
        has_image: bool,
    },
}

#[derive(Debug, Default)]
pub(crate) struct MacosMainWindowHost {
    requests: RefCell<Vec<MacosMainWindowRequest>>,
    created: Option<NativeMainWindowHandles<MacosMainWindowHandle>>,
    appearances: RefCell<Vec<MacosMainWindowHandle>>,
    lifecycle_actions: RefCell<Vec<MacosMainWindowLifecycleAction>>,
    bounds: RefCell<Vec<(MacosMainWindowHandle, UiRect)>>,
    repaint_requests: RefCell<Vec<(MacosMainWindowHandle, Option<UiRect>, bool)>>,
    pointer_leave_requests: RefCell<Vec<MacosMainWindowHandle>>,
    layout_dpi_queries: RefCell<Vec<MacosMainWindowHandle>>,
    client_bounds_queries: RefCell<Vec<MacosMainWindowHandle>>,
    window_bounds_queries: RefCell<Vec<MacosMainWindowHandle>>,
}

#[derive(Debug, Default)]
pub(crate) struct MacosMainSearchControlHost {
    requests: RefCell<Vec<MacosMainSearchControlRequest>>,
    style_requests: RefCell<
        Vec<
            NativeMainSearchStyleRequest<
                MacosMainSearchControlHandle,
                MacosMainSearchStyleResource,
            >,
        >,
    >,
    released_style_resources: RefCell<Vec<MacosMainSearchStyleResource>>,
    last_bounds: RefCell<Option<(MacosMainSearchControlHandle, UiRect)>>,
    visible: RefCell<Vec<(MacosMainSearchControlHandle, bool)>>,
    text: RefCell<String>,
    focused: RefCell<Option<MacosMainSearchControlHandle>>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MacosTextLayoutAction {
    Measure {
        text: String,
        style: TextStyle,
        max_width: i32,
    },
    LayoutRuns {
        text: String,
        style: TextStyle,
        bounds: Rect,
    },
}

#[derive(Debug, Default, PartialEq)]
pub(crate) struct MacosTextLayout {
    actions: RefCell<Vec<MacosTextLayoutAction>>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MacosRenderCommand {
    FillRect(Rect, Color),
    StrokeRect(Rect, Color, i32),
    DrawText(TextRun, TextStyle),
    PushClip(Rect),
    PopClip,
}

#[derive(Debug, Default, PartialEq)]
pub(crate) struct MacosRenderer {
    commands: Vec<MacosRenderCommand>,
}

#[derive(Debug, Default)]
pub(crate) struct MacosStatusItemHost {
    installed: bool,
    tooltip: String,
    menu_entries: Vec<StatusMenuEntry>,
}

#[derive(Debug, Default)]
pub(crate) struct MacosPopupMenuHost {
    last_owner: (),
    last_position: (i32, i32),
    last_placement: Option<NativePopupMenuPlacement>,
    last_entries: Vec<NativePopupMenuEntry>,
    next_command: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MacosTransientWindowHandle(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MacosTransientWindowCreateRequest {
    owner: MacosMainWindowHandle,
    bounds: UiRect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MacosTransientWindowAction {
    Present {
        handle: MacosTransientWindowHandle,
        bounds: UiRect,
    },
    Hide(MacosTransientWindowHandle),
    Destroy(MacosTransientWindowHandle),
}

#[derive(Debug, Default)]
pub(crate) struct MacosTransientWindowHost {
    requests: RefCell<Vec<MacosTransientWindowCreateRequest>>,
    actions: RefCell<Vec<MacosTransientWindowAction>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MacosImeHandle(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MacosImeAction {
    QueryCandidate { focus: MacosImeHandle, index: u32 },
    QueryComposition(MacosImeHandle),
    HasDefaultImeWindow(MacosImeHandle),
}

#[derive(Debug, Default)]
pub(crate) struct MacosImeHost {
    actions: RefCell<Vec<MacosImeAction>>,
    next_candidate: RefCell<Option<NativeImeCandidateAnchor>>,
    next_composition: RefCell<Option<NativeImeCompositionAnchor>>,
    next_has_default_ime_window: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MacosTextCaretHandle(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MacosTextCaretAction {
    QueryAccessible(MacosTextCaretHandle),
    QueryThread(MacosTextCaretHandle),
    QueryFocusRect {
        focus: MacosTextCaretHandle,
        max_width: i32,
        max_height: i32,
    },
    QueryCursor,
    ResolveFocus(MacosTextCaretHandle),
}

#[derive(Debug, Default)]
pub(crate) struct MacosTextCaretHost {
    actions: RefCell<Vec<MacosTextCaretAction>>,
    next_accessible: RefCell<Option<NativeTextCaretAnchor>>,
    next_thread: RefCell<Option<NativeTextCaretAnchor>>,
    next_focus_rect: RefCell<Option<NativeTextCaretAnchor>>,
    next_cursor: RefCell<Option<NativeTextCaretAnchor>>,
    next_focus_handle: RefCell<Option<MacosTextCaretHandle>>,
}

#[derive(Debug, Default)]
pub(crate) struct MacosDialogHost {
    last_message: RefCell<Option<MacosDialogMessage>>,
}

#[derive(Debug, Default)]
pub(crate) struct MacosShellOpenHost {
    opened_paths: RefCell<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MacosWindowIdentityHandle(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MacosWindowIdentityAction {
    ProcessName(MacosWindowIdentityHandle),
    ClassName(MacosWindowIdentityHandle),
    RootHandle(MacosWindowIdentityHandle),
    ForegroundHandle,
    Exists(MacosWindowIdentityHandle),
    IsForeground(MacosWindowIdentityHandle),
    IsCurrentProcessWindow(MacosWindowIdentityHandle),
}

#[derive(Debug, Default)]
pub(crate) struct MacosWindowIdentityHost {
    actions: RefCell<Vec<MacosWindowIdentityAction>>,
    process_name: RefCell<String>,
    class_name: RefCell<String>,
    root_handle: RefCell<Option<MacosWindowIdentityHandle>>,
    foreground_handle: RefCell<Option<MacosWindowIdentityHandle>>,
    existing_windows: RefCell<Vec<MacosWindowIdentityHandle>>,
    current_process_windows: RefCell<Vec<MacosWindowIdentityHandle>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MacosIdentitySmokeSummary {
    pub(crate) current_pid: u64,
    pub(crate) process_name_seen: bool,
    pub(crate) bundle_id_seen: bool,
    pub(crate) foreground_seen: bool,
    pub(crate) current_process_exists: bool,
    pub(crate) current_process_foreground: bool,
    pub(crate) current_process_window: bool,
    pub(crate) foreground_requested: bool,
    pub(crate) focus_status: PasteTargetFocusStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MacosPasteTargetHandle(u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MacosPasteTargetAction {
    ForceForeground(MacosPasteTargetHandle),
    RestoreFocus {
        target: MacosPasteTargetHandle,
        focus: MacosPasteTargetHandle,
    },
    SetText {
        target: MacosPasteTargetHandle,
        text: String,
    },
    QueryTextInputCapabilities(MacosPasteTargetHandle),
    QueryTextInputReady(MacosPasteTargetHandle),
    QueryFocusStatus {
        target: MacosPasteTargetHandle,
        passthrough_focus: MacosPasteTargetHandle,
    },
    SendPasteShortcut(MacosPasteTargetHandle),
}

#[derive(Debug, Default)]
pub(crate) struct MacosPasteTargetHost {
    actions: RefCell<Vec<MacosPasteTargetAction>>,
    next_foreground_result: bool,
    next_text_input_capabilities: PasteTargetTextInputCapabilities,
    next_text_input_ready: bool,
    next_focus_status: PasteTargetFocusStatus,
}

#[derive(Debug)]
pub(crate) struct MacosFileDialogHost {
    requests: RefCell<Vec<MacosFileDialogRequest>>,
    next_result: RefCell<Result<Option<String>, String>>,
}

#[derive(Debug, Default)]
pub(crate) struct MacosTextInputDialogHost {
    requests: RefCell<Vec<MacosTextInputDialogRequest>>,
    next_result: RefCell<Option<String>>,
}

#[derive(Debug, Default)]
pub(crate) struct MacosEditTextDialogHost {
    requests: RefCell<Vec<MacosEditTextDialogRequest>>,
    next_saved_text: RefCell<Option<String>>,
    next_final_size: RefCell<Option<crate::app_core::Size>>,
}

#[derive(Debug, Default)]
pub(crate) struct MacosMailMergeWindowHost {
    requests: RefCell<Vec<MacosMailMergeWindowRequest>>,
}

#[derive(Debug, Default)]
pub(crate) struct MacosSettingsControlHost {
    requests: RefCell<Vec<MacosSettingsControlRequest>>,
    destroyed: RefCell<Vec<MacosSettingsControlHandle>>,
    visible: RefCell<Vec<(MacosSettingsControlHandle, bool)>>,
    enabled: RefCell<Vec<(MacosSettingsControlHandle, bool)>>,
    bounds: RefCell<Vec<(MacosSettingsControlHandle, UiRect)>>,
    hit_test_queries: RefCell<Vec<Point>>,
    screen_bounds_queries: RefCell<Vec<MacosSettingsControlHandle>>,
    text: RefCell<Vec<(MacosSettingsControlHandle, String)>>,
    repainted: RefCell<Vec<MacosSettingsControlHandle>>,
}

#[derive(Debug, Default)]
pub(crate) struct MacosSettingsActionContext;

#[derive(Debug, Default)]
pub(crate) struct MacosSettingsActionExecutor {
    actions: RefCell<Vec<(SettingsActionRoute, SettingsAction)>>,
}

#[derive(Debug, Default)]
pub(crate) struct MacosSettingsWindowHost {
    open_handle: Option<MacosSettingsWindowHandle>,
    window_bounds: Option<UiRect>,
    requests: RefCell<Vec<MacosSettingsWindowRequest>>,
    bounds_updates: RefCell<Vec<(MacosSettingsWindowHandle, UiRect)>>,
    destroyed: RefCell<Vec<MacosSettingsWindowHandle>>,
    focused: RefCell<Vec<MacosSettingsWindowHandle>>,
    pointer_leave_tracking: RefCell<Vec<MacosSettingsWindowHandle>>,
    captured: RefCell<Vec<MacosSettingsWindowHandle>>,
    released: RefCell<Vec<MacosSettingsWindowHandle>>,
    repainted: RefCell<Vec<MacosSettingsWindowHandle>>,
    area_repaints: RefCell<Vec<(MacosSettingsWindowHandle, Option<UiRect>, bool)>>,
    layout_dpi_queries: RefCell<Vec<MacosSettingsWindowHandle>>,
    client_to_screen: RefCell<Vec<(MacosSettingsWindowHandle, Point)>>,
    client_bounds_queries: RefCell<Vec<MacosSettingsWindowHandle>>,
    window_bounds_queries: RefCell<Vec<MacosSettingsWindowHandle>>,
    cloud_refreshes: RefCell<Vec<MacosSettingsWindowHandle>>,
}

#[derive(Debug, Default)]
pub(crate) struct MacosSettingsDropdownHost {
    requests: RefCell<Vec<MacosSettingsDropdownRequest>>,
    destroyed: RefCell<Vec<MacosSettingsDropdownHandle>>,
    bounds: RefCell<Vec<(MacosSettingsDropdownHandle, UiRect)>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MacosFileDialogRequest {
    title: String,
    filter_name: String,
    filter_pattern: String,
    current_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MacosTextInputDialogRequest {
    title: String,
    label: String,
    initial: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MacosEditTextDialogRequest {
    title: String,
    initial_text: String,
    initial_size: Option<crate::app_core::Size>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MacosMailMergeWindowRequest {
    initial_excel_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MacosSettingsControlHandle(u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MacosSettingsControlRequest {
    spec: SettingsControlSpec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MacosMainWindowHandle(u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MacosMainWindowRequest {
    title: String,
    size: crate::app_core::Size,
    main_visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MacosMainWindowLifecycleAction {
    SetAppIcon {
        handle: MacosMainWindowHandle,
        icon: NativeAppIconResource<&'static str>,
    },
    Hide(MacosMainWindowHandle),
    Present {
        handle: MacosMainWindowHandle,
        mode: NativeMainWindowPresentMode,
    },
    Bounds {
        handle: MacosMainWindowHandle,
        bounds: UiRect,
    },
    Activate(MacosMainWindowHandle),
    Foreground(MacosMainWindowHandle),
    Restore(MacosMainWindowHandle),
    Close(MacosMainWindowHandle),
    ActivationPolicy {
        handle: MacosMainWindowHandle,
        allow_activation: bool,
    },
    RequestClose(MacosMainWindowHandle),
    Destroy(MacosMainWindowHandle),
    CapturePointer(MacosMainWindowHandle),
    ReleasePointer(MacosMainWindowHandle),
    BeginDrag(MacosMainWindowHandle),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MacosMainSearchControlHandle(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MacosMainSearchStyleResource(u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MacosMainSearchControlRequest {
    owner: MacosMainWindowHandle,
    id: i64,
    bounds: UiRect,
    visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MacosSettingsWindowHandle(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MacosSettingsDropdownHandle(u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MacosSettingsWindowRequest {
    owner: MacosSettingsWindowHandle,
    existing: Option<MacosSettingsWindowHandle>,
    bounds: UiRect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MacosSettingsDropdownRequest {
    owner: MacosSettingsWindowHandle,
    control_id: isize,
    anchor: UiRect,
    items: Vec<String>,
    selected: usize,
    width: i32,
}

impl Default for MacosFileDialogHost {
    fn default() -> Self {
        Self {
            requests: RefCell::new(Vec::new()),
            next_result: RefCell::new(Ok(None)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MacosDialogMessage {
    title: String,
    message: String,
    level: NativeDialogLevel,
}

pub(crate) struct MacosMainWindowModel {
    layout: MainUiLayout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MacosSettingsSnapshot {
    pub(crate) quick_search_enabled: bool,
    pub(crate) image_ocr_provider: String,
    pub(crate) text_translate_provider: String,
    pub(crate) super_mail_merge_enabled: bool,
    pub(crate) wps_taskpane_enabled: bool,
    pub(crate) multi_sync_mode: String,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MacosAiActionPresentationSessionState {
    menu_requests: Vec<NativeAiActionMenuRequest<MacosMainWindowHandle>>,
    settings_requests: Vec<NativeAiSettingsSurfaceRequest<MacosMainWindowHandle>>,
    executed_action_names: Vec<&'static str>,
    last_surface: Option<ProductAiUiSurface>,
    generation: u64,
}

impl MacosAiActionPresentationSessionState {
    pub(crate) fn record_menu_request(
        &mut self,
        request: NativeAiActionMenuRequest<MacosMainWindowHandle>,
    ) {
        self.last_surface = Some(request.surface);
        self.menu_requests.push(request);
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn record_settings_request(
        &mut self,
        request: NativeAiSettingsSurfaceRequest<MacosMainWindowHandle>,
    ) {
        self.last_surface = Some(request.surface);
        self.settings_requests.push(request);
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn record_execution_plan(&mut self, action_name: &'static str) {
        self.executed_action_names.push(action_name);
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn menu_request_count(&self) -> usize {
        self.menu_requests.len()
    }

    pub(crate) fn settings_request_count(&self) -> usize {
        self.settings_requests.len()
    }

    pub(crate) fn executed_action_names(&self) -> &[&'static str] {
        &self.executed_action_names
    }

    pub(crate) fn last_surface(&self) -> Option<ProductAiUiSurface> {
        self.last_surface
    }
}

impl Default for MacosApplicationModel {
    fn default() -> Self {
        Self {
            lifecycle: LifecycleState::new(),
            commands: CommandQueue::default(),
            product_adapter: ZsclipProductAdapter::default(),
            product_command_results: Vec::new(),
            product_event_results: Vec::new(),
            ai_action_presentation: MacosAiActionPresentationSessionState::default(),
            runtime_events: Vec::new(),
            runtime_shutdown_requested: false,
            clipboard_capture_enabled: true,
            clipboard_monitor: ClipboardMonitorState::default(),
            main_window: MacosMainWindowModel::default(),
            settings_window: MacosSettingsWindowModel::default(),
            text_layout: MacosTextLayout::default(),
            renderer: MacosRenderer::default(),
            events: MacosMainEventModel::default(),
            background_tasks: MacosBackgroundTaskState::default(),
            window_session: MacosWindowSessionState::default(),
            clip_payloads: MacosClipPayloadDataState::default(),
            list_session: MacosMainListSessionState::default(),
            paste_target_host: MacosPasteTargetHost::default(),
            settings_session: MacosSettingsSessionState::default(),
            main_visual_session: MacosMainVisualSessionState::default(),
            adapter_prelude: MacosAdapterPreludeState::default(),
            native_ids: MacosNativeIdSessionState::default(),
            main_search_session: MacosMainSearchSessionState::default(),
            transient_session: MacosTransientWindowSessionState::default(),
            paste_target_discovery: MacosPasteTargetDiscoverySessionState::default(),
            low_level_input: MacosLowLevelInputSessionState::default(),
            hover_preview: MacosHoverPreviewSessionState::default(),
            startup_integrations: MacosStartupIntegrationSessionState::default(),
            autostart_host: MacosAutostartHost::default(),
            window_refresh: MacosWindowRefreshSessionState::default(),
            window_registry: MacosWindowRegistrySessionState::default(),
            hover_clear: MacosHoverClearSessionState::default(),
            settings_plugin_sections: MacosSettingsPluginSectionSessionState::default(),
            settings_plugin_section_domains: MacosSettingsPluginSectionDomainSessionState::default(
            ),
            settings_multi_sync_sections: MacosSettingsMultiSyncSectionSessionState::default(),
            settings_group_sections: MacosSettingsGroupSectionSessionState::default(),
            settings_group_section_domains: MacosSettingsGroupSectionDomainSessionState::default(),
            settings_group_page: MacosSettingsGroupPageSessionState::default(),
            settings_general_page: MacosSettingsGeneralPageSessionState::default(),
            settings_general_page_sections: MacosSettingsGeneralPageSectionSessionState::default(),
            settings_hotkey_page: MacosSettingsHotkeyPageSessionState::default(),
            settings_hotkey_page_sections: MacosSettingsHotkeyPageSectionSessionState::default(),
            settings_plugin_page: MacosSettingsPluginPageSessionState::default(),
            settings_plugin_page_sections: MacosSettingsPluginPageSectionSessionState::default(),
            settings_about_page: MacosSettingsAboutPageSessionState::default(),
            settings_about_page_sections: MacosSettingsAboutPageSectionSessionState::default(),
            settings_cloud_page: MacosSettingsCloudPageSessionState::default(),
            settings_cloud_webdav_page: MacosSettingsCloudWebdavPageSessionState::default(),
            settings_cloud_lan_page: MacosSettingsCloudLanPageSessionState::default(),
            settings_cloud_lan_devices: MacosSettingsCloudLanDeviceListSessionState::default(),
            settings_owner_draw: MacosSettingsOwnerDrawSessionState::default(),
            settings_owner_draw_domains: MacosSettingsOwnerDrawDomainSessionState::default(),
            settings_page_builder: MacosSettingsPageBuilderSessionState::default(),
            settings_raw_controls: MacosSettingsRawControlSessionState::default(),
            settings_form_actions: MacosSettingsFormActionSessionState::default(),
            settings_form_fields: MacosSettingsFormFieldSessionState::default(),
            settings_control_factory: MacosSettingsControlFactorySessionState::default(),
            settings_control_registry: MacosSettingsControlRegistrySessionState::default(),
            settings_page_navigation: MacosSettingsPageNavigationSessionState::default(),
            settings_page_navigation_domains:
                MacosSettingsPageNavigationDomainSessionState::default(),
            settings_page_ensure: MacosSettingsPageEnsureSessionState::default(),
            settings_page_sync: MacosSettingsPageSyncSessionState::default(),
            settings_cloud_sync: MacosSettingsCloudSyncSessionState::default(),
            settings_cloud_webdav_sync: MacosSettingsCloudWebdavSyncSessionState::default(),
            settings_cloud_lan_sync: MacosSettingsCloudLanSyncSessionState::default(),
            settings_plugin_sync: MacosSettingsPluginSyncSessionState::default(),
            settings_control_selection: MacosSettingsControlSelectionSessionState::default(),
            settings_dropdown_plugin: MacosSettingsDropdownPluginSessionState::default(),
            settings_dropdown_domains: MacosSettingsDropdownDomainSessionState::default(),
            settings_toggle_state: MacosSettingsToggleStateSessionState::default(),
            settings_toggle_domains: MacosSettingsToggleDomainSessionState::default(),
            settings_host_helpers: MacosSettingsHostHelperSessionState::default(),
            settings_app_apply_collect: MacosSettingsAppApplyCollectSessionState::default(),
            settings_app_collect_domains: MacosSettingsAppCollectDomainSessionState::default(),
            settings_app_effects: MacosSettingsAppEffectsSessionState::default(),
            settings_sync_action_domains: MacosSettingsSyncActionDomainSessionState::default(),
            settings_platform_action_domains:
                MacosSettingsPlatformActionDomainSessionState::default(),
            settings_window_state: MacosSettingsWindowStateSessionState::default(),
            settings_window_create: MacosSettingsWindowCreateSessionState::default(),
            settings_window_metrics: MacosSettingsWindowMetricsSessionState::default(),
            settings_window_layout: MacosSettingsWindowLayoutSessionState::default(),
            settings_window_lifecycle: MacosSettingsWindowLifecycleSessionState::default(),
            settings_window_destroy: MacosSettingsWindowDestroySessionState::default(),
            settings_window_color: MacosSettingsWindowColorSessionState::default(),
            settings_window_surface_controls:
                MacosSettingsWindowSurfaceControlSessionState::default(),
            settings_window_paint: MacosSettingsWindowPaintSessionState::default(),
        }
    }
}

impl MacosApplicationModel {
    pub(crate) fn mount(
        &mut self,
        title: impl Into<String>,
        main_visible: bool,
    ) -> Result<MacosStartupPlan, String> {
        let startup = self.main_window.startup_plan(title, main_visible);
        if !self.lifecycle.apply(startup.lifecycle) {
            return Err("macOS application lifecycle rejected mount".to_string());
        }
        Ok(startup)
    }

    pub(crate) fn activate(&mut self) -> bool {
        self.lifecycle.apply(LifecycleEvent::Resume)
    }

    pub(crate) fn suspend(&mut self) -> bool {
        self.lifecycle.apply(LifecycleEvent::Suspend)
    }

    pub(crate) fn unmount(&mut self) -> bool {
        self.lifecycle.apply(LifecycleEvent::Unmount)
    }

    pub(crate) fn lifecycle_phase(&self) -> ComponentPhase {
        self.lifecycle.phase()
    }

    pub(crate) fn queue_command(&mut self, command: Command) {
        self.commands.push(command);
    }

    pub(crate) fn pop_command(&mut self) -> Option<Command> {
        self.commands.pop()
    }

    pub(crate) fn pending_command_count(&self) -> usize {
        self.commands.len()
    }

    pub(crate) fn product_command_results(&self) -> &[ProductAdapterCommandResult] {
        &self.product_command_results
    }

    pub(crate) fn product_event_results(&self) -> &[ProductAdapterAsyncBridgeResult] {
        &self.product_event_results
    }

    pub(crate) fn ai_action_presentation_session(&self) -> &MacosAiActionPresentationSessionState {
        &self.ai_action_presentation
    }

    pub(crate) fn text_layout(&self) -> &MacosTextLayout {
        &self.text_layout
    }

    pub(crate) fn renderer(&mut self) -> &mut MacosRenderer {
        &mut self.renderer
    }

    pub(crate) fn begin_cloud_sync(&mut self) -> bool {
        if self.background_tasks.cloud_sync_in_progress {
            return false;
        }
        self.background_tasks.cloud_sync_in_progress = true;
        true
    }

    pub(crate) fn route_application_event(
        &mut self,
        event: ApplicationEvent,
    ) -> MacosApplicationEventRoute {
        let bridge = self.product_adapter.bridge_async_event(event);
        let route = self.events.accept_application_event(event);
        self.background_tasks.accept_application_route(route);
        self.product_event_results.push(bridge);
        route
    }

    pub(crate) fn set_clipboard_capture_enabled(&mut self, enabled: bool) {
        self.clipboard_capture_enabled = enabled;
    }

    pub(crate) fn poll_clipboard_capture_event(&mut self) -> Option<ApplicationEvent> {
        let result = poll_clipboard_monitor::<MacosClipboardHost>(
            &mut self.clipboard_monitor,
            self.clipboard_capture_enabled,
        );
        if result.should_bridge_application_event() {
            Some(ApplicationEvent::ClipboardChanged {
                sequence: result.sequence(),
            })
        } else {
            None
        }
    }

    pub(crate) fn paste_target_host_mut(&mut self) -> &mut MacosPasteTargetHost {
        &mut self.paste_target_host
    }

    pub(crate) fn execute_native_vv_paste(
        &mut self,
        index: usize,
        items: &[ClipItem],
        target: MacosPasteTargetHandle,
        focus: MacosPasteTargetHandle,
        backspaces: u8,
    ) -> NativeHostVvPasteExecution {
        let Some(plan) = crate::app_core::native_host_vv_paste_plan(true, index, items, backspaces)
        else {
            return NativeHostVvPasteExecution::rejected("zsclip.vv_paste.no_item");
        };
        let NativeHostVvPastePlan::Paste(item) = plan else {
            return NativeHostVvPasteExecution::rejected("zsclip.vv_paste.hide_popup");
        };

        let wrote_clipboard = match &item.clipboard_write {
            NativeHostClipboardWrite::Text(text) => {
                MacosClipboardHost::write_text_ignored_by_monitors(text)
            }
            NativeHostClipboardWrite::FilePaths(paths) => {
                MacosClipboardHost::write_file_paths(paths)
            }
            NativeHostClipboardWrite::ImageRgba {
                bytes,
                width,
                height,
            } => MacosClipboardHost::write_image_rgba(bytes, *width, *height),
        };
        if !wrote_clipboard {
            return NativeHostVvPasteExecution::rejected("zsclip.vv_paste.clipboard_failed");
        }

        let activation = crate::app_core::native_paste_target_activation_snapshot(
            &mut self.paste_target_host,
            target,
            focus,
        );
        let direct_text_set = item
            .clipboard_write
            .direct_text()
            .filter(|_| activation.can_directly_set_text())
            .map(|text| self.paste_target_host.set_paste_target_text(target, text))
            .unwrap_or(false);
        let paste_shortcut_sent = activation.can_send_paste_shortcut()
            && self.paste_target_host.send_paste_shortcut(target);
        self.paste_target_host
            .restore_paste_target_focus(target, focus);
        NativeHostVvPasteExecution::pasted(
            &item,
            activation.foregrounded,
            direct_text_set,
            paste_shortcut_sent,
        )
    }

    pub(crate) fn route_async_event(&mut self, event: &MainAsyncEvent) -> MacosMainAsyncEventRoute {
        let route = self.events.accept_async_event(event);
        self.background_tasks.accept_async_route(route);
        self.clip_payloads.accept_async_route(route);
        route
    }

    pub(crate) fn main_window(&self) -> &MacosMainWindowModel {
        &self.main_window
    }

    pub(crate) fn select_settings_page(&mut self, page: SettingsPage) {
        self.settings_window.select_page(page);
        self.settings_session.select_page(page);
    }

    pub(crate) fn background_tasks(&self) -> &MacosBackgroundTaskState {
        &self.background_tasks
    }

    pub(crate) fn attach_main_windows(
        &mut self,
        handles: NativeMainWindowHandles<MacosMainWindowHandle>,
        main_visible: bool,
    ) {
        self.window_session.main_windows = Some(handles);
        self.window_session.main_visible = main_visible;
    }

    pub(crate) fn note_main_render(&mut self) {
        self.window_session.main_render_generation =
            self.window_session.main_render_generation.saturating_add(1);
    }

    pub(crate) fn record_main_window_host_session(
        &mut self,
        handle: MacosMainWindowHandle,
        bounds: UiRect,
        allow_activation: bool,
    ) {
        self.window_session
            .record_main_host_session(handle, bounds, allow_activation);
    }

    pub(crate) fn record_edge_auto_hide_session(
        &mut self,
        enabled: bool,
        hidden: bool,
        bounds: Option<UiRect>,
    ) {
        self.window_session
            .record_edge_auto_hide_session(enabled, hidden, bounds);
    }

    pub(crate) fn present_settings_window(&mut self, handle: MacosSettingsWindowHandle) {
        self.window_session.settings_window = Some(handle);
        self.window_session.settings_visible = true;
        self.window_session.settings_presentation_generation = self
            .window_session
            .settings_presentation_generation
            .saturating_add(1);
    }

    pub(crate) fn hide_settings_window(&mut self) {
        self.window_session.settings_visible = false;
    }

    pub(crate) fn window_session(&self) -> &MacosWindowSessionState {
        &self.window_session
    }

    pub(crate) fn accept_clip_item_for_preview(&mut self, item: &ClipItem) {
        self.clip_payloads.accept_clip_item(item);
    }

    pub(crate) fn clip_payloads(&self) -> &MacosClipPayloadDataState {
        &self.clip_payloads
    }

    pub(crate) fn replace_visible_clip_items(&mut self, items: &[ClipItem]) {
        self.list_session.replace_visible_items(items);
    }

    pub(crate) fn select_clip_item_ids(&mut self, ids: &[i64]) {
        self.list_session.select_ids(ids);
    }

    pub(crate) fn remember_scroll_anchor(&mut self, item_id: i64, offset: i32) {
        self.list_session.remember_scroll_anchor(item_id, offset);
    }

    pub(crate) fn list_session(&self) -> &MacosMainListSessionState {
        &self.list_session
    }

    pub(crate) fn note_settings_draft_changed(&mut self) {
        self.settings_session.note_draft_changed();
    }

    pub(crate) fn note_settings_applied(&mut self) {
        self.settings_session.note_applied();
    }

    pub(crate) fn note_settings_presented(&mut self) {
        self.settings_session.note_presented();
    }

    pub(crate) fn settings_session(&self) -> &MacosSettingsSessionState {
        &self.settings_session
    }

    pub(crate) fn record_settings_plugin_sections_session(
        &mut self,
        visible_provider_sections: &[&str],
        enabled_feature_count: usize,
    ) {
        self.settings_plugin_sections
            .record(visible_provider_sections, enabled_feature_count);
    }

    pub(crate) fn settings_plugin_sections_session(
        &self,
    ) -> &MacosSettingsPluginSectionSessionState {
        &self.settings_plugin_sections
    }

    pub(crate) fn record_settings_plugin_section_domain_session(
        &mut self,
        control_domain_count: usize,
        layout_domain_count: usize,
        provider_domain_count: usize,
        tool_domain_count: usize,
        host_refresh_count: usize,
    ) {
        self.settings_plugin_section_domains.record(
            control_domain_count,
            layout_domain_count,
            provider_domain_count,
            tool_domain_count,
            host_refresh_count,
        );
    }

    pub(crate) fn settings_plugin_section_domain_session(
        &self,
    ) -> &MacosSettingsPluginSectionDomainSessionState {
        &self.settings_plugin_section_domains
    }

    pub(crate) fn record_settings_multi_sync_sections_session(
        &mut self,
        selected_mode: impl Into<String>,
        visible_section_count: usize,
        rebuilt: bool,
    ) {
        self.settings_multi_sync_sections.record(
            selected_mode.into(),
            visible_section_count,
            rebuilt,
        );
    }

    pub(crate) fn settings_multi_sync_sections_session(
        &self,
    ) -> &MacosSettingsMultiSyncSectionSessionState {
        &self.settings_multi_sync_sections
    }

    pub(crate) fn record_settings_group_sections_session(
        &mut self,
        vv_source_tab: usize,
        group_view_tab: usize,
        selected_group_id: Option<i64>,
        record_group_count: usize,
        phrase_group_count: usize,
    ) {
        self.settings_group_sections.record(
            vv_source_tab,
            group_view_tab,
            selected_group_id,
            record_group_count,
            phrase_group_count,
        );
    }

    pub(crate) fn settings_group_sections_session(&self) -> &MacosSettingsGroupSectionSessionState {
        &self.settings_group_sections
    }

    pub(crate) fn record_settings_group_section_domain_session(
        &mut self,
        cache_domain_count: usize,
        display_domain_count: usize,
        list_domain_count: usize,
        selection_domain_count: usize,
        order_domain_count: usize,
    ) {
        self.settings_group_section_domains.record(
            cache_domain_count,
            display_domain_count,
            list_domain_count,
            selection_domain_count,
            order_domain_count,
        );
    }

    pub(crate) fn settings_group_section_domain_session(
        &self,
    ) -> &MacosSettingsGroupSectionDomainSessionState {
        &self.settings_group_section_domains
    }

    pub(crate) fn record_settings_group_page_session(
        &mut self,
        toggle_count: usize,
        dropdown_count: usize,
        tab_button_count: usize,
        list_count: usize,
        action_button_count: usize,
        status_label_count: usize,
    ) {
        self.settings_group_page.record(
            toggle_count,
            dropdown_count,
            tab_button_count,
            list_count,
            action_button_count,
            status_label_count,
        );
    }

    pub(crate) fn settings_group_page_session(&self) -> &MacosSettingsGroupPageSessionState {
        &self.settings_group_page
    }

    pub(crate) fn record_settings_general_page_session(
        &mut self,
        startup_toggle_count: usize,
        behavior_toggle_count: usize,
        max_items_label: impl Into<String>,
        skip_window_enabled: bool,
    ) {
        self.settings_general_page.record(
            startup_toggle_count,
            behavior_toggle_count,
            max_items_label.into(),
            skip_window_enabled,
        );
    }

    pub(crate) fn settings_general_page_session(&self) -> &MacosSettingsGeneralPageSessionState {
        &self.settings_general_page
    }

    pub(crate) fn record_settings_general_page_sections_session(
        &mut self,
        startup_toggle_count: usize,
        retention_control_count: usize,
        behavior_toggle_count: usize,
        sound_control_count: usize,
        skip_window_control_count: usize,
        position_control_count: usize,
        action_button_count: usize,
    ) {
        self.settings_general_page_sections.record(
            startup_toggle_count,
            retention_control_count,
            behavior_toggle_count,
            sound_control_count,
            skip_window_control_count,
            position_control_count,
            action_button_count,
        );
    }

    pub(crate) fn settings_general_page_sections_session(
        &self,
    ) -> &MacosSettingsGeneralPageSectionSessionState {
        &self.settings_general_page_sections
    }

    pub(crate) fn record_settings_hotkey_page_session(
        &mut self,
        main_hotkey_preview: impl Into<String>,
        plain_hotkey_preview: impl Into<String>,
        recording: bool,
    ) {
        self.settings_hotkey_page.record(
            main_hotkey_preview.into(),
            plain_hotkey_preview.into(),
            recording,
        );
    }

    pub(crate) fn settings_hotkey_page_session(&self) -> &MacosSettingsHotkeyPageSessionState {
        &self.settings_hotkey_page
    }

    pub(crate) fn record_settings_hotkey_page_sections_session(
        &mut self,
        main_shortcut_control_count: usize,
        plain_shortcut_control_count: usize,
        system_action_count: usize,
        note_label_count: usize,
    ) {
        self.settings_hotkey_page_sections.record(
            main_shortcut_control_count,
            plain_shortcut_control_count,
            system_action_count,
            note_label_count,
        );
    }

    pub(crate) fn settings_hotkey_page_sections_session(
        &self,
    ) -> &MacosSettingsHotkeyPageSectionSessionState {
        &self.settings_hotkey_page_sections
    }

    pub(crate) fn record_settings_plugin_page_session(
        &mut self,
        quick_search_enabled: bool,
        ocr_provider: impl Into<String>,
        translate_provider: impl Into<String>,
        tool_toggle_count: usize,
    ) {
        self.settings_plugin_page.record(
            quick_search_enabled,
            ocr_provider.into(),
            translate_provider.into(),
            tool_toggle_count,
        );
    }

    pub(crate) fn settings_plugin_page_session(&self) -> &MacosSettingsPluginPageSessionState {
        &self.settings_plugin_page
    }

    pub(crate) fn record_settings_plugin_page_sections_session(
        &mut self,
        quick_search_control_count: usize,
        ocr_control_count: usize,
        translate_control_count: usize,
        tool_toggle_count: usize,
        tool_action_count: usize,
    ) {
        self.settings_plugin_page_sections.record(
            quick_search_control_count,
            ocr_control_count,
            translate_control_count,
            tool_toggle_count,
            tool_action_count,
        );
    }

    pub(crate) fn settings_plugin_page_sections_session(
        &self,
    ) -> &MacosSettingsPluginPageSectionSessionState {
        &self.settings_plugin_page_sections
    }

    pub(crate) fn record_settings_about_page_session(
        &mut self,
        source_available: bool,
        update_available: bool,
        data_dir: impl Into<String>,
    ) {
        self.settings_about_page
            .record(source_available, update_available, data_dir.into());
    }

    pub(crate) fn settings_about_page_session(&self) -> &MacosSettingsAboutPageSessionState {
        &self.settings_about_page
    }

    pub(crate) fn record_settings_about_page_sections_session(
        &mut self,
        metadata_label_count: usize,
        source_link_count: usize,
        update_status_count: usize,
        update_action_count: usize,
        data_label_count: usize,
    ) {
        self.settings_about_page_sections.record(
            metadata_label_count,
            source_link_count,
            update_status_count,
            update_action_count,
            data_label_count,
        );
    }

    pub(crate) fn settings_about_page_sections_session(
        &self,
    ) -> &MacosSettingsAboutPageSectionSessionState {
        &self.settings_about_page_sections
    }

    pub(crate) fn record_settings_cloud_page_session(
        &mut self,
        selected_mode: impl Into<String>,
        pending_pair_count: usize,
        discovered_device_count: usize,
        selected_lan_row: Option<usize>,
    ) {
        self.settings_cloud_page.record(
            selected_mode.into(),
            pending_pair_count,
            discovered_device_count,
            selected_lan_row,
        );
    }

    pub(crate) fn settings_cloud_page_session(&self) -> &MacosSettingsCloudPageSessionState {
        &self.settings_cloud_page
    }

    pub(crate) fn record_settings_cloud_webdav_page_session(
        &mut self,
        field_count: usize,
        action_row_count: usize,
        status_label_count: usize,
    ) {
        self.settings_cloud_webdav_page
            .record(field_count, action_row_count, status_label_count);
    }

    pub(crate) fn settings_cloud_webdav_page_session(
        &self,
    ) -> &MacosSettingsCloudWebdavPageSessionState {
        &self.settings_cloud_webdav_page
    }

    pub(crate) fn record_settings_cloud_lan_page_session(
        &mut self,
        field_count: usize,
        action_row_count: usize,
        device_list_count: usize,
        qr_action_count: usize,
        helper_label_count: usize,
    ) {
        self.settings_cloud_lan_page.record(
            field_count,
            action_row_count,
            device_list_count,
            qr_action_count,
            helper_label_count,
        );
    }

    pub(crate) fn settings_cloud_lan_page_session(&self) -> &MacosSettingsCloudLanPageSessionState {
        &self.settings_cloud_lan_page
    }

    pub(crate) fn record_settings_cloud_lan_devices_session(
        &mut self,
        pending_pair_count: usize,
        discovered_device_count: usize,
        selected_pair_row: Option<usize>,
        selected_device_row: Option<usize>,
    ) {
        self.settings_cloud_lan_devices.record(
            pending_pair_count,
            discovered_device_count,
            selected_pair_row,
            selected_device_row,
        );
    }

    pub(crate) fn settings_cloud_lan_devices_session(
        &self,
    ) -> &MacosSettingsCloudLanDeviceListSessionState {
        &self.settings_cloud_lan_devices
    }

    pub(crate) fn record_settings_owner_draw_session(
        &mut self,
        hover_control_active: bool,
        qr_payload_available: bool,
        toggle_draw_count: usize,
        button_draw_count: usize,
    ) {
        self.settings_owner_draw.record(
            hover_control_active,
            qr_payload_available,
            toggle_draw_count,
            button_draw_count,
        );
    }

    pub(crate) fn settings_owner_draw_session(&self) -> &MacosSettingsOwnerDrawSessionState {
        &self.settings_owner_draw
    }

    pub(crate) fn record_settings_owner_draw_domain_session(
        &mut self,
        qr_draw_count: usize,
        source_link_draw_count: usize,
        toggle_role_count: usize,
        dropdown_role_count: usize,
        accent_role_count: usize,
        button_role_count: usize,
    ) {
        self.settings_owner_draw_domains.record(
            qr_draw_count,
            source_link_draw_count,
            toggle_role_count,
            dropdown_role_count,
            accent_role_count,
            button_role_count,
        );
    }

    pub(crate) fn settings_owner_draw_domain_session(
        &self,
    ) -> &MacosSettingsOwnerDrawDomainSessionState {
        &self.settings_owner_draw_domains
    }

    pub(crate) fn record_settings_page_builder_session(
        &mut self,
        registered_control_count: usize,
        ownerdraw_control_count: usize,
        section_count: usize,
    ) {
        self.settings_page_builder.record(
            registered_control_count,
            ownerdraw_control_count,
            section_count,
        );
    }

    pub(crate) fn settings_page_builder_session(&self) -> &MacosSettingsPageBuilderSessionState {
        &self.settings_page_builder
    }

    pub(crate) fn record_settings_raw_control_session(
        &mut self,
        label_control_count: usize,
        button_control_count: usize,
        dropdown_control_count: usize,
        input_control_count: usize,
        listbox_control_count: usize,
        toggle_row_count: usize,
    ) {
        self.settings_raw_controls.record(
            label_control_count,
            button_control_count,
            dropdown_control_count,
            input_control_count,
            listbox_control_count,
            toggle_row_count,
        );
    }

    pub(crate) fn settings_raw_control_session(&self) -> &MacosSettingsRawControlSessionState {
        &self.settings_raw_controls
    }

    pub(crate) fn record_settings_form_action_session(
        &mut self,
        ownerdraw_action_count: usize,
        action_row_count: usize,
        qr_action_count: usize,
        toggle_action_count: usize,
    ) {
        self.settings_form_actions.record(
            ownerdraw_action_count,
            action_row_count,
            qr_action_count,
            toggle_action_count,
        );
    }

    pub(crate) fn settings_form_action_session(&self) -> &MacosSettingsFormActionSessionState {
        &self.settings_form_actions
    }

    pub(crate) fn record_settings_form_field_session(
        &mut self,
        label_row_count: usize,
        value_label_row_count: usize,
        dropdown_row_count: usize,
        input_row_count: usize,
        button_row_count: usize,
    ) {
        self.settings_form_fields.record(
            label_row_count,
            value_label_row_count,
            dropdown_row_count,
            input_row_count,
            button_row_count,
        );
    }

    pub(crate) fn settings_form_field_session(&self) -> &MacosSettingsFormFieldSessionState {
        &self.settings_form_fields
    }

    pub(crate) fn record_settings_control_factory_session(
        &mut self,
        label_count: usize,
        input_count: usize,
        listbox_count: usize,
        action_button_count: usize,
        toggle_count: usize,
    ) {
        self.settings_control_factory.record(
            label_count,
            input_count,
            listbox_count,
            action_button_count,
            toggle_count,
        );
    }

    pub(crate) fn settings_control_factory_session(
        &self,
    ) -> &MacosSettingsControlFactorySessionState {
        &self.settings_control_factory
    }

    pub(crate) fn record_settings_control_registry_session(
        &mut self,
        registered_control_count: usize,
        scrollable_control_count: usize,
        page_count: usize,
    ) {
        self.settings_control_registry.record(
            registered_control_count,
            scrollable_control_count,
            page_count,
        );
    }

    pub(crate) fn settings_control_registry_session(
        &self,
    ) -> &MacosSettingsControlRegistrySessionState {
        &self.settings_control_registry
    }

    pub(crate) fn record_settings_page_navigation_session(
        &mut self,
        current_page: SettingsPage,
        scroll_y: i32,
        reposition_count: usize,
    ) {
        self.settings_page_navigation
            .record(current_page, scroll_y, reposition_count);
    }

    pub(crate) fn settings_page_navigation_session(
        &self,
    ) -> &MacosSettingsPageNavigationSessionState {
        &self.settings_page_navigation
    }

    pub(crate) fn record_settings_page_navigation_domain_session(
        &mut self,
        control_reposition_count: usize,
        scroll_update_count: usize,
        page_switch_count: usize,
        visibility_update_count: usize,
        redraw_count: usize,
    ) {
        self.settings_page_navigation_domains.record(
            control_reposition_count,
            scroll_update_count,
            page_switch_count,
            visibility_update_count,
            redraw_count,
        );
    }

    pub(crate) fn settings_page_navigation_domain_session(
        &self,
    ) -> &MacosSettingsPageNavigationDomainSessionState {
        &self.settings_page_navigation_domains
    }

    pub(crate) fn record_settings_page_ensure_session(
        &mut self,
        ensured_page: SettingsPage,
        built_page_count: usize,
    ) {
        self.settings_page_ensure
            .record(ensured_page, built_page_count);
    }

    pub(crate) fn settings_page_ensure_session(&self) -> &MacosSettingsPageEnsureSessionState {
        &self.settings_page_ensure
    }

    pub(crate) fn record_settings_page_sync_session(
        &mut self,
        synced_page_count: usize,
        enabled_control_count: usize,
        invalidation_count: usize,
    ) {
        self.settings_page_sync.record(
            synced_page_count,
            enabled_control_count,
            invalidation_count,
        );
    }

    pub(crate) fn settings_page_sync_session(&self) -> &MacosSettingsPageSyncSessionState {
        &self.settings_page_sync
    }

    pub(crate) fn record_settings_cloud_sync_session(
        &mut self,
        mode: impl Into<String>,
        webdav_control_count: usize,
        lan_control_count: usize,
        lan_refreshed: bool,
    ) {
        self.settings_cloud_sync.record(
            mode,
            webdav_control_count,
            lan_control_count,
            lan_refreshed,
        );
    }

    pub(crate) fn settings_cloud_sync_session(&self) -> &MacosSettingsCloudSyncSessionState {
        &self.settings_cloud_sync
    }

    pub(crate) fn record_settings_cloud_webdav_sync_session(
        &mut self,
        control_count: usize,
        enabled: bool,
        status_text_available: bool,
    ) {
        self.settings_cloud_webdav_sync
            .record(control_count, enabled, status_text_available);
    }

    pub(crate) fn settings_cloud_webdav_sync_session(
        &self,
    ) -> &MacosSettingsCloudWebdavSyncSessionState {
        &self.settings_cloud_webdav_sync
    }

    pub(crate) fn record_settings_cloud_lan_sync_session(
        &mut self,
        control_count: usize,
        enabled: bool,
        list_refreshed: bool,
        invalidation_count: usize,
    ) {
        self.settings_cloud_lan_sync.record(
            control_count,
            enabled,
            list_refreshed,
            invalidation_count,
        );
    }

    pub(crate) fn settings_cloud_lan_sync_session(&self) -> &MacosSettingsCloudLanSyncSessionState {
        &self.settings_cloud_lan_sync
    }

    pub(crate) fn record_settings_plugin_sync_session(
        &mut self,
        search_enabled: bool,
        ocr_fields_visible: bool,
        translate_enabled: bool,
        tool_control_count: usize,
    ) {
        self.settings_plugin_sync.record(
            search_enabled,
            ocr_fields_visible,
            translate_enabled,
            tool_control_count,
        );
    }

    pub(crate) fn settings_plugin_sync_session(&self) -> &MacosSettingsPluginSyncSessionState {
        &self.settings_plugin_sync
    }

    pub(crate) fn record_settings_control_selection_session(
        &mut self,
        general_selection_count: usize,
        cloud_selection_count: usize,
        hotkey_selection_count: usize,
        plugin_selection_count: usize,
        group_selection_count: usize,
    ) {
        self.settings_control_selection.record(
            general_selection_count,
            cloud_selection_count,
            hotkey_selection_count,
            plugin_selection_count,
            group_selection_count,
        );
    }

    pub(crate) fn settings_control_selection_session(
        &self,
    ) -> &MacosSettingsControlSelectionSessionState {
        &self.settings_control_selection
    }

    pub(crate) fn record_settings_dropdown_plugin_session(
        &mut self,
        search_option_count: usize,
        ocr_option_count: usize,
        translate_provider_count: usize,
        translate_target_count: usize,
    ) {
        self.settings_dropdown_plugin.record(
            search_option_count,
            ocr_option_count,
            translate_provider_count,
            translate_target_count,
        );
    }

    pub(crate) fn settings_dropdown_plugin_session(
        &self,
    ) -> &MacosSettingsDropdownPluginSessionState {
        &self.settings_dropdown_plugin
    }

    pub(crate) fn record_settings_dropdown_domain_session(
        &mut self,
        general_dropdown_count: usize,
        cloud_dropdown_count: usize,
        hotkey_dropdown_count: usize,
        plugin_dropdown_count: usize,
        group_dropdown_count: usize,
    ) {
        self.settings_dropdown_domains.record(
            general_dropdown_count,
            cloud_dropdown_count,
            hotkey_dropdown_count,
            plugin_dropdown_count,
            group_dropdown_count,
        );
    }

    pub(crate) fn settings_dropdown_domain_session(
        &self,
    ) -> &MacosSettingsDropdownDomainSessionState {
        &self.settings_dropdown_domains
    }

    pub(crate) fn record_settings_toggle_state_session(
        &mut self,
        toggled_control_id: i32,
        enabled_toggle_count: usize,
    ) {
        self.settings_toggle_state
            .record(toggled_control_id, enabled_toggle_count);
    }

    pub(crate) fn settings_toggle_state_session(&self) -> &MacosSettingsToggleStateSessionState {
        &self.settings_toggle_state
    }

    pub(crate) fn record_settings_toggle_domain_session(
        &mut self,
        general_toggle_count: usize,
        cloud_toggle_count: usize,
        hotkey_toggle_count: usize,
        plugin_toggle_count: usize,
        group_toggle_count: usize,
    ) {
        self.settings_toggle_domains.record(
            general_toggle_count,
            cloud_toggle_count,
            hotkey_toggle_count,
            plugin_toggle_count,
            group_toggle_count,
        );
    }

    pub(crate) fn settings_toggle_domain_session(&self) -> &MacosSettingsToggleDomainSessionState {
        &self.settings_toggle_domains
    }

    pub(crate) fn record_settings_host_helper_session(
        &mut self,
        text_update_count: usize,
        invalidation_count: usize,
        theme_refreshed: bool,
    ) {
        self.settings_host_helpers
            .record(text_update_count, invalidation_count, theme_refreshed);
    }

    pub(crate) fn settings_host_helper_session(&self) -> &MacosSettingsHostHelperSessionState {
        &self.settings_host_helpers
    }

    pub(crate) fn record_settings_app_apply_collect_session(
        &mut self,
        applied: bool,
        collected: bool,
        saved_settings_count: usize,
        peer_synced: bool,
    ) {
        self.settings_app_apply_collect.record(
            applied,
            collected,
            saved_settings_count,
            peer_synced,
        );
    }

    pub(crate) fn settings_app_apply_collect_session(
        &self,
    ) -> &MacosSettingsAppApplyCollectSessionState {
        &self.settings_app_apply_collect
    }

    pub(crate) fn record_settings_app_collect_domain_session(
        &mut self,
        general_collect_count: usize,
        hotkey_collect_count: usize,
        plugin_collect_count: usize,
        group_collect_count: usize,
        cloud_collect_count: usize,
    ) {
        self.settings_app_collect_domains.record(
            general_collect_count,
            hotkey_collect_count,
            plugin_collect_count,
            group_collect_count,
            cloud_collect_count,
        );
    }

    pub(crate) fn settings_app_collect_domain_session(
        &self,
    ) -> &MacosSettingsAppCollectDomainSessionState {
        &self.settings_app_collect_domains
    }

    pub(crate) fn record_settings_app_effects_session(
        &mut self,
        persisted: bool,
        integration_refreshed: bool,
        data_refreshed: bool,
        window_refreshed: bool,
        peer_synced: bool,
    ) {
        self.settings_app_effects.record(
            persisted,
            integration_refreshed,
            data_refreshed,
            window_refreshed,
            peer_synced,
        );
    }

    pub(crate) fn settings_app_effects_session(&self) -> &MacosSettingsAppEffectsSessionState {
        &self.settings_app_effects
    }

    pub(crate) fn record_settings_sync_action_domain_session(
        &mut self,
        webdav_action_count: usize,
        lan_action_count: usize,
    ) {
        self.settings_sync_action_domains
            .record(webdav_action_count, lan_action_count);
    }

    pub(crate) fn settings_sync_action_domain_session(
        &self,
    ) -> &MacosSettingsSyncActionDomainSessionState {
        &self.settings_sync_action_domains
    }

    pub(crate) fn record_settings_platform_action_domain_session(
        &mut self,
        hotkey_action_count: usize,
        general_action_count: usize,
        plugin_action_count: usize,
        about_action_count: usize,
        system_action_count: usize,
    ) {
        self.settings_platform_action_domains.record(
            hotkey_action_count,
            general_action_count,
            plugin_action_count,
            about_action_count,
            system_action_count,
        );
    }

    pub(crate) fn settings_platform_action_domain_session(
        &self,
    ) -> &MacosSettingsPlatformActionDomainSessionState {
        &self.settings_platform_action_domains
    }

    pub(crate) fn record_settings_window_state_session(
        &mut self,
        initial_page: SettingsPage,
        ui_dpi: u32,
        reset_control_count: usize,
        dynamic_section_count: usize,
    ) {
        self.settings_window_state.record(
            initial_page,
            ui_dpi,
            reset_control_count,
            dynamic_section_count,
        );
    }

    pub(crate) fn settings_window_state_session(&self) -> &MacosSettingsWindowStateSessionState {
        &self.settings_window_state
    }

    pub(crate) fn record_settings_window_create_session(
        &mut self,
        parent: MacosSettingsWindowHandle,
        initial_page: SettingsPage,
        save_close_control_count: usize,
        page_built_count: usize,
        applied: bool,
    ) {
        self.settings_window_create.record(
            parent,
            initial_page,
            save_close_control_count,
            page_built_count,
            applied,
        );
    }

    pub(crate) fn settings_window_create_session(&self) -> &MacosSettingsWindowCreateSessionState {
        &self.settings_window_create
    }

    pub(crate) fn record_settings_window_metrics_session(
        &mut self,
        measured_content_height: i32,
        scroll_slot_count: usize,
        rebuilt_page_count: usize,
        visible_control_count: usize,
    ) {
        self.settings_window_metrics.record(
            measured_content_height,
            scroll_slot_count,
            rebuilt_page_count,
            visible_control_count,
        );
    }

    pub(crate) fn settings_window_metrics_session(
        &self,
    ) -> &MacosSettingsWindowMetricsSessionState {
        &self.settings_window_metrics
    }

    pub(crate) fn record_settings_window_layout_session(
        &mut self,
        layout_dpi: u32,
        client_bounds: UiRect,
        window_bounds: UiRect,
        move_plan_count: usize,
    ) {
        self.settings_window_layout.record(
            layout_dpi,
            client_bounds,
            window_bounds,
            move_plan_count,
        );
    }

    pub(crate) fn settings_window_layout_session(&self) -> &MacosSettingsWindowLayoutSessionState {
        &self.settings_window_layout
    }

    pub(crate) fn record_settings_window_lifecycle_session(
        &mut self,
        presented: bool,
        bounds_updated: bool,
        focused: bool,
        destroyed: bool,
        pointer_captured: bool,
        repainted: bool,
        cloud_refreshed: bool,
    ) {
        self.settings_window_lifecycle.record(
            presented,
            bounds_updated,
            focused,
            destroyed,
            pointer_captured,
            repainted,
            cloud_refreshed,
        );
    }

    pub(crate) fn settings_window_lifecycle_session(
        &self,
    ) -> &MacosSettingsWindowLifecycleSessionState {
        &self.settings_window_lifecycle
    }

    pub(crate) fn record_settings_window_destroy_session(
        &mut self,
        timer_cleanup_count: usize,
        dropdown_cleanup_count: usize,
        resource_cleanup_count: usize,
    ) {
        self.settings_window_destroy.record(
            timer_cleanup_count,
            dropdown_cleanup_count,
            resource_cleanup_count,
        );
    }

    pub(crate) fn settings_window_destroy_session(
        &self,
    ) -> &MacosSettingsWindowDestroySessionState {
        &self.settings_window_destroy
    }

    pub(crate) fn record_settings_window_color_session(
        &mut self,
        surface_role_count: usize,
        edit_role_count: usize,
        list_role_count: usize,
    ) {
        self.settings_window_color
            .record(surface_role_count, edit_role_count, list_role_count);
    }

    pub(crate) fn settings_window_color_session(&self) -> &MacosSettingsWindowColorSessionState {
        &self.settings_window_color
    }

    pub(crate) fn record_settings_window_surface_controls_session(
        &mut self,
        general_count: usize,
        hotkey_count: usize,
        group_count: usize,
        cloud_count: usize,
        plugin_count: usize,
        about_count: usize,
    ) {
        self.settings_window_surface_controls.record(
            general_count,
            hotkey_count,
            group_count,
            cloud_count,
            plugin_count,
            about_count,
        );
    }

    pub(crate) fn settings_window_surface_controls_session(
        &self,
    ) -> &MacosSettingsWindowSurfaceControlSessionState {
        &self.settings_window_surface_controls
    }

    pub(crate) fn record_settings_window_paint_session(
        &mut self,
        chrome_painted: bool,
        content_painted: bool,
        scrollbar_painted: bool,
        owner_draw_count: usize,
    ) {
        self.settings_window_paint.record(
            chrome_painted,
            content_painted,
            scrollbar_painted,
            owner_draw_count,
        );
    }

    pub(crate) fn settings_window_paint_session(&self) -> &MacosSettingsWindowPaintSessionState {
        &self.settings_window_paint
    }

    pub(crate) fn update_main_visual_state(
        &mut self,
        title_buttons: TitleButtonVisibility,
        empty_state: crate::app_core::MainEmptyStateKind,
        image_preview_enabled: bool,
    ) {
        self.main_visual_session
            .update(title_buttons, empty_state, image_preview_enabled);
    }

    pub(crate) fn main_visual_session(&self) -> &MacosMainVisualSessionState {
        &self.main_visual_session
    }

    pub(crate) fn record_adapter_prelude_boundary(
        &mut self,
        shared_contract_roots: &[&'static str],
        native_adapter_roots: &[&'static str],
    ) {
        self.adapter_prelude
            .record(shared_contract_roots, native_adapter_roots);
    }

    pub(crate) fn adapter_prelude(&self) -> &MacosAdapterPreludeState {
        &self.adapter_prelude
    }

    pub(crate) fn record_native_id_session(
        &mut self,
        window_identifiers: &[&'static str],
        timer_identifiers: &[&'static str],
        status_item_identifier: &'static str,
    ) {
        self.native_ids.record(
            window_identifiers,
            timer_identifiers,
            status_item_identifier,
        );
    }

    pub(crate) fn native_ids(&self) -> &MacosNativeIdSessionState {
        &self.native_ids
    }

    pub(crate) fn record_main_search_session(
        &mut self,
        handle: MacosMainSearchControlHandle,
        visible: bool,
        text: impl Into<String>,
        style_resource: Option<MacosMainSearchStyleResource>,
    ) {
        self.main_search_session
            .record(handle, visible, text, style_resource);
    }

    pub(crate) fn main_search_session(&self) -> &MacosMainSearchSessionState {
        &self.main_search_session
    }

    pub(crate) fn record_transient_window_session(
        &mut self,
        owner: MacosMainWindowHandle,
        handle: MacosTransientWindowHandle,
        bounds: UiRect,
        visible: bool,
    ) {
        self.transient_session
            .record(owner, handle, bounds, visible);
    }

    pub(crate) fn transient_session(&self) -> &MacosTransientWindowSessionState {
        &self.transient_session
    }

    pub(crate) fn record_paste_target_discovery_session(
        &mut self,
        skip_class_names: impl Into<String>,
        last_candidate: Option<MacosPasteTargetHandle>,
    ) {
        self.paste_target_discovery
            .record(skip_class_names, last_candidate);
    }

    pub(crate) fn paste_target_discovery_session(&self) -> &MacosPasteTargetDiscoverySessionState {
        &self.paste_target_discovery
    }

    pub(crate) fn record_low_level_input_session(
        &mut self,
        outside_hide_timer_active: bool,
        edge_auto_hide_timer_active: bool,
        quick_escape_monitor_active: bool,
        last_pointer_scope: MacosPointerScope,
    ) {
        self.low_level_input.record(
            outside_hide_timer_active,
            edge_auto_hide_timer_active,
            quick_escape_monitor_active,
            last_pointer_scope,
        );
    }

    pub(crate) fn low_level_input_session(&self) -> &MacosLowLevelInputSessionState {
        &self.low_level_input
    }

    pub(crate) fn record_hover_preview_session(
        &mut self,
        visible: bool,
        hovered_item_id: Option<i64>,
        mouse_leave_tracking_active: bool,
    ) {
        self.hover_preview
            .record(visible, hovered_item_id, mouse_leave_tracking_active);
    }

    pub(crate) fn hover_preview_session(&self) -> &MacosHoverPreviewSessionState {
        &self.hover_preview
    }

    pub(crate) fn record_startup_integrations_session(
        &mut self,
        status_item_registered: bool,
        hotkeys_registered: bool,
        clipboard_monitor_registered: bool,
        vv_monitor_registered: bool,
        recovery_ticks: u32,
    ) {
        self.startup_integrations.record(
            status_item_registered,
            hotkeys_registered,
            clipboard_monitor_registered,
            vv_monitor_registered,
            recovery_ticks,
        );
    }

    pub(crate) fn startup_integrations_session(&self) -> &MacosStartupIntegrationSessionState {
        &self.startup_integrations
    }

    pub(crate) fn autostart_host(&self) -> &MacosAutostartHost {
        &self.autostart_host
    }

    pub(crate) fn autostart_host_mut(&mut self) -> &mut MacosAutostartHost {
        &mut self.autostart_host
    }

    pub(crate) fn autostart_status(&self) -> NativeAutostartStatus {
        self.autostart_host.autostart_status()
    }

    pub(crate) fn apply_autostart(&mut self, enabled: bool) -> NativeAutostartApplyResult {
        self.autostart_host.set_autostart_enabled(enabled)
    }

    pub(crate) fn record_window_refresh_session(
        &mut self,
        reload_settings: bool,
        reload_database: bool,
        refresh_settings_window: bool,
        peer_source: Option<MacosMainWindowHandle>,
    ) {
        self.window_refresh.record(
            reload_settings,
            reload_database,
            refresh_settings_window,
            peer_source,
        );
    }

    pub(crate) fn window_refresh_session(&self) -> &MacosWindowRefreshSessionState {
        &self.window_refresh
    }

    pub(crate) fn record_window_registry_session(
        &mut self,
        main: Option<MacosMainWindowHandle>,
        quick: Option<MacosMainWindowHandle>,
        clipboard_ignore: bool,
        skip_next_clipboard: bool,
    ) {
        self.window_registry
            .record(main, quick, clipboard_ignore, skip_next_clipboard);
    }

    pub(crate) fn window_registry_session(&self) -> &MacosWindowRegistrySessionState {
        &self.window_registry
    }

    pub(crate) fn record_hover_clear_session(
        &mut self,
        preserved_scrollbar_hover: bool,
        cleared_pointer_down_state: bool,
        noactivate_hit_item: bool,
    ) {
        self.hover_clear.record(
            preserved_scrollbar_hover,
            cleared_pointer_down_state,
            noactivate_hit_item,
        );
    }

    pub(crate) fn hover_clear_session(&self) -> &MacosHoverClearSessionState {
        &self.hover_clear
    }
}

impl MacosBackgroundTaskState {
    fn accept_application_route(&mut self, route: MacosApplicationEventRoute) {
        match route {
            MacosApplicationEventRoute::RefreshLan => {
                self.lan_refresh_generation = self.lan_refresh_generation.saturating_add(1);
            }
            MacosApplicationEventRoute::ApplyCloudSync => {
                self.cloud_sync_in_progress = false;
            }
            _ => {}
        }
    }

    fn accept_async_route(&mut self, route: MacosMainAsyncEventRoute) {
        match route {
            MacosMainAsyncEventRoute::PasteImage { .. } => {
                self.completed_image_pastes = self.completed_image_pastes.saturating_add(1);
            }
            MacosMainAsyncEventRoute::ImageOcr { .. }
            | MacosMainAsyncEventRoute::TextTranslate { .. } => {
                self.completed_text_operations = self.completed_text_operations.saturating_add(1);
            }
            MacosMainAsyncEventRoute::CacheThumbnail { item_id, .. } => {
                if !self.cached_thumbnail_ids.contains(&item_id) {
                    self.cached_thumbnail_ids.push(item_id);
                }
            }
        }
    }

    pub(crate) fn cloud_sync_in_progress(&self) -> bool {
        self.cloud_sync_in_progress
    }

    pub(crate) fn lan_refresh_generation(&self) -> u64 {
        self.lan_refresh_generation
    }

    pub(crate) fn completed_image_pastes(&self) -> u64 {
        self.completed_image_pastes
    }

    pub(crate) fn completed_text_operations(&self) -> u64 {
        self.completed_text_operations
    }

    pub(crate) fn cached_thumbnail_ids(&self) -> &[i64] {
        &self.cached_thumbnail_ids
    }
}

impl MacosWindowSessionState {
    fn record_main_host_session(
        &mut self,
        handle: MacosMainWindowHandle,
        bounds: UiRect,
        allow_activation: bool,
    ) {
        self.main_host_appearance = Some(handle);
        self.main_host_bounds = Some((handle, bounds));
        self.main_host_activation_policy = Some((handle, allow_activation));
        self.main_host_generation = self.main_host_generation.saturating_add(1);
    }

    fn record_edge_auto_hide_session(
        &mut self,
        enabled: bool,
        hidden: bool,
        bounds: Option<UiRect>,
    ) {
        self.edge_auto_hide_enabled = enabled;
        self.edge_hidden = hidden;
        self.edge_bounds = bounds;
        self.edge_generation = self.edge_generation.saturating_add(1);
    }

    pub(crate) fn main_windows(&self) -> Option<NativeMainWindowHandles<MacosMainWindowHandle>> {
        self.main_windows
    }

    pub(crate) fn settings_window(&self) -> Option<MacosSettingsWindowHandle> {
        self.settings_window
    }

    pub(crate) fn main_visible(&self) -> bool {
        self.main_visible
    }

    pub(crate) fn settings_visible(&self) -> bool {
        self.settings_visible
    }

    pub(crate) fn main_render_generation(&self) -> u64 {
        self.main_render_generation
    }

    pub(crate) fn settings_presentation_generation(&self) -> u64 {
        self.settings_presentation_generation
    }

    pub(crate) fn main_host_appearance(&self) -> Option<MacosMainWindowHandle> {
        self.main_host_appearance
    }

    pub(crate) fn main_host_bounds(&self) -> Option<(MacosMainWindowHandle, UiRect)> {
        self.main_host_bounds
    }

    pub(crate) fn main_host_activation_policy(&self) -> Option<(MacosMainWindowHandle, bool)> {
        self.main_host_activation_policy
    }

    pub(crate) fn main_host_generation(&self) -> u64 {
        self.main_host_generation
    }

    pub(crate) fn edge_auto_hide_enabled(&self) -> bool {
        self.edge_auto_hide_enabled
    }

    pub(crate) fn edge_hidden(&self) -> bool {
        self.edge_hidden
    }

    pub(crate) fn edge_bounds(&self) -> Option<UiRect> {
        self.edge_bounds
    }

    pub(crate) fn edge_generation(&self) -> u64 {
        self.edge_generation
    }
}

impl MacosClipPayloadDataState {
    fn accept_clip_item(&mut self, item: &ClipItem) {
        self.latest_kind = Some(item.kind);
        self.latest_preview.clear();
        self.latest_preview.push_str(&item.preview);
        self.preview_generation = self.preview_generation.saturating_add(1);
        match item.kind {
            ClipKind::Text | ClipKind::Phrase => {
                self.text_items_seen = self.text_items_seen.saturating_add(1);
            }
            ClipKind::Files => {
                self.file_items_seen = self.file_items_seen.saturating_add(1);
            }
            ClipKind::Image => {
                self.image_items_seen = self.image_items_seen.saturating_add(1);
                if item.id > 0 && !self.cached_thumbnail_ids.contains(&item.id) {
                    self.cached_thumbnail_ids.push(item.id);
                }
            }
        }
    }

    fn accept_async_route(&mut self, route: MacosMainAsyncEventRoute) {
        if let MacosMainAsyncEventRoute::CacheThumbnail {
            item_id,
            has_image: true,
        } = route
        {
            if item_id > 0 && !self.cached_thumbnail_ids.contains(&item_id) {
                self.cached_thumbnail_ids.push(item_id);
            }
        }
    }

    pub(crate) fn latest_kind(&self) -> Option<ClipKind> {
        self.latest_kind
    }

    pub(crate) fn latest_preview(&self) -> &str {
        &self.latest_preview
    }

    pub(crate) fn text_items_seen(&self) -> u64 {
        self.text_items_seen
    }

    pub(crate) fn file_items_seen(&self) -> u64 {
        self.file_items_seen
    }

    pub(crate) fn image_items_seen(&self) -> u64 {
        self.image_items_seen
    }

    pub(crate) fn preview_generation(&self) -> u64 {
        self.preview_generation
    }

    pub(crate) fn cached_thumbnail_ids(&self) -> &[i64] {
        &self.cached_thumbnail_ids
    }
}

impl MacosMainListSessionState {
    fn replace_visible_items(&mut self, items: &[ClipItem]) {
        self.visible_item_ids.clear();
        self.visible_item_ids
            .extend(items.iter().map(|item| item.id).filter(|id| *id > 0));
        self.list_generation = self.list_generation.saturating_add(1);
        self.selected_item_ids
            .retain(|id| self.visible_item_ids.contains(id));
        if let Some((anchor_id, _)) = self.scroll_anchor {
            if !self.visible_item_ids.contains(&anchor_id) {
                self.scroll_anchor = self.visible_item_ids.first().copied().map(|id| (id, 0));
            }
        }
    }

    fn select_ids(&mut self, ids: &[i64]) {
        self.selected_item_ids.clear();
        self.selected_item_ids.extend(
            ids.iter()
                .copied()
                .filter(|id| *id > 0 && self.visible_item_ids.contains(id)),
        );
        self.selected_item_ids.sort_unstable();
        self.selected_item_ids.dedup();
        self.selection_generation = self.selection_generation.saturating_add(1);
    }

    fn remember_scroll_anchor(&mut self, item_id: i64, offset: i32) {
        self.scroll_anchor = (item_id > 0 && self.visible_item_ids.contains(&item_id))
            .then_some((item_id, offset.max(0)));
        self.scroll_generation = self.scroll_generation.saturating_add(1);
    }

    pub(crate) fn visible_item_ids(&self) -> &[i64] {
        &self.visible_item_ids
    }

    pub(crate) fn selected_item_ids(&self) -> &[i64] {
        &self.selected_item_ids
    }

    pub(crate) fn scroll_anchor(&self) -> Option<(i64, i32)> {
        self.scroll_anchor
    }

    pub(crate) fn list_generation(&self) -> u64 {
        self.list_generation
    }

    pub(crate) fn selection_generation(&self) -> u64 {
        self.selection_generation
    }

    pub(crate) fn scroll_generation(&self) -> u64 {
        self.scroll_generation
    }
}

impl MacosSettingsSessionState {
    fn select_page(&mut self, page: SettingsPage) {
        self.current_page = page;
        self.presentation_generation = self.presentation_generation.saturating_add(1);
    }

    fn note_draft_changed(&mut self) {
        self.dirty = true;
        self.draft_generation = self.draft_generation.saturating_add(1);
    }

    fn note_applied(&mut self) {
        self.dirty = false;
        self.applied_generation = self.applied_generation.saturating_add(1);
    }

    fn note_presented(&mut self) {
        self.presentation_generation = self.presentation_generation.saturating_add(1);
    }

    pub(crate) fn current_page(&self) -> SettingsPage {
        self.current_page
    }

    pub(crate) fn dirty(&self) -> bool {
        self.dirty
    }

    pub(crate) fn draft_generation(&self) -> u64 {
        self.draft_generation
    }

    pub(crate) fn applied_generation(&self) -> u64 {
        self.applied_generation
    }

    pub(crate) fn presentation_generation(&self) -> u64 {
        self.presentation_generation
    }
}

impl MacosSettingsPluginSectionSessionState {
    fn record(&mut self, visible_provider_sections: &[&str], enabled_feature_count: usize) {
        self.visible_provider_sections = visible_provider_sections
            .iter()
            .map(|section| (*section).to_string())
            .collect();
        self.enabled_feature_count = enabled_feature_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn visible_provider_sections(&self) -> &[String] {
        &self.visible_provider_sections
    }

    pub(crate) fn enabled_feature_count(&self) -> usize {
        self.enabled_feature_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsPluginSectionDomainSessionState {
    fn record(
        &mut self,
        control_domain_count: usize,
        layout_domain_count: usize,
        provider_domain_count: usize,
        tool_domain_count: usize,
        host_refresh_count: usize,
    ) {
        self.control_domain_count = control_domain_count;
        self.layout_domain_count = layout_domain_count;
        self.provider_domain_count = provider_domain_count;
        self.tool_domain_count = tool_domain_count;
        self.host_refresh_count = host_refresh_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn control_domain_count(&self) -> usize {
        self.control_domain_count
    }

    pub(crate) fn layout_domain_count(&self) -> usize {
        self.layout_domain_count
    }

    pub(crate) fn provider_domain_count(&self) -> usize {
        self.provider_domain_count
    }

    pub(crate) fn tool_domain_count(&self) -> usize {
        self.tool_domain_count
    }

    pub(crate) fn host_refresh_count(&self) -> usize {
        self.host_refresh_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsMultiSyncSectionSessionState {
    fn record(&mut self, selected_mode: String, visible_section_count: usize, rebuilt: bool) {
        self.selected_mode = selected_mode;
        self.visible_section_count = visible_section_count;
        if rebuilt {
            self.rebuild_generation = self.rebuild_generation.saturating_add(1);
        }
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn selected_mode(&self) -> &str {
        &self.selected_mode
    }

    pub(crate) fn visible_section_count(&self) -> usize {
        self.visible_section_count
    }

    pub(crate) fn rebuild_generation(&self) -> u64 {
        self.rebuild_generation
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsGroupSectionSessionState {
    fn record(
        &mut self,
        vv_source_tab: usize,
        group_view_tab: usize,
        selected_group_id: Option<i64>,
        record_group_count: usize,
        phrase_group_count: usize,
    ) {
        self.vv_source_tab = vv_source_tab;
        self.group_view_tab = group_view_tab;
        self.selected_group_id = selected_group_id;
        self.record_group_count = record_group_count;
        self.phrase_group_count = phrase_group_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn vv_source_tab(&self) -> usize {
        self.vv_source_tab
    }

    pub(crate) fn group_view_tab(&self) -> usize {
        self.group_view_tab
    }

    pub(crate) fn selected_group_id(&self) -> Option<i64> {
        self.selected_group_id
    }

    pub(crate) fn record_group_count(&self) -> usize {
        self.record_group_count
    }

    pub(crate) fn phrase_group_count(&self) -> usize {
        self.phrase_group_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsGroupSectionDomainSessionState {
    fn record(
        &mut self,
        cache_domain_count: usize,
        display_domain_count: usize,
        list_domain_count: usize,
        selection_domain_count: usize,
        order_domain_count: usize,
    ) {
        self.cache_domain_count = cache_domain_count;
        self.display_domain_count = display_domain_count;
        self.list_domain_count = list_domain_count;
        self.selection_domain_count = selection_domain_count;
        self.order_domain_count = order_domain_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn cache_domain_count(&self) -> usize {
        self.cache_domain_count
    }

    pub(crate) fn display_domain_count(&self) -> usize {
        self.display_domain_count
    }

    pub(crate) fn list_domain_count(&self) -> usize {
        self.list_domain_count
    }

    pub(crate) fn selection_domain_count(&self) -> usize {
        self.selection_domain_count
    }

    pub(crate) fn order_domain_count(&self) -> usize {
        self.order_domain_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsGroupPageSessionState {
    fn record(
        &mut self,
        toggle_count: usize,
        dropdown_count: usize,
        tab_button_count: usize,
        list_count: usize,
        action_button_count: usize,
        status_label_count: usize,
    ) {
        self.toggle_count = toggle_count;
        self.dropdown_count = dropdown_count;
        self.tab_button_count = tab_button_count;
        self.list_count = list_count;
        self.action_button_count = action_button_count;
        self.status_label_count = status_label_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn toggle_count(&self) -> usize {
        self.toggle_count
    }

    pub(crate) fn dropdown_count(&self) -> usize {
        self.dropdown_count
    }

    pub(crate) fn tab_button_count(&self) -> usize {
        self.tab_button_count
    }

    pub(crate) fn list_count(&self) -> usize {
        self.list_count
    }

    pub(crate) fn action_button_count(&self) -> usize {
        self.action_button_count
    }

    pub(crate) fn status_label_count(&self) -> usize {
        self.status_label_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsGeneralPageSessionState {
    fn record(
        &mut self,
        startup_toggle_count: usize,
        behavior_toggle_count: usize,
        max_items_label: String,
        skip_window_enabled: bool,
    ) {
        self.startup_toggle_count = startup_toggle_count;
        self.behavior_toggle_count = behavior_toggle_count;
        self.max_items_label = max_items_label;
        self.skip_window_enabled = skip_window_enabled;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn startup_toggle_count(&self) -> usize {
        self.startup_toggle_count
    }

    pub(crate) fn behavior_toggle_count(&self) -> usize {
        self.behavior_toggle_count
    }

    pub(crate) fn max_items_label(&self) -> &str {
        &self.max_items_label
    }

    pub(crate) fn skip_window_enabled(&self) -> bool {
        self.skip_window_enabled
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsGeneralPageSectionSessionState {
    fn record(
        &mut self,
        startup_toggle_count: usize,
        retention_control_count: usize,
        behavior_toggle_count: usize,
        sound_control_count: usize,
        skip_window_control_count: usize,
        position_control_count: usize,
        action_button_count: usize,
    ) {
        self.startup_toggle_count = startup_toggle_count;
        self.retention_control_count = retention_control_count;
        self.behavior_toggle_count = behavior_toggle_count;
        self.sound_control_count = sound_control_count;
        self.skip_window_control_count = skip_window_control_count;
        self.position_control_count = position_control_count;
        self.action_button_count = action_button_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn startup_toggle_count(&self) -> usize {
        self.startup_toggle_count
    }

    pub(crate) fn retention_control_count(&self) -> usize {
        self.retention_control_count
    }

    pub(crate) fn behavior_toggle_count(&self) -> usize {
        self.behavior_toggle_count
    }

    pub(crate) fn sound_control_count(&self) -> usize {
        self.sound_control_count
    }

    pub(crate) fn skip_window_control_count(&self) -> usize {
        self.skip_window_control_count
    }

    pub(crate) fn position_control_count(&self) -> usize {
        self.position_control_count
    }

    pub(crate) fn action_button_count(&self) -> usize {
        self.action_button_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsHotkeyPageSessionState {
    fn record(
        &mut self,
        main_hotkey_preview: String,
        plain_hotkey_preview: String,
        recording: bool,
    ) {
        self.main_hotkey_preview = main_hotkey_preview;
        self.plain_hotkey_preview = plain_hotkey_preview;
        self.recording = recording;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn main_hotkey_preview(&self) -> &str {
        &self.main_hotkey_preview
    }

    pub(crate) fn plain_hotkey_preview(&self) -> &str {
        &self.plain_hotkey_preview
    }

    pub(crate) fn recording(&self) -> bool {
        self.recording
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsHotkeyPageSectionSessionState {
    fn record(
        &mut self,
        main_shortcut_control_count: usize,
        plain_shortcut_control_count: usize,
        system_action_count: usize,
        note_label_count: usize,
    ) {
        self.main_shortcut_control_count = main_shortcut_control_count;
        self.plain_shortcut_control_count = plain_shortcut_control_count;
        self.system_action_count = system_action_count;
        self.note_label_count = note_label_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn main_shortcut_control_count(&self) -> usize {
        self.main_shortcut_control_count
    }

    pub(crate) fn plain_shortcut_control_count(&self) -> usize {
        self.plain_shortcut_control_count
    }

    pub(crate) fn system_action_count(&self) -> usize {
        self.system_action_count
    }

    pub(crate) fn note_label_count(&self) -> usize {
        self.note_label_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsPluginPageSessionState {
    fn record(
        &mut self,
        quick_search_enabled: bool,
        ocr_provider: String,
        translate_provider: String,
        tool_toggle_count: usize,
    ) {
        self.quick_search_enabled = quick_search_enabled;
        self.ocr_provider = ocr_provider;
        self.translate_provider = translate_provider;
        self.tool_toggle_count = tool_toggle_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn quick_search_enabled(&self) -> bool {
        self.quick_search_enabled
    }

    pub(crate) fn ocr_provider(&self) -> &str {
        &self.ocr_provider
    }

    pub(crate) fn translate_provider(&self) -> &str {
        &self.translate_provider
    }

    pub(crate) fn tool_toggle_count(&self) -> usize {
        self.tool_toggle_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsPluginPageSectionSessionState {
    fn record(
        &mut self,
        quick_search_control_count: usize,
        ocr_control_count: usize,
        translate_control_count: usize,
        tool_toggle_count: usize,
        tool_action_count: usize,
    ) {
        self.quick_search_control_count = quick_search_control_count;
        self.ocr_control_count = ocr_control_count;
        self.translate_control_count = translate_control_count;
        self.tool_toggle_count = tool_toggle_count;
        self.tool_action_count = tool_action_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn quick_search_control_count(&self) -> usize {
        self.quick_search_control_count
    }

    pub(crate) fn ocr_control_count(&self) -> usize {
        self.ocr_control_count
    }

    pub(crate) fn translate_control_count(&self) -> usize {
        self.translate_control_count
    }

    pub(crate) fn tool_toggle_count(&self) -> usize {
        self.tool_toggle_count
    }

    pub(crate) fn tool_action_count(&self) -> usize {
        self.tool_action_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsAboutPageSessionState {
    fn record(&mut self, source_available: bool, update_available: bool, data_dir: String) {
        self.source_available = source_available;
        self.update_available = update_available;
        self.data_dir = data_dir;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn source_available(&self) -> bool {
        self.source_available
    }

    pub(crate) fn update_available(&self) -> bool {
        self.update_available
    }

    pub(crate) fn data_dir(&self) -> &str {
        &self.data_dir
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsAboutPageSectionSessionState {
    fn record(
        &mut self,
        metadata_label_count: usize,
        source_link_count: usize,
        update_status_count: usize,
        update_action_count: usize,
        data_label_count: usize,
    ) {
        self.metadata_label_count = metadata_label_count;
        self.source_link_count = source_link_count;
        self.update_status_count = update_status_count;
        self.update_action_count = update_action_count;
        self.data_label_count = data_label_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn metadata_label_count(&self) -> usize {
        self.metadata_label_count
    }

    pub(crate) fn source_link_count(&self) -> usize {
        self.source_link_count
    }

    pub(crate) fn update_status_count(&self) -> usize {
        self.update_status_count
    }

    pub(crate) fn update_action_count(&self) -> usize {
        self.update_action_count
    }

    pub(crate) fn data_label_count(&self) -> usize {
        self.data_label_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsCloudPageSessionState {
    fn record(
        &mut self,
        selected_mode: String,
        pending_pair_count: usize,
        discovered_device_count: usize,
        selected_lan_row: Option<usize>,
    ) {
        self.selected_mode = selected_mode;
        self.pending_pair_count = pending_pair_count;
        self.discovered_device_count = discovered_device_count;
        self.selected_lan_row = selected_lan_row;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn selected_mode(&self) -> &str {
        &self.selected_mode
    }

    pub(crate) fn pending_pair_count(&self) -> usize {
        self.pending_pair_count
    }

    pub(crate) fn discovered_device_count(&self) -> usize {
        self.discovered_device_count
    }

    pub(crate) fn selected_lan_row(&self) -> Option<usize> {
        self.selected_lan_row
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsCloudWebdavPageSessionState {
    fn record(&mut self, field_count: usize, action_row_count: usize, status_label_count: usize) {
        self.field_count = field_count;
        self.action_row_count = action_row_count;
        self.status_label_count = status_label_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn field_count(&self) -> usize {
        self.field_count
    }

    pub(crate) fn action_row_count(&self) -> usize {
        self.action_row_count
    }

    pub(crate) fn status_label_count(&self) -> usize {
        self.status_label_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsCloudLanPageSessionState {
    fn record(
        &mut self,
        field_count: usize,
        action_row_count: usize,
        device_list_count: usize,
        qr_action_count: usize,
        helper_label_count: usize,
    ) {
        self.field_count = field_count;
        self.action_row_count = action_row_count;
        self.device_list_count = device_list_count;
        self.qr_action_count = qr_action_count;
        self.helper_label_count = helper_label_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn field_count(&self) -> usize {
        self.field_count
    }

    pub(crate) fn action_row_count(&self) -> usize {
        self.action_row_count
    }

    pub(crate) fn device_list_count(&self) -> usize {
        self.device_list_count
    }

    pub(crate) fn qr_action_count(&self) -> usize {
        self.qr_action_count
    }

    pub(crate) fn helper_label_count(&self) -> usize {
        self.helper_label_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsCloudLanDeviceListSessionState {
    fn record(
        &mut self,
        pending_pair_count: usize,
        discovered_device_count: usize,
        selected_pair_row: Option<usize>,
        selected_device_row: Option<usize>,
    ) {
        self.pending_pair_count = pending_pair_count;
        self.discovered_device_count = discovered_device_count;
        self.selected_pair_row = selected_pair_row;
        self.selected_device_row = selected_device_row;
        self.refresh_generation = self.refresh_generation.saturating_add(1);
    }

    pub(crate) fn pending_pair_count(&self) -> usize {
        self.pending_pair_count
    }

    pub(crate) fn discovered_device_count(&self) -> usize {
        self.discovered_device_count
    }

    pub(crate) fn selected_pair_row(&self) -> Option<usize> {
        self.selected_pair_row
    }

    pub(crate) fn selected_device_row(&self) -> Option<usize> {
        self.selected_device_row
    }

    pub(crate) fn refresh_generation(&self) -> u64 {
        self.refresh_generation
    }
}

impl MacosSettingsOwnerDrawSessionState {
    fn record(
        &mut self,
        hover_control_active: bool,
        qr_payload_available: bool,
        toggle_draw_count: usize,
        button_draw_count: usize,
    ) {
        self.hover_control_active = hover_control_active;
        self.qr_payload_available = qr_payload_available;
        self.toggle_draw_count = toggle_draw_count;
        self.button_draw_count = button_draw_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn hover_control_active(&self) -> bool {
        self.hover_control_active
    }

    pub(crate) fn qr_payload_available(&self) -> bool {
        self.qr_payload_available
    }

    pub(crate) fn toggle_draw_count(&self) -> usize {
        self.toggle_draw_count
    }

    pub(crate) fn button_draw_count(&self) -> usize {
        self.button_draw_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsOwnerDrawDomainSessionState {
    fn record(
        &mut self,
        qr_draw_count: usize,
        source_link_draw_count: usize,
        toggle_role_count: usize,
        dropdown_role_count: usize,
        accent_role_count: usize,
        button_role_count: usize,
    ) {
        self.qr_draw_count = qr_draw_count;
        self.source_link_draw_count = source_link_draw_count;
        self.toggle_role_count = toggle_role_count;
        self.dropdown_role_count = dropdown_role_count;
        self.accent_role_count = accent_role_count;
        self.button_role_count = button_role_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn qr_draw_count(&self) -> usize {
        self.qr_draw_count
    }

    pub(crate) fn source_link_draw_count(&self) -> usize {
        self.source_link_draw_count
    }

    pub(crate) fn toggle_role_count(&self) -> usize {
        self.toggle_role_count
    }

    pub(crate) fn dropdown_role_count(&self) -> usize {
        self.dropdown_role_count
    }

    pub(crate) fn accent_role_count(&self) -> usize {
        self.accent_role_count
    }

    pub(crate) fn button_role_count(&self) -> usize {
        self.button_role_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsPageBuilderSessionState {
    fn record(
        &mut self,
        registered_control_count: usize,
        ownerdraw_control_count: usize,
        section_count: usize,
    ) {
        self.registered_control_count = registered_control_count;
        self.ownerdraw_control_count = ownerdraw_control_count;
        self.section_count = section_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn registered_control_count(&self) -> usize {
        self.registered_control_count
    }

    pub(crate) fn ownerdraw_control_count(&self) -> usize {
        self.ownerdraw_control_count
    }

    pub(crate) fn section_count(&self) -> usize {
        self.section_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsRawControlSessionState {
    fn record(
        &mut self,
        label_control_count: usize,
        button_control_count: usize,
        dropdown_control_count: usize,
        input_control_count: usize,
        listbox_control_count: usize,
        toggle_row_count: usize,
    ) {
        self.label_control_count = label_control_count;
        self.button_control_count = button_control_count;
        self.dropdown_control_count = dropdown_control_count;
        self.input_control_count = input_control_count;
        self.listbox_control_count = listbox_control_count;
        self.toggle_row_count = toggle_row_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn label_control_count(&self) -> usize {
        self.label_control_count
    }

    pub(crate) fn button_control_count(&self) -> usize {
        self.button_control_count
    }

    pub(crate) fn dropdown_control_count(&self) -> usize {
        self.dropdown_control_count
    }

    pub(crate) fn input_control_count(&self) -> usize {
        self.input_control_count
    }

    pub(crate) fn listbox_control_count(&self) -> usize {
        self.listbox_control_count
    }

    pub(crate) fn toggle_row_count(&self) -> usize {
        self.toggle_row_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsFormActionSessionState {
    fn record(
        &mut self,
        ownerdraw_action_count: usize,
        action_row_count: usize,
        qr_action_count: usize,
        toggle_action_count: usize,
    ) {
        self.ownerdraw_action_count = ownerdraw_action_count;
        self.action_row_count = action_row_count;
        self.qr_action_count = qr_action_count;
        self.toggle_action_count = toggle_action_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn ownerdraw_action_count(&self) -> usize {
        self.ownerdraw_action_count
    }

    pub(crate) fn action_row_count(&self) -> usize {
        self.action_row_count
    }

    pub(crate) fn qr_action_count(&self) -> usize {
        self.qr_action_count
    }

    pub(crate) fn toggle_action_count(&self) -> usize {
        self.toggle_action_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsFormFieldSessionState {
    fn record(
        &mut self,
        label_row_count: usize,
        value_label_row_count: usize,
        dropdown_row_count: usize,
        input_row_count: usize,
        button_row_count: usize,
    ) {
        self.label_row_count = label_row_count;
        self.value_label_row_count = value_label_row_count;
        self.dropdown_row_count = dropdown_row_count;
        self.input_row_count = input_row_count;
        self.button_row_count = button_row_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn label_row_count(&self) -> usize {
        self.label_row_count
    }

    pub(crate) fn value_label_row_count(&self) -> usize {
        self.value_label_row_count
    }

    pub(crate) fn dropdown_row_count(&self) -> usize {
        self.dropdown_row_count
    }

    pub(crate) fn input_row_count(&self) -> usize {
        self.input_row_count
    }

    pub(crate) fn button_row_count(&self) -> usize {
        self.button_row_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsControlFactorySessionState {
    fn record(
        &mut self,
        label_count: usize,
        input_count: usize,
        listbox_count: usize,
        action_button_count: usize,
        toggle_count: usize,
    ) {
        self.label_count = label_count;
        self.input_count = input_count;
        self.listbox_count = listbox_count;
        self.action_button_count = action_button_count;
        self.toggle_count = toggle_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn label_count(&self) -> usize {
        self.label_count
    }

    pub(crate) fn input_count(&self) -> usize {
        self.input_count
    }

    pub(crate) fn listbox_count(&self) -> usize {
        self.listbox_count
    }

    pub(crate) fn action_button_count(&self) -> usize {
        self.action_button_count
    }

    pub(crate) fn toggle_count(&self) -> usize {
        self.toggle_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsControlRegistrySessionState {
    fn record(
        &mut self,
        registered_control_count: usize,
        scrollable_control_count: usize,
        page_count: usize,
    ) {
        self.registered_control_count = registered_control_count;
        self.scrollable_control_count = scrollable_control_count;
        self.page_count = page_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn registered_control_count(&self) -> usize {
        self.registered_control_count
    }

    pub(crate) fn scrollable_control_count(&self) -> usize {
        self.scrollable_control_count
    }

    pub(crate) fn page_count(&self) -> usize {
        self.page_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsPageNavigationSessionState {
    fn record(&mut self, current_page: SettingsPage, scroll_y: i32, reposition_count: usize) {
        self.current_page = current_page;
        self.scroll_y = scroll_y;
        self.reposition_count = reposition_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn current_page(&self) -> SettingsPage {
        self.current_page
    }

    pub(crate) fn scroll_y(&self) -> i32 {
        self.scroll_y
    }

    pub(crate) fn reposition_count(&self) -> usize {
        self.reposition_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsPageNavigationDomainSessionState {
    fn record(
        &mut self,
        control_reposition_count: usize,
        scroll_update_count: usize,
        page_switch_count: usize,
        visibility_update_count: usize,
        redraw_count: usize,
    ) {
        self.control_reposition_count = control_reposition_count;
        self.scroll_update_count = scroll_update_count;
        self.page_switch_count = page_switch_count;
        self.visibility_update_count = visibility_update_count;
        self.redraw_count = redraw_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn control_reposition_count(&self) -> usize {
        self.control_reposition_count
    }

    pub(crate) fn scroll_update_count(&self) -> usize {
        self.scroll_update_count
    }

    pub(crate) fn page_switch_count(&self) -> usize {
        self.page_switch_count
    }

    pub(crate) fn visibility_update_count(&self) -> usize {
        self.visibility_update_count
    }

    pub(crate) fn redraw_count(&self) -> usize {
        self.redraw_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsPageEnsureSessionState {
    fn record(&mut self, ensured_page: SettingsPage, built_page_count: usize) {
        self.ensured_page = Some(ensured_page);
        self.built_page_count = built_page_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn ensured_page(&self) -> Option<SettingsPage> {
        self.ensured_page
    }

    pub(crate) fn built_page_count(&self) -> usize {
        self.built_page_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsPageSyncSessionState {
    fn record(
        &mut self,
        synced_page_count: usize,
        enabled_control_count: usize,
        invalidation_count: usize,
    ) {
        self.synced_page_count = synced_page_count;
        self.enabled_control_count = enabled_control_count;
        self.invalidation_count = invalidation_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn synced_page_count(&self) -> usize {
        self.synced_page_count
    }

    pub(crate) fn enabled_control_count(&self) -> usize {
        self.enabled_control_count
    }

    pub(crate) fn invalidation_count(&self) -> usize {
        self.invalidation_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsCloudSyncSessionState {
    fn record(
        &mut self,
        mode: impl Into<String>,
        webdav_control_count: usize,
        lan_control_count: usize,
        lan_refreshed: bool,
    ) {
        self.mode = mode.into();
        self.webdav_control_count = webdav_control_count;
        self.lan_control_count = lan_control_count;
        if lan_refreshed {
            self.lan_refresh_generation = self.lan_refresh_generation.saturating_add(1);
        }
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn mode(&self) -> &str {
        &self.mode
    }

    pub(crate) fn webdav_control_count(&self) -> usize {
        self.webdav_control_count
    }

    pub(crate) fn lan_control_count(&self) -> usize {
        self.lan_control_count
    }

    pub(crate) fn lan_refresh_generation(&self) -> u64 {
        self.lan_refresh_generation
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsCloudWebdavSyncSessionState {
    fn record(&mut self, control_count: usize, enabled: bool, status_text_available: bool) {
        self.control_count = control_count;
        self.enabled = enabled;
        self.status_text_available = status_text_available;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn control_count(&self) -> usize {
        self.control_count
    }

    pub(crate) fn enabled(&self) -> bool {
        self.enabled
    }

    pub(crate) fn status_text_available(&self) -> bool {
        self.status_text_available
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsCloudLanSyncSessionState {
    fn record(
        &mut self,
        control_count: usize,
        enabled: bool,
        list_refreshed: bool,
        invalidation_count: usize,
    ) {
        self.control_count = control_count;
        self.enabled = enabled;
        self.list_refreshed = list_refreshed;
        self.invalidation_count = invalidation_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn control_count(&self) -> usize {
        self.control_count
    }

    pub(crate) fn enabled(&self) -> bool {
        self.enabled
    }

    pub(crate) fn list_refreshed(&self) -> bool {
        self.list_refreshed
    }

    pub(crate) fn invalidation_count(&self) -> usize {
        self.invalidation_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsPluginSyncSessionState {
    fn record(
        &mut self,
        search_enabled: bool,
        ocr_fields_visible: bool,
        translate_enabled: bool,
        tool_control_count: usize,
    ) {
        self.search_enabled = search_enabled;
        self.ocr_fields_visible = ocr_fields_visible;
        self.translate_enabled = translate_enabled;
        self.tool_control_count = tool_control_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn search_enabled(&self) -> bool {
        self.search_enabled
    }

    pub(crate) fn ocr_fields_visible(&self) -> bool {
        self.ocr_fields_visible
    }

    pub(crate) fn translate_enabled(&self) -> bool {
        self.translate_enabled
    }

    pub(crate) fn tool_control_count(&self) -> usize {
        self.tool_control_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsControlSelectionSessionState {
    fn record(
        &mut self,
        general_selection_count: usize,
        cloud_selection_count: usize,
        hotkey_selection_count: usize,
        plugin_selection_count: usize,
        group_selection_count: usize,
    ) {
        self.general_selection_count = general_selection_count;
        self.cloud_selection_count = cloud_selection_count;
        self.hotkey_selection_count = hotkey_selection_count;
        self.plugin_selection_count = plugin_selection_count;
        self.group_selection_count = group_selection_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn general_selection_count(&self) -> usize {
        self.general_selection_count
    }

    pub(crate) fn cloud_selection_count(&self) -> usize {
        self.cloud_selection_count
    }

    pub(crate) fn hotkey_selection_count(&self) -> usize {
        self.hotkey_selection_count
    }

    pub(crate) fn plugin_selection_count(&self) -> usize {
        self.plugin_selection_count
    }

    pub(crate) fn group_selection_count(&self) -> usize {
        self.group_selection_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsDropdownPluginSessionState {
    fn record(
        &mut self,
        search_option_count: usize,
        ocr_option_count: usize,
        translate_provider_count: usize,
        translate_target_count: usize,
    ) {
        self.search_option_count = search_option_count;
        self.ocr_option_count = ocr_option_count;
        self.translate_provider_count = translate_provider_count;
        self.translate_target_count = translate_target_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn search_option_count(&self) -> usize {
        self.search_option_count
    }

    pub(crate) fn ocr_option_count(&self) -> usize {
        self.ocr_option_count
    }

    pub(crate) fn translate_provider_count(&self) -> usize {
        self.translate_provider_count
    }

    pub(crate) fn translate_target_count(&self) -> usize {
        self.translate_target_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsDropdownDomainSessionState {
    fn record(
        &mut self,
        general_dropdown_count: usize,
        cloud_dropdown_count: usize,
        hotkey_dropdown_count: usize,
        plugin_dropdown_count: usize,
        group_dropdown_count: usize,
    ) {
        self.general_dropdown_count = general_dropdown_count;
        self.cloud_dropdown_count = cloud_dropdown_count;
        self.hotkey_dropdown_count = hotkey_dropdown_count;
        self.plugin_dropdown_count = plugin_dropdown_count;
        self.group_dropdown_count = group_dropdown_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn general_dropdown_count(&self) -> usize {
        self.general_dropdown_count
    }

    pub(crate) fn cloud_dropdown_count(&self) -> usize {
        self.cloud_dropdown_count
    }

    pub(crate) fn hotkey_dropdown_count(&self) -> usize {
        self.hotkey_dropdown_count
    }

    pub(crate) fn plugin_dropdown_count(&self) -> usize {
        self.plugin_dropdown_count
    }

    pub(crate) fn group_dropdown_count(&self) -> usize {
        self.group_dropdown_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsToggleStateSessionState {
    fn record(&mut self, toggled_control_id: i32, enabled_toggle_count: usize) {
        self.toggled_control_id = toggled_control_id;
        self.enabled_toggle_count = enabled_toggle_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn toggled_control_id(&self) -> i32 {
        self.toggled_control_id
    }

    pub(crate) fn enabled_toggle_count(&self) -> usize {
        self.enabled_toggle_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsToggleDomainSessionState {
    fn record(
        &mut self,
        general_toggle_count: usize,
        cloud_toggle_count: usize,
        hotkey_toggle_count: usize,
        plugin_toggle_count: usize,
        group_toggle_count: usize,
    ) {
        self.general_toggle_count = general_toggle_count;
        self.cloud_toggle_count = cloud_toggle_count;
        self.hotkey_toggle_count = hotkey_toggle_count;
        self.plugin_toggle_count = plugin_toggle_count;
        self.group_toggle_count = group_toggle_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn general_toggle_count(&self) -> usize {
        self.general_toggle_count
    }

    pub(crate) fn cloud_toggle_count(&self) -> usize {
        self.cloud_toggle_count
    }

    pub(crate) fn hotkey_toggle_count(&self) -> usize {
        self.hotkey_toggle_count
    }

    pub(crate) fn plugin_toggle_count(&self) -> usize {
        self.plugin_toggle_count
    }

    pub(crate) fn group_toggle_count(&self) -> usize {
        self.group_toggle_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsHostHelperSessionState {
    fn record(
        &mut self,
        text_update_count: usize,
        invalidation_count: usize,
        theme_refreshed: bool,
    ) {
        self.text_update_count = text_update_count;
        self.invalidation_count = invalidation_count;
        if theme_refreshed {
            self.theme_generation = self.theme_generation.saturating_add(1);
        }
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn text_update_count(&self) -> usize {
        self.text_update_count
    }

    pub(crate) fn invalidation_count(&self) -> usize {
        self.invalidation_count
    }

    pub(crate) fn theme_generation(&self) -> u64 {
        self.theme_generation
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsAppApplyCollectSessionState {
    fn record(
        &mut self,
        applied: bool,
        collected: bool,
        saved_settings_count: usize,
        peer_synced: bool,
    ) {
        if applied {
            self.applied_generation = self.applied_generation.saturating_add(1);
        }
        if collected {
            self.collected_generation = self.collected_generation.saturating_add(1);
        }
        self.saved_settings_count = saved_settings_count;
        if peer_synced {
            self.peer_sync_generation = self.peer_sync_generation.saturating_add(1);
        }
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn applied_generation(&self) -> u64 {
        self.applied_generation
    }

    pub(crate) fn collected_generation(&self) -> u64 {
        self.collected_generation
    }

    pub(crate) fn saved_settings_count(&self) -> usize {
        self.saved_settings_count
    }

    pub(crate) fn peer_sync_generation(&self) -> u64 {
        self.peer_sync_generation
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsAppCollectDomainSessionState {
    fn record(
        &mut self,
        general_collect_count: usize,
        hotkey_collect_count: usize,
        plugin_collect_count: usize,
        group_collect_count: usize,
        cloud_collect_count: usize,
    ) {
        self.general_collect_count = general_collect_count;
        self.hotkey_collect_count = hotkey_collect_count;
        self.plugin_collect_count = plugin_collect_count;
        self.group_collect_count = group_collect_count;
        self.cloud_collect_count = cloud_collect_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn general_collect_count(&self) -> usize {
        self.general_collect_count
    }

    pub(crate) fn hotkey_collect_count(&self) -> usize {
        self.hotkey_collect_count
    }

    pub(crate) fn plugin_collect_count(&self) -> usize {
        self.plugin_collect_count
    }

    pub(crate) fn group_collect_count(&self) -> usize {
        self.group_collect_count
    }

    pub(crate) fn cloud_collect_count(&self) -> usize {
        self.cloud_collect_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsAppEffectsSessionState {
    fn record(
        &mut self,
        persisted: bool,
        integration_refreshed: bool,
        data_refreshed: bool,
        window_refreshed: bool,
        peer_synced: bool,
    ) {
        if persisted {
            self.persisted_generation = self.persisted_generation.saturating_add(1);
        }
        if integration_refreshed {
            self.integration_refresh_generation =
                self.integration_refresh_generation.saturating_add(1);
        }
        if data_refreshed {
            self.data_refresh_generation = self.data_refresh_generation.saturating_add(1);
        }
        if window_refreshed {
            self.window_refresh_generation = self.window_refresh_generation.saturating_add(1);
        }
        if peer_synced {
            self.peer_sync_generation = self.peer_sync_generation.saturating_add(1);
        }
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn persisted_generation(&self) -> u64 {
        self.persisted_generation
    }

    pub(crate) fn integration_refresh_generation(&self) -> u64 {
        self.integration_refresh_generation
    }

    pub(crate) fn data_refresh_generation(&self) -> u64 {
        self.data_refresh_generation
    }

    pub(crate) fn window_refresh_generation(&self) -> u64 {
        self.window_refresh_generation
    }

    pub(crate) fn peer_sync_generation(&self) -> u64 {
        self.peer_sync_generation
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsSyncActionDomainSessionState {
    fn record(&mut self, webdav_action_count: usize, lan_action_count: usize) {
        self.webdav_action_count = webdav_action_count;
        self.lan_action_count = lan_action_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn webdav_action_count(&self) -> usize {
        self.webdav_action_count
    }

    pub(crate) fn lan_action_count(&self) -> usize {
        self.lan_action_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsPlatformActionDomainSessionState {
    fn record(
        &mut self,
        hotkey_action_count: usize,
        general_action_count: usize,
        plugin_action_count: usize,
        about_action_count: usize,
        system_action_count: usize,
    ) {
        self.hotkey_action_count = hotkey_action_count;
        self.general_action_count = general_action_count;
        self.plugin_action_count = plugin_action_count;
        self.about_action_count = about_action_count;
        self.system_action_count = system_action_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn hotkey_action_count(&self) -> usize {
        self.hotkey_action_count
    }

    pub(crate) fn general_action_count(&self) -> usize {
        self.general_action_count
    }

    pub(crate) fn plugin_action_count(&self) -> usize {
        self.plugin_action_count
    }

    pub(crate) fn about_action_count(&self) -> usize {
        self.about_action_count
    }

    pub(crate) fn system_action_count(&self) -> usize {
        self.system_action_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsWindowStateSessionState {
    fn record(
        &mut self,
        initial_page: SettingsPage,
        ui_dpi: u32,
        reset_control_count: usize,
        dynamic_section_count: usize,
    ) {
        self.initial_page = initial_page;
        self.ui_dpi = ui_dpi.max(1);
        self.reset_control_count = reset_control_count;
        self.dynamic_section_count = dynamic_section_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn initial_page(&self) -> SettingsPage {
        self.initial_page
    }

    pub(crate) fn ui_dpi(&self) -> u32 {
        self.ui_dpi
    }

    pub(crate) fn reset_control_count(&self) -> usize {
        self.reset_control_count
    }

    pub(crate) fn dynamic_section_count(&self) -> usize {
        self.dynamic_section_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsWindowCreateSessionState {
    fn record(
        &mut self,
        parent: MacosSettingsWindowHandle,
        initial_page: SettingsPage,
        save_close_control_count: usize,
        page_built_count: usize,
        applied: bool,
    ) {
        self.parent = Some(parent);
        self.initial_page = initial_page;
        self.save_close_control_count = save_close_control_count;
        self.page_built_count = page_built_count;
        if applied {
            self.applied_generation = self.applied_generation.saturating_add(1);
        }
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn parent(&self) -> Option<MacosSettingsWindowHandle> {
        self.parent
    }

    pub(crate) fn initial_page(&self) -> SettingsPage {
        self.initial_page
    }

    pub(crate) fn save_close_control_count(&self) -> usize {
        self.save_close_control_count
    }

    pub(crate) fn page_built_count(&self) -> usize {
        self.page_built_count
    }

    pub(crate) fn applied_generation(&self) -> u64 {
        self.applied_generation
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsWindowMetricsSessionState {
    fn record(
        &mut self,
        measured_content_height: i32,
        scroll_slot_count: usize,
        rebuilt_page_count: usize,
        visible_control_count: usize,
    ) {
        self.measured_content_height = measured_content_height.max(0);
        self.scroll_slot_count = scroll_slot_count;
        self.rebuilt_page_count = rebuilt_page_count;
        self.visible_control_count = visible_control_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn measured_content_height(&self) -> i32 {
        self.measured_content_height
    }

    pub(crate) fn scroll_slot_count(&self) -> usize {
        self.scroll_slot_count
    }

    pub(crate) fn rebuilt_page_count(&self) -> usize {
        self.rebuilt_page_count
    }

    pub(crate) fn visible_control_count(&self) -> usize {
        self.visible_control_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsWindowLayoutSessionState {
    fn record(
        &mut self,
        layout_dpi: u32,
        client_bounds: UiRect,
        window_bounds: UiRect,
        move_plan_count: usize,
    ) {
        self.layout_dpi = layout_dpi.max(1);
        self.client_bounds = Some(client_bounds);
        self.window_bounds = Some(window_bounds);
        self.move_plan_count = move_plan_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn layout_dpi(&self) -> u32 {
        self.layout_dpi
    }

    pub(crate) fn client_bounds(&self) -> Option<UiRect> {
        self.client_bounds
    }

    pub(crate) fn window_bounds(&self) -> Option<UiRect> {
        self.window_bounds
    }

    pub(crate) fn move_plan_count(&self) -> usize {
        self.move_plan_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsWindowLifecycleSessionState {
    fn record(
        &mut self,
        presented: bool,
        bounds_updated: bool,
        focused: bool,
        destroyed: bool,
        pointer_captured: bool,
        repainted: bool,
        cloud_refreshed: bool,
    ) {
        if presented {
            self.presented_generation = self.presented_generation.saturating_add(1);
        }
        if bounds_updated {
            self.bounds_update_generation = self.bounds_update_generation.saturating_add(1);
        }
        if focused {
            self.focused_generation = self.focused_generation.saturating_add(1);
        }
        if destroyed {
            self.destroyed_generation = self.destroyed_generation.saturating_add(1);
        }
        if pointer_captured {
            self.pointer_capture_generation = self.pointer_capture_generation.saturating_add(1);
        }
        if repainted {
            self.repaint_generation = self.repaint_generation.saturating_add(1);
        }
        if cloud_refreshed {
            self.cloud_refresh_generation = self.cloud_refresh_generation.saturating_add(1);
        }
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn presented_generation(&self) -> u64 {
        self.presented_generation
    }

    pub(crate) fn bounds_update_generation(&self) -> u64 {
        self.bounds_update_generation
    }

    pub(crate) fn focused_generation(&self) -> u64 {
        self.focused_generation
    }

    pub(crate) fn destroyed_generation(&self) -> u64 {
        self.destroyed_generation
    }

    pub(crate) fn pointer_capture_generation(&self) -> u64 {
        self.pointer_capture_generation
    }

    pub(crate) fn repaint_generation(&self) -> u64 {
        self.repaint_generation
    }

    pub(crate) fn cloud_refresh_generation(&self) -> u64 {
        self.cloud_refresh_generation
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsWindowDestroySessionState {
    fn record(
        &mut self,
        timer_cleanup_count: usize,
        dropdown_cleanup_count: usize,
        resource_cleanup_count: usize,
    ) {
        self.timer_cleanup_count = timer_cleanup_count;
        self.dropdown_cleanup_count = dropdown_cleanup_count;
        self.resource_cleanup_count = resource_cleanup_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn timer_cleanup_count(&self) -> usize {
        self.timer_cleanup_count
    }

    pub(crate) fn dropdown_cleanup_count(&self) -> usize {
        self.dropdown_cleanup_count
    }

    pub(crate) fn resource_cleanup_count(&self) -> usize {
        self.resource_cleanup_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsWindowColorSessionState {
    fn record(
        &mut self,
        surface_role_count: usize,
        edit_role_count: usize,
        list_role_count: usize,
    ) {
        self.surface_role_count = surface_role_count;
        self.edit_role_count = edit_role_count;
        self.list_role_count = list_role_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn surface_role_count(&self) -> usize {
        self.surface_role_count
    }

    pub(crate) fn edit_role_count(&self) -> usize {
        self.edit_role_count
    }

    pub(crate) fn list_role_count(&self) -> usize {
        self.list_role_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsWindowSurfaceControlSessionState {
    fn record(
        &mut self,
        general_count: usize,
        hotkey_count: usize,
        group_count: usize,
        cloud_count: usize,
        plugin_count: usize,
        about_count: usize,
    ) {
        self.general_count = general_count;
        self.hotkey_count = hotkey_count;
        self.group_count = group_count;
        self.cloud_count = cloud_count;
        self.plugin_count = plugin_count;
        self.about_count = about_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn general_count(&self) -> usize {
        self.general_count
    }

    pub(crate) fn hotkey_count(&self) -> usize {
        self.hotkey_count
    }

    pub(crate) fn group_count(&self) -> usize {
        self.group_count
    }

    pub(crate) fn cloud_count(&self) -> usize {
        self.cloud_count
    }

    pub(crate) fn plugin_count(&self) -> usize {
        self.plugin_count
    }

    pub(crate) fn about_count(&self) -> usize {
        self.about_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosSettingsWindowPaintSessionState {
    fn record(
        &mut self,
        chrome_painted: bool,
        content_painted: bool,
        scrollbar_painted: bool,
        owner_draw_count: usize,
    ) {
        if chrome_painted {
            self.chrome_paint_generation = self.chrome_paint_generation.saturating_add(1);
        }
        if content_painted {
            self.content_paint_generation = self.content_paint_generation.saturating_add(1);
        }
        if scrollbar_painted {
            self.scrollbar_paint_generation = self.scrollbar_paint_generation.saturating_add(1);
        }
        self.owner_draw_count = owner_draw_count;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn chrome_paint_generation(&self) -> u64 {
        self.chrome_paint_generation
    }

    pub(crate) fn content_paint_generation(&self) -> u64 {
        self.content_paint_generation
    }

    pub(crate) fn scrollbar_paint_generation(&self) -> u64 {
        self.scrollbar_paint_generation
    }

    pub(crate) fn owner_draw_count(&self) -> usize {
        self.owner_draw_count
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosMainVisualSessionState {
    fn update(
        &mut self,
        title_buttons: TitleButtonVisibility,
        empty_state: crate::app_core::MainEmptyStateKind,
        image_preview_enabled: bool,
    ) {
        self.title_buttons = title_buttons;
        self.empty_state = Some(empty_state);
        self.image_preview_enabled = image_preview_enabled;
        self.visual_generation = self.visual_generation.saturating_add(1);
    }

    pub(crate) fn title_buttons(&self) -> TitleButtonVisibility {
        self.title_buttons
    }

    pub(crate) fn empty_state(&self) -> Option<crate::app_core::MainEmptyStateKind> {
        self.empty_state
    }

    pub(crate) fn image_preview_enabled(&self) -> bool {
        self.image_preview_enabled
    }

    pub(crate) fn visual_generation(&self) -> u64 {
        self.visual_generation
    }
}

impl MacosAdapterPreludeState {
    fn record(
        &mut self,
        shared_contract_roots: &[&'static str],
        native_adapter_roots: &[&'static str],
    ) {
        self.shared_contract_roots = shared_contract_roots.to_vec();
        self.native_adapter_roots = native_adapter_roots.to_vec();
        self.boundary_generation = self.boundary_generation.saturating_add(1);
    }

    pub(crate) fn shared_contract_roots(&self) -> &[&'static str] {
        &self.shared_contract_roots
    }

    pub(crate) fn native_adapter_roots(&self) -> &[&'static str] {
        &self.native_adapter_roots
    }

    pub(crate) fn boundary_generation(&self) -> u64 {
        self.boundary_generation
    }
}

impl MacosNativeIdSessionState {
    fn record(
        &mut self,
        window_identifiers: &[&'static str],
        timer_identifiers: &[&'static str],
        status_item_identifier: &'static str,
    ) {
        self.window_identifiers = window_identifiers.to_vec();
        self.timer_identifiers = timer_identifiers.to_vec();
        self.status_item_identifier = Some(status_item_identifier);
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn window_identifiers(&self) -> &[&'static str] {
        &self.window_identifiers
    }

    pub(crate) fn timer_identifiers(&self) -> &[&'static str] {
        &self.timer_identifiers
    }

    pub(crate) fn status_item_identifier(&self) -> Option<&'static str> {
        self.status_item_identifier
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosMainSearchSessionState {
    fn record(
        &mut self,
        handle: MacosMainSearchControlHandle,
        visible: bool,
        text: impl Into<String>,
        style_resource: Option<MacosMainSearchStyleResource>,
    ) {
        self.handle = Some(handle);
        self.visible = visible;
        self.text = text.into();
        self.style_resource = style_resource;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn handle(&self) -> Option<MacosMainSearchControlHandle> {
        self.handle
    }

    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn style_resource(&self) -> Option<MacosMainSearchStyleResource> {
        self.style_resource
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosTransientWindowSessionState {
    fn record(
        &mut self,
        owner: MacosMainWindowHandle,
        handle: MacosTransientWindowHandle,
        bounds: UiRect,
        visible: bool,
    ) {
        self.owner = Some(owner);
        self.handle = Some(handle);
        self.bounds = Some(bounds);
        self.visible = visible;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn handle(&self) -> Option<MacosTransientWindowHandle> {
        self.handle
    }

    pub(crate) fn owner(&self) -> Option<MacosMainWindowHandle> {
        self.owner
    }

    pub(crate) fn bounds(&self) -> Option<UiRect> {
        self.bounds
    }

    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosPasteTargetDiscoverySessionState {
    fn record(
        &mut self,
        skip_class_names: impl Into<String>,
        last_candidate: Option<MacosPasteTargetHandle>,
    ) {
        self.skip_class_names = skip_class_names.into();
        self.last_candidate = last_candidate;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn skip_class_names(&self) -> &str {
        &self.skip_class_names
    }

    pub(crate) fn last_candidate(&self) -> Option<MacosPasteTargetHandle> {
        self.last_candidate
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosLowLevelInputSessionState {
    fn record(
        &mut self,
        outside_hide_timer_active: bool,
        edge_auto_hide_timer_active: bool,
        quick_escape_monitor_active: bool,
        last_pointer_scope: MacosPointerScope,
    ) {
        self.outside_hide_timer_active = outside_hide_timer_active;
        self.edge_auto_hide_timer_active = edge_auto_hide_timer_active;
        self.quick_escape_monitor_active = quick_escape_monitor_active;
        self.last_pointer_scope = Some(last_pointer_scope);
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn outside_hide_timer_active(&self) -> bool {
        self.outside_hide_timer_active
    }

    pub(crate) fn edge_auto_hide_timer_active(&self) -> bool {
        self.edge_auto_hide_timer_active
    }

    pub(crate) fn quick_escape_monitor_active(&self) -> bool {
        self.quick_escape_monitor_active
    }

    pub(crate) fn last_pointer_scope(&self) -> Option<MacosPointerScope> {
        self.last_pointer_scope
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosHoverPreviewSessionState {
    fn record(
        &mut self,
        visible: bool,
        hovered_item_id: Option<i64>,
        mouse_leave_tracking_active: bool,
    ) {
        self.visible = visible;
        self.hovered_item_id = hovered_item_id;
        self.mouse_leave_tracking_active = mouse_leave_tracking_active;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn hovered_item_id(&self) -> Option<i64> {
        self.hovered_item_id
    }

    pub(crate) fn mouse_leave_tracking_active(&self) -> bool {
        self.mouse_leave_tracking_active
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosStartupIntegrationSessionState {
    fn record(
        &mut self,
        status_item_registered: bool,
        hotkeys_registered: bool,
        clipboard_monitor_registered: bool,
        vv_monitor_registered: bool,
        recovery_ticks: u32,
    ) {
        self.status_item_registered = status_item_registered;
        self.hotkeys_registered = hotkeys_registered;
        self.clipboard_monitor_registered = clipboard_monitor_registered;
        self.vv_monitor_registered = vv_monitor_registered;
        self.recovery_ticks = recovery_ticks;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn status_item_registered(&self) -> bool {
        self.status_item_registered
    }

    pub(crate) fn hotkeys_registered(&self) -> bool {
        self.hotkeys_registered
    }

    pub(crate) fn clipboard_monitor_registered(&self) -> bool {
        self.clipboard_monitor_registered
    }

    pub(crate) fn vv_monitor_registered(&self) -> bool {
        self.vv_monitor_registered
    }

    pub(crate) fn recovery_ticks(&self) -> u32 {
        self.recovery_ticks
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl Default for MacosAutostartHost {
    fn default() -> Self {
        Self {
            launch_agents_dir: macos_launch_agents_dir(),
            executable_path: env::current_exe().ok(),
        }
    }
}

impl MacosAutostartHost {
    pub(crate) fn with_paths(launch_agents_dir: PathBuf, executable_path: PathBuf) -> Self {
        Self {
            launch_agents_dir,
            executable_path: Some(executable_path),
        }
    }

    fn launch_agent_path(&self) -> PathBuf {
        self.launch_agents_dir
            .join("io.github.qiu7824.zsclip.plist")
    }

    fn executable_path(&self) -> Result<&Path, String> {
        self.executable_path
            .as_deref()
            .ok_or_else(|| "current executable path is unavailable".to_string())
    }
}

impl NativeAutostartHost for MacosAutostartHost {
    fn autostart_status(&self) -> NativeAutostartStatus {
        let launch_agent_path = self.launch_agent_path();
        let Ok(plist) = fs::read_to_string(&launch_agent_path) else {
            return NativeAutostartStatus::disabled();
        };
        let Some(executable_path) = self.executable_path.as_ref() else {
            return NativeAutostartStatus::disabled();
        };
        if plist.contains("io.github.qiu7824.zsclip")
            && plist.contains(&macos_plist_escape(&executable_path.to_string_lossy()))
        {
            NativeAutostartStatus::enabled_at(launch_agent_path.to_string_lossy())
        } else {
            NativeAutostartStatus::disabled()
        }
    }

    fn set_autostart_enabled(&mut self, enabled: bool) -> NativeAutostartApplyResult {
        let launch_agent_path = self.launch_agent_path();
        if enabled {
            let executable_path = match self.executable_path() {
                Ok(path) => path,
                Err(err) => return NativeAutostartApplyResult::failed(true, err),
            };
            if let Err(err) = fs::create_dir_all(&self.launch_agents_dir) {
                return NativeAutostartApplyResult::failed(true, err.to_string());
            }
            let plist = macos_launch_agent_plist(executable_path);
            if let Err(err) = fs::write(&launch_agent_path, plist) {
                return NativeAutostartApplyResult::failed(true, err.to_string());
            }
            return NativeAutostartApplyResult::applied(true, self.autostart_status());
        }

        match fs::remove_file(&launch_agent_path) {
            Ok(()) => NativeAutostartApplyResult::applied(false, self.autostart_status()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                NativeAutostartApplyResult::applied(false, self.autostart_status())
            }
            Err(err) => NativeAutostartApplyResult::failed(false, err.to_string()),
        }
    }
}

fn macos_launch_agents_dir() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Library")
        .join("LaunchAgents")
}

fn macos_launch_agent_plist(executable_path: &Path) -> String {
    let executable = macos_plist_escape(&executable_path.to_string_lossy());
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>io.github.qiu7824.zsclip</string>
    <key>ProgramArguments</key>
    <array>
        <string>{executable}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
"#
    )
}

fn macos_plist_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

impl MacosWindowRefreshSessionState {
    fn record(
        &mut self,
        reload_settings: bool,
        reload_database: bool,
        refresh_settings_window: bool,
        peer_source: Option<MacosMainWindowHandle>,
    ) {
        if reload_settings {
            self.settings_reload_generation = self.settings_reload_generation.saturating_add(1);
        }
        if reload_database {
            self.database_reload_generation = self.database_reload_generation.saturating_add(1);
        }
        if refresh_settings_window {
            self.settings_window_refresh_generation =
                self.settings_window_refresh_generation.saturating_add(1);
        }
        if peer_source.is_some() {
            self.peer_sync_generation = self.peer_sync_generation.saturating_add(1);
        }
        self.last_peer_source = peer_source;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn settings_reload_generation(&self) -> u64 {
        self.settings_reload_generation
    }

    pub(crate) fn database_reload_generation(&self) -> u64 {
        self.database_reload_generation
    }

    pub(crate) fn settings_window_refresh_generation(&self) -> u64 {
        self.settings_window_refresh_generation
    }

    pub(crate) fn peer_sync_generation(&self) -> u64 {
        self.peer_sync_generation
    }

    pub(crate) fn last_peer_source(&self) -> Option<MacosMainWindowHandle> {
        self.last_peer_source
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosWindowRegistrySessionState {
    fn record(
        &mut self,
        main: Option<MacosMainWindowHandle>,
        quick: Option<MacosMainWindowHandle>,
        clipboard_ignore: bool,
        skip_next_clipboard: bool,
    ) {
        self.main = main;
        self.quick = quick;
        if clipboard_ignore {
            self.clipboard_ignore_generation = self.clipboard_ignore_generation.saturating_add(1);
        }
        if skip_next_clipboard {
            self.skip_next_clipboard_generation =
                self.skip_next_clipboard_generation.saturating_add(1);
        }
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn main(&self) -> Option<MacosMainWindowHandle> {
        self.main
    }

    pub(crate) fn quick(&self) -> Option<MacosMainWindowHandle> {
        self.quick
    }

    pub(crate) fn clipboard_ignore_generation(&self) -> u64 {
        self.clipboard_ignore_generation
    }

    pub(crate) fn skip_next_clipboard_generation(&self) -> u64 {
        self.skip_next_clipboard_generation
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosHoverClearSessionState {
    fn record(
        &mut self,
        preserved_scrollbar_hover: bool,
        cleared_pointer_down_state: bool,
        noactivate_hit_item: bool,
    ) {
        self.preserved_scrollbar_hover = preserved_scrollbar_hover;
        self.cleared_pointer_down_state = cleared_pointer_down_state;
        self.noactivate_hit_item = noactivate_hit_item;
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn preserved_scrollbar_hover(&self) -> bool {
        self.preserved_scrollbar_hover
    }

    pub(crate) fn cleared_pointer_down_state(&self) -> bool {
        self.cleared_pointer_down_state
    }

    pub(crate) fn noactivate_hit_item(&self) -> bool {
        self.noactivate_hit_item
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl MacosMainEventModel {
    pub(crate) fn accept_application_event(
        &mut self,
        event: ApplicationEvent,
    ) -> MacosApplicationEventRoute {
        let route = match event {
            ApplicationEvent::LanSyncReady => MacosApplicationEventRoute::RefreshLan,
            ApplicationEvent::VvShowRequested { target } => {
                MacosApplicationEventRoute::ScheduleVvShow { target }
            }
            ApplicationEvent::VvHideRequested => MacosApplicationEventRoute::HideVv,
            ApplicationEvent::VvSelectRequested { index } => {
                MacosApplicationEventRoute::SelectVv { index }
            }
            ApplicationEvent::ClipboardChanged { sequence } => {
                MacosApplicationEventRoute::CaptureClipboardChange { sequence }
            }
            ApplicationEvent::ItemsPageReady => MacosApplicationEventRoute::RefreshItems,
            ApplicationEvent::StartupDataReconciled { deleted } => {
                MacosApplicationEventRoute::ReconcileData { deleted }
            }
            ApplicationEvent::CloudSyncReady => MacosApplicationEventRoute::ApplyCloudSync,
            ApplicationEvent::UpdateCheckReady => MacosApplicationEventRoute::RefreshSettings,
            ApplicationEvent::ShellIntegrationRestored => {
                MacosApplicationEventRoute::RestoreShellIntegration
            }
            ApplicationEvent::TrayCallback { code } => {
                MacosApplicationEventRoute::TrayCallback { code }
            }
        };
        self.application_routes.push(route);
        route
    }

    pub(crate) fn accept_async_event(
        &mut self,
        event: &MainAsyncEvent,
    ) -> MacosMainAsyncEventRoute {
        let route = match event {
            MainAsyncEvent::ImagePaste(payload) => MacosMainAsyncEventRoute::PasteImage {
                has_image: payload.image.is_some(),
                target: payload.target,
                hide_main: payload.hide_main,
                backspaces: payload.backspaces,
            },
            MainAsyncEvent::ImageOcr(payload) => MacosMainAsyncEventRoute::ImageOcr {
                has_text: payload.text.is_some(),
                has_error: payload.error.is_some(),
            },
            MainAsyncEvent::TextTranslate(payload) => MacosMainAsyncEventRoute::TextTranslate {
                has_text: payload.text.is_some(),
                has_error: payload.error.is_some(),
            },
            MainAsyncEvent::ImageThumbnail(payload) => MacosMainAsyncEventRoute::CacheThumbnail {
                item_id: payload.item_id,
                has_image: payload.image.is_some(),
            },
        };
        self.async_routes.push(route);
        route
    }

    pub(crate) fn application_routes(&self) -> &[MacosApplicationEventRoute] {
        &self.application_routes
    }

    pub(crate) fn async_routes(&self) -> &[MacosMainAsyncEventRoute] {
        &self.async_routes
    }
}

impl Default for MacosSettingsSnapshot {
    fn default() -> Self {
        Self {
            quick_search_enabled: false,
            image_ocr_provider: "off".to_string(),
            text_translate_provider: "off".to_string(),
            super_mail_merge_enabled: false,
            wps_taskpane_enabled: false,
            multi_sync_mode: "off".to_string(),
        }
    }
}

pub(crate) struct MacosSettingsWindowModel {
    current_page: SettingsPage,
    scroll_y: [i32; 6],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MacosSettingsWindowPresentation {
    pub(crate) chrome: SettingsChromeRenderPlan,
    pub(crate) chrome_paint: SettingsChromePaintPlan,
    pub(crate) navigation: SettingsNavRenderPlan,
    pub(crate) navigation_paint: Vec<SettingsNavItemPaintPlan>,
    pub(crate) content: SettingsContentRenderPlan,
    pub(crate) content_paint: SettingsContentPaintPlan,
}

impl Default for MacosSettingsWindowModel {
    fn default() -> Self {
        Self {
            current_page: SettingsPage::General,
            scroll_y: [0; 6],
        }
    }
}

impl NativeRuntimeDriver for MacosApplicationModel {
    type WindowHandle = MacosMainWindowHandle;

    fn start_runtime(
        &mut self,
        request: NativeRuntimeStartupRequest,
    ) -> NativeRuntimeStartupResult<Self::WindowHandle> {
        if !self.lifecycle.apply(LifecycleEvent::Mount) {
            return NativeRuntimeStartupResult::Failed;
        }
        if let Some(tooltip) = request.status_item_tooltip.as_deref() {
            let mut status_item = MacosStatusItemHost::default();
            status_item.install(tooltip);
        }
        let mut main_window_host = MacosMainWindowHost::default();
        match main_window_host.create_main_windows(request.main_window.clone()) {
            NativeMainWindowPresentation::Created(handles) => {
                self.attach_main_windows(handles, request.main_window.main_visible);
                NativeRuntimeStartupResult::Started(handles)
            }
            NativeMainWindowPresentation::Failed => NativeRuntimeStartupResult::Failed,
        }
    }

    fn dispatch_ui_command(&mut self, command: Command) {
        let result = self
            .product_adapter
            .execute_product_command(command.clone());
        self.queue_command(command);
        self.product_command_results.push(result);
        self.runtime_events.push(ApplicationEvent::ItemsPageReady);
    }

    fn poll_application_event(&mut self) -> Option<ApplicationEvent> {
        self.runtime_events
            .pop()
            .or_else(|| self.poll_clipboard_capture_event())
    }

    fn request_shutdown(&mut self) {
        self.runtime_shutdown_requested = true;
    }
}

impl NativeAiActionPresenter for MacosApplicationModel {
    type WindowHandle = MacosMainWindowHandle;

    fn present_ai_action_menu(
        &mut self,
        request: NativeAiActionMenuRequest<Self::WindowHandle>,
    ) -> Option<ProductAiInvocation> {
        let selected_capability = request.capabilities.first().cloned();
        self.ai_action_presentation.record_menu_request(request);
        selected_capability.map(|capability| ProductAiInvocation {
            capability_id: capability.id.to_string(),
            input_text: String::new(),
            context_item_ids: Vec::new(),
        })
    }

    fn present_ai_settings_surface(
        &mut self,
        request: NativeAiSettingsSurfaceRequest<Self::WindowHandle>,
    ) -> bool {
        let has_capabilities = !request.capabilities.is_empty();
        self.ai_action_presentation.record_settings_request(request);
        has_capabilities
    }

    fn bridge_ai_execution_plan(
        &mut self,
        plan: ProductAiExecutionPlan,
    ) -> ProductAdapterCommandResult {
        let action_name = plan.action_name();
        let result = self.product_adapter.execute_ai_plan(plan);
        self.ai_action_presentation
            .record_execution_plan(action_name);
        self.product_command_results.push(result.clone());
        result
    }
}

impl MacosSettingsWindowModel {
    pub(crate) fn select_page(&mut self, page: SettingsPage) {
        self.current_page = page;
    }

    pub(crate) fn set_scroll_y(&mut self, scroll_y: i32) {
        self.scroll_y[self.current_page.index()] = scroll_y.min(0);
    }

    pub(crate) fn nav_hover_transition(
        &self,
        current_hot: i32,
        next_hot: i32,
    ) -> SettingsNavHoverTransition {
        settings_nav_hover_transition(
            current_hot,
            next_hot,
            crate::app_core::SETTINGS_PAGE_LABELS.len(),
        )
    }

    pub(crate) fn pointer_move_transition(
        &self,
        position: Point,
        current_nav_hot: i32,
        scroll_dragging: bool,
        scroll_layout: SettingsScrollLayout,
        drag_start_y: i32,
        drag_start_scroll: i32,
    ) -> SettingsPointerMoveTransition {
        settings_pointer_move_transition(
            position.x,
            position.y,
            crate::app_core::SETTINGS_PAGE_LABELS.len(),
            current_nav_hot,
            scroll_dragging,
            scroll_layout,
            drag_start_y,
            drag_start_scroll,
        )
    }

    pub(crate) fn pointer_down_target(
        &self,
        position: Point,
        scroll_layout: SettingsScrollLayout,
        scroll_y: i32,
        track_padding: i32,
        thumb_padding: i32,
        min_thumb_rows: i32,
    ) -> SettingsPointerDownTarget {
        settings_pointer_down_target(
            position.x,
            position.y,
            crate::app_core::SETTINGS_PAGE_LABELS.len(),
            scroll_layout,
            scroll_y,
            track_padding,
            thumb_padding,
            min_thumb_rows,
        )
    }

    pub(crate) fn wheel_scroll_delta(&self, delta: i32) -> i32 {
        settings_scroll_delta_for_wheel(delta)
    }

    pub(crate) fn fit_window_plan(
        &self,
        current: UiRect,
        work: UiRect,
        pad: i32,
        min_width: i32,
        min_height: i32,
    ) -> Option<SettingsWindowMovePlan> {
        settings_window_fit_plan(current, work, pad, min_width, min_height)
    }

    pub(crate) fn scale_transition_plan(
        &self,
        current: UiRect,
        work: UiRect,
        pad: i32,
        min_width: i32,
        min_height: i32,
        old_scale_dpi: u32,
        new_scale_dpi: u32,
    ) -> Option<SettingsWindowMovePlan> {
        settings_window_dpi_transition_plan(
            current,
            work,
            pad,
            min_width,
            min_height,
            old_scale_dpi,
            new_scale_dpi,
        )
    }

    pub(crate) fn presentation(
        &self,
        width: i32,
        height: i32,
        snapshot: &MacosSettingsSnapshot,
        update_available: bool,
    ) -> MacosSettingsWindowPresentation {
        let window = UiRect::new(0, 0, width.max(1), height.max(1));
        let chrome = settings_chrome_render_plan(window);
        let navigation =
            settings_nav_render_plan(self.current_page.index(), None, update_available);
        let navigation_paint = navigation
            .items
            .iter()
            .map(settings_nav_item_paint_plan)
            .collect();
        let plugin_sections = settings_plugin_cards_for_state(
            snapshot.quick_search_enabled,
            &snapshot.image_ocr_provider,
            &snapshot.text_translate_provider,
            snapshot.super_mail_merge_enabled,
            snapshot.wps_taskpane_enabled,
        );
        let multi_sync_sections =
            settings_multi_sync_cards_for_mode(snapshot.multi_sync_mode.as_str());
        let content = settings_content_render_plan(
            self.current_page.index(),
            self.scroll_y[self.current_page.index()],
            &plugin_sections,
            &multi_sync_sections,
        );
        let chrome_paint = settings_chrome_paint_plan(
            &chrome,
            crate::app_core::SETTINGS_PAGE_LABELS[self.current_page.index()],
        );
        let content_paint = settings_content_paint_plan(&content);
        MacosSettingsWindowPresentation {
            chrome,
            chrome_paint,
            navigation,
            navigation_paint,
            content,
            content_paint,
        }
    }
}

impl Default for MacosMainWindowModel {
    fn default() -> Self {
        Self {
            layout: MainUiLayout::zsclip(),
        }
    }
}

impl MacosMainWindowModel {
    pub(crate) fn startup_plan(
        &self,
        title: impl Into<String>,
        main_visible: bool,
    ) -> MacosStartupPlan {
        MacosStartupPlan {
            main_window: NativeMainWindowRequest {
                title: title.into(),
                size: crate::app_core::Size {
                    width: self.layout.win_w,
                    height: self.layout.list_y + self.layout.list_h + 7,
                },
                main_visible,
            },
            lifecycle: LifecycleEvent::Mount,
        }
    }

    pub(crate) fn render_plan(&self, input: MainRenderInput) -> MainRenderPlan {
        self.layout.render_plan(input)
    }

    pub(crate) fn pointer_move_transition(
        &self,
        position: Point,
        visible_count: usize,
        scroll_y: i32,
        title_buttons: TitleButtonVisibility,
        scroll_to_top_visible: bool,
        current_hover: MainHoverTarget,
    ) -> MainPointerMoveTransition {
        self.layout.pointer_move_transition(
            position.x,
            position.y,
            visible_count,
            scroll_y,
            title_buttons,
            scroll_to_top_visible,
            current_hover,
            false,
            0,
            scroll_y,
        )
    }

    pub(crate) fn pointer_down_target(
        &self,
        position: Point,
        visible_count: usize,
        scroll_y: i32,
        title_buttons: TitleButtonVisibility,
        search_on: bool,
        scroll_to_top_visible: bool,
    ) -> MainPointerDownTarget {
        self.layout.pointer_down_target(
            position.x,
            position.y,
            visible_count,
            scroll_y,
            title_buttons,
            search_on,
            scroll_to_top_visible,
        )
    }

    pub(crate) fn pointer_up_transition(
        &self,
        position: Point,
        visible_count: usize,
        scroll_y: i32,
        down_title_button: &'static str,
        down_scroll_to_top: bool,
        down_row: i32,
    ) -> MainPointerUpTransition {
        self.layout.pointer_up_transition(
            position.x,
            position.y,
            visible_count,
            scroll_y,
            down_title_button,
            down_scroll_to_top,
            down_row,
        )
    }

    pub(crate) fn shortcut_execution_plan(
        &self,
        action: MainShortcutAction,
        escape_plan: Option<MainShortcutEscapePlan>,
    ) -> MainShortcutExecutionPlan {
        main_shortcut_execution_plan(action, escape_plan)
    }

    pub(crate) fn initial_render_plan(&self, width: i32, height: i32) -> MainRenderPlan {
        let client_rect = UiRect::new(0, 0, width.max(1), height.max(1));
        self.render_plan(MainRenderInput::empty_records(client_rect))
    }
}

impl MacosClipboardHost {
    fn state() -> &'static Mutex<MacosClipboardState> {
        static STATE: OnceLock<Mutex<MacosClipboardState>> = OnceLock::new();
        STATE.get_or_init(|| Mutex::new(MacosClipboardState::default()))
    }

    fn mutate_state<T>(f: impl FnOnce(&mut MacosClipboardState) -> T) -> T {
        let mut state = Self::state()
            .lock()
            .expect("macOS clipboard state lock poisoned");
        f(&mut state)
    }

    #[cfg(test)]
    pub(crate) fn reset_for_tests() {
        Self::mutate_state(|state| *state = MacosClipboardState::default());
    }

    #[cfg(all(target_os = "macos", not(test)))]
    fn system_read_text() -> Option<String> {
        let mut clipboard = Clipboard::new().ok()?;
        clipboard.get_text().ok()
    }

    #[cfg(not(all(target_os = "macos", not(test))))]
    fn system_read_text() -> Option<String> {
        None
    }

    #[cfg(all(target_os = "macos", not(test)))]
    fn system_write_text(text: &str) -> bool {
        let Ok(mut clipboard) = Clipboard::new() else {
            return false;
        };
        clipboard.set_text(text.to_string()).is_ok()
    }

    #[cfg(not(all(target_os = "macos", not(test))))]
    fn system_write_text(_text: &str) -> bool {
        true
    }

    #[cfg(all(target_os = "macos", not(test)))]
    fn system_read_image_rgba() -> Option<(Vec<u8>, usize, usize)> {
        let mut clipboard = Clipboard::new().ok()?;
        let image = clipboard.get_image().ok()?;
        Some((image.bytes.into_owned(), image.width, image.height))
    }

    #[cfg(not(all(target_os = "macos", not(test))))]
    fn system_read_image_rgba() -> Option<(Vec<u8>, usize, usize)> {
        None
    }

    #[cfg(all(target_os = "macos", not(test)))]
    fn system_write_image_rgba(bytes: &[u8], width: usize, height: usize) -> bool {
        let Ok(mut clipboard) = Clipboard::new() else {
            return false;
        };
        clipboard
            .set_image(ImageData {
                width,
                height,
                bytes: Cow::Owned(bytes.to_vec()),
            })
            .is_ok()
    }

    #[cfg(not(all(target_os = "macos", not(test))))]
    fn system_write_image_rgba(_bytes: &[u8], width: usize, height: usize) -> bool {
        width > 0 && height > 0
    }

    #[cfg(all(target_os = "macos", not(test)))]
    fn system_read_file_paths() -> Option<Vec<String>> {
        use objc2::runtime::{AnyClass, AnyObject};
        use objc2::ClassType;
        use objc2_app_kit::NSPasteboard;
        use objc2_foundation::{NSArray, NSURL};

        let pasteboard = NSPasteboard::generalPasteboard();
        let classes = NSArray::<AnyClass>::from_slice(&[NSURL::class()]);
        let objects = unsafe { pasteboard.readObjectsForClasses_options(&classes, None) }?;
        let mut paths = Vec::new();
        for index in 0..objects.len() {
            let object: &AnyObject = unsafe { objects.objectAtIndex_unchecked(index) };
            let Some(url) = object.downcast_ref::<NSURL>() else {
                continue;
            };
            if !url.isFileURL() {
                continue;
            }
            if let Some(path) = url.to_file_path() {
                paths.push(path.to_string_lossy().to_string());
            }
        }

        if paths.is_empty() {
            None
        } else {
            Some(paths)
        }
    }

    #[cfg(not(all(target_os = "macos", not(test))))]
    fn system_read_file_paths() -> Option<Vec<String>> {
        None
    }

    #[cfg(all(target_os = "macos", not(test)))]
    fn system_write_file_paths(paths: &[String]) -> bool {
        use objc2::runtime::ProtocolObject;
        use objc2_app_kit::{NSPasteboard, NSPasteboardWriting};
        use objc2_foundation::{NSArray, NSURL};

        if paths.is_empty() {
            return false;
        }

        let urls = paths
            .iter()
            .filter_map(NSURL::from_file_path)
            .map(ProtocolObject::<dyn NSPasteboardWriting>::from_retained)
            .collect::<Vec<_>>();

        if urls.is_empty() {
            return false;
        }

        let objects = NSArray::from_retained_slice(&urls);
        let pasteboard = NSPasteboard::generalPasteboard();
        pasteboard.clearContents();
        pasteboard.writeObjects(&objects)
    }

    #[cfg(not(all(target_os = "macos", not(test))))]
    fn system_write_file_paths(paths: &[String]) -> bool {
        !paths.is_empty()
    }

    #[cfg(all(target_os = "macos", not(test)))]
    fn system_sequence_number() -> Option<u32> {
        let count = objc2_app_kit::NSPasteboard::generalPasteboard().changeCount();
        u32::try_from(count).ok()
    }

    #[cfg(not(all(target_os = "macos", not(test))))]
    fn system_sequence_number() -> Option<u32> {
        None
    }
}

impl ClipboardHost for MacosClipboardHost {
    fn read_text() -> Option<String> {
        Self::system_read_text().or_else(|| Self::mutate_state(|state| state.text.clone()))
    }

    fn write_text(text: &str) -> bool {
        let written = Self::system_write_text(text);
        if written {
            Self::mutate_state(|state| {
                state.text = Some(text.to_string());
                state.image = None;
                state.file_paths = None;
                state.sequence = state.sequence.saturating_add(1);
            });
        }
        written
    }

    fn read_image_rgba() -> Option<(Vec<u8>, usize, usize)> {
        Self::system_read_image_rgba().or_else(|| Self::mutate_state(|state| state.image.clone()))
    }

    fn write_image_rgba(bytes: &[u8], width: usize, height: usize) -> bool {
        if width == 0
            || height == 0
            || bytes.len() != width.saturating_mul(height).saturating_mul(4)
        {
            return false;
        }
        let written = Self::system_write_image_rgba(bytes, width, height);
        if written {
            Self::mutate_state(|state| {
                state.text = None;
                state.image = Some((bytes.to_vec(), width, height));
                state.file_paths = None;
                state.sequence = state.sequence.saturating_add(1);
            });
        }
        written
    }

    fn read_file_paths() -> Option<Vec<String>> {
        Self::system_read_file_paths()
            .or_else(|| Self::mutate_state(|state| state.file_paths.clone()))
    }

    fn write_file_paths(paths: &[String]) -> bool {
        let system_written = Self::system_write_file_paths(paths);
        if system_written {
            Self::mutate_state(|state| {
                state.text = None;
                state.image = None;
                state.file_paths = Some(paths.to_vec());
                state.sequence = state.sequence.saturating_add(1);
            });
        }
        system_written
    }

    fn sequence_number() -> u32 {
        Self::system_sequence_number().unwrap_or_else(|| Self::mutate_state(|state| state.sequence))
    }

    fn write_text_ignored_by_monitors(text: &str) -> bool {
        if !Self::write_text(text) {
            return false;
        }
        Self::mutate_state(|state| {
            state.ignore_next_capture = true;
        });
        true
    }

    fn should_ignore_capture_by_named_format() -> bool {
        Self::mutate_state(|state| {
            let ignore = state.ignore_next_capture;
            state.ignore_next_capture = false;
            ignore
        })
    }
}

impl StatusItemHost for MacosStatusItemHost {
    fn install(&mut self, tooltip: &str) -> bool {
        self.installed = true;
        self.tooltip = tooltip.to_string();
        true
    }

    fn remove(&mut self) {
        self.installed = false;
        self.menu_entries.clear();
    }

    fn present_menu(&mut self, entries: &[StatusMenuEntry]) {
        self.menu_entries = entries.to_vec();
    }
}

impl NativePopupMenuHost for MacosPopupMenuHost {
    type Owner = ();

    fn present_popup_menu(
        &mut self,
        owner: Self::Owner,
        x: i32,
        y: i32,
        placement: NativePopupMenuPlacement,
        entries: &[NativePopupMenuEntry],
    ) -> usize {
        self.last_owner = owner;
        self.last_position = (x, y);
        self.last_placement = Some(placement);
        self.last_entries = entries.to_vec();
        self.next_command
    }
}

impl NativeTransientWindowHost for MacosTransientWindowHost {
    type Handle = MacosTransientWindowHandle;
    type Owner = MacosMainWindowHandle;

    fn create_transient_window(
        &mut self,
        request: NativeTransientWindowRequest<Self::Owner>,
    ) -> NativeTransientWindowPresentation<Self::Handle> {
        self.requests
            .borrow_mut()
            .push(MacosTransientWindowCreateRequest {
                owner: request.owner,
                bounds: request.bounds,
            });
        NativeTransientWindowPresentation::Created(MacosTransientWindowHandle(1))
    }

    fn present_transient_window(&mut self, handle: Self::Handle, bounds: UiRect) {
        self.actions
            .borrow_mut()
            .push(MacosTransientWindowAction::Present { handle, bounds });
    }

    fn hide_transient_window(&mut self, handle: Self::Handle) {
        self.actions
            .borrow_mut()
            .push(MacosTransientWindowAction::Hide(handle));
    }

    fn destroy_transient_window(&mut self, handle: Self::Handle) {
        self.actions
            .borrow_mut()
            .push(MacosTransientWindowAction::Destroy(handle));
    }
}

impl MacosTransientWindowHost {
    pub(crate) fn requests(&self) -> Vec<MacosTransientWindowCreateRequest> {
        self.requests.borrow().clone()
    }

    pub(crate) fn actions(&self) -> Vec<MacosTransientWindowAction> {
        self.actions.borrow().clone()
    }
}

impl NativeImeHost for MacosImeHost {
    type Handle = MacosImeHandle;

    fn candidate_anchor(
        &mut self,
        focus: Self::Handle,
        index: u32,
    ) -> Option<NativeImeCandidateAnchor> {
        self.actions
            .borrow_mut()
            .push(MacosImeAction::QueryCandidate { focus, index });
        *self.next_candidate.borrow()
    }

    fn composition_anchor(&mut self, focus: Self::Handle) -> Option<NativeImeCompositionAnchor> {
        self.actions
            .borrow_mut()
            .push(MacosImeAction::QueryComposition(focus));
        *self.next_composition.borrow()
    }

    fn has_default_ime_window(&mut self, focus: Self::Handle) -> bool {
        self.actions
            .borrow_mut()
            .push(MacosImeAction::HasDefaultImeWindow(focus));
        self.next_has_default_ime_window
    }
}

impl MacosImeHost {
    pub(crate) fn set_next_candidate(&self, anchor: Option<NativeImeCandidateAnchor>) {
        *self.next_candidate.borrow_mut() = anchor;
    }

    pub(crate) fn set_next_composition(&self, anchor: Option<NativeImeCompositionAnchor>) {
        *self.next_composition.borrow_mut() = anchor;
    }

    pub(crate) fn set_next_has_default_ime_window(&mut self, value: bool) {
        self.next_has_default_ime_window = value;
    }

    pub(crate) fn actions(&self) -> Vec<MacosImeAction> {
        self.actions.borrow().clone()
    }
}

impl NativeTextCaretHost for MacosTextCaretHost {
    type Handle = MacosTextCaretHandle;

    fn accessible_caret_anchor(&mut self, focus: Self::Handle) -> Option<NativeTextCaretAnchor> {
        self.actions
            .borrow_mut()
            .push(MacosTextCaretAction::QueryAccessible(focus));
        *self.next_accessible.borrow()
    }

    fn thread_caret_anchor(&mut self, target: Self::Handle) -> Option<NativeTextCaretAnchor> {
        self.actions
            .borrow_mut()
            .push(MacosTextCaretAction::QueryThread(target));
        *self.next_thread.borrow()
    }

    fn focus_rect_anchor(
        &mut self,
        focus: Self::Handle,
        max_width: i32,
        max_height: i32,
    ) -> Option<NativeTextCaretAnchor> {
        self.actions
            .borrow_mut()
            .push(MacosTextCaretAction::QueryFocusRect {
                focus,
                max_width,
                max_height,
            });
        *self.next_focus_rect.borrow()
    }

    fn cursor_anchor(&mut self) -> Option<NativeTextCaretAnchor> {
        self.actions
            .borrow_mut()
            .push(MacosTextCaretAction::QueryCursor);
        *self.next_cursor.borrow()
    }

    fn focus_handle_for_target(&mut self, target: Self::Handle) -> Self::Handle {
        self.actions
            .borrow_mut()
            .push(MacosTextCaretAction::ResolveFocus(target));
        self.next_focus_handle.borrow().unwrap_or(target)
    }
}

impl MacosTextCaretHost {
    pub(crate) fn set_next_accessible(&self, anchor: Option<NativeTextCaretAnchor>) {
        *self.next_accessible.borrow_mut() = anchor;
    }

    pub(crate) fn set_next_thread(&self, anchor: Option<NativeTextCaretAnchor>) {
        *self.next_thread.borrow_mut() = anchor;
    }

    pub(crate) fn set_next_focus_rect(&self, anchor: Option<NativeTextCaretAnchor>) {
        *self.next_focus_rect.borrow_mut() = anchor;
    }

    pub(crate) fn set_next_cursor(&self, anchor: Option<NativeTextCaretAnchor>) {
        *self.next_cursor.borrow_mut() = anchor;
    }

    pub(crate) fn set_next_focus_handle(&self, handle: Option<MacosTextCaretHandle>) {
        *self.next_focus_handle.borrow_mut() = handle;
    }

    pub(crate) fn actions(&self) -> Vec<MacosTextCaretAction> {
        self.actions.borrow().clone()
    }
}

impl NativeDialogHost for MacosDialogHost {
    type Owner = ();

    fn show_message(
        &self,
        _owner: Self::Owner,
        title: &str,
        message: &str,
        level: NativeDialogLevel,
    ) {
        self.record_message(title, message, level);
    }

    fn confirm(
        &self,
        _owner: Self::Owner,
        _title: &str,
        _message: &str,
        _level: NativeDialogLevel,
        _buttons: NativeDialogButtons,
    ) -> NativeDialogResponse {
        NativeDialogResponse::Cancel
    }
}

impl MacosDialogHost {
    pub(crate) fn record_message(&self, title: &str, message: &str, level: NativeDialogLevel) {
        *self.last_message.borrow_mut() = Some(MacosDialogMessage {
            title: title.to_string(),
            message: message.to_string(),
            level,
        });
    }

    pub(crate) fn last_message(&self) -> Option<MacosDialogMessage> {
        self.last_message.borrow().clone()
    }
}

impl NativeShellOpenHost for MacosShellOpenHost {
    fn open_path(&self, path: &str) {
        self.opened_paths.borrow_mut().push(path.to_string());
        let _ = Self::system_open_path(path);
    }
}

impl MacosShellOpenHost {
    #[cfg(all(target_os = "macos", not(test)))]
    fn system_open_path(path: &str) -> bool {
        if matches!(
            std::env::var("ZSCLIP_NATIVE_HOST_SHELL_OPEN_DRY_RUN").as_deref(),
            Ok("1")
        ) {
            return true;
        }
        open_macos_url_or_file(path.to_string()).is_ok()
    }

    #[cfg(not(all(target_os = "macos", not(test))))]
    fn system_open_path(_path: &str) -> bool {
        true
    }

    pub(crate) fn opened_paths(&self) -> Vec<String> {
        self.opened_paths.borrow().clone()
    }
}

impl NativeWindowIdentityHost for MacosWindowIdentityHost {
    type Handle = MacosWindowIdentityHandle;

    fn process_name(&self, handle: Self::Handle) -> String {
        self.actions
            .borrow_mut()
            .push(MacosWindowIdentityAction::ProcessName(handle));
        let injected = self.process_name.borrow().clone();
        if injected.is_empty() {
            macos_process_name_for_pid(handle).unwrap_or_default()
        } else {
            injected
        }
    }

    fn class_name(&self, handle: Self::Handle) -> String {
        self.actions
            .borrow_mut()
            .push(MacosWindowIdentityAction::ClassName(handle));
        let injected = self.class_name.borrow().clone();
        if injected.is_empty() {
            macos_bundle_id_for_pid(handle).unwrap_or_default()
        } else {
            injected
        }
    }

    fn root_handle(&self, handle: Self::Handle) -> Self::Handle {
        self.actions
            .borrow_mut()
            .push(MacosWindowIdentityAction::RootHandle(handle));
        self.root_handle.borrow().unwrap_or(handle)
    }

    fn foreground_handle(&self) -> Self::Handle {
        self.actions
            .borrow_mut()
            .push(MacosWindowIdentityAction::ForegroundHandle);
        self.foreground_handle
            .borrow()
            .or_else(macos_frontmost_process_handle)
            .unwrap_or(MacosWindowIdentityHandle(0))
    }

    fn exists(&self, handle: Self::Handle) -> bool {
        self.actions
            .borrow_mut()
            .push(MacosWindowIdentityAction::Exists(handle));
        self.existing_windows.borrow().contains(&handle) || macos_process_exists(handle)
    }

    fn is_foreground(&self, handle: Self::Handle) -> bool {
        self.actions
            .borrow_mut()
            .push(MacosWindowIdentityAction::IsForeground(handle));
        self.foreground_handle
            .borrow()
            .or_else(macos_frontmost_process_handle)
            == Some(handle)
    }

    fn is_current_process_window(&self, handle: Self::Handle) -> bool {
        self.actions
            .borrow_mut()
            .push(MacosWindowIdentityAction::IsCurrentProcessWindow(handle));
        self.current_process_windows.borrow().contains(&handle)
            || handle.0 == u64::from(std::process::id())
    }
}

impl MacosWindowIdentityHost {
    pub(crate) fn set_process_name(&self, value: &str) {
        *self.process_name.borrow_mut() = value.to_string();
    }

    pub(crate) fn set_class_name(&self, value: &str) {
        *self.class_name.borrow_mut() = value.to_string();
    }

    pub(crate) fn set_root_handle(&self, handle: Option<MacosWindowIdentityHandle>) {
        *self.root_handle.borrow_mut() = handle;
    }

    pub(crate) fn set_foreground_handle(&self, handle: Option<MacosWindowIdentityHandle>) {
        *self.foreground_handle.borrow_mut() = handle;
    }

    pub(crate) fn set_existing_windows(&self, handles: Vec<MacosWindowIdentityHandle>) {
        *self.existing_windows.borrow_mut() = handles;
    }

    pub(crate) fn set_current_process_windows(&self, handles: Vec<MacosWindowIdentityHandle>) {
        *self.current_process_windows.borrow_mut() = handles;
    }

    pub(crate) fn actions(&self) -> Vec<MacosWindowIdentityAction> {
        self.actions.borrow().clone()
    }
}

impl NativePasteTargetHost for MacosPasteTargetHost {
    type Handle = MacosPasteTargetHandle;

    fn force_paste_target_foreground(&mut self, target: Self::Handle) -> bool {
        self.actions
            .borrow_mut()
            .push(MacosPasteTargetAction::ForceForeground(target));
        self.next_foreground_result || macos_activate_process(MacosWindowIdentityHandle(target.0))
    }

    fn restore_paste_target_focus(&mut self, target: Self::Handle, focus: Self::Handle) {
        self.actions
            .borrow_mut()
            .push(MacosPasteTargetAction::RestoreFocus { target, focus });
    }

    fn set_paste_target_text(&mut self, target: Self::Handle, text: &str) -> bool {
        self.actions
            .borrow_mut()
            .push(MacosPasteTargetAction::SetText {
                target,
                text: text.to_string(),
            });
        true
    }

    fn paste_target_text_input_capabilities(
        &mut self,
        target: Self::Handle,
    ) -> PasteTargetTextInputCapabilities {
        self.actions
            .borrow_mut()
            .push(MacosPasteTargetAction::QueryTextInputCapabilities(target));
        self.next_text_input_capabilities
    }

    fn paste_target_focus_status(
        &mut self,
        target: Self::Handle,
        passthrough_focus: Self::Handle,
    ) -> PasteTargetFocusStatus {
        self.actions
            .borrow_mut()
            .push(MacosPasteTargetAction::QueryFocusStatus {
                target,
                passthrough_focus,
            });
        if self.next_focus_status != PasteTargetFocusStatus::Unknown {
            return self.next_focus_status;
        }
        macos_frontmost_process_handle()
            .map(|foreground| {
                if foreground.0 == target.0 {
                    PasteTargetFocusStatus::InsideTarget
                } else if foreground.0 == passthrough_focus.0 {
                    PasteTargetFocusStatus::NoActiveFocus
                } else {
                    PasteTargetFocusStatus::OutsideTarget
                }
            })
            .unwrap_or(PasteTargetFocusStatus::Unknown)
    }

    fn paste_target_text_input_ready(&mut self, target: Self::Handle) -> bool {
        self.actions
            .borrow_mut()
            .push(MacosPasteTargetAction::QueryTextInputReady(target));
        self.next_text_input_ready
    }

    fn send_paste_shortcut(&mut self, target: Self::Handle) -> bool {
        self.actions
            .borrow_mut()
            .push(MacosPasteTargetAction::SendPasteShortcut(target));
        macos_post_command_v_shortcut().unwrap_or(true)
    }
}

fn macos_frontmost_process_handle() -> Option<MacosWindowIdentityHandle> {
    macos_osascript_line(
        r#"tell application "System Events" to get unix id of first application process whose frontmost is true"#,
    )
    .and_then(|line| line.parse::<u64>().ok())
    .map(MacosWindowIdentityHandle)
}

fn macos_process_name_for_pid(handle: MacosWindowIdentityHandle) -> Option<String> {
    if handle.0 == 0 {
        return None;
    }
    macos_osascript_line(&format!(
        r#"tell application "System Events" to get name of first application process whose unix id is {}"#,
        handle.0
    ))
}

fn macos_bundle_id_for_pid(handle: MacosWindowIdentityHandle) -> Option<String> {
    if handle.0 == 0 {
        return None;
    }
    macos_osascript_line(&format!(
        r#"tell application "System Events" to get bundle identifier of first application process whose unix id is {}"#,
        handle.0
    ))
}

fn macos_process_exists(handle: MacosWindowIdentityHandle) -> bool {
    if handle.0 == 0 {
        return false;
    }
    macos_osascript_line(&format!(
        r#"tell application "System Events" to exists first application process whose unix id is {}"#,
        handle.0
    ))
    .is_some_and(|line| line == "true")
}

fn macos_activate_process(handle: MacosWindowIdentityHandle) -> bool {
    if handle.0 == 0 {
        return false;
    }
    macos_osascript_line(&format!(
        r#"tell application "System Events" to set frontmost of first application process whose unix id is {} to true"#,
        handle.0
    ))
    .is_some()
}

fn macos_post_command_v_shortcut() -> Option<bool> {
    macos_osascript_line(r#"tell application "System Events" to keystroke "v" using command down"#)
        .map(|_| true)
}

#[cfg(all(target_os = "macos", not(test)))]
fn macos_osascript_line(script: &str) -> Option<String> {
    let output = ProcessCommand::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

#[cfg(not(all(target_os = "macos", not(test))))]
fn macos_osascript_line(_script: &str) -> Option<String> {
    None
}

impl MacosPasteTargetHost {
    pub(crate) fn set_next_foreground_result(&mut self, result: bool) {
        self.next_foreground_result = result;
    }

    pub(crate) fn set_next_text_input_capabilities(
        &mut self,
        capabilities: PasteTargetTextInputCapabilities,
    ) {
        self.next_text_input_capabilities = capabilities;
    }

    pub(crate) fn set_next_text_input_ready(&mut self, ready: bool) {
        self.next_text_input_ready = ready;
    }

    pub(crate) fn set_next_focus_status(&mut self, status: PasteTargetFocusStatus) {
        self.next_focus_status = status;
    }

    pub(crate) fn actions(&self) -> Vec<MacosPasteTargetAction> {
        self.actions.borrow().clone()
    }
}

impl NativeMainWindowHost for MacosMainWindowHost {
    type Handle = MacosMainWindowHandle;
    type AppIcon = &'static str;

    fn create_main_windows(
        &mut self,
        request: NativeMainWindowRequest,
    ) -> NativeMainWindowPresentation<Self::Handle> {
        self.requests.borrow_mut().push(MacosMainWindowRequest {
            title: request.title,
            size: request.size,
            main_visible: request.main_visible,
        });
        let handles = NativeMainWindowHandles {
            main: MacosMainWindowHandle(1),
            quick: MacosMainWindowHandle(2),
        };
        self.created = Some(handles);
        NativeMainWindowPresentation::Created(handles)
    }

    fn apply_main_window_appearance(&mut self, handle: Self::Handle) {
        self.appearances.borrow_mut().push(handle);
    }

    fn set_main_window_app_icon(
        &mut self,
        handle: Self::Handle,
        icon: NativeAppIconResource<Self::AppIcon>,
    ) {
        self.lifecycle_actions
            .borrow_mut()
            .push(MacosMainWindowLifecycleAction::SetAppIcon { handle, icon });
    }

    fn hide_main_window(&mut self, handle: Self::Handle) {
        self.lifecycle_actions
            .borrow_mut()
            .push(MacosMainWindowLifecycleAction::Hide(handle));
    }

    fn present_main_window(&mut self, handle: Self::Handle, mode: NativeMainWindowPresentMode) {
        self.lifecycle_actions
            .borrow_mut()
            .push(MacosMainWindowLifecycleAction::Present { handle, mode });
    }

    fn set_main_window_bounds(&mut self, handle: Self::Handle, bounds: UiRect) {
        self.bounds.borrow_mut().push((handle, bounds));
        self.lifecycle_actions
            .borrow_mut()
            .push(MacosMainWindowLifecycleAction::Bounds { handle, bounds });
    }

    fn activate_main_window(&mut self, handle: Self::Handle) {
        self.lifecycle_actions
            .borrow_mut()
            .push(MacosMainWindowLifecycleAction::Activate(handle));
    }

    fn foreground_main_window(&mut self, handle: Self::Handle) {
        self.lifecycle_actions
            .borrow_mut()
            .push(MacosMainWindowLifecycleAction::Foreground(handle));
    }

    fn restore_main_window(&mut self, handle: Self::Handle) {
        self.lifecycle_actions
            .borrow_mut()
            .push(MacosMainWindowLifecycleAction::Restore(handle));
    }

    fn close_main_window(&mut self, handle: Self::Handle) {
        self.lifecycle_actions
            .borrow_mut()
            .push(MacosMainWindowLifecycleAction::Close(handle));
    }

    fn set_main_window_activation_policy(&mut self, handle: Self::Handle, allow_activation: bool) {
        self.lifecycle_actions.borrow_mut().push(
            MacosMainWindowLifecycleAction::ActivationPolicy {
                handle,
                allow_activation,
            },
        );
    }

    fn request_main_window_close(&mut self, handle: Self::Handle) {
        self.lifecycle_actions
            .borrow_mut()
            .push(MacosMainWindowLifecycleAction::RequestClose(handle));
    }

    fn destroy_main_window(&mut self, handle: Self::Handle) {
        self.lifecycle_actions
            .borrow_mut()
            .push(MacosMainWindowLifecycleAction::Destroy(handle));
    }

    fn capture_main_pointer(&mut self, handle: Self::Handle) {
        self.lifecycle_actions
            .borrow_mut()
            .push(MacosMainWindowLifecycleAction::CapturePointer(handle));
    }

    fn release_main_pointer(&mut self, handle: Self::Handle) {
        self.lifecycle_actions
            .borrow_mut()
            .push(MacosMainWindowLifecycleAction::ReleasePointer(handle));
    }

    fn begin_main_window_drag(&mut self, handle: Self::Handle) {
        self.lifecycle_actions
            .borrow_mut()
            .push(MacosMainWindowLifecycleAction::BeginDrag(handle));
    }

    fn track_main_pointer_leave(&mut self, handle: Self::Handle) -> bool {
        self.pointer_leave_requests.borrow_mut().push(handle);
        true
    }

    fn request_main_window_area_repaint(
        &mut self,
        handle: Self::Handle,
        area: Option<UiRect>,
        erase: bool,
    ) -> bool {
        self.repaint_requests
            .borrow_mut()
            .push((handle, area, erase));
        true
    }

    fn main_window_layout_dpi(&mut self, handle: Self::Handle) -> u32 {
        self.layout_dpi_queries.borrow_mut().push(handle);
        96
    }

    fn main_window_client_bounds(&mut self, handle: Self::Handle) -> Option<UiRect> {
        self.client_bounds_queries.borrow_mut().push(handle);
        self.bounds
            .borrow()
            .iter()
            .rev()
            .find(|(candidate, _)| *candidate == handle)
            .map(|(_, bounds)| UiRect::new(0, 0, bounds.width(), bounds.height()))
    }

    fn main_window_bounds(&mut self, handle: Self::Handle) -> Option<UiRect> {
        self.window_bounds_queries.borrow_mut().push(handle);
        self.bounds
            .borrow()
            .iter()
            .rev()
            .find(|(candidate, _)| *candidate == handle)
            .map(|(_, bounds)| *bounds)
    }
}

impl MacosMainWindowHost {
    pub(crate) fn requests(&self) -> Vec<MacosMainWindowRequest> {
        self.requests.borrow().clone()
    }

    pub(crate) fn appearances(&self) -> Vec<MacosMainWindowHandle> {
        self.appearances.borrow().clone()
    }

    pub(crate) fn lifecycle_actions(&self) -> Vec<MacosMainWindowLifecycleAction> {
        self.lifecycle_actions.borrow().clone()
    }

    pub(crate) fn repaint_requests(&self) -> Vec<(MacosMainWindowHandle, Option<UiRect>, bool)> {
        self.repaint_requests.borrow().clone()
    }

    pub(crate) fn pointer_leave_requests(&self) -> Vec<MacosMainWindowHandle> {
        self.pointer_leave_requests.borrow().clone()
    }

    pub(crate) fn layout_dpi_queries(&self) -> Vec<MacosMainWindowHandle> {
        self.layout_dpi_queries.borrow().clone()
    }

    pub(crate) fn client_bounds_queries(&self) -> Vec<MacosMainWindowHandle> {
        self.client_bounds_queries.borrow().clone()
    }

    pub(crate) fn window_bounds_queries(&self) -> Vec<MacosMainWindowHandle> {
        self.window_bounds_queries.borrow().clone()
    }
}

impl NativeMainSearchControlHost for MacosMainSearchControlHost {
    type Owner = MacosMainWindowHandle;
    type Handle = MacosMainSearchControlHandle;
    type StyleResource = MacosMainSearchStyleResource;

    fn create_search_control(
        &mut self,
        request: NativeMainSearchControlRequest<Self::Owner>,
    ) -> NativeMainSearchControlPresentation<Self::Handle> {
        self.requests
            .borrow_mut()
            .push(MacosMainSearchControlRequest {
                owner: request.owner,
                id: request.id,
                bounds: request.bounds,
                visible: request.visible,
            });
        NativeMainSearchControlPresentation::Created(MacosMainSearchControlHandle(1))
    }

    fn apply_search_style(
        &mut self,
        request: NativeMainSearchStyleRequest<Self::Handle, Self::StyleResource>,
    ) -> NativeMainSearchStylePresentation<Self::StyleResource> {
        self.style_requests.borrow_mut().push(request);
        NativeMainSearchStylePresentation::Applied(Some(MacosMainSearchStyleResource(1)))
    }

    fn release_search_style_resource(&mut self, resource: Self::StyleResource) {
        self.released_style_resources.borrow_mut().push(resource);
    }

    fn set_search_bounds(&mut self, handle: Self::Handle, bounds: UiRect) {
        *self.last_bounds.borrow_mut() = Some((handle, bounds));
    }

    fn set_search_visible(&mut self, handle: Self::Handle, visible: bool) {
        self.visible.borrow_mut().push((handle, visible));
    }

    fn search_text(&self, _handle: Self::Handle) -> String {
        self.text.borrow().clone()
    }

    fn set_search_text(&mut self, _handle: Self::Handle, text: &str) {
        *self.text.borrow_mut() = text.to_string();
    }

    fn focus_search(&mut self, handle: Self::Handle) {
        *self.focused.borrow_mut() = Some(handle);
    }
}

impl MacosMainSearchControlHost {
    pub(crate) fn requests(&self) -> Vec<MacosMainSearchControlRequest> {
        self.requests.borrow().clone()
    }

    pub(crate) fn style_requests(
        &self,
    ) -> Vec<NativeMainSearchStyleRequest<MacosMainSearchControlHandle, MacosMainSearchStyleResource>>
    {
        self.style_requests.borrow().clone()
    }

    pub(crate) fn released_style_resources(&self) -> Vec<MacosMainSearchStyleResource> {
        self.released_style_resources.borrow().clone()
    }

    pub(crate) fn last_bounds(&self) -> Option<(MacosMainSearchControlHandle, UiRect)> {
        *self.last_bounds.borrow()
    }

    pub(crate) fn visibility_changes(&self) -> Vec<(MacosMainSearchControlHandle, bool)> {
        self.visible.borrow().clone()
    }

    pub(crate) fn focused(&self) -> Option<MacosMainSearchControlHandle> {
        *self.focused.borrow()
    }
}

impl MacosTextLayout {
    pub(crate) fn actions(&self) -> Vec<MacosTextLayoutAction> {
        self.actions.borrow().clone()
    }
}

impl TextLayout for MacosTextLayout {
    fn measure(&self, text: &str, style: &TextStyle, max_width: i32) -> crate::app_core::Size {
        self.actions
            .borrow_mut()
            .push(MacosTextLayoutAction::Measure {
                text: text.to_string(),
                style: style.clone(),
                max_width,
            });
        let width = ((text.chars().count() as f32 * style.size * 0.6).ceil() as i32)
            .max(1)
            .min(max_width.max(1));
        let height = (style.size * 1.4).ceil() as i32;
        crate::app_core::Size { width, height }
    }

    fn layout_runs(&self, text: &str, style: &TextStyle, bounds: Rect) -> Vec<TextRun> {
        self.actions
            .borrow_mut()
            .push(MacosTextLayoutAction::LayoutRuns {
                text: text.to_string(),
                style: style.clone(),
                bounds,
            });
        if text.is_empty() {
            Vec::new()
        } else {
            vec![TextRun {
                text: text.to_string(),
                bounds,
            }]
        }
    }
}

impl MacosRenderer {
    pub(crate) fn commands(&self) -> &[MacosRenderCommand] {
        &self.commands
    }
}

impl Renderer for MacosRenderer {
    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.commands
            .push(MacosRenderCommand::FillRect(rect, color));
    }

    fn stroke_rect(&mut self, rect: Rect, color: Color, width: i32) {
        self.commands
            .push(MacosRenderCommand::StrokeRect(rect, color, width));
    }

    fn draw_text(&mut self, run: &TextRun, style: &TextStyle) {
        self.commands
            .push(MacosRenderCommand::DrawText(run.clone(), style.clone()));
    }

    fn push_clip(&mut self, rect: Rect) {
        self.commands.push(MacosRenderCommand::PushClip(rect));
    }

    fn pop_clip(&mut self) {
        self.commands.push(MacosRenderCommand::PopClip);
    }
}

impl NativeFileDialogHost for MacosFileDialogHost {
    fn pick_file(&self, request: NativeFileDialogRequest<'_>) -> Result<Option<String>, String> {
        self.requests.borrow_mut().push(MacosFileDialogRequest {
            title: request.title.to_string(),
            filter_name: request.filter_name.to_string(),
            filter_pattern: request.filter_pattern.to_string(),
            current_path: request.current_path.to_string(),
        });
        #[cfg(all(target_os = "macos", not(test)))]
        {
            return Self::system_pick_file(request);
        }
        #[cfg(not(all(target_os = "macos", not(test))))]
        self.next_result.borrow().clone()
    }
}

impl MacosFileDialogHost {
    #[cfg(all(target_os = "macos", not(test)))]
    fn system_pick_file(request: NativeFileDialogRequest<'_>) -> Result<Option<String>, String> {
        if let Ok(path) = std::env::var("ZSCLIP_NATIVE_HOST_FILE_PICKER_SMOKE_PATH") {
            return Ok(Some(path));
        }
        pick_macos_native_file(request)
    }

    #[cfg(not(all(target_os = "macos", not(test))))]
    fn system_pick_file(_request: NativeFileDialogRequest<'_>) -> Result<Option<String>, String> {
        Err("macOS native file picker is only available on macOS".to_string())
    }

    pub(crate) fn set_next_result(&self, result: Result<Option<String>, String>) {
        *self.next_result.borrow_mut() = result;
    }

    pub(crate) fn requests(&self) -> Vec<MacosFileDialogRequest> {
        self.requests.borrow().clone()
    }
}

impl NativeTextInputDialogHost for MacosTextInputDialogHost {
    type Owner = ();

    fn prompt_text(
        &self,
        _owner: Self::Owner,
        request: NativeTextInputDialogRequest<'_>,
    ) -> Option<String> {
        self.requests
            .borrow_mut()
            .push(MacosTextInputDialogRequest {
                title: request.title.to_string(),
                label: request.label.to_string(),
                initial: request.initial.to_string(),
            });
        #[cfg(all(target_os = "macos", not(test)))]
        {
            return Self::system_prompt_text(request);
        }
        #[cfg(not(all(target_os = "macos", not(test))))]
        self.next_result.borrow().clone()
    }
}

impl MacosTextInputDialogHost {
    #[cfg(all(target_os = "macos", not(test)))]
    fn system_prompt_text(request: NativeTextInputDialogRequest<'_>) -> Option<String> {
        prompt_macos_native_text(request)
    }

    #[cfg(not(all(target_os = "macos", not(test))))]
    fn system_prompt_text(_request: NativeTextInputDialogRequest<'_>) -> Option<String> {
        None
    }

    pub(crate) fn set_next_result(&self, result: Option<String>) {
        *self.next_result.borrow_mut() = result;
    }

    pub(crate) fn requests(&self) -> Vec<MacosTextInputDialogRequest> {
        self.requests.borrow().clone()
    }
}

impl NativeEditTextDialogHost for MacosEditTextDialogHost {
    type Owner = ();

    fn open_edit_text(
        &self,
        _owner: Self::Owner,
        request: NativeEditTextDialogRequest<'_>,
        save_handler: &mut dyn NativeEditTextSaveHandler,
    ) -> NativeEditTextDialogResult {
        self.requests.borrow_mut().push(MacosEditTextDialogRequest {
            title: request.title.to_string(),
            initial_text: request.initial_text.to_string(),
            initial_size: request.initial_size,
        });
        #[cfg(all(target_os = "macos", not(test)))]
        {
            return Self::system_open_edit_text(request, save_handler);
        }
        #[cfg(not(all(target_os = "macos", not(test))))]
        {
            let saved = self
                .next_saved_text
                .borrow()
                .as_deref()
                .map(|text| save_handler.save_text(text).is_ok())
                .unwrap_or(false);
            NativeEditTextDialogResult {
                saved,
                final_size: *self.next_final_size.borrow(),
            }
        }
    }
}

impl MacosEditTextDialogHost {
    #[cfg(all(target_os = "macos", not(test)))]
    fn system_open_edit_text(
        request: NativeEditTextDialogRequest<'_>,
        save_handler: &mut dyn NativeEditTextSaveHandler,
    ) -> NativeEditTextDialogResult {
        edit_macos_native_text(request, save_handler)
    }

    #[cfg(not(all(target_os = "macos", not(test))))]
    fn system_open_edit_text(
        _request: NativeEditTextDialogRequest<'_>,
        _save_handler: &mut dyn NativeEditTextSaveHandler,
    ) -> NativeEditTextDialogResult {
        NativeEditTextDialogResult::default()
    }

    pub(crate) fn set_next_result(
        &self,
        saved_text: Option<String>,
        final_size: Option<crate::app_core::Size>,
    ) {
        *self.next_saved_text.borrow_mut() = saved_text;
        *self.next_final_size.borrow_mut() = final_size;
    }

    pub(crate) fn requests(&self) -> Vec<MacosEditTextDialogRequest> {
        self.requests.borrow().clone()
    }
}

impl NativeMailMergeWindowHost for MacosMailMergeWindowHost {
    type Owner = ();

    fn open_mail_merge(&self, _owner: Self::Owner, request: NativeMailMergeWindowRequest<'_>) {
        self.requests
            .borrow_mut()
            .push(MacosMailMergeWindowRequest {
                initial_excel_path: request.initial_excel_path.map(str::to_string),
            });
    }
}

impl MacosMailMergeWindowHost {
    pub(crate) fn requests(&self) -> Vec<MacosMailMergeWindowRequest> {
        self.requests.borrow().clone()
    }
}

impl SettingsActionExecutor for MacosSettingsActionExecutor {
    type Context = MacosSettingsActionContext;

    fn execute_sync(&mut self, _context: &mut Self::Context, action: SettingsAction) -> bool {
        self.actions
            .borrow_mut()
            .push((SettingsActionRoute::Sync, action));
        true
    }

    fn execute_group(&mut self, _context: &mut Self::Context, action: SettingsAction) -> bool {
        self.actions
            .borrow_mut()
            .push((SettingsActionRoute::Group, action));
        true
    }

    fn execute_platform(&mut self, _context: &mut Self::Context, action: SettingsAction) -> bool {
        self.actions
            .borrow_mut()
            .push((SettingsActionRoute::Platform, action));
        true
    }
}

impl MacosSettingsActionExecutor {
    pub(crate) fn actions(&self) -> Vec<(SettingsActionRoute, SettingsAction)> {
        self.actions.borrow().clone()
    }

    pub(crate) fn sync_action_count(&self) -> usize {
        self.action_count_for_route(SettingsActionRoute::Sync)
    }

    pub(crate) fn group_action_count(&self) -> usize {
        self.action_count_for_route(SettingsActionRoute::Group)
    }

    pub(crate) fn platform_action_count(&self) -> usize {
        self.action_count_for_route(SettingsActionRoute::Platform)
    }

    fn action_count_for_route(&self, route: SettingsActionRoute) -> usize {
        self.actions
            .borrow()
            .iter()
            .filter(|(action_route, _)| *action_route == route)
            .count()
    }
}

impl NativeSettingsControlHost for MacosSettingsControlHost {
    type Handle = MacosSettingsControlHandle;

    fn create_control(&mut self, spec: &SettingsControlSpec) -> Self::Handle {
        self.requests
            .borrow_mut()
            .push(MacosSettingsControlRequest { spec: spec.clone() });
        MacosSettingsControlHandle(self.requests.borrow().len() as u64)
    }

    fn destroy_control(&mut self, handle: Self::Handle) {
        self.destroyed.borrow_mut().push(handle);
    }

    fn control_exists(&self, handle: Self::Handle) -> bool {
        let index = match handle.0.checked_sub(1) {
            Some(index) => index as usize,
            None => return false,
        };
        index < self.requests.borrow().len() && !self.destroyed.borrow().contains(&handle)
    }

    fn set_control_visible(&mut self, handle: Self::Handle, visible: bool) {
        self.visible.borrow_mut().push((handle, visible));
    }

    fn set_control_enabled(&mut self, handle: Self::Handle, enabled: bool) {
        self.enabled.borrow_mut().push((handle, enabled));
    }

    fn set_control_bounds(&mut self, handle: Self::Handle, bounds: UiRect) {
        self.bounds.borrow_mut().push((handle, bounds));
    }

    fn control_at_point(&self, point: Point) -> Option<Self::Handle> {
        self.hit_test_queries.borrow_mut().push(point);
        let requests = self.requests.borrow();
        let destroyed = self.destroyed.borrow();
        let visible = self.visible.borrow();
        let enabled = self.enabled.borrow();
        let bounds_changes = self.bounds.borrow();
        for index in (0..requests.len()).rev() {
            let handle = MacosSettingsControlHandle(index as u64 + 1);
            if destroyed.contains(&handle) {
                continue;
            }
            let is_visible = visible
                .iter()
                .rev()
                .find(|(changed_handle, _)| *changed_handle == handle)
                .map(|(_, value)| *value)
                .unwrap_or(true);
            let is_enabled = enabled
                .iter()
                .rev()
                .find(|(changed_handle, _)| *changed_handle == handle)
                .map(|(_, value)| *value)
                .unwrap_or(true);
            if !is_visible || !is_enabled {
                continue;
            }
            let bounds = bounds_changes
                .iter()
                .rev()
                .find(|(changed_handle, _)| *changed_handle == handle)
                .map(|(_, bounds)| *bounds)
                .unwrap_or(requests[index].spec.bounds);
            if bounds.contains(point.x, point.y) {
                return Some(handle);
            }
        }
        None
    }

    fn control_screen_bounds(&self, handle: Self::Handle) -> Option<UiRect> {
        self.screen_bounds_queries.borrow_mut().push(handle);
        if let Some((_, bounds)) = self
            .bounds
            .borrow()
            .iter()
            .rev()
            .find(|(bounds_handle, _)| *bounds_handle == handle)
        {
            return Some(*bounds);
        }
        let index = handle.0.checked_sub(1)? as usize;
        self.requests
            .borrow()
            .get(index)
            .map(|request| request.spec.bounds)
    }

    fn control_text(&self, handle: Self::Handle) -> String {
        self.text
            .borrow()
            .iter()
            .rev()
            .find(|(text_handle, _)| *text_handle == handle)
            .map(|(_, text)| text.clone())
            .unwrap_or_default()
    }

    fn set_control_text(&mut self, handle: Self::Handle, text: &str) {
        self.text.borrow_mut().push((handle, text.to_string()));
    }

    fn request_control_repaint(&mut self, handle: Self::Handle) -> bool {
        self.repainted.borrow_mut().push(handle);
        true
    }
}

impl MacosSettingsControlHost {
    pub(crate) fn requests(&self) -> Vec<MacosSettingsControlRequest> {
        self.requests.borrow().clone()
    }

    pub(crate) fn destroyed(&self) -> Vec<MacosSettingsControlHandle> {
        self.destroyed.borrow().clone()
    }

    pub(crate) fn visibility_changes(&self) -> Vec<(MacosSettingsControlHandle, bool)> {
        self.visible.borrow().clone()
    }

    pub(crate) fn enabled_changes(&self) -> Vec<(MacosSettingsControlHandle, bool)> {
        self.enabled.borrow().clone()
    }

    pub(crate) fn bounds_changes(&self) -> Vec<(MacosSettingsControlHandle, UiRect)> {
        self.bounds.borrow().clone()
    }

    pub(crate) fn hit_test_queries(&self) -> Vec<Point> {
        self.hit_test_queries.borrow().clone()
    }

    pub(crate) fn screen_bounds_queries(&self) -> Vec<MacosSettingsControlHandle> {
        self.screen_bounds_queries.borrow().clone()
    }

    pub(crate) fn text_changes(&self) -> Vec<(MacosSettingsControlHandle, String)> {
        self.text.borrow().clone()
    }

    pub(crate) fn repainted(&self) -> Vec<MacosSettingsControlHandle> {
        self.repainted.borrow().clone()
    }
}

impl NativeSettingsWindowHost for MacosSettingsWindowHost {
    type Handle = MacosSettingsWindowHandle;

    fn present_settings_window(
        &mut self,
        request: NativeSettingsWindowRequest<Self::Handle>,
    ) -> NativeSettingsWindowPresentation<Self::Handle> {
        self.requests.borrow_mut().push(MacosSettingsWindowRequest {
            owner: request.owner,
            existing: request.existing,
            bounds: request.bounds,
        });
        if let Some(existing) = request.existing.or(self.open_handle) {
            self.open_handle = Some(existing);
            return NativeSettingsWindowPresentation::FocusedExisting(existing);
        }

        let handle = MacosSettingsWindowHandle(1);
        self.open_handle = Some(handle);
        self.window_bounds = Some(request.bounds);
        NativeSettingsWindowPresentation::Created(handle)
    }

    fn set_settings_window_bounds(&mut self, handle: Self::Handle, bounds: UiRect) {
        self.bounds_updates.borrow_mut().push((handle, bounds));
        if self.open_handle == Some(handle) {
            self.window_bounds = Some(bounds);
        }
    }

    fn destroy_settings_window(&mut self, handle: Self::Handle) {
        self.destroyed.borrow_mut().push(handle);
        if self.open_handle == Some(handle) {
            self.open_handle = None;
            self.window_bounds = None;
        }
    }

    fn focus_settings_window(&mut self, handle: Self::Handle) {
        self.focused.borrow_mut().push(handle);
    }

    fn track_settings_pointer_leave(&mut self, handle: Self::Handle) -> bool {
        self.pointer_leave_tracking.borrow_mut().push(handle);
        self.open_handle == Some(handle)
    }

    fn capture_settings_pointer(&mut self, handle: Self::Handle) {
        self.captured.borrow_mut().push(handle);
    }

    fn release_settings_pointer(&mut self, handle: Self::Handle) {
        self.released.borrow_mut().push(handle);
    }

    fn request_settings_window_repaint(&mut self, handle: Self::Handle) -> bool {
        self.repainted.borrow_mut().push(handle);
        self.open_handle == Some(handle)
    }

    fn request_settings_window_area_repaint(
        &mut self,
        handle: Self::Handle,
        area: Option<UiRect>,
        erase: bool,
    ) -> bool {
        self.area_repaints.borrow_mut().push((handle, area, erase));
        self.open_handle == Some(handle)
    }

    fn settings_window_layout_dpi(&mut self, handle: Self::Handle) -> u32 {
        self.layout_dpi_queries.borrow_mut().push(handle);
        96
    }

    fn settings_window_client_to_screen(
        &mut self,
        handle: Self::Handle,
        point: Point,
    ) -> Option<Point> {
        self.client_to_screen.borrow_mut().push((handle, point));
        Some(point)
    }

    fn settings_window_client_bounds(&mut self, handle: Self::Handle) -> Option<UiRect> {
        self.client_bounds_queries.borrow_mut().push(handle);
        if self.open_handle != Some(handle) {
            return None;
        }
        let bounds = self.window_bounds?;
        Some(UiRect::new(
            0,
            0,
            (bounds.right - bounds.left).max(0),
            (bounds.bottom - bounds.top).max(0),
        ))
    }

    fn settings_window_bounds(&mut self, handle: Self::Handle) -> Option<UiRect> {
        self.window_bounds_queries.borrow_mut().push(handle);
        if self.open_handle == Some(handle) {
            self.window_bounds
        } else {
            None
        }
    }
}

impl MacosSettingsWindowHost {
    pub(crate) fn requests(&self) -> Vec<MacosSettingsWindowRequest> {
        self.requests.borrow().clone()
    }

    pub(crate) fn bounds_updates(&self) -> Vec<(MacosSettingsWindowHandle, UiRect)> {
        self.bounds_updates.borrow().clone()
    }

    pub(crate) fn destroyed(&self) -> Vec<MacosSettingsWindowHandle> {
        self.destroyed.borrow().clone()
    }

    pub(crate) fn focused(&self) -> Vec<MacosSettingsWindowHandle> {
        self.focused.borrow().clone()
    }

    pub(crate) fn pointer_leave_tracking(&self) -> Vec<MacosSettingsWindowHandle> {
        self.pointer_leave_tracking.borrow().clone()
    }

    pub(crate) fn captured(&self) -> Vec<MacosSettingsWindowHandle> {
        self.captured.borrow().clone()
    }

    pub(crate) fn released(&self) -> Vec<MacosSettingsWindowHandle> {
        self.released.borrow().clone()
    }

    pub(crate) fn repainted(&self) -> Vec<MacosSettingsWindowHandle> {
        self.repainted.borrow().clone()
    }

    pub(crate) fn area_repaints(&self) -> Vec<(MacosSettingsWindowHandle, Option<UiRect>, bool)> {
        self.area_repaints.borrow().clone()
    }

    pub(crate) fn layout_dpi_queries(&self) -> Vec<MacosSettingsWindowHandle> {
        self.layout_dpi_queries.borrow().clone()
    }

    pub(crate) fn client_to_screen_requests(&self) -> Vec<(MacosSettingsWindowHandle, Point)> {
        self.client_to_screen.borrow().clone()
    }

    pub(crate) fn client_bounds_queries(&self) -> Vec<MacosSettingsWindowHandle> {
        self.client_bounds_queries.borrow().clone()
    }

    pub(crate) fn window_bounds_queries(&self) -> Vec<MacosSettingsWindowHandle> {
        self.window_bounds_queries.borrow().clone()
    }

    pub(crate) fn request_cloud_settings_refresh(
        &mut self,
        handle: MacosSettingsWindowHandle,
    ) -> bool {
        self.cloud_refreshes.borrow_mut().push(handle);
        self.open_handle == Some(handle)
    }

    pub(crate) fn cloud_refreshes(&self) -> Vec<MacosSettingsWindowHandle> {
        self.cloud_refreshes.borrow().clone()
    }
}

impl NativeSettingsDropdownHost for MacosSettingsDropdownHost {
    type Handle = MacosSettingsDropdownHandle;
    type Owner = MacosSettingsWindowHandle;

    fn present_settings_dropdown(
        &mut self,
        request: NativeSettingsDropdownRequest<Self::Owner>,
    ) -> NativeSettingsDropdownPresentation<Self::Handle> {
        let handle = MacosSettingsDropdownHandle(1);
        let bounds = request.anchor;
        self.requests
            .borrow_mut()
            .push(MacosSettingsDropdownRequest {
                owner: request.owner,
                control_id: request.control_id,
                anchor: request.anchor,
                items: request.items,
                selected: request.selected,
                width: request.width,
            });
        self.bounds.borrow_mut().push((handle, bounds));
        NativeSettingsDropdownPresentation::Created(handle)
    }

    fn destroy_settings_dropdown(&mut self, handle: Self::Handle) {
        self.destroyed.borrow_mut().push(handle);
    }

    fn settings_dropdown_bounds(&self, handle: Self::Handle) -> Option<UiRect> {
        self.bounds
            .borrow()
            .iter()
            .rev()
            .find(|(bounds_handle, _)| *bounds_handle == handle)
            .map(|(_, bounds)| *bounds)
    }
}

impl MacosSettingsDropdownHost {
    pub(crate) fn requests(&self) -> Vec<MacosSettingsDropdownRequest> {
        self.requests.borrow().clone()
    }

    pub(crate) fn destroyed(&self) -> Vec<MacosSettingsDropdownHandle> {
        self.destroyed.borrow().clone()
    }

    pub(crate) fn bounds(&self) -> Vec<(MacosSettingsDropdownHandle, UiRect)> {
        self.bounds.borrow().clone()
    }
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn run() -> Result<(), String> {
    let summary = MacosUiHost::contract_summary();
    let launch_plan = macos_native_host_launch_plan();
    if launch_plan.enters_real_event_loop() {
        return crate::macos_native_host::run_real_appkit_host(summary);
    }
    run_macos_contract_scaffold(summary)
}

pub(crate) fn dispatch_macos_native_host_action(
    action: NativeHostUiAction,
) -> ProductAdapterCommandResult {
    let mut application = MacosApplicationModel::default();
    application.dispatch_ui_command(action.command());
    application
        .product_command_results()
        .last()
        .cloned()
        .unwrap_or(ProductAdapterCommandResult {
            accepted: false,
            result_name: "zsclip.no_native_host_action_result".to_string(),
        })
}

pub(crate) fn dispatch_macos_native_settings_action(
    action: NativeHostSettingsAction,
) -> ProductAdapterCommandResult {
    let mut application = MacosApplicationModel::default();
    application.dispatch_ui_command(action.command());
    application
        .product_command_results()
        .last()
        .cloned()
        .unwrap_or(ProductAdapterCommandResult {
            accepted: false,
            result_name: "zsclip.no_native_settings_action_result".to_string(),
        })
}

#[allow(dead_code)]
fn macos_native_settings_file() -> std::path::PathBuf {
    #[cfg(test)]
    if let Some(path) = macos_native_settings_file_override()
        .lock()
        .expect("macOS settings file override lock poisoned")
        .clone()
    {
        return path;
    }
    if let Ok(path) = std::env::var("ZSCLIP_NATIVE_SETTINGS_FILE") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return std::path::PathBuf::from(trimmed);
        }
    }
    let data_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|dir| dir.join("data")))
        .unwrap_or_else(|| std::path::PathBuf::from("data"));
    data_dir.join("settings.json")
}

#[cfg(test)]
fn macos_native_settings_file_override() -> &'static Mutex<Option<std::path::PathBuf>> {
    static OVERRIDE: OnceLock<Mutex<Option<std::path::PathBuf>>> = OnceLock::new();
    OVERRIDE.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
fn set_macos_native_settings_file_for_tests(path: Option<std::path::PathBuf>) {
    *macos_native_settings_file_override()
        .lock()
        .expect("macOS settings file override lock poisoned") = path;
}

#[allow(dead_code)]
fn macos_native_data_dir() -> std::path::PathBuf {
    macos_native_settings_file()
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| std::path::PathBuf::from("data"))
}

#[allow(dead_code)]
fn read_macos_native_settings_json(path: &std::path::Path) -> serde_json::Value {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

#[allow(dead_code)]
pub(crate) fn macos_native_settings_json_snapshot() -> serde_json::Value {
    read_macos_native_settings_json(&macos_native_settings_file())
}

#[allow(dead_code)]
pub(crate) fn macos_native_clipboard_capture_enabled() -> bool {
    macos_native_settings_json_snapshot()
        .get("clipboard_capture_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true)
}

#[allow(dead_code)]
pub(crate) fn macos_native_grouping_enabled() -> bool {
    macos_native_settings_json_snapshot()
        .get("grouping_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true)
}

#[allow(dead_code)]
pub(crate) fn macos_native_status_menu_action_state(
    action: NativeHostStatusMenuAction,
) -> Option<bool> {
    match action {
        NativeHostStatusMenuAction::ToggleClipboardCapture => {
            Some(macos_native_clipboard_capture_enabled())
        }
        #[cfg(feature = "lan-sync")]
        NativeHostStatusMenuAction::ToggleLanSync => macos_native_settings_json_snapshot()
            .get("lan_sync_enabled")
            .and_then(serde_json::Value::as_bool)
            .or_else(|| {
                macos_native_settings_json_snapshot()
                    .get("lan_enable")
                    .and_then(serde_json::Value::as_bool)
            }),
        _ => None,
    }
}

#[allow(dead_code)]
fn persist_macos_native_bool_toggle(
    control_key: &str,
    field_name: &str,
    default_current: bool,
) -> ProductAdapterCommandResult {
    let current = macos_native_settings_json_snapshot()
        .get(field_name)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default_current);
    let submission = crate::settings_model::settings_native_collect_submission(&[
        crate::settings_model::SettingsNativeSubmittedControlValue {
            control_key: control_key.to_string(),
            raw_value: (!current).to_string(),
        },
    ]);
    persist_macos_native_settings_submission(&submission)
}

#[allow(dead_code)]
pub(crate) fn persist_macos_native_settings_submission(
    submission: &crate::settings_model::SettingsNativeCollectSubmission,
) -> ProductAdapterCommandResult {
    let path = macos_native_settings_file();
    let existing_json = read_macos_native_settings_json(&path);
    let applied =
        crate::settings_model::settings_native_apply_submission_to_json(existing_json, submission);
    if applied.field_updates.is_empty() {
        return ProductAdapterCommandResult {
            accepted: applied.rejected_fields.is_empty(),
            result_name: format!(
                "zsclip.settings.native_save.no_updates.rejected_{}",
                applied.rejected_fields.len()
            ),
        };
    }

    let write_result = (|| -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        let text =
            serde_json::to_string_pretty(&applied.settings_json).map_err(|err| err.to_string())?;
        std::fs::write(&path, text).map_err(|err| err.to_string())
    })();

    if write_result.is_err() {
        return ProductAdapterCommandResult {
            accepted: false,
            result_name: "zsclip.settings.native_save_failed".to_string(),
        };
    }

    let autostart_result = macos_apply_autostart_from_settings_updates(&applied.field_updates);
    let autostart_failed = autostart_result
        .as_ref()
        .map(|result| !result.applied)
        .unwrap_or(false);
    ProductAdapterCommandResult {
        accepted: !autostart_failed,
        result_name: if autostart_result.is_some() {
            format!(
                "zsclip.settings.native_save.updates_{}.rejected_{}.autostart_{}",
                applied.field_updates.len(),
                applied.rejected_fields.len(),
                if autostart_failed {
                    "failed"
                } else {
                    "applied"
                }
            )
        } else {
            format!(
                "zsclip.settings.native_save.updates_{}.rejected_{}",
                applied.field_updates.len(),
                applied.rejected_fields.len()
            )
        },
    }
}

fn macos_apply_autostart_from_settings_updates(
    updates: &[crate::settings_model::SettingsNativeJsonFieldUpdate],
) -> Option<NativeAutostartApplyResult> {
    let enabled = crate::settings_model::settings_native_bool_field_update(updates, "auto_start")?;
    if cfg!(target_os = "macos") {
        let mut application = MacosApplicationModel::default();
        return Some(application.apply_autostart(enabled));
    }
    Some(NativeAutostartApplyResult::applied(
        enabled,
        if enabled {
            NativeAutostartStatus::enabled_at("macos_autostart_scaffold")
        } else {
            NativeAutostartStatus::disabled()
        },
    ))
}

pub(crate) fn dispatch_macos_native_settings_control_action(
    action: NativeHostSettingsControlAction,
) -> ProductAdapterCommandResult {
    let mut application = MacosApplicationModel::default();
    application.dispatch_ui_command(action.command());
    let command_result = application
        .product_command_results()
        .last()
        .cloned()
        .unwrap_or(ProductAdapterCommandResult {
            accepted: false,
            result_name: "zsclip.no_native_settings_control_action_result".to_string(),
        });

    if action == NativeHostSettingsControlAction::ToggleAutostart {
        let desired_enabled = !application.autostart_status().enabled;
        if cfg!(target_os = "macos") {
            let autostart = application.apply_autostart(desired_enabled);
            return ProductAdapterCommandResult {
                accepted: command_result.accepted && autostart.applied,
                result_name: if autostart.applied {
                    "zsclip.settings.toggle_autostart"
                } else {
                    "zsclip.settings.toggle_autostart_failed"
                }
                .to_string(),
            };
        }
        return ProductAdapterCommandResult {
            accepted: command_result.accepted,
            result_name: "zsclip.settings.toggle_autostart_scaffold".to_string(),
        };
    }

    command_result
}

pub(crate) fn dispatch_macos_native_settings_platform_action(
    action: NativeHostSettingsPlatformAction,
) -> ProductAdapterCommandResult {
    let result_name = match action {
        NativeHostSettingsPlatformAction::OpenSourceRepository => {
            if open_macos_url_or_file(macos_source_url()).is_ok() {
                "zsclip.settings.open_source_repository"
            } else {
                "zsclip.settings.open_source_repository_failed"
            }
        }
        NativeHostSettingsPlatformAction::CheckForUpdates => {
            if open_macos_url_or_file(macos_latest_release_url()).is_ok() {
                "zsclip.settings.check_for_updates"
            } else {
                "zsclip.settings.check_for_updates_failed"
            }
        }
        NativeHostSettingsPlatformAction::OpenWpsTaskpaneDocs => {
            if open_macos_url_or_file(macos_wps_taskpane_docs_url()).is_ok() {
                "zsclip.settings.open_wps_taskpane_docs"
            } else {
                "zsclip.settings.open_wps_taskpane_docs_failed"
            }
        }
    };
    ProductAdapterCommandResult {
        accepted: true,
        result_name: result_name.to_string(),
    }
}

fn macos_native_settings_platform_action_for_shared_action(
    action: SettingsAction,
) -> Option<NativeHostSettingsPlatformAction> {
    match action {
        SettingsAction::OpenSourceRepository => {
            Some(NativeHostSettingsPlatformAction::OpenSourceRepository)
        }
        SettingsAction::CheckForUpdates => Some(NativeHostSettingsPlatformAction::CheckForUpdates),
        SettingsAction::OpenWpsTaskpaneDocs => {
            Some(NativeHostSettingsPlatformAction::OpenWpsTaskpaneDocs)
        }
        _ => None,
    }
}

fn macos_native_cloud_sync_action_for_shared_action(
    action: SettingsAction,
) -> Option<crate::cloud_sync::CloudSyncAction> {
    match action {
        SettingsAction::SyncWebDavNow => Some(crate::cloud_sync::CloudSyncAction::SyncNow),
        SettingsAction::UploadWebDavConfig => {
            Some(crate::cloud_sync::CloudSyncAction::UploadConfig)
        }
        SettingsAction::ApplyWebDavConfig => {
            Some(crate::cloud_sync::CloudSyncAction::ApplyRemoteConfig)
        }
        SettingsAction::RestoreWebDavBackup => {
            Some(crate::cloud_sync::CloudSyncAction::RestoreBackup)
        }
        _ => None,
    }
}

fn macos_native_cloud_sync_config_from_json(
    settings_json: &serde_json::Value,
) -> crate::cloud_sync::CloudSyncConfig {
    fn field(settings_json: &serde_json::Value, key: &str) -> String {
        settings_json
            .get(key)
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string()
    }
    crate::cloud_sync::CloudSyncConfig {
        webdav_url: field(settings_json, "cloud_webdav_url"),
        webdav_user: field(settings_json, "cloud_webdav_user"),
        webdav_pass: field(settings_json, "cloud_webdav_pass"),
        remote_dir: field(settings_json, "cloud_remote_dir"),
    }
}

fn macos_native_cloud_sync_paths() -> crate::cloud_sync::CloudSyncPaths {
    let data_dir = macos_native_data_dir();
    crate::cloud_sync::CloudSyncPaths {
        settings_file: data_dir.join("settings.json"),
        db_file: data_dir.join("clipboard.db"),
        data_dir,
    }
}

fn macos_native_lan_runtime_context() -> crate::lan_sync_core::LanRuntimePlatformContext {
    crate::lan_sync_core::LanRuntimePlatformContext::new(
        macos_native_data_dir(),
        crate::lan_sync_core::LanRuntimeEventSink::None,
        macos_native_encrypt_secret_for_storage,
        macos_native_decrypt_secret_from_storage,
    )
}

fn macos_native_encrypt_secret_for_storage(secret: &str) -> Option<String> {
    Some(secret.to_string())
}

fn macos_native_decrypt_secret_from_storage(encoded: &str) -> Option<String> {
    Some(encoded.to_string())
}

fn macos_native_latest_lan_clip_envelope(
    runtime_settings: &crate::lan_sync_core::LanRuntimeSettings,
) -> Option<crate::lan_sync_core::LanClipEnvelope> {
    let projection = crate::db_runtime::native_clip_list_items(0, 1)
        .ok()?
        .into_iter()
        .next()?;
    let item = crate::db_runtime::native_clip_item(projection.id)
        .ok()
        .flatten()?;
    crate::lan_sync_core::lan_clip_envelope_from_native_clip_item(
        &runtime_settings.device_id,
        &item,
        item.id.max(0) as u64,
        macos_native_now_ms(),
    )
}

fn macos_native_latest_lan_file_paths() -> Vec<std::path::PathBuf> {
    let Some(item) = crate::db_runtime::native_clip_list_items(0, 1)
        .ok()
        .and_then(|items| items.into_iter().next())
        .and_then(|projection| {
            crate::db_runtime::native_clip_item(projection.id)
                .ok()
                .flatten()
        })
    else {
        return Vec::new();
    };
    if item.kind != ClipKind::Files {
        return Vec::new();
    }
    item.file_paths
        .unwrap_or_default()
        .into_iter()
        .map(std::path::PathBuf::from)
        .collect()
}

pub(crate) fn dispatch_macos_native_lan_background_clip_sync_once(
    runtime_settings: &crate::lan_sync_core::LanRuntimeSettings,
    trusted_devices: &[crate::lan_sync_core::LanDevice],
) -> ProductAdapterCommandResult {
    if !runtime_settings.lan_sync_enabled || runtime_settings.device_id.trim().is_empty() {
        return ProductAdapterCommandResult {
            accepted: false,
            result_name: "zsclip.lan.background_clip_sync.disabled_on_macos_native_host"
                .to_string(),
        };
    }
    let config = crate::lan_sync_core::LanRuntimeConfig::from_core_config(
        macos_native_lan_runtime_context(),
        runtime_settings.core_config(),
    );
    let execution = crate::lan_sync_core::execute_lan_background_clip_sync_once(
        &config,
        trusted_devices,
        macos_native_latest_lan_clip_envelope(runtime_settings),
        std::time::Duration::from_millis(250),
    );
    let latest_file_paths = macos_native_latest_lan_file_paths();
    let file_execution = (!latest_file_paths.is_empty()).then(|| {
        crate::lan_sync_core::execute_lan_file_payload_transfer_once(
            &config,
            trusted_devices,
            &latest_file_paths,
            crate::lan_sync_core::LAN_FILE_AUTO_MAX_BYTES,
            std::time::Duration::from_secs(20),
        )
    });
    let file_pushed_count = file_execution
        .as_ref()
        .map(|execution| execution.pushed_count)
        .unwrap_or(0);
    let file_failed_count = file_execution
        .as_ref()
        .map(|execution| execution.failed_count)
        .unwrap_or(0);
    ProductAdapterCommandResult {
        accepted: execution.failed_count == 0 && file_failed_count == 0,
        result_name: format!(
            "zsclip.lan.background_clip_sync.pushed_{}_pulled_{}_files_{}_failed_{}_on_macos_native_host",
            execution.pushed_count,
            execution.pulled_count,
            file_pushed_count,
            execution.failed_count + file_failed_count
        ),
    }
}

pub(crate) fn dispatch_macos_native_settings_webdav_action(
    action: SettingsAction,
) -> ProductAdapterCommandResult {
    let Some(cloud_action) = macos_native_cloud_sync_action_for_shared_action(action) else {
        return ProductAdapterCommandResult {
            accepted: false,
            result_name: "zsclip.settings_sync.not_webdav_action".to_string(),
        };
    };
    let config = macos_native_cloud_sync_config_from_json(&macos_native_settings_json_snapshot());
    let paths = macos_native_cloud_sync_paths();
    match crate::cloud_sync::perform_cloud_sync(cloud_action, &config, &paths) {
        Ok(outcome) => ProductAdapterCommandResult {
            accepted: true,
            result_name: format!("zsclip.settings_sync.webdav.ok.{}", outcome.status_text),
        },
        Err(err) => ProductAdapterCommandResult {
            accepted: false,
            result_name: format!("zsclip.settings_sync.webdav.failed.{err}"),
        },
    }
}

pub(crate) fn dispatch_macos_native_settings_lan_mobile_link_action(
    action: SettingsAction,
) -> Option<ProductAdapterCommandResult> {
    if !matches!(
        action,
        SettingsAction::CopyLanPairUrl
            | SettingsAction::CopyLanSetupUrl
            | SettingsAction::OpenLanSetupPage
    ) {
        return None;
    }
    let projection = settings_lan_mobile_link_projection_from_json(
        action,
        "macos_native_host",
        &macos_native_settings_json_snapshot(),
    )?;
    if !projection.accepted {
        return Some(ProductAdapterCommandResult {
            accepted: false,
            result_name: projection.result_name,
        });
    }
    let Some(target_url) = projection.target_url else {
        return Some(ProductAdapterCommandResult {
            accepted: false,
            result_name: format!("{}.missing_target_url", projection.result_name),
        });
    };
    let accepted = match action {
        SettingsAction::CopyLanPairUrl | SettingsAction::CopyLanSetupUrl => {
            MacosClipboardHost::write_text_ignored_by_monitors(&target_url)
        }
        SettingsAction::OpenLanSetupPage => {
            MacosShellOpenHost::default().open_path(&target_url);
            true
        }
        _ => false,
    };
    Some(ProductAdapterCommandResult {
        accepted,
        result_name: if accepted {
            projection.result_name
        } else {
            format!("{}.host_failed", projection.result_name)
        },
    })
}

pub(crate) fn dispatch_macos_native_settings_lan_device_book_action(
    action: SettingsAction,
) -> Option<ProductAdapterCommandResult> {
    if action != SettingsAction::RefreshLanDevices {
        return None;
    }
    let settings_json = macos_native_settings_json_snapshot();
    let runtime_settings =
        crate::lan_sync_core::lan_runtime_settings_from_settings_json(&settings_json);
    if runtime_settings.lan_sync_enabled && !runtime_settings.device_id.trim().is_empty() {
        let config = crate::lan_sync_core::LanRuntimeConfig::from_core_config(
            macos_native_lan_runtime_context(),
            runtime_settings.core_config(),
        );
        let _ = crate::lan_sync_core::probe_lan_discovery_once(
            &config,
            std::time::Duration::from_millis(250),
        );
    }
    let data_dir = macos_native_data_dir();
    let trusted_devices = crate::lan_sync_core::load_lan_devices_from_store(
        crate::lan_sync_core::lan_device_book_path(&data_dir),
        macos_native_decrypt_secret_from_storage,
        crate::lan_sync_core::normalize_lan_capabilities,
    );
    let discovered_devices = crate::lan_sync_core::load_lan_discovered_devices_from_store(
        crate::lan_sync_core::lan_discovered_device_cache_path(&data_dir),
        macos_native_decrypt_secret_from_storage,
        crate::lan_sync_core::normalize_lan_capabilities,
    );
    if runtime_settings.lan_sync_enabled {
        let _ = dispatch_macos_native_lan_background_clip_sync_once(
            &runtime_settings,
            &trusted_devices,
        );
    }
    let devices = crate::lan_sync_core::merge_lan_device_book_and_discovery_cache(
        trusted_devices,
        discovered_devices,
    )
    .into_iter()
    .map(|device| {
        settings_lan_device_projection(
            device.device_id,
            device.name,
            device.addr,
            device.tcp_port,
            device.last_seen_ms,
            device.trusted,
            device.capabilities,
        )
    })
    .collect();
    let projection = settings_lan_device_book_projection("macos_native_host", devices);
    Some(ProductAdapterCommandResult {
        accepted: projection.accepted,
        result_name: projection.result_name,
    })
}

fn macos_native_now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn macos_native_save_lan_accepted_device(
    accepted_device: SettingsLanAcceptedDeviceProjection,
) -> std::io::Result<()> {
    let new_device = crate::lan_sync_core::LanDevice {
        device_id: accepted_device.device.device_id,
        name: accepted_device.device.name,
        addr: accepted_device.addr,
        tcp_port: accepted_device.tcp_port,
        token: accepted_device.token,
        last_seen_ms: accepted_device.device.last_seen_ms,
        trusted: true,
        capabilities: crate::lan_sync_core::normalize_lan_capabilities(
            accepted_device.device.capabilities,
            accepted_device.tcp_port,
        ),
    };
    crate::lan_sync_core::upsert_lan_device_in_store(
        macos_native_data_dir(),
        new_device,
        macos_native_decrypt_secret_from_storage,
        macos_native_encrypt_secret_for_storage,
        crate::lan_sync_core::normalize_lan_capabilities,
    )
}

pub(crate) fn dispatch_macos_native_settings_lan_pair_approval_action(
    action: SettingsAction,
) -> Option<ProductAdapterCommandResult> {
    let accept = match action {
        SettingsAction::AcceptLanPairing => true,
        SettingsAction::RejectLanPairing => false,
        _ => return None,
    };
    let action_name = if accept {
        "accept_lan_pairing"
    } else {
        "reject_lan_pairing"
    };
    match crate::lan_sync_core::apply_lan_pending_pair_decision_in_store(
        macos_native_data_dir(),
        None,
        accept,
        macos_native_now_ms(),
        macos_native_decrypt_secret_from_storage,
        macos_native_encrypt_secret_for_storage,
        crate::lan_sync_core::normalize_lan_capabilities,
    ) {
        Ok(Some(decision)) => ProductAdapterCommandResult {
            accepted: true,
            result_name: if decision.accepted {
                format!("zsclip.settings_sync.{action_name}.accepted_saved_on_macos_native_host")
            } else {
                format!("zsclip.settings_sync.{action_name}.rejected_on_macos_native_host")
            },
        },
        Ok(None) => ProductAdapterCommandResult {
            accepted: false,
            result_name: format!(
                "zsclip.settings_sync.{action_name}.no_pending_pair_on_macos_native_host"
            ),
        },
        Err(err) => ProductAdapterCommandResult {
            accepted: false,
            result_name: format!("zsclip.settings_sync.{action_name}.store_failed.{err}"),
        },
    }
    .into()
}

pub(crate) fn dispatch_macos_native_settings_lan_pair_action(
    action: SettingsAction,
) -> Option<ProductAdapterCommandResult> {
    if action != SettingsAction::PairLanDevice {
        return None;
    }
    let projection = settings_lan_pair_request_projection_from_json(
        action,
        "macos_native_host",
        &macos_native_settings_json_snapshot(),
    )?;
    if !projection.accepted {
        return Some(ProductAdapterCommandResult {
            accepted: false,
            result_name: projection.result_name,
        });
    }
    let Some(host) = projection.host else {
        return Some(ProductAdapterCommandResult {
            accepted: false,
            result_name: format!("{}.missing_host", projection.result_name),
        });
    };
    let Some(body) = projection.request_body_json else {
        return Some(ProductAdapterCommandResult {
            accepted: false,
            result_name: format!("{}.missing_body", projection.result_name),
        });
    };
    Some(
        match crate::lan_sync_core::http_request(
            "POST",
            &host,
            "/v1/pair/request",
            &[("Content-Type", "application/json")],
            Some(body.as_bytes()),
            std::time::Duration::from_secs(5),
        ) {
            Ok(response) => {
                let response_projection = settings_lan_pair_request_response_projection(
                    &projection.result_name,
                    &response,
                );
                if !response_projection.accepted {
                    return Some(ProductAdapterCommandResult {
                        accepted: false,
                        result_name: response_projection.result_name,
                    });
                }
                let Some(pair_id) = response_projection.pair_id else {
                    return Some(ProductAdapterCommandResult {
                        accepted: false,
                        result_name: format!("{}.missing_pair_id", response_projection.result_name),
                    });
                };
                let status_path = format!("/v1/pair/status?id={pair_id}");
                match crate::lan_sync_core::http_request(
                    "GET",
                    &host,
                    &status_path,
                    &[],
                    None,
                    std::time::Duration::from_secs(5),
                ) {
                    Ok(status_response) => {
                        let status_projection = settings_lan_pair_status_projection(
                            &response_projection.result_name,
                            &host,
                            projection.tcp_port,
                            macos_native_now_ms(),
                            &status_response,
                        );
                        if let Some(accepted_device) = status_projection.accepted_device {
                            match macos_native_save_lan_accepted_device(accepted_device) {
                                Ok(()) => ProductAdapterCommandResult {
                                    accepted: true,
                                    result_name: format!("{}.saved", status_projection.result_name),
                                },
                                Err(err) => ProductAdapterCommandResult {
                                    accepted: false,
                                    result_name: format!(
                                        "{}.save_failed.{err}",
                                        status_projection.result_name
                                    ),
                                },
                            }
                        } else {
                            ProductAdapterCommandResult {
                                accepted: status_projection.accepted
                                    || status_projection.status_name == "pending",
                                result_name: status_projection.result_name,
                            }
                        }
                    }
                    Err(err) => ProductAdapterCommandResult {
                        accepted: true,
                        result_name: format!(
                            "{}.status_poll_failed.{err}",
                            response_projection.result_name
                        ),
                    },
                }
            }
            Err(err) => ProductAdapterCommandResult {
                accepted: false,
                result_name: format!("{}.request_failed.{err}", projection.result_name),
            },
        },
    )
}

pub(crate) fn dispatch_macos_native_settings_route_action(
    route_name: &str,
    action_name: &str,
) -> ProductAdapterCommandResult {
    let Some(action) = settings_action_for_route(route_name, action_name) else {
        return ProductAdapterCommandResult {
            accepted: false,
            result_name: format!(
                "zsclip.settings.unknown_route.{}.{}",
                route_name, action_name
            ),
        };
    };

    match settings_action_route(action) {
        SettingsActionRoute::Platform => {
            if let Some(platform_action) =
                macos_native_settings_platform_action_for_shared_action(action)
            {
                return dispatch_macos_native_settings_platform_action(platform_action);
            }
            ProductAdapterCommandResult {
                accepted: false,
                result_name: format!(
                    "zsclip.settings_platform.{}.unsupported_on_macos_native_host",
                    action_name
                ),
            }
        }
        SettingsActionRoute::Sync => {
            if macos_native_cloud_sync_action_for_shared_action(action).is_some() {
                return dispatch_macos_native_settings_webdav_action(action);
            }
            if let Some(result) = dispatch_macos_native_settings_lan_mobile_link_action(action) {
                return result;
            }
            if let Some(result) = dispatch_macos_native_settings_lan_device_book_action(action) {
                return result;
            }
            if let Some(result) = dispatch_macos_native_settings_lan_pair_approval_action(action) {
                return result;
            }
            if let Some(result) = dispatch_macos_native_settings_lan_pair_action(action) {
                return result;
            }
            let support_status_name =
                zsui_native_feature_status_for(NativeUiPlatform::Macos, "sync_lan")
                    .map(|status| status.support_status_name)
                    .unwrap_or("unknown_support_status");
            if let Some(plan) = settings_lan_sync_action_support_plan(
                action,
                "macos_native_host",
                support_status_name,
            ) {
                return ProductAdapterCommandResult {
                    accepted: plan.accepted,
                    result_name: plan.result_name,
                };
            }
            ProductAdapterCommandResult {
                accepted: false,
                result_name: format!(
                    "zsclip.settings_sync.{}.unsupported_on_macos_native_host",
                    action_name
                ),
            }
        }
        SettingsActionRoute::Group => ProductAdapterCommandResult {
            accepted: false,
            result_name: format!(
                "zsclip.settings_group.{}.requires_settings_group_context",
                action_name
            ),
        },
    }
}

pub(crate) fn dispatch_macos_native_dialog_action(
    action: NativeHostDialogAction,
) -> ProductAdapterCommandResult {
    let dialog_host = MacosDialogHost::default();
    let result_name = match action {
        NativeHostDialogAction::ShowInfoMessage => {
            dialog_host.show_message(
                (),
                action.title(),
                action.message(),
                NativeDialogLevel::Info,
            );
            "zsclip.dialog.show_info_message".to_string()
        }
        NativeHostDialogAction::ConfirmQuestion => {
            let response = dialog_host.confirm(
                (),
                action.title(),
                action.message(),
                NativeDialogLevel::Question,
                NativeDialogButtons::YesNo,
            );
            format!(
                "zsclip.dialog.confirm_{}",
                native_dialog_response_name(response)
            )
        }
    };
    ProductAdapterCommandResult {
        accepted: true,
        result_name,
    }
}

pub(crate) fn dispatch_macos_native_status_menu_action(
    action: NativeHostStatusMenuAction,
) -> ProductAdapterCommandResult {
    let mut application = MacosApplicationModel::default();
    application.dispatch_ui_command(action.command());
    let command_result = application
        .product_command_results()
        .last()
        .cloned()
        .unwrap_or(ProductAdapterCommandResult {
            accepted: false,
            result_name: "zsclip.no_native_status_menu_action_result".to_string(),
        });
    let persist_result = match action {
        NativeHostStatusMenuAction::ToggleClipboardCapture => Some(
            persist_macos_native_bool_toggle("capture_enable", "clipboard_capture_enabled", true),
        ),
        #[cfg(feature = "lan-sync")]
        NativeHostStatusMenuAction::ToggleLanSync => Some(persist_macos_native_bool_toggle(
            "lan_enable",
            "lan_sync_enabled",
            false,
        )),
        _ => None,
    };
    if persist_result
        .as_ref()
        .map(|result| !result.accepted)
        .unwrap_or(false)
    {
        return ProductAdapterCommandResult {
            accepted: false,
            result_name: format!("{}.settings_failed", command_result.result_name),
        };
    }
    command_result
}

pub(crate) fn dispatch_macos_native_menu_command_id(menu_id: usize) -> ProductAdapterCommandResult {
    let Some(command) = main_menu_command_for_id(menu_id) else {
        return ProductAdapterCommandResult {
            accepted: false,
            result_name: format!("zsclip.invalid_native_menu_command_{}", menu_id),
        };
    };
    let mut application = MacosApplicationModel::default();
    application.dispatch_ui_command(command);
    application
        .product_command_results()
        .last()
        .cloned()
        .unwrap_or(ProductAdapterCommandResult {
            accepted: false,
            result_name: "zsclip.no_native_menu_command_result".to_string(),
        })
}

pub(crate) fn dispatch_macos_native_row_action(
    action: NativeHostRowAction,
) -> ProductAdapterCommandResult {
    let mut application = MacosApplicationModel::default();
    application.dispatch_ui_command(action.command());
    application
        .product_command_results()
        .last()
        .cloned()
        .unwrap_or(ProductAdapterCommandResult {
            accepted: false,
            result_name: "zsclip.no_native_row_action_result".to_string(),
        })
}

pub(crate) fn dispatch_macos_native_row_action_for_item(
    action: NativeHostRowAction,
    item_id: i64,
) -> ProductAdapterCommandResult {
    let item = match crate::db_runtime::native_clip_item(item_id) {
        Ok(Some(item)) => item,
        Ok(None) => {
            return ProductAdapterCommandResult {
                accepted: false,
                result_name: format!("{}.missing_item", action.action_name()),
            };
        }
        Err(_) => {
            return ProductAdapterCommandResult {
                accepted: false,
                result_name: format!("{}.load_failed", action.action_name()),
            };
        }
    };

    match action {
        NativeHostRowAction::Copy | NativeHostRowAction::Paste => {
            let Some(write) = crate::app_core::native_host_clipboard_write_for_item(&item) else {
                return ProductAdapterCommandResult {
                    accepted: false,
                    result_name: format!("{}.empty_payload", action.action_name()),
                };
            };
            let accepted =
                crate::app_core::native_host_write_clipboard_payload::<MacosClipboardHost>(&write);
            ProductAdapterCommandResult {
                accepted,
                result_name: if accepted {
                    format!("{}.clipboard_{}", action.action_name(), write.kind_name())
                } else {
                    format!("{}.clipboard_failed", action.action_name())
                },
            }
        }
        NativeHostRowAction::Pin => {
            let pinned = !item.pinned;
            match crate::db_runtime::update_native_clip_items_pinned(&[item.id], pinned) {
                Ok(affected) if affected > 0 => ProductAdapterCommandResult {
                    accepted: true,
                    result_name: if pinned {
                        "zsclip.row.pin_db".to_string()
                    } else {
                        "zsclip.row.unpin_db".to_string()
                    },
                },
                Ok(_) => ProductAdapterCommandResult {
                    accepted: false,
                    result_name: "zsclip.row.pin_missing".to_string(),
                },
                Err(_) => ProductAdapterCommandResult {
                    accepted: false,
                    result_name: "zsclip.row.pin_failed".to_string(),
                },
            }
        }
        NativeHostRowAction::Delete => {
            match crate::db_runtime::delete_native_clip_items(&[item.id]) {
                Ok(affected) if affected > 0 => ProductAdapterCommandResult {
                    accepted: true,
                    result_name: "zsclip.row.delete_db".to_string(),
                },
                Ok(_) => ProductAdapterCommandResult {
                    accepted: false,
                    result_name: "zsclip.row.delete_missing".to_string(),
                },
                Err(_) => ProductAdapterCommandResult {
                    accepted: false,
                    result_name: "zsclip.row.delete_failed".to_string(),
                },
            }
        }
        NativeHostRowAction::ToPhrase => {
            match crate::db_runtime::insert_native_phrase_from_item(&item, "ZSClip") {
                Ok(outcome) if outcome.inserted => ProductAdapterCommandResult {
                    accepted: true,
                    result_name: "zsclip.row.to_phrase_db".to_string(),
                },
                Ok(outcome) if outcome.reason == "duplicate" => ProductAdapterCommandResult {
                    accepted: true,
                    result_name: "zsclip.row.to_phrase_duplicate".to_string(),
                },
                Ok(outcome) => ProductAdapterCommandResult {
                    accepted: false,
                    result_name: format!("zsclip.row.to_phrase_{}", outcome.reason),
                },
                Err(_) => ProductAdapterCommandResult {
                    accepted: false,
                    result_name: "zsclip.row.to_phrase_failed".to_string(),
                },
            }
        }
        NativeHostRowAction::OpenPath => {
            match main_row_external_action_plan(MainRowMenuAction::OpenPath, Some(&item), &[]) {
                Some(MainRowExternalActionPlan::OpenPaths(paths)) if !paths.is_empty() => {
                    let host = MacosShellOpenHost::default();
                    for path in &paths {
                        host.open_path(path);
                    }
                    ProductAdapterCommandResult {
                        accepted: true,
                        result_name: format!("zsclip.row.open_path_native_{}", paths.len()),
                    }
                }
                _ => ProductAdapterCommandResult {
                    accepted: false,
                    result_name: "zsclip.row.open_path_no_paths".to_string(),
                },
            }
        }
        #[cfg(feature = "ai-actions")]
        NativeHostRowAction::TextTranslate => {
            match main_row_external_action_plan(MainRowMenuAction::TextTranslate, Some(&item), &[])
            {
                Some(MainRowExternalActionPlan::TextTranslate(text)) if !text.is_empty() => {
                    ProductAdapterCommandResult {
                        accepted: true,
                        result_name: format!("zsclip.row.text_translate_ready_{}", text.len()),
                    }
                }
                _ => ProductAdapterCommandResult {
                    accepted: false,
                    result_name: "zsclip.row.text_translate_no_text".to_string(),
                },
            }
        }
        _ => dispatch_macos_native_row_action(action),
    }
}

pub(crate) fn dispatch_macos_native_edit_text_save(
    item_id: i64,
    text: &str,
) -> ProductAdapterCommandResult {
    match crate::db_runtime::update_item_text(item_id, text) {
        Ok(true) => ProductAdapterCommandResult {
            accepted: true,
            result_name: "zsclip.row.edit.save_db".to_string(),
        },
        Ok(false) => ProductAdapterCommandResult {
            accepted: false,
            result_name: "zsclip.row.edit.save_missing".to_string(),
        },
        Err(_) => ProductAdapterCommandResult {
            accepted: false,
            result_name: "zsclip.row.edit.save_failed".to_string(),
        },
    }
}

pub(crate) fn dispatch_macos_native_assign_group(
    item_id: i64,
    group_id: i64,
) -> ProductAdapterCommandResult {
    match crate::db_runtime::assign_native_clip_group(&[item_id], group_id) {
        Ok(affected) if affected > 0 => ProductAdapterCommandResult {
            accepted: true,
            result_name: "zsclip.row.assign_group_db".to_string(),
        },
        Ok(_) => ProductAdapterCommandResult {
            accepted: false,
            result_name: "zsclip.row.assign_group_missing".to_string(),
        },
        Err(_) => ProductAdapterCommandResult {
            accepted: false,
            result_name: "zsclip.row.assign_group_failed".to_string(),
        },
    }
}

pub(crate) fn dispatch_macos_native_remove_group(item_id: i64) -> ProductAdapterCommandResult {
    match crate::db_runtime::assign_native_clip_group(&[item_id], 0) {
        Ok(affected) if affected > 0 => ProductAdapterCommandResult {
            accepted: true,
            result_name: "zsclip.row.remove_group_db".to_string(),
        },
        Ok(_) => ProductAdapterCommandResult {
            accepted: false,
            result_name: "zsclip.row.remove_group_missing".to_string(),
        },
        Err(_) => ProductAdapterCommandResult {
            accepted: false,
            result_name: "zsclip.row.remove_group_failed".to_string(),
        },
    }
}

pub(crate) fn dispatch_macos_native_group_filter(group_id: i64) -> ProductAdapterCommandResult {
    ProductAdapterCommandResult {
        accepted: group_id >= 0,
        result_name: if group_id > 0 {
            "zsclip.group_filter.select_db"
        } else {
            "zsclip.group_filter.all_db"
        }
        .to_string(),
    }
}

pub(crate) fn dispatch_macos_native_create_group(
    category: i64,
    name: &str,
) -> ProductAdapterCommandResult {
    if name.trim().is_empty() {
        return ProductAdapterCommandResult {
            accepted: false,
            result_name: "zsclip.settings.group.create_empty".to_string(),
        };
    }
    match crate::db_runtime::create_native_clip_group(category, name.trim()) {
        Ok(_) => ProductAdapterCommandResult {
            accepted: true,
            result_name: "zsclip.settings.group.create_db".to_string(),
        },
        Err(_) => ProductAdapterCommandResult {
            accepted: false,
            result_name: "zsclip.settings.group.create_failed".to_string(),
        },
    }
}

pub(crate) fn dispatch_macos_native_rename_group(
    category: i64,
    group_id: i64,
    name: &str,
) -> ProductAdapterCommandResult {
    if name.trim().is_empty() {
        return ProductAdapterCommandResult {
            accepted: false,
            result_name: "zsclip.settings.group.rename_empty".to_string(),
        };
    }
    match crate::db_runtime::rename_native_clip_group(category, group_id, name.trim()) {
        Ok(true) => ProductAdapterCommandResult {
            accepted: true,
            result_name: "zsclip.settings.group.rename_db".to_string(),
        },
        Ok(false) => ProductAdapterCommandResult {
            accepted: false,
            result_name: "zsclip.settings.group.rename_missing".to_string(),
        },
        Err(_) => ProductAdapterCommandResult {
            accepted: false,
            result_name: "zsclip.settings.group.rename_failed".to_string(),
        },
    }
}

pub(crate) fn dispatch_macos_native_delete_group(group_id: i64) -> ProductAdapterCommandResult {
    match crate::db_runtime::delete_native_clip_group(group_id) {
        Ok(true) => ProductAdapterCommandResult {
            accepted: true,
            result_name: "zsclip.settings.group.delete_db".to_string(),
        },
        Ok(false) => ProductAdapterCommandResult {
            accepted: false,
            result_name: "zsclip.settings.group.delete_missing".to_string(),
        },
        Err(_) => ProductAdapterCommandResult {
            accepted: false,
            result_name: "zsclip.settings.group.delete_failed".to_string(),
        },
    }
}

pub(crate) fn dispatch_macos_native_move_group(
    category: i64,
    group_id: i64,
    step: i32,
) -> ProductAdapterCommandResult {
    match crate::db_runtime::move_native_clip_group(category, group_id, step) {
        Ok(true) => ProductAdapterCommandResult {
            accepted: true,
            result_name: "zsclip.settings.group.move_db".to_string(),
        },
        Ok(false) => ProductAdapterCommandResult {
            accepted: false,
            result_name: "zsclip.settings.group.move_unchanged".to_string(),
        },
        Err(_) => ProductAdapterCommandResult {
            accepted: false,
            result_name: "zsclip.settings.group.move_failed".to_string(),
        },
    }
}

pub(crate) fn dispatch_macos_native_search_text_action(
    action: NativeHostSearchTextAction,
) -> ProductAdapterCommandResult {
    let mut application = MacosApplicationModel::default();
    application.dispatch_ui_command(action.command());
    application
        .product_command_results()
        .last()
        .cloned()
        .unwrap_or(ProductAdapterCommandResult {
            accepted: false,
            result_name: "zsclip.no_native_search_action_result".to_string(),
        })
}

pub(crate) fn dispatch_macos_native_vv_select_event(
    index: usize,
) -> ProductAdapterAsyncBridgeResult {
    let mut application = MacosApplicationModel::default();
    application.route_application_event(crate::app_core::native_host_vv_select_event(index));
    application
        .product_event_results()
        .last()
        .cloned()
        .unwrap_or(ProductAdapterAsyncBridgeResult {
            bridged: false,
            event_name: "zsclip.no_native_vv_select_event".to_string(),
        })
}

#[allow(dead_code)]
fn macos_native_vv_trigger_state() -> &'static Mutex<NativeHostVvTriggerState> {
    static STATE: OnceLock<Mutex<NativeHostVvTriggerState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(NativeHostVvTriggerState::default()))
}

#[allow(dead_code)]
pub(crate) fn dispatch_macos_native_vv_trigger_key(
    input: NativeHostVvTriggerInput,
) -> NativeHostVvTriggerTransition {
    let Ok(mut state) = macos_native_vv_trigger_state().lock() else {
        return NativeHostVvTriggerTransition {
            action: NativeHostVvTriggerAction::Ignore,
            consume_key: false,
        };
    };
    let transition = state.apply(input);
    let mut application = MacosApplicationModel::default();
    match transition.action {
        NativeHostVvTriggerAction::Show { target_token } => {
            application.route_application_event(ApplicationEvent::VvShowRequested {
                target: NativeWindowToken(target_token as usize),
            });
        }
        NativeHostVvTriggerAction::Hide => {
            application.route_application_event(ApplicationEvent::VvHideRequested);
        }
        NativeHostVvTriggerAction::Select { index } => {
            application
                .route_application_event(crate::app_core::native_host_vv_select_event(index));
        }
        NativeHostVvTriggerAction::Ignore => {}
    }
    transition
}

fn macos_native_host_vv_clip_items_for_group(group_id: i64) -> Vec<ClipItem> {
    macos_native_host_projected_clip_items_for_group(group_id)
        .into_iter()
        .map(|item| {
            crate::db_runtime::native_clip_item(item.id)
                .ok()
                .flatten()
                .unwrap_or_else(|| crate::app_core::native_host_clip_item_from_projection(&item))
        })
        .collect()
}

pub(crate) fn dispatch_macos_native_vv_paste(index: usize) -> NativeHostVvPasteExecution {
    dispatch_macos_native_vv_paste_for_group(index, 0)
}

pub(crate) fn dispatch_macos_native_vv_paste_for_group(
    index: usize,
    group_id: i64,
) -> NativeHostVvPasteExecution {
    let items = macos_native_host_vv_clip_items_for_group(group_id);
    let mut application = MacosApplicationModel::default();
    application
        .paste_target_host_mut()
        .set_next_foreground_result(true);
    application
        .paste_target_host_mut()
        .set_next_text_input_capabilities(PasteTargetTextInputCapabilities::text_input());
    application
        .paste_target_host_mut()
        .set_next_text_input_ready(true);
    application.execute_native_vv_paste(
        index,
        &items,
        MacosPasteTargetHandle(7),
        MacosPasteTargetHandle(8),
        0,
    )
}

#[allow(dead_code)]
pub(crate) fn macos_native_identity_smoke() -> MacosIdentitySmokeSummary {
    let current = MacosWindowIdentityHandle(u64::from(std::process::id()));
    let identity = MacosWindowIdentityHost::default();
    let foreground = identity.foreground_handle();
    let process_name = identity.process_name(current);
    let bundle_id = identity.class_name(current);
    let current_process_exists = identity.exists(current);
    let current_process_foreground = identity.is_foreground(current);
    let current_process_window = identity.is_current_process_window(current);
    let mut paste_target = MacosPasteTargetHost::default();
    let foreground_requested =
        paste_target.force_paste_target_foreground(MacosPasteTargetHandle(current.0));
    let focus_status = paste_target.paste_target_focus_status(
        MacosPasteTargetHandle(current.0),
        MacosPasteTargetHandle(foreground.0),
    );

    MacosIdentitySmokeSummary {
        current_pid: current.0,
        process_name_seen: !process_name.is_empty(),
        bundle_id_seen: !bundle_id.is_empty(),
        foreground_seen: foreground.0 != 0,
        current_process_exists,
        current_process_foreground,
        current_process_window,
        foreground_requested,
        focus_status,
    }
}

fn native_dialog_response_name(response: NativeDialogResponse) -> &'static str {
    match response {
        NativeDialogResponse::Yes => "yes",
        NativeDialogResponse::No => "no",
        NativeDialogResponse::Cancel => "cancel",
    }
}

#[cfg(target_os = "macos")]
fn open_macos_url_or_file(target: String) -> Result<(), String> {
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::{NSString, NSURL};

    let workspace = NSWorkspace::sharedWorkspace();
    let url = if target.contains("://") {
        let url_string = NSString::from_str(target.as_str());
        NSURL::URLWithString(&url_string).ok_or_else(|| "invalid URL".to_string())?
    } else {
        let path = NSString::from_str(target.as_str());
        NSURL::fileURLWithPath(&path)
    };
    if workspace.openURL(&url) {
        Ok(())
    } else {
        Err("NSWorkspace refused to open target".to_string())
    }
}

#[cfg(not(target_os = "macos"))]
fn open_macos_url_or_file(_target: String) -> Result<(), String> {
    Err("macOS host URL opening is only available on macOS".to_string())
}

#[cfg(target_os = "macos")]
fn prompt_macos_native_text(request: NativeTextInputDialogRequest<'_>) -> Option<String> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSAlert, NSAlertFirstButtonReturn, NSAlertStyle, NSTextField};
    use objc2_foundation::{ns_string, NSPoint, NSRect, NSSize, NSString};

    let mtm = MainThreadMarker::new()?;
    let alert = NSAlert::new(mtm);
    let title = NSString::from_str(request.title);
    let label = NSString::from_str(request.label);
    alert.setMessageText(&title);
    alert.setInformativeText(&label);
    alert.setAlertStyle(NSAlertStyle::Informational);
    alert.addButtonWithTitle(ns_string!("OK"));
    alert.addButtonWithTitle(ns_string!("Cancel"));

    let initial = NSString::from_str(request.initial);
    let field = NSTextField::labelWithString(&initial, mtm);
    field.setFrame(NSRect::new(
        NSPoint::new(0.0, 0.0),
        NSSize::new(320.0, 24.0),
    ));
    field.setEditable(true);
    field.setSelectable(true);
    field.setBezeled(true);
    alert.setAccessoryView(Some(&field));

    if alert.runModal() == NSAlertFirstButtonReturn {
        Some(field.stringValue().to_string())
    } else {
        None
    }
}

#[cfg(not(target_os = "macos"))]
fn prompt_macos_native_text(_request: NativeTextInputDialogRequest<'_>) -> Option<String> {
    None
}

#[cfg(target_os = "macos")]
fn edit_macos_native_text(
    request: NativeEditTextDialogRequest<'_>,
    save_handler: &mut dyn NativeEditTextSaveHandler,
) -> NativeEditTextDialogResult {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSAlert, NSAlertFirstButtonReturn, NSAlertStyle, NSTextField};
    use objc2_foundation::{ns_string, NSPoint, NSRect, NSSize, NSString};

    let Some(mtm) = MainThreadMarker::new() else {
        return NativeEditTextDialogResult::default();
    };
    let requested = request.initial_size.unwrap_or(crate::app_core::Size {
        width: 520,
        height: 160,
    });
    let editor_width = requested.width.clamp(320, 720) as f64;
    let editor_height = requested.height.clamp(72, 280) as f64;

    let alert = NSAlert::new(mtm);
    let title = NSString::from_str(request.title);
    alert.setMessageText(&title);
    alert.setInformativeText(ns_string!("Edit clipboard text"));
    alert.setAlertStyle(NSAlertStyle::Informational);
    alert.addButtonWithTitle(ns_string!("Save"));
    alert.addButtonWithTitle(ns_string!("Cancel"));

    let initial = NSString::from_str(request.initial_text);
    let field = NSTextField::labelWithString(&initial, mtm);
    field.setFrame(NSRect::new(
        NSPoint::new(0.0, 0.0),
        NSSize::new(editor_width, editor_height),
    ));
    field.setEditable(true);
    field.setSelectable(true);
    field.setBezeled(true);
    alert.setAccessoryView(Some(&field));

    if alert.runModal() != NSAlertFirstButtonReturn {
        return NativeEditTextDialogResult {
            saved: false,
            final_size: Some(requested),
        };
    }

    let saved = save_handler
        .save_text(&field.stringValue().to_string())
        .is_ok();
    NativeEditTextDialogResult {
        saved,
        final_size: Some(requested),
    }
}

#[cfg(not(target_os = "macos"))]
fn edit_macos_native_text(
    _request: NativeEditTextDialogRequest<'_>,
    _save_handler: &mut dyn NativeEditTextSaveHandler,
) -> NativeEditTextDialogResult {
    NativeEditTextDialogResult::default()
}

#[cfg(target_os = "macos")]
pub(crate) fn pick_macos_native_file(
    request: NativeFileDialogRequest<'_>,
) -> Result<Option<String>, String> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSModalResponseOK, NSOpenPanel};
    use objc2_foundation::{NSArray, NSString, NSURL};

    let mtm = MainThreadMarker::new()
        .ok_or_else(|| "NSOpenPanel must run on the macOS main thread".to_string())?;
    let panel = NSOpenPanel::openPanel(mtm);
    panel.setCanChooseFiles(true);
    panel.setCanChooseDirectories(false);
    panel.setAllowsMultipleSelection(false);
    panel.setTitle(Some(&NSString::from_str(request.title)));
    panel.setPrompt(Some(&NSString::from_str("Choose")));

    let file_types = native_file_dialog_extensions(request.filter_pattern);
    let retained_file_types: Vec<_> = file_types
        .iter()
        .map(|extension| NSString::from_str(extension))
        .collect();
    let file_type_refs: Vec<&NSString> = retained_file_types
        .iter()
        .map(|extension| extension.as_ref())
        .collect();
    if !file_type_refs.is_empty() {
        let allowed = NSArray::from_slice(&file_type_refs);
        #[allow(deprecated)]
        panel.setAllowedFileTypes(Some(&allowed));
    }

    if let Some(directory) = native_file_dialog_initial_directory(request.current_path) {
        if let Some(url) = NSURL::from_directory_path(directory) {
            panel.setDirectoryURL(Some(&url));
        }
    }

    let response = panel.runModal();
    if response == NSModalResponseOK {
        let url = panel
            .URL()
            .ok_or_else(|| "NSOpenPanel returned no selected URL".to_string())?;
        let path = url
            .to_file_path()
            .ok_or_else(|| "NSOpenPanel returned a non-file URL".to_string())?;
        Ok(Some(path.to_string_lossy().to_string()))
    } else {
        Ok(None)
    }
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn pick_macos_native_file(
    _request: NativeFileDialogRequest<'_>,
) -> Result<Option<String>, String> {
    Err("macOS native file picker is only available on macOS".to_string())
}

#[cfg(target_os = "macos")]
fn macos_source_url() -> String {
    option_env!("CARGO_PKG_REPOSITORY")
        .unwrap_or("")
        .trim()
        .to_string()
}

#[cfg(not(target_os = "macos"))]
fn macos_source_url() -> String {
    String::new()
}

#[cfg(target_os = "macos")]
fn macos_latest_release_url() -> String {
    let repo = macos_source_url();
    if repo.is_empty() {
        String::new()
    } else {
        format!("{}/releases/latest", repo.trim_end_matches('/'))
    }
}

#[cfg(not(target_os = "macos"))]
fn macos_latest_release_url() -> String {
    String::new()
}

#[cfg(target_os = "macos")]
fn macos_wps_taskpane_docs_url() -> String {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.join("docs")
        .join("wps-taskpane.md")
        .to_string_lossy()
        .to_string()
}

#[cfg(not(target_os = "macos"))]
fn macos_wps_taskpane_docs_url() -> String {
    String::new()
}

#[cfg(target_os = "macos")]
fn native_file_dialog_extensions(filter_pattern: &str) -> Vec<String> {
    filter_pattern
        .split([';', ','])
        .filter_map(|pattern| {
            pattern
                .trim()
                .strip_prefix("*.")
                .map(str::trim)
                .filter(|extension| !extension.is_empty() && !extension.contains('*'))
                .map(ToOwned::to_owned)
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn native_file_dialog_initial_directory(current_path: &str) -> Option<&std::path::Path> {
    let path = std::path::Path::new(current_path.trim());
    if path.as_os_str().is_empty() {
        None
    } else if path.extension().is_some() {
        path.parent()
    } else {
        Some(path)
    }
}

pub(crate) fn macos_native_host_projected_clip_items() -> Vec<NativeHostClipListItemProjection> {
    macos_native_host_projected_clip_items_for_group(0)
}

pub(crate) fn macos_native_host_projected_clip_items_for_group(
    group_id: i64,
) -> Vec<NativeHostClipListItemProjection> {
    if group_id > 0 {
        if let Ok(items) = crate::db_runtime::native_clip_list_items_for_group(0, group_id, 64) {
            return items;
        }
    } else if let Ok(items) = crate::db_runtime::native_clip_list_items(0, 64) {
        if !items.is_empty() {
            return items;
        }
    }
    let application = MacosApplicationModel::default();
    application
        .product_adapter
        .project_product_state()
        .native_clip_items
}

fn run_macos_contract_scaffold(summary: MacosHostContractSummary) -> Result<(), String> {
    let _adapter_boundary =
        crate::macos_appkit_adapter::MacosAppKitAdapterBoundary::default_from_macos_contract();
    let mut application = MacosApplicationModel::default();
    let startup = application.mount("ZSClip", true)?;
    let mut host = MacosMainWindowHost::default();
    let presentation = host.create_main_windows(startup.main_window.clone());
    let main_plan = application.main_window().initial_render_plan(
        startup.main_window.size.width,
        startup.main_window.size.height,
    );
    let NativeMainWindowPresentation::Created(handles) = presentation else {
        return Err("macOS main window host failed to create startup windows".to_string());
    };
    application.attach_main_windows(handles, startup.main_window.main_visible);
    application.note_main_render();
    host.apply_main_window_appearance(handles.main);
    if !application.activate() {
        return Err("macOS application lifecycle rejected activation".to_string());
    }
    eprintln!(
        "ZSClip macOS host scaffold loaded: app_core {}.{}, {} surfaces, {} main paint commands, lifecycle {:?}",
        summary.api_major,
        summary.api_minor,
        summary.surfaces,
        main_plan.chrome_commands.len(),
        application.lifecycle_phase()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn macos_clipboard_test_guard() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .expect("macOS clipboard test lock poisoned")
    }

    fn macos_settings_file_test_guard() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .expect("macOS settings file test lock poisoned")
    }

    fn native_settings_temp_file(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "zsclip-{name}-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn macos_native_host_launch_plan_targets_real_appkit_entry() {
        let plan = macos_native_host_launch_plan();

        assert_eq!(plan.platform_name(), "macos");
        assert_eq!(plan.toolkit_name(), "appkit_swiftui");
        assert_eq!(plan.entry_point, "macos_app::run");
        assert_eq!(plan.native_application_type, "objc2_app_kit::NSApplication");
        assert_eq!(plan.native_window_type, "objc2_app_kit::NSWindow");
        assert_eq!(plan.real_host_module_path, "src/macos_native_host.rs");
        assert!(plan.needs_target_os_verification());
        if cfg!(target_os = "macos") {
            assert_eq!(plan.mode_name(), "real_native_host");
            assert!(plan.enters_real_event_loop());
        } else {
            assert_eq!(plan.mode_name(), "contract_scaffold_fallback");
            assert!(!plan.enters_real_event_loop());
            assert!(
                crate::macos_native_host::run_real_appkit_host(MacosUiHost::contract_summary())
                    .is_err()
            );
        }
    }

    #[test]
    fn macos_autostart_host_writes_launch_agent_plist() {
        let root = std::env::temp_dir().join(format!(
            "zsclip-macos-autostart-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let executable = root
            .join("ZSClip Test.app")
            .join("Contents")
            .join("MacOS")
            .join("zsclip");
        let launch_agents = root.join("Library").join("LaunchAgents");
        let mut application = MacosApplicationModel::default();
        application.autostart_host =
            MacosAutostartHost::with_paths(launch_agents.clone(), executable.clone());

        let enabled = application.apply_autostart(true);
        assert!(enabled.applied, "{enabled:?}");
        assert!(application.autostart_status().enabled);
        let plist =
            std::fs::read_to_string(launch_agents.join("io.github.qiu7824.zsclip.plist")).unwrap();
        assert!(plist.contains("<string>io.github.qiu7824.zsclip</string>"));
        assert!(plist.contains("<key>ProgramArguments</key>"));
        assert!(plist.contains(&macos_plist_escape(&executable.to_string_lossy())));

        let disabled = application.apply_autostart(false);
        assert!(disabled.applied, "{disabled:?}");
        assert!(!application.autostart_status().enabled);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn macos_native_host_actions_enter_product_command_routes() {
        let settings = dispatch_macos_native_host_action(NativeHostUiAction::OpenSettings);
        assert!(settings.accepted);
        assert_eq!(settings.result_name, "zsclip.window.open_settings");

        let search =
            crate::macos_native_host::dispatch_appkit_host_action(NativeHostUiAction::ToggleSearch);
        assert!(search.accepted);
        assert_eq!(search.result_name, "zsclip.window.toggle_search");
    }

    #[test]
    fn macos_native_settings_actions_enter_product_command_routes() {
        let save = dispatch_macos_native_settings_action(NativeHostSettingsAction::Save);
        assert!(save.accepted);
        assert_eq!(save.result_name, "zsclip.settings.save");

        let config = crate::macos_native_host::dispatch_appkit_settings_action(
            NativeHostSettingsAction::OpenConfig,
        );
        assert!(config.accepted);
        assert_eq!(config.result_name, "zsclip.settings.open_config");

        let close = crate::macos_native_host::dispatch_appkit_settings_action(
            NativeHostSettingsAction::Close,
        );
        assert!(close.accepted);
        assert_eq!(close.result_name, "zsclip.settings.close");
    }

    #[test]
    fn macos_native_settings_route_actions_report_shared_matrix_status() {
        let sync = dispatch_macos_native_settings_route_action("settings_sync", "sync_webdav_now");
        assert!(!sync.accepted);
        assert!(sync
            .result_name
            .starts_with("zsclip.settings_sync.webdav.failed."));

        let lan =
            dispatch_macos_native_settings_route_action("settings_sync", "refresh_lan_devices");
        assert!(lan.accepted);
        assert_eq!(
            lan.result_name,
            "zsclip.settings_sync.refresh_lan_devices.device_book_projected_0_on_macos_native_host"
        );

        let lan_pair =
            dispatch_macos_native_settings_route_action("settings_sync", "pair_lan_device");
        assert!(!lan_pair.accepted);
        assert_eq!(
            lan_pair.result_name,
            "zsclip.settings_sync.pair_lan_device.lan_disabled_on_macos_native_host"
        );

        let lan_pair_link =
            dispatch_macos_native_settings_route_action("settings_sync", "copy_lan_pair_url");
        assert!(!lan_pair_link.accepted);
        assert_eq!(
            lan_pair_link.result_name,
            "zsclip.settings_sync.copy_lan_pair_url.lan_disabled_on_macos_native_host"
        );

        let accept_pair =
            dispatch_macos_native_settings_route_action("settings_sync", "accept_lan_pairing");
        assert!(!accept_pair.accepted);
        assert_eq!(
            accept_pair.result_name,
            "zsclip.settings_sync.accept_lan_pairing.no_pending_pair_on_macos_native_host"
        );

        let reject_pair =
            dispatch_macos_native_settings_route_action("settings_sync", "reject_lan_pairing");
        assert!(!reject_pair.accepted);
        assert_eq!(
            reject_pair.result_name,
            "zsclip.settings_sync.reject_lan_pairing.no_pending_pair_on_macos_native_host"
        );

        let missing = dispatch_macos_native_settings_route_action("settings_sync", "missing");
        assert!(!missing.accepted);
        assert_eq!(
            missing.result_name,
            "zsclip.settings.unknown_route.settings_sync.missing"
        );
    }

    #[test]
    fn macos_appkit_settings_save_collects_all_native_bindings() {
        let source = include_str!("macos_native_host.rs").replace("\r\n", "\n");

        assert!(source.contains("settings_native_text_fields"));
        assert!(source.contains("settings_native_toggle_buttons"));
        assert!(source.contains("settings_native_dropdown_buttons"));
        assert!(source.contains("NativeSettingsTextFieldBinding"));
        assert!(source.contains("NativeSettingsToggleButtonBinding"));
        assert!(source.contains("NativeSettingsDropdownButtonBinding"));
        assert!(source.contains("initial_value"));
        assert!(source.contains("NSSize::new(1160.0, 760.0)"));
        assert!(source.contains("NSButtonType::Switch"));
        assert!(source.contains("NSPopUpButton"));
        assert!(source.contains("settings_native_dropdown_options"));
        assert!(source.contains("settings_native_vv_group_dropdown_options"));
        assert!(source.contains("settings_native_vv_source_tab(settings_json)"));
        assert!(source.contains("crate::db_runtime::native_clip_groups(category)"));
        assert!(source.contains("popup.addItemWithTitle(&title)"));
        assert!(source.contains("popup.selectItemAtIndex(options.selected_index as _)"));
        assert!(source.contains(".take(8)"));
        assert!(source.contains(".take(12)"));
        assert!(source.contains("NSPoint::new(430.0, 608.0)"));
        assert!(source.contains("binding.button.state() == NSControlStateValueOn"));
        assert!(source.contains("binding.button.indexOfSelectedItem()"));
        assert!(source.contains("binding.field.stringValue().to_string()"));
        assert!(!source.contains("raw_value == binding.initial_value"));
        assert!(!source.contains("value == binding.initial_value"));
        assert!(source.contains("SettingsNativeSubmittedControlValue"));
        assert!(source.contains("settings_native_collect_submission(&submitted_values)"));
        assert!(!source.contains("settings_native_collect_submission(&[])"));
        assert!(source.contains("refresh_main_state_after_settings_save()"));
        assert!(include_str!("../Cargo.toml").contains("\"NSPopUpButton\""));
    }

    #[test]
    fn macos_native_settings_save_routes_auto_start_to_autostart_host() {
        let source = include_str!("macos_app.rs").replace("\r\n", "\n");

        assert!(
            source.contains("macos_apply_autostart_from_settings_updates(&applied.field_updates)")
        );
        assert!(source.contains("settings_native_bool_field_update(updates, \"auto_start\")"));
        assert!(source.contains("zsclip.settings.native_save.updates_{}.rejected_{}.autostart_{}"));

        if !cfg!(target_os = "macos") {
            let updates = [crate::settings_model::SettingsNativeJsonFieldUpdate {
                field_name: "auto_start".to_string(),
                value: serde_json::Value::Bool(true),
            }];
            let applied = macos_apply_autostart_from_settings_updates(&updates).unwrap();

            assert!(applied.applied);
            assert!(applied.status.enabled);
            assert_eq!(
                applied.status.registration_path.as_deref(),
                Some("macos_autostart_scaffold")
            );
        }
    }

    #[test]
    fn macos_native_settings_save_persists_submission_to_json_file() {
        let _guard = macos_settings_file_test_guard();
        let path = native_settings_temp_file("macos-settings-save");
        set_macos_native_settings_file_for_tests(Some(path.clone()));

        let submission = crate::settings_model::settings_native_collect_submission(&[
            crate::settings_model::SettingsNativeSubmittedControlValue {
                control_key: "capture_enable".to_string(),
                raw_value: "false".to_string(),
            },
            crate::settings_model::SettingsNativeSubmittedControlValue {
                control_key: "max_items".to_string(),
                raw_value: "120".to_string(),
            },
        ]);
        let result = persist_macos_native_settings_submission(&submission);
        assert!(result.accepted, "{result:?}");
        assert!(result
            .result_name
            .starts_with("zsclip.settings.native_save.updates_2.rejected_0"));

        let snapshot = macos_native_settings_json_snapshot();
        assert_eq!(
            snapshot
                .get("clipboard_capture_enabled")
                .and_then(serde_json::Value::as_bool),
            Some(false)
        );
        assert_eq!(
            snapshot
                .get("max_items")
                .and_then(serde_json::Value::as_u64),
            Some(120)
        );

        set_macos_native_settings_file_for_tests(None);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn macos_native_clipboard_capture_enabled_reads_saved_setting() {
        let _guard = macos_settings_file_test_guard();
        let path = native_settings_temp_file("macos-capture-enabled");
        set_macos_native_settings_file_for_tests(Some(path.clone()));

        assert!(macos_native_clipboard_capture_enabled());
        std::fs::write(
            &path,
            serde_json::json!({ "clipboard_capture_enabled": false }).to_string(),
        )
        .unwrap();
        assert!(!macos_native_clipboard_capture_enabled());

        set_macos_native_settings_file_for_tests(None);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn macos_native_status_menu_action_state_reads_saved_settings() {
        let _guard = macos_settings_file_test_guard();
        let path = native_settings_temp_file("macos-status-action-state");
        set_macos_native_settings_file_for_tests(Some(path.clone()));

        assert_eq!(
            macos_native_status_menu_action_state(NativeHostStatusMenuAction::ToggleWindow),
            None
        );
        assert_eq!(
            macos_native_status_menu_action_state(
                NativeHostStatusMenuAction::ToggleClipboardCapture
            ),
            Some(true)
        );
        std::fs::write(
            &path,
            serde_json::json!({ "clipboard_capture_enabled": false }).to_string(),
        )
        .unwrap();
        assert_eq!(
            macos_native_status_menu_action_state(
                NativeHostStatusMenuAction::ToggleClipboardCapture
            ),
            Some(false)
        );

        set_macos_native_settings_file_for_tests(None);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn macos_native_grouping_enabled_reads_saved_setting() {
        let _guard = macos_settings_file_test_guard();
        let path = native_settings_temp_file("macos-grouping-enabled");
        set_macos_native_settings_file_for_tests(Some(path.clone()));

        assert!(macos_native_grouping_enabled());
        std::fs::write(
            &path,
            serde_json::json!({ "grouping_enabled": false }).to_string(),
        )
        .unwrap();
        assert!(!macos_native_grouping_enabled());

        set_macos_native_settings_file_for_tests(None);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn macos_native_settings_control_actions_enter_product_command_routes() {
        if !cfg!(target_os = "macos") {
            let autostart = dispatch_macos_native_settings_control_action(
                NativeHostSettingsControlAction::ToggleAutostart,
            );
            assert!(autostart.accepted);
            assert_eq!(
                autostart.result_name,
                "zsclip.settings.toggle_autostart_scaffold"
            );
        }

        let capture = dispatch_macos_native_settings_control_action(
            NativeHostSettingsControlAction::ToggleClipboardCapture,
        );
        assert!(capture.accepted);
        assert_eq!(capture.result_name, "zsclip.settings.toggle_control");

        #[cfg(feature = "lan-sync")]
        {
            let lan = crate::macos_native_host::dispatch_appkit_settings_control_action(
                NativeHostSettingsControlAction::ToggleLanSync,
            );
            assert!(lan.accepted);
            assert_eq!(lan.result_name, "zsclip.settings.toggle_control");
        }

        #[cfg(feature = "cloud-sync")]
        {
            let cloud = crate::macos_native_host::dispatch_appkit_settings_control_action(
                NativeHostSettingsControlAction::ToggleCloudSync,
            );
            assert!(cloud.accepted);
            assert_eq!(cloud.result_name, "zsclip.settings.toggle_control");
        }

        #[cfg(any(feature = "cloud-sync", feature = "lan-sync"))]
        {
            let dropdown = crate::macos_native_host::dispatch_appkit_settings_control_action(
                NativeHostSettingsControlAction::OpenSyncModeDropdown,
            );
            assert!(dropdown.accepted);
            assert_eq!(dropdown.result_name, "zsclip.settings.open_dropdown");
        }
    }

    #[test]
    fn macos_native_file_picker_is_target_only_and_callable() {
        if cfg!(target_os = "macos") {
            return;
        }

        let result = pick_macos_native_file(NativeFileDialogRequest {
            title: "Choose sound",
            filter_name: "Wave Files",
            filter_pattern: "*.wav",
            current_path: "/tmp/old.wav",
        });

        assert_eq!(
            result.unwrap_err(),
            "macOS native file picker is only available on macOS"
        );
    }

    #[test]
    fn macos_native_dialog_actions_enter_native_dialog_routes() {
        let info = dispatch_macos_native_dialog_action(NativeHostDialogAction::ShowInfoMessage);
        assert!(info.accepted);
        assert_eq!(info.result_name, "zsclip.dialog.show_info_message");

        let confirm = crate::macos_native_host::dispatch_appkit_dialog_action(
            NativeHostDialogAction::ConfirmQuestion,
        );
        assert!(confirm.accepted);
        assert_eq!(confirm.result_name, "zsclip.dialog.confirm_cancel");
    }

    #[test]
    fn macos_native_status_menu_actions_enter_product_command_routes() {
        let _guard = macos_settings_file_test_guard();
        let path = native_settings_temp_file("macos-status-menu-settings");
        set_macos_native_settings_file_for_tests(Some(path.clone()));

        let host_source = include_str!("macos_native_host.rs").replace("\r\n", "\n");
        assert!(host_source.contains("macos_native_status_menu_action_state(action)"));
        assert!(host_source.contains("item.setTag(action.menu_id() as _)"));
        assert!(host_source.contains("refresh_status_menu_action_state(action)"));
        assert!(host_source.contains("refresh_status_menu_state_from_settings()"));

        let toggle =
            dispatch_macos_native_status_menu_action(NativeHostStatusMenuAction::ToggleWindow);
        assert!(toggle.accepted);
        assert_eq!(toggle.result_name, "zsclip.tray.toggle_window");

        assert!(macos_native_clipboard_capture_enabled());
        let capture = crate::macos_native_host::dispatch_appkit_status_menu_action(
            NativeHostStatusMenuAction::ToggleClipboardCapture,
        );
        assert!(capture.accepted);
        assert_eq!(capture.result_name, "zsclip.tray.toggle_clipboard_capture");
        assert!(!macos_native_clipboard_capture_enabled());
        assert_eq!(
            macos_native_status_menu_action_state(
                NativeHostStatusMenuAction::ToggleClipboardCapture
            ),
            Some(false)
        );
        let capture = dispatch_macos_native_status_menu_action(
            NativeHostStatusMenuAction::ToggleClipboardCapture,
        );
        assert!(capture.accepted);
        assert_eq!(capture.result_name, "zsclip.tray.toggle_clipboard_capture");
        assert!(macos_native_clipboard_capture_enabled());
        assert_eq!(
            macos_native_status_menu_action_state(
                NativeHostStatusMenuAction::ToggleClipboardCapture
            ),
            Some(true)
        );

        #[cfg(feature = "lan-sync")]
        {
            assert_eq!(
                macos_native_settings_json_snapshot()
                    .get("lan_sync_enabled")
                    .and_then(serde_json::Value::as_bool),
                None
            );
            let lan = crate::macos_native_host::dispatch_appkit_status_menu_action(
                NativeHostStatusMenuAction::ToggleLanSync,
            );
            assert!(lan.accepted);
            assert_eq!(lan.result_name, "zsclip.tray.toggle_lan_sync");
            assert_eq!(
                macos_native_settings_json_snapshot()
                    .get("lan_sync_enabled")
                    .and_then(serde_json::Value::as_bool),
                Some(true)
            );
        }

        let exit = crate::macos_native_host::dispatch_appkit_status_menu_action(
            NativeHostStatusMenuAction::Exit,
        );
        assert!(exit.accepted);
        assert_eq!(exit.result_name, "zsclip.tray.exit");

        set_macos_native_settings_file_for_tests(None);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn macos_native_row_actions_enter_product_command_routes() {
        let paste = dispatch_macos_native_row_action(NativeHostRowAction::Paste);
        assert!(paste.accepted);
        assert_eq!(paste.result_name, "zsclip.row.paste");

        let copy = crate::macos_native_host::dispatch_appkit_row_action(NativeHostRowAction::Copy);
        assert!(copy.accepted);
        assert_eq!(copy.result_name, "zsclip.row.copy");

        let pin = crate::macos_native_host::dispatch_appkit_row_action(NativeHostRowAction::Pin);
        assert!(pin.accepted);
        assert_eq!(pin.result_name, "zsclip.row.toggle_pin");

        let phrase =
            crate::macos_native_host::dispatch_appkit_row_action(NativeHostRowAction::ToPhrase);
        assert!(phrase.accepted);
        assert_eq!(phrase.result_name, "zsclip.row.to_phrase");

        let delete =
            crate::macos_native_host::dispatch_appkit_row_action(NativeHostRowAction::Delete);
        assert!(delete.accepted);
        assert_eq!(delete.result_name, "zsclip.row.delete");

        let edit = crate::macos_native_host::dispatch_appkit_row_action(NativeHostRowAction::Edit);
        assert!(edit.accepted);
        assert_eq!(edit.result_name, "zsclip.row.edit");

        let open_path =
            crate::macos_native_host::dispatch_appkit_row_action(NativeHostRowAction::OpenPath);
        assert!(open_path.accepted);
        assert_eq!(open_path.result_name, "zsclip.row.open_path");

        #[cfg(feature = "ai-actions")]
        {
            let translate = crate::macos_native_host::dispatch_appkit_row_action(
                NativeHostRowAction::TextTranslate,
            );
            assert!(translate.accepted);
            assert_eq!(translate.result_name, "zsclip.row.text_translate");
        }
    }

    #[test]
    fn macos_native_row_actions_execute_real_item_payloads() -> rusqlite::Result<()> {
        crate::db_runtime::with_test_db(|| {
            let text_id = crate::db_runtime::with_db_mut(|conn| {
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, text_data, source_app) VALUES(0, 'text', 'phrase me', 'macos-row-phrase', 'phrase me full text', 'Notes')",
                    [],
                )?;
                Ok(conn.last_insert_rowid())
            })?;
            let phrase =
                dispatch_macos_native_row_action_for_item(NativeHostRowAction::ToPhrase, text_id);
            assert!(phrase.accepted);
            assert_eq!(phrase.result_name, "zsclip.row.to_phrase_db");
            let phrases = crate::db_runtime::native_clip_list_items(1, 10)?;
            assert_eq!(phrases.len(), 1);
            assert_eq!(phrases[0].kind, ClipKind::Phrase);
            assert_eq!(
                crate::db_runtime::item_text(phrases[0].id)?.as_deref(),
                Some("phrase me full text")
            );

            let file_id = crate::db_runtime::with_db_mut(|conn| {
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, text_data, file_paths, source_app) VALUES(0, 'files', '/tmp/zsclip-macos.txt', 'macos-row-file', '/tmp/zsclip-macos.txt', '/tmp/zsclip-macos.txt', 'Finder')",
                    [],
                )?;
                Ok(conn.last_insert_rowid())
            })?;
            let open =
                dispatch_macos_native_row_action_for_item(NativeHostRowAction::OpenPath, file_id);
            assert!(open.accepted);
            assert_eq!(open.result_name, "zsclip.row.open_path_native_1");

            #[cfg(feature = "ai-actions")]
            {
                let translate = dispatch_macos_native_row_action_for_item(
                    NativeHostRowAction::TextTranslate,
                    text_id,
                );
                assert!(translate.accepted);
                assert!(translate
                    .result_name
                    .starts_with("zsclip.row.text_translate_ready_"));
            }
            Ok(())
        })
    }

    #[test]
    fn macos_native_edit_text_save_updates_database_item() {
        crate::db_runtime::with_test_db(|| {
            let item_id = crate::db_runtime::with_db_mut(|conn| {
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, text_data, source_app) VALUES(0, 'text', 'old', 'macos-edit', 'old', 'test')",
                    [],
                )?;
                Ok(conn.last_insert_rowid())
            })?;

            let result = dispatch_macos_native_edit_text_save(item_id, "edited on macOS");
            assert!(result.accepted);
            assert_eq!(result.result_name, "zsclip.row.edit.save_db");
            assert_eq!(
                crate::db_runtime::item_text(item_id)?,
                Some("edited on macOS".to_string())
            );

            let missing = dispatch_macos_native_edit_text_save(item_id + 10_000, "missing");
            assert!(!missing.accepted);
            assert_eq!(missing.result_name, "zsclip.row.edit.save_missing");
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn macos_native_assign_group_updates_database_and_group_projection() {
        crate::db_runtime::with_test_db(|| {
            let item_id = crate::db_runtime::with_db_mut(|conn| {
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, text_data, source_app) VALUES(0, 'text', 'group me', 'macos-group', 'group me', 'test')",
                    [],
                )?;
                Ok(conn.last_insert_rowid())
            })?;
            let group = crate::db_runtime::create_native_clip_group(0, "macOS Group")?;

            let result = dispatch_macos_native_assign_group(item_id, group.id);
            assert!(result.accepted);
            assert_eq!(result.result_name, "zsclip.row.assign_group_db");

            let grouped = macos_native_host_projected_clip_items_for_group(group.id);
            assert_eq!(grouped.len(), 1);
            assert_eq!(grouped[0].id, item_id);
            let remove = dispatch_macos_native_remove_group(item_id);
            assert!(remove.accepted);
            assert_eq!(remove.result_name, "zsclip.row.remove_group_db");
            assert!(macos_native_host_projected_clip_items_for_group(group.id).is_empty());
            assert_eq!(
                dispatch_macos_native_group_filter(group.id).result_name,
                "zsclip.group_filter.select_db"
            );
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn macos_native_settings_group_management_updates_database() {
        crate::db_runtime::with_test_db(|| {
            let create = dispatch_macos_native_create_group(0, "macOS Managed");
            assert!(create.accepted);
            assert_eq!(create.result_name, "zsclip.settings.group.create_db");
            let group = crate::db_runtime::native_clip_groups(0)?
                .into_iter()
                .find(|group| group.name == "macOS Managed")
                .expect("created group should exist");

            let rename = dispatch_macos_native_rename_group(0, group.id, "macOS Renamed");
            assert!(rename.accepted);
            assert_eq!(rename.result_name, "zsclip.settings.group.rename_db");
            assert_eq!(
                crate::db_runtime::native_clip_groups(0)?[0].name,
                "macOS Renamed"
            );

            let second = crate::db_runtime::create_native_clip_group(0, "macOS Second")?;
            let moved = dispatch_macos_native_move_group(0, second.id, -1);
            assert!(moved.accepted);
            assert_eq!(moved.result_name, "zsclip.settings.group.move_db");
            assert_eq!(crate::db_runtime::native_clip_groups(0)?[0].id, second.id);

            let delete = dispatch_macos_native_delete_group(group.id);
            assert!(delete.accepted);
            assert_eq!(delete.result_name, "zsclip.settings.group.delete_db");
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn macos_native_popup_menu_ids_enter_product_command_routes() {
        let copy = dispatch_macos_native_menu_command_id(crate::app_core::menu_ids::ROW_COPY);
        assert!(copy.accepted);
        assert_eq!(copy.result_name, "zsclip.row.copy");

        let assign_group = crate::macos_native_host::dispatch_appkit_menu_command_id(
            crate::app_core::menu_ids::ROW_GROUP_BASE + 1,
        );
        assert!(assign_group.accepted);
        assert_eq!(assign_group.result_name, "zsclip.row.assign_group");
        let source = include_str!("macos_native_host.rs").replace("\r\n", "\n");
        assert!(source.contains("menu_id == menu_ids::ROW_GROUP_REMOVE"));
        assert!(source.contains("dispatch_macos_native_remove_group(item_id)"));
        assert!(source.contains("self.reload_native_clip_items()"));

        let filter_all = crate::macos_native_host::dispatch_appkit_menu_command_id(
            crate::app_core::menu_ids::GROUP_FILTER_ALL,
        );
        assert!(filter_all.accepted);
        assert_eq!(filter_all.result_name, "zsclip.group_filter.all");

        let filter_group = crate::macos_native_host::dispatch_appkit_menu_command_id(
            crate::app_core::menu_ids::GROUP_FILTER_BASE + 1,
        );
        assert!(filter_group.accepted);
        assert_eq!(filter_group.result_name, "zsclip.group_filter.select");
    }

    #[test]
    fn macos_native_vv_select_enters_product_event_bridge() {
        let result = dispatch_macos_native_vv_select_event(1);
        assert!(result.bridged);
        assert_eq!(result.event_name, "vv_select_requested");

        let appkit_result = crate::macos_native_host::dispatch_appkit_vv_select_event(2);
        assert!(appkit_result.bridged);
        assert_eq!(appkit_result.event_name, "vv_select_requested");
    }

    #[test]
    fn macos_native_host_rows_open_shared_popup_menu_on_right_click() {
        let host_source = include_str!("macos_native_host.rs").replace("\r\n", "\n");

        assert!(host_source.contains("install_row_context_event_monitor"));
        assert!(host_source.contains("NSEventMask::RightMouseDown"));
        assert!(host_source.contains("perform_native_row_context_event"));
        assert!(host_source.contains("table_view.rowAtPoint(table_location)"));
        assert!(host_source.contains("self.select_native_row(item.id)"));
        assert!(host_source.contains("self.present_native_row_popup_menu_at(location)"));
        assert!(host_source.contains("present_native_row_popup_menu_at"));
        assert!(host_source.contains("native_host_full_row_popup_menu_entries_for_groups"));
        assert!(host_source.contains("native_host_row_popup_menu_input_for_projection"));
        assert!(host_source.contains("macos_native_grouping_enabled()"));
        assert!(host_source.contains("self.ivars().selected_item_id.get()"));
        assert!(host_source.contains("let items = self.ivars().clip_items.borrow()"));
    }

    #[test]
    fn macos_native_vv_trigger_uses_shared_trigger_state() {
        let app_source = include_str!("macos_app.rs").replace("\r\n", "\n");
        let host_source = include_str!("macos_native_host.rs").replace("\r\n", "\n");

        assert!(app_source.contains("fn macos_native_vv_trigger_state()"));
        assert!(app_source.contains("NativeHostVvTriggerState::default()"));
        assert!(app_source.contains("dispatch_macos_native_vv_trigger_key"));
        assert!(app_source.contains("ApplicationEvent::VvShowRequested"));
        assert!(app_source.contains("ApplicationEvent::VvHideRequested"));
        assert!(host_source.contains("dispatch_appkit_vv_trigger_key"));
        assert!(host_source.contains("dispatch_macos_native_vv_trigger_key(input)"));
        assert!(host_source.contains("zsclipTriggerVvDemo"));
        assert!(host_source.contains("install_vv_local_event_monitor"));
        assert!(host_source.contains("install_vv_global_event_monitor"));
        assert!(host_source.contains("install_vv_cg_event_tap_monitor"));
        assert!(host_source.contains("addLocalMonitorForEventsMatchingMask_handler"));
        assert!(host_source.contains("addGlobalMonitorForEventsMatchingMask_handler"));
        assert!(host_source.contains("CGEvent::tap_create"));
        assert!(host_source.contains("CGEvent::tap_enable"));
        assert!(host_source.contains("CFMachPort::new_run_loop_source"));
        assert!(host_source.contains("CFRunLoopAddSource"));
        assert!(host_source.contains("NSEventMask::KeyDown"));
        assert!(host_source.contains("perform_native_vv_key_event"));
        assert!(host_source.contains("perform_native_vv_global_key_event"));
        assert!(host_source.contains("perform_native_vv_cg_event"));
        assert!(host_source.contains("appkit_vv_cg_event_tap_callback"));
        assert!(host_source.contains("charactersIgnoringModifiers"));
        assert!(host_source.contains("appkit_vv_trigger_key_from_event"));
        assert!(host_source.contains("appkit_vv_trigger_key_from_cg_event"));
        assert!(host_source.contains("appkit_post_native_key_event"));
        assert!(host_source.contains("appkit_post_native_paste_shortcut"));
        assert!(host_source.contains("appkit_post_native_delete_backspaces"));
        assert!(host_source.contains("CGEvent::new_keyboard_event"));
        assert!(host_source.contains("CGEvent::set_flags"));
        assert!(host_source.contains("CGEvent::post(CGEventTapLocation::HIDEventTap"));
        assert!(host_source.contains("CGEventFlags::MaskCommand"));
        assert!(host_source.contains("paste.paste_shortcut_sent"));
        assert!(host_source.contains("ZSClip AppKit VV native paste shortcut posted="));
        assert!(host_source.contains("ZSClip AppKit VV delete backspaces requested="));
        assert!(host_source.contains("appkit_vv_target_token_for_event"));
        assert!(host_source.contains("appkit_vv_target_token_for_cg_event"));
        assert!(host_source.contains("appkit_vv_has_command_modifier"));
        assert!(host_source.contains("appkit_vv_has_cg_command_modifier"));
        assert!(host_source.contains("VV global monitor cannot consume external key"));
        assert!(host_source.contains("transition.consume_key"));
        assert!(host_source.contains("ptr::null_mut()"));
        assert!(host_source.contains("perform_native_vv_trigger_demo"));
        assert!(host_source.contains("perform_native_vv_key_text"));
        assert!(host_source.contains("appkit_vv_trigger_key_from_text"));
        assert!(host_source.contains("self.perform_native_vv_key_text(\"v\""));
        assert!(host_source.contains("handle_native_vv_trigger_transition"));
        assert!(host_source.contains("NativeHostVvTriggerAction::Show"));
        assert!(host_source.contains("NativeHostVvTriggerAction::Select"));
        assert!(host_source.contains("macos_native_host_projected_clip_items()"));
        assert!(host_source.contains("native_host_vv_popup_render_plan_for_projection"));
    }

    #[test]
    fn macos_native_host_applies_first_pass_ui_polish() {
        let host_source = include_str!("macos_native_host.rs").replace("\r\n", "\n");

        assert!(host_source
            .contains("search_field.setPlaceholderString(Some(ns_string!(\"Search clipboard\")))"));
        assert!(host_source.contains("window.makeFirstResponder(Some(search_field))"));
        assert!(host_source.contains("search_field.setStringValue(ns_string!(\"\"))"));
        assert!(host_source.contains("self.update_clip_list_visibility(\"\")"));
        assert!(host_source.contains("tableView:viewForTableColumn:row:"));
        assert!(host_source.contains("native_host_clip_row_presentation_for_projection(&item)"));
        assert!(host_source.contains("NSImage::imageWithSystemSymbolName_accessibilityDescription"));
        assert!(host_source.contains("doc.on.clipboard"));
        assert!(host_source.contains("image.setTemplate(true)"));
        assert!(host_source.contains("NSWindowStyleMask::Borderless"));
        assert!(host_source.contains("window.setHasShadow(true)"));
        assert!(host_source.contains("NSColor::clearColor()"));
        assert!(host_source.contains("NSWindowStyleMask::FullSizeContentView"));
        assert!(host_source.contains("window.setTitleVisibility(NSWindowTitleVisibility::Hidden)"));
        assert!(host_source.contains("window.setTitlebarAppearsTransparent(true)"));
        assert!(host_source.contains("window.setHidesOnDeactivate(false)"));
        assert!(host_source.contains("applicationShouldTerminateAfterLastWindowClosed:"));
        assert!(host_source.contains("sender.orderOut(None)"));
        assert!(host_source.contains("install_clipboard_capture_timer"));
        assert!(host_source.contains("NativeClipboardCaptureService::capture_current"));
        assert!(host_source.contains("macos_native_clipboard_capture_enabled()"));
        assert!(host_source.contains("scheduledTimerWithTimeInterval"));
        assert!(!host_source.contains("native_host_main_action_button_specs()"));
        assert!(!host_source.contains("native_host_row_action_button_specs()"));
        assert!(host_source.contains("window.setLevel(NSFloatingWindowLevel)"));
        assert!(host_source.contains("setMovableByWindowBackground: true"));
        assert!(host_source.contains("fn appkit_enable_rounded_layer("));
        assert!(host_source.contains("setWantsLayer: true"));
        assert!(host_source.contains("setCornerRadius: radius"));
        assert!(host_source.contains("appkit_enable_rounded_layer(view.as_ref(), 12.0)"));
        assert!(host_source.contains("window.backingScaleFactor()"));
        assert!(host_source.contains("fn appkit_is_dark_appearance(app: &NSApplication)"));
        assert!(host_source.contains("app.effectiveAppearance().name()"));
        assert!(host_source.contains("NSAppearanceNameDarkAqua"));
        assert!(host_source.contains("NSEvent::mouseLocation()"));
        assert!(host_source.contains("window.setFrameOrigin(NSPoint::new"));
        assert!(host_source.contains("appkit_position_window_near_cursor(&window)"));
        assert!(host_source.contains("NSVisualEffectView::initWithFrame"));
        assert!(host_source.contains("NSVisualEffectMaterial::WindowBackground"));
        assert!(host_source.contains("NSVisualEffectBlendingMode::BehindWindow"));
        assert!(host_source.contains("NSVisualEffectState::FollowsWindowActiveState"));
        assert!(host_source.contains("window.setContentView(Some(&view))"));
        assert!(host_source.contains("fn appkit_set_accessibility_label<T>"));
        assert!(host_source.contains("element.setAccessibilityLabel(Some(&label))"));
        assert!(host_source.contains("appkit_set_accessibility_label::<NSSearchField>"));
        assert!(host_source.contains("\"Clipboard history list\""));
        assert!(host_source.contains("\"ZSClip status menu\""));
        assert!(host_source.contains("if spec.starts_section"));
        assert!(host_source.contains("fn appkit_status_menu_symbol_name("));
        assert!(host_source.contains("appkit_status_menu_symbol_name(spec.icon_name)"));
        assert!(host_source.contains("NSString::from_str(spec.accelerator_key)"));
        assert!(host_source.contains("appkit_set_menu_item_command_modifier(item.as_ref())"));
        assert!(host_source.contains("item.setImage(Some(&image))"));
        assert!(host_source.contains("native_popup_menu_command_macos_symbol_name(*id)"));
        assert!(host_source.contains("native_popup_menu_command_macos_key_equivalent(*id)"));
        assert!(host_source.contains("\"ZSClip main window content\""));
        assert!(host_source.contains("fn appkit_set_view_alpha_animated("));
        assert!(host_source.contains("appkit_set_view_alpha(search_field.as_ref(), 0.0)"));
        assert!(host_source.contains("appkit_set_view_alpha_animated(search_field.as_ref(), 1.0)"));
        assert!(host_source.contains("appkit_set_view_alpha_animated(search_field.as_ref(), 0.0)"));
        assert!(host_source.contains("fn appkit_vv_popup_text_font("));
        assert!(host_source.contains("MainVvPopupTextRole::RowPreview"));
        assert!(host_source.contains("NSFont::monospacedSystemFontOfSize_weight"));
        assert!(host_source.contains("MainVvPopupTextRole::RowIndex"));
        assert!(host_source.contains("fn dismiss_native_vv_popup_for_local_mouse_event("));
        assert!(host_source.contains("NSEventMask::LeftMouseDown"));
        assert!(host_source.contains("NSEventMask::RightMouseDown"));
        assert!(host_source.contains("NSEventMask::OtherMouseDown"));
        assert!(host_source.contains("dismiss_native_vv_popup(\"global_mouse_down\")"));
        assert!(host_source.contains("native_host_group_filter_label_for_groups"));
        assert!(host_source.contains("macos_native_host_projected_clip_items_for_group"));
        assert!(host_source.contains("dispatch_appkit_vv_paste_for_group"));
        assert!(host_source.contains("clip_scroll_view: OnceCell<Retained<NSScrollView>>"));
        assert!(host_source.contains("clip_table_view: OnceCell<Retained<NSTableView>>"));
        assert!(host_source.contains("clip_table_column: OnceCell<Retained<NSTableColumn>>"));
        assert!(host_source
            .contains("clip_table_items: RefCell<Vec<NativeHostClipListItemProjection>>"));
        assert!(host_source.contains("clip_list_document_view: OnceCell<Retained<NSView>>"));
        assert!(host_source.contains("NSScrollView::initWithFrame"));
        assert!(host_source.contains("NSTableView::initWithFrame"));
        assert!(host_source.contains("NSTableColumn::new"));
        assert!(host_source.contains("clip_table_view.addTableColumn(&clip_table_column)"));
        assert!(host_source.contains("clip_table_view.setHeaderView(None)"));
        assert!(host_source.contains("clip_table_view.setRowHeight(clip_row_height)"));
        assert!(host_source.contains("clip_table_view.setUsesAlternatingRowBackgroundColors(true)"));
        assert!(host_source.contains("clip_table_view.setAllowsMultipleSelection(false)"));
        assert!(host_source.contains("clip_table_view.setAllowsEmptySelection(false)"));
        assert!(host_source.contains("NSTableViewSelectionHighlightStyle::Regular"));
        assert!(host_source.contains("NSTableViewStyle::Plain"));
        assert!(host_source.contains("clip_table_view.setTarget(Some(target))"));
        assert!(host_source
            .contains("clip_table_view.setDoubleAction(Some(sel!(zsclipActivateClipTableRow:)))"));
        assert!(
            host_source.contains("fn zsclip_activate_clip_table_row(&self, _sender: &AnyObject)")
        );
        assert!(host_source
            .contains("clip_table_view.setDataSource(Some(ProtocolObject::from_ref(self)))"));
        assert!(host_source
            .contains("clip_table_view.setDelegate(Some(ProtocolObject::from_ref(self)))"));
        assert!(host_source.contains("edit_text_view: OnceCell<Retained<NSTextView>>"));
        assert!(host_source.contains("NSTextView::initWithFrame"));
        assert!(host_source.contains("edit_text_view.setString(&initial_text)"));
        assert!(host_source.contains("edit_text_view.setEditable(true)"));
        assert!(host_source.contains("edit_text_view.setRichText(false)"));
        assert!(host_source.contains("edit_text_view.setAllowsUndo(true)"));
        assert!(host_source.contains("\"Clipboard text editor\""));
        assert!(host_source.contains("edit_text_scroller.setDocumentView(Some(&edit_text_view))"));
        assert!(host_source.contains("parent.beginSheet_completionHandler(&window, None)"));
        assert!(host_source.contains("parent.endSheet(window)"));
        assert!(host_source.contains("edit_text_view.string().to_string()"));
        assert!(
            host_source.contains("NativeHostEditTextAction::Cancel => sel!(zsclipCancelEditText:)")
        );
        assert!(host_source.contains("fn perform_native_edit_cancel(&self)"));
        assert!(host_source.contains("ZSClip AppKit edit cancel"));
        assert!(host_source.contains("windowShouldClose:"));
        assert!(
            host_source.contains("native_host_edit_text_close_plan(&initial_text, &current_text)")
        );
        assert!(host_source.contains("fn present_native_edit_unsaved_changes_alert(&self)"));
        assert!(host_source.contains("Save edited clipboard text?"));
        assert!(host_source.contains("alert.addButtonWithTitle(ns_string!(\"Discard\"))"));
        assert!(host_source.contains("fn appkit_settings_scroll_tab_item("));
        assert!(host_source.contains("NSTabView::initWithFrame"));
        assert!(host_source.contains("settings_tab_view.setTabViewType"));
        assert!(host_source.contains("NSTabViewType::TopTabsBezelBorder"));
        assert!(host_source.contains("for spec in native_host_settings_page_tab_specs()"));
        assert!(host_source.contains("appkit_settings_scroll_tab_item(mtm, spec.label)"));
        assert!(host_source.contains("NativeSettingsPageTabKind::General"));
        assert!(host_source.contains("NativeSettingsPageTabKind::Groups"));
        assert!(host_source.contains("NativeSettingsPageTabKind::Actions"));
        assert!(host_source.contains("native_host_settings_section_label(\"settings_controls\")"));
        assert!(host_source.contains("native_host_settings_section_label(\"group_selector\")"));
        assert!(host_source.contains("native_host_settings_toggle_specs()"));
        assert!(host_source.contains("native_host_settings_dropdown_specs()"));
        assert!(host_source.contains("fn appkit_switch_from_spec<Spec>"));
        assert!(host_source.contains("button.setButtonType(NSButtonType::Switch)"));
        assert!(host_source.contains("fn appkit_dropdown_from_spec("));
        assert!(host_source.contains("NSPopUpButton::initWithFrame_pullsDown"));
        assert!(host_source.contains("if spec.options.is_empty()"));
        assert!(host_source.contains("for option in spec.options"));
        assert!(host_source.contains("let title = NSString::from_str(option.label)"));
        assert!(host_source.contains("popup.addItemWithTitle(&title)"));
        assert!(host_source.contains("appkit_set_accessibility_label::<NSPopUpButton>"));
        assert!(host_source.contains("settings_tab_view.addTabViewItem(&tab_item)"));
        assert!(host_source.contains("scroller.setDocumentView(Some(&content))"));
        assert!(
            host_source.contains("let scroller_label = format!(\"{label} settings scroll area\")")
        );
        assert!(host_source.contains("appkit_set_accessibility_label::<NSScrollView>"));
        assert!(host_source.contains("let view = groups_content"));
        assert!(host_source.contains("let view = actions_content"));
        assert!(host_source.contains("unsafe impl NSTableViewDataSource for Delegate"));
        assert!(host_source
            .contains("fn numberOfRowsInTableView(&self, _table_view: &NSTableView) -> NSInteger"));
        assert!(host_source.contains("unsafe impl NSTableViewDelegate for Delegate"));
        assert!(host_source.contains("fn tableView_viewForTableColumn_row"));
        assert!(host_source.contains("native_host_clip_row_presentation_for_projection(&item)"));
        assert!(host_source.contains("Retained::autorelease_return(appkit_clip_table_cell_view("));
        assert!(host_source.contains("fn appkit_clip_table_cell_view"));
        assert!(host_source.contains("fn appkit_clip_table_label"));
        assert!(host_source.contains("&presentation.accessibility_label"));
        assert!(host_source.contains("appkit_set_accessibility_label::<NSTextField>"));
        assert!(host_source.contains("presentation.kind_prefix"));
        assert!(host_source.contains("presentation.kind_icon"));
        assert!(host_source.contains("presentation.pin_badge"));
        assert!(host_source.contains("NSLineBreakMode::ByTruncatingTail"));
        assert!(host_source.contains("label.setMaximumNumberOfLines(1)"));
        assert!(host_source.contains("NSColor::controlAccentColor()"));
        assert!(host_source
            .contains("fn tableViewSelectionDidChange(&self, _notification: &NSNotification)"));
        assert!(host_source.contains("NSView::initWithFrame"));
        assert!(host_source.contains("clip_scroll_view.setHasVerticalScroller(true)"));
        assert!(host_source.contains("clip_scroll_view.setAutohidesScrollers(true)"));
        assert!(host_source.contains("clip_scroll_view.setDocumentView(Some(&clip_table_view))"));
        assert!(host_source.contains("table_view.reloadData()"));
        assert!(host_source.contains("table_view.rowAtPoint(table_location)"));
        assert!(host_source.contains("NSIndexSet::indexSetWithIndex(index as NSUInteger)"));
        assert!(host_source
            .contains("table_view.selectRowIndexes_byExtendingSelection(&indexes, false)"));
        assert!(host_source.contains("view.addSubview(&clip_scroll_view)"));
        assert!(host_source.contains("let clip_row_height = 44.0_f64"));
        assert!(host_source.contains("let clip_list_height = 300.0_f64"));
        assert!(!host_source.contains("row.setButtonType(NSButtonType::Toggle)"));
        assert!(host_source.contains("fn refresh_native_clip_row_selection(&self)"));
        assert!(host_source
            .contains("fn perform_native_clip_list_key_event(&self, event: &NSEvent) -> bool"));
        assert!(host_source.contains("fn focus_native_search_field(&self)"));
        assert!(host_source.contains("fn hide_native_search_field(&self) -> bool"));
        assert!(host_source.contains("appkit_event_has_command_modifier(event.modifierFlags())"));
        assert!(host_source.contains("appkit_event_key_text(event).eq_ignore_ascii_case(\"f\")"));
        assert!(host_source.contains("event.keyCode() == 53 && self.hide_native_search_field()"));
        assert!(host_source.contains("window.makeFirstResponder(Some(&clip_table_view))"));
        assert!(host_source.contains("window.makeFirstResponder(Some(table_view))"));
        assert!(host_source.contains("table_view.scrollRowToVisible(index as NSInteger)"));
        assert!(host_source.contains("self.perform_native_row_action(NativeHostRowAction::Paste)"));
        assert!(host_source.contains("125 => self.move_native_clip_row_selection(1)"));
        assert!(host_source.contains("126 => self.move_native_clip_row_selection(-1)"));
        assert!(host_source.contains("fn visible_native_clip_row_item_ids(&self) -> Vec<i64>"));
        let cargo_source = include_str!("../Cargo.toml").replace("\r\n", "\n");
        assert!(cargo_source.contains("\"NSAccessibilityProtocols\""));
        assert!(cargo_source.contains("\"NSAppearance\""));
        assert!(cargo_source.contains("\"NSScrollView\""));
        assert!(cargo_source.contains("\"NSTabView\""));
        assert!(cargo_source.contains("\"NSTabViewItem\""));
        assert!(cargo_source.contains("\"NSTableView\""));
        assert!(cargo_source.contains("\"NSTableColumn\""));
        assert!(cargo_source.contains("\"NSTableHeaderView\""));
        assert!(cargo_source.contains("\"NSControl\""));
        assert!(cargo_source.contains("\"NSView\""));
        assert!(cargo_source.contains("\"NSVisualEffectView\""));
    }

    #[test]
    fn macos_native_vv_paste_writes_clipboard_and_targets_text_input() {
        let _guard = macos_clipboard_test_guard();
        crate::db_runtime::with_test_db(|| {
            MacosClipboardHost::reset_for_tests();
            crate::db_runtime::with_db_mut(|conn| {
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, text_data, source_app) VALUES(0, 'text', 'mac vv', 'mac-vv', 'mac vv full text', 'Notes')",
                    [],
                )?;
                Ok(())
            })?;

            let result = dispatch_macos_native_vv_paste(0);
            assert!(result.accepted);
            assert_eq!(result.result_name, "zsclip.vv_paste.clipboard_target");
            assert_eq!(result.clipboard_kind, Some("text"));
            assert!(result.focused_target);
            assert!(result.direct_text_set);
            assert!(result.paste_shortcut_sent);
            assert_eq!(
                MacosClipboardHost::read_text(),
                Some("mac vv full text".to_string())
            );
            let appkit_result = crate::macos_native_host::dispatch_appkit_vv_paste(0);
            assert!(appkit_result.accepted);
            assert_eq!(
                appkit_result.result_name,
                "zsclip.vv_paste.clipboard_target"
            );
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn macos_native_vv_paste_for_group_uses_group_projection() {
        let _guard = macos_clipboard_test_guard();
        crate::db_runtime::with_test_db(|| {
            MacosClipboardHost::reset_for_tests();
            let (first_id, second_id) = crate::db_runtime::with_db_mut(|conn| {
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, text_data, source_app) VALUES(0, 'text', 'all only', 'mac-vv-all', 'all only text', 'Notes')",
                    [],
                )?;
                let first_id = conn.last_insert_rowid();
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, text_data, source_app) VALUES(0, 'text', 'group only', 'mac-vv-group', 'group only text', 'Notes')",
                    [],
                )?;
                Ok((first_id, conn.last_insert_rowid()))
            })?;
            let group = crate::db_runtime::create_native_clip_group(0, "VV Group")?;
            assert!(dispatch_macos_native_assign_group(second_id, group.id).accepted);
            assert_ne!(first_id, second_id);

            let result = dispatch_macos_native_vv_paste_for_group(0, group.id);
            assert!(result.accepted);
            assert_eq!(result.result_name, "zsclip.vv_paste.clipboard_target");
            assert_eq!(
                MacosClipboardHost::read_text(),
                Some("group only text".to_string())
            );
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn macos_native_search_text_enters_product_command_route() {
        let result =
            dispatch_macos_native_search_text_action(NativeHostSearchTextAction::new("needle"));
        assert!(result.accepted);
        assert_eq!(result.result_name, "zsclip.window.search_text_update");

        let appkit_result = crate::macos_native_host::dispatch_appkit_search_text_action(
            NativeHostSearchTextAction::new("from appkit"),
        );
        assert!(appkit_result.accepted);
        assert_eq!(
            appkit_result.result_name,
            "zsclip.window.search_text_update"
        );
    }

    #[test]
    fn macos_native_host_projected_clip_items_prefer_database_and_fallback_to_product_adapter() {
        crate::db_runtime::with_test_db(|| {
            let fallback = macos_native_host_projected_clip_items();
            assert_eq!(fallback.len(), 4);
            assert_eq!(fallback[0].title, "Welcome text");

            crate::db_runtime::with_db_mut(|conn| {
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, text_data, source_app) VALUES(0, 'text', 'real macOS history', 'macos-history', 'real macOS history', 'Safari')",
                    [],
                )?;
                Ok(())
            })?;

            let items = macos_native_host_projected_clip_items();
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].title, "Safari");
            assert_eq!(items[0].preview, "real macOS history");
            assert_eq!(items[0].kind, crate::app_core::ClipKind::Text);
            assert!(!items[0].pinned);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn macos_host_scaffold_tracks_current_core_contract() {
        let summary = MacosUiHost::contract_summary();

        assert_eq!(summary.api_major, 0);
        assert_eq!(summary.api_minor, 78);
        assert_eq!(summary.surfaces, 5);
        assert_eq!(summary.main_execution_plans, 5);
        assert_eq!(summary.native_style_operations, 1);
        assert_eq!(summary.native_control_mapper_operations, 1);
        assert_eq!(summary.text_layout_operations, 2);
        assert_eq!(summary.renderer_operations, 5);
        assert_eq!(summary.settings_control_operations, 11);
        assert_eq!(summary.status_item_operations, 3);
        assert_eq!(summary.native_popup_menu_operations, 1);
        assert_eq!(summary.native_ime_operations, 3);
        assert_eq!(summary.native_text_caret_operations, 5);
        assert_eq!(summary.native_dialog_operations, 2);
        assert_eq!(summary.native_shell_open_operations, 1);
        assert_eq!(summary.native_window_identity_operations, 7);
        assert_eq!(summary.native_paste_target_operations, 7);
        assert_eq!(summary.native_file_dialog_operations, 1);
        assert_eq!(summary.native_text_input_dialog_operations, 1);
        assert_eq!(summary.native_edit_text_dialog_operations, 1);
        assert_eq!(summary.native_mail_merge_window_operations, 1);
        assert_eq!(summary.native_main_search_control_operations, 8);
        assert_eq!(summary.native_main_window_operations, 21);
        assert_eq!(summary.native_settings_window_operations, 13);
        assert_eq!(summary.native_settings_dropdown_operations, 3);
        assert_eq!(summary.native_transient_window_operations, 4);
        assert_eq!(summary.shared_non_host_protocols, 3);
    }

    #[test]
    fn macos_renderer_and_text_layout_consume_shared_rendering_contracts() {
        let mut application = MacosApplicationModel::default();
        let text_style = TextStyle::line(
            "SF Pro",
            14.0,
            Color {
                r: 20,
                g: 24,
                b: 30,
                a: 255,
            },
        );
        let measured = application.text_layout().measure("hello", &text_style, 200);
        assert_eq!(measured.height, 20);
        assert!(measured.width > 0);

        let runs = application.text_layout().layout_runs(
            "hello",
            &text_style,
            Rect {
                x: 0,
                y: 0,
                width: 100,
                height: 20,
            },
        );
        assert_eq!(runs.len(), 1);
        assert_eq!(application.text_layout().actions().len(), 2);

        let rect = Rect {
            x: 1,
            y: 2,
            width: 30,
            height: 40,
        };
        let color = Color {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        };
        application.renderer().fill_rect(rect, color);
        application.renderer().stroke_rect(rect, color, 2);
        application.renderer().draw_text(&runs[0], &text_style);
        application.renderer().push_clip(rect);
        application.renderer().pop_clip();

        assert_eq!(application.renderer().commands().len(), 5);
        assert_eq!(
            application.renderer().commands()[0],
            MacosRenderCommand::FillRect(rect, color)
        );
        assert_eq!(
            application.renderer().commands()[4],
            MacosRenderCommand::PopClip
        );
    }

    #[test]
    fn macos_host_consumes_shared_ui_surface_protocol() {
        use crate::app_core::ui_surface_protocol::{UiHostSurface, REQUIRED_UI_HOST_SURFACES};

        assert_eq!(
            REQUIRED_UI_HOST_SURFACES,
            [
                UiHostSurface::MainWindow,
                UiHostSurface::SettingsWindow,
                UiHostSurface::SettingsDropdown,
                UiHostSurface::InputDialog,
                UiHostSurface::EditDialog,
            ]
        );
        let adapters: Vec<_> = REQUIRED_UI_HOST_SURFACES
            .iter()
            .map(|surface| surface.adapter_name())
            .collect();
        assert_eq!(
            adapters,
            vec![
                "main_window_host_event_from_message",
                "settings_window_host_event_from_message",
                "dropdown_window_host_event_from_message",
                "input_dialog_host_event_from_message",
                "edit_dialog_host_event_from_message",
            ]
        );
    }

    #[test]
    fn macos_clipboard_host_exposes_basic_text_and_image_operations() {
        let _guard = macos_clipboard_test_guard();
        let source = include_str!("macos_app.rs");

        MacosClipboardHost::reset_for_tests();
        assert!(source.contains("impl ClipboardHost for MacosClipboardHost"));
        assert!(source.contains("fn read_text()"));
        assert!(source.contains("fn write_text(text: &str)"));
        assert!(source.contains("fn read_image_rgba()"));
        assert!(source.contains("fn write_image_rgba("));
        assert!(source.contains("NSPasteboard::generalPasteboard()"));
        assert!(source.contains("pasteboard.writeObjects(&objects)"));
        assert!(source.contains("pasteboard.readObjectsForClasses_options"));
        assert!(source.contains("NSURL::from_file_path"));
        assert!(source.contains("url.to_file_path()"));
        assert!(source.contains("changeCount()"));

        assert_eq!(MacosClipboardHost::sequence_number(), 0);
        assert!(MacosClipboardHost::write_text("hello macOS clipboard"));
        assert_eq!(
            MacosClipboardHost::read_text().as_deref(),
            Some("hello macOS clipboard")
        );
        assert_eq!(MacosClipboardHost::sequence_number(), 1);

        let pixels = [255, 0, 0, 255, 0, 255, 0, 255];
        assert!(MacosClipboardHost::write_image_rgba(&pixels, 2, 1));
        assert_eq!(
            MacosClipboardHost::read_image_rgba(),
            Some((pixels.to_vec(), 2, 1))
        );
        assert_eq!(MacosClipboardHost::sequence_number(), 2);

        assert!(MacosClipboardHost::write_file_paths(&[
            "/tmp/a.txt".to_string(),
            "/tmp/b.txt".to_string()
        ]));
        assert_eq!(
            MacosClipboardHost::read_file_paths(),
            Some(vec!["/tmp/a.txt".to_string(), "/tmp/b.txt".to_string()])
        );
        assert_eq!(MacosClipboardHost::sequence_number(), 3);

        assert!(MacosClipboardHost::write_text_ignored_by_monitors("self"));
        assert!(MacosClipboardHost::should_ignore_capture_by_named_format());
        assert!(!MacosClipboardHost::should_ignore_capture_by_named_format());
    }

    #[test]
    fn macos_application_polls_clipboard_changes_into_events() {
        let _guard = macos_clipboard_test_guard();
        MacosClipboardHost::reset_for_tests();
        let mut application = MacosApplicationModel::default();

        assert_eq!(application.poll_application_event(), None);
        assert!(MacosClipboardHost::write_text("external macOS change"));
        assert_eq!(
            application.poll_application_event(),
            Some(ApplicationEvent::ClipboardChanged { sequence: 1 })
        );
        assert_eq!(application.poll_application_event(), None);

        assert!(MacosClipboardHost::write_text_ignored_by_monitors(
            "self macOS write"
        ));
        assert_eq!(application.poll_application_event(), None);

        application.set_clipboard_capture_enabled(false);
        assert!(MacosClipboardHost::write_text("disabled macOS change"));
        assert_eq!(application.poll_application_event(), None);
    }

    #[test]
    fn macos_main_window_consumes_shared_render_plan() {
        let model = MacosMainWindowModel::default();
        let plan = model.initial_render_plan(360, 720);

        assert!(!plan.chrome_commands.is_empty());
        assert!(plan.empty_state_rect.is_some());
        assert!(plan.visible_rows.is_empty());
        assert_eq!(plan.search_rect, None);

        let populated = model.render_plan(MainRenderInput {
            client_rect: UiRect::new(0, 0, 360, 720),
            visible_len: 30,
            scroll_y: 120,
            empty_state: crate::app_core::MainEmptyStateKind::Records,
            hover_idx: 3,
            sel_idx: 4,
            selected_rows: vec![2, 4],
            row_icon_kinds: vec![crate::app_core::MainIconKind::Text; 30],
            tab_index: 0,
            hover_tab: 0,
            hover_title_button: "search",
            down_title_button: "",
            search_on: true,
            active_loading: false,
            scroll_fade_alpha: 180,
            hover_scroll: true,
            scroll_to_top_visible: true,
            hover_scroll_to_top: false,
            down_scroll_to_top: false,
            title_buttons: crate::app_core::TitleButtonVisibility::default(),
        });

        assert!(!populated.visible_rows.is_empty());
        assert!(populated.visible_rows.len() < 30);
        assert!(populated.empty_state_rect.is_none());
        assert!(populated.search_rect.is_some());
        assert!(populated.scrollbar_thumb_rect.is_some());
        assert!(populated.scroll_to_top_rect.is_some());
    }

    #[test]
    fn macos_main_window_consumes_shared_vv_popup_plans() {
        use crate::app_core::{main_vv_select_plan, ClipItem, ClipKind};

        let layout = MainVvPopupLayout::default();
        let strings = MainVvPopupRenderStrings {
            title: "VV Mode".to_string(),
            hint: "Press 1-9".to_string(),
            empty: "No records".to_string(),
        };
        let render_plan = layout.render_plan(
            UiRect::new(0, 0, layout.width, layout.height(2)),
            &strings,
            "Work",
            &[
                MainVvPopupRenderItem {
                    index: 1,
                    label: "first".to_string(),
                },
                MainVvPopupRenderItem {
                    index: 2,
                    label: "second".to_string(),
                },
            ],
        );

        assert_eq!(
            layout.hit_test(layout.group_rect().left + 1, layout.group_rect().top + 1, 2),
            MainVvPopupHit::Group
        );
        assert_eq!(
            layout.hit_test(24, layout.row_rect(1).top + 1, 2),
            MainVvPopupHit::Row(1)
        );
        assert!(render_plan.text_commands.iter().any(|command| command.role
            == MainVvPopupTextRole::GroupName
            && command.text == "Work"));
        assert!(render_plan
            .text_commands
            .iter()
            .any(|command| command.role == MainVvPopupTextRole::RowPreview));

        let item = ClipItem {
            id: 5,
            kind: ClipKind::Text,
            preview: "first".to_string(),
            text: Some("first".to_string()),
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
            main_vv_select_plan(true, 0, &[item.clone()], 2),
            Some(MainVvSelectPlan::Paste {
                item,
                backspaces: 2,
            })
        );
        assert_eq!(
            main_vv_select_plan(true, 3, &[], 0),
            Some(MainVvSelectPlan::HidePopup)
        );
    }

    #[test]
    fn macos_main_window_consumes_shared_input_and_shortcut_plans() {
        let model = MacosMainWindowModel::default();
        let close = model.layout.title_button_rect("close");
        let close_point = Point {
            x: close.left + 1,
            y: close.top + 1,
        };

        let target = model.pointer_down_target(
            close_point,
            30,
            0,
            TitleButtonVisibility::default(),
            false,
            false,
        );
        assert_eq!(target, MainPointerDownTarget::TitleButton("close"));

        let hover = model.pointer_move_transition(
            close_point,
            30,
            0,
            TitleButtonVisibility::default(),
            false,
            MainHoverTarget::default(),
        );
        assert_eq!(
            hover.hover.map(|transition| transition.next.title_button),
            Some("close")
        );

        let release = model.pointer_up_transition(close_point, 30, 0, "close", false, -1);
        assert_eq!(
            release.target,
            crate::app_core::MainPointerUpTarget::TitleButton {
                key: "close",
                activated: true,
            }
        );

        assert_eq!(
            model.shortcut_execution_plan(
                MainShortcutAction::Escape,
                Some(MainShortcutEscapePlan::HideWindow),
            ),
            MainShortcutExecutionPlan::WindowCommand(
                crate::app_core::MainWindowCommandIntent::HideWindow
            )
        );
    }

    #[test]
    fn macos_main_window_consumes_shared_window_behavior_protocol() {
        use crate::app_core::main_window_protocol::{
            main_search_visibility_plan, main_window_position_plan, MainSearchVisibilityAction,
            MainSearchVisibilityInput, MainSearchVisibilityRequest, MainWindowPositionInput,
            MainWindowPositionMode, MainWindowPositionPlan,
        };

        let position = main_window_position_plan(MainWindowPositionInput {
            mode: MainWindowPositionMode::Center,
            by_hotkey: false,
            cursor_x: 0,
            cursor_y: 0,
            mouse_dx: 0,
            mouse_dy: 0,
            fixed_x: 0,
            fixed_y: 0,
            last_x: -1,
            last_y: -1,
            bounds: UiRect::new(0, 0, 1440, 900),
            win_w: 360,
            win_h: 720,
        });
        assert_eq!(
            position,
            MainWindowPositionPlan {
                x: 540,
                y: 60,
                width: 360,
                height: 720,
            }
        );

        let search = main_search_visibility_plan(MainSearchVisibilityInput {
            request: MainSearchVisibilityRequest::Open,
            search_on: false,
            search_text_empty: true,
            persistent_search_box: false,
            main_window_noactivate: true,
            quick_window: false,
        });
        assert_eq!(search.action, MainSearchVisibilityAction::Open);
        assert!(search.activate_window);
    }

    #[test]
    fn macos_main_window_host_tracks_main_and_quick_creation_request() {
        let mut host = MacosMainWindowHost::default();

        let presentation = host.create_main_windows(NativeMainWindowRequest {
            title: "ZSClip".to_string(),
            size: crate::app_core::Size {
                width: 300,
                height: 614,
            },
            main_visible: true,
        });

        assert_eq!(
            presentation,
            NativeMainWindowPresentation::Created(NativeMainWindowHandles {
                main: MacosMainWindowHandle(1),
                quick: MacosMainWindowHandle(2),
            })
        );
        assert_eq!(host.requests().len(), 1);
        assert_eq!(host.requests()[0].title, "ZSClip");
        assert_eq!(host.requests()[0].size.width, 300);
        assert!(host.requests()[0].main_visible);

        host.apply_main_window_appearance(MacosMainWindowHandle(1));
        assert_eq!(host.appearances(), vec![MacosMainWindowHandle(1)]);
        host.set_main_window_app_icon(
            MacosMainWindowHandle(1),
            NativeAppIconResource {
                small: "app-small",
                big: "app-big",
            },
        );
        host.hide_main_window(MacosMainWindowHandle(1));
        host.present_main_window(
            MacosMainWindowHandle(1),
            NativeMainWindowPresentMode::ActivateAndFocus,
        );
        host.set_main_window_bounds(MacosMainWindowHandle(1), UiRect::new(10, 20, 310, 640));
        host.activate_main_window(MacosMainWindowHandle(1));
        host.foreground_main_window(MacosMainWindowHandle(1));
        host.restore_main_window(MacosMainWindowHandle(1));
        host.close_main_window(MacosMainWindowHandle(1));
        host.set_main_window_activation_policy(MacosMainWindowHandle(1), false);
        host.request_main_window_close(MacosMainWindowHandle(1));
        host.destroy_main_window(MacosMainWindowHandle(2));
        host.capture_main_pointer(MacosMainWindowHandle(1));
        host.release_main_pointer(MacosMainWindowHandle(1));
        host.begin_main_window_drag(MacosMainWindowHandle(1));
        assert!(host.track_main_pointer_leave(MacosMainWindowHandle(1)));
        assert!(host.request_main_window_area_repaint(
            MacosMainWindowHandle(1),
            Some(UiRect::new(20, 30, 120, 80)),
            false,
        ));
        assert_eq!(host.main_window_layout_dpi(MacosMainWindowHandle(1)), 96);
        assert_eq!(
            host.main_window_client_bounds(MacosMainWindowHandle(1)),
            Some(UiRect::new(0, 0, 300, 620))
        );
        assert_eq!(
            host.main_window_bounds(MacosMainWindowHandle(1)),
            Some(UiRect::new(10, 20, 310, 640))
        );
        assert_eq!(
            host.lifecycle_actions(),
            vec![
                MacosMainWindowLifecycleAction::SetAppIcon {
                    handle: MacosMainWindowHandle(1),
                    icon: NativeAppIconResource {
                        small: "app-small",
                        big: "app-big",
                    },
                },
                MacosMainWindowLifecycleAction::Hide(MacosMainWindowHandle(1)),
                MacosMainWindowLifecycleAction::Present {
                    handle: MacosMainWindowHandle(1),
                    mode: NativeMainWindowPresentMode::ActivateAndFocus,
                },
                MacosMainWindowLifecycleAction::Bounds {
                    handle: MacosMainWindowHandle(1),
                    bounds: UiRect::new(10, 20, 310, 640),
                },
                MacosMainWindowLifecycleAction::Activate(MacosMainWindowHandle(1)),
                MacosMainWindowLifecycleAction::Foreground(MacosMainWindowHandle(1)),
                MacosMainWindowLifecycleAction::Restore(MacosMainWindowHandle(1)),
                MacosMainWindowLifecycleAction::Close(MacosMainWindowHandle(1)),
                MacosMainWindowLifecycleAction::ActivationPolicy {
                    handle: MacosMainWindowHandle(1),
                    allow_activation: false,
                },
                MacosMainWindowLifecycleAction::RequestClose(MacosMainWindowHandle(1)),
                MacosMainWindowLifecycleAction::Destroy(MacosMainWindowHandle(2)),
                MacosMainWindowLifecycleAction::CapturePointer(MacosMainWindowHandle(1)),
                MacosMainWindowLifecycleAction::ReleasePointer(MacosMainWindowHandle(1)),
                MacosMainWindowLifecycleAction::BeginDrag(MacosMainWindowHandle(1)),
            ]
        );
        assert_eq!(
            host.pointer_leave_requests(),
            vec![MacosMainWindowHandle(1)]
        );
        assert_eq!(
            host.repaint_requests(),
            vec![(
                MacosMainWindowHandle(1),
                Some(UiRect::new(20, 30, 120, 80)),
                false,
            )]
        );
        assert_eq!(host.layout_dpi_queries(), vec![MacosMainWindowHandle(1)]);
        assert_eq!(host.client_bounds_queries(), vec![MacosMainWindowHandle(1)]);
        assert_eq!(host.window_bounds_queries(), vec![MacosMainWindowHandle(1)]);
    }

    #[test]
    fn macos_main_search_control_host_tracks_native_search_field_request() {
        let mut host = MacosMainSearchControlHost::default();

        let presentation = host.create_search_control(NativeMainSearchControlRequest {
            owner: MacosMainWindowHandle(1),
            id: 1001,
            bounds: UiRect::new(16, 20, 284, 48),
            visible: true,
        });

        assert_eq!(
            presentation,
            NativeMainSearchControlPresentation::Created(MacosMainSearchControlHandle(1))
        );
        assert_eq!(host.requests().len(), 1);
        assert_eq!(host.requests()[0].owner, MacosMainWindowHandle(1));
        assert_eq!(host.requests()[0].id, 1001);
        assert_eq!(host.requests()[0].bounds, UiRect::new(16, 20, 284, 48));
        assert!(host.requests()[0].visible);

        let handle = MacosMainSearchControlHandle(1);
        let style = host.apply_search_style(NativeMainSearchStyleRequest {
            handle,
            font_family: "System".to_string(),
            font_px: 14,
            previous_resource: None,
        });
        host.set_search_bounds(handle, UiRect::new(20, 24, 300, 54));
        host.set_search_visible(handle, false);
        host.set_search_text(handle, "abc");
        host.focus_search(handle);
        host.release_search_style_resource(MacosMainSearchStyleResource(1));

        assert_eq!(
            style,
            NativeMainSearchStylePresentation::Applied(Some(MacosMainSearchStyleResource(1)))
        );
        assert_eq!(host.style_requests().len(), 1);
        assert_eq!(host.style_requests()[0].font_family, "System");
        assert_eq!(
            host.last_bounds(),
            Some((handle, UiRect::new(20, 24, 300, 54)))
        );
        assert_eq!(host.visibility_changes(), vec![(handle, false)]);
        assert_eq!(host.search_text(handle), "abc");
        assert_eq!(host.focused(), Some(handle));
        assert_eq!(
            host.released_style_resources(),
            vec![MacosMainSearchStyleResource(1)]
        );
    }

    #[test]
    fn macos_main_window_consumes_shared_row_action_plans() {
        use crate::app_core::{
            main_row_current_item_action_plan, main_row_dialog_action_plan,
            main_row_external_action_plan, ClipItem, ClipKind, MainRowCurrentItemActionPlan,
            MainRowDialogActionPlan, MainRowExternalActionPlan, MainRowMenuAction,
        };

        let text_item = ClipItem {
            id: 7,
            kind: ClipKind::Text,
            preview: "hello".to_string(),
            text: Some("hello".to_string()),
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
        let image_item = ClipItem {
            id: 8,
            kind: ClipKind::Image,
            preview: "image".to_string(),
            text: None,
            source_app: String::new(),
            file_paths: None,
            image_bytes: Some(vec![255, 0, 0, 255]),
            image_path: None,
            image_width: 1,
            image_height: 1,
            pinned: false,
            group_id: 0,
            created_at: String::new(),
        };

        assert_eq!(
            main_row_external_action_plan(MainRowMenuAction::QuickSearch, Some(&text_item), &[]),
            Some(MainRowExternalActionPlan::QuickSearch("hello".to_string()))
        );
        assert!(matches!(
            main_row_current_item_action_plan(MainRowMenuAction::SaveImage, Some(&image_item)),
            Some(MainRowCurrentItemActionPlan::SaveImage { .. })
        ));
        assert_eq!(
            main_row_dialog_action_plan(MainRowMenuAction::Edit, Some(&text_item)),
            Some(MainRowDialogActionPlan::EditItem {
                item_id: 7,
                title: "编辑 — hello".to_string()
            })
        );
    }

    #[test]
    fn macos_main_event_model_consumes_shared_product_events() {
        let mut model = MacosMainEventModel::default();

        assert_eq!(
            model.accept_application_event(ApplicationEvent::VvShowRequested {
                target: NativeWindowToken(42),
            }),
            MacosApplicationEventRoute::ScheduleVvShow {
                target: NativeWindowToken(42),
            }
        );
        assert_eq!(
            model.accept_application_event(ApplicationEvent::UpdateCheckReady),
            MacosApplicationEventRoute::RefreshSettings
        );
        assert_eq!(
            model.accept_application_event(ApplicationEvent::StartupDataReconciled { deleted: 3 }),
            MacosApplicationEventRoute::ReconcileData { deleted: 3 }
        );

        let paste = MainAsyncEvent::ImagePaste(crate::app_core::ImagePasteReadyResult {
            image: Some((vec![255, 0, 0, 255], 1, 1)),
            target: NativeWindowToken(7),
            hide_main: true,
            backspaces: 2,
        });
        assert_eq!(
            model.accept_async_event(&paste),
            MacosMainAsyncEventRoute::PasteImage {
                has_image: true,
                target: NativeWindowToken(7),
                hide_main: true,
                backspaces: 2,
            }
        );

        let translate = MainAsyncEvent::TextTranslate(crate::app_core::TextOperationReadyResult {
            text: Some("translated".to_string()),
            error: None,
        });
        assert_eq!(
            model.accept_async_event(&translate),
            MacosMainAsyncEventRoute::TextTranslate {
                has_text: true,
                has_error: false,
            }
        );
        assert_eq!(model.application_routes().len(), 3);
        assert_eq!(model.async_routes().len(), 2);
    }

    #[test]
    fn macos_startup_plan_creates_native_windows_from_shared_layout() {
        let model = MacosMainWindowModel::default();
        let startup = model.startup_plan("ZSClip Test", false);
        let mut host = MacosMainWindowHost::default();

        assert_eq!(startup.lifecycle, LifecycleEvent::Mount);
        assert_eq!(startup.main_window.title, "ZSClip Test");
        assert_eq!(startup.main_window.size.width, 300);
        assert_eq!(startup.main_window.size.height, 615);
        assert!(!startup.main_window.main_visible);

        let presentation = host.create_main_windows(startup.main_window.clone());
        assert!(matches!(
            presentation,
            NativeMainWindowPresentation::Created(_)
        ));
        assert_eq!(host.requests().len(), 1);
        assert_eq!(host.requests()[0].title, "ZSClip Test");
        assert!(!host.requests()[0].main_visible);
    }

    #[test]
    fn macos_application_model_implements_native_runtime_driver() {
        let mut application = MacosApplicationModel::default();
        let startup = application.start_runtime(NativeRuntimeStartupRequest {
            app_name: "Demo Mac".to_string(),
            main_window: NativeMainWindowRequest {
                title: "Demo Mac".to_string(),
                size: crate::app_core::Size {
                    width: 640,
                    height: 420,
                },
                main_visible: true,
            },
            status_item_tooltip: Some("Demo Mac".to_string()),
        });

        let NativeRuntimeStartupResult::Started(handles) = startup else {
            panic!("macOS runtime driver should create native window handles");
        };
        assert_eq!(application.window_session().main_windows(), Some(handles));
        assert!(application.window_session().main_visible());
        assert_eq!(application.lifecycle_phase(), ComponentPhase::Mounted);

        application
            .dispatch_ui_command(Command::window(crate::app_core::command_ids::OPEN_SETTINGS));
        assert_eq!(application.pending_command_count(), 1);
        assert_eq!(
            application.product_command_results()[0].result_name,
            "zsclip.window.open_settings"
        );
        assert_eq!(
            application.poll_application_event(),
            Some(ApplicationEvent::ItemsPageReady)
        );
        assert_eq!(
            application.pop_command().map(|command| command.id),
            Some(crate::app_core::command_ids::OPEN_SETTINGS)
        );

        application.request_shutdown();
        assert!(application.runtime_shutdown_requested);
    }

    #[test]
    fn macos_application_can_reuse_zsclip_product_adapter() {
        use crate::app_core::{main_menu_command_for_id, menu_ids, ProductAdapterHost};
        use crate::zsclip_product_adapter::{
            zsclip_product_adapter_manifest, ZsclipProductAdapter,
        };

        let manifest = zsclip_product_adapter_manifest();
        assert!(manifest.command_routes.iter().any(|route| {
            route.family_name == "row"
                && route.result_name == "zsclip.row.copy"
                && route.execution_owner == "product_adapter"
        }));
        assert!(manifest
            .ai_capability_ids
            .contains(&"clipboard.product.ocr"));

        let mut adapter = ZsclipProductAdapter::default();
        let command = main_menu_command_for_id(menu_ids::ROW_COPY).unwrap();
        let result = adapter.execute_product_command(command);
        assert!(result.accepted);
        assert_eq!(result.result_name, "zsclip.row.copy");

        let mut application = MacosApplicationModel::default();
        application.dispatch_ui_command(
            main_menu_command_for_id(menu_ids::ROW_COPY)
                .expect("row copy should be a shared ZSClip menu command"),
        );
        assert_eq!(
            application.product_command_results()[0].result_name,
            "zsclip.row.copy"
        );
    }

    #[test]
    fn macos_application_model_owns_lifecycle_commands_windows_and_events() {
        let mut application = MacosApplicationModel::default();
        assert_eq!(application.lifecycle_phase(), ComponentPhase::New);

        let startup = application.mount("ZSClip App", true).unwrap();
        assert_eq!(startup.lifecycle, LifecycleEvent::Mount);
        assert_eq!(application.lifecycle_phase(), ComponentPhase::Mounted);
        assert!(application.mount("duplicate", true).is_err());
        let handles = NativeMainWindowHandles {
            main: MacosMainWindowHandle(10),
            quick: MacosMainWindowHandle(11),
        };
        application.attach_main_windows(handles, true);
        application.note_main_render();
        assert_eq!(application.window_session().main_windows(), Some(handles));
        assert!(application.window_session().main_visible());
        assert_eq!(application.window_session().main_render_generation(), 1);
        application.record_main_window_host_session(
            MacosMainWindowHandle(10),
            UiRect::new(12, 24, 312, 638),
            false,
        );
        assert_eq!(
            application.window_session().main_host_appearance(),
            Some(MacosMainWindowHandle(10))
        );
        assert_eq!(
            application.window_session().main_host_bounds(),
            Some((MacosMainWindowHandle(10), UiRect::new(12, 24, 312, 638)))
        );
        assert_eq!(
            application.window_session().main_host_activation_policy(),
            Some((MacosMainWindowHandle(10), false))
        );
        assert_eq!(application.window_session().main_host_generation(), 1);
        application.record_edge_auto_hide_session(true, true, Some(UiRect::new(0, 0, 300, 615)));
        assert!(application.window_session().edge_auto_hide_enabled());
        assert!(application.window_session().edge_hidden());
        assert_eq!(
            application.window_session().edge_bounds(),
            Some(UiRect::new(0, 0, 300, 615))
        );
        assert_eq!(application.window_session().edge_generation(), 1);

        let settings_handle = MacosSettingsWindowHandle(12);
        application.present_settings_window(settings_handle);
        assert_eq!(
            application.window_session().settings_window(),
            Some(settings_handle)
        );
        assert!(application.window_session().settings_visible());
        assert_eq!(
            application
                .window_session()
                .settings_presentation_generation(),
            1
        );
        application.hide_settings_window();
        assert!(!application.window_session().settings_visible());

        let text_item = ClipItem {
            id: 7,
            kind: ClipKind::Text,
            preview: "hello macOS".to_string(),
            text: Some("hello macOS".to_string()),
            source_app: "test".to_string(),
            file_paths: None,
            image_bytes: None,
            image_path: None,
            image_width: 0,
            image_height: 0,
            pinned: false,
            group_id: 0,
            created_at: String::new(),
        };
        application.accept_clip_item_for_preview(&text_item);
        assert_eq!(
            application.clip_payloads().latest_kind(),
            Some(ClipKind::Text)
        );
        assert_eq!(application.clip_payloads().latest_preview(), "hello macOS");
        assert_eq!(application.clip_payloads().text_items_seen(), 1);
        assert_eq!(application.clip_payloads().preview_generation(), 1);

        let file_item = ClipItem {
            id: 8,
            kind: ClipKind::Files,
            preview: "report.pdf".to_string(),
            text: None,
            source_app: "test".to_string(),
            file_paths: Some(vec!["/tmp/report.pdf".to_string()]),
            image_bytes: None,
            image_path: None,
            image_width: 0,
            image_height: 0,
            pinned: false,
            group_id: 0,
            created_at: String::new(),
        };
        application.accept_clip_item_for_preview(&file_item);
        assert_eq!(
            application.clip_payloads().latest_kind(),
            Some(ClipKind::Files)
        );
        assert_eq!(application.clip_payloads().file_items_seen(), 1);
        assert_eq!(application.clip_payloads().preview_generation(), 2);

        let image_item = ClipItem {
            id: 9,
            kind: ClipKind::Image,
            preview: "image payload".to_string(),
            text: None,
            source_app: "test".to_string(),
            file_paths: None,
            image_bytes: Some(vec![255, 255, 255, 255]),
            image_path: None,
            image_width: 1,
            image_height: 1,
            pinned: false,
            group_id: 0,
            created_at: String::new(),
        };
        application.accept_clip_item_for_preview(&image_item);
        assert_eq!(
            application.clip_payloads().latest_kind(),
            Some(ClipKind::Image)
        );
        assert_eq!(application.clip_payloads().image_items_seen(), 1);
        assert_eq!(application.clip_payloads().preview_generation(), 3);
        assert_eq!(application.clip_payloads().cached_thumbnail_ids(), &[9]);

        application.replace_visible_clip_items(&[
            text_item.clone(),
            file_item.clone(),
            image_item.clone(),
        ]);
        assert_eq!(application.list_session().visible_item_ids(), &[7, 8, 9]);
        assert_eq!(application.list_session().list_generation(), 1);
        application.select_clip_item_ids(&[9, 0, 7, 999, 9]);
        assert_eq!(application.list_session().selected_item_ids(), &[7, 9]);
        assert_eq!(application.list_session().selection_generation(), 1);
        application.remember_scroll_anchor(8, 14);
        assert_eq!(application.list_session().scroll_anchor(), Some((8, 14)));
        assert_eq!(application.list_session().scroll_generation(), 1);
        application.replace_visible_clip_items(&[text_item.clone()]);
        assert_eq!(application.list_session().visible_item_ids(), &[7]);
        assert_eq!(application.list_session().selected_item_ids(), &[7]);
        assert_eq!(application.list_session().scroll_anchor(), Some((7, 0)));
        assert_eq!(application.list_session().list_generation(), 2);
        application.update_main_visual_state(
            TitleButtonVisibility {
                search: true,
                setting: true,
                minimize: true,
                close: true,
            },
            crate::app_core::MainEmptyStateKind::Records,
            true,
        );
        assert!(application.main_visual_session().title_buttons().search);
        assert_eq!(
            application.main_visual_session().empty_state(),
            Some(crate::app_core::MainEmptyStateKind::Records)
        );
        assert!(application.main_visual_session().image_preview_enabled());
        assert_eq!(application.main_visual_session().visual_generation(), 1);
        application.record_adapter_prelude_boundary(
            &["app_core", "settings_model"],
            &["macos_app", "AppKitHost"],
        );
        assert_eq!(
            application.adapter_prelude().shared_contract_roots(),
            &["app_core", "settings_model"]
        );
        assert_eq!(
            application.adapter_prelude().native_adapter_roots(),
            &["macos_app", "AppKitHost"]
        );
        assert_eq!(application.adapter_prelude().boundary_generation(), 1);
        application.record_native_id_session(
            &["zsclip.main", "zsclip.quick", "zsclip.settings"],
            &["startupRecovery", "vvShow", "cloudSync"],
            "zsclip.statusItem",
        );
        assert_eq!(
            application.native_ids().window_identifiers(),
            &["zsclip.main", "zsclip.quick", "zsclip.settings"]
        );
        assert_eq!(
            application.native_ids().timer_identifiers(),
            &["startupRecovery", "vvShow", "cloudSync"]
        );
        assert_eq!(
            application.native_ids().status_item_identifier(),
            Some("zsclip.statusItem")
        );
        assert_eq!(application.native_ids().generation(), 1);
        application.record_main_search_session(
            MacosMainSearchControlHandle(3),
            true,
            "query",
            Some(MacosMainSearchStyleResource(4)),
        );
        assert_eq!(
            application.main_search_session().handle(),
            Some(MacosMainSearchControlHandle(3))
        );
        assert!(application.main_search_session().visible());
        assert_eq!(application.main_search_session().text(), "query");
        assert_eq!(
            application.main_search_session().style_resource(),
            Some(MacosMainSearchStyleResource(4))
        );
        assert_eq!(application.main_search_session().generation(), 1);
        application.record_transient_window_session(
            MacosMainWindowHandle(2),
            MacosTransientWindowHandle(5),
            UiRect::new(20, 30, 220, 180),
            true,
        );
        assert_eq!(
            application.transient_session().owner(),
            Some(MacosMainWindowHandle(2))
        );
        assert_eq!(
            application.transient_session().handle(),
            Some(MacosTransientWindowHandle(5))
        );
        assert_eq!(
            application.transient_session().bounds(),
            Some(UiRect::new(20, 30, 220, 180))
        );
        assert!(application.transient_session().visible());
        assert_eq!(application.transient_session().generation(), 1);
        application.record_paste_target_discovery_session(
            "Chrome_WidgetWin_1, WorkerW",
            Some(MacosPasteTargetHandle(8)),
        );
        assert_eq!(
            application
                .paste_target_discovery_session()
                .skip_class_names(),
            "Chrome_WidgetWin_1, WorkerW"
        );
        assert_eq!(
            application
                .paste_target_discovery_session()
                .last_candidate(),
            Some(MacosPasteTargetHandle(8))
        );
        assert_eq!(application.paste_target_discovery_session().generation(), 1);
        application.record_low_level_input_session(
            true,
            true,
            true,
            MacosPointerScope::TransientWindow,
        );
        assert!(application
            .low_level_input_session()
            .outside_hide_timer_active());
        assert!(application
            .low_level_input_session()
            .edge_auto_hide_timer_active());
        assert!(application
            .low_level_input_session()
            .quick_escape_monitor_active());
        assert_eq!(
            application.low_level_input_session().last_pointer_scope(),
            Some(MacosPointerScope::TransientWindow)
        );
        assert_eq!(application.low_level_input_session().generation(), 1);
        application.record_hover_preview_session(true, Some(9), true);
        assert!(application.hover_preview_session().visible());
        assert_eq!(
            application.hover_preview_session().hovered_item_id(),
            Some(9)
        );
        assert!(application
            .hover_preview_session()
            .mouse_leave_tracking_active());
        assert_eq!(application.hover_preview_session().generation(), 1);
        application.record_startup_integrations_session(true, true, true, true, 2);
        assert!(application
            .startup_integrations_session()
            .status_item_registered());
        assert!(application
            .startup_integrations_session()
            .hotkeys_registered());
        assert!(application
            .startup_integrations_session()
            .clipboard_monitor_registered());
        assert!(application
            .startup_integrations_session()
            .vv_monitor_registered());
        assert_eq!(
            application.startup_integrations_session().recovery_ticks(),
            2
        );
        assert_eq!(application.startup_integrations_session().generation(), 1);
        application.record_window_refresh_session(
            true,
            true,
            true,
            Some(MacosMainWindowHandle(10)),
        );
        assert_eq!(
            application
                .window_refresh_session()
                .settings_reload_generation(),
            1
        );
        assert_eq!(
            application
                .window_refresh_session()
                .database_reload_generation(),
            1
        );
        assert_eq!(
            application
                .window_refresh_session()
                .settings_window_refresh_generation(),
            1
        );
        assert_eq!(
            application.window_refresh_session().peer_sync_generation(),
            1
        );
        assert_eq!(
            application.window_refresh_session().last_peer_source(),
            Some(MacosMainWindowHandle(10))
        );
        assert_eq!(application.window_refresh_session().generation(), 1);
        application.record_window_registry_session(
            Some(MacosMainWindowHandle(10)),
            Some(MacosMainWindowHandle(11)),
            true,
            true,
        );
        assert_eq!(
            application.window_registry_session().main(),
            Some(MacosMainWindowHandle(10))
        );
        assert_eq!(
            application.window_registry_session().quick(),
            Some(MacosMainWindowHandle(11))
        );
        assert_eq!(
            application
                .window_registry_session()
                .clipboard_ignore_generation(),
            1
        );
        assert_eq!(
            application
                .window_registry_session()
                .skip_next_clipboard_generation(),
            1
        );
        assert_eq!(application.window_registry_session().generation(), 1);
        application.record_hover_clear_session(true, true, true);
        assert!(application
            .hover_clear_session()
            .preserved_scrollbar_hover());
        assert!(application
            .hover_clear_session()
            .cleared_pointer_down_state());
        assert!(application.hover_clear_session().noactivate_hit_item());
        assert_eq!(application.hover_clear_session().generation(), 1);

        application.queue_command(Command::window(crate::app_core::command_ids::OPEN_SETTINGS));
        assert_eq!(application.pending_command_count(), 1);
        assert_eq!(
            application.pop_command().unwrap().id,
            crate::app_core::command_ids::OPEN_SETTINGS
        );
        assert_eq!(application.pending_command_count(), 0);

        assert!(application.begin_cloud_sync());
        assert!(!application.begin_cloud_sync());
        assert!(application.background_tasks().cloud_sync_in_progress());
        assert_eq!(
            application.route_application_event(ApplicationEvent::CloudSyncReady),
            MacosApplicationEventRoute::ApplyCloudSync
        );
        assert_eq!(
            application.product_event_results()[0].event_name,
            "cloud_sync_ready"
        );
        assert!(!application.background_tasks().cloud_sync_in_progress());
        assert_eq!(
            application.route_application_event(ApplicationEvent::LanSyncReady),
            MacosApplicationEventRoute::RefreshLan
        );
        assert_eq!(
            application.product_event_results()[1].event_name,
            "lan_sync_ready"
        );
        assert_eq!(application.background_tasks().lan_refresh_generation(), 1);
        let thumbnail = MainAsyncEvent::ImageThumbnail(crate::app_core::ImageThumbReadyResult {
            item_id: 9,
            image: None,
        });
        assert_eq!(
            application.route_async_event(&thumbnail),
            MacosMainAsyncEventRoute::CacheThumbnail {
                item_id: 9,
                has_image: false,
            }
        );
        assert_eq!(application.background_tasks().cached_thumbnail_ids(), &[9]);
        assert_eq!(application.clip_payloads().cached_thumbnail_ids(), &[9]);
        let text = MainAsyncEvent::TextTranslate(crate::app_core::TextOperationReadyResult {
            text: Some("done".to_string()),
            error: None,
        });
        application.route_async_event(&text);
        assert_eq!(
            application.background_tasks().completed_text_operations(),
            1
        );
        assert_eq!(application.background_tasks().completed_image_pastes(), 0);

        application.select_settings_page(SettingsPage::Cloud);
        application.note_settings_presented();
        application.note_settings_draft_changed();
        assert_eq!(
            application.settings_session().current_page(),
            SettingsPage::Cloud
        );
        assert!(application.settings_session().dirty());
        assert_eq!(application.settings_session().draft_generation(), 1);
        assert_eq!(application.settings_session().presentation_generation(), 2);
        application.note_settings_applied();
        assert!(!application.settings_session().dirty());
        assert_eq!(application.settings_session().applied_generation(), 1);
        application.record_settings_plugin_sections_session(
            &["quick_search", "ocr", "translate", "ai"],
            3,
        );
        assert_eq!(
            application
                .settings_plugin_sections_session()
                .visible_provider_sections()
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            vec!["quick_search", "ocr", "translate", "ai"]
        );
        assert_eq!(
            application
                .settings_plugin_sections_session()
                .enabled_feature_count(),
            3
        );
        assert_eq!(
            application.settings_plugin_sections_session().generation(),
            1
        );
        application.record_settings_plugin_section_domain_session(4, 3, 3, 4, 1);
        assert_eq!(
            application
                .settings_plugin_section_domain_session()
                .control_domain_count(),
            4
        );
        assert_eq!(
            application
                .settings_plugin_section_domain_session()
                .layout_domain_count(),
            3
        );
        assert_eq!(
            application
                .settings_plugin_section_domain_session()
                .provider_domain_count(),
            3
        );
        assert_eq!(
            application
                .settings_plugin_section_domain_session()
                .tool_domain_count(),
            4
        );
        assert_eq!(
            application
                .settings_plugin_section_domain_session()
                .host_refresh_count(),
            1
        );
        assert_eq!(
            application
                .settings_plugin_section_domain_session()
                .generation(),
            1
        );
        application.record_settings_multi_sync_sections_session("lan_webdav", 4, true);
        assert_eq!(
            application
                .settings_multi_sync_sections_session()
                .selected_mode(),
            "lan_webdav"
        );
        assert_eq!(
            application
                .settings_multi_sync_sections_session()
                .visible_section_count(),
            4
        );
        assert_eq!(
            application
                .settings_multi_sync_sections_session()
                .rebuild_generation(),
            1
        );
        assert_eq!(
            application
                .settings_multi_sync_sections_session()
                .generation(),
            1
        );
        application.record_settings_group_sections_session(1, 0, Some(42), 3, 2);
        assert_eq!(
            application
                .settings_group_sections_session()
                .vv_source_tab(),
            1
        );
        assert_eq!(
            application
                .settings_group_sections_session()
                .group_view_tab(),
            0
        );
        assert_eq!(
            application
                .settings_group_sections_session()
                .selected_group_id(),
            Some(42)
        );
        assert_eq!(
            application
                .settings_group_sections_session()
                .record_group_count(),
            3
        );
        assert_eq!(
            application
                .settings_group_sections_session()
                .phrase_group_count(),
            2
        );
        assert_eq!(
            application.settings_group_sections_session().generation(),
            1
        );
        application.record_settings_group_section_domain_session(2, 4, 3, 1, 1);
        assert_eq!(
            application
                .settings_group_section_domain_session()
                .cache_domain_count(),
            2
        );
        assert_eq!(
            application
                .settings_group_section_domain_session()
                .display_domain_count(),
            4
        );
        assert_eq!(
            application
                .settings_group_section_domain_session()
                .list_domain_count(),
            3
        );
        assert_eq!(
            application
                .settings_group_section_domain_session()
                .selection_domain_count(),
            1
        );
        assert_eq!(
            application
                .settings_group_section_domain_session()
                .order_domain_count(),
            1
        );
        assert_eq!(
            application
                .settings_group_section_domain_session()
                .generation(),
            1
        );
        application.record_settings_group_page_session(1, 2, 2, 1, 5, 1);
        assert_eq!(application.settings_group_page_session().toggle_count(), 1);
        assert_eq!(
            application.settings_group_page_session().dropdown_count(),
            2
        );
        assert_eq!(
            application.settings_group_page_session().tab_button_count(),
            2
        );
        assert_eq!(application.settings_group_page_session().list_count(), 1);
        assert_eq!(
            application
                .settings_group_page_session()
                .action_button_count(),
            5
        );
        assert_eq!(
            application
                .settings_group_page_session()
                .status_label_count(),
            1
        );
        assert_eq!(application.settings_group_page_session().generation(), 1);
        application.record_settings_general_page_session(7, 9, "200", true);
        assert_eq!(
            application
                .settings_general_page_session()
                .startup_toggle_count(),
            7
        );
        assert_eq!(
            application
                .settings_general_page_session()
                .behavior_toggle_count(),
            9
        );
        assert_eq!(
            application
                .settings_general_page_session()
                .max_items_label(),
            "200"
        );
        assert!(application
            .settings_general_page_session()
            .skip_window_enabled());
        assert_eq!(application.settings_general_page_session().generation(), 1);
        application.record_settings_general_page_sections_session(10, 1, 4, 3, 3, 5, 1);
        assert_eq!(
            application
                .settings_general_page_sections_session()
                .startup_toggle_count(),
            10
        );
        assert_eq!(
            application
                .settings_general_page_sections_session()
                .retention_control_count(),
            1
        );
        assert_eq!(
            application
                .settings_general_page_sections_session()
                .behavior_toggle_count(),
            4
        );
        assert_eq!(
            application
                .settings_general_page_sections_session()
                .sound_control_count(),
            3
        );
        assert_eq!(
            application
                .settings_general_page_sections_session()
                .skip_window_control_count(),
            3
        );
        assert_eq!(
            application
                .settings_general_page_sections_session()
                .position_control_count(),
            5
        );
        assert_eq!(
            application
                .settings_general_page_sections_session()
                .action_button_count(),
            1
        );
        assert_eq!(
            application
                .settings_general_page_sections_session()
                .generation(),
            1
        );
        application.record_settings_hotkey_page_session("Win + V", "Ctrl+Shift + V", true);
        assert_eq!(
            application
                .settings_hotkey_page_session()
                .main_hotkey_preview(),
            "Win + V"
        );
        assert_eq!(
            application
                .settings_hotkey_page_session()
                .plain_hotkey_preview(),
            "Ctrl+Shift + V"
        );
        assert!(application.settings_hotkey_page_session().recording());
        assert_eq!(application.settings_hotkey_page_session().generation(), 1);
        application.record_settings_hotkey_page_sections_session(5, 4, 3, 3);
        assert_eq!(
            application
                .settings_hotkey_page_sections_session()
                .main_shortcut_control_count(),
            5
        );
        assert_eq!(
            application
                .settings_hotkey_page_sections_session()
                .plain_shortcut_control_count(),
            4
        );
        assert_eq!(
            application
                .settings_hotkey_page_sections_session()
                .system_action_count(),
            3
        );
        assert_eq!(
            application
                .settings_hotkey_page_sections_session()
                .note_label_count(),
            3
        );
        assert_eq!(
            application
                .settings_hotkey_page_sections_session()
                .generation(),
            1
        );
        application.record_settings_plugin_page_session(true, "winocr", "baidu", 5);
        assert!(application
            .settings_plugin_page_session()
            .quick_search_enabled());
        assert_eq!(
            application.settings_plugin_page_session().ocr_provider(),
            "winocr"
        );
        assert_eq!(
            application
                .settings_plugin_page_session()
                .translate_provider(),
            "baidu"
        );
        assert_eq!(
            application
                .settings_plugin_page_session()
                .tool_toggle_count(),
            5
        );
        assert_eq!(application.settings_plugin_page_session().generation(), 1);
        application.record_settings_plugin_page_sections_session(5, 4, 4, 4, 2);
        assert_eq!(
            application
                .settings_plugin_page_sections_session()
                .quick_search_control_count(),
            5
        );
        assert_eq!(
            application
                .settings_plugin_page_sections_session()
                .ocr_control_count(),
            4
        );
        assert_eq!(
            application
                .settings_plugin_page_sections_session()
                .translate_control_count(),
            4
        );
        assert_eq!(
            application
                .settings_plugin_page_sections_session()
                .tool_toggle_count(),
            4
        );
        assert_eq!(
            application
                .settings_plugin_page_sections_session()
                .tool_action_count(),
            2
        );
        assert_eq!(
            application
                .settings_plugin_page_sections_session()
                .generation(),
            1
        );
        application.record_settings_about_page_session(true, true, "/Users/test/Library/ZsClip");
        assert!(application.settings_about_page_session().source_available());
        assert!(application.settings_about_page_session().update_available());
        assert_eq!(
            application.settings_about_page_session().data_dir(),
            "/Users/test/Library/ZsClip"
        );
        assert_eq!(application.settings_about_page_session().generation(), 1);
        application.record_settings_about_page_sections_session(2, 1, 1, 1, 1);
        assert_eq!(
            application
                .settings_about_page_sections_session()
                .metadata_label_count(),
            2
        );
        assert_eq!(
            application
                .settings_about_page_sections_session()
                .source_link_count(),
            1
        );
        assert_eq!(
            application
                .settings_about_page_sections_session()
                .update_status_count(),
            1
        );
        assert_eq!(
            application
                .settings_about_page_sections_session()
                .update_action_count(),
            1
        );
        assert_eq!(
            application
                .settings_about_page_sections_session()
                .data_label_count(),
            1
        );
        assert_eq!(
            application
                .settings_about_page_sections_session()
                .generation(),
            1
        );
        application.record_settings_cloud_page_session("lan", 2, 4, Some(3));
        assert_eq!(
            application.settings_cloud_page_session().selected_mode(),
            "lan"
        );
        assert_eq!(
            application
                .settings_cloud_page_session()
                .pending_pair_count(),
            2
        );
        assert_eq!(
            application
                .settings_cloud_page_session()
                .discovered_device_count(),
            4
        );
        assert_eq!(
            application.settings_cloud_page_session().selected_lan_row(),
            Some(3)
        );
        assert_eq!(application.settings_cloud_page_session().generation(), 1);
        application.record_settings_cloud_webdav_page_session(5, 2, 1);
        assert_eq!(
            application
                .settings_cloud_webdav_page_session()
                .field_count(),
            5
        );
        assert_eq!(
            application
                .settings_cloud_webdav_page_session()
                .action_row_count(),
            2
        );
        assert_eq!(
            application
                .settings_cloud_webdav_page_session()
                .status_label_count(),
            1
        );
        assert_eq!(
            application
                .settings_cloud_webdav_page_session()
                .generation(),
            1
        );
        application.record_settings_cloud_lan_page_session(5, 1, 1, 2, 4);
        assert_eq!(
            application.settings_cloud_lan_page_session().field_count(),
            5
        );
        assert_eq!(
            application
                .settings_cloud_lan_page_session()
                .action_row_count(),
            1
        );
        assert_eq!(
            application
                .settings_cloud_lan_page_session()
                .device_list_count(),
            1
        );
        assert_eq!(
            application
                .settings_cloud_lan_page_session()
                .qr_action_count(),
            2
        );
        assert_eq!(
            application
                .settings_cloud_lan_page_session()
                .helper_label_count(),
            4
        );
        assert_eq!(
            application.settings_cloud_lan_page_session().generation(),
            1
        );
        application.record_settings_cloud_lan_devices_session(2, 4, Some(1), Some(3));
        assert_eq!(
            application
                .settings_cloud_lan_devices_session()
                .pending_pair_count(),
            2
        );
        assert_eq!(
            application
                .settings_cloud_lan_devices_session()
                .discovered_device_count(),
            4
        );
        assert_eq!(
            application
                .settings_cloud_lan_devices_session()
                .selected_pair_row(),
            Some(1)
        );
        assert_eq!(
            application
                .settings_cloud_lan_devices_session()
                .selected_device_row(),
            Some(3)
        );
        assert_eq!(
            application
                .settings_cloud_lan_devices_session()
                .refresh_generation(),
            1
        );
        application.record_settings_owner_draw_session(true, true, 7, 5);
        assert!(application
            .settings_owner_draw_session()
            .hover_control_active());
        assert!(application
            .settings_owner_draw_session()
            .qr_payload_available());
        assert_eq!(
            application
                .settings_owner_draw_session()
                .toggle_draw_count(),
            7
        );
        assert_eq!(
            application
                .settings_owner_draw_session()
                .button_draw_count(),
            5
        );
        assert_eq!(application.settings_owner_draw_session().generation(), 1);
        application.record_settings_owner_draw_domain_session(2, 1, 25, 10, 3, 18);
        assert_eq!(
            application
                .settings_owner_draw_domain_session()
                .qr_draw_count(),
            2
        );
        assert_eq!(
            application
                .settings_owner_draw_domain_session()
                .source_link_draw_count(),
            1
        );
        assert_eq!(
            application
                .settings_owner_draw_domain_session()
                .toggle_role_count(),
            25
        );
        assert_eq!(
            application
                .settings_owner_draw_domain_session()
                .dropdown_role_count(),
            10
        );
        assert_eq!(
            application
                .settings_owner_draw_domain_session()
                .accent_role_count(),
            3
        );
        assert_eq!(
            application
                .settings_owner_draw_domain_session()
                .button_role_count(),
            18
        );
        assert_eq!(
            application
                .settings_owner_draw_domain_session()
                .generation(),
            1
        );
        application.record_settings_page_builder_session(32, 11, 6);
        assert_eq!(
            application
                .settings_page_builder_session()
                .registered_control_count(),
            32
        );
        assert_eq!(
            application
                .settings_page_builder_session()
                .ownerdraw_control_count(),
            11
        );
        assert_eq!(
            application.settings_page_builder_session().section_count(),
            6
        );
        assert_eq!(application.settings_page_builder_session().generation(), 1);
        application.record_settings_raw_control_session(16, 7, 5, 9, 2, 19);
        assert_eq!(
            application
                .settings_raw_control_session()
                .label_control_count(),
            16
        );
        assert_eq!(
            application
                .settings_raw_control_session()
                .button_control_count(),
            7
        );
        assert_eq!(
            application
                .settings_raw_control_session()
                .dropdown_control_count(),
            5
        );
        assert_eq!(
            application
                .settings_raw_control_session()
                .input_control_count(),
            9
        );
        assert_eq!(
            application
                .settings_raw_control_session()
                .listbox_control_count(),
            2
        );
        assert_eq!(
            application
                .settings_raw_control_session()
                .toggle_row_count(),
            19
        );
        assert_eq!(application.settings_raw_control_session().generation(), 1);
        application.record_settings_form_action_session(21, 3, 2, 19);
        assert_eq!(
            application
                .settings_form_action_session()
                .ownerdraw_action_count(),
            21
        );
        assert_eq!(
            application
                .settings_form_action_session()
                .action_row_count(),
            3
        );
        assert_eq!(
            application.settings_form_action_session().qr_action_count(),
            2
        );
        assert_eq!(
            application
                .settings_form_action_session()
                .toggle_action_count(),
            19
        );
        assert_eq!(application.settings_form_action_session().generation(), 1);
        application.record_settings_form_field_session(18, 4, 6, 8, 3);
        assert_eq!(
            application.settings_form_field_session().label_row_count(),
            18
        );
        assert_eq!(
            application
                .settings_form_field_session()
                .value_label_row_count(),
            4
        );
        assert_eq!(
            application
                .settings_form_field_session()
                .dropdown_row_count(),
            6
        );
        assert_eq!(
            application.settings_form_field_session().input_row_count(),
            8
        );
        assert_eq!(
            application.settings_form_field_session().button_row_count(),
            3
        );
        assert_eq!(application.settings_form_field_session().generation(), 1);
        application.record_settings_control_factory_session(14, 9, 2, 7, 5);
        assert_eq!(
            application.settings_control_factory_session().label_count(),
            14
        );
        assert_eq!(
            application.settings_control_factory_session().input_count(),
            9
        );
        assert_eq!(
            application
                .settings_control_factory_session()
                .listbox_count(),
            2
        );
        assert_eq!(
            application
                .settings_control_factory_session()
                .action_button_count(),
            7
        );
        assert_eq!(
            application
                .settings_control_factory_session()
                .toggle_count(),
            5
        );
        assert_eq!(
            application.settings_control_factory_session().generation(),
            1
        );
        application.record_settings_control_registry_session(32, 18, 5);
        assert_eq!(
            application
                .settings_control_registry_session()
                .registered_control_count(),
            32
        );
        assert_eq!(
            application
                .settings_control_registry_session()
                .scrollable_control_count(),
            18
        );
        assert_eq!(
            application.settings_control_registry_session().page_count(),
            5
        );
        assert_eq!(
            application.settings_control_registry_session().generation(),
            1
        );
        application.record_settings_page_navigation_session(SettingsPage::Cloud, 144, 3);
        assert_eq!(
            application
                .settings_page_navigation_session()
                .current_page(),
            SettingsPage::Cloud
        );
        assert_eq!(
            application.settings_page_navigation_session().scroll_y(),
            144
        );
        assert_eq!(
            application
                .settings_page_navigation_session()
                .reposition_count(),
            3
        );
        assert_eq!(
            application.settings_page_navigation_session().generation(),
            1
        );
        application.record_settings_page_navigation_domain_session(5, 2, 1, 6, 3);
        assert_eq!(
            application
                .settings_page_navigation_domain_session()
                .control_reposition_count(),
            5
        );
        assert_eq!(
            application
                .settings_page_navigation_domain_session()
                .scroll_update_count(),
            2
        );
        assert_eq!(
            application
                .settings_page_navigation_domain_session()
                .page_switch_count(),
            1
        );
        assert_eq!(
            application
                .settings_page_navigation_domain_session()
                .visibility_update_count(),
            6
        );
        assert_eq!(
            application
                .settings_page_navigation_domain_session()
                .redraw_count(),
            3
        );
        assert_eq!(
            application
                .settings_page_navigation_domain_session()
                .generation(),
            1
        );
        application.record_settings_page_ensure_session(SettingsPage::Cloud, 5);
        assert_eq!(
            application.settings_page_ensure_session().ensured_page(),
            Some(SettingsPage::Cloud)
        );
        assert_eq!(
            application
                .settings_page_ensure_session()
                .built_page_count(),
            5
        );
        assert_eq!(application.settings_page_ensure_session().generation(), 1);
        application.record_settings_page_sync_session(6, 18, 6);
        assert_eq!(
            application.settings_page_sync_session().synced_page_count(),
            6
        );
        assert_eq!(
            application
                .settings_page_sync_session()
                .enabled_control_count(),
            18
        );
        assert_eq!(
            application
                .settings_page_sync_session()
                .invalidation_count(),
            6
        );
        assert_eq!(application.settings_page_sync_session().generation(), 1);
        application.record_settings_cloud_sync_session("lan", 5, 14, true);
        assert_eq!(application.settings_cloud_sync_session().mode(), "lan");
        assert_eq!(
            application
                .settings_cloud_sync_session()
                .webdav_control_count(),
            5
        );
        assert_eq!(
            application
                .settings_cloud_sync_session()
                .lan_control_count(),
            14
        );
        assert_eq!(
            application
                .settings_cloud_sync_session()
                .lan_refresh_generation(),
            1
        );
        assert_eq!(application.settings_cloud_sync_session().generation(), 1);
        application.record_settings_cloud_webdav_sync_session(5, true, true);
        assert_eq!(
            application
                .settings_cloud_webdav_sync_session()
                .control_count(),
            5
        );
        assert!(application.settings_cloud_webdav_sync_session().enabled());
        assert!(application
            .settings_cloud_webdav_sync_session()
            .status_text_available());
        assert_eq!(
            application
                .settings_cloud_webdav_sync_session()
                .generation(),
            1
        );
        application.record_settings_cloud_lan_sync_session(14, true, true, 14);
        assert_eq!(
            application
                .settings_cloud_lan_sync_session()
                .control_count(),
            14
        );
        assert!(application.settings_cloud_lan_sync_session().enabled());
        assert!(application
            .settings_cloud_lan_sync_session()
            .list_refreshed());
        assert_eq!(
            application
                .settings_cloud_lan_sync_session()
                .invalidation_count(),
            14
        );
        assert_eq!(
            application.settings_cloud_lan_sync_session().generation(),
            1
        );
        application.record_settings_plugin_sync_session(true, true, false, 2);
        assert!(application.settings_plugin_sync_session().search_enabled());
        assert!(application
            .settings_plugin_sync_session()
            .ocr_fields_visible());
        assert!(!application
            .settings_plugin_sync_session()
            .translate_enabled());
        assert_eq!(
            application
                .settings_plugin_sync_session()
                .tool_control_count(),
            2
        );
        assert_eq!(application.settings_plugin_sync_session().generation(), 1);
        application.record_settings_control_selection_session(3, 3, 4, 4, 2);
        assert_eq!(
            application
                .settings_control_selection_session()
                .general_selection_count(),
            3
        );
        assert_eq!(
            application
                .settings_control_selection_session()
                .cloud_selection_count(),
            3
        );
        assert_eq!(
            application
                .settings_control_selection_session()
                .hotkey_selection_count(),
            4
        );
        assert_eq!(
            application
                .settings_control_selection_session()
                .plugin_selection_count(),
            4
        );
        assert_eq!(
            application
                .settings_control_selection_session()
                .group_selection_count(),
            2
        );
        assert_eq!(
            application
                .settings_control_selection_session()
                .generation(),
            1
        );
        application.record_settings_dropdown_plugin_session(4, 3, 2, 5);
        assert_eq!(
            application
                .settings_dropdown_plugin_session()
                .search_option_count(),
            4
        );
        assert_eq!(
            application
                .settings_dropdown_plugin_session()
                .ocr_option_count(),
            3
        );
        assert_eq!(
            application
                .settings_dropdown_plugin_session()
                .translate_provider_count(),
            2
        );
        assert_eq!(
            application
                .settings_dropdown_plugin_session()
                .translate_target_count(),
            5
        );
        assert_eq!(
            application.settings_dropdown_plugin_session().generation(),
            1
        );
        application.record_settings_dropdown_domain_session(3, 3, 4, 4, 2);
        assert_eq!(
            application
                .settings_dropdown_domain_session()
                .general_dropdown_count(),
            3
        );
        assert_eq!(
            application
                .settings_dropdown_domain_session()
                .cloud_dropdown_count(),
            3
        );
        assert_eq!(
            application
                .settings_dropdown_domain_session()
                .hotkey_dropdown_count(),
            4
        );
        assert_eq!(
            application
                .settings_dropdown_domain_session()
                .plugin_dropdown_count(),
            4
        );
        assert_eq!(
            application
                .settings_dropdown_domain_session()
                .group_dropdown_count(),
            2
        );
        assert_eq!(
            application.settings_dropdown_domain_session().generation(),
            1
        );
        application.record_settings_toggle_state_session(7102, 12);
        assert_eq!(
            application
                .settings_toggle_state_session()
                .toggled_control_id(),
            7102
        );
        assert_eq!(
            application
                .settings_toggle_state_session()
                .enabled_toggle_count(),
            12
        );
        assert_eq!(application.settings_toggle_state_session().generation(), 1);
        application.record_settings_toggle_domain_session(16, 2, 2, 5, 1);
        assert_eq!(
            application
                .settings_toggle_domain_session()
                .general_toggle_count(),
            16
        );
        assert_eq!(
            application
                .settings_toggle_domain_session()
                .cloud_toggle_count(),
            2
        );
        assert_eq!(
            application
                .settings_toggle_domain_session()
                .hotkey_toggle_count(),
            2
        );
        assert_eq!(
            application
                .settings_toggle_domain_session()
                .plugin_toggle_count(),
            5
        );
        assert_eq!(
            application
                .settings_toggle_domain_session()
                .group_toggle_count(),
            1
        );
        assert_eq!(application.settings_toggle_domain_session().generation(), 1);
        application.record_settings_host_helper_session(14, 5, true);
        assert_eq!(
            application
                .settings_host_helper_session()
                .text_update_count(),
            14
        );
        assert_eq!(
            application
                .settings_host_helper_session()
                .invalidation_count(),
            5
        );
        assert_eq!(
            application
                .settings_host_helper_session()
                .theme_generation(),
            1
        );
        assert_eq!(application.settings_host_helper_session().generation(), 1);
        application.record_settings_app_apply_collect_session(true, true, 2, true);
        assert_eq!(
            application
                .settings_app_apply_collect_session()
                .applied_generation(),
            1
        );
        assert_eq!(
            application
                .settings_app_apply_collect_session()
                .collected_generation(),
            1
        );
        assert_eq!(
            application
                .settings_app_apply_collect_session()
                .saved_settings_count(),
            2
        );
        assert_eq!(
            application
                .settings_app_apply_collect_session()
                .peer_sync_generation(),
            1
        );
        assert_eq!(
            application
                .settings_app_apply_collect_session()
                .generation(),
            1
        );
        application.record_settings_app_collect_domain_session(7, 2, 5, 1, 8);
        assert_eq!(
            application
                .settings_app_collect_domain_session()
                .general_collect_count(),
            7
        );
        assert_eq!(
            application
                .settings_app_collect_domain_session()
                .hotkey_collect_count(),
            2
        );
        assert_eq!(
            application
                .settings_app_collect_domain_session()
                .plugin_collect_count(),
            5
        );
        assert_eq!(
            application
                .settings_app_collect_domain_session()
                .group_collect_count(),
            1
        );
        assert_eq!(
            application
                .settings_app_collect_domain_session()
                .cloud_collect_count(),
            8
        );
        assert_eq!(
            application
                .settings_app_collect_domain_session()
                .generation(),
            1
        );
        application.record_settings_app_effects_session(true, true, true, true, true);
        assert_eq!(
            application
                .settings_app_effects_session()
                .persisted_generation(),
            1
        );
        assert_eq!(
            application
                .settings_app_effects_session()
                .integration_refresh_generation(),
            1
        );
        assert_eq!(
            application
                .settings_app_effects_session()
                .data_refresh_generation(),
            1
        );
        assert_eq!(
            application
                .settings_app_effects_session()
                .window_refresh_generation(),
            1
        );
        assert_eq!(
            application
                .settings_app_effects_session()
                .peer_sync_generation(),
            1
        );
        assert_eq!(application.settings_app_effects_session().generation(), 1);
        application.record_settings_sync_action_domain_session(4, 7);
        assert_eq!(
            application
                .settings_sync_action_domain_session()
                .webdav_action_count(),
            4
        );
        assert_eq!(
            application
                .settings_sync_action_domain_session()
                .lan_action_count(),
            7
        );
        assert_eq!(
            application
                .settings_sync_action_domain_session()
                .generation(),
            1
        );
        application.record_settings_platform_action_domain_session(1, 2, 4, 2, 3);
        assert_eq!(
            application
                .settings_platform_action_domain_session()
                .hotkey_action_count(),
            1
        );
        assert_eq!(
            application
                .settings_platform_action_domain_session()
                .general_action_count(),
            2
        );
        assert_eq!(
            application
                .settings_platform_action_domain_session()
                .plugin_action_count(),
            4
        );
        assert_eq!(
            application
                .settings_platform_action_domain_session()
                .about_action_count(),
            2
        );
        assert_eq!(
            application
                .settings_platform_action_domain_session()
                .system_action_count(),
            3
        );
        assert_eq!(
            application
                .settings_platform_action_domain_session()
                .generation(),
            1
        );
        application.record_settings_window_state_session(SettingsPage::General, 144, 96, 2);
        assert_eq!(
            application.settings_window_state_session().initial_page(),
            SettingsPage::General
        );
        assert_eq!(application.settings_window_state_session().ui_dpi(), 144);
        assert_eq!(
            application
                .settings_window_state_session()
                .reset_control_count(),
            96
        );
        assert_eq!(
            application
                .settings_window_state_session()
                .dynamic_section_count(),
            2
        );
        assert_eq!(application.settings_window_state_session().generation(), 1);
        application.record_settings_window_create_session(
            MacosSettingsWindowHandle(12),
            SettingsPage::General,
            2,
            1,
            true,
        );
        assert_eq!(
            application.settings_window_create_session().parent(),
            Some(MacosSettingsWindowHandle(12))
        );
        assert_eq!(
            application.settings_window_create_session().initial_page(),
            SettingsPage::General
        );
        assert_eq!(
            application
                .settings_window_create_session()
                .save_close_control_count(),
            2
        );
        assert_eq!(
            application
                .settings_window_create_session()
                .page_built_count(),
            1
        );
        assert_eq!(
            application
                .settings_window_create_session()
                .applied_generation(),
            1
        );
        assert_eq!(application.settings_window_create_session().generation(), 1);
        application.record_settings_window_metrics_session(960, 6, 2, 18);
        assert_eq!(
            application
                .settings_window_metrics_session()
                .measured_content_height(),
            960
        );
        assert_eq!(
            application
                .settings_window_metrics_session()
                .scroll_slot_count(),
            6
        );
        assert_eq!(
            application
                .settings_window_metrics_session()
                .rebuilt_page_count(),
            2
        );
        assert_eq!(
            application
                .settings_window_metrics_session()
                .visible_control_count(),
            18
        );
        assert_eq!(
            application.settings_window_metrics_session().generation(),
            1
        );
        application.record_settings_window_layout_session(
            144,
            UiRect::new(0, 0, 1100, 740),
            UiRect::new(120, 80, 1220, 820),
            2,
        );
        assert_eq!(
            application.settings_window_layout_session().layout_dpi(),
            144
        );
        assert_eq!(
            application.settings_window_layout_session().client_bounds(),
            Some(UiRect::new(0, 0, 1100, 740))
        );
        assert_eq!(
            application.settings_window_layout_session().window_bounds(),
            Some(UiRect::new(120, 80, 1220, 820))
        );
        assert_eq!(
            application
                .settings_window_layout_session()
                .move_plan_count(),
            2
        );
        assert_eq!(application.settings_window_layout_session().generation(), 1);
        application
            .record_settings_window_lifecycle_session(true, true, true, true, true, true, true);
        assert_eq!(
            application
                .settings_window_lifecycle_session()
                .presented_generation(),
            1
        );
        assert_eq!(
            application
                .settings_window_lifecycle_session()
                .bounds_update_generation(),
            1
        );
        assert_eq!(
            application
                .settings_window_lifecycle_session()
                .focused_generation(),
            1
        );
        assert_eq!(
            application
                .settings_window_lifecycle_session()
                .destroyed_generation(),
            1
        );
        assert_eq!(
            application
                .settings_window_lifecycle_session()
                .pointer_capture_generation(),
            1
        );
        assert_eq!(
            application
                .settings_window_lifecycle_session()
                .repaint_generation(),
            1
        );
        assert_eq!(
            application
                .settings_window_lifecycle_session()
                .cloud_refresh_generation(),
            1
        );
        assert_eq!(
            application.settings_window_lifecycle_session().generation(),
            1
        );
        application.record_settings_window_destroy_session(3, 1, 7);
        assert_eq!(
            application
                .settings_window_destroy_session()
                .timer_cleanup_count(),
            3
        );
        assert_eq!(
            application
                .settings_window_destroy_session()
                .dropdown_cleanup_count(),
            1
        );
        assert_eq!(
            application
                .settings_window_destroy_session()
                .resource_cleanup_count(),
            7
        );
        assert_eq!(
            application.settings_window_destroy_session().generation(),
            1
        );
        application.record_settings_window_color_session(41, 9, 5);
        assert_eq!(
            application
                .settings_window_color_session()
                .surface_role_count(),
            41
        );
        assert_eq!(
            application
                .settings_window_color_session()
                .edit_role_count(),
            9
        );
        assert_eq!(
            application
                .settings_window_color_session()
                .list_role_count(),
            5
        );
        assert_eq!(application.settings_window_color_session().generation(), 1);
        application.record_settings_window_surface_controls_session(20, 4, 3, 7, 9, 2);
        assert_eq!(
            application
                .settings_window_surface_controls_session()
                .general_count(),
            20
        );
        assert_eq!(
            application
                .settings_window_surface_controls_session()
                .hotkey_count(),
            4
        );
        assert_eq!(
            application
                .settings_window_surface_controls_session()
                .group_count(),
            3
        );
        assert_eq!(
            application
                .settings_window_surface_controls_session()
                .cloud_count(),
            7
        );
        assert_eq!(
            application
                .settings_window_surface_controls_session()
                .plugin_count(),
            9
        );
        assert_eq!(
            application
                .settings_window_surface_controls_session()
                .about_count(),
            2
        );
        assert_eq!(
            application
                .settings_window_surface_controls_session()
                .generation(),
            1
        );
        application.record_settings_window_paint_session(true, true, true, 4);
        assert_eq!(
            application
                .settings_window_paint_session()
                .chrome_paint_generation(),
            1
        );
        assert_eq!(
            application
                .settings_window_paint_session()
                .content_paint_generation(),
            1
        );
        assert_eq!(
            application
                .settings_window_paint_session()
                .scrollbar_paint_generation(),
            1
        );
        assert_eq!(
            application
                .settings_window_paint_session()
                .owner_draw_count(),
            4
        );
        assert_eq!(application.settings_window_paint_session().generation(), 1);

        assert!(application.activate());
        assert_eq!(application.lifecycle_phase(), ComponentPhase::Active);
        assert!(application.suspend());
        assert_eq!(application.lifecycle_phase(), ComponentPhase::Suspended);
        assert!(application.activate());
        assert!(application.unmount());
        assert_eq!(application.lifecycle_phase(), ComponentPhase::Unmounted);
    }

    #[test]
    fn macos_settings_window_consumes_shared_navigation_and_content_plans() {
        let mut model = MacosSettingsWindowModel::default();
        model.select_page(SettingsPage::Plugin);
        model.set_scroll_y(-48);
        let snapshot = MacosSettingsSnapshot {
            quick_search_enabled: true,
            image_ocr_provider: "cloud".to_string(),
            text_translate_provider: "baidu".to_string(),
            super_mail_merge_enabled: true,
            wps_taskpane_enabled: true,
            multi_sync_mode: "off".to_string(),
        };

        let presentation = model.presentation(1100, 740, &snapshot, true);

        assert_eq!(presentation.content.page, SettingsPage::Plugin);
        assert_eq!(
            presentation.content.source,
            crate::settings_model::SettingsContentSource::PluginDynamic
        );
        assert_eq!(presentation.content.scroll_y, -48);
        assert_eq!(presentation.navigation.items.len(), 6);
        assert_eq!(presentation.navigation_paint.len(), 6);
        assert!(!presentation.chrome_paint.text_commands.is_empty());
        assert!(!presentation.content.sections.is_empty());
        assert!(!presentation.content_paint.paint_commands.is_empty());
    }

    #[test]
    fn macos_settings_window_consumes_shared_input_plans() {
        let model = MacosSettingsWindowModel::default();
        let plugin_index = SettingsPage::Plugin.index() as i32;

        let hover = model.nav_hover_transition(-1, plugin_index);
        assert_eq!(hover.next_hot, plugin_index);
        assert_eq!(hover.invalidate_rects.len(), 1);

        let layout = SettingsScrollLayout::new(100, 600, 1000, 500, 800, 3, 5);
        assert_eq!(
            model.pointer_down_target(Point { x: 790, y: 300 }, layout, 250, 4, 4, 2,),
            SettingsPointerDownTarget::ScrollbarThumb {
                drag_start_y: 300,
                drag_start_scroll: 250,
            }
        );

        let drag =
            model.pointer_move_transition(Point { x: 790, y: 360 }, -1, true, layout, 300, 250);
        assert!(drag.drag_scroll_y.is_some());
        assert_eq!(drag.nav_hover, None);
        assert_eq!(model.wheel_scroll_delta(120), -60);
    }

    #[test]
    fn macos_settings_window_consumes_shared_geometry_plans() {
        let model = MacosSettingsWindowModel::default();
        let work = UiRect::new(0, 0, 1440, 900);

        let fit = model
            .fit_window_plan(UiRect::new(1200, 700, 1700, 1100), work, 12, 720, 480)
            .unwrap();
        assert_eq!((fit.width, fit.height), (720, 480));
        assert!(fit.x + fit.width <= work.right - 12);
        assert!(fit.y + fit.height <= work.bottom - 12);

        let scaled = model
            .scale_transition_plan(UiRect::new(100, 100, 900, 700), work, 12, 720, 480, 96, 144)
            .unwrap();
        assert_eq!((scaled.width, scaled.height), (1200, 876));
        assert!(scaled.x >= 12);
        assert!(scaled.y >= 12);
    }

    #[test]
    fn macos_settings_window_uses_exclusive_multi_sync_sections() {
        let mut model = MacosSettingsWindowModel::default();
        model.select_page(SettingsPage::Cloud);

        let webdav = model.presentation(
            1100,
            740,
            &MacosSettingsSnapshot {
                multi_sync_mode: "webdav".to_string(),
                ..Default::default()
            },
            false,
        );
        let lan = model.presentation(
            1100,
            740,
            &MacosSettingsSnapshot {
                multi_sync_mode: "lan".to_string(),
                ..Default::default()
            },
            false,
        );

        assert_eq!(
            webdav.content.source,
            crate::settings_model::SettingsContentSource::MultiSyncDynamic
        );
        assert_eq!(
            lan.content.source,
            crate::settings_model::SettingsContentSource::MultiSyncDynamic
        );
        assert_ne!(webdav.content.sections, lan.content.sections);
        assert!(webdav
            .content
            .sections
            .iter()
            .any(|section| section.title == "WebDAV 传输"));
        assert!(lan
            .content
            .sections
            .iter()
            .any(|section| section.title == "扫码绑定"));
        assert!(!webdav
            .content
            .sections
            .iter()
            .any(|section| section.title == "扫码绑定"));
    }

    #[test]
    fn macos_settings_window_host_tracks_present_and_focus_requests() {
        let owner = MacosSettingsWindowHandle(99);
        let mut host = MacosSettingsWindowHost::default();

        let first = host.present_settings_window(NativeSettingsWindowRequest {
            owner,
            existing: None,
            bounds: UiRect::new(10, 20, 1110, 760),
        });

        assert_eq!(
            first,
            NativeSettingsWindowPresentation::Created(MacosSettingsWindowHandle(1))
        );
        assert_eq!(host.requests().len(), 1);
        assert_eq!(host.requests()[0].bounds, UiRect::new(10, 20, 1110, 760));

        let second = host.present_settings_window(NativeSettingsWindowRequest {
            owner,
            existing: Some(MacosSettingsWindowHandle(1)),
            bounds: UiRect::new(0, 0, 0, 0),
        });

        assert_eq!(
            second,
            NativeSettingsWindowPresentation::FocusedExisting(MacosSettingsWindowHandle(1))
        );
        assert_eq!(host.requests().len(), 2);
        host.set_settings_window_bounds(
            MacosSettingsWindowHandle(1),
            UiRect::new(30, 40, 900, 680),
        );
        assert_eq!(
            host.bounds_updates(),
            vec![(MacosSettingsWindowHandle(1), UiRect::new(30, 40, 900, 680))]
        );

        host.destroy_settings_window(MacosSettingsWindowHandle(1));
        assert_eq!(host.destroyed(), vec![MacosSettingsWindowHandle(1)]);

        host.focus_settings_window(MacosSettingsWindowHandle(1));
        assert_eq!(host.focused(), vec![MacosSettingsWindowHandle(1)]);
        assert!(!host.track_settings_pointer_leave(MacosSettingsWindowHandle(1)));
        assert_eq!(
            host.pointer_leave_tracking(),
            vec![MacosSettingsWindowHandle(1)]
        );
        host.capture_settings_pointer(MacosSettingsWindowHandle(1));
        host.release_settings_pointer(MacosSettingsWindowHandle(1));
        assert_eq!(host.captured(), vec![MacosSettingsWindowHandle(1)]);
        assert_eq!(host.released(), vec![MacosSettingsWindowHandle(1)]);
        assert_eq!(
            host.settings_window_client_to_screen(
                MacosSettingsWindowHandle(1),
                Point { x: 8, y: 9 }
            ),
            Some(Point { x: 8, y: 9 })
        );
        assert_eq!(
            host.client_to_screen_requests(),
            vec![(MacosSettingsWindowHandle(1), Point { x: 8, y: 9 })]
        );
        assert_eq!(
            host.settings_window_client_bounds(MacosSettingsWindowHandle(1)),
            None
        );
        assert_eq!(
            host.client_bounds_queries(),
            vec![MacosSettingsWindowHandle(1)]
        );
        assert_eq!(
            host.settings_window_bounds(MacosSettingsWindowHandle(1)),
            None
        );
        assert_eq!(
            host.window_bounds_queries(),
            vec![MacosSettingsWindowHandle(1)]
        );
        assert!(!host.request_settings_window_repaint(MacosSettingsWindowHandle(1)));
        assert_eq!(host.repainted(), vec![MacosSettingsWindowHandle(1)]);
        assert!(!host.request_settings_window_area_repaint(
            MacosSettingsWindowHandle(1),
            Some(UiRect::new(1, 2, 3, 4)),
            false
        ));
        assert_eq!(
            host.settings_window_layout_dpi(MacosSettingsWindowHandle(1)),
            96
        );
        assert_eq!(
            host.layout_dpi_queries(),
            vec![MacosSettingsWindowHandle(1)]
        );
        assert_eq!(
            host.area_repaints(),
            vec![(
                MacosSettingsWindowHandle(1),
                Some(UiRect::new(1, 2, 3, 4)),
                false
            )]
        );
        assert!(!host.request_cloud_settings_refresh(MacosSettingsWindowHandle(1)));
        assert_eq!(host.cloud_refreshes(), vec![MacosSettingsWindowHandle(1)]);

        let reopened = host.present_settings_window(NativeSettingsWindowRequest {
            owner,
            existing: None,
            bounds: UiRect::new(1, 2, 301, 602),
        });
        assert_eq!(
            reopened,
            NativeSettingsWindowPresentation::Created(MacosSettingsWindowHandle(1))
        );
        assert_eq!(
            host.settings_window_client_bounds(MacosSettingsWindowHandle(1)),
            Some(UiRect::new(0, 0, 300, 600))
        );
        assert_eq!(
            host.settings_window_bounds(MacosSettingsWindowHandle(1)),
            Some(UiRect::new(1, 2, 301, 602))
        );
        assert!(host.request_settings_window_repaint(MacosSettingsWindowHandle(1)));
        assert!(host.request_settings_window_area_repaint(
            MacosSettingsWindowHandle(1),
            None,
            true
        ));
        assert_eq!(
            host.settings_window_layout_dpi(MacosSettingsWindowHandle(1)),
            96
        );
        assert!(host.track_settings_pointer_leave(MacosSettingsWindowHandle(1)));
        assert!(host.request_cloud_settings_refresh(MacosSettingsWindowHandle(1)));
    }

    #[test]
    fn macos_ai_action_presenter_tracks_menu_settings_and_execution_routes() {
        let mut application = MacosApplicationModel::default();
        let capabilities: Vec<_> = crate::app_core::product_ai_capability_catalog()
            .iter()
            .take(2)
            .map(|descriptor| descriptor.capability())
            .collect();

        let menu_request = NativeAiActionMenuRequest {
            owner: MacosMainWindowHandle(1),
            surface: ProductAiUiSurface::RowContextMenu,
            anchor: Point { x: 12, y: 18 },
            capabilities: capabilities.clone(),
            context_item_ids: vec![42],
            prompt_text: "make it concise".to_string(),
        };
        let invocation = application
            .present_ai_action_menu(menu_request)
            .expect("macOS AI presenter should select the first capability");

        assert_eq!(invocation.capability_id, capabilities[0].id);
        assert_eq!(
            application
                .ai_action_presentation_session()
                .menu_request_count(),
            1
        );
        assert_eq!(
            application.ai_action_presentation_session().last_surface(),
            Some(ProductAiUiSurface::RowContextMenu)
        );

        let settings_request = NativeAiSettingsSurfaceRequest {
            owner: Some(MacosMainWindowHandle(1)),
            surface: ProductAiUiSurface::SettingsPluginPage,
            capabilities: capabilities.clone(),
            provider_names: vec!["llms", "skills"],
        };
        assert!(application.present_ai_settings_surface(settings_request));
        assert_eq!(
            application
                .ai_action_presentation_session()
                .settings_request_count(),
            1
        );
        assert_eq!(
            application.ai_action_presentation_session().last_surface(),
            Some(ProductAiUiSurface::SettingsPluginPage)
        );

        let plan = crate::app_core::product_ai_execution_plan(invocation).unwrap();
        let result = application.bridge_ai_execution_plan(plan);
        assert!(result.accepted);
        assert_eq!(
            application
                .ai_action_presentation_session()
                .executed_action_names(),
            &["clean_text"]
        );
        assert_eq!(application.product_command_results().len(), 1);
    }

    #[test]
    fn macos_settings_control_host_tracks_native_control_requests() {
        let mut host = MacosSettingsControlHost::default();
        let spec = SettingsControlSpec::action(
            SettingsComponentKind::Dropdown,
            42,
            "WebDAV",
            UiRect::new(10, 20, 210, 52),
        );

        let handle = host.create_control(&spec);
        assert!(host.control_exists(handle));
        host.set_control_visible(handle, true);
        host.set_control_bounds(handle, UiRect::new(12, 24, 212, 56));
        assert_eq!(host.control_at_point(Point { x: 20, y: 30 }), Some(handle));
        host.set_control_enabled(handle, false);
        assert_eq!(host.control_at_point(Point { x: 20, y: 30 }), None);
        host.set_control_text(handle, "LAN");

        assert_eq!(handle, MacosSettingsControlHandle(1));
        assert_eq!(host.requests(), vec![MacosSettingsControlRequest { spec }]);
        assert_eq!(host.visibility_changes(), vec![(handle, true)]);
        assert_eq!(host.enabled_changes(), vec![(handle, false)]);
        assert_eq!(
            host.hit_test_queries(),
            vec![Point { x: 20, y: 30 }, Point { x: 20, y: 30 }]
        );
        assert_eq!(
            host.bounds_changes(),
            vec![(handle, UiRect::new(12, 24, 212, 56))]
        );
        assert_eq!(
            host.control_screen_bounds(handle),
            Some(UiRect::new(12, 24, 212, 56))
        );
        assert_eq!(host.screen_bounds_queries(), vec![handle]);
        assert_eq!(host.control_text(handle), "LAN".to_string());
        assert_eq!(host.text_changes(), vec![(handle, "LAN".to_string())]);
        assert!(host.request_control_repaint(handle));
        assert_eq!(host.repainted(), vec![handle]);
        host.destroy_control(handle);
        assert_eq!(host.destroyed(), vec![handle]);
        assert!(!host.control_exists(handle));
    }

    #[test]
    fn macos_settings_dropdown_host_tracks_popup_lifecycle() {
        let mut host = MacosSettingsDropdownHost::default();
        let items: Vec<String> = settings_dropdown_max_items_labels()
            .iter()
            .map(|label| (*label).to_string())
            .collect();
        let selected = settings_dropdown_index_for_max_items(1000);

        let created = host.present_settings_dropdown(NativeSettingsDropdownRequest {
            owner: MacosSettingsWindowHandle(1),
            control_id: 501,
            anchor: UiRect::new(10, 20, 210, 52),
            items: items.clone(),
            selected,
            width: 180,
        });

        assert_eq!(
            created,
            NativeSettingsDropdownPresentation::Created(MacosSettingsDropdownHandle(1))
        );
        assert_eq!(
            host.requests(),
            vec![MacosSettingsDropdownRequest {
                owner: MacosSettingsWindowHandle(1),
                control_id: 501,
                anchor: UiRect::new(10, 20, 210, 52),
                items,
                selected,
                width: 180,
            }]
        );
        assert_eq!(
            host.settings_dropdown_bounds(MacosSettingsDropdownHandle(1)),
            Some(UiRect::new(10, 20, 210, 52))
        );
        assert_eq!(
            host.bounds(),
            vec![(MacosSettingsDropdownHandle(1), UiRect::new(10, 20, 210, 52))]
        );

        host.destroy_settings_dropdown(MacosSettingsDropdownHandle(1));
        assert_eq!(host.destroyed(), vec![MacosSettingsDropdownHandle(1)]);
    }

    #[test]
    fn macos_status_item_host_consumes_shared_menu_entries() {
        use crate::app_core::MainTrayMenuAction;

        let mut host = MacosStatusItemHost::default();
        assert!(host.install("ZSClip"));
        host.present_menu(&[
            StatusMenuEntry::Command {
                action: MainTrayMenuAction::ToggleWindow,
                label: "显示/隐藏".to_string(),
                icon_name: "window-new-symbolic".to_string(),
            },
            StatusMenuEntry::Separator,
        ]);

        assert!(host.installed);
        assert_eq!(host.tooltip, "ZSClip");
        assert_eq!(host.menu_entries.len(), 2);

        host.remove();
        assert!(!host.installed);
        assert!(host.menu_entries.is_empty());
    }

    #[test]
    fn macos_popup_menu_host_consumes_shared_popup_entries() {
        use crate::app_core::{
            main_row_menu_plan, ClipKind, MainRowMenuAction, MainRowMenuEntry, MainRowMenuInput,
        };

        let mut host = MacosPopupMenuHost {
            next_command: 42,
            ..Default::default()
        };
        let command = host.present_popup_menu(
            (),
            12,
            34,
            NativePopupMenuPlacement::TopLeft,
            &[NativePopupMenuEntry::Command {
                id: 7,
                label: "复制".to_string(),
                enabled: true,
                checked: false,
            }],
        );

        assert_eq!(command, 42);
        assert_eq!(host.last_position, (12, 34));
        assert_eq!(host.last_placement, Some(NativePopupMenuPlacement::TopLeft));
        assert_eq!(host.last_entries.len(), 1);

        let row_plan = main_row_menu_plan(MainRowMenuInput {
            selected_count: 1,
            has_unpinned: true,
            current_kind: ClipKind::Text,
            grouping_enabled: true,
            current_can_ocr: false,
            current_can_translate: true,
            current_is_excel: false,
            quick_search_enabled: true,
            qr_quick_enabled: true,
            super_mail_merge_enabled: false,
            lan_push_available: false,
        });
        let row_actions = row_plan
            .entries
            .iter()
            .filter_map(|entry| match entry {
                MainRowMenuEntry::Action { action, .. } => Some(*action),
                MainRowMenuEntry::Separator => None,
            })
            .collect::<Vec<_>>();
        assert!(row_actions.contains(&MainRowMenuAction::AddToGroup));
        assert!(row_actions.contains(&MainRowMenuAction::TextTranslate));
    }

    #[test]
    fn macos_transient_window_host_tracks_float_panel_actions() {
        let mut host = MacosTransientWindowHost::default();

        let created = host.create_transient_window(NativeTransientWindowRequest {
            owner: MacosMainWindowHandle(9),
            bounds: UiRect::new(0, 0, 200, 80),
        });
        assert_eq!(
            created,
            NativeTransientWindowPresentation::Created(MacosTransientWindowHandle(1))
        );
        assert_eq!(
            host.requests(),
            vec![MacosTransientWindowCreateRequest {
                owner: MacosMainWindowHandle(9),
                bounds: UiRect::new(0, 0, 200, 80),
            }]
        );

        host.present_transient_window(MacosTransientWindowHandle(1), UiRect::new(10, 20, 210, 120));
        host.hide_transient_window(MacosTransientWindowHandle(1));
        host.destroy_transient_window(MacosTransientWindowHandle(1));

        assert_eq!(
            host.actions(),
            vec![
                MacosTransientWindowAction::Present {
                    handle: MacosTransientWindowHandle(1),
                    bounds: UiRect::new(10, 20, 210, 120),
                },
                MacosTransientWindowAction::Hide(MacosTransientWindowHandle(1)),
                MacosTransientWindowAction::Destroy(MacosTransientWindowHandle(1)),
            ]
        );
    }

    #[test]
    fn macos_ime_host_tracks_candidate_and_composition_queries() {
        let mut host = MacosImeHost::default();
        host.set_next_candidate(Some(NativeImeCandidateAnchor::CandidatePoint {
            position: Point { x: 10, y: 20 },
        }));
        host.set_next_composition(Some(NativeImeCompositionAnchor::Rect {
            rect: UiRect::new(1, 2, 30, 40),
        }));
        host.set_next_has_default_ime_window(true);

        assert_eq!(
            host.candidate_anchor(MacosImeHandle(7), 2),
            Some(NativeImeCandidateAnchor::CandidatePoint {
                position: Point { x: 10, y: 20 },
            })
        );
        assert_eq!(
            host.composition_anchor(MacosImeHandle(7)),
            Some(NativeImeCompositionAnchor::Rect {
                rect: UiRect::new(1, 2, 30, 40),
            })
        );
        assert!(host.has_default_ime_window(MacosImeHandle(7)));
        assert_eq!(
            host.actions(),
            vec![
                MacosImeAction::QueryCandidate {
                    focus: MacosImeHandle(7),
                    index: 2,
                },
                MacosImeAction::QueryComposition(MacosImeHandle(7)),
                MacosImeAction::HasDefaultImeWindow(MacosImeHandle(7)),
            ]
        );
    }

    #[test]
    fn macos_text_caret_host_tracks_input_anchor_queries() {
        let mut host = MacosTextCaretHost::default();
        host.set_next_accessible(Some(NativeTextCaretAnchor::new(10, 20, 36)));
        host.set_next_thread(Some(NativeTextCaretAnchor::new(11, 21, 37)));
        host.set_next_focus_rect(Some(NativeTextCaretAnchor::new(12, 22, 38)));
        host.set_next_cursor(Some(NativeTextCaretAnchor::new(13, 23, 23)));
        host.set_next_focus_handle(Some(MacosTextCaretHandle(8)));

        assert_eq!(
            host.accessible_caret_anchor(MacosTextCaretHandle(3)),
            Some(NativeTextCaretAnchor::new(10, 20, 36))
        );
        assert_eq!(
            host.thread_caret_anchor(MacosTextCaretHandle(4)),
            Some(NativeTextCaretAnchor::new(11, 21, 37))
        );
        assert_eq!(
            host.focus_rect_anchor(MacosTextCaretHandle(5), 260, 180),
            Some(NativeTextCaretAnchor::new(12, 22, 38))
        );
        assert_eq!(
            host.cursor_anchor(),
            Some(NativeTextCaretAnchor::new(13, 23, 23))
        );
        assert_eq!(
            host.focus_handle_for_target(MacosTextCaretHandle(6)),
            MacosTextCaretHandle(8)
        );
        assert_eq!(
            host.actions(),
            vec![
                MacosTextCaretAction::QueryAccessible(MacosTextCaretHandle(3)),
                MacosTextCaretAction::QueryThread(MacosTextCaretHandle(4)),
                MacosTextCaretAction::QueryFocusRect {
                    focus: MacosTextCaretHandle(5),
                    max_width: 260,
                    max_height: 180,
                },
                MacosTextCaretAction::QueryCursor,
                MacosTextCaretAction::ResolveFocus(MacosTextCaretHandle(6)),
            ]
        );
    }

    #[test]
    fn macos_dialog_host_tracks_shared_message_contract() {
        let host = MacosDialogHost::default();
        host.record_message("标题", "内容", NativeDialogLevel::Info);

        assert_eq!(
            host.last_message(),
            Some(MacosDialogMessage {
                title: "标题".to_string(),
                message: "内容".to_string(),
                level: NativeDialogLevel::Info,
            })
        );
        assert_eq!(
            host.confirm(
                (),
                "标题",
                "内容",
                NativeDialogLevel::Warning,
                NativeDialogButtons::YesNoCancel,
            ),
            NativeDialogResponse::Cancel
        );
    }

    #[test]
    fn macos_shell_open_host_consumes_shared_open_contract() {
        let host = MacosShellOpenHost::default();
        host.open_path("https://example.test");

        assert_eq!(
            host.opened_paths(),
            vec!["https://example.test".to_string()]
        );
    }

    #[test]
    fn macos_window_identity_host_tracks_native_window_identity_queries() {
        let host = MacosWindowIdentityHost::default();
        host.set_process_name("wps");
        host.set_class_name("WPSDocumentView");
        host.set_root_handle(Some(MacosWindowIdentityHandle(9)));
        host.set_foreground_handle(Some(MacosWindowIdentityHandle(10)));
        host.set_existing_windows(vec![MacosWindowIdentityHandle(7)]);
        host.set_current_process_windows(vec![MacosWindowIdentityHandle(11)]);

        assert_eq!(
            host.process_name(MacosWindowIdentityHandle(7)),
            "wps".to_string()
        );
        assert_eq!(
            host.class_name(MacosWindowIdentityHandle(7)),
            "WPSDocumentView".to_string()
        );
        assert_eq!(
            host.root_handle(MacosWindowIdentityHandle(7)),
            MacosWindowIdentityHandle(9)
        );
        assert_eq!(host.foreground_handle(), MacosWindowIdentityHandle(10));
        assert!(host.exists(MacosWindowIdentityHandle(7)));
        assert!(host.is_foreground(MacosWindowIdentityHandle(10)));
        assert!(host.is_current_process_window(MacosWindowIdentityHandle(11)));
        assert_eq!(
            host.actions(),
            vec![
                MacosWindowIdentityAction::ProcessName(MacosWindowIdentityHandle(7)),
                MacosWindowIdentityAction::ClassName(MacosWindowIdentityHandle(7)),
                MacosWindowIdentityAction::RootHandle(MacosWindowIdentityHandle(7)),
                MacosWindowIdentityAction::ForegroundHandle,
                MacosWindowIdentityAction::Exists(MacosWindowIdentityHandle(7)),
                MacosWindowIdentityAction::IsForeground(MacosWindowIdentityHandle(10)),
                MacosWindowIdentityAction::IsCurrentProcessWindow(MacosWindowIdentityHandle(11)),
            ]
        );
    }

    #[test]
    fn macos_paste_target_host_consumes_shared_target_contract() {
        let mut host = MacosPasteTargetHost::default();
        host.set_next_foreground_result(true);
        host.set_next_text_input_capabilities(PasteTargetTextInputCapabilities::text_input());
        host.set_next_text_input_ready(true);
        host.set_next_focus_status(PasteTargetFocusStatus::InsideTarget);

        assert!(host.force_paste_target_foreground(MacosPasteTargetHandle(7)));
        host.restore_paste_target_focus(MacosPasteTargetHandle(7), MacosPasteTargetHandle(8));
        assert!(host.set_paste_target_text(MacosPasteTargetHandle(7), "hello"));
        assert!(host
            .paste_target_text_input_capabilities(MacosPasteTargetHandle(7))
            .accepts_text_input());
        assert!(host.paste_target_text_input_ready(MacosPasteTargetHandle(7)));
        assert!(host.send_paste_shortcut(MacosPasteTargetHandle(7)));
        assert_eq!(
            host.paste_target_focus_status(MacosPasteTargetHandle(7), MacosPasteTargetHandle(8)),
            PasteTargetFocusStatus::InsideTarget
        );

        assert_eq!(
            host.actions(),
            vec![
                MacosPasteTargetAction::ForceForeground(MacosPasteTargetHandle(7)),
                MacosPasteTargetAction::RestoreFocus {
                    target: MacosPasteTargetHandle(7),
                    focus: MacosPasteTargetHandle(8),
                },
                MacosPasteTargetAction::SetText {
                    target: MacosPasteTargetHandle(7),
                    text: "hello".to_string(),
                },
                MacosPasteTargetAction::QueryTextInputCapabilities(MacosPasteTargetHandle(7)),
                MacosPasteTargetAction::QueryTextInputReady(MacosPasteTargetHandle(7)),
                MacosPasteTargetAction::SendPasteShortcut(MacosPasteTargetHandle(7)),
                MacosPasteTargetAction::QueryFocusStatus {
                    target: MacosPasteTargetHandle(7),
                    passthrough_focus: MacosPasteTargetHandle(8),
                },
            ]
        );
    }

    #[test]
    fn macos_file_dialog_host_consumes_shared_pick_file_contract() {
        let host = MacosFileDialogHost::default();
        host.set_next_result(Ok(Some("/tmp/paste.wav".to_string())));

        let result = host
            .pick_file(NativeFileDialogRequest {
                title: "选择提示音文件",
                filter_name: "Wave Files",
                filter_pattern: "*.wav",
                current_path: "/tmp/old.wav",
            })
            .unwrap();

        assert_eq!(result.as_deref(), Some("/tmp/paste.wav"));
        assert_eq!(
            host.requests(),
            vec![MacosFileDialogRequest {
                title: "选择提示音文件".to_string(),
                filter_name: "Wave Files".to_string(),
                filter_pattern: "*.wav".to_string(),
                current_path: "/tmp/old.wav".to_string(),
            }]
        );
    }

    #[test]
    fn macos_text_input_dialog_host_consumes_shared_prompt_contract() {
        let host = MacosTextInputDialogHost::default();
        host.set_next_result(Some("新名称".to_string()));

        let result = host.prompt_text(
            (),
            NativeTextInputDialogRequest {
                title: "重命名分组",
                label: "请输入新名称：",
                initial: "旧名称",
            },
        );

        assert_eq!(result.as_deref(), Some("新名称"));
        assert_eq!(
            host.requests(),
            vec![MacosTextInputDialogRequest {
                title: "重命名分组".to_string(),
                label: "请输入新名称：".to_string(),
                initial: "旧名称".to_string(),
            }]
        );
    }

    #[test]
    fn macos_text_input_dialog_host_consumes_shared_group_prompt_model() {
        let host = MacosTextInputDialogHost::default();
        host.set_next_result(Some("分组 B".to_string()));

        let result = host.prompt_text(
            (),
            settings_group_text_input_request(SettingsGroupTextInputKind::Rename, "分组 A"),
        );

        assert_eq!(result.as_deref(), Some("分组 B"));
        assert_eq!(
            host.requests(),
            vec![MacosTextInputDialogRequest {
                title: "重命名分组".to_string(),
                label: "请输入新名称：".to_string(),
                initial: "分组 A".to_string(),
            }]
        );
    }

    #[test]
    fn macos_settings_consumes_shared_settings_protocol_commands() {
        use crate::app_core::settings_protocol::{
            dispatch_settings_action, settings_action_route, settings_command_for_control_role,
            settings_command_id_for_role, SettingsControlRole,
        };
        use crate::app_core::{
            command_ids, settings_timer_task_for_id, Command, CommandPayload, SettingsTimerIds,
            SettingsTimerTask,
        };

        assert_eq!(
            settings_command_id_for_role(SettingsControlRole::Save),
            command_ids::SAVE_SETTINGS
        );
        assert_eq!(
            settings_command_for_control_role(SettingsControlRole::OpenConfig, 42),
            Command::window(command_ids::OPEN_SETTINGS_CONFIG)
        );
        assert_eq!(
            settings_command_for_control_role(SettingsControlRole::Dropdown, 6102).payload,
            CommandPayload::ControlId(6102)
        );
        let timer_ids = SettingsTimerIds {
            hide_scrollbar: 1,
            clear_save_hint: 2,
            dpi_fit: 3,
        };
        assert_eq!(
            settings_timer_task_for_id(3, timer_ids),
            Some(SettingsTimerTask::DpiFit)
        );
        assert_eq!(
            settings_action_route(SettingsAction::SyncWebDavNow),
            SettingsActionRoute::Sync
        );
        assert_eq!(
            settings_action_route(SettingsAction::AddGroup),
            SettingsActionRoute::Group
        );
        assert_eq!(
            settings_action_route(SettingsAction::OpenSourceRepository),
            SettingsActionRoute::Platform
        );

        let mut executor = MacosSettingsActionExecutor::default();
        let mut context = MacosSettingsActionContext;
        assert!(dispatch_settings_action(
            &mut executor,
            &mut context,
            SettingsAction::SyncWebDavNow
        ));
        assert!(dispatch_settings_action(
            &mut executor,
            &mut context,
            SettingsAction::AddGroup
        ));
        assert!(dispatch_settings_action(
            &mut executor,
            &mut context,
            SettingsAction::OpenSourceRepository
        ));
        assert_eq!(
            executor.actions(),
            vec![
                (SettingsActionRoute::Sync, SettingsAction::SyncWebDavNow),
                (SettingsActionRoute::Group, SettingsAction::AddGroup),
                (
                    SettingsActionRoute::Platform,
                    SettingsAction::OpenSourceRepository,
                ),
            ]
        );
        assert_eq!(executor.sync_action_count(), 1);
        assert_eq!(executor.group_action_count(), 1);
        assert_eq!(executor.platform_action_count(), 1);
    }

    #[test]
    fn macos_edit_text_dialog_host_consumes_shared_edit_contract() {
        let host = MacosEditTextDialogHost::default();
        host.set_next_result(
            Some("新内容".to_string()),
            Some(crate::app_core::Size {
                width: 720,
                height: 540,
            }),
        );
        let mut saved_text = String::new();
        let mut save_handler = |text: &str| {
            saved_text = text.to_string();
            Ok(())
        };

        let result = host.open_edit_text(
            (),
            NativeEditTextDialogRequest {
                title: "编辑记录",
                initial_text: "旧内容",
                initial_size: Some(crate::app_core::Size {
                    width: 640,
                    height: 500,
                }),
            },
            &mut save_handler,
        );

        assert!(result.saved);
        assert_eq!(saved_text, "新内容");
        assert_eq!(result.final_size.unwrap().width, 720);
        assert_eq!(
            host.requests(),
            vec![MacosEditTextDialogRequest {
                title: "编辑记录".to_string(),
                initial_text: "旧内容".to_string(),
                initial_size: Some(crate::app_core::Size {
                    width: 640,
                    height: 500,
                }),
            }]
        );
    }

    #[test]
    fn macos_edit_text_dialog_reports_unsaved_when_save_handler_rejects_text() {
        let host = MacosEditTextDialogHost::default();
        host.set_next_result(Some("无法保存".to_string()), None);
        let mut save_handler = |_text: &str| Err("save failed".to_string());

        let result = host.open_edit_text(
            (),
            NativeEditTextDialogRequest {
                title: "编辑记录",
                initial_text: "旧内容",
                initial_size: None,
            },
            &mut save_handler,
        );

        assert!(!result.saved);
        assert_eq!(result.final_size, None);
    }

    #[test]
    fn macos_mail_merge_window_host_consumes_shared_open_contract() {
        let host = MacosMailMergeWindowHost::default();

        host.open_mail_merge(
            (),
            NativeMailMergeWindowRequest {
                initial_excel_path: Some("/tmp/data.xlsx"),
            },
        );
        host.open_mail_merge(
            (),
            NativeMailMergeWindowRequest {
                initial_excel_path: None,
            },
        );

        assert_eq!(
            host.requests(),
            vec![
                MacosMailMergeWindowRequest {
                    initial_excel_path: Some("/tmp/data.xlsx".to_string()),
                },
                MacosMailMergeWindowRequest {
                    initial_excel_path: None,
                },
            ]
        );
    }
}

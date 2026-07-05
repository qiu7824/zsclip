use super::prelude::*;
use crate::app_core::{
    product_ai_execution_plan, ProductAdapterHost, ProductAdapterSettingsSnapshot,
    ProductAiInvocation,
};
use crate::zsclip_product_adapter::{
    zsclip_product_adapter_manifest, ZsclipProductAdapter, ZsclipProductSnapshot,
};

fn text_item(text: &str) -> ClipItem {
    ClipItem {
        id: 0,
        kind: ClipKind::Text,
        preview: build_preview(text),
        text: Some(text.to_string()),
        source_app: String::new(),
        file_paths: None,
        image_bytes: None,
        image_path: None,
        image_width: 0,
        image_height: 0,
        pinned: false,
        group_id: 0,
        created_at: String::new(),
    }
}

fn settings_actions_source() -> String {
    [
        include_str!("app/settings_actions.rs"),
        include_str!("app/settings_sync_actions.rs"),
        include_str!("app/settings_sync_actions_webdav.rs"),
        include_str!("app/settings_sync_actions_lan.rs"),
        include_str!("app/settings_group_actions.rs"),
        include_str!("app/settings_platform_actions.rs"),
        include_str!("app/settings_platform_actions_hotkey.rs"),
        include_str!("app/settings_platform_actions_general.rs"),
        include_str!("app/settings_platform_actions_plugin.rs"),
        include_str!("app/settings_platform_actions_about.rs"),
        include_str!("app/settings_platform_actions_system.rs"),
    ]
    .join("\n")
    .replace("\r\n", "\n")
}

fn settings_actions_executor_source() -> String {
    include_str!("app/settings_actions.rs").replace("\r\n", "\n")
}

fn settings_sync_actions_source() -> String {
    [
        include_str!("app/settings_sync_actions.rs"),
        include_str!("app/settings_sync_actions_webdav.rs"),
        include_str!("app/settings_sync_actions_lan.rs"),
    ]
    .join("\n")
    .replace("\r\n", "\n")
}

fn settings_sync_actions_entry_source() -> String {
    include_str!("app/settings_sync_actions.rs").replace("\r\n", "\n")
}

fn settings_sync_actions_webdav_source() -> String {
    include_str!("app/settings_sync_actions_webdav.rs").replace("\r\n", "\n")
}

fn settings_sync_actions_lan_source() -> String {
    include_str!("app/settings_sync_actions_lan.rs").replace("\r\n", "\n")
}

fn settings_group_actions_source() -> String {
    include_str!("app/settings_group_actions.rs").replace("\r\n", "\n")
}

fn settings_platform_actions_source() -> String {
    [
        include_str!("app/settings_platform_actions.rs"),
        include_str!("app/settings_platform_actions_hotkey.rs"),
        include_str!("app/settings_platform_actions_general.rs"),
        include_str!("app/settings_platform_actions_plugin.rs"),
        include_str!("app/settings_platform_actions_about.rs"),
        include_str!("app/settings_platform_actions_system.rs"),
    ]
    .join("\n")
    .replace("\r\n", "\n")
}

fn settings_platform_actions_entry_source() -> String {
    include_str!("app/settings_platform_actions.rs").replace("\r\n", "\n")
}

fn settings_platform_actions_hotkey_source() -> String {
    include_str!("app/settings_platform_actions_hotkey.rs").replace("\r\n", "\n")
}

fn settings_platform_actions_general_source() -> String {
    include_str!("app/settings_platform_actions_general.rs").replace("\r\n", "\n")
}

fn settings_platform_actions_plugin_source() -> String {
    include_str!("app/settings_platform_actions_plugin.rs").replace("\r\n", "\n")
}

fn settings_platform_actions_about_source() -> String {
    include_str!("app/settings_platform_actions_about.rs").replace("\r\n", "\n")
}

fn settings_platform_actions_system_source() -> String {
    include_str!("app/settings_platform_actions_system.rs").replace("\r\n", "\n")
}

fn settings_app_effects_source() -> String {
    include_str!("app/settings_app_effects.rs").replace("\r\n", "\n")
}

fn settings_app_effect_state_source() -> String {
    include_str!("app/settings_app_effect_state.rs").replace("\r\n", "\n")
}

fn settings_app_integration_effects_source() -> String {
    include_str!("app/settings_app_integration_effects.rs").replace("\r\n", "\n")
}

fn settings_app_data_effects_source() -> String {
    include_str!("app/settings_app_data_effects.rs").replace("\r\n", "\n")
}

fn settings_app_window_effects_source() -> String {
    include_str!("app/settings_app_window_effects.rs").replace("\r\n", "\n")
}

fn settings_app_apply_source() -> String {
    include_str!("app/settings_app_apply.rs").replace("\r\n", "\n")
}

fn settings_app_collect_source() -> String {
    include_str!("app/settings_app_collect.rs").replace("\r\n", "\n")
}

fn settings_app_collect_general_source() -> String {
    include_str!("app/settings_app_collect_general.rs").replace("\r\n", "\n")
}

fn settings_app_collect_hotkey_source() -> String {
    include_str!("app/settings_app_collect_hotkey.rs").replace("\r\n", "\n")
}

fn settings_app_collect_plugin_source() -> String {
    include_str!("app/settings_app_collect_plugin.rs").replace("\r\n", "\n")
}

fn settings_app_collect_group_source() -> String {
    include_str!("app/settings_app_collect_group.rs").replace("\r\n", "\n")
}

fn settings_app_collect_cloud_source() -> String {
    include_str!("app/settings_app_collect_cloud.rs").replace("\r\n", "\n")
}

fn settings_commands_source() -> String {
    [
        include_str!("app/settings_command_queue.rs"),
        include_str!("app/settings_timer_tasks.rs"),
        include_str!("app/settings_control_selection.rs"),
    ]
    .join("\n")
    .replace("\r\n", "\n")
}

fn settings_command_queue_source() -> String {
    include_str!("app/settings_command_queue.rs").replace("\r\n", "\n")
}

fn settings_timer_tasks_source() -> String {
    include_str!("app/settings_timer_tasks.rs").replace("\r\n", "\n")
}

fn settings_control_selection_source() -> String {
    include_str!("app/settings_control_selection.rs").replace("\r\n", "\n")
}

fn settings_control_selection_general_source() -> String {
    include_str!("app/settings_control_selection_general.rs").replace("\r\n", "\n")
}

fn settings_control_selection_cloud_source() -> String {
    include_str!("app/settings_control_selection_cloud.rs").replace("\r\n", "\n")
}

fn settings_control_selection_hotkey_source() -> String {
    include_str!("app/settings_control_selection_hotkey.rs").replace("\r\n", "\n")
}

fn settings_control_selection_plugin_source() -> String {
    include_str!("app/settings_control_selection_plugin.rs").replace("\r\n", "\n")
}

fn settings_control_selection_group_source() -> String {
    include_str!("app/settings_control_selection_group.rs").replace("\r\n", "\n")
}

fn settings_input_source() -> String {
    [
        include_str!("app/settings_pointer_input.rs"),
        include_str!("app/settings_keyboard_input.rs"),
        include_str!("app/settings_window_events.rs"),
        include_str!("app/settings_window_destroy.rs"),
        include_str!("app/settings_input.rs"),
    ]
    .join("\n")
    .replace("\r\n", "\n")
}

fn settings_input_dispatch_source() -> String {
    include_str!("app/settings_input.rs").replace("\r\n", "\n")
}

fn settings_pointer_input_source() -> String {
    include_str!("app/settings_pointer_input.rs").replace("\r\n", "\n")
}

fn settings_keyboard_input_source() -> String {
    include_str!("app/settings_keyboard_input.rs").replace("\r\n", "\n")
}

fn settings_window_events_source() -> String {
    include_str!("app/settings_window_events.rs").replace("\r\n", "\n")
}

fn settings_window_destroy_source() -> String {
    include_str!("app/settings_window_destroy.rs").replace("\r\n", "\n")
}

fn settings_plugin_sections_source() -> String {
    include_str!("app/settings_plugin_sections.rs").replace("\r\n", "\n")
}

fn settings_plugin_sections_controls_source() -> String {
    include_str!("app/settings_plugin_sections_controls.rs").replace("\r\n", "\n")
}

fn settings_plugin_sections_layout_source() -> String {
    include_str!("app/settings_plugin_sections_layout.rs").replace("\r\n", "\n")
}

fn settings_plugin_sections_providers_source() -> String {
    include_str!("app/settings_plugin_sections_providers.rs").replace("\r\n", "\n")
}

fn settings_plugin_sections_tools_source() -> String {
    include_str!("app/settings_plugin_sections_tools.rs").replace("\r\n", "\n")
}

fn settings_multi_sync_sections_source() -> String {
    include_str!("app/settings_multi_sync_sections.rs").replace("\r\n", "\n")
}

fn settings_group_sections_source() -> String {
    include_str!("app/settings_group_sections.rs").replace("\r\n", "\n")
}

fn settings_group_sections_cache_source() -> String {
    include_str!("app/settings_group_sections_cache.rs").replace("\r\n", "\n")
}

fn settings_group_sections_display_source() -> String {
    include_str!("app/settings_group_sections_display.rs").replace("\r\n", "\n")
}

fn settings_group_sections_list_source() -> String {
    include_str!("app/settings_group_sections_list.rs").replace("\r\n", "\n")
}

fn settings_group_page_source() -> String {
    include_str!("app/settings_group_page.rs").replace("\r\n", "\n")
}

fn settings_host_helpers_source() -> String {
    include_str!("app/settings_host_helpers.rs").replace("\r\n", "\n")
}

fn settings_general_page_source() -> String {
    include_str!("app/settings_general_page.rs").replace("\r\n", "\n")
}

fn settings_general_page_startup_source() -> String {
    include_str!("app/settings_general_page_startup.rs").replace("\r\n", "\n")
}

fn settings_general_page_window_source() -> String {
    include_str!("app/settings_general_page_window.rs").replace("\r\n", "\n")
}

fn settings_hotkey_page_source() -> String {
    include_str!("app/settings_hotkey_page.rs").replace("\r\n", "\n")
}

fn settings_hotkey_page_shortcuts_source() -> String {
    include_str!("app/settings_hotkey_page_shortcuts.rs").replace("\r\n", "\n")
}

fn settings_hotkey_page_system_source() -> String {
    include_str!("app/settings_hotkey_page_system.rs").replace("\r\n", "\n")
}

fn settings_plugin_page_source() -> String {
    include_str!("app/settings_plugin_page.rs").replace("\r\n", "\n")
}

fn settings_plugin_page_search_source() -> String {
    include_str!("app/settings_plugin_page_search.rs").replace("\r\n", "\n")
}

fn settings_plugin_page_ocr_translate_source() -> String {
    include_str!("app/settings_plugin_page_ocr_translate.rs").replace("\r\n", "\n")
}

fn settings_plugin_page_tools_source() -> String {
    include_str!("app/settings_plugin_page_tools.rs").replace("\r\n", "\n")
}

fn settings_about_page_source() -> String {
    include_str!("app/settings_about_page.rs").replace("\r\n", "\n")
}

fn settings_about_page_metadata_source() -> String {
    include_str!("app/settings_about_page_metadata.rs").replace("\r\n", "\n")
}

fn settings_about_page_update_source() -> String {
    include_str!("app/settings_about_page_update.rs").replace("\r\n", "\n")
}

fn settings_about_page_data_source() -> String {
    include_str!("app/settings_about_page_data.rs").replace("\r\n", "\n")
}

fn settings_cloud_page_source() -> String {
    include_str!("app/settings_cloud_page.rs").replace("\r\n", "\n")
}

fn settings_cloud_page_lan_devices_source() -> String {
    include_str!("app/settings_cloud_page_lan_devices.rs").replace("\r\n", "\n")
}

fn settings_cloud_page_webdav_source() -> String {
    include_str!("app/settings_cloud_page_webdav.rs").replace("\r\n", "\n")
}

fn settings_cloud_page_lan_source() -> String {
    include_str!("app/settings_cloud_page_lan.rs").replace("\r\n", "\n")
}

fn settings_owner_draw_source() -> String {
    include_str!("app/settings_owner_draw.rs").replace("\r\n", "\n")
}

fn settings_owner_draw_qr_source() -> String {
    include_str!("app/settings_owner_draw_qr.rs").replace("\r\n", "\n")
}

fn settings_owner_draw_link_source() -> String {
    include_str!("app/settings_owner_draw_link.rs").replace("\r\n", "\n")
}

fn settings_owner_draw_roles_source() -> String {
    include_str!("app/settings_owner_draw_roles.rs").replace("\r\n", "\n")
}

fn settings_page_builder_source() -> String {
    include_str!("app/settings_page_builder.rs").replace("\r\n", "\n")
}

fn settings_control_factory_source() -> String {
    include_str!("app/settings_control_factory.rs").replace("\r\n", "\n")
}

fn settings_control_registry_source() -> String {
    include_str!("app/settings_control_registry.rs").replace("\r\n", "\n")
}

fn settings_form_actions_source() -> String {
    include_str!("app/settings_form_actions.rs").replace("\r\n", "\n")
}

fn settings_form_fields_source() -> String {
    include_str!("app/settings_form_fields.rs").replace("\r\n", "\n")
}

fn settings_raw_controls_source() -> String {
    include_str!("app/settings_raw_controls.rs").replace("\r\n", "\n")
}

fn settings_page_navigation_source() -> String {
    include_str!("app/settings_page_navigation.rs").replace("\r\n", "\n")
}

fn settings_page_navigation_controls_source() -> String {
    include_str!("app/settings_page_navigation_controls.rs").replace("\r\n", "\n")
}

fn settings_page_navigation_scroll_source() -> String {
    include_str!("app/settings_page_navigation_scroll.rs").replace("\r\n", "\n")
}

fn settings_page_navigation_switch_source() -> String {
    include_str!("app/settings_page_navigation_switch.rs").replace("\r\n", "\n")
}

fn settings_page_ensure_source() -> String {
    include_str!("app/settings_page_ensure.rs").replace("\r\n", "\n")
}

fn settings_page_sync_source() -> String {
    include_str!("app/settings_page_sync.rs").replace("\r\n", "\n")
}

fn settings_page_sync_cloud_source() -> String {
    include_str!("app/settings_page_sync_cloud.rs").replace("\r\n", "\n")
}

fn settings_page_sync_cloud_webdav_source() -> String {
    include_str!("app/settings_page_sync_cloud_webdav.rs").replace("\r\n", "\n")
}

fn settings_page_sync_cloud_lan_source() -> String {
    include_str!("app/settings_page_sync_cloud_lan.rs").replace("\r\n", "\n")
}

fn settings_page_sync_plugin_source() -> String {
    include_str!("app/settings_page_sync_plugin.rs").replace("\r\n", "\n")
}

fn settings_state_source() -> String {
    include_str!("app/settings_state.rs").replace("\r\n", "\n")
}

fn settings_toggle_state_source() -> String {
    include_str!("app/settings_toggle_state.rs").replace("\r\n", "\n")
}

fn settings_toggle_state_general_source() -> String {
    include_str!("app/settings_toggle_state_general.rs").replace("\r\n", "\n")
}

fn settings_toggle_state_cloud_source() -> String {
    include_str!("app/settings_toggle_state_cloud.rs").replace("\r\n", "\n")
}

fn settings_toggle_state_hotkey_source() -> String {
    include_str!("app/settings_toggle_state_hotkey.rs").replace("\r\n", "\n")
}

fn settings_toggle_state_plugin_source() -> String {
    include_str!("app/settings_toggle_state_plugin.rs").replace("\r\n", "\n")
}

fn settings_toggle_state_group_source() -> String {
    include_str!("app/settings_toggle_state_group.rs").replace("\r\n", "\n")
}

fn settings_window_source() -> String {
    [
        include_str!("app/settings_window_metrics.rs"),
        include_str!("app/settings_window_layout.rs"),
        include_str!("app/settings_window_create.rs"),
        include_str!("app/settings_window_destroy.rs"),
        include_str!("app/settings_window.rs"),
        include_str!("app/settings_window_lifecycle.rs"),
        include_str!("app/settings_window_colors.rs"),
        include_str!("app/settings_window_owner_draw.rs"),
        include_str!("app/settings_window_paint.rs"),
    ]
    .join("\n")
    .replace("\r\n", "\n")
}

fn settings_window_proc_source() -> String {
    include_str!("app/settings_window.rs").replace("\r\n", "\n")
}

fn settings_window_layout_source() -> String {
    include_str!("app/settings_window_layout.rs").replace("\r\n", "\n")
}

fn settings_window_metrics_source() -> String {
    include_str!("app/settings_window_metrics.rs").replace("\r\n", "\n")
}

fn settings_window_create_source() -> String {
    include_str!("app/settings_window_create.rs").replace("\r\n", "\n")
}

fn settings_window_colors_source() -> String {
    include_str!("app/settings_window_colors.rs").replace("\r\n", "\n")
}

fn settings_window_surface_controls_source() -> String {
    include_str!("app/settings_window_surface_controls.rs").replace("\r\n", "\n")
}

fn settings_window_owner_draw_source() -> String {
    include_str!("app/settings_window_owner_draw.rs").replace("\r\n", "\n")
}

fn settings_window_paint_source() -> String {
    include_str!("app/settings_window_paint.rs").replace("\r\n", "\n")
}

fn settings_window_lifecycle_source() -> String {
    include_str!("app/settings_window_lifecycle.rs").replace("\r\n", "\n")
}

fn app_state_source() -> String {
    include_str!("app/state.rs").replace("\r\n", "\n")
}

fn app_state_runtime_source() -> String {
    include_str!("app/state_runtime.rs").replace("\r\n", "\n")
}

fn app_data_source() -> String {
    include_str!("app/data.rs").replace("\r\n", "\n")
}

fn app_prelude_source() -> String {
    include_str!("app/prelude.rs").replace("\r\n", "\n")
}

fn app_hosts_source() -> String {
    String::new()
}

fn app_constants_source() -> String {
    include_str!("app/constants.rs").replace("\r\n", "\n")
}

fn settings_dropdown_source() -> String {
    include_str!("app/settings_dropdown.rs").replace("\r\n", "\n")
}

fn settings_dropdown_general_source() -> String {
    include_str!("app/settings_dropdown_general.rs").replace("\r\n", "\n")
}

fn settings_dropdown_cloud_source() -> String {
    include_str!("app/settings_dropdown_cloud.rs").replace("\r\n", "\n")
}

fn settings_dropdown_hotkey_source() -> String {
    include_str!("app/settings_dropdown_hotkey.rs").replace("\r\n", "\n")
}

fn settings_dropdown_group_source() -> String {
    include_str!("app/settings_dropdown_group.rs").replace("\r\n", "\n")
}

fn settings_dropdown_host_source() -> String {
    include_str!("app/settings_dropdown_host.rs").replace("\r\n", "\n")
}

fn settings_dropdown_plugin_source() -> String {
    include_str!("app/settings_dropdown_plugin.rs").replace("\r\n", "\n")
}

fn main_clipboard_capture_source() -> String {
    include_str!("app/main_clipboard_capture.rs").replace("\r\n", "\n")
}

fn main_cloud_sync_source() -> String {
    include_str!("app/main_cloud_sync.rs").replace("\r\n", "\n")
}

fn main_edge_auto_hide_source() -> String {
    include_str!("app/main_edge_auto_hide.rs").replace("\r\n", "\n")
}

fn main_events_source() -> String {
    include_str!("app/main_events.rs").replace("\r\n", "\n")
}

fn main_hover_preview_source() -> String {
    include_str!("app/main_hover_preview.rs").replace("\r\n", "\n")
}

fn main_entry_source() -> String {
    include_str!("app/main_entry.rs").replace("\r\n", "\n")
}

fn main_lan_sync_source() -> String {
    include_str!("app/main_lan_sync.rs").replace("\r\n", "\n")
}

fn main_low_level_input_source() -> String {
    include_str!("app/main_low_level_input.rs").replace("\r\n", "\n")
}

fn main_row_commands_source() -> String {
    include_str!("app/main_row_commands.rs").replace("\r\n", "\n")
}

fn main_row_tools_source() -> String {
    include_str!("app/main_row_tools.rs").replace("\r\n", "\n")
}

fn main_paste_source() -> String {
    include_str!("app/main_paste.rs").replace("\r\n", "\n")
}

fn main_paste_target_discovery_source() -> String {
    include_str!("app/main_paste_target_discovery.rs").replace("\r\n", "\n")
}

fn main_search_source() -> String {
    include_str!("app/main_search.rs").replace("\r\n", "\n")
}

fn main_search_host_source() -> String {
    include_str!("app/main_search_host.rs").replace("\r\n", "\n")
}

fn main_startup_integrations_source() -> String {
    include_str!("app/main_startup_integrations.rs").replace("\r\n", "\n")
}

fn main_window_refresh_source() -> String {
    include_str!("app/main_window_refresh.rs").replace("\r\n", "\n")
}

fn main_window_host_source() -> String {
    include_str!("app/main_window_host.rs").replace("\r\n", "\n")
}

fn main_window_registry_source() -> String {
    include_str!("app/main_window_registry.rs").replace("\r\n", "\n")
}

fn transient_window_host_source() -> String {
    include_str!("app/transient_window_host.rs").replace("\r\n", "\n")
}

fn main_view_helpers_source() -> String {
    include_str!("app/main_view_helpers.rs").replace("\r\n", "\n")
}

fn main_window_source() -> String {
    include_str!("app/main_window.rs").replace("\r\n", "\n")
}

fn platform_helpers_source() -> String {
    include_str!("app/platform_helpers.rs").replace("\r\n", "\n")
}

fn main_platform_bindings_source() -> String {
    include_str!("app/main_platform_bindings.rs").replace("\r\n", "\n")
}

fn main_popup_menus_source() -> String {
    include_str!("app/main_popup_menus.rs").replace("\r\n", "\n")
}

fn vv_popup_source() -> String {
    include_str!("app/vv_popup.rs").replace("\r\n", "\n")
}

fn vv_hook_source() -> String {
    include_str!("app/vv_hook.rs").replace("\r\n", "\n")
}

#[test]
fn text_signature_uses_capture_normalization() {
    assert_eq!(
        text_content_signature("  hello\r\nworld  \u{200B}"),
        text_content_signature("hello\nworld")
    );
}

#[test]
fn clipboard_capture_allowed_follows_tray_toggle_setting() {
    let mut settings = AppSettings::default();
    assert!(clipboard_capture_allowed(&settings));

    settings.clipboard_capture_enabled = false;
    assert!(!clipboard_capture_allowed(&settings));
}

#[test]
fn file_signature_ignores_order_and_case() {
    let a = vec![r"C:\Temp\A.txt".to_string(), r"D:\Work\B.txt".to_string()];
    let b = vec![r"d:/work/b.txt".to_string(), r"c:/temp/a.txt".to_string()];
    assert_eq!(file_paths_signature(&a), file_paths_signature(&b));
}

#[test]
fn windows_screen_clip_paths_are_treated_as_image_payloads() {
    let shell_experience_paths = vec![
        r"C:\Users\me\AppData\Local\Packages\Microsoft.Windows.ShellExperienceHost_cw5n1h2txyewy\TempState\ScreenClip\{abc}.png".to_string(),
    ];
    let client_cbs_paths = vec![
        r"C:\Users\me\AppData\Local\Packages\MicrosoftWindows.Client.CBS_cw5n1h2txyewy\TempState\ScreenClip\{abc}.png".to_string(),
    ];
    assert!(paths_look_like_windows_screen_clip(&shell_experience_paths));
    assert!(paths_look_like_windows_screen_clip(&client_cbs_paths));
}

#[test]
fn windows_main_clipboard_capture_lives_outside_app_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let capture = main_clipboard_capture_source();

    assert!(app.contains("mod main_clipboard_capture;"));
    assert!(prelude.contains("use super::main_clipboard_capture::*;"));
    assert!(!app.contains("unsafe fn capture_clipboard("));
    assert!(!app.contains("fn browser_download_selection_should_skip("));
    assert!(!app.contains("fn paths_look_like_windows_screen_clip("));
    assert!(!app.contains("fn normalize_captured_text("));
    assert!(!app.contains("fn normalize_captured_image_rgba("));
    assert!(!app.contains("fn read_windows_clipboard_bitmap_rgba("));
    assert!(!app.contains("unsafe fn clipboard_source_app_name("));

    assert!(capture.contains("pub(super) unsafe fn capture_clipboard("));
    assert!(capture.contains("pub(super) fn browser_download_selection_should_skip("));
    assert!(capture.contains("pub(super) fn paths_look_like_windows_screen_clip("));
    assert!(capture.contains("pub(super) fn normalize_captured_text("));
    assert!(capture.contains("pub(super) fn normalize_captured_image_rgba("));
    assert!(capture.contains("fn read_windows_clipboard_bitmap_rgba("));
    assert!(capture.contains("unsafe fn clipboard_source_app_name("));
    assert!(capture.contains("WindowsClipboardHost::read_text"));
    assert!(capture.contains("WindowsClipboardHost::read_image_rgba"));
    assert!(capture.contains("WindowsClipboardHost::read_file_paths"));
    assert!(capture.contains("platform_clipboard::url_format_payloads"));
}

#[test]
fn ordinary_image_file_paths_are_not_screen_clips() {
    let paths = vec![r"C:\Users\me\Pictures\screenshot.png".to_string()];
    assert!(!paths_look_like_windows_screen_clip(&paths));
}

#[test]
fn image_signature_includes_dimensions() {
    let bytes = vec![0, 0, 0, 255, 255, 255, 255, 255];
    assert_ne!(
        image_content_signature(&bytes, 2, 1),
        image_content_signature(&bytes, 1, 2)
    );
}

#[test]
fn captured_bitmap_with_zero_alpha_is_normalized_opaque() {
    let bytes = vec![10, 20, 30, 0, 40, 50, 60, 0];
    let (normalized, width, height) = normalize_captured_image_rgba(bytes, 2, 1).unwrap();
    assert_eq!((width, height), (2, 1));
    assert_eq!(normalized, vec![10, 20, 30, 255, 40, 50, 60, 255]);
}

#[test]
fn quick_qr_builds_image_clip_from_text() {
    let item = text_item("https://example.test/path?q=zsclip");
    assert_eq!(
        main_row_external_action_plan(MainRowMenuAction::QrImage, Some(&item), &[]),
        Some(MainRowExternalActionPlan::QrText(
            "https://example.test/path?q=zsclip".to_string()
        ))
    );

    let (qr_item, sig) = build_qr_clip_item(item.text.as_deref().unwrap()).unwrap();

    assert_eq!(qr_item.kind, ClipKind::Image);
    assert!(qr_item.preview.contains("二维码") || qr_item.preview.contains("QR"));
    assert!(qr_item.image_width >= 128);
    assert_eq!(qr_item.image_width, qr_item.image_height);
    assert_eq!(
        qr_item.image_bytes.as_ref().unwrap().len(),
        qr_item.image_width * qr_item.image_height * 4
    );
    assert!(!sig.is_empty());
}

#[test]
fn load_16_bit_png_normalizes_to_rgba8() {
    let dir = std::env::temp_dir().join(format!(
        "zsclip-image-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("rgb16.png");
    {
        let file = fs::File::create(&path).unwrap();
        let mut encoder = png::Encoder::new(file, 2, 1);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Sixteen);
        let mut writer = encoder.write_header().unwrap();
        writer
            .write_image_data(&[
                0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0x00, 0x00,
            ])
            .unwrap();
    }

    let (bytes, width, height) = load_image_bytes_from_path(path.to_str().unwrap()).unwrap();
    assert_eq!((width, height), (2, 1));
    assert_eq!(bytes, vec![255, 0, 0, 255, 0, 255, 0, 255]);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn browser_download_selection_text_is_skipped() {
    assert!(browser_download_selection_should_skip(
        "chrome.exe",
        "chrome.exe",
        "report\nhttps://example.test/file.zip",
        &[]
    ));
    assert!(browser_download_selection_should_skip(
        "msedge.exe",
        "msedge.exe",
        "report",
        &["https://example.test/file.zip".to_string()]
    ));
    assert!(browser_download_selection_should_skip(
        "doubao.exe",
        "chrome.exe",
        "report",
        &["https://example.test/file.zip".to_string()]
    ));
    assert!(!browser_download_selection_should_skip(
        "chrome.exe",
        "chrome.exe",
        "https://example.test/file.zip",
        &["https://example.test/file.zip".to_string()]
    ));
    assert!(!browser_download_selection_should_skip(
        "notepad.exe",
        "notepad.exe",
        "report\nhttps://example.test/file.zip",
        &[]
    ));
}

#[test]
fn max_items_label_parser_rejects_empty_text() {
    assert_eq!(settings_dropdown_max_items_from_label_opt("500"), Some(500));
    assert_eq!(
        settings_dropdown_max_items_from_label_opt(settings_dropdown_label_for_max_items(0)),
        Some(0)
    );
    assert_eq!(settings_dropdown_max_items_from_label_opt(""), None);
}

#[test]
fn multi_sync_mode_is_exclusive() {
    let mut settings = AppSettings::default();
    settings_apply_multi_sync_mode(&mut settings, "webdav");
    assert!(settings.cloud_sync_enabled);
    assert!(!settings.lan_sync_enabled);

    settings_apply_multi_sync_mode(&mut settings, "lan");
    assert!(!settings.cloud_sync_enabled);
    assert!(settings.lan_sync_enabled);

    settings.cloud_sync_enabled = true;
    settings.lan_sync_enabled = true;
    settings_normalize_multi_sync_mode(&mut settings);
    assert!(!settings.cloud_sync_enabled);
    assert!(settings.lan_sync_enabled);

    settings_apply_multi_sync_mode(&mut settings, "off");
    assert!(!settings.cloud_sync_enabled);
    assert!(!settings.lan_sync_enabled);
}

#[test]
fn qq_vv_targets_do_not_backspace_existing_text() {
    assert_eq!(
        vv_backspace_count_for_target_identity("qq.exe", "", "", false),
        0
    );
    assert_eq!(
        vv_backspace_count_for_target_identity("QQNT.exe", "", "", false),
        0
    );
    assert_eq!(
        vv_backspace_count_for_target_identity("TIM.exe", "", "", false),
        0
    );
    assert_eq!(
        vv_backspace_count_for_target_identity("chrome.exe", "", "", false),
        0
    );
    assert_eq!(
        vv_backspace_count_for_target_identity("msedge.exe", "", "", false),
        0
    );
    assert_eq!(
        vv_backspace_count_for_target_identity("catsxp.exe", "", "", false),
        0
    );
    assert_eq!(
        vv_backspace_count_for_target_identity("", "catsxp.exe", "", false),
        0
    );
    assert_eq!(
        vv_backspace_count_for_target_identity("", "", "Chrome_WidgetWin_1", false),
        0
    );
    assert_eq!(
        vv_backspace_count_for_target_identity("", "", "Chrome_RenderWidgetHostHWND", false),
        0
    );
    assert_eq!(
        vv_backspace_count_for_target_identity("wps.exe", "", "", false),
        2
    );
    assert_eq!(
        vv_backspace_count_for_target_identity("notepad.exe", "", "", true),
        0
    );
    assert_eq!(
        vv_backspace_count_for_target_identity("notepad.exe", "", "", false),
        2
    );
}

#[test]
fn item_signature_prefers_computed_payload() {
    let item = text_item("same text");
    assert_eq!(
        dedupe_signature_for_item(&item, "legacy:signature"),
        text_content_signature("same text")
    );
}

#[test]
fn bdd_lan_latest_envelope_is_seeded_from_database_item_shape() {
    // Given the database latest row is a normal text clipboard item.
    // When LAN seeds /v1/latest from that row after service startup.
    // Then the envelope uses DB identity and the stable CRC signature.
    let mut settings = AppSettings::default();
    settings.lan_sync_enabled = true;
    settings.lan_device_id = "pc-1".to_string();
    let mut item = text_item(" hello ");
    item.id = 9;

    let envelope = lan_latest_envelope_from_item(&settings, &item, "").unwrap();

    assert_eq!(envelope.message_id, "pc-1-db-9");
    assert_eq!(envelope.origin_device_id, "pc-1");
    assert_eq!(envelope.origin_seq, 9);
    assert_eq!(envelope.kind, "text");
    assert_eq!(envelope.hash, text_content_signature("hello"));
    assert_eq!(envelope.text.as_deref(), Some(" hello "));
}

#[test]
fn bdd_lan_latest_envelope_restores_persisted_android_origin_from_database() {
    // Given Android pushed a LAN item and Windows persisted its original envelope identity.
    // When /v1/latest is rebuilt from the database.
    // Then the rebuilt envelope still identifies the Android device as the origin.
    let mut settings = AppSettings::default();
    settings.lan_sync_enabled = true;
    settings.lan_device_id = "pc-1".to_string();

    crate::db_runtime::with_test_db(|| {
        let item = text_item("hello from android");
        let id = db_insert_item(0, &item, Some("ignored"))?;
        db_save_lan_origin_metadata(
            id,
            &LanOriginMetadata {
                message_id: "android-1-42".to_string(),
                origin_device_id: "android-1".to_string(),
                origin_seq: 42,
                hash: "msg:android-1:42".to_string(),
            },
        )?;

        let (mut loaded, signature) = db_load_latest_item_with_signature(0).unwrap();
        loaded.id = id;
        let envelope = lan_latest_envelope_from_item(&settings, &loaded, &signature).unwrap();

        assert_eq!(envelope.message_id, "android-1-42");
        assert_eq!(envelope.origin_device_id, "android-1");
        assert_eq!(envelope.origin_seq, 42);
        assert_eq!(envelope.hash, "msg:android-1:42");
        assert_eq!(envelope.text.as_deref(), Some("hello from android"));
        Ok(())
    })
    .unwrap();
}

#[test]
fn bdd_lan_incoming_clip_preserves_origin_when_seeded_as_latest() {
    // Given Android pushes a LAN item to Windows.
    // When Windows stores it and updates /v1/latest.
    // Then the original LAN envelope identity is reused so Android can recognize its own item.
    let lan = main_lan_sync_source();
    let decoded = lan
        .split_once("struct LanDecodedClip")
        .and_then(|(_, rest)| {
            rest.split_once("fn lan_envelope_from_item")
                .map(|(block, _)| block)
        })
        .unwrap();
    let ready = lan
        .split_once("pub(super) unsafe fn handle_lan_sync_ready")
        .and_then(|(_, rest)| {
            rest.split_once("unsafe fn apply_lan_item_to_clipboard")
                .map(|(block, _)| block)
        })
        .unwrap();

    assert!(decoded.contains("latest_envelope: LanClipEnvelope"));
    assert!(lan.contains("let latest_envelope = envelope.clone();"));
    assert!(ready.contains("let latest_envelope = decoded.latest_envelope.clone();"));
    assert!(ready.contains("db_save_lan_origin_metadata"));
    assert!(ready.contains("lan_sync::set_latest_clip(Some(latest_envelope));"));
    assert!(!ready.contains("refresh_lan_latest_from_db(&state.settings);"));
}

#[test]
fn settings_window_buttons_map_to_stable_commands() {
    assert_eq!(
        settings_command_for_control(IDC_SET_SAVE),
        Some(Command::window(command_ids::SAVE_SETTINGS))
    );
    assert_eq!(
        settings_command_for_control(IDC_SET_CLOSE),
        Some(Command::window(command_ids::CLOSE_SETTINGS))
    );
    assert_eq!(
        settings_command_for_control(IDC_SET_BTN_OPENCFG),
        Some(Command::window(command_ids::OPEN_SETTINGS_CONFIG))
    );
    assert_eq!(
        settings_command_for_control(IDC_SET_MAX),
        Some(Command::window_with_payload(
            command_ids::OPEN_SETTINGS_DROPDOWN,
            CommandPayload::ControlId(IDC_SET_MAX as i64)
        ))
    );
    for control_id in [
        IDC_SET_MAX,
        IDC_SET_POSMODE,
        IDC_SET_CLOUD_INTERVAL,
        IDC_SET_MULTI_SYNC_MODE,
        IDC_SET_LAN_RECEIVE_MODE,
        6102,
        6103,
        IDC_SET_PASTE_SOUND_KIND,
        IDC_SET_PLAIN_HK_MOD,
        IDC_SET_PLAIN_HK_KEY,
        7201,
        IDC_SET_OCR_PROVIDER,
        IDC_SET_TRANSLATE_PROVIDER,
        IDC_SET_TRANSLATE_TARGET,
        IDC_SET_VV_SOURCE,
        IDC_SET_VV_GROUP,
    ] {
        assert_eq!(
            settings_command_for_control(control_id),
            Some(Command::window_with_payload(
                command_ids::OPEN_SETTINGS_DROPDOWN,
                CommandPayload::ControlId(control_id as i64)
            ))
        );
    }
    for control_id in [
        IDC_SET_AUTOSTART,
        IDC_SET_SILENTSTART,
        IDC_SET_TRAYICON,
        IDC_SET_CLOSETRAY,
        IDC_SET_CLICK_HIDE,
        IDC_SET_PASTE_MOVE_TOP,
        IDC_SET_DEDUPE_FILTER,
        IDC_SET_PERSIST_SEARCH,
        IDC_SET_PASTE_SOUND_ENABLE,
        IDC_SET_SKIP_WINDOW_ENABLE,
        IDC_SET_AUTOHIDE_BLUR,
        IDC_SET_EDGEHIDE,
        IDC_SET_HOVERPREVIEW,
        IDC_SET_VV_MODE,
        IDC_SET_IMAGE_PREVIEW,
        IDC_SET_QUICK_DELETE,
        IDC_SET_GROUP_ENABLE,
        IDC_SET_GROUP_TYPE_FILTER,
        IDC_SET_CLOUD_ENABLE,
        IDC_SET_LAN_ENABLE,
        6101,
        IDC_SET_PLAIN_HK_ENABLE,
        7102,
        7106,
        7101,
        7103,
        7104,
    ] {
        assert_eq!(
            settings_command_for_control(control_id),
            Some(Command::window_with_payload(
                command_ids::TOGGLE_SETTINGS_CONTROL,
                CommandPayload::ControlId(control_id as i64)
            ))
        );
    }
    assert_eq!(
        settings_page_to_sync_after_toggle(IDC_SET_PASTE_SOUND_ENABLE),
        Some(SettingsPage::General.index())
    );
    assert_eq!(
        settings_page_to_sync_after_toggle(IDC_SET_LAN_ENABLE),
        Some(SettingsPage::Cloud.index())
    );
    assert_eq!(
        settings_page_to_sync_after_toggle(7106),
        Some(SettingsPage::Plugin.index())
    );
    assert_eq!(settings_page_to_sync_after_toggle(IDC_SET_EDGEHIDE), None);
    assert_eq!(settings_command_for_control(IDC_SET_GROUP_ADD), None);
}

#[test]
fn main_menu_ids_map_to_stable_commands() {
    for id in [
        IDM_TRAY_TOGGLE,
        IDM_TRAY_LAN_TOGGLE,
        IDM_TRAY_CAPTURE_TOGGLE,
        IDM_TRAY_EXIT,
        IDM_ROW_PASTE,
        IDM_ROW_COPY,
        IDM_ROW_PIN,
        IDM_ROW_DELETE,
        IDM_ROW_DELETE_UNPINNED,
        IDM_ROW_TO_PHRASE,
        IDM_ROW_STICKER,
        IDM_ROW_SAVE_IMAGE,
        IDM_ROW_IMAGE_OCR,
        IDM_ROW_TEXT_TRANSLATE,
        IDM_ROW_QR_IMAGE,
        IDM_ROW_LAN_PUSH,
        IDM_ROW_OPEN_PATH,
        IDM_ROW_OPEN_FOLDER,
        IDM_ROW_COPY_PATH,
        IDM_ROW_GROUP_REMOVE,
        IDM_ROW_EDIT,
        IDM_ROW_QUICK_SEARCH,
        IDM_ROW_EXPORT_FILE,
        IDM_ROW_MAIL_MERGE,
        IDM_GROUP_FILTER_ALL,
        IDM_ROW_GROUP_BASE,
        IDM_ROW_GROUP_BASE + 1999,
        IDM_GROUP_FILTER_BASE,
        IDM_GROUP_FILTER_BASE + 1999,
    ] {
        assert_eq!(
            main_menu_command_for_id(id),
            Some(Command::window_with_payload(
                command_ids::INVOKE_MAIN_MENU_COMMAND,
                CommandPayload::ControlId(id as i64)
            ))
        );
    }

    assert_eq!(main_menu_command_for_id(IDC_SEARCH as usize), None);
    assert_eq!(main_menu_command_for_id(IDM_GROUP_FILTER_BASE + 2000), None);
}

#[test]
fn main_timer_ids_map_to_application_tasks() {
    let cases = [
        (ID_TIMER_STARTUP_RECOVERY, MainTimerTask::StartupRecovery),
        (ID_TIMER_VV_WATCH, MainTimerTask::VvWatch),
        (ID_TIMER_VV_SHOW, MainTimerTask::VvShow),
        (ID_TIMER_PASTE, MainTimerTask::Paste),
        (ID_TIMER_SEARCH_DEBOUNCE, MainTimerTask::SearchDebounce),
        (ID_TIMER_HIDDEN_RECLAIM, MainTimerTask::HiddenReclaim),
        (ID_TIMER_CLIPBOARD_RETRY, MainTimerTask::ClipboardRetry),
        (ID_TIMER_DPI_FIT, MainTimerTask::DpiFit),
        (ID_TIMER_SCROLL_FADE, MainTimerTask::ScrollFade),
        (ID_TIMER_EDGE_AUTO_HIDE, MainTimerTask::EdgeAutoHide),
        (ID_TIMER_OUTSIDE_HIDE, MainTimerTask::OutsideHide),
        (ID_TIMER_CLOUD_SYNC, MainTimerTask::CloudSync),
    ];
    for (timer_id, task) in cases {
        assert_eq!(main_timer_task_for_id(timer_id, MAIN_TIMER_IDS), Some(task));
    }

    assert_eq!(
        main_timer_task_for_id(ID_TIMER_SETTINGS_SCROLLBAR, MAIN_TIMER_IDS),
        None
    );
    assert_eq!(
        main_timer_task_for_id(ID_TIMER_SETTINGS_SAVE_HINT, MAIN_TIMER_IDS),
        None
    );
    assert_eq!(
        main_timer_task_for_id(ID_TIMER_SETTINGS_DPI_FIT, MAIN_TIMER_IDS),
        None
    );
    assert_eq!(main_timer_task_for_id(usize::MAX, MAIN_TIMER_IDS), None);
}

#[test]
fn windows_custom_messages_map_to_platform_neutral_application_events() {
    assert_eq!(
        main_application_event_from_window_message(WM_LAN_SYNC_READY, 0, 0),
        Some(UiEvent::Application(ApplicationEvent::LanSyncReady))
    );
    assert_eq!(
        main_application_event_from_window_message(WM_VV_SHOW, 42, 0),
        Some(UiEvent::Application(ApplicationEvent::VvShowRequested {
            target: NativeWindowToken(42)
        }))
    );
    assert_eq!(
        main_application_event_from_window_message(WM_VV_SELECT, 3, 0),
        Some(UiEvent::Application(ApplicationEvent::VvSelectRequested {
            index: 3
        }))
    );
    assert_eq!(
        main_application_event_from_window_message(WM_ITEMS_PAGE_READY, 0, 0),
        Some(UiEvent::Application(ApplicationEvent::ItemsPageReady))
    );
    assert_eq!(
        main_application_event_from_window_message(WM_STARTUP_DATA_RECONCILED, 7, 0),
        Some(UiEvent::Application(
            ApplicationEvent::StartupDataReconciled { deleted: 7 }
        ))
    );
    assert_eq!(
        main_application_event_from_window_message(WM_TRAYICON, 0, 0x0202),
        Some(UiEvent::Application(ApplicationEvent::TrayCallback {
            code: 0x0202
        }))
    );
    assert_eq!(
        main_application_event_from_window_message(taskbar_created_message(), 0, 0),
        Some(UiEvent::Application(
            ApplicationEvent::ShellIntegrationRestored
        ))
    );
    assert_eq!(
        main_application_event_from_window_message(WM_PAINT, 0, 0),
        None
    );
}

#[test]
fn startup_data_reconcile_does_not_block_main_window_creation() {
    let main_entry = main_entry_source();
    let data = include_str!("app/data.rs").replace("\r\n", "\n");
    let start = main_entry.find("unsafe fn on_create").unwrap();
    let end = main_entry[start..]
        .find("\npub(super) unsafe fn handle_control_command")
        .map(|offset| start + offset)
        .unwrap();
    let create_block = &main_entry[start..end];

    assert!(create_block.contains("reload_state_from_db_persisting(state)"));
    assert!(create_block.contains("spawn_startup_data_reconcile(hwnd"));
    assert!(!create_block.contains("db_reconcile_dedupe_signatures("));
    assert!(data.contains("pub(super) fn spawn_startup_data_reconcile"));
    assert!(data.contains("std::thread::spawn(move ||"));
    assert!(data.contains("WM_STARTUP_DATA_RECONCILED"));
    assert!(data.contains("CASE WHEN COALESCE(signature, '')='' THEN image_data ELSE NULL END"));
}

#[test]
fn windows_boxed_result_message_transfers_payload_ownership_once() {
    let payload = Box::new(TextOperationReadyResult {
        text: Some("hello".to_string()),
        error: None,
    });
    let raw = Box::into_raw(payload) as isize;
    let event = unsafe {
        take_main_async_event_from_window_message(WM_IMAGE_OCR_READY, raw)
            .expect("image OCR payload")
    };
    match event {
        MainAsyncEvent::ImageOcr(result) => {
            assert_eq!(result.text.as_deref(), Some("hello"));
            assert_eq!(result.error, None);
        }
        _ => panic!("unexpected async event"),
    }
    assert!(is_main_async_result_message(WM_IMAGE_OCR_READY));
    assert!(!is_main_async_result_message(WM_PAINT));
    assert!(unsafe { take_main_async_event_from_window_message(WM_IMAGE_OCR_READY, 0).is_none() });
}

#[test]
fn main_window_host_event_adapter_routes_async_and_ui_messages() {
    let payload = Box::new(TextOperationReadyResult {
        text: Some("hello".to_string()),
        error: None,
    });
    let raw = Box::into_raw(payload) as isize;
    let event = unsafe {
        main_window_host_event_from_message(WM_TEXT_TRANSLATE_READY, 0, raw).expect("async event")
    };
    match event {
        MainWindowHostEvent::Async(MainAsyncEvent::TextTranslate(result)) => {
            assert_eq!(result.text.as_deref(), Some("hello"));
            assert_eq!(result.error, None);
        }
        _ => panic!("unexpected async route"),
    }

    assert_eq!(
        unsafe { main_window_host_event_from_message(WM_LAN_SYNC_READY, 0, 0) }.map(|event| {
            match event {
                MainWindowHostEvent::Ui(event) => event,
                MainWindowHostEvent::Async(_) => panic!("unexpected async route"),
            }
        }),
        Some(UiEvent::Application(ApplicationEvent::LanSyncReady))
    );
}

#[test]
fn input_dialog_host_event_adapter_routes_commands_keys_and_close() {
    const INPUT_OK: usize = 9002;
    assert_eq!(
        input_dialog_host_event_from_message(WM_COMMAND, INPUT_OK, 0),
        Some(UiEvent::ControlCommand {
            control_id: INPUT_OK as u32,
            notification: 0,
        })
    );
    assert_eq!(
        input_dialog_host_event_from_message(WM_KEYDOWN, 0x0D, 0),
        Some(UiEvent::Key {
            code: 0x0D,
            state: UiKeyState::Down,
            system: false,
        })
    );
    assert_eq!(
        input_dialog_host_event_from_message(WM_CLOSE, 0, 0),
        Some(UiEvent::CloseRequested)
    );
    assert_eq!(input_dialog_host_event_from_message(WM_PAINT, 0, 0), None);
}

#[test]
fn edit_dialog_host_event_adapter_routes_commands_size_keys_and_close() {
    const EDIT_TEXTAREA: usize = 9010;
    assert_eq!(
        edit_dialog_host_event_from_message(
            WM_COMMAND,
            EDIT_TEXTAREA | ((EN_CHANGE_CODE as usize) << 16),
            0
        ),
        Some(UiEvent::ControlCommand {
            control_id: EDIT_TEXTAREA as u32,
            notification: EN_CHANGE_CODE,
        })
    );
    assert_eq!(
        edit_dialog_host_event_from_message(WM_SIZE, 0, 640 | (480 << 16)),
        Some(UiEvent::WindowSize {
            size: UiSize {
                width: 640,
                height: 480,
            },
            minimized: false,
        })
    );
    assert_eq!(
        edit_dialog_host_event_from_message(WM_KEYDOWN, 'S' as usize, 0),
        Some(UiEvent::Key {
            code: 'S' as u32,
            state: UiKeyState::Down,
            system: false,
        })
    );
    assert_eq!(
        edit_dialog_host_event_from_message(WM_CLOSE, 0, 0),
        Some(UiEvent::CloseRequested)
    );
    assert_eq!(edit_dialog_host_event_from_message(WM_PAINT, 0, 0), None);
}

#[test]
fn app_window_procs_do_not_decode_platform_ui_messages_directly() {
    let source = include_str!("app.rs");
    let forbidden = [
        ["platform_ui_event", "::", "from_window_message"].concat(),
        ["platform_ui_event", "::", "command_words"].concat(),
        ["platform_ui_event", "::", "size_from_lparam"].concat(),
    ];
    for forbidden in forbidden {
        assert!(
            !source.contains(&forbidden),
            "app.rs should route Windows message decoding through host adapters, found {forbidden}"
        );
    }
}

#[test]
fn platform_ui_message_decoding_stays_in_host_adapters() {
    fn collect_rs_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("read source dir") {
            let path = entry.expect("read source entry").path();
            if path.is_dir() {
                collect_rs_files(&path, out);
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = root.join("src");
    let allowed = [
        std::path::PathBuf::from("src/platform/ui_event.rs"),
        std::path::PathBuf::from("src/app/windows_messages.rs"),
        std::path::PathBuf::from("src/settings_ui_host.rs"),
        std::path::PathBuf::from("src/windows_edit_text_dialog.rs"),
        std::path::PathBuf::from("src/windows_text_input_dialog.rs"),
    ];
    let forbidden = [
        ["platform_ui_event", "::", "from_window_message"].concat(),
        ["platform_ui_event", "::", "command_words"].concat(),
        ["platform_ui_event", "::", "size_from_lparam"].concat(),
        ["use crate::platform::ui_event as ", "platform_ui_event"].concat(),
    ];
    let mut files = Vec::new();
    collect_rs_files(&src, &mut files);

    let mut violations = Vec::new();
    for file in files {
        let rel = file.strip_prefix(root).expect("source relative path");
        let rel_string = rel.to_string_lossy().replace('\\', "/");
        if allowed
            .iter()
            .any(|allowed| allowed.to_string_lossy().replace('\\', "/") == rel_string)
        {
            continue;
        }
        let text = std::fs::read_to_string(&file).expect("read source file");
        for forbidden in &forbidden {
            if text.contains(forbidden) {
                violations.push(format!("{rel_string}: {forbidden}"));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Windows UI message decoding should stay in platform/host adapters:\n{}",
        violations.join("\n")
    );
}

#[test]
fn windows_host_adapters_cover_required_ui_surfaces() {
    let app_messages = include_str!("app/windows_messages.rs");
    let settings_host = include_str!("settings_ui_host.rs");
    let edit_text_host = include_str!("windows_edit_text_dialog.rs");
    let text_input_host = include_str!("windows_text_input_dialog.rs");

    for surface in REQUIRED_UI_HOST_SURFACES {
        let adapter = surface.adapter_name();
        let implemented = match surface {
            UiHostSurface::MainWindow => app_messages.contains(adapter),
            UiHostSurface::EditDialog => edit_text_host.contains(adapter),
            UiHostSurface::InputDialog => text_input_host.contains(adapter),
            UiHostSurface::SettingsWindow | UiHostSurface::SettingsDropdown => {
                settings_host.contains(adapter)
            }
        };
        assert!(implemented, "missing Windows UI host adapter: {adapter}");
    }
}

#[test]
fn ui_host_porting_doc_covers_required_surfaces() {
    let doc = include_str!("../docs/ui-host-porting.md");
    let zsui_doc = include_str!("../docs/zsui.md");
    assert!(doc.contains(crate::app_core::ZSUI_FRAMEWORK_NAME));
    assert!(zsui_doc.contains(crate::app_core::ZSUI_FRAMEWORK_NAME));
    assert!(zsui_doc.contains(crate::app_core::ZSUI_FRAMEWORK_TAGLINE));
    assert!(zsui_doc.contains("Platform hosts own native windows"));
    assert!(zsui_doc.contains("Application-specific layers should stay outside ZSUI"));
    assert!(doc.contains("APP_CORE_API_VERSION"));
    let api_version = format!(
        "`{}.{}`",
        crate::app_core::APP_CORE_API_VERSION.major,
        crate::app_core::APP_CORE_API_VERSION.minor
    );
    assert!(
        doc.contains(&api_version),
        "porting doc must mention current app_core version {api_version}"
    );

    for surface in REQUIRED_UI_HOST_SURFACES {
        let surface_name = format!("{surface:?}");
        let adapter = surface.adapter_name();
        assert!(
            doc.contains(&surface_name),
            "porting doc must mention surface {surface_name}"
        );
        assert!(
            doc.contains(adapter),
            "porting doc must mention adapter {adapter}"
        );
    }

    for kind in REQUIRED_MAIN_HOST_EXECUTION_PLAN_KINDS {
        let plan_name = kind.plan_name();
        assert!(
            doc.contains(plan_name),
            "porting doc must mention main host execution plan {plan_name}"
        );
    }

    for operation in REQUIRED_NATIVE_STYLE_HOST_OPERATIONS {
        let operation_name = operation.operation_name();
        assert!(
            doc.contains(operation_name),
            "porting doc must mention native style host operation {operation_name}"
        );
    }

    for operation in REQUIRED_NATIVE_CONTROL_MAPPER_OPERATIONS {
        let operation_name = operation.operation_name();
        assert!(
            doc.contains(operation_name),
            "porting doc must mention native control mapper operation {operation_name}"
        );
    }

    for operation in REQUIRED_TEXT_LAYOUT_HOST_OPERATIONS {
        let operation_name = operation.operation_name();
        assert!(
            doc.contains(operation_name),
            "porting doc must mention text layout host operation {operation_name}"
        );
    }

    for operation in REQUIRED_RENDERER_HOST_OPERATIONS {
        let operation_name = operation.operation_name();
        assert!(
            doc.contains(operation_name),
            "porting doc must mention renderer host operation {operation_name}"
        );
    }

    for operation in REQUIRED_SETTINGS_CONTROL_HOST_OPERATIONS {
        let operation_name = operation.operation_name();
        assert!(
            doc.contains(operation_name),
            "porting doc must mention settings control host operation {operation_name}"
        );
    }

    for operation_name in [
        "read_text",
        "write_text",
        "read_image_rgba",
        "write_image_rgba",
        "read_file_paths",
        "write_file_paths",
        "sequence_number",
        "write_text_ignored_by_monitors",
        "should_ignore_capture_by_named_format",
    ] {
        assert!(
            doc.contains(operation_name),
            "porting doc must mention ClipboardHost method {operation_name}"
        );
    }

    for operation in REQUIRED_STATUS_ITEM_HOST_OPERATIONS {
        let operation_name = operation.operation_name();
        assert!(
            doc.contains(operation_name),
            "porting doc must mention status item host operation {operation_name}"
        );
    }

    for operation in REQUIRED_NATIVE_POPUP_MENU_HOST_OPERATIONS {
        let operation_name = operation.operation_name();
        assert!(
            doc.contains(operation_name),
            "porting doc must mention native popup menu host operation {operation_name}"
        );
    }

    for operation in REQUIRED_NATIVE_DIALOG_HOST_OPERATIONS {
        let operation_name = operation.operation_name();
        assert!(
            doc.contains(operation_name),
            "porting doc must mention native dialog host operation {operation_name}"
        );
    }

    for operation in REQUIRED_NATIVE_SHELL_OPEN_HOST_OPERATIONS {
        let operation_name = operation.operation_name();
        assert!(
            doc.contains(operation_name),
            "porting doc must mention native shell open host operation {operation_name}"
        );
    }

    for operation in REQUIRED_NATIVE_PASTE_TARGET_HOST_OPERATIONS {
        let operation_name = operation.operation_name();
        assert!(
            doc.contains(operation_name),
            "porting doc must mention native paste target host operation {operation_name}"
        );
    }

    for operation in REQUIRED_NATIVE_FILE_DIALOG_HOST_OPERATIONS {
        let operation_name = operation.operation_name();
        assert!(
            doc.contains(operation_name),
            "porting doc must mention native file dialog host operation {operation_name}"
        );
    }

    for operation in REQUIRED_NATIVE_TEXT_INPUT_DIALOG_HOST_OPERATIONS {
        let operation_name = operation.operation_name();
        assert!(
            doc.contains(operation_name),
            "porting doc must mention native text input dialog host operation {operation_name}"
        );
    }

    for operation in REQUIRED_NATIVE_EDIT_TEXT_DIALOG_HOST_OPERATIONS {
        let operation_name = operation.operation_name();
        assert!(
            doc.contains(operation_name),
            "porting doc must mention native edit text dialog host operation {operation_name}"
        );
    }

    for operation in REQUIRED_NATIVE_MAIL_MERGE_WINDOW_HOST_OPERATIONS {
        let operation_name = operation.operation_name();
        assert!(
            doc.contains(operation_name),
            "porting doc must mention native mail merge window host operation {operation_name}"
        );
    }

    for operation in REQUIRED_NATIVE_MAIN_WINDOW_HOST_OPERATIONS {
        let operation_name = operation.operation_name();
        assert!(
            doc.contains(operation_name),
            "porting doc must mention native main window host operation {operation_name}"
        );
    }

    for operation in REQUIRED_NATIVE_MAIN_SEARCH_CONTROL_HOST_OPERATIONS {
        let operation_name = operation.operation_name();
        assert!(
            doc.contains(operation_name),
            "porting doc must mention native main search control host operation {operation_name}"
        );
    }

    for operation in REQUIRED_NATIVE_SETTINGS_WINDOW_HOST_OPERATIONS {
        let operation_name = operation.operation_name();
        assert!(
            doc.contains(operation_name),
            "porting doc must mention native settings window host operation {operation_name}"
        );
    }

    for operation in REQUIRED_NATIVE_SETTINGS_DROPDOWN_HOST_OPERATIONS {
        let operation_name = operation.operation_name();
        assert!(
            doc.contains(operation_name),
            "porting doc must mention native settings dropdown host operation {operation_name}"
        );
    }

    for protocol in SHARED_NON_HOST_UI_PROTOCOLS {
        let protocol_name = protocol.protocol_name();
        assert!(
            doc.contains(protocol_name),
            "porting doc must mention shared non-host UI protocol {protocol_name}"
        );
    }
}

#[test]
fn windows_clipboard_host_implements_clipboard_trait_contract() {
    let app = include_str!("app.rs");
    let clipboard_host = include_str!("platform/clipboard.rs");
    let direct_constructor = ["Clipboard", "::new"].concat();

    assert!(
        !app.contains(&direct_constructor),
        "app.rs should consume WindowsClipboardHost instead of creating arboard clipboard instances directly"
    );
    assert!(
        clipboard_host.contains("pub(crate) struct WindowsClipboardHost"),
        "missing Windows clipboard host"
    );
    assert!(
        clipboard_host.contains("impl ClipboardHost for WindowsClipboardHost"),
        "Windows clipboard host should rely on the Rust trait contract"
    );
    assert!(
        clipboard_host.contains("fn read_text"),
        "missing clipboard text read"
    );
    assert!(
        clipboard_host.contains("fn write_text"),
        "missing clipboard text write"
    );
    assert!(
        clipboard_host.contains("fn read_image_rgba"),
        "missing clipboard image read"
    );
    assert!(
        clipboard_host.contains("fn write_image_rgba"),
        "missing clipboard image write"
    );
    assert!(
        clipboard_host.contains("pub(crate) fn file_paths"),
        "missing CF_HDROP read host"
    );
    assert!(
        clipboard_host.contains("pub(crate) fn set_file_paths"),
        "missing CF_HDROP write host"
    );
    assert!(
        clipboard_host.contains("pub(crate) fn sequence_number"),
        "missing clipboard sequence host"
    );
    assert!(
        clipboard_host.contains("pub(crate) fn set_text_ignored_by_monitors")
            && clipboard_host.contains("Clipboard Viewer Ignore")
            && clipboard_host.contains("ExcludeClipboardContentFromMonitorProcessing")
            && clipboard_host.contains("CanIncludeInClipboardHistory"),
        "missing monitor-ignore clipboard formats"
    );
}

#[test]
fn windows_status_item_host_owns_native_tray_menu_operations() {
    let tray = include_str!("../src/tray.rs");
    let status_host = include_str!("../src/platform/tray_icon.rs");

    assert!(status_host.contains("pub(crate) struct WindowsStatusItemHost"));
    assert!(status_host.contains("impl StatusItemHost for WindowsStatusItemHost"));
    assert!(status_host.contains("fn install(&mut self"));
    assert!(status_host.contains("fn remove(&mut self"));
    assert!(status_host.contains("fn present_menu(&mut self"));
    assert!(status_host.contains("menu::create_popup()"));
    assert!(status_host.contains("menu::track_popup_raw("));
    assert!(status_host.contains("icon_name"));
    assert!(status_host.contains("fn windows_status_menu_bitmap_for_icon_name("));
    assert!(status_host.contains("fn apply_status_menu_icon("));
    assert!(status_host.contains("SetMenuItemInfoW"));
    assert!(status_host.contains("MIIM_BITMAP"));
    assert!(status_host.contains("HBMMENU_POPUP_CLOSE"));
    assert!(status_host
        .contains("apply_status_menu_icon(popup, action.command_id() as u32, icon_name)"));
    let create_popup_call = ["platform_menu", "::", "create_popup"].concat();
    let track_popup_call = ["platform_menu", "::", "track_popup_raw"].concat();
    assert!(
        !tray.contains(&create_popup_call) && !tray.contains(&track_popup_call),
        "tray.rs should build shared entries instead of owning native menu rendering"
    );
    assert!(tray.contains("StatusMenuEntry::Command"));
    assert!(tray.contains("if spec.starts_section"));
    assert!(tray.contains("icon_name: spec.icon_name.to_string()"));
    assert!(!tray.contains("tray_action == MainTrayMenuAction::Exit"));
    assert!(tray.contains("WindowsStatusItemHost::new"));
}

#[test]
fn windows_popup_menu_host_owns_main_native_menu_operations() {
    let app = include_str!("app.rs");
    let main_popup_menus = main_popup_menus_source();
    let popup_host = include_str!("platform/menu.rs");

    assert!(popup_host.contains("pub(crate) struct WindowsPopupMenuHost"));
    assert!(popup_host.contains("impl NativePopupMenuHost for WindowsPopupMenuHost"));
    assert!(popup_host.contains("fn present_popup_menu("));
    assert!(popup_host.contains("NativePopupMenuEntry::Submenu"));
    assert!(popup_host.contains("CreatePopupMenu"));
    assert!(popup_host.contains("TrackPopupMenu"));
    let create_popup_call = ["platform_menu", "::", "create_popup"].concat();
    let append_raw_call = ["platform_menu", "::", "append_raw"].concat();
    let track_popup_call = ["platform_menu", "::", "track_popup_raw"].concat();
    assert!(
        !app.contains(&create_popup_call)
            && !app.contains(&append_raw_call)
            && !app.contains(&track_popup_call),
        "app.rs should build NativePopupMenuEntry values instead of owning Win32 menu rendering"
    );
    assert!(main_popup_menus.contains("native_host_full_row_popup_menu_entries_for_groups("));
    assert!(main_popup_menus.contains("NativeHostRowPopupMenuInput"));
    assert!(main_popup_menus.contains("WindowsPopupMenuHost::new().present_popup_menu"));
}

#[test]
fn windows_dialog_host_owns_row_action_message_boxes() {
    let platform_helpers = platform_helpers_source();
    let cloud = main_cloud_sync_source();
    let row_commands = main_row_commands_source();
    let dialog_host = include_str!("platform/dialog.rs");
    let product_dialog_sources = format!("{cloud}\n{row_commands}");

    assert!(dialog_host.contains("pub(crate) struct WindowsDialogHost"));
    assert!(dialog_host.contains("impl NativeDialogHost for WindowsDialogHost"));
    assert!(dialog_host.contains("fn show_message("));
    assert!(dialog_host.contains("MessageBoxW"));
    assert!(dialog_host.contains("native_host_dialog_button_specs"));
    assert!(!dialog_host.contains("native_host_dialog_component_specs"));
    assert!(!dialog_host.contains("NativeComponentAction::Dialog(dialog_action)"));
    assert!(platform_helpers.contains("WindowsDialogHost::new().show_message"));
    assert!(product_dialog_sources.contains("NativeDialogLevel::Info"));
    assert!(product_dialog_sources.contains("NativeDialogLevel::Error"));
}

#[test]
fn windows_dialog_host_owns_edit_close_confirmation() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let app_production = app
        .split("\n#[cfg(test)]\nmod tests")
        .next()
        .unwrap_or(&app);
    let dialog_host = include_str!("platform/dialog.rs");
    let edit_host = include_str!("windows_edit_text_dialog.rs");

    assert!(dialog_host.contains("fn confirm("));
    assert!(dialog_host.contains("NativeDialogButtons::YesNoCancel"));
    assert!(dialog_host.contains("NativeDialogResponse::Yes"));
    assert!(dialog_host.contains("IDYES"));
    assert!(dialog_host.contains("native_host_dialog_button_specs"));
    assert!(!dialog_host.contains("native_host_dialog_component_specs"));
    assert!(!dialog_host.contains("NativeComponentAction::Dialog(dialog_action)"));
    assert!(!app_production.contains("NativeDialogResponse::Yes => edit_dialog_save"));
    assert!(edit_host.contains("WindowsDialogHost::new().confirm"));
    assert!(edit_host.contains("native_host_edit_text_button_specs"));
    assert!(!edit_host.contains("native_host_edit_text_component_specs"));
    assert!(edit_host.contains("spec.action == NativeHostEditTextAction::Save"));
    assert!(edit_host.contains("spec.action == NativeHostEditTextAction::Cancel"));
    assert!(edit_host.contains("NativeDialogButtons::YesNoCancel"));
    assert!(edit_host.contains("NativeDialogResponse::Yes =>"));
    assert!(edit_host.contains("queue_save(hwnd, data, true)"));
    assert!(edit_host.contains("NativeDialogResponse::No => true"));
    assert!(edit_host.contains("NativeDialogResponse::Cancel => false"));
}

#[test]
fn windows_dialog_host_owns_main_paste_warning_messages() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let main_paste = main_paste_source();

    assert!(app.contains("mod main_paste;"));
    assert!(!app.contains("unsafe fn show_paste_failure_message"));
    assert!(!app.contains("unsafe fn show_clipboard_write_failure_message"));
    assert!(main_paste.contains("show_paste_failure_message"));
    assert!(main_paste.contains("show_clipboard_write_failure_message"));
    assert!(main_paste.contains("WindowsDialogHost::new().show_message"));
    assert!(main_paste.contains("NativeDialogLevel::Warning"));
    let direct_message_box = ["platform_dialog", "::", "message_box("].concat();
    assert!(
        !main_paste.contains(&format!(
            "{direct_message_box}\n        hwnd,\n        &text,"
        )),
        "paste failure warnings should use NativeDialogHost"
    );
}

#[test]
fn windows_dialog_host_owns_settings_sync_messages() {
    let platform_helpers = platform_helpers_source();
    let settings_actions = settings_actions_source();

    let start = settings_actions
        .find("unsafe fn execute_settings_sync_action")
        .unwrap();
    let end = settings_actions[start..]
        .find("\npub(super) unsafe fn execute_settings_group_action")
        .map(|offset| start + offset)
        .unwrap();
    let sync_block = &settings_actions[start..end];
    let direct_message_box = ["platform_dialog", "::", "message_box_wide"].concat();

    assert!(platform_helpers.contains("fn show_native_dialog_message"));
    assert!(sync_block.contains("show_native_dialog_message("));
    assert!(sync_block.contains("NativeDialogLevel::Info"));
    assert!(sync_block.contains("NativeDialogLevel::Warning"));
    assert!(
        !sync_block.contains(&direct_message_box),
        "settings sync OK prompts should use NativeDialogHost"
    );
}

#[test]
fn windows_dialog_host_owns_settings_group_messages() {
    let platform_helpers = platform_helpers_source();
    let settings_actions = settings_actions_source();

    let start = settings_actions
        .find("unsafe fn execute_settings_group_action")
        .unwrap();
    let end = settings_actions[start..]
        .find("\npub(super) unsafe fn execute_settings_platform_action")
        .map(|offset| start + offset)
        .unwrap();
    let group_block = &settings_actions[start..end];
    let direct_message_box = ["platform_dialog", "::", "message_box_wide"].concat();

    assert!(platform_helpers.contains("fn confirm_native_dialog"));
    assert!(group_block.contains("show_native_dialog_message("));
    assert!(group_block.contains("confirm_native_dialog("));
    assert!(group_block.contains("NativeDialogLevel::Error"));
    assert!(group_block.contains("NativeDialogButtons::YesNo"));
    assert!(group_block.contains("NativeDialogResponse::Yes"));
    assert!(
        !group_block.contains(&direct_message_box),
        "settings group prompts should use NativeDialogHost"
    );
}

#[test]
fn windows_dialog_host_owns_settings_platform_messages() {
    let settings_actions = settings_actions_source();

    let start = settings_actions
        .find("unsafe fn execute_settings_platform_action")
        .unwrap();
    let platform_block = &settings_actions[start..];
    let direct_message_box = ["platform_dialog", "::", "message_box_wide"].concat();

    assert!(platform_block.contains("show_native_dialog_message("));
    assert!(platform_block.contains("NativeDialogLevel::Info"));
    assert!(platform_block.contains("NativeDialogLevel::Error"));
    assert!(
        !platform_block.contains(&direct_message_box),
        "settings platform/plugin prompts should use NativeDialogHost"
    );
}

#[test]
fn windows_dialog_host_owns_non_host_message_boxes() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let platform_helpers = platform_helpers_source();
    let settings_actions = settings_actions_source();
    let sticker = include_str!("sticker.rs").replace("\r\n", "\n");
    let dialog_host = include_str!("platform/dialog.rs").replace("\r\n", "\n");
    let direct_message_box = ["platform_dialog", "::", "message_box("].concat();
    let direct_message_box_wide = ["platform_dialog", "::", "message_box_wide"].concat();

    assert!(
        !app.contains(&direct_message_box)
            && !app.contains(&direct_message_box_wide)
            && !settings_actions.contains(&direct_message_box)
            && !settings_actions.contains(&direct_message_box_wide),
        "Windows app UI modules should not call old platform_dialog message boxes directly"
    );
    assert!(
        !sticker.contains(&direct_message_box) && !sticker.contains(&direct_message_box_wide),
        "sticker.rs should not call old platform_dialog message boxes directly"
    );
    assert!(!dialog_host.contains("pub(crate) fn message_box("));
    assert!(!dialog_host.contains("message_box_wide"));
    assert!(platform_helpers.contains("show_native_dialog_message("));
    assert!(sticker.contains("WindowsDialogHost::new().show_message"));
}

#[test]
fn windows_shell_open_host_owns_shell_execute_operations() {
    let shell = include_str!("platform/shell.rs");
    let app_shell = include_str!("shell.rs");

    assert!(shell.contains("pub(crate) struct WindowsShellOpenHost"));
    assert!(shell.contains("impl NativeShellOpenHost for WindowsShellOpenHost"));
    assert!(shell.contains("ShellExecuteW"));
    assert!(app_shell.contains("WindowsShellOpenHost::new().open_path(path)"));
    assert!(!shell.contains("pub(crate) fn open_path(path: &str)"));
    assert!(!app_shell.contains("platform_shell::open_path(path)"));
}

#[test]
fn windows_paste_target_host_owns_foreground_and_focus_restore() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let main_events = main_events_source();
    let vv_hook = vv_hook_source();
    let main_paste = main_paste_source();
    let production = app
        .split("\n#[cfg(test)]\nmod tests")
        .next()
        .unwrap_or(&app);
    let paste_target_host = include_str!("platform/paste_target.rs").replace("\r\n", "\n");
    let timer_start = main_events.find("MainTimerTask::Paste =>").unwrap();
    let timer_end = main_events[timer_start..]
        .find("\n        MainTimerTask::SearchDebounce")
        .map(|offset| timer_start + offset)
        .unwrap();
    let timer_block = &main_events[timer_start..timer_end];
    let restore_start = main_paste
        .find("unsafe fn restore_hotkey_focus_target")
        .unwrap();
    let restore_end = main_paste[restore_start..]
        .find("\npub(super) unsafe fn can_send_ctrl_v_to_target")
        .map(|offset| restore_start + offset)
        .unwrap();
    let restore_block = &main_paste[restore_start..restore_end];
    let ready_start = main_paste
        .find("unsafe fn paste_after_clipboard_ready_to_target")
        .unwrap();
    let ready_block = &main_paste[ready_start..];
    let direct_edit_start = main_paste
        .find("unsafe fn try_apply_to_explorer_rename")
        .unwrap();
    let direct_edit_end = main_paste[direct_edit_start..]
        .find("\nunsafe fn maybe_promote_pasted_item")
        .map(|offset| direct_edit_start + offset)
        .unwrap();
    let direct_edit_block = &main_paste[direct_edit_start..direct_edit_end];
    let async_image_start = main_paste
        .find("unsafe fn queue_async_image_paste_if_needed")
        .unwrap();
    let async_image_end = main_paste[async_image_start..]
        .find("\npub(super) unsafe fn try_apply_to_explorer_rename")
        .map(|offset| async_image_start + offset)
        .unwrap();
    let async_image_block = &main_paste[async_image_start..async_image_end];
    let target_ready_start = vv_hook
        .find("unsafe fn vv_target_is_text_input_ready")
        .unwrap();
    let target_ready_end = vv_hook[target_ready_start..]
        .find("\nunsafe extern \"system\" fn vv_keyboard_hook_proc")
        .map(|offset| target_ready_start + offset)
        .unwrap();
    let target_ready_block = &vv_hook[target_ready_start..target_ready_end];

    assert!(
        timer_block.contains("WindowsPasteTargetHost::new().force_paste_target_foreground(target)")
    );
    assert!(production.contains("mod main_paste;"));
    assert!(!production.contains("unsafe fn paste_after_clipboard_ready_to_target"));
    assert!(!production.contains("unsafe fn try_apply_to_explorer_rename"));
    assert!(!production.contains("unsafe fn queue_async_image_paste_if_needed"));
    assert!(restore_block
        .contains("WindowsPasteTargetHost::new().restore_paste_target_focus(target, focus)"));
    assert!(
        ready_block.contains("WindowsPasteTargetHost::new().force_paste_target_foreground(target)")
    );
    assert!(async_image_block.contains("WindowsWindowIdentityHost::new().exists(target)"));
    assert!(direct_edit_block
        .contains("WindowsWindowIdentityHost::new().exists(state.hotkey_passthrough_edit)"));
    assert!(
        direct_edit_block.contains("set_paste_target_text(state.hotkey_passthrough_edit, &text)")
    );
    assert!(!production.contains("unsafe fn vv_target_is_text_input_ready"));
    assert!(target_ready_block.contains("paste_target_text_input_ready(target)"));
    assert!(!timer_block.contains("platform_window::force_foreground(target)"));
    assert!(!restore_block.contains("platform_input::set_focus(focus)"));
    assert!(!ready_block.contains("platform_window::force_foreground(target)"));
    assert!(!direct_edit_block.contains("WM_SETTEXT"));
    assert!(!direct_edit_block.contains("EM_SETSEL"));
    assert!(!direct_edit_block.contains("platform_window::send_message("));
    assert!(!direct_edit_block.contains("platform_window::exists("));
    assert!(!async_image_block.contains("platform_window::exists("));
    assert!(!target_ready_block.contains("WM_GETDLGCODE"));
    assert!(!target_ready_block.contains("DLGC_"));
    assert!(!target_ready_block.contains("platform_input::default_ime_window"));
    assert!(!target_ready_block.contains("GUITHREADINFO"));
    assert!(!target_ready_block.contains("platform_window::gui_thread_info"));
    assert!(!target_ready_block.contains("platform_window::window_thread_id"));
    assert!(!target_ready_block.contains("platform_accessibility::caret_rect"));
    assert!(!target_ready_block.contains("WindowsImeHost::new()"));
    assert!(paste_target_host.contains("pub(crate) struct WindowsPasteTargetHost"));
    assert!(paste_target_host.contains("impl NativePasteTargetHost for WindowsPasteTargetHost"));
    assert!(paste_target_host.contains("platform_window::force_foreground(target)"));
    assert!(paste_target_host.contains("platform_input::set_focus(focus)"));
    assert!(paste_target_host.contains("fn set_paste_target_text"));
    assert!(paste_target_host.contains("fn paste_target_text_input_capabilities"));
    assert!(paste_target_host.contains("fn paste_target_text_input_ready"));
    assert!(paste_target_host.contains("platform_window::gui_thread_info"));
    assert!(paste_target_host.contains("platform_accessibility::caret_rect"));
    assert!(paste_target_host.contains("WM_GETDLGCODE"));
    assert!(paste_target_host.contains("DLGC_WANTCHARS"));
    assert!(paste_target_host.contains("WM_SETTEXT"));
    assert!(paste_target_host.contains("EM_SETSEL"));
}

#[test]
fn quick_window_explorer_rename_detection_checks_focus_and_caret_edit() {
    let tray = include_str!("tray.rs").replace("\r\n", "\n");
    let helper_start = tray
        .find("unsafe fn explorer_rename_edit_from_focus_or_caret")
        .unwrap();
    let helper_end = tray[helper_start..]
        .find("\nunsafe fn foreground_focus_snapshot")
        .map(|offset| helper_start + offset)
        .unwrap();
    let helper = &tray[helper_start..helper_end];

    assert!(tray.contains("explorer_rename_edit_from_focus_or_caret(fg, focus)"));
    assert!(helper.contains("matches!(window_class_name(focus).as_str(), \"Edit\")"));
    assert!(helper.contains("platform_window::gui_thread_info(thread_id, &mut info)"));
    assert!(helper.contains("for candidate in [info.hwndFocus, info.hwndCaret]"));
    assert!(helper.contains("platform_window::root_ancestor(candidate) != fg"));
    assert!(helper.contains("matches!(window_class_name(candidate).as_str(), \"Edit\")"));
}

#[test]
fn windows_file_dialog_host_owns_open_file_dialog_operations() {
    let file_dialog = include_str!("platform/file_dialog.rs");
    let app_shell = include_str!("shell.rs");

    assert!(file_dialog.contains("pub(crate) struct WindowsFileDialogHost"));
    assert!(file_dialog.contains("impl NativeFileDialogHost for WindowsFileDialogHost"));
    assert!(file_dialog.contains("System.Windows.Forms.OpenFileDialog"));
    assert!(app_shell.contains("WindowsFileDialogHost::new().pick_file"));
    assert!(app_shell.contains("NativeFileDialogRequest"));
    assert!(!app_shell.contains("New-Object System.Windows.Forms.OpenFileDialog"));
    assert!(!app_shell.contains("run_hidden_powershell_encoded"));
}

#[test]
fn windows_file_dialog_host_owns_mail_merge_excel_picker() {
    let mail_merge = include_str!("mail_merge_native.rs");

    assert!(mail_merge.contains("WindowsFileDialogHost::new().pick_file"));
    assert!(mail_merge.contains("NativeFileDialogRequest"));
    assert!(mail_merge.contains("filter_pattern: \"*.xlsx;*.xls;*.xlsm;*.csv\""));
    assert!(
        mail_merge.contains("WindowsPasteTargetHost::new().force_paste_target_foreground(target)")
    );
    assert!(!mail_merge.contains("platform_window::force_foreground(target)"));
    assert!(!mail_merge.contains("fn ps_pick_excel_file"));
    assert!(!mail_merge.contains("New-Object System.Windows.Forms.OpenFileDialog"));
}

#[test]
fn windows_text_input_dialog_host_owns_group_name_prompts() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let settings_actions = settings_actions_source();
    let host = include_str!("windows_text_input_dialog.rs").replace("\r\n", "\n");
    let app_production = app
        .split("\n#[cfg(test)]\nmod tests")
        .next()
        .unwrap_or(&app);

    let start = settings_actions
        .find("unsafe fn execute_settings_group_action")
        .unwrap();
    let end = settings_actions[start..]
        .find("\npub(super) unsafe fn execute_settings_platform_action")
        .map(|offset| start + offset)
        .unwrap();
    let group_block = &settings_actions[start..end];

    assert!(prelude.contains("use crate::windows_text_input_dialog::WindowsTextInputDialogHost;"));
    assert!(!app_production.contains("struct WindowsTextInputDialogHost"));
    assert!(!app_production.contains("struct InputDlgData"));
    assert!(!app_production.contains("input_name_dialog"));
    assert!(host.contains("pub(crate) struct WindowsTextInputDialogHost"));
    assert!(host.contains("impl NativeTextInputDialogHost for WindowsTextInputDialogHost"));
    assert!(host.contains("unsafe extern \"system\" fn input_dialog_proc"));
    assert!(group_block.contains("settings_group_text_input_request("));
    assert!(group_block.contains("SettingsGroupTextInputKind::Add"));
    assert!(group_block.contains("SettingsGroupTextInputKind::Rename"));
    assert!(group_block.contains("WindowsTextInputDialogHost::new().prompt_text"));
    assert!(!group_block.contains("input_name_dialog("));
    assert!(!group_block.contains("NativeTextInputDialogRequest {"));
}

#[test]
fn windows_edit_text_dialog_host_owns_row_edit_action() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let main_row_commands = main_row_commands_source();
    let host = include_str!("windows_edit_text_dialog.rs").replace("\r\n", "\n");
    let app_production = app
        .split("\n#[cfg(test)]\nmod tests")
        .next()
        .unwrap_or(&app);
    let host_production = host
        .split("\n#[cfg(test)]\nmod tests")
        .next()
        .unwrap_or(&host);

    let start = main_row_commands
        .find("unsafe fn execute_row_dialog_action")
        .unwrap();
    let end = main_row_commands[start..]
        .find("\nunsafe fn execute_row_current_item_action")
        .map(|offset| start + offset)
        .unwrap();
    let action_block = &main_row_commands[start..end];

    assert!(prelude.contains("use crate::windows_edit_text_dialog::WindowsEditTextDialogHost;"));
    assert!(!app_production.contains("struct WindowsEditTextDialogHost"));
    assert!(!app_production.contains("struct EditDlgData"));
    assert!(!app_production.contains("show_edit_item_dialog"));
    assert!(host_production.contains("pub(crate) struct WindowsEditTextDialogHost"));
    assert!(host_production.contains("impl NativeEditTextDialogHost for WindowsEditTextDialogHost"));
    assert!(host_production.contains("native_host_edit_text_button_specs"));
    assert!(!host_production.contains("native_host_edit_text_component_specs"));
    assert!(host_production.contains("spec.action == NativeHostEditTextAction::Save"));
    assert!(host_production.contains("spec.action == NativeHostEditTextAction::Cancel"));
    assert!(!host_production.contains("db_update_item_text"));
    assert!(!host_production.contains("with_db("));
    assert!(!host_production.contains("load_settings("));
    assert!(action_block.contains("WindowsEditTextDialogHost::new().open_edit_text"));
    assert!(action_block.contains("NativeEditTextDialogRequest"));
    assert!(action_block.contains("initial_text: &initial_text"));
    assert!(action_block.contains("&mut save_handler"));
    assert!(!action_block.contains("show_edit_item_dialog("));
}

#[test]
fn windows_settings_window_open_path_uses_settings_ui_host() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let settings_window = settings_window_source();
    let production = app
        .split("\n#[cfg(test)]\nmod tests")
        .next()
        .unwrap_or(&app);
    let start = settings_window
        .find("unsafe fn open_settings_window")
        .unwrap();
    let end = settings_window[start..]
        .find("\npub(super) fn set_settings_window_bounds")
        .map(|offset| start + offset)
        .unwrap();
    let open_block = &settings_window[start..end];

    assert!(open_block.contains("WindowsSettingsWindowHost::new"));
    assert!(open_block.contains("present_settings_window("));
    assert!(open_block.contains("NativeSettingsWindowRequest"));
    assert!(open_block.contains("NativeSettingsWindowPresentation::Created"));
    assert!(!open_block.contains("WindowsSettingsWindowRequest"));
    assert!(!open_block.contains("WindowsSettingsWindowPresentation::Created"));
    assert!(!open_block.contains("register_class_ex"));
    assert!(!open_block.contains("create_window_ex"));
    assert!(!open_block.contains("SETTINGS_CLASS"));
    assert!(!production.contains("unsafe fn ensure_settings_class"));
}

#[test]
fn windows_settings_window_bounds_updates_use_settings_host() {
    let settings_window = settings_window_source();
    let settings_input = settings_input_source();
    let settings_host = include_str!("settings_ui_host.rs").replace("\r\n", "\n");
    let dpi_transition_start = settings_window
        .find("unsafe fn resize_settings_window_for_dpi_transition")
        .unwrap();
    let work_area_start = settings_window
        .find("unsafe fn ensure_settings_window_in_work_area")
        .unwrap();
    let reset_start = settings_window
        .find("unsafe fn reset_settings_dpi_compensation")
        .unwrap();
    let base_start = settings_window
        .find("unsafe fn update_settings_dpi_compensation_base")
        .unwrap();
    let compensation_start = settings_window
        .find("unsafe fn apply_settings_system_dpi_compensation")
        .unwrap();
    let suggested_start = settings_input
        .find("unsafe fn apply_dpi_suggested_rect")
        .unwrap();
    let dpi_changed_start = settings_input
        .find("unsafe fn handle_settings_dpi_changed")
        .unwrap();
    let compensation_end = settings_window[compensation_start..]
        .find("\npub(super) unsafe extern \"system\" fn settings_wnd_proc")
        .map(|offset| compensation_start + offset)
        .unwrap();
    let dpi_transition_block = &settings_window[dpi_transition_start..work_area_start];
    let work_area_block = &settings_window[work_area_start..reset_start];
    let base_block = &settings_window[base_start..compensation_start];
    let compensation_block = &settings_window[compensation_start..compensation_end];
    let suggested_block = &settings_input[suggested_start..dpi_changed_start];

    for block in [
        dpi_transition_block,
        work_area_block,
        compensation_block,
        suggested_block,
    ] {
        assert!(block.contains("set_settings_window_bounds("));
        assert!(!block.contains("platform_window::set_pos("));
    }
    assert!(settings_host.contains("fn set_settings_window_bounds(&mut self"));
    assert!(settings_host.contains("platform_window::set_pos("));
    for block in [
        dpi_transition_block,
        work_area_block,
        base_block,
        compensation_block,
    ] {
        assert!(block.contains("settings_window_bounds(hwnd)"));
        assert!(!block.contains("platform_window::window_rect(hwnd)"));
    }
}

#[test]
fn windows_settings_window_layout_dpi_uses_settings_host() {
    let settings_window = settings_window_source();
    let settings_create = settings_window_create_source();
    let settings_state = settings_state_source();
    let settings_input = settings_input_source();
    let settings_paint = settings_window_paint_source();
    let settings_host = include_str!("settings_ui_host.rs").replace("\r\n", "\n");
    let refresh_start = settings_window
        .find("unsafe fn refresh_settings_window_metrics")
        .unwrap();
    let resize_start = settings_window[refresh_start..]
        .find("\npub(super) unsafe fn resize_settings_window_for_dpi_transition")
        .map(|offset| refresh_start + offset)
        .unwrap();
    let refresh_block = &settings_window[refresh_start..resize_start];
    let system_start = settings_input
        .find("unsafe fn handle_settings_system_metrics_changed")
        .unwrap();
    let move_start = settings_input
        .find("unsafe fn handle_settings_window_move_completed")
        .unwrap();
    let destroy_start = settings_input
        .find("unsafe fn handle_settings_destroy")
        .unwrap();
    let system_block = &settings_input[system_start..move_start];
    let move_block = &settings_input[move_start..destroy_start];
    let open_start = settings_window
        .find("unsafe fn open_settings_window")
        .unwrap();
    let open_end = settings_window[open_start..]
        .find("\npub(super) fn set_settings_window_bounds")
        .map(|offset| open_start + offset)
        .unwrap();
    let open_block = &settings_window[open_start..open_end];

    assert!(refresh_block.contains("settings_window_layout_dpi(hwnd).max(96)"));
    assert!(system_block.contains("settings_window_layout_dpi(hwnd).max(96)"));
    assert!(move_block.contains("settings_window_layout_dpi(hwnd).max(96)"));
    assert!(settings_create.contains("set_settings_ui_dpi(settings_window_layout_dpi(hwnd))"));
    assert!(settings_create.contains("settings_window_layout_dpi(hwnd).max(96)"));
    assert!(settings_state.contains("ui_dpi,"));
    assert!(settings_paint.contains("let paint_dpi = settings_window_layout_dpi(hwnd)"));
    assert!(open_block.contains("settings_window_layout_dpi(app.settings_hwnd).max(96)"));
    assert!(open_block.contains("let old_dpi = (*st_ptr).ui_dpi.max(96)"));
    assert!(open_block.contains(
        "resize_settings_window_for_dpi_transition(app.settings_hwnd, old_dpi, next_dpi)"
    ));
    for block in [
        refresh_block,
        system_block,
        move_block,
        settings_create.as_str(),
    ] {
        assert!(!block.contains("platform_dpi::layout_dpi_for_window(hwnd)"));
    }
    assert!(!open_block.contains("platform_dpi::layout_dpi_for_window(app.settings_hwnd)"));
    assert!(settings_host.contains("fn settings_window_layout_dpi("));
    assert!(settings_host.contains("platform_dpi::layout_dpi_for_window(handle)"));
}

#[test]
fn windows_settings_control_bounds_and_visibility_use_control_host() {
    let settings_window = settings_window_source();
    let settings_host = include_str!("settings_ui_host.rs").replace("\r\n", "\n");
    let refresh_start = settings_window
        .find("unsafe fn refresh_settings_window_metrics")
        .unwrap();
    let resize_start = settings_window[refresh_start..]
        .find("\npub(super) unsafe fn resize_settings_window_for_dpi_transition")
        .map(|offset| refresh_start + offset)
        .unwrap();
    let refresh_block = &settings_window[refresh_start..resize_start];

    assert!(refresh_block.contains("settings_host_set_bounds("));
    assert!(refresh_block.contains("settings_host_set_visible("));
    assert!(refresh_block.contains("settings_host_exists("));
    assert!(refresh_block.contains("settings_window_client_bounds(hwnd)"));
    assert!(refresh_block.contains("repaint_settings_window(hwnd, true)"));
    assert!(!refresh_block.contains("platform_window::move_window("));
    assert!(!refresh_block.contains("platform_window::set_visible("));
    assert!(!refresh_block.contains("platform_window::exists("));
    assert!(!refresh_block.contains("platform_window::client_rect(hwnd)"));
    assert!(!refresh_block.contains("platform_gdi::invalidate_rect(hwnd, null(), 1)"));
    assert!(settings_host.contains("fn set_control_bounds(&mut self"));
    assert!(settings_host.contains("fn set_control_visible(&mut self"));
    assert!(settings_host.contains("fn request_settings_window_area_repaint("));
}

#[test]
fn windows_settings_control_text_reads_use_control_host() {
    let main_entry = main_entry_source();
    let settings_actions = settings_actions_source();
    let settings_commands = settings_commands_source();
    let settings_dropdown = settings_dropdown_source();
    let product_sources =
        format!("{main_entry}\n{settings_actions}\n{settings_commands}\n{settings_dropdown}");
    let settings_host = include_str!("settings_ui_host.rs").replace("\r\n", "\n");

    assert!(product_sources.contains("settings_host_text("));
    assert!(!product_sources.contains("platform_window::text("));
    assert!(settings_host.contains("fn control_text(&self, handle: Self::Handle) -> String"));
    assert!(settings_host.contains("pub(crate) fn settings_host_text(hwnd: HWND) -> String"));
}

#[test]
fn windows_settings_control_repaint_uses_control_host() {
    let main_entry = main_entry_source();
    let settings_commands = settings_commands_source();
    let settings_dropdown = settings_dropdown_source();
    let dropdown_host = settings_dropdown_host_source();
    let settings_actions = settings_actions_source();
    let settings_host = include_str!("settings_ui_host.rs").replace("\r\n", "\n");
    let selection_sources = [
        settings_control_selection_source(),
        settings_control_selection_general_source(),
        settings_control_selection_cloud_source(),
        settings_control_selection_hotkey_source(),
        settings_control_selection_plugin_source(),
        settings_control_selection_group_source(),
    ]
    .join("\n");
    let selection_start = settings_commands
        .find("unsafe fn handle_settings_control_selection")
        .unwrap();
    let selection_end = settings_commands[selection_start..]
        .find("\nfn show_settings_saved_feedback")
        .map(|offset| selection_start + offset)
        .unwrap_or(settings_commands.len());
    let selection_block = format!(
        "{}\n{}",
        &settings_commands[selection_start..selection_end],
        selection_sources
    );
    let saved_start = settings_commands
        .find("unsafe fn show_settings_saved_feedback")
        .unwrap();
    let saved_end = settings_commands[saved_start..]
        .find("\npub(super) fn queue_settings_command")
        .map(|offset| saved_start + offset)
        .unwrap();
    let saved_block = &settings_commands[saved_start..saved_end];
    let settings_input = settings_input_source();
    let key_start = settings_input
        .find("unsafe fn handle_settings_key_down")
        .unwrap();
    let key_end = settings_input[key_start..]
        .find("\npub(super) unsafe fn handle_settings_theme_changed")
        .map(|offset| key_start + offset)
        .unwrap();
    let key_block = &settings_input[key_start..key_end];
    let timer_start = settings_commands
        .find("unsafe fn handle_settings_timer_task")
        .unwrap();
    let timer_end = settings_commands[timer_start..]
        .find("\npub(super) unsafe fn handle_settings_control_selection")
        .map(|offset| timer_start + offset)
        .unwrap();
    let timer_block = &settings_commands[timer_start..timer_end];
    let sync_start = settings_actions
        .find("unsafe fn execute_settings_sync_action")
        .unwrap();
    let sync_end = settings_actions[sync_start..]
        .find("\npub(super) unsafe fn execute_settings_platform_action")
        .map(|offset| sync_start + offset)
        .unwrap();
    let sync_block = &settings_actions[sync_start..sync_end];
    let platform_action_start = settings_actions
        .find("unsafe fn execute_settings_platform_action")
        .unwrap();
    let platform_action_block = &settings_actions[platform_action_start..];
    let hotkey_start = main_entry
        .find("unsafe fn settings_set_hotkey_recording")
        .unwrap();
    let hotkey_end = main_entry[hotkey_start..]
        .find("\npub(super) unsafe fn handle_vv_select")
        .map(|offset| hotkey_start + offset)
        .unwrap();
    let hotkey_block = &main_entry[hotkey_start..hotkey_end];

    assert!(selection_block.contains("repaint_settings_control(st.cb_max)"));
    assert!(selection_block.contains("repaint_settings_control(st.cb_vv_group)"));
    assert!(!selection_block.contains("platform_gdi::invalidate_rect(st.cb_"));
    assert!(saved_block.contains("repaint_settings_control(st.btn_save)"));
    assert!(timer_block.contains("repaint_settings_control(st.btn_save)"));
    assert!(key_block.contains("repaint_settings_control(st.lb_hk_preview)"));
    assert!(sync_block.contains("repaint_settings_control(sender)"));
    assert!(platform_action_block.contains("repaint_settings_control(st.ed_skip_class_names)"));
    assert!(hotkey_block.contains("repaint_settings_control(st.btn_hk_record)"));
    assert!(hotkey_block.contains("repaint_settings_control(st.lb_hk_preview)"));
    for block in [
        saved_block,
        key_block,
        timer_block,
        sync_block,
        platform_action_block,
        hotkey_block,
    ] {
        assert!(!block.contains("platform_gdi::invalidate_rect(st.btn_"));
        assert!(!block.contains("platform_gdi::invalidate_rect(st.lb_"));
        assert!(!block.contains("platform_gdi::invalidate_rect(st.ed_"));
        assert!(!block.contains("platform_gdi::invalidate_rect(sender"));
    }
    assert!(!settings_dropdown.contains("fn repaint_settings_control(hwnd: HWND)"));
    assert!(dropdown_host.contains("fn repaint_settings_control(hwnd: HWND)"));
    assert!(dropdown_host.contains("settings_host_request_repaint(hwnd)"));
    assert!(settings_host.contains("fn request_control_repaint(&mut self"));
    assert!(settings_host.contains("platform_gdi::invalidate_rect(handle, null(), 1)"));
    assert!(settings_host.contains("pub(crate) fn settings_host_request_repaint"));
}

#[test]
fn windows_settings_window_destroy_uses_settings_host() {
    let settings_window = settings_window_source();
    let settings_commands = settings_commands_source();
    let settings_input = settings_input_source();
    let settings_host = include_str!("settings_ui_host.rs").replace("\r\n", "\n");
    let dispatch_start = settings_input
        .find("unsafe fn dispatch_settings_ui_event")
        .unwrap();
    let queue_start = settings_input[dispatch_start..]
        .find("\nfn queue_settings_command")
        .map(|offset| dispatch_start + offset)
        .unwrap_or(settings_input.len());
    let command_start = settings_commands
        .find("unsafe fn execute_settings_ui_command")
        .unwrap();
    let drain_start = settings_commands[command_start..]
        .find("\npub(super) unsafe fn drain_settings_ui_commands")
        .map(|offset| command_start + offset)
        .unwrap();
    let helper_start = settings_window
        .find("fn destroy_settings_window(hwnd: HWND)")
        .unwrap();
    let helper_end = settings_window[helper_start..]
        .find("\npub(super) fn focus_settings_window")
        .map(|offset| helper_start + offset)
        .unwrap();
    let dispatch_block = &settings_input[dispatch_start..queue_start];
    let command_block = &settings_commands[command_start..drain_start];
    let helper_block = &settings_window[helper_start..helper_end];

    assert!(dispatch_block.contains("UiEvent::CloseRequested"));
    assert!(dispatch_block.contains("destroy_settings_window(hwnd)"));
    assert!(!dispatch_block.contains("platform_window::destroy(hwnd)"));
    assert!(command_block.contains("command_ids::CLOSE_SETTINGS"));
    assert!(command_block.contains("destroy_settings_window(hwnd)"));
    assert!(!command_block.contains("platform_window::destroy(hwnd)"));
    assert!(helper_block.contains("WindowsSettingsWindowHost::new"));
    assert!(helper_block.contains(".destroy_settings_window(hwnd)"));
    assert!(settings_host.contains("fn destroy_settings_window(&mut self"));
    assert!(settings_host.contains("platform_window::destroy(handle)"));
}

#[test]
fn windows_settings_actions_use_shared_route_dispatch() {
    let settings_input = settings_input_source();
    let settings_actions = settings_actions_source();
    let dispatch_start = settings_input
        .find("unsafe fn dispatch_settings_ui_event")
        .unwrap();
    let dispatch_end = settings_input[dispatch_start..]
        .find("\nfn queue_settings_command")
        .map(|offset| dispatch_start + offset)
        .unwrap_or(settings_input.len());
    let dispatch_block = &settings_input[dispatch_start..dispatch_end];

    assert!(dispatch_block.contains("WindowsSettingsActionExecutor::new(hwnd)"));
    assert!(dispatch_block.contains("dispatch_settings_action(&mut executor"));
    assert!(!dispatch_block.contains("SettingsActionRoute::"));
    assert!(!dispatch_block.contains("if execute_settings_sync_action"));
    assert!(!dispatch_block.contains("else if execute_settings_group_action"));
    assert!(
        settings_actions.contains("impl SettingsActionExecutor for WindowsSettingsActionExecutor")
    );
    assert!(settings_actions.contains("fn execute_sync("));
    assert!(settings_actions.contains("fn execute_group("));
    assert!(settings_actions.contains("fn execute_platform("));
}

#[test]
fn windows_settings_action_domains_live_in_dedicated_modules() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let executor = settings_actions_executor_source();
    let sync_actions = settings_sync_actions_source();
    let sync_entry = settings_sync_actions_entry_source();
    let sync_webdav = settings_sync_actions_webdav_source();
    let sync_lan = settings_sync_actions_lan_source();
    let group_actions = settings_group_actions_source();
    let platform_actions = settings_platform_actions_source();
    let platform_entry = settings_platform_actions_entry_source();
    let platform_hotkey = settings_platform_actions_hotkey_source();
    let platform_general = settings_platform_actions_general_source();
    let platform_plugin = settings_platform_actions_plugin_source();
    let platform_about = settings_platform_actions_about_source();
    let platform_system = settings_platform_actions_system_source();

    for module in [
        "settings_sync_actions",
        "settings_sync_actions_webdav",
        "settings_sync_actions_lan",
        "settings_group_actions",
        "settings_platform_actions",
        "settings_platform_actions_hotkey",
        "settings_platform_actions_general",
        "settings_platform_actions_plugin",
        "settings_platform_actions_about",
        "settings_platform_actions_system",
    ] {
        assert!(app.contains(&format!("mod {module};")));
        assert!(prelude.contains(&format!("use super::{module}::*;")));
    }

    for action_fn in [
        "pub(super) unsafe fn execute_settings_sync_action(",
        "pub(super) unsafe fn execute_settings_group_action(",
        "pub(super) unsafe fn execute_settings_platform_action(",
    ] {
        assert!(!executor.contains(action_fn));
    }

    assert!(executor.contains("impl SettingsActionExecutor for WindowsSettingsActionExecutor"));
    assert!(sync_actions.contains("pub(super) unsafe fn execute_settings_sync_action("));
    for function in [
        "execute_settings_webdav_sync_action(hwnd, st, action)",
        "execute_settings_lan_sync_action(hwnd, st, action)",
    ] {
        assert!(
            sync_entry.contains(function),
            "{function} should be dispatched by settings_sync_actions.rs"
        );
    }
    for detail in [
        "CloudSyncAction::SyncNow",
        "crate::lan_sync::start_pair_with_host",
        "copy_text_to_clipboard_in_background",
        "open_path_with_shell",
    ] {
        assert!(
            !sync_entry.contains(detail),
            "{detail} should live in a settings sync action domain module"
        );
    }
    assert!(sync_webdav.contains("pub(super) unsafe fn execute_settings_webdav_sync_action"));
    assert!(sync_webdav.contains("CloudSyncAction::SyncNow"));
    assert!(sync_webdav.contains("queue_cloud_sync"));
    assert!(sync_lan.contains("pub(super) unsafe fn execute_settings_lan_sync_action"));
    assert!(sync_lan.contains("crate::lan_sync::start_pair_with_host"));
    assert!(sync_lan.contains("crate::lan_sync::trigger_discovery"));
    assert!(sync_lan.contains("copy_text_to_clipboard_in_background"));
    assert!(group_actions.contains("pub(super) unsafe fn execute_settings_group_action("));
    assert!(group_actions.contains("db_create_named_group"));
    assert!(group_actions.contains("db_delete_group"));
    assert!(platform_actions.contains("pub(super) unsafe fn execute_settings_platform_action("));
    for function in [
        "execute_settings_platform_hotkey_action(hwnd, st, action)",
        "execute_settings_platform_general_action(hwnd, st, action)",
        "execute_settings_platform_plugin_action(hwnd, st, action)",
        "execute_settings_platform_about_action(hwnd, action)",
        "execute_settings_platform_system_action(hwnd, action)",
    ] {
        assert!(
            platform_entry.contains(function),
            "{function} should be dispatched by settings_platform_actions.rs"
        );
    }
    for detail in [
        "pick_paste_sound_file",
        "detect_wechat_runtime_dir",
        "set_system_clipboard_history_enabled",
        "WindowsMailMergeWindowHost::new().open_mail_merge",
        "open_source_url()",
        "restart_explorer_shell",
    ] {
        assert!(
            !platform_entry.contains(detail),
            "{detail} should live in a settings platform action domain module"
        );
    }
    assert!(
        platform_hotkey.contains("pub(super) unsafe fn execute_settings_platform_hotkey_action")
    );
    assert!(platform_hotkey.contains("SettingsAction::ToggleHotkeyRecording"));
    assert!(platform_hotkey.contains("focus_settings_window(hwnd)"));
    assert!(
        platform_general.contains("pub(super) unsafe fn execute_settings_platform_general_action")
    );
    assert!(platform_general.contains("pick_paste_sound_file"));
    assert!(platform_general.contains("find_next_paste_target_after"));
    assert!(
        platform_plugin.contains("pub(super) unsafe fn execute_settings_platform_plugin_action")
    );
    assert!(platform_plugin.contains("detect_wechat_runtime_dir"));
    assert!(platform_plugin.contains("WindowsMailMergeWindowHost::new().open_mail_merge"));
    assert!(platform_about.contains("pub(super) unsafe fn execute_settings_platform_about_action"));
    assert!(platform_about.contains("open_source_url()"));
    assert!(platform_about.contains("start_update_check"));
    assert!(
        platform_system.contains("pub(super) unsafe fn execute_settings_platform_system_action")
    );
    assert!(platform_system.contains("set_system_clipboard_history_enabled"));
    assert!(platform_system.contains("restart_explorer_shell"));
}

#[test]
fn windows_settings_window_focus_uses_settings_host() {
    let settings_window = settings_window_source();
    let settings_actions = settings_actions_source();
    let settings_host = include_str!("settings_ui_host.rs").replace("\r\n", "\n");
    let action_start = settings_actions
        .find("unsafe fn execute_settings_platform_action")
        .unwrap();
    let helper_start = settings_window
        .find("fn focus_settings_window(hwnd: HWND)")
        .unwrap();
    let helper_end = settings_window[helper_start..]
        .find("\npub(super) fn capture_settings_pointer")
        .map(|offset| helper_start + offset)
        .unwrap();
    let action_block = &settings_actions[action_start..];
    let helper_block = &settings_window[helper_start..helper_end];

    assert!(action_block.contains("SettingsAction::ToggleHotkeyRecording"));
    assert!(action_block.contains("focus_settings_window(hwnd)"));
    assert!(!action_block.contains("platform_input::set_focus(hwnd)"));
    assert!(helper_block.contains("WindowsSettingsWindowHost::new"));
    assert!(helper_block.contains(".focus_settings_window(hwnd)"));
    assert!(settings_host.contains("fn focus_settings_window(&mut self"));
    assert!(settings_host.contains("platform_input::set_focus(handle)"));
}

#[test]
fn windows_settings_window_repaint_uses_settings_host() {
    let main_events = main_events_source();
    let settings_window = settings_window_source();
    let settings_commands = settings_commands_source();
    let settings_input = settings_input_source();
    let settings_actions = settings_actions_source();
    let settings_host = include_str!("settings_ui_host.rs").replace("\r\n", "\n");
    let tray_start = main_events
        .find("MainTrayActionPlan::SetLanSync { enabled }")
        .unwrap();
    let tray_end = main_events[tray_start..]
        .find("\n                MainTrayActionPlan::Exit")
        .map(|offset| tray_start + offset)
        .unwrap();
    let event_start = main_events
        .find("ApplicationEvent::UpdateCheckReady =>")
        .unwrap();
    let event_end = main_events[event_start..]
        .find("\n        ApplicationEvent::ShellIntegrationRestored")
        .map(|offset| event_start + offset)
        .unwrap();
    let tray_block = &main_events[tray_start..tray_end];
    let event_block = &main_events[event_start..event_end];
    let theme_start = settings_input
        .find("unsafe fn handle_settings_theme_changed")
        .unwrap();
    let theme_end = settings_input[theme_start..]
        .find("\npub(super) unsafe fn apply_dpi_suggested_rect")
        .map(|offset| theme_start + offset)
        .unwrap();
    let theme_block = &settings_input[theme_start..theme_end];
    let toggle_start = settings_commands
        .find("unsafe fn execute_settings_toggle_control")
        .unwrap();
    let toggle_end = settings_commands[toggle_start..]
        .find("\nunsafe fn execute_settings_ui_command")
        .map(|offset| toggle_start + offset)
        .unwrap();
    let toggle_block = &settings_commands[toggle_start..toggle_end];
    let command_start = settings_commands
        .find("unsafe fn execute_settings_ui_command")
        .unwrap();
    let command_end = settings_commands[command_start..]
        .find("\npub(super) unsafe fn drain_settings_ui_commands")
        .map(|offset| command_start + offset)
        .unwrap();
    let command_block = &settings_commands[command_start..command_end];
    let timer_start = settings_commands
        .find("unsafe fn handle_settings_timer_task")
        .unwrap();
    let timer_end = settings_commands[timer_start..]
        .find("\npub(super) unsafe fn handle_settings_control_selection")
        .map(|offset| timer_start + offset)
        .unwrap();
    let timer_block = &settings_commands[timer_start..timer_end];
    let action_start = settings_actions
        .find("unsafe fn execute_settings_platform_action")
        .unwrap();
    let action_block = &settings_actions[action_start..];

    for block in [tray_block, event_block] {
        assert!(block.contains("request_settings_window_repaint("));
        assert!(!block.contains("WindowsSettingsWindowHost::new"));
        assert!(!block.contains("platform_window::exists("));
        assert!(!block.contains("platform_gdi::invalidate_rect("));
    }
    let helper_start = settings_window
        .find("fn request_settings_window_repaint(hwnd: HWND)")
        .unwrap();
    let helper_end = settings_window[helper_start..]
        .find("\npub(super) unsafe fn refresh_settings_cloud_page_after_lan_sync")
        .map(|offset| helper_start + offset)
        .unwrap();
    let helper_block = &settings_window[helper_start..helper_end];
    assert!(helper_block.contains("WindowsSettingsWindowHost::new(Some(settings_wnd_proc))"));
    assert!(helper_block.contains(".request_settings_window_repaint(hwnd)"));
    assert!(settings_host.contains("fn request_settings_window_repaint(&mut self"));
    assert!(settings_host.contains("platform_window::exists(handle)"));
    assert!(settings_host.contains("platform_gdi::invalidate_rect(handle, null(), 1)"));
    assert!(theme_block.contains("repaint_settings_window(hwnd, true)"));
    assert!(!theme_block.contains("platform_gdi::invalidate_rect(hwnd, null(), 1)"));
    assert!(toggle_block.contains("repaint_settings_window(hwnd, true)"));
    assert!(!toggle_block.contains("platform_gdi::invalidate_rect(hwnd, null(), 1)"));
    assert!(command_block.contains("repaint_settings_window(hwnd, true)"));
    assert!(!command_block.contains("platform_gdi::invalidate_rect(hwnd, null(), 1)"));
    assert!(timer_block.contains("repaint_settings_window(hwnd, false)"));
    assert!(!timer_block.contains("platform_gdi::invalidate_rect(hwnd, null(), 0)"));
    assert!(action_block.contains("repaint_settings_window(hwnd, true)"));
    assert!(!action_block.contains("platform_gdi::invalidate_rect(hwnd, null(), 1)"));
    assert!(settings_host.contains("fn request_settings_window_area_repaint("));
    assert!(settings_host.contains("platform_gdi::invalidate_rect(handle, rect_ptr"));
}

#[test]
fn lan_sync_ready_cloud_settings_refresh_uses_hosts() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let lan_sync = main_lan_sync_source();
    let settings_window = settings_window_source();
    let production = app
        .split("\n#[cfg(test)]\nmod tests")
        .next()
        .unwrap_or(&app);
    let ready_start = lan_sync
        .find("pub(super) unsafe fn handle_lan_sync_ready(hwnd: HWND)")
        .unwrap();
    let ready_end = lan_sync[ready_start..]
        .find("\nunsafe fn apply_lan_item_to_clipboard")
        .map(|offset| ready_start + offset)
        .unwrap();
    let helper_start = settings_window
        .find("unsafe fn refresh_settings_cloud_page_after_lan_sync")
        .unwrap();
    let ready_block = &lan_sync[ready_start..ready_end];
    let helper_block = &settings_window[helper_start..];

    assert!(production.contains("mod main_lan_sync;"));
    assert!(!production.contains("unsafe fn handle_lan_sync_ready"));
    assert!(ready_block.contains("refresh_settings_cloud_page_after_lan_sync(state.settings_hwnd)"));
    assert!(!ready_block.contains("platform_window::exists(state.settings_hwnd)"));
    assert!(!ready_block.contains("platform_gdi::invalidate_rect(state.settings_hwnd"));
    assert!(helper_block.contains("WindowsWindowIdentityHost::new().exists(settings_hwnd)"));
    assert!(helper_block
        .contains("settings_sync_page_state(&mut *st_ptr, SettingsPage::Cloud.index())"));
    assert!(helper_block.contains("request_settings_window_repaint(settings_hwnd)"));
}

#[test]
fn windows_main_sync_adapters_live_outside_app_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let cloud = main_cloud_sync_source();
    let lan = main_lan_sync_source();

    assert!(app.contains("mod main_cloud_sync;"));
    assert!(app.contains("mod main_lan_sync;"));
    for function in [
        "unsafe fn queue_cloud_sync",
        "unsafe fn apply_ready_cloud_syncs",
        "fn maybe_broadcast_lan_clip_item",
        "fn lan_envelope_from_item",
        "fn lan_latest_envelope_from_item",
        "unsafe fn handle_lan_sync_ready",
        "fn lan_item_from_envelope",
    ] {
        assert!(
            !app.contains(function),
            "{function} must stay outside app.rs"
        );
    }

    assert!(cloud.contains("pub(super) unsafe fn queue_cloud_sync"));
    assert!(cloud.contains("pub(super) unsafe fn apply_ready_cloud_syncs"));
    assert!(cloud.contains("spawn_cloud_sync_job("));
    assert!(cloud.contains("sync_peer_windows_from_settings(hwnd)"));
    assert!(lan.contains("pub(super) fn maybe_broadcast_lan_clip_item"));
    assert!(lan.contains("pub(super) fn lan_latest_envelope_from_item"));
    assert!(lan.contains("pub(super) unsafe fn handle_lan_sync_ready"));
    assert!(lan.contains("WindowsClipboardHost::write_image_rgba"));
    assert!(lan.contains("lan_sync::broadcast_clip"));
}

#[test]
fn windows_main_runtime_state_definition_lives_outside_app_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let state = app_state_source();

    assert!(app.contains("pub(crate) mod state;"));
    assert!(!app.contains("pub(crate) enum WindowRole"));
    assert!(!app.contains("pub(crate) struct Icons"));
    assert!(!app.contains("pub(crate) struct AppState"));
    assert!(!app.contains("impl Deref for AppState"));
    assert!(state.contains("pub(crate) enum WindowRole"));
    assert!(state.contains("pub(crate) struct Icons"));
    assert!(state.contains("pub(crate) struct AppState"));
    assert!(state.contains("impl Deref for AppState"));
    assert!(state.contains("pub(super) image_thumb_cache: ImageThumbnailCache"));
    assert!(state.contains("pub(super) vv_popup_items: Vec<VvPopupEntry>"));
}

#[test]
fn windows_clip_payload_data_helpers_live_outside_app_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let data = app_data_source();

    for function in [
        "fn build_preview(",
        "fn build_files_preview(",
        "fn text_content_signature(",
        "fn image_content_signature(",
        "fn file_paths_signature(",
        "fn dedupe_signature_for_item(",
        "fn build_qr_clip_item(",
        "fn output_image_path(",
        "fn write_image_bytes_to_path(",
        "fn load_image_bytes_from_path(",
        "fn save_image_item(",
        "fn ensure_item_image_bytes(",
        "fn ensure_item_thumbnail_bytes(",
    ] {
        assert!(
            !app.contains(function),
            "{function} should stay out of app.rs"
        );
        assert!(
            data.contains(function),
            "{function} should live in app/data.rs"
        );
    }

    assert!(data.contains("post_boxed_message(hwnd_raw, WM_IMAGE_THUMB_READY"));
}

#[test]
fn windows_main_runtime_state_behavior_lives_outside_app_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let state_runtime = app_state_runtime_source();

    assert!(app.contains("mod state_runtime;"));
    assert!(!app.contains("impl AppState {"));
    for function in [
        "fn should_skip_transient_duplicate_capture(",
        "fn remember_lan_message_key(",
        "fn note_programmatic_clipboard_signature(",
        "fn consume_skip_next_clipboard_update_once(",
        "fn consume_recent_programmatic_clipboard_signature(",
        "fn selected_items_owned(",
        "fn current_scroll_anchor(",
        "fn reload_state_from_db_preserve_scroll(",
        "fn resolve_item_for_use(",
        "fn add_clip_item_inner(",
        "fn selected_db_ids(",
    ] {
        assert!(
            !app.contains(function),
            "{function} should stay out of app.rs"
        );
        assert!(
            state_runtime.contains(function),
            "{function} should live in app/state_runtime.rs"
        );
    }

    assert!(state_runtime.contains("pub(super) fn add_clip_item("));
    assert!(state_runtime.contains("pub(super) fn layout("));
    assert!(state_runtime.contains("sync_peer_windows_from_db(self.hwnd)"));
}

#[test]
fn windows_settings_state_definition_lives_outside_app_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let settings_state = settings_state_source();
    let settings_create = settings_window_create_source();

    assert!(app.contains("mod settings_state;"));
    assert!(!app.contains("struct SettingsWndState {"));
    assert!(settings_state.contains("pub(super) struct SettingsWndState"));
    assert!(settings_state.contains("impl SettingsWndState"));
    assert!(settings_state.contains("pub(super) fn new("));
    assert!(settings_state.contains("pub(super) parent_hwnd: HWND"));
    assert!(settings_state.contains("pub(super) cur_page: usize"));
    assert!(settings_state.contains("pub(super) draft: AppSettings"));
    assert!(settings_state.contains("pub(super) plugin_sections: Vec<SettingsSection>"));
    assert!(settings_state.contains("pub(super) multi_sync_sections: Vec<SettingsSection>"));
    assert!(settings_create.contains("SettingsWndState::new("));
    assert!(!settings_create.contains("SettingsWndState {"));
}

#[test]
fn windows_settings_plugin_sections_live_outside_hosts_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let hosts = app_hosts_source();
    let plugin_sections = settings_plugin_sections_source();
    let plugin_controls = settings_plugin_sections_controls_source();
    let plugin_layout = settings_plugin_sections_layout_source();
    let plugin_providers = settings_plugin_sections_providers_source();
    let plugin_tools = settings_plugin_sections_tools_source();
    let plugin_domains = [
        plugin_sections.as_str(),
        plugin_controls.as_str(),
        plugin_layout.as_str(),
        plugin_providers.as_str(),
        plugin_tools.as_str(),
    ]
    .join("\n");

    assert!(app.contains("mod settings_plugin_sections;"));
    assert!(prelude.contains("use super::settings_plugin_sections::*;"));
    for module in [
        "settings_plugin_sections_controls",
        "settings_plugin_sections_layout",
        "settings_plugin_sections_providers",
        "settings_plugin_sections_tools",
    ] {
        assert!(app.contains(&format!("mod {module};")));
        assert!(prelude.contains(&format!("use super::{module}::*;")));
    }

    for function in [
        "fn settings_plugin_move_control(",
        "fn settings_plugin_move_toggle_row(",
        "fn settings_plugin_move_labeled_field(",
        "fn settings_plugin_layout(",
        "fn settings_refresh_plugin_cards(",
        "fn settings_relayout_plugin_page(",
    ] {
        assert!(
            !hosts.contains(function),
            "{function} should stay out of app/hosts.rs"
        );
        assert!(
            plugin_domains.contains(function),
            "{function} should live in settings plugin section domain modules"
        );
    }

    for control_function in [
        "fn settings_plugin_show_enable(",
        "fn settings_plugin_move_control(",
        "fn settings_plugin_move_toggle_row(",
        "fn settings_plugin_move_labeled_field(",
    ] {
        assert!(plugin_controls.contains(control_function));
        assert!(!plugin_sections.contains(control_function));
    }
    for layout_function in [
        "fn settings_plugin_layout(",
        "fn settings_refresh_plugin_cards(",
        "fn settings_refresh_plugin_host_after_relayout(",
    ] {
        assert!(plugin_layout.contains(layout_function));
        assert!(!plugin_sections.contains(layout_function));
    }
    for provider_function in [
        "fn settings_relayout_plugin_quick_search_section(",
        "fn settings_relayout_plugin_ocr_section(",
        "fn settings_relayout_plugin_translate_section(",
    ] {
        assert!(plugin_providers.contains(provider_function));
        assert!(!plugin_sections.contains(provider_function));
    }
    assert!(plugin_tools.contains("fn settings_relayout_plugin_tool_sections("));
    assert!(!plugin_sections.contains("fn settings_relayout_plugin_tool_sections("));

    assert!(!hosts.contains("fn settings_plugin_show_enable("));
    assert!(plugin_controls.contains("pub(super) unsafe fn settings_plugin_show_enable("));
    assert!(plugin_layout.contains("settings_plugin_cards_for_state"));
    assert!(plugin_layout.contains("settings_form_layout_for_section"));
    assert!(plugin_layout.contains("SettingsPage::Plugin.index()"));
    assert!(!plugin_sections.contains("SettingsPage::Plugin.index()"));
    assert!(plugin_sections.contains("settings_relayout_plugin_quick_search_section(st, line_h)"));
    assert!(plugin_sections.contains("settings_relayout_plugin_tool_sections(st)"));
}

#[test]
fn windows_settings_multi_sync_sections_live_outside_hosts_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let hosts = app_hosts_source();
    let multi_sync_sections = settings_multi_sync_sections_source();

    assert!(app.contains("mod settings_multi_sync_sections;"));
    assert!(prelude.contains("use super::settings_multi_sync_sections::*;"));

    for function in [
        "fn settings_multi_sync_layout(",
        "fn settings_reset_cloud_page_handles(",
        "fn settings_refresh_multi_sync_cards(",
        "fn settings_rebuild_cloud_page(",
    ] {
        assert!(
            !hosts.contains(function),
            "{function} should stay out of app/hosts.rs"
        );
        assert!(
            multi_sync_sections.contains(function),
            "{function} should live in app/settings_multi_sync_sections.rs"
        );
    }

    assert!(multi_sync_sections.contains("settings_multi_sync_cards_for_mode"));
    assert!(multi_sync_sections.contains("SettingsPage::Cloud.index()"));
    assert!(multi_sync_sections.contains("settings_create_cloud_page(hwnd, st)"));
    assert!(multi_sync_sections.contains("settings_sync_page_state(st, page)"));
}

#[test]
fn windows_settings_group_sections_live_outside_hosts_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let hosts = app_hosts_source();
    let group_sections = settings_group_sections_source();
    let group_sections_cache = settings_group_sections_cache_source();
    let group_sections_display = settings_group_sections_display_source();
    let group_sections_list = settings_group_sections_list_source();
    let group_sections_domains = [
        group_sections.as_str(),
        group_sections_cache.as_str(),
        group_sections_display.as_str(),
        group_sections_list.as_str(),
    ]
    .join("\n");
    let group_page = settings_group_page_source();

    assert!(app.contains("mod settings_group_sections;"));
    for module in [
        "settings_group_sections_cache",
        "settings_group_sections_display",
        "settings_group_sections_list",
    ] {
        assert!(app.contains(&format!("mod {module};")));
        assert!(prelude.contains(&format!("use super::{module}::*;")));
    }
    assert!(app.contains("mod settings_group_page;"));
    assert!(prelude.contains("use super::settings_group_sections::*;"));
    assert!(prelude.contains("use super::settings_group_page::*;"));

    for function in [
        "fn settings_groups_cache_for_tab(",
        "fn settings_groups_cache_for_tab_mut(",
        "fn settings_group_current_filter_text(",
        "fn settings_sync_vv_source_display(",
        "fn settings_sync_vv_group_display(",
        "fn settings_sync_group_view_tabs(",
        "fn settings_sync_group_overview(",
        "fn settings_vv_source_current(",
        "fn settings_group_view_current(",
        "fn settings_vv_source_from_app(",
        "fn settings_group_view_from_app(",
        "fn settings_sync_group_page(",
        "fn settings_groups_refresh_list(",
        "fn settings_groups_selected(",
        "fn settings_groups_sync_name(",
        "fn settings_groups_move(",
    ] {
        assert!(
            !hosts.contains(function),
            "{function} should stay out of app/hosts.rs"
        );
        assert!(
            group_sections_domains.contains(function),
            "{function} should live in settings group section domain modules"
        );
    }

    for cache_function in [
        "fn settings_groups_cache_for_tab(",
        "fn settings_groups_cache_for_tab_mut(",
        "fn settings_vv_source_current(",
        "fn settings_group_view_current(",
        "fn settings_vv_source_from_app(",
        "fn settings_group_view_from_app(",
    ] {
        assert!(group_sections_cache.contains(cache_function));
        assert!(!group_sections.contains(cache_function));
    }
    for display_function in [
        "fn settings_group_current_filter_text(",
        "fn settings_sync_vv_source_display(",
        "fn settings_sync_vv_group_display(",
        "fn settings_sync_group_view_tabs(",
        "fn settings_sync_group_overview(",
    ] {
        assert!(group_sections_display.contains(display_function));
        assert!(!group_sections.contains(display_function));
    }
    for list_function in [
        "fn settings_groups_refresh_list(",
        "fn settings_groups_selected(",
        "fn settings_groups_sync_name(",
        "fn settings_groups_move(",
    ] {
        assert!(group_sections_list.contains(list_function));
        assert!(!group_sections.contains(list_function));
    }
    assert!(group_sections.contains("pub(super) unsafe fn settings_sync_group_page("));

    assert!(!hosts.contains("fn settings_create_group_page("));
    assert!(!group_sections.contains("fn settings_create_group_page("));
    assert!(group_page.contains("pub(super) unsafe fn settings_create_group_page("));
    assert!(group_page.contains("IDC_SET_GROUP_ENABLE"));
    assert!(group_page.contains("IDC_SET_VV_SOURCE"));
    assert!(group_page.contains("IDC_SET_GROUP_LIST"));

    assert!(group_sections.contains("db_load_groups"));
    assert!(group_sections_list.contains("db_load_groups"));
    assert!(group_sections_list.contains("db_set_groups_order"));
    assert!(!group_sections.contains("db_set_groups_order"));
    assert!(group_sections_display.contains("settings_group_overview_text"));
    assert!(!group_sections.contains("settings_group_overview_text"));
}

#[test]
fn windows_settings_general_page_lives_outside_hosts_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let hosts = app_hosts_source();
    let general_page = settings_general_page_source();
    let general_startup = settings_general_page_startup_source();
    let general_window = settings_general_page_window_source();

    assert!(app.contains("mod settings_general_page;"));
    assert!(app.contains("mod settings_general_page_startup;"));
    assert!(app.contains("mod settings_general_page_window;"));
    assert!(prelude.contains("use super::settings_general_page::*;"));
    assert!(prelude.contains("use super::settings_general_page_startup::*;"));
    assert!(prelude.contains("use super::settings_general_page_window::*;"));
    assert!(!hosts.contains("fn settings_create_general_page("));
    assert!(general_page.contains("pub(super) unsafe fn settings_create_general_page("));
    assert!(general_page.contains("settings_create_general_startup_behavior_page"));
    assert!(general_page.contains("settings_create_general_window_position_page"));
    assert!(!general_page.contains("IDC_SET_AUTOSTART"));
    assert!(!general_page.contains("IDC_SET_SKIP_WINDOW_CLASSNAMES"));
    assert!(general_startup
        .contains("pub(super) unsafe fn settings_create_general_startup_behavior_page("));
    assert!(general_startup.contains("IDC_SET_AUTOSTART"));
    assert!(general_startup.contains("IDC_SET_MAX"));
    assert!(general_startup.contains("IDC_SET_PASTE_SOUND_KIND"));
    assert!(general_window
        .contains("pub(super) unsafe fn settings_create_general_window_position_page("));
    assert!(general_window.contains("IDC_SET_SKIP_WINDOW_CLASSNAMES"));
    assert!(general_window.contains("IDC_SET_POSMODE"));
    assert!(general_window.contains("IDC_SET_BTN_OPENCFG"));
    assert!(general_page.contains("SettingsPage::General.index()"));
}

#[test]
fn windows_settings_hotkey_page_lives_outside_hosts_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let hosts = app_hosts_source();
    let hotkey_page = settings_hotkey_page_source();
    let hotkey_shortcuts = settings_hotkey_page_shortcuts_source();
    let hotkey_system = settings_hotkey_page_system_source();

    assert!(app.contains("mod settings_hotkey_page;"));
    assert!(app.contains("mod settings_hotkey_page_shortcuts;"));
    assert!(app.contains("mod settings_hotkey_page_system;"));
    assert!(prelude.contains("use super::settings_hotkey_page::*;"));
    assert!(prelude.contains("use super::settings_hotkey_page_shortcuts::*;"));
    assert!(prelude.contains("use super::settings_hotkey_page_system::*;"));
    assert!(!hosts.contains("fn settings_create_hotkey_page("));
    assert!(hotkey_page.contains("pub(super) unsafe fn settings_create_hotkey_page("));
    assert!(hotkey_page.contains("settings_create_hotkey_shortcut_controls"));
    assert!(hotkey_page.contains("settings_create_hotkey_system_controls"));
    assert!(!hotkey_page.contains("IDC_SET_HK_RECORD"));
    assert!(!hotkey_page.contains("IDC_SET_PLAIN_HK_ENABLE"));
    assert!(
        hotkey_shortcuts.contains("pub(super) unsafe fn settings_create_hotkey_shortcut_controls(")
    );
    assert!(hotkey_shortcuts.contains("IDC_SET_HK_RECORD"));
    assert!(hotkey_shortcuts.contains("IDC_SET_PLAIN_HK_ENABLE"));
    assert!(hotkey_shortcuts.contains("hotkey_preview_text"));
    assert!(hotkey_system.contains("pub(super) unsafe fn settings_create_hotkey_system_controls("));
    assert!(hotkey_system.contains("6111"));
    assert!(hotkey_system.contains("6113"));
    assert!(hotkey_page.contains("SettingsPage::Hotkey.index()"));
}

#[test]
fn windows_settings_plugin_page_lives_outside_hosts_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let hosts = app_hosts_source();
    let plugin_page = settings_plugin_page_source();
    let plugin_search = settings_plugin_page_search_source();
    let plugin_ocr_translate = settings_plugin_page_ocr_translate_source();
    let plugin_tools = settings_plugin_page_tools_source();

    assert!(app.contains("mod settings_plugin_page;"));
    assert!(app.contains("mod settings_plugin_page_search;"));
    assert!(app.contains("mod settings_plugin_page_ocr_translate;"));
    assert!(app.contains("mod settings_plugin_page_tools;"));
    assert!(prelude.contains("use super::settings_plugin_page::*;"));
    assert!(prelude.contains("use super::settings_plugin_page_search::*;"));
    assert!(prelude.contains("use super::settings_plugin_page_ocr_translate::*;"));
    assert!(prelude.contains("use super::settings_plugin_page_tools::*;"));
    assert!(!hosts.contains("fn settings_create_plugin_page("));
    assert!(plugin_page.contains("pub(super) unsafe fn settings_create_plugin_page("));
    assert!(plugin_page.contains("settings_create_plugin_quick_search_page"));
    assert!(plugin_page.contains("settings_create_plugin_ocr_translate_page"));
    assert!(plugin_page.contains("settings_create_plugin_tools_page"));
    assert!(!plugin_page.contains("IDC_SET_OCR_PROVIDER"));
    assert!(!plugin_page.contains("IDC_SET_TRANSLATE_APP_ID"));
    assert!(!plugin_page.contains("IDC_SET_PLUGIN_MAILMERGE"));
    assert!(
        plugin_search.contains("pub(super) unsafe fn settings_create_plugin_quick_search_page(")
    );
    assert!(plugin_search.contains("7201"));
    assert!(plugin_ocr_translate
        .contains("pub(super) unsafe fn settings_create_plugin_ocr_translate_page("));
    assert!(plugin_ocr_translate.contains("IDC_SET_OCR_PROVIDER"));
    assert!(plugin_ocr_translate.contains("IDC_SET_TRANSLATE_APP_ID"));
    assert!(plugin_tools.contains("pub(super) unsafe fn settings_create_plugin_tools_page("));
    assert!(plugin_tools.contains("IDC_SET_PLUGIN_MAILMERGE"));
    assert!(plugin_tools.contains("IDC_SET_WPS_TASKPANE_DOCS"));
    assert!(plugin_page.contains("SettingsPage::Plugin.index()"));
}

#[test]
fn windows_settings_about_page_lives_outside_hosts_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let hosts = app_hosts_source();
    let about_page = settings_about_page_source();
    let about_metadata = settings_about_page_metadata_source();
    let about_update = settings_about_page_update_source();
    let about_data = settings_about_page_data_source();

    assert!(app.contains("mod settings_about_page;"));
    assert!(app.contains("mod settings_about_page_metadata;"));
    assert!(app.contains("mod settings_about_page_update;"));
    assert!(app.contains("mod settings_about_page_data;"));
    assert!(prelude.contains("use super::settings_about_page::*;"));
    assert!(prelude.contains("use super::settings_about_page_metadata::*;"));
    assert!(prelude.contains("use super::settings_about_page_update::*;"));
    assert!(prelude.contains("use super::settings_about_page_data::*;"));
    assert!(!hosts.contains("fn settings_create_about_page("));
    assert!(about_page.contains("pub(super) unsafe fn settings_create_about_page("));
    assert!(about_page.contains("settings_create_about_metadata_section"));
    assert!(about_page.contains("settings_create_about_update_section"));
    assert!(about_page.contains("settings_create_about_data_section"));
    assert!(!about_page.contains("settings_update_presentation"));
    assert!(!about_page.contains("update_check_state_snapshot"));
    assert!(!about_page.contains("IDC_SET_OPEN_SOURCE"));
    assert!(!about_page.contains("IDC_SET_OPEN_UPDATE"));
    assert!(about_metadata.contains("pub(super) unsafe fn settings_create_about_metadata_section("));
    assert!(about_metadata.contains("IDC_SET_OPEN_SOURCE"));
    assert!(about_update.contains("pub(super) unsafe fn settings_create_about_update_section("));
    assert!(about_update.contains("IDC_SET_OPEN_UPDATE"));
    assert!(about_update.contains("settings_update_presentation"));
    assert!(about_update.contains("update_check_state_snapshot"));
    assert!(about_data.contains("pub(super) unsafe fn settings_create_about_data_section("));
    assert!(about_data.contains("data_dir()"));
    assert!(about_page.contains("SettingsPage::About.index()"));
}

#[test]
fn windows_settings_cloud_page_lives_outside_hosts_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let hosts = app_hosts_source();
    let cloud_page = settings_cloud_page_source();
    let cloud_lan_devices = settings_cloud_page_lan_devices_source();
    let cloud_webdav = settings_cloud_page_webdav_source();
    let cloud_lan = settings_cloud_page_lan_source();

    assert!(app.contains("mod settings_cloud_page;"));
    assert!(app.contains("mod settings_cloud_page_lan_devices;"));
    assert!(app.contains("mod settings_cloud_page_webdav;"));
    assert!(app.contains("mod settings_cloud_page_lan;"));
    assert!(prelude.contains("use super::settings_cloud_page::*;"));
    assert!(prelude.contains("use super::settings_cloud_page_lan_devices::*;"));
    assert!(prelude.contains("use super::settings_cloud_page_webdav::*;"));
    assert!(prelude.contains("use super::settings_cloud_page_lan::*;"));

    for function in ["fn settings_create_cloud_page("] {
        assert!(
            !hosts.contains(function),
            "{function} should stay out of app/hosts.rs"
        );
        assert!(
            cloud_page.contains(function),
            "{function} should live in app/settings_cloud_page.rs"
        );
    }

    for function in [
        "fn settings_lan_refresh_lists(",
        "fn settings_lan_selected_device(",
        "fn settings_lan_selected_pair(",
    ] {
        assert!(
            !hosts.contains(function),
            "{function} should stay out of app/hosts.rs"
        );
        assert!(
            !cloud_page.contains(function),
            "{function} should stay out of app/settings_cloud_page.rs"
        );
        assert!(
            cloud_lan_devices.contains(function),
            "{function} should live in app/settings_cloud_page_lan_devices.rs"
        );
    }

    for function in [
        "fn settings_create_cloud_webdav_page(",
        "fn settings_create_cloud_lan_page(",
    ] {
        assert!(
            !hosts.contains(function),
            "{function} should stay out of app/hosts.rs"
        );
        assert!(
            !cloud_page.contains(function),
            "{function} should stay out of app/settings_cloud_page.rs"
        );
    }

    assert!(cloud_page.contains("settings_multi_sync_layout"));
    assert!(!cloud_page.contains("pending_pair_requests"));
    assert!(!cloud_page.contains("discovered_devices"));
    assert!(cloud_page.contains("SettingsPage::Cloud.index()"));
    assert!(cloud_page.contains("settings_create_cloud_webdav_page"));
    assert!(cloud_page.contains("settings_create_cloud_lan_page"));
    assert!(cloud_lan_devices.contains("pending_pair_requests"));
    assert!(cloud_lan_devices.contains("discovered_devices"));
    assert!(cloud_lan_devices.contains("settings_lan_selected_row"));
    assert!(cloud_webdav.contains("pub(super) unsafe fn settings_create_cloud_webdav_page("));
    assert!(cloud_webdav.contains("IDC_SET_CLOUD_SYNC_NOW"));
    assert!(cloud_webdav.contains("IDC_SET_CLOUD_RESTORE_BACKUP"));
    assert!(cloud_lan.contains("pub(super) unsafe fn settings_create_cloud_lan_page("));
    assert!(cloud_lan.contains("IDC_SET_LAN_DISCOVERED_LIST"));
    assert!(cloud_lan.contains("IDC_SET_LAN_QR_ANDROID"));
}

#[test]
fn windows_settings_owner_draw_lives_outside_hosts_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let hosts = app_hosts_source();
    let owner_draw = settings_owner_draw_source();
    let owner_draw_qr = settings_owner_draw_qr_source();
    let owner_draw_link = settings_owner_draw_link_source();
    let owner_draw_roles = settings_owner_draw_roles_source();

    assert!(app.contains("mod settings_owner_draw;"));
    assert!(app.contains("mod settings_owner_draw_qr;"));
    assert!(app.contains("mod settings_owner_draw_link;"));
    assert!(app.contains("mod settings_owner_draw_roles;"));
    assert!(prelude.contains("use super::settings_owner_draw::*;"));
    assert!(prelude.contains("use super::settings_owner_draw_qr::*;"));
    assert!(prelude.contains("use super::settings_owner_draw_link::*;"));
    assert!(prelude.contains("use super::settings_owner_draw_roles::*;"));

    for function in ["fn settings_button_hover(", "fn settings_draw_button_item("] {
        assert!(
            !hosts.contains(function),
            "{function} should stay out of app/hosts.rs"
        );
        assert!(
            owner_draw.contains(function),
            "{function} should live in app/settings_owner_draw.rs"
        );
    }

    for function in [
        "fn settings_lan_qr_payload(",
        "fn settings_qr_cache_for_payload",
        "fn draw_settings_qr_payload(",
        "fn draw_settings_qr_item(",
    ] {
        assert!(
            !hosts.contains(function),
            "{function} should stay out of app/hosts.rs"
        );
        assert!(
            owner_draw_qr.contains(function),
            "{function} should live in app/settings_owner_draw_qr.rs"
        );
    }

    for detail in [
        "settings_qr_render_plan",
        "IDC_SET_LAN_QR_ANDROID",
        "open_source_url()",
        "IDC_SET_AUTOSTART",
        "IDC_SET_GROUP_VIEW_RECORDS",
    ] {
        assert!(
            !owner_draw.contains(detail),
            "{detail} should live in an owner-draw domain module"
        );
    }
    assert!(owner_draw.contains("settings_owner_draw_is_qr(cid)"));
    assert!(owner_draw.contains("settings_owner_draw_is_toggle(cid)"));
    assert!(owner_draw.contains("draw_settings_source_link_item"));
    assert!(owner_draw.contains("settings_owner_draw_button_kind(st, cid)"));
    assert!(owner_draw.contains("draw_settings_toggle_component"));
    assert!(owner_draw.contains("draw_settings_button_component"));
    assert!(owner_draw_qr.contains("settings_qr_render_plan"));
    assert!(owner_draw_qr.contains("IDC_SET_LAN_QR_ANDROID"));
    assert!(owner_draw_qr.contains("qr_payload_cache_reuses_same_payload_and_rebuilds_on_change"));
    assert!(owner_draw_link.contains("pub(super) unsafe fn draw_settings_source_link_item"));
    assert!(owner_draw_link.contains("open_source_url()"));
    assert!(owner_draw_roles.contains("pub(super) fn settings_owner_draw_is_toggle"));
    assert!(owner_draw_roles.contains("pub(super) fn settings_owner_draw_button_kind"));
    assert!(owner_draw_roles.contains("IDC_SET_AUTOSTART"));
    assert!(owner_draw_roles.contains("IDC_SET_GROUP_VIEW_RECORDS"));
}

#[test]
fn windows_settings_page_builder_lives_outside_hosts_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let hosts = app_hosts_source();
    let page_builder = settings_page_builder_source();
    let control_factory = settings_control_factory_source();
    let control_registry = settings_control_registry_source();
    let form_actions = settings_form_actions_source();
    let form_fields = settings_form_fields_source();
    let raw_controls = settings_raw_controls_source();

    assert!(app.contains("mod settings_page_builder;"));
    assert!(app.contains("mod settings_raw_controls;"));
    assert!(app.contains("mod settings_form_actions;"));
    assert!(app.contains("mod settings_form_fields;"));
    assert!(app.contains("mod settings_control_factory;"));
    assert!(app.contains("mod settings_control_registry;"));
    assert!(prelude.contains("use super::settings_page_builder::*;"));
    assert!(prelude.contains("use super::settings_control_factory::*;"));
    assert!(prelude.contains("use super::settings_control_registry::*;"));

    for item in ["struct SettingsPageBuilder", "impl SettingsPageBuilder"] {
        assert!(
            !hosts.contains(item),
            "{item} should stay out of app/hosts.rs"
        );
        assert!(
            page_builder.contains(item),
            "{item} should live in app/settings_page_builder.rs"
        );
    }

    for item in ["fn settings_register_ctrl(", "fn settings_page_push_ctrl("] {
        assert!(
            !hosts.contains(item),
            "{item} should stay out of app/hosts.rs"
        );
        assert!(
            !page_builder.contains(item),
            "{item} should stay out of app/settings_page_builder.rs"
        );
        assert!(
            control_registry.contains(item),
            "{item} should live in app/settings_control_registry.rs"
        );
    }

    for item in [
        "fn own_button(",
        "fn form_action_row(",
        "fn form_qr_action(",
        "fn own_toggle_row(",
    ] {
        assert!(
            !hosts.contains(item),
            "{item} should stay out of app/hosts.rs"
        );
        assert!(
            !page_builder.contains(item),
            "{item} should stay out of app/settings_page_builder.rs"
        );
        assert!(
            form_actions.contains(item),
            "{item} should live in app/settings_form_actions.rs"
        );
    }

    for item in [
        "fn form_label(",
        "fn form_value_label(",
        "fn form_value_label_auto(",
        "fn form_dropdown(",
        "fn form_edit(",
        "fn form_password_edit(",
        "fn form_button(",
    ] {
        assert!(
            !hosts.contains(item),
            "{item} should stay out of app/hosts.rs"
        );
        assert!(
            !page_builder.contains(item),
            "{item} should stay out of app/settings_page_builder.rs"
        );
        assert!(
            form_fields.contains(item),
            "{item} should live in app/settings_form_fields.rs"
        );
    }

    for item in [
        "fn label(",
        "fn label_auto(",
        "fn button(",
        "fn button_sized(",
        "fn dropdown(",
        "fn edit(",
        "fn password_edit(",
        "fn listbox(",
        "fn toggle_row(",
    ] {
        assert!(
            !hosts.contains(item),
            "{item} should stay out of app/hosts.rs"
        );
        assert!(
            !page_builder.contains(item),
            "{item} should stay out of app/settings_page_builder.rs"
        );
        assert!(
            raw_controls.contains(item),
            "{item} should live in app/settings_raw_controls.rs"
        );
    }

    for item in [
        "fn settings_create_label(",
        "fn settings_create_label_auto(",
        "fn settings_create_edit(",
        "fn settings_create_password_edit(",
        "fn settings_create_listbox(",
        "fn settings_create_small_btn(",
        "fn settings_create_dropdown_btn(",
        "fn settings_create_toggle_plain(",
    ] {
        assert!(
            !hosts.contains(item),
            "{item} should stay out of app/hosts.rs"
        );
        assert!(
            !page_builder.contains(item),
            "{item} should stay out of app/settings_page_builder.rs"
        );
        assert!(
            control_factory.contains(item),
            "{item} should live in app/settings_control_factory.rs"
        );
    }

    assert!(page_builder.contains("SettingsFormSectionLayout::new"));
    assert!(raw_controls.contains("settings_scale(32)"));
    assert!(raw_controls.contains("settings_scale(28)"));
    assert!(raw_controls.contains("settings_create_toggle_plain"));
    assert!(form_fields.contains("field_row_rect"));
    assert!(form_fields.contains("field_sized_row_rect"));
    assert!(form_fields.contains("field_full_rect"));
    assert!(form_actions.contains("st.ownerdraw_ctrls.push(hwnd)"));
    assert!(form_actions.contains("action_row_rects"));
    assert!(form_actions.contains("qr_action_layout"));
    assert!(control_registry.contains("SettingsCtrlReg::new"));
    assert!(control_registry.contains("settings_page_scrollable"));
    assert!(control_factory.contains("create_settings_dropdown_button"));
    assert!(control_factory.contains("create_settings_toggle_plain"));
}

#[test]
fn windows_settings_page_navigation_lives_outside_hosts_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let hosts = app_hosts_source();
    let navigation = settings_page_navigation_source();
    let navigation_controls = settings_page_navigation_controls_source();
    let navigation_scroll = settings_page_navigation_scroll_source();
    let navigation_switch = settings_page_navigation_switch_source();
    let navigation_domains = [
        navigation.as_str(),
        navigation_controls.as_str(),
        navigation_scroll.as_str(),
        navigation_switch.as_str(),
    ]
    .join("\n");

    assert!(app.contains("mod settings_page_navigation;"));
    assert!(!prelude.contains("use super::settings_page_navigation::*;"));
    for module in [
        "settings_page_navigation_controls",
        "settings_page_navigation_scroll",
        "settings_page_navigation_switch",
    ] {
        assert!(app.contains(&format!("mod {module};")));
        assert!(prelude.contains(&format!("use super::{module}::*;")));
    }

    for function in [
        "fn settings_repos_controls(",
        "fn settings_scroll_to(",
        "fn settings_scrollbar_show(",
        "fn settings_scroll(",
        "fn settings_show_page(",
    ] {
        assert!(
            !hosts.contains(function),
            "{function} should stay out of app/hosts.rs"
        );
        assert!(
            navigation_domains.contains(function),
            "{function} should live in settings page navigation domain modules"
        );
    }

    assert!(navigation_controls.contains("platform_window::defer_move_windows"));
    assert!(navigation_scroll.contains("settings_scroll_update_for_target"));
    assert!(navigation_scroll.contains("settings_repos_controls(hwnd, st, true)"));
    assert!(!navigation_scroll.contains("settings_repos_controls(hwnd, st, false)"));
    assert!(navigation_switch.contains("settings_page_switch_plan"));
    assert!(navigation_switch.contains("settings_host_set_visible"));
    assert!(navigation_switch.contains("settings_sync_page_state(st, page)"));
    for old_detail in [
        "settings_page_switch_plan",
        "settings_scroll_update_for_target",
        "settings_host_set_visible",
        "platform_window::defer_move_windows",
    ] {
        assert!(!navigation.contains(old_detail));
    }
}

#[test]
fn windows_settings_page_sync_lives_outside_hosts_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let hosts = app_hosts_source();
    let page_sync = settings_page_sync_source();
    let cloud_sync = settings_page_sync_cloud_source();
    let cloud_webdav_sync = settings_page_sync_cloud_webdav_source();
    let cloud_lan_sync = settings_page_sync_cloud_lan_source();
    let plugin_sync = settings_page_sync_plugin_source();

    assert!(app.contains("mod settings_page_sync;"));
    assert!(app.contains("mod settings_page_sync_cloud;"));
    assert!(app.contains("mod settings_page_sync_cloud_webdav;"));
    assert!(app.contains("mod settings_page_sync_cloud_lan;"));
    assert!(app.contains("mod settings_page_sync_plugin;"));
    assert!(prelude.contains("use super::settings_page_sync::*;"));
    assert!(prelude.contains("use super::settings_page_sync_cloud::*;"));
    assert!(prelude.contains("use super::settings_page_sync_cloud_webdav::*;"));
    assert!(prelude.contains("use super::settings_page_sync_cloud_lan::*;"));
    assert!(prelude.contains("use super::settings_page_sync_plugin::*;"));

    for function in [
        "fn multi_sync_mode_from_settings(",
        "fn settings_apply_multi_sync_mode(",
        "fn settings_normalize_multi_sync_mode(",
        "fn settings_sync_page_state(",
        "fn settings_sync_pos_fields_enabled(",
    ] {
        assert!(
            !hosts.contains(function),
            "{function} should stay out of app/hosts.rs"
        );
        assert!(
            page_sync.contains(function),
            "{function} should live in app/settings_page_sync.rs"
        );
    }

    assert!(page_sync.contains("SettingsPage::Plugin => settings_sync_plugin_page_state(st)"));
    assert!(page_sync.contains("SettingsPage::Cloud => settings_sync_cloud_page_state(st)"));
    assert!(!page_sync.contains("settings_relayout_plugin_page(st)"));
    assert!(!page_sync.contains("settings_plugin_show_enable"));
    assert!(!page_sync.contains("image_ocr_provider_display"));
    assert!(!page_sync.contains("text_translate_provider_display"));
    assert!(plugin_sync.contains("pub(super) unsafe fn settings_sync_plugin_page_state"));
    assert!(plugin_sync.contains("settings_relayout_plugin_page(st)"));
    assert!(plugin_sync.contains("settings_plugin_show_enable"));
    assert!(plugin_sync.contains("image_ocr_provider_display"));
    assert!(plugin_sync.contains("text_translate_provider_display"));
    assert!(!page_sync.contains("lan_trusted_summary_value_text"));
    assert!(!page_sync.contains("settings_lan_refresh_lists(st)"));
    assert!(!page_sync.contains("platform_gdi::invalidate_rect(hwnd, null(), 1)"));
    assert!(cloud_sync.contains("pub(super) unsafe fn settings_sync_cloud_page_state"));
    assert!(cloud_sync.contains("settings_sync_cloud_webdav_state(st, webdav_enabled)"));
    assert!(cloud_sync.contains("settings_sync_cloud_lan_state(st, lan_enabled)"));
    assert!(!cloud_sync.contains("lan_trusted_summary_value_text"));
    assert!(!cloud_sync.contains("settings_lan_refresh_lists(st)"));
    assert!(!cloud_sync.contains("platform_gdi::invalidate_rect(hwnd, null(), 1)"));
    assert!(!cloud_sync.contains("lan_receive_mode_display(&s.lan_receive_mode)"));
    assert!(page_sync.contains("settings_host_set_enabled"));
    assert!(cloud_sync.contains("multi_sync_mode_display(mode)"));
    assert!(cloud_webdav_sync.contains("pub(super) unsafe fn settings_sync_cloud_webdav_state"));
    assert!(cloud_webdav_sync.contains("localized_cloud_status_text"));
    assert!(cloud_webdav_sync.contains("settings_host_set_enabled"));
    assert!(cloud_lan_sync.contains("pub(super) unsafe fn settings_sync_cloud_lan_state"));
    assert!(cloud_lan_sync.contains("lan_trusted_summary_value_text"));
    assert!(cloud_lan_sync.contains("settings_lan_refresh_lists(st)"));
    assert!(cloud_lan_sync.contains("platform_gdi::invalidate_rect(hwnd, null(), 1)"));
    assert!(cloud_lan_sync.contains("lan_receive_mode_display(&s.lan_receive_mode)"));
    assert!(page_sync.contains("settings_invalidate_page_ctrls(st.parent_hwnd, st, page)"));
}

#[test]
fn windows_settings_toggle_state_lives_outside_hosts_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let hosts = app_hosts_source();
    let toggle_state = settings_toggle_state_source();
    let toggle_general = settings_toggle_state_general_source();
    let toggle_cloud = settings_toggle_state_cloud_source();
    let toggle_hotkey = settings_toggle_state_hotkey_source();
    let toggle_plugin = settings_toggle_state_plugin_source();
    let toggle_group = settings_toggle_state_group_source();

    assert!(app.contains("mod settings_toggle_state;"));
    assert!(app.contains("mod settings_toggle_state_general;"));
    assert!(app.contains("mod settings_toggle_state_cloud;"));
    assert!(app.contains("mod settings_toggle_state_hotkey;"));
    assert!(app.contains("mod settings_toggle_state_plugin;"));
    assert!(app.contains("mod settings_toggle_state_group;"));
    assert!(prelude.contains("use super::settings_toggle_state::*;"));
    assert!(prelude.contains("use super::settings_toggle_state_general::*;"));
    assert!(prelude.contains("use super::settings_toggle_state_cloud::*;"));
    assert!(prelude.contains("use super::settings_toggle_state_hotkey::*;"));
    assert!(prelude.contains("use super::settings_toggle_state_plugin::*;"));
    assert!(prelude.contains("use super::settings_toggle_state_group::*;"));
    assert!(!hosts.contains("fn settings_toggle_get("));
    assert!(!hosts.contains("fn settings_toggle_flip("));
    assert!(toggle_state.contains("pub(super) unsafe fn settings_toggle_get("));
    assert!(toggle_state.contains("pub(super) unsafe fn settings_toggle_flip("));
    for function in [
        "settings_toggle_general_get(st, cid)",
        "settings_toggle_cloud_get(st, cid)",
        "settings_toggle_hotkey_get(st, cid)",
        "settings_toggle_plugin_get(st, cid)",
        "settings_toggle_group_get(st, cid)",
        "settings_toggle_general_flip(st, cid)",
        "settings_toggle_cloud_flip(st, cid)",
        "settings_toggle_hotkey_flip(st, cid)",
        "settings_toggle_plugin_flip(st, cid)",
        "settings_toggle_group_flip(st, cid)",
    ] {
        assert!(
            toggle_state.contains(function),
            "{function} should be dispatched by settings_toggle_state.rs"
        );
    }
    for detail in [
        "IDC_SET_AUTOSTART",
        "IDC_SET_PLAIN_HK_ENABLE",
        "IDC_SET_LAN_ENABLE",
        "st.draft.quick_search_enabled",
        "st.draft.grouping_enabled",
    ] {
        assert!(
            !toggle_state.contains(detail),
            "{detail} should live in a settings toggle domain module"
        );
    }
    assert!(toggle_general.contains("pub(super) fn settings_toggle_general_get"));
    assert!(toggle_general.contains("pub(super) fn settings_toggle_general_flip"));
    assert!(toggle_general.contains("IDC_SET_AUTOSTART"));
    assert!(toggle_general.contains("st.draft.quick_delete_button"));
    assert!(toggle_cloud.contains("pub(super) fn settings_toggle_cloud_get"));
    assert!(toggle_cloud.contains("pub(super) fn settings_toggle_cloud_flip"));
    assert!(toggle_cloud.contains("IDC_SET_CLOUD_ENABLE"));
    assert!(toggle_cloud.contains("IDC_SET_LAN_ENABLE"));
    assert!(toggle_hotkey.contains("pub(super) fn settings_toggle_hotkey_get"));
    assert!(toggle_hotkey.contains("pub(super) fn settings_toggle_hotkey_flip"));
    assert!(toggle_hotkey.contains("IDC_SET_PLAIN_HK_ENABLE"));
    assert!(toggle_hotkey.contains("st.draft.hotkey_enabled"));
    assert!(toggle_plugin.contains("pub(super) fn settings_toggle_plugin_get"));
    assert!(toggle_plugin.contains("pub(super) fn settings_toggle_plugin_flip"));
    assert!(toggle_plugin.contains("st.draft.quick_search_enabled"));
    assert!(toggle_plugin.contains("st.draft.wps_taskpane_enabled"));
    assert!(toggle_group.contains("pub(super) fn settings_toggle_group_get"));
    assert!(toggle_group.contains("pub(super) fn settings_toggle_group_flip"));
    assert!(toggle_group.contains("IDC_SET_GROUP_ENABLE"));
}

#[test]
fn windows_settings_host_helpers_live_outside_hosts_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let hosts = app_hosts_source();
    let helpers = settings_host_helpers_source();

    assert!(app.contains("mod settings_host_helpers;"));
    assert!(prelude.contains("use super::settings_host_helpers::*;"));

    for function in [
        "fn settings_set_text(",
        "fn settings_show_enable(",
        "fn settings_invalidate_page_ctrls(",
        "fn settings_refresh_theme_resources(",
    ] {
        assert!(
            !hosts.contains(function),
            "{function} should stay out of app/hosts.rs"
        );
        assert!(
            helpers.contains(function),
            "{function} should live in app/settings_host_helpers.rs"
        );
    }

    assert!(helpers.contains("settings_host_set_text"));
    assert!(helpers.contains("settings_host_set_visible_enabled"));
    assert!(helpers.contains("create_solid_brush"));
    assert!(helpers.contains("settings_viewport_rect"));
}

#[test]
fn windows_settings_apply_collect_live_in_dedicated_modules() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let hosts = app_hosts_source();
    let app_apply = settings_app_apply_source();
    let app_collect = settings_app_collect_source();
    let app_collect_general = settings_app_collect_general_source();
    let app_collect_hotkey = settings_app_collect_hotkey_source();
    let app_collect_plugin = settings_app_collect_plugin_source();
    let app_collect_group = settings_app_collect_group_source();
    let app_collect_cloud = settings_app_collect_cloud_source();
    let app_collect_domains = [
        app_collect_general.as_str(),
        app_collect_hotkey.as_str(),
        app_collect_plugin.as_str(),
        app_collect_group.as_str(),
        app_collect_cloud.as_str(),
    ]
    .join("\n");
    let effects = settings_app_effects_source();
    let effect_state = settings_app_effect_state_source();
    let integration_effects = settings_app_integration_effects_source();
    let data_effects = settings_app_data_effects_source();
    let window_effects = settings_app_window_effects_source();

    assert!(app.contains("mod settings_app_apply;"));
    assert!(prelude.contains("use super::settings_app_apply::*;"));
    assert!(app.contains("mod settings_app_collect;"));
    assert!(prelude.contains("use super::settings_app_collect::*;"));
    for module in [
        "settings_app_collect_general",
        "settings_app_collect_hotkey",
        "settings_app_collect_plugin",
        "settings_app_collect_group",
        "settings_app_collect_cloud",
    ] {
        assert!(app.contains(&format!("mod {module};")));
        assert!(prelude.contains(&format!("use super::{module}::*;")));
    }
    assert!(!app.contains("mod settings_app_sync;"));
    assert!(!prelude.contains("use super::settings_app_sync::*;"));
    assert!(app.contains("mod settings_app_effects;"));
    assert!(prelude.contains("use super::settings_app_effects::*;"));
    assert!(app.contains("mod settings_app_effect_state;"));
    assert!(prelude.contains("use super::settings_app_effect_state::*;"));
    assert!(app.contains("mod settings_app_integration_effects;"));
    assert!(prelude.contains("use super::settings_app_integration_effects::*;"));
    assert!(app.contains("mod settings_app_data_effects;"));
    assert!(prelude.contains("use super::settings_app_data_effects::*;"));
    assert!(app.contains("mod settings_app_window_effects;"));
    assert!(prelude.contains("use super::settings_app_window_effects::*;"));

    assert!(!hosts.contains("fn settings_apply_from_app("));
    assert!(!hosts.contains("fn settings_collect_to_app("));
    assert!(app_apply.contains("pub(super) unsafe fn settings_apply_from_app("));
    assert!(app_collect.contains("pub(super) unsafe fn settings_collect_to_app("));

    assert!(!app_apply.contains("settings_host_text"));
    assert!(!app_collect.contains("settings_host_text"));
    assert!(app_collect.contains("settings_collect_general_to_draft(st)"));
    assert!(app_collect.contains("settings_collect_hotkey_to_draft(st)"));
    assert!(app_collect.contains("settings_collect_plugin_to_draft(st)"));
    assert!(app_collect.contains("settings_collect_group_to_draft(st)"));
    assert!(app_collect.contains("settings_collect_cloud_to_draft(st)"));
    assert!(app_collect.contains("settings_commit_collected_app_settings(st, &mut *pst)"));
    assert!(app_collect_general.contains("settings_dropdown_max_items_from_label_opt"));
    assert!(app_collect_general.contains("paste_sound_key_from_display"));
    assert!(app_collect_hotkey.contains("normalize_hotkey_mod"));
    assert!(app_collect_hotkey.contains("plain_paste_hotkey_key"));
    assert!(app_collect_plugin.contains("search_engine_key_from_display"));
    assert!(app_collect_plugin.contains("image_ocr_provider_key_from_display"));
    assert!(app_collect_plugin.contains("text_translate_provider_key_from_display"));
    assert!(app_collect_group.contains("settings_vv_source_current"));
    assert!(app_collect_group.contains("settings_groups_cache_for_tab"));
    assert!(app_collect_cloud.contains("multi_sync_mode_from_label"));
    assert!(app_collect_cloud.contains("lan_receive_mode_from_label"));
    assert!(app_collect_domains.contains("settings_host_text"));
    for side_effect in [
        "save_settings(&app.settings)",
        "apply_autostart(app.settings.auto_start)",
        "register_hotkey_for(st.parent_hwnd, app)",
        "crate::lan_sync::refresh_service",
        "sync_peer_windows_from_settings(st.parent_hwnd)",
    ] {
        assert!(
            !app_apply.contains(side_effect)
                && !app_collect.contains(side_effect)
                && !app_collect_domains.contains(side_effect),
            "{side_effect} should stay out of settings app apply/collect modules"
        );
    }
    assert!(effects.contains("pub(super) unsafe fn settings_commit_collected_app_settings("));
    assert!(effects.contains("SettingsAppEffectBaseline::capture(app)"));
    assert!(effects.contains("settings_refresh_integrations_after_commit(st, app, &baseline)"));
    assert!(effects.contains("settings_refresh_data_after_commit(st, app)"));
    assert!(effects.contains("settings_refresh_windows_after_commit(st, app, &baseline)"));
    assert!(effect_state.contains("struct SettingsAppEffectBaseline"));
    assert!(integration_effects.contains("apply_autostart(app.settings.auto_start)"));
    assert!(integration_effects.contains("register_hotkey_for(st.parent_hwnd, app)"));
    assert!(integration_effects.contains("register_plain_paste_hotkey_for(st.parent_hwnd, app)"));
    assert!(integration_effects.contains("update_vv_mode_hook"));
    assert!(data_effects.contains("crate::lan_sync::refresh_service"));
    assert!(data_effects.contains("db_prune_items"));
    assert!(data_effects.contains("reload_state_from_db_persisting(app)"));
    assert!(window_effects.contains("sync_peer_windows_from_settings(st.parent_hwnd)"));
    assert!(window_effects.contains("platform_gdi::invalidate_rect"));
}

#[test]
fn windows_settings_page_ensure_lives_outside_hosts_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let hosts = app_hosts_source();
    let ensure = settings_page_ensure_source();

    assert!(app.contains("mod settings_page_ensure;"));
    assert!(prelude.contains("use super::settings_page_ensure::*;"));
    assert!(!hosts.contains("fn settings_ensure_page("));
    assert!(ensure.contains("pub(super) unsafe fn settings_ensure_page("));
    assert!(ensure.contains("settings_create_general_page(hwnd, st)"));
    assert!(ensure.contains("settings_create_hotkey_page(hwnd, st)"));
    assert!(ensure.contains("settings_create_plugin_page(hwnd, st)"));
    assert!(ensure.contains("settings_create_group_page(hwnd, st)"));
    assert!(ensure.contains("settings_create_cloud_page(hwnd, st)"));
    assert!(ensure.contains("settings_create_about_page(hwnd, st)"));
}

#[test]
fn windows_settings_hosts_module_is_retired() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();

    assert!(!app.contains("mod hosts;"));
    assert!(!app.contains("pub(crate) mod hosts;"));
    assert!(!prelude.contains("use super::hosts::*;"));
    assert!(prelude.contains("lan_receive_mode_from_label"));
    assert!(prelude.contains("multi_sync_mode_from_label"));
    assert!(prelude.contains("MULTI_SYNC_MODE_OPTIONS"));
}

#[test]
fn windows_remaining_app_helpers_live_outside_app_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let state_runtime = app_state_runtime_source();
    let main_view_helpers = main_view_helpers_source();
    let platform_helpers = platform_helpers_source();

    assert!(app.contains("mod main_view_helpers;"));
    assert!(app.contains("mod platform_helpers;"));
    assert!(!app.contains("\nfn "));
    assert!(!app.contains("\nunsafe fn "));
    assert!(state_runtime.contains("fn reload_state_from_db_persisting("));

    for function in [
        "fn main_theme_role_color(",
        "fn pt_in_rect(",
        "fn row_supports_image_preview(",
        "fn scroll_to_top_visible(",
        "fn main_title_button_visibility(",
        "fn main_empty_state_kind(",
        "unsafe fn hovered_item_clone(",
    ] {
        assert!(
            !app.contains(function),
            "{function} should stay out of app.rs"
        );
        assert!(
            main_view_helpers.contains(function),
            "{function} should live in app/main_view_helpers.rs"
        );
    }

    for function in [
        "fn copy_text_to_clipboard_in_background(",
        "fn show_native_dialog_message(",
        "fn confirm_native_dialog(",
    ] {
        assert!(
            !app.contains(function),
            "{function} should stay out of app.rs"
        );
        assert!(
            platform_helpers.contains(function),
            "{function} should live in app/platform_helpers.rs"
        );
    }
}

#[test]
fn windows_app_prelude_and_constants_own_adapter_imports_and_ids() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let constants = app_constants_source();

    assert!(app.contains("mod constants;"));
    assert!(app.contains("mod prelude;"));
    assert!(app.contains("pub(crate) use self::constants::TRAY_UID;"));
    assert!(app.contains("pub(crate) use self::prelude::{ClipItem, ClipKind};"));
    assert!(!app.contains("use crate::app_core::{"));
    assert!(!app.contains("use windows_sys::Win32::{"));
    assert!(!app.contains("const MAIN_TIMER_IDS:"));
    assert!(!app.contains("const SETTINGS_TIMER_IDS:"));
    assert!(!app.contains("type AppResult"));

    for required in ["use crate::app_core::{", "use windows_sys::Win32::{"] {
        assert!(
            prelude.contains(required),
            "app/prelude.rs should own shared import {required}"
        );
    }
    assert!(prelude.contains("use super::constants::*;"));
    assert!(!prelude.contains("const MAIN_TIMER_IDS:"));
    assert!(!prelude.contains("const SETTINGS_TIMER_IDS:"));
    assert!(!prelude.contains("type AppResult"));

    for required in [
        "use crate::app_core::{",
        "const MAIN_TIMER_IDS:",
        "const SETTINGS_TIMER_IDS:",
        "const MAIN_UI_LAYOUT:",
        "type AppResult",
    ] {
        assert!(
            constants.contains(required),
            "app/constants.rs should own {required}"
        );
    }

    for (module, source) in [
        ("main_clipboard_capture.rs", main_clipboard_capture_source()),
        ("main_events.rs", main_events_source()),
        ("settings_window.rs", settings_window_source()),
        ("state.rs", app_state_source()),
    ] {
        assert!(
            source.starts_with("use super::prelude::*;"),
            "app/{module} should consume app/prelude.rs instead of globbing app.rs"
        );
    }
}

#[test]
fn windows_settings_scroll_capture_uses_settings_host() {
    let settings_window = settings_window_source();
    let settings_input = settings_input_source();
    let settings_host = include_str!("settings_ui_host.rs").replace("\r\n", "\n");

    let cancel_start = settings_input
        .find("unsafe fn cancel_settings_scroll_drag")
        .unwrap();
    let cancel_end = settings_input[cancel_start..]
        .find("\npub(super) unsafe fn handle_settings_pointer_move")
        .map(|offset| cancel_start + offset)
        .unwrap();
    let cancel_block = &settings_input[cancel_start..cancel_end];

    let down_start = settings_input
        .find("unsafe fn handle_settings_lbutton_down")
        .unwrap();
    let down_end = settings_input[down_start..]
        .find("\npub(super) unsafe fn handle_settings_lbutton_up")
        .map(|offset| down_start + offset)
        .unwrap();
    let down_block = &settings_input[down_start..down_end];
    let move_start = settings_input
        .find("unsafe fn handle_settings_pointer_move")
        .unwrap();
    let move_end = settings_input[move_start..]
        .find("\npub(super) unsafe fn handle_settings_pointer_leave")
        .map(|offset| move_start + offset)
        .unwrap();
    let move_block = &settings_input[move_start..move_end];
    let leave_start = settings_input
        .find("unsafe fn handle_settings_pointer_leave")
        .unwrap();
    let leave_end = settings_input[leave_start..]
        .find("\npub(super) unsafe fn handle_settings_lbutton_down")
        .map(|offset| leave_start + offset)
        .unwrap();
    let leave_block = &settings_input[leave_start..leave_end];

    let helper_start = settings_window
        .find("fn capture_settings_pointer(hwnd: HWND)")
        .unwrap();
    let helper_end = settings_window[helper_start..]
        .find("\npub(super) fn repaint_settings_window_area")
        .map(|offset| helper_start + offset)
        .unwrap();
    let helper_block = &settings_window[helper_start..helper_end];

    assert!(cancel_block.contains("release_settings_pointer(hwnd)"));
    assert!(cancel_block.contains("repaint_settings_window(hwnd, false)"));
    assert!(!cancel_block.contains("platform_input::release_capture()"));
    assert!(!cancel_block.contains("platform_gdi::invalidate_rect(hwnd, null(), 0)"));
    assert!(move_block.contains("settings_window_client_bounds(hwnd)"));
    assert!(move_block.contains("settings_window_track_pointer_leave(hwnd)"));
    assert!(move_block.contains("settings_host_control_at_point(hwnd, position)"));
    assert!(move_block.contains("repaint_settings_window_area(hwnd, Some(rect), false)"));
    assert!(!move_block.contains("platform_window::client_rect(hwnd)"));
    assert!(!move_block.contains("ensure_mouse_leave_tracking(hwnd)"));
    assert!(!move_block.contains("platform_window::child_from_point_ex("));
    assert!(!move_block.contains("CHILD_FROM_POINT_SKIP_"));
    assert!(!move_block.contains("platform_gdi::invalidate_rect(hwnd, &rect, 0)"));
    assert!(leave_block.contains("repaint_settings_window_area(hwnd, Some(rect), false)"));
    assert!(!leave_block.contains("platform_gdi::invalidate_rect(hwnd, &rect, 0)"));
    assert!(down_block.contains("capture_settings_pointer(hwnd)"));
    assert!(down_block.contains("settings_window_client_bounds(hwnd)"));
    assert!(
        down_block.contains("repaint_settings_window_area(hwnd, Some((&viewport).into()), false)")
    );
    assert!(down_block.contains("repaint_settings_window(hwnd, false)"));
    assert!(!down_block.contains("platform_input::set_capture(hwnd)"));
    assert!(!down_block.contains("platform_window::client_rect(hwnd)"));
    assert!(!down_block.contains("platform_gdi::invalidate_rect(hwnd, null(), 0)"));
    assert!(helper_block.contains(".capture_settings_pointer(hwnd)"));
    assert!(helper_block.contains(".release_settings_pointer(hwnd)"));
    assert!(settings_host.contains("fn capture_settings_pointer(&mut self"));
    assert!(settings_host.contains("platform_input::set_capture(handle)"));
    assert!(settings_host.contains("fn release_settings_pointer(&mut self"));
    assert!(settings_host.contains("platform_input::release_capture()"));
    assert!(settings_host.contains("fn settings_window_client_bounds("));
    assert!(settings_host.contains("platform_window::client_rect(handle)"));
}

#[test]
fn windows_settings_dropdown_lifecycle_uses_dropdown_host() {
    let settings_dropdown = settings_dropdown_source();
    let dropdown_requests = [
        settings_dropdown.clone(),
        settings_dropdown_general_source(),
        settings_dropdown_cloud_source(),
        settings_dropdown_hotkey_source(),
        settings_dropdown_group_source(),
        settings_dropdown_plugin_source(),
    ]
    .join("\n");
    let dropdown_host = settings_dropdown_host_source();
    let settings_input = settings_input_source();
    let settings_host = include_str!("settings_ui_host.rs").replace("\r\n", "\n");
    let open_start = settings_dropdown
        .find("unsafe fn open_settings_dropdown_for_control")
        .unwrap();
    let command_start = settings_dropdown[open_start..]
        .find("\npub(super) unsafe fn open_settings_config_file")
        .map(|offset| open_start + offset)
        .unwrap_or(settings_dropdown.len());
    let open_block = &settings_dropdown[open_start..command_start];
    let close_start = dropdown_host
        .find("unsafe fn close_settings_dropdown_popup")
        .unwrap();
    let config_start = dropdown_host[close_start..]
        .find("\npub(super) fn present_settings_dropdown_popup")
        .map(|offset| close_start + offset)
        .unwrap();
    let close_block = &dropdown_host[close_start..config_start];
    let down_start = settings_input
        .find("unsafe fn handle_settings_lbutton_down")
        .unwrap();
    let down_end = settings_input[down_start..]
        .find("\npub(super) unsafe fn handle_settings_lbutton_up")
        .map(|offset| down_start + offset)
        .unwrap();
    let down_block = &settings_input[down_start..down_end];
    let destroy_start = settings_input
        .find("unsafe fn handle_settings_destroy")
        .unwrap();
    let destroy_end = settings_input[destroy_start..]
        .find("\npub(super) unsafe fn dispatch_settings_ui_event")
        .map(|offset| destroy_start + offset)
        .unwrap();
    let destroy_block = &settings_input[destroy_start..destroy_end];

    assert!(dropdown_requests.contains("present_settings_dropdown_popup("));
    assert!(dropdown_requests.contains("settings_control_screen_rect_or_empty("));
    assert!(!open_block.contains("show_settings_dropdown_popup("));
    assert!(!open_block.contains("window_rect_or_empty(st.cb_"));
    assert!(close_block.contains("destroy_settings_dropdown_popup(st.dropdown_popup)"));
    assert!(!close_block.contains("platform_window::destroy(st.dropdown_popup)"));
    assert!(close_block.contains("settings_dropdown_popup_exists(st.dropdown_popup)"));
    assert!(down_block.contains("settings_dropdown_popup_exists(st.dropdown_popup)"));
    assert!(down_block.contains("settings_dropdown_popup_bounds(st.dropdown_popup)"));
    assert!(down_block.contains("settings_window_client_to_screen(hwnd"));
    assert!(destroy_block.contains("settings_dropdown_popup_exists((*st_ptr).dropdown_popup)"));
    for block in [close_block, down_block, destroy_block] {
        assert!(!block.contains("platform_window::exists(st.dropdown_popup)"));
        assert!(!block.contains("platform_window::exists((*st_ptr).dropdown_popup)"));
    }
    assert!(!down_block.contains("platform_window::window_rect(st.dropdown_popup)"));
    assert!(!down_block.contains("platform_window::client_to_screen(hwnd"));
    assert!(dropdown_host.contains("fn settings_dropdown_popup_exists(handle: HWND) -> bool"));
    assert!(dropdown_host.contains("WindowsWindowIdentityHost::new().exists(handle)"));
    assert!(settings_input.contains("settings_dropdown_popup_bounds"));
    assert!(settings_input.contains("settings_window_client_to_screen"));
    assert!(!settings_dropdown.contains("show_settings_dropdown_popup("));
    assert!(dropdown_host.contains("WindowsSettingsDropdownHost"));
    assert!(settings_host.contains("pub struct WindowsSettingsDropdownHost"));
    assert!(
        settings_host.contains("impl NativeSettingsDropdownHost for WindowsSettingsDropdownHost")
    );
    assert!(settings_host.contains("fn present_settings_dropdown("));
    assert!(settings_host.contains("fn destroy_settings_dropdown(&mut self"));
    assert!(settings_host.contains("fn settings_dropdown_bounds(&self"));
    assert!(settings_host.contains("platform_window::window_rect(handle)"));
    assert!(settings_host.contains("fn settings_window_client_to_screen("));
    assert!(settings_host.contains("platform_window::client_to_screen(handle"));
    assert!(settings_host.contains("fn control_screen_bounds(&self"));
    assert!(settings_host.contains("settings_host_screen_bounds"));
}

#[test]
fn windows_vv_popup_window_presentation_uses_transient_host() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let production = app
        .split("\n#[cfg(test)]\nmod tests")
        .next()
        .unwrap_or(&app);
    let main_window = main_window_source();
    let vv_popup = vv_popup_source();
    let hosts = app_hosts_source();
    let transient_host = transient_window_host_source();
    let ime_host = include_str!("platform/ime.rs").replace("\r\n", "\n");
    let text_caret_host = include_str!("platform/text_caret.rs").replace("\r\n", "\n");
    let create_start = vv_popup.find("unsafe fn vv_popup_hwnd").unwrap();
    let create_end = vv_popup[create_start..]
        .find("\n")
        .and_then(|_| vv_popup[create_start..].find("current_vv_popup_hwnd"))
        .map(|offset| create_start + offset)
        .unwrap();
    let move_start = vv_popup
        .find("unsafe fn vv_popup_move_near_target")
        .unwrap();
    let move_end = vv_popup[move_start..]
        .find("vv_popup_sync_hook_state")
        .map(|offset| move_start + offset)
        .unwrap();
    let hide_start = vv_popup
        .find("pub(super) unsafe fn vv_popup_hide(_hwnd: HWND, state: &mut AppState)")
        .unwrap();
    let show_start = vv_popup
        .find("pub(super) unsafe fn vv_popup_show(hwnd: HWND, state: &mut AppState, target: HWND)")
        .unwrap();
    let proc_start = vv_popup
        .find("unsafe extern \"system\" fn vv_popup_wnd_proc")
        .unwrap();
    let ignored_start = vv_popup[show_start..]
        .find("\nunsafe extern \"system\" fn vv_popup_wnd_proc")
        .map(|offset| show_start + offset + 1)
        .unwrap();
    let destroy_start = main_window.find("unsafe fn handle_main_destroy").unwrap();
    let destroy_end = main_window[destroy_start..]
        .find("\npub(super) unsafe fn handle_main_dpi_changed")
        .map(|offset| destroy_start + offset)
        .unwrap();
    let destroy_block = &main_window[destroy_start..destroy_end];
    let ime_start = vv_popup.find("unsafe fn vv_imm_overlay_anchor").unwrap();
    let ime_end = vv_popup[ime_start..]
        .find("\nunsafe fn vv_focus_rect_anchor")
        .map(|offset| ime_start + offset)
        .unwrap();
    let thread_caret_start = vv_popup.find("unsafe fn vv_thread_caret_anchor").unwrap();
    let thread_caret_end = vv_popup[thread_caret_start..]
        .find("\nunsafe fn vv_accessible_caret_anchor")
        .map(|offset| thread_caret_start + offset)
        .unwrap();
    let accessible_caret_start = vv_popup
        .find("unsafe fn vv_accessible_caret_anchor")
        .unwrap();
    let accessible_caret_end = vv_popup[accessible_caret_start..]
        .find("\nunsafe fn vv_imm_overlay_anchor")
        .map(|offset| accessible_caret_start + offset)
        .unwrap();
    let focus_rect_start = vv_popup.find("unsafe fn vv_focus_rect_anchor").unwrap();
    let focus_rect_end = vv_popup[focus_rect_start..]
        .find("\nunsafe fn vv_cursor_anchor")
        .map(|offset| focus_rect_start + offset)
        .unwrap();
    let cursor_start = vv_popup.find("unsafe fn vv_cursor_anchor").unwrap();
    let cursor_end = vv_popup[cursor_start..]
        .find("\nunsafe fn vv_focus_hwnd_for_target")
        .map(|offset| cursor_start + offset)
        .unwrap();
    let focus_handle_start = vv_popup.find("unsafe fn vv_focus_hwnd_for_target").unwrap();
    let focus_handle_end = vv_popup[focus_handle_start..]
        .find("\nfn present_vv_popup_window")
        .map(|offset| focus_handle_start + offset)
        .unwrap();
    let create_block = &vv_popup[create_start..create_end];
    let move_block = &vv_popup[move_start..move_end];
    let hide_block = &vv_popup[hide_start..show_start];
    let show_block = &vv_popup[show_start..ignored_start];
    let proc_block = &vv_popup[proc_start..];
    let ime_block = &vv_popup[ime_start..ime_end];
    let thread_caret_block = &vv_popup[thread_caret_start..thread_caret_end];
    let accessible_caret_block = &vv_popup[accessible_caret_start..accessible_caret_end];
    let focus_rect_block = &vv_popup[focus_rect_start..focus_rect_end];
    let cursor_block = &vv_popup[cursor_start..cursor_end];
    let focus_handle_block = &vv_popup[focus_handle_start..focus_handle_end];

    assert!(production.contains("mod vv_popup;"));
    assert!(!production.contains("unsafe fn vv_popup_hwnd"));
    assert!(!production.contains("unsafe fn vv_popup_move_near_target"));
    assert!(!production.contains("unsafe extern \"system\" fn vv_popup_wnd_proc"));
    assert!(create_block.contains("create_transient_window(NativeTransientWindowRequest"));
    assert!(create_block.contains("NativeTransientWindowPresentation::Created"));
    assert!(create_block.contains("NativeTransientWindowPresentation::Failed"));
    assert!(destroy_block.contains("destroy_vv_popup_window(popup)"));
    assert!(!create_block.contains("register_class_ex"));
    assert!(!create_block.contains("create_window_ex"));
    assert!(!create_block.contains("to_wide(VV_POPUP_CLASS)"));
    assert!(move_block.contains("present_vv_popup_window("));
    assert!(move_block.contains("vv_popup_layout_for_window(focus_hwnd)"));
    assert!(hide_block.contains("hide_vv_popup_window("));
    assert!(show_block.contains("vv_popup_move_near_target(state, popup)"));
    assert!(thread_caret_block.contains("WindowsTextCaretHost::new().thread_caret_anchor(target)"));
    assert!(accessible_caret_block
        .contains("WindowsTextCaretHost::new().accessible_caret_anchor(focus_hwnd)"));
    assert!(focus_rect_block.contains("WindowsTextCaretHost::new().focus_rect_anchor("));
    assert!(cursor_block.contains("WindowsTextCaretHost::new().cursor_anchor()"));
    assert!(
        focus_handle_block.contains("WindowsTextCaretHost::new().focus_handle_for_target(target)")
    );
    assert!(ime_block.contains("WindowsImeHost::new()"));
    assert!(ime_block.contains("candidate_anchor(focus_hwnd, index)"));
    assert!(ime_block.contains("composition_anchor(focus_hwnd)"));
    assert!(proc_block.contains("WindowsPasteTargetHost::new()"));
    assert!(proc_block.contains("force_paste_target_foreground(state.vv_popup_target)"));
    assert!(proc_block.contains("vv_popup_move_near_target(state, hwnd)"));
    for block in [move_block, hide_block, show_block, proc_block] {
        assert!(!block.contains("platform_window::set_pos("));
        assert!(!block.contains("platform_window::show_no_activate"));
        assert!(!block.contains("platform_window::hide("));
    }
    assert!(!ime_block.contains("WM_IME_CONTROL"));
    assert!(!ime_block.contains("CandidateForm"));
    assert!(!ime_block.contains("CompositionForm"));
    assert!(!thread_caret_block.contains("GUITHREADINFO"));
    assert!(!thread_caret_block.contains("platform_window::gui_thread_info"));
    assert!(!thread_caret_block.contains("platform_window::client_to_screen"));
    assert!(!accessible_caret_block.contains("platform_accessibility::caret_rect"));
    assert!(!focus_rect_block.contains("platform_window::window_rect"));
    assert!(!cursor_block.contains("platform_input::cursor_pos"));
    assert!(!focus_handle_block.contains("GUITHREADINFO"));
    assert!(!proc_block.contains("platform_window::force_foreground(state.vv_popup_target)"));
    assert!(!destroy_block.contains("platform_window::destroy(popup)"));
    assert!(proc_block.contains("vv_popup_layout_for_window(hwnd).with_width"));
    assert!(proc_block.contains("WM_SIZE"));
    assert!(!hosts.contains("pub(super) struct WindowsTransientWindowHost"));
    assert!(transient_host.contains("pub(super) struct WindowsTransientWindowHost"));
    assert!(
        transient_host.contains("impl NativeTransientWindowHost for WindowsTransientWindowHost")
    );
    assert!(transient_host.contains("WS_POPUP | WS_THICKFRAME"));
    assert!(transient_host.contains("fn destroy_transient_window(&mut self"));
    assert!(ime_host.contains("pub(crate) struct WindowsImeHost"));
    assert!(ime_host.contains("impl NativeImeHost for WindowsImeHost"));
    assert!(ime_host.contains("WM_IME_CONTROL"));
    assert!(ime_host.contains("CandidateForm"));
    assert!(ime_host.contains("CompositionForm"));
    assert!(text_caret_host.contains("pub(crate) struct WindowsTextCaretHost"));
    assert!(text_caret_host.contains("impl NativeTextCaretHost for WindowsTextCaretHost"));
    assert!(text_caret_host.contains("GUITHREADINFO"));
    assert!(text_caret_host.contains("platform_accessibility::caret_rect"));
    assert!(text_caret_host.contains("platform_input::cursor_pos"));
    assert!(text_caret_host.contains("fn focus_handle_for_target"));
}

#[test]
fn windows_window_identity_queries_use_identity_host() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let clipboard_capture = main_clipboard_capture_source();
    let main_events = main_events_source();
    let settings_actions = settings_actions_source();
    let vv_hook = vv_hook_source();
    let production = app
        .split("\n#[cfg(test)]\nmod tests")
        .next()
        .unwrap_or(&app);
    let identity_host = include_str!("platform/window_identity.rs").replace("\r\n", "\n");
    let ignored_start = vv_hook.find("unsafe fn vv_target_is_ignored").unwrap();
    let source_start = clipboard_capture
        .find("unsafe fn clipboard_source_app_name")
        .unwrap();
    let foreground_start = clipboard_capture
        .find("unsafe fn foreground_source_app_name")
        .unwrap();
    let process_start = vv_hook
        .find("pub(super) unsafe fn window_process_name")
        .unwrap();
    let class_start = vv_hook
        .find("pub(super) unsafe fn vv_window_class_name")
        .unwrap();
    let backspace_start = vv_hook
        .find("pub(super) unsafe fn vv_backspace_count_for_target_window")
        .unwrap();
    let hook_start = vv_hook
        .find("unsafe extern \"system\" fn vv_keyboard_hook_proc")
        .unwrap();
    let hook_end = vv_hook[hook_start..]
        .find("\npub(super) unsafe fn update_vv_mode_hook")
        .map(|offset| hook_start + offset)
        .unwrap();
    let vv_watch_start = main_events.find("MainTimerTask::VvWatch =>").unwrap();
    let vv_show_timer_start = main_events.find("MainTimerTask::VvShow =>").unwrap();
    let vv_show_timer_end = main_events[vv_show_timer_start..]
        .find("\n        MainTimerTask::Paste")
        .map(|offset| vv_show_timer_start + offset)
        .unwrap();
    let vv_show_event_start = main_events
        .find("ApplicationEvent::VvShowRequested")
        .unwrap();
    let vv_show_event_end = main_events[vv_show_event_start..]
        .find("\n        ApplicationEvent::VvHideRequested")
        .map(|offset| vv_show_event_start + offset)
        .unwrap();
    let capture_skip_start = settings_actions
        .find("SettingsAction::CaptureSkippedWindowClass =>")
        .unwrap();
    let capture_skip_end = settings_actions[capture_skip_start..]
        .find("\n        SettingsAction::RestoreSearchEnginePreset")
        .map(|offset| capture_skip_start + offset)
        .unwrap();
    let ignored_end = vv_hook[ignored_start..]
        .find("\npub(super) unsafe fn vv_window_class_name")
        .map(|offset| ignored_start + offset)
        .unwrap();
    let ignored_block = &vv_hook[ignored_start..ignored_end];
    let source_block = &clipboard_capture[source_start..foreground_start];
    let foreground_end = clipboard_capture[foreground_start..]
        .find("\nfn is_self_clipboard_source_app")
        .map(|offset| foreground_start + offset)
        .unwrap();
    let foreground_block = &clipboard_capture[foreground_start..foreground_end];
    let process_end = vv_hook[process_start..]
        .find("\npub(super) unsafe fn send_escape_key")
        .map(|offset| process_start + offset)
        .unwrap();
    let process_block = &vv_hook[process_start..process_end];
    let class_end = vv_hook[class_start..]
        .find("\npub(super) fn vv_is_qq_wps_process")
        .map(|offset| class_start + offset)
        .unwrap();
    let class_block = &vv_hook[class_start..class_end];
    let backspace_end = vv_hook[backspace_start..]
        .find("\nunsafe fn vv_target_is_text_input_ready")
        .map(|offset| backspace_start + offset)
        .unwrap();
    let backspace_block = &vv_hook[backspace_start..backspace_end];
    let hook_block = &vv_hook[hook_start..hook_end];
    let vv_watch_block = &main_events[vv_watch_start..vv_show_timer_start];
    let vv_show_timer_block = &main_events[vv_show_timer_start..vv_show_timer_end];
    let vv_show_event_block = &main_events[vv_show_event_start..vv_show_event_end];
    let capture_skip_block = &settings_actions[capture_skip_start..capture_skip_end];

    assert!(production.contains("mod vv_hook;"));
    assert!(!production.contains("unsafe fn clipboard_source_app_name"));
    assert!(!production.contains("unsafe fn foreground_source_app_name"));
    assert!(!production.contains("unsafe fn window_process_name"));
    assert!(!production.contains("unsafe fn vv_target_is_ignored"));
    assert!(!production.contains("unsafe extern \"system\" fn vv_keyboard_hook_proc"));
    assert!(ignored_block.contains("current_vv_popup_hwnd()"));
    assert!(
        ignored_block.contains("WindowsWindowIdentityHost::new().is_current_process_window(hwnd)")
    );
    assert!(source_block.contains("WindowsWindowIdentityHost::new()"));
    assert!(source_block.contains("identity_host.root_handle(owner)"));
    assert!(source_block.contains("identity_host.foreground_handle()"));
    assert!(source_block.contains("identity_host.process_name("));
    assert!(foreground_block.contains("identity_host.foreground_handle()"));
    assert!(foreground_block.contains("identity_host.process_name(foreground)"));
    assert!(process_block.contains("WindowsWindowIdentityHost::new().process_name(hwnd)"));
    assert!(class_block.contains("WindowsWindowIdentityHost::new().class_name(hwnd)"));
    assert!(backspace_block.contains("WindowsWindowIdentityHost::new().root_handle(target)"));
    assert!(hook_block.contains("WindowsWindowIdentityHost::new()"));
    assert!(hook_block.contains("identity_host.foreground_handle()"));
    assert!(hook_block.contains("identity_host.exists(fg)"));
    assert!(hook_block.contains("identity_host.exists(target)"));
    assert!(vv_watch_block.contains("WindowsWindowIdentityHost::new()"));
    assert!(vv_watch_block.contains("identity_host.is_foreground(state.vv_popup_target)"));
    assert!(vv_watch_block.contains("identity_host.exists(state.vv_popup_target)"));
    assert!(vv_show_timer_block.contains("WindowsWindowIdentityHost::new()"));
    assert!(vv_show_timer_block.contains("identity_host.exists(target)"));
    assert!(vv_show_timer_block.contains("identity_host.is_foreground(target)"));
    assert!(vv_show_event_block.contains("WindowsWindowIdentityHost::new()"));
    assert!(vv_show_event_block.contains("identity_host.foreground_handle()"));
    assert!(vv_show_event_block.contains("identity_host.exists(target)"));
    assert!(vv_show_event_block.contains("identity_host.exists(foreground)"));
    assert!(capture_skip_block.contains("WindowsWindowIdentityHost::new()"));
    assert!(capture_skip_block.contains("identity_host.exists(target)"));
    assert!(capture_skip_block.contains("identity_host.is_current_process_window(target)"));
    for block in [
        ignored_block,
        source_block,
        foreground_block,
        process_block,
        class_block,
        backspace_block,
        hook_block,
        vv_watch_block,
        vv_show_timer_block,
        vv_show_event_block,
        capture_skip_block,
    ] {
        assert!(!block.contains("platform_window::window_process_id"));
        assert!(!block.contains("platform_process::process_image_name"));
        assert!(!block.contains("platform_window::class_name"));
        assert!(!block.contains("platform_window::root_ancestor"));
        assert!(!block.contains("platform_window::foreground"));
        assert!(!block.contains("platform_window::exists("));
        assert!(!block.contains("platform_window::is_foreground("));
    }
    assert!(identity_host.contains("impl NativeWindowIdentityHost for WindowsWindowIdentityHost"));
    assert!(identity_host.contains("platform_window::window_process_id"));
    assert!(identity_host.contains("platform_process::process_image_name"));
    assert!(identity_host.contains("platform_window::class_name"));
    assert!(identity_host.contains("platform_window::root_ancestor"));
    assert!(identity_host.contains("platform_window::foreground"));
}

#[test]
fn windows_paste_target_state_queries_use_window_identity_host() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let production = app
        .split("\n#[cfg(test)]\nmod tests")
        .next()
        .unwrap_or(&app);
    let main_paste = main_paste_source();
    let can_send_start = main_paste
        .find("unsafe fn can_send_ctrl_v_to_target")
        .unwrap();
    let failure_start = main_paste
        .find("unsafe fn paste_failure_message_for_target")
        .unwrap();
    let show_failure_start = main_paste
        .find("unsafe fn show_paste_failure_message")
        .unwrap();
    let effective_start = main_paste.find("unsafe fn effective_paste_target").unwrap();
    let paste_ready_start = main_paste
        .find("unsafe fn paste_after_clipboard_ready")
        .unwrap();
    let can_send_block = &main_paste[can_send_start..failure_start];
    let failure_block = &main_paste[failure_start..show_failure_start];
    let effective_block = &main_paste[effective_start..paste_ready_start];

    assert!(production.contains("mod main_paste;"));
    assert!(!production.contains("unsafe fn can_send_ctrl_v_to_target"));
    assert!(!production.contains("unsafe fn paste_failure_message_for_target"));
    assert!(!production.contains("unsafe fn effective_paste_target"));
    assert!(can_send_block.contains("WindowsWindowIdentityHost::new()"));
    assert!(can_send_block.contains("identity_host.exists(target)"));
    assert!(can_send_block.contains("identity_host.is_foreground(target)"));
    assert!(can_send_block.contains(".paste_target_focus_status("));
    assert!(can_send_block.contains(".allows_paste_attempt()"));
    assert!(failure_block.contains("WindowsWindowIdentityHost::new()"));
    assert!(failure_block.contains("identity_host.exists(target)"));
    assert!(failure_block.contains("identity_host.is_foreground(target)"));
    assert!(failure_block.contains("PasteTargetFocusStatus::Unknown"));
    assert!(failure_block.contains("PasteTargetFocusStatus::NoActiveFocus"));
    assert!(failure_block.contains("PasteTargetFocusStatus::OutsideTarget"));
    assert!(failure_block.contains("PasteTargetFocusStatus::InsideTarget"));
    assert!(effective_block.contains("WindowsWindowIdentityHost::new().foreground_handle()"));
    for block in [can_send_block, failure_block, effective_block] {
        assert!(!block.contains("platform_window::exists(target)"));
        assert!(!block.contains("platform_window::is_foreground(target)"));
        assert!(!block.contains("platform_window::foreground()"));
        assert!(!block.contains("GUITHREADINFO"));
        assert!(!block.contains("platform_window::gui_thread_info"));
        assert!(!block.contains("platform_window::window_thread_id(target)"));
    }
}

#[test]
fn windows_paste_target_discovery_lives_outside_hosts_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let hosts = app_hosts_source();
    let discovery = main_paste_target_discovery_source();

    assert!(app.contains("mod main_paste_target_discovery;"));
    assert!(prelude.contains("use super::main_paste_target_discovery::*;"));
    for forbidden in [
        "fn paste_skip_class_tokens",
        "pub(super) unsafe fn is_viable_paste_window",
        "pub(super) unsafe fn find_next_paste_target(",
        "pub(super) unsafe fn find_next_paste_target_after(",
        "pub(super) fn append_unique_skip_class_name",
    ] {
        assert!(
            !hosts.contains(forbidden),
            "paste target discovery should not live in hosts.rs: {forbidden}"
        );
    }
    for required in [
        "fn paste_skip_class_tokens",
        "pub(super) unsafe fn is_viable_paste_window",
        "pub(super) unsafe fn find_next_paste_target(",
        "pub(super) unsafe fn find_next_paste_target_after(",
        "pub(super) fn append_unique_skip_class_name",
        "platform_window::visible_enabled_top_level_windows",
        "platform_window::root_ancestor",
        "WS_EX_TOOLWINDOW",
    ] {
        assert!(
            discovery.contains(required),
            "main_paste_target_discovery.rs should contain {required}"
        );
    }
}

#[test]
fn windows_low_level_input_lives_outside_hosts_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let hosts = app_hosts_source();
    let low_level = main_low_level_input_source();
    let main_events = main_events_source();
    let main_window = main_window_source();

    assert!(app.contains("mod main_low_level_input;"));
    assert!(
        app.contains("pub(crate) use self::main_low_level_input::refresh_low_level_input_hooks;")
    );
    assert!(prelude.contains("use super::main_low_level_input::*;"));
    for forbidden in [
        "fn screen_point_hits_window_scope",
        "fn should_ignore_outside_click_for_point",
        "fn refresh_outside_hide_timers",
        "fn refresh_edge_auto_hide_timers",
        "fn quick_escape_keyboard_hook_proc",
        "pub(crate) unsafe fn refresh_low_level_input_hooks",
        "pub(super) unsafe fn handle_outside_hide_tick",
    ] {
        assert!(
            !hosts.contains(forbidden),
            "low-level input behavior should not live in hosts.rs: {forbidden}"
        );
    }
    for required in [
        "fn screen_point_hits_window_scope",
        "fn should_ignore_outside_click_for_point",
        "fn refresh_outside_hide_timers",
        "fn refresh_edge_auto_hide_timers",
        "fn quick_escape_keyboard_hook_proc",
        "pub(crate) unsafe fn refresh_low_level_input_hooks",
        "pub(crate) unsafe fn shutdown_low_level_input_hooks",
        "pub(super) unsafe fn handle_outside_hide_tick",
        "platform_hook::install_low_level_keyboard",
        "ID_TIMER_OUTSIDE_HIDE",
        "ID_TIMER_EDGE_AUTO_HIDE",
    ] {
        assert!(
            low_level.contains(required),
            "main_low_level_input.rs should contain {required}"
        );
    }
    assert!(main_events.contains("MainTimerTask::OutsideHide => handle_outside_hide_tick(hwnd)"));
    assert!(main_window.contains("!edge_window_scope_contains_point(hwnd, pt)"));
    assert!(!main_events.contains("super::hosts::handle_outside_hide_tick"));
    assert!(!main_window.contains("super::hosts::edge_window_scope_contains_point"));
}

#[test]
fn windows_main_hover_preview_lives_outside_hosts_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let hosts = app_hosts_source();
    let hover = main_hover_preview_source();
    let main_events = main_events_source();
    let edge = main_edge_auto_hide_source();

    assert!(app.contains("mod main_hover_preview;"));
    assert!(prelude.contains("use super::main_hover_preview::*;"));
    for forbidden in [
        "pub(super) unsafe fn ensure_mouse_leave_tracking",
        "pub(super) unsafe fn hover_preview_blocked_at_point",
        "fn refresh_hover_preview",
        "pub(super) unsafe fn handle_mouse_hover_main",
        "pub(super) unsafe fn handle_mouse_leave_main",
    ] {
        assert!(
            !hosts.contains(forbidden),
            "main hover preview behavior should not live in hosts.rs: {forbidden}"
        );
    }
    for required in [
        "pub(super) unsafe fn ensure_mouse_leave_tracking",
        "pub(super) unsafe fn hover_preview_blocked_at_point",
        "fn refresh_hover_preview",
        "pub(super) unsafe fn handle_mouse_hover_main",
        "pub(super) unsafe fn handle_mouse_leave_main",
        "platform_input::track_mouse_leave_and_hover",
        "platform_system_parameters::mouse_hover_time_ms",
        "show_hover_preview",
        "hide_hover_preview",
    ] {
        assert!(
            hover.contains(required),
            "main_hover_preview.rs should contain {required}"
        );
    }
    assert!(main_events.contains("UiEvent::PointerHover { position } => handle_mouse_hover_main"));
    assert!(main_events.contains("UiEvent::PointerLeave => handle_mouse_leave_main(hwnd)"));
    assert!(edge.contains("ensure_mouse_leave_tracking(hwnd)"));
}

#[test]
fn windows_main_window_registry_lives_outside_hosts_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let hosts = app_hosts_source();
    let registry = main_window_registry_source();

    assert!(app.contains("mod main_window_registry;"));
    assert!(app.contains(
        "pub(crate) use self::main_window_registry::{get_state_ptr, quick_window_hwnd};"
    ));
    assert!(prelude.contains("use super::main_window_registry::*;"));
    for forbidden in [
        "struct WindowHosts",
        "static WINDOW_HOSTS",
        "fn window_hosts()",
        "pub(super) fn set_window_host",
        "pub(super) fn clear_window_host",
        "pub(super) fn window_host_hwnds",
        "pub(super) unsafe fn set_ignore_clipboard_for_all_hosts",
        "pub(super) fn is_app_window",
        "pub(crate) unsafe fn get_state_ptr",
        "pub(super) unsafe fn get_state_mut",
    ] {
        assert!(
            !hosts.contains(forbidden),
            "main window registry should not live in hosts.rs: {forbidden}"
        );
    }
    for required in [
        "struct WindowHosts",
        "static WINDOW_HOSTS",
        "fn window_hosts()",
        "pub(super) fn set_window_host",
        "pub(super) fn clear_window_host",
        "pub(super) fn window_host_hwnds",
        "pub(super) fn window_host_hwnds_try",
        "pub(super) unsafe fn set_ignore_clipboard_for_all_hosts",
        "pub(super) unsafe fn skip_next_clipboard_update_for_all_hosts",
        "pub(super) fn is_app_window",
        "pub(crate) unsafe fn get_state_ptr",
        "pub(super) unsafe fn get_state_mut",
    ] {
        assert!(
            registry.contains(required),
            "main_window_registry.rs should contain {required}"
        );
    }
}

#[test]
fn windows_main_hover_clear_lives_with_hover_preview() {
    let hosts = app_hosts_source();
    let hover = main_hover_preview_source();

    for forbidden in [
        "pub(super) unsafe fn clear_main_hover_state",
        "pub(super) unsafe fn main_window_should_stay_noactivate",
    ] {
        assert!(
            !hosts.contains(forbidden),
            "main hover clear behavior should not live in hosts.rs: {forbidden}"
        );
    }
    for required in [
        "pub(super) unsafe fn clear_main_hover_state",
        "pub(super) unsafe fn main_window_should_stay_noactivate",
        "main_hover_target_from_state(state).clear_transition(false)",
        "hit_test_row(state, x, y) >= 0",
    ] {
        assert!(
            hover.contains(required),
            "main_hover_preview.rs should contain {required}"
        );
    }
}

#[test]
fn windows_startup_integrations_live_outside_hosts_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let hosts = app_hosts_source();
    let startup = main_startup_integrations_source();
    let main_events = main_events_source();
    let main_entry = main_entry_source();

    assert!(app.contains("mod main_startup_integrations;"));
    assert!(prelude.contains("use super::main_startup_integrations::*;"));
    for forbidden in [
        "static TASKBAR_CREATED_MESSAGE",
        "pub(super) fn taskbar_created_message",
        "pub(super) unsafe fn sync_main_tray_icon",
        "pub(super) unsafe fn retry_startup_integrations",
        "fn startup_integrations_need_retry",
        "pub(super) fn arm_startup_recovery_if_needed",
        "pub(super) unsafe fn notify_update_state_changed",
    ] {
        assert!(
            !hosts.contains(forbidden),
            "startup integration behavior should not live in hosts.rs: {forbidden}"
        );
    }
    for required in [
        "static TASKBAR_CREATED_MESSAGE",
        "pub(super) fn taskbar_created_message",
        "pub(super) unsafe fn sync_main_tray_icon",
        "pub(super) unsafe fn retry_startup_integrations",
        "fn startup_integrations_need_retry",
        "pub(super) fn arm_startup_recovery_if_needed",
        "pub(super) unsafe fn notify_update_state_changed",
        "register_hotkey_for",
        "register_plain_paste_hotkey_for",
        "register_clipboard_listener_for",
        "update_vv_mode_hook",
    ] {
        assert!(
            startup.contains(required),
            "main_startup_integrations.rs should contain {required}"
        );
    }
    assert!(main_events.contains("retry_startup_integrations(hwnd"));
    assert!(main_entry.contains("sync_main_tray_icon(hwnd, state)"));
}

#[test]
fn windows_main_window_refresh_lives_outside_hosts_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let hosts = app_hosts_source();
    let refresh = main_window_refresh_source();
    let cloud = main_cloud_sync_source();
    let state_runtime = app_state_runtime_source();

    assert!(app.contains("mod main_window_refresh;"));
    assert!(app.contains("pub(crate) use self::main_window_refresh::refresh_window_for_show;"));
    assert!(prelude.contains("use super::main_window_refresh::*;"));
    for forbidden in [
        "pub(super) unsafe fn refresh_settings_window_from_app",
        "pub(super) unsafe fn apply_loaded_settings",
        "pub(super) unsafe fn refresh_window_state",
        "pub(super) unsafe fn sync_peer_windows_from_db",
        "pub(super) unsafe fn sync_peer_windows_from_settings",
        "pub(crate) unsafe fn refresh_window_for_show",
    ] {
        assert!(
            !hosts.contains(forbidden),
            "main window refresh behavior should not live in hosts.rs: {forbidden}"
        );
    }
    for required in [
        "pub(super) unsafe fn refresh_settings_window_from_app",
        "pub(super) unsafe fn apply_loaded_settings",
        "pub(super) unsafe fn refresh_window_state",
        "pub(super) unsafe fn sync_peer_windows_from_db",
        "pub(super) unsafe fn sync_peer_windows_from_settings",
        "pub(crate) unsafe fn refresh_window_for_show",
        "reload_state_from_db_persisting",
        "refresh_low_level_input_hooks",
        "sync_main_tray_icon",
    ] {
        assert!(
            refresh.contains(required),
            "main_window_refresh.rs should contain {required}"
        );
    }
    assert!(cloud.contains("apply_loaded_settings(hwnd, state)"));
    assert!(cloud.contains("sync_peer_windows_from_settings(hwnd)"));
    assert!(state_runtime.contains("sync_peer_windows_from_db(self.hwnd)"));
}

#[test]
fn windows_main_window_creation_path_uses_main_window_host() {
    let main_entry = main_entry_source();
    let hosts = app_hosts_source();
    let main_window_host = main_window_host_source();
    let start = main_entry.find("pub(crate) fn run()").unwrap();
    let end = main_entry[start..]
        .find("\npub(super) unsafe extern \"system\" fn wnd_proc")
        .map(|offset| start + offset)
        .unwrap();
    let run_block = &main_entry[start..end];

    assert!(run_block.contains("WindowsMainWindowHost::new"));
    assert!(run_block.contains("crate::zsui::Window::new(app_title())"));
    assert!(run_block.contains("NativeMainWindowRequest::from_zsui_window_for_host"));
    assert!(run_block.contains("HostCapabilities::windows_native_window_host()"));
    assert!(run_block.contains("NativeMainWindowPresentation::Failed"));
    assert!(!run_block.contains("register_class_ex"));
    assert!(!run_block.contains("create_window_ex"));
    assert!(!hosts.contains("pub(super) struct WindowsMainWindowHost"));
    assert!(main_window_host.contains("pub(super) struct WindowsMainWindowHost"));
    assert!(main_window_host.contains("impl NativeMainWindowHost for WindowsMainWindowHost"));
}

#[test]
fn windows_main_window_adapter_lives_outside_app_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let main_window = main_window_source();

    assert!(app.contains("mod main_window;"));
    for forbidden in [
        "unsafe fn reclaim_hidden_window_memory",
        "unsafe fn handle_main_window_size",
        "unsafe fn handle_main_close_requested",
        "unsafe fn refresh_main_window_metrics",
        "unsafe fn ensure_main_window_size_for_monitor",
        "fn capture_main_pointer(hwnd: HWND)",
        "fn begin_main_window_drag(hwnd: HWND)",
    ] {
        assert!(
            !app.contains(forbidden),
            "main window adapter code should not live in app.rs: {forbidden}"
        );
    }

    for required in [
        "pub(super) unsafe fn handle_main_window_size",
        "pub(super) unsafe fn handle_main_close_requested",
        "pub(crate) unsafe fn refresh_main_window_layout_for_monitor",
        "pub(crate) fn set_main_window_activation_policy",
        "pub(crate) fn set_main_window_bounds",
        "WindowsMainWindowHost::new(Some(wnd_proc))",
        "main_window_layout_dpi(hwnd)",
        "main_window_bounds(hwnd)",
    ] {
        assert!(
            main_window.contains(required),
            "main_window.rs should contain {required}"
        );
    }
}

#[test]
fn windows_main_edge_auto_hide_lives_outside_hosts_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let hosts = app_hosts_source();
    let edge = main_edge_auto_hide_source();

    assert!(app.contains("mod main_edge_auto_hide;"));
    assert!(prelude.contains("use super::main_edge_auto_hide::*;"));
    for forbidden in [
        "fn edge_choose_dock_side",
        "fn begin_edge_animation",
        "fn edge_hidden_position",
        "pub(crate) fn clear_edge_dock_state",
        "pub(super) unsafe fn handle_edge_auto_hide_tick",
        "pub(crate) unsafe fn note_window_moved_for_edge_hide",
    ] {
        assert!(
            !hosts.contains(forbidden),
            "edge auto-hide implementation should not live in hosts.rs: {forbidden}"
        );
    }
    for required in [
        "pub(crate) fn clear_edge_dock_state",
        "fn edge_choose_dock_side",
        "fn begin_edge_animation",
        "fn edge_hidden_position",
        "pub(super) unsafe fn handle_edge_auto_hide_tick",
        "pub(crate) unsafe fn note_window_moved_for_edge_hide",
        "platform_monitor::nearest_work_rect_for_window",
        "platform_window::set_pos(",
    ] {
        assert!(
            edge.contains(required),
            "main_edge_auto_hide.rs should contain {required}"
        );
    }
}

#[test]
fn windows_main_event_executor_lives_outside_app_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let main_events = main_events_source();

    assert!(app.contains("mod main_events;"));
    assert!(prelude.contains("use super::main_events::*;"));
    for forbidden in [
        "unsafe fn execute_main_menu_command",
        "unsafe fn execute_main_ui_command",
        "unsafe fn drain_main_ui_commands",
        "unsafe fn handle_main_timer_task",
        "unsafe fn dispatch_main_ui_event",
        "unsafe fn handle_main_application_event",
        "unsafe fn handle_main_async_event",
        "unsafe fn handle_text_processing_result",
    ] {
        assert!(
            !app.contains(forbidden),
            "main event executor code should not live in app.rs: {forbidden}"
        );
    }

    for required in [
        "pub(super) unsafe fn execute_main_menu_command",
        "pub(super) unsafe fn execute_main_ui_command",
        "pub(super) unsafe fn drain_main_ui_commands",
        "pub(super) unsafe fn handle_main_timer_task",
        "pub(super) unsafe fn dispatch_main_ui_event",
        "pub(super) unsafe fn handle_main_application_event",
        "pub(super) unsafe fn handle_main_async_event",
        "pub(super) unsafe fn handle_text_processing_result",
        "main_host_execution_plan(action)",
        "main_timer_task_for_id(id as usize, MAIN_TIMER_IDS)",
        "ApplicationEvent::LanSyncReady",
        "MainAsyncEvent::ImagePaste(payload)",
    ] {
        assert!(
            main_events.contains(required),
            "main_events.rs should contain {required}"
        );
    }
}

#[test]
fn windows_main_entry_adapter_lives_outside_app_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let main_entry = main_entry_source();

    assert!(app.contains("mod main_entry;"));
    assert!(app.contains("pub(crate) use self::main_entry::run;"));
    for forbidden in [
        "pub fn run() -> AppResult<()>",
        "unsafe extern \"system\" fn wnd_proc",
        "unsafe fn on_create",
        "unsafe fn handle_control_command",
        "fn normalize_plain_text_for_paste",
        "unsafe fn stop_search_debounce_timer",
        "unsafe fn apply_search_filter",
        "unsafe fn cancel_main_scroll_drag",
        "unsafe fn settings_set_hotkey_recording",
        "unsafe fn handle_vv_select",
    ] {
        assert!(
            !app.contains(forbidden),
            "main entry adapter code should not live in app.rs: {forbidden}"
        );
    }

    for required in [
        "pub(crate) fn run() -> AppResult<()>",
        "pub(super) unsafe extern \"system\" fn wnd_proc",
        "pub(super) unsafe fn on_create",
        "pub(super) unsafe fn handle_control_command",
        "pub(super) fn normalize_plain_text_for_paste",
        "pub(super) unsafe fn stop_search_debounce_timer",
        "pub(super) unsafe fn apply_search_filter",
        "pub(super) unsafe fn cancel_main_scroll_drag",
        "pub(super) unsafe fn settings_set_hotkey_recording",
        "pub(super) unsafe fn handle_vv_select",
        "NativeMainWindowRequest::from_zsui_window_for_host",
        "crate::zsui::Window::new(app_title())",
        "WM_GETMINMAXINFO",
        "apply_min_track_size",
        "main_window_host_event_from_message(msg, wparam, lparam)",
        "UiEvent::Lifecycle(LifecycleEvent::Mount)",
    ] {
        assert!(
            main_entry.contains(required),
            "main_entry.rs should contain {required}"
        );
    }
}

#[test]
fn windows_main_window_appearance_uses_main_window_host() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let main_entry = main_entry_source();
    let main_window = main_window_source();
    let production = app
        .split("\n#[cfg(test)]\nmod tests")
        .next()
        .unwrap_or(&app);
    let main_window_host = main_window_host_source();
    let on_create_start = main_entry.find("unsafe fn on_create").unwrap();
    let on_create_end = main_entry[on_create_start..]
        .find("\npub(super) unsafe fn handle_control_command")
        .map(|offset| on_create_start + offset)
        .unwrap();
    let on_create_block = &main_entry[on_create_start..on_create_end];
    let size_start = main_window
        .find("pub(super) unsafe fn handle_main_window_size")
        .unwrap();
    let size_end = main_window[size_start..]
        .find("\npub(super) unsafe fn handle_main_app_activation_changed")
        .map(|offset| size_start + offset)
        .unwrap();
    let size_block = &main_window[size_start..size_end];

    assert!(production.contains("mod main_window;"));
    assert!(!production.contains("unsafe fn handle_main_window_size"));
    assert!(on_create_block.contains("apply_main_window_appearance(hwnd)"));
    assert!(on_create_block.contains("set_main_window_app_icon("));
    assert!(on_create_block.contains("NativeAppIconResource"));
    assert!(size_block.contains("apply_main_window_appearance(hwnd)"));
    assert!(!production.contains("unsafe fn apply_main_window_region"));
    assert!(!on_create_block.contains("platform_appearance::set_rounded_corners(hwnd)"));
    assert!(!on_create_block.contains("platform_appearance::apply_dark_mode_to_window(hwnd)"));
    assert!(!on_create_block.contains("WM_SETICON"));
    assert!(!size_block.contains("platform_appearance::set_rounded_corners(hwnd)"));
    assert!(!size_block.contains("platform_appearance::apply_dark_mode_to_window(hwnd)"));
    assert!(main_window_host.contains("fn apply_main_window_appearance(&mut self"));
    assert!(main_window_host.contains("fn set_main_window_app_icon("));
    assert!(main_window_host.contains("platform_appearance::set_rounded_corners(handle)"));
    assert!(main_window_host.contains("platform_appearance::apply_dark_mode_to_window(handle)"));
    assert!(main_window_host.contains("WM_SETICON"));
}

#[test]
fn release_workflow_bundles_macos_icon_and_ad_hoc_signature() {
    let workflow = include_str!("../.github/workflows/release-packages.yml");

    assert!(workflow.contains("iconutil -c icns"));
    assert!(workflow.contains("AppIcon.icns"));
    assert!(workflow.contains("<key>CFBundleIconFile</key>"));
    assert!(workflow.contains("<string>AppIcon</string>"));
    assert!(workflow.contains("codesign --force --deep --sign - dist/ZSClip.app"));
    assert!(workflow.contains("codesign --verify --deep --strict dist/ZSClip.app"));
    assert!(workflow.contains("xattr -dr com.apple.quarantine /Applications/ZSClip.app"));
    assert!(workflow.contains("zsclip-windows-x86_64-no-lan-portable.zip"));
    assert!(!workflow.contains("- 当前包未签名、未公证。"));
}

#[test]
fn windows_main_window_lifecycle_commands_use_main_window_host() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let main_events = main_events_source();
    let main_window = main_window_source();
    let production = app
        .split("\n#[cfg(test)]\nmod tests")
        .next()
        .unwrap_or(&app);
    let main_window_host = main_window_host_source();
    let command_start = main_events
        .find("unsafe fn execute_main_ui_command")
        .unwrap();
    let command_end = main_events[command_start..]
        .find("\npub(super) unsafe fn drain_main_ui_commands")
        .map(|offset| command_start + offset)
        .unwrap();
    let command_block = &main_events[command_start..command_end];
    let menu_start = main_events
        .find("unsafe fn execute_main_menu_command")
        .unwrap();
    let menu_end = main_events[menu_start..]
        .find("\npub(super) unsafe fn execute_main_ui_command")
        .map(|offset| menu_start + offset)
        .unwrap();
    let menu_block = &main_events[menu_start..menu_end];
    let close_start = main_window
        .find("pub(super) unsafe fn handle_main_close_requested")
        .unwrap();
    let close_end = main_window[close_start..]
        .find("\npub(super) unsafe fn handle_main_lifecycle_event")
        .map(|offset| close_start + offset)
        .unwrap();
    let close_block = &main_window[close_start..close_end];
    let destroy_start = main_window.find("unsafe fn handle_main_destroy").unwrap();
    let destroy_end = main_window[destroy_start..]
        .find("\npub(super) unsafe fn handle_main_dpi_changed")
        .map(|offset| destroy_start + offset)
        .unwrap();
    let destroy_block = &main_window[destroy_start..destroy_end];

    assert!(production.contains("mod main_window;"));
    assert!(!production.contains("unsafe fn handle_main_close_requested"));
    assert!(!production.contains("unsafe fn handle_main_destroy"));
    assert!(command_block.contains("hide_main_window(hwnd)"));
    assert!(command_block.contains("request_main_window_close(hwnd)"));
    assert!(command_block.contains("windows_main_action_from_native_specs(&command)"));
    assert!(main_events.contains("native_host_main_action_button_specs()"));
    assert!(!main_events.contains("native_host_main_action_component_specs()"));
    assert!(!main_events.contains("NativeComponentAction::HostUi(action)"));
    assert!(menu_block.contains("MainTrayActionPlan::Exit"));
    assert!(menu_block.contains("destroy_main_window(hwnd)"));
    assert!(close_block.contains("hide_main_window(hwnd)"));
    assert!(close_block.contains("destroy_main_window(hwnd)"));
    assert!(destroy_block.contains("destroy_main_window(quick)"));
    assert!(!command_block.contains("platform_window::hide(hwnd)"));
    assert!(!command_block.contains("platform_window::send_message(hwnd, WM_CLOSE"));
    assert!(!menu_block.contains("platform_window::destroy(hwnd)"));
    assert!(!close_block.contains("platform_window::hide(hwnd)"));
    assert!(!close_block.contains("platform_window::destroy(hwnd)"));
    assert!(!destroy_block.contains("platform_window::destroy(quick)"));
    assert!(main_window_host.contains("fn hide_main_window(&mut self"));
    assert!(main_window_host.contains("fn request_main_window_close(&mut self"));
    assert!(main_window_host.contains("fn destroy_main_window(&mut self"));
    assert!(main_window_host.contains("fn capture_main_pointer(&mut self"));
    assert!(main_window_host.contains("platform_input::set_capture(handle)"));
    assert!(main_window_host.contains("fn release_main_pointer(&mut self"));
    assert!(main_window_host.contains("platform_input::release_capture()"));
}

#[test]
fn windows_main_platform_bindings_lives_outside_app_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let bindings = main_platform_bindings_source();

    assert!(app.contains("mod main_platform_bindings;"));
    for forbidden in [
        "fn register_hotkey_for",
        "fn unregister_hotkey_for",
        "fn register_plain_paste_hotkey_for",
        "fn register_clipboard_listener_for",
        "unsafe fn handle_global_hotkey",
    ] {
        assert!(
            !app.contains(forbidden),
            "platform binding code should not live in app.rs: {forbidden}"
        );
    }

    assert!(bindings.contains("pub(super) fn register_hotkey_for"));
    assert!(bindings.contains("main_hotkey_registration_plan(MainHotkeyRegistrationInput"));
    assert!(bindings.contains("hotkey::register("));
    assert!(bindings.contains("hotkey::unregister("));
    assert!(bindings.contains("clipboard_listener::register("));
    assert!(bindings.contains("clipboard_listener::unregister("));
    assert!(bindings.contains("HOTKEY_ID => {"));
    assert!(bindings.contains("HOTKEY_ID_PLAIN => {"));
    assert!(bindings.contains("prepare_plain_paste_hotkey_target(hwnd, state)"));
    assert!(bindings.contains("paste_selected(hwnd, state)"));
    assert!(bindings.contains("state.paste_target_override = null_mut()"));

    let normal_start = bindings.find("HOTKEY_ID => {").unwrap();
    let plain_start = bindings.find("HOTKEY_ID_PLAIN => {").unwrap();
    let normal_block = &bindings[normal_start..plain_start];
    let plain_end = bindings[plain_start..]
        .find("\n        _ => {}")
        .map(|offset| plain_start + offset)
        .unwrap();
    let plain_block = &bindings[plain_start..plain_end];
    assert!(normal_block.contains("toggle_window_visibility_hotkey(hwnd)"));
    assert!(!plain_block.contains("toggle_window_visibility_hotkey(hwnd)"));
}

#[test]
fn windows_main_window_drag_capture_uses_main_window_host() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let main_entry = main_entry_source();
    let main_window = main_window_source();
    let row_tools = main_row_tools_source();
    let input = include_str!("app/main_input.rs").replace("\r\n", "\n");
    let production = app
        .split("\n#[cfg(test)]\nmod tests")
        .next()
        .unwrap_or(&app);
    let main_window_host = main_window_host_source();

    let down_start = input.find("unsafe fn handle_lbutton_down").unwrap();
    let down_end = input[down_start..]
        .find("\npub(super) unsafe fn handle_lbutton_up")
        .map(|offset| down_start + offset)
        .unwrap();
    let down_block = &input[down_start..down_end];

    let cancel_start = main_entry
        .find("unsafe fn cancel_main_scroll_drag")
        .unwrap();
    let cancel_end = main_entry[cancel_start..]
        .find("\npub(super) unsafe fn settings_set_hotkey_recording")
        .map(|offset| cancel_start + offset)
        .unwrap();
    let cancel_block = &main_entry[cancel_start..cancel_end];

    let export_start = row_tools
        .find("pub(super) unsafe fn begin_row_drag_export")
        .unwrap();
    let export_end = row_tools[export_start..]
        .find("\npub(super) fn clear_hotkey_passthrough_state")
        .map(|offset| export_start + offset)
        .unwrap();
    let export_block = &row_tools[export_start..export_end];

    let helper_start = main_window
        .find("pub(super) fn capture_main_pointer(hwnd: HWND)")
        .unwrap();
    let helper_block = &main_window[helper_start..];

    assert!(production.contains("mod main_window;"));
    assert!(production.contains("mod main_row_tools;"));
    assert!(!production.contains("fn capture_main_pointer(hwnd: HWND)"));
    assert!(!production.contains("unsafe fn begin_row_drag_export"));
    assert!(down_block.contains("begin_main_window_drag(hwnd)"));
    assert!(down_block.contains("capture_main_pointer(hwnd)"));
    assert!(!down_block.contains("platform_window::force_foreground(hwnd)"));
    assert!(!down_block.contains("platform_window::send_message("));
    assert!(!down_block.contains("WM_SYSCOMMAND"));
    assert!(!down_block.contains("SC_MOVE"));
    assert!(!down_block.contains("platform_input::release_capture()"));
    assert!(!down_block.contains("platform_input::set_capture(hwnd)"));
    assert!(cancel_block.contains("release_main_pointer(hwnd)"));
    assert!(!cancel_block.contains("platform_input::release_capture()"));
    assert!(export_block.contains("release_main_pointer(hwnd)"));
    assert!(!export_block.contains("platform_input::release_capture()"));
    assert!(helper_block.contains(".capture_main_pointer(hwnd)"));
    assert!(helper_block.contains(".release_main_pointer(hwnd)"));
    assert!(helper_block.contains(".begin_main_window_drag(hwnd)"));
    assert!(main_window_host.contains("fn capture_main_pointer(&mut self"));
    assert!(main_window_host.contains("fn release_main_pointer(&mut self"));
    assert!(main_window_host.contains("fn begin_main_window_drag(&mut self"));
    assert!(main_window_host.contains("platform_window::send_message("));
    assert!(main_window_host.contains("WM_SYSCOMMAND"));
    assert!(main_window_host.contains("SC_MOVE"));
}

#[test]
fn windows_main_window_activation_uses_main_window_host() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let production = app
        .split("\n#[cfg(test)]\nmod tests")
        .next()
        .unwrap_or(&app);
    let main_search = main_search_source();
    let main_window_host = main_window_host_source();
    let activate_start = main_search
        .find("unsafe fn activate_window_for_search_input")
        .unwrap();
    let activate_end = main_search[activate_start..]
        .find("\npub(super) unsafe fn search_visibility_plan_for_request")
        .map(|offset| activate_start + offset)
        .unwrap();
    let activate_block = &main_search[activate_start..activate_end];

    assert!(production.contains("mod main_search;"));
    assert!(!production.contains("unsafe fn activate_window_for_search_input"));
    assert!(activate_block.contains("set_main_window_activation_policy(hwnd, true)"));
    assert!(activate_block.contains("activate_main_window(hwnd)"));
    assert!(activate_block.contains("focus_search(state.search_hwnd)"));
    assert!(!activate_block.contains("platform_window::show(hwnd)"));
    assert!(!activate_block.contains("platform_window::set_pos("));
    assert!(!activate_block.contains("platform_window::force_foreground(hwnd)"));
    assert!(main_window_host.contains("fn activate_main_window(&mut self"));
    assert!(main_window_host.contains("fn set_main_window_activation_policy("));
    assert!(
        main_window_host.contains("apply_main_window_noactivate_mode(handle, !allow_activation)")
    );
    assert!(main_window_host.contains("platform_window::show(handle)"));
    assert!(main_window_host.contains("platform_window::force_foreground(handle)"));
    assert!(!production.contains("set_main_window_noactivate_mode"));
}

#[test]
fn windows_main_window_bounds_updates_use_main_window_host() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let main_window = main_window_source();
    let production = app
        .split("\n#[cfg(test)]\nmod tests")
        .next()
        .unwrap_or(&app);
    let main_window_host = main_window_host_source();
    let metrics_start = main_window
        .find("unsafe fn refresh_main_window_metrics")
        .unwrap();
    let metrics_end = main_window[metrics_start..]
        .find("\npub(super) unsafe fn ensure_main_window_size_for_monitor")
        .map(|offset| metrics_start + offset)
        .unwrap();
    let metrics_block = &main_window[metrics_start..metrics_end];
    let ensure_start = metrics_end;
    let ensure_end = main_window[ensure_start..]
        .find("\nunsafe fn reset_main_dpi_compensation")
        .map(|offset| ensure_start + offset)
        .unwrap();
    let ensure_block = &main_window[ensure_start..ensure_end];
    let dpi_start = main_window
        .find("unsafe fn apply_main_system_dpi_compensation")
        .unwrap();
    let dpi_end = main_window[dpi_start..]
        .find("\nunsafe fn sync_main_window_dpi")
        .map(|offset| dpi_start + offset)
        .unwrap();
    let dpi_block = &main_window[dpi_start..dpi_end];

    assert!(production.contains("mod main_window;"));
    assert!(!production.contains("unsafe fn refresh_main_window_metrics"));
    assert!(!production.contains("unsafe fn ensure_main_window_size_for_monitor"));
    assert!(metrics_block.contains("set_main_window_bounds("));
    assert!(ensure_block.contains("set_main_window_bounds("));
    assert!(dpi_block.contains("set_main_window_bounds("));
    assert!(!metrics_block.contains("platform_window::set_pos("));
    assert!(!ensure_block.contains("platform_window::set_pos("));
    assert!(!dpi_block.contains("platform_window::set_pos("));
    assert!(main_window_host.contains("fn set_main_window_bounds(&mut self"));
    assert!(main_window_host.contains("platform_window::set_pos("));
}

#[test]
fn windows_main_window_metrics_repaint_and_pointer_tracking_use_main_window_host() {
    let main_entry = main_entry_source();
    let main_window = main_window_source();
    let renderer = include_str!("app/main_renderer.rs").replace("\r\n", "\n");
    let input = include_str!("app/main_input.rs").replace("\r\n", "\n");
    let main_window_implementation = format!("{main_entry}\n{main_window}\n{renderer}\n{input}");
    let main_window_host = main_window_host_source();

    for forbidden in [
        "platform_gdi::invalidate_rect(hwnd",
        "platform_dpi::layout_dpi_for_window(hwnd)",
        "platform_window::client_rect(hwnd)",
        "platform_window::window_rect(hwnd)",
        "ensure_mouse_leave_tracking(hwnd)",
    ] {
        assert!(
            !main_window_implementation.contains(forbidden),
            "main window implementation should use NativeMainWindowHost instead of {forbidden}"
        );
    }

    for helper in [
        "repaint_main_window(hwnd",
        "main_window_layout_dpi(hwnd)",
        "main_window_client_bounds(hwnd)",
        "main_window_bounds(hwnd)",
        "track_main_pointer_leave(hwnd)",
    ] {
        assert!(
            main_window_implementation.contains(helper),
            "main window implementation should consume {helper}"
        );
    }

    for operation in [
        "fn track_main_pointer_leave(&mut self",
        "fn request_main_window_area_repaint(",
        "fn main_window_layout_dpi(&mut self",
        "fn main_window_client_bounds(&mut self",
        "fn main_window_bounds(&mut self",
    ] {
        assert!(
            main_window_host.contains(operation),
            "WindowsMainWindowHost should implement {operation}"
        );
    }
}

#[test]
fn windows_main_renderer_executes_shared_render_plan_outside_app_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let main_entry = main_entry_source();
    let renderer = include_str!("app/main_renderer.rs").replace("\r\n", "\n");

    assert!(app.contains("mod main_renderer;"));
    assert!(main_entry.contains("paint_main_window(hwnd);"));
    assert!(!app.contains("unsafe fn paint("));
    assert!(!app.contains("fn main_paint_fill_color("));
    assert!(!app.contains("unsafe fn draw_main_text_command("));
    assert!(!app.contains("unsafe fn draw_rgba_image_fit("));

    assert!(renderer.contains("pub(super) unsafe fn paint_main_window"));
    assert!(renderer.contains("layout.render_plan(MainRenderInput"));
    assert!(renderer.contains("draw_main_paint_command"));
    assert!(renderer.contains("draw_main_text_commands"));
    assert!(renderer.contains("draw_main_icon_command"));
    assert!(renderer.contains("platform_gdi::begin_paint"));
    assert!(renderer.contains("platform_gdi::end_paint"));
    assert!(renderer.contains("platform_gdi::copy_bits"));
}

#[test]
fn windows_main_input_executes_shared_input_plans_outside_app_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let main_events = main_events_source();
    let input = include_str!("app/main_input.rs").replace("\r\n", "\n");

    assert!(app.contains("mod main_input;"));
    assert!(main_events
        .contains("UiEvent::PointerMove { position } => handle_mouse_move(hwnd, position)"));
    assert!(!app.contains("unsafe fn handle_mouse_move"));
    assert!(!app.contains("unsafe fn handle_lbutton_down"));
    assert!(!app.contains("unsafe fn handle_lbutton_up"));
    assert!(!app.contains("unsafe fn handle_rbutton_up"));
    assert!(!app.contains("unsafe fn handle_keydown"));

    assert!(input.contains("pub(super) unsafe fn handle_mouse_move"));
    assert!(input.contains("layout.pointer_move_transition"));
    assert!(input.contains("layout.pointer_down_target"));
    assert!(input.contains("pointer_up_transition"));
    assert!(input.contains("state.list.shortcut_row_command_plan"));
    assert!(input.contains("main_shortcut_execution_plan(action"));
    assert!(input.contains("main_title_button_window_command_for_key(key)"));
    assert!(input.contains("native_host_main_tool_button_specs()"));
    assert!(!input.contains("native_host_main_tool_component_specs()"));
    assert!(!input.contains("NativeComponentAction::MainTool(tool)"));
    assert!(input.contains("NativeHostMainToolAction::GroupFilter"));
    assert!(input.contains("NativeHostMainToolAction::RowMenu"));
    assert!(input.contains("begin_main_window_drag(hwnd)"));
    assert!(input.contains("capture_main_pointer(hwnd)"));
    assert!(!input.contains("platform_window::send_message("));
    assert!(!input.contains("platform_input::set_capture("));
    assert!(!input.contains("platform_input::release_capture("));
}

#[test]
fn windows_settings_input_executes_shared_input_plans_outside_app_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let settings_input = settings_input_source();
    let settings_window = settings_window_source();

    assert!(app.contains("mod settings_input;"));
    assert!(
        settings_window.contains("dispatch_settings_ui_event(hwnd, msg, wparam, lparam, event)")
    );
    assert!(!app.contains("unsafe fn handle_settings_pointer_move"));
    assert!(!app.contains("unsafe fn handle_settings_lbutton_down"));
    assert!(!app.contains("unsafe fn handle_settings_mouse_wheel"));
    assert!(!app.contains("unsafe fn dispatch_settings_ui_event"));

    assert!(settings_input.contains("pub(super) unsafe fn dispatch_settings_ui_event"));
    assert!(settings_input.contains("settings_pointer_move_transition"));
    assert!(settings_input.contains("settings_pointer_down_target"));
    assert!(settings_input.contains("settings_scroll_delta_for_wheel(delta)"));
    assert!(settings_input.contains("settings_nav_hover_transition"));
    assert!(settings_input.contains("dispatch_settings_action(&mut executor"));
    assert!(settings_input.contains("settings_window_track_pointer_leave(hwnd)"));
    assert!(settings_input.contains("capture_settings_pointer(hwnd)"));
    assert!(!settings_input.contains("platform_input::set_capture("));
    assert!(!settings_input.contains("platform_input::release_capture("));
    assert!(!settings_input.contains("platform_window::client_rect(hwnd)"));
}

#[test]
fn windows_settings_input_domains_live_in_dedicated_modules() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let dispatcher = settings_input_dispatch_source();
    let pointer_input = settings_pointer_input_source();
    let keyboard_input = settings_keyboard_input_source();
    let window_events = settings_window_events_source();
    let window_destroy = settings_window_destroy_source();

    for module in [
        "settings_pointer_input",
        "settings_keyboard_input",
        "settings_window_events",
        "settings_window_destroy",
    ] {
        assert!(app.contains(&format!("mod {module};")));
        assert!(prelude.contains(&format!("use super::{module}::*;")));
    }

    assert!(dispatcher.contains("pub(super) unsafe fn dispatch_settings_ui_event"));
    for handler in [
        "pub(super) unsafe fn handle_settings_pointer_move",
        "pub(super) unsafe fn handle_settings_lbutton_down",
        "pub(super) unsafe fn handle_settings_mouse_wheel",
        "pub(super) unsafe fn handle_settings_key_down",
        "pub(super) unsafe fn handle_settings_theme_changed",
        "pub(super) unsafe fn handle_settings_dpi_changed",
        "pub(super) unsafe fn handle_settings_destroy",
    ] {
        assert!(!dispatcher.contains(handler));
    }

    assert!(pointer_input.contains("pub(super) unsafe fn handle_settings_pointer_move"));
    assert!(pointer_input.contains("pub(super) unsafe fn handle_settings_lbutton_down"));
    assert!(pointer_input.contains("settings_pointer_move_transition"));
    assert!(pointer_input.contains("settings_pointer_down_target"));
    assert!(pointer_input.contains("settings_scroll_delta_for_wheel(delta)"));
    assert!(keyboard_input.contains("pub(super) unsafe fn handle_settings_key_down"));
    assert!(keyboard_input.contains("hotkey::key_label_from_vk(code)"));
    assert!(window_events.contains("pub(super) unsafe fn handle_settings_theme_changed"));
    assert!(window_events.contains("pub(super) unsafe fn handle_settings_dpi_changed"));
    assert!(window_events.contains("apply_dpi_suggested_rect(hwnd, lparam)"));
    assert!(!window_events.contains("pub(super) unsafe fn handle_settings_destroy"));
    assert!(window_destroy.contains("pub(super) unsafe fn handle_settings_destroy"));
    assert!(window_destroy.contains("destroy_settings_dropdown_popup"));
    assert!(window_destroy.contains("refresh_low_level_input_hooks()"));
}

#[test]
fn windows_settings_window_adapter_lives_outside_app_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let settings_window = settings_window_source();

    assert!(app.contains("mod settings_window;"));
    assert!(prelude.contains("use super::settings_window::*;"));
    for forbidden in [
        "unsafe extern \"system\" fn settings_wnd_proc",
        "unsafe fn open_settings_window",
        "unsafe fn refresh_settings_window_metrics",
        "unsafe fn resize_settings_window_for_dpi_transition",
        "unsafe fn ensure_settings_window_in_work_area",
        "fn set_settings_window_bounds",
        "fn destroy_settings_window",
        "fn focus_settings_window",
        "fn capture_settings_pointer",
        "fn repaint_settings_window_area",
        "unsafe fn refresh_settings_cloud_page_after_lan_sync",
    ] {
        assert!(
            !app.contains(forbidden),
            "settings window adapter code should not live in app.rs: {forbidden}"
        );
    }

    for required in [
        "pub(super) unsafe extern \"system\" fn settings_wnd_proc",
        "pub(super) unsafe fn open_settings_window",
        "pub(super) unsafe fn refresh_settings_window_metrics",
        "pub(super) unsafe fn resize_settings_window_for_dpi_transition",
        "pub(super) unsafe fn ensure_settings_window_in_work_area",
        "pub(super) fn set_settings_window_bounds",
        "pub(super) fn destroy_settings_window",
        "pub(super) fn focus_settings_window",
        "pub(super) fn capture_settings_pointer",
        "pub(super) fn repaint_settings_window_area",
        "pub(super) unsafe fn refresh_settings_cloud_page_after_lan_sync",
        "WindowsSettingsWindowHost::new(Some(settings_wnd_proc))",
        "settings_window_layout_dpi(hwnd)",
        "settings_window_client_bounds(hwnd)",
    ] {
        assert!(
            settings_window.contains(required),
            "settings window sources should contain {required}"
        );
    }
}

#[test]
fn windows_settings_window_layout_and_paint_live_in_dedicated_modules() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let proc_source = settings_window_proc_source();
    let metrics = settings_window_metrics_source();
    let layout = settings_window_layout_source();
    let create = settings_window_create_source();
    let colors = settings_window_colors_source();
    let surface_controls = settings_window_surface_controls_source();
    let owner_draw = settings_window_owner_draw_source();
    let paint = settings_window_paint_source();
    let lifecycle = settings_window_lifecycle_source();

    assert!(app.contains("mod settings_window_create;"));
    assert!(app.contains("mod settings_window_colors;"));
    assert!(app.contains("mod settings_window_surface_controls;"));
    assert!(app.contains("mod settings_window_layout;"));
    assert!(app.contains("mod settings_window_lifecycle;"));
    assert!(app.contains("mod settings_window_metrics;"));
    assert!(app.contains("mod settings_window_owner_draw;"));
    assert!(app.contains("mod settings_window_paint;"));
    assert!(prelude.contains("use super::settings_window_create::*;"));
    assert!(prelude.contains("use super::settings_window_colors::*;"));
    assert!(prelude.contains("use super::settings_window_surface_controls::*;"));
    assert!(prelude.contains("use super::settings_window_layout::*;"));
    assert!(prelude.contains("use super::settings_window_lifecycle::*;"));
    assert!(prelude.contains("use super::settings_window_metrics::*;"));
    assert!(prelude.contains("use super::settings_window_owner_draw::*;"));
    assert!(prelude.contains("use super::settings_window_paint::*;"));

    for forbidden in [
        "pub(super) unsafe fn refresh_settings_window_metrics",
        "pub(super) unsafe fn resize_settings_window_for_dpi_transition",
        "pub(super) unsafe fn ensure_settings_window_in_work_area",
        "pub(super) unsafe fn apply_settings_system_dpi_compensation",
        "unsafe fn paint_settings_window",
        "unsafe fn draw_settings_window_item",
        "Box::new(SettingsWndState",
        "settings_create_btn(hwnd",
        "settings_refresh_theme_resources(&mut st)",
        "unsafe fn settings_control_color",
        "fn is_settings_surface_control",
        "unsafe fn open_settings_window",
        "fn set_settings_window_bounds",
        "fn destroy_settings_window",
        "fn focus_settings_window",
        "fn capture_settings_pointer",
        "fn repaint_settings_window_area",
        "unsafe fn refresh_settings_cloud_page_after_lan_sync",
    ] {
        assert!(
            !proc_source.contains(forbidden),
            "settings_window.rs should not own layout/paint helper: {forbidden}"
        );
    }

    for required in [
        "pub(super) fn settings_page_content_total_h_for_state",
        "pub(super) fn settings_page_max_scroll_for_state",
        "pub(super) fn settings_scroll_layout_for_state",
        "pub(super) unsafe fn refresh_settings_window_metrics",
        "settings_host_set_bounds(",
        "settings_repos_controls(hwnd, st, true)",
    ] {
        assert!(
            metrics.contains(required),
            "settings_window_metrics.rs should contain {required}"
        );
    }

    for required in [
        "pub(super) unsafe fn resize_settings_window_for_dpi_transition",
        "pub(super) unsafe fn ensure_settings_window_in_work_area",
        "pub(super) unsafe fn apply_settings_system_dpi_compensation",
        "pub(super) fn window_rect_or_empty",
    ] {
        assert!(
            layout.contains(required),
            "settings_window_layout.rs should contain {required}"
        );
    }

    for forbidden in [
        "refresh_settings_window_metrics",
        "settings_page_content_total_h_for_state",
        "settings_scroll_layout_for_state",
        "settings_host_set_bounds(",
        "settings_repos_controls(hwnd, st, true)",
    ] {
        assert!(
            !layout.contains(forbidden),
            "settings_window_layout.rs should not contain metrics helper: {forbidden}"
        );
    }

    for required in [
        "pub(super) unsafe fn create_settings_window_state",
        "SettingsWndState::new(",
        "settings_refresh_theme_resources(&mut st)",
        "settings_create_btn(hwnd",
        "settings_apply_from_app(&mut st)",
        "settings_show_page(hwnd, &mut st, 0)",
    ] {
        assert!(
            create.contains(required),
            "settings_window_create.rs should contain {required}"
        );
    }

    for required in [
        "pub(super) enum SettingsControlColorRole",
        "pub(super) unsafe fn settings_control_color",
    ] {
        assert!(
            colors.contains(required),
            "settings_window_colors.rs should contain {required}"
        );
    }
    assert!(!colors.contains("pub(super) fn is_settings_surface_control"));
    assert!(!colors.contains("IDC_SET_AUTOSTART"));

    for required in [
        "pub(super) fn is_settings_surface_control",
        "fn is_general_surface_control",
        "fn is_hotkey_surface_control",
        "fn is_group_surface_control",
        "fn is_cloud_surface_control",
        "fn is_plugin_surface_control",
        "fn is_about_surface_control",
        "IDC_SET_AUTOSTART",
        "IDC_SET_LAN_QR_ANDROID",
        "IDC_SET_OPEN_SOURCE",
    ] {
        assert!(
            surface_controls.contains(required),
            "settings_window_surface_controls.rs should contain {required}"
        );
    }

    for required in [
        "pub(super) unsafe fn draw_settings_window_item",
        "settings_draw_button_item(st, &dis2)",
        "is_settings_surface_control(dis.CtlID as isize)",
    ] {
        assert!(
            owner_draw.contains(required),
            "settings_window_owner_draw.rs should contain {required}"
        );
    }

    for required in [
        "pub(super) unsafe fn paint_settings_window",
        "settings_window_layout_dpi(hwnd)",
        "draw_settings_chrome(",
        "draw_settings_content(",
        "draw_settings_scrollbar(",
    ] {
        assert!(
            paint.contains(required),
            "settings_window_paint.rs should contain {required}"
        );
    }

    for forbidden in [
        "SettingsControlColorRole",
        "settings_control_color",
        "draw_settings_window_item",
        "is_settings_surface_control",
    ] {
        assert!(
            !paint.contains(forbidden),
            "settings_window_paint.rs should not contain {forbidden}"
        );
    }

    for required in [
        "pub(super) unsafe fn open_settings_window",
        "pub(super) fn set_settings_window_bounds",
        "pub(super) fn destroy_settings_window",
        "pub(super) fn focus_settings_window",
        "pub(super) fn capture_settings_pointer",
        "pub(super) fn release_settings_pointer",
        "pub(super) fn repaint_settings_window_area",
        "pub(super) unsafe fn refresh_settings_cloud_page_after_lan_sync",
        "WindowsSettingsWindowHost::new(Some(settings_wnd_proc))",
    ] {
        assert!(
            lifecycle.contains(required),
            "settings_window_lifecycle.rs should contain {required}"
        );
    }
}

#[test]
fn windows_settings_dropdown_executor_lives_outside_app_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let settings_commands = settings_commands_source();
    let settings_dropdown = settings_dropdown_source();
    let dropdown_general = settings_dropdown_general_source();
    let dropdown_cloud = settings_dropdown_cloud_source();
    let dropdown_hotkey = settings_dropdown_hotkey_source();
    let dropdown_group = settings_dropdown_group_source();
    let dropdown_host = settings_dropdown_host_source();
    let dropdown_plugin = settings_dropdown_plugin_source();

    assert!(app.contains("mod settings_dropdown;"));
    assert!(app.contains("mod settings_dropdown_general;"));
    assert!(app.contains("mod settings_dropdown_cloud;"));
    assert!(app.contains("mod settings_dropdown_hotkey;"));
    assert!(app.contains("mod settings_dropdown_group;"));
    assert!(app.contains("mod settings_dropdown_host;"));
    assert!(app.contains("mod settings_dropdown_plugin;"));
    assert!(prelude.contains("use super::settings_dropdown::*;"));
    assert!(prelude.contains("use super::settings_dropdown_general::*;"));
    assert!(prelude.contains("use super::settings_dropdown_cloud::*;"));
    assert!(prelude.contains("use super::settings_dropdown_hotkey::*;"));
    assert!(prelude.contains("use super::settings_dropdown_group::*;"));
    assert!(prelude.contains("use super::settings_dropdown_host::*;"));
    assert!(prelude.contains("use super::settings_dropdown_plugin::*;"));
    assert!(settings_commands.contains("open_settings_dropdown_for_control(hwnd, st"));
    assert!(!app.contains("unsafe fn open_settings_dropdown_for_control"));
    assert!(!app.contains("fn present_settings_dropdown_popup"));
    assert!(!app.contains("fn settings_dropdown_popup_exists"));

    assert!(settings_dropdown.contains("pub(super) unsafe fn open_settings_dropdown_for_control"));
    assert!(settings_dropdown.contains("settings_open_plugin_dropdown_for_control(hwnd, st"));
    assert!(settings_dropdown.contains("open_settings_general_dropdown(hwnd, st, control_id)"));
    assert!(settings_dropdown.contains("open_settings_cloud_dropdown(hwnd, st, control_id)"));
    assert!(settings_dropdown.contains("open_settings_hotkey_dropdown(hwnd, st, control_id)"));
    assert!(settings_dropdown.contains("open_settings_group_dropdown(hwnd, st, control_id)"));
    assert!(dropdown_host.contains("NativeSettingsDropdownRequest"));
    assert!(dropdown_host.contains("host.present_settings_dropdown(request)"));
    for forbidden in [
        "settings_dropdown_max_items_labels()",
        "settings_dropdown_index_for_pos_mode",
        "settings_groups_cache_for_tab",
        "HOTKEY_MOD_OPTIONS",
        "MULTI_SYNC_MODE_OPTIONS",
        "PASTE_SOUND_OPTIONS",
    ] {
        assert!(
            !settings_dropdown.contains(forbidden),
            "settings_dropdown.rs should only dispatch dropdown domains: {forbidden}"
        );
    }
    assert!(dropdown_general.contains("pub(super) unsafe fn open_settings_general_dropdown"));
    assert!(dropdown_general.contains("settings_dropdown_max_items_labels()"));
    assert!(dropdown_general.contains("settings_dropdown_index_for_pos_mode"));
    assert!(dropdown_general.contains("PASTE_SOUND_OPTIONS"));
    assert!(dropdown_cloud.contains("pub(super) unsafe fn open_settings_cloud_dropdown"));
    assert!(dropdown_cloud.contains("MULTI_SYNC_MODE_OPTIONS"));
    assert!(dropdown_cloud.contains("IDC_SET_LAN_RECEIVE_MODE"));
    assert!(dropdown_hotkey.contains("pub(super) unsafe fn open_settings_hotkey_dropdown"));
    assert!(dropdown_hotkey.contains("HOTKEY_MOD_OPTIONS"));
    assert!(dropdown_hotkey.contains("HOTKEY_KEY_OPTIONS"));
    assert!(dropdown_group.contains("pub(super) unsafe fn open_settings_group_dropdown"));
    assert!(dropdown_group.contains("settings_groups_cache_for_tab"));
    assert!(dropdown_group.contains("source_tab_all_label"));
    assert!(!settings_dropdown.contains("IMAGE_OCR_PROVIDER_OPTIONS"));
    assert!(!settings_dropdown.contains("TEXT_TRANSLATE_PROVIDER_OPTIONS"));
    assert!(dropdown_plugin.contains("pub(super) fn settings_open_plugin_dropdown_for_control"));
    assert!(dropdown_plugin.contains("SEARCH_ENGINE_PRESETS"));
    assert!(dropdown_plugin.contains("IMAGE_OCR_PROVIDER_OPTIONS"));
    assert!(dropdown_plugin.contains("TEXT_TRANSLATE_PROVIDER_OPTIONS"));
    assert!(!settings_dropdown.contains("platform_window::create_window_ex"));
    assert!(!settings_dropdown.contains("show_settings_dropdown_popup("));
}

#[test]
fn windows_settings_command_executor_lives_outside_app_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let command_queue = settings_command_queue_source();
    let timer_tasks = settings_timer_tasks_source();
    let control_selection = settings_control_selection_source();
    let selection_general = settings_control_selection_general_source();
    let selection_cloud = settings_control_selection_cloud_source();
    let selection_hotkey = settings_control_selection_hotkey_source();
    let selection_plugin = settings_control_selection_plugin_source();
    let selection_group = settings_control_selection_group_source();
    let settings_input = settings_input_source();

    assert!(!app.contains("mod settings_commands;"));
    assert!(app.contains("mod settings_command_queue;"));
    assert!(app.contains("mod settings_timer_tasks;"));
    assert!(app.contains("mod settings_control_selection;"));
    assert!(app.contains("mod settings_control_selection_general;"));
    assert!(app.contains("mod settings_control_selection_cloud;"));
    assert!(app.contains("mod settings_control_selection_hotkey;"));
    assert!(app.contains("mod settings_control_selection_plugin;"));
    assert!(app.contains("mod settings_control_selection_group;"));
    assert!(prelude.contains("use super::settings_command_queue::*;"));
    assert!(prelude.contains("use super::settings_timer_tasks::*;"));
    assert!(prelude.contains("use super::settings_control_selection::*;"));
    assert!(prelude.contains("use super::settings_control_selection_general::*;"));
    assert!(prelude.contains("use super::settings_control_selection_cloud::*;"));
    assert!(prelude.contains("use super::settings_control_selection_hotkey::*;"));
    assert!(prelude.contains("use super::settings_control_selection_plugin::*;"));
    assert!(prelude.contains("use super::settings_control_selection_group::*;"));
    assert!(!app.contains("unsafe fn execute_settings_ui_command"));
    assert!(!app.contains("unsafe fn handle_settings_timer_task"));
    assert!(!app.contains("unsafe fn handle_settings_control_selection"));
    assert!(!app.contains("unsafe fn show_settings_saved_feedback"));

    assert!(settings_input.contains("queue_settings_command(st, command)"));
    assert!(settings_input.contains("drain_settings_ui_commands(hwnd, st)"));
    assert!(settings_input.contains("handle_settings_control_selection(hwnd"));
    assert!(settings_input.contains("handle_settings_timer_task(hwnd, task)"));

    assert!(command_queue.contains("unsafe fn execute_settings_ui_command"));
    assert!(command_queue.contains("command_ids::SAVE_SETTINGS"));
    assert!(command_queue.contains("unsafe fn show_settings_saved_feedback"));
    assert!(timer_tasks.contains("pub(super) unsafe fn handle_settings_timer_task"));
    assert!(timer_tasks.contains("SettingsTimerTask::DpiFit"));
    assert!(control_selection.contains("pub(super) unsafe fn handle_settings_control_selection"));
    assert!(control_selection.contains("handle_settings_general_selection(st, control_id, index)"));
    assert!(
        control_selection.contains("handle_settings_cloud_selection(hwnd, st, control_id, index)")
    );
    assert!(control_selection.contains("handle_settings_hotkey_selection(st, control_id, index)"));
    assert!(control_selection.contains("handle_settings_plugin_selection(st, control_id, index)"));
    assert!(control_selection.contains("handle_settings_group_selection(st, control_id, index)"));
    for forbidden in [
        "settings_dropdown_max_items_labels()",
        "settings_rebuild_cloud_page(hwnd, st)",
        "HOTKEY_MOD_OPTIONS",
        "SEARCH_ENGINE_PRESETS",
        "settings_groups_cache_for_tab",
    ] {
        assert!(
            !control_selection.contains(forbidden),
            "settings_control_selection.rs should only dispatch selection domains: {forbidden}"
        );
    }
    assert!(selection_general.contains("pub(super) unsafe fn handle_settings_general_selection"));
    assert!(selection_general.contains("settings_dropdown_max_items_labels()"));
    assert!(selection_general.contains("settings_sync_pos_fields_enabled(st)"));
    assert!(selection_cloud.contains("pub(super) unsafe fn handle_settings_cloud_selection"));
    assert!(selection_cloud.contains("settings_rebuild_cloud_page(hwnd, st)"));
    assert!(selection_cloud.contains("lan_receive_mode_from_label(label)"));
    assert!(selection_hotkey.contains("pub(super) unsafe fn handle_settings_hotkey_selection"));
    assert!(selection_hotkey.contains("HOTKEY_MOD_OPTIONS"));
    assert!(selection_hotkey.contains("hotkey_preview_text"));
    assert!(selection_plugin.contains("pub(super) unsafe fn handle_settings_plugin_selection"));
    assert!(selection_plugin.contains("SEARCH_ENGINE_PRESETS"));
    assert!(selection_plugin.contains("IMAGE_OCR_PROVIDER_OPTIONS"));
    assert!(selection_plugin.contains("TEXT_TRANSLATE_PROVIDER_OPTIONS"));
    assert!(selection_group.contains("pub(super) unsafe fn handle_settings_group_selection"));
    assert!(selection_group.contains("settings_groups_cache_for_tab"));
    assert!(selection_group.contains("settings_sync_vv_group_display(st)"));
    for source in [command_queue, timer_tasks, control_selection] {
        assert!(!source.contains("settings_window_host_event_from_message"));
    }
}

#[test]
fn windows_main_row_command_executor_lives_outside_app_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let main_events = main_events_source();
    let main_input = include_str!("app/main_input.rs").replace("\r\n", "\n");
    let main_row_commands = main_row_commands_source();

    assert!(app.contains("mod main_row_commands;"));
    assert!(main_events.contains("execute_row_command(hwnd, state, row_intent)"));
    assert!(!app.contains("unsafe fn execute_row_command"));
    assert!(!app.contains("unsafe fn execute_row_external_action"));
    assert!(!app.contains("unsafe fn execute_row_dialog_action"));
    assert!(!app.contains("unsafe fn execute_row_data_plan"));
    assert!(!app.contains("fn select_context_row"));
    assert!(!app.contains("unsafe fn quick_search_open"));
    assert!(!app.contains("fn url_encode_component"));

    assert!(main_input.contains("execute_delete_selection_data_plan(hwnd, state)"));
    assert!(main_row_commands.contains("pub(super) unsafe fn execute_row_command"));
    assert!(main_row_commands.contains("pub(super) unsafe fn execute_delete_selection_data_plan"));
    assert!(main_row_commands.contains("main_row_external_action_plan("));
    assert!(main_row_commands.contains("main_row_dialog_action_plan("));
    assert!(main_row_commands.contains("main_row_data_action_plan("));
    assert!(main_row_commands.contains("main_row_group_assignment_plan("));
    assert!(main_row_commands.contains("unsafe fn quick_search_open"));
    assert!(main_row_commands.contains("fn url_encode_component"));
    assert!(main_row_commands.contains("WindowsEditTextDialogHost::new().open_edit_text"));
    assert!(main_row_commands.contains("WindowsMailMergeWindowHost::new().open_mail_merge"));
    assert!(!main_row_commands.contains("main_row_menu_plan("));
    assert!(!main_row_commands.contains("platform_menu::WindowsPopupMenuHost"));
}

#[test]
fn windows_main_row_tools_live_outside_app_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let main_input = include_str!("app/main_input.rs").replace("\r\n", "\n");
    let main_row_commands = main_row_commands_source();
    let main_paste = main_paste_source();
    let row_tools = main_row_tools_source();

    assert!(app.contains("mod main_row_tools;"));
    assert!(!app.contains("fn ai_clean_text"));
    assert!(!app.contains("unsafe fn maybe_ai_clean_text"));
    assert!(!app.contains("unsafe fn spawn_image_ocr_job"));
    assert!(!app.contains("unsafe fn spawn_text_translate_text_job"));
    assert!(!app.contains("unsafe fn begin_row_drag_export"));
    assert!(!app.contains("fn clear_hotkey_passthrough_state"));

    assert!(main_paste.contains("maybe_ai_clean_text(state"));
    assert!(main_paste.contains("clear_hotkey_passthrough_state(state)"));
    assert!(main_row_commands.contains("spawn_text_translate_text_job("));
    assert!(main_row_commands.contains("spawn_image_ocr_job("));
    assert!(main_input.contains("begin_row_drag_export(hwnd, state"));
    assert!(main_input.contains("image_input_for_ocr"));

    for required in [
        "pub(super) fn ai_clean_text",
        "pub(super) unsafe fn maybe_ai_clean_text",
        "pub(super) unsafe fn spawn_image_ocr_job",
        "pub(super) unsafe fn spawn_text_translate_text_job",
        "pub(super) unsafe fn begin_row_drag_export",
        "pub(super) fn clear_hotkey_passthrough_state",
        "pub(crate) fn image_input_for_ocr",
        "run_baidu_ocr_api(",
        "run_winocr_dll_ocr(",
        "run_baidu_translate_api(",
        "platform_dragdrop::begin_file_drag",
    ] {
        assert!(
            row_tools.contains(required),
            "main_row_tools.rs should contain {required}"
        );
    }
}

#[test]
fn windows_main_paste_executor_lives_outside_app_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let main_input = include_str!("app/main_input.rs").replace("\r\n", "\n");
    let main_row_commands = main_row_commands_source();
    let main_paste = main_paste_source();

    assert!(app.contains("mod main_paste;"));
    assert!(!app.contains("unsafe fn copy_selection_to_clipboard"));
    assert!(!app.contains("unsafe fn apply_item_to_clipboard"));
    assert!(!app.contains("unsafe fn paste_selected"));
    assert!(!app.contains("unsafe fn paste_after_clipboard_ready_to_target"));
    assert!(main_input.contains("paste_selected(hwnd, state)"));
    assert!(main_input.contains("paste_after_clipboard_ready(hwnd, state"));
    assert!(main_row_commands.contains("apply_item_to_clipboard(state"));
    assert!(main_paste.contains("pub(super) unsafe fn copy_selection_to_clipboard"));
    assert!(main_paste.contains("pub(super) unsafe fn apply_item_to_clipboard"));
    assert!(main_paste.contains("pub(super) unsafe fn paste_selected"));
    assert!(main_paste.contains("main_paste_preparation_plan("));
    assert!(main_paste.contains("main_paste_completion_plan("));
    assert!(main_paste.contains("WindowsClipboardHost::write_text"));
    assert!(main_paste.contains("WindowsPasteTargetHost::new().force_paste_target_foreground"));
    assert!(main_paste.contains("WindowsWindowIdentityHost::new().exists(target)"));
}

#[test]
fn windows_main_popup_menu_executor_lives_outside_app_rs() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let main_input = include_str!("app/main_input.rs").replace("\r\n", "\n");
    let main_popup_menus = main_popup_menus_source();

    assert!(app.contains("mod main_popup_menus;"));
    assert!(main_input.contains("show_row_menu("));
    assert!(main_input.contains("show_group_filter_menu("));
    assert!(!app.contains("unsafe fn show_row_menu"));
    assert!(!app.contains("unsafe fn show_group_filter_menu"));
    assert!(!app.contains("main_row_menu_plan(MainRowMenuInput"));

    assert!(main_popup_menus.contains("pub(super) unsafe fn show_row_menu"));
    assert!(main_popup_menus.contains("pub(super) unsafe fn show_group_filter_menu"));
    assert!(main_popup_menus.contains("native_host_row_action_button_specs("));
    assert!(!main_popup_menus.contains("native_host_row_action_component_specs("));
    assert!(!main_popup_menus.contains("NativeComponentAction::Row(action)"));
    assert!(main_popup_menus.contains("native_host_full_row_popup_menu_entries_for_groups("));
    assert!(main_popup_menus.contains("NativeHostRowPopupMenuInput"));
    assert!(!main_popup_menus.contains("main_row_menu_plan(MainRowMenuInput"));
    assert!(!main_popup_menus.contains("main_row_popup_menu_entries("));
    assert!(main_popup_menus.contains("native_host_group_filter_popup_menu_entries_for_groups("));
    assert!(main_popup_menus.contains("localize_group_filter_entry("));
    assert!(main_popup_menus.contains("WindowsPopupMenuHost::new().present_popup_menu"));
    assert!(main_popup_menus.contains("NativePopupMenuPlacement::TopLeft"));
    assert!(main_popup_menus.contains("NativePopupMenuPlacement::BottomLeft"));
    assert!(!main_popup_menus.contains("platform_menu::create_popup"));
    assert!(!main_popup_menus.contains("platform_menu::track_popup_raw"));
}

#[test]
fn windows_main_window_visibility_state_uses_main_window_host() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let main_entry = main_entry_source();
    let production = app
        .split("\n#[cfg(test)]\nmod tests")
        .next()
        .unwrap_or(&app);
    let main_window = main_window_source();
    let main_paste = main_paste_source();
    let main_window_host = main_window_host_source();
    let single_start = main_entry.find("if already_running {").unwrap();
    let single_end = main_entry[single_start..]
        .find("\n    unsafe {")
        .map(|offset| single_start + offset)
        .unwrap();
    let single_block = &main_entry[single_start..single_end];
    let deactivate_start = main_window
        .find("pub(super) unsafe fn handle_main_app_activation_changed")
        .unwrap();
    let deactivate_end = main_window[deactivate_start..]
        .find("\npub(super) unsafe fn handle_main_system_metrics_changed")
        .map(|offset| deactivate_start + offset)
        .unwrap();
    let deactivate_block = &main_window[deactivate_start..deactivate_end];
    let paste_plan_start = main_paste
        .find("unsafe fn execute_paste_completion_plan_to_target")
        .unwrap();
    let paste_plan_end = main_paste[paste_plan_start..]
        .find("\npub(super) unsafe fn paste_selected")
        .map(|offset| paste_plan_start + offset)
        .unwrap();
    let paste_plan_block = &main_paste[paste_plan_start..paste_plan_end];
    let paste_ready_start = main_paste
        .find("unsafe fn paste_after_clipboard_ready_to_target")
        .unwrap();
    let paste_ready_block = &main_paste[paste_ready_start..];

    assert!(single_block.contains("close_main_window(hwnd)"));
    assert!(single_block.contains("restore_main_window(hwnd)"));
    assert!(single_block.contains("foreground_main_window(hwnd)"));
    assert!(deactivate_block.contains("hide_main_window(hwnd)"));
    assert!(paste_plan_block.contains("hide_main_window(hwnd)"));
    assert!(paste_ready_block.contains("hide_main_window(hwnd)"));
    assert!(paste_ready_block.contains("foreground_main_window(hwnd)"));
    assert!(production.contains("mod main_window;"));
    assert!(!production.contains("unsafe fn handle_main_app_activation_changed"));
    assert!(!production.contains("unsafe fn execute_paste_completion_plan_to_target"));
    assert!(!production.contains("unsafe fn paste_selected"));
    assert!(!single_block.contains("platform_window::close(hwnd)"));
    assert!(!single_block.contains("platform_window::restore(hwnd)"));
    assert!(!single_block.contains("platform_window::set_foreground(hwnd)"));
    assert!(!deactivate_block.contains("platform_window::hide(hwnd)"));
    assert!(!paste_plan_block.contains("platform_window::hide(hwnd)"));
    assert!(!paste_ready_block.contains("platform_window::hide(hwnd)"));
    assert!(!paste_ready_block.contains("platform_window::set_foreground(hwnd)"));
    assert!(main_window_host.contains("fn foreground_main_window(&mut self"));
    assert!(main_window_host.contains("fn restore_main_window(&mut self"));
    assert!(main_window_host.contains("fn close_main_window(&mut self"));
    assert!(main_window_host.contains("platform_window::set_foreground(handle)"));
    assert!(main_window_host.contains("platform_window::restore(handle)"));
    assert!(main_window_host.contains("platform_window::close(handle)"));
}

#[test]
fn windows_main_search_control_creation_path_uses_search_control_host() {
    let main_entry = main_entry_source();
    let hosts = app_hosts_source();
    let main_search_host = main_search_host_source();
    let start = main_entry.find("unsafe fn on_create").unwrap();
    let end = main_entry[start..]
        .find("\npub(super) unsafe fn handle_control_command")
        .map(|offset| start + offset)
        .unwrap();
    let create_block = &main_entry[start..end];

    assert!(create_block.contains("WindowsMainSearchControlHost::new"));
    assert!(create_block.contains("search_control_request_from_native_spec("));
    assert!(create_block.contains("create_search_control(search_request)"));
    assert!(create_block.contains("NativeMainSearchControlPresentation::Created"));
    assert!(!create_block.contains("create_window_ex"));
    assert!(!create_block.contains("to_wide(\"EDIT\")"));
    assert!(!hosts.contains("pub(super) struct WindowsMainSearchControlHost"));
    assert!(main_search_host.contains("pub(super) struct WindowsMainSearchControlHost"));
    assert!(main_search_host
        .contains("impl NativeMainSearchControlHost for WindowsMainSearchControlHost"));
    assert!(main_search_host.contains("native_host_search_input_specs"));
    assert!(!main_search_host.contains("native_host_search_component_specs"));
    assert!(!main_search_host.contains("NativeComponentAction::SearchControl"));
    assert!(main_search_host.contains("to_wide(\"EDIT\")"));
}

#[test]
fn windows_main_search_control_operations_use_search_control_host() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let main_entry = main_entry_source();
    let production = app
        .split("\n#[cfg(test)]\nmod tests")
        .next()
        .unwrap_or(&app);
    let main_search = main_search_source();
    let main_search_host = main_search_host_source();

    assert!(production.contains("mod main_search;"));
    assert!(!production.contains("pub(crate) unsafe fn layout_children"));
    assert!(!production.contains("unsafe fn search_visibility_plan_for_request"));
    assert!(!production.contains("unsafe fn activate_window_for_search_input"));
    assert!(main_search.contains("pub(crate) unsafe fn layout_children"));
    assert!(main_search.contains("set_search_bounds(state.search_hwnd"));
    assert!(main_search.contains("set_search_visible(state.search_hwnd"));
    assert!(main_search.contains("set_search_text(state.search_hwnd"));
    assert!(main_search.contains("focus_search(state.search_hwnd"));
    assert!(main_search.contains("search_text(state.search_hwnd"));
    assert!(main_search.contains("apply_search_style(request)"));
    assert!(main_search.contains("handle_search_control_command("));
    assert!(main_search.contains("main_search_visibility_plan(MainSearchVisibilityInput"));
    assert!(main_entry.contains("release_search_style_resource((*ptr).search_font)"));
    assert!(main_search_host.contains("fn apply_search_style("));
    assert!(main_search_host.contains("fn release_search_style_resource("));
    assert!(main_search_host.contains("fn set_search_bounds(&mut self"));
    assert!(main_search_host.contains("fn set_search_visible(&mut self"));
    assert!(main_search_host.contains("fn search_text(&self"));
    assert!(main_search_host.contains("fn set_search_text(&mut self"));
    assert!(main_search_host.contains("fn focus_search(&mut self"));
    assert!(main_search_host.contains("WM_SETFONT"));
    assert!(main_search_host.contains("platform_gdi::create_font_w"));
    assert!(!production.contains("platform_window::move_window(\n        state.search_hwnd"));
    assert!(!production.contains("platform_window::set_visible(state.search_hwnd"));
    assert!(!production.contains("platform_window::set_text(state.search_hwnd"));
    assert!(!production.contains("platform_window::text(state.search_hwnd"));
    assert!(!production.contains("platform_input::set_focus(state.search_hwnd"));
    assert!(!production.contains("WM_SETFONT"));
    assert!(!production.contains(
        "platform_gdi::create_font_w(\n        -platform_dpi::scale_for_window(state.hwnd, 14)"
    ));
    assert!(!production.contains("platform_gdi::delete_object((*ptr).search_font"));
}

#[test]
fn windows_mail_merge_window_host_owns_mail_merge_actions() {
    let app = include_str!("app.rs").replace("\r\n", "\n");
    let prelude = app_prelude_source();
    let main_row_commands = main_row_commands_source();
    let settings_actions = settings_actions_source();
    let mail_merge = include_str!("mail_merge_native.rs").replace("\r\n", "\n");
    let app_production = app
        .split("\n#[cfg(test)]\nmod tests")
        .next()
        .unwrap_or(&app);

    let row_start = main_row_commands
        .find("unsafe fn execute_row_dialog_action")
        .unwrap();
    let row_end = main_row_commands[row_start..]
        .find("\nunsafe fn execute_row_current_item_action")
        .map(|offset| row_start + offset)
        .unwrap();
    let row_block = &main_row_commands[row_start..row_end];
    let settings_start = settings_actions
        .find("unsafe fn execute_settings_platform_action")
        .unwrap();
    let settings_block = &settings_actions[settings_start..];

    assert!(prelude.contains("use crate::mail_merge_native::WindowsMailMergeWindowHost;"));
    assert!(!app_production.contains("struct WindowsMailMergeWindowHost"));
    assert!(!app_production.contains("launch_mail_merge_window_with_excel"));
    assert!(mail_merge.contains("pub(crate) struct WindowsMailMergeWindowHost"));
    assert!(mail_merge.contains("impl NativeMailMergeWindowHost for WindowsMailMergeWindowHost"));
    assert!(mail_merge.contains("unsafe fn launch_mail_merge_window_with_excel"));
    assert!(!mail_merge.contains("pub(crate) unsafe fn launch_mail_merge_window_with_excel"));
    assert!(row_block.contains("WindowsMailMergeWindowHost::new().open_mail_merge"));
    assert!(settings_block.contains("WindowsMailMergeWindowHost::new().open_mail_merge"));
    assert!(row_block.contains("NativeMailMergeWindowRequest"));
    assert!(!row_block.contains("launch_mail_merge_window"));
    assert!(!settings_block.contains("launch_mail_merge_window"));
}

#[test]
fn settings_timer_ids_map_to_settings_tasks() {
    assert_eq!(
        settings_timer_task_for_id(ID_TIMER_SETTINGS_SCROLLBAR, SETTINGS_TIMER_IDS),
        Some(SettingsTimerTask::HideScrollbar)
    );
    assert_eq!(
        settings_timer_task_for_id(ID_TIMER_SETTINGS_SAVE_HINT, SETTINGS_TIMER_IDS),
        Some(SettingsTimerTask::ClearSaveHint)
    );
    assert_eq!(
        settings_timer_task_for_id(ID_TIMER_SETTINGS_DPI_FIT, SETTINGS_TIMER_IDS),
        Some(SettingsTimerTask::DpiFit)
    );

    assert_eq!(
        settings_timer_task_for_id(ID_TIMER_STARTUP_RECOVERY, SETTINGS_TIMER_IDS),
        None
    );
    assert_eq!(
        settings_timer_task_for_id(ID_TIMER_PASTE, SETTINGS_TIMER_IDS),
        None
    );
    assert_eq!(
        settings_timer_task_for_id(ID_TIMER_CLOUD_SYNC, SETTINGS_TIMER_IDS),
        None
    );
    assert_eq!(
        settings_timer_task_for_id(usize::MAX, SETTINGS_TIMER_IDS),
        None
    );
}

#[test]
fn zsclip_product_adapter_routes_clipboard_commands_events_and_ai() {
    let mut snapshot = ZsclipProductSnapshot::default();
    snapshot.visible_items = 12;
    snapshot.selected_items = 2;
    snapshot.search_text = "receipt".to_string();
    snapshot.settings.revision = 9;
    snapshot.settings.ai_clean_enabled = true;
    let mut adapter = ZsclipProductAdapter::new(snapshot);

    assert_eq!(adapter.product_identity().product_id, "zsclip");
    let projected = adapter.project_product_state();
    assert_eq!(projected.state_name, "search_results");
    assert_eq!(projected.revision, 9);

    let row_copy = main_menu_command_for_id(IDM_ROW_COPY).unwrap();
    let copy_result = adapter.execute_product_command(row_copy);
    assert!(copy_result.accepted);
    assert_eq!(copy_result.result_name, "zsclip.row.copy");

    let tray_lan = main_menu_command_for_id(IDM_TRAY_LAN_TOGGLE).unwrap();
    let tray_result = adapter.execute_product_command(tray_lan);
    assert!(tray_result.accepted);
    assert_eq!(tray_result.result_name, "zsclip.tray.toggle_lan_sync");
    assert_eq!(adapter.command_records().len(), 2);

    adapter.bind_settings_model(ProductAdapterSettingsSnapshot {
        profile_name: "default".to_string(),
        revision: 13,
    });
    assert_eq!(adapter.project_product_state().revision, 15);

    let bridged = adapter.bridge_async_event(ApplicationEvent::CloudSyncReady);
    assert!(bridged.bridged);
    assert_eq!(bridged.event_name, "cloud_sync_ready");
    assert_eq!(adapter.bridged_event_names(), &["cloud_sync_ready"]);

    let plan = product_ai_execution_plan(ProductAiInvocation {
        capability_id: "clipboard.clean".to_string(),
        input_text: " clean this ".to_string(),
        context_item_ids: vec![7, 8],
    })
    .unwrap();
    let ai_result = adapter.execute_ai_plan(plan);
    assert!(ai_result.accepted);
    assert_eq!(ai_result.result_name, "clipboard_text");
    assert_eq!(adapter.ai_action_names(), &["clean_text"]);
    assert!(adapter
        .publish_ai_catalog()
        .iter()
        .any(|capability| capability.id == "clipboard.skill.translate"));

    let manifest = zsclip_product_adapter_manifest();
    assert_eq!(manifest.product_id, "zsclip");
    assert!(manifest.command_routes.iter().any(|route| {
        route.family_name == "row"
            && route.result_name == "zsclip.row.image_ocr"
            && route.execution_owner == "product_adapter"
            && route.requires_selection
            && route.ai_capability_id == Some("clipboard.product.ocr")
    }));
    assert!(manifest.command_routes.iter().any(|route| {
        route.family_name == "tray"
            && route.result_name == "zsclip.tray.toggle_lan_sync"
            && route.execution_owner == "product_adapter"
            && !route.requires_selection
    }));
    assert!(manifest.event_routes.iter().any(|route| {
        route.event_name == "cloud_sync_ready"
            && route.product_effect_name == "refresh_cloud_sync_state"
    }));
    assert_eq!(
        manifest.ai_provider_names,
        vec!["llms", "skills", "product_adapter"]
    );
    assert!(manifest.ai_capability_ids.contains(&"clipboard.clean"));
}

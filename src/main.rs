#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "windows")]
mod app;
mod app_core;
mod cloud_sync;
mod db_runtime;
#[cfg(target_os = "windows")]
mod hover_preview;
#[path = "i18n_runtime.rs"]
mod i18n;
#[cfg(target_os = "windows")]
mod lan_sync;
mod lan_sync_core;
#[cfg(any(target_os = "linux", test))]
mod linux_app;
#[cfg(any(target_os = "linux", test))]
mod linux_gtk_adapter;
#[cfg(any(target_os = "linux", test))]
mod linux_native_host;
#[cfg(any(target_os = "macos", test))]
mod macos_app;
#[cfg(any(target_os = "macos", test))]
mod macos_appkit_adapter;
#[cfg(any(target_os = "macos", test))]
mod macos_native_host;
#[cfg(all(target_os = "windows", feature = "mail-merge"))]
mod mail_merge_native;
mod multi_sync;
#[cfg(target_os = "windows")]
mod platform;
mod settings_model;
#[cfg(target_os = "windows")]
mod settings_render;
#[cfg(target_os = "windows")]
mod settings_ui_host;
#[cfg(target_os = "windows")]
mod shell;
#[cfg(all(target_os = "windows", feature = "sticker"))]
mod sticker;
mod time_utils;
#[cfg(target_os = "windows")]
mod tray;
#[cfg(target_os = "windows")]
mod ui;
#[cfg(target_os = "windows")]
mod win_native_style;
#[cfg(target_os = "windows")]
mod win_system_params;
#[cfg(target_os = "windows")]
mod win_system_ui;
#[cfg(target_os = "windows")]
mod win_ui_rect;
#[cfg(target_os = "windows")]
mod win_ui_render;
#[cfg(target_os = "windows")]
mod windows_edit_text_dialog;
#[cfg(target_os = "windows")]
mod windows_text_input_dialog;
#[cfg(any(target_os = "windows", test))]
mod windows_win32_adapter;
mod zsclip_product_adapter;

#[cfg(target_os = "windows")]
fn main() {
    let _adapter_boundary =
        windows_win32_adapter::WindowsWin32AdapterBoundary::default_from_core_contract();
    if let Some(code) = shell::maybe_run_wechat_ocr_helper_from_args() {
        std::process::exit(code);
    }
    if let Err(err) = app::run() {
        eprintln!("error: {err}");
    }
}

#[cfg(target_os = "macos")]
fn main() {
    if let Err(err) = macos_app::run() {
        eprintln!("error: {err}");
    }
}

#[cfg(target_os = "linux")]
fn main() {
    if let Err(err) = linux_app::run() {
        eprintln!("error: {err}");
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn main() {
    eprintln!("ZSClip currently provides Windows, macOS and Linux host entry points.");
}

#[cfg(test)]
mod source_encoding_tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    #[test]
    fn bdd_rust_sources_and_docs_do_not_contain_common_mojibake_markers() {
        let root = crate_root();
        let mut files = Vec::new();
        collect_files(&root.join("src"), &mut files);
        collect_files(&root.join("docs"), &mut files);
        files.push(root.join("README.md"));
        files.push(root.join("README.en.md"));

        let markers = mojibake_markers();
        let mut hits = Vec::new();
        for file in files.into_iter().filter(|file| file.exists()) {
            let Ok(text) = fs::read_to_string(&file) else {
                continue;
            };
            for (line_no, line) in text.lines().enumerate() {
                if markers.iter().any(|marker| line.contains(marker.as_str())) {
                    hits.push(format!(
                        "{}:{}:{}",
                        file.strip_prefix(&root).unwrap_or(&file).display(),
                        line_no + 1,
                        line
                    ));
                }
            }
        }

        assert!(hits.is_empty(), "{}", hits.join("\n"));
    }

    #[test]
    fn zsclip_native_ui_skill_points_to_host_contract_sources() {
        let root = crate_root();
        let skill = fs::read_to_string(root.join("docs/skills/zsclip-native-ui/SKILL.md"))
            .expect("zsclip native UI skill should exist");
        let reference = fs::read_to_string(
            root.join("docs/skills/zsclip-native-ui/references/native-ui-entrypoints.md"),
        )
        .expect("zsclip native UI entrypoint reference should exist");
        let porting_doc = fs::read_to_string(root.join("docs/ui-host-porting.md"))
            .expect("UI host porting doc should exist");

        assert!(skill.contains("src/app_core/"));
        assert!(skill.contains("src/macos_native_host.rs"));
        assert!(skill.contains("src/linux_native_host.rs"));
        assert!(skill.contains("docs/native-host-verification.md"));
        assert!(skill.contains("code-level"));
        assert!(skill.contains("target-smoke"));
        assert!(skill.contains("system-complete"));
        assert!(reference.contains("src/app_core/native_host_actions.rs"));
        assert!(reference.contains("src/app_core/host_protocol.rs"));
        assert!(reference.contains("zsui_native_feature_parity_statuses()"));
        assert!(reference.contains("Right-click edit/save route"));
        assert!(reference.contains("Group create, rename, delete, reorder, assign and filter"));
        assert!(reference.contains("VV popup/select and VV paste bridge"));
        assert!(porting_doc.contains("docs/skills/zsclip-native-ui/SKILL.md"));
    }

    fn crate_root() -> PathBuf {
        let mut dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        loop {
            if dir.join("Cargo.toml").is_file() {
                return dir;
            }
            if !dir.pop() {
                return PathBuf::from(".");
            }
        }
    }

    fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_files(&path, files);
            } else if matches!(
                path.extension().and_then(|ext| ext.to_str()),
                Some("rs" | "md" | "json")
            ) {
                files.push(path);
            }
        }
    }

    fn mojibake_markers() -> Vec<String> {
        [
            &[0x951f][..],
            &[0xfffd],
            &[0x6d93],
            &[0x9365],
            &[0x6f76],
            &[0x93ba],
            &[0x7487],
            &[0x9422],
            &[0x9225],
            &[0x6769],
            &[0x704f],
            &[0x58c0],
            &[0x7ed4],
            &[0x704f, 0x5fd4, 0x6902],
            &[0x9352, 0x55db, 0x6313],
        ]
        .into_iter()
        .map(|codes| {
            codes
                .iter()
                .filter_map(|code| char::from_u32(*code))
                .collect::<String>()
        })
        .collect()
    }

    #[test]
    fn platform_entry_points_are_cfg_gated_for_windows_macos_and_linux() {
        let main_rs = fs::read_to_string(crate_root().join("src/main.rs"))
            .unwrap()
            .replace("\r\n", "\n");
        let root = crate_root();
        let windows_win32_rs =
            fs::read_to_string(root.join("src/windows_win32_adapter.rs")).unwrap();
        let macos_rs = fs::read_to_string(root.join("src/macos_app.rs")).unwrap();
        let macos_appkit_rs = fs::read_to_string(root.join("src/macos_appkit_adapter.rs")).unwrap();
        let macos_native_host_rs =
            fs::read_to_string(root.join("src/macos_native_host.rs")).unwrap();
        let linux_rs = fs::read_to_string(root.join("src/linux_app.rs")).unwrap();
        let linux_gtk_rs = fs::read_to_string(root.join("src/linux_gtk_adapter.rs")).unwrap();
        let linux_native_host_rs =
            fs::read_to_string(root.join("src/linux_native_host.rs")).unwrap();
        let windows_main_renderer_rs =
            fs::read_to_string(root.join("src/app/main_renderer.rs")).unwrap();
        let native_host_actions_rs =
            fs::read_to_string(root.join("src/app_core/native_host_actions.rs")).unwrap();
        let native_component_protocol_rs =
            fs::read_to_string(root.join("src/app_core/native_component_protocol.rs")).unwrap();
        let settings_model_rs = fs::read_to_string(root.join("src/settings_model.rs")).unwrap();
        let macos_smoke_script =
            fs::read_to_string(root.join("scripts/native-host-smoke-macos.sh")).unwrap();
        let linux_smoke_script =
            fs::read_to_string(root.join("scripts/native-host-smoke-linux.sh")).unwrap();

        assert!(main_rs.contains("#[cfg(target_os = \"windows\")]\nmod app;"));
        assert!(main_rs
            .contains("#[cfg(any(target_os = \"windows\", test))]\nmod windows_win32_adapter;"));
        assert!(main_rs.contains("#[cfg(any(target_os = \"macos\", test))]\nmod macos_app;"));
        assert!(
            main_rs.contains("#[cfg(any(target_os = \"macos\", test))]\nmod macos_appkit_adapter;")
        );
        assert!(
            main_rs.contains("#[cfg(any(target_os = \"macos\", test))]\nmod macos_native_host;")
        );
        assert!(main_rs.contains("#[cfg(any(target_os = \"linux\", test))]\nmod linux_app;"));
        assert!(
            main_rs.contains("#[cfg(any(target_os = \"linux\", test))]\nmod linux_gtk_adapter;")
        );
        assert!(
            main_rs.contains("#[cfg(any(target_os = \"linux\", test))]\nmod linux_native_host;")
        );
        assert!(main_rs.contains("#[path = \"i18n_runtime.rs\"]\nmod i18n;"));
        assert!(main_rs.contains("\nmod settings_model;"));
        assert!(!main_rs.contains("#[cfg(target_os = \"windows\")]\n#[path = \"i18n_runtime.rs\"]"));
        assert!(!main_rs.contains("#[cfg(target_os = \"windows\")]\nmod settings_model;"));
        assert!(main_rs.contains("WindowsWin32AdapterBoundary::default_from_core_contract"));
        assert!(main_rs.contains("fn main() {\n    let _adapter_boundary ="));
        assert!(
            main_rs.contains("if let Some(code) = shell::maybe_run_wechat_ocr_helper_from_args()")
        );
        assert!(main_rs.contains("fn main() {\n    if let Err(err) = macos_app::run()"));
        assert!(main_rs.contains("fn main() {\n    if let Err(err) = linux_app::run()"));
        assert!(!windows_win32_rs.contains("app::run"));
        assert!(windows_win32_rs.contains("WindowsWin32AdapterBoundary"));
        assert!(windows_win32_rs.contains("NativeUiAdapterManifest"));
        assert!(windows_win32_rs.contains("NativeUiPlatform::Windows"));
        assert!(windows_win32_rs.contains("NativeUiToolkit::Win32Gdi"));
        assert!(windows_win32_rs.contains("REQUIRED_WINDOWS_WIN32_HOST_BINDINGS"));
        assert!(windows_win32_rs.contains("WindowsWin32HostBinding::MainWindow"));
        assert!(windows_win32_rs.contains("WindowsWin32HostBinding::SettingsWindow"));
        assert!(windows_win32_rs.contains("WindowsWin32ControlRole"));
        assert!(windows_win32_rs.contains("EDIT"));
        assert!(windows_win32_rs.contains("COMBOBOX"));
        assert!(!macos_rs.contains("windows_sys"));
        assert!(!macos_rs.contains("crate::app::run"));
        assert!(!macos_rs.contains(" app::run("));
        assert!(!macos_appkit_rs.contains("windows_sys"));
        assert!(!macos_appkit_rs.contains("crate::app::run"));
        assert!(!macos_appkit_rs.contains(" app::run("));
        assert!(macos_rs.contains("REQUIRED_UI_HOST_SURFACES"));
        assert!(macos_rs.contains("MacosAppKitAdapterBoundary::default_from_macos_contract"));
        assert!(macos_rs.contains("macos_native_host_launch_plan"));
        assert!(macos_rs.contains("run_real_appkit_host"));
        assert!(macos_rs.contains("MacosAutostartHost"));
        assert!(macos_rs.contains("impl NativeAutostartHost for MacosAutostartHost"));
        assert!(macos_rs.contains("pub(crate) fn apply_autostart("));
        assert!(macos_rs.contains("io.github.qiu7824.zsclip.plist"));
        assert!(macos_rs.contains(".join(\"Library\")"));
        assert!(macos_rs.contains(".join(\"LaunchAgents\")"));
        assert!(macos_appkit_rs.contains("MacosAppKitAdapterBoundary"));
        assert!(macos_appkit_rs.contains("NativeUiAdapterManifest"));
        assert!(macos_appkit_rs.contains("NativeUiPlatform::Macos"));
        assert!(macos_appkit_rs.contains("NativeUiToolkit::AppKitSwiftUI"));
        assert!(macos_native_host_rs.contains("NSApplication"));
        assert!(macos_native_host_rs.contains("NSWindow"));
        assert!(macos_native_host_rs.contains("NSWindowStyleMask::FullSizeContentView"));
        assert!(macos_native_host_rs.contains("NSVisualEffectView::initWithFrame"));
        assert!(macos_native_host_rs.contains("window.setTitlebarAppearsTransparent(true)"));
        assert!(macos_native_host_rs.contains("window.setHidesOnDeactivate(true)"));
        assert!(macos_native_host_rs.contains("window.setLevel(NSFloatingWindowLevel)"));
        assert!(macos_native_host_rs.contains("window.backingScaleFactor()"));
        assert!(macos_native_host_rs.contains("NSAppearanceNameDarkAqua"));
        assert!(macos_native_host_rs.contains("NSEvent::mouseLocation()"));
        assert!(macos_native_host_rs.contains("window.setFrameOrigin(NSPoint::new"));
        assert!(macos_native_host_rs.contains("fn appkit_set_accessibility_label<T>("));
        assert!(macos_native_host_rs.contains("setAccessibilityLabel"));
        assert!(macos_native_host_rs.contains("Clipboard history list"));
        assert!(macos_native_host_rs.contains("presentation.accessibility_label"));
        assert!(macos_native_host_rs.contains("NSSearchField"));
        assert!(macos_native_host_rs.contains("native_host_search_input_specs()"));
        assert!(macos_native_host_rs.contains("toggle_search_field"));
        assert!(macos_native_host_rs.contains("focus_native_search_field"));
        assert!(macos_native_host_rs
            .contains("appkit_event_key_text(event).eq_ignore_ascii_case(\"f\")"));
        assert!(macos_native_host_rs.contains("table_view.scrollRowToVisible(index as NSInteger)"));
        assert!(macos_native_host_rs.contains("zsclipSearchTextChanged:"));
        assert!(macos_native_host_rs.contains("update_clip_list_visibility"));
        assert!(macos_native_host_rs.contains("macos_native_host_projected_clip_items"));
        assert!(macos_native_host_rs.contains("native_host_filtered_projected_clip_item_ids"));
        assert!(macos_native_host_rs.contains("dispatch_appkit_search_text_action"));
        assert!(macos_native_host_rs.contains("setHidden"));
        assert!(macos_native_host_rs.contains("ZSClip Settings"));
        assert!(macos_native_host_rs.contains("present_settings_window"));
        assert!(macos_native_host_rs.contains("NSTabView::initWithFrame"));
        assert!(macos_native_host_rs.contains("native_host_settings_page_tab_specs()"));
        assert!(macos_native_host_rs.contains("appkit_settings_scroll_tab_item(mtm, spec.label)"));
        assert!(macos_native_host_rs.contains("scroller.setDocumentView(Some(&content))"));
        assert!(
            macos_native_host_rs.contains("native_host_settings_section_label(\"group_selector\")")
        );
        assert!(macos_native_host_rs.contains("zsclipAddSettingsGroup:"));
        assert!(macos_native_host_rs.contains("zsclipRenameSettingsGroup:"));
        assert!(macos_native_host_rs.contains("zsclipDeleteSettingsGroup:"));
        assert!(macos_native_host_rs.contains("zsclipMoveSettingsGroupUp:"));
        assert!(macos_native_host_rs.contains("refresh_main_group_state_after_settings_change"));
        assert!(macos_native_host_rs
            .contains("deleted_group_id == Some(self.ivars().current_group_filter.get())"));
        assert!(macos_native_host_rs.contains("zsclipSaveSettings:"));
        assert!(macos_native_host_rs.contains("zsclipOpenSettingsConfig:"));
        assert!(macos_native_host_rs.contains("dispatch_appkit_settings_action"));
        assert!(macos_native_host_rs.contains("zsclipSettingsNativeRouteAction:"));
        assert!(macos_native_host_rs.contains("dispatch_macos_native_settings_route_action"));
        assert!(macos_native_host_rs.contains("zsclipToggleClipboardCapture:"));
        assert!(macos_native_host_rs.contains("zsclipToggleAutostart:"));
        assert!(macos_native_host_rs.contains("zsclipToggleLanSync:"));
        assert!(macos_native_host_rs.contains("zsclipToggleCloudSync:"));
        assert!(macos_native_host_rs.contains("zsclipOpenSyncModeDropdown:"));
        assert!(macos_native_host_rs.contains("dispatch_appkit_settings_control_action"));
        assert!(macos_native_host_rs.contains("NSStatusBar::systemStatusBar"));
        assert!(macos_native_host_rs.contains("NSStatusItem"));
        assert!(macos_native_host_rs.contains("NSMenuItem"));
        assert!(macos_native_host_rs.contains("native_host_status_menu_item_specs()"));
        assert!(macos_native_host_rs.contains("dispatch_appkit_status_menu_action"));
        assert!(macos_native_host_rs.contains("toggle_main_window_visibility"));
        assert!(macos_native_host_rs.contains("action.toggles_main_window_surface()"));
        assert!(macos_native_host_rs.contains("zsclipStatusToggleWindow:"));
        assert!(macos_native_host_rs.contains("zsclipStatusToggleClipboardCapture:"));
        assert!(macos_native_host_rs.contains("zsclipStatusToggleLanSync:"));
        assert!(macos_native_host_rs.contains("zsclipStatusExit:"));
        assert!(macos_native_host_rs.contains("zsclipRowPaste:"));
        assert!(macos_native_host_rs.contains("zsclipRowCopy:"));
        assert!(macos_native_host_rs.contains("zsclipRowPin:"));
        assert!(macos_native_host_rs.contains("zsclipRowToPhrase:"));
        assert!(macos_native_host_rs.contains("zsclipRowDelete:"));
        assert!(macos_native_host_rs.contains("zsclipRowEdit:"));
        assert!(macos_native_host_rs.contains("zsclipRowOpenPath:"));
        assert!(macos_native_host_rs.contains("zsclipRowTextTranslate:"));
        assert!(macos_native_host_rs.contains("dispatch_appkit_row_action"));
        assert!(macos_native_host_rs.contains("zsclipShowVvPopup:"));
        assert!(macos_native_host_rs.contains("zsclipVvSelect:"));
        assert!(macos_native_host_rs.contains("dispatch_appkit_vv_select_event"));
        assert!(macos_native_host_rs.contains("dispatch_appkit_vv_paste"));
        assert!(macos_native_host_rs.contains("ZSClip AppKit VV paste"));
        assert!(macos_native_host_rs.contains("ZSClip AppKit VV native paste shortcut posted="));
        assert!(macos_native_host_rs.contains("native_host_vv_popup_render_plan"));
        assert!(macos_native_host_rs.contains("ZSClip VV Popup"));
        assert!(macos_native_host_rs.contains("NSFont::monospacedSystemFontOfSize_weight"));
        assert!(macos_native_host_rs.contains("fn dismiss_native_vv_popup_for_local_mouse_event("));
        assert!(macos_native_host_rs.contains("dismiss_native_vv_popup(\"global_mouse_down\")"));
        assert!(macos_native_host_rs.contains("run_auto_smoke_if_requested"));
        assert!(macos_native_host_rs.contains("ZSCLIP_NATIVE_HOST_AUTO_SMOKE"));
        assert!(macos_native_host_rs.contains("ZSClip AppKit auto smoke finished"));
        assert!(macos_native_host_rs.contains("ZSClip AppKit clipboard text smoke"));
        assert!(macos_native_host_rs.contains("ZSClip AppKit clipboard file smoke"));
        assert!(macos_native_host_rs.contains("ZSClip AppKit clipboard sequence smoke"));
        assert!(macos_native_host_rs.contains("ZSClip AppKit clipboard monitor smoke"));
        assert!(macos_native_host_rs.contains("ZSClip AppKit shell open smoke dry_run="));
        assert!(macos_native_host_rs.contains("ZSClip AppKit file picker smoke injected=true"));
        assert!(macos_native_host_rs.contains("macos_native_identity_smoke"));
        assert!(macos_native_host_rs.contains("ZSClip AppKit identity smoke queried=true"));
        assert!(macos_rs.contains("ZSCLIP_NATIVE_HOST_SHELL_OPEN_DRY_RUN"));
        assert!(macos_rs.contains("ZSCLIP_NATIVE_HOST_FILE_PICKER_SMOKE_PATH"));
        assert!(macos_smoke_script.contains("ZSClip AppKit VV native paste shortcut posted="));
        assert!(macos_smoke_script.contains("ZSCLIP_NATIVE_HOST_SHELL_OPEN_DRY_RUN"));
        assert!(macos_smoke_script.contains("SHELL_OPEN_DRY_RUN_LOG=true"));
        assert!(macos_smoke_script.contains(
            "ZSClip AppKit shell open smoke dry_run=$SHELL_OPEN_DRY_RUN_LOG recorded=true"
        ));
        assert!(macos_smoke_script
            .contains("ZSClip AppKit file picker smoke injected=true recorded=true selected=true"));
        assert!(macos_smoke_script.contains("ZSClip AppKit identity smoke queried=true"));
        assert!(macos_native_host_rs.contains("ZSClip Edit"));
        assert!(macos_native_host_rs.contains("zsclipSaveEditText:"));
        assert!(macos_native_host_rs.contains("edit_text_view: OnceCell<Retained<NSTextView>>"));
        assert!(macos_native_host_rs.contains("NSTextView::initWithFrame"));
        assert!(macos_native_host_rs
            .contains("edit_text_scroller.setDocumentView(Some(&edit_text_view))"));
        assert!(macos_native_host_rs.contains("beginSheet_completionHandler(&window, None)"));
        assert!(macos_native_host_rs.contains("edit_text_view.string().to_string()"));
        assert!(macos_native_host_rs.contains("ZSClip AppKit edit save"));
        assert!(macos_native_host_rs.contains("dispatch_appkit_edit_text_save"));
        assert!(macos_native_host_rs.contains("edit save item_id="));
        assert!(macos_native_host_rs.contains("selected_item_id"));
        assert!(macos_native_host_rs.contains("row selected item_id="));
        assert!(macos_native_host_rs.contains("refresh_native_clip_rows"));
        assert!(macos_native_host_rs.contains("NSButton::buttonWithTitle_target_action"));
        assert!(macos_native_host_rs.contains("native_host_clip_row_specs("));
        assert!(macos_native_host_rs.contains("native_host_edit_text_button_specs()"));
        assert!(macos_native_host_rs.contains("native_host_main_action_button_specs()"));
        assert!(macos_native_host_rs.contains("native_host_main_tool_button_specs()"));
        assert!(macos_native_host_rs.contains("native_host_row_action_button_specs()"));
        assert!(macos_native_host_rs.contains("native_host_settings_action_button_specs()"));
        assert!(macos_native_host_rs.contains("native_host_settings_control_button_specs()"));
        assert!(macos_native_host_rs.contains("native_host_settings_group_button_specs()"));
        assert!(macos_native_host_rs.contains("native_host_settings_platform_button_specs()"));
        assert!(macos_native_host_rs.contains("apply_native_settings_control_action"));
        assert!(macos_native_host_rs.contains("action.binding_control_key()"));
        assert!(macos_native_host_rs.contains("native_host_dialog_button_specs()"));
        assert!(macos_native_host_rs.contains("native_host_vv_select_specs(&plan"));
        assert!(macos_native_host_rs.contains("appkit_host_action_selector(spec.action)"));
        assert!(macos_native_host_rs.contains("appkit_main_tool_action_selector(spec.action)"));
        assert!(macos_native_host_rs.contains("appkit_row_action_selector(spec.action)"));
        assert!(macos_native_host_rs.contains("dispatch_macos_native_row_action_for_item"));
        assert!(macos_native_host_rs.contains("ZSClip AppKit row paste shortcut posted="));
        assert!(macos_native_host_rs.contains("native_host_reconciled_selected_item_id"));
        assert!(macos_rs.contains("dispatch_macos_native_row_action_for_item"));
        assert!(macos_rs.contains("update_native_clip_items_pinned"));
        assert!(macos_rs.contains("delete_native_clip_items"));
        assert!(macos_native_host_rs.contains("appkit_settings_action_selector(spec.action)"));
        assert!(
            macos_native_host_rs.contains("appkit_settings_control_action_selector(spec.action)")
        );
        assert!(macos_native_host_rs.contains("appkit_settings_group_action_selector(spec.action)"));
        assert!(
            macos_native_host_rs.contains("appkit_settings_platform_action_selector(spec.action)")
        );
        assert!(macos_native_host_rs.contains("appkit_edit_text_action_selector(spec.action)"));
        assert!(macos_native_host_rs.contains("appkit_dialog_action_selector(spec.action)"));
        assert!(macos_native_host_rs.contains("zsclipOpenSettings:"));
        assert!(macos_native_host_rs.contains("app.run()"));
        assert!(macos_native_host_rs.contains("dispatch_appkit_host_action"));
        assert!(macos_appkit_rs.contains("REQUIRED_MACOS_APPKIT_HOST_BINDINGS"));
        assert!(macos_appkit_rs.contains("MacosAppKitHostBinding::MainWindow"));
        assert!(macos_appkit_rs.contains("MacosAppKitHostBinding::SettingsWindow"));
        assert!(macos_appkit_rs.contains("MacosAppKitWidgetRole"));
        assert!(macos_appkit_rs.contains("NSTextField"));
        assert!(macos_appkit_rs.contains("NSPopUpButton"));
        assert!(macos_rs.contains("REQUIRED_STATUS_ITEM_HOST_OPERATIONS"));
        assert!(macos_rs.contains("impl ClipboardHost for MacosClipboardHost"));
        assert!(macos_rs.contains("MacosClipboardState"));
        assert!(macos_rs.contains("fn read_text()"));
        assert!(macos_rs.contains("fn write_image_rgba("));
        assert!(macos_rs.contains("NSPasteboard::generalPasteboard()"));
        assert!(macos_rs.contains("pasteboard.readObjectsForClasses_options"));
        assert!(macos_rs.contains("pasteboard.writeObjects(&objects)"));
        assert!(macos_rs.contains("NSURL::from_file_path"));
        assert!(macos_rs.contains("url.to_file_path()"));
        assert!(macos_rs.contains("changeCount()"));
        assert!(macos_rs.contains("poll_clipboard_monitor::<MacosClipboardHost>"));
        assert!(macos_rs.contains("ApplicationEvent::ClipboardChanged"));
        assert!(macos_rs.contains("impl StatusItemHost for MacosStatusItemHost"));
        assert!(macos_rs.contains("fn present_menu(&mut self"));
        assert!(macos_rs.contains("impl NativePopupMenuHost for MacosPopupMenuHost"));
        assert!(macos_rs.contains("fn present_popup_menu("));
        assert!(macos_rs.contains("impl NativeDialogHost for MacosDialogHost"));
        assert!(macos_rs.contains("MacosDialogMessage"));
        assert!(macos_rs.contains("fn confirm("));
        assert!(macos_rs.contains("MacosShellOpenHost"));
        assert!(macos_rs.contains("impl NativeShellOpenHost for MacosShellOpenHost"));
        assert!(macos_rs.contains("system_open_path"));
        assert!(macos_rs.contains("open_macos_url_or_file(path.to_string()).is_ok()"));
        assert!(macos_rs.contains("NSWorkspace::sharedWorkspace"));
        assert!(macos_rs.contains("macos_frontmost_process_handle"));
        assert!(macos_rs.contains("macos_process_name_for_pid"));
        assert!(macos_rs.contains("macos_bundle_id_for_pid"));
        assert!(macos_rs.contains("macos_activate_process"));
        assert!(macos_rs.contains("macos_post_command_v_shortcut"));
        assert!(macos_rs.contains("System Events"));
        assert!(macos_rs.contains("osascript"));
        assert!(macos_rs.contains("impl NativeTextInputDialogHost for MacosTextInputDialogHost"));
        assert!(macos_rs.contains("prompt_macos_native_text"));
        assert!(macos_rs.contains("alert.setAccessoryView(Some(&field))"));
        assert!(macos_rs.contains("impl NativeEditTextDialogHost for MacosEditTextDialogHost"));
        assert!(macos_rs.contains("edit_macos_native_text"));
        assert!(macos_rs.contains("alert.addButtonWithTitle(ns_string!(\"Save\"))"));
        assert!(macos_rs.contains("save_handler"));
        assert!(macos_rs.contains(".save_text(&field.stringValue().to_string())"));
        assert!(macos_rs.contains("dispatch_macos_native_edit_text_save"));
        assert!(macos_rs.contains("dispatch_macos_native_vv_paste"));
        assert!(macos_rs.contains("execute_native_vv_paste"));
        assert!(macos_rs.contains("dispatch_macos_native_create_group"));
        assert!(macos_rs.contains("dispatch_macos_native_rename_group"));
        assert!(macos_rs.contains("dispatch_macos_native_delete_group"));
        assert!(macos_rs.contains("dispatch_macos_native_move_group"));
        assert!(macos_rs.contains("native_clip_list_items(0, 64)"));
        assert!(macos_rs.contains("zsclip.row.edit.save_db"));
        assert!(macos_rs.contains("zsclip.row.edit.save_missing"));
        assert!(macos_rs.contains("MainRenderInput::empty_records"));
        assert!(macos_rs.contains(".render_plan("));
        assert!(macos_rs.contains("MacosMainWindowHost"));
        assert!(macos_rs.contains("impl NativeMainWindowHost for MacosMainWindowHost"));
        assert!(macos_rs.contains("MacosMainSearchControlHost"));
        assert!(
            macos_rs.contains("impl NativeMainSearchControlHost for MacosMainSearchControlHost")
        );
        assert!(macos_rs.contains("MacosSettingsWindowModel"));
        assert!(macos_rs.contains("MacosSettingsWindowHost"));
        assert!(macos_rs.contains("impl NativeSettingsWindowHost for MacosSettingsWindowHost"));
        assert!(macos_rs.contains("settings_content_render_plan("));
        assert!(!linux_rs.contains("windows_sys"));
        assert!(!linux_rs.contains("crate::app::run"));
        assert!(!linux_rs.contains(" app::run("));
        assert!(!linux_gtk_rs.contains("windows_sys"));
        assert!(!linux_gtk_rs.contains("crate::app::run"));
        assert!(!linux_gtk_rs.contains(" app::run("));
        assert!(linux_rs.contains("LinuxApplicationModel"));
        assert!(linux_rs.contains("system_read_text"));
        assert!(linux_rs.contains("system_write_text"));
        assert!(linux_rs.contains("system_clipboard_fingerprint"));
        assert!(linux_rs.contains("last_system_fingerprint"));
        assert!(linux_rs.contains("observed_system_sequence"));
        assert!(linux_rs.contains("poll_clipboard_monitor::<LinuxClipboardHost>"));
        assert!(linux_rs.contains("ApplicationEvent::ClipboardChanged"));
        assert!(linux_rs.contains("arboard::Clipboard"));
        assert!(linux_rs.contains("system_open_path"));
        assert!(linux_rs.contains("open_linux_url_or_file(path.to_string()).is_ok()"));
        assert!(linux_rs.contains("AppInfo::launch_default_for_uri"));
        assert!(linux_rs.contains("dispatch_linux_native_edit_text_save"));
        assert!(linux_rs.contains("dispatch_linux_native_vv_paste"));
        assert!(linux_rs.contains("execute_native_vv_paste"));
        assert!(linux_rs.contains("dispatch_linux_native_create_group"));
        assert!(linux_rs.contains("dispatch_linux_native_rename_group"));
        assert!(linux_rs.contains("dispatch_linux_native_delete_group"));
        assert!(linux_rs.contains("dispatch_linux_native_move_group"));
        assert!(linux_rs.contains("native_clip_list_items(0, 64)"));
        assert!(linux_rs.contains("zsclip.row.edit.save_db"));
        assert!(linux_rs.contains("zsclip.row.edit.save_missing"));
        assert!(linux_rs.contains("LinuxGtkAdapterBoundary::default_from_linux_contract"));
        assert!(linux_rs.contains("linux_native_host_launch_plan"));
        assert!(linux_rs.contains("run_real_gtk_host"));
        assert!(linux_rs.contains("LinuxAutostartHost"));
        assert!(linux_rs.contains("impl NativeAutostartHost for LinuxAutostartHost"));
        assert!(linux_rs.contains("pub(crate) fn apply_autostart("));
        assert!(linux_rs.contains("zsclip.desktop"));
        assert!(linux_rs.contains("X-ZSClip-Autostart=true"));
        assert!(linux_gtk_rs.contains("LinuxGtkAdapterBoundary"));
        assert!(linux_gtk_rs.contains("NativeUiAdapterManifest"));
        assert!(linux_gtk_rs.contains("NativeUiPlatform::Linux"));
        assert!(linux_gtk_rs.contains("NativeUiToolkit::Gtk4Libadwaita"));
        assert!(linux_native_host_rs.contains("Application::builder()"));
        assert!(linux_native_host_rs.contains("ApplicationWindow::builder()"));
        assert!(linux_native_host_rs.contains("fn gtk_native_window_traits("));
        assert!(linux_native_host_rs.contains("gtk::Settings::default()"));
        assert!(linux_native_host_rs.contains("is_gtk_application_prefer_dark_theme()"));
        assert!(linux_native_host_rs.contains(".monitor_at_surface(&surface)"));
        assert!(linux_native_host_rs.contains("monitor.scale_factor()"));
        assert!(linux_native_host_rs.contains("window.scale_factor()"));
        assert!(linux_native_host_rs.contains("always_on_top_supported={}"));
        assert!(linux_native_host_rs.contains("SearchEntry::new()"));
        assert!(linux_native_host_rs.contains("native_host_search_input_specs()"));
        assert!(linux_native_host_rs.contains("connect_search_changed"));
        assert!(linux_native_host_rs.contains("update_clip_list_visibility"));
        assert!(linux_native_host_rs.contains("linux_native_host_projected_clip_items"));
        assert!(linux_native_host_rs.contains("native_host_filtered_projected_clip_item_ids"));
        assert!(linux_native_host_rs.contains("dispatch_linux_native_search_text_action"));
        assert!(linux_native_host_rs.contains("dispatch_gtk_search_text_action"));
        assert!(linux_native_host_rs.contains("toggles_search_surface"));
        assert!(linux_native_host_rs.contains("set_visible"));
        assert!(linux_native_host_rs.contains("ZSClip Settings"));
        assert!(linux_native_host_rs.contains("present_settings_window"));
        assert!(
            linux_native_host_rs.contains("native_host_settings_section_label(\"group_selector\")")
        );
        assert!(linux_native_host_rs.contains("dispatch_linux_native_create_group"));
        assert!(linux_native_host_rs.contains("dispatch_linux_native_rename_group"));
        assert!(linux_native_host_rs.contains("dispatch_linux_native_delete_group"));
        assert!(linux_native_host_rs.contains("dispatch_linux_native_move_group"));
        assert!(linux_native_host_rs.contains("native_host_settings_action_button_specs()"));
        assert!(linux_native_host_rs.contains("native_host_settings_control_button_specs()"));
        assert!(linux_native_host_rs.contains("native_host_settings_group_button_specs()"));
        assert!(linux_native_host_rs.contains("native_host_settings_platform_button_specs()"));
        assert!(linux_native_host_rs.contains("apply_gtk_settings_control_action"));
        assert!(linux_native_host_rs.contains("action.binding_control_key()"));
        assert!(linux_native_host_rs.contains("native_host_dialog_button_specs()"));
        assert!(linux_native_host_rs.contains("dispatch_linux_native_settings_action"));
        assert!(linux_native_host_rs.contains("dispatch_gtk_settings_action"));
        assert!(linux_native_host_rs.contains("dispatch_linux_native_settings_route_action"));
        assert!(linux_native_host_rs.contains("dispatch_linux_native_settings_control_action"));
        assert!(linux_native_host_rs.contains("dispatch_gtk_settings_control_action"));
        assert!(linux_native_host_rs.contains("MenuButton::builder()"));
        assert!(linux_native_host_rs.contains("PopoverMenu::from_model"));
        assert!(linux_native_host_rs.contains("gio::SimpleAction"));
        assert!(linux_native_host_rs.contains("ZsclipGtkStatusNotifier"));
        assert!(linux_native_host_rs.contains("ZSClip GTK StatusNotifierItem installed"));
        assert!(linux_native_host_rs.contains("ZSClip GTK StatusNotifierItem unavailable:"));
        assert!(linux_native_host_rs.contains("struct GroupPopupMenus"));
        assert!(linux_native_host_rs.contains("register_dynamic_group_popup_actions"));
        assert!(linux_native_host_rs.contains("refresh_group_popup_menus"));
        assert!(linux_native_host_rs.contains("replace_popup_menu_entries"));
        assert!(linux_native_host_rs.contains("menu.remove_all()"));
        assert!(linux_native_host_rs.contains("dispatch_gtk_status_menu_action"));
        assert!(linux_native_host_rs.contains("toggle_gtk_main_window"));
        assert!(linux_native_host_rs.contains("install_status_menu(app, &status, &window)"));
        assert!(linux_native_host_rs.contains("install_status_notifier(app, &status, &window)"));
        assert!(linux_native_host_rs.contains("native_host_status_menu_item_specs()"));
        assert!(linux_native_host_rs.contains("native_host_row_action_button_specs"));
        assert!(linux_native_host_rs.contains("dispatch_linux_native_row_action"));
        assert!(linux_native_host_rs.contains("dispatch_gtk_row_action"));
        assert!(linux_native_host_rs.contains("VV Popup"));
        assert!(linux_native_host_rs.contains("present_vv_popup_window"));
        assert!(linux_native_host_rs.contains("native_host_vv_popup_render_plan"));
        assert!(linux_native_host_rs.contains("native_host_vv_select_specs(&plan"));
        assert!(linux_native_host_rs.contains("number.add_css_class(\"vv-index\")"));
        assert!(linux_native_host_rs.contains("label.add_css_class(\"vv-preview\")"));
        assert!(linux_native_host_rs.contains("window.connect_notify_local(Some(\"is-active\")"));
        assert!(linux_native_host_rs.contains("dispatch_linux_native_vv_select_event"));
        assert!(linux_native_host_rs.contains("dispatch_gtk_vv_select_event"));
        assert!(linux_native_host_rs.contains("dispatch_linux_native_vv_paste"));
        assert!(linux_native_host_rs.contains("dispatch_gtk_vv_paste"));
        assert!(linux_native_host_rs.contains("ZSClip GTK VV paste"));
        assert!(linux_native_host_rs.contains("ZSClip GTK VV native paste shortcut posted="));
        assert!(linux_native_host_rs.contains("run_auto_smoke_if_requested"));
        assert!(linux_native_host_rs.contains("ZSCLIP_NATIVE_HOST_AUTO_SMOKE"));
        assert!(linux_native_host_rs.contains("ZSClip GTK auto smoke finished"));
        assert!(linux_native_host_rs.contains("ZSClip GTK clipboard text smoke"));
        assert!(linux_native_host_rs.contains("ZSClip GTK clipboard file smoke"));
        assert!(linux_native_host_rs.contains("ZSClip GTK clipboard sequence smoke"));
        assert!(linux_native_host_rs.contains("ZSClip GTK clipboard monitor smoke"));
        assert!(linux_native_host_rs.contains("ZSClip GTK shell open smoke dry_run="));
        assert!(linux_native_host_rs.contains("ZSClip GTK file picker smoke injected=true"));
        assert!(linux_native_host_rs.contains("linux_native_identity_smoke"));
        assert!(linux_native_host_rs.contains("ZSClip GTK identity smoke queried=true"));
        assert!(linux_rs.contains("ZSCLIP_NATIVE_HOST_SHELL_OPEN_DRY_RUN"));
        assert!(linux_rs.contains("ZSCLIP_NATIVE_HOST_FILE_PICKER_SMOKE_PATH"));
        assert!(linux_smoke_script.contains("ZSClip GTK VV native paste shortcut posted="));
        assert!(linux_smoke_script.contains("ZSClip GTK StatusNotifierItem installed"));
        assert!(linux_smoke_script.contains("ZSClip GTK StatusNotifierItem unavailable:"));
        assert!(linux_smoke_script.contains("ZSCLIP_NATIVE_HOST_SHELL_OPEN_DRY_RUN"));
        assert!(linux_smoke_script.contains("SHELL_OPEN_DRY_RUN_LOG=true"));
        assert!(linux_smoke_script
            .contains("ZSClip GTK shell open smoke dry_run=$SHELL_OPEN_DRY_RUN_LOG recorded=true"));
        assert!(linux_smoke_script
            .contains("ZSClip GTK file picker smoke injected=true recorded=true selected=true"));
        assert!(linux_smoke_script.contains("ZSClip GTK identity smoke queried=true"));
        assert!(linux_native_host_rs.contains("ZSClip Edit"));
        assert!(linux_native_host_rs.contains("present_edit_text_window"));
        assert!(linux_native_host_rs.contains("ZSClip GTK edit save"));
        assert!(linux_native_host_rs.contains("dispatch_linux_native_edit_text_save"));
        assert!(linux_native_host_rs.contains("edit save item_id="));
        assert!(linux_native_host_rs.contains("struct EditRefreshTarget"));
        assert!(linux_native_host_rs.contains("linux_native_host_projected_clip_items_for_group"));
        assert!(linux_native_host_rs.contains("target.current_group_filter.get()"));
        assert!(linux_native_host_rs.contains("selected_item_id"));
        assert!(linux_native_host_rs.contains("row selected item_id="));
        assert!(linux_native_host_rs.contains("refresh_clip_rows"));
        assert!(linux_native_host_rs.contains("Button::with_label"));
        assert!(linux_native_host_rs.contains("native_host_clip_row_specs("));
        assert!(linux_native_host_rs.contains("native_host_edit_text_button_specs()"));
        assert!(linux_native_host_rs.contains("native_host_main_action_button_specs()"));
        assert!(linux_native_host_rs.contains("native_host_main_tool_button_specs()"));
        assert!(linux_native_host_rs.contains("native_host_row_action_button_specs()"));
        assert!(linux_native_host_rs.contains("button.set_widget_name(spec.id)"));
        assert!(linux_native_host_rs.contains("connect_clicked"));
        assert!(linux_native_host_rs.contains("dispatch_linux_native_host_action"));
        assert!(linux_native_host_rs.contains("dispatch_linux_native_row_action_for_item"));
        assert!(linux_native_host_rs.contains("ZSClip GTK row paste shortcut posted="));
        assert!(linux_native_host_rs.contains("reconcile_selected_item_id"));
        assert!(linux_native_host_rs.contains("sync_clip_list_selection(&clip_list, &clip_rows"));
        assert!(linux_native_host_rs.contains("fn install_main_window_keyboard_controller("));
        assert!(linux_native_host_rs.contains("gdk::ModifierType::CONTROL_MASK"));
        assert!(
            linux_native_host_rs.contains("key == gdk::Key::Return || key == gdk::Key::KP_Enter")
        );
        assert!(linux_native_host_rs.contains("reload_clip_items_for_group_with_selection"));
        assert!(linux_rs.contains("dispatch_linux_native_row_action_for_item"));
        assert!(linux_rs.contains("update_native_clip_items_pinned"));
        assert!(linux_rs.contains("delete_native_clip_items"));
        assert!(
            windows_main_renderer_rs.contains("native_host_clip_row_presentation_for_clip_item")
        );
        assert!(windows_main_renderer_rs.contains("main_row_icon_kind_for_clip_presentation"));
        assert!(windows_main_renderer_rs.contains("row_presentation.pin_badge.is_some()"));
        assert!(windows_main_renderer_rs.contains("row_presentation.preview"));
        assert!(linux_native_host_rs.contains("window.present()"));
        assert!(native_host_actions_rs.contains("NativeHostClipRowPresentation"));
        assert!(native_host_actions_rs.contains("NativeHostClipKindIcon::Folder"));
        assert!(native_host_actions_rs.contains("native_host_clip_row_presentation_for_clip_item"));
        assert!(native_host_actions_rs.contains("NativeHostRowAction::ToPhrase"));
        assert!(native_host_actions_rs.contains("NativeHostRowAction::Edit"));
        assert!(native_host_actions_rs.contains("native_host_edit_text_plan"));
        assert!(native_host_actions_rs.contains("native_host_edit_text_plan_for_item"));
        assert!(native_host_actions_rs.contains("NativeHostRowAction::OpenPath"));
        assert!(native_host_actions_rs.contains("NativeHostRowAction::TextTranslate"));
        assert!(native_host_actions_rs.contains("NativeHostStatusMenuAction::ToggleWindow"));
        assert!(
            native_host_actions_rs.contains("NativeHostStatusMenuAction::ToggleClipboardCapture")
        );
        assert!(native_host_actions_rs.contains("NativeHostStatusMenuAction::ToggleLanSync"));
        assert!(native_host_actions_rs.contains("NativeHostStatusMenuAction::Exit"));
        assert!(settings_model_rs.contains("\"capture_enable\""));
        assert!(settings_model_rs.contains("native_setting_binding(\"clipboard_capture_enabled\")"));
        assert!(settings_model_rs.contains("\"clipboard_capture_enabled\""));
        assert!(native_component_protocol_rs.contains("NativeComponentSpec"));
        assert!(native_component_protocol_rs.contains("NativeUiProtocolSurface"));
        assert!(native_component_protocol_rs.contains("NativeUiProtocolSurfaceKind::MainWindow"));
        assert!(native_component_protocol_rs.contains("NativeUiProtocolSurfaceKind::Menu"));
        assert!(native_component_protocol_rs.contains("NativeUiProtocolSurfaceKind::SettingsPage"));
        assert!(native_component_protocol_rs.contains("NativeUiProtocolSurfaceKind::Dialog"));
        assert!(
            native_component_protocol_rs.contains("NativeUiProtocolSurfaceKind::DynamicControls")
        );
        assert!(native_component_protocol_rs.contains("native_ui_protocol_surfaces"));
        assert!(native_component_protocol_rs.contains("NativeComponentKind::MenuItem"));
        assert!(native_component_protocol_rs.contains("NativeComponentKind::SearchInput"));
        assert!(native_component_protocol_rs.contains("NativeComponentAction::HostUi"));
        assert!(native_component_protocol_rs.contains("NativeComponentAction::MainTool"));
        assert!(native_component_protocol_rs.contains("NativeComponentAction::ClipRow"));
        assert!(native_component_protocol_rs.contains("NativeComponentAction::Row"));
        assert!(native_component_protocol_rs.contains("NativeComponentAction::SearchControl"));
        assert!(native_component_protocol_rs.contains("NativeComponentAction::StatusMenu"));
        assert!(native_component_protocol_rs.contains("NativeComponentAction::Settings"));
        assert!(native_component_protocol_rs.contains("NativeComponentAction::SettingsControl"));
        assert!(native_component_protocol_rs.contains("NativeComponentAction::SettingsGroup"));
        assert!(native_component_protocol_rs.contains("NativeComponentAction::SettingsPlatform"));
        assert!(native_component_protocol_rs.contains("NativeComponentAction::EditText"));
        assert!(native_component_protocol_rs.contains("NativeComponentAction::VvSelect"));
        assert!(native_component_protocol_rs.contains("NativeComponentAction::Dialog"));
        assert!(native_component_protocol_rs.contains("native_host_clip_row_specs"));
        assert!(native_component_protocol_rs.contains("native_host_search_component_specs"));
        assert!(native_component_protocol_rs.contains("native_host_status_menu_component_specs"));
        assert!(native_component_protocol_rs.contains("native_host_edit_text_button_specs"));
        assert!(native_component_protocol_rs.contains("native_host_vv_select_specs"));
        assert!(native_component_protocol_rs.contains("native_host_main_action_button_specs"));
        assert!(native_component_protocol_rs.contains("native_host_main_tool_button_specs"));
        assert!(native_component_protocol_rs.contains("native_host_row_action_button_specs"));
        assert!(native_component_protocol_rs.contains("native_host_settings_action_button_specs"));
        assert!(native_component_protocol_rs.contains("native_host_settings_control_button_specs"));
        assert!(native_component_protocol_rs.contains("settings.control.autostart"));
        assert!(native_host_actions_rs.contains("NativeHostSettingsControlAction::ToggleAutostart"));
        assert!(native_host_actions_rs.contains("Some(\"auto_start\")"));
        assert!(native_component_protocol_rs.contains("native_host_settings_group_button_specs"));
        assert!(native_component_protocol_rs.contains("native_host_settings_platform_button_specs"));
        assert!(native_component_protocol_rs.contains("native_host_dialog_button_specs"));
        assert!(linux_gtk_rs.contains("REQUIRED_LINUX_GTK_HOST_BINDINGS"));
        assert!(linux_gtk_rs.contains("LinuxGtkHostBinding::MainExecutionPlan"));
        assert!(linux_gtk_rs.contains("shared_main_execution_plan_bridge"));
        assert!(linux_gtk_rs.contains("LinuxGtkHostBinding::MainWindow"));
        assert!(linux_gtk_rs.contains("LinuxGtkHostBinding::SettingsWindow"));
        assert!(linux_gtk_rs.contains("LinuxGtkWidgetRole"));
        assert!(linux_gtk_rs.contains("gtk::Entry"));
        assert!(linux_gtk_rs.contains("adw::ComboRow"));
        assert!(linux_rs.contains("LinuxClipboardHost"));
        assert!(linux_rs.contains("LinuxNativeStyleResolver"));
        assert!(linux_rs.contains("LinuxNativeControlMapper"));
        assert!(linux_rs.contains("LinuxTextLayout"));
        assert!(linux_rs.contains("LinuxStatusItemHost"));
        assert!(linux_rs.contains("LinuxPopupMenuHost"));
        assert!(linux_rs.contains("LinuxTransientWindowHost"));
        assert!(linux_rs.contains("LinuxImeHost"));
        assert!(linux_rs.contains("LinuxDialogHost"));
        assert!(linux_rs.contains("LinuxShellOpenHost"));
        assert!(linux_rs.contains("LinuxWindowIdentityHost"));
        assert!(linux_rs.contains("LinuxPasteTargetHost"));
        assert!(linux_rs.contains("linux_foreground_window_handle"));
        assert!(linux_rs.contains("linux_process_name_for_window"));
        assert!(linux_rs.contains("linux_window_class_name"));
        assert!(linux_rs.contains("linux_activate_window"));
        assert!(linux_rs.contains("linux_send_ctrl_v"));
        assert!(linux_rs.contains("xdotool"));
        assert!(linux_rs.contains("xprop"));
        assert!(linux_rs.contains("LinuxTextCaretHost"));
        assert!(linux_rs.contains("LinuxFileDialogHost"));
        assert!(linux_rs.contains("LinuxTextInputDialogHost"));
        assert!(linux_rs.contains("LinuxEditTextDialogHost"));
        assert!(linux_rs.contains("LinuxMailMergeWindowHost"));
        assert!(linux_rs.contains("LinuxRenderer"));
        assert!(linux_rs.contains("LinuxMainWindowHost"));
        assert!(linux_rs.contains("LinuxMainSearchControlHost"));
        assert!(linux_rs.contains("LinuxSettingsWindowHost"));
        assert!(linux_rs.contains("LinuxSettingsControlHost"));
        assert!(linux_rs.contains("LinuxSettingsDropdownHost"));
        assert!(linux_rs.contains("impl ClipboardHost for LinuxClipboardHost"));
        assert!(linux_rs.contains("impl NativeStyleResolver for LinuxNativeStyleResolver"));
        assert!(linux_rs.contains("impl NativeControlMapper for LinuxNativeControlMapper"));
        assert!(linux_rs.contains("impl TextLayout for LinuxTextLayout"));
        assert!(linux_rs.contains("impl StatusItemHost for LinuxStatusItemHost"));
        assert!(linux_rs.contains("impl NativePopupMenuHost for LinuxPopupMenuHost"));
        assert!(linux_rs.contains("impl NativeTransientWindowHost for LinuxTransientWindowHost"));
        assert!(linux_rs.contains("impl NativeImeHost for LinuxImeHost"));
        assert!(linux_rs.contains("impl NativeDialogHost for LinuxDialogHost"));
        assert!(linux_rs.contains("impl NativeShellOpenHost for LinuxShellOpenHost"));
        assert!(linux_rs.contains("impl NativeWindowIdentityHost for LinuxWindowIdentityHost"));
        assert!(linux_rs.contains("impl NativePasteTargetHost for LinuxPasteTargetHost"));
        assert!(linux_rs.contains("impl NativeTextCaretHost for LinuxTextCaretHost"));
        assert!(linux_rs.contains("impl NativeFileDialogHost for LinuxFileDialogHost"));
        assert!(linux_rs.contains("impl NativeTextInputDialogHost for LinuxTextInputDialogHost"));
        assert!(linux_rs.contains("prompt_linux_native_text"));
        assert!(linux_rs.contains("dialog.add_button(\"OK\", ResponseType::Accept)"));
        assert!(linux_rs.contains("entry.set_activates_default(true)"));
        assert!(linux_rs.contains("impl NativeEditTextDialogHost for LinuxEditTextDialogHost"));
        assert!(linux_rs.contains("edit_linux_native_text"));
        assert!(linux_rs.contains("dialog.add_button(\"Save\", ResponseType::Accept)"));
        assert!(linux_rs.contains("save_handler.save_text(&entry.text().to_string())"));
        assert!(linux_rs.contains("impl NativeMailMergeWindowHost for LinuxMailMergeWindowHost"));
        assert!(linux_rs.contains("impl Renderer for LinuxRenderer"));
        assert!(linux_rs.contains("impl NativeMainWindowHost for LinuxMainWindowHost"));
        assert!(
            linux_rs.contains("impl NativeMainSearchControlHost for LinuxMainSearchControlHost")
        );
        assert!(linux_rs.contains("impl NativeSettingsWindowHost for LinuxSettingsWindowHost"));
        assert!(linux_rs.contains("impl NativeSettingsControlHost for LinuxSettingsControlHost"));
        assert!(linux_rs.contains("impl NativeSettingsDropdownHost for LinuxSettingsDropdownHost"));
        assert!(linux_rs.contains("REQUIRED_UI_HOST_SURFACES"));
        assert!(linux_rs.contains("REQUIRED_MAIN_HOST_EXECUTION_PLAN_KINDS"));
        assert!(linux_rs.contains("REQUIRED_NATIVE_STYLE_HOST_OPERATIONS"));
        assert!(linux_rs.contains("REQUIRED_NATIVE_CONTROL_MAPPER_OPERATIONS"));
        assert!(linux_rs.contains("REQUIRED_TEXT_LAYOUT_HOST_OPERATIONS"));
        assert!(linux_rs.contains("REQUIRED_STATUS_ITEM_HOST_OPERATIONS"));
        assert!(linux_rs.contains("REQUIRED_NATIVE_POPUP_MENU_HOST_OPERATIONS"));
        assert!(linux_rs.contains("REQUIRED_NATIVE_TRANSIENT_WINDOW_HOST_OPERATIONS"));
        assert!(linux_rs.contains("REQUIRED_NATIVE_IME_HOST_OPERATIONS"));
        assert!(linux_rs.contains("REQUIRED_NATIVE_DIALOG_HOST_OPERATIONS"));
        assert!(linux_rs.contains("REQUIRED_NATIVE_SHELL_OPEN_HOST_OPERATIONS"));
        assert!(linux_rs.contains("REQUIRED_NATIVE_WINDOW_IDENTITY_HOST_OPERATIONS"));
        assert!(linux_rs.contains("REQUIRED_NATIVE_PASTE_TARGET_HOST_OPERATIONS"));
        assert!(linux_rs.contains("REQUIRED_NATIVE_TEXT_CARET_HOST_OPERATIONS"));
        assert!(linux_rs.contains("REQUIRED_NATIVE_FILE_DIALOG_HOST_OPERATIONS"));
        assert!(linux_rs.contains("REQUIRED_NATIVE_TEXT_INPUT_DIALOG_HOST_OPERATIONS"));
        assert!(linux_rs.contains("REQUIRED_NATIVE_EDIT_TEXT_DIALOG_HOST_OPERATIONS"));
        assert!(linux_rs.contains("REQUIRED_NATIVE_MAIL_MERGE_WINDOW_HOST_OPERATIONS"));
        assert!(linux_rs.contains("REQUIRED_RENDERER_HOST_OPERATIONS"));
        assert!(linux_rs.contains("REQUIRED_NATIVE_MAIN_WINDOW_HOST_OPERATIONS"));
        assert!(linux_rs.contains("REQUIRED_NATIVE_MAIN_SEARCH_CONTROL_HOST_OPERATIONS"));
        assert!(linux_rs.contains("REQUIRED_NATIVE_SETTINGS_WINDOW_HOST_OPERATIONS"));
        assert!(linux_rs.contains("REQUIRED_SETTINGS_CONTROL_HOST_OPERATIONS"));
        assert!(linux_rs.contains("REQUIRED_NATIVE_SETTINGS_DROPDOWN_HOST_OPERATIONS"));
        assert!(linux_rs.contains("SHARED_NON_HOST_UI_PROTOCOLS"));
        assert!(linux_rs.contains("Gtk4Libadwaita"));
    }
}

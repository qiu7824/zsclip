use crate::app_core::{
    NativeHostDialogAction, NativeHostRowAction, NativeHostSearchTextAction,
    NativeHostSettingsAction, NativeHostSettingsControlAction, NativeHostSettingsPlatformAction,
    NativeHostStatusMenuAction, NativeHostUiAction, NativeHostVvPasteExecution,
    NativeHostVvTriggerInput, NativeHostVvTriggerTransition, ProductAdapterAsyncBridgeResult,
    ProductAdapterCommandResult,
};
use crate::macos_app::MacosHostContractSummary;

#[cfg(target_os = "macos")]
mod appkit {
    use crate::app_core::native_content_preferences::{NativeContentPreferences,NATIVE_RENAME_PHRASE_COMMAND_ID};
    use std::{
        cell::{Cell, OnceCell, RefCell},
        ffi::c_void,
        fmt,
        ptr::{self, NonNull},
        time::{SystemTime, UNIX_EPOCH},
    };

    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2::runtime::{AnyClass, AnyObject, Bool, ProtocolObject, Sel};
    use objc2::{define_class, msg_send, sel, AnyThread, DefinedClass, MainThreadOnly, Message};
    use objc2_app_kit::{
        NSAccessibility, NSAlert, NSAlertFirstButtonReturn, NSAlertSecondButtonReturn,
        NSAlertStyle, NSAppearanceNameDarkAqua, NSApplication, NSApplicationActivationPolicy,
        NSApplicationDelegate, NSAutoresizingMaskOptions, NSBackingStoreType, NSBorderType,
        NSButton, NSButtonType, NSColor, NSControlStateValueOff, NSControlStateValueOn,
        NSControlTextEditingDelegate, NSEvent, NSEventMask, NSEventModifierFlags, NSEventType,
        NSFloatingWindowLevel, NSFont, NSImage, NSImageScaling, NSImageView, NSLineBreakMode,
        NSMenu, NSMenuItem, NSPanel, NSPopUpButton, NSRunningApplication, NSScrollView, NSSearchField, NSStatusBar,
        NSStatusBarButton, NSStatusItem, NSTabView, NSTabViewItem, NSTabViewType, NSTableColumn,
        NSTableView, NSTableViewDataSource, NSTableViewDelegate,
        NSTableViewSelectionHighlightStyle, NSTableViewStyle, NSTextAlignment, NSTextField,
        NSTextView, NSVariableStatusItemLength, NSView, NSVisualEffectBlendingMode,
        NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView, NSWindow,
        NSWindowDelegate, NSWindowStyleMask, NSWindowTitleVisibility, NSWorkspace, NSApplicationActivationOptions,
    };
    use objc2_core_foundation::{
        kCFRunLoopCommonModes, CFMachPort, CFRetained, CFRunLoopAddSource, CFRunLoopGetCurrent,
        CFRunLoopSource,
    };
    use objc2_core_graphics::{
        CGEvent, CGEventField, CGEventFlags, CGEventTapLocation, CGEventTapOptions,
        CGEventTapPlacement, CGEventTapProxy, CGEventType, CGPreflightPostEventAccess,
    };
    use objc2_foundation::{
        ns_string, MainThreadMarker, NSData, NSIndexSet, NSInteger, NSNotification, NSObject,
        NSObjectProtocol, NSPoint, NSPointInRect, NSRect, NSSize, NSString, NSUInteger,
    };

    use crate::app_core::{
        clip_kind_filter_options_for_tab, main_group_filter_selection_for_id,
        main_row_group_selection_for_id, menu_ids,
        native_host_clip_row_presentation_for_projection, native_host_clip_row_specs,
        native_host_dialog_button_specs, native_host_edit_text_button_specs,
        native_host_edit_text_close_plan, native_host_edit_text_plan_for_item,
        native_host_full_row_popup_menu_entries_for_groups,
        native_host_group_filter_label_for_groups,
        native_host_group_filter_popup_menu_entries_for_groups_kind_filter,
        native_host_main_tool_button_specs, native_host_projected_clip_row_title,
        native_host_reconciled_selected_item_id, native_host_row_action_button_specs,
        native_host_row_popup_menu_input_for_projection, native_host_search_input_specs,
        native_host_settings_action_button_specs, native_host_settings_dropdown_specs,
        native_host_settings_group_button_specs, native_host_settings_page_tab_specs,
        native_host_settings_platform_button_specs, native_host_settings_section_label,
        native_host_settings_toggle_specs, native_host_source_tab_for_category,
        native_host_status_menu_item_specs, native_host_vv_popup_render_plan_for_projection,
        native_popup_menu_command_macos_key_equivalent,
        native_popup_menu_command_macos_symbol_name, ClipKind, ClipKindFilter, HostComponent,
        MainGroupFilterSelection, MainRowGroupSelection, MainVvPopupTextRole,
        NativeButtonStyleRole, NativeClipRowSpec, NativeComponentAction,
        NativeComponentInstanceSpec, NativeComponentSpec, NativeDialogResponse, NativeDropdownSpec,
        NativeHostClipKindIcon, NativeHostClipListItemProjection, NativeHostClipRowPresentation,
        NativeHostDialogAction, NativeHostEditTextAction, NativeHostEditTextPlan,
        NativeHostMainToolAction, NativeHostRowAction, NativeHostSearchTextAction,
        NativeHostSettingsAction, NativeHostSettingsControlAction, NativeHostSettingsGroupAction,
        NativeHostSettingsPlatformAction, NativeHostStatusMenuAction, NativeHostUiAction,
        NativeHostVvTriggerAction, NativeHostVvTriggerInput, NativeHostVvTriggerKey,
        NativeHostVvTriggerTransition, NativeMenuItemSpec, NativePopupMenuEntry,
        NativeSettingsPageTabKind, ProductAdapterCommandResult, SettingsControlRole,
        NATIVE_HOST_SOURCE_TABS, REQUIRED_NATIVE_HOST_STATUS_MENU_ACTIONS,
    };
    use crate::macos_app::MacosHostContractSummary;
    use crate::zsui::{HostCapabilities, Window};

    fn appkit_tr(source: &'static str, fallback_en: &'static str) -> &'static str {
        crate::i18n::tr(source, fallback_en)
    }

    fn appkit_localized_label(label: &str) -> String {
        match label {
            "Search" => appkit_tr("搜索", "Search").to_string(),
            "Settings" => appkit_tr("设置", "Settings").to_string(),
            "Hide" => appkit_tr("隐藏", "Hide").to_string(),
            "Close" => appkit_tr("关闭", "Close").to_string(),
            "Row Menu" => appkit_tr("行菜单", "Row Menu").to_string(),
            "Group Filter" => appkit_tr("分组", "Group Filter").to_string(),
            "VV Popup" => appkit_tr("VV 粘贴", "VV Popup").to_string(),
            "VV Trigger" => appkit_tr("VV 触发", "VV Trigger").to_string(),
            "Show ZSClip" => appkit_tr("显示剪贴板", "Show ZSClip").to_string(),
            "Toggle Capture" => appkit_tr("启用/暂停捕获", "Toggle Capture").to_string(),
            "Toggle LAN Sync" => appkit_tr("启用/暂停局域网同步", "Toggle LAN Sync").to_string(),
            "Exit" => appkit_tr("退出", "Exit").to_string(),
            "Paste" => appkit_tr("粘贴", "Paste").to_string(),
            "Copy" => appkit_tr("复制", "Copy").to_string(),
            "Pin" => appkit_tr("置顶", "Pin").to_string(),
            "To Phrase" => appkit_tr("转为常用短语", "To Phrase").to_string(),
            "Delete" => appkit_tr("删除", "Delete").to_string(),
            "Edit" => appkit_tr("编辑", "Edit").to_string(),
            "Open Path" => appkit_tr("打开路径", "Open Path").to_string(),
            "Open Folder" => appkit_tr("打开所在文件夹", "Open Folder").to_string(),
            "Copy Path" => appkit_tr("复制路径", "Copy Path").to_string(),
            "Translate" => appkit_tr("翻译", "Translate").to_string(),
            "Save" => appkit_tr("保存", "Save").to_string(),
            "Cancel" => appkit_tr("取消", "Cancel").to_string(),
            "Discard" => appkit_tr("不保存", "Discard").to_string(),
            "Open Config" => appkit_tr("打开配置", "Open Config").to_string(),
            "Auto Start" => appkit_tr("开机自启", "Auto Start").to_string(),
            "Capture" => appkit_tr("剪贴板捕获", "Capture").to_string(),
            "LAN Sync" => appkit_tr("局域网同步", "LAN Sync").to_string(),
            "Cloud Sync" => appkit_tr("云同步", "Cloud Sync").to_string(),
            "Sync Mode" => appkit_tr("同步模式", "Sync Mode").to_string(),
            "All" => appkit_tr("全部", "All").to_string(),
            "(No groups)" => appkit_tr("（暂无分组）", "(No groups)").to_string(),
            "PIN" => appkit_tr("置顶", "PIN").to_string(),
            _ => crate::i18n::translate(label).into_owned(),
        }
    }

    #[derive(Default)]
    struct AppDelegateIvars {
        content_preferences: Cell<NativeContentPreferences>,
        search_service: OnceCell<crate::native_search::NativeSearchService>,
        search_due: Cell<Option<std::time::Instant>>,
        search_pending: Cell<bool>,
        search_generation: Cell<u64>,
        search_page: Cell<usize>,
        search_has_more: Cell<bool>,
        previous_page_button: OnceCell<Retained<NSButton>>,
        next_page_button: OnceCell<Retained<NSButton>>,
        page_label: OnceCell<Retained<NSTextField>>,
        screenshot_scene: RefCell<Option<String>>,
        auto_smoke_row_item_id: Cell<Option<i64>>,
        image_export_result: RefCell<Option<std::sync::mpsc::Receiver<Result<(),String>>>>,
        last_external_pid: Cell<i32>,
        pending_row_paste: RefCell<Option<(i32,u64,u32,String,std::time::Instant)>>,
        window: OnceCell<Retained<NSWindow>>,
        settings_window: OnceCell<Retained<NSWindow>>,
        status_item: OnceCell<Retained<NSStatusItem>>,
        status_menu: OnceCell<Retained<NSMenu>>,
        status_menu_items: RefCell<Vec<NativeStatusMenuItemBinding>>,
        clip_scroll_view: OnceCell<Retained<NSScrollView>>,
        clip_table_view: OnceCell<Retained<NSTableView>>,
        clip_table_column: OnceCell<Retained<NSTableColumn>>,
        clip_list_document_view: OnceCell<Retained<NSView>>,
        vv_event_monitor: OnceCell<Retained<AnyObject>>,
        vv_global_event_monitor: OnceCell<Retained<AnyObject>>,
        vv_cg_event_tap: OnceCell<CFRetained<CFMachPort>>,
        vv_cg_event_tap_source: OnceCell<CFRetained<CFRunLoopSource>>,
        vv_cg_event_tap_delegate: OnceCell<Retained<AnyObject>>,
        row_context_event_monitor: OnceCell<Retained<AnyObject>>,
        vv_popup_window: OnceCell<Retained<NSWindow>>,
        vv_presentation: RefCell<Option<MacosVvPresentation>>,
        vv_input: RefCell<crate::app_core::vv_session::VvInputSession>,
        vv_session_serial: Cell<u64>,
        vv_preview_text: RefCell<Option<Retained<NSTextView>>>,
        vv_preview_scroll: RefCell<Option<Retained<NSScrollView>>>,
        vv_status_label: RefCell<Option<Retained<NSTextField>>>,
        vv_candidate_buttons: RefCell<Vec<Retained<NSButton>>>,
        vv_preview_selected: Cell<usize>,
        vv_preview_hover: Cell<Option<usize>>,
        vv_preview_due: Cell<Option<(std::time::Instant, usize)>>,
        vv_preview_request: Cell<u64>,
        vv_preview_result: RefCell<Option<std::sync::mpsc::Receiver<MacosVvPreviewResult>>>,
        vv_screenshot_waiting: Cell<bool>,
        vv_delivery_smoke_phase: Cell<u8>,
        vv_delivery_smoke_pid: Cell<i32>,
        vv_delivery_smoke_item_id: Cell<i64>,
        vv_delivery_smoke_deadline: Cell<Option<std::time::Instant>>,
        edit_window: OnceCell<Retained<NSWindow>>,
        edit_text_view: OnceCell<Retained<NSTextView>>,
        edit_title_field: OnceCell<Retained<NSTextField>>,
        edit_initial_title: RefCell<String>,
        edit_is_phrase: Cell<bool>,
        edit_save_as_phrase: Cell<bool>,
        edit_initial_text: RefCell<String>,
        edit_item_id: Cell<i64>,
        edit_data_generation: Cell<u64>,
        selected_item_id: Cell<i64>,
        current_group_filter: Cell<i64>,
        current_source_category: Cell<i64>,
        current_kind_filter: Cell<ClipKindFilter>,
        last_clipboard_sequence: Cell<u32>,
        settings_group_category: Cell<i64>,
        selected_settings_group_id: Cell<i64>,
        search_field: OnceCell<Retained<NSSearchField>>,
        settings_route_label: OnceCell<Retained<NSTextField>>,
        settings_tabs: OnceCell<Retained<NSTabView>>,
        settings_page_scrollers: OnceCell<Vec<Retained<NSScrollView>>>,
        settings_save_button: OnceCell<Retained<NSButton>>,
        settings_group_list_view: OnceCell<Retained<NSView>>,
        settings_group_list_scroll: OnceCell<Retained<NSScrollView>>,
        settings_group_name_field: OnceCell<Retained<NSTextField>>,
        settings_native_text_fields: RefCell<Vec<NativeSettingsTextFieldBinding>>,
        settings_native_toggle_buttons: RefCell<Vec<NativeSettingsToggleButtonBinding>>,
        settings_native_dropdown_buttons: RefCell<Vec<NativeSettingsDropdownButtonBinding>>,
        settings_native_route_buttons: RefCell<Vec<NativeSettingsRouteButtonBinding>>,
        settings_sound_path: RefCell<Option<String>>,
        settings_sound_file_button: OnceCell<Retained<NSButton>>,
        settings_sound_preview_button: OnceCell<Retained<NSButton>>,
        sound_preview_result: RefCell<Option<std::sync::mpsc::Receiver<Result<crate::native_feedback::NativeFeedbackPlayback,String>>>>,
        settings_group_rows: RefCell<Vec<Retained<NSButton>>>,
        clip_items: RefCell<Vec<NativeHostClipListItemProjection>>,
        clip_table_items: RefCell<Vec<NativeHostClipListItemProjection>>,
        group_filter_button: OnceCell<Retained<NSButton>>,
        source_tab_buttons: OnceCell<Vec<Retained<NSButton>>>,
    }

    impl fmt::Debug for AppDelegateIvars {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("AppDelegateIvars").finish_non_exhaustive()
        }
    }

    #[derive(Clone)]
    struct MacosVvPresentation {
        serial: u64,
        target_pid: i32,
        snapshot: crate::native_vv::NativeVvSnapshot,
    }

    struct MacosVvPreviewResult {
        serial: u64,
        request: u64,
        index: usize,
        body: Result<String, String>,
    }

    define_class!(
        #[unsafe(super = NSPanel)]
        #[thread_kind = MainThreadOnly]
        #[name = "ZSClipVvNonactivatingPanel"]
        struct VvNonactivatingPanel;
        impl VvNonactivatingPanel {
            #[unsafe(method(canBecomeKeyWindow))]
            fn can_become_key_window(&self) -> bool { false }
            #[unsafe(method(canBecomeMainWindow))]
            fn can_become_main_window(&self) -> bool { false }
        }
    );

    #[derive(Clone)]
    struct NativeStatusMenuItemBinding {
        action: NativeHostStatusMenuAction,
        item: Retained<NSMenuItem>,
    }

    #[derive(Clone)]
    struct NativeSettingsTextFieldBinding {
        control_key: &'static str,
        initial_value: String,
        field: Retained<NSTextField>,
    }

    #[derive(Clone)]
    struct NativeSettingsToggleButtonBinding {
        control_key: &'static str,
        initial_value: bool,
        button: Retained<NSButton>,
    }

    #[derive(Clone)]
    struct NativeSettingsDropdownButtonBinding {
        control_key: &'static str,
        initial_value: String,
        option_values: Vec<String>,
        button: Retained<NSPopUpButton>,
    }

    #[derive(Clone, Debug)]
    struct NativeSettingsRouteButtonBinding {
        tag: isize,
        route_name: &'static str,
        action_name: &'static str,
    }

    impl fmt::Debug for NativeSettingsTextFieldBinding {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("NativeSettingsTextFieldBinding")
                .field("control_key", &self.control_key)
                .finish_non_exhaustive()
        }
    }

    fn native_settings_dropdown_options_for_host(
        control: &crate::settings_model::SettingsNativeControlSummary,
        settings_json: &serde_json::Value,
    ) -> Option<crate::settings_model::SettingsNativeDropdownOptions> {
        crate::settings_model::settings_native_dropdown_options(control, settings_json).or_else(
            || {
                let category =
                    crate::settings_model::settings_native_vv_source_tab(settings_json) as i64;
                let groups = crate::db_runtime::native_clip_groups(category).unwrap_or_default();
                crate::settings_model::settings_native_vv_group_dropdown_options(
                    control,
                    settings_json,
                    groups.iter().map(|group| (group.id, group.name.as_str())),
                )
            },
        )
    }

    fn appkit_main_window_capabilities() -> HostCapabilities {
        HostCapabilities::macos_native_window_host()
    }

    fn appkit_main_window_spec() -> Window {
        let requested = Window::new("ZSClip")
            .size(640, 420)
            .min_size(420, 300)
            .resizable(true)
            .decorations(true)
            .always_on_top(true);
        requested
            .resolve_for(&appkit_main_window_capabilities())
            .effective
    }

    fn appkit_window_style_mask(spec: &Window) -> NSWindowStyleMask {
        let mut style = if spec.decorations {
            NSWindowStyleMask::Titled
                | NSWindowStyleMask::Closable
                | NSWindowStyleMask::Miniaturizable
                | NSWindowStyleMask::FullSizeContentView
        } else {
            NSWindowStyleMask::Borderless
        };
        if spec.resizable {
            style |= NSWindowStyleMask::Resizable;
        }
        style
    }

    define_class!(
        #[unsafe(super = NSObject)]
        #[thread_kind = MainThreadOnly]
        #[ivars = AppDelegateIvars]
        struct Delegate;

        impl Delegate {
            #[unsafe(method(zsclipToggleSearch:))]
            fn zsclip_toggle_search(&self, _sender: &AnyObject) {
                self.perform_native_host_action(NativeHostUiAction::ToggleSearch);
            }

            #[unsafe(method(zsclipOpenSettings:))]
            fn zsclip_open_settings(&self, _sender: &AnyObject) {
                self.perform_native_host_action(NativeHostUiAction::OpenSettings);
            }

            #[unsafe(method(zsclipHideWindow:))]
            fn zsclip_hide_window(&self, _sender: &AnyObject) {
                self.perform_native_host_action(NativeHostUiAction::HideWindow);
            }

            #[unsafe(method(zsclipCloseWindow:))]
            fn zsclip_close_window(&self, _sender: &AnyObject) {
                self.perform_native_host_action(NativeHostUiAction::CloseWindow);
            }

            #[unsafe(method(zsclipSelectRow:))]
            fn zsclip_select_row(&self, sender: &AnyObject) {
                let item_id: isize = unsafe { msg_send![sender, tag] };
                self.select_native_row(item_id as i64);
            }

            #[unsafe(method(zsclipActivateClipTableRow:))]
            fn zsclip_activate_clip_table_row(&self, _sender: &AnyObject) {
                self.perform_native_row_action(NativeHostRowAction::Paste);
            }

            #[unsafe(method(zsclipSelectSourceTab:))]
            fn zsclip_select_source_tab(&self, sender: &AnyObject) {
                let item_id: isize = unsafe { msg_send![sender, tag] };
                self.select_native_source_category(item_id as i64);
            }

            #[unsafe(method(zsclipSelectSettingsGroup:))]
            fn zsclip_select_settings_group(&self, sender: &AnyObject) {
                let group_id: isize = unsafe { msg_send![sender, tag] };
                self.select_settings_group(group_id as i64);
            }

            #[unsafe(method(zsclipShowRecordGroups:))]
            fn zsclip_show_record_groups(&self, _sender: &AnyObject) {
                self.ivars().settings_group_category.set(0);
                self.ivars().selected_settings_group_id.set(0);
                self.refresh_settings_group_rows();
            }

            #[unsafe(method(zsclipShowPhraseGroups:))]
            fn zsclip_show_phrase_groups(&self, _sender: &AnyObject) {
                self.ivars().settings_group_category.set(1);
                self.ivars().selected_settings_group_id.set(0);
                self.refresh_settings_group_rows();
            }

            #[unsafe(method(zsclipAddSettingsGroup:))]
            fn zsclip_add_settings_group(&self, _sender: &AnyObject) {
                self.perform_settings_group_create();
            }

            #[unsafe(method(zsclipRenameSettingsGroup:))]
            fn zsclip_rename_settings_group(&self, _sender: &AnyObject) {
                self.perform_settings_group_rename();
            }

            #[unsafe(method(zsclipDeleteSettingsGroup:))]
            fn zsclip_delete_settings_group(&self, _sender: &AnyObject) {
                self.perform_settings_group_delete();
            }

            #[unsafe(method(zsclipMoveSettingsGroupUp:))]
            fn zsclip_move_settings_group_up(&self, _sender: &AnyObject) {
                self.perform_settings_group_move(-1);
            }

            #[unsafe(method(zsclipMoveSettingsGroupDown:))]
            fn zsclip_move_settings_group_down(&self, _sender: &AnyObject) {
                self.perform_settings_group_move(1);
            }

            #[unsafe(method(zsclipStatusToggleWindow:))]
            fn zsclip_status_toggle_window(&self, _sender: &AnyObject) {
                self.perform_native_status_menu_action(NativeHostStatusMenuAction::ToggleWindow);
            }

            #[unsafe(method(zsclipStatusToggleClipboardCapture:))]
            fn zsclip_status_toggle_clipboard_capture(&self, _sender: &AnyObject) {
                self.perform_native_status_menu_action(
                    NativeHostStatusMenuAction::ToggleClipboardCapture,
                );
            }

            #[cfg(feature = "lan-sync")]
            #[unsafe(method(zsclipStatusToggleLanSync:))]
            fn zsclip_status_toggle_lan_sync(&self, _sender: &AnyObject) {
                self.perform_native_status_menu_action(NativeHostStatusMenuAction::ToggleLanSync);
            }

            #[unsafe(method(zsclipStatusExit:))]
            fn zsclip_status_exit(&self, _sender: &AnyObject) {
                self.perform_native_status_menu_action(NativeHostStatusMenuAction::Exit);
            }

            #[unsafe(method(zsclipSaveSettings:))]
            fn zsclip_save_settings(&self, _sender: &AnyObject) {
                self.perform_native_settings_action(NativeHostSettingsAction::Save);
            }

            #[unsafe(method(zsclipCloseSettings:))]
            fn zsclip_close_settings(&self, _sender: &AnyObject) {
                self.perform_native_settings_action(NativeHostSettingsAction::Close);
            }

            #[unsafe(method(zsclipOpenSettingsConfig:))]
            fn zsclip_open_settings_config(&self, _sender: &AnyObject) {
                self.perform_native_settings_action(NativeHostSettingsAction::OpenConfig);
            }

            #[unsafe(method(zsclipSettingsNativeRouteAction:))]
            fn zsclip_settings_native_route_action(&self, sender: &AnyObject) {
                let tag: isize = unsafe { msg_send![sender, tag] };
                self.perform_native_settings_route_action(tag);
            }

            #[unsafe(method(zsclipSettingsDraftChanged:))]
            fn zsclip_settings_draft_changed(&self, _sender: &AnyObject) {
                self.refresh_settings_dependencies();
                if let Some(label) = self.ivars().settings_route_label.get() {
                    label.setStringValue(&NSString::from_str(appkit_tr("有未保存的更改", "Unsaved changes")));
                }
            }

            #[unsafe(method(zsclipToggleClipboardCapture:))]
            fn zsclip_toggle_clipboard_capture(&self, _sender: &AnyObject) {
                self.perform_native_settings_control_action(
                    NativeHostSettingsControlAction::ToggleClipboardCapture,
                );
            }

            #[unsafe(method(zsclipToggleAutostart:))]
            fn zsclip_toggle_autostart(&self, _sender: &AnyObject) {
                self.perform_native_settings_control_action(
                    NativeHostSettingsControlAction::ToggleAutostart,
                );
            }

            #[cfg(feature = "lan-sync")]
            #[unsafe(method(zsclipToggleLanSync:))]
            fn zsclip_toggle_lan_sync(&self, _sender: &AnyObject) {
                self.perform_native_settings_control_action(
                    NativeHostSettingsControlAction::ToggleLanSync,
                );
            }

            #[cfg(feature = "cloud-sync")]
            #[unsafe(method(zsclipToggleCloudSync:))]
            fn zsclip_toggle_cloud_sync(&self, _sender: &AnyObject) {
                self.perform_native_settings_control_action(
                    NativeHostSettingsControlAction::ToggleCloudSync,
                );
            }

            #[cfg(any(feature = "cloud-sync", feature = "lan-sync"))]
            #[unsafe(method(zsclipOpenSyncModeDropdown:))]
            fn zsclip_open_sync_mode_dropdown(&self, _sender: &AnyObject) {
                self.perform_native_settings_control_action(
                    NativeHostSettingsControlAction::OpenSyncModeDropdown,
                );
            }

            #[unsafe(method(zsclipOpenSourceRepository:))]
            fn zsclip_open_source_repository(&self, _sender: &AnyObject) {
                self.perform_native_settings_platform_action(
                    NativeHostSettingsPlatformAction::OpenSourceRepository,
                );
            }

            #[unsafe(method(zsclipCheckForUpdates:))]
            fn zsclip_check_for_updates(&self, _sender: &AnyObject) {
                self.perform_native_settings_platform_action(
                    NativeHostSettingsPlatformAction::CheckForUpdates,
                );
            }

            #[unsafe(method(zsclipOpenWpsTaskpaneDocs:))]
            fn zsclip_open_wps_taskpane_docs(&self, _sender: &AnyObject) {
                self.perform_native_settings_platform_action(
                    NativeHostSettingsPlatformAction::OpenWpsTaskpaneDocs,
                );
            }

            #[unsafe(method(zsclipDisableSystemClipboardHistory:))]
            fn zsclip_disable_system_clipboard_history(&self, _sender: &AnyObject) {
                self.perform_native_settings_platform_action(
                    NativeHostSettingsPlatformAction::DisableSystemClipboardHistory,
                );
            }

            #[unsafe(method(zsclipEnableSystemClipboardHistory:))]
            fn zsclip_enable_system_clipboard_history(&self, _sender: &AnyObject) {
                self.perform_native_settings_platform_action(
                    NativeHostSettingsPlatformAction::EnableSystemClipboardHistory,
                );
            }

            #[unsafe(method(zsclipRestartSystemShell:))]
            fn zsclip_restart_system_shell(&self, _sender: &AnyObject) {
                self.perform_native_settings_platform_action(
                    NativeHostSettingsPlatformAction::RestartSystemShell,
                );
            }

            #[unsafe(method(zsclipShowInfoDialog:))]
            fn zsclip_show_info_dialog(&self, _sender: &AnyObject) {
                self.perform_native_dialog_action(NativeHostDialogAction::ShowInfoMessage);
            }

            #[unsafe(method(zsclipShowConfirmDialog:))]
            fn zsclip_show_confirm_dialog(&self, _sender: &AnyObject) {
                self.perform_native_dialog_action(NativeHostDialogAction::ConfirmQuestion);
            }

            #[unsafe(method(zsclipRowPaste:))]
            fn zsclip_row_paste(&self, _sender: &AnyObject) {
                self.perform_native_row_action(NativeHostRowAction::Paste);
            }

            #[unsafe(method(zsclipRowCopy:))]
            fn zsclip_row_copy(&self, _sender: &AnyObject) {
                self.perform_native_row_action(NativeHostRowAction::Copy);
            }

            #[unsafe(method(zsclipRowPin:))]
            fn zsclip_row_pin(&self, _sender: &AnyObject) {
                self.perform_native_row_action(NativeHostRowAction::Pin);
            }

            #[unsafe(method(zsclipRowToPhrase:))]
            fn zsclip_row_to_phrase(&self, _sender: &AnyObject) {
                self.perform_native_row_action(NativeHostRowAction::ToPhrase);
            }

            #[unsafe(method(zsclipRowDelete:))]
            fn zsclip_row_delete(&self, _sender: &AnyObject) {
                self.perform_native_row_action(NativeHostRowAction::Delete);
            }

            #[unsafe(method(zsclipRowEdit:))]
            fn zsclip_row_edit(&self, _sender: &AnyObject) {
                self.perform_native_row_action(NativeHostRowAction::Edit);
            }

            #[unsafe(method(zsclipRowOpenPath:))]
            fn zsclip_row_open_path(&self, _sender: &AnyObject) {
                self.perform_native_row_action(NativeHostRowAction::OpenPath);
            }

            #[unsafe(method(zsclipRowOpenFolder:))]
            fn zsclip_row_open_folder(&self, _sender: &AnyObject) {
                self.perform_native_row_action(NativeHostRowAction::OpenFolder);
            }

            #[unsafe(method(zsclipRowCopyPath:))]
            fn zsclip_row_copy_path(&self, _sender: &AnyObject) {
                self.perform_native_row_action(NativeHostRowAction::CopyPath);
            }

            #[cfg(feature = "ai-actions")]
            #[unsafe(method(zsclipRowTextTranslate:))]
            fn zsclip_row_text_translate(&self, _sender: &AnyObject) {
                self.perform_native_row_action(NativeHostRowAction::TextTranslate);
            }

            #[unsafe(method(zsclipShowRowPopupMenu:))]
            fn zsclip_show_row_popup_menu(&self, _sender: &AnyObject) {
                self.present_native_row_popup_menu();
            }

            #[unsafe(method(zsclipShowGroupFilterPopupMenu:))]
            fn zsclip_show_group_filter_popup_menu(&self, _sender: &AnyObject) {
                self.present_native_group_filter_popup_menu();
            }

            #[unsafe(method(zsclipShowVvPopup:))]
            fn zsclip_show_vv_popup(&self, _sender: &AnyObject) {
                self.present_native_vv_popup();
            }

            #[unsafe(method(zsclipTriggerVvDemo:))]
            fn zsclip_trigger_vv_demo(&self, _sender: &AnyObject) {
                self.perform_native_vv_trigger_demo();
            }

            #[unsafe(method(zsclipVvSelect:))]
            fn zsclip_vv_select(&self, sender: &NSButton) {
                self.perform_native_vv_select(sender.tag() as usize);
            }

            #[unsafe(method(zsclipPopupRowCommand:))]
            fn zsclip_popup_row_command(&self, sender: &NSMenuItem) {
                let menu_id = sender.tag() as usize;
                self.perform_native_popup_menu_command(menu_id);
            }

            #[unsafe(method(zsclipSaveEditText:))]
            fn zsclip_save_edit_text(&self, _sender: &AnyObject) {
                self.perform_native_edit_save();
            }

            #[unsafe(method(zsclipCancelEditText:))]
            fn zsclip_cancel_edit_text(&self, _sender: &AnyObject) {
                self.perform_native_edit_cancel();
            }

            #[unsafe(method(zsclipSearchTextChanged:))]
            fn zsclip_search_text_changed(&self, sender: &AnyObject) {
                let Some(search_field) = sender.downcast_ref::<NSSearchField>() else {
                    return;
                };
                self.perform_native_search_text_action(search_field.stringValue().to_string());
            }

            #[unsafe(method(zsclipClipboardPoll:))]
            fn zsclip_clipboard_poll(&self, _sender: &AnyObject) {
                self.poll_native_clipboard_capture();
            }

            #[unsafe(method(zsclipSearchPoll:))]
            fn zsclip_search_poll(&self, _sender: &AnyObject) {self.poll_native_search();}

            #[unsafe(method(zsclipSoundPreview:))]
            fn zsclip_sound_preview(&self,_sender:&AnyObject) {self.preview_native_sound();}

            #[unsafe(method(zsclipPreviousPage:))]
            fn zsclip_previous_page(&self,_sender:&AnyObject) {
                if !self.ivars().search_pending.get() && self.ivars().search_page.get()>0 {self.request_native_search_page(self.ivars().search_page.get()-1);}
            }

            #[unsafe(method(zsclipNextPage:))]
            fn zsclip_next_page(&self,_sender:&AnyObject) {
                if !self.ivars().search_pending.get() && self.ivars().search_has_more.get() {self.request_native_search_page(self.ivars().search_page.get()+1);}
            }
        }

        unsafe impl NSObjectProtocol for Delegate {}

        unsafe impl NSApplicationDelegate for Delegate {
            #[unsafe(method(applicationShouldTerminateAfterLastWindowClosed:))]
            fn application_should_terminate_after_last_window_closed(
                &self,
                _sender: &NSApplication,
            ) -> Bool {
                false.into()
            }

            #[unsafe(method(applicationDidFinishLaunching:))]
            fn did_finish_launching(&self, notification: &NSNotification) {
                let mtm = self.mtm();
                let app = unsafe { notification.object() }
                    .unwrap()
                    .downcast::<NSApplication>()
                    .unwrap();
                if let Some(pid)=Self::appkit_frontmost_pid().filter(|pid|*pid!=std::process::id() as i32) {self.ivars().last_external_pid.set(pid);}
                let text_field = unsafe {
                    let text_field = NSTextField::labelWithString(ns_string!("ZSClip"), mtm);
                    text_field.setFrame(NSRect::new(
                        NSPoint::new(16.0, 240.0),
                        NSSize::new(608.0, 64.0),
                    ));
                    text_field.setTextColor(Some(&NSColor::labelColor()));
                    text_field.setAlignment(NSTextAlignment::Center);
                    text_field.setFont(Some(&NSFont::systemFontOfSize(32.0)));
                    text_field.setAutoresizingMask(
                        NSAutoresizingMaskOptions::ViewWidthSizable
                            | NSAutoresizingMaskOptions::ViewMinYMargin,
                    );
                    text_field
                };
                appkit_set_accessibility_label::<NSTextField>(
                    text_field.as_ref(),
                    "ZSClip app title",
                );
                let target: &AnyObject = self.as_ref();
                let search_spec = native_host_search_input_specs()[0];
                let search_bounds = search_spec.bounds();
                let search_field = NSSearchField::new(mtm);
                search_field.setFrame(NSRect::new(
                    NSPoint::new(search_bounds.left as f64, search_bounds.top as f64),
                    NSSize::new(search_bounds.width() as f64, search_bounds.height() as f64),
                ));
                unsafe { search_field.setTarget(Some(target)) };
                unsafe { search_field.setAction(Some(sel!(zsclipSearchTextChanged:))) };
                search_field.setPlaceholderString(Some(&NSString::from_str(appkit_tr(
                    "搜索剪贴板",
                    "Search clipboard",
                ))));
                search_field.setHidden(true);
                appkit_set_view_alpha(search_field.as_ref(), 0.0);
                search_field.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable);
                appkit_set_accessibility_label::<NSSearchField>(
                    search_field.as_ref(),
                    search_spec.label(),
                );
                let clip_items = crate::macos_app::macos_native_host_projected_clip_items();
                let preferences=NativeContentPreferences::from_json(&crate::macos_app::macos_native_settings_json_snapshot());
                self.ivars().content_preferences.set(preferences);
                appkit_apply_content_theme(mtm);
                search_field.setFont(Some(&NSFont::systemFontOfSize(preferences.content_font_size as f64)));
                let clip_row_height = preferences.row_height();
                let clip_list_width = 608.0_f64;
                let clip_list_height = 300.0_f64;
                let clip_list_document_view = NSView::initWithFrame(
                    NSView::alloc(mtm),
                    NSRect::new(
                        NSPoint::new(0.0, 0.0),
                        NSSize::new(clip_list_width, clip_list_height),
                    ),
                );
                clip_list_document_view.setAutoresizesSubviews(true);
                let clip_table_view = NSTableView::initWithFrame(
                    NSTableView::alloc(mtm),
                    NSRect::new(
                        NSPoint::new(0.0, 0.0),
                        NSSize::new(clip_list_width, clip_list_height),
                    ),
                );
                let clip_table_column = NSTableColumn::new(mtm);
                clip_table_column.setWidth(clip_list_width);
                clip_table_column.setTitle(&NSString::from_str(appkit_tr("剪贴板", "Clipboard")));
                clip_table_view.addTableColumn(&clip_table_column);
                clip_table_view.setHeaderView(None);
                clip_table_view.setRowHeight(clip_row_height);
                clip_table_view.setIntercellSpacing(NSSize::new(0.0, if preferences.card_view_enabled {4.0}else{1.0}));
                clip_table_view.setUsesAlternatingRowBackgroundColors(!preferences.card_view_enabled);
                clip_table_view.setAllowsMultipleSelection(false);
                clip_table_view.setAllowsEmptySelection(false);
                clip_table_view
                    .setSelectionHighlightStyle(NSTableViewSelectionHighlightStyle::Regular);
                clip_table_view.setStyle(NSTableViewStyle::Plain);
                appkit_set_accessibility_label::<NSTableView>(
                    clip_table_view.as_ref(),
                    "Clipboard history list",
                );
                unsafe { clip_table_view.setTarget(Some(target)) };
                unsafe { clip_table_view.setDoubleAction(Some(sel!(zsclipActivateClipTableRow:))) };
                unsafe {
                    clip_table_view.setDataSource(Some(ProtocolObject::from_ref(self)));
                    clip_table_view.setDelegate(Some(ProtocolObject::from_ref(self)));
                }
                let clip_scroll_view = NSScrollView::initWithFrame(
                    NSScrollView::alloc(mtm),
                    NSRect::new(
                        NSPoint::new(16.0, 38.0),
                        NSSize::new(clip_list_width, 282.0),
                    ),
                );
                clip_scroll_view.setHasVerticalScroller(true);
                clip_scroll_view.setHasHorizontalScroller(false);
                clip_scroll_view.setAutohidesScrollers(true);
                clip_scroll_view.setDrawsBackground(false);
                clip_scroll_view.setAutoresizingMask(
                    NSAutoresizingMaskOptions::ViewWidthSizable
                        | NSAutoresizingMaskOptions::ViewHeightSizable,
                );
                clip_scroll_view.setDocumentView(Some(&clip_table_view));
                appkit_set_accessibility_label::<NSScrollView>(
                    clip_scroll_view.as_ref(),
                    "Clipboard history scroll area",
                );
                let tool_buttons: Vec<_> = native_host_main_tool_button_specs()
                    .into_iter()
                    .filter(|spec| spec.action == NativeHostMainToolAction::GroupFilter)
                    .map(|spec| {
                        let localized = appkit_localized_label(spec.label);
                        let title = NSString::from_str(&localized);
                        let button = unsafe {
                            NSButton::buttonWithTitle_target_action(
                                &title,
                                Some(target),
                                Some(appkit_main_tool_action_selector(spec.action)),
                                mtm,
                            )
                        };
                        button.setFrame(NSRect::new(
                            NSPoint::new(410.0, 326.0),
                            NSSize::new(96.0, 28.0),
                        ));
                        appkit_set_accessibility_label::<NSButton>(button.as_ref(), &localized);
                        button
                    })
                    .collect();
                for button in &tool_buttons {
                    button.setAutoresizingMask(NSAutoresizingMaskOptions::ViewMinYMargin);
                }
                let source_tab_buttons: Vec<_> = NATIVE_HOST_SOURCE_TABS
                    .iter()
                    .enumerate()
                    .map(|(index, tab)| {
                        let title =
                            NSString::from_str(appkit_tr(tab.label_source, tab.label_en));
                        let button = unsafe {
                            NSButton::buttonWithTitle_target_action(
                                &title,
                                Some(target),
                                Some(sel!(zsclipSelectSourceTab:)),
                                mtm,
                            )
                        };
                        button.setFrame(NSRect::new(
                            NSPoint::new(16.0 + index as f64 * 116.0, 326.0),
                            NSSize::new(108.0, 28.0),
                        ));
                        button.setButtonType(NSButtonType::PushOnPushOff);
                        button.setTag(tab.category as isize);
                        button.setState(if tab.category == 0 {
                            NSControlStateValueOn
                        } else {
                            NSControlStateValueOff
                        });
                        button.setAutoresizingMask(NSAutoresizingMaskOptions::ViewMinYMargin);
                        appkit_set_accessibility_label::<NSButton>(
                            button.as_ref(),
                            appkit_tr(tab.label_source, tab.label_en),
                        );
                        button
                    })
                    .collect();
                let window_spec = appkit_main_window_spec();
                let window = unsafe {
                    NSWindow::initWithContentRect_styleMask_backing_defer(
                        NSWindow::alloc(mtm),
                        NSRect::new(
                            NSPoint::new(0.0, 0.0),
                            NSSize::new(window_spec.width as f64, window_spec.height as f64),
                        ),
                        appkit_window_style_mask(&window_spec),
                        NSBackingStoreType::Buffered,
                        false,
                    )
                };
                unsafe { window.setReleasedWhenClosed(false) };
                let window_title = NSString::from_str(&window_spec.title);
                window.setTitle(&window_title);
                window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
                window.setTitlebarAppearsTransparent(true);
                window.setHidesOnDeactivate(false);
                if window_spec.always_on_top {
                    window.setLevel(NSFloatingWindowLevel);
                }
                unsafe {
                    let _: () = msg_send![&*window, setMovableByWindowBackground: true];
                }
                let view = NSVisualEffectView::initWithFrame(
                    NSVisualEffectView::alloc(mtm),
                    NSRect::new(
                        NSPoint::new(0.0, 0.0),
                        NSSize::new(window_spec.width as f64, window_spec.height as f64),
                    ),
                );
                view.setMaterial(NSVisualEffectMaterial::WindowBackground);
                view.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
                view.setState(NSVisualEffectState::FollowsWindowActiveState);
                view.setAutoresizingMask(
                    NSAutoresizingMaskOptions::ViewWidthSizable
                        | NSAutoresizingMaskOptions::ViewHeightSizable,
                );
                appkit_set_accessibility_label::<NSVisualEffectView>(
                    view.as_ref(),
                    "ZSClip main window content",
                );
                appkit_enable_rounded_layer(view.as_ref(), 12.0);
                window.setContentView(Some(&view));
                unsafe { view.addSubview(&text_field) };
                unsafe { view.addSubview(&search_field) };
                unsafe { view.addSubview(&clip_scroll_view) };
                for button in &source_tab_buttons {
                    unsafe { view.addSubview(button) };
                }
                for button in &tool_buttons {
                    unsafe { view.addSubview(button) };
                }
                window.center();
                appkit_position_window_near_cursor(&window);
                let appkit_scale_factor = window.backingScaleFactor();
                let appkit_dark_mode = appkit_is_dark_appearance(&app);
                eprintln!(
                    "ZSClip AppKit native window traits always_on_top={} scale_factor={} dark_mode={}",
                    window_spec.always_on_top,
                    appkit_scale_factor, appkit_dark_mode
                );
                if let (Some(min_width), Some(min_height)) =
                    (window_spec.min_width, window_spec.min_height)
                {
                    unsafe {
                        window.setContentMinSize(NSSize::new(min_width as f64, min_height as f64))
                    };
                }
                window.setDelegate(Some(ProtocolObject::from_ref(self)));
                window.makeKeyAndOrderFront(None);
                window.makeFirstResponder(Some(&clip_table_view));
                self.ivars().window.set(window).unwrap();
                self.ivars().search_field.set(search_field).unwrap();
                self.ivars()
                    .clip_scroll_view
                    .set(clip_scroll_view)
                    .unwrap();
                self.ivars()
                    .clip_table_view
                    .set(clip_table_view)
                    .unwrap();
                self.ivars()
                    .clip_table_column
                    .set(clip_table_column)
                    .unwrap();
                self.ivars()
                    .clip_list_document_view
                    .set(clip_list_document_view)
                    .unwrap();
                *self.ivars().clip_table_items.borrow_mut() = clip_items.clone();
                *self.ivars().clip_items.borrow_mut() = clip_items;
                if let Some(group_filter_button) = tool_buttons.first() {
                    self.ivars()
                        .group_filter_button
                        .set(group_filter_button.clone())
                        .unwrap();
                }
                self.ivars()
                    .source_tab_buttons
                    .set(source_tab_buttons)
                    .unwrap();
                self.refresh_native_clip_rows();
                if let Some(view)=self.ivars().window.get().and_then(|window|window.contentView()) {
                    let previous=unsafe {NSButton::buttonWithTitle_target_action(&NSString::from_str(appkit_tr("上一页","Previous")),Some(target),Some(sel!(zsclipPreviousPage:)),mtm)};
                    let next=unsafe {NSButton::buttonWithTitle_target_action(&NSString::from_str(appkit_tr("下一页","Next")),Some(target),Some(sel!(zsclipNextPage:)),mtm)};
                    previous.setFrame(NSRect::new(NSPoint::new(16.0,6.0),NSSize::new(90.0,26.0)));
                    next.setFrame(NSRect::new(NSPoint::new(116.0,6.0),NSSize::new(90.0,26.0)));
                    previous.setEnabled(false);next.setEnabled(false);
                    let page=NSTextField::labelWithString(&NSString::from_str(appkit_tr("第 1 页","Page 1")),mtm);
                    page.setFrame(NSRect::new(NSPoint::new(220.0,8.0),NSSize::new(190.0,22.0)));
                    unsafe {view.addSubview(&previous);view.addSubview(&next);view.addSubview(&page);}
                    self.ivars().previous_page_button.set(previous).unwrap();self.ivars().next_page_button.set(next).unwrap();self.ivars().page_label.set(page).unwrap();
                }

                app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
                self.install_status_item();
                self.install_clipboard_capture_timer();
                self.install_native_search_timer();
                self.install_vv_local_event_monitor();
                self.install_vv_global_event_monitor();
                self.install_vv_cg_event_tap_monitor();
                self.install_row_context_event_monitor();
                #[allow(deprecated)]
                app.activateIgnoringOtherApps(true);
                self.run_auto_smoke_if_requested();
                self.prepare_native_screenshot_scene();
            }
        }

        unsafe impl NSWindowDelegate for Delegate {
            #[unsafe(method(windowShouldClose:))]
            fn window_should_close(&self, sender: &NSWindow) -> Bool {
                if self
                    .ivars()
                    .window
                    .get()
                    .map(|window| {
                        Retained::<NSWindow>::as_ptr(window)
                            == (sender as *const NSWindow).cast_mut()
                    })
                    .unwrap_or(false)
                {
                    self.cancel_native_search();
                    sender.orderOut(None);
                    return false.into();
                }
                if self
                    .ivars()
                    .edit_window
                    .get()
                    .map(|window| {
                        Retained::<NSWindow>::as_ptr(window)
                            == (sender as *const NSWindow).cast_mut()
                    })
                    .unwrap_or(false)
                {
                    return self.perform_native_edit_close_request().into();
                }
                true.into()
            }

            #[unsafe(method(windowWillClose:))]
            fn window_will_close(&self, notification: &NSNotification) {
                let object =
                    unsafe { notification.object() }
                        .map(|object| Retained::as_ptr(&object).cast::<NSWindow>());
                if self
                    .ivars()
                    .edit_window
                    .get()
                    .map(|window| Some(Retained::<NSWindow>::as_ptr(window)) == object)
                    .unwrap_or(false)
                {
                    return;
                }
            }
        }

        unsafe impl NSControlTextEditingDelegate for Delegate {}

        unsafe impl NSTableViewDataSource for Delegate {
            #[unsafe(method(numberOfRowsInTableView:))]
            fn numberOfRowsInTableView(&self, _table_view: &NSTableView) -> NSInteger {
                self.ivars().clip_table_items.borrow().len().max(1) as NSInteger
            }

        }

        unsafe impl NSTableViewDelegate for Delegate {
            #[unsafe(method(tableView:viewForTableColumn:row:))]
            fn tableView_viewForTableColumn_row(
                &self,
                table_view: &NSTableView,
                _table_column: Option<&NSTableColumn>,
                row: NSInteger,
            ) -> *mut NSView {
                let Some(item) = self
                    .ivars()
                    .clip_table_items
                    .borrow()
                    .get(row as usize)
                    .cloned()
                else {
                    let width = table_view.bounds().size.width.max(320.0);
                    return Retained::autorelease_return(appkit_empty_clip_table_cell_view(
                        self.mtm(),
                        width,
                    ));
                };
                let presentation = native_host_clip_row_presentation_for_projection(&item);
                let width = table_view.bounds().size.width.max(320.0);
                Retained::autorelease_return(appkit_clip_table_cell_view(
                    self.mtm(),
                    &presentation,
                    width,
                    self.ivars().content_preferences.get(),
                    table_view.selectedRow()==row,
                ))
            }

            #[unsafe(method(tableViewSelectionDidChange:))]
            fn tableViewSelectionDidChange(&self, _notification: &NSNotification) {
                let Some(table_view) = self.ivars().clip_table_view.get() else {
                    return;
                };
                let row = table_view.selectedRow();
                if row < 0 {
                    return;
                }
                let Some(item) = self
                    .ivars()
                    .clip_table_items
                    .borrow()
                    .get(row as usize)
                    .cloned()
                else {
                    return;
                };
                let previous=self.ivars().selected_item_id.replace(item.id);
                if previous!=item.id && self.ivars().content_preferences.get().card_view_enabled {table_view.reloadData();}
                self.refresh_native_clip_row_selection();
            }
        }
    );

    fn appkit_empty_clip_table_cell_view(mtm: MainThreadMarker, width: f64) -> Retained<NSView> {
        let row_height = 44.0_f64;
        let cell = NSView::initWithFrame(
            NSView::alloc(mtm),
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, row_height)),
        );
        cell.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable);
        appkit_set_accessibility_label::<NSView>(cell.as_ref(), "No clipboard records");
        let label = appkit_clip_table_label(
            mtm,
            "No clipboard records",
            NSRect::new(
                NSPoint::new(16.0, 12.0),
                NSSize::new((width - 32.0).max(120.0), 20.0),
            ),
            13.0,
            &NSColor::secondaryLabelColor(),
        );
        cell.addSubview(&label);
        cell
    }

    fn appkit_clip_table_cell_view(
        mtm: MainThreadMarker,
        presentation: &NativeHostClipRowPresentation,
        width: f64,
        preferences:NativeContentPreferences,
        selected:bool,
    ) -> Retained<NSView> {
        let row_height = preferences.row_height();
        let cell = NSView::initWithFrame(
            NSView::alloc(mtm),
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, row_height)),
        );
        cell.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable);
        appkit_set_accessibility_label::<NSView>(cell.as_ref(), &presentation.accessibility_label);
        if preferences.card_view_enabled {
            let card=NSView::initWithFrame(NSView::alloc(mtm),NSRect::new(NSPoint::new(3.0,3.0),NSSize::new((width-6.0).max(1.0),row_height-6.0)));
            card.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable);
            card.setWantsLayer(true);
            if let Some(layer)=card.layer() {
                let color=if selected {NSColor::selectedControlColor()}else{NSColor::controlBackgroundColor()};
                let fill=color.CGColor();
                let border=NSColor::separatorColor().CGColor();
                let shadow=NSColor::blackColor().CGColor();
                layer.setBackgroundColor(Some(&fill));
                layer.setCornerRadius(7.0);
                layer.setBorderColor(Some(&border));
                layer.setBorderWidth(if preferences.card_border_enabled {1.0}else{0.0});
                layer.setShadowColor(Some(&shadow));
                layer.setShadowOpacity(if preferences.card_shadow_enabled {0.15}else{0.0});
                layer.setShadowRadius(2.0);
                layer.setShadowOffset(NSSize::new(0.0,-1.0));
            }
            unsafe {cell.addSubview(&card);}
        }

        let kind_icon = appkit_clip_table_icon_view(
            mtm,
            presentation.kind_icon.zsui_icon(),
            NSRect::new(NSPoint::new(20.0, (row_height-24.0)/2.0), NSSize::new(24.0, 24.0)),
        );

        let pin_width = 36.0_f64;
        let text_left = 82.0_f64;
        let text_right_padding = 16.0_f64
            + if presentation.pin_badge.is_some() {
                pin_width
            } else {
                0.0
            };
        let text_width = (width - text_left - text_right_padding).max(160.0);
        let content=if presentation.kind_icon==crate::app_core::NativeHostClipKindIcon::Phrase && !presentation.title.is_empty() {&presentation.title}else{&presentation.preview};
        let content_color=if selected {NSColor::selectedControlTextColor()}else{NSColor::labelColor()};
        let title_label = appkit_clip_table_label(
            mtm,
            content,
            NSRect::new(NSPoint::new(text_left, (row_height-preferences.content_font_size as f64-6.0)/2.0), NSSize::new(text_width, preferences.content_font_size as f64+6.0)),
            preferences.content_font_size as f64,
            &content_color,
        );

        unsafe { cell.addSubview(&kind_icon) };
        unsafe { cell.addSubview(&title_label) };

        if let Some(pin_badge) = presentation.pin_badge {
            let localized_pin_badge = appkit_localized_label(pin_badge);
            let pin_color = if selected { NSColor::selectedControlTextColor() } else { NSColor::controlAccentColor() };
            let pin_label = appkit_clip_table_label(
                mtm,
                &localized_pin_badge,
                NSRect::new(
                    NSPoint::new((width - pin_width - 8.0).max(text_left), 13.0),
                    NSSize::new(pin_width, 18.0),
                ),
                11.0,
                &pin_color,
            );
            pin_label.setAlignment(NSTextAlignment::Center);
            pin_label.setAutoresizingMask(NSAutoresizingMaskOptions::ViewMinXMargin);
            unsafe { cell.addSubview(&pin_label) };
        }

        cell
    }

    fn appkit_clip_table_label(
        mtm: MainThreadMarker,
        text: &str,
        frame: NSRect,
        font_size: f64,
        color: &NSColor,
    ) -> Retained<NSTextField> {
        let label = unsafe { NSTextField::labelWithString(&NSString::from_str(text), mtm) };
        label.setFrame(frame);
        label.setFont(Some(&NSFont::systemFontOfSize(font_size)));
        label.setTextColor(Some(color));
        label.setLineBreakMode(NSLineBreakMode::ByTruncatingTail);
        label.setUsesSingleLineMode(true);
        label.setMaximumNumberOfLines(1);
        label.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable);
        appkit_set_accessibility_label::<NSTextField>(label.as_ref(), text);
        label
    }

    fn appkit_clip_table_icon_view(
        mtm: MainThreadMarker,
        icon: crate::zsui::ZsIcon,
        frame: NSRect,
    ) -> Retained<NSImageView> {
        let image_view = NSImageView::initWithFrame(NSImageView::alloc(mtm), frame);
        image_view.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
        if let Some(image) = appkit_image_for_zsui_icon(icon) {
            image_view.setImage(Some(&image));
        }
        appkit_set_accessibility_label::<NSImageView>(image_view.as_ref(), icon.asset_name());
        image_view
    }

    fn appkit_image_for_zsui_icon(icon: crate::zsui::ZsIcon) -> Option<Retained<NSImage>> {
        let bytes = appkit_zsui_icon_png_bytes(icon)?;
        let data = unsafe { NSData::dataWithBytes_length(bytes.as_ptr().cast(), bytes.len()) };
        let image = NSImage::initWithData(NSImage::alloc(), &data)?;
        image.setTemplate(true);
        Some(image)
    }

    fn appkit_zsui_icon_png_bytes(icon: crate::zsui::ZsIcon) -> Option<&'static [u8]> {
        icon.png_24_bytes()
    }

    fn appkit_status_menu_symbol_name(icon_name: &str) -> Option<&'static str> {
        match icon_name {
            "window-new-symbolic" => Some("macwindow"),
            "media-record-symbolic" => Some("record.circle"),
            "network-wireless-symbolic" => Some("network"),
            "application-exit-symbolic" => Some("power"),
            _ => None,
        }
    }

    fn appkit_set_menu_item_command_modifier(item: &NSMenuItem) {
        unsafe {
            let _: () =
                msg_send![item, setKeyEquivalentModifierMask: NSEventModifierFlags::Command];
        }
    }

    fn appkit_enable_rounded_layer(view: &NSView, radius: f64) {
        unsafe {
            let _: () = msg_send![view, setWantsLayer: true];
            let layer: *mut AnyObject = msg_send![view, layer];
            if !layer.is_null() {
                let _: () = msg_send![layer, setCornerRadius: radius];
                let _: () = msg_send![layer, setMasksToBounds: true];
            }
        }
    }

    fn appkit_set_view_alpha(view: &NSView, alpha: f64) {
        unsafe {
            let _: () = msg_send![view, setAlphaValue: alpha];
        }
    }

    fn appkit_set_view_alpha_animated(view: &NSView, alpha: f64) {
        unsafe {
            let animator: *mut AnyObject = msg_send![view, animator];
            if animator.is_null() {
                appkit_set_view_alpha(view, alpha);
            } else {
                let _: () = msg_send![animator, setAlphaValue: alpha];
            }
        }
    }

    fn appkit_row_action_selector(action: NativeHostRowAction) -> Sel {
        match action {
            NativeHostRowAction::Paste => sel!(zsclipRowPaste:),
            NativeHostRowAction::Copy => sel!(zsclipRowCopy:),
            NativeHostRowAction::Pin => sel!(zsclipRowPin:),
            NativeHostRowAction::ToPhrase => sel!(zsclipRowToPhrase:),
            NativeHostRowAction::Delete => sel!(zsclipRowDelete:),
            NativeHostRowAction::Edit => sel!(zsclipRowEdit:),
            NativeHostRowAction::OpenPath => sel!(zsclipRowOpenPath:),
            NativeHostRowAction::OpenFolder => sel!(zsclipRowOpenFolder:),
            NativeHostRowAction::CopyPath => sel!(zsclipRowCopyPath:),
            #[cfg(feature = "ai-actions")]
            NativeHostRowAction::TextTranslate => sel!(zsclipRowTextTranslate:),
        }
    }

    fn appkit_main_tool_action_selector(action: NativeHostMainToolAction) -> Sel {
        match action {
            NativeHostMainToolAction::RowMenu => sel!(zsclipShowRowPopupMenu:),
            NativeHostMainToolAction::GroupFilter => sel!(zsclipShowGroupFilterPopupMenu:),
            #[cfg(feature = "vv-paste")]
            NativeHostMainToolAction::VvPopup => sel!(zsclipShowVvPopup:),
            #[cfg(feature = "vv-paste")]
            NativeHostMainToolAction::VvTrigger => sel!(zsclipTriggerVvDemo:),
        }
    }

    fn appkit_settings_action_selector(action: NativeHostSettingsAction) -> Sel {
        match action {
            NativeHostSettingsAction::Save => sel!(zsclipSaveSettings:),
            NativeHostSettingsAction::Close => sel!(zsclipCloseSettings:),
            NativeHostSettingsAction::OpenConfig => sel!(zsclipOpenSettingsConfig:),
        }
    }

    fn appkit_settings_control_action_selector(action: NativeHostSettingsControlAction) -> Sel {
        match action {
            NativeHostSettingsControlAction::ToggleAutostart => sel!(zsclipToggleAutostart:),
            NativeHostSettingsControlAction::ToggleClipboardCapture => {
                sel!(zsclipToggleClipboardCapture:)
            }
            #[cfg(feature = "lan-sync")]
            NativeHostSettingsControlAction::ToggleLanSync => sel!(zsclipToggleLanSync:),
            #[cfg(feature = "cloud-sync")]
            NativeHostSettingsControlAction::ToggleCloudSync => sel!(zsclipToggleCloudSync:),
            #[cfg(any(feature = "cloud-sync", feature = "lan-sync"))]
            NativeHostSettingsControlAction::OpenSyncModeDropdown => {
                sel!(zsclipOpenSyncModeDropdown:)
            }
        }
    }

    fn appkit_settings_platform_action_selector(action: NativeHostSettingsPlatformAction) -> Sel {
        match action {
            NativeHostSettingsPlatformAction::OpenSourceRepository => {
                sel!(zsclipOpenSourceRepository:)
            }
            NativeHostSettingsPlatformAction::CheckForUpdates => sel!(zsclipCheckForUpdates:),
            NativeHostSettingsPlatformAction::OpenWpsTaskpaneDocs => {
                sel!(zsclipOpenWpsTaskpaneDocs:)
            }
            NativeHostSettingsPlatformAction::DisableSystemClipboardHistory => {
                sel!(zsclipDisableSystemClipboardHistory:)
            }
            NativeHostSettingsPlatformAction::EnableSystemClipboardHistory => {
                sel!(zsclipEnableSystemClipboardHistory:)
            }
            NativeHostSettingsPlatformAction::RestartSystemShell => sel!(zsclipRestartSystemShell:),
        }
    }

    fn appkit_settings_group_action_selector(action: NativeHostSettingsGroupAction) -> Sel {
        match action {
            NativeHostSettingsGroupAction::ShowRecords => sel!(zsclipShowRecordGroups:),
            NativeHostSettingsGroupAction::ShowPhrases => sel!(zsclipShowPhraseGroups:),
            NativeHostSettingsGroupAction::Add => sel!(zsclipAddSettingsGroup:),
            NativeHostSettingsGroupAction::Rename => sel!(zsclipRenameSettingsGroup:),
            NativeHostSettingsGroupAction::Delete => sel!(zsclipDeleteSettingsGroup:),
            NativeHostSettingsGroupAction::MoveUp => sel!(zsclipMoveSettingsGroupUp:),
            NativeHostSettingsGroupAction::MoveDown => sel!(zsclipMoveSettingsGroupDown:),
        }
    }

    fn appkit_edit_text_action_selector(action: NativeHostEditTextAction) -> Sel {
        match action {
            NativeHostEditTextAction::Save => sel!(zsclipSaveEditText:),
            NativeHostEditTextAction::Cancel => sel!(zsclipCancelEditText:),
        }
    }

    fn appkit_dialog_action_selector(action: NativeHostDialogAction) -> Sel {
        match action {
            NativeHostDialogAction::ShowInfoMessage => sel!(zsclipShowInfoDialog:),
            NativeHostDialogAction::ConfirmQuestion => sel!(zsclipShowConfirmDialog:),
        }
    }

    fn appkit_button_from_spec<Spec>(
        mtm: MainThreadMarker,
        target: &AnyObject,
        spec: Spec,
        selector: Sel,
    ) -> Retained<NSButton>
    where
        Spec: HostComponent,
    {
        let bounds = spec.bounds();
        let localized = appkit_localized_label(spec.label());
        let title = NSString::from_str(&localized);
        let button = unsafe {
            NSButton::buttonWithTitle_target_action(&title, Some(target), Some(selector), mtm)
        };
        button.setFrame(NSRect::new(
            NSPoint::new(bounds.left as f64, bounds.top as f64),
            NSSize::new(bounds.width() as f64, bounds.height() as f64),
        ));
        appkit_set_accessibility_label::<NSButton>(button.as_ref(), &localized);
        appkit_apply_button_style_role(button.as_ref(), spec.style_role());
        button
    }

    fn appkit_apply_button_style_role(button: &NSButton, style_role: NativeButtonStyleRole) {
        match style_role {
            NativeButtonStyleRole::Plain | NativeButtonStyleRole::Destructive => {}
            NativeButtonStyleRole::Suggested => unsafe {
                let _: () = msg_send![button, setKeyEquivalent: ns_string!("\r")];
            },
        }
    }

    fn appkit_switch_from_spec<Spec>(
        mtm: MainThreadMarker,
        target: &AnyObject,
        spec: Spec,
        selector: Sel,
    ) -> Retained<NSButton>
    where
        Spec: HostComponent,
    {
        let button = appkit_button_from_spec(mtm, target, spec, selector);
        button.setButtonType(NSButtonType::Switch);
        button
    }

    fn appkit_dropdown_from_spec(
        mtm: MainThreadMarker,
        target: &AnyObject,
        spec: NativeDropdownSpec<NativeHostSettingsControlAction>,
        selector: Sel,
    ) -> Retained<NSPopUpButton> {
        let bounds = spec.bounds();
        let popup = unsafe {
            NSPopUpButton::initWithFrame_pullsDown(
                NSPopUpButton::alloc(mtm),
                NSRect::new(
                    NSPoint::new(bounds.left as f64, bounds.top as f64),
                    NSSize::new(bounds.width() as f64, bounds.height() as f64),
                ),
                false,
            )
        };
        if spec.options.is_empty() {
            let title = NSString::from_str(&appkit_localized_label(spec.label()));
            popup.addItemWithTitle(&title);
        } else {
            for option in spec.options {
                let title = NSString::from_str(&appkit_localized_label(option.label));
                popup.addItemWithTitle(&title);
            }
        }
        unsafe {
            let _: () = msg_send![&*popup, setTarget: target];
            let _: () = msg_send![&*popup, setAction: selector];
        }
        appkit_set_accessibility_label::<NSPopUpButton>(
            popup.as_ref(),
            &appkit_localized_label(spec.label),
        );
        popup
    }

    fn appkit_instance_button_from_spec(
        mtm: MainThreadMarker,
        target: &AnyObject,
        spec: &NativeComponentInstanceSpec,
        selector: Sel,
    ) -> Retained<NSButton> {
        let localized = appkit_localized_label(&spec.label);
        let title = NSString::from_str(&localized);
        let button = unsafe {
            NSButton::buttonWithTitle_target_action(&title, Some(target), Some(selector), mtm)
        };
        button.setFrame(NSRect::new(
            NSPoint::new(spec.bounds.left as f64, spec.bounds.top as f64),
            NSSize::new(spec.width() as f64, spec.height() as f64),
        ));
        appkit_set_accessibility_label::<NSButton>(button.as_ref(), &localized);
        button
    }

    fn appkit_clip_row_button_from_spec(
        mtm: MainThreadMarker,
        target: &AnyObject,
        spec: &NativeClipRowSpec,
        selector: Sel,
    ) -> Retained<NSButton> {
        let localized = appkit_localized_label(&spec.label);
        let title = NSString::from_str(&localized);
        let button = unsafe {
            NSButton::buttonWithTitle_target_action(&title, Some(target), Some(selector), mtm)
        };
        button.setFrame(NSRect::new(
            NSPoint::new(spec.bounds.left as f64, spec.bounds.top as f64),
            NSSize::new(spec.width() as f64, spec.height() as f64),
        ));
        appkit_set_accessibility_label::<NSButton>(button.as_ref(), &localized);
        button
    }

    fn appkit_set_accessibility_label<T>(element: &T, label: &str)
    where
        T: NSAccessibility + Message,
    {
        let label = NSString::from_str(label);
        element.setAccessibilityLabel(Some(&label));
    }

    fn appkit_is_dark_appearance(app: &NSApplication) -> bool {
        let name = app.effectiveAppearance().name();
        <Retained<NSString> as AsRef<NSString>>::as_ref(&name)
            == unsafe { NSAppearanceNameDarkAqua }
    }

    fn appkit_position_window_near_cursor(window: &NSWindow) {
        let mouse = NSEvent::mouseLocation();
        window.setFrameOrigin(NSPoint::new(mouse.x + 12.0, mouse.y - 420.0));
    }

    fn appkit_vv_popup_text_font(role: MainVvPopupTextRole, size: i32) -> Retained<NSFont> {
        match role {
            MainVvPopupTextRole::RowPreview => {
                NSFont::monospacedSystemFontOfSize_weight(size as f64, 0.0)
            }
            MainVvPopupTextRole::RowIndex => {
                NSFont::systemFontOfSize_weight((size + 4) as f64, 0.4)
            }
            _ => NSFont::systemFontOfSize(size as f64),
        }
    }

    fn appkit_is_mouse_down_event(event: &NSEvent) -> bool {
        matches!(
            event.r#type(),
            NSEventType::LeftMouseDown | NSEventType::RightMouseDown | NSEventType::OtherMouseDown
        )
    }

    fn appkit_event_key_text(event: &NSEvent) -> String {
        event
            .charactersIgnoringModifiers()
            .map(|text| text.to_string())
            .unwrap_or_default()
    }

    fn appkit_event_has_command_modifier(flags: NSEventModifierFlags) -> bool {
        flags.contains(NSEventModifierFlags::Command)
    }

    fn appkit_event_has_navigation_blocking_modifier(flags: NSEventModifierFlags) -> bool {
        flags.intersects(
            NSEventModifierFlags::Command
                | NSEventModifierFlags::Control
                | NSEventModifierFlags::Option,
        )
    }

    fn appkit_settings_page_label(page: crate::settings_model::SettingsPage) -> &'static str {
        use crate::settings_model::SettingsPage::*;
        match page {
            General => appkit_tr("常规", "General"),
            Appearance => appkit_tr("外观", "Appearance"),
            Clipboard => appkit_tr("剪贴板", "Clipboard"),
            Hotkey => appkit_tr("快捷键与 VV", "Hotkeys & VV"),
            Group => appkit_tr("分组", "Groups"),
            Plugin => appkit_tr("插件", "Plugins"),
            Cloud => appkit_tr("多端同步", "Sync"),
            About => appkit_tr("关于", "About"),
        }
    }

    fn appkit_settings_section_label(page: crate::settings_model::SettingsPage, index: usize, source: &'static str) -> &'static str {
        use crate::settings_model::SettingsPage::*;
        let english = match (page, index) {
            (General, 0) => "Startup & menu bar", (General, _) => "Configuration",
            (Appearance, 0) => "Text & cards", (Appearance, 1) => "History list & previews",
            (Appearance, 2) => "Window behavior", (Appearance, _) => "Window position",
            (Clipboard, 0) => "History & formatting", (Clipboard, 1) => "Paste behavior", (Clipboard, _) => "Sounds",
            (Hotkey, 0) => "Keyboard shortcuts", (Hotkey, 1) => "Mouse buttons", (Hotkey, 3) => "Using shortcuts", (Hotkey, _) => "VV quick paste",
            (Group, 0) => "Grouping", (Group, 1) => "Manage groups", (Group, _) => "Phrases",
            (Plugin, 0) => "Search", (Plugin, 1) => "Text recognition", (Plugin, 2) => "Translation",
            (Plugin, 3) => "Text cleanup", (Plugin, 4) => "Mail merge", (Plugin, 5) => "WPS task pane", (Plugin, _) => "QR codes",
            (Cloud, 0) => "Sync method", (Cloud, 1) => "WebDAV connection", (Cloud, 2) => "Cloud backup",
            (Cloud, 3) => "LAN connection", (Cloud, 4) => "Pair devices", (Cloud, _) => "Trusted devices",
            (About, 0) => "ZSClip", (About, 1) => "Updates", (About, _) => "Storage",
        };
        appkit_tr(source, english)
    }

    fn appkit_settings_control_label(control: &crate::settings_model::SettingsNativeControlSummary) -> String {
        let english = match control.key {
            "auto_start" => "Start at login", "silent_start" => "Start without opening the window",
            "tray_icon" => "Show menu bar icon", "app_icon" => "Show application icon", "close_to_tray" => "Keep running when the window closes",
            "dark_mode" => "Dark appearance", "content_font_size" => "Content font size", "card_view" => "Card view",
            "card_border" => "Card borders", "card_shadow" => "Subtle card shadows", "image_preview" => "Image thumbnails",
            "hover_preview" => "Preview on hover", "quick_delete" => "Quick delete button", "show_pin" => "Pin button",
            "image_row_height" => "Image row height", "text_row_height" => "Text row height", "file_row_height" => "File row height",
            "auto_hide_on_blur" => "Hide when focus leaves", "edge_auto_hide" => "Hide at the screen edge", "click_hide" => "Hide after pasting",
            "persistent_search" => "Keep search visible", "position_mode" => "Open window at", "mouse_offset" => "Pointer offset x / y",
            "fixed_position" => "Fixed position x / y", "capture_enable" => "Capture clipboard history", "max_items" => "Maximum saved items",
            "rich_text" => "Preserve text and table formatting", "dedupe_filter" => "Move duplicate content to the top",
            "paste_move_top" => "Move pasted items to the top", "context_menu_copy" => "Show Copy in the context menu", "skip_window" => "Skip selected paste targets",
            "skip_window_classes" => "Excluded window classes", "capture_skip_window" => "Capture current target",
            "copy_sound" => "Sound after copying", "paste_sound" => "Sound after pasting", "paste_sound_kind" => "Sound", "paste_sound_file" => "Choose sound file",
            "hotkey_enable" => "Enable global shortcut", "hotkey_modifier" => "Modifier", "hotkey_key" => "Key", "hotkey_record" => "Record shortcut",
            "plain_hotkey_enable" => "Enable plain-text paste shortcut", "plain_hotkey_modifier" => "Plain-text modifier", "plain_hotkey_key" => "Plain-text key",
            "mouse_side_button_enable" => "Enable mouse side buttons", "mouse_side_button_1" => "Side button 1", "mouse_side_button_2" => "Side button 2",
            "vv_mode" => "VV quick paste", "vv_source" => "VV source", "vv_group" => "Default VV group",
            "group_enable" => "Enable grouping", "group_type_filter" => "Show content type filters", "phrase_titles" => "Use separate phrase titles",
            "plugin_search" => "Enable web search", "search_engine" => "Search engine", "search_engine_reset" => "Restore preset",
            "ocr_provider" => "OCR provider", "ocr_cloud_url" => "OCR service address", "ocr_cloud_token" => "OCR access token",
            "translate_provider" => "Translation provider", "translate_app_id" => "Translation application ID", "translate_secret" => "Translation key", "translate_target" => "Target language",
            "plugin_ai_clean" => "Clean up text", "plugin_super_mail_merge" => "Super Mail Merge", "plugin_mail_merge" => "Open mail merge",
            "plugin_wps_taskpane" => "WPS task pane", "wps_taskpane_docs" => "WPS connection guide", "plugin_qr_quick" => "Convert text to QR code",
            "multi_sync_mode" => "Sync method", "cloud_sync_interval" => "Sync interval", "cloud_webdav_url" => "WebDAV address",
            "cloud_webdav_user" => "Username", "cloud_webdav_pass" => "Password", "cloud_remote_dir" => "Remote folder",
            "cloud_sync_now" => "Sync now", "cloud_upload_config" => "Upload configuration", "cloud_apply_config" => "Apply cloud configuration", "cloud_restore_backup" => "Restore cloud backup",
            "lan_device_name" => "Device name", "lan_tcp_port" => "TCP port", "lan_receive_mode" => "Received content", "lan_sync_mode" => "Automatic sync direction",
            "lan_manual_host" => "Desktop IP address", "lan_pair" => "Pair selected device", "lan_refresh" => "Refresh devices", "lan_accept_pair" => "Allow pairing", "lan_reject_pair" => "Reject pairing",
            "open_config" => "Open configuration file", "open_source" => "Source repository", "check_updates" => "Check for updates",
            _ => return appkit_localized_label(control.label),
        };
        appkit_tr(control.label, english).to_string()
    }

    fn appkit_apply_content_theme(mtm:MainThreadMarker) {
        let settings=crate::macos_app::macos_native_settings_json_snapshot();
        let dark=settings.get("dark_mode_enabled").and_then(serde_json::Value::as_bool).unwrap_or(false);
        let name=unsafe {if dark {NSAppearanceNameDarkAqua}else{objc2_app_kit::NSAppearanceNameAqua}};
        let appearance=objc2_app_kit::NSAppearance::appearanceNamed(name);
        let app=NSApplication::sharedApplication(mtm);
        unsafe {let _:()=msg_send![&app,setAppearance:appearance.as_deref()];}
    }

    fn appkit_settings_profile() -> serde_json::Value {
        crate::app_core::native_content_preferences::native_settings_profile(
            &crate::macos_app::macos_native_settings_json_snapshot(),
        )
    }

    fn appkit_settings_control_visible(control: &crate::settings_model::SettingsNativeControlSummary) -> bool {
        !matches!(control.key, "clipboard_history_disable" | "clipboard_history_enable" | "restart_shell" | "cloud_enable" | "lan_enable")
    }

    fn appkit_settings_text_label(mtm: MainThreadMarker, text: &str, frame: NSRect, size: f64, heading: bool) -> Retained<NSTextField> {
        let label = NSTextField::labelWithString(&NSString::from_str(text), mtm);
        label.setFrame(frame);
        let font = if heading { NSFont::boldSystemFontOfSize(size) } else { NSFont::systemFontOfSize(size) };
        label.setFont(Some(&font));
        label.setLineBreakMode(NSLineBreakMode::ByWordWrapping);
        label.setMaximumNumberOfLines(2);
        appkit_set_accessibility_label::<NSTextField>(label.as_ref(), text);
        label
    }

    fn appkit_settings_scroll_tab_item(
        mtm: MainThreadMarker,
        label: &str,
        document_height: f64,
    ) -> (Retained<NSTabViewItem>, Retained<NSView>, Retained<NSScrollView>) {
        let content = NSView::initWithFrame(NSView::alloc(mtm),
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(860.0, document_height)));
        let scroller = NSScrollView::initWithFrame(NSScrollView::alloc(mtm),
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(900.0, 500.0)));
        scroller.setHasVerticalScroller(true);
        scroller.setHasHorizontalScroller(false);
        scroller.setAutohidesScrollers(true);
        scroller.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable);
        scroller.setDocumentView(Some(&content));
        let content_label = format!("{label} settings page");
        let scroller_label = format!("{label} settings scroll area");
        appkit_set_accessibility_label::<NSView>(content.as_ref(), &content_label);
        appkit_set_accessibility_label::<NSScrollView>(scroller.as_ref(), &scroller_label);
        let item = unsafe { NSTabViewItem::initWithIdentifier(NSTabViewItem::alloc(), None) };
        item.setLabel(&NSString::from_str(label));
        item.setView(Some(&scroller));
        (item, content, scroller)
    }


    impl Delegate {
        fn new(mtm: MainThreadMarker) -> Retained<Self> {
            let this = Self::alloc(mtm).set_ivars(AppDelegateIvars::default());
            unsafe { msg_send![super(this), init] }
        }

        fn perform_native_host_action(&self, action: NativeHostUiAction) {
            let result = super::dispatch_appkit_host_action(action);
            eprintln!(
                "ZSClip AppKit action {} -> {}",
                action.action_name(),
                result.result_name
            );
            if action.opens_settings_surface() {
                self.present_settings_window(&result.result_name);
            }
            if action.toggles_search_surface() {
                self.toggle_search_field();
            }
            if action.hides_main_window_surface() {
                self.hide_main_window();
            }
            if action.should_close_host() {
                unsafe { NSApplication::sharedApplication(self.mtm()).terminate(None) };
            }
        }

        fn hide_main_window(&self) {
            self.cancel_native_search();
            if let Some(window) = self.ivars().window.get() {
                window.orderOut(None);
            }
        }

        fn toggle_main_window_visibility(&self) {
            let Some(window) = self.ivars().window.get() else {
                return;
            };
            if window.isVisible() {
                self.cancel_native_search();
                window.orderOut(None);
                return;
            }
            window.makeKeyAndOrderFront(None);
            self.reload_native_clip_items();
            unsafe {
                NSApplication::sharedApplication(self.mtm()).activateIgnoringOtherApps(true);
            }
        }

        fn install_status_item(&self) {
            if self.ivars().status_item.get().is_some() {
                return;
            }

            let mtm = self.mtm();
            let target: &AnyObject = self.as_ref();
            let status_bar = NSStatusBar::systemStatusBar();
            let status_item = status_bar.statusItemWithLength(NSVariableStatusItemLength);
            if let Some(button) = status_item.button(mtm) {
                let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(
                    ns_string!("doc.on.clipboard"),
                    Some(ns_string!("ZSClip")),
                );
                if let Some(image) = image {
                    image.setTemplate(true);
                    button.setImage(Some(&image));
                    button.setTitle(ns_string!(""));
                } else {
                    button.setTitle(ns_string!("ZSClip"));
                }
                appkit_set_accessibility_label::<NSStatusBarButton>(
                    button.as_ref(),
                    "ZSClip status menu",
                );
            }

            let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("ZSClip"));
            for spec in native_host_status_menu_item_specs() {
                if spec.starts_section {
                    menu.addItem(&NSMenuItem::separatorItem(mtm));
                }
                self.add_status_menu_item(&menu, spec, target);
            }
            status_item.setMenu(Some(&menu));
            self.ivars().status_menu.set(menu).unwrap();
            self.ivars().status_item.set(status_item).unwrap();
        }

        fn install_clipboard_capture_timer(&self) {
            let sequence =
                <crate::macos_app::MacosClipboardHost as crate::app_core::ClipboardHost>::sequence_number();
            self.ivars().last_clipboard_sequence.set(sequence);
            let Some(timer_class) = AnyClass::get(c"NSTimer") else {
                eprintln!("ZSClip AppKit clipboard timer unavailable");
                return;
            };
            unsafe {
                let _: *mut AnyObject = msg_send![
                    timer_class,
                    scheduledTimerWithTimeInterval: 0.8_f64,
                    target: self,
                    selector: sel!(zsclipClipboardPoll:),
                    userInfo: ptr::null_mut::<AnyObject>(),
                    repeats: true
                ];
            }
            eprintln!("ZSClip AppKit clipboard capture timer installed");
        }

        fn poll_native_clipboard_capture(&self) {
            let sequence =
                <crate::macos_app::MacosClipboardHost as crate::app_core::ClipboardHost>::sequence_number();
            if sequence == self.ivars().last_clipboard_sequence.get() {
                return;
            }
            self.ivars().last_clipboard_sequence.set(sequence);
            if !crate::macos_app::macos_native_clipboard_capture_enabled() {
                eprintln!("ZSClip AppKit clipboard capture skipped: disabled by settings");
                return;
            }
            let result =
                crate::native_clipboard_capture::NativeClipboardCaptureService::capture_current_with_html::<
                    crate::macos_app::MacosClipboardHost,
                >(0, "",|| {
                    if crate::macos_app::macos_native_settings_json_snapshot().get("rich_text_clipboard_enabled").and_then(serde_json::Value::as_bool).unwrap_or(true) {
                        crate::macos_app::MacosClipboardHost::read_html()
                    } else {None}
                });
            eprintln!(
                "ZSClip AppKit clipboard capture sequence={} inserted={} item_id={:?} reason={}",
                sequence, result.inserted, result.item_id, result.reason
            );
            if result.inserted {
                let _=crate::native_feedback::notify_success(crate::native_feedback::NativeFeedbackKind::Copy,&crate::macos_app::macos_native_settings_json_snapshot());
                self.reload_native_clip_items();
            }
        }

        fn add_status_menu_item(
            &self,
            menu: &NSMenu,
            spec: NativeMenuItemSpec<NativeHostStatusMenuAction>,
            target: &AnyObject,
        ) {
            let action = spec.action;
            let localized = appkit_localized_label(spec.label);
            let title = NSString::from_str(&localized);
            let key_equivalent = NSString::from_str(spec.accelerator_key);
            let item = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    NSMenuItem::alloc(self.mtm()),
                    &title,
                    Some(Self::status_selector(action)),
                    &key_equivalent,
                )
            };
            unsafe { item.setTarget(Some(target)) };
            if !spec.accelerator_key.is_empty() {
                appkit_set_menu_item_command_modifier(item.as_ref());
            }
            item.setTag(action.menu_id() as _);
            if let Some(enabled) = crate::macos_app::macos_native_status_menu_action_state(action) {
                item.setState(if enabled {
                    NSControlStateValueOn
                } else {
                    NSControlStateValueOff
                });
            }
            if let Some(symbol_name) = appkit_status_menu_symbol_name(spec.icon_name) {
                if let Some(image) = NSImage::imageWithSystemSymbolName_accessibilityDescription(
                    &NSString::from_str(symbol_name),
                    Some(&title),
                ) {
                    image.setTemplate(true);
                    item.setImage(Some(&image));
                }
            }
            menu.addItem(&item);
            self.ivars()
                .status_menu_items
                .borrow_mut()
                .push(NativeStatusMenuItemBinding { action, item });
        }

        fn status_selector(action: NativeHostStatusMenuAction) -> Sel {
            match action {
                NativeHostStatusMenuAction::ToggleWindow => sel!(zsclipStatusToggleWindow:),
                NativeHostStatusMenuAction::ToggleClipboardCapture => {
                    sel!(zsclipStatusToggleClipboardCapture:)
                }
                #[cfg(feature = "lan-sync")]
                NativeHostStatusMenuAction::ToggleLanSync => sel!(zsclipStatusToggleLanSync:),
                NativeHostStatusMenuAction::Exit => sel!(zsclipStatusExit:),
            }
        }

        fn perform_native_status_menu_action(&self, action: NativeHostStatusMenuAction) {
            let result = super::dispatch_appkit_status_menu_action(action);
            eprintln!(
                "ZSClip AppKit status menu action {} -> {}",
                action.action_name(),
                result.result_name
            );
            if action.toggles_main_window_surface() {
                self.toggle_main_window_visibility();
            }
            self.refresh_status_menu_action_state(action);
            if action.should_exit_host() {
                unsafe { NSApplication::sharedApplication(self.mtm()).terminate(None) };
            }
        }

        fn refresh_status_menu_action_state(&self, action: NativeHostStatusMenuAction) {
            let Some(enabled) = crate::macos_app::macos_native_status_menu_action_state(action)
            else {
                return;
            };
            for binding in self.ivars().status_menu_items.borrow().iter() {
                if binding.action != action {
                    continue;
                }
                let item: &NSMenuItem = binding.item.as_ref();
                item.setState(if enabled {
                    NSControlStateValueOn
                } else {
                    NSControlStateValueOff
                });
                break;
            }
        }

        fn run_auto_smoke_if_requested(&self) {
            if !matches!(
                std::env::var("ZSCLIP_NATIVE_HOST_AUTO_SMOKE").as_deref(),
                Ok("1")
            ) {
                return;
            }

            eprintln!("ZSClip AppKit auto smoke started");

            let clipboard_text = "zsclip appkit auto smoke clipboard";
            let clipboard_written =
                <crate::macos_app::MacosClipboardHost as crate::app_core::ClipboardHost>::write_text(
                    clipboard_text,
                );
            let clipboard_read =
                <crate::macos_app::MacosClipboardHost as crate::app_core::ClipboardHost>::read_text()
                    .unwrap_or_default();
            eprintln!(
                "ZSClip AppKit clipboard text smoke write={} read={}",
                clipboard_written,
                clipboard_read == clipboard_text
            );
            let file_sequence_before =
                <crate::macos_app::MacosClipboardHost as crate::app_core::ClipboardHost>::sequence_number();
            let smoke_file = std::env::temp_dir().join("zsclip-appkit-auto-smoke-file.txt");
            let _ = std::fs::write(&smoke_file, "zsclip appkit auto smoke file");
            let smoke_path = smoke_file.to_string_lossy().to_string();
            let file_written =
                <crate::macos_app::MacosClipboardHost as crate::app_core::ClipboardHost>::write_file_paths(
                    &[smoke_path.clone()],
                );
            let file_read =
                <crate::macos_app::MacosClipboardHost as crate::app_core::ClipboardHost>::read_file_paths()
                    .unwrap_or_default();
            let file_sequence_after =
                <crate::macos_app::MacosClipboardHost as crate::app_core::ClipboardHost>::sequence_number();
            eprintln!(
                "ZSClip AppKit clipboard file smoke write={} read={}",
                file_written,
                file_read.iter().any(|path| path == &smoke_path)
            );
            eprintln!(
                "ZSClip AppKit clipboard sequence smoke before={} after={} changed={}",
                file_sequence_before,
                file_sequence_after,
                file_sequence_after != file_sequence_before
            );
            let mut monitor_model = crate::macos_app::MacosApplicationModel::default();
            let _ = monitor_model.poll_clipboard_capture_event();
            let monitor_sequence_before =
                <crate::macos_app::MacosClipboardHost as crate::app_core::ClipboardHost>::sequence_number();
            let _ =
                <crate::macos_app::MacosClipboardHost as crate::app_core::ClipboardHost>::write_text(
                    "zsclip appkit monitor smoke clipboard",
                );
            let monitor_event = monitor_model.poll_clipboard_capture_event();
            let monitor_changed = matches!(
                monitor_event,
                Some(crate::app_core::ApplicationEvent::ClipboardChanged { sequence })
                    if sequence != monitor_sequence_before
            );
            eprintln!(
                "ZSClip AppKit clipboard monitor smoke changed={}",
                monitor_changed
            );
            let shell_open_host = crate::macos_app::MacosShellOpenHost::default();
            <crate::macos_app::MacosShellOpenHost as crate::app_core::NativeShellOpenHost>::open_path(
                &shell_open_host,
                &smoke_path,
            );
            let shell_open_recorded = shell_open_host
                .opened_paths()
                .iter()
                .any(|path| path == &smoke_path);
            eprintln!(
                "ZSClip AppKit shell open smoke dry_run={} recorded={}",
                matches!(
                    std::env::var("ZSCLIP_NATIVE_HOST_SHELL_OPEN_DRY_RUN").as_deref(),
                    Ok("1")
                ),
                shell_open_recorded
            );
            let previous_file_picker_smoke =
                std::env::var_os("ZSCLIP_NATIVE_HOST_FILE_PICKER_SMOKE_PATH");
            std::env::set_var("ZSCLIP_NATIVE_HOST_FILE_PICKER_SMOKE_PATH", &smoke_path);
            let file_dialog_host = crate::macos_app::MacosFileDialogHost::default();
            let file_picker_result =
                <crate::macos_app::MacosFileDialogHost as crate::app_core::NativeFileDialogHost>::pick_file(
                    &file_dialog_host,
                    crate::app_core::NativeFileDialogRequest {
                        title: "ZSClip Smoke File",
                        filter_name: "Text",
                        filter_pattern: "*.txt",
                        current_path: &smoke_path,
                    },
                );
            match previous_file_picker_smoke {
                Some(value) => {
                    std::env::set_var("ZSCLIP_NATIVE_HOST_FILE_PICKER_SMOKE_PATH", value)
                }
                None => std::env::remove_var("ZSCLIP_NATIVE_HOST_FILE_PICKER_SMOKE_PATH"),
            }
            let file_picker_recorded = !file_dialog_host.requests().is_empty();
            let file_picker_selected = matches!(
                file_picker_result.as_ref().map(|result| result.as_deref()),
                Ok(Some(path)) if path == smoke_path
            );
            eprintln!(
                "ZSClip AppKit file picker smoke injected=true recorded={} selected={}",
                file_picker_recorded, file_picker_selected
            );
            let identity = crate::macos_app::macos_native_identity_smoke();
            eprintln!(
                "ZSClip AppKit identity smoke queried=true pid={} process_name_seen={} bundle_id_seen={} foreground_seen={} exists={} foreground={} current_process_window={} foreground_requested={} focus_status={:?}",
                identity.current_pid,
                identity.process_name_seen,
                identity.bundle_id_seen,
                identity.foreground_seen,
                identity.current_process_exists,
                identity.current_process_foreground,
                identity.current_process_window,
                identity.foreground_requested,
                identity.focus_status
            );
            let seeded_item_id = crate::db_runtime::insert_native_clipboard_text(
                0,
                "zsclip appkit auto smoke editable record",
                "AppKit Smoke",
            )
            .ok()
            .and_then(|outcome| outcome.item_id);
            let seeded = seeded_item_id.is_some();
            self.reload_native_clip_items();
            eprintln!("ZSClip AppKit auto smoke real record seeded={}", seeded);

            self.perform_native_host_action(NativeHostUiAction::OpenSettings);
            self.perform_native_settings_control_action(
                NativeHostSettingsControlAction::ToggleClipboardCapture,
            );
            #[cfg(feature = "lan-sync")]
            self.perform_native_settings_control_action(
                NativeHostSettingsControlAction::ToggleLanSync,
            );
            #[cfg(any(feature = "cloud-sync", feature = "lan-sync"))]
            self.perform_native_settings_control_action(
                NativeHostSettingsControlAction::OpenSyncModeDropdown,
            );
            for action in [
                NativeHostDialogAction::ShowInfoMessage,
                NativeHostDialogAction::ConfirmQuestion,
            ] {
                let result = super::dispatch_appkit_dialog_action(action);
                eprintln!(
                    "ZSClip AppKit auto smoke dialog action {} -> {} accepted={}",
                    action.action_name(),
                    result.result_name,
                    result.accepted
                );
            }
            if let Some(item_id) = seeded_item_id {
                for action in [
                    NativeHostRowAction::Copy,
                    NativeHostRowAction::Paste,
                    NativeHostRowAction::Pin,
                ] {
                    let result = crate::macos_app::dispatch_macos_native_row_action_for_item(
                        action, item_id,
                    );
                    eprintln!(
                        "ZSClip AppKit auto smoke row action {} item_id={} -> {} accepted={}",
                        action.action_name(),
                        item_id,
                        result.result_name,
                        result.accepted
                    );
                }
                let edited_text = "zsclip appkit auto smoke edited text";
                let edit =
                    crate::macos_app::dispatch_macos_native_edit_text_save(item_id, edited_text);
                let edit_read_back = crate::db_runtime::item_text(item_id)
                    .ok()
                    .flatten()
                    .as_deref()
                    == Some(edited_text);
                eprintln!(
                    "ZSClip AppKit auto smoke edit save item_id={} -> {} accepted={} read_back={}",
                    item_id, edit.result_name, edit.accepted, edit_read_back
                );
                let image_seed = crate::db_runtime::insert_native_clipboard_image(
                    0,
                    &[255, 0, 0, 255, 0, 128, 255, 255],
                    2,
                    1,
                    "AppKit Smoke",
                )
                .ok()
                .and_then(|outcome| outcome.item_id);
                if let Some(image_item_id) = image_seed {
                    let image_copy = crate::macos_app::dispatch_macos_native_row_action_for_item(
                        NativeHostRowAction::Copy,
                        image_item_id,
                    );
                    let image_read =
                        <crate::macos_app::MacosClipboardHost as crate::app_core::ClipboardHost>::read_image_rgba()
                            .map(|(_, width, height)| (width, height));
                    eprintln!(
                        "ZSClip AppKit auto smoke image copy item_id={} -> {} accepted={} read={:?}",
                        image_item_id, image_copy.result_name, image_copy.accepted, image_read
                    );
                }
                if let Ok(group) =
                    crate::db_runtime::create_native_clip_group(0, "AppKit Auto Smoke")
                {
                    let assign =
                        crate::macos_app::dispatch_macos_native_assign_group(item_id, group.id);
                    eprintln!(
                        "ZSClip AppKit auto smoke assign group item_id={} group_id={} -> {} accepted={}",
                        item_id, group.id, assign.result_name, assign.accepted
                    );
                    let filter = crate::macos_app::dispatch_macos_native_group_filter(group.id);
                    eprintln!(
                        "ZSClip AppKit auto smoke group filter group_id={} -> {} accepted={}",
                        group.id, filter.result_name, filter.accepted
                    );
                    let remove = crate::macos_app::dispatch_macos_native_remove_group(item_id);
                    eprintln!(
                        "ZSClip AppKit auto smoke remove group item_id={} -> {} accepted={}",
                        item_id, remove.result_name, remove.accepted
                    );
                }
                let delete_seed = crate::db_runtime::insert_native_clipboard_text(
                    0,
                    "zsclip appkit auto smoke delete record",
                    "AppKit Smoke",
                )
                .ok()
                .and_then(|outcome| outcome.item_id);
                if let Some(delete_item_id) = delete_seed {
                    let delete = crate::macos_app::dispatch_macos_native_row_action_for_item(
                        NativeHostRowAction::Delete,
                        delete_item_id,
                    );
                    eprintln!(
                        "ZSClip AppKit auto smoke delete item_id={} -> {} accepted={}",
                        delete_item_id, delete.result_name, delete.accepted
                    );
                }
                self.reload_native_clip_items();
            }
            if std::env::var_os("ZSCLIP_NATIVE_HOST_SCREENSHOT_SCENE").is_none() {
                self.ivars().auto_smoke_row_item_id.set(seeded_item_id);
            }
            let self_target = std::process::id() as u64;
            let first = self.perform_native_vv_key_text("v", false, self_target, 1);
            let second = self.perform_native_vv_key_text("v", false, self_target, 2);
            let rejected = matches!(first.action, NativeHostVvTriggerAction::Ignore)
                && matches!(second.action, NativeHostVvTriggerAction::Ignore)
                && !first.consume_key && !second.consume_key;
            eprintln!("ZSClip AppKit auto smoke VV self-target rejected={rejected}");
            #[cfg(feature = "lan-sync")]
            self.perform_native_status_menu_action(NativeHostStatusMenuAction::ToggleLanSync);

            if self.ivars().auto_smoke_row_item_id.get().is_none() {
                eprintln!("ZSClip AppKit auto smoke finished");
            }
        }

        fn finish_auto_smoke_rows_after_search(&self) {
            let Some(item_id) = self.ivars().auto_smoke_row_item_id.take() else { return; };
            if !self.ivars().clip_table_items.borrow().iter().any(|item| item.id == item_id) {
                eprintln!("ZSClip AppKit auto smoke row verification blocked=seed_not_visible");
                return;
            }
            self.ivars().selected_item_id.set(item_id);
            self.refresh_native_clip_row_selection();
            let expected = crate::db_runtime::item_text(item_id).ok().flatten();
            self.perform_native_row_action(NativeHostRowAction::Copy);
            let copied = expected.is_some() &&
                <crate::macos_app::MacosClipboardHost as crate::app_core::ClipboardHost>::read_text() == expected;
            self.perform_native_row_action(NativeHostRowAction::Edit);
            #[cfg(feature = "ai-actions")]
            self.perform_native_row_action(NativeHostRowAction::TextTranslate);
            let edited_text = "zsclip appkit native editor saved text";
            if let Some(editor) = self.ivars().edit_text_view.get() {
                editor.setString(&NSString::from_str(edited_text));
                self.perform_native_edit_save();
            }
            let edit_verified = crate::db_runtime::item_text(item_id).ok().flatten().as_deref() == Some(edited_text);
            eprintln!("ZSClip AppKit auto smoke native rows copy_verified={copied} edit_verified={edit_verified}");
            eprintln!("ZSClip AppKit auto smoke finished");
        }

        fn present_native_row_popup_menu(&self) {
            self.present_native_row_popup_menu_at(NSPoint::new(24.0, 272.0));
        }

        fn present_native_row_popup_menu_at(&self, location: NSPoint) {
            let Some(window) = self.ivars().window.get() else {
                return;
            };
            let Some(view) = window.contentView() else {
                return;
            };
            let target: &AnyObject = self.as_ref();
            let groups = crate::db_runtime::native_clip_groups(self.active_source_category())
                .unwrap_or_default();
            let items = self.ivars().clip_items.borrow();
            let grouping_enabled = crate::macos_app::macos_native_grouping_enabled();
            let row_actions_title = NSString::from_str(appkit_tr("行操作", "Row Actions"));
            let mut entries=native_host_full_row_popup_menu_entries_for_groups(
                &groups,native_host_row_popup_menu_input_for_projection(&items,self.ivars().selected_item_id.get(),grouping_enabled),
                |label|crate::i18n::translate(label).into_owned());
            let kind=items.iter().find(|item|item.id==self.ivars().selected_item_id.get()).map(|item|item.kind).unwrap_or(ClipKind::Text);
            if let Some(entry)=self.ivars().content_preferences.get().rename_phrase_menu_entry(kind,appkit_tr("重命名短语","Rename Phrase")) {entries.insert(0,entry);}
            drop(items);
            let menu = self.build_popup_menu(
                &row_actions_title,
                &entries,
                target,
            );
            let shown =
                menu.popUpMenuPositioningItem_atLocation_inView(None, location, Some(&view));
            eprintln!("ZSClip AppKit row popup menu shown: {}", shown);
        }

        fn present_native_group_filter_popup_menu(&self) {
            self.present_native_group_filter_popup_menu_at(NSPoint::new(4.0, 196.0));
        }

        fn present_native_group_filter_popup_menu_at(&self, location: NSPoint) {
            let Some(window) = self.ivars().window.get() else {
                return;
            };
            let Some(view) = window.contentView() else {
                return;
            };
            let target: &AnyObject = self.as_ref();
            let groups = crate::db_runtime::native_clip_groups(self.active_source_category())
                .unwrap_or_default();
            let group_filter_title = NSString::from_str(appkit_tr("分组", "Group Filter"));
            let category = self.active_source_category();
            let menu = self.build_popup_menu(
                &group_filter_title,
                &native_host_group_filter_popup_menu_entries_for_groups_kind_filter(
                    &groups,
                    self.ivars().current_group_filter.get(),
                    crate::macos_app::macos_native_group_type_filter_enabled(),
                    self.ivars().current_kind_filter.get(),
                    clip_kind_filter_options_for_tab(category as usize),
                ),
                target,
            );
            let shown =
                menu.popUpMenuPositioningItem_atLocation_inView(None, location, Some(&view));
            eprintln!("ZSClip AppKit group filter popup menu shown: {}", shown);
        }

        fn present_native_vv_popup(&self) {
            let mtm = self.mtm();
            self.dismiss_native_vv_popup("replace_session");
            let settings = crate::macos_app::macos_native_settings_json_snapshot();
            let source_category = crate::settings_model::settings_native_vv_source_tab(&settings) as i64;
            let current_group_id = settings.get("vv_group_id").and_then(serde_json::Value::as_i64).unwrap_or(0);
            let target_pid = Self::appkit_frontmost_pid().filter(|pid| *pid != std::process::id() as i32).unwrap_or(0);
            let snapshot = match crate::native_vv::NativeVvSnapshot::capture(source_category, current_group_id) {
                Ok(snapshot) => snapshot,
                Err(error) => { eprintln!("ZSClip AppKit VV open blocked: {error}"); return; }
            };
            let preferences = NativeContentPreferences::from_json(&settings);
            let items = preferences.apply_projection(snapshot.items.clone());
            let groups = crate::db_runtime::native_clip_groups(source_category).unwrap_or_default();
            let group_label = native_host_group_filter_label_for_groups(&groups, current_group_id);
            let width = 820.0;
            let height = 460.0;
            let window = self.ivars().vv_popup_window.get_or_init(|| {
                let allocated = VvNonactivatingPanel::alloc(mtm);
                let panel: Retained<VvNonactivatingPanel> = unsafe { msg_send![allocated,
                    initWithContentRect: NSRect::new(NSPoint::new(0.0,0.0),NSSize::new(width,height)),
                    styleMask: NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel,
                    backing: NSBackingStoreType::Buffered, defer: false] };
                let panel: Retained<NSPanel> = panel.into_super();
                panel.setBecomesKeyOnlyIfNeeded(true);
                panel.setFloatingPanel(true);
                panel.setHidesOnDeactivate(false);
                let window: Retained<NSWindow> = panel.into_super();
                unsafe { window.setReleasedWhenClosed(false); }
                window.setLevel(NSFloatingWindowLevel);
                window.setHasShadow(true);
                window.setBackgroundColor(Some(&NSColor::windowBackgroundColor()));
                window.setTitle(&NSString::from_str(appkit_tr("ZSClip VV 粘贴", "ZSClip VV Popup")));
                window
            });
            let view = unsafe { NSView::initWithFrame(NSView::alloc(mtm), NSRect::new(NSPoint::new(0.0,0.0),NSSize::new(width,height))) };
            let heading = appkit_settings_text_label(mtm, &format!("VV · {}", appkit_localized_label(&group_label)), NSRect::new(NSPoint::new(18.0,height-42.0),NSSize::new(760.0,24.0)),15.0,true);
            heading.setFont(Some(&NSFont::boldSystemFontOfSize(15.0)));
            unsafe { view.addSubview(&heading); }
            self.ivars().vv_candidate_buttons.borrow_mut().clear();
            for (index, item) in items.iter().enumerate() {
                let summary = if item.kind == crate::app_core::ClipKind::Phrase && !item.title.trim().is_empty() {
                    item.title.as_str()
                } else {
                    item.preview.as_str()
                };
                let pinned = if item.pinned { format!(" · {}", appkit_tr("置顶", "Pinned")) } else { String::new() };
                let title = format!("{}  {}{}", index+1, summary, pinned);
                let button = unsafe { NSButton::buttonWithTitle_target_action(&NSString::from_str(&title),Some(self.as_ref()),Some(sel!(zsclipVvSelect:)),mtm) };
                button.setFrame(NSRect::new(NSPoint::new(16.0,height-86.0-index as f64*38.0),NSSize::new(300.0,34.0)));
                button.setFont(Some(&NSFont::systemFontOfSize(preferences.content_font_size as f64)));
                button.setAlignment(NSTextAlignment::Left);
                button.setTag(index as _);
                appkit_set_accessibility_label::<NSButton>(button.as_ref(), &title);
                unsafe { view.addSubview(&button); }
                self.ivars().vv_candidate_buttons.borrow_mut().push(button);
            }
            if items.is_empty() {
                let empty=appkit_settings_text_label(mtm,appkit_tr("此分组暂无记录","No records in this group"),NSRect::new(NSPoint::new(20.0,height-96.0),NSSize::new(290.0,36.0)),13.0,false);
                unsafe {view.addSubview(&empty);}
            }
            let scroller = unsafe { NSScrollView::initWithFrame(NSScrollView::alloc(mtm),NSRect::new(NSPoint::new(332.0,54.0),NSSize::new(470.0,350.0))) };
            scroller.setHasVerticalScroller(true);
            scroller.setHasHorizontalScroller(false);
            scroller.setBorderType(NSBorderType::BezelBorder);
            let body = unsafe { NSTextView::initWithFrame(NSTextView::alloc(mtm),NSRect::new(NSPoint::new(0.0,0.0),NSSize::new(450.0,350.0))) };
            body.setEditable(false);
            body.setSelectable(false);
            body.setRichText(false);
            body.setVerticallyResizable(true);
            body.setHorizontallyResizable(false);
            body.setMinSize(NSSize::new(0.0,350.0));
            body.setMaxSize(NSSize::new(450.0,f64::MAX));
            body.setTextContainerInset(NSSize::new(10.0,8.0));
            body.setFont(Some(&NSFont::systemFontOfSize(preferences.content_font_size as f64)));
            unsafe {
                if let Some(container)=body.textContainer() {
                    container.setContainerSize(NSSize::new(450.0,f64::MAX));
                    container.setWidthTracksTextView(true);
                }
            }
            appkit_set_accessibility_label::<NSTextView>(body.as_ref(),appkit_tr("VV 完整正文预览","VV full text preview"));
            scroller.setDocumentView(Some(&body));
            unsafe {view.addSubview(&scroller);}
            let status=appkit_settings_text_label(mtm,appkit_tr("1–9 粘贴 · ↑↓ 预览 · PgUp/PgDn 翻页 · Esc 关闭","1–9 paste · ↑↓ preview · PgUp/PgDn scroll · Esc close"),NSRect::new(NSPoint::new(18.0,15.0),NSSize::new(780.0,28.0)),12.0,false);
            if self.ivars().vv_cg_event_tap.get().is_none() {
                status.setStringValue(&NSString::from_str(appkit_tr("点击候选粘贴；键盘 VV 需要系统的辅助功能与输入监控权限。","Click a candidate to paste; keyboard VV requires Accessibility and Input Monitoring permission.")));
            }
            unsafe {view.addSubview(&status);}
            window.setContentView(Some(&view));
            *self.ivars().vv_preview_text.borrow_mut()=Some(body);
            *self.ivars().vv_preview_scroll.borrow_mut()=Some(scroller);
            *self.ivars().vv_status_label.borrow_mut()=Some(status);
            let serial=self.ivars().vv_session_serial.get().wrapping_add(1).max(1);
            self.ivars().vv_session_serial.set(serial);
            *self.ivars().vv_presentation.borrow_mut()=Some(MacosVvPresentation {serial,target_pid,snapshot});
            let mut input=self.ivars().vv_input.borrow_mut();
            let id=input.begin(target_pid as usize,target_pid as usize,false);
            input.show(id,items.len());
            drop(input);
            window.center();
            window.orderFrontRegardless();
            self.ivars().vv_preview_hover.set(None);
            self.queue_native_vv_preview(0,0);
            self.ivars().vv_screenshot_waiting.set(std::env::var("ZSCLIP_NATIVE_HOST_SCREENSHOT_SCENE").as_deref()==Ok("vv"));
            eprintln!("ZSClip AppKit VV opened session={serial} target_pid={target_pid} category={source_category} candidates={} key_window={} frontmost_unchanged={}",items.len(),window.isKeyWindow(),target_pid==0||Self::appkit_frontmost_pid()==Some(target_pid));
        }

        fn appkit_frontmost_pid() -> Option<i32> {
            NSWorkspace::sharedWorkspace().frontmostApplication().map(|app|app.processIdentifier()).filter(|pid|*pid>0)
        }

        fn queue_native_vv_preview(&self, index: usize, delay_ms: u64) {
            let Some(session)=self.ivars().vv_presentation.borrow().clone() else {return;};
            if index>=session.snapshot.items.len() {return;}
            self.ivars().vv_preview_request.set(self.ivars().vv_preview_request.get().wrapping_add(1));
            self.ivars().vv_preview_due.set(Some((std::time::Instant::now()+std::time::Duration::from_millis(delay_ms),index)));
            self.ivars().vv_preview_result.borrow_mut().take();
        }

        fn start_native_vv_preview(&self, index: usize) {
            let Some(session)=self.ivars().vv_presentation.borrow().clone() else {return;};
            let request=self.ivars().vv_preview_request.get();
            self.ivars().vv_preview_selected.set(index);
            for (row,button) in self.ivars().vv_candidate_buttons.borrow().iter().enumerate() {
                button.setState(if row==index {NSControlStateValueOn}else{NSControlStateValueOff});
            }
            if let Some(body)=self.ivars().vv_preview_text.borrow().as_ref() {
                body.setString(&NSString::from_str(appkit_tr("正在读取完整正文…","Loading full text…")));
            }
            let (sender,receiver)=std::sync::mpsc::channel();
            *self.ivars().vv_preview_result.borrow_mut()=Some(receiver);
            if let Err(error)=std::thread::Builder::new().name("zsclip-vv-preview".into()).spawn(move|| {
                let body=session.snapshot.load_item(index).map(|item| {
                    item.text.or_else(||item.file_paths.map(|paths|paths.join("\n")))
                        .unwrap_or_else(||crate::i18n::tr("图片记录","Image record").to_string())
                });
                let _=sender.send(MacosVvPreviewResult {serial:session.serial,request,index,body});
            }) {
                eprintln!("ZSClip AppKit VV preview worker unavailable: {error}");
                self.dismiss_native_vv_popup("preview_worker_unavailable");
            }
        }

        fn poll_native_vv_preview(&self) {
            self.poll_native_vv_delivery_smoke();
            let Some(session)=self.ivars().vv_presentation.borrow().clone() else {return;};
            let Some(window)=self.ivars().vv_popup_window.get().filter(|window|window.isVisible()) else {return;};
            if session.snapshot.validate().is_err() || (session.target_pid>0 && Self::appkit_frontmost_pid()!=Some(session.target_pid)) {
                self.dismiss_native_vv_popup("stale_target_or_history");
                return;
            }
            let point=window.mouseLocationOutsideOfEventStream();
            let hover=self.ivars().vv_candidate_buttons.borrow().iter().position(|button|NSPointInRect(point,button.frame()));
            if hover!=self.ivars().vv_preview_hover.get() {
                let previous=self.ivars().vv_preview_hover.replace(hover);
                if let Some(index)=hover {self.queue_native_vv_preview(index,200);}
                else if self.ivars().vv_preview_due.get().is_some_and(|(_,index)|Some(index)==previous) {self.ivars().vv_preview_due.set(None);}
            }
            if let Some((due,index))=self.ivars().vv_preview_due.get() {
                if std::time::Instant::now()>=due {self.ivars().vv_preview_due.set(None);self.start_native_vv_preview(index);}
            }
            let received=self.ivars().vv_preview_result.borrow().as_ref().map(|receiver|receiver.try_recv());
            if matches!(&received,Some(Err(std::sync::mpsc::TryRecvError::Disconnected))) {
                self.dismiss_native_vv_popup("preview_worker_disconnected");return;
            }
            let result=received.and_then(Result::ok);
            if let Some(result)=result {
                self.ivars().vv_preview_result.borrow_mut().take();
                if result.serial!=session.serial || result.request!=self.ivars().vv_preview_request.get() || result.index!=self.ivars().vv_preview_selected.get() || session.snapshot.validate().is_err() {return;}
                let body=match result.body {
                    Ok(body)=>body,
                    Err(error)=>{eprintln!("ZSClip AppKit VV preview rejected: {error}");self.dismiss_native_vv_popup("preview_unavailable");return;}
                };
                if let Some(view)=self.ivars().vv_preview_text.borrow().as_ref() {
                    view.setString(&NSString::from_str(&body));
                    view.sizeToFit();
                    view.scrollRangeToVisible(objc2_foundation::NSRange::new(0,0));
                }
                eprintln!("ZSClip AppKit VV preview ready session={} item_id={} characters={}",session.serial,session.snapshot.items[result.index].id,body.chars().count());
                if self.ivars().vv_screenshot_waiting.replace(false) {eprintln!("ZSClip AppKit screenshot scene ready=vv");}
                if self.ivars().vv_delivery_smoke_phase.get()==2 {
                    self.ivars().vv_delivery_smoke_phase.set(3);
                    let selected=session.snapshot.items.iter().position(|item|item.id==self.ivars().vv_delivery_smoke_item_id.get());
                    let button=selected.and_then(|index|self.ivars().vv_candidate_buttons.borrow().get(index).cloned());
                    if let Some(button)=button {unsafe {button.performClick(None);}}
                }
            } else if session.snapshot.items.is_empty() && self.ivars().vv_screenshot_waiting.replace(false) {
                eprintln!("ZSClip AppKit screenshot scene ready=vv");
            }
        }

        fn scroll_native_vv_preview(&self, direction: i32) {
            if let Some(scroller)=self.ivars().vv_preview_scroll.borrow().as_ref() {
                let clip=scroller.contentView();
                let bounds=clip.bounds();
                let document_height=scroller.documentView().map(|view|view.frame().size.height).unwrap_or(0.0);
                let y=(bounds.origin.y+direction as f64*bounds.size.height*0.85).clamp(0.0,(document_height-bounds.size.height).max(0.0));
                clip.scrollToPoint(NSPoint::new(0.0,y));
                scroller.reflectScrolledClipView(&clip);
            }
        }

        fn poll_native_vv_delivery_smoke(&self) {
            let phase=self.ivars().vv_delivery_smoke_phase.get();
            if phase==3 {return;}
            if self.ivars().vv_delivery_smoke_deadline.get().is_some_and(|deadline|std::time::Instant::now()>deadline) {
                self.ivars().vv_delivery_smoke_phase.set(3);
                eprintln!("ZSClip AppKit VV delivery blocked=receiver_or_capture_timeout");return;
            }
            if phase==0 {
                self.ivars().vv_delivery_smoke_phase.set(3);
                if std::env::var("ZSCLIP_NATIVE_VV_DELIVERY_SMOKE").as_deref()!=Ok("1") || std::env::var_os("ZSCLIP_DATA_DIR").is_none() {return;}
                let Some(pid)=std::env::var("ZSCLIP_NATIVE_VV_RECEIVER_PID").ok().and_then(|value|value.parse::<i32>().ok()).filter(|pid|*pid>0&&*pid!=std::process::id() as i32) else {
                    eprintln!("ZSClip AppKit VV delivery blocked=invalid_receiver_pid");return;
                };
                if !CGPreflightPostEventAccess() {eprintln!("ZSClip AppKit VV delivery blocked=post_event_permission; allow Accessibility in System Settings");return;}
                self.ivars().vv_delivery_smoke_pid.set(pid);
                self.ivars().vv_delivery_smoke_deadline.set(Some(std::time::Instant::now()+std::time::Duration::from_secs(20)));
                self.ivars().vv_delivery_smoke_phase.set(1);
                eprintln!("ZSClip AppKit VV delivery waiting=receiver_clipboard target_pid={pid}");
            } else if phase==1 {
                let expected=std::env::var("ZSCLIP_VV_RECEIVER_PAYLOAD").unwrap_or_else(|_|"VV-DELIVERY-PAYLOAD".into());
                if <crate::macos_app::MacosClipboardHost as crate::app_core::ClipboardHost>::read_text().as_deref()!=Some(expected.as_str()) {return;}
                let captured=crate::native_clipboard_capture::NativeClipboardCaptureService::capture_current::<crate::macos_app::MacosClipboardHost>(0,"VV receiver");
                let Some(item_id)=captured.item_id else {return;};
                self.ivars().vv_delivery_smoke_item_id.set(item_id);
                let pid=self.ivars().vv_delivery_smoke_pid.get();
                if let Some(receiver)=NSRunningApplication::runningApplicationWithProcessIdentifier(pid) {
                    let activated=receiver.activateWithOptions(NSApplicationActivationOptions::ActivateIgnoringOtherApps);
                    eprintln!("ZSClip AppKit VV delivery activation requested={activated} target_pid={pid}");
                    self.ivars().vv_delivery_smoke_phase.set(4);
                } else {self.ivars().vv_delivery_smoke_phase.set(3);eprintln!("ZSClip AppKit VV delivery blocked=receiver_not_running");}
            } else if phase==4 {
                if Self::appkit_frontmost_pid()==Some(self.ivars().vv_delivery_smoke_pid.get()) {
                    self.ivars().vv_delivery_smoke_phase.set(2);
                    self.present_native_vv_popup();
                    let selected=self.ivars().vv_presentation.borrow().as_ref().and_then(|session|session.snapshot.items.iter().position(|item|item.id==self.ivars().vv_delivery_smoke_item_id.get()));
                    if let Some(index)=selected {self.queue_native_vv_preview(index,0);}
                    else {self.ivars().vv_delivery_smoke_phase.set(3);eprintln!("ZSClip AppKit VV delivery blocked=captured_candidate_unavailable");}
                }
            }
        }

        fn build_popup_menu(
            &self,
            title: &NSString,
            entries: &[NativePopupMenuEntry],
            target: &AnyObject,
        ) -> Retained<NSMenu> {
            let menu = unsafe { NSMenu::initWithTitle(NSMenu::alloc(self.mtm()), title) };
            for entry in entries {
                self.add_popup_menu_entry(&menu, entry, target);
            }
            menu
        }

        fn add_popup_menu_entry(
            &self,
            menu: &NSMenu,
            entry: &NativePopupMenuEntry,
            target: &AnyObject,
        ) {
            match entry {
                NativePopupMenuEntry::Command {
                    id,
                    label,
                    enabled,
                    checked,
                } => {
                    let localized = appkit_localized_label(label);
                    let title = NSString::from_str(&localized);
                    let key_equivalent = NSString::from_str(
                        native_popup_menu_command_macos_key_equivalent(*id).unwrap_or(""),
                    );
                    let item = unsafe {
                        NSMenuItem::initWithTitle_action_keyEquivalent(
                            NSMenuItem::alloc(self.mtm()),
                            &title,
                            Some(sel!(zsclipPopupRowCommand:)),
                            &key_equivalent,
                        )
                    };
                    unsafe { item.setTarget(Some(target)) };
                    if native_popup_menu_command_macos_key_equivalent(*id).is_some() {
                        appkit_set_menu_item_command_modifier(item.as_ref());
                    }
                    item.setTag(*id as _);
                    item.setEnabled(*enabled);
                    if *checked {
                        item.setState(NSControlStateValueOn);
                    }
                    if let Some(symbol_name) = native_popup_menu_command_macos_symbol_name(*id) {
                        if let Some(image) =
                            NSImage::imageWithSystemSymbolName_accessibilityDescription(
                                &NSString::from_str(symbol_name),
                                Some(&title),
                            )
                        {
                            image.setTemplate(true);
                            item.setImage(Some(&image));
                        }
                    }
                    menu.addItem(&item);
                }
                NativePopupMenuEntry::Submenu {
                    label,
                    enabled,
                    entries,
                } => {
                    let localized = appkit_localized_label(label);
                    let title = NSString::from_str(&localized);
                    let item = unsafe {
                        NSMenuItem::initWithTitle_action_keyEquivalent(
                            NSMenuItem::alloc(self.mtm()),
                            &title,
                            None,
                            ns_string!(""),
                        )
                    };
                    item.setEnabled(*enabled);
                    let submenu = self.build_popup_menu(&title, entries, target);
                    item.setSubmenu(Some(&submenu));
                    menu.addItem(&item);
                }
                NativePopupMenuEntry::Separator => {
                    menu.addItem(&NSMenuItem::separatorItem(self.mtm()));
                }
            }
        }

        fn toggle_search_field(&self) {
            let Some(search_field) = self.ivars().search_field.get() else {
                return;
            };
            let next_hidden = !search_field.isHidden();
            if !next_hidden {
                self.focus_native_search_field();
            } else {
                self.hide_native_search_field();
            }
        }

        fn focus_native_search_field(&self) {
            let Some(search_field) = self.ivars().search_field.get() else {
                return;
            };
            search_field.setHidden(false);
            appkit_set_view_alpha_animated(search_field.as_ref(), 1.0);
            if let Some(window) = self.ivars().window.get() {
                window.makeFirstResponder(Some(search_field));
            }
        }

        fn hide_native_search_field(&self) -> bool {
            let Some(search_field) = self.ivars().search_field.get() else {
                return false;
            };
            let was_visible = !search_field.isHidden();
            appkit_set_view_alpha_animated(search_field.as_ref(), 0.0);
            search_field.setHidden(true);
            search_field.setStringValue(ns_string!(""));
            self.update_clip_list_visibility("");
            if let (true, Some(window), Some(table_view)) = (
                was_visible,
                self.ivars().window.get(),
                self.ivars().clip_table_view.get(),
            ) {
                window.makeFirstResponder(Some(table_view));
            }
            was_visible
        }

        fn present_settings_window(&self, _route_name: &str) {
            if let Some(window) = self.ivars().settings_window.get() {
                window.makeKeyAndOrderFront(None);
                self.refresh_settings_dependencies();
                return;
            }
            use crate::settings_model::SettingsPage;
            let mtm = self.mtm();
            let window = unsafe { NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm), NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(960.0, 680.0)),
                NSWindowStyleMask::Titled | NSWindowStyleMask::Closable | NSWindowStyleMask::Miniaturizable | NSWindowStyleMask::Resizable,
                NSBackingStoreType::Buffered, false) };
            unsafe { window.setReleasedWhenClosed(false); }
            window.setTitle(&NSString::from_str(appkit_tr("ZSClip 设置", "ZSClip Settings")));
            window.setContentMinSize(NSSize::new(900.0, 560.0));
            let view = NSVisualEffectView::initWithFrame(NSVisualEffectView::alloc(mtm),
                NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(960.0, 680.0)));
            view.setMaterial(NSVisualEffectMaterial::WindowBackground);
            view.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
            view.setState(NSVisualEffectState::FollowsWindowActiveState);
            view.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable);
            window.setContentView(Some(&view));
            let title = appkit_settings_text_label(mtm, appkit_tr("设置", "Settings"),
                NSRect::new(NSPoint::new(28.0, 628.0), NSSize::new(860.0, 32.0)), 23.0, true);
            title.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewMinYMargin);
            view.addSubview(&title);
            let status = appkit_settings_text_label(mtm, appkit_tr("更改将在保存后应用", "Changes apply when saved"),
                NSRect::new(NSPoint::new(28.0, 26.0), NSSize::new(600.0, 22.0)), 12.0, false);
            status.setTextColor(Some(&NSColor::secondaryLabelColor()));
            status.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable);
            view.addSubview(&status);
            self.ivars().settings_route_label.set(status).ok();

            let settings_tab_view = NSTabView::initWithFrame(NSTabView::alloc(mtm),
                NSRect::new(NSPoint::new(20.0, 70.0), NSSize::new(920.0, 540.0)));
            settings_tab_view.setTabViewType(NSTabViewType::TopTabsBezelBorder);
            settings_tab_view.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable);
            appkit_set_accessibility_label::<NSTabView>(settings_tab_view.as_ref(), appkit_tr("设置分类", "Settings categories"));
            view.addSubview(&settings_tab_view);
            self.ivars().settings_native_text_fields.borrow_mut().clear();
            self.ivars().settings_native_toggle_buttons.borrow_mut().clear();
            self.ivars().settings_native_dropdown_buttons.borrow_mut().clear();
            self.ivars().settings_native_route_buttons.borrow_mut().clear();
            let settings_json = appkit_settings_profile();
            let pages = crate::settings_model::settings_native_page_summaries();
            let sections = crate::settings_model::settings_native_section_summaries();
            let controls = crate::settings_model::settings_native_control_summaries();
            let mut page_scrollers = Vec::new();
            for page in pages {
                let page_sections = sections.iter().filter(|section| section.page == page.page)
                    .filter(|section| (page.page == SettingsPage::Group && section.section_index == 1)
                        || controls.iter().any(|control| control.page == page.page && control.section_index == section.section_index && appkit_settings_control_visible(control)))
                    .collect::<Vec<_>>();
                let section_rows = |section_index| controls.iter().filter(|control| control.page == page.page
                    && control.section_index == section_index && appkit_settings_control_visible(control)).count();
                let document_height = (40.0 + page_sections.iter().map(|section| {
                    58.0 + if page.page == SettingsPage::Group && section.section_index == 1 { 338.0 }
                    else { section_rows(section.section_index) as f64 * 44.0 }
                }).sum::<f64>()).max(500.0);
                let label = appkit_settings_page_label(page.page);
                let (tab_item, content, scroller) = appkit_settings_scroll_tab_item(mtm, label, document_height);
                settings_tab_view.addTabViewItem(&tab_item);
                let mut offset = 24.0;
                for section in page_sections {
                    let section_label = appkit_settings_section_label(page.page, section.section_index, section.section_title);
                    let heading = appkit_settings_text_label(mtm, section_label,
                        NSRect::new(NSPoint::new(28.0, document_height - offset - 28.0), NSSize::new(780.0, 28.0)), 16.0, true);
                    content.addSubview(&heading);
                    offset += 38.0;
                    if page.page == SettingsPage::Group && section.section_index == 1 {
                        self.build_settings_group_controls(&content, document_height - offset);
                        offset += 338.0;
                    } else {
                        for control in controls.iter().filter(|control| control.page == page.page
                            && control.section_index == section.section_index && appkit_settings_control_visible(control)) {
                            self.build_settings_control(&content, control, &settings_json, document_height - offset - 32.0);
                            offset += 44.0;
                        }
                    }
                    offset += 20.0;
                }
                page_scrollers.push((scroller, document_height));
            }
            self.ivars().settings_tabs.set(settings_tab_view).ok();
            let target: &AnyObject = self.as_ref();
            for (action, x, width) in [(NativeHostSettingsAction::Close, 700.0, 100.0), (NativeHostSettingsAction::Save, 814.0, 118.0)] {
                let button = unsafe { NSButton::buttonWithTitle_target_action(
                    &NSString::from_str(&appkit_localized_label(action.button_label())), Some(target), Some(appkit_settings_action_selector(action)), mtm) };
                button.setFrame(NSRect::new(NSPoint::new(x, 20.0), NSSize::new(width, 32.0)));
                button.setAutoresizingMask(NSAutoresizingMaskOptions::ViewMinXMargin);
                appkit_set_accessibility_label::<NSButton>(button.as_ref(), &appkit_localized_label(action.button_label()));
                view.addSubview(&button);
                if action == NativeHostSettingsAction::Save { self.ivars().settings_save_button.set(button).ok(); }
            }
            window.center();
            window.makeKeyAndOrderFront(None);
            self.ivars().settings_window.set(window).ok();
            self.ivars().settings_page_scrollers.set(page_scrollers.iter().map(|(scroller, _)| scroller.clone()).collect()).ok();
            for (scroller, height) in page_scrollers {
                let clip = scroller.contentView();
                clip.scrollToPoint(NSPoint::new(0.0, (height - clip.bounds().size.height).max(0.0)));
                scroller.reflectScrolledClipView(&clip);
            }
            self.refresh_settings_group_rows();
            self.refresh_settings_dependencies();
        }

        fn build_settings_control(&self, view: &NSView, control: &crate::settings_model::SettingsNativeControlSummary,
            settings_json: &serde_json::Value, y: f64) {
            use crate::settings_model::{SettingsNativeControlKind as Kind, SettingsNativeControlRouteKind};
            let mtm = self.mtm();
            let title = appkit_settings_control_label(control);
            let display = crate::settings_model::settings_native_control_display_value(control, settings_json);
            let target: &AnyObject = self.as_ref();
            match control.kind {
                Kind::Toggle => {
                    let button = unsafe { NSButton::buttonWithTitle_target_action(&NSString::from_str(&title),
                        Some(target), Some(sel!(zsclipSettingsDraftChanged:)), mtm) };
                    button.setButtonType(NSButtonType::Switch);
                    let initial_value = display.as_ref().is_some_and(|display| display.value.eq_ignore_ascii_case("true"));
                    button.setState(if initial_value { NSControlStateValueOn } else { NSControlStateValueOff });
                    button.setFrame(NSRect::new(NSPoint::new(32.0, y), NSSize::new(770.0, 28.0)));
                    appkit_set_accessibility_label::<NSButton>(button.as_ref(), &title);
                    self.ivars().settings_native_toggle_buttons.borrow_mut().push(NativeSettingsToggleButtonBinding {
                        control_key: control.key, initial_value, button: button.clone(),
                    });
                    view.addSubview(&button);
                }
                Kind::TextInput => {
                    view.addSubview(&appkit_settings_text_label(mtm, &title,
                        NSRect::new(NSPoint::new(32.0, y + 4.0), NSSize::new(264.0, 26.0)), 13.0, false));
                    let field = NSTextField::labelWithString(ns_string!(""), mtm);
                    field.setFrame(NSRect::new(NSPoint::new(310.0, y), NSSize::new(492.0, 30.0)));
                    field.setBezeled(true);
                    let sensitive = display.as_ref().is_some_and(|value| value.sensitive);
                    field.setEditable(!sensitive);
                    field.setSelectable(!sensitive);
                    field.setEnabled(!sensitive);
                    let initial_value = display.as_ref().map(|value| value.value.clone()).unwrap_or_default();
                    field.setStringValue(&NSString::from_str(if sensitive { appkit_tr("凭据单独管理", "Credentials are managed separately") } else { &initial_value }));
                    appkit_set_accessibility_label::<NSTextField>(field.as_ref(), &title);
                    if !sensitive {
                        self.ivars().settings_native_text_fields.borrow_mut().push(NativeSettingsTextFieldBinding {
                            control_key: control.key, initial_value, field: field.clone(),
                        });
                    }
                    view.addSubview(&field);
                }
                Kind::Dropdown => {
                    view.addSubview(&appkit_settings_text_label(mtm, &title,
                        NSRect::new(NSPoint::new(32.0, y + 4.0), NSSize::new(264.0, 26.0)), 13.0, false));
                    let Some(options) = native_settings_dropdown_options_for_host(control, settings_json) else { return; };
                    let popup = NSPopUpButton::initWithFrame_pullsDown(NSPopUpButton::alloc(mtm),
                        NSRect::new(NSPoint::new(310.0, y), NSSize::new(360.0, 30.0)), false);
                    let mut option_values = Vec::new();
                    for option in &options.options {
                        popup.addItemWithTitle(&NSString::from_str(&appkit_localized_label(&option.label)));
                        option_values.push(option.raw_value.clone());
                    }
                    popup.selectItemAtIndex(options.selected_index as _);
                    unsafe { popup.setTarget(Some(target)); popup.setAction(Some(sel!(zsclipSettingsDraftChanged:))); }
                    appkit_set_accessibility_label::<NSPopUpButton>(popup.as_ref(), &title);
                    let initial_value = option_values.get(options.selected_index).cloned().unwrap_or_default();
                    self.ivars().settings_native_dropdown_buttons.borrow_mut().push(NativeSettingsDropdownButtonBinding {
                        control_key: control.key, initial_value, option_values, button: popup.clone(),
                    });
                    view.addSubview(&popup);
                }
                Kind::Button => {
                    let selector = if control.key == "open_config" { Some(sel!(zsclipOpenSettingsConfig:)) }
                        else if control.route.is_some_and(|route| route.kind == SettingsNativeControlRouteKind::Action) { Some(sel!(zsclipSettingsNativeRouteAction:)) }
                        else { None };
                    let button = unsafe { NSButton::buttonWithTitle_target_action(&NSString::from_str(&title), Some(target), selector, mtm) };
                    button.setFrame(NSRect::new(NSPoint::new(32.0, y), NSSize::new(360.0, 30.0)));
                    button.setEnabled(selector.is_some());
                    appkit_set_accessibility_label::<NSButton>(button.as_ref(), &title);
                    if control.key != "open_config" {
                        if let Some(route) = control.route {
                            if let Some(action_name) = route.action_name {
                                let mut bindings = self.ivars().settings_native_route_buttons.borrow_mut();
                                let tag = 10_000 + bindings.len() as isize;
                                button.setTag(tag);
                                bindings.push(NativeSettingsRouteButtonBinding { tag, route_name: route.route_name, action_name });
                            }
                        }
                    }
                    view.addSubview(&button);
                    if control.key=="paste_sound_file" {
                        self.ivars().settings_sound_file_button.set(button.clone()).ok();
                        let preview=unsafe {NSButton::buttonWithTitle_target_action(&NSString::from_str(appkit_tr("试听","Preview sound")),Some(target),Some(sel!(zsclipSoundPreview:)),mtm)};
                        preview.setFrame(NSRect::new(NSPoint::new(410.0,y),NSSize::new(150.0,30.0)));
                        appkit_set_accessibility_label::<NSButton>(preview.as_ref(),appkit_tr("试听提示音","Preview notification sound"));
                        view.addSubview(&preview);self.ivars().settings_sound_preview_button.set(preview).ok();
                    }
                }
                Kind::Label | Kind::List => {
                    let text = match control.key {
                        "about_version" => format!("ZSClip {}", crate::app_version::APP_VERSION),
                        "data_directory" => appkit_tr("配置与历史记录按当前用户保存。", "Configuration and history are saved for the current user.").to_string(),
                        "phrase_titles_note" => appkit_tr("关闭后显示正文摘要，已保存的标题保留。", "Turning titles off keeps existing titles and shows a content preview.").to_string(),
                        "hotkey_preview" => format!("{} + {}", settings_json["hotkey_mod"].as_str().unwrap_or(""), settings_json["hotkey_key"].as_str().unwrap_or("")),
                        "plain_hotkey_preview" => format!("{} + {}", settings_json["plain_paste_hotkey_mod"].as_str().unwrap_or(""), settings_json["plain_paste_hotkey_key"].as_str().unwrap_or("")),
                        "hotkey_note_main" => appkit_tr("全局快捷键与 VV 可分别开启。", "The global shortcut and VV can be enabled independently.").to_string(),
                        "hotkey_note_plain" => appkit_tr("纯文本粘贴不保留格式。", "Plain-text paste removes formatting.").to_string(),
                        "lan_discovered_list" => appkit_tr("发现的设备会在局域网连接中显示。", "Discovered devices are shown in the LAN connection list.").to_string(),
                        _ => display.map(|value| if value.value.is_empty() { title.clone() } else { format!("{title}: {}", value.value) }).unwrap_or(title),
                    };
                    let label = appkit_settings_text_label(mtm, &text,
                        NSRect::new(NSPoint::new(32.0, y), NSSize::new(770.0, 34.0)), 12.0, false);
                    label.setTextColor(Some(&NSColor::secondaryLabelColor()));
                    view.addSubview(&label);
                }
            }
        }

        fn build_settings_group_controls(&self, view: &NSView, top: f64) {
            let mtm = self.mtm();
            let target: &AnyObject = self.as_ref();
            let actions = [NativeHostSettingsGroupAction::ShowRecords, NativeHostSettingsGroupAction::ShowPhrases,
                NativeHostSettingsGroupAction::Add, NativeHostSettingsGroupAction::Rename, NativeHostSettingsGroupAction::Delete,
                NativeHostSettingsGroupAction::MoveUp, NativeHostSettingsGroupAction::MoveDown];
            let labels = [appkit_tr("复制记录", "Clipboard Records"), appkit_tr("常用短语", "Phrases"), appkit_tr("新建分组", "Add group"),
                appkit_tr("重命名", "Rename"), appkit_tr("删除", "Delete"), appkit_tr("上移", "Move up"), appkit_tr("下移", "Move down")];
            for (index, action) in actions.into_iter().enumerate() {
                let button = unsafe { NSButton::buttonWithTitle_target_action(&NSString::from_str(labels[index]), Some(target), Some(appkit_settings_group_action_selector(action)), mtm) };
                let (x, y, width) = if index < 2 { (32.0 + index as f64 * 194.0, top - 32.0, 182.0) }
                    else { (32.0 + (index - 2) as f64 * 156.0, top - 120.0, 144.0) };
                button.setFrame(NSRect::new(NSPoint::new(x, y), NSSize::new(width, 30.0)));
                appkit_set_accessibility_label::<NSButton>(button.as_ref(), labels[index]);
                view.addSubview(&button);
            }
            view.addSubview(&appkit_settings_text_label(mtm, appkit_tr("分组名称", "Group name"),
                NSRect::new(NSPoint::new(32.0, top - 73.0), NSSize::new(148.0, 26.0)), 13.0, false));
            let name_field = NSTextField::labelWithString(ns_string!(""), mtm);
            name_field.setFrame(NSRect::new(NSPoint::new(190.0, top - 78.0), NSSize::new(610.0, 30.0)));
            name_field.setEditable(true); name_field.setSelectable(true); name_field.setBezeled(true);
            appkit_set_accessibility_label::<NSTextField>(name_field.as_ref(), appkit_tr("分组名称", "Group name"));
            view.addSubview(&name_field);
            self.ivars().settings_group_name_field.set(name_field).ok();
            let list = NSView::initWithFrame(NSView::alloc(mtm), NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(748.0, 190.0)));
            let scroll = NSScrollView::initWithFrame(NSScrollView::alloc(mtm),
                NSRect::new(NSPoint::new(32.0, top - 324.0), NSSize::new(770.0, 190.0)));
            scroll.setHasVerticalScroller(true); scroll.setAutohidesScrollers(true);
            scroll.setDocumentView(Some(&list));
            appkit_set_accessibility_label::<NSScrollView>(scroll.as_ref(), appkit_tr("分组列表", "Group list"));
            view.addSubview(&scroll);
            self.ivars().settings_group_list_view.set(list).ok();
            self.ivars().settings_group_list_scroll.set(scroll).ok();
        }

        fn refresh_settings_dependencies(&self) {
            let toggles = self.ivars().settings_native_toggle_buttons.borrow();
            let cards = toggles.iter().find(|binding| binding.control_key == "card_view")
                .is_some_and(|binding| binding.button.state() == NSControlStateValueOn);
            for binding in toggles.iter().filter(|binding| matches!(binding.control_key, "card_border" | "card_shadow")) {
                binding.button.setEnabled(cards);
            }
            drop(toggles);
            let mut bindings = self.ivars().settings_native_dropdown_buttons.borrow_mut();
            let source = bindings.iter().find(|binding| binding.control_key == "vv_source")
                .and_then(|binding| binding.option_values.get(binding.button.indexOfSelectedItem().max(0) as usize))
                .and_then(|value| value.parse::<i64>().ok()).unwrap_or(0);
            if let Some(binding) = bindings.iter_mut().find(|binding| binding.control_key == "vv_group") {
                let selected = binding.option_values.get(binding.button.indexOfSelectedItem().max(0) as usize)
                    .and_then(|value| value.parse::<i64>().ok()).unwrap_or(0);
                let controls = crate::settings_model::settings_native_control_summaries();
                if let Some(control) = controls.iter().find(|control| control.key == "vv_group") {
                    if let Some(options) = native_settings_dropdown_options_for_host(control, &serde_json::json!({"vv_source_tab":source,"vv_group_id":selected})) {
                        binding.button.removeAllItems();
                        binding.option_values.clear();
                        for option in options.options {
                            binding.button.addItemWithTitle(&NSString::from_str(&appkit_localized_label(&option.label)));
                            binding.option_values.push(option.raw_value);
                        }
                        binding.button.selectItemAtIndex(options.selected_index as _);
                    }
                }
            }
        }

        fn show_settings_screenshot_scene(&self, scene: &str) -> bool {
            let index = match scene {
                "settings-general" => 0, "settings-appearance" => 1, "settings-clipboard" => 2,
                "settings-hotkey" => 3, "settings-group" => 4, "settings-plugin" => 5,
                "settings-cloud" => 6, "settings-about" => 7, _ => return false,
            };
            self.present_settings_window("");
            if let Some(tabs) = self.ivars().settings_tabs.get() { tabs.selectTabViewItemAtIndex(index); }
            if std::env::var_os("ZSCLIP_DATA_DIR").is_some() && matches!(scene, "settings-appearance" | "settings-group") {
                if scene == "settings-appearance" {
                    let font = self.ivars().settings_native_dropdown_buttons.borrow().iter().find(|binding| binding.control_key == "content_font_size").cloned();
                    if let Some(binding) = font {
                        if let Some(index) = binding.option_values.iter().position(|value| value == "18") { binding.button.selectItemAtIndex(index as _); }
                    }
                }
                let key = if scene == "settings-appearance" { "card_view" } else { "phrase_titles" };
                let enabled = scene == "settings-appearance";
                let toggle = self.ivars().settings_native_toggle_buttons.borrow().iter().find(|binding| binding.control_key == key).cloned();
                if let Some(binding) = toggle {
                    if (binding.button.state() == NSControlStateValueOn) != enabled { unsafe { binding.button.performClick(None); } }
                }
                if let Some(save) = self.ivars().settings_save_button.get() { unsafe { save.performClick(None); } }
                let settings = crate::macos_app::macos_native_settings_json_snapshot();
                let verified = if scene == "settings-appearance" { settings["content_font_size"] == 18 && settings["card_view_enabled"] == true }
                    else { settings["phrase_titles_enabled"] == false };
                eprintln!("ZSClip AppKit settings scene saved verification scene={} verified={}", scene, verified);
            }
            if scene == "settings-group" {
                if let Some(scroll) = self.ivars().settings_page_scrollers.get().and_then(|scrolls| scrolls.get(4)) {
                    let clip = scroll.contentView();
                    clip.scrollToPoint(NSPoint::new(0.0, 0.0));
                    scroll.reflectScrolledClipView(&clip);
                }
            }
            if let Some(window) = self.ivars().settings_window.get() { window.makeKeyAndOrderFront(None); }
            eprintln!("ZSClip AppKit screenshot scene ready={}", scene);
            true
        }


        fn settings_group_name_input(&self) -> String {
            self.ivars()
                .settings_group_name_field
                .get()
                .map(|field| field.stringValue().to_string())
                .unwrap_or_else(|| "新分组".to_string())
        }

        fn settings_groups(&self) -> Vec<crate::app_core::ClipGroup> {
            crate::db_runtime::native_clip_groups(self.ivars().settings_group_category.get())
                .unwrap_or_default()
        }

        fn select_settings_group(&self, group_id: i64) {
            if group_id <= 0 {
                return;
            }
            self.ivars().selected_settings_group_id.set(group_id);
            if let Some(group) = self
                .settings_groups()
                .into_iter()
                .find(|group| group.id == group_id)
            {
                if let Some(field) = self.ivars().settings_group_name_field.get() {
                    let text = NSString::from_str(&group.name);
                    field.setStringValue(&text);
                }
            }
            self.refresh_settings_group_rows();
            eprintln!("ZSClip AppKit settings group selected id={}", group_id);
        }

        fn refresh_settings_group_rows(&self) {
            let Some(list) = self.ivars().settings_group_list_view.get() else {
                return;
            };
            let groups = self.settings_groups();
            let mut selected_id = self.ivars().selected_settings_group_id.get();
            if selected_id == 0 || !groups.iter().any(|group| group.id == selected_id) {
                selected_id = groups.first().map(|group| group.id).unwrap_or_default();
                self.ivars().selected_settings_group_id.set(selected_id);
            }
            let mut rows = self.ivars().settings_group_rows.borrow_mut();
            for row in rows.drain(..) { row.removeFromSuperview(); }
            let height = (groups.len().max(1) as f64 * 34.0 + 12.0).max(190.0);
            list.setFrameSize(NSSize::new(748.0, height));
            let target: &AnyObject = self.as_ref();
            if groups.is_empty() {
                let row = unsafe { NSButton::buttonWithTitle_target_action(
                    &NSString::from_str(appkit_tr("暂无分组", "No groups yet")), None, None, self.mtm()) };
                row.setFrame(NSRect::new(NSPoint::new(8.0, height - 36.0), NSSize::new(724.0, 28.0)));
                row.setEnabled(false);
                list.addSubview(&row); rows.push(row);
            }
            for (index, group) in groups.iter().enumerate() {
                let row = unsafe { NSButton::buttonWithTitle_target_action(&NSString::from_str(&group.name),
                    Some(target), Some(sel!(zsclipSelectSettingsGroup:)), self.mtm()) };
                row.setButtonType(NSButtonType::Radio);
                row.setState(if group.id == selected_id { NSControlStateValueOn } else { NSControlStateValueOff });
                row.setFrame(NSRect::new(NSPoint::new(8.0, height - 36.0 - index as f64 * 34.0), NSSize::new(724.0, 28.0)));
                row.setTag(group.id as isize);
                appkit_set_accessibility_label::<NSButton>(row.as_ref(), &group.name);
                list.addSubview(&row); rows.push(row);
            }
            if let Some(scroll) = self.ivars().settings_group_list_scroll.get() {
                let clip = scroll.contentView();
                clip.scrollToPoint(NSPoint::new(0.0, (height - clip.bounds().size.height).max(0.0)));
                scroll.reflectScrolledClipView(&clip);
            }
        }

        fn perform_settings_group_create(&self) {
            let category = self.ivars().settings_group_category.get();
            let name = self.settings_group_name_input();
            let result = crate::macos_app::dispatch_macos_native_create_group(category, &name);
            eprintln!(
                "ZSClip AppKit settings group create -> {}",
                result.result_name
            );
            self.refresh_settings_group_rows();
            self.refresh_main_group_state_after_settings_change(category, None);
        }

        fn perform_settings_group_rename(&self) {
            let category = self.ivars().settings_group_category.get();
            let group_id = self.ivars().selected_settings_group_id.get();
            let name = self.settings_group_name_input();
            let result =
                crate::macos_app::dispatch_macos_native_rename_group(category, group_id, &name);
            eprintln!(
                "ZSClip AppKit settings group rename id={} -> {}",
                group_id, result.result_name
            );
            self.refresh_settings_group_rows();
            self.refresh_main_group_state_after_settings_change(category, None);
        }

        fn perform_settings_group_delete(&self) {
            let category = self.ivars().settings_group_category.get();
            let group_id = self.ivars().selected_settings_group_id.get();
            let result = crate::macos_app::dispatch_macos_native_delete_group(group_id);
            eprintln!(
                "ZSClip AppKit settings group delete id={} -> {}",
                group_id, result.result_name
            );
            self.ivars().selected_settings_group_id.set(0);
            self.refresh_settings_group_rows();
            self.refresh_main_group_state_after_settings_change(category, Some(group_id));
        }

        fn perform_settings_group_move(&self, step: i32) {
            let category = self.ivars().settings_group_category.get();
            let group_id = self.ivars().selected_settings_group_id.get();
            let result =
                crate::macos_app::dispatch_macos_native_move_group(category, group_id, step);
            eprintln!(
                "ZSClip AppKit settings group move id={} step={} -> {}",
                group_id, step, result.result_name
            );
            self.refresh_settings_group_rows();
            self.refresh_main_group_state_after_settings_change(category, None);
        }

        fn refresh_main_group_state_after_settings_change(
            &self,
            category: i64,
            deleted_group_id: Option<i64>,
        ) {
            let changed_category = native_host_source_tab_for_category(category).category;
            if changed_category != self.active_source_category() {
                return;
            }
            if deleted_group_id == Some(self.ivars().current_group_filter.get()) {
                self.ivars().current_group_filter.set(0);
            }
            self.reload_native_clip_items();
        }

        fn refresh_main_state_after_settings_save(&self) {
            appkit_apply_content_theme(self.mtm());
            let preferences=NativeContentPreferences::from_json(&crate::macos_app::macos_native_settings_json_snapshot());
            self.ivars().content_preferences.set(preferences);
            if let Some(table)=self.ivars().clip_table_view.get() {
                table.setRowHeight(preferences.row_height());
                table.setIntercellSpacing(NSSize::new(0.0,if preferences.card_view_enabled {4.0}else{1.0}));
                table.setUsesAlternatingRowBackgroundColors(!preferences.card_view_enabled);
            }
            if let Some(search)=self.ivars().search_field.get() {search.setFont(Some(&NSFont::systemFontOfSize(preferences.content_font_size as f64)));}
            let groups = crate::db_runtime::native_clip_groups(self.active_source_category())
                .unwrap_or_default();
            let current_group_id = self.ivars().current_group_filter.get();
            if current_group_id > 0 && !groups.iter().any(|group| group.id == current_group_id) {
                self.ivars().current_group_filter.set(0);
            }
            self.reload_native_clip_items();
        }

        fn refresh_status_menu_state_from_settings(&self) {
            for spec in native_host_status_menu_item_specs() {
                self.refresh_status_menu_action_state(spec.action);
            }
        }

        fn perform_native_settings_action(&self, action: NativeHostSettingsAction) {
            if matches!(action, NativeHostSettingsAction::Save) {
                let plan = crate::settings_model::settings_native_apply_collect_plan();
                let submitted_values = self
                    .ivars()
                    .settings_native_text_fields
                    .borrow()
                    .iter()
                    .map(|binding| {
                        let raw_value = binding.field.stringValue().to_string();
                        crate::settings_model::SettingsNativeSubmittedControlValue {
                            control_key: binding.control_key.to_string(),
                            raw_value,
                        }
                    })
                    .collect::<Vec<_>>();
                let mut submitted_values = submitted_values;
                submitted_values.extend(
                    self.ivars()
                        .settings_native_toggle_buttons
                        .borrow()
                        .iter()
                        .map(|binding| {
                            let value = binding.button.state() == NSControlStateValueOn;
                            crate::settings_model::SettingsNativeSubmittedControlValue {
                                control_key: binding.control_key.to_string(),
                                raw_value: value.to_string(),
                            }
                        }),
                );
                submitted_values.extend(
                    self.ivars()
                        .settings_native_dropdown_buttons
                        .borrow()
                        .iter()
                        .filter_map(|binding| {
                            let selected_index = binding.button.indexOfSelectedItem();
                            if selected_index < 0 {
                                return None;
                            }
                            let raw_value = binding
                                .option_values
                                .get(selected_index as usize)
                                .cloned()
                                .unwrap_or_default();
                            Some(crate::settings_model::SettingsNativeSubmittedControlValue {
                                control_key: binding.control_key.to_string(),
                                raw_value,
                            })
                        }),
                );
                if let Some(path)=self.ivars().settings_sound_path.borrow().as_ref() {
                    submitted_values.push(crate::settings_model::SettingsNativeSubmittedControlValue {control_key:"paste_sound_file".into(),raw_value:path.clone()});
                }
                let submission =
                    crate::settings_model::settings_native_collect_submission(&submitted_values);
                let json_apply = crate::settings_model::settings_native_apply_submission_to_json(
                    serde_json::json!({}),
                    &submission,
                );
                let persist_result =
                    crate::macos_app::persist_macos_native_settings_submission(&submission);
                eprintln!(
                    "ZSClip AppKit settings apply/collect submission -> {} | {} | {} | {}",
                    plan.summary_label(),
                    submission.summary_label(),
                    json_apply.summary_label(),
                    persist_result.result_name
                );
                self.refresh_main_state_after_settings_save();
                self.refresh_status_menu_state_from_settings();
                self.refresh_settings_dependencies();
                if let Some(label) = self.ivars().settings_route_label.get() {
                    let text = if persist_result.accepted && json_apply.rejected_fields.is_empty() {
                        appkit_tr("已保存", "Saved")
                    } else {
                        appkit_tr("部分设置未能保存，请检查输入。", "Some settings could not be saved. Check the entered values.")
                    };
                    label.setStringValue(&NSString::from_str(text));
                }
            }
            let result = super::dispatch_appkit_settings_action(action);
            eprintln!(
                "ZSClip AppKit settings action {} -> {}",
                action.action_name(),
                result.result_name
            );
            if action.should_close_settings_surface() {
                if let Some(window) = self.ivars().settings_window.get() {
                    window.close();
                }
            }
        }

        fn perform_native_settings_route_action(&self, tag: isize) {
            let Some(binding) = self
                .ivars()
                .settings_native_route_buttons
                .borrow()
                .iter()
                .find(|binding| binding.tag == tag)
                .cloned()
            else {
                eprintln!("ZSClip AppKit settings route action missing tag={}", tag);
                return;
            };
            if binding.action_name=="pick_paste_sound" {
                let saved=crate::macos_app::macos_native_settings_json_snapshot();
                let current=self.ivars().settings_sound_path.borrow().clone().unwrap_or_else(||saved.get("paste_success_sound_path").and_then(serde_json::Value::as_str).unwrap_or("").into());
                let request=crate::app_core::NativeFileDialogRequest {title:appkit_tr("选择提示音文件","Choose notification sound"),filter_name:"WAV",filter_pattern:"*.wav",current_path:&current};
                if let Ok(Some(path))=<crate::macos_app::MacosFileDialogHost as crate::app_core::NativeFileDialogHost>::pick_file(&crate::macos_app::MacosFileDialogHost::default(),request) {
                    *self.ivars().settings_sound_path.borrow_mut()=Some(path.clone());
                    if let Some(button)=self.ivars().settings_sound_file_button.get() {button.setTitle(&NSString::from_str(std::path::Path::new(&path).file_name().and_then(|name|name.to_str()).unwrap_or(&path)));}
                    if let Some(binding)=self.ivars().settings_native_dropdown_buttons.borrow().iter().find(|binding|binding.control_key=="paste_sound_kind") {
                        if let Some(index)=binding.option_values.iter().position(|value|value=="custom") {binding.button.selectItemAtIndex(index as _);}
                    }
                }
                return;
            }
            let result = crate::macos_app::dispatch_macos_native_settings_route_action(
                binding.route_name,
                binding.action_name,
            );
            eprintln!(
                "ZSClip AppKit settings route action {}/{} -> {}",
                binding.route_name, binding.action_name, result.result_name
            );
            if let Some(label) = self.ivars().settings_route_label.get() {
                label.setStringValue(&NSString::from_str(if result.accepted {
                    appkit_tr("操作完成", "Done")
                } else {
                    appkit_tr("操作未完成，请检查设置。", "Unable to complete this action. Check the settings.")
                }));
            }
        }

        fn perform_native_settings_control_action(&self, action: NativeHostSettingsControlAction) {
            let result = super::dispatch_appkit_settings_control_action(action);
            let applied = self.apply_native_settings_control_action(action);
            eprintln!(
                "ZSClip AppKit settings control action {} -> {} applied={}",
                action.action_name(),
                result.result_name,
                applied
            );
        }

        fn apply_native_settings_control_action(
            &self,
            action: NativeHostSettingsControlAction,
        ) -> bool {
            let Some(control_key) = action.binding_control_key() else {
                return false;
            };
            if action.role() == SettingsControlRole::Toggle {
                if let Some(binding) = self
                    .ivars()
                    .settings_native_toggle_buttons
                    .borrow()
                    .iter()
                    .find(|binding| binding.control_key == control_key)
                    .cloned()
                {
                    let next_state = if binding.button.state() == NSControlStateValueOn {
                        NSControlStateValueOff
                    } else {
                        NSControlStateValueOn
                    };
                    binding.button.setState(next_state);
                    return true;
                }
            }
            if action.role() == SettingsControlRole::Dropdown {
                if let Some(binding) = self
                    .ivars()
                    .settings_native_dropdown_buttons
                    .borrow()
                    .iter()
                    .find(|binding| binding.control_key == control_key)
                    .cloned()
                {
                    if binding.option_values.is_empty() {
                        return false;
                    }
                    let selected_index = binding.button.indexOfSelectedItem();
                    let next_index = if selected_index < 0 {
                        0
                    } else {
                        ((selected_index as usize + 1) % binding.option_values.len()) as isize
                    };
                    binding.button.selectItemAtIndex(next_index);
                    return true;
                }
            }
            false
        }

        fn perform_native_settings_platform_action(
            &self,
            action: NativeHostSettingsPlatformAction,
        ) {
            let result = super::dispatch_appkit_settings_platform_action(action);
            eprintln!(
                "ZSClip AppKit settings platform action {} -> {}",
                action.action_name(),
                result.result_name
            );
        }

        fn perform_native_dialog_action(&self, action: NativeHostDialogAction) {
            let result = self.present_native_dialog_action(action);
            eprintln!(
                "ZSClip AppKit dialog action {} -> {}",
                action.action_name(),
                result.result_name
            );
        }

        fn present_native_dialog_action(
            &self,
            action: NativeHostDialogAction,
        ) -> ProductAdapterCommandResult {
            match action {
                NativeHostDialogAction::ShowInfoMessage => {
                    Self::present_appkit_message_dialog(
                        self.mtm(),
                        action.title(),
                        action.message(),
                        NSAlertStyle::Informational,
                    );
                    ProductAdapterCommandResult {
                        accepted: true,
                        result_name: "zsclip.dialog.show_info_message".to_string(),
                    }
                }
                NativeHostDialogAction::ConfirmQuestion => {
                    let response = Self::present_appkit_confirm_dialog(
                        self.mtm(),
                        action.title(),
                        action.message(),
                    );
                    ProductAdapterCommandResult {
                        accepted: true,
                        result_name: format!(
                            "zsclip.dialog.confirm_{}",
                            Self::native_dialog_response_name(response)
                        ),
                    }
                }
            }
        }

        fn perform_native_row_action(&self, action: NativeHostRowAction) {
            if self.ivars().search_pending.get() {return;}
            if action==NativeHostRowAction::ToPhrase && self.ivars().content_preferences.get().phrase_titles_enabled {
                self.ivars().edit_save_as_phrase.set(true);
                self.present_native_edit_window(false);
                return;
            }
            if action==NativeHostRowAction::Edit {self.ivars().edit_save_as_phrase.set(false);}
            let item_id = self.ivars().selected_item_id.get();
            let result =
                crate::macos_app::dispatch_macos_native_row_action_for_item(action, item_id);
            eprintln!(
                "ZSClip AppKit row action {} item_id={} -> {}",
                action.action_name(),
                item_id,
                result.result_name
            );
            if result.accepted && matches!(action, NativeHostRowAction::Paste) {
                self.begin_native_row_paste();
            } else if result.accepted && action==NativeHostRowAction::Copy {
                let _=crate::native_feedback::notify_success(crate::native_feedback::NativeFeedbackKind::Copy,&crate::macos_app::macos_native_settings_json_snapshot());
            }
            if result.accepted
                && matches!(
                    action,
                    NativeHostRowAction::Pin | NativeHostRowAction::Delete
                )
            {
                self.reload_native_clip_items();
            }
            if matches!(action, NativeHostRowAction::Edit) {
                self.present_native_edit_window(false);
            }
        }

        fn perform_native_popup_menu_command(&self, menu_id: usize) {
            if menu_id==menu_ids::ROW_SAVE_IMAGE {
                if !self.ivars().search_pending.get() {self.present_native_save_image();}
                return;
            }
            if menu_id==NATIVE_RENAME_PHRASE_COMMAND_ID {
                if self.ivars().search_pending.get() || !self.ivars().content_preferences.get().phrase_titles_enabled {return;}
                self.ivars().edit_save_as_phrase.set(false);
                self.present_native_edit_window(false);
                if let (Some(window),Some(field))=(self.ivars().edit_window.get(),self.ivars().edit_title_field.get()) {window.makeFirstResponder(Some(field));unsafe {field.selectText(None);}}
                return;
            }
            let result = super::dispatch_appkit_menu_command_id(menu_id);
            eprintln!(
                "ZSClip AppKit popup menu command {} -> {}",
                menu_id, result.result_name
            );
            if self.perform_native_group_menu_command(menu_id) {
                return;
            }
            if let Some(action) = NativeHostRowAction::from_menu_id(menu_id) {
                self.perform_native_row_action(action);
            }
        }

        fn perform_native_group_menu_command(&self, menu_id: usize) -> bool {
            let groups = crate::db_runtime::native_clip_groups(self.active_source_category())
                .unwrap_or_default();
            if menu_id == menu_ids::ROW_GROUP_REMOVE {
                let item_id = self.ivars().selected_item_id.get();
                let result = crate::macos_app::dispatch_macos_native_remove_group(item_id);
                eprintln!(
                    "ZSClip AppKit remove group item_id={} -> {}",
                    item_id, result.result_name
                );
                self.reload_native_clip_items();
                return true;
            }
            if let Some(MainRowGroupSelection::Group { index }) =
                main_row_group_selection_for_id(menu_id)
            {
                let Some(group) = groups.get(index) else {
                    return true;
                };
                let item_id = self.ivars().selected_item_id.get();
                let result =
                    crate::macos_app::dispatch_macos_native_assign_group(item_id, group.id);
                eprintln!(
                    "ZSClip AppKit assign group item_id={} group_id={} -> {}",
                    item_id, group.id, result.result_name
                );
                self.reload_native_clip_items();
                return true;
            }

            match main_group_filter_selection_for_id(menu_id) {
                Some(MainGroupFilterSelection::All) => {
                    self.ivars().current_group_filter.set(0);
                    self.ivars().current_kind_filter.set(ClipKindFilter::All);
                    let result = crate::macos_app::dispatch_macos_native_group_filter(0);
                    eprintln!("ZSClip AppKit group filter all -> {}", result.result_name);
                    self.reload_native_clip_items();
                    true
                }
                Some(MainGroupFilterSelection::Group { index }) => {
                    let Some(group) = groups.get(index) else {
                        return true;
                    };
                    self.ivars().current_group_filter.set(group.id);
                    let result = crate::macos_app::dispatch_macos_native_group_filter(group.id);
                    eprintln!(
                        "ZSClip AppKit group filter group_id={} -> {}",
                        group.id, result.result_name
                    );
                    self.reload_native_clip_items();
                    true
                }
                Some(MainGroupFilterSelection::Kind { index }) => {
                    if let Some(filter) =
                        clip_kind_filter_options_for_tab(self.active_source_category() as usize)
                            .get(index)
                    {
                        self.ivars().current_kind_filter.set(*filter);
                        self.reload_native_clip_items();
                    }
                    true
                }
                None => false,
            }
        }

        fn select_native_row(&self, item_id: i64) {
            if item_id <= 0 {
                return;
            }
            self.ivars().selected_item_id.set(item_id);
            self.refresh_native_clip_row_selection();
            eprintln!("ZSClip AppKit row selected item_id={}", item_id);
        }

        fn reload_native_clip_items(&self) {
            self.request_native_search_page(0);
        }

        fn request_native_search_page(&self,page:usize) {
            self.ivars().search_page.set(page);
            let service=self.ivars().search_service.get_or_init(crate::native_search::NativeSearchService::new);
            service.cancel();
            self.ivars().search_due.set(Some(std::time::Instant::now()+std::time::Duration::from_millis(140)));
            self.ivars().search_pending.set(true);
            if let Some(table)=self.ivars().clip_table_view.get() {table.setEnabled(false);}
            if let Some(button)=self.ivars().previous_page_button.get() {button.setEnabled(false);}
            if let Some(button)=self.ivars().next_page_button.get() {button.setEnabled(false);}
            if let Some(window)=self.ivars().window.get() {window.setTitle(&NSString::from_str(appkit_tr("ZSClip · 搜索中…","ZSClip · Searching…")));}
        }

        fn install_native_search_timer(&self) {
            self.ivars().search_service.get_or_init(crate::native_search::NativeSearchService::new);
            if let Some(timer_class)=AnyClass::get(c"NSTimer") {
                unsafe {let _:*mut AnyObject=msg_send![timer_class,scheduledTimerWithTimeInterval:0.025_f64,
                    target:self,selector:sel!(zsclipSearchPoll:),userInfo:ptr::null_mut::<AnyObject>(),repeats:true];}
            }
        }

        fn prepare_native_screenshot_scene(&self) {
            let Ok(scene)=std::env::var("ZSCLIP_NATIVE_HOST_SCREENSHOT_SCENE") else {self.reload_native_clip_items();return;};
            if std::env::var_os("ZSCLIP_DATA_DIR").is_none() {eprintln!("ZSClip AppKit screenshot scene requires an isolated data directory");return;}
            self.cancel_native_search();
            let rows=[("项目交接清单","项目交接清单\n一、检查施工记录与设备状态。\n二、整理技术资料和待办事项。\n三、明确负责人和完成时间。".to_string()),
                ("周报摘要","本周完成安装检查、资料整理和现场协调。\n下周安排：复核参数，完成验收与归档。".to_string()),
                ("多行工作记录",(1..=40).map(|i|format!("第 {i:02} 项：检查记录完整，责任人确认后归档。\n")).collect::<String>())];
            for (title,body) in rows {
                if let Ok(outcome)=crate::db_runtime::insert_native_clipboard_text(0,&body,"项目资料") {
                    if let Some(id)=outcome.item_id {
                        if let Ok(Some(mut item))=crate::db_runtime::native_clip_item(id) {
                            item.phrase_title=title.into();let _=crate::db_runtime::insert_native_phrase_from_item(&item,"项目资料");
                        }
                    }
                }
            }
            let values=[("content_font_size","16"),("card_view","true"),("card_border","true"),("card_shadow","true"),("phrase_titles","true")]
                .into_iter().map(|(key,value)|crate::settings_model::SettingsNativeSubmittedControlValue {control_key:key.into(),raw_value:value.into()}).collect::<Vec<_>>();
            let submission=crate::settings_model::settings_native_collect_submission(&values);
            let _=crate::macos_app::persist_macos_native_settings_submission(&submission);
            self.ivars().content_preferences.set(NativeContentPreferences::from_json(&crate::macos_app::macos_native_settings_json_snapshot()));
            self.ivars().current_source_category.set(1);self.ivars().current_group_filter.set(0);self.ivars().current_kind_filter.set(ClipKindFilter::All);
            if let Some(search)=self.ivars().search_field.get() {search.setStringValue(ns_string!(""));}
            for window in [self.ivars().settings_window.get(),self.ivars().edit_window.get(),self.ivars().vv_popup_window.get()].into_iter().flatten() {window.orderOut(None);}
            *self.ivars().screenshot_scene.borrow_mut()=Some(scene);
            self.refresh_main_state_after_settings_save();
            self.refresh_native_source_tab_buttons();
            self.reload_native_clip_items();
        }

        fn fit_native_scene_window(&self,window:&NSWindow,width:f64,height:f64) {
            if let Some(screen)=window.screen() {
                let frame=screen.visibleFrame();
                let width=width.min((frame.size.width-24.0).max(320.0));
                let height=height.min((frame.size.height-24.0).max(240.0));
                unsafe {window.setFrame_display(NSRect::new(NSPoint::new(frame.origin.x+(frame.size.width-width)/2.0,frame.origin.y+(frame.size.height-height)/2.0),NSSize::new(width,height)),true);}
            }
            window.makeKeyAndOrderFront(None);
        }

        fn finish_native_screenshot_scene(&self) {
            let Some(scene)=self.ivars().screenshot_scene.borrow_mut().take() else {return;};
            if scene.starts_with("settings-") {
                if self.show_settings_screenshot_scene(&scene) {
                    if let Some(window)=self.ivars().settings_window.get() {self.fit_native_scene_window(window,980.0,708.0);}
                }
                return;
            }
            if let Some(window)=self.ivars().window.get() {self.fit_native_scene_window(window,660.0,460.0);}
            match scene.as_str() {
                "main"=>{},
                "edit"=>{self.ivars().edit_save_as_phrase.set(false);self.present_native_edit_window(false);},
                "vv"=>{self.present_native_vv_popup();return;},
                _=>{eprintln!("ZSClip AppKit unknown screenshot scene={scene}");return;}
            }
            eprintln!("ZSClip AppKit screenshot scene ready={scene}");
        }

        fn cancel_native_search(&self) {
            self.ivars().search_due.set(None);
            if let Some(service)=self.ivars().search_service.get() {service.cancel();}
            self.ivars().search_pending.set(false);
            if let Some(table)=self.ivars().clip_table_view.get() {table.setEnabled(true);}
        }

        fn poll_native_search(&self) {
            self.poll_native_vv_preview();
            self.poll_native_image_export();
            self.poll_native_row_paste();
            self.poll_native_sound_preview();
            let Some(service)=self.ivars().search_service.get() else {return;};
            if self.ivars().search_due.get().is_some_and(|due|std::time::Instant::now()>=due) {
                self.ivars().search_due.set(None);
                let generation=service.submit_page(self.active_source_category(),self.ivars().current_group_filter.get(),
                    self.ivars().current_kind_filter.get(),self.native_search_text(),self.ivars().content_preferences.get().phrase_titles_enabled,self.ivars().search_page.get());
                self.ivars().search_generation.set(generation);
            }
            let Some(result)=service.try_latest_result() else {return;};
            if result.generation!=self.ivars().search_generation.get() {return;}
            self.ivars().search_pending.set(false);
            match result.items {
                Ok(items)=>{
                    self.ivars().search_page.set(result.page_index);self.ivars().search_has_more.set(result.has_more);
                    *self.ivars().clip_items.borrow_mut()=items;
                    self.refresh_native_clip_rows();
                    if let Some(table)=self.ivars().clip_table_view.get() {table.setEnabled(true);}
                    if let Some(window)=self.ivars().window.get() {window.setTitle(ns_string!("ZSClip"));}
                    if let Some(button)=self.ivars().previous_page_button.get() {button.setEnabled(result.page_index>0);}
                    if let Some(button)=self.ivars().next_page_button.get() {button.setEnabled(result.has_more);}
                    if let Some(label)=self.ivars().page_label.get() {label.setStringValue(&NSString::from_str(&format!("{} {}",appkit_tr("第","Page"),result.page_index+1)));}
                    self.finish_auto_smoke_rows_after_search();
                    self.finish_native_screenshot_scene();
                }
                Err(error)=>{
                    // Old rows remain visible but cannot be acted on after a failed query.
                    self.ivars().search_pending.set(true);
                    if let Some(window)=self.ivars().window.get() {window.setTitle(&NSString::from_str(appkit_tr("ZSClip · 搜索失败，请重新搜索","ZSClip · Search failed; retry the query")));}
                    eprintln!("ZSClip AppKit search failed: {error}");
                }
            }
        }

        fn preview_native_sound(&self) {
            if self.ivars().sound_preview_result.borrow().is_some() {return;}
            let mut settings=crate::macos_app::macos_native_settings_json_snapshot();
            if !settings.is_object() {settings=serde_json::json!({});}
            if let Some(binding)=self.ivars().settings_native_dropdown_buttons.borrow().iter().find(|binding|binding.control_key=="paste_sound_kind") {
                if let Some(value)=binding.option_values.get(binding.button.indexOfSelectedItem().max(0) as usize) {settings["paste_success_sound_kind"]=serde_json::Value::String(value.clone());}
            }
            if let Some(path)=self.ivars().settings_sound_path.borrow().as_ref() {settings["paste_success_sound_path"]=serde_json::Value::String(path.clone());}
            if let Some(button)=self.ivars().settings_sound_preview_button.get() {button.setEnabled(false);}
            if let Some(label)=self.ivars().settings_route_label.get() {label.setStringValue(&NSString::from_str(appkit_tr("正在试听…","Playing preview…")));}
            let (sender,receiver)=std::sync::mpsc::channel();
            *self.ivars().sound_preview_result.borrow_mut()=Some(receiver);
            std::thread::spawn(move || {let result=std::panic::catch_unwind(||crate::native_feedback::preview(&settings)).unwrap_or_else(|_|Err("Sound backend failed".into()));let _=sender.send(result);});
        }

        fn poll_native_sound_preview(&self) {
            let result=self.ivars().sound_preview_result.borrow().as_ref().and_then(|receiver|receiver.try_recv().ok());
            let Some(result)=result else {return;};
            self.ivars().sound_preview_result.borrow_mut().take();
            if let Some(button)=self.ivars().settings_sound_preview_button.get() {button.setEnabled(true);}
            let text=match result {
                Ok(played)=>{eprintln!("ZSClip AppKit sound preview completed=true backend={} fallback={}",played.backend,played.used_default_fallback);
                    if played.used_default_fallback {appkit_tr("自定义声音不可用，已试听默认音效。","Custom sound unavailable; the default sound was previewed.")}else{appkit_tr("试听完成","Preview finished")}},
                Err(error)=>{eprintln!("ZSClip AppKit sound preview completed=false error={error}");appkit_tr("未能播放，请检查声音文件及系统音频输出。","Unable to play. Check the sound file and system audio output.")},
            };
            if let Some(label)=self.ivars().settings_route_label.get() {label.setStringValue(&NSString::from_str(text));}
        }

        fn begin_native_row_paste(&self) {
            let pid=self.ivars().last_external_pid.get();
            if pid<=0 || pid==std::process::id() as i32 {
                Self::present_appkit_message_dialog(self.mtm(),appkit_tr("无法粘贴","Unable to paste"),appkit_tr("请先将光标放入目标应用，再返回选择记录。内容已复制到剪贴板。","Place the cursor in the destination app, then return and select a record. The content is on the clipboard."),NSAlertStyle::Warning);
                return;
            }
            let Some(app)=NSRunningApplication::runningApplicationWithProcessIdentifier(pid) else {return;};
            let Ok(revision)=crate::db_runtime::search_protection_revision() else {return;};
            let sequence=<crate::macos_app::MacosClipboardHost as crate::app_core::ClipboardHost>::sequence_number();
            *self.ivars().pending_row_paste.borrow_mut()=Some((pid,crate::db_runtime::current_app_data_generation(),sequence,revision,std::time::Instant::now()+std::time::Duration::from_secs(2)));
            self.hide_main_window();
            if !app.activateWithOptions(NSApplicationActivationOptions::ActivateIgnoringOtherApps) {
                self.ivars().pending_row_paste.borrow_mut().take();
                eprintln!("ZSClip AppKit row paste rejected=target_activation_failed");
            }
        }

        fn poll_native_row_paste(&self) {
            if let Some(pid)=Self::appkit_frontmost_pid().filter(|pid|*pid!=std::process::id() as i32) {self.ivars().last_external_pid.set(pid);}
            let Some((pid,generation,sequence,revision,deadline))=self.ivars().pending_row_paste.borrow().clone() else {return;};
            if std::time::Instant::now()>deadline || generation!=crate::db_runtime::current_app_data_generation()
                || sequence!=<crate::macos_app::MacosClipboardHost as crate::app_core::ClipboardHost>::sequence_number()
                || crate::db_runtime::search_protection_revision().ok().as_ref()!=Some(&revision) {
                self.ivars().pending_row_paste.borrow_mut().take();
                eprintln!("ZSClip AppKit row paste rejected=stale_operation");return;
            }
            if Self::appkit_frontmost_pid()!=Some(pid) {return;}
            self.ivars().pending_row_paste.borrow_mut().take();
            let posted=Self::appkit_post_native_paste_shortcut_to_pid(pid);
            eprintln!("ZSClip AppKit row paste shortcut posted={posted} target_pid={pid}");
            if posted {let _=crate::native_feedback::notify_success(crate::native_feedback::NativeFeedbackKind::Paste,&crate::macos_app::macos_native_settings_json_snapshot());}
        }

        fn native_search_text(&self) -> String {
            self.ivars()
                .search_field
                .get()
                .map(|field| field.stringValue().to_string())
                .unwrap_or_default()
        }

        fn active_source_category(&self) -> i64 {
            native_host_source_tab_for_category(self.ivars().current_source_category.get()).category
        }

        fn select_native_source_category(&self, category: i64) {
            let normalized = native_host_source_tab_for_category(category).category;
            if self.ivars().current_source_category.get() == normalized {
                self.refresh_native_source_tab_buttons();
                return;
            }
            self.ivars().current_source_category.set(normalized);
            self.ivars().current_group_filter.set(0);
            self.ivars().current_kind_filter.set(ClipKindFilter::All);
            self.ivars().selected_item_id.set(0);
            self.refresh_native_source_tab_buttons();
            self.reload_native_clip_items();
            eprintln!("ZSClip AppKit source tab category={} selected", normalized);
        }

        fn refresh_native_source_tab_buttons(&self) {
            let category = self.active_source_category();
            if let Some(buttons) = self.ivars().source_tab_buttons.get() {
                for button in buttons {
                    button.setState(if button.tag() as i64 == category {
                        NSControlStateValueOn
                    } else {
                        NSControlStateValueOff
                    });
                }
            }
        }

        fn refresh_native_clip_rows(&self) {
            let items = self.ivars().clip_items.borrow();
            let selected_item_id = native_host_reconciled_selected_item_id(
                self.ivars().selected_item_id.get(),
                &items,
            );
            self.ivars().selected_item_id.set(selected_item_id);
            *self.ivars().clip_table_items.borrow_mut() = items.clone();
            if let Some(table_view) = self.ivars().clip_table_view.get() {
                table_view.reloadData();
            }
            self.refresh_native_clip_row_selection();
        }

        fn refresh_native_clip_row_selection(&self) {
            let selected_item_id = self.ivars().selected_item_id.get();
            if let Some(table_view) = self.ivars().clip_table_view.get() {
                if let Some(index) = self
                    .ivars()
                    .clip_table_items
                    .borrow()
                    .iter()
                    .position(|item| item.id == selected_item_id)
                {
                    if table_view.selectedRow() != index as NSInteger {
                        let indexes = NSIndexSet::indexSetWithIndex(index as NSUInteger);
                        table_view.selectRowIndexes_byExtendingSelection(&indexes, false);
                    }
                    table_view.scrollRowToVisible(index as NSInteger);
                }
            }
        }

        fn present_native_save_image(&self) {
            let item_id=self.ivars().selected_item_id.get();
            let generation=crate::db_runtime::current_app_data_generation();
            let panel=objc2_app_kit::NSSavePanel::savePanel(self.mtm());
            panel.setTitle(Some(&NSString::from_str(appkit_tr("另存为 PNG","Save as PNG"))));
            panel.setNameFieldStringValue(&NSString::from_str(&format!("ZSClip-{item_id}.png")));
            panel.setCanCreateDirectories(true);
            let types=objc2_foundation::NSArray::from_slice(&[ns_string!("png")]);
            #[allow(deprecated)]
            panel.setAllowedFileTypes(Some(&types));
            if panel.runModal()!=objc2_app_kit::NSModalResponseOK {return;}
            let Some(path)=panel.URL().and_then(|url|url.to_file_path()) else {return;};
            let (sender,receiver)=std::sync::mpsc::channel();
            *self.ivars().image_export_result.borrow_mut()=Some(receiver);
            std::thread::spawn(move ||{let _=sender.send(crate::native_image_export::save_native_item_png(item_id,&path,generation));});
        }

        fn poll_native_image_export(&self) {
            let result=self.ivars().image_export_result.borrow().as_ref().and_then(|receiver|receiver.try_recv().ok());
            let Some(result)=result else {return;};
            self.ivars().image_export_result.borrow_mut().take();
            if let Err(message)=result {
                Self::present_appkit_message_dialog(self.mtm(),appkit_tr("图片保存失败","Image save failed"),&message,NSAlertStyle::Warning);
            } else {eprintln!("ZSClip AppKit image export saved=true");}
        }

        fn native_edit_plan(&self) -> Option<NativeHostEditTextPlan> {
            let items = self.ivars().clip_items.borrow();
            let selected_item_id = match self.ivars().selected_item_id.get() {
                0 => None,
                item_id => Some(item_id),
            };
            let mut plan = native_host_edit_text_plan_for_item(&items, selected_item_id)?;
            let generation=crate::db_runtime::current_app_data_generation();
            let item=crate::db_runtime::with_shared_app_data_generation(generation,||crate::db_runtime::native_clip_item(plan.item_id))?.ok()??;
            if !matches!(item.kind,ClipKind::Text|ClipKind::Phrase) {return None;}
            plan.initial_text=item.text?;
            self.ivars().edit_data_generation.set(generation);
            Some(plan)
        }

        fn prepare_native_edit_title(&self,item_id:i64)->bool {
            let Some(Ok(item))=crate::db_runtime::with_shared_app_data_generation(self.ivars().edit_data_generation.get(),||crate::db_runtime::native_clip_item(item_id)) else {return false;};
            if item.is_none() {return false;}
            self.ivars().edit_is_phrase.set(item.as_ref().is_some_and(|item|item.kind==ClipKind::Phrase));
            let title=item.map(|item|item.phrase_title).unwrap_or_default();
            *self.ivars().edit_initial_title.borrow_mut()=title.clone();
            if let Some(field)=self.ivars().edit_title_field.get() {
                field.setStringValue(&NSString::from_str(&title));
                field.setHidden(!self.native_edit_title_enabled());
            }
            true
        }

        fn native_edit_title_enabled(&self)->bool {
            self.ivars().content_preferences.get().phrase_titles_enabled
                && (self.ivars().edit_is_phrase.get() || self.ivars().edit_save_as_phrase.get())
        }

        fn native_edit_title(&self)->String {
            if self.native_edit_title_enabled() {
                self.ivars().edit_title_field.get().map(|field|field.stringValue().to_string()).unwrap_or_default()
            } else {self.ivars().edit_initial_title.borrow().clone()}
        }

        fn present_native_edit_window(&self, auto_save: bool) {
            if let Some(window) = self.ivars().edit_window.get() {
                let Some(plan)=self.native_edit_plan() else {return;};
                {
                    if !self.prepare_native_edit_title(plan.item_id) {return;}
                    self.ivars().edit_item_id.set(plan.item_id);
                    *self.ivars().edit_initial_text.borrow_mut() = plan.initial_text.clone();
                    if let Some(edit_text_view) = self.ivars().edit_text_view.get() {
                        let text = NSString::from_str(&plan.initial_text);
                        edit_text_view.setString(&text);
                    }
                }
                if !window.isSheet() {
                    if let Some(parent) = self.ivars().window.get() {
                        unsafe { window.setParentWindow(Some(parent)) };
                        parent.beginSheet_completionHandler(window, None);
                    } else {
                        window.makeKeyAndOrderFront(None);
                    }
                }
                if let Some(edit_text_view) = self.ivars().edit_text_view.get() {
                    window.makeFirstResponder(Some(edit_text_view));
                }
                if auto_save {
                    self.perform_native_edit_save();
                }
                return;
            }

            let Some(plan) = self.native_edit_plan() else {
                return;
            };
            if !self.prepare_native_edit_title(plan.item_id) {return;}
            let mtm = self.mtm();
            let target: &AnyObject = self.as_ref();
            let window = unsafe {
                NSWindow::initWithContentRect_styleMask_backing_defer(
                    NSWindow::alloc(mtm),
                    NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(560.0, 320.0)),
                    NSWindowStyleMask::Titled | NSWindowStyleMask::Closable,
                    NSBackingStoreType::Buffered,
                    false,
                )
            };
            unsafe { window.setReleasedWhenClosed(false) };
            window.setDelegate(Some(ProtocolObject::from_ref(self)));
            let window_title = NSString::from_str(appkit_tr("编辑剪贴板", "ZSClip Edit"));
            window.setTitle(&window_title);
            let view = window
                .contentView()
                .expect("edit window must have content view");
            let title = unsafe {
                let label = NSTextField::labelWithString(
                    &NSString::from_str(appkit_tr("编辑剪贴板内容", "Edit clipboard text")),
                    mtm,
                );
                label.setFrame(NSRect::new(
                    NSPoint::new(20.0, 276.0),
                    NSSize::new(520.0, 24.0),
                ));
                label
            };
            let initial_text = NSString::from_str(&plan.initial_text);
            let phrase_title=NSTextField::new(mtm);
            phrase_title.setFrame(NSRect::new(NSPoint::new(20.0,242.0),NSSize::new(520.0,26.0)));
            phrase_title.setStringValue(&NSString::from_str(&self.ivars().edit_initial_title.borrow()));
            phrase_title.setPlaceholderString(Some(&NSString::from_str(appkit_tr("短语标题（可空，最多60个字符）","Phrase title (optional, up to 60 characters)"))));
            phrase_title.setHidden(!self.native_edit_title_enabled());
            appkit_set_accessibility_label::<NSTextField>(phrase_title.as_ref(),appkit_tr("短语标题","Phrase title"));
            let edit_text_view = unsafe {
                NSTextView::initWithFrame(
                    NSTextView::alloc(mtm),
                    NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(520.0, 164.0)),
                )
            };
            edit_text_view.setString(&initial_text);
            edit_text_view.setEditable(true);
            edit_text_view.setSelectable(true);
            edit_text_view.setRichText(false);
            edit_text_view.setAllowsUndo(true);
            edit_text_view.setFont(Some(&NSFont::systemFontOfSize(self.ivars().content_preferences.get().content_font_size as f64)));
            appkit_set_accessibility_label::<NSTextView>(
                edit_text_view.as_ref(),
                appkit_tr("剪贴板内容编辑器", "Clipboard text editor"),
            );
            let edit_text_scroller = unsafe {
                NSScrollView::initWithFrame(
                    NSScrollView::alloc(mtm),
                    NSRect::new(NSPoint::new(20.0, 70.0), NSSize::new(520.0, 164.0)),
                )
            };
            edit_text_scroller.setBorderType(NSBorderType::BezelBorder);
            edit_text_scroller.setHasVerticalScroller(true);
            edit_text_scroller.setHasHorizontalScroller(false);
            edit_text_scroller.setAutohidesScrollers(true);
            edit_text_scroller.setDocumentView(Some(&edit_text_view));
            appkit_set_accessibility_label::<NSScrollView>(
                edit_text_scroller.as_ref(),
                appkit_tr("剪贴板内容编辑区域", "Clipboard text editor scroll area"),
            );
            let save_buttons: Vec<_> = native_host_edit_text_button_specs()
                .into_iter()
                .map(|spec| {
                    appkit_button_from_spec(
                        mtm,
                        target,
                        spec,
                        appkit_edit_text_action_selector(spec.action),
                    )
                })
                .collect();
            unsafe { view.addSubview(&title) };
            unsafe { view.addSubview(&phrase_title) };
            unsafe { view.addSubview(&edit_text_scroller) };
            for button in &save_buttons {
                unsafe { view.addSubview(button) };
            }
            window.center();
            if let Some(parent) = self.ivars().window.get() {
                unsafe { window.setParentWindow(Some(parent)) };
                parent.beginSheet_completionHandler(&window, None);
            } else {
                window.makeKeyAndOrderFront(None);
            }
            window.makeFirstResponder(Some(&edit_text_view));
            self.ivars().edit_item_id.set(plan.item_id);
            *self.ivars().edit_initial_text.borrow_mut() = plan.initial_text.clone();
            self.ivars().edit_text_view.set(edit_text_view).unwrap();
            self.ivars().edit_title_field.set(phrase_title).unwrap();
            self.ivars().edit_window.set(window).unwrap();
            eprintln!("ZSClip AppKit edit window shown");

            if auto_save {
                self.perform_native_edit_save();
            }
        }

        fn perform_native_edit_save(&self) {
            let text = self
                .ivars()
                .edit_text_view
                .get()
                .map(|edit_text_view| edit_text_view.string().to_string())
                .unwrap_or_default();
            let item_id = self.ivars().edit_item_id.get();
            let title=self.native_edit_title();
            let result = crate::db_runtime::with_shared_app_data_generation(self.ivars().edit_data_generation.get(),||if self.ivars().edit_save_as_phrase.get() {
                let saved=crate::db_runtime::native_clip_item(item_id).and_then(|item| {
                    let Some(mut item)=item else {return Ok(false);};
                    if !matches!(item.kind,ClipKind::Text|ClipKind::Phrase) {return Ok(false);}
                    item.phrase_title=crate::app_core::normalize_phrase_title(&title).map_err(|_|rusqlite::Error::InvalidQuery)?;
                    if item.text.as_deref()!=Some(text.as_str()) {item.rich_text_html=None;}
                    item.text=Some(text.clone());item.preview=text.chars().take(120).collect();
                    crate::db_runtime::insert_native_phrase_from_item(&item,"ZSClip").map(|outcome|outcome.item_id.is_some())
                }).unwrap_or(false);
                ProductAdapterCommandResult {accepted:saved,result_name:"zsclip.row.to_phrase_save".into()}
            } else if self.ivars().edit_is_phrase.get() {
                ProductAdapterCommandResult {accepted:crate::db_runtime::save_native_phrase(item_id,&title,&text).unwrap_or(false),result_name:"zsclip.row.phrase_save".into()}
            } else {super::dispatch_appkit_edit_text_save(item_id, &text)})
                .unwrap_or_else(||ProductAdapterCommandResult {accepted:false,result_name:"zsclip.row.edit_stale_data".into()});
            eprintln!(
                "ZSClip AppKit edit save item_id={} text_len={} -> {}",
                item_id,
                text.chars().count(),
                result.result_name
            );
            if result.accepted {
                self.ivars().edit_save_as_phrase.set(false);
                self.reload_native_clip_items();
                if let Some(window) = self.ivars().edit_window.get() {
                    if let Some(parent) = self.ivars().window.get() {
                        if window.isSheet() {
                            parent.endSheet(window);
                        }
                    }
                    window.orderOut(None);
                }
            } else {
                let alert=NSAlert::new(self.mtm());
                alert.setMessageText(&NSString::from_str(appkit_tr("无法保存","Unable to save")));
                alert.setInformativeText(&NSString::from_str(appkit_tr("请检查标题长度与内容，记录可能已被删除或受到保护。","Check the title length and content. The record may have been removed or protected.")));
                alert.runModal();
            }
        }

        fn perform_native_edit_cancel(&self) {
            self.perform_native_edit_close_request();
        }

        fn native_edit_current_text(&self) -> String {
            self.ivars()
                .edit_text_view
                .get()
                .map(|edit_text_view| edit_text_view.string().to_string())
                .unwrap_or_default()
        }

        fn perform_native_edit_close_without_prompt(&self) {
            if let Some(window) = self.ivars().edit_window.get() {
                if let Some(parent) = self.ivars().window.get() {
                    if window.isSheet() {
                        parent.endSheet(window);
                    }
                }
                window.orderOut(None);
                eprintln!("ZSClip AppKit edit cancel");
            }
        }

        fn perform_native_edit_close_request(&self) -> bool {
            let initial_text = self.ivars().edit_initial_text.borrow().clone();
            let current_text = self.native_edit_current_text();
            let close_plan = native_host_edit_text_close_plan(&initial_text, &current_text);
            if !close_plan.requires_unsaved_confirmation && self.native_edit_title()==*self.ivars().edit_initial_title.borrow() {
                self.perform_native_edit_close_without_prompt();
                return false;
            }

            match self.present_native_edit_unsaved_changes_alert() {
                NativeDialogResponse::Yes => {
                    self.perform_native_edit_save();
                    false
                }
                NativeDialogResponse::No => {
                    self.perform_native_edit_close_without_prompt();
                    false
                }
                NativeDialogResponse::Cancel => false,
            }
        }

        fn present_native_edit_unsaved_changes_alert(&self) -> NativeDialogResponse {
            let alert = NSAlert::new(self.mtm());
            alert.setMessageText(&NSString::from_str(appkit_tr(
                "保存编辑后的剪贴板内容？",
                "Save edited clipboard text?",
            )));
            alert.setInformativeText(&NSString::from_str(appkit_tr(
                "编辑后的剪贴板内容还没有保存。",
                "The edited clipboard text has unsaved changes.",
            )));
            alert.setAlertStyle(NSAlertStyle::Warning);
            alert.addButtonWithTitle(&NSString::from_str(appkit_tr("保存", "Save")));
            alert.addButtonWithTitle(&NSString::from_str(appkit_tr("不保存", "Discard")));
            alert.addButtonWithTitle(&NSString::from_str(appkit_tr("取消", "Cancel")));
            let response = alert.runModal();
            if response == NSAlertFirstButtonReturn {
                NativeDialogResponse::Yes
            } else if response == NSAlertSecondButtonReturn {
                NativeDialogResponse::No
            } else {
                NativeDialogResponse::Cancel
            }
        }

        fn perform_native_search_text_action(&self, text: String) {
            let action = NativeHostSearchTextAction::new(text);
            self.update_clip_list_visibility(action.normalized_text());
            let result = super::dispatch_appkit_search_text_action(action);
            eprintln!("ZSClip AppKit search text -> {}", result.result_name);
        }

        fn install_vv_local_event_monitor(&self) {
            if self.ivars().vv_event_monitor.get().is_some() {
                return;
            }

            let delegate = self.retain();
            let block = RcBlock::new(move |event: NonNull<NSEvent>| -> *mut NSEvent {
                let event_ref = unsafe { event.as_ref() };
                if delegate.dismiss_native_vv_popup_for_local_mouse_event(event_ref) {
                    return event.as_ptr();
                }
                if event_ref.r#type() != NSEventType::KeyDown {
                    return event.as_ptr();
                }
                if delegate.perform_native_clip_list_key_event(event_ref) {
                    return ptr::null_mut();
                }
                if delegate.ivars().vv_cg_event_tap.get().is_some() {return event.as_ptr();}
                let transition = delegate.perform_native_vv_key_event(event_ref);
                if transition.consume_key {
                    ptr::null_mut()
                } else {
                    event.as_ptr()
                }
            });
            let event_mask = NSEventMask::KeyDown
                | NSEventMask::LeftMouseDown
                | NSEventMask::RightMouseDown
                | NSEventMask::OtherMouseDown;
            let Some(monitor) = (unsafe {
                NSEvent::addLocalMonitorForEventsMatchingMask_handler(event_mask, &block)
            }) else {
                eprintln!("ZSClip AppKit VV local event monitor unavailable");
                return;
            };
            if self.ivars().vv_event_monitor.set(monitor).is_ok() {
                eprintln!("ZSClip AppKit VV local event monitor installed");
            }
        }

        fn dismiss_native_vv_popup_for_local_mouse_event(&self, event: &NSEvent) -> bool {
            if !appkit_is_mouse_down_event(event) {
                return false;
            }
            let Some(popup_window) = self.ivars().vv_popup_window.get() else {
                return false;
            };
            if !popup_window.isVisible() {
                return false;
            }
            let event_window = event
                .window(self.mtm())
                .map(|window| Retained::<NSWindow>::as_ptr(&window));
            if event_window == Some(Retained::<NSWindow>::as_ptr(popup_window)) {
                return false;
            }
            self.dismiss_native_vv_popup("local_mouse_down")
        }

        fn dismiss_native_vv_popup(&self, reason: &str) -> bool {
            let visible=self.ivars().vv_popup_window.get().is_some_and(|window|window.isVisible());
            if let Some(window)=self.ivars().vv_popup_window.get() {window.orderOut(None);}
            self.ivars().vv_input.borrow_mut().cancel();
            crate::macos_app::reset_macos_native_vv_trigger();
            self.ivars().vv_presentation.borrow_mut().take();
            self.ivars().vv_preview_due.set(None);
            self.ivars().vv_preview_request.set(self.ivars().vv_preview_request.get().wrapping_add(1));
            self.ivars().vv_preview_result.borrow_mut().take();
            if let Some(body)=self.ivars().vv_preview_text.borrow().as_ref() {body.setString(ns_string!(""));}
            if visible {eprintln!("ZSClip AppKit VV popup dismissed reason={}",reason);}
            visible
        }

        fn perform_native_clip_list_key_event(&self, event: &NSEvent) -> bool {
            if appkit_event_has_command_modifier(event.modifierFlags())
                && appkit_event_key_text(event).eq_ignore_ascii_case("f")
            {
                self.focus_native_search_field();
                return true;
            }
            if event.keyCode() == 53 && self.hide_native_search_field() {
                return true;
            }
            if appkit_event_has_navigation_blocking_modifier(event.modifierFlags()) {
                return false;
            }
            match event.keyCode() {
                36 | 76 => {
                    self.perform_native_row_action(NativeHostRowAction::Paste);
                    true
                }
                125 => self.move_native_clip_row_selection(1),
                126 => self.move_native_clip_row_selection(-1),
                _ => false,
            }
        }

        fn move_native_clip_row_selection(&self, direction: isize) -> bool {
            let visible_item_ids = self.visible_native_clip_row_item_ids();
            if visible_item_ids.is_empty() {
                return false;
            }
            let current = self.ivars().selected_item_id.get();
            let current_index = visible_item_ids
                .iter()
                .position(|item_id| *item_id == current)
                .unwrap_or(0);
            let next_index = if direction > 0 {
                (current_index + 1).min(visible_item_ids.len().saturating_sub(1))
            } else {
                current_index.saturating_sub(1)
            };
            let next_item_id = visible_item_ids[next_index];
            self.ivars().selected_item_id.set(next_item_id);
            self.refresh_native_clip_row_selection();
            eprintln!(
                "ZSClip AppKit keyboard row selected item_id={}",
                next_item_id
            );
            true
        }

        fn visible_native_clip_row_item_ids(&self) -> Vec<i64> {
            self.ivars()
                .clip_table_items
                .borrow()
                .iter()
                .map(|item| item.id)
                .collect()
        }

        fn install_vv_global_event_monitor(&self) {
            if self.ivars().vv_global_event_monitor.get().is_some() {
                return;
            }

            let delegate = self.retain();
            let block = RcBlock::new(move |event: NonNull<NSEvent>| {
                let event_ref = unsafe { event.as_ref() };
                if appkit_is_mouse_down_event(event_ref) {
                    let _ = delegate.dismiss_native_vv_popup("global_mouse_down");
                    return;
                }
                // Observation-only monitors cannot consume keys. CGEventTap owns external VV input.
            });
            let event_mask = NSEventMask::KeyDown
                | NSEventMask::LeftMouseDown
                | NSEventMask::RightMouseDown
                | NSEventMask::OtherMouseDown;
            let Some(monitor) =
                NSEvent::addGlobalMonitorForEventsMatchingMask_handler(event_mask, &block)
            else {
                eprintln!("ZSClip AppKit VV global event monitor unavailable");
                return;
            };
            if self.ivars().vv_global_event_monitor.set(monitor).is_ok() {
                eprintln!("ZSClip AppKit VV global event monitor installed");
            }
        }

        fn install_vv_cg_event_tap_monitor(&self) {
            if self.ivars().vv_cg_event_tap.get().is_some() {
                return;
            }

            let retained_delegate = self.retain();
            let user_info = Retained::as_ptr(&retained_delegate) as *mut c_void;
            let delegate_object: Retained<AnyObject> = retained_delegate.into();
            let event_mask = (1_u64 << CGEventType::KeyDown.0) | (1_u64 << CGEventType::KeyUp.0);
            let Some(event_tap) = (unsafe {
                CGEvent::tap_create(
                    CGEventTapLocation::SessionEventTap,
                    CGEventTapPlacement::HeadInsertEventTap,
                    CGEventTapOptions::Default,
                    event_mask,
                    Some(Self::appkit_vv_cg_event_tap_callback),
                    user_info,
                )
            }) else {
                eprintln!(
                    "ZSClip AppKit VV CGEventTap unavailable; Accessibility/Input Monitoring may be required"
                );
                return;
            };

            let Some(run_loop_source) = CFMachPort::new_run_loop_source(None, Some(&event_tap), 0)
            else {
                eprintln!("ZSClip AppKit VV CGEventTap run loop source unavailable");
                return;
            };
            let Some(run_loop) = CFRunLoopGetCurrent() else {
                eprintln!("ZSClip AppKit VV CGEventTap run loop unavailable");
                return;
            };
            CFRunLoopAddSource(&run_loop, Some(&run_loop_source), unsafe {
                kCFRunLoopCommonModes
            });
            CGEvent::tap_enable(&event_tap, true);

            let _ = self.ivars().vv_cg_event_tap_delegate.set(delegate_object);
            let _ = self.ivars().vv_cg_event_tap_source.set(run_loop_source);
            if self.ivars().vv_cg_event_tap.set(event_tap).is_ok() {
                eprintln!("ZSClip AppKit VV CGEventTap monitor installed");
            }
        }

        fn install_row_context_event_monitor(&self) {
            if self.ivars().row_context_event_monitor.get().is_some() {
                return;
            }

            let delegate = self.retain();
            let block = RcBlock::new(move |event: NonNull<NSEvent>| -> *mut NSEvent {
                if delegate.perform_native_row_context_event(unsafe { event.as_ref() }) {
                    ptr::null_mut()
                } else {
                    event.as_ptr()
                }
            });
            let Some(monitor) = (unsafe {
                NSEvent::addLocalMonitorForEventsMatchingMask_handler(
                    NSEventMask::RightMouseDown,
                    &block,
                )
            }) else {
                eprintln!("ZSClip AppKit row context event monitor unavailable");
                return;
            };
            if self.ivars().row_context_event_monitor.set(monitor).is_ok() {
                eprintln!("ZSClip AppKit row context event monitor installed");
            }
        }

        fn perform_native_row_context_event(&self, event: &NSEvent) -> bool {
            let location = event.locationInWindow();
            if let Some(buttons) = self.ivars().source_tab_buttons.get() {
                for button in buttons {
                    if NSPointInRect(location, button.frame()) {
                        self.select_native_source_category(button.tag() as i64);
                        self.present_native_group_filter_popup_menu_at(location);
                        eprintln!(
                            "ZSClip AppKit source tab group menu category={}",
                            self.active_source_category()
                        );
                        return true;
                    }
                }
            }
            if let Some(table_view) = self.ivars().clip_table_view.get() {
                let table_location = unsafe { table_view.convertPoint_fromView(location, None) };
                let row = table_view.rowAtPoint(table_location);
                if row >= 0 {
                    let Some(item) = self
                        .ivars()
                        .clip_table_items
                        .borrow()
                        .get(row as usize)
                        .cloned()
                    else {
                        return false;
                    };
                    self.select_native_row(item.id);
                    self.present_native_row_popup_menu_at(location);
                    eprintln!("ZSClip AppKit row context menu item_id={}", item.id);
                    return true;
                }
            }
            false
        }

        fn perform_native_vv_key_event(&self, event: &NSEvent) -> NativeHostVvTriggerTransition {
            let key_text = event
                .charactersIgnoringModifiers()
                .map(|characters| characters.to_string())
                .unwrap_or_default();
            let target_token =
                Self::appkit_vv_target_token_for_event(event, self.native_window_target_token());
            self.perform_native_vv_trigger_input(NativeHostVvTriggerInput {
                key: Self::appkit_vv_trigger_key_from_event(&key_text, event.keyCode()),
                target_token,
                target_ready: true,
                command_modifier: Self::appkit_vv_has_command_modifier(event.modifierFlags()),
                popup_menu_active: false,
                now_ms: Self::appkit_vv_now_ms(),
            })
        }

        fn perform_native_vv_global_key_event(
            &self,
            event: &NSEvent,
        ) -> NativeHostVvTriggerTransition {
            let key_text = event
                .charactersIgnoringModifiers()
                .map(|characters| characters.to_string())
                .unwrap_or_default();
            self.perform_native_vv_trigger_input(NativeHostVvTriggerInput {
                key: Self::appkit_vv_trigger_key_from_event(&key_text, event.keyCode()),
                target_token: Self::appkit_vv_target_token_for_event(event, 0),
                target_ready: true,
                command_modifier: Self::appkit_vv_has_command_modifier(event.modifierFlags()),
                popup_menu_active: false,
                now_ms: Self::appkit_vv_now_ms(),
            })
        }

        fn perform_native_vv_cg_event(&self, event: &CGEvent) -> NativeHostVvTriggerTransition {
            let key_code=CGEvent::integer_value_field(Some(event),CGEventField::KeyboardEventKeycode) as u16;
            let down=CGEvent::r#type(Some(event))==CGEventType::KeyDown;
            let target=Self::appkit_frontmost_pid().unwrap_or(0);
            let same_target=self.ivars().vv_presentation.borrow().as_ref().is_some_and(|session|session.target_pid==target&&target>0);
            let modifiers=Self::appkit_vv_has_cg_command_modifier(CGEvent::flags(Some(event)))
                || CGEvent::flags(Some(event)).contains(CGEventFlags::MaskShift);
            let result=self.ivars().vv_input.borrow_mut().key(Self::appkit_vv_owned_key(key_code),down,modifiers,same_target);
            use crate::app_core::vv_session::VvKeyAction;
            match result.action {
                VvKeyAction::Hide=>{self.dismiss_native_vv_popup("keyboard_cancel");},
                VvKeyAction::Select(index)=>self.perform_native_vv_select(index),
                VvKeyAction::Navigate(direction)=>{
                    let count=self.ivars().vv_presentation.borrow().as_ref().map(|session|session.snapshot.items.len()).unwrap_or(0);
                    let selected=self.ivars().vv_preview_selected.get();
                    let next=if direction<0 {selected.saturating_sub(1)}else{(selected+1).min(count.saturating_sub(1))};
                    self.queue_native_vv_preview(next,0);
                },
                VvKeyAction::Scroll(direction)=>self.scroll_native_vv_preview(direction),
                VvKeyAction::None=>{},
            }
            if result.consume || result.action!=VvKeyAction::None || !down || result.repeat {
                return NativeHostVvTriggerTransition {action:NativeHostVvTriggerAction::Ignore,consume_key:result.consume};
            }
            self.perform_native_vv_trigger_input(NativeHostVvTriggerInput {
                key: Self::appkit_vv_trigger_key_from_cg_event(event),
                target_token: Self::appkit_vv_target_token_for_cg_event(event),
                target_ready: target>0 && target!=std::process::id() as i32,
                command_modifier: Self::appkit_vv_has_cg_command_modifier(CGEvent::flags(Some(
                    event,
                ))),
                popup_menu_active: false,
                now_ms: Self::appkit_vv_now_ms(),
            })
        }

        fn native_window_target_token(&self) -> u64 {
            Self::appkit_frontmost_pid().unwrap_or(0) as u64
        }

        fn perform_native_vv_trigger_demo(&self) {
            let target_token = self.native_window_target_token();
            let _ = self.perform_native_vv_key_text("v", false, target_token, 1);
            let transition = self.perform_native_vv_key_text("v", false, target_token, 2);
            eprintln!(
                "ZSClip AppKit VV trigger demo -> {:?} consume={}",
                transition.action, transition.consume_key
            );
        }

        fn perform_native_vv_key_text(
            &self,
            key_text: &str,
            command_modifier: bool,
            target_token: u64,
            now_ms: u64,
        ) -> NativeHostVvTriggerTransition {
            let key = Self::appkit_vv_trigger_key_from_text(key_text);
            self.perform_native_vv_trigger_input(NativeHostVvTriggerInput {
                key,
                target_token,
                target_ready: true,
                command_modifier,
                popup_menu_active: false,
                now_ms,
            })
        }

        fn perform_native_vv_trigger_input(
            &self,
            input: NativeHostVvTriggerInput,
        ) -> NativeHostVvTriggerTransition {
            let settings=crate::macos_app::macos_native_settings_json_snapshot();
            if !cfg!(feature="vv-paste") || !settings.get("vv_mode_enabled").and_then(serde_json::Value::as_bool).unwrap_or(true)
                || input.target_token==0 || input.target_token==std::process::id() as u64 {
                return NativeHostVvTriggerTransition {action:NativeHostVvTriggerAction::Ignore,consume_key:false};
            }
            let transition = super::dispatch_appkit_vv_trigger_key(input);
            self.handle_native_vv_trigger_transition(transition);
            transition
        }

        fn handle_native_vv_trigger_transition(&self, transition: NativeHostVvTriggerTransition) {
            match transition.action {
                NativeHostVvTriggerAction::Show { .. } => self.present_native_vv_popup(),
                NativeHostVvTriggerAction::Select { index } => self.perform_native_vv_select(index),
                NativeHostVvTriggerAction::Hide => {
                    self.dismiss_native_vv_popup("trigger_cancel");
                }
                NativeHostVvTriggerAction::Ignore => {}
            }
        }

        fn appkit_vv_trigger_key_from_text(key_text: &str) -> NativeHostVvTriggerKey {
            match key_text.chars().next() {
                Some('v' | 'V') => NativeHostVvTriggerKey::TriggerV,
                Some('\u{1b}') => NativeHostVvTriggerKey::Escape,
                Some('\u{8}' | '\u{7f}') => NativeHostVvTriggerKey::Backspace,
                Some('1'..='9') => NativeHostVvTriggerKey::Digit1To9(
                    key_text.chars().next().unwrap() as usize - '1' as usize,
                ),
                Some(_) => NativeHostVvTriggerKey::Other,
                None => NativeHostVvTriggerKey::Other,
            }
        }

        fn appkit_vv_trigger_key_from_event(
            key_text: &str,
            key_code: u16,
        ) -> NativeHostVvTriggerKey {
            match key_code {
                9 if key_text.is_empty() => NativeHostVvTriggerKey::TriggerV,
                51 => NativeHostVvTriggerKey::Backspace,
                53 => NativeHostVvTriggerKey::Escape,
                _ => Self::appkit_vv_trigger_key_from_text(key_text),
            }
        }

        fn appkit_vv_has_command_modifier(flags: NSEventModifierFlags) -> bool {
            flags.intersects(
                NSEventModifierFlags::Command
                    | NSEventModifierFlags::Control
                    | NSEventModifierFlags::Option,
            )
        }

        fn appkit_vv_has_cg_command_modifier(flags: CGEventFlags) -> bool {
            flags.intersects(
                CGEventFlags::MaskCommand | CGEventFlags::MaskControl | CGEventFlags::MaskAlternate,
            )
        }

        fn appkit_vv_target_token_for_event(_event: &NSEvent, _fallback: u64) -> u64 {
            Self::appkit_frontmost_pid().unwrap_or(0) as u64
        }

        fn appkit_vv_target_token_for_cg_event(_event: &CGEvent) -> u64 {
            Self::appkit_frontmost_pid().unwrap_or(0) as u64
        }

        fn appkit_vv_owned_key(key: u16) -> u32 {
            match key {
                18=>0x31,19=>0x32,20=>0x33,21=>0x34,23=>0x35,22=>0x36,26=>0x37,28=>0x38,25=>0x39,
                83=>0x61,84=>0x62,85=>0x63,86=>0x64,87=>0x65,88=>0x66,89=>0x67,91=>0x68,92=>0x69,
                53=>0x1b,126=>0x26,125=>0x28,116=>0x21,121=>0x22,
                _=>0x80+u32::from(key),
            }
        }

        fn appkit_vv_trigger_key_from_cg_event(event: &CGEvent) -> NativeHostVvTriggerKey {
            let mut actual_len: core::ffi::c_ulong = 0;
            let mut units = [0_u16; 8];
            unsafe {
                CGEvent::keyboard_get_unicode_string(
                    Some(event),
                    units.len() as core::ffi::c_ulong,
                    &mut actual_len,
                    units.as_mut_ptr(),
                );
            }
            let text_len = (actual_len as usize).min(units.len());
            let key_text = String::from_utf16_lossy(&units[..text_len]);
            let key_code =
                CGEvent::integer_value_field(Some(event), CGEventField::KeyboardEventKeycode)
                    as u16;
            Self::appkit_vv_trigger_key_from_event(&key_text, key_code)
        }

        fn appkit_post_native_key_event(target_pid: i32, virtual_key: u16, flags: CGEventFlags) -> bool {
            if target_pid<=0 || Self::appkit_frontmost_pid()!=Some(target_pid) || !CGPreflightPostEventAccess() {return false;}
            let Some(key_down) = CGEvent::new_keyboard_event(None, virtual_key, true) else {
                return false;
            };
            let Some(key_up) = CGEvent::new_keyboard_event(None, virtual_key, false) else {
                return false;
            };
            CGEvent::set_flags(Some(&key_down), flags);
            CGEvent::set_flags(Some(&key_up), flags);
            CGEvent::set_integer_value_field(Some(&key_down),CGEventField::EventSourceUserData,0x5a53434c4950);
            CGEvent::set_integer_value_field(Some(&key_up),CGEventField::EventSourceUserData,0x5a53434c4950);
            CGEvent::post_to_pid(target_pid,Some(&key_down));
            CGEvent::post_to_pid(target_pid,Some(&key_up));
            true
        }

        fn appkit_post_native_paste_shortcut() -> bool {
            Self::appkit_frontmost_pid().is_some_and(Self::appkit_post_native_paste_shortcut_to_pid)
        }

        fn appkit_post_native_paste_shortcut_to_pid(target_pid: i32) -> bool {
            Self::appkit_post_native_key_event(target_pid,9,CGEventFlags::MaskCommand)
        }

        unsafe extern "C-unwind" fn appkit_vv_cg_event_tap_callback(
            _proxy: CGEventTapProxy,
            event_type: CGEventType,
            event: NonNull<CGEvent>,
            user_info: *mut c_void,
        ) -> *mut CGEvent {
            if user_info.is_null() {
                return event.as_ptr();
            }

            let delegate = unsafe { &*(user_info as *const Delegate) };
            if matches!(
                event_type,
                CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput
            ) {
                if let Some(event_tap) = delegate.ivars().vv_cg_event_tap.get() {
                    CGEvent::tap_enable(event_tap, true);
                }
                return event.as_ptr();
            }
            if !matches!(event_type,CGEventType::KeyDown|CGEventType::KeyUp)
                || CGEvent::integer_value_field(Some(unsafe {event.as_ref()}),CGEventField::EventSourceUserData)==0x5a53434c4950 {
                return event.as_ptr();
            }

            let transition = delegate.perform_native_vv_cg_event(unsafe { event.as_ref() });
            if transition.consume_key {
                ptr::null_mut()
            } else {
                event.as_ptr()
            }
        }

        fn appkit_vv_now_ms() -> u64 {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
                .unwrap_or(0)
        }

        fn perform_native_vv_select(&self, index: usize) {
            let Some(session)=self.ivars().vv_presentation.borrow().clone() else {return;};
            if !cfg!(feature="vv-paste") || session.target_pid<=0 || Self::appkit_frontmost_pid()!=Some(session.target_pid) {
                eprintln!("ZSClip AppKit VV selection rejected=target_changed");
                self.dismiss_native_vv_popup("invalid_target");return;
            }
            if !CGPreflightPostEventAccess() {
                eprintln!("ZSClip AppKit VV delivery blocked=post_event_permission; allow Accessibility in System Settings");
                if let Some(label)=self.ivars().vv_status_label.borrow().as_ref() {
                    label.setStringValue(&NSString::from_str(appkit_tr("请在系统设置中允许 ZSClip 使用辅助功能后粘贴。","Allow ZSClip in System Settings → Accessibility to paste.")));
                }
                return;
            }
            let delivered=crate::db_runtime::with_shared_app_data_generation(session.snapshot.data_generation,|| {
                let item=session.snapshot.load_item(index)?;
                let write=crate::app_core::native_host_clipboard_write_for_item(&item).ok_or("VV candidate has no clipboard payload")?;
                if Self::appkit_frontmost_pid()!=Some(session.target_pid) {return Err("VV target changed before clipboard write".to_string());}
                if !crate::app_core::native_host_write_clipboard_payload_with_html::<crate::macos_app::MacosClipboardHost>(
                    &write,crate::macos_app::MacosClipboardHost::write_rich_text) {return Err("VV clipboard write failed".into());}
                session.snapshot.validate()?;
                // The non-activating panel preserves the editor and its insertion point. No speculative Backspace.
                self.dismiss_native_vv_popup("selection");
                let posted=Self::appkit_post_native_paste_shortcut_to_pid(session.target_pid);
                eprintln!("ZSClip AppKit VV native paste shortcut posted={posted} target_pid={} item_id={} backspaces=0 delivery_unverified=true",session.target_pid,item.id);
                if posted {
                    crate::native_feedback::notify_success(crate::native_feedback::NativeFeedbackKind::Paste,&crate::macos_app::macos_native_settings_json_snapshot());
                    Ok(())
                } else {Err("VV target or post-event permission changed".into())}
            }).unwrap_or_else(||Err("VV history was replaced".into()));
            if let Err(error)=delivered {
                eprintln!("ZSClip AppKit VV selection rejected: {error}");
                self.dismiss_native_vv_popup("selection_failed");
            }
        }

        fn update_clip_list_visibility(&self, query: &str) {
            if let Some(field)=self.ivars().search_field.get() {
                if field.stringValue().to_string()!=query {field.setStringValue(&NSString::from_str(query));}
            }
            self.reload_native_clip_items();
        }

        fn present_appkit_message_dialog(
            mtm: MainThreadMarker,
            title: &str,
            message: &str,
            style: NSAlertStyle,
        ) {
            let alert = NSAlert::new(mtm);
            let title = NSString::from_str(title);
            let message = NSString::from_str(message);
            alert.setMessageText(&title);
            alert.setInformativeText(&message);
            alert.setAlertStyle(style);
            alert.addButtonWithTitle(ns_string!("OK"));
            alert.runModal();
        }

        fn present_appkit_confirm_dialog(
            mtm: MainThreadMarker,
            title: &str,
            message: &str,
        ) -> NativeDialogResponse {
            let alert = NSAlert::new(mtm);
            let title = NSString::from_str(title);
            let message = NSString::from_str(message);
            alert.setMessageText(&title);
            alert.setInformativeText(&message);
            alert.setAlertStyle(NSAlertStyle::Informational);
            alert.addButtonWithTitle(ns_string!("Yes"));
            alert.addButtonWithTitle(ns_string!("No"));
            let response = alert.runModal();
            if response == NSAlertFirstButtonReturn {
                NativeDialogResponse::Yes
            } else if response == NSAlertSecondButtonReturn {
                NativeDialogResponse::No
            } else {
                NativeDialogResponse::Cancel
            }
        }

        fn native_dialog_response_name(response: NativeDialogResponse) -> &'static str {
            match response {
                NativeDialogResponse::Yes => "yes",
                NativeDialogResponse::No => "no",
                NativeDialogResponse::Cancel => "cancel",
            }
        }
    }

    pub(super) fn run(_summary: MacosHostContractSummary) -> Result<(), String> {
        let mtm = MainThreadMarker::new()
            .ok_or_else(|| "AppKit host must be launched on the macOS main thread".to_string())?;
        let app = NSApplication::sharedApplication(mtm);
        let delegate = Delegate::new(mtm);
        app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        app.run();
        Ok(())
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn real_appkit_host_is_compiled() -> bool {
    true
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn real_appkit_host_is_compiled() -> bool {
    false
}

#[cfg(target_os = "macos")]
pub(crate) fn run_real_appkit_host(summary: MacosHostContractSummary) -> Result<(), String> {
    appkit::run(summary)
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn run_real_appkit_host(_summary: MacosHostContractSummary) -> Result<(), String> {
    Err("AppKit host can only be launched on macOS".to_string())
}

pub(crate) fn dispatch_appkit_host_action(
    action: NativeHostUiAction,
) -> ProductAdapterCommandResult {
    crate::macos_app::dispatch_macos_native_host_action(action)
}

pub(crate) fn dispatch_appkit_settings_action(
    action: NativeHostSettingsAction,
) -> ProductAdapterCommandResult {
    crate::macos_app::dispatch_macos_native_settings_action(action)
}

pub(crate) fn dispatch_appkit_settings_control_action(
    action: NativeHostSettingsControlAction,
) -> ProductAdapterCommandResult {
    crate::macos_app::dispatch_macos_native_settings_control_action(action)
}

pub(crate) fn dispatch_appkit_settings_platform_action(
    action: NativeHostSettingsPlatformAction,
) -> ProductAdapterCommandResult {
    crate::macos_app::dispatch_macos_native_settings_platform_action(action)
}

pub(crate) fn dispatch_appkit_dialog_action(
    action: NativeHostDialogAction,
) -> ProductAdapterCommandResult {
    crate::macos_app::dispatch_macos_native_dialog_action(action)
}

#[cfg(all(test, not(target_os = "macos")))]
mod tests {
    use super::*;

    #[test]
    fn appkit_settings_platform_bridge_is_callable_from_non_target_tests() {
        let result = dispatch_appkit_settings_platform_action(
            NativeHostSettingsPlatformAction::OpenSourceRepository,
        );

        assert!(result.accepted);
        assert_eq!(
            result.result_name,
            "zsclip.settings.open_source_repository_failed"
        );
    }

    #[test]
    fn appkit_dialog_bridge_is_callable_from_non_target_tests() {
        let info = dispatch_appkit_dialog_action(NativeHostDialogAction::ShowInfoMessage);
        let confirm = dispatch_appkit_dialog_action(NativeHostDialogAction::ConfirmQuestion);

        assert!(info.accepted);
        assert_eq!(info.result_name, "zsclip.dialog.show_info_message");
        assert!(confirm.accepted);
        assert_eq!(confirm.result_name, "zsclip.dialog.confirm_cancel");
    }
}

pub(crate) fn dispatch_appkit_status_menu_action(
    action: NativeHostStatusMenuAction,
) -> ProductAdapterCommandResult {
    crate::macos_app::dispatch_macos_native_status_menu_action(action)
}

pub(crate) fn dispatch_appkit_menu_command_id(menu_id: usize) -> ProductAdapterCommandResult {
    crate::macos_app::dispatch_macos_native_menu_command_id(menu_id)
}

pub(crate) fn dispatch_appkit_row_action(
    action: NativeHostRowAction,
) -> ProductAdapterCommandResult {
    crate::macos_app::dispatch_macos_native_row_action(action)
}

#[cfg(target_os = "macos")]
pub(crate) fn dispatch_appkit_edit_text_save(
    item_id: i64,
    text: &str,
) -> ProductAdapterCommandResult {
    crate::macos_app::dispatch_macos_native_edit_text_save(item_id, text)
}

pub(crate) fn dispatch_appkit_search_text_action(
    action: NativeHostSearchTextAction,
) -> ProductAdapterCommandResult {
    crate::macos_app::dispatch_macos_native_search_text_action(action)
}

pub(crate) fn dispatch_appkit_vv_select_event(index: usize) -> ProductAdapterAsyncBridgeResult {
    crate::macos_app::dispatch_macos_native_vv_select_event(index)
}

#[allow(dead_code)]
pub(crate) fn dispatch_appkit_vv_trigger_key(
    input: NativeHostVvTriggerInput,
) -> NativeHostVvTriggerTransition {
    crate::macos_app::dispatch_macos_native_vv_trigger_key(input)
}

pub(crate) fn dispatch_appkit_vv_paste(index: usize) -> NativeHostVvPasteExecution {
    crate::macos_app::dispatch_macos_native_vv_paste(index)
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn dispatch_appkit_vv_paste_for_group(
    index: usize,
    group_id: i64,
) -> NativeHostVvPasteExecution {
    crate::macos_app::dispatch_macos_native_vv_paste_for_group(index, group_id)
}

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
        NSFloatingWindowLevel, NSFont, NSImage, NSLineBreakMode, NSMenu, NSMenuItem, NSPopUpButton,
        NSScrollView, NSSearchField, NSStatusBar, NSStatusBarButton, NSStatusItem, NSTabView,
        NSTabViewItem, NSTabViewType, NSTableColumn, NSTableView, NSTableViewDataSource,
        NSTableViewDelegate, NSTableViewSelectionHighlightStyle, NSTableViewStyle, NSTextAlignment,
        NSTextField, NSTextView, NSVariableStatusItemLength, NSView, NSVisualEffectBlendingMode,
        NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView, NSWindow,
        NSWindowDelegate, NSWindowStyleMask, NSWindowTitleVisibility,
    };
    use objc2_core_foundation::{
        kCFRunLoopCommonModes, CFMachPort, CFRetained, CFRunLoopAddSource, CFRunLoopGetCurrent,
        CFRunLoopSource,
    };
    use objc2_core_graphics::{
        CGEvent, CGEventField, CGEventFlags, CGEventTapLocation, CGEventTapOptions,
        CGEventTapPlacement, CGEventTapProxy, CGEventType,
    };
    use objc2_foundation::{
        ns_string, MainThreadMarker, NSIndexSet, NSInteger, NSNotification, NSObject,
        NSObjectProtocol, NSPoint, NSPointInRect, NSRect, NSSize, NSString, NSUInteger,
    };

    use crate::app_core::{
        main_group_filter_selection_for_id, main_row_group_selection_for_id, menu_ids,
        native_host_clip_row_presentation_for_projection, native_host_clip_row_specs,
        native_host_dialog_button_specs, native_host_edit_text_button_specs,
        native_host_edit_text_close_plan, native_host_edit_text_plan_for_item,
        native_host_filtered_projected_clip_item_ids,
        native_host_full_row_popup_menu_entries_for_groups,
        native_host_group_filter_label_for_groups,
        native_host_group_filter_popup_menu_entries_for_groups,
        native_host_main_action_button_specs, native_host_main_tool_button_specs,
        native_host_projected_clip_row_title, native_host_reconciled_selected_item_id,
        native_host_row_action_button_specs, native_host_row_popup_menu_input_for_projection,
        native_host_search_input_specs, native_host_settings_action_button_specs,
        native_host_settings_control_button_specs, native_host_settings_dropdown_specs,
        native_host_settings_group_button_specs, native_host_settings_page_tab_specs,
        native_host_settings_platform_button_specs, native_host_settings_section_label,
        native_host_settings_toggle_specs, native_host_status_menu_item_specs,
        native_host_vv_popup_render_plan_for_projection,
        native_popup_menu_command_macos_key_equivalent,
        native_popup_menu_command_macos_symbol_name, HostComponent, MainGroupFilterSelection,
        MainRowGroupSelection, MainVvPopupTextRole, NativeButtonStyleRole, NativeClipRowSpec,
        NativeComponentAction, NativeComponentInstanceSpec, NativeComponentSpec,
        NativeDialogResponse, NativeDropdownSpec, NativeHostClipKindIcon,
        NativeHostClipListItemProjection, NativeHostClipRowPresentation, NativeHostDialogAction,
        NativeHostEditTextAction, NativeHostEditTextPlan, NativeHostMainToolAction,
        NativeHostRowAction, NativeHostSearchTextAction, NativeHostSettingsAction,
        NativeHostSettingsControlAction, NativeHostSettingsGroupAction,
        NativeHostSettingsPlatformAction, NativeHostStatusMenuAction, NativeHostUiAction,
        NativeHostVvTriggerAction, NativeHostVvTriggerInput, NativeHostVvTriggerKey,
        NativeHostVvTriggerTransition, NativeMenuItemSpec, NativePopupMenuEntry,
        NativeSettingsPageTabKind, ProductAdapterCommandResult, SettingsControlRole,
        REQUIRED_NATIVE_HOST_STATUS_MENU_ACTIONS,
    };
    use crate::macos_app::MacosHostContractSummary;

    #[derive(Default)]
    struct AppDelegateIvars {
        window: OnceCell<Retained<NSWindow>>,
        settings_window: OnceCell<Retained<NSWindow>>,
        status_item: OnceCell<Retained<NSStatusItem>>,
        status_menu: OnceCell<Retained<NSMenu>>,
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
        edit_window: OnceCell<Retained<NSWindow>>,
        edit_text_view: OnceCell<Retained<NSTextView>>,
        edit_initial_text: RefCell<String>,
        edit_item_id: Cell<i64>,
        selected_item_id: Cell<i64>,
        current_group_filter: Cell<i64>,
        last_clipboard_sequence: Cell<u32>,
        settings_group_category: Cell<i64>,
        selected_settings_group_id: Cell<i64>,
        search_field: OnceCell<Retained<NSSearchField>>,
        settings_route_label: OnceCell<Retained<NSTextField>>,
        settings_group_name_field: OnceCell<Retained<NSTextField>>,
        settings_native_text_fields: RefCell<Vec<NativeSettingsTextFieldBinding>>,
        settings_native_toggle_buttons: RefCell<Vec<NativeSettingsToggleButtonBinding>>,
        settings_native_dropdown_buttons: RefCell<Vec<NativeSettingsDropdownButtonBinding>>,
        settings_native_route_buttons: RefCell<Vec<NativeSettingsRouteButtonBinding>>,
        settings_group_rows: OnceCell<Vec<Retained<NSButton>>>,
        clip_items: RefCell<Vec<NativeHostClipListItemProjection>>,
        clip_table_items: RefCell<Vec<NativeHostClipListItemProjection>>,
        group_filter_button: OnceCell<Retained<NSButton>>,
    }

    impl fmt::Debug for AppDelegateIvars {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("AppDelegateIvars").finish_non_exhaustive()
        }
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
                search_field.setPlaceholderString(Some(ns_string!("Search clipboard")));
                search_field.setHidden(true);
                appkit_set_view_alpha(search_field.as_ref(), 0.0);
                search_field.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable);
                appkit_set_accessibility_label::<NSSearchField>(
                    search_field.as_ref(),
                    search_spec.label(),
                );
                let clip_items = crate::macos_app::macos_native_host_projected_clip_items();
                let clip_row_height = 44.0_f64;
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
                clip_table_column.setTitle(ns_string!("Clipboard"));
                clip_table_view.addTableColumn(&clip_table_column);
                clip_table_view.setHeaderView(None);
                clip_table_view.setRowHeight(clip_row_height);
                clip_table_view.setIntercellSpacing(NSSize::new(0.0, 1.0));
                clip_table_view.setUsesAlternatingRowBackgroundColors(true);
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
                        NSPoint::new(16.0, 20.0),
                        NSSize::new(clip_list_width, 300.0),
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
                let main_buttons: Vec<_> = native_host_main_action_button_specs()
                    .into_iter()
                    .map(|spec| {
                        let title = NSString::from_str(spec.label);
                        let button = unsafe {
                            NSButton::buttonWithTitle_target_action(
                                &title,
                                Some(target),
                                Some(appkit_host_action_selector(spec.action)),
                                mtm,
                            )
                        };
                        button.setFrame(NSRect::new(
                            NSPoint::new(spec.bounds.left as f64, spec.bounds.top as f64),
                            NSSize::new(spec.width() as f64, spec.height() as f64),
                        ));
                        appkit_set_accessibility_label::<NSButton>(button.as_ref(), spec.label);
                        button
                    })
                    .collect();
                for button in &main_buttons {
                    button.setAutoresizingMask(NSAutoresizingMaskOptions::ViewMinYMargin);
                }
                let tool_buttons: Vec<_> = native_host_main_tool_button_specs()
                    .into_iter()
                    .filter(|spec| spec.action == NativeHostMainToolAction::GroupFilter)
                    .map(|spec| {
                        let title = NSString::from_str(spec.label);
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
                        appkit_set_accessibility_label::<NSButton>(button.as_ref(), spec.label);
                        button
                    })
                    .collect();
                for button in &tool_buttons {
                    button.setAutoresizingMask(NSAutoresizingMaskOptions::ViewMinYMargin);
                }
                let window = unsafe {
                    NSWindow::initWithContentRect_styleMask_backing_defer(
                        NSWindow::alloc(mtm),
                        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(640.0, 420.0)),
                        NSWindowStyleMask::Titled
                            | NSWindowStyleMask::Closable
                            | NSWindowStyleMask::Miniaturizable
                            | NSWindowStyleMask::Resizable
                            | NSWindowStyleMask::FullSizeContentView,
                        NSBackingStoreType::Buffered,
                        false,
                    )
                };
                unsafe { window.setReleasedWhenClosed(false) };
                window.setTitle(ns_string!("ZSClip"));
                window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
                window.setTitlebarAppearsTransparent(true);
                window.setHidesOnDeactivate(false);
                window.setLevel(NSFloatingWindowLevel);
                unsafe {
                    let _: () = msg_send![&*window, setMovableByWindowBackground: true];
                }
                let view = NSVisualEffectView::initWithFrame(
                    NSVisualEffectView::alloc(mtm),
                    NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(640.0, 420.0)),
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
                for button in &main_buttons {
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
                    "ZSClip AppKit native window traits always_on_top=true scale_factor={} dark_mode={}",
                    appkit_scale_factor, appkit_dark_mode
                );
                unsafe { window.setContentMinSize(NSSize::new(420.0, 300.0)) };
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
                self.refresh_native_clip_rows();

                app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
                self.install_status_item();
                self.install_clipboard_capture_timer();
                self.install_vv_local_event_monitor();
                self.install_vv_global_event_monitor();
                self.install_vv_cg_event_tap_monitor();
                self.install_row_context_event_monitor();
                #[allow(deprecated)]
                app.activateIgnoringOtherApps(true);
                self.run_auto_smoke_if_requested();
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
                self.ivars().clip_table_items.borrow().len() as NSInteger
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
                    return ptr::null_mut();
                };
                let presentation = native_host_clip_row_presentation_for_projection(&item);
                let width = table_view.bounds().size.width.max(320.0);
                Retained::autorelease_return(appkit_clip_table_cell_view(
                    self.mtm(),
                    &presentation,
                    width,
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
                self.ivars().selected_item_id.set(item.id);
                self.refresh_native_clip_row_selection();
            }
        }
    );

    fn appkit_clip_table_cell_view(
        mtm: MainThreadMarker,
        presentation: &NativeHostClipRowPresentation,
        width: f64,
    ) -> Retained<NSView> {
        let row_height = 44.0_f64;
        let cell = NSView::initWithFrame(
            NSView::alloc(mtm),
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, row_height)),
        );
        cell.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable);
        appkit_set_accessibility_label::<NSView>(cell.as_ref(), &presentation.accessibility_label);

        let kind_label = appkit_clip_table_label(
            mtm,
            presentation.kind_prefix,
            NSRect::new(NSPoint::new(10.0, 13.0), NSSize::new(62.0, 18.0)),
            11.0,
            &appkit_clip_table_kind_color(presentation.kind_icon),
        );
        kind_label.setAlignment(NSTextAlignment::Center);

        let pin_width = 36.0_f64;
        let text_left = 82.0_f64;
        let text_right_padding = 16.0_f64
            + if presentation.pin_badge.is_some() {
                pin_width
            } else {
                0.0
            };
        let text_width = (width - text_left - text_right_padding).max(160.0);
        let title_label = appkit_clip_table_label(
            mtm,
            &presentation.title,
            NSRect::new(NSPoint::new(text_left, 22.0), NSSize::new(text_width, 17.0)),
            13.0,
            &NSColor::labelColor(),
        );
        let preview_label = appkit_clip_table_label(
            mtm,
            &presentation.preview,
            NSRect::new(NSPoint::new(text_left, 5.0), NSSize::new(text_width, 16.0)),
            11.0,
            &NSColor::secondaryLabelColor(),
        );

        unsafe { cell.addSubview(&kind_label) };
        unsafe { cell.addSubview(&title_label) };
        unsafe { cell.addSubview(&preview_label) };

        if let Some(pin_badge) = presentation.pin_badge {
            let pin_label = appkit_clip_table_label(
                mtm,
                pin_badge,
                NSRect::new(
                    NSPoint::new((width - pin_width - 8.0).max(text_left), 13.0),
                    NSSize::new(pin_width, 18.0),
                ),
                11.0,
                &NSColor::controlAccentColor(),
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

    fn appkit_clip_table_kind_color(kind_icon: NativeHostClipKindIcon) -> Retained<NSColor> {
        match kind_icon {
            NativeHostClipKindIcon::Text | NativeHostClipKindIcon::Phrase => {
                NSColor::secondaryLabelColor()
            }
            NativeHostClipKindIcon::Image => NSColor::controlAccentColor(),
            NativeHostClipKindIcon::Files | NativeHostClipKindIcon::Folder => {
                NSColor::systemGrayColor()
            }
        }
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

    fn appkit_host_action_selector(action: NativeHostUiAction) -> Sel {
        match action {
            NativeHostUiAction::ToggleSearch => sel!(zsclipToggleSearch:),
            NativeHostUiAction::OpenSettings => sel!(zsclipOpenSettings:),
            NativeHostUiAction::HideWindow => sel!(zsclipHideWindow:),
            NativeHostUiAction::CloseWindow => sel!(zsclipCloseWindow:),
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
        let title = NSString::from_str(spec.label());
        let button = unsafe {
            NSButton::buttonWithTitle_target_action(&title, Some(target), Some(selector), mtm)
        };
        button.setFrame(NSRect::new(
            NSPoint::new(bounds.left as f64, bounds.top as f64),
            NSSize::new(bounds.width() as f64, bounds.height() as f64),
        ));
        appkit_set_accessibility_label::<NSButton>(button.as_ref(), spec.label());
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
            let title = NSString::from_str(spec.label());
            popup.addItemWithTitle(&title);
        } else {
            for option in spec.options {
                let title = NSString::from_str(option.label);
                popup.addItemWithTitle(&title);
            }
        }
        unsafe {
            let _: () = msg_send![&*popup, setTarget: target];
            let _: () = msg_send![&*popup, setAction: selector];
        }
        appkit_set_accessibility_label::<NSPopUpButton>(popup.as_ref(), spec.label);
        popup
    }

    fn appkit_instance_button_from_spec(
        mtm: MainThreadMarker,
        target: &AnyObject,
        spec: &NativeComponentInstanceSpec,
        selector: Sel,
    ) -> Retained<NSButton> {
        let title = NSString::from_str(&spec.label);
        let button = unsafe {
            NSButton::buttonWithTitle_target_action(&title, Some(target), Some(selector), mtm)
        };
        button.setFrame(NSRect::new(
            NSPoint::new(spec.bounds.left as f64, spec.bounds.top as f64),
            NSSize::new(spec.width() as f64, spec.height() as f64),
        ));
        appkit_set_accessibility_label::<NSButton>(button.as_ref(), &spec.label);
        button
    }

    fn appkit_clip_row_button_from_spec(
        mtm: MainThreadMarker,
        target: &AnyObject,
        spec: &NativeClipRowSpec,
        selector: Sel,
    ) -> Retained<NSButton> {
        let title = NSString::from_str(&spec.label);
        let button = unsafe {
            NSButton::buttonWithTitle_target_action(&title, Some(target), Some(selector), mtm)
        };
        button.setFrame(NSRect::new(
            NSPoint::new(spec.bounds.left as f64, spec.bounds.top as f64),
            NSSize::new(spec.width() as f64, spec.height() as f64),
        ));
        appkit_set_accessibility_label::<NSButton>(button.as_ref(), &spec.label);
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

    fn appkit_settings_scroll_tab_item(
        mtm: MainThreadMarker,
        label: &str,
    ) -> (Retained<NSTabViewItem>, Retained<NSView>) {
        let content = unsafe {
            NSView::initWithFrame(
                NSView::alloc(mtm),
                NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1088.0, 660.0)),
            )
        };
        let scroller = unsafe {
            NSScrollView::initWithFrame(
                NSScrollView::alloc(mtm),
                NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1088.0, 584.0)),
            )
        };
        scroller.setHasVerticalScroller(true);
        scroller.setHasHorizontalScroller(false);
        scroller.setAutohidesScrollers(true);
        scroller.setDocumentView(Some(&content));
        let content_label = format!("{label} settings page");
        let scroller_label = format!("{label} settings scroll area");
        appkit_set_accessibility_label::<NSView>(content.as_ref(), &content_label);
        appkit_set_accessibility_label::<NSScrollView>(scroller.as_ref(), &scroller_label);
        let item = unsafe { NSTabViewItem::initWithIdentifier(NSTabViewItem::alloc(), None) };
        let title = NSString::from_str(label);
        item.setLabel(&title);
        item.setView(Some(&scroller));
        (item, content)
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
            if let Some(window) = self.ivars().window.get() {
                window.orderOut(None);
            }
        }

        fn toggle_main_window_visibility(&self) {
            let Some(window) = self.ivars().window.get() else {
                return;
            };
            if window.isVisible() {
                window.orderOut(None);
                return;
            }
            window.makeKeyAndOrderFront(None);
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
            let result =
                crate::native_clipboard_capture::NativeClipboardCaptureService::capture_current::<
                    crate::macos_app::MacosClipboardHost,
                >(0, "macOS");
            eprintln!(
                "ZSClip AppKit clipboard capture sequence={} inserted={} item_id={:?} reason={}",
                sequence, result.inserted, result.item_id, result.reason
            );
            if result.inserted {
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
            let title = NSString::from_str(spec.label);
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
            if action.should_exit_host() {
                unsafe { NSApplication::sharedApplication(self.mtm()).terminate(None) };
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
            self.perform_native_row_action(NativeHostRowAction::Copy);
            self.perform_native_row_action(NativeHostRowAction::Edit);
            self.present_native_edit_window(true);
            #[cfg(feature = "ai-actions")]
            self.perform_native_row_action(NativeHostRowAction::TextTranslate);
            self.perform_native_vv_trigger_demo();
            self.perform_native_vv_select(0);
            #[cfg(feature = "lan-sync")]
            self.perform_native_status_menu_action(NativeHostStatusMenuAction::ToggleLanSync);

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
            let groups = crate::db_runtime::native_clip_groups(0).unwrap_or_default();
            let items = self.ivars().clip_items.borrow();
            let menu = self.build_popup_menu(
                ns_string!("Row Actions"),
                &native_host_full_row_popup_menu_entries_for_groups(
                    &groups,
                    native_host_row_popup_menu_input_for_projection(
                        &items,
                        self.ivars().selected_item_id.get(),
                        true,
                    ),
                    |label| label.to_string(),
                ),
                target,
            );
            let shown =
                menu.popUpMenuPositioningItem_atLocation_inView(None, location, Some(&view));
            eprintln!("ZSClip AppKit row popup menu shown: {}", shown);
        }

        fn present_native_group_filter_popup_menu(&self) {
            let Some(window) = self.ivars().window.get() else {
                return;
            };
            let Some(view) = window.contentView() else {
                return;
            };
            let target: &AnyObject = self.as_ref();
            let groups = crate::db_runtime::native_clip_groups(0).unwrap_or_default();
            let menu = self.build_popup_menu(
                ns_string!("Group Filter"),
                &native_host_group_filter_popup_menu_entries_for_groups(
                    &groups,
                    self.ivars().current_group_filter.get(),
                ),
                target,
            );
            let shown = menu.popUpMenuPositioningItem_atLocation_inView(
                None,
                NSPoint::new(4.0, 196.0),
                Some(&view),
            );
            eprintln!("ZSClip AppKit group filter popup menu shown: {}", shown);
        }

        fn present_native_vv_popup(&self) {
            if let Some(window) = self.ivars().vv_popup_window.get() {
                window.makeKeyAndOrderFront(None);
                return;
            }

            let mtm = self.mtm();
            let current_group_id = self.ivars().current_group_filter.get();
            let groups = crate::db_runtime::native_clip_groups(0).unwrap_or_default();
            let group_label = native_host_group_filter_label_for_groups(&groups, current_group_id);
            let items = crate::macos_app::macos_native_host_projected_clip_items_for_group(
                current_group_id,
            );
            let plan = native_host_vv_popup_render_plan_for_projection(&items, &group_label);
            let width = plan
                .text_commands
                .iter()
                .map(|command| command.rect.right)
                .max()
                .unwrap_or(360)
                .max(360);
            let height = plan
                .text_commands
                .iter()
                .map(|command| command.rect.bottom)
                .max()
                .unwrap_or(168)
                .max(168)
                + 12;
            let window = unsafe {
                NSWindow::initWithContentRect_styleMask_backing_defer(
                    NSWindow::alloc(mtm),
                    NSRect::new(
                        NSPoint::new(0.0, 0.0),
                        NSSize::new(width as f64, height as f64),
                    ),
                    NSWindowStyleMask::Borderless,
                    NSBackingStoreType::Buffered,
                    false,
                )
            };
            unsafe { window.setReleasedWhenClosed(false) };
            unsafe { window.setOpaque(false) };
            window.setHasShadow(true);
            window.setBackgroundColor(Some(&NSColor::clearColor()));
            window.setTitle(ns_string!("ZSClip VV Popup"));
            let view = window
                .contentView()
                .expect("vv popup must have content view");
            for command in &plan.text_commands {
                let title = NSString::from_str(&command.text);
                let label = unsafe { NSTextField::labelWithString(&title, mtm) };
                let rect = command.rect;
                label.setFrame(NSRect::new(
                    NSPoint::new(rect.left as f64, (height - rect.bottom) as f64),
                    NSSize::new(rect.width() as f64, rect.height() as f64),
                ));
                label.setFont(Some(&appkit_vv_popup_text_font(command.role, command.size)));
                appkit_set_accessibility_label::<NSTextField>(label.as_ref(), &command.text);
                unsafe { view.addSubview(&label) };
            }
            #[cfg(feature = "vv-paste")]
            {
                let target: &AnyObject = self.as_ref();
                for spec in crate::app_core::native_host_vv_select_specs(&plan, width, height) {
                    let action = spec.action;
                    let title = NSString::from_str(&spec.label);
                    let button = unsafe {
                        NSButton::buttonWithTitle_target_action(
                            &title,
                            Some(target),
                            Some(sel!(zsclipVvSelect:)),
                            mtm,
                        )
                    };
                    button.setFrame(NSRect::new(
                        NSPoint::new(spec.bounds.left as f64, spec.bounds.top as f64),
                        NSSize::new(spec.width() as f64, spec.height() as f64),
                    ));
                    appkit_set_accessibility_label::<NSButton>(button.as_ref(), &spec.label);
                    button.setTag(action.index as _);
                    unsafe { view.addSubview(&button) };
                }
            }
            window.center();
            window.makeKeyAndOrderFront(None);
            self.ivars().vv_popup_window.set(window).unwrap();
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
                    let title = NSString::from_str(label);
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
                    let title = NSString::from_str(label);
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
                return;
            }

            let mtm = self.mtm();
            let window = unsafe {
                NSWindow::initWithContentRect_styleMask_backing_defer(
                    NSWindow::alloc(mtm),
                    NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1160.0, 760.0)),
                    NSWindowStyleMask::Titled
                        | NSWindowStyleMask::Closable
                        | NSWindowStyleMask::Miniaturizable
                        | NSWindowStyleMask::Resizable
                        | NSWindowStyleMask::FullSizeContentView,
                    NSBackingStoreType::Buffered,
                    false,
                )
            };
            unsafe { window.setReleasedWhenClosed(false) };
            window.setTitle(ns_string!("ZSClip Settings"));
            window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
            window.setTitlebarAppearsTransparent(true);
            unsafe {
                let _: () = msg_send![&*window, setMovableByWindowBackground: true];
            }

            let title = unsafe {
                let label = NSTextField::labelWithString(ns_string!("Settings"), mtm);
                label.setFrame(NSRect::new(
                    NSPoint::new(24.0, 692.0),
                    NSSize::new(1100.0, 32.0),
                ));
                label.setFont(Some(&NSFont::systemFontOfSize(24.0)));
                label
            };
            let route = unsafe {
                let label =
                    NSTextField::labelWithString(ns_string!("zsclip.window.open_settings"), mtm);
                label.setFrame(NSRect::new(
                    NSPoint::new(24.0, 656.0),
                    NSSize::new(1100.0, 24.0),
                ));
                label.setTextColor(Some(&NSColor::secondaryLabelColor()));
                label
            };
            let view = NSVisualEffectView::initWithFrame(
                NSVisualEffectView::alloc(mtm),
                NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1160.0, 760.0)),
            );
            view.setMaterial(NSVisualEffectMaterial::WindowBackground);
            view.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
            view.setState(NSVisualEffectState::FollowsWindowActiveState);
            view.setAutoresizingMask(
                NSAutoresizingMaskOptions::ViewWidthSizable
                    | NSAutoresizingMaskOptions::ViewHeightSizable,
            );
            appkit_enable_rounded_layer(view.as_ref(), 12.0);
            window.setContentView(Some(&view));
            unsafe { view.addSubview(&title) };
            unsafe { view.addSubview(&route) };
            let _ = self.ivars().settings_route_label.set(route.clone());
            let target: &AnyObject = self.as_ref();
            let settings_tab_view = unsafe {
                NSTabView::initWithFrame(
                    NSTabView::alloc(mtm),
                    NSRect::new(NSPoint::new(20.0, 20.0), NSSize::new(1120.0, 620.0)),
                )
            };
            settings_tab_view.setTabViewType(NSTabViewType::TopTabsBezelBorder);
            let mut general_content = None;
            let mut groups_content = None;
            let mut actions_content = None;
            for spec in native_host_settings_page_tab_specs() {
                let (tab_item, tab_content) = appkit_settings_scroll_tab_item(mtm, spec.label);
                settings_tab_view.addTabViewItem(&tab_item);
                match spec.kind {
                    NativeSettingsPageTabKind::General => general_content = Some(tab_content),
                    NativeSettingsPageTabKind::Groups => groups_content = Some(tab_content),
                    NativeSettingsPageTabKind::Actions => actions_content = Some(tab_content),
                }
            }
            unsafe { view.addSubview(&settings_tab_view) };
            let general_content = general_content.expect("settings general tab spec");
            let groups_content = groups_content.expect("settings groups tab spec");
            let actions_content = actions_content.expect("settings actions tab spec");
            let view = general_content;

            let page_summaries = crate::settings_model::settings_native_page_summaries();
            let section_summaries = crate::settings_model::settings_native_section_summaries();
            let control_summaries = crate::settings_model::settings_native_control_summaries();
            let settings_json = crate::macos_app::macos_native_settings_json_snapshot();
            eprintln!(
                "ZSClip AppKit settings native page summaries count={} section summaries count={} control summaries count={}",
                page_summaries.len(),
                section_summaries.len(),
                control_summaries.len()
            );
            for (index, summary) in page_summaries.into_iter().enumerate() {
                let control_rows = section_summaries
                    .iter()
                    .filter(|section| section.page == summary.page)
                    .map(|section| section.control_rows)
                    .sum::<i32>();
                let control_count = control_summaries
                    .iter()
                    .filter(|control| control.page == summary.page)
                    .count();
                let page_name = NSString::from_str(&format!(
                    "{} ({}/{}/{})",
                    summary.label,
                    summary.section_titles.len(),
                    control_rows,
                    control_count
                ));
                let row = unsafe {
                    let label = NSTextField::labelWithString(&page_name, mtm);
                    label.setFrame(NSRect::new(
                        NSPoint::new(24.0, 608.0 - index as f64 * 32.0),
                        NSSize::new(172.0, 28.0),
                    ));
                    label
                };
                unsafe { view.addSubview(&row) };
            }
            let control_title = unsafe {
                let title = NSString::from_str(
                    native_host_settings_section_label("settings_controls")
                        .unwrap_or("Shared Controls"),
                );
                let label = NSTextField::labelWithString(&title, mtm);
                label.setFrame(NSRect::new(
                    NSPoint::new(24.0, 440.0),
                    NSSize::new(360.0, 20.0),
                ));
                label.setFont(Some(&NSFont::systemFontOfSize(12.0)));
                label
            };
            unsafe { view.addSubview(&control_title) };
            self.ivars()
                .settings_native_text_fields
                .borrow_mut()
                .clear();
            self.ivars()
                .settings_native_toggle_buttons
                .borrow_mut()
                .clear();
            self.ivars()
                .settings_native_dropdown_buttons
                .borrow_mut()
                .clear();
            self.ivars()
                .settings_native_route_buttons
                .borrow_mut()
                .clear();
            let mut toggle_controls = control_summaries
                .iter()
                .filter(|control| {
                    control.kind == crate::settings_model::SettingsNativeControlKind::Toggle
                })
                .collect::<Vec<_>>();
            toggle_controls.sort_by_key(|control| match control.key {
                "capture_enable" => 0,
                "lan_enable" => 1,
                "cloud_enable" => 2,
                _ => 10,
            });
            for (index, control) in toggle_controls.into_iter().take(8).enumerate() {
                let display = crate::settings_model::settings_native_control_display_value(
                    control,
                    &settings_json,
                )
                .unwrap_or_else(|| {
                    crate::settings_model::SettingsNativeControlDisplayValue {
                        control_key: control.key,
                        value: "false".to_string(),
                        sensitive: false,
                    }
                });
                let button_title =
                    NSString::from_str(&format!("{}: {}", control.section_title, control.label));
                let button = unsafe {
                    NSButton::buttonWithTitle_target_action(&button_title, None, None, mtm)
                };
                button.setButtonType(NSButtonType::Switch);
                let initial_value = display.value.eq_ignore_ascii_case("true");
                button.setState(if initial_value {
                    NSControlStateValueOn
                } else {
                    NSControlStateValueOff
                });
                button.setFrame(NSRect::new(
                    NSPoint::new(220.0, 608.0 - index as f64 * 26.0),
                    NSSize::new(176.0, 22.0),
                ));
                self.ivars()
                    .settings_native_toggle_buttons
                    .borrow_mut()
                    .push(NativeSettingsToggleButtonBinding {
                        control_key: display.control_key,
                        initial_value,
                        button: button.clone(),
                    });
                unsafe { view.addSubview(&button) };
            }
            for (index, control) in control_summaries
                .iter()
                .filter(|control| {
                    control.kind == crate::settings_model::SettingsNativeControlKind::TextInput
                })
                .take(12)
                .enumerate()
            {
                let display = crate::settings_model::settings_native_control_display_value(
                    control,
                    &settings_json,
                )
                .unwrap_or_else(|| {
                    crate::settings_model::SettingsNativeControlDisplayValue {
                        control_key: control.key,
                        value: String::new(),
                        sensitive: false,
                    }
                });
                let row_label_text =
                    NSString::from_str(&format!("{}: {}", control.section_title, control.label));
                let row_label = unsafe {
                    let label = NSTextField::labelWithString(&row_label_text, mtm);
                    label.setFrame(NSRect::new(
                        NSPoint::new(24.0, 414.0 - index as f64 * 26.0),
                        NSSize::new(156.0, 20.0),
                    ));
                    label.setFont(Some(&NSFont::systemFontOfSize(11.0)));
                    label.setTextColor(Some(&NSColor::secondaryLabelColor()));
                    label
                };
                unsafe { view.addSubview(&row_label) };
                if display.sensitive {
                    let secure_label = unsafe {
                        let label = NSTextField::labelWithString(ns_string!("secure"), mtm);
                        label.setFrame(NSRect::new(
                            NSPoint::new(188.0, 414.0 - index as f64 * 26.0),
                            NSSize::new(156.0, 20.0),
                        ));
                        label.setFont(Some(&NSFont::systemFontOfSize(11.0)));
                        label.setTextColor(Some(&NSColor::secondaryLabelColor()));
                        label
                    };
                    unsafe { view.addSubview(&secure_label) };
                    continue;
                }
                let field = unsafe { NSTextField::labelWithString(ns_string!(""), mtm) };
                field.setFrame(NSRect::new(
                    NSPoint::new(188.0, 410.0 - index as f64 * 26.0),
                    NSSize::new(176.0, 24.0),
                ));
                field.setEditable(true);
                field.setBezeled(true);
                if !display.value.is_empty() {
                    let value = NSString::from_str(&display.value);
                    field.setStringValue(&value);
                }
                self.ivars().settings_native_text_fields.borrow_mut().push(
                    NativeSettingsTextFieldBinding {
                        control_key: display.control_key,
                        initial_value: display.value,
                        field: field.clone(),
                    },
                );
                unsafe { view.addSubview(&field) };
            }
            for (index, (control, options)) in control_summaries
                .iter()
                .filter(|control| {
                    control.kind == crate::settings_model::SettingsNativeControlKind::Dropdown
                })
                .filter_map(|control| {
                    native_settings_dropdown_options_for_host(control, &settings_json)
                        .map(|options| (control, options))
                })
                .take(8)
                .enumerate()
            {
                let selected_label = options
                    .options
                    .get(options.selected_index)
                    .map(|option| option.label.as_str())
                    .unwrap_or("");
                let label_text =
                    NSString::from_str(&format!("{}: {}", control.section_title, selected_label));
                let row_label = unsafe {
                    let label = NSTextField::labelWithString(&label_text, mtm);
                    label.setFrame(NSRect::new(
                        NSPoint::new(430.0, 344.0 - index as f64 * 30.0),
                        NSSize::new(170.0, 20.0),
                    ));
                    label.setFont(Some(&NSFont::systemFontOfSize(11.0)));
                    label.setTextColor(Some(&NSColor::secondaryLabelColor()));
                    label
                };
                let popup = unsafe {
                    NSPopUpButton::initWithFrame_pullsDown(
                        NSPopUpButton::alloc(mtm),
                        NSRect::new(
                            NSPoint::new(610.0, 340.0 - index as f64 * 30.0),
                            NSSize::new(220.0, 24.0),
                        ),
                        false,
                    )
                };
                let mut option_values = Vec::new();
                for option in &options.options {
                    let title = NSString::from_str(&option.label);
                    popup.addItemWithTitle(&title);
                    option_values.push(option.raw_value.clone());
                }
                popup.selectItemAtIndex(options.selected_index as _);
                let initial_value = option_values
                    .get(options.selected_index)
                    .cloned()
                    .unwrap_or_default();
                self.ivars()
                    .settings_native_dropdown_buttons
                    .borrow_mut()
                    .push(NativeSettingsDropdownButtonBinding {
                        control_key: options.control_key,
                        initial_value,
                        option_values,
                        button: popup.clone(),
                    });
                unsafe { view.addSubview(&row_label) };
                unsafe { view.addSubview(&popup) };
            }
            for (index, (control, route_name, action_name)) in control_summaries
                .iter()
                .filter(|control| {
                    control.kind == crate::settings_model::SettingsNativeControlKind::Button
                })
                .filter_map(|control| {
                    let route = control.route?;
                    if route.kind != crate::settings_model::SettingsNativeControlRouteKind::Action {
                        return None;
                    }
                    route
                        .action_name
                        .map(|action_name| (control, route.route_name, action_name))
                })
                .take(10)
                .enumerate()
            {
                let tag = 10_000 + index as isize;
                let title =
                    NSString::from_str(&format!("{}: {}", control.section_title, control.label));
                let button = unsafe {
                    NSButton::buttonWithTitle_target_action(
                        &title,
                        Some(target),
                        Some(sel!(zsclipSettingsNativeRouteAction:)),
                        mtm,
                    )
                };
                button.setFrame(NSRect::new(
                    NSPoint::new(900.0, 608.0 - index as f64 * 26.0),
                    NSSize::new(220.0, 22.0),
                ));
                button.setTag(tag);
                self.ivars()
                    .settings_native_route_buttons
                    .borrow_mut()
                    .push(NativeSettingsRouteButtonBinding {
                        tag,
                        route_name,
                        action_name,
                    });
                unsafe { view.addSubview(&button) };
            }

            let view = groups_content;
            let group_title = unsafe {
                let title = NSString::from_str(
                    native_host_settings_section_label("group_selector")
                        .unwrap_or("Group Management"),
                );
                let label = NSTextField::labelWithString(&title, mtm);
                label.setFrame(NSRect::new(
                    NSPoint::new(430.0, 608.0),
                    NSSize::new(420.0, 24.0),
                ));
                label.setFont(Some(&NSFont::systemFontOfSize(15.0)));
                label
            };
            let name_field = unsafe { NSTextField::labelWithString(ns_string!("新分组"), mtm) };
            name_field.setFrame(NSRect::new(
                NSPoint::new(430.0, 574.0),
                NSSize::new(220.0, 28.0),
            ));
            name_field.setEditable(true);
            name_field.setBezeled(true);
            let group_buttons: Vec<_> = native_host_settings_group_button_specs()
                .into_iter()
                .map(|spec| {
                    appkit_button_from_spec(
                        mtm,
                        target,
                        spec,
                        appkit_settings_group_action_selector(spec.action),
                    )
                })
                .collect();

            let group_rows: Vec<_> = (0..5)
                .map(|index| {
                    let row = unsafe {
                        NSButton::buttonWithTitle_target_action(
                            ns_string!(""),
                            Some(target),
                            Some(sel!(zsclipSelectSettingsGroup:)),
                            mtm,
                        )
                    };
                    row.setFrame(NSRect::new(
                        NSPoint::new(430.0, 536.0 - index as f64 * 30.0),
                        NSSize::new(424.0, 26.0),
                    ));
                    row.setTag(0);
                    row.setHidden(true);
                    row
                })
                .collect();
            unsafe { view.addSubview(&group_title) };
            unsafe { view.addSubview(&name_field) };
            for row in &group_rows {
                unsafe { view.addSubview(row) };
            }
            for button in &group_buttons {
                unsafe { view.addSubview(button) };
            }
            self.ivars().settings_group_name_field.set(name_field).ok();
            self.ivars().settings_group_rows.set(group_rows).ok();

            let view = actions_content;
            let settings_toggle_buttons: Vec<_> = native_host_settings_toggle_specs()
                .into_iter()
                .map(|spec| {
                    appkit_switch_from_spec(
                        mtm,
                        target,
                        spec,
                        appkit_settings_control_action_selector(spec.action),
                    )
                })
                .collect();
            let settings_dropdown_buttons: Vec<_> = native_host_settings_dropdown_specs()
                .into_iter()
                .map(|spec| {
                    appkit_dropdown_from_spec(
                        mtm,
                        target,
                        spec,
                        appkit_settings_control_action_selector(spec.action),
                    )
                })
                .collect();
            let _settings_control_compat_specs = native_host_settings_control_button_specs();
            let platform_action_buttons: Vec<_> = native_host_settings_platform_button_specs()
                .into_iter()
                .map(|spec| {
                    appkit_button_from_spec(
                        mtm,
                        target,
                        spec,
                        appkit_settings_platform_action_selector(spec.action),
                    )
                })
                .collect();
            let dialog_buttons: Vec<_> = native_host_dialog_button_specs()
                .into_iter()
                .map(|spec| {
                    appkit_button_from_spec(
                        mtm,
                        target,
                        spec,
                        appkit_dialog_action_selector(spec.action),
                    )
                })
                .collect();
            let settings_action_buttons: Vec<_> = native_host_settings_action_button_specs()
                .into_iter()
                .map(|spec| {
                    appkit_button_from_spec(
                        mtm,
                        target,
                        spec,
                        appkit_settings_action_selector(spec.action),
                    )
                })
                .collect();
            for button in &settings_toggle_buttons {
                unsafe { view.addSubview(button) };
            }
            for button in &settings_dropdown_buttons {
                unsafe { view.addSubview(button) };
            }
            for button in &platform_action_buttons {
                unsafe { view.addSubview(button) };
            }
            for button in &dialog_buttons {
                unsafe { view.addSubview(button) };
            }
            for button in &settings_action_buttons {
                unsafe { view.addSubview(button) };
            }

            window.center();
            window.makeKeyAndOrderFront(None);
            self.ivars().settings_window.set(window).unwrap();
            self.refresh_settings_group_rows();
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
            let Some(rows) = self.ivars().settings_group_rows.get() else {
                return;
            };
            let groups = self.settings_groups();
            let mut selected_id = self.ivars().selected_settings_group_id.get();
            if selected_id == 0 || !groups.iter().any(|group| group.id == selected_id) {
                selected_id = groups.first().map(|group| group.id).unwrap_or_default();
                self.ivars().selected_settings_group_id.set(selected_id);
            }
            for (index, row) in rows.iter().enumerate() {
                if let Some(group) = groups.get(index) {
                    let prefix = if group.id == selected_id { "> " } else { "  " };
                    let label = NSString::from_str(&format!("{}{}", prefix, group.name));
                    row.setTitle(&label);
                    row.setTag(group.id as isize);
                    row.setHidden(false);
                } else {
                    row.setTitle(ns_string!(""));
                    row.setTag(0);
                    row.setHidden(true);
                }
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
            if category != 0 {
                return;
            }
            if deleted_group_id == Some(self.ivars().current_group_filter.get()) {
                self.ivars().current_group_filter.set(0);
            }
            self.reload_native_clip_items();
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
            let result = crate::macos_app::dispatch_macos_native_settings_route_action(
                binding.route_name,
                binding.action_name,
            );
            eprintln!(
                "ZSClip AppKit settings route action {}/{} -> {}",
                binding.route_name, binding.action_name, result.result_name
            );
            if let Some(label) = self.ivars().settings_route_label.get() {
                label.setStringValue(&NSString::from_str(&result.result_name));
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
                let posted = Self::appkit_post_native_paste_shortcut();
                eprintln!("ZSClip AppKit row paste shortcut posted={}", posted);
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
            let groups = crate::db_runtime::native_clip_groups(0).unwrap_or_default();
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
            let items = crate::macos_app::macos_native_host_projected_clip_items_for_group(
                self.ivars().current_group_filter.get(),
            );
            *self.ivars().clip_items.borrow_mut() = items;
            self.refresh_native_clip_rows();
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

        fn native_edit_plan(&self) -> Option<NativeHostEditTextPlan> {
            let items = self.ivars().clip_items.borrow();
            let selected_item_id = match self.ivars().selected_item_id.get() {
                0 => None,
                item_id => Some(item_id),
            };
            let mut plan = native_host_edit_text_plan_for_item(&items, selected_item_id)?;
            if let Ok(Some(text)) = crate::db_runtime::item_text(plan.item_id) {
                plan.initial_text = text;
            }
            Some(plan)
        }

        fn present_native_edit_window(&self, auto_save: bool) {
            if let Some(window) = self.ivars().edit_window.get() {
                if let Some(plan) = self.native_edit_plan() {
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
            let window_title = NSString::from_str(&format!("ZSClip Edit - {}", plan.title));
            window.setTitle(&window_title);
            let view = window
                .contentView()
                .expect("edit window must have content view");
            let title = unsafe {
                let label = NSTextField::labelWithString(ns_string!("Edit clipboard text"), mtm);
                label.setFrame(NSRect::new(
                    NSPoint::new(20.0, 276.0),
                    NSSize::new(520.0, 24.0),
                ));
                label
            };
            let initial_text = NSString::from_str(&plan.initial_text);
            let edit_text_view = unsafe {
                NSTextView::initWithFrame(
                    NSTextView::alloc(mtm),
                    NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(520.0, 198.0)),
                )
            };
            edit_text_view.setString(&initial_text);
            edit_text_view.setEditable(true);
            edit_text_view.setSelectable(true);
            edit_text_view.setRichText(false);
            edit_text_view.setAllowsUndo(true);
            edit_text_view.setFont(Some(&NSFont::systemFontOfSize(13.0)));
            appkit_set_accessibility_label::<NSTextView>(
                edit_text_view.as_ref(),
                "Clipboard text editor",
            );
            let edit_text_scroller = unsafe {
                NSScrollView::initWithFrame(
                    NSScrollView::alloc(mtm),
                    NSRect::new(NSPoint::new(20.0, 70.0), NSSize::new(520.0, 198.0)),
                )
            };
            edit_text_scroller.setBorderType(NSBorderType::BezelBorder);
            edit_text_scroller.setHasVerticalScroller(true);
            edit_text_scroller.setHasHorizontalScroller(false);
            edit_text_scroller.setAutohidesScrollers(true);
            edit_text_scroller.setDocumentView(Some(&edit_text_view));
            appkit_set_accessibility_label::<NSScrollView>(
                edit_text_scroller.as_ref(),
                "Clipboard text editor scroll area",
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
            let result = super::dispatch_appkit_edit_text_save(item_id, &text);
            eprintln!(
                "ZSClip AppKit edit save item_id={} text_len={} -> {}",
                item_id,
                text.chars().count(),
                result.result_name
            );
            if result.accepted {
                self.reload_native_clip_items();
                if let Some(window) = self.ivars().edit_window.get() {
                    if let Some(parent) = self.ivars().window.get() {
                        if window.isSheet() {
                            parent.endSheet(window);
                        }
                    }
                    window.orderOut(None);
                }
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
            if !close_plan.requires_unsaved_confirmation {
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
            alert.setMessageText(ns_string!("Save edited clipboard text?"));
            alert.setInformativeText(ns_string!("The edited clipboard text has unsaved changes."));
            alert.setAlertStyle(NSAlertStyle::Warning);
            alert.addButtonWithTitle(ns_string!("Save"));
            alert.addButtonWithTitle(ns_string!("Discard"));
            alert.addButtonWithTitle(ns_string!("Cancel"));
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
            let Some(window) = self.ivars().vv_popup_window.get() else {
                return false;
            };
            if !window.isVisible() {
                return false;
            }
            window.orderOut(None);
            eprintln!("ZSClip AppKit VV popup dismissed reason={}", reason);
            true
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
                let transition = delegate.perform_native_vv_global_key_event(event_ref);
                if transition.consume_key {
                    eprintln!("ZSClip AppKit VV global monitor cannot consume external key");
                }
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
            let event_mask = 1_u64 << CGEventType::KeyDown.0;
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
            if let Some(table_view) = self.ivars().clip_table_view.get() {
                let location = event.locationInWindow();
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
                target_token: Self::appkit_vv_target_token_for_event(event, 2),
                target_ready: true,
                command_modifier: Self::appkit_vv_has_command_modifier(event.modifierFlags()),
                popup_menu_active: false,
                now_ms: Self::appkit_vv_now_ms(),
            })
        }

        fn perform_native_vv_cg_event(&self, event: &CGEvent) -> NativeHostVvTriggerTransition {
            self.perform_native_vv_trigger_input(NativeHostVvTriggerInput {
                key: Self::appkit_vv_trigger_key_from_cg_event(event),
                target_token: Self::appkit_vv_target_token_for_cg_event(event),
                target_ready: true,
                command_modifier: Self::appkit_vv_has_cg_command_modifier(CGEvent::flags(Some(
                    event,
                ))),
                popup_menu_active: false,
                now_ms: Self::appkit_vv_now_ms(),
            })
        }

        fn native_window_target_token(&self) -> u64 {
            self.ivars()
                .window
                .get()
                .map(|window| Retained::<NSWindow>::as_ptr(window) as usize as u64)
                .unwrap_or(1)
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
            let transition = super::dispatch_appkit_vv_trigger_key(input);
            self.handle_native_vv_trigger_transition(transition);
            transition
        }

        fn handle_native_vv_trigger_transition(&self, transition: NativeHostVvTriggerTransition) {
            match transition.action {
                NativeHostVvTriggerAction::Show { .. } => self.present_native_vv_popup(),
                NativeHostVvTriggerAction::Select { index } => self.perform_native_vv_select(index),
                NativeHostVvTriggerAction::Hide => {
                    if let Some(window) = self.ivars().vv_popup_window.get() {
                        window.orderOut(None);
                    }
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

        fn appkit_vv_target_token_for_event(event: &NSEvent, fallback: u64) -> u64 {
            let window_number = event.windowNumber();
            if window_number > 0 {
                (window_number as u64) | (1_u64 << 63)
            } else {
                fallback
            }
        }

        fn appkit_vv_target_token_for_cg_event(event: &CGEvent) -> u64 {
            let pid =
                CGEvent::integer_value_field(Some(event), CGEventField::EventTargetUnixProcessID);
            if pid > 0 {
                (pid as u64) | (1_u64 << 62)
            } else {
                3
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

        fn appkit_post_native_key_event(virtual_key: u16, flags: CGEventFlags) -> bool {
            let Some(key_down) = CGEvent::new_keyboard_event(None, virtual_key, true) else {
                return false;
            };
            CGEvent::set_flags(Some(&key_down), flags);
            CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&key_down));

            let Some(key_up) = CGEvent::new_keyboard_event(None, virtual_key, false) else {
                return false;
            };
            CGEvent::set_flags(Some(&key_up), flags);
            CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&key_up));
            true
        }

        fn appkit_post_native_paste_shortcut() -> bool {
            Self::appkit_post_native_key_event(9, CGEventFlags::MaskCommand)
        }

        fn appkit_post_native_delete_backspaces(backspaces: u8) -> u8 {
            let mut posted = 0;
            for _ in 0..backspaces {
                if Self::appkit_post_native_key_event(51, CGEventFlags::empty()) {
                    posted += 1;
                }
            }
            posted
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
            if event_type != CGEventType::KeyDown {
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
            let result = super::dispatch_appkit_vv_select_event(index);
            eprintln!("ZSClip AppKit VV select {} -> {}", index, result.event_name);
            let paste = super::dispatch_appkit_vv_paste_for_group(
                index,
                self.ivars().current_group_filter.get(),
            );
            eprintln!(
                "ZSClip AppKit VV paste {} -> {} accepted={} kind={}",
                index,
                paste.result_name,
                paste.accepted,
                paste.clipboard_kind.unwrap_or("none")
            );
            if paste.accepted && paste.backspaces > 0 {
                let deleted = Self::appkit_post_native_delete_backspaces(paste.backspaces);
                eprintln!(
                    "ZSClip AppKit VV delete backspaces requested={} posted={}",
                    paste.backspaces, deleted
                );
            }
            if paste.accepted && paste.paste_shortcut_sent {
                let posted = Self::appkit_post_native_paste_shortcut();
                eprintln!("ZSClip AppKit VV native paste shortcut posted={}", posted);
            }
            if let Some(window) = self.ivars().vv_popup_window.get() {
                window.orderOut(None);
            }
        }

        fn update_clip_list_visibility(&self, query: &str) {
            let items = self.ivars().clip_items.borrow();
            let visible_ids = native_host_filtered_projected_clip_item_ids(&items, query);
            let visible_items = items
                .iter()
                .filter(|item| visible_ids.contains(&item.id))
                .cloned()
                .collect::<Vec<_>>();
            let selected_item_id = native_host_reconciled_selected_item_id(
                self.ivars().selected_item_id.get(),
                &visible_items,
            );
            self.ivars().selected_item_id.set(selected_item_id);
            *self.ivars().clip_table_items.borrow_mut() = visible_items;
            if let Some(table_view) = self.ivars().clip_table_view.get() {
                table_view.reloadData();
            }
            self.refresh_native_clip_row_selection();
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

pub(crate) fn dispatch_appkit_vv_paste_for_group(
    index: usize,
    group_id: i64,
) -> NativeHostVvPasteExecution {
    crate::macos_app::dispatch_macos_native_vv_paste_for_group(index, group_id)
}

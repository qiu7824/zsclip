use crate::app_core::{
    command_ids, main_menu_command_intent_for_id, native_host_default_clip_list_projection,
    product_ai_capability_catalog, ApplicationEvent, Command, CommandPayload, CommandScope,
    MainMenuCommandIntent, MainRowMenuAction, MainTrayMenuAction, NativeHostClipListItemProjection,
    ProductAdapterAsyncBridgeResult, ProductAdapterCommandResult, ProductAdapterHost,
    ProductAdapterIdentity, ProductAdapterProjectedState, ProductAdapterSettingsSnapshot,
    ProductAiCapability, ProductAiExecutionPlan,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsclipProductSettingsSnapshot {
    pub(crate) clipboard_capture_enabled: bool,
    pub(crate) grouping_enabled: bool,
    pub(crate) ai_clean_enabled: bool,
    pub(crate) image_ocr_provider: String,
    pub(crate) text_translate_provider: String,
    pub(crate) cloud_sync_enabled: bool,
    pub(crate) lan_sync_enabled: bool,
    pub(crate) revision: u64,
}

impl ZsclipProductSettingsSnapshot {
    pub(crate) fn new(
        clipboard_capture_enabled: bool,
        grouping_enabled: bool,
        ai_clean_enabled: bool,
        image_ocr_provider: impl Into<String>,
        text_translate_provider: impl Into<String>,
        cloud_sync_enabled: bool,
        lan_sync_enabled: bool,
        revision: u64,
    ) -> Self {
        Self {
            clipboard_capture_enabled,
            grouping_enabled,
            ai_clean_enabled,
            image_ocr_provider: image_ocr_provider.into(),
            text_translate_provider: text_translate_provider.into(),
            cloud_sync_enabled,
            lan_sync_enabled,
            revision,
        }
    }
}

impl Default for ZsclipProductSettingsSnapshot {
    fn default() -> Self {
        Self::new(true, true, false, "off", "off", false, false, 0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsclipProductSnapshot {
    pub(crate) tab_index: usize,
    pub(crate) visible_items: usize,
    pub(crate) selected_items: usize,
    pub(crate) search_text: String,
    pub(crate) record_count: usize,
    pub(crate) phrase_count: usize,
    pub(crate) native_clip_items: Vec<NativeHostClipListItemProjection>,
    pub(crate) settings: ZsclipProductSettingsSnapshot,
}

impl Default for ZsclipProductSnapshot {
    fn default() -> Self {
        Self {
            tab_index: 0,
            visible_items: 0,
            selected_items: 0,
            search_text: String::new(),
            record_count: 0,
            phrase_count: 0,
            native_clip_items: default_native_clip_items(),
            settings: ZsclipProductSettingsSnapshot::default(),
        }
    }
}

fn default_native_clip_items() -> Vec<NativeHostClipListItemProjection> {
    native_host_default_clip_list_projection()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsclipProductCommandRecord {
    pub(crate) command_id: &'static str,
    pub(crate) result_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct ZsclipProductCommandRoute {
    pub(crate) family_name: &'static str,
    pub(crate) result_name: &'static str,
    pub(crate) execution_owner: &'static str,
    pub(crate) requires_selection: bool,
    pub(crate) ai_capability_id: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct ZsclipProductEventRoute {
    pub(crate) event_name: &'static str,
    pub(crate) product_effect_name: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct ZsclipProductAdapterManifest {
    pub(crate) product_id: &'static str,
    pub(crate) command_routes: Vec<ZsclipProductCommandRoute>,
    pub(crate) event_routes: Vec<ZsclipProductEventRoute>,
    pub(crate) ai_capability_ids: Vec<&'static str>,
    pub(crate) ai_provider_names: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZsclipProductAdapter {
    snapshot: ZsclipProductSnapshot,
    command_records: Vec<ZsclipProductCommandRecord>,
    bridged_event_names: Vec<String>,
    ai_action_names: Vec<&'static str>,
}

impl ZsclipProductAdapter {
    pub(crate) fn new(snapshot: ZsclipProductSnapshot) -> Self {
        Self {
            snapshot,
            command_records: Vec::new(),
            bridged_event_names: Vec::new(),
            ai_action_names: Vec::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn command_records(&self) -> &[ZsclipProductCommandRecord] {
        &self.command_records
    }

    #[cfg(test)]
    pub(crate) fn bridged_event_names(&self) -> &[String] {
        &self.bridged_event_names
    }

    #[cfg(test)]
    pub(crate) fn ai_action_names(&self) -> &[&'static str] {
        &self.ai_action_names
    }

    fn product_state_name(&self) -> &'static str {
        if !self.snapshot.search_text.trim().is_empty() {
            "search_results"
        } else if self.snapshot.tab_index == 1 {
            "phrases"
        } else {
            "clipboard_records"
        }
    }

    fn result_name_for_command(&self, command: &Command) -> Option<&'static str> {
        if command.scope != CommandScope::Window {
            return None;
        }
        if command.id == command_ids::TOGGLE_SEARCH {
            Some("zsclip.window.toggle_search")
        } else if command.id == command_ids::UPDATE_SEARCH_TEXT {
            match command.payload {
                CommandPayload::Text(_) => Some("zsclip.window.search_text_update"),
                _ => None,
            }
        } else if command.id == command_ids::OPEN_SETTINGS {
            Some("zsclip.window.open_settings")
        } else if command.id == command_ids::SAVE_SETTINGS {
            Some("zsclip.settings.save")
        } else if command.id == command_ids::CLOSE_SETTINGS {
            Some("zsclip.settings.close")
        } else if command.id == command_ids::OPEN_SETTINGS_CONFIG {
            Some("zsclip.settings.open_config")
        } else if command.id == command_ids::OPEN_SETTINGS_DROPDOWN {
            Some("zsclip.settings.open_dropdown")
        } else if command.id == command_ids::TOGGLE_SETTINGS_CONTROL {
            Some("zsclip.settings.toggle_control")
        } else if command.id == command_ids::HIDE_WINDOW {
            Some("zsclip.window.hide")
        } else if command.id == command_ids::CLOSE_WINDOW {
            Some("zsclip.window.close")
        } else if command.id == command_ids::INVOKE_MAIN_MENU_COMMAND {
            match command.payload {
                CommandPayload::ControlId(id) if id >= 0 => {
                    self.result_name_for_menu_id(id as usize)
                }
                _ => None,
            }
        } else {
            None
        }
    }

    fn result_name_for_menu_id(&self, id: usize) -> Option<&'static str> {
        match main_menu_command_intent_for_id(id)? {
            MainMenuCommandIntent::Tray(action) => Some(result_name_for_tray_action(action)),
            MainMenuCommandIntent::RowPaste => Some("zsclip.row.paste"),
            MainMenuCommandIntent::RowAction(action) => Some(result_name_for_row_action(action)),
            MainMenuCommandIntent::AssignRowGroup { .. } => Some("zsclip.row.assign_group"),
            MainMenuCommandIntent::GroupFilterAll => Some("zsclip.group_filter.all"),
            MainMenuCommandIntent::GroupFilter { .. } => Some("zsclip.group_filter.select"),
            MainMenuCommandIntent::GroupKindFilter { .. } => Some("zsclip.group_filter.kind"),
        }
    }
}

#[allow(dead_code)]
pub(crate) fn zsclip_product_adapter_manifest() -> ZsclipProductAdapterManifest {
    let ai_catalog = product_ai_capability_catalog();
    let mut ai_provider_names = Vec::new();
    for capability in ai_catalog {
        let provider_name = capability.provider.provider_name();
        if !ai_provider_names.contains(&provider_name) {
            ai_provider_names.push(provider_name);
        }
    }

    ZsclipProductAdapterManifest {
        product_id: "zsclip",
        command_routes: zsclip_product_command_routes(),
        event_routes: zsclip_product_event_routes(),
        ai_capability_ids: ai_catalog.iter().map(|capability| capability.id).collect(),
        ai_provider_names,
    }
}

#[allow(dead_code)]
pub(crate) fn zsclip_product_command_routes() -> Vec<ZsclipProductCommandRoute> {
    vec![
        command_route(
            "window",
            "zsclip.window.toggle_search",
            "native_window",
            false,
            None,
        ),
        command_route(
            "window",
            "zsclip.window.search_text_update",
            "product_adapter",
            false,
            None,
        ),
        command_route(
            "window",
            "zsclip.window.open_settings",
            "native_window",
            false,
            None,
        ),
        command_route("window", "zsclip.window.hide", "native_window", false, None),
        command_route(
            "window",
            "zsclip.window.close",
            "native_window",
            false,
            None,
        ),
        command_route(
            "settings",
            "zsclip.settings.save",
            "product_adapter",
            false,
            None,
        ),
        command_route(
            "settings",
            "zsclip.settings.close",
            "native_window",
            false,
            None,
        ),
        command_route(
            "settings",
            "zsclip.settings.open_config",
            "native_window",
            false,
            None,
        ),
        command_route(
            "settings",
            "zsclip.settings.open_dropdown",
            "native_window",
            false,
            None,
        ),
        command_route(
            "settings",
            "zsclip.settings.toggle_control",
            "product_adapter",
            false,
            None,
        ),
        command_route(
            "tray",
            "zsclip.tray.toggle_window",
            "native_window",
            false,
            None,
        ),
        command_route(
            "tray",
            "zsclip.tray.toggle_clipboard_capture",
            "product_adapter",
            false,
            None,
        ),
        command_route(
            "tray",
            "zsclip.tray.toggle_lan_sync",
            "product_adapter",
            false,
            None,
        ),
        command_route("tray", "zsclip.tray.exit", "native_window", false, None),
        command_route("row", "zsclip.row.paste", "product_adapter", true, None),
        command_route("row", "zsclip.row.copy", "product_adapter", true, None),
        command_route(
            "row",
            "zsclip.row.toggle_pin",
            "product_adapter",
            true,
            None,
        ),
        command_route("row", "zsclip.row.to_phrase", "product_adapter", true, None),
        command_route(
            "row",
            "zsclip.row.assign_group",
            "product_adapter",
            true,
            None,
        ),
        command_route(
            "row",
            "zsclip.row.add_to_group",
            "product_adapter",
            true,
            None,
        ),
        command_route(
            "row",
            "zsclip.row.remove_from_group",
            "product_adapter",
            true,
            None,
        ),
        command_route("row", "zsclip.row.delete", "product_adapter", true, None),
        command_route(
            "row",
            "zsclip.row.delete_unpinned",
            "product_adapter",
            false,
            None,
        ),
        command_route("row", "zsclip.row.sticker", "product_adapter", true, None),
        command_route(
            "row",
            "zsclip.row.save_image",
            "product_adapter",
            true,
            None,
        ),
        command_route(
            "row",
            "zsclip.row.image_ocr",
            "product_adapter",
            true,
            Some("clipboard.product.ocr"),
        ),
        command_route(
            "row",
            "zsclip.row.export_file",
            "product_adapter",
            true,
            None,
        ),
        command_route("row", "zsclip.row.open_path", "native_shell", true, None),
        command_route("row", "zsclip.row.open_folder", "native_shell", true, None),
        command_route("row", "zsclip.row.copy_path", "product_adapter", true, None),
        command_route("row", "zsclip.row.qr_image", "product_adapter", true, None),
        command_route(
            "row",
            "zsclip.row.mail_merge",
            "product_adapter",
            true,
            None,
        ),
        command_route("row", "zsclip.row.lan_push", "product_adapter", true, None),
        command_route("row", "zsclip.row.edit", "product_adapter", true, None),
        command_route("row", "zsclip.row.quick_search", "native_shell", true, None),
        command_route(
            "row",
            "zsclip.row.text_translate",
            "product_adapter",
            true,
            Some("clipboard.skill.translate"),
        ),
        command_route(
            "group_filter",
            "zsclip.group_filter.all",
            "product_adapter",
            false,
            None,
        ),
        command_route(
            "group_filter",
            "zsclip.group_filter.select",
            "product_adapter",
            false,
            None,
        ),
    ]
}

#[allow(dead_code)]
pub(crate) fn zsclip_product_event_routes() -> Vec<ZsclipProductEventRoute> {
    vec![
        event_route("lan_sync_ready", "refresh_lan_sync_state"),
        event_route("vv_show_requested", "show_vv_popup"),
        event_route("vv_hide_requested", "hide_vv_popup"),
        event_route("vv_select_requested", "paste_vv_selection"),
        event_route("clipboard_changed", "capture_clipboard_change"),
        event_route("items_page_ready", "merge_items_page"),
        event_route("startup_data_reconciled", "refresh_after_startup_cleanup"),
        event_route("cloud_sync_ready", "refresh_cloud_sync_state"),
        event_route("update_check_ready", "refresh_update_state"),
        event_route("shell_integration_restored", "refresh_shell_integration"),
        event_route("tray_callback", "dispatch_tray_callback"),
    ]
}

fn command_route(
    family_name: &'static str,
    result_name: &'static str,
    execution_owner: &'static str,
    requires_selection: bool,
    ai_capability_id: Option<&'static str>,
) -> ZsclipProductCommandRoute {
    ZsclipProductCommandRoute {
        family_name,
        result_name,
        execution_owner,
        requires_selection,
        ai_capability_id,
    }
}

fn event_route(
    event_name: &'static str,
    product_effect_name: &'static str,
) -> ZsclipProductEventRoute {
    ZsclipProductEventRoute {
        event_name,
        product_effect_name,
    }
}

impl Default for ZsclipProductAdapter {
    fn default() -> Self {
        Self::new(ZsclipProductSnapshot::default())
    }
}

impl ProductAdapterHost for ZsclipProductAdapter {
    fn product_identity(&self) -> ProductAdapterIdentity {
        ProductAdapterIdentity {
            product_id: "zsclip".to_string(),
            display_name: "ZSClip".to_string(),
        }
    }

    fn project_product_state(&self) -> ProductAdapterProjectedState {
        ProductAdapterProjectedState {
            state_name: self.product_state_name().to_string(),
            revision: self.snapshot.settings.revision
                + self.command_records.len() as u64
                + self.bridged_event_names.len() as u64
                + self.ai_action_names.len() as u64,
            native_clip_items: self.snapshot.native_clip_items.clone(),
        }
    }

    fn execute_product_command(&mut self, command: Command) -> ProductAdapterCommandResult {
        let Some(result_name) = self.result_name_for_command(&command) else {
            return ProductAdapterCommandResult {
                accepted: false,
                result_name: "zsclip.unsupported_command".to_string(),
            };
        };
        if command.id == command_ids::UPDATE_SEARCH_TEXT {
            if let CommandPayload::Text(text) = &command.payload {
                self.snapshot.search_text = text.clone();
            }
        }
        self.command_records.push(ZsclipProductCommandRecord {
            command_id: command.id.0,
            result_name: result_name.to_string(),
        });
        ProductAdapterCommandResult {
            accepted: true,
            result_name: result_name.to_string(),
        }
    }

    fn bind_settings_model(&mut self, settings: ProductAdapterSettingsSnapshot) {
        self.snapshot.settings.revision = settings.revision;
    }

    fn bridge_async_event(&mut self, event: ApplicationEvent) -> ProductAdapterAsyncBridgeResult {
        let event_name = event_name(event);
        self.bridged_event_names.push(event_name.to_string());
        ProductAdapterAsyncBridgeResult {
            bridged: true,
            event_name: event_name.to_string(),
        }
    }

    fn publish_ai_catalog(&self) -> Vec<ProductAiCapability> {
        product_ai_capability_catalog()
            .iter()
            .map(|descriptor| descriptor.capability())
            .collect()
    }

    fn execute_ai_plan(&mut self, plan: ProductAiExecutionPlan) -> ProductAdapterCommandResult {
        self.ai_action_names.push(plan.action_name());
        ProductAdapterCommandResult {
            accepted: true,
            result_name: plan.result_name().to_string(),
        }
    }
}

fn result_name_for_tray_action(action: MainTrayMenuAction) -> &'static str {
    match action {
        MainTrayMenuAction::ToggleWindow => "zsclip.tray.toggle_window",
        MainTrayMenuAction::ToggleClipboardCapture => "zsclip.tray.toggle_clipboard_capture",
        MainTrayMenuAction::ToggleLanSync => "zsclip.tray.toggle_lan_sync",
        MainTrayMenuAction::Exit => "zsclip.tray.exit",
    }
}

fn result_name_for_row_action(action: MainRowMenuAction) -> &'static str {
    match action {
        MainRowMenuAction::Copy => "zsclip.row.copy",
        MainRowMenuAction::Pin => "zsclip.row.toggle_pin",
        MainRowMenuAction::ToPhrase => "zsclip.row.to_phrase",
        MainRowMenuAction::AddToGroup => "zsclip.row.add_to_group",
        MainRowMenuAction::RemoveFromGroup => "zsclip.row.remove_from_group",
        MainRowMenuAction::Delete => "zsclip.row.delete",
        MainRowMenuAction::DeleteUnpinned => "zsclip.row.delete_unpinned",
        MainRowMenuAction::Sticker => "zsclip.row.sticker",
        MainRowMenuAction::SaveImage => "zsclip.row.save_image",
        MainRowMenuAction::ImageOcr => "zsclip.row.image_ocr",
        MainRowMenuAction::ExportFile => "zsclip.row.export_file",
        MainRowMenuAction::OpenPath => "zsclip.row.open_path",
        MainRowMenuAction::OpenFolder => "zsclip.row.open_folder",
        MainRowMenuAction::CopyPath => "zsclip.row.copy_path",
        MainRowMenuAction::QrImage => "zsclip.row.qr_image",
        MainRowMenuAction::MailMerge => "zsclip.row.mail_merge",
        MainRowMenuAction::LanPush => "zsclip.row.lan_push",
        MainRowMenuAction::Edit => "zsclip.row.edit",
        MainRowMenuAction::QuickSearch => "zsclip.row.quick_search",
        MainRowMenuAction::TextTranslate => "zsclip.row.text_translate",
    }
}

fn event_name(event: ApplicationEvent) -> &'static str {
    match event {
        ApplicationEvent::LanSyncReady => "lan_sync_ready",
        ApplicationEvent::VvShowRequested { .. } => "vv_show_requested",
        ApplicationEvent::VvHideRequested => "vv_hide_requested",
        ApplicationEvent::VvSelectRequested { .. } => "vv_select_requested",
        ApplicationEvent::ClipboardChanged { .. } => "clipboard_changed",
        ApplicationEvent::ItemsPageReady => "items_page_ready",
        ApplicationEvent::StartupDataReconciled { .. } => "startup_data_reconciled",
        ApplicationEvent::CloudSyncReady => "cloud_sync_ready",
        ApplicationEvent::UpdateCheckReady => "update_check_ready",
        ApplicationEvent::ShellIntegrationRestored => "shell_integration_restored",
        ApplicationEvent::TrayCallback { .. } => "tray_callback",
    }
}

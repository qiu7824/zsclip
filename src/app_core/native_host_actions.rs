use crate::app_core::ClipboardHost;
use crate::app_core::{
    command_ids, main_group_filter_menu_plan, main_group_filter_popup_entries,
    main_menu_command_for_id, main_row_group_popup_entries, main_row_menu_plan,
    main_row_popup_menu_entries, main_vv_select_plan, menu_ids, settings_command_for_control_role,
    ApplicationEvent, ClipGroup, ClipItem, ClipKind, ClipKindFilter, Command, CommandId,
    MainRowMenuInput, MainRowMenuLabelInput, MainTrayMenuAction, MainVvPopupLayout,
    MainVvPopupRenderItem, MainVvPopupRenderPlan, MainVvPopupRenderStrings, MainVvSelectPlan,
    NativePopupMenuEntry, SettingsControlRole, UiRect,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeHostUiAction {
    ToggleSearch,
    OpenSettings,
    HideWindow,
    CloseWindow,
}

impl NativeHostUiAction {
    pub(crate) const fn action_name(self) -> &'static str {
        match self {
            Self::ToggleSearch => "toggle_search",
            Self::OpenSettings => "open_settings",
            Self::HideWindow => "hide_window",
            Self::CloseWindow => "close_window",
        }
    }

    pub(crate) const fn button_label(self) -> &'static str {
        match self {
            Self::ToggleSearch => "Search",
            Self::OpenSettings => "Settings",
            Self::HideWindow => "Hide",
            Self::CloseWindow => "Close",
        }
    }

    pub(crate) const fn command_id(self) -> CommandId {
        match self {
            Self::ToggleSearch => command_ids::TOGGLE_SEARCH,
            Self::OpenSettings => command_ids::OPEN_SETTINGS,
            Self::HideWindow => command_ids::HIDE_WINDOW,
            Self::CloseWindow => command_ids::CLOSE_WINDOW,
        }
    }

    pub(crate) fn command(self) -> Command {
        Command::window(self.command_id())
    }

    pub(crate) const fn opens_settings_surface(self) -> bool {
        matches!(self, Self::OpenSettings)
    }

    pub(crate) const fn toggles_search_surface(self) -> bool {
        matches!(self, Self::ToggleSearch)
    }

    pub(crate) const fn hides_main_window_surface(self) -> bool {
        matches!(self, Self::HideWindow)
    }

    pub(crate) const fn should_close_host(self) -> bool {
        matches!(self, Self::CloseWindow)
    }
}

pub(crate) const REQUIRED_NATIVE_HOST_UI_ACTIONS: [NativeHostUiAction; 4] = [
    NativeHostUiAction::ToggleSearch,
    NativeHostUiAction::OpenSettings,
    NativeHostUiAction::HideWindow,
    NativeHostUiAction::CloseWindow,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeHostMainToolAction {
    RowMenu,
    GroupFilter,
    #[cfg(feature = "vv-paste")]
    VvPopup,
    #[cfg(feature = "vv-paste")]
    VvTrigger,
}

impl NativeHostMainToolAction {
    pub(crate) const fn action_name(self) -> &'static str {
        match self {
            Self::RowMenu => "main_row_menu",
            Self::GroupFilter => "main_group_filter",
            #[cfg(feature = "vv-paste")]
            Self::VvPopup => "main_vv_popup",
            #[cfg(feature = "vv-paste")]
            Self::VvTrigger => "main_vv_trigger",
        }
    }

    pub(crate) const fn button_label(self) -> &'static str {
        match self {
            Self::RowMenu => "Row Menu",
            Self::GroupFilter => "Group Filter",
            #[cfg(feature = "vv-paste")]
            Self::VvPopup => "VV Popup",
            #[cfg(feature = "vv-paste")]
            Self::VvTrigger => "VV Trigger",
        }
    }

    pub(crate) const fn opens_row_menu(self) -> bool {
        matches!(self, Self::RowMenu)
    }

    pub(crate) const fn opens_group_filter_menu(self) -> bool {
        matches!(self, Self::GroupFilter)
    }

    pub(crate) const fn opens_vv_popup(self) -> bool {
        #[cfg(feature = "vv-paste")]
        {
            matches!(self, Self::VvPopup)
        }
        #[cfg(not(feature = "vv-paste"))]
        {
            let _ = self;
            false
        }
    }

    pub(crate) const fn triggers_vv_demo(self) -> bool {
        #[cfg(feature = "vv-paste")]
        {
            matches!(self, Self::VvTrigger)
        }
        #[cfg(not(feature = "vv-paste"))]
        {
            let _ = self;
            false
        }
    }
}

pub(crate) fn native_host_main_tool_actions() -> Vec<NativeHostMainToolAction> {
    vec![
        NativeHostMainToolAction::RowMenu,
        NativeHostMainToolAction::GroupFilter,
        #[cfg(feature = "vv-paste")]
        NativeHostMainToolAction::VvPopup,
        #[cfg(feature = "vv-paste")]
        NativeHostMainToolAction::VvTrigger,
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeHostStatusMenuAction {
    ToggleWindow,
    ToggleClipboardCapture,
    #[cfg(feature = "lan-sync")]
    ToggleLanSync,
    Exit,
}

impl NativeHostStatusMenuAction {
    pub(crate) const fn action_name(self) -> &'static str {
        match self {
            Self::ToggleWindow => "status_toggle_window",
            Self::ToggleClipboardCapture => "status_toggle_clipboard_capture",
            #[cfg(feature = "lan-sync")]
            Self::ToggleLanSync => "status_toggle_lan_sync",
            Self::Exit => "status_exit",
        }
    }

    pub(crate) const fn menu_label(self) -> &'static str {
        match self {
            Self::ToggleWindow => "Show ZSClip",
            Self::ToggleClipboardCapture => "Toggle Capture",
            #[cfg(feature = "lan-sync")]
            Self::ToggleLanSync => "Toggle LAN Sync",
            Self::Exit => "Exit",
        }
    }

    pub(crate) const fn tray_action(self) -> MainTrayMenuAction {
        match self {
            Self::ToggleWindow => MainTrayMenuAction::ToggleWindow,
            Self::ToggleClipboardCapture => MainTrayMenuAction::ToggleClipboardCapture,
            #[cfg(feature = "lan-sync")]
            Self::ToggleLanSync => MainTrayMenuAction::ToggleLanSync,
            Self::Exit => MainTrayMenuAction::Exit,
        }
    }

    pub(crate) const fn menu_id(self) -> usize {
        self.tray_action().command_id()
    }

    pub(crate) fn command(self) -> Command {
        main_menu_command_for_id(self.menu_id())
            .expect("native status menu action must map to tray menu command")
    }

    pub(crate) const fn should_exit_host(self) -> bool {
        matches!(self, Self::Exit)
    }

    pub(crate) const fn toggles_main_window_surface(self) -> bool {
        matches!(self, Self::ToggleWindow)
    }
}

#[cfg(feature = "lan-sync")]
pub(crate) const REQUIRED_NATIVE_HOST_STATUS_MENU_ACTIONS: [NativeHostStatusMenuAction; 4] = [
    NativeHostStatusMenuAction::ToggleWindow,
    NativeHostStatusMenuAction::ToggleClipboardCapture,
    NativeHostStatusMenuAction::ToggleLanSync,
    NativeHostStatusMenuAction::Exit,
];

#[cfg(not(feature = "lan-sync"))]
pub(crate) const REQUIRED_NATIVE_HOST_STATUS_MENU_ACTIONS: [NativeHostStatusMenuAction; 3] = [
    NativeHostStatusMenuAction::ToggleWindow,
    NativeHostStatusMenuAction::ToggleClipboardCapture,
    NativeHostStatusMenuAction::Exit,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeHostSourceTabSpec {
    pub(crate) id: &'static str,
    pub(crate) label_source: &'static str,
    pub(crate) label_en: &'static str,
    pub(crate) category: i64,
}

impl NativeHostSourceTabSpec {
    pub(crate) const fn zsui_tab(self) -> crate::zsui::ZsTabSpec {
        crate::zsui::ZsTabSpec::new(self.id, self.label_source)
    }
}

pub(crate) const NATIVE_HOST_SOURCE_TABS: [NativeHostSourceTabSpec; 2] = [
    NativeHostSourceTabSpec {
        id: "clipboard_records",
        label_source: "复制记录",
        label_en: "Clipboard Records",
        category: 0,
    },
    NativeHostSourceTabSpec {
        id: "phrases",
        label_source: "常用短语",
        label_en: "Phrases",
        category: 1,
    },
];

pub(crate) const fn native_host_source_tab_for_category(category: i64) -> NativeHostSourceTabSpec {
    if category == NATIVE_HOST_SOURCE_TABS[1].category {
        NATIVE_HOST_SOURCE_TABS[1]
    } else {
        NATIVE_HOST_SOURCE_TABS[0]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeHostRowAction {
    Paste,
    Copy,
    Pin,
    ToPhrase,
    Delete,
    Edit,
    OpenPath,
    OpenFolder,
    CopyPath,
    #[cfg(feature = "ai-actions")]
    TextTranslate,
}

impl NativeHostRowAction {
    pub(crate) const fn action_name(self) -> &'static str {
        match self {
            Self::Paste => "row_paste",
            Self::Copy => "row_copy",
            Self::Pin => "row_toggle_pin",
            Self::ToPhrase => "row_to_phrase",
            Self::Delete => "row_delete",
            Self::Edit => "row_edit",
            Self::OpenPath => "row_open_path",
            Self::OpenFolder => "row_open_folder",
            Self::CopyPath => "row_copy_path",
            #[cfg(feature = "ai-actions")]
            Self::TextTranslate => "row_text_translate",
        }
    }

    pub(crate) const fn button_label(self) -> &'static str {
        match self {
            Self::Paste => "Paste",
            Self::Copy => "Copy",
            Self::Pin => "Pin",
            Self::ToPhrase => "To Phrase",
            Self::Delete => "Delete",
            Self::Edit => "Edit",
            Self::OpenPath => "Open Path",
            Self::OpenFolder => "Open Folder",
            Self::CopyPath => "Copy Path",
            #[cfg(feature = "ai-actions")]
            Self::TextTranslate => "Translate",
        }
    }

    pub(crate) const fn menu_id(self) -> usize {
        match self {
            Self::Paste => menu_ids::ROW_PASTE,
            Self::Copy => menu_ids::ROW_COPY,
            Self::Pin => menu_ids::ROW_PIN,
            Self::ToPhrase => menu_ids::ROW_TO_PHRASE,
            Self::Delete => menu_ids::ROW_DELETE,
            Self::Edit => menu_ids::ROW_EDIT,
            Self::OpenPath => menu_ids::ROW_OPEN_PATH,
            Self::OpenFolder => menu_ids::ROW_OPEN_FOLDER,
            Self::CopyPath => menu_ids::ROW_COPY_PATH,
            #[cfg(feature = "ai-actions")]
            Self::TextTranslate => menu_ids::ROW_TEXT_TRANSLATE,
        }
    }

    pub(crate) const fn from_menu_id(menu_id: usize) -> Option<Self> {
        match menu_id {
            menu_ids::ROW_PASTE => Some(Self::Paste),
            menu_ids::ROW_COPY => Some(Self::Copy),
            menu_ids::ROW_PIN => Some(Self::Pin),
            menu_ids::ROW_TO_PHRASE => Some(Self::ToPhrase),
            menu_ids::ROW_DELETE => Some(Self::Delete),
            menu_ids::ROW_EDIT => Some(Self::Edit),
            menu_ids::ROW_OPEN_PATH => Some(Self::OpenPath),
            menu_ids::ROW_OPEN_FOLDER => Some(Self::OpenFolder),
            menu_ids::ROW_COPY_PATH => Some(Self::CopyPath),
            #[cfg(feature = "ai-actions")]
            menu_ids::ROW_TEXT_TRANSLATE => Some(Self::TextTranslate),
            _ => None,
        }
    }

    pub(crate) fn command(self) -> Command {
        main_menu_command_for_id(self.menu_id())
            .expect("native row action must map to menu command")
    }
}

pub(crate) fn native_host_row_actions() -> Vec<NativeHostRowAction> {
    vec![
        NativeHostRowAction::Paste,
        NativeHostRowAction::Copy,
        NativeHostRowAction::Pin,
        NativeHostRowAction::ToPhrase,
        NativeHostRowAction::Delete,
        NativeHostRowAction::Edit,
        NativeHostRowAction::OpenPath,
        NativeHostRowAction::OpenFolder,
        NativeHostRowAction::CopyPath,
        #[cfg(feature = "ai-actions")]
        NativeHostRowAction::TextTranslate,
    ]
}

pub(crate) fn native_host_row_popup_menu_entries() -> Vec<NativePopupMenuEntry> {
    native_host_row_actions()
        .into_iter()
        .map(|action| NativePopupMenuEntry::Command {
            id: action.menu_id(),
            label: action.button_label().to_string(),
            enabled: true,
            checked: false,
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NativeHostRowPopupMenuInput {
    pub(crate) menu: MainRowMenuInput,
    pub(crate) labels: MainRowMenuLabelInput,
    pub(crate) empty_group_label: String,
}

impl NativeHostRowPopupMenuInput {
    pub(crate) fn demo() -> Self {
        Self {
            menu: MainRowMenuInput {
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
            },
            labels: MainRowMenuLabelInput {
                selected_count: 1,
                has_unpinned: true,
                current_is_dir: false,
            },
            empty_group_label: "(No groups)".to_string(),
        }
    }
}

pub(crate) fn native_host_row_popup_menu_input_for_projection(
    items: &[NativeHostClipListItemProjection],
    selected_item_id: i64,
    grouping_enabled: bool,
) -> NativeHostRowPopupMenuInput {
    let selected = if selected_item_id > 0 {
        items
            .iter()
            .find(|item| item.id == selected_item_id)
            .or_else(|| items.first())
    } else {
        items.first()
    };
    let selected_count = usize::from(selected.is_some());
    let current_kind = selected.map(|item| item.kind).unwrap_or(ClipKind::Text);
    let has_unpinned = selected.map(|item| !item.pinned).unwrap_or(false);
    let current_is_excel = selected
        .map(|item| native_host_projection_looks_like_excel_file(item))
        .unwrap_or(false);

    NativeHostRowPopupMenuInput {
        menu: MainRowMenuInput {
            selected_count,
            has_unpinned,
            current_kind,
            grouping_enabled,
            current_can_ocr: matches!(current_kind, ClipKind::Image | ClipKind::Files),
            current_can_translate: matches!(current_kind, ClipKind::Text | ClipKind::Phrase),
            current_is_excel,
            quick_search_enabled: matches!(current_kind, ClipKind::Text | ClipKind::Phrase),
            qr_quick_enabled: true,
            super_mail_merge_enabled: current_is_excel,
            lan_push_available: matches!(current_kind, ClipKind::Files),
        },
        labels: MainRowMenuLabelInput {
            selected_count,
            has_unpinned,
            current_is_dir: false,
        },
        empty_group_label: "(No groups)".to_string(),
    }
}

fn native_host_projection_looks_like_excel_file(item: &NativeHostClipListItemProjection) -> bool {
    if item.kind != ClipKind::Files {
        return false;
    }
    let preview = item.preview.trim().to_ascii_lowercase();
    [".xls", ".xlsx", ".xlsm", ".xlsb", ".csv"]
        .iter()
        .any(|suffix| preview.ends_with(suffix))
}

pub(crate) fn native_host_full_row_popup_menu_entries() -> Vec<NativePopupMenuEntry> {
    let groups = vec![
        ClipGroup {
            id: 1,
            category: 0,
            name: "Work".to_string(),
        },
        ClipGroup {
            id: 2,
            category: 0,
            name: "Phrase".to_string(),
        },
    ];
    native_host_full_row_popup_menu_entries_for_groups(
        &groups,
        NativeHostRowPopupMenuInput::demo(),
        |label| label.to_string(),
    )
}

pub(crate) fn native_host_full_row_popup_menu_entries_for_groups<F>(
    groups: &[ClipGroup],
    input: NativeHostRowPopupMenuInput,
    label_localizer: F,
) -> Vec<NativePopupMenuEntry>
where
    F: Fn(&str) -> String,
{
    let grouping_enabled = input.menu.grouping_enabled;
    let plan = main_row_menu_plan(input.menu);
    main_row_popup_menu_entries(
        &plan,
        input.labels,
        grouping_enabled,
        main_row_group_popup_entries(groups, input.empty_group_label),
        label_localizer,
    )
}

pub(crate) fn native_host_group_filter_popup_menu_entries() -> Vec<NativePopupMenuEntry> {
    let groups = vec![
        ClipGroup {
            id: 1,
            category: 0,
            name: "All Work".to_string(),
        },
        ClipGroup {
            id: 2,
            category: 0,
            name: "Phrase Bank".to_string(),
        },
    ];
    native_host_group_filter_popup_menu_entries_for_groups(&groups, 2)
}

pub(crate) fn native_host_group_filter_popup_menu_entries_for_groups(
    groups: &[ClipGroup],
    current_group_id: i64,
) -> Vec<NativePopupMenuEntry> {
    native_host_group_filter_popup_menu_entries_for_groups_kind_filter(
        groups,
        current_group_id,
        false,
        ClipKindFilter::All,
        &[],
    )
}

pub(crate) fn native_host_group_filter_popup_menu_entries_for_groups_kind_filter(
    groups: &[ClipGroup],
    current_group_id: i64,
    kind_filter_enabled: bool,
    current_kind_filter: ClipKindFilter,
    kind_filters: &[ClipKindFilter],
) -> Vec<NativePopupMenuEntry> {
    let plan = main_group_filter_menu_plan(
        current_group_id,
        groups,
        kind_filter_enabled,
        current_kind_filter,
        kind_filters,
    );
    main_group_filter_popup_entries(&plan, "All")
}

pub(crate) fn native_host_group_filter_label_for_groups(
    groups: &[ClipGroup],
    current_group_id: i64,
) -> String {
    if current_group_id <= 0 {
        return "All".to_string();
    }
    groups
        .iter()
        .find(|group| group.id == current_group_id)
        .map(|group| group.name.clone())
        .unwrap_or_else(|| "All".to_string())
}

pub(crate) fn native_host_vv_popup_render_plan() -> MainVvPopupRenderPlan {
    native_host_vv_popup_render_plan_for_projection(
        &native_host_default_clip_list_projection(),
        "All",
    )
}

pub(crate) fn native_host_vv_popup_render_plan_for_projection(
    items: &[NativeHostClipListItemProjection],
    group_label: &str,
) -> MainVvPopupRenderPlan {
    let layout = MainVvPopupLayout::default();
    let visible_items = items.iter().take(9).collect::<Vec<_>>();
    let client_rect = UiRect::new(0, 0, layout.width, layout.height(visible_items.len()));
    let strings = MainVvPopupRenderStrings {
        title: "VV Mode".to_string(),
        hint: "Press 1-9 to paste, Esc to cancel".to_string(),
        empty: "No records in this group".to_string(),
    };
    let render_items = visible_items
        .iter()
        .enumerate()
        .map(|(index, item)| MainVvPopupRenderItem {
            index: index + 1,
            label: native_host_projected_clip_row_title(item),
        })
        .collect::<Vec<_>>();
    layout.render_plan(client_rect, &strings, group_label, &render_items)
}

pub(crate) fn native_host_vv_select_event(index: usize) -> ApplicationEvent {
    ApplicationEvent::VvSelectRequested { index }
}

#[cfg(feature = "vv-paste")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeHostVvSelectAction {
    pub(crate) index: usize,
}

#[cfg(feature = "vv-paste")]
impl NativeHostVvSelectAction {
    pub(crate) const fn new(index: usize) -> Self {
        Self { index }
    }

    pub(crate) const fn action_name(self) -> &'static str {
        "vv_select"
    }

    pub(crate) fn button_label(self) -> String {
        format!("Select {}", self.index + 1)
    }

    pub(crate) fn event(self) -> ApplicationEvent {
        native_host_vv_select_event(self.index)
    }
}

pub(crate) const NATIVE_HOST_VV_TRIGGER_TIMEOUT_MS: u64 = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeHostVvTriggerKey {
    TriggerV,
    Digit1To9(usize),
    Escape,
    Backspace,
    Modifier,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeHostVvTriggerInput {
    pub(crate) key: NativeHostVvTriggerKey,
    pub(crate) target_token: u64,
    pub(crate) target_ready: bool,
    pub(crate) command_modifier: bool,
    pub(crate) popup_menu_active: bool,
    pub(crate) now_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeHostVvTriggerAction {
    Ignore,
    Show { target_token: u64 },
    Select { index: usize },
    Hide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeHostVvTriggerTransition {
    pub(crate) action: NativeHostVvTriggerAction,
    pub(crate) consume_key: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeHostVvTriggerState {
    last_v_target: u64,
    last_v_at_ms: Option<u64>,
    popup_active: bool,
    popup_target: u64,
    timeout_ms: u64,
}

impl Default for NativeHostVvTriggerState {
    fn default() -> Self {
        Self {
            last_v_target: 0,
            last_v_at_ms: None,
            popup_active: false,
            popup_target: 0,
            timeout_ms: NATIVE_HOST_VV_TRIGGER_TIMEOUT_MS,
        }
    }
}

impl NativeHostVvTriggerState {
    pub(crate) fn mark_popup_active(&mut self, target_token: u64) {
        self.popup_active = target_token != 0;
        self.popup_target = target_token;
    }

    pub(crate) fn clear_popup(&mut self) {
        self.popup_active = false;
        self.popup_target = 0;
    }

    fn clear_last_v(&mut self) {
        self.last_v_target = 0;
        self.last_v_at_ms = None;
    }

    fn reset_trigger(&mut self) {
        self.clear_last_v();
        self.clear_popup();
    }

    pub(crate) fn apply(
        &mut self,
        input: NativeHostVvTriggerInput,
    ) -> NativeHostVvTriggerTransition {
        if input.target_token == 0 || input.command_modifier {
            self.clear_last_v();
            return NativeHostVvTriggerTransition {
                action: NativeHostVvTriggerAction::Ignore,
                consume_key: false,
            };
        }

        if self.popup_active {
            if input.popup_menu_active {
                return NativeHostVvTriggerTransition {
                    action: NativeHostVvTriggerAction::Ignore,
                    consume_key: false,
                };
            }
            if input.target_token != self.popup_target {
                self.reset_trigger();
                return NativeHostVvTriggerTransition {
                    action: NativeHostVvTriggerAction::Hide,
                    consume_key: false,
                };
            }
            match input.key {
                NativeHostVvTriggerKey::Digit1To9(index) => {
                    self.reset_trigger();
                    return NativeHostVvTriggerTransition {
                        action: NativeHostVvTriggerAction::Select { index },
                        consume_key: true,
                    };
                }
                NativeHostVvTriggerKey::Escape => {
                    self.reset_trigger();
                    return NativeHostVvTriggerTransition {
                        action: NativeHostVvTriggerAction::Hide,
                        consume_key: true,
                    };
                }
                NativeHostVvTriggerKey::Backspace => {
                    self.clear_popup();
                    self.clear_last_v();
                    return NativeHostVvTriggerTransition {
                        action: NativeHostVvTriggerAction::Hide,
                        consume_key: false,
                    };
                }
                NativeHostVvTriggerKey::Modifier => {
                    return NativeHostVvTriggerTransition {
                        action: NativeHostVvTriggerAction::Ignore,
                        consume_key: false,
                    };
                }
                NativeHostVvTriggerKey::TriggerV | NativeHostVvTriggerKey::Other => {
                    self.reset_trigger();
                    return NativeHostVvTriggerTransition {
                        action: NativeHostVvTriggerAction::Hide,
                        consume_key: false,
                    };
                }
            }
        }

        match input.key {
            NativeHostVvTriggerKey::TriggerV if input.target_ready => {
                let within_timeout = self
                    .last_v_at_ms
                    .map(|last| input.now_ms.saturating_sub(last) <= self.timeout_ms)
                    .unwrap_or(false);
                if self.last_v_target == input.target_token && within_timeout {
                    self.clear_last_v();
                    self.mark_popup_active(input.target_token);
                    NativeHostVvTriggerTransition {
                        action: NativeHostVvTriggerAction::Show {
                            target_token: input.target_token,
                        },
                        consume_key: false,
                    }
                } else {
                    self.last_v_target = input.target_token;
                    self.last_v_at_ms = Some(input.now_ms);
                    NativeHostVvTriggerTransition {
                        action: NativeHostVvTriggerAction::Ignore,
                        consume_key: false,
                    }
                }
            }
            NativeHostVvTriggerKey::Modifier => NativeHostVvTriggerTransition {
                action: NativeHostVvTriggerAction::Ignore,
                consume_key: false,
            },
            _ => {
                self.clear_last_v();
                NativeHostVvTriggerTransition {
                    action: NativeHostVvTriggerAction::Ignore,
                    consume_key: false,
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NativeHostClipboardWrite {
    Text(String),
    FilePaths(Vec<String>),
    ImageRgba {
        bytes: Vec<u8>,
        width: usize,
        height: usize,
    },
}

impl NativeHostClipboardWrite {
    pub(crate) const fn kind_name(&self) -> &'static str {
        match self {
            Self::Text(_) => "text",
            Self::FilePaths(_) => "files",
            Self::ImageRgba { .. } => "image",
        }
    }

    pub(crate) fn direct_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            Self::FilePaths(_) | Self::ImageRgba { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeHostVvPasteItem {
    pub(crate) item_id: i64,
    pub(crate) clipboard_write: NativeHostClipboardWrite,
    pub(crate) backspaces: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NativeHostVvPastePlan {
    HidePopup,
    Paste(NativeHostVvPasteItem),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeHostVvPasteExecution {
    pub(crate) accepted: bool,
    pub(crate) result_name: String,
    pub(crate) item_id: Option<i64>,
    pub(crate) clipboard_kind: Option<&'static str>,
    pub(crate) backspaces: u8,
    pub(crate) focused_target: bool,
    pub(crate) direct_text_set: bool,
    pub(crate) paste_shortcut_sent: bool,
}

impl NativeHostVvPasteExecution {
    pub(crate) fn rejected(result_name: impl Into<String>) -> Self {
        Self {
            accepted: false,
            result_name: result_name.into(),
            item_id: None,
            clipboard_kind: None,
            backspaces: 0,
            focused_target: false,
            direct_text_set: false,
            paste_shortcut_sent: false,
        }
    }

    pub(crate) fn pasted(
        item: &NativeHostVvPasteItem,
        focused_target: bool,
        direct_text_set: bool,
        paste_shortcut_sent: bool,
    ) -> Self {
        Self {
            accepted: true,
            result_name: "zsclip.vv_paste.clipboard_target".to_string(),
            item_id: Some(item.item_id),
            clipboard_kind: Some(item.clipboard_write.kind_name()),
            backspaces: item.backspaces,
            focused_target,
            direct_text_set,
            paste_shortcut_sent,
        }
    }
}

pub(crate) fn native_host_clipboard_write_for_item(
    item: &ClipItem,
) -> Option<NativeHostClipboardWrite> {
    match item.kind {
        ClipKind::Text | ClipKind::Phrase => item
            .text
            .as_ref()
            .filter(|text| !text.is_empty())
            .cloned()
            .map(NativeHostClipboardWrite::Text),
        ClipKind::Files => item
            .file_paths
            .as_ref()
            .filter(|paths| !paths.is_empty())
            .cloned()
            .map(NativeHostClipboardWrite::FilePaths)
            .or_else(|| {
                item.text
                    .as_ref()
                    .filter(|text| !text.is_empty())
                    .cloned()
                    .map(NativeHostClipboardWrite::Text)
            }),
        ClipKind::Image => item
            .image_bytes
            .as_ref()
            .filter(|bytes| !bytes.is_empty() && item.image_width > 0 && item.image_height > 0)
            .cloned()
            .map(|bytes| NativeHostClipboardWrite::ImageRgba {
                bytes,
                width: item.image_width,
                height: item.image_height,
            }),
    }
}

pub(crate) fn native_host_reconciled_selected_item_id(
    selected_item_id: i64,
    items: &[NativeHostClipListItemProjection],
) -> i64 {
    if selected_item_id > 0 && items.iter().any(|item| item.id == selected_item_id) {
        return selected_item_id;
    }
    items.first().map(|item| item.id).unwrap_or_default()
}

pub(crate) fn native_host_write_clipboard_payload<Host: ClipboardHost>(
    write: &NativeHostClipboardWrite,
) -> bool {
    match write {
        NativeHostClipboardWrite::Text(text) => Host::write_text_ignored_by_monitors(text),
        NativeHostClipboardWrite::FilePaths(paths) => Host::write_file_paths(paths),
        NativeHostClipboardWrite::ImageRgba {
            bytes,
            width,
            height,
        } => Host::write_image_rgba(bytes, *width, *height),
    }
}

pub(crate) fn native_host_vv_paste_plan(
    popup_visible: bool,
    index: usize,
    items: &[ClipItem],
    backspaces: u8,
) -> Option<NativeHostVvPastePlan> {
    match main_vv_select_plan(popup_visible, index, items, backspaces)? {
        MainVvSelectPlan::HidePopup => Some(NativeHostVvPastePlan::HidePopup),
        MainVvSelectPlan::Paste { item, backspaces } => {
            let clipboard_write = native_host_clipboard_write_for_item(&item)?;
            Some(NativeHostVvPastePlan::Paste(NativeHostVvPasteItem {
                item_id: item.id,
                clipboard_write,
                backspaces,
            }))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeHostSearchTextAction {
    pub(crate) text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeHostSearchControlAction {
    UpdateText,
}

impl NativeHostSearchControlAction {
    pub(crate) const fn action_name(self) -> &'static str {
        match self {
            Self::UpdateText => "search_text_changed",
        }
    }

    pub(crate) const fn placeholder(self) -> &'static str {
        match self {
            Self::UpdateText => "Search clipboard",
        }
    }
}

pub(crate) const REQUIRED_NATIVE_HOST_SEARCH_CONTROL_ACTIONS: [NativeHostSearchControlAction; 1] =
    [NativeHostSearchControlAction::UpdateText];

impl NativeHostSearchTextAction {
    pub(crate) fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    pub(crate) fn command(&self) -> Command {
        Command::window_with_payload(
            command_ids::UPDATE_SEARCH_TEXT,
            crate::app_core::CommandPayload::Text(self.text.clone()),
        )
    }

    pub(crate) fn normalized_text(&self) -> &str {
        self.text.trim()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeHostClipListItem {
    pub(crate) id: i64,
    pub(crate) title: &'static str,
    pub(crate) preview: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeHostClipListItemProjection {
    pub(crate) id: i64,
    pub(crate) title: String,
    pub(crate) preview: String,
    pub(crate) kind: ClipKind,
    pub(crate) pinned: bool,
}

impl NativeHostClipListItemProjection {
    pub(crate) fn new(id: i64, title: impl Into<String>, preview: impl Into<String>) -> Self {
        Self::with_metadata(id, title, preview, ClipKind::Text, false)
    }

    pub(crate) fn with_metadata(
        id: i64,
        title: impl Into<String>,
        preview: impl Into<String>,
        kind: ClipKind,
        pinned: bool,
    ) -> Self {
        Self {
            id,
            title: title.into(),
            preview: preview.into(),
            kind,
            pinned,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeHostEditTextPlan {
    pub(crate) item_id: i64,
    pub(crate) title: String,
    pub(crate) initial_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeHostEditTextClosePlan {
    pub(crate) dirty: bool,
    pub(crate) requires_unsaved_confirmation: bool,
}

pub(crate) fn native_host_edit_text_close_plan(
    initial_text: &str,
    current_text: &str,
) -> NativeHostEditTextClosePlan {
    let dirty = initial_text != current_text;
    NativeHostEditTextClosePlan {
        dirty,
        requires_unsaved_confirmation: dirty,
    }
}

impl From<&NativeHostClipListItem> for NativeHostClipListItemProjection {
    fn from(item: &NativeHostClipListItem) -> Self {
        Self::new(item.id, item.title, item.preview)
    }
}

pub(crate) const REQUIRED_NATIVE_HOST_CLIP_LIST_ITEMS: [NativeHostClipListItem; 4] = [
    NativeHostClipListItem {
        id: 1,
        title: "Welcome text",
        preview: "ZSClip keeps clipboard history searchable.",
    },
    NativeHostClipListItem {
        id: 2,
        title: "Image item",
        preview: "PNG screenshot preview",
    },
    NativeHostClipListItem {
        id: 3,
        title: "File path",
        preview: "C:\\Users\\Public\\Documents\\report.xlsx",
    },
    NativeHostClipListItem {
        id: 4,
        title: "Cloud sync note",
        preview: "WebDAV and LAN sync settings",
    },
];

pub(crate) fn native_host_clip_list_item_label(item: &NativeHostClipListItem) -> String {
    format!("{} - {}", item.title, item.preview)
}

pub(crate) fn native_host_projected_clip_list_item_label(
    item: &NativeHostClipListItemProjection,
) -> String {
    format!("{} - {}", item.title, item.preview)
}

pub(crate) fn native_host_projected_clip_row_title(
    item: &NativeHostClipListItemProjection,
) -> String {
    let pin_prefix = if item.pinned { "[PIN] " } else { "" };
    format!(
        "{}{} {}",
        pin_prefix,
        native_host_projected_clip_kind_prefix(item.kind),
        native_host_projected_clip_list_item_label(item)
    )
}

pub(crate) const fn native_host_projected_clip_kind_prefix(kind: ClipKind) -> &'static str {
    match kind {
        ClipKind::Text => "[TEXT]",
        ClipKind::Image => "[IMG]",
        ClipKind::Phrase => "[PHRASE]",
        ClipKind::Files => "[FILE]",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeHostClipKindIcon {
    Text,
    Image,
    Phrase,
    Files,
    Folder,
}

impl NativeHostClipKindIcon {
    pub(crate) const fn semantic_name(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Image => "image",
            Self::Phrase => "phrase",
            Self::Files => "files",
            Self::Folder => "folder",
        }
    }

    pub(crate) const fn zsui_icon(self) -> crate::zsui::ZsIcon {
        match self {
            Self::Text => crate::zsui::ZsIcon::Text,
            Self::Image => crate::zsui::ZsIcon::Image,
            Self::Phrase => crate::zsui::ZsIcon::Phrase,
            Self::Files => crate::zsui::ZsIcon::File,
            Self::Folder => crate::zsui::ZsIcon::Folder,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeHostClipRowPresentation {
    pub(crate) item_id: i64,
    pub(crate) title: String,
    pub(crate) preview: String,
    pub(crate) compact_label: String,
    pub(crate) accessibility_label: String,
    pub(crate) kind_prefix: &'static str,
    pub(crate) kind_icon: NativeHostClipKindIcon,
    pub(crate) pin_badge: Option<&'static str>,
}

pub(crate) fn native_host_clip_kind_icon_for_kind(kind: ClipKind) -> NativeHostClipKindIcon {
    match kind {
        ClipKind::Text => NativeHostClipKindIcon::Text,
        ClipKind::Image => NativeHostClipKindIcon::Image,
        ClipKind::Phrase => NativeHostClipKindIcon::Phrase,
        ClipKind::Files => NativeHostClipKindIcon::Files,
    }
}

pub(crate) fn native_host_clip_kind_icon_for_item(
    item: &ClipItem,
    is_directory: bool,
) -> NativeHostClipKindIcon {
    match item.kind {
        ClipKind::Files if is_directory => NativeHostClipKindIcon::Folder,
        kind => native_host_clip_kind_icon_for_kind(kind),
    }
}

pub(crate) fn native_host_clip_row_presentation_for_projection(
    item: &NativeHostClipListItemProjection,
) -> NativeHostClipRowPresentation {
    let kind_prefix = native_host_projected_clip_kind_prefix(item.kind);
    let compact_label = native_host_projected_clip_row_title(item);
    let pin_badge = item.pinned.then_some("PIN");
    let pinned_label = if item.pinned { "Pinned " } else { "" };
    NativeHostClipRowPresentation {
        item_id: item.id,
        title: item.title.clone(),
        preview: item.preview.clone(),
        accessibility_label: format!(
            "{}{} clipboard item: {}. {}",
            pinned_label,
            native_host_clip_kind_icon_for_kind(item.kind).semantic_name(),
            item.title,
            item.preview
        ),
        compact_label,
        kind_prefix,
        kind_icon: native_host_clip_kind_icon_for_kind(item.kind),
        pin_badge,
    }
}

pub(crate) fn native_host_clip_row_presentation_for_clip_item(
    item: &ClipItem,
    is_directory: bool,
) -> NativeHostClipRowPresentation {
    let projection = NativeHostClipListItemProjection::with_metadata(
        item.id,
        item.source_app.clone(),
        item.preview.clone(),
        item.kind,
        item.pinned,
    );
    let mut presentation = native_host_clip_row_presentation_for_projection(&projection);
    presentation.kind_icon = native_host_clip_kind_icon_for_item(item, is_directory);
    presentation
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeHostClipRowAction {
    pub(crate) index: usize,
    pub(crate) item_id: i64,
}

impl NativeHostClipRowAction {
    pub(crate) const fn new(index: usize, item_id: i64) -> Self {
        Self { index, item_id }
    }

    pub(crate) const fn action_name(self) -> &'static str {
        "clip_row_select"
    }

    pub(crate) const fn has_item(self) -> bool {
        self.item_id > 0
    }
}

pub(crate) fn native_host_clip_item_from_projection(
    item: &NativeHostClipListItemProjection,
) -> ClipItem {
    ClipItem {
        id: item.id,
        kind: item.kind,
        preview: item.preview.clone(),
        text: Some(item.preview.clone()),
        source_app: item.title.clone(),
        file_paths: None,
        image_bytes: None,
        image_path: None,
        image_width: 0,
        image_height: 0,
        pinned: item.pinned,
        group_id: 0,
        created_at: String::new(),
    }
}

pub(crate) fn native_host_edit_text_plan(
    items: &[NativeHostClipListItemProjection],
) -> Option<NativeHostEditTextPlan> {
    native_host_edit_text_plan_for_item(items, None)
}

pub(crate) fn native_host_edit_text_plan_for_item(
    items: &[NativeHostClipListItemProjection],
    selected_item_id: Option<i64>,
) -> Option<NativeHostEditTextPlan> {
    let item = selected_item_id
        .and_then(|item_id| items.iter().find(|item| item.id == item_id))
        .or_else(|| items.first())?;
    Some(NativeHostEditTextPlan {
        item_id: item.id,
        title: format!(
            "编辑 - {}",
            item.preview.chars().take(40).collect::<String>()
        ),
        initial_text: item.preview.clone(),
    })
}

pub(crate) fn native_host_default_clip_list_projection() -> Vec<NativeHostClipListItemProjection> {
    REQUIRED_NATIVE_HOST_CLIP_LIST_ITEMS
        .iter()
        .map(NativeHostClipListItemProjection::from)
        .collect()
}

pub(crate) fn native_host_filtered_clip_item_ids(query: &str) -> Vec<i64> {
    native_host_filtered_projected_clip_item_ids(&native_host_default_clip_list_projection(), query)
}

pub(crate) fn native_host_filtered_projected_clip_item_ids(
    items: &[NativeHostClipListItemProjection],
    query: &str,
) -> Vec<i64> {
    let normalized = query.trim().to_lowercase();
    items
        .iter()
        .filter(|item| {
            normalized.is_empty()
                || item.title.to_lowercase().contains(&normalized)
                || item.preview.to_lowercase().contains(&normalized)
        })
        .map(|item| item.id)
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeHostSettingsAction {
    Save,
    Close,
    OpenConfig,
}

impl NativeHostSettingsAction {
    pub(crate) const fn action_name(self) -> &'static str {
        match self {
            Self::Save => "save_settings",
            Self::Close => "close_settings",
            Self::OpenConfig => "open_settings_config",
        }
    }

    pub(crate) const fn button_label(self) -> &'static str {
        match self {
            Self::Save => "Save",
            Self::Close => "Close",
            Self::OpenConfig => "Open Config",
        }
    }

    pub(crate) const fn command_id(self) -> CommandId {
        match self {
            Self::Save => command_ids::SAVE_SETTINGS,
            Self::Close => command_ids::CLOSE_SETTINGS,
            Self::OpenConfig => command_ids::OPEN_SETTINGS_CONFIG,
        }
    }

    pub(crate) fn command(self) -> Command {
        Command::window(self.command_id())
    }

    pub(crate) const fn should_close_settings_surface(self) -> bool {
        matches!(self, Self::Close)
    }
}

pub(crate) const REQUIRED_NATIVE_HOST_SETTINGS_ACTIONS: [NativeHostSettingsAction; 3] = [
    NativeHostSettingsAction::Save,
    NativeHostSettingsAction::Close,
    NativeHostSettingsAction::OpenConfig,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeHostSettingsControlAction {
    ToggleAutostart,
    ToggleClipboardCapture,
    #[cfg(feature = "lan-sync")]
    ToggleLanSync,
    #[cfg(feature = "cloud-sync")]
    ToggleCloudSync,
    #[cfg(any(feature = "cloud-sync", feature = "lan-sync"))]
    OpenSyncModeDropdown,
}

impl NativeHostSettingsControlAction {
    pub(crate) const fn action_name(self) -> &'static str {
        match self {
            Self::ToggleAutostart => "settings_toggle_autostart",
            Self::ToggleClipboardCapture => "settings_toggle_clipboard_capture",
            #[cfg(feature = "lan-sync")]
            Self::ToggleLanSync => "settings_toggle_lan_sync",
            #[cfg(feature = "cloud-sync")]
            Self::ToggleCloudSync => "settings_toggle_cloud_sync",
            #[cfg(any(feature = "cloud-sync", feature = "lan-sync"))]
            Self::OpenSyncModeDropdown => "settings_open_sync_mode_dropdown",
        }
    }

    pub(crate) const fn button_label(self) -> &'static str {
        match self {
            Self::ToggleAutostart => "Auto Start",
            Self::ToggleClipboardCapture => "Capture",
            #[cfg(feature = "lan-sync")]
            Self::ToggleLanSync => "LAN Sync",
            #[cfg(feature = "cloud-sync")]
            Self::ToggleCloudSync => "Cloud Sync",
            #[cfg(any(feature = "cloud-sync", feature = "lan-sync"))]
            Self::OpenSyncModeDropdown => "Sync Mode",
        }
    }

    pub(crate) const fn control_id(self) -> i64 {
        match self {
            Self::ToggleAutostart => 5_010,
            Self::ToggleClipboardCapture => 5_101,
            #[cfg(feature = "lan-sync")]
            Self::ToggleLanSync => 7_102,
            #[cfg(feature = "cloud-sync")]
            Self::ToggleCloudSync => 7_103,
            #[cfg(any(feature = "cloud-sync", feature = "lan-sync"))]
            Self::OpenSyncModeDropdown => 6_102,
        }
    }

    pub(crate) const fn role(self) -> SettingsControlRole {
        match self {
            #[cfg(any(feature = "cloud-sync", feature = "lan-sync"))]
            Self::OpenSyncModeDropdown => SettingsControlRole::Dropdown,
            Self::ToggleAutostart => SettingsControlRole::Toggle,
            Self::ToggleClipboardCapture => SettingsControlRole::Toggle,
            #[cfg(feature = "lan-sync")]
            Self::ToggleLanSync => SettingsControlRole::Toggle,
            #[cfg(feature = "cloud-sync")]
            Self::ToggleCloudSync => SettingsControlRole::Toggle,
        }
    }

    pub(crate) const fn binding_control_key(self) -> Option<&'static str> {
        match self {
            Self::ToggleAutostart => Some("auto_start"),
            Self::ToggleClipboardCapture => Some("capture_enable"),
            #[cfg(feature = "lan-sync")]
            Self::ToggleLanSync => Some("lan_enable"),
            #[cfg(feature = "cloud-sync")]
            Self::ToggleCloudSync => Some("cloud_enable"),
            #[cfg(any(feature = "cloud-sync", feature = "lan-sync"))]
            Self::OpenSyncModeDropdown => Some("multi_sync_mode"),
        }
    }

    pub(crate) fn command(self) -> Command {
        settings_command_for_control_role(self.role(), self.control_id())
    }
}

#[cfg(all(feature = "cloud-sync", feature = "lan-sync"))]
pub(crate) const REQUIRED_NATIVE_HOST_SETTINGS_CONTROL_ACTIONS: [NativeHostSettingsControlAction;
    5] = [
    NativeHostSettingsControlAction::ToggleAutostart,
    NativeHostSettingsControlAction::ToggleClipboardCapture,
    NativeHostSettingsControlAction::ToggleLanSync,
    NativeHostSettingsControlAction::ToggleCloudSync,
    NativeHostSettingsControlAction::OpenSyncModeDropdown,
];

#[cfg(all(feature = "lan-sync", not(feature = "cloud-sync")))]
pub(crate) const REQUIRED_NATIVE_HOST_SETTINGS_CONTROL_ACTIONS: [NativeHostSettingsControlAction;
    4] = [
    NativeHostSettingsControlAction::ToggleAutostart,
    NativeHostSettingsControlAction::ToggleClipboardCapture,
    NativeHostSettingsControlAction::ToggleLanSync,
    NativeHostSettingsControlAction::OpenSyncModeDropdown,
];

#[cfg(all(feature = "cloud-sync", not(feature = "lan-sync")))]
pub(crate) const REQUIRED_NATIVE_HOST_SETTINGS_CONTROL_ACTIONS: [NativeHostSettingsControlAction;
    4] = [
    NativeHostSettingsControlAction::ToggleAutostart,
    NativeHostSettingsControlAction::ToggleClipboardCapture,
    NativeHostSettingsControlAction::ToggleCloudSync,
    NativeHostSettingsControlAction::OpenSyncModeDropdown,
];

#[cfg(not(any(feature = "cloud-sync", feature = "lan-sync")))]
pub(crate) const REQUIRED_NATIVE_HOST_SETTINGS_CONTROL_ACTIONS: [NativeHostSettingsControlAction;
    2] = [
    NativeHostSettingsControlAction::ToggleAutostart,
    NativeHostSettingsControlAction::ToggleClipboardCapture,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeHostSettingsPlatformAction {
    OpenSourceRepository,
    CheckForUpdates,
    OpenWpsTaskpaneDocs,
    DisableSystemClipboardHistory,
    EnableSystemClipboardHistory,
    RestartSystemShell,
}

impl NativeHostSettingsPlatformAction {
    pub(crate) const fn action_name(self) -> &'static str {
        match self {
            Self::OpenSourceRepository => "settings_open_source_repository",
            Self::CheckForUpdates => "settings_check_for_updates",
            Self::OpenWpsTaskpaneDocs => "settings_open_wps_taskpane_docs",
            Self::DisableSystemClipboardHistory => "settings_disable_system_clipboard_history",
            Self::EnableSystemClipboardHistory => "settings_enable_system_clipboard_history",
            Self::RestartSystemShell => "settings_restart_system_shell",
        }
    }

    pub(crate) const fn button_label(self) -> &'static str {
        match self {
            Self::OpenSourceRepository => "Open Source",
            Self::CheckForUpdates => "Check Updates",
            Self::OpenWpsTaskpaneDocs => "WPS Docs",
            Self::DisableSystemClipboardHistory => "Disable Clipboard History",
            Self::EnableSystemClipboardHistory => "Enable Clipboard History",
            Self::RestartSystemShell => "Restart Shell",
        }
    }
}

pub(crate) const REQUIRED_NATIVE_HOST_SETTINGS_PLATFORM_ACTIONS:
    [NativeHostSettingsPlatformAction; 6] = [
    NativeHostSettingsPlatformAction::OpenSourceRepository,
    NativeHostSettingsPlatformAction::CheckForUpdates,
    NativeHostSettingsPlatformAction::OpenWpsTaskpaneDocs,
    NativeHostSettingsPlatformAction::DisableSystemClipboardHistory,
    NativeHostSettingsPlatformAction::EnableSystemClipboardHistory,
    NativeHostSettingsPlatformAction::RestartSystemShell,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeHostSettingsGroupAction {
    ShowRecords,
    ShowPhrases,
    Add,
    Rename,
    Delete,
    MoveUp,
    MoveDown,
}

impl NativeHostSettingsGroupAction {
    pub(crate) const fn action_name(self) -> &'static str {
        match self {
            Self::ShowRecords => "settings_group_show_records",
            Self::ShowPhrases => "settings_group_show_phrases",
            Self::Add => "settings_group_add",
            Self::Rename => "settings_group_rename",
            Self::Delete => "settings_group_delete",
            Self::MoveUp => "settings_group_move_up",
            Self::MoveDown => "settings_group_move_down",
        }
    }

    pub(crate) const fn button_label(self) -> &'static str {
        match self {
            Self::ShowRecords => "Records",
            Self::ShowPhrases => "Phrases",
            Self::Add => "Add",
            Self::Rename => "Rename",
            Self::Delete => "Delete",
            Self::MoveUp => "Up",
            Self::MoveDown => "Down",
        }
    }

    pub(crate) const fn target_category(self) -> Option<i64> {
        match self {
            Self::ShowRecords => Some(0),
            Self::ShowPhrases => Some(1),
            _ => None,
        }
    }

    pub(crate) const fn move_step(self) -> Option<i64> {
        match self {
            Self::MoveUp => Some(-1),
            Self::MoveDown => Some(1),
            _ => None,
        }
    }
}

pub(crate) const REQUIRED_NATIVE_HOST_SETTINGS_GROUP_ACTIONS: [NativeHostSettingsGroupAction; 7] = [
    NativeHostSettingsGroupAction::ShowRecords,
    NativeHostSettingsGroupAction::ShowPhrases,
    NativeHostSettingsGroupAction::Add,
    NativeHostSettingsGroupAction::Rename,
    NativeHostSettingsGroupAction::Delete,
    NativeHostSettingsGroupAction::MoveUp,
    NativeHostSettingsGroupAction::MoveDown,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeHostEditTextAction {
    Save,
    Cancel,
}

impl NativeHostEditTextAction {
    pub(crate) const fn action_name(self) -> &'static str {
        match self {
            Self::Save => "edit_text_save",
            Self::Cancel => "edit_text_cancel",
        }
    }

    pub(crate) const fn button_label(self) -> &'static str {
        match self {
            Self::Save => "Save",
            Self::Cancel => "Cancel",
        }
    }
}

pub(crate) const REQUIRED_NATIVE_HOST_EDIT_TEXT_ACTIONS: [NativeHostEditTextAction; 2] = [
    NativeHostEditTextAction::Save,
    NativeHostEditTextAction::Cancel,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeHostDialogAction {
    ShowInfoMessage,
    ConfirmQuestion,
}

impl NativeHostDialogAction {
    pub(crate) const fn action_name(self) -> &'static str {
        match self {
            Self::ShowInfoMessage => "dialog_show_info_message",
            Self::ConfirmQuestion => "dialog_confirm_question",
        }
    }

    pub(crate) const fn button_label(self) -> &'static str {
        match self {
            Self::ShowInfoMessage => "Info Dialog",
            Self::ConfirmQuestion => "Confirm",
        }
    }

    pub(crate) const fn title(self) -> &'static str {
        match self {
            Self::ShowInfoMessage => "ZSClip",
            Self::ConfirmQuestion => "Confirm Native Dialog",
        }
    }

    pub(crate) const fn message(self) -> &'static str {
        match self {
            Self::ShowInfoMessage => {
                "This message is presented by the platform native dialog host."
            }
            Self::ConfirmQuestion => "Route this confirmation through the native dialog host?",
        }
    }
}

pub(crate) const REQUIRED_NATIVE_HOST_DIALOG_ACTIONS: [NativeHostDialogAction; 2] = [
    NativeHostDialogAction::ShowInfoMessage,
    NativeHostDialogAction::ConfirmQuestion,
];

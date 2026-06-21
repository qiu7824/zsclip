use super::{
    Command, CommandId, CommandPayload, CommandScope, MainRowMenuAction,
    MainSearchVisibilityRequest, MainShortcutEscapePlan, MainShortcutRowCommand,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainHostAction {
    ToggleSearch,
    OpenSettings,
    HideWindow,
    CloseWindow,
    InvokeMenuCommand(MainMenuCommandIntent),
}

pub(crate) fn main_host_action_for_command(command: &Command) -> Option<MainHostAction> {
    if command.scope != CommandScope::Window {
        return None;
    }
    if command.id == command_ids::TOGGLE_SEARCH {
        Some(MainHostAction::ToggleSearch)
    } else if command.id == command_ids::OPEN_SETTINGS {
        Some(MainHostAction::OpenSettings)
    } else if command.id == command_ids::HIDE_WINDOW {
        Some(MainHostAction::HideWindow)
    } else if command.id == command_ids::CLOSE_WINDOW {
        Some(MainHostAction::CloseWindow)
    } else if command.id == command_ids::INVOKE_MAIN_MENU_COMMAND {
        match command.payload {
            CommandPayload::ControlId(id) if id >= 0 => {
                main_menu_command_intent_for_id(id as usize).map(MainHostAction::InvokeMenuCommand)
            }
            _ => None,
        }
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainHostExecutionPlan {
    Search(MainSearchVisibilityRequest),
    OpenSettings,
    HideWindow,
    CloseWindow,
    InvokeMenuCommand(MainMenuCommandIntent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainHostExecutionPlanKind {
    Search,
    OpenSettings,
    HideWindow,
    CloseWindow,
    InvokeMenuCommand,
}

impl MainHostExecutionPlanKind {
    pub(crate) const fn plan_name(self) -> &'static str {
        match self {
            Self::Search => "Search",
            Self::OpenSettings => "OpenSettings",
            Self::HideWindow => "HideWindow",
            Self::CloseWindow => "CloseWindow",
            Self::InvokeMenuCommand => "InvokeMenuCommand",
        }
    }
}

pub(crate) const REQUIRED_MAIN_HOST_EXECUTION_PLAN_KINDS: [MainHostExecutionPlanKind; 5] = [
    MainHostExecutionPlanKind::Search,
    MainHostExecutionPlanKind::OpenSettings,
    MainHostExecutionPlanKind::HideWindow,
    MainHostExecutionPlanKind::CloseWindow,
    MainHostExecutionPlanKind::InvokeMenuCommand,
];

impl MainHostExecutionPlan {
    pub(crate) const fn kind(self) -> MainHostExecutionPlanKind {
        match self {
            Self::Search(_) => MainHostExecutionPlanKind::Search,
            Self::OpenSettings => MainHostExecutionPlanKind::OpenSettings,
            Self::HideWindow => MainHostExecutionPlanKind::HideWindow,
            Self::CloseWindow => MainHostExecutionPlanKind::CloseWindow,
            Self::InvokeMenuCommand(_) => MainHostExecutionPlanKind::InvokeMenuCommand,
        }
    }
}

pub(crate) fn main_host_execution_plan(action: MainHostAction) -> MainHostExecutionPlan {
    match action {
        MainHostAction::ToggleSearch => {
            MainHostExecutionPlan::Search(MainSearchVisibilityRequest::Toggle)
        }
        MainHostAction::OpenSettings => MainHostExecutionPlan::OpenSettings,
        MainHostAction::HideWindow => MainHostExecutionPlan::HideWindow,
        MainHostAction::CloseWindow => MainHostExecutionPlan::CloseWindow,
        MainHostAction::InvokeMenuCommand(intent) => {
            MainHostExecutionPlan::InvokeMenuCommand(intent)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShortcutKey {
    Up,
    Down,
    Enter,
    A,
    C,
    Delete,
    Escape,
    P,
    F,
    Other(u32),
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShortcutModifiers {
    pub(crate) ctrl: bool,
    pub(crate) shift: bool,
    pub(crate) alt: bool,
    pub(crate) meta: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainShortcutAction {
    MoveSelection { delta: i32, extend: bool },
    ActivateSelection,
    SelectAll,
    CopySelection,
    DeleteSelection,
    Escape,
    TogglePin,
    ToggleSearch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainWindowCommandIntent {
    ToggleSearch,
    OpenSettings,
    HideWindow,
    CloseWindow,
}

pub(crate) fn main_shortcut_action(
    key: ShortcutKey,
    modifiers: ShortcutModifiers,
) -> Option<MainShortcutAction> {
    if modifiers.alt || modifiers.meta {
        return None;
    }
    match key {
        ShortcutKey::Up if !modifiers.ctrl => Some(MainShortcutAction::MoveSelection {
            delta: -1,
            extend: modifiers.shift,
        }),
        ShortcutKey::Down if !modifiers.ctrl => Some(MainShortcutAction::MoveSelection {
            delta: 1,
            extend: modifiers.shift,
        }),
        ShortcutKey::Enter if !modifiers.ctrl && !modifiers.shift => {
            Some(MainShortcutAction::ActivateSelection)
        }
        ShortcutKey::A if modifiers.ctrl && !modifiers.shift => Some(MainShortcutAction::SelectAll),
        ShortcutKey::C if modifiers.ctrl && !modifiers.shift => {
            Some(MainShortcutAction::CopySelection)
        }
        ShortcutKey::Delete if !modifiers.ctrl && !modifiers.shift => {
            Some(MainShortcutAction::DeleteSelection)
        }
        ShortcutKey::Escape if !modifiers.ctrl && !modifiers.shift => {
            Some(MainShortcutAction::Escape)
        }
        ShortcutKey::P if modifiers.ctrl && !modifiers.shift => Some(MainShortcutAction::TogglePin),
        ShortcutKey::F if modifiers.ctrl && !modifiers.shift => {
            Some(MainShortcutAction::ToggleSearch)
        }
        _ => None,
    }
}

pub(crate) fn main_shortcut_window_command_for_action(
    action: MainShortcutAction,
) -> Option<MainWindowCommandIntent> {
    match action {
        MainShortcutAction::ToggleSearch => Some(MainWindowCommandIntent::ToggleSearch),
        _ => None,
    }
}

pub(crate) fn main_title_button_window_command_for_key(key: &str) -> MainWindowCommandIntent {
    match key {
        "search" => MainWindowCommandIntent::ToggleSearch,
        "setting" => MainWindowCommandIntent::OpenSettings,
        "min" => MainWindowCommandIntent::HideWindow,
        "close" => MainWindowCommandIntent::CloseWindow,
        _ => MainWindowCommandIntent::CloseWindow,
    }
}

pub(crate) fn main_shortcut_row_command_for_action(
    action: MainShortcutAction,
) -> Option<MainShortcutRowCommand> {
    match action {
        MainShortcutAction::CopySelection => Some(MainShortcutRowCommand::CopySelection),
        MainShortcutAction::DeleteSelection => Some(MainShortcutRowCommand::DeleteSelection),
        MainShortcutAction::TogglePin => Some(MainShortcutRowCommand::TogglePin),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainShortcutExecutionPlan {
    MoveSelection { delta: i32, extend: bool },
    ActivateSelection,
    SelectAll,
    RowCommand(MainShortcutRowCommand),
    ClearSelection,
    CloseSearch,
    WindowCommand(MainWindowCommandIntent),
    Noop,
}

pub(crate) fn main_shortcut_execution_plan(
    action: MainShortcutAction,
    escape_plan: Option<MainShortcutEscapePlan>,
) -> MainShortcutExecutionPlan {
    match action {
        MainShortcutAction::MoveSelection { delta, extend } => {
            MainShortcutExecutionPlan::MoveSelection { delta, extend }
        }
        MainShortcutAction::ActivateSelection => MainShortcutExecutionPlan::ActivateSelection,
        MainShortcutAction::SelectAll => MainShortcutExecutionPlan::SelectAll,
        MainShortcutAction::CopySelection
        | MainShortcutAction::DeleteSelection
        | MainShortcutAction::TogglePin => main_shortcut_row_command_for_action(action)
            .map(MainShortcutExecutionPlan::RowCommand)
            .unwrap_or(MainShortcutExecutionPlan::Noop),
        MainShortcutAction::Escape => match escape_plan {
            Some(MainShortcutEscapePlan::ClearSelection) => {
                MainShortcutExecutionPlan::ClearSelection
            }
            Some(MainShortcutEscapePlan::CloseSearch) => MainShortcutExecutionPlan::CloseSearch,
            Some(MainShortcutEscapePlan::HideWindow) => {
                MainShortcutExecutionPlan::WindowCommand(MainWindowCommandIntent::HideWindow)
            }
            None => MainShortcutExecutionPlan::Noop,
        },
        MainShortcutAction::ToggleSearch => main_shortcut_window_command_for_action(action)
            .map(MainShortcutExecutionPlan::WindowCommand)
            .unwrap_or(MainShortcutExecutionPlan::Noop),
    }
}

pub(crate) mod command_ids {
    use super::CommandId;

    pub(crate) const TOGGLE_SEARCH: CommandId = CommandId("window.search.toggle");
    pub(crate) const UPDATE_SEARCH_TEXT: CommandId = CommandId("window.search.text.update");
    pub(crate) const INVOKE_MAIN_MENU_COMMAND: CommandId = CommandId("window.menu.invoke");
    pub(crate) const OPEN_SETTINGS: CommandId = CommandId("window.settings.open");
    pub(crate) const SAVE_SETTINGS: CommandId = CommandId("window.settings.save");
    pub(crate) const CLOSE_SETTINGS: CommandId = CommandId("window.settings.close");
    pub(crate) const OPEN_SETTINGS_CONFIG: CommandId = CommandId("window.settings.config.open");
    pub(crate) const OPEN_SETTINGS_DROPDOWN: CommandId = CommandId("window.settings.dropdown.open");
    pub(crate) const TOGGLE_SETTINGS_CONTROL: CommandId =
        CommandId("window.settings.control.toggle");
    pub(crate) const HIDE_WINDOW: CommandId = CommandId("window.hide");
    pub(crate) const CLOSE_WINDOW: CommandId = CommandId("window.close");
}

pub(crate) mod menu_ids {
    pub(crate) const TRAY_TOGGLE: usize = 40001;
    pub(crate) const TRAY_EXIT: usize = 40002;
    pub(crate) const TRAY_LAN_TOGGLE: usize = 40003;
    pub(crate) const TRAY_CAPTURE_TOGGLE: usize = 40004;
    pub(crate) const ROW_PASTE: usize = 41001;
    pub(crate) const ROW_COPY: usize = 41002;
    pub(crate) const ROW_PIN: usize = 41003;
    pub(crate) const ROW_DELETE: usize = 41004;
    pub(crate) const ROW_TO_PHRASE: usize = 41005;
    pub(crate) const ROW_STICKER: usize = 41006;
    pub(crate) const ROW_SAVE_IMAGE: usize = 41007;
    pub(crate) const ROW_OPEN_PATH: usize = 41008;
    pub(crate) const ROW_OPEN_FOLDER: usize = 41009;
    pub(crate) const ROW_COPY_PATH: usize = 41010;
    pub(crate) const ROW_GROUP_REMOVE: usize = 41011;
    pub(crate) const ROW_EDIT: usize = 41012;
    pub(crate) const ROW_QUICK_SEARCH: usize = 41013;
    pub(crate) const ROW_EXPORT_FILE: usize = 41014;
    pub(crate) const ROW_MAIL_MERGE: usize = 41015;
    pub(crate) const ROW_DELETE_UNPINNED: usize = 41016;
    pub(crate) const ROW_IMAGE_OCR: usize = 41017;
    pub(crate) const ROW_QR_IMAGE: usize = 41018;
    pub(crate) const ROW_TEXT_TRANSLATE: usize = 41019;
    pub(crate) const ROW_LAN_PUSH: usize = 41020;
    pub(crate) const ROW_GROUP_BASE: usize = 41100;
    pub(crate) const GROUP_FILTER_ALL: usize = 43100;
    pub(crate) const GROUP_FILTER_BASE: usize = 43110;
    pub(crate) const DYNAMIC_GROUP_LIMIT: usize = 2000;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainGroupFilterSelection {
    All,
    Group { index: usize },
}

pub(crate) fn main_group_filter_menu_all_id() -> usize {
    menu_ids::GROUP_FILTER_ALL
}

pub(crate) fn main_group_filter_menu_group_id(index: usize) -> usize {
    menu_ids::GROUP_FILTER_BASE + index
}

pub(crate) fn main_group_filter_selection_for_id(id: usize) -> Option<MainGroupFilterSelection> {
    if id == menu_ids::GROUP_FILTER_ALL {
        return Some(MainGroupFilterSelection::All);
    }
    if (menu_ids::GROUP_FILTER_BASE..menu_ids::GROUP_FILTER_BASE + menu_ids::DYNAMIC_GROUP_LIMIT)
        .contains(&id)
    {
        return Some(MainGroupFilterSelection::Group {
            index: id - menu_ids::GROUP_FILTER_BASE,
        });
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainRowGroupSelection {
    Group { index: usize },
}

pub(crate) fn main_row_group_menu_group_id(index: usize) -> usize {
    menu_ids::ROW_GROUP_BASE + index
}

pub(crate) fn main_row_group_selection_for_id(id: usize) -> Option<MainRowGroupSelection> {
    if (menu_ids::ROW_GROUP_BASE..menu_ids::ROW_GROUP_BASE + menu_ids::DYNAMIC_GROUP_LIMIT)
        .contains(&id)
    {
        return Some(MainRowGroupSelection::Group {
            index: id - menu_ids::ROW_GROUP_BASE,
        });
    }
    None
}

pub(crate) fn main_row_menu_action_id(action: MainRowMenuAction) -> usize {
    match action {
        MainRowMenuAction::Copy => menu_ids::ROW_COPY,
        MainRowMenuAction::Pin => menu_ids::ROW_PIN,
        MainRowMenuAction::ToPhrase => menu_ids::ROW_TO_PHRASE,
        MainRowMenuAction::AddToGroup => menu_ids::ROW_GROUP_BASE,
        MainRowMenuAction::RemoveFromGroup => menu_ids::ROW_GROUP_REMOVE,
        MainRowMenuAction::Delete => menu_ids::ROW_DELETE,
        MainRowMenuAction::DeleteUnpinned => menu_ids::ROW_DELETE_UNPINNED,
        MainRowMenuAction::Sticker => menu_ids::ROW_STICKER,
        MainRowMenuAction::SaveImage => menu_ids::ROW_SAVE_IMAGE,
        MainRowMenuAction::ImageOcr => menu_ids::ROW_IMAGE_OCR,
        MainRowMenuAction::ExportFile => menu_ids::ROW_EXPORT_FILE,
        MainRowMenuAction::OpenPath => menu_ids::ROW_OPEN_PATH,
        MainRowMenuAction::OpenFolder => menu_ids::ROW_OPEN_FOLDER,
        MainRowMenuAction::CopyPath => menu_ids::ROW_COPY_PATH,
        MainRowMenuAction::QrImage => menu_ids::ROW_QR_IMAGE,
        MainRowMenuAction::MailMerge => menu_ids::ROW_MAIL_MERGE,
        MainRowMenuAction::LanPush => menu_ids::ROW_LAN_PUSH,
        MainRowMenuAction::Edit => menu_ids::ROW_EDIT,
        MainRowMenuAction::QuickSearch => menu_ids::ROW_QUICK_SEARCH,
        MainRowMenuAction::TextTranslate => menu_ids::ROW_TEXT_TRANSLATE,
    }
}

pub(crate) fn main_row_menu_action_for_id(id: usize) -> Option<MainRowMenuAction> {
    Some(match id {
        menu_ids::ROW_COPY => MainRowMenuAction::Copy,
        menu_ids::ROW_PIN => MainRowMenuAction::Pin,
        menu_ids::ROW_TO_PHRASE => MainRowMenuAction::ToPhrase,
        menu_ids::ROW_GROUP_REMOVE => MainRowMenuAction::RemoveFromGroup,
        menu_ids::ROW_DELETE => MainRowMenuAction::Delete,
        menu_ids::ROW_DELETE_UNPINNED => MainRowMenuAction::DeleteUnpinned,
        menu_ids::ROW_STICKER => MainRowMenuAction::Sticker,
        menu_ids::ROW_SAVE_IMAGE => MainRowMenuAction::SaveImage,
        menu_ids::ROW_IMAGE_OCR => MainRowMenuAction::ImageOcr,
        menu_ids::ROW_EXPORT_FILE => MainRowMenuAction::ExportFile,
        menu_ids::ROW_OPEN_PATH => MainRowMenuAction::OpenPath,
        menu_ids::ROW_OPEN_FOLDER => MainRowMenuAction::OpenFolder,
        menu_ids::ROW_COPY_PATH => MainRowMenuAction::CopyPath,
        menu_ids::ROW_QR_IMAGE => MainRowMenuAction::QrImage,
        menu_ids::ROW_MAIL_MERGE => MainRowMenuAction::MailMerge,
        menu_ids::ROW_LAN_PUSH => MainRowMenuAction::LanPush,
        menu_ids::ROW_EDIT => MainRowMenuAction::Edit,
        menu_ids::ROW_QUICK_SEARCH => MainRowMenuAction::QuickSearch,
        menu_ids::ROW_TEXT_TRANSLATE => MainRowMenuAction::TextTranslate,
        _ => return None,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainMenuCommandIntent {
    Tray(MainTrayMenuAction),
    RowPaste,
    RowAction(MainRowMenuAction),
    AssignRowGroup { index: usize },
    GroupFilterAll,
    GroupFilter { index: usize },
}

pub(crate) fn main_menu_command_intent_for_id(id: usize) -> Option<MainMenuCommandIntent> {
    let tray_action = match id {
        menu_ids::TRAY_TOGGLE => Some(MainTrayMenuAction::ToggleWindow),
        menu_ids::TRAY_CAPTURE_TOGGLE => Some(MainTrayMenuAction::ToggleClipboardCapture),
        menu_ids::TRAY_LAN_TOGGLE => Some(MainTrayMenuAction::ToggleLanSync),
        menu_ids::TRAY_EXIT => Some(MainTrayMenuAction::Exit),
        _ => None,
    };
    if let Some(action) = tray_action {
        return Some(MainMenuCommandIntent::Tray(action));
    }
    if id == menu_ids::ROW_PASTE {
        return Some(MainMenuCommandIntent::RowPaste);
    }
    if let Some(action) = main_row_menu_action_for_id(id) {
        return Some(MainMenuCommandIntent::RowAction(action));
    }
    if id == menu_ids::GROUP_FILTER_ALL {
        return Some(MainMenuCommandIntent::GroupFilterAll);
    }
    if let Some(MainRowGroupSelection::Group { index }) = main_row_group_selection_for_id(id) {
        return Some(MainMenuCommandIntent::AssignRowGroup { index });
    }
    if (menu_ids::GROUP_FILTER_BASE..menu_ids::GROUP_FILTER_BASE + menu_ids::DYNAMIC_GROUP_LIMIT)
        .contains(&id)
    {
        return Some(MainMenuCommandIntent::GroupFilter {
            index: id - menu_ids::GROUP_FILTER_BASE,
        });
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainTrayMenuAction {
    ToggleWindow,
    ToggleClipboardCapture,
    ToggleLanSync,
    Exit,
}

impl MainTrayMenuAction {
    pub(crate) const fn command_id(self) -> usize {
        match self {
            Self::ToggleWindow => menu_ids::TRAY_TOGGLE,
            Self::ToggleClipboardCapture => menu_ids::TRAY_CAPTURE_TOGGLE,
            Self::ToggleLanSync => menu_ids::TRAY_LAN_TOGGLE,
            Self::Exit => menu_ids::TRAY_EXIT,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainTrayMenuText {
    ToggleWindow,
    EnableClipboardCapture,
    DisableClipboardCapture,
    LanSyncOn,
    LanSyncOff,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainTrayMenuItem {
    Command {
        action: MainTrayMenuAction,
        text: MainTrayMenuText,
    },
    Separator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MainTrayMenuInput {
    pub(crate) clipboard_capture_enabled: bool,
    pub(crate) lan_sync_enabled: bool,
}

pub(crate) fn main_tray_menu_plan(input: MainTrayMenuInput) -> Vec<MainTrayMenuItem> {
    vec![
        MainTrayMenuItem::Command {
            action: MainTrayMenuAction::ToggleWindow,
            text: MainTrayMenuText::ToggleWindow,
        },
        MainTrayMenuItem::Command {
            action: MainTrayMenuAction::ToggleClipboardCapture,
            text: if input.clipboard_capture_enabled {
                MainTrayMenuText::DisableClipboardCapture
            } else {
                MainTrayMenuText::EnableClipboardCapture
            },
        },
        MainTrayMenuItem::Command {
            action: MainTrayMenuAction::ToggleLanSync,
            text: if input.lan_sync_enabled {
                MainTrayMenuText::LanSyncOn
            } else {
                MainTrayMenuText::LanSyncOff
            },
        },
        MainTrayMenuItem::Separator,
        MainTrayMenuItem::Command {
            action: MainTrayMenuAction::Exit,
            text: MainTrayMenuText::Exit,
        },
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MainTrayActionInput {
    pub(crate) action: MainTrayMenuAction,
    pub(crate) clipboard_capture_enabled: bool,
    pub(crate) lan_sync_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainTrayActionPlan {
    ToggleWindow,
    SetClipboardCapture { enabled: bool },
    SetLanSync { enabled: bool },
    Exit,
}

pub(crate) fn main_tray_action_plan(input: MainTrayActionInput) -> MainTrayActionPlan {
    match input.action {
        MainTrayMenuAction::ToggleWindow => MainTrayActionPlan::ToggleWindow,
        MainTrayMenuAction::ToggleClipboardCapture => MainTrayActionPlan::SetClipboardCapture {
            enabled: !input.clipboard_capture_enabled,
        },
        MainTrayMenuAction::ToggleLanSync => MainTrayActionPlan::SetLanSync {
            enabled: !input.lan_sync_enabled,
        },
        MainTrayMenuAction::Exit => MainTrayActionPlan::Exit,
    }
}

fn main_menu_id_is_static_command(id: usize) -> bool {
    matches!(
        id,
        menu_ids::TRAY_TOGGLE
            | menu_ids::TRAY_LAN_TOGGLE
            | menu_ids::TRAY_CAPTURE_TOGGLE
            | menu_ids::TRAY_EXIT
            | menu_ids::ROW_PASTE
            | menu_ids::ROW_COPY
            | menu_ids::ROW_PIN
            | menu_ids::ROW_DELETE
            | menu_ids::ROW_DELETE_UNPINNED
            | menu_ids::ROW_TO_PHRASE
            | menu_ids::ROW_STICKER
            | menu_ids::ROW_SAVE_IMAGE
            | menu_ids::ROW_IMAGE_OCR
            | menu_ids::ROW_TEXT_TRANSLATE
            | menu_ids::ROW_QR_IMAGE
            | menu_ids::ROW_LAN_PUSH
            | menu_ids::ROW_OPEN_PATH
            | menu_ids::ROW_OPEN_FOLDER
            | menu_ids::ROW_COPY_PATH
            | menu_ids::ROW_GROUP_REMOVE
            | menu_ids::ROW_EDIT
            | menu_ids::ROW_QUICK_SEARCH
            | menu_ids::ROW_EXPORT_FILE
            | menu_ids::ROW_MAIL_MERGE
            | menu_ids::GROUP_FILTER_ALL
    )
}

fn main_menu_id_is_dynamic_group_command(id: usize) -> bool {
    (menu_ids::ROW_GROUP_BASE..menu_ids::ROW_GROUP_BASE + menu_ids::DYNAMIC_GROUP_LIMIT)
        .contains(&id)
        || (menu_ids::GROUP_FILTER_BASE
            ..menu_ids::GROUP_FILTER_BASE + menu_ids::DYNAMIC_GROUP_LIMIT)
            .contains(&id)
}

pub(crate) fn main_menu_command_for_id(id: usize) -> Option<Command> {
    if main_menu_id_is_static_command(id) || main_menu_id_is_dynamic_group_command(id) {
        Some(Command::window_with_payload(
            command_ids::INVOKE_MAIN_MENU_COMMAND,
            CommandPayload::ControlId(id as i64),
        ))
    } else {
        None
    }
}

pub(crate) fn main_menu_command_for_shortcut_row_command(
    command: MainShortcutRowCommand,
) -> Command {
    let id = match command {
        MainShortcutRowCommand::CopySelection => menu_ids::ROW_COPY,
        MainShortcutRowCommand::DeleteSelection => menu_ids::ROW_DELETE,
        MainShortcutRowCommand::TogglePin => menu_ids::ROW_PIN,
    };
    Command::window_with_payload(
        command_ids::INVOKE_MAIN_MENU_COMMAND,
        CommandPayload::ControlId(id as i64),
    )
}

pub(crate) fn main_window_command_for_intent(command: MainWindowCommandIntent) -> Command {
    match command {
        MainWindowCommandIntent::ToggleSearch => Command::window(command_ids::TOGGLE_SEARCH),
        MainWindowCommandIntent::OpenSettings => Command::window(command_ids::OPEN_SETTINGS),
        MainWindowCommandIntent::HideWindow => Command::window(command_ids::HIDE_WINDOW),
        MainWindowCommandIntent::CloseWindow => Command::window(command_ids::CLOSE_WINDOW),
    }
}

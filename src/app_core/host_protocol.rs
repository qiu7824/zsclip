#![allow(unused_imports)]

use super::{menu_ids, MainTrayMenuAction};

pub(crate) use zsui::{
    clipboard_monitor_poll_result_for_sequence, native_paste_target_activation_snapshot,
    native_window_identity_snapshot, poll_clipboard_monitor, ClipboardHost,
    ClipboardMonitorPollResult, ClipboardMonitorState, NativeAutostartApplyResult,
    NativeAutostartHost, NativeAutostartStatus, NativeDialogButtons, NativeDialogHost,
    NativeDialogHostOperation, NativeDialogLevel, NativeDialogResponse, NativeEditTextDialogHost,
    NativeEditTextDialogHostOperation, NativeEditTextDialogRequest, NativeEditTextDialogResult,
    NativeEditTextSaveHandler, NativeFileDialogHost, NativeFileDialogHostOperation,
    NativeFileDialogRequest, NativeImeCandidateAnchor, NativeImeCompositionAnchor, NativeImeHost,
    NativeImeHostOperation, NativeMailMergeWindowHost, NativeMailMergeWindowHostOperation,
    NativeMailMergeWindowRequest, NativePasteTargetActivationSnapshot, NativePasteTargetHost,
    NativePasteTargetHostOperation, NativePopupMenuEntry, NativePopupMenuHost,
    NativePopupMenuHostOperation, NativePopupMenuPlacement, NativeShellOpenHost,
    NativeShellOpenHostOperation, NativeTextCaretAnchor, NativeTextCaretHost,
    NativeTextCaretHostOperation, NativeTextInputDialogHost, NativeTextInputDialogHostOperation,
    NativeTextInputDialogRequest, NativeTransientWindowHost, NativeTransientWindowHostOperation,
    NativeTransientWindowPresentation, NativeTransientWindowRequest, NativeWindowIdentityHost,
    NativeWindowIdentityHostOperation, NativeWindowIdentitySnapshot, PasteTargetFocusStatus,
    PasteTargetTextInputCapabilities, REQUIRED_NATIVE_DIALOG_HOST_OPERATIONS,
    REQUIRED_NATIVE_EDIT_TEXT_DIALOG_HOST_OPERATIONS, REQUIRED_NATIVE_FILE_DIALOG_HOST_OPERATIONS,
    REQUIRED_NATIVE_IME_HOST_OPERATIONS, REQUIRED_NATIVE_MAIL_MERGE_WINDOW_HOST_OPERATIONS,
    REQUIRED_NATIVE_PASTE_TARGET_HOST_OPERATIONS, REQUIRED_NATIVE_POPUP_MENU_HOST_OPERATIONS,
    REQUIRED_NATIVE_SHELL_OPEN_HOST_OPERATIONS, REQUIRED_NATIVE_TEXT_CARET_HOST_OPERATIONS,
    REQUIRED_NATIVE_TEXT_INPUT_DIALOG_HOST_OPERATIONS,
    REQUIRED_NATIVE_TRANSIENT_WINDOW_HOST_OPERATIONS,
    REQUIRED_NATIVE_WINDOW_IDENTITY_HOST_OPERATIONS,
};
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StatusMenuEntry {
    Command {
        action: MainTrayMenuAction,
        label: String,
        icon_name: String,
    },
    Separator,
}

pub(crate) trait StatusItemHost {
    fn install(&mut self, tooltip: &str) -> bool;
    fn remove(&mut self);
    fn present_menu(&mut self, entries: &[StatusMenuEntry]);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StatusItemHostOperation {
    Install,
    Remove,
    PresentMenu,
}

impl StatusItemHostOperation {
    pub(crate) const fn operation_name(self) -> &'static str {
        match self {
            Self::Install => "install_status_item",
            Self::Remove => "remove_status_item",
            Self::PresentMenu => "present_status_menu",
        }
    }
}

pub(crate) const REQUIRED_STATUS_ITEM_HOST_OPERATIONS: [StatusItemHostOperation; 3] = [
    StatusItemHostOperation::Install,
    StatusItemHostOperation::Remove,
    StatusItemHostOperation::PresentMenu,
];

pub(crate) const fn native_popup_menu_command_icon_name(menu_id: usize) -> Option<&'static str> {
    match menu_id {
        menu_ids::ROW_COPY => Some("edit-copy-symbolic"),
        menu_ids::ROW_PASTE => Some("edit-paste-symbolic"),
        menu_ids::ROW_EDIT => Some("document-edit-symbolic"),
        menu_ids::ROW_DELETE | menu_ids::ROW_DELETE_UNPINNED => Some("user-trash-symbolic"),
        menu_ids::ROW_PIN => Some("view-pin-symbolic"),
        menu_ids::ROW_COPY_PATH => Some("document-open-symbolic"),
        menu_ids::ROW_GROUP_REMOVE => Some("list-remove-symbolic"),
        menu_ids::GROUP_FILTER_ALL => Some("view-list-symbolic"),
        _ => None,
    }
}

pub(crate) const fn native_popup_menu_command_zsui_icon(
    menu_id: usize,
) -> Option<crate::zsui::ZsIcon> {
    match menu_id {
        menu_ids::ROW_COPY => Some(crate::zsui::ZsIcon::Copy),
        menu_ids::ROW_PASTE => Some(crate::zsui::ZsIcon::Paste),
        menu_ids::ROW_EDIT => Some(crate::zsui::ZsIcon::Edit),
        menu_ids::ROW_DELETE | menu_ids::ROW_DELETE_UNPINNED => Some(crate::zsui::ZsIcon::Delete),
        menu_ids::ROW_PIN => Some(crate::zsui::ZsIcon::Pin),
        menu_ids::ROW_COPY_PATH => Some(crate::zsui::ZsIcon::File),
        menu_ids::ROW_GROUP_REMOVE => Some(crate::zsui::ZsIcon::Group),
        menu_ids::GROUP_FILTER_ALL => Some(crate::zsui::ZsIcon::Group),
        _ => None,
    }
}

pub(crate) const fn native_popup_menu_command_accelerator_label(
    menu_id: usize,
) -> Option<&'static str> {
    match menu_id {
        menu_ids::ROW_COPY => Some("Ctrl+C"),
        menu_ids::ROW_PASTE => Some("Ctrl+V"),
        menu_ids::ROW_EDIT => Some("Enter"),
        menu_ids::ROW_DELETE | menu_ids::ROW_DELETE_UNPINNED => Some("Delete"),
        menu_ids::ROW_PIN => Some("P"),
        _ => None,
    }
}

pub(crate) const fn native_popup_menu_command_macos_symbol_name(
    menu_id: usize,
) -> Option<&'static str> {
    match menu_id {
        menu_ids::ROW_COPY => Some("doc.on.doc"),
        menu_ids::ROW_PASTE => Some("doc.on.clipboard"),
        menu_ids::ROW_EDIT => Some("pencil"),
        menu_ids::ROW_DELETE | menu_ids::ROW_DELETE_UNPINNED => Some("trash"),
        menu_ids::ROW_PIN => Some("pin"),
        menu_ids::ROW_COPY_PATH => Some("doc.text.magnifyingglass"),
        menu_ids::ROW_GROUP_REMOVE => Some("minus.circle"),
        menu_ids::GROUP_FILTER_ALL => Some("line.3.horizontal"),
        _ => None,
    }
}

pub(crate) const fn native_popup_menu_command_macos_key_equivalent(
    menu_id: usize,
) -> Option<&'static str> {
    match menu_id {
        menu_ids::ROW_COPY => Some("c"),
        menu_ids::ROW_PASTE => Some("v"),
        menu_ids::ROW_EDIT => Some("\r"),
        menu_ids::ROW_DELETE | menu_ids::ROW_DELETE_UNPINNED => Some("\u{8}"),
        menu_ids::ROW_PIN => Some("p"),
        _ => None,
    }
}

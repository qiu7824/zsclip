use super::{menu_ids, MainTrayMenuAction, Point, Size, UiRect};

pub(crate) trait ClipboardHost {
    fn read_text() -> Option<String>;
    fn write_text(text: &str) -> bool;
    fn read_image_rgba() -> Option<(Vec<u8>, usize, usize)>;
    fn write_image_rgba(bytes: &[u8], width: usize, height: usize) -> bool;
    fn read_file_paths() -> Option<Vec<String>>;
    fn write_file_paths(paths: &[String]) -> bool;
    fn sequence_number() -> u32;
    fn write_text_ignored_by_monitors(text: &str) -> bool;
    fn should_ignore_capture_by_named_format() -> bool;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClipboardMonitorState {
    last_sequence: Option<u32>,
}

impl ClipboardMonitorState {
    pub(crate) const fn last_sequence(self) -> Option<u32> {
        self.last_sequence
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClipboardMonitorPollResult {
    Disabled { sequence: u32 },
    Baseline { sequence: u32 },
    Unchanged { sequence: u32 },
    IgnoredSelfWrite { sequence: u32 },
    Changed { sequence: u32 },
}

impl ClipboardMonitorPollResult {
    pub(crate) const fn should_bridge_application_event(self) -> bool {
        matches!(self, Self::Changed { .. })
    }

    pub(crate) const fn sequence(self) -> u32 {
        match self {
            Self::Disabled { sequence }
            | Self::Baseline { sequence }
            | Self::Unchanged { sequence }
            | Self::IgnoredSelfWrite { sequence }
            | Self::Changed { sequence } => sequence,
        }
    }
}

pub(crate) fn clipboard_monitor_poll_result_for_sequence(
    state: &mut ClipboardMonitorState,
    capture_enabled: bool,
    current_sequence: u32,
    ignore_next_capture: bool,
) -> ClipboardMonitorPollResult {
    if !capture_enabled {
        state.last_sequence = Some(current_sequence);
        return ClipboardMonitorPollResult::Disabled {
            sequence: current_sequence,
        };
    }

    let previous = state.last_sequence.replace(current_sequence);
    match previous {
        None => ClipboardMonitorPollResult::Baseline {
            sequence: current_sequence,
        },
        Some(previous_sequence) if previous_sequence == current_sequence => {
            ClipboardMonitorPollResult::Unchanged {
                sequence: current_sequence,
            }
        }
        Some(_) if ignore_next_capture => ClipboardMonitorPollResult::IgnoredSelfWrite {
            sequence: current_sequence,
        },
        Some(_) => ClipboardMonitorPollResult::Changed {
            sequence: current_sequence,
        },
    }
}

pub(crate) fn poll_clipboard_monitor<H: ClipboardHost>(
    state: &mut ClipboardMonitorState,
    capture_enabled: bool,
) -> ClipboardMonitorPollResult {
    let current_sequence = H::sequence_number();
    let ignore_next_capture = capture_enabled
        && state.last_sequence != Some(current_sequence)
        && H::should_ignore_capture_by_named_format();
    clipboard_monitor_poll_result_for_sequence(
        state,
        capture_enabled,
        current_sequence,
        ignore_next_capture,
    )
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativePopupMenuPlacement {
    TopLeft,
    BottomLeft,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NativePopupMenuEntry {
    Command {
        id: usize,
        label: String,
        enabled: bool,
        checked: bool,
    },
    Submenu {
        label: String,
        enabled: bool,
        entries: Vec<NativePopupMenuEntry>,
    },
    Separator,
}

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

pub(crate) trait NativePopupMenuHost {
    type Owner;

    fn present_popup_menu(
        &mut self,
        owner: Self::Owner,
        x: i32,
        y: i32,
        placement: NativePopupMenuPlacement,
        entries: &[NativePopupMenuEntry],
    ) -> usize;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativePopupMenuHostOperation {
    PresentPopupMenu,
}

impl NativePopupMenuHostOperation {
    pub(crate) const fn operation_name(self) -> &'static str {
        match self {
            Self::PresentPopupMenu => "present_popup_menu",
        }
    }
}

pub(crate) const REQUIRED_NATIVE_POPUP_MENU_HOST_OPERATIONS: [NativePopupMenuHostOperation; 1] =
    [NativePopupMenuHostOperation::PresentPopupMenu];

pub(crate) trait NativeTransientWindowHost {
    type Handle: Copy + Eq;
    type Owner: Copy + Eq;

    fn create_transient_window(
        &mut self,
        request: NativeTransientWindowRequest<Self::Owner>,
    ) -> NativeTransientWindowPresentation<Self::Handle>;
    fn present_transient_window(&mut self, handle: Self::Handle, bounds: UiRect);
    fn hide_transient_window(&mut self, handle: Self::Handle);
    fn destroy_transient_window(&mut self, handle: Self::Handle);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeTransientWindowRequest<Owner: Copy + Eq> {
    pub(crate) owner: Owner,
    pub(crate) bounds: UiRect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeTransientWindowPresentation<Handle: Copy + Eq> {
    Created(Handle),
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeTransientWindowHostOperation {
    CreateTransientWindow,
    PresentTransientWindow,
    HideTransientWindow,
    DestroyTransientWindow,
}

impl NativeTransientWindowHostOperation {
    pub(crate) const fn operation_name(self) -> &'static str {
        match self {
            Self::CreateTransientWindow => "create_transient_window",
            Self::PresentTransientWindow => "present_transient_window",
            Self::HideTransientWindow => "hide_transient_window",
            Self::DestroyTransientWindow => "destroy_transient_window",
        }
    }
}

pub(crate) const REQUIRED_NATIVE_TRANSIENT_WINDOW_HOST_OPERATIONS:
    [NativeTransientWindowHostOperation; 4] = [
    NativeTransientWindowHostOperation::CreateTransientWindow,
    NativeTransientWindowHostOperation::PresentTransientWindow,
    NativeTransientWindowHostOperation::HideTransientWindow,
    NativeTransientWindowHostOperation::DestroyTransientWindow,
];

pub(crate) trait NativeImeHost {
    type Handle: Copy + Eq;

    fn candidate_anchor(
        &mut self,
        focus: Self::Handle,
        index: u32,
    ) -> Option<NativeImeCandidateAnchor>;
    fn composition_anchor(&mut self, focus: Self::Handle) -> Option<NativeImeCompositionAnchor>;
    fn has_default_ime_window(&mut self, focus: Self::Handle) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeImeCandidateAnchor {
    CandidatePoint { position: Point },
    ExcludeRect { rect: UiRect },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeImeCompositionAnchor {
    Point { position: Point },
    Rect { rect: UiRect },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeImeHostOperation {
    CandidateAnchor,
    CompositionAnchor,
    HasDefaultImeWindow,
}

impl NativeImeHostOperation {
    pub(crate) const fn operation_name(self) -> &'static str {
        match self {
            Self::CandidateAnchor => "candidate_anchor",
            Self::CompositionAnchor => "composition_anchor",
            Self::HasDefaultImeWindow => "has_default_ime_window",
        }
    }
}

pub(crate) const REQUIRED_NATIVE_IME_HOST_OPERATIONS: [NativeImeHostOperation; 3] = [
    NativeImeHostOperation::CandidateAnchor,
    NativeImeHostOperation::CompositionAnchor,
    NativeImeHostOperation::HasDefaultImeWindow,
];

pub(crate) trait NativeTextCaretHost {
    type Handle: Copy + Eq;

    fn accessible_caret_anchor(&mut self, focus: Self::Handle) -> Option<NativeTextCaretAnchor>;
    fn thread_caret_anchor(&mut self, target: Self::Handle) -> Option<NativeTextCaretAnchor>;
    fn focus_rect_anchor(
        &mut self,
        focus: Self::Handle,
        max_width: i32,
        max_height: i32,
    ) -> Option<NativeTextCaretAnchor>;
    fn cursor_anchor(&mut self) -> Option<NativeTextCaretAnchor>;
    fn focus_handle_for_target(&mut self, target: Self::Handle) -> Self::Handle;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeTextCaretAnchor {
    pub(crate) left: i32,
    pub(crate) top: i32,
    pub(crate) bottom: i32,
}

impl NativeTextCaretAnchor {
    pub(crate) const fn new(left: i32, top: i32, bottom: i32) -> Self {
        Self { left, top, bottom }
    }

    pub(crate) const fn has_vertical_span(self) -> bool {
        self.bottom > self.top
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeTextCaretHostOperation {
    AccessibleCaretAnchor,
    ThreadCaretAnchor,
    FocusRectAnchor,
    CursorAnchor,
    FocusHandleForTarget,
}

impl NativeTextCaretHostOperation {
    pub(crate) const fn operation_name(self) -> &'static str {
        match self {
            Self::AccessibleCaretAnchor => "accessible_caret_anchor",
            Self::ThreadCaretAnchor => "thread_caret_anchor",
            Self::FocusRectAnchor => "focus_rect_anchor",
            Self::CursorAnchor => "cursor_anchor",
            Self::FocusHandleForTarget => "focus_handle_for_target",
        }
    }
}

pub(crate) const REQUIRED_NATIVE_TEXT_CARET_HOST_OPERATIONS: [NativeTextCaretHostOperation; 5] = [
    NativeTextCaretHostOperation::AccessibleCaretAnchor,
    NativeTextCaretHostOperation::ThreadCaretAnchor,
    NativeTextCaretHostOperation::FocusRectAnchor,
    NativeTextCaretHostOperation::CursorAnchor,
    NativeTextCaretHostOperation::FocusHandleForTarget,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeDialogLevel {
    Info,
    Warning,
    Error,
    Question,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeDialogButtons {
    YesNoCancel,
    YesNo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeDialogResponse {
    Yes,
    No,
    Cancel,
}

pub(crate) trait NativeDialogHost {
    type Owner;

    fn show_message(
        &self,
        owner: Self::Owner,
        title: &str,
        message: &str,
        level: NativeDialogLevel,
    );

    fn confirm(
        &self,
        owner: Self::Owner,
        title: &str,
        message: &str,
        level: NativeDialogLevel,
        buttons: NativeDialogButtons,
    ) -> NativeDialogResponse;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeDialogHostOperation {
    ShowMessage,
    Confirm,
}

impl NativeDialogHostOperation {
    pub(crate) const fn operation_name(self) -> &'static str {
        match self {
            Self::ShowMessage => "show_message",
            Self::Confirm => "confirm",
        }
    }
}

pub(crate) const REQUIRED_NATIVE_DIALOG_HOST_OPERATIONS: [NativeDialogHostOperation; 2] = [
    NativeDialogHostOperation::ShowMessage,
    NativeDialogHostOperation::Confirm,
];

pub(crate) trait NativeShellOpenHost {
    fn open_path(&self, path: &str);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeShellOpenHostOperation {
    OpenPath,
}

impl NativeShellOpenHostOperation {
    pub(crate) const fn operation_name(self) -> &'static str {
        match self {
            Self::OpenPath => "open_path",
        }
    }
}

pub(crate) const REQUIRED_NATIVE_SHELL_OPEN_HOST_OPERATIONS: [NativeShellOpenHostOperation; 1] =
    [NativeShellOpenHostOperation::OpenPath];

pub(crate) trait NativeWindowIdentityHost {
    type Handle: Copy + Eq;

    fn process_name(&self, handle: Self::Handle) -> String;
    fn class_name(&self, handle: Self::Handle) -> String;
    fn root_handle(&self, handle: Self::Handle) -> Self::Handle;
    fn foreground_handle(&self) -> Self::Handle;
    fn exists(&self, handle: Self::Handle) -> bool;
    fn is_foreground(&self, handle: Self::Handle) -> bool;
    fn is_current_process_window(&self, handle: Self::Handle) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeWindowIdentitySnapshot<Handle: Copy + Eq> {
    pub(crate) handle: Handle,
    pub(crate) process_name: String,
    pub(crate) class_name: String,
    pub(crate) root_handle: Handle,
    pub(crate) foreground_handle: Handle,
    pub(crate) exists: bool,
    pub(crate) is_foreground: bool,
    pub(crate) is_current_process_window: bool,
}

impl<Handle: Copy + Eq> NativeWindowIdentitySnapshot<Handle> {
    pub(crate) const fn is_external_existing_target(&self) -> bool {
        self.exists && !self.is_current_process_window
    }

    pub(crate) fn foreground_matches_target(&self) -> bool {
        self.is_foreground || self.root_handle == self.foreground_handle
    }

    pub(crate) fn can_restore_or_paste_to_target(&self) -> bool {
        self.is_external_existing_target() && self.foreground_matches_target()
    }
}

pub(crate) fn native_window_identity_snapshot<H>(
    host: &H,
    handle: H::Handle,
) -> NativeWindowIdentitySnapshot<H::Handle>
where
    H: NativeWindowIdentityHost,
{
    NativeWindowIdentitySnapshot {
        handle,
        process_name: host.process_name(handle),
        class_name: host.class_name(handle),
        root_handle: host.root_handle(handle),
        foreground_handle: host.foreground_handle(),
        exists: host.exists(handle),
        is_foreground: host.is_foreground(handle),
        is_current_process_window: host.is_current_process_window(handle),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeWindowIdentityHostOperation {
    ProcessName,
    ClassName,
    RootHandle,
    ForegroundHandle,
    Exists,
    IsForeground,
    IsCurrentProcessWindow,
}

impl NativeWindowIdentityHostOperation {
    pub(crate) const fn operation_name(self) -> &'static str {
        match self {
            Self::ProcessName => "process_name",
            Self::ClassName => "class_name",
            Self::RootHandle => "root_handle",
            Self::ForegroundHandle => "foreground_handle",
            Self::Exists => "exists",
            Self::IsForeground => "is_foreground",
            Self::IsCurrentProcessWindow => "is_current_process_window",
        }
    }
}

pub(crate) const REQUIRED_NATIVE_WINDOW_IDENTITY_HOST_OPERATIONS:
    [NativeWindowIdentityHostOperation; 7] = [
    NativeWindowIdentityHostOperation::ProcessName,
    NativeWindowIdentityHostOperation::ClassName,
    NativeWindowIdentityHostOperation::RootHandle,
    NativeWindowIdentityHostOperation::ForegroundHandle,
    NativeWindowIdentityHostOperation::Exists,
    NativeWindowIdentityHostOperation::IsForeground,
    NativeWindowIdentityHostOperation::IsCurrentProcessWindow,
];

pub(crate) trait NativePasteTargetHost {
    type Handle: Copy + Eq;

    fn force_paste_target_foreground(&mut self, target: Self::Handle) -> bool;
    fn restore_paste_target_focus(&mut self, target: Self::Handle, focus: Self::Handle);
    fn set_paste_target_text(&mut self, target: Self::Handle, text: &str) -> bool;
    fn paste_target_text_input_capabilities(
        &mut self,
        target: Self::Handle,
    ) -> PasteTargetTextInputCapabilities;
    fn paste_target_focus_status(
        &mut self,
        target: Self::Handle,
        passthrough_focus: Self::Handle,
    ) -> PasteTargetFocusStatus;
    fn paste_target_text_input_ready(&mut self, target: Self::Handle) -> bool;
    fn send_paste_shortcut(&mut self, target: Self::Handle) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativePasteTargetActivationSnapshot<Handle: Copy + Eq> {
    pub(crate) target: Handle,
    pub(crate) passthrough_focus: Handle,
    pub(crate) foregrounded: bool,
    pub(crate) text_input_capabilities: PasteTargetTextInputCapabilities,
    pub(crate) focus_status: PasteTargetFocusStatus,
    pub(crate) text_input_ready: bool,
}

impl<Handle: Copy + Eq> NativePasteTargetActivationSnapshot<Handle> {
    pub(crate) const fn can_directly_set_text(&self) -> bool {
        self.foregrounded
            && self.text_input_ready
            && self.text_input_capabilities.accepts_text_input()
            && self.focus_status.allows_paste_attempt()
    }

    pub(crate) const fn can_send_paste_shortcut(&self) -> bool {
        self.foregrounded && self.focus_status.allows_paste_attempt()
    }
}

pub(crate) fn native_paste_target_activation_snapshot<H>(
    host: &mut H,
    target: H::Handle,
    passthrough_focus: H::Handle,
) -> NativePasteTargetActivationSnapshot<H::Handle>
where
    H: NativePasteTargetHost,
{
    let foregrounded = host.force_paste_target_foreground(target);
    let text_input_capabilities = host.paste_target_text_input_capabilities(target);
    let focus_status = host.paste_target_focus_status(target, passthrough_focus);
    let text_input_ready = host.paste_target_text_input_ready(target);

    NativePasteTargetActivationSnapshot {
        target,
        passthrough_focus,
        foregrounded,
        text_input_capabilities,
        focus_status,
        text_input_ready,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct PasteTargetTextInputCapabilities {
    pub(crate) has_selection: bool,
    pub(crate) wants_chars: bool,
    pub(crate) wants_tab: bool,
    pub(crate) wants_arrows: bool,
}

impl PasteTargetTextInputCapabilities {
    pub(crate) const fn text_input() -> Self {
        Self {
            has_selection: true,
            wants_chars: true,
            wants_tab: false,
            wants_arrows: false,
        }
    }

    pub(crate) const fn accepts_text_input(self) -> bool {
        self.has_selection || self.wants_chars || self.wants_tab || self.wants_arrows
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PasteTargetFocusStatus {
    Unknown,
    NoActiveFocus,
    InsideTarget,
    OutsideTarget,
}

impl Default for PasteTargetFocusStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

impl PasteTargetFocusStatus {
    pub(crate) const fn allows_paste_attempt(self) -> bool {
        !matches!(self, Self::OutsideTarget)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativePasteTargetHostOperation {
    ForcePasteTargetForeground,
    RestorePasteTargetFocus,
    SetPasteTargetText,
    PasteTargetTextInputCapabilities,
    PasteTargetFocusStatus,
    PasteTargetTextInputReady,
    SendPasteShortcut,
}

impl NativePasteTargetHostOperation {
    pub(crate) const fn operation_name(self) -> &'static str {
        match self {
            Self::ForcePasteTargetForeground => "force_paste_target_foreground",
            Self::RestorePasteTargetFocus => "restore_paste_target_focus",
            Self::SetPasteTargetText => "set_paste_target_text",
            Self::PasteTargetTextInputCapabilities => "paste_target_text_input_capabilities",
            Self::PasteTargetFocusStatus => "paste_target_focus_status",
            Self::PasteTargetTextInputReady => "paste_target_text_input_ready",
            Self::SendPasteShortcut => "send_paste_shortcut",
        }
    }
}

pub(crate) const REQUIRED_NATIVE_PASTE_TARGET_HOST_OPERATIONS: [NativePasteTargetHostOperation; 7] = [
    NativePasteTargetHostOperation::ForcePasteTargetForeground,
    NativePasteTargetHostOperation::RestorePasteTargetFocus,
    NativePasteTargetHostOperation::SetPasteTargetText,
    NativePasteTargetHostOperation::PasteTargetTextInputCapabilities,
    NativePasteTargetHostOperation::PasteTargetFocusStatus,
    NativePasteTargetHostOperation::PasteTargetTextInputReady,
    NativePasteTargetHostOperation::SendPasteShortcut,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeFileDialogRequest<'a> {
    pub(crate) title: &'a str,
    pub(crate) filter_name: &'a str,
    pub(crate) filter_pattern: &'a str,
    pub(crate) current_path: &'a str,
}

pub(crate) trait NativeFileDialogHost {
    fn pick_file(&self, request: NativeFileDialogRequest<'_>) -> Result<Option<String>, String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeFileDialogHostOperation {
    PickFile,
}

impl NativeFileDialogHostOperation {
    pub(crate) const fn operation_name(self) -> &'static str {
        match self {
            Self::PickFile => "pick_file",
        }
    }
}

pub(crate) const REQUIRED_NATIVE_FILE_DIALOG_HOST_OPERATIONS: [NativeFileDialogHostOperation; 1] =
    [NativeFileDialogHostOperation::PickFile];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeTextInputDialogRequest<'a> {
    pub(crate) title: &'a str,
    pub(crate) label: &'a str,
    pub(crate) initial: &'a str,
}

pub(crate) trait NativeTextInputDialogHost {
    type Owner;

    fn prompt_text(
        &self,
        owner: Self::Owner,
        request: NativeTextInputDialogRequest<'_>,
    ) -> Option<String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeTextInputDialogHostOperation {
    PromptText,
}

impl NativeTextInputDialogHostOperation {
    pub(crate) const fn operation_name(self) -> &'static str {
        match self {
            Self::PromptText => "prompt_text",
        }
    }
}

pub(crate) const REQUIRED_NATIVE_TEXT_INPUT_DIALOG_HOST_OPERATIONS:
    [NativeTextInputDialogHostOperation; 1] = [NativeTextInputDialogHostOperation::PromptText];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeEditTextDialogRequest<'a> {
    pub(crate) title: &'a str,
    pub(crate) initial_text: &'a str,
    pub(crate) initial_size: Option<Size>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct NativeEditTextDialogResult {
    pub(crate) saved: bool,
    pub(crate) final_size: Option<Size>,
}

pub(crate) trait NativeEditTextSaveHandler {
    fn save_text(&mut self, text: &str) -> Result<(), String>;
}

impl<F> NativeEditTextSaveHandler for F
where
    F: FnMut(&str) -> Result<(), String>,
{
    fn save_text(&mut self, text: &str) -> Result<(), String> {
        self(text)
    }
}

pub(crate) trait NativeEditTextDialogHost {
    type Owner;

    fn open_edit_text(
        &self,
        owner: Self::Owner,
        request: NativeEditTextDialogRequest<'_>,
        save_handler: &mut dyn NativeEditTextSaveHandler,
    ) -> NativeEditTextDialogResult;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeEditTextDialogHostOperation {
    OpenEditText,
}

impl NativeEditTextDialogHostOperation {
    pub(crate) const fn operation_name(self) -> &'static str {
        match self {
            Self::OpenEditText => "open_edit_text",
        }
    }
}

pub(crate) const REQUIRED_NATIVE_EDIT_TEXT_DIALOG_HOST_OPERATIONS:
    [NativeEditTextDialogHostOperation; 1] = [NativeEditTextDialogHostOperation::OpenEditText];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeMailMergeWindowRequest<'a> {
    pub(crate) initial_excel_path: Option<&'a str>,
}

pub(crate) trait NativeMailMergeWindowHost {
    type Owner;

    fn open_mail_merge(&self, owner: Self::Owner, request: NativeMailMergeWindowRequest<'_>);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeMailMergeWindowHostOperation {
    OpenMailMerge,
}

impl NativeMailMergeWindowHostOperation {
    pub(crate) const fn operation_name(self) -> &'static str {
        match self {
            Self::OpenMailMerge => "open_mail_merge",
        }
    }
}

pub(crate) const REQUIRED_NATIVE_MAIL_MERGE_WINDOW_HOST_OPERATIONS:
    [NativeMailMergeWindowHostOperation; 1] = [NativeMailMergeWindowHostOperation::OpenMailMerge];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeAutostartStatus {
    pub(crate) enabled: bool,
    pub(crate) registration_path: Option<String>,
}

impl NativeAutostartStatus {
    pub(crate) fn disabled() -> Self {
        Self {
            enabled: false,
            registration_path: None,
        }
    }

    pub(crate) fn enabled_at(path: impl Into<String>) -> Self {
        Self {
            enabled: true,
            registration_path: Some(path.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeAutostartApplyResult {
    pub(crate) requested_enabled: bool,
    pub(crate) applied: bool,
    pub(crate) status: NativeAutostartStatus,
    pub(crate) error: Option<String>,
}

impl NativeAutostartApplyResult {
    pub(crate) fn applied(requested_enabled: bool, status: NativeAutostartStatus) -> Self {
        Self {
            requested_enabled,
            applied: status.enabled == requested_enabled,
            status,
            error: None,
        }
    }

    pub(crate) fn failed(requested_enabled: bool, error: impl Into<String>) -> Self {
        Self {
            requested_enabled,
            applied: false,
            status: NativeAutostartStatus::disabled(),
            error: Some(error.into()),
        }
    }
}

pub(crate) trait NativeAutostartHost {
    fn autostart_status(&self) -> NativeAutostartStatus;
    fn set_autostart_enabled(&mut self, enabled: bool) -> NativeAutostartApplyResult;
}

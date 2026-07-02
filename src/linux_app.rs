#![allow(dead_code)]
#![cfg_attr(not(target_os = "linux"), allow(unused_imports))]

use std::cell::RefCell;
#[cfg(all(target_os = "linux", not(test)))]
use std::collections::hash_map::DefaultHasher;
#[cfg(all(target_os = "linux", not(test)))]
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
#[cfg(all(target_os = "linux", not(test)))]
use std::process::Command as ProcessCommand;
use std::sync::{Mutex, OnceLock};
use std::{env, fs};

use crate::app_core::{
    main_menu_command_for_id, main_row_external_action_plan, poll_clipboard_monitor,
    settings_action_for_route, settings_action_route, settings_lan_device_book_projection,
    settings_lan_device_projection, settings_lan_mobile_link_projection_from_json,
    settings_lan_pair_request_projection_from_json, settings_lan_pair_request_response_projection,
    settings_lan_pair_status_projection, settings_lan_sync_action_support_plan,
    zsui_native_feature_status_for, ApplicationEvent, ClipItem, ClipboardHost,
    ClipboardMonitorState, Color, ColorRole, Command, CommandQueue, ComponentPhase, LifecycleEvent,
    LifecycleState, MainRowExternalActionPlan, MainRowMenuAction, NativeAiActionMenuRequest,
    NativeAiActionPresenter, NativeAiSettingsSurfaceRequest, NativeAppIconResource,
    NativeAutostartApplyResult, NativeAutostartHost, NativeAutostartStatus, NativeControlMapper,
    NativeDialogButtons, NativeDialogHost, NativeDialogLevel, NativeDialogResponse,
    NativeEditTextDialogHost, NativeEditTextDialogRequest, NativeEditTextDialogResult,
    NativeEditTextSaveHandler, NativeFileDialogHost, NativeFileDialogRequest,
    NativeHostClipListItemProjection, NativeHostClipboardWrite, NativeHostDialogAction,
    NativeHostLaunchMode, NativeHostLaunchPlan, NativeHostRowAction, NativeHostSearchTextAction,
    NativeHostSettingsAction, NativeHostSettingsControlAction, NativeHostSettingsPlatformAction,
    NativeHostStatusMenuAction, NativeHostUiAction, NativeHostVvPasteExecution,
    NativeHostVvPastePlan, NativeHostVvTriggerAction, NativeHostVvTriggerInput,
    NativeHostVvTriggerState, NativeHostVvTriggerTransition, NativeImeCandidateAnchor,
    NativeImeCompositionAnchor, NativeImeHost, NativeMailMergeWindowHost,
    NativeMailMergeWindowRequest, NativeMainSearchControlHost, NativeMainSearchControlPresentation,
    NativeMainSearchControlRequest, NativeMainSearchStylePresentation,
    NativeMainSearchStyleRequest, NativeMainWindowHandles, NativeMainWindowHost,
    NativeMainWindowPresentMode, NativeMainWindowPresentation, NativeMainWindowRequest,
    NativePasteTargetHost, NativePopupMenuEntry, NativePopupMenuHost, NativePopupMenuPlacement,
    NativeRuntimeDriver, NativeRuntimeStartupRequest, NativeRuntimeStartupResult,
    NativeSettingsControlHost, NativeSettingsDropdownHost, NativeSettingsDropdownPresentation,
    NativeSettingsDropdownRequest, NativeSettingsWindowHost, NativeSettingsWindowPresentation,
    NativeSettingsWindowRequest, NativeShellOpenHost, NativeStyleResolver, NativeTextCaretAnchor,
    NativeTextCaretHost, NativeTextInputDialogHost, NativeTextInputDialogRequest,
    NativeTransientWindowHost, NativeTransientWindowPresentation, NativeTransientWindowRequest,
    NativeUiPlatform, NativeUiToolkit, NativeWindowIdentityHost, NativeWindowOptions,
    NativeWindowToken, PasteTargetFocusStatus, PasteTargetTextInputCapabilities, Point,
    ProductAdapterAsyncBridgeResult, ProductAdapterCommandResult, ProductAdapterHost,
    ProductAiExecutionPlan, ProductAiInvocation, ProductAiUiSurface, Rect, Renderer,
    SemanticTextStyle, SettingsAction, SettingsActionRoute, SettingsComponentKind,
    SettingsControlSpec, SettingsLanAcceptedDeviceProjection, Size, StatusItemHost,
    StatusMenuEntry, TextLayout, TextRole, TextRun, TextStyle, UiRect, APP_CORE_API_VERSION,
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
    ZSUI_FRAMEWORK_NAME,
};
use crate::zsclip_product_adapter::ZsclipProductAdapter;

mod contract;
pub(crate) use contract::{
    linux_host_contract_summary, LinuxHostContractSummary, LinuxNativeBackend,
};
mod startup;
pub(crate) use startup::{linux_native_host_launch_plan, LinuxStartupPlan};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LinuxMainWindowHandle(u64);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LinuxAppIconHandle(u64);

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct LinuxStartupSessionState {
    backend: Option<LinuxNativeBackend>,
    created_main_windows: Option<NativeMainWindowHandles<LinuxMainWindowHandle>>,
    generation: u64,
}

#[derive(Debug)]
pub(crate) struct LinuxApplicationModel {
    lifecycle: LifecycleState,
    commands: CommandQueue,
    product_adapter: ZsclipProductAdapter,
    product_command_results: Vec<ProductAdapterCommandResult>,
    product_event_results: Vec<ProductAdapterAsyncBridgeResult>,
    ai_action_presentation: LinuxAiActionPresentationSessionState,
    runtime_events: Vec<ApplicationEvent>,
    runtime_shutdown_requested: bool,
    clipboard_capture_enabled: bool,
    clipboard_monitor: ClipboardMonitorState,
    startup_session: LinuxStartupSessionState,
    autostart_host: LinuxAutostartHost,
    style_resolver: LinuxNativeStyleResolver,
    control_mapper: LinuxNativeControlMapper,
    text_layout: LinuxTextLayout,
    status_item_host: LinuxStatusItemHost,
    popup_menu_host: LinuxPopupMenuHost,
    transient_window_host: LinuxTransientWindowHost,
    ime_host: LinuxImeHost,
    dialog_host: LinuxDialogHost,
    shell_open_host: LinuxShellOpenHost,
    window_identity_host: LinuxWindowIdentityHost,
    paste_target_host: LinuxPasteTargetHost,
    text_caret_host: LinuxTextCaretHost,
    file_dialog_host: LinuxFileDialogHost,
    text_input_dialog_host: LinuxTextInputDialogHost,
    edit_text_dialog_host: LinuxEditTextDialogHost,
    mail_merge_window_host: LinuxMailMergeWindowHost,
    renderer: LinuxRenderer,
    main_window_host: LinuxMainWindowHost,
    main_search_host: LinuxMainSearchControlHost,
    settings_window_host: LinuxSettingsWindowHost,
    settings_control_host: LinuxSettingsControlHost,
    settings_dropdown_host: LinuxSettingsDropdownHost,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct LinuxMainWindowHost {
    next_handle: u64,
    create_requests: Vec<NativeMainWindowRequest>,
    created: Option<NativeMainWindowHandles<LinuxMainWindowHandle>>,
    appearance_applied: Vec<LinuxMainWindowHandle>,
    icon_updates: Vec<(
        LinuxMainWindowHandle,
        NativeAppIconResource<LinuxAppIconHandle>,
    )>,
    hidden: Vec<LinuxMainWindowHandle>,
    presented: Vec<(LinuxMainWindowHandle, NativeMainWindowPresentMode)>,
    bounds_updates: Vec<(LinuxMainWindowHandle, UiRect)>,
    activation_policy: Vec<(LinuxMainWindowHandle, bool)>,
    close_requests: Vec<LinuxMainWindowHandle>,
    destroyed: Vec<LinuxMainWindowHandle>,
    pointer_captures: u64,
    pointer_releases: u64,
    drag_count: u64,
    repaint_requests: Vec<(LinuxMainWindowHandle, Option<UiRect>, bool)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinuxAutostartHost {
    autostart_dir: PathBuf,
    executable_path: Option<PathBuf>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LinuxMainSearchHandle(u64);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LinuxSearchStyleResource(u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinuxMainSearchRecord {
    handle: LinuxMainSearchHandle,
    request: NativeMainSearchControlRequest<LinuxMainWindowHandle>,
    text: String,
    visible: bool,
    bounds: UiRect,
    focused: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct LinuxMainSearchControlHost {
    next_handle: u64,
    next_style_resource: u64,
    searches: Vec<LinuxMainSearchRecord>,
    style_requests:
        Vec<NativeMainSearchStyleRequest<LinuxMainSearchHandle, LinuxSearchStyleResource>>,
    released_style_resources: Vec<LinuxSearchStyleResource>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct LinuxStatusItemHost {
    installed_tooltip: Option<String>,
    menu_entries: Vec<StatusMenuEntry>,
    install_count: u64,
    remove_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinuxPopupMenuPresentation {
    owner: LinuxMainWindowHandle,
    x: i32,
    y: i32,
    placement: NativePopupMenuPlacement,
    entries: Vec<NativePopupMenuEntry>,
    selected_id: usize,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct LinuxPopupMenuHost {
    presentations: Vec<LinuxPopupMenuPresentation>,
}

#[derive(Debug, Default, PartialEq)]
pub(crate) struct LinuxNativeStyleResolver {
    resolved: RefCell<Vec<SemanticTextStyle>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LinuxNativeControlClass {
    Label,
    Entry,
    Switch,
    ComboRow,
    Button,
    SuggestedActionButton,
}

#[derive(Debug, Default, PartialEq)]
pub(crate) struct LinuxNativeControlMapper {
    mapped: RefCell<Vec<(SettingsComponentKind, LinuxNativeControlClass)>>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LinuxTextLayoutAction {
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
pub(crate) struct LinuxTextLayout {
    actions: RefCell<Vec<LinuxTextLayoutAction>>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LinuxTransientWindowHandle(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LinuxTransientWindowCreateRequest {
    owner: LinuxMainWindowHandle,
    bounds: UiRect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LinuxTransientWindowAction {
    Present {
        handle: LinuxTransientWindowHandle,
        bounds: UiRect,
    },
    Hide(LinuxTransientWindowHandle),
    Destroy(LinuxTransientWindowHandle),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct LinuxTransientWindowHost {
    next_handle: u64,
    requests: Vec<LinuxTransientWindowCreateRequest>,
    actions: Vec<LinuxTransientWindowAction>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LinuxImeHandle(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LinuxImeAction {
    QueryCandidate { focus: LinuxImeHandle, index: u32 },
    QueryComposition(LinuxImeHandle),
    HasDefaultImeWindow(LinuxImeHandle),
}

#[derive(Debug, Default, PartialEq)]
pub(crate) struct LinuxImeHost {
    actions: RefCell<Vec<LinuxImeAction>>,
    next_candidate: RefCell<Option<NativeImeCandidateAnchor>>,
    next_composition: RefCell<Option<NativeImeCompositionAnchor>>,
    next_has_default_ime_window: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LinuxClipboardHost;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct LinuxClipboardState {
    text: Option<String>,
    image: Option<(Vec<u8>, usize, usize)>,
    file_paths: Option<Vec<String>>,
    sequence: u32,
    last_system_fingerprint: Option<u64>,
    ignore_next_capture: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinuxDialogMessage {
    owner: LinuxMainWindowHandle,
    title: String,
    message: String,
    level: NativeDialogLevel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinuxDialogConfirmation {
    owner: LinuxMainWindowHandle,
    title: String,
    message: String,
    level: NativeDialogLevel,
    buttons: NativeDialogButtons,
}

#[derive(Debug, PartialEq)]
pub(crate) struct LinuxDialogHost {
    messages: RefCell<Vec<LinuxDialogMessage>>,
    confirmations: RefCell<Vec<LinuxDialogConfirmation>>,
    next_response: RefCell<NativeDialogResponse>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct LinuxShellOpenHost {
    opened_paths: RefCell<Vec<String>>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LinuxWindowIdentityHandle(u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LinuxWindowIdentityAction {
    ProcessName(LinuxWindowIdentityHandle),
    ClassName(LinuxWindowIdentityHandle),
    RootHandle(LinuxWindowIdentityHandle),
    ForegroundHandle,
    Exists(LinuxWindowIdentityHandle),
    IsForeground(LinuxWindowIdentityHandle),
    IsCurrentProcessWindow(LinuxWindowIdentityHandle),
}

#[derive(Debug, Default, PartialEq)]
pub(crate) struct LinuxWindowIdentityHost {
    actions: RefCell<Vec<LinuxWindowIdentityAction>>,
    process_name: RefCell<String>,
    class_name: RefCell<String>,
    root_handle: RefCell<Option<LinuxWindowIdentityHandle>>,
    foreground_handle: RefCell<Option<LinuxWindowIdentityHandle>>,
    existing_windows: RefCell<Vec<LinuxWindowIdentityHandle>>,
    current_process_windows: RefCell<Vec<LinuxWindowIdentityHandle>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinuxIdentitySmokeSummary {
    pub(crate) foreground_seen: bool,
    pub(crate) process_name_seen: bool,
    pub(crate) class_name_seen: bool,
    pub(crate) foreground_exists: bool,
    pub(crate) foreground_matches: bool,
    pub(crate) current_process_window: bool,
    pub(crate) foreground_requested: bool,
    pub(crate) focus_status: PasteTargetFocusStatus,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LinuxPasteTargetHandle(u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LinuxPasteTargetAction {
    ForceForeground(LinuxPasteTargetHandle),
    RestoreFocus {
        target: LinuxPasteTargetHandle,
        focus: LinuxPasteTargetHandle,
    },
    SetText {
        target: LinuxPasteTargetHandle,
        text: String,
    },
    QueryTextInputCapabilities(LinuxPasteTargetHandle),
    QueryFocusStatus {
        target: LinuxPasteTargetHandle,
        passthrough_focus: LinuxPasteTargetHandle,
    },
    QueryTextInputReady(LinuxPasteTargetHandle),
    SendPasteShortcut(LinuxPasteTargetHandle),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct LinuxPasteTargetHost {
    actions: Vec<LinuxPasteTargetAction>,
    next_foreground_result: bool,
    next_text_input_capabilities: PasteTargetTextInputCapabilities,
    next_focus_status: PasteTargetFocusStatus,
    next_text_input_ready: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LinuxTextCaretHandle(u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LinuxTextCaretAction {
    QueryAccessible(LinuxTextCaretHandle),
    QueryThread(LinuxTextCaretHandle),
    QueryFocusRect {
        focus: LinuxTextCaretHandle,
        max_width: i32,
        max_height: i32,
    },
    QueryCursor,
    ResolveFocus(LinuxTextCaretHandle),
}

#[derive(Debug, Default, PartialEq)]
pub(crate) struct LinuxTextCaretHost {
    actions: RefCell<Vec<LinuxTextCaretAction>>,
    next_accessible: RefCell<Option<NativeTextCaretAnchor>>,
    next_thread: RefCell<Option<NativeTextCaretAnchor>>,
    next_focus_rect: RefCell<Option<NativeTextCaretAnchor>>,
    next_cursor: RefCell<Option<NativeTextCaretAnchor>>,
    next_focus_handle: RefCell<Option<LinuxTextCaretHandle>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinuxFileDialogRecord {
    title: String,
    filter_name: String,
    filter_pattern: String,
    current_path: String,
}

#[derive(Debug, PartialEq)]
pub(crate) struct LinuxFileDialogHost {
    requests: RefCell<Vec<LinuxFileDialogRecord>>,
    next_result: RefCell<Result<Option<String>, String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinuxTextInputDialogRecord {
    owner: LinuxMainWindowHandle,
    title: String,
    label: String,
    initial: String,
}

#[derive(Debug, Default, PartialEq)]
pub(crate) struct LinuxTextInputDialogHost {
    requests: RefCell<Vec<LinuxTextInputDialogRecord>>,
    next_result: RefCell<Option<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinuxEditTextDialogRecord {
    owner: LinuxMainWindowHandle,
    title: String,
    initial_text: String,
    initial_size: Option<Size>,
}

#[derive(Debug, Default, PartialEq)]
pub(crate) struct LinuxEditTextDialogHost {
    requests: RefCell<Vec<LinuxEditTextDialogRecord>>,
    next_saved_text: RefCell<Option<String>>,
    next_final_size: RefCell<Option<Size>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinuxMailMergeWindowRecord {
    owner: LinuxMainWindowHandle,
    initial_excel_path: Option<String>,
}

#[derive(Debug, Default, PartialEq)]
pub(crate) struct LinuxMailMergeWindowHost {
    requests: RefCell<Vec<LinuxMailMergeWindowRecord>>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LinuxRenderCommand {
    FillRect(Rect, Color),
    StrokeRect(Rect, Color, i32),
    DrawText(TextRun, TextStyle),
    PushClip(Rect),
    PopClip,
}

#[derive(Debug, Default, PartialEq)]
pub(crate) struct LinuxRenderer {
    commands: Vec<LinuxRenderCommand>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct LinuxSettingsWindowHost {
    next_handle: u64,
    present_requests: Vec<NativeSettingsWindowRequest<LinuxMainWindowHandle>>,
    presented: Option<LinuxMainWindowHandle>,
    bounds_updates: Vec<(LinuxMainWindowHandle, UiRect)>,
    destroyed: Vec<LinuxMainWindowHandle>,
    focused: Vec<LinuxMainWindowHandle>,
    pointer_leave_tracks: u64,
    pointer_captures: u64,
    pointer_releases: u64,
    repaint_requests: Vec<(LinuxMainWindowHandle, Option<UiRect>, bool)>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LinuxSettingsControlHandle(u64);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LinuxSettingsDropdownHandle(u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinuxSettingsControlRecord {
    handle: LinuxSettingsControlHandle,
    spec: SettingsControlSpec,
    visible: bool,
    enabled: bool,
    destroyed: bool,
    repaint_count: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct LinuxSettingsControlHost {
    next_handle: u64,
    controls: Vec<LinuxSettingsControlRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinuxSettingsDropdownRecord {
    handle: LinuxSettingsDropdownHandle,
    request: NativeSettingsDropdownRequest<LinuxMainWindowHandle>,
    destroyed: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct LinuxSettingsDropdownHost {
    next_handle: u64,
    dropdowns: Vec<LinuxSettingsDropdownRecord>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct LinuxAiActionPresentationSessionState {
    menu_requests: Vec<NativeAiActionMenuRequest<LinuxMainWindowHandle>>,
    settings_requests: Vec<NativeAiSettingsSurfaceRequest<LinuxMainWindowHandle>>,
    executed_action_names: Vec<&'static str>,
    last_surface: Option<ProductAiUiSurface>,
    generation: u64,
}

impl LinuxAiActionPresentationSessionState {
    pub(crate) fn record_menu_request(
        &mut self,
        request: NativeAiActionMenuRequest<LinuxMainWindowHandle>,
    ) {
        self.last_surface = Some(request.surface);
        self.menu_requests.push(request);
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn record_settings_request(
        &mut self,
        request: NativeAiSettingsSurfaceRequest<LinuxMainWindowHandle>,
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

impl Default for LinuxApplicationModel {
    fn default() -> Self {
        Self {
            lifecycle: LifecycleState::new(),
            commands: CommandQueue::default(),
            product_adapter: ZsclipProductAdapter::default(),
            product_command_results: Vec::new(),
            product_event_results: Vec::new(),
            ai_action_presentation: LinuxAiActionPresentationSessionState::default(),
            runtime_events: Vec::new(),
            runtime_shutdown_requested: false,
            clipboard_capture_enabled: true,
            clipboard_monitor: ClipboardMonitorState::default(),
            startup_session: LinuxStartupSessionState::default(),
            autostart_host: LinuxAutostartHost::default(),
            style_resolver: LinuxNativeStyleResolver::default(),
            control_mapper: LinuxNativeControlMapper::default(),
            text_layout: LinuxTextLayout::default(),
            status_item_host: LinuxStatusItemHost::default(),
            popup_menu_host: LinuxPopupMenuHost::default(),
            transient_window_host: LinuxTransientWindowHost::default(),
            ime_host: LinuxImeHost::default(),
            dialog_host: LinuxDialogHost::default(),
            shell_open_host: LinuxShellOpenHost::default(),
            window_identity_host: LinuxWindowIdentityHost::default(),
            paste_target_host: LinuxPasteTargetHost::default(),
            text_caret_host: LinuxTextCaretHost::default(),
            file_dialog_host: LinuxFileDialogHost::default(),
            text_input_dialog_host: LinuxTextInputDialogHost::default(),
            edit_text_dialog_host: LinuxEditTextDialogHost::default(),
            mail_merge_window_host: LinuxMailMergeWindowHost::default(),
            renderer: LinuxRenderer::default(),
            main_window_host: LinuxMainWindowHost::default(),
            main_search_host: LinuxMainSearchControlHost::default(),
            settings_window_host: LinuxSettingsWindowHost::default(),
            settings_control_host: LinuxSettingsControlHost::default(),
            settings_dropdown_host: LinuxSettingsDropdownHost::default(),
        }
    }
}

impl Default for LinuxAutostartHost {
    fn default() -> Self {
        Self {
            autostart_dir: linux_autostart_dir(),
            executable_path: env::current_exe().ok(),
        }
    }
}

impl LinuxAutostartHost {
    pub(crate) fn with_paths(autostart_dir: PathBuf, executable_path: PathBuf) -> Self {
        Self {
            autostart_dir,
            executable_path: Some(executable_path),
        }
    }

    fn desktop_entry_path(&self) -> PathBuf {
        self.autostart_dir.join("zsclip.desktop")
    }

    fn executable_path(&self) -> Result<&Path, String> {
        self.executable_path
            .as_deref()
            .ok_or_else(|| "current executable path is unavailable".to_string())
    }
}

impl NativeAutostartHost for LinuxAutostartHost {
    fn autostart_status(&self) -> NativeAutostartStatus {
        let desktop_entry_path = self.desktop_entry_path();
        let Ok(entry) = fs::read_to_string(&desktop_entry_path) else {
            return NativeAutostartStatus::disabled();
        };
        let Some(executable_path) = self.executable_path.as_ref() else {
            return NativeAutostartStatus::disabled();
        };
        if entry.contains("X-ZSClip-Autostart=true")
            && entry.contains(&format!(
                "Exec={}",
                linux_desktop_exec_escape(&executable_path.to_string_lossy())
            ))
        {
            NativeAutostartStatus::enabled_at(desktop_entry_path.to_string_lossy())
        } else {
            NativeAutostartStatus::disabled()
        }
    }

    fn set_autostart_enabled(&mut self, enabled: bool) -> NativeAutostartApplyResult {
        let desktop_entry_path = self.desktop_entry_path();
        if enabled {
            let executable_path = match self.executable_path() {
                Ok(path) => path,
                Err(err) => return NativeAutostartApplyResult::failed(true, err),
            };
            if let Err(err) = fs::create_dir_all(&self.autostart_dir) {
                return NativeAutostartApplyResult::failed(true, err.to_string());
            }
            let entry = linux_autostart_desktop_entry(executable_path);
            if let Err(err) = fs::write(&desktop_entry_path, entry) {
                return NativeAutostartApplyResult::failed(true, err.to_string());
            }
            return NativeAutostartApplyResult::applied(true, self.autostart_status());
        }

        match fs::remove_file(&desktop_entry_path) {
            Ok(()) => NativeAutostartApplyResult::applied(false, self.autostart_status()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                NativeAutostartApplyResult::applied(false, self.autostart_status())
            }
            Err(err) => NativeAutostartApplyResult::failed(false, err.to_string()),
        }
    }
}

fn linux_autostart_dir() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("autostart")
}

fn linux_autostart_desktop_entry(executable_path: &Path) -> String {
    let executable = linux_desktop_exec_escape(&executable_path.to_string_lossy());
    format!(
        "[Desktop Entry]\nType=Application\nName=ZSClip\nExec={executable}\nTerminal=false\nX-GNOME-Autostart-enabled=true\nX-ZSClip-Autostart=true\n"
    )
}

fn linux_desktop_exec_escape(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-' | ':'))
    {
        return value.to_string();
    }
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

impl Default for LinuxMainWindowHost {
    fn default() -> Self {
        Self {
            next_handle: 1,
            create_requests: Vec::new(),
            created: None,
            appearance_applied: Vec::new(),
            icon_updates: Vec::new(),
            hidden: Vec::new(),
            presented: Vec::new(),
            bounds_updates: Vec::new(),
            activation_policy: Vec::new(),
            close_requests: Vec::new(),
            destroyed: Vec::new(),
            pointer_captures: 0,
            pointer_releases: 0,
            drag_count: 0,
            repaint_requests: Vec::new(),
        }
    }
}

impl Default for LinuxMainSearchControlHost {
    fn default() -> Self {
        Self {
            next_handle: 3_000,
            next_style_resource: 1,
            searches: Vec::new(),
            style_requests: Vec::new(),
            released_style_resources: Vec::new(),
        }
    }
}

impl Default for LinuxSettingsWindowHost {
    fn default() -> Self {
        Self {
            next_handle: 100,
            present_requests: Vec::new(),
            presented: None,
            bounds_updates: Vec::new(),
            destroyed: Vec::new(),
            focused: Vec::new(),
            pointer_leave_tracks: 0,
            pointer_captures: 0,
            pointer_releases: 0,
            repaint_requests: Vec::new(),
        }
    }
}

impl Default for LinuxTransientWindowHost {
    fn default() -> Self {
        Self {
            next_handle: 4_000,
            requests: Vec::new(),
            actions: Vec::new(),
        }
    }
}

impl Default for LinuxDialogHost {
    fn default() -> Self {
        Self {
            messages: RefCell::new(Vec::new()),
            confirmations: RefCell::new(Vec::new()),
            next_response: RefCell::new(NativeDialogResponse::Cancel),
        }
    }
}

impl Default for LinuxPasteTargetHost {
    fn default() -> Self {
        Self {
            actions: Vec::new(),
            next_foreground_result: false,
            next_text_input_capabilities: PasteTargetTextInputCapabilities::default(),
            next_focus_status: PasteTargetFocusStatus::Unknown,
            next_text_input_ready: false,
        }
    }
}

impl Default for LinuxFileDialogHost {
    fn default() -> Self {
        Self {
            requests: RefCell::new(Vec::new()),
            next_result: RefCell::new(Ok(None)),
        }
    }
}

impl Default for LinuxSettingsControlHost {
    fn default() -> Self {
        Self {
            next_handle: 1_000,
            controls: Vec::new(),
        }
    }
}

impl Default for LinuxSettingsDropdownHost {
    fn default() -> Self {
        Self {
            next_handle: 2_000,
            dropdowns: Vec::new(),
        }
    }
}

pub(crate) fn run() -> Result<(), String> {
    let summary = linux_host_contract_summary();
    let launch_plan = linux_native_host_launch_plan();
    if launch_plan.enters_real_event_loop() {
        return crate::linux_native_host::run_real_gtk_host(summary);
    }
    run_linux_contract_scaffold()
}

pub(crate) fn dispatch_linux_native_host_action(
    action: NativeHostUiAction,
) -> ProductAdapterCommandResult {
    let mut application = LinuxApplicationModel::default();
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

pub(crate) fn dispatch_linux_native_settings_action(
    action: NativeHostSettingsAction,
) -> ProductAdapterCommandResult {
    let mut application = LinuxApplicationModel::default();
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

fn linux_native_settings_file() -> std::path::PathBuf {
    #[cfg(test)]
    if let Some(path) = linux_native_settings_file_override()
        .lock()
        .expect("Linux settings file override lock poisoned")
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
fn linux_native_settings_file_override() -> &'static Mutex<Option<std::path::PathBuf>> {
    static OVERRIDE: OnceLock<Mutex<Option<std::path::PathBuf>>> = OnceLock::new();
    OVERRIDE.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
fn set_linux_native_settings_file_for_tests(path: Option<std::path::PathBuf>) {
    *linux_native_settings_file_override()
        .lock()
        .expect("Linux settings file override lock poisoned") = path;
}

fn linux_native_data_dir() -> std::path::PathBuf {
    linux_native_settings_file()
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| std::path::PathBuf::from("data"))
}

fn read_linux_native_settings_json(path: &std::path::Path) -> serde_json::Value {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

pub(crate) fn linux_native_settings_json_snapshot() -> serde_json::Value {
    read_linux_native_settings_json(&linux_native_settings_file())
}

pub(crate) fn linux_native_clipboard_capture_enabled() -> bool {
    linux_native_settings_json_snapshot()
        .get("clipboard_capture_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true)
}

pub(crate) fn linux_native_grouping_enabled() -> bool {
    linux_native_settings_json_snapshot()
        .get("grouping_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true)
}

pub(crate) fn linux_native_group_type_filter_enabled() -> bool {
    linux_native_settings_json_snapshot()
        .get("group_type_filter_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

pub(crate) fn linux_native_status_menu_action_state(
    action: NativeHostStatusMenuAction,
) -> Option<bool> {
    match action {
        NativeHostStatusMenuAction::ToggleClipboardCapture => {
            Some(linux_native_clipboard_capture_enabled())
        }
        #[cfg(feature = "lan-sync")]
        NativeHostStatusMenuAction::ToggleLanSync => linux_native_settings_json_snapshot()
            .get("lan_sync_enabled")
            .and_then(serde_json::Value::as_bool)
            .or_else(|| {
                linux_native_settings_json_snapshot()
                    .get("lan_enable")
                    .and_then(serde_json::Value::as_bool)
            }),
        _ => None,
    }
}

fn persist_linux_native_bool_toggle(
    control_key: &str,
    field_name: &str,
    default_current: bool,
) -> ProductAdapterCommandResult {
    let current = linux_native_settings_json_snapshot()
        .get(field_name)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default_current);
    let submission = crate::settings_model::settings_native_collect_submission(&[
        crate::settings_model::SettingsNativeSubmittedControlValue {
            control_key: control_key.to_string(),
            raw_value: (!current).to_string(),
        },
    ]);
    persist_linux_native_settings_submission(&submission)
}

pub(crate) fn persist_linux_native_settings_submission(
    submission: &crate::settings_model::SettingsNativeCollectSubmission,
) -> ProductAdapterCommandResult {
    let path = linux_native_settings_file();
    let existing_json = read_linux_native_settings_json(&path);
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

    let autostart_result = linux_apply_autostart_from_settings_updates(&applied.field_updates);
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

fn linux_apply_autostart_from_settings_updates(
    updates: &[crate::settings_model::SettingsNativeJsonFieldUpdate],
) -> Option<NativeAutostartApplyResult> {
    let enabled = crate::settings_model::settings_native_bool_field_update(updates, "auto_start")?;
    if cfg!(target_os = "linux") {
        let mut application = LinuxApplicationModel::default();
        return Some(application.apply_autostart(enabled));
    }
    Some(NativeAutostartApplyResult::applied(
        enabled,
        if enabled {
            NativeAutostartStatus::enabled_at("linux_autostart_scaffold")
        } else {
            NativeAutostartStatus::disabled()
        },
    ))
}

pub(crate) fn dispatch_linux_native_settings_control_action(
    action: NativeHostSettingsControlAction,
) -> ProductAdapterCommandResult {
    let mut application = LinuxApplicationModel::default();
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
        if cfg!(target_os = "linux") {
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

pub(crate) fn dispatch_linux_native_settings_platform_action(
    action: NativeHostSettingsPlatformAction,
) -> ProductAdapterCommandResult {
    let result_name = match action {
        NativeHostSettingsPlatformAction::OpenSourceRepository => {
            if open_linux_url_or_file(linux_source_url()).is_ok() {
                "zsclip.settings.open_source_repository"
            } else {
                "zsclip.settings.open_source_repository_failed"
            }
        }
        NativeHostSettingsPlatformAction::CheckForUpdates => {
            if open_linux_url_or_file(linux_latest_release_url()).is_ok() {
                "zsclip.settings.check_for_updates"
            } else {
                "zsclip.settings.check_for_updates_failed"
            }
        }
        NativeHostSettingsPlatformAction::OpenWpsTaskpaneDocs => {
            if open_linux_url_or_file(linux_wps_taskpane_docs_path()).is_ok() {
                "zsclip.settings.open_wps_taskpane_docs"
            } else {
                "zsclip.settings.open_wps_taskpane_docs_failed"
            }
        }
        NativeHostSettingsPlatformAction::DisableSystemClipboardHistory => {
            "zsclip.settings_platform.disable_system_clipboard_history.not_applicable_on_linux_native_host"
        }
        NativeHostSettingsPlatformAction::EnableSystemClipboardHistory => {
            "zsclip.settings_platform.enable_system_clipboard_history.not_applicable_on_linux_native_host"
        }
        NativeHostSettingsPlatformAction::RestartSystemShell => {
            "zsclip.settings_platform.restart_system_shell.not_required_on_linux_native_host"
        }
    };
    ProductAdapterCommandResult {
        accepted: true,
        result_name: result_name.to_string(),
    }
}

fn linux_native_settings_platform_action_for_shared_action(
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
        SettingsAction::DisableSystemClipboardHistory => {
            Some(NativeHostSettingsPlatformAction::DisableSystemClipboardHistory)
        }
        SettingsAction::EnableSystemClipboardHistory => {
            Some(NativeHostSettingsPlatformAction::EnableSystemClipboardHistory)
        }
        SettingsAction::RestartSystemShell => {
            Some(NativeHostSettingsPlatformAction::RestartSystemShell)
        }
        _ => None,
    }
}

fn linux_native_cloud_sync_action_for_shared_action(
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

fn linux_native_cloud_sync_config_from_json(
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

fn linux_native_cloud_sync_paths() -> crate::cloud_sync::CloudSyncPaths {
    let data_dir = linux_native_data_dir();
    crate::cloud_sync::CloudSyncPaths {
        settings_file: data_dir.join("settings.json"),
        db_file: data_dir.join("clipboard.db"),
        data_dir,
    }
}

fn linux_native_lan_runtime_context() -> crate::lan_sync_core::LanRuntimePlatformContext {
    crate::lan_sync_core::LanRuntimePlatformContext::new(
        linux_native_data_dir(),
        crate::lan_sync_core::LanRuntimeEventSink::None,
        linux_native_encrypt_secret_for_storage,
        linux_native_decrypt_secret_from_storage,
    )
}

fn linux_native_encrypt_secret_for_storage(secret: &str) -> Option<String> {
    Some(secret.to_string())
}

fn linux_native_decrypt_secret_from_storage(encoded: &str) -> Option<String> {
    Some(encoded.to_string())
}

fn linux_native_latest_lan_clip_envelope(
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
        linux_native_now_ms(),
    )
}

fn linux_native_latest_lan_file_paths() -> Vec<std::path::PathBuf> {
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
    if item.kind != crate::app_core::ClipKind::Files {
        return Vec::new();
    }
    item.file_paths
        .unwrap_or_default()
        .into_iter()
        .map(std::path::PathBuf::from)
        .collect()
}

pub(crate) fn dispatch_linux_native_lan_background_clip_sync_once(
    runtime_settings: &crate::lan_sync_core::LanRuntimeSettings,
    trusted_devices: &[crate::lan_sync_core::LanDevice],
) -> ProductAdapterCommandResult {
    if !runtime_settings.lan_sync_enabled || runtime_settings.device_id.trim().is_empty() {
        return ProductAdapterCommandResult {
            accepted: false,
            result_name: "zsclip.lan.background_clip_sync.disabled_on_linux_native_host"
                .to_string(),
        };
    }
    let config = crate::lan_sync_core::LanRuntimeConfig::from_core_config(
        linux_native_lan_runtime_context(),
        runtime_settings.core_config(),
    );
    let execution = crate::lan_sync_core::execute_lan_background_clip_sync_once(
        &config,
        trusted_devices,
        linux_native_latest_lan_clip_envelope(runtime_settings),
        std::time::Duration::from_millis(250),
    );
    let latest_file_paths = linux_native_latest_lan_file_paths();
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
            "zsclip.lan.background_clip_sync.pushed_{}_pulled_{}_files_{}_failed_{}_on_linux_native_host",
            execution.pushed_count,
            execution.pulled_count,
            file_pushed_count,
            execution.failed_count + file_failed_count
        ),
    }
}

pub(crate) fn dispatch_linux_native_settings_webdav_action(
    action: SettingsAction,
) -> ProductAdapterCommandResult {
    let Some(cloud_action) = linux_native_cloud_sync_action_for_shared_action(action) else {
        return ProductAdapterCommandResult {
            accepted: false,
            result_name: "zsclip.settings_sync.not_webdav_action".to_string(),
        };
    };
    let config = linux_native_cloud_sync_config_from_json(&linux_native_settings_json_snapshot());
    let paths = linux_native_cloud_sync_paths();
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

pub(crate) fn dispatch_linux_native_settings_lan_mobile_link_action(
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
        "linux_native_host",
        &linux_native_settings_json_snapshot(),
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
            LinuxClipboardHost::write_text_ignored_by_monitors(&target_url)
        }
        SettingsAction::OpenLanSetupPage => {
            LinuxShellOpenHost::default().open_path(&target_url);
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

pub(crate) fn dispatch_linux_native_settings_lan_device_book_action(
    action: SettingsAction,
) -> Option<ProductAdapterCommandResult> {
    if action != SettingsAction::RefreshLanDevices {
        return None;
    }
    let settings_json = linux_native_settings_json_snapshot();
    let runtime_settings =
        crate::lan_sync_core::lan_runtime_settings_from_settings_json(&settings_json);
    if runtime_settings.lan_sync_enabled && !runtime_settings.device_id.trim().is_empty() {
        let config = crate::lan_sync_core::LanRuntimeConfig::from_core_config(
            linux_native_lan_runtime_context(),
            runtime_settings.core_config(),
        );
        let _ = crate::lan_sync_core::probe_lan_discovery_once(
            &config,
            std::time::Duration::from_millis(250),
        );
    }
    let data_dir = linux_native_data_dir();
    let trusted_devices = crate::lan_sync_core::load_lan_devices_from_store(
        crate::lan_sync_core::lan_device_book_path(&data_dir),
        linux_native_decrypt_secret_from_storage,
        crate::lan_sync_core::normalize_lan_capabilities,
    );
    let discovered_devices = crate::lan_sync_core::load_lan_discovered_devices_from_store(
        crate::lan_sync_core::lan_discovered_device_cache_path(&data_dir),
        linux_native_decrypt_secret_from_storage,
        crate::lan_sync_core::normalize_lan_capabilities,
    );
    if runtime_settings.lan_sync_enabled {
        let _ = dispatch_linux_native_lan_background_clip_sync_once(
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
    let projection = settings_lan_device_book_projection("linux_native_host", devices);
    Some(ProductAdapterCommandResult {
        accepted: projection.accepted,
        result_name: projection.result_name,
    })
}

fn linux_native_now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn linux_native_save_lan_accepted_device(
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
        linux_native_data_dir(),
        new_device,
        linux_native_decrypt_secret_from_storage,
        linux_native_encrypt_secret_for_storage,
        crate::lan_sync_core::normalize_lan_capabilities,
    )
}

pub(crate) fn dispatch_linux_native_settings_lan_pair_approval_action(
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
        linux_native_data_dir(),
        None,
        accept,
        linux_native_now_ms(),
        linux_native_decrypt_secret_from_storage,
        linux_native_encrypt_secret_for_storage,
        crate::lan_sync_core::normalize_lan_capabilities,
    ) {
        Ok(Some(decision)) => ProductAdapterCommandResult {
            accepted: true,
            result_name: if decision.accepted {
                format!("zsclip.settings_sync.{action_name}.accepted_saved_on_linux_native_host")
            } else {
                format!("zsclip.settings_sync.{action_name}.rejected_on_linux_native_host")
            },
        },
        Ok(None) => ProductAdapterCommandResult {
            accepted: false,
            result_name: format!(
                "zsclip.settings_sync.{action_name}.no_pending_pair_on_linux_native_host"
            ),
        },
        Err(err) => ProductAdapterCommandResult {
            accepted: false,
            result_name: format!("zsclip.settings_sync.{action_name}.store_failed.{err}"),
        },
    }
    .into()
}

pub(crate) fn dispatch_linux_native_settings_lan_pair_action(
    action: SettingsAction,
) -> Option<ProductAdapterCommandResult> {
    if action != SettingsAction::PairLanDevice {
        return None;
    }
    let projection = settings_lan_pair_request_projection_from_json(
        action,
        "linux_native_host",
        &linux_native_settings_json_snapshot(),
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
                            linux_native_now_ms(),
                            &status_response,
                        );
                        if let Some(accepted_device) = status_projection.accepted_device {
                            match linux_native_save_lan_accepted_device(accepted_device) {
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

pub(crate) fn dispatch_linux_native_settings_route_action(
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
                linux_native_settings_platform_action_for_shared_action(action)
            {
                return dispatch_linux_native_settings_platform_action(platform_action);
            }
            ProductAdapterCommandResult {
                accepted: false,
                result_name: format!(
                    "zsclip.settings_platform.{}.unsupported_on_linux_native_host",
                    action_name
                ),
            }
        }
        SettingsActionRoute::Sync => {
            if linux_native_cloud_sync_action_for_shared_action(action).is_some() {
                return dispatch_linux_native_settings_webdav_action(action);
            }
            if let Some(result) = dispatch_linux_native_settings_lan_mobile_link_action(action) {
                return result;
            }
            if let Some(result) = dispatch_linux_native_settings_lan_device_book_action(action) {
                return result;
            }
            if let Some(result) = dispatch_linux_native_settings_lan_pair_approval_action(action) {
                return result;
            }
            if let Some(result) = dispatch_linux_native_settings_lan_pair_action(action) {
                return result;
            }
            let support_status_name =
                zsui_native_feature_status_for(NativeUiPlatform::Linux, "sync_lan")
                    .map(|status| status.support_status_name)
                    .unwrap_or("unknown_support_status");
            if let Some(plan) = settings_lan_sync_action_support_plan(
                action,
                "linux_native_host",
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
                    "zsclip.settings_sync.{}.unsupported_on_linux_native_host",
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

pub(crate) fn dispatch_linux_native_dialog_action(
    action: NativeHostDialogAction,
) -> ProductAdapterCommandResult {
    let dialog_host = LinuxDialogHost::default();
    let owner = LinuxMainWindowHandle(1);
    let result_name = match action {
        NativeHostDialogAction::ShowInfoMessage => {
            dialog_host.show_message(
                owner,
                action.title(),
                action.message(),
                NativeDialogLevel::Info,
            );
            "zsclip.dialog.show_info_message".to_string()
        }
        NativeHostDialogAction::ConfirmQuestion => {
            let response = dialog_host.confirm(
                owner,
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

pub(crate) fn dispatch_linux_native_status_menu_action(
    action: NativeHostStatusMenuAction,
) -> ProductAdapterCommandResult {
    let mut application = LinuxApplicationModel::default();
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
            persist_linux_native_bool_toggle("capture_enable", "clipboard_capture_enabled", true),
        ),
        #[cfg(feature = "lan-sync")]
        NativeHostStatusMenuAction::ToggleLanSync => Some(persist_linux_native_bool_toggle(
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

pub(crate) fn dispatch_linux_native_menu_command_id(menu_id: usize) -> ProductAdapterCommandResult {
    let Some(command) = main_menu_command_for_id(menu_id) else {
        return ProductAdapterCommandResult {
            accepted: false,
            result_name: format!("zsclip.invalid_native_menu_command_{}", menu_id),
        };
    };
    let mut application = LinuxApplicationModel::default();
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

fn native_dialog_response_name(response: NativeDialogResponse) -> &'static str {
    match response {
        NativeDialogResponse::Yes => "yes",
        NativeDialogResponse::No => "no",
        NativeDialogResponse::Cancel => "cancel",
    }
}

#[cfg(target_os = "linux")]
fn open_linux_url_or_file(target: String) -> Result<(), String> {
    use gtk4::gio::{AppInfo, File};
    use gtk4::prelude::*;

    if target.trim().is_empty() {
        return Err("empty target".to_string());
    }
    let uri = if target.contains("://") {
        target
    } else {
        File::for_path(target).uri().to_string()
    };
    AppInfo::launch_default_for_uri(&uri, None::<&gtk4::gio::AppLaunchContext>)
        .map_err(|err| err.to_string())
}

#[cfg(not(target_os = "linux"))]
fn open_linux_url_or_file(_target: String) -> Result<(), String> {
    Err("Linux host URL opening is only available on Linux".to_string())
}

fn percent_decode_uri_path(path: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(path.len());
    let raw = path.as_bytes();
    let mut index = 0;
    while index < raw.len() {
        if raw[index] == b'%' {
            if index + 2 >= raw.len() {
                return None;
            }
            let high = hex_value(raw[index + 1])?;
            let low = hex_value(raw[index + 2])?;
            bytes.push((high << 4) | low);
            index += 3;
        } else {
            bytes.push(raw[index]);
            index += 1;
        }
    }
    let decoded = String::from_utf8(bytes).ok()?;
    let local_path = decoded.strip_prefix("localhost/").unwrap_or(&decoded);
    Some(format!("/{}", local_path.trim_start_matches('/')))
}

fn file_paths_to_uri_list(paths: &[String]) -> Option<String> {
    let uris = paths
        .iter()
        .map(|path| path.trim())
        .filter(|path| !path.is_empty())
        .map(file_path_to_uri)
        .collect::<Vec<_>>();
    (!uris.is_empty()).then(|| {
        let mut text = uris.join("\n");
        text.push('\n');
        text
    })
}

fn file_path_to_uri(path: &str) -> String {
    if path.starts_with("file://") {
        return path.to_string();
    }
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    format!("file://{}", percent_encode_uri_path(&path))
}

fn percent_encode_uri_path(path: &str) -> String {
    let mut encoded = String::with_capacity(path.len());
    for byte in path.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*byte as char)
            }
            byte => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn clipboard_text_without_uri_list(text: String) -> Option<String> {
    if LinuxClipboardHost::file_paths_from_uri_list(&text).is_some() {
        None
    } else {
        Some(text)
    }
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(target_os = "linux")]
pub(crate) fn pick_linux_native_file(
    request: NativeFileDialogRequest<'_>,
) -> Result<Option<String>, String> {
    use gtk4::gio::File;
    use gtk4::glib::MainContext;
    use gtk4::prelude::*;
    use gtk4::{FileChooserAction, FileChooserNative, FileFilter, ResponseType};

    let filter = FileFilter::new();
    filter.set_name(Some(request.filter_name));
    for pattern in native_file_dialog_patterns(request.filter_pattern) {
        filter.add_pattern(pattern.as_str());
    }

    let dialog = FileChooserNative::builder()
        .title(request.title)
        .action(FileChooserAction::Open)
        .accept_label("Choose")
        .cancel_label("Cancel")
        .filter(&filter)
        .modal(true)
        .select_multiple(false)
        .build();

    if !request.current_path.trim().is_empty() {
        let file = File::for_path(request.current_path);
        let _ = dialog.set_file(&file);
    }

    let response = MainContext::default().block_on(dialog.run_future());
    let result = if response == ResponseType::Accept {
        let file = dialog
            .file()
            .ok_or_else(|| "GTK file chooser returned no selected file".to_string())?;
        let path = file
            .path()
            .ok_or_else(|| "GTK file chooser returned a non-local file".to_string())?;
        Ok(Some(path.to_string_lossy().to_string()))
    } else {
        Ok(None)
    };
    dialog.destroy();
    result
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn pick_linux_native_file(
    _request: NativeFileDialogRequest<'_>,
) -> Result<Option<String>, String> {
    Err("Linux native file picker is only available on Linux".to_string())
}

#[cfg(target_os = "linux")]
pub(crate) fn prompt_linux_native_text(
    request: NativeTextInputDialogRequest<'_>,
) -> Option<String> {
    use gtk4::glib::MainContext;
    use gtk4::prelude::*;
    use gtk4::{Box as GtkBox, Dialog, Entry, Label, Orientation, ResponseType};

    let dialog = Dialog::builder().title(request.title).modal(true).build();
    dialog.add_button("Cancel", ResponseType::Cancel);
    dialog.add_button("OK", ResponseType::Accept);
    dialog.set_default_response(ResponseType::Accept);

    let content = dialog.content_area();
    let body = GtkBox::new(Orientation::Vertical, 8);
    body.set_margin_top(16);
    body.set_margin_bottom(16);
    body.set_margin_start(16);
    body.set_margin_end(16);

    let label = Label::new(Some(request.label));
    label.set_xalign(0.0);
    let entry = Entry::new();
    entry.set_text(request.initial);
    entry.set_activates_default(true);

    body.append(&label);
    body.append(&entry);
    content.append(&body);

    dialog.present();
    let response = MainContext::default().block_on(dialog.run_future());
    let result = (response == ResponseType::Accept).then(|| entry.text().to_string());
    dialog.close();
    result
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn prompt_linux_native_text(
    _request: NativeTextInputDialogRequest<'_>,
) -> Option<String> {
    None
}

#[cfg(target_os = "linux")]
pub(crate) fn edit_linux_native_text(
    request: NativeEditTextDialogRequest<'_>,
    save_handler: &mut dyn NativeEditTextSaveHandler,
) -> NativeEditTextDialogResult {
    use gtk4::glib::MainContext;
    use gtk4::prelude::*;
    use gtk4::{Box as GtkBox, Dialog, Entry, Label, Orientation, ResponseType};

    let requested = request.initial_size.unwrap_or(Size {
        width: 560,
        height: 180,
    });
    let dialog = Dialog::builder().title(request.title).modal(true).build();
    dialog.add_button("Cancel", ResponseType::Cancel);
    dialog.add_button("Save", ResponseType::Accept);
    dialog.set_default_response(ResponseType::Accept);

    let content = dialog.content_area();
    let body = GtkBox::new(Orientation::Vertical, 8);
    body.set_margin_top(16);
    body.set_margin_bottom(16);
    body.set_margin_start(16);
    body.set_margin_end(16);

    let label = Label::new(Some("Edit clipboard text"));
    label.set_xalign(0.0);
    let entry = Entry::new();
    entry.set_text(request.initial_text);
    entry.set_width_chars((requested.width / 8).clamp(32, 90));
    entry.set_activates_default(true);

    body.append(&label);
    body.append(&entry);
    content.append(&body);

    dialog.present();
    let response = MainContext::default().block_on(dialog.run_future());
    let saved = if response == ResponseType::Accept {
        save_handler.save_text(&entry.text().to_string()).is_ok()
    } else {
        false
    };
    dialog.close();

    NativeEditTextDialogResult {
        saved,
        final_size: Some(requested),
    }
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn edit_linux_native_text(
    _request: NativeEditTextDialogRequest<'_>,
    _save_handler: &mut dyn NativeEditTextSaveHandler,
) -> NativeEditTextDialogResult {
    NativeEditTextDialogResult::default()
}

fn linux_source_url() -> String {
    option_env!("CARGO_PKG_REPOSITORY")
        .unwrap_or("")
        .trim()
        .to_string()
}

fn linux_latest_release_url() -> String {
    let repo = linux_source_url();
    if repo.is_empty() {
        String::new()
    } else {
        format!("{}/releases/latest", repo.trim_end_matches('/'))
    }
}

fn linux_wps_taskpane_docs_path() -> String {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("docs")
        .join("wps-taskpane.md")
        .to_string_lossy()
        .to_string()
}

fn native_file_dialog_patterns(filter_pattern: &str) -> Vec<String> {
    filter_pattern
        .split([';', ','])
        .map(str::trim)
        .filter(|pattern| !pattern.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub(crate) fn dispatch_linux_native_row_action(
    action: NativeHostRowAction,
) -> ProductAdapterCommandResult {
    let mut application = LinuxApplicationModel::default();
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

pub(crate) fn dispatch_linux_native_row_action_for_item(
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
                crate::app_core::native_host_write_clipboard_payload::<LinuxClipboardHost>(&write);
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
                    let host = LinuxShellOpenHost::default();
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
        NativeHostRowAction::OpenFolder => {
            match main_row_external_action_plan(MainRowMenuAction::OpenFolder, Some(&item), &[]) {
                Some(MainRowExternalActionPlan::OpenParentFolders(paths)) if !paths.is_empty() => {
                    let host = LinuxShellOpenHost::default();
                    let mut opened = 0;
                    for path in &paths {
                        if let Some(parent) = native_parent_folder_path(path) {
                            host.open_path(&parent);
                            opened += 1;
                        }
                    }
                    ProductAdapterCommandResult {
                        accepted: opened > 0,
                        result_name: if opened > 0 {
                            format!("zsclip.row.open_folder_native_{}", opened)
                        } else {
                            "zsclip.row.open_folder_no_parent".to_string()
                        },
                    }
                }
                _ => ProductAdapterCommandResult {
                    accepted: false,
                    result_name: "zsclip.row.open_folder_no_paths".to_string(),
                },
            }
        }
        NativeHostRowAction::CopyPath => {
            match main_row_external_action_plan(MainRowMenuAction::CopyPath, Some(&item), &[]) {
                Some(MainRowExternalActionPlan::CopyText(text)) if !text.is_empty() => {
                    let accepted = LinuxClipboardHost::write_text_ignored_by_monitors(&text);
                    ProductAdapterCommandResult {
                        accepted,
                        result_name: if accepted {
                            format!("zsclip.row.copy_path_clipboard_{}", text.len())
                        } else {
                            "zsclip.row.copy_path_clipboard_failed".to_string()
                        },
                    }
                }
                _ => ProductAdapterCommandResult {
                    accepted: false,
                    result_name: "zsclip.row.copy_path_no_paths".to_string(),
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
        _ => dispatch_linux_native_row_action(action),
    }
}

fn native_parent_folder_path(path: &str) -> Option<String> {
    std::path::Path::new(path)
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(|parent| parent.to_string_lossy().to_string())
}

pub(crate) fn dispatch_linux_native_edit_text_save(
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

pub(crate) fn dispatch_linux_native_assign_group(
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

pub(crate) fn dispatch_linux_native_remove_group(item_id: i64) -> ProductAdapterCommandResult {
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

pub(crate) fn dispatch_linux_native_group_filter(group_id: i64) -> ProductAdapterCommandResult {
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

pub(crate) fn dispatch_linux_native_create_group(
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

pub(crate) fn dispatch_linux_native_rename_group(
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

pub(crate) fn dispatch_linux_native_delete_group(group_id: i64) -> ProductAdapterCommandResult {
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

pub(crate) fn dispatch_linux_native_move_group(
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

pub(crate) fn dispatch_linux_native_search_text_action(
    action: NativeHostSearchTextAction,
) -> ProductAdapterCommandResult {
    let mut application = LinuxApplicationModel::default();
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

pub(crate) fn dispatch_linux_native_vv_select_event(
    index: usize,
) -> ProductAdapterAsyncBridgeResult {
    let mut application = LinuxApplicationModel::default();
    application.route_application_event(crate::app_core::native_host_vv_select_event(index))
}

#[allow(dead_code)]
fn linux_native_vv_trigger_state() -> &'static Mutex<NativeHostVvTriggerState> {
    static STATE: OnceLock<Mutex<NativeHostVvTriggerState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(NativeHostVvTriggerState::default()))
}

#[allow(dead_code)]
pub(crate) fn dispatch_linux_native_vv_trigger_key(
    input: NativeHostVvTriggerInput,
) -> NativeHostVvTriggerTransition {
    let Ok(mut state) = linux_native_vv_trigger_state().lock() else {
        return NativeHostVvTriggerTransition {
            action: NativeHostVvTriggerAction::Ignore,
            consume_key: false,
        };
    };
    let transition = state.apply(input);
    let mut application = LinuxApplicationModel::default();
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

fn linux_native_host_vv_clip_items_for_group(group_id: i64) -> Vec<ClipItem> {
    linux_native_host_projected_clip_items_for_group(group_id)
        .into_iter()
        .map(|item| {
            crate::db_runtime::native_clip_item(item.id)
                .ok()
                .flatten()
                .unwrap_or_else(|| crate::app_core::native_host_clip_item_from_projection(&item))
        })
        .collect()
}

pub(crate) fn dispatch_linux_native_vv_paste(index: usize) -> NativeHostVvPasteExecution {
    dispatch_linux_native_vv_paste_for_group(index, 0)
}

pub(crate) fn dispatch_linux_native_vv_paste_for_group(
    index: usize,
    group_id: i64,
) -> NativeHostVvPasteExecution {
    let items = linux_native_host_vv_clip_items_for_group(group_id);
    let mut application = LinuxApplicationModel::default();
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
        LinuxPasteTargetHandle(7),
        LinuxPasteTargetHandle(8),
        0,
    )
}

pub(crate) fn linux_native_identity_smoke() -> LinuxIdentitySmokeSummary {
    let identity = LinuxWindowIdentityHost::default();
    let foreground = identity.foreground_handle();
    let process_name = identity.process_name(foreground);
    let class_name = identity.class_name(foreground);
    let foreground_exists = identity.exists(foreground);
    let foreground_matches = identity.is_foreground(foreground);
    let current_process_window = identity.is_current_process_window(foreground);
    let mut paste_target = LinuxPasteTargetHost::default();
    let target = LinuxPasteTargetHandle(foreground.0);
    let foreground_requested = paste_target.force_paste_target_foreground(target);
    let focus_status = paste_target.paste_target_focus_status(target, target);

    LinuxIdentitySmokeSummary {
        foreground_seen: foreground.0 != 0,
        process_name_seen: !process_name.is_empty(),
        class_name_seen: !class_name.is_empty(),
        foreground_exists,
        foreground_matches,
        current_process_window,
        foreground_requested,
        focus_status,
    }
}

pub(crate) fn linux_native_host_projected_clip_items() -> Vec<NativeHostClipListItemProjection> {
    linux_native_host_projected_clip_items_for_group(0)
}

pub(crate) fn linux_native_host_projected_clip_items_for_group(
    group_id: i64,
) -> Vec<NativeHostClipListItemProjection> {
    linux_native_host_projected_clip_items_for_category_group(0, group_id)
}

pub(crate) fn linux_native_host_projected_clip_items_for_category_group(
    category: i64,
    group_id: i64,
) -> Vec<NativeHostClipListItemProjection> {
    linux_native_host_projected_clip_items_for_category_group_kind_filter(
        category,
        group_id,
        crate::app_core::ClipKindFilter::All,
    )
}

pub(crate) fn linux_native_host_projected_clip_items_for_category_group_kind_filter(
    category: i64,
    group_id: i64,
    kind_filter: crate::app_core::ClipKindFilter,
) -> Vec<NativeHostClipListItemProjection> {
    if let Ok(items) =
        crate::db_runtime::native_clip_list_items_for_group_kind_filter(
            category,
            group_id,
            kind_filter,
            64,
        )
    {
        return items;
    }
    Vec::new()
}

pub(crate) fn linux_native_host_projected_clip_items_for_category_group_kind_filter_search(
    category: i64,
    group_id: i64,
    kind_filter: crate::app_core::ClipKindFilter,
    search_text: &str,
) -> Vec<NativeHostClipListItemProjection> {
    if let Ok(items) = crate::db_runtime::native_clip_list_items_for_query(
        category,
        group_id,
        kind_filter,
        search_text,
        64,
    ) {
        return items;
    }
    Vec::new()
}

fn run_linux_contract_scaffold() -> Result<(), String> {
    let _adapter_boundary =
        crate::linux_gtk_adapter::LinuxGtkAdapterBoundary::default_from_linux_contract();
    let mut application = LinuxApplicationModel::default();
    let startup = application.mount(ZSUI_FRAMEWORK_NAME, true)?;
    let presentation = application
        .main_window_host_mut()
        .create_main_windows(startup.request.clone());
    application.record_startup_presentation(startup.backend, presentation);
    application.activate();
    Ok(())
}

impl LinuxApplicationModel {
    pub(crate) fn mount(
        &mut self,
        title: impl Into<String>,
        main_visible: bool,
    ) -> Result<LinuxStartupPlan, String> {
        let lifecycle = LifecycleEvent::Mount;
        if !self.lifecycle.apply(lifecycle) {
            return Err("Linux application lifecycle rejected mount".to_string());
        }
        let main_window = crate::zsui::Window::new(title)
            .size(760, 520)
            .visible(main_visible)
            .resizable(true)
            .decorations(true);
        Ok(LinuxStartupPlan {
            request: NativeMainWindowRequest::from_zsui_window_for_host(
                &main_window,
                &crate::zsui::HostCapabilities::linux_native_window_host(),
            ),
            lifecycle,
            backend: LinuxNativeBackend::Gtk4Libadwaita,
        })
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

    pub(crate) fn drain_command(&mut self) -> Option<Command> {
        self.commands.pop()
    }

    pub(crate) fn command_count(&self) -> usize {
        self.commands.len()
    }

    pub(crate) fn product_command_results(&self) -> &[ProductAdapterCommandResult] {
        &self.product_command_results
    }

    pub(crate) fn product_event_results(&self) -> &[ProductAdapterAsyncBridgeResult] {
        &self.product_event_results
    }

    pub(crate) fn ai_action_presentation_session(&self) -> &LinuxAiActionPresentationSessionState {
        &self.ai_action_presentation
    }

    pub(crate) fn route_application_event(
        &mut self,
        event: ApplicationEvent,
    ) -> ProductAdapterAsyncBridgeResult {
        let bridge = self.product_adapter.bridge_async_event(event);
        self.product_event_results.push(bridge.clone());
        bridge
    }

    pub(crate) fn set_clipboard_capture_enabled(&mut self, enabled: bool) {
        self.clipboard_capture_enabled = enabled;
    }

    pub(crate) fn poll_clipboard_capture_event(&mut self) -> Option<ApplicationEvent> {
        let result = poll_clipboard_monitor::<LinuxClipboardHost>(
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

    pub(crate) fn execute_native_vv_paste(
        &mut self,
        index: usize,
        items: &[ClipItem],
        target: LinuxPasteTargetHandle,
        focus: LinuxPasteTargetHandle,
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
                LinuxClipboardHost::write_text_ignored_by_monitors(text)
            }
            NativeHostClipboardWrite::FilePaths(paths) => {
                LinuxClipboardHost::write_file_paths(paths)
            }
            NativeHostClipboardWrite::ImageRgba {
                bytes,
                width,
                height,
            } => LinuxClipboardHost::write_image_rgba(bytes, *width, *height),
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

    pub(crate) fn main_window_host_mut(&mut self) -> &mut LinuxMainWindowHost {
        &mut self.main_window_host
    }

    pub(crate) fn autostart_host(&self) -> &LinuxAutostartHost {
        &self.autostart_host
    }

    pub(crate) fn autostart_host_mut(&mut self) -> &mut LinuxAutostartHost {
        &mut self.autostart_host
    }

    pub(crate) fn autostart_status(&self) -> NativeAutostartStatus {
        self.autostart_host.autostart_status()
    }

    pub(crate) fn apply_autostart(&mut self, enabled: bool) -> NativeAutostartApplyResult {
        self.autostart_host.set_autostart_enabled(enabled)
    }

    pub(crate) fn style_resolver(&self) -> &LinuxNativeStyleResolver {
        &self.style_resolver
    }

    pub(crate) fn control_mapper(&self) -> &LinuxNativeControlMapper {
        &self.control_mapper
    }

    pub(crate) fn text_layout(&self) -> &LinuxTextLayout {
        &self.text_layout
    }

    pub(crate) fn status_item_host_mut(&mut self) -> &mut LinuxStatusItemHost {
        &mut self.status_item_host
    }

    pub(crate) fn popup_menu_host_mut(&mut self) -> &mut LinuxPopupMenuHost {
        &mut self.popup_menu_host
    }

    pub(crate) fn transient_window_host_mut(&mut self) -> &mut LinuxTransientWindowHost {
        &mut self.transient_window_host
    }

    pub(crate) fn ime_host(&self) -> &LinuxImeHost {
        &self.ime_host
    }

    pub(crate) fn ime_host_mut(&mut self) -> &mut LinuxImeHost {
        &mut self.ime_host
    }

    pub(crate) fn dialog_host(&self) -> &LinuxDialogHost {
        &self.dialog_host
    }

    pub(crate) fn shell_open_host(&self) -> &LinuxShellOpenHost {
        &self.shell_open_host
    }

    pub(crate) fn window_identity_host(&self) -> &LinuxWindowIdentityHost {
        &self.window_identity_host
    }

    pub(crate) fn paste_target_host_mut(&mut self) -> &mut LinuxPasteTargetHost {
        &mut self.paste_target_host
    }

    pub(crate) fn text_caret_host(&self) -> &LinuxTextCaretHost {
        &self.text_caret_host
    }

    pub(crate) fn text_caret_host_mut(&mut self) -> &mut LinuxTextCaretHost {
        &mut self.text_caret_host
    }

    pub(crate) fn file_dialog_host(&self) -> &LinuxFileDialogHost {
        &self.file_dialog_host
    }

    pub(crate) fn text_input_dialog_host(&self) -> &LinuxTextInputDialogHost {
        &self.text_input_dialog_host
    }

    pub(crate) fn edit_text_dialog_host(&self) -> &LinuxEditTextDialogHost {
        &self.edit_text_dialog_host
    }

    pub(crate) fn mail_merge_window_host(&self) -> &LinuxMailMergeWindowHost {
        &self.mail_merge_window_host
    }

    pub(crate) fn renderer(&self) -> &LinuxRenderer {
        &self.renderer
    }

    pub(crate) fn main_search_host_mut(&mut self) -> &mut LinuxMainSearchControlHost {
        &mut self.main_search_host
    }

    pub(crate) fn settings_window_host_mut(&mut self) -> &mut LinuxSettingsWindowHost {
        &mut self.settings_window_host
    }

    pub(crate) fn settings_control_host_mut(&mut self) -> &mut LinuxSettingsControlHost {
        &mut self.settings_control_host
    }

    pub(crate) fn settings_dropdown_host_mut(&mut self) -> &mut LinuxSettingsDropdownHost {
        &mut self.settings_dropdown_host
    }

    pub(crate) fn startup_session(&self) -> &LinuxStartupSessionState {
        &self.startup_session
    }

    pub(crate) fn record_startup_presentation(
        &mut self,
        backend: LinuxNativeBackend,
        presentation: NativeMainWindowPresentation<LinuxMainWindowHandle>,
    ) {
        self.startup_session.backend = Some(backend);
        if let NativeMainWindowPresentation::Created(handles) = presentation {
            self.startup_session.created_main_windows = Some(handles);
        }
        self.startup_session.generation = self.startup_session.generation.saturating_add(1);
    }
}

impl NativeRuntimeDriver for LinuxApplicationModel {
    type WindowHandle = LinuxMainWindowHandle;

    fn start_runtime(
        &mut self,
        request: NativeRuntimeStartupRequest,
    ) -> NativeRuntimeStartupResult<Self::WindowHandle> {
        if !self.lifecycle.apply(LifecycleEvent::Mount) {
            return NativeRuntimeStartupResult::Failed;
        }
        if let Some(tooltip) = request.status_item_tooltip.as_deref() {
            self.status_item_host.install(tooltip);
        }
        let presentation = self
            .main_window_host
            .create_main_windows(request.main_window);
        self.record_startup_presentation(LinuxNativeBackend::Gtk4Libadwaita, presentation);
        match presentation {
            NativeMainWindowPresentation::Created(handles) => {
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

impl NativeAiActionPresenter for LinuxApplicationModel {
    type WindowHandle = LinuxMainWindowHandle;

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

impl LinuxStartupSessionState {
    pub(crate) fn backend(&self) -> Option<LinuxNativeBackend> {
        self.backend
    }

    pub(crate) fn created_main_windows(
        &self,
    ) -> Option<NativeMainWindowHandles<LinuxMainWindowHandle>> {
        self.created_main_windows
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl LinuxMainWindowHost {
    pub(crate) fn create_request_count(&self) -> usize {
        self.create_requests.len()
    }

    pub(crate) fn create_requests(&self) -> &[NativeMainWindowRequest] {
        &self.create_requests
    }

    pub(crate) fn created_handles(&self) -> Option<NativeMainWindowHandles<LinuxMainWindowHandle>> {
        self.created
    }

    pub(crate) fn repaint_request_count(&self) -> usize {
        self.repaint_requests.len()
    }
}

impl LinuxStatusItemHost {
    pub(crate) fn installed_tooltip(&self) -> Option<&str> {
        self.installed_tooltip.as_deref()
    }

    pub(crate) fn menu_entry_count(&self) -> usize {
        self.menu_entries.len()
    }
}

impl LinuxPopupMenuHost {
    pub(crate) fn presentation_count(&self) -> usize {
        self.presentations.len()
    }

    pub(crate) fn last_selected_id(&self) -> Option<usize> {
        self.presentations
            .last()
            .map(|presentation| presentation.selected_id)
    }
}

impl LinuxNativeStyleResolver {
    pub(crate) fn resolved_count(&self) -> usize {
        self.resolved.borrow().len()
    }
}

impl LinuxNativeControlMapper {
    pub(crate) fn mapped_controls(&self) -> Vec<(SettingsComponentKind, LinuxNativeControlClass)> {
        self.mapped.borrow().clone()
    }
}

impl LinuxTextLayout {
    pub(crate) fn actions(&self) -> Vec<LinuxTextLayoutAction> {
        self.actions.borrow().clone()
    }
}

impl LinuxTransientWindowHost {
    pub(crate) fn requests(&self) -> &[LinuxTransientWindowCreateRequest] {
        &self.requests
    }

    pub(crate) fn actions(&self) -> &[LinuxTransientWindowAction] {
        &self.actions
    }
}

impl LinuxImeHost {
    pub(crate) fn set_next_candidate(&self, anchor: Option<NativeImeCandidateAnchor>) {
        *self.next_candidate.borrow_mut() = anchor;
    }

    pub(crate) fn set_next_composition(&self, anchor: Option<NativeImeCompositionAnchor>) {
        *self.next_composition.borrow_mut() = anchor;
    }

    pub(crate) fn set_next_has_default_ime_window(&mut self, value: bool) {
        self.next_has_default_ime_window = value;
    }

    pub(crate) fn actions(&self) -> Vec<LinuxImeAction> {
        self.actions.borrow().clone()
    }
}

impl LinuxClipboardHost {
    fn file_paths_from_uri_list(text: &str) -> Option<Vec<String>> {
        let paths = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .filter_map(|line| line.strip_prefix("file://"))
            .filter_map(|path| {
                let decoded = percent_decode_uri_path(path)?;
                if decoded.is_empty() {
                    return None;
                }
                Some(decoded)
            })
            .collect::<Vec<_>>();
        (!paths.is_empty()).then_some(paths)
    }

    #[cfg(all(target_os = "linux", not(test)))]
    fn system_read_text() -> Option<String> {
        let mut clipboard = arboard::Clipboard::new().ok()?;
        clipboard.get_text().ok()
    }

    #[cfg(all(target_os = "linux", not(test)))]
    fn system_write_text(text: &str) -> bool {
        let Ok(mut clipboard) = arboard::Clipboard::new() else {
            return false;
        };
        clipboard.set_text(text.to_string()).is_ok()
    }

    #[cfg(all(target_os = "linux", not(test)))]
    fn system_read_image_rgba() -> Option<(Vec<u8>, usize, usize)> {
        let mut clipboard = arboard::Clipboard::new().ok()?;
        let image = clipboard.get_image().ok()?;
        Some((image.bytes.into_owned(), image.width, image.height))
    }

    #[cfg(all(target_os = "linux", not(test)))]
    fn system_read_file_paths() -> Option<Vec<String>> {
        Self::system_read_text().and_then(|text| Self::file_paths_from_uri_list(&text))
    }

    #[cfg(all(target_os = "linux", not(test)))]
    fn system_write_file_paths(paths: &[String]) -> bool {
        let Some(uri_list) = file_paths_to_uri_list(paths) else {
            return false;
        };
        Self::system_write_text(&uri_list)
    }

    #[cfg(all(target_os = "linux", not(test)))]
    fn system_write_image_rgba(bytes: &[u8], width: usize, height: usize) -> bool {
        let Ok(mut clipboard) = arboard::Clipboard::new() else {
            return false;
        };
        clipboard
            .set_image(arboard::ImageData {
                width,
                height,
                bytes: std::borrow::Cow::Owned(bytes.to_vec()),
            })
            .is_ok()
    }

    #[cfg(all(target_os = "linux", not(test)))]
    fn system_clipboard_fingerprint() -> Option<u64> {
        let mut hasher = DefaultHasher::new();

        if let Some(text) = Self::system_read_text() {
            "text".hash(&mut hasher);
            text.hash(&mut hasher);
            return Some(hasher.finish());
        }

        if let Some((bytes, width, height)) = Self::system_read_image_rgba() {
            "image".hash(&mut hasher);
            width.hash(&mut hasher);
            height.hash(&mut hasher);
            bytes.hash(&mut hasher);
            return Some(hasher.finish());
        }

        None
    }

    #[cfg(not(all(target_os = "linux", not(test))))]
    fn system_clipboard_fingerprint() -> Option<u64> {
        None
    }

    fn observed_system_sequence() -> Option<u32> {
        let fingerprint = Self::system_clipboard_fingerprint()?;
        Some(Self::mutate_state(|state| {
            match state.last_system_fingerprint {
                None => {
                    state.last_system_fingerprint = Some(fingerprint);
                }
                Some(previous) if previous != fingerprint => {
                    state.last_system_fingerprint = Some(fingerprint);
                    state.sequence = state.sequence.saturating_add(1);
                }
                Some(_) => {}
            }
            state.sequence
        }))
    }

    fn state() -> &'static Mutex<LinuxClipboardState> {
        static STATE: OnceLock<Mutex<LinuxClipboardState>> = OnceLock::new();
        STATE.get_or_init(|| Mutex::new(LinuxClipboardState::default()))
    }

    fn mutate_state<R>(mutator: impl FnOnce(&mut LinuxClipboardState) -> R) -> R {
        let mut state = Self::state()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        mutator(&mut state)
    }

    #[cfg(test)]
    pub(crate) fn reset_for_tests() {
        Self::mutate_state(|state| *state = LinuxClipboardState::default());
    }
}

impl LinuxDialogHost {
    pub(crate) fn set_next_response(&self, response: NativeDialogResponse) {
        *self.next_response.borrow_mut() = response;
    }

    pub(crate) fn messages(&self) -> Vec<LinuxDialogMessage> {
        self.messages.borrow().clone()
    }

    pub(crate) fn confirmations(&self) -> Vec<LinuxDialogConfirmation> {
        self.confirmations.borrow().clone()
    }
}

impl LinuxShellOpenHost {
    #[cfg(all(target_os = "linux", not(test)))]
    fn system_open_path(path: &str) -> bool {
        if matches!(
            std::env::var("ZSCLIP_NATIVE_HOST_SHELL_OPEN_DRY_RUN").as_deref(),
            Ok("1")
        ) {
            return true;
        }
        open_linux_url_or_file(path.to_string()).is_ok()
    }

    #[cfg(not(all(target_os = "linux", not(test))))]
    fn system_open_path(_path: &str) -> bool {
        true
    }

    pub(crate) fn opened_paths(&self) -> Vec<String> {
        self.opened_paths.borrow().clone()
    }
}

impl LinuxWindowIdentityHost {
    pub(crate) fn set_process_name(&self, value: &str) {
        *self.process_name.borrow_mut() = value.to_string();
    }

    pub(crate) fn set_class_name(&self, value: &str) {
        *self.class_name.borrow_mut() = value.to_string();
    }

    pub(crate) fn set_root_handle(&self, handle: Option<LinuxWindowIdentityHandle>) {
        *self.root_handle.borrow_mut() = handle;
    }

    pub(crate) fn set_foreground_handle(&self, handle: Option<LinuxWindowIdentityHandle>) {
        *self.foreground_handle.borrow_mut() = handle;
    }

    pub(crate) fn set_existing_windows(&self, handles: Vec<LinuxWindowIdentityHandle>) {
        *self.existing_windows.borrow_mut() = handles;
    }

    pub(crate) fn set_current_process_windows(&self, handles: Vec<LinuxWindowIdentityHandle>) {
        *self.current_process_windows.borrow_mut() = handles;
    }

    pub(crate) fn actions(&self) -> Vec<LinuxWindowIdentityAction> {
        self.actions.borrow().clone()
    }
}

impl LinuxPasteTargetHost {
    pub(crate) fn set_next_foreground_result(&mut self, result: bool) {
        self.next_foreground_result = result;
    }

    pub(crate) fn set_next_text_input_capabilities(
        &mut self,
        capabilities: PasteTargetTextInputCapabilities,
    ) {
        self.next_text_input_capabilities = capabilities;
    }

    pub(crate) fn set_next_focus_status(&mut self, status: PasteTargetFocusStatus) {
        self.next_focus_status = status;
    }

    pub(crate) fn set_next_text_input_ready(&mut self, ready: bool) {
        self.next_text_input_ready = ready;
    }

    pub(crate) fn actions(&self) -> &[LinuxPasteTargetAction] {
        &self.actions
    }
}

impl LinuxTextCaretHost {
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

    pub(crate) fn set_next_focus_handle(&self, handle: Option<LinuxTextCaretHandle>) {
        *self.next_focus_handle.borrow_mut() = handle;
    }

    pub(crate) fn actions(&self) -> Vec<LinuxTextCaretAction> {
        self.actions.borrow().clone()
    }
}

impl LinuxFileDialogHost {
    #[cfg(all(target_os = "linux", not(test)))]
    fn system_pick_file(request: NativeFileDialogRequest<'_>) -> Result<Option<String>, String> {
        if let Ok(path) = std::env::var("ZSCLIP_NATIVE_HOST_FILE_PICKER_SMOKE_PATH") {
            return Ok(Some(path));
        }
        pick_linux_native_file(request)
    }

    #[cfg(not(all(target_os = "linux", not(test))))]
    fn system_pick_file(_request: NativeFileDialogRequest<'_>) -> Result<Option<String>, String> {
        Err("Linux native file picker is disabled outside Linux app builds".to_string())
    }

    pub(crate) fn set_next_result(&self, result: Result<Option<String>, String>) {
        *self.next_result.borrow_mut() = result;
    }

    pub(crate) fn requests(&self) -> Vec<LinuxFileDialogRecord> {
        self.requests.borrow().clone()
    }
}

impl LinuxTextInputDialogHost {
    #[cfg(all(target_os = "linux", not(test)))]
    fn system_prompt_text(request: NativeTextInputDialogRequest<'_>) -> Option<String> {
        prompt_linux_native_text(request)
    }

    #[cfg(not(all(target_os = "linux", not(test))))]
    fn system_prompt_text(_request: NativeTextInputDialogRequest<'_>) -> Option<String> {
        None
    }

    pub(crate) fn set_next_result(&self, result: Option<String>) {
        *self.next_result.borrow_mut() = result;
    }

    pub(crate) fn requests(&self) -> Vec<LinuxTextInputDialogRecord> {
        self.requests.borrow().clone()
    }
}

impl LinuxEditTextDialogHost {
    #[cfg(all(target_os = "linux", not(test)))]
    fn system_open_edit_text(
        request: NativeEditTextDialogRequest<'_>,
        save_handler: &mut dyn NativeEditTextSaveHandler,
    ) -> NativeEditTextDialogResult {
        edit_linux_native_text(request, save_handler)
    }

    #[cfg(not(all(target_os = "linux", not(test))))]
    fn system_open_edit_text(
        _request: NativeEditTextDialogRequest<'_>,
        _save_handler: &mut dyn NativeEditTextSaveHandler,
    ) -> NativeEditTextDialogResult {
        NativeEditTextDialogResult::default()
    }

    pub(crate) fn set_next_result(&self, saved_text: Option<String>, final_size: Option<Size>) {
        *self.next_saved_text.borrow_mut() = saved_text;
        *self.next_final_size.borrow_mut() = final_size;
    }

    pub(crate) fn requests(&self) -> Vec<LinuxEditTextDialogRecord> {
        self.requests.borrow().clone()
    }
}

impl LinuxMailMergeWindowHost {
    pub(crate) fn requests(&self) -> Vec<LinuxMailMergeWindowRecord> {
        self.requests.borrow().clone()
    }
}

impl LinuxRenderer {
    pub(crate) fn commands(&self) -> &[LinuxRenderCommand] {
        &self.commands
    }
}

impl LinuxMainSearchControlHost {
    pub(crate) fn search_count(&self) -> usize {
        self.searches.len()
    }

    pub(crate) fn style_request_count(&self) -> usize {
        self.style_requests.len()
    }

    fn search(&self, handle: LinuxMainSearchHandle) -> Option<&LinuxMainSearchRecord> {
        self.searches.iter().find(|search| search.handle == handle)
    }

    fn search_mut(&mut self, handle: LinuxMainSearchHandle) -> Option<&mut LinuxMainSearchRecord> {
        self.searches
            .iter_mut()
            .find(|search| search.handle == handle)
    }
}

impl LinuxSettingsWindowHost {
    pub(crate) fn present_request_count(&self) -> usize {
        self.present_requests.len()
    }

    pub(crate) fn presented_handle(&self) -> Option<LinuxMainWindowHandle> {
        self.presented
    }

    pub(crate) fn repaint_request_count(&self) -> usize {
        self.repaint_requests.len()
    }

    pub(crate) fn focused_count(&self) -> usize {
        self.focused.len()
    }
}

impl LinuxSettingsControlHost {
    pub(crate) fn control_count(&self) -> usize {
        self.controls
            .iter()
            .filter(|control| !control.destroyed)
            .count()
    }

    pub(crate) fn repaint_count(&self, handle: LinuxSettingsControlHandle) -> u64 {
        self.control(handle)
            .map(|control| control.repaint_count)
            .unwrap_or_default()
    }

    fn control(&self, handle: LinuxSettingsControlHandle) -> Option<&LinuxSettingsControlRecord> {
        self.controls
            .iter()
            .find(|control| control.handle == handle && !control.destroyed)
    }

    fn control_mut(
        &mut self,
        handle: LinuxSettingsControlHandle,
    ) -> Option<&mut LinuxSettingsControlRecord> {
        self.controls
            .iter_mut()
            .find(|control| control.handle == handle && !control.destroyed)
    }
}

impl LinuxSettingsDropdownHost {
    pub(crate) fn dropdown_count(&self) -> usize {
        self.dropdowns
            .iter()
            .filter(|dropdown| !dropdown.destroyed)
            .count()
    }

    fn dropdown(
        &self,
        handle: LinuxSettingsDropdownHandle,
    ) -> Option<&LinuxSettingsDropdownRecord> {
        self.dropdowns
            .iter()
            .find(|dropdown| dropdown.handle == handle && !dropdown.destroyed)
    }

    fn dropdown_mut(
        &mut self,
        handle: LinuxSettingsDropdownHandle,
    ) -> Option<&mut LinuxSettingsDropdownRecord> {
        self.dropdowns
            .iter_mut()
            .find(|dropdown| dropdown.handle == handle && !dropdown.destroyed)
    }
}

impl ClipboardHost for LinuxClipboardHost {
    fn read_text() -> Option<String> {
        #[cfg(all(target_os = "linux", not(test)))]
        if let Some(text) = Self::system_read_text() {
            return clipboard_text_without_uri_list(text);
        }

        Self::mutate_state(|state| state.text.clone())
    }

    fn write_text(text: &str) -> bool {
        #[cfg(all(target_os = "linux", not(test)))]
        let system_written = Self::system_write_text(text);
        #[cfg(not(all(target_os = "linux", not(test))))]
        let system_written = true;

        Self::mutate_state(|state| {
            state.text = Some(text.to_string());
            state.image = None;
            state.file_paths = None;
            state.sequence = state.sequence.saturating_add(1);
        });
        system_written
    }

    fn read_image_rgba() -> Option<(Vec<u8>, usize, usize)> {
        #[cfg(all(target_os = "linux", not(test)))]
        if let Some(image) = Self::system_read_image_rgba() {
            return Some(image);
        }

        Self::mutate_state(|state| state.image.clone())
    }

    fn write_image_rgba(bytes: &[u8], width: usize, height: usize) -> bool {
        if width == 0
            || height == 0
            || bytes.len() != width.saturating_mul(height).saturating_mul(4)
        {
            return false;
        }
        #[cfg(all(target_os = "linux", not(test)))]
        let system_written = Self::system_write_image_rgba(bytes, width, height);
        #[cfg(not(all(target_os = "linux", not(test))))]
        let system_written = true;

        Self::mutate_state(|state| {
            state.text = None;
            state.image = Some((bytes.to_vec(), width, height));
            state.file_paths = None;
            state.sequence = state.sequence.saturating_add(1);
        });
        system_written
    }

    fn read_file_paths() -> Option<Vec<String>> {
        #[cfg(all(target_os = "linux", not(test)))]
        if let Some(paths) = Self::system_read_file_paths() {
            return Some(paths);
        }

        Self::mutate_state(|state| state.file_paths.clone())
    }

    fn write_file_paths(paths: &[String]) -> bool {
        #[cfg(all(target_os = "linux", not(test)))]
        let system_written = Self::system_write_file_paths(paths);
        #[cfg(not(all(target_os = "linux", not(test))))]
        let system_written = !paths.is_empty();

        Self::mutate_state(|state| {
            state.text = None;
            state.image = None;
            state.file_paths = Some(paths.to_vec());
            state.sequence = state.sequence.saturating_add(1);
        });
        system_written
    }

    fn sequence_number() -> u32 {
        Self::observed_system_sequence()
            .unwrap_or_else(|| Self::mutate_state(|state| state.sequence))
    }

    fn write_text_ignored_by_monitors(text: &str) -> bool {
        if !Self::write_text(text) {
            return false;
        }
        Self::mutate_state(|state| {
            state.ignore_next_capture = true;
            true
        })
    }

    fn should_ignore_capture_by_named_format() -> bool {
        Self::mutate_state(|state| {
            let ignore = state.ignore_next_capture;
            state.ignore_next_capture = false;
            ignore
        })
    }
}

impl StatusItemHost for LinuxStatusItemHost {
    fn install(&mut self, tooltip: &str) -> bool {
        self.installed_tooltip = Some(tooltip.to_string());
        self.install_count = self.install_count.saturating_add(1);
        true
    }

    fn remove(&mut self) {
        self.installed_tooltip = None;
        self.remove_count = self.remove_count.saturating_add(1);
    }

    fn present_menu(&mut self, entries: &[StatusMenuEntry]) {
        self.menu_entries = entries.to_vec();
    }
}

impl NativePopupMenuHost for LinuxPopupMenuHost {
    type Owner = LinuxMainWindowHandle;

    fn present_popup_menu(
        &mut self,
        owner: Self::Owner,
        x: i32,
        y: i32,
        placement: NativePopupMenuPlacement,
        entries: &[NativePopupMenuEntry],
    ) -> usize {
        let selected_id = first_enabled_popup_command(entries).unwrap_or_default();
        self.presentations.push(LinuxPopupMenuPresentation {
            owner,
            x,
            y,
            placement,
            entries: entries.to_vec(),
            selected_id,
        });
        selected_id
    }
}

fn first_enabled_popup_command(entries: &[NativePopupMenuEntry]) -> Option<usize> {
    entries.iter().find_map(|entry| match entry {
        NativePopupMenuEntry::Command { id, enabled, .. } if *enabled => Some(*id),
        NativePopupMenuEntry::Submenu {
            enabled, entries, ..
        } if *enabled => first_enabled_popup_command(entries),
        _ => None,
    })
}

impl NativeStyleResolver for LinuxNativeStyleResolver {
    fn resolve_text_style(&self, style: SemanticTextStyle) -> TextStyle {
        self.resolved.borrow_mut().push(style);
        let color = match style.color {
            ColorRole::PrimaryText => Color {
                r: 34,
                g: 38,
                b: 44,
                a: 255,
            },
            ColorRole::SecondaryText => Color {
                r: 92,
                g: 99,
                b: 112,
                a: 255,
            },
            ColorRole::Accent => Color {
                r: 20,
                g: 115,
                b: 230,
                a: 255,
            },
            ColorRole::Surface => Color {
                r: 246,
                g: 247,
                b: 249,
                a: 255,
            },
            ColorRole::Control => Color {
                r: 228,
                g: 232,
                b: 238,
                a: 255,
            },
            ColorRole::Danger => Color {
                r: 204,
                g: 56,
                b: 56,
                a: 255,
            },
        };
        let size = match style.role {
            TextRole::Caption => 12.0,
            TextRole::Title => 18.0,
            TextRole::Button => 14.0,
            TextRole::Monospace => 13.0,
            TextRole::Body => 14.0,
        };
        TextStyle {
            font_family: if matches!(style.role, TextRole::Monospace) {
                "Monospace".to_string()
            } else {
                "Sans".to_string()
            },
            size,
            weight: style.weight,
            color,
            horizontal_align: style.horizontal_align,
            vertical_align: style.vertical_align,
            wrap: style.wrap,
            ellipsis: style.ellipsis,
        }
    }
}

impl NativeControlMapper for LinuxNativeControlMapper {
    type ClassName = LinuxNativeControlClass;

    fn class_name(&self, kind: SettingsComponentKind) -> Self::ClassName {
        let class = match kind {
            SettingsComponentKind::Label => LinuxNativeControlClass::Label,
            SettingsComponentKind::TextInput => LinuxNativeControlClass::Entry,
            SettingsComponentKind::Toggle => LinuxNativeControlClass::Switch,
            SettingsComponentKind::Dropdown => LinuxNativeControlClass::ComboRow,
            SettingsComponentKind::Button => LinuxNativeControlClass::Button,
            SettingsComponentKind::AccentButton => LinuxNativeControlClass::SuggestedActionButton,
        };
        self.mapped.borrow_mut().push((kind, class));
        class
    }
}

impl TextLayout for LinuxTextLayout {
    fn measure(&self, text: &str, style: &TextStyle, max_width: i32) -> Size {
        self.actions
            .borrow_mut()
            .push(LinuxTextLayoutAction::Measure {
                text: text.to_string(),
                style: style.clone(),
                max_width,
            });
        let width = ((text.chars().count() as f32 * style.size * 0.6).ceil() as i32)
            .max(1)
            .min(max_width.max(1));
        let height = (style.size * 1.4).ceil() as i32;
        Size { width, height }
    }

    fn layout_runs(&self, text: &str, style: &TextStyle, bounds: Rect) -> Vec<TextRun> {
        self.actions
            .borrow_mut()
            .push(LinuxTextLayoutAction::LayoutRuns {
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

impl NativeTransientWindowHost for LinuxTransientWindowHost {
    type Handle = LinuxTransientWindowHandle;
    type Owner = LinuxMainWindowHandle;

    fn create_transient_window(
        &mut self,
        request: NativeTransientWindowRequest<Self::Owner>,
    ) -> NativeTransientWindowPresentation<Self::Handle> {
        let handle = LinuxTransientWindowHandle(self.next_handle);
        self.next_handle = self.next_handle.saturating_add(1);
        self.requests.push(LinuxTransientWindowCreateRequest {
            owner: request.owner,
            bounds: request.bounds,
        });
        NativeTransientWindowPresentation::Created(handle)
    }

    fn present_transient_window(&mut self, handle: Self::Handle, bounds: UiRect) {
        self.actions
            .push(LinuxTransientWindowAction::Present { handle, bounds });
    }

    fn hide_transient_window(&mut self, handle: Self::Handle) {
        self.actions.push(LinuxTransientWindowAction::Hide(handle));
    }

    fn destroy_transient_window(&mut self, handle: Self::Handle) {
        self.actions
            .push(LinuxTransientWindowAction::Destroy(handle));
    }
}

impl NativeImeHost for LinuxImeHost {
    type Handle = LinuxImeHandle;

    fn candidate_anchor(
        &mut self,
        focus: Self::Handle,
        index: u32,
    ) -> Option<NativeImeCandidateAnchor> {
        self.actions
            .borrow_mut()
            .push(LinuxImeAction::QueryCandidate { focus, index });
        *self.next_candidate.borrow()
    }

    fn composition_anchor(&mut self, focus: Self::Handle) -> Option<NativeImeCompositionAnchor> {
        self.actions
            .borrow_mut()
            .push(LinuxImeAction::QueryComposition(focus));
        *self.next_composition.borrow()
    }

    fn has_default_ime_window(&mut self, focus: Self::Handle) -> bool {
        self.actions
            .borrow_mut()
            .push(LinuxImeAction::HasDefaultImeWindow(focus));
        self.next_has_default_ime_window
    }
}

impl NativeDialogHost for LinuxDialogHost {
    type Owner = LinuxMainWindowHandle;

    fn show_message(
        &self,
        owner: Self::Owner,
        title: &str,
        message: &str,
        level: NativeDialogLevel,
    ) {
        self.messages.borrow_mut().push(LinuxDialogMessage {
            owner,
            title: title.to_string(),
            message: message.to_string(),
            level,
        });
    }

    fn confirm(
        &self,
        owner: Self::Owner,
        title: &str,
        message: &str,
        level: NativeDialogLevel,
        buttons: NativeDialogButtons,
    ) -> NativeDialogResponse {
        self.confirmations
            .borrow_mut()
            .push(LinuxDialogConfirmation {
                owner,
                title: title.to_string(),
                message: message.to_string(),
                level,
                buttons,
            });
        *self.next_response.borrow()
    }
}

impl NativeShellOpenHost for LinuxShellOpenHost {
    fn open_path(&self, path: &str) {
        self.opened_paths.borrow_mut().push(path.to_string());
        let _ = Self::system_open_path(path);
    }
}

impl NativeWindowIdentityHost for LinuxWindowIdentityHost {
    type Handle = LinuxWindowIdentityHandle;

    fn process_name(&self, handle: Self::Handle) -> String {
        self.actions
            .borrow_mut()
            .push(LinuxWindowIdentityAction::ProcessName(handle));
        let injected = self.process_name.borrow().clone();
        if injected.is_empty() {
            linux_process_name_for_window(handle).unwrap_or_default()
        } else {
            injected
        }
    }

    fn class_name(&self, handle: Self::Handle) -> String {
        self.actions
            .borrow_mut()
            .push(LinuxWindowIdentityAction::ClassName(handle));
        let injected = self.class_name.borrow().clone();
        if injected.is_empty() {
            linux_window_class_name(handle).unwrap_or_default()
        } else {
            injected
        }
    }

    fn root_handle(&self, handle: Self::Handle) -> Self::Handle {
        self.actions
            .borrow_mut()
            .push(LinuxWindowIdentityAction::RootHandle(handle));
        self.root_handle.borrow().unwrap_or(handle)
    }

    fn foreground_handle(&self) -> Self::Handle {
        self.actions
            .borrow_mut()
            .push(LinuxWindowIdentityAction::ForegroundHandle);
        self.foreground_handle
            .borrow()
            .or_else(linux_foreground_window_handle)
            .unwrap_or_default()
    }

    fn exists(&self, handle: Self::Handle) -> bool {
        self.actions
            .borrow_mut()
            .push(LinuxWindowIdentityAction::Exists(handle));
        self.existing_windows.borrow().contains(&handle) || linux_window_exists(handle)
    }

    fn is_foreground(&self, handle: Self::Handle) -> bool {
        self.actions
            .borrow_mut()
            .push(LinuxWindowIdentityAction::IsForeground(handle));
        self.foreground_handle
            .borrow()
            .or_else(linux_foreground_window_handle)
            == Some(handle)
    }

    fn is_current_process_window(&self, handle: Self::Handle) -> bool {
        self.actions
            .borrow_mut()
            .push(LinuxWindowIdentityAction::IsCurrentProcessWindow(handle));
        self.current_process_windows.borrow().contains(&handle)
            || linux_window_pid(handle).is_some_and(|pid| pid == std::process::id())
    }
}

impl NativePasteTargetHost for LinuxPasteTargetHost {
    type Handle = LinuxPasteTargetHandle;

    fn force_paste_target_foreground(&mut self, target: Self::Handle) -> bool {
        self.actions
            .push(LinuxPasteTargetAction::ForceForeground(target));
        self.next_foreground_result || linux_activate_window(LinuxWindowIdentityHandle(target.0))
    }

    fn restore_paste_target_focus(&mut self, target: Self::Handle, focus: Self::Handle) {
        self.actions
            .push(LinuxPasteTargetAction::RestoreFocus { target, focus });
    }

    fn set_paste_target_text(&mut self, target: Self::Handle, text: &str) -> bool {
        self.actions.push(LinuxPasteTargetAction::SetText {
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
            .push(LinuxPasteTargetAction::QueryTextInputCapabilities(target));
        self.next_text_input_capabilities
    }

    fn paste_target_focus_status(
        &mut self,
        target: Self::Handle,
        passthrough_focus: Self::Handle,
    ) -> PasteTargetFocusStatus {
        self.actions.push(LinuxPasteTargetAction::QueryFocusStatus {
            target,
            passthrough_focus,
        });
        if self.next_focus_status != PasteTargetFocusStatus::Unknown {
            return self.next_focus_status;
        }
        linux_foreground_window_handle()
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
            .push(LinuxPasteTargetAction::QueryTextInputReady(target));
        self.next_text_input_ready
    }

    fn send_paste_shortcut(&mut self, target: Self::Handle) -> bool {
        self.actions
            .push(LinuxPasteTargetAction::SendPasteShortcut(target));
        linux_send_ctrl_v(LinuxWindowIdentityHandle(target.0)).unwrap_or(true)
    }
}

fn linux_foreground_window_handle() -> Option<LinuxWindowIdentityHandle> {
    linux_command_line("xdotool", &["getwindowfocus"])
        .or_else(|| linux_command_line("xdotool", &["getactivewindow"]))
        .and_then(|line| linux_parse_window_id(&line))
}

fn linux_process_name_for_window(handle: LinuxWindowIdentityHandle) -> Option<String> {
    let pid = linux_window_pid(handle)?;
    let path = format!("/proc/{pid}/comm");
    std::fs::read_to_string(path)
        .ok()
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

fn linux_window_class_name(handle: LinuxWindowIdentityHandle) -> Option<String> {
    if handle.0 == 0 {
        return None;
    }
    linux_command_line("xprop", &["-id", &handle.0.to_string(), "WM_CLASS"]).map(|line| {
        line.split('=')
            .nth(1)
            .unwrap_or(line.as_str())
            .trim()
            .trim_matches('"')
            .to_string()
    })
}

fn linux_window_exists(handle: LinuxWindowIdentityHandle) -> bool {
    if handle.0 == 0 {
        return false;
    }
    linux_command_line("xdotool", &["getwindowname", &handle.0.to_string()]).is_some()
}

fn linux_window_pid(handle: LinuxWindowIdentityHandle) -> Option<u32> {
    if handle.0 == 0 {
        return None;
    }
    linux_command_line("xdotool", &["getwindowpid", &handle.0.to_string()])
        .and_then(|line| line.parse::<u32>().ok())
}

fn linux_activate_window(handle: LinuxWindowIdentityHandle) -> bool {
    if handle.0 == 0 {
        return false;
    }
    linux_command_line(
        "xdotool",
        &["windowactivate", "--sync", &handle.0.to_string()],
    )
    .is_some()
}

fn linux_send_ctrl_v(handle: LinuxWindowIdentityHandle) -> Option<bool> {
    if handle.0 == 0 {
        return None;
    }
    linux_command_line(
        "xdotool",
        &[
            "key",
            "--window",
            &handle.0.to_string(),
            "--clearmodifiers",
            "ctrl+v",
        ],
    )
    .map(|_| true)
}

fn linux_parse_window_id(value: &str) -> Option<LinuxWindowIdentityHandle> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(hex) = trimmed.strip_prefix("0x") {
        u64::from_str_radix(hex, 16).ok()
    } else {
        trimmed.parse::<u64>().ok()
    }
    .map(LinuxWindowIdentityHandle)
}

#[cfg(all(target_os = "linux", not(test)))]
fn linux_command_line(program: &str, args: &[&str]) -> Option<String> {
    let output = ProcessCommand::new(program).args(args).output().ok()?;
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

#[cfg(not(all(target_os = "linux", not(test))))]
fn linux_command_line(_program: &str, _args: &[&str]) -> Option<String> {
    None
}

impl NativeTextCaretHost for LinuxTextCaretHost {
    type Handle = LinuxTextCaretHandle;

    fn accessible_caret_anchor(&mut self, focus: Self::Handle) -> Option<NativeTextCaretAnchor> {
        self.actions
            .borrow_mut()
            .push(LinuxTextCaretAction::QueryAccessible(focus));
        *self.next_accessible.borrow()
    }

    fn thread_caret_anchor(&mut self, target: Self::Handle) -> Option<NativeTextCaretAnchor> {
        self.actions
            .borrow_mut()
            .push(LinuxTextCaretAction::QueryThread(target));
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
            .push(LinuxTextCaretAction::QueryFocusRect {
                focus,
                max_width,
                max_height,
            });
        *self.next_focus_rect.borrow()
    }

    fn cursor_anchor(&mut self) -> Option<NativeTextCaretAnchor> {
        self.actions
            .borrow_mut()
            .push(LinuxTextCaretAction::QueryCursor);
        *self.next_cursor.borrow()
    }

    fn focus_handle_for_target(&mut self, target: Self::Handle) -> Self::Handle {
        self.actions
            .borrow_mut()
            .push(LinuxTextCaretAction::ResolveFocus(target));
        self.next_focus_handle.borrow().unwrap_or(target)
    }
}

impl NativeFileDialogHost for LinuxFileDialogHost {
    fn pick_file(&self, request: NativeFileDialogRequest<'_>) -> Result<Option<String>, String> {
        self.requests.borrow_mut().push(LinuxFileDialogRecord {
            title: request.title.to_string(),
            filter_name: request.filter_name.to_string(),
            filter_pattern: request.filter_pattern.to_string(),
            current_path: request.current_path.to_string(),
        });
        #[cfg(all(target_os = "linux", not(test)))]
        {
            return Self::system_pick_file(request);
        }
        #[cfg(not(all(target_os = "linux", not(test))))]
        self.next_result.borrow().clone()
    }
}

impl NativeTextInputDialogHost for LinuxTextInputDialogHost {
    type Owner = LinuxMainWindowHandle;

    fn prompt_text(
        &self,
        owner: Self::Owner,
        request: NativeTextInputDialogRequest<'_>,
    ) -> Option<String> {
        self.requests.borrow_mut().push(LinuxTextInputDialogRecord {
            owner,
            title: request.title.to_string(),
            label: request.label.to_string(),
            initial: request.initial.to_string(),
        });
        #[cfg(all(target_os = "linux", not(test)))]
        {
            return Self::system_prompt_text(request);
        }
        #[cfg(not(all(target_os = "linux", not(test))))]
        self.next_result.borrow().clone()
    }
}

impl NativeEditTextDialogHost for LinuxEditTextDialogHost {
    type Owner = LinuxMainWindowHandle;

    fn open_edit_text(
        &self,
        owner: Self::Owner,
        request: NativeEditTextDialogRequest<'_>,
        save_handler: &mut dyn NativeEditTextSaveHandler,
    ) -> NativeEditTextDialogResult {
        self.requests.borrow_mut().push(LinuxEditTextDialogRecord {
            owner,
            title: request.title.to_string(),
            initial_text: request.initial_text.to_string(),
            initial_size: request.initial_size,
        });
        #[cfg(all(target_os = "linux", not(test)))]
        {
            return Self::system_open_edit_text(request, save_handler);
        }
        #[cfg(not(all(target_os = "linux", not(test))))]
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

impl NativeMailMergeWindowHost for LinuxMailMergeWindowHost {
    type Owner = LinuxMainWindowHandle;

    fn open_mail_merge(&self, owner: Self::Owner, request: NativeMailMergeWindowRequest<'_>) {
        self.requests.borrow_mut().push(LinuxMailMergeWindowRecord {
            owner,
            initial_excel_path: request.initial_excel_path.map(str::to_string),
        });
    }
}

impl Renderer for LinuxRenderer {
    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.commands
            .push(LinuxRenderCommand::FillRect(rect, color));
    }

    fn stroke_rect(&mut self, rect: Rect, color: Color, width: i32) {
        self.commands
            .push(LinuxRenderCommand::StrokeRect(rect, color, width));
    }

    fn draw_text(&mut self, run: &TextRun, style: &TextStyle) {
        self.commands
            .push(LinuxRenderCommand::DrawText(run.clone(), style.clone()));
    }

    fn push_clip(&mut self, rect: Rect) {
        self.commands.push(LinuxRenderCommand::PushClip(rect));
    }

    fn pop_clip(&mut self) {
        self.commands.push(LinuxRenderCommand::PopClip);
    }
}

impl NativeMainWindowHost for LinuxMainWindowHost {
    type Handle = LinuxMainWindowHandle;
    type AppIcon = LinuxAppIconHandle;

    fn create_main_windows(
        &mut self,
        request: NativeMainWindowRequest,
    ) -> NativeMainWindowPresentation<Self::Handle> {
        self.create_requests.push(request);
        let handles = NativeMainWindowHandles {
            main: LinuxMainWindowHandle(self.next_handle),
            quick: LinuxMainWindowHandle(self.next_handle + 1),
        };
        self.next_handle = self.next_handle.saturating_add(2);
        self.created = Some(handles);
        NativeMainWindowPresentation::Created(handles)
    }

    fn apply_main_window_appearance(&mut self, handle: Self::Handle) {
        self.appearance_applied.push(handle);
    }

    fn set_main_window_app_icon(
        &mut self,
        handle: Self::Handle,
        icon: NativeAppIconResource<Self::AppIcon>,
    ) {
        self.icon_updates.push((handle, icon));
    }

    fn hide_main_window(&mut self, handle: Self::Handle) {
        self.hidden.push(handle);
    }

    fn present_main_window(&mut self, handle: Self::Handle, mode: NativeMainWindowPresentMode) {
        self.presented.push((handle, mode));
    }

    fn set_main_window_bounds(&mut self, handle: Self::Handle, bounds: UiRect) {
        self.bounds_updates.push((handle, bounds));
    }

    fn activate_main_window(&mut self, handle: Self::Handle) {
        self.presented
            .push((handle, NativeMainWindowPresentMode::ActivateAndFocus));
    }

    fn foreground_main_window(&mut self, handle: Self::Handle) {
        self.presented
            .push((handle, NativeMainWindowPresentMode::ActivateAndFocus));
    }

    fn restore_main_window(&mut self, handle: Self::Handle) {
        self.presented
            .push((handle, NativeMainWindowPresentMode::ActivateAndFocus));
    }

    fn close_main_window(&mut self, handle: Self::Handle) {
        self.close_requests.push(handle);
    }

    fn set_main_window_activation_policy(&mut self, handle: Self::Handle, allow_activation: bool) {
        self.activation_policy.push((handle, allow_activation));
    }

    fn request_main_window_close(&mut self, handle: Self::Handle) {
        self.close_requests.push(handle);
    }

    fn destroy_main_window(&mut self, handle: Self::Handle) {
        self.destroyed.push(handle);
    }

    fn capture_main_pointer(&mut self, _handle: Self::Handle) {
        self.pointer_captures = self.pointer_captures.saturating_add(1);
    }

    fn release_main_pointer(&mut self, _handle: Self::Handle) {
        self.pointer_releases = self.pointer_releases.saturating_add(1);
    }

    fn begin_main_window_drag(&mut self, _handle: Self::Handle) {
        self.drag_count = self.drag_count.saturating_add(1);
    }

    fn track_main_pointer_leave(&mut self, _handle: Self::Handle) -> bool {
        true
    }

    fn request_main_window_area_repaint(
        &mut self,
        handle: Self::Handle,
        area: Option<UiRect>,
        erase: bool,
    ) -> bool {
        self.repaint_requests.push((handle, area, erase));
        true
    }

    fn main_window_layout_dpi(&mut self, _handle: Self::Handle) -> u32 {
        96
    }

    fn main_window_client_bounds(&mut self, _handle: Self::Handle) -> Option<UiRect> {
        Some(UiRect::new(0, 0, 760, 520))
    }

    fn main_window_bounds(&mut self, _handle: Self::Handle) -> Option<UiRect> {
        Some(UiRect::new(0, 0, 760, 520))
    }
}

impl NativeMainSearchControlHost for LinuxMainSearchControlHost {
    type Owner = LinuxMainWindowHandle;
    type Handle = LinuxMainSearchHandle;
    type StyleResource = LinuxSearchStyleResource;

    fn create_search_control(
        &mut self,
        request: NativeMainSearchControlRequest<Self::Owner>,
    ) -> NativeMainSearchControlPresentation<Self::Handle> {
        let handle = LinuxMainSearchHandle(self.next_handle);
        self.next_handle = self.next_handle.saturating_add(1);
        self.searches.push(LinuxMainSearchRecord {
            handle,
            text: String::new(),
            visible: request.visible,
            bounds: request.bounds,
            request,
            focused: false,
        });
        NativeMainSearchControlPresentation::Created(handle)
    }

    fn apply_search_style(
        &mut self,
        request: NativeMainSearchStyleRequest<Self::Handle, Self::StyleResource>,
    ) -> NativeMainSearchStylePresentation<Self::StyleResource> {
        self.style_requests.push(request);
        let resource = LinuxSearchStyleResource(self.next_style_resource);
        self.next_style_resource = self.next_style_resource.saturating_add(1);
        NativeMainSearchStylePresentation::Applied(Some(resource))
    }

    fn release_search_style_resource(&mut self, resource: Self::StyleResource) {
        self.released_style_resources.push(resource);
    }

    fn set_search_bounds(&mut self, handle: Self::Handle, bounds: UiRect) {
        if let Some(search) = self.search_mut(handle) {
            search.bounds = bounds;
        }
    }

    fn set_search_visible(&mut self, handle: Self::Handle, visible: bool) {
        if let Some(search) = self.search_mut(handle) {
            search.visible = visible;
        }
    }

    fn search_text(&self, handle: Self::Handle) -> String {
        self.search(handle)
            .map(|search| search.text.clone())
            .unwrap_or_default()
    }

    fn set_search_text(&mut self, handle: Self::Handle, text: &str) {
        if let Some(search) = self.search_mut(handle) {
            search.text = text.to_string();
        }
    }

    fn focus_search(&mut self, handle: Self::Handle) {
        if let Some(search) = self.search_mut(handle) {
            search.focused = true;
        }
    }
}

impl NativeSettingsControlHost for LinuxSettingsControlHost {
    type Handle = LinuxSettingsControlHandle;

    fn create_control(&mut self, spec: &SettingsControlSpec) -> Self::Handle {
        let handle = LinuxSettingsControlHandle(self.next_handle);
        self.next_handle = self.next_handle.saturating_add(1);
        self.controls.push(LinuxSettingsControlRecord {
            handle,
            spec: spec.clone(),
            visible: true,
            enabled: true,
            destroyed: false,
            repaint_count: 0,
        });
        handle
    }

    fn destroy_control(&mut self, handle: Self::Handle) {
        if let Some(control) = self.control_mut(handle) {
            control.destroyed = true;
        }
    }

    fn control_exists(&self, handle: Self::Handle) -> bool {
        self.control(handle).is_some()
    }

    fn set_control_visible(&mut self, handle: Self::Handle, visible: bool) {
        if let Some(control) = self.control_mut(handle) {
            control.visible = visible;
        }
    }

    fn set_control_enabled(&mut self, handle: Self::Handle, enabled: bool) {
        if let Some(control) = self.control_mut(handle) {
            control.enabled = enabled;
        }
    }

    fn set_control_bounds(&mut self, handle: Self::Handle, bounds: UiRect) {
        if let Some(control) = self.control_mut(handle) {
            control.spec.bounds = bounds;
        }
    }

    fn control_at_point(&self, point: Point) -> Option<Self::Handle> {
        self.controls
            .iter()
            .find(|control| {
                !control.destroyed
                    && control.visible
                    && control.enabled
                    && control.spec.bounds.contains(point.x, point.y)
            })
            .map(|control| control.handle)
    }

    fn control_screen_bounds(&self, handle: Self::Handle) -> Option<UiRect> {
        self.control(handle).map(|control| control.spec.bounds)
    }

    fn control_text(&self, handle: Self::Handle) -> String {
        self.control(handle)
            .map(|control| control.spec.text.clone())
            .unwrap_or_default()
    }

    fn set_control_text(&mut self, handle: Self::Handle, text: &str) {
        if let Some(control) = self.control_mut(handle) {
            control.spec.text = text.to_string();
        }
    }

    fn request_control_repaint(&mut self, handle: Self::Handle) -> bool {
        if let Some(control) = self.control_mut(handle) {
            control.repaint_count = control.repaint_count.saturating_add(1);
            true
        } else {
            false
        }
    }
}

impl NativeSettingsDropdownHost for LinuxSettingsDropdownHost {
    type Handle = LinuxSettingsDropdownHandle;
    type Owner = LinuxMainWindowHandle;

    fn present_settings_dropdown(
        &mut self,
        request: NativeSettingsDropdownRequest<Self::Owner>,
    ) -> NativeSettingsDropdownPresentation<Self::Handle> {
        let handle = LinuxSettingsDropdownHandle(self.next_handle);
        self.next_handle = self.next_handle.saturating_add(1);
        self.dropdowns.push(LinuxSettingsDropdownRecord {
            handle,
            request,
            destroyed: false,
        });
        NativeSettingsDropdownPresentation::Created(handle)
    }

    fn destroy_settings_dropdown(&mut self, handle: Self::Handle) {
        if let Some(dropdown) = self.dropdown_mut(handle) {
            dropdown.destroyed = true;
        }
    }

    fn settings_dropdown_bounds(&self, handle: Self::Handle) -> Option<UiRect> {
        self.dropdown(handle).map(|dropdown| {
            let mut bounds = dropdown.request.anchor;
            bounds.bottom = bounds.top + 32 + (dropdown.request.items.len() as i32 * 28);
            bounds.right = bounds.left + dropdown.request.width.max(bounds.width());
            bounds
        })
    }
}

impl NativeSettingsWindowHost for LinuxSettingsWindowHost {
    type Handle = LinuxMainWindowHandle;

    fn present_settings_window(
        &mut self,
        request: NativeSettingsWindowRequest<Self::Handle>,
    ) -> NativeSettingsWindowPresentation<Self::Handle> {
        self.present_requests.push(request);
        if let Some(existing) = request.existing {
            self.presented = Some(existing);
            return NativeSettingsWindowPresentation::FocusedExisting(existing);
        }
        let handle = LinuxMainWindowHandle(self.next_handle);
        self.next_handle = self.next_handle.saturating_add(1);
        self.presented = Some(handle);
        NativeSettingsWindowPresentation::Created(handle)
    }

    fn set_settings_window_bounds(&mut self, handle: Self::Handle, bounds: UiRect) {
        self.bounds_updates.push((handle, bounds));
    }

    fn destroy_settings_window(&mut self, handle: Self::Handle) {
        self.destroyed.push(handle);
        if self.presented == Some(handle) {
            self.presented = None;
        }
    }

    fn focus_settings_window(&mut self, handle: Self::Handle) {
        self.focused.push(handle);
    }

    fn track_settings_pointer_leave(&mut self, _handle: Self::Handle) -> bool {
        self.pointer_leave_tracks = self.pointer_leave_tracks.saturating_add(1);
        true
    }

    fn capture_settings_pointer(&mut self, _handle: Self::Handle) {
        self.pointer_captures = self.pointer_captures.saturating_add(1);
    }

    fn release_settings_pointer(&mut self, _handle: Self::Handle) {
        self.pointer_releases = self.pointer_releases.saturating_add(1);
    }

    fn request_settings_window_repaint(&mut self, handle: Self::Handle) -> bool {
        self.repaint_requests.push((handle, None, true));
        true
    }

    fn request_settings_window_area_repaint(
        &mut self,
        handle: Self::Handle,
        area: Option<UiRect>,
        erase: bool,
    ) -> bool {
        self.repaint_requests.push((handle, area, erase));
        true
    }

    fn settings_window_layout_dpi(&mut self, _handle: Self::Handle) -> u32 {
        96
    }

    fn settings_window_client_to_screen(
        &mut self,
        _handle: Self::Handle,
        point: Point,
    ) -> Option<Point> {
        Some(point)
    }

    fn settings_window_client_bounds(&mut self, _handle: Self::Handle) -> Option<UiRect> {
        Some(UiRect::new(0, 0, 760, 520))
    }

    fn settings_window_bounds(&mut self, _handle: Self::Handle) -> Option<UiRect> {
        Some(UiRect::new(80, 80, 840, 600))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_core::{command_ids, CommandPayload, CommandScope};

    fn linux_clipboard_test_guard() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .expect("Linux clipboard test lock poisoned")
    }

    fn linux_settings_file_test_guard() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .expect("Linux settings file test lock poisoned")
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
    fn linux_native_host_launch_plan_targets_real_gtk_entry() {
        let plan = linux_native_host_launch_plan();

        assert_eq!(plan.platform_name(), "linux");
        assert_eq!(plan.toolkit_name(), "gtk4_libadwaita");
        assert_eq!(plan.entry_point, "linux_app::run");
        assert_eq!(plan.native_application_type, "gtk4::Application");
        assert_eq!(plan.native_window_type, "gtk4::ApplicationWindow");
        assert_eq!(plan.real_host_module_path, "src/linux_native_host.rs");
        assert!(plan.needs_target_os_verification());
        if cfg!(target_os = "linux") {
            assert_eq!(plan.mode_name(), "real_native_host");
            assert!(plan.enters_real_event_loop());
        } else {
            assert_eq!(plan.mode_name(), "contract_scaffold_fallback");
            assert!(!plan.enters_real_event_loop());
            assert!(
                crate::linux_native_host::run_real_gtk_host(linux_host_contract_summary()).is_err()
            );
        }
    }

    #[test]
    fn linux_autostart_host_writes_xdg_desktop_entry() {
        let root = std::env::temp_dir().join(format!(
            "zsclip-linux-autostart-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let executable = root.join("ZSClip App").join("zsclip");
        let autostart_dir = root.join(".config").join("autostart");
        let mut application = LinuxApplicationModel::default();
        application.autostart_host =
            LinuxAutostartHost::with_paths(autostart_dir.clone(), executable.clone());

        let enabled = application.apply_autostart(true);
        assert!(enabled.applied, "{enabled:?}");
        assert!(application.autostart_status().enabled);
        let entry = std::fs::read_to_string(autostart_dir.join("zsclip.desktop")).unwrap();
        assert!(entry.contains("[Desktop Entry]"));
        assert!(entry.contains("Name=ZSClip"));
        assert!(entry.contains("X-ZSClip-Autostart=true"));
        assert!(entry.contains(&format!(
            "Exec={}",
            linux_desktop_exec_escape(&executable.to_string_lossy())
        )));

        let disabled = application.apply_autostart(false);
        assert!(disabled.applied, "{disabled:?}");
        assert!(!application.autostart_status().enabled);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn linux_native_host_actions_enter_product_command_routes() {
        let settings = dispatch_linux_native_host_action(NativeHostUiAction::OpenSettings);
        assert!(settings.accepted);
        assert_eq!(settings.result_name, "zsclip.window.open_settings");

        let hide =
            crate::linux_native_host::dispatch_gtk_host_action(NativeHostUiAction::HideWindow);
        assert!(hide.accepted);
        assert_eq!(hide.result_name, "zsclip.window.hide");
    }

    #[test]
    fn linux_native_settings_actions_enter_product_command_routes() {
        let save = dispatch_linux_native_settings_action(NativeHostSettingsAction::Save);
        assert!(save.accepted);
        assert_eq!(save.result_name, "zsclip.settings.save");

        let config = crate::linux_native_host::dispatch_gtk_settings_action(
            NativeHostSettingsAction::OpenConfig,
        );
        assert!(config.accepted);
        assert_eq!(config.result_name, "zsclip.settings.open_config");

        let close =
            crate::linux_native_host::dispatch_gtk_settings_action(NativeHostSettingsAction::Close);
        assert!(close.accepted);
        assert_eq!(close.result_name, "zsclip.settings.close");
    }

    #[test]
    fn linux_native_settings_route_actions_report_shared_matrix_status() {
        let sync = dispatch_linux_native_settings_route_action("settings_sync", "sync_webdav_now");
        assert!(!sync.accepted);
        assert!(sync
            .result_name
            .starts_with("zsclip.settings_sync.webdav.failed."));

        let lan =
            dispatch_linux_native_settings_route_action("settings_sync", "refresh_lan_devices");
        assert!(lan.accepted);
        assert_eq!(
            lan.result_name,
            "zsclip.settings_sync.refresh_lan_devices.device_book_projected_0_on_linux_native_host"
        );

        let lan_pair =
            dispatch_linux_native_settings_route_action("settings_sync", "pair_lan_device");
        assert!(!lan_pair.accepted);
        assert_eq!(
            lan_pair.result_name,
            "zsclip.settings_sync.pair_lan_device.lan_disabled_on_linux_native_host"
        );

        let lan_pair_link =
            dispatch_linux_native_settings_route_action("settings_sync", "copy_lan_pair_url");
        assert!(!lan_pair_link.accepted);
        assert_eq!(
            lan_pair_link.result_name,
            "zsclip.settings_sync.copy_lan_pair_url.lan_disabled_on_linux_native_host"
        );

        let accept_pair =
            dispatch_linux_native_settings_route_action("settings_sync", "accept_lan_pairing");
        assert!(!accept_pair.accepted);
        assert_eq!(
            accept_pair.result_name,
            "zsclip.settings_sync.accept_lan_pairing.no_pending_pair_on_linux_native_host"
        );

        let reject_pair =
            dispatch_linux_native_settings_route_action("settings_sync", "reject_lan_pairing");
        assert!(!reject_pair.accepted);
        assert_eq!(
            reject_pair.result_name,
            "zsclip.settings_sync.reject_lan_pairing.no_pending_pair_on_linux_native_host"
        );

        let disable_history = dispatch_linux_native_settings_route_action(
            "settings_platform",
            "disable_system_clipboard_history",
        );
        assert!(disable_history.accepted);
        assert_eq!(
            disable_history.result_name,
            "zsclip.settings_platform.disable_system_clipboard_history.not_applicable_on_linux_native_host"
        );

        let enable_history = dispatch_linux_native_settings_route_action(
            "settings_platform",
            "enable_system_clipboard_history",
        );
        assert!(enable_history.accepted);
        assert_eq!(
            enable_history.result_name,
            "zsclip.settings_platform.enable_system_clipboard_history.not_applicable_on_linux_native_host"
        );

        let restart_shell = dispatch_linux_native_settings_route_action(
            "settings_platform",
            "restart_system_shell",
        );
        assert!(restart_shell.accepted);
        assert_eq!(
            restart_shell.result_name,
            "zsclip.settings_platform.restart_system_shell.not_required_on_linux_native_host"
        );

        let missing = dispatch_linux_native_settings_route_action("settings_sync", "missing");
        assert!(!missing.accepted);
        assert_eq!(
            missing.result_name,
            "zsclip.settings.unknown_route.settings_sync.missing"
        );
    }

    #[test]
    fn linux_gtk_settings_save_collects_all_native_bindings() {
        let source = include_str!("linux_native_host.rs").replace("\r\n", "\n");

        assert!(source.contains("NativeSettingsEntryBinding"));
        assert!(source.contains("NativeSettingsToggleBinding"));
        assert!(source.contains("NativeSettingsDropdownBinding"));
        assert!(source.contains("NativeSettingsControlBindings"));
        assert!(source.contains("Switch::new()"));
        assert!(source.contains("DropDown::from_strings(&labels)"));
        assert!(source.contains("Notebook::new()"));
        assert!(source.contains("native_host_settings_page_tab_specs()"));
        assert!(source.contains("initial_value"));
        assert!(source.contains("switch.set_active(initial_value)"));
        assert!(source.contains("settings_native_dropdown_options"));
        assert!(source.contains("settings_native_vv_group_dropdown_options"));
        assert!(source.contains("settings_native_vv_source_tab(settings_json)"));
        assert!(source.contains("crate::db_runtime::native_clip_groups(category)"));
        assert!(source.contains("raw_values: options"));
        assert!(source.contains("dropdown.set_selected(options.selected_index as u32)"));
        assert!(source.contains("binding.entry.text().to_string()"));
        assert!(source.contains("binding.switch.is_active()"));
        assert!(source.contains("binding.dropdown.selected() as usize"));
        assert!(source.contains("SettingsNativeSubmittedControlValue"));
        assert!(source.contains("settings_native_collect_submission(\n"));
        assert!(source.contains("refresh_group_popup_menus_for_category(0"));
    }

    #[test]
    fn linux_native_settings_save_routes_auto_start_to_autostart_host() {
        let source = include_str!("linux_app.rs").replace("\r\n", "\n");

        assert!(
            source.contains("linux_apply_autostart_from_settings_updates(&applied.field_updates)")
        );
        assert!(source.contains("settings_native_bool_field_update(updates, \"auto_start\")"));
        assert!(source.contains("zsclip.settings.native_save.updates_{}.rejected_{}.autostart_{}"));

        if !cfg!(target_os = "linux") {
            let updates = [crate::settings_model::SettingsNativeJsonFieldUpdate {
                field_name: "auto_start".to_string(),
                value: serde_json::Value::Bool(true),
            }];
            let applied = linux_apply_autostart_from_settings_updates(&updates).unwrap();

            assert!(applied.applied);
            assert!(applied.status.enabled);
            assert_eq!(
                applied.status.registration_path.as_deref(),
                Some("linux_autostart_scaffold")
            );
        }
    }

    #[test]
    fn linux_native_settings_save_persists_submission_to_json_file() {
        let _guard = linux_settings_file_test_guard();
        let path = native_settings_temp_file("linux-settings-save");
        set_linux_native_settings_file_for_tests(Some(path.clone()));

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
        let result = persist_linux_native_settings_submission(&submission);
        assert!(result.accepted, "{result:?}");
        assert!(result
            .result_name
            .starts_with("zsclip.settings.native_save.updates_2.rejected_0"));

        let snapshot = linux_native_settings_json_snapshot();
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

        set_linux_native_settings_file_for_tests(None);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn linux_native_clipboard_capture_enabled_reads_saved_setting() {
        let _guard = linux_settings_file_test_guard();
        let path = native_settings_temp_file("linux-capture-enabled");
        set_linux_native_settings_file_for_tests(Some(path.clone()));

        assert!(linux_native_clipboard_capture_enabled());
        std::fs::write(
            &path,
            serde_json::json!({ "clipboard_capture_enabled": false }).to_string(),
        )
        .unwrap();
        assert!(!linux_native_clipboard_capture_enabled());

        set_linux_native_settings_file_for_tests(None);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn linux_native_status_menu_action_state_reads_saved_settings() {
        let _guard = linux_settings_file_test_guard();
        let path = native_settings_temp_file("linux-status-action-state");
        set_linux_native_settings_file_for_tests(Some(path.clone()));

        assert_eq!(
            linux_native_status_menu_action_state(NativeHostStatusMenuAction::ToggleWindow),
            None
        );
        assert_eq!(
            linux_native_status_menu_action_state(
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
            linux_native_status_menu_action_state(
                NativeHostStatusMenuAction::ToggleClipboardCapture
            ),
            Some(false)
        );

        set_linux_native_settings_file_for_tests(None);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn linux_native_grouping_enabled_reads_saved_setting() {
        let _guard = linux_settings_file_test_guard();
        let path = native_settings_temp_file("linux-grouping-enabled");
        set_linux_native_settings_file_for_tests(Some(path.clone()));

        assert!(linux_native_grouping_enabled());
        std::fs::write(
            &path,
            serde_json::json!({ "grouping_enabled": false }).to_string(),
        )
        .unwrap();
        assert!(!linux_native_grouping_enabled());

        set_linux_native_settings_file_for_tests(None);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn linux_native_settings_control_actions_enter_product_command_routes() {
        if !cfg!(target_os = "linux") {
            let autostart = dispatch_linux_native_settings_control_action(
                NativeHostSettingsControlAction::ToggleAutostart,
            );
            assert!(autostart.accepted);
            assert_eq!(
                autostart.result_name,
                "zsclip.settings.toggle_autostart_scaffold"
            );
        }

        let capture = dispatch_linux_native_settings_control_action(
            NativeHostSettingsControlAction::ToggleClipboardCapture,
        );
        assert!(capture.accepted);
        assert_eq!(capture.result_name, "zsclip.settings.toggle_control");

        #[cfg(feature = "lan-sync")]
        {
            let lan = crate::linux_native_host::dispatch_gtk_settings_control_action(
                NativeHostSettingsControlAction::ToggleLanSync,
            );
            assert!(lan.accepted);
            assert_eq!(lan.result_name, "zsclip.settings.toggle_control");
        }

        #[cfg(feature = "cloud-sync")]
        {
            let cloud = crate::linux_native_host::dispatch_gtk_settings_control_action(
                NativeHostSettingsControlAction::ToggleCloudSync,
            );
            assert!(cloud.accepted);
            assert_eq!(cloud.result_name, "zsclip.settings.toggle_control");
        }

        #[cfg(any(feature = "cloud-sync", feature = "lan-sync"))]
        {
            let dropdown = crate::linux_native_host::dispatch_gtk_settings_control_action(
                NativeHostSettingsControlAction::OpenSyncModeDropdown,
            );
            assert!(dropdown.accepted);
            assert_eq!(dropdown.result_name, "zsclip.settings.open_dropdown");
        }
    }

    #[test]
    fn linux_native_file_picker_is_target_only_and_callable() {
        if cfg!(target_os = "linux") {
            return;
        }

        let result = pick_linux_native_file(NativeFileDialogRequest {
            title: "Choose sound",
            filter_name: "Wave Files",
            filter_pattern: "*.wav",
            current_path: "/tmp/old.wav",
        });

        assert_eq!(
            result.unwrap_err(),
            "Linux native file picker is only available on Linux"
        );
    }

    #[test]
    fn linux_native_dialog_actions_enter_native_dialog_routes() {
        let info = dispatch_linux_native_dialog_action(NativeHostDialogAction::ShowInfoMessage);
        assert!(info.accepted);
        assert_eq!(info.result_name, "zsclip.dialog.show_info_message");

        let confirm = crate::linux_native_host::dispatch_gtk_dialog_action(
            NativeHostDialogAction::ConfirmQuestion,
        );
        assert!(confirm.accepted);
        assert_eq!(confirm.result_name, "zsclip.dialog.confirm_cancel");
    }

    #[test]
    fn linux_native_status_menu_actions_enter_product_command_routes() {
        let _guard = linux_settings_file_test_guard();
        let path = native_settings_temp_file("linux-status-menu-settings");
        set_linux_native_settings_file_for_tests(Some(path.clone()));

        let host_source = include_str!("linux_native_host.rs").replace("\r\n", "\n");
        assert!(host_source.contains("ZsclipGtkStatusNotifier"));
        assert!(host_source.contains("impl ksni::Tray for ZsclipGtkStatusNotifier"));
        assert!(host_source.contains("install_status_notifier(app, &status, &window)"));
        assert!(host_source.contains("native_host_status_menu_item_specs()"));
        assert!(!host_source.contains("NativeComponentAction::StatusMenu(action)"));
        assert!(host_source.contains("items.push(ksni::MenuItem::Separator)"));
        assert!(host_source.contains("icon_name: spec.icon_name.to_string()"));
        assert!(
            host_source.contains("gio::MenuItem::new(Some(spec.label), Some(&detailed_action))")
        );
        assert!(host_source.contains("gio::ThemedIcon::new(spec.icon_name)"));
        assert!(host_source.contains("menu.append_section(None, &section)"));
        assert!(host_source.contains("dispatch_linux_native_status_menu_action(action)"));
        assert!(host_source.contains("linux_native_status_menu_action_state(action)"));
        assert!(host_source.contains("gio::SimpleAction::new_stateful"));
        assert!(host_source.contains("simple_action.set_state(&enabled.to_variant())"));
        assert!(host_source.contains("refresh_gtk_status_action_states(&app)"));
        assert!(host_source.contains("assume_sni_available(true)"));

        let toggle =
            dispatch_linux_native_status_menu_action(NativeHostStatusMenuAction::ToggleWindow);
        assert!(toggle.accepted);
        assert_eq!(toggle.result_name, "zsclip.tray.toggle_window");

        assert!(linux_native_clipboard_capture_enabled());
        let capture = crate::linux_native_host::dispatch_gtk_status_menu_action(
            NativeHostStatusMenuAction::ToggleClipboardCapture,
        );
        assert!(capture.accepted);
        assert_eq!(capture.result_name, "zsclip.tray.toggle_clipboard_capture");
        assert!(!linux_native_clipboard_capture_enabled());
        assert_eq!(
            linux_native_status_menu_action_state(
                NativeHostStatusMenuAction::ToggleClipboardCapture
            ),
            Some(false)
        );
        let capture = dispatch_linux_native_status_menu_action(
            NativeHostStatusMenuAction::ToggleClipboardCapture,
        );
        assert!(capture.accepted);
        assert_eq!(capture.result_name, "zsclip.tray.toggle_clipboard_capture");
        assert!(linux_native_clipboard_capture_enabled());
        assert_eq!(
            linux_native_status_menu_action_state(
                NativeHostStatusMenuAction::ToggleClipboardCapture
            ),
            Some(true)
        );

        #[cfg(feature = "lan-sync")]
        {
            assert_eq!(
                linux_native_settings_json_snapshot()
                    .get("lan_sync_enabled")
                    .and_then(serde_json::Value::as_bool),
                None
            );
            let lan = crate::linux_native_host::dispatch_gtk_status_menu_action(
                NativeHostStatusMenuAction::ToggleLanSync,
            );
            assert!(lan.accepted);
            assert_eq!(lan.result_name, "zsclip.tray.toggle_lan_sync");
            assert_eq!(
                linux_native_settings_json_snapshot()
                    .get("lan_sync_enabled")
                    .and_then(serde_json::Value::as_bool),
                Some(true)
            );
        }

        let exit = crate::linux_native_host::dispatch_gtk_status_menu_action(
            NativeHostStatusMenuAction::Exit,
        );
        assert!(exit.accepted);
        assert_eq!(exit.result_name, "zsclip.tray.exit");

        set_linux_native_settings_file_for_tests(None);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn linux_native_row_actions_enter_product_command_routes() {
        let paste = dispatch_linux_native_row_action(NativeHostRowAction::Paste);
        assert!(paste.accepted);
        assert_eq!(paste.result_name, "zsclip.row.paste");

        let copy = crate::linux_native_host::dispatch_gtk_row_action(NativeHostRowAction::Copy);
        assert!(copy.accepted);
        assert_eq!(copy.result_name, "zsclip.row.copy");

        let pin = crate::linux_native_host::dispatch_gtk_row_action(NativeHostRowAction::Pin);
        assert!(pin.accepted);
        assert_eq!(pin.result_name, "zsclip.row.toggle_pin");

        let phrase =
            crate::linux_native_host::dispatch_gtk_row_action(NativeHostRowAction::ToPhrase);
        assert!(phrase.accepted);
        assert_eq!(phrase.result_name, "zsclip.row.to_phrase");

        let delete = crate::linux_native_host::dispatch_gtk_row_action(NativeHostRowAction::Delete);
        assert!(delete.accepted);
        assert_eq!(delete.result_name, "zsclip.row.delete");

        let edit = crate::linux_native_host::dispatch_gtk_row_action(NativeHostRowAction::Edit);
        assert!(edit.accepted);
        assert_eq!(edit.result_name, "zsclip.row.edit");

        let open_path =
            crate::linux_native_host::dispatch_gtk_row_action(NativeHostRowAction::OpenPath);
        assert!(open_path.accepted);
        assert_eq!(open_path.result_name, "zsclip.row.open_path");

        let open_folder =
            crate::linux_native_host::dispatch_gtk_row_action(NativeHostRowAction::OpenFolder);
        assert!(open_folder.accepted);
        assert_eq!(open_folder.result_name, "zsclip.row.open_folder");

        let copy_path =
            crate::linux_native_host::dispatch_gtk_row_action(NativeHostRowAction::CopyPath);
        assert!(copy_path.accepted);
        assert_eq!(copy_path.result_name, "zsclip.row.copy_path");

        #[cfg(feature = "ai-actions")]
        {
            let translate = crate::linux_native_host::dispatch_gtk_row_action(
                NativeHostRowAction::TextTranslate,
            );
            assert!(translate.accepted);
            assert_eq!(translate.result_name, "zsclip.row.text_translate");
        }
    }

    #[test]
    fn linux_native_row_actions_execute_real_item_payloads() -> rusqlite::Result<()> {
        crate::db_runtime::with_test_db(|| {
            let text_id = crate::db_runtime::with_db_mut(|conn| {
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, text_data, source_app) VALUES(0, 'text', 'phrase me', 'linux-row-phrase', 'phrase me full text', 'Terminal')",
                    [],
                )?;
                Ok(conn.last_insert_rowid())
            })?;
            let phrase =
                dispatch_linux_native_row_action_for_item(NativeHostRowAction::ToPhrase, text_id);
            assert!(phrase.accepted);
            assert_eq!(phrase.result_name, "zsclip.row.to_phrase_db");
            let phrases = crate::db_runtime::native_clip_list_items(1, 10)?;
            assert_eq!(phrases.len(), 1);
            assert_eq!(phrases[0].kind, crate::app_core::ClipKind::Phrase);
            assert_eq!(
                crate::db_runtime::item_text(phrases[0].id)?.as_deref(),
                Some("phrase me full text")
            );

            let file_id = crate::db_runtime::with_db_mut(|conn| {
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, text_data, file_paths, source_app) VALUES(0, 'files', '/tmp/zsclip-linux.txt', 'linux-row-file', '/tmp/zsclip-linux.txt', '/tmp/zsclip-linux.txt', 'Files')",
                    [],
                )?;
                Ok(conn.last_insert_rowid())
            })?;
            let open =
                dispatch_linux_native_row_action_for_item(NativeHostRowAction::OpenPath, file_id);
            assert!(open.accepted);
            assert_eq!(open.result_name, "zsclip.row.open_path_native_1");

            let open_folder =
                dispatch_linux_native_row_action_for_item(NativeHostRowAction::OpenFolder, file_id);
            assert!(open_folder.accepted);
            assert_eq!(open_folder.result_name, "zsclip.row.open_folder_native_1");

            let copy_path =
                dispatch_linux_native_row_action_for_item(NativeHostRowAction::CopyPath, file_id);
            assert!(copy_path.accepted);
            assert!(copy_path
                .result_name
                .starts_with("zsclip.row.copy_path_clipboard_"));
            assert_eq!(
                LinuxClipboardHost::read_text().as_deref(),
                Some("/tmp/zsclip-linux.txt")
            );

            #[cfg(feature = "ai-actions")]
            {
                let translate = dispatch_linux_native_row_action_for_item(
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
    fn linux_native_edit_text_save_updates_database_item() {
        crate::db_runtime::with_test_db(|| {
            let item_id = crate::db_runtime::with_db_mut(|conn| {
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, text_data, source_app) VALUES(0, 'text', 'old', 'linux-edit', 'old', 'test')",
                    [],
                )?;
                Ok(conn.last_insert_rowid())
            })?;

            let result = dispatch_linux_native_edit_text_save(item_id, "edited on Linux");
            assert!(result.accepted);
            assert_eq!(result.result_name, "zsclip.row.edit.save_db");
            assert_eq!(
                crate::db_runtime::item_text(item_id)?,
                Some("edited on Linux".to_string())
            );

            let missing = dispatch_linux_native_edit_text_save(item_id + 10_000, "missing");
            assert!(!missing.accepted);
            assert_eq!(missing.result_name, "zsclip.row.edit.save_missing");
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn linux_native_assign_group_updates_database_and_group_projection() {
        crate::db_runtime::with_test_db(|| {
            let item_id = crate::db_runtime::with_db_mut(|conn| {
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, text_data, source_app) VALUES(0, 'text', 'group me', 'linux-group', 'group me', 'test')",
                    [],
                )?;
                Ok(conn.last_insert_rowid())
            })?;
            let group = crate::db_runtime::create_native_clip_group(0, "Linux Group")?;

            let result = dispatch_linux_native_assign_group(item_id, group.id);
            assert!(result.accepted);
            assert_eq!(result.result_name, "zsclip.row.assign_group_db");

            let grouped = linux_native_host_projected_clip_items_for_group(group.id);
            assert_eq!(grouped.len(), 1);
            assert_eq!(grouped[0].id, item_id);
            let remove = dispatch_linux_native_remove_group(item_id);
            assert!(remove.accepted);
            assert_eq!(remove.result_name, "zsclip.row.remove_group_db");
            assert!(linux_native_host_projected_clip_items_for_group(group.id).is_empty());
            assert_eq!(
                dispatch_linux_native_group_filter(group.id).result_name,
                "zsclip.group_filter.select_db"
            );
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn linux_native_category_projection_keeps_records_and_phrases_separate() {
        crate::db_runtime::with_test_db(|| {
            let (record_id, phrase_id) = crate::db_runtime::with_db_mut(|conn| {
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, text_data, source_app) VALUES(0, 'text', 'record only', 'linux-record-only', 'record only text', '')",
                    [],
                )?;
                let record_id = conn.last_insert_rowid();
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, text_data, source_app) VALUES(1, 'phrase', 'phrase only', 'linux-phrase-only', 'phrase only text', '')",
                    [],
                )?;
                Ok((record_id, conn.last_insert_rowid()))
            })?;
            let phrase_group = crate::db_runtime::create_native_clip_group(1, "Linux Phrase Group")?;
            assert_eq!(
                crate::db_runtime::assign_native_clip_group(&[phrase_id], phrase_group.id)?,
                1
            );

            let records = linux_native_host_projected_clip_items_for_category_group(0, 0);
            assert!(records.iter().any(|item| item.id == record_id));
            assert!(!records.iter().any(|item| item.id == phrase_id));

            let phrases = linux_native_host_projected_clip_items_for_category_group(1, 0);
            assert!(phrases.iter().any(|item| item.id == phrase_id));
            assert!(!phrases.iter().any(|item| item.id == record_id));

            let grouped_phrases =
                linux_native_host_projected_clip_items_for_category_group(1, phrase_group.id);
            assert_eq!(grouped_phrases.len(), 1);
            assert_eq!(grouped_phrases[0].id, phrase_id);
            assert!(
                linux_native_host_projected_clip_items_for_category_group(0, phrase_group.id)
                    .is_empty()
            );
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn linux_native_settings_group_management_updates_database() {
        crate::db_runtime::with_test_db(|| {
            let create = dispatch_linux_native_create_group(0, "Linux Managed");
            assert!(create.accepted);
            assert_eq!(create.result_name, "zsclip.settings.group.create_db");
            let group = crate::db_runtime::native_clip_groups(0)?
                .into_iter()
                .find(|group| group.name == "Linux Managed")
                .expect("created group should exist");

            let rename = dispatch_linux_native_rename_group(0, group.id, "Linux Renamed");
            assert!(rename.accepted);
            assert_eq!(rename.result_name, "zsclip.settings.group.rename_db");
            assert_eq!(
                crate::db_runtime::native_clip_groups(0)?[0].name,
                "Linux Renamed"
            );

            let second = crate::db_runtime::create_native_clip_group(0, "Linux Second")?;
            let moved = dispatch_linux_native_move_group(0, second.id, -1);
            assert!(moved.accepted);
            assert_eq!(moved.result_name, "zsclip.settings.group.move_db");
            assert_eq!(crate::db_runtime::native_clip_groups(0)?[0].id, second.id);

            let delete = dispatch_linux_native_delete_group(group.id);
            assert!(delete.accepted);
            assert_eq!(delete.result_name, "zsclip.settings.group.delete_db");
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn linux_native_popup_menu_ids_enter_product_command_routes() {
        let copy = dispatch_linux_native_menu_command_id(crate::app_core::menu_ids::ROW_COPY);
        assert!(copy.accepted);
        assert_eq!(copy.result_name, "zsclip.row.copy");

        let assign_group = crate::linux_native_host::dispatch_gtk_menu_command_id(
            crate::app_core::menu_ids::ROW_GROUP_BASE + 1,
        );
        assert!(assign_group.accepted);
        assert_eq!(assign_group.result_name, "zsclip.row.assign_group");
        let source = include_str!("linux_native_host.rs").replace("\r\n", "\n");
        assert!(source.contains("menu_id == menu_ids::ROW_GROUP_REMOVE"));
        assert!(source.contains("dispatch_linux_native_remove_group(selected_item_id)"));
        assert!(source.contains("reload_clip_items_for_group(&current_group_filter"));

        let filter_all = crate::linux_native_host::dispatch_gtk_menu_command_id(
            crate::app_core::menu_ids::GROUP_FILTER_ALL,
        );
        assert!(filter_all.accepted);
        assert_eq!(filter_all.result_name, "zsclip.group_filter.all");

        let filter_group = crate::linux_native_host::dispatch_gtk_menu_command_id(
            crate::app_core::menu_ids::GROUP_FILTER_BASE + 1,
        );
        assert!(filter_group.accepted);
        assert_eq!(filter_group.result_name, "zsclip.group_filter.select");
    }

    #[test]
    fn linux_native_host_rows_open_shared_popup_menu_on_right_click() {
        let host_source = include_str!("linux_native_host.rs").replace("\r\n", "\n");

        assert!(host_source.contains("install_row_context_menu"));
        assert!(host_source.contains("GestureClick::new()"));
        assert!(host_source.contains("gesture.set_button(3)"));
        assert!(host_source.contains("PopoverMenu::from_model(Some(row_menu))"));
        assert!(host_source.contains("selected_item_id.set(item_id)"));
        assert!(host_source.contains("native_host_row_popup_menu_input_for_projection"));
        assert!(host_source.contains("linux_native_grouping_enabled()"));
        assert!(host_source.contains("&clip_items.borrow()"));
        assert!(host_source.contains("replace_popup_menu_entries(\n                &row_menu"));
        assert!(host_source.contains("popover.popup()"));
        assert!(host_source.contains("install_row_context_menu(\n                    row,"));
        assert!(host_source.contains("native_popup_menu_command_accelerator_label(*id)"));
        assert!(host_source.contains("native_popup_menu_command_icon_name(*id)"));
        assert!(host_source.contains("gio::ThemedIcon::new(icon_name)"));
    }

    #[test]
    fn linux_native_vv_select_enters_product_event_bridge() {
        let result = dispatch_linux_native_vv_select_event(1);
        assert!(result.bridged);
        assert_eq!(result.event_name, "vv_select_requested");

        let gtk_result = crate::linux_native_host::dispatch_gtk_vv_select_event(2);
        assert!(gtk_result.bridged);
        assert_eq!(gtk_result.event_name, "vv_select_requested");
    }

    #[test]
    fn linux_native_vv_trigger_uses_shared_trigger_state() {
        let app_source = include_str!("linux_app.rs").replace("\r\n", "\n");
        let host_source = include_str!("linux_native_host.rs").replace("\r\n", "\n");
        let action_source = include_str!("app_core/native_host_actions.rs").replace("\r\n", "\n");
        let component_source =
            include_str!("app_core/native_component_protocol.rs").replace("\r\n", "\n");

        assert!(app_source.contains("fn linux_native_vv_trigger_state()"));
        assert!(app_source.contains("NativeHostVvTriggerState::default()"));
        assert!(app_source.contains("dispatch_linux_native_vv_trigger_key"));
        assert!(app_source.contains("ApplicationEvent::VvShowRequested"));
        assert!(app_source.contains("ApplicationEvent::VvHideRequested"));
        assert!(host_source.contains("dispatch_gtk_vv_trigger_key"));
        assert!(host_source.contains("dispatch_linux_native_vv_trigger_key(input)"));
        assert!(host_source.contains("perform_vv_trigger_demo"));
        assert!(host_source.contains("install_vv_key_controller"));
        assert!(host_source.contains("EventControllerKey::new()"));
        assert!(host_source.contains("connect_key_pressed"));
        assert!(host_source.contains("gtk_vv_trigger_key_from_gdk"));
        assert!(host_source.contains("install_vv_global_key_tap(app)"));
        assert!(host_source.contains("keytap::Tap::new()"));
        assert!(host_source.contains("GtkVvKeytapModifierState"));
        assert!(host_source.contains("gtk_vv_trigger_input_from_keytap_event"));
        assert!(host_source.contains("gtk_vv_trigger_key_from_keytap"));
        assert!(host_source.contains("gtk_vv_global_keytap_target_token"));
        assert!(host_source.contains("VV global keytap installed (observe-only)"));
        assert!(host_source.contains("VV global keytap cannot consume external key"));
        assert!(host_source.contains("handle_vv_trigger_transition"));
        assert!(host_source.contains("perform_gtk_vv_paste"));
        assert!(host_source.contains("let vv_paste = perform_gtk_vv_paste(0, 0);"));
        assert!(host_source.contains("dispatch_linux_native_vv_paste_for_group"));
        assert!(host_source.contains("gtk_post_native_paste_shortcut"));
        assert!(host_source.contains("gtk_post_native_delete_backspaces"));
        assert!(host_source.contains("gtk_try_ydotool_paste_shortcut"));
        assert!(host_source.contains("gtk_try_xdotool_paste_shortcut"));
        assert!(host_source.contains("Command::new(program).args(args).status()"));
        assert!(host_source.contains("ydotool"));
        assert!(host_source.contains("xdotool"));
        assert!(host_source.contains("ZSClip GTK VV native paste shortcut posted="));
        assert!(host_source.contains("ZSClip GTK VV delete backspaces requested="));
        assert!(host_source.contains("NativeHostVvTriggerAction::Show"));
        assert!(host_source.contains("NativeHostVvTriggerAction::Select"));
        assert!(host_source.contains("linux_native_host_projected_clip_items()"));
        assert!(host_source.contains("native_host_vv_popup_render_plan_for_projection"));
        assert!(host_source.contains("native_host_main_tool_button_specs()"));
        assert!(host_source.contains("NativeHostMainToolAction::VvTrigger"));
        assert!(action_source.contains("NativeHostMainToolAction::VvTrigger"));
        assert!(action_source.contains("\"VV Trigger\""));
        assert!(component_source.contains("NativeHostMainToolAction::VvTrigger"));
    }

    #[test]
    fn linux_native_host_applies_first_pass_ui_polish() {
        let host_source = include_str!("linux_native_host.rs").replace("\r\n", "\n");

        assert!(host_source.contains("search_entry.set_max_width_chars(60)"));
        assert!(host_source.contains("let search_escape_controller = EventControllerKey::new()"));
        assert!(host_source.contains("if key == gdk::Key::Escape"));
        assert!(host_source.contains("search_entry_for_escape.set_text(\"\")"));
        assert!(host_source.contains("let search_revealer = Revealer::new()"));
        assert!(host_source
            .contains("search_revealer.set_transition_type(RevealerTransitionType::SlideDown)"));
        assert!(host_source.contains("search_revealer_for_escape.set_reveal_child(false)"));
        assert!(host_source.contains("search_revealer_for_toggle.set_reveal_child(active)"));
        assert!(host_source.contains("search_entry_for_toggle.set_text(\"\")"));
        assert!(host_source.contains("search_entry.connect_search_changed"));
        assert!(host_source.contains("install_main_window_keyboard_controller("));
        assert!(!host_source.contains("native_host_main_action_button_specs()"));
        assert!(host_source.contains(".decorated(false)"));
        assert!(host_source.contains(".title(\"ZSClip VV Popup\")"));
        assert!(host_source.contains("const ZSCLIP_GTK_CSS"));
        assert!(host_source.contains("window.set_icon_name(Some(\"edit-paste\"))"));
        assert!(host_source.contains("gtk::CssProvider::new()"));
        assert!(host_source.contains("provider.load_from_data(ZSCLIP_GTK_CSS)"));
        assert!(host_source.contains("gtk::style_context_add_provider_for_display"));
        assert!(host_source.contains("gtk::STYLE_PROVIDER_PRIORITY_APPLICATION"));
        assert!(host_source.contains("row.add_css_class(\"clip-row\")"));
        assert!(host_source.contains("row.add_css_class(\"settings-row\")"));
        assert!(host_source.contains("window.add_css_class(\"vv-popup\")"));
        assert!(host_source.contains("let popup_key_controller = EventControllerKey::new()"));
        assert!(host_source.contains("NativeHostVvTriggerKey::Escape =>"));
        assert!(host_source.contains("NativeHostVvTriggerKey::Digit1To9(selected_index)"));
        assert!(host_source.contains("dispatch_linux_native_vv_select_event(selected_index)"));
        assert!(host_source.contains("perform_gtk_vv_paste(selected_index, current_group_id)"));
        assert!(host_source.contains("window_for_keys.close()"));
        assert!(host_source.contains("window.add_controller(popup_key_controller)"));
        assert!(host_source.contains(".vv-index"));
        assert!(host_source.contains("font-size: 22px"));
        assert!(host_source.contains("font-weight: 700"));
        assert!(host_source.contains(".vv-preview"));
        assert!(host_source.contains("font-family: monospace"));
        assert!(host_source.contains("number.add_css_class(\"vv-index\")"));
        assert!(host_source.contains("label.add_css_class(\"vv-preview\")"));
        assert!(host_source.contains("window.connect_notify_local(Some(\"is-active\")"));
        assert!(host_source.contains("background: @theme_bg_color"));
        assert!(host_source.contains("alpha(@theme_fg_color, 0.05)"));
        assert!(host_source.contains(".clip-list row:selected"));
        assert!(host_source.contains("background: alpha(@accent_color, 0.22)"));
        assert!(host_source.contains("let header = HeaderBar::new()"));
        assert!(host_source.contains("header.set_show_title_buttons(true)"));
        assert!(host_source.contains("header.set_title_widget(Some(&header_title))"));
        assert!(host_source.contains("trait GtkWindowSystemBackend"));
        assert!(host_source.contains("struct Gtk4WindowSystemBackend"));
        assert!(host_source.contains("struct X11CommandWindowSystemBackend"));
        assert!(host_source.contains("fn gtk_select_window_system_backend("));
        assert!(host_source.contains("fn gtk_main_window_capabilities()"));
        assert!(host_source.contains("HostCapabilities::linux_native_window_host()"));
        assert!(host_source.contains(".resolve_for(&gtk_main_window_capabilities())"));
        assert!(host_source.contains("window.set_resizable(window_spec.resizable)"));
        assert!(host_source.contains("window.set_decorated(window_spec.decorations)"));
        assert!(host_source.contains("(window_spec.min_width, window_spec.min_height)"));
        assert!(
            host_source.contains("window.set_size_request(min_width as i32, min_height as i32)")
        );
        assert!(host_source.contains(
            "gtk_native_window_traits(&window, &window_backend, window_spec.always_on_top)"
        ));
        assert!(host_source.contains("fn apply_always_on_top("));
        assert!(host_source.contains("fn position_near_cursor("));
        assert!(host_source.contains("gtk_window_command_success(\"wmctrl\""));
        assert!(
            host_source.contains("gtk_window_command_output(\"xdotool\", &[\"getactivewindow\"]")
        );
        assert!(host_source.contains(
            "gtk_window_command_output(\"xdotool\", &[\"getmouselocation\", \"--shell\"]"
        ));
        assert!(host_source.contains("fn gtk_native_window_traits("));
        assert!(host_source.contains("gtk::Settings::default()"));
        assert!(host_source.contains("is_gtk_application_prefer_dark_theme()"));
        assert!(host_source.contains(".monitor_at_surface(&surface)"));
        assert!(host_source.contains("monitor.scale_factor()"));
        assert!(host_source.contains("window.scale_factor()"));
        assert!(host_source.contains("zsclip.gtk.window.always_on_top.requires_backend_adapter"));
        assert!(host_source.contains("zsclip.gtk.window.cursor_follow.requires_backend_adapter"));
        assert!(host_source.contains("let window_backend = gtk_select_window_system_backend()"));
        assert!(host_source.contains("always_on_top_supported={}"));
        assert!(host_source.contains("cursor_follow_supported={}"));
        assert!(host_source.contains("ToggleButton::builder()"));
        assert!(host_source.contains(".icon_name(\"edit-find-symbolic\")"));
        assert!(host_source.contains("header.pack_start(&search_button)"));
        assert!(host_source.contains(".icon_name(\"open-menu-symbolic\")"));
        assert!(host_source.contains("header.pack_end(&status_button)"));
        assert!(host_source.contains("window.set_titlebar(Some(&header))"));
        assert!(host_source.contains("search_button.connect_toggled"));
        assert!(host_source.contains("search_revealer_for_toggle.set_reveal_child(active)"));
        assert!(host_source.contains("let main_toolbar = GtkBox::new(Orientation::Horizontal, 8)"));
        assert!(host_source.contains("main_toolbar.add_css_class(\"main-toolbar\")"));
        assert!(host_source.contains("row_actions.add_css_class(\"main-toolbar-actions\")"));
        assert!(host_source.contains("main_toolbar.append(&source_tabs)"));
        assert!(host_source.contains("main_toolbar.append(&row_actions)"));
        assert!(host_source.contains("root.append(&main_toolbar)"));
        assert!(host_source.contains("install_source_tab_group_context_menu("));
        assert!(host_source.contains("gesture.set_button(3)"));
        assert!(host_source.contains("PopoverMenu::from_model(Some(&menus.group_filter_menu))"));
        assert!(host_source.contains("refresh_group_popup_menus(&menus)"));
        assert!(
            host_source.find("root.append(&main_toolbar)").unwrap()
                < host_source.find("root.append(&clip_scroller)").unwrap(),
            "GTK source tabs and group/action toolbar should stay above the list"
        );
        assert!(host_source.contains("let clip_scroller = ScrolledWindow::builder()"));
        assert!(host_source.contains(".hscrollbar_policy(PolicyType::Never)"));
        assert!(host_source.contains(".vscrollbar_policy(PolicyType::Automatic)"));
        assert!(host_source.contains("let clip_list = ListBox::new()"));
        assert!(host_source.contains("let row = ListBoxRow::new()"));
        assert!(host_source.contains("clip_list.set_selection_mode(SelectionMode::Single)"));
        assert!(host_source.contains("clip_list.set_focusable(true)"));
        assert!(host_source.contains("sync_clip_list_selection(&clip_list, &clip_rows"));
        assert!(host_source.contains("clip_list.grab_focus()"));
        assert!(host_source.contains("clip_list.set_show_separators(true)"));
        assert!(host_source.contains("clip_list.connect_row_selected"));
        assert!(host_source.contains("clip_list.connect_row_activated"));
        assert!(!host_source.contains("native_host_row_action_button_specs()"));
        assert!(host_source.contains("fn install_main_window_keyboard_controller("));
        assert!(host_source.contains("gdk::ModifierType::CONTROL_MASK"));
        assert!(host_source.contains("matches!(key.to_unicode(), Some('f' | 'F'))"));
        assert!(host_source.contains("key == gdk::Key::Return || key == gdk::Key::KP_Enter"));
        assert!(host_source.contains("key == gdk::Key::Delete || key == gdk::Key::BackSpace"));
        assert!(host_source.contains(
            "perform_gtk_item_row_action(\n                    NativeHostRowAction::Paste"
        ));
        assert!(host_source.contains(
            "perform_gtk_item_row_action(\n                    NativeHostRowAction::Delete"
        ));
        assert!(host_source.contains("clip_list.add_css_class(\"boxed-list\")"));
        assert!(host_source.contains("clip_list.add_css_class(\"clip-list\")"));
        assert!(host_source.contains("clip_list.append(row)"));
        assert!(host_source.contains("clip_scroller.set_child(Some(&clip_list))"));
        assert!(host_source.contains("root.append(&clip_scroller)"));
        assert!(host_source.contains("install_clipboard_capture_timer("));
        assert!(host_source.contains("Duration::from_millis(800)"));
        assert!(host_source.contains("linux_native_clipboard_capture_enabled()"));
        assert!(host_source.contains("NativeClipboardCaptureService::capture_current"));
        assert!(host_source.contains("ZSClip GTK auto smoke row action"));
        assert!(host_source.contains("NativeHostRowAction::Paste"));
        assert!(host_source.contains("NativeHostRowAction::Pin"));
        assert!(host_source.contains("dispatch_linux_native_assign_group(item_id, group.id)"));
        assert!(host_source.contains("dispatch_linux_native_group_filter(group.id)"));
        assert!(host_source.contains("dispatch_linux_native_remove_group(item_id)"));
        assert!(host_source.contains("ZSClip GTK auto smoke delete item_id="));
        assert!(host_source.contains("insert_native_clipboard_image("));
        assert!(host_source.contains("ZSClip GTK auto smoke image copy item_id="));
        assert!(host_source
            .contains("LinuxClipboardHost as crate::app_core::ClipboardHost>::read_image_rgba()"));
        assert!(!host_source.contains("native_host_main_action_button_specs()"));
        assert!(host_source.contains("reload_clip_items_for_group_with_selection("));
        assert!(host_source.contains("fn gtk_clip_row_content("));
        assert!(host_source.contains("NativeHostClipRowPresentation"));
        assert!(host_source.contains("native_host_clip_row_presentation_for_projection"));
        assert!(host_source.contains("presentation.title.as_str()"));
        assert!(host_source.contains("presentation.preview.as_str()"));
        assert!(host_source.contains("presentation.accessibility_label"));
        assert!(host_source.contains("presentation.pin_badge"));
        assert!(host_source.contains("presentation.kind_icon"));
        assert!(host_source.contains("Image::from_icon_name(gtk_clip_row_icon_name("));
        assert!(host_source.contains("fn gtk_clip_row_icon_name("));
        assert!(host_source.contains(".zsui_icon()"));
        assert!(host_source.contains(".gtk_symbolic_name()"));
        assert!(host_source.contains("Image::from_icon_name(\"view-pin-symbolic\")"));
        assert!(host_source.contains("pin_icon.add_css_class(\"clip-row-pin\")"));
        assert!(host_source.contains("icon.set_pixel_size(24)"));
        assert!(host_source.contains("title_label.set_ellipsize(gtk::pango::EllipsizeMode::End)"));
        assert!(host_source.contains("preview_label.set_ellipsize(gtk::pango::EllipsizeMode::End)"));
        assert!(host_source.contains("preview_label.set_lines(1)"));
        assert!(host_source.contains("row.set_child(Some(&gtk_clip_row_content("));
        assert!(host_source.contains("No clipboard records"));
        assert!(host_source.contains("items.is_empty() && index == 0"));
        assert!(host_source.contains("row.set_selectable(false)"));
        assert!(host_source.contains("row.set_selectable(action.has_item())"));
        assert!(host_source.contains("row.set_activatable(action.has_item())"));
        assert!(host_source.contains("row_box.add_css_class(\"clip-row-content\")"));
        assert!(host_source.contains("let text_view = TextView::new()"));
        assert!(host_source.contains("text_view.set_wrap_mode(gtk::WrapMode::WordChar)"));
        assert!(host_source.contains("let editor_scroller = ScrolledWindow::builder()"));
        assert!(host_source.contains("editor_scroller.set_child(Some(&text_view))"));
        assert!(host_source.contains("window.set_modal(true)"));
        assert!(host_source.contains("window.set_transient_for(Some(&parent))"));
        assert!(host_source.contains("fn gtk_edit_text_view_text(text_view: &TextView) -> String"));
        assert!(host_source.contains("let buffer = text_view.buffer()"));
        assert!(host_source.contains(".text(&buffer.start_iter(), &buffer.end_iter(), true)"));
        assert!(host_source.contains("NativeHostEditTextAction::Save =>"));
        assert!(host_source.contains("if save_gtk_edit_text("));
        assert!(host_source.contains("NativeHostEditTextAction::Cancel =>"));
        assert!(
            host_source.contains("native_host_edit_text_close_plan(initial_text, &current_text)")
        );
        assert!(host_source.contains("fn present_gtk_edit_unsaved_changes_dialog("));
        assert!(host_source.contains("dialog.add_button(\"Save\", gtk::ResponseType::Yes)"));
        assert!(host_source.contains("dialog.add_button(\"Discard\", gtk::ResponseType::No)"));
        assert!(host_source.contains("window.connect_close_request(move |_|"));
        assert!(host_source.contains("let notebook = Notebook::new()"));
        assert!(host_source.contains("for spec in native_host_settings_page_tab_specs()"));
        assert!(host_source.contains("notebook.append_page(&scroller"));
        assert!(host_source.contains("NativeSettingsPageTabKind::General"));
        assert!(host_source.contains("NativeSettingsPageTabKind::Groups"));
        assert!(host_source.contains("NativeSettingsPageTabKind::Actions"));
        assert!(host_source.contains("native_host_settings_section_label(\"settings_controls\")"));
        assert!(host_source.contains("native_host_settings_section_label(\"group_selector\")"));
        assert!(host_source.contains("native_host_settings_toggle_specs()"));
        assert!(host_source.contains("native_host_settings_dropdown_specs()"));
        assert!(host_source.contains("let switch = Switch::new()"));
        assert!(host_source.contains("switch.set_widget_name(spec.id)"));
        assert!(host_source.contains("switch.connect_active_notify"));
        assert!(host_source.contains("if spec.options.is_empty()"));
        assert!(host_source.contains("spec.options\n                    .iter()"));
        assert!(host_source.contains(".map(|option| option.label)"));
        assert!(host_source.contains("option.raw_value == binding.initial_value"));
        assert!(host_source.contains("let dropdown = DropDown::from_strings(&labels)"));
        assert!(host_source.contains("dropdown.set_widget_name(spec.id)"));
        assert!(host_source.contains("dropdown.connect_selected_notify"));
    }

    #[test]
    fn linux_native_vv_paste_writes_clipboard_and_targets_text_input() {
        let _guard = linux_clipboard_test_guard();
        crate::db_runtime::with_test_db(|| {
            LinuxClipboardHost::reset_for_tests();
            crate::db_runtime::with_db_mut(|conn| {
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, text_data, source_app) VALUES(0, 'text', 'linux vv', 'linux-vv', 'linux vv full text', 'Terminal')",
                    [],
                )?;
                Ok(())
            })?;

            let result = dispatch_linux_native_vv_paste(0);
            assert!(result.accepted);
            assert_eq!(result.result_name, "zsclip.vv_paste.clipboard_target");
            assert_eq!(result.clipboard_kind, Some("text"));
            assert!(result.focused_target);
            assert!(result.direct_text_set);
            assert!(result.paste_shortcut_sent);
            assert_eq!(
                LinuxClipboardHost::read_text(),
                Some("linux vv full text".to_string())
            );
            let gtk_result = crate::linux_native_host::dispatch_gtk_vv_paste(0);
            assert!(gtk_result.accepted);
            assert_eq!(gtk_result.result_name, "zsclip.vv_paste.clipboard_target");
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn linux_native_vv_paste_for_group_uses_group_projection() {
        let _guard = linux_clipboard_test_guard();
        crate::db_runtime::with_test_db(|| {
            LinuxClipboardHost::reset_for_tests();
            let (first_id, second_id) = crate::db_runtime::with_db_mut(|conn| {
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, text_data, source_app) VALUES(0, 'text', 'all only', 'linux-vv-all', 'all only text', 'Terminal')",
                    [],
                )?;
                let first_id = conn.last_insert_rowid();
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, text_data, source_app) VALUES(0, 'text', 'group only', 'linux-vv-group', 'group only text', 'Terminal')",
                    [],
                )?;
                Ok((first_id, conn.last_insert_rowid()))
            })?;
            let group = crate::db_runtime::create_native_clip_group(0, "VV Group")?;
            assert!(dispatch_linux_native_assign_group(second_id, group.id).accepted);
            assert_ne!(first_id, second_id);

            let result = dispatch_linux_native_vv_paste_for_group(0, group.id);
            assert!(result.accepted);
            assert_eq!(result.result_name, "zsclip.vv_paste.clipboard_target");
            assert_eq!(
                LinuxClipboardHost::read_text(),
                Some("group only text".to_string())
            );
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn linux_native_search_text_enters_product_command_route() {
        let result =
            dispatch_linux_native_search_text_action(NativeHostSearchTextAction::new("needle"));
        assert!(result.accepted);
        assert_eq!(result.result_name, "zsclip.window.search_text_update");

        let gtk_result = crate::linux_native_host::dispatch_gtk_search_text_action(
            NativeHostSearchTextAction::new("from gtk"),
        );
        assert!(gtk_result.accepted);
        assert_eq!(gtk_result.result_name, "zsclip.window.search_text_update");
    }

    #[test]
    fn linux_native_search_reload_uses_database_query_path() {
        let host_source = include_str!("linux_native_host.rs");
        assert!(host_source.contains(
            "linux_native_host_projected_clip_items_for_category_group_kind_filter_search"
        ));
        assert!(host_source.contains("reload_clip_items_for_group_search_with_selection"));
        assert!(host_source.contains("search_entry.text().as_str()"));
    }

    #[test]
    fn linux_native_host_projected_clip_items_read_database_without_demo_fallback() {
        crate::db_runtime::with_test_db(|| {
            assert!(linux_native_host_projected_clip_items().is_empty());

            crate::db_runtime::with_db_mut(|conn| {
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, text_data, source_app) VALUES(0, 'text', 'real GTK history', 'gtk-history', 'real GTK history', 'Terminal')",
                    [],
                )?;
                Ok(())
            })?;

            let items = linux_native_host_projected_clip_items();
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].title, "Terminal");
            assert_eq!(items[0].preview, "real GTK history");
            assert_eq!(items[0].kind, crate::app_core::ClipKind::Text);
            assert!(!items[0].pinned);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn linux_host_scaffold_consumes_zsui_contract_summary() {
        let summary = linux_host_contract_summary();

        assert_eq!(summary.api_major, APP_CORE_API_VERSION.major);
        assert_eq!(summary.surfaces, REQUIRED_UI_HOST_SURFACES.len());
        assert_eq!(
            summary.main_execution_plans,
            REQUIRED_MAIN_HOST_EXECUTION_PLAN_KINDS.len()
        );
        assert_eq!(
            summary.native_style_operations,
            REQUIRED_NATIVE_STYLE_HOST_OPERATIONS.len()
        );
        assert_eq!(
            summary.native_control_mapper_operations,
            REQUIRED_NATIVE_CONTROL_MAPPER_OPERATIONS.len()
        );
        assert_eq!(
            summary.text_layout_operations,
            REQUIRED_TEXT_LAYOUT_HOST_OPERATIONS.len()
        );
        assert_eq!(
            summary.status_item_operations,
            REQUIRED_STATUS_ITEM_HOST_OPERATIONS.len()
        );
        assert_eq!(
            summary.popup_menu_operations,
            REQUIRED_NATIVE_POPUP_MENU_HOST_OPERATIONS.len()
        );
        assert_eq!(
            summary.transient_window_operations,
            REQUIRED_NATIVE_TRANSIENT_WINDOW_HOST_OPERATIONS.len()
        );
        assert_eq!(
            summary.ime_operations,
            REQUIRED_NATIVE_IME_HOST_OPERATIONS.len()
        );
        assert_eq!(
            summary.dialog_operations,
            REQUIRED_NATIVE_DIALOG_HOST_OPERATIONS.len()
        );
        assert_eq!(
            summary.shell_open_operations,
            REQUIRED_NATIVE_SHELL_OPEN_HOST_OPERATIONS.len()
        );
        assert_eq!(
            summary.window_identity_operations,
            REQUIRED_NATIVE_WINDOW_IDENTITY_HOST_OPERATIONS.len()
        );
        assert_eq!(
            summary.paste_target_operations,
            REQUIRED_NATIVE_PASTE_TARGET_HOST_OPERATIONS.len()
        );
        assert_eq!(
            summary.text_caret_operations,
            REQUIRED_NATIVE_TEXT_CARET_HOST_OPERATIONS.len()
        );
        assert_eq!(
            summary.file_dialog_operations,
            REQUIRED_NATIVE_FILE_DIALOG_HOST_OPERATIONS.len()
        );
        assert_eq!(
            summary.text_input_dialog_operations,
            REQUIRED_NATIVE_TEXT_INPUT_DIALOG_HOST_OPERATIONS.len()
        );
        assert_eq!(
            summary.edit_text_dialog_operations,
            REQUIRED_NATIVE_EDIT_TEXT_DIALOG_HOST_OPERATIONS.len()
        );
        assert_eq!(
            summary.mail_merge_window_operations,
            REQUIRED_NATIVE_MAIL_MERGE_WINDOW_HOST_OPERATIONS.len()
        );
        assert_eq!(
            summary.renderer_operations,
            REQUIRED_RENDERER_HOST_OPERATIONS.len()
        );
        assert_eq!(
            summary.main_window_operations,
            REQUIRED_NATIVE_MAIN_WINDOW_HOST_OPERATIONS.len()
        );
        assert_eq!(
            summary.main_search_operations,
            REQUIRED_NATIVE_MAIN_SEARCH_CONTROL_HOST_OPERATIONS.len()
        );
        assert_eq!(
            summary.settings_window_operations,
            REQUIRED_NATIVE_SETTINGS_WINDOW_HOST_OPERATIONS.len()
        );
        assert_eq!(
            summary.settings_control_operations,
            REQUIRED_SETTINGS_CONTROL_HOST_OPERATIONS.len()
        );
        assert_eq!(
            summary.settings_dropdown_operations,
            REQUIRED_NATIVE_SETTINGS_DROPDOWN_HOST_OPERATIONS.len()
        );
        assert_eq!(
            summary.shared_non_host_protocols,
            SHARED_NON_HOST_UI_PROTOCOLS.len()
        );
        assert_eq!(summary.backend, LinuxNativeBackend::Gtk4Libadwaita);
    }

    #[test]
    fn linux_application_model_owns_lifecycle_commands_and_startup_host() {
        let mut application = LinuxApplicationModel::default();
        let startup = application.mount("ZSClip Linux", true).unwrap();

        assert_eq!(application.lifecycle_phase(), ComponentPhase::Mounted);
        assert_eq!(startup.lifecycle, LifecycleEvent::Mount);
        assert_eq!(startup.backend, LinuxNativeBackend::Gtk4Libadwaita);
        assert_eq!(startup.request.title, "ZSClip Linux");
        assert_eq!(startup.request.size.width, 760);
        assert!(startup.request.options.resizable);
        assert!(startup.request.options.decorations);

        let presentation = application
            .main_window_host_mut()
            .create_main_windows(startup.request);
        let NativeMainWindowPresentation::Created(handles) = presentation else {
            panic!("Linux scaffold should produce native window handles");
        };
        application.record_startup_presentation(startup.backend, presentation);
        assert_eq!(
            application.startup_session().backend(),
            Some(LinuxNativeBackend::Gtk4Libadwaita)
        );
        assert_eq!(
            application.startup_session().created_main_windows(),
            Some(handles)
        );
        assert_eq!(application.startup_session().generation(), 1);
        assert_eq!(application.main_window_host.create_request_count(), 1);
        assert_eq!(
            application.main_window_host.created_handles(),
            Some(handles)
        );

        application
            .main_window_host_mut()
            .request_main_window_area_repaint(handles.main, Some(UiRect::new(0, 0, 20, 20)), false);
        assert_eq!(application.main_window_host.repaint_request_count(), 1);

        application.queue_command(Command {
            id: command_ids::OPEN_SETTINGS,
            scope: CommandScope::Window,
            payload: CommandPayload::None,
        });
        assert_eq!(application.command_count(), 1);
        assert_eq!(
            application.drain_command().map(|command| command.id),
            Some(command_ids::OPEN_SETTINGS)
        );
        assert!(application.activate());
        assert_eq!(application.lifecycle_phase(), ComponentPhase::Active);
    }

    #[test]
    fn linux_main_window_host_records_zsui_degradation_details() {
        let mut host = LinuxMainWindowHost::default();
        let request = NativeMainWindowRequest::from_zsui_window_for_host(
            &crate::zsui::Window::new("Transparent Linux").transparent(true),
            &crate::zsui::HostCapabilities::linux_native_window_host(),
        );

        assert!(!request.options.transparent);
        assert!(request
            .degraded_capabilities
            .iter()
            .any(|detail| detail.contains("window_transparency")));

        let presentation = host.create_main_windows(request);
        assert!(matches!(
            presentation,
            NativeMainWindowPresentation::Created(_)
        ));
        assert_eq!(host.create_requests().len(), 1);
        assert!(!host.create_requests()[0].options.transparent);
        assert!(host.create_requests()[0]
            .degraded_capabilities
            .iter()
            .any(|detail| detail.contains("window_transparency")));
    }

    #[test]
    fn linux_application_model_implements_native_runtime_driver() {
        let mut application = LinuxApplicationModel::default();
        let startup = application.start_runtime(NativeRuntimeStartupRequest {
            app_name: "Demo Linux".to_string(),
            main_window: NativeMainWindowRequest {
                title: "Demo Linux".to_string(),
                size: Size {
                    width: 640,
                    height: 420,
                },
                options: NativeWindowOptions::standard(),
                main_visible: true,
                degraded_capabilities: Vec::new(),
            },
            status_item_tooltip: Some("Demo Linux".to_string()),
        });

        let NativeRuntimeStartupResult::Started(handles) = startup else {
            panic!("Linux runtime driver should create native window handles");
        };
        assert_eq!(
            application.startup_session().backend(),
            Some(LinuxNativeBackend::Gtk4Libadwaita)
        );
        assert_eq!(
            application.startup_session().created_main_windows(),
            Some(handles)
        );
        assert_eq!(
            application.status_item_host.installed_tooltip(),
            Some("Demo Linux")
        );

        application.dispatch_ui_command(Command::window(command_ids::OPEN_SETTINGS));
        assert_eq!(application.command_count(), 1);
        assert_eq!(
            application.product_command_results()[0].result_name,
            "zsclip.window.open_settings"
        );
        assert_eq!(
            application.poll_application_event(),
            Some(ApplicationEvent::ItemsPageReady)
        );
        assert_eq!(
            application.drain_command().map(|command| command.id),
            Some(command_ids::OPEN_SETTINGS)
        );

        application.request_shutdown();
        assert!(application.runtime_shutdown_requested);
    }

    #[test]
    fn linux_application_can_reuse_zsclip_product_adapter() {
        use crate::app_core::{main_menu_command_for_id, menu_ids, ProductAdapterHost};
        use crate::zsclip_product_adapter::{
            zsclip_product_adapter_manifest, ZsclipProductAdapter,
        };

        let manifest = zsclip_product_adapter_manifest();
        assert!(manifest.command_routes.iter().any(|route| {
            route.family_name == "tray"
                && route.result_name == "zsclip.tray.toggle_lan_sync"
                && route.execution_owner == "product_adapter"
        }));
        assert_eq!(
            manifest.ai_provider_names,
            vec!["llms", "skills", "product_adapter"]
        );

        let mut adapter = ZsclipProductAdapter::default();
        let command = main_menu_command_for_id(menu_ids::TRAY_LAN_TOGGLE).unwrap();
        let result = adapter.execute_product_command(command);
        assert!(result.accepted);
        assert_eq!(result.result_name, "zsclip.tray.toggle_lan_sync");

        let mut application = LinuxApplicationModel::default();
        application.dispatch_ui_command(
            main_menu_command_for_id(menu_ids::TRAY_LAN_TOGGLE)
                .expect("tray LAN toggle should be a shared ZSClip menu command"),
        );
        assert_eq!(
            application.product_command_results()[0].result_name,
            "zsclip.tray.toggle_lan_sync"
        );

        let bridge = application.route_application_event(ApplicationEvent::CloudSyncReady);
        assert!(bridge.bridged);
        assert_eq!(bridge.event_name, "cloud_sync_ready");
        assert_eq!(
            application.product_event_results()[0].event_name,
            "cloud_sync_ready"
        );
    }

    #[test]
    fn linux_status_and_popup_hosts_consume_shared_menu_contracts() {
        let mut application = LinuxApplicationModel::default();
        assert!(application.status_item_host_mut().install("ZSClip Linux"));
        application.status_item_host_mut().present_menu(&[
            StatusMenuEntry::Command {
                action: crate::app_core::MainTrayMenuAction::ToggleWindow,
                label: "Show".to_string(),
                icon_name: "window-new-symbolic".to_string(),
            },
            StatusMenuEntry::Separator,
            StatusMenuEntry::Command {
                action: crate::app_core::MainTrayMenuAction::Exit,
                label: "Exit".to_string(),
                icon_name: "application-exit-symbolic".to_string(),
            },
        ]);
        assert_eq!(
            application.status_item_host.installed_tooltip(),
            Some("ZSClip Linux")
        );
        assert_eq!(application.status_item_host.menu_entry_count(), 3);

        let selected = application.popup_menu_host_mut().present_popup_menu(
            LinuxMainWindowHandle(1),
            10,
            20,
            NativePopupMenuPlacement::BottomLeft,
            &[
                NativePopupMenuEntry::Command {
                    id: 0,
                    label: "Disabled".to_string(),
                    enabled: false,
                    checked: false,
                },
                NativePopupMenuEntry::Submenu {
                    label: "More".to_string(),
                    enabled: true,
                    entries: vec![NativePopupMenuEntry::Command {
                        id: 42,
                        label: "Open".to_string(),
                        enabled: true,
                        checked: false,
                    }],
                },
            ],
        );
        assert_eq!(selected, 42);
        assert_eq!(application.popup_menu_host.presentation_count(), 1);
        assert_eq!(application.popup_menu_host.last_selected_id(), Some(42));

        application.status_item_host_mut().remove();
        assert_eq!(application.status_item_host.installed_tooltip(), None);
    }

    #[test]
    fn linux_style_and_text_layout_hosts_consume_shared_rendering_contracts() {
        let application = LinuxApplicationModel::default();
        let text_style = application
            .style_resolver()
            .resolve_text_style(SemanticTextStyle::body());
        assert_eq!(text_style.font_family, "Sans");
        assert_eq!(application.style_resolver().resolved_count(), 1);

        assert_eq!(
            application
                .control_mapper()
                .class_name(SettingsComponentKind::Label),
            LinuxNativeControlClass::Label
        );
        assert_eq!(
            application
                .control_mapper()
                .class_name(SettingsComponentKind::TextInput),
            LinuxNativeControlClass::Entry
        );
        assert_eq!(
            application
                .control_mapper()
                .class_name(SettingsComponentKind::Dropdown),
            LinuxNativeControlClass::ComboRow
        );
        assert_eq!(
            application
                .control_mapper()
                .class_name(SettingsComponentKind::AccentButton),
            LinuxNativeControlClass::SuggestedActionButton
        );
        assert_eq!(application.control_mapper().mapped_controls().len(), 4);

        let measured = application.text_layout().measure("hello", &text_style, 200);
        assert!(measured.width > 0);
        assert!(measured.height > 0);

        let bounds = Rect {
            x: 0,
            y: 0,
            width: 120,
            height: 24,
        };
        let runs = application
            .text_layout()
            .layout_runs("hello", &text_style, bounds);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text, "hello");
        assert_eq!(application.text_layout().actions().len(), 2);
    }

    #[test]
    fn linux_transient_window_and_ime_hosts_consume_shared_contracts() {
        let mut application = LinuxApplicationModel::default();

        let presentation = application
            .transient_window_host_mut()
            .create_transient_window(NativeTransientWindowRequest {
                owner: LinuxMainWindowHandle(5),
                bounds: UiRect::new(0, 0, 200, 80),
            });
        let NativeTransientWindowPresentation::Created(handle) = presentation else {
            panic!("Linux transient host should create a handle");
        };
        application
            .transient_window_host_mut()
            .present_transient_window(handle, UiRect::new(10, 20, 210, 120));
        application
            .transient_window_host_mut()
            .hide_transient_window(handle);
        application
            .transient_window_host_mut()
            .destroy_transient_window(handle);
        assert_eq!(application.transient_window_host.requests().len(), 1);
        assert_eq!(application.transient_window_host.actions().len(), 3);

        application
            .ime_host()
            .set_next_candidate(Some(NativeImeCandidateAnchor::CandidatePoint {
                position: Point { x: 10, y: 20 },
            }));
        application
            .ime_host()
            .set_next_composition(Some(NativeImeCompositionAnchor::Rect {
                rect: UiRect::new(1, 2, 30, 40),
            }));
        application
            .ime_host_mut()
            .set_next_has_default_ime_window(true);
        assert_eq!(
            application
                .ime_host_mut()
                .candidate_anchor(LinuxImeHandle(7), 2),
            Some(NativeImeCandidateAnchor::CandidatePoint {
                position: Point { x: 10, y: 20 },
            })
        );
        assert_eq!(
            application
                .ime_host_mut()
                .composition_anchor(LinuxImeHandle(7)),
            Some(NativeImeCompositionAnchor::Rect {
                rect: UiRect::new(1, 2, 30, 40),
            })
        );
        assert!(application
            .ime_host_mut()
            .has_default_ime_window(LinuxImeHandle(7)));
        assert_eq!(application.ime_host().actions().len(), 3);
    }

    #[test]
    fn linux_clipboard_host_consumes_shared_clipboard_contract() {
        let _guard = linux_clipboard_test_guard();
        let source = include_str!("linux_app.rs");

        LinuxClipboardHost::reset_for_tests();
        assert!(source.contains("fn system_clipboard_fingerprint() -> Option<u64>"));
        assert!(source.contains("DefaultHasher::new()"));
        assert!(source.contains("last_system_fingerprint"));
        assert!(source.contains("state.sequence = state.sequence.saturating_add(1)"));
        assert!(source.contains("Self::observed_system_sequence().unwrap_or_else"));
        assert_eq!(LinuxClipboardHost::sequence_number(), 0);

        assert!(LinuxClipboardHost::write_text("hello"));
        assert_eq!(LinuxClipboardHost::read_text(), Some("hello".to_string()));
        assert_eq!(LinuxClipboardHost::sequence_number(), 1);

        let image = vec![255, 0, 0, 255, 0, 255, 0, 255];
        assert!(LinuxClipboardHost::write_image_rgba(&image, 2, 1));
        assert_eq!(LinuxClipboardHost::read_image_rgba(), Some((image, 2, 1)));
        assert_eq!(LinuxClipboardHost::read_text(), None);
        assert!(!LinuxClipboardHost::write_image_rgba(&[1, 2, 3], 1, 1));

        let paths = vec!["/tmp/a.txt".to_string(), "/tmp/b.txt".to_string()];
        assert!(LinuxClipboardHost::write_file_paths(&paths));
        assert_eq!(LinuxClipboardHost::read_file_paths(), Some(paths));

        assert!(LinuxClipboardHost::write_text_ignored_by_monitors("self"));
        assert_eq!(LinuxClipboardHost::read_text(), Some("self".to_string()));
        assert!(LinuxClipboardHost::should_ignore_capture_by_named_format());
        assert!(!LinuxClipboardHost::should_ignore_capture_by_named_format());
    }

    #[test]
    fn linux_clipboard_host_parses_uri_list_file_paths() {
        assert_eq!(
            LinuxClipboardHost::file_paths_from_uri_list(
                "# copied files\nfile:///home/user/a%20file.txt\nfile://localhost/tmp/b.txt\n"
            ),
            Some(vec![
                "/home/user/a file.txt".to_string(),
                "/tmp/b.txt".to_string()
            ])
        );
        assert_eq!(
            LinuxClipboardHost::file_paths_from_uri_list("plain clipboard text"),
            None
        );
        assert_eq!(
            LinuxClipboardHost::file_paths_from_uri_list("file:///tmp/bad%zz.txt"),
            None
        );
    }

    #[test]
    fn linux_clipboard_host_serializes_file_paths_as_uri_list() {
        assert_eq!(
            file_paths_to_uri_list(&[
                "/home/user/a file.txt".to_string(),
                "/tmp/#hash.txt".to_string()
            ])
            .as_deref(),
            Some("file:///home/user/a%20file.txt\nfile:///tmp/%23hash.txt\n")
        );
        assert_eq!(
            LinuxClipboardHost::file_paths_from_uri_list(
                file_paths_to_uri_list(&["/home/user/a file.txt".to_string()])
                    .unwrap()
                    .as_str(),
            ),
            Some(vec!["/home/user/a file.txt".to_string()])
        );
        assert_eq!(
            clipboard_text_without_uri_list(
                file_paths_to_uri_list(&["/home/user/a file.txt".to_string()]).unwrap()
            ),
            None
        );
        assert_eq!(
            clipboard_text_without_uri_list("plain clipboard text".to_string()).as_deref(),
            Some("plain clipboard text")
        );
        assert_eq!(file_paths_to_uri_list(&[]), None);

        let _guard = linux_clipboard_test_guard();
        LinuxClipboardHost::reset_for_tests();
        assert!(!LinuxClipboardHost::write_file_paths(&[]));
    }

    #[test]
    fn linux_application_polls_clipboard_changes_into_events() {
        let _guard = linux_clipboard_test_guard();
        LinuxClipboardHost::reset_for_tests();
        let mut application = LinuxApplicationModel::default();

        assert_eq!(application.poll_application_event(), None);
        assert!(LinuxClipboardHost::write_text("external linux change"));
        assert_eq!(
            application.poll_application_event(),
            Some(ApplicationEvent::ClipboardChanged { sequence: 1 })
        );
        assert_eq!(application.poll_application_event(), None);

        assert!(LinuxClipboardHost::write_text_ignored_by_monitors(
            "self linux write"
        ));
        assert_eq!(application.poll_application_event(), None);

        application.set_clipboard_capture_enabled(false);
        assert!(LinuxClipboardHost::write_text("disabled linux change"));
        assert_eq!(application.poll_application_event(), None);
    }

    #[test]
    fn linux_dialog_and_window_identity_hosts_consume_shared_contracts() {
        let application = LinuxApplicationModel::default();

        application.dialog_host().show_message(
            LinuxMainWindowHandle(1),
            "Notice",
            "Saved",
            NativeDialogLevel::Info,
        );
        application
            .dialog_host()
            .set_next_response(NativeDialogResponse::Yes);
        let response = application.dialog_host().confirm(
            LinuxMainWindowHandle(1),
            "Confirm",
            "Continue?",
            NativeDialogLevel::Question,
            NativeDialogButtons::YesNo,
        );
        assert_eq!(response, NativeDialogResponse::Yes);
        assert_eq!(application.dialog_host().messages().len(), 1);
        assert_eq!(application.dialog_host().confirmations().len(), 1);

        let identity = application.window_identity_host();
        identity.set_process_name("wps");
        identity.set_class_name("WPSDocumentView");
        identity.set_root_handle(Some(LinuxWindowIdentityHandle(9)));
        identity.set_foreground_handle(Some(LinuxWindowIdentityHandle(10)));
        identity.set_existing_windows(vec![LinuxWindowIdentityHandle(7)]);
        identity.set_current_process_windows(vec![LinuxWindowIdentityHandle(11)]);

        assert_eq!(
            identity.process_name(LinuxWindowIdentityHandle(7)),
            "wps".to_string()
        );
        assert_eq!(
            identity.class_name(LinuxWindowIdentityHandle(7)),
            "WPSDocumentView".to_string()
        );
        assert_eq!(
            identity.root_handle(LinuxWindowIdentityHandle(7)),
            LinuxWindowIdentityHandle(9)
        );
        assert_eq!(identity.foreground_handle(), LinuxWindowIdentityHandle(10));
        assert!(identity.exists(LinuxWindowIdentityHandle(7)));
        assert!(identity.is_foreground(LinuxWindowIdentityHandle(10)));
        assert!(identity.is_current_process_window(LinuxWindowIdentityHandle(11)));
        assert_eq!(identity.actions().len(), 7);
    }

    #[test]
    fn linux_system_service_hosts_consume_shared_shell_and_dialog_contracts() {
        let application = LinuxApplicationModel::default();

        application
            .shell_open_host()
            .open_path("file:///home/user/report.txt");
        assert_eq!(
            application.shell_open_host().opened_paths(),
            vec!["file:///home/user/report.txt".to_string()]
        );

        application
            .file_dialog_host()
            .set_next_result(Ok(Some("/home/user/input.xlsx".to_string())));
        let picked = application
            .file_dialog_host()
            .pick_file(NativeFileDialogRequest {
                title: "Pick file",
                filter_name: "Excel",
                filter_pattern: "*.xlsx",
                current_path: "/home/user",
            })
            .unwrap();
        assert_eq!(picked, Some("/home/user/input.xlsx".to_string()));
        assert_eq!(application.file_dialog_host().requests().len(), 1);
        assert_eq!(
            application.file_dialog_host().requests()[0].filter_pattern,
            "*.xlsx"
        );

        application
            .text_input_dialog_host()
            .set_next_result(Some("Linux group".to_string()));
        let text = application.text_input_dialog_host().prompt_text(
            LinuxMainWindowHandle(9),
            NativeTextInputDialogRequest {
                title: "Group",
                label: "Name",
                initial: "old",
            },
        );
        assert_eq!(text, Some("Linux group".to_string()));
        assert_eq!(application.text_input_dialog_host().requests().len(), 1);
        assert_eq!(
            application.text_input_dialog_host().requests()[0].owner,
            LinuxMainWindowHandle(9)
        );

        application.edit_text_dialog_host().set_next_result(
            Some("saved".to_string()),
            Some(Size {
                width: 320,
                height: 240,
            }),
        );
        let mut saved_text = String::new();
        let result = application.edit_text_dialog_host().open_edit_text(
            LinuxMainWindowHandle(10),
            NativeEditTextDialogRequest {
                title: "Edit",
                initial_text: "draft",
                initial_size: None,
            },
            &mut |text: &str| {
                saved_text = text.to_string();
                Ok(())
            },
        );
        assert!(result.saved);
        assert_eq!(
            result.final_size,
            Some(Size {
                width: 320,
                height: 240
            })
        );
        assert_eq!(saved_text, "saved");
        assert_eq!(application.edit_text_dialog_host().requests().len(), 1);
    }

    #[test]
    fn linux_mail_merge_window_host_consumes_shared_open_contract() {
        let application = LinuxApplicationModel::default();
        application.mail_merge_window_host().open_mail_merge(
            LinuxMainWindowHandle(12),
            NativeMailMergeWindowRequest {
                initial_excel_path: Some("/tmp/input.xlsx"),
            },
        );

        assert_eq!(
            application.mail_merge_window_host().requests(),
            vec![LinuxMailMergeWindowRecord {
                owner: LinuxMainWindowHandle(12),
                initial_excel_path: Some("/tmp/input.xlsx".to_string()),
            }]
        );
    }

    #[test]
    fn linux_paste_target_host_consumes_shared_target_contract() {
        let mut application = LinuxApplicationModel::default();
        let host = application.paste_target_host_mut();
        host.set_next_foreground_result(true);
        host.set_next_text_input_capabilities(PasteTargetTextInputCapabilities::text_input());
        host.set_next_focus_status(PasteTargetFocusStatus::InsideTarget);
        host.set_next_text_input_ready(true);

        assert!(host.force_paste_target_foreground(LinuxPasteTargetHandle(7)));
        host.restore_paste_target_focus(LinuxPasteTargetHandle(7), LinuxPasteTargetHandle(8));
        assert!(host.set_paste_target_text(LinuxPasteTargetHandle(7), "hello"));
        assert!(host
            .paste_target_text_input_capabilities(LinuxPasteTargetHandle(7))
            .accepts_text_input());
        assert_eq!(
            host.paste_target_focus_status(LinuxPasteTargetHandle(7), LinuxPasteTargetHandle(8)),
            PasteTargetFocusStatus::InsideTarget
        );
        assert!(host.paste_target_text_input_ready(LinuxPasteTargetHandle(7)));
        assert!(host.send_paste_shortcut(LinuxPasteTargetHandle(7)));
        assert_eq!(host.actions().len(), 7);
        assert!(host
            .actions()
            .contains(&LinuxPasteTargetAction::SendPasteShortcut(
                LinuxPasteTargetHandle(7)
            )));
    }

    #[test]
    fn linux_text_caret_host_consumes_shared_anchor_contract() {
        let mut application = LinuxApplicationModel::default();
        let host = application.text_caret_host();
        host.set_next_accessible(Some(NativeTextCaretAnchor::new(10, 20, 36)));
        host.set_next_thread(Some(NativeTextCaretAnchor::new(11, 21, 37)));
        host.set_next_focus_rect(Some(NativeTextCaretAnchor::new(12, 22, 38)));
        host.set_next_cursor(Some(NativeTextCaretAnchor::new(13, 23, 23)));
        host.set_next_focus_handle(Some(LinuxTextCaretHandle(8)));

        let host = application.text_caret_host_mut();
        assert_eq!(
            host.accessible_caret_anchor(LinuxTextCaretHandle(3)),
            Some(NativeTextCaretAnchor::new(10, 20, 36))
        );
        assert_eq!(
            host.thread_caret_anchor(LinuxTextCaretHandle(4)),
            Some(NativeTextCaretAnchor::new(11, 21, 37))
        );
        assert_eq!(
            host.focus_rect_anchor(LinuxTextCaretHandle(5), 260, 180),
            Some(NativeTextCaretAnchor::new(12, 22, 38))
        );
        assert_eq!(
            host.cursor_anchor(),
            Some(NativeTextCaretAnchor::new(13, 23, 23))
        );
        assert_eq!(
            host.focus_handle_for_target(LinuxTextCaretHandle(6)),
            LinuxTextCaretHandle(8)
        );
        assert_eq!(host.actions().len(), 5);
    }

    #[test]
    fn linux_renderer_consumes_shared_render_contract() {
        let mut application = LinuxApplicationModel::default();
        let rect = Rect {
            x: 1,
            y: 2,
            width: 30,
            height: 40,
        };
        let color = Color {
            r: 10,
            g: 20,
            b: 30,
            a: 255,
        };
        let text_style = TextStyle::line("Sans", 14.0, color);
        let run = TextRun {
            text: "ZSUI".to_string(),
            bounds: rect,
        };

        application.renderer.fill_rect(rect, color);
        application.renderer.stroke_rect(rect, color, 2);
        application.renderer.draw_text(&run, &text_style);
        application.renderer.push_clip(rect);
        application.renderer.pop_clip();

        assert_eq!(application.renderer().commands().len(), 5);
        assert!(matches!(
            application.renderer().commands()[0],
            LinuxRenderCommand::FillRect(_, _)
        ));
        assert!(matches!(
            application.renderer().commands()[4],
            LinuxRenderCommand::PopClip
        ));
    }

    #[test]
    fn linux_main_search_host_consumes_shared_search_contract() {
        let mut application = LinuxApplicationModel::default();
        let request = NativeMainSearchControlRequest {
            owner: LinuxMainWindowHandle(1),
            id: 100,
            bounds: UiRect::new(8, 8, 280, 40),
            visible: true,
        };
        let presentation = application
            .main_search_host_mut()
            .create_search_control(request);
        let NativeMainSearchControlPresentation::Created(handle) = presentation else {
            panic!("Linux search scaffold should create a search handle");
        };
        assert_eq!(application.main_search_host.search_count(), 1);

        let style =
            application
                .main_search_host_mut()
                .apply_search_style(NativeMainSearchStyleRequest {
                    handle,
                    font_family: "Sans".to_string(),
                    font_px: 14,
                    previous_resource: None,
                });
        let NativeMainSearchStylePresentation::Applied(Some(resource)) = style else {
            panic!("Linux search scaffold should record a style resource");
        };
        assert_eq!(application.main_search_host.style_request_count(), 1);

        application
            .main_search_host_mut()
            .set_search_text(handle, "rust");
        application
            .main_search_host_mut()
            .set_search_bounds(handle, UiRect::new(12, 12, 320, 44));
        application
            .main_search_host_mut()
            .set_search_visible(handle, false);
        application.main_search_host_mut().focus_search(handle);
        assert_eq!(application.main_search_host.search_text(handle), "rust");
        application
            .main_search_host_mut()
            .release_search_style_resource(resource);
        assert_eq!(
            application.main_search_host.released_style_resources,
            vec![resource]
        );
    }

    #[test]
    fn linux_settings_window_host_consumes_shared_settings_contract() {
        let mut application = LinuxApplicationModel::default();
        let startup = application.mount("ZSClip Linux", true).unwrap();
        let presentation = application
            .main_window_host_mut()
            .create_main_windows(startup.request);
        let NativeMainWindowPresentation::Created(handles) = presentation else {
            panic!("Linux scaffold should produce main window handles");
        };

        let settings_request = NativeSettingsWindowRequest {
            owner: handles.main,
            existing: None,
            bounds: UiRect::new(80, 80, 840, 600),
        };
        let settings_presentation = application
            .settings_window_host_mut()
            .present_settings_window(settings_request);
        let NativeSettingsWindowPresentation::Created(settings_handle) = settings_presentation
        else {
            panic!("Linux settings scaffold should create a settings handle");
        };
        assert_eq!(application.settings_window_host.present_request_count(), 1);
        assert_eq!(
            application.settings_window_host.presented_handle(),
            Some(settings_handle)
        );

        application
            .settings_window_host_mut()
            .focus_settings_window(settings_handle);
        application
            .settings_window_host_mut()
            .set_settings_window_bounds(settings_handle, UiRect::new(100, 100, 860, 620));
        application
            .settings_window_host_mut()
            .request_settings_window_area_repaint(
                settings_handle,
                Some(UiRect::new(0, 0, 32, 32)),
                false,
            );
        assert_eq!(application.settings_window_host.focused_count(), 1);
        assert_eq!(application.settings_window_host.repaint_request_count(), 1);

        let existing_request = NativeSettingsWindowRequest {
            owner: handles.main,
            existing: Some(settings_handle),
            bounds: UiRect::new(100, 100, 860, 620),
        };
        assert_eq!(
            application
                .settings_window_host_mut()
                .present_settings_window(existing_request),
            NativeSettingsWindowPresentation::FocusedExisting(settings_handle)
        );
    }

    #[test]
    fn linux_settings_control_host_consumes_shared_control_contract() {
        let mut application = LinuxApplicationModel::default();
        let button = SettingsControlSpec::action(
            crate::app_core::SettingsComponentKind::Button,
            42,
            "Apply",
            UiRect::new(10, 10, 110, 42),
        );
        let input = SettingsControlSpec::text_input(43, "initial", UiRect::new(10, 50, 210, 82));

        let button_handle = application
            .settings_control_host_mut()
            .create_control(&button);
        let input_handle = application
            .settings_control_host_mut()
            .create_control(&input);
        assert_eq!(application.settings_control_host.control_count(), 2);
        assert!(application
            .settings_control_host
            .control_exists(button_handle));
        assert_eq!(
            application
                .settings_control_host
                .control_at_point(Point { x: 20, y: 20 }),
            Some(button_handle)
        );

        application
            .settings_control_host_mut()
            .set_control_text(input_handle, "changed");
        assert_eq!(
            application.settings_control_host.control_text(input_handle),
            "changed"
        );
        assert!(application
            .settings_control_host_mut()
            .request_control_repaint(button_handle));
        assert_eq!(
            application
                .settings_control_host
                .repaint_count(button_handle),
            1
        );

        application
            .settings_control_host_mut()
            .set_control_visible(button_handle, false);
        assert_eq!(
            application
                .settings_control_host
                .control_at_point(Point { x: 20, y: 20 }),
            None
        );
        application
            .settings_control_host_mut()
            .destroy_control(input_handle);
        assert_eq!(application.settings_control_host.control_count(), 1);
    }

    #[test]
    fn linux_settings_dropdown_host_consumes_shared_dropdown_contract() {
        let mut application = LinuxApplicationModel::default();
        let owner = LinuxMainWindowHandle(7);
        let request = NativeSettingsDropdownRequest {
            owner,
            control_id: 42,
            anchor: UiRect::new(20, 30, 160, 60),
            items: vec!["Auto".to_string(), "Manual".to_string()],
            selected: 1,
            width: 180,
        };

        let presentation = application
            .settings_dropdown_host_mut()
            .present_settings_dropdown(request);
        let NativeSettingsDropdownPresentation::Created(handle) = presentation else {
            panic!("Linux dropdown scaffold should create a dropdown handle");
        };
        assert_eq!(application.settings_dropdown_host.dropdown_count(), 1);
        assert_eq!(
            application
                .settings_dropdown_host
                .settings_dropdown_bounds(handle),
            Some(UiRect::new(20, 30, 200, 118))
        );

        application
            .settings_dropdown_host_mut()
            .destroy_settings_dropdown(handle);
        assert_eq!(application.settings_dropdown_host.dropdown_count(), 0);
        assert_eq!(
            application
                .settings_dropdown_host
                .settings_dropdown_bounds(handle),
            None
        );
    }

    #[test]
    fn linux_ai_action_presenter_tracks_menu_settings_and_execution_routes() {
        let mut application = LinuxApplicationModel::default();
        let capabilities: Vec<_> = crate::app_core::product_ai_capability_catalog()
            .iter()
            .take(2)
            .map(|descriptor| descriptor.capability())
            .collect();

        let menu_request = NativeAiActionMenuRequest {
            owner: LinuxMainWindowHandle(1),
            surface: ProductAiUiSurface::RowContextMenu,
            anchor: Point { x: 14, y: 22 },
            capabilities: capabilities.clone(),
            context_item_ids: vec![77],
            prompt_text: "summarize this".to_string(),
        };
        let invocation = application
            .present_ai_action_menu(menu_request)
            .expect("Linux AI presenter should select the first capability");

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
            owner: Some(LinuxMainWindowHandle(1)),
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
}

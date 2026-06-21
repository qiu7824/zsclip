use super::command_protocol::Command;
use super::product_adapter::ApplicationEvent;
use super::{Point, Size, UiRect};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeRuntimeStartupRequest {
    pub(crate) app_name: String,
    pub(crate) main_window: NativeMainWindowRequest,
    pub(crate) status_item_tooltip: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeRuntimeStartupResult<Handle: Copy + Eq> {
    Started(NativeMainWindowHandles<Handle>),
    Failed,
}

pub(crate) trait NativeRuntimeDriver {
    type WindowHandle: Copy + Eq;

    fn start_runtime(
        &mut self,
        request: NativeRuntimeStartupRequest,
    ) -> NativeRuntimeStartupResult<Self::WindowHandle>;
    fn dispatch_ui_command(&mut self, command: Command);
    fn poll_application_event(&mut self) -> Option<ApplicationEvent>;
    fn request_shutdown(&mut self);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeRuntimeDriverOperation {
    StartRuntime,
    DispatchUiCommand,
    PollApplicationEvent,
    RequestShutdown,
}

impl NativeRuntimeDriverOperation {
    pub(crate) const fn operation_name(self) -> &'static str {
        match self {
            Self::StartRuntime => "start_runtime",
            Self::DispatchUiCommand => "dispatch_ui_command",
            Self::PollApplicationEvent => "poll_application_event",
            Self::RequestShutdown => "request_shutdown",
        }
    }
}

pub(crate) const REQUIRED_NATIVE_RUNTIME_DRIVER_OPERATIONS: [NativeRuntimeDriverOperation; 4] = [
    NativeRuntimeDriverOperation::StartRuntime,
    NativeRuntimeDriverOperation::DispatchUiCommand,
    NativeRuntimeDriverOperation::PollApplicationEvent,
    NativeRuntimeDriverOperation::RequestShutdown,
];

pub(crate) fn required_native_runtime_driver_operation_names() -> Vec<&'static str> {
    REQUIRED_NATIVE_RUNTIME_DRIVER_OPERATIONS
        .iter()
        .map(|operation| operation.operation_name())
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeMainWindowRequest {
    pub(crate) title: String,
    pub(crate) size: Size,
    pub(crate) main_visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeMainWindowHandles<Handle: Copy + Eq> {
    pub(crate) main: Handle,
    pub(crate) quick: Handle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeMainWindowPresentation<Handle: Copy + Eq> {
    Created(NativeMainWindowHandles<Handle>),
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeMainWindowPresentMode {
    ActivateAndFocus,
    NoActivate,
}

pub(crate) trait NativeMainWindowHost {
    type Handle: Copy + Eq;
    type AppIcon: Copy + Eq;

    fn create_main_windows(
        &mut self,
        request: NativeMainWindowRequest,
    ) -> NativeMainWindowPresentation<Self::Handle>;
    fn apply_main_window_appearance(&mut self, handle: Self::Handle);
    fn set_main_window_app_icon(
        &mut self,
        handle: Self::Handle,
        icon: NativeAppIconResource<Self::AppIcon>,
    );
    fn hide_main_window(&mut self, handle: Self::Handle);
    fn present_main_window(&mut self, handle: Self::Handle, mode: NativeMainWindowPresentMode);
    fn set_main_window_bounds(&mut self, handle: Self::Handle, bounds: UiRect);
    fn activate_main_window(&mut self, handle: Self::Handle);
    fn foreground_main_window(&mut self, handle: Self::Handle);
    fn restore_main_window(&mut self, handle: Self::Handle);
    fn close_main_window(&mut self, handle: Self::Handle);
    fn set_main_window_activation_policy(&mut self, handle: Self::Handle, allow_activation: bool);
    fn request_main_window_close(&mut self, handle: Self::Handle);
    fn destroy_main_window(&mut self, handle: Self::Handle);
    fn capture_main_pointer(&mut self, handle: Self::Handle);
    fn release_main_pointer(&mut self, handle: Self::Handle);
    fn begin_main_window_drag(&mut self, handle: Self::Handle);
    fn track_main_pointer_leave(&mut self, handle: Self::Handle) -> bool;
    fn request_main_window_area_repaint(
        &mut self,
        handle: Self::Handle,
        area: Option<UiRect>,
        erase: bool,
    ) -> bool;
    fn main_window_layout_dpi(&mut self, handle: Self::Handle) -> u32;
    fn main_window_client_bounds(&mut self, handle: Self::Handle) -> Option<UiRect>;
    fn main_window_bounds(&mut self, handle: Self::Handle) -> Option<UiRect>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeMainWindowHostOperation {
    CreateMainWindows,
    ApplyMainWindowAppearance,
    SetMainWindowAppIcon,
    HideMainWindow,
    PresentMainWindow,
    SetMainWindowBounds,
    ActivateMainWindow,
    ForegroundMainWindow,
    RestoreMainWindow,
    CloseMainWindow,
    SetMainWindowActivationPolicy,
    RequestMainWindowClose,
    DestroyMainWindow,
    CaptureMainPointer,
    ReleaseMainPointer,
    BeginMainWindowDrag,
    TrackMainPointerLeave,
    RequestMainWindowAreaRepaint,
    MainWindowLayoutDpi,
    MainWindowClientBounds,
    MainWindowBounds,
}

impl NativeMainWindowHostOperation {
    pub(crate) const fn operation_name(self) -> &'static str {
        match self {
            Self::CreateMainWindows => "create_main_windows",
            Self::ApplyMainWindowAppearance => "apply_main_window_appearance",
            Self::SetMainWindowAppIcon => "set_main_window_app_icon",
            Self::HideMainWindow => "hide_main_window",
            Self::PresentMainWindow => "present_main_window",
            Self::SetMainWindowBounds => "set_main_window_bounds",
            Self::ActivateMainWindow => "activate_main_window",
            Self::ForegroundMainWindow => "foreground_main_window",
            Self::RestoreMainWindow => "restore_main_window",
            Self::CloseMainWindow => "close_main_window",
            Self::SetMainWindowActivationPolicy => "set_main_window_activation_policy",
            Self::RequestMainWindowClose => "request_main_window_close",
            Self::DestroyMainWindow => "destroy_main_window",
            Self::CaptureMainPointer => "capture_main_pointer",
            Self::ReleaseMainPointer => "release_main_pointer",
            Self::BeginMainWindowDrag => "begin_main_window_drag",
            Self::TrackMainPointerLeave => "track_main_pointer_leave",
            Self::RequestMainWindowAreaRepaint => "request_main_window_area_repaint",
            Self::MainWindowLayoutDpi => "main_window_layout_dpi",
            Self::MainWindowClientBounds => "main_window_client_bounds",
            Self::MainWindowBounds => "main_window_bounds",
        }
    }
}

pub(crate) const REQUIRED_NATIVE_MAIN_WINDOW_HOST_OPERATIONS: [NativeMainWindowHostOperation; 21] = [
    NativeMainWindowHostOperation::CreateMainWindows,
    NativeMainWindowHostOperation::ApplyMainWindowAppearance,
    NativeMainWindowHostOperation::SetMainWindowAppIcon,
    NativeMainWindowHostOperation::HideMainWindow,
    NativeMainWindowHostOperation::PresentMainWindow,
    NativeMainWindowHostOperation::SetMainWindowBounds,
    NativeMainWindowHostOperation::ActivateMainWindow,
    NativeMainWindowHostOperation::ForegroundMainWindow,
    NativeMainWindowHostOperation::RestoreMainWindow,
    NativeMainWindowHostOperation::CloseMainWindow,
    NativeMainWindowHostOperation::SetMainWindowActivationPolicy,
    NativeMainWindowHostOperation::RequestMainWindowClose,
    NativeMainWindowHostOperation::DestroyMainWindow,
    NativeMainWindowHostOperation::CaptureMainPointer,
    NativeMainWindowHostOperation::ReleaseMainPointer,
    NativeMainWindowHostOperation::BeginMainWindowDrag,
    NativeMainWindowHostOperation::TrackMainPointerLeave,
    NativeMainWindowHostOperation::RequestMainWindowAreaRepaint,
    NativeMainWindowHostOperation::MainWindowLayoutDpi,
    NativeMainWindowHostOperation::MainWindowClientBounds,
    NativeMainWindowHostOperation::MainWindowBounds,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeAppIconResource<Icon: Copy + Eq> {
    pub(crate) small: Icon,
    pub(crate) big: Icon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeMainSearchControlRequest<Owner: Copy + Eq> {
    pub(crate) owner: Owner,
    pub(crate) id: i64,
    pub(crate) bounds: UiRect,
    pub(crate) visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeMainSearchStyleRequest<Handle: Copy + Eq, StyleResource: Copy + Eq> {
    pub(crate) handle: Handle,
    pub(crate) font_family: String,
    pub(crate) font_px: i32,
    pub(crate) previous_resource: Option<StyleResource>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeMainSearchStylePresentation<StyleResource: Copy + Eq> {
    Applied(Option<StyleResource>),
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeMainSearchControlPresentation<Handle: Copy + Eq> {
    Created(Handle),
    Failed,
}

pub(crate) trait NativeMainSearchControlHost {
    type Owner: Copy + Eq;
    type Handle: Copy + Eq;
    type StyleResource: Copy + Eq;

    fn create_search_control(
        &mut self,
        request: NativeMainSearchControlRequest<Self::Owner>,
    ) -> NativeMainSearchControlPresentation<Self::Handle>;
    fn apply_search_style(
        &mut self,
        request: NativeMainSearchStyleRequest<Self::Handle, Self::StyleResource>,
    ) -> NativeMainSearchStylePresentation<Self::StyleResource>;
    fn release_search_style_resource(&mut self, resource: Self::StyleResource);
    fn set_search_bounds(&mut self, handle: Self::Handle, bounds: UiRect);
    fn set_search_visible(&mut self, handle: Self::Handle, visible: bool);
    fn search_text(&self, handle: Self::Handle) -> String;
    fn set_search_text(&mut self, handle: Self::Handle, text: &str);
    fn focus_search(&mut self, handle: Self::Handle);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeMainSearchControlHostOperation {
    CreateSearchControl,
    ApplySearchStyle,
    ReleaseSearchStyleResource,
    SetSearchBounds,
    SetSearchVisible,
    SearchText,
    SetSearchText,
    FocusSearch,
}

impl NativeMainSearchControlHostOperation {
    pub(crate) const fn operation_name(self) -> &'static str {
        match self {
            Self::CreateSearchControl => "create_search_control",
            Self::ApplySearchStyle => "apply_search_style",
            Self::ReleaseSearchStyleResource => "release_search_style_resource",
            Self::SetSearchBounds => "set_search_bounds",
            Self::SetSearchVisible => "set_search_visible",
            Self::SearchText => "search_text",
            Self::SetSearchText => "set_search_text",
            Self::FocusSearch => "focus_search",
        }
    }
}

pub(crate) const REQUIRED_NATIVE_MAIN_SEARCH_CONTROL_HOST_OPERATIONS:
    [NativeMainSearchControlHostOperation; 8] = [
    NativeMainSearchControlHostOperation::CreateSearchControl,
    NativeMainSearchControlHostOperation::ApplySearchStyle,
    NativeMainSearchControlHostOperation::ReleaseSearchStyleResource,
    NativeMainSearchControlHostOperation::SetSearchBounds,
    NativeMainSearchControlHostOperation::SetSearchVisible,
    NativeMainSearchControlHostOperation::SearchText,
    NativeMainSearchControlHostOperation::SetSearchText,
    NativeMainSearchControlHostOperation::FocusSearch,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeSettingsWindowRequest<Handle: Copy + Eq> {
    pub(crate) owner: Handle,
    pub(crate) existing: Option<Handle>,
    pub(crate) bounds: UiRect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeSettingsWindowPresentation<Handle: Copy + Eq> {
    FocusedExisting(Handle),
    Created(Handle),
    Failed,
}

pub(crate) trait NativeSettingsWindowHost {
    type Handle: Copy + Eq;

    fn present_settings_window(
        &mut self,
        request: NativeSettingsWindowRequest<Self::Handle>,
    ) -> NativeSettingsWindowPresentation<Self::Handle>;
    fn set_settings_window_bounds(&mut self, handle: Self::Handle, bounds: UiRect);
    fn destroy_settings_window(&mut self, handle: Self::Handle);
    fn focus_settings_window(&mut self, handle: Self::Handle);
    fn track_settings_pointer_leave(&mut self, handle: Self::Handle) -> bool;
    fn capture_settings_pointer(&mut self, handle: Self::Handle);
    fn release_settings_pointer(&mut self, handle: Self::Handle);
    fn request_settings_window_repaint(&mut self, handle: Self::Handle) -> bool;
    fn request_settings_window_area_repaint(
        &mut self,
        handle: Self::Handle,
        area: Option<UiRect>,
        erase: bool,
    ) -> bool;
    fn settings_window_layout_dpi(&mut self, handle: Self::Handle) -> u32;
    fn settings_window_client_to_screen(
        &mut self,
        handle: Self::Handle,
        point: Point,
    ) -> Option<Point>;
    fn settings_window_client_bounds(&mut self, handle: Self::Handle) -> Option<UiRect>;
    fn settings_window_bounds(&mut self, handle: Self::Handle) -> Option<UiRect>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeSettingsWindowHostOperation {
    PresentSettingsWindow,
    SetSettingsWindowBounds,
    DestroySettingsWindow,
    FocusSettingsWindow,
    TrackSettingsPointerLeave,
    CaptureSettingsPointer,
    ReleaseSettingsPointer,
    RequestSettingsWindowRepaint,
    RequestSettingsWindowAreaRepaint,
    SettingsWindowLayoutDpi,
    SettingsWindowClientToScreen,
    SettingsWindowClientBounds,
    SettingsWindowBounds,
}

impl NativeSettingsWindowHostOperation {
    pub(crate) const fn operation_name(self) -> &'static str {
        match self {
            Self::PresentSettingsWindow => "present_settings_window",
            Self::SetSettingsWindowBounds => "set_settings_window_bounds",
            Self::DestroySettingsWindow => "destroy_settings_window",
            Self::FocusSettingsWindow => "focus_settings_window",
            Self::TrackSettingsPointerLeave => "track_settings_pointer_leave",
            Self::CaptureSettingsPointer => "capture_settings_pointer",
            Self::ReleaseSettingsPointer => "release_settings_pointer",
            Self::RequestSettingsWindowRepaint => "request_settings_window_repaint",
            Self::RequestSettingsWindowAreaRepaint => "request_settings_window_area_repaint",
            Self::SettingsWindowLayoutDpi => "settings_window_layout_dpi",
            Self::SettingsWindowClientToScreen => "settings_window_client_to_screen",
            Self::SettingsWindowClientBounds => "settings_window_client_bounds",
            Self::SettingsWindowBounds => "settings_window_bounds",
        }
    }
}

pub(crate) const REQUIRED_NATIVE_SETTINGS_WINDOW_HOST_OPERATIONS:
    [NativeSettingsWindowHostOperation; 13] = [
    NativeSettingsWindowHostOperation::PresentSettingsWindow,
    NativeSettingsWindowHostOperation::SetSettingsWindowBounds,
    NativeSettingsWindowHostOperation::DestroySettingsWindow,
    NativeSettingsWindowHostOperation::FocusSettingsWindow,
    NativeSettingsWindowHostOperation::TrackSettingsPointerLeave,
    NativeSettingsWindowHostOperation::CaptureSettingsPointer,
    NativeSettingsWindowHostOperation::ReleaseSettingsPointer,
    NativeSettingsWindowHostOperation::RequestSettingsWindowRepaint,
    NativeSettingsWindowHostOperation::RequestSettingsWindowAreaRepaint,
    NativeSettingsWindowHostOperation::SettingsWindowLayoutDpi,
    NativeSettingsWindowHostOperation::SettingsWindowClientToScreen,
    NativeSettingsWindowHostOperation::SettingsWindowClientBounds,
    NativeSettingsWindowHostOperation::SettingsWindowBounds,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeSettingsDropdownRequest<Owner: Copy + Eq> {
    pub(crate) owner: Owner,
    pub(crate) control_id: isize,
    pub(crate) anchor: UiRect,
    pub(crate) items: Vec<String>,
    pub(crate) selected: usize,
    pub(crate) width: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeSettingsDropdownPresentation<Handle: Copy + Eq> {
    Created(Handle),
    Failed,
}

pub(crate) trait NativeSettingsDropdownHost {
    type Handle: Copy + Eq;
    type Owner: Copy + Eq;

    fn present_settings_dropdown(
        &mut self,
        request: NativeSettingsDropdownRequest<Self::Owner>,
    ) -> NativeSettingsDropdownPresentation<Self::Handle>;
    fn destroy_settings_dropdown(&mut self, handle: Self::Handle);
    fn settings_dropdown_bounds(&self, handle: Self::Handle) -> Option<UiRect>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeSettingsDropdownHostOperation {
    PresentSettingsDropdown,
    DestroySettingsDropdown,
    SettingsDropdownBounds,
}

impl NativeSettingsDropdownHostOperation {
    pub(crate) const fn operation_name(self) -> &'static str {
        match self {
            Self::PresentSettingsDropdown => "present_settings_dropdown",
            Self::DestroySettingsDropdown => "destroy_settings_dropdown",
            Self::SettingsDropdownBounds => "settings_dropdown_bounds",
        }
    }
}

pub(crate) const REQUIRED_NATIVE_SETTINGS_DROPDOWN_HOST_OPERATIONS:
    [NativeSettingsDropdownHostOperation; 3] = [
    NativeSettingsDropdownHostOperation::PresentSettingsDropdown,
    NativeSettingsDropdownHostOperation::DestroySettingsDropdown,
    NativeSettingsDropdownHostOperation::SettingsDropdownBounds,
];

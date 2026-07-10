#![allow(unused_imports)]

use super::command_protocol::Command;
use super::product_adapter::ApplicationEvent;

pub(crate) use zsui::{
    required_native_runtime_driver_operation_names, NativeAppIconResource,
    NativeMainSearchControlHost, NativeMainSearchControlHostOperation,
    NativeMainSearchControlPresentation, NativeMainSearchControlRequest,
    NativeMainSearchStylePresentation, NativeMainSearchStyleRequest, NativeMainWindowHandles,
    NativeMainWindowHost, NativeMainWindowHostOperation, NativeMainWindowPresentMode,
    NativeMainWindowPresentation, NativeMainWindowRequest, NativeRuntimeDriverOperation,
    NativeRuntimeStartupRequest, NativeRuntimeStartupResult, NativeSettingsDropdownHost,
    NativeSettingsDropdownHostOperation, NativeSettingsDropdownPresentation,
    NativeSettingsDropdownRequest, NativeSettingsWindowHost, NativeSettingsWindowHostOperation,
    NativeSettingsWindowPresentation, NativeSettingsWindowRequest, NativeWindowOptions,
    REQUIRED_NATIVE_MAIN_SEARCH_CONTROL_HOST_OPERATIONS,
    REQUIRED_NATIVE_MAIN_WINDOW_HOST_OPERATIONS, REQUIRED_NATIVE_RUNTIME_DRIVER_OPERATIONS,
    REQUIRED_NATIVE_SETTINGS_DROPDOWN_HOST_OPERATIONS,
    REQUIRED_NATIVE_SETTINGS_WINDOW_HOST_OPERATIONS,
};

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

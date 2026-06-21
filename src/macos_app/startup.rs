use crate::app_core::{
    LifecycleEvent, NativeHostLaunchMode, NativeHostLaunchPlan, NativeMainWindowRequest,
    NativeUiPlatform, NativeUiToolkit,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MacosStartupPlan {
    pub(crate) main_window: NativeMainWindowRequest,
    pub(crate) lifecycle: LifecycleEvent,
}

pub(crate) fn macos_native_host_launch_plan() -> NativeHostLaunchPlan {
    let mode = if crate::macos_native_host::real_appkit_host_is_compiled() {
        NativeHostLaunchMode::RealNativeHost
    } else {
        NativeHostLaunchMode::ContractScaffoldFallback
    };
    NativeHostLaunchPlan {
        platform: NativeUiPlatform::Macos,
        toolkit: NativeUiToolkit::AppKitSwiftUI,
        entry_point: "macos_app::run",
        native_application_type: "objc2_app_kit::NSApplication",
        native_window_type: "objc2_app_kit::NSWindow",
        real_host_module_path: "src/macos_native_host.rs",
        fallback_module_path: "src/macos_app.rs",
        mode,
        target_os_verification_required: true,
    }
}

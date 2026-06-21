use crate::app_core::{
    LifecycleEvent, NativeHostLaunchMode, NativeHostLaunchPlan, NativeMainWindowRequest,
    NativeUiPlatform, NativeUiToolkit,
};

use super::LinuxNativeBackend;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinuxStartupPlan {
    pub(crate) request: NativeMainWindowRequest,
    pub(crate) lifecycle: LifecycleEvent,
    pub(crate) backend: LinuxNativeBackend,
}

pub(crate) fn linux_native_host_launch_plan() -> NativeHostLaunchPlan {
    let mode = if crate::linux_native_host::real_gtk_host_is_compiled() {
        NativeHostLaunchMode::RealNativeHost
    } else {
        NativeHostLaunchMode::ContractScaffoldFallback
    };
    NativeHostLaunchPlan {
        platform: NativeUiPlatform::Linux,
        toolkit: NativeUiToolkit::Gtk4Libadwaita,
        entry_point: "linux_app::run",
        native_application_type: "gtk4::Application",
        native_window_type: "gtk4::ApplicationWindow",
        real_host_module_path: "src/linux_native_host.rs",
        fallback_module_path: "src/linux_app.rs",
        mode,
        target_os_verification_required: true,
    }
}

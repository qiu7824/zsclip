#![allow(unused_imports)]

pub(crate) use zsui::{
    native_ui_adapter_parity_report, NativeUiAdapterBindingPlan, NativeUiAdapterCapability,
    NativeUiAdapterManifest, NativeUiAdapterParityReport, NativeUiAdapterReusePackage,
    NativeUiBackendCapabilityMatrix, NativeUiBackendDescriptor, NativeUiBackendStatus,
    NativeUiPlatform, NativeUiToolkit, REQUIRED_NATIVE_UI_ADAPTER_CAPABILITIES,
};

pub(crate) const SUPPORTED_NATIVE_UI_PLATFORMS: [NativeUiPlatform; 3] = [
    NativeUiPlatform::Windows,
    NativeUiPlatform::Macos,
    NativeUiPlatform::Linux,
];

pub(crate) const SUPPORTED_NATIVE_UI_TOOLKITS: [NativeUiToolkit; 3] = [
    NativeUiToolkit::Win32Gdi,
    NativeUiToolkit::AppKitSwiftUI,
    NativeUiToolkit::Gtk4Libadwaita,
];

pub(crate) const SUPPORTED_NATIVE_UI_BACKENDS: [NativeUiBackendDescriptor; 3] = [
    NativeUiBackendDescriptor {
        platform: NativeUiPlatform::Windows,
        toolkit: NativeUiToolkit::Win32Gdi,
        status: NativeUiBackendStatus::NativeHostIntegrated,
        adapter_boundary: "WindowsWin32AdapterBoundary",
        module_path: "src/windows_win32_adapter.rs",
    },
    NativeUiBackendDescriptor {
        platform: NativeUiPlatform::Macos,
        toolkit: NativeUiToolkit::AppKitSwiftUI,
        status: NativeUiBackendStatus::NativeHostFirstPass,
        adapter_boundary: "MacosAppKitAdapterBoundary",
        module_path: "src/macos_appkit_adapter.rs",
    },
    NativeUiBackendDescriptor {
        platform: NativeUiPlatform::Linux,
        toolkit: NativeUiToolkit::Gtk4Libadwaita,
        status: NativeUiBackendStatus::NativeHostFirstPass,
        adapter_boundary: "LinuxGtkAdapterBoundary",
        module_path: "src/linux_gtk_adapter.rs",
    },
];

pub(crate) fn native_ui_backend_for_platform(
    platform: NativeUiPlatform,
) -> Option<&'static NativeUiBackendDescriptor> {
    SUPPORTED_NATIVE_UI_BACKENDS
        .iter()
        .find(|backend| backend.platform == platform)
}

pub(crate) fn native_ui_backend_for_toolkit(
    toolkit: NativeUiToolkit,
) -> Option<&'static NativeUiBackendDescriptor> {
    SUPPORTED_NATIVE_UI_BACKENDS
        .iter()
        .find(|backend| backend.toolkit == toolkit)
}

pub(crate) fn native_ui_platform_for_current_target() -> Option<NativeUiPlatform> {
    #[cfg(target_os = "windows")]
    {
        return Some(NativeUiPlatform::Windows);
    }
    #[cfg(target_os = "macos")]
    {
        return Some(NativeUiPlatform::Macos);
    }
    #[cfg(all(target_os = "linux", not(target_env = "ohos")))]
    {
        return Some(NativeUiPlatform::Linux);
    }
    #[allow(unreachable_code)]
    None
}

pub(crate) fn native_ui_backend_for_current_target() -> Option<&'static NativeUiBackendDescriptor> {
    native_ui_backend_for_platform(native_ui_platform_for_current_target()?)
}

pub(crate) fn native_ui_backend_capability_matrix() -> Vec<NativeUiBackendCapabilityMatrix> {
    SUPPORTED_NATIVE_UI_BACKENDS
        .iter()
        .map(|backend| NativeUiBackendCapabilityMatrix {
            backend: *backend,
            required_capabilities: REQUIRED_NATIVE_UI_ADAPTER_CAPABILITIES.to_vec(),
        })
        .collect()
}

pub(crate) fn native_ui_backend_capability_matrix_for_platform(
    platform: NativeUiPlatform,
) -> Option<NativeUiBackendCapabilityMatrix> {
    native_ui_backend_for_platform(platform).map(|backend| NativeUiBackendCapabilityMatrix {
        backend: *backend,
        required_capabilities: REQUIRED_NATIVE_UI_ADAPTER_CAPABILITIES.to_vec(),
    })
}

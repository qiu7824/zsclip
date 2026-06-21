#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeUiPlatform {
    Windows,
    Macos,
    Linux,
}

impl NativeUiPlatform {
    pub(crate) const fn platform_name(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Macos => "macos",
            Self::Linux => "linux",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeUiToolkit {
    Win32Gdi,
    AppKitSwiftUI,
    Gtk4Libadwaita,
}

impl NativeUiToolkit {
    pub(crate) const fn toolkit_name(self) -> &'static str {
        match self {
            Self::Win32Gdi => "win32_gdi",
            Self::AppKitSwiftUI => "appkit_swiftui",
            Self::Gtk4Libadwaita => "gtk4_libadwaita",
        }
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeUiBackendStatus {
    NativeHostIntegrated,
    NativeHostFirstPass,
    AdapterBoundaryScaffold,
}

impl NativeUiBackendStatus {
    pub(crate) const fn status_name(self) -> &'static str {
        match self {
            Self::NativeHostIntegrated => "native_host_integrated",
            Self::NativeHostFirstPass => "native_host_first_pass",
            Self::AdapterBoundaryScaffold => "adapter_boundary_scaffold",
        }
    }

    pub(crate) const fn is_native_runtime_ready(self) -> bool {
        matches!(self, Self::NativeHostIntegrated)
    }

    pub(crate) const fn is_scaffold(self) -> bool {
        matches!(self, Self::AdapterBoundaryScaffold)
    }

    pub(crate) const fn is_first_pass_native_host(self) -> bool {
        matches!(self, Self::NativeHostFirstPass)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeUiBackendDescriptor {
    pub(crate) platform: NativeUiPlatform,
    pub(crate) toolkit: NativeUiToolkit,
    pub(crate) status: NativeUiBackendStatus,
    pub(crate) adapter_boundary: &'static str,
    pub(crate) module_path: &'static str,
}

impl NativeUiBackendDescriptor {
    pub(crate) const fn platform_name(&self) -> &'static str {
        self.platform.platform_name()
    }

    pub(crate) const fn toolkit_name(&self) -> &'static str {
        self.toolkit.toolkit_name()
    }

    pub(crate) const fn status_name(&self) -> &'static str {
        self.status.status_name()
    }
}

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
    #[cfg(target_os = "linux")]
    {
        return Some(NativeUiPlatform::Linux);
    }
    #[allow(unreachable_code)]
    None
}

pub(crate) fn native_ui_backend_for_current_target() -> Option<&'static NativeUiBackendDescriptor> {
    native_ui_backend_for_platform(native_ui_platform_for_current_target()?)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeUiAdapterCapability {
    MainWindow,
    SettingsWindow,
    SettingsDropdown,
    InputDialog,
    EditDialog,
    Clipboard,
    PopupMenu,
    StatusItem,
    Renderer,
    TextLayout,
    MainSearchControl,
    TransientWindow,
    Ime,
    ShellOpen,
    FileDialog,
    PasteTarget,
    WindowIdentity,
    MainExecutionPlanBridge,
}

impl NativeUiAdapterCapability {
    pub(crate) const fn capability_name(self) -> &'static str {
        match self {
            Self::MainWindow => "main_window",
            Self::SettingsWindow => "settings_window",
            Self::SettingsDropdown => "settings_dropdown",
            Self::InputDialog => "input_dialog",
            Self::EditDialog => "edit_dialog",
            Self::Clipboard => "clipboard",
            Self::PopupMenu => "popup_menu",
            Self::StatusItem => "status_item",
            Self::Renderer => "renderer",
            Self::TextLayout => "text_layout",
            Self::MainSearchControl => "main_search_control",
            Self::TransientWindow => "transient_window",
            Self::Ime => "ime",
            Self::ShellOpen => "shell_open",
            Self::FileDialog => "file_dialog",
            Self::PasteTarget => "paste_target",
            Self::WindowIdentity => "window_identity",
            Self::MainExecutionPlanBridge => "main_execution_plan_bridge",
        }
    }
}

pub(crate) const REQUIRED_NATIVE_UI_ADAPTER_CAPABILITIES: [NativeUiAdapterCapability; 18] = [
    NativeUiAdapterCapability::MainWindow,
    NativeUiAdapterCapability::SettingsWindow,
    NativeUiAdapterCapability::SettingsDropdown,
    NativeUiAdapterCapability::InputDialog,
    NativeUiAdapterCapability::EditDialog,
    NativeUiAdapterCapability::Clipboard,
    NativeUiAdapterCapability::PopupMenu,
    NativeUiAdapterCapability::StatusItem,
    NativeUiAdapterCapability::Renderer,
    NativeUiAdapterCapability::TextLayout,
    NativeUiAdapterCapability::MainSearchControl,
    NativeUiAdapterCapability::TransientWindow,
    NativeUiAdapterCapability::Ime,
    NativeUiAdapterCapability::ShellOpen,
    NativeUiAdapterCapability::FileDialog,
    NativeUiAdapterCapability::PasteTarget,
    NativeUiAdapterCapability::WindowIdentity,
    NativeUiAdapterCapability::MainExecutionPlanBridge,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeUiBackendCapabilityMatrix {
    pub(crate) backend: NativeUiBackendDescriptor,
    pub(crate) required_capabilities: Vec<NativeUiAdapterCapability>,
}

impl NativeUiBackendCapabilityMatrix {
    pub(crate) fn native_runtime_ready(&self) -> bool {
        self.backend.status.is_native_runtime_ready()
    }

    pub(crate) fn scaffolded(&self) -> bool {
        self.backend.status.is_scaffold()
    }

    pub(crate) fn required_capability_names(&self) -> Vec<&'static str> {
        self.required_capabilities
            .iter()
            .map(|capability| capability.capability_name())
            .collect()
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeUiAdapterBindingPlan {
    pub(crate) platform: NativeUiPlatform,
    pub(crate) toolkit: NativeUiToolkit,
    pub(crate) status: NativeUiBackendStatus,
    pub(crate) adapter_boundary: &'static str,
    pub(crate) binding_names: Vec<&'static str>,
}

impl NativeUiAdapterBindingPlan {
    pub(crate) fn new(
        platform: NativeUiPlatform,
        toolkit: NativeUiToolkit,
        status: NativeUiBackendStatus,
        adapter_boundary: &'static str,
        binding_names: Vec<&'static str>,
    ) -> Self {
        Self {
            platform,
            toolkit,
            status,
            adapter_boundary,
            binding_names,
        }
    }

    pub(crate) const fn platform_name(&self) -> &'static str {
        self.platform.platform_name()
    }

    pub(crate) const fn toolkit_name(&self) -> &'static str {
        self.toolkit.toolkit_name()
    }

    pub(crate) const fn status_name(&self) -> &'static str {
        self.status.status_name()
    }

    pub(crate) fn has_binding_name(&self, binding_name: &str) -> bool {
        self.binding_names.contains(&binding_name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeUiAdapterReusePackage<TBootstrap> {
    pub(crate) manifest: NativeUiAdapterManifest,
    pub(crate) bootstrap: TBootstrap,
    pub(crate) binding_plan: NativeUiAdapterBindingPlan,
}

impl<TBootstrap> NativeUiAdapterReusePackage<TBootstrap> {
    pub(crate) const fn new(
        manifest: NativeUiAdapterManifest,
        bootstrap: TBootstrap,
        binding_plan: NativeUiAdapterBindingPlan,
    ) -> Self {
        Self {
            manifest,
            bootstrap,
            binding_plan,
        }
    }

    pub(crate) const fn platform_name(&self) -> &'static str {
        self.manifest.platform_name()
    }

    pub(crate) const fn toolkit_name(&self) -> &'static str {
        self.manifest.toolkit_name()
    }

    pub(crate) const fn status_name(&self) -> &'static str {
        self.manifest.status_name()
    }

    pub(crate) fn binding_count_matches_manifest(&self) -> bool {
        self.binding_plan.binding_names.len() == self.manifest.binding_count
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeUiAdapterParityReport {
    pub(crate) platform_names: Vec<&'static str>,
    pub(crate) toolkit_names: Vec<&'static str>,
    pub(crate) status_names: Vec<&'static str>,
    pub(crate) adapter_boundaries: Vec<&'static str>,
    pub(crate) binding_counts: Vec<usize>,
    pub(crate) main_execution_plan_counts: Vec<usize>,
    pub(crate) shared_non_host_protocol_counts: Vec<usize>,
    pub(crate) native_runtime_ready_platforms: Vec<&'static str>,
    pub(crate) first_pass_native_host_platforms: Vec<&'static str>,
    pub(crate) scaffold_platforms: Vec<&'static str>,
    pub(crate) all_binding_counts_match_manifest: bool,
    pub(crate) all_main_execution_plan_counts_match: bool,
    pub(crate) all_shared_non_host_protocol_counts_match: bool,
}

pub(crate) fn native_ui_adapter_parity_report<TBootstrap>(
    packages: &[NativeUiAdapterReusePackage<TBootstrap>],
) -> NativeUiAdapterParityReport {
    let main_execution_plan_counts: Vec<_> = packages
        .iter()
        .map(|package| package.manifest.main_execution_plans)
        .collect();
    let shared_non_host_protocol_counts: Vec<_> = packages
        .iter()
        .map(|package| package.manifest.shared_non_host_protocols)
        .collect();

    NativeUiAdapterParityReport {
        platform_names: packages
            .iter()
            .map(|package| package.platform_name())
            .collect(),
        toolkit_names: packages
            .iter()
            .map(|package| package.toolkit_name())
            .collect(),
        status_names: packages
            .iter()
            .map(|package| package.status_name())
            .collect(),
        adapter_boundaries: packages
            .iter()
            .map(|package| package.binding_plan.adapter_boundary)
            .collect(),
        binding_counts: packages
            .iter()
            .map(|package| package.manifest.binding_count)
            .collect(),
        main_execution_plan_counts: main_execution_plan_counts.clone(),
        shared_non_host_protocol_counts: shared_non_host_protocol_counts.clone(),
        native_runtime_ready_platforms: packages
            .iter()
            .filter(|package| package.manifest.status.is_native_runtime_ready())
            .map(|package| package.platform_name())
            .collect(),
        first_pass_native_host_platforms: packages
            .iter()
            .filter(|package| package.manifest.status.is_first_pass_native_host())
            .map(|package| package.platform_name())
            .collect(),
        scaffold_platforms: packages
            .iter()
            .filter(|package| package.manifest.status.is_scaffold())
            .map(|package| package.platform_name())
            .collect(),
        all_binding_counts_match_manifest: packages
            .iter()
            .all(NativeUiAdapterReusePackage::binding_count_matches_manifest),
        all_main_execution_plan_counts_match: all_counts_match(&main_execution_plan_counts),
        all_shared_non_host_protocol_counts_match: all_counts_match(
            &shared_non_host_protocol_counts,
        ),
    }
}

fn all_counts_match(counts: &[usize]) -> bool {
    match counts.first() {
        Some(expected) => counts.iter().all(|count| count == expected),
        None => true,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeUiAdapterManifest {
    pub(crate) platform: NativeUiPlatform,
    pub(crate) toolkit: NativeUiToolkit,
    pub(crate) status: NativeUiBackendStatus,
    pub(crate) binding_count: usize,
    pub(crate) main_execution_plans: usize,
    pub(crate) shared_non_host_protocols: usize,
}

impl NativeUiAdapterManifest {
    pub(crate) const fn new(
        platform: NativeUiPlatform,
        toolkit: NativeUiToolkit,
        status: NativeUiBackendStatus,
        binding_count: usize,
        main_execution_plans: usize,
        shared_non_host_protocols: usize,
    ) -> Self {
        Self {
            platform,
            toolkit,
            status,
            binding_count,
            main_execution_plans,
            shared_non_host_protocols,
        }
    }

    pub(crate) const fn platform_name(&self) -> &'static str {
        self.platform.platform_name()
    }

    pub(crate) const fn toolkit_name(&self) -> &'static str {
        self.toolkit.toolkit_name()
    }

    pub(crate) const fn status_name(&self) -> &'static str {
        self.status.status_name()
    }
}

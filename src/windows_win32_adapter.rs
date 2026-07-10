#![allow(dead_code)]

use crate::app_core::{
    zsui_reuse_bootstrap_plan, NativeUiAdapterBindingPlan, NativeUiAdapterManifest,
    NativeUiAdapterReusePackage, NativeUiBackendStatus, NativeUiPlatform, NativeUiToolkit,
    SettingsComponentKind, ZsuiReuseBootstrapPlan, REQUIRED_MAIN_HOST_EXECUTION_PLAN_KINDS,
    SHARED_NON_HOST_UI_PROTOCOLS,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WindowsWin32Backend {
    Win32Gdi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WindowsWin32HostBinding {
    RuntimeDriver,
    ApplicationLifecycle,
    CommandQueue,
    MainExecutionPlan,
    StyleResolver,
    ControlMapper,
    TextLayout,
    Renderer,
    SettingsControl,
    Clipboard,
    StatusItem,
    PopupMenu,
    Ime,
    TextCaret,
    Dialog,
    ShellOpen,
    WindowIdentity,
    PasteTarget,
    FileDialog,
    TextInputDialog,
    EditTextDialog,
    MailMergeWindow,
    MainSearch,
    MainWindow,
    SettingsWindow,
    SettingsDropdown,
    TransientWindow,
}

impl WindowsWin32HostBinding {
    pub(crate) const fn adapter_name(self) -> &'static str {
        match self {
            Self::RuntimeDriver => "win32_native_runtime_driver",
            Self::ApplicationLifecycle => "win32_message_loop_lifecycle",
            Self::CommandQueue => "win32_command_dispatch",
            Self::MainExecutionPlan => "shared_main_execution_plan_bridge",
            Self::StyleResolver => "win32_theme_style_resolver",
            Self::ControlMapper => "win32_control_factory",
            Self::TextLayout => "gdi_text_layout",
            Self::Renderer => "gdi_renderer",
            Self::SettingsControl => "win32_settings_control_host",
            Self::Clipboard => "windows_clipboard_host",
            Self::StatusItem => "shell_notify_icon_status_item",
            Self::PopupMenu => "win32_popup_menu_host",
            Self::Ime => "imm32_ime_bridge",
            Self::TextCaret => "win32_caret_bridge",
            Self::Dialog => "win32_dialog_host",
            Self::ShellOpen => "shell_execute_launcher",
            Self::WindowIdentity => "win32_window_identity",
            Self::PasteTarget => "win32_paste_target",
            Self::FileDialog => "win32_open_file_dialog",
            Self::TextInputDialog => "win32_text_input_dialog",
            Self::EditTextDialog => "win32_edit_text_dialog",
            Self::MailMergeWindow => "win32_mail_merge_window",
            Self::MainSearch => "win32_edit_search_control",
            Self::MainWindow => "win32_main_window_pair",
            Self::SettingsWindow => "win32_settings_window",
            Self::SettingsDropdown => "win32_dropdown_popup",
            Self::TransientWindow => "win32_no_activate_transient_window",
        }
    }
}

pub(crate) const REQUIRED_WINDOWS_WIN32_HOST_BINDINGS: [WindowsWin32HostBinding; 27] = [
    WindowsWin32HostBinding::RuntimeDriver,
    WindowsWin32HostBinding::ApplicationLifecycle,
    WindowsWin32HostBinding::CommandQueue,
    WindowsWin32HostBinding::MainExecutionPlan,
    WindowsWin32HostBinding::StyleResolver,
    WindowsWin32HostBinding::ControlMapper,
    WindowsWin32HostBinding::TextLayout,
    WindowsWin32HostBinding::Renderer,
    WindowsWin32HostBinding::SettingsControl,
    WindowsWin32HostBinding::Clipboard,
    WindowsWin32HostBinding::StatusItem,
    WindowsWin32HostBinding::PopupMenu,
    WindowsWin32HostBinding::Ime,
    WindowsWin32HostBinding::TextCaret,
    WindowsWin32HostBinding::Dialog,
    WindowsWin32HostBinding::ShellOpen,
    WindowsWin32HostBinding::WindowIdentity,
    WindowsWin32HostBinding::PasteTarget,
    WindowsWin32HostBinding::FileDialog,
    WindowsWin32HostBinding::TextInputDialog,
    WindowsWin32HostBinding::EditTextDialog,
    WindowsWin32HostBinding::MailMergeWindow,
    WindowsWin32HostBinding::MainSearch,
    WindowsWin32HostBinding::MainWindow,
    WindowsWin32HostBinding::SettingsWindow,
    WindowsWin32HostBinding::SettingsDropdown,
    WindowsWin32HostBinding::TransientWindow,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WindowsWin32ControlRole {
    StaticText,
    Edit,
    CheckBox,
    ComboBox,
    Button,
    DefaultButton,
}

impl From<SettingsComponentKind> for WindowsWin32ControlRole {
    fn from(kind: SettingsComponentKind) -> Self {
        match kind {
            SettingsComponentKind::Label => Self::StaticText,
            SettingsComponentKind::TextInput => Self::Edit,
            SettingsComponentKind::Toggle => Self::CheckBox,
            SettingsComponentKind::Dropdown => Self::ComboBox,
            SettingsComponentKind::Button => Self::Button,
            SettingsComponentKind::AccentButton => Self::DefaultButton,
        }
    }
}

impl WindowsWin32ControlRole {
    pub(crate) const fn win32_class_name(self) -> &'static str {
        match self {
            Self::StaticText => "STATIC",
            Self::Edit => "EDIT",
            Self::CheckBox => "BUTTON.checkbox",
            Self::ComboBox => "COMBOBOX",
            Self::Button => "BUTTON",
            Self::DefaultButton => "BUTTON.default",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WindowsWin32AdapterBoundary {
    backend: WindowsWin32Backend,
    bindings: Vec<WindowsWin32HostBinding>,
    main_execution_plans: usize,
    shared_non_host_protocols: usize,
}

impl WindowsWin32AdapterBoundary {
    pub(crate) fn default_from_core_contract() -> Self {
        Self {
            backend: WindowsWin32Backend::Win32Gdi,
            bindings: REQUIRED_WINDOWS_WIN32_HOST_BINDINGS.to_vec(),
            main_execution_plans: REQUIRED_MAIN_HOST_EXECUTION_PLAN_KINDS.len(),
            shared_non_host_protocols: SHARED_NON_HOST_UI_PROTOCOLS.len(),
        }
    }

    pub(crate) fn backend(&self) -> WindowsWin32Backend {
        self.backend
    }

    pub(crate) fn binding_count(&self) -> usize {
        self.bindings.len()
    }

    pub(crate) fn has_binding(&self, binding: WindowsWin32HostBinding) -> bool {
        self.bindings.contains(&binding)
    }

    pub(crate) fn binding_names(&self) -> Vec<&'static str> {
        self.bindings
            .iter()
            .map(|binding| binding.adapter_name())
            .collect()
    }

    pub(crate) fn main_execution_plans(&self) -> usize {
        self.main_execution_plans
    }

    pub(crate) fn shared_non_host_protocols(&self) -> usize {
        self.shared_non_host_protocols
    }

    pub(crate) fn manifest(&self) -> NativeUiAdapterManifest {
        NativeUiAdapterManifest::new(
            NativeUiPlatform::Windows,
            NativeUiToolkit::Win32Gdi,
            NativeUiBackendStatus::NativeHostIntegrated,
            self.binding_count(),
            self.main_execution_plans,
            self.shared_non_host_protocols,
        )
    }

    pub(crate) fn reuse_bootstrap_plan(&self) -> ZsuiReuseBootstrapPlan {
        zsui_reuse_bootstrap_plan(NativeUiPlatform::Windows)
            .expect("Windows is a supported ZSUI native UI platform")
    }

    pub(crate) fn adapter_binding_plan(&self) -> NativeUiAdapterBindingPlan {
        NativeUiAdapterBindingPlan::new(
            NativeUiPlatform::Windows,
            NativeUiToolkit::Win32Gdi,
            NativeUiBackendStatus::NativeHostIntegrated,
            "WindowsWin32AdapterBoundary",
            self.binding_names(),
        )
    }

    pub(crate) fn reuse_package(&self) -> NativeUiAdapterReusePackage<ZsuiReuseBootstrapPlan> {
        NativeUiAdapterReusePackage::new(
            self.manifest(),
            self.reuse_bootstrap_plan(),
            self.adapter_binding_plan(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_win32_adapter_boundary_covers_current_zsui_hosts() {
        let boundary = WindowsWin32AdapterBoundary::default_from_core_contract();

        assert_eq!(boundary.backend(), WindowsWin32Backend::Win32Gdi);
        assert_eq!(
            boundary.binding_count(),
            REQUIRED_WINDOWS_WIN32_HOST_BINDINGS.len()
        );
        assert_eq!(
            boundary.main_execution_plans(),
            REQUIRED_MAIN_HOST_EXECUTION_PLAN_KINDS.len()
        );
        assert_eq!(
            boundary.shared_non_host_protocols(),
            SHARED_NON_HOST_UI_PROTOCOLS.len()
        );
        assert_eq!(
            boundary.manifest(),
            NativeUiAdapterManifest::new(
                NativeUiPlatform::Windows,
                NativeUiToolkit::Win32Gdi,
                NativeUiBackendStatus::NativeHostIntegrated,
                REQUIRED_WINDOWS_WIN32_HOST_BINDINGS.len(),
                REQUIRED_MAIN_HOST_EXECUTION_PLAN_KINDS.len(),
                SHARED_NON_HOST_UI_PROTOCOLS.len()
            )
        );
        assert!(boundary.has_binding(WindowsWin32HostBinding::RuntimeDriver));
        assert!(boundary.has_binding(WindowsWin32HostBinding::MainWindow));
        assert!(boundary.has_binding(WindowsWin32HostBinding::SettingsWindow));
        assert!(boundary.has_binding(WindowsWin32HostBinding::Clipboard));
        assert!(boundary.has_binding(WindowsWin32HostBinding::StatusItem));
        assert!(boundary.has_binding(WindowsWin32HostBinding::Ime));
        assert!(boundary.has_binding(WindowsWin32HostBinding::TransientWindow));
        assert!(boundary
            .binding_names()
            .contains(&"shared_main_execution_plan_bridge"));
        let bootstrap = boundary.reuse_bootstrap_plan();
        assert_eq!(bootstrap.platform, NativeUiPlatform::Windows);
        assert_eq!(bootstrap.platform_name, "windows");
        assert_eq!(bootstrap.toolkit_name, "win32_gdi");
        assert_eq!(bootstrap.adapter_boundary, "WindowsWin32AdapterBoundary");
        assert!(bootstrap.native_runtime_ready());
        assert!(bootstrap
            .native_adapter_capability_names
            .contains(&"main_window"));
        assert_eq!(
            bootstrap.ai_executor_boundary_names,
            vec!["llm_executor", "skill_registry", "product_adapter_tools"]
        );
        let binding_plan = boundary.adapter_binding_plan();
        assert_eq!(binding_plan.platform_name(), "windows");
        assert_eq!(binding_plan.toolkit_name(), "win32_gdi");
        assert_eq!(binding_plan.status_name(), "native_host_integrated");
        assert_eq!(binding_plan.adapter_boundary, "WindowsWin32AdapterBoundary");
        assert_eq!(
            binding_plan.binding_names.len(),
            REQUIRED_WINDOWS_WIN32_HOST_BINDINGS.len()
        );
        assert!(binding_plan.has_binding_name("win32_native_runtime_driver"));
        assert!(binding_plan.has_binding_name("win32_main_window_pair"));
        assert!(binding_plan.has_binding_name("gdi_renderer"));
        let package = boundary.reuse_package();
        assert_eq!(package.platform_name(), "windows");
        assert_eq!(package.toolkit_name(), "win32_gdi");
        assert_eq!(package.status_name(), "native_host_integrated");
        assert_eq!(package.bootstrap.platform_name, "windows");
        assert!(package.binding_count_matches_manifest());
        assert!(package
            .binding_plan
            .has_binding_name("win32_message_loop_lifecycle"));
    }

    #[test]
    fn windows_win32_control_roles_map_settings_component_kinds() {
        assert_eq!(
            WindowsWin32ControlRole::from(SettingsComponentKind::Label).win32_class_name(),
            "STATIC"
        );
        assert_eq!(
            WindowsWin32ControlRole::from(SettingsComponentKind::TextInput).win32_class_name(),
            "EDIT"
        );
        assert_eq!(
            WindowsWin32ControlRole::from(SettingsComponentKind::Dropdown).win32_class_name(),
            "COMBOBOX"
        );
        assert_eq!(
            WindowsWin32ControlRole::from(SettingsComponentKind::AccentButton).win32_class_name(),
            "BUTTON.default"
        );
    }
}

#![allow(dead_code)]

use crate::app_core::{
    zsui_reuse_bootstrap_plan, NativeUiAdapterBindingPlan, NativeUiAdapterManifest,
    NativeUiAdapterReusePackage, NativeUiBackendStatus, NativeUiPlatform, NativeUiToolkit,
    SettingsComponentKind, ZsuiReuseBootstrapPlan,
};
use crate::macos_app::{MacosHostContractSummary, MacosUiHost};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MacosAppKitBackend {
    AppKitSwiftUI,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MacosAppKitHostBinding {
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

impl MacosAppKitHostBinding {
    pub(crate) const fn adapter_name(self) -> &'static str {
        match self {
            Self::RuntimeDriver => "appkit_native_runtime_driver",
            Self::ApplicationLifecycle => "ns_application_lifecycle",
            Self::CommandQueue => "main_dispatch_queue",
            Self::MainExecutionPlan => "shared_main_execution_plan_bridge",
            Self::StyleResolver => "ns_appearance_style_resolver",
            Self::ControlMapper => "appkit_control_factory",
            Self::TextLayout => "core_text_layout",
            Self::Renderer => "core_graphics_renderer",
            Self::SettingsControl => "ns_view_settings_control_host",
            Self::Clipboard => "ns_pasteboard_bridge",
            Self::StatusItem => "ns_status_item_bridge",
            Self::PopupMenu => "ns_menu_bridge",
            Self::Ime => "ns_input_context_bridge",
            Self::TextCaret => "accessibility_caret_bridge",
            Self::Dialog => "ns_alert_bridge",
            Self::ShellOpen => "ns_workspace_launcher",
            Self::WindowIdentity => "ns_running_application_window_identity",
            Self::PasteTarget => "accessibility_paste_target",
            Self::FileDialog => "ns_open_panel",
            Self::TextInputDialog => "ns_alert_text_field_dialog",
            Self::EditTextDialog => "ns_text_view_editor_window",
            Self::MailMergeWindow => "product_mail_merge_window",
            Self::MainSearch => "ns_search_field",
            Self::MainWindow => "ns_window_pair",
            Self::SettingsWindow => "settings_window_controller",
            Self::SettingsDropdown => "ns_pop_up_button_or_menu",
            Self::TransientWindow => "ns_panel_or_popover",
        }
    }
}

pub(crate) const REQUIRED_MACOS_APPKIT_HOST_BINDINGS: [MacosAppKitHostBinding; 27] = [
    MacosAppKitHostBinding::RuntimeDriver,
    MacosAppKitHostBinding::ApplicationLifecycle,
    MacosAppKitHostBinding::CommandQueue,
    MacosAppKitHostBinding::MainExecutionPlan,
    MacosAppKitHostBinding::StyleResolver,
    MacosAppKitHostBinding::ControlMapper,
    MacosAppKitHostBinding::TextLayout,
    MacosAppKitHostBinding::Renderer,
    MacosAppKitHostBinding::SettingsControl,
    MacosAppKitHostBinding::Clipboard,
    MacosAppKitHostBinding::StatusItem,
    MacosAppKitHostBinding::PopupMenu,
    MacosAppKitHostBinding::Ime,
    MacosAppKitHostBinding::TextCaret,
    MacosAppKitHostBinding::Dialog,
    MacosAppKitHostBinding::ShellOpen,
    MacosAppKitHostBinding::WindowIdentity,
    MacosAppKitHostBinding::PasteTarget,
    MacosAppKitHostBinding::FileDialog,
    MacosAppKitHostBinding::TextInputDialog,
    MacosAppKitHostBinding::EditTextDialog,
    MacosAppKitHostBinding::MailMergeWindow,
    MacosAppKitHostBinding::MainSearch,
    MacosAppKitHostBinding::MainWindow,
    MacosAppKitHostBinding::SettingsWindow,
    MacosAppKitHostBinding::SettingsDropdown,
    MacosAppKitHostBinding::TransientWindow,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MacosAppKitWidgetRole {
    Label,
    TextField,
    Switch,
    PopUpButton,
    Button,
    ProminentButton,
}

impl From<SettingsComponentKind> for MacosAppKitWidgetRole {
    fn from(kind: SettingsComponentKind) -> Self {
        match kind {
            SettingsComponentKind::Label => Self::Label,
            SettingsComponentKind::TextInput => Self::TextField,
            SettingsComponentKind::Toggle => Self::Switch,
            SettingsComponentKind::Dropdown => Self::PopUpButton,
            SettingsComponentKind::Button => Self::Button,
            SettingsComponentKind::AccentButton => Self::ProminentButton,
        }
    }
}

impl MacosAppKitWidgetRole {
    pub(crate) const fn appkit_type_name(self) -> &'static str {
        match self {
            Self::Label => "NSTextField.label",
            Self::TextField => "NSTextField",
            Self::Switch => "NSSwitch",
            Self::PopUpButton => "NSPopUpButton",
            Self::Button => "NSButton",
            Self::ProminentButton => "NSButton.borderedProminent",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MacosAppKitAdapterBoundary {
    backend: MacosAppKitBackend,
    bindings: Vec<MacosAppKitHostBinding>,
    main_execution_plans: usize,
    shared_non_host_protocols: usize,
}

impl MacosAppKitAdapterBoundary {
    pub(crate) fn from_contract(summary: MacosHostContractSummary) -> Self {
        Self {
            backend: MacosAppKitBackend::AppKitSwiftUI,
            bindings: REQUIRED_MACOS_APPKIT_HOST_BINDINGS.to_vec(),
            main_execution_plans: summary.main_execution_plans,
            shared_non_host_protocols: summary.shared_non_host_protocols,
        }
    }

    pub(crate) fn default_from_macos_contract() -> Self {
        Self::from_contract(MacosUiHost::contract_summary())
    }

    pub(crate) fn backend(&self) -> MacosAppKitBackend {
        self.backend
    }

    pub(crate) fn binding_count(&self) -> usize {
        self.bindings.len()
    }

    pub(crate) fn has_binding(&self, binding: MacosAppKitHostBinding) -> bool {
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
            NativeUiPlatform::Macos,
            NativeUiToolkit::AppKitSwiftUI,
            NativeUiBackendStatus::NativeHostFirstPass,
            self.binding_count(),
            self.main_execution_plans,
            self.shared_non_host_protocols,
        )
    }

    pub(crate) fn reuse_bootstrap_plan(&self) -> ZsuiReuseBootstrapPlan {
        zsui_reuse_bootstrap_plan(NativeUiPlatform::Macos)
            .expect("macOS is a supported ZSUI native UI platform")
    }

    pub(crate) fn adapter_binding_plan(&self) -> NativeUiAdapterBindingPlan {
        NativeUiAdapterBindingPlan::new(
            NativeUiPlatform::Macos,
            NativeUiToolkit::AppKitSwiftUI,
            NativeUiBackendStatus::NativeHostFirstPass,
            "MacosAppKitAdapterBoundary",
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
    use crate::app_core::{REQUIRED_MAIN_HOST_EXECUTION_PLAN_KINDS, SHARED_NON_HOST_UI_PROTOCOLS};

    #[test]
    fn macos_appkit_adapter_boundary_covers_current_zsui_hosts() {
        let boundary = MacosAppKitAdapterBoundary::default_from_macos_contract();

        assert_eq!(boundary.backend(), MacosAppKitBackend::AppKitSwiftUI);
        assert_eq!(
            boundary.binding_count(),
            REQUIRED_MACOS_APPKIT_HOST_BINDINGS.len()
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
                NativeUiPlatform::Macos,
                NativeUiToolkit::AppKitSwiftUI,
                NativeUiBackendStatus::NativeHostFirstPass,
                REQUIRED_MACOS_APPKIT_HOST_BINDINGS.len(),
                REQUIRED_MAIN_HOST_EXECUTION_PLAN_KINDS.len(),
                SHARED_NON_HOST_UI_PROTOCOLS.len()
            )
        );
        assert!(boundary.has_binding(MacosAppKitHostBinding::RuntimeDriver));
        assert!(boundary.has_binding(MacosAppKitHostBinding::MainWindow));
        assert!(boundary.has_binding(MacosAppKitHostBinding::SettingsWindow));
        assert!(boundary.has_binding(MacosAppKitHostBinding::Clipboard));
        assert!(boundary.has_binding(MacosAppKitHostBinding::StatusItem));
        assert!(boundary.has_binding(MacosAppKitHostBinding::Ime));
        assert!(boundary.has_binding(MacosAppKitHostBinding::TransientWindow));
        assert!(boundary
            .binding_names()
            .contains(&"shared_main_execution_plan_bridge"));
        let bootstrap = boundary.reuse_bootstrap_plan();
        assert_eq!(bootstrap.platform, NativeUiPlatform::Macos);
        assert_eq!(bootstrap.platform_name, "macos");
        assert_eq!(bootstrap.toolkit_name, "appkit_swiftui");
        assert_eq!(bootstrap.adapter_boundary, "MacosAppKitAdapterBoundary");
        assert!(!bootstrap.native_runtime_ready());
        assert!(!bootstrap.scaffolded());
        assert!(bootstrap.backend_status.is_first_pass_native_host());
        assert!(bootstrap
            .native_adapter_capability_names
            .contains(&"settings_window"));
        assert_eq!(
            bootstrap.ai_executor_boundary_names,
            vec!["llm_executor", "skill_registry", "product_adapter_tools"]
        );
        let binding_plan = boundary.adapter_binding_plan();
        assert_eq!(binding_plan.platform_name(), "macos");
        assert_eq!(binding_plan.toolkit_name(), "appkit_swiftui");
        assert_eq!(binding_plan.status_name(), "native_host_first_pass");
        assert_eq!(binding_plan.adapter_boundary, "MacosAppKitAdapterBoundary");
        assert_eq!(
            binding_plan.binding_names.len(),
            REQUIRED_MACOS_APPKIT_HOST_BINDINGS.len()
        );
        assert!(binding_plan.has_binding_name("appkit_native_runtime_driver"));
        assert!(binding_plan.has_binding_name("ns_window_pair"));
        assert!(binding_plan.has_binding_name("core_graphics_renderer"));
        let package = boundary.reuse_package();
        assert_eq!(package.platform_name(), "macos");
        assert_eq!(package.toolkit_name(), "appkit_swiftui");
        assert_eq!(package.status_name(), "native_host_first_pass");
        assert_eq!(package.bootstrap.platform_name, "macos");
        assert!(package.binding_count_matches_manifest());
        assert!(package
            .binding_plan
            .has_binding_name("ns_application_lifecycle"));
    }

    #[test]
    fn macos_appkit_widget_roles_map_settings_component_kinds() {
        assert_eq!(
            MacosAppKitWidgetRole::from(SettingsComponentKind::Label).appkit_type_name(),
            "NSTextField.label"
        );
        assert_eq!(
            MacosAppKitWidgetRole::from(SettingsComponentKind::TextInput).appkit_type_name(),
            "NSTextField"
        );
        assert_eq!(
            MacosAppKitWidgetRole::from(SettingsComponentKind::Dropdown).appkit_type_name(),
            "NSPopUpButton"
        );
        assert_eq!(
            MacosAppKitWidgetRole::from(SettingsComponentKind::AccentButton).appkit_type_name(),
            "NSButton.borderedProminent"
        );
    }
}

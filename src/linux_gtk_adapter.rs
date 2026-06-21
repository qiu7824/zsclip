#![allow(dead_code)]

use crate::app_core::{
    zsui_reuse_bootstrap_plan, NativeUiAdapterBindingPlan, NativeUiAdapterManifest,
    NativeUiAdapterReusePackage, NativeUiBackendStatus, NativeUiPlatform, NativeUiToolkit,
    ZsuiReuseBootstrapPlan,
};
use crate::linux_app::{
    linux_host_contract_summary, LinuxHostContractSummary, LinuxNativeBackend,
    LinuxNativeControlClass,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LinuxGtkHostBinding {
    RuntimeDriver,
    ApplicationLifecycle,
    CommandQueue,
    MainExecutionPlan,
    StyleResolver,
    ControlMapper,
    TextLayout,
    Renderer,
    Clipboard,
    StatusItem,
    PopupMenu,
    TransientWindow,
    Ime,
    Dialog,
    ShellOpen,
    WindowIdentity,
    PasteTarget,
    TextCaret,
    FileDialog,
    TextInputDialog,
    EditTextDialog,
    MailMergeWindow,
    MainWindow,
    MainSearch,
    SettingsWindow,
    SettingsControl,
    SettingsDropdown,
}

impl LinuxGtkHostBinding {
    pub(crate) const fn adapter_name(self) -> &'static str {
        match self {
            Self::RuntimeDriver => "gtk_native_runtime_driver",
            Self::ApplicationLifecycle => "gtk_application_lifecycle",
            Self::CommandQueue => "glib_command_dispatch",
            Self::MainExecutionPlan => "shared_main_execution_plan_bridge",
            Self::StyleResolver => "gtk_style_context_resolver",
            Self::ControlMapper => "libadwaita_control_factory",
            Self::TextLayout => "pango_text_layout",
            Self::Renderer => "gtk_snapshot_renderer",
            Self::Clipboard => "gdk_clipboard_bridge",
            Self::StatusItem => "app_indicator_status_item",
            Self::PopupMenu => "gtk_popover_menu",
            Self::TransientWindow => "gtk_popover_or_layer_surface",
            Self::Ime => "gtk_input_method_bridge",
            Self::Dialog => "adw_dialog_bridge",
            Self::ShellOpen => "gio_app_info_launcher",
            Self::WindowIdentity => "portal_or_ats_pi_window_identity",
            Self::PasteTarget => "portal_or_ats_pi_paste_target",
            Self::TextCaret => "portal_or_ats_pi_text_caret",
            Self::FileDialog => "gtk_file_dialog",
            Self::TextInputDialog => "adw_entry_dialog",
            Self::EditTextDialog => "adw_text_editor_dialog",
            Self::MailMergeWindow => "product_mail_merge_window",
            Self::MainWindow => "adw_application_window",
            Self::MainSearch => "gtk_search_entry",
            Self::SettingsWindow => "adw_preferences_window",
            Self::SettingsControl => "adw_preferences_row_factory",
            Self::SettingsDropdown => "gtk_popover_or_combo_row",
        }
    }
}

pub(crate) const REQUIRED_LINUX_GTK_HOST_BINDINGS: [LinuxGtkHostBinding; 27] = [
    LinuxGtkHostBinding::RuntimeDriver,
    LinuxGtkHostBinding::ApplicationLifecycle,
    LinuxGtkHostBinding::CommandQueue,
    LinuxGtkHostBinding::MainExecutionPlan,
    LinuxGtkHostBinding::StyleResolver,
    LinuxGtkHostBinding::ControlMapper,
    LinuxGtkHostBinding::TextLayout,
    LinuxGtkHostBinding::Renderer,
    LinuxGtkHostBinding::Clipboard,
    LinuxGtkHostBinding::StatusItem,
    LinuxGtkHostBinding::PopupMenu,
    LinuxGtkHostBinding::TransientWindow,
    LinuxGtkHostBinding::Ime,
    LinuxGtkHostBinding::Dialog,
    LinuxGtkHostBinding::ShellOpen,
    LinuxGtkHostBinding::WindowIdentity,
    LinuxGtkHostBinding::PasteTarget,
    LinuxGtkHostBinding::TextCaret,
    LinuxGtkHostBinding::FileDialog,
    LinuxGtkHostBinding::TextInputDialog,
    LinuxGtkHostBinding::EditTextDialog,
    LinuxGtkHostBinding::MailMergeWindow,
    LinuxGtkHostBinding::MainWindow,
    LinuxGtkHostBinding::MainSearch,
    LinuxGtkHostBinding::SettingsWindow,
    LinuxGtkHostBinding::SettingsControl,
    LinuxGtkHostBinding::SettingsDropdown,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LinuxGtkWidgetRole {
    Label,
    Entry,
    Switch,
    ComboRow,
    Button,
    SuggestedActionButton,
}

impl From<LinuxNativeControlClass> for LinuxGtkWidgetRole {
    fn from(class: LinuxNativeControlClass) -> Self {
        match class {
            LinuxNativeControlClass::Label => Self::Label,
            LinuxNativeControlClass::Entry => Self::Entry,
            LinuxNativeControlClass::Switch => Self::Switch,
            LinuxNativeControlClass::ComboRow => Self::ComboRow,
            LinuxNativeControlClass::Button => Self::Button,
            LinuxNativeControlClass::SuggestedActionButton => Self::SuggestedActionButton,
        }
    }
}

impl LinuxGtkWidgetRole {
    pub(crate) const fn gtk_type_name(self) -> &'static str {
        match self {
            Self::Label => "gtk::Label",
            Self::Entry => "gtk::Entry",
            Self::Switch => "gtk::Switch",
            Self::ComboRow => "adw::ComboRow",
            Self::Button => "gtk::Button",
            Self::SuggestedActionButton => "gtk::Button.suggested-action",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinuxGtkAdapterBoundary {
    backend: LinuxNativeBackend,
    bindings: Vec<LinuxGtkHostBinding>,
    main_execution_plans: usize,
    shared_non_host_protocols: usize,
}

impl LinuxGtkAdapterBoundary {
    pub(crate) fn from_contract(summary: LinuxHostContractSummary) -> Self {
        debug_assert_eq!(summary.backend, LinuxNativeBackend::Gtk4Libadwaita);
        Self {
            backend: summary.backend,
            bindings: REQUIRED_LINUX_GTK_HOST_BINDINGS.to_vec(),
            main_execution_plans: summary.main_execution_plans,
            shared_non_host_protocols: summary.shared_non_host_protocols,
        }
    }

    pub(crate) fn default_from_linux_contract() -> Self {
        Self::from_contract(linux_host_contract_summary())
    }

    pub(crate) fn backend(&self) -> LinuxNativeBackend {
        self.backend
    }

    pub(crate) fn binding_count(&self) -> usize {
        self.bindings.len()
    }

    pub(crate) fn has_binding(&self, binding: LinuxGtkHostBinding) -> bool {
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
            NativeUiPlatform::Linux,
            NativeUiToolkit::Gtk4Libadwaita,
            NativeUiBackendStatus::NativeHostFirstPass,
            self.binding_count(),
            self.main_execution_plans,
            self.shared_non_host_protocols,
        )
    }

    pub(crate) fn reuse_bootstrap_plan(&self) -> ZsuiReuseBootstrapPlan {
        zsui_reuse_bootstrap_plan(NativeUiPlatform::Linux)
            .expect("Linux is a supported ZSUI native UI platform")
    }

    pub(crate) fn adapter_binding_plan(&self) -> NativeUiAdapterBindingPlan {
        NativeUiAdapterBindingPlan::new(
            NativeUiPlatform::Linux,
            NativeUiToolkit::Gtk4Libadwaita,
            NativeUiBackendStatus::NativeHostFirstPass,
            "LinuxGtkAdapterBoundary",
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
    fn linux_gtk_adapter_boundary_covers_current_zsui_hosts() {
        let boundary = LinuxGtkAdapterBoundary::default_from_linux_contract();

        assert_eq!(boundary.backend(), LinuxNativeBackend::Gtk4Libadwaita);
        assert_eq!(
            boundary.binding_count(),
            REQUIRED_LINUX_GTK_HOST_BINDINGS.len()
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
                NativeUiPlatform::Linux,
                NativeUiToolkit::Gtk4Libadwaita,
                NativeUiBackendStatus::NativeHostFirstPass,
                REQUIRED_LINUX_GTK_HOST_BINDINGS.len(),
                REQUIRED_MAIN_HOST_EXECUTION_PLAN_KINDS.len(),
                SHARED_NON_HOST_UI_PROTOCOLS.len()
            )
        );
        assert!(boundary.has_binding(LinuxGtkHostBinding::RuntimeDriver));
        assert!(boundary.has_binding(LinuxGtkHostBinding::MainExecutionPlan));
        assert!(boundary.has_binding(LinuxGtkHostBinding::MainWindow));
        assert!(boundary.has_binding(LinuxGtkHostBinding::SettingsWindow));
        assert!(boundary.has_binding(LinuxGtkHostBinding::ControlMapper));
        assert!(boundary.has_binding(LinuxGtkHostBinding::Renderer));
        assert!(boundary.has_binding(LinuxGtkHostBinding::Clipboard));
        assert!(boundary.has_binding(LinuxGtkHostBinding::Ime));
        assert!(boundary
            .binding_names()
            .contains(&"shared_main_execution_plan_bridge"));
        let bootstrap = boundary.reuse_bootstrap_plan();
        assert_eq!(bootstrap.platform, NativeUiPlatform::Linux);
        assert_eq!(bootstrap.platform_name, "linux");
        assert_eq!(bootstrap.toolkit_name, "gtk4_libadwaita");
        assert_eq!(bootstrap.adapter_boundary, "LinuxGtkAdapterBoundary");
        assert!(!bootstrap.native_runtime_ready());
        assert!(!bootstrap.scaffolded());
        assert!(bootstrap.backend_status.is_first_pass_native_host());
        assert!(bootstrap
            .native_adapter_capability_names
            .contains(&"main_execution_plan_bridge"));
        assert_eq!(
            bootstrap.ai_executor_boundary_names,
            vec!["llm_executor", "skill_registry", "product_adapter_tools"]
        );
        let binding_plan = boundary.adapter_binding_plan();
        assert_eq!(binding_plan.platform_name(), "linux");
        assert_eq!(binding_plan.toolkit_name(), "gtk4_libadwaita");
        assert_eq!(binding_plan.status_name(), "native_host_first_pass");
        assert_eq!(binding_plan.adapter_boundary, "LinuxGtkAdapterBoundary");
        assert_eq!(
            binding_plan.binding_names.len(),
            REQUIRED_LINUX_GTK_HOST_BINDINGS.len()
        );
        assert!(binding_plan.has_binding_name("gtk_native_runtime_driver"));
        assert!(binding_plan.has_binding_name("adw_application_window"));
        assert!(binding_plan.has_binding_name("gtk_snapshot_renderer"));
        let package = boundary.reuse_package();
        assert_eq!(package.platform_name(), "linux");
        assert_eq!(package.toolkit_name(), "gtk4_libadwaita");
        assert_eq!(package.status_name(), "native_host_first_pass");
        assert_eq!(package.bootstrap.platform_name, "linux");
        assert!(package.binding_count_matches_manifest());
        assert!(package
            .binding_plan
            .has_binding_name("gtk_application_lifecycle"));
    }

    #[test]
    fn linux_gtk_widget_roles_map_native_control_classes() {
        assert_eq!(
            LinuxGtkWidgetRole::from(LinuxNativeControlClass::Label).gtk_type_name(),
            "gtk::Label"
        );
        assert_eq!(
            LinuxGtkWidgetRole::from(LinuxNativeControlClass::Entry).gtk_type_name(),
            "gtk::Entry"
        );
        assert_eq!(
            LinuxGtkWidgetRole::from(LinuxNativeControlClass::ComboRow).gtk_type_name(),
            "adw::ComboRow"
        );
        assert_eq!(
            LinuxGtkWidgetRole::from(LinuxNativeControlClass::SuggestedActionButton)
                .gtk_type_name(),
            "gtk::Button.suggested-action"
        );
    }
}

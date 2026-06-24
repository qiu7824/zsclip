use super::{
    native_host_projected_clip_list_item_label, MainVvPopupRenderPlan,
    NativeHostClipListItemProjection, NativeHostClipRowAction, NativeHostDialogAction,
    NativeHostEditTextAction, NativeHostMainToolAction, NativeHostRowAction,
    NativeHostSearchControlAction, NativeHostSettingsAction, NativeHostSettingsControlAction,
    NativeHostSettingsGroupAction, NativeHostSettingsPlatformAction, NativeHostStatusMenuAction,
    NativeHostUiAction, UiRect,
};
#[cfg(feature = "vv-paste")]
use super::{MainVvPopupTextRole, NativeHostVvSelectAction};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeComponentKind {
    Button,
    Dropdown,
    MenuItem,
    SearchInput,
    Toggle,
}

impl NativeComponentKind {
    pub(crate) const fn role_name(self) -> &'static str {
        match self {
            Self::Button => "button",
            Self::Dropdown => "dropdown",
            Self::MenuItem => "menu_item",
            Self::SearchInput => "search_input",
            Self::Toggle => "toggle",
        }
    }
}

pub(crate) trait HostComponent {
    type Action;

    fn id(&self) -> &str;
    fn label(&self) -> &str;
    fn bounds(&self) -> UiRect;
    fn action(&self) -> &Self::Action;

    fn style_role(&self) -> NativeButtonStyleRole {
        NativeButtonStyleRole::Plain
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeButtonStyleRole {
    Plain,
    Suggested,
    Destructive,
}

impl NativeButtonStyleRole {
    pub(crate) const fn role_name(self) -> &'static str {
        match self {
            Self::Plain => "plain",
            Self::Suggested => "suggested",
            Self::Destructive => "destructive",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeButtonSpec<Action> {
    pub(crate) id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) bounds: UiRect,
    pub(crate) style_role: NativeButtonStyleRole,
    pub(crate) action: Action,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeSearchInputSpec<Action> {
    pub(crate) id: &'static str,
    pub(crate) placeholder: &'static str,
    pub(crate) bounds: UiRect,
    pub(crate) action: Action,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeToggleSpec<Action> {
    pub(crate) id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) bounds: UiRect,
    pub(crate) action: Action,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeDropdownSpec<Action> {
    pub(crate) id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) bounds: UiRect,
    pub(crate) options: &'static [NativeDropdownOptionSpec],
    pub(crate) action: Action,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeDropdownOptionSpec {
    pub(crate) raw_value: &'static str,
    pub(crate) label: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeMenuItemSpec<Action> {
    pub(crate) id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) icon_name: &'static str,
    pub(crate) accelerator_key: &'static str,
    pub(crate) starts_section: bool,
    pub(crate) action: Action,
}

impl<Action> NativeButtonSpec<Action> {
    pub(crate) const fn new(
        id: &'static str,
        label: &'static str,
        bounds: UiRect,
        action: Action,
    ) -> Self {
        Self {
            id,
            label,
            bounds,
            style_role: NativeButtonStyleRole::Plain,
            action,
        }
    }

    pub(crate) const fn with_style_role(mut self, style_role: NativeButtonStyleRole) -> Self {
        self.style_role = style_role;
        self
    }

    pub(crate) const fn width(&self) -> i32 {
        self.bounds.width()
    }

    pub(crate) const fn height(&self) -> i32 {
        self.bounds.height()
    }
}

impl<Action> NativeSearchInputSpec<Action> {
    pub(crate) const fn new(
        id: &'static str,
        placeholder: &'static str,
        bounds: UiRect,
        action: Action,
    ) -> Self {
        Self {
            id,
            placeholder,
            bounds,
            action,
        }
    }
}

impl<Action> NativeToggleSpec<Action> {
    pub(crate) const fn new(
        id: &'static str,
        label: &'static str,
        bounds: UiRect,
        action: Action,
    ) -> Self {
        Self {
            id,
            label,
            bounds,
            action,
        }
    }

    pub(crate) const fn width(&self) -> i32 {
        self.bounds.width()
    }

    pub(crate) const fn height(&self) -> i32 {
        self.bounds.height()
    }
}

impl<Action> NativeDropdownSpec<Action> {
    pub(crate) const fn new(
        id: &'static str,
        label: &'static str,
        bounds: UiRect,
        action: Action,
    ) -> Self {
        Self {
            id,
            label,
            bounds,
            options: &[],
            action,
        }
    }

    pub(crate) const fn with_options(
        mut self,
        options: &'static [NativeDropdownOptionSpec],
    ) -> Self {
        self.options = options;
        self
    }

    pub(crate) const fn width(&self) -> i32 {
        self.bounds.width()
    }

    pub(crate) const fn height(&self) -> i32 {
        self.bounds.height()
    }
}

pub(crate) const NATIVE_SYNC_MODE_DROPDOWN_OPTIONS: [NativeDropdownOptionSpec; 3] = [
    NativeDropdownOptionSpec {
        raw_value: "off",
        label: "关闭",
    },
    NativeDropdownOptionSpec {
        raw_value: "webdav",
        label: "WebDAV",
    },
    NativeDropdownOptionSpec {
        raw_value: "lan",
        label: "局域网",
    },
];

impl<Action> NativeMenuItemSpec<Action> {
    pub(crate) const fn new(id: &'static str, label: &'static str, action: Action) -> Self {
        Self {
            id,
            label,
            icon_name: "",
            accelerator_key: "",
            starts_section: false,
            action,
        }
    }

    pub(crate) const fn with_icon_name(mut self, icon_name: &'static str) -> Self {
        self.icon_name = icon_name;
        self
    }

    pub(crate) const fn with_accelerator_key(mut self, accelerator_key: &'static str) -> Self {
        self.accelerator_key = accelerator_key;
        self
    }

    pub(crate) const fn starting_section(mut self) -> Self {
        self.starts_section = true;
        self
    }
}

impl<Action> HostComponent for NativeButtonSpec<Action> {
    type Action = Action;

    fn id(&self) -> &str {
        self.id
    }

    fn label(&self) -> &str {
        self.label
    }

    fn bounds(&self) -> UiRect {
        self.bounds
    }

    fn action(&self) -> &Self::Action {
        &self.action
    }

    fn style_role(&self) -> NativeButtonStyleRole {
        self.style_role
    }
}

impl<Action> HostComponent for NativeSearchInputSpec<Action> {
    type Action = Action;

    fn id(&self) -> &str {
        self.id
    }

    fn label(&self) -> &str {
        self.placeholder
    }

    fn bounds(&self) -> UiRect {
        self.bounds
    }

    fn action(&self) -> &Self::Action {
        &self.action
    }
}

impl<Action> HostComponent for NativeToggleSpec<Action> {
    type Action = Action;

    fn id(&self) -> &str {
        self.id
    }

    fn label(&self) -> &str {
        self.label
    }

    fn bounds(&self) -> UiRect {
        self.bounds
    }

    fn action(&self) -> &Self::Action {
        &self.action
    }
}

impl<Action> HostComponent for NativeDropdownSpec<Action> {
    type Action = Action;

    fn id(&self) -> &str {
        self.id
    }

    fn label(&self) -> &str {
        self.label
    }

    fn bounds(&self) -> UiRect {
        self.bounds
    }

    fn action(&self) -> &Self::Action {
        &self.action
    }
}

impl<Action> HostComponent for NativeMenuItemSpec<Action> {
    type Action = Action;

    fn id(&self) -> &str {
        self.id
    }

    fn label(&self) -> &str {
        self.label
    }

    fn bounds(&self) -> UiRect {
        UiRect::new(0, 0, 0, 0)
    }

    fn action(&self) -> &Self::Action {
        &self.action
    }
}

impl HostComponent for NativeComponentSpec {
    type Action = NativeComponentAction;

    fn id(&self) -> &str {
        self.id
    }

    fn label(&self) -> &str {
        self.label
    }

    fn bounds(&self) -> UiRect {
        self.bounds
    }

    fn action(&self) -> &Self::Action {
        &self.action
    }

    fn style_role(&self) -> NativeButtonStyleRole {
        self.style_role
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeComponentAction {
    HostUi(NativeHostUiAction),
    MainTool(NativeHostMainToolAction),
    ClipRow(NativeHostClipRowAction),
    Row(NativeHostRowAction),
    SearchControl(NativeHostSearchControlAction),
    StatusMenu(NativeHostStatusMenuAction),
    Settings(NativeHostSettingsAction),
    SettingsControl(NativeHostSettingsControlAction),
    SettingsPlatform(NativeHostSettingsPlatformAction),
    SettingsGroup(NativeHostSettingsGroupAction),
    EditText(NativeHostEditTextAction),
    #[cfg(feature = "vv-paste")]
    VvSelect(NativeHostVvSelectAction),
    Dialog(NativeHostDialogAction),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeUiProtocolSurfaceKind {
    MainWindow,
    Menu,
    SettingsPage,
    Dialog,
    DynamicControls,
}

impl NativeUiProtocolSurfaceKind {
    pub(crate) const fn surface_name(self) -> &'static str {
        match self {
            Self::MainWindow => "main_window",
            Self::Menu => "menu",
            Self::SettingsPage => "settings_page",
            Self::Dialog => "dialog",
            Self::DynamicControls => "dynamic_controls",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeUiProtocolSurface {
    pub(crate) kind: NativeUiProtocolSurfaceKind,
    pub(crate) protocol_builder_names: &'static [&'static str],
    pub(crate) dynamic_protocol_builder_names: &'static [&'static str],
    pub(crate) action_family_names: &'static [&'static str],
    pub(crate) platform_host_rule: &'static str,
}

impl NativeUiProtocolSurface {
    pub(crate) const fn surface_name(self) -> &'static str {
        self.kind.surface_name()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeComponentFamilyDescriptor {
    pub(crate) family_name: &'static str,
    pub(crate) surface_name: &'static str,
    pub(crate) action_family_name: &'static str,
    pub(crate) typed_spec_name: &'static str,
    pub(crate) spec_builder_name: &'static str,
    pub(crate) erased_spec_name: &'static str,
    pub(crate) erased_spec_builder_name: Option<&'static str>,
    pub(crate) erased_component_kind: NativeComponentKind,
    pub(crate) dynamic: bool,
    pub(crate) feature_gate: Option<&'static str>,
    pub(crate) extension_rule: &'static str,
}

impl NativeComponentFamilyDescriptor {
    pub(crate) const fn is_feature_gated(self) -> bool {
        self.feature_gate.is_some()
    }
}

pub(crate) fn native_component_family_descriptors() -> Vec<NativeComponentFamilyDescriptor> {
    native_component_family_descriptor_slice().to_vec()
}

pub(crate) fn native_component_family_descriptors_for_surface(
    surface_name: &str,
) -> Vec<NativeComponentFamilyDescriptor> {
    native_component_family_descriptor_slice()
        .iter()
        .copied()
        .filter(|descriptor| descriptor.surface_name == surface_name)
        .collect()
}

pub(crate) fn native_component_family_descriptor_for_builder(
    builder_name: &str,
) -> Option<NativeComponentFamilyDescriptor> {
    native_component_family_descriptor_slice()
        .iter()
        .copied()
        .find(|descriptor| {
            descriptor.spec_builder_name == builder_name
                || descriptor.erased_spec_builder_name == Some(builder_name)
        })
}

#[cfg(feature = "vv-paste")]
const fn native_component_family_descriptor_slice() -> &'static [NativeComponentFamilyDescriptor] {
    &[
        NATIVE_COMPONENT_FAMILY_MAIN_ACTION,
        NATIVE_COMPONENT_FAMILY_MAIN_TOOL,
        NATIVE_COMPONENT_FAMILY_SEARCH,
        NATIVE_COMPONENT_FAMILY_ROW_ACTION,
        NATIVE_COMPONENT_FAMILY_STATUS_MENU,
        NATIVE_COMPONENT_FAMILY_SETTINGS_ACTION,
        NATIVE_COMPONENT_FAMILY_SETTINGS_CONTROL,
        NATIVE_COMPONENT_FAMILY_SETTINGS_TOGGLE,
        NATIVE_COMPONENT_FAMILY_SETTINGS_DROPDOWN,
        NATIVE_COMPONENT_FAMILY_SETTINGS_GROUP,
        NATIVE_COMPONENT_FAMILY_SETTINGS_PLATFORM,
        NATIVE_COMPONENT_FAMILY_DIALOG,
        NATIVE_COMPONENT_FAMILY_EDIT_TEXT,
        NATIVE_COMPONENT_FAMILY_CLIP_ROW,
        NATIVE_COMPONENT_FAMILY_VV_SELECT,
    ]
}

#[cfg(not(feature = "vv-paste"))]
const fn native_component_family_descriptor_slice() -> &'static [NativeComponentFamilyDescriptor] {
    &[
        NATIVE_COMPONENT_FAMILY_MAIN_ACTION,
        NATIVE_COMPONENT_FAMILY_MAIN_TOOL,
        NATIVE_COMPONENT_FAMILY_SEARCH,
        NATIVE_COMPONENT_FAMILY_ROW_ACTION,
        NATIVE_COMPONENT_FAMILY_STATUS_MENU,
        NATIVE_COMPONENT_FAMILY_SETTINGS_ACTION,
        NATIVE_COMPONENT_FAMILY_SETTINGS_CONTROL,
        NATIVE_COMPONENT_FAMILY_SETTINGS_TOGGLE,
        NATIVE_COMPONENT_FAMILY_SETTINGS_DROPDOWN,
        NATIVE_COMPONENT_FAMILY_SETTINGS_GROUP,
        NATIVE_COMPONENT_FAMILY_SETTINGS_PLATFORM,
        NATIVE_COMPONENT_FAMILY_DIALOG,
        NATIVE_COMPONENT_FAMILY_EDIT_TEXT,
        NATIVE_COMPONENT_FAMILY_CLIP_ROW,
    ]
}

const NATIVE_COMPONENT_FAMILY_MAIN_ACTION: NativeComponentFamilyDescriptor =
    NativeComponentFamilyDescriptor {
        family_name: "main_action_button",
        surface_name: "main_window",
        action_family_name: "HostUi",
        typed_spec_name: "NativeButtonSpec<NativeHostUiAction>",
        spec_builder_name: "native_host_main_action_button_specs",
        erased_spec_name: "NativeComponentSpec",
        erased_spec_builder_name: Some("native_host_main_action_component_specs"),
        erased_component_kind: NativeComponentKind::Button,
        dynamic: false,
        feature_gate: None,
        extension_rule:
            "add the typed action and button spec in app_core before adapting platform hosts",
    };

const NATIVE_COMPONENT_FAMILY_MAIN_TOOL: NativeComponentFamilyDescriptor =
    NativeComponentFamilyDescriptor {
        family_name: "main_tool_button",
        surface_name: "main_window",
        action_family_name: "MainTool",
        typed_spec_name: "NativeButtonSpec<NativeHostMainToolAction>",
        spec_builder_name: "native_host_main_tool_button_specs",
        erased_spec_name: "NativeComponentSpec",
        erased_spec_builder_name: Some("native_host_main_tool_component_specs"),
        erased_component_kind: NativeComponentKind::Button,
        dynamic: false,
        feature_gate: None,
        extension_rule:
            "extend the typed main tool action family before wiring a native toolbar control",
    };

const NATIVE_COMPONENT_FAMILY_SEARCH: NativeComponentFamilyDescriptor =
    NativeComponentFamilyDescriptor {
        family_name: "search_input",
        surface_name: "main_window",
        action_family_name: "SearchControl",
        typed_spec_name: "NativeSearchInputSpec<NativeHostSearchControlAction>",
        spec_builder_name: "native_host_search_input_specs",
        erased_spec_name: "NativeComponentSpec",
        erased_spec_builder_name: Some("native_host_search_component_specs"),
        erased_component_kind: NativeComponentKind::SearchInput,
        dynamic: false,
        feature_gate: None,
        extension_rule: "extend the typed search control contract before platform search widgets",
    };

const NATIVE_COMPONENT_FAMILY_ROW_ACTION: NativeComponentFamilyDescriptor =
    NativeComponentFamilyDescriptor {
        family_name: "row_action_button",
        surface_name: "menu",
        action_family_name: "Row",
        typed_spec_name: "NativeButtonSpec<NativeHostRowAction>",
        spec_builder_name: "native_host_row_action_button_specs",
        erased_spec_name: "NativeComponentSpec",
        erased_spec_builder_name: Some("native_host_row_action_component_specs"),
        erased_component_kind: NativeComponentKind::Button,
        dynamic: false,
        feature_gate: None,
        extension_rule:
            "extend row actions and shared menu plans before touching platform popup menus",
    };

const NATIVE_COMPONENT_FAMILY_STATUS_MENU: NativeComponentFamilyDescriptor =
    NativeComponentFamilyDescriptor {
        family_name: "status_menu_item",
        surface_name: "menu",
        action_family_name: "StatusMenu",
        typed_spec_name: "NativeMenuItemSpec<NativeHostStatusMenuAction>",
        spec_builder_name: "native_host_status_menu_item_specs",
        erased_spec_name: "NativeComponentSpec",
        erased_spec_builder_name: Some("native_host_status_menu_component_specs"),
        erased_component_kind: NativeComponentKind::MenuItem,
        dynamic: false,
        feature_gate: None,
        extension_rule:
            "extend status menu actions and typed menu item metadata before platform tray menus",
    };

const NATIVE_COMPONENT_FAMILY_SETTINGS_ACTION: NativeComponentFamilyDescriptor =
    NativeComponentFamilyDescriptor {
        family_name: "settings_action_button",
        surface_name: "settings_page",
        action_family_name: "Settings",
        typed_spec_name: "NativeButtonSpec<NativeHostSettingsAction>",
        spec_builder_name: "native_host_settings_action_button_specs",
        erased_spec_name: "NativeComponentSpec",
        erased_spec_builder_name: Some("native_host_settings_action_component_specs"),
        erased_component_kind: NativeComponentKind::Button,
        dynamic: false,
        feature_gate: None,
        extension_rule: "extend shared settings actions before native settings window buttons",
    };

const NATIVE_COMPONENT_FAMILY_SETTINGS_CONTROL: NativeComponentFamilyDescriptor =
    NativeComponentFamilyDescriptor {
        family_name: "settings_control_button",
        surface_name: "settings_page",
        action_family_name: "SettingsControl",
        typed_spec_name: "NativeButtonSpec<NativeHostSettingsControlAction>",
        spec_builder_name: "native_host_settings_control_button_specs",
        erased_spec_name: "NativeComponentSpec",
        erased_spec_builder_name: Some("native_host_settings_control_component_specs"),
        erased_component_kind: NativeComponentKind::Button,
        dynamic: false,
        feature_gate: None,
        extension_rule:
            "add the settings control action and model mapping before native settings hosts",
    };

const NATIVE_COMPONENT_FAMILY_SETTINGS_TOGGLE: NativeComponentFamilyDescriptor =
    NativeComponentFamilyDescriptor {
        family_name: "settings_toggle",
        surface_name: "settings_page",
        action_family_name: "SettingsControl",
        typed_spec_name: "NativeToggleSpec<NativeHostSettingsControlAction>",
        spec_builder_name: "native_host_settings_toggle_specs",
        erased_spec_name: "NativeComponentSpec",
        erased_spec_builder_name: Some("native_host_settings_control_component_specs"),
        erased_component_kind: NativeComponentKind::Toggle,
        dynamic: false,
        feature_gate: None,
        extension_rule:
            "add toggle settings through typed toggle specs before native Switch/checkbox hosts",
    };

const NATIVE_COMPONENT_FAMILY_SETTINGS_DROPDOWN: NativeComponentFamilyDescriptor =
    NativeComponentFamilyDescriptor {
        family_name: "settings_dropdown",
        surface_name: "settings_page",
        action_family_name: "SettingsControl",
        typed_spec_name: "NativeDropdownSpec<NativeHostSettingsControlAction>",
        spec_builder_name: "native_host_settings_dropdown_specs",
        erased_spec_name: "NativeComponentSpec",
        erased_spec_builder_name: Some("native_host_settings_control_component_specs"),
        erased_component_kind: NativeComponentKind::Dropdown,
        dynamic: false,
        feature_gate: Some("cloud-sync|lan-sync"),
        extension_rule:
            "add dropdown settings through typed dropdown specs before native combo/dropdown hosts",
    };

const NATIVE_COMPONENT_FAMILY_SETTINGS_GROUP: NativeComponentFamilyDescriptor =
    NativeComponentFamilyDescriptor {
        family_name: "settings_group_button",
        surface_name: "settings_page",
        action_family_name: "SettingsGroup",
        typed_spec_name: "NativeButtonSpec<NativeHostSettingsGroupAction>",
        spec_builder_name: "native_host_settings_group_button_specs",
        erased_spec_name: "NativeComponentSpec",
        erased_spec_builder_name: Some("native_host_settings_group_component_specs"),
        erased_component_kind: NativeComponentKind::Button,
        dynamic: false,
        feature_gate: None,
        extension_rule:
            "extend group management actions in app_core before platform group controls",
    };

const NATIVE_COMPONENT_FAMILY_SETTINGS_PLATFORM: NativeComponentFamilyDescriptor =
    NativeComponentFamilyDescriptor {
        family_name: "settings_platform_button",
        surface_name: "settings_page",
        action_family_name: "SettingsPlatform",
        typed_spec_name: "NativeButtonSpec<NativeHostSettingsPlatformAction>",
        spec_builder_name: "native_host_settings_platform_button_specs",
        erased_spec_name: "NativeComponentSpec",
        erased_spec_builder_name: Some("native_host_settings_platform_component_specs"),
        erased_component_kind: NativeComponentKind::Button,
        dynamic: false,
        feature_gate: None,
        extension_rule:
            "add platform-neutral settings intent first, then native platform affordances",
    };

const NATIVE_COMPONENT_FAMILY_DIALOG: NativeComponentFamilyDescriptor =
    NativeComponentFamilyDescriptor {
        family_name: "dialog_button",
        surface_name: "dialog",
        action_family_name: "Dialog",
        typed_spec_name: "NativeButtonSpec<NativeHostDialogAction>",
        spec_builder_name: "native_host_dialog_button_specs",
        erased_spec_name: "NativeComponentSpec",
        erased_spec_builder_name: Some("native_host_dialog_component_specs"),
        erased_component_kind: NativeComponentKind::Button,
        dynamic: false,
        feature_gate: None,
        extension_rule: "extend shared dialog actions before native modal implementations",
    };

const NATIVE_COMPONENT_FAMILY_EDIT_TEXT: NativeComponentFamilyDescriptor =
    NativeComponentFamilyDescriptor {
        family_name: "edit_text_button",
        surface_name: "dialog",
        action_family_name: "EditText",
        typed_spec_name: "NativeButtonSpec<NativeHostEditTextAction>",
        spec_builder_name: "native_host_edit_text_button_specs",
        erased_spec_name: "NativeComponentSpec",
        erased_spec_builder_name: Some("native_host_edit_text_component_specs"),
        erased_component_kind: NativeComponentKind::Button,
        dynamic: false,
        feature_gate: None,
        extension_rule: "extend edit text actions before native edit dialogs",
    };

const NATIVE_COMPONENT_FAMILY_CLIP_ROW: NativeComponentFamilyDescriptor =
    NativeComponentFamilyDescriptor {
        family_name: "clip_row_instance",
        surface_name: "dynamic_controls",
        action_family_name: "ClipRow",
        typed_spec_name: "NativeClipRowSpec",
        spec_builder_name: "native_host_clip_row_specs",
        erased_spec_name: "NativeComponentInstanceSpec",
        erased_spec_builder_name: Some("native_host_clip_row_component_specs"),
        erased_component_kind: NativeComponentKind::Button,
        dynamic: true,
        feature_gate: None,
        extension_rule:
            "extend clip row actions and projection data before native list row rendering",
    };

#[cfg(feature = "vv-paste")]
const NATIVE_COMPONENT_FAMILY_VV_SELECT: NativeComponentFamilyDescriptor =
    NativeComponentFamilyDescriptor {
        family_name: "vv_select_instance",
        surface_name: "dynamic_controls",
        action_family_name: "VvSelect",
        typed_spec_name: "NativeVvSelectSpec",
        spec_builder_name: "native_host_vv_select_specs",
        erased_spec_name: "NativeComponentInstanceSpec",
        erased_spec_builder_name: Some("native_host_vv_select_component_specs"),
        erased_component_kind: NativeComponentKind::Button,
        dynamic: true,
        feature_gate: Some("vv-paste"),
        extension_rule: "guard VV controls with #[cfg(feature = \"vv-paste\")] and keep the popup plan in app_core",
    };

pub(crate) const fn native_ui_protocol_surfaces() -> [NativeUiProtocolSurface; 5] {
    [
        NativeUiProtocolSurface {
            kind: NativeUiProtocolSurfaceKind::MainWindow,
            protocol_builder_names: &[
                "native_host_main_action_button_specs",
                "native_host_main_tool_button_specs",
                "native_host_search_input_specs",
            ],
            dynamic_protocol_builder_names: &[],
            action_family_names: &["HostUi", "MainTool", "SearchControl"],
            platform_host_rule: "translate main window specs into native controls and forward actions into app_core commands",
        },
        NativeUiProtocolSurface {
            kind: NativeUiProtocolSurfaceKind::Menu,
            protocol_builder_names: &[
                "native_host_row_action_button_specs",
                "native_host_status_menu_item_specs",
                "native_host_full_row_popup_menu_entries_for_groups",
                "native_host_group_filter_popup_menu_entries_for_groups",
            ],
            dynamic_protocol_builder_names: &[],
            action_family_names: &["Row", "StatusMenu"],
            platform_host_rule: "translate app_core row, group, and status menu plans into native menu items",
        },
        NativeUiProtocolSurface {
            kind: NativeUiProtocolSurfaceKind::SettingsPage,
            protocol_builder_names: &[
                "native_host_settings_action_button_specs",
                "native_host_settings_control_button_specs",
                "native_host_settings_toggle_specs",
                "native_host_settings_dropdown_specs",
                "native_host_settings_group_button_specs",
                "native_host_settings_platform_button_specs",
            ],
            dynamic_protocol_builder_names: &[],
            action_family_names: &[
                "Settings",
                "SettingsControl",
                "SettingsGroup",
                "SettingsPlatform",
            ],
            platform_host_rule: "translate shared settings specs and settings_model summaries into native settings controls",
        },
        NativeUiProtocolSurface {
            kind: NativeUiProtocolSurfaceKind::Dialog,
            protocol_builder_names: &[
                "native_host_dialog_button_specs",
                "native_host_edit_text_button_specs",
            ],
            dynamic_protocol_builder_names: &[],
            action_family_names: &["Dialog", "EditText"],
            platform_host_rule: "translate shared dialog and edit specs into native modal or document-style dialogs",
        },
        NativeUiProtocolSurface {
            kind: NativeUiProtocolSurfaceKind::DynamicControls,
            protocol_builder_names: &[],
            dynamic_protocol_builder_names: native_host_dynamic_protocol_builder_names(),
            action_family_names: native_host_dynamic_action_family_names(),
            platform_host_rule: "translate app_core dynamic row and VV instance specs without inventing platform-local control identity",
        },
    ]
}

#[cfg(feature = "vv-paste")]
const fn native_host_dynamic_protocol_builder_names() -> &'static [&'static str] {
    &["native_host_clip_row_specs", "native_host_vv_select_specs"]
}

#[cfg(not(feature = "vv-paste"))]
const fn native_host_dynamic_protocol_builder_names() -> &'static [&'static str] {
    &["native_host_clip_row_specs"]
}

#[cfg(feature = "vv-paste")]
const fn native_host_dynamic_action_family_names() -> &'static [&'static str] {
    &["ClipRow", "VvSelect"]
}

#[cfg(not(feature = "vv-paste"))]
const fn native_host_dynamic_action_family_names() -> &'static [&'static str] {
    &["ClipRow"]
}

impl NativeComponentAction {
    pub(crate) const fn action_name(self) -> &'static str {
        match self {
            Self::HostUi(action) => action.action_name(),
            Self::MainTool(action) => action.action_name(),
            Self::ClipRow(action) => action.action_name(),
            Self::Row(action) => action.action_name(),
            Self::SearchControl(action) => action.action_name(),
            Self::StatusMenu(action) => action.action_name(),
            Self::Settings(action) => action.action_name(),
            Self::SettingsControl(action) => action.action_name(),
            Self::SettingsPlatform(action) => action.action_name(),
            Self::SettingsGroup(action) => action.action_name(),
            Self::EditText(action) => action.action_name(),
            #[cfg(feature = "vv-paste")]
            Self::VvSelect(action) => action.action_name(),
            Self::Dialog(action) => action.action_name(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeComponentSpec {
    pub(crate) id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) kind: NativeComponentKind,
    pub(crate) bounds: UiRect,
    pub(crate) style_role: NativeButtonStyleRole,
    pub(crate) action: NativeComponentAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeClipRowSpec {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) bounds: UiRect,
    pub(crate) action: NativeHostClipRowAction,
}

impl NativeClipRowSpec {
    pub(crate) fn new(index: usize, item_id: i64, label: String) -> Self {
        Self {
            id: format!("clip.row.{}", index + 1),
            label,
            bounds: native_host_clip_row_bounds(index),
            action: NativeHostClipRowAction::new(index, item_id),
        }
    }

    pub(crate) const fn width(&self) -> i32 {
        self.bounds.width()
    }

    pub(crate) const fn height(&self) -> i32 {
        self.bounds.height()
    }
}

impl HostComponent for NativeClipRowSpec {
    type Action = NativeHostClipRowAction;

    fn id(&self) -> &str {
        &self.id
    }

    fn label(&self) -> &str {
        &self.label
    }

    fn bounds(&self) -> UiRect {
        self.bounds
    }

    fn action(&self) -> &Self::Action {
        &self.action
    }
}

#[cfg(feature = "vv-paste")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeVvSelectSpec {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) bounds: UiRect,
    pub(crate) action: NativeHostVvSelectAction,
}

#[cfg(feature = "vv-paste")]
impl NativeVvSelectSpec {
    pub(crate) fn new(index: usize, bounds: UiRect) -> Self {
        let action = NativeHostVvSelectAction::new(index);
        Self {
            id: format!("vv.select.{}", index + 1),
            label: action.button_label(),
            bounds,
            action,
        }
    }

    pub(crate) const fn width(&self) -> i32 {
        self.bounds.width()
    }

    pub(crate) const fn height(&self) -> i32 {
        self.bounds.height()
    }
}

#[cfg(feature = "vv-paste")]
impl HostComponent for NativeVvSelectSpec {
    type Action = NativeHostVvSelectAction;

    fn id(&self) -> &str {
        &self.id
    }

    fn label(&self) -> &str {
        &self.label
    }

    fn bounds(&self) -> UiRect {
        self.bounds
    }

    fn action(&self) -> &Self::Action {
        &self.action
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeComponentInstanceSpec {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) kind: NativeComponentKind,
    pub(crate) bounds: UiRect,
    pub(crate) action: NativeComponentAction,
}

impl NativeComponentInstanceSpec {
    pub(crate) fn clip_row_button(index: usize, item_id: i64, label: String) -> Self {
        Self::from_clip_row_spec(NativeClipRowSpec::new(index, item_id, label))
    }

    pub(crate) fn from_clip_row_spec(spec: NativeClipRowSpec) -> Self {
        Self {
            id: spec.id,
            label: spec.label,
            kind: NativeComponentKind::Button,
            bounds: spec.bounds,
            action: NativeComponentAction::ClipRow(spec.action),
        }
    }

    #[cfg(feature = "vv-paste")]
    pub(crate) fn vv_select_button(index: usize, bounds: UiRect) -> Self {
        Self::from_vv_select_spec(NativeVvSelectSpec::new(index, bounds))
    }

    #[cfg(feature = "vv-paste")]
    pub(crate) fn from_vv_select_spec(spec: NativeVvSelectSpec) -> Self {
        Self {
            id: spec.id,
            label: spec.label,
            kind: NativeComponentKind::Button,
            bounds: spec.bounds,
            action: NativeComponentAction::VvSelect(spec.action),
        }
    }

    pub(crate) const fn width(&self) -> i32 {
        self.bounds.width()
    }

    pub(crate) const fn height(&self) -> i32 {
        self.bounds.height()
    }
}

pub(crate) const NATIVE_HOST_CLIP_ROW_CAPACITY: usize = 64;

pub(crate) const fn native_host_clip_row_bounds(index: usize) -> UiRect {
    let top = 120 - index as i32 * 28;
    UiRect::new(64, top, 576, top + 22)
}

pub(crate) fn native_host_clip_row_specs(
    items: &[NativeHostClipListItemProjection],
    capacity: usize,
) -> Vec<NativeClipRowSpec> {
    (0..capacity)
        .map(|index| {
            if let Some(item) = items.get(index) {
                NativeClipRowSpec::new(
                    index,
                    item.id,
                    native_host_projected_clip_list_item_label(item),
                )
            } else {
                NativeClipRowSpec::new(index, 0, String::new())
            }
        })
        .collect()
}

pub(crate) fn native_host_clip_row_component_specs(
    items: &[NativeHostClipListItemProjection],
    capacity: usize,
) -> Vec<NativeComponentInstanceSpec> {
    native_host_clip_row_specs(items, capacity)
        .into_iter()
        .map(NativeComponentInstanceSpec::from_clip_row_spec)
        .collect()
}

pub(crate) const fn native_host_search_input_specs(
) -> [NativeSearchInputSpec<NativeHostSearchControlAction>; 1] {
    [NativeSearchInputSpec::new(
        "main.search",
        NativeHostSearchControlAction::UpdateText.placeholder(),
        UiRect::new(120, 296, 520, 324),
        NativeHostSearchControlAction::UpdateText,
    )]
}

pub(crate) const fn native_host_search_component_specs() -> [NativeComponentSpec; 1] {
    let specs = native_host_search_input_specs();
    [NativeComponentSpec::search_input(
        specs[0].id,
        specs[0].action,
        UiRect::new(120, 296, 520, 324),
    )]
}

pub(crate) fn native_host_status_menu_item_specs(
) -> Vec<NativeMenuItemSpec<NativeHostStatusMenuAction>> {
    vec![
        NativeMenuItemSpec::new(
            "status.toggle_window",
            NativeHostStatusMenuAction::ToggleWindow.menu_label(),
            NativeHostStatusMenuAction::ToggleWindow,
        )
        .with_icon_name("window-new-symbolic")
        .with_accelerator_key("z"),
        NativeMenuItemSpec::new(
            "status.toggle_capture",
            NativeHostStatusMenuAction::ToggleClipboardCapture.menu_label(),
            NativeHostStatusMenuAction::ToggleClipboardCapture,
        )
        .with_icon_name("media-record-symbolic")
        .with_accelerator_key("c"),
        #[cfg(feature = "lan-sync")]
        NativeMenuItemSpec::new(
            "status.toggle_lan_sync",
            NativeHostStatusMenuAction::ToggleLanSync.menu_label(),
            NativeHostStatusMenuAction::ToggleLanSync,
        )
        .with_icon_name("network-wireless-symbolic")
        .with_accelerator_key("l"),
        NativeMenuItemSpec::new(
            "status.exit",
            NativeHostStatusMenuAction::Exit.menu_label(),
            NativeHostStatusMenuAction::Exit,
        )
        .with_icon_name("application-exit-symbolic")
        .with_accelerator_key("q")
        .starting_section(),
    ]
}

pub(crate) fn native_host_status_menu_component_specs() -> Vec<NativeComponentSpec> {
    native_host_status_menu_item_specs()
        .into_iter()
        .map(|spec| NativeComponentSpec::status_menu_item(spec.id, spec.action))
        .collect()
}

impl NativeComponentSpec {
    pub(crate) const fn host_button(
        id: &'static str,
        action: NativeHostUiAction,
        bounds: UiRect,
    ) -> Self {
        let spec = NativeButtonSpec::new(id, action.button_label(), bounds, action);
        Self {
            id,
            label: spec.label,
            kind: NativeComponentKind::Button,
            bounds: spec.bounds,
            style_role: spec.style_role,
            action: NativeComponentAction::HostUi(spec.action),
        }
    }

    pub(crate) const fn row_button(
        id: &'static str,
        action: NativeHostRowAction,
        bounds: UiRect,
    ) -> Self {
        let spec = NativeButtonSpec::new(id, action.button_label(), bounds, action);
        Self {
            id,
            label: spec.label,
            kind: NativeComponentKind::Button,
            bounds: spec.bounds,
            style_role: spec.style_role,
            action: NativeComponentAction::Row(spec.action),
        }
    }

    pub(crate) const fn search_input(
        id: &'static str,
        action: NativeHostSearchControlAction,
        bounds: UiRect,
    ) -> Self {
        let spec = NativeSearchInputSpec::new(id, action.placeholder(), bounds, action);
        Self {
            id,
            label: spec.placeholder,
            kind: NativeComponentKind::SearchInput,
            bounds: spec.bounds,
            style_role: NativeButtonStyleRole::Plain,
            action: NativeComponentAction::SearchControl(spec.action),
        }
    }

    pub(crate) const fn status_menu_item(
        id: &'static str,
        action: NativeHostStatusMenuAction,
    ) -> Self {
        let spec = NativeMenuItemSpec::new(id, action.menu_label(), action);
        Self {
            id,
            label: spec.label,
            kind: NativeComponentKind::MenuItem,
            bounds: UiRect::new(0, 0, 0, 0),
            style_role: NativeButtonStyleRole::Plain,
            action: NativeComponentAction::StatusMenu(spec.action),
        }
    }

    pub(crate) const fn main_tool_button(
        id: &'static str,
        action: NativeHostMainToolAction,
        bounds: UiRect,
    ) -> Self {
        let spec = NativeButtonSpec::new(id, action.button_label(), bounds, action);
        Self {
            id,
            label: spec.label,
            kind: NativeComponentKind::Button,
            bounds: spec.bounds,
            style_role: spec.style_role,
            action: NativeComponentAction::MainTool(spec.action),
        }
    }

    pub(crate) const fn settings_button(
        id: &'static str,
        action: NativeHostSettingsAction,
        bounds: UiRect,
    ) -> Self {
        let spec = NativeButtonSpec::new(id, action.button_label(), bounds, action)
            .with_style_role(match action {
                NativeHostSettingsAction::Save => NativeButtonStyleRole::Suggested,
                NativeHostSettingsAction::Close | NativeHostSettingsAction::OpenConfig => {
                    NativeButtonStyleRole::Plain
                }
            });
        Self {
            id,
            label: spec.label,
            kind: NativeComponentKind::Button,
            bounds: spec.bounds,
            style_role: spec.style_role,
            action: NativeComponentAction::Settings(spec.action),
        }
    }

    pub(crate) const fn settings_control_button(
        id: &'static str,
        action: NativeHostSettingsControlAction,
        bounds: UiRect,
    ) -> Self {
        let spec = NativeButtonSpec::new(id, action.button_label(), bounds, action);
        Self {
            id,
            label: spec.label,
            kind: NativeComponentKind::Button,
            bounds: spec.bounds,
            style_role: spec.style_role,
            action: NativeComponentAction::SettingsControl(spec.action),
        }
    }

    pub(crate) const fn settings_control_toggle(
        id: &'static str,
        action: NativeHostSettingsControlAction,
        bounds: UiRect,
    ) -> Self {
        let spec = NativeToggleSpec::new(id, action.button_label(), bounds, action);
        Self {
            id,
            label: spec.label,
            kind: NativeComponentKind::Toggle,
            bounds: spec.bounds,
            style_role: NativeButtonStyleRole::Plain,
            action: NativeComponentAction::SettingsControl(spec.action),
        }
    }

    pub(crate) const fn settings_control_dropdown(
        id: &'static str,
        action: NativeHostSettingsControlAction,
        bounds: UiRect,
    ) -> Self {
        let spec = NativeDropdownSpec::new(id, action.button_label(), bounds, action);
        Self {
            id,
            label: spec.label,
            kind: NativeComponentKind::Dropdown,
            bounds: spec.bounds,
            style_role: NativeButtonStyleRole::Plain,
            action: NativeComponentAction::SettingsControl(spec.action),
        }
    }

    pub(crate) const fn settings_platform_button(
        id: &'static str,
        action: NativeHostSettingsPlatformAction,
        bounds: UiRect,
    ) -> Self {
        let spec = NativeButtonSpec::new(id, action.button_label(), bounds, action);
        Self {
            id,
            label: spec.label,
            kind: NativeComponentKind::Button,
            bounds: spec.bounds,
            style_role: spec.style_role,
            action: NativeComponentAction::SettingsPlatform(spec.action),
        }
    }

    pub(crate) const fn settings_group_button(
        id: &'static str,
        action: NativeHostSettingsGroupAction,
        bounds: UiRect,
    ) -> Self {
        let spec = NativeButtonSpec::new(id, action.button_label(), bounds, action)
            .with_style_role(match action {
                NativeHostSettingsGroupAction::Delete => NativeButtonStyleRole::Destructive,
                NativeHostSettingsGroupAction::ShowRecords
                | NativeHostSettingsGroupAction::ShowPhrases
                | NativeHostSettingsGroupAction::Add
                | NativeHostSettingsGroupAction::Rename
                | NativeHostSettingsGroupAction::MoveUp
                | NativeHostSettingsGroupAction::MoveDown => NativeButtonStyleRole::Plain,
            });
        Self {
            id,
            label: spec.label,
            kind: NativeComponentKind::Button,
            bounds: spec.bounds,
            style_role: spec.style_role,
            action: NativeComponentAction::SettingsGroup(spec.action),
        }
    }

    pub(crate) const fn edit_text_button(
        id: &'static str,
        action: NativeHostEditTextAction,
        bounds: UiRect,
    ) -> Self {
        let spec = NativeButtonSpec::new(id, action.button_label(), bounds, action)
            .with_style_role(match action {
                NativeHostEditTextAction::Save => NativeButtonStyleRole::Suggested,
                NativeHostEditTextAction::Cancel => NativeButtonStyleRole::Plain,
            });
        Self {
            id,
            label: spec.label,
            kind: NativeComponentKind::Button,
            bounds: spec.bounds,
            style_role: spec.style_role,
            action: NativeComponentAction::EditText(spec.action),
        }
    }

    pub(crate) const fn dialog_button(
        id: &'static str,
        action: NativeHostDialogAction,
        bounds: UiRect,
    ) -> Self {
        let spec = NativeButtonSpec::new(id, action.button_label(), bounds, action);
        Self {
            id,
            label: spec.label,
            kind: NativeComponentKind::Button,
            bounds: spec.bounds,
            style_role: spec.style_role,
            action: NativeComponentAction::Dialog(spec.action),
        }
    }

    pub(crate) const fn width(self) -> i32 {
        self.bounds.width()
    }

    pub(crate) const fn height(self) -> i32 {
        self.bounds.height()
    }
}

#[cfg(feature = "vv-paste")]
pub(crate) fn native_host_vv_select_specs(
    plan: &MainVvPopupRenderPlan,
    window_width: i32,
    window_height: i32,
) -> Vec<NativeVvSelectSpec> {
    plan.text_commands
        .iter()
        .filter(|command| command.role == MainVvPopupTextRole::RowPreview)
        .enumerate()
        .map(|(index, command)| {
            NativeVvSelectSpec::new(
                index,
                UiRect::new(
                    window_width - 96,
                    window_height - command.rect.bottom - 4,
                    window_width - 12,
                    window_height - command.rect.bottom + 20,
                ),
            )
        })
        .collect()
}

pub(crate) fn native_host_vv_select_component_specs(
    plan: &MainVvPopupRenderPlan,
    window_width: i32,
    window_height: i32,
) -> Vec<NativeComponentInstanceSpec> {
    #[cfg(not(feature = "vv-paste"))]
    {
        let _ = (plan, window_width, window_height);
        Vec::new()
    }
    #[cfg(feature = "vv-paste")]
    {
        native_host_vv_select_specs(plan, window_width, window_height)
            .into_iter()
            .map(NativeComponentInstanceSpec::from_vv_select_spec)
            .collect()
    }
}

pub(crate) const fn native_host_main_action_button_specs(
) -> [NativeButtonSpec<NativeHostUiAction>; 4] {
    [
        NativeButtonSpec::new(
            "main.toggle_search",
            NativeHostUiAction::ToggleSearch.button_label(),
            UiRect::new(96, 160, 192, 192),
            NativeHostUiAction::ToggleSearch,
        ),
        NativeButtonSpec::new(
            "main.open_settings",
            NativeHostUiAction::OpenSettings.button_label(),
            UiRect::new(212, 160, 308, 192),
            NativeHostUiAction::OpenSettings,
        ),
        NativeButtonSpec::new(
            "main.hide_window",
            NativeHostUiAction::HideWindow.button_label(),
            UiRect::new(328, 160, 424, 192),
            NativeHostUiAction::HideWindow,
        ),
        NativeButtonSpec::new(
            "main.close_window",
            NativeHostUiAction::CloseWindow.button_label(),
            UiRect::new(444, 160, 540, 192),
            NativeHostUiAction::CloseWindow,
        ),
    ]
}

pub(crate) const fn native_host_main_action_component_specs() -> [NativeComponentSpec; 4] {
    let specs = native_host_main_action_button_specs();
    [
        NativeComponentSpec::host_button(specs[0].id, specs[0].action, specs[0].bounds),
        NativeComponentSpec::host_button(specs[1].id, specs[1].action, specs[1].bounds),
        NativeComponentSpec::host_button(specs[2].id, specs[2].action, specs[2].bounds),
        NativeComponentSpec::host_button(specs[3].id, specs[3].action, specs[3].bounds),
    ]
}

#[cfg(feature = "vv-paste")]
pub(crate) const fn native_host_main_tool_button_specs(
) -> [NativeButtonSpec<NativeHostMainToolAction>; 4] {
    [
        NativeButtonSpec::new(
            "main.row_menu",
            NativeHostMainToolAction::RowMenu.button_label(),
            UiRect::new(24, 200, 88, 268),
            NativeHostMainToolAction::RowMenu,
        ),
        NativeButtonSpec::new(
            "main.group_filter",
            NativeHostMainToolAction::GroupFilter.button_label(),
            UiRect::new(4, 164, 92, 196),
            NativeHostMainToolAction::GroupFilter,
        ),
        NativeButtonSpec::new(
            "main.vv_popup",
            NativeHostMainToolAction::VvPopup.button_label(),
            UiRect::new(4, 128, 92, 160),
            NativeHostMainToolAction::VvPopup,
        ),
        NativeButtonSpec::new(
            "main.vv_trigger",
            NativeHostMainToolAction::VvTrigger.button_label(),
            UiRect::new(4, 92, 92, 124),
            NativeHostMainToolAction::VvTrigger,
        ),
    ]
}

#[cfg(not(feature = "vv-paste"))]
pub(crate) const fn native_host_main_tool_button_specs(
) -> [NativeButtonSpec<NativeHostMainToolAction>; 2] {
    [
        NativeButtonSpec::new(
            "main.row_menu",
            NativeHostMainToolAction::RowMenu.button_label(),
            UiRect::new(24, 200, 88, 268),
            NativeHostMainToolAction::RowMenu,
        ),
        NativeButtonSpec::new(
            "main.group_filter",
            NativeHostMainToolAction::GroupFilter.button_label(),
            UiRect::new(4, 164, 92, 196),
            NativeHostMainToolAction::GroupFilter,
        ),
    ]
}

pub(crate) fn native_host_main_tool_component_specs() -> Vec<NativeComponentSpec> {
    native_host_main_tool_button_specs()
        .into_iter()
        .map(|spec| NativeComponentSpec::main_tool_button(spec.id, spec.action, spec.bounds))
        .collect()
}

pub(crate) const fn native_host_edit_text_button_specs(
) -> [NativeButtonSpec<NativeHostEditTextAction>; 2] {
    [
        NativeButtonSpec::new(
            "edit.save",
            NativeHostEditTextAction::Save.button_label(),
            UiRect::new(20, 28, 116, 60),
            NativeHostEditTextAction::Save,
        )
        .with_style_role(NativeButtonStyleRole::Suggested),
        NativeButtonSpec::new(
            "edit.cancel",
            NativeHostEditTextAction::Cancel.button_label(),
            UiRect::new(128, 28, 224, 60),
            NativeHostEditTextAction::Cancel,
        ),
    ]
}

pub(crate) const fn native_host_edit_text_component_specs() -> [NativeComponentSpec; 2] {
    let specs = native_host_edit_text_button_specs();
    [
        NativeComponentSpec::edit_text_button(specs[0].id, specs[0].action, specs[0].bounds),
        NativeComponentSpec::edit_text_button(specs[1].id, specs[1].action, specs[1].bounds),
    ]
}

pub(crate) const fn native_host_settings_group_button_specs(
) -> [NativeButtonSpec<NativeHostSettingsGroupAction>; 7] {
    [
        NativeButtonSpec::new(
            "settings.group.show_records",
            NativeHostSettingsGroupAction::ShowRecords.button_label(),
            UiRect::new(662, 574, 754, 602),
            NativeHostSettingsGroupAction::ShowRecords,
        ),
        NativeButtonSpec::new(
            "settings.group.show_phrases",
            NativeHostSettingsGroupAction::ShowPhrases.button_label(),
            UiRect::new(762, 574, 854, 602),
            NativeHostSettingsGroupAction::ShowPhrases,
        ),
        NativeButtonSpec::new(
            "settings.group.add",
            NativeHostSettingsGroupAction::Add.button_label(),
            UiRect::new(430, 376, 508, 404),
            NativeHostSettingsGroupAction::Add,
        ),
        NativeButtonSpec::new(
            "settings.group.rename",
            NativeHostSettingsGroupAction::Rename.button_label(),
            UiRect::new(516, 376, 594, 404),
            NativeHostSettingsGroupAction::Rename,
        ),
        NativeButtonSpec::new(
            "settings.group.delete",
            NativeHostSettingsGroupAction::Delete.button_label(),
            UiRect::new(602, 376, 680, 404),
            NativeHostSettingsGroupAction::Delete,
        )
        .with_style_role(NativeButtonStyleRole::Destructive),
        NativeButtonSpec::new(
            "settings.group.move_up",
            NativeHostSettingsGroupAction::MoveUp.button_label(),
            UiRect::new(688, 376, 766, 404),
            NativeHostSettingsGroupAction::MoveUp,
        ),
        NativeButtonSpec::new(
            "settings.group.move_down",
            NativeHostSettingsGroupAction::MoveDown.button_label(),
            UiRect::new(774, 376, 852, 404),
            NativeHostSettingsGroupAction::MoveDown,
        ),
    ]
}

pub(crate) const fn native_host_settings_group_component_specs() -> [NativeComponentSpec; 7] {
    let specs = native_host_settings_group_button_specs();
    [
        NativeComponentSpec::settings_group_button(specs[0].id, specs[0].action, specs[0].bounds),
        NativeComponentSpec::settings_group_button(specs[1].id, specs[1].action, specs[1].bounds),
        NativeComponentSpec::settings_group_button(specs[2].id, specs[2].action, specs[2].bounds),
        NativeComponentSpec::settings_group_button(specs[3].id, specs[3].action, specs[3].bounds),
        NativeComponentSpec::settings_group_button(specs[4].id, specs[4].action, specs[4].bounds),
        NativeComponentSpec::settings_group_button(specs[5].id, specs[5].action, specs[5].bounds),
        NativeComponentSpec::settings_group_button(specs[6].id, specs[6].action, specs[6].bounds),
    ]
}

pub(crate) fn native_host_settings_control_button_specs(
) -> Vec<NativeButtonSpec<NativeHostSettingsControlAction>> {
    vec![
        NativeButtonSpec::new(
            "settings.control.autostart",
            NativeHostSettingsControlAction::ToggleAutostart.button_label(),
            UiRect::new(24, 46, 156, 78),
            NativeHostSettingsControlAction::ToggleAutostart,
        ),
        NativeButtonSpec::new(
            "settings.control.capture",
            NativeHostSettingsControlAction::ToggleClipboardCapture.button_label(),
            UiRect::new(168, 46, 300, 78),
            NativeHostSettingsControlAction::ToggleClipboardCapture,
        ),
        #[cfg(feature = "lan-sync")]
        NativeButtonSpec::new(
            "settings.control.lan_sync",
            NativeHostSettingsControlAction::ToggleLanSync.button_label(),
            UiRect::new(24, 86, 156, 118),
            NativeHostSettingsControlAction::ToggleLanSync,
        ),
        #[cfg(feature = "cloud-sync")]
        NativeButtonSpec::new(
            "settings.control.cloud_sync",
            NativeHostSettingsControlAction::ToggleCloudSync.button_label(),
            UiRect::new(312, 86, 444, 118),
            NativeHostSettingsControlAction::ToggleCloudSync,
        ),
        #[cfg(any(feature = "cloud-sync", feature = "lan-sync"))]
        NativeButtonSpec::new(
            "settings.control.sync_mode",
            NativeHostSettingsControlAction::OpenSyncModeDropdown.button_label(),
            UiRect::new(456, 86, 588, 118),
            NativeHostSettingsControlAction::OpenSyncModeDropdown,
        ),
    ]
}

pub(crate) fn native_host_settings_toggle_specs(
) -> Vec<NativeToggleSpec<NativeHostSettingsControlAction>> {
    vec![
        NativeToggleSpec::new(
            "settings.control.autostart",
            NativeHostSettingsControlAction::ToggleAutostart.button_label(),
            UiRect::new(24, 46, 156, 78),
            NativeHostSettingsControlAction::ToggleAutostart,
        ),
        NativeToggleSpec::new(
            "settings.control.capture",
            NativeHostSettingsControlAction::ToggleClipboardCapture.button_label(),
            UiRect::new(168, 46, 300, 78),
            NativeHostSettingsControlAction::ToggleClipboardCapture,
        ),
        #[cfg(feature = "lan-sync")]
        NativeToggleSpec::new(
            "settings.control.lan_sync",
            NativeHostSettingsControlAction::ToggleLanSync.button_label(),
            UiRect::new(24, 86, 156, 118),
            NativeHostSettingsControlAction::ToggleLanSync,
        ),
        #[cfg(feature = "cloud-sync")]
        NativeToggleSpec::new(
            "settings.control.cloud_sync",
            NativeHostSettingsControlAction::ToggleCloudSync.button_label(),
            UiRect::new(312, 86, 444, 118),
            NativeHostSettingsControlAction::ToggleCloudSync,
        ),
    ]
}

pub(crate) fn native_host_settings_dropdown_specs(
) -> Vec<NativeDropdownSpec<NativeHostSettingsControlAction>> {
    vec![
        #[cfg(any(feature = "cloud-sync", feature = "lan-sync"))]
        NativeDropdownSpec::new(
            "settings.control.sync_mode",
            NativeHostSettingsControlAction::OpenSyncModeDropdown.button_label(),
            UiRect::new(456, 86, 588, 118),
            NativeHostSettingsControlAction::OpenSyncModeDropdown,
        )
        .with_options(&NATIVE_SYNC_MODE_DROPDOWN_OPTIONS),
    ]
}

pub(crate) fn native_host_settings_control_component_specs() -> Vec<NativeComponentSpec> {
    let mut specs = native_host_settings_toggle_specs()
        .into_iter()
        .map(|spec| NativeComponentSpec::settings_control_toggle(spec.id, spec.action, spec.bounds))
        .collect::<Vec<_>>();
    specs.extend(
        native_host_settings_dropdown_specs()
            .into_iter()
            .map(|spec| {
                NativeComponentSpec::settings_control_dropdown(spec.id, spec.action, spec.bounds)
            }),
    );
    specs
}

pub(crate) const fn native_host_settings_platform_button_specs(
) -> [NativeButtonSpec<NativeHostSettingsPlatformAction>; 3] {
    [
        NativeButtonSpec::new(
            "settings.platform.open_source",
            NativeHostSettingsPlatformAction::OpenSourceRepository.button_label(),
            UiRect::new(24, 46, 156, 78),
            NativeHostSettingsPlatformAction::OpenSourceRepository,
        ),
        NativeButtonSpec::new(
            "settings.platform.check_updates",
            NativeHostSettingsPlatformAction::CheckForUpdates.button_label(),
            UiRect::new(168, 46, 300, 78),
            NativeHostSettingsPlatformAction::CheckForUpdates,
        ),
        NativeButtonSpec::new(
            "settings.platform.wps_docs",
            NativeHostSettingsPlatformAction::OpenWpsTaskpaneDocs.button_label(),
            UiRect::new(312, 46, 444, 78),
            NativeHostSettingsPlatformAction::OpenWpsTaskpaneDocs,
        ),
    ]
}

pub(crate) const fn native_host_settings_platform_component_specs() -> [NativeComponentSpec; 3] {
    let specs = native_host_settings_platform_button_specs();
    [
        NativeComponentSpec::settings_platform_button(
            specs[0].id,
            specs[0].action,
            specs[0].bounds,
        ),
        NativeComponentSpec::settings_platform_button(
            specs[1].id,
            specs[1].action,
            specs[1].bounds,
        ),
        NativeComponentSpec::settings_platform_button(
            specs[2].id,
            specs[2].action,
            specs[2].bounds,
        ),
    ]
}

pub(crate) const fn native_host_dialog_button_specs(
) -> [NativeButtonSpec<NativeHostDialogAction>; 2] {
    [
        NativeButtonSpec::new(
            "settings.dialog.info",
            NativeHostDialogAction::ShowInfoMessage.button_label(),
            UiRect::new(456, 46, 588, 78),
            NativeHostDialogAction::ShowInfoMessage,
        ),
        NativeButtonSpec::new(
            "settings.dialog.confirm",
            NativeHostDialogAction::ConfirmQuestion.button_label(),
            UiRect::new(600, 46, 732, 78),
            NativeHostDialogAction::ConfirmQuestion,
        ),
    ]
}

pub(crate) const fn native_host_dialog_component_specs() -> [NativeComponentSpec; 2] {
    let specs = native_host_dialog_button_specs();
    [
        NativeComponentSpec::dialog_button(specs[0].id, specs[0].action, specs[0].bounds),
        NativeComponentSpec::dialog_button(specs[1].id, specs[1].action, specs[1].bounds),
    ]
}

pub(crate) const fn native_host_settings_action_button_specs(
) -> [NativeButtonSpec<NativeHostSettingsAction>; 3] {
    [
        NativeButtonSpec::new(
            "settings.action.save",
            NativeHostSettingsAction::Save.button_label(),
            UiRect::new(24, 8, 140, 40),
            NativeHostSettingsAction::Save,
        )
        .with_style_role(NativeButtonStyleRole::Suggested),
        NativeButtonSpec::new(
            "settings.action.close",
            NativeHostSettingsAction::Close.button_label(),
            UiRect::new(152, 8, 268, 40),
            NativeHostSettingsAction::Close,
        ),
        NativeButtonSpec::new(
            "settings.action.open_config",
            NativeHostSettingsAction::OpenConfig.button_label(),
            UiRect::new(280, 8, 396, 40),
            NativeHostSettingsAction::OpenConfig,
        ),
    ]
}

pub(crate) const fn native_host_settings_action_component_specs() -> [NativeComponentSpec; 3] {
    let specs = native_host_settings_action_button_specs();
    [
        NativeComponentSpec::settings_button(specs[0].id, specs[0].action, specs[0].bounds),
        NativeComponentSpec::settings_button(specs[1].id, specs[1].action, specs[1].bounds),
        NativeComponentSpec::settings_button(specs[2].id, specs[2].action, specs[2].bounds),
    ]
}

#[cfg(feature = "ai-actions")]
pub(crate) const fn native_host_row_action_button_specs(
) -> [NativeButtonSpec<NativeHostRowAction>; 10] {
    [
        NativeButtonSpec::new(
            "row.paste",
            NativeHostRowAction::Paste.button_label(),
            UiRect::new(96, 200, 200, 232),
            NativeHostRowAction::Paste,
        ),
        NativeButtonSpec::new(
            "row.copy",
            NativeHostRowAction::Copy.button_label(),
            UiRect::new(212, 200, 316, 232),
            NativeHostRowAction::Copy,
        ),
        NativeButtonSpec::new(
            "row.pin",
            NativeHostRowAction::Pin.button_label(),
            UiRect::new(328, 200, 432, 232),
            NativeHostRowAction::Pin,
        ),
        NativeButtonSpec::new(
            "row.to_phrase",
            NativeHostRowAction::ToPhrase.button_label(),
            UiRect::new(444, 200, 548, 232),
            NativeHostRowAction::ToPhrase,
        ),
        NativeButtonSpec::new(
            "row.delete",
            NativeHostRowAction::Delete.button_label(),
            UiRect::new(96, 164, 200, 196),
            NativeHostRowAction::Delete,
        ),
        NativeButtonSpec::new(
            "row.edit",
            NativeHostRowAction::Edit.button_label(),
            UiRect::new(212, 164, 316, 196),
            NativeHostRowAction::Edit,
        ),
        NativeButtonSpec::new(
            "row.open_path",
            NativeHostRowAction::OpenPath.button_label(),
            UiRect::new(328, 164, 432, 196),
            NativeHostRowAction::OpenPath,
        ),
        NativeButtonSpec::new(
            "row.open_folder",
            NativeHostRowAction::OpenFolder.button_label(),
            UiRect::new(444, 164, 548, 196),
            NativeHostRowAction::OpenFolder,
        ),
        NativeButtonSpec::new(
            "row.copy_path",
            NativeHostRowAction::CopyPath.button_label(),
            UiRect::new(96, 128, 200, 160),
            NativeHostRowAction::CopyPath,
        ),
        NativeButtonSpec::new(
            "row.text_translate",
            NativeHostRowAction::TextTranslate.button_label(),
            UiRect::new(212, 128, 316, 160),
            NativeHostRowAction::TextTranslate,
        ),
    ]
}

#[cfg(not(feature = "ai-actions"))]
pub(crate) const fn native_host_row_action_button_specs(
) -> [NativeButtonSpec<NativeHostRowAction>; 9] {
    [
        NativeButtonSpec::new(
            "row.paste",
            NativeHostRowAction::Paste.button_label(),
            UiRect::new(96, 200, 200, 232),
            NativeHostRowAction::Paste,
        ),
        NativeButtonSpec::new(
            "row.copy",
            NativeHostRowAction::Copy.button_label(),
            UiRect::new(212, 200, 316, 232),
            NativeHostRowAction::Copy,
        ),
        NativeButtonSpec::new(
            "row.pin",
            NativeHostRowAction::Pin.button_label(),
            UiRect::new(328, 200, 432, 232),
            NativeHostRowAction::Pin,
        ),
        NativeButtonSpec::new(
            "row.to_phrase",
            NativeHostRowAction::ToPhrase.button_label(),
            UiRect::new(444, 200, 548, 232),
            NativeHostRowAction::ToPhrase,
        ),
        NativeButtonSpec::new(
            "row.delete",
            NativeHostRowAction::Delete.button_label(),
            UiRect::new(96, 164, 200, 196),
            NativeHostRowAction::Delete,
        ),
        NativeButtonSpec::new(
            "row.edit",
            NativeHostRowAction::Edit.button_label(),
            UiRect::new(212, 164, 316, 196),
            NativeHostRowAction::Edit,
        ),
        NativeButtonSpec::new(
            "row.open_path",
            NativeHostRowAction::OpenPath.button_label(),
            UiRect::new(328, 164, 432, 196),
            NativeHostRowAction::OpenPath,
        ),
        NativeButtonSpec::new(
            "row.open_folder",
            NativeHostRowAction::OpenFolder.button_label(),
            UiRect::new(444, 164, 548, 196),
            NativeHostRowAction::OpenFolder,
        ),
        NativeButtonSpec::new(
            "row.copy_path",
            NativeHostRowAction::CopyPath.button_label(),
            UiRect::new(96, 128, 200, 160),
            NativeHostRowAction::CopyPath,
        ),
    ]
}

pub(crate) fn native_host_row_action_component_specs() -> Vec<NativeComponentSpec> {
    native_host_row_action_button_specs()
        .into_iter()
        .map(|spec| NativeComponentSpec::row_button(spec.id, spec.action, spec.bounds))
        .collect()
}

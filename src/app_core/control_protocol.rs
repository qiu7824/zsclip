use super::{Point, UiRect};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum SettingsComponentKind {
    Label,
    TextInput,
    Toggle,
    Dropdown,
    Button,
    AccentButton,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeControlFamily {
    StaticText,
    TextInput,
    Action,
}

impl SettingsComponentKind {
    pub(crate) const fn family(self) -> NativeControlFamily {
        match self {
            SettingsComponentKind::Label => NativeControlFamily::StaticText,
            SettingsComponentKind::TextInput => NativeControlFamily::TextInput,
            SettingsComponentKind::Toggle
            | SettingsComponentKind::Dropdown
            | SettingsComponentKind::Button
            | SettingsComponentKind::AccentButton => NativeControlFamily::Action,
        }
    }

    pub(crate) const fn is_action(self) -> bool {
        matches!(self.family(), NativeControlFamily::Action)
    }
}

pub(crate) trait NativeControlMapper {
    type ClassName: Copy + Eq;

    fn class_name(&self, kind: SettingsComponentKind) -> Self::ClassName;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeControlMapperOperation {
    ClassName,
}

impl NativeControlMapperOperation {
    pub(crate) const fn operation_name(self) -> &'static str {
        match self {
            Self::ClassName => "class_name",
        }
    }
}

pub(crate) const REQUIRED_NATIVE_CONTROL_MAPPER_OPERATIONS: [NativeControlMapperOperation; 1] =
    [NativeControlMapperOperation::ClassName];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettingsControlSpec {
    pub(crate) id: Option<i64>,
    pub(crate) text: String,
    pub(crate) kind: SettingsComponentKind,
    pub(crate) bounds: UiRect,
}

impl SettingsControlSpec {
    pub(crate) fn new(
        kind: SettingsComponentKind,
        id: Option<i64>,
        text: impl Into<String>,
        bounds: UiRect,
    ) -> Self {
        Self {
            id,
            text: text.into(),
            kind,
            bounds,
        }
    }

    pub(crate) fn action(
        kind: SettingsComponentKind,
        id: i64,
        text: impl Into<String>,
        bounds: UiRect,
    ) -> Self {
        debug_assert!(kind.is_action());
        Self::new(kind, Some(id), text, bounds)
    }

    pub(crate) fn label(text: impl Into<String>, bounds: UiRect) -> Self {
        Self::new(SettingsComponentKind::Label, None, text, bounds)
    }

    pub(crate) fn text_input(id: i64, text: impl Into<String>, bounds: UiRect) -> Self {
        Self::new(SettingsComponentKind::TextInput, Some(id), text, bounds)
    }

    pub(crate) const fn width(&self) -> i32 {
        self.bounds.right - self.bounds.left
    }

    pub(crate) const fn height(&self) -> i32 {
        self.bounds.bottom - self.bounds.top
    }
}

pub(crate) trait NativeSettingsControlHost {
    type Handle: Copy + Eq;

    fn create_control(&mut self, spec: &SettingsControlSpec) -> Self::Handle;
    fn destroy_control(&mut self, handle: Self::Handle);
    fn control_exists(&self, handle: Self::Handle) -> bool;
    fn set_control_visible(&mut self, handle: Self::Handle, visible: bool);
    fn set_control_enabled(&mut self, handle: Self::Handle, enabled: bool);
    fn set_control_bounds(&mut self, handle: Self::Handle, bounds: UiRect);
    fn control_at_point(&self, point: Point) -> Option<Self::Handle>;
    fn control_screen_bounds(&self, handle: Self::Handle) -> Option<UiRect>;
    fn control_text(&self, handle: Self::Handle) -> String;
    fn set_control_text(&mut self, handle: Self::Handle, text: &str);
    fn request_control_repaint(&mut self, handle: Self::Handle) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingsControlHostOperation {
    CreateControl,
    DestroyControl,
    ControlExists,
    SetControlVisible,
    SetControlEnabled,
    SetControlBounds,
    ControlAtPoint,
    ControlScreenBounds,
    ControlText,
    SetControlText,
    RequestControlRepaint,
}

impl SettingsControlHostOperation {
    pub(crate) const fn operation_name(self) -> &'static str {
        match self {
            Self::CreateControl => "create_control",
            Self::DestroyControl => "destroy_control",
            Self::ControlExists => "control_exists",
            Self::SetControlVisible => "set_control_visible",
            Self::SetControlEnabled => "set_control_enabled",
            Self::SetControlBounds => "set_control_bounds",
            Self::ControlAtPoint => "control_at_point",
            Self::ControlScreenBounds => "control_screen_bounds",
            Self::ControlText => "control_text",
            Self::SetControlText => "set_control_text",
            Self::RequestControlRepaint => "request_control_repaint",
        }
    }
}

pub(crate) const REQUIRED_SETTINGS_CONTROL_HOST_OPERATIONS: [SettingsControlHostOperation; 11] = [
    SettingsControlHostOperation::CreateControl,
    SettingsControlHostOperation::DestroyControl,
    SettingsControlHostOperation::ControlExists,
    SettingsControlHostOperation::SetControlVisible,
    SettingsControlHostOperation::SetControlEnabled,
    SettingsControlHostOperation::SetControlBounds,
    SettingsControlHostOperation::ControlAtPoint,
    SettingsControlHostOperation::ControlScreenBounds,
    SettingsControlHostOperation::ControlText,
    SettingsControlHostOperation::SetControlText,
    SettingsControlHostOperation::RequestControlRepaint,
];

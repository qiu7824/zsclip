#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UiHostSurface {
    MainWindow,
    SettingsWindow,
    SettingsDropdown,
    InputDialog,
    EditDialog,
}

impl UiHostSurface {
    pub(crate) const fn adapter_name(self) -> &'static str {
        match self {
            Self::MainWindow => "main_window_host_event_from_message",
            Self::SettingsWindow => "settings_window_host_event_from_message",
            Self::SettingsDropdown => "dropdown_window_host_event_from_message",
            Self::InputDialog => "input_dialog_host_event_from_message",
            Self::EditDialog => "edit_dialog_host_event_from_message",
        }
    }
}

pub(crate) const REQUIRED_UI_HOST_SURFACES: [UiHostSurface; 5] = [
    UiHostSurface::MainWindow,
    UiHostSurface::SettingsWindow,
    UiHostSurface::SettingsDropdown,
    UiHostSurface::InputDialog,
    UiHostSurface::EditDialog,
];

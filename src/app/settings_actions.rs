use super::prelude::*;

pub(super) struct WindowsSettingsActionExecutor {
    hwnd: HWND,
}

impl WindowsSettingsActionExecutor {
    pub(super) const fn new(hwnd: HWND) -> Self {
        Self { hwnd }
    }
}

impl SettingsActionExecutor for WindowsSettingsActionExecutor {
    type Context = SettingsWndState;

    fn execute_sync(&mut self, context: &mut Self::Context, action: SettingsAction) -> bool {
        unsafe { execute_settings_sync_action(self.hwnd, context, action) }
    }

    fn execute_group(&mut self, context: &mut Self::Context, action: SettingsAction) -> bool {
        unsafe { execute_settings_group_action(self.hwnd, context, action) }
    }

    fn execute_platform(&mut self, context: &mut Self::Context, action: SettingsAction) -> bool {
        unsafe { execute_settings_platform_action(self.hwnd, context, action) }
    }
}

use crate::zsui::core::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotkeySpec {
    pub accelerator: String,
    pub command: Command,
    pub enabled: bool,
}

impl HotkeySpec {
    pub fn new(accelerator: impl Into<String>, command: Command) -> Self {
        Self {
            accelerator: accelerator.into(),
            command,
            enabled: true,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

use crate::zsui::core::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MenuSpec {
    pub id: Option<String>,
    pub title: Option<String>,
    pub items: Vec<MenuItemSpec>,
}

impl MenuSpec {
    pub fn new() -> Self {
        Self {
            id: None,
            title: None,
            items: Vec::new(),
        }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn item(mut self, label: impl Into<String>, command: Command) -> Self {
        self.items.push(MenuItemSpec::command(label, command));
        self
    }

    pub fn separator(mut self) -> Self {
        self.items.push(MenuItemSpec::Separator);
        self
    }

    pub fn submenu(mut self, label: impl Into<String>, menu: MenuSpec) -> Self {
        self.items.push(MenuItemSpec::Submenu {
            id: None,
            label: label.into(),
            enabled: true,
            menu,
        });
        self
    }
}

impl Default for MenuSpec {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MenuItemSpec {
    Command {
        id: Option<String>,
        label: String,
        command: Command,
        enabled: bool,
        checked: bool,
        accelerator: Option<String>,
    },
    Separator,
    Submenu {
        id: Option<String>,
        label: String,
        enabled: bool,
        menu: MenuSpec,
    },
}

impl MenuItemSpec {
    pub fn command(label: impl Into<String>, command: Command) -> Self {
        Self::Command {
            id: None,
            label: label.into(),
            command,
            enabled: true,
            checked: false,
            accelerator: None,
        }
    }

    pub fn disabled(mut self) -> Self {
        if let Self::Command { enabled, .. } | Self::Submenu { enabled, .. } = &mut self {
            *enabled = false;
        }
        self
    }

    pub fn checked(mut self, checked_value: bool) -> Self {
        if let Self::Command { checked, .. } = &mut self {
            *checked = checked_value;
        }
        self
    }

    pub fn accelerator(mut self, value: impl Into<String>) -> Self {
        if let Self::Command { accelerator, .. } = &mut self {
            *accelerator = Some(value.into());
        }
        self
    }

    pub fn id(mut self, value: impl Into<String>) -> Self {
        match &mut self {
            Self::Command { id, .. } | Self::Submenu { id, .. } => *id = Some(value.into()),
            Self::Separator => {}
        }
        self
    }
}

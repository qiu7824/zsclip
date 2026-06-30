use crate::zsui::core::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SettingsValue {
    Bool(bool),
    Text(String),
    Number(f64),
    Choice(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SettingsItemKind {
    Toggle,
    Text,
    Number { min: Option<f64>, max: Option<f64> },
    Choice { options: Vec<String> },
    Button,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettingsItemSpec {
    pub id: String,
    pub label: String,
    pub kind: SettingsItemKind,
    pub description: Option<String>,
    pub default_value: Option<SettingsValue>,
    pub command: Option<Command>,
}

impl SettingsItemSpec {
    pub fn toggle(id: impl Into<String>, label: impl Into<String>, value: bool) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind: SettingsItemKind::Toggle,
            description: None,
            default_value: Some(SettingsValue::Bool(value)),
            command: None,
        }
    }

    pub fn button(id: impl Into<String>, label: impl Into<String>, command: Command) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind: SettingsItemKind::Button,
            description: None,
            default_value: None,
            command: Some(command),
        }
    }

    pub fn text(id: impl Into<String>, label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind: SettingsItemKind::Text,
            description: None,
            default_value: Some(SettingsValue::Text(value.into())),
            command: None,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettingsPageSpec {
    pub id: String,
    pub title: String,
    pub items: Vec<SettingsItemSpec>,
}

impl SettingsPageSpec {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            items: Vec::new(),
        }
    }

    pub fn item(mut self, item: SettingsItemSpec) -> Self {
        self.items.push(item);
        self
    }
}

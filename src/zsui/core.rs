use serde::{Deserialize, Serialize};
use std::fmt;

pub type ZsuiResult<T> = Result<T, ZsuiError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZsuiError {
    Unsupported { capability: String, reason: String },
    InvalidSpec { field: String, message: String },
    Host { operation: String, message: String },
}

impl ZsuiError {
    pub fn unsupported(capability: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Unsupported {
            capability: capability.into(),
            reason: reason.into(),
        }
    }

    pub fn invalid_spec(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidSpec {
            field: field.into(),
            message: message.into(),
        }
    }

    pub fn host(operation: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Host {
            operation: operation.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for ZsuiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported { capability, reason } => {
                write!(f, "unsupported zsui capability `{capability}`: {reason}")
            }
            Self::InvalidSpec { field, message } => {
                write!(f, "invalid zsui spec field `{field}`: {message}")
            }
            Self::Host { operation, message } => {
                write!(f, "zsui host operation `{operation}` failed: {message}")
            }
        }
    }
}

impl std::error::Error for ZsuiError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WindowId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrayId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HotkeyId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    ShowMainWindow,
    HideMainWindow,
    ToggleMainWindow,
    OpenQuickPanel,
    OpenSettings,
    CopySelection,
    PasteSelection,
    ReadClipboard,
    WriteClipboard,
    Quit,
    Custom { id: String, payload: Option<String> },
}

impl Command {
    pub fn custom(id: impl Into<String>) -> Self {
        Self::Custom {
            id: id.into(),
            payload: None,
        }
    }

    pub fn custom_with_payload(id: impl Into<String>, payload: impl Into<String>) -> Self {
        Self::Custom {
            id: id.into(),
            payload: Some(payload.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppEvent {
    Started,
    WindowCreated { window: WindowId },
    WindowShown { window: WindowId },
    WindowHidden { window: WindowId },
    TrayCommand { command: Command },
    MenuCommand { command: Command },
    HotkeyPressed { hotkey: HotkeyId, command: Command },
    ClipboardChanged,
    SettingsChanged { page: String, item: String },
    DialogClosed { response: DialogResponse },
    QuitRequested,
    HostDegraded { capability: String, reason: String },
    Custom { id: String, payload: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileDialogFilter {
    pub name: String,
    pub patterns: Vec<String>,
}

impl FileDialogFilter {
    pub fn new(
        name: impl Into<String>,
        patterns: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            name: name.into(),
            patterns: patterns.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileDialogSpec {
    pub title: String,
    pub current_path: Option<String>,
    pub filters: Vec<FileDialogFilter>,
    pub allow_multiple: bool,
}

impl FileDialogSpec {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            current_path: None,
            filters: Vec::new(),
            allow_multiple: false,
        }
    }

    pub fn current_path(mut self, path: impl Into<String>) -> Self {
        self.current_path = Some(path.into());
        self
    }

    pub fn filter(
        mut self,
        name: impl Into<String>,
        patterns: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.filters.push(FileDialogFilter::new(name, patterns));
        self
    }

    pub fn allow_multiple(mut self, allow_multiple: bool) -> Self {
        self.allow_multiple = allow_multiple;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DialogLevel {
    Info,
    Warning,
    Error,
    Question,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DialogButtons {
    Ok,
    OkCancel,
    YesNo,
    YesNoCancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DialogResponse {
    Ok,
    Cancel,
    Yes,
    No,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeDialogSpec {
    pub title: String,
    pub message: String,
    pub level: DialogLevel,
    pub buttons: DialogButtons,
}

impl NativeDialogSpec {
    pub fn message(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            level: DialogLevel::Info,
            buttons: DialogButtons::Ok,
        }
    }

    pub fn level(mut self, level: DialogLevel) -> Self {
        self.level = level;
        self
    }

    pub fn buttons(mut self, buttons: DialogButtons) -> Self {
        self.buttons = buttons;
        self
    }
}

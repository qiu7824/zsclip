use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClipboardData {
    Empty,
    Text(String),
    ImageRgba {
        width: usize,
        height: usize,
        bytes: Vec<u8>,
    },
    Files(Vec<String>),
}

impl ClipboardData {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    pub fn files(paths: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::Files(paths.into_iter().map(Into::into).collect())
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }
}

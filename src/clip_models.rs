#[derive(Clone, Debug)]
pub(crate) struct ClipGroup {
    pub(crate) id: i64,
    pub(crate) name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ClipKind {
    Text,
    Image,
    Phrase,
    Files,
}

#[derive(Clone, Debug)]
pub(crate) struct ClipItem {
    pub(crate) id: i64,
    pub(crate) kind: ClipKind,
    pub(crate) preview: String,
    pub(crate) text: Option<String>,
    pub(crate) file_paths: Option<Vec<String>>,
    pub(crate) image_bytes: Option<Vec<u8>>,
    pub(crate) image_path: Option<String>,
    pub(crate) image_width: usize,
    pub(crate) image_height: usize,
    pub(crate) pinned: bool,
    pub(crate) group_id: i64,
    pub(crate) created_at: String,
}

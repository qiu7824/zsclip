#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ZsIcon {
    App,
    Search,
    Settings,
    Minimize,
    Close,
    Text,
    Image,
    File,
    Folder,
    Pin,
    Delete,
    Copy,
    Paste,
    Edit,
    Group,
    Phrase,
}

impl ZsIcon {
    pub const fn asset_name(self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Search => "search",
            Self::Settings => "setting",
            Self::Minimize => "min",
            Self::Close => "exit",
            Self::Text | Self::Phrase => "text",
            Self::Image => "image",
            Self::File => "file",
            Self::Folder | Self::Group => "fold",
            Self::Pin => "top",
            Self::Delete => "del",
            Self::Copy => "copy",
            Self::Paste => "paste",
            Self::Edit => "edit",
        }
    }

    pub const fn gtk_symbolic_name(self) -> &'static str {
        match self {
            Self::App => "edit-paste-symbolic",
            Self::Search => "edit-find-symbolic",
            Self::Settings => "emblem-system-symbolic",
            Self::Minimize => "window-minimize-symbolic",
            Self::Close => "window-close-symbolic",
            Self::Text | Self::Phrase => "text-x-generic-symbolic",
            Self::Image => "image-x-generic-symbolic",
            Self::File => "text-x-generic-symbolic",
            Self::Folder | Self::Group => "folder-symbolic",
            Self::Pin => "view-pin-symbolic",
            Self::Delete => "user-trash-symbolic",
            Self::Copy => "edit-copy-symbolic",
            Self::Paste => "edit-paste-symbolic",
            Self::Edit => "document-edit-symbolic",
        }
    }
}

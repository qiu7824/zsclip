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

    pub const fn png_24_bytes(self) -> Option<&'static [u8]> {
        match self {
            Self::Search => Some(include_bytes!("../../assets/icons/search/search_24x24.png")),
            Self::Settings => Some(include_bytes!("../../assets/icons/setting/setting_24x24.png")),
            Self::Minimize => Some(include_bytes!("../../assets/icons/min/min_24x24.png")),
            Self::Close => Some(include_bytes!("../../assets/icons/exit/exit_24x24.png")),
            Self::Text | Self::Phrase => {
                Some(include_bytes!("../../assets/icons/text/text_24x24.png"))
            }
            Self::Image => Some(include_bytes!("../../assets/icons/image/image_24x24.png")),
            Self::File => Some(include_bytes!("../../assets/icons/file/file_24x24.png")),
            Self::Folder | Self::Group => {
                Some(include_bytes!("../../assets/icons/fold/fold_24x24.png"))
            }
            Self::Pin => Some(include_bytes!("../../assets/icons/top/top_24x24.png")),
            Self::Delete => Some(include_bytes!("../../assets/icons/del/del_24x24.png")),
            Self::App | Self::Copy | Self::Paste | Self::Edit => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ZsIcon;

    #[test]
    fn png_24_assets_cover_shared_clipboard_row_icons() {
        for icon in [
            ZsIcon::Text,
            ZsIcon::Phrase,
            ZsIcon::Image,
            ZsIcon::File,
            ZsIcon::Folder,
            ZsIcon::Pin,
            ZsIcon::Delete,
        ] {
            let bytes = icon.png_24_bytes().expect("row icon should have PNG asset");
            assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
        }
    }

    #[test]
    fn unavailable_png_assets_are_explicit() {
        assert!(ZsIcon::Copy.png_24_bytes().is_none());
        assert!(ZsIcon::Paste.png_24_bytes().is_none());
        assert!(ZsIcon::Edit.png_24_bytes().is_none());
    }
}

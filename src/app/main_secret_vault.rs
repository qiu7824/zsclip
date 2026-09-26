use super::prelude::*;
pub(super) const OPEN_VAULT: usize = 54001;
const SAVE_SECRET: usize = 54003;

fn command(id: usize, label: &str, enabled: bool) -> NativePopupMenuEntry {
    NativePopupMenuEntry::Command {
        id,
        label: label.into(),
        enabled,
        checked: false,
    }
}

pub(super) fn append_groups(mut entries: Vec<NativePopupMenuEntry>) -> Vec<NativePopupMenuEntry> {
    entries.push(NativePopupMenuEntry::Separator);
    entries.push(command(OPEN_VAULT, "密码与密钥", true));
    entries
}

pub(super) fn append_row_menu(
    mut entries: Vec<NativePopupMenuEntry>,
    kind: ClipKind,
    count: usize,
) -> Vec<NativePopupMenuEntry> {
    let can_save = count <= 1 && matches!(kind, ClipKind::Text | ClipKind::Phrase);
    entries.push(NativePopupMenuEntry::Separator);
    entries.push(NativePopupMenuEntry::Submenu {
        label: "密码与密钥".into(),
        enabled: true,
        entries: vec![
            command(SAVE_SECRET, "存入密码与密钥…", can_save),
            NativePopupMenuEntry::Separator,
            command(OPEN_VAULT, "打开密码与密钥", true),
        ],
    });
    entries
}

pub(super) unsafe fn dispatch(owner: HWND, id: usize, item: Option<&ClipItem>) -> bool {
    let importing = match id {
        OPEN_VAULT => false,
        SAVE_SECRET => true,
        _ => return false,
    };
    let import = if importing {
        let Some(item) = item.filter(|i| matches!(i.kind, ClipKind::Text | ClipKind::Phrase))
        else {
            return true;
        };
        let Some(text) = item.text.as_ref().filter(|t| !t.is_empty()) else {
            return true;
        };
        Some(text.clone())
    } else {
        None
    };
    super::secret_vault_ui::open_main_view(owner, import);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vault_import_is_only_available_for_single_text_and_management_is_always_available() {
        for (kind, count, can_import) in [
            (ClipKind::Text, 1, true),
            (ClipKind::Phrase, 1, true),
            (ClipKind::Text, 2, false),
            (ClipKind::Image, 1, false),
            (ClipKind::Files, 1, false),
        ] {
            let entries = append_row_menu(vec![], kind, count);
            let NativePopupMenuEntry::Submenu { entries, .. } = &entries[1] else {
                panic!("missing manager menu")
            };
            for entry in &entries[..1] {
                let NativePopupMenuEntry::Command { enabled, .. } = entry else {
                    panic!("missing save")
                };
                assert_eq!(*enabled, can_import);
            }
            for entry in &entries[2..] {
                let NativePopupMenuEntry::Command { enabled, .. } = entry else {
                    panic!("missing manager")
                };
                assert!(*enabled);
            }
        }
    }

    #[test]
    fn secret_group_has_one_unified_entry() {
        let groups = append_groups(vec![]);
        assert_eq!(groups.len(), 2);
        assert!(
            matches!(&groups[1], NativePopupMenuEntry::Command { id: OPEN_VAULT, label, .. } if label == "密码与密钥")
        );
    }
}

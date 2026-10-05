use std::path::{Path, PathBuf};

fn choose_data_directory(
    explicit: Option<PathBuf>,
    legacy: Option<PathBuf>,
    legacy_exists: bool,
    home: Option<PathBuf>,
    xdg: Option<PathBuf>,
    macos: bool,
) -> PathBuf {
    if let Some(path) = explicit.filter(|path| !path.as_os_str().is_empty()) {
        return path;
    }
    // An existing portable store stays authoritative. Never silently start a new
    // empty history because an older store is temporarily inaccessible/read-only.
    if legacy_exists {
        if let Some(path) = legacy.as_ref() {
            return path.clone();
        }
    }
    if macos {
        if let Some(home) = home {
            return home.join("Library/Application Support/ZSClip");
        }
    } else {
        if let Some(xdg) = xdg.filter(|path| path.is_absolute()) {
            return xdg.join("zsclip");
        }
        if let Some(home) = home {
            return home.join(".local/share/zsclip");
        }
    }
    legacy.unwrap_or_else(|| PathBuf::from("data"))
}

pub(crate) fn data_directory() -> PathBuf {
    let legacy = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|p| p.join("data")));
    let exists = legacy.as_ref().is_some_and(|path| path.try_exists().unwrap_or(true));
    choose_data_directory(
        std::env::var_os("ZSCLIP_DATA_DIR").map(PathBuf::from),
        legacy,
        exists,
        std::env::var_os("HOME").map(PathBuf::from),
        std::env::var_os("XDG_DATA_HOME").map(PathBuf::from),
        cfg!(target_os = "macos"),
    )
}

pub(crate) fn prepare_data_directory() -> Result<(), String> {
    let path = data_directory();
    std::fs::create_dir_all(&path)
        .map_err(|error| format!("Cannot open data directory {}: {error}", path.display()))?;
    verify_writable(&path).map_err(|error|format!("The existing data directory {} is not writable: {error}. History was not moved or replaced. Use a writable portable directory or select ZSCLIP_DATA_DIR.",path.display()))?;
    crate::native_protection::validate_profile(&path)
}

fn verify_writable(path: &Path) -> std::io::Result<()> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let file = path.join(format!(
        ".zsclip-write-check-{}-{stamp}",
        std::process::id()
    ));
    let handle = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&file)?;
    drop(handle);
    std::fs::remove_file(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_profile_is_shared_and_overrides_portable_or_user_locations() {
        let explicit = PathBuf::from("test-profile");
        for mac in [true, false] {
            assert_eq!(
                choose_data_directory(
                    Some(explicit.clone()),
                    Some("app/data".into()),
                    true,
                    Some("user".into()),
                    None,
                    mac
                ),
                explicit
            );
        }
    }
    #[test]
    fn existing_legacy_store_never_disappears_when_user_location_is_available() {
        assert_eq!(
            choose_data_directory(
                None,
                Some("portable/data".into()),
                true,
                Some("user".into()),
                None,
                true
            ),
            PathBuf::from("portable/data")
        );
    }
    #[test]
    fn new_installs_use_writable_user_directories_and_relative_xdg_is_ignored() {
        assert_eq!(
            choose_data_directory(None, None, false, Some("user".into()), None, true),
            PathBuf::from("user/Library/Application Support/ZSClip")
        );
        assert_eq!(
            choose_data_directory(
                None,
                None,
                false,
                Some("user".into()),
                Some("relative".into()),
                false
            ),
            PathBuf::from("user/.local/share/zsclip")
        );
        let absolute = std::env::current_dir().unwrap().join("xdg-fixture");
        assert_eq!(
            choose_data_directory(None, None, false, None, Some(absolute.clone()), false),
            absolute.join("zsclip")
        );
    }
}

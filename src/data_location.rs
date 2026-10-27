//! Per-user storage selection and restart-time migration of a closed profile.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

const MIGRATION_MARKER: &str = ".zsclip-location-migration.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Location {
    pub path: PathBuf,
    #[serde(default)]
    pub migrate_from: Option<PathBuf>,
}

#[derive(Serialize, Deserialize)]
struct MigrationProof {
    source: PathBuf,
    source_hash: String,
    target_hash: String,
}

pub(crate) fn config_file() -> Option<PathBuf> {
    #[cfg(test)]
    return Some(
        std::env::temp_dir()
            .join(format!("zsclip-location-test-{}", std::process::id()))
            .join("storage.json"),
    );
    #[cfg(not(test))]
    std::env::var_os("LOCALAPPDATA")
        .map(|path| PathBuf::from(path).join("ZSClip").join("storage.json"))
}

pub(crate) fn read(file: &Path) -> Result<Option<Location>, String> {
    let text = match fs::read(file) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("无法读取数据目录设置 {}：{error}", file.display())),
    };
    let location: Location = serde_json::from_slice(&text)
        .map_err(|error| format!("数据目录设置无效 {}：{error}", file.display()))?;
    validate_path(&location.path)?;
    if let Some(source) = &location.migrate_from {
        validate_path(source)?;
    }
    Ok(Some(location))
}

fn validate_path(path: &Path) -> Result<(), String> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("请填写不含 .. 的绝对文件夹路径。".into());
    }
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) => {
                use std::os::windows::fs::MetadataExt;
                if metadata.file_attributes() & 0x400 != 0 {
                    return Err(format!("数据目录不能使用目录链接：{}", ancestor.display()));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("无法检查目录 {}：{error}", ancestor.display())),
        }
    }
    Ok(())
}

fn path_key(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase()
}

fn nested(left: &Path, right: &Path) -> bool {
    let a = path_key(left);
    let b = path_key(right);
    a == b || a.starts_with(&(b + "\\"))
}

fn write_atomic(file: &Path, value: &impl Serialize) -> Result<(), String> {
    let parent = file.parent().ok_or("设置路径没有父目录")?;
    fs::create_dir_all(parent).map_err(|error| format!("无法创建设置目录：{error}"))?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temp = parent.join(format!(
        ".zsclip-location-{}-{stamp}.tmp",
        std::process::id()
    ));
    let result = (|| {
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|error| error.to_string())?;
        serde_json::to_writer_pretty(&mut output, value).map_err(|error| error.to_string())?;
        output
            .flush()
            .and_then(|_| output.sync_all())
            .map_err(|error| error.to_string())?;
        drop(output);
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn MoveFileExW(from: *const u16, to: *const u16, flags: u32) -> i32;
        }
        use std::os::windows::ffi::OsStrExt;
        let from: Vec<u16> = temp.as_os_str().encode_wide().chain([0]).collect();
        let to: Vec<u16> = file.as_os_str().encode_wide().chain([0]).collect();
        if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 1 | 8) } == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result.map_err(|error| format!("无法保存数据目录设置 {}：{error}", file.display()))
}

pub(crate) fn request_change(file: &Path, source: &Path, target: &Path) -> Result<(), String> {
    validate_path(source)?;
    validate_path(target)?;
    if !source.is_dir() {
        return Err(format!("原数据目录不存在：{}", source.display()));
    }
    if path_key(source) == path_key(target) {
        return write_atomic(
            file,
            &Location {
                path: source.into(),
                migrate_from: None,
            },
        );
    }
    if nested(source, target) || nested(target, source) {
        return Err("新旧数据目录不能相互包含。".into());
    }
    fs::create_dir_all(target).map_err(|error| format!("无法创建新数据目录：{error}"))?;
    if fs::read_dir(target)
        .map_err(|error| error.to_string())?
        .next()
        .is_some()
    {
        return Err("请选择空文件夹；现有文件不会被覆盖。".into());
    }
    let probe = target.join(format!(".zsclip-write-check-{}", std::process::id()));
    let output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .map_err(|error| format!("新数据目录不可写：{error}"))?;
    drop(output);
    fs::remove_file(&probe).map_err(|error| error.to_string())?;
    write_atomic(
        file,
        &Location {
            path: target.into(),
            migrate_from: Some(source.into()),
        },
    )
}

fn profile_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    fn visit(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
        for entry in
            fs::read_dir(dir).map_err(|error| format!("无法读取 {}：{error}", dir.display()))?
        {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path();
            let relative = path.strip_prefix(root).map_err(|error| error.to_string())?;
            if relative.components().count() == 1
                && matches!(
                    entry.file_name().to_str(),
                    Some(
                        "temp"
                            | "logs"
                            | "clipboard.db-wal"
                            | "clipboard.db-shm"
                            | MIGRATION_MARKER
                    )
                )
            {
                continue;
            }
            validate_path(&path)?;
            let kind = entry.file_type().map_err(|error| error.to_string())?;
            if kind.is_dir() {
                visit(root, &path, files)?;
            } else if kind.is_file() {
                files.push(relative.into());
            } else {
                return Err(format!("不支持的数据文件类型：{}", path.display()));
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    visit(root, root, &mut files)?;
    files.sort();
    Ok(files)
}

fn fingerprint(root: &Path, files: &[PathBuf]) -> Result<String, String> {
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    for path in files {
        hash.update(path.to_string_lossy().as_bytes());
        hash.update([0]);
        let mut file = fs::File::open(root.join(path)).map_err(|error| error.to_string())?;
        hash.update(
            file.metadata()
                .map_err(|error| error.to_string())?
                .len()
                .to_le_bytes(),
        );
        loop {
            let count = file.read(&mut buffer).map_err(|error| error.to_string())?;
            if count == 0 {
                break;
            }
            hash.update(&buffer[..count]);
        }
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn remap_images(source: &Path, target: &Path) -> Result<(), String> {
    let database = target.join("clipboard.db");
    if !database.is_file() {
        return Ok(());
    }
    let mut conn = rusqlite::Connection::open(database).map_err(|error| error.to_string())?;
    let has_images: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info('items') WHERE name='image_path')",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if has_images {
        let tx = conn.transaction().map_err(|error| error.to_string())?;
        let rows = {
            let mut query = tx
                .prepare("SELECT id,image_path FROM items WHERE image_path IS NOT NULL")
                .map_err(|error| error.to_string())?;
            let rows = query
                .query_map([], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|error| error.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?
        };
        for (id, path) in rows {
            let old_path = Path::new(&path);
            if nested(old_path, source) && path_key(old_path) != path_key(source) {
                let relative: PathBuf = old_path
                    .components()
                    .skip(source.components().count())
                    .collect();
                if relative
                    .components()
                    .any(|c| !matches!(c, Component::Normal(_)))
                {
                    continue;
                }
                let new_path = target.join(&relative);
                tx.execute(
                    "UPDATE items SET image_path=? WHERE id=?",
                    rusqlite::params![new_path.to_string_lossy(), id],
                )
                .map_err(|error| error.to_string())?;
            }
        }
        tx.commit().map_err(|error| error.to_string())?;
    }
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;")
        .map_err(|error| error.to_string())?;
    let check: String = conn
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if check != "ok" {
        return Err(format!("新目录数据库校验失败：{check}"));
    }
    Ok(())
}

/// Called only after acquiring the application single-instance mutex, before
/// settings, database connections and background workers are initialized.
pub(crate) fn activate(file: &Path) -> Result<Option<PathBuf>, String> {
    let Some(mut location) = read(file)? else {
        return Ok(None);
    };
    if let Some(source) = location.migrate_from.as_ref() {
        if nested(source, &location.path) || nested(&location.path, source) {
            return Err("新旧数据目录不能相互包含。".into());
        }
        let target = &location.path;
        crate::db_runtime::with_exclusive_db_snapshot(&source.join("clipboard.db"), || {
            let files = profile_files(source)?;
            let source_hash = fingerprint(source, &files)?;
            fs::create_dir_all(target).map_err(|error| error.to_string())?;
            let marker = target.join(MIGRATION_MARKER);
            validate_path(&marker)?;
            if marker.is_file() {
                let proof: MigrationProof =
                    serde_json::from_slice(&fs::read(&marker).map_err(|error| error.to_string())?)
                        .map_err(|error| error.to_string())?;
                if path_key(&proof.source) != path_key(source)
                    || proof.source_hash != source_hash
                    || proof.target_hash != fingerprint(target, &profile_files(target)?)?
                {
                    return Err("迁移期间数据发生变化，新旧目录均已保留，请重新选择空目录。".into());
                }
                return Ok(());
            }
            if fs::read_dir(target)
                .map_err(|error| error.to_string())?
                .next()
                .is_some()
            {
                return Err("新数据目录已有文件，迁移已停止；原数据仍完整保留。".into());
            }
            for relative in &files {
                let destination = target.join(relative);
                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                }
                let origin = source.join(relative);
                let mut input = fs::File::open(&origin)
                    .map_err(|error| format!("无法读取 {}：{error}", origin.display()))?;
                let mut output = fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&destination)
                    .map_err(|error| format!("无法创建 {}：{error}", destination.display()))?;
                std::io::copy(&mut input, &mut output)
                    .map_err(|error| format!("无法复制到 {}：{error}", destination.display()))?;
                output.sync_all().map_err(|error| error.to_string())?;
            }
            if fingerprint(source, &files)? != source_hash
                || fingerprint(target, &files)? != source_hash
            {
                return Err("数据复制校验失败，原目录和复制文件均已保留。".into());
            }
            remap_images(source, target)?;
            write_atomic(
                &marker,
                &MigrationProof {
                    source: source.clone(),
                    source_hash,
                    target_hash: fingerprint(target, &profile_files(target)?)?,
                },
            )
        })?;
        location.migrate_from = None;
        write_atomic(file, &location)?;
        let _ = fs::remove_file(location.path.join(MIGRATION_MARKER));
    }
    if !location.path.is_dir() {
        return Err(format!(
            "配置的数据目录不存在：{}。历史记录未切换到其他目录。",
            location.path.display()
        ));
    }
    Ok(Some(location.path))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root =
                std::env::temp_dir().join(format!("zsclip-storage-{}-{stamp}", std::process::id()));
            fs::create_dir(&root).unwrap();
            Self(root)
        }
        fn source(&self) -> PathBuf {
            let path = self.0.join("旧资料 中文");
            fs::create_dir_all(&path).unwrap();
            path
        }
        fn config(&self) -> PathBuf {
            self.0.join("config/storage.json")
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn restart_migration_keeps_latest_history_images_secrets_and_originals() {
        let fixture = Fixture::new();
        let source = fixture.source();
        let target = fixture.0.join("新目录 with spaces");
        fs::create_dir(source.join("images")).unwrap();
        fs::write(source.join("images/a.png"), b"synthetic-image").unwrap();
        fs::write(
            source.join("settings.json"),
            br#"{"clipboard_capture_enabled":true}"#,
        )
        .unwrap();
        fs::write(source.join("vault.dat"), b"synthetic-encrypted-vault").unwrap();
        let connection = rusqlite::Connection::open(source.join("clipboard.db")).unwrap();
        connection.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE items(id INTEGER PRIMARY KEY, text_data TEXT, image_path TEXT);").unwrap();
        request_change(&fixture.config(), &source, &target).unwrap();
        // Writes between selecting the destination and quitting must be copied.
        connection
            .execute(
                "INSERT INTO items VALUES(1,'latest',?)",
                [source.join("images/a.png").to_string_lossy().as_ref()],
            )
            .unwrap();
        assert!(fs::metadata(source.join("clipboard.db-wal")).unwrap().len() > 0);
        assert_eq!(activate(&fixture.config()).unwrap(), Some(target.clone()));
        drop(connection);
        assert_eq!(
            fs::read(target.join("vault.dat")).unwrap(),
            b"synthetic-encrypted-vault"
        );
        assert_eq!(
            fs::read(target.join("images/a.png")).unwrap(),
            b"synthetic-image"
        );
        let copied = rusqlite::Connection::open(target.join("clipboard.db")).unwrap();
        let row: (String, String) = copied
            .query_row("SELECT text_data,image_path FROM items", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap();
        assert_eq!(row.0, "latest");
        assert_eq!(PathBuf::from(row.1), target.join("images/a.png"));
        assert!(source.join("clipboard.db").is_file());
        assert!(source.join("vault.dat").is_file());
        assert!(read(&fixture.config())
            .unwrap()
            .unwrap()
            .migrate_from
            .is_none());
        assert_eq!(activate(&fixture.config()).unwrap(), Some(target));
    }

    #[test]
    fn occupied_nested_and_relative_destinations_do_not_change_selection() {
        let fixture = Fixture::new();
        let source = fixture.source();
        request_change(&fixture.config(), &source, &source).unwrap();
        let before = fs::read(fixture.config()).unwrap();
        let occupied = fixture.0.join("occupied");
        fs::create_dir(&occupied).unwrap();
        fs::write(occupied.join("keep.txt"), b"preserve").unwrap();
        for target in [
            occupied.clone(),
            source.join("nested"),
            fixture.0.clone(),
            PathBuf::from("relative"),
        ] {
            assert!(request_change(&fixture.config(), &source, &target).is_err());
            assert_eq!(fs::read(fixture.config()).unwrap(), before);
        }
        assert_eq!(fs::read(occupied.join("keep.txt")).unwrap(), b"preserve");
    }

    #[test]
    fn startup_refuses_destination_changes_without_overwriting_history() {
        let fixture = Fixture::new();
        let source = fixture.source();
        fs::write(source.join("settings.json"), b"{}").unwrap();
        let target = fixture.0.join("target");
        request_change(&fixture.config(), &source, &target).unwrap();
        fs::write(target.join("unrelated.txt"), b"keep").unwrap();
        assert!(activate(&fixture.config()).is_err());
        assert_eq!(fs::read(target.join("unrelated.txt")).unwrap(), b"keep");
        assert_eq!(fs::read(source.join("settings.json")).unwrap(), b"{}");
        assert_eq!(
            read(&fixture.config()).unwrap().unwrap().migrate_from,
            Some(source)
        );
    }

    #[test]
    fn completed_copy_can_finish_configuration_after_an_interruption() {
        let fixture = Fixture::new();
        let source = fixture.source();
        let target = fixture.0.join("target");
        fs::write(source.join("settings.json"), b"{}").unwrap();
        request_change(&fixture.config(), &source, &target).unwrap();
        fs::copy(source.join("settings.json"), target.join("settings.json")).unwrap();
        write_atomic(
            &target.join(MIGRATION_MARKER),
            &MigrationProof {
                source: source.clone(),
                source_hash: fingerprint(&source, &profile_files(&source).unwrap()).unwrap(),
                target_hash: fingerprint(&target, &profile_files(&target).unwrap()).unwrap(),
            },
        )
        .unwrap();
        assert_eq!(activate(&fixture.config()).unwrap(), Some(target.clone()));
        assert!(!target.join(MIGRATION_MARKER).exists());
    }

    #[test]
    fn invalid_configuration_is_reported_without_silently_selecting_another_store() {
        let fixture = Fixture::new();
        fs::create_dir_all(fixture.config().parent().unwrap()).unwrap();
        fs::write(fixture.config(), b"broken").unwrap();
        assert!(activate(&fixture.config()).is_err());
        assert_eq!(fs::read(fixture.config()).unwrap(), b"broken");
    }
}

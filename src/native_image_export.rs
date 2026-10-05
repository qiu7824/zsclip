use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

const MAX_IMAGE_BYTES: usize = 256 * 1024 * 1024;

pub(crate) fn save_native_item_png(
    item_id: i64,
    path: &Path,
    expected_generation: u64,
) -> Result<(), String> {
    if !path.extension().and_then(|extension| extension.to_str()).is_some_and(|extension| extension.eq_ignore_ascii_case("png")) {
        return Err("Choose a PNG filename".into());
    }
    crate::db_runtime::with_shared_app_data_generation(expected_generation, || {
        let revision = crate::db_runtime::search_protection_revision()
            .map_err(|error| error.to_string())?;
        let png = encode_native_item_png(item_id)?;
        if crate::db_runtime::current_app_data_generation() != expected_generation {
            return Err("Image data changed before export".into());
        }
        write_png_atomically(path, &png, || {
            crate::db_runtime::current_app_data_generation() == expected_generation
                && crate::db_runtime::search_protection_revision().ok().as_ref() == Some(&revision)
        })
    })
    .unwrap_or_else(|| Err("Image data changed before export".into()))
}

fn write_png_atomically(
    path: &Path,
    bytes: &[u8],
    can_commit: impl FnOnce() -> bool,
) -> Result<(), String> {
    static NEXT_TEMP: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    struct TemporaryFile(PathBuf);
    impl Drop for TemporaryFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temporary = None;
    for _ in 0..8 {
        let counter = NEXT_TEMP.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let candidate = parent.join(format!(
            ".zsclip-image-{}-{stamp}-{counter}.tmp",
            std::process::id()
        ));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => {
                temporary = Some((TemporaryFile(candidate), file));
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.to_string()),
        }
    }
    let (temporary, mut file) =
        temporary.ok_or_else(|| "Unable to create export file".to_string())?;
    if let Ok(metadata) = std::fs::metadata(path) {
        file.set_permissions(metadata.permissions()).map_err(|error| error.to_string())?;
    }
    file.write_all(bytes).map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())?;
    drop(file);
    if !can_commit() {
        return Err("Image data changed before export".into());
    }
    std::fs::rename(&temporary.0, path).map_err(|error| error.to_string())
}

pub(crate) fn encode_native_item_png(item_id: i64) -> Result<Vec<u8>, String> {
    let generation = crate::db_runtime::current_app_data_generation();
    let revision =
        crate::db_runtime::search_protection_revision().map_err(|error| error.to_string())?;
    let item = crate::db_runtime::native_clip_item(item_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Image is unavailable".to_string())?;
    if item.kind != crate::app_core::ClipKind::Image {
        return Err("The selected record is not an image".into());
    }
    let expected_raw_size = item
        .image_width
        .checked_mul(item.image_height)
        .and_then(|size| size.checked_mul(4));
    let stored_pixels = item
        .image_bytes
        .as_ref()
        .is_some_and(|bytes| expected_raw_size == Some(bytes.len()) && !bytes.is_empty());
    let bytes = if let Some(bytes) = item.image_bytes.filter(|bytes| !bytes.is_empty()) {
        bytes
    } else if let Some(path) = item.image_path {
        let file = std::fs::File::open(path).map_err(|error| error.to_string())?;
        if file.metadata().map_err(|error| error.to_string())?.len() > MAX_IMAGE_BYTES as u64 {
            return Err("Image exceeds the export size limit".into());
        }
        let mut bytes = Vec::new();
        file.take(MAX_IMAGE_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        bytes
    } else {
        return Err("Image data is unavailable".into());
    };
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err("Image exceeds the export size limit".into());
    }
    let png = if stored_pixels {
        encode_rgba_png(&bytes, item.image_width, item.image_height)?
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        let mut reader = png::Decoder::new(Cursor::new(&bytes))
            .read_info()
            .map_err(|error| error.to_string())?;
        if reader.output_buffer_size() > MAX_IMAGE_BYTES {
            return Err("Image exceeds the export size limit".into());
        }
        let mut decoded = vec![0; reader.output_buffer_size()];
        reader
            .next_frame(&mut decoded)
            .map_err(|error| error.to_string())?;
        drop(reader);
        bytes
    } else if bytes.starts_with(b"BM") {
        let (width, height) =
            image::ImageReader::with_format(Cursor::new(&bytes), image::ImageFormat::Bmp)
                .into_dimensions()
                .map_err(|error| error.to_string())?;
        if u64::from(width)
            .saturating_mul(u64::from(height))
            .saturating_mul(4)
            > MAX_IMAGE_BYTES as u64
        {
            return Err("Image exceeds the export size limit".into());
        }
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(width);
        limits.max_image_height = Some(height);
        limits.max_alloc = Some(MAX_IMAGE_BYTES as u64);
        let mut reader =
            image::ImageReader::with_format(Cursor::new(&bytes), image::ImageFormat::Bmp);
        reader.limits(limits);
        let image = reader
            .decode()
            .map_err(|error| error.to_string())?
            .to_rgba8();
        encode_rgba_png(
            image.as_raw(),
            image.width() as usize,
            image.height() as usize,
        )?
    } else {
        encode_rgba_png(&bytes, item.image_width, item.image_height)?
    };
    if generation != crate::db_runtime::current_app_data_generation()
        || revision
            != crate::db_runtime::search_protection_revision().map_err(|error| error.to_string())?
    {
        return Err("Image data changed before export".into());
    }
    Ok(png)
}

fn encode_rgba_png(bytes: &[u8], width: usize, height: usize) -> Result<Vec<u8>, String> {
    let expected = width
        .checked_mul(height)
        .and_then(|size| size.checked_mul(4));
    if width == 0 || height == 0 || expected != Some(bytes.len()) || bytes.len() > MAX_IMAGE_BYTES {
        return Err("Invalid image dimensions or pixel data".into());
    }
    let width = u32::try_from(width).map_err(|_| "Invalid image width")?;
    let height = u32::try_from(height).map_err(|_| "Invalid image height")?;
    let mut output = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut output, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|error| error.to_string())?;
        writer
            .write_image_data(bytes)
            .map_err(|error| error.to_string())?;
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ExportDirectory(PathBuf);
    impl ExportDirectory {
        fn new() -> Self {
            let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
            let path = std::env::temp_dir().join(format!("zsclip-png-export-{}-{stamp}", std::process::id()));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for ExportDirectory {
        fn drop(&mut self) {
            let parent = std::env::temp_dir().canonicalize().unwrap();
            if let Ok(path) = self.0.canonicalize() {
                if path.parent() == Some(parent.as_path()) {
                    let _ = std::fs::remove_dir_all(path);
                }
            }
        }
    }

    #[test]
    fn cancelled_native_image_export_preserves_existing_file_and_cleans_temp() {
        let directory = ExportDirectory::new();
        let destination = directory.0.join("selected.png");
        std::fs::write(&destination, b"original file").unwrap();
        let png = encode_rgba_png(&[10, 20, 30, 40], 1, 1).unwrap();
        assert!(write_png_atomically(&destination, &png, || false).is_err());
        assert_eq!(std::fs::read(&destination).unwrap(), b"original file");
        assert_eq!(std::fs::read_dir(&directory.0).unwrap().count(), 1);
        write_png_atomically(&destination, &png, || true).unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), png);
        assert_eq!(std::fs::read_dir(&directory.0).unwrap().count(), 1);
    }

    #[test]
    fn stale_native_image_dialog_cannot_replace_a_file_after_database_restore() {
        let directory = ExportDirectory::new();
        let destination = directory.0.join("selected.png");
        std::fs::write(&destination, b"original file").unwrap();
        let stale_generation = crate::db_runtime::current_app_data_generation().wrapping_add(2);
        assert!(save_native_item_png(1, &destination, stale_generation).is_err());
        assert_eq!(std::fs::read(&destination).unwrap(), b"original file");
        assert_eq!(std::fs::read_dir(&directory.0).unwrap().count(), 1);
        assert!(save_native_item_png(1, &directory.0.join("invalid.jpg"), stale_generation).is_err());
        assert!(!directory.0.join("invalid.jpg").exists());
    }
    #[test]
    fn native_png_export_preserves_dimensions_and_alpha() {
        let pixels = [1, 2, 3, 255, 100, 150, 200, 40];
        let png = encode_rgba_png(&pixels, 2, 1).unwrap();
        let mut decoder = png::Decoder::new(Cursor::new(png)).read_info().unwrap();
        let mut output = vec![0; decoder.output_buffer_size()];
        let info = decoder.next_frame(&mut output).unwrap();
        assert_eq!(
            (info.width, info.height, info.color_type),
            (2, 1, png::ColorType::Rgba)
        );
        assert_eq!(&output[..info.buffer_size()], &pixels);
        assert!(encode_rgba_png(&pixels, 2, 2).is_err());
        assert!(encode_rgba_png(&[], usize::MAX, usize::MAX).is_err());
    }

    #[test]
    fn native_png_export_does_not_misread_rgba_pixels_as_a_file_header() {
        crate::db_runtime::with_test_protected_texts(&[], || {
            crate::db_runtime::with_test_db(|| {
                let inserted = crate::db_runtime::insert_native_clipboard_image(
                    0,
                    &[66, 77, 0, 255],
                    1,
                    1,
                    "test",
                )?;
                let png = encode_native_item_png(inserted.item_id.unwrap()).unwrap();
                let mut decoder = png::Decoder::new(Cursor::new(png)).read_info().unwrap();
                let mut rgba = vec![0; decoder.output_buffer_size()];
                decoder.next_frame(&mut rgba).unwrap();
                assert_eq!(rgba, [66, 77, 0, 255]);
                Ok(())
            })
        })
        .unwrap();
    }
}

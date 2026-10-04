use super::prelude::*;
use std::io::Write;

fn encode_png(bytes: &[u8], width: usize, height: usize) -> Result<Vec<u8>, String> {
    if width == 0
        || height == 0
        || width > 32768
        || height > 32768
        || width.checked_mul(height).and_then(|n| n.checked_mul(4)) != Some(bytes.len())
    {
        return Err("图片尺寸或像素数据无效".to_string());
    }
    let mut png_data = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_data, width as u32, height as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Fast);
        let mut writer = encoder.write_header().map_err(|error| error.to_string())?;
        writer
            .write_image_data(bytes)
            .map_err(|error| error.to_string())?;
        writer.finish().map_err(|error| error.to_string())?;
    }
    Ok(png_data)
}

fn write_png_atomically(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("保存目录无效")?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let temporary = parent.join(format!(".zsclip-save-{}-{stamp}.tmp", std::process::id()));
    let result = (|| -> std::io::Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|error| error.to_string())
}

pub(super) unsafe fn save_image_as(hwnd: HWND, item: &ClipItem) {
    let name = format!("ZSClip-{}.png", item.id.max(0));
    let result = crate::platform::file_dialog::WindowsFileDialogHost::new()
        .save_png(hwnd, tr("另存为 PNG", "Save as PNG"), &name)
        .and_then(|path| {
            let Some(path) = path else {
                return Ok(());
            };
            let (bytes, width, height) = ensure_item_image_bytes(item)
                .ok_or_else(|| tr("无法读取图片数据", "Unable to read image data").to_string())?;
            let png = encode_png(&bytes, width, height)?;
            write_png_atomically(&path, &png)
        });
    if let Err(error) = result {
        platform_dialog::WindowsDialogHost::new().show_message(
            hwnd,
            tr("图片保存失败", "Image save failed"),
            &error,
            NativeDialogLevel::Error,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_save_preserves_pixels_and_alpha() {
        let source = [255, 0, 0, 255, 0, 128, 255, 64];
        let png = encode_png(&source, 2, 1).unwrap();
        let mut reader = png::Decoder::new(std::io::Cursor::new(png))
            .read_info()
            .unwrap();
        let mut output = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut output).unwrap();
        assert_eq!((info.width, info.height), (2, 1));
        assert_eq!(&output[..info.buffer_size()], &source);
        assert!(encode_png(&source, 3, 1).is_err());
    }

    #[test]
    fn atomic_png_save_replaces_only_chosen_file() {
        let root = std::env::temp_dir().join(format!(
            "zsclip-png-save-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("中文 图片.png");
        fs::write(&path, b"old").unwrap();
        let png = encode_png(&[0, 1, 2, 3], 1, 1).unwrap();
        write_png_atomically(&path, &png).unwrap();
        assert_eq!(fs::read(&path).unwrap(), png);
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        fs::remove_file(path).unwrap();
        fs::remove_dir(root).unwrap();
    }
}

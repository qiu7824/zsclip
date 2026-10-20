use crate::app_core::{NativeFileDialogHost, NativeFileDialogRequest};

pub(crate) struct WindowsFileDialogHost;

impl WindowsFileDialogHost {
    pub(crate) const fn new() -> Self {
        Self
    }

    pub(crate) fn save_png(
        &self,
        owner: windows_sys::Win32::Foundation::HWND,
        title: &str,
        file_name: &str,
    ) -> Result<Option<std::path::PathBuf>, String> {
        use windows_sys::Win32::UI::Controls::Dialogs::{
            CommDlgExtendedError, GetSaveFileNameW, OFN_EXPLORER, OFN_NOCHANGEDIR,
            OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST, OPENFILENAMEW,
        };
        let title = crate::platform::string::to_wide(title);
        let extension = crate::platform::string::to_wide("png");
        let filter = "PNG (*.png)\0*.png\0\0".encode_utf16().collect::<Vec<_>>();
        let mut path = vec![0u16; 32768];
        let name = file_name
            .encode_utf16()
            .take(path.len() - 1)
            .collect::<Vec<_>>();
        path[..name.len()].copy_from_slice(&name);
        let mut dialog: OPENFILENAMEW = unsafe { std::mem::zeroed() };
        dialog.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        dialog.hwndOwner = owner;
        dialog.lpstrFilter = filter.as_ptr();
        dialog.nFilterIndex = 1;
        dialog.lpstrFile = path.as_mut_ptr();
        dialog.nMaxFile = path.len() as u32;
        dialog.lpstrTitle = title.as_ptr();
        dialog.lpstrDefExt = extension.as_ptr();
        dialog.Flags = OFN_EXPLORER | OFN_NOCHANGEDIR | OFN_OVERWRITEPROMPT | OFN_PATHMUSTEXIST;
        if unsafe { GetSaveFileNameW(&mut dialog) } == 0 {
            let error = unsafe { CommDlgExtendedError() };
            return if error == 0 {
                Ok(None)
            } else {
                Err(format!("保存对话框错误: {error:#x}"))
            };
        }
        let end = path
            .iter()
            .position(|unit| *unit == 0)
            .unwrap_or(path.len());
        let path = String::from_utf16(&path[..end]).map_err(|_| "保存路径编码无效".to_string())?;
        let path = std::path::PathBuf::from(path);
        if !path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("png"))
        {
            return Err("请使用 .png 扩展名保存图片".to_string());
        }
        Ok(Some(path))
    }
}

fn open_file_filter(name: &str, pattern: &str) -> Vec<u16> {
    // Common dialogs use pairs of NUL-terminated strings and a final NUL.
    let name = name.replace('\0', "");
    let pattern = pattern.replace('\0', "");
    format!("{name}\0{pattern}\0All Files (*.*)\0*.*\0\0")
        .encode_utf16()
        .collect()
}

impl NativeFileDialogHost for WindowsFileDialogHost {
    fn pick_file(&self, request: NativeFileDialogRequest<'_>) -> Result<Option<String>, String> {
        use windows_sys::Win32::UI::{
            Controls::Dialogs::{
                CommDlgExtendedError, GetOpenFileNameW, OFN_EXPLORER, OFN_FILEMUSTEXIST,
                OFN_NOCHANGEDIR, OFN_PATHMUSTEXIST, OPENFILENAMEW,
            },
            Input::KeyboardAndMouse::GetActiveWindow,
        };
        let title = crate::platform::string::to_wide(request.title);
        let filter = open_file_filter(request.filter_name, request.filter_pattern);
        let current: Vec<u16> = request.current_path.encode_utf16().collect();
        let mut path = vec![0u16; 32768];
        if current.len() >= path.len() || current.contains(&0) {
            return Err("文件路径过长或包含无效字符".to_string());
        }
        let current_path = std::path::Path::new(request.current_path);
        if current_path.is_file() {
            path[..current.len()].copy_from_slice(&current);
        }
        let initial_directory = current_path.parent().filter(|parent| parent.is_dir())
            .map(|parent| crate::platform::string::to_wide(&parent.to_string_lossy()));
        let mut dialog: OPENFILENAMEW = unsafe { std::mem::zeroed() };
        dialog.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        dialog.hwndOwner = unsafe { GetActiveWindow() };
        dialog.lpstrFilter = filter.as_ptr();
        dialog.nFilterIndex = 1;
        dialog.lpstrFile = path.as_mut_ptr();
        dialog.nMaxFile = path.len() as u32;
        dialog.lpstrTitle = title.as_ptr();
        dialog.lpstrInitialDir = initial_directory.as_ref()
            .map_or(std::ptr::null(), |directory| directory.as_ptr());
        dialog.Flags = OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;
        if unsafe { GetOpenFileNameW(&mut dialog) } == 0 {
            let error = unsafe { CommDlgExtendedError() };
            return if error == 0 {
                Ok(None)
            } else {
                Err(format!("打开文件对话框失败: {error:#x}"))
            };
        }
        let end = path.iter().position(|unit| *unit == 0).unwrap_or(path.len());
        String::from_utf16(&path[..end])
            .map(Some)
            .map_err(|_| "文件路径编码无效".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_filter_keeps_unicode_and_multiple_audio_extensions() {
        let filter = open_file_filter("音频文件", "*.wav;*.mp3;*.wma");
        assert_eq!(String::from_utf16(&filter).unwrap(),
            "音频文件\0*.wav;*.mp3;*.wma\0All Files (*.*)\0*.*\0\0");
    }
}

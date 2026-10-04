use std::os::windows::process::CommandExt;
use std::process::Command;

use base64::Engine;

use crate::app_core::{NativeFileDialogHost, NativeFileDialogRequest};

const CREATE_NO_WINDOW_FLAG: u32 = 0x08000000;

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

fn encode_powershell_script(script: &str) -> String {
    let bytes: Vec<u8> = script
        .encode_utf16()
        .flat_map(|u| u.to_le_bytes())
        .collect();
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn run_hidden_powershell_encoded(script: &str, args: &[&str]) -> Result<String, String> {
    let encoded = encode_powershell_script(script);
    let out = Command::new("powershell")
        .creation_flags(CREATE_NO_WINDOW_FLAG)
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-EncodedCommand")
        .arg(encoded)
        .args(args)
        .output()
        .map_err(|e| format!("启动 PowerShell 失败: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        Err(if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            "PowerShell 执行失败".to_string()
        })
    }
}

impl NativeFileDialogHost for WindowsFileDialogHost {
    fn pick_file(&self, request: NativeFileDialogRequest<'_>) -> Result<Option<String>, String> {
        let script = r#"
Add-Type -AssemblyName System.Windows.Forms
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$dlg = New-Object System.Windows.Forms.OpenFileDialog
$dlg.Filter = "$($args[1])|$($args[2])|All Files|*.*"
$dlg.Title = $args[0]
$dlg.Multiselect = $false
if ($args.Count -gt 3 -and -not [string]::IsNullOrWhiteSpace($args[3])) {
  $current = $args[3]
  if (Test-Path $current) {
    $dlg.FileName = $current
    $parent = Split-Path -Parent $current
    if (Test-Path $parent) { $dlg.InitialDirectory = $parent }
  } else {
    $parent = Split-Path -Parent $current
    if (Test-Path $parent) { $dlg.InitialDirectory = $parent }
  }
}
if ($dlg.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
  Write-Output $dlg.FileName
}
"#;
        let out = run_hidden_powershell_encoded(
            script,
            &[
                request.title,
                request.filter_name,
                request.filter_pattern,
                request.current_path,
            ],
        )?;
        if out.trim().is_empty() {
            Ok(None)
        } else {
            Ok(Some(out))
        }
    }
}

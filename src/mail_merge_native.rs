use arboard::Clipboard;
use std::fs;
use std::mem::{size_of, zeroed};
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::ptr::{null, null_mut};
use std::sync::OnceLock;
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::Gdi::{BeginPaint, CreateFontW, CreateSolidBrush, DEFAULT_GUI_FONT, DeleteObject, EndPaint, FillRect, GetStockObject, PAINTSTRUCT, SetBkColor, SetBkMode, SetTextColor},
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Controls::{DRAWITEMSTRUCT, ODS_SELECTED},
        Input::KeyboardAndMouse::{keybd_event, KEYEVENTF_KEYUP, VK_CONTROL, VK_SHIFT},
        WindowsAndMessaging::*,
    },
};

use crate::i18n::translate;
use crate::shell::load_icons;
use crate::ui::{draw_round_rect, draw_text_ex, Theme};
use crate::win_system_ui::{
    apply_dark_mode_to_window, apply_window_corner_preference, create_settings_component,
    draw_settings_button_component, force_foreground_window, get_window_text, send_ctrl_v, to_wide,
    SettingsComponentKind,
};

unsafe extern "system" {
    fn EnableWindow(hwnd: HWND, benable: i32) -> i32;
    fn IsWindowEnabled(hwnd: HWND) -> i32;
}

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const CLASS_NAME: &str = "ZsClipMailMergeNative";
const ID_TIMER_PASTE: usize = 0x6A50;
const ID_TIMER_TARGET_TRACK: usize = 0x6A51;

const IDC_EXCEL: isize = 1001;
const IDC_LOAD: isize = 1002;
const IDC_SHEET: isize = 1003;
const IDC_HEADER_ROW: isize = 1004;
const IDC_DATA_ROW: isize = 1005;
const IDC_PREV_ROW: isize = 1006;
const IDC_NEXT_ROW: isize = 1007;
const IDC_MODE_MERGE: isize = 1008;
const IDC_MODE_FILL: isize = 1009;
const IDC_FIELDS: isize = 1010;
const IDC_OPEN_WORD: isize = 1011;
const IDC_INSERT_FIELD: isize = 1012;
const IDC_INSERT_INLINE: isize = 1013;
const IDC_INSERT_TABLE: isize = 1014;
const IDC_STATUS: isize = 1015;
const IDC_GUESS: isize = 1016;
const IDC_COPY_FIELDS: isize = 1017;
const IDC_BROWSE: isize = 1018;
const IDC_APPLY_ROW: isize = 1019;
const IDC_ROW_LABEL: isize = 1020;
const IDC_MM_TITLE: isize = 9001;
const IDC_MM_DESC: isize = 9002;
const CB_GETCURSEL: u32 = 0x0147;
const CB_ADDSTRING: u32 = 0x0143;
const CB_RESETCONTENT: u32 = 0x014B;
const CB_SETCURSEL: u32 = 0x014E;
const CB_GETLBTEXT: u32 = 0x0148;
const CB_GETLBTEXTLEN: u32 = 0x0149;
const BM_GETCHECK: u32 = 0x00F0;
const BM_SETCHECK: u32 = 0x00F1;
const BST_CHECKED: usize = 1;
const BST_UNCHECKED: usize = 0;
const VK_F9_KEY: u8 = 0x78;

#[derive(Copy, Clone, Eq, PartialEq, Default)]
enum PendingPasteKind {
    #[default]
    PlainText,
    MergeFieldCode,
}

#[derive(Default)]
struct MailMergeState {
    excel_path: String,
    active_sheet: String,
    header_row: i32,
    headers: Vec<String>,
    values: Vec<String>,
    row_count: usize,
    data_row: i32,
    cache_key: String,
    last_action_tick: u64,
    last_target_hwnd: HWND,
    last_target_title: String,
    pending_paste_kind: PendingPasteKind,
    font: *mut core::ffi::c_void,
    card_brush: *mut core::ffi::c_void,
    edit_brush: *mut core::ffi::c_void,
}

struct CreateArgs {
    initial_excel: String,
}

struct InspectResult {
    sheet_names: Vec<String>,
    active_sheet: String,
    headers: Vec<String>,
    values: Vec<String>,
    row_count: usize,
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn excel_cache_key(excel: &str, sheet: &str, header_row: i32, data_row: i32) -> String {
    format!("{}|{}|{}|{}", normalize_path_like(excel), sheet.trim(), header_row.max(1), data_row.max(1))
}

fn normalize_path_like(raw: &str) -> String {
    let mut s = raw.trim().to_string();
    if s.len() >= 2 && ((s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\''))) {
        s = s[1..s.len() - 1].trim().to_string();
    }
    if s.starts_with('[') && s.ends_with(']') {
        s = s[1..s.len() - 1].trim().trim_matches('"').trim_matches('\'').trim().to_string();
    }
    if let Ok(full) = std::fs::canonicalize(&s) {
        full.to_string_lossy().to_string()
    } else {
        s
    }
}

fn is_excel_path(path: &str) -> bool {
    let lower = normalize_path_like(path).to_ascii_lowercase();
    lower.ends_with(".xls") || lower.ends_with(".xlsx") || lower.ends_with(".xlsm") || lower.ends_with(".csv")
}

fn ps_run(script: &str, args: &[String]) -> Result<String, String> {
    let temp_path = std::env::temp_dir().join(format!(
        "zsclip_mailmerge_{}.ps1",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    ));
    fs::write(
        &temp_path,
        format!("[Console]::OutputEncoding=[System.Text.Encoding]::UTF8\r\n{}", script),
    )
    .map_err(|e| format!("写入 PowerShell 脚本失败: {e}"))?;

    let out = Command::new("powershell")
        .creation_flags(CREATE_NO_WINDOW)
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(&temp_path)
        .args(args)
        .output();
    let _ = fs::remove_file(&temp_path);
    let out = out.map_err(|e| format!("启动 PowerShell 失败: {e}"))?;
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

fn parse_proto_list(raw: &str) -> Vec<String> {
    raw.split('\u{1f}')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn ps_pick_excel_file() -> Result<Option<String>, String> {
    let script = r#"Add-Type -AssemblyName System.Windows.Forms
$dlg = New-Object System.Windows.Forms.OpenFileDialog
$dlg.Filter = 'Excel Files|*.xlsx;*.xls;*.xlsm;*.csv|All Files|*.*'
$dlg.Title = '选择 Excel 文件'
$dlg.Multiselect = $false
if ($dlg.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { Write-Output $dlg.FileName }"#;
    let out = ps_run(script, &[])?;
    if out.trim().is_empty() { Ok(None) } else { Ok(Some(out)) }
}

fn ps_guess_header_row(excel: &str, sheet: &str) -> Result<i32, String> {
    let script = r#"$excelPath=$args[0]
$sheetName=$args[1]
$excel = $null
try { $excel = [Runtime.InteropServices.Marshal]::GetActiveObject('Excel.Application') } catch {}
if ($null -eq $excel) { $excel = New-Object -ComObject Excel.Application }
$excel.Visible = $false
$excel.DisplayAlerts = $false
$wb = $null
foreach($candidate in $excel.Workbooks){
  if ([string]::Equals([string]$candidate.FullName, [string]$excelPath, [System.StringComparison]::OrdinalIgnoreCase)) {
    $wb = $candidate
    break
  }
}
if ($null -eq $wb) { $wb = $excel.Workbooks.Open($excelPath, 0, $true, 5) }
try {
  if ([string]::IsNullOrWhiteSpace($sheetName)) { $sheetName = [string]$wb.Worksheets(1).Name }
  $ws = $wb.Worksheets.Item($sheetName)
  $bestRow = 1; $bestScore = -999999
  for($r=1; $r -le 20; $r++){
    $lastCol = [int]$ws.Cells($r, $ws.Columns.Count).End(-4159).Column
    if ($lastCol -le 1) { continue }
    $score = 0
    for($c=1; $c -le [Math]::Min($lastCol, 40); $c++){
      $v = [string]($ws.Cells($r, $c).Text)
      if (![string]::IsNullOrWhiteSpace($v)) {
        $score += 6
        if ($v -match '^[0-9\./:\-\s]+$') { $score -= 3 } else { $score += 4 }
      }
    }
    if ($score -gt $bestScore) { $bestScore = $score; $bestRow = $r }
  }
  Write-Output $bestRow
} finally {}"#;
    let out = ps_run(script, &[normalize_path_like(excel), sheet.to_string()])?;
    out.parse::<i32>().map_err(|e| format!("解析表头行失败: {e}"))
}

fn ps_inspect_excel(excel: &str, sheet: &str, header_row: i32, data_row: i32) -> Result<InspectResult, String> {
    let excel_path = normalize_path_like(excel);
    if !std::path::Path::new(&excel_path).exists() {
        return Err(format!("解析 Excel 数据失败: 文件不存在 {}", excel_path));
    }
    let script = r#"$excelPath=$args[0]
$sheetName=$args[1]
$headerRow=[int]$args[2]
$dataRow=[int]$args[3]
$fs=[char]31
function Clean([object]$v) {
  $s=[string]$v
  $s=$s.Replace($fs, ' ')
  $s=$s.Replace("`r", ' ')
  $s=$s.Replace("`n", ' ')
  return $s.Trim()
}
$excel = $null
try { $excel = [Runtime.InteropServices.Marshal]::GetActiveObject('Excel.Application') } catch {}
if ($null -eq $excel) { $excel = New-Object -ComObject Excel.Application }
$excel.Visible = $false
$excel.DisplayAlerts = $false
$wb = $null
foreach($candidate in $excel.Workbooks){
  if ([string]::Equals([string]$candidate.FullName, [string]$excelPath, [System.StringComparison]::OrdinalIgnoreCase)) {
    $wb = $candidate
    break
  }
}
if ($null -eq $wb) { $wb = $excel.Workbooks.Open($excelPath, 0, $true, 5) }
try {
  $sheets = @(); foreach($ws in $wb.Worksheets){ $sheets += [string]$ws.Name }
  if ([string]::IsNullOrWhiteSpace($sheetName)) { $sheetName = [string]$wb.Worksheets(1).Name }
  $ws = $wb.Worksheets.Item($sheetName)
  $lastCol = [int]$ws.Cells($headerRow, $ws.Columns.Count).End(-4159).Column
  try { $lastRow = [int]$ws.Cells($ws.Rows.Count, 1).End(-4162).Row } catch { $lastRow = [int]$ws.UsedRange.Rows.Count }
  $headers = @(); $values = @()
  for($c=1; $c -le $lastCol; $c++){
    $headers += (Clean ($ws.Cells($headerRow, $c).Text))
    $values += (Clean ($ws.Cells($headerRow + $dataRow, $c).Text))
  }
  Write-Output ('sheet_names=' + (($sheets | ForEach-Object { Clean $_ }) -join $fs))
  Write-Output ('active_sheet=' + (Clean $sheetName))
  Write-Output ('headers=' + ($headers -join $fs))
  Write-Output ('values=' + ($values -join $fs))
  Write-Output ('row_count=' + [Math]::Max(0, $lastRow - $headerRow))
} finally {}"#;
    let out = ps_run(script, &[excel_path.clone(), sheet.to_string(), header_row.max(1).to_string(), data_row.max(1).to_string()])?;
    if out.trim().is_empty() {
        return Err(format!("解析 Excel 数据失败: PowerShell 没有返回任何内容 [{}]", excel_path));
    }
    let mut result = InspectResult {
        sheet_names: Vec::new(),
        active_sheet: String::new(),
        headers: Vec::new(),
        values: Vec::new(),
        row_count: 0,
    };
    for line in out.lines() {
        let Some((key, value)) = line.split_once('=') else { continue; };
        match key.trim() {
            "sheet_names" => result.sheet_names = parse_proto_list(value),
            "active_sheet" => result.active_sheet = value.trim().to_string(),
            "headers" => result.headers = parse_proto_list(value),
            "values" => result.values = parse_proto_list(value),
            "row_count" => result.row_count = value.trim().parse::<usize>().ok().unwrap_or(0),
            _ => {}
        }
    }
    if result.active_sheet.is_empty() && !result.sheet_names.is_empty() {
        result.active_sheet = result.sheet_names[0].clone();
    }
    if result.headers.is_empty() && result.values.is_empty() && result.sheet_names.is_empty() {
        return Err(format!("解析 Excel 数据失败: {}", out.trim()));
    }
    Ok(result)
}

fn ps_fetch_excel_row_values(excel: &str, sheet: &str, header_row: i32, data_row: i32) -> Result<(Vec<String>, usize), String> {
    let excel_path = normalize_path_like(excel);
    if !std::path::Path::new(&excel_path).exists() {
        return Err(format!("解析 Excel 数据失败: 文件不存在 {}", excel_path));
    }
    let script = r#"$excelPath=$args[0]
$sheetName=$args[1]
$headerRow=[int]$args[2]
$dataRow=[int]$args[3]
$fs=[char]31
function Clean([object]$v) {
  $s=[string]$v
  $s=$s.Replace($fs, ' ')
  $s=$s.Replace("`r", ' ')
  $s=$s.Replace("`n", ' ')
  return $s.Trim()
}
$excel = $null
try { $excel = [Runtime.InteropServices.Marshal]::GetActiveObject('Excel.Application') } catch {}
if ($null -eq $excel) { $excel = New-Object -ComObject Excel.Application }
$excel.Visible = $false
$excel.DisplayAlerts = $false
$wb = $null
foreach($candidate in $excel.Workbooks){
  if ([string]::Equals([string]$candidate.FullName, [string]$excelPath, [System.StringComparison]::OrdinalIgnoreCase)) {
    $wb = $candidate
    break
  }
}
if ($null -eq $wb) { $wb = $excel.Workbooks.Open($excelPath, 0, $true, 5) }
try {
  if ([string]::IsNullOrWhiteSpace($sheetName)) { $sheetName = [string]$wb.Worksheets(1).Name }
  $ws = $wb.Worksheets.Item($sheetName)
  $lastCol = [int]$ws.Cells($headerRow, $ws.Columns.Count).End(-4159).Column
  try { $lastRow = [int]$ws.Cells($ws.Rows.Count, 1).End(-4162).Row } catch { $lastRow = [int]$ws.UsedRange.Rows.Count }
  $values = @()
  for($c=1; $c -le $lastCol; $c++){
    $values += (Clean ($ws.Cells($headerRow + $dataRow, $c).Text))
  }
  Write-Output ('values=' + ($values -join $fs))
  Write-Output ('row_count=' + [Math]::Max(0, $lastRow - $headerRow))
} finally {}"#;
    let out = ps_run(
        script,
        &[excel_path, sheet.to_string(), header_row.max(1).to_string(), data_row.max(1).to_string()],
    )?;
    if out.trim().is_empty() {
        return Err("读取当前行失败: PowerShell 没有返回任何内容".to_string());
    }
    let mut values = Vec::new();
    let mut row_count = 0usize;
    for line in out.lines() {
        let Some((key, value)) = line.split_once('=') else { continue; };
        match key.trim() {
            "values" => values = parse_proto_list(value),
            "row_count" => row_count = value.trim().parse::<usize>().ok().unwrap_or(0),
            _ => {}
        }
    }
    Ok((values, row_count))
}

fn ps_word_open() -> Result<(), String> {
    ps_run(r#"$word=$null; try { $word=[Runtime.InteropServices.Marshal]::GetActiveObject('Word.Application') } catch {}
if ($null -eq $word) { $word = New-Object -ComObject Word.Application }
$word.Visible=$true
if ($word.Documents.Count -le 0) { $word.Documents.Add() | Out-Null }
$word.Activate()"#, &[]).map(|_| ())
}

fn ps_word_insert_inline(fields: &[String]) -> Result<(), String> {
    let json = serde_json::to_string(fields).map_err(|e| e.to_string())?;
    ps_run(r#"$fields=ConvertFrom-Json $args[0]
$word=$null; try { $word=[Runtime.InteropServices.Marshal]::GetActiveObject('Word.Application') } catch {}
if ($null -eq $word) { $word=New-Object -ComObject Word.Application }
$word.Visible=$true
$doc = if ($word.Documents.Count -gt 0) { $word.ActiveDocument } else { $word.Documents.Add() }
$doc.MailMerge.MainDocumentType = 1
$first=$true
foreach($f in $fields){
  if(-not $first){ $word.Selection.TypeText(' ') }
  $doc.MailMerge.Fields.Add($word.Selection.Range, [string]$f) | Out-Null
  $first=$false
}"#, &[json]).map(|_| ())
}

fn ps_word_insert_table(headers: &[String], values: Option<&[String]>) -> Result<(), String> {
    let headers_json = serde_json::to_string(headers).map_err(|e| e.to_string())?;
    let values_json = serde_json::to_string(values.unwrap_or(&[])).map_err(|e| e.to_string())?;
    let fill_mode = if values.is_some() { "1" } else { "0" };
    ps_run(r#"$headers=ConvertFrom-Json $args[0]; $values=ConvertFrom-Json $args[1]; $fillMode=[int]$args[2]
$word=$null; try { $word=[Runtime.InteropServices.Marshal]::GetActiveObject('Word.Application') } catch {}
if ($null -eq $word) { $word=New-Object -ComObject Word.Application }
$word.Visible=$true
$doc = if ($word.Documents.Count -gt 0) { $word.ActiveDocument } else { $word.Documents.Add() }
if ($fillMode -eq 0) { $doc.MailMerge.MainDocumentType = 1 }
$cols=[Math]::Max(1,$headers.Count)
$tbl=$doc.Tables.Add($word.Selection.Range,2,$cols)
for($i=1; $i -le $cols; $i++){
  $tbl.Cell(1,$i).Range.Text=[string]$headers[$i-1]
  if($fillMode -eq 1){ $tbl.Cell(2,$i).Range.Text = if($i-1 -lt $values.Count){ [string]$values[$i-1] } else { '' } }
  else { $doc.MailMerge.Fields.Add($tbl.Cell(2,$i).Range, [string]$headers[$i-1]) | Out-Null }
}"#, &[headers_json, values_json, fill_mode.to_string()]).map(|_| ())
}

unsafe fn set_font(hwnd: HWND, font: *mut core::ffi::c_void) { if !hwnd.is_null() { SendMessageW(hwnd, WM_SETFONT, font as usize, 1); } }
unsafe fn set_status(hwnd: HWND, text: &str) { SetWindowTextW(GetDlgItem(hwnd, IDC_STATUS as i32), to_wide(translate(text).as_ref()).as_ptr()); }
unsafe fn set_text(hwnd: HWND, id: isize, text: &str) { SetWindowTextW(GetDlgItem(hwnd, id as i32), to_wide(translate(text).as_ref()).as_ptr()); }
unsafe fn is_fill_mode(hwnd: HWND) -> bool { SendDlgItemMessageW(hwnd, IDC_MODE_FILL as i32, BM_GETCHECK, 0, 0) == BST_CHECKED as isize }
unsafe fn current_data_row(hwnd: HWND) -> i32 { get_window_text(GetDlgItem(hwnd, IDC_DATA_ROW as i32)).parse::<i32>().ok().unwrap_or(1).max(1) }

unsafe fn create_ctrl(class: &str, text: &str, style: u32, ex_style: u32, x: i32, y: i32, w: i32, h: i32, parent: HWND, id: isize, font: *mut core::ffi::c_void) -> HWND {
    let caption = if matches!(class, "STATIC" | "BUTTON") { translate(text).into_owned() } else { text.to_string() };
    let hh = CreateWindowExW(ex_style, to_wide(class).as_ptr(), to_wide(&caption).as_ptr(), WS_CHILD | WS_VISIBLE | style, x, y, w, h, parent, id as usize as _, GetModuleHandleW(null()), null());
    set_font(hh, font);
    hh
}

unsafe fn create_action_btn(parent: HWND, text: &str, id: isize, x: i32, y: i32, w: i32, font: *mut core::ffi::c_void) -> HWND {
    create_settings_component(parent, text, id, SettingsComponentKind::Button, x, y, w, 30, font)
}

unsafe fn create_accent_btn(parent: HWND, text: &str, id: isize, x: i32, y: i32, w: i32, font: *mut core::ffi::c_void) -> HWND {
    create_settings_component(parent, text, id, SettingsComponentKind::AccentButton, x, y, w, 30, font)
}

unsafe fn current_combo_text(hwnd: HWND) -> String {
    let combo = GetDlgItem(hwnd, IDC_SHEET as i32);
    let idx = SendMessageW(combo, CB_GETCURSEL, 0, 0) as i32;
    if idx < 0 { return String::new(); }
    let len = SendMessageW(combo, CB_GETLBTEXTLEN, idx as usize, 0) as usize;
    let mut buf = vec![0u16; len + 1];
    SendMessageW(combo, CB_GETLBTEXT, idx as usize, buf.as_mut_ptr() as LPARAM);
    String::from_utf16_lossy(&buf).trim_end_matches('\0').to_string()
}

unsafe fn combo_fill(hwnd: HWND, items: &[String], active: &str) {
    let combo = GetDlgItem(hwnd, IDC_SHEET as i32);
    SendMessageW(combo, CB_RESETCONTENT, 0, 0);
    let mut sel = 0usize;
    for (idx, item) in items.iter().enumerate() {
        SendMessageW(combo, CB_ADDSTRING, 0, to_wide(item).as_ptr() as LPARAM);
        if item == active { sel = idx; }
    }
    if !items.is_empty() { SendMessageW(combo, CB_SETCURSEL, sel, 0); }
}

fn button_kind(id: isize) -> SettingsComponentKind {
    match id {
        IDC_LOAD | IDC_INSERT_FIELD | IDC_INSERT_INLINE | IDC_INSERT_TABLE => SettingsComponentKind::AccentButton,
        _ => SettingsComponentKind::Button,
    }
}

unsafe fn good_target(hwnd: HWND, self_hwnd: HWND) -> bool {
    !hwnd.is_null() && hwnd != self_hwnd && IsWindow(hwnd) != 0 && IsWindowVisible(hwnd) != 0 && IsWindowEnabled(hwnd) != 0 && !get_window_text(hwnd).trim().is_empty()
}

unsafe fn send_ctrl_f9() { keybd_event(VK_CONTROL as u8, 0, 0, 0); keybd_event(VK_F9_KEY, 0, 0, 0); keybd_event(VK_F9_KEY, 0, KEYEVENTF_KEYUP, 0); keybd_event(VK_CONTROL as u8, 0, KEYEVENTF_KEYUP, 0); }
unsafe fn send_f9() { keybd_event(VK_F9_KEY, 0, 0, 0); keybd_event(VK_F9_KEY, 0, KEYEVENTF_KEYUP, 0); }
unsafe fn send_shift_f9() { keybd_event(VK_SHIFT as u8, 0, 0, 0); keybd_event(VK_F9_KEY, 0, 0, 0); keybd_event(VK_F9_KEY, 0, KEYEVENTF_KEYUP, 0); keybd_event(VK_SHIFT as u8, 0, KEYEVENTF_KEYUP, 0); }

unsafe fn refresh_mode_ui(hwnd: HWND) {
    let fill = is_fill_mode(hwnd);
    set_text(hwnd, IDC_INSERT_FIELD, if fill { "粘贴字段" } else { "插入字段" });
    set_text(hwnd, IDC_INSERT_INLINE, if fill { "粘贴一行" } else { "插入一行" });
    set_text(hwnd, IDC_INSERT_TABLE, if fill { "填表格" } else { "插入表格" });
    for id in [IDC_DATA_ROW, IDC_PREV_ROW, IDC_NEXT_ROW, IDC_APPLY_ROW, IDC_ROW_LABEL] {
        let hh = GetDlgItem(hwnd, id as i32);
        if !hh.is_null() { ShowWindow(hh, if fill { SW_SHOW } else { SW_HIDE }); EnableWindow(hh, if fill { 1 } else { 0 }); }
    }
    let hh = GetDlgItem(hwnd, IDC_OPEN_WORD as i32);
    if !hh.is_null() { ShowWindow(hh, if fill { SW_HIDE } else { SW_SHOW }); EnableWindow(hh, if fill { 0 } else { 1 }); }
}

fn word_merge_field_clipboard_text(field: &str) -> String {
    let field = field.trim();
    if field.is_empty() {
        return String::new();
    }
    if field.contains(' ') || field.contains('"') {
        format!(" MERGEFIELD  \"{}\" ", field.replace('"', "\"\""))
    } else {
        format!(" MERGEFIELD  {} ", field)
    }
}

unsafe fn start_clipboard_paste_like_main(
    hwnd: HWND,
    st: &mut MailMergeState,
    text: &str,
    pending_paste_kind: PendingPasteKind,
) -> Result<(), String> {
    if text.trim().is_empty() { return Err("没有可粘贴内容".to_string()); }
    let mut cb = Clipboard::new().map_err(|e| format!("打开剪贴板失败: {e}"))?;
    cb.set_text(text.to_string()).map_err(|e| format!("写入剪贴板失败: {e}"))?;
    let target = if good_target(st.last_target_hwnd, hwnd) { st.last_target_hwnd } else { GetForegroundWindow() };
    if !good_target(target, hwnd) { return Err("未找到目标窗口，请先点击要录入的外部输入框".to_string()); }
    st.pending_paste_kind = pending_paste_kind;
    let _ = force_foreground_window(target);
    KillTimer(hwnd, ID_TIMER_PASTE);
    SetTimer(hwnd, ID_TIMER_PASTE, 150, None);
    Ok(())
}

unsafe fn paste_text_like_main(hwnd: HWND, st: &mut MailMergeState, text: &str) -> Result<(), String> {
    start_clipboard_paste_like_main(hwnd, st, text, PendingPasteKind::PlainText)
}

unsafe fn paste_merge_field_like_main(hwnd: HWND, st: &mut MailMergeState, field: &str) -> Result<(), String> {
    let code = word_merge_field_clipboard_text(field);
    if code.is_empty() {
        return Err("请先选择字段".to_string());
    }
    start_clipboard_paste_like_main(hwnd, st, &code, PendingPasteKind::MergeFieldCode)
}

unsafe fn reload_excel(hwnd: HWND, st: &mut MailMergeState) {
    let excel = normalize_path_like(&get_window_text(GetDlgItem(hwnd, IDC_EXCEL as i32)));
    if excel.trim().is_empty() { st.headers.clear(); st.values.clear(); st.row_count = 0; SendDlgItemMessageW(hwnd, IDC_FIELDS as i32, LB_RESETCONTENT, 0, 0); set_status(hwnd, "请先选择 Excel 文件"); return; }
    if !is_excel_path(&excel) { set_status(hwnd, "请选择 xls/xlsx/xlsm/csv 文件"); return; }
    let header_row = get_window_text(GetDlgItem(hwnd, IDC_HEADER_ROW as i32)).parse::<i32>().ok().unwrap_or(1).max(1);
    let data_row = current_data_row(hwnd);
    let sheet = current_combo_text(hwnd);
    if st.excel_path == excel && st.active_sheet == sheet && st.header_row == header_row && !st.headers.is_empty() && st.data_row != data_row {
        match ps_fetch_excel_row_values(&excel, &sheet, header_row, data_row) {
            Ok((values, row_count)) => {
                st.values = values;
                st.row_count = row_count;
                st.data_row = data_row;
                st.cache_key = excel_cache_key(&excel, &sheet, header_row, data_row);
                set_status(hwnd, &format!("已切换到第 {} 行，共 {} 行数据", st.data_row, st.row_count));
            }
            Err(e) => set_status(hwnd, &e),
        }
        return;
    }
    let cache_key = excel_cache_key(&excel, &sheet, header_row, data_row);
    if st.cache_key == cache_key && !st.headers.is_empty() {
        set_status(hwnd, &format!("已使用缓存：{} 个字段，共 {} 行数据，当前第 {} 行", st.headers.len(), st.row_count, data_row));
        return;
    }
    match ps_inspect_excel(&excel, &sheet, header_row, data_row) {
        Ok(data) => {
            st.excel_path = excel;
            st.active_sheet = data.active_sheet.clone();
            st.header_row = header_row;
            st.headers = data.headers.into_iter().filter(|s| !s.trim().is_empty()).collect();
            st.values = data.values;
            st.row_count = data.row_count;
            st.data_row = data_row;
            st.cache_key = cache_key;
            combo_fill(hwnd, &data.sheet_names, &data.active_sheet);
            SendDlgItemMessageW(hwnd, IDC_FIELDS as i32, LB_RESETCONTENT, 0, 0);
            for name in &st.headers { SendDlgItemMessageW(hwnd, IDC_FIELDS as i32, LB_ADDSTRING, 0, to_wide(name).as_ptr() as LPARAM); }
            set_status(hwnd, &format!("已加载 {} 个字段，共 {} 行数据，当前第 {} 行", st.headers.len(), st.row_count, st.data_row));
        }
        Err(e) => set_status(hwnd, &e),
    }
}

unsafe fn selected_field(hwnd: HWND) -> Option<String> {
    let idx = SendDlgItemMessageW(hwnd, IDC_FIELDS as i32, LB_GETCURSEL, 0, 0) as i32;
    if idx < 0 { return None; }
    let len = SendDlgItemMessageW(hwnd, IDC_FIELDS as i32, LB_GETTEXTLEN, idx as usize, 0) as usize;
    let mut buf = vec![0u16; len + 1];
    SendDlgItemMessageW(hwnd, IDC_FIELDS as i32, LB_GETTEXT, idx as usize, buf.as_mut_ptr() as LPARAM);
    Some(String::from_utf16_lossy(&buf).trim_end_matches('\0').to_string())
}

unsafe fn run_primary_action(hwnd: HWND, st: &mut MailMergeState) {
    run_primary_action_inner(hwnd, st, true);
}

unsafe fn run_primary_action_inner(hwnd: HWND, st: &mut MailMergeState, throttle: bool) {
    let now = now_millis();
    if throttle && now.saturating_sub(st.last_action_tick) < 220 {
        return;
    }
    st.last_action_tick = now;
    if let Some(field) = selected_field(hwnd) {
        let idx = st.headers.iter().position(|h| h == &field).unwrap_or(0);
        let value = st.values.get(idx).cloned().unwrap_or_default();
        let res = if is_fill_mode(hwnd) { paste_text_like_main(hwnd, st, &value) } else { paste_merge_field_like_main(hwnd, st, &field) };
        match res { Ok(_) => set_status(hwnd, if is_fill_mode(hwnd) { "已粘贴当前字段" } else { "已插入字段" }), Err(e) => set_status(hwnd, &e) }
    } else {
        set_status(hwnd, "请先选择字段");
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            let cs = &*(lparam as *const CREATESTRUCTW);
            let args = if !cs.lpCreateParams.is_null() { Box::from_raw(cs.lpCreateParams as *mut CreateArgs) } else { Box::new(CreateArgs { initial_excel: String::new() }) };
            let font = CreateFontW(-16, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, to_wide(crate::ui::ui_text_font_family()).as_ptr());
            let font = if font.is_null() { GetStockObject(DEFAULT_GUI_FONT) as _ } else { font };
            let th = Theme::default();
            let st = Box::new(MailMergeState {
                last_target_hwnd: GetForegroundWindow(),
                last_target_title: String::new(),
                font,
                card_brush: CreateSolidBrush(th.surface) as _,
                edit_brush: CreateSolidBrush(th.control_bg) as _,
                ..Default::default()
            });
            create_ctrl("STATIC", "超级邮件合并", 0, 0, 24, 18, 260, 24, hwnd, IDC_MM_TITLE, font);
            create_ctrl("STATIC", "Excel -> Word（邮件合并 / 数据填表）", 0, 0, 24, 48, 360, 20, hwnd, IDC_MM_DESC, font);
            create_ctrl("EDIT", &args.initial_excel, ES_AUTOHSCROLL as u32 | WS_TABSTOP, WS_EX_CLIENTEDGE, 96, 98, 398, 32, hwnd, IDC_EXCEL, font);
            create_action_btn(hwnd, "浏览...", IDC_BROWSE, 504, 99, 76, font);
            create_accent_btn(hwnd, "加载字段", IDC_LOAD, 590, 99, 80, font);
            create_ctrl("STATIC", "工作表", 0, 0, 16, 144, 54, 22, hwnd, 9003, font);
            create_ctrl("COMBOBOX", "", CBS_DROPDOWNLIST as u32 | WS_VSCROLL | WS_TABSTOP, 0, 74, 142, 200, 240, hwnd, IDC_SHEET, font);
            create_ctrl("STATIC", "表头行", 0, 0, 286, 144, 54, 22, hwnd, 9004, font);
            create_ctrl("EDIT", "1", ES_AUTOHSCROLL as u32 | WS_TABSTOP, WS_EX_CLIENTEDGE, 344, 142, 56, 30, hwnd, IDC_HEADER_ROW, font);
            create_action_btn(hwnd, "自动识别", IDC_GUESS, 408, 142, 84, font);
            create_ctrl("BUTTON", "邮件合并", BS_AUTORADIOBUTTON as u32 | WS_TABSTOP, 0, 16, 186, 96, 28, hwnd, IDC_MODE_MERGE, font);
            create_ctrl("BUTTON", "数据填表", BS_AUTORADIOBUTTON as u32 | WS_TABSTOP, 0, 118, 186, 96, 28, hwnd, IDC_MODE_FILL, font);
            SendDlgItemMessageW(hwnd, IDC_MODE_MERGE as i32, BM_SETCHECK, BST_CHECKED, 0);
            SendDlgItemMessageW(hwnd, IDC_MODE_FILL as i32, BM_SETCHECK, BST_UNCHECKED, 0);
            create_ctrl("STATIC", "行", 0, 0, 228, 186, 18, 20, hwnd, IDC_ROW_LABEL, font);
            create_ctrl("EDIT", "1", ES_AUTOHSCROLL as u32 | WS_TABSTOP, WS_EX_CLIENTEDGE, 250, 182, 44, 30, hwnd, IDC_DATA_ROW, font);
            create_action_btn(hwnd, "上一行", IDC_PREV_ROW, 302, 182, 68, font);
            create_action_btn(hwnd, "下一行", IDC_NEXT_ROW, 378, 182, 68, font);
            create_action_btn(hwnd, "切换", IDC_APPLY_ROW, 454, 182, 58, font);
            create_ctrl("LISTBOX", "", LBS_NOTIFY as u32 | WS_VSCROLL | WS_TABSTOP, WS_EX_CLIENTEDGE, 16, 228, 654, 206, hwnd, IDC_FIELDS, font);
            create_action_btn(hwnd, "打开 Word", IDC_OPEN_WORD, 16, 448, 108, font);
            create_accent_btn(hwnd, "插入字段", IDC_INSERT_FIELD, 134, 448, 108, font);
            create_accent_btn(hwnd, "插入一行", IDC_INSERT_INLINE, 252, 448, 108, font);
            create_accent_btn(hwnd, "插入表格", IDC_INSERT_TABLE, 370, 448, 108, font);
            create_action_btn(hwnd, "复制字段", IDC_COPY_FIELDS, 488, 448, 90, font);
            create_ctrl("STATIC", "请选择 Excel 文件（xlsx/xlsm/xls/csv）", 0, 0, 16, 492, 654, 24, hwnd, IDC_STATUS, font);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(st) as isize);
            let icons = load_icons();
            if icons.app != 0 {
                SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, icons.app as LPARAM);
                SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, icons.app as LPARAM);
                SetClassLongPtrW(hwnd, GCLP_HICON, icons.app);
                SetClassLongPtrW(hwnd, GCLP_HICONSM, icons.app);
            }
            apply_window_corner_preference(hwnd);
            apply_dark_mode_to_window(hwnd);
            refresh_mode_ui(hwnd);
            SetTimer(hwnd, ID_TIMER_TARGET_TRACK, 200, None);
            if !args.initial_excel.trim().is_empty() { PostMessageW(hwnd, WM_COMMAND, IDC_LOAD as usize, 0); }
            0
        }
        WM_ACTIVATE => {
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut MailMergeState;
            if !st_ptr.is_null() {
                let st = &mut *st_ptr;
                let state = (wparam & 0xffff) as u32;
                if state == WA_ACTIVE || state == WA_CLICKACTIVE {
                    let prev = lparam as HWND;
                    if good_target(prev, hwnd) { st.last_target_hwnd = prev; st.last_target_title = get_window_text(prev); }
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_TIMER => {
            if wparam == ID_TIMER_PASTE {
                KillTimer(hwnd, ID_TIMER_PASTE);
                let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut MailMergeState;
                if !st_ptr.is_null() {
                    let st = &mut *st_ptr;
                    match st.pending_paste_kind {
                        PendingPasteKind::PlainText => send_ctrl_v(),
                        PendingPasteKind::MergeFieldCode => {
                            send_ctrl_f9();
                            send_ctrl_v();
                            send_f9();
                            send_shift_f9();
                        }
                    }
                    st.pending_paste_kind = PendingPasteKind::PlainText;
                }
                return 0;
            }
            if wparam == ID_TIMER_TARGET_TRACK {
                let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut MailMergeState;
                if !st_ptr.is_null() {
                    let st = &mut *st_ptr;
                    let fg = GetForegroundWindow();
                    if good_target(fg, hwnd) { st.last_target_hwnd = fg; st.last_target_title = get_window_text(fg); }
                }
                return 0;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_DRAWITEM => {
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut MailMergeState;
            if st_ptr.is_null() {
                return 0;
            }
            let st = &mut *st_ptr;
            let dis = &*(lparam as *const DRAWITEMSTRUCT);
            let text = get_window_text(dis.hwndItem);
            let hover = {
                let mut pt: POINT = zeroed();
                GetCursorPos(&mut pt);
                let mut rc: RECT = zeroed();
                GetWindowRect(dis.hwndItem, &mut rc);
                pt.x >= rc.left && pt.x < rc.right && pt.y >= rc.top && pt.y < rc.bottom
            };
            draw_settings_button_component(
                dis.hDC as _,
                &dis.rcItem,
                &text,
                button_kind(dis.CtlID as isize),
                hover,
                (dis.itemState & ODS_SELECTED) != 0,
                Theme::default(),
            );
            let _ = st;
            1
        }
        WM_CTLCOLORSTATIC => {
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut MailMergeState;
            let hdc = wparam as *mut core::ffi::c_void;
            if !st_ptr.is_null() {
                let th = Theme::default();
                SetBkMode(hdc, 1);
                SetBkColor(hdc, th.surface);
                SetTextColor(hdc, th.text);
                return (*st_ptr).card_brush as isize;
            }
            0
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX => {
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut MailMergeState;
            let hdc = wparam as *mut core::ffi::c_void;
            if !st_ptr.is_null() {
                let th = Theme::default();
                SetBkColor(hdc, th.control_bg);
                SetTextColor(hdc, th.text);
                return (*st_ptr).edit_brush as isize;
            }
            0
        }
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            if !hdc.is_null() {
                let th = Theme::default();
                let mut rc: RECT = zeroed();
                GetClientRect(hwnd, &mut rc);
                let bg = CreateSolidBrush(th.bg);
                FillRect(hdc, &rc, bg);
                DeleteObject(bg as _);

                let cards = [
                    RECT { left: 12, top: 8, right: rc.right - 12, bottom: 176 },
                    RECT { left: 12, top: 176, right: rc.right - 12, bottom: 438 },
                    RECT { left: 12, top: 438, right: rc.right - 12, bottom: rc.bottom - 12 },
                ];
                for card in cards.iter() {
                    draw_round_rect(hdc as _, card, th.surface, th.stroke, 8);
                }
                let sec1 = RECT { left: 24, top: 74, right: 220, bottom: 96 };
                draw_text_ex(hdc as _, "Excel 数据", &sec1, th.text_muted, 12, true, false, "Segoe UI Variable Text");
                let sec2 = RECT { left: 24, top: 186, right: 220, bottom: 208 };
                draw_text_ex(hdc as _, "字段与模式", &sec2, th.text_muted, 12, true, false, "Segoe UI Variable Text");
                let sec3 = RECT { left: 24, top: 448, right: 220, bottom: 470 };
                draw_text_ex(hdc as _, "操作", &sec3, th.text_muted, 12, true, false, "Segoe UI Variable Text");
                EndPaint(hwnd, &ps);
            }
            0
        }
        WM_COMMAND => {
            let id = (wparam as u32 & 0xffff) as isize;
            let code = (wparam as u32 >> 16) & 0xffff;
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut MailMergeState;
            if st_ptr.is_null() { return 0; }
            let st = &mut *st_ptr;
            match id {
                IDC_BROWSE => match ps_pick_excel_file() {
                    Ok(Some(path)) => { set_text(hwnd, IDC_EXCEL, &path); reload_excel(hwnd, st); }
                    Ok(None) => {}
                    Err(e) => set_status(hwnd, &e),
                },
                IDC_LOAD => reload_excel(hwnd, st),
                IDC_SHEET if code == CBN_SELCHANGE => reload_excel(hwnd, st),
                IDC_GUESS => {
                    let excel = normalize_path_like(&get_window_text(GetDlgItem(hwnd, IDC_EXCEL as i32)));
                    if excel.trim().is_empty() { set_status(hwnd, "请先选择 Excel 文件"); }
                    else {
                        match ps_guess_header_row(&excel, &current_combo_text(hwnd)) {
                            Ok(row) => { set_text(hwnd, IDC_HEADER_ROW, &row.to_string()); reload_excel(hwnd, st); }
                            Err(e) => set_status(hwnd, &e),
                        }
                    }
                }
                IDC_PREV_ROW => { set_text(hwnd, IDC_DATA_ROW, &(current_data_row(hwnd) - 1).max(1).to_string()); reload_excel(hwnd, st); }
                IDC_NEXT_ROW => { let next = if st.row_count > 0 { (current_data_row(hwnd) + 1).min(st.row_count as i32) } else { current_data_row(hwnd) + 1 }; set_text(hwnd, IDC_DATA_ROW, &next.to_string()); reload_excel(hwnd, st); }
                IDC_APPLY_ROW => reload_excel(hwnd, st),
                IDC_MODE_MERGE | IDC_MODE_FILL => { refresh_mode_ui(hwnd); if is_fill_mode(hwnd) { reload_excel(hwnd, st); } }
                IDC_COPY_FIELDS => {
                    let mut cb = match Clipboard::new() { Ok(v) => v, Err(e) => { set_status(hwnd, &format!("打开剪贴板失败: {e}")); return 0; } };
                    match cb.set_text(st.headers.join("\r\n")) { Ok(_) => set_status(hwnd, "已复制字段名"), Err(e) => set_status(hwnd, &format!("复制字段失败: {e}")) }
                }
                IDC_OPEN_WORD => match ps_word_open() { Ok(_) => set_status(hwnd, "已打开 Word"), Err(e) => set_status(hwnd, &e) },
                IDC_INSERT_FIELD => run_primary_action(hwnd, st),
                IDC_INSERT_INLINE => {
                    let res = if is_fill_mode(hwnd) { paste_text_like_main(hwnd, st, &st.values.join(" ")) } else { ps_word_insert_inline(&st.headers) };
                    match res { Ok(_) => set_status(hwnd, if is_fill_mode(hwnd) { "已粘贴当前行" } else { "已插入一行字段" }), Err(e) => set_status(hwnd, &e) }
                }
                IDC_INSERT_TABLE => {
                    let res = if is_fill_mode(hwnd) { ps_word_insert_table(&st.headers, Some(&st.values)) } else { ps_word_insert_table(&st.headers, None) };
                    match res { Ok(_) => set_status(hwnd, if is_fill_mode(hwnd) { "已填入表格" } else { "已插入表格" }), Err(e) => set_status(hwnd, &e) }
                }
                IDC_FIELDS if code == LBN_DBLCLK => run_primary_action_inner(hwnd, st, false),
                _ => {}
            }
            0
        }
        WM_CLOSE => { DestroyWindow(hwnd); 0 }
        WM_NCDESTROY => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut MailMergeState;
            if !ptr.is_null() {
                let st = Box::from_raw(ptr);
                KillTimer(hwnd, ID_TIMER_PASTE);
                KillTimer(hwnd, ID_TIMER_TARGET_TRACK);
                if !st.font.is_null() && !core::ptr::eq(st.font, GetStockObject(DEFAULT_GUI_FONT)) {
                    DeleteObject(st.font as _);
                }
                if !st.card_brush.is_null() {
                    DeleteObject(st.card_brush as _);
                }
                if !st.edit_brush.is_null() {
                    DeleteObject(st.edit_brush as _);
                }
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn ensure_class() {
    static DONE: OnceLock<()> = OnceLock::new();
    if DONE.get().is_some() { return; }
    let class_name = to_wide(CLASS_NAME);
    let mut wc: WNDCLASSEXW = zeroed();
    wc.cbSize = size_of::<WNDCLASSEXW>() as u32;
    wc.lpfnWndProc = Some(wnd_proc);
    wc.hInstance = GetModuleHandleW(null());
    wc.hCursor = LoadCursorW(null_mut(), IDC_ARROW);
    wc.lpszClassName = class_name.as_ptr();
    RegisterClassExW(&wc);
    let _ = DONE.set(());
}

pub(crate) unsafe fn launch_mail_merge_window(owner: HWND) { launch_mail_merge_window_with_excel(owner, None); }

pub(crate) unsafe fn launch_mail_merge_window_with_excel(owner: HWND, initial_excel: Option<&str>) {
    ensure_class();
    let class_name = to_wide(CLASS_NAME);
    let title = to_wide("超级邮件合并");
    let mut rc: RECT = zeroed();
    if !owner.is_null() { GetWindowRect(owner, &mut rc); }
    let args = Box::new(CreateArgs { initial_excel: initial_excel.unwrap_or("").to_string() });
    let hwnd = CreateWindowExW(
        WS_EX_APPWINDOW,
        class_name.as_ptr(),
        title.as_ptr(),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_VISIBLE,
        if rc.right > rc.left { rc.left + 80 } else { CW_USEDEFAULT },
        if rc.bottom > rc.top { rc.top + 60 } else { CW_USEDEFAULT },
        704,
        612,
        owner,
        null_mut(),
        GetModuleHandleW(null()),
        Box::into_raw(args) as _,
    );
    if !hwnd.is_null() { ShowWindow(hwnd, SW_SHOW); SetForegroundWindow(hwnd); }
}

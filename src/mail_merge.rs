use arboard::Clipboard;
use std::mem::{size_of, zeroed};
use std::process::Command;
use std::ptr::{null, null_mut};
use std::sync::OnceLock;

use serde::Deserialize;
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::{CreateFontW, DEFAULT_GUI_FONT, GetStockObject},
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::*,
};

use crate::win_system_ui::{
    apply_dark_mode_to_window, apply_window_corner_preference, get_window_text, to_wide,
};

const CLASS_NAME: &str = "ZsClipMailMerge";
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

const CB_GETCURSEL: u32 = 0x0147;
const CB_ADDSTRING: u32 = 0x0143;
const CB_RESETCONTENT: u32 = 0x014B;
const CB_SETCURSEL: u32 = 0x014E;
const BM_GETCHECK: u32 = 0x00F0;
const BST_CHECKED: usize = 1;

#[derive(Default)]
struct MailMergeState {
    excel_path: String,
    headers: Vec<String>,
    values: Vec<String>,
    row_count: usize,
}

#[derive(Deserialize)]
struct InspectResult {
    #[serde(default)]
    sheet_names: Vec<String>,
    #[serde(default)]
    active_sheet: String,
    #[serde(default)]
    headers: Vec<String>,
    #[serde(default)]
    values: Vec<String>,
    #[serde(default)]
    row_count: usize,
}

fn ps_run(script: &str, args: &[String]) -> Result<String, String> {
    let output = Command::new("powershell")
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-Command")
        .arg(script)
        .args(args)
        .output()
        .map_err(|e| format!("启动 PowerShell 失败: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if stderr.is_empty() { "PowerShell 执行失败".to_string() } else { stderr })
    }
}

fn ps_inspect_excel(excel: &str, sheet: &str, header_row: i32, data_row: i32) -> Result<InspectResult, String> {
    let script = r#"$excelPath=$args[0]
$sheetName=$args[1]
$headerRow=[int]$args[2]
$dataRow=[int]$args[3]
$excel = New-Object -ComObject Excel.Application
$excel.Visible = $false
$excel.DisplayAlerts = $false
$wb = $excel.Workbooks.Open($excelPath, 0, $true)
try {
  $sheets = @()
  foreach($ws in $wb.Worksheets){ $sheets += [string]$ws.Name }
  if ([string]::IsNullOrWhiteSpace($sheetName)) { $sheetName = [string]$wb.Worksheets(1).Name }
  $ws = $wb.Worksheets.Item($sheetName)
  $lastCol = [int]$ws.Cells($headerRow, $ws.Columns.Count).End(-4159).Column
  $lastRow = [int]$ws.UsedRange.Rows.Count
  $headers = @()
  $values = @()
  for($c=1; $c -le $lastCol; $c++){
    $headers += [string]($ws.Cells($headerRow, $c).Text)
    $values += [string]($ws.Cells($headerRow + $dataRow, $c).Text)
  }
  $obj = @{
    sheet_names = $sheets
    active_sheet = [string]$sheetName
    headers = $headers
    values = $values
    row_count = [Math]::Max(0, $lastRow - $headerRow)
  }
  $obj | ConvertTo-Json -Compress
} finally {
  $wb.Close($false)
  $excel.Quit()
}"#;
    let out = ps_run(script, &[excel.to_string(), sheet.to_string(), header_row.to_string(), data_row.to_string()])?;
    serde_json::from_str(&out).map_err(|e| format!("解析 Excel 数据失败: {e}"))
}

fn ps_guess_header_row(excel: &str, sheet: &str) -> Result<i32, String> {
    let script = r#"$excelPath=$args[0]
$sheetName=$args[1]
$excel = New-Object -ComObject Excel.Application
$excel.Visible = $false
$excel.DisplayAlerts = $false
$wb = $excel.Workbooks.Open($excelPath, 0, $true)
try {
  if ([string]::IsNullOrWhiteSpace($sheetName)) { $sheetName = [string]$wb.Worksheets(1).Name }
  $ws = $wb.Worksheets.Item($sheetName)
  $bestRow = 1
  $bestScore = -999999
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
} finally {
  $wb.Close($false)
  $excel.Quit()
}"#;
    let out = ps_run(script, &[excel.to_string(), sheet.to_string()])?;
    out.trim().parse::<i32>().map_err(|e| format!("解析表头行失败: {e}"))
}

fn ps_word_open() -> Result<(), String> {
    let script = r#"$word = $null
try { $word = [Runtime.InteropServices.Marshal]::GetActiveObject('Word.Application') } catch {}
if ($null -eq $word) { $word = New-Object -ComObject Word.Application }
$word.Visible = $true
if ($word.Documents.Count -le 0) { $word.Documents.Add() | Out-Null }
$word.Activate()"#;
    ps_run(script, &[]).map(|_| ())
}

fn ps_word_insert_field(field: &str) -> Result<(), String> {
    let script = r#"$fieldName = $args[0]
$word = $null
try { $word = [Runtime.InteropServices.Marshal]::GetActiveObject('Word.Application') } catch {}
if ($null -eq $word) { $word = New-Object -ComObject Word.Application }
$word.Visible = $true
$doc = if ($word.Documents.Count -gt 0) { $word.ActiveDocument } else { $word.Documents.Add() }
$doc.MailMerge.MainDocumentType = 1
$doc.MailMerge.Fields.Add($word.Selection.Range, $fieldName) | Out-Null"#;
    ps_run(script, &[field.to_string()]).map(|_| ())
}

fn ps_word_insert_inline(fields: &[String]) -> Result<(), String> {
    let json = serde_json::to_string(fields).map_err(|e| e.to_string())?;
    let script = r#"$json = $args[0]
$fields = ConvertFrom-Json $json
$word = $null
try { $word = [Runtime.InteropServices.Marshal]::GetActiveObject('Word.Application') } catch {}
if ($null -eq $word) { $word = New-Object -ComObject Word.Application }
$word.Visible = $true
$doc = if ($word.Documents.Count -gt 0) { $word.ActiveDocument } else { $word.Documents.Add() }
$doc.MailMerge.MainDocumentType = 1
$first = $true
foreach($f in $fields){
  if (-not $first) { $word.Selection.TypeText(' ') }
  $doc.MailMerge.Fields.Add($word.Selection.Range, [string]$f) | Out-Null
  $first = $false
}"#;
    ps_run(script, &[json]).map(|_| ())
}

fn ps_word_insert_table(headers: &[String], values: Option<&[String]>) -> Result<(), String> {
    let headers_json = serde_json::to_string(headers).map_err(|e| e.to_string())?;
    let values_json = serde_json::to_string(values.unwrap_or(&[])).map_err(|e| e.to_string())?;
    let fill_mode = if values.is_some() { "1" } else { "0" };
    let script = r#"$headers = ConvertFrom-Json $args[0]
$values = ConvertFrom-Json $args[1]
$fillMode = [int]$args[2]
$word = $null
try { $word = [Runtime.InteropServices.Marshal]::GetActiveObject('Word.Application') } catch {}
if ($null -eq $word) { $word = New-Object -ComObject Word.Application }
$word.Visible = $true
$doc = if ($word.Documents.Count -gt 0) { $word.ActiveDocument } else { $word.Documents.Add() }
if ($fillMode -eq 0) { $doc.MailMerge.MainDocumentType = 1 }
$cols = [Math]::Max(1, $headers.Count)
$tbl = $doc.Tables.Add($word.Selection.Range, 2, $cols)
for($i=1; $i -le $cols; $i++){
  $tbl.Cell(1, $i).Range.Text = [string]$headers[$i-1]
  if ($fillMode -eq 1) {
    $tbl.Cell(2, $i).Range.Text = if ($i-1 -lt $values.Count) { [string]$values[$i-1] } else { '' }
  } else {
    $doc.MailMerge.Fields.Add($tbl.Cell(2, $i).Range, [string]$headers[$i-1]) | Out-Null
  }
}"#;
    ps_run(script, &[headers_json, values_json, fill_mode.to_string()]).map(|_| ())
}

fn ps_word_fill_inline(values: &[String]) -> Result<(), String> {
    let joined = values.join(" ");
    let script = r#"$text = $args[0]
$word = $null
try { $word = [Runtime.InteropServices.Marshal]::GetActiveObject('Word.Application') } catch {}
if ($null -eq $word) { $word = New-Object -ComObject Word.Application }
$word.Visible = $true
if ($word.Documents.Count -le 0) { $word.Documents.Add() | Out-Null }
$word.Selection.TypeText($text)"#;
    ps_run(script, &[joined]).map(|_| ())
}

unsafe fn set_font(hwnd: HWND, font: *mut core::ffi::c_void) {
    if !hwnd.is_null() {
        SendMessageW(hwnd, WM_SETFONT, font as usize, 1);
    }
}

unsafe fn create_ctrl(class: &str, text: &str, style: u32, ex_style: u32, x: i32, y: i32, w: i32, h: i32, parent: HWND, id: isize, font: *mut core::ffi::c_void) -> HWND {
    let hwnd = CreateWindowExW(
        ex_style,
        to_wide(class).as_ptr(),
        to_wide(text).as_ptr(),
        WS_CHILD | WS_VISIBLE | style,
        x, y, w, h,
        parent,
        id as usize as _,
        GetModuleHandleW(null()),
        null(),
    );
    set_font(hwnd, font);
    hwnd
}

unsafe fn set_status(hwnd: HWND, text: &str) {
    let hh = GetDlgItem(hwnd, IDC_STATUS as i32);
    if !hh.is_null() {
        SetWindowTextW(hh, to_wide(text).as_ptr());
    }
}

unsafe fn refresh_action_labels(hwnd: HWND) {
    let fill = is_fill_mode(hwnd);
    SetWindowTextW(GetDlgItem(hwnd, IDC_INSERT_FIELD as i32), to_wide(if fill { "填字段" } else { "插入字段" }).as_ptr());
    SetWindowTextW(GetDlgItem(hwnd, IDC_INSERT_INLINE as i32), to_wide(if fill { "填一行" } else { "插入一行" }).as_ptr());
    SetWindowTextW(GetDlgItem(hwnd, IDC_INSERT_TABLE as i32), to_wide(if fill { "填表格" } else { "插入表格" }).as_ptr());
}

unsafe fn lb_reset(hwnd: HWND) {
    SendDlgItemMessageW(hwnd, IDC_FIELDS as i32, LB_RESETCONTENT, 0, 0);
}

unsafe fn lb_fill(hwnd: HWND, items: &[String]) {
    lb_reset(hwnd);
    for item in items {
        SendDlgItemMessageW(hwnd, IDC_FIELDS as i32, LB_ADDSTRING, 0, to_wide(item).as_ptr() as LPARAM);
    }
}

unsafe fn combo_fill(hwnd: HWND, items: &[String], active: &str) {
    let combo = GetDlgItem(hwnd, IDC_SHEET as i32);
    SendMessageW(combo, CB_RESETCONTENT, 0, 0);
    let mut sel = 0usize;
    for (idx, item) in items.iter().enumerate() {
        SendMessageW(combo, CB_ADDSTRING, 0, to_wide(item).as_ptr() as LPARAM);
        if item == active {
            sel = idx;
        }
    }
    SendMessageW(combo, CB_SETCURSEL, sel, 0);
}

unsafe fn current_combo_text(hwnd: HWND) -> String {
    let combo = GetDlgItem(hwnd, IDC_SHEET as i32);
    let idx = SendMessageW(combo, CB_GETCURSEL, 0, 0) as i32;
    if idx < 0 {
        return String::new();
    }
    let len = SendMessageW(combo, CB_GETLBTEXTLEN, idx as usize, 0) as usize;
    let mut buf = vec![0u16; len + 1];
    SendMessageW(combo, CB_GETLBTEXT, idx as usize, buf.as_mut_ptr() as LPARAM);
    String::from_utf16_lossy(&buf).trim_end_matches('\0').to_string()
}

unsafe fn selected_field(hwnd: HWND) -> Option<String> {
    let idx = SendDlgItemMessageW(hwnd, IDC_FIELDS as i32, LB_GETCURSEL, 0, 0) as i32;
    if idx < 0 {
        return None;
    }
    let len = SendDlgItemMessageW(hwnd, IDC_FIELDS as i32, LB_GETTEXTLEN, idx as usize, 0) as usize;
    let mut buf = vec![0u16; len + 1];
    SendDlgItemMessageW(hwnd, IDC_FIELDS as i32, LB_GETTEXT, idx as usize, buf.as_mut_ptr() as LPARAM);
    Some(String::from_utf16_lossy(&buf).trim_end_matches('\0').to_string())
}

unsafe fn is_fill_mode(hwnd: HWND) -> bool {
    SendDlgItemMessageW(hwnd, IDC_MODE_FILL as i32, BM_GETCHECK, 0, 0) == BST_CHECKED as isize
}

unsafe fn current_data_row(hwnd: HWND) -> i32 {
    get_window_text(GetDlgItem(hwnd, IDC_DATA_ROW as i32))
        .parse::<i32>()
        .ok()
        .unwrap_or(1)
        .max(1)
}

unsafe fn set_edit_number(hwnd: HWND, id: isize, value: i32) {
    SetWindowTextW(GetDlgItem(hwnd, id as i32), to_wide(&value.max(1).to_string()).as_ptr());
}

fn selected_field_value(st: &MailMergeState, field: &str) -> String {
    let idx = st.headers.iter().position(|h| h == field).unwrap_or(0);
    st.values.get(idx).cloned().unwrap_or_default()
}

unsafe fn run_primary_action(hwnd: HWND, st: &MailMergeState) {
    if let Some(field) = selected_field(hwnd) {
        let res = if is_fill_mode(hwnd) {
            ps_word_fill_inline(&[selected_field_value(st, &field)])
        } else {
            ps_word_insert_field(&field)
        };
        if let Err(e) = res {
            set_status(hwnd, &e);
        } else if is_fill_mode(hwnd) {
            set_status(hwnd, "已填入当前字段值");
        } else {
            set_status(hwnd, "已插入邮件合并字段");
        }
    } else {
        set_status(hwnd, "请先在字段列表中选择一个字段");
    }
}

unsafe fn reload_excel(hwnd: HWND, st: &mut MailMergeState) {
    let excel = get_window_text(GetDlgItem(hwnd, IDC_EXCEL as i32));
    let sheet = current_combo_text(hwnd);
    let header_row = get_window_text(GetDlgItem(hwnd, IDC_HEADER_ROW as i32)).parse::<i32>().ok().unwrap_or(1).max(1);
    let data_row = get_window_text(GetDlgItem(hwnd, IDC_DATA_ROW as i32)).parse::<i32>().ok().unwrap_or(1).max(1);
    match ps_inspect_excel(&excel, &sheet, header_row, data_row) {
        Ok(data) => {
            st.excel_path = excel;
            st.headers = data.headers.clone();
            st.values = data.values.clone();
            st.row_count = data.row_count;
            combo_fill(hwnd, &data.sheet_names, &data.active_sheet);
            lb_fill(hwnd, &st.headers);
            set_status(hwnd, &format!("字段 {} 个，数据行 {}，当前第 {} 行", st.headers.len(), st.row_count, data_row));
        }
        Err(e) => set_status(hwnd, &e),
    }
}

fn copy_fields_to_clipboard(fields: &[String]) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| format!("打开剪贴板失败: {e}"))?;
    clipboard.set_text(fields.join("\r\n")).map_err(|e| format!("复制字段失败: {e}"))
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            let font = CreateFontW(-16, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, to_wide("Segoe UI Variable Text").as_ptr()) as *mut core::ffi::c_void;
            let font = if font.is_null() { GetStockObject(DEFAULT_GUI_FONT) as _ } else { font };
            let st = Box::new(MailMergeState::default());
            create_ctrl("STATIC", "超级邮件合并", 0, 0, 16, 14, 260, 24, hwnd, 9001, font);
            create_ctrl("STATIC", "Excel", 0, 0, 16, 50, 44, 22, hwnd, 9002, font);
            create_ctrl("EDIT", "", ES_AUTOHSCROLL as u32 | WS_TABSTOP, WS_EX_CLIENTEDGE, 64, 48, 520, 28, hwnd, IDC_EXCEL, font);
            create_ctrl("BUTTON", "加载", BS_PUSHBUTTON as u32 | WS_TABSTOP, 0, 594, 48, 76, 28, hwnd, IDC_LOAD, font);
            create_ctrl("STATIC", "工作表", 0, 0, 16, 88, 44, 22, hwnd, 9003, font);
            create_ctrl("COMBOBOX", "", CBS_DROPDOWNLIST as u32 | WS_VSCROLL | WS_TABSTOP, 0, 64, 86, 210, 240, hwnd, IDC_SHEET, font);
            create_ctrl("STATIC", "表头行", 0, 0, 286, 88, 50, 22, hwnd, 9004, font);
            create_ctrl("EDIT", "1", ES_AUTOHSCROLL as u32 | WS_TABSTOP, WS_EX_CLIENTEDGE, 338, 86, 52, 28, hwnd, IDC_HEADER_ROW, font);
            create_ctrl("STATIC", "数据行", 0, 0, 404, 88, 50, 22, hwnd, 9005, font);
            create_ctrl("EDIT", "1", ES_AUTOHSCROLL as u32 | WS_TABSTOP, WS_EX_CLIENTEDGE, 456, 86, 52, 28, hwnd, IDC_DATA_ROW, font);
            create_ctrl("BUTTON", "上一行", BS_PUSHBUTTON as u32 | WS_TABSTOP, 0, 522, 86, 70, 28, hwnd, IDC_PREV_ROW, font);
            create_ctrl("BUTTON", "下一行", BS_PUSHBUTTON as u32 | WS_TABSTOP, 0, 600, 86, 70, 28, hwnd, IDC_NEXT_ROW, font);
            create_ctrl("BUTTON", "邮件合并", BS_AUTORADIOBUTTON as u32 | WS_TABSTOP, 0, 64, 124, 96, 28, hwnd, IDC_MODE_MERGE, font);
            create_ctrl("BUTTON", "数据填表", BS_AUTORADIOBUTTON as u32 | WS_TABSTOP, 0, 170, 124, 96, 28, hwnd, IDC_MODE_FILL, font);
            SendDlgItemMessageW(hwnd, IDC_MODE_MERGE as i32, BM_SETCHECK, BST_CHECKED, 0);
            create_ctrl("LISTBOX", "", LBS_NOTIFY as u32 | WS_VSCROLL | WS_TABSTOP, WS_EX_CLIENTEDGE, 16, 170, 654, 260, hwnd, IDC_FIELDS, font);
            create_ctrl("BUTTON", "打开 Word", BS_PUSHBUTTON as u32 | WS_TABSTOP, 0, 16, 446, 110, 30, hwnd, IDC_OPEN_WORD, font);
            create_ctrl("BUTTON", "插入字段", BS_PUSHBUTTON as u32 | WS_TABSTOP, 0, 136, 446, 110, 30, hwnd, IDC_INSERT_FIELD, font);
            create_ctrl("BUTTON", "插入一行", BS_PUSHBUTTON as u32 | WS_TABSTOP, 0, 256, 446, 110, 30, hwnd, IDC_INSERT_INLINE, font);
            create_ctrl("BUTTON", "插入表格", BS_PUSHBUTTON as u32 | WS_TABSTOP, 0, 376, 446, 110, 30, hwnd, IDC_INSERT_TABLE, font);
            create_ctrl("STATIC", "就绪：输入 Excel 路径后点击加载。", 0, 0, 16, 486, 654, 22, hwnd, IDC_STATUS, font);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(st) as isize);
            apply_window_corner_preference(hwnd);
            apply_dark_mode_to_window(hwnd);
            0
        }
        WM_COMMAND => {
            let id = (wparam as u32 & 0xffff) as isize;
            let st = &mut *(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut MailMergeState);
            match id {
                IDC_LOAD => reload_excel(hwnd, st),
                IDC_SHEET => {
                    if ((wparam as u32 >> 16) & 0xffff) == CBN_SELCHANGE {
                        reload_excel(hwnd, st);
                    }
                }
                IDC_PREV_ROW => {
                    let cur = get_window_text(GetDlgItem(hwnd, IDC_DATA_ROW as i32)).parse::<i32>().ok().unwrap_or(1).max(1);
                    SetWindowTextW(GetDlgItem(hwnd, IDC_DATA_ROW as i32), to_wide(&(cur - 1).max(1).to_string()).as_ptr());
                    reload_excel(hwnd, st);
                }
                IDC_NEXT_ROW => {
                    let cur = get_window_text(GetDlgItem(hwnd, IDC_DATA_ROW as i32)).parse::<i32>().ok().unwrap_or(1).max(1);
                    SetWindowTextW(GetDlgItem(hwnd, IDC_DATA_ROW as i32), to_wide(&(cur + 1).to_string()).as_ptr());
                    reload_excel(hwnd, st);
                }
                IDC_OPEN_WORD => {
                    if let Err(e) = ps_word_open() { set_status(hwnd, &e); } else { set_status(hwnd, "已打开 Word"); }
                }
                IDC_INSERT_FIELD => {
                    if let Some(field) = selected_field(hwnd) {
                        let res = if is_fill_mode(hwnd) {
                            let idx = st.headers.iter().position(|h| h == &field).unwrap_or(0);
                            ps_word_fill_inline(&[st.values.get(idx).cloned().unwrap_or_default()])
                        } else {
                            ps_word_insert_field(&field)
                        };
                        if let Err(e) = res { set_status(hwnd, &e); } else { set_status(hwnd, "已执行字段操作"); }
                    } else {
                        set_status(hwnd, "请先选择字段");
                    }
                }
                IDC_INSERT_INLINE => {
                    let res = if is_fill_mode(hwnd) {
                        ps_word_fill_inline(&st.values)
                    } else {
                        ps_word_insert_inline(&st.headers)
                    };
                    if let Err(e) = res { set_status(hwnd, &e); } else { set_status(hwnd, "已插入一行"); }
                }
                IDC_INSERT_TABLE => {
                    let res = if is_fill_mode(hwnd) {
                        ps_word_insert_table(&st.headers, Some(&st.values))
                    } else {
                        ps_word_insert_table(&st.headers, None)
                    };
                    if let Err(e) = res { set_status(hwnd, &e); } else { set_status(hwnd, "已插入表格"); }
                }
                _ => {}
            }
            0
        }
        WM_CLOSE => { DestroyWindow(hwnd); 0 }
        WM_NCDESTROY => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut MailMergeState;
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
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

pub(crate) unsafe fn launch_mail_merge_window(owner: HWND) {
    ensure_class();
    let class_name = to_wide(CLASS_NAME);
    let title = to_wide("超级邮件合并");
    let mut rc: RECT = zeroed();
    if !owner.is_null() {
        GetWindowRect(owner, &mut rc);
    }
    let x = if rc.right > rc.left { rc.left + 80 } else { CW_USEDEFAULT };
    let y = if rc.bottom > rc.top { rc.top + 60 } else { CW_USEDEFAULT };
    let hwnd = CreateWindowExW(
        WS_EX_APPWINDOW,
        class_name.as_ptr(),
        title.as_ptr(),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_VISIBLE,
        x,
        y,
        700,
        560,
        owner,
        null_mut(),
        GetModuleHandleW(null()),
        null(),
    );
    if !hwnd.is_null() {
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
    }
}

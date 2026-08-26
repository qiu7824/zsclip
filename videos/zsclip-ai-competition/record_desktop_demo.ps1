$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$runtime = Join-Path $PSScriptRoot "runtime"
$exe = Join-Path $runtime "zsclip.exe"
$outDir = Join-Path $PSScriptRoot "assets\recordings"
$ffmpegCwd = Join-Path $root "videos\zsclip-0.9-preview"
$out = Join-Path $outDir "03-desktop-non-overlap-demo.mp4"
$targetFormScript = Join-Path $PSScriptRoot "demo_target_form.ps1"
$desktopDir = Join-Path ([Environment]::GetFolderPath("Desktop")) "ZSClip视频素材"

New-Item -ItemType Directory -Force -Path $outDir,$desktopDir | Out-Null

Add-Type -AssemblyName System.Windows.Forms
Add-Type @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public class DeskDemoWin {
  public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int max);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int cmd);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr hWnd, int x, int y, int w, int h, bool repaint);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr hWndInsertAfter, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr hWnd, StringBuilder className, int max);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint dwFlags, uint dx, uint dy, uint dwData, UIntPtr dwExtraInfo);
  [DllImport("user32.dll")] public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtraInfo);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern bool PostMessage(IntPtr hWnd, uint msg, UIntPtr wParam, IntPtr lParam);
}
'@

$HWND_TOPMOST = [IntPtr](-1)
$HWND_NOTOPMOST = [IntPtr](-2)
$SWP_SHOWWINDOW = 0x0040
$WM_VV_SHOW = [uint32](0x8000 + 20)
$WM_VV_SELECT = [uint32](0x8000 + 22)

$appLeft = 80
$appTop = 130
$appWidth = 320
$appHeight = 620
$noteLeft = 460
$noteTop = 130
$noteWidth = 780
$noteHeight = 620
$captureLeft = 40
$captureTop = 95
$captureWidth = 1260
$captureHeight = 720

function Get-TopWindowForProcess {
  param([int]$ProcessId, [string]$TitleContains = "")
  $matches = New-Object System.Collections.ArrayList
  $cb = [DeskDemoWin+EnumWindowsProc]{
    param($hwnd, $lparam)
    $windowPid = [uint32]0
    [DeskDemoWin]::GetWindowThreadProcessId($hwnd, [ref]$windowPid) | Out-Null
    if ($windowPid -eq [uint32]$ProcessId -and [DeskDemoWin]::IsWindowVisible($hwnd)) {
      $sb = New-Object System.Text.StringBuilder 256
      [DeskDemoWin]::GetWindowText($hwnd, $sb, 256) | Out-Null
      $title = $sb.ToString()
      if ($TitleContains -eq "" -or $title.Contains($TitleContains)) {
        [void]$matches.Add($hwnd)
      }
    }
    return $true
  }
  [DeskDemoWin]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null
  if ($matches.Count -gt 0) { return [IntPtr]$matches[0] }
  return [IntPtr]::Zero
}

function Get-TopWindowByTitle {
  param([string]$TitleContains)
  $matches = New-Object System.Collections.ArrayList
  $cb = [DeskDemoWin+EnumWindowsProc]{
    param($hwnd, $lparam)
    if ([DeskDemoWin]::IsWindowVisible($hwnd)) {
      $sb = New-Object System.Text.StringBuilder 256
      [DeskDemoWin]::GetWindowText($hwnd, $sb, 256) | Out-Null
      $title = $sb.ToString()
      if ($title.Contains($TitleContains)) {
        [void]$matches.Add($hwnd)
      }
    }
    return $true
  }
  [DeskDemoWin]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null
  if ($matches.Count -gt 0) { return [IntPtr]$matches[0] }
  return [IntPtr]::Zero
}

function Click-At {
  param([int]$X, [int]$Y)
  [DeskDemoWin]::SetCursorPos($X, $Y) | Out-Null
  Start-Sleep -Milliseconds 160
  [DeskDemoWin]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 80
  [DeskDemoWin]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 180
}

function Press-KeyCode {
  param([byte]$Vk, [int]$DelayMs = 70)
  [DeskDemoWin]::keybd_event($Vk, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds $DelayMs
  [DeskDemoWin]::keybd_event($Vk, 0, 0x0002, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds $DelayMs
}

function Press-CtrlCombo {
  param([byte]$Vk)
  [DeskDemoWin]::keybd_event(0x11, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 60
  Press-KeyCode $Vk
  [DeskDemoWin]::keybd_event(0x11, 0, 0x0002, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 120
}

function Paste-Text {
  param([string]$Text)
  Set-Clipboard -Value $Text
  Start-Sleep -Milliseconds 100
  Press-CtrlCombo 0x56
}

function Show-VvPopupAndSelectOne {
  param([IntPtr]$MainHwnd, [IntPtr]$TargetHwnd)
  Press-KeyCode 0x56 90
  Press-KeyCode 0x56 90
  Start-Sleep -Milliseconds 250
  [DeskDemoWin]::PostMessage(
    $MainHwnd,
    $WM_VV_SHOW,
    ([UIntPtr]::new([uint64]$TargetHwnd.ToInt64())),
    [IntPtr]::Zero
  ) | Out-Null
  Start-Sleep -Milliseconds 3200
  [DeskDemoWin]::PostMessage($MainHwnd, $WM_VV_SELECT, [UIntPtr]::Zero, [IntPtr]::Zero) | Out-Null
}

function Minimize-OtherWindows {
  param([IntPtr[]]$Keep = @())
  $keepValues = @{}
  foreach ($hwnd in $Keep) {
    if ($hwnd -ne [IntPtr]::Zero) {
      $keepValues[$hwnd.ToInt64()] = $true
    }
  }

  $cb = [DeskDemoWin+EnumWindowsProc]{
    param($hwnd, $lparam)
    if ($keepValues.ContainsKey($hwnd.ToInt64())) { return $true }
    if (-not [DeskDemoWin]::IsWindowVisible($hwnd)) { return $true }

    $className = New-Object System.Text.StringBuilder 256
    [DeskDemoWin]::GetClassName($hwnd, $className, 256) | Out-Null
    if (@("Progman", "WorkerW", "Shell_TrayWnd").Contains($className.ToString())) { return $true }

    $title = New-Object System.Text.StringBuilder 256
    [DeskDemoWin]::GetWindowText($hwnd, $title, 256) | Out-Null
    if ([string]::IsNullOrWhiteSpace($title.ToString())) { return $true }

    [DeskDemoWin]::ShowWindow($hwnd, 6) | Out-Null
    return $true
  }
  [DeskDemoWin]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null
}

Get-Process zsclip -ErrorAction SilentlyContinue |
  Where-Object { $_.Path -like "*videos*zsclip-ai-competition*" } |
  Stop-Process -Force
Get-Process pwsh -ErrorAction SilentlyContinue |
  Where-Object { $_.MainWindowTitle -like "*ZSClip 演示目标*" } |
  Stop-Process -Force

$shell = New-Object -ComObject Shell.Application
$shell.MinimizeAll()
Minimize-OtherWindows
Start-Sleep -Seconds 1

$app = Start-Process -FilePath $exe -WorkingDirectory $runtime -PassThru
Start-Sleep -Seconds 2
$targetProc = Start-Process -FilePath "pwsh" -ArgumentList @(
  "-NoProfile",
  "-ExecutionPolicy", "Bypass",
  "-File", $targetFormScript,
  "$noteLeft", "$noteTop", "$noteWidth", "$noteHeight"
) -PassThru
Start-Sleep -Seconds 2

$appHwnd = Get-TopWindowForProcess -ProcessId $app.Id -TitleContains "剪贴板"
if ($appHwnd -eq [IntPtr]::Zero) {
  $app.Refresh()
  $appHwnd = [IntPtr]$app.MainWindowHandle
}
$noteHwnd = Get-TopWindowByTitle -TitleContains "ZSClip 演示目标"

if ($appHwnd -ne [IntPtr]::Zero) {
  [DeskDemoWin]::ShowWindow($appHwnd, 5) | Out-Null
  [DeskDemoWin]::SetWindowPos($appHwnd, $HWND_TOPMOST, $appLeft, $appTop, $appWidth, $appHeight, $SWP_SHOWWINDOW) | Out-Null
}
if ($noteHwnd -ne [IntPtr]::Zero) {
  [DeskDemoWin]::ShowWindow($noteHwnd, 5) | Out-Null
  [DeskDemoWin]::SetWindowPos($noteHwnd, $HWND_TOPMOST, $noteLeft, $noteTop, $noteWidth, $noteHeight, $SWP_SHOWWINDOW) | Out-Null
  [DeskDemoWin]::SetForegroundWindow($noteHwnd) | Out-Null
}
Minimize-OtherWindows -Keep @($appHwnd, $noteHwnd)

$ffArgs = @(
  "remotion", "ffmpeg",
  "-y",
  "-f", "gdigrab",
  "-framerate", "30",
  "-draw_mouse", "1",
  "-offset_x", "$captureLeft",
  "-offset_y", "$captureTop",
  "-video_size", "${captureWidth}x${captureHeight}",
  "-i", "desktop",
  "-t", "24",
  "-c:v", "libx264",
  "-preset", "ultrafast",
  "-pix_fmt", "yuv420p",
  $out
)
$rec = Start-Process -FilePath "npx.cmd" -ArgumentList $ffArgs -WorkingDirectory $ffmpegCwd -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 2

if ($noteHwnd -ne [IntPtr]::Zero) {
  [DeskDemoWin]::SetForegroundWindow($noteHwnd) | Out-Null
  Click-At ($noteLeft + 160) ($noteTop + 95)
}

if ($appHwnd -ne [IntPtr]::Zero) {
  [DeskDemoWin]::SetForegroundWindow($appHwnd) | Out-Null
  Click-At ($appLeft + 178) ($appTop + 18)
  Paste-Text "WPS 合同"
  Start-Sleep -Seconds 2
  Click-At ($appLeft + 120) ($appTop + 100)
  Start-Sleep -Seconds 1
}

if ($noteHwnd -ne [IntPtr]::Zero) {
  [DeskDemoWin]::SetForegroundWindow($noteHwnd) | Out-Null
  Click-At ($noteLeft + 220) ($noteTop + 170)
  Press-CtrlCombo 0x23
  Press-KeyCode 0x0D
  Press-KeyCode 0x0D
  Show-VvPopupAndSelectOne -MainHwnd $appHwnd -TargetHwnd $noteHwnd
  Start-Sleep -Seconds 2
  Start-Sleep -Seconds 3
}

Wait-Process -Id $rec.Id
if ($appHwnd -ne [IntPtr]::Zero) {
  [DeskDemoWin]::SetWindowPos($appHwnd, $HWND_NOTOPMOST, $appLeft, $appTop, $appWidth, $appHeight, $SWP_SHOWWINDOW) | Out-Null
}
if ($noteHwnd -ne [IntPtr]::Zero) {
  [DeskDemoWin]::SetWindowPos($noteHwnd, $HWND_NOTOPMOST, $noteLeft, $noteTop, $noteWidth, $noteHeight, $SWP_SHOWWINDOW) | Out-Null
}
if ($targetProc -and -not $targetProc.HasExited) {
  Stop-Process -Id $targetProc.Id -Force
}
Copy-Item -LiteralPath $out -Destination (Join-Path $desktopDir "03-desktop-non-overlap-demo.mp4") -Force
Write-Output $out
Write-Output (Join-Path $desktopDir "03-desktop-non-overlap-demo.mp4")

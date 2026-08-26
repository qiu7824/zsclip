$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$runtime = Join-Path $PSScriptRoot "runtime"
$exe = Join-Path $runtime "zsclip.exe"
$outDir = Join-Path $PSScriptRoot "assets\recordings"
$ffmpegCwd = Join-Path $root "videos\zsclip-0.9-preview"
$out = Join-Path $outDir "05-feature-closeups-search-group-ocr.mp4"
$desktopDir = Join-Path ([Environment]::GetFolderPath("Desktop")) "ZSClip视频素材"

New-Item -ItemType Directory -Force -Path $outDir,$desktopDir | Out-Null

Add-Type -AssemblyName System.Windows.Forms
Add-Type @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public class FeatureCloseupWin {
  public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int max);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr hWnd, StringBuilder className, int max);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int cmd);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr hWndInsertAfter, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint dwFlags, uint dx, uint dy, uint dwData, UIntPtr dwExtraInfo);
  [DllImport("user32.dll")] public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtraInfo);
}
'@

$HWND_TOPMOST = [IntPtr](-1)
$SWP_SHOWWINDOW = 0x0040

$appLeft = 70
$appTop = 100
$appWidth = 380
$appHeight = 660
$captureLeft = 30
$captureTop = 72
$captureWidth = 1260
$captureHeight = 720

function Get-TopWindowForProcess {
  param([int]$ProcessId, [string]$TitleContains = "")
  $matches = New-Object System.Collections.ArrayList
  $cb = [FeatureCloseupWin+EnumWindowsProc]{
    param($hwnd, $lparam)
    $windowPid = [uint32]0
    [FeatureCloseupWin]::GetWindowThreadProcessId($hwnd, [ref]$windowPid) | Out-Null
    if ($windowPid -eq [uint32]$ProcessId -and [FeatureCloseupWin]::IsWindowVisible($hwnd)) {
      $sb = New-Object System.Text.StringBuilder 256
      [FeatureCloseupWin]::GetWindowText($hwnd, $sb, 256) | Out-Null
      $title = $sb.ToString()
      if ($TitleContains -eq "" -or $title.Contains($TitleContains)) {
        [void]$matches.Add($hwnd)
      }
    }
    return $true
  }
  [FeatureCloseupWin]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null
  if ($matches.Count -gt 0) { return [IntPtr]$matches[0] }
  return [IntPtr]::Zero
}

function Minimize-OtherWindows {
  param([IntPtr[]]$Keep = @())
  $keepValues = @{}
  foreach ($hwnd in $Keep) {
    if ($hwnd -ne [IntPtr]::Zero) { $keepValues[$hwnd.ToInt64()] = $true }
  }

  $cb = [FeatureCloseupWin+EnumWindowsProc]{
    param($hwnd, $lparam)
    if ($keepValues.ContainsKey($hwnd.ToInt64())) { return $true }
    if (-not [FeatureCloseupWin]::IsWindowVisible($hwnd)) { return $true }

    $className = New-Object System.Text.StringBuilder 256
    [FeatureCloseupWin]::GetClassName($hwnd, $className, 256) | Out-Null
    if (@("Progman", "WorkerW", "Shell_TrayWnd").Contains($className.ToString())) { return $true }

    $title = New-Object System.Text.StringBuilder 256
    [FeatureCloseupWin]::GetWindowText($hwnd, $title, 256) | Out-Null
    if ([string]::IsNullOrWhiteSpace($title.ToString())) { return $true }

    [FeatureCloseupWin]::ShowWindow($hwnd, 6) | Out-Null
    return $true
  }
  [FeatureCloseupWin]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null
}

function Click-At {
  param([int]$X, [int]$Y, [string]$Button = "left")
  [FeatureCloseupWin]::SetCursorPos($X, $Y) | Out-Null
  Start-Sleep -Milliseconds 150
  if ($Button -eq "right") {
    [FeatureCloseupWin]::mouse_event(0x0008, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 80
    [FeatureCloseupWin]::mouse_event(0x0010, 0, 0, 0, [UIntPtr]::Zero)
  } else {
    [FeatureCloseupWin]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 80
    [FeatureCloseupWin]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
  }
  Start-Sleep -Milliseconds 180
}

function Press-KeyCode {
  param([byte]$Vk, [int]$DelayMs = 70)
  [FeatureCloseupWin]::keybd_event($Vk, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds $DelayMs
  [FeatureCloseupWin]::keybd_event($Vk, 0, 0x0002, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds $DelayMs
}

function Press-CtrlCombo {
  param([byte]$Vk)
  [FeatureCloseupWin]::keybd_event(0x11, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 60
  Press-KeyCode $Vk
  [FeatureCloseupWin]::keybd_event(0x11, 0, 0x0002, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 120
}

function Paste-Search {
  param([string]$Text)
  Click-At ($appLeft + 104) ($appTop + 18)
  Press-CtrlCombo 0x41
  Set-Clipboard -Value $Text
  Start-Sleep -Milliseconds 80
  Press-CtrlCombo 0x56
  Start-Sleep -Milliseconds 1500
}

function Clear-Search {
  Click-At ($appLeft + 104) ($appTop + 18)
  Press-CtrlCombo 0x41
  Press-KeyCode 0x08
  Start-Sleep -Milliseconds 1000
}

Get-Process zsclip -ErrorAction SilentlyContinue |
  Stop-Process -Force

$shell = New-Object -ComObject Shell.Application
$shell.MinimizeAll()
Minimize-OtherWindows
Start-Sleep -Milliseconds 800

& python (Join-Path $PSScriptRoot "prepare_demo_runtime.py") | Out-Null

$app = Start-Process -FilePath $exe -WorkingDirectory $runtime -PassThru
Start-Sleep -Seconds 2
$appHwnd = Get-TopWindowForProcess -ProcessId $app.Id
if ($appHwnd -eq [IntPtr]::Zero) {
  $app.Refresh()
  $appHwnd = [IntPtr]$app.MainWindowHandle
}
if ($appHwnd -eq [IntPtr]::Zero) {
  throw "Could not find ZSClip window"
}

[FeatureCloseupWin]::ShowWindow($appHwnd, 5) | Out-Null
[FeatureCloseupWin]::SetWindowPos($appHwnd, $HWND_TOPMOST, $appLeft, $appTop, $appWidth, $appHeight, $SWP_SHOWWINDOW) | Out-Null
[FeatureCloseupWin]::SetForegroundWindow($appHwnd) | Out-Null
Minimize-OtherWindows -Keep @($appHwnd)
Start-Sleep -Milliseconds 700

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
  "-t", "56",
  "-c:v", "libx264",
  "-preset", "ultrafast",
  "-pix_fmt", "yuv420p",
  $out
)
$rec = Start-Process -FilePath "npx.cmd" -ArgumentList $ffArgs -WorkingDirectory $ffmpegCwd -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 2

[FeatureCloseupWin]::SetForegroundWindow($appHwnd) | Out-Null
Click-At ($appLeft + 178) ($appTop + 18)
Start-Sleep -Milliseconds 600

Paste-Search "合同"
Click-At ($appLeft + 175) ($appTop + 100) "right"
Start-Sleep -Seconds 4
Press-KeyCode 0x1B
Start-Sleep -Milliseconds 600

Paste-Search "日期:今天 发票"
Paste-Search "应用:WPS 合同"
Paste-Search "cargo 应用:Visual Studio Code"

Clear-Search
Click-At ($appLeft + 170) ($appTop + 140) "right"
Start-Sleep -Seconds 4
Press-KeyCode 0x1B
Start-Sleep -Milliseconds 700

Paste-Search "合同盖章扫描件"
Click-At ($appLeft + 170) ($appTop + 140) "right"
Start-Sleep -Seconds 4
Press-KeyCode 0x1B
Start-Sleep -Seconds 2

Wait-Process -Id $rec.Id

Copy-Item -LiteralPath $out -Destination (Join-Path $desktopDir "05-feature-closeups-search-group-ocr.mp4") -Force

if ($app -and -not $app.HasExited) {
  $app.CloseMainWindow() | Out-Null
  Start-Sleep -Milliseconds 500
  if (-not $app.HasExited) { $app.Kill() }
}

Write-Output $out
Write-Output (Join-Path $desktopDir "05-feature-closeups-search-group-ocr.mp4")

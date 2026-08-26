$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$html = Resolve-Path (Join-Path $PSScriptRoot "dev-status-scroll.html")
$outDir = Join-Path $PSScriptRoot "assets\recordings"
$ffmpegCwd = Join-Path $root "videos\zsclip-0.9-preview"
$out = Join-Path $outDir "04-dev-status-scroll.mp4"
$desktopDir = Join-Path ([Environment]::GetFolderPath("Desktop")) "ZSClip视频素材"

New-Item -ItemType Directory -Force -Path $outDir,$desktopDir | Out-Null

Add-Type @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public class DevScrollWin {
  public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int max);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int cmd);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr hWnd, int x, int y, int w, int h, bool repaint);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern bool PostMessage(IntPtr hWnd, uint msg, UIntPtr wParam, IntPtr lParam);
}
'@

function Get-TopWindowByTitle {
  param([string]$TitleContains)
  $matches = New-Object System.Collections.ArrayList
  $cb = [DevScrollWin+EnumWindowsProc]{
    param($hwnd, $lparam)
    if ([DevScrollWin]::IsWindowVisible($hwnd)) {
      $sb = New-Object System.Text.StringBuilder 256
      [DevScrollWin]::GetWindowText($hwnd, $sb, 256) | Out-Null
      $title = $sb.ToString()
      if ($title.Contains($TitleContains)) {
        [void]$matches.Add($hwnd)
      }
    }
    return $true
  }
  [DevScrollWin]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null
  if ($matches.Count -gt 0) { return [IntPtr]$matches[0] }
  return [IntPtr]::Zero
}

$shell = New-Object -ComObject Shell.Application
$shell.MinimizeAll()
Start-Sleep -Milliseconds 800

$fileUrl = ([Uri]$html.Path).AbsoluteUri
$edge = Start-Process -FilePath "msedge" -ArgumentList @("--new-window", "--app=$fileUrl") -PassThru

$edgeHwnd = [IntPtr]::Zero
for ($i = 0; $i -lt 30; $i++) {
  Start-Sleep -Milliseconds 300
  $edgeHwnd = Get-TopWindowByTitle -TitleContains "ZSClip 多平台"
  if ($edgeHwnd -ne [IntPtr]::Zero) { break }
}
if ($edgeHwnd -eq [IntPtr]::Zero) {
  throw "未找到 Edge 滚动素材窗口"
}

[DevScrollWin]::ShowWindow($edgeHwnd, 5) | Out-Null
[DevScrollWin]::MoveWindow($edgeHwnd, 70, 70, 1220, 790, $true) | Out-Null
[DevScrollWin]::SetForegroundWindow($edgeHwnd) | Out-Null
Start-Sleep -Milliseconds 700

$ffArgs = @(
  "remotion", "ffmpeg",
  "-y",
  "-f", "gdigrab",
  "-framerate", "30",
  "-draw_mouse", "1",
  "-offset_x", "50",
  "-offset_y", "60",
  "-video_size", "1280x720",
  "-i", "desktop",
  "-t", "31",
  "-c:v", "libx264",
  "-preset", "ultrafast",
  "-pix_fmt", "yuv420p",
  $out
)

$rec = Start-Process -FilePath "npx.cmd" -ArgumentList $ffArgs -WorkingDirectory $ffmpegCwd -PassThru -WindowStyle Hidden
Wait-Process -Id $rec.Id

[DevScrollWin]::PostMessage($edgeHwnd, 0x0010, [UIntPtr]::Zero, [IntPtr]::Zero) | Out-Null
Copy-Item -LiteralPath $out -Destination (Join-Path $desktopDir "04-dev-status-scroll.mp4") -Force
Write-Output $out
Write-Output (Join-Path $desktopDir "04-dev-status-scroll.mp4")

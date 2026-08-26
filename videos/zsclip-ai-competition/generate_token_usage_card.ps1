param(
  [int]$TokensUsed = 795135,
  [int]$TimeUsedSeconds = 2599,
  [string]$ThreadId = "019eeb47-10b4-7841-9e73-9b6457d95c7f"
)

$ErrorActionPreference = "Stop"

$outDir = Join-Path $PSScriptRoot "assets\screenshots"
$desktopDir = Join-Path ([Environment]::GetFolderPath("Desktop")) "ZSClip视频素材"
$out = Join-Path $outDir "07-codex-token-usage.png"
New-Item -ItemType Directory -Force -Path $outDir,$desktopDir | Out-Null

Add-Type -AssemblyName System.Drawing

function New-Brush([int]$r, [int]$g, [int]$b) {
  return New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb($r, $g, $b))
}

function Draw-RoundedRect {
  param($Graphics, $Brush, [float]$X, [float]$Y, [float]$W, [float]$H, [float]$R)
  $path = New-Object System.Drawing.Drawing2D.GraphicsPath
  $d = $R * 2
  $path.AddArc($X, $Y, $d, $d, 180, 90)
  $path.AddArc($X + $W - $d, $Y, $d, $d, 270, 90)
  $path.AddArc($X + $W - $d, $Y + $H - $d, $d, $d, 0, 90)
  $path.AddArc($X, $Y + $H - $d, $d, $d, 90, 90)
  $path.CloseFigure()
  $Graphics.FillPath($Brush, $path)
  $path.Dispose()
}

$width = 1280
$height = 720
$bmp = New-Object System.Drawing.Bitmap($width, $height)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::ClearTypeGridFit

$bg = New-Brush 245 247 251
$card = New-Brush 255 255 255
$ink = New-Brush 25 32 51
$muted = New-Brush 94 107 122
$blue = New-Brush 23 105 224
$lightBlue = New-Brush 232 241 255
$linePen = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(218, 225, 235), 2)

$g.FillRectangle($bg, 0, 0, $width, $height)
Draw-RoundedRect $g $card 96 82 1088 556 16
$g.DrawRectangle($linePen, 96, 82, 1088, 556)

$fontTitle = New-Object System.Drawing.Font("Microsoft YaHei UI", 38, [System.Drawing.FontStyle]::Bold)
$fontSub = New-Object System.Drawing.Font("Microsoft YaHei UI", 18, [System.Drawing.FontStyle]::Regular)
$fontLabel = New-Object System.Drawing.Font("Microsoft YaHei UI", 17, [System.Drawing.FontStyle]::Regular)
$fontValue = New-Object System.Drawing.Font("Segoe UI", 48, [System.Drawing.FontStyle]::Bold)
$fontCode = New-Object System.Drawing.Font("Cascadia Code", 16, [System.Drawing.FontStyle]::Regular)

$g.DrawString("Codex 构建过程统计", $fontTitle, $ink, 150, 130)
$g.DrawString("真实线程数据，用作视频里的 token 使用量截图素材", $fontSub, $muted, 154, 190)

Draw-RoundedRect $g $lightBlue 150 252 450 190 14
Draw-RoundedRect $g (New-Brush 238 247 241) 680 252 450 190 14
$g.DrawString("Tokens Used", $fontLabel, $muted, 188, 286)
$g.DrawString(("{0:N0}" -f $TokensUsed), $fontValue, $blue, 184, 318)

$elapsed = [TimeSpan]::FromSeconds($TimeUsedSeconds)
$elapsedText = "{0}分{1:D2}秒" -f [int]$elapsed.TotalMinutes, $elapsed.Seconds
$g.DrawString("Elapsed Time", $fontLabel, $muted, 718, 286)
$g.DrawString($elapsedText, $fontValue, (New-Brush 17 132 91), 714, 318)

$g.DrawString("Thread ID", $fontLabel, $muted, 154, 492)
$g.DrawString($ThreadId, $fontCode, $ink, 154, 526)
$g.DrawString("用于说明：这期视频不只展示产品，也展示 AI 协作、测试和多平台打磨过程。", $fontSub, $muted, 154, 580)

$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose()
$bmp.Dispose()

Copy-Item -LiteralPath $out -Destination (Join-Path $desktopDir "07-codex-token-usage.png") -Force
Write-Output $out
Write-Output (Join-Path $desktopDir "07-codex-token-usage.png")

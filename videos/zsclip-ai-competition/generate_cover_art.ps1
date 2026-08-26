$ErrorActionPreference = "Stop"

$outDir = Join-Path $PSScriptRoot "assets\cover"
$desktopDir = Join-Path ([Environment]::GetFolderPath("Desktop")) "ZSClip视频素材"
$out = Join-Path $outDir "bilibili-cover-zsclip.png"
$desktopOut = Join-Path $desktopDir "bilibili-cover-zsclip.png"
$productShot = Join-Path $PSScriptRoot "assets\qa\draft-v3-00-33-text-menu.png"
$tokenShot = Join-Path $PSScriptRoot "assets\screenshots\07-codex-token-usage.png"

New-Item -ItemType Directory -Force -Path $outDir,$desktopDir | Out-Null
Add-Type -AssemblyName System.Drawing

function New-Brush([int]$r, [int]$g, [int]$b) {
  return New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb($r, $g, $b))
}

function New-Pen([int]$r, [int]$g, [int]$b, [float]$w) {
  return New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb($r, $g, $b), $w)
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

function Draw-Text {
  param($Graphics, [string]$Text, $Font, $Brush, [float]$X, [float]$Y)
  $shadow = New-Brush 5 10 18
  $Graphics.DrawString($Text, $Font, $shadow, $X + 4, $Y + 4)
  $Graphics.DrawString($Text, $Font, $Brush, $X, $Y)
  $shadow.Dispose()
}

$bmp = New-Object System.Drawing.Bitmap 1920, 1080
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::ClearTypeGridFit

$bg = New-Brush 12 18 28
$g.FillRectangle($bg, 0, 0, 1920, 1080)

$band = New-Brush 19 32 48
$g.FillRectangle($band, 0, 820, 1920, 260)

$green = New-Brush 34 197 94
$blue = New-Brush 43 111 246
$white = New-Brush 255 255 255
$muted = New-Brush 203 213 225
$yellow = New-Brush 250 204 21

$fontBrand = New-Object System.Drawing.Font "Segoe UI", 72, ([System.Drawing.FontStyle]::Bold)
$fontTitle = New-Object System.Drawing.Font "Microsoft YaHei UI", 82, ([System.Drawing.FontStyle]::Bold)
$fontTitle2 = New-Object System.Drawing.Font "Microsoft YaHei UI", 88, ([System.Drawing.FontStyle]::Bold)
$fontSub = New-Object System.Drawing.Font "Microsoft YaHei UI", 36, ([System.Drawing.FontStyle]::Bold)
$fontTag = New-Object System.Drawing.Font "Microsoft YaHei UI", 28, ([System.Drawing.FontStyle]::Bold)
$fontSmall = New-Object System.Drawing.Font "Microsoft YaHei UI", 24, ([System.Drawing.FontStyle]::Regular)

Draw-Text $g "ZSClip" $fontBrand $white 96 78
$g.FillRectangle($green, 100, 190, 110, 12)
$g.FillRectangle($blue, 224, 190, 76, 12)
Draw-Text $g "我用 AI 打磨了" $fontTitle $white 94 244
Draw-Text $g "剪贴板办公副驾驶" $fontTitle2 $yellow 94 362
$g.DrawString("VV / OCR / 搜索 / 分组 / WPS", $fontSub, $muted, 102, 510)

$tags = @("Rust", "开源", "Windows 原生")
$tagX = 104
foreach ($tag in $tags) {
  $size = $g.MeasureString($tag, $fontTag)
  Draw-RoundedRect $g (New-Brush 30 41 59) $tagX 616 ($size.Width + 38) 60 18
  $g.DrawString($tag, $fontTag, $white, ($tagX + 19), 627)
  $tagX += $size.Width + 58
}

$shot = [System.Drawing.Image]::FromFile($productShot)
$srcRect = New-Object System.Drawing.Rectangle 0, 0, 610, 720
$dstRect = New-Object System.Drawing.Rectangle 1138, 112, 636, 850
Draw-RoundedRect $g (New-Brush 255 255 255) 1114 88 684 898 28
$g.DrawImage($shot, $dstRect, $srcRect, [System.Drawing.GraphicsUnit]::Pixel)
$g.DrawRectangle((New-Pen 226 232 240 4), 1114, 88, 684, 898)
$shot.Dispose()

$token = [System.Drawing.Image]::FromFile($tokenShot)
$tokenSrc = New-Object System.Drawing.Rectangle 0, 0, $token.Width, $token.Height
$tokenDst = New-Object System.Drawing.Rectangle 1198, 766, 510, 287
Draw-RoundedRect $g (New-Brush 248 250 252) 1180 748 546 320 20
$g.DrawImage($token, $tokenDst, $tokenSrc, [System.Drawing.GraphicsUnit]::Pixel)
$g.DrawRectangle((New-Pen 209 213 219 3), 1180, 748, 546, 320)
$token.Dispose()

$g.FillRectangle($green, 100, 878, 12, 50)
$g.DrawString("复制不是终点，复用才是工作流", $fontSub, $white, 132, 868)
$g.DrawString("截图 OCR、右键分组、组合搜索、低内存常驻", $fontSmall, $muted, 134, 928)

$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
Copy-Item -LiteralPath $out -Destination $desktopOut -Force

$g.Dispose()
$bmp.Dispose()

Write-Output $out
Write-Output $desktopOut

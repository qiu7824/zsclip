$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$ffmpegCwd = Join-Path $root "videos\zsclip-0.9-preview"
$recordings = Join-Path $PSScriptRoot "assets\recordings"
$screenshots = Join-Path $PSScriptRoot "assets\screenshots"
$work = Join-Path $PSScriptRoot "work\draft"
$cards = Join-Path $work "cards"
$outDir = Join-Path $PSScriptRoot "output"
$desktopDir = Join-Path ([Environment]::GetFolderPath("Desktop")) "ZSClip视频素材"
$out = Join-Path $outDir "zsclip-ai-competition-silent-draft.mp4"

New-Item -ItemType Directory -Force -Path $work,$cards,$outDir,$desktopDir | Out-Null
Add-Type -AssemblyName System.Drawing

function Invoke-RemotionFfmpeg {
  param([string[]]$FfmpegArgs)
  Push-Location $ffmpegCwd
  try {
    & npx.cmd remotion ffmpeg @FfmpegArgs
    if ($LASTEXITCODE -ne 0) {
      throw "ffmpeg failed with exit code $LASTEXITCODE"
    }
  } finally {
    Pop-Location
  }
}

function Normalize-Video {
  param([string]$InputPath, [string]$OutputPath)
  Invoke-RemotionFfmpeg -FfmpegArgs @(
    "-y",
    "-i", $InputPath,
    "-vf", "scale=1280:720",
    "-r", "30",
    "-an",
    "-c:v", "libx264",
    "-pix_fmt", "yuv420p",
    "-preset", "veryfast",
    $OutputPath
  )
}

function Image-Clip {
  param([string]$InputPath, [string]$OutputPath, [string]$Duration)
  Invoke-RemotionFfmpeg -FfmpegArgs @(
    "-y",
    "-loop", "1",
    "-t", $Duration,
    "-i", $InputPath,
    "-vf", "scale=1280:720",
    "-r", "30",
    "-an",
    "-c:v", "libx264",
    "-pix_fmt", "yuv420p",
    "-preset", "veryfast",
    $OutputPath
  )
}

function New-ImageCard {
  param([string]$InputPath, [string]$OutputPath, [string]$Title, [string]$Subtitle)
  $bmp = New-Object System.Drawing.Bitmap 1280, 720
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
  $g.Clear([System.Drawing.Color]::FromArgb(17, 24, 39))
  $titleFont = New-Object System.Drawing.Font "Microsoft YaHei UI", 38, ([System.Drawing.FontStyle]::Bold)
  $subFont = New-Object System.Drawing.Font "Microsoft YaHei UI", 20, ([System.Drawing.FontStyle]::Regular)
  $titleBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::White)
  $subBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(209, 213, 219))
  $panelBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::White)
  $borderPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(75, 85, 99)), 2
  $g.DrawString($Title, $titleFont, $titleBrush, 70, 52)
  $g.DrawString($Subtitle, $subFont, $subBrush, 72, 108)
  $src = [System.Drawing.Image]::FromFile($InputPath)
  $maxW = 1100
  $maxH = 500
  $scale = [Math]::Min($maxW / $src.Width, $maxH / $src.Height)
  $w = [int]($src.Width * $scale)
  $h = [int]($src.Height * $scale)
  $x = [int]((1280 - $w) / 2)
  $y = [int](166 + (($maxH - $h) / 2))
  $g.FillRectangle($panelBrush, $x - 12, $y - 12, $w + 24, $h + 24)
  $g.DrawRectangle($borderPen, $x - 12, $y - 12, $w + 24, $h + 24)
  $g.DrawImage($src, $x, $y, $w, $h)
  $src.Dispose()
  $bmp.Save($OutputPath, [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose()
  $bmp.Dispose()
}

function New-TextCard {
  param([string]$OutputPath, [string]$Title, [string]$Subtitle, [string[]]$Lines)
  $bmp = New-Object System.Drawing.Bitmap 1280, 720
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
  $g.Clear([System.Drawing.Color]::FromArgb(12, 18, 28))
  $titleFont = New-Object System.Drawing.Font "Microsoft YaHei UI", 54, ([System.Drawing.FontStyle]::Bold)
  $subFont = New-Object System.Drawing.Font "Microsoft YaHei UI", 24, ([System.Drawing.FontStyle]::Regular)
  $lineFont = New-Object System.Drawing.Font "Microsoft YaHei UI", 34, ([System.Drawing.FontStyle]::Bold)
  $titleBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::White)
  $subBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(209, 213, 219))
  $lineBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(241, 245, 249))
  $accentBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(34, 197, 94))
  $g.DrawString($Title, $titleFont, $titleBrush, 78, 78)
  $g.DrawString($Subtitle, $subFont, $subBrush, 82, 158)
  $y = 270
  foreach ($line in $Lines) {
    $g.FillRectangle($accentBrush, 86, $y + 12, 10, 38)
    $g.DrawString($line, $lineFont, $lineBrush, 118, $y)
    $y += 92
  }
  $bmp.Save($OutputPath, [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose()
  $bmp.Dispose()
}

$titleCard = Join-Path $cards "00-title.png"
$memoryCard = Join-Path $cards "03-memory-card.png"
$tokenCard = Join-Path $cards "04-token-card.png"
$roadmapCard = Join-Path $cards "05-roadmap-card.png"
New-TextCard $titleCard "ZSClip" "讲功能，也讲这个工具为什么被做出来" @(
  "搜索 / 分组 / VV / OCR",
  "内存优化 / 多平台原生 UI",
  "AI 协作构建过程"
)
New-ImageCard (Join-Path $screenshots "06-task-manager-zsclip-memory.png") $memoryCard "运行内存实测" "任务管理器搜索 zsclip，展示常驻占用"
New-ImageCard (Join-Path $screenshots "07-codex-token-usage.png") $tokenCard "AI 构建过程" "Codex 协作与 token 使用量截图"
New-TextCard $roadmapCard "后续更新" "下一步继续把剪贴板做深" @(
  "搜索添加：附近记录",
  "分组添加：文件类型选项",
  "继续打磨 macOS / Linux 多平台"
)

$clip0 = Join-Path $work "00-title.mp4"
$clip1 = Join-Path $work "01-product-demo.mp4"
$clip2 = Join-Path $work "02-feature-closeups.mp4"
$clip3 = Join-Path $work "03-memory.mp4"
$clip4 = Join-Path $work "04-token.mp4"
$clip5 = Join-Path $work "05-roadmap.mp4"
$clip6 = Join-Path $work "06-dev-scroll.mp4"

Image-Clip $titleCard $clip0 "4"
Normalize-Video (Join-Path $recordings "03-desktop-non-overlap-demo.mp4") $clip1
Normalize-Video (Join-Path $recordings "05-feature-closeups-search-group-ocr.mp4") $clip2
Image-Clip $memoryCard $clip3 "5"
Image-Clip $tokenCard $clip4 "5"
Image-Clip $roadmapCard $clip5 "6"
Normalize-Video (Join-Path $recordings "04-dev-status-scroll.mp4") $clip6

$concat = Join-Path $work "concat.txt"
@(
  "file '$($clip0.Replace("'", "'\''"))'",
  "file '$($clip1.Replace("'", "'\''"))'",
  "file '$($clip2.Replace("'", "'\''"))'",
  "file '$($clip3.Replace("'", "'\''"))'",
  "file '$($clip4.Replace("'", "'\''"))'",
  "file '$($clip5.Replace("'", "'\''"))'",
  "file '$($clip6.Replace("'", "'\''"))'"
) | Set-Content -Path $concat -Encoding utf8

Invoke-RemotionFfmpeg -FfmpegArgs @(
  "-y",
  "-f", "concat",
  "-safe", "0",
  "-i", $concat,
  "-c", "copy",
  $out
)

Copy-Item -LiteralPath $out -Destination (Join-Path $desktopDir "zsclip-ai-competition-silent-draft.mp4") -Force
Write-Output $out
Write-Output (Join-Path $desktopDir "zsclip-ai-competition-silent-draft.mp4")

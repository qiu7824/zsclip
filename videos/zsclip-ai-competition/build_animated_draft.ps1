$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$remotionDir = Join-Path $root "videos\zsclip-0.9-preview"
$publicDir = Join-Path $remotionDir "public\ai-competition"
$recordings = Join-Path $PSScriptRoot "assets\recordings"
$screenshots = Join-Path $PSScriptRoot "assets\screenshots"
$aiGenerated = Join-Path $PSScriptRoot "assets\ai-generated"
$coverDir = Join-Path $PSScriptRoot "assets\cover"
$outDir = Join-Path $PSScriptRoot "output"
$desktopDir = Join-Path ([Environment]::GetFolderPath("Desktop")) "ZSClip视频素材"
$out = Join-Path $outDir "zsclip-ai-competition-silent-draft.mp4"

New-Item -ItemType Directory -Force -Path $publicDir,$outDir,$desktopDir | Out-Null

$assets = @(
  @{ Source = Join-Path $recordings "03-desktop-non-overlap-demo.mp4"; Target = "desktop-demo.mp4" },
  @{ Source = Join-Path $recordings "05-feature-closeups-search-group-ocr.mp4"; Target = "feature-closeups.mp4" },
  @{ Source = Join-Path $recordings "04-dev-status-scroll.mp4"; Target = "dev-scroll.mp4" },
  @{ Source = Join-Path $screenshots "01-main-window.png"; Target = "main-window.png" },
  @{ Source = Join-Path $screenshots "06-task-manager-zsclip-memory.png"; Target = "task-manager-memory.png" },
  @{ Source = Join-Path $screenshots "07-codex-token-usage.png"; Target = "token-usage.png" },
  @{ Source = Join-Path $coverDir "bilibili-cover-zsclip.png"; Target = "bilibili-cover-zsclip.png" },
  @{ Source = Join-Path $aiGenerated "scene-python-rust-fragments.png"; Target = "scene-python-rust-fragments.png" },
  @{ Source = Join-Path $aiGenerated "scene-memory-minimal.png"; Target = "scene-memory-minimal.png" },
  @{ Source = Join-Path $aiGenerated "scene-zsui-open-engineering.png"; Target = "scene-zsui-open-engineering.png" }
)

foreach ($asset in $assets) {
  if (-not (Test-Path -LiteralPath $asset.Source)) {
    throw "缺少素材：$($asset.Source)"
  }
  Copy-Item -LiteralPath $asset.Source -Destination (Join-Path $publicDir $asset.Target) -Force
}

Push-Location $remotionDir
try {
  & npx.cmd remotion render src/index.ts ZsclipAiCompetition $out `
    --codec h264 `
    --pixel-format yuv420p `
    --crf 18

  if ($LASTEXITCODE -ne 0) {
    throw "Remotion render failed with exit code $LASTEXITCODE"
  }

  $videoOnlyOut = Join-Path $outDir "zsclip-ai-competition-silent-draft-video-only.mp4"
  & npx.cmd remotion ffmpeg `
    "-y" `
    "-i" $out `
    "-map" "0:v:0" `
    "-c:v" "copy" `
    "-an" `
    "-movflags" "+faststart" `
    $videoOnlyOut
  if ($LASTEXITCODE -ne 0) {
    throw "ffmpeg video-only remux failed with exit code $LASTEXITCODE"
  }
  if (Test-Path -LiteralPath $out) {
    Remove-Item -LiteralPath $out -Force
  }
  Move-Item -LiteralPath $videoOnlyOut -Destination $out -Force
} finally {
  Pop-Location
}

Copy-Item -LiteralPath $out -Destination (Join-Path $desktopDir "zsclip-ai-competition-silent-draft.mp4") -Force

Write-Output $out
Write-Output (Join-Path $desktopDir "zsclip-ai-competition-silent-draft.mp4")

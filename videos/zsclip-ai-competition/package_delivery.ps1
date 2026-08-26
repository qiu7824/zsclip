$ErrorActionPreference = "Stop"

$desktopDir = Join-Path ([Environment]::GetFolderPath("Desktop")) "ZSClip视频素材"
$packageRoot = Join-Path $PSScriptRoot "work\package\ZSClip视频素材包"
$zipPath = Join-Path $desktopDir "ZSClip视频素材包.zip"

New-Item -ItemType Directory -Force -Path $desktopDir | Out-Null
& (Join-Path $PSScriptRoot "generate_final_readiness.ps1") | Out-Null
if (Test-Path -LiteralPath $packageRoot) {
  Remove-Item -LiteralPath $packageRoot -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $packageRoot | Out-Null

function Copy-IntoPackage {
  param([string]$Source, [string]$RelativeDestination)
  if (-not (Test-Path -LiteralPath $Source)) {
    return
  }
  $destination = Join-Path $packageRoot $RelativeDestination
  New-Item -ItemType Directory -Force -Path (Split-Path -Parent $destination) | Out-Null
  Copy-Item -LiteralPath $Source -Destination $destination -Force
}

$rootFiles = @(
  "zsclip-ai-competition-silent-draft.mp4",
  "zsclip-ai-competition-voiceover-draft.mp4",
  "zsclip-ai-competition-final-with-bgm.mp4",
  "zsclip-soft-tech-bgm.wav",
  "bilibili-cover-zsclip.png",
  "review-handoff.html",
  "materials-audit.html",
  "final-readiness.html",
  "voiceover-teleprompter.html",
  "voiceover-final-2m11.md",
  "voiceover-input-README.md",
  "bilibili-release-copy.md",
  "materials-manifest.md",
  "delivery-qa-report.md",
  "delivery-qa-final-report.md",
  "START_HERE.md",
  "double_click_finalize.cmd",
  "02-desktop-search-vv-clean.mp4",
  "03-desktop-non-overlap-demo.mp4",
  "04-dev-status-scroll.mp4",
  "05-feature-closeups-search-group-ocr.mp4",
  "06-task-manager-zsclip-memory.png",
  "07-codex-token-usage.png",
  "05-roadmap-card.png"
)

foreach ($file in $rootFiles) {
  Copy-IntoPackage (Join-Path $desktopDir $file) $file
}

Copy-IntoPackage (Join-Path $desktopDir "captions\voiceover-captions.srt") "captions\voiceover-captions.srt"
Copy-IntoPackage (Join-Path $desktopDir "captions\voiceover-captions.json") "captions\voiceover-captions.json"
Copy-IntoPackage (Join-Path $desktopDir "voiceover-input\README.md") "voiceover-input\README.md"

$voiceoverPatterns = @("*.wav", "*.mp3", "*.m4a", "*.aac", "*.flac", "*.ogg")
$voiceoverSources = @(
  (Join-Path $PSScriptRoot "voiceover-input"),
  (Join-Path $desktopDir "voiceover-input"),
  $desktopDir
)
foreach ($dir in $voiceoverSources) {
  foreach ($pattern in $voiceoverPatterns) {
    Get-ChildItem -LiteralPath $dir -Filter $pattern -File -ErrorAction SilentlyContinue | ForEach-Object {
      Copy-IntoPackage $_.FullName (Join-Path "voiceover-input" $_.Name)
    }
  }
}

$scriptFiles = @(
  "START_HERE.md",
  "double_click_finalize.cmd",
  "generate_final_readiness.ps1",
  "generate_volcengine_voiceover.py",
  "generate_volcengine_voiceover.ps1",
  "generate_original_tech_bgm.py",
  "generate_bgm.ps1",
  "build_final_with_bgm.ps1",
  "build_with_voiceover.ps1",
  "finalize_after_voiceover.ps1",
  "watch_voiceover_and_finalize.ps1",
  "verify_delivery.ps1",
  "package_delivery.ps1"
)
foreach ($file in $scriptFiles) {
  Copy-IntoPackage (Join-Path $PSScriptRoot $file) (Join-Path "scripts" $file)
}

if (Test-Path -LiteralPath $zipPath) {
  Remove-Item -LiteralPath $zipPath -Force
}
Compress-Archive -Path (Join-Path $packageRoot "*") -DestinationPath $zipPath -Force

$item = Get-Item -LiteralPath $zipPath
Write-Output $item.FullName
Write-Output ("Package size: {0:N0} bytes" -f $item.Length)

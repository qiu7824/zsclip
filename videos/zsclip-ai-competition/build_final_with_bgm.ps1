param(
  [string]$VoiceoverPath = "",
  [string]$BgmPath = "",
  [string]$OutputPath = "",
  [switch]$AllowTrim
)

$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$ffmpegCwd = Join-Path $root "videos\zsclip-0.9-preview"
$outDir = Join-Path $PSScriptRoot "output"
$desktopDir = Join-Path ([Environment]::GetFolderPath("Desktop")) "ZSClip视频素材"
$silentVideo = Join-Path $outDir "zsclip-ai-competition-silent-draft.mp4"

if ([string]::IsNullOrWhiteSpace($VoiceoverPath)) {
  $VoiceoverPath = Join-Path $PSScriptRoot "voiceover-input\volcengine-voiceover.mp3"
}
if ([string]::IsNullOrWhiteSpace($BgmPath)) {
  $BgmPath = Join-Path $PSScriptRoot "assets\audio\zsclip-soft-tech-bgm.wav"
}
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
  $OutputPath = Join-Path $outDir "zsclip-ai-competition-final-with-bgm.mp4"
}

New-Item -ItemType Directory -Force -Path $outDir,$desktopDir | Out-Null

function Invoke-RemotionFfprobeDuration {
  param([string]$Path)
  Push-Location $ffmpegCwd
  try {
    $raw = & npx.cmd remotion ffprobe "-v" "error" "-show_entries" "format=duration" "-of" "default=noprint_wrappers=1:nokey=1" $Path
    if ($LASTEXITCODE -ne 0) {
      throw "ffprobe failed with exit code $LASTEXITCODE"
    }
    return [double]::Parse(($raw | Select-Object -First 1).Trim(), [System.Globalization.CultureInfo]::InvariantCulture)
  } finally {
    Pop-Location
  }
}

foreach ($required in @($silentVideo, $VoiceoverPath, $BgmPath)) {
  if (-not (Test-Path -LiteralPath $required)) {
    throw "缺少文件：$required"
  }
}

$videoDuration = Invoke-RemotionFfprobeDuration $silentVideo
$voiceDuration = Invoke-RemotionFfprobeDuration $VoiceoverPath
if ($voiceDuration -gt ($videoDuration + 0.25) -and -not $AllowTrim) {
  throw ("口播音频比视频长：音频 {0:N2}s，视频 {1:N2}s。请提高 TTS SpeechRate，或确认允许裁尾后加 -AllowTrim。" -f $voiceDuration,$videoDuration)
}

$durationText = $videoDuration.ToString("0.###", [System.Globalization.CultureInfo]::InvariantCulture)
Push-Location $ffmpegCwd
try {
  & npx.cmd remotion ffmpeg `
    "-y" `
    "-i" $silentVideo `
    "-i" $VoiceoverPath `
    "-i" $BgmPath `
    "-filter_complex" "[1:a]loudnorm=I=-16:TP=-1.5:LRA=11,aresample=48000,apad=pad_dur=300[vo];[2:a]volume=0.080,apad=pad_dur=300[bg];[vo][bg]amix=inputs=2:duration=first:dropout_transition=0[a]" `
    "-map" "0:v:0" `
    "-map" "[a]" `
    "-t" $durationText `
    "-c:v" "copy" `
    "-c:a" "aac" `
    "-b:a" "192k" `
    "-movflags" "+faststart" `
    $OutputPath
  if ($LASTEXITCODE -ne 0) {
    throw "ffmpeg failed with exit code $LASTEXITCODE"
  }
} finally {
  Pop-Location
}

$compatOut = Join-Path $outDir "zsclip-ai-competition-voiceover-draft.mp4"
Copy-Item -LiteralPath $OutputPath -Destination $compatOut -Force

$desktopFinal = Join-Path $desktopDir "zsclip-ai-competition-final-with-bgm.mp4"
$desktopCompat = Join-Path $desktopDir "zsclip-ai-competition-voiceover-draft.mp4"
Copy-Item -LiteralPath $OutputPath -Destination $desktopFinal -Force
Copy-Item -LiteralPath $compatOut -Destination $desktopCompat -Force

Write-Output ("Video duration: {0:N2} s" -f $videoDuration)
Write-Output ("Voiceover duration: {0:N2} s" -f $voiceDuration)
Write-Output $OutputPath
Write-Output $compatOut
Write-Output $desktopFinal
Write-Output $desktopCompat

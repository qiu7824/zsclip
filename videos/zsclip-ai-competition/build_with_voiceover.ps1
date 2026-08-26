param(
  [string]$VoiceoverPath = "",
  [string]$OutputPath = "",
  [switch]$AllowTrim,
  [switch]$NoDesktopCopy
)

$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$ffmpegCwd = Join-Path $root "videos\zsclip-0.9-preview"
$inputDir = Join-Path $PSScriptRoot "voiceover-input"
$workDir = Join-Path $PSScriptRoot "work\voiceover"
$outDir = Join-Path $PSScriptRoot "output"
$desktopDir = Join-Path ([Environment]::GetFolderPath("Desktop")) "ZSClip视频素材"
$desktopInputDir = Join-Path $desktopDir "voiceover-input"
$silentVideo = Join-Path $outDir "zsclip-ai-competition-silent-draft.mp4"
$normalizedAudio = Join-Path $workDir "voiceover-normalized.mp4"
$defaultOut = Join-Path $outDir "zsclip-ai-competition-voiceover-draft.mp4"
$outCandidate = if ([string]::IsNullOrWhiteSpace($OutputPath)) { $defaultOut } else { $OutputPath }
$out = [System.IO.Path]::GetFullPath($outCandidate)

New-Item -ItemType Directory -Force -Path $inputDir,$workDir,$outDir,$desktopDir,$desktopInputDir | Out-Null
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $out) | Out-Null

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

function Invoke-RemotionFfprobe {
  param([string[]]$FfprobeArgs)
  Push-Location $ffmpegCwd
  try {
    $result = & npx.cmd remotion ffprobe @FfprobeArgs
    if ($LASTEXITCODE -ne 0) {
      throw "ffprobe failed with exit code $LASTEXITCODE"
    }
    return $result
  } finally {
    Pop-Location
  }
}

function Get-MediaDurationSeconds {
  param([string]$Path)
  $raw = Invoke-RemotionFfprobe -FfprobeArgs @(
    "-v", "error",
    "-show_entries", "format=duration",
    "-of", "default=noprint_wrappers=1:nokey=1",
    $Path
  )
  $text = ($raw | Select-Object -First 1).Trim()
  return [double]::Parse($text, [System.Globalization.CultureInfo]::InvariantCulture)
}

if (-not (Test-Path -LiteralPath $silentVideo)) {
  throw "缺少无声草稿：$silentVideo。请先运行 build_silent_draft.ps1。"
}

if ([string]::IsNullOrWhiteSpace($VoiceoverPath)) {
  $supported = @("*.wav", "*.mp3", "*.m4a", "*.aac", "*.flac", "*.ogg")
  $searchDirs = @($inputDir, $desktopInputDir, $desktopDir)
  $candidates = foreach ($dir in $searchDirs) {
    foreach ($pattern in $supported) {
      Get-ChildItem -LiteralPath $dir -Filter $pattern -File -ErrorAction SilentlyContinue
    }
  }
  $voiceover = $candidates | Sort-Object LastWriteTime -Descending | Select-Object -First 1
  if ($null -eq $voiceover) {
    throw "请先把口播音频放到以下任一位置：$inputDir；$desktopInputDir；$desktopDir。推荐命名 voiceover.wav / voiceover.mp3 / voiceover.m4a。"
  }
  $VoiceoverPath = $voiceover.FullName
}

$VoiceoverPath = (Resolve-Path -LiteralPath $VoiceoverPath).Path
$videoDuration = Get-MediaDurationSeconds $silentVideo
$audioDuration = Get-MediaDurationSeconds $VoiceoverPath

if ($audioDuration -gt ($videoDuration + 0.25) -and -not $AllowTrim) {
  $videoText = "{0:N2}" -f $videoDuration
  $audioText = "{0:N2}" -f $audioDuration
  throw "口播音频比视频长：音频 ${audioText}s，视频 ${videoText}s。请重录短一点，或确认允许裁尾后加 -AllowTrim。"
}

Invoke-RemotionFfmpeg -FfmpegArgs @(
  "-y",
  "-i", $VoiceoverPath,
  "-vn",
  "-af", "loudnorm=I=-16:TP=-1.5:LRA=11,aresample=48000",
  "-ac", "2",
  "-c:a", "aac",
  "-b:a", "192k",
  $normalizedAudio
)

$durationText = $videoDuration.ToString("0.###", [System.Globalization.CultureInfo]::InvariantCulture)
Invoke-RemotionFfmpeg -FfmpegArgs @(
  "-y",
  "-i", $silentVideo,
  "-i", $normalizedAudio,
  "-filter_complex", "[1:a]apad=pad_dur=300[a]",
  "-map", "0:v:0",
  "-map", "[a]",
  "-t", $durationText,
  "-c:v", "copy",
  "-c:a", "aac",
  "-b:a", "192k",
  "-movflags", "+faststart",
  $out
)

$desktopOut = Join-Path $desktopDir "zsclip-ai-competition-voiceover-draft.mp4"
if (-not $NoDesktopCopy) {
  Copy-Item -LiteralPath $out -Destination $desktopOut -Force
}

Write-Output "Video duration: $durationText s"
Write-Output ("Audio duration: {0:N2} s" -f $audioDuration)
Write-Output $out
if (-not $NoDesktopCopy) {
  Write-Output $desktopOut
}

param(
  [string]$SpeakerId = "S_WgFVfXhO1",
  [string]$MaleSpeakerId = "S_WgFVfXhO1",
  [string]$FemaleSpeakerId = "zh_female_linxiao_uranus_bigtts",
  [string]$ResourceIds = "seed-icl-2.0,seed-icl-1.0,seed-tts-2.0,seed-tts-1.0",
  [string]$SpeakerResourceId = "",
  [string]$MaleResourceId = "seed-icl-2.0",
  [string]$FemaleResourceId = "seed-tts-2.0",
  [int]$SpeechRate = 18,
  [string]$ExplicitLanguage = "zh-cn",
  [string]$OutputPath = ""
)

$ErrorActionPreference = "Stop"

$apiKey = $env:VOLCENGINE_TTS_API_KEY
$appId = $env:VOLCENGINE_TTS_APP_ID
$accessKey = $env:VOLCENGINE_TTS_ACCESS_KEY
if ([string]::IsNullOrWhiteSpace($apiKey) -and ([string]::IsNullOrWhiteSpace($appId) -or [string]::IsNullOrWhiteSpace($accessKey))) {
  throw "请先设置 VOLCENGINE_TTS_API_KEY，或同时设置 VOLCENGINE_TTS_APP_ID / VOLCENGINE_TTS_ACCESS_KEY。"
}

$textFile = Join-Path $PSScriptRoot "captions\voiceover-captions.json"
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
  $OutputPath = Join-Path $PSScriptRoot "voiceover-input\volcengine-voiceover.mp3"
}
$root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$ffmpegCwd = Join-Path $root "videos\zsclip-0.9-preview"
$segmentsDir = Join-Path (Split-Path -Parent $OutputPath) "volcengine-voiceover-segments"

New-Item -ItemType Directory -Force -Path (Split-Path -Parent $OutputPath),$segmentsDir | Out-Null

python (Join-Path $PSScriptRoot "generate_volcengine_voiceover.py") `
  --speaker $SpeakerId `
  --male-speaker $MaleSpeakerId `
  --female-speaker $FemaleSpeakerId `
  --resource-ids $ResourceIds `
  --speaker-resource-id $SpeakerResourceId `
  --male-resource-id $MaleResourceId `
  --female-resource-id $FemaleResourceId `
  --speech-rate $SpeechRate `
  --explicit-language $ExplicitLanguage `
  --text-file $textFile `
  --output $OutputPath `
  --segments-dir $segmentsDir

if ($LASTEXITCODE -ne 0) {
  throw "火山 TTS 配音生成失败。"
}

$concatList = Join-Path $segmentsDir "concat-list.txt"
if (Test-Path -LiteralPath $concatList) {
  $concatOutputPath = Join-Path (Split-Path -Parent $OutputPath) "volcengine-voiceover-concat.mp3"
  Push-Location $ffmpegCwd
  try {
    & npx.cmd remotion ffmpeg `
      "-y" `
      "-f" "concat" `
      "-safe" "0" `
      "-i" $concatList `
      "-c" "copy" `
      $concatOutputPath
    if ($LASTEXITCODE -ne 0) {
      throw "ffmpeg concat failed with exit code $LASTEXITCODE"
    }
  } finally {
    Pop-Location
  }
  if (Test-Path -LiteralPath $OutputPath) {
    Remove-Item -LiteralPath $OutputPath -Force
  }
  Move-Item -LiteralPath $concatOutputPath -Destination $OutputPath -Force
}

$desktopDir = Join-Path ([Environment]::GetFolderPath("Desktop")) "ZSClip视频素材"
$desktopInputDir = Join-Path $desktopDir "voiceover-input"
New-Item -ItemType Directory -Force -Path $desktopInputDir | Out-Null
Copy-Item -LiteralPath $OutputPath -Destination (Join-Path $desktopInputDir "volcengine-voiceover.mp3") -Force

Write-Output $OutputPath
Write-Output (Join-Path $desktopInputDir "volcengine-voiceover.mp3")

param(
  [int]$TimeoutMinutes = 30,
  [int]$PollSeconds = 5
)

$ErrorActionPreference = "Stop"

$desktopDir = Join-Path ([Environment]::GetFolderPath("Desktop")) "ZSClip视频素材"
$voiceoverDirs = @(
  (Join-Path $PSScriptRoot "voiceover-input"),
  (Join-Path $desktopDir "voiceover-input"),
  $desktopDir
)
$patterns = @("*.wav", "*.mp3", "*.m4a", "*.aac", "*.flac", "*.ogg")
$deadline = (Get-Date).AddMinutes($TimeoutMinutes)

function Find-Voiceover {
  $files = @()
  foreach ($dir in $voiceoverDirs) {
    foreach ($pattern in $patterns) {
      $files += Get-ChildItem -LiteralPath $dir -Filter $pattern -File -ErrorAction SilentlyContinue
    }
  }
  $files | Sort-Object LastWriteTime -Descending | Select-Object -First 1
}

Write-Output "ZSClip 口播监听自动收口"
Write-Output "等待真实口播音频，超时：$TimeoutMinutes 分钟；轮询：$PollSeconds 秒。"
foreach ($dir in $voiceoverDirs) {
  Write-Output " - $dir"
}

while ((Get-Date) -lt $deadline) {
  $voiceover = Find-Voiceover
  if ($null -ne $voiceover) {
    Write-Output "检测到口播音频：$($voiceover.FullName)"
    & (Join-Path $PSScriptRoot "finalize_after_voiceover.ps1")
    if ($LASTEXITCODE -ne 0) {
      throw "最终合成流程失败"
    }
    & (Join-Path $PSScriptRoot "package_delivery.ps1")
    if ($LASTEXITCODE -ne 0) {
      throw "素材打包失败"
    }
    Write-Output "有声版合成、严格 QA 和素材打包已完成。"
    Write-Output (Join-Path $desktopDir "zsclip-ai-competition-voiceover-draft.mp4")
    Write-Output (Join-Path $desktopDir "ZSClip视频素材包.zip")
    exit 0
  }
  Write-Output ("{0:HH:mm:ss} 还没有检测到口播音频..." -f (Get-Date))
  Start-Sleep -Seconds $PollSeconds
}

Write-Output "等待超时，还没有检测到口播音频。"
Write-Output "请把 voiceover.wav / voiceover.mp3 / voiceover.m4a 放到上面任一目录后重试。"
exit 1

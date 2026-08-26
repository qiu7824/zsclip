$ErrorActionPreference = "Stop"

$desktopDir = Join-Path ([Environment]::GetFolderPath("Desktop")) "ZSClip视频素材"
$voiceoverDirs = @(
  (Join-Path $PSScriptRoot "voiceover-input"),
  (Join-Path $desktopDir "voiceover-input"),
  $desktopDir
)

Write-Output "ZSClip 视频最终合成流程"
Write-Output "正在查找真实口播音频..."
foreach ($dir in $voiceoverDirs) {
  Write-Output " - $dir"
}

try {
  & (Join-Path $PSScriptRoot "build_with_voiceover.ps1")
  if ($LASTEXITCODE -ne 0) {
    throw "有声版合成失败"
  }
} catch {
  Write-Output "还没有找到可用的真实口播音频，暂时不能生成有声版。"
  Write-Output $_.Exception.Message
  exit 1
}

Write-Output "有声版合成完成，开始严格 QA..."
try {
  & (Join-Path $PSScriptRoot "verify_delivery.ps1") -RequireVoiceover
  if ($LASTEXITCODE -ne 0) {
    throw "严格 QA 未通过"
  }
} catch {
  Write-Output "严格 QA 未通过，请查看桌面素材目录里的 delivery-qa-final-report.md。"
  Write-Output $_.Exception.Message
  exit 1
}

& (Join-Path $PSScriptRoot "generate_final_readiness.ps1") | Out-Null

Write-Output "最终有声版和 QA 报告已生成："
Write-Output (Join-Path $PSScriptRoot "output\zsclip-ai-competition-voiceover-draft.mp4")
Write-Output (Join-Path $desktopDir "zsclip-ai-competition-voiceover-draft.mp4")
Write-Output (Join-Path $desktopDir "delivery-qa-report.md")

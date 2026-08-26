param(
  [double]$DurationSeconds = 216,
  [string]$OutputPath = ""
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($OutputPath)) {
  $OutputPath = Join-Path $PSScriptRoot "assets\audio\zsclip-soft-tech-bgm.wav"
}

New-Item -ItemType Directory -Force -Path (Split-Path -Parent $OutputPath) | Out-Null

$duration = $DurationSeconds.ToString("0.###", [System.Globalization.CultureInfo]::InvariantCulture)
$generator = Join-Path $PSScriptRoot "generate_original_tech_bgm.py"
python $generator --duration $duration --output $OutputPath
if ($LASTEXITCODE -ne 0) {
  throw "原创科技感 BGM 生成失败。"
}

$desktopDir = Join-Path ([Environment]::GetFolderPath("Desktop")) "ZSClip视频素材"
New-Item -ItemType Directory -Force -Path $desktopDir | Out-Null
Copy-Item -LiteralPath $OutputPath -Destination (Join-Path $desktopDir "zsclip-soft-tech-bgm.wav") -Force

Write-Output $OutputPath
Write-Output (Join-Path $desktopDir "zsclip-soft-tech-bgm.wav")

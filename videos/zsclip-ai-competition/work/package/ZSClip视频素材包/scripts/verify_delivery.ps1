param(
  [switch]$RequireVoiceover
)

$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$ffmpegCwd = Join-Path $root "videos\zsclip-0.9-preview"
$desktopDir = Join-Path ([Environment]::GetFolderPath("Desktop")) "ZSClip视频素材"
$voiceoverInputDir = Join-Path $PSScriptRoot "voiceover-input"
$desktopVoiceoverInputDir = Join-Path $desktopDir "voiceover-input"
$outDir = Join-Path $PSScriptRoot "output"
$reportFileName = if ($RequireVoiceover) { "delivery-qa-final-report.md" } else { "delivery-qa-report.md" }
$reportPath = Join-Path $outDir $reportFileName
$desktopReportPath = Join-Path $desktopDir $reportFileName

New-Item -ItemType Directory -Force -Path $outDir,$desktopDir | Out-Null

$checks = New-Object System.Collections.Generic.List[object]

function Add-Check {
  param(
    [string]$Name,
    [string]$Status,
    [string]$Detail,
    [string]$Path = ""
  )
  $checks.Add([pscustomobject]@{
    Name = $Name
    Status = $Status
    Detail = $Detail
    Path = $Path
  })
}

function Test-RequiredFile {
  param([string]$Name, [string]$Path, [int64]$MinBytes = 1)
  if (-not (Test-Path -LiteralPath $Path)) {
    Add-Check $Name "FAIL" "文件不存在" $Path
    return
  }
  $item = Get-Item -LiteralPath $Path
  if ($item.Length -lt $MinBytes) {
    Add-Check $Name "FAIL" "文件过小：$($item.Length) bytes，预期至少 $MinBytes bytes" $Path
    return
  }
  Add-Check $Name "PASS" "存在，$($item.Length) bytes" $Path
}

function Copy-WithRetry {
  param([string]$Source, [string]$Destination)
  $lastError = $null
  for ($i = 1; $i -le 5; $i++) {
    try {
      Copy-Item -LiteralPath $Source -Destination $Destination -Force
      return
    } catch {
      $lastError = $_
      Start-Sleep -Milliseconds (200 * $i)
    }
  }
  throw $lastError
}

function Invoke-RemotionFfprobeJson {
  param([string]$Path)
  Push-Location $ffmpegCwd
  try {
    $raw = & npx.cmd remotion ffprobe `
      "-v" "error" `
      "-show_entries" "format=duration:stream=index,codec_type,codec_name,pix_fmt,width,height,avg_frame_rate,duration" `
      "-of" "json" `
      $Path
    if ($LASTEXITCODE -ne 0) {
      throw "ffprobe failed with exit code $LASTEXITCODE"
    }
    return ($raw -join "`n") | ConvertFrom-Json
  } finally {
    Pop-Location
  }
}

function Test-VideoSpec {
  param(
    [string]$Name,
    [string]$Path,
    [int]$ExpectedWidth,
    [int]$ExpectedHeight,
    [string]$ExpectedCodec,
    [string]$ExpectedPixFmt,
    [bool]$ExpectAudio
  )
  if (-not (Test-Path -LiteralPath $Path)) {
    Add-Check $Name "FAIL" "视频文件不存在，无法 ffprobe" $Path
    return
  }
  $probe = Invoke-RemotionFfprobeJson $Path
  $video = @($probe.streams | Where-Object { $_.codec_type -eq "video" } | Select-Object -First 1)
  $audio = @($probe.streams | Where-Object { $_.codec_type -eq "audio" })
  if ($video.Count -eq 0) {
    Add-Check $Name "FAIL" "没有视频流" $Path
    return
  }
  $v = $video[0]
  $problems = New-Object System.Collections.Generic.List[string]
  if ([int]$v.width -ne $ExpectedWidth) { $problems.Add("width=$($v.width)") }
  if ([int]$v.height -ne $ExpectedHeight) { $problems.Add("height=$($v.height)") }
  if ($v.codec_name -ne $ExpectedCodec) { $problems.Add("codec=$($v.codec_name)") }
  $actualPixFmt = [string]$v.pix_fmt
  $pixFmtCompatible = $ExpectedPixFmt -eq "yuv420p" -and $actualPixFmt -in @("yuv420p", "yuvj420p")
  if ($ExpectedPixFmt -and (-not $pixFmtCompatible) -and $actualPixFmt -ne $ExpectedPixFmt) { $problems.Add("pix_fmt=$($v.pix_fmt)") }
  if ($ExpectAudio -and $audio.Count -eq 0) { $problems.Add("缺少音频流") }
  if ((-not $ExpectAudio) -and $audio.Count -gt 0) { $problems.Add("无声草稿不应有音频流") }
  $duration = if ($probe.format.duration) { [double]::Parse($probe.format.duration, [System.Globalization.CultureInfo]::InvariantCulture) } else { 0 }
  if ($duration -lt 205 -or $duration -gt 220) { $problems.Add(("duration={0:N2}s" -f $duration)) }

  if ($problems.Count -gt 0) {
    Add-Check $Name "FAIL" ("规格不符：" + ($problems -join "；")) $Path
  } else {
    Add-Check $Name "PASS" ("{0}x{1} {2}/{3} {4} duration={5:N2}s audioStreams={6}" -f $v.width,$v.height,$v.codec_name,$v.pix_fmt,$v.avg_frame_rate,$duration,$audio.Count) $Path
  }
}

function Test-PngDimensions {
  param([string]$Name, [string]$Path, [int]$ExpectedWidth, [int]$ExpectedHeight)
  if (-not (Test-Path -LiteralPath $Path)) {
    Add-Check $Name "FAIL" "图片文件不存在" $Path
    return
  }
  Add-Type -AssemblyName System.Drawing
  $img = [System.Drawing.Image]::FromFile($Path)
  try {
    if ($img.Width -eq $ExpectedWidth -and $img.Height -eq $ExpectedHeight) {
      Add-Check $Name "PASS" "$($img.Width)x$($img.Height)" $Path
    } else {
      Add-Check $Name "FAIL" "尺寸为 $($img.Width)x$($img.Height)，预期 $ExpectedWidth x $ExpectedHeight" $Path
    }
  } finally {
    $img.Dispose()
  }
}

$repoRequired = @(
  @("无声草稿", "output\zsclip-ai-competition-silent-draft.mp4", 1000000),
  @("口播稿", "voiceover-final-2m11.md", 1000),
  @("口播提词器", "voiceover-teleprompter.html", 5000),
  @("起步说明", "START_HERE.md", 1000),
  @("双击最终收口入口", "double_click_finalize.cmd", 50),
  @("审片页", "review-handoff.html", 3000),
  @("素材录制核对页", "materials-audit.html", 5000),
  @("最终发布状态页", "final-readiness.html", 5000),
  @("素材清单", "materials-manifest.md", 1000),
  @("B 站发布文案", "bilibili-release-copy.md", 500),
  @("封面 PNG", "assets\cover\bilibili-cover-zsclip.png", 100000),
  @("SRT 字幕", "captions\voiceover-captions.srt", 1000),
  @("JSON 字幕", "captions\voiceover-captions.json", 1000),
  @("桌面主演示录屏", "assets\recordings\03-desktop-non-overlap-demo.mp4", 100000),
  @("搜索分组 OCR 特写", "assets\recordings\05-feature-closeups-search-group-ocr.mp4", 100000),
  @("开发状态滚动录屏", "assets\recordings\04-dev-status-scroll.mp4", 1000000),
  @("任务管理器内存截图", "assets\screenshots\06-task-manager-zsclip-memory.png", 10000),
  @("Codex token 截图", "assets\screenshots\07-codex-token-usage.png", 10000),
  @("后续更新卡片", "work\draft\cards\05-roadmap-card.png", 10000),
  @("火山 TTS Python 脚本", "generate_volcengine_voiceover.py", 1000),
  @("火山 TTS PowerShell 入口", "generate_volcengine_voiceover.ps1", 1000),
  @("BGM 生成入口脚本", "generate_bgm.ps1", 500),
  @("原创 BGM 合成脚本", "generate_original_tech_bgm.py", 1000),
  @("最终 BGM 合成脚本", "build_final_with_bgm.ps1", 1000),
  @("有声合成脚本", "build_with_voiceover.ps1", 1000),
  @("最终合成收口脚本", "finalize_after_voiceover.ps1", 500),
  @("口播监听自动收口脚本", "watch_voiceover_and_finalize.ps1", 1000),
  @("最终发布状态生成脚本", "generate_final_readiness.ps1", 1000),
  @("素材打包脚本", "package_delivery.ps1", 1000)
)

foreach ($entry in $repoRequired) {
  Test-RequiredFile $entry[0] (Join-Path $PSScriptRoot $entry[1]) ([int64]$entry[2])
}

$desktopRequired = @(
  "zsclip-ai-competition-silent-draft.mp4",
  "zsclip-ai-competition-final-with-bgm.mp4",
  "zsclip-soft-tech-bgm.wav",
  "voiceover-final-2m11.md",
  "voiceover-teleprompter.html",
  "START_HERE.md",
  "double_click_finalize.cmd",
  "review-handoff.html",
  "materials-audit.html",
  "final-readiness.html",
  "materials-manifest.md",
  "bilibili-release-copy.md",
  "bilibili-cover-zsclip.png",
  "03-desktop-non-overlap-demo.mp4",
  "05-feature-closeups-search-group-ocr.mp4",
  "04-dev-status-scroll.mp4",
  "06-task-manager-zsclip-memory.png",
  "07-codex-token-usage.png",
  "05-roadmap-card.png"
)

foreach ($file in $desktopRequired) {
  Test-RequiredFile "桌面副本：$file" (Join-Path $desktopDir $file) 1
}

Test-RequiredFile "桌面 SRT 字幕" (Join-Path $desktopDir "captions\voiceover-captions.srt") 1000
Test-RequiredFile "桌面 JSON 字幕" (Join-Path $desktopDir "captions\voiceover-captions.json") 1000
if (Test-Path -LiteralPath (Join-Path $desktopDir "ZSClip视频素材包.zip")) {
  Test-RequiredFile "桌面素材包 zip" (Join-Path $desktopDir "ZSClip视频素材包.zip") 1000000
} else {
  Add-Check "桌面素材包 zip" "PENDING" "尚未打包。可运行 package_delivery.ps1 生成。" (Join-Path $desktopDir "ZSClip视频素材包.zip")
}

Test-VideoSpec "无声草稿规格" (Join-Path $PSScriptRoot "output\zsclip-ai-competition-silent-draft.mp4") 1280 720 "h264" "yuv420p" $false
Test-PngDimensions "封面尺寸" (Join-Path $PSScriptRoot "assets\cover\bilibili-cover-zsclip.png") 1920 1080

$voiceoverPatterns = @("*.wav", "*.mp3", "*.m4a", "*.aac", "*.flac", "*.ogg")
$voiceoverSearchDirs = @($voiceoverInputDir, $desktopVoiceoverInputDir, $desktopDir)
$voiceoverFiles = foreach ($dir in $voiceoverSearchDirs) {
  foreach ($pattern in $voiceoverPatterns) {
    Get-ChildItem -LiteralPath $dir -Filter $pattern -File -ErrorAction SilentlyContinue
  }
}
if (@($voiceoverFiles).Count -eq 0) {
  if ($RequireVoiceover) {
    Add-Check "TTS / 口播音频" "FAIL" "严格模式要求 TTS / 口播音频。可放到仓库 voiceover-input 或桌面素材目录。" $voiceoverInputDir
  } else {
    Add-Check "TTS / 口播音频" "PENDING" "尚未提供。可放到仓库 voiceover-input 或桌面素材目录；提供后运行 build_final_with_bgm.ps1 生成有声版。" $voiceoverInputDir
  }
} else {
  $latest = @($voiceoverFiles | Sort-Object LastWriteTime -Descending | Select-Object -First 1)[0]
  Add-Check "TTS / 口播音频" "PASS" "检测到 $($latest.Name)，$($latest.Length) bytes" $latest.FullName
}

$voiceoverVideo = Join-Path $PSScriptRoot "output\zsclip-ai-competition-voiceover-draft.mp4"
$finalBgmVideo = Join-Path $PSScriptRoot "output\zsclip-ai-competition-final-with-bgm.mp4"
$desktopVoiceoverVideo = Join-Path $desktopDir "zsclip-ai-competition-voiceover-draft.mp4"
$desktopFinalBgmVideo = Join-Path $desktopDir "zsclip-ai-competition-final-with-bgm.mp4"
if (Test-Path -LiteralPath $voiceoverVideo) {
  Test-VideoSpec "有声版规格" $voiceoverVideo 1280 720 "h264" "yuv420p" $true
  Test-RequiredFile "桌面有声版副本" $desktopVoiceoverVideo 1000000
} elseif ($RequireVoiceover) {
  Add-Check "有声版输出" "FAIL" "严格模式要求有声版输出。请先运行 finalize_after_voiceover.ps1。" $voiceoverVideo
} else {
  Add-Check "有声版输出" "PENDING" "等待 TTS / 口播音频后生成。" $voiceoverVideo
}

if (Test-Path -LiteralPath (Join-Path $PSScriptRoot "assets\audio\zsclip-soft-tech-bgm.wav")) {
  Test-RequiredFile "BGM 音频" (Join-Path $PSScriptRoot "assets\audio\zsclip-soft-tech-bgm.wav") 100000
  Test-RequiredFile "桌面 BGM 副本" (Join-Path $desktopDir "zsclip-soft-tech-bgm.wav") 100000
} elseif ($RequireVoiceover) {
  Add-Check "BGM 音频" "FAIL" "严格模式要求 BGM。请先运行 generate_bgm.ps1。" (Join-Path $PSScriptRoot "assets\audio\zsclip-soft-tech-bgm.wav")
} else {
  Add-Check "BGM 音频" "PENDING" "等待最终配乐生成。" (Join-Path $PSScriptRoot "assets\audio\zsclip-soft-tech-bgm.wav")
}

if (Test-Path -LiteralPath $finalBgmVideo) {
  Test-VideoSpec "最终 BGM 有声版规格" $finalBgmVideo 1280 720 "h264" "yuv420p" $true
  Test-RequiredFile "桌面最终 BGM 有声版副本" $desktopFinalBgmVideo 1000000
} elseif ($RequireVoiceover) {
  Add-Check "最终 BGM 有声版输出" "FAIL" "严格模式要求最终带 BGM 视频。请先运行 build_final_with_bgm.ps1。" $finalBgmVideo
} else {
  Add-Check "最终 BGM 有声版输出" "PENDING" "等待 TTS 口播和 BGM 后生成。" $finalBgmVideo
}

$tinyScreenshots = @(Get-ChildItem -LiteralPath (Join-Path $PSScriptRoot "assets\screenshots") -File | Where-Object { $_.Length -lt 1024 })
if ($tinyScreenshots.Count -gt 0) {
  Add-Check "早期失败截图" "INFO" ("检测到未引用的小文件：" + (($tinyScreenshots | Select-Object -ExpandProperty Name) -join ", ")) (Join-Path $PSScriptRoot "assets\screenshots")
}

$lines = New-Object System.Collections.Generic.List[string]
$title = if ($RequireVoiceover) { "# ZSClip 视频最终严格 QA 报告" } else { "# ZSClip 视频交付 QA 报告" }
$lines.Add($title)
$lines.Add("")
$lines.Add(("生成时间：{0:yyyy-MM-dd HH:mm:ss}" -f (Get-Date)))
$lines.Add("")
$lines.Add("| 状态 | 检查项 | 说明 | 路径 |")
$lines.Add("|---|---|---|---|")
foreach ($check in $checks) {
  $detail = ($check.Detail -replace "\|", "\|")
  $path = ($check.Path -replace "\|", "\|")
  $pathCell = '`' + $path + '`'
  $lines.Add("| $($check.Status) | $($check.Name) | $detail | $pathCell |")
}

$summary = $checks | Group-Object Status | ForEach-Object { "$($_.Name): $($_.Count)" }
$lines.Add("")
$lines.Add("汇总：" + ($summary -join " / "))

$lines | Set-Content -Path $reportPath -Encoding utf8
Copy-WithRetry $reportPath $desktopReportPath

$failCount = @($checks | Where-Object { $_.Status -eq "FAIL" }).Count
Write-Output $reportPath
Write-Output $desktopReportPath
Write-Output ("QA summary: " + ($summary -join " / "))
if ($failCount -gt 0) {
  exit 1
}

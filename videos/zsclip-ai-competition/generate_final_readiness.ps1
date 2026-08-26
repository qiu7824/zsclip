$ErrorActionPreference = "Stop"

$desktopDir = Join-Path ([Environment]::GetFolderPath("Desktop")) "ZSClip视频素材"
$outPath = Join-Path $PSScriptRoot "final-readiness.html"
$desktopOutPath = Join-Path $desktopDir "final-readiness.html"
$voiceoverDirs = @(
  (Join-Path $PSScriptRoot "voiceover-input"),
  (Join-Path $desktopDir "voiceover-input"),
  $desktopDir
)
$voiceoverPatterns = @("*.wav", "*.mp3", "*.m4a", "*.aac", "*.flac", "*.ogg")

New-Item -ItemType Directory -Force -Path $desktopDir | Out-Null

function ConvertTo-HtmlText {
  param([string]$Text)
  return [System.Net.WebUtility]::HtmlEncode($Text)
}

function Test-FileState {
  param([string]$Path, [int64]$MinBytes = 1)
  if (-not (Test-Path -LiteralPath $Path)) {
    return [pscustomobject]@{ Exists = $false; Length = 0; Path = $Path }
  }
  $item = Get-Item -LiteralPath $Path
  return [pscustomobject]@{ Exists = ($item.Length -ge $MinBytes); Length = $item.Length; Path = $item.FullName }
}

function Find-LatestVoiceover {
  $files = @()
  foreach ($dir in $voiceoverDirs) {
    foreach ($pattern in $voiceoverPatterns) {
      $files += Get-ChildItem -LiteralPath $dir -Filter $pattern -File -ErrorAction SilentlyContinue
    }
  }
  $files | Sort-Object LastWriteTime -Descending | Select-Object -First 1
}

function Get-ReportSummary {
  param([string]$Path)
  if (-not (Test-Path -LiteralPath $Path)) {
    return "未生成"
  }
  $summary = Get-Content -LiteralPath $Path | Where-Object { $_ -like "汇总：*" } | Select-Object -Last 1
  if ([string]::IsNullOrWhiteSpace($summary)) {
    return "已生成，未读取到汇总"
  }
  return $summary
}

$voiceover = Find-LatestVoiceover
$silentVideo = Test-FileState (Join-Path $PSScriptRoot "output\zsclip-ai-competition-silent-draft.mp4") 1000000
$voiceoverVideo = Test-FileState (Join-Path $PSScriptRoot "output\zsclip-ai-competition-voiceover-draft.mp4") 1000000
$desktopVoiceoverVideo = Test-FileState (Join-Path $desktopDir "zsclip-ai-competition-voiceover-draft.mp4") 1000000
$cover = Test-FileState (Join-Path $desktopDir "bilibili-cover-zsclip.png") 100000
$releaseCopy = Test-FileState (Join-Path $desktopDir "bilibili-release-copy.md") 500
$qaReport = Test-FileState (Join-Path $desktopDir "delivery-qa-report.md") 500
$finalQaReport = Test-FileState (Join-Path $desktopDir "delivery-qa-final-report.md") 500
$package = Test-FileState (Join-Path $desktopDir "ZSClip视频素材包.zip") 1000000

$voiceoverReady = $null -ne $voiceover
$finalVideoReady = $voiceoverVideo.Exists -and $desktopVoiceoverVideo.Exists
$publishReady = $voiceoverReady -and $finalVideoReady -and $cover.Exists -and $releaseCopy.Exists -and $package.Exists

$statusText = if ($publishReady) {
  "可以发布"
} elseif ($voiceoverReady) {
  "已检测到口播，等待或需要重新生成有声版"
} else {
  "等待 TTS 或口播音频"
}
$statusClass = if ($publishReady) { "ok" } elseif ($voiceoverReady) { "warn" } else { "wait" }
$voiceoverName = if ($voiceoverReady) { $voiceover.Name } else { "未检测到" }
$voiceoverPath = if ($voiceoverReady) { $voiceover.FullName } else { "请放入 voiceover.wav / voiceover.mp3 / voiceover.m4a" }
$voiceoverSize = if ($voiceoverReady) { "{0:N0} bytes" -f $voiceover.Length } else { "-" }

$checks = @(
  [pscustomobject]@{ Name = "无声草稿"; Status = $silentVideo.Exists; Detail = "剪辑底片，3:36"; Link = "zsclip-ai-competition-silent-draft.mp4" },
  [pscustomobject]@{ Name = "TTS / 口播音频"; Status = $voiceoverReady; Detail = "$voiceoverName / $voiceoverSize"; Link = "voiceover-input-README.md" },
  [pscustomobject]@{ Name = "最终有声版"; Status = $finalVideoReady; Detail = "生成后用于上传 B 站"; Link = $(if ($finalVideoReady) { "zsclip-ai-competition-voiceover-draft.mp4" } else { "voiceover-input-README.md" }) },
  [pscustomobject]@{ Name = "封面"; Status = $cover.Exists; Detail = "1920x1080 B 站封面"; Link = "bilibili-cover-zsclip.png" },
  [pscustomobject]@{ Name = "发布文案"; Status = $releaseCopy.Exists; Detail = "标题、简介、标签、章节"; Link = "bilibili-release-copy.md" },
  [pscustomobject]@{ Name = "普通 QA"; Status = $qaReport.Exists; Detail = (Get-ReportSummary $qaReport.Path); Link = "delivery-qa-report.md" },
  [pscustomobject]@{ Name = "最终严格 QA"; Status = ($publishReady -and $finalQaReport.Exists); Detail = (Get-ReportSummary $finalQaReport.Path); Link = "delivery-qa-final-report.md" },
  [pscustomobject]@{ Name = "素材包"; Status = $package.Exists; Detail = "{0:N0} bytes" -f $package.Length; Link = "ZSClip视频素材包.zip" }
)

$rows = foreach ($check in $checks) {
  $label = if ($check.Status) { "已就绪" } else { "待完成" }
  $klass = if ($check.Status) { "ok" } else { "wait" }
  "<tr><td><span class=""pill $klass"">$label</span></td><td>$(ConvertTo-HtmlText $check.Name)</td><td>$(ConvertTo-HtmlText $check.Detail)</td><td><a href=""$(ConvertTo-HtmlText $check.Link)"">打开</a></td></tr>"
}

$html = @"
<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>ZSClip 最终发布状态</title>
  <style>
    :root {
      color-scheme: light;
      --bg: #f6f8fb;
      --panel: #fff;
      --ink: #182033;
      --muted: #647084;
      --line: #dce3ee;
      --ok: #0f7a4f;
      --ok-bg: #e7f7ee;
      --wait: #a15d00;
      --wait-bg: #fff2d9;
      --warn: #9a4d00;
      --warn-bg: #fff0df;
      --accent: #1665d8;
      --soft: #eef2f7;
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      background: var(--bg);
      color: var(--ink);
      font-family: "Microsoft YaHei UI", "Segoe UI", Arial, sans-serif;
      line-height: 1.65;
    }
    header {
      background: #fff;
      border-bottom: 1px solid var(--line);
      padding: 34px 38px 24px;
    }
    main { max-width: 1080px; margin: 0 auto; padding: 24px 20px 48px; }
    section {
      background: var(--panel);
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 20px;
      margin-bottom: 18px;
    }
    h1, h2, h3 { margin: 0; line-height: 1.25; }
    h1 { font-size: 30px; }
    h2 { font-size: 22px; margin-bottom: 12px; }
    p { margin: 0 0 10px; color: var(--muted); }
    table { width: 100%; border-collapse: collapse; font-size: 14px; }
    th, td {
      border-bottom: 1px solid var(--line);
      padding: 10px 8px;
      text-align: left;
      vertical-align: top;
    }
    th { background: var(--soft); color: #33405a; }
    tr:last-child td { border-bottom: 0; }
    .hero-status {
      display: inline-flex;
      align-items: center;
      border-radius: 999px;
      font-weight: 700;
      padding: 7px 12px;
      margin-top: 12px;
    }
    .hero-status.ok, .pill.ok { color: var(--ok); background: var(--ok-bg); }
    .hero-status.wait, .pill.wait { color: var(--wait); background: var(--wait-bg); }
    .hero-status.warn, .pill.warn { color: var(--warn); background: var(--warn-bg); }
    .pill {
      display: inline-block;
      border-radius: 999px;
      font-weight: 700;
      font-size: 12px;
      line-height: 1;
      padding: 5px 8px;
      white-space: nowrap;
    }
    .grid {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 14px;
    }
    .card {
      border: 1px solid var(--line);
      border-radius: 8px;
      background: #fff;
      padding: 14px;
    }
    video, img {
      width: 100%;
      display: block;
      margin-top: 10px;
      border: 1px solid var(--line);
      border-radius: 6px;
      background: #111827;
    }
    img { background: #fff; }
    code {
      background: var(--soft);
      border-radius: 4px;
      padding: 2px 5px;
      font-family: "Cascadia Code", Consolas, monospace;
      font-size: 0.92em;
    }
    pre {
      background: var(--soft);
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 12px;
      white-space: pre-wrap;
      overflow-wrap: anywhere;
      font-family: "Cascadia Code", Consolas, monospace;
      font-size: 13px;
    }
    a { color: var(--accent); text-decoration: none; }
    a:hover { text-decoration: underline; }
    ul { color: var(--muted); margin: 8px 0 0 20px; padding: 0; }
    li { margin: 4px 0; }
    @media (max-width: 760px) {
      header { padding: 24px 18px 18px; }
      main { padding: 18px 12px 36px; }
      .grid { grid-template-columns: 1fr; }
    }
  </style>
</head>
<body>
  <header>
    <h1>ZSClip 最终发布状态</h1>
    <p>生成时间：$(Get-Date -Format "yyyy-MM-dd HH:mm:ss")</p>
    <p>这页只回答一个问题：现在能不能拿去发布。它会随着打包脚本自动刷新。</p>
    <span class="hero-status $statusClass">$statusText</span>
  </header>
  <main>
    <section>
      <h2>发布检查</h2>
      <table>
        <thead>
          <tr>
            <th>状态</th>
            <th>项目</th>
            <th>说明</th>
            <th>文件</th>
          </tr>
        </thead>
        <tbody>
          $($rows -join "`n          ")
        </tbody>
      </table>
    </section>

    <section>
      <h2>口播音频</h2>
      <p>当前检测：<code>$(ConvertTo-HtmlText $voiceoverPath)</code></p>
      <p>推荐放置位置：</p>
      <pre>C:\Users\xs\Desktop\ZSClip视频素材\voiceover-input
E:\rust\zsclip\videos\zsclip-ai-competition\voiceover-input</pre>
      <p>录完口播后，双击桌面素材目录里的 <code>double_click_finalize.cmd</code>，它会自动合成有声版、严格 QA、重新打包。</p>
    </section>

    <section>
      <h2>快速预览</h2>
      <div class="grid">
        <div class="card">
          <h3>无声草稿</h3>
          <video controls preload="metadata" src="zsclip-ai-competition-silent-draft.mp4"></video>
        </div>
        <div class="card">
          <h3>B 站封面</h3>
          <img src="bilibili-cover-zsclip.png" alt="ZSClip B站封面">
        </div>
      </div>
    </section>

    <section>
      <h2>发布前动作</h2>
      <ul>
        <li>打开 <a href="voiceover-teleprompter.html">口播提词器</a> 检查双人口播稿；口播约 3:30，画面保留到 3:36 完整收束。</li>
        <li>运行 <code>build_final_with_bgm.ps1</code> 生成 <code>zsclip-ai-competition-final-with-bgm.mp4</code>。</li>
        <li>严格 QA 通过后，使用 <a href="bilibili-release-copy.md">B 站发布文案</a> 和 <a href="bilibili-cover-zsclip.png">封面</a> 发布。</li>
      </ul>
    </section>
  </main>
</body>
</html>
"@

$html | Set-Content -LiteralPath $outPath -Encoding utf8
Copy-Item -LiteralPath $outPath -Destination $desktopOutPath -Force

Write-Output $outPath
Write-Output $desktopOutPath
Write-Output $statusText

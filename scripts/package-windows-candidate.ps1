param([string]$Version = (Get-Content "$PSScriptRoot\..\VERSION.txt" -Raw).Trim())
$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path "$PSScriptRoot\..").Path
Push-Location $projectRoot
try {
    $destination = Join-Path $projectRoot "dist\$Version-candidate"
    New-Item -ItemType Directory -Force $destination | Out-Null
    New-Item -ItemType Directory -Force (Join-Path $projectRoot 'verification') | Out-Null
    foreach ($variant in @('standard','no-lan')) {
        $buildArgs = @('build','--release','--locked','--offline','--bin','zsclip')
        if ($variant -eq 'no-lan') {
            $buildArgs += @('--no-default-features','--features','vv-paste,cloud-sync,mail-merge,ai-actions,sticker')
        }
        & cargo @buildArgs *> "verification\build-$variant.log"
        if ($LASTEXITCODE -ne 0) { throw "Build failed: $variant. See verification/build-$variant.log" }
        $stage = Join-Path $destination $variant
        New-Item -ItemType Directory -Force $stage | Out-Null
        Copy-Item -LiteralPath 'target\release\zsclip.exe' -Destination (Join-Path $stage '剪贴板.exe') -Force
        $releaseNotes = [System.IO.File]::ReadAllText((Join-Path $projectRoot "docs\releases\$Version.md"))
        $releaseNotes = $releaseNotes.Replace("../verification/$Version.md", 'VERIFICATION.md')
        [System.IO.File]::WriteAllText((Join-Path $stage 'RELEASE-NOTES.md'),$releaseNotes,[System.Text.UTF8Encoding]::new($false))
        Copy-Item -LiteralPath "docs\verification\$Version.md" -Destination (Join-Path $stage 'VERIFICATION.md') -Force
        $instructions = "ZSClip $Version ($variant) - local review candidate`r`nExtract into a NEW directory. Existing history is not included.`r`nStop the old application before launching this copy. Preserve/back up your existing data directory when updating.`r`nThis archive has not been published to an update channel.`r`n"
        [System.IO.File]::WriteAllText((Join-Path $stage 'INSTALL.txt'),$instructions,[System.Text.UTF8Encoding]::new($false))
        $name = if ($variant -eq 'standard') { "zsclip-$Version-windows-x86_64-portable.zip" } else { "zsclip-$Version-windows-x86_64-no-lan-portable.zip" }
        Compress-Archive -LiteralPath (Join-Path $stage '剪贴板.exe'),(Join-Path $stage 'RELEASE-NOTES.md'),(Join-Path $stage 'VERIFICATION.md'),(Join-Path $stage 'INSTALL.txt') -DestinationPath (Join-Path $destination $name) -Force
        Write-Output "Built $name"
    }
    $hashes = Get-ChildItem -LiteralPath $destination -File -Filter '*.zip' | Sort-Object Name | ForEach-Object {
        "{0}  {1}" -f (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant(),$_.Name
    }
    [System.IO.File]::WriteAllLines((Join-Path $destination 'SHA256SUMS.txt'),$hashes,[System.Text.UTF8Encoding]::new($false))
    Write-Output "Candidates: $destination"
} finally { Pop-Location }

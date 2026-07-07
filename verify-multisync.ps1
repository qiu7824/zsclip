param(
    [switch]$SkipRust,
    [switch]$SkipAndroid,
    [switch]$SkipAssemble,
    [switch]$SkipReleaseHash,
    [switch]$RunDeviceSmoke,
    [string]$SmokeHost = "192.168.1.10:38473"
)

$ErrorActionPreference = "Stop"

function Invoke-Step {
    param(
        [string]$Name,
        [scriptblock]$Block
    )
    Write-Host "==> $Name"
    & $Block
    Write-Host "OK: $Name"
}

function Invoke-Native {
    param(
        [string]$FilePath,
        [string[]]$Arguments = @()
    )
    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$FilePath failed with exit code $LASTEXITCODE"
    }
}

function Assert-FileContains {
    param(
        [string]$Path,
        [string]$Needle
    )
    $text = [System.IO.File]::ReadAllText((Resolve-Path -LiteralPath $Path), [System.Text.Encoding]::UTF8)
    if (-not $text.Contains($Needle)) {
        throw "Missing expected text in ${Path}: ${Needle}"
    }
}

function Assert-NoMojibake {
    param([string[]]$Roots)
    $markers = @(0x951f, 0xfffd, 0x6d93, 0x9365, 0x6f76, 0x93ba, 0x7487, 0x9422, 0x9225, 0x6769, 0x704f, 0x58c0, 0x7ed4) |
        ForEach-Object { [char]$_ }
    $extensions = @(".rs", ".md", ".kt", ".xml", ".json", ".gradle", ".ps1")
    $hits = New-Object System.Collections.Generic.List[string]
    foreach ($root in $Roots) {
        if (-not (Test-Path -LiteralPath $root)) {
            continue
        }
        $item = Get-Item -LiteralPath $root
        $files = if ($item.PSIsContainer) {
            Get-ChildItem -LiteralPath $root -Recurse -File | Where-Object { $extensions -contains $_.Extension }
        } else {
            @($item)
        }
        foreach ($file in $files) {
            $text = [System.IO.File]::ReadAllText($file.FullName, [System.Text.Encoding]::UTF8)
            foreach ($marker in $markers) {
                if ($text.Contains($marker)) {
                    $hits.Add($file.FullName)
                    break
                }
            }
        }
    }
    if ($hits.Count -gt 0) {
        throw "Possible mojibake markers found:`n$($hits -join "`n")"
    }
}

function Assert-IosShortcutContract {
    $configPath = "docs/ios-shortcuts-config.example.json"
    $docsPath = "docs/ios-shortcuts.md"
    $configText = [System.IO.File]::ReadAllText((Resolve-Path -LiteralPath $configPath), [System.Text.Encoding]::UTF8)
    $docsText = [System.IO.File]::ReadAllText((Resolve-Path -LiteralPath $docsPath), [System.Text.Encoding]::UTF8)
    $config = $configText | ConvertFrom-Json

    foreach ($property in @("host", "device_id", "device_name", "token", "setup_url", "manifest_url", "images_url")) {
        if (-not ($config.PSObject.Properties.Name -contains $property)) {
            throw "Missing iOS Shortcut config property: $property"
        }
    }

    if (-not $config.setup_url.EndsWith("/mobile/setup")) {
        throw "iOS setup_url should point to /mobile/setup"
    }
    if (-not $config.manifest_url.Contains("/SyncClipboard.json?device=")) {
        throw "iOS manifest_url should point to SyncClipboard.json with device query"
    }
    if (-not $config.images_url.Contains("/mobile/images?device=")) {
        throw "iOS images_url should point to /mobile/images with device query"
    }

    foreach ($needle in @(
            "## iOS ",
            "http://<host>/mobile/setup",
            "manifest_url",
            "images_url",
            "ZSCLIP_MULTI_SYNC_V1",
            "dataName",
            "/mobile/images"
        )) {
        if (-not $docsText.Contains($needle)) {
            throw "Missing expected iOS Shortcut docs text: $needle"
        }
    }
}

function Assert-AndroidSmokeDryRun {
    $before = (Get-Location).Path
    $output = & ".\mobile\android\smoke-adb.ps1" -HostAddress $SmokeHost -SkipBuild -DryRun 2>&1
    if ($null -ne $LASTEXITCODE -and $LASTEXITCODE -ne 0) {
        throw "Android smoke dry-run failed with exit code $LASTEXITCODE"
    }
    $after = (Get-Location).Path
    if ($before -ne $after) {
        throw "Android smoke dry-run changed current directory. before=$before after=$after"
    }
    $text = $output -join "`n"
    foreach ($needle in @(
            "DRY-RUN adb devices",
            "DRY-RUN adb install -r app/build/outputs/apk/debug/app-debug.apk",
            "DRY-RUN adb shell am start -n com.zsclip.lan/.MainActivity",
            "android.intent.action.VIEW",
            "zsclip://pair?host=192.168.1.10%3A38473",
            "android.intent.action.SEND",
            "android.intent.extra.TEXT hello from adb",
            "ZSClip Android smoke test dry-run completed."
        )) {
        if (-not $text.Contains($needle)) {
            throw "Missing expected Android smoke dry-run output: $needle"
        }
    }
    Write-Host $text
}

function Assert-AndroidReleaseContract {
    $readmePath = "release/0.9.9.4/android/README.md"
    $apkPath = "release/0.9.9.4/android/zsclip-lan-debug.apk"
    if (-not (Test-Path -LiteralPath $readmePath)) {
        throw "Missing Android release README: $readmePath"
    }
    if (-not (Test-Path -LiteralPath $apkPath)) {
        throw "Missing Android release APK: $apkPath"
    }
    $apk = Get-Item -LiteralPath $apkPath
    if ($apk.Length -le 0) {
        throw "Android release APK is empty: $apkPath"
    }
    $apkHash = (Get-FileHash -LiteralPath $apkPath -Algorithm SHA256).Hash
    foreach ($needle in @(
            'ZSClip Android Debug APK',
            'com.zsclip.lan',
            'Version: `0.9.9.4`',
            'Version code: `9094`',
            'zsclip-lan-debug.apk',
            "SHA256: ``$apkHash``",
            '.\verify-multisync.ps1',
            '.\smoke-adb.ps1 -HostAddress "192.168.1.10:38473" -SkipBuild -DryRun',
            'Full completion still requires manual Android + Windows LAN + WebDAV + iOS Shortcuts end-to-end verification.'
        )) {
        Assert-FileContains $readmePath $needle
    }
}

function Assert-WpsTaskPaneContract {
    $root = "integrations/wps-taskpane"
    $ribbonPath = Join-Path $root "ribbon.xml"
    $mainPath = Join-Path $root "main.js"
    $indexPath = Join-Path $root "index.html"
    $packagePath = Join-Path $root "package.json"
    $readmePath = Join-Path $root "README.md"
    $docsPath = "docs/wps-taskpane.md"
    foreach ($path in @($ribbonPath, $mainPath, $indexPath, $packagePath, $readmePath, $docsPath)) {
        if (-not (Test-Path -LiteralPath $path)) {
            throw "Missing WPS task pane file: $path"
        }
    }
    foreach ($needle in @(
            'zsclipAddinTab',
            'btnOpenZsclipTaskPane',
            'OnAction',
            'screentip='
        )) {
        Assert-FileContains $ribbonPath $needle
    }
    foreach ($needle in @(
            'http://127.0.0.1:38473/office/wps/taskpane',
            'OnOpenZsclipTaskPane',
            'CreateTaskPane',
            'CreateTaskpane',
            'GetTaskPane',
            'ZSClipWpsTaskPane'
        )) {
        Assert-FileContains $mainPath $needle
    }
    Assert-FileContains $indexPath './main.js'
    Assert-FileContains $packagePath '"addonType": "wps"'
    foreach ($needle in @(
            '/office/wps/items',
            'Selection.TypeText',
            'loopback',
            '.\verify-multisync.ps1'
        )) {
        Assert-FileContains $docsPath $needle
    }
}

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $root

Invoke-Step "UTF-8 source and docs scan" {
    Assert-NoMojibake @(
        "src",
        "docs",
        "README.md",
        "README.en.md",
        "mobile/android/app/src/main",
        "mobile/android/app/src/test",
        "mobile/android/smoke-adb.ps1",
        "integrations/wps-taskpane",
        "release/0.9.9.4/android/README.md"
    )
}

Invoke-Step "WPS task pane contract" {
    Assert-WpsTaskPaneContract
}

Invoke-Step "iOS Shortcut docs contract" {
    Assert-IosShortcutContract
}

if (-not $SkipRust) {
    Invoke-Step "Rust tests" {
        Invoke-Native "cargo" @("test", "-j", "1")
    }
}

if (-not $SkipAndroid) {
    Push-Location "mobile/android"
    try {
        Invoke-Step "Android unit tests" {
            Invoke-Native "gradle" @("test")
        }
        if (-not $SkipAssemble) {
            Invoke-Step "Android debug APK build" {
                Invoke-Native "gradle" @("assembleDebug")
            }
        }
    } finally {
        Pop-Location
    }
}

Invoke-Step "Android APK metadata" {
    $metadataPath = "mobile/android/app/build/outputs/apk/debug/output-metadata.json"
    Assert-FileContains $metadataPath '"applicationId": "com.zsclip.lan"'
    Assert-FileContains $metadataPath '"versionCode": 9094'
    Assert-FileContains $metadataPath '"versionName": "0.9.9.4"'
}

Invoke-Step "Android smoke command dry-run" {
    Assert-AndroidSmokeDryRun
}

Invoke-Step "Android release package contract" {
    Assert-AndroidReleaseContract
}

if (-not $SkipReleaseHash) {
    Invoke-Step "Release APK matches current build" {
        $buildApk = "mobile/android/app/build/outputs/apk/debug/app-debug.apk"
        $releaseApk = "release/0.9.9.4/android/zsclip-lan-debug.apk"
        if (-not (Test-Path -LiteralPath $buildApk)) {
            throw "Missing build APK: $buildApk"
        }
        if (-not (Test-Path -LiteralPath $releaseApk)) {
            throw "Missing release APK: $releaseApk"
        }
        $buildHash = (Get-FileHash -LiteralPath $buildApk -Algorithm SHA256).Hash
        $releaseHash = (Get-FileHash -LiteralPath $releaseApk -Algorithm SHA256).Hash
        if ($buildHash -ne $releaseHash) {
            throw "Release APK hash does not match build APK. build=$buildHash release=$releaseHash"
        }
        Write-Host "APK SHA256: $buildHash"
    }
}

if ($RunDeviceSmoke) {
    Invoke-Step "Android device smoke" {
        & ".\mobile\android\smoke-adb.ps1" -HostAddress $SmokeHost -SkipBuild
        if ($LASTEXITCODE -ne 0) {
            throw "Android device smoke failed with exit code $LASTEXITCODE"
        }
    }
}

Write-Host "ZSClip multi-sync verification passed."

param(
    [string]$HostAddress = "192.168.1.10:38473",
    [string]$ShareText = "hello from adb",
    [string]$ApkPath = "app/build/outputs/apk/debug/app-debug.apk",
    [switch]$SkipBuild,
    [switch]$CheckAutoSync,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

function Invoke-Adb {
    param([string[]]$Arguments)
    if ($DryRun) {
        Write-Output "DRY-RUN adb $($Arguments -join ' ')"
        return
    }
    & $script:AdbPath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "adb failed: $($Arguments -join ' ')"
    }
}

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Push-Location $scriptDir
try {

    if (-not $DryRun) {
        $adb = Get-Command adb -ErrorAction SilentlyContinue
        if ($null -eq $adb) {
            $sdkRoot = if ($env:ANDROID_SDK_ROOT) { $env:ANDROID_SDK_ROOT } else { $env:ANDROID_HOME }
            if ($sdkRoot) {
                $sdkAdb = Join-Path $sdkRoot "platform-tools\adb.exe"
                if (Test-Path -LiteralPath $sdkAdb) {
                    $adb = [pscustomobject]@{ Source = $sdkAdb }
                }
            }
        }
        if ($null -eq $adb) {
            throw "adb was not found in PATH or ANDROID_SDK_ROOT. Install Android Platform Tools and enable USB debugging."
        }
        $script:AdbPath = $adb.Source
    }

    if (-not $SkipBuild) {
        if ($DryRun) {
            Write-Output "DRY-RUN gradle assembleDebug"
        } else {
            gradle assembleDebug
            if ($LASTEXITCODE -ne 0) {
                throw "gradle assembleDebug failed"
            }
        }
    }

    if ($DryRun) {
        $resolvedApk = [pscustomobject]@{ Path = $ApkPath }
    } else {
        $resolvedApk = Resolve-Path -LiteralPath $ApkPath -ErrorAction SilentlyContinue
        if ($null -eq $resolvedApk) {
            throw "APK not found: $ApkPath"
        }
    }

    if ($DryRun) {
        Write-Output "DRY-RUN adb devices"
    } else {
        $deviceLines = & $script:AdbPath devices
        $onlineDevices = @($deviceLines | Where-Object { $_ -match "`tdevice$" })
        if ($onlineDevices.Count -eq 0) {
            throw "No online Android device found. Check USB debugging and run 'adb devices'."
        }
    }

    Invoke-Adb @("install", "-r", $resolvedApk.Path)
    Invoke-Adb @("shell", "am", "start", "-n", "com.zsclip.lan/.MainActivity")

    $encodedHost = [uri]::EscapeDataString($HostAddress)
    Invoke-Adb @(
        "shell", "am", "start",
        "-a", "android.intent.action.VIEW",
        "-d", "zsclip://pair?host=$encodedHost"
    )

    Invoke-Adb @(
        "shell", "am", "start",
        "-n", "com.zsclip.lan/.MainActivity",
        "-a", "android.intent.action.SEND",
        "-t", "text/plain",
        "--es", "android.intent.extra.TEXT", $ShareText
    )

    if ($CheckAutoSync) {
        if ($DryRun) {
            Write-Output "DRY-RUN adb shell dumpsys activity services com.zsclip.lan"
        } else {
            $serviceState = ""
            for ($i = 0; $i -lt 10; $i++) {
                Start-Sleep -Seconds 2
                $serviceState = & $script:AdbPath shell dumpsys activity services com.zsclip.lan
                if ($serviceState -match "LanAutoSyncService") {
                    break
                }
            }
            if ($serviceState -notmatch "LanAutoSyncService") {
                throw "LanAutoSyncService is not running. Complete pairing or configure WebDAV, then retry."
            }
            Write-Host "LanAutoSyncService is running."
        }
    }

    if ($DryRun) {
        Write-Output "ZSClip Android smoke test dry-run completed."
    } else {
        Write-Host "ZSClip Android smoke test launched successfully."
    }
} finally {
    Pop-Location
}

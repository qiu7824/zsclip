# Package Manager Distribution

This document tracks the package-manager work requested in issue #25.

## Current Status

ZSClip publishes release assets on GitHub:

- Windows installer: `zsclip-v<version>-setup.exe`
- Windows no-LAN installer: `zsclip-v<version>-setup-no-lan.exe`
- Windows portable zip: `zsclip-windows-x86_64-portable.zip`
- Windows no-LAN portable zip: `zsclip-windows-x86_64-no-lan-portable.zip`
- macOS DMG test packages
- Linux `.deb` and `.tar.gz` test packages

## Windows

### Scoop

Scoop is the best first target because ZSClip already supports a portable layout.

Recommended package:

- Use `zsclip-windows-x86_64-portable.zip` for the full build.
- Use `zsclip-windows-x86_64-no-lan-portable.zip` for a no-LAN build if a separate manifest is needed.
- Add `persist: "data"` so `settings.json`, `clipboard.db`, images, LAN device books and other runtime files survive upgrades.

ZSClip resolves data storage in this order on Windows:

1. `<app>\data` when the executable directory is writable.
2. A writable secondary-drive fallback such as `D:\ZSClip\data`.
3. `%LOCALAPPDATA%\ZSClip\data`.

Scoop's `persist` symlink makes option 1 stable across upgrades.

Local manifests:

- `packaging/scoop/zsclip.json`
- `packaging/scoop/zsclip-no-lan.json`

Manifest shape:

```json
{
  "version": "0.9.9.5",
  "description": "Local clipboard history, snippets and quick paste tool.",
  "homepage": "https://github.com/qiu7824/zsclip",
  "license": "GPL-3.0-or-later",
  "url": "https://github.com/qiu7824/zsclip/releases/download/0.9.9.5/zsclip-windows-x86_64-portable.zip",
  "hash": "<sha256>",
  "bin": [["剪贴板.exe", "zsclip"]],
  "shortcuts": [["剪贴板.exe", "ZSClip"]],
  "persist": "data",
  "checkver": {
    "github": "https://github.com/qiu7824/zsclip"
  },
  "autoupdate": {
    "url": "https://github.com/qiu7824/zsclip/releases/download/$version/zsclip-windows-x86_64-portable.zip"
  }
}
```

### winget

winget should use the signed installer once Windows code signing is available. Without signing, users may still see SmartScreen warnings even if installed through winget.

Required winget inputs:

- Installer URL from a GitHub Release.
- SHA256 of the installer.
- Silent install switches from the Inno Setup installer.
- Publisher and package identifiers.

Recommended identifier:

- `qiu7824.ZSClip`

## macOS

Homebrew Cask can point to the DMG assets.

Before submitting to Homebrew Cask, the macOS package should ideally be signed and notarized. The current workflow uses ad-hoc signing for test packages, which is not equivalent to notarization.

## Linux

Current assets are suitable for manual installation:

- Debian/Ubuntu: `.deb`
- Other distributions: `.tar.gz`

Package-manager targets:

- Homebrew on Linux can use the tarball.
- apt requires hosting a Debian repository or submitting to a repository maintainer.
- pacman requires an Arch package recipe, commonly through AUR first.

## Release Checklist

For every package-manager update:

1. Create the GitHub Release assets.
2. Compute SHA256 for the selected asset.
3. Update the manifest or external package repository.
4. Verify install, upgrade and uninstall.
5. Verify runtime data persists after upgrade.

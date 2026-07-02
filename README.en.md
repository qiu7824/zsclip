<div align="center">

# Clipboard / ZSClip

A lightweight Windows clipboard enhancement tool built for local office workflows.

It combines clipboard history, phrases, grouping, VV quick paste, drag-out export, image stickers, and Super Mail Merge in one compact window.

[![Version](https://img.shields.io/github/v/release/qiu7824/zsclip?label=version)](https://github.com/qiu7824/zsclip/releases)
[![License](https://img.shields.io/github/license/qiu7824/zsclip)](LICENSE)
![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-blue)
![Rust](https://img.shields.io/badge/core-Rust-orange)

[简体中文](README.md) | **English**

</div>

## Download

- GitHub release: <https://github.com/qiu7824/zsclip/releases/tag/0.9.9>


## What It Is

`Clipboard` is more than a clipboard history list.

It is designed as a local productivity toolbox for Windows, focused on these everyday tasks:

- Reusing copied text, images, and files quickly
- Managing reusable phrases and template content
- Organizing records with groups
- Triggering `VV` quick paste directly while typing
- Dragging text, images, and files out as real files
- Using Excel + Word together for mail merge workflows

Typical use cases:

- Chat, customer support, and operation replies
- Reusing copywriting, screenshots, and files
- Organizing project or client materials
- Dragging content directly to the desktop or another app
- Inserting merge fields or filling Word documents from Excel

## Key Features

### Clipboard Records

- Automatic clipboard monitoring
- Supports text, images, and files
- Separate tabs for `Clipboard Records` and `Phrases`
- Hotkey opens a quick window that tries not to interrupt current input
- Tray or normal open enters the full main window
- Items can be copied, pasted, or copied then pasted depending on settings
- File items use the native Windows file clipboard instead of plain path text

### Content Management

- Pin records
- Group management
- Group filtering
- Separate phrase management
- Text search and time search
- SQLite local persistence

### Productivity Tools

- Quick search
- Hover preview
- Image sticker window
- AI text cleanup
- WPS task pane MVP: search local text clips from the WPS side pane and insert them into the active document. See [docs/wps-taskpane.md](docs/wps-taskpane.md)
- Multi-device sync entry with an exclusive WebDAV-or-LAN transport choice and LAN QR pairing
- LAN clipboard sync, disabled by default
- Multi-device sync design: [docs/multi-sync.md](docs/multi-sync.md)
- Edge auto-hide
- Drag out text, images, and files to generate real files
- Inline thumbnail preview for image records

### Multilingual UI

- Chinese and English are supported
- The app automatically detects the system UI language
- English is used on non-Chinese systems by default

### VV Mode

- `VV Mode` can be enabled separately in settings
- Type `vv` while inputting text to open a candidate popup near the IME area
- Type `1-9` to paste the matching record directly
- `VV Source` can be set to `Clipboard Records` or `Phrases`
- Supports a default VV group
- Groups can also be switched inside the popup

### Super Mail Merge

- Read Excel files
- Detect sheets, headers, and data rows
- Insert `MERGEFIELD` into Word
- Supports both merge mode and fill mode

### System Integration

- Tray resident
- Auto start
- Global hotkey
- Chinese / English UI with automatic system language detection
- Native lightweight Win32 windows
- Multi-monitor and mixed DPI support

### Multi-Device / LAN Sync

- `Settings -> Multi-device Sync` uses one `Sync Method` choice: `Off`, `WebDAV`, or `LAN`. WebDAV and LAN are not enabled at the same time.
- WebDAV sync reads the remote `zsSyncClipboard.json` before snapshot upload and imports new remote text or image clips into local history; images are downloaded by `dataName` from the cloud `file/` directory.
- The Android client follows the chosen sync method: WebDAV reads/writes the cloud `zsSyncClipboard.json`; LAN connects to the paired Windows device, with selected-text and quick-settings push entrypoints plus in-app image/file history.
- LAN transport is disabled by default and is configured inside `Settings -> Multi-device Sync`; selecting LAN starts discovery and TCP APIs, while selecting WebDAV or Off stops them.
- UDP discovery and TCP API start only after LAN sync is enabled
- Windows devices can be discovered automatically, or paired by manually entering an IP
- Pairing is handled in `Settings -> Multi-device Sync`: request pairing on one device, then approve the `[Pending]` request on the other device
- `Settings -> Multi-device Sync -> Open Pairing QR Page` opens a page with an Android pairing QR code and an iOS/browser entry QR code
- Text, images under 10 MB, and small files can sync automatically
- Remote content can be configured to enter history only or also overwrite the local system clipboard
- iOS Shortcuts and the minimal Android client use the same protocol; see `docs/lan-sync.md` and `docs/ios-shortcuts.md`

### Translations

- The app detects the system UI language automatically
- Chinese and English are currently included
- Translation files are stored in `locales/`
- New languages can be added by copying `locales/en.json` to a new `<language-code>.json`

## Data Loading Model

The current version no longer loads an entire category into memory at once.

The main list now uses a Rust-friendly model:

- paged loading
- virtualized list drawing
- background loading for next pages
- lazy payload hydration for full text, file paths, and image data
- a small cache to avoid repeated database reads

Benefits:

- memory usage stays lower as data grows
- scrolling only renders visible rows
- search, groups, and tab switching reload by query instead of filtering one huge in-memory list

## Usage

1. Run `zsclip.exe`
2. Copy text, images, or files in any app
3. Press the default hotkey `Win + V` to open the quick window
4. Click an item to paste, or use the context menu for more actions
5. Open the full main window through the tray or UI entry when you need search, settings, or batch organization
6. Move frequently used content into `Phrases`
7. Open `Super Mail Merge` directly when the selected item is an Excel file

## Common Shortcuts

- `Win + V`: show or hide the quick window
- `Ctrl + F`: open or close search
- `Ctrl + C`: copy the current item
- `Ctrl + P`: pin or unpin the current item
- `Delete`: delete the current item
- `Esc`: close search, clear selection, or hide the window

## Search Syntax

The search box no longer shows inline hint text. Common examples:

- Plain keyword: `contract`
- Date search: `date:2026-3-26`
- Time search: `time:2026-3-26`
- App search: `app:Word`
- App name with spaces: `app:"Visual Studio Code"`
- Combined search: `date:2026-3-26 app:Word contract`

## VV Mode

`VV Mode` is designed for "typing while inserting content" scenarios.

Basic flow:

1. Enable `VV Mode` in settings
2. Choose `VV Source`
3. Optionally choose a default VV group
4. Type `vv` in an input box
5. When the popup appears, type `1-9`
6. The matching content is pasted directly into the current input target

Notes:

- `Esc` cancels the popup
- group switching is supported inside the popup
- works well in Word, WeChat, Notepad, browser inputs, and similar scenarios

## Super Mail Merge

This part is aimed at local office automation with Word templates and batch document filling.

Typical flow:

1. Choose an Excel file
2. Read sheets, headers, and data rows
3. Insert fields or fill current-row data into Word
4. Use it for mail merge templates or normal form filling

Notes:

- Requires local Windows, Excel, and Word environment
- The current implementation focuses on local Office automation workflows

## Data Directory

The app prefers a `data` folder next to the executable.  
If installed inside a non-writable folder such as `C:\Program Files`, it automatically falls back to `%LOCALAPPDATA%\ZSClip\data`.

Common files:

- Config: `data/settings.json`
- Database: `data/clipboard.db`
- Images: `data/images/`
- Exports: `data/exports/`

If you are using the executable-local `data` folder, migrating the full folder also migrates settings and history.

## Build

Use `Developer PowerShell for VS 2022` or another MSVC-ready environment.

Development:

```powershell
cargo run
```

Release:

```powershell
cargo build --release
```

## Project Structure

The project is being reorganized for better long-term portability. The main entry points are being consolidated into:

- `src/ui.rs`: shared UI model, theme, layout, and list logic
- `src/win_system_ui.rs`: Win32 hosts, drawing adapters, drag-and-drop, and native UI integration
- `src/win_system_params.rs`: Win32 constants, control IDs, and platform parameters
- `src/app.rs`: app state, workflows, data loading, and command dispatch
- `locales/`: UI translation files

Current `0.5.x` also uses two window hosts:

- Full main window: search, settings, editing, grouping, dragging, and full interaction
- Quick window: opened by hotkey for fast paste and typing-focused scenarios

The goal is not just to change the shell, but to keep separating:

- UI logic
- Win32 platform logic
- data and business flow

## Open Source

- GitHub: <https://github.com/qiu7824/zsclip/>

## Support

If this project helps you, support is welcome.

![Support](docs/images/donate.png)

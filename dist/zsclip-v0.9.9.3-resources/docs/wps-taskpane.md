# WPS 任务窗格

剪贴板提供 WPS 任务窗格插件，入口位于 `integrations/wps-taskpane`。

## Scope

- WPS opens a local task pane at `http://127.0.0.1:38473/office/wps/taskpane`.
- The pane reads clipboard records and phrases from `/office/wps/items`.
- Text insertion calls WPS `Selection.TypeText`.
- Image insertion loads `/office/wps/image?id=<id>` and inserts it at the current WPS cursor position.
- The pane listens to `/office/wps/events` for live refresh, with polling fallback in the browser.

## Security

The office endpoints are local-only. Requests from non-loopback peers return `403`. When only the WPS task pane plugin is enabled, the HTTP listener binds to `127.0.0.1` and does not start LAN discovery.

## Usage

1. Start the clipboard app.
2. Enable `Settings -> Plugins -> WPS Task Pane`.
3. Open WPS and enable the task pane add-in from the packaged add-in files.
4. Click `剪贴板 -> 打开剪贴板`.
5. Search clipboard records or phrases, then click `插入`.

Do not use the WPS debug host for acceptance. The debug host can inject its own debugging ribbon button; the packaged add-in ribbon only defines `剪贴板 -> 打开剪贴板`.

## Verification

```powershell
.\verify-multisync.ps1
```

The verification script checks the WPS add-in files, the task pane HTTP contract, Rust tests, Android tests, Android APK metadata, Android smoke dry-run, and release APK consistency.

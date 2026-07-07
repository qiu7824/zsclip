# ZSClip 多端同步设计

## 中文

### 方向

多端同步统一成一个产品入口，底层保留两种传输方案：

- `WebDAV 云同步`：适合跨网络、多设备长期同步。参考 SyncClipboard 的思路，使用一个最新清单文件描述剪贴板状态，再按需读取图片或文件数据。
- `局域网同步`：适合同一 Wi-Fi 下快速传输。保留现有 ZSClip LAN 配对 token、`/v1/clip`、`/v1/latest`，并增加只读的 `zsSyncClipboard.json` 清单入口，方便 Android、iOS 快捷指令和后续二维码绑定复用。

桌面端设置页现在以 `多端同步` 作为统一入口，顶部 `同步方案` 在 `关闭 / WebDAV / 局域网` 之间单选。WebDAV 和局域网不会同时开启；选择局域网后才显示扫码绑定入口并启动本地服务。局域网发现、配对和手机接入也收敛在同一页，不再提供单独的局域网设置页。

WebDAV 不能天然实现“服务端推送式实时同步”，它更适合按间隔轮询和手动同步。SyncClipboard 的实时体验主要来自客户端内置服务器或独立服务器这类服务端方案；如果 ZSClip 后续要做同类能力，应作为 `内置同步服务` 这样的第三种同步方案加入单选列表，而不是把 WebDAV 伪装成实时通道。

本轮不切换到 MQTT。MQTT 需要 broker、账号和主题配置，适合实时推送路线，但会提高普通用户的配置成本。

### 统一清单

局域网服务新增只读入口：

```http
GET /zsSyncClipboard.json?device=<device_id>&token=<token>
GET /file/zsclip_image_<id>.png?device=<device_id>&token=<token>
```

`zsSyncClipboard.json` 返回：

- `protocol`: `ZSCLIP_MULTI_SYNC_V1`
- `transport`: `lan` / `webdav`，客户端会把它纳入状态 key，避免不同传输来源的同类记录互相误判为同一次同步
- `clip`: 最新可同步记录；文本直接放在 `content`，图片只返回 `dataName`，手机按需下载 `/file/...`

这样 WebDAV 以后可以保存同样形状的 `zsSyncClipboard.json`，局域网则实时从 Windows 本地数据库生成，避免两套客户端逻辑。

当前 WebDAV 云同步也会在远程目录写入同形态文件：

```text
<remote_dir>/zsSyncClipboard.json
<remote_dir>/file/zsclip_image_<id>.png
```

桌面端执行 WebDAV 同步时，会先读取远端 `zsSyncClipboard.json`。如果远端最新记录是文本且本地还没有同签名记录，会导入为本地剪贴板记录；如果远端最新记录是图片，会按 `dataName` 下载 `<remote_dir>/file/...` 中的 PNG，校验后导入本地图片记录。

现有快照同步仍然保留 `manifest.json` 和 `backups/latest.zip`，用于完整数据库/设置恢复；`zsSyncClipboard.json` 则面向移动端和轻量客户端读取最新剪贴板状态。

### 二维码绑定

QuickClipboard 的二维码核心价值不是协议本身，而是把“输入地址”和“携带配对信息”合成一个入口。ZSClip 更适合分两级：

- Android 扫描 `zsclip://pair?host=<ip:port>`：打开 App，填入主机地址，然后发起现有配对请求，Windows 仍然需要点击允许。
- iOS 扫描 `http://<ip:port>/mobile/setup`：打开网页，展示快捷指令配置、图片列表入口和清单 URL；iOS 不安装原生 App。

这样比把 token 直接写进公开二维码安全，也不破坏现有信任设备列表。

### Android UI

Android 端应逐步收敛为三个主操作：

- 绑定 Windows
- 推送到电脑
- 检查多端同步清单并打开图片/多端同步页面

Android App 当前会读取 LAN 暴露的 `zsSyncClipboard.json`，也可以配置 WebDAV 地址、账号和远程目录来检查、手动拉取或上传 `<remote_dir>/zsSyncClipboard.json` 中的最新文本/图片。产品层按用户选择的同步方案工作：局域网方案连接已扫码绑定的 Windows，WebDAV 方案读写云端清单；两种传输使用同一套清单解析逻辑，但不在桌面设置中同时开启。图片通过下载页或 `/file/...` 按需获取，不写入手机系统剪贴板。

UI 层建议采用 Android 官方 Material 3 组件和动态颜色。小米、三星等系统没有统一的厂商 UI 调用层，稳定做法是使用系统主题能力、Material You 动态色、系统分享面板、快捷设置 Tile、通知前台服务这些通用接口。

### 本地验证

仓库根目录提供一键验证脚本：

```powershell
.\verify-multisync.ps1
```

脚本会执行 Rust 测试、Android 单测、Debug APK 构建、UTF-8/乱码扫描、iOS 快捷指令文档契约校验、APK metadata 校验、Android smoke 命令 dry-run、Android 发布包契约校验，并确认 `release/0.9.9.4/android/zsclip-lan-debug.apk` 与当前构建产物哈希、发布说明中的 SHA256 一致。真机入口烟测可以在安装 adb 且连接 Android 手机后执行：

```powershell
.\verify-multisync.ps1 -RunDeviceSmoke -SmokeHost "192.168.1.10:38473"
```

iOS 手工验收按 `docs/ios-shortcuts.md` 的“iOS 局域网验收清单”执行。

## English

### Direction

Multi-device sync should be one product entry with two transports:

- `WebDAV cloud sync` for cross-network and long-running multi-device sync.
- `LAN sync` for fast trusted local-network transfer.

The desktop settings page now uses `Multi-device Sync` as the shared entry with a single `Off / WebDAV / LAN` method choice. WebDAV and LAN are mutually exclusive. LAN discovery, pairing, and mobile setup are folded into the same page instead of a standalone LAN page.

The shared contract is a latest-state manifest. LAN now exposes a read-only `zsSyncClipboard.json` and lazy `/file/...` downloads while preserving the existing ZSClip pairing token, `/v1/clip`, and `/v1/latest` protocol. WebDAV sync also uploads the same manifest shape to `<remote_dir>/zsSyncClipboard.json`, with image payloads under `<remote_dir>/file/`.

When desktop WebDAV sync runs, it reads the remote `zsSyncClipboard.json` first. Remote text clips are imported into local history when their lightweight signature is new. Remote image clips download the referenced `<remote_dir>/file/...` PNG and import it into local image history after validation.

### Manifest

```http
GET /zsSyncClipboard.json?device=<device_id>&token=<token>
GET /file/zsclip_image_<id>.png?device=<device_id>&token=<token>
```

`transport` is part of the client status key, so LAN and WebDAV records do not accidentally collapse into the same sync state. Text is inline in `content`; image records expose `dataName` and are downloaded only when requested.

### QR Pairing

The QR flow should simplify input, not bypass trust:

- Android scans `zsclip://pair?host=<ip:port>`, opens the app, and starts the existing pairing request.
- iOS scans `http://<ip:port>/mobile/setup`, opens a browser page with Shortcuts configuration and read-only download links.

### Android UI

The Android app reads the LAN `zsSyncClipboard.json` and can also check, manually pull, or upload latest text/image records through the WebDAV `<remote_dir>/zsSyncClipboard.json` after the user configures the WebDAV URL, account, and remote directory. At the product level the chosen method decides the path: LAN talks to the paired Windows host, while WebDAV reads and writes the cloud manifest. Both transports use the same manifest parser; images are downloaded on demand instead of being written to the Android system clipboard.

Use official Android Material 3 patterns, dynamic colors where available, system share sheets, Quick Settings tiles, and foreground-service notifications. There is no reliable unified Xiaomi/Samsung-specific UI layer worth depending on.

### Local Verification

Run the repository-level verification script:

```powershell
.\verify-multisync.ps1
```

It runs Rust tests, Android unit tests, Debug APK assembly, UTF-8/mojibake scans, the iOS Shortcuts docs contract check, APK metadata checks, the Android smoke command dry-run, the Android release package contract check, and verifies that `release/0.9.9.4/android/zsclip-lan-debug.apk` matches both the current build hash and the SHA256 recorded in the release README. Android device smoke testing can be added when adb and a phone are available:

```powershell
.\verify-multisync.ps1 -RunDeviceSmoke -SmokeHost "192.168.1.10:38473"
```

iOS manual acceptance follows the `iOS 局域网验收清单` section in `docs/ios-shortcuts.md`.

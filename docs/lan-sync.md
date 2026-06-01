# ZSClip 局域网剪贴板同步

局域网同步独立于 WebDAV 云同步，默认关闭。开启后，ZSClip 会启动 UDP 发现、TCP API 和后台同步线程；关闭后会停止监听和发送队列，避免最小化后继续占用资源。

## 默认行为

- 发现端口：UDP `38472`。
- API 端口：TCP `38473`，如果被占用会自动尝试后续端口。
- 配对方式：发起端在附近设备列表选择设备并点击配对，接收端在同一个附近设备列表选中 `[待允许]` 请求后点击允许。
- Windows 对 Windows：信任设备之间自动双向同步。
- Android 客户端：作为 `client_only` / `pull_only` 设备接入；手机可主动推送文本/图片到 Windows，也可拉取 Windows 最新文本到手机剪贴板，Windows 不会主动 POST 给 Android。
- 安全边界：局域网 + 配对 token + 信任设备列表；token 使用 Windows DPAPI 加密保存到 `data/lan_devices.json`。
- 同步方式可在设置页下拉选择：默认 `只进入记录`；也可以选择 `直接覆盖剪贴板`。
- 开启同步时会自动尝试添加 Windows 防火墙入站规则：TCP API 端口和 UDP 发现端口都会放行当前 `zsclip.exe`。如果没有管理员权限导致失败，设置页状态会提示需要管理员运行一次或手动允许专用网络访问。

## Android 快速开始

1. Windows 打开 `设置 -> 局域网`，开启 `启用局域网同步` 并保存。
2. Android 打开 ZSClip LAN，点击 `发现设备`；如果发现不到，手动输入 Windows 设置页显示的 `IP:端口`。
3. 点击 `请求配对`，在 Windows 局域网页选中 `[待允许]` 请求并点击 `允许配对`。
4. 配对成功后，Android 会保存绑定；`局域网自动同步` 开关只使用这个已保存绑定，不会重新匹配。
5. 在 Android 主界面或通知栏快捷开关中使用：
   - `推送到电脑`：把手机剪贴板文本或输入框文本推送到 Windows。
   - `拉取到手机`：把 Windows 最新文本写入手机剪贴板。
   - `局域网自动同步`：前台服务轮询 Windows 最新文本，有新文本时写入手机剪贴板。
6. 从微信、浏览器、相册等 App 分享文本/图片到 ZSClip，也会使用同一套协议推送到 Windows。

## Windows 对 Windows

1. 两台电脑连接到同一个局域网。
2. 打开 `设置 -> 局域网`。
3. 开启 `启用局域网同步` 并保存。
4. 等待发现设备，或在 `手动 IP` 输入 `192.168.x.x` / `192.168.x.x:38473`。
5. 在附近设备列表点击设备名，再点击 `配对选中设备`。
6. 另一台电脑的附近设备列表会出现 `[待允许]` 请求，选中后点击 `允许配对`。
7. 之后任意一端复制文本、小于 10MB 的图片或总量不超过 50MB 的普通文件，另一端会自动进入记录列表顶部。

## Android 接入

Windows 端提供轻量 HTTP API，Android App 请求配对后在 Windows 设置页点允许即可自动保存凭据。Android 和 iOS 使用同一套移动端 `/v1/clip` envelope，移动端 `hash` 使用 `msg:<device_id>:<seq>`，Windows 入库时再按内容重新计算 CRC 签名并强制去重。App 内提供三个通知栏快捷开关入口：

- `局域网自动同步`：常驻前台服务，轮询 Windows 最新记录；目前只会把文本写入手机剪贴板。
- `拉取到手机`：单次拉取 Windows 最新文本并写入手机剪贴板。
- `推送到电脑`：把 Android 剪贴板文本、输入框文本或分享菜单中的文本主动推送到 Windows。
- `图片下载页`：在手机浏览器打开 Windows 局域网页，查看最近图片记录并手动下载 PNG。
- `分享图片到 ZSClip`：从 Android 分享菜单接收图片，转为 PNG/Base64 后逐张推送到 Windows。

Android 当前没有 TCP 接收服务，所以保存为信任设备后，Windows 会识别其 `client_only` / `pull_only` 能力并跳过主动推送。

## API

- `GET /v1/info`：读取设备信息和能力。
- `POST /v1/pair/request`：发起配对。
- `GET /v1/pair/status?id=...`：轮询配对结果。
- `POST /v1/clip`：推送文本或图片记录。
- `GET /v1/latest`：拉取数据库中最新有效记录。
- `GET /mobile/images?device=...&token=...`：移动端图片下载列表页。
- `GET /mobile/image?id=...&device=...&token=...`：下载单张 PNG 图片。

`POST /v1/clip` 和 `GET /v1/latest` 需要请求头：

```http
X-ZSClip-Device: <device_id>
X-ZSClip-Token: <pair_token>
```

## 内容限制

- 文本：自动同步；移动端发送 `msg:<device_id>:<seq>`，Windows 入库时统一计算内容签名。
- 图片：Windows 对 Windows 自动同步 PNG/JPEG 落盘记录，编码后不超过 `10MB`。
- Android 分享图片：转 PNG 后通过同一 `/v1/clip` envelope 推送，超限图片跳过并提示。
- 文件：普通小文件可自动同步，总量和单文件默认不超过 `50MB`；目录和超限文件跳过。大文件可在文件记录上右键 `推送到局域网设备` 手动分块发送。
- 去重：LAN 入站强制去重，不受普通“重复内容过滤并提升到首行”开关影响；重复重试不会更新 `/v1/latest`。

## 常见问题

- 未发现设备：检查两端是否同网段、Windows 防火墙是否允许 ZSClip 访问专用网络。
- 防火墙：设置页显示“防火墙自动放行失败”时，请右键以管理员身份运行一次 ZSClip 后重新开启局域网同步，或手动在 Windows 防火墙中允许 `zsclip.exe` 的专用网络入站访问。
- 端口占用：TCP 会自动尝试后续端口，设置页状态会显示实际监听结果。
- Android 自动同步不写入图片/文件：这是当前版本的预期行为，Android 只把 Windows 最新文本写入手机剪贴板。
- Android/iOS 需要图片时，打开图片下载页手动选择下载，不写入手机系统剪贴板。
- Android 自动同步不需要重新匹配：只要主界面显示已绑定 Windows，通知栏 `局域网自动同步` 开关会直接使用已保存 token。
- `推送到电脑` 读不到剪贴板：Android 系统可能限制后台读取剪贴板，快捷开关会打开主界面，可在输入框粘贴后发送。
- 不想后台运行：关闭 `启用局域网同步` 并保存，或在托盘中切换 `局域网同步：关`。

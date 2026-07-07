# ZSClip 局域网剪贴板同步

局域网同步现在作为 `多端同步` 里的一个传输方案，和 WebDAV 使用同一份最新清单思路；默认关闭。开启后，ZSClip 会启动 UDP 发现、TCP API 和后台同步线程；关闭后会停止监听和发送队列，避免最小化后继续占用资源。

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

1. Windows 打开 `设置 -> 多端同步`，把 `同步方案` 选择为 `局域网` 并保存。
2. 点击 `打开扫码绑定页`，用 Android 扫描页面中的 `Android 配对` 二维码；App 会自动填入 Windows 地址并请求配对。
3. 如果扫码不可用，也可以在 Android 手动输入 Windows 设置页显示的 `IP:端口`，再点击 `请求配对`。
4. 在 Windows `设置 -> 多端同步` 的设备列表中选中 `[待允许]` 请求并点击 `允许配对`。
5. 配对成功后，Android 会保存绑定；`多端自动同步` 开关优先使用这个已保存绑定，不会重新匹配。
6. 在 Android 主界面或通知栏快捷开关中使用：
   - `推送到电脑`：折叠通知栏并启动透明前台同步页，读取手机剪贴板中的新文本后推送到已配对 Windows 或 WebDAV。
   - `拉取到手机`：局域网方案下把 Windows 最新文本写入手机剪贴板。
   - `多端自动同步`：局域网方案下实时前台服务会先自动推送手机新文本，再拉取 Windows 最新文本；服务被系统暂停时仍保留周期后台检查。
7. 在支持 `选中文本` 菜单的 App 里选择一段文字，点 `同步到电脑` 可直接走同一推送路径；这个入口可在 ZSClip 主界面的“选中文本菜单”开关里关闭。
8. 从微信、浏览器、相册等 App 分享文本/图片到 ZSClip 时，局域网方案会推送到已配对 Windows；WebDAV 方案会写入云端清单。

### Android 调试安装

本地调试包由 `mobile/android` 工程生成：

```powershell
cd mobile/android
gradle assembleDebug
adb install -r app/build/outputs/apk/debug/app-debug.apk
```

安装后应用名显示为 `ZSClip 多端同步`，APK 版本名为 `0.9.9.4`，系统快捷开关名称为 `推送到电脑`、`拉取到手机`、`多端自动同步`。Android 端支持 `zsclip://pair?host=<ip:port>` 深链，扫码后会自动填入 Windows 地址并发起配对请求；系统 `选中文本` 菜单会显示 `同步到电脑`。

安装后可先用脚本做 adb 入口烟测：

```powershell
cd mobile/android
.\smoke-adb.ps1 -HostAddress "192.168.1.10:38473" -SkipBuild -DryRun
.\smoke-adb.ps1 -HostAddress "192.168.1.10:38473"
.\smoke-adb.ps1 -HostAddress "192.168.1.10:38473" -SkipBuild -CheckAutoSync
```

`-DryRun` 不需要 adb 或真机，只打印将要执行的安装、打开主界面、触发 `zsclip://pair` 配对深链和分享文本命令；去掉 `-DryRun` 后脚本会自动构建并安装 APK，然后依次执行这些入口烟测。完成配对并在 App 中开启自动同步后，`-CheckAutoSync` 会用 `dumpsys` 验证 `LanAutoSyncService` 是否真实运行。也可以手动执行同等命令：

```powershell
adb shell am start -n com.zsclip.lan/.MainActivity
adb shell am start -a android.intent.action.VIEW -d "zsclip://pair?host=192.168.1.10%3A38473"
adb shell am start -n com.zsclip.lan/.MainActivity -a android.intent.action.SEND -t text/plain --es android.intent.extra.TEXT "hello from adb"
```

第一条应打开 Android 主界面；第二条应填入二维码里的 Windows 地址并请求配对；第三条应走分享文本入口，并按当前同步方案推送到已配对 Windows 或云端清单。

## Windows 对 Windows

1. 两台电脑连接到同一个局域网。
2. 打开 `设置 -> 多端同步`。
3. 把 `同步方案` 选择为 `局域网` 并保存。
4. 等待发现设备，或在 `手动 IP` 输入 `192.168.x.x` / `192.168.x.x:38473`。
5. 在附近设备列表点击设备名，再点击 `配对选中设备`。
6. 另一台电脑的附近设备列表会出现 `[待允许]` 请求，选中后点击 `允许配对`。
7. 之后任意一端复制文本、小于 10MB 的图片或总量不超过 50MB 的普通文件，另一端会自动进入记录列表顶部。

## Android 接入

Windows 端提供轻量 HTTP API，Android App 请求配对后在 Windows 设置页点允许即可自动保存凭据。Android 和 iOS 使用同一套移动端 `/v1/clip` envelope，移动端 `hash` 使用 `msg:<device_id>:<seq>`，Windows 入库时再按内容重新计算 CRC 签名并强制去重。App 内提供快捷入口：

- `多端自动同步`：实时前台服务按当前同步方案双向检查文本。局域网方案会先读取手机剪贴板中新出现的文本并推送到已配对 Windows，再拉取 Windows 最新文本写入手机剪贴板；WebDAV 方案同样读写云端 `zsSyncClipboard.json`。服务被系统暂停或进程重建后，WorkManager 会保留至少每 15 分钟一次的周期后台检查；重新打开 App 会恢复实时服务。图片/文件提示到 App 内 `图片和文件` 查看，不写入手机系统剪贴板。
- `拉取到手机`：单次拉取最新文本并写入手机剪贴板，来源跟随当前同步方案。
- `推送到电脑`：通知栏快捷开关会启动透明前台 Activity 读取手机剪贴板并自动关闭；主界面输入框、分享菜单和选中文本菜单也会走同一推送路径。
- `图片和文件`：局域网方案读取 Windows 最近图片/文件 JSON 历史，图片可在 App 内预览、保存、分享；文件可下载到 App 下载目录后通过 FileProvider 打开或分享。WebDAV 方案仅显示最新图片清单项。
- `分享图片到 ZSClip`：从 Android 分享菜单接收图片，转为 PNG/Base64 后按当前同步方案推送到 Windows 或上传到 `<remote_dir>/file/` 并更新云端清单。

Android 当前没有 TCP 接收服务，所以保存为信任设备后，Windows 会识别其 `client_only` / `pull_only` 能力并跳过主动推送。

## API

- `GET /v1/info`：读取设备信息和能力。
- `POST /v1/pair/request`：发起配对。
- `GET /v1/pair/status?id=...`：轮询配对结果。
- `POST /v1/clip`：推送文本或图片记录。
- `GET /v1/latest`：拉取数据库中最新有效记录。
- `GET /mobile/setup`：移动端连接说明页，提供 Android 配对二维码、iOS/浏览器二维码、快捷指令配置提示和多端同步入口。
- `GET /v1/mobile/items?limit=50`：Android App 内历史 JSON，只返回最近图片和文件记录，需要 `X-ZSClip-Device` / `X-ZSClip-Token`。
- `GET /v1/mobile/items/{id}/image`：Android App 内预览或保存指定图片，需要请求头鉴权。
- `GET /v1/mobile/items/{id}/file/{index}`：按数据库文件记录和文件索引下载文件，需要请求头鉴权，不接受路径直传。
- `GET /mobile/images?device=...&token=...`：旧移动端图片下载列表页，保留兼容。
- `GET /mobile/image?id=...&device=...&token=...`：旧接口下载单张 PNG 图片。
- `GET /zsSyncClipboard.json?device=...&token=...`：多端同步只读清单；文本内联，图片返回 `dataName`。
- `GET /file/zsclip_image_<id>.png?device=...&token=...`：按清单中的 `dataName` 懒下载图片。

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
- Android/iOS 需要图片时，Android 打开 App 内 `图片和文件` 预览或下载；iOS 仍用网页/快捷指令入口。不写入手机系统剪贴板。
- Android 主界面的“检查多端同步”会按当前同步方案读取 `zsSyncClipboard.json`，用于确认最新文本或图片 `dataName` 状态；图片/文件浏览走 App 内 `图片和文件`。
- Android 主界面的 `WebDAV 多端同步` 可读取云端 `<remote_dir>/zsSyncClipboard.json`，用于跨网络检查同一份最新清单，也可以手动把云端最新文本拉到手机剪贴板、带 WebDAV 认证下载最新图片，或把输入框文本推送回 WebDAV。手机分享文本/图片到 ZSClip 时，如果尚未完成局域网配对但已经配置 WebDAV，会写入同一份 `zsSyncClipboard.json`；图片额外上传到 `<remote_dir>/file/`。
- Android 自动同步不需要重新匹配：主界面显示已绑定 Windows 且同步方案为局域网时，通知栏 `多端自动同步` 开关会直接使用已保存 token；同步方案为 WebDAV 时使用云端清单轮询。
- Android 10+ 后台剪贴板限制：普通后台 App 不能持续读取其它 App 刚复制的系统剪贴板。ZSClip 会在 App 回到前台、获得焦点或系统允许前台服务读取时自动推送手机新文本；如果复制后一直停留在其它 App，系统可能不会把内容开放给 ZSClip。分享菜单入口不受这个限制。
- Android 15 会限制 `dataSync` 前台服务的连续运行时间；WebDAV 实时服务被系统暂停后会继续执行周期后台检查，打开 ZSClip 可恢复实时轮询。局域网配对模式使用 `connectedDevice` 前台服务类型。
- `推送到电脑` 读不到剪贴板：Android 系统可能限制后台读取剪贴板；快捷开关现在会先启动透明前台 Activity 再读取。仍失败时，可打开 ZSClip 后重试，或用选中文本菜单/分享菜单直接发送。
- 不想后台运行：在 `设置 -> 多端同步` 把 `同步方案` 改为 `关闭` 或 `WebDAV` 并保存，局域网服务会停止。

## 真机验收清单

1. Windows 选择 `同步方案 -> 局域网` 后，`打开扫码绑定页` 能显示 Android 配对二维码和 iOS/浏览器入口二维码。
2. Android 扫描 `Android 配对` 二维码后自动填入 `IP:端口`，发起配对；Windows 点允许后，Android 显示已绑定设备。
3. Android 已配对时，从主界面、分享菜单、`同步到电脑` 选中文本菜单和 `推送到电脑` 快捷开关推送文本，Windows 都能新增文本记录。
4. Android 已配对时，从相册或其他 App 分享图片到 ZSClip，Windows 能新增图片记录；超过 10MB 或超大像素图片会跳过并提示。
5. Android `拉取到手机` 在最新记录是文本时写入手机剪贴板；最新记录是图片/文件时只提示到 `图片和文件` 查看或下载，不写入图片/文件剪贴板。
6. Android 选择 WebDAV 方案时，主界面输入框、分享文本和 `推送到电脑` 快捷开关会更新 `<remote_dir>/zsSyncClipboard.json`。
7. Android 选择 WebDAV 方案并分享图片时，会上传 `<remote_dir>/file/zsclip_image_*.png`，并更新同一份 `zsSyncClipboard.json`。
8. Android `多端自动同步` 开启后，复制手机文本并回到 ZSClip 会自动推送到 Windows 或 WebDAV；Windows/WebDAV 有新文本时会自动写入手机剪贴板；关闭 App 或系统暂停实时服务后，周期后台检查仍会继续。
9. Android `图片和文件` 在局域网配对时显示最近图片/文件历史，图片可预览/保存/分享，文件可下载打开/分享；WebDAV 仅显示最新图片项。
10. iOS 快捷指令仍按 `docs/ios-shortcuts.md` 手动推送/拉取；图片通过 `/mobile/images` 页面手动下载。

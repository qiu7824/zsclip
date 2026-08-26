# iOS 快捷指令接入 ZSClip

iOS 不支持普通 App 长期后台读取系统剪贴板，所以第一版采用快捷指令手动推送/拉取。iOS 和 Android 使用同一套移动端 `/v1/clip` envelope，区别只是入口：Android 由 App/分享菜单触发，iOS 由快捷指令/分享菜单/小组件/NFC 触发。

## 能实现什么

- `推送到电脑`：快捷指令读取 iOS 剪贴板文本或分享菜单输入，POST 到 Windows `/v1/clip`。
- `拉取到手机`：快捷指令读取 Windows `/v1/latest`，仅当最新记录是文本时写入 iOS 剪贴板。
- `推送图片到电脑`：快捷指令从分享菜单/照片选择图片，Base64 后 POST 到同一个 `/v1/clip`。
- 不支持后台常驻自动监听 iOS 剪贴板；可以放到桌面小组件、操作按钮、NFC 或分享菜单里手动触发。

## 准备配置

可以在“快捷指令”App 中先准备这些变量，也可以参考 `docs/ios-shortcuts-config.example.json` 保存到“文件 -> Shortcuts -> ZSClip/config.json”：

```json
{
  "host": "192.168.1.10:38473",
  "device_id": "ios-iphone",
  "device_name": "iPhone",
  "token": "",
  "setup_url": "http://192.168.1.10:38473/mobile/setup",
  "manifest_url": "http://192.168.1.10:38473/zsSyncClipboard.json?device=ios-iphone&token=<token>",
  "images_url": "http://192.168.1.10:38473/mobile/images?device=ios-iphone&token=<token>"
}
```

- `host`：Windows 设置页显示的局域网地址和端口。
- `device_id`：自己填写一个稳定 ID，建议 `ios-设备名`。
- `token`：配对成功后写入。
- `setup_url`：可选，打开 Windows 手机连接页；这里会显示 Android 配对二维码、iOS/浏览器入口和多端同步清单提示。
- `manifest_url`：可选，读取多端同步最新清单；把 `<token>` 替换成实际 token。
- `images_url`：可选，打开图片下载页；把 `<token>` 替换成实际 token。

## 快捷指令一：ZSClip 配对

目标：向 Windows 发起配对，用户在 Windows `设置 -> 多端同步` 允许后，iOS 保存 token。

1. 动作：`文本`，内容为配对 JSON：

```json
{
  "device_id": "<device_id>",
  "name": "<device_name>",
  "tcp_port": 0,
  "capabilities": ["text", "image", "latest", "client_only", "pull_only"]
}
```

2. 动作：`获取 URL 内容`。
   - URL：`http://<host>/v1/pair/request`
   - 方法：`POST`
   - 请求体：`文件`
   - 请求头：`Content-Type: application/json`
3. 动作：`从输入中获取词典`，读取返回的 `pair_id` 和 `code`。
4. 动作：`显示提醒`，内容：`请在 Windows 多端同步页允许配对，安全码：<code>`。
5. 动作：`重复 90 次`：
   - `等待 1 秒`
   - `获取 URL 内容`：`http://<host>/v1/pair/status?id=<pair_id>`
   - 从 JSON 读取 `status`
   - 如果 `status` 是 `accepted`，读取 `token`，保存到配置 JSON，然后 `停止此快捷指令`
   - 如果 `status` 是 `rejected`，显示“配对被拒绝”，然后停止

## 移动端统一 Envelope

文本和图片都使用消息级 hash：`msg:<device_id>:<seq>`。Windows 收到后会按文本正文或图片内容重新计算 CRC 签名并强制去重，所以 iOS 快捷指令不需要自己实现 CRC32。

文本 envelope：

```json
{
  "message_id": "<device_id>-<seq>",
  "origin_device_id": "<device_id>",
  "origin_seq": <seq>,
  "kind": "text",
  "hash": "msg:<device_id>:<seq>",
  "created_at_ms": <seq>,
  "preview": "<文本前80字>",
  "text": "<文本正文>",
  "image_png_base64": null,
  "file_meta": []
}
```

图片 envelope：

```json
{
  "message_id": "<device_id>-<seq>",
  "origin_device_id": "<device_id>",
  "origin_seq": <seq>,
  "kind": "image",
  "hash": "msg:<device_id>:<seq>",
  "created_at_ms": <seq>,
  "preview": "<图片名称或 iOS 图片>",
  "text": null,
  "image_png_base64": "<PNG或图片文件Base64>",
  "file_meta": []
}
```

## 快捷指令二：推送到电脑

1. 动作：`获取剪贴板`。
2. 动作：生成当前时间毫秒作为 `seq`。
3. 动作：`文本`，按上面的文本 envelope 模板生成 JSON。
4. 动作：`获取 URL 内容`。
   - URL：`http://<host>/v1/clip`
   - 方法：`POST`
   - 请求体：`文件`
   - 请求头：

```text
Content-Type: application/json
X-ZSClip-Device: <device_id>
X-ZSClip-Token: <token>
```

5. 动作：从返回 JSON 判断 `ok`，成功时显示“已推送到 Windows”。

## 快捷指令三：从分享菜单推送到电脑

1. 快捷指令详情中打开 `在共享表单中显示`。
2. 接收类型选择 `文本`、`网页`，需要图片时再勾选 `图像`。
3. 如果输入是文本，复用“推送到电脑”的文本 envelope。
4. 如果输入是图片，走下面“推送图片到电脑”的图片 envelope。

## 快捷指令四：推送图片到电脑（可选）

1. 快捷指令接收“共享表单”的图片输入，或使用 `选择照片`。
2. 动作：`编码媒体` 或 `Base64 编码`，得到图片 Base64。
3. 动作：生成当前时间毫秒作为 `seq`。
4. 动作：`文本`，按上面的图片 envelope 模板生成 JSON。
5. 动作：`获取 URL 内容`，POST 到 `http://<host>/v1/clip`，请求头同文本推送。

说明：Windows 当前字段名是 `image_png_base64`。如果快捷指令能先转 PNG，优先转 PNG；如果直接 Base64 原图，需手工验证 Windows 是否能识别该格式。第一版建议先把文本推送/拉取跑通，图片作为可选流程。

## 快捷指令五：拉取到手机

1. 动作：`获取 URL 内容`。
   - URL：`http://<host>/v1/latest`
   - 方法：`GET`
   - 请求头：

```text
X-ZSClip-Device: <device_id>
X-ZSClip-Token: <token>
```

2. 动作：`从输入中获取词典`，读取 `clip.kind` 和 `clip.text`。
3. 如果 `clip.kind` 等于 `text` 且 `clip.text` 不为空：
   - 动作：`拷贝到剪贴板`，内容为 `clip.text`
   - 显示“已拉取 Windows 最新文本”
4. 否则显示“最新记录不是文本，未写入 iOS 剪贴板”。

## 快捷指令六：打开图片列表

目标：不把图片写入 iOS 剪贴板，而是在 Safari 中打开 Windows 提供的局域网图片下载页，用户自行选择下载。

1. 动作：`文本`，拼出图片列表 URL：

```text
http://<host>/mobile/images?device=<device_id>&token=<token>
```

2. 如果 `device_id` 或 `token` 中包含中文、空格或特殊字符，先使用快捷指令的 URL 编码动作分别编码后再拼接。
3. 动作：`打开 URL`，打开上面的地址。
4. 页面会列出最近可下载的图片记录；点击 `下载 PNG` 后由 Safari/文件 App 处理下载。

## iOS 局域网验收清单

1. Windows 开启局域网同步后，在 iPhone Safari 打开 `http://<host>/mobile/setup`，确认页面能显示手机连接说明、`zsSyncClipboard.json` 清单地址和图片列表地址。
2. 运行 `ZSClip 配对` 快捷指令，Windows 多端同步页点允许后，配置里的 `token` 被保存。
3. 运行 `推送到电脑`，Windows 剪贴板历史应新增一条来自 iOS 的文本记录。
4. 在 Windows 复制一段文本后运行 `拉取到手机`，iOS 剪贴板应变成 Windows 最新文本。
5. 在 Windows 复制一张图片后运行 `拉取到手机`，快捷指令只提示“最新记录不是文本，未写入 iOS 剪贴板”。
6. 打开 `manifest_url`，返回 JSON 的 `protocol` 应是 `ZSCLIP_MULTI_SYNC_V1`；最新图片只出现 `dataName`，不会内联大图。
7. 运行 `打开图片列表`，Safari 应打开 `/mobile/images` 页面，用户点击 `下载 PNG` 后手动保存图片。

## 可选：URL Scheme 触发

创建好快捷指令后，可以用以下 URL 从小组件、NFC、自动化或浏览器触发：

```text
shortcuts://run-shortcut?name=ZSClip%20%E6%8E%A8%E9%80%81%E5%88%B0%E7%94%B5%E8%84%91
shortcuts://run-shortcut?name=ZSClip%20%E6%8B%89%E5%8F%96%E5%88%B0%E6%89%8B%E6%9C%BA
shortcuts://run-shortcut?name=ZSClip%20%E5%9B%BE%E7%89%87%E5%88%97%E8%A1%A8
```

## 说明

- 第一版不交付 `.shortcut` 二进制导入文件，避免依赖未公开且易变的导入格式。
- iOS 快捷指令不做后台常驻同步；需要用户手动运行、桌面小组件触发、NFC 触发或自动化触发。
- 图片同步采用 Windows 局域网页列表下载，不写入 iOS 系统剪贴板。
- LAN 入站去重由 Windows 强制执行，不受普通“重复内容过滤并提升到首行”开关影响。

# 更新发布

“关于 → 更新源设置”留空时，从 [官方 GitHub Releases](https://github.com/qiu7824/zsclip/releases) 读取版本与更新说明。程序匹配当前普通版或无局域网版的安装包；用户选择“下载并安装”后，才开始下载、校验和安装。已经配置的非空更新源继续使用，不会被默认线路覆盖。

## 官方发布资产

Windows 安装包按发布版本命名：

| 版本类型 | 文件名 |
| --- | --- |
| 普通版 | `zsclip-vVERSION-setup.exe` |
| 无局域网版 | `zsclip-vVERSION-setup-no-lan.exe` |

`VERSION` 为发布版本号。标签发布工作流从实际构建产物生成并上传 `SHA256SUMS.txt` 和 `zsclip-update.json`，清单包含两个 Windows 安装包的官方 GitHub 下载地址、准确文件名、SHA-256、字节数和同一发布的更新说明。

默认更新线路以该发布的 `SHA256SUMS.txt` 和资产字节数校验安装包；没有校验清单的旧发布可使用 GitHub 资产提供的 SHA-256 `digest`。缺少有效校验信息时不会安装。修改安装包、重新签名或重新压缩后，必须重新生成大小和摘要。

## 可选蓝奏镜像

同一个 GitHub Release 可附带 `mirrors.json`。清单的 `version` 必须与该发布一致，`files` 中的 `name` 必须与对应 Windows 安装包同名；`share_url` 为真实、无需登录或提取码的公开蓝奏分享链接，`code` 省略或设为空字符串。

程序优先尝试匹配的镜像，并始终按官方发布的 SHA-256 和字节数校验。镜像不存在、无法匿名下载、文件大小或摘要不符时，回退到同一发布的 GitHub 安装包。无匹配镜像时直接使用 GitHub。

发布者可在仓库 Actions 变量 `LANZOU_MIRRORS_JSON` 中配置镜像。使用 [蓝奏公开更新目录](https://www.ilanzou.com/s/PyjrO6mP) 时，先上传 `zsclip-v1.0.7-setup.exe` 和 `zsclip-v1.0.7-setup-no-lan.exe`，再核对两个文件的大小与摘要。目录中没有这两个严格同名安装包时，不启用 1.0.7 镜像配置。完成上传和匿名下载校验后，可执行以下配置：

```powershell
$version = '1.0.7'
$shareUrl = 'https://www.ilanzou.com/s/PyjrO6mP'
$mirrors = @{
    version = $version
    files = @(@{
        name = "zsclip-v$version-setup.exe"
        share_url = $shareUrl
        code = ''
    }, @{
        name = "zsclip-v$version-setup-no-lan.exe"
        share_url = $shareUrl
        code = ''
    })
} | ConvertTo-Json -Depth 4
gh variable set LANZOU_MIRRORS_JSON --repo qiu7824/zsclip --body $mirrors
```

发布工作流只附带版本完全匹配、包含合法公开分享地址的非空镜像清单；未配置或配置属于其他版本时不发布 `mirrors.json`。当前版本配置格式错误、文件名重复或地址不合法时，工作流停止发布。变量内容只包含公开分享信息，不填写账号 Cookie、appToken 或其他登录凭据。

先上传与本次 GitHub 发布字节完全一致的安装包，再配置镜像。首次发布时若尚无可用镜像，可先发布 GitHub 资产，随后将下载并校验过的安装包上传至蓝奏，生成同版本 `mirrors.json` 并作为资产附加到已有发布。仓库变量供后续标签发布使用；仅修改变量不会更新已有 Release。已有版本可将上例 `$mirrors` 保存为 UTF-8 `mirrors.json`，再执行 `gh release upload 1.0.7 mirrors.json --repo qiu7824/zsclip --clobber`。不要为添加镜像重新构建或替换已经发布的安装包。

## 自定义更新源与兼容清单

非空更新源支持公开 HTTPS `zsclip-update-v1` 清单地址，也支持承载该清单的蓝奏公开目录或单文件分享。蓝奏清单文件名固定为 `zsclip-update.json`；目录中只保留一个同名清单。已有蓝奏目录配置仍可使用，网盘管理后台的 `/console/` 地址不能作为更新源。

| 字段 | 内容 |
| --- | --- |
| `format` | 固定为 `zsclip-update-v1` |
| `version` | 三段或四段数字版本号 |
| `page_url` | 公开发布页 HTTPS 地址 |
| `notes` | 更新说明文本 |
| `windows_x64` | 普通 Windows 安装包信息 |
| `windows_x64_no_lan` | 无局域网版本安装包信息，可省略 |

每个安装包条目包含 `url`、`file_name`、`sha256`、`size`。`url` 可以是公开 HTTPS 文件直链，也可以是包含该安装包的蓝奏公开分享链接；使用蓝奏时必须填写准确的 `file_name`。`sha256` 是文件的 SHA-256 摘要，`size` 是文件字节数。发布工作流生成的 `zsclip-update.json` 使用官方 GitHub 安装包地址，可直接作为兼容更新源或上传至蓝奏供旧配置读取。

蓝奏接口要求登录、提取码、付费或人工验证时，程序停止使用该镜像，不保存或分发网盘登录凭据。自定义源若没有可下载的安装包，应在分享页完成平台要求后手动下载，或由发布者将清单中的地址改为公开 HTTPS 文件直链。

## 安装行为

下载完成后校验 SHA-256、字节数及 Windows PE 格式，通过后才启动安装器；安装器取消或失败后，仍在运行的程序恢复重试入口。下载与校验完成前，当前程序保持运行。安装器只退出目标安装目录中的旧版程序，并等待进程和文件占用释放；超时则停止替换，不强制结束其他便携版或外部进程。自动更新完成后重新启动程序，普通交互安装保留运行选项。

## 发布检查

1. 确认安装包版本、文件名、清单版本一致，并保留普通版与无局域网版的对应关系。
2. 确认发布资产包含安装包、`SHA256SUMS.txt` 和 `zsclip-update.json`，校验清单记录的摘要与文件字节数吻合。
3. 配置镜像前上传实际安装包，在未登录网盘的环境下载并核对 SHA-256 和大小；仅发布已验证的分享链接。
4. 使用旧版本分别检查默认源与已有自定义源，核对版本、说明和安装包类型。
5. 检查镜像正常时的下载，以及镜像不可用时回退 GitHub 的行为；确认校验失败的文件不会执行。

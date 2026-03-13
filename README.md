# Win32 Clipboard UI Ported

这是一版继续按原 ZSClip Win32 交互迁移的 Rust 源码包，当前已实现：

- Win32 自绘主窗口（接近原 300x615 布局）
- 顶部标题区、搜索/设置/最小化/关闭按钮
- 搜索框默认关闭，点击搜索按钮展开，再点一次或 `Esc` 关闭
- 二段式标签条（复制记录 / 常用短语）
- 自绘剪贴板列表容器与列表项
- `WM_CLIPBOARDUPDATE` 剪贴板监听
- 文本 / 图片采集
- 单击列表项可按设置执行“仅选中”或“复制并粘贴”
- 托盘小图标（左键显示/隐藏，右键菜单显示/隐藏、退出）
- 全局热键 `Win + V` 显示/隐藏（可在设置里关闭）
- SQLite 持久化记录（优先兼容旧版 `data/clipboard.db`，新安装默认写入 `%LOCALAPPDATA%/ZsClip/data/clipboard.db`）
- 设置窗口（热键开关、单击后隐藏、最大保存条数、分组等）
- 行右键菜单（复制并粘贴 / 仅复制 / 置顶 / 添加到短语 / 删除）
- `Ctrl+C`：仅复制当前项
- `Ctrl+F`：打开搜索
- `Ctrl+P`：置顶 / 取消置顶

## 运行

请在 **Developer PowerShell for VS 2022** 或 **Developer Command Prompt for VS 2022** 中运行：

```powershell
cargo run
```

发布构建：

```powershell
cargo build --release
```

## 数据文件

默认情况下：

- 设置：`%LOCALAPPDATA%/ZsClip/data/settings.json`
- 记录：`%LOCALAPPDATA%/ZsClip/data/clipboard.db`

兼容旧版便携目录：如果 exe 同级存在 `portable.mode`，或旧版 `data/settings.json` / `data/clipboard.db` 已存在，则继续使用 exe 同级 `data/`。

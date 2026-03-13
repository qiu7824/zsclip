# PATCH NOTES

## v6
- 修正单击粘贴后主窗口层级：主窗口改为更接近 Python 版的 topmost/toolwindow 行为；在“单击后隐藏主窗口”关闭时，粘贴后不再强行抢焦点，而是把主窗口重新置于最上层。
- 调整托盘显示与主窗口定位逻辑，保持主窗口以 TopMost 方式显示。
- 微调主界面搜索框 Edit 的字体、位置与高度，改善输入文本的视觉居中。
- 恢复设置页预留入口：快捷键、插件、云同步、关于，以及常规页中的“贴边自动隐藏”“悬停预览”开关；这些入口目前仅保留 UI/配置位，未补运行逻辑。
- 恢复设置页卡片与导航结构，避免后续继续扩展时入口丢失。
- 调整常规页滚动高度，避免恢复预留入口后底部控件被截断。


## v7
- 移除 `image 0.25.x` 依赖，改为直接使用 `png` crate 写出图片文件。
- 规避 `moxcms 0.7.11`（edition 2024）在旧 Rust toolchain 上的编译失败问题。
- 删除 `Cargo.lock`，让目标机器按当前 `Cargo.toml` 重新解析并生成锁文件。

## v9
- 抽出 `src/settings_framework.rs`，把设置页的视口裁剪、顶部遮罩、下拉按钮绘制、下拉弹层等 WinUI 风格逻辑从 `app.rs` 中分离，便于后续在别的窗口复用。
- 修复设置页滚动后顶部出现白色突出的问题：增加内容视口顶部遮罩，并让滚动控件在进入顶部遮罩区前隐藏，避免卡片圆角和子控件穿出标题区。
- 原生 `COMBOBOX` 改为自绘下拉按钮 + 自定义弹层选择窗，避免滚动后下拉不可选的问题，同时把样式调整为更接近你给的图 2 风格。
- 保留快捷键 / 插件 / 云同步 / 关于等预留入口，不再删除后续准备继续完善的功能位。


## v11
- Added build marker and VERSION.txt to help verify the correct source tree is being compiled.
- settings_framework.rs does not contain FreeLibrary/GetCapture/ReleaseCapture/UpdateWindow/SetCapture anymore.

## v12
- 新增 `src/settings_layout.rs`，把设置页卡片尺寸、滚动高度、滚动条尺寸、常规页关键控件 Y 位置从 `app.rs` 中拆出，后续其他窗口可复用同一套布局常量。
- 设置页滚动绘制改为只在“安全绘制区”内绘制内容卡片，避免卡片圆角或浅色背景在标题区边缘露白。
- 设置窗口创建增加 `WS_EX_COMPOSITED`，并把滚动重绘限制在内容视口内，减少滚动时子控件闪烁。
- 自绘下拉弹层改为在鼠标按下时直接提交选择，同时补充非激活弹层处理，修复“下拉展开后能看到但选了没有生效”的问题。
- 设置页输入框改为更清晰的输入视觉：编辑框使用轻量边框，背景改为 `control_bg`，不再与卡片背景混成一片。
- 主界面搜索框增加占位提示、左侧搜索图标、焦点强调边框，并重新调整内边距和 Edit 区域位置，让输入框更像可输入控件。


## v13
- 修复 `src/app.rs` 中遗漏的 Win32 API 导入：`ClientToScreen`、`GetFocus`。
- 这次属于 UI 框架调整后再次出现的导入漏补问题；已改为显式导入，避免再依赖隐式通配导入。


## v14
- 移除对 GetFocus 的依赖，改为由主窗口通过 EN_SETFOCUS / EN_KILLFOCUS 跟踪搜索框焦点状态。
- 保留 ClientToScreen 的现有实现，修复 v13 在 windows-sys 0.52 下的 GetFocus 导入错误。

## v15
- 恢复主界面搜索框的原始风格：撤回搜索图标、占位提示和改动后的 Edit 布局，只保留原本的原生输入框位置与边框绘制。
- 修复设置页下拉“点了没切换”的根因：`WM_SETTINGS_DROPDOWN_SELECTED` 之前误写进了 `input_dlg_proc`，现已移回 `settings_wnd_proc`，点击选项会正确回填到按钮文本。
- 新增 `settings_sync_pos_fields_enabled()`：切换“弹出位置”时，立即联动启用/禁用对应的 dx/dy、x/y 输入框，便于后续继续扩展设置页逻辑。
- 设置页输入框继续保留原生 EDIT 边框，并显式增加左右内边距，避免和卡片背景混在一起。


## v16
- Restored the main search box back to the v11 style and removed the later search-box visual changes.
- Fixed settings dropdown application chain by keeping the selection handler in settings_wnd_proc and correcting show_pos_mode logic to use "mouse" for follow-mouse mode.
- Kept the settings input-box visibility improvements only for the settings window.

## v17
- 主界面搜索框恢复到 v11 的位置、字体与边距。
- 弹窗位置逻辑独立拆到 `window_position.rs`，让 `显示位置` 下拉真正决定实际弹出位置；热键只作为回退策略，不再强行覆盖 `fixed/last/mouse`。
- 设置窗口移除 `WS_EX_COMPOSITED`，并把设置下拉弹层改为直接绘制，减少额外工作集与绘制层。


## v17b
- 修复 tray.rs 缺少 resolve_main_window_position 导入
- 修复 tray.rs 缺少 null_mut 导入
- 清理 tray.rs 未使用导入与 app.rs 未使用变量 warning

## v18
- 数据库运行时新增 `db_runtime.rs`：改为线程内复用单个 SQLite 连接，并用 `OnceLock` 只执行一次建表/迁移，避免每次 DB 操作都重复 `ensure_db + open + ALTER TABLE`。
- 时间格式化工具抽到 `time_utils.rs`：统一本地时区偏移计算，去掉图片预览时间与创建时间格式化中的重复实现。
- `send_ctrl_v()` 改为 `SendInput`，替换已废弃的 `keybd_event`。
- 粘贴目标窗口查找去掉标题黑名单，优先用 `GetGUIThreadInfo` / 前台根窗口判定，回退时只筛选可见、可用、非工具窗口。
- 输入对话框与文本编辑对话框不再返回 `WHITE_BRUSH`：改为缓存与当前主题匹配的 `surface/control/gutter` 画刷，减少深色模式闪白。
- 设置窗口新增 `settings_refresh_theme_resources()`，在 `WM_THEMECHANGED / WM_SETTINGCHANGE` 时重建 brush，修复打开设置窗口后切换主题颜色过期。
- `settings_create_small_btn()` 合并为对 `settings_create_btn()` 的复用，去掉重复实现。
- `ensure_sticker_class()` 改用 `OnceLock`，不再使用 `static mut DONE`。
- app.rs 内两个 `#[link(name = "user32")]` extern 块已合并。
- 架构上新增 `db_runtime.rs` 与 `time_utils.rs` 两个模块，先把数据库运行时和时间工具从 `app.rs` 拆出；主窗口/设置窗口/对话框 WndProc 仍在 `app.rs`，后续可继续拆 `dialogs.rs / settings_window.rs / clipboard.rs`。


## v18b
- 修复 `IsWindowEnabled` 在当前 windows-sys 版本下不可用，改为基于 `GetWindowLongW + WS_DISABLED` 的兼容判断
- 为 `KEYBDINPUT / INPUT_UNION / INPUT` 补上 `Copy, Clone`，修复 `union` 字段约束
- 显式指定 `rusqlite` 的 `String` 读取类型，修复 `r.get(0)` 推断失败


## v18c
- 修复 `db_prune_items()` 中 `COUNT(*)` 被误写为 `String` 导致的 `E0308`。
- 修复编辑对话框加载文本时 `query_row` 缺少显式返回类型导致的 `E0282`。

## v19
- 全局热键注册失败时，检测 `ERROR_HOTKEY_ALREADY_REGISTERED`，只弹一次冲突提示，不做重复重试，也不增加常驻对象。
- 设置窗口滚动改为 `ScrollWindowEx + 局部失效区域`，避免每次滚动都整块重绘内容视口，减轻滚动闪烁。
- 数据目录改回 `exe/data`，不再优先写入 `%LOCALAPPDATA%`。

## v20
- 按方案 A 去掉设置窗口 `WS_EX_COMPOSITED` 路线，避免拖慢滚动流畅度。
- 设置页滚动改为：父窗口 `ScrollWindowEx(..., 0)` 滚动已有像素 + `DeferWindowPos` 批量移动子控件，不使用 `SW_SCROLLCHILDREN`。
- 设置页仍统一 `WM_ERASEBKGND => 1`，并只对新暴露条带、顶部遮罩区、滚动条区做 `InvalidateRect(..., FALSE)`。
- 设置页主绘制改为 `BufferedPaint`，不再叠加 `CreateCompatibleDC/CreateCompatibleBitmap/BitBlt` 那套离屏位图。
- 新增 `src/win_buffered_paint.rs`，把官方 `BufferedPaint` 初始化与调用单独抽出来，便于后续其它窗口复用。


## v20b
- 停用设置页滚动中的 ScrollWindowEx 像素复制，改为精确 Invalidate + 立即 RedrawWindow。
- 子控件重定位增加 SWP_NOCOPYBITS，并对旧/新区域逐块失效，修复滚动白框残留。
- 子控件移动后立即 RedrawWindow，避免滚动过程中白色占位块晚一拍绘制。

## v20c
- 去掉 settings_scroll_to/settings_repos_controls 中新增的 `RedrawWindow` / `RDW_*` 依赖，改回兼容的 `InvalidateRect(..., FALSE)`，修复 windows-sys 0.59 下的编译错误。


## v21
- 清理 v20c 的 7 个 warning（unused variable/const/extern、FFI struct non_snake_case）。
- 主界面右上角 4 个按钮改为按标题栏高度垂直居中，不再使用硬编码 `+8/+7`。
- 顶部两个 tab 改为更接近 WinUI/Windows 的“中性色层级”选中态：选中项使用更浅/更高层级表面色，不再用明显强调色填充；选中文字加粗，未选中文字减弱。

- v22: 主界面 tab 改为深浅区分不加粗；补了快捷键页与插件页核心设置；接通快速搜索；新增文本/图片导出为文件。


## v22b
- 修复 quick_search_open 中错误换行字符字面量导致的编译错误。
- 补上热键修饰键常量 MOD_ALT / MOD_CONTROL / MOD_SHIFT，兼容 windows-sys 0.59。


## v23
- 修复设置页控件串页：快捷键/插件/分组页的 toggle label 不再错误注册到常规页滚动控件集合。
- 新增 settings_create_toggle_plain()，把“控件创建”和“page0 滚动注册”解耦，减少后续页面布局混乱。
- 补齐快捷键页/插件页 toggle 的 owner-draw 注册。
- 清理分组页残留死代码。


## v24
- 设置页引入第一阶段 UI Registry：新增 `settings_registry.rs`，统一登记控件所属页面，页面切换不再靠分散的 `push` 逻辑。
- 修复快捷键页卡片高度不足导致的文字越界。
- 新增多行自适应标签测量与创建，长说明文本自动按宽度计算高度。
- 参考 `zsclip.zip` 补齐快捷键页的系统剪贴板历史（Win+V）功能：
  - 显示 DisabledHotkeys 状态
  - 屏蔽 Win+V
  - 恢复 Win+V
  - 重启资源管理器
- 保留现有快捷键启用/修饰键/按键/预览逻辑，并继续使用保存后即时重新注册。

- v24b: 补上 app.rs 中使用的 GetDC / ReleaseDC extern 声明，修复快捷键页自适应文本高度测量编译失败。

## v25
- 完整的 Settings UI 框架化第一版：引入 SettingsPage / SettingsUiRegistry，统一页面归属与控件注册。
- 设置页改为按页面惰性创建：首次打开仅创建常规页，其余页面在切换时再创建，减轻打开卡顿。
- 页面切换统一走 settings_ensure_page + settings_show_page，不再预创建全部页面。
- 滚动页与静态页注册逻辑统一收敛到 registry，减少串页和排版互相污染。


## v26
- 修复快捷键页系统剪贴板历史状态在惰性创建后不刷新的问题
- 调整分组页第二张卡片的正文布局，避免正文控件压进标题区
- 增加 settings_sync_page_state，继续收紧 Settings UI 框架的页面创建/显示/同步链路


## v26b
- 修复快捷键页系统剪贴板历史状态不再卡在“读取中...”，创建后和切页后都会立即刷新状态文本。
- 调整分组页第二张卡片的起始位置与正文控件纵向布局，避免“当前分组”和“分组管理”标题区域重叠。
- 清理 settings_registry.rs 的 dead_code warning。

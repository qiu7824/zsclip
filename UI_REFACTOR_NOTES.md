# UI Refactor Notes

## Step 1
- 设置页改为 schema 驱动，页面卡片由静态结构描述。

## Step 2
- 将设置页中的 Toggle / Dropdown / Button 的创建与绘制统一收口到 `settings_framework.rs`
- 新增 `SettingsComponentKind`
- 新增 `create_settings_component`
- 新增 `draw_settings_toggle_component`
- 新增 `draw_settings_button_component`

## Win+V 系统剪贴板历史状态
- 为避免进入设置页时同步读取注册表带来的卡顿，已取消实时状态展示
- 保留“屏蔽 Win+V / 恢复 Win+V / 重启资源管理器”操作按钮
- 说明文字中明确提示：修改后通常需要重启资源管理器或重新登录


## step3（本次新增）
- 修复设置页切换时的闪烁：
  - 切页时临时关闭父窗口重绘（WM_SETREDRAW）
  - 只隐藏旧页、只显示新页，不再遍历全量控件做无差别闪现
  - 同步页面状态时只刷新当前页控件，不再把其他页 ownerdraw 一起失效重绘
- 继续 UI 框架化：
  - 新增 `SettingsPageBuilder`，统一 page 内的 label / label_auto / button / dropdown / edit / toggle_row 创建入口
  - 热键页、插件页已切到 builder 风格，后续可继续把分组页和常规页迁过去
- 跨平台迁移收益：
  - 当前是“页面描述/组件创建入口”在收口，迁移到其他平台时更容易保留设置项结构和业务绑定
  - 但底层仍然是 Win32 HWND/消息机制，自绘和窗口管理部分还不能直接跨平台复用

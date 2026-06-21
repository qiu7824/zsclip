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

## step4
- 继续拆设置页 host/render 与平台无关模型：
  - 将设置页 timer 任务类型与 id 映射从 Windows host 移到 `app_core`
  - 新增 `settings_nav_render_plan`，由 `settings_model` 输出左侧导航项的页面、矩形、选中/悬停和更新提示点
  - Windows 渲染层只负责把导航 plan 映射成 Fluent 图标和 GDI 绘制
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step5
- 继续压缩设置页 `WM_PAINT` 中的平台耦合：
  - 新增 `SettingsChromeRenderPlan`，由 `settings_model` 描述左侧栏、分割线、标题、内容裁剪区和遮罩分割线
  - 新增 `SettingsScrollbarRenderPlan`，由 `settings_model` 描述滚动条普通/拖拽状态、轨道和滑块矩形
  - Windows 渲染层新增 `draw_settings_chrome`、`draw_settings_viewport_mask`、`draw_settings_scrollbar`，只消费 render plan 并映射为 GDI 绘制
  - 删除不再使用的 `settings_title_rect_win`、`nav_divider_x`、`settings_safe_paint_rect` 包装函数
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step6
- 继续收口设置页内容区域绘制：
  - 新增 `SettingsContentRenderPlan` 和 `SettingsContentSource`
  - 将“插件页使用动态插件卡片 / 多端同步页使用动态同步卡片 / 其他页面使用静态卡片”的选择规则移入 `settings_model`
  - Windows 渲染层改为 `draw_settings_content`，统一消费内容 render plan
  - `SettingsSection` 增加 `Debug / PartialEq / Eq`，便于模型层 plan 测试
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step7
- 继续收口设置页交互路径：
  - 新增 `SettingsNavHoverTransition`
  - 将左侧导航 hover 的命中、下一状态和重绘矩形计算移入 `settings_model`
  - `handle_settings_pointer_move` / `handle_settings_pointer_leave` 改为消费模型层 transition
  - 删除 `settings_wnd_proc` 中已被统一 `platform_ui_event -> dispatch_settings_ui_event` 覆盖的旧 Win32 鼠标/键盘重复分支
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step8
- 继续收口设置页按下命中逻辑：
  - 新增 `SettingsPointerDownTarget`
  - 将导航点击、滚动条拇指拖拽、滚动条轨道点击的目标判定移入 `settings_model`
  - `handle_settings_lbutton_down` 保留平台弹窗处理和动作执行，只消费模型层 target
  - 删除不再需要的 Win32 `settings_scrollbar_thumb_w` 包装
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor settings_pointer_down_target`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step9
- 继续收口设置页指针移动逻辑：
  - 新增 `SettingsPointerMoveTransition`
  - 将“滚动条拖拽优先消费 pointer move / 非拖拽才更新导航 hover”的事件优先级移入 `settings_model`
  - `handle_settings_pointer_move` 改为消费模型层 transition，再执行滚动与失效重绘
  - ownerdraw 子控件 hover 仍保留在 Windows 层，因为它依赖 HWND 子窗口命中
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor settings_pointer_move_transition`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step10
- 补齐主窗口模型层捕获开关的 Windows 接线：
  - `MainRenderInput` 的捕获状态、hover/down 状态已接入主窗口 paint
  - `MainPointerDownTarget::CaptureToggle` 已接入主窗口按下/释放路径
  - 释放后切换 `clipboard_capture_enabled` 并持久化设置
  - `capture_clipboard` 在捕获关闭时直接跳过，避免 UI 状态和后台行为不一致
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step11
- 继续收口设置页滚动状态更新：
  - 新增 `SettingsScrollUpdate`
  - 新增 `settings_scroll_update_for_target`，由 `settings_model` 负责目标滚动值 clamp 和当前页缓存同步计划
  - `app::hosts::settings_scroll_to` 改为消费模型层 update，Windows host 只执行控件 reposition、滚动条显示和重绘失效
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor settings_scroll_update_for_target`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step12
- 继续收口设置页切页状态：
  - 新增 `settings_normalized_page_index`
  - 新增 `SettingsPageSwitchPlan` / `SettingsPageSwitchMode` / `SettingsPageSwitchScrollState`
  - 将目标页归一化、同页已构建时只同步、离开热键页取消录制、切页时恢复目标页滚动缓存和重置滚动条显示的规则移入 `settings_model`
  - `app::hosts::settings_show_page` 改为消费模型层切页 plan，Windows host 保留 HWND 显隐、控件构建、重绘和下拉销毁等平台动作
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor settings_page_switch_plan`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step13
- 修正剪贴板捕获开关入口位置：
  - 新增 `TRAY_CAPTURE_TOGGLE` 菜单 id
  - 托盘图标右键菜单新增“剪贴板捕获：开/关”
  - `execute_main_menu_command` 通过托盘菜单切换 `clipboard_capture_enabled` 并保存设置
  - 移除主窗口里的 `CaptureToggle` 绘制、hover、按下和释放入口
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor menu_ids_map_to_platform_neutral_window_commands`
  - `cargo test -j 1 --target-dir target_ui_refactor main_menu_ids_map_to_stable_commands`
  - `cargo test -j 1 --target-dir target_ui_refactor main_layout_core_maps_pointer_targets_without_host_state`
  - `cargo test -j 1 --target-dir target_ui_refactor main_render_plan`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step14
- 继续分离主窗口指针移动状态：
  - 新增 `MainPointerMoveTransition` / `MainHoverTransition`
  - `MainUiLayout::pointer_move_transition` 负责计算滚动条拖动目标、下一 hover target、是否需要重绘、是否需要隐藏 hover 预览、是否需要展开滚动条反馈
  - `handle_mouse_move` 改为消费模型层 transition，Windows host 保留鼠标离开追踪、按键/系统拖拽阈值、行拖拽导出、滚动条反馈和重绘等平台动作
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor main_pointer_move_transition_describes_hover_and_drag_without_host_state`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step15
- 继续分离主窗口指针释放状态：
  - 新增 `MainPointerUpTransition` / `MainPointerUpTarget` / `MainRowRelease`
  - `MainUiLayout::pointer_up_transition` 负责判断标题按钮释放是否激活、回到顶部释放是否激活、行释放是否仍落在按下行
  - `handle_lbutton_up` 改为消费模型层 release target，Windows host 保留命令入队、快速删除、选择修饰键、粘贴和重绘等平台动作
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor main_pointer_up_transition_describes_release_targets_without_host_state`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step16
- 继续分离主窗口行释放语义：
  - 新增 `MainPointerModifiers` / `MainRowReleaseAction`
  - `MainUiLayout::row_release_action` 负责将已接受的行释放映射为 `QuickDelete` / `Select` / `Paste` / `None`
  - 快速删除按钮命中、ctrl/shift 选择、普通点击粘贴的优先级进入平台无关模型
  - `handle_lbutton_up` 改为消费 action plan，Windows host 保留数据删除、修饰键采集、粘贴执行和重绘等平台动作
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor main_row_release_action_prioritizes_quick_delete_selection_and_paste`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step17
- 继续分离主窗口 tab 切换状态：
  - 新增 `MainTabSwitchPlan`
  - `ClipListState::tab_switch_plan` / `apply_tab_switch_plan` 负责归一化目标 tab、恢复该 tab 的分组过滤、清空选择/hover/context 和重置滚动
  - `handle_lbutton_down` 的 tab 分支改为消费模型层 plan，Windows host 保留 `refilter`、共享 tab 状态记忆和重绘等执行动作
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor main_tab_switch_plan_updates_list_state_without_host_actions`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step18
- 继续分离主窗口滚动更新状态：
  - 新增 `MainScrollUpdate`
  - `MainUiLayout::scroll_update_for_target` / `scroll_update_for_wheel` / `scroll_update_for_track_click` 负责滚动目标 clamp 和 changed 标记
  - `handle_mouse_wheel` 与滚动条轨道点击分支改为消费模型层 scroll update
  - Windows host 保留按需加载、滚动条反馈和重绘等执行动作
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor main_layout_core_scrollbar_math_is_platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step19
- 继续分离主窗口指针按下状态：
  - 新增 `MainPointerDownStatePlan` / `MainScrollDragStart`
  - `MainUiLayout::pointer_down_state_plan` 负责把命中目标映射为标题按钮按下、回到顶部按下、行按下和滚动条拖拽起点等状态更新
  - `handle_lbutton_down` 改为消费模型层 down-state plan，Windows host 保留窗口拖动、鼠标捕获、资源管理器直写、数据刷新和重绘等执行动作
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor main_pointer_down_state_plan_describes_press_state_without_host_actions`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step20
- 继续分离主窗口 hover 清理状态：
  - 新增 `MainHoverClearTransition`
  - `MainHoverTarget::clear_transition` 负责将当前 hover 状态清理为默认状态，并支持保留或清除滚动条 hover
  - `handle_mouse_leave_main` 与 `clear_main_hover_state` 改为消费模型层 hover-clear transition
  - Windows host 保留隐藏 hover 预览、边缘隐藏追踪、按下态清理和重绘等执行动作
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor main_hover_clear_transition_can_preserve_or_clear_scrollbar_hover`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step21
- 继续分离主窗口分组过滤状态：
  - 新增 `MainGroupFilterPlan`
  - `ClipListState::group_filter_plan` / `apply_group_filter_plan` 负责切换目标 tab、更新对应 tab 的分组过滤、清空选择/hover/context 和重置滚动
  - 主菜单分组过滤与右键 tab 分组过滤入口改为消费模型层 plan
  - Windows host 保留菜单弹出、group id 查询、数据过滤刷新、共享 tab 状态记忆和重绘等执行动作
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor main_group_filter_plan_updates_target_tab_without_host_actions`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step22
- 继续分离主窗口键盘选择状态：
  - 新增 `MainSelectionPlan`
  - `ClipListState::keyboard_move_selection_plan` / `apply_selection_plan` 负责上下移动选择、Shift 扩展范围和 selection anchor 更新
  - `move_main_selection` 改为消费模型层 plan，Windows host 保留 ensure-visible 和重绘等执行动作
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor main_keyboard_selection_plan_moves_and_extends_without_host_actions`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step23
- 继续分离主窗口快捷键选择状态：
  - 将 selection plan 泛化为 `MainSelectionPlan`
  - 新增 `ClipListState::select_all_selection_plan`，由模型层负责 `Ctrl+A` 的可见行全选、selection anchor 更新，并保留当前焦点行
  - `MainShortcutAction::SelectAll` 改为消费模型层 plan，Windows host 只负责重绘失效
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor main_select_all_selection_plan_selects_visible_rows_without_host_actions`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step24
- 继续分离主窗口 Escape 快捷键决策：
  - 新增 `MainShortcutEscapePlan`
  - `ClipListState::escape_shortcut_plan` 负责决定 Escape 应清理多选、关闭搜索，还是隐藏窗口
  - `MainShortcutAction::Escape` 改为消费模型层 plan，Windows host 只执行清理状态、关闭搜索控件或窗口隐藏命令
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor main_escape_shortcut_plan_prioritizes_selection_search_then_window`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step25
- 继续分离主窗口 Enter 激活选择决策：
  - 新增 `MainActivateSelectionPlan`
  - `ClipListState::activate_selection_plan` 负责决定 Enter 是单项粘贴，还是多选合并复制后粘贴
  - `MainShortcutAction::ActivateSelection` 改为消费模型层 plan，Windows host 保留剪贴板写入、粘贴执行和窗口隐藏等平台动作
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor main_activate_selection_plan_uses_combined_paste_only_for_multi_selection`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step26
- 继续分离主窗口快捷键行命令决策：
  - 新增 `MainShortcutRowCommand` / `MainShortcutRowCommandPlan`
  - `main_shortcut_row_command_for_action` 负责将 `CopySelection` / `DeleteSelection` / `TogglePin` 映射为平台无关行命令
  - `ClipListState::shortcut_row_command_plan` / `apply_shortcut_row_command_plan` 负责将当前焦点行作为快捷键行命令的 context row
  - `MainShortcutAction::CopySelection` / `DeleteSelection` / `TogglePin` 改为消费模型层行命令 plan，Windows host 只负责推入 window command 并执行
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor main_shortcut_row_commands_map_to_stable_window_commands`
  - `cargo test -j 1 --target-dir target_ui_refactor main_shortcut_row_command_plan_uses_focused_row_as_context`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step27
- 继续分离主窗口快捷键窗口命令决策：
  - 新增 `MainWindowCommandIntent`
  - `main_shortcut_window_command_for_action` 负责将 `ToggleSearch` 映射为平台无关窗口命令
  - `main_window_command_for_intent` 负责生成稳定 window command，供搜索切换和 Escape 隐藏窗口分支复用
  - `MainShortcutAction::ToggleSearch` 与 Escape 的 `HideWindow` 分支改为消费模型层 window command，Windows host 只负责入队和执行
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor main_shortcut_window_commands_map_to_stable_window_commands`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step28
- 继续分离主窗口标题按钮窗口命令决策：
  - 将窗口命令语义泛化为 `MainWindowCommandIntent`
  - 新增 `main_title_button_window_command_for_key`，由模型层负责将 `search` / `setting` / `min` / `close` 映射为稳定窗口命令语义
  - 标题按钮释放分支改为消费模型层 window command intent，Windows host 保留入队、执行和重绘等平台动作
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor main_title_buttons_map_to_stable_window_command_intents`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step29
- 继续分离主窗口回到顶部释放状态：
  - 新增 `MainScrollToTopReleasePlan`
  - `ClipListState::scroll_to_top_release_plan` / `apply_scroll_to_top_release_plan` 负责释放后的按下态清理、滚动位置更新和滚动条反馈计划
  - `MainPointerUpTarget::ScrollToTop` 分支改为消费模型层 release plan，Windows host 保留滚动条反馈显示和重绘等平台动作
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor main_scroll_to_top_release_plan_resets_scroll_without_host_actions`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step30
- 继续分离主窗口行释放状态：
  - 新增 `MainRowReleaseStatePlan`
  - `ClipListState::row_release_state_plan` / `apply_row_release_state_plan` 负责行释放后的按下态清理和焦点行更新
  - `MainPointerUpTarget::Row` 分支改为先消费模型层 state plan，Windows host 保留 quick delete、选择、粘贴和重绘等执行动作
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor main_row_release_state_plan_clears_press_and_updates_focus_without_host_actions`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step31
- 继续分离主窗口 pointer-up 空目标状态：
  - 新增 `MainPointerUpPressClearPlan`
  - `ClipListState::pointer_up_press_clear_plan` 负责描述无目标释放后的按下态清理
  - `MainPointerUpTarget::None` 分支改为消费模型层 clear plan，Windows host 不再直接决定 `down_row/down_x/down_y` 的清理值
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor main_pointer_up_press_clear_plan_clears_press_without_changing_focus`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step32
- 继续分离主窗口双击行状态：
  - 新增 `MainRowDoubleClickStatePlan`
  - `ClipListState::row_double_click_state_plan` / `apply_row_double_click_focus_plan` / `apply_row_double_click_finish_plan` 负责双击行粘贴前焦点行设置和粘贴后 hover/focus 清理
  - `handle_lbutton_dblclk` 改为消费模型层 double-click state plan，Windows host 保留命中检测、粘贴执行和重绘等平台动作
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor main_row_double_click_state_plan_focuses_then_clears_without_host_actions`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step33
- 继续分离主窗口右键行菜单上下文状态：
  - 新增 `MainContextMenuStatePlan` / `MainContextRowSelectionPlan` / `MainContextMenuFinishPlan`
  - `ClipListState::context_menu_state_plan` / `apply_context_menu_state_plan` 负责右键行菜单打开前的选区、context row 和菜单选择数量快照
  - `ClipListState::context_row_selection_plan` / `apply_context_row_selection_plan` 负责执行行菜单命令前把 context row 映射为当前焦点行
  - `ClipListState::context_menu_finish_plan` / `apply_context_menu_finish_plan` 负责菜单命令结束后的 context row 清理
  - `handle_rbutton_up` / `select_context_row` / `execute_main_menu_command` 改为消费模型层 context plan，Windows host 保留命中检测、菜单展示、命令执行和重绘等平台动作
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor main_context_menu_state_plan_tracks_row_context_without_host_actions`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step34
- 继续分离主窗口滚动状态应用：
  - 新增 `ClipListState::scroll_position_update_plan` / `apply_scroll_update`
  - 滚轮、滚动条拖拽和滚动条轨道点击路径改为消费模型层 scroll update，Windows host 保留按需加载、滚动条反馈和鼠标捕获等平台动作
  - `MainScrollUpdate::changed` 现在也可由 list 状态生成，供后续触摸板/移动端滑动入口复用
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor main_scroll_update_plan_applies_position_without_host_actions`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step35
- 继续分离主窗口行按下焦点状态：
  - 新增 `MainRowPointerDownFocusPlan`
  - `ClipListState::row_pointer_down_focus_plan` / `apply_row_pointer_down_focus_plan` 负责左键按下行时的焦点行有效性判断和焦点更新
  - `handle_lbutton_down` 的行分支改为消费模型层 focus plan，Windows host 保留资源管理器重命名直写、按下态、鼠标修饰键和重绘等平台动作
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor main_row_pointer_down_focus_plan_updates_focus_without_host_actions`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step36
- 继续分离主窗口搜索状态：
  - 新增 `MainSearchFilterApplyPlan` / `MainSearchResetPlan`
  - `ClipListState::search_filter_apply_plan` / `apply_search_filter_plan` 负责搜索过滤应用后的焦点行和滚动复位
  - `ClipListState::search_reset_plan` / `apply_search_reset_plan` 负责搜索关闭/重置时的搜索框状态和选区清理
  - `apply_search_filter` / `reset_search_ui_state` 改为消费模型层 search plan，Windows host 保留停止 timer、同步 Win32 edit 文本、refilter、layout 和重绘等平台动作
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor main_search_filter_apply_plan_resets_focus_and_scroll_without_host_actions`
  - `cargo test -j 1 --target-dir target_ui_refactor main_search_reset_plan_clears_query_and_selection_without_host_actions`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step37
- 继续收口主窗口图标渲染语义：
  - `draw_main_icon_command` 改为消费模型层 `MainIconColorMode`
  - 主窗口 render plan 测试改为断言 `Original` / `ThemeAware` 语义，而不是旧的 host 字段
  - Windows host 只负责把 `MainIconColorMode` 翻译为 GDI 图标 tint 参数
- 验证：
  - `cargo check -j 1 --target-dir target_ui_refactor`
  - `cargo test -j 1 --target-dir target_ui_refactor main_render_plan_describes_visible_regions_without_host_renderer`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step38
- 继续分离托盘与热键窗口状态决策：
  - 新增 `MainWindowVisibilityPlan`，将托盘“显示/隐藏”中的快捷窗口隐藏、边缘隐藏恢复、主窗口显示/隐藏顺序移入 `app_core`
  - 新增 `MainWindowHotkeyVisibilityPlan`，将全局热键中的快捷窗口优先、主窗口次之、都隐藏时打开快捷窗口的策略移入 `app_core`
  - `tray.rs` 只消费模型层 step 并调用 Win32/platform wrapper 执行显示、隐藏、恢复和焦点刷新
  - 这部分后续可复用于 macOS 菜单栏入口和全局快捷键入口，平台层只需要实现窗口可见性、显示/隐藏与定位
- 验证：
  - `cargo test window_toggle_visibility_plan_preserves_edge_restore_fallback_order --target-dir target_ui_isolate_window_visibility`
  - `cargo test hotkey_visibility_plan_prefers_quick_then_main_then_show_quick --target-dir target_ui_isolate_hotkey_visibility`
  - `cargo check --target-dir target_ui_isolate_hotkey_visibility`

## step39
- 继续分离主窗口定位策略：
  - 新增 `MainWindowPositionMode` / `MainWindowPositionInput` / `MainWindowPositionPlan`
  - 将固定位置、上次位置、鼠标附近、居中，以及热键打开时 Last/Center 回退到鼠标附近的规则移入 `app_core`
  - 新增 `main_window_position_anchor`，保留旧逻辑中“先算候选位置，再按候选位置所在显示器工作区 clamp”的行为
  - `tray.rs` 只负责读取当前设置、查询光标和显示器边界、调用平台窗口定位 API
- 验证：
  - `cargo test window_position_plan_uses_mode_fallbacks_and_clamps_to_bounds --target-dir target_ui_isolate_position`
  - `cargo check --target-dir target_ui_isolate_position`

## step40
- 继续分离边缘隐藏窗口的位置状态：
  - 新增 `MainEdgeRestorePositionInput` / `main_edge_restore_position`
  - 将边缘隐藏恢复时“有效 dock restore 优先，其次 last window”的规则移入 `app_core`
  - 新增 `MainRememberWindowPositionInput` / `main_remember_window_position`
  - 将窗口位置保存时“边缘隐藏状态保存 restore 坐标，否则保存当前窗口坐标”的规则移入 `app_core`
  - `tray.rs` 保留窗口矩形读取和设置写回，状态决策由核心层承担
- 验证：
  - `cargo test edge_restore_and_remember_position_are_platform_neutral --target-dir target_ui_isolate_edge_position`
  - `cargo check --target-dir target_ui_isolate_edge_position`

## step41
- 继续分离主窗口/快捷窗口显示前状态：
  - 新增 `MainShowWindowStateInput` / `MainShowWindowStatePlan`
  - 将显示窗口时清理边缘隐藏状态、选择 edge dock 后续动作、主窗口清空 passthrough、快捷窗口热键模式使用前台快照的规则移入 `app_core`
  - 将纯文本粘贴模式归属收口到模型层：主窗口显示总是清空，快捷窗口沿用调用方输入
  - `tray.rs` 新增小执行器消费 plan，保留 HWND、前台焦点、资源管理器重命名编辑框等 Windows 查询
- 验证：
  - `cargo test show_window_state_plan_separates_main_and_quick_passthrough --target-dir target_ui_isolate_show_state`
  - `cargo check --target-dir target_ui_isolate_show_state`

## step42
- 继续分离全局热键注册策略：
  - 新增 `MainHotkeyModifiers` / `MainHotkeyKey` / `MainHotkeySpec`
  - 新增 `MainHotkeyRegistrationInput` / `MainHotkeyRegistrationPlan`
  - 将“已注册则先注销、禁用则不再注册、启用则生成规范化热键 spec”的规则移入 `app_core`
  - `app.rs` 只负责把平台无关 spec 映射为 Windows `MOD_*` / `VK_*` 并调用注册 API
  - 主热键继续保留冲突提示，纯文本粘贴热键继续静默失败
- 验证：
  - `cargo test hotkey_registration_plan_normalizes_platform_neutral_specs --target-dir target_ui_isolate_hotkey_plan`
  - `cargo check --target-dir target_ui_isolate_hotkey_plan`

## step43
- 继续分离主窗口/快捷窗口显示前准备流程：
  - 新增 `MainShowPrepareInput` / `MainShowPreparePlan`
  - 将共享 tab 变化后的清选择、滚动归零、重滤规则移入 `app_core`
  - 将显示窗口时“持久搜索框则显示，否则重置搜索”的决策移入 `app_core`
  - `tray.rs` 新增执行器消费 plan，Windows 层只负责调用现有 search/layout/invalidating 执行动作
  - `show_main_window` 与 `show_quick_window` 共用同一个准备函数，减少两条显示路径的重复状态逻辑
- 验证：
  - `cargo test show_prepare_plan_combines_shared_tab_and_search_state --target-dir target_ui_isolate_show_prepare`
  - `cargo check --target-dir target_ui_isolate_show_prepare`

## step44
- 继续分离搜索框打开/关闭状态：
  - 新增 `MainSearchVisibilityRequest` / `MainSearchVisibilityPlan`
  - 将 Toggle 到 Open/Close 的决策、持久搜索框关闭行为、普通关闭时清文本/清选择/重滤/停止 debounce 的规则移入 `app_core`
  - `app.rs` 只负责执行 plan：窗口激活、焦点设置、Win32 edit 文本同步、layout 和 invalidate
  - 删除抽离后不再使用的 `open_search_ui` 包装函数
- 验证：
  - `cargo test search_visibility_plan_describes_toggle_open_and_close --target-dir target_ui_isolate_search_visibility`
  - `cargo check --target-dir target_ui_isolate_search_visibility`

## step45
- 继续分离主窗口快捷键执行分派：
  - 新增 `MainShortcutExecutionPlan`
  - 将 `MainShortcutAction` 到移动选择、激活选择、全选、行命令、窗口命令、Escape 子动作的分派规则移入 `app_core`
  - `app.rs` 只消费 execution plan 并执行数据/剪贴板/窗口平台动作
  - 具体复制、删除、粘贴等行为保持在 Windows host 执行层，本轮不改变业务效果
- 验证：
  - `cargo test shortcut_execution_plan_routes_actions_without_platform_state --target-dir target_ui_isolate_shortcut_exec`
  - `cargo check --target-dir target_ui_isolate_shortcut_exec`

## step46
- 继续分离托盘菜单命令语义：
  - 新增 `MainTrayActionInput` / `MainTrayActionPlan`
  - 将托盘菜单动作到“切换窗口、设置剪贴板捕获目标状态、设置局域网同步目标状态、退出”的规则移入 `app_core`
  - `app.rs` 只消费 tray action plan，并执行保存设置、刷新 LAN 服务、重置剪贴板 retry、销毁窗口等平台/运行时副作用
  - 这为后续 macOS 菜单栏复用相同菜单语义留出入口
- 验证：
  - `cargo test tray_action_plan_toggles_state_without_host_side_effects --target-dir target_ui_isolate_tray_action`
  - `cargo check --target-dir target_ui_isolate_tray_action`

## step47
- 继续分离主窗口 host action 执行映射：
  - 新增 `MainHostExecutionPlan`
  - 将 `MainHostAction` 到搜索可见性请求、打开设置、隐藏窗口、关闭窗口、执行菜单命令的映射移入 `app_core`
  - `app.rs` 只消费 execution plan 并执行 Win32 窗口操作或菜单命令副作用
  - 这为后续 macOS 窗口 host 复用同一套命令执行语义提供入口
- 验证：
  - `cargo test host_actions_map_to_platform_execution_plans --target-dir target_ui_isolate_host_exec`
  - `cargo check --target-dir target_ui_isolate_host_exec`

## step48
- 继续收紧 Windows 平台热键边界：
  - 将 `MainHotkeySpec` 到 Win32 `MOD_*` / `VK_*` 的翻译从 `app.rs` 移到 `platform::hotkey`
  - 将主窗口快捷键 `VK_*` 到 `ShortcutKey` 的翻译、设置页录制热键 `VK_*` 到显示标签的翻译一并移到 `platform::hotkey`
  - 将设置页录制热键的修饰键状态读取/标签拼接、VV 数字选择和修饰键判断一并收口到 `platform::hotkey`
  - 将主窗口快捷键、鼠标行选择、上下文菜单、VV 命令修饰键、编辑框 Ctrl+S 和 AI 清洗 Shift 跳过的按键状态读取封装为平台层输入快照
  - `app.rs` 只负责消费核心层注册计划并调用平台热键注册 API
  - 新增平台层映射单测，覆盖 modifier 组合、字符键、特殊键、主窗口快捷键、设置页录制标签、VV 数字/修饰键辅助判断和核心 modifier 类型生成
  - 这让后续 macOS host 可以提供自己的热键编码实现，而不需要复制主窗口业务层
- 验证：
  - `cargo test hotkey_specs_map_to_win32_modifiers_and_vk_codes --target-dir target_ui_isolate_platform_hotkey`
  - `cargo test main_window_virtual_keys_map_to_shortcut_keys --target-dir target_ui_isolate_platform_hotkey`
  - `cargo test settings_hotkey_virtual_keys_map_to_option_labels --target-dir target_ui_isolate_platform_hotkey`
  - `cargo test settings_hotkey_modifier_state_maps_to_option_labels --target-dir target_ui_isolate_platform_hotkey`
  - `cargo test virtual_key_helpers_identify_vv_digits_and_modifiers --target-dir target_ui_isolate_platform_hotkey`
  - `cargo test pressed_modifier_state_maps_to_core_modifier_types --target-dir target_ui_isolate_platform_hotkey`
  - `cargo check --target-dir target_ui_isolate_platform_hotkey`

## step49
- 继续收紧 Windows 消息拆包边界：
  - 在 `platform::ui_event` 新增 `command_words`，统一拆分 `WM_COMMAND` 的 control id 和 notification code
  - 将设置窗口命令分派、主窗口命令分派、分组列表 `LBN_SELCHANGE` 判断从本地 `loword/hiword` 切到平台事件层
  - 删除 `app.rs` 内部的 `loword` / `hiword` helper
  - 这让主 UI host 继续减少 Win32 消息二进制布局细节，后续其他平台只需要提供自己的事件到命令转换
- 验证：
  - `cargo test command_words_split_control_id_and_notification_code --target-dir target_ui_isolate_command_words`
  - `cargo check --target-dir target_ui_isolate_command_words`

## step50
- 继续收紧 Windows 虚拟键语义边界：
  - 在 `platform::hotkey` 新增 Enter/Escape/Backspace/Find 的语义判断，以及 Escape/Find 的消息参数 helper
  - 将 VV 弹窗取消、Backspace 透传、输入对话框 Enter/Escape、编辑对话框 Escape/Ctrl+S、设置热键录制取消、quick hook Ctrl+F/Escape 转发改为调用平台层 helper
  - 将输入/编辑对话框内部 `WM_COMMAND` 的 id/notify 拆包切到 `platform::ui_event::command_words`
  - `app.rs` 和 `app/hosts.rs` 不再直接依赖这些键的 Win32 `VK_*` 常量
- 验证：
  - `cargo test virtual_key_helpers_identify_vv_digits_and_modifiers --target-dir target_ui_isolate_key_semantics`
  - `cargo test command_words_split_control_id_and_notification_code --target-dir target_ui_isolate_key_semantics`
  - `cargo check --target-dir target_ui_isolate_key_semantics`

## step51
- 继续收紧 Windows 消息参数和鼠标状态边界：
  - 在 `platform::ui_event` 新增 `dpi_from_wparam` 与 `size_from_lparam`
  - 将设置窗口/主窗口 `WM_DPICHANGED` 的 DPI 解码、编辑对话框 `WM_SIZE` 的宽高解码从 `app.rs` 移到平台事件层
  - 在 `platform::input` 新增 `primary_mouse_button_down` / `any_mouse_button_down`
  - 将主窗口拖动判断和外部点击隐藏判断从直接读取 `VK_LBUTTON/RBUTTON/MBUTTON` 改为调用平台输入层
  - `app.rs` / `app/hosts.rs` 不再直接依赖鼠标按键 `VK_*` 常量或 `WM_SIZE/WM_DPICHANGED` 的 bit 布局
- 验证：
  - `cargo test dpi_and_size_helpers_decode_win32_message_words --target-dir target_ui_isolate_message_params`
  - `cargo check --target-dir target_ui_isolate_message_params`

## step52
- 继续收紧 Windows 窗口状态消息语义：
  - 在 `platform::ui_event` 新增 `size_is_minimized`、`show_window_visible`、`app_activation_active`
  - 将设置窗口 `WM_SIZE`、主窗口 `WM_SIZE`、主窗口 `WM_SHOWWINDOW`、`WM_ACTIVATEAPP` 的 `wparam` 判断从直接比较 Win32 值改为调用平台事件层
  - `SIZE_MINIMIZED` 只保留在平台事件层，业务层不再知道窗口大小消息的具体常量值
  - 这进一步把 Windows 消息参数语义收口到 host/platform，为后续非 Win32 UI host 留出更清晰的事件语义入口
- 验证：
  - `cargo test dpi_and_size_helpers_decode_win32_message_words --target-dir target_ui_isolate_window_state`
  - `cargo check --target-dir target_ui_isolate_window_state`

## step53
- 继续收紧 Windows 定时器消息分派边界：
  - 将设置窗口和主窗口的 `WM_TIMER` 处理移入统一 `UiEvent::Timer` dispatch
  - 保留 `settings_timer_task_for_id` / `main_timer_task_for_id` 作为核心层 timer id 到应用任务的映射
  - 删除两个窗口过程中的直接 `WM_TIMER` 分支，让 Win32 消息到应用事件的入口更集中
  - 后续非 Win32 host 只需要产生 `UiEvent::Timer`，无需模拟 Windows `WM_TIMER`
- 验证：
  - `cargo test settings_timer_ids_map_to_settings_tasks --target-dir target_ui_isolate_timer_events`
  - `cargo test main_timer_ids_map_to_application_tasks --target-dir target_ui_isolate_timer_events`
  - `cargo check --target-dir target_ui_isolate_timer_events`

## step54
- 继续收紧设置窗口系统事件分派：
  - 将设置窗口 `WM_THEMECHANGED` 处理移入 `UiEvent::ThemeChanged` dispatch
  - 将设置窗口 `WM_DPICHANGED` 处理移入 `UiEvent::DpiChanged` dispatch，保持原有 suggested rect、DPI 同步、work area 校正和 metrics refresh 行为
  - 设置窗口过程减少两个直接 Win32 系统消息分支，系统事件入口继续向 `platform_ui_event::from_window_message` 集中
  - 主窗口 DPI 分支和输入/编辑对话框主题分支保留在各自 host 层，后续再分批处理
- 验证：
  - `cargo test clipboard_and_lifecycle_messages_are_mapped --target-dir target_ui_isolate_settings_system_events`
  - `cargo check --target-dir target_ui_isolate_settings_system_events`

## step55
- 继续收紧主窗口 DPI 系统事件分派：
  - 将主窗口 DPI 状态刷新移入 `UiEvent::DpiChanged` dispatch
  - 新增共享 `apply_dpi_suggested_rect` host helper，设置窗口和主窗口共用 Windows suggested rect 应用逻辑
  - 主窗口 `WM_DPICHANGED` 不再作为独立 match 分支处理业务状态；窗口定位副作用在进入事件分派前执行，DPI/layout 刷新由统一事件处理
  - 这让主窗口和设置窗口的 DPI 事件路径更一致，也让后续非 Win32 host 可以直接投递 DPI 事件
- 验证：
  - `cargo test clipboard_and_lifecycle_messages_are_mapped --target-dir target_ui_isolate_main_dpi_event`
  - `cargo check --target-dir target_ui_isolate_main_dpi_event`

## step56
- 继续收紧主窗口生命周期系统事件分派：
  - 将主窗口 `WM_SHOWWINDOW` 的隐藏/显示副作用移入 `UiEvent::Lifecycle` dispatch
  - `LifecycleEvent::Suspend` 负责隐藏内存回收和延迟 reclaim timer，`LifecycleEvent::Resume` 负责取消 reclaim timer 并启动 DPI fit timer
  - 低层输入 hook 刷新也跟随 Suspend/Resume 生命周期事件执行
  - 删除主窗口过程中的直接 `WM_SHOWWINDOW` 分支，窗口默认处理继续由 default window proc 承担
  - 这让主窗口显示状态变化的应用副作用不再依赖 Win32 分支，非 Win32 host 只需要投递 lifecycle 事件
- 验证：
  - `cargo test clipboard_and_lifecycle_messages_are_mapped --target-dir target_ui_isolate_lifecycle_events`
  - `cargo check --target-dir target_ui_isolate_lifecycle_events`

## step57
- 继续收紧主窗口销毁生命周期分派：
  - 将主窗口 `WM_DESTROY` 的应用清理移入 `LifecycleEvent::Unmount` dispatch
  - 抽出 `handle_main_destroy`，复用原有清理顺序：取消拖动、清页面/云同步结果、停止 timers、关闭 VV hook/LAN/剪贴板监听/热键/tray、销毁 quick 窗口并退出消息循环
  - `dispatch_main_ui_event` 对 `Unmount` 返回 handled，避免窗口过程再走独立 `WM_DESTROY` 分支
  - 主窗口过程删除直接 `WM_DESTROY` 分支；`WM_NCDESTROY` 继续保留 host 资源释放和 state box 回收
- 验证：
  - `cargo test clipboard_and_lifecycle_messages_are_mapped --target-dir target_ui_isolate_destroy_lifecycle`
  - `cargo check --target-dir target_ui_isolate_destroy_lifecycle`

## step58
- 继续收紧设置窗口销毁生命周期分派：
  - 将设置窗口 `WM_DESTROY` 的资源释放移入 `LifecycleEvent::Unmount` dispatch
  - 抽出 `handle_settings_destroy`，复用原有清理顺序：取消设置页滚动拖拽、停止设置页 timers、销毁下拉弹窗、释放字体/画刷、清空父窗口 `settings_hwnd` 并刷新低层输入 hook
  - 删除 `settings_wnd_proc` 中直接 `WM_DESTROY` 分支，让设置窗口销毁也统一经过 `platform_ui_event -> dispatch_settings_ui_event`
  - 这让主窗口和设置窗口的销毁路径都从 Win32 消息分支收口到生命周期事件，后续迁移其他 host 时只需要投递 `Unmount`
- 验证：
  - `cargo test clipboard_and_lifecycle_messages_are_mapped --target-dir target_ui_isolate_settings_destroy`
  - `cargo check --target-dir target_ui_isolate_settings_destroy`

## step59
- 继续收紧主窗口和设置窗口尺寸事件边界：
  - 在平台无关 `UiEvent` 新增 `WindowSize { size, minimized }`
  - `platform::ui_event` 统一将 Win32 `WM_SIZE` 的宽高和最小化状态转换为 `WindowSize`
  - 设置窗口通过统一 dispatch 执行 DPI compensation base 更新和 metrics refresh
  - 主窗口通过统一 dispatch 执行隐藏内存回收、窗口 region、子控件布局和重绘
  - 删除主窗口和设置窗口过程中的直接 `WM_SIZE` 分支；局部编辑对话框的尺寸处理仍保留在其 Windows host 内
  - 后续非 Win32 host 只需要发送平台无关尺寸事件，不需要构造 Windows `wparam/lparam`
- 验证：
  - `cargo test dpi_and_size_helpers_decode_win32_message_words --target-dir target_ui_isolate_window_size`
  - `cargo check --target-dir target_ui_isolate_window_size`

## step60
- 继续收紧应用激活和系统显示环境事件边界：
  - 在平台无关 `UiEvent` 新增 `AppActivationChanged { active }` 与 `SystemMetricsChanged`
  - `platform::ui_event` 将 `WM_ACTIVATEAPP` 转换为应用激活语义，将 `WM_SETTINGCHANGE / WM_DISPLAYCHANGE` 统一转换为系统指标变化语义
  - 主窗口失活时的 hover 清理、按配置自动隐藏和低层 hook 刷新移入统一 dispatch
  - 主窗口系统指标变化时的 metrics refresh 和 DPI fit timer 移入统一 dispatch
  - 设置窗口系统指标变化时的 DPI 同步、老系统 compensation、work area 校正和 metrics refresh 移入统一 dispatch
  - 删除主窗口和设置窗口过程中的直接 `WM_ACTIVATEAPP / WM_SETTINGCHANGE / WM_DISPLAYCHANGE` 分支；输入和编辑对话框的主题刷新仍保留在各自 Windows host
- 验证：
  - `cargo test dpi_and_size_helpers_decode_win32_message_words --target-dir target_ui_isolate_system_events`
  - `cargo check --target-dir target_ui_isolate_system_events`

## step61
- 继续收紧窗口移动和跨显示器 DPI 校正边界：
  - 在平台无关 `UiEvent` 新增 `WindowMoved` 与 `WindowMoveCompleted`
  - `platform::ui_event` 将 Win32 `WM_MOVE / WM_EXITSIZEMOVE` 转换为稳定窗口移动语义
  - 主窗口移动中的位置持久化、边缘隐藏状态更新、跨显示器 DPI layout 刷新和老系统 DPI fit timer 移入统一 dispatch
  - 主窗口移动结束后的目标显示器尺寸校正和最终位置持久化移入统一 dispatch
  - 设置窗口移动结束后的 DPI transition、老系统 compensation 状态同步和 work area 校正移入统一 dispatch
  - 删除主窗口和设置窗口过程中的直接 `WM_MOVE / WM_EXITSIZEMOVE` 分支
- 验证：
  - `cargo test dpi_and_size_helpers_decode_win32_message_words --target-dir target_ui_isolate_move_events`
  - `cargo check --target-dir target_ui_isolate_move_events`

## step62
- 继续收紧窗口关闭请求边界：
  - 在平台无关 `UiEvent` 新增 `CloseRequested`
  - `platform::ui_event` 将 Win32 `WM_CLOSE` 转换为稳定的用户关闭意图
  - 设置窗口通过统一 dispatch 销毁窗口
  - 主窗口通过统一 dispatch 执行关闭到托盘或真正销毁，并在处理后直接返回，避免销毁后继续访问命令队列
  - 删除主窗口和设置窗口过程中的直接 `WM_CLOSE` 分支；输入和编辑对话框仍保留各自专用 Windows host 关闭逻辑
- 验证：
  - `cargo test dpi_and_size_helpers_decode_win32_message_words --target-dir target_ui_isolate_close_event`
  - `cargo check --target-dir target_ui_isolate_close_event`

## step63
- 开始收紧原生控件命令边界：
  - 在平台无关 `UiEvent` 新增 `ControlCommand { control_id, notification }`
  - `platform::ui_event` 将 Win32 `WM_COMMAND` 的控件 id 和通知码转换为稳定控件命令事件
  - 主窗口搜索框 `EN_CHANGE` 和菜单命令全部改为通过统一 dispatch 处理，主窗口过程删除直接 `WM_COMMAND` 分支
  - 设置窗口保存、关闭、下拉选择和 toggle 等已具有稳定 `Command` 映射的通用控件命令改为通过统一 dispatch 入队执行
  - 设置窗口旧 `WM_COMMAND` 分支不再重复处理通用命令，只保留分组数据库操作、文件选择、系统配置和同步动作等 Windows host 专用执行逻辑
  - 后续将继续把这些专用动作拆成平台无关 intent 和各平台 executor
- 验证：
  - `cargo test command_words_split_control_id_and_notification_code --target-dir target_ui_isolate_control_command`
  - `cargo test settings_window_buttons_map_to_stable_commands --target-dir target_ui_isolate_control_command`
  - `cargo test main_menu_ids_map_to_stable_commands --target-dir target_ui_isolate_control_command`
  - `cargo check --target-dir target_ui_isolate_control_command`

## step64
- 继续收紧全局热键事件边界：
  - 在平台无关 `UiEvent` 新增 `GlobalHotkey { id }`
  - `platform::ui_event` 将 Win32 `WM_HOTKEY` 转换为稳定全局热键事件
  - 主窗口普通呼出热键和纯文本粘贴热键统一由 dispatch 选择粘贴模式并切换窗口
  - 删除主窗口过程中的直接 `WM_HOTKEY` 分支
  - 后续 macOS host 可将系统级快捷键回调直接映射为相同事件，无需模拟 Windows 消息
- 验证：
  - `cargo test command_words_split_control_id_and_notification_code --target-dir target_ui_isolate_hotkey_event`
  - `cargo check --target-dir target_ui_isolate_hotkey_event`

## step65
- 继续拆分设置页专用控件动作：
  - 在 `app_core` 新增平台无关 `SettingsAction`，描述分组管理、提示音选择、OCR 检测、更新检查、WebDAV 操作、局域网配对等用户意图
  - 在 Windows `settings_ui_host` 新增 `settings_action_for_control`，集中完成原生控件 ID/通知码到语义动作的映射
  - 为原来的 `6111 / 6112 / 6113 / 7203` 补充具名 Windows 控件常量，删除业务执行路径中的魔法数字
  - 设置窗口专用动作执行改为匹配 `SettingsAction`，`app.rs` 不再负责解释 Windows 控件编号
  - WebDAV 四种动作与局域网配对/复制入口保持原行为，但决策入口已可由其他平台 UI 直接复用
  - 当前设置窗口仍由 Windows `WM_COMMAND` 触发专用 executor，下一步继续将 executor 从窗口过程移到统一事件 dispatch
- 验证：
  - `cargo test settings_host_control_ids_map_to_semantic_actions --target-dir target_ui_isolate_settings_actions`
  - `cargo check --target-dir target_ui_isolate_settings_actions`

## step66
- 增强跨平台核心层架构门禁：
  - 将 `settings_model.rs` 纳入 `core_api_stays_platform_neutral` 自动审计
  - 禁止 `app_core / app_core/* / settings_model` 引入 `windows_sys`、HWND/HDC、WPARAM/LPARAM/LRESULT、WM/VK 常量、Windows 控件 ID
  - 禁止核心层重新依赖 `crate::platform` 或 `crate::win_*`
  - 当前审计确认核心模型、事件、布局和设置描述层均保持平台无关
  - 后续新增 Windows、macOS 或移动端 host 时，平台能力只能通过 adapter/executor 接入，不能反向污染核心层
- 验证：
  - `cargo test core_api_stays_platform_neutral --target-dir target_ui_isolate_core_guard`
  - `cargo check --target-dir target_ui_isolate_core_guard`

## step67
- 完成主设置窗口控件命令与窗口过程解耦：
  - 将设置动作 executor 拆为 `execute_settings_sync_action`、`execute_settings_group_action`、`execute_settings_platform_action`
  - `dispatch_settings_ui_event` 在收到 `ControlCommand` 后，先执行稳定 `Command`，再将专用控件映射为 `SettingsAction` 并交给对应 executor
  - WebDAV、局域网、分组管理、提示音、OCR、更新检查、系统剪贴板历史等动作全部移出 `settings_wnd_proc`
  - 局域网复制入口不再依赖 `WM_COMMAND lparam` 携带的 sender HWND，而是从设置 host 状态定位对应按钮
  - 删除设置窗口过程中的直接 `WM_COMMAND` 分支；当前 `app.rs` 中剩余 `WM_COMMAND` 仅属于输入/编辑对话框专用 Windows host
  - 主窗口与设置窗口现在共享统一的 `UiEvent::ControlCommand` 入口，其他平台 UI 可直接投递控件意图
- 验证：
  - `cargo test settings_host_control_ids_map_to_semantic_actions --target-dir target_ui_isolate_settings_executor`
  - `cargo test settings_window_buttons_map_to_stable_commands --target-dir target_ui_isolate_settings_executor`
  - `cargo test core_api_stays_platform_neutral --target-dir target_ui_isolate_settings_executor`
  - `cargo check --target-dir target_ui_isolate_settings_executor`

## step68
- 完成设置下拉选择事件与 Windows 私有消息解耦：
  - 在平台无关 `UiEvent` 新增 `ControlSelectionChanged { control_id, index }`
  - Windows `settings_ui_host` 将 `WM_SETTINGS_DROPDOWN_SELECTED` 转换为统一选择事件
  - 将最大保存条数、窗口位置模式、同步方案、热键、搜索引擎、OCR、翻译、VV 来源/分组等选择更新移入 `handle_settings_control_selection`
  - 用具名控件常量替换下拉处理中的 `6102 / 6103 / 7201` 魔法数字
  - 删除 `settings_wnd_proc` 中直接处理私有下拉消息的业务分支
  - 设置窗口过程目前只保留创建、owner-draw/paint、原生颜色和默认窗口处理等 Windows host 职责
- 验证：
  - `cargo test dropdown_private_message_maps_to_platform_neutral_selection_event --target-dir target_ui_isolate_selection_event`
  - `cargo test max_items_label_parser_rejects_empty_text --target-dir target_ui_isolate_selection_event`
  - `cargo test core_api_stays_platform_neutral --target-dir target_ui_isolate_selection_event`
  - `cargo check --target-dir target_ui_isolate_selection_event`

## step69
- 完成主窗口应用通知与 Windows 自定义消息解耦：
  - 在平台无关核心层新增 `ApplicationEvent` 与不透明 `NativeWindowToken`
  - 将局域网刷新、VV 显示/隐藏/选择、分页完成、云同步完成、更新检查、Explorer 重启恢复和托盘回调映射为应用事件
  - 这些事件的业务处理统一进入 `dispatch_main_ui_event -> handle_main_application_event`
  - 将图片粘贴、OCR、翻译和缩略图的 Box 载荷转换为拥有所有权的 `MainAsyncEvent`
  - Windows adapter 只负责从 `lparam` 取回 Box 一次；业务处理不再读取消息参数或裸指针
  - OCR 与文本翻译复用统一文本结果处理，减少重复的剪贴板写入、记录创建和错误提示逻辑
  - 删除主窗口过程中的全部自定义业务消息分支；窗口过程剩余分支集中于创建、绘制、mouse activate、非客户区命中和 host 资源释放
- 验证：
  - `cargo test windows_custom_messages_map_to_platform_neutral_application_events --target-dir target_ui_isolate_app_events`
  - `cargo test windows_boxed_result_message_transfers_payload_ownership_once --target-dir target_ui_isolate_app_events`
  - `cargo test core_api_stays_platform_neutral --target-dir target_ui_isolate_app_events`
  - `cargo check --target-dir target_ui_isolate_app_events`

## step70
- 继续收口主窗口异步结果边界：
  - 将 `ImageThumbnail` 从 `app::state` 移入平台无关 `app_core`
  - 将图片粘贴、图片 OCR、文本翻译、图片缩略图完成结果移入 `app_core::MainAsyncEvent`
  - 图片粘贴目标从裸 `isize` 改为不透明 `NativeWindowToken`，业务事件不再直接暴露 Windows 句柄语义
  - Windows adapter 仍负责从 `lparam` 接管 Box 所有权，但事件载荷类型已经由核心层定义
  - `APP_CORE_API_VERSION` 提升到 `0.10`，标记核心 UI contract 扩展
  - 后续 macOS/其他平台 host 可以复用同一组异步完成事件，只需要提供自己的投递和回调 adapter
- 验证：
  - `cargo test main_async_events_are_plain_platform_neutral_payloads --target-dir target_ui_isolate_async_payload`
  - `cargo test windows_boxed_result_message_transfers_payload_ownership_once --target-dir target_ui_isolate_async_payload`
  - `cargo test core_api_stays_platform_neutral --target-dir target_ui_isolate_async_payload`
  - `cargo check --target-dir target_ui_isolate_async_payload`

## step71
- 继续拆分 Windows 主窗口消息 adapter：
  - 新增 `src/app/windows_messages.rs`，集中保存主窗口 `WM_APP + N` 私有消息编号
  - 将 Windows 自定义消息到 `UiEvent::Application` 的转换从 `app.rs` 移入 adapter 模块
  - 将异步 Box payload 的 `lparam` 接管逻辑从 `app.rs` 移入 adapter 模块
  - `app.rs` 只消费 `main_application_event_from_window_message` 和 `take_main_async_event_from_window_message` 的平台无关结果
  - `WM_LAN_SYNC_READY` 与 `WM_TRAYICON` 继续通过 `app` re-export 给 `lan_sync` / `tray` 使用，外部接口保持兼容
  - 这一步让后续 macOS host 可以单独提供自己的事件投递 adapter，而不需要理解 Windows 私有消息编号
- 验证：
  - `cargo test windows_custom_messages_map_to_platform_neutral_application_events --target-dir target_ui_isolate_windows_messages`
  - `cargo test windows_boxed_result_message_transfers_payload_ownership_once --target-dir target_ui_isolate_windows_messages`
  - `cargo test core_api_stays_platform_neutral --target-dir target_ui_isolate_windows_messages`
  - `cargo check --target-dir target_ui_isolate_windows_messages`

## step72
- 继续压缩主窗口过程中的消息路由职责：
  - 在 `src/app/windows_messages.rs` 新增 `MainWindowHostEvent`
  - 新增 `main_window_host_event_from_message`，统一完成 Windows 消息到 `UiEvent` / `MainAsyncEvent` 的路由
  - `wnd_proc` 不再手写 async message、application message、generic platform UI event 的优先级判断
  - 主窗口过程现在只消费统一 host event：异步结果交给 `handle_main_async_event`，UI 事件交给 `dispatch_main_ui_event`
  - 这一步进一步把“Windows 消息解释”限制在 Windows adapter 文件里，方便后续 macOS host 用自己的事件来源替换
- 验证：
  - `cargo test main_window_host_event_adapter_routes_async_and_ui_messages --target-dir target_ui_isolate_host_event`
  - `cargo test windows_custom_messages_map_to_platform_neutral_application_events --target-dir target_ui_isolate_host_event`
  - `cargo test windows_boxed_result_message_transfers_payload_ownership_once --target-dir target_ui_isolate_host_event`
  - `cargo test core_api_stays_platform_neutral --target-dir target_ui_isolate_host_event`
  - `cargo check --target-dir target_ui_isolate_host_event`

## step73
- 继续压缩设置窗口过程中的消息路由职责：
  - 在 `settings_ui_host` 新增 `settings_window_host_event_from_message`
  - 将设置页私有下拉消息和通用平台 UI 事件的拼接逻辑从 `settings_wnd_proc` 移入 Windows 设置 host
  - `settings_wnd_proc` 现在只消费统一后的 `UiEvent`，再交给 `dispatch_settings_ui_event`
  - 移除 `win_system_ui` 对旧 `settings_event_from_window_message` 的 re-export，避免上层继续绕过统一入口
  - 这一步让主窗口与设置窗口都拥有各自的 host event adapter，后续非 Windows UI 可以直接生成同一组平台无关事件
- 验证：
  - `cargo test settings_window_host_event_routes_private_and_generic_messages --target-dir target_ui_isolate_settings_host_event`
  - `cargo test dropdown_private_message_maps_to_platform_neutral_selection_event --target-dir target_ui_isolate_settings_host_event`
  - `cargo test core_api_stays_platform_neutral --target-dir target_ui_isolate_settings_host_event`
  - `cargo check --target-dir target_ui_isolate_settings_host_event`

## step74
- 继续压缩设置下拉弹窗子窗口的消息路由职责：
  - 在 `settings_ui_host` 新增 `dropdown_window_host_event_from_message`
  - 将下拉弹窗子窗口对 `platform_ui_event::from_window_message` 的直接调用收进专用 host adapter
  - `dropdown_popup_proc` 现在只消费统一后的 `UiEvent`，再交给 `dispatch_dropdown_ui_event`
  - 主窗口、设置窗口和设置下拉弹窗现在都拥有明确的 host event adapter 入口
  - 后续迁移到 macOS/其他 UI host 时，可分别替换主窗口、设置窗口和弹窗的事件来源，而不是散落查找 Win32 消息翻译
- 验证：
  - `cargo test dropdown_window_host_event_routes_pointer_messages --target-dir target_ui_isolate_dropdown_host_event`
  - `cargo test settings_window_host_event_routes_private_and_generic_messages --target-dir target_ui_isolate_dropdown_host_event`
  - `cargo test core_api_stays_platform_neutral --target-dir target_ui_isolate_dropdown_host_event`
  - `cargo check --target-dir target_ui_isolate_dropdown_host_event`

## step75
- 继续压缩输入对话框窗口过程中的消息路由职责：
  - 在 Windows 主窗口 adapter 中新增 `input_dialog_host_event_from_message`
  - 输入对话框的保存/取消命令、Enter/Escape 按键、主题/系统指标变化和关闭请求统一转换为 `UiEvent`
  - 新增 `dispatch_input_dialog_ui_event`，`input_dlg_proc` 不再直接处理 `WM_COMMAND / WM_KEYDOWN / WM_THEMECHANGED / WM_SETTINGCHANGE / WM_CLOSE`
  - 输入对话框仍保留创建、绘制、owner-draw、颜色和资源释放等 Windows host 职责
  - 这一步给后续编辑对话框复用同样的小窗口 host event 模式打好基础
- 验证：
  - `cargo test input_dialog_host_event_adapter_routes_commands_keys_and_close --target-dir target_ui_isolate_input_dialog_event`
  - `cargo test main_window_host_event_adapter_routes_async_and_ui_messages --target-dir target_ui_isolate_input_dialog_event`
  - `cargo test core_api_stays_platform_neutral --target-dir target_ui_isolate_input_dialog_event`
  - `cargo check --target-dir target_ui_isolate_input_dialog_event`

## step76
- 继续压缩编辑对话框窗口过程中的消息路由职责：
  - 在 Windows 主窗口 adapter 中新增 `edit_dialog_host_event_from_message`
  - 编辑对话框的文本区命令、保存/取消命令、窗口尺寸变化、Escape/Ctrl+S 快捷键、主题/系统指标变化和关闭请求统一转换为 `UiEvent`
  - 新增 `dispatch_edit_dialog_ui_event`，`edit_dlg_proc` 不再直接处理 `WM_COMMAND / WM_SIZE / WM_KEYDOWN / WM_THEMECHANGED / WM_SETTINGCHANGE / WM_CLOSE`
  - 编辑对话框仍保留创建、绘制、owner-draw、颜色、滚动同步和资源释放等 Windows host 职责
  - `app.rs` 不再直接导入 `platform_ui_event`，窗口消息转换集中到 host adapter 层
- 验证：
  - `cargo test edit_dialog_host_event_adapter_routes_commands_size_keys_and_close --target-dir target_ui_isolate_edit_dialog_event`
  - `cargo test input_dialog_host_event_adapter_routes_commands_keys_and_close --target-dir target_ui_isolate_edit_dialog_event`
  - `cargo test core_api_stays_platform_neutral --target-dir target_ui_isolate_edit_dialog_event`
  - `cargo check --target-dir target_ui_isolate_edit_dialog_event`

## step77
- 增加 UI 事件隔离防回退守卫：
  - 新增 `app_window_procs_do_not_decode_platform_ui_messages_directly`
  - 自动检查 `app.rs` 不再直接调用 `platform_ui_event::from_window_message`
  - 自动检查 `app.rs` 不再直接调用 `platform_ui_event::command_words` 或 `platform_ui_event::size_from_lparam`
  - 主窗口、设置窗口、输入对话框、编辑对话框和下拉弹窗的消息转换入口都应保持在各自 host adapter 中
  - 这条守卫防止后续开发又把 Win32 消息解码逻辑写回大 `app.rs`
- 验证：
  - `cargo test app_window_procs_do_not_decode_platform_ui_messages_directly --target-dir target_ui_isolate_message_guard`
  - `cargo test edit_dialog_host_event_adapter_routes_commands_size_keys_and_close --target-dir target_ui_isolate_message_guard`
  - `cargo test core_api_stays_platform_neutral --target-dir target_ui_isolate_message_guard`
  - `cargo check --target-dir target_ui_isolate_message_guard`

## step78
- 将 UI 消息解码边界守卫扩展到整个 `src`：
  - 新增 `platform_ui_message_decoding_stays_in_host_adapters`
  - 运行时遍历 `src/**/*.rs`，只允许 `src/platform/ui_event.rs`、`src/app/windows_messages.rs`、`src/settings_ui_host.rs` 使用 Windows UI 消息解码入口
  - 禁止业务模块直接调用 `platform_ui_event::from_window_message`
  - 禁止业务模块直接调用 `platform_ui_event::command_words` 或 `platform_ui_event::size_from_lparam`
  - 禁止非 adapter 文件直接 `use crate::platform::ui_event as platform_ui_event`
  - 这条守卫把“Windows 消息翻译只能留在平台/host adapter”从约定变成自动测试
- 验证：
  - `cargo test platform_ui_message_decoding_stays_in_host_adapters --target-dir target_ui_isolate_boundary_guard`
  - `cargo test app_window_procs_do_not_decode_platform_ui_messages_directly --target-dir target_ui_isolate_boundary_guard`
  - `cargo test core_api_stays_platform_neutral --target-dir target_ui_isolate_boundary_guard`
  - `cargo check --target-dir target_ui_isolate_boundary_guard`

## step79
- 将跨平台 UI host surface 清单固化到核心契约：
  - 在 `app_core` 新增 `UiHostSurface`
  - 新增 `REQUIRED_UI_HOST_SURFACES`，明确新平台至少要实现主窗口、设置窗口、设置下拉弹窗、输入对话框和编辑对话框 5 个 host surface
  - 每个 surface 提供稳定 adapter 名称，作为 Windows/macOS 等 host 的迁移对照表
  - `APP_CORE_API_VERSION` 提升到 `0.11`，标记 UI host contract 扩展
  - 新增 `windows_host_adapters_cover_required_ui_surfaces`，确认 Windows host 已覆盖核心层要求的全部 surface
- 验证：
  - `cargo test required_ui_host_surfaces_are_explicit_porting_contract --target-dir target_ui_isolate_host_contract`
  - `cargo test windows_host_adapters_cover_required_ui_surfaces --target-dir target_ui_isolate_host_contract`
  - `cargo test platform_ui_message_decoding_stays_in_host_adapters --target-dir target_ui_isolate_host_contract`
  - `cargo test core_api_stays_platform_neutral --target-dir target_ui_isolate_host_contract`
  - `cargo check --target-dir target_ui_isolate_host_contract`

## step80
- 补齐 UI host 移植契约文档：
  - 新增 `docs/ui-host-porting.md`，面向后续 macOS/其他平台 host 说明最低迁移面
  - 文档以 `REQUIRED_UI_HOST_SURFACES` 为权威来源，列出 5 个 host surface 和对应 adapter
  - 新增 `ui_host_porting_doc_covers_required_surfaces`，确保文档覆盖核心层要求的 surface 与 adapter 名称
  - 这让“如何继续移植”从历史记录变成可检查的迁移清单
- 验证：
  - `cargo test ui_host_porting_doc_covers_required_surfaces --target-dir target_ui_isolate_porting_doc`
  - `cargo test required_ui_host_surfaces_are_explicit_porting_contract --target-dir target_ui_isolate_porting_doc`
  - `cargo test windows_host_adapters_cover_required_ui_surfaces --target-dir target_ui_isolate_porting_doc`
  - `cargo test platform_ui_message_decoding_stays_in_host_adapters --target-dir target_ui_isolate_porting_doc`
  - `cargo test core_api_stays_platform_neutral --target-dir target_ui_isolate_porting_doc`
  - `cargo check --target-dir target_ui_isolate_porting_doc`

## step81
- 将主窗口 host 执行动作纳入跨平台迁移契约：
  - `APP_CORE_API_VERSION` 提升到 `0.12`
  - 新增 `MainHostExecutionPlanKind` 和 `REQUIRED_MAIN_HOST_EXECUTION_PLAN_KINDS`
  - 明确新平台主窗口 host 至少要支持搜索显隐、打开设置、隐藏主窗、关闭主窗、执行菜单命令 5 类平台动作
  - `MainHostExecutionPlan::kind()` 可把带 payload 的执行计划归类为稳定迁移清单
  - `docs/ui-host-porting.md` 新增 Main Host Execution Plans 小节，并由测试校验文档覆盖全部 plan kind
- 验证：
  - `cargo test main_host_execution_plan_kinds_are_explicit_porting_contract --target-dir target_ui_isolate_host_execution`
  - `cargo test ui_host_porting_doc_covers_required_surfaces --target-dir target_ui_isolate_host_execution`
  - `cargo test core_api_stays_platform_neutral --target-dir target_ui_isolate_host_execution`
  - `cargo test platform_ui_message_decoding_stays_in_host_adapters --target-dir target_ui_isolate_host_execution`
  - `cargo fmt`
  - `cargo check --target-dir target_ui_isolate_host_execution`

## step82
- 将设置页原生控件 host 能力纳入跨平台迁移契约：
  - `APP_CORE_API_VERSION` 提升到 `0.13`
  - 新增 `SettingsControlHostOperation` 和 `REQUIRED_SETTINGS_CONTROL_HOST_OPERATIONS`
  - 明确非 Windows 设置页 host 至少要支持创建/销毁控件、显隐、启用、布局、读取文本、写入文本 7 类能力
  - `docs/ui-host-porting.md` 新增 Settings Control Host 小节，说明 `SettingsControlSpec` 与 `NativeSettingsControlHost` 的迁移边界
  - 文档覆盖测试现在同时校验 surface、主窗口 execution plan 和设置控件 host operation
- 验证：
  - `cargo test settings_control_host_operations_are_explicit_porting_contract --target-dir target_ui_isolate_settings_host_contract`
  - `cargo test ui_host_porting_doc_covers_required_surfaces --target-dir target_ui_isolate_settings_host_contract`
  - `cargo test core_api_stays_platform_neutral --target-dir target_ui_isolate_settings_host_contract`
  - `cargo test platform_ui_message_decoding_stays_in_host_adapters --target-dir target_ui_isolate_settings_host_contract`
  - `cargo test windows_settings_control_host_uses_platform_neutral_specs --target-dir target_ui_isolate_settings_host_contract`
  - `cargo fmt`
  - `cargo check --target-dir target_ui_isolate_settings_host_contract`

## step83
- 将原生样式解析与控件映射纳入跨平台迁移契约：
  - `APP_CORE_API_VERSION` 提升到 `0.14`
  - 新增 `NativeStyleHostOperation` / `REQUIRED_NATIVE_STYLE_HOST_OPERATIONS`
  - 新增 `NativeControlMapperOperation` / `REQUIRED_NATIVE_CONTROL_MAPPER_OPERATIONS`
  - 明确各平台 host 通过 `resolve_text_style` 将语义文本样式转换为系统字体、字号、颜色和对齐
  - 明确各平台 host 通过 `class_name` 将 `SettingsComponentKind` 映射到本平台原生控件族/类
  - `docs/ui-host-porting.md` 新增 Native Style And Control Mapping 小节，避免后续平台为了复刻 Windows 外观而污染共享逻辑
- 验证：
  - `cargo test native_style_host_operations_are_explicit_porting_contract --target-dir target_ui_isolate_native_style_contract`
  - `cargo test native_control_mapper_operations_are_explicit_porting_contract --target-dir target_ui_isolate_native_style_contract`
  - `cargo test ui_host_porting_doc_covers_required_surfaces --target-dir target_ui_isolate_native_style_contract`
  - `cargo test windows_native_style_host_covers_required_operations --target-dir target_ui_isolate_native_style_contract`
  - `cargo test core_api_stays_platform_neutral --target-dir target_ui_isolate_native_style_contract`
  - `cargo test platform_ui_message_decoding_stays_in_host_adapters --target-dir target_ui_isolate_native_style_contract`
  - `cargo fmt`
  - `cargo check --target-dir target_ui_isolate_native_style_contract`

## step84
- 将组件渲染 host 原语纳入跨平台迁移契约：
  - `APP_CORE_API_VERSION` 提升到 `0.15`
  - 新增 `TextLayoutHostOperation` / `REQUIRED_TEXT_LAYOUT_HOST_OPERATIONS`
  - 新增 `RendererHostOperation` / `REQUIRED_RENDERER_HOST_OPERATIONS`
  - 明确使用组件抽象的平台 host 需要提供文本测量、文本 run 布局、矩形填充/描边、文本绘制、clip push/pop 等原语
  - `docs/ui-host-porting.md` 新增 Rendering Host Primitives 小节，并说明 `LayoutProtocol` / `Component` 是共享组件协议，不是单独平台 surface
  - Windows 侧新增 `windows_gdi_renderer_covers_required_render_primitives`，确认 `GdiRenderer` / `GdiTextLayout` 覆盖必需原语
- 验证：
  - `cargo test text_layout_host_operations_are_explicit_porting_contract --target-dir target_ui_isolate_render_host_contract`
  - `cargo test renderer_host_operations_are_explicit_porting_contract --target-dir target_ui_isolate_render_host_contract`
  - `cargo test ui_host_porting_doc_covers_required_surfaces --target-dir target_ui_isolate_render_host_contract`
  - `cargo test windows_gdi_renderer_covers_required_render_primitives --target-dir target_ui_isolate_render_host_contract`
  - `cargo test core_api_stays_platform_neutral --target-dir target_ui_isolate_render_host_contract`
  - `cargo test platform_ui_message_decoding_stays_in_host_adapters --target-dir target_ui_isolate_render_host_contract`
  - `cargo test label_uses_layout_and_renderer_protocols --target-dir target_ui_isolate_render_host_contract`
  - `cargo fmt`
  - `cargo check --target-dir target_ui_isolate_render_host_contract`

## step85
- 完成 UI host 迁移契约的最终审计收口：
  - `APP_CORE_API_VERSION` 提升到 `0.16`
  - 新增 `SharedUiProtocol` 和 `SHARED_NON_HOST_UI_PROTOCOLS`
  - 明确 `LayoutProtocol` / `Component` 是共享组件协议，不是额外平台 host surface
  - `docs/ui-host-porting.md` 将权威来源扩展为 `app_core` 的 `REQUIRED_*` 与 `SHARED_*` 清单
  - 文档覆盖测试现在同时检查 host surface、host execution plan、native style、native control mapper、text layout、renderer、settings control host 和 shared non-host protocol
  - 完整 `cargo test -j 1` 覆盖 290 个测试，作为本轮完成审计证据
- 验证：
  - `cargo test shared_ui_protocols_are_explicitly_not_platform_host_surfaces --target-dir target_ui_isolate_final_audit`
  - `cargo test ui_host_porting_doc_covers_required_surfaces --target-dir target_ui_isolate_final_audit`
  - `cargo test core_api_stays_platform_neutral --target-dir target_ui_isolate_final_audit`
  - `cargo test windows_host_adapters_cover_required_ui_surfaces --target-dir target_ui_isolate_final_audit`
  - `cargo test platform_ui_message_decoding_stays_in_host_adapters --target-dir target_ui_isolate_final_audit`
  - `cargo test windows_native_style_host_covers_required_operations --target-dir target_ui_isolate_final_audit`
  - `cargo test windows_gdi_renderer_covers_required_render_primitives --target-dir target_ui_isolate_final_audit`
  - `cargo test windows_settings_control_host_uses_platform_neutral_specs --target-dir target_ui_isolate_final_audit`
  - `cargo test label_uses_layout_and_renderer_protocols --target-dir target_ui_isolate_final_audit`
  - `cargo check --target-dir target_ui_isolate_final_audit`
  - `cargo test -j 1 --target-dir target_ui_isolate_final_audit`

## step86
- 开始 Windows 旧 UI 反向清理与 macOS host 并行推进：
  - 保留 `app_core/components`、`TextLayout`、`Renderer` 等新框架协议，不删除新体系
  - 将 `windows-sys` / `clipboard-win` 移入 Windows target dependency，避免 macOS host 被 Windows 依赖阻塞
  - `src/main.rs` 按平台拆分入口：Windows 继续进入现有 Win32 host，macOS 进入 `src/macos_app.rs`
  - 新增 `src/macos_app.rs`，作为 macOS UI host scaffold，只读取 `app_core` contract summary，不复制 Windows `app.rs`
  - 新增 `docs/macos-ui.md`，明确 macOS 用原生 host 逐步接共享契约，每个 macOS 功能都反推 Windows 清理旧 UI
  - 新增 `platform_entry_points_are_cfg_gated_for_windows_and_macos`，防止 macOS scaffold 误引用 Windows host
- 验证：
  - `cargo test label_uses_layout_and_renderer_protocols --target-dir target_ui_restore_check`
  - `cargo test ui_host_porting_doc_covers_required_surfaces --target-dir target_ui_restore_check`
  - `cargo check --target-dir target_ui_restore_check`

## step87
- 将剪贴板能力纳入跨平台 host 契约，作为 macOS UI 与 Windows 旧路径反向清理的下一块边界：
  - `APP_CORE_API_VERSION` 提升到 `0.17`
  - 新增 `ClipboardHostOperation` / `REQUIRED_CLIPBOARD_HOST_OPERATIONS`
  - 明确 host 需要覆盖 `read_text`、`write_text`、`read_image`、`write_image`、`read_file_paths`、`write_file_paths`、`sequence_number`、`monitor_ignore_formats`
  - macOS scaffold 增加 `MacosClipboardHost`，先跟踪 shared clipboard contract，后续用 `NSPasteboard` 实现
  - `docs/ui-host-porting.md` 新增 Clipboard Host 小节，说明 Windows 现阶段由 `src/platform/clipboard.rs` + `arboard` 覆盖，下一步再收口到统一 host
  - 新增 `windows_clipboard_host_covers_required_operations`，避免清理 Windows 旧路径时漏掉文本、图片、文件路径、序列号和忽略监控格式
- 验证：
  - `cargo test clipboard_host_operations_are_explicit_porting_contract --target-dir target_clipboard_host_step1`
  - `cargo test ui_host_porting_doc_covers_required_surfaces --target-dir target_clipboard_host_step1`
  - `cargo test windows_clipboard_host_covers_required_operations --target-dir target_clipboard_host_step1`
  - `cargo test platform_entry_points_are_cfg_gated_for_windows_and_macos --target-dir target_clipboard_host_step1`
  - `cargo check --target-dir target_clipboard_host_step1`

## step88
- 将剪贴板 contract 从清单推进为真实的平台 host：
  - 新增编译期 `ClipboardHost` trait，Windows 与 macOS host 必须实现同一组剪贴板能力
  - `src/platform/clipboard.rs` 新增 `WindowsClipboardHost`，统一承接 `arboard` 文本/图片与 Win32 文件路径、sequence、monitor-ignore 格式
  - `src/app.rs` 移除直接 `Clipboard::new()` / `ImageData` / `Cow` 调用，复制记录、图片粘贴、纯文本粘贴、LAN 镜像与捕获读取统一走 `WindowsClipboardHost`
  - Windows 功能不删除，只删除迁移后重复的平台调用路径
  - `MacosClipboardHost` 实现基础文字和 RGBA 图片读写；文件 URL、原生 change count 与监控语义后续用 `NSPasteboard` 补齐
  - 文档明确统一框架共享功能语义、状态、布局计划和组件 contract，不强制 Windows/macOS 使用同一视觉皮肤
- 验证：
  - `cargo test clipboard_host_operations_are_explicit_porting_contract --target-dir target_clipboard_host_step2`
  - `cargo test windows_clipboard_host_covers_required_operations --target-dir target_clipboard_host_step2`
  - `cargo test ui_host_porting_doc_covers_required_surfaces --target-dir target_clipboard_host_step2`
  - `cargo check --target-dir target_clipboard_host_step2`

## step89
- 修正剪贴板 host 边界守门测试：
  - `windows_clipboard_host_covers_required_operations` 不再被自身的 `Clipboard` 构造器检查字符串误命中
  - 检查词改为运行时拼接，确保只在 `app.rs` 存在真实直接构造时失败
  - 保持 `WindowsClipboardHost` 作为主应用剪贴板操作的唯一平台入口
- 验证：
  - `cargo test -j 1 --target-dir target_ui_refactor windows_clipboard_host_covers_required_operations`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step90
- 让 macOS host 开始实际消费共享主窗口 UI 模型：
  - 搜索日期上下文与共享首屏 render input 将 contract 扩展到 `0.18`；合并状态栏 host 契约后当前 `APP_CORE_API_VERSION` 为 `0.19`
  - 新增 `SearchDateContext` / `parse_search_query_with_context`，由平台 host 提供本地日期，`app_core` 只负责确定性搜索语义解析
  - 移除 `app_core::main_window` 对 Windows `time_utils` / 时区实现的隐藏依赖
  - Windows 数据查询继续使用本地日期上下文，保持“今天/昨天/无年份日期”搜索行为
  - 新增 `MainRenderInput::empty_records`，作为平台无关的主窗口空历史首屏输入
  - 新增 `MacosMainWindowModel`，实际调用 `MainUiLayout::render_plan` 生成首屏 render plan
  - macOS contract summary 同步跟踪 `StatusItemHost` 三项操作，为原生菜单栏入口保留明确实现清单
  - 测试构建也编译 `macos_app.rs`，使 Windows CI 能发现 macOS scaffold 与共享契约的编译回归
- 验证：
  - `cargo test -j 1 --target-dir target_ui_refactor search_query_date_context_keeps_local_calendar_out_of_core_platform_calls`
  - `cargo test -j 1 --target-dir target_ui_refactor empty_records_render_input_builds_first_screen_without_host_state`
  - `cargo test -j 1 --target-dir target_ui_refactor macos_main_window_consumes_shared_render_plan`
  - `cargo test -j 1 --target-dir target_ui_refactor macos_status_item_host_consumes_shared_menu_entries`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_entry_points_are_cfg_gated_for_windows_and_macos`
  - `cargo test -j 1 --target-dir target_ui_refactor platform_neutral`
  - `cargo test -j 1 --target-dir target_ui_refactor`

## step91
- 将 Windows 托盘与 macOS 菜单栏状态项纳入同一 host 边界：
  - `APP_CORE_API_VERSION` 提升到 `0.19`
  - 新增 `StatusMenuEntry`、`StatusItemHost` 与 `REQUIRED_STATUS_ITEM_HOST_OPERATIONS`
  - `MainTrayMenuAction::command_id` 统一共享 action 到稳定命令 id 的映射
  - 新增 `WindowsStatusItemHost`，承接托盘图标安装/移除、Win32 原生弹出菜单创建与呈现
  - `src/tray.rs` 不再直接创建/绘制 Win32 菜单，只负责共享菜单 plan、本地化文案与窗口行为
  - 新增 `MacosStatusItemHost`，编译期消费相同的 install/remove/menu contract；下一步用 `NSStatusItem` 替换当前状态 scaffold
  - Windows/macOS 功能与原生视觉保持各自平台实现，不删除托盘开关、LAN、捕获和退出功能
- 验证：
  - `cargo test status_item_host_operations_are_explicit_porting_contract --target-dir target_status_host_step1`
  - `cargo test windows_status_item_host_owns_native_tray_menu_operations --target-dir target_status_host_step1`
  - `cargo test macos_status_item_host_consumes_shared_menu_entries --target-dir target_status_host_step1`
  - `cargo test ui_host_porting_doc_covers_required_surfaces --target-dir target_status_host_step1`
  - `cargo check --target-dir target_status_host_step1`

## step92
- 将主窗口原生弹出菜单纳入跨平台 host 边界：
  - `APP_CORE_API_VERSION` 提升到 `0.20`
  - 新增 `NativePopupMenuEntry`、`NativePopupMenuHost`、`NativePopupMenuPlacement` 与 `REQUIRED_NATIVE_POPUP_MENU_HOST_OPERATIONS`
  - 新增 `WindowsPopupMenuHost`，统一负责 Win32 popup menu 创建、子菜单、禁用项、勾选项、呈现和销毁
  - `src/app.rs` 中 VV 分组菜单、行右键菜单、tab 分组过滤菜单不再直接调用 `platform_menu::create_popup/append_raw/track_popup_raw`
  - 行菜单的“添加到分组”继续通过 `NativePopupMenuEntry::Submenu` 保留嵌套菜单
  - 新增 `MacosPopupMenuHost`，编译期消费相同 popup menu entries；下一步用原生 `NSMenu` 呈现
  - Windows/macOS 都继续使用本平台原生菜单视觉，不引入统一皮肤
- 验证：
  - `cargo test native_popup_menu_host_operations_are_explicit_porting_contract --target-dir target_popup_menu_host_step1`
  - `cargo test windows_popup_menu_host_owns_main_native_menu_operations --target-dir target_popup_menu_host_step1`
  - `cargo test macos_popup_menu_host_consumes_shared_popup_entries --target-dir target_popup_menu_host_step1`
  - `cargo test ui_host_porting_doc_covers_required_surfaces --target-dir target_popup_menu_host_step1`
  - `cargo check --target-dir target_popup_menu_host_step1`

## step93
- 将简单原生消息框纳入跨平台 host 边界：
  - `APP_CORE_API_VERSION` 提升到 `0.21`
  - 新增 `NativeDialogLevel`、`NativeDialogHost` 与 `REQUIRED_NATIVE_DIALOG_HOST_OPERATIONS`
  - 新增 `WindowsDialogHost`，将 `Info/Warning/Error/Question` 映射到系统 `MessageBoxW` 图标和 OK 按钮
  - `src/app.rs` 中行操作的“没有可推送文件”和“二维码生成失败”提示改走 `WindowsDialogHost`
  - 新增 `MacosDialogHost` scaffold，先跟踪简单消息 contract；下一步用原生 `NSAlert` 呈现
  - 本轮只迁移一键消息框，不触碰 Yes/No/Cancel 确认对话框，避免行为回归
- 验证：
  - `cargo test native_dialog_host_operations_are_explicit_porting_contract --target-dir target_dialog_host_step1`
  - `cargo test windows_dialog_host_owns_row_action_message_boxes --target-dir target_dialog_host_step1`
  - `cargo test macos_dialog_host_tracks_shared_message_contract --target-dir target_dialog_host_step1`
  - `cargo test ui_host_porting_doc_covers_required_surfaces --target-dir target_dialog_host_step1`
  - `cargo check --target-dir target_dialog_host_step1`

## step94
- 扩展原生对话框 host 到三态确认：
  - `APP_CORE_API_VERSION` 提升到 `0.22`
  - 新增 `NativeDialogButtons` / `NativeDialogResponse`
  - `NativeDialogHost` 增加 `confirm`，返回 `Yes` / `No` / `Cancel`
  - `WindowsDialogHost` 将 `YesNoCancel` / `YesNo` 映射到系统 `MessageBoxW`，并把 `IDYES/IDNO/IDCANCEL` 转成平台无关响应
  - `edit_dialog_confirm_close` 改走 `WindowsDialogHost::confirm`，保持 Yes=保存、No=放弃关闭、Cancel=停留不变
  - `MacosDialogHost` 实现同一 confirm contract，当前安全默认返回 Cancel；后续用 `NSAlert` 接真实按钮
  - 本轮不迁移文件选择/保存等专门对话框
- 验证：
  - `cargo test native_dialog_host_operations_are_explicit_porting_contract --target-dir target_dialog_confirm_host_step1`
  - `cargo test windows_dialog_host_owns_edit_close_confirmation --target-dir target_dialog_confirm_host_step1`
  - `cargo test macos_dialog_host_tracks_shared_message_contract --target-dir target_dialog_confirm_host_step1`
  - `cargo test ui_host_porting_doc_covers_required_surfaces --target-dir target_dialog_confirm_host_step1`
  - `cargo check --target-dir target_dialog_confirm_host_step1`

## step95
- 扩大 `NativeDialogHost` 在主窗口粘贴失败路径的覆盖：
  - `show_paste_failure_message` 改走 `WindowsDialogHost::show_message`
  - `show_clipboard_write_failure_message` 改走 `WindowsDialogHost::show_message`
  - 无可用粘贴目标时的 warning 改走 `WindowsDialogHost::show_message`
  - 保持原有 warning 文案和行为：仅提示，内容仍已保留在剪贴板
  - 新增 `windows_dialog_host_owns_main_paste_warning_messages`，防止粘贴失败提示回退到直接 `platform_dialog::message_box`
- 验证：
  - `cargo test windows_dialog_host_owns_main_paste_warning_messages --target-dir target_dialog_paste_host_step1`
  - `cargo test native_dialog_host_operations_are_explicit_porting_contract --target-dir target_dialog_paste_host_step1`
  - `cargo test ui_host_porting_doc_covers_required_surfaces --target-dir target_dialog_paste_host_step1`
  - `cargo check --target-dir target_dialog_paste_host_step1`

## step96
- 将设置页多端同步/扫码绑定普通提示纳入 `NativeDialogHost`：
  - 新增 `show_native_dialog_message` 本地 helper，统一从设置动作调用 `WindowsDialogHost::show_message`
  - `execute_settings_sync_action` 中 WebDAV 未选择、局域网未选择、待允许请求提示、扫码链接未启动、扫码页不可用等 OK 提示不再直接调用 `platform_dialog::message_box_wide`
  - 保留非同步设置页和需要确认语义的旧路径，后续分批迁移到 `NativeDialogHost::confirm`
  - 新增 `windows_dialog_host_owns_settings_sync_messages`，防止多端同步普通提示回退到直接 Win32 MessageBox
- 验证：
  - `cargo test windows_dialog_host_owns_settings_sync_messages --target-dir target_settings_dialog_host_step1`
  - `cargo test native_dialog_host_operations_are_explicit_porting_contract --target-dir target_settings_dialog_host_step1`
  - `cargo test ui_host_porting_doc_covers_required_surfaces --target-dir target_settings_dialog_host_step1`
  - `cargo check --target-dir target_settings_dialog_host_step1`

## step97
- 将设置页分组弹窗纳入 `NativeDialogHost`：
  - 新增 `confirm_native_dialog` 本地 helper，统一从设置动作调用 `WindowsDialogHost::confirm`
  - `execute_settings_group_action` 中新建分组失败、重命名失败、未选择分组、删除分组失败等提示改走 `show_native_dialog_message`
  - 删除分组确认改走 `confirm_native_dialog`，由 `NativeDialogButtons::YesNo` 和 `NativeDialogResponse::Yes` 表达语义，不再依赖 Win32 `IDYES`
  - 新增 `windows_dialog_host_owns_settings_group_messages`，防止分组设置页回退到直接 `platform_dialog::message_box_wide`
- 验证：
  - `cargo test windows_dialog_host_owns_settings_group_messages --target-dir target_settings_group_dialog_host_step1`
  - `cargo test windows_dialog_host_owns_settings_sync_messages --target-dir target_settings_group_dialog_host_step1`
  - `cargo test native_dialog_host_operations_are_explicit_porting_contract --target-dir target_settings_group_dialog_host_step1`
  - `cargo test ui_host_porting_doc_covers_required_surfaces --target-dir target_settings_group_dialog_host_step1`
  - `cargo check --target-dir target_settings_group_dialog_host_step1`

## step98
- 将设置页平台/插件动作普通提示纳入 `NativeDialogHost`：
  - `execute_settings_platform_action` 中提示音文件选择失败、捕获当前窗口不可用、目标窗口类名不可用、WinOCR 目录检测失败、开源地址未配置、Win+V 屏蔽/恢复失败、资源管理器重启失败等提示改走 `show_native_dialog_message`
  - 文件选择器、打开文档、打开网页和更新检查动作仍保留在 Windows host 执行层，未被误收进普通 message host
  - 新增 `windows_dialog_host_owns_settings_platform_messages`，防止平台/插件设置动作回退到直接 `platform_dialog::message_box_wide`
- 验证：
  - `cargo test windows_dialog_host_owns_settings_platform_messages --target-dir target_settings_platform_dialog_host_step1`
  - `cargo test windows_dialog_host_owns_settings_group_messages --target-dir target_settings_platform_dialog_host_step1`
  - `cargo test native_dialog_host_operations_are_explicit_porting_contract --target-dir target_settings_platform_dialog_host_step1`
  - `cargo test ui_host_porting_doc_covers_required_surfaces --target-dir target_settings_platform_dialog_host_step1`
  - `cargo check --target-dir target_settings_platform_dialog_host_step1`

## step99
- 清零非 host 模块中的旧消息框直连：
  - `queue_cloud_sync` 的 WebDAV 未配置提示和同步失败提示改走 `show_native_dialog_message`
  - 全局热键冲突提示改走 `show_native_dialog_message`
  - 编辑记录保存失败提示改走 `show_native_dialog_message`
  - AI/文本处理结果错误提示改走 `show_native_dialog_message`
  - `sticker.rs` 图片 OCR 错误提示改走 `WindowsDialogHost::show_message`
  - 删除未使用的 `platform_dialog::message_box_wide`，并把底层 `message_box` 收为 `WindowsDialogHost` 内部私有实现
  - 新增 `windows_dialog_host_owns_non_host_message_boxes`，要求 `app.rs` 和 `sticker.rs` 不再直接调用旧 `platform_dialog::message_box/message_box_wide`
- 验证：
  - `cargo test windows_dialog_host_owns_non_host_message_boxes --target-dir target_dialog_host_cleanup_step1`
  - `cargo test windows_dialog_host_owns_settings_platform_messages --target-dir target_dialog_host_cleanup_step1`
  - `cargo test native_dialog_host_operations_are_explicit_porting_contract --target-dir target_dialog_host_cleanup_step1`
  - `cargo test ui_host_porting_doc_covers_required_surfaces --target-dir target_dialog_host_cleanup_step1`
  - `cargo check --target-dir target_dialog_host_cleanup_step1`

## step100
- 新增原生打开链接/路径 host，继续拆 Windows ShellExecute 直连：
  - `APP_CORE_API_VERSION` 提升到 `0.23`
  - 新增 `NativeShellOpenHost`、`NativeShellOpenHostOperation` 与 `REQUIRED_NATIVE_SHELL_OPEN_HOST_OPERATIONS`
  - 新增 `WindowsShellOpenHost`，将 `ShellExecuteW` 收口到 Windows shell host 内部
  - `src/shell.rs` 的 `open_path_with_shell` 保留 URL scheme 校验，但打开动作改为消费 `WindowsShellOpenHost`
  - 新增 `MacosShellOpenHost` scaffold，消费同一 `open_path` contract；后续用 `NSWorkspace.open` 接真实 macOS 打开行为
  - `docs/ui-host-porting.md` 更新到 `0.23`，补充 Native Shell Open Host 规则和 Windows/macOS reference
- 验证：
  - `cargo test native_shell_open_host_operations_are_explicit_porting_contract --target-dir target_shell_open_host_step1`
  - `cargo test windows_shell_open_host_owns_shell_execute_operations --target-dir target_shell_open_host_step1`
  - `cargo test macos_shell_open_host_consumes_shared_open_contract --target-dir target_shell_open_host_step1`
  - `cargo test macos_host_scaffold_tracks_current_core_contract --target-dir target_shell_open_host_step1`
  - `cargo test ui_host_porting_doc_covers_required_surfaces --target-dir target_shell_open_host_step1`
  - `cargo check --target-dir target_shell_open_host_step1`

## step101
- 新增原生文件选择 host，继续拆 Windows 文件对话框直连：
  - `APP_CORE_API_VERSION` 提升到 `0.24`
  - 新增 `NativeFileDialogHost`、`NativeFileDialogRequest`、`NativeFileDialogHostOperation` 与 `REQUIRED_NATIVE_FILE_DIALOG_HOST_OPERATIONS`
  - 新增 `WindowsFileDialogHost`，将提示音选择用的 PowerShell + WinForms `OpenFileDialog` 收口到 Windows file dialog host 内部
  - `src/shell.rs` 的 `pick_paste_sound_file` 只表达“选择 wav 提示音”的产品语义，并消费 `NativeFileDialogRequest`
  - 新增 `MacosFileDialogHost` scaffold，消费同一 `pick_file` contract；后续用 `NSOpenPanel` 接真实 macOS 文件选择
  - `docs/ui-host-porting.md` 更新到 `0.24`，补充 Native File Dialog Host 规则和 Windows/macOS reference
- 验证：
  - `cargo test native_file_dialog_host_operations_are_explicit_porting_contract --target-dir target_file_dialog_host_step1`
  - `cargo test windows_file_dialog_host_owns_open_file_dialog_operations --target-dir target_file_dialog_host_step1`
  - `cargo test macos_file_dialog_host_consumes_shared_pick_file_contract --target-dir target_file_dialog_host_step1`
  - `cargo test macos_host_scaffold_tracks_current_core_contract --target-dir target_file_dialog_host_step1`
  - `cargo test ui_host_porting_doc_covers_required_surfaces --target-dir target_file_dialog_host_step1`
  - `cargo check --target-dir target_file_dialog_host_step1`

## step102
- 将邮件合并 Excel 文件选择迁入 `NativeFileDialogHost`：
  - `mail_merge_native.rs` 删除独立 `ps_pick_excel_file`，不再在邮件合并窗口内直接创建 WinForms `OpenFileDialog`
  - 新增 `pick_excel_file`，通过 `WindowsFileDialogHost::pick_file` 请求 Excel/CSV 文件
  - 保留邮件合并读取 Excel 的 PowerShell/COM 执行逻辑，本轮只迁移“选择文件”这个 UI host 边界
  - 新增 `windows_file_dialog_host_owns_mail_merge_excel_picker`，防止邮件合并路径重新内嵌文件选择器
- 验证：
  - `cargo test windows_file_dialog_host_owns_mail_merge_excel_picker --target-dir target_mail_merge_file_dialog_step1`
  - `cargo test windows_file_dialog_host_owns_open_file_dialog_operations --target-dir target_mail_merge_file_dialog_step1`
  - `cargo test native_file_dialog_host_operations_are_explicit_porting_contract --target-dir target_mail_merge_file_dialog_step1`
  - `cargo test macos_file_dialog_host_consumes_shared_pick_file_contract --target-dir target_mail_merge_file_dialog_step1`
  - `cargo check --target-dir target_mail_merge_file_dialog_step1`

## step103
- 新增原生短文本输入 host，继续拆设置分组输入弹窗：
  - `APP_CORE_API_VERSION` 提升到 `0.25`
  - 新增 `NativeTextInputDialogHost`、`NativeTextInputDialogRequest`、`NativeTextInputDialogHostOperation` 与 `REQUIRED_NATIVE_TEXT_INPUT_DIALOG_HOST_OPERATIONS`
  - 新增 `WindowsTextInputDialogHost`，暂时包住现有 Win32 输入弹窗实现；后续可继续把窗口过程迁入更独立的 Windows host 模块
  - 设置页分组新建/重命名不再直接调用 `input_name_dialog`，改为发送 `prompt_text` 请求
  - 新增 `MacosTextInputDialogHost` scaffold，消费同一 `prompt_text` contract；后续用原生 `NSAlert` 输入框或 SwiftUI sheet 呈现
  - `docs/ui-host-porting.md` 更新到 `0.25`，补充 Native Text Input Dialog Host 规则和 Windows/macOS reference
- 验证：
  - `cargo test native_text_input_dialog_host_operations_are_explicit_porting_contract --target-dir target_text_input_dialog_host_step1`
  - `cargo test windows_text_input_dialog_host_owns_group_name_prompts --target-dir target_text_input_dialog_host_step1`
  - `cargo test macos_text_input_dialog_host_consumes_shared_prompt_contract --target-dir target_text_input_dialog_host_step1`
  - `cargo test macos_host_scaffold_tracks_current_core_contract --target-dir target_text_input_dialog_host_step1`
  - `cargo test ui_host_porting_doc_covers_required_surfaces --target-dir target_text_input_dialog_host_step1`
  - `cargo check --target-dir target_text_input_dialog_host_step1`

## step104
- 将设置分组输入请求文案提到共享模型：
  - 新增 `SettingsGroupTextInputKind` 与 `settings_group_text_input_request`
  - 设置页分组新建/重命名动作不再在 Windows action 分支内硬编码标题、标签和默认值，而是消费共享 request model
  - `MacosTextInputDialogHost` 新增测试，直接消费同一个分组重命名 request，为 macOS 设置页复用同一业务输入语义铺路
  - `docs/ui-host-porting.md` 补充规则：分组创建/重命名 prompt 先走共享模型，再进入平台 text input host
- 验证：
  - `cargo test settings_group_text_input_requests_are_shared_prompt_models --target-dir target_group_prompt_model_step1`
  - `cargo test windows_text_input_dialog_host_owns_group_name_prompts --target-dir target_group_prompt_model_step1`
  - `cargo test macos_text_input_dialog_host_consumes_shared_group_prompt_model --target-dir target_group_prompt_model_step1`
  - `cargo test ui_host_porting_doc_covers_required_surfaces --target-dir target_group_prompt_model_step1`
  - `cargo check --target-dir target_group_prompt_model_step1`

## step105
- 新增原生长文本编辑 host，继续拆行编辑弹窗：
  - `APP_CORE_API_VERSION` 提升到 `0.26`
  - 新增 `NativeEditTextDialogHost`、`NativeEditTextDialogRequest`、`NativeEditTextDialogHostOperation` 与 `REQUIRED_NATIVE_EDIT_TEXT_DIALOG_HOST_OPERATIONS`
  - 新增 `WindowsEditTextDialogHost`，暂时包住现有 Win32 多行编辑对话框
  - 行菜单“编辑记录”不再直接调用 `show_edit_item_dialog`，改为发送 `open_edit_text` 请求
  - 新增 `MacosEditTextDialogHost` scaffold，消费同一编辑 request；后续用 macOS 原生多行编辑窗口或 sheet 呈现
  - `docs/ui-host-porting.md` 更新到 `0.26`，补充 Native Edit Text Dialog Host 规则和 Windows/macOS reference
- 验证：
  - `cargo test native_edit_text_dialog_host_operations_are_explicit_porting_contract --target-dir target_edit_text_dialog_host_step1`
  - `cargo test windows_edit_text_dialog_host_owns_row_edit_action --target-dir target_edit_text_dialog_host_step1`
  - `cargo test macos_edit_text_dialog_host_consumes_shared_edit_contract --target-dir target_edit_text_dialog_host_step1`
  - `cargo test macos_host_scaffold_tracks_current_core_contract --target-dir target_edit_text_dialog_host_step1`
  - `cargo test ui_host_porting_doc_covers_required_surfaces --target-dir target_edit_text_dialog_host_step1`
  - `cargo check --target-dir target_edit_text_dialog_host_step1`

## step106
- 新增原生超级邮件合并窗口 host，继续拆插件/行菜单窗口入口：
  - `APP_CORE_API_VERSION` 提升到 `0.27`
  - 新增 `NativeMailMergeWindowHost`、`NativeMailMergeWindowRequest`、`NativeMailMergeWindowHostOperation` 与 `REQUIRED_NATIVE_MAIL_MERGE_WINDOW_HOST_OPERATIONS`
  - 新增 `WindowsMailMergeWindowHost`，暂时包住现有 Windows 原生邮件合并窗口
  - 插件页“打开超级邮件合并”和行菜单“超级邮件合并”不再直接调用 `launch_mail_merge_window*`，改为发送 `open_mail_merge` 请求
  - 新增 `MacosMailMergeWindowHost` scaffold，消费同一邮件合并打开 request
  - `docs/ui-host-porting.md` 更新到 `0.27`，补充 Native Mail Merge Window Host 规则和 Windows/macOS reference
- 验证：
  - `cargo test native_mail_merge_window_host_operations_are_explicit_porting_contract --target-dir target_mail_merge_window_host_step1`
  - `cargo test windows_mail_merge_window_host_owns_mail_merge_actions --target-dir target_mail_merge_window_host_step1`
  - `cargo test macos_mail_merge_window_host_consumes_shared_open_contract --target-dir target_mail_merge_window_host_step1`
  - `cargo test macos_host_scaffold_tracks_current_core_contract --target-dir target_mail_merge_window_host_step1`
  - `cargo test ui_host_porting_doc_covers_required_surfaces --target-dir target_mail_merge_window_host_step1`
  - `cargo check --target-dir target_mail_merge_window_host_step1`

## step107
- 收紧 Windows 超级邮件合并窗口的宿主边界，删除业务层残留的旧启动入口：
  - `WindowsMailMergeWindowHost` 从 `src/app.rs` 迁入 `src/mail_merge_native.rs`
  - `launch_mail_merge_window_with_excel` 改为模块私有，只允许 Windows 邮件合并 host 调用
  - `src/app.rs` 只导入并消费 `WindowsMailMergeWindowHost`，不再知道任何 `launch_*` 窗口函数
  - macOS 继续消费同一个 `NativeMailMergeWindowHost` contract，不引入 Windows 窗口实现
  - 更新守卫测试，防止原生启动函数重新泄漏到业务动作层

## step108
- 将 Windows 短文本输入窗口完整迁出 `src/app.rs`：
  - 新增 `src/windows_text_input_dialog.rs`，集中持有 `WindowsTextInputDialogHost`、Win32 窗口类、窗口过程、绘制和模态消息循环
  - `src/app.rs` 删除 `InputDlgData`、`input_dlg_proc`、`input_name_dialog` 及本地 host 套壳，只保留共享请求的消费
  - `input_dialog_host_event_from_message` 随窗口实现迁入专属 host 模块，避免输入弹窗消息适配继续挂在主窗口模块
  - 保留原有 Windows 原生外观、Enter/Escape 行为、主题刷新和输入校验
  - macOS 继续消费同一个 `NativeTextInputDialogHost` contract，不复制任何 Win32 代码

## step109
- 将 Windows 长文本编辑窗口完整迁出 `src/app.rs`，并把编辑契约改成可由 macOS 真正实现的形状：
  - `APP_CORE_API_VERSION` 提升到 `0.28`
  - `NativeEditTextDialogRequest` 改为携带标题、初始文本和首选尺寸，不再把数据库 `item_id` 交给平台 UI
  - 新增 `NativeEditTextSaveHandler` 与 `NativeEditTextDialogResult`，保存失败可留在原生窗口内重试，并返回最终窗口尺寸
  - 新增 `src/windows_edit_text_dialog.rs`，集中持有 Win32 窗口类、窗口过程、绘制、行号、模态循环和关闭确认
  - Windows host 不再直接访问 SQLite 或 settings；`src/app.rs` 负责读取初始文本、提交数据库更新和持久化最终尺寸
  - `MacosEditTextDialogHost` 同步消费初始文本、保存处理器和最终尺寸 contract，为后续原生多行编辑器提供完整边界
  - `src/app.rs` 删除原有 `EditDlgData`、窗口过程、绘制和薄 host 套壳
- 验证：
  - `cargo test native_edit_text_dialog_host_operations_are_explicit_porting_contract --target-dir target_edit_text_host_step4`
  - `cargo test windows_edit_text_dialog_is_owned_by_dedicated_host_module --target-dir target_edit_text_host_step4`
  - `cargo test windows_edit_text_dialog_host_owns_row_edit_action --target-dir target_edit_text_host_step4`
  - `cargo test macos_edit_text_dialog --target-dir target_edit_text_host_step4`
  - `cargo test platform_ui_message_decoding_stays_in_host_adapters --target-dir target_edit_text_host_step4`
  - `cargo check --target-dir target_edit_text_host_step4`
  - `cargo test -j 1 --target-dir target_edit_text_host_step4`（335 项通过）

## step110
- 开始迁移设置窗口宿主并让 macOS 复用真实设置模型：
  - `APP_CORE_API_VERSION` 提升到 `0.29`
  - `i18n_runtime.rs` 和 `settings_model.rs` 解除 Windows-only cfg；macOS 使用环境 locale fallback，Windows 继续使用系统 UI language API
  - `settings_ui_host.rs` 新增 `present_settings_window`，统一持有设置窗口类注册、原生创建、显示、聚焦、圆角和深色标题栏
  - `src/app.rs` 的 `open_settings_window` 不再直接注册 `SETTINGS_CLASS` 或调用 `CreateWindowExW`
  - 新增 `MacosSettingsWindowModel`，消费共享的六页导航、chrome/content paint plan、插件动态卡片和 WebDAV/局域网互斥 section
  - macOS 设置页仍保持原生 AppKit/SwiftUI 呈现方向，不复制 Windows 控件皮肤

## step111
- 将设置窗口展示能力提升为显式 native host contract：
  - `APP_CORE_API_VERSION` 提升到 `0.30`
  - `app_core.rs` 新增 `NativeSettingsWindowHost`、`NativeSettingsWindowRequest`、`NativeSettingsWindowPresentation` 和 required operation 清单
  - `WindowsSettingsWindowHost` 实现共享 contract，继续持有 Win32 设置窗口类注册、创建、显示和聚焦逻辑
  - `src/app.rs` 的 `open_settings_window` 改为通过 `NativeSettingsWindowRequest` 调用 host，不再消费 Windows 专用 request/presentation 名称
  - `MacosSettingsWindowHost` 同步消费同一 contract，先记录创建/聚焦请求，为后续 AppKit 设置窗口接入预留边界
  - `docs/ui-host-porting.md` 和 `docs/macos-ui.md` 更新到设置窗口 host contract

## step112
- 将主窗口启动创建能力提升为显式 native host contract：
  - `APP_CORE_API_VERSION` 提升到 `0.31`
  - `app_core.rs` 新增 `NativeMainWindowHost`、`NativeMainWindowRequest`、`NativeMainWindowPresentation` 和 main/quick handle pair
  - `WindowsMainWindowHost` 迁入 `src/app/hosts.rs`，集中持有 main/quick Win32 class 注册、窗口创建、初始显示/隐藏和圆角设置
  - `src/app.rs` 的 `run()` 不再直接注册 main/quick window class，也不再直接 `create_window_ex`
  - `MacosMainWindowHost` 同步消费同一 contract，先记录启动窗口请求并返回 main/quick 占位 handle，为后续 NSWindow/AppKit 接入预留边界
  - `docs/ui-host-porting.md` 和 `docs/macos-ui.md` 更新到主窗口 host contract

## step113
- 将主窗口搜索输入控件创建提升为显式 native host contract：
  - `APP_CORE_API_VERSION` 提升到 `0.32`
  - `app_core.rs` 新增 `NativeMainSearchControlHost`、`NativeMainSearchControlRequest` 和 `NativeMainSearchControlPresentation`
  - `WindowsMainSearchControlHost` 集中持有 Win32 `EDIT` 控件创建、初始可见性和 margin 设置
  - `src/app.rs` 的 `on_create()` 不再直接创建搜索框 `EDIT` 控件
  - `MacosMainSearchControlHost` 同步消费同一 contract，为后续原生 `NSSearchField` / SwiftUI search field 接入预留边界
  - `docs/ui-host-porting.md` 和 `docs/macos-ui.md` 更新到主窗口搜索控件 host contract

## step114
- 扩展主窗口搜索输入控件 host，继续清理 `app.rs` 对搜索框的直接 Win32 操作：
  - `APP_CORE_API_VERSION` 提升到 `0.33`
  - `NativeMainSearchControlHost` 新增 bounds、visible、text、set_text、focus 操作
  - `WindowsMainSearchControlHost` 统一持有搜索框移动/显隐、文本读写和聚焦
  - `src/app.rs` 的布局刷新、搜索重置、搜索开启、`EN_CHANGE` 文本读取和粘贴失败回焦不再直接调用 `move_window`、`set_visible`、`set_text`、`text`、`set_focus` 操作搜索框
  - `MacosMainSearchControlHost` 同步记录这些操作，为后续原生搜索框状态桥接预留边界

## step115
- 扩展主窗口 native host，继续清理 `app.rs` 对主窗口外观的直接 Windows 调用：
  - `APP_CORE_API_VERSION` 提升到 `0.34`
  - `NativeMainWindowHost` 新增 `apply_main_window_appearance`
  - `WindowsMainWindowHost` 统一持有主窗口圆角和深色 frame 应用
  - `src/app.rs` 删除旧的 `apply_main_window_region` helper，创建后和尺寸变化后的外观刷新都改走主窗口 host
  - `MacosMainWindowHost` 同步记录外观应用请求，为后续 NSWindow material / toolbar style / appearance 接入预留边界

## step116
- 扩展主窗口搜索输入控件 host，继续清理 `app.rs` 对搜索框字体资源的直接 Windows 调用：
  - `APP_CORE_API_VERSION` 提升到 `0.35`
  - `NativeMainSearchControlHost` 新增 `apply_search_style` 和 `release_search_style_resource`
  - `WindowsMainSearchControlHost` 统一持有搜索框字体创建、`WM_SETFONT` 应用和旧字体资源释放
  - `src/app.rs` 的 `refresh_search_font` 和 `WM_NCDESTROY` 不再直接创建/应用/释放搜索框 GDI 字体
  - `MacosMainSearchControlHost` 同步记录样式请求和资源释放，为后续 `NSSearchField` / SwiftUI search field 原生样式接入预留边界

## step117
- 扩展主窗口 native host，继续清理 `app.rs` 对主窗口生命周期的直接 Windows 调用：
  - `APP_CORE_API_VERSION` 提升到 `0.36`
  - `NativeMainWindowHost` 新增 `hide_main_window`、`request_main_window_close` 和 `destroy_main_window`
  - `WindowsMainWindowHost` 统一持有主窗口隐藏、请求关闭和销毁操作
  - `execute_main_ui_command` 的 Hide/Close 执行路径和 `handle_main_close_requested` 的 close-to-tray / destroy 路径改走主窗口 host
  - `MacosMainWindowHost` 同步记录 hide / request-close / destroy 请求，为后续 NSWindow 生命周期接入预留边界

## step118
- 扩展主窗口 native host，继续清理搜索入口激活时的直接 Windows 窗口调用：
  - `APP_CORE_API_VERSION` 提升到 `0.37`
  - `NativeMainWindowHost` 新增 `activate_main_window`
  - `WindowsMainWindowHost` 统一持有主窗口 show、topmost 前置和 force foreground 激活行为
  - `activate_window_for_search_input` 不再直接调用 `platform_window::show`、`set_pos(HWND_TOPMOST)` 或 `force_foreground`
  - `MacosMainWindowHost` 同步记录 activate 请求，为后续 `NSWindow.makeKeyAndOrderFront` / AppKit 激活接入预留边界

## step119
- 扩展主窗口 native host，继续清理 `app.rs` 对主窗口前台和隐藏状态的直接 Windows 调用：
  - `APP_CORE_API_VERSION` 提升到 `0.38`
  - `NativeMainWindowHost` 新增 `foreground_main_window`，区别于更强的 `activate_main_window`
  - `WindowsMainWindowHost` 统一持有轻量前台切换 `SetForegroundWindow`
  - 已有实例前台、失焦自动隐藏、粘贴计划立即隐藏、粘贴目标准备隐藏和粘贴失败回前台路径改走主窗口 host
  - `MacosMainWindowHost` 同步记录 foreground 请求，为后续 AppKit 普通前台切换接入预留边界

## step120
- 扩展主窗口 native host，继续清理已有实例路径中的直接 Windows 窗口调用：
  - `APP_CORE_API_VERSION` 提升到 `0.39`
  - `NativeMainWindowHost` 新增 `restore_main_window` 和 `close_main_window`
  - `WindowsMainWindowHost` 统一持有 `SW_RESTORE` 还原和 `PostMessage(WM_CLOSE)` 普通关闭请求
  - `run()` 的已有实例分支不再直接调用 `platform_window::restore` / `close`
  - `MacosMainWindowHost` 同步记录 restore / close 请求，为后续 AppKit window restore / performClose 接入预留边界

## step121
- 扩展主窗口 native host，继续清理 `app.rs` 对 Windows no-activate 样式的直接调用：
  - `APP_CORE_API_VERSION` 提升到 `0.40`
  - `NativeMainWindowHost` 新增 `set_main_window_activation_policy`
  - `WindowsMainWindowHost` 统一持有 `WS_EX_NOACTIVATE` 切换、frame refresh 和相关 hook refresh
  - `app.rs` 不再直接调用旧的 `set_main_window_noactivate_mode`
  - `MacosMainWindowHost` 同步记录 activation policy 请求，为后续 AppKit/SwiftUI 窗口焦点策略接入预留边界

## step122
- 扩展主窗口 native host，继续清理托盘/热键显示路径里的直接 Windows 窗口展示调用：
  - `APP_CORE_API_VERSION` 提升到 `0.41`
  - `NativeMainWindowHost` 新增 `present_main_window`
  - `WindowsMainWindowHost` 统一持有主窗口激活聚焦展示和快速窗口 no-activate 展示策略
  - `tray.rs` 的主窗口/快速窗口 show 路径不再直接调用 `platform_window::show`、`show_no_activate`、`set_pos(HWND_TOPMOST)`、`set_foreground` 或 `platform_input::set_focus`
  - `MacosMainWindowHost` 同步记录 present 请求，为后续 `NSWindow.orderFront` / `makeKeyAndOrderFront` 策略映射预留边界

## step123
- 扩展主窗口 native host，继续清理托盘/热键定位路径里的直接 Windows 窗口定位调用：
  - `APP_CORE_API_VERSION` 提升到 `0.42`
  - `NativeMainWindowHost` 新增 `set_main_window_bounds`
  - `WindowsMainWindowHost` 统一持有 `SetWindowPos` bounds 应用和 `SWP_NOZORDER | SWP_NOACTIVATE` 定位策略
  - `tray.rs` 继续负责计算主窗口位置，但不再直接调用 `platform_window::set_pos`
  - `MacosMainWindowHost` 同步记录 bounds 请求，为后续 AppKit `setFrame` / SwiftUI window placement 接入预留边界

## step124
- 复用主窗口 native host 的 bounds 契约，继续清理 `app.rs` 主窗口尺寸/DPI 修正路径：
  - `refresh_main_window_metrics`、`ensure_main_window_size_for_monitor` 和 `apply_main_system_dpi_compensation` 改走 `set_main_window_bounds`
  - 主窗口 metrics 刷新、PerMonitor DPI 尺寸修正和非 PerMonitor fallback compensation 不再直接调用 `platform_window::set_pos`
  - `WindowsMainWindowHost` 继续统一持有 bounds 应用策略，macOS 侧沿用 `set_main_window_bounds` scaffold 语义

## step125
- 扩展设置窗口 native host，继续清理 `app.rs` 设置窗口 DPI / 工作区调整里的直接 Windows 定位调用：
  - `APP_CORE_API_VERSION` 提升到 `0.43`
  - `NativeSettingsWindowHost` 新增 `set_settings_window_bounds`
  - `WindowsSettingsWindowHost` 统一持有设置窗口 `SetWindowPos` bounds 应用和 `SWP_NOZORDER | SWP_NOACTIVATE` 策略
  - `resize_settings_window_for_dpi_transition`、`ensure_settings_window_in_work_area`、`apply_settings_system_dpi_compensation` 和 `apply_dpi_suggested_rect` 改走 settings window host
  - `MacosSettingsWindowHost` 同步记录 settings bounds 更新，为后续 AppKit/SwiftUI 设置窗口 placement 接入预留边界

## step126
- 新增 transient window native host，继续清理 VV popup 浮层的直接 Windows 展示调用：
  - `APP_CORE_API_VERSION` 提升到 `0.44`
  - 新增 `NativeTransientWindowHost`，包含 `present_transient_window` 和 `hide_transient_window`
  - `WindowsTransientWindowHost` 统一持有 VV popup 的 no-activate `SetWindowPos` / show / hide 策略
  - `vv_popup_move_near_target`、`vv_popup_show`、`vv_popup_hide` 和 VV 分组菜单返回路径不再直接调用 `platform_window::set_pos` / `show_no_activate` / `hide`
  - `MacosTransientWindowHost` 同步记录 transient floating window actions，为后续 `NSPanel` / popover 实现预留边界

## step127
- 扩展 transient window native host，继续清理 VV popup 创建路径：
  - `APP_CORE_API_VERSION` 提升到 `0.45`
  - `NativeTransientWindowHost` 新增 `create_transient_window`
  - `WindowsTransientWindowHost` 统一持有 VV popup class registration 和 `create_window_ex`
  - `app.rs` 不再直接注册 VV popup class 或调用 `create_window_ex` 创建该浮层
  - `MacosTransientWindowHost` 同步记录 transient create request，为后续 `NSPanel` / popover 实例创建预留边界

## step128
- 扩展设置窗口 native host，继续清理设置窗口关闭生命周期路径：
  - `APP_CORE_API_VERSION` 提升到 `0.46`
  - `NativeSettingsWindowHost` 新增 `destroy_settings_window`
  - 设置窗口关闭按钮和 close requested 事件改走 `WindowsSettingsWindowHost`
  - `app.rs` 不再在设置窗口关闭路径直接调用 `platform_window::destroy(hwnd)`
  - `MacosSettingsWindowHost` 同步记录 settings destroy action，为后续 AppKit/SwiftUI settings window close 接入预留边界

## step129
- 新增设置下拉 native host，继续清理设置页 dropdown popup 创建/销毁路径：
  - `APP_CORE_API_VERSION` 提升到 `0.47`
  - 新增 `NativeSettingsDropdownHost`，包含 `present_settings_dropdown` 和 `destroy_settings_dropdown`
  - `WindowsSettingsDropdownHost` 统一持有设置下拉 popup 的 native 创建、定位展示和销毁入口
  - `app.rs` 不再直接调用 `show_settings_dropdown_popup` 或直接销毁 `st.dropdown_popup`
  - `MacosSettingsDropdownHost` 同步记录 dropdown present/destroy，为后续 AppKit/SwiftUI popover 或 menu 接入预留边界

## step130
- 复用设置控件 native host，继续清理设置窗口 metrics 刷新里的直接控件定位/显隐调用：
  - `refresh_settings_window_metrics` 的保存/关闭按钮 bounds 更新改走 `settings_host_set_bounds`
  - 页面控件显隐同步改走 `settings_host_set_visible`
  - 该路径不再直接调用 `platform_window::move_window` 或 `platform_window::set_visible`
  - macOS 侧沿用 `NativeSettingsControlHost` 的 bounds/visibility scaffold 语义，无需新增契约版本

## step131
- 复用设置控件 native host，继续清理设置页直接读取 Win32 控件文本路径：
  - 设置下拉当前值、热键预览、搜索引擎模板和跳过窗口类名等文本读取改走 `settings_host_text`
  - `app.rs` 生产路径不再直接调用 `platform_window::text`
  - macOS 侧沿用 `NativeSettingsControlHost::control_text` scaffold 语义，无需新增契约版本

## step132
- 扩展设置窗口 native host，继续清理热键录制里的直接焦点调用：
  - `APP_CORE_API_VERSION` 提升到 `0.48`
  - `NativeSettingsWindowHost` 新增 `focus_settings_window`
  - 设置页开启热键录制时改走 `WindowsSettingsWindowHost`
  - `app.rs` 设置页平台动作路径不再直接调用 `platform_input::set_focus(hwnd)`
  - `MacosSettingsWindowHost` 同步记录 settings focus action，为后续 AppKit/SwiftUI keyboard focus 接入预留边界

## step133
- 扩展设置窗口 native host，继续清理设置页滚动条拖拽里的直接鼠标捕获调用：
  - `APP_CORE_API_VERSION` 提升到 `0.49`
  - `NativeSettingsWindowHost` 新增 `capture_settings_pointer` / `release_settings_pointer`
  - 设置页滚动条拖拽开始/结束改走 `WindowsSettingsWindowHost`
  - `app.rs` 设置页滚动条路径不再直接调用 `platform_input::set_capture(hwnd)` / `platform_input::release_capture()`
  - `MacosSettingsWindowHost` 同步记录 pointer capture/release action，为后续 AppKit/SwiftUI drag tracking 接入预留边界

## step134
- 扩展主窗口 native host，继续清理主窗口拖拽路径里的直接鼠标捕获调用：
  - `APP_CORE_API_VERSION` 提升到 `0.50`
  - `NativeMainWindowHost` 新增 `capture_main_pointer` / `release_main_pointer`
  - 主窗口标题拖动、滚动条拖拽和文件拖拽导出前的捕获释放改走 `WindowsMainWindowHost`
  - `app.rs` 主窗口拖拽路径不再直接调用 `platform_input::set_capture(hwnd)` / `platform_input::release_capture()`
  - `MacosMainWindowHost` 同步记录 pointer capture/release action，为后续 AppKit/SwiftUI drag tracking 接入预留边界

## step135
- 复用主窗口 native host，继续清理托盘显示/隐藏路径里的直接窗口隐藏调用：
  - 托盘菜单和热键隐藏 main/quick 窗口改走 `app::hide_main_window`
  - `app::hide_main_window` 统一转发到 `WindowsMainWindowHost::hide_main_window`
  - `tray.rs` 生产路径不再直接调用 `platform_window::hide(...)`
  - macOS 侧沿用已有 `NativeMainWindowHost::hide_main_window` scaffold 语义，无需新增契约版本

## step136
- 扩展主窗口 native host，继续清理标题栏拖动路径里的 Win32 消息拼装：
  - `APP_CORE_API_VERSION` 提升到 `0.51`
  - `NativeMainWindowHost` 新增 `begin_main_window_drag`
  - 主窗口标题拖动开始改走 `WindowsMainWindowHost::begin_main_window_drag`
  - `app.rs` 标题拖动路径不再直接调用 `platform_window::force_foreground(hwnd)` 或发送 `WM_SYSCOMMAND/SC_MOVE`
  - `MacosMainWindowHost` 同步记录 begin-drag action，为后续 AppKit/SwiftUI native window drag 接入预留边界

## step137
- 复用主窗口 native host，继续清理托盘退出和关闭请求里的直接窗口销毁调用：
  - 新增 `app::destroy_main_window` facade，统一转发到 `WindowsMainWindowHost::destroy_main_window`
  - 托盘菜单 `Exit` 和 close-request fallback 改走 `destroy_main_window(hwnd)`
  - `app.rs` 对应路径不再直接调用 `platform_window::destroy(hwnd)`
  - macOS 侧沿用已有 `NativeMainWindowHost::destroy_main_window` scaffold 语义，无需新增契约版本

## step138
- 正式将当前共享 UI 架构命名为 `ZSUI`，为后续抽成独立开源 UI 框架预留边界：
  - `app_core.rs` 新增 `ZSUI_FRAMEWORK_NAME` 和 `ZSUI_FRAMEWORK_TAGLINE`
  - 新增 `docs/zsui.md`，定义 ZSUI 是共享 Rust UI logic + 原生平台 host 的架构，不是统一视觉皮肤
  - `docs/ui-host-porting.md` 和 `docs/macos-ui.md` 改用 ZSUI 命名
  - 文档测试覆盖 ZSUI 名称、tagline 和可复用/应用专属边界

## step139
- 复用主窗口 native host，继续清理主窗口销毁清理路径里的 quick 窗口直接销毁调用：
  - `handle_main_destroy` 中销毁 quick window 改走 `destroy_main_window(quick)`
  - `app.rs` 主窗口销毁清理路径不再直接调用 `platform_window::destroy(quick)`
  - macOS 侧沿用已有 `NativeMainWindowHost::destroy_main_window` scaffold 语义，无需新增契约版本

## step140
- 扩展 transient window native host，补齐 VV popup 生命周期的销毁边界：
  - `APP_CORE_API_VERSION` 提升到 `0.52`
  - `NativeTransientWindowHost` 新增 `destroy_transient_window`
  - `handle_main_destroy` 中销毁 VV popup 改走 `destroy_vv_popup_window(popup)`
  - `app.rs` VV popup 清理路径不再直接调用 `platform_window::destroy(popup)`
  - `MacosTransientWindowHost` 同步记录 `Destroy` action，为后续 AppKit floating panel 关闭接入预留边界

## step141
- 扩展主窗口 native host，继续清理主窗口创建阶段的窗口图标设置：
  - `APP_CORE_API_VERSION` 提升到 `0.53`
  - `NativeMainWindowHost` 新增 `set_main_window_app_icon`
  - `on_create` 加载应用图标后改走 `WindowsMainWindowHost::set_main_window_app_icon`
  - `app.rs` 主窗口创建路径不再直接发送 `WM_SETICON`
  - `MacosMainWindowHost` 同步记录 `SetAppIcon` action，为后续 NSWindow/NSApplication 图标接入预留边界

## step142
- 新增 paste target native host，继续清理粘贴目标窗口控制路径：
  - `APP_CORE_API_VERSION` 提升到 `0.54`
  - `NativePasteTargetHost` 新增 `force_paste_target_foreground` 和 `restore_paste_target_focus`
  - `MainTimerTask::Paste` 与 `paste_after_clipboard_ready_to_target` 切前台改走 `WindowsPasteTargetHost`
  - `restore_hotkey_focus_target` 改为转发到 `WindowsPasteTargetHost::restore_paste_target_focus`
  - `app.rs` 粘贴目标路径不再直接调用 `platform_window::force_foreground(target)` 或 `platform_input::set_focus(focus)`
  - `MacosPasteTargetHost` 同步记录 foreground/focus action，为后续 NSWorkspace/Accessibility 接入预留边界

## step143
- 将 Windows paste target host 从 `app/hosts.rs` 下沉到平台 host 模块，并复用到邮件合并路径：
  - 新增 `src/platform/paste_target.rs`，集中实现 `WindowsPasteTargetHost`
  - `src/platform/mod.rs` 注册 `paste_target` 模块
  - `app.rs` 和 `mail_merge_native.rs` 统一从 `crate::platform::paste_target::WindowsPasteTargetHost` 消费目标窗口前台控制
  - `mail_merge_native.rs` 的外部目标粘贴路径不再直接调用 `platform_window::force_foreground(target)`
  - macOS 侧沿用已有 `MacosPasteTargetHost` scaffold，无需新增契约版本

## step144
- 继续复用 paste target host，清理 VV popup 分组切换后的目标窗口前台控制：
  - VV popup 分组菜单选择后恢复目标窗口前台改走 `WindowsPasteTargetHost::force_paste_target_foreground`
  - `app.rs` VV popup window proc 不再直接调用 `platform_window::force_foreground(state.vv_popup_target)`
  - macOS 侧沿用已有 `MacosPasteTargetHost` scaffold，无需新增契约版本

## step145
- 扩展 paste target native host，清理热键透传直接编辑框写入路径：
  - `APP_CORE_API_VERSION` 提升到 `0.55`
  - `NativePasteTargetHost` 新增 `set_paste_target_text`
  - 热键透传 direct edit 写入改走 `WindowsPasteTargetHost::set_paste_target_text`
  - `app.rs` 该路径不再直接发送 `WM_SETTEXT` / `EM_SETSEL`
  - `MacosPasteTargetHost` 同步记录 `SetText` action，为后续 Accessibility/NSResponder 文本写入预留边界

## step146
- 继续扩展 paste target native host，清理 VV 目标文本输入能力检测里的 Win32 消息：
  - `APP_CORE_API_VERSION` 提升到 `0.56`
  - `NativePasteTargetHost` 新增 `paste_target_text_input_capabilities`
  - `PasteTargetTextInputCapabilities` 用平台无关字段表达 selection/chars/tab/arrows 能力
  - `vv_target_is_text_input_ready` 改走 `WindowsPasteTargetHost::paste_target_text_input_capabilities`
  - `app.rs` 该路径不再直接发送 `WM_GETDLGCODE` 或读取 `DLGC_*` 位掩码
  - `MacosPasteTargetHost` 同步记录 `QueryTextInputCapabilities` action，为后续 Accessibility 文本目标检测预留边界

## step147
- 新增 IME native host，清理 VV 输入法候选/组合窗口定位里的 Win32 IMM 消息：
  - `APP_CORE_API_VERSION` 提升到 `0.57`
  - 新增 `NativeImeHost`、`NativeImeCandidateAnchor`、`NativeImeCompositionAnchor`
  - 新增 `src/platform/ime.rs`，集中实现 `WindowsImeHost`
  - `vv_imm_overlay_anchor` 改走 `WindowsImeHost::candidate_anchor` / `composition_anchor`
  - `app.rs` VV IME 定位路径不再直接发送 `WM_IME_CONTROL`，也不再持有 `CandidateForm` / `CompositionForm`
  - `MacosImeHost` 同步记录 candidate/composition 查询 action，为后续 AppKit input context 或 Accessibility 定位预留边界

## step148
- 继续扩展 IME native host，清理 VV 文本目标判断里的默认 IME 窗口探测：
  - `APP_CORE_API_VERSION` 提升到 `0.58`
  - `NativeImeHost` 新增 `has_default_ime_window`
  - `vv_target_is_text_input_ready` 改走 `WindowsImeHost::has_default_ime_window`
  - `app.rs` 该路径不再直接调用 `platform_input::default_ime_window`
  - `MacosImeHost` 同步记录 `HasDefaultImeWindow` action，为后续 AppKit input context 可用性判断预留边界

## step149
- 新增 text caret native host，清理 VV 弹窗定位里的 caret 几何查询：
  - `APP_CORE_API_VERSION` 提升到 `0.59`
  - 新增 `NativeTextCaretHost` 和 `NativeTextCaretAnchor`
  - 新增 `src/platform/text_caret.rs`，集中实现 `WindowsTextCaretHost`
  - `vv_thread_caret_anchor` 改走 `WindowsTextCaretHost::thread_caret_anchor`
  - `vv_accessible_caret_anchor` 改走 `WindowsTextCaretHost::accessible_caret_anchor`
  - `app.rs` VV caret 定位路径不再直接读取 `GUITHREADINFO`、调用 `platform_accessibility::caret_rect` 或做 caret 坐标转换
  - `MacosTextCaretHost` 同步记录 accessible/thread caret 查询 action，为后续 AXFocusedUIElement/Accessibility 定位预留边界

## step150
- 继续扩展 text caret native host，清理 VV 弹窗定位里的 focus rect、cursor 和 target focus 解析：
  - `APP_CORE_API_VERSION` 提升到 `0.60`
  - `NativeTextCaretHost` 新增 `focus_rect_anchor`、`cursor_anchor`、`focus_handle_for_target`
  - `WindowsTextCaretHost` 集中处理 focused window rect、鼠标 fallback 坐标和 `GUITHREADINFO` focus 解析
  - `vv_focus_rect_anchor`、`vv_cursor_anchor`、`vv_focus_hwnd_for_target` 改走 `WindowsTextCaretHost`
  - `app.rs` VV 定位 fallback 路径不再直接调用 `platform_window::window_rect`、`platform_input::cursor_pos` 或读取 focus `GUITHREADINFO`
  - `MacosTextCaretHost` 同步记录 focus rect、cursor 和 ResolveFocus action，为后续 AppKit first responder / Accessibility 定位预留边界

## step151
- 新增 window identity native host，清理剪贴板来源和 VV 身份判断里的窗口/进程探针：
  - `APP_CORE_API_VERSION` 提升到 `0.61`
  - 新增 `NativeWindowIdentityHost`，包含 `process_name`、`class_name`、`root_handle`、`foreground_handle`、`is_current_process_window`
  - 新增 `src/platform/window_identity.rs`，集中实现 `WindowsWindowIdentityHost`
  - `clipboard_source_app_name`、`foreground_source_app_name`、`window_process_name`、`vv_window_class_name`、`vv_target_is_ignored` 改走 `WindowsWindowIdentityHost`
  - VV backspace 判断中的 root window process 查询改走 window identity host
  - `MacosWindowIdentityHost` 同步记录 process/class/root/foreground/self-window action，为后续 NSRunningApplication/NSWindow/Accessibility 身份查询预留边界

## step152
- 扩展 window identity native host，清理粘贴目标状态判断里的窗口存在/前台查询：
  - `APP_CORE_API_VERSION` 提升到 `0.62`
  - `NativeWindowIdentityHost` 新增 `exists`、`is_foreground`
  - `WindowsWindowIdentityHost` 集中代理窗口存在性和前台状态查询
  - `can_send_ctrl_v_to_target`、`paste_failure_message_for_target`、`effective_paste_target` 改走 `WindowsWindowIdentityHost`
  - `app.rs` 粘贴目标可用性和失败提示路径不再直接调用 `platform_window::exists(target)`、`platform_window::is_foreground(target)` 或 `platform_window::foreground()`
  - `MacosWindowIdentityHost` 同步记录 exists/is_foreground action，为后续 AppKit key window / Accessibility validity 判断预留边界

## step153
- 扩展 paste target native host，清理粘贴目标焦点归属判断里的 GUI thread 探针：
  - `APP_CORE_API_VERSION` 提升到 `0.63`
  - 新增 `PasteTargetFocusStatus`，表达 `Unknown`、`NoActiveFocus`、`InsideTarget`、`OutsideTarget`
  - `NativePasteTargetHost` 新增 `paste_target_focus_status`
  - `WindowsPasteTargetHost` 集中读取 `GUITHREADINFO`、解析当前 focus 是否仍属于目标窗口
  - `can_send_ctrl_v_to_target` 和 `paste_failure_message_for_target` 改走 `WindowsPasteTargetHost::paste_target_focus_status`
  - `app.rs` 粘贴目标可用性和失败提示路径不再直接读取 `GUITHREADINFO`、调用 `platform_window::gui_thread_info` 或判断 focus root
  - `MacosPasteTargetHost` 同步记录 QueryFocusStatus action，为后续 AppKit first responder / Accessibility 焦点归属判断预留边界

## step154
- 扩展 paste target native host，清理 VV 文本输入 readiness 判断里的 GUI thread / IME / caret 探针：
  - `APP_CORE_API_VERSION` 提升到 `0.64`
  - `NativePasteTargetHost` 新增 `paste_target_text_input_ready`
  - `WindowsPasteTargetHost` 集中处理 `vv_target_is_text_input_ready` 原有的 QQ/WPS、Word、Chromium、IME、accessibility caret 和 `WM_GETDLGCODE` 兼容判断
  - `app.rs` 的 `vv_target_is_text_input_ready` 缩减为 `WindowsPasteTargetHost::new().paste_target_text_input_ready(target)`
  - `app.rs` VV 文本输入 readiness 路径不再直接读取 `GUITHREADINFO`、调用 `platform_window::gui_thread_info`、查询 default IME window 或 accessibility caret
  - `MacosPasteTargetHost` 同步记录 QueryTextInputReady action，为后续 AppKit first responder / Accessibility 文本输入 readiness 判断预留边界

## step155
- 扩大 window identity native host 的消费范围，清理 VV popup target 生命周期里的窗口身份查询：
  - `APP_CORE_API_VERSION` 保持 `0.64`，不新增 contract operation
  - `vv_keyboard_hook_proc` 改走 `WindowsWindowIdentityHost::foreground_handle` 和 `exists`
  - `MainTimerTask::VvWatch`、`MainTimerTask::VvShow` 改走 `WindowsWindowIdentityHost::exists` / `is_foreground`
  - `ApplicationEvent::VvShowRequested` 的 fallback foreground target 改走 `WindowsWindowIdentityHost::foreground_handle` / `exists`
  - `app.rs` VV popup target 生命周期路径不再直接调用 `platform_window::foreground`、`platform_window::exists` 或 `platform_window::is_foreground`
  - macOS 侧沿用既有 `MacosWindowIdentityHost` ForegroundHandle / Exists / IsForeground action，为后续 NSWindow key/main window 与 Accessibility target validity 判断保持同一 contract

## step156
- 扩大 window identity native host 的消费范围，清理 paste/direct-edit helper 里的目标存在性判断：
  - `APP_CORE_API_VERSION` 保持 `0.64`，不新增 contract operation
  - `queue_async_image_paste_if_needed` 改走 `WindowsWindowIdentityHost::exists(target)`
  - `try_apply_to_explorer_rename` 改走 `WindowsWindowIdentityHost::exists(state.hotkey_passthrough_edit)`
  - `app.rs` 异步图片粘贴和 Explorer rename direct edit 路径不再直接调用 `platform_window::exists`
  - macOS 侧继续复用既有 `MacosWindowIdentityHost` Exists action，为后续异步 paste target validity / direct edit target validity 判断保持同一 contract

## step157
- 扩大 window identity native host 的消费范围，清理设置页捕获跳过窗口类名里的目标身份判断：
  - `APP_CORE_API_VERSION` 保持 `0.64`，不新增 contract operation
  - `SettingsAction::CaptureSkippedWindowClass` 改走 `WindowsWindowIdentityHost::exists(target)` 和 `is_current_process_window(target)`
  - `app.rs` 设置页捕获当前外部窗口类名路径不再直接调用 `platform_window::exists(target)` 或绕回 `is_app_window(target)`
  - macOS 侧继续复用既有 `MacosWindowIdentityHost` Exists / IsCurrentProcessWindow action，为后续设置页 Accessibility window target 捕获保持同一 contract

## step158
- 扩展 settings window native host，清理主窗口逻辑里直接刷新设置窗口的 Win32 调用：
  - `APP_CORE_API_VERSION` 提升到 `0.65`
  - `NativeSettingsWindowHost` 新增 `request_settings_window_repaint`
  - `WindowsSettingsWindowHost` 集中处理 settings window exists 检查和 `InvalidateRect`
  - tray LAN toggle 和 update-check ready 路径改走 `WindowsSettingsWindowHost::request_settings_window_repaint`
  - `app.rs` 这两条主窗口路径不再直接调用 `platform_window::exists(state.settings_hwnd)` 或 `platform_gdi::invalidate_rect(settings_hwnd)`
  - `MacosSettingsWindowHost` 同步记录 repaint action，为后续 AppKit/SwiftUI settings window refresh 接入预留边界

## step159
- 收紧 LAN ready 后 Cloud 设置页刷新路径，继续减少事件处理里的旧 Win32 UI 直连：
  - `APP_CORE_API_VERSION` 保持 `0.65`，复用既有 window identity host 和 settings repaint host
  - `handle_lan_sync_ready` 改为调用 `refresh_settings_cloud_page_after_lan_sync(state.settings_hwnd)`
  - 新 helper 用 `WindowsWindowIdentityHost::exists(settings_hwnd)` 判断设置窗口存在性，Cloud 页已构建时只同步 `settings_sync_page_state`
  - Cloud 页未构建或 settings state 不可用时，fallback 改走 `WindowsSettingsWindowHost::request_settings_window_repaint(settings_hwnd)`
  - `handle_lan_sync_ready` 不再直接调用 `platform_window::exists(state.settings_hwnd)` 或 `platform_gdi::invalidate_rect(state.settings_hwnd, ...)`
  - `MacosSettingsWindowHost` 同步记录 Cloud settings refresh action，为后续 LAN/WebDAV 状态变化驱动 SwiftUI/AppKit settings model refresh 预留入口

## step160
- 收紧 settings dropdown popup 生命周期里的窗口存在性查询：
  - `APP_CORE_API_VERSION` 保持 `0.65`，不新增 contract operation
  - 新增 `settings_dropdown_popup_exists(handle)` helper，内部复用 `WindowsWindowIdentityHost::exists(handle)`
  - settings 鼠标按下关闭 dropdown、settings destroy 清理 dropdown、通用 `close_settings_dropdown_popup` 不再直接调用 `platform_window::exists(st.dropdown_popup)`
  - dropdown 创建/销毁继续走 `WindowsSettingsDropdownHost`，存在性判断走 window identity host，职责边界更接近 macOS `NSMenu` / `NSPopUpButton` 接入方式
  - macOS 侧复用既有 `MacosWindowIdentityHost::exists` 和 `MacosSettingsDropdownHost` lifecycle scaffold，无需新增 core operation

## step161
- 扩展 settings control native host，开始清理设置控件级 repaint 的 Win32 直连：
  - `APP_CORE_API_VERSION` 提升到 `0.66`
  - `NativeSettingsControlHost` 新增 `request_control_repaint`
  - `WindowsSettingsControlHost` 集中处理 settings control 的 `InvalidateRect(handle, null(), 1)`，并暴露 `settings_host_request_repaint`
  - `handle_settings_control_selection` 中 dropdown 选择后的 `st.cb_*` 控件刷新改走 `repaint_settings_control`
  - `app.rs` 这条 settings dropdown selection 路径不再直接调用 `platform_gdi::invalidate_rect(st.cb_..., ...)`
  - `MacosSettingsControlHost` 新增 scaffold，记录 control create / visible / enabled / bounds / text / repaint / destroy，为后续 AppKit/SwiftUI settings controls 接入预留完整边界

## step162
- 扩大 settings control repaint host 的消费范围，继续清理控件级 Win32 repaint：
  - `APP_CORE_API_VERSION` 保持 `0.66`，复用 step161 新增的 `request_control_repaint`
  - 保存成功反馈和保存提示清除 timer 中的保存按钮刷新改走 `repaint_settings_control(st.btn_save)`
  - 热键录制按钮、热键预览 label、owner-draw hover 控件、LAN 复制链接按钮和跳过窗口类名编辑框刷新改走 `repaint_settings_control`
  - `app.rs` 上述 settings control 路径不再直接调用 `platform_gdi::invalidate_rect(st.btn_...)` / `st.lb_...` / `st.ed_...` / `sender`
  - macOS 侧继续复用 `MacosSettingsControlHost` repaint action，为 AppKit/SwiftUI control refresh 接线保持同一 contract

## step163
- 扩展 settings dropdown native host，清理 popup 外部点击判断里的 Win32 几何查询：
  - `APP_CORE_API_VERSION` 提升到 `0.67`
  - `NativeSettingsDropdownHost` 新增 `settings_dropdown_bounds`
  - `WindowsSettingsDropdownHost` 集中处理 dropdown popup 的 `platform_window::window_rect(handle)`，并暴露 `settings_dropdown_popup_bounds`
  - settings 鼠标按下关闭 dropdown 的外部点击判断改走 `settings_dropdown_popup_bounds(st.dropdown_popup)`
  - `app.rs` 该路径不再直接调用 `platform_window::window_rect(st.dropdown_popup)`
  - `MacosSettingsDropdownHost` 同步记录并返回 dropdown bounds，为后续 `NSPopUpButton` / `NSMenu` 或 popover 外部点击处理预留边界

## step164
- 扩展 settings window native host，清理 dropdown 外部点击判断里的 settings 窗口坐标转换：
  - `APP_CORE_API_VERSION` 提升到 `0.68`
  - `NativeSettingsWindowHost` 新增 `settings_window_client_to_screen`
  - `WindowsSettingsWindowHost` 集中处理 settings 窗口 client point 到 screen point 的 `platform_window::client_to_screen(handle, ...)`
  - settings 鼠标按下关闭 dropdown 的外部点击判断改走 `settings_window_client_to_screen(hwnd, ...)`
  - `app.rs` 该路径不再直接调用 `platform_window::client_to_screen(hwnd, ...)`
  - `MacosSettingsWindowHost` 同步记录 client-to-screen conversion，为后续 AppKit/SwiftUI 坐标系和 popup dismissal 接入预留边界

## step165
- 扩展 settings window native host，清理 settings pointer 事件里的 client bounds 查询：
  - `APP_CORE_API_VERSION` 提升到 `0.69`
  - `NativeSettingsWindowHost` 新增 `settings_window_client_bounds`
  - `WindowsSettingsWindowHost` 集中处理 settings 窗口内容区的 `platform_window::client_rect(handle)` 查询
  - `handle_settings_pointer_move` 和 `handle_settings_lbutton_down` 改走 `settings_window_client_bounds(hwnd)`
  - `app.rs` 上述 settings pointer 路径不再直接调用 `platform_window::client_rect(hwnd)`
  - `MacosSettingsWindowHost` 同步记录 client bounds queries，并从记录的窗口 bounds 返回 content bounds，为后续 AppKit/SwiftUI pointer hit testing 与 scroll layout 接入预留边界

## step166
- 扩展 settings window native host，清理 settings pointer/theme 路径里的窗口 repaint 直连：
  - `APP_CORE_API_VERSION` 提升到 `0.70`
  - `NativeSettingsWindowHost` 新增 `request_settings_window_area_repaint`
  - `WindowsSettingsWindowHost` 集中处理 settings window 的整窗/局部 `platform_gdi::invalidate_rect(handle, rect_ptr, erase)`
  - `cancel_settings_scroll_drag`、settings nav hover/leave、settings left-button down 的 nav/scrollbar 路径和 theme changed 改走 `repaint_settings_window_area` / `repaint_settings_window`
  - `app.rs` 上述 settings window pointer/theme 路径不再直接调用 `platform_gdi::invalidate_rect(hwnd, null/rect, ...)`
  - `MacosSettingsWindowHost` 同步记录 full/regional repaint requests，为后续 AppKit `setNeedsDisplay` / SwiftUI refresh 接入预留边界

## step167
- 扩大 settings window regional repaint contract 的消费范围，继续清理 settings 命令/计时器路径：
  - `APP_CORE_API_VERSION` 保持 `0.70`，复用 step166 新增的 `request_settings_window_area_repaint`
  - settings toggle、保存命令、滚动条隐藏 timer、粘贴提示音选择、OCR runtime 检测和更新检查启动路径改走 `repaint_settings_window`
  - `app.rs` 上述 settings command/timer/action 路径不再直接调用 `platform_gdi::invalidate_rect(hwnd, null(), ...)`
  - macOS 侧继续复用 `MacosSettingsWindowHost` 的 full/regional repaint request 记录，为后续 AppKit/SwiftUI settings refresh 接线保持同一 contract

## step168
- 继续扩大 settings window repaint host 的消费范围，收紧 settings metrics 刷新尾部的旧 Win32 直连：
  - `APP_CORE_API_VERSION` 保持 `0.70`，继续复用 `request_settings_window_area_repaint`
  - `refresh_settings_window_metrics` 完成 DPI/font/control bounds/visibility 同步后，整窗刷新改走 `repaint_settings_window(hwnd, true)`
  - `app.rs` 这条 settings metrics rebuild 路径不再直接调用 `platform_gdi::invalidate_rect(hwnd, null(), 1)`
  - macOS 侧继续复用 `MacosSettingsWindowHost` 的 repaint request 记录，为后续 SwiftUI/AppKit settings metrics rebuild 后刷新保持同一 host 边界

## step169
- 扩展 settings window native host，继续清理 settings DPI/工作区路径的窗口几何直连：
  - `APP_CORE_API_VERSION` 提升到 `0.71`
  - `NativeSettingsWindowHost` 新增 `settings_window_bounds`
  - `WindowsSettingsWindowHost` 集中处理 settings 窗口的 `platform_window::window_rect(handle)`，并暴露 `settings_window_bounds`
  - `refresh_settings_window_metrics` 的 client bounds 查询改走既有 `settings_window_client_bounds`
  - settings DPI transition、work-area fit、DPI compensation base/update 路径改走 `settings_window_bounds`
  - `app.rs` 上述 settings window geometry 路径不再直接调用 `platform_window::client_rect(hwnd)` / `platform_window::window_rect(hwnd)`
  - `MacosSettingsWindowHost` 同步记录并返回 window bounds queries，为后续 `NSWindow.frame` / SwiftUI window geometry 接入预留边界

## step170
- 扩展 settings control native host，清理 settings dropdown anchor 的 HWND 几何直连：
  - `APP_CORE_API_VERSION` 提升到 `0.72`
  - `NativeSettingsControlHost` 新增 `control_screen_bounds`
  - `WindowsSettingsControlHost` 集中处理 settings control 的 `platform_window::window_rect(handle)`，并暴露 `settings_host_screen_bounds`
  - 16 个 settings dropdown 打开路径统一改走 `settings_control_screen_rect_or_empty`
  - `app.rs` 上述 dropdown anchor 路径不再直接调用通用 `window_rect_or_empty(st.cb_...)`
  - `MacosSettingsControlHost` 同步记录 screen-bounds queries，并从最近一次 control bounds 更新返回 popup anchor，为后续 `NSView` 转 screen coordinates / SwiftUI popover anchor 接入预留边界

## step171
- 扩展 settings control native host，清理 settings metrics rebuild 后的 HWND 存在性查询：
  - `APP_CORE_API_VERSION` 提升到 `0.73`
  - `NativeSettingsControlHost` 新增 `control_exists`
  - `WindowsSettingsControlHost` 集中处理 settings control 的 `platform_window::exists(handle)`，并暴露 `settings_host_exists`
  - `refresh_settings_window_metrics` 清理失效 owner-draw controls 和 hover handle 时改走 `settings_host_exists`
  - `app.rs` 这条 settings control lifecycle 路径不再直接调用 `platform_window::exists(ctrl/st.hot_ownerdraw)`
  - `MacosSettingsControlHost` 根据 control create/destroy 记录返回 existence，为后续 AppKit view lifecycle / SwiftUI control identity 接入保持同一 contract

## step172
- 扩展 settings control native host，清理 owner-draw hover 的 Win32 child hit-test：
  - `APP_CORE_API_VERSION` 提升到 `0.74`
  - `NativeSettingsControlHost` 新增 `control_at_point`
  - `WindowsSettingsControlHost` 集中处理 `ChildWindowFromPointEx`，保留跳过 hidden/disabled controls 的原有语义，并暴露 `settings_host_control_at_point`
  - `handle_settings_pointer_move` 的 owner-draw hover 命中改走 settings control host
  - `app.rs` 该路径不再直接调用 `platform_window::child_from_point_ex` 或引用 `CHILD_FROM_POINT_SKIP_*`
  - `MacosSettingsControlHost` 根据 create/destroy、最新 bounds、visible 和 enabled 状态执行反向 hit-test，为后续 AppKit `hitTest` / SwiftUI pointer hover 接入保持同一 contract

## step173
- 扩展 settings window native host，清理 settings pointer move 的鼠标离开追踪直连：
  - `APP_CORE_API_VERSION` 提升到 `0.75`
  - `NativeSettingsWindowHost` 新增 `track_settings_pointer_leave`
  - `WindowsSettingsWindowHost` 集中处理 settings window 的 `TrackMouseEvent(TME_LEAVE | TME_HOVER)`，并暴露 `settings_window_track_pointer_leave`
  - `handle_settings_pointer_move` 改走 settings window host 请求 pointer leave tracking
  - `app.rs` 这条 settings pointer 路径不再直接调用通用 `ensure_mouse_leave_tracking(hwnd)`
  - `MacosSettingsWindowHost` 同步记录 pointer leave tracking 请求，为后续 `NSTrackingArea` / SwiftUI hover cleanup 接入保持同一 contract

## step174
- 扩展 settings window native host，清理 settings layout DPI 查询直连：
  - `APP_CORE_API_VERSION` 提升到 `0.76`
  - `NativeSettingsWindowHost` 新增 `settings_window_layout_dpi`
  - `WindowsSettingsWindowHost` 集中处理 settings window 的 `platform_dpi::layout_dpi_for_window(handle)`，并暴露 `settings_window_layout_dpi`
  - settings metrics rebuild、system metrics、move completed、WM_CREATE、WM_PAINT 和 open-existing DPI refresh 路径改走 settings window host
  - `app.rs` 上述 settings window DPI 路径不再直接调用 `platform_dpi::layout_dpi_for_window(hwnd/app.settings_hwnd)`
  - `MacosSettingsWindowHost` 同步记录 layout DPI queries，并返回默认 96，为后续 `NSWindow.backingScaleFactor` / SwiftUI display scale 接入保持同一 contract

## step175
- 将 ZSUI 从“单纯 UI 包装”继续推进成 Rust 基础 UI 分层：
  - `app.rs` 的迁移护栏测试外移到 `src/app_tests.rs`，主实现文件从约 12789 行降到约 9998 行
  - 新增 `app_core::zsui`，集中放置 `APP_CORE_API_VERSION`、ZSUI 名称/tagline 和基础层枚举
  - `ZsuiLayer` 明确区分 `CoreContracts` / `LayoutModel` / `RenderProtocol` / `NativeHost` 这些可复用基础层，以及 `ProductAdapter` 应用适配层
  - `docs/zsui.md` 补充 Rust foundation shape，避免 ZSUI 退化成某个产品页面的 UI wrapper
  - `docs/zsui.md` 明确外部 Rust 程序的复用目标：提供自己的 product adapter，并接入 Windows/macOS native host，而不是复制 ZSClip 业务代码
  - `docs/ui-host-porting.md` 调整权威来源说明：框架身份/版本来自 `app_core::zsui`，host contract 继续由 `app_core` 导出

## step176
- 继续将 ZSUI host contract 从应用大文件中分层，服务“其他 Rust 程序复用后接 Windows/macOS 原生 host”的目标：
  - 新增 `app_core::native_hosts`，承载 main window、main search control、settings window 和 settings dropdown 的 native host request/presentation/trait/required-operation contract
  - `app_core.rs` 继续 re-export `native_hosts::*`，保持既有 Windows 和 macOS scaffold 调用点兼容
  - `app_core.rs` 主文件减少约 359 行，窗口级 host contract 不再混在应用核心状态/业务 plan 中
  - `docs/zsui.md` 和 `docs/ui-host-porting.md` 标注 `src/app_core/native_hosts.rs` 为 native window/control host contract 来源

## step177
- 继续将 ZSUI 渲染基础协议从应用核心大文件中拆出，方便其他 Rust 程序接入 Windows/macOS 原生绘制后端：
  - 新增 `app_core::render_protocol`，承载 `Color`、文本语义、`TextStyle`、`NativeStyleResolver`、`TextLayout`、`Renderer` 及 required operation 清单
  - `app_core.rs` 继续 re-export `render_protocol::*`，保持现有 Windows GDI renderer/style resolver 和组件测试调用点兼容
  - `app_core.rs` 主文件继续缩小约 193 行，semantic render/text/style contract 不再混在应用状态/业务 plan 中
  - `docs/zsui.md` 和 `docs/ui-host-porting.md` 标注 `src/app_core/render_protocol.rs` 为 semantic render/text/style 协议来源

## step178
- 继续将 ZSUI 控件基础协议从应用核心大文件中拆出，方便其他 Rust 程序用同一 spec 创建 Windows/macOS 原生控件：
  - 新增 `app_core::control_protocol`，承载 `SettingsComponentKind`、`NativeControlFamily`、`NativeControlMapper`、`SettingsControlSpec`、`NativeSettingsControlHost` 及 required operation 清单
  - `app_core.rs` 继续 re-export `control_protocol::*`，保持 Windows settings host、Windows native style mapper 和 macOS settings control scaffold 调用点兼容
  - `app_core.rs` 主文件继续缩小约 168 行，native control spec/mapper/host contract 不再混在应用状态/业务 plan 中
  - `docs/zsui.md` 和 `docs/ui-host-porting.md` 标注 `src/app_core/control_protocol.rs` 为 native control protocol 来源

## step179
- 继续将 ZSUI 几何/布局基础协议从应用核心大文件中拆出，方便 Windows/macOS 原生 host 复用同一套窗口几何和 DPI 规则：
  - 新增 `app_core::layout_protocol`，承载 `UiRect`、`Point`、`Size`、`Rect`、DPI compensation、`ComponentId`、`LayoutProtocol` 和 shared non-host UI protocol 清单
  - `settings_nav_item_rect` 等 settings 专属布局函数保留在 `app_core.rs`，避免把产品/页面特定布局误标成基础协议
  - `app_core.rs` 继续 re-export `layout_protocol::*`，保持 Windows settings host、renderer、macOS scaffold 和组件测试调用点兼容
  - `docs/zsui.md` 和 `docs/ui-host-porting.md` 标注 `src/app_core/layout_protocol.rs` 为 geometry/layout protocol 来源

## step180
- 继续将 ZSUI 事件基础协议从应用核心大文件中拆出，并为其他 Rust 程序的 product adapter 预留应用事件类型：
  - 新增 `app_core::event_protocol`，承载 component lifecycle、`MouseButton`、`KeyState` 和 `UiEvent`
  - `UiEvent` 泛化为 `UiEvent<AppEvent = ApplicationEvent>`，ZSClip 继续默认使用自己的 `ApplicationEvent`，外部程序可替换为自己的应用事件类型
  - ZSClip 专属 `ApplicationEvent` / `MainAsyncEvent` 暂时仍留在 `app_core.rs`，避免把 LAN/VV/Cloud 等产品事件误标成 ZSUI 基础协议
  - `docs/zsui.md` 和 `docs/ui-host-porting.md` 标注 `src/app_core/event_protocol.rs` 为 lifecycle/input event protocol 来源

## step181
- 继续将 ZSUI 平台服务 host contract 从 `app_core.rs` 拆出，服务 Windows/macOS 原生 host 并行推进：
  - 新增 `app_core::host_protocol`，承载剪贴板、状态栏/菜单、临时窗口、IME、文本光标、对话框、shell 打开、窗口识别、粘贴目标、文件/文本/编辑/邮件合并窗口等平台服务 host trait 和 required-operation 清单
  - `app_core.rs` 继续 re-export `host_protocol::*`，保持 Windows 平台实现、macOS scaffold 和现有调用点兼容
  - `app_core.rs` 主文件从 4555 行降到约 3894 行，基础 host 协议不再继续堆在应用核心文件里
  - `docs/zsui.md`、`docs/ui-host-porting.md` 和 `docs/macos-ui.md` 标注 `src/app_core/host_protocol.rs` 为平台服务 host contract 来源

## step182
- 继续将 ZSUI core command/component contract 从 `app_core.rs` 拆出，避免基础 UI 骨架继续和 ZSClip 产品逻辑混在一个文件：
  - 新增 `app_core::command_protocol`，承载 `CommandId`、`CommandScope`、`CommandPayload`、`Command` 和 `CommandQueue`
  - 新增 `app_core::component_protocol`，承载 shared `Component` lifecycle/update/layout/render contract
  - `app_core.rs` 继续 re-export 两个 protocol，保持 Windows UI、settings host、macOS scaffold 和现有测试调用点兼容
  - `app_core.rs` 主文件从 3894 行降到约 3818 行，命令队列和组件协议成为可复用 Rust UI 基础层
  - `docs/zsui.md`、`docs/ui-host-porting.md` 和 `docs/macos-ui.md` 标注 command/component protocol 来源，明确 macOS host 不需要复用 Windows 消息派发代码

## step183
- 继续区分 ZSUI 基础事件协议和 ZSClip product adapter：
  - 新增 `app_core::product_adapter`，承载 ZSClip 专属 `NativeWindowToken`、`ApplicationEvent`、`MainAsyncEvent` 以及图片/文本异步 payload
  - `app_core.rs` 继续 re-export `product_adapter::*`，保持 Windows host adapter、macOS scaffold 和现有测试调用点兼容
  - `event_protocol::UiEvent<AppEvent = ApplicationEvent>` 继续保留默认 ZSClip 事件类型，但其他 Rust 程序仍可替换自己的 app event enum
  - `app_core.rs` 主文件从约 3819 行降到约 3770 行，产品异步事件不再直接堆在核心入口文件里
  - `docs/zsui.md`、`docs/ui-host-porting.md` 和 `docs/macos-ui.md` 标注 `src/app_core/product_adapter.rs` 为 ZSClip product adapter 来源

## step184
- 继续将 Windows/macOS 共用的主窗口命令路由从 `app_core.rs` 拆出：
  - 新增 `app_core::main_commands`，承载 `MainHostAction` / `MainHostExecutionPlan`、快捷键路由、`command_ids`、`menu_ids`、菜单 intent、托盘菜单/action plan
  - `app_core.rs` 继续 re-export `main_commands::*`，保持 Windows `app.rs`、settings host、macOS scaffold 和测试调用点兼容
  - `app_core.rs` 主文件从约 3770 行降到约 3233 行，主窗口命令/菜单/托盘路由不再继续堆在核心入口文件里
  - `docs/zsui.md`、`docs/ui-host-porting.md` 和 `docs/macos-ui.md` 标注 `src/app_core/main_commands.rs` 为 main command routing 来源
  - macOS 后续 menu/status item 处理可以消费同一套 semantic command intent，而不需要复制 Win32 command id 派发

## step185
- 继续补齐 Windows/macOS 共享 command/timer 边界：
  - 将 `main_menu_command_for_id`、`main_menu_command_for_shortcut_row_command`、`main_window_command_for_intent` 以及 menu id 静态/动态判定补迁到 `app_core::main_commands`
  - 新增 `app_core::timer_protocol`，承载 `MainTimerTask` / `MainTimerIds`、`SettingsTimerTask` / `SettingsTimerIds` 和 native timer id 到 semantic task 的映射函数
  - `app_core.rs` 继续 re-export `timer_protocol::*`，保持 Windows timer/message 路径和 macOS scaffold 调用点兼容
  - `app_core.rs` 主文件从约 3233 行降到约 3071 行，timer id 映射不再堆在核心入口文件里
  - `docs/zsui.md`、`docs/ui-host-porting.md` 和 `docs/macos-ui.md` 标注 `src/app_core/timer_protocol.rs` 为 timer protocol 来源

## step186
- 继续将主窗口行为策略从 `app_core.rs` 拆成可复用 ZSUI 基础协议：
  - 新增 `app_core::main_window_protocol`，承载 main/quick window 显示隐藏顺序、窗口位置/边缘隐藏恢复、热键注册与显示策略、主搜索框开关计划
  - `app_core.rs` 继续 re-export `main_window_protocol::*`，保持 Windows `tray.rs`、`app.rs`、平台热键映射和既有测试调用点兼容
  - `app_core.rs` 主文件从约 3071 行降到约 2498 行，主窗口行为策略不再继续堆在核心入口文件里
  - `macos_app` 新增契约测试，直接消费 `app_core::main_window_protocol` 的窗口位置和搜索可见性计划，防止 macOS scaffold 复制 Windows show/search 逻辑
  - `docs/zsui.md`、`docs/ui-host-porting.md` 和 `docs/macos-ui.md` 标注 `src/app_core/main_window_protocol.rs` 为 main window behavior protocol 来源

## step187
- 继续将 settings 相关共享协议从 `app_core.rs` 拆出，同时保留 ZSClip 产品动作和可复用 UI 边界的区分：
  - 新增 `app_core::settings_protocol`，承载 settings 尺寸/DPI helper、settings nav rect、settings control role 到稳定 command 的映射、分组 prompt model 和当前 `SettingsAction`
  - `app_core.rs` 继续 re-export `settings_protocol::*`，保持 Windows `app.rs`、`settings_ui_host.rs`、`settings_model.rs` 和 macOS scaffold 调用点兼容
  - `app_core.rs` 主文件从约 2498 行降到约 2342 行，settings 行为/命令边界不再继续堆在核心入口文件里
  - `macos_app` 新增契约测试，直接消费 `app_core::settings_protocol` 的 settings command role 映射，证明 macOS settings host 复用共享 Rust 协议
  - `docs/zsui.md`、`docs/ui-host-porting.md` 和 `docs/macos-ui.md` 标注 `src/app_core/settings_protocol.rs` 为 settings protocol 来源，并说明 WebDAV/LAN/WPS 等 `SettingsAction` 仍属于 ZSClip 产品适配候选

## step188
- 继续把 `app_core.rs` 收敛成 ZSUI 基础入口，而不是继续承载具体协议和大量测试：
  - 新增 `app_core::ui_surface_protocol`，承载 `UiHostSurface` 和 `REQUIRED_UI_HOST_SURFACES`
  - 将 `app_core.rs` 原有 `#[cfg(test)] mod tests` 迁移到 `src/app_core/tests.rs`，保留同一批契约测试但不再撑大核心入口文件
  - `app_core.rs` 主文件从约 2342 行降到约 69 行，现在基本只负责模块声明和 re-export，更像可复用 Rust UI foundation 的 crate 入口
  - `macos_app` 新增契约测试，直接消费 `app_core::ui_surface_protocol` 的 required surface 顺序和 adapter 名称，证明 macOS host surface 覆盖和 Windows 使用同一份 surface 清单
  - `docs/zsui.md`、`docs/ui-host-porting.md` 和 `docs/macos-ui.md` 标注 `src/app_core/ui_surface_protocol.rs` 为 required host surface 来源

## step189
- 回到 Windows 旧 UI 本体清理 settings action 执行，同时给 macOS 建立同一份 action route：
  - `app_core::settings_protocol` 新增 `SettingsActionRoute` 和 `settings_action_route`，将 settings action 明确分为 `Sync` / `Group` / `Platform` 三个执行域
  - Windows `dispatch_settings_ui_event` 改为按 shared route 单次分发，不再依次尝试三个 executor
  - 新增 `src/app/settings_actions.rs`，承载 Windows/ZSClip 的 sync、group、platform settings action executor；`app.rs` 从约 9999 行降到约 9508 行
  - `macos_app` 契约测试验证 WebDAV、分组和平台 action 使用同一份 `SettingsActionRoute`，后续 AppKit/SwiftUI 可实现自己的三个执行域
  - `app_tests` 的 dialog、repaint、identity、text-input 和 mail-merge 护栏改为检查新的 settings action 模块，并新增 Windows shared-route dispatch 护栏
  - `docs/zsui.md`、`docs/ui-host-porting.md` 和 `docs/macos-ui.md` 明确 shared route 属于 ZSUI settings protocol，Windows 具体执行器属于 ZSClip product adapter

## step190
- 将 settings action route 继续提升为 Windows/macOS/Linux 都可实现的 executor contract：
  - `app_core::settings_protocol` 新增 `SettingsActionExecutor` 和 `dispatch_settings_action`
  - Windows 新增 `WindowsSettingsActionExecutor`，`app.rs` 只创建 executor 并调用 shared dispatcher，不再知道 sync/group/platform 的 match 细节
  - macOS 新增 `MacosSettingsActionExecutor` / `MacosSettingsActionContext` scaffold，并通过 shared dispatcher 验证三个 action domain
  - `app_core` 新增 fake executor 契约测试，证明 shared dispatcher 只调用对应 action domain
  - `APP_CORE_API_VERSION` 从 `0.76` 提升到 `0.77`
  - `docs/zsui.md` 将 GTK4/libadwaita Linux native host 加入复用目标；同一 executor contract 可供后续 Linux host 直接实现

## step191
- 补齐 main window 与 settings window 对称的 native host 边界，继续同步清理 Windows 旧 UI 和推进 macOS host：
  - `NativeMainWindowHost` 新增 pointer-leave tracking、整窗/局部 repaint、layout DPI、client bounds 和 window bounds 五项操作
  - Windows 主窗口实现区不再直接调用 `InvalidateRect`、`layout_dpi_for_window(hwnd)`、`client_rect(hwnd)`、`window_rect(hwnd)` 或旧的 mouse-leave helper
  - `WindowsMainWindowHost` 集中映射上述能力到 Win32；`app.rs` 只消费语义化 helper
  - `MacosMainWindowHost` 同步记录 repaint、pointer leave、display scale 和几何查询，为 AppKit `setNeedsDisplay`、tracking area、`backingScaleFactor` 和 `NSWindow.frame` 接线预留统一 contract
  - `APP_CORE_API_VERSION` 从 `0.77` 提升到 `0.78`，required native main window operations 从 16 增至 21

## step192
- 将 Windows 主窗口 GDI 绘制执行从产品入口中拆出，并扩大 macOS 对共享 render plan 的实际消费：
  - 新增 `src/app/main_renderer.rs`，集中执行 `MainPaintCommand`、文本、图标、图片缩略图、双缓冲和 `BeginPaint` / `EndPaint`
  - `app.rs` 的 `WM_PAINT` 只调用 `paint_main_window(hwnd)`，不再承载主列表 GDI command 执行器
  - `app.rs` 从约 9492 行降到约 9115 行；迁出的 Windows renderer 约 379 行
  - `MacosMainWindowModel` 新增通用 `render_plan(MainRenderInput)`，不再只支持空历史首屏
  - macOS 契约测试覆盖三行数据、选中态、搜索框、滚动条和回到顶部覆盖层，证明 AppKit/SwiftUI host 可直接消费同一份 populated render plan
  - 新增 source guard，阻止 `paint`、main paint/text executor 和 RGBA GDI 绘制重新回流 `app.rs`

## step193
- 将 Windows 主窗口输入执行从产品入口中拆出，并让 macOS scaffold 直接消费同一套输入计划：
  - 新增 `src/app/main_input.rs`，集中执行滚轮、指针移动、左/右键、双击、快捷键和 `WM_NCHITTEST` 的 Windows adapter 逻辑
  - `app.rs` 只负责把 `UiEvent` / `WM_NCHITTEST` 派发给 `main_input`，不再承载主输入处理函数
  - Windows input executor 继续消费 `MainUiLayout` 的 pointer move/down/up 计划、列表选择计划和 `MainShortcutExecutionPlan`
  - `MacosMainWindowModel` 新增 pointer move/down/up 和 shortcut execution plan 入口，证明 AppKit/SwiftUI host 可直接复用 Rust 输入契约
  - 新增 source guard，阻止主输入处理器重新回流 `app.rs`，并确保捕获/释放/拖动仍通过 `NativeMainWindowHost`

## step194
- 将 Windows settings 窗口输入执行从 `app.rs` 拆出，并补齐 macOS settings 输入计划消费：
  - 新增 `src/app/settings_input.rs`，集中执行 settings 指针移动、离开、按下/释放、滚轮、热键录制、DPI/主题/销毁和 `dispatch_settings_ui_event`
  - `app.rs` 只保留 `settings_wnd_proc` 的事件来源和派发调用，不再承载 settings 输入处理函数
  - Windows settings input executor 继续消费 `settings_pointer_move_transition`、`settings_pointer_down_target`、`settings_scroll_delta_for_wheel`、`settings_nav_hover_transition` 和 shared settings action dispatcher
  - `MacosSettingsWindowModel` 新增导航 hover、pointer move/down 和 wheel delta 入口，并新增测试证明 AppKit/SwiftUI settings host 复用同一套 Rust 输入计划
  - 新增 source guard，阻止 settings 输入处理器重新回流 `app.rs`，并确保 settings pointer capture/release 和 repaint 仍通过 host 边界

## step195
- 将 Windows settings dropdown 执行从 `app.rs` 拆出，并让 macOS scaffold 直接消费同一组选项模型：
  - 新增 `src/app/settings_dropdown.rs`，集中执行 settings dropdown anchor、当前值索引、popup request 创建、关闭和 config 打开 helper
  - `app.rs` 只保留 command 分发入口，调用 `open_settings_dropdown_for_control`，不再承载 dropdown popup executor
  - Windows dropdown executor 继续通过 `NativeSettingsDropdownRequest` / `WindowsSettingsDropdownHost` 创建和销毁 native popup，存在性判断仍走 `WindowsWindowIdentityHost`
  - `MacosSettingsDropdownHost` 测试改为消费 `settings_dropdown_max_items_labels` 和 `settings_dropdown_index_for_max_items`，避免 macOS scaffold 复制 Windows 下拉文案/索引规则
  - 新增 source guard，阻止 dropdown executor、popup presentation 和 popup existence helper 重新回流 `app.rs`

## step196
- 将 Windows settings command/timer/dropdown selection 执行从 `app.rs` 拆出，并补齐 macOS 对共享 command/timer 协议的消费：
  - 新增 `src/app/settings_commands.rs`，集中执行 settings `Command` drain、保存反馈、toggle、timer task 和 dropdown selection 更新
  - `app.rs` 只保留 settings window proc 和事件来源，settings input executor 调用 `queue_settings_command` / `drain_settings_ui_commands` / timer / selection helper
  - `MacosSettingsWindowModel` 契约测试补充 `settings_timer_task_for_id`，证明 macOS RunLoop/DispatchSource 可直接映射到 shared `SettingsTimerTask`
  - 新增 source guard，阻止 settings command executor、timer executor、dropdown selection executor 和 save feedback helper 重新回流 `app.rs`
  - `app.rs` 从约 7710 行降到约 7439 行，settings command 执行器约 274 行

## step197
- 将 Windows main row command 执行从 `app.rs` 拆出，并补齐 macOS 对共享 row action plans 的消费：
  - 新增 `src/app/main_row_commands.rs`，集中执行 row external/dialog/current-item/data/group command、pin/delete selection data plan 和 row menu intent
  - `app.rs` 只保留主窗口 command dispatch 中对 `execute_row_command` 的调用，不再承载 row command executor
  - `main_input.rs` 继续调用 `execute_delete_selection_data_plan`，该 helper 作为同级模块接口暴露，保持键盘删除路径复用同一 data plan executor
  - `macos_app` 新增契约测试，直接消费 `main_row_external_action_plan`、`main_row_current_item_action_plan` 和 `main_row_dialog_action_plan`
  - 新增 source guard，阻止 row command executor、row data executor、row dialog executor 和 context-row selection helper 重新回流 `app.rs`
  - `app.rs` 从约 7439 行降到约 7004 行，main row command 执行器约 442 行

## step198
- 将 Windows main row/group popup menu presentation 从 `app.rs` 拆出，并补齐 macOS 对共享 row menu plan 的消费：
  - 新增 `src/app/main_popup_menus.rs`，集中把 `main_row_menu_plan` / `main_group_filter_menu_plan` 转成 `NativePopupMenuEntry`，再交给 `WindowsPopupMenuHost`
  - `main_input.rs` 继续负责右键入口和命令回流，`app.rs` 不再承载 row/group popup menu presenter
  - `macos_app` 的 popup host 契约测试补充直接消费 shared row menu plan，证明 AppKit/SwiftUI `NSMenu` 后续可复用同一套 Rust 菜单计划
  - 新增 source guard，阻止 row/group popup presenter、submenu 构建和 popup host 调用重新回流 `app.rs`
  - `app.rs` 从约 7004 行降到约 6847 行，main popup menu presenter 约 160 行

## step199
- 将 Windows VV popup floating UI presentation 从 `app.rs` 拆出，并补齐 macOS 对共享 VV popup plans 的消费：
  - 新增 `src/app/vv_popup.rs`，集中执行 VV popup native window 创建/隐藏/销毁、IME/caret anchor 定位、分组 popup menu、render plan 绘制和 click hit-test
  - `app.rs` 保留 VV keyboard hook、target identity/backspace 和主窗口消息入口，但不再承载 VV popup window presenter / wndproc
  - `macos_app` 新增契约测试，直接消费 `MainVvPopupLayout` render/hit plan 和 `main_vv_select_plan`，证明 future `NSPanel` / popover 可复用同一套 Rust VV 行为
  - 更新 source guard，阻止 VV popup presenter、IME/caret anchor adapter 和 transient host 调用重新回流 `app.rs`
  - `app.rs` 从约 6847 行降到约 6295 行，VV popup presenter 约 556 行

## step200
- 将 Windows VV keyboard hook / target identity / backspace adapter 从 `app.rs` 拆出，并继续保持 macOS 只消费 shared VV plans：
  - 新增 `src/app/vv_hook.rs`，集中执行 VV low-level keyboard hook、target ignore、text-input readiness、QQ/WPS/browser backspace 兼容策略和 hook install/uninstall
  - `app.rs` 只保留 VV timer/application event 入口、通用 clipboard source/window process helper 和 `handle_vv_select`，不再承载 VV keyboard hook 函数
  - Windows paste-target/window-identity 护栏改为检查 `vv_hook.rs`，并确认 hook/backspace/text-input readiness 不回流 `app.rs`
  - macOS 路线继续通过 `macos_main_window_consumes_shared_vv_popup_plans` 验证 `MainVvPopupLayout` / `main_vv_select_plan`，低层 trigger 由未来 AppKit/event tap host 自己实现
  - `app.rs` 从约 6295 行降到约 6024 行，VV hook adapter 约 275 行

## step201
- 将 Windows main search executor 从 `app.rs` 拆出，并继续让 macOS 搜索框消费同一 native host contract：
  - 新增 `src/app/main_search.rs`，集中执行主窗口搜索控件 layout、字体/style resource、搜索重置、show 前准备、visibility plan、focus 激活和 `EDIT` 文本变化 debounce
  - `app.rs` 只保留主窗口 control command 分发入口，先交给 `handle_search_control_command`，再处理菜单 command，不再承载 search visibility executor
  - Windows main search 护栏改为检查 `main_search.rs`，并确认 `layout_children`、`search_visibility_plan_for_request`、`activate_window_for_search_input` 不回流 `app.rs`
  - macOS 路线继续通过 `MacosMainSearchControlHost` 消费 `NativeMainSearchControlHost`，后续可映射为 `NSSearchField` / SwiftUI search field
- 修复打开慢的启动路径阻塞：
  - `on_create` 不再同步执行 `db_reconcile_dedupe_signatures`，避免窗口首屏被全库扫描和图片数据读取阻塞
  - 新增后台 `spawn_startup_data_reconcile`，完成后通过 `StartupDataReconciled` 应用事件回主窗口；只有真的删除重复项时才刷新列表和 LAN latest
  - `db_reconcile_dedupe_signatures` 只为缺 signature 的旧行读取 `text_data/file_paths/image_data/image_path`，已有 signature 的记录不再重读大块图片 payload

## step202
- 将 Windows main paste executor 从 `app.rs` 拆出，并继续保持 macOS 复用 shared paste/clipboard/paste-target contract：
  - 新增 `src/app/main_paste.rs`，集中执行复制选择、写入剪贴板、纯文本粘贴、异步图片粘贴、direct edit passthrough、paste completion plan、目标窗口选择、失败提示和延迟 `Ctrl+V`
  - `app.rs` 保留 paste timer、async image result 和 VV select 等入口，但不再承载 `copy_selection_to_clipboard`、`apply_item_to_clipboard`、`paste_selected`、`paste_after_clipboard_ready_to_target`、paste target 状态查询等 executor
  - Windows paste 守卫改为检查 `main_paste.rs`，并确认 shared `main_paste_preparation_plan` / `main_paste_completion_plan` 继续通过 `WindowsClipboardHost`、`WindowsPasteTargetHost` 和 `WindowsWindowIdentityHost` 执行
  - macOS 路线继续由 `MacosClipboardHost`、`MacosPasteTargetHost` 和 shared paste plans 承接，后续可映射到 `NSPasteboard`、Accessibility focus restore 和原生 paste target 判断

## step203
- 将 Windows clipboard capture executor 从 `app.rs` 拆出，为 macOS/Linux 原生剪贴板监听留出同级 adapter 边界：
  - 新增 `src/app/main_clipboard_capture.rs`，集中执行捕获开关、clipboard retry、来源 app 过滤、浏览器下载选择过滤、文件/文本/图片捕获路由、Windows screen clip 识别和图片 payload 归一化
  - `app.rs` 只保留 clipboard changed/timer/menu toggle 的事件入口，不再承载 `capture_clipboard`、`browser_download_selection_should_skip`、`paths_look_like_windows_screen_clip`、`normalize_captured_text` 等 capture executor
  - Windows capture 守卫改为检查 `main_clipboard_capture.rs`，确认剪贴板读取继续通过 `WindowsClipboardHost::read_text/read_image_rgba/read_file_paths` 和 named format helpers
  - macOS 路线后续可把 `NSPasteboard.changeCount`、文件 URL、文本、图片和 source filtering 映射到同一产品捕获语义；Linux 路线后续可用 GTK/GDK clipboard monitor 或 wlroots/X11 bridge 接入同一边界

## step204
- 采用批量迁移节奏：先连续拆 Windows adapter，再统一测试/构建/清理，减少重复 release build。
- 将 Windows main-window lifecycle/geometry adapter 从 `app.rs` 拆出：
  - 新增 `src/app/main_window.rs`，集中执行隐藏窗口内存回收、滚动条反馈、main layout DPI、窗口 size/fit、系统 DPI 补偿、主窗口 size/move/close/lifecycle/destroy/DPI 事件以及 `WindowsMainWindowHost` wrapper
  - `app.rs` 保留 `wnd_proc`、`on_create` 和事件派发入口，但不再承载 `handle_main_window_size`、`handle_main_close_requested`、`refresh_main_window_metrics`、`ensure_main_window_size_for_monitor`、pointer capture/drag wrapper 等主窗口 adapter
- 将 Windows main platform bindings 从 `app.rs` 拆出：
  - 新增 `src/app/main_platform_bindings.rs`，集中执行主热键/纯文本粘贴热键注册、注销、冲突提示、剪贴板 listener 注册/注销和 global hotkey 事件入口
  - `app.rs` 只通过 shared `UiEvent::GlobalHotkey` / startup/destroy 路径调用该模块，不再直接承载 hotkey/listener 注册实现
- 新增 source guard，阻止 main-window lifecycle/geometry 和 hotkey/listener adapter 回流 `app.rs`
- macOS 路线后续可把这两块映射到 `NSWindow` lifecycle、AppKit event monitor、`NSPasteboard` monitor；Linux 路线后续可映射到 GTK/libadwaita window、GDK clipboard monitor 和 Wayland/X11/global-shortcut bridge

## step205
- 将 Windows row tools / AI-OCR side effects 从 `app.rs` 拆出，为后续 LLMs/skills 能力留出明确产品工具边界：
  - 新增 `src/app/main_row_tools.rs`，集中执行 AI text cleanup、Markdown 清理、OCR image input 准备、Baidu/WinOCR job、文本翻译 job、拖拽导出文件 materialization、paste passthrough 清理
  - `app.rs` 不再承载 `ai_clean_text`、`maybe_ai_clean_text`、`spawn_image_ocr_job`、`spawn_text_translate_text_job`、`begin_row_drag_export`、`clear_hotkey_passthrough_state`
  - `main_paste.rs` 继续调用 AI clean 和 passthrough cleanup；`main_row_commands.rs` 继续调用 OCR/translation job；`main_input.rs` 继续调用 row drag export 和 OCR input 预检
  - 新增 source guard，阻止 row tools 回流 `app.rs`
  - macOS/Linux 后续可以复用 AI cleanup、OCR/translation job dispatch 和 item materialization，只替换原生 drag/drop 或平台 OCR host

## step206
- 继续采用批量迁移后统一验证的节奏，将 Windows settings-window adapter 从 `app.rs` 拆出：
  - 新增 `src/app/settings_window.rs`，集中 settings window proc、open/focus/destroy、DPI/work-area geometry、pointer capture、repaint forwarding 和 Cloud settings refresh
  - `app.rs` 只保留 settings command/application event 调用入口，不再承载 `settings_wnd_proc`、窗口生命周期和几何 adapter
  - 新增 source guard，阻止 settings-window lifecycle/geometry/paint entry 回流 `app.rs`
  - `app.rs` 从约 3940 行降到约 3182 行，settings-window adapter 约 794 行
- 同步推进 macOS settings UI：
  - `MacosSettingsWindowModel` 新增共享 window-fit 和 DPI/scale-transition plan 入口
  - 新增契约测试，证明未来 AppKit `NSWindow.frame` / backing-scale 更新可以直接消费 Rust 几何计划，而不复制 Win32 adapter
- Linux 路线同步记录为复用同一 settings geometry plans 和 `NativeSettingsWindowHost`，只替换 GTK/libadwaita lifecycle、scale-factor 和 repaint bridge

## step207
- 将 Windows 主 command/timer/event/async executor 从 `app.rs` 拆出：
  - 新增 `src/app/main_events.rs`，集中执行 main menu command、shared host command、command queue drain、timer task、`UiEvent`、ZSClip `ApplicationEvent` / `MainAsyncEvent` 和文本处理结果
  - `app.rs` 只保留 `wnd_proc`、startup/create 和少量 control command 入口，不再承载主事件执行器
  - 新增 source guard，阻止 command/timer/application/async executor 回流 `app.rs`
  - `app.rs` 从约 3182 行降到约 2685 行，主事件执行器约 501 行
- 同步推进 macOS event adapter：
  - 新增 `MacosMainEventModel`，把共享产品事件映射为 LAN/VV/page/settings refresh、image paste、OCR/translation 和 thumbnail cache 等 AppKit-facing semantic routes
  - 新增契约测试，证明 macOS scaffold 直接消费 `ApplicationEvent` / `MainAsyncEvent`，不依赖 Windows message ids 或 `wnd_proc`
- Linux 路线同步要求 GLib/GTK event loop 消费相同 shared/product event enums，只替换平台回调和 side effects

## step208
- 将 Windows process/main-window entry adapter 从 `app.rs` 拆出：
  - 新增 `src/app/main_entry.rs`，集中 `run`、`wnd_proc`、`on_create`、search/control command 入口、search debounce、scroll cancel、settings hotkey feedback 和 VV selection glue
  - `app.rs` 通过 `pub(crate) use self::main_entry::run` 暴露 Windows 入口，自身不再承载窗口过程和 startup integration wiring
  - 新增 source guard，阻止 Windows process entry、main wndproc 和 create/control glue 回流 `app.rs`
  - `app.rs` 从约 2685 行降到约 2342 行，main entry adapter 约 352 行
- 同步推进 macOS native startup：
  - 新增 `MacosStartupPlan`，从共享 `MainUiLayout` 生成 `NativeMainWindowRequest` 和 `LifecycleEvent::Mount`
  - macOS `run()` 改为执行 startup plan、调用 `MacosMainWindowHost::create_main_windows` 并应用 main window appearance，不再只是打印 contract summary
  - 新增契约测试验证 startup request 的标题、尺寸、可见性、mount lifecycle 和 host 创建路径
- Linux 后续可采用同一 startup-plan 形状，只把 host 替换为 GTK/libadwaita application/window entry

## step209
- 继续按“多轮迁移后统一验证”推进 Windows 旧 UI 反向清理：
  - 剪贴板 source/foreground identity、自身进程过滤、RGBA 归一化和 guarded legacy bitmap decode 从 `app.rs` 归入 `src/app/main_clipboard_capture.rs`
  - VV target process name 查询归入 `src/app/vv_hook.rs`
  - quick-search URL 编码、模板展开和 shell launch 归入 `src/app/main_row_commands.rs`
  - source guards 扩展为阻止上述 Windows/product glue 回流 `app.rs`
  - `app.rs` 从约 2342 行降到约 2149 行
- 同步推进 macOS 应用状态根：
  - 新增 `MacosApplicationModel`，统一拥有 `LifecycleState`、`CommandQueue`、main/settings window models 和 `MacosMainEventModel`
  - macOS `run()` 通过 application model 完成 mount/activate，再创建 native main windows
  - 新增契约测试覆盖 lifecycle 顺序、command queue、settings model ownership 以及 application/async event routing
- Linux 后续 application state root 可复用同一 lifecycle/command/event 组织方式，只替换 GTK application/window host

## step210
- 将 Windows 同步执行从 `app.rs` 批量拆出：
  - 新增 `src/app/main_cloud_sync.rs`，集中 cloud sync 排队、配置校验、后台任务启动、完成队列应用、数据/设置刷新和错误反馈
  - 新增 `src/app/main_lan_sync.rs`，集中 LAN envelope 编解码、最新记录发布、接收去重、图片/文件落盘、远端 clipboard mirror 和清理
  - `main_events.rs` / `settings_actions.rs` 继续通过语义事件调用两个 adapter，`app.rs` 只保留状态与通用数据能力
  - 新增 source guard，阻止 cloud/LAN transport 与 completion glue 回流 `app.rs`
  - `app.rs` 从约 2149 行降到约 1574 行
- 同步推进 macOS async state：
  - 新增 `MacosBackgroundTaskState`，记录 cloud sync 排他状态、LAN refresh generation、paste/text completion 计数和 thumbnail ids
  - `MacosApplicationModel` 在 shared `ApplicationEvent` / `MainAsyncEvent` 路由时同步更新 background task state
  - 契约测试覆盖重复 cloud start 拒绝、cloud completion、LAN refresh、thumbnail 与 text operation 状态
- Linux 后续可在同一 application/background-task 组织上接 GLib futures、GIO network 和 GDK clipboard

## step211
- 将 Windows 主运行状态定义从 `app.rs` 归入 `src/app/state.rs`：
  - `WindowRole`、`Icons`、`AppState` 及其 `Deref` / `DerefMut` 实现迁入状态模块
  - cache、VV popup item 等原私有字段仅开放到 `app` sibling adapter 范围，不扩大为公共 API
  - `app.rs` 保留产品行为方法，但不再定义原生窗口状态容器
  - 新增 source guard，阻止主状态定义回流 `app.rs`
- 同步推进 macOS window session：
  - 新增 `MacosWindowSessionState`，持有 main/quick handles、settings handle、可见性和 render/presentation generations
  - macOS `run()` 在 native window 创建后登记 handles 并记录初始 render
  - application model 测试覆盖 main/settings session 的创建、呈现和隐藏状态
- 设置窗口状态仍与大量 settings host adapter 紧耦合，下一批将连同 host 访问边界一起收口，避免单纯把一百多个字段改成宽泛可见

## step212
- 将 Windows 剪贴 payload 数据能力从 `app.rs` 归入 `src/app/data.rs`：
  - 文本/文件预览、文本/图片/文件签名、去重签名、二维码图片生成、图片落盘/读取和缩略图缓存入口都离开应用根模块
  - `app.rs` 继续调用这些产品数据能力，但不再定义可被 macOS/Linux 复用的数据语义
  - 新增 source guard，阻止 payload helper 回流 `app.rs`
  - 这一步把下一阶段 AI/LLM/skills 能力需要理解的剪贴条目语义收口到数据层，而不是 Windows 窗口过程
- 同步推进 macOS payload session：
  - 新增 `MacosClipPayloadDataState`，记录当前原生 UI 最近消费的 `ClipItem` 类型、预览文本、预览 generation 和缩略图缓存 id
  - `MacosApplicationModel` 现在同时拥有 lifecycle、commands、windows、background tasks 和 clip payload session
  - macOS 测试覆盖文本/图片条目进入 payload session，以及 shared thumbnail async route 更新缓存 id
- Linux 后续可复用同一 `ClipItem` / payload session 形状，先接 GTK/GDK 列表模型和 thumbnail cache，再替换平台剪贴板读取实现

## step213
- 将 Windows 主运行状态行为从 `app.rs` 迁入 `src/app/state_runtime.rs`：
  - transient clipboard duplicate guard、LAN message 去重、programmatic clipboard signature guard、payload cache、选择解析、滚动锚点、去重插入、主列表布局 helper 等 `AppState` 方法离开应用根模块
  - 这些方法只以 `pub(super)` 暴露给 `app` adapter 层，不扩大为 crate 公共 API
  - `app.rs` 进一步收敛为模块聚合、常量、settings window state 和少量平台 helper
  - 新增 source guard，阻止 `impl AppState` 和主状态行为方法回流 `app.rs`
- 同步推进 macOS 主列表 session：
  - 新增 `MacosMainListSessionState`，记录可见 `ClipItem` ids、选中 ids、滚动锚点和 list/selection/scroll generations
  - `MacosApplicationModel` 现在拥有 window session、payload session 和 list session，未来 AppKit 列表视图可以直接消费 Rust 状态而不是复制 Windows `AppState`
- Linux 后续可复用同一 list session 形状接 GTK/GDK list model，只替换原生 selection、scroll 和 repaint bridge

## step214
- 将 Windows settings window 状态定义从 `app.rs` 迁入 `src/app/settings_state.rs`：
  - `SettingsWndState` 不再由应用根模块定义，settings window/control/action/input/dropdown adapters 通过同一状态模块访问
  - 字段暂时以 `pub(super)` 限定在 `app` adapter 层，保持行为稳定；后续再用更小的 accessor/section state 逐步收口直接字段访问
  - 新增 source guard，阻止 settings 状态定义回流 `app.rs`
- 同步推进 macOS settings session：
  - 新增 `MacosSettingsSessionState`，记录当前设置页、dirty 状态、draft/applied/presentation generations
  - `MacosApplicationModel` 通过 `select_settings_page` 同步 settings window model 和 settings session
  - application model 测试覆盖页面切换、草稿变更、保存应用和 presentation generation
- Linux 后续可复用同一 settings session 形状接 GTK/libadwaita preferences window，只替换原生控件和窗口 host

## step215
- 将 `app.rs` 剩余 helper 继续拆出：
  - 新增 `src/app/platform_helpers.rs`，集中后台文本写入剪贴板、native dialog message/confirm helper
  - 新增 `src/app/main_view_helpers.rs`，集中主窗口主题色映射、矩形命中、图片预览可见性、scroll-to-top 可见性、标题按钮可见性、空状态和 hover item helper
  - `reload_state_from_db_persisting` 归入 `src/app/state_runtime.rs`
  - `app.rs` 不再定义自由函数，进一步收敛为模块声明、导出、导入和平台常量
  - 新增 source guard，阻止剩余 helper 回流 `app.rs`
- 同步推进 macOS main visual session：
  - 新增 `MacosMainVisualSessionState`，记录标题按钮可见性、空状态、图片预览启用状态和 visual generation
  - `MacosApplicationModel` 新增 `update_main_visual_state`，未来 AppKit 主窗口可消费 Rust 视觉状态而不复制 Windows view helper
- Linux 后续可复用同一 main visual session 形状接 GTK/libadwaita 主窗口标题按钮、空状态和 preview policy

## step216
- 将 Windows app adapter 共享导入和平台常量从 `app.rs` 迁入 `src/app/prelude.rs`：
  - `app.rs` 现在只保留模块声明、必要 re-export 和测试模块入口，不再承载 `use crate::*`、Win32 imports、timer ids、窗口 class name 或 `AppResult`
  - 所有 `src/app/*.rs` 顶层从 `use super::*` 改为 `use super::prelude::*`，显式消费 app adapter prelude，而不是把应用根模块当隐藏全局导入桶
  - 新增 source guard，阻止共享 imports / constants 回流 `app.rs`
- 同步推进 macOS adapter prelude 边界：
  - 新增 `MacosAdapterPreludeState`，记录 shared contract roots 与 native adapter roots
  - `MacosApplicationModel` 新增 `record_adapter_prelude_boundary`，未来 AppKit/SwiftUI host 可拥有自己的 prelude/host 入口，而不是复制 Windows `app/prelude.rs`
- Linux 后续可复用同一边界思路：GTK/libadwaita host 也应建立自己的 adapter prelude，只引用 `app_core`/settings model 契约和 Linux 原生 host，而不是移植 Windows prelude

## step217
- 将 Windows 平台 ID / 常量从 `src/app/prelude.rs` 再拆入 `src/app/constants.rs`：
  - timer ids、main/settings timer mapping、窗口 class name、hotkey id、剪贴板格式 id、edge auto-hide 参数、分页加载参数和 `AppResult` 都离开 prelude
  - `src/app/prelude.rs` 只负责 app adapter 导入聚合，并显式 re-export `constants::*`
  - `src/app.rs` 通过 `self::constants::TRAY_UID` 暴露托盘 id，继续避免把平台常量写回应用根模块
  - 更新 source guard，要求 constants 归属到 `app/constants.rs`，prelude 不再直接定义 timer ids 或 `AppResult`
- 同步推进 macOS native id session：
  - 新增 `MacosNativeIdSessionState`，记录 native window identifiers、timer identifiers 和 status item identity
  - `MacosApplicationModel` 新增 `record_native_id_session`，未来 AppKit/SwiftUI host 可拥有自己的平台 ID/session 层，而不是复制 Windows `app/constants.rs`
- Linux 后续可沿用同一形状：GTK/libadwaita host 应在 Linux adapter 内维护窗口 id、GLib timer/source id 和 status/app-indicator identity

## step218
- 将 Windows 主搜索框 native host 从大 `src/app/hosts.rs` 拆入 `src/app/main_search_host.rs`：
  - `WindowsMainSearchControlHost`、native EDIT 创建、搜索框字体资源创建/释放、bounds/visible/text/focus 操作都离开 `hosts.rs`
  - `src/app/prelude.rs` re-export `main_search_host::*`，现有 main entry/search/paste 路径继续消费同一个 shared `NativeMainSearchControlHost` 契约
  - 更新 source guard，要求搜索框 host 归属到 `main_search_host.rs`，不再回流到大 `hosts.rs`
- 同步推进 macOS main search session：
  - 新增 `MacosMainSearchSessionState`，记录 native search handle、visible、text、style resource 和 generation
  - `MacosApplicationModel` 新增 `record_main_search_session`，未来 AppKit `NSSearchField` / SwiftUI search field 可持久化 Rust session，而不是复制 Windows search host 内部实现
- Linux 后续可复用同一 search session 形状接 GTK `SearchEntry` / libadwaita search bar，仅替换 native control host

## step219
- 将 Windows transient floating-window native host 从大 `src/app/hosts.rs` 拆入 `src/app/transient_window_host.rs`：
  - `WindowsTransientWindowHost`、no-activate class 注册、popup 创建、bounds presentation、hide 和 destroy 操作都离开 `hosts.rs`
  - `src/app/prelude.rs` re-export `transient_window_host::*`，`src/app/vv_popup.rs` 继续消费 shared `NativeTransientWindowHost` 契约
  - 更新 source guard，要求 transient host 归属到 `transient_window_host.rs`，不再回流到大 `hosts.rs`
- 同步推进 macOS transient window session：
  - 新增 `MacosTransientWindowSessionState`，记录 owner、native transient handle、bounds、visible 和 generation
  - `MacosApplicationModel` 新增 `record_transient_window_session`，未来 AppKit `NSPanel` / popover 可持久化 Rust session，而不是复制 Windows popup host 内部实现
- Linux 后续可复用同一 transient session 形状接 GTK/libadwaita popup、popover 或 layer-shell 浮窗，仅替换 native floating-window host

## step220
- 将 Windows main/quick native window host 从大 `src/app/hosts.rs` 拆入 `src/app/main_window_host.rs`：
  - `WindowsMainWindowHost`、Win32 class registration、main/quick window creation、appearance、icon、bounds、activation、pointer capture、native drag、repaint、DPI/client/window bounds query 和 destroy 操作都离开 `hosts.rs`
  - `src/app/prelude.rs` re-export `main_window_host::*`，现有 `main_entry.rs` / `main_window.rs` / `main_search.rs` / paste 路径继续消费同一个 shared `NativeMainWindowHost` 契约
  - 更新 source guard，要求 main window host 归属到 `main_window_host.rs`，不再回流到大 `hosts.rs`
- 同步推进 macOS main window host session：
  - `MacosWindowSessionState` 新增 main host appearance、bounds、activation-policy 和 generation 记录
  - `MacosApplicationModel` 新增 `record_main_window_host_session`，未来 AppKit `NSWindow` 可持久化 Rust 主窗口 host 状态，而不是复制 Windows host 内部实现
- Linux 后续可复用同一 main window host session 形状接 GTK/libadwaita application/window、Wayland/X11 activation 和 native drag/repaint/bounds 查询，仅替换 native main-window host

## step221
- 将 Windows edge auto-hide 行为从大 `src/app/hosts.rs` 拆入 `src/app/main_edge_auto_hide.rs`：
  - screen-edge detection、docked bounds、hidden-position calculation、animation tick、hot-zone restore、edge move reconciliation 和 edge state reset 都离开 `hosts.rs`
  - `src/app/prelude.rs` re-export `main_edge_auto_hide::*`，现有 main window/timer/settings 路径继续消费同一组 edge 行为入口
  - 新增 source guard，要求 edge auto-hide 主体归属到 `main_edge_auto_hide.rs`，不再回流到大 `hosts.rs`
- 同步推进 macOS edge auto-hide session：
  - `MacosWindowSessionState` 新增 edge auto-hide enabled、hidden、bounds 和 generation 记录
  - `MacosApplicationModel` 新增 `record_edge_auto_hide_session`，未来 AppKit edge docking 可持久化 Rust session，而不是复制 Windows edge animation/monitor 内部实现
- Linux 后续可复用同一 edge session 形状接 GTK/libadwaita 或 layer-shell/Wayland/X11 edge docking，仅替换 native monitor、activation 和 animation 服务

## step222
- 将 Windows paste target discovery 从大 `src/app/hosts.rs` 拆入 `src/app/main_paste_target_discovery.rs`：
  - skip-class parsing、目标窗口 class skip 判断、top-level window 枚举、viability checks、title ignore 和 next-target selection 都离开 `hosts.rs`
  - `src/app/prelude.rs` re-export `main_paste_target_discovery::*`，现有 `main_paste.rs` 和 `settings_actions.rs` 继续消费同一组 paste target discovery 入口
  - 新增 source guard，要求 paste target discovery 主体归属到 `main_paste_target_discovery.rs`，不再回流到大 `hosts.rs`
- 同步推进 macOS paste target discovery session：
  - 新增 `MacosPasteTargetDiscoverySessionState`，记录 skip-class names、last candidate 和 generation
  - `MacosApplicationModel` 新增 `record_paste_target_discovery_session`，未来 AppKit/Accessibility paste target discovery 可持久化 Rust session，而不是复制 Windows top-level-window 枚举内部实现
- Linux 后续可复用同一 discovery session 形状接 GTK/libadwaita、Wayland/X11 focused-window discovery 或 portal/AT-SPI 查询，仅替换 native window enumeration

## step223
- 将 Windows low-level input / outside-hide 调度从大 `src/app/hosts.rs` 拆入 `src/app/main_low_level_input.rs`：
  - protected-scope hit testing、outside-hide timer、edge-auto-hide timer scheduling、Quick Escape low-level keyboard hook 和 outside-hide tick 都离开 `hosts.rs`
  - `src/app/prelude.rs` re-export `main_low_level_input::*`，`main_events.rs` / `main_window.rs` / edge auto-hide 路径继续消费同一组低层输入入口
  - 新增 source guard，要求 low-level input 主体归属到 `main_low_level_input.rs`，不再回流到大 `hosts.rs`
- 同步推进 macOS low-level input session：
  - 新增 `MacosLowLevelInputSessionState` 和 `MacosPointerScope`，记录 outside-hide timer、edge-auto-hide timer、Quick Escape event monitor 和最后一次 protected pointer scope
  - `MacosApplicationModel` 新增 `record_low_level_input_session`，未来 AppKit event monitor / run-loop timer 可持久化 Rust session，而不是复制 Windows low-level hook 内部实现
- Linux 后续可复用同一 low-level input session 形状接 GTK/GDK event controller、GLib source timer、Wayland/X11 shortcut bridge 或 portal 能力，仅替换 native event/timer 服务

## step224
- 将 Windows main hover preview / mouse-leave 行为从大 `src/app/hosts.rs` 拆入 `src/app/main_hover_preview.rs`：
  - hover preview blocked-hit checks、preview refresh、mouse hover dispatch、mouse-leave cleanup 和 mouse-leave tracking rearm 都离开 `hosts.rs`
  - `src/app/prelude.rs` re-export `main_hover_preview::*`，`main_events.rs` 和 edge auto-hide 路径继续消费同一组 hover/leave 入口
  - 新增 source guard，要求 main hover preview 主体归属到 `main_hover_preview.rs`，不再回流到大 `hosts.rs`
- 同步推进 macOS hover preview session：
  - 新增 `MacosHoverPreviewSessionState`，记录 hover preview visibility、hovered item id、mouse-leave tracking active 和 generation
  - `MacosApplicationModel` 新增 `record_hover_preview_session`，未来 AppKit tracking area / popover 可持久化 Rust session，而不是复制 Windows mouse-hover/message 内部实现
- Linux 后续可复用同一 hover preview session 形状接 GTK/GDK motion/leave controller、popover 或 tooltip surface，仅替换 native pointer tracking 服务

## step225
- 将 Windows startup integration recovery 从大 `src/app/hosts.rs` 拆入 `src/app/main_startup_integrations.rs`：
  - TaskbarCreated message id、tray icon resync、hotkey/plain-paste hotkey retry、VV hook retry、clipboard listener retry、startup recovery arming 和 update-state notification 都离开 `hosts.rs`
  - `src/app/prelude.rs` re-export `main_startup_integrations::*`，`main_events.rs` / `main_entry.rs` / settings apply 路径继续消费同一组启动集成入口
  - 新增 source guard，要求 startup integrations 主体归属到 `main_startup_integrations.rs`，不再回流到大 `hosts.rs`
- 同步推进 macOS startup integration session：
  - 新增 `MacosStartupIntegrationSessionState`，记录 status item、hotkeys、clipboard monitor、VV monitor registration 和 recovery ticks
  - `MacosApplicationModel` 新增 `record_startup_integrations_session`，未来 `NSApplicationDelegate` 可持久化启动恢复状态，而不是复制 Windows tray/hotkey/listener retry 内部实现
- Linux 后续可复用同一 startup integration session 形状接 status/app-indicator、global-shortcut bridge、GDK/clipboard monitor 和 GLib retry source，仅替换 native registration 服务

## step226
- 将 Windows main/quick window refresh 从大 `src/app/hosts.rs` 拆入 `src/app/main_window_refresh.rs`：
  - loaded settings application、settings-window refresh、window state refresh、peer-window sync 和 refresh-for-show 都离开 `hosts.rs`
  - `src/app/prelude.rs` re-export `main_window_refresh::*`，cloud sync、row commands、state runtime 和 show path 继续消费同一组刷新入口
  - 新增 source guard，要求 main window refresh 主体归属到 `main_window_refresh.rs`，不再回流到大 `hosts.rs`
- 同步推进 macOS window refresh session：
  - 新增 `MacosWindowRefreshSessionState`，记录 settings reload、database reload、settings-window refresh、peer sync 和 last peer source
  - `MacosApplicationModel` 新增 `record_window_refresh_session`，未来 AppKit main/quick window 可持久化 Rust 刷新会话，而不是复制 Windows refresh helper 内部实现
- Linux 后续可复用同一 window refresh session 形状接 GTK/libadwaita main/quick windows、settings window refresh 和 clipboard-history database reload，仅替换 native window/repaint 服务

## step227
- 将 Windows main/quick window registry 从大 `src/app/hosts.rs` 拆入 `src/app/main_window_registry.rs`：
  - main/quick host registration、host handle iteration、app-window checks、state pointer lookup、cross-host clipboard ignore/skip guards 都离开 `hosts.rs`
  - `src/app/prelude.rs` re-export `main_window_registry::*`，各 Windows main/settings/paste/event 路径继续消费同一组 registry 入口
  - 新增 source guard，要求 main window registry 主体归属到 `main_window_registry.rs`，不再回流到大 `hosts.rs`
- 同步推进 macOS window registry session：
  - 新增 `MacosWindowRegistrySessionState`，记录 main/quick native handles、clipboard ignore generation 和 skip-next clipboard generation
  - `MacosApplicationModel` 新增 `record_window_registry_session`，未来 AppKit window registry 可持久化 Rust 侧窗口注册状态，而不是复制 Windows HWND registry 内部实现
- Linux 后续可复用同一 window registry session 形状接 GTK/libadwaita window registry、clipboard guard 和 app-window checks，仅替换 native handle 类型

## step228
- 将 Windows main hover clear / noactivate hit-test 残留并入 `src/app/main_hover_preview.rs`：
  - `clear_main_hover_state` 和 `main_window_should_stay_noactivate` 离开 `hosts.rs`，与 hover preview refresh、mouse hover/leave handling 归为同一主窗口 hover 行为模块
  - 新增 source guard，要求 hover clear 和 noactivate row hit-test 不再回流到大 `hosts.rs`
- 同步推进 macOS hover clear session：
  - 新增 `MacosHoverClearSessionState`，记录 preserved scrollbar hover、cleared pointer-down state、noactivate hit item 和 generation
  - `MacosApplicationModel` 新增 `record_hover_clear_session`，未来 AppKit row hit-testing / tracking cleanup 可持久化 Rust 行为状态，而不是复制 Windows hover invalidation helper
- Linux 后续可复用同一 hover clear session 形状接 GTK/GDK pointer leave、row hit-test 和 popover cleanup，仅替换 native invalidation 服务

## step229
- 将 Windows plugin settings sections 从大 `src/app/hosts.rs` 拆入 `src/app/settings_plugin_sections.rs`：
  - plugin/provider dynamic cards、插件页 row layout、provider 字段移动和 plugin page relayout 都离开 `hosts.rs`
  - `src/app/prelude.rs` re-export `settings_plugin_sections::*`，`settings_sync_page_state` 继续消费同一组插件设置页入口
  - 新增 source guard，要求插件设置页 section 逻辑不再回流到大 `hosts.rs`
- 同步推进 macOS plugin settings section session：
  - 新增 `MacosSettingsPluginSectionSessionState`，记录 visible provider sections、enabled feature count 和 generation
  - `MacosApplicationModel` 新增 `record_settings_plugin_sections_session`，未来 AppKit preferences 插件页可持久化 Rust section 状态，而不是复制 Windows 控件移动逻辑
- Linux 后续可复用同一 plugin section session 形状接 GTK/libadwaita preferences groups，仅替换 native control layout 服务

## step230
- 将 Windows multi-sync settings sections 从大 `src/app/hosts.rs` 拆入 `src/app/settings_multi_sync_sections.rs`：
  - WebDAV/LAN dynamic section layout、cloud page handle reset、multi-sync card refresh 和 cloud page rebuild 都离开 `hosts.rs`
  - `src/app/prelude.rs` re-export `settings_multi_sync_sections::*`，`settings_create_cloud_page` 和 cloud mode 切换继续消费同一组多同步设置入口
  - 新增 source guard，要求多同步设置页 section/rebuild 逻辑不再回流到大 `hosts.rs`
- 同步推进 macOS multi-sync settings section session：
  - 新增 `MacosSettingsMultiSyncSectionSessionState`，记录 selected mode、visible section count、rebuild generation 和 generation
  - `MacosApplicationModel` 新增 `record_settings_multi_sync_sections_session`，未来 AppKit WebDAV/LAN 设置页可持久化 Rust 动态 section 状态，而不是复制 Windows cloud page rebuild
- Linux 后续可复用同一 multi-sync section session 形状接 GTK/libadwaita preferences pages，仅替换 native page/control rebuild 服务

## step231
- 将 Windows group settings sections 从大 `src/app/hosts.rs` 拆入 `src/app/settings_group_sections.rs`：
  - group cache selection、VV source/group display、group overview、group list refresh、选中项读取、排序移动和 group page creation 都离开 `hosts.rs`
  - `src/app/prelude.rs` re-export `settings_group_sections::*`，settings apply/collect、group action 和 owner-draw 路径继续消费同一组分组设置入口
  - 新增 source guard，要求分组设置页 domain 不再回流到大 `hosts.rs`
- 同步推进 macOS group settings section session：
  - 新增 `MacosSettingsGroupSectionSessionState`，记录 VV source tab、group view tab、selected group id、record group count、phrase group count 和 generation
  - `MacosApplicationModel` 新增 `record_settings_group_sections_session`，未来 AppKit preferences 分组页可持久化 Rust section 状态，而不是复制 Windows listbox/order helper
- Linux 后续可复用同一 group section session 形状接 GTK/libadwaita list/segmented controls，仅替换 native list presentation 和 selection 服务

## step232
- 将 Windows About settings page 从大 `src/app/hosts.rs` 拆入 `src/app/settings_about_page.rs`：
  - About page 的版本、ZSUI 布局说明、开源地址、更新状态和数据目录控件创建离开 `hosts.rs`
  - `src/app/prelude.rs` re-export `settings_about_page::*`，settings page ensure 继续通过同一入口创建 About 页
  - 新增 source guard，要求 About page creation 不再回流到大 `hosts.rs`
- 同步推进 macOS About page session：
  - 新增 `MacosSettingsAboutPageSessionState`，记录 source available、update available、data dir 和 generation
  - `MacosApplicationModel` 新增 `record_settings_about_page_session`，未来 AppKit About/preferences 页可持久化 Rust metadata 状态，而不是复制 Windows label/button 创建
- Linux 后续可复用同一 About page session 形状接 GTK/libadwaita about/preferences page，仅替换 native label/button 服务

## step233
- 将 Windows Cloud/LAN settings page 从大 `src/app/hosts.rs` 拆入 `src/app/settings_cloud_page.rs`：
  - LAN pending/discovered list refresh、LAN selected device/pair 解析和 Cloud/WebDAV/LAN page creation 都离开 `hosts.rs`
  - `src/app/prelude.rs` re-export `settings_cloud_page::*`，settings sync/apply/action 路径继续消费同一组 Cloud/LAN 页面入口
  - 新增 source guard，要求 Cloud/LAN settings page 逻辑不再回流到大 `hosts.rs`
- 同步推进 macOS Cloud/LAN settings session：
  - 新增 `MacosSettingsCloudPageSessionState`，记录 selected mode、pending pair count、discovered device count、selected LAN row 和 generation
  - `MacosApplicationModel` 新增 `record_settings_cloud_page_session`，未来 AppKit WebDAV/LAN 设置页可持久化 Rust list/page 状态，而不是复制 Windows listbox/page creation
- Linux 后续可复用同一 Cloud/LAN page session 形状接 GTK/libadwaita list/table controls，仅替换 native list presentation 和 selection 服务

## step234
- 将 Windows settings owner-draw 从大 `src/app/hosts.rs` 拆入 `src/app/settings_owner_draw.rs`：
  - settings button hover、LAN QR payload/cache/render、QR owner-draw item、button/toggle owner-draw 和 QR cache 测试都离开 `hosts.rs`
  - `src/app/prelude.rs` re-export `settings_owner_draw::*`，Windows settings window 继续通过同一入口处理 owner-draw 控件
  - 新增 source guard，要求 settings owner-draw/QR 绘制逻辑不再回流到大 `hosts.rs`
- 同步推进 macOS settings owner-draw session：
  - 新增 `MacosSettingsOwnerDrawSessionState`，记录 hover control active、QR payload available、toggle draw count、button draw count 和 generation
  - `MacosApplicationModel` 新增 `record_settings_owner_draw_session`，未来 AppKit rendering/native controls 可持久化 Rust 绘制语义状态，而不是复制 Windows owner-draw
- Linux 后续可复用同一 owner-draw session 形状接 GTK/libadwaita native controls 或 renderer，仅替换 native drawing 服务

## step235
- 将 Windows General settings page 从大 `src/app/hosts.rs` 拆入 `src/app/settings_general_page.rs`：
  - startup、behavior、paste、max-items 和 skip-window controls 创建离开 `hosts.rs`
  - `src/app/prelude.rs` re-export `settings_general_page::*`，settings page ensure 继续通过同一入口创建 General 页
  - 新增 source guard，要求 General page creation 不再回流到大 `hosts.rs`
- 同步推进 macOS General page session：
  - 新增 `MacosSettingsGeneralPageSessionState`，记录 startup/behavior toggle counts、max-items label、skip-window enabled 和 generation
  - `MacosApplicationModel` 新增 `record_settings_general_page_session`，未来 AppKit general preferences 可持久化 Rust 设置页状态，而不是复制 Windows 控件创建
- Linux 后续可复用同一 General page session 形状接 GTK/libadwaita preferences page，仅替换 native control 服务

## step236
- 将 Windows Hotkey settings page 从大 `src/app/hosts.rs` 拆入 `src/app/settings_hotkey_page.rs`：
  - main/plain hotkey enablement、preview 和 record controls 创建离开 `hosts.rs`
  - `src/app/prelude.rs` re-export `settings_hotkey_page::*`，settings page ensure 继续通过同一入口创建 Hotkey 页
  - 新增 source guard，要求 Hotkey page creation 不再回流到大 `hosts.rs`
- 同步推进 macOS Hotkey page session：
  - 新增 `MacosSettingsHotkeyPageSessionState`，记录 main/plain hotkey previews、recording 和 generation
  - `MacosApplicationModel` 新增 `record_settings_hotkey_page_session`，未来 AppKit shortcut recorder 可持久化 Rust 快捷键页状态，而不是复制 Windows hotkey controls
- Linux 后续可复用同一 Hotkey page session 形状接 GTK/libadwaita shortcut controls，仅替换 native recorder 服务

## step237
- 将 Windows Plugin settings page 从大 `src/app/hosts.rs` 拆入 `src/app/settings_plugin_page.rs`：
  - quick search、OCR、translate 和 plugin tool controls 创建离开 `hosts.rs`
  - `src/app/prelude.rs` re-export `settings_plugin_page::*`，settings page ensure 继续通过同一入口创建 Plugin 页
  - 新增 source guard，要求 Plugin page creation 不再回流到大 `hosts.rs`
- 同步推进 macOS Plugin page session：
  - 新增 `MacosSettingsPluginPageSessionState`，记录 quick-search enabled、OCR provider、translate provider、tool-toggle count 和 generation
  - `MacosApplicationModel` 新增 `record_settings_plugin_page_session`，未来 AppKit plugin/provider preferences 可持久化 Rust 插件页状态，而不是复制 Windows provider controls
- Linux 后续可复用同一 Plugin page session 形状接 GTK/libadwaita provider/tool preferences，仅替换 native control 服务

## step238
- 将 Windows settings page builder / control factory 从大 `src/app/hosts.rs` 拆入 `src/app/settings_page_builder.rs`：
  - settings control registration、page push、`SettingsPageBuilder`、form section helper、label/edit/password/listbox wrapper 和 toggle/dropdown/button factory 都离开 `hosts.rs`
  - `src/app/prelude.rs` re-export `settings_page_builder::*`，各 settings page 模块继续复用同一 builder 入口
  - 新增 source guard，要求 page builder/control factory 不再回流到大 `hosts.rs`
- 同步推进 macOS settings page builder session：
  - 新增 `MacosSettingsPageBuilderSessionState`，记录 registered control count、ownerdraw control count、section count 和 generation
  - `MacosApplicationModel` 新增 `record_settings_page_builder_session`，未来 AppKit/SwiftUI settings page construction 可持久化 Rust builder/session 语义，而不是复制 Windows 控件创建
- Linux 后续可复用同一 page builder session 形状接 GTK/libadwaita preferences controls，仅替换 native control factory 服务

## step239
- 将 Windows settings page sync runtime 从大 `src/app/hosts.rs` 拆入 `src/app/settings_page_sync.rs`：
  - `settings_sync_page_state`、position field enablement、multi-sync mode helpers 和 LAN trusted summary text 都离开 `hosts.rs`
  - `src/app/prelude.rs` re-export `settings_page_sync::*`，settings actions/commands/window/refresh 路径继续消费同一组页面同步入口
  - 新增 source guard，要求 page sync/runtime 同步逻辑不再回流到大 `hosts.rs`
- 同步推进 macOS settings page sync session：
  - 新增 `MacosSettingsPageSyncSessionState`，记录 synced page count、enabled control count、invalidation count 和 generation
  - `MacosApplicationModel` 新增 `record_settings_page_sync_session`，未来 AppKit preferences 可持久化 Rust page synchronization 语义，而不是复制 Windows 控件 text/enabled 更新
- Linux 后续可复用同一 page sync session 形状接 GTK/libadwaita state-to-control updates，仅替换 native control update 服务

## step240
- 将 Windows settings page navigation / scroll runtime 从大 `src/app/hosts.rs` 拆入 `src/app/settings_page_navigation.rs`：
  - `settings_repos_controls`、`settings_scroll_to`、`settings_scrollbar_show`、`settings_scroll` 和 `settings_show_page` 都离开 `hosts.rs`
  - `src/app/prelude.rs` re-export `settings_page_navigation::*`，settings input/window/multi-sync relayout 路径继续消费同一组页面导航入口
  - 新增 source guard，要求 page navigation/scroll runtime 不再回流到大 `hosts.rs`
- 同步推进 macOS settings page navigation session：
  - 新增 `MacosSettingsPageNavigationSessionState`，记录 current page、scroll offset、reposition count 和 generation
  - `MacosApplicationModel` 新增 `record_settings_page_navigation_session`，未来 AppKit preferences 可持久化 Rust navigation/scroll 语义，而不是复制 Windows child-window movement
- Linux 后续可复用同一 page navigation session 形状接 GTK/libadwaita stack/scroll containers，仅替换 native scroll/control visibility 服务

## step241
- 将 Windows settings toggle state runtime 从大 `src/app/hosts.rs` 拆入 `src/app/settings_toggle_state.rs`：
  - `settings_toggle_get` 和 `settings_toggle_flip` 离开 `hosts.rs`
  - `src/app/prelude.rs` re-export `settings_toggle_state::*`，settings command 和 owner-draw 路径继续消费同一组 toggle state 入口
  - 新增 source guard，要求 toggle state runtime 不再回流到大 `hosts.rs`
- 同步推进 macOS settings toggle state session：
  - 新增 `MacosSettingsToggleStateSessionState`，记录 toggled control id、enabled toggle count 和 generation
  - `MacosApplicationModel` 新增 `record_settings_toggle_state_session`，未来 AppKit preferences 可持久化 Rust toggle 语义，而不是复制 Windows button/toggle handling
- Linux 后续可复用同一 toggle session 形状接 GTK/libadwaita switches/checkboxes，仅替换 native toggle 服务

## step242
- 将 Windows settings host helper runtime 从大 `src/app/hosts.rs` 拆入 `src/app/settings_host_helpers.rs`：
  - `settings_set_text`、`settings_show_enable`、`settings_invalidate_page_ctrls` 和 `settings_refresh_theme_resources` 离开 `hosts.rs`
  - `src/app/prelude.rs` re-export `settings_host_helpers::*`，settings sync/page/input/window 路径继续消费同一组 host helper 入口
  - 新增 source guard，要求 text/visibility/invalidation/theme helper 不再回流到大 `hosts.rs`
- 同步推进 macOS settings host helper session：
  - 新增 `MacosSettingsHostHelperSessionState`，记录 text update count、invalidation count、theme generation 和 generation
  - `MacosApplicationModel` 新增 `record_settings_host_helper_session`，未来 AppKit preferences 可持久化 Rust host-helper 语义，而不是复制 Windows text/repaint/GDI brush helper
- Linux 后续可复用同一 host-helper session 形状接 GTK/libadwaita 控件 text/visible/enabled 更新、queue_draw 和 theme resource，仅替换 native control/repaint 服务

## step243
- 将 Windows settings apply/collect 与 lazy page ensure 从大 `src/app/hosts.rs` 拆入专用模块：
  - `settings_apply_from_app` 和 `settings_collect_to_app` 迁入 `src/app/settings_app_sync.rs`
  - `settings_ensure_page` 迁入 `src/app/settings_page_ensure.rs`
  - `src/app/prelude.rs` re-export `settings_app_sync::*` 与 `settings_page_ensure::*`，settings window/action/command/refresh 路径继续消费同一组入口
  - 新增 source guard，要求 settings apply/collect/ensure 不再回流到大 `hosts.rs`
  - `src/app/hosts.rs` 现在只保留 settings model 兼容 re-export，大型 Windows settings runtime 已迁出
- 同步推进 macOS settings app-sync/page-ensure session：
  - 新增 `MacosSettingsAppSyncSessionState`，记录 apply/collect generations、saved settings count、peer sync generation 和 generation
  - 新增 `MacosSettingsPageEnsureSessionState`，记录 ensured page、built page count 和 generation
  - `MacosApplicationModel` 新增 `record_settings_app_sync_session` 与 `record_settings_page_ensure_session`，未来 AppKit preferences 可持久化 Rust 同步和懒建页语义，而不是复制 Windows save/reload/page creation side effects
- Linux 后续可复用同一 app-sync/page-ensure session 形状接 GTK/libadwaita preferences window、settings persistence 和 peer-window refresh，仅替换 native controls 和 product side-effect 服务

## step244
- 删除 Windows 旧 `src/app/hosts.rs` 聚合模块：
  - settings model 兼容 re-export 改为由 `src/app/prelude.rs` 直接导入 `lan_receive_mode_from_label`、`multi_sync_mode_display`、`multi_sync_mode_from_label` 和 `MULTI_SYNC_MODE_OPTIONS`
  - `src/app.rs` 不再声明 `hosts` 模块，`src/app/prelude.rs` 不再 `use super::hosts::*`
  - 旧 source guard 改为空源护栏，并新增 `windows_settings_hosts_module_is_retired`，要求 `hosts` 模块不再回归
- 这一步让 Windows 旧 UI 的大 host 聚合文件真正退场，后续清理重点转向各专用 adapter 的产品副作用瘦身和 `app_core`/platform host contract 的继续上移

## step245
- 将 Windows settings app-sync 的产品副作用从 `src/app/settings_app_sync.rs` 拆入 `src/app/settings_app_effects.rs`：
  - `settings_app_sync.rs` 保留 native control apply/collect 和 draft 读取，最终调用 `settings_commit_collected_app_settings`
  - `settings_app_effects.rs` 接管 settings persistence、autostart、tray icon refresh、hotkey/plain hotkey refresh、VV hook refresh、cloud/LAN refresh、database reload、edge-hide reconciliation、search UI refresh、layout 和 peer-window sync
  - 新增 source guard，要求 `save_settings`、`apply_autostart`、hotkey refresh、LAN refresh 和 peer sync 不再回流到 `settings_app_sync.rs`
- 同步推进 macOS settings app-effects session：
  - 新增 `MacosSettingsAppEffectsSessionState`，记录 persisted、integration refresh、data refresh、peer sync 和 generation
  - `MacosApplicationModel` 新增 `record_settings_app_effects_session`，未来 AppKit preferences 可把 post-save product effects 和 native control apply/collect 分开持久化
- Linux 后续可复用同一 app-effects session 形状接 GTK/libadwaita preferences save、desktop integration refresh、clipboard-history reload 和 peer-window refresh，仅替换 native/product side-effect 服务

## step246
- 将 Windows settings app-effects 继续拆成 post-save effect pipeline：
  - 新增 `src/app/settings_app_effect_state.rs`，记录 commit 前的 `SettingsAppEffectBaseline`
  - 新增 `src/app/settings_app_integration_effects.rs`，接管 autostart、tray icon、hotkey/plain hotkey、VV hook 和 startup recovery 刷新
  - 新增 `src/app/settings_app_data_effects.rs`，接管 cloud scheduling、LAN refresh、database prune/reconcile/reload
  - 新增 `src/app/settings_app_window_effects.rs`，接管 edge-hide reconciliation、low-level input refresh、search/layout、peer-window sync 和 repaint
  - `settings_app_effects.rs` 现在只负责 draft 提交、保存和调度 integration/data/window 三条 effect pipeline
  - 更新 source guard，要求各类 settings post-save side effects 归属到对应 pipeline 模块

## step247
- 将 Windows settings apply/collect 边界继续拆细：
  - `settings_apply_from_app` 迁入 `src/app/settings_app_apply.rs`，只负责把 Rust settings state 同步回 native settings controls
  - `settings_collect_to_app` 迁入 `src/app/settings_app_collect.rs`，只负责从 native controls 收集 draft，然后调用 `settings_commit_collected_app_settings`
  - `src/app/settings_app_sync.rs` 退场，`src/app.rs` 和 `src/app/prelude.rs` 改为声明/导出 apply 与 collect 两个专用模块
  - 更新 source guard，要求 `settings_host_text` 只出现在 collect 侧，post-save side effects 继续留在 app-effects pipeline
- macOS 后续的 preferences 同步仍可保留 apply/collect session 语义，但 AppKit adapter 可以分别实现 state-to-control hydration 与 control-to-draft collection
- macOS scaffold 同步重命名为 `MacosSettingsAppApplyCollectSessionState`、`record_settings_app_apply_collect_session` 和 `settings_app_apply_collect_session`，避免继续使用已经退场的 app-sync 模块名
- Linux 后续 GTK/libadwaita preferences window 可复用同一拆分：apply 接 settings model 到 widget state，collect 接 widget state 到 Rust draft，effects pipeline 单独处理产品刷新

## step248
- 将 Windows settings action executor 继续拆成三类产品动作域：
  - `src/app/settings_actions.rs` 现在只保留 `WindowsSettingsActionExecutor` 和 `SettingsActionExecutor` 分发实现
  - 新增 `src/app/settings_sync_actions.rs`，承载 WebDAV、LAN discovery/pair/copy/open setup 等同步动作
  - 新增 `src/app/settings_group_actions.rs`，承载 add/rename/delete/move/select group 等分组动作
  - 新增 `src/app/settings_platform_actions.rs`，承载 hotkey recording、paste sound、skip-window capture、OCR detect、mail merge、WPS docs、source/update、Win+V 和 Explorer restart 等平台动作
  - 新增 source guard，要求三类大 match 不再回流到 executor 文件
- macOS scaffold 的 `MacosSettingsActionExecutor` 新增 sync/group/platform 计数 getter，继续验证共享 `SettingsActionExecutor` contract，同时让 AppKit 后续按域实现 native/product side effects
- Linux 后续 GTK/libadwaita settings window 可以复用同一 action-domain 拆分：sync 域接 WebDAV/LAN，group 域接数据库分组，platform 域接 GTK/file-dialog/desktop-service 能力

## step249
- 将 Windows settings command executor 继续拆成三类命令/事件域：
  - `src/app/settings_command_queue.rs` 承载 settings `Command` 队列、保存反馈、toggle command 和 drain 执行
  - `src/app/settings_timer_tasks.rs` 承载 `SettingsTimerTask` 执行，包括 hide-scrollbar、clear-save-hint 和 DPI fit
  - `src/app/settings_control_selection.rs` 承载 dropdown selection 应用，包括 max-items、position mode、multi-sync、LAN receive mode、hotkey、OCR/translate、VV source/group 等控件选择
  - `src/app/settings_commands.rs` 退场，`src/app.rs` 和 `src/app/prelude.rs` 改为声明/导出三个专用模块
  - 更新 source guard，要求 command queue、timer 和 control selection 分别归属到对应模块，避免重新合并成一个 Windows settings command 大文件
- macOS 后续可继续用 shared command/timer 协议：RunLoop/DispatchSource 映射到 timer tasks，NSPopUpButton/NSMenu selection 映射到 control selection，而 settings Command drain 保持独立
- Linux 后续 GTK/libadwaita 可复用同一拆分：GLib timeout 接 timer tasks，ComboRow/DropDown selection 接 control selection，command queue 接窗口命令执行

## step250
- 将 Windows settings input executor 继续拆成 native event 域：
  - `src/app/settings_input.rs` 现在只保留 `dispatch_settings_ui_event`，负责从统一 `UiEvent` 分派到各专用 handler
  - 新增 `src/app/settings_pointer_input.rs`，承载 pointer move/leave/down/up/cancel、scroll drag、wheel scroll、dropdown outside-click 和 owner-draw hover
  - 新增 `src/app/settings_keyboard_input.rs`，承载 hotkey recorder 的 keydown 处理
  - 新增 `src/app/settings_window_events.rs`，承载 theme change、DPI suggested rect、DPI changed、size、system metrics、move completed 和 destroy cleanup
  - 更新 source guard，要求 pointer/keyboard/window event handler 不再回流到 dispatcher 文件
- macOS 后续可将 NSEvent pointer/keyboard 分流到对应 Rust adapter，将 NSWindow/appearance/backingScaleFactor lifecycle 映射到 window events，而 dispatcher 继续消费共享 `UiEvent`
- Linux 后续 GTK/libadwaita 可将 GDK pointer/key event、scale-factor/theme/window lifecycle 分别映射到同一拆分，减少对 Windows event module 的复制
- 同步推进 macOS app-effects session：
  - `MacosSettingsAppEffectsSessionState` 增加 window refresh generation，和 persisted/integration/data/peer sync generation 一起描述 post-save native effect pipeline
- Linux 后续可复用同一 pipeline 形状，把 desktop integration、data refresh 和 native window refresh 分别接到 GTK/libadwaita/GLib/portal 或产品服务

## step251
- 将 Windows settings window 继续拆成生命周期、布局和绘制三段：
  - `src/app/settings_window.rs` 只保留 Win32 window proc、open/focus/destroy、pointer capture、repaint forwarding 和 Cloud 页刷新 facade
  - 新增 `src/app/settings_window_layout.rs`，承载 settings content metrics、scroll layout、DPI transition、work-area fit 和 DPI compensation
  - 新增 `src/app/settings_window_paint.rs`，承载 CTLCOLOR、surface/control color role、buffered paint 和 settings window chrome/content/scrollbar 绘制入口
  - 更新 source guard，要求 layout/paint helper 不再回流到 settings window proc/lifecycle 文件
- macOS 后续可按同一形状实现：NSWindow/NSViewController lifecycle facade、AppKit layout/backing-scale policy 和 drawLayer/drawRect paint bridge 分离，而不是复制 Win32 window proc 文件
- Linux 后续 GTK/libadwaita 可复用同一拆分：ApplicationWindow lifecycle、scale/work-area layout policy 和 snapshot/drawing-area paint bridge 分离

## step252
- 将 Windows settings window 的创建和 owner-draw 路径继续从 window proc 中抽出：
  - 新增 `src/app/settings_window_create.rs`，承载 `WM_CREATE` 的 settings state 初始化、主题资源刷新、保存/关闭按钮创建、首屏 lazy page ensure、settings apply 和初始 page show
  - `src/app/settings_window.rs` 的 `WM_CREATE` 现在只解析 parent HWND 并调用 `create_settings_window_state`，不再直接持有超长 `SettingsWndState` 字面量
  - `WM_DRAWITEM` 的双缓冲绘制迁入 `src/app/settings_window_paint.rs` 的 `draw_settings_window_item`，window proc 只保留消息路由
  - 更新 source guard，要求 state creation、owner-draw paint 和 layout/paint helper 不再回流到 settings window proc/lifecycle 文件
- 同步推进 macOS settings window session：
  - 新增 `MacosSettingsWindowCreateSessionState`，记录 parent、initial page、save/close control count、built page count 和 applied generation
  - 新增 `MacosSettingsWindowLayoutSessionState`，记录 layout DPI、client/window bounds、move plan count 和 generation
  - 新增 `MacosSettingsWindowPaintSessionState`，记录 chrome/content/scrollbar paint generation、owner-draw count 和 generation
- Linux 后续 GTK/libadwaita settings window 可以复用同一 create/layout/paint session 形状，仅替换 widget creation、scale/work-area policy 和 snapshot/paint bridge

## step253
- 将 Windows settings window lifecycle/facade 从 window proc 文件中拆出：
  - 新增 `src/app/settings_window_lifecycle.rs`，承载 open existing/new window、bounds update、destroy、focus、pointer capture/release、full/regional repaint forwarding 和 Cloud settings refresh
  - `src/app/settings_window.rs` 现在只保留 Win32 message proc 和消息到 create/paint/control-color/default-proc 的路由
  - 更新 source guard，要求 open/focus/destroy/capture/repaint/cloud-refresh facade 不再回流到 settings window proc 文件
- 同步推进 macOS settings window lifecycle session：
  - 新增 `MacosSettingsWindowLifecycleSessionState`，记录 presented、bounds update、focused、destroyed、pointer capture、repaint 和 cloud refresh generations
  - `MacosApplicationModel` 新增 `record_settings_window_lifecycle_session`，让 AppKit preferences 后续能沿同一 lifecycle facade 接 NSWindow action，而不是复制 Win32 proc
- Linux 后续 GTK/libadwaita settings window 可复用同一 lifecycle session 形状，将 ApplicationWindow present/focus/close、GDK pointer capture、queue_draw 和 sync status refresh 分别落到平台 adapter

## step254
- 将 Windows settings window 的 Rust 状态默认构造从 create adapter 中抽出：
  - `SettingsWndState::new` 迁入 `src/app/settings_state.rs`，集中 parent、DPI、字体、控件句柄默认值、滚动状态、缓存、draft 和动态 section 初始值
  - `src/app/settings_window_create.rs` 现在只负责 `WM_CREATE` 周围的 native bootstrap：DPI/font 创建、主题资源刷新、保存/关闭按钮创建、metrics refresh、首屏 page ensure/apply/show
  - 更新 source guard，要求 `settings_window_create.rs` 调用 `SettingsWndState::new`，不再持有完整 `SettingsWndState { ... }` 字面量
- 同步推进 macOS settings state session：
  - 新增 `MacosSettingsWindowStateSessionState`，记录 initial page、DPI、reset control count、dynamic section count 和 generation
  - `MacosApplicationModel` 新增 `record_settings_window_state_session`，让 AppKit preferences 可复用 Rust state-default 语义而不是复制 Windows `SettingsWndState`
- Linux 后续 GTK/libadwaita settings window 可复用同一 state session 形状，把 Rust state defaults、widget creation 和 native lifecycle 分开接入

## step255
- 将 Windows settings window paint 继续拆成颜色、owner-draw 和整窗绘制三段：
  - 新增 `src/app/settings_window_colors.rs`，承载 `SettingsControlColorRole`、CTLCOLOR 背景/文字色设置和 surface 控件识别
  - 新增 `src/app/settings_window_owner_draw.rs`，承载 `WM_DRAWITEM` 的 per-control 双缓冲绘制，并复用 color adapter 的 surface 控件判断
  - `src/app/settings_window_paint.rs` 现在只保留 `paint_settings_window`，负责整窗 buffered paint、chrome/content/scrollbar 绘制和 paint DPI override
  - 更新 source guard，要求 color/owner-draw helper 不再回流到 full-window paint 文件
- 同步推进 macOS settings window color session：
  - 新增 `MacosSettingsWindowColorSessionState`，记录 surface/edit/list color role count 和 generation
  - `MacosApplicationModel` 新增 `record_settings_window_color_session`，让 AppKit/SwiftUI preferences 后续能用原生 NSColor/NSView 背景接同一 Rust color-role 边界
- Linux 后续 GTK/libadwaita settings window 可复用同一 color/owner-draw/paint 拆分，把 CSS provider/style context、snapshot/render node 和 drawing-area paint 分别接入平台 adapter

## step256
- 将 Windows settings window destroy cleanup 从 window event adapter 中拆出：
  - 新增 `src/app/settings_window_destroy.rs`，承载 `handle_settings_destroy` 的 scroll drag cancel、settings timer stop、dropdown popup cleanup、GDI font/brush release、state pointer drop 和 main state settings HWND 回收
  - `src/app/settings_window_events.rs` 现在只保留 theme changed、DPI suggested rect、DPI changed、size、system metrics 和 move-completed 等窗口事件
  - 更新 source guard，要求 destroy cleanup 不再回流到 settings window event module
- 同步推进 macOS settings window destroy session：
  - 新增 `MacosSettingsWindowDestroySessionState`，记录 timer/dropdown/resource cleanup count 和 generation
  - `MacosApplicationModel` 新增 `record_settings_window_destroy_session`，让 AppKit/SwiftUI preferences 后续将 NSWindow close/deinit 与资源清理分开接线
- Linux 后续 GTK/libadwaita settings window 可复用同一 destroy cleanup session，把 GLib source/timer cleanup、popover cleanup、CSS/resource cleanup 和 shared state release 分别接入平台 adapter

## step257
- 将 Windows settings window metrics 从 geometry/layout adapter 中拆出：
  - 新增 `src/app/settings_window_metrics.rs`，承载 settings page content height、max scroll、scroll layout、字体重建、保存/关闭按钮 bounds、built page rebuild、owner-draw control cleanup、visible control sync 和 metrics refresh repaint
  - `src/app/settings_window_layout.rs` 现在只保留 window rect、DPI transition、work-area fit 和 DPI compensation policy
  - 更新 source guard，要求 page/control metrics helper 不再回流到 geometry layout 文件
- 同步推进 macOS settings window metrics session：
  - 新增 `MacosSettingsWindowMetricsSessionState`，记录 measured content height、scroll slot count、rebuilt page count、visible control count 和 generation
  - `MacosApplicationModel` 新增 `record_settings_window_metrics_session`，让 AppKit/SwiftUI preferences 后续把 SwiftUI/AppKit view metrics 和 NSWindow frame/DPI policy 分开接线
- Linux 后续 GTK/libadwaita settings window 可复用同一 metrics/layout 拆分，把 widget allocation/content measurement 和 window scale/work-area policy 分别落到平台 adapter

## step258
- 将 Windows settings page sync 里的 Cloud/WebDAV/LAN 与 Plugin provider 同步拆出：
  - 新增 `src/app/settings_page_sync_cloud.rs`，承载 Cloud 页 transport summary、WebDAV 字段文本/启用状态、LAN 字段文本/启用状态、LAN trusted summary、LAN 列表刷新和 LAN 控件 repaint
  - 新增 `src/app/settings_page_sync_plugin.rs`，承载 Plugin 页 quick search、OCR provider、translate provider、AI/Mail Merge/WPS/QR 状态同步
  - `src/app/settings_page_sync.rs` 现在保留页面总分派、General/Hotkey/Group/About 页面同步、position fields enablement 和 multi-sync mode helper
  - 更新 source guard，要求 LAN/WebDAV 与 Plugin provider 控件刷新不再回流到 settings page 总同步文件
- 同步推进 macOS Cloud/Plugin sync session：
  - 新增 `MacosSettingsCloudSyncSessionState`，记录 transport mode、WebDAV control count、LAN control count、LAN refresh generation 和 generation
  - 新增 `MacosSettingsPluginSyncSessionState`，记录 quick-search enabled、OCR fields visible、translate enabled、tool control count 和 generation
  - `MacosApplicationModel` 新增 `record_settings_cloud_sync_session`，让 AppKit preferences 后续可把 WebDAV 表单和 LAN 列表接到 native controls，而不是复制 Windows 控件更新
  - `MacosApplicationModel` 新增 `record_settings_plugin_sync_session`，让 AppKit preferences 后续可把 plugin provider rows 接到 native controls，而不是复制 Windows 控件更新
- Linux 后续 GTK/libadwaita 可复用同一 Cloud/Plugin sync session，把 WebDAV entry sensitivity、LAN list model refresh、QR/repaint 请求和 plugin provider row sensitivity 分别落到 GTK adapter

## step259
- 将 Windows settings dropdown executor 继续拆成主分派、host lifecycle 和 plugin provider options：
  - 新增 `src/app/settings_dropdown_host.rs`，承载 dropdown control screen bounds、popup present/destroy/exists、settings control repaint helper
  - 新增 `src/app/settings_dropdown_plugin.rs`，承载 quick-search engine、OCR provider、translate provider、translate target 的 option list 与 selected-index request 构造
  - `src/app/settings_dropdown.rs` 现在保留通用 settings dropdown 分派、max/position/cloud interval/multi-sync/LAN/hotkey/paste sound/VV group 等非 plugin-provider option request
  - 更新 source guard，要求 plugin provider options 不再回流到主 dropdown executor，popup lifecycle helper 不再回流到主 executor
- 同步推进 macOS dropdown plugin session：
  - 新增 `MacosSettingsDropdownPluginSessionState`，记录 search/OCR/translate-provider/translate-target option count 和 generation
  - `MacosApplicationModel` 新增 `record_settings_dropdown_plugin_session`，让 AppKit `NSPopUpButton` / `NSMenu` 后续可复用 Rust provider option 语义
- Linux 后续 GTK/libadwaita 可复用同一 dropdown plugin session，将 provider combo row / popover menu 的 option model 接到 GTK adapter

## step260
- 将 Windows settings page builder 与 native control factory 继续拆开：
  - 新增 `src/app/settings_control_factory.rs`，承载 settings label、auto label、edit、password edit、listbox、small button、dropdown button 和 toggle row wrapper
  - `src/app/settings_page_builder.rs` 现在只保留 control registration、page scrollability resolution、section helper 和 builder method flow
  - 更新 source guard，要求 native control factory wrapper 不再回流到 page builder 或旧 host 聚合层
- 同步推进 macOS settings control factory session：
  - 新增 `MacosSettingsControlFactorySessionState`，记录 label/input/listbox/action button/toggle count 和 generation
  - `MacosApplicationModel` 新增 `record_settings_control_factory_session`，让 AppKit/SwiftUI preferences 后续按 Rust 控件族语义接 native controls，而不是复制 Windows HWND wrapper
- Linux 后续 GTK/libadwaita 可复用同一 control factory session，把 label、entry、list、button、switch/control row 分别落到 GTK adapter

## step261
- 将 Windows settings control registry 从 page builder 中拆出：
  - 新增 `src/app/settings_control_registry.rs`，承载 settings control registration、page push 和 scrollable-page resolution
  - `src/app/settings_page_builder.rs` 现在只保留 section helper、owner-draw ownership 和 builder method flow
  - 更新 source guard，要求 registry helper 不再回流到 page builder 或旧 host 聚合层
- 同步推进 macOS settings control registry session：
  - 新增 `MacosSettingsControlRegistrySessionState`，记录 registered control count、scrollable control count、page count 和 generation
  - `MacosApplicationModel` 新增 `record_settings_control_registry_session`，让 AppKit/SwiftUI preferences 后续用原生 view tree 接 Rust 控件注册语义，而不是复制 Windows HWND registry
- Linux 后续 GTK/libadwaita 可复用同一 control registry session，把 control model、scrollable rows 和 preferences page ownership 分别接到 GTK widget tree

## step262
- 将 Windows settings form actions 从 page builder 中拆出：
  - 新增 `src/app/settings_form_actions.rs`，承载 owner-draw button ownership、form action row、QR action row 和 owner-draw toggle row 组合
  - `src/app/settings_page_builder.rs` 继续收窄为 section helper、基础表单字段和基础控件创建 flow
  - 更新 source guard，要求 action/QR/owner-draw 组合不再回流到 page builder 或旧 host 聚合层
- 同步推进 macOS settings form action session：
  - 新增 `MacosSettingsFormActionSessionState`，记录 ownerdraw action count、action row count、QR action count、toggle action count 和 generation
  - `MacosApplicationModel` 新增 `record_settings_form_action_session`，让 AppKit/SwiftUI preferences 后续用原生按钮组、QR/action row 或 switch row 接 Rust action composition 语义
- Linux 后续 GTK/libadwaita 可复用同一 form action session，把 action rows、QR rows 和 switch/action ownership 分别映射到 GTK button box、row action 或 preferences switch row

## step263
- 将 Windows settings form fields 从 page builder 中拆出：
  - 新增 `src/app/settings_form_fields.rs`，承载 form label、value label、auto value label、dropdown、edit、password edit 和 button row helper
  - `src/app/settings_page_builder.rs` 现在只保留 section helper、raw label/button/dropdown/edit/listbox/toggle 创建 flow 和 control registration 入口调用
  - 更新 source guard，要求 form field row helper 不再回流到 page builder 或旧 host 聚合层
- 同步推进 macOS settings form field session：
  - 新增 `MacosSettingsFormFieldSessionState`，记录 label/value/dropdown/input/button row count 和 generation
  - `MacosApplicationModel` 新增 `record_settings_form_field_session`，让 AppKit/SwiftUI preferences 后续用原生 Form row、Picker row、TextField/SecureField row 和 Button row 接 Rust field row 语义
- Linux 后续 GTK/libadwaita 可复用同一 form field session，把 label row、value row、combo row、entry row 和 button row 映射到 GTK/libadwaita preferences row

## step264
- 将 Windows settings raw controls 从 page builder 中拆出：
  - 新增 `src/app/settings_raw_controls.rs`，承载 raw label、auto label、button、sized button、dropdown、edit、password edit、listbox 和 toggle row helper
  - `src/app/settings_page_builder.rs` 现在只保留 builder identity、control registration entry 和 section helper
  - 更新 source guard，要求 raw control helper 不再回流到 page builder 或旧 host 聚合层
- 同步推进 macOS settings raw control session：
  - 新增 `MacosSettingsRawControlSessionState`，记录 raw label/button/dropdown/input/listbox/toggle helper count 和 generation
  - `MacosApplicationModel` 新增 `record_settings_raw_control_session`，让 AppKit/SwiftUI preferences 后续在 native control host 与 form row 之间保留一层 Rust raw-control 语义
- Linux 后续 GTK/libadwaita 可复用同一 raw control session，把 label/button/combo/entry/list/switch helper 映射到 GTK widget 构造层

## step265
- 将 Windows Cloud/WebDAV/LAN settings page 继续拆成总入口与传输页构建：
  - 新增 `src/app/settings_cloud_page_webdav.rs`，承载 WebDAV fields、status label 和 WebDAV action rows
  - 新增 `src/app/settings_cloud_page_lan.rs`，承载 LAN status/fields、manual host/device list、pair/discovery actions、trusted summary、QR rows 和 docs action
  - `src/app/settings_cloud_page.rs` 现在保留 LAN list refresh/selected row helper、Cloud page mode dropdown/summary 和 WebDAV/LAN mode dispatch
  - 更新 source guard，要求 WebDAV/LAN page construction 不再回流到 Cloud page 总入口或旧 host 聚合层
- 同步推进 macOS Cloud WebDAV/LAN page session：
  - 新增 `MacosSettingsCloudWebdavPageSessionState`，记录 field count、action row count、status label count 和 generation
  - 新增 `MacosSettingsCloudLanPageSessionState`，记录 field count、action row count、device list count、QR action count、helper label count 和 generation
  - `MacosApplicationModel` 新增对应 record/getter，后续 AppKit preferences 可分别接 WebDAV Form 与 LAN pairing/list/QR 页面
- Linux 后续 GTK/libadwaita 可复用同一 WebDAV/LAN page session，把 WebDAV entry/action rows 与 LAN preferences rows、device list、QR rows 分别落到 GTK adapter

## step266
- 将 Windows Group settings page 从 group section 业务同步里拆出：
  - 新增 `src/app/settings_group_page.rs`，承载 Group page toggle、VV source/group dropdown、record/phrase tab buttons、group list 和 group action buttons
  - `src/app/settings_group_sections.rs` 现在只保留 group cache selection、VV source/group display、group overview/list refresh、selected row 和 reorder helper
  - 更新 source guard，要求 Group page construction 不再回流到 group section 或旧 host 聚合层
- 同步推进 macOS Group page session：
  - 新增 `MacosSettingsGroupPageSessionState`，记录 toggle/dropdown/tab/list/action/status control count 和 generation
  - `MacosApplicationModel` 新增对应 record/getter，后续 AppKit preferences 可用 Rust page structure 接 native segmented control、list/table 和 action buttons
- Linux 后续 GTK/libadwaita 可复用同一 Group page session，把 group switch、combo rows、segmented tabs、group list 和 action button row 映射到 native preferences/list adapter

## step267
- 将 Windows General settings page 继续拆成入口与分段构建：
  - `src/app/settings_general_page.rs` 现在只保留 General page builder、section 创建和分段 dispatch
  - 新增 `src/app/settings_general_page_startup.rs`，承载 startup toggles、max-items retention、behavior toggles 和 paste-sound controls
  - 新增 `src/app/settings_general_page_window.rs`，承载 skip-window controls、position controls 和 open-config action
  - 更新 source guard，要求 General page 的具体 control IDs 不再回流到总入口或旧 host 聚合层
- 同步推进 macOS General page section session：
  - 新增 `MacosSettingsGeneralPageSectionSessionState`，记录 startup、retention、behavior、sound、skip-window、position、action 分段 control count 和 generation
  - `MacosApplicationModel` 新增对应 record/getter，后续 AppKit preferences 可直接把 Rust section shape 映射为 native preference groups
- Linux 后续 GTK/libadwaita 可复用同一 General page section session，把 startup switch group、retention combo row、behavior switches、sound rows、skip-window row、position entry rows 和 action button row 映射到 native preferences adapter

## step268
- 将 Windows Plugin settings page 继续拆成入口与分段构建：
  - `src/app/settings_plugin_page.rs` 现在只保留 Plugin page builder、section 创建和分段 dispatch
  - 新增 `src/app/settings_plugin_page_search.rs`，承载 quick-search toggle、engine dropdown、URL template、restore preset 和 hint row
  - 新增 `src/app/settings_plugin_page_ocr_translate.rs`，承载 OCR provider/API fields/detect action 与 translate provider/app/secret/target fields
  - 新增 `src/app/settings_plugin_page_tools.rs`，承载 AI cleanup、mail merge、WPS task pane 和 QR conversion tool controls
  - 更新 source guard，要求 Plugin page 的 OCR/translate/tool control IDs 不再回流到总入口或旧 host 聚合层
- 同步推进 macOS Plugin page section session：
  - 新增 `MacosSettingsPluginPageSectionSessionState`，记录 quick-search、OCR、translate、tool-toggle、tool-action control count 和 generation
  - `MacosApplicationModel` 新增对应 record/getter，后续 AppKit preferences 可把 Rust Plugin section shape 映射为 native provider/tool groups
- Linux 后续 GTK/libadwaita 可复用同一 Plugin page section session，把 search provider row、OCR rows、translate rows、tool switches 和 action buttons 映射到 native preferences adapter

## step269
- 将 Windows Hotkey settings page 继续拆成入口与分段构建：
  - `src/app/settings_hotkey_page.rs` 现在只保留 Hotkey page builder、section 创建和分段 dispatch
  - 新增 `src/app/settings_hotkey_page_shortcuts.rs`，承载主快捷键 enable/mod/key/preview/record 与纯文本快捷键 enable/mod/key/preview controls
  - 新增 `src/app/settings_hotkey_page_system.rs`，承载 Win+V 屏蔽/恢复/重启资源管理器 action row 和快捷键说明 notes
  - 更新 source guard，要求 Hotkey page 的具体 hotkey control IDs 不再回流到总入口或旧 host 聚合层
- 同步推进 macOS Hotkey page section session：
  - 新增 `MacosSettingsHotkeyPageSectionSessionState`，记录 main shortcut、plain shortcut、system action、note label count 和 generation
  - `MacosApplicationModel` 新增对应 record/getter，后续 AppKit preferences 可把 Rust Hotkey section shape 映射为 native shortcut recorder 与 action rows
- Linux 后续 GTK/libadwaita 可复用同一 Hotkey page section session，把 shortcut recorder rows、system action buttons 和 explanatory notes 映射到 native preferences adapter

## step270
- 将 Windows About settings page 继续拆成入口与分段构建：
  - `src/app/settings_about_page.rs` 现在只保留 About page builder、section 创建、flow 初始化和分段 dispatch
  - 新增 `src/app/settings_about_page_metadata.rs`，承载 version label、summary label 和 source-link row
  - 新增 `src/app/settings_about_page_update.rs`，承载 update status label 与 update action button
  - 新增 `src/app/settings_about_page_data.rs`，承载 data-directory display label
  - 更新 source guard，要求 About page 的 source/update/data 具体控件不再回流到总入口或旧 host 聚合层
- 同步推进 macOS About page section session：
  - 新增 `MacosSettingsAboutPageSectionSessionState`，记录 metadata label、source link、update status、update action、data label count 和 generation
  - `MacosApplicationModel` 新增对应 record/getter，后续 AppKit About/preferences 可把 Rust metadata/update/data section shape 映射为 native labels 与 buttons
- Linux 后续 GTK/libadwaita 可复用同一 About page section session，把 metadata rows、source link button、update status/action 和 data-directory row 映射到 native preferences/about adapter

## step271
- 将 Windows Cloud/LAN settings page 继续拆薄：
  - 新增 `src/app/settings_cloud_page_lan_devices.rs`，承载 LAN pending/discovered list refresh、selected device 和 selected pair 解析
  - `src/app/settings_cloud_page.rs` 现在只保留 Cloud page mode dropdown/summary 和 WebDAV/LAN mode dispatch，不再直接依赖 `pending_pair_requests`/`discovered_devices`
  - `src/app.rs` 和 `src/app/prelude.rs` 接入 `settings_cloud_page_lan_devices`
- 同步推进 macOS Cloud LAN device list session：
  - 新增 `MacosSettingsCloudLanDeviceListSessionState`，记录 pending pair count、discovered device count、selected pair/device row 和 refresh generation
  - `MacosApplicationModel` 新增 `record_settings_cloud_lan_devices_session`，让 AppKit 后续能用 native table/list 承接 LAN 设备列表状态
- 架构守卫：
  - `windows_settings_cloud_page_lives_outside_hosts_rs` 现在要求 LAN list refresh/selection 留在独立模块，Cloud 入口不能重新包含 LAN discovery 数据源调用
- Linux 后续 GTK/libadwaita 可复用同一 LAN device list session，把 pending pair rows、discovered device rows 和 selected row 映射到 native list/table adapter

## step272
- 将 Windows settings page sync 的 Cloud 传输同步继续拆薄：
  - 新增 `src/app/settings_page_sync_cloud_webdav.rs`，承载 WebDAV URL/user/pass/dir/interval/status 文本与 enabled 同步
  - 新增 `src/app/settings_page_sync_cloud_lan.rs`，承载 LAN name/port/manual host/receive mode/status/trusted summary、LAN list refresh、LAN 控件 enabled 与 repaint
  - `src/app/settings_page_sync_cloud.rs` 现在只保留 Cloud transport summary、mode enabled 判定和 WebDAV/LAN sync dispatch
- 同步推进 macOS Cloud sync session：
  - 新增 `MacosSettingsCloudWebdavSyncSessionState`，记录 WebDAV sync control count、enabled、status text availability 和 generation
  - 新增 `MacosSettingsCloudLanSyncSessionState`，记录 LAN sync control count、enabled、list refreshed、invalidation count 和 generation
  - `MacosApplicationModel` 新增对应 record/getter，后续 AppKit preferences 可分别把 WebDAV form sync 与 LAN list sync 接到 native controls
- 架构守卫：
  - `windows_settings_page_sync_lives_outside_hosts_rs` 现在要求 Cloud sync 入口不再直接包含 LAN list refresh、LAN receive mode display 或 WebDAV/LAN 具体 control update
- Linux 后续 GTK/libadwaita 可复用同一 WebDAV/LAN sync session，把 WebDAV entry/status rows 与 LAN list/table invalidation 分别映射到 native adapter

## step273
- 将 Windows settings window surface 控件分类从 CTLCOLOR 处理里拆出：
  - 新增 `src/app/settings_window_surface_controls.rs`，按 General/Hotkey/Group/Cloud/Plugin/About 域分类 `is_settings_surface_control`
  - `src/app/settings_window_colors.rs` 现在只保留 `SettingsControlColorRole` 和 `settings_control_color`，不再直接承载 Windows control id 大表
  - `src/app.rs` 和 `src/app/prelude.rs` 接入 surface-control 分类模块，owner-draw 继续消费同一入口
- 同步推进 macOS settings window surface-control session：
  - 新增 `MacosSettingsWindowSurfaceControlSessionState`，记录 general/hotkey/group/cloud/plugin/about surface-control count 和 generation
  - `MacosApplicationModel` 新增对应 record/getter，后续 AppKit preferences 可把这些域映射为 NSView/NSControl 背景角色，而不是复制 Win32 owner-draw ID 表
- 架构守卫：
  - `windows_settings_window_layout_and_paint_live_in_dedicated_modules` 现在要求 surface-control 分类在独立模块中，且 color 模块不能重新包含 `IDC_SET_AUTOSTART` 这类控件表
- Linux 后续 GTK/libadwaita 可复用同一 surface-control session，把 settings 域映射为 GTK style class 或 libadwaita row background 角色

## step274
- 将 Windows settings dropdown selection 应用继续按设置域拆分：
  - `src/app/settings_control_selection.rs` 现在只保留 `handle_settings_control_selection` 入口、状态指针读取和 domain dispatch
  - 新增 `src/app/settings_control_selection_general.rs`，承载 max items、position mode、paste sound selection
  - 新增 `src/app/settings_control_selection_cloud.rs`，承载 cloud interval、multi-sync mode、LAN receive mode selection
  - 新增 `src/app/settings_control_selection_hotkey.rs`，承载 main/plain hotkey modifier/key selection 与 preview 更新
  - 新增 `src/app/settings_control_selection_plugin.rs`，承载 quick-search engine、OCR provider、translate provider/target selection
  - 新增 `src/app/settings_control_selection_group.rs`，承载 VV source/group selection
- 同步推进 macOS settings control selection session：
  - 新增 `MacosSettingsControlSelectionSessionState`，记录 general/cloud/hotkey/plugin/group selection count 和 generation
  - `MacosApplicationModel` 新增对应 record/getter，让 AppKit 后续可把 `NSPopUpButton` / `NSMenu` selection 映射到同一组 Rust domain handlers
- 架构守卫：
  - `windows_settings_command_executor_lives_outside_app_rs` 现在要求主 selection 文件不再包含具体 option model 或 group/cache 细节，只负责 domain dispatch
- Linux 后续 GTK/libadwaita 可复用同一 selection-domain session，把 ComboRow/DropDown selection 分发到 General/Cloud/Hotkey/Plugin/Group adapter

## step275
- 将 Windows settings toggle state 继续按设置域拆分：
  - `src/app/settings_toggle_state.rs` 现在只保留 `settings_toggle_get` / `settings_toggle_flip` 入口，并按 domain dispatch
  - 新增 `src/app/settings_toggle_state_general.rs`，承载 General/startup/window/behavior/sound/skip/preview 快关字段
  - 新增 `src/app/settings_toggle_state_cloud.rs`，承载 Cloud 与 LAN enable toggle
  - 新增 `src/app/settings_toggle_state_hotkey.rs`，承载 main/plain hotkey enable toggle
  - 新增 `src/app/settings_toggle_state_plugin.rs`，承载 quick-search、AI clean、mail merge、WPS taskpane、QR quick toggle
  - 新增 `src/app/settings_toggle_state_group.rs`，承载 group enable toggle
- 同步推进 macOS settings toggle domain session：
  - 新增 `MacosSettingsToggleDomainSessionState`，记录 general/cloud/hotkey/plugin/group toggle count 和 generation
  - `MacosApplicationModel` 新增对应 record/getter，让 AppKit 后续可把 `NSSwitch` / checkbox 状态接到同一组 Rust toggle-domain handlers
- 架构守卫：
  - `windows_settings_toggle_state_lives_outside_hosts_rs` 现在要求主 toggle 文件不再包含具体 settings draft 字段，只负责 domain dispatch
- Linux 后续 GTK/libadwaita 可复用同一 toggle-domain session，把 SwitchRow/CheckButton 状态分发到 General/Cloud/Hotkey/Plugin/Group adapter

## step276
- 将 Windows settings platform actions 继续按原生动作域拆分：
  - `src/app/settings_platform_actions.rs` 现在只保留 `execute_settings_platform_action` 入口和 Hotkey/General/Plugin/About/System dispatch
  - 新增 `src/app/settings_platform_actions_hotkey.rs`，承载 hotkey recording toggle 和 settings window focus
  - 新增 `src/app/settings_platform_actions_general.rs`，承载 paste sound picker 与 skipped-window class capture
  - 新增 `src/app/settings_platform_actions_plugin.rs`，承载 search preset restore、WeChat OCR runtime detect、mail merge 和 WPS taskpane docs
  - 新增 `src/app/settings_platform_actions_about.rs`，承载 source repository 与 update check/open
  - 新增 `src/app/settings_platform_actions_system.rs`，承载 Win+V clipboard history enable/disable 和 Explorer restart
- 同步推进 macOS settings platform action domain session：
  - 新增 `MacosSettingsPlatformActionDomainSessionState`，记录 hotkey/general/plugin/about/system action count 和 generation
  - `MacosApplicationModel` 新增对应 record/getter，让 AppKit 后续可按原生能力替换平台动作 side effects，而不是复制 Windows dispatch chain
- 架构守卫：
  - `windows_settings_action_domains_live_in_dedicated_modules` 现在要求主 platform action 文件不再包含具体平台副作用，只负责 domain dispatch
- Linux 后续 GTK/libadwaita 可复用同一 platform-action-domain session，把文件选择、系统动作、About/update、插件工具动作接到对应 native adapter

## step277
- 将 Windows settings owner-draw 继续按绘制语义拆分：
  - `src/app/settings_owner_draw.rs` 现在只保留 hover 判断和 `settings_draw_button_item` 分发入口
  - 新增 `src/app/settings_owner_draw_qr.rs`，承载 LAN QR payload、QR cache、QR payload 绘制和 QR item 绘制测试
  - 新增 `src/app/settings_owner_draw_link.rs`，承载 source repository link 的特殊文本绘制
  - 新增 `src/app/settings_owner_draw_roles.rs`，承载 QR/toggle/dropdown/accent/button role 分类
- 同步推进 macOS settings owner-draw domain session：
  - 新增 `MacosSettingsOwnerDrawDomainSessionState`，记录 QR/source-link/toggle/dropdown/accent/button role count 和 generation
  - `MacosApplicationModel` 新增对应 record/getter，让 AppKit 后续可把这些 owner-draw 语义映射到 native controls / attributed link / QR view
- 架构守卫：
  - `windows_settings_owner_draw_lives_outside_hosts_rs` 现在要求主 owner-draw 文件不再包含 QR render、source link 或控件 ID 分类细节
- Linux 后续 GTK/libadwaita 可复用同一 owner-draw-domain session，把 QR view、LinkButton、SwitchRow、ComboRow 和 accent/default buttons 映射到 native widget role

## step278
- 将 Windows settings dropdown request 构造继续按设置域拆分：
  - `src/app/settings_dropdown.rs` 现在只保留 config 打开 helper、关闭旧 popup 和 General/Cloud/Hotkey/Group/Plugin dispatch
  - 新增 `src/app/settings_dropdown_general.rs`，承载 max items、position mode、paste sound dropdown request
  - 新增 `src/app/settings_dropdown_cloud.rs`，承载 cloud interval、multi-sync mode、LAN receive mode dropdown request
  - 新增 `src/app/settings_dropdown_hotkey.rs`，承载 main/plain hotkey modifier/key dropdown request
  - 新增 `src/app/settings_dropdown_group.rs`，承载 VV source/group dropdown request
  - `src/app/settings_dropdown_plugin.rs` 继续承载 quick-search/OCR/translate provider option request
- 同步推进 macOS settings dropdown domain session：
  - 新增 `MacosSettingsDropdownDomainSessionState`，记录 general/cloud/hotkey/plugin/group dropdown count 和 generation
  - `MacosApplicationModel` 新增对应 record/getter，让 AppKit 后续可把 `NSPopUpButton` / `NSMenu` 请求按同一组 Rust dropdown-domain handlers 构造
- 架构守卫：
  - `windows_settings_dropdown_executor_lives_outside_app_rs` 现在要求主 dropdown 文件不再包含具体 option model、hotkey options 或 group cache 细节
- Linux 后续 GTK/libadwaita 可复用同一 dropdown-domain session，把 ComboRow/DropDown request 构造分发到 General/Cloud/Hotkey/Plugin/Group adapter

## step279
- 将 Windows settings collect 继续按设置域拆分：
  - `src/app/settings_app_collect.rs` 现在只保留 `settings_collect_to_app` 入口、AppState 指针读取、General/Hotkey/Plugin/Group/Cloud collect dispatch 和最终 commit
  - 新增 `src/app/settings_app_collect_general.rs`，承载 max items、position、paste sound、skip-window class 等 General 控件读取
  - 新增 `src/app/settings_app_collect_hotkey.rs`，承载 main/plain hotkey modifier/key 控件读取和 normalize
  - 新增 `src/app/settings_app_collect_plugin.rs`，承载 quick-search、OCR、translate provider 及模板/密钥控件读取
  - 新增 `src/app/settings_app_collect_group.rs`，承载 VV source/group selection 回写
  - 新增 `src/app/settings_app_collect_cloud.rs`，承载 Cloud/WebDAV/LAN interval、mode、endpoint、port、receive-mode 控件读取
- 同步推进 macOS settings collect domain session：
  - 新增 `MacosSettingsAppCollectDomainSessionState`，记录 general/hotkey/plugin/group/cloud collect count 和 generation
  - `MacosApplicationModel` 新增对应 record/getter，让 AppKit preferences 后续可按同一 Rust collect-domain 边界接 native controls，而不是复制 Win32 control read chain
- 架构守卫：
  - `windows_settings_apply_collect_live_in_dedicated_modules` 现在要求主 collect 文件不再直接调用 `settings_host_text`，并要求各 collect-domain 模块承载对应 option/provider/group/cloud 读取逻辑
- Linux 后续 GTK/libadwaita 可复用同一 collect-domain session，把 Entry/ComboRow/SwitchRow 数据收集分发到 General/Hotkey/Plugin/Group/Cloud adapter

## step280
- 将 Windows group settings sections 继续按分组页语义拆分：
  - `src/app/settings_group_sections.rs` 现在只保留 `settings_sync_group_page` 整页同步编排
  - 新增 `src/app/settings_group_sections_cache.rs`，承载 record/phrase group cache、VV source/current group view tab、从 app/draft 同步 tab 状态
  - 新增 `src/app/settings_group_sections_display.rs`，承载当前过滤文本、VV source/group dropdown display、group overview 和 tab repaint
  - 新增 `src/app/settings_group_sections_list.rs`，承载 group listbox refresh、selection、rename 占位和顺序移动
- 同步推进 macOS group section domain session：
  - 新增 `MacosSettingsGroupSectionDomainSessionState`，记录 cache/display/list/selection/order domain count 和 generation
  - `MacosApplicationModel` 新增对应 record/getter，让 AppKit preferences 后续可以把 `NSSegmentedControl`、`NSPopUpButton`、`NSTableView`/`NSOutlineView` 分别接到同一组 Rust group-section domain
- 架构守卫：
  - `windows_settings_group_sections_live_outside_hosts_rs` 现在要求 cache/display/list 细节分别落在独立模块，主 group sections 文件不能重新包含这些函数
- Linux 后续 GTK/libadwaita 可复用同一 group-section-domain session，把 ComboRow/DropDown、ListBox/ListView、排序动作映射到 GTK native controls

## step281
- 将 Windows plugin settings sections 继续按插件页语义拆分：
  - `src/app/settings_plugin_sections.rs` 现在只保留 `settings_relayout_plugin_page` 编排：刷新插件卡片、分发 quick-search/OCR/translate/tool section relayout、最后触发 host refresh
  - 新增 `src/app/settings_plugin_sections_controls.rs`，承载 plugin section control visibility/enabled 和 bounds movement
  - 新增 `src/app/settings_plugin_sections_layout.rs`，承载 plugin card model refresh、section layout 选择和 relayout 后 host repaint/reposition
  - 新增 `src/app/settings_plugin_sections_providers.rs`，承载 quick-search、OCR、translate provider section relayout
  - 新增 `src/app/settings_plugin_sections_tools.rs`，承载 AI clean、mail merge、WPS taskpane、QR tool section relayout
- 同步推进 macOS plugin section domain session：
  - 新增 `MacosSettingsPluginSectionDomainSessionState`，记录 controls/layout/provider/tool/host-refresh domain count 和 generation
  - `MacosApplicationModel` 新增对应 record/getter，让 AppKit preferences 后续可以把 provider cards、tool rows 和 repaint/reflow 分别映射到 native controls，而不是复制 Win32 child-window movement
- 架构守卫：
  - `windows_settings_plugin_sections_live_outside_hosts_rs` 现在要求主 plugin sections 文件不再包含 control move、layout model、provider/tool section 细节
- Linux 后续 GTK/libadwaita 可复用同一 plugin-section-domain session，把 provider cards、tool rows 和 reflow 映射为 GTK/libadwaita native preferences rows

## step282
- 将 Windows settings page navigation 继续按页面导航语义拆分：
  - `src/app/settings_page_navigation.rs` 现在只保留页面导航模块入口
  - 新增 `src/app/settings_page_navigation_controls.rs`，承载 scrollable child control reposition、dirty rect invalidation 和 deferred window movement
  - 新增 `src/app/settings_page_navigation_scroll.rs`，承载 scroll target update、scrollbar reveal timer、viewport mask/scroll strip redraw
  - 新增 `src/app/settings_page_navigation_switch.rs`，承载 settings page switch plan 执行、hotkey recording cleanup、dropdown close、page visibility 和 full redraw
- 同步推进 macOS settings page navigation domain session：
  - 新增 `MacosSettingsPageNavigationDomainSessionState`，记录 control-reposition、scroll-update、page-switch、visibility、redraw domain count 和 generation
  - `MacosApplicationModel` 新增对应 record/getter，让 AppKit preferences 后续可以把 `NSScrollView`、page controller/tab selection、native view visibility 和 redraw invalidation 分开接入 Rust navigation semantics
- 架构守卫：
  - `windows_settings_page_navigation_lives_outside_hosts_rs` 现在要求主 navigation 文件不再包含 scroll update、page switch、host visibility 或 deferred window movement 细节
- Linux 后续 GTK/libadwaita 可复用同一 page-navigation-domain session，把 `ScrolledWindow`、Stack/Page switch、widget visibility 和 redraw queue 映射到 GTK native controls

## step283
- 将 Windows settings sync actions 继续按同步动作语义拆分：
  - `src/app/settings_sync_actions.rs` 现在只保留 WebDAV/LAN action-domain dispatch
  - 新增 `src/app/settings_sync_actions_webdav.rs`，承载 WebDAV now/upload/apply/restore 动作到 `CloudSyncAction` 的映射和 queue
  - 新增 `src/app/settings_sync_actions_lan.rs`，承载 LAN discovery、pair/accept/reject、QR link copy 和 setup page open
- 同步推进 macOS settings sync action domain session：
  - 新增 `MacosSettingsSyncActionDomainSessionState`，记录 WebDAV/LAN action count 和 generation
  - `MacosApplicationModel` 新增对应 record/getter，让 AppKit preferences 后续可把 WebDAV scheduling、LAN discovery/pairing、clipboard copy 和 shell-open 分别接到 native/product adapter
- 架构守卫：
  - `windows_settings_action_domains_live_in_dedicated_modules` 现在要求 sync action 主文件不再包含 WebDAV/LAN 具体副作用，只负责 domain dispatch
- Linux 后续 GTK/libadwaita 可复用同一 sync-action-domain session，把 WebDAV jobs、LAN discovery/pairing、QR link copy/open 映射到 GLib/GIO、portal 或产品服务

## step284
- 将 Linux 从文档路线推进到代码级 ZSUI scaffold：
  - `src/main.rs` 新增 `#[cfg(any(target_os = "linux", test))] mod linux_app;` 和 Linux `main()` 入口
  - 新增 `src/linux_app.rs`，包含 `LinuxApplicationModel`、`LinuxStartupPlan`、`LinuxMainWindowHost` 和 `LinuxHostContractSummary`
  - Linux scaffold 消费 shared lifecycle、`CommandQueue`、`NativeMainWindowRequest`、`NativeMainWindowHost`、`REQUIRED_UI_HOST_SURFACES` 和 `REQUIRED_NATIVE_MAIN_WINDOW_HOST_OPERATIONS`
- 架构守卫：
  - `platform_entry_points_are_cfg_gated_for_windows_macos_and_linux` 现在要求 Windows/macOS/Linux 三个平台入口都存在，并要求 Linux scaffold 不引用 `windows_sys` 或 `app::run`
  - 新增 Linux contract tests，验证 GTK/libadwaita backend 标记、startup plan、main-window host handles、repaint request 和 command queue
- 后续 Linux 工作从“是否能接入”变成“补 GTK/libadwaita native event loop、settings/control/dropdown/dialog hosts”，与 Windows/macOS 共用同一 Rust ZSUI contract

## step285
- 继续推进 Linux ZSUI scaffold，从 main-window 扩展到 settings-window host：
  - `src/linux_app.rs` 新增 `LinuxSettingsWindowHost`，实现 `NativeSettingsWindowHost`
  - `LinuxHostContractSummary` 新增 settings-window required operation count，覆盖 `REQUIRED_NATIVE_SETTINGS_WINDOW_HOST_OPERATIONS`
  - `LinuxApplicationModel` 持有 settings window host，并暴露 `settings_window_host_mut`
- Linux settings scaffold 现在覆盖：
  - present/focus existing settings window
  - create settings window handle
  - bounds update、destroy、focus、pointer leave tracking、pointer capture/release
  - full/area repaint、layout DPI、client-to-screen、client/window bounds
- 架构守卫：
  - `platform_entry_points_are_cfg_gated_for_windows_macos_and_linux` 要求 Linux scaffold 同时实现 `NativeMainWindowHost` 和 `NativeSettingsWindowHost`
  - 新增 `linux_settings_window_host_consumes_shared_settings_contract`，验证 settings host request、created/focused existing presentation、focus、bounds 和 repaint
- 后续 Linux GTK/libadwaita 适配可以把这个记录型 host 替换为真实 `ApplicationWindow` / preferences window / widget repaint bridge，而不复制 Windows settings window procedure

## step286
- 继续推进 Linux ZSUI scaffold，从 settings-window 扩展到 settings controls 和 dropdown：
  - `src/linux_app.rs` 新增 `LinuxSettingsControlHost`，实现 `NativeSettingsControlHost`
  - `src/linux_app.rs` 新增 `LinuxSettingsDropdownHost`，实现 `NativeSettingsDropdownHost`
  - `LinuxHostContractSummary` 新增 settings-control 和 settings-dropdown required operation count
  - `LinuxApplicationModel` 持有 control/dropdown hosts，并暴露对应 mutable getter
- Linux settings scaffold 现在覆盖：
  - control create/destroy/exists、visible/enabled、bounds、hit-test、screen bounds、text read/write 和 repaint
  - dropdown present/destroy/bounds 查询
- 架构守卫：
  - `platform_entry_points_are_cfg_gated_for_windows_macos_and_linux` 要求 Linux scaffold 同时实现 main window、settings window、settings control 和 settings dropdown hosts
  - 新增 `linux_settings_control_host_consumes_shared_control_contract` 和 `linux_settings_dropdown_host_consumes_shared_dropdown_contract`
- 后续 Linux GTK/libadwaita 适配可以把 control specs 映射到 Label/Entry/Switch/Button/ComboRow，把 dropdown request 映射到 popover/menu，而不复制 Windows HWND control helper

## step287
- 继续推进 Linux ZSUI scaffold，从 settings controls 扩展到应用壳 UI：
  - `src/linux_app.rs` 新增 `LinuxStatusItemHost`，实现 `StatusItemHost`
  - `src/linux_app.rs` 新增 `LinuxPopupMenuHost`，实现 `NativePopupMenuHost`
  - `src/linux_app.rs` 新增 `LinuxMainSearchControlHost`，实现 `NativeMainSearchControlHost`
  - `LinuxHostContractSummary` 新增 status-item、popup-menu、main-search required operation count
  - `LinuxApplicationModel` 持有 status/popup/search hosts，并暴露对应 mutable getter
- 同步新增 `app_core::product_adapter` AI capability 描述：
  - `ProductAiProviderKind::{Llms, Skills, ProductAdapter}`
  - `ProductAiCapability`
  - `ProductAiInvocation`
  - 目的：后续 AI 能力可作为 LLM、skill 或产品 adapter 接入，不进入 Windows/macOS/Linux native host 层
- 架构守卫：
  - `platform_entry_points_are_cfg_gated_for_windows_macos_and_linux` 要求 Linux scaffold 覆盖 status item、popup menu、main window、main search、settings window/control/dropdown hosts
  - 新增 Linux status/popup/search contract tests
  - 新增 product adapter AI capability contract test
- 后续 Linux GTK/libadwaita 可把 status item 映射到 app indicator/status menu，把 popup menu 映射到 PopoverMenu/native menu，把 search host 映射到 SearchEntry/search bar；AI 则通过 product adapter/skills/LLMs 输出语义命令和结果

## step288
- 继续推进 Linux ZSUI scaffold，从应用壳 UI 扩展到系统服务和绘制基础 host：
  - `src/linux_app.rs` 新增 `LinuxShellOpenHost`，实现 `NativeShellOpenHost`
  - `src/linux_app.rs` 新增 `LinuxFileDialogHost`，实现 `NativeFileDialogHost`
  - `src/linux_app.rs` 新增 `LinuxTextInputDialogHost`，实现 `NativeTextInputDialogHost`
  - `src/linux_app.rs` 新增 `LinuxEditTextDialogHost`，实现 `NativeEditTextDialogHost`
  - `src/linux_app.rs` 新增 `LinuxRenderer`，实现 shared `Renderer`
  - `LinuxHostContractSummary` 新增 shell-open、file-dialog、text-input-dialog、edit-text-dialog 和 renderer required operation count
  - `LinuxApplicationModel` 持有这些基础 host，后续 GTK/libadwaita entry adapter 可直接替换 recording host 内部实现
- 架构守卫：
  - `platform_entry_points_are_cfg_gated_for_windows_macos_and_linux` 要求 Linux scaffold 覆盖 shell open、file dialog、text input dialog、edit text dialog 和 renderer host
  - 新增 `linux_system_service_hosts_consume_shared_shell_and_dialog_contracts`
  - 新增 `linux_renderer_consumes_shared_render_contract`
- 后续 Linux GTK/libadwaita 可把 shell open 映射到 GIO/portal，file dialog 映射到 `FileDialog`/portal，text/edit dialog 映射到 native dialog 或 preferences row，renderer 映射到 snapshot/drawing-area；上层继续只依赖 Rust ZSUI contract

## step289
- 继续推进 Linux ZSUI scaffold，从系统服务和绘制基础 host 扩展到 clipboard、dialog、window identity、paste target 和 text caret：
  - `src/linux_app.rs` 新增 `LinuxClipboardHost`，实现 `ClipboardHost`
  - `src/linux_app.rs` 新增 `LinuxDialogHost`，实现 `NativeDialogHost`
  - `src/linux_app.rs` 新增 `LinuxWindowIdentityHost`，实现 `NativeWindowIdentityHost`
  - `src/linux_app.rs` 新增 `LinuxPasteTargetHost`，实现 `NativePasteTargetHost`
  - `src/linux_app.rs` 新增 `LinuxTextCaretHost`，实现 `NativeTextCaretHost`
  - `LinuxHostContractSummary` 新增 clipboard、dialog、window-identity、paste-target 和 text-caret required operation count
  - `LinuxApplicationModel` 持有 dialog/window-identity/paste-target/text-caret hosts；clipboard 先用进程内 recording state 表达同一 contract
- 架构守卫：
  - `platform_entry_points_are_cfg_gated_for_windows_macos_and_linux` 要求 Linux scaffold 覆盖 clipboard、dialog、window identity、paste target 和 text caret hosts
  - 新增 `linux_clipboard_host_consumes_shared_clipboard_contract`
  - 新增 `linux_dialog_and_window_identity_hosts_consume_shared_contracts`
  - 新增 `linux_paste_target_host_consumes_shared_target_contract`
  - 新增 `linux_text_caret_host_consumes_shared_anchor_contract`
- 分层说明：
  - 主程序继续负责产品功能和数据
  - ZSUI 将产品状态翻译成平台无关 UI model、command、event 和 host request
  - Windows/macOS/Linux host 再分别把 request 翻译为 native window/control/menu/clipboard/dialog/desktop API
- 后续 Linux GTK/libadwaita 可把 clipboard 映射到 GDK clipboard，把 dialog 映射到 libadwaita dialog，把 window identity/paste target/text caret 映射到 portal、Wayland/X11 或 AT-SPI 桥接；上层仍只依赖 Rust ZSUI contract

## step290
- 继续推进 Linux ZSUI scaffold，从平台服务 host 扩展到 style、text layout、transient window、IME 和 mail merge：
  - `src/linux_app.rs` 新增 `LinuxNativeStyleResolver`，实现 `NativeStyleResolver`
  - `src/linux_app.rs` 新增 `LinuxTextLayout`，实现 `TextLayout`
  - `src/linux_app.rs` 新增 `LinuxTransientWindowHost`，实现 `NativeTransientWindowHost`
  - `src/linux_app.rs` 新增 `LinuxImeHost`，实现 `NativeImeHost`
  - `src/linux_app.rs` 新增 `LinuxMailMergeWindowHost`，实现 `NativeMailMergeWindowHost`
  - `LinuxHostContractSummary` 新增 native-style、text-layout、transient-window、IME 和 mail-merge required operation count
  - `LinuxApplicationModel` 持有 style/text-layout/transient/IME/mail-merge hosts，让 Linux 后续可直接替换为 GTK/libadwaita/Pango/GDK/portal adapter
- 架构守卫：
  - `platform_entry_points_are_cfg_gated_for_windows_macos_and_linux` 要求 Linux scaffold 覆盖 style resolver、text layout、transient window、IME 和 mail-merge hosts
  - 新增 `linux_style_and_text_layout_hosts_consume_shared_rendering_contracts`
  - 新增 `linux_transient_window_and_ime_hosts_consume_shared_contracts`
  - 新增 `linux_mail_merge_window_host_consumes_shared_open_contract`
- 后续 Linux GTK/libadwaita 可把 style resolver 映射到 CSS/style context，把 text layout 映射到 Pango，把 transient window 映射到 popover/layer surface，把 IME 映射到 GTK/GDK input-method APIs，把 mail merge 映射到产品窗口；主程序仍只负责功能，ZSUI 继续承担多平台 UI 翻译

## step291
- 继续推进 Linux ZSUI scaffold，从 style/text layout 扩展到 native control mapping 和 shared protocol summary：
  - `src/linux_app.rs` 新增 `LinuxNativeControlMapper`，实现 `NativeControlMapper`
  - `src/linux_app.rs` 新增 `LinuxNativeControlClass`，记录 GTK/libadwaita 控件角色：Label、Entry、Switch、ComboRow、Button、SuggestedActionButton
  - `LinuxHostContractSummary` 新增 native-control-mapper required operation count
  - `LinuxHostContractSummary` 新增 `shared_non_host_protocols`，与 macOS summary 对齐，明确 Command/LayoutProtocol/Component 仍属于共享 Rust 协议而非平台 host
- 架构守卫：
  - `platform_entry_points_are_cfg_gated_for_windows_macos_and_linux` 要求 Linux scaffold 覆盖 `LinuxNativeControlMapper` 和 `SHARED_NON_HOST_UI_PROTOCOLS`
  - `linux_style_and_text_layout_hosts_consume_shared_rendering_contracts` 扩展验证 settings component kind 到 GTK/libadwaita 控件角色的映射
- 后续 Linux GTK/libadwaita 可直接把 `SettingsComponentKind` 映射到实际 widget factory，而主程序继续只输出功能和 UI 语义

## step292
- 开始把 Linux 从 recording scaffold 推进到可替换 GTK/libadwaita adapter 边界：
  - 新增 `src/linux_gtk_adapter.rs`
  - 新增 `LinuxGtkHostBinding`，为当前 Linux/ZSUI host contract 命名 GTK/libadwaita adapter 入口，例如 `adw_application_window`、`gtk_search_entry`、`gtk_snapshot_renderer`、`gdk_clipboard_bridge`、`adw_preferences_window`
  - 新增 `REQUIRED_LINUX_GTK_HOST_BINDINGS`，覆盖 lifecycle、command、style/control/text、renderer、clipboard、menu、dialog、IME、window identity、paste target、main/settings host 等 25 个 adapter binding
  - 新增 `LinuxGtkWidgetRole`，把 `LinuxNativeControlClass` 翻译成 GTK/libadwaita widget role，例如 `gtk::Label`、`gtk::Entry`、`gtk::Switch`、`adw::ComboRow`、`gtk::Button.suggested-action`
  - 新增 `LinuxGtkAdapterBoundary`，从 `linux_host_contract_summary()` 生成 dependency-free adapter boundary，后续真实 GTK/libadwaita 实现可替换 recording host 内部
  - `linux_app::run()` 现在会构造 `LinuxGtkAdapterBoundary::default_from_linux_contract()`，防止 adapter 边界成为孤立文件
- 架构守卫：
  - `platform_entry_points_are_cfg_gated_for_windows_macos_and_linux` 要求 `linux_gtk_adapter` 模块存在、不能引用 `windows_sys` 或 `app::run`
  - 新增 `linux_gtk_adapter_boundary_covers_current_zsui_hosts`
  - 新增 `linux_gtk_widget_roles_map_native_control_classes`
- 后续可以在这个边界后面逐步接真实 GTK/libadwaita event loop、widget factory、GDK clipboard、Pango layout、GIO/portal 服务，而不改变主程序和 ZSUI contract

## step293
- 让 macOS 和 Linux adapter 边界对称，开始把 `macos_app.rs` 的 recording scaffold 推进到可替换 AppKit/SwiftUI adapter 边界：
  - 新增 `src/macos_appkit_adapter.rs`
  - 新增 `MacosAppKitHostBinding`，为当前 macOS/ZSUI host contract 命名 AppKit/SwiftUI adapter 入口，例如 `ns_window_pair`、`ns_search_field`、`core_graphics_renderer`、`ns_pasteboard_bridge`、`settings_window_controller`
  - 新增 `REQUIRED_MACOS_APPKIT_HOST_BINDINGS`，覆盖 lifecycle、command、main execution plan、style/control/text、renderer、clipboard、menu、dialog、IME、window identity、paste target、main/settings host 等 26 个 adapter binding
  - 新增 `MacosAppKitWidgetRole`，把 `SettingsComponentKind` 翻译成 AppKit widget role，例如 `NSTextField.label`、`NSTextField`、`NSSwitch`、`NSPopUpButton`、`NSButton.borderedProminent`
  - 新增 `MacosAppKitAdapterBoundary`，从 `MacosUiHost::contract_summary()` 生成 dependency-free adapter boundary，后续真实 AppKit/SwiftUI 实现可替换 recording host 内部
  - `macos_app::run()` 现在会构造 `MacosAppKitAdapterBoundary::default_from_macos_contract()`，防止 adapter 边界成为孤立文件
- 架构守卫：
  - `platform_entry_points_are_cfg_gated_for_windows_macos_and_linux` 要求 `macos_appkit_adapter` 模块存在、不能引用 `windows_sys` 或 `app::run`
  - 新增 `macos_appkit_adapter_boundary_covers_current_zsui_hosts`
  - 新增 `macos_appkit_widget_roles_map_settings_component_kinds`
- 后续可以在这个边界后面逐步接真实 `NSApplication` / `NSWindow` / `NSStatusItem` / `NSMenu` / `NSAlert` / `NSPasteboard` / Accessibility / SwiftUI settings controls，而不改变主程序和 ZSUI contract

## step294
- 开始补 AI 能力理解层，让后续 LLMs / skills / product adapter 可以结构化理解这套 UI/产品能力，而不是只看到平台按钮或字符串：
  - `src/app_core/product_adapter.rs` 新增 `ProductAiUiSurface`，标记能力属于 MainWindow、RowContextMenu、SettingsPluginPage 或 BackgroundTask
  - 新增 `ProductAiActionKind`，标记 clean text、translate、OCR、summarize、explain、invoke skill、configure provider 等语义动作
  - 新增 `ProductAiContextKind`，标记 user prompt、selected text/image/file path、clipboard item ids、settings profile 等输入上下文
  - 新增 `ProductAiResultKind`，标记 text、clipboard text、clipboard items、product command、settings mutation 等结果类型
  - 新增 `ProductAiCapabilityDescriptor` 和 `PRODUCT_AI_CAPABILITY_CATALOG`，把 7 个当前产品 AI 能力描述为 provider/action/surface/context/result 的结构化目录
  - `product_ai_capability_catalog()` 和 `product_ai_capability_descriptor()` 提供稳定读取入口，后续 Windows/macOS/Linux 原生 UI 可以复用同一批 AI action 描述
- 架构守卫：
  - `product_ai_capabilities_stay_in_product_adapter_layer` 现在读取真实 catalog，验证 LLM、skills、product adapter 三类 provider，验证 selected text/image/settings profile 上下文和 result kind
- 后续真实 LLM client、skill registry、prompt/tool permission 和产品数据访问仍应留在 product adapter 执行层；ZSUI/native host 只消费 capability 描述与 invocation request

## step295
- 将 AI catalog 接到 UI 语义入口，避免 catalog 只是孤立能力表：
  - `src/app_core/product_adapter.rs` 新增 `product_ai_capabilities_for_surface()`，按 MainWindow、RowContextMenu、SettingsPluginPage 等 UI surface 查询能力
  - 新增 `product_ai_capabilities_for_context()`，按 UI surface + selected text/image/file/settings profile 等上下文过滤能力
  - 新增 `product_ai_capability_for_action()`，按 UI surface + `ProductAiActionKind` 查找能力描述
  - `MainRowMenuAction` 新增 `ai_action_kind()`，将现有 `ImageOcr` 和 `TextTranslate` 行菜单动作映射到 `OcrImage` / `TranslateText`
- 架构守卫：
  - `product_ai_capabilities_stay_in_product_adapter_layer` 扩展验证 surface/context/action 查询
  - 新增 `main_row_ai_menu_actions_resolve_to_product_ai_catalog`，确认行菜单 OCR/翻译动作能解析到 `clipboard.product.ocr` 与 `clipboard.skill.translate`
- 后续 Windows/macOS/Linux 原生 UI 都可以从 row menu action 出发查同一份 AI descriptor，再把执行交给 LLM、skill registry 或 product adapter

## step296
- 将 AI catalog 接到设置插件页的 presentation 模型，让 provider 配置也能被三平台原生设置 UI 复用：
  - `src/settings_model.rs` 新增 `SettingsPluginAiCapabilityPresentation`，记录 capability id、label、provider、action 和 result
  - 新增 `SettingsPluginAiPanel`，记录 SettingsPluginPage surface、SettingsProfile context 和对应 capabilities
  - 新增 `settings_plugin_ai_panel()`，从 `product_ai_capabilities_for_context(SettingsPluginPage, SettingsProfile)` 生成设置页 AI provider 配置 presentation
- 架构守卫：
  - 新增 `settings_plugin_ai_panel_reads_product_ai_catalog`，确认设置插件页读取 `clipboard.product.configure_ai`，并且不会混入 row-menu OCR 能力
- 后续 Windows/macOS/Linux 设置页可以用同一个 panel presentation 创建本平台原生控件；真实 provider client、token、prompt、skill registry 和持久化仍留给 product adapter 执行层

## step297
- 让 AI 能力理解当前主窗口行/选中项上下文，而不是只知道静态 catalog：
  - `ProductAiCapabilityDescriptor` 的 OCR 上下文新增 `SelectedFilePath`，对齐现有行菜单允许对文件项执行 OCR 的行为
  - `src/app_core/main_window.rs` 新增 `MainRowAiCapabilityPresentation`，记录可给 UI/AI 读取的 capability id、label、provider、action 和 result
  - 新增 `MainRowAiCapabilityPlan`，记录当前选择的 `ProductAiContextKind`、目标 item ids 和可用能力
  - 新增 `main_row_ai_capability_plan()`，根据当前行或选中行推断 selected text/image/file context，并按 RowContextMenu surface 从 product AI catalog 生成去重后的能力列表
  - `app_core.rs` re-export plan 类型和函数，方便 Windows/macOS/Linux host 与 product adapter 复用
- 架构守卫：
  - 新增 `main_row_ai_capability_plan_describes_selected_context_for_ai`，覆盖文本、图片、文件和混合选择，确认文本能力、OCR 能力、目标 ids 和上下文都从同一 catalog 派生
- 后续 AI agent / LLM / skill registry 可以先读取这个 plan 理解当前 UI 选择，再决定是否调用具体 product adapter；平台 native menu 只负责展示和触发

## step298
- 让主窗口 AI plan 能生成标准 `ProductAiInvocation`，把“理解当前上下文”推进到“准备调用请求”：
  - `src/app_core/main_window.rs` 新增 `main_row_ai_invocation()`，输入当前 `MainRowAiCapabilityPlan`、capability id 和 prompt/input text
  - 函数会先确认 capability 属于当前 plan，再生成 `ProductAiInvocation { capability_id, input_text, context_item_ids }`
  - `app_core.rs` re-export `main_row_ai_invocation()`，方便 Windows/macOS/Linux host 和 product adapter 使用同一入口
- 架构守卫：
  - 新增 `main_row_ai_invocation_uses_capability_plan_targets`，确认可用能力能生成 invocation，且当前上下文不支持的 OCR capability 不会被文本行错误调用
- 后续 LLM client、skill registry 和 product adapter executor 可以直接消费 `ProductAiInvocation`，native UI 不再手写 capability id 或 item id 列表

## step299
- 给 `ProductAiInvocation` 增加 provider 路由计划，让三平台 UI 不需要根据 capability id 字符串判断该走 LLM、skills 还是 product adapter：
  - `src/app_core/product_adapter.rs` 新增 `ProductAiExecutionPlan`
  - 新增 `product_ai_execution_plan()`，根据 invocation 的 capability id 回查 catalog，生成 provider/action/result 路由信息
  - 未知 capability id 返回 `None`，避免错误或过期 UI 入口进入执行层
- 架构守卫：
  - 新增 `product_ai_execution_plan_routes_invocation_to_provider_family`，覆盖 LLM clean、skill translate、product OCR 和 unknown capability
- 后续真实执行器只需要匹配 `ProductAiProviderKind::{Llms, Skills, ProductAdapter}`，平台 host 不再手写 provider 分发逻辑

## step300
- 补齐 Windows 的顶层 native adapter boundary，让 Windows/macOS/Linux 三个平台都有对称的原生 UI 翻译层总表：
  - 新增 `src/windows_win32_adapter.rs`
  - 新增 `WindowsWin32HostBinding`，为当前 Windows/ZSUI host contract 命名 Win32/GDI adapter 入口，例如 `win32_main_window_pair`、`win32_edit_search_control`、`gdi_renderer`、`windows_clipboard_host`、`win32_settings_window`
  - 新增 `REQUIRED_WINDOWS_WIN32_HOST_BINDINGS`，覆盖 lifecycle、command、main execution plan、style/control/text、renderer、clipboard、menu、dialog、IME、window identity、paste target、main/settings host 等 26 个 adapter binding
  - 新增 `WindowsWin32ControlRole`，把 `SettingsComponentKind` 翻译成 Win32 控件角色，例如 `STATIC`、`EDIT`、`BUTTON.checkbox`、`COMBOBOX`、`BUTTON.default`
  - 新增 `WindowsWin32AdapterBoundary`，从 core contract 常量生成 dependency-free adapter boundary
  - Windows `main()` 现在会构造 `WindowsWin32AdapterBoundary::default_from_core_contract()`，防止 adapter 边界成为孤立文件
- 架构守卫：
  - `platform_entry_points_are_cfg_gated_for_windows_macos_and_linux` 要求 `windows_win32_adapter` 模块存在，并检查 Win32 adapter boundary、binding list 和 control role
  - 新增 `windows_win32_adapter_boundary_covers_current_zsui_hosts`
  - 新增 `windows_win32_control_roles_map_settings_component_kinds`
- 后续三个平台可以按同一模式推进：Windows 替换/收紧 Win32 host 内部，macOS 接 AppKit/SwiftUI，Linux 接 GTK/libadwaita，但主程序和 ZSUI contract 仍保持同一套 Rust 语义

## step301
- 补齐 Linux GTK/libadwaita adapter boundary 的主窗口执行计划桥，让 Linux 与 Windows/macOS 在顶层 native adapter 形状上对齐：
  - `LinuxHostContractSummary` 新增 `main_execution_plans`，从 `REQUIRED_MAIN_HOST_EXECUTION_PLAN_KINDS` 读取主窗口 Search/OpenSettings/HideWindow/CloseWindow/InvokeMenuCommand 计划数量
  - `LinuxGtkHostBinding` 新增 `MainExecutionPlan`，映射到 `shared_main_execution_plan_bridge`
  - `REQUIRED_LINUX_GTK_HOST_BINDINGS` 从 25 个扩展到 26 个，覆盖 lifecycle、command、main execution plan、style/control/text、renderer、clipboard、menu、dialog、IME、window identity、paste target、main/settings host 等 adapter binding
  - `LinuxGtkAdapterBoundary` 现在保存并暴露 main execution plan 数量，真实 GTK/libadwaita host 后续只需要翻译共享计划，不需要在 Linux 侧重新发明主窗口命令语义
- 架构守卫：
  - `linux_host_scaffold_consumes_zsui_contract_summary` 验证 Linux contract summary 消费 `REQUIRED_MAIN_HOST_EXECUTION_PLAN_KINDS`
  - `linux_gtk_adapter_boundary_covers_current_zsui_hosts` 验证 Linux GTK boundary 包含 `MainExecutionPlan` 并与 core plan 数量一致
  - `platform_entry_points_are_cfg_gated_for_windows_macos_and_linux` 要求 Linux GTK adapter 保留 `shared_main_execution_plan_bridge`
- 现在三平台顶层 native adapter boundary 都承认同一套主程序执行计划：主程序继续只输出 Rust 语义，Windows/macOS/Linux 分别翻译为 Win32、AppKit/SwiftUI、GTK/libadwaita 原生行为

## step302
- 给三平台 native adapter boundary 加统一 manifest，避免 Windows/macOS/Linux 只是三个相似但互不相认的 adapter 文件：
  - 新增 `src/app_core/native_adapter_manifest.rs`
  - 新增 `NativeUiPlatform::{Windows, Macos, Linux}`
  - 新增 `NativeUiToolkit::{Win32Gdi, AppKitSwiftUI, Gtk4Libadwaita}`
  - 新增 `NativeUiAdapterManifest`，统一记录 platform、toolkit、adapter binding 数量、main execution plan 数量和 shared non-host protocol 数量
  - `WindowsWin32AdapterBoundary::manifest()`、`MacosAppKitAdapterBoundary::manifest()`、`LinuxGtkAdapterBoundary::manifest()` 都返回同一结构
- 架构守卫：
  - 三个平台 adapter boundary 测试现在验证 manifest 与各自 binding list、`REQUIRED_MAIN_HOST_EXECUTION_PLAN_KINDS` 和 `SHARED_NON_HOST_UI_PROTOCOLS` 一致
  - `platform_entry_points_are_cfg_gated_for_windows_macos_and_linux` 要求三个 adapter 都暴露 `NativeUiAdapterManifest`
- 后续其他 Rust 程序复用 ZSUI 时，可以先读取统一 manifest 决定接 Win32/GDI、AppKit/SwiftUI 还是 GTK/libadwaita backend，再把自己的 product adapter 接到同一套 Rust UI contract

## step303
- 给 AI 能力层加统一 integration manifest，让后续 LLMs / skills / product adapter 能被 native host 或 AI agent 快速理解：
  - `src/app_core/product_adapter.rs` 新增 `ProductAiIntegrationManifest`
  - 新增 `product_ai_integration_manifest()`，从 `PRODUCT_AI_CAPABILITY_CATALOG` 推导总能力数、LLM 能力数、skill 能力数、product-adapter 能力数、涉及的 UI surface、输入 context 和结果类型
  - manifest 只做能力索引，不引入真实模型 client、skill registry、prompt/token 配置或产品数据访问，继续保持执行层属于 product adapter
- 架构守卫：
  - `product_ai_capabilities_stay_in_product_adapter_layer` 现在验证 manifest 与 catalog 同源，覆盖 LLMs、skills、product adapter 三类 provider，并确认 RowContextMenu/MainWindow/SettingsPluginPage、SelectedText/Image/File/SettingsProfile 等 context 暴露给宿主读取
- 后续 Windows/macOS/Linux 原生 UI 可以先读 native adapter manifest 确定平台 backend，再读 AI integration manifest 确定可展示/可调用的 AI 能力，把真实执行交给 LLM、skill registry 或 product adapter executor

## step304
- 新增 ZSUI 框架级复用入口，让其他 Rust 程序不需要分别翻版本、surface、native adapter 和 AI catalog 文件：
  - 新增 `src/app_core/framework_manifest.rs`
  - 新增 `ZsuiFrameworkManifest`
  - 新增 `zsui_framework_manifest()`，统一暴露框架名称/tagline、`APP_CORE_API_VERSION`、支持的平台 `Windows/macOS/Linux`、支持的 toolkit `Win32Gdi/AppKitSwiftUI/Gtk4Libadwaita`、`REQUIRED_UI_HOST_SURFACES`、`SHARED_NON_HOST_UI_PROTOCOLS` 和 `ProductAiIntegrationManifest`
  - `src/app_core/native_adapter_manifest.rs` 新增 `SUPPORTED_NATIVE_UI_PLATFORMS` 与 `SUPPORTED_NATIVE_UI_TOOLKITS`，供框架 manifest 和后续复用方同源读取
- 架构守卫：
  - 新增 `zsui_framework_manifest_is_single_reuse_entry_point`，验证 framework manifest 与 ZSUI identity、API version、host surfaces、shared protocols、AI catalog 和三平台/toolkit 列表保持一致
- 这一步把复用路径变成三层：
  - `zsui_framework_manifest()` 读全局框架能力
  - `NativeUiAdapterManifest` 读具体平台 backend
  - `ProductAiIntegrationManifest` 读 LLMs / skills / product adapter 能力摘要

## step305
- 扩展 AI integration manifest，从“能力数量摘要”推进到“执行器路由摘要”：
  - `ProductAiIntegrationManifest` 新增 `providers`，列出当前 catalog 涉及的 `Llms / Skills / ProductAdapter`
  - `ProductAiIntegrationManifest` 新增 `actions`，列出 clean、summarize、explain、translate、invoke skill、OCR、configure provider 等 action kind
  - 新增 `ProductAiExecutionRoute`，记录 capability id 对应的 provider/action/result
  - `product_ai_integration_manifest()` 现在从同一份 `PRODUCT_AI_CAPABILITY_CATALOG` 推导 execution routes，避免 UI host 或 AI executor 手写 capability id 分发表
- 架构守卫：
  - `product_ai_capabilities_stay_in_product_adapter_layer` 验证 providers/actions/execution_routes 与 catalog 一致
  - `zsui_framework_manifest_is_single_reuse_entry_point` 验证 framework manifest 中的 AI routes 与 catalog id 同源
- 后续真实执行层可以按 manifest 先准备 LLM client、skill registry 和 product adapter executor，再用 `product_ai_execution_plan()` 路由具体 invocation；Windows/macOS/Linux native UI 仍只负责展示和触发语义 action

## step306
- 给外部 Rust 程序复用 ZSUI 补通用 product adapter 接入契约：
  - 新增 `ProductAdapterContractSurface`
  - 新增 `REQUIRED_PRODUCT_ADAPTER_CONTRACT_SURFACES`
  - 新增 `ProductAdapterIntegrationContract`
  - 新增 `product_adapter_integration_contract()`，声明 product adapter 至少需要提供 product identity、product state model、product command executor、settings model、async event bridge 和 AI capability catalog
  - `ZsuiFrameworkManifest` 新增 `product_adapter` 字段，把 product adapter contract 和 AI routes 一起暴露给复用方
- 架构守卫：
  - `product_adapter_integration_contract_names_reusable_boundaries` 验证 product adapter contract 的 6 个接入面
  - `zsui_framework_manifest_is_single_reuse_entry_point` 验证 framework manifest 同时暴露 product adapter contract 和 AI execution routes
- 这样其他 Rust 应用复用时的路径更明确：应用实现 product adapter contract，平台选择 native adapter manifest，AI 执行层读取 AI manifest/routes

## step307
- 给三平台 native backend 增加可发现 catalog，让复用方不只知道支持 Windows/macOS/Linux，还能知道应该接哪个 adapter boundary：
  - 新增 `NativeUiBackendDescriptor`
  - 新增 `SUPPORTED_NATIVE_UI_BACKENDS`
  - 三个 backend descriptor 分别指向 `WindowsWin32AdapterBoundary` / `src/windows_win32_adapter.rs`、`MacosAppKitAdapterBoundary` / `src/macos_appkit_adapter.rs`、`LinuxGtkAdapterBoundary` / `src/linux_gtk_adapter.rs`
  - `ZsuiFrameworkManifest` 新增 `native_backends`，把 backend descriptor 与平台/toolkit 列表一起暴露
- 架构守卫：
  - `zsui_framework_manifest_is_single_reuse_entry_point` 验证三平台 backend descriptor 的 platform、toolkit、adapter boundary 和 module path
- 复用路径进一步明确为：读 `zsui_framework_manifest()` -> 选择 `native_backends` 中的平台 adapter -> 实现 `ProductAdapterIntegrationContract` -> 读取 AI routes 准备 LLM/skills/product adapter executor

## step308
- 给 native backend catalog 增加查询 helper，让复用方按平台或 toolkit 直接解析 adapter boundary：
  - 新增 `native_ui_backend_for_platform()`
  - 新增 `native_ui_backend_for_toolkit()`
  - `zsui_framework_manifest_is_single_reuse_entry_point` 覆盖 Windows/macOS/Linux 平台查询和 GTK/libadwaita toolkit 查询
- 后续外部程序可先按目标平台解析 backend descriptor，再加载对应 native adapter boundary，不需要手写字符串匹配或平台分支表

## step309
- 给 native backend catalog 增加当前编译目标解析入口，减少复用方自己写 `cfg(target_os)` 分支：
  - 新增 `native_ui_platform_for_current_target()`
  - 新增 `native_ui_backend_for_current_target()`
  - `zsui_framework_manifest_is_single_reuse_entry_point` 在当前测试目标上验证 current backend 能解析到对应平台
- 后续复用方可以直接按当前 Rust target 选择 Win32/GDI、AppKit/SwiftUI 或 GTK/libadwaita adapter descriptor，再接入对应 native host

## step310
- 给 native backend descriptor 增加实现状态，避免复用方误以为三个平台都已经是完整 native runtime：
  - 新增 `NativeUiBackendStatus`
  - Windows backend 标记为 `NativeHostIntegrated`
  - macOS/Linux backend 标记为 `AdapterBoundaryScaffold`
  - `NativeUiBackendStatus::status_name()` 提供稳定文本，供文档、外部工具或 AI agent 读取
- 架构守卫：
  - `zsui_framework_manifest_is_single_reuse_entry_point` 验证三个 backend descriptor 的 status 与 status_name
- 后续进度推进时，可以逐步把 macOS/Linux 从 adapter scaffold 提升到 native host integrated，而不改变 framework manifest 的读取方式

## step311
- 将 backend 实现状态下沉到每个平台自己的 `NativeUiAdapterManifest`：
  - `NativeUiAdapterManifest` 新增 `status`
  - Windows manifest 返回 `NativeHostIntegrated`
  - macOS/Linux manifest 返回 `AdapterBoundaryScaffold`
- 架构守卫：
  - 三平台 adapter boundary 测试现在同时验证 platform、toolkit、status、binding count、main execution plan 和 shared protocol count
- 这样复用方读取 framework backend catalog 或读取某个具体 adapter boundary manifest，都能得到一致的实现成熟度信息

## step312
- 给 backend status 增加语义 helper，避免复用工具或 AI agent 直接比较状态字符串：
  - `NativeUiBackendStatus::is_native_runtime_ready()`
  - `NativeUiBackendStatus::is_scaffold()`
  - `zsui_framework_manifest_is_single_reuse_entry_point` 验证 Windows ready、macOS/Linux scaffold
- 后续外部程序可以用 helper 判断是否能直接运行 native host，或是否仍需等待/接入真实 AppKit/GTK runtime

## step313
- 扩展 AI execution plan/route，让 executor 能读到 source surface 和 required contexts：
  - `ProductAiExecutionPlan` 新增 `surface` 和 `input_contexts`
  - `ProductAiExecutionRoute` 新增 `surface` 和 `input_contexts`
  - `product_ai_execution_plan()` 从 catalog descriptor 同源填充 provider/action/surface/context/result
- 架构守卫：
  - `product_ai_capabilities_stay_in_product_adapter_layer` 验证 OCR route 暴露 RowContextMenu 和图片/文件/item id context
  - `product_ai_execution_plan_routes_invocation_to_provider_family` 验证 execution plan 暴露 LLM/product adapter 的 surface/context
- 后续 LLM client、skill registry 或 product adapter executor 可以按 plan 判断需要 prompt、选中文本、图片、文件路径或 settings profile，不需要回查 UI 菜单字符串

## step314
- 给 AI capability 相关 enum 增加稳定名称 helper，方便 LLM、skills、外部工具或 agent 读取 manifest：
  - `ProductAiProviderKind::provider_name()`
  - `ProductAiUiSurface::surface_name()`
  - `ProductAiActionKind::action_name()`
  - `ProductAiContextKind::context_name()`
  - `ProductAiResultKind::result_name()`
- 架构守卫：
  - `product_ai_capabilities_stay_in_product_adapter_layer` 验证 provider/action/surface/context/result 的稳定名称
- 后续 AI manifest 可以直接输出这些稳定文本，不需要让 executor 或 agent 依赖 Rust enum Debug 格式

## step315
- 将稳定名称 helper 扩展到 AI execution plan/route：
  - `ProductAiExecutionPlan::{provider_name, action_name, surface_name, input_context_names, result_name}`
  - `ProductAiExecutionRoute::{provider_name, action_name, surface_name, input_context_names, result_name}`
- 架构守卫：
  - `product_ai_capabilities_stay_in_product_adapter_layer` 验证 OCR route 的稳定 provider/action/surface/context/result 名称
  - `product_ai_execution_plan_routes_invocation_to_provider_family` 验证 LLM execution plan 的稳定名称
- 后续 LLM、skills、product adapter executor 或外部 AI agent 可以直接读取 route/plan 的稳定文本，不需要依赖 enum Debug 输出或重复映射

## step316
- 给三平台 UI 复用入口增加机器可读 readiness/capability 层：
  - `NativeUiPlatform::platform_name()` 和 `NativeUiToolkit::toolkit_name()` 暴露稳定平台/toolkit 名称
  - `NativeUiAdapterCapability` 和 `REQUIRED_NATIVE_UI_ADAPTER_CAPABILITIES` 列出主窗口、设置窗口、菜单、剪贴板、renderer、IME、文件/输入/编辑对话框、paste target、window identity 和 main execution plan bridge 等 adapter 能力
  - `native_ui_backend_capability_matrix()` 按 Windows/macOS/Linux 输出同一能力矩阵，并保留 backend ready/scaffold 状态
  - `zsui_reuse_readiness_report()` 汇总平台名称、ready/scaffold 平台、adapter capability 名称、product adapter surfaces 和 AI provider families
- 架构守卫：
  - `zsui_framework_manifest_is_single_reuse_entry_point` 验证三平台稳定名称、adapter capability 列表和 capability matrix
  - `zsui_reuse_readiness_report_names_platform_ai_and_product_boundaries` 验证复用方/AI agent 可从单一报告读取平台、product adapter 和 AI 边界
- 后续其他 Rust 程序可以先读 readiness report 决定接 Windows/macOS/Linux 哪个 native host、还缺哪些 adapter 能力，以及 AI 应该走 LLMs、skills 还是 product adapter

## step317
- 给 Product Adapter 增加可执行接入 checklist，让复用方知道主程序/AI 层必须实现什么，而不是只知道有几个 surface：
  - 新增 `ProductAdapterIntegrationTask`
  - 新增 `REQUIRED_PRODUCT_ADAPTER_INTEGRATION_TASKS`
  - 新增 `ProductAdapterReuseChecklist`
  - 新增 `product_adapter_reuse_checklist()`
- checklist 明确 9 个接入任务：
  - provide product identity
  - project product state
  - execute product commands
  - bind settings model
  - bridge async events
  - publish AI catalog
  - connect LLM executor
  - connect skill registry
  - connect product AI tools
- `zsui_reuse_readiness_report()` 现在同时暴露 `product_adapter_task_names`，让 AI agent 或其他 Rust 产品可以从一个入口读取三平台 UI 能力、product adapter surfaces、product adapter tasks 和 AI provider families
- 架构守卫：
  - `product_adapter_integration_contract_names_reusable_boundaries` 验证 9 个接入任务、surface 名称、AI providers 和 route ids
  - `zsui_reuse_readiness_report_names_platform_ai_and_product_boundaries` 验证 readiness report 暴露 product adapter task names

## step318
- 给 AI provider 增加 executor boundary 映射，让 `llms`、`skills`、`product_adapter` 能直接接到真实执行层：
  - 新增 `ProductAiExecutorBoundary::{LlmExecutor, SkillRegistry, ProductAdapterTools}`
  - `ProductAiProviderKind::executor_boundary()` 将 provider 映射为 `llm_executor` / `skill_registry` / `product_adapter_tools`
  - `ProductAiProviderKind::integration_task()` 将 provider 映射到 `connect_llm_executor` / `connect_skill_registry` / `connect_product_ai_tools`
  - `ProductAiExecutionPlan` 和 `ProductAiExecutionRoute` 新增 `executor_boundary`
  - `ProductAiIntegrationManifest` 新增 `executor_boundaries`
  - `ProductAdapterReuseChecklist` 和 `zsui_reuse_readiness_report()` 新增 `ai_executor_boundary_names`
- 架构守卫：
  - `product_ai_capabilities_stay_in_product_adapter_layer` 验证 manifest 和 OCR route 的 executor boundary / task name
  - `product_ai_execution_plan_routes_invocation_to_provider_family` 验证 LLM、skill、product-adapter 三类 invocation 会路由到对应 executor boundary
  - `product_adapter_integration_contract_names_reusable_boundaries` 和 readiness report 测试验证 executor boundary 名称可被外部工具读取
- 后续真实 AI 执行器可以只按 execution plan 的 executor boundary 接 LLM client、skill registry 或 product-specific tools，Windows/macOS/Linux host 仍只负责触发和展示

## step319
- 新增单平台复用启动计划入口，减少外部 Rust 程序接入时需要自己拼 manifest/checklist 的步骤：
  - 新增 `ZsuiReuseBootstrapPlan`
  - 新增 `zsui_reuse_bootstrap_plan(platform)`
  - bootstrap plan 汇总目标平台、平台/toolkit 名称、backend status、adapter boundary、adapter module path、native adapter capability 名称、product adapter surface/task 名称、AI provider 名称和 AI executor boundary 名称
  - `ZsuiReuseBootstrapPlan::{native_runtime_ready, scaffolded}` 使用 backend status enum 判断当前平台成熟度
- 架构守卫：
  - `zsui_reuse_bootstrap_plan_combines_platform_product_and_ai_requirements` 验证 Windows plan 已 ready，macOS/Linux 仍是 scaffold，并且三者都能暴露各自 adapter boundary 与统一 product/AI 接入要求
- 后续复用方可以先按目标平台读取 bootstrap plan，再按计划接 native host、product adapter、LLM executor、skill registry 和 product-specific AI tools

## step320
- 将复用启动计划接到三个具体 native adapter boundary 上：
  - `WindowsWin32AdapterBoundary::reuse_bootstrap_plan()`
  - `MacosAppKitAdapterBoundary::reuse_bootstrap_plan()`
  - `LinuxGtkAdapterBoundary::reuse_bootstrap_plan()`
- 三个平台的 adapter 测试现在都验证：
  - 平台和 toolkit 名称
  - adapter boundary 名称
  - Windows 为 native runtime ready
  - macOS/Linux 为 adapter scaffold
  - native adapter capability 名称可读
  - AI executor boundaries 为 `llm_executor` / `skill_registry` / `product_adapter_tools`
- 这样复用方选中某个平台 adapter 后，不需要再回到 framework catalog 手动查找启动信息；adapter 自身就能给出 native host、product adapter 和 AI executor 接线计划

## step321
- 给 native adapter 增加机器可读 binding plan，让复用方和 AI agent 能看到每个平台具体要接哪些 native binding：
  - 新增 `NativeUiAdapterBindingPlan`
  - `WindowsWin32AdapterBoundary::binding_names()` / `adapter_binding_plan()`
  - `MacosAppKitAdapterBoundary::binding_names()` / `adapter_binding_plan()`
  - `LinuxGtkAdapterBoundary::binding_names()` / `adapter_binding_plan()`
- binding plan 暴露 platform、toolkit、status、adapter boundary 和稳定 binding 名称列表，例如：
  - Windows: `win32_main_window_pair`、`gdi_renderer`
  - macOS: `ns_window_pair`、`core_graphics_renderer`
  - Linux: `adw_application_window`、`gtk_snapshot_renderer`
- 架构守卫：
  - 三个平台 adapter 测试都验证 binding plan 的平台/toolkit/status、binding 数量，以及关键 binding 名称
- 后续 AI/skills/product adapter 可以读取 bootstrap plan 知道“要接什么能力”，再读取 binding plan 知道“当前平台用什么 native binding 名称实现”

## step322
- 新增 adapter reuse package，把每个平台 adapter 的复用信息合成一个对象：
  - 新增 `NativeUiAdapterReusePackage<TBootstrap>`
  - `WindowsWin32AdapterBoundary::reuse_package()`
  - `MacosAppKitAdapterBoundary::reuse_package()`
  - `LinuxGtkAdapterBoundary::reuse_package()`
- reuse package 包含：
  - `NativeUiAdapterManifest`
  - `ZsuiReuseBootstrapPlan`
  - `NativeUiAdapterBindingPlan`
- 三个平台 adapter 测试验证：
  - package 的 platform/toolkit/status 与 manifest 一致
  - bootstrap plan 与平台一致
  - binding plan 数量与 manifest binding count 一致
  - package 内能读取关键 native lifecycle binding 名称
- 后续复用方选中一个平台 adapter 后，可以直接拿 package 完成 native host、product adapter、AI executor 和 binding 发现，不需要分别调用 manifest/bootstrap/binding 三个入口

## step323
- 新增三平台 adapter parity report，用 reuse package 比较 Windows/macOS/Linux 是否保持同一复用形状：
  - 新增 `NativeUiAdapterParityReport`
  - 新增 `native_ui_adapter_parity_report(packages)`
- parity report 暴露：
  - platform/toolkit/status 名称
  - adapter boundary 名称
  - binding counts
  - main execution plan counts
  - shared non-host protocol counts
  - ready/scaffold 平台列表
  - binding count 是否与 manifest 一致
  - main execution plan/shared protocol count 是否三平台一致
- 架构守卫：
  - `native_ui_adapter_reuse_packages_report_three_platform_parity` 用三个 adapter 的 `reuse_package()` 验证 Windows/macOS/Linux 都是 27 个 binding、5 个 main execution plan、3 个 shared non-host protocol，并且 Windows ready、macOS/Linux scaffold
- 后续迁移真实 AppKit/GTK runtime 时，可以用 parity report 防止某个平台 adapter 落后或漏接核心 binding

## step324
- 将 adapter parity 接入 framework readiness，让外部工具可以从一个结果读取“可用性 + 三平台一致性”：
  - `ZsuiReuseReadinessReport` 新增 `adapter_parity: Option<NativeUiAdapterParityReport>`
  - `zsui_reuse_readiness_report()` 默认不绑定具体 adapter package，`adapter_parity` 为 `None`
  - 新增 `zsui_reuse_readiness_report_with_adapter_parity(packages)`，把选中的三平台 reuse package parity report 附加到 readiness report
- 架构守卫：
  - `zsui_reuse_readiness_report_names_platform_ai_and_product_boundaries` 验证基础 readiness 默认不携带 adapter parity
  - `native_ui_adapter_reuse_packages_report_three_platform_parity` 验证带 parity 的 readiness report 能读到三平台平台名、binding counts 和 manifest 一致性
- 后续 AI agent / skills / product adapter 可以优先读取带 parity 的 readiness report，一次知道支持平台、AI executor 边界、product adapter 任务以及三平台 adapter 是否保持一致

## step325
- 新增 AI/agent 可读 ZSUI context，让 LLM、skills 和 product adapter 能快速理解当前 UI framework：
  - 新增 `ZsuiAgentAiRouteSummary`
  - 新增 `ZsuiAgentContext`
  - 新增 `zsui_agent_context()`
  - 新增 `zsui_agent_context_with_adapter_parity(packages)`
- agent context 汇总：
  - framework name / api version
  - readiness report
  - 可选 adapter parity
  - AI route 的稳定字符串字段：capability id、provider、executor boundary、executor task、action、surface、input contexts、result
- 架构守卫：
  - `zsui_agent_context_summarizes_readiness_parity_and_ai_routes` 验证带 parity 的 context 能读取三平台 binding counts，并能读取 OCR route 的 product-adapter executor 边界和 context 列表
- 后续 AI agent 不需要解析 Rust enum Debug，也不需要跨多个 manifest 自己拼上下文；读 agent context 就能知道 ZSUI 平台状态和 AI 执行路线

## step326
- 扩展 AI/agent context，加入三平台 bootstrap 摘要：
  - 新增 `ZsuiAgentPlatformBootstrapSummary`
  - `ZsuiAgentContext` 新增 `platform_bootstrap`
  - `zsui_agent_context()` / `zsui_agent_context_with_adapter_parity()` 现在输出 Windows/macOS/Linux 的 platform、toolkit、backend status、adapter boundary、adapter module path 和 native capability 名称
- 架构守卫：
  - `zsui_agent_context_summarizes_readiness_parity_and_ai_routes` 现在同时验证三平台 bootstrap 摘要和 OCR AI route 摘要
- 后续 AI agent / skills 可以直接读 agent context 判断目标平台该接 `WindowsWin32AdapterBoundary`、`MacosAppKitAdapterBoundary` 还是 `LinuxGtkAdapterBoundary`，不需要再调用 bootstrap plan helper 自己拼三平台列表

## step327
- 给 AI/agent context 增加有序集成步骤清单：
  - 新增 `ZsuiAgentIntegrationStep`
  - `ZsuiAgentContext` 新增 `integration_steps`
  - 集成顺序固定为：选择 native adapter、校验 adapter capability parity、实现 product adapter surfaces、完成 product adapter tasks、连接 LLM executor、连接 skill registry、连接 product-specific AI tools
- 架构守卫：
  - `zsui_agent_context_summarizes_readiness_parity_and_ai_routes` 现在验证 7 个 integration steps 的名称、owner 和关键 required names
- 后续 AI agent / skills 不只知道三平台和 AI route，还能按稳定步骤把 native adapter、product adapter、LLM、skills 和 product tools 接起来

## step328
- 给 AI/agent context 增加三平台 native runtime gate：
  - 新增 `ZsuiAgentPlatformRuntimeGate`
  - `ZsuiAgentContext` 新增 `platform_runtime_gates`
  - runtime gate 固定包含 native event loop、window surfaces、control mapping、renderer、clipboard services、dialog services、settings surfaces 和 AI action presentation
- 架构守卫：
  - `zsui_agent_context_summarizes_readiness_parity_and_ai_routes` 现在验证 Windows 已 runtime ready 且没有缺失 gate，macOS/Linux 仍从 `native_event_loop` 开始补真实 native runtime
- 后续迁移 AppKit/SwiftUI 或 GTK/libadwaita 时，AI agent / skills 可以直接读 gate 判断下一步要补哪个 runtime 能力，而不是只看到“scaffold”这个粗粒度状态

## step329
- 将 native runtime gate 下沉到单平台 bootstrap plan：
  - `ZsuiReuseBootstrapPlan` 新增 `native_runtime_gate_names`
  - `ZsuiReuseBootstrapPlan` 新增 `missing_native_runtime_gate_names`
  - `ZsuiReuseBootstrapPlan` 新增 `next_native_runtime_gate_name`
  - `ZsuiAgentPlatformRuntimeGate` 现在复用 bootstrap plan 中的 gate 信息，避免 agent context 和单平台接入计划各维护一套 gate 规则
- 架构守卫：
  - `zsui_reuse_bootstrap_plan_combines_platform_product_and_ai_requirements` 现在验证 Windows 没有缺失 gate，macOS/Linux 缺失完整 runtime gate 且下一项是 `native_event_loop`
- 后续外部 Rust 程序只读取目标平台的 bootstrap plan，也能知道当前 native runtime 成熟度和下一步 AppKit/GTK 接入门槛

## step330
- 将 native runtime gate 升级为 capability plan：
  - 新增 `ZsuiNativeRuntimeGateCapabilityPlan`
  - `ZsuiReuseBootstrapPlan` 新增 `native_runtime_gate_plans`
  - `ZsuiAgentPlatformRuntimeGate` 新增 `gate_plans`
  - 每个 gate 现在能列出依赖的 native adapter capability，例如 `native_renderer` 依赖 `renderer` / `text_layout`
  - `ai_action_presentation` gate 同时列出 `publish_ai_catalog`、`connect_llm_executor`、`connect_skill_registry`、`connect_product_ai_tools` 和三个 AI executor boundary
- 架构守卫：
  - bootstrap 测试验证 event loop、renderer、AI presentation 三类 gate 的 adapter/product/AI 依赖
  - agent context 测试验证 macOS/Linux 也能读到同一份 gate capability plan
- 后续 macOS/Linux 迁移时，AI agent / skills 不只知道“缺哪个 gate”，还知道补这个 gate 要接哪些 adapter 能力和 AI/product adapter 边界

## step331
- 给 native runtime gate 增加 completion report：
  - 新增 `ZsuiNativeRuntimeGateCompletionReport`
  - `ZsuiReuseBootstrapPlan` 新增 `native_runtime_gate_completion`
  - `ZsuiAgentPlatformRuntimeGate` 新增 `completion`
  - completion report 暴露 total/completed/missing gate count、completion percent、missing gate names 和 next gate
- 架构守卫：
  - bootstrap 测试验证 Windows runtime gate 8/8 完成，macOS/Linux 0/8 且下一项为 `native_event_loop`
  - agent context 测试验证 AI/skills 读取到同一份 completion percent、missing count 和 next gate
- 后续迁移 macOS/Linux 时，可以逐步把单个 gate 标为完成，并让 AI agent / skills 用同一份报告判断真实进度

## step332
- 将 native runtime gate 继续映射到平台 binding 名称：
  - 新增 `ZsuiNativeRuntimeGatePlatformBindingPlan`
  - `ZsuiReuseBootstrapPlan` 新增 `native_runtime_gate_binding_plans`
  - `ZsuiAgentPlatformRuntimeGate` 新增 `gate_binding_plans`
  - 每个 gate 现在能从 adapter capability 继续翻译到具体平台 binding，例如 Windows `win32_main_window_pair`、macOS `ns_window_pair`、Linux `adw_application_window`
- 架构守卫：
  - bootstrap 测试验证 event loop、renderer、AI presentation gate 的平台 binding 名称
  - adapter reuse package 测试验证 gate binding plan 中的 binding 名称都存在于对应平台 adapter binding plan
  - agent context 测试验证 macOS/Linux AI 入口也能读取 AppKit/GTK binding 名称
- 后续 macOS/Linux 迁移时，AI agent / skills 可以从“缺哪个 gate”一路追到“这个 gate 要实现哪些 native binding”

## step333
- 给 adapter reuse package 增加 gate binding summary：
  - 新增 `ZsuiAdapterReusePackageGateBindingSummary`
  - 新增 `zsui_adapter_reuse_package_gate_binding_summaries(packages)`
  - summary 暴露平台/toolkit/status、adapter boundary、gate names、每个 gate 的 binding 数量、missing gate、next gate、completion percent，以及 gate binding 是否全部存在于 adapter binding plan
- 架构守卫：
  - `native_ui_adapter_reuse_packages_report_three_platform_parity` 现在验证三平台 gate binding summary 都能匹配对应 adapter binding plan
  - 同一测试验证 Windows 100% / macOS 0% / Linux 0% 的 runtime gate 完成度仍能从 package summary 读取
- 后续复用方拿到一个 adapter `reuse_package()` 后，不用再自己遍历 bootstrap 和 binding plan，就能确认 gate->binding 接线是否完整

## step334
- 给 adapter reuse package 增加 porting work items：
  - 新增 `ZsuiAdapterPortingWorkItem`
  - 新增 `zsui_adapter_reuse_package_porting_work_items(packages)`
  - work item 只输出未完成 runtime gate，包含平台/toolkit/status、adapter boundary、adapter module path、gate name、required adapter capabilities、required platform bindings、product adapter tasks 和 AI executor boundaries
- 架构守卫：
  - `native_ui_adapter_reuse_packages_report_three_platform_parity` 现在验证 Windows 没有 porting work item，macOS/Linux 各 8 个 work item
  - 同一测试验证 macOS 第一项指向 `src/macos_appkit_adapter.rs` 和 AppKit binding，Linux AI presentation 项指向 GTK/libadwaita binding 以及 LLM/skills/product adapter executor 边界
- 后续 AI agent / skills 可以直接读取 work items 执行 macOS/Linux native runtime 迁移，不需要自己从 gate summary 反推任务

## step335
- 将 porting work items 接入 AI/agent context：
  - `ZsuiAgentContext` 新增 `porting_work_items`
  - `zsui_agent_context()` 不绑定具体 adapter package，默认 work items 为空
  - `zsui_agent_context_with_adapter_parity(packages)` 现在从 adapter reuse packages 生成 macOS/Linux 未完成 runtime gate 施工项
- 架构守卫：
  - `zsui_agent_context_summarizes_readiness_parity_and_ai_routes` 现在验证 agent context 中直接可读 16 个 porting work items，macOS/Linux 各 8 个
  - 同一测试验证 macOS renderer 项指向 AppKit binding，Linux AI presentation 项指向 GTK/libadwaita binding 和 AI executor boundaries
- 后续 AI agent / skills 读取一个 agent context 就能同时理解三平台状态、AI routes、integration steps 和 macOS/Linux 施工清单

## step336
- 将 agent 视角从 ZSClip 产品能力继续上移到“通用 Rust UI 程序”能力：
  - 新增 `ProductAdapterFunctionFlowKind`
  - 新增 `ProductAdapterFunctionFlow`
  - 新增 `product_adapter_function_flows()`
  - 新增 `ProductAdapterPipelineStageKind`
  - 新增 `ProductAdapterPipelineStage`
  - 新增 `product_adapter_execution_pipeline()`
  - 新增 `ProductAdapterHost` trait
  - 新增 `ProductAdapterIdentity`、`ProductAdapterProjectedState`、`ProductAdapterSettingsSnapshot`、`ProductAdapterCommandResult`、`ProductAdapterAsyncBridgeResult`
  - 新增 `ProductAdapterHostMethod` 和 `required_product_adapter_host_method_names()`
  - 新增 `NativeRuntimeDriver`
  - 新增 `NativeRuntimeStartupRequest`、`NativeRuntimeStartupResult`
  - 新增 `NativeRuntimeDriverOperation` 和 `required_native_runtime_driver_operation_names()`
  - 新增 `ZsuiReusableRuntimeHarness`
  - 新增 `ZsuiReusableRuntimeHarnessStage` 和 `zsui_reusable_runtime_harness_stage_names()`
  - `MacosApplicationModel` 实现 `NativeRuntimeDriver`
  - `LinuxApplicationModel` 实现 `NativeRuntimeDriver`
  - 新增 `WindowsWin32RuntimeDriver`，让 Windows adapter 也显式暴露同一 runtime driver shape
  - Windows/macOS/Linux adapter binding 都新增 runtime driver binding：
    - Windows `win32_native_runtime_driver`
    - macOS `appkit_native_runtime_driver`
    - Linux `gtk_native_runtime_driver`
  - 三平台 adapter binding 数从 26 提升到 27
  - 新增 `ZsuiReusableAppFeatureRequirement`
  - 新增 `ZsuiReusableAppFeaturePlatformStatus`
  - 新增 `ZsuiReusableAppBlueprint`
  - 新增 `zsui_reusable_app_blueprint()`
  - `ZsuiAgentContext` 新增 `reusable_app_blueprint`
  - reusable blueprint 现在暴露 native runtime driver operation names，AI/skills 能知道三平台 runtime 必须提供哪些入口
  - reusable blueprint 现在暴露 runtime harness stage names，AI/skills 能知道 native runtime、product adapter 和 AI invocation 的统一接线顺序
  - reusable blueprint 现在暴露 product adapter method names，AI/skills 能直接知道外部产品需要实现哪些 Rust 方法
  - product adapter 功能流覆盖 app bootstrap、state projection、user command、settings sync、async event 和 AI action，避免框架只迁 UI 不迁功能接线
  - product execution pipeline 固定 UI intent -> product state -> product command -> async event -> AI action -> UI update projection 的执行顺序
  - 通用 feature 固定覆盖 native app entry、window surfaces、control mapping、renderer/text layout、system services、settings surfaces 和 AI action surfaces
  - 每个 feature 都能列出需要的 runtime gate、adapter capability、平台 binding、product adapter task 和 AI executor boundary
- 架构守卫：
  - `zsui_agent_context_summarizes_readiness_parity_and_ai_routes` 现在验证 reusable blueprint 暴露通用 Rust UI contract、三平台、native runtime driver operations、7 类通用 feature、product adapter surfaces、6 条 product function flows、6 段 product execution pipeline 和 AI executor boundaries
  - 新增 `native_runtime_driver_trait_executes_platform_entry_path`，用 recording runtime driver 验证 start runtime、dispatch UI command、poll app event 和 request shutdown
  - 新增 `windows_win32_runtime_driver_exposes_common_runtime_path`
  - 新增 `macos_application_model_implements_native_runtime_driver`
  - 新增 `linux_application_model_implements_native_runtime_driver`
  - `native_ui_adapter_reuse_packages_report_three_platform_parity` 现在验证三平台 binding count 都为 27，并且 runtime driver binding 参与各自 adapter binding plan
  - 新增 `product_adapter_host_trait_executes_reusable_function_path`，用 recording adapter 验证 UI command、settings、async event 和 AI plan 都能经由 `ProductAdapterHost` trait 承接
  - 新增 `reusable_runtime_harness_connects_native_driver_and_product_adapter`，验证同一 harness 可以串起 native runtime driver、product adapter、async event 和 AI invocation
  - 同一测试验证 Windows 通用入口已 ready，macOS system services 被 native clipboard/dialog gates 阻塞，Linux AI action surfaces 指向 GTK/libadwaita binding 与 LLM/skills/product adapter 边界
- 这一步明确 ZSClip 只是先行产品；外部工具复用 ZSUI 时应该实现自己的 product adapter，而不是继承剪贴板历史业务逻辑

## step337
- 开始把 ZSClip 自己接成第一套真实 product adapter，而不是只迁 UI：
  - 新增 `src/app/product_adapter.rs`
  - 新增 `ZsclipProductSettingsSnapshot`
  - 新增 `ZsclipProductSnapshot`
  - 新增 `ZsclipProductAdapter`
  - `ZsclipProductAdapter` 实现 `ProductAdapterHost`
  - window command 映射为 `zsclip.window.*`
  - row command 映射为 `zsclip.row.*`
  - tray command 映射为 `zsclip.tray.*`
  - async event 映射为稳定事件名，例如 `cloud_sync_ready`
  - AI execution plan 通过 `ProductAdapterHost::execute_ai_plan` 接收，后续可继续接 llms / skills / product adapter executor
- 架构守卫：
  - 新增 `zsclip_product_adapter_routes_clipboard_commands_events_and_ai`
  - 验证 ZSClip row copy、tray LAN toggle、settings bind、cloud sync event、AI clean text plan 都能走 product adapter
- 这一步让“剪贴板功能”开始进入通用 Rust UI/product adapter 路径，后续 macOS/Linux 不需要复制 Windows UI 里的产品业务语义

## step338
- 给 ZSClip product adapter 增加 AI/skills 可读 manifest：
  - 新增 `ZsclipProductCommandRoute`
  - 新增 `ZsclipProductEventRoute`
  - 新增 `ZsclipProductAdapterManifest`
  - 新增 `zsclip_product_adapter_manifest()`
  - 新增 `zsclip_product_command_routes()`
  - 新增 `zsclip_product_event_routes()`
  - manifest 暴露 command family、result name、execution owner、是否需要选中项、关联 AI capability id
  - manifest 暴露 async event name 到 product effect name 的映射
  - manifest 暴露 ZSClip 当前 AI capability ids 和 provider names，覆盖 llms / skills / product_adapter
- 架构守卫：
  - `zsclip_product_adapter_routes_clipboard_commands_events_and_ai` 现在同时验证 manifest 中的 row image OCR、tray LAN toggle、cloud sync event、AI provider names 和 AI capability ids
- 这一步让 macOS/Linux host 和后续 AI agent 可以先读 ZSClip 产品能力 manifest，再决定如何把原生控件、菜单、LLM、skills 和 product adapter executor 接起来

## step339
- 按“先做其他平台完整功能，不继续堆抽象”的方向调整 product adapter 放置：
  - 将 `src/app/product_adapter.rs` 移到顶层 `src/zsclip_product_adapter.rs`
  - `src/main.rs` 直接挂载 `mod zsclip_product_adapter`
  - 移除 Windows `app` 模块内的 product adapter 子模块
  - `ZsclipProductSettingsSnapshot` 不再依赖 Windows `AppSettings`
  - `ZsclipProductSnapshot` 不再依赖 Windows `AppState`
- macOS/Linux 复用守卫：
  - 新增 `macos_application_can_reuse_zsclip_product_adapter`
  - 新增 `linux_application_can_reuse_zsclip_product_adapter`
  - macOS 直接读取 ZSClip manifest 并执行 row copy product command
  - Linux 直接读取 ZSClip manifest 并执行 tray LAN toggle product command
- 这一步把 ZSClip 产品功能语义从 Windows app 模块挪出来，macOS/Linux 后续可以直接复用同一 product adapter，而不是复制 Windows 事件处理逻辑

## step340
- 让 macOS/Linux runtime command dispatch 真正进入 ZSClip product adapter：
  - `MacosApplicationModel` 新增 `product_adapter: ZsclipProductAdapter`
  - `MacosApplicationModel` 新增 `product_command_results`
  - `LinuxApplicationModel` 新增 `product_adapter: ZsclipProductAdapter`
  - `LinuxApplicationModel` 新增 `product_command_results`
  - macOS/Linux `dispatch_ui_command()` 现在会先执行 `ProductAdapterHost::execute_product_command()`，再保留平台 command queue
  - macOS/Linux runtime driver 测试验证 `OPEN_SETTINGS` 会得到 `zsclip.window.open_settings`
  - macOS 复用测试验证 runtime dispatch row copy 得到 `zsclip.row.copy`
  - Linux 复用测试验证 runtime dispatch tray LAN toggle 得到 `zsclip.tray.toggle_lan_sync`
- 这一步让其他平台不只是“能读取”产品能力，而是 UI command 已经实际进入产品功能层

## step341
- 继续把 macOS/Linux 的应用事件接入 ZSClip product adapter：
  - `MacosApplicationModel` 新增 `product_event_results`
  - `LinuxApplicationModel` 新增 `product_event_results`
  - macOS `route_application_event()` 现在同时调用 `ProductAdapterHost::bridge_async_event()`
  - Linux 新增 `route_application_event()`，把 shared `ApplicationEvent` 桥接到 product adapter
  - macOS 测试验证 `CloudSyncReady` -> `cloud_sync_ready`、`LanSyncReady` -> `lan_sync_ready`
  - Linux 测试验证 `CloudSyncReady` -> `cloud_sync_ready`
- 文档补充其他 AI 做多平台预览/测试的分层路线：读 agent context / blueprint / bootstrap / product manifest，先生成共享预览，再跑 scaffold tests，最后在真实 macOS/Linux runner 做原生截图和交互测试

## step342
- 将 native runtime gate 进度从“一刀切 ready/scaffold”改成逐 gate 记录：
  - `ZsuiNativeRuntimeGateCompletionReport` 新增 `completed_gate_names`
  - Windows 仍是 8/8 runtime gate 完成
  - macOS/Linux 现在记录 `native_event_loop` runtime-driver 契约已完成，进度为 1/8
  - macOS/Linux 下一项变为 `native_window_surfaces`，porting work items 从 16 项降到 14 项
- 这一步没有宣称 macOS/Linux 已经真实原生完成；它只把已经通过 `NativeRuntimeDriver` 启动、命令、事件和 shutdown 测试的共享 runtime 入口从“未记录进度”变成可读证据，后续继续接 AppKit/GTK 真实窗口 surface

## step343
- 将 macOS/Linux 的 `native_window_surfaces` 契约进度入账：
  - macOS/Linux completed gate 从 `native_event_loop` 扩展到 `native_event_loop` + `native_window_surfaces`
  - 两个平台 runtime gate 进度变为 2/8，completion percent 为 25
  - 下一项变为 `native_control_mapping`
  - porting work items 从 14 项降到 12 项
- 依据是两侧都已有 main/settings window、settings dropdown、text input dialog、edit text dialog 和 transient window host 契约与测试；这仍然不是真实 AppKit/GTK 截图完成，真实平台 runner 后续还要验证原生窗口行为

## step344
- 将 macOS/Linux 的 `native_control_mapping` 契约进度入账：
  - completed gate 扩展为 `native_event_loop`、`native_window_surfaces`、`native_control_mapping`
  - 两个平台 runtime gate 进度变为 3/8，completion percent 为 37
  - 下一项变为 `native_renderer`
  - porting work items 从 12 项降到 10 项
- 依据是两侧都有 main search control、settings control host、settings dropdown、text input dialog、edit text dialog、IME host 与 native control mapper 契约/测试；后续仍要接真实 CoreGraphics/CoreText 与 GTK snapshot/Pango 渲染

## step345
- 补齐 macOS renderer/text-layout scaffold，并将 `native_renderer` 契约进度入账：
  - 新增 `MacosTextLayout` / `MacosRenderer`
  - 新增 `MacosTextLayoutAction` / `MacosRenderCommand`
  - `MacosApplicationModel` 持有 text layout 和 renderer host
  - macOS 新增测试直接消费 shared `TextLayout` / `Renderer` trait
  - macOS/Linux completed gate 扩展到 4/8，completion percent 为 50
  - 下一项变为 `native_clipboard_services`
  - porting work items 从 10 项降到 8 项
- 这一步仍然是契约级 scaffold，不宣称真实 CoreGraphics/CoreText 或 GTK snapshot/Pango 截图完成；真实平台 runner 后续还要验证原生绘制结果

## step346
- 将 macOS/Linux 的 `native_clipboard_services` 契约进度入账：
  - completed gate 扩展到 5/8，completion percent 为 62
  - 下一项变为 `native_dialog_services`
  - porting work items 从 8 项降到 6 项
  - macOS clipboard 契约测试补充 required operation count、file-path/sequence/monitor-ignore fallback 行为
- 依据是两侧都有 clipboard、paste target 和 window identity host 契约与测试；macOS 真实剪贴板读写仍需要在 macOS runner 上验证

## step347
- 将 macOS/Linux 的 `native_dialog_services` 契约进度入账：
  - completed gate 扩展到 6/8，completion percent 为 75
  - 下一项变为 `native_settings_surfaces`
  - porting work items 从 6 项降到 4 项
  - reusable app blueprint 中 macOS `system_services` 不再被 clipboard/dialog gate 阻塞
- 依据是两侧都有 popup menu、shell open、file dialog、text input dialog 和 edit text dialog host 契约与测试；真实 AppKit/GTK 对话框仍需目标平台 runner 验证

## step348
- 将 macOS/Linux 的 `native_settings_surfaces` 契约进度入账：
  - completed gate 扩展到 7/8，completion percent 为 87
  - 下一项只剩 `ai_action_presentation`
  - porting work items 从 4 项降到 2 项
- 依据是两侧都有 settings window 和 settings dropdown host 契约与测试；真实 AppKit/GTK settings UI 仍需目标平台 runner 验证

## step349
- 将 macOS/Linux 的 `ai_action_presentation` 契约补进基础 UI 层：
  - completed gate 扩展到 8/8，completion percent 为 100
  - 下一项变为 `None`
  - porting work items 收敛为 0 项
- 依据是新增 `app_core::ai_action_protocol`，并让 macOS/Linux application model 能记录 AI 菜单请求、settings surface 请求和 execution plan 桥接；真实 AppKit/GTK 还需要目标平台 runner 验证

## step350
- 开始真实 AppKit/GTK native host 落地：
  - 新增 `app_core::native_host_launch`，区分 `real_native_host` 与 `contract_scaffold_fallback`
  - 新增 `src/macos_native_host.rs`，目标 macOS 上通过 `objc2-app-kit` 创建 `NSApplication` / `NSWindow` 并进入 AppKit event loop
  - 新增 `src/linux_native_host.rs`，目标 Linux 上通过 `gtk4::Application` / `gtk4::ApplicationWindow` 创建 GTK 窗口并进入 GTK event loop
  - macOS/Linux `run()` 现在先读取 launch plan，目标 OS 上走真实 native host，非目标 OS 测试环境继续使用 scaffold fallback
  - 新增 launch-plan 测试，避免把当前 Windows 上的 contract fallback 误报为目标 OS 真实验证完成
- 这一步让 AppKit/GTK 原生 host 有了可调用落点；最终完成仍需要在 macOS/Linux runner 上编译、启动并做截图/交互验证

## step351
- 继续把真实 AppKit/GTK host 从“能开窗口”推进到“能调用功能”：
  - 新增 `app_core::native_host_actions`，统一 Search、Settings、Hide、Close 四个 native host 主窗口动作
  - macOS AppKit 窗口新增 `NSButton` target/action selector，点击后进入 shared `NativeHostUiAction`
  - Linux GTK 窗口新增 `Button::with_label` / `connect_clicked`，点击后进入 shared `NativeHostUiAction`
  - macOS/Linux action bridge 测试验证动作会进入 ZSClip product adapter，并返回 `zsclip.window.toggle_search`、`zsclip.window.open_settings`、`zsclip.window.hide` 等产品结果
- 这一步让其他平台的原生窗口具备第一组可点击功能；完整原生设置页、菜单、剪贴板、对话框和截图/交互验证仍需继续在目标平台 runner 上完成

## step352
- 继续真实 AppKit/GTK settings surface 落地：
  - `NativeHostUiAction` 新增 settings-surface 和 close-host 行为标记
  - AppKit `Settings` 按钮现在创建或聚焦 `ZSClip Settings` 原生 `NSWindow`
  - GTK `Settings` 按钮现在创建 `ZSClip Settings` 原生 `ApplicationWindow`
  - 两侧 settings surface 先展示共享设置页名称：General、Hotkeys、Plugins、Groups、Cloud、About
  - `Close` 动作现在通过 shared 行为标记关闭/退出原生 host
- 这一步让 macOS/Linux 真实 host 从单窗口推进到 main + settings 双 surface；完整设置控件绑定和目标平台截图/点击验证仍需继续

## step353
- 继续把 settings surface 从“能打开窗口”推进到“能调用设置功能”：
  - `app_core::native_host_actions` 新增 `NativeHostSettingsAction`
  - 统一 Save、Close、Open Config 三个原生设置动作到 shared command ids
  - AppKit settings window 新增 `Save`、`Open Config`、`Close` 三个 `NSButton` selector
  - GTK settings window 新增 `Save`、`Open Config`、`Close` 三个 `Button::with_label` / `connect_clicked` 控件
  - macOS/Linux settings action 测试验证进入 product adapter：`zsclip.settings.save`、`zsclip.settings.open_config`、`zsclip.settings.close`
- 这一步让 settings window 具备第一组真实可调用控件；后续还要继续绑定各设置页具体字段、菜单、剪贴板和目标平台截图/点击验证

## step354
- 继续把主窗口 Search 从“命令可调用”推进到“真实原生搜索控件可见”：
  - `NativeHostUiAction` 新增 `toggles_search_surface`
  - AppKit main window 新增 `NSSearchField`，默认隐藏，点击 Search 时显示/隐藏
  - GTK main window 新增 `SearchEntry`，默认隐藏，点击 Search 时显示/隐藏并聚焦
  - 入口 guard 测试确认 AppKit/GTK host 源码包含原生搜索控件路径
- 这一步让 Search 动作同时进入 product adapter 并改变真实原生 UI；后续要继续接搜索文本变化到共享列表过滤和目标平台交互验证

## step355
- 继续把原生 Search 从“控件可见”推进到“文本进入产品层”：
  - 新增 `command_ids::UPDATE_SEARCH_TEXT`
  - 新增 `NativeHostSearchTextAction`，把原生搜索文本包装为 `CommandPayload::Text`
  - `ZsclipProductAdapter` 新增 `zsclip.window.search_text_update` 路由，并把文本写入 product snapshot
  - AppKit `NSSearchField` 通过 `zsclipSearchTextChanged:` selector 派发搜索文本
  - GTK `SearchEntry` 通过 `connect_search_changed` 派发搜索文本
  - macOS/Linux 测试验证 AppKit/GTK search text bridge 会进入 product adapter
- 这一步让其他平台搜索框不再只是本地控件；后续要继续把 product search state 反投影到原生列表过滤/渲染，并在目标平台做交互验证

## step356
- 继续把 Search 文本从“进入产品层”推进到“真实原生列表可见过滤”：
  - 新增 `NativeHostClipListItem` 和 `REQUIRED_NATIVE_HOST_CLIP_LIST_ITEMS`
  - 新增 `native_host_filtered_clip_item_ids()`，统一 AppKit/GTK 预览列表过滤规则
  - AppKit main window 新增剪贴板预览行，搜索文本变化时隐藏/显示匹配行
  - GTK main window 新增剪贴板预览行，搜索文本变化时隐藏/显示匹配行
  - app_core 测试验证标题/预览文本过滤规则
- 这一步让搜索输入在原生 UI 上产生可见列表反馈；后续需要把预览数据换成真实 product projection 并做目标平台截图/点击验证

## step357
- 把 AppKit/GTK 原生列表从静态预览行推进到 product adapter 投影驱动：
  - 新增 `NativeHostClipListItemProjection` 与 `native_host_filtered_projected_clip_item_ids()`
  - `ProductAdapterProjectedState` 新增 `native_clip_items`
  - `ZsclipProductAdapter::project_product_state()` 现在把 snapshot 的 native clip items 投影给平台 host
  - AppKit `NSWindow` 和 GTK `ApplicationWindow` 都从 `macos_native_host_projected_clip_items()` / `linux_native_host_projected_clip_items()` 构造原生行
  - 搜索过滤改为对 product 投影列表生效，而不是直接读取静态 fixture
- 这一步没有重写功能；它把原生 UI 的列表数据源接到现有 product adapter。后续应优先直接接现有功能命令和真实启动数据，避免继续做重复 UI 架构。

## step358
- 按“原生 UI 只做壳，功能继续调用现有产品层”的路线，给 AppKit/GTK main window 增加第一组行功能入口：
  - 新增 `NativeHostRowAction`，只映射 Paste、Copy、Pin、Delete 到现有 shared row menu ids
  - AppKit 新增 `Paste`、`Copy`、`Pin`、`Delete` 原生 `NSButton` 和对应 selector
  - GTK 新增同名原生 `Button`，点击后直接调用 `dispatch_linux_native_row_action`
  - macOS/Linux 测试验证原生行动作进入 `zsclip.row.paste`、`zsclip.row.copy`、`zsclip.row.toggle_pin`、`zsclip.row.delete`
- 这一步避免重写行功能；native host 只负责把原生点击翻译成现有 `window.menu.invoke` 命令。

## step359
- 继续扩充 AppKit/GTK 原生 row 功能入口，但仍只调用已有产品命令：
  - `NativeHostRowAction` 从 4 个扩展到 8 个：Paste、Copy、Pin、To Phrase、Delete、Edit、Open Path、Translate
  - AppKit 新增 `zsclipRowToPhrase:`、`zsclipRowEdit:`、`zsclipRowOpenPath:`、`zsclipRowTextTranslate:` selector 和第二排行按钮
  - GTK 继续遍历 `REQUIRED_NATIVE_HOST_ROW_ACTIONS`，自动得到新增按钮
  - macOS/Linux 测试覆盖 `zsclip.row.to_phrase`、`zsclip.row.edit`、`zsclip.row.open_path`、`zsclip.row.text_translate`
- 这一步让原生 host 能调用更多已存在功能，不重写业务逻辑。

## step360
- 将 macOS/Linux 真实平台验收路径落到仓库：
  - 新增 `scripts/native-host-smoke-macos.sh`，在 macOS 上运行 AppKit launch/dispatch 测试、构建 `zsclip`、启动真实 AppKit host、截图，并可选用 AppleScript 点击 Search/Settings/Copy/Translate
  - 新增 `scripts/native-host-smoke-linux.sh`，在 Linux 桌面会话中运行 GTK launch/dispatch 测试、构建 `zsclip`、启动真实 GTK host、截图，并可选用 `xdotool` 点击控件
  - 新增 `docs/native-host-verification.md`，明确哪些 artifact 才能证明目标 OS 上的原生 UI 真实可见可点
  - `docs/ui-host-porting.md` 链接到这套 smoke 入口，避免再把 Windows 侧 scaffold 测试误当成真实平台完成
- 这一步不声称 macOS/Linux 已完成；它提供后续完成验收所需的目标平台证据路径。

## step361
- 给 AppKit/GTK settings window 增加第一组具体设置控件入口，继续保持“原生 UI 只调用现有产品命令”：
  - 新增 `NativeHostSettingsControlAction`，覆盖 Capture、LAN Sync、Cloud Sync 三个 Toggle 和 Sync Mode Dropdown
  - AppKit settings window 新增对应 `NSButton` 和 selector：`zsclipToggleClipboardCapture:`、`zsclipToggleLanSync:`、`zsclipToggleCloudSync:`、`zsclipOpenSyncModeDropdown:`
  - GTK settings window 遍历 `REQUIRED_NATIVE_HOST_SETTINGS_CONTROL_ACTIONS` 生成同名原生按钮
  - macOS/Linux 测试验证 Toggle 进入 `zsclip.settings.toggle_control`，Dropdown 进入 `zsclip.settings.open_dropdown`
- 这一步让 settings surface 不再只有 Save/Close 类窗口按钮，而开始具备真实设置控件调用入口。

## step362
- 给 macOS/Linux 原生 host 补上可见的状态栏/菜单入口，继续沿用现有 tray 产品命令：
  - 新增 `NativeHostStatusMenuAction`，覆盖 Show ZSClip、Toggle Capture、Toggle LAN Sync、Exit
  - AppKit 通过 `NSStatusItem` + `NSMenu` + `NSMenuItem` 挂出菜单项，点击后进入 `zsclip.tray.*` 路由
  - GTK 通过 `gio::Menu` + `gio::SimpleAction` + `MenuButton` 提供可见状态菜单入口，点击后进入同一批 tray 路由
  - macOS/Linux 测试覆盖状态菜单动作到 tray route 的映射
- 这一步不是重做 tray 逻辑，而是把原有功能露到原生 host 上，方便目标平台直接验收。

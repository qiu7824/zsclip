# 纯白键盘皮肤

默认皮肤 ID 为 `1`。白色键盘与工具栏、黑色正文和图标、灰色上标、浅灰细边圆角键帽。按下时使用浅灰底色。

`assets/` 与 APK 的资源目录对应；`vectors/` 保存键帽矢量源文件，`palette.json` 定义颜色。`mdpi`、`xdpi`、`xxdpi` 分别提供原引擎对应密度的键帽。

键帽使用 Android 编译格式的 9-patch PNG，保持各密度原有画布、伸缩区和内边距。键盘布局、点击范围、字符映射、长按符号及切换动作由输入法原有布局定义。

`skin_configer/config_qqxml/style/style.xml` 定义键盘文字、上标、图标、背景和候选样式；`skin_configer/board_config.ini` 定义功能面板颜色。

默认皮肤使用输入法自带的透明图标资源，覆盖大小写切换、退格、回车、空格语音和功能面板。`WhiteSkinResources` 在默认皮肤启用时补齐缺失的图标缓存及按下状态图标，保留已有资源及其他皮肤。

本地与云剪贴板使用独立的原生页面和列表适配器，其纯白背景、黑灰文字、浅边框及选择状态由 `ClipboardVisualStyle` 在动态绑定后应用。设置工具键由 `ToolbarIconStyle` 按可见邻键的实际图像内容边界对齐，不修改原始位图文件。

# ZSClip 下一版 AI 素材生成清单

## 统一要求

- 画幅：16:9，优先 1920x1080 或 4K。
- 风格：高级系统工具发布片，克制、清晰、有工程美感，不要营销海报感。
- 禁止：不要生成假的 ZSClip 界面、不要生成可读长文字、不要生成 logo、不要把真实截图上传后改坏。
- 留白：画面下方 90px 留给章节进度条，左右至少 80px 安全边距。
- 用法：AI 素材只做背景、转场、隐喻和节奏层；真实功能演示继续叠真实录屏/截图。

## Edge GPT 生成图片

### 01-python-to-rust-origin.png

用途：0:00 - 0:28，讲 ZSClip 从 Python 小工具一路迭代到 Rust。

提示词：

```text
16:9 cinematic product film key visual, a local desktop utility evolving from lightweight Python scripts into a Rust native system tool, floating code fragments, clipboard memory fragments, subtle desktop window silhouettes, elegant dark workstation atmosphere, premium engineering aesthetic, realistic glass and metal, no readable text, no logos, no fake UI, leave clean lower area for progress bar, high detail
```

### 02-rust-native-system-ui.png

用途：0:28 - 0:58，讲 Rust 纯原生 UI、托盘、热键、窗口、剪贴板贴系统运行。

提示词：

```text
16:9 cinematic architecture visualization of a Rust native system utility UI framework, modular layers for tray, hotkeys, clipboard, popup window, settings panel and list view, type-safe explicit contracts represented by clean mechanical connectors, low magic, low runtime reflection, dark premium engineering scene, no readable text, no logo, no fake application screenshot, leave lower area clear
```

### 03-vv-instant-paste-flow.png

用途：0:58 - 1:16，VV 模式的动作隐喻，用来接真实 VV 录屏。

提示词：

```text
16:9 elegant motion-ready key visual, a cursor-centered quick paste flow, small candidate window silhouette near typing cursor, clipboard history fragments flowing into a focused writing surface, warm orange accent, fast but quiet productivity feeling, no readable text, no brand logo, no fake UI details, clean lower safe zone
```

### 04-search-nearby-timeline.png

用途：1:16 - 1:38，搜索独特点：应用、日期、时间、类型、附近记录。

提示词：

```text
16:9 cinematic data-search visualization for a clipboard manager, records arranged along a timeline, filters represented as app, date, time, type, and nearby context nodes, calm purple-blue accent but not dominated by purple, premium desktop utility feeling, no readable labels, no fake UI, no logo, lower 90px left clean for chapter progress
```

### 05-right-click-grouping-material-board.png

用途：1:38 - 1:53，右键分组、复制记录/常用记录、文件类型选项。

提示词：

```text
16:9 product film key visual, copied text snippets, image thumbnails, and file cards being organized into clear groups through a subtle right-click action metaphor, material board, local-first office workflow, refined engineering aesthetic, no readable text, no fake software UI, no logo, clean lower area
```

### 06-ocr-materialization.png

用途：1:53 - 2:12，OCR 把截图变成可复用素材。

提示词：

```text
16:9 cinematic visualization of OCR materialization, a screenshot-like image fragment gently transforming into editable text blocks and reusable phrase cards, teal accent, office productivity tool, elegant and realistic, no readable text, no fake UI, no logo, leave room for overlaying real ZSClip recording
```

### 07-memory-aesthetic-minimal.png

用途：2:12 - 2:47，0.1M 级内存占用美学、分页、虚拟列表、释放缓存。

提示词：

```text
16:9 minimal high-end engineering visual, an ultra-light resident system tool represented by a tiny glowing memory footprint in a vast dark workspace, virtualized list rows fading in only where visible, cache blocks dissolving when hidden, restrained yellow accent, premium and quiet, no readable numbers or text, no logo, no fake UI
```

### 08-zsui-open-engineering-contracts.png

用途：2:47 - 3:07，ZSUI 不是通用 UI 框架，而是 Rust 原生系统工具开放工程。

提示词：

```text
16:9 cinematic open engineering framework visual, contract-first modular Rust native system utility framework, contributors and AI agents adding components by explicit interfaces, tray, hotkey, popup, settings, list modules as clean replaceable blocks, type-safe compositional low-magic architecture, no readable text, no logo, no fake UI, refined dark workstation style
```

### 09-multiplatform-test-lab.png

用途：3:07 - 3:36，Android / iOS / macOS / Linux 初代测试、欢迎反馈。

提示词：

```text
16:9 calm ending key visual, multi-platform native system utility test lab, Windows main workstation in focus, Android, iOS, macOS and Linux devices as early test benches around it, open beta feedback atmosphere, hopeful but restrained, no readable text, no logos, no fake screenshots, leave lower area clear
```

## 豆包生成视频

每条 6-8 秒，优先用上面对应图片作为首帧/参考图。运动要轻，不能像科幻片乱飞。

### db-01-python-rust-evolution.mp4

参考图：`01-python-to-rust-origin.png`

提示词：

```text
基于参考图生成 8 秒 16:9 视频。镜头缓慢推进，Python 脚本碎片逐渐汇聚成 Rust 原生系统工具的抽象结构，光线轻微流动，节奏克制，工程美学，不出现可读文字、logo、假软件界面。下方 90px 保持干净。
```

### db-02-native-ui-contracts.mp4

参考图：`02-rust-native-system-ui.png`

提示词：

```text
基于参考图生成 7 秒 16:9 视频。模块像系统组件一样轻微拼合，托盘、热键、弹窗、列表用抽象块表示，镜头有细微视差和光扫，不要生成文字，不要生成真实软件界面，保持高级发布片质感。
```

### db-03-search-nearby-timeline.mp4

参考图：`04-search-nearby-timeline.png`

提示词：

```text
基于参考图生成 7 秒 16:9 视频。复制记录沿时间线轻微展开，筛选节点依次亮起，最后出现“附近记录”的空间感，但不要出现可读文字。画面用于叠加真实 ZSClip 搜索录屏，所以不要生成任何假 UI。
```

### db-04-ocr-materialization.mp4

参考图：`06-ocr-materialization.png`

提示词：

```text
基于参考图生成 6 秒 16:9 视频。截图碎片轻轻分解成可编辑文本块和素材卡片，像办公资料被重新整理，不要出现可读文字、logo、假 UI，运动轻柔，不破坏真实素材叠层空间。
```

### db-05-memory-aesthetic.mp4

参考图：`07-memory-aesthetic-minimal.png`

提示词：

```text
基于参考图生成 8 秒 16:9 视频。极小的内存占用光点在大空间里稳定存在，可见列表区域轻微点亮，隐藏缓存块缓慢消散，整体安静、克制、有系统工具的低存在感。不要出现数字、文字、logo。
```

### db-06-zsui-open-engineering.mp4

参考图：`08-zsui-open-engineering-contracts.png`

提示词：

```text
基于参考图生成 8 秒 16:9 视频。AI 和贡献者通过明确合同补组件、补平台、补测试的抽象工程场景，模块轻微连接，镜头缓慢横移，强调 Rust 原生系统工具框架，不出现可读文字，不出现假界面。
```

### db-07-ending-test-lab.mp4

参考图：`09-multiplatform-test-lab.png`

提示词：

```text
基于参考图生成 8 秒 16:9 视频。多平台测试台缓慢亮起，Windows 主力版本在中心，其他平台是初代测试状态，结尾光线慢慢收束，适合作为视频结束画面。不要出现可读文字、logo、假截图。
```

## 交付文件命名

把生成好的文件放到：

```text
E:\rust\zsclip\videos\zsclip-ai-competition\assets\ai-generated-next
```

如果只方便发给我文件，按上面的文件名命名即可。下一版我会把这些视频做成真实过渡层，不再只靠静态图片推拉。

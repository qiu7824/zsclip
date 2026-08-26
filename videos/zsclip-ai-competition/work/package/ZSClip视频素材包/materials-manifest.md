# ZSClip 视频素材清单

## 主交付

- `output/zsclip-ai-competition-silent-draft.mp4`：约 3 分 36 秒无声草稿，1280x720 / 30fps / H.264 yuv420p。
- `output/zsclip-ai-competition-final-with-bgm.mp4`：火山 TTS 口播和低音量 BGM 合成后的最终版。
- `voiceover-final-2m11.md`：对应无声草稿的双人口播稿，口播约 3 分 30 秒，画面保留到 3 分 36 秒完整收束。
- `voiceover-teleprompter.html`：本地口播提词器，播放草稿并按时间高亮口播段落。
- `START_HERE.md`：桌面素材目录的起步说明。
- `double_click_finalize.cmd`：录完口播后双击执行自动收口。
- `review-handoff.html`：本地审片总控页。
- `materials-audit.html`：录制素材核对页，对照每个用户要求标记已录制、待口播和可选补录。
- `final-readiness.html`：最终发布状态页，检测口播音频、有声版、封面、文案和 QA 状态。
- `bilibili-release-copy.md`：B 站标题、简介、标签和章节文案。
- `assets/cover/bilibili-cover-zsclip.png`：B 站封面图。
- `output/delivery-qa-report.md`：视频交付 QA 报告。
- `output/delivery-qa-final-report.md`：TTS / 口播到位后的最终严格 QA 报告。
- `finalize_after_voiceover.ps1`：TTS / 口播到位后一键合成有声版并运行严格 QA。
- `watch_voiceover_and_finalize.ps1`：监听口播音频，检测到后自动合成、严格 QA、重新打包。
- `generate_final_readiness.ps1`：生成最终发布状态页。
- `generate_volcengine_voiceover.ps1` / `generate_volcengine_voiceover.py`：用火山引擎 TTS 生成口播 MP3，密钥只从环境变量读取。
- `generate_bgm.ps1`：生成无版权依赖的低音量技术感背景音乐。
- `build_final_with_bgm.ps1`：混合 TTS 口播和 BGM，输出最终有声版。
- `package_delivery.ps1`：把当前可交付素材打包到桌面 `ZSClip视频素材包.zip`。

## 单段录屏

- `assets/recordings/03-desktop-non-overlap-demo.mp4`：桌面主演示，包含真实鼠标操作、搜索和 VV 模式，窗口不重叠。
- `assets/recordings/05-feature-closeups-search-group-ocr.mp4`：搜索、右键分组、常用短语、图片 OCR 特写。
- `assets/recordings/04-dev-status-scroll.mp4`：Codex 协作记录、Android/iOS/macOS/Linux 初代状态滚动页。
- `assets/recordings/02-desktop-search-vv-clean.mp4`：备用搜索/VV 清洁录屏。

## 截图与卡片

- `assets/screenshots/06-task-manager-zsclip-memory.png`：任务管理器搜索 zsclip 的内存截图。
- `assets/screenshots/07-codex-token-usage.png`：Codex token 使用量截图。
- `assets/ai-generated/scene-python-rust-fragments.png`：Edge / ChatGPT 生成的 Python 到 Rust 迭代艺术关键帧。
- `assets/ai-generated/scene-memory-minimal.png`：Edge / ChatGPT 生成的极简内存占用美学关键帧。
- `assets/ai-generated/scene-zsui-open-engineering.png`：Edge / ChatGPT 生成的 ZSUI 开放工程关键帧。
- `work/draft/cards/05-roadmap-card.png`：后续更新卡片，搜索“附近”、分组“文件类型选项”、继续打磨多平台。

## 字幕

- `captions/voiceover-captions.srt`：SRT 字幕。
- `captions/voiceover-captions.json`：Remotion Caption JSON。

## 有声版合成

可以直接运行火山 TTS 双人口播脚本生成 `voiceover-input/volcengine-voiceover.mp3`，也可以把真人口播音频放到以下任一位置：

```powershell
videos\zsclip-ai-competition\voiceover-input
C:\Users\xs\Desktop\ZSClip视频素材\voiceover-input
C:\Users\xs\Desktop\ZSClip视频素材
```

推荐命名：

- `voiceover.wav`
- `voiceover.mp3`
- `voiceover.m4a`

合成命令：

```powershell
.\videos\zsclip-ai-competition\build_final_with_bgm.ps1
```

最终收口命令：

```powershell
.\videos\zsclip-ai-competition\finalize_after_voiceover.ps1
```

等待口播并自动收口：

```powershell
.\videos\zsclip-ai-competition\watch_voiceover_and_finalize.ps1
```

脚本会自动选择最新的音频文件，检查音频时长、标准化响度并输出：

- `output/zsclip-ai-competition-voiceover-draft.mp4`
- `C:\Users\xs\Desktop\ZSClip视频素材\zsclip-ai-competition-voiceover-draft.mp4`

## 当前状态

- 已录制：主功能、VV、搜索、右键分组、OCR、内存、token、多平台开发状态。
- 已剪辑：3 分 36 秒新版无声草稿时间线。
- 已准备：双人口播稿、字幕、发布文案、封面图、审片页、有声合成脚本。
- 已验证：无声草稿规格、封面尺寸、素材桌面副本和有声合成管线。
- 已打包：桌面 `ZSClip视频素材包.zip` 会收纳当前可交付素材；TTS / 口播和有声版生成后再次运行打包脚本即可更新。
- 待完成：生成双人 TTS、BGM、最终合成并重新跑严格 QA。

# ZSClip 视频项目从这里开始

## 先审片

打开：

```text
C:\Users\xs\Desktop\ZSClip视频素材\review-handoff.html
```

这里可以查看无声草稿、封面、分段录屏、截图、口播稿、字幕、发布文案、QA 报告和素材包。

如果担心素材是否漏录，打开：

```text
C:\Users\xs\Desktop\ZSClip视频素材\materials-audit.html
```

如果想确认现在能不能发布，打开：

```text
C:\Users\xs\Desktop\ZSClip视频素材\final-readiness.html
```

## 录口播

推荐打开提词器：

```text
C:\Users\xs\Desktop\ZSClip视频素材\voiceover-teleprompter.html
```

导出口播音频，推荐命名：

- `voiceover.wav`
- `voiceover.mp3`
- `voiceover.m4a`

放到任一位置：

```text
C:\Users\xs\Desktop\ZSClip视频素材
C:\Users\xs\Desktop\ZSClip视频素材\voiceover-input
E:\rust\zsclip\videos\zsclip-ai-competition\voiceover-input
```

## 生成最终有声版

最简单方式：双击桌面素材目录里的：

```text
double_click_finalize.cmd
```

它会自动查找 TTS / 口播音频，然后执行：

1. 合成有声版
2. 运行最终严格 QA
3. 重新打包 `ZSClip视频素材包.zip`

有声版输出：

```text
C:\Users\xs\Desktop\ZSClip视频素材\zsclip-ai-competition-voiceover-draft.mp4
C:\Users\xs\Desktop\ZSClip视频素材\zsclip-ai-competition-final-with-bgm.mp4
```

## 当前状态

当前素材已经完成 3:36 无声草稿、双人口播、BGM 和最终有声版输出；运行严格 QA 后即可发布或继续替换真人录音。

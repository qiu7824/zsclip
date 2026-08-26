# 口播音频放这里

把录好的口播文件放到下面任一位置：

- `videos\zsclip-ai-competition\voiceover-input`
- `C:\Users\xs\Desktop\ZSClip视频素材\voiceover-input`
- `C:\Users\xs\Desktop\ZSClip视频素材`

推荐文件名：

- `voiceover.wav`
- `voiceover.mp3`
- `voiceover.m4a`

当前双人口播目标时长约 3 分 30 秒，对应：

- `videos/zsclip-ai-competition/voiceover-final-2m11.md`
- `videos/zsclip-ai-competition/output/zsclip-ai-competition-silent-draft.mp4`

音频放好后，在仓库根目录运行：

```powershell
.\videos\zsclip-ai-competition\build_final_with_bgm.ps1
```

也可以直接运行最终收口脚本，它会合成有声版并立刻跑严格 QA：

```powershell
.\videos\zsclip-ai-competition\finalize_after_voiceover.ps1
```

输出文件：

- `videos/zsclip-ai-competition/output/zsclip-ai-competition-voiceover-draft.mp4`
- `C:\Users\xs\Desktop\ZSClip视频素材\zsclip-ai-competition-voiceover-draft.mp4`

如果口播比视频长，最终 BGM 合成脚本会提示调整语速或显式允许裁掉尾部静音。

脚本会自动从上面几个位置里选择最新的音频文件。
